#![no_std]

use core::cmp::Ordering;
use core::num::NonZeroU64;

use clinkz_wot_foundation::{ClockId, MonotonicInstant, SourceTimestamp};

/// Uses the real Foundation comparison surface.
pub fn checked_source_order(
    left: SourceTimestamp,
    right: SourceTimestamp,
) -> Option<Ordering> {
    left.checked_cmp(right)
}

/// Extends a finite raw sample with a clock-source-owned overflow epoch.
///
/// The epoch must come from a reliable hardware or adapter-owned source; this
/// function does not infer it from successive raw samples.
pub fn extend_raw_tick(
    clock_id: ClockId,
    raw_period: NonZeroU64,
    overflow_epoch: u64,
    raw_tick: u64,
) -> Option<MonotonicInstant> {
    if raw_tick >= raw_period.get() {
        return None;
    }
    let epoch_ticks = overflow_epoch.checked_mul(raw_period.get())?;
    Some(MonotonicInstant::new(
        clock_id,
        epoch_ticks.checked_add(raw_tick)?,
    ))
}
