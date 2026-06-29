use alloc::{boxed::Box, format, string::ToString, sync::Arc};

use clinkz_wot_core::{
    AffordanceTarget, ClientBinding, CodecInput, CoreError, CredentialStore, InteractionInput,
    InteractionOutput, Payload, SecurityContext, SecurityError, Subscription, TransportRequest,
};
#[cfg(feature = "async")]
use clinkz_wot_core::{BindingRequest, SubscriptionGuard};
use clinkz_wot_discovery::ThingDirectory;
use clinkz_wot_protocol_bindings::{
    AffordanceRef, FormSelectionCriteria, expand_uri_template, resolve_form_security,
    select_affordance_form_with_criteria, validate_form_operation,
};
use clinkz_wot_td::{
    data_type::{FormHref, Operation, UriReference},
    form::Form,
    security_scheme::SecurityScheme,
};

use crate::{
    BindingPlan, SelectedFormCacheKey, ServientError, ServientResult,
    consumed::ConsumedThingEntry,
    servient::Servient,
    servient::{BindingFactoryRegistry, PayloadCodecRegistry, SecurityProviderRegistry},
};

struct ActiveBindingPlan {
    form: Arc<Form>,
    binding: Arc<dyn ClientBinding + Send + Sync>,
}

/// Expands a form's URI template href using the caller-supplied uriVariables.
///
/// If the form href is a concrete reference (not a template), returns the
/// original `Arc<Form>` unchanged (zero-cost fast path).
///
/// If the form href is a template, clones the form, expands the template
/// using `input.parameters` (the uriVariables), and returns a new `Arc<Form>`
/// with a concrete `FormHref::Reference`.
fn expand_form_href_if_template(
    form: &Arc<Form>,
    input: &InteractionInput,
) -> ServientResult<Arc<Form>> {
    match &form.href {
        FormHref::Template(template) => {
            let expanded = expand_uri_template(template, &input.parameters)
                .map_err(|err| ServientError::Accept(err.to_string()))?;

            let resolved = UriReference::parse(&expanded).map_err(|err| {
                ServientError::Accept(format!("expanded URI is invalid: {}", err))
            })?;

            let mut form_clone = (**form).clone();
            form_clone.href = FormHref::Reference(resolved);
            Ok(Arc::new(form_clone))
        }
        FormHref::Reference(_) => Ok(Arc::clone(form)),
    }
}

pub(crate) struct InteractionRuntime {
    binding_factories: BindingFactoryRegistry,
    payload_codecs: PayloadCodecRegistry,
    security_providers: SecurityProviderRegistry,
    credential_store: Option<alloc::sync::Arc<dyn CredentialStore>>,
    /// When `true` (default), every consumed interaction payload whose content
    /// type matches a registered codec is decoded and re-encoded for
    /// canonicalization/validation. When `false`, payloads pass through
    /// untouched — saving two `Vec<u8>` allocations per interaction for
    /// deployments that do not need canonical bytes (signing/hashing).
    normalize_payloads: bool,
}

