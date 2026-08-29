#![allow(dead_code)]

use clinkz_wot_core::{
    BindingArtifactCompatibility, BindingConfigurationDigest, BindingGeneration, BindingId,
    BindingRegistrationIdentity,
};

/// Non-production ownership proof for workspace/0063.
///
/// One selected complete registration is split only by Servient into:
/// - an immutable Planning projection containing build identity; and
/// - a persistent execution pin borrowing the exact same complete entry.
///
/// Neither projection has a public constructor in this fixture. The purpose is
/// to prove that build-time compiler identity and post-Frozen execution owner
/// can be derived atomically without letting Planning retain execution authority
/// or reconstruct a registration by compatible-looking fields.
mod stage_a {
    use super::*;

    #[derive(Debug)]
    struct CompleteRegistration {
        identity: BindingRegistrationIdentity,
        snapshot_ordinal: u32,
        compiler_tag: u32,
        execution_tag: u32,
    }

    impl CompleteRegistration {
        fn fixture(
            binding_id: u32,
            snapshot_ordinal: u32,
            diagnostic_ordinal: u32,
            compiler_tag: u32,
            execution_tag: u32,
        ) -> Self {
            Self {
                identity: BindingRegistrationIdentity::new(
                    BindingId::new(binding_id),
                    BindingGeneration::INITIAL,
                    BindingConfigurationDigest::new([binding_id as u8; 32]),
                    BindingArtifactCompatibility::new([binding_id as u8; 16]),
                    diagnostic_ordinal,
                ),
                snapshot_ordinal,
                compiler_tag,
                execution_tag,
            }
        }
    }

    struct RegistrationSnapshot {
        entries: Vec<CompleteRegistration>,
    }

    impl RegistrationSnapshot {
        fn new(entries: Vec<CompleteRegistration>) -> Self {
            Self { entries }
        }

        fn select_unique_consumer_for_first_proof(
            &self,
            snapshot_ordinal: u32,
        ) -> Option<SelectedCompleteRegistration<'_>> {
            let entry = self.entries.get(snapshot_ordinal as usize)?;
            if entry.snapshot_ordinal != snapshot_ordinal {
                return None;
            }
            Some(SelectedCompleteRegistration { entry })
        }
    }

    struct SelectedCompleteRegistration<'reg> {
        entry: &'reg CompleteRegistration,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct PlanningRegistrationView {
        identity: BindingRegistrationIdentity,
        snapshot_ordinal: u32,
        compiler_tag: u32,
    }

    #[derive(Debug)]
    struct ExecutionRegistrationPin<'reg> {
        entry: &'reg CompleteRegistration,
    }

    impl<'reg> SelectedCompleteRegistration<'reg> {
        /// The only split point. Both outputs originate from one borrowed entry.
        fn split_for_build_and_runtime(
            self,
        ) -> (PlanningRegistrationView, ExecutionRegistrationPin<'reg>) {
            (
                PlanningRegistrationView {
                    identity: self.entry.identity,
                    snapshot_ordinal: self.entry.snapshot_ordinal,
                    compiler_tag: self.entry.compiler_tag,
                },
                ExecutionRegistrationPin { entry: self.entry },
            )
        }
    }

    struct PlanningDraft {
        registration: PlanningRegistrationView,
        artifact_compiler_tag: u32,
    }

    struct FrozenConsumerPlanSet<'reg> {
        registration: PlanningRegistrationView,
        execution: ExecutionRegistrationPin<'reg>,
        artifact_compiler_tag: u32,
    }

    fn freeze<'reg>(
        draft: PlanningDraft,
        execution: ExecutionRegistrationPin<'reg>,
    ) -> Result<FrozenConsumerPlanSet<'reg>, &'static str> {
        let entry = execution.entry;
        if draft.registration.identity != entry.identity
            || draft.registration.snapshot_ordinal != entry.snapshot_ordinal
            || draft.registration.compiler_tag != entry.compiler_tag
            || draft.artifact_compiler_tag != entry.compiler_tag
        {
            return Err("draft and execution pin are not the same complete registration");
        }
        Ok(FrozenConsumerPlanSet {
            registration: draft.registration,
            execution,
            artifact_compiler_tag: draft.artifact_compiler_tag,
        })
    }

    #[test]
    fn planning_view_and_execution_pin_come_from_one_complete_entry() {
        let snapshot = RegistrationSnapshot::new(vec![
            CompleteRegistration::fixture(10, 0, 90, 100, 1000),
            CompleteRegistration::fixture(11, 1, 7, 101, 1001),
        ]);

        let selected = snapshot
            .select_unique_consumer_for_first_proof(1)
            .expect("fixture selected complete registration exists");
        let (planning, execution) = selected.split_for_build_and_runtime();

        // Snapshot ordinal and diagnostic ordinal are deliberately different.
        assert_eq!(planning.snapshot_ordinal, 1);
        assert_eq!(planning.identity.diagnostic_ordinal(), 7);
        assert_eq!(planning.compiler_tag, 101);
        assert_eq!(execution.entry.execution_tag, 1001);
        assert_eq!(execution.entry.identity, planning.identity);

        let frozen = freeze(
            PlanningDraft {
                registration: planning,
                artifact_compiler_tag: planning.compiler_tag,
            },
            execution,
        )
        .expect("same-entry build and execution owner may freeze");

        assert_eq!(frozen.registration.identity.binding_id(), BindingId::new(11));
        assert_eq!(frozen.execution.entry.execution_tag, 1001);
        assert_eq!(frozen.artifact_compiler_tag, 101);
    }

    #[test]
    fn compatible_looking_but_different_entry_cannot_freeze() {
        let compatibility = BindingArtifactCompatibility::new([4; 16]);
        let configuration = BindingConfigurationDigest::new([8; 32]);
        let entries = vec![
            CompleteRegistration {
                identity: BindingRegistrationIdentity::new(
                    BindingId::new(20),
                    BindingGeneration::INITIAL,
                    configuration,
                    compatibility,
                    3,
                ),
                snapshot_ordinal: 0,
                compiler_tag: 200,
                execution_tag: 2000,
            },
            CompleteRegistration {
                identity: BindingRegistrationIdentity::new(
                    BindingId::new(21),
                    BindingGeneration::INITIAL,
                    configuration,
                    compatibility,
                    9,
                ),
                snapshot_ordinal: 1,
                compiler_tag: 201,
                execution_tag: 2001,
            },
        ];
        let snapshot = RegistrationSnapshot::new(entries);

        let (planning_a, _pin_a) = snapshot
            .select_unique_consumer_for_first_proof(0)
            .unwrap()
            .split_for_build_and_runtime();
        let (_planning_b, pin_b) = snapshot
            .select_unique_consumer_for_first_proof(1)
            .unwrap()
            .split_for_build_and_runtime();

        let result = freeze(
            PlanningDraft {
                registration: planning_a,
                artifact_compiler_tag: planning_a.compiler_tag,
            },
            pin_b,
        );
        assert!(result.is_err());
    }
}
