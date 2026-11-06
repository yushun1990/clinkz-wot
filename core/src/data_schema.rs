//! Source-independent DataSchema meaning and evaluation. No encoding, TD,
//! std, pointer atomics or execution authority belongs in this representation.
use alloc::{boxed::Box, vec::Vec};
use core::cmp::Ordering;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueType {
    Null,
    Boolean,
    Number,
    Integer,
    String,
    Object,
    Array,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PreparedSchema {
    Any,
    Type(ValueType),
    Integer {
        minimum: Option<i64>,
        exclusive_minimum: Option<i64>,
        maximum: Option<i64>,
        exclusive_maximum: Option<i64>,
    },
    String {
        min_length: Option<u32>,
        max_length: Option<u32>,
    },
    Object(ObjectSchema),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaProperty {
    pub name: Box<str>,
    pub required: bool,
    pub schema: PreparedSchema,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectSchema {
    properties: Vec<SchemaProperty>,
}
impl ObjectSchema {
    pub fn properties(&self) -> &[SchemaProperty] {
        &self.properties
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchemaViolation {
    Value,
    Capacity,
}

/// Codec-owned handles lend values; they do not transfer a payload, allocate a
/// semantic tree, or choose constraint meaning. The codec bounds syntax, depth,
/// duplicate-name handling and the complete cost of these passive operations.
pub trait ValueAccess {
    type Node: Copy;
    fn kind(&self, node: Self::Node) -> Result<ValueType, SchemaViolation>;
    fn number(&self, node: Self::Node) -> Result<NumberValue<'_>, SchemaViolation>;
    /// Unicode code points, independent of encoded bytes or grapheme clusters.
    fn string_length(&self, node: Self::Node) -> Result<usize, SchemaViolation>;
    fn property(&self, node: Self::Node, name: &str)
    -> Result<Option<Self::Node>, SchemaViolation>;
}

/// Exact numeric evidence supplied by an encoding implementation. A decimal is
/// a syntax-checked finite base-ten numeral, not a binary float approximation.
#[derive(Clone, Copy, Debug)]
pub enum NumberValue<'a> {
    Signed(i64),
    Unsigned(u64),
    Decimal(&'a str),
}
impl NumberValue<'_> {
    pub fn is_integer(self) -> bool {
        self.integer().is_some()
    }
    pub fn compare_integer(self, bound: i64) -> Result<Ordering, SchemaViolation> {
        let (negative, magnitude) = self.integer().ok_or(SchemaViolation::Value)?;
        let bound_negative = bound < 0;
        if negative != bound_negative {
            return Ok(if negative {
                Ordering::Less
            } else {
                Ordering::Greater
            });
        }
        let order = magnitude.map_or(Ordering::Greater, |n| {
            n.cmp(&(bound.unsigned_abs() as u128))
        });
        Ok(if negative { order.reverse() } else { order })
    }
    // None magnitude denotes a nonzero integer larger than any u64. Decimal
    // exponents saturate only beyond the token length plus a comparison margin;
    // that cannot change integrality or comparison with an i64 bound.
    fn integer(self) -> Option<(bool, Option<u128>)> {
        match self {
            Self::Signed(n) => Some((n < 0, Some(n.unsigned_abs() as u128))),
            Self::Unsigned(n) => Some((false, Some(n as u128))),
            Self::Decimal(token) => decimal_integer(token),
        }
    }
}

fn decimal_integer(token: &str) -> Option<(bool, Option<u128>)> {
    let limit = i64::try_from(token.len()).ok()?.saturating_add(64);
    let (mantissa, exponent) = token.split_once(['e', 'E']).unwrap_or((token, "0"));
    let exp_negative = exponent.starts_with('-');
    let exp_digits = exponent.trim_start_matches(['+', '-']);
    if exp_digits.is_empty() || !exp_digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let magnitude = exp_digits.bytes().fold(0i64, |n, b| {
        n.saturating_mul(10)
            .saturating_add((b - b'0') as i64)
            .min(limit)
    });
    let exponent = if exp_negative { -magnitude } else { magnitude };
    let negative = mantissa.starts_with('-');
    let digits = mantissa.strip_prefix('-').unwrap_or(mantissa);
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let mut significant = 0i64;
    let mut trailing = 0i64;
    for b in whole.bytes().chain(fraction.bytes()) {
        if significant != 0 || b != b'0' {
            significant += 1;
        }
        trailing = if b == b'0' { trailing + 1 } else { 0 };
    }
    if significant == 0 {
        return Some((false, Some(0)));
    }
    let shift = exponent - fraction.len() as i64;
    if shift + trailing < 0 {
        return None;
    }
    let length = significant + shift;
    if length > 20 {
        return Some((negative, None));
    }
    let mut value = 0u128;
    let mut taken = 0i64;
    for b in whole
        .bytes()
        .chain(fraction.bytes())
        .skip_while(|b| *b == b'0')
    {
        if taken == length {
            break;
        }
        value = value * 10 + (b - b'0') as u128;
        taken += 1;
    }
    while taken < length {
        value *= 10;
        taken += 1;
    }
    Some((negative, Some(value)))
}

impl PreparedSchema {
    pub const MAX_NODES: usize = 64;
    pub const MAX_DEPTH: usize = 8;
    pub const MAX_PROPERTIES: usize = 32;
    pub const MAX_NAME_BYTES: usize = 256;
    pub const MAX_TOTAL_NAME_BYTES: usize = 4096;
    pub const BACKING_BOUND: usize =
        Self::MAX_NODES * core::mem::size_of::<SchemaProperty>() + Self::MAX_TOTAL_NAME_BYTES;

    pub fn object(properties: Vec<SchemaProperty>) -> Result<Self, SchemaViolation> {
        if properties.len() > Self::MAX_PROPERTIES
            || properties.capacity() > Self::MAX_PROPERTIES
            || properties.iter().enumerate().any(|(i, p)| {
                p.name.len() > Self::MAX_NAME_BYTES
                    || properties[..i].iter().any(|old| old.name == p.name)
            })
        {
            return Err(SchemaViolation::Capacity);
        }
        let result = Self::Object(ObjectSchema { properties });
        if result.node_count() > Self::MAX_NODES
            || result.depth() > Self::MAX_DEPTH
            || result.name_bytes() > Self::MAX_TOTAL_NAME_BYTES
            || result.retained_bytes() > Self::BACKING_BOUND
        {
            return Err(SchemaViolation::Capacity);
        }
        Ok(result)
    }
    pub fn is_type_only(&self) -> bool {
        matches!(self, Self::Any | Self::Type(_))
    }
    pub fn accepts_kind(&self, actual: ValueType) -> bool {
        let expected = match self {
            Self::Any => return true,
            Self::Type(t) => *t,
            Self::Integer { .. } => ValueType::Integer,
            Self::String { .. } => ValueType::String,
            Self::Object(_) => ValueType::Object,
        };
        expected == actual || expected == ValueType::Number && actual == ValueType::Integer
    }
    pub fn drop_work(&self) -> u64 {
        if matches!(self, Self::Object(_)) {
            self.node_count() as u64 * 2
        } else {
            0
        }
    }
    pub fn node_count(&self) -> usize {
        match self {
            Self::Object(o) => {
                1 + o
                    .properties
                    .iter()
                    .map(|p| p.schema.node_count())
                    .sum::<usize>()
            }
            _ => 1,
        }
    }
    fn depth(&self) -> usize {
        match self {
            Self::Object(o) => {
                1 + o
                    .properties
                    .iter()
                    .map(|p| p.schema.depth())
                    .max()
                    .unwrap_or(0)
            }
            _ => 1,
        }
    }
    pub fn retained_bytes(&self) -> usize {
        match self {
            Self::Object(o) => {
                o.properties.capacity() * core::mem::size_of::<SchemaProperty>()
                    + o.properties
                        .iter()
                        .map(|p| p.name.len() + p.schema.retained_bytes())
                        .sum::<usize>()
            }
            _ => 0,
        }
    }
    /// Largest owned field-table or name allocation, excluding this inline
    /// schema value. Concrete shared owners account for their own wrapper.
    pub fn largest_allocation_bytes(&self) -> usize {
        match self {
            Self::Object(o) => o.properties.iter().fold(
                o.properties.capacity() * core::mem::size_of::<SchemaProperty>(),
                |largest, p| {
                    largest
                        .max(p.name.len())
                        .max(p.schema.largest_allocation_bytes())
                },
            ),
            _ => 0,
        }
    }
    fn name_bytes(&self) -> usize {
        match self {
            Self::Object(o) => o
                .properties
                .iter()
                .map(|p| p.name.len() + p.schema.name_bytes())
                .sum(),
            _ => 0,
        }
    }
    pub fn validate<A: ValueAccess>(
        &self,
        values: &A,
        node: A::Node,
    ) -> Result<(), SchemaViolation> {
        if !self.accepts_kind(values.kind(node)?) {
            return Err(SchemaViolation::Value);
        }
        let bad = match self {
            Self::Any | Self::Type(_) => false,
            Self::Integer {
                minimum,
                exclusive_minimum,
                maximum,
                exclusive_maximum,
            } => {
                let n = values.number(node)?;
                minimum.is_some_and(|b| n.compare_integer(b).is_ok_and(|c| c == Ordering::Less))
                    || exclusive_minimum
                        .is_some_and(|b| n.compare_integer(b).is_ok_and(|c| c != Ordering::Greater))
                    || maximum
                        .is_some_and(|b| n.compare_integer(b).is_ok_and(|c| c == Ordering::Greater))
                    || exclusive_maximum
                        .is_some_and(|b| n.compare_integer(b).is_ok_and(|c| c != Ordering::Less))
                    || !n.is_integer()
            }
            Self::String {
                min_length,
                max_length,
            } => {
                let length = values.string_length(node)? as u64;
                min_length.is_some_and(|n| length < (n as u64))
                    || max_length.is_some_and(|n| length > (n as u64))
            }
            Self::Object(o) => {
                for field in &o.properties {
                    match values.property(node, &field.name)? {
                        Some(child) => field.schema.validate(values, child)?,
                        None if field.required => return Err(SchemaViolation::Value),
                        None => {}
                    }
                }
                false
            }
        };
        if bad {
            Err(SchemaViolation::Value)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    // Native typed values, without JSON bytes, a decoder, std or an Arc in the
    // semantic interface. Other media can lend the same logical value kinds.
    struct NativeValues(i64);
    impl ValueAccess for NativeValues {
        type Node = u8;
        fn kind(&self, node: u8) -> Result<ValueType, SchemaViolation> {
            Ok(if node == 0 {
                ValueType::Object
            } else {
                ValueType::Integer
            })
        }
        fn number(&self, _: u8) -> Result<NumberValue<'_>, SchemaViolation> {
            Ok(NumberValue::Signed(self.0))
        }
        fn string_length(&self, _: u8) -> Result<usize, SchemaViolation> {
            Err(SchemaViolation::Value)
        }
        fn property(&self, node: u8, name: &str) -> Result<Option<u8>, SchemaViolation> {
            Ok((node == 0 && name == "level").then_some(1))
        }
    }
    #[test]
    fn prepared_semantics_evaluate_native_values_without_a_json_representation() {
        let schema = PreparedSchema::object(vec![SchemaProperty {
            name: "level".into(),
            required: true,
            schema: PreparedSchema::Integer {
                minimum: Some(1),
                maximum: Some(99),
                exclusive_minimum: None,
                exclusive_maximum: None,
            },
        }])
        .unwrap();
        assert_eq!(schema.validate(&NativeValues(42), 0), Ok(()));
        assert_eq!(
            schema.validate(&NativeValues(0), 0),
            Err(SchemaViolation::Value)
        );
        assert_eq!(
            schema.validate(&NativeValues(100), 0),
            Err(SchemaViolation::Value)
        );
    }
}
