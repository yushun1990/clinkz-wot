use alloc::{boxed::Box, format};

use clinkz_wot_core::{
    AffordanceTarget, BoundConsumedThing, CodecInput, ConsumedThing, CoreError, EventSink,
    ExposedThing, InteractionInput, InteractionOutput, Payload, PayloadCodec, ProtocolBinding,
    SecurityContext, TransportRequest,
};
use clinkz_wot_discovery::ThingDirectory;
use clinkz_wot_protocol_bindings::{
    resolve_form_security, select_affordance_form_with_criteria,
    validate_affordance_form_with_criteria, AffordanceRef, FormSelectionCriteria,
};
use clinkz_wot_td::{
    data_type::Operation, form::Form, security_scheme::SecurityScheme, thing::Thing,
};

use crate::{
    BindingPlan, BindingPlanCache, ConsumedThingCache, ExposedThingRegistry, SelectedFormCache,
    SelectedFormCacheAffordance, SelectedFormCacheKey, Servient, ServientError, ServientResult,
};

struct ActiveBindingPlan {
    form: Form,
    binding: Box<dyn ProtocolBinding>,
}

impl<D, R, C, S, P> Servient<D, R, C, S, P>
where
    D: ThingDirectory,
    R: ExposedThingRegistry,
    C: ConsumedThingCache,
    S: SelectedFormCache,
    P: BindingPlanCache,
{
    /// Reads a property on a locally exposed Thing.
    pub fn read_property(
        &mut self,
        id: &str,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        let thing = self.local_thing_description(id)?;
        let form = self.select_interaction_form(
            &thing,
            AffordanceRef::Property(name),
            Operation::ReadProperty,
        )?;
        let input =
            self.prepare_interaction_input(&thing, form.as_ref(), Operation::ReadProperty, input)?;
        let output = self
            .exposed_thing_mut(id)?
            .read_property(name, input)
            .map_err(ServientError::from)?;
        self.prepare_interaction_output(output)
    }

    /// Writes a property on a locally exposed Thing.
    pub fn write_property(
        &mut self,
        id: &str,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        let thing = self.local_thing_description(id)?;
        let form = self.select_interaction_form(
            &thing,
            AffordanceRef::Property(name),
            Operation::WriteProperty,
        )?;
        let input =
            self.prepare_interaction_input(&thing, form.as_ref(), Operation::WriteProperty, input)?;
        let output = self
            .exposed_thing_mut(id)?
            .write_property(name, input)
            .map_err(ServientError::from)?;
        self.prepare_interaction_output(output)
    }

    /// Invokes an action on a locally exposed Thing.
    pub fn invoke_action(
        &mut self,
        id: &str,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        let thing = self.local_thing_description(id)?;
        let form = self.select_interaction_form(
            &thing,
            AffordanceRef::Action(name),
            Operation::InvokeAction,
        )?;
        let input =
            self.prepare_interaction_input(&thing, form.as_ref(), Operation::InvokeAction, input)?;
        let output = self
            .exposed_thing_mut(id)?
            .invoke_action(name, input)
            .map_err(ServientError::from)?;
        self.prepare_interaction_output(output)
    }

    /// Subscribes to an event on a locally exposed Thing.
    pub fn subscribe_event(
        &mut self,
        id: &str,
        name: &str,
        input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> ServientResult<InteractionOutput> {
        let thing = self.local_thing_description(id)?;
        let form = self.select_interaction_form(
            &thing,
            AffordanceRef::Event(name),
            Operation::SubscribeEvent,
        )?;
        let input = self.prepare_interaction_input(
            &thing,
            form.as_ref(),
            Operation::SubscribeEvent,
            input,
        )?;
        let output = self
            .exposed_thing_mut(id)?
            .subscribe_event(name, input, sink)
            .map_err(ServientError::from)?;
        self.prepare_interaction_output(output)
    }

    /// Creates a consumed Thing dispatcher from a directory entry.
    pub fn consume(&self, id: &str) -> ServientResult<BoundConsumedThing> {
        let thing = self.consumed_thing_description(id)?;
        Ok(self.bound_consumed_thing(thing))
    }

    /// Creates a consumed Thing dispatcher directly from a TD.
    pub fn consume_thing(&self, thing: Thing) -> BoundConsumedThing {
        self.bound_consumed_thing(thing)
    }

    /// Reads a property on a remote Thing through a caller-selected form.
    pub fn read_remote_property(
        &mut self,
        id: &str,
        name: &str,
        form: &Form,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.request_remote_selected_form(
            id,
            AffordanceTarget::Property(name),
            Operation::ReadProperty,
            form,
            input,
        )
    }

    /// Reads a property on a remote Thing through the first form matching criteria.
    pub fn read_remote_property_with_criteria(
        &mut self,
        id: &str,
        name: &str,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.request_remote_with_criteria(
            id,
            AffordanceTarget::Property(name),
            AffordanceRef::Property(name),
            criteria_for_operation(criteria, Operation::ReadProperty),
            input,
        )
    }

    /// Writes a property on a remote Thing through a caller-selected form.
    pub fn write_remote_property(
        &mut self,
        id: &str,
        name: &str,
        form: &Form,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.request_remote_selected_form(
            id,
            AffordanceTarget::Property(name),
            Operation::WriteProperty,
            form,
            input,
        )
    }

    /// Writes a property on a remote Thing through the first form matching criteria.
    pub fn write_remote_property_with_criteria(
        &mut self,
        id: &str,
        name: &str,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.request_remote_with_criteria(
            id,
            AffordanceTarget::Property(name),
            AffordanceRef::Property(name),
            criteria_for_operation(criteria, Operation::WriteProperty),
            input,
        )
    }

    /// Invokes an action on a remote Thing through a caller-selected form.
    pub fn invoke_remote_action(
        &mut self,
        id: &str,
        name: &str,
        form: &Form,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.request_remote_selected_form(
            id,
            AffordanceTarget::Action(name),
            Operation::InvokeAction,
            form,
            input,
        )
    }

    /// Invokes an action on a remote Thing through the first form matching criteria.
    pub fn invoke_remote_action_with_criteria(
        &mut self,
        id: &str,
        name: &str,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.request_remote_with_criteria(
            id,
            AffordanceTarget::Action(name),
            AffordanceRef::Action(name),
            criteria_for_operation(criteria, Operation::InvokeAction),
            input,
        )
    }

    /// Subscribes to a remote event through a caller-selected form.
    pub fn subscribe_remote_event(
        &mut self,
        id: &str,
        name: &str,
        form: &Form,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.request_remote_selected_form(
            id,
            AffordanceTarget::Event(name),
            Operation::SubscribeEvent,
            form,
            input,
        )
    }

    /// Subscribes to a remote event through the first form matching criteria.
    pub fn subscribe_remote_event_with_criteria(
        &mut self,
        id: &str,
        name: &str,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.request_remote_with_criteria(
            id,
            AffordanceTarget::Event(name),
            AffordanceRef::Event(name),
            criteria_for_operation(criteria, Operation::SubscribeEvent),
            input,
        )
    }

    fn request_remote_with_criteria(
        &mut self,
        id: &str,
        target: AffordanceTarget<'_>,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        let thing = self.consumed_thing_description(id)?;
        let active_plan = self.cached_or_select_binding_plan(&thing, id, affordance, criteria)?;
        let input = self.prepare_interaction_input(
            &thing,
            Some(&active_plan.form),
            criteria.operation,
            input,
        )?;
        let mut consumed = self.bound_consumed_thing_with_binding(thing, active_plan.binding);

        let output = consumed
            .request(target, criteria.operation, &active_plan.form, input)
            .map_err(ServientError::from)?;
        self.prepare_interaction_output(output)
    }

    fn request_remote_selected_form(
        &mut self,
        id: &str,
        target: AffordanceTarget<'_>,
        operation: Operation,
        form: &Form,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        let thing = self.consumed_thing_description(id)?;
        let input = self.prepare_interaction_input(&thing, Some(form), operation, input)?;
        let mut consumed = self.bound_consumed_thing(thing);
        let output = consumed
            .request(target, operation, form, input)
            .map_err(ServientError::from)?;
        self.prepare_interaction_output(output)
    }

    fn local_thing_description(&mut self, id: &str) -> ServientResult<Thing> {
        self.exposed_thing_mut(id)
            .map(|thing| thing.thing_description().clone())
    }

    fn select_interaction_form(
        &self,
        thing: &Thing,
        affordance: AffordanceRef<'_>,
        operation: Operation,
    ) -> ServientResult<Option<Form>> {
        if thing_has_no_affordance_forms(thing, affordance) {
            return Ok(None);
        }

        select_affordance_form_with_criteria(
            thing,
            affordance,
            FormSelectionCriteria::new(operation),
        )
        .map(|selected| Some(selected.selection.form.clone()))
        .map_err(Into::into)
    }

    fn prepare_interaction_input(
        &mut self,
        thing: &Thing,
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
        &mut self,
        thing: &Thing,
        form: &Form,
        operation: Operation,
        mut input: InteractionInput,
    ) -> ServientResult<InteractionInput> {
        let effective_security = resolve_form_security(thing, form);
        for scheme_name in effective_security.security {
            let scheme = thing.security_definitions.get(scheme_name).ok_or_else(|| {
                CoreError::Security(format!(
                    "Security definition '{}' is not declared",
                    scheme_name
                ))
            })?;

            if is_nosec_security(scheme) {
                continue;
            }

            let provider = self
                .security_providers
                .iter_mut()
                .find(|provider| provider.scheme_name() == scheme_name)
                .ok_or_else(|| {
                    CoreError::Security(format!(
                        "No security provider registered for '{}'",
                        scheme_name
                    ))
                })?;

            if !provider.supports_scopes(effective_security.scopes) {
                return Err(CoreError::Security(format!(
                    "Security provider '{}' does not support scopes {:?}",
                    scheme_name, effective_security.scopes
                ))
                .into());
            }

            // Servient does not own a concrete transport request here, so
            // provider metadata is carried through the protocol-neutral
            // interaction parameters visible to handlers and bindings.
            let mut request = TransportRequest::new(form.href.as_str(), format!("{:?}", operation));
            request.metadata = input.parameters;
            request.payload = input.payload;
            provider.apply(
                SecurityContext {
                    thing,
                    form,
                    scheme_name,
                    scheme,
                },
                &mut request,
            )?;
            input = InteractionInput {
                payload: request.payload,
                parameters: request.metadata,
            };
        }

        Ok(input)
    }

    fn cached_or_select_binding_plan(
        &self,
        thing: &Thing,
        id: &str,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
    ) -> ServientResult<ActiveBindingPlan> {
        let key = SelectedFormCacheKey::new(
            id,
            SelectedFormCacheAffordance::from_affordance_ref(affordance),
            criteria,
        );

        if let Some(plan) = self.binding_plan_cache.get(&key) {
            match self.active_binding_plan_from_cache(thing, affordance, criteria, plan) {
                Ok(active_plan) => return Ok(active_plan),
                Err(_) => {
                    self.binding_plan_cache.remove(&key);
                }
            }
        }

        let form = self.cached_or_select_form(thing, id, affordance, criteria)?;
        let (binding_factory_index, binding) =
            self.select_binding_factory_for_form(&form, criteria.operation)?;
        self.binding_plan_cache.insert(
            key,
            BindingPlan {
                form: form.clone(),
                binding_factory_index,
            },
        );

        Ok(ActiveBindingPlan { form, binding })
    }

    fn active_binding_plan_from_cache(
        &self,
        thing: &Thing,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
        plan: BindingPlan,
    ) -> ServientResult<ActiveBindingPlan> {
        validate_affordance_form_with_criteria(thing, affordance, &plan.form, criteria)?;
        let binding = self.binding_from_factory_index(plan.binding_factory_index)?;
        if binding.supports(&plan.form, criteria.operation) {
            Ok(ActiveBindingPlan {
                form: plan.form,
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
        thing: &Thing,
        id: &str,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
    ) -> ServientResult<Form> {
        let key = SelectedFormCacheKey::new(
            id,
            SelectedFormCacheAffordance::from_affordance_ref(affordance),
            criteria,
        );

        if let Some(form) = self.selected_form_cache.get(&key) {
            if validate_affordance_form_with_criteria(thing, affordance, &form, criteria).is_ok() {
                return Ok(form);
            }
            self.selected_form_cache.remove(&key);
        }

        let form = select_affordance_form_with_criteria(thing, affordance, criteria)?
            .selection
            .form
            .clone();
        self.selected_form_cache.insert(key, form.clone());
        Ok(form)
    }

    fn select_binding_factory_for_form(
        &self,
        form: &Form,
        operation: Operation,
    ) -> ServientResult<(usize, Box<dyn ProtocolBinding>)> {
        for (index, factory) in self.binding_factories.iter().enumerate() {
            let binding = factory();
            if binding.supports(form, operation) {
                return Ok((index, binding));
            }
        }

        Err(CoreError::UnsupportedBinding(format!(
            "No binding supports {:?} for {}",
            operation,
            form.href.as_str()
        ))
        .into())
    }

    fn binding_from_factory_index(&self, index: usize) -> ServientResult<Box<dyn ProtocolBinding>> {
        self.binding_factories
            .get(index)
            .map(|factory| factory())
            .ok_or_else(|| {
                CoreError::UnsupportedBinding(format!(
                    "Binding factory index {} is not registered",
                    index
                ))
                .into()
            })
    }

    fn consumed_thing_description(&self, id: &str) -> ServientResult<Thing> {
        match self.consumed_cache.get(id) {
            Some(thing) => Ok(thing),
            None => self.directory.get(id).map_err(Into::into),
        }
    }

    fn bound_consumed_thing(&self, thing: Thing) -> BoundConsumedThing {
        let mut consumed = BoundConsumedThing::new(thing);
        for factory in &self.binding_factories {
            consumed.register_binding(factory());
        }
        consumed
    }

    fn bound_consumed_thing_with_binding(
        &self,
        thing: Thing,
        binding: Box<dyn ProtocolBinding>,
    ) -> BoundConsumedThing {
        let mut consumed = BoundConsumedThing::new(thing);
        consumed.register_binding(binding);
        consumed
    }
}

fn criteria_for_operation<'a>(
    criteria: FormSelectionCriteria<'a>,
    operation: Operation,
) -> FormSelectionCriteria<'a> {
    FormSelectionCriteria {
        operation,
        content_type: criteria.content_type,
        subprotocol: criteria.subprotocol,
    }
}

fn normalize_interaction_input(
    codecs: &[Box<dyn PayloadCodec>],
    mut input: InteractionInput,
) -> ServientResult<InteractionInput> {
    if let Some(payload) = input.payload.take() {
        input.payload = Some(normalize_payload(codecs, payload)?);
    }

    Ok(input)
}

fn normalize_interaction_output(
    codecs: &[Box<dyn PayloadCodec>],
    mut output: InteractionOutput,
) -> ServientResult<InteractionOutput> {
    if let Some(payload) = output.payload.take() {
        output.payload = Some(normalize_payload(codecs, payload)?);
    }

    Ok(output)
}

fn normalize_payload(
    codecs: &[Box<dyn PayloadCodec>],
    payload: Payload,
) -> ServientResult<Payload> {
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
}

fn is_nosec_security(scheme: &SecurityScheme) -> bool {
    scheme.scheme() == "nosec"
}

fn thing_has_no_affordance_forms(thing: &Thing, affordance: AffordanceRef<'_>) -> bool {
    match affordance {
        AffordanceRef::Thing => match &thing.forms {
            Some(forms) => forms.is_empty(),
            None => true,
        },
        AffordanceRef::Property(name) => thing
            .properties
            .as_ref()
            .and_then(|affordances| affordances.get(name))
            .map(|property| property._interaction.forms.is_empty())
            .unwrap_or(false),
        AffordanceRef::Action(name) => thing
            .actions
            .as_ref()
            .and_then(|affordances| affordances.get(name))
            .map(|action| action._interaction.forms.is_empty())
            .unwrap_or(false),
        AffordanceRef::Event(name) => thing
            .events
            .as_ref()
            .and_then(|affordances| affordances.get(name))
            .map(|event| event._interaction.forms.is_empty())
            .unwrap_or(false),
    }
}
