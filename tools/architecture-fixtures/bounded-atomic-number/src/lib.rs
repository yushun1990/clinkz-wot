//! Non-production lexical-boundary model and public-projection workload.
//! This does not implement a TD decoder or complete pre-readmission item 6.
#![no_std]

extern crate alloc;

use alloc::{format, string::String};
use core::hint::black_box;
use serde_json::Number;

pub const HARD_MAX: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rejection {
    InvalidConfiguration,
    Limit,
    InvalidSchema,
}

/// Models the future owning builder's validation, not raw ResourceLimits.
#[derive(Clone, Copy, Debug)]
pub struct LexemeLimit(usize);

impl LexemeLimit {
    pub fn new(configured: usize) -> Result<Self, Rejection> {
        if configured > HARD_MAX {
            Err(Rejection::InvalidConfiguration)
        } else {
            Ok(Self(configured))
        }
    }

    /// Called by a lexer upon observing a Number byte, before copying it.
    /// The lexer owns syntax, per-byte work charging, and cancellation.
    pub fn observe_byte(self, accepted_bytes: &mut usize) -> Result<(), Rejection> {
        if *accepted_bytes >= self.0 {
            return Err(Rejection::Limit);
        }
        *accepted_bytes += 1;
        Ok(())
    }

    /// Borrowed public AP access; does no projection, formatting, or copying.
    pub fn check_number(self, number: &Number) -> Result<&str, Rejection> {
        let text = number.as_str();
        if text.len() > self.0 {
            Err(Rejection::Limit)
        } else {
            Ok(text)
        }
    }
}

/// Timed board entry: call once between the two cancellation checkpoints.
/// Input construction and work debit are setup; this includes the length guard,
/// public AP projection, finite check, and return to the caller. No cached float.
#[inline(never)]
pub fn project(number: &Number, limit: LexemeLimit) -> Result<f64, Rejection> {
    let number = black_box(number);
    limit.check_number(number)?;
    black_box(number.as_f64())
        .filter(|value| value.is_finite())
        .ok_or(Rejection::InvalidSchema)
}

