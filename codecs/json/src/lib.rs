//! Concrete JSON decoding, with the type assertion evaluated by Core. No TD
//! or Planning dependency; no JSON tree or normalized body copy is constructed.
use clinkz_wot_core::{
    CoreError, CoreResult, ErrorContext, ErrorPhase, InteractionOutput, RetryClass,
    value_type::{TypeSchema, ValueType},
};

/// The HTTP JSON collection mapping: one object with every described readable
/// Property exactly once. Validate original member bytes, including exact
/// decimal integrality, without a JSON tree or normalized value copy.
pub fn validate_property_map(
    schema: &clinkz_wot_core::collection::PropertyCollectionSchema,
    payload: &clinkz_wot_core::Payload,
) -> CoreResult<()> {
    use serde::de::{DeserializeSeed, MapAccess, Visitor};
    validate_payload(TypeSchema::Type(ValueType::Object), payload)?;
    struct Key<'a>(&'a clinkz_wot_core::collection::PropertyCollectionSchema);
    impl<'de> DeserializeSeed<'de> for Key<'_> {
        type Value = usize;
        fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<usize, D::Error> {
            struct Name<'a>(&'a clinkz_wot_core::collection::PropertyCollectionSchema);
            impl Visitor<'_> for Name<'_> {
                type Value = usize;
                fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_str("a described readable Property")
                }
                fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<usize, E> {
                    self.0
                        .members()
                        .iter()
                        .position(|m| m.name.as_ref() == s)
                        .ok_or_else(|| E::custom("unexpected collection member"))
                }
            }
            d.deserialize_str(Name(self.0))
        }
    }
    struct Members<'a>(&'a clinkz_wot_core::collection::PropertyCollectionSchema);
    impl<'de> Visitor<'de> for Members<'_> {
        type Value = ();
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a complete Property collection")
        }
        fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<(), M::Error> {
            use serde::de::Error;
            let mut seen = 0u32;
            while let Some(i) = map.next_key_seed(Key(self.0))? {
                let bit = 1u32 << i;
                if seen & bit != 0 {
                    return Err(M::Error::custom("duplicate collection member"));
                }
                let value: &serde_json::value::RawValue = map.next_value()?;
                validate_json(self.0.members()[i].schema, value.get().as_bytes())
                    .map_err(|_| M::Error::custom("invalid collection member"))?;
                seen |= bit;
            }
            let expected = ((1u64 << self.0.members().len()) - 1) as u32;
            if seen != expected {
                return Err(M::Error::custom("incomplete collection response"));
            }
            Ok(())
        }
    }
    let mut decoder = serde_json::Deserializer::from_slice(&payload.body);
    serde::Deserializer::deserialize_map(&mut decoder, Members(schema)).map_err(|_| invalid())?;
    decoder.end().map_err(|_| invalid())
}

pub fn validate(schema: TypeSchema, output: &InteractionOutput) -> CoreResult<()> {
    let payload = output.data().ok_or_else(invalid)?;
    validate_payload(schema, payload)
}

pub fn validate_payload(schema: TypeSchema, payload: &clinkz_wot_core::Payload) -> CoreResult<()> {
    if payload.body.len() > 4096
        || payload.content_type != "application/json"
        || payload.content_coding.is_some()
    {
        return Err(invalid());
    }
    validate_json(schema, &payload.body)
}

pub fn validate_json(schema: TypeSchema, body: &[u8]) -> CoreResult<()> {
    if body.len() > 4096 {
        return Err(invalid());
    }
    let json = std::str::from_utf8(body).map_err(|_| invalid())?;
    // IgnoredAny skips UTF-8 checks inside strings. The check above covers the
    // complete body, including object keys and nested values.
    serde_json::from_str::<serde::de::IgnoredAny>(json).map_err(|_| invalid())?;
    if !unicode_escapes(json.as_bytes()) {
        return Err(invalid());
    }
    let token = json.trim_matches([' ', '\t', '\r', '\n']);
    let actual = match token.as_bytes().first() {
        Some(b'n') => ValueType::Null,
        Some(b't' | b'f') => ValueType::Boolean,
        Some(b'"') => ValueType::String,
        Some(b'{') => ValueType::Object,
        Some(b'[') => ValueType::Array,
        Some(b'-' | b'0'..=b'9') if integral(token) => ValueType::Integer,
        Some(b'-' | b'0'..=b'9') => ValueType::Number,
        _ => return Err(invalid()),
    };
    if schema.accepts(actual) {
        Ok(())
    } else {
        Err(invalid())
    }
}

// IgnoredAny also skips surrogate-pair validation. Check scalar Unicode in
// every string/key without decoding/copying the strings. JSON syntax was
// checked above, so each escape and four-digit code unit is already present.
fn unicode_escapes(mut bytes: &[u8]) -> bool {
    while let Some(index) = bytes.iter().position(|b| *b == b'\\') {
        bytes = &bytes[index + 1..];
        if bytes[0] != b'u' {
            bytes = &bytes[1..];
            continue;
        }
        let unit = code_unit(&bytes[1..5]);
        bytes = &bytes[5..];
        if (0xDC00..=0xDFFF).contains(&unit) {
            return false;
        }
        if (0xD800..=0xDBFF).contains(&unit) {
            if bytes.len() < 6
                || !bytes.starts_with(b"\\u")
                || !(0xDC00..=0xDFFF).contains(&code_unit(&bytes[2..6]))
            {
                return false;
            }
            bytes = &bytes[6..];
        }
    }
    true
}

fn code_unit(bytes: &[u8]) -> u16 {
    bytes.iter().fold(0, |unit, byte| {
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => unreachable!("JSON syntax checked"),
        };
        unit * 16 + u16::from(digit)
    })
}

// Exact decimal arithmetic on a previously syntax-checked token. Binary float
// conversion would misclassify small fractions and large integers. Exponents
// saturate outside the maximum body length, where further magnitude cannot
// affect integrality. Zero is integral for every exponent.
fn integral(token: &str) -> bool {
    let (mantissa, exponent) = token.split_once(['e', 'E']).unwrap_or((token, "0"));
    let negative = exponent.starts_with('-');
    let magnitude = exponent
        .trim_start_matches(['+', '-'])
        .bytes()
        .fold(0i32, |n, b| (n * 10 + i32::from(b - b'0')).min(4097));
    let exponent = if negative { -magnitude } else { magnitude };
    let mut fractional = 0i32;
    let mut trailing_zeroes = 0i32;
    let mut after_dot = false;
    let mut nonzero = false;
    for byte in mantissa.bytes() {
        match byte {
            b'.' => after_dot = true,
            b'0'..=b'9' => {
                fractional += i32::from(after_dot);
                if byte == b'0' {
                    trailing_zeroes += 1;
                } else {
                    trailing_zeroes = 0;
                    nonzero = true;
                }
            }
            _ => {} // A leading minus, already checked by the JSON decoder.
        }
    }
    !nonzero || exponent >= fractional - trailing_zeroes
}

fn invalid() -> CoreError {
    CoreError::Validation(
        ErrorContext::new(ErrorPhase::Validate, RetryClass::Never)
            .with_redacted_cause(625, "JSON payload violates prepared schema or encoding"),
    )
}
