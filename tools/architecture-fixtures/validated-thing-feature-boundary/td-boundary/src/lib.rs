#![no_std]

use serde_json::Value;

pub const LOCAL_VALIDATED_THING: bool = cfg!(feature = "validated-thing");

/// Selected synchronous Basic projection for the five extension predicates.
/// A JSON Number with no finite public projection is invalid, not absent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PredicateNumber {
    Absent,
    Finite(f64),
    Invalid,
}

pub fn predicate_number(value: Option<&Value>) -> PredicateNumber {
    let Some(number) = value.and_then(Value::as_number) else {
        return PredicateNumber::Absent;
    };
    match number.as_f64().filter(|value| value.is_finite()) {
        Some(value) => PredicateNumber::Finite(value),
        None => PredicateNumber::Invalid,
    }
}

// Dependency AP alone cannot expose this module. This is not a mock of the
// full frozen ValidatedThing public API.
#[cfg(feature = "validated-thing")]
pub mod bounded {
    use clinkz_wot_foundation::{WorkBudget, WorkClass};
    use serde_json::Number;

    use super::PredicateNumber;

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

    /// One precharged projection witness, not a production admission cursor.
    /// The caller must have already identified a Number used by a predicate.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum ProjectionProgress {
        Pending,
        Limit,
        Cancelled,
        InvalidConfiguration,
        Complete(PredicateNumber),
    }

    pub fn project(
        number: &Number,
        ceiling: usize,
        budget: &mut WorkBudget,
        lifetime: &mut u64,
        mut cancelled: impl FnMut() -> bool,
    ) -> ProjectionProgress {
        let length = decimal(number).len();
        if ceiling > 256 {
            return ProjectionProgress::InvalidConfiguration;
        }
        if length > ceiling {
            return ProjectionProgress::Limit;
        }
        let charge = length as u64;
        if budget.remaining(WorkClass::CodecInputBytes) < charge {
            return ProjectionProgress::Pending;
        }
        if *lifetime < charge {
            return ProjectionProgress::Limit;
        }
        if cancelled() {
            return ProjectionProgress::Cancelled;
        }
        budget.consume(WorkClass::CodecInputBytes, charge).unwrap();
        *lifetime -= charge;
        let result = match number.as_f64().filter(|value| value.is_finite()) {
            Some(value) => PredicateNumber::Finite(value),
            None => PredicateNumber::Invalid,
        };
        if cancelled() {
            ProjectionProgress::Cancelled
        } else {
            ProjectionProgress::Complete(result)
        }
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
