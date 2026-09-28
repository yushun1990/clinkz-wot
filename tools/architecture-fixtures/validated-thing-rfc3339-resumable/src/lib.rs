//! Non-production shared RFC3339 resumability probe for WP-100.
#![no_std]
#![allow(dead_code)]

extern crate alloc;

use clinkz_wot_foundation::{WorkBudget, WorkClass};
use time::OffsetDateTime;

#[allow(dead_code)]
mod existing {
    include!(concat!(env!("OUT_DIR"), "/existing.rs"));
}

#[allow(dead_code)]
mod prototype {
    include!(concat!(env!("OUT_DIR"), "/prototype.rs"));
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Trace {
    source_bytes: u64,
    json_transitions: u64,
    date_transitions: u64,
    fixed_finalizations: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum JsonPhase {
    OpeningQuote,
    Normal,
    Escape,
    Unicode,
    HighSurrogateBackslash,
    HighSurrogateU,
    LowSurrogate,
    Utf8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StrictInvalid {
    JsonSyntax,
    Date(prototype::ParseError),
}

/// Fixed-size composition of JSON string unescaping and the shared date
/// decoder. This models one strict date-field token, not a complete TD parser.
struct StrictDateCursor<'a> {
    input: &'a [u8],
    position: usize,
    lifetime_remaining: u64,
    json_phase: JsonPhase,
    unicode_value: u16,
    unicode_digits: u8,
    high_surrogate: u16,
    utf8: [u8; 4],
    utf8_len: u8,
    utf8_expected: u8,
    date: prototype::Decoder,
    trace: Trace,
}

enum StrictProgress<'a> {
    Pending(StrictDateCursor<'a>),
    Complete {
        value: OffsetDateTime,
        trace: Trace,
    },
    Invalid {
        cause: StrictInvalid,
        position: usize,
        trace: Trace,
    },
    Limit {
        position: usize,
        trace: Trace,
    },
    Cancelled {
        position: usize,
        trace: Trace,
    },
}

impl<'a> StrictDateCursor<'a> {
    const fn new(input: &'a [u8], lifetime_remaining: u64) -> Self {
        Self {
            input,
            position: 0,
            lifetime_remaining,
            json_phase: JsonPhase::OpeningQuote,
            unicode_value: 0,
            unicode_digits: 0,
            high_surrogate: 0,
            utf8: [0; 4],
            utf8_len: 0,
            utf8_expected: 0,
            date: prototype::Decoder::new(),
            trace: Trace {
                source_bytes: 0,
                json_transitions: 0,
                date_transitions: 0,
                fixed_finalizations: 0,
            },
        }
    }

    const fn position(&self) -> usize {
        self.position
    }

    const fn lifetime_remaining(&self) -> u64 {
        self.lifetime_remaining
    }

    const fn trace(&self) -> Trace {
        self.trace
    }

    /// Runs only work paid by this call's budget and the cursor's shared,
    /// non-resettable lifetime remainder.
    fn step(mut self, budget: &mut WorkBudget, cancel_requested: bool) -> StrictProgress<'a> {
        if cancel_requested {
            return StrictProgress::Cancelled {
                position: self.position,
                trace: self.trace,
            };
        }

        while self.position < self.input.len() {
            if budget.remaining(WorkClass::CodecInputBytes) == 0 {
                return StrictProgress::Pending(self);
            }
            if self.lifetime_remaining == 0 {
                return StrictProgress::Limit {
                    position: self.position,
                    trace: self.trace,
                };
            }

            // Both debits happen before this byte is read or interpreted.
            budget.consume(WorkClass::CodecInputBytes, 1).unwrap();
            self.lifetime_remaining -= 1;
            let byte = self.input[self.position];
            self.position += 1;
            self.trace.source_bytes += 1;
            self.trace.json_transitions += 1;

            match self.process_json_byte(byte) {
                Ok(Some(value)) => {
                    return StrictProgress::Complete {
                        value,
                        trace: self.trace,
                    };
                }
                Ok(None) => {}
                Err(cause) => {
                    return StrictProgress::Invalid {
                        cause,
                        position: self.position,
                        trace: self.trace,
                    };
                }
            }
        }

        // Empty input has no charged predecessor to own its EOF rejection, so
        // its fixed transition consumes one standalone unit from both work
        // budgets before becoming terminal.
        if self.input.is_empty() {
            if budget.remaining(WorkClass::CodecInputBytes) == 0 {
                return StrictProgress::Pending(self);
            }
            if self.lifetime_remaining == 0 {
                return StrictProgress::Limit {
                    position: self.position,
                    trace: self.trace,
                };
            }
            budget.consume(WorkClass::CodecInputBytes, 1).unwrap();
            self.lifetime_remaining -= 1;
        }

        // Reaching token EOF for non-empty input is a bounded check owned
        // by the last charged source byte. Valid completion occurs at `"`.
        self.trace.fixed_finalizations += 1;
        StrictProgress::Invalid {
            cause: StrictInvalid::JsonSyntax,
            position: self.position,
            trace: self.trace,
        }
    }

    fn process_json_byte(&mut self, byte: u8) -> Result<Option<OffsetDateTime>, StrictInvalid> {
        match self.json_phase {
            JsonPhase::OpeningQuote => {
                if byte != b'"' {
                    return Err(StrictInvalid::JsonSyntax);
                }
                self.json_phase = JsonPhase::Normal;
            }
            JsonPhase::Normal => match byte {
                b'"' => {
                    // This fixture consumes exactly one complete JSON string
                    // token; a full strict TD parser would return control to
                    // its outer JSON continuation here.
                    if self.position != self.input.len() {
                        return Err(StrictInvalid::JsonSyntax);
                    }
                    self.trace.fixed_finalizations += 1;
                    let value = self.date.finish().map_err(StrictInvalid::Date)?;
                    return Ok(Some(value));
                }
                b'\\' => self.json_phase = JsonPhase::Escape,
                0x00..=0x1f => return Err(StrictInvalid::JsonSyntax),
                0x20..=0x7f => self.feed_decoded(byte)?,
                0xc2..=0xdf => self.start_utf8(byte, 2),
                0xe0..=0xef => self.start_utf8(byte, 3),
                0xf0..=0xf4 => self.start_utf8(byte, 4),
                _ => return Err(StrictInvalid::JsonSyntax),
            },
            JsonPhase::Escape => {
                let decoded = match byte {
                    b'"' | b'\\' | b'/' => Some(byte),
                    b'b' => Some(0x08),
                    b'f' => Some(0x0c),
                    b'n' => Some(b'\n'),
                    b'r' => Some(b'\r'),
                    b't' => Some(b'\t'),
                    b'u' => {
                        self.unicode_value = 0;
                        self.unicode_digits = 0;
                        self.json_phase = JsonPhase::Unicode;
                        None
                    }
                    _ => return Err(StrictInvalid::JsonSyntax),
                };
                if let Some(decoded) = decoded {
                    self.json_phase = JsonPhase::Normal;
                    self.feed_decoded(decoded)?;
                }
            }
            JsonPhase::Unicode => {
                if self.push_hex(byte)? {
                    let value = self.unicode_value;
                    if (0xd800..=0xdbff).contains(&value) {
                        self.high_surrogate = value;
                        self.json_phase = JsonPhase::HighSurrogateBackslash;
                    } else if (0xdc00..=0xdfff).contains(&value) {
                        return Err(StrictInvalid::JsonSyntax);
                    } else {
                        self.json_phase = JsonPhase::Normal;
                        self.feed_scalar(u32::from(value))?;
                    }
                }
            }
            JsonPhase::HighSurrogateBackslash => {
                if byte != b'\\' {
                    return Err(StrictInvalid::JsonSyntax);
                }
                self.json_phase = JsonPhase::HighSurrogateU;
            }
            JsonPhase::HighSurrogateU => {
                if byte != b'u' {
                    return Err(StrictInvalid::JsonSyntax);
                }
                self.unicode_value = 0;
                self.unicode_digits = 0;
                self.json_phase = JsonPhase::LowSurrogate;
            }
            JsonPhase::LowSurrogate => {
                if self.push_hex(byte)? {
                    let low = self.unicode_value;
                    if !(0xdc00..=0xdfff).contains(&low) {
                        return Err(StrictInvalid::JsonSyntax);
                    }
                    let scalar = 0x1_0000
                        + ((u32::from(self.high_surrogate) - 0xd800) << 10)
                        + (u32::from(low) - 0xdc00);
                    self.json_phase = JsonPhase::Normal;
                    self.feed_scalar(scalar)?;
                }
            }
            JsonPhase::Utf8 => {
                if !matches!(byte, 0x80..=0xbf) {
                    return Err(StrictInvalid::JsonSyntax);
                }
                self.utf8[usize::from(self.utf8_len)] = byte;
                self.utf8_len += 1;
                if self.utf8_len == self.utf8_expected {
                    let len = usize::from(self.utf8_len);
                    core::str::from_utf8(&self.utf8[..len])
                        .map_err(|_| StrictInvalid::JsonSyntax)?;
                    self.json_phase = JsonPhase::Normal;
                    self.feed_utf8_buffer(len)?;
                }
            }
        }
        Ok(None)
    }

    fn push_hex(&mut self, byte: u8) -> Result<bool, StrictInvalid> {
        let digit = match byte {
            b'0'..=b'9' => u16::from(byte - b'0'),
            b'a'..=b'f' => u16::from(byte - b'a') + 10,
            b'A'..=b'F' => u16::from(byte - b'A') + 10,
            _ => return Err(StrictInvalid::JsonSyntax),
        };
        self.unicode_value = self.unicode_value * 16 + digit;
        self.unicode_digits += 1;
        Ok(self.unicode_digits == 4)
    }

    fn start_utf8(&mut self, first: u8, expected: u8) {
        self.utf8 = [0; 4];
        self.utf8[0] = first;
        self.utf8_len = 1;
        self.utf8_expected = expected;
        self.json_phase = JsonPhase::Utf8;
    }

    fn feed_utf8_buffer(&mut self, len: usize) -> Result<(), StrictInvalid> {
        // The loop has a compile-time maximum of four bytes.
        for index in 0..len {
            self.feed_decoded(self.utf8[index])?;
        }
        Ok(())
    }

    fn feed_scalar(&mut self, scalar: u32) -> Result<(), StrictInvalid> {
        let scalar = char::from_u32(scalar).ok_or(StrictInvalid::JsonSyntax)?;
        let mut bytes = [0; 4];
        let encoded = scalar.encode_utf8(&mut bytes);
        for &byte in encoded.as_bytes() {
            self.feed_decoded(byte)?;
        }
        Ok(())
    }

    fn feed_decoded(&mut self, byte: u8) -> Result<(), StrictInvalid> {
        self.trace.date_transitions += 1;
        self.date.feed(byte).map_err(StrictInvalid::Date)
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use alloc::{
        format,
        string::{String, ToString},
        vec::Vec,
    };
    use core::{alloc::GlobalAlloc, cell::Cell, mem};
    use serde::de::value::{Error as SerdeError, StrDeserializer};
    use std::alloc::System;

    struct ThreadCountingAllocator;

    std::thread_local! {
        static TRACK_ALLOCATIONS: Cell<bool> = const { Cell::new(false) };
        static ALLOCATION_COUNT: Cell<usize> = const { Cell::new(0) };
    }

    unsafe impl GlobalAlloc for ThreadCountingAllocator {
        unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
            TRACK_ALLOCATIONS.with(|active| {
                if active.get() {
                    ALLOCATION_COUNT.with(|count| count.set(count.get() + 1));
                }
            });
            // SAFETY: forwarding the allocator contract unchanged to System.
            unsafe { System.alloc(layout) }
        }

        unsafe fn dealloc(&self, pointer: *mut u8, layout: core::alloc::Layout) {
            // SAFETY: `pointer` and `layout` came from the forwarded System allocation.
            unsafe { System.dealloc(pointer, layout) }
        }
    }

    #[global_allocator]
    static GLOBAL_ALLOCATOR: ThreadCountingAllocator = ThreadCountingAllocator;

    fn count_allocations<T>(operation: impl FnOnce() -> T) -> (T, usize) {
        ALLOCATION_COUNT.with(|count| count.set(0));
        TRACK_ALLOCATIONS.with(|active| active.set(true));
        let value = operation();
        TRACK_ALLOCATIONS.with(|active| active.set(false));
        let count = ALLOCATION_COUNT.with(Cell::get);
        (value, count)
    }

    fn parse_existing(input: &str) -> Result<OffsetDateTime, String> {
        existing::deserialize(StrDeserializer::<SerdeError>::new(input))
            .map_err(|error| error.to_string())
    }

    fn parse_prototype(input: &str) -> Result<OffsetDateTime, String> {
        prototype::deserialize(StrDeserializer::<SerdeError>::new(input))
            .map_err(|error| error.to_string())
    }

    fn assert_same(input: &str) {
        assert_eq!(
            parse_prototype(input),
            parse_existing(input),
            "semantic mismatch for {input:?}"
        );
    }

    #[derive(Debug, Eq, PartialEq)]
    enum OwnedOutcome {
        Complete(OffsetDateTime, Trace),
        Invalid(StrictInvalid, usize, Trace),
        Limit(usize, Trace),
        Cancelled(usize, Trace),
    }

    type Utf8Pause = (u8, u8);
    type Utf8Case<'a> = (&'a [u8], &'a [Utf8Pause]);

    fn trace_of(progress: &StrictProgress<'_>) -> Trace {
        match progress {
            StrictProgress::Pending(cursor) => cursor.trace(),
            StrictProgress::Complete { trace, .. }
            | StrictProgress::Invalid { trace, .. }
            | StrictProgress::Limit { trace, .. }
            | StrictProgress::Cancelled { trace, .. } => *trace,
        }
    }

    fn run_strict_observing_pending(
        input: &[u8],
        lifetime: u64,
        allowances: &[u64],
        mut observe: impl FnMut(&StrictDateCursor<'_>),
    ) -> OwnedOutcome {
        assert!(!allowances.is_empty());
        let mut cursor = StrictDateCursor::new(input, lifetime);
        let mut step = 0_usize;
        loop {
            let allowance = allowances[step % allowances.len()];
            let before = cursor.trace();
            let before_position = cursor.position();
            let mut budget =
                WorkBudget::new().with_remaining(WorkClass::CodecInputBytes, allowance);
            let progress = cursor.step(&mut budget, false);
            let after = trace_of(&progress);
            let source_delta = after.source_bytes - before.source_bytes;
            let json_delta = after.json_transitions - before.json_transitions;
            let date_delta = after.date_transitions - before.date_transitions;
            let finalize_delta = after.fixed_finalizations - before.fixed_finalizations;
            let charged_delta = allowance - budget.remaining(WorkClass::CodecInputBytes);
            let standalone_eof_delta = u64::from(source_delta == 0 && finalize_delta == 1);

            assert_eq!(source_delta, json_delta);
            assert_eq!(charged_delta, source_delta + standalone_eof_delta);
            assert!(source_delta <= allowance);
            assert!(date_delta <= source_delta * 4);
            assert!(finalize_delta <= charged_delta);
            assert!(json_delta + date_delta + finalize_delta <= charged_delta * 6);

            match progress {
                StrictProgress::Pending(next) => {
                    assert_eq!(next.position() as u64, after.source_bytes);
                    if allowance == 0 {
                        assert_eq!(next.position(), before_position);
                        assert_eq!(after, before);
                    } else {
                        assert!(next.position() > before_position);
                    }
                    observe(&next);
                    cursor = next;
                }
                StrictProgress::Complete { value, trace } => {
                    return OwnedOutcome::Complete(value, trace);
                }
                StrictProgress::Invalid {
                    cause,
                    position,
                    trace,
                } => return OwnedOutcome::Invalid(cause, position, trace),
                StrictProgress::Limit { position, trace } => {
                    return OwnedOutcome::Limit(position, trace);
                }
                StrictProgress::Cancelled { position, trace } => {
                    return OwnedOutcome::Cancelled(position, trace);
                }
            }
            step += 1;
            assert!(step <= input.len().saturating_mul(4).saturating_add(16));
        }
    }

    fn run_strict(input: &[u8], lifetime: u64, allowances: &[u64]) -> OwnedOutcome {
        run_strict_observing_pending(input, lifetime, allowances, |_| {})
    }

    fn json_oracle(input: &[u8]) -> Result<OffsetDateTime, String> {
        let decoded: String = serde_json::from_slice(input).map_err(|error| error.to_string())?;
        parse_existing(&decoded)
    }

    fn strict_date_result(outcome: OwnedOutcome) -> Result<OffsetDateTime, String> {
        match outcome {
            OwnedOutcome::Complete(value, _) => Ok(value),
            OwnedOutcome::Invalid(StrictInvalid::Date(error), _, _) => Err(error.to_string()),
            other => panic!("unexpected strict result: {other:?}"),
        }
    }

    fn timestamp(fraction_digits: usize) -> String {
        format!("2026-09-09T00:00:00.{}Z", "1".repeat(fraction_digits))
    }

    #[test]
    fn synchronous_adapter_matches_the_current_real_decoder() {
        let representative = [
            "2018-09-10T06:30:00Z",
            "2018-09-10t06:30:00z",
            "2018-09-10 06:30:00+05:30",
            "1985-04-12T23:20:50.52-00:30",
            "1990-12-31T23:59:60Z",
            "2024-02-29T12:34:56.123456789123+05:30:45",
            "0000-01-01T00:00:00Z",
            "9999-12-31T23:59:59Z",
        ];
        let invalid = [
            "",
            "2018-09-10T06:30:00",
            "2018-09-10T06:30:00.Z",
            "2018-09-10T06:30:00Zx",
            "2018-09-10T06:30:00+05",
            "2018-09-10T06:30:00+05:30:",
            "2018-09-10T06:30:00+24:00",
            "2018-13-10T06:30:00Z",
            "2018-02-30T06:30:00Z",
            "2018-09-10T24:00:00Z",
            "2018-09-10X06:30:00Z",
            "2018-09-10T06:30:00.1xZ",
            "2018-13-10T06:30:00Zx",
            "2018-13-10T06:30:00+24:00x",
        ];
        for input in representative.into_iter().chain(invalid) {
            assert_same(input);
        }

        let canonical = b"2026-09-09T00:00:00.123456789+05:30:45";
        for index in 0..canonical.len() {
            let mut replaced = canonical.to_vec();
            replaced[index] = b'x';
            assert_same(core::str::from_utf8(&replaced).unwrap());

            let mut removed = canonical.to_vec();
            removed.remove(index);
            assert_same(core::str::from_utf8(&removed).unwrap());
        }

        for digits in [1, 8, 9, 10, 4_096, 65_536] {
            assert_same(&timestamp(digits));
        }
    }

    #[test]
    fn structured_component_and_error_precedence_corpus_matches() {
        for year in ["0000", "2024", "9999"] {
            for month in ["00", "01", "12", "13"] {
                for day in ["00", "01", "29", "32"] {
                    for separator in ["T", "t", " "] {
                        for hour in ["00", "23", "24"] {
                            for minute in ["00", "59", "60"] {
                                for second in ["00", "59", "60"] {
                                    for fraction in ["", ".", ".1", ".1234567890"] {
                                        for offset in [
                                            "Z",
                                            "z",
                                            "+00:00",
                                            "-00:30",
                                            "+23:59:59",
                                            "+24:00",
                                            "+05:60",
                                            "+05:30x",
                                        ] {
                                            let input = format!(
                                                "{year}-{month}-{day}{separator}{hour}:{minute}:{second}{fraction}{offset}"
                                            );
                                            assert_same(&input);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn long_fraction_resumes_with_one_paid_source_byte_per_step() {
        let decoded = timestamp(65_536);
        let token = format!("\"{decoded}\"");
        let outcome = run_strict(token.as_bytes(), token.len() as u64, &[1]);
        let OwnedOutcome::Complete(value, trace) = outcome else {
            panic!("long valid token did not complete")
        };
        assert_eq!(Ok(value), parse_existing(&decoded));
        assert_eq!(trace.source_bytes, token.len() as u64);
        assert_eq!(trace.date_transitions, decoded.len() as u64);
        assert_eq!(trace.fixed_finalizations, 1);
    }

    #[test]
    fn late_invalid_suffix_is_rejected_without_a_prefix_shortcut_or_tail_scan() {
        let decoded = timestamp(4_096).replace('Z', "xZ");
        let token = format!("\"{decoded}\"");
        let oracle = parse_existing(&decoded);
        let outcome = run_strict(token.as_bytes(), token.len() as u64, &[1, 2, 0, 3]);
        let OwnedOutcome::Invalid(StrictInvalid::Date(error), position, trace) = outcome else {
            panic!("late-invalid token was not a date rejection")
        };
        assert_eq!(Err(error.to_string()), oracle);
        assert_eq!(error, prototype::ParseError::Invalid);
        assert!(position < token.len() - 1);
        assert_eq!(trace.source_bytes, position as u64);

        let trailing = format!("\"{}extra\"", timestamp(4_096));
        let trailing_decoded = serde_json::from_str::<String>(&trailing).unwrap();
        let outcome = run_strict(trailing.as_bytes(), trailing.len() as u64, &[2, 1, 7]);
        assert_eq!(
            strict_date_result(outcome),
            parse_existing(&trailing_decoded)
        );
    }

    #[test]
    fn json_unescaping_and_date_progress_pause_together() {
        let cases: &[&[u8]] = &[
            br#""2026-09-09\u005400\u003a00:00\u002e123456789012\u005a""#,
            br#""2026\u002d09-09T00:00:00\u002b05:30\u003a45""#,
            br#""2026-09-09T00:00:00.1\u0032\u0033z""#,
            br#""2026-09-09T00:00:00Z\u0020extra""#,
            br#""2026-09-09T00:00:00.1\uD83D\uDE00Z""#,
            br#""2026-09-09T00:00:00.1\"Z""#,
            br#""2026-09-09T00:00:00.1\\Z""#,
        ];
        for input in cases {
            let strict = run_strict(input, input.len() as u64, &[1, 0, 2, 3]);
            assert_eq!(strict_date_result(strict), json_oracle(input));
        }

        for malformed in [
            br#""2026-09-09T00:00:00\q""#.as_slice(),
            br#""2026-09-09T00:00:00\u12xz""#.as_slice(),
            br#""2026-09-09T00:00:00\uD800Z""#.as_slice(),
            br#""2026-09-09T00:00:00\uDC00Z""#.as_slice(),
        ] {
            assert!(serde_json::from_slice::<String>(malformed).is_err());
            assert!(matches!(
                run_strict(malformed, malformed.len() as u64, &[1]),
                OwnedOutcome::Invalid(StrictInvalid::JsonSyntax, _, _)
            ));
        }
    }

    #[test]
    fn raw_utf8_pauses_at_each_continuation_and_rejects_malformed_sequences() {
        let valid: &[Utf8Case<'_>] = &[
            (b"\"2026-09-09T00:00:00.1\xc2\xa9Z\"", &[(1, 2)]),
            (b"\"2026-09-09T00:00:00.1\xe2\x82\xacZ\"", &[(1, 3), (2, 3)]),
            (
                b"\"2026-09-09T00:00:00.1\xf0\x9f\x98\x80Z\"",
                &[(1, 4), (2, 4), (3, 4)],
            ),
        ];
        for &(input, expected_pauses) in valid {
            let mut pauses = Vec::new();
            let outcome = run_strict_observing_pending(input, input.len() as u64, &[1], |cursor| {
                if cursor.json_phase == JsonPhase::Utf8 {
                    pauses.push((cursor.utf8_len, cursor.utf8_expected));
                }
            });
            assert_eq!(pauses.as_slice(), expected_pauses);
            assert_eq!(strict_date_result(outcome), json_oracle(input));
        }

        let malformed: &[Utf8Case<'_>] = &[
            (b"\"2026-09-09T00:00:00.1\xc2Z\"", &[(1, 2)]),
            (b"\"2026-09-09T00:00:00.1\xe2\x82Z\"", &[(1, 3), (2, 3)]),
            (
                b"\"2026-09-09T00:00:00.1\xf0\x9f\x98Z\"",
                &[(1, 4), (2, 4), (3, 4)],
            ),
            (b"\"2026-09-09T00:00:00.1\xe0\x80\x80Z\"", &[(1, 3), (2, 3)]),
        ];
        for &(input, expected_pauses) in malformed {
            assert!(serde_json::from_slice::<String>(input).is_err());
            let mut pauses = Vec::new();
            let outcome = run_strict_observing_pending(input, input.len() as u64, &[1], |cursor| {
                if cursor.json_phase == JsonPhase::Utf8 {
                    pauses.push((cursor.utf8_len, cursor.utf8_expected));
                }
            });
            assert_eq!(pauses.as_slice(), expected_pauses);
            assert!(matches!(
                outcome,
                OwnedOutcome::Invalid(StrictInvalid::JsonSyntax, _, _)
            ));
        }
    }

    #[test]
    fn empty_input_eof_transition_obeys_step_and_lifetime_budgets() {
        let cursor = StrictDateCursor::new(b"", 1);
        let mut zero = WorkBudget::new();
        let StrictProgress::Pending(cursor) = cursor.step(&mut zero, false) else {
            panic!("empty input must not become terminal at zero budget")
        };
        assert_eq!(cursor.position(), 0);
        assert_eq!(cursor.trace(), Trace::default());
        assert_eq!(cursor.lifetime_remaining(), 1);

        let mut one = WorkBudget::new().with_remaining(WorkClass::CodecInputBytes, 1);
        assert!(matches!(
            cursor.step(&mut one, false),
            StrictProgress::Invalid {
                cause: StrictInvalid::JsonSyntax,
                position: 0,
                trace: Trace {
                    source_bytes: 0,
                    json_transitions: 0,
                    date_transitions: 0,
                    fixed_finalizations: 1,
                },
            }
        ));
        assert_eq!(one.remaining(WorkClass::CodecInputBytes), 0);

        assert_eq!(
            run_strict(b"", 0, &[0, 1]),
            OwnedOutcome::Limit(0, Trace::default())
        );
    }

    #[test]
    fn zero_and_tiny_step_budgets_do_not_accumulate_bulk_scan_credit() {
        let token = b"\"2026-09-09T00:00:00.12345678901234567890Z\"";
        let cursor = StrictDateCursor::new(token, token.len() as u64);
        let before_size = mem::size_of_val(&cursor);
        let mut zero = WorkBudget::new();
        let StrictProgress::Pending(cursor) = cursor.step(&mut zero, false) else {
            panic!("zero budget must stay pending")
        };
        assert_eq!(cursor.position(), 0);
        assert_eq!(cursor.trace(), Trace::default());
        assert_eq!(cursor.lifetime_remaining(), token.len() as u64);
        assert_eq!(mem::size_of_val(&cursor), before_size);

        let outcome = run_strict(token, token.len() as u64, &[0, 1]);
        assert_eq!(strict_date_result(outcome), json_oracle(token));
    }

    #[test]
    fn lifetime_is_non_resettable_across_fresh_step_budgets() {
        let token = b"\"2026-09-09T00:00:00Z\"";
        let lifetime = 7_u64;
        let outcome = run_strict(token, lifetime, &[1, 3, 9]);
        assert_eq!(
            outcome,
            OwnedOutcome::Limit(
                lifetime as usize,
                Trace {
                    source_bytes: lifetime,
                    json_transitions: lifetime,
                    date_transitions: lifetime - 1,
                    fixed_finalizations: 0,
                }
            )
        );

        // With no current step allowance, Pending wins without inspecting the
        // already exhausted lifetime. A later paid step observes the Limit.
        let cursor = StrictDateCursor::new(token, 0);
        let mut zero = WorkBudget::new();
        let StrictProgress::Pending(cursor) = cursor.step(&mut zero, false) else {
            panic!("zero current allowance must not perform a lifetime check as work")
        };
        let mut one = WorkBudget::new().with_remaining(WorkClass::CodecInputBytes, 1);
        assert!(matches!(
            cursor.step(&mut one, false),
            StrictProgress::Limit {
                position: 0,
                trace: Trace {
                    source_bytes: 0,
                    ..
                }
            }
        ));
        assert_eq!(one.remaining(WorkClass::CodecInputBytes), 1);

        let before_closing_quote = (token.len() - 1) as u64;
        assert_eq!(
            run_strict(token, before_closing_quote, &[100]),
            OwnedOutcome::Limit(
                token.len() - 1,
                Trace {
                    source_bytes: before_closing_quote,
                    json_transitions: before_closing_quote,
                    date_transitions: before_closing_quote - 1,
                    fixed_finalizations: 0,
                }
            )
        );
    }

    #[test]
    fn cancellation_at_a_step_boundary_performs_no_more_work() {
        let token = b"\"2026-09-09T00:00:00.123456789Z\"";
        let cursor = StrictDateCursor::new(token, token.len() as u64);
        let mut first = WorkBudget::new().with_remaining(WorkClass::CodecInputBytes, 9);
        let StrictProgress::Pending(cursor) = cursor.step(&mut first, false) else {
            panic!("first short step must pause")
        };
        let position = cursor.position();
        let trace = cursor.trace();
        let mut next = WorkBudget::new().with_remaining(WorkClass::CodecInputBytes, 100);
        assert!(matches!(
            cursor.step(&mut next, true),
            StrictProgress::Cancelled {
                position: cancelled_position,
                trace: cancelled_trace,
            } if cancelled_position == position && cancelled_trace == trace
        ));
        assert_eq!(next.remaining(WorkClass::CodecInputBytes), 100);

        let cursor = StrictDateCursor::new(token, token.len() as u64);
        let mut budget = WorkBudget::new().with_remaining(WorkClass::CodecInputBytes, 100);
        assert!(matches!(
            cursor.step(&mut budget, true),
            StrictProgress::Cancelled {
                position: 0,
                trace: Trace {
                    source_bytes: 0,
                    ..
                }
            }
        ));
        assert_eq!(budget.remaining(WorkClass::CodecInputBytes), 100);
    }

    #[test]
    fn decoder_and_composed_cursor_are_fixed_inline_and_allocation_free() {
        assert!(!mem::needs_drop::<prototype::Decoder>());
        assert!(!mem::needs_drop::<StrictDateCursor<'static>>());
        assert!(mem::size_of::<prototype::Decoder>() <= 128);
        assert!(mem::size_of::<StrictDateCursor<'static>>() <= 256);

        let token = b"\"2026-09-09T00:00:00.12345678901234567890Z\"";
        let cursor = StrictDateCursor::new(token, token.len() as u64);
        let (progress, allocations) = count_allocations(|| {
            let mut budget =
                WorkBudget::new().with_remaining(WorkClass::CodecInputBytes, token.len() as u64);
            cursor.step(&mut budget, false)
        });
        assert!(matches!(progress, StrictProgress::Complete { .. }));
        assert_eq!(allocations, 0);

        let cursor = StrictDateCursor::new(token, token.len() as u64);
        let (_, drop_allocations) = count_allocations(|| {
            let _unfinished_cursor = cursor;
        });
        assert_eq!(drop_allocations, 0);

        let (parsed, sync_allocations) = count_allocations(|| {
            prototype::deserialize(StrDeserializer::<SerdeError>::new(
                "2026-09-09T00:00:00.12345678901234567890Z",
            ))
        });
        assert!(parsed.is_ok());
        assert_eq!(sync_allocations, 0);
    }

    #[test]
    fn every_small_step_has_a_constant_work_bound() {
        let mut inputs = Vec::new();
        inputs.push(format!("\"{}\"", timestamp(4_096)).into_bytes());
        inputs.push(br#""2026-09-09\u005400\u003a00:00\u002e12345678901234567890\u005a""#.to_vec());
        for input in inputs {
            for allowance in 1..=8 {
                let outcome = run_strict(&input, input.len() as u64, &[allowance]);
                assert!(matches!(outcome, OwnedOutcome::Complete(_, _)));
            }
        }
    }
}