impl InteractionRuntime {
    pub(crate) fn new(
        binding_factories: BindingFactoryRegistry,
        payload_codecs: PayloadCodecRegistry,
        security_providers: SecurityProviderRegistry,
        credential_store: Option<alloc::sync::Arc<dyn CredentialStore>>,
        normalize_payloads: bool,
    ) -> Self {
        Self {
            binding_factories,
            payload_codecs,
            security_providers,
            credential_store,
            normalize_payloads,
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
        target: AffordanceTarget,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.interaction_runtime()
            .consumed_request(entry, target, affordance, criteria, input)
    }

    pub(crate) fn consumed_subscribe(
        &self,
        entry: &ConsumedThingEntry,
        target: AffordanceTarget,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<Subscription> {
        self.interaction_runtime()
            .consumed_subscribe(entry, target, affordance, criteria, input)
    }

    #[cfg(feature = "async")]
    pub(crate) async fn consumed_request_async(
        &self,
        entry: &ConsumedThingEntry,
        target: AffordanceTarget,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.interaction_runtime()
            .consumed_request_async(entry, target, affordance, criteria, input)
            .await
    }

    #[cfg(feature = "async")]
    pub(crate) async fn consumed_subscribe_async(
        &self,
        entry: &ConsumedThingEntry,
        target: AffordanceTarget,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<Subscription> {
        self.interaction_runtime()
            .consumed_subscribe_async(entry, target, affordance, criteria, input)
            .await
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
        &self,
        entry: &ConsumedThingEntry,
        target: AffordanceTarget,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        let thing = entry.thing();
        let active_plan = self.cached_or_select_binding_plan(entry, thing, affordance, criteria)?;
        let input = self.prepare_interaction_input(
            thing,
            Some(&active_plan.form),
            criteria.operation,
            input,
        )?;

        // Expand URI template form hrefs using caller-supplied uriVariables
        // (InteractionInput.parameters) before handing the form to the binding.
        let form = expand_form_href_if_template(&active_plan.form, &input)?;

        // Invoke the cached binding directly (no per-request BoundConsumedThing
        // reconstruction). The plan was validated at cache time and the TD is
        // immutable for the entry's life, so re-validating the form on the hot
        // path is unnecessary (mirrors `consumed_subscribe`).
        let request = clinkz_wot_core::BindingRequest {
            thing: entry.thing_arc(),
            target,
            operation: criteria.operation,
            form,
            input,
        };
        let output = active_plan
            .binding
            .invoke(request)
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
        &self,
        entry: &ConsumedThingEntry,
        target: AffordanceTarget,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<Subscription> {
        let thing = entry.thing();
        let active_plan = self.cached_or_select_binding_plan(entry, thing, affordance, criteria)?;
        let input = self.prepare_interaction_input(
            thing,
            Some(&active_plan.form),
            criteria.operation,
            input,
        )?;

        // Expand URI template form hrefs before handing to the binding.
        let form = expand_form_href_if_template(&active_plan.form, &input)?;

        let request = clinkz_wot_core::BindingRequest {
            thing: entry.thing_arc(),
            target: target.clone(),
            operation: criteria.operation,
            form,
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
    // Async consumed interactions (behind `async` feature).
    //
    // These mirror the sync methods but route through `AsyncClientBinding`
    // when the concrete binding implements it, giving true non-blocking I/O.
    // When the binding does not implement `AsyncClientBinding`, the sync path
    // is used as a fallback (which may block the async executor).
    // -----------------------------------------------------------------------

    #[cfg(feature = "async")]
    pub(crate) async fn consumed_request_async(
        &self,
        entry: &ConsumedThingEntry,
        target: AffordanceTarget,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        let thing = entry.thing();
        let active_plan = self.cached_or_select_binding_plan(entry, thing, affordance, criteria)?;
        let input = self.prepare_interaction_input(
            thing,
            Some(&active_plan.form),
            criteria.operation,
            input,
        )?;
        let form = expand_form_href_if_template(&active_plan.form, &input)?;

        let request = clinkz_wot_core::BindingRequest {
            thing: entry.thing_arc(),
            target,
            operation: criteria.operation,
            form,
            input,
        };

        let output = if let Some(async_binding) = active_plan.binding.as_async_binding() {
            async_binding
                .invoke_async(request)
                .await
                .map_err(ServientError::from)?
        } else {
            // Fallback for bindings without a native async path. With a std
            // tokio runtime the blocking `invoke` is offloaded to a
            // blocking-pool thread so the async executor is not stalled
            // (baseline addendum §9.3). On no_std async (cooperative
            // single-thread) there is no blocking pool and types are not
            // `Send`, so the call runs inline — bindings targeting no_std async
            // should implement `AsyncClientBinding`.
            offload_invoke(Arc::clone(&active_plan.binding), request).await?
        };
        self.prepare_interaction_output(output)
    }

    #[cfg(feature = "async")]
    pub(crate) async fn consumed_subscribe_async(
        &self,
        entry: &ConsumedThingEntry,
        target: AffordanceTarget,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<Subscription> {
        let thing = entry.thing();
        let active_plan = self.cached_or_select_binding_plan(entry, thing, affordance, criteria)?;
        let input = self.prepare_interaction_input(
            thing,
            Some(&active_plan.form),
            criteria.operation,
            input,
        )?;
        let form = expand_form_href_if_template(&active_plan.form, &input)?;

        let request = clinkz_wot_core::BindingRequest {
            thing: entry.thing_arc(),
            target: target.clone(),
            operation: criteria.operation,
            form,
            input,
        };

        let (subscription, guard) =
            if let Some(async_binding) = active_plan.binding.as_async_binding() {
                async_binding
                    .subscribe_async(request)
                    .await
                    .map_err(ServientError::from)?
            } else {
                offload_subscribe(Arc::clone(&active_plan.binding), request).await?
            };

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
        let input = if self.normalize_payloads {
            normalize_interaction_input(&self.payload_codecs, input)?
        } else {
            input
        };
        match form {
            Some(form) => self.apply_security(thing, form, operation, input),
            None => Ok(input),
        }
    }

    fn prepare_interaction_output(
        &self,
        output: InteractionOutput,
    ) -> ServientResult<InteractionOutput> {
        if self.normalize_payloads {
            normalize_interaction_output(&self.payload_codecs, output)
        } else {
            Ok(output)
        }
    }

    fn apply_security(
        &self,
        thing: &clinkz_wot_td::thing::Thing,
        form: &Form,
        operation: Operation,
        mut input: InteractionInput,
    ) -> ServientResult<InteractionInput> {
        let effective_security = resolve_form_security(thing, form);
        // Snapshot provider handles under a brief *read* lock, then apply
        // *outside* the registry lock so a slow provider (e.g. token refresh,
        // signing) does not serialize every outbound security application.
        // `apply` takes `&self`, so an `Arc` clone is sufficient to release
        // the handle.
        let providers = self.security_providers.with_read_recover(Arc::clone);

        // Hoist the transport request out of the per-scheme loop so that the
        // `target` and `method` String allocations are paid once per outbound
        // interaction rather than once per scheme. The `metadata` buffer is
        // likewise reused across schemes via `clear()` + extend, which keeps
        // the previously allocated BTreeMap node capacity live across scheme
        // iterations and avoids the per-scheme map allocation that
        // `input.parameters.clone()` would otherwise incur.
        let mut request = TransportRequest::new(form.href.as_str(), operation.as_str());
        request.payload = input.payload.take();

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
                .iter()
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

            // Reset the working metadata to a fresh copy of the original
            // parameters so each scheme is applied in isolation. `clear()`
            // keeps the BTreeMap's allocated capacity around for the next
            // iteration's `extend`, avoiding the fresh root-node allocation
            // that `BTreeMap::clone` performs.
            request.metadata.clear();
            request
                .metadata
                .extend(input.parameters.iter().map(|(k, v)| (k.clone(), v.clone())));

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
            // Diff against the original parameters to extract only the
            // security-added metadata.
            for (key, value) in &request.metadata {
                if input.parameters.get(key) != Some(value) {
                    input.security_metadata.insert(key.clone(), value.clone());
                }
            }
        }

        input.payload = request.payload;
        Ok(input)
    }

    fn cached_or_select_binding_plan(
        &self,
        entry: &ConsumedThingEntry,
        thing: &clinkz_wot_td::thing::Thing,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
    ) -> ServientResult<ActiveBindingPlan> {
        let key = SelectedFormCacheKey::new(entry.affordance_target(affordance), criteria);
        let current_generation = self.binding_factories.generation();

        if let Some(plan) = entry.get_plan(&key) {
            // Fast path: the binding factory registry has not changed since
            // this plan was validated, and the TD is immutable for the life of
            // the entry (the entry is invalidated on TD update). Reuse the
            // cached live binding instance (cheap `Arc` clone) — no
            // `make_binding`, no per-call session-handle/buffer construction.
            if plan.factory_generation == current_generation {
                return Ok(ActiveBindingPlan {
                    form: Arc::clone(&plan.form),
                    binding: Arc::clone(&plan.binding),
                });
            }
            // Generation changed (a factory was appended): revalidate the
            // cached plan against the current factory set using the cached
            // binding itself. Factories are append-only, so the cached
            // binding's factory index is still valid; only its `supports` need
            // re-checking. On success, refresh the cached generation.
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

        let form = self.cached_or_select_form(thing, affordance, criteria)?;
        let (binding_factory_index, binding) =
            self.select_binding_factory_for_form(thing, &form, criteria.operation)?;
        let binding: Arc<dyn ClientBinding + Send + Sync> = Arc::from(binding);
        entry.insert_plan(
            key,
            BindingPlan {
                form: Arc::clone(&form),
                binding_factory_index,
                binding: Arc::clone(&binding),
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
        // The cached form was selected from this affordance; use the lightweight
        // operation check instead of the full O(n) membership search. Reuse the
        // cached binding instance (no make_binding): factories are append-only,
        // so the cached binding is still from a valid factory.
        validate_form_operation(thing, affordance, &plan.form, criteria.operation)?;
        if plan
            .binding
            .supports_with_thing(thing, &plan.form, criteria.operation)
        {
            Ok(ActiveBindingPlan {
                form: Arc::clone(&plan.form),
                binding: Arc::clone(&plan.binding),
            })
        } else {
            Err(CoreError::UnsupportedBinding(format!(
                "Cached binding factory {} no longer supports {} for {}",
                plan.binding_factory_index,
                criteria.operation.as_str(),
                plan.form.href.as_str()
            ))
            .into())
        }
    }

    fn cached_or_select_form(
        &self,
        thing: &clinkz_wot_td::thing::Thing,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
    ) -> ServientResult<Arc<Form>> {
        // Reached only on plan-cache miss (the cold path). The selected form is
        // stored in the binding plan, so a separate form cache would just hold
        // the same `Arc<Form>` under the same key — recompute it here instead.
        Ok(Arc::new(
            select_affordance_form_with_criteria(thing, affordance, criteria)?
                .selection
                .form
                .clone(),
        ))
    }

    fn select_binding_factory_for_form(
        &self,
        thing: &clinkz_wot_td::thing::Thing,
        form: &Form,
        operation: Operation,
    ) -> ServientResult<(usize, Box<dyn ClientBinding + Send + Sync>)> {
        let binding_factory_index = self
            .binding_factories
            .find_supporting_index(thing, form, operation)
            .ok_or_else(|| {
                ServientError::from(CoreError::UnsupportedBinding(format!(
                    "No binding supports {} for {}",
                    operation.as_str(),
                    form.href.as_str()
                )))
            })?;
        let binding = self.binding_from_factory_index(binding_factory_index)?;
        debug_assert!(
            binding.supports_with_thing(thing, form, operation),
            "binding factory support predicate accepted a binding that rejected the same form"
        );
        Ok((binding_factory_index, binding))
    }

    fn binding_from_factory_index(
        &self,
        index: usize,
    ) -> ServientResult<Box<dyn ClientBinding + Send + Sync>> {
        self.binding_factories.make_binding(index).ok_or_else(|| {
            CoreError::UnsupportedBinding(format!(
                "Binding factory index {} is not registered",
                index
            ))
            .into()
        })
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
/// The matching codec is looked up under a brief registry lock and then
/// cloned out as a cheap `Arc` clone; the decode+encode round-trip runs
/// *outside* the lock so concurrent interactions are not serialized on codec
/// work. The round-trip itself still allocates two `Vec<u8>` per call; skipping
/// it when the caller's bytes are already canonical would require a separate
/// "validate-only" API on `PayloadCodec` and is tracked as a follow-up.
fn normalize_payload(codecs: &PayloadCodecRegistry, payload: Payload) -> ServientResult<Payload> {
    let codec = codecs.with_read_recover(|codecs| {
        codecs
            .iter()
            .find(|codec| codec.content_type().as_ref() == payload.content_type.as_str())
            .cloned()
    });

    let Some(codec) = codec else {
        return Ok(payload);
    };

    let decoded = codec.decode(&payload)?;
    codec
        .encode(CodecInput {
            body: decoded.as_slice(),
        })
        .map_err(Into::into)
}

fn is_nosec_security(scheme: &SecurityScheme) -> bool {
    matches!(scheme, SecurityScheme::NoSec(_))
}

// ---------------------------------------------------------------------------
// Async sync-binding fallback offload helpers.
//
// `std` + async: a sync-only binding's blocking call is offloaded to the tokio
// blocking pool so the async executor is not stalled. `no_std` + async
// (cooperative single-thread): there is no blocking pool and binding results
// are not `Send`, so the call runs inline. Bindings targeting no_std async
// should implement `AsyncClientBinding` to avoid blocking the executor.
// ---------------------------------------------------------------------------

#[cfg(feature = "async")]
async fn offload_invoke(
    binding: Arc<dyn ClientBinding + Send + Sync>,
    request: BindingRequest,
) -> ServientResult<InteractionOutput> {
    #[cfg(feature = "std")]
    {
        tokio::task::spawn_blocking(move || binding.invoke(request))
            .await
            .map_err(|join_err| {
                ServientError::Accept(format!("blocking invoke task failed: {join_err}"))
            })?
            .map_err(ServientError::from)
    }
    #[cfg(not(feature = "std"))]
    {
        binding.invoke(request).map_err(ServientError::from)
    }
}

#[cfg(feature = "async")]
async fn offload_subscribe(
    binding: Arc<dyn ClientBinding + Send + Sync>,
    request: BindingRequest,
) -> ServientResult<(Subscription, Box<dyn SubscriptionGuard>)> {
    #[cfg(feature = "std")]
    {
        tokio::task::spawn_blocking(move || binding.subscribe(request))
            .await
            .map_err(|join_err| {
                ServientError::Accept(format!("blocking subscribe task failed: {join_err}"))
            })?
            .map_err(ServientError::from)
    }
    #[cfg(not(feature = "std"))]
    {
        binding.subscribe(request).map_err(ServientError::from)
    }
}
