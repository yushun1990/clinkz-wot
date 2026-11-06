//! Concrete JSON decoding, with the type assertion evaluated by Core. No TD
//! or Planning dependency; no JSON tree or normalized body copy is constructed.
use clinkz_wot_core::{
    CoreError, CoreResult, ErrorContext, ErrorPhase, InteractionOutput, RetryClass,
    data_schema::{PreparedSchema, ValueType},
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
        f.write_str("a described eligible Property name")
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
    validate_payload(&PreparedSchema::Type(ValueType::Array), payload)?;
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
    buffer.0.try_reserve_exact(4096).map_err(|_| {
        CoreError::Backpressure(ErrorContext::new(ErrorPhase::Codec, RetryClass::Never))
    })?;
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

/// One object containing every member of this role-specific schema exactly once.
pub fn validate_property_map(
    schema: &PropertyCollectionSchema,
    payload: &Payload,
) -> CoreResult<()> {
    validate_selected_property_map(schema, schema.all(), payload)
}

/// Validate a selected writable map and derive its set in the same pass. Every
/// supplied member must be eligible and schema-valid; omitted members are not
/// writes. Duplicate decoded names (including escapes) refuse the whole input.
pub fn validate_property_write_map(
    schema: &PropertyCollectionSchema,
    payload: &Payload,
) -> CoreResult<PropertySelection> {
    property_map(schema, None, payload)
}

/// Validate precisely the original requested subset and each original value.
/// Unrequested, missing, duplicate and invalid members refuse the whole result.
pub fn validate_selected_property_map(
    schema: &PropertyCollectionSchema,
    selected: PropertySelection,
    payload: &Payload,
) -> CoreResult<()> {
    property_map(schema, Some(selected), payload).map(|_| ())
}

fn property_map(
    schema: &PropertyCollectionSchema,
    selected: Option<PropertySelection>,
    payload: &Payload,
) -> CoreResult<PropertySelection> {
    if let Some(selected) = selected {
        schema.selection_from_bits(selected.bits())?;
    }
    validate_payload(&PreparedSchema::Type(ValueType::Object), payload)?;
    struct Members<'a>(&'a PropertyCollectionSchema, Option<PropertySelection>);
    impl<'de> Visitor<'de> for Members<'_> {
        type Value = PropertySelection;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a complete selected Property collection")
        }
        fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
            use serde::de::Error;
            let mut seen = 0u32;
            while let Some(i) = map.next_key_seed(MemberIndex(self.0))? {
                let bit = 1u32 << i;
                if seen & bit != 0 || self.1.is_some_and(|s| !s.contains(i)) {
                    return Err(M::Error::custom(
                        "duplicate or unrequested collection member",
                    ));
                }
                let value: &serde_json::value::RawValue = map.next_value()?;
                validate_json(&self.0.members()[i].schema, value.get().as_bytes())
                    .map_err(|_| M::Error::custom("invalid collection member"))?;
                seen |= bit;
            }
            if self.1.is_some_and(|s| seen != s.bits()) {
                return Err(M::Error::custom("incomplete collection map"));
            }
            self.0
                .selection_from_bits(seen)
                .map_err(|_| M::Error::custom("invalid member set"))
        }
    }
    let mut decoder = serde_json::Deserializer::from_slice(&payload.body);
    let selected = serde::Deserializer::deserialize_map(&mut decoder, Members(schema, selected))
        .map_err(|_| invalid())?;
    decoder.end().map_err(|_| invalid())?;
    Ok(selected)
}

pub fn validate(schema: &PreparedSchema, output: &InteractionOutput) -> CoreResult<()> {
    let payload = output.data().ok_or_else(invalid)?;
    validate_payload(schema, payload)
}

pub fn validate_payload(
    schema: &PreparedSchema,
    payload: &clinkz_wot_core::Payload,
) -> CoreResult<()> {
    if payload.body.len() > 4096
        || payload.content_type != "application/json"
        || payload.content_coding.is_some()
    {
        return Err(invalid());
    }
    validate_json(schema, &payload.body)
}

