//! Shared scalar URI-variable meaning and a bounded RFC 6570 subset. No
//! protocol parsing, destination policy, matching, routing or I/O lives here.
use crate::data_schema::{NumberValue, PreparedSchema, SchemaViolation, ValueAccess, ValueType};
use crate::{CoreError, CoreResult, ErrorContext, ErrorPhase, RetryClass};
use alloc::{boxed::Box, string::String, vec::Vec};

pub const MAX_VARIABLES: usize = 8;
pub const MAX_NAME_BYTES: usize = 32;
pub const MAX_VALUE_BYTES: usize = 128;
pub const MAX_TEMPLATE_BYTES: usize = 256;

#[derive(Debug, Eq, PartialEq)]
pub struct UriVariable {
    pub name: Box<str>,
    pub schema: PreparedSchema,
    /// Referenced by the selected Form. Declarations alone supply no values.
    pub referenced: bool,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct PreparedUriVariables {
    declarations: Vec<UriVariable>,
}
impl PreparedUriVariables {
    pub fn try_new(declarations: Vec<UriVariable>) -> CoreResult<Self> {
        if declarations.len() > MAX_VARIABLES
            || declarations.iter().enumerate().any(|(i, v)| {
                !variable_name(&v.name)
                    || !scalar_schema(&v.schema)
                    || declarations[..i].iter().any(|old| old.name == v.name)
            })
        {
            return Err(refused());
        }
        Ok(Self { declarations })
    }
    pub fn declarations(&self) -> &[UriVariable] {
        &self.declarations
    }
    pub fn is_dynamic(&self) -> bool {
        self.declarations.iter().any(|v| v.referenced)
    }
    pub fn retained_bytes(&self) -> usize {
        self.declarations.capacity() * core::mem::size_of::<UriVariable>()
            + self
                .declarations
                .iter()
                .map(|v| v.name.len() + v.schema.retained_bytes())
                .sum::<usize>()
    }
    pub fn drop_work(&self) -> u64 {
        self.declarations.len() as u64
    }
    /// Values are unescaped scalar lexical strings, not encoded URI fragments
    /// or JSON string literals. Integers use canonical base-ten i64 spelling;
    /// booleans use true/false. All constraints use shared PreparedSchema logic.
    pub fn validate(&self, values: &[(&str, &str)]) -> CoreResult<()> {
        if values.len() > MAX_VARIABLES {
            return Err(invalid());
        }
        for (i, (name, value)) in values.iter().enumerate() {
            if name.len() > MAX_NAME_BYTES
                || value.len() > MAX_VALUE_BYTES
                || values[..i].iter().any(|(n, _)| n == name)
            {
                return Err(invalid());
            }
            let v = self
                .declarations
                .iter()
                .find(|v| v.name.as_ref() == *name && v.referenced)
                .ok_or_else(invalid)?;
            let kind = match &v.schema {
                PreparedSchema::String { .. } | PreparedSchema::Type(ValueType::String) => {
                    ValueType::String
                }
                PreparedSchema::Integer { .. } | PreparedSchema::Type(ValueType::Integer) => {
                    let n = value.parse::<i64>().map_err(|_| invalid())?;
                    // Reject plus signs/leading zeroes/whitespace which have no
                    // canonical scalar representation in this supported slice.
                    let digits = value.strip_prefix('-').unwrap_or(value);
                    if digits.is_empty()
                        || !digits.bytes().all(|b| b.is_ascii_digit())
                        || (digits.len() > 1 && digits.starts_with('0'))
                        || (*value == "-0")
                    {
                        return Err(invalid());
                    }
                    v.schema
                        .validate(
                            &Scalar {
                                kind: ValueType::Integer,
                                text: value,
                                integer: n,
                            },
                            (),
                        )
                        .map_err(|_| invalid())?;
                    continue;
                }
                PreparedSchema::Type(ValueType::Boolean) if matches!(*value, "true" | "false") => {
                    ValueType::Boolean
                }
                _ => return Err(invalid()),
            };
            v.schema
                .validate(
                    &Scalar {
                        kind,
                        text: value,
                        integer: 0,
                    },
                    (),
                )
                .map_err(|_| invalid())?;
        }
        if self
            .declarations
            .iter()
            .any(|v| v.referenced && !values.iter().any(|(n, _)| *n == v.name.as_ref()))
        {
            return Err(invalid());
        }
        Ok(())
    }
}
pub fn scalar_schema(s: &PreparedSchema) -> bool {
    matches!(
        s,
        PreparedSchema::Type(ValueType::String | ValueType::Integer | ValueType::Boolean)
            | PreparedSchema::String { .. }
            | PreparedSchema::Integer { .. }
    )
}
struct Scalar<'a> {
    kind: ValueType,
    text: &'a str,
    integer: i64,
}
impl ValueAccess for Scalar<'_> {
    type Node = ();
    fn kind(&self, _: ()) -> Result<ValueType, SchemaViolation> {
        Ok(self.kind)
    }
    fn number(&self, _: ()) -> Result<NumberValue<'_>, SchemaViolation> {
        Ok(NumberValue::Signed(self.integer))
    }
    fn string_length(&self, _: ()) -> Result<usize, SchemaViolation> {
        Ok(self.text.chars().count())
    }
    fn property(&self, _: (), _: &str) -> Result<Option<()>, SchemaViolation> {
        Err(SchemaViolation::Value)
    }
}

