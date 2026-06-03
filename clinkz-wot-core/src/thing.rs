use alloc::{boxed::Box, collections::BTreeMap, format, string::String, vec::Vec};

use clinkz_wot_td::{
    data_type::Operation,
    form::Form,
    td_defaults::{FormContext, effective_form_operations},
    thing::Thing,
};

use crate::{BindingRequest, CoreError, CoreResult, Payload, ProtocolBinding};

/// Location of an affordance within a Thing Description.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AffordanceTarget<'a> {
    /// A form declared at Thing level.
    Thing,
    /// A property affordance by name.
    Property(&'a str),
    /// An action affordance by name.
    Action(&'a str),
    /// An event affordance by name.
    Event(&'a str),
}

/// Input provided to an interaction handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionInput {
    /// Optional encoded payload for write, action, subscription, or cancellation flows.
    pub payload: Option<Payload>,
    /// URI template or protocol binding parameters supplied by the caller.
    pub parameters: BTreeMap<String, String>,
}

impl InteractionInput {
    /// Creates an empty interaction input.
    pub fn empty() -> Self {
        Self {
            payload: None,
            parameters: BTreeMap::new(),
        }
    }

    /// Creates an interaction input containing a payload.
    pub fn with_payload(payload: Payload) -> Self {
        Self {
            payload: Some(payload),
            parameters: BTreeMap::new(),
        }
    }
}

/// Output returned by an interaction handler or consumed Thing call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionOutput {
    /// Optional encoded response payload.
    pub payload: Option<Payload>,
}

impl InteractionOutput {
    /// Creates an empty output.
    pub fn empty() -> Self {
        Self { payload: None }
    }

    /// Creates an output containing a payload.
    pub fn with_payload(payload: Payload) -> Self {
        Self {
            payload: Some(payload),
        }
    }
}

/// Sink used by event subscriptions without requiring an async runtime.
pub trait EventSink {
    /// Accepts the next event payload.
    fn emit(&mut self, payload: Payload) -> CoreResult<()>;
}

/// Handler for a local property affordance.
pub trait PropertyHandler {
    /// Reads the current property value.
    fn read(&mut self, input: InteractionInput) -> CoreResult<InteractionOutput>;

    /// Writes a new property value.
    fn write(&mut self, input: InteractionInput) -> CoreResult<InteractionOutput>;
}

/// Handler for a local action affordance.
pub trait ActionHandler {
    /// Invokes the action.
    fn invoke(&mut self, input: InteractionInput) -> CoreResult<InteractionOutput>;
}

/// Handler for a local event affordance.
pub trait EventHandler {
    /// Subscribes to the event source and may emit initial event payloads.
    fn subscribe(
        &mut self,
        input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> CoreResult<InteractionOutput>;
}

/// Protocol-neutral local Thing dispatcher.
pub struct LocalThing {
    thing: Thing,
    property_handlers: BTreeMap<String, Box<dyn PropertyHandler>>,
    action_handlers: BTreeMap<String, Box<dyn ActionHandler>>,
    event_handlers: BTreeMap<String, Box<dyn EventHandler>>,
}

/// Protocol-neutral dispatcher for consuming a remote Thing through bindings.
pub struct BoundConsumedThing {
    thing: Thing,
    bindings: Vec<Box<dyn ProtocolBinding>>,
}

impl BoundConsumedThing {
    /// Creates a consumed Thing dispatcher for a Thing Description.
    pub fn new(thing: Thing) -> Self {
        Self {
            thing,
            bindings: Vec::new(),
        }
    }

    /// Returns the Thing Description owned by this dispatcher.
    pub fn thing_description(&self) -> &Thing {
        &self.thing
    }

    /// Registers a protocol binding.
    pub fn register_binding(&mut self, binding: impl ProtocolBinding + 'static) {
        self.bindings.push(Box::new(binding));
    }

    fn forms_for_target<'a>(&'a self, target: AffordanceTarget<'_>) -> CoreResult<FormSet<'a>> {
        match target {
            AffordanceTarget::Thing => Ok(FormSet {
                context: FormContext::Thing,
                forms: self.thing.forms.as_deref().unwrap_or(&[]),
            }),
            AffordanceTarget::Property(name) => {
                let property = find_affordance("property", name, &self.thing.properties)?;
                Ok(FormSet {
                    context: FormContext::Property(property),
                    forms: property._interaction.forms.as_slice(),
                })
            }
            AffordanceTarget::Action(name) => {
                let action = find_affordance("action", name, &self.thing.actions)?;
                Ok(FormSet {
                    context: FormContext::Action(action),
                    forms: action._interaction.forms.as_slice(),
                })
            }
            AffordanceTarget::Event(name) => {
                let event = find_affordance("event", name, &self.thing.events)?;
                Ok(FormSet {
                    context: FormContext::Event(event),
                    forms: event._interaction.forms.as_slice(),
                })
            }
        }
    }

    fn validate_selected_form(
        &self,
        target: AffordanceTarget<'_>,
        operation: Operation,
        form: &Form,
    ) -> CoreResult<()> {
        let form_set = self.forms_for_target(target)?;
        let selected = form_set
            .forms
            .iter()
            .find(|candidate| *candidate == form)
            .ok_or_else(|| {
                CoreError::InvalidInteraction(
                    "Selected form does not belong to the requested affordance".into(),
                )
            })?;

        if effective_form_operations(form_set.context, selected)
            .iter()
            .any(|candidate| *candidate == operation)
        {
            Ok(())
        } else {
            Err(CoreError::UnsupportedOperation(format!(
                "Form does not support {:?}",
                operation
            )))
        }
    }
}

struct FormSet<'a> {
    context: FormContext<'a>,
    forms: &'a [Form],
}

