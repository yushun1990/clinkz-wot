#![allow(dead_code)]

use clinkz_wot_core::{PlanId, PlanSetGeneration};
use clinkz_wot_foundation::{Generation, SlotIndex};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExecutionPin {
    snapshot_ordinal: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReservationLedger {
    persistent_plan_bytes: u64,
    persistent_artifact_bytes: u64,
    persistent_index_bytes: u64,
    temporary_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CommittedAccount {
    persistent_plan_bytes: u64,
    persistent_artifact_bytes: u64,
    persistent_index_bytes: u64,
    temporary_bytes: u64,
}

impl ReservationLedger {
    fn commit_frozen(self) -> CommittedAccount {
        CommittedAccount {
            persistent_plan_bytes: self.persistent_plan_bytes,
            persistent_artifact_bytes: self.persistent_artifact_bytes,
            persistent_index_bytes: self.persistent_index_bytes,
            temporary_bytes: 0,
        }
    }

    fn release_abort(self) -> ReleasedAccount {
        ReleasedAccount {
            released_bytes: self.persistent_plan_bytes
                + self.persistent_artifact_bytes
                + self.persistent_index_bytes
                + self.temporary_bytes,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReleasedAccount {
    released_bytes: u64,
}

struct IdentityReservation {
    generation: PlanSetGeneration,
    plan_ids: Vec<PlanId>,
}

struct FrozenOwner {
    generation: PlanSetGeneration,
    plan_ids: Vec<PlanId>,
    account: CommittedAccount,
    execution_pin: ExecutionPin,
}

struct PlanSetSlot {
    next_generation: Generation,
    frozen: Option<FrozenOwner>,
}

impl PlanSetSlot {
    fn new() -> Self {
        Self {
            next_generation: Generation::INITIAL,
            frozen: None,
        }
    }

    fn reserve_identities(&self, plan_count: u32) -> IdentityReservation {
        assert!(self.frozen.is_none(), "slot must not have a live Frozen owner");
        let generation = PlanSetGeneration::new(self.next_generation);
        let plan_ids = (0..plan_count)
            .map(|slot| PlanId::new(SlotIndex::new(slot), self.next_generation))
            .collect();
        IdentityReservation {
            generation,
            plan_ids,
        }
    }

    fn abort(
        &mut self,
        identities: IdentityReservation,
        ledger: ReservationLedger,
    ) -> ReleasedAccount {
        assert_eq!(identities.generation.get(), self.next_generation);
        let released = ledger.release_abort();
        self.next_generation = self
            .next_generation
            .checked_next()
            .expect("Stage-A fixture generation must advance");
        released
    }

    fn freeze(
        &mut self,
        identities: IdentityReservation,
        ledger: ReservationLedger,
        execution_pin: ExecutionPin,
    ) -> &FrozenOwner {
        assert!(self.frozen.is_none());
        assert_eq!(identities.generation.get(), self.next_generation);
        let owner = FrozenOwner {
            generation: identities.generation,
            plan_ids: identities.plan_ids,
            account: ledger.commit_frozen(),
            execution_pin,
        };
        self.frozen = Some(owner);
        self.frozen.as_ref().expect("Frozen owner installed")
    }

    fn reclaim_frozen(&mut self) -> ReleasedAccount {
        let frozen = self.frozen.take().expect("Frozen owner must exist");
        let released = ReleasedAccount {
            released_bytes: frozen.account.persistent_plan_bytes
                + frozen.account.persistent_artifact_bytes
                + frozen.account.persistent_index_bytes,
        };
        self.next_generation = self
            .next_generation
            .checked_next()
            .expect("Stage-A fixture generation must advance");
        released
    }
}

fn ledger() -> ReservationLedger {
    ReservationLedger {
        persistent_plan_bytes: 400,
        persistent_artifact_bytes: 300,
        persistent_index_bytes: 100,
        temporary_bytes: 200,
    }
}

#[test]
fn aborted_unpublished_generation_is_invalidated_before_slot_reuse() {
    let mut slot = PlanSetSlot::new();
    let first = slot.reserve_identities(2);
    let old_generation = first.generation;
    let old_plan_ids = first.plan_ids.clone();

    let released = slot.abort(first, ledger());
    assert_eq!(released.released_bytes, 1000);

    let second = slot.reserve_identities(2);
    assert_ne!(second.generation, old_generation);
    for (old, new) in old_plan_ids.iter().zip(second.plan_ids.iter()) {
        assert_eq!(old.slot(), new.slot());
        assert_ne!(old.generation(), new.generation());
    }
}

#[test]
fn freeze_transfers_persistent_capacity_and_execution_pin_into_frozen_owner() {
    let mut slot = PlanSetSlot::new();
    let identities = slot.reserve_identities(2);
    let generation = identities.generation;

    let frozen = slot.freeze(
        identities,
        ledger(),
        ExecutionPin {
            snapshot_ordinal: 3,
        },
    );

    assert_eq!(frozen.generation, generation);
    assert_eq!(frozen.plan_ids.len(), 2);
    assert_eq!(frozen.account.persistent_plan_bytes, 400);
    assert_eq!(frozen.account.persistent_artifact_bytes, 300);
    assert_eq!(frozen.account.persistent_index_bytes, 100);
    assert_eq!(frozen.account.temporary_bytes, 0);
    assert_eq!(frozen.execution_pin.snapshot_ordinal, 3);

    // Freeze releases phase-local temporary reservation but does not erase the
    // persistent accounting that now belongs to the Frozen plan-set record.
    assert_eq!(
        frozen.account.persistent_plan_bytes
            + frozen.account.persistent_artifact_bytes
            + frozen.account.persistent_index_bytes,
        800
    );
}

#[test]
fn frozen_generation_is_not_reused_until_reclaim_releases_persistent_owner() {
    let mut slot = PlanSetSlot::new();
    let identities = slot.reserve_identities(1);
    let frozen_generation = identities.generation;
    slot.freeze(
        identities,
        ledger(),
        ExecutionPin {
            snapshot_ordinal: 7,
        },
    );

    assert_eq!(
        slot.frozen.as_ref().expect("live Frozen owner").generation,
        frozen_generation
    );

    let released = slot.reclaim_frozen();
    assert_eq!(released.released_bytes, 800);

    let next = slot.reserve_identities(1);
    assert_ne!(next.generation, frozen_generation);
}
