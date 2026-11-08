//! Shared finite execution, independent of storage, reference counting, threads
//! and native driving. Owners retain actual work/results and supply bounded
//! callbacks; this kernel grants no TD support or logical authority by itself.
pub mod read;
pub mod responses;

use crate::{
    AffordanceSite, BindingGeneration, BindingId, CoreError, CoreResult, ErrorContext, ErrorPhase,
    InteractionOutputView, Operation, PlanId, ResponsePayloadRole, RetryClass,
    data_schema::ValueSchemas, security::SecurityRequirement,
};
use clinkz_wot_foundation::{WorkBudget, WorkClass as W};
use core::task::Context;
use core::task::Poll;

/// Borrowed immutable meaning for a finite single-value interaction. The loan
/// may come from a prepared Host owner, a caller value or static storage. It
/// retains no authority, endpoint, service, native program or resource account.
#[derive(Clone, Copy)]
pub struct ValueExecution<'a> {
    site: AffordanceSite<'a>,
    operation: Operation,
    schemas: &'a ValueSchemas,
    security: &'a SecurityRequirement,
    responses: Option<&'a responses::FiniteResponses>,
}
impl<'a> ValueExecution<'a> {
    pub fn try_new(
        site: AffordanceSite<'a>,
        operation: Operation,
        schemas: &'a ValueSchemas,
        security: &'a SecurityRequirement,
    ) -> CoreResult<Self> {
        let valid = match (site, operation) {
            (AffordanceSite::Property(_), Operation::ReadProperty) => {
                schemas.input.is_none() && schemas.output.is_some()
            }
            (AffordanceSite::Property(_), Operation::WriteProperty) => {
                schemas.input.is_some() && schemas.output.is_none()
            }
            (AffordanceSite::Action(_), Operation::InvokeAction) => true,
            _ => false,
        };
        if !valid {
            return Err(invalid());
        }
        Ok(Self {
            site,
            operation,
            schemas,
            security,
            responses: None,
        })
    }
    pub fn with_responses(mut self, responses: Option<&'a responses::FiniteResponses>) -> Self {
        self.responses = responses;
        self
    }
    pub fn responses(self) -> Option<&'a responses::FiniteResponses> {
        self.responses
    }
    pub fn site(self) -> AffordanceSite<'a> {
        self.site
    }
    pub fn operation(self) -> Operation {
        self.operation
    }
    pub fn schemas(self) -> &'a ValueSchemas {
        self.schemas
    }
    pub fn security(self) -> &'a SecurityRequirement {
        self.security
    }
}

/// Shape/identity seal shared by all finite Consumer output representations.
/// It deliberately does not assert media or DataSchema conformance. Expected
/// identities must be captured from the selected request by its actual owner.
pub fn validate_finite_output(
    binding: BindingId,
    generation: BindingGeneration,
    plan: PlanId,
    operation: Operation,
    output: InteractionOutputView<'_>,
) -> CoreResult<()> {
    validate_finite_response(
        binding,
        generation,
        plan,
        operation,
        responses::FiniteResponseShape::primary(operation),
        output,
    )
}

/// Validate only against the original owner's admitted response branches.
pub fn validate_finite_response(
    binding: BindingId,
    generation: BindingGeneration,
    plan: PlanId,
    operation: Operation,
    responses: responses::FiniteResponseShape,
    output: InteractionOutputView<'_>,
) -> CoreResult<()> {
    let metadata = output.metadata();
    let valid_response = metadata.binding_response().is_some_and(|response| {
        response.binding_id() == binding
            && response.binding_generation() == generation
            && response.plan_id() == plan
    });
    if valid_response
        && matches!(
            operation,
            Operation::ReadProperty
                | Operation::WriteProperty
                | Operation::InvokeAction
                | Operation::QueryAllActions
                | Operation::ReadAllProperties
                | Operation::ReadMultipleProperties
                | Operation::WriteAllProperties
                | Operation::WriteMultipleProperties
        )
        && responses
            .validate(metadata.response_selection(), output)
            .is_ok()
        && metadata.payload_role() == ResponsePayloadRole::Application
        && metadata.action_invocation().is_none()
        && (metadata.property_write_report().is_none()
            || matches!(
                operation,
                Operation::WriteAllProperties | Operation::WriteMultipleProperties
            ))
    {
        Ok(())
    } else {
        Err(CoreError::Validation(
            ErrorContext::new(ErrorPhase::Validate, RetryClass::Never)
                .with_operation(operation)
                .with_plan(plan)
                .with_binding(binding, generation),
        ))
    }
}

/// Native ownership progress, not application acceptance or effect certainty.
pub enum NativeProgress {
    Pending,
    /// Cancellation was accepted; subsequent progress polls that same cleanup.
    Cancelling,
    /// The driver retained a terminal native result/cancellation disposition.
    Settled,
}

