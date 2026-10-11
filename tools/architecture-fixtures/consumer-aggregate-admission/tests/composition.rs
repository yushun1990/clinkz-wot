use clinkz_wot_core::*;
use clinkz_wot_foundation::{
    AdmissionLedger, GatewayDefaultV1, Generation, ResourceAccount, ResourceKind, SlotIndex,
    StaticResourceProfile, WorkBudget, WorkClass as W,
};
use clinkz_wot_td::{
    ValidatedPropertyReadCursor, ValidatedPropertyReadEvent as Event,
    ValidatedPropertyReadStep as Step, ValidatedThing, ValidatedThingAdmissionConfig,
    ValidatedThingCursor, ValidatedThingProgress, thing::Thing,
};
use consumer_aggregate_admission_probe::{
    InputOwner, Outcome, Registration, Slot, compiler_envelope,
};
use consumer_compiler_support_probe::production_static_registration;
use std::alloc::Layout;

fn budget(n: u64) -> WorkBudget {
    W::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, class| b.with_remaining(class, n))
}
fn source() -> Thing {
    serde_json::from_str(r#"{
      "@context":"https://www.w3.org/2022/wot/td/v1.1",
      "title":"closed support join", "id":"urn:joined", "base":"https://peer.test/a/",
      "security":["none"], "securityDefinitions":{"none":{"scheme":"nosec"}},
      "properties":{
        "a-empty":{"forms":[]},
        "b-read":{"forms":[{"href":"ignored","op":[]},{"href":"../first"}]},
        "c-empty":{"forms":[{"href":"ignored","op":[]}]},
        "z-read":{"forms":[{"href":"ignored","op":["writeproperty"]},{"href":"last","scopes":["二", ""]}]}
      }
    }"#).unwrap()
}
fn validate(source: &Thing) -> ValidatedThing<'_> {
    let config = ValidatedThingAdmissionConfig::try_from_limits(GatewayDefaultV1::LIMITS).unwrap();
    let ledger = AdmissionLedger::new(
        SlotIndex::new(4),
        Generation::INITIAL,
        0,
        8 << 20,
        0,
        0,
        0,
        0,
    );
    let mut cursor = ValidatedThingCursor::from_thing(source, &config, ledger);
    for _ in 0..100_000 {
        match cursor.step(&mut budget(4096), false) {
            ValidatedThingProgress::Pending(next) => cursor = next,
            ValidatedThingProgress::Complete(proof) => return proof,
            ValidatedThingProgress::Failed(cause) => panic!("{cause:?}"),
        }
    }
    panic!("validation stalled")
}
fn identity(compiler: &ResolvedTargetCompiler) -> BindingRegistrationIdentity {
    BindingRegistrationIdentity::new(
        BindingId::new(91),
        BindingGeneration::INITIAL.checked_next().unwrap(),
        compiler.configuration(),
        compiler.compatibility(),
        7,
    )
}
fn candidate(id: BindingRegistrationIdentity) -> BindingCandidate {
    BindingCandidate::new(
        id.binding_id(),
        id.binding_generation(),
        id.configuration(),
        id.artifact_compatibility(),
        0,
        0,
    )
}