/// RFC 6570 simple string and form-style query/query-continuation expressions.
/// Literals are ASCII and names are bounded ASCII alphanumeric/underscore. Reserved expansion,
/// fragments, modifiers, composites and other operators explicitly refuse.
#[derive(Clone, Copy)]
pub struct UriTemplate<'a>(&'a str);
impl<'a> UriTemplate<'a> {
    pub fn parse(text: &'a str) -> CoreResult<Self> {
        if text.len() > MAX_TEMPLATE_BYTES {
            return Err(refused());
        }
        let mut rest = text;
        while let Some(start) = rest.find('{') {
            literal(&rest[..start])?;
            let (expression, tail) = rest[start + 1..].split_once('}').ok_or_else(refused)?;
            let (op, names) = expression_parts(expression);
            if names.is_empty()
                || (op == 0 && names.contains(','))
                || names.split(',').any(|name| !variable_name(name))
                || names.split(',').count() > MAX_VARIABLES
            {
                return Err(refused());
            }
            rest = tail;
        }
        literal(rest)?;
        Ok(Self(text))
    }
    pub fn references(self) -> impl Iterator<Item = &'a str> {
        self.0.split('{').skip(1).flat_map(|s| {
            let expression = s.split_once('}').unwrap().0;
            expression_parts(expression).1.split(',')
        })
    }
    /// Pure expansion utility for native Bindings. Capacity is the actual
    /// Binding's destination bound, not a universal address-size contract.
    pub fn expand<'v>(
        self,
        mut value: impl FnMut(&str) -> Option<&'v str>,
        capacity: usize,
    ) -> CoreResult<Box<str>> {
        let mut result = String::new();
        result.try_reserve_exact(capacity).map_err(|_| invalid())?;
        let mut rest = self.0;
        while let Some(start) = rest.find('{') {
            append(&mut result, &rest[..start], capacity)?;
            let (expression, tail) = rest[start + 1..].split_once('}').unwrap();
            let (op, names) = expression_parts(expression);
            for (i, name) in names.split(',').enumerate() {
                let text = value(name).ok_or_else(invalid)?;
                if text.len() > MAX_VALUE_BYTES {
                    return Err(invalid());
                }
                if op != 0 {
                    append(
                        &mut result,
                        if i == 0 && op == b'?' { "?" } else { "&" },
                        capacity,
                    )?;
                    append(&mut result, name, capacity)?;
                    append(&mut result, "=", capacity)?;
                }
                // RFC 6570 simple/query expansion encodes UTF-8 octets outside
                // unreserved. A supplied percent sign is data, never an escape.
                for b in text.bytes() {
                    if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                        append(&mut result, core::str::from_utf8(&[b]).unwrap(), capacity)?;
                    } else {
                        let hex = b"0123456789ABCDEF";
                        let encoded = [b'%', hex[(b >> 4) as usize], hex[(b & 15) as usize]];
                        append(
                            &mut result,
                            core::str::from_utf8(&encoded).unwrap(),
                            capacity,
                        )?;
                    }
                }
            }
            rest = tail;
        }
        append(&mut result, rest, capacity)?;
        Ok(result.into_boxed_str())
    }
}
fn expression_parts(expression: &str) -> (u8, &str) {
    match expression.as_bytes().first() {
        Some(b'?' | b'&') => (expression.as_bytes()[0], &expression[1..]),
        _ => (0, expression),
    }
}
pub fn variable_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_NAME_BYTES
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
fn literal(text: &str) -> CoreResult<()> {
    let bytes = text.as_bytes();
    for (i, b) in bytes.iter().copied().enumerate() {
        if b <= 32
            || b >= 127
            || b"{}<>\"\\^`|".contains(&b)
            || (b == b'%'
                && (i + 2 >= bytes.len()
                    || !bytes[i + 1].is_ascii_hexdigit()
                    || !bytes[i + 2].is_ascii_hexdigit()))
        {
            return Err(refused());
        }
    }
    Ok(())
}
fn append(out: &mut String, text: &str, capacity: usize) -> CoreResult<()> {
    if out
        .len()
        .checked_add(text.len())
        .is_none_or(|n| n > capacity)
    {
        return Err(invalid());
    }
    out.push_str(text);
    Ok(())
}
fn invalid() -> CoreError {
    CoreError::Validation(ErrorContext::new(ErrorPhase::Validate, RetryClass::Never))
}
fn refused() -> CoreError {
    CoreError::UnsupportedOperation(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rfc_simple_and_query_expansion_encode_utf8_and_percent_as_data() {
        let template = UriTemplate::parse("{hello}/find{?x,y}{&empty}").unwrap();
        let values = [
            ("hello", "Hello World!"),
            ("x", "1024"),
            ("y", "雪/%20"),
            ("empty", ""),
        ];
        let result = template
            .expand(
                |n| values.iter().find(|(name, _)| *name == n).map(|(_, v)| *v),
                256,
            )
            .unwrap();
        assert_eq!(
            &*result,
            "Hello%20World%21/find?x=1024&y=%E9%9B%AA%2F%2520&empty="
        );
        assert!(template.expand(|_| None, 256).is_err());
        assert!(template.expand(|_| Some("long"), 2).is_err());
        for text in [
            "{x", "x}", "{+x}", "{x*}", "{x:2}", "{?x,,y}", "%xx", "{}", "é/{x}",
        ] {
            assert!(UriTemplate::parse(text).is_err(), "{text}");
        }
    }
    #[test]
    fn scalar_values_apply_shared_constraints_before_native_expansion() {
        let schemas = PreparedUriVariables::try_new(alloc::vec![
            UriVariable {
                name: "n".into(),
                schema: PreparedSchema::Integer {
                    minimum: Some(-2),
                    exclusive_minimum: None,
                    maximum: None,
                    exclusive_maximum: Some(3)
                },
                referenced: true
            },
            UriVariable {
                name: "s".into(),
                schema: PreparedSchema::String {
                    min_length: Some(2),
                    max_length: Some(3)
                },
                referenced: true
            },
            UriVariable {
                name: "b".into(),
                schema: PreparedSchema::Type(ValueType::Boolean),
                referenced: true
            },
        ])
        .unwrap();
        assert!(
            schemas
                .validate(&[("n", "-2"), ("s", "雪☃"), ("b", "true")])
                .is_ok()
        );
        for n in [
            "3",
            "-3",
            "+1",
            "01",
            "-0",
            "1.0",
            " 1",
            "9223372036854775808",
        ] {
            assert!(
                schemas
                    .validate(&[("n", n), ("s", "雪☃"), ("b", "true")])
                    .is_err(),
                "{n}"
            );
        }
        assert!(
            schemas
                .validate(&[("n", "1"), ("s", "雪"), ("b", "true")])
                .is_err()
        );
        assert!(
            schemas
                .validate(&[("n", "1"), ("s", "雪☃"), ("b", "1")])
                .is_err()
        );
        assert!(schemas.validate(&[("n", "1"), ("s", "雪☃")]).is_err());
    }
}
