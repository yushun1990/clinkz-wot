#![allow(dead_code)]

use core::cell::Cell;

/// Non-production Stage-A constructibility model for the application-static
/// cancellation boundary in workspace/0063.
///
/// The first proof intentionally models only caller-driven Servient progress:
/// `begin_destroy()` is the static cancellation request owner, admissions hold
/// a read-only cancellation view, every progress step checks the request before
/// semantic callbacks, and the first terminal cause is immutable.
mod stage_a {
    use super::*;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum StaticLifecycle {
        Active,
        DestroyRequested,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum TerminalCause {
        CancelledByDestroy { request_generation: u32 },
        SemanticFailure,
    }

    struct StaticServientOwner {
        lifecycle: Cell<StaticLifecycle>,
        destroy_generation: Cell<u32>,
    }

    impl StaticServientOwner {
        fn new() -> Self {
            Self {
                lifecycle: Cell::new(StaticLifecycle::Active),
                destroy_generation: Cell::new(0),
            }
        }

        fn begin_destroy(&self) -> u32 {
            if self.lifecycle.get() == StaticLifecycle::Active {
                let next = self
                    .destroy_generation
                    .get()
                    .checked_add(1)
                    .expect("fixture destroy generation cannot overflow");
                self.destroy_generation.set(next);
                self.lifecycle.set(StaticLifecycle::DestroyRequested);
            }
            self.destroy_generation.get()
        }

        fn begin_admission(&self) -> Result<StaticCancellationView<'_>, TerminalCause> {
            if self.lifecycle.get() == StaticLifecycle::DestroyRequested {
                return Err(TerminalCause::CancelledByDestroy {
                    request_generation: self.destroy_generation.get(),
                });
            }
            Ok(StaticCancellationView {
                owner: self,
                captured_destroy_generation: self.destroy_generation.get(),
            })
        }
    }

    #[derive(Clone, Copy)]
    struct StaticCancellationView<'a> {
        owner: &'a StaticServientOwner,
        captured_destroy_generation: u32,
    }

    impl StaticCancellationView<'_> {
        fn requested(self) -> Option<TerminalCause> {
            let current = self.owner.destroy_generation.get();
            if self.owner.lifecycle.get() == StaticLifecycle::DestroyRequested
                && current != self.captured_destroy_generation
            {
                Some(TerminalCause::CancelledByDestroy {
                    request_generation: current,
                })
            } else {
                None
            }
        }
    }

    struct StaticAdmission<'a> {
        cancellation: StaticCancellationView<'a>,
        first_cause: Option<TerminalCause>,
        callback_calls: u32,
        frozen: bool,
    }

    impl<'a> StaticAdmission<'a> {
        fn new(cancellation: StaticCancellationView<'a>) -> Self {
            Self {
                cancellation,
                first_cause: None,
                callback_calls: 0,
                frozen: false,
            }
        }

        fn record_first_cause(&mut self, cause: TerminalCause) {
            if self.first_cause.is_none() {
                self.first_cause = Some(cause);
            }
        }

        fn poll_cancellation(&mut self) -> bool {
            if let Some(cause) = self.cancellation.requested() {
                self.record_first_cause(cause);
            }
            self.first_cause.is_some()
        }

        /// Caller-driven static progress. Cancellation is linearized before
        /// invoking any semantic callback in the step.
        fn step(&mut self, semantic_failure: bool) {
            if self.poll_cancellation() {
                return;
            }

            self.callback_calls += 1;
            if semantic_failure {
                self.record_first_cause(TerminalCause::SemanticFailure);
            }
        }

        /// The same cancellation source is checked immediately before the
        /// unpublished Frozen transition.
        fn try_freeze(&mut self) -> Result<(), TerminalCause> {
            self.poll_cancellation();
            if let Some(cause) = self.first_cause {
                return Err(cause);
            }
            self.frozen = true;
            Ok(())
        }
    }

    #[test]
    fn begin_destroy_is_the_static_admission_cancellation_owner() {
        let owner = StaticServientOwner::new();
        let view = owner.begin_admission().expect("active owner admits work");
        let mut admission = StaticAdmission::new(view);

        let request_generation = owner.begin_destroy();
        admission.step(false);

        assert_eq!(admission.callback_calls, 0);
        assert_eq!(
            admission.first_cause,
            Some(TerminalCause::CancelledByDestroy { request_generation })
        );
    }

    #[test]
    fn first_terminal_cause_is_immutable() {
        let owner = StaticServientOwner::new();
        let view = owner.begin_admission().expect("active owner admits work");
        let mut admission = StaticAdmission::new(view);

        admission.step(true);
        assert_eq!(admission.first_cause, Some(TerminalCause::SemanticFailure));

        owner.begin_destroy();
        admission.poll_cancellation();
        assert_eq!(admission.first_cause, Some(TerminalCause::SemanticFailure));
    }

    #[test]
    fn destroy_between_steps_wins_before_the_next_callback() {
        let owner = StaticServientOwner::new();
        let view = owner.begin_admission().expect("active owner admits work");
        let mut admission = StaticAdmission::new(view);

        admission.step(false);
        assert_eq!(admission.callback_calls, 1);

        let request_generation = owner.begin_destroy();
        admission.step(true);

        assert_eq!(admission.callback_calls, 1);
        assert_eq!(
            admission.first_cause,
            Some(TerminalCause::CancelledByDestroy { request_generation })
        );
    }

    #[test]
    fn cancellation_is_rechecked_immediately_before_freeze() {
        let owner = StaticServientOwner::new();
        let view = owner.begin_admission().expect("active owner admits work");
        let mut admission = StaticAdmission::new(view);

        owner.begin_destroy();
        let result = admission.try_freeze();

        assert!(matches!(
            result,
            Err(TerminalCause::CancelledByDestroy { .. })
        ));
        assert!(!admission.frozen);
    }

    #[test]
    fn admission_cannot_start_after_destroy_is_requested() {
        let owner = StaticServientOwner::new();
        let request_generation = owner.begin_destroy();

        let result = owner.begin_admission();
        assert!(matches!(
            result,
            Err(TerminalCause::CancelledByDestroy {
                request_generation: generation
            }) if generation == request_generation
        ));
    }
}
