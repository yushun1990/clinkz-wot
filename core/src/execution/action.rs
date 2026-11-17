//! Continuing Consumer Action transitions. This state owns no native exchange,
//! continuation, result, authority, account, registry or wake mechanism.
use super::ValueExecution;
use super::read::LocalConsumerRef;
use crate::{
    BindingGeneration, BindingId, CoreError, CoreResult, Deadline, InteractionOutputView,
    Operation, PayloadView,
};
use clinkz_wot_foundation::{WorkBudget, WorkClass as W};
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
pub trait ActionOutput {
    fn view(&self) -> InteractionOutputView<'_>;
}
pub trait ActionInput {
    fn view(&self) -> PayloadView<'_>;
    /// Indirect original input backing beyond this owner's inline layout.
    fn backing_bytes(&self) -> u64;
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
    fn configuration(&self) -> &Self::Configuration;
    fn mapping<'a>(
        &'a self,
        program: &'a Self::Program,
        operation: Operation,
    ) -> Option<ActionMapping<'a>>;
    /// Pure actual destination, checked before policy; start must use this same
    /// destination. Redirects need equivalent explicit policy coverage.
    fn destination<'a>(
        &'a self,
        program: &'a Self::Program,
        operation: Operation,
        continuation: Option<&'a <Self::Exchange as ActionExchange>::Continuation>,
    ) -> CoreResult<ActionMapping<'a>>;
    fn supports(&self, program: &Self::Program, meaning: ValueExecution<'_>) -> bool;
    /// Complete poll + close work per step, including pure destination work.
    fn poll_work(&self, operation: Operation) -> u64;
    /// Complete indirect Engine work, input/result, token and retirement backing
    /// beyond inline owner layout. Private native resources have their own owner.
    fn backing_bytes(&self) -> u64;
    fn start(
        &self,
        program: &Self::Program,
        operation: Operation,
        input: Option<PayloadView<'_>>,
        continuation: Option<&<Self::Exchange as ActionExchange>::Continuation>,
        deadline: Deadline,
    ) -> CoreResult<Self::Exchange>;
}
/// Actual destination and original-instance permission. Mutable policy advances
/// revision without reuse, including changes made inside linked callbacks.
pub trait ActionAccess {
    fn revision(&self) -> u64;
    fn work(&self) -> u64;
    fn authorize(
        &self,
        consumer: LocalConsumerRef<'_>,
        operation: Operation,
        mapping: ActionMapping<'_>,
    ) -> CoreResult<()>;
}

/// Common application-result shape, independent of native artifact/storage.
pub fn action_output_shape(output: InteractionOutputView<'_>) -> bool {
    output.status() == crate::InteractionStatus::Ok
        && output.metadata().payload_role() == crate::ResponsePayloadRole::Application
        && output.metadata().action_invocation().is_none()
        && output.metadata().binding_response().is_none()
        && output.metadata().property_write_report().is_none()
}

pub struct ActionProgress {
    pub state: ConsumerActionState,
    pub retiring: bool,
    pub first_cause: Option<CoreError>,
    pub cleanup_error: Option<CoreError>,
    pub validation_error: Option<CoreError>,
    pub accepted: bool,
    pub remote_cancel: ConsumerActionCancellation,
}
impl Default for ActionProgress {
    fn default() -> Self {
        Self {
            state: ConsumerActionState::Invoking,
            retiring: false,
            first_cause: None,
            cleanup_error: None,
            validation_error: None,
            accepted: false,
            remote_cancel: ConsumerActionCancellation::NotRequested,
        }
    }
}
impl ActionProgress {
    pub fn close(&mut self, cause: CoreError, has_exchange: bool) {
        self.first_cause.get_or_insert(cause);
        self.retiring = has_exchange;
        self.state = if has_exchange {
            ConsumerActionState::Closing
        } else {
            ConsumerActionState::Closed
        };
    }
}

/// Original complete callback allowances; no declaration is refreshed at use.
pub struct ActionWork {
    pub access: u64,
    pub native: u64,
    pub validation: u64,
}
impl ActionWork {
    pub fn prepay(&self, budget: &mut WorkBudget) -> bool {
        self.access.checked_mul(2).is_some_and(|access| {
            budget.consume(W::ProviderProbes, access).is_ok()
                && budget.consume(W::BindingPolls, self.native).is_ok()
                && budget.consume(W::CleanupItems, 1).is_ok()
                && budget.consume(W::CodecInputBytes, self.validation).is_ok()
        })
    }
}

