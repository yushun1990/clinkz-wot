//! Raw Producer Action semantics. An invocation is not a finite exchange or a
//! native session: the Binding keeps the initiating reply and wire correlation.
//! These contracts make no TD qualification or schema-validation claim.
use alloc::boxed::Box;
use core::task::Poll;

use crate::{
    AffordanceTarget, BindingGeneration, BindingId, CoreResult, HandlerContext, InteractionInput,
    InteractionOutput, Operation, producer::ProducerInstanceKey,
};

/// Binding-supplied, opaque identity for the selected Action mapping within a
/// registration. This is not a URL, request correlation, or permission token.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActionMappingKey(pub u64);

/// One described control/initiating mapping, selected by the caller's actual
/// Binding. Query and cancel may have DIFFERENT routes, including a different
/// Binding, from invoke. No native continuation is synthesized from this key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActionRoute {
    pub binding: (BindingId, BindingGeneration),
    pub mapping: ActionMappingKey,
}

/// A candidate observation. The native owner retains the input/reply opportunity;
/// constructing this value does not grant application dispatch. Raw callers
/// supply mapping associations; this type does not verify TD coverage or
/// Binding readiness, which publication must qualify separately.
pub struct RawActionRequest<'a> {
    pub instance: ProducerInstanceKey,
    pub target: &'a AffordanceTarget,
    pub invoke: ActionRoute,
    pub query: Option<ActionRoute>,
    pub cancel: Option<ActionRoute>,
    pub input: &'a InteractionInput,
}

/// Cooperative, application-owned continuation. `request_cancel` is a request,
/// not proof of termination. `poll_close` retires a stalled continuation when
/// the owner explicitly closes it. Pending and errors retain the complete owner.
/// `abort_local` is the mandatory Host-root fallback if the owner itself drops:
/// it consumes the complete application continuation and MUST perform finite,
/// fixed-bound, nonblocking local abort (including its destructor), without
/// waiting for network/user progress or escaping live work. It MUST
/// NOT claim remote cancellation or delivery. Implementations unable to provide
/// that guarantee cannot be admitted by this raw Host kernel; a longer-lived
/// acknowledged cleanup owner would be needed instead. Linked code and its
/// destructor remain trusted contracts, not preemptible callbacks.
pub trait RawActionWork {
    fn poll(&mut self) -> Poll<CoreResult<InteractionOutput>>;
    fn request_cancel(&mut self) -> CoreResult<()>;
    fn poll_close(&mut self) -> Poll<CoreResult<()>>;
    fn abort_local(self: Box<Self>);
}

pub enum RawActionStart {
    Complete(InteractionOutput),
    Continue(Box<dyn RawActionWork>),
}

/// After application entry the effect is not proven absent by cancellation,
/// malformed output, a handler error or failed native delivery. No implicit
/// replay is authorized by this fact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionEffect {
    PossiblyApplied,
}

/// Application behavior, never lent to a Binding. Authorization is performed
/// afresh for invoke, query and cancel. Linked callbacks must be cooperative;
/// the declared work footprint is a trusted application contract.
pub trait RawActionHandler {
    fn authorize(&self, operation: Operation, input: &InteractionInput) -> CoreResult<()>;
    fn invoke(
        &self,
        context: HandlerContext<'_>,
        input: &InteractionInput,
    ) -> CoreResult<RawActionStart>;
}

/// Raw Action output shape; only completed application output is accepted.
/// Continuing state and invocation references are sealed by the Servient,
/// never trusted from handler metadata or a Binding response.
pub fn raw_action_output_shape(output: &InteractionOutput) -> bool {
    use crate::{InteractionStatus, ResponsePayloadRole};
    output.status() == InteractionStatus::Ok
        && output.metadata().payload_role() == ResponsePayloadRole::Application
        && output.metadata().action_invocation().is_none()
        && output.metadata().binding_response().is_none()
}
