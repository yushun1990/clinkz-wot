use alloc::{boxed::Box, format};

use clinkz_wot_core::{
    AffordanceTarget, BoundConsumedThing, ClientBinding, CodecInput, ConsumedThing, CoreError,
    CredentialStore, InteractionInput, InteractionOutput, Payload, SecurityContext, SecurityError,
    Subscription, TransportRequest,
};
use clinkz_wot_discovery::ThingDirectory;
use clinkz_wot_protocol_bindings::{
    AffordanceRef, FormSelectionCriteria, resolve_form_security,
    select_affordance_form_with_criteria, validate_affordance_form_with_criteria,
};
use clinkz_wot_td::{data_type::Operation, form::Form, security_scheme::SecurityScheme};

use crate::{
    BindingPlan, SelectedFormCacheKey, ServientError, ServientResult,
    cache::affordance_target_from_ref,
    consumed::ConsumedThingEntry,
    servient::Servient,
    servient::{BindingFactoryRegistry, PayloadCodecRegistry, SecurityProviderRegistry},
};

struct ActiveBindingPlan {
    form: Form,
    binding: Box<dyn ClientBinding>,
}

pub(crate) struct InteractionRuntime {
    binding_factories: BindingFactoryRegistry,
    payload_codecs: PayloadCodecRegistry,
    security_providers: SecurityProviderRegistry,
    credential_store: Option<alloc::sync::Arc<dyn CredentialStore>>,
}

impl InteractionRuntime {
    pub(crate) fn new(
        binding_factories: BindingFactoryRegistry,
        payload_codecs: PayloadCodecRegistry,
        security_providers: SecurityProviderRegistry,
        credential_store: Option<alloc::sync::Arc<dyn CredentialStore>>,
    ) -> Self {
        Self {
            binding_factories,
            payload_codecs,
            security_providers,
            credential_store,
        }
    }
}