/// Deterministic adversarial workload. Only one generated lexeme need be live.
/// Allocation and JSON Number construction happen outside timed projection.
pub fn for_each_case(mut visit: impl FnMut(&str, &str)) {
    // Small boundaries and shortest difficult IEEE-754 neighborhoods.
    for text in [
        "0",
        "-0.0",
        "1",
        "-1",
        "9007199254740993",
        "2.2250738585072011e-308",
        "2.2250738585072012e-308",
        "2.2250738585072013e-308",
        "2.2250738585072014e-308",
        "2.4703282292062327e-324",
        "2.4703282292062328e-324",
        "4.9406564584124654e-324",
        "1.7976931348623157e+308",
        "1.7976931348623159e+308",
        "1e+309",
        "-1e+309",
        "1e-999",
        "-1e-999",
    ] {
        visit("ieee-boundary", text);
    }

    for len in 1..=HARD_MAX {
        visit("integer-nines", &"9".repeat(len));
        if len >= 2 {
            visit(
                "negative-integer-nines",
                &format!("-{}", "9".repeat(len - 1)),
            );
        }
        if len >= 5 {
            for tail in ["0", "1", "9"] {
                visit(
                    "exponent-leading-zero",
                    &format!("1e+{}{tail}", "0".repeat(len - 4)),
                );
            }
            visit("exponent-overflow", &format!("1e+{}", "9".repeat(len - 3)));
            visit("exponent-underflow", &format!("1e-{}", "9".repeat(len - 3)));
        }
        // Long significands at exponent extremes force work beyond lexical
        // scanning. Both signs and three digit patterns avoid testing only zeros.
        for sign in ["", "-"] {
            for exponent in [-324, -308, -100, 0, 100, 308, 309] {
                let suffix = format!("e{exponent:+}");
                let overhead = sign.len() + 2 + suffix.len();
                if len <= overhead {
                    continue;
                }
                for digit in ["0", "5", "9"] {
                    let text = format!("{sign}1.{}{suffix}", digit.repeat(len - overhead));
                    visit("long-significand", &text);
                }
            }
        }
    }

    // Exact midpoint between 1 and its next larger binary64. With many digits,
    // the two truncated mantissa candidates straddle a rounding decision in
    // rustc 1.95's dec2flt, forcing its slow fallback. Target coverage must still
    // verify the branch; a future compiler may choose another algorithm.
    const MIDPOINT: &str = "1.00000000000000011102230246251565404236316680908203125";
    for len in MIDPOINT.len()..=HARD_MAX {
        let mut tie = String::from(MIDPOINT);
        tie.push_str(&"0".repeat(len - tie.len()));
        visit("halfway-tie", &tie);
        let mut below = tie.clone().into_bytes();
        // Decimal predecessor at the final decimal place, including borrow.
        for byte in below.iter_mut().rev() {
            if *byte == b'0' {
                *byte = b'9';
            } else {
                *byte -= 1;
                break;
            }
        }
        visit("halfway-below", core::str::from_utf8(&below).unwrap());
        let mut above = tie.into_bytes();
        *above.last_mut().unwrap() += 1;
        visit("halfway-above", core::str::from_utf8(&above).unwrap());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clinkz_wot_foundation::{
        BenchmarkStaticReferenceV1, DirectoryClientDefaultV1, GatewayDefaultV1,
        StaticResourceProfile,
    };

    #[test]
    fn configured_boundaries_match_for_strict_observation_and_typed_input() {
        // All legal values, not just the two named profiles. Zero has no legal
        // Number length; the first byte fails without being copied/projected.
        for configured in 0..=HARD_MAX {
            let limit = LexemeLimit::new(configured).unwrap();
            for len in [configured.saturating_sub(1), configured, configured + 1] {
                if len == 0 {
                    continue;
                } // Empty text is not a JSON Number.
                let input = "7".repeat(len);
                let number: Number = serde_json::from_str(&input).unwrap();
                let mut copied = 0;
                let mut observed = 0;
                let mut strict = Ok(());
                for _byte in input.bytes() {
                    observed += 1;
                    strict = limit.observe_byte(&mut copied);
                    if strict.is_err() {
                        break;
                    }
                }
                assert_eq!(strict, limit.check_number(&number).map(|_| ()));
                if len > configured {
                    assert_eq!(strict, Err(Rejection::Limit));
                    assert_eq!(observed, configured + 1);
                    assert_eq!(copied, configured);
                    assert_eq!(project(&number, limit), Err(Rejection::Limit));
                } else {
                    assert_eq!(strict, Ok(()));
                    assert_eq!(copied, len);
                }
            }
        }
        assert!(matches!(
            LexemeLimit::new(257),
            Err(Rejection::InvalidConfiguration)
        ));
        assert!(matches!(
            LexemeLimit::new(usize::MAX),
            Err(Rejection::InvalidConfiguration)
        ));
    }

    #[test]
    fn over_limit_stream_does_not_read_a_finishing_suffix() {
        for configured in [0, 1, 64, 256] {
            let limit = LexemeLimit::new(configured).unwrap();
            let mut copied = 0;
            // An effectively unbounded Number prefix; touching byte L+2 fails
            // the oracle. No string construction or complete input length.
            let stream = (0..).map(|index| {
                assert!(index <= configured, "read beyond first rejected byte");
                b'7'
            });
            for _byte in stream {
                if limit.observe_byte(&mut copied).is_err() {
                    break;
                }
            }
            assert_eq!(copied, configured);
        }
    }

    #[test]
    fn opaque_storage_check_does_not_require_finite_projection() {
        let number: Number = serde_json::from_str("1e+309").unwrap();
        let limit = LexemeLimit::new(64).unwrap();
        assert_eq!(limit.check_number(&number), Ok("1e+309"));
        assert_eq!(project(&number, limit), Err(Rejection::InvalidSchema));
        // A resource failure precedes the otherwise invalid predicate.
        assert_eq!(
            project(&number, LexemeLimit::new(4).unwrap()),
            Err(Rejection::Limit)
        );
        assert_eq!(
            project(&number, LexemeLimit::new(0).unwrap()),
            Err(Rejection::Limit)
        );
    }

    #[test]
    fn corpus_uses_public_ap_numbers_and_covers_every_admitted_length() {
        let mut lengths = [false; HARD_MAX + 1];
        let mut ties = 0;
        for_each_case(|family, text| {
            assert!((1..=HARD_MAX).contains(&text.len()));
            lengths[text.len()] = true;
            let number: Number = serde_json::from_str(text).unwrap();
            assert_eq!(number.as_str(), text);
            let result = project(&number, LexemeLimit::new(HARD_MAX).unwrap());
            let oracle = text
                .parse::<f64>()
                .ok()
                .filter(|f| f.is_finite())
                .ok_or(Rejection::InvalidSchema);
            assert_eq!(result, oracle);
            // Check independently known rounding results, not only the parser
            // oracle. These values are identical at all padded lengths.
            match family {
                "halfway-tie" | "halfway-below" => {
                    assert_eq!(result, Ok(1.0));
                    ties += 1;
                }
                "halfway-above" => assert_eq!(result, Ok(f64::from_bits(1.0f64.to_bits() + 1))),
                _ => {}
            }
        });
        assert!(lengths[1..].iter().all(|present| *present));
        assert!(ties > 0);
        assert_eq!(
            GatewayDefaultV1::LIMITS.number_lexeme_bytes_max(),
            Some(HARD_MAX as u64)
        );
        assert_eq!(
            BenchmarkStaticReferenceV1::LIMITS.number_lexeme_bytes_max(),
            Some(64)
        );
        assert_eq!(
            DirectoryClientDefaultV1::LIMITS.number_lexeme_bytes_max(),
            None
        );
    }
}