/// Representation-specific edges. Callbacks are bounded cooperative work.
/// The driver retains original owners on Pending, error, late reply and unwind.
/// Inspection/fencing is passive. Poll/close share the prepaid native allowance;
/// its declaration must cover both edges and finite local disposal.
pub trait ActionDriver {
    fn progress(&mut self) -> &mut ActionProgress;
    fn operation(&self) -> Operation;
    fn has_exchange(&self) -> bool;
    fn work(&self) -> ActionWork;
    /// Check time, original authority and live dependencies, preserving first cause.
    fn fence(&mut self);
    fn close(&mut self, cause: CoreError);
    fn invalid_reply(&self) -> CoreError;
    fn check_current(&mut self) -> CoreResult<()>;
    fn refresh_access(&mut self, budget: &mut WorkBudget) -> CoreResult<()>;
    /// Retain the reply before returning Ready(Ok). Retain native poll errors
    /// separately before returning Err; a later close may still produce output.
    fn poll_exchange(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>>;
    /// Only acknowledged local close removes the exchange. Retain an original
    /// late observation if none was already returned. Errors retain all owners.
    fn poll_close(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>>;
    /// Move the stored observation into original continuation/result storage
    /// BEFORE returning its kind, in particular before any validator callback.
    fn retain_reply(&mut self) -> Option<ActionObservation>;
    fn validate_output(&mut self) -> CoreResult<()>;
    /// A retired nonterminal control no longer owns live dependencies.
    fn reset_operation(&mut self);
}

/// One common continuing algorithm for independently owned representations.
/// Query/Cancel initiation and successful result handoff remain explicit edges.
pub fn step_action(d: &mut impl ActionDriver, cx: &mut Context<'_>, budget: &mut WorkBudget) {
    d.fence();
    if d.progress().state.settlement_ready() {
        let _ = d.refresh_access(budget);
        d.fence();
        return;
    }
    if !d.work().prepay(budget) {
        return;
    }
    if d.progress().first_cause.is_none()
        && let Err(e) = d.check_current()
    {
        d.close(e);
    }
    d.fence();
    if d.has_exchange() && !d.progress().retiring {
        match d.poll_exchange(cx) {
            Poll::Pending => {
                d.fence();
                if d.progress().first_cause.is_none() {
                    return;
                }
            }
            Poll::Ready(Ok(())) => d.progress().retiring = true,
            Poll::Ready(Err(e)) => d.close(e),
        }
    }
    d.fence();
    if d.has_exchange() {
        match d.poll_close(cx) {
            Poll::Pending => {
                d.fence();
                return;
            }
            Poll::Ready(Err(e)) => {
                d.progress().cleanup_error.get_or_insert(e);
                d.fence();
                return;
            }
            Poll::Ready(Ok(())) => {}
        }
    }
    d.fence();
    if let Some(reply) = d.retain_reply() {
        let operation = d.operation();
        match reply {
            ActionObservation::Complete if operation != Operation::CancelAction => {
                if let Err(e) = d.validate_output() {
                    d.progress().validation_error = Some(e.clone());
                    d.close(e);
                }
                if operation == Operation::InvokeAction {
                    d.progress().accepted = true;
                }
                d.progress().state = ConsumerActionState::Completed;
            }
            ActionObservation::Accepted if operation == Operation::InvokeAction => {
                d.progress().accepted = true;
                d.progress().state = ConsumerActionState::Pending;
            }
            ActionObservation::Pending if operation == Operation::QueryAction => {
                d.progress().state = ConsumerActionState::Pending;
            }
            ActionObservation::CancelRequested if operation == Operation::CancelAction => {
                d.progress().remote_cancel = ConsumerActionCancellation::Requested;
                d.progress().state = ConsumerActionState::Pending;
            }
            ActionObservation::AlreadyTerminal if operation == Operation::CancelAction => {
                d.progress().remote_cancel = ConsumerActionCancellation::AlreadyTerminal;
                d.progress().state = ConsumerActionState::Pending;
            }
            ActionObservation::Failed => d.progress().state = ConsumerActionState::Failed,
            _ => d.close(d.invalid_reply()),
        }
        d.fence();
        if d.progress().first_cause.is_none()
            && let Err(e) = d.check_current()
        {
            d.close(e);
        }
    }
    d.fence();
    if d.progress().first_cause.is_some() {
        d.progress().state = ConsumerActionState::Closed;
    } else if d.progress().state == ConsumerActionState::Pending {
        d.reset_operation();
    }
    d.progress().retiring = false;
}
