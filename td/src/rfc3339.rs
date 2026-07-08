//! Minimal, dependency-free RFC 3339 serde for [`time::OffsetDateTime`].
//!
//! `time`'s built-in `time::serde::rfc3339` requires the `formatting` and
//! `parsing` features, both of which hard-depend on `std` (see `time`'s
//! `[features]`: `formatting = […, "std", …]`). That would force `std` onto
//! [`crate::Thing`]'s `created` / `modified` fields and break the crate's
//! `no_std + alloc` guarantee.
//!
//! This module reimplements just the RFC 3339 subset that WoT Thing
//! Descriptions use — `YYYY-MM-DDTHH:MM:SS[.frac][Z|±HH:MM[:SS]]` — using only
//! `time`'s core type accessors and constructors (which need no feature flags)
//! and `core`/`alloc`. The serializer matches `time`'s well-known RFC 3339
//! output: fractional seconds are emitted with trailing zeros trimmed
//! (e.g. `.52`, `.451`, or omitted entirely when the sub-second is zero), and a
//! zero offset is emitted as `Z`.
//!
//! Round-trip fidelity is *semantic*: the project's fixture comparison
//! (`td::tests`) treats RFC 3339 values as equal after trimming trailing `Z`
//! and `0`, so the exact fractional width is not significant.

use alloc::string::String;

use core::fmt::Write as _;

use serde::{Deserializer, Serializer};
use time::{Date, Month, OffsetDateTime, Time, UtcOffset};

/// Serialize an [`OffsetDateTime`] as an RFC 3339 string.
pub fn serialize<S>(datetime: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&format_rfc3339(datetime))
}

/// Deserialize an [`OffsetDateTime`] from an RFC 3339 string.
pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{Error, Visitor};

    struct Rfc3339Visitor;

    impl Visitor<'_> for Rfc3339Visitor {
        type Value = OffsetDateTime;

        fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("an RFC 3339 date-time string")
        }

        fn visit_str<E>(self, value: &str) -> Result<OffsetDateTime, E>
        where
            E: Error,
        {
            parse_rfc3339(value).map_err(E::custom)
        }
    }

    deserializer.deserialize_str(Rfc3339Visitor)
}

/// Formats `datetime` as an RFC 3339 string, matching `time`'s well-known
/// RFC 3339 output (trailing-zero-trimmed fractional seconds; `Z` for UTC).
fn format_rfc3339(datetime: &OffsetDateTime) -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        datetime.year(),
        datetime.month() as u8,
        datetime.day(),
        datetime.hour(),
        datetime.minute(),
        datetime.second()
    );

    let nanos = datetime.nanosecond();
    if nanos != 0 {
        // Nine-digit nanosecond fraction with trailing zeros trimmed, matching
        // `time`'s well-known RFC 3339 (e.g. 520_000_000 -> ".52").
        let mut digits = [b'0'; 9];
        let mut rest = nanos;
        for slot in digits.iter_mut().rev() {
            *slot = b'0' + (rest % 10) as u8;
            rest /= 10;
        }
        let mut len = digits.len();
        while len > 0 && digits[len - 1] == b'0' {
            len -= 1;
        }
        out.push('.');
        for &b in &digits[..len] {
            out.push(b as char);
        }
    }

    let offset = datetime.offset();
    if offset.whole_seconds() == 0 {
        out.push('Z');
    } else {
        let (hours, minutes, seconds) = offset.as_hms();
        let sign = if hours >= 0 { '+' } else { '-' };
        let _ = write!(
            out,
            "{}{:02}:{:02}",
            sign,
            hours.unsigned_abs(),
            minutes.unsigned_abs()
        );
        if seconds != 0 {
            let _ = write!(out, ":{:02}", seconds.unsigned_abs());
        }
    }

    out
}

