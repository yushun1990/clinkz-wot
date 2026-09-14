#![no_std]

use core::fmt::{self, Write};
use serde_json::Number;

pub const LOCAL_VALIDATED_THING: bool = cfg!(feature = "validated-thing");

/// Synchronous source adapter. No latency or callback-partition assumption.
/// The sink retains no callback borrow and allocates no output. Re-driving
/// Display for a comparison pass is synchronous work too.
pub fn synchronous_decimal(number: &Number, mut byte: impl FnMut(u8)) -> fmt::Result {
    struct Sink<F>(F);
    impl<F: FnMut(u8)> Write for Sink<F> {
        fn write_str(&mut self, text: &str) -> fmt::Result {
            for byte in text.bytes() {
                (self.0)(byte);
            }
            Ok(())
        }
    }
    write!(Sink(&mut byte), "{number}")
}

// Dependency AP alone cannot expose this module. This is not a mock of the
// full frozen ValidatedThing public API.
#[cfg(feature = "validated-thing")]
pub mod bounded {
    use clinkz_wot_foundation::{WorkBudget, WorkClass};
    use serde_json::Number;

    pub fn decimal(number: &Number) -> &str {
        number.as_str()
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Progress {
        Pending,
        Complete,
        Limit,
        Cancelled,
    }

    /// Source-access witness, not a comparator or admission cursor.
    pub struct Scan<'a> {
        bytes: &'a [u8],
        pub position: usize,
        pub lifetime: u64,
        outcome: Progress,
    }

    impl<'a> Scan<'a> {
        pub fn new(number: &'a Number, lifetime: u64) -> Self {
            Self {
                bytes: decimal(number).as_bytes(),
                position: 0,
                lifetime,
                outcome: Progress::Pending,
            }
        }

        pub fn step(
            &mut self,
            budget: &mut WorkBudget,
            cancel: bool,
            mut byte: impl FnMut(u8),
        ) -> Progress {
            if self.outcome != Progress::Pending {
                return self.outcome;
            }
            if cancel {
                self.outcome = Progress::Cancelled;
                return self.outcome;
            }
            while self.position < self.bytes.len() {
                if budget.remaining(WorkClass::CodecInputBytes) == 0 {
                    return Progress::Pending;
                }
                if self.lifetime == 0 {
                    self.outcome = Progress::Limit;
                    return self.outcome;
                }
                budget.consume(WorkClass::CodecInputBytes, 1).unwrap();
                self.lifetime -= 1;
                byte(self.bytes[self.position]);
                self.position += 1;
            }
            self.outcome = Progress::Complete;
            self.outcome
        }
    }
}