pub fn validate_json(schema: &PreparedSchema, body: &[u8]) -> CoreResult<()> {
    if body.len() > 4096 {
        return Err(invalid());
    }
    let json = std::str::from_utf8(body).map_err(|_| invalid())?;
    // IgnoredAny skips UTF-8 checks inside strings. The check above covers the
    // complete body, including object keys and nested values.
    let root: &serde_json::value::RawValue = serde_json::from_str(json).map_err(|_| invalid())?;
    if !unicode_escapes(json.as_bytes()) || !bounded_depth(json.as_bytes()) {
        return Err(invalid());
    }
    schema
        .validate(&JsonValues { _source: json }, root)
        .map_err(|_| invalid())
}

use clinkz_wot_core::data_schema::{NumberValue, SchemaViolation, ValueAccess};
struct JsonValues<'a> {
    _source: &'a str,
}
impl<'a> ValueAccess for JsonValues<'a> {
    type Node = &'a serde_json::value::RawValue;
    fn kind(&self, node: Self::Node) -> Result<ValueType, SchemaViolation> {
        let token = node.get();
        Ok(match token.as_bytes().first() {
            Some(b'n') => ValueType::Null,
            Some(b't' | b'f') => ValueType::Boolean,
            Some(b'"') => ValueType::String,
            Some(b'{') => ValueType::Object,
            Some(b'[') => ValueType::Array,
            Some(b'-' | b'0'..=b'9') if NumberValue::Decimal(token).is_integer() => {
                ValueType::Integer
            }
            Some(b'-' | b'0'..=b'9') => ValueType::Number,
            _ => return Err(SchemaViolation::Value),
        })
    }
    fn number(&self, node: Self::Node) -> Result<NumberValue<'_>, SchemaViolation> {
        Ok(NumberValue::Decimal(node.get()))
    }
    fn string_length(&self, node: Self::Node) -> Result<usize, SchemaViolation> {
        let value: std::string::String =
            serde_json::from_str(node.get()).map_err(|_| SchemaViolation::Value)?;
        Ok(value.chars().count())
    }
    fn property(
        &self,
        node: Self::Node,
        name: &str,
    ) -> Result<Option<Self::Node>, SchemaViolation> {
        struct Find<'n>(&'n str);
        impl<'de> Visitor<'de> for Find<'_> {
            type Value = Option<&'de serde_json::value::RawValue>;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("an object")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                use serde::de::Error;
                let mut found = None;
                while let Some(key) = map.next_key::<std::borrow::Cow<'de, str>>()? {
                    let value: &'de serde_json::value::RawValue = map.next_value()?;
                    if key == self.0 {
                        if found.is_some() {
                            return Err(M::Error::custom("duplicate described field"));
                        }
                        found = Some(value);
                    }
                }
                Ok(found)
            }
        }
        let mut decoder = serde_json::Deserializer::from_str(node.get());
        serde::Deserializer::deserialize_map(&mut decoder, Find(name))
            .map_err(|_| SchemaViolation::Value)
    }
}
// Each callback is atomic under a prepaid bound. Also bound ignored/unknown
// subtrees: a shallow schema does not authorize unbounded encoded nesting.
fn bounded_depth(bytes: &[u8]) -> bool {
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for b in bytes {
        if quoted {
            if escaped {
                escaped = false;
            } else if *b == b'\\' {
                escaped = true;
            } else if *b == b'"' {
                quoted = false;
            }
        } else {
            match b {
                b'"' => quoted = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > 32 {
                        return false;
                    }
                }
                b'}' | b']' => depth -= 1,
                _ => {}
            }
        }
    }
    true
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

fn invalid() -> CoreError {
    CoreError::Validation(
        ErrorContext::new(ErrorPhase::Validate, RetryClass::Never)
            .with_redacted_cause(625, "JSON payload violates prepared schema or encoding"),
    )
}