/// Parses an RFC 3339 date-time string into an [`OffsetDateTime`].
fn parse_rfc3339(input: &str) -> Result<OffsetDateTime, ParseError> {
    let mut cursor = Cursor {
        bytes: input.as_bytes(),
        pos: 0,
    };

    let year = cursor.digits(4)? as i32;
    cursor.eat(b'-')?;
    let month = cursor.digits(2)? as u8;
    cursor.eat(b'-')?;
    let day = cursor.digits(2)? as u8;

    let sep = cursor.bump().ok_or(ParseError::Invalid)?;
    if !matches!(sep, b'T' | b't' | b' ') {
        return Err(ParseError::Invalid);
    }

    let hour = cursor.digits(2)? as u8;
    cursor.eat(b':')?;
    let minute = cursor.digits(2)? as u8;
    cursor.eat(b':')?;
    let second = cursor.digits(2)? as u8;

    let nanosecond = cursor.optional_fraction()?;

    let offset = match cursor.peek() {
        Some(b'Z' | b'z') => {
            cursor.bump();
            UtcOffset::UTC
        }
        Some(sign @ (b'+' | b'-')) => {
            cursor.bump();
            let hours = cursor.digits(2)? as i8;
            cursor.eat(b':')?;
            let minutes = cursor.digits(2)? as i8;
            let mut seconds = 0_i8;
            if cursor.peek() == Some(b':') {
                cursor.bump();
                seconds = cursor.digits(2)? as i8;
            }
            let signed = |v: i8| -> i8 { if sign == b'+' { v } else { -v } };
            UtcOffset::from_hms(signed(hours), signed(minutes), signed(seconds))
                .map_err(|_| ParseError::OutOfRange)?
        }
        _ => return Err(ParseError::Invalid),
    };

    if cursor.pos != cursor.bytes.len() {
        return Err(ParseError::Trailing);
    }

    let date = Date::from_calendar_date(
        year,
        Month::try_from(month).map_err(|_| ParseError::OutOfRange)?,
        day,
    )
    .map_err(|_| ParseError::OutOfRange)?;
    let time = Time::from_hms_nano(hour, minute, second, nanosecond)
        .map_err(|_| ParseError::OutOfRange)?;
    Ok(OffsetDateTime::new_in_offset(date, time, offset))
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    fn eat(&mut self, expected: u8) -> Result<(), ParseError> {
        match self.bump() {
            Some(c) if c == expected => Ok(()),
            _ => Err(ParseError::Invalid),
        }
    }

    fn digits(&mut self, count: usize) -> Result<u64, ParseError> {
        let mut value = 0_u64;
        for _ in 0..count {
            let c = self.bump().ok_or(ParseError::Invalid)?;
            if !c.is_ascii_digit() {
                return Err(ParseError::Invalid);
            }
            value = value * 10 + u64::from(c - b'0');
        }
        Ok(value)
    }

    /// Reads a `.` followed by one or more digits and returns the value scaled
    /// to nanoseconds. Extra digits beyond nine (nanosecond precision) are
    /// truncated; fewer are zero-padded.
    fn optional_fraction(&mut self) -> Result<u32, ParseError> {
        if self.peek() != Some(b'.') {
            return Ok(0);
        }
        self.bump();

        let mut nanos = 0_u32;
        let mut count = 0_u32;
        while let Some(c) = self.peek() {
            if !c.is_ascii_digit() {
                break;
            }
            self.bump();
            if count < 9 {
                nanos = nanos * 10 + u32::from(c - b'0');
                count += 1;
            }
        }
        if count == 0 {
            return Err(ParseError::Invalid);
        }
        while count < 9 {
            nanos *= 10;
            count += 1;
        }
        Ok(nanos)
    }
}

/// Parse failures for [`parse_rfc3339`].
#[derive(Debug)]
enum ParseError {
    /// Malformed syntax (wrong separator, non-digit, missing offset, ...).
    Invalid,
    /// A calendar component was out of the valid range.
    OutOfRange,
    /// Trailing characters after a complete RFC 3339 value.
    Trailing,
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParseError::Invalid => f.write_str("invalid RFC 3339 date-time syntax"),
            ParseError::OutOfRange => f.write_str("RFC 3339 component out of range"),
            ParseError::Trailing => f.write_str("trailing characters after RFC 3339 value"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{format_rfc3339, parse_rfc3339};
    use time::{Date, Month, OffsetDateTime, Time, UtcOffset};

    fn odt(year: i32, month: u8, day: u8, h: u8, m: u8, s: u8, nanos: u32) -> OffsetDateTime {
        let date = Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap();
        let time = Time::from_hms_nano(h, m, s, nanos).unwrap();
        OffsetDateTime::new_in_offset(date, time, UtcOffset::UTC)
    }

    #[test]
    fn formats_whole_seconds_as_z() {
        assert_eq!(
            format_rfc3339(&odt(2018, 9, 10, 6, 30, 0, 0)),
            "2018-09-10T06:30:00Z"
        );
    }

    #[test]
    fn trims_fractional_trailing_zeros() {
        assert_eq!(
            format_rfc3339(&odt(2019, 1, 29, 21, 15, 20, 451_000_000)),
            "2019-01-29T21:15:20.451Z"
        );
        // 520 ms matches `time`'s documented ".52" output.
        assert_eq!(
            format_rfc3339(&odt(1985, 4, 12, 23, 20, 50, 520_000_000)),
            "1985-04-12T23:20:50.52Z"
        );
    }

    #[test]
    fn round_trips_fixture_shapes() {
        for original in [
            "2018-09-10T06:30:00Z",
            "2019-01-29T21:15:20.451Z",
            "2019-05-28T05:43:02.346Z",
        ] {
            let parsed = parse_rfc3339(original).unwrap();
            assert_eq!(format_rfc3339(&parsed), original);
        }
    }

    #[test]
    fn parses_lowercase_z_and_offset() {
        assert!(parse_rfc3339("2018-09-10T06:30:00z").is_ok());
        let with_offset = parse_rfc3339("2018-09-10T06:30:00+05:30").unwrap();
        assert_eq!(with_offset.offset(), UtcOffset::from_hms(5, 30, 0).unwrap());
    }

    #[test]
    fn rejects_trailing_garbage() {
        assert!(parse_rfc3339("2018-09-10T06:30:00Z extra").is_err());
    }
}
