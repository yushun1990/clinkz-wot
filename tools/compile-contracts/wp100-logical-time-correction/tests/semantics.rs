use core::cmp::Ordering;
use core::num::NonZeroU64;

use clinkz_wot_foundation::{ClockId, MonotonicInstant, RuntimeClock, SourceTimestamp};
use clinkz_wot_wp100_logical_time_correction_contract::{
    checked_source_order, extend_raw_tick,
};

fn scale(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).expect("test scale is nonzero")
}

#[test]
fn logical_instants_order_and_reject_other_clocks() {
    let id = ClockId::new(11);
    let before = MonotonicInstant::new(id, 40);
    let equal = MonotonicInstant::new(id, 40);
    let after = MonotonicInstant::new(id, 41);

    assert_eq!(before.checked_cmp(equal), Some(Ordering::Equal));
    assert_eq!(before.checked_cmp(after), Some(Ordering::Less));
    assert_eq!(after.checked_cmp(before), Some(Ordering::Greater));
    assert_eq!(
        after.checked_cmp(MonotonicInstant::new(ClockId::new(12), 41)),
        None
    );
}

#[test]
fn checked_addition_rejects_logical_exhaustion() {
    let maximum = MonotonicInstant::new(ClockId::new(13), u64::MAX);
    assert_eq!(maximum.checked_add_ticks(0), Some(maximum));
    assert_eq!(maximum.checked_add_ticks(1), None);
}

#[test]
fn source_timestamps_use_kind_id_and_scale() {
    let id = ClockId::new(17);
    let left = SourceTimestamp::Monotonic {
        clock_id: id,
        ticks: 100,
        ticks_per_second: scale(1_000),
    };
    let later = SourceTimestamp::Monotonic {
        clock_id: id,
        ticks: 101,
        ticks_per_second: scale(1_000),
    };
    let other_id = SourceTimestamp::Monotonic {
        clock_id: ClockId::new(18),
        ticks: 101,
        ticks_per_second: scale(1_000),
    };
    let other_scale = SourceTimestamp::Monotonic {
        clock_id: id,
        ticks: 101,
        ticks_per_second: scale(2_000),
    };

    assert_eq!(checked_source_order(left, later), Some(Ordering::Less));
    assert_eq!(checked_source_order(left, other_id), None);
    assert_eq!(checked_source_order(left, other_scale), None);
    assert_eq!(
        checked_source_order(SourceTimestamp::UnixMillis(-1), SourceTimestamp::UnixMillis(2)),
        Some(Ordering::Less)
    );
    assert_eq!(
        checked_source_order(SourceTimestamp::Unknown, SourceTimestamp::Unknown),
        None
    );
    assert_eq!(
        checked_source_order(left, SourceTimestamp::UnixMillis(100)),
        None
    );
}

#[test]
fn external_epoch_extends_one_and_multiple_raw_wraps() {
    let id = ClockId::new(19);
    let period = scale(256);
    let before_wrap = extend_raw_tick(id, period, 0, 250).expect("sample fits");
    let after_one_wrap = extend_raw_tick(id, period, 1, 3).expect("sample fits");
    let after_three_wraps = extend_raw_tick(id, period, 3, 3).expect("sample fits");

    assert_eq!(before_wrap.ticks(), 250);
    assert_eq!(after_one_wrap.ticks(), 259);
    assert_eq!(after_three_wraps.ticks(), 771);
    assert_eq!(
        after_one_wrap.checked_duration_since(before_wrap),
        Some(9)
    );
    assert_eq!(
        after_three_wraps.checked_duration_since(after_one_wrap),
        Some(512)
    );
}

#[test]
fn reset_or_lost_epoch_uses_an_incomparable_clock_id() {
    let old = extend_raw_tick(ClockId::new(23), scale(256), 7, 200).expect("sample fits");
    let replacement =
        extend_raw_tick(ClockId::new(24), scale(256), 0, 2).expect("sample fits");

    assert_eq!(old.checked_cmp(replacement), None);
    assert_eq!(replacement.checked_duration_since(old), None);
}

struct DiagnosticRawPeriodClock {
    now: MonotonicInstant,
}

impl RuntimeClock for DiagnosticRawPeriodClock {
    fn now(&self) -> MonotonicInstant {
        self.now
    }

    fn ticks_per_second(&self) -> NonZeroU64 {
        scale(1_000)
    }

    fn wrap_period_ticks(&self) -> Option<NonZeroU64> {
        Some(scale(256))
    }
}

#[test]
fn raw_period_is_diagnostic_not_the_logical_domain() {
    let clock = DiagnosticRawPeriodClock {
        now: MonotonicInstant::new(ClockId::new(29), 771),
    };

    assert_eq!(clock.now().ticks(), 771);
    assert_eq!(clock.wrap_period_ticks().map(NonZeroU64::get), Some(256));
}
