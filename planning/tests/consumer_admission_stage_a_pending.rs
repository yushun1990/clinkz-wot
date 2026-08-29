#![allow(dead_code)]

use clinkz_wot_core::{PlanId, PlanSetGeneration};
use clinkz_wot_foundation::{Generation, SlotIndex};
use clinkz_wot_td::thing::Thing;

/// Focused non-production Stage-A constructibility model for the exact
/// substitution-resistance gap across Planning `Pending` boundaries.
///
/// The production API does not exist yet. This fixture demonstrates that a
/// sealed wrapper can own the borrowed source, captured registration snapshot,
/// selected entry, and one opaque PlanId + PlanSetGeneration lease, then move
/// that same authority through every Planning step without accepting a fresh
/// PlanBuildInput (or any replacement identity/source/snapshot argument).
mod pending_model {
    use super::*;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct BuildIdentity {
        plan_id: PlanId,
        plan_set_generation: PlanSetGeneration,
    }

    /// Move-only authority. The raw pair is constructible only by the upstream
    /// identity owner in this model; Planning receives the lease as one value.
    pub struct UnpublishedPlanBuildLease {
        identity: BuildIdentity,
    }

    impl UnpublishedPlanBuildLease {
        fn plan_id(&self) -> PlanId {
            self.identity.plan_id
        }

        fn plan_set_generation(&self) -> PlanSetGeneration {
            self.identity.plan_set_generation
        }
    }

    pub struct PlanSetIdentityAuthority {
        plan_set_generation: PlanSetGeneration,
        next_plan_slot: u32,
        next_plan_generation: Generation,
    }

    impl PlanSetIdentityAuthority {
        pub fn new(
            plan_set_generation: PlanSetGeneration,
            next_plan_slot: u32,
            next_plan_generation: Generation,
        ) -> Self {
            Self {
                plan_set_generation,
                next_plan_slot,
                next_plan_generation,
            }
        }

        pub fn reserve(&mut self) -> UnpublishedPlanBuildLease {
            let identity = BuildIdentity {
                plan_id: PlanId::new(
                    SlotIndex::new(self.next_plan_slot),
                    self.next_plan_generation,
                ),
                plan_set_generation: self.plan_set_generation,
            };

            self.next_plan_slot = self
                .next_plan_slot
                .checked_add(1)
                .expect("Stage-A fixture plan slots stay bounded");
            self.next_plan_generation = self
                .next_plan_generation
                .checked_next()
                .expect("Stage-A fixture plan generations stay bounded");

            UnpublishedPlanBuildLease { identity }
        }
    }

    /// External immutable complete-registration snapshot stand-in. The fixture
    /// only needs a stable entry identity because same-entry compiler/bounds
    /// derivation is covered by consumer_admission_stage_a.rs.
    pub struct RegistrationSnapshot {
        entries: Vec<u32>,
    }

    impl RegistrationSnapshot {
        pub fn new(entries: Vec<u32>) -> Self {
            Self { entries }
        }

        fn entry(&self, ordinal: usize) -> Option<u32> {
            self.entries.get(ordinal).copied()
        }
    }

    /// Validation has already completed. This state owns no caller-replaceable
    /// Planning input; it only carries borrows and the selected snapshot entry.
    pub struct Validated<'td, 'reg> {
        source: &'td Thing,
        snapshot: &'reg RegistrationSnapshot,
        selected_snapshot_ordinal: usize,
    }

    impl<'td, 'reg> Validated<'td, 'reg> {
        pub fn new(
            source: &'td Thing,
            snapshot: &'reg RegistrationSnapshot,
            selected_snapshot_ordinal: usize,
        ) -> Result<Self, ()> {
            snapshot.entry(selected_snapshot_ordinal).ok_or(())?;
            Ok(Self {
                source,
                snapshot,
                selected_snapshot_ordinal,
            })
        }

        /// Consumes Validated and the opaque build lease. No raw PlanId,
        /// PlanSetGeneration, Thing, snapshot, or PlanBuildInput is accepted.
        pub fn enter_planning(
            self,
            lease: UnpublishedPlanBuildLease,
        ) -> Result<Planning<'td, 'reg>, ()> {
            let selected_registration = self
                .snapshot
                .entry(self.selected_snapshot_ordinal)
                .ok_or(())?;

            Ok(Planning {
                source: self.source,
                snapshot: self.snapshot,
                selected_snapshot_ordinal: self.selected_snapshot_ordinal,
                selected_registration,
                lease,
                remaining_steps: 2,
            })
        }
    }

