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

/// Application permission for one root status query. Per-invocation permission
/// is additionally checked against each original Action handler/principal.
/// The complete callback is cooperative and included in paid HandlerSteps.
pub trait ActionQueryPolicy {
    fn authorize(&self, input: &InteractionInput) -> CoreResult<()>;
}

/// Source-independent Action definitions used to validate a status collection.
/// Output describes completed application data, never the status envelope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionMemberSchema {
    pub name: alloc::boxed::Box<str>,
    pub output: Option<crate::value_type::TypeSchema>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionCollectionSchema {
    members: alloc::vec::Vec<ActionMemberSchema>,
}
impl ActionCollectionSchema {
    pub const MAX_MEMBERS: usize = 32;
    pub const MAX_TOTAL_NAME_BYTES: usize = 4096;
    pub fn try_new(members: alloc::vec::Vec<ActionMemberSchema>) -> CoreResult<Self> {
        if members.len() > Self::MAX_MEMBERS
            || members.capacity() > Self::MAX_MEMBERS
            || members.iter().map(|m| m.name.len()).sum::<usize>() > Self::MAX_TOTAL_NAME_BYTES
            || members.iter().enumerate().any(|(i, m)| {
                m.name.len() > 256 || members[..i].iter().any(|old| old.name == m.name)
            })
        {
            return Err(crate::CoreError::Validation(crate::ErrorContext::new(
                crate::ErrorPhase::Admission,
                crate::RetryClass::Never,
            )));
        }
        Ok(Self { members })
    }
    pub fn members(&self) -> &[ActionMemberSchema] {
        &self.members
    }
    pub fn retained_bytes(&self) -> usize {
        self.members.capacity() * core::mem::size_of::<ActionMemberSchema>()
            + self.members.iter().map(|m| m.name.len()).sum::<usize>()
    }
}

/// A candidate observation. The native owner retains the input/reply opportunity;
/// constructing this value does not grant application dispatch. Raw callers
/// supply mapping associations; this type does not verify TD coverage or
/// Binding readiness, which publication must qualify separately. A published
/// Servient checks supplied invoke and control routes against captured mappings;
/// explicitly raw registrations have no such TD guarantee.
pub struct RawActionRequest<'a> {
    pub instance: ProducerInstanceKey,
    pub target: &'a AffordanceTarget,
    pub invoke: ActionRoute,
    pub query: Option<ActionRoute>,
    pub cancel: Option<ActionRoute>,
    pub input: &'a InteractionInput,
}

/// A published Action's required service is captured by the logical Servient
/// at qualification, never selected by an individual native request. Raw
/// instances remain explicitly unvalidated beyond their shape/backing checks.
pub trait ActionValidator {
    fn validate_input(&self, input: &InteractionInput) -> CoreResult<()>;
    fn validate_output(&self, output: &InteractionOutput) -> CoreResult<()>;
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
        && output.metadata().property_write_report().is_none()
}
