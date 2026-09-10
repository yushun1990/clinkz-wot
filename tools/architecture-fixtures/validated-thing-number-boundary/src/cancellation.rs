//! Non-production falsification of value-preserving decimal reduction.
//! This restricted witness recognizes positive powers of ten, not all Numbers.
//! It neither substitutes Basic rules nor claims a general projection algorithm.
use super::tests::{lexical, thing_with_maximum};
use clinkz_wot_foundation::{WorkBudget, WorkClass};
use clinkz_wot_td::validate::{Validate, ValidateError, ValidationLevel};
use serde_json::Number;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Pending,
    UnitValue,
    OutsideWitness,
    Limit,
    Cancelled,
}

// No owned input, heap, large scratch array, or recursive destructor. Counts
// and arithmetic are checked; OutsideWitness is NOT TD Invalid. Input syntax
// is certified by ordinary public Number parsing before this restricted fold.
#[derive(Debug)]
struct UnitFold<'a> {
    input: &'a [u8],
    position: usize,
    lifetime: u64,
    outcome: Outcome,
    fraction: bool,
    scientific: bool,
    negative_exponent: bool,
    one_seen: bool,
    trailing: i64,
    fractional: i64,
    exponent: i64,
}

impl<'a> UnitFold<'a> {
    fn new(input: &'a str, lifetime: u64) -> Self {
        Self {
            input: input.as_bytes(),
            position: 0,
            lifetime,
            outcome: Outcome::Pending,
            fraction: false,
            scientific: false,
            negative_exponent: false,
            one_seen: false,
            trailing: 0,
            fractional: 0,
            exponent: 0,
        }
    }

    fn byte(&mut self, byte: u8) -> Option<()> {
        match byte {
            b'.' if !self.scientific => self.fraction = true,
            b'e' | b'E' => self.scientific = true,
            b'-' if self.scientific => self.negative_exponent = true,
            b'+' if self.scientific => (),
            b'0'..=b'9' => {
                let digit = i64::from(byte - b'0');
                if self.scientific {
                    self.exponent = self.exponent.checked_mul(10)?.checked_add(digit)?;
                } else {
                    if self.fraction {
                        self.fractional = self.fractional.checked_add(1)?;
                    }
                    if self.one_seen {
                        if digit != 0 {
                            return None;
                        }
                        self.trailing = self.trailing.checked_add(1)?;
                    } else if digit != 0 {
                        if digit != 1 {
                            return None;
                        }
                        self.one_seen = true;
                    }
                }
            }
            _ => return None,
        }
        Some(())
    }

    fn step(&mut self, budget: &mut WorkBudget, cancel: bool) -> Outcome {
        if self.outcome != Outcome::Pending {
            return self.outcome;
        }
        if cancel {
            self.outcome = Outcome::Cancelled;
            return self.outcome;
        }
        while self.position < self.input.len() {
            // Check BOTH allowances before either debit or input observation.
            if budget.remaining(WorkClass::CodecInputBytes) == 0 {
                break;
            }
            if self.lifetime == 0 {
                self.outcome = Outcome::Limit;
                return self.outcome;
            }
            budget.consume(WorkClass::CodecInputBytes, 1).unwrap();
            self.lifetime -= 1;
            let byte = self.input[self.position];
            self.position += 1;
            if self.byte(byte).is_none() {
                self.outcome = Outcome::OutsideWitness;
                return self.outcome;
            }
            // Final fixed scalar arithmetic belongs to this last byte's work.
            if self.position == self.input.len() {
                let exponent = if self.negative_exponent {
                    -self.exponent
                } else {
                    self.exponent
                };
                let power = self
                    .trailing
                    .checked_sub(self.fractional)
                    .and_then(|value| value.checked_add(exponent));
                self.outcome = if self.one_seen && power == Some(0) {
                    Outcome::UnitValue
                } else {
                    Outcome::OutsideWitness
                };
            }
        }
        self.outcome
    }
}

// A test-only inert snapshot cannot execute work or reconstruct a cursor.
// Pointer/length checks avoid rescanning borrowed content in test assertions.
type Checkpoint = (usize, u64, Outcome, [bool; 4], [i64; 3], (*const u8, usize));
fn checkpoint(cursor: &UnitFold<'_>) -> Checkpoint {
    (
        cursor.position,
        cursor.lifetime,
        cursor.outcome,
        [
            cursor.fraction,
            cursor.scientific,
            cursor.negative_exponent,
            cursor.one_seen,
        ],
        [cursor.trailing, cursor.fractional, cursor.exponent],
        (cursor.input.as_ptr(), cursor.input.len()),
    )
}

