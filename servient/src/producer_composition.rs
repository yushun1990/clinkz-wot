//! The qualified handoff to the shared logical Servient. Preparation produces
//! independent definitions; native setup is owned by each participating
//! Binding; this boundary commits one final TD and behavior owner only after
//! all participants are closed and the *whole* description is requalified.
#![allow(clippy::result_large_err)]
use clinkz_wot_core::{CoreError, CoreResult, ErrorContext, ErrorPhase, Operation, RetryClass};
use clinkz_wot_foundation::{ResourceLimits, WorkBudget};
use clinkz_wot_planning::producer_composition::{
    PreparedThing, ProducerCapability, check_final_thing,
};
use clinkz_wot_servient_finite::{
    ClosedProducerParticipation, ProducerPublication, RawProducerServient,
};
use clinkz_wot_td::ValidatedProducerScope;

/// Final qualification is fallible, publication is not. Retain the prepared
/// definitions separately from this logical publication and all physical
/// resources. No native poll, handler, allocation or fallible callback is made
/// between the last readiness check and the local commit.
pub fn publish_prepared_thing(
    owner: &mut RawProducerServient,
    publication: &mut ProducerPublication,
    prepared: &PreparedThing,
    providers: &[&dyn ProducerCapability],
    participants: &mut [&mut dyn ClosedProducerParticipation],
    limits: &ResourceLimits,
    budget: &mut WorkBudget,
) -> CoreResult<()> {
    let description = publication
        .staged_description()
        .ok_or_else(|| CoreError::Validation(context()))?;
    check_final_thing(
        description,
        publication.key(),
        limits,
        budget,
        providers,
        prepared,
        |scope, op| publication_supports(publication, scope, op),
    )?;
    if !publication.behaviors_described_by(|site, op| {
        prepared
            .operations()
            .iter()
            .any(|prepared| prepared.site.site() == site && prepared.operation == op)
    }) {
        return Err(CoreError::UnsupportedOperation(context()));
    }
    for operation in prepared.operations() {
        let owners = participants
            .iter()
            .filter(|p| {
                p.covers(
                    publication.key(),
                    operation.binding,
                    operation.site.site(),
                    operation.form,
                    operation.operation,
                    operation.definition_any(),
                )
            })
            .count();
        if owners != 1 {
            return Err(CoreError::UnsupportedOperation(context()));
        }
    }
    owner.publish_composed(publication, participants)
}

/// Check actual staged behavior, not merely a nominal operation family. A
/// Thing-level collection Form needs its own registered collection behavior;
/// it is not lowered into individual Property reads.
pub fn publication_supports(
    publication: &ProducerPublication,
    scope: ValidatedProducerScope<'_>,
    op: Operation,
) -> bool {
    match scope {
        ValidatedProducerScope::Thing => false, // no collection behavior registered yet
        ValidatedProducerScope::Property(n) => publication.supports_property(n, op),
        ValidatedProducerScope::Action(n) => publication.supports_action(n, op),
        ValidatedProducerScope::Event(_) => false, // no notification behavior registered yet
    }
}
fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
}
