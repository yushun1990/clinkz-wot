#![allow(dead_code)]

//! Non-production Stage-A proof of the application-static cancellation owner
//! topology proposed in workspace/0063.
//!
//! The real static root is driven through `&mut self`. Its live admission is
//! therefore stored inside that root and is cancelled directly between
//! bounded progress calls. No `Cell`, shared cancellation view, self-reference,
//! or invented destroy generation is needed.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StaticLifecycle {
    Active,
    DestroyRequested,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BuildingPhase {
    Compiling,
    ReadyToFreeze,
    FailedSettled,
    Frozen,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalCause {
    CancelledByDestroy,
    SemanticFailure,
}

struct StaticAdmission {
    phase: BuildingPhase,
    first_cause: Option<TerminalCause>,
    callback_calls: u32,
    cursor_live: bool,
    cursor_abort_calls: u32,
    reservations_live: bool,
}

impl StaticAdmission {
    fn new() -> Self {
        Self {
            phase: BuildingPhase::Compiling,
            first_cause: None,
            callback_calls: 0,
            cursor_live: true,
            cursor_abort_calls: 0,
            reservations_live: true,
        }
    }

    fn record_first_cause(&mut self, cause: TerminalCause) {
        if self.first_cause.is_none() {
            self.first_cause = Some(cause);
        }
    }

    fn settle_if_cancelled(&mut self, lifecycle: StaticLifecycle) -> bool {
        if lifecycle == StaticLifecycle::DestroyRequested {
            self.record_first_cause(TerminalCause::CancelledByDestroy);
        }
        if self.first_cause.is_none() {
            return false;
        }
        if self.cursor_live {
            self.cursor_live = false;
            self.cursor_abort_calls += 1;
        }
        self.reservations_live = false;
        self.phase = BuildingPhase::FailedSettled;
        true
    }
}

struct StaticServientRoot {
    lifecycle: StaticLifecycle,
    admission: Option<StaticAdmission>,
}

impl StaticServientRoot {
    fn new() -> Self {
        Self {
            lifecycle: StaticLifecycle::Active,
            admission: None,
        }
    }

    fn begin_admission(&mut self) -> Result<(), TerminalCause> {
        if self.lifecycle == StaticLifecycle::DestroyRequested {
            return Err(TerminalCause::CancelledByDestroy);
        }
        assert!(self.admission.is_none());
        self.admission = Some(StaticAdmission::new());
        Ok(())
    }

    /// Same ownership shape as the current application-static destruction
    /// entry: the root is exclusively borrowed and mutates its child
    /// transaction directly.
    fn begin_destroy(&mut self) {
        self.lifecycle = StaticLifecycle::DestroyRequested;
        if let Some(admission) = self.admission.as_mut() {
            admission.record_first_cause(TerminalCause::CancelledByDestroy);
        }
    }

    /// One caller-driven progress step checks the root-owned cancellation
    /// state before making a semantic callback.
    fn step(&mut self, semantic_failure: bool) {
        let lifecycle = self.lifecycle;
        let admission = self.admission.as_mut().expect("live admission");
        if admission.settle_if_cancelled(lifecycle) {
            return;
        }
        assert_eq!(admission.phase, BuildingPhase::Compiling);
        admission.callback_calls += 1;
        if semantic_failure {
            admission.record_first_cause(TerminalCause::SemanticFailure);
            admission.settle_if_cancelled(lifecycle);
        }
    }

    fn mark_ready_to_freeze(&mut self) {
        let admission = self.admission.as_mut().expect("live admission");
        assert_eq!(admission.phase, BuildingPhase::Compiling);
        admission.cursor_live = false;
        admission.phase = BuildingPhase::ReadyToFreeze;
    }

    /// Cancellation is checked again at the linearization point immediately
    /// before unpublished ownership could become Frozen.
    fn try_freeze(&mut self) -> Result<(), TerminalCause> {
        let lifecycle = self.lifecycle;
        let admission = self.admission.as_mut().expect("live admission");
        if admission.settle_if_cancelled(lifecycle) {
            return Err(admission.first_cause.expect("settled first cause"));
        }
        assert_eq!(admission.phase, BuildingPhase::ReadyToFreeze);
        admission.phase = BuildingPhase::Frozen;
        Ok(())
    }

    fn admission(&self) -> &StaticAdmission {
        self.admission.as_ref().expect("fixture admission")
    }
}

#[test]
fn destroy_between_steps_prevents_the_next_callback_and_settles_building() {
    let mut root = StaticServientRoot::new();
    root.begin_admission().expect("active root admits work");
    root.step(false);
    assert_eq!(root.admission().callback_calls, 1);

    root.begin_destroy();
    root.step(true);

    let admission = root.admission();
    assert_eq!(admission.callback_calls, 1);
    assert_eq!(
        admission.first_cause,
        Some(TerminalCause::CancelledByDestroy)
    );
    assert_eq!(admission.cursor_abort_calls, 1);
    assert!(!admission.cursor_live);
    assert!(!admission.reservations_live);
    assert_eq!(admission.phase, BuildingPhase::FailedSettled);
}

#[test]
fn first_semantic_cause_survives_a_later_destroy_request() {
    let mut root = StaticServientRoot::new();
    root.begin_admission().expect("active root admits work");
    root.step(true);
    assert_eq!(
        root.admission().first_cause,
        Some(TerminalCause::SemanticFailure)
    );

    root.begin_destroy();
    root.step(false);

    let admission = root.admission();
    assert_eq!(admission.first_cause, Some(TerminalCause::SemanticFailure));
    assert_eq!(admission.callback_calls, 1);
    assert_eq!(admission.cursor_abort_calls, 1);
}

#[test]
fn destroy_is_rechecked_at_the_pre_frozen_linearization_point() {
    let mut root = StaticServientRoot::new();
    root.begin_admission().expect("active root admits work");
    root.mark_ready_to_freeze();
    root.begin_destroy();

    assert_eq!(root.try_freeze(), Err(TerminalCause::CancelledByDestroy));
    assert_eq!(root.admission().phase, BuildingPhase::FailedSettled);
    assert!(!root.admission().reservations_live);
}

#[test]
fn admission_cannot_start_after_exclusive_destroy_request() {
    let mut root = StaticServientRoot::new();
    root.begin_destroy();
    assert_eq!(
        root.begin_admission(),
        Err(TerminalCause::CancelledByDestroy)
    );
    assert!(root.admission.is_none());
}

#[test]
fn destroy_of_an_unpublished_frozen_aggregate_releases_its_persistent_owner() {
    let mut root = StaticServientRoot::new();
    root.begin_admission().expect("active root admits work");
    root.mark_ready_to_freeze();
    root.try_freeze()
        .expect("fixture reaches unpublished Frozen");
    assert_eq!(root.admission().phase, BuildingPhase::Frozen);
    assert!(root.admission().reservations_live);

    root.begin_destroy();
    root.step(false);

    let admission = root.admission();
    assert_eq!(
        admission.first_cause,
        Some(TerminalCause::CancelledByDestroy)
    );
    assert_eq!(admission.phase, BuildingPhase::FailedSettled);
    assert!(!admission.reservations_live);
    assert_eq!(admission.cursor_abort_calls, 0);
}
