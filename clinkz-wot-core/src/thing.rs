use alloc::{collections::BTreeMap, string::String};

use clinkz_wot_td::{data_type::Operation, form::Form, thing::Thing};

use crate::{CoreResult, Payload};

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
