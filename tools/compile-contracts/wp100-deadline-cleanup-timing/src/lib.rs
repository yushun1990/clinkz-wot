#![no_std]

use clinkz_wot_core::{CoreError, Deadline, ErrorContext, ErrorPhase, RetryClass};
use clinkz_wot_foundation::MonotonicInstant;

/// Post-admission owner of a deadline comparison.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeadlineOwner {
    Handler,
    Binding,
    Cleanup,
}

/// Deterministic outcome at the timeout cancellation linearization point.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeoutRaceOutcome {
    Pending,
    Success,
    TimedOut,
}

/// Applies the caller-supplied admission disposition from the frozen table.
pub fn check_admission_deadline(
    deadline: Deadline,
    now: MonotonicInstant,
) -> Result<bool, CoreError> {
    deadline.checked_is_elapsed_at(now).ok_or_else(|| {
        CoreError::Validation(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
    })
}

/// Applies the post-admission incomparable-clock disposition for an owner.
pub fn check_admitted_deadline(
    owner: DeadlineOwner,
    deadline: Deadline,
    now: MonotonicInstant,
) -> Result<bool, CoreError> {
    deadline.checked_is_elapsed_at(now).ok_or_else(|| {
        let phase = match owner {
            DeadlineOwner::Handler => ErrorPhase::Handler,
            DeadlineOwner::Binding => ErrorPhase::Binding,
            DeadlineOwner::Cleanup => ErrorPhase::Cleanup,
        };
        CoreError::InternalInvariant(ErrorContext::new(phase, RetryClass::Never))
    })
}

/// Models the existing timeout linearization contract without owning runtime
/// scheduling or cancellation.
pub fn resolve_timeout_race(success_published: bool, deadline_elapsed: bool) -> TimeoutRaceOutcome {
    if success_published {
        TimeoutRaceOutcome::Success
    } else if deadline_elapsed {
        TimeoutRaceOutcome::TimedOut
    } else {
        TimeoutRaceOutcome::Pending
    }
}