/// Representation-specific edges of one finite call. Every callback is bounded
/// cooperative work. The driver retains original native/output owners, funding,
/// mapping and authority through settlement and finite local disposal. It must
/// never report Settled for an unacknowledged cleanup transfer.
pub trait FiniteCallDriver {
    /// Passive observations of cancellation, deadline, currentness and the first
    /// recorded cause. No linked callback, allocation or work credit required.
    fn fence(&mut self) -> Option<CoreError>;
    /// Preserve first-cause order, including causes recorded during callbacks.
    fn record(&mut self, error: CoreError);
    /// Captured complete permission/dependency-check allowance.
    fn access_work(&self) -> u64;
    /// Fresh original-instance permission, with dependency/currentness checks
    /// before and after the linked callback. The kernel prepays access_work.
    fn check_current(&mut self) -> CoreResult<()>;
    /// Exactly one native progress edge. Only isolated BindingPolls credit is
    /// provided. Some(cause, started) selects cancellation of original work.
    /// Errors/late outputs must remain owned; cleanup errors do not replace cause.
    fn poll_native(
        &mut self,
        cx: &mut Context<'_>,
        cancel: Option<(&CoreError, bool)>,
        budget: &mut WorkBudget,
    ) -> NativeProgress;
    fn has_success(&self) -> bool;
    /// Validate complete shape/backing and captured media/schema meaning. Short
    /// credit retains the output. Refusal retains the original rejected output.
    /// The driver prepays its captured validation allowance before callbacks.
    fn validate_output(&mut self, budget: &mut WorkBudget) -> Poll<CoreResult<()>>;
}

/// One common transition kernel for Host and caller-owned finite calls. This
/// owns semantic progress only; it imposes no owner/layout/dispatch strategy.
#[derive(Default)]
pub struct FiniteCallState {
    first_cause: Option<CoreError>,
    cancel_started: bool,
    native_settled: bool,
    terminal: bool,
    output_validated: bool,
}
impl FiniteCallState {
    pub fn first_cause(&self) -> Option<&CoreError> {
        self.first_cause.as_ref()
    }
    pub fn is_terminal(&self) -> bool {
        self.terminal
    }
    fn fence(&mut self, driver: &mut impl FiniteCallDriver) {
        if let Some(error) = driver.fence() {
            self.first_cause.get_or_insert(error);
        }
    }
    /// Ready requires native settlement plus fresh paid policy observations.
    /// Cancellation, revocation or validation failure cannot erase late output.
    /// No zero-credit native/codec/permission callback is entered.
    pub fn step(
        &mut self,
        driver: &mut impl FiniteCallDriver,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<()> {
        self.fence(driver);
        if budget
            .consume(W::ProviderProbes, driver.access_work())
            .is_err()
        {
            return Poll::Pending;
        }
        if let Err(error) = driver.check_current() {
            driver.record(error);
        }
        self.fence(driver);
        if self.terminal || self.output_validated {
            self.terminal = true;
            return Poll::Ready(());
        }
        if !self.native_settled {
            if budget.consume(W::HandlerSteps, 1).is_err() {
                return Poll::Pending;
            }
            if self.first_cause.is_some() && budget.consume(W::CleanupItems, 1).is_err() {
                return Poll::Pending;
            }
            if budget.consume(W::BindingPolls, 1).is_err() {
                return Poll::Pending;
            }
            let mut native = WorkBudget::new().with_remaining(W::BindingPolls, 1);
            let cancel = self
                .first_cause
                .as_ref()
                .map(|cause| (cause, self.cancel_started));
            match driver.poll_native(cx, cancel, &mut native) {
                NativeProgress::Pending => {}
                NativeProgress::Cancelling => self.cancel_started = true,
                NativeProgress::Settled => self.native_settled = true,
            }
            budget.set_remaining(
                W::BindingPolls,
                budget.remaining(W::BindingPolls) + native.remaining(W::BindingPolls),
            );
        }
        self.fence(driver);
        if !self.native_settled {
            return Poll::Pending;
        }
        if driver.has_success() {
            match driver.validate_output(budget) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Ok(())) => self.output_validated = true,
                Poll::Ready(Err(error)) => driver.record(error),
            }
        }
        self.fence(driver);
        // Native and codec callbacks may have changed live policy. Short credit
        // retains validation and the whole output for a later fresh handoff.
        if self.first_cause.is_none() && driver.has_success() {
            if budget
                .consume(W::ProviderProbes, driver.access_work())
                .is_err()
            {
                return Poll::Pending;
            }
            if let Err(error) = driver.check_current() {
                driver.record(error);
            }
        }
        self.fence(driver);
        self.terminal = true;
        Poll::Ready(())
    }
}

fn invalid() -> CoreError {
    CoreError::Validation(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
}