// Deliberately fixed at two readable coordinates. TD owns all semantic work;
// this positive witness makes ordinary owned copies, not an aggregate allocator.
fn preflight(td: &mut ValidatedPropertyReadCursor<'_>) {
    let mut rows = Vec::new();
    let mut coordinates = 0;
    for _ in 0..100_000 {
        match td.step(&mut budget(4096), false).unwrap() {
            Step::Pending => continue,
            Step::Done => break,
            Step::Ready(Event::Property { ordinal, name }) => rows.push((ordinal, name.to_owned())),
            Step::Ready(Event::Form(form)) => {
                assert_eq!(
                    (form.security_count(), form.security_scheme()),
                    (1, Some("nosec"))
                );
                coordinates += 1;
            }
        }
        td.acknowledge();
    }
    assert_eq!(
        rows,
        [
            (0, "a-empty".into()),
            (1, "b-read".into()),
            (2, "c-empty".into()),
            (3, "z-read".into())
        ]
    );
    assert_eq!(coordinates, 2);
    // rewind consumes the original production continuation and retains its
    // remaining TD lifetime. No second proof/validation pass is constructed.
    // The second pass below makes that consumed move explicit.
}
fn materialize(td: &mut ValidatedPropertyReadCursor<'_>) -> Vec<LogicalInteractionPlan> {
    let id = td.id().unwrap().to_owned();
    let mut plans = Vec::new();
    for _ in 0..100_000 {
        match td.step(&mut budget(4096), false).unwrap() {
            Step::Pending => continue,
            Step::Done => return plans,
            Step::Ready(Event::Property { .. }) => {}
            Step::Ready(Event::Form(form)) => {
                assert_eq!(
                    (form.security_count(), form.security_scheme()),
                    (1, Some("nosec"))
                );
                plans.push(
                    LogicalInteractionPlan::try_property_read(
                        PlanId::new(SlotIndex::new(plans.len() as u32), Generation::INITIAL),
                        ThingId::from(id.as_str()),
                        form.property_name().into(),
                        form.original_index(),
                        form.resolved_href().into(),
                        Some(form.content_type().into()),
                        form.subprotocol().map(Into::into),
                    )
                    .unwrap(),
                );
            }
        }
        td.acknowledge();
    }
    panic!("materialization stalled")
}
fn compile<R: Registration>(
    r: &R,
    plans: &[LogicalInteractionPlan],
    events: &mut Vec<&'static str>,
) -> CoreResult<Vec<BindingArtifactEnvelope<R::Artifact>>> {
    let candidate = candidate(r.identity());
    let mut preparation = budget(4096);
    let mut bounds = Vec::new();
    for plan in plans {
        events.push("bounds");
        preparation
            .consume(W::BindingPolls, r.support().bounds_work())
            .unwrap();
        let bound = r.bounds(&BindingCompilerInput::new(
            plan,
            candidate,
            BindingArtifactRole::ConsumerCall,
        ))?;
        assert!(compiler_envelope(&bound, plans.len() as u64, 16).is_some());
        bounds.push(bound);
    }
    let mut artifacts = Vec::new();
    for (plan, bound) in plans.iter().zip(bounds) {
        let input = BindingCompilerInput::new(plan, candidate, BindingArtifactRole::ConsumerCall);
        events.push("start");
        preparation
            .consume(W::BindingPolls, r.support().start_work())
            .unwrap();
        preparation
            .consume(
                W::CleanupItems,
                r.support().allocation_work()
                    + r.support().abort_work()
                    + r.support().destruction_work()
                    + r.support().release_work(),
            )
            .unwrap();
        let mut cursor = r.start(&input)?;
        let admitted = bound.artifact();
        let mut remainder = bound.into_work();
        loop {
            match r.step(&input, cursor, &mut remainder) {
                BindingCompilerStep::Pending(next) => {
                    assert!(!remainder.is_exhausted());
                    cursor = next;
                }
                BindingCompilerStep::Failed(failure) => {
                    let (error, cursor) = failure.into_parts();
                    r.abort(cursor);
                    return Err(error);
                }
                BindingCompilerStep::Complete(output) => {
                    let artifact = output.into_artifact();
                    let identity = BindingArtifactIdentity::new(
                        PlanSetGeneration::INITIAL,
                        plan.plan_id(),
                        candidate.binding_id(),
                        candidate.binding_generation(),
                        candidate.configuration(),
                        candidate.compatibility(),
                        BindingArtifactRole::ConsumerCall,
                    );
                    let envelope = BindingArtifactEnvelope::try_new(identity, admitted, artifact)
                        .unwrap_or_else(|_| panic!("complete envelope"));
                    artifacts.push(envelope);
                    break;
                }
            }
        }
    }
    Ok(artifacts)
}

