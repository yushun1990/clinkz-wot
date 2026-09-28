/// Parses an RFC 3339 date-time string into an [`OffsetDateTime`].
///
/// This synchronous adapter and the strict probe below drive the same
/// [`Decoder`] rule body. Only the adapter owns this loop.
fn parse_rfc3339(input: &str) -> Result<OffsetDateTime, ParseError> {
    let mut decoder = Decoder::new();
    for &byte in input.as_bytes() {
        decoder.feed(byte)?;
    }
    decoder.finish()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Year,
    FirstDateDash,
    Month,
    SecondDateDash,
    Day,
    DateTimeSeparator,
    Hour,
    FirstTimeColon,
    Minute,
    SecondTimeColon,
    Second,
    FractionOrOffset,
    Fraction,
    OffsetHour,
    OffsetColon,
    OffsetMinute,
    OffsetMaybeSeconds,
    OffsetSecond,
    Complete,
}

/// Fixed-size scalar continuation for the private RFC3339 semantic owner.
///
/// It borrows and allocates nothing. Fraction precision saturates at nine
/// while callers continue feeding every later digit and suffix.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Decoder {
    phase: Phase,
    component_value: u64,
    component_digits: u8,
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    nanosecond: u32,
    fraction_digits: u8,
    offset_sign: i8,
    offset_hour: u8,
    offset_minute: u8,
    offset_second: u8,
    offset: UtcOffset,
    first_error: Option<ParseError>,
}

impl Decoder {
    pub(crate) const fn new() -> Self {
        Self {
            phase: Phase::Year,
            component_value: 0,
            component_digits: 0,
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            nanosecond: 0,
            fraction_digits: 0,
            offset_sign: 1,
            offset_hour: 0,
            offset_minute: 0,
            offset_second: 0,
            offset: UtcOffset::UTC,
            first_error: None,
        }
    }

    /// Processes exactly one decoded string byte with bounded scalar work.
    pub(crate) fn feed(&mut self, byte: u8) -> Result<(), ParseError> {
        if let Some(error) = self.first_error {
            return Err(error);
        }
        let result = self.feed_inner(byte);
        if let Err(error) = result {
            self.first_error = Some(error);
        }
        result
    }

    /// Completes range validation once the caller reaches string end.
    ///
    /// The strict driver invokes this while processing the already charged
    /// closing quote, so it cannot become an uncharged finishing scan.
    pub(crate) fn finish(&mut self) -> Result<OffsetDateTime, ParseError> {
        if let Some(error) = self.first_error {
            return Err(error);
        }
        let result = self.finish_inner();
        if let Err(error) = result {
            self.first_error = Some(error);
        }
        result
    }

    fn feed_inner(&mut self, byte: u8) -> Result<(), ParseError> {
        match self.phase {
            Phase::Year => {
                if let Some(value) = self.digit_component(byte, 4)? {
                    self.year = value as i32;
                    self.phase = Phase::FirstDateDash;
                }
            }
            Phase::FirstDateDash => {
                self.expect(byte, b'-', Phase::Month)?;
            }
            Phase::Month => {
                if let Some(value) = self.digit_component(byte, 2)? {
                    self.month = value as u8;
                    self.phase = Phase::SecondDateDash;
                }
            }
            Phase::SecondDateDash => {
                self.expect(byte, b'-', Phase::Day)?;
            }
            Phase::Day => {
                if let Some(value) = self.digit_component(byte, 2)? {
                    self.day = value as u8;
                    self.phase = Phase::DateTimeSeparator;
                }
            }
            Phase::DateTimeSeparator => {
                if !matches!(byte, b'T' | b't' | b' ') {
                    return Err(ParseError::Invalid);
                }
                self.phase = Phase::Hour;
            }
            Phase::Hour => {
                if let Some(value) = self.digit_component(byte, 2)? {
                    self.hour = value as u8;
                    self.phase = Phase::FirstTimeColon;
                }
            }
            Phase::FirstTimeColon => {
                self.expect(byte, b':', Phase::Minute)?;
            }
            Phase::Minute => {
                if let Some(value) = self.digit_component(byte, 2)? {
                    self.minute = value as u8;
                    self.phase = Phase::SecondTimeColon;
                }
            }
            Phase::SecondTimeColon => {
                self.expect(byte, b':', Phase::Second)?;
            }
            Phase::Second => {
                if let Some(value) = self.digit_component(byte, 2)? {
                    self.second = value as u8;
                    self.phase = Phase::FractionOrOffset;
                }
            }
            Phase::FractionOrOffset => {
                if byte == b'.' {
                    self.phase = Phase::Fraction;
                } else {
                    self.start_offset(byte)?;
                }
            }
            Phase::Fraction => {
                if byte.is_ascii_digit() {
                    if self.fraction_digits < 9 {
                        self.nanosecond = self.nanosecond * 10 + u32::from(byte - b'0');
                        self.fraction_digits += 1;
                    }
                } else {
                    if self.fraction_digits == 0 {
                        return Err(ParseError::Invalid);
                    }
                    self.pad_fraction();
                    self.start_offset(byte)?;
                }
            }
            Phase::OffsetHour => {
                if let Some(value) = self.digit_component(byte, 2)? {
                    self.offset_hour = value as u8;
                    self.phase = Phase::OffsetColon;
                }
            }
            Phase::OffsetColon => {
                self.expect(byte, b':', Phase::OffsetMinute)?;
            }
            Phase::OffsetMinute => {
                if let Some(value) = self.digit_component(byte, 2)? {
                    self.offset_minute = value as u8;
                    self.phase = Phase::OffsetMaybeSeconds;
                }
            }
            Phase::OffsetMaybeSeconds => {
                if byte == b':' {
                    self.phase = Phase::OffsetSecond;
                } else {
                    self.finish_offset()?;
                    return Err(ParseError::Trailing);
                }
            }
            Phase::OffsetSecond => {
                if let Some(value) = self.digit_component(byte, 2)? {
                    self.offset_second = value as u8;
                    self.finish_offset()?;
                }
            }
            Phase::Complete => return Err(ParseError::Trailing),
        }
        Ok(())
    }

