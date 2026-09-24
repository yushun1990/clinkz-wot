//! Non-production strict JSON Number lexical-limit witness for WP-100.
//! A caller charges each observed byte before feeding it to this scalar state.
#![no_std]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum State {
    Start,
    Minus,
    Zero,
    Integer,
    Dot,
    Fraction,
    Exponent,
    ExponentSign,
    ExponentDigits,
}

impl State {
    const fn may_end(self) -> bool {
        matches!(
            self,
            Self::Zero | Self::Integer | Self::Fraction | Self::ExponentDigits
        )
    }

    fn next(self, byte: u8) -> Option<Self> {
        match (self, byte) {
            (Self::Start, b'-') => Some(Self::Minus),
            (Self::Start | Self::Minus, b'0') => Some(Self::Zero),
            (Self::Start | Self::Minus, b'1'..=b'9') => Some(Self::Integer),
            (Self::Integer, b'0'..=b'9') => Some(Self::Integer),
            (Self::Zero | Self::Integer, b'.') => Some(Self::Dot),
            (Self::Dot | Self::Fraction, b'0'..=b'9') => Some(Self::Fraction),
            (Self::Zero | Self::Integer | Self::Fraction, b'e' | b'E') => Some(Self::Exponent),
            (Self::Exponent, b'+' | b'-') => Some(Self::ExponentSign),
            (Self::Exponent | Self::ExponentSign | Self::ExponentDigits, b'0'..=b'9') => {
                Some(Self::ExponentDigits)
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Feed {
    /// This Number byte passed the lexical limit; the caller may copy it.
    Consumed,
    /// A delimiter was observed but remains with the outer JSON parser.
    Complete,
    /// The first over-limit Number byte was observed, but not consumed/copied.
    Limit,
    Invalid,
}

/// Scalar Number-token state. `limit` stands for an already checked admission
/// configuration's finite `number_lexeme_bytes_max`; this fixture does not
/// implement that complete configuration or the outer JSON decoder.
pub struct NumberLexeme {
    state: State,
    consumed: usize,
    limit: usize,
    terminal: Option<Feed>,
}

impl NumberLexeme {
    pub const fn new(limit: usize) -> Self {
        Self {
            state: State::Start,
            consumed: 0,
            limit,
            terminal: None,
        }
    }

    pub const fn consumed(&self) -> usize {
        self.consumed
    }

    /// Feed one already charged byte. Only `Consumed` authorizes a byte copy.
    /// `Limit` is decided before an input-sized numeric query or token finish.
    pub fn feed(&mut self, byte: u8) -> Feed {
        if let Some(terminal) = self.terminal {
            return terminal;
        }
        if let Some(next) = self.state.next(byte) {
            if self.consumed == self.limit {
                self.terminal = Some(Feed::Limit);
                return Feed::Limit;
            }
            self.state = next;
            self.consumed += 1;
            Feed::Consumed
        } else {
            let outcome = if is_delimiter(byte) && self.state.may_end() {
                Feed::Complete
            } else {
                Feed::Invalid
            };
            self.terminal = Some(outcome);
            outcome
        }
    }

    /// A Number may end at input EOF only after a complete JSON Number grammar.
    pub fn finish_eof(&mut self) -> Feed {
        if let Some(terminal) = self.terminal {
            return terminal;
        }
        let outcome = if self.state.may_end() {
            Feed::Complete
        } else {
            Feed::Invalid
        };
        self.terminal = Some(outcome);
        outcome
    }
}

const fn is_delimiter(byte: u8) -> bool {
    matches!(byte, b',' | b']' | b'}' | b' ' | b'\t' | b'\n' | b'\r')
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use clinkz_wot_foundation::{
        BenchmarkStaticReferenceV1, GatewayDefaultV1, StaticResourceProfile, WorkBudget, WorkClass,
    };
    use std::{format, string::String};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Outcome {
        Pending,
        Limit,
        Invalid,
        Complete,
    }

    #[derive(Debug, Eq, PartialEq)]
    struct Trace {
        outcome: Outcome,
        inspected: usize,
        copied: usize,
        lifetime_remaining: u64,
    }

    // A minimal caller: current-step and shared lifetime charges precede each
    // byte observation. This driver does not purport to be a TD JSON decoder.
    fn run(input: &[u8], limit: usize, step_allowance: u64, lifetime: u64) -> Trace {
        let mut number = NumberLexeme::new(limit);
        let mut inspected = 0;
        let mut copied = 0;
        let mut lifetime_remaining = lifetime;
        let mut sink = [0_u8; 512];
        let mut steps = 0;
        loop {
            let mut budget =
                WorkBudget::new().with_remaining(WorkClass::CodecInputBytes, step_allowance);
            let before = inspected;
            while inspected < input.len() {
                if budget.remaining(WorkClass::CodecInputBytes) == 0 {
                    break;
                }
                if lifetime_remaining == 0 {
                    return Trace {
                        outcome: Outcome::Limit,
                        inspected,
                        copied,
                        lifetime_remaining,
                    };
                }
                budget.consume(WorkClass::CodecInputBytes, 1).unwrap();
                lifetime_remaining -= 1;
                let byte = input[inspected];
                inspected += 1;
                match number.feed(byte) {
                    Feed::Consumed => {
                        // The test sink is fixed-size; every tested ceiling
                        // is below its capacity. The lexer itself owns none.
                        sink[copied] = byte;
                        copied += 1;
                    }
                    Feed::Complete => {
                        return Trace {
                            outcome: Outcome::Complete,
                            inspected,
                            copied,
                            lifetime_remaining,
                        };
                    }
                    Feed::Limit => {
                        return Trace {
                            outcome: Outcome::Limit,
                            inspected,
                            copied,
                            lifetime_remaining,
                        };
                    }
                    Feed::Invalid => {
                        return Trace {
                            outcome: Outcome::Invalid,
                            inspected,
                            copied,
                            lifetime_remaining,
                        };
                    }
                }
            }
            if inspected == input.len() {
                let outcome = match number.finish_eof() {
                    Feed::Complete => Outcome::Complete,
                    Feed::Invalid => Outcome::Invalid,
                    Feed::Limit => Outcome::Limit,
                    Feed::Consumed => unreachable!(),
                };
                return Trace {
                    outcome,
                    inspected,
                    copied,
                    lifetime_remaining,
                };
            }
            steps += 1;
            if before == inspected || steps > input.len() {
                return Trace {
                    outcome: Outcome::Pending,
                    inspected,
                    copied,
                    lifetime_remaining,
                };
            }
        }
    }

    fn exponent(length: usize) -> String {
        assert!(length >= 4);
        format!("1e+{}1", "0".repeat(length - 4))
    }

    #[test]
    fn named_profile_thresholds_stop_at_first_over_limit_number_byte() {
        for (limit, expected) in [
            (GatewayDefaultV1::limits().number_lexeme_bytes_max(), 256),
            (
                BenchmarkStaticReferenceV1::limits().number_lexeme_bytes_max(),
                64,
            ),
        ] {
            let limit = usize::try_from(limit.unwrap()).unwrap();
            assert_eq!(limit, expected);
            for length in [limit - 1, limit, limit + 1] {
                let input = format!("{},", exponent(length));
                let trace = run(input.as_bytes(), limit, 1, 1_000);
                if length <= limit {
                    assert_eq!(trace.outcome, Outcome::Complete);
                    assert_eq!(trace.inspected, length + 1); // delimiter observed
                    assert_eq!(trace.copied, length);
                } else {
                    assert_eq!(trace.outcome, Outcome::Limit);
                    assert_eq!(trace.inspected, limit + 1);
                    assert_eq!(trace.copied, limit);
                }
                assert_eq!(trace.lifetime_remaining, 1_000 - trace.inspected as u64);
            }
        }
    }

    #[test]
    fn zero_rejects_the_first_number_byte_even_for_opaque_numbers() {
        for input in ["0]", "1e309]", "-1]"] {
            assert_eq!(
                run(input.as_bytes(), 0, 1, 10),
                Trace {
                    outcome: Outcome::Limit,
                    inspected: 1,
                    copied: 0,
                    lifetime_remaining: 9,
                }
            );
        }
        assert_eq!(NumberLexeme::new(0).finish_eof(), Feed::Invalid);
    }

    #[test]
    fn long_exponent_stops_without_finishing_or_copying_the_tail() {
        let input = format!("{}X", exponent(65_536));
        let trace = run(input.as_bytes(), 64, 3, 1_000);
        assert_eq!(trace.outcome, Outcome::Limit);
        assert_eq!(trace.inspected, 65);
        assert_eq!(trace.copied, 64);
        assert_eq!(trace.lifetime_remaining, 935);
        // The late X would be invalid syntax if the lexer scanned the tail.
        assert!(input.len() > trace.inspected + 65_000);
    }

    #[test]
    fn application_limit_above_256_is_a_real_lexical_policy() {
        let accepted = format!("{}]", exponent(257));
        assert_eq!(
            run(accepted.as_bytes(), 257, 3, 1_000).outcome,
            Outcome::Complete
        );
        let rejected = format!("{}]", exponent(258));
        let trace = run(rejected.as_bytes(), 257, 3, 1_000);
        assert_eq!(trace.outcome, Outcome::Limit);
        assert_eq!((trace.inspected, trace.copied), (258, 257));
    }

    #[test]
    fn zero_step_and_short_lifetime_observe_no_unpaid_byte() {
        assert_eq!(
            run(b"1e+01,", 64, 0, 64),
            Trace {
                outcome: Outcome::Pending,
                inspected: 0,
                copied: 0,
                lifetime_remaining: 64,
            }
        );
        assert_eq!(
            run(b"1e+01,", 64, 1, 2),
            Trace {
                outcome: Outcome::Limit,
                inspected: 2,
                copied: 2,
                lifetime_remaining: 0,
            }
        );
    }

    #[test]
    fn grammar_and_terminal_state_are_stable_across_steps() {
        for valid in ["0", "-0", "12.3e-4", "1E+9"] {
            assert_eq!(run(valid.as_bytes(), 64, 1, 64).outcome, Outcome::Complete);
            assert!(serde_json::from_str::<serde_json::Number>(valid).is_ok());
        }
        for invalid in ["", "-", "01", "1.", "1e", "1e+", "1x", "1e+x"] {
            assert_eq!(
                run(invalid.as_bytes(), 64, 1, 64).outcome,
                Outcome::Invalid,
                "{invalid}"
            );
            assert!(serde_json::from_str::<serde_json::Number>(invalid).is_err());
        }
        let mut number = NumberLexeme::new(1);
        assert_eq!(number.feed(b'1'), Feed::Consumed);
        assert_eq!(number.feed(b'2'), Feed::Limit);
        assert_eq!(number.consumed(), 1);
        assert_eq!(number.feed(b','), Feed::Limit); // terminal Limit cannot be finished away
    }
}
