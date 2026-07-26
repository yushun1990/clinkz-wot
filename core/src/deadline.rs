//! Protocol-neutral logical deadlines.

use core::cmp::Ordering;

use clinkz_wot_foundation::MonotonicInstant;

/// Optional terminal instant in one extended logical clock domain.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Deadline {
    instant: Option<MonotonicInstant>,
}

impl Deadline {
    /// A disabled convenience deadline.
    pub const NONE: Self = Self { instant: None };

    /// Creates a finite deadline at an extended logical instant.
    pub const fn at(instant: MonotonicInstant) -> Self {
        Self {
            instant: Some(instant),
        }
    }

    /// Returns the terminal instant when this deadline is finite.
    pub const fn instant(self) -> Option<MonotonicInstant> {
        self.instant
    }

    /// Checks whether this deadline has elapsed in `now`'s clock domain.
    ///
    /// A disabled deadline never elapses. Different clock identities are
    /// incomparable and return `None`; callers must apply the disposition for
    /// their admission or post-admission boundary.
    pub fn checked_is_elapsed_at(self, now: MonotonicInstant) -> Option<bool> {
        match self.instant {
            None => Some(false),
            Some(instant) => {
                let ordering = now.checked_cmp(instant)?;
                Some(matches!(ordering, Ordering::Equal | Ordering::Greater))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use clinkz_wot_foundation::{ClockId, MonotonicInstant};

    use super::Deadline;

    const fn instant(clock: u64, ticks: u64) -> MonotonicInstant {
        MonotonicInstant::new(ClockId::new(clock), ticks)
    }

    #[test]
    fn none_is_default_and_never_elapses() {
        assert_eq!(Deadline::default(), Deadline::NONE);
        assert_eq!(
            Deadline::NONE.checked_is_elapsed_at(instant(99, u64::MAX)),
            Some(false)
        );
    }

    #[test]
    fn finite_deadlines_use_checked_logical_ordering() {
        let deadline = Deadline::at(instant(7, 250));

        assert_eq!(deadline.instant(), Some(instant(7, 250)));
        assert_eq!(deadline.checked_is_elapsed_at(instant(7, 249)), Some(false));
        assert_eq!(deadline.checked_is_elapsed_at(instant(7, 250)), Some(true));
        assert_eq!(deadline.checked_is_elapsed_at(instant(7, 771)), Some(true));
        assert_eq!(deadline.checked_is_elapsed_at(instant(8, 250)), None);
    }
}