fn find_affordance<'a, T>(
    kind: &'static str,
    name: &str,
    affordances: &'a Option<BTreeMap<String, T>>,
) -> CoreResult<&'a T> {
    affordances
        .as_ref()
        .and_then(|affordances| affordances.get(name))
        .ok_or_else(|| CoreError::UnknownAffordance {
            kind,
            name: name.into(),
        })
}

impl ConsumedThing for BoundConsumedThing {
    fn thing_description(&self) -> &Thing {
        &self.thing
    }

    fn request(
        &mut self,
        target: AffordanceTarget<'_>,
        operation: Operation,
        form: &Form,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.validate_selected_form(target, operation, form)?;

        let binding = self
            .bindings
            .iter_mut()
            .find(|binding| binding.supports(form, operation))
            .ok_or_else(|| {
                CoreError::UnsupportedBinding(format!(
                    "No binding supports {:?} for {}",
                    operation,
                    form.href.as_str()
                ))
            })?;

        binding.invoke(BindingRequest {
            thing: &self.thing,
            target,
            operation,
            form,
            input,
        })
    }
}

impl LocalThing {
    /// Creates a local dispatcher for a Thing Description.
    pub fn new(thing: Thing) -> Self {
        Self {
            thing,
            property_handlers: BTreeMap::new(),
            action_handlers: BTreeMap::new(),
            event_handlers: BTreeMap::new(),
        }
    }

    /// Returns the Thing Description owned by this dispatcher.
    pub fn thing_description(&self) -> &Thing {
        &self.thing
    }

    /// Registers a property handler by affordance name.
    pub fn register_property_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl PropertyHandler + 'static,
    ) -> Option<Box<dyn PropertyHandler>> {
        self.property_handlers
            .insert(name.into(), Box::new(handler))
    }

    /// Registers an action handler by affordance name.
    pub fn register_action_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl ActionHandler + 'static,
    ) -> Option<Box<dyn ActionHandler>> {
        self.action_handlers.insert(name.into(), Box::new(handler))
    }

    /// Registers an event handler by affordance name.
    pub fn register_event_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl EventHandler + 'static,
    ) -> Option<Box<dyn EventHandler>> {
        self.event_handlers.insert(name.into(), Box::new(handler))
    }

    fn ensure_property_affordance(&self, name: &str) -> CoreResult<()> {
        ensure_affordance("property", name, &self.thing.properties)
    }

    fn ensure_action_affordance(&self, name: &str) -> CoreResult<()> {
        ensure_affordance("action", name, &self.thing.actions)
    }

    fn ensure_event_affordance(&self, name: &str) -> CoreResult<()> {
        ensure_affordance("event", name, &self.thing.events)
    }
}

impl ExposedThing for LocalThing {
    fn thing_description(&self) -> &Thing {
        &self.thing
    }

    fn read_property(
        &mut self,
        name: &str,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.ensure_property_affordance(name)?;
        let handler = self.property_handlers.get_mut(name).ok_or_else(|| {
            CoreError::InvalidInteraction(format!("No property handler registered for '{}'", name))
        })?;
        handler.read(input)
    }

    fn write_property(
        &mut self,
        name: &str,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.ensure_property_affordance(name)?;
        let handler = self.property_handlers.get_mut(name).ok_or_else(|| {
            CoreError::InvalidInteraction(format!("No property handler registered for '{}'", name))
        })?;
        handler.write(input)
    }

    fn invoke_action(
        &mut self,
        name: &str,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.ensure_action_affordance(name)?;
        let handler = self.action_handlers.get_mut(name).ok_or_else(|| {
            CoreError::InvalidInteraction(format!("No action handler registered for '{}'", name))
        })?;
        handler.invoke(input)
    }

    fn subscribe_event(
        &mut self,
        name: &str,
        input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> CoreResult<InteractionOutput> {
        self.ensure_event_affordance(name)?;
        let handler = self.event_handlers.get_mut(name).ok_or_else(|| {
            CoreError::InvalidInteraction(format!("No event handler registered for '{}'", name))
        })?;
        handler.subscribe(input, sink)
    }
}

fn ensure_affordance<T>(
    kind: &'static str,
    name: &str,
    affordances: &Option<BTreeMap<String, T>>,
) -> CoreResult<()> {
    if affordances
        .as_ref()
        .is_some_and(|affordances| affordances.contains_key(name))
    {
        Ok(())
    } else {
        Err(CoreError::UnknownAffordance {
            kind,
            name: name.into(),
        })
    }
}

/// Protocol-neutral interface implemented by locally exposed Things.
pub trait ExposedThing {
    /// Returns the Thing Description that describes this exposed Thing.
    fn thing_description(&self) -> &Thing;

    /// Reads a property.
    fn read_property(
        &mut self,
        name: &str,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput>;

    /// Writes a property.
    fn write_property(
        &mut self,
        name: &str,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput>;

    /// Invokes an action.
    fn invoke_action(
        &mut self,
        name: &str,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput>;

    /// Subscribes to an event source.
    fn subscribe_event(
        &mut self,
        name: &str,
        input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> CoreResult<InteractionOutput>;
}

/// Protocol-neutral interface for consuming a remote Thing through bindings.
pub trait ConsumedThing {
    /// Returns the Thing Description used by this consumed Thing.
    fn thing_description(&self) -> &Thing;

    /// Performs an operation against a selected affordance form.
    fn request(
        &mut self,
        target: AffordanceTarget<'_>,
        operation: Operation,
        form: &Form,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput>;
}
