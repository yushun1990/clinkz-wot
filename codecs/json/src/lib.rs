//! Concrete JSON decoding, with the type assertion evaluated by Core. No TD
//! or Planning dependency; no JSON tree or normalized body copy is constructed.
use clinkz_wot_core::{
    CoreError, CoreResult, ErrorContext, ErrorPhase, InteractionOutput, RetryClass,
    value_type::{TypeSchema, ValueType},
};

use clinkz_wot_core::{
    Payload,
    collection::{PropertyCollectionSchema, PropertySelection},
};
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};

struct MemberIndex<'a>(&'a PropertyCollectionSchema);
impl<'de> DeserializeSeed<'de> for MemberIndex<'_> {
    type Value = usize;
    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<usize, D::Error> {
        d.deserialize_str(self)
    }
}

impl Visitor<'_> for MemberIndex<'_> {
    type Value = usize;
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a described readable Property name")
    }
    fn visit_str<E: serde::de::Error>(self, name: &str) -> Result<usize, E> {
        self.0
            .members()
            .iter()
            .position(|m| m.name.as_ref() == name)
            .ok_or_else(|| E::custom("unexpected collection member"))
    }
}

/// Default TD 1.1 request representation: an array of exact Property names.
/// Decode against the immutable readable set, including escaped duplicates.
/// No request tree, owned name list or source TD is constructed.
pub fn decode_property_selection(
    schema: &PropertyCollectionSchema,
    payload: &Payload,
) -> CoreResult<PropertySelection> {
    validate_payload(TypeSchema::Type(ValueType::Array), payload)?;
    struct Names<'a>(&'a PropertyCollectionSchema);
    impl<'de> Visitor<'de> for Names<'_> {
        type Value = PropertySelection;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a Property name array")
        }
        fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> Result<Self::Value, S::Error> {
            use serde::de::Error;
            let mut bits = 0;
            let mut count = 0;
            while let Some(index) = seq.next_element_seed(MemberIndex(self.0))? {
                count += 1;
                let bit = 1u32 << index;
                if count > PropertyCollectionSchema::MAX_MEMBERS || bits & bit != 0 {
                    return Err(S::Error::custom("invalid or duplicate selection"));
                }
                bits |= bit;
            }
            self.0
                .selection_from_bits(bits)
                .map_err(|_| S::Error::custom("invalid selection"))
        }
    }
    let mut decoder = serde_json::Deserializer::from_slice(&payload.body);
    let selected =
        serde::Deserializer::deserialize_seq(&mut decoder, Names(schema)).map_err(|_| invalid())?;
    decoder.end().map_err(|_| invalid())?;
    Ok(selected)
}

/// Serialize the TD default name array under the existing 4096-byte envelope.
/// The writer refuses before growing past it, including escaping expansion.
pub fn encode_property_selection(
    schema: &PropertyCollectionSchema,
    selected: PropertySelection,
) -> CoreResult<Payload> {
    use serde::ser::{SerializeSeq, Serializer};
    schema.selection_from_bits(selected.bits())?;
    struct Bounded(Vec<u8>);
    impl std::io::Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > 4096 - self.0.len() {
                return Err(std::io::ErrorKind::InvalidData.into());
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut buffer = Bounded(Vec::new());
    buffer.0.try_reserve_exact(4096).map_err(|_| invalid())?;
    let mut encoder = serde_json::Serializer::new(&mut buffer);
    let mut seq = (&mut encoder)
        .serialize_seq(Some(selected.bits().count_ones() as usize))
        .map_err(|_| invalid())?;
    for (index, member) in schema.members().iter().enumerate() {
        if selected.contains(index) {
            seq.serialize_element(member.name.as_ref())
                .map_err(|_| invalid())?;
        }
    }
    seq.end().map_err(|_| invalid())?;
    Ok(Payload::new(buffer.0, "application/json"))
}

/// One object containing every readable Property exactly once.
pub fn validate_property_map(
    schema: &PropertyCollectionSchema,
    payload: &Payload,
) -> CoreResult<()> {
    validate_selected_property_map(schema, schema.all(), payload)
}

/// Validate precisely the original requested subset and each original value.
/// Unrequested, missing, duplicate and invalid members refuse the whole result.
pub fn validate_selected_property_map(
    schema: &PropertyCollectionSchema,
    selected: PropertySelection,
    payload: &Payload,
) -> CoreResult<()> {
    schema.selection_from_bits(selected.bits())?;
    validate_payload(TypeSchema::Type(ValueType::Object), payload)?;
    struct Members<'a>(&'a PropertyCollectionSchema, PropertySelection);
    impl<'de> Visitor<'de> for Members<'_> {
        type Value = ();
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a complete selected Property collection")
        }
        fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<(), M::Error> {
            use serde::de::Error;
            let mut seen = 0u32;
            while let Some(i) = map.next_key_seed(MemberIndex(self.0))? {
                let bit = 1u32 << i;
                if seen & bit != 0 || !self.1.contains(i) {
                    return Err(M::Error::custom(
                        "duplicate or unrequested collection member",
                    ));
                }
                let value: &serde_json::value::RawValue = map.next_value()?;
                validate_json(self.0.members()[i].schema, value.get().as_bytes())
                    .map_err(|_| M::Error::custom("invalid collection member"))?;
                seen |= bit;
            }
            if seen != self.1.bits() {
                return Err(M::Error::custom("incomplete collection response"));
            }
            Ok(())
        }
    }
    let mut decoder = serde_json::Deserializer::from_slice(&payload.body);
    serde::Deserializer::deserialize_map(&mut decoder, Members(schema, selected))
        .map_err(|_| invalid())?;
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