fn joined<R: Registration>(source: &Thing, r: &R) -> Vec<BindingArtifactEnvelope<R::Artifact>> {
    let mut td = validate(source).into_property_read();
    preflight(&mut td);
    td = td.rewind();
    let plans = materialize(&mut td);
    assert_eq!(plans.len(), 2);
    assert_eq!(
        (plans[0].property_name(), plans[0].form_index()),
        ("b-read", 1)
    );
    assert_eq!(
        (plans[1].property_name(), plans[1].form_index()),
        ("z-read", 1)
    );
    drop(td);
    let mut events = Vec::new();
    let artifacts = compile(r, &plans, &mut events).unwrap();
    assert_eq!(events, ["bounds", "bounds", "start", "start"]);
    artifacts
}

#[test]
fn production_td_and_checked_static_owner_compose_after_actual_input_destruction() {
    let source = source();
    let compiler = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&compiler);
    let r = production_static_registration(compiler, id);
    let artifacts = joined(&source, &r);
    drop(r);
    drop(source);
    assert_eq!(
        artifacts[0].artifact().payload().target(),
        "https://peer.test/first"
    );
    assert_eq!(
        artifacts[1].artifact().payload().target(),
        "https://peer.test/a/last"
    );
    assert_eq!(
        artifacts[1].identity().binding_generation(),
        id.binding_generation()
    );
}

#[cfg(feature = "std")]
#[test]
fn production_td_and_public_host_owner_preserve_the_same_facts_with_distinct_storage() {
    let source = source();
    let compiler = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&compiler);
    let r = consumer_compiler_support_probe::production_host_registration(compiler, id);
    let slot = r.support().host_slot_layout().unwrap();
    assert!(slot.size() > r.support().artifact_layout().size());
    let artifacts = joined(&source, &r);
    drop(r);
    drop(source);
    for (artifact, expected) in artifacts
        .iter()
        .zip(["https://peer.test/first", "https://peer.test/a/last"])
    {
        assert_eq!(
            artifact
                .artifact()
                .try_payload::<ResolvedTargetArtifact>(id.artifact_compatibility())
                .unwrap()
                .target(),
            expected
        );
    }
}

#[test]
fn later_actual_bounds_failure_prevents_every_start() {
    let source = source();
    let compiler = ResolvedTargetCompiler::try_new(64).unwrap();
    let r = production_static_registration(
        compiler,
        identity(&ResolvedTargetCompiler::try_new(64).unwrap()),
    );
    let mut td = validate(&source).into_property_read();
    let mut plans = materialize(&mut td);
    plans[1] = LogicalInteractionPlan::try_property_read(
        plans[1].plan_id(),
        ThingId::from("urn:joined"),
        "z-read".into(),
        1,
        "x".repeat(65).into_boxed_str(),
        None,
        None,
    )
    .unwrap();
    let mut events = Vec::new();
    assert!(compile(&r, &plans, &mut events).is_err());
    assert_eq!(events, ["bounds", "bounds"]);
}

#[test]
fn malformed_work_and_overflow_reject_without_a_callback() {
    let bound = |work| BindingCompilerBounds::new(BindingArtifactFootprint::new(1, 72), 1, 0, work);
    assert_eq!(compiler_envelope(&bound(WorkBudget::new()), 2, 16), None);
    for class in W::ALL.into_iter().filter(|&class| class != W::BindingPolls) {
        assert_eq!(
            compiler_envelope(
                &bound(
                    WorkBudget::new()
                        .with_remaining(W::BindingPolls, 4)
                        .with_remaining(class, 1)
                ),
                2,
                16
            ),
            None
        );
    }
    assert_eq!(
        compiler_envelope(
            &bound(WorkBudget::new().with_remaining(W::BindingPolls, 17)),
            2,
            16
        ),
        None
    );
    let valid = bound(WorkBudget::new().with_remaining(W::BindingPolls, 4));
    assert_eq!(compiler_envelope(&valid, 2, 16), Some(32));
    assert_eq!(compiler_envelope(&valid, 0, 16), Some(0));
    assert_eq!(compiler_envelope(&valid, u64::MAX, 16), None);
}

