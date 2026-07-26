//! Monotonic clock identities and source timestamps.

use core::cmp::Ordering;
use core::fmt;
use core::num::NonZeroU64;

/// Opaque identity of one monotonic clock and tick scale.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ClockId(u64);

impl ClockId {
    /// Creates a clock identity from an application-assigned value.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the application-assigned value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for ClockId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// An extended logical tick value that is comparable only within one [`ClockId`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MonotonicInstant {
    clock_id: ClockId,
    ticks: u64,
}

impl MonotonicInstant {
    /// Creates a clock-qualified instant.
    pub const fn new(clock_id: ClockId, ticks: u64) -> Self {
        Self { clock_id, ticks }
    }

    /// Returns the clock identity.
    pub const fn clock_id(self) -> ClockId {
        self.clock_id
    }

    /// Returns the extended logical tick value.
    pub const fn ticks(self) -> u64 {
        self.ticks
    }

    /// Compares two non-wrapping instants when their clock identities match.
    pub fn checked_cmp(self, other: Self) -> Option<Ordering> {
        (self.clock_id == other.clock_id).then(|| self.ticks.cmp(&other.ticks))
    }

    /// Returns elapsed ticks when `earlier` uses the same clock and is not later.
    pub const fn checked_duration_since(self, earlier: Self) -> Option<u64> {
        if self.clock_id.get() != earlier.clock_id.get() {
            return None;
        }
        self.ticks.checked_sub(earlier.ticks)
    }

    /// Adds ticks without wrapping.
    pub const fn checked_add_ticks(self, ticks: u64) -> Option<Self> {
        match self.ticks.checked_add(ticks) {
            Some(ticks) => Some(Self::new(self.clock_id, ticks)),
            None => None,
        }
    }

    /// Adds a nanosecond duration, rounding toward earlier expiry.
    pub fn checked_add_nanos_earlier(
        self,
        nanos: u64,
        ticks_per_second: NonZeroU64,
    ) -> Option<Self> {
        let ticks =
            u128::from(nanos).checked_mul(u128::from(ticks_per_second.get()))? / 1_000_000_000u128;
        self.checked_add_ticks(u64::try_from(ticks).ok()?)
    }

    /// Reports elapsed nanoseconds, rounding toward zero.
    pub fn checked_nanos_since(self, earlier: Self, ticks_per_second: NonZeroU64) -> Option<u64> {
        let ticks = u128::from(self.checked_duration_since(earlier)?);
        let nanos = ticks.checked_mul(1_000_000_000u128)? / u128::from(ticks_per_second.get());
        u64::try_from(nanos).ok()
    }
}

/// Supplies non-wrapping logical monotonic time without imposing a host clock
/// or executor.
pub trait RuntimeClock {
    /// Returns the current instant.
    fn now(&self) -> MonotonicInstant;

    /// Returns the immutable number of ticks per second for this clock id.
    fn ticks_per_second(&self) -> NonZeroU64;

    /// Returns diagnostic metadata for the finite period of the underlying raw source.
    ///
    /// This does not select a comparison algorithm, describe the exposed
    /// logical domain, or bound admitted operation lifetimes.
    /// `None` means that no raw-source period is reported.
    fn wrap_period_ticks(&self) -> Option<NonZeroU64> {
        None
    }
}

/// Time metadata retained from a TD, Directory, or resolver source.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SourceTimestamp {
    /// A monotonic timestamp with its immutable tick scale.
    Monotonic {
        /// Identity of the source clock.
        clock_id: ClockId,
        /// Extended logical source clock ticks.
        ticks: u64,
        /// Immutable ticks per second for `clock_id`.
        ticks_per_second: NonZeroU64,
    },
    /// Milliseconds since the Unix epoch from a host source.
    UnixMillis(i64),
    /// The source provided no comparable timestamp.
    Unknown,
}

impl SourceTimestamp {
    /// Compares two timestamps when their kinds and clock domains match.
    ///
    /// Monotonic values require equal clock identities and tick scales. Unix
    /// millisecond values compare directly. Unknown values, mixed kinds, and
    /// conflicting monotonic domains are incomparable.
    pub fn checked_cmp(self, other: Self) -> Option<Ordering> {
        match (self, other) {
            (
                Self::Monotonic {
                    clock_id: left_id,
                    ticks: left_ticks,
                    ticks_per_second: left_scale,
                },
                Self::Monotonic {
                    clock_id: right_id,
                    ticks: right_ticks,
                    ticks_per_second: right_scale,
                },
            ) if left_id == right_id && left_scale == right_scale => {
                Some(left_ticks.cmp(&right_ticks))
            }
            (Self::UnixMillis(left), Self::UnixMillis(right)) => Some(left.cmp(&right)),
            _ => None,
        }
    }