fn budget(units: u64) -> WorkBudget {
    WorkBudget::new().with_remaining(WorkClass::CodecInputBytes, units)
}

fn fold(number: &Number, allowance: u64) -> usize {
    let mut cursor = UnitFold::new(number.as_str(), number.as_str().len() as u64);
    let mut steps = 0;
    loop {
        let before = checkpoint(&cursor);
        assert_eq!(cursor.step(&mut budget(0), false), Outcome::Pending);
        assert_eq!(checkpoint(&cursor), before);
        let outcome = cursor.step(&mut budget(allowance), false);
        steps += 1;
        assert!(cursor.position > before.0);
        assert!(cursor.position - before.0 <= allowance as usize);
        if outcome != Outcome::Pending {
            assert_eq!(outcome, Outcome::UnitValue);
            assert_eq!(cursor.lifetime, 0);
            assert_eq!(cursor.position, number.as_str().len());
            return steps;
        }
    }
}

fn basic(number: Number) -> bool {
    match thing_with_maximum(number, Number::from_f64(0.5).unwrap())
        .validate_with_level(ValidationLevel::Basic)
    {
        Ok(()) => true,
        Err(ValidateError::InvalidSchema(_)) => false,
        other => panic!("unexpected Basic result: {other:?}"),
    }
}

#[test]
fn exact_unit_reduction_changes_actual_number_and_basic_semantics() {
    // 655359 is a control: the lexical exponent has not yet lost a digit.
    // At 655360, the current lexical parser stops accumulating at 65536.
    for n in [655359, 655360] {
        for fractional in [false, true] {
            let input = if fractional {
                format!("0.{}1e+{n}", "0".repeat(n - 1))
            } else {
                format!("1{}e-{n}", "0".repeat(n))
            };
            let number: Number = serde_json::from_str(&input).unwrap();
            assert_eq!(number.as_str(), input);
            // Independent mathematical identity: the input is respectively
            // 10^n * 10^-n or 10^-n * 10^n. No float rounding is needed.
            let projected = Number::from(1);
            assert_eq!(projected.as_f64(), Some(1.0));
            assert!(!basic(projected));
            let actual = number.as_f64();
            let expected = if n == 655359 {
                Some(1.0)
            } else if fractional {
                Some(0.0)
            } else {
                None
            };
            assert_eq!(actual.map(f64::to_bits), expected.map(f64::to_bits));
            assert_eq!(
                input.parse::<f64>().unwrap().to_bits(),
                expected.unwrap_or(f64::INFINITY).to_bits()
            );
            let (decimal, trace) = lexical(&number);
            assert_eq!(trace.furthest, input.len());
            for allowance in [1, 7, 128] {
                let steps = fold(&number, allowance);
                assert_eq!(steps, input.len().div_ceil(allowance as usize));
                println!(
                    "unit-fold n={n} fractional={fractional} bytes={} allowance={allowance} steps={steps} byte_charges={} lifetime=0 actual={actual:?} exact=Some(1.0) lexical_exponent={} lexical_reads={}",
                    input.len(),
                    input.len(),
                    decimal.exponent,
                    trace.reads
                );
            }
            let accepted = basic(number);
            assert_eq!(accepted, n == 655360);
            println!(
                "unit-fold n={n} fractional={fractional} actual_basic={accepted} exact_basic=false"
            );
        }
    }
}

#[test]
fn restricted_fold_lifetime_and_cancellation_are_terminal_without_storage() {
    let input: Number = serde_json::from_str("100000e-5").unwrap();
    let mut cursor = UnitFold::new(input.as_str(), 3);
    for position in 1..=3 {
        assert_eq!(cursor.step(&mut budget(1), false), Outcome::Pending);
        assert_eq!(cursor.position, position);
    }
    assert_eq!(cursor.step(&mut budget(1), false), Outcome::Limit);
    let terminal = checkpoint(&cursor);
    assert_eq!(cursor.step(&mut budget(100), true), Outcome::Limit);
    assert_eq!(checkpoint(&cursor), terminal);
    for position in 0..input.as_str().len() {
        let mut cursor = UnitFold::new(input.as_str(), 100);
        cursor.step(&mut budget(position as u64), false);
        assert_eq!(cursor.step(&mut budget(0), true), Outcome::Cancelled);
        let terminal = checkpoint(&cursor);
        assert_eq!(cursor.step(&mut budget(100), false), Outcome::Cancelled);
        assert_eq!(checkpoint(&cursor), terminal);
    }
    assert!(!std::mem::needs_drop::<UnitFold<'_>>());
    println!(
        "restricted fold inline bytes={} allocations=0; no full cursor/arena resource claim",
        std::mem::size_of::<UnitFold<'_>>()
    );
}
