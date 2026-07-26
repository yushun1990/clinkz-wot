use clinkz_wot_core::Deadline;
use clinkz_wot_foundation::{ClockId, MonotonicInstant};

fn main() {
    let _ = Deadline {
        instant: Some(MonotonicInstant::new(ClockId::new(1), 2)),
    };
}