#[test]
fn committed_slot_ends_reservation_loan_and_abandonment_ends_input_loans() {
    let source = source();
    let compiler = ResolvedTargetCompiler::try_new(64).unwrap();
    let r = production_static_registration(
        compiler,
        identity(&ResolvedTargetCompiler::try_new(64).unwrap()),
    );
    let size = Layout::new::<Slot<ResolvedTargetArtifact>>().size() as u64;
    let mut account = ResourceAccount::new(
        SlotIndex::new(9),
        Generation::INITIAL,
        ResourceKind::CleanupBytesMax,
        size,
    );
    let mut slot =
        Slot::try_new(account.try_reserve(size).unwrap()).unwrap_or_else(|_| panic!("slot"));
    assert_eq!(account.used(), size); // The mutable reservation loan has ended.
    let mut owner = InputOwner::new(validate(&source), &r, &mut slot);
    let plans = materialize(&mut owner.td);
    let artifact = compile(owner.registration, &plans, &mut Vec::new())
        .unwrap()
        .pop()
        .unwrap()
        .into_artifact()
        .into_payload();
    owner.retain(artifact);
    drop(owner); // Fixed source-free deposit; not a variable rollback proof.
    drop(r);
    drop(source);
    assert_eq!(slot.take().unwrap().target(), "https://peer.test/a/last");
    drop(slot);
    assert!(account.release_committed(size));
    assert_eq!(account.used(), 0);
}

#[test]
fn consuming_owner_returns_concrete_output_and_leaves_its_slot_vacant() {
    for failed in [false, true] {
        let source = source();
        let compiler = ResolvedTargetCompiler::try_new(64).unwrap();
        let id = identity(&compiler);
        let r = production_static_registration(compiler, id);
        let size = Layout::new::<Slot<ResolvedTargetArtifact>>().size() as u64;
        let mut account = ResourceAccount::new(
            SlotIndex::new(9),
            Generation::INITIAL,
            ResourceKind::CleanupBytesMax,
            size,
        );
        let mut slot =
            Slot::try_new(account.try_reserve(size).unwrap()).unwrap_or_else(|_| panic!("slot"));
        let mut owner = InputOwner::new(validate(&source), &r, &mut slot);
        let plans = materialize(&mut owner.td);
        let artifact = compile(owner.registration, &plans, &mut Vec::new())
            .unwrap()
            .pop()
            .unwrap()
            .into_artifact()
            .into_payload();
        owner.retain(artifact);
        let artifact = match if failed { owner.fail() } else { owner.finish() } {
            Outcome::Complete(artifact) => artifact,
            Outcome::Failed {
                cleanup: Some(artifact),
            } => artifact,
            _ => panic!("fixed output expected"),
        };
        drop(r);
        drop(source);
        assert_eq!(artifact.target(), "https://peer.test/a/last");
        assert!(slot.take().is_none());
        assert_eq!(account.used(), size);
        drop(artifact);
        drop(slot);
        assert!(account.release_committed(size));
    }
}

#[test]
fn slot_rejection_returns_the_original_guard_and_rollback_preserves_history() {
    let size = Layout::new::<Slot<ResolvedTargetArtifact>>().size() as u64;
    let mut account = ResourceAccount::new(
        SlotIndex::new(9),
        Generation::INITIAL,
        ResourceKind::CleanupBytesMax,
        size,
    );
    let reservation = account.try_reserve(size - 1).unwrap();
    let rejected = match Slot::<ResolvedTargetArtifact>::try_new(reservation) {
        Err(guard) => guard,
        Ok(_) => panic!("undersized slot"),
    };
    assert_eq!(rejected.bytes(), size - 1);
    drop(rejected);
    assert_eq!((account.used(), account.peak()), (0, size - 1));
}
