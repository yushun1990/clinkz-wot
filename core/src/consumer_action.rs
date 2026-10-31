//! Continuing Consumer Actions. Native mappings and opaque continuation backing
//! belong to the selected Binding; neither is logical invocation authority.
use crate::{consumer::*, host_accounts::*, *};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::{
    any::Any,
    task::{Context, Poll},
};

/// Complete trusted bounds, including variable backing and local destruction.
/// One invocation has at most one native exchange at a time. All callbacks are
/// cooperative; abort (including destructors) is finite and nonblocking.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsumerActionBounds {
    pub exchange_bytes: u64,
    pub temporary_bytes: u64,
    pub continuation_bytes: u64,
    pub response_bytes: u64,
    pub poll_work: u64,
}

/// Distinct native observations. Acceptance is never a completed application
/// result. A cancel acknowledgement proves only a request, not remote rollback.
pub enum ConsumerActionReply {
    Accepted(Box<dyn Any>),
    Pending,
    Complete(InteractionOutput),
    Failed(CoreError),
    CancelRequested,
    AlreadyTerminal,
}

/// Owns a finite native exchange, not the continuing invocation. Ready/error
/// never disposes this owner. Close may return an original late observation;
/// pending/error retains every owner and requires further cleanup progress.
/// A poll error does not waive late response validation or retention: the
/// invocation keeps that error independently of an observation from close.
pub trait ConsumerActionExchange {
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<ConsumerActionReply>>;
    fn poll_close(&mut self, cx: &mut Context<'_>)
    -> Poll<CoreResult<Option<ConsumerActionReply>>>;
    fn abort_local(self: Box<Self>);
}

/// One immutable, actually compiled native operation. An external Binding can
/// supply this facet without a finite-call adapter or a protocol whitelist.
/// `start` refusal has no native effects; success captures all borrowed input.
/// Controls explicitly check compatibility with the original invoke mapping;
/// different registrations/Bindings are allowed when they implement it.
pub trait ConsumerActionMapping {
    fn identity(&self) -> BindingRegistrationIdentity;
    fn operation(&self) -> Operation;
    fn bounds(&self) -> ConsumerActionBounds;
    fn native_definition(&self) -> &dyn Any;
    fn supports_continuation(&self, invoke: &dyn ConsumerActionMapping) -> CoreResult<()>;
    /// Pure bounded actual destination calculation, before fresh policy checks.
    /// Start MUST use that same destination; redirects/proxies are unsupported
    /// unless the concrete mapping supplies equivalent policy coverage.
    fn destination(&self, continuation: Option<&dyn Any>) -> CoreResult<Box<str>>;
    fn start(
        &self,
        input: Option<&Payload>,
        continuation: Option<&dyn Any>,
    ) -> CoreResult<Box<dyn ConsumerActionExchange>>;
}

pub struct ConsumerActionOperation {
    pub mapping: Box<dyn ConsumerActionMapping>,
    /// Original selected Form target, including any native URI template. Only
    /// the Binding expands it, under the captured destination/security policy.
    pub endpoint: Box<str>,
    pub form: u32,
    pub services: ConsumerServices,
}
pub struct ConsumerActionDefinitionInput {
    pub target: AffordanceTarget,
    pub schemas: ConsumerSchemas,
    pub invoke: ConsumerActionOperation,
    pub query: Option<ConsumerActionOperation>,
    pub cancel: Option<ConsumerActionOperation>,
    pub bank: Arc<HostResourceAccounts>,
    // Last: actual native definitions and services die before original funding.
    pub funding: Vec<HostCapacityReservation>,
}
/// Source-free association; no compiler or source TD crosses into execution.
pub struct ConsumerActionDefinition(ConsumerActionDefinitionInput);
impl ConsumerActionDefinition {
    /// Source-free construction checks pairing, not TD support. Planning alone
    /// admits TD operation applicability and complete common/native semantics.
    pub fn try_new(
        input: ConsumerActionDefinitionInput,
    ) -> Result<Self, BindingInputRejection<ConsumerActionDefinitionInput>> {
        let checked = (|| {
            if !matches!(input.target, AffordanceTarget::Action(_))
                || input.target.name().is_none_or(|n| n.len() > 256)
            {
                return Err(invalid());
            }
            for (operation, selected) in [
                (Operation::InvokeAction, Some(&input.invoke)),
                (Operation::QueryAction, input.query.as_ref()),
                (Operation::CancelAction, input.cancel.as_ref()),
            ] {
                let Some(selected) = selected else { continue };
                let b = selected.mapping.bounds();
                if selected.mapping.operation() != operation
                    || b.poll_work == 0
                    || b.exchange_bytes == 0
                    || b.response_bytes == 0
                    || b.continuation_bytes == 0
                    || selected.endpoint.len() > 256
                    || !selected.services.data.available()
                    || selected.services.data.policy().validation_work == 0
                    || selected.services.access.work() == 0
                    || !input
                        .schemas
                        .input
                        .into_iter()
                        .chain(input.schemas.output)
                        .all(|s| selected.services.data.supports_schema(s))
                {
                    return Err(invalid());
                }
                if operation != Operation::InvokeAction {
                    selected
                        .mapping
                        .supports_continuation(input.invoke.mapping.as_ref())?;
                }
            }
            Ok(())
        })();
        match checked {
            Ok(()) => Ok(Self(input)),
            Err(e) => Err(BindingInputRejection::new(
                input,
                BindingOperationalError::new(e),
            )),
        }
    }
    pub fn input(&self) -> &ConsumerActionDefinitionInput {
        &self.0
    }
    pub fn into_input(self) -> ConsumerActionDefinitionInput {
        self.0
    }
}
fn invalid() -> CoreError {
    CoreError::Validation(ErrorContext::new(ErrorPhase::Validate, RetryClass::Never))
}
