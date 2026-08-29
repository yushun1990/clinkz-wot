#![allow(dead_code)]

use core::cell::Cell;
use clinkz_wot_core::{PlanId, PlanSetGeneration};
use clinkz_wot_foundation::{Generation, SlotIndex};
use clinkz_wot_td::thing::Thing;
use std::rc::Rc;

/// Focused non-production Stage-A constructibility model for substitution
/// resistance across Planning `Pending` boundaries.
mod pending_model {
    use super::*;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct BuildIdentity {
        plan_id: PlanId,
        plan_set_generation: PlanSetGeneration,
    }

    #[must_use]
    pub struct UnpublishedPlanBuildLease {
        identity: BuildIdentity,
        outstanding: Rc<Cell<u32>>,
        active: bool,
    }

    impl UnpublishedPlanBuildLease {
        fn plan_id(&self) -> PlanId {
            self.identity.plan_id
        }

        fn plan_set_generation(&self) -> PlanSetGeneration {
            self.identity.plan_set_generation
        }

        fn settle(&mut self) {
            if self.active {
                self.outstanding.set(
                    self.outstanding
                        .get()
                        .checked_sub(1)
                        .expect("active lease has an outstanding reservation"),
                );
                self.active = false;
            }
        }

        pub fn release(mut self) {
            self.settle();
        }
    }

    impl Drop for UnpublishedPlanBuildLease {
        fn drop(&mut self) {
            self.settle();
        }
    }

    pub struct PlanSetIdentityAuthority {
        plan_set_generation: PlanSetGeneration,
        next_plan_slot: u32,
        next_plan_generation: Generation,
        outstanding: Rc<Cell<u32>>,
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
                outstanding: Rc::new(Cell::new(0)),
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
                .expect("fixture plan slots stay bounded");
            self.next_plan_generation = self
                .next_plan_generation
                .checked_next()
                .expect("fixture plan generations stay bounded");
            self.outstanding.set(self.outstanding.get() + 1);
            UnpublishedPlanBuildLease {
                identity,
                outstanding: Rc::clone(&self.outstanding),
                active: true,
            }
        }

        pub fn outstanding(&self) -> u32 {
            self.outstanding.get()
        }
    }

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
        ) -> Self {
            Self {
                source,
                snapshot,
                selected_snapshot_ordinal,
            }
        }

        pub fn enter_planning(
            self,
            lease: UnpublishedPlanBuildLease,
        ) -> Result<Planning<'td, 'reg>, PlanningEntryRejection<'td, 'reg>> {
            let selected_registration = match self.snapshot.entry(self.selected_snapshot_ordinal) {
                Some(entry) => entry,
                None => {
                    return Err(PlanningEntryRejection {
                        validated: self,
                        lease,
                    });
                }
            };

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

    pub struct PlanningEntryRejection<'td, 'reg> {
        validated: Validated<'td, 'reg>,
        lease: UnpublishedPlanBuildLease,
    }

    impl<'td, 'reg> PlanningEntryRejection<'td, 'reg> {
        pub fn into_parts(self) -> (Validated<'td, 'reg>, UnpublishedPlanBuildLease) {
            (self.validated, self.lease)
        }
    }

    struct EphemeralPlanBuildInput<'td, 'reg> {
        source: &'td Thing,
        snapshot: &'reg RegistrationSnapshot,
        plan_id: PlanId,
        plan_set_generation: PlanSetGeneration,
        selected_snapshot_ordinal: usize,
        selected_registration: u32,
    }

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
        Pending {
            transaction: Planning<'td, 'reg>,
            observed: ObservedBuildInput,
        },
        Complete {
            transaction: Planning<'td, 'reg>,
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
                .expect("fixture has a fixed two-step compiler");

            if self.remaining_steps == 0 {
                PlanningProgress::Complete {
                    transaction: self,
                    observed,
                }
            } else {
                PlanningProgress::Pending {
                    transaction: self,
                    observed,
                }
            }
        }

        pub fn abort(self) -> UnpublishedPlanBuildLease {
            self.lease
        }
    }
}

#[test]
fn pending_step_preserves_one_owned_build_authority_without_input_substitution() {
    let thing = Thing::default();
    let snapshot = pending_model::RegistrationSnapshot::new(vec![101, 202]);
    let validated = pending_model::Validated::new(&thing, &snapshot, 1);

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
    assert_eq!(identity_authority.outstanding(), 1);

    let (pending, first) = match planning.step() {
        pending_model::PlanningProgress::Pending {
            transaction,
            observed,
        } => (transaction, observed),
        pending_model::PlanningProgress::Complete { .. } => {
            panic!("first model step must remain Pending")
        }
    };

    let (complete, second) = match pending.step() {
        pending_model::PlanningProgress::Complete {
            transaction,
            observed,
        } => (transaction, observed),
        pending_model::PlanningProgress::Pending { .. } => {
            panic!("second model step must complete")
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

    complete.abort().release();
    assert_eq!(identity_authority.outstanding(), 0);
}

#[test]
fn rejected_entry_returns_the_exact_lease_for_release() {
    let thing = Thing::default();
    let snapshot = pending_model::RegistrationSnapshot::new(vec![101]);
    let validated = pending_model::Validated::new(&thing, &snapshot, 9);
    let mut authority = pending_model::PlanSetIdentityAuthority::new(
        PlanSetGeneration::new(Generation::new(3).unwrap()),
        0,
        Generation::new(2).unwrap(),
    );

    let rejection = validated
        .enter_planning(authority.reserve())
        .expect_err("missing selected registration must reject");
    assert_eq!(authority.outstanding(), 1);
    let (_validated, lease) = rejection.into_parts();
    lease.release();
    assert_eq!(authority.outstanding(), 0);
}
