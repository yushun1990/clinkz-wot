//! Shared finite and continuing execution, independent of storage, reference counting, threads
//! and native driving. Owners retain actual work/results and supply bounded
//! callbacks; this kernel grants no TD support or logical authority by itself.
pub mod action;
pub mod producer;
pub mod read;
pub mod responses;

use crate::{
    AffordanceSite, BindingGeneration, BindingId, CoreError, CoreResult, ErrorContext, ErrorPhase,
    InteractionOutputView, Operation, PayloadView, PlanId, ResponsePayloadRole, RetryClass,
    data_schema::{PreparedSchema, ValueSchemas},
    security::SecurityRequirement,
};
use core::task::Context;
use core::task::Poll;

/// Borrowed immutable single-value meaning, including the input/output roles of
/// a continuing Action. Invoke and Query share schemas, not a native exchange. The loan
/// may come from a prepared Host owner, a caller value or static storage. It
/// retains no authority, endpoint, service, native program or resource account.
#[derive(Clone, Copy)]
pub struct ValueExecution<'a> {
    site: AffordanceSite<'a>,
    operation: Operation,
    schemas: &'a ValueSchemas,
    security: &'a SecurityRequirement,
    responses: Option<&'a responses::FiniteResponses>,
    uri_variables: Option<&'a crate::uri_variables::PreparedUriVariables>,
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
            uri_variables: None,
        })
    }
    pub fn with_responses(mut self, responses: Option<&'a responses::FiniteResponses>) -> Self {
        self.responses = responses;
        self
    }
    pub fn responses(self) -> Option<&'a responses::FiniteResponses> {
        self.responses
    }
    pub fn with_uri_variables(
        mut self,
        variables: &'a crate::uri_variables::PreparedUriVariables,
    ) -> Self {
        self.uri_variables = Some(variables);
        self
    }
    pub fn uri_variables(self) -> Option<&'a crate::uri_variables::PreparedUriVariables> {
        self.uri_variables
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
    /// Check the original dependencies and live permission for an effect or handoff.
    fn check_current(&mut self) -> CoreResult<()>;
    /// One bounded poll on the original exchange or its local cancellation.
    /// The native owner retains errors and late evidence through physical release.
    fn poll_native(
        &mut self,
        cx: &mut Context<'_>,
        cancel: Option<(&CoreError, bool)>,
    ) -> NativeProgress;
}

/// Common native and authority transitions. Result backing stays with the
/// storage owner, which validates the exact value at application handoff.
#[derive(Default)]
pub struct FiniteCallState {
    first_cause: Option<CoreError>,
    cancel_started: bool,
    settled: bool,
}
impl FiniteCallState {
    pub fn first_cause(&self) -> Option<&CoreError> {
        self.first_cause.as_ref()
    }
    /// The original native exchange acknowledged a terminal disposition.
    pub fn is_terminal(&self) -> bool {
        self.settled
    }
    pub fn fence(&mut self, driver: &mut impl FiniteCallDriver) {
        if let Some(error) = driver.fence() {
            self.first_cause.get_or_insert(error);
        }
    }
    /// An observed refusal survives later recovery, including while unread.
    pub fn check_current(&mut self, driver: &mut impl FiniteCallDriver) {
        self.fence(driver);
        if self.first_cause.is_none() {
            if let Err(error) = driver.check_current() {
                driver.record(error);
            }
            self.fence(driver);
        }
    }
    /// Manually driven. Retry on native readiness or an independent maintenance
    /// turn (including cancellation, a cleanup failure or a deadline).
    pub fn step(&mut self, driver: &mut impl FiniteCallDriver, cx: &mut Context<'_>) -> Poll<()> {
        self.check_current(driver);
        if !self.settled {
            let cancel = self
                .first_cause
                .as_ref()
                .map(|cause| (cause, self.cancel_started));
            match driver.poll_native(cx, cancel) {
                NativeProgress::Pending => {}
                NativeProgress::Cancelling => self.cancel_started = true,
                NativeProgress::Settled => self.settled = true,
            }
            self.fence(driver);
        }
        if self.settled {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

fn invalid() -> CoreError {
    CoreError::Validation(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
}

/// Actual bounded media/schema service. Constraint meaning is Core's shared
/// PreparedSchema; decoding stays with the actual codec. Support is complete,
/// not merely outer type recognition. No source TD or callback is retained.
/// Callback scratch is bounded caller/service storage, not an Engine allocation;
/// validation retains no payload and acquires no unreserved Engine backing.
pub trait ValueValidator {
    fn content_type(&self) -> &str;
    fn supports(&self, schema: &PreparedSchema) -> bool;
    fn body_bytes(&self) -> usize;
    fn available(&self) -> bool {
        true
    }
    /// One bounded validation. Use bounded stack or workspace already owned
    /// by the service. Admitted result delivery cannot depend on ordinary
    /// scratch acquisition.
    fn validate(&self, schema: &PreparedSchema, payload: PayloadView<'_>) -> CoreResult<()>;
}

/// Conservative remote-operation certainty, independent of local cancellation
/// and validated result delivery. Any native progress may cross the effect edge;
/// failure or cancellation afterwards cannot authorize implicit replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsumerEffect {
    NotAttempted,
    /// Remote effects may include a partially applied collection. No replay.
    Unknown,
    /// A Property or collection write's native completion was validated. This
    /// reports completion under the selected mapping, not atomicity, rollback
    /// or independent confirmation of physical state.
    /// Local cancellation/policy failure can still refuse application handoff.
    Completed,
    /// A validated member report is retained; the whole group is incomplete.
    /// Consult the original native output for exact member/atomicity evidence.
    Reported,
}
