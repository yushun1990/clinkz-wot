//! Continuing Consumer Action transitions. This state owns no native exchange,
//! continuation, result, authority, account, registry or wake mechanism.
use super::ValueExecution;
use super::read::LocalConsumerRef;
use crate::{
    BindingGeneration, BindingId, CoreError, CoreResult, Deadline, InteractionOutputView,
    Operation, PayloadView,
};
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsumerActionState {
    Invoking,
    Pending,
    Querying,
    Cancelling,
    Closing,
    /// Validated native completion; acceptance still requires a fresh handoff.
    Completed,
    Failed,
    Closed,
}
impl ConsumerActionState {
    pub fn settlement_ready(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Closed)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsumerActionCancellation {
    NotRequested,
    Uncertain,
    Requested,
    AlreadyTerminal,
}

/// Only the semantic observation kind crosses this boundary. Actual tokens,
/// values and failures have already moved into their original driver's storage.
#[derive(Clone, Copy)]
pub enum ActionObservation {
    Invalid,
    Accepted,
    Pending,
    Complete,
    Failed,
    CancelRequested,
    AlreadyTerminal,
}

/// Native facts with concrete, representation-local owners. A continuation is
/// opaque to the Engine and is never an authorization token.
// CoreError intentionally remains inline; this facet requires no allocator.
#[allow(clippy::large_enum_variant)]
pub enum ActionReply<C, O> {
    Accepted(C),
    Pending,
    Complete(O),
    Failed(CoreError),
    CancelRequested,
    AlreadyTerminal,
}

/// Store the original owners before semantic observation or a linked callback.
pub fn retain_action_reply<C, O>(
    operation: Operation,
    reply: ActionReply<C, O>,
    continuation: &mut Option<C>,
    result: &mut Option<CoreResult<O>>,
) -> ActionObservation {
    match reply {
        ActionReply::Accepted(token) if operation == Operation::InvokeAction => {
            *continuation = Some(token);
            ActionObservation::Accepted
        }
        ActionReply::Accepted(_) => ActionObservation::Invalid,
        ActionReply::Complete(output) => {
            *result = Some(Ok(output));
            ActionObservation::Complete
        }
        ActionReply::Failed(error) => {
            *result = Some(Err(error));
            ActionObservation::Failed
        }
        ActionReply::Pending => ActionObservation::Pending,
        ActionReply::CancelRequested => ActionObservation::CancelRequested,
        ActionReply::AlreadyTerminal => ActionObservation::AlreadyTerminal,
    }
}
pub trait ActionOutput {
    /// One bounded local view-adapter operation, with no native progress. Safe
    /// interior mutability may select different immutable backing between calls.
    /// A view is untrusted: application access must validate the exact loan it
    /// returns, including bytes, media, coding, status and metadata.
    fn view(&self) -> InteractionOutputView<'_>;
}
pub trait ActionInput {
    /// Lend encoded input through a bounded local view adapter. Admission
    /// captures one view after claiming its result slot, before fresh permission,
    /// then reuses that loan for validation and start.
    /// A later view is not proof of the previously validated bytes or metadata.
    fn view(&self) -> PayloadView<'_>;
}
/// Drop performs finite local disposal without waiting for a remote peer.
/// Ready/error does not waive close. Close errors retain the original owner.
pub trait ActionExchange {
    type Continuation;
    type Output: ActionOutput;
    fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<CoreResult<ActionReply<Self::Continuation, Self::Output>>>;
    // Keep concrete original token/output types visible at the native boundary.
    #[allow(clippy::type_complexity)]
    fn poll_close(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<CoreResult<Option<ActionReply<Self::Continuation, Self::Output>>>>;
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActionMapping<'a> {
    pub binding: BindingId,
    pub generation: BindingGeneration,
    pub target: &'a str,
}
/// Concrete source-free native bundle for one continuing Action. Programs,
/// continuation types and exchange storage need not match the Host facet.
/// Configuration includes all mapping/continuation-compatibility dependencies.
/// Metadata/support inspection is bounded and effect-free. A refused start
/// leaves no native effects or partially retained work.
pub trait ActionBinding {
    type Program;
    type Configuration: Eq;
    type Exchange: ActionExchange;
    /// A bounded owned or program-borrowed destination. Its exact text stays
    /// immutable through exchange and continuation retirement. AsRef and
    /// disposal are bounded local operations.
    type Destination: AsRef<str>;
    fn configuration(&self) -> &Self::Configuration;
    fn mapping<'a>(
        &'a self,
        program: &'a Self::Program,
        operation: Operation,
    ) -> Option<ActionMapping<'a>>;
    /// Pure actual destination, checked before policy; start must use this same
    /// destination. Redirects need equivalent explicit policy coverage.
    fn destination(
        &self,
        program: &Self::Program,
        operation: Operation,
        continuation: Option<&<Self::Exchange as ActionExchange>::Continuation>,
    ) -> CoreResult<Self::Destination>;
    fn supports(&self, program: &Self::Program, meaning: ValueExecution<'_>) -> bool;
    /// Secure native correlation, reply and retirement backing before an
    /// effect. The exchange owns its concrete types and any indirect backing.
    fn start(
        &self,
        program: &Self::Program,
        operation: Operation,
        destination: &Self::Destination,
        input: Option<PayloadView<'_>>,
        continuation: Option<&<Self::Exchange as ActionExchange>::Continuation>,
        deadline: Deadline,
    ) -> CoreResult<Self::Exchange>;
}
/// Actual destination and original-instance permission. Mutable policy advances
/// revision without reuse, including changes made inside linked callbacks.
pub trait ActionAccess {
    fn revision(&self) -> u64;
    fn authorize(
        &self,
        consumer: LocalConsumerRef<'_>,
        operation: Operation,
        mapping: ActionMapping<'_>,
    ) -> CoreResult<()>;
}

/// Common application-result shape, independent of native artifact/storage.
/// This continuing facet admits primary application output only.
pub fn action_output_shape(output: InteractionOutputView<'_>) -> bool {
    output.status() == crate::InteractionStatus::Ok
        && output.metadata().response_selection() == crate::ResponseSelection::Primary
        && output.metadata().payload_role() == crate::ResponsePayloadRole::Application
        && output.metadata().action_invocation().is_none()
        && output.metadata().binding_response().is_none()
        && output.metadata().property_write_report().is_none()
}

/// A bounded Action turn polls the exchange or its retirement at most once.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionTurn {
    Idle,
    Advanced,
    Waiting,
    RetirementBlocked,
    Settled,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Poll,
    Retire,
    Observe,
    Idle,
}

pub struct ActionProgress {
    pub state: ConsumerActionState,
    pub first_cause: Option<CoreError>,
    pub cleanup_error: Option<CoreError>,
    pub validation_error: Option<CoreError>,
    pub accepted: bool,
    pub remote_cancel: ConsumerActionCancellation,
    operation: Operation,
    phase: Phase,
    observation: Option<ActionObservation>,
}
impl Default for ActionProgress {
    fn default() -> Self {
        Self {
            state: ConsumerActionState::Invoking,
            first_cause: None,
            cleanup_error: None,
            validation_error: None,
            accepted: false,
            remote_cancel: ConsumerActionCancellation::NotRequested,
            operation: Operation::InvokeAction,
            phase: Phase::Poll,
            observation: None,
        }
    }
}
impl ActionProgress {
    pub fn operation(&self) -> Operation {
        self.operation
    }
    pub fn can_control(&self) -> bool {
        self.state == ConsumerActionState::Pending && self.phase == Phase::Idle
    }
    pub fn start_control(&mut self, operation: Operation) {
        debug_assert!(self.can_control());
        self.operation = operation;
        self.phase = Phase::Poll;
        self.state = match operation {
            Operation::QueryAction => ConsumerActionState::Querying,
            Operation::CancelAction => {
                self.remote_cancel = ConsumerActionCancellation::Uncertain;
                ConsumerActionState::Cancelling
            }
            _ => unreachable!("continuing Action control"),
        };
    }
    pub fn close(&mut self, cause: CoreError) {
        self.first_cause.get_or_insert(cause);
        if self.phase == Phase::Poll {
            self.phase = Phase::Retire;
        }
        self.state = if self.phase == Phase::Idle && self.state.settlement_ready() {
            ConsumerActionState::Closed
        } else {
            ConsumerActionState::Closing
        };
    }
    fn settle(&mut self) {
        self.phase = Phase::Idle;
        self.observation = None;
        if self.first_cause.is_some() {
            self.state = ConsumerActionState::Closed;
        }
    }
}

/// Representation-specific edges. Each callback is bounded cooperative work.
/// Passive fences call no policy, codec or native I/O. Pending and error leave
/// native backing with its original owner until acknowledgement.
pub trait ActionDriver {
    fn progress(&mut self) -> &mut ActionProgress;
    /// Observe time, instance and cancellation without entering linked code.
    fn fence_local(&mut self);
    /// Also observe the current operation's original service dependencies.
    fn fence(&mut self);
    fn close(&mut self, cause: CoreError);
    fn invalid_reply(&self) -> CoreError;
    fn check_current(&mut self) -> CoreResult<()>;
    /// Retain the reply or native failure before returning. A later close can
    /// supply an original observation even when the normal poll failed.
    fn poll_exchange(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>>;
    /// On acknowledgement retain any late observation and release the exchange.
    fn poll_close(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>>;
    /// Move the observation into original continuation/result storage.
    fn retain_reply(&mut self) -> Option<ActionObservation>;
    fn validate_output(&mut self) -> CoreResult<()>;
    /// Release per-operation backing on a nonterminal reply, or the now
    /// unusable continuation on settlement. Retain original result owners.
    fn finish_operation(&mut self);
}

/// Recheck actual destination permission before application handoff. Closed
/// work remains available as raw evidence without calling access or a codec.
pub fn check_action_handoff(d: &mut impl ActionDriver) {
    d.fence();
    if d.progress().first_cause.is_none() {
        if let Err(error) = d.check_current() {
            d.close(error);
        }
        d.fence();
    }
}

/// One phase of the common continuing algorithm. The application schedules a
/// locally runnable turn, native readiness, or maintenance after failed close.
pub fn step_action(d: &mut impl ActionDriver, cx: &mut Context<'_>) -> ActionTurn {
    use ActionTurn::*;
    d.fence_local();
    let phase = d.progress().phase;
    match phase {
        Phase::Idle if d.progress().state.settlement_ready() => {
            check_action_handoff(d);
            Settled
        }
        Phase::Idle if d.progress().first_cause.is_none() => {
            check_action_handoff(d);
            if d.progress().first_cause.is_some() {
                finish(d)
            } else {
                Idle
            }
        }
        Phase::Idle => finish(d),
        Phase::Poll => {
            check_action_handoff(d);
            if d.progress().phase != Phase::Poll {
                return Advanced;
            }
            let polled = d.poll_exchange(cx);
            match polled {
                Poll::Ready(Ok(())) => d.progress().phase = Phase::Retire,
                Poll::Ready(Err(error)) => d.close(error),
                Poll::Pending => {}
            }
            d.fence();
            if d.progress().phase == Phase::Poll {
                Waiting
            } else {
                Advanced
            }
        }
        Phase::Retire => match d.poll_close(cx) {
            Poll::Pending => {
                d.fence_local();
                Waiting
            }
            Poll::Ready(Err(error)) => {
                d.progress().cleanup_error.get_or_insert(error);
                d.fence_local();
                RetirementBlocked
            }
            Poll::Ready(Ok(())) => {
                d.progress().phase = Phase::Observe;
                d.fence_local();
                if d.progress().first_cause.is_some() {
                    observe(d)
                } else {
                    Advanced
                }
            }
        },
        Phase::Observe => {
            d.fence();
            observe(d)
        }
    }
}

fn observe(d: &mut impl ActionDriver) -> ActionTurn {
    use ActionObservation::*;
    use ConsumerActionState as S;
    if d.progress().observation.is_none() {
        d.progress().observation = d.retain_reply();
    }
    let operation = d.progress().operation;
    let observation = d.progress().observation;
    match observation {
        Some(Complete) if operation != Operation::CancelAction => {
            if operation == Operation::InvokeAction {
                d.progress().accepted = true;
            }
            if d.progress().first_cause.is_none() {
                if let Err(error) = d.validate_output() {
                    d.progress().validation_error = Some(error.clone());
                    d.close(error);
                }
                check_action_handoff(d);
            }
            d.progress().state = S::Completed;
        }
        Some(Accepted) if operation == Operation::InvokeAction => {
            d.progress().accepted = true;
            d.progress().state = S::Pending;
        }
        Some(Pending) if operation == Operation::QueryAction => {
            d.progress().state = S::Pending;
        }
        Some(CancelRequested) if operation == Operation::CancelAction => {
            d.progress().remote_cancel = ConsumerActionCancellation::Requested;
            d.progress().state = S::Pending;
        }
        Some(AlreadyTerminal) if operation == Operation::CancelAction => {
            d.progress().remote_cancel = ConsumerActionCancellation::AlreadyTerminal;
            d.progress().state = S::Pending;
        }
        Some(Failed) => d.progress().state = S::Failed,
        _ if d.progress().first_cause.is_none() => d.close(d.invalid_reply()),
        _ => {}
    }
    d.fence();
    finish(d)
}

fn finish(d: &mut impl ActionDriver) -> ActionTurn {
    d.progress().settle();
    if d.progress().state == ConsumerActionState::Pending {
        d.progress().operation = Operation::InvokeAction;
    }
    d.finish_operation();
    d.fence();
    if d.progress().state == ConsumerActionState::Closing {
        d.progress().settle();
        d.finish_operation();
    }
    d.progress().settle();
    if d.progress().state.settlement_ready() {
        ActionTurn::Settled
    } else {
        ActionTurn::Advanced
    }
}
