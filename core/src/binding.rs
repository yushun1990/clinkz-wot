use alloc::boxed::Box;

use clinkz_wot_td::{data_type::Operation, form::Form, thing::Thing};

use crate::{AffordanceTarget, CoreResult, InteractionInput, InteractionOutput};

/// Request passed from the core runtime to a protocol binding.
pub struct BindingRequest<'a> {
    /// Thing Description that owns the selected form.
    pub thing: &'a Thing,
    /// Affordance location for the selected form.
    pub target: AffordanceTarget<'a>,
    /// Effective operation being performed.
    pub operation: Operation,
    /// Selected TD form.
    pub form: &'a Form,
    /// Caller input.
    pub input: InteractionInput,
}

/// Protocol binding contract shared by all concrete bindings.
pub trait ProtocolBinding {
    /// Returns true when this binding can handle the selected form and operation.
    fn supports(&self, form: &Form, operation: Operation) -> bool;

    /// Returns true when this binding can handle the selected form, Thing, and operation.
    ///
    /// The default implementation falls back to [`ProtocolBinding::supports`].
    fn supports_with_thing(&self, thing: &Thing, form: &Form, operation: Operation) -> bool {
        let _ = thing;
        self.supports(form, operation)
    }

    /// Performs the requested interaction through the concrete protocol.
    fn invoke(&mut self, request: BindingRequest<'_>) -> CoreResult<InteractionOutput>;
}

impl ProtocolBinding for Box<dyn ProtocolBinding> {
    fn supports(&self, form: &Form, operation: Operation) -> bool {
        self.as_ref().supports(form, operation)
    }

    fn supports_with_thing(&self, thing: &Thing, form: &Form, operation: Operation) -> bool {
        self.as_ref().supports_with_thing(thing, form, operation)
    }

    fn invoke(&mut self, request: BindingRequest<'_>) -> CoreResult<InteractionOutput> {
        self.as_mut().invoke(request)
    }
}
