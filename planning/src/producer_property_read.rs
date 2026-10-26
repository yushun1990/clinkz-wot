//! Complete-draft admission for the initial Host Producer Property Read scope.
//! This preparation result carries no publication or native installation power.
use clinkz_wot_core::{CoreError, CoreResult, ErrorContext, ErrorPhase, RetryClass};
use clinkz_wot_foundation::WorkBudget;
use clinkz_wot_td::{
    ValidatedProducerCursor, ValidatedProducerEvent as Event, ValidatedProducerScope as Scope,
    ValidatedProducerStep as Step, ValidatedSemanticError, ValidatedThing, ValidatedThingCause,
    data_type::Operation, semantic::ResolvedSemanticDocument,
};

/// Resolved facts lent only after complete supported shape and exact handler
/// coverage. Binding support still has to be established by native lowering.
/// This deliberately has no logical plan or required source TD id.
pub struct ResolvedProducerInput<'a> {
    document: &'a ResolvedSemanticDocument<'a>,
    target: &'a str,
    property: &'a str,
}
impl<'a> ResolvedProducerInput<'a> {
    pub fn document(&self) -> &'a ResolvedSemanticDocument<'a> {
        self.document
    }
    pub fn target(&self) -> &'a str {
        self.target
    }
    pub fn property(&self) -> &'a str {
        self.property
    }
    pub fn form_index(&self) -> u32 {
        0
    }
    pub fn operation(&self) -> Operation {
        Operation::ReadProperty
    }
}

/// Coverage is pending or complete. Complete means the entire advertised
/// shape has been checked; it does not certify protocol/security support.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProducerPropertyReadStep {
    Pending,
    Covered,
}

/// A private complete-coverage accumulator over TD's Producer continuation.
/// The handler argument is an exact capability key, never a handler object.
/// Initial single-route availability is confined to this preparation adapter.
pub struct ProducerPropertyReadAdmission<'td, 'handler> {
    td: ValidatedProducerCursor<'td>,
    handler: &'handler str,
    properties: usize,
    forms: usize,
    unsupported: bool,
    covered: bool,
    cause: Option<CoreError>,
}
impl<'td, 'handler> ProducerPropertyReadAdmission<'td, 'handler> {
    pub fn new(td: ValidatedThing<'td>, handler: &'handler str) -> Self {
        Self {
            td: td.into_producer(),
            handler,
            properties: 0,
            forms: 0,
            unsupported: false,
            covered: false,
            cause: None,
        }
    }
    pub fn step(
        &mut self,
        budget: &mut WorkBudget,
        cancel: bool,
    ) -> CoreResult<ProducerPropertyReadStep> {
        if let Some(cause) = &self.cause {
            return Err(cause.clone());
        }
        if cancel {
            let cause = CoreError::Cancelled(context());
            self.cause = Some(cause.clone());
            return Err(cause);
        }
        if self.covered {
            return Ok(ProducerPropertyReadStep::Covered);
        }
        let step = match self.td.step(budget, false) {
            Ok(step) => step,
            Err(cause) => {
                let cause = td_error(cause);
                self.cause = Some(cause.clone());
                return Err(cause);
            }
        };
        match step {
            Step::Pending => return Ok(ProducerPropertyReadStep::Pending),
            Step::Ready(Event::Scope { scope, forms }) => match scope {
                Scope::Thing => self.unsupported |= forms.is_some(),
                Scope::Property(name) => {
                    self.properties += 1;
                    self.unsupported |= forms != Some(1) || name != self.handler;
                }
                Scope::Action(_) | Scope::Event(_) => self.unsupported = true,
            },
            Step::Ready(Event::Form(form)) => {
                self.forms += 1;
                self.unsupported |= form.scope != Scope::Property(self.handler)
                    || form.index != 0
                    || form.effective_operations != [Operation::ReadProperty];
            }
            Step::Done => {
                if self.unsupported || self.properties != 1 || self.forms != 1 {
                    let cause = unsupported();
                    self.cause = Some(cause.clone());
                    return Err(cause);
                }
                self.covered = true;
                return Ok(ProducerPropertyReadStep::Covered);
            }
        }
        self.td.acknowledge();
        Ok(ProducerPropertyReadStep::Pending)
    }

    /// Pure native preparation after complete coverage. The scoped callback
    /// may return only source-free owned data. Arena Drop precedes release of
    /// its original account. None means insufficient step credit.
    /// No entry/registration/driver is issued by this semantic checkpoint.
    pub fn with_input<T: 'static>(
        &mut self,
        budget: &mut WorkBudget,
        lower: impl FnOnce(ResolvedProducerInput<'_>, &mut WorkBudget) -> CoreResult<Option<T>>,
    ) -> CoreResult<Option<T>> {
        if let Some(cause) = &self.cause {
            return Err(cause.clone());
        }
        if !self.covered {
            return Err(unsupported());
        }
        let result =
            self.td
                .with_property_semantics(self.handler, 0, budget, |document, target, budget| {
                    let property = document.property().name;
                    lower(
                        ResolvedProducerInput {
                            document,
                            target,
                            property,
                        },
                        budget,
                    )
                });
        let result = match result {
            Ok(Some(result)) => result,
            Ok(None) => Ok(None),
            Err(ValidatedSemanticError::Admission(cause)) => Err(td_error(cause)),
            Err(ValidatedSemanticError::Resolution(
                clinkz_wot_td::semantic::SemanticError::Allocation,
            )) => Err(CoreError::Backpressure(context())),
            Err(ValidatedSemanticError::Resolution(_)) => Err(unsupported()),
        };
        if let Err(cause) = &result {
            self.cause = Some(cause.clone());
        }
        result
    }
}
fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
}
fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(
        context().with_redacted_cause(620, "unsupported Producer draft coverage"),
    )
}
pub(crate) fn td_error(cause: ValidatedThingCause) -> CoreError {
    match cause {
        ValidatedThingCause::Cancelled { .. } => CoreError::Cancelled(context()),
        ValidatedThingCause::Limit(limit) => CoreError::LimitExceeded {
            context: context(),
            resource: limit.kind(),
            limit: limit.configured(),
            requested: None,
            observed: Some(limit.observed()),
        },
        ValidatedThingCause::Failed(failure)
            if failure.kind() == clinkz_wot_td::ValidatedThingFailureKind::AllocationFailed =>
        {
            CoreError::Backpressure(context())
        }
        _ => CoreError::Validation(context()),
    }
}
