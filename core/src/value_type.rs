//! A source-independent type-only validation subset, not a complete DataSchema
//! representation. Codecs establish the actual value kind; preparation must
//! refuse every assertion this subset cannot represent.

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

/// Prepared type assertion, independent of payload encoding and source TD.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeSchema {
    Any,
    Type(ValueType),
}

impl TypeSchema {
    /// `Integer` is a mathematical value kind, including integral decimal and
    /// exponent encodings. A codec must not infer it from machine storage type.
    pub fn accepts(self, actual: ValueType) -> bool {
        match self {
            Self::Any => true,
            Self::Type(ValueType::Number) => {
                matches!(actual, ValueType::Number | ValueType::Integer)
            }
            Self::Type(expected) => expected == actual,
        }
    }
}
