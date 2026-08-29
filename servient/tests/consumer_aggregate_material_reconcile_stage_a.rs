#![allow(dead_code)]

use std::{collections::BTreeMap, mem::size_of};

use clinkz_wot_core::{
    BindingArtifact, BindingArtifactCompatibility, BindingArtifactEnvelope,
    BindingArtifactFootprint, BindingArtifactIdentity, BindingArtifactRef, BindingArtifactRole,
    BindingCandidate, BindingConfigurationDigest, BindingGeneration, BindingId,
    LogicalInteractionPlan, PlanId, PlanSetGeneration, ThingId,
};
use clinkz_wot_foundation::{Generation, SlotIndex};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CoordinateKey {
    property: String,
    form_index: u32,
}

#[derive(Debug, Eq, PartialEq)]
struct TestArtifact(Vec<u8>);

struct BoundCoordinate {
    plan: LogicalInteractionPlan,
    candidate: BindingCandidate,
    admitted: BindingArtifactFootprint,
}

impl BoundCoordinate {
    fn new(
        plan: LogicalInteractionPlan,
        candidate: BindingCandidate,
        admitted: BindingArtifactFootprint,
    ) -> Self {
        Self {
            plan,
            candidate,
            admitted,
        }
    }

    fn materialize(
        self,
        plan_set_generation: PlanSetGeneration,
        artifact_slot: SlotIndex,
        payload_bytes: usize,
    ) -> BuiltCoordinate {
        let identity = BindingArtifactIdentity::new(
            plan_set_generation,
            self.plan.plan_id(),
            self.candidate.binding_id(),
            self.candidate.binding_generation(),
            self.candidate.configuration(),
            self.candidate.compatibility(),
            BindingArtifactRole::ConsumerCall,
        );
        let measured = BindingArtifactFootprint::new(1, payload_bytes as u64);
        let artifact = BindingArtifact::new(
            self.candidate.compatibility(),
            measured,
            TestArtifact(vec![artifact_slot.get() as u8; payload_bytes]),
        );
        let envelope = BindingArtifactEnvelope::try_new(identity, self.admitted, artifact)
            .expect("measured artifact fits admitted bounds");
        let artifact_ref = BindingArtifactRef::new(identity, artifact_slot);
        let key = CoordinateKey {
            property: self.plan.property_name().to_owned(),
            form_index: self.plan.form_index(),
        };

        BuiltCoordinate {
            key,
            plan: self.plan,
            envelope,
            artifact_ref,
        }
    }
}

