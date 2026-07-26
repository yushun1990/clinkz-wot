use clinkz_wot_core::{
    CleanupHandle, CleanupOperation, CleanupRecord, CoreError, Deadline, ErrorPhase, RetryClass,
};
use clinkz_wot_foundation::{ClockId, Generation, MonotonicInstant, SlotIndex};
use clinkz_wot_wp100_deadline_cleanup_timing_contract::{
    DeadlineOwner, TimeoutRaceOutcome, check_admission_deadline, check_admitted_deadline,
    resolve_timeout_race,
};

fn instant(clock: u64, ticks: u64) -> MonotonicInstant {
    MonotonicInstant::new(ClockId::new(clock), ticks)
}

fn slot(index: u32) -> clinkz_wot_core::CleanupSlotId {
    clinkz_wot_core::CleanupSlotId::new(SlotIndex::new(index), Generation::INITIAL)
}

fn cleanup_record() -> CleanupRecord {
    CleanupRecord::try_new(
        CleanupHandle::new(slot(1)),
        slot(2),
        slot(3),
        CleanupOperation::CancelProcess,
        0,
        RetryClass::Never,
        7,
        2,
    )
    .expect("the initial cleanup attempt fits")
}

#[test]
fn deadline_preserves_the_frozen_value_semantics() {
    fn assert_copy<T: Copy>() {}
    assert_copy::<Deadline>();

    let none = Deadline::default();
    assert_eq!(none, Deadline::NONE);
    assert_eq!(none.instant(), None);
    assert_eq!(
        none.checked_is_elapsed_at(instant(99, u64::MAX)),
        Some(false)
    );

    let deadline = Deadline::at(instant(11, 250));
    assert_eq!(deadline.instant(), Some(instant(11, 250)));
    assert_eq!(
        deadline.checked_is_elapsed_at(instant(11, 249)),
        Some(false)
    );
    assert_eq!(deadline.checked_is_elapsed_at(instant(11, 250)), Some(true));
    assert_eq!(deadline.checked_is_elapsed_at(instant(11, 251)), Some(true));
    assert_eq!(deadline.checked_is_elapsed_at(instant(11, 771)), Some(true));
    assert_eq!(deadline.checked_is_elapsed_at(instant(12, 251)), None);
}

#[test]
fn cleanup_timing_uses_one_checked_logical_domain() {
    let id = ClockId::new(21);
    let before = instant(id.get(), 40);
    let equal = instant(id.get(), 50);
    let deadline = instant(id.get(), 50);

    assert!(
        cleanup_record()
            .try_with_timing(id, Some(deadline), Some(before))
            .is_some()
    );
    assert!(
        cleanup_record()
            .try_with_timing(id, Some(deadline), Some(equal))
            .is_some()
    );
    assert!(
        cleanup_record()
            .try_with_timing(id, Some(deadline), Some(instant(id.get(), 51)))
            .is_none()
    );
    assert!(
        cleanup_record()
            .try_with_timing(id, Some(deadline), Some(instant(22, 41)))
            .is_none()
    );
    assert!(
        cleanup_record()
            .try_with_timing(ClockId::new(22), Some(deadline), None)
            .is_none()
    );
}

#[test]
fn incomparable_clock_disposition_is_exact() {
    let deadline = Deadline::at(instant(31, 100));
    let other_clock = instant(32, 100);

    let admission = check_admission_deadline(deadline, other_clock)
        .expect_err("caller mismatch must fail admission");
    assert!(matches!(admission, CoreError::Validation(_)));
    assert_eq!(admission.context().phase(), ErrorPhase::Admission);
    assert_eq!(admission.retry_class(), RetryClass::Never);

    for (owner, phase) in [
        (DeadlineOwner::Handler, ErrorPhase::Handler),
        (DeadlineOwner::Binding, ErrorPhase::Binding),
        (DeadlineOwner::Cleanup, ErrorPhase::Cleanup),
    ] {
        let error = check_admitted_deadline(owner, deadline, other_clock)
            .expect_err("post-admission mismatch must fail closed");
        assert!(matches!(error, CoreError::InternalInvariant(_)));
        assert_eq!(error.context().phase(), phase);
        assert_eq!(error.retry_class(), RetryClass::Never);
    }
}

#[test]
fn timeout_race_respects_the_publication_linearization_point() {
    assert_eq!(
        resolve_timeout_race(true, true),
        TimeoutRaceOutcome::Success
    );
    assert_eq!(
        resolve_timeout_race(false, true),
        TimeoutRaceOutcome::TimedOut
    );
    assert_eq!(
        resolve_timeout_race(false, false),
        TimeoutRaceOutcome::Pending
    );
}
