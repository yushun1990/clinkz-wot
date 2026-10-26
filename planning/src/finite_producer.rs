//! Complete-description preparation for the first joined finite Producer.
//! This is a single Property Read subset, not a general operation compiler.
// CoreError is intentionally the exact inline, allocation-free error carrier.
#![allow(clippy::result_large_err)]
use crate::{ProducerPropertyReadAdmission, ProducerPropertyReadStep, ResolvedProducerInput};
use clinkz_wot_core::{finite::FiniteIngressBinding, producer::ProducerInstanceKey, *};
use clinkz_wot_foundation::{AdmissionLedger, ResourceKind, ResourceLimits, WorkBudget, WorkClass};
use clinkz_wot_td::{
    ValidatedThingAdmissionConfig, ValidatedThingCursor, ValidatedThingProgress, thing::Thing,
};

/// Host decoding envelope. Parsing is paid before entry and has serde's finite
/// recursion limit. This is not a transitive allocator/SDK heap certificate.
pub const PRODUCER_DESCRIPTION_BYTES: usize = 16_384;

/// One actual startup-registered preparation provider. Runtime-only Bindings
/// implement only Core's finite facet. No executor whitelist or legacy
/// ProducerExecutor/installation ABI participates in this path.
pub trait FiniteProducerCompiler {
    type Binding: FiniteIngressBinding<Configuration: 'static> + 'static;

    /// Pure complete mapping construction, including actual codec/schema and
    /// security support. Return an independently provisioned configuration
    /// snapshot for passive runtime checks; neither value contains authority.
    #[allow(clippy::type_complexity)]
    fn prepare(
        &self,
        input: ResolvedProducerInput<'_>,
        instance: ProducerInstanceKey,
        budget: &mut WorkBudget,
    ) -> CoreResult<
        Option<(
            Self::Binding,
            <Self::Binding as FiniteIngressBinding>::Configuration,
        )>,
    >;

    /// Recheck the *same* actual provider and final resolved meaning after
    /// closed native setup. Check native endpoints/collisions/readiness as well
    /// as every immutable dependency. This cannot acquire more native effects.
    fn check_final(
        &self,
        input: ResolvedProducerInput<'_>,
        instance: ProducerInstanceKey,
        binding: &Self::Binding,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<()>>;

    /// Additional Engine-retained indirect backing in the compiler and the
    /// independently provisioned configuration snapshot. Native driver/backing
    /// stays under the Binding's declared limits and finite disposal policy.
    fn retained_bytes(&self) -> usize;
}

/// Reinterpret the entire exact retained description on each qualification.
/// No selected-leaf shortcut, cached Covered flag or source callback survives.
/// The callback is reached only after Basic and complete operation coverage.
/// Work exhaustion returns Backpressure; callers retain inputs and may retry.
pub fn qualify_property_read<T: 'static>(
    description: &str,
    property: &str,
    instance: ProducerInstanceKey,
    limits: &ResourceLimits,
    budget: &mut WorkBudget,
    lower: impl FnOnce(ResolvedProducerInput<'_>, &mut WorkBudget) -> CoreResult<Option<T>>,
) -> CoreResult<T> {
    if description.len() > PRODUCER_DESCRIPTION_BYTES || property.len() > 256 {
        return Err(CoreError::UnsupportedOperation(context()));
    }
    budget
        .consume(WorkClass::CodecInputBytes, description.len() as u64)
        .map_err(|_| busy())?;
    let td: Thing =
        serde_json::from_str(description).map_err(|_| CoreError::Validation(context()))?;
    let config = ValidatedThingAdmissionConfig::try_from_limits(limits)
        .map_err(|_| CoreError::Validation(context()))?;
    let temporary = limits
        .get(ResourceKind::AdmissionTemporaryBytesPerOperationMax)
        .unwrap_or(0);
    let mut cursor = ValidatedThingCursor::from_thing(
        &td,
        &config,
        AdmissionLedger::new(
            instance.slot().slot(),
            instance.slot().generation(),
            0,
            temporary,
            0,
            0,
            0,
            0,
        ),
    );
    let validated = loop {
        budget
            .consume(WorkClass::PlanningItems, 1)
            .map_err(|_| busy())?;
        let before = WorkClass::ALL.map(|class| budget.remaining(class));
        cursor = match cursor.step(budget, false) {
            ValidatedThingProgress::Complete(value) => break value,
            ValidatedThingProgress::Failed(cause) => {
                return Err(crate::producer_property_read::td_error(cause));
            }
            ValidatedThingProgress::Pending(next) => next,
        };
        // The finite caller budget also bounds retries of an unpaid TD step.
        if before == WorkClass::ALL.map(|class| budget.remaining(class)) {
            return Err(busy());
        }
    };
    let mut admission = ProducerPropertyReadAdmission::new(validated, property);
    loop {
        budget
            .consume(WorkClass::PlanningItems, 1)
            .map_err(|_| busy())?;
        let before = WorkClass::ALL.map(|class| budget.remaining(class));
        if admission.step(budget, false)? == ProducerPropertyReadStep::Covered {
            break;
        }
        if before == WorkClass::ALL.map(|class| budget.remaining(class)) {
            return Err(busy());
        }
    }
    admission.with_input(budget, lower)?.ok_or_else(busy)
}

fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
}
fn busy() -> CoreError {
    CoreError::Backpressure(context())
}