    /// Private call value reconstructed from authority already owned by the
    /// linear Planning transaction. Callers can neither construct nor replace
    /// this value at a Pending boundary.
    struct EphemeralPlanBuildInput<'td, 'reg> {
        source: &'td Thing,
        snapshot: &'reg RegistrationSnapshot,
        plan_id: PlanId,
        plan_set_generation: PlanSetGeneration,
        selected_snapshot_ordinal: usize,
        selected_registration: u32,
    }

    /// Linear Planning owner. Deliberately not Clone/Copy.
    pub struct Planning<'td, 'reg> {
        source: &'td Thing,
        snapshot: &'reg RegistrationSnapshot,
        selected_snapshot_ordinal: usize,
        selected_registration: u32,
        lease: UnpublishedPlanBuildLease,
        remaining_steps: u8,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct ObservedBuildInput {
        pub source_address: usize,
        pub snapshot_address: usize,
        pub plan_id: PlanId,
        pub plan_set_generation: PlanSetGeneration,
        pub selected_snapshot_ordinal: usize,
        pub selected_registration: u32,
    }

    pub enum PlanningProgress<'td, 'reg> {
        /// The transaction itself is moved into Pending. Resumption can only
        /// consume this exact value; there is no step(input) substitution path.
        Pending {
            transaction: Planning<'td, 'reg>,
            observed: ObservedBuildInput,
        },
        Complete {
            observed: ObservedBuildInput,
        },
    }

    impl<'td, 'reg> Planning<'td, 'reg> {
        fn ephemeral_input(&self) -> EphemeralPlanBuildInput<'td, 'reg> {
            EphemeralPlanBuildInput {
                source: self.source,
                snapshot: self.snapshot,
                plan_id: self.lease.plan_id(),
                plan_set_generation: self.lease.plan_set_generation(),
                selected_snapshot_ordinal: self.selected_snapshot_ordinal,
                selected_registration: self.selected_registration,
            }
        }

        /// One sealed Planning step. The method consumes self and accepts no
        /// replacement source, registration, lease, PlanId, generation, or
        /// PlanBuildInput. Pending therefore carries the only resumable owner.
        pub fn step(mut self) -> PlanningProgress<'td, 'reg> {
            let observed = {
                let input = self.ephemeral_input();
                ObservedBuildInput {
                    source_address: input.source as *const Thing as usize,
                    snapshot_address: input.snapshot as *const RegistrationSnapshot as usize,
                    plan_id: input.plan_id,
                    plan_set_generation: input.plan_set_generation,
                    selected_snapshot_ordinal: input.selected_snapshot_ordinal,
                    selected_registration: input.selected_registration,
                }
            };

            self.remaining_steps = self
                .remaining_steps
                .checked_sub(1)
                .expect("Stage-A fixture has a fixed two-step compiler");

            if self.remaining_steps == 0 {
                PlanningProgress::Complete { observed }
            } else {
                PlanningProgress::Pending {
                    transaction: self,
                    observed,
                }
            }
        }
    }
}

#[test]
fn pending_step_preserves_one_owned_build_authority_without_input_substitution() {
    let thing = Thing::default();
    let snapshot = pending_model::RegistrationSnapshot::new(vec![101, 202]);
    let validated = pending_model::Validated::new(&thing, &snapshot, 1)
        .expect("selected registration exists");

    let plan_set_generation =
        PlanSetGeneration::new(Generation::new(9).expect("nonzero generation"));
    let mut identity_authority = pending_model::PlanSetIdentityAuthority::new(
        plan_set_generation,
        4,
        Generation::new(7).expect("nonzero generation"),
    );
    let planning = validated
        .enter_planning(identity_authority.reserve())
        .expect("sealed Planning entry succeeds");

    let (pending, first) = match planning.step() {
        pending_model::PlanningProgress::Pending {
            transaction,
            observed,
        } => (transaction, observed),
        pending_model::PlanningProgress::Complete { .. } => {
            panic!("the first model step must remain Pending")
        }
    };

    let second = match pending.step() {
        pending_model::PlanningProgress::Complete { observed } => observed,
        pending_model::PlanningProgress::Pending { .. } => {
            panic!("the second model step must complete")
        }
    };

    assert_eq!(first, second);
    assert_eq!(first.source_address, &thing as *const Thing as usize);
    assert_eq!(
        first.snapshot_address,
        &snapshot as *const pending_model::RegistrationSnapshot as usize
    );
    assert_eq!(first.plan_id.slot(), SlotIndex::new(4));
    assert_eq!(first.plan_id.generation().get(), 7);
    assert_eq!(first.plan_set_generation, plan_set_generation);
    assert_eq!(first.selected_snapshot_ordinal, 1);
    assert_eq!(first.selected_registration, 202);
}
