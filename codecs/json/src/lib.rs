//! Concrete JSON decoding, with prepared constraints evaluated by Core. No TD
//! or Planning dependency; no JSON tree or normalized body copy is constructed.
use clinkz_wot_core::{
    CoreError, CoreResult, ErrorContext, ErrorPhase, InteractionOutput, RetryClass,
    data_schema::{PreparedSchema, ValueType},
};

use clinkz_wot_core::data_schema::{NumberValue, SchemaViolation, ValueAccess};
use clinkz_wot_core::{
    Payload,
    collection::{PropertyCollectionSchema, PropertySelection},
};
use justjson::{
    JsonString,
    parser::{ParseConfig, Parser, Token, Tokenizer},
};

fn member_bit(
    schema: &PropertyCollectionSchema,
    name: &JsonString<'_>,
    seen: u32,
) -> CoreResult<(usize, u32)> {
    let index = schema
        .members()
        .iter()
        .position(|member| *name == member.name.as_ref())
        .ok_or_else(invalid)?;
    let bit = 1u32 << index;
    if seen & bit != 0 {
        return Err(invalid());
    }
    Ok((index, bit))
}

/// Default TD 1.1 request representation: an array of exact Property names.
/// Decode against the immutable readable set, including escaped duplicates.
/// No request tree, owned name list or source TD is constructed.
pub fn decode_property_selection(
    schema: &PropertyCollectionSchema,
    payload: &Payload,
) -> CoreResult<PropertySelection> {
    validate_payload(&PreparedSchema::Type(ValueType::Array), payload)?;
    let json = std::str::from_utf8(&payload.body).map_err(|_| invalid())?;
    let mut bits = 0u32;
    let mut tokens = Tokenizer::for_json(json);
    tokens.next(); // validated root array
    for token in tokens {
        match token.map_err(|_| invalid())? {
            Token::Comma | Token::ArrayEnd => {}
            Token::String(name) => bits |= member_bit(schema, &name, bits)?.1,
            _ => return Err(invalid()),
        }
    }
    schema.selection_from_bits(bits)
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
    let json = std::str::from_utf8(&payload.body).map_err(|_| invalid())?;
    let mut seen = 0u32;
    fields(json, |name, value| {
        let (i, bit) = member_bit(schema, &name, seen)?;
        if selected.is_some_and(|s| !s.contains(i)) {
            return Err(invalid());
        }
        JsonValues(value).validate(&schema.members()[i].schema)?;
        seen |= bit;
        Ok(())
    })?;
    if selected.is_some_and(|s| s.bits() != seen) {
        return Err(invalid());
    }
    schema.selection_from_bits(seen)
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
    // Syntax, UTF-8, scalar escapes and nesting are checked before lending
    // any subtree. The borrowed decoder uses bounded stack throughout,
    // including escaped field names and length-constrained strings.
    Parser::validate_json_with_config(json, ParseConfig::new().with_recursion_limit(32))
        .map_err(|_| invalid())?;
    JsonValues(json.trim()).validate(schema)
}

struct JsonValues<'a>(&'a str);
impl JsonValues<'_> {
    fn validate(&self, schema: &PreparedSchema) -> CoreResult<()> {
        schema.validate(self, self.0).map_err(|_| invalid())
    }
}
impl<'a> ValueAccess for JsonValues<'a> {
    type Node = &'a str;
    fn kind(&self, node: Self::Node) -> Result<ValueType, SchemaViolation> {
        Ok(match node.as_bytes().first() {
            Some(b'n') => ValueType::Null,
            Some(b't' | b'f') => ValueType::Boolean,
            Some(b'"') => ValueType::String,
            Some(b'{') => ValueType::Object,
            Some(b'[') => ValueType::Array,
            Some(b'-' | b'0'..=b'9') if NumberValue::Decimal(node).is_integer() => {
                ValueType::Integer
            }
            Some(b'-' | b'0'..=b'9') => ValueType::Number,
            _ => return Err(SchemaViolation::Value),
        })
    }
    fn number(&self, node: Self::Node) -> Result<NumberValue<'_>, SchemaViolation> {
        Ok(NumberValue::Decimal(node))
    }
    fn string_length(&self, node: Self::Node) -> Result<usize, SchemaViolation> {
        JsonString::from_json(node)
            .map(|s| s.decoded().count())
            .map_err(|_| SchemaViolation::Value)
    }
    fn property(
        &self,
        node: Self::Node,
        name: &str,
    ) -> Result<Option<Self::Node>, SchemaViolation> {
        let mut found = None;
        fields(node, |key, value| {
            if key == name && found.replace(value).is_some() {
                return Err(invalid());
            }
            Ok(())
        })
        .map_err(|_| SchemaViolation::Value)?;
        Ok(found)
    }
}

// Walk the immediate fields of an already syntax-checked object. Values
// remain slices of the original input, including exact decimal lexemes.
fn fields<'a>(
    object: &'a str,
    mut visit: impl FnMut(JsonString<'a>, &'a str) -> CoreResult<()>,
) -> CoreResult<()> {
    let mut tokens = Tokenizer::for_json(object);
    if tokens.next_or_eof().map_err(|_| invalid())? != Token::Object {
        return Err(invalid());
    }
    loop {
        let name = match tokens.next_or_eof().map_err(|_| invalid())? {
            Token::ObjectEnd => return Ok(()),
            Token::Comma => continue,
            Token::String(name) => name,
            _ => return Err(invalid()),
        };
        tokens.next_or_eof().map_err(|_| invalid())?; // colon
        let start = tokens.offset();
        let mut depth = 0;
        loop {
            match tokens.next_or_eof().map_err(|_| invalid())? {
                Token::Object | Token::Array => depth += 1,
                Token::ObjectEnd | Token::ArrayEnd => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                break;
            }
        }
        visit(name, object[start..tokens.offset()].trim_start())?;
    }
}

fn invalid() -> CoreError {
    CoreError::Validation(
        ErrorContext::new(ErrorPhase::Validate, RetryClass::Never)
            .with_redacted_cause(625, "JSON payload violates prepared schema or encoding"),
    )
}