    fn finish_inner(&mut self) -> Result<OffsetDateTime, ParseError> {
        if self.phase == Phase::OffsetMaybeSeconds {
            self.finish_offset()?;
        }
        if self.phase != Phase::Complete {
            return Err(ParseError::Invalid);
        }

        let date = Date::from_calendar_date(
            self.year,
            Month::try_from(self.month).map_err(|_| ParseError::OutOfRange)?,
            self.day,
        )
        .map_err(|_| ParseError::OutOfRange)?;
        let time = Time::from_hms_nano(
            self.hour,
            self.minute,
            self.second,
            self.nanosecond,
        )
        .map_err(|_| ParseError::OutOfRange)?;
        Ok(OffsetDateTime::new_in_offset(date, time, self.offset))
    }

    fn digit_component(&mut self, byte: u8, width: u8) -> Result<Option<u64>, ParseError> {
        if !byte.is_ascii_digit() {
            return Err(ParseError::Invalid);
        }
        self.component_value = self.component_value * 10 + u64::from(byte - b'0');
        self.component_digits += 1;
        if self.component_digits == width {
            let value = self.component_value;
            self.component_value = 0;
            self.component_digits = 0;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    fn expect(&mut self, byte: u8, expected: u8, next: Phase) -> Result<(), ParseError> {
        if byte != expected {
            return Err(ParseError::Invalid);
        }
        self.phase = next;
        Ok(())
    }

    fn start_offset(&mut self, byte: u8) -> Result<(), ParseError> {
        match byte {
            b'Z' | b'z' => {
                self.offset = UtcOffset::UTC;
                self.phase = Phase::Complete;
                Ok(())
            }
            b'+' | b'-' => {
                self.offset_sign = if byte == b'+' { 1 } else { -1 };
                self.phase = Phase::OffsetHour;
                Ok(())
            }
            _ => Err(ParseError::Invalid),
        }
    }

    fn pad_fraction(&mut self) {
        const SCALE: [u32; 10] = [
            1_000_000_000,
            100_000_000,
            10_000_000,
            1_000_000,
            100_000,
            10_000,
            1_000,
            100,
            10,
            1,
        ];
        self.nanosecond *= SCALE[usize::from(self.fraction_digits)];
    }

    fn finish_offset(&mut self) -> Result<(), ParseError> {
        let signed = |value: u8| -> i8 { (value as i8) * self.offset_sign };
        self.offset = UtcOffset::from_hms(
            signed(self.offset_hour),
            signed(self.offset_minute),
            signed(self.offset_second),
        )
        .map_err(|_| ParseError::OutOfRange)?;
        self.phase = Phase::Complete;
        Ok(())
    }
}