struct BuiltCoordinate {
    key: CoordinateKey,
    plan: LogicalInteractionPlan,
    envelope: BindingArtifactEnvelope<TestArtifact>,
    artifact_ref: BindingArtifactRef,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct MeasuredLedger {
    logical_plans: u32,
    logical_bytes: u64,
    artifacts: u32,
    artifact_bytes: u64,
    artifact_refs: u32,
    index_entries: u32,
    index_bytes: u64,
}

impl MeasuredLedger {
    fn fits(self, ceiling: ReservationCeiling) -> bool {
        self.logical_plans <= ceiling.logical_plans
            && self.logical_bytes <= ceiling.logical_bytes
            && self.artifacts <= ceiling.artifacts
            && self.artifact_bytes <= ceiling.artifact_bytes
            && self.artifact_refs <= ceiling.artifact_refs
            && self.index_entries <= ceiling.index_entries
            && self.index_bytes <= ceiling.index_bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReservationCeiling {
    logical_plans: u32,
    logical_bytes: u64,
    artifacts: u32,
    artifact_bytes: u64,
    artifact_refs: u32,
    index_entries: u32,
    index_bytes: u64,
    temporary_bytes: u64,
}

struct ReservationLease {
    ceiling: ReservationCeiling,
}

impl ReservationLease {
    fn commit(self, measured: MeasuredLedger) -> CommittedLedger {
        assert!(measured.fits(self.ceiling));
        CommittedLedger {
            measured,
            persistent_reserved_bytes: measured.logical_bytes
                + measured.artifact_bytes
                + measured.index_bytes
                + measured.artifact_refs as u64 * size_of::<BindingArtifactRef>() as u64,
            temporary_reserved_bytes: 0,
        }
    }

    fn release(self) -> ReservationRelease {
        ReservationRelease {
            released_persistent_capacity: self.ceiling.logical_bytes
                + self.ceiling.artifact_bytes
                + self.ceiling.index_bytes,
            released_temporary_capacity: self.ceiling.temporary_bytes,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CommittedLedger {
    measured: MeasuredLedger,
    persistent_reserved_bytes: u64,
    temporary_reserved_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReservationRelease {
    released_persistent_capacity: u64,
    released_temporary_capacity: u64,
}

struct AggregateDraft {
    plan_set_generation: PlanSetGeneration,
    plans: Vec<LogicalInteractionPlan>,
    artifacts: Vec<BindingArtifactEnvelope<TestArtifact>>,
    artifact_refs: Vec<BindingArtifactRef>,
    index: BTreeMap<CoordinateKey, SlotIndex>,
    measured: MeasuredLedger,
}

#[derive(Default)]
struct DraftBuilder {
    plans: Vec<LogicalInteractionPlan>,
    artifacts: Vec<BindingArtifactEnvelope<TestArtifact>>,
    artifact_refs: Vec<BindingArtifactRef>,
    index: BTreeMap<CoordinateKey, SlotIndex>,
}

impl DraftBuilder {
    fn push(&mut self, built: BuiltCoordinate) {
        let expected_slot = SlotIndex::new(self.artifacts.len() as u32);
        assert_eq!(built.artifact_ref.artifact_slot(), expected_slot);
        assert!(self.index.insert(built.key, expected_slot).is_none());
        self.plans.push(built.plan);
        self.artifacts.push(built.envelope);
        self.artifact_refs.push(built.artifact_ref);
    }

    fn seal(self, generation: PlanSetGeneration) -> AggregateDraft {
        let measured = measure(
            &self.plans,
            &self.artifacts,
            &self.artifact_refs,
            &self.index,
        );
        AggregateDraft {
            plan_set_generation: generation,
            plans: self.plans,
            artifacts: self.artifacts,
            artifact_refs: self.artifact_refs,
            index: self.index,
            measured,
        }
    }

    fn abort(self) -> ProvisionalRelease {
        ProvisionalRelease {
            plans: self.plans.len() as u32,
            artifacts: self.artifacts.len() as u32,
            artifact_refs: self.artifact_refs.len() as u32,
            index_entries: self.index.len() as u32,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ProvisionalRelease {
    plans: u32,
    artifacts: u32,
    artifact_refs: u32,
    index_entries: u32,
}

struct FrozenOwner {
    draft: AggregateDraft,
    committed: CommittedLedger,
}

fn reconcile_and_freeze(draft: AggregateDraft, reservation: ReservationLease) -> FrozenOwner {
    assert!(draft.measured.fits(reservation.ceiling));
    assert_eq!(draft.plans.len(), draft.artifacts.len());
    assert_eq!(draft.plans.len(), draft.artifact_refs.len());
    assert_eq!(draft.plans.len(), draft.index.len());

    for (position, ((plan, envelope), artifact_ref)) in draft
        .plans
        .iter()
        .zip(draft.artifacts.iter())
        .zip(draft.artifact_refs.iter())
        .enumerate()
    {
        let identity = envelope.identity();
        assert_eq!(identity.plan_set_generation(), draft.plan_set_generation);
        assert_eq!(identity.plan_id(), plan.plan_id());
        assert_eq!(artifact_ref.identity(), identity);
        assert_eq!(artifact_ref.artifact_slot(), SlotIndex::new(position as u32));

        let key = CoordinateKey {
            property: plan.property_name().to_owned(),
            form_index: plan.form_index(),
        };
        assert_eq!(draft.index.get(&key), Some(&artifact_ref.artifact_slot()));
    }

    let committed = reservation.commit(draft.measured);
    FrozenOwner { draft, committed }
}

fn measure(
    plans: &[LogicalInteractionPlan],
    artifacts: &[BindingArtifactEnvelope<TestArtifact>],
    refs: &[BindingArtifactRef],
    index: &BTreeMap<CoordinateKey, SlotIndex>,
) -> MeasuredLedger {
    let logical_bytes = plans.iter().map(measure_plan).sum();
    let artifact_bytes = artifacts
        .iter()
        .map(|envelope| envelope.artifact().footprint().retained_bytes())
        .sum();
    let index_bytes = index
        .keys()
        .map(|key| {
            key.property.len() as u64
                + size_of::<u32>() as u64
                + size_of::<SlotIndex>() as u64
        })
        .sum();

    MeasuredLedger {
        logical_plans: plans.len() as u32,
        logical_bytes,
        artifacts: artifacts.len() as u32,
        artifact_bytes,
        artifact_refs: refs.len() as u32,
        index_entries: index.len() as u32,
        index_bytes,
    }
}

fn measure_plan(plan: &LogicalInteractionPlan) -> u64 {
    size_of::<LogicalInteractionPlan>() as u64
        + plan.thing_id().as_str().len() as u64
        + plan.property_name().len() as u64
        + plan.resolved_target().len() as u64
        + plan.content_type().map_or(0, |value| value.len() as u64)
        + plan.subprotocol().map_or(0, |value| value.len() as u64)
}

fn plan_id(slot: u32) -> PlanId {
    PlanId::new(SlotIndex::new(slot), Generation::INITIAL)
}

fn plan(slot: u32, property: &str, form_index: u32) -> LogicalInteractionPlan {
    LogicalInteractionPlan::try_property_read(
        plan_id(slot),
        ThingId::from("urn:test:aggregate-material"),
        Box::from(property),
        form_index,
        Box::from(format!("mock://thing/{property}/{form_index}")),
        Some(Box::from("application/json")),
        None,
    )
    .expect("valid logical plan")
}

fn candidate() -> BindingCandidate {
    BindingCandidate::new(
        BindingId::new(7),
        BindingGeneration::INITIAL,
        BindingConfigurationDigest::new([7; 32]),
        BindingArtifactCompatibility::new([9; 16]),
        3,
        0,
    )
}

fn bound(slot: u32, property: &str, form_index: u32, admitted_bytes: u64) -> BoundCoordinate {
    BoundCoordinate::new(
        plan(slot, property, form_index),
        candidate(),
        BindingArtifactFootprint::new(1, admitted_bytes),
    )
}

fn reservation() -> ReservationLease {
    ReservationLease {
        ceiling: ReservationCeiling {
            logical_plans: 2,
            logical_bytes: 4096,
            artifacts: 2,
            artifact_bytes: 256,
            artifact_refs: 2,
            index_entries: 2,
            index_bytes: 512,
            temporary_bytes: 1024,
        },
    }
}

#[test]
fn frozen_owner_retains_complete_real_aggregate_material_and_committed_ledger() {
    let generation = PlanSetGeneration::INITIAL;
    let mut builder = DraftBuilder::default();

    // Each BoundCoordinate owns the exact final LogicalInteractionPlan before
    // materialization. Build consumes that value; it does not reconstruct a
    // second plan from PlanId-only facts.
    builder.push(bound(0, "humidity", 1, 128).materialize(
        generation,
        SlotIndex::new(0),
        48,
    ));
    builder.push(bound(1, "temperature", 2, 128).materialize(
        generation,
        SlotIndex::new(1),
        64,
    ));

    let draft = builder.seal(generation);
    assert_eq!(draft.measured.logical_plans, 2);
    assert_eq!(draft.measured.artifacts, 2);
    assert_eq!(draft.measured.artifact_refs, 2);
    assert_eq!(draft.measured.index_entries, 2);
    assert_eq!(draft.measured.artifact_bytes, 112);

    let frozen = reconcile_and_freeze(draft, reservation());
    assert_eq!(frozen.draft.plans.len(), 2);
    assert_eq!(frozen.draft.artifacts.len(), 2);
    assert_eq!(frozen.draft.artifact_refs.len(), 2);
    assert_eq!(frozen.draft.index.len(), 2);
    assert!(frozen.committed.persistent_reserved_bytes > 0);
    assert_eq!(frozen.committed.temporary_reserved_bytes, 0);
    assert_eq!(frozen.committed.measured.artifact_bytes, 112);
}

#[test]
fn partial_aggregate_success_is_owned_and_released_on_abort() {
    let generation = PlanSetGeneration::INITIAL;
    let mut builder = DraftBuilder::default();

    // Coordinate 0 completed before coordinate 1 later failed/pended. Its real
    // provisional plan/envelope/ref/index material remains owned by the
    // aggregate builder until Aborting consumes it.
    builder.push(bound(0, "humidity", 1, 128).materialize(
        generation,
        SlotIndex::new(0),
        48,
    ));

    let released_material = builder.abort();
    let released_reservation = reservation().release();

    assert_eq!(
        released_material,
        ProvisionalRelease {
            plans: 1,
            artifacts: 1,
            artifact_refs: 1,
            index_entries: 1,
        }
    );
    assert!(released_reservation.released_persistent_capacity > 0);
    assert_eq!(released_reservation.released_temporary_capacity, 1024);
}

#[test]
fn reconciliation_rejects_identity_or_index_substitution() {
    let generation = PlanSetGeneration::INITIAL;
    let mut builder = DraftBuilder::default();
    builder.push(bound(0, "humidity", 1, 128).materialize(
        generation,
        SlotIndex::new(0),
        48,
    ));
    builder.push(bound(1, "temperature", 2, 128).materialize(
        generation,
        SlotIndex::new(1),
        64,
    ));
    let mut draft = builder.seal(generation);

    let temperature = CoordinateKey {
        property: "temperature".to_owned(),
        form_index: 2,
    };
    draft.index.insert(temperature, SlotIndex::new(0));

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = reconcile_and_freeze(draft, reservation());
    }));
    assert!(result.is_err(), "mismatched index must fail reconciliation");
}