    /// Returns the timestamp as a qualified monotonic instant when applicable.
    pub const fn monotonic_instant(self) -> Option<MonotonicInstant> {
        match self {
            Self::Monotonic {
                clock_id, ticks, ..
            } => Some(MonotonicInstant::new(clock_id, ticks)),
            Self::UnixMillis(_) | Self::Unknown => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use core::cmp::Ordering;
    use core::num::NonZeroU64;

    use super::{ClockId, MonotonicInstant, RuntimeClock, SourceTimestamp};

    struct DiagnosticRawPeriodClock;

    impl RuntimeClock for DiagnosticRawPeriodClock {
        fn now(&self) -> MonotonicInstant {
            MonotonicInstant::new(ClockId::new(7), 771)
        }

        fn ticks_per_second(&self) -> NonZeroU64 {
            NonZeroU64::new(1_000).expect("the scale is nonzero")
        }

        fn wrap_period_ticks(&self) -> Option<NonZeroU64> {
            NonZeroU64::new(256)
        }
    }

    #[test]
    fn different_clocks_are_incomparable() {
        let left = MonotonicInstant::new(ClockId::new(1), 10);
        let right = MonotonicInstant::new(ClockId::new(2), 10);
        assert_eq!(left.checked_cmp(right), None);
        assert_eq!(left.checked_duration_since(right), None);
    }

    #[test]
    fn logical_tick_addition_fails_before_exhaustion() {
        let maximum = MonotonicInstant::new(ClockId::new(3), u64::MAX);
        assert_eq!(maximum.checked_add_ticks(0), Some(maximum));
        assert_eq!(maximum.checked_add_ticks(1), None);
    }

    #[test]
    fn duration_conversion_uses_documented_rounding() {
        let scale = NonZeroU64::new(3).expect("three is nonzero");
        let start = MonotonicInstant::new(ClockId::new(1), 10);
        let deadline = start
            .checked_add_nanos_earlier(666_666_667, scale)
            .expect("the deadline fits");
        assert_eq!(deadline.ticks(), 12);
        assert_eq!(deadline.checked_cmp(start), Some(Ordering::Greater));
        assert_eq!(
            deadline.checked_nanos_since(start, scale),
            Some(666_666_666)
        );
    }

    #[test]
    fn source_timestamps_compare_only_in_one_kind_and_domain() {
        let scale = NonZeroU64::new(1_000).expect("the scale is nonzero");
        let clock_id = ClockId::new(9);
        let before = SourceTimestamp::Monotonic {
            clock_id,
            ticks: 41,
            ticks_per_second: scale,
        };
        let equal = SourceTimestamp::Monotonic {
            clock_id,
            ticks: 41,
            ticks_per_second: scale,
        };
        let after = SourceTimestamp::Monotonic {
            clock_id,
            ticks: 42,
            ticks_per_second: scale,
        };
        let other_clock = SourceTimestamp::Monotonic {
            clock_id: ClockId::new(10),
            ticks: 42,
            ticks_per_second: scale,
        };
        let other_scale = SourceTimestamp::Monotonic {
            clock_id,
            ticks: 42,
            ticks_per_second: NonZeroU64::new(2_000).expect("the scale is nonzero"),
        };

        assert_eq!(before.checked_cmp(equal), Some(Ordering::Equal));
        assert_eq!(before.checked_cmp(after), Some(Ordering::Less));
        assert_eq!(after.checked_cmp(before), Some(Ordering::Greater));
        assert_eq!(before.checked_cmp(other_clock), None);
        assert_eq!(before.checked_cmp(other_scale), None);
        assert_eq!(
            SourceTimestamp::UnixMillis(-1).checked_cmp(SourceTimestamp::UnixMillis(-1)),
            Some(Ordering::Equal)
        );
        assert_eq!(
            SourceTimestamp::UnixMillis(-1).checked_cmp(SourceTimestamp::UnixMillis(2)),
            Some(Ordering::Less)
        );
        assert_eq!(
            SourceTimestamp::UnixMillis(2).checked_cmp(SourceTimestamp::UnixMillis(-1)),
            Some(Ordering::Greater)
        );
        assert_eq!(
            SourceTimestamp::Unknown.checked_cmp(SourceTimestamp::Unknown),
            None
        );
        assert_eq!(before.checked_cmp(SourceTimestamp::UnixMillis(41)), None);
        assert_eq!(SourceTimestamp::UnixMillis(41).checked_cmp(before), None);
        assert_eq!(
            after.monotonic_instant(),
            Some(MonotonicInstant::new(clock_id, 42))
        );
    }

    #[test]
    fn source_timestamp_preserves_clock_identity() {
        let scale = NonZeroU64::new(1_000).expect("the scale is nonzero");
        let timestamp = SourceTimestamp::Monotonic {
            clock_id: ClockId::new(9),
            ticks: 42,
            ticks_per_second: scale,
        };
        assert_eq!(
            timestamp.monotonic_instant(),
            Some(MonotonicInstant::new(ClockId::new(9), 42))
        );
    }

    #[test]
    fn raw_period_is_diagnostic_not_the_logical_domain() {
        let clock = DiagnosticRawPeriodClock;
        assert_eq!(clock.now().clock_id(), ClockId::new(7));
        assert_eq!(clock.now().ticks(), 771);
        assert_eq!(clock.ticks_per_second().get(), 1_000);
        assert_eq!(clock.wrap_period_ticks().map(NonZeroU64::get), Some(256));
    }
}