impl<D> Servient<D>
where
    D: ThingDirectory,
{
    pub(crate) fn consumed_request(
        &self,
        entry: &ConsumedThingEntry,
        id: &str,
        target: AffordanceTarget,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.interaction_runtime()
            .consumed_request(entry, id, target, affordance, criteria, input)
    }

    pub(crate) fn consumed_subscribe(
        &self,
        entry: &ConsumedThingEntry,
        id: &str,
        target: AffordanceTarget,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<Subscription> {
        self.interaction_runtime()
            .consumed_subscribe(entry, id, target, affordance, criteria, input)
    }
}

impl InteractionRuntime {
    // -----------------------------------------------------------------------
    // Remote (consumed) interactions: form selection + binding invocation.
    //
    // Per baseline v3.0 §5.1, form selections and binding plans are internalized
    // inside the interned ConsumedThingEntry, not recomputed per call.
    // -----------------------------------------------------------------------

    /// Performs a remote interaction against an interned consumed-Thing entry,
    /// selecting a form, applying transport security, and invoking a binding.
    pub(crate) fn consumed_request(
        &mut self,
        entry: &ConsumedThingEntry,
        id: &str,
        target: AffordanceTarget,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        let thing = entry.thing();
        let active_plan =
            self.cached_or_select_binding_plan(entry, thing, id, affordance, criteria)?;
        let input = self.prepare_interaction_input(
            thing,
            Some(&active_plan.form),
            criteria.operation,
            input,
        )?;
        let thing_arc = entry.thing_arc();
        let mut consumed = self.bound_consumed_thing_with_binding(thing_arc, active_plan.binding);
        let output = consumed
            .request(target, criteria.operation, &active_plan.form, input)
            .map_err(ServientError::from)?;
        self.prepare_interaction_output(output)
    }

    /// Opens a streaming subscription against an interned consumed-Thing entry,
    /// selecting a form, applying transport security, and invoking the binding's
    /// streaming `subscribe` path.
    ///
    /// The returned [`Subscription`] is for the caller to drain pushed samples.
    /// The wire cleanup [`SubscriptionGuard`] is stored in the entry and cleaned
    /// up by `unsubscribe_event` / `unobserve_property` / entry invalidation.
    pub(crate) fn consumed_subscribe(
        &mut self,
        entry: &ConsumedThingEntry,
        id: &str,
        target: AffordanceTarget,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<Subscription> {
        let thing = entry.thing();
        let active_plan =
            self.cached_or_select_binding_plan(entry, thing, id, affordance, criteria)?;
        let input = self.prepare_interaction_input(
            thing,
            Some(&active_plan.form),
            criteria.operation,
            input,
        )?;

        let request = clinkz_wot_core::BindingRequest {
            thing: entry.thing_arc(),
            target: target.clone(),
            operation: criteria.operation,
            form: alloc::sync::Arc::new(active_plan.form.clone()),
            input,
        };

        let (subscription, guard) = active_plan
            .binding
            .subscribe(request)
            .map_err(ServientError::from)?;

        let key = crate::consumed::SubscriptionKey::new(&target, criteria.operation.as_str());
        entry.store_subscription(key, guard);

        Ok(subscription)
    }

    // -----------------------------------------------------------------------
    // Private form-selection / security / codec helpers.
    // -----------------------------------------------------------------------

    fn prepare_interaction_input(
        &self,
        thing: &clinkz_wot_td::thing::Thing,
        form: Option<&Form>,
        operation: Operation,
        input: InteractionInput,
    ) -> ServientResult<InteractionInput> {
        let input = normalize_interaction_input(&self.payload_codecs, input)?;
        match form {
            Some(form) => self.apply_security(thing, form, operation, input),
            None => Ok(input),
        }
    }

    fn prepare_interaction_output(
        &self,
        output: InteractionOutput,
    ) -> ServientResult<InteractionOutput> {
        normalize_interaction_output(&self.payload_codecs, output)
    }

    fn apply_security(
        &self,
        thing: &clinkz_wot_td::thing::Thing,
        form: &Form,
        operation: Operation,
        mut input: InteractionInput,
    ) -> ServientResult<InteractionInput> {
        let effective_security = resolve_form_security(thing, form);
        self.security_providers.with(|providers| {
            for scheme_name in effective_security.security {
                let scheme = thing.security_definitions.get(scheme_name).ok_or_else(|| {
                    CoreError::Security(SecurityError::SchemeFailure(format!(
                        "Security definition '{}' is not declared",
                        scheme_name
                    )))
                })?;

                if is_nosec_security(scheme) {
                    continue;
                }

                let provider = providers
                    .iter_mut()
                    .find(|provider| provider.scheme_name() == scheme_name)
                    .ok_or_else(|| {
                        CoreError::Security(SecurityError::SchemeFailure(format!(
                            "No security provider registered for '{}'",
                            scheme_name
                        )))
                    })?;

                if !provider.supports_scopes(effective_security.scopes) {
                    return Err(CoreError::Security(SecurityError::SchemeFailure(format!(
                        "Security provider '{}' does not support scopes {:?}",
                        scheme_name, effective_security.scopes
                    )))
                    .into());
                }

                let mut request = TransportRequest::new(form.href.as_str(), operation.as_str());
                request.metadata = input.parameters.clone();
                request.payload = input.payload.take();
                provider.apply(
                    SecurityContext {
                        thing,
                        form,
                        scheme_name,
                        scheme,
                        credentials: self.credential_store.as_deref(),
                    },
                    &mut request,
                )?;
                // Security provider modifies request.metadata with auth headers.
                // Diff to extract only the security-added metadata.
                for (key, value) in &request.metadata {
                    if input.parameters.get(key) != Some(value) {
                        input.security_metadata.insert(key.clone(), value.clone());
                    }
                }
                input.payload = request.payload;
            }

            Ok(input)
        })
    }

    fn cached_or_select_binding_plan(
        &self,
        entry: &ConsumedThingEntry,
        thing: &clinkz_wot_td::thing::Thing,
        id: &str,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
    ) -> ServientResult<ActiveBindingPlan> {
        let key = SelectedFormCacheKey::new(id, affordance_target_from_ref(affordance), criteria);
        let current_generation = self.binding_factories.generation();

        if let Some(plan) = entry.get_plan(&key) {
            // Fast path: the binding factory registry has not changed since
            // this plan was validated, and the TD is immutable for the life of
            // the entry (the entry is invalidated on TD update). Skip both the
            // form revalidation and the `supports_with_thing` check; just
            // reconstruct the binding.
            if plan.factory_generation == current_generation {
                if let Some(binding) = self
                    .binding_factories
                    .make_binding(plan.binding_factory_index)
                {
                    return Ok(ActiveBindingPlan {
                        form: plan.form,
                        binding,
                    });
                }
                // Factory index is stale (registry shrank). Fall through to
                // full recompute.
                entry.remove_plan(&key);
            } else {
                // Generation changed: revalidate the cached plan against the
                // current factory set. On success, refresh the cached
                // generation so subsequent hits take the fast path.
                match self.active_binding_plan_from_cache(thing, affordance, criteria, &plan) {
                    Ok(active_plan) => {
                        entry.update_plan_generation(&key, current_generation);
                        return Ok(active_plan);
                    }
                    Err(_) => {
                        entry.remove_plan(&key);
                    }
                }
            }
        }

        let form = self.cached_or_select_form(entry, thing, id, affordance, criteria)?;
        let (binding_factory_index, binding) =
            self.select_binding_factory_for_form(thing, &form, criteria.operation)?;
        entry.insert_plan(
            key,
            BindingPlan {
                form: form.clone(),
                binding_factory_index,
                factory_generation: current_generation,
            },
        );

        Ok(ActiveBindingPlan { form, binding })
    }

    fn active_binding_plan_from_cache(
        &self,
        thing: &clinkz_wot_td::thing::Thing,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
        plan: &BindingPlan,
    ) -> ServientResult<ActiveBindingPlan> {
        validate_affordance_form_with_criteria(thing, affordance, &plan.form, criteria)?;
        let binding = self.binding_from_factory_index(plan.binding_factory_index)?;
        if binding.supports_with_thing(thing, &plan.form, criteria.operation) {
            Ok(ActiveBindingPlan {
                form: plan.form.clone(),
                binding,
            })
        } else {
            Err(CoreError::UnsupportedBinding(format!(
                "Cached binding factory {} no longer supports {:?} for {}",
                plan.binding_factory_index,
                criteria.operation,
                plan.form.href.as_str()
            ))
            .into())
        }
    }

    fn cached_or_select_form(
        &self,
        entry: &ConsumedThingEntry,
        thing: &clinkz_wot_td::thing::Thing,
        id: &str,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
    ) -> ServientResult<Form> {
        let key = SelectedFormCacheKey::new(id, affordance_target_from_ref(affordance), criteria);

        if let Some(form) = entry.get_form(&key) {
            // The TD is immutable for the life of the entry, so a cached form
            // is always still valid for the same affordance + criteria. Skip
            // revalidation.
            return Ok(form);
        }

        let form = select_affordance_form_with_criteria(thing, affordance, criteria)?
            .selection
            .form
            .clone();
        entry.insert_form(key, form.clone());
        Ok(form)
    }

    fn select_binding_factory_for_form(
        &self,
        thing: &clinkz_wot_td::thing::Thing,
        form: &Form,
        operation: Operation,
    ) -> ServientResult<(usize, Box<dyn ClientBinding>)> {
        self.binding_factories
            .find_supporting(thing, form, operation)
            .ok_or_else(|| {
                CoreError::UnsupportedBinding(format!(
                    "No binding supports {:?} for {}",
                    operation,
                    form.href.as_str()
                ))
                .into()
            })
    }

    fn binding_from_factory_index(&self, index: usize) -> ServientResult<Box<dyn ClientBinding>> {
        self.binding_factories.make_binding(index).ok_or_else(|| {
            CoreError::UnsupportedBinding(format!(
                "Binding factory index {} is not registered",
                index
            ))
            .into()
        })
    }

    fn bound_consumed_thing_with_binding(
        &self,
        thing: alloc::sync::Arc<clinkz_wot_td::thing::Thing>,
        binding: Box<dyn ClientBinding>,
    ) -> BoundConsumedThing {
        let mut consumed = BoundConsumedThing::from_arc(thing);
        consumed.register_binding(binding);
        consumed
    }
}

fn normalize_interaction_input(
    codecs: &PayloadCodecRegistry,
    mut input: InteractionInput,
) -> ServientResult<InteractionInput> {
    if let Some(payload) = input.payload.take() {
        input.payload = Some(normalize_payload(codecs, payload)?);
    }

    Ok(input)
}

fn normalize_interaction_output(
    codecs: &PayloadCodecRegistry,
    mut output: InteractionOutput,
) -> ServientResult<InteractionOutput> {
    if let Some(payload) = output.payload.take() {
        output.payload = Some(normalize_payload(codecs, payload)?);
    }

    Ok(output)
}

/// Normalizes an interaction payload through its registered codec, if any.
///
/// When a `PayloadCodec` is registered for the payload's content type, the
/// payload is decoded and re-encoded by that codec. This is intentional
/// canonicalization/validation: it lets a codec reject malformed payloads and
/// produce canonical bytes for downstream signing or hashing.
///
/// # Performance note
///
/// This round-trip allocates two `Vec<u8>` per call. Skipping it when the
/// caller's bytes are already canonical would require a separate
/// "target content type" or "validate-only" API on `PayloadCodec` so callers
/// can opt into the fast path. Tracked as a follow-up.
fn normalize_payload(codecs: &PayloadCodecRegistry, payload: Payload) -> ServientResult<Payload> {
    codecs.with(|codecs| {
        let Some(codec) = codecs
            .iter()
            .find(|codec| codec.content_type().as_ref() == payload.content_type.as_str())
        else {
            return Ok(payload);
        };

        let decoded = codec.decode(&payload)?;
        codec
            .encode(CodecInput {
                body: decoded.as_slice(),
                data_type: None,
            })
            .map_err(Into::into)
    })
}

fn is_nosec_security(scheme: &SecurityScheme) -> bool {
    matches!(scheme, SecurityScheme::NoSec(_))
}
