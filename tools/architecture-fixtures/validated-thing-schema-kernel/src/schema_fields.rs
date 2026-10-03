//! One DataSchema field policy, above the adapters' JSON representation boundary.
//! Ordinary serde keeps its existing map/RawValue conversions. The arena adapter
//! supplies literal kinds. This semantic seam is synchronous, not admission.
use super::schema_kernel::SchemaKind;

// The candidate Metadata declaration and its literal reader expand these same
// rows. In particular nullable metadata is not conflated with flat::take's
// presence-only Option, and @type retains its existing one-or-many adapter.
macro_rules! metadata_fields {
    ($apply:ident) => {
        $apply! {
            tags, Tags, "@type", Vec<String>, Strings, strings_many,
                [#[serde_as(as = "Option<OneOrMany<_>>")]];
            title, Title, "title", String, Text, text, [];
            titles, Titles, "titles", MultiLanguage, Languages, languages, [];
            description, Description, "description", String, Text, text, [];
            descriptions, Descriptions, "descriptions", MultiLanguage, Languages, languages, [];
        }
    };
}
pub(crate) use metadata_fields;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Field {
    Tags,
    Title,
    Titles,
    Description,
    Descriptions,
    Const,
    Default,
    Unit,
    OneOf,
    Enum,
    ReadOnly,
    WriteOnly,
    Format,
    Type,
    Items,
    MinItems,
    MaxItems,
    Minimum,
    ExclusiveMinimum,
    Maximum,
    ExclusiveMaximum,
    MultipleOf,
    Properties,
    Required,
    MinLength,
    MaxLength,
    Pattern,
    ContentEncoding,
    ContentMediaType,
}

macro_rules! metadata_names {
    ($( $name:ident, $field:ident, $wire:literal, $ty:ty, $assoc:ident, $read:ident,
        [$($attribute:tt)*]; )*) => {
        fn metadata_name(field: Field) -> Option<&'static str> {
            match field { $(Field::$field => Some($wire),)* _ => None }
        }
    };
}
metadata_fields!(metadata_names);

impl Field {
    pub fn name(self) -> &'static str {
        if let Some(name) = metadata_name(self) {
            return name;
        }
        match self {
            Self::Const => "const",
            Self::Default => "default",
            Self::Unit => "unit",
            Self::OneOf => "oneOf",
            Self::Enum => "enum",
            Self::ReadOnly => "readOnly",
            Self::WriteOnly => "writeOnly",
            Self::Format => "format",
            Self::Type => "type",
            Self::Items => "items",
            Self::MinItems => "minItems",
            Self::MaxItems => "maxItems",
            Self::Minimum => "minimum",
            Self::ExclusiveMinimum => "exclusiveMinimum",
            Self::Maximum => "maximum",
            Self::ExclusiveMaximum => "exclusiveMaximum",
            Self::MultipleOf => "multipleOf",
            Self::Properties => "properties",
            Self::Required => "required",
            Self::MinLength => "minLength",
            Self::MaxLength => "maxLength",
            Self::Pattern => "pattern",
            Self::ContentEncoding => "contentEncoding",
            Self::ContentMediaType => "contentMediaType",
            _ => unreachable!(),
        }
    }
    pub const ALL: [Self; 29] = [
        Self::Tags,
        Self::Title,
        Self::Titles,
        Self::Description,
        Self::Descriptions,
        Self::Const,
        Self::Default,
        Self::Unit,
        Self::OneOf,
        Self::Enum,
        Self::ReadOnly,
        Self::WriteOnly,
        Self::Format,
        Self::Type,
        Self::Items,
        Self::MinItems,
        Self::MaxItems,
        Self::Minimum,
        Self::ExclusiveMinimum,
        Self::Maximum,
        Self::ExclusiveMaximum,
        Self::MultipleOf,
        Self::Properties,
        Self::Required,
        Self::MinLength,
        Self::MaxLength,
        Self::Pattern,
        Self::ContentEncoding,
        Self::ContentMediaType,
    ];
}

/// No adapter chooses field ownership, null/default policy or variant dispatch.
/// Its conversions supply representation facts and the established scalar types.
pub trait Source: Sized {
    type Value;
    type Text;
    type Values;
    type Strings;
    type Languages;
    type Schemas;
    type SchemaMap;
    type Extras;
    type Error;
    fn take(&mut self, field: Field) -> Option<Self::Value>;
    fn is_null(value: &Self::Value) -> bool;
    fn literal(&mut self, value: Self::Value) -> Result<Self::Value, Self::Error>;
    fn text(&mut self, value: Self::Value) -> Result<Self::Text, Self::Error>;
    fn values(&mut self, value: Self::Value) -> Result<Self::Values, Self::Error>;
    fn strings(&mut self, value: Self::Value) -> Result<Self::Strings, Self::Error>;
    fn strings_many(&mut self, value: Self::Value) -> Result<Self::Strings, Self::Error>;
    fn languages(&mut self, value: Self::Value) -> Result<Self::Languages, Self::Error>;
    fn schemas(&mut self, value: Self::Value) -> Result<Self::Schemas, Self::Error>;
    fn schemas_many(&mut self, value: Self::Value) -> Result<Self::Schemas, Self::Error>;
    fn schema_map(&mut self, value: Self::Value) -> Result<Self::SchemaMap, Self::Error>;
    fn boolean(&mut self, value: Self::Value) -> Result<bool, Self::Error>;
    fn unsigned(&mut self, value: Self::Value) -> Result<u32, Self::Error>;
    fn float(&mut self, value: Self::Value) -> Result<f64, Self::Error>;
    fn integer(&mut self, value: Self::Value) -> Result<i64, Self::Error>;
    fn metadata(&mut self) -> Result<Metadata<Self>, Self::Error>;
    fn remaining_context(self) -> Result<Context<Self>, Self::Error>;
    fn extras(self) -> Self::Extras;
}

#[derive(Clone, Copy)]
enum Null {
    Value,
    Absent,
}

fn optional<B: Source, T>(
    source: &mut B,
    field: Field,
    null: Null,
    convert: impl FnOnce(&mut B, B::Value) -> Result<T, B::Error>,
) -> Result<Option<T>, B::Error> {
    match source.take(field) {
        None => Ok(None),
        Some(value) if matches!(null, Null::Absent) && B::is_null(&value) => Ok(None),
        Some(value) => convert(source, value).map(Some),
    }
}

macro_rules! metadata_reader {
    ($( $name:ident, $field:ident, $wire:literal, $ty:ty, $assoc:ident, $read:ident,
        [$($attribute:tt)*]; )*) => {
        pub struct Metadata<B: Source> { $(pub $name: Option<B::$assoc>,)* }
        pub fn metadata<B: Source>(source: &mut B) -> Result<Metadata<B>, B::Error> {
            Ok(Metadata { $(
                $name: optional(source, Field::$field, Null::Absent, B::$read)?,
            )* })
        }
    };
}
metadata_fields!(metadata_reader);

pub struct Context<B: Source> {
    pub metadata: Metadata<B>,
    pub constant: Option<B::Value>,
    pub default: Option<B::Value>,
    pub unit: Option<B::Text>,
    pub one_of: Option<B::Schemas>,
    pub enumerate: Option<B::Values>,
    pub read_only: bool,
    pub write_only: bool,
    pub format: Option<B::Text>,
    pub data_type: Option<B::Text>,
    pub extras: B::Extras,
}

pub fn context<B: Source>(mut source: B) -> Result<Context<B>, B::Error> {
    let metadata = source.metadata()?;
    let constant = optional(&mut source, Field::Const, Null::Value, B::literal)?;
    let default = optional(&mut source, Field::Default, Null::Value, B::literal)?;
    let unit = optional(&mut source, Field::Unit, Null::Value, B::text)?;
    let one_of = optional(&mut source, Field::OneOf, Null::Value, B::schemas)?;
    let enumerate = optional(&mut source, Field::Enum, Null::Value, B::values)?;
    let read_only =
        optional(&mut source, Field::ReadOnly, Null::Value, B::boolean)?.unwrap_or(false);
    let write_only =
        optional(&mut source, Field::WriteOnly, Null::Value, B::boolean)?.unwrap_or(false);
    let format = optional(&mut source, Field::Format, Null::Value, B::text)?;
    let data_type = optional(&mut source, Field::Type, Null::Value, B::text)?;
    Ok(Context {
        metadata,
        constant,
        default,
        unit,
        one_of,
        enumerate,
        read_only,
        write_only,
        format,
        data_type,
        extras: source.extras(),
    })
}

pub fn dispatch(data_type: Option<&str>) -> SchemaKind {
    match data_type {
        Some("array") => SchemaKind::Array,
        Some("boolean") => SchemaKind::Boolean,
        Some("number") => SchemaKind::Number,
        Some("integer") => SchemaKind::Integer,
        Some("string") => SchemaKind::String,
        Some("null") => SchemaKind::Null,
        _ => SchemaKind::Object,
    }
}

pub enum Shape<B: Source> {
    Array {
        items: Option<B::Schemas>,
        min: Option<u32>,
        max: Option<u32>,
    },
    Boolean,
    Number([Option<f64>; 5]),
    Integer([Option<i64>; 5]),
    Object {
        properties: Option<B::SchemaMap>,
        required: Option<B::Strings>,
    },
    String {
        min: Option<u32>,
        max: Option<u32>,
        pattern: Option<B::Text>,
        encoding: Option<B::Text>,
        media_type: Option<B::Text>,
    },
    Null,
}
pub struct Decoded<B: Source> {
    pub context: Context<B>,
    pub shape: Shape<B>,
}

fn numeric<B: Source, T>(
    source: &mut B,
    convert: fn(&mut B, B::Value) -> Result<T, B::Error>,
) -> Result<[Option<T>; 5], B::Error> {
    Ok([
        optional(source, Field::Minimum, Null::Value, convert)?,
        optional(source, Field::ExclusiveMinimum, Null::Value, convert)?,
        optional(source, Field::Maximum, Null::Value, convert)?,
        optional(source, Field::ExclusiveMaximum, Null::Value, convert)?,
        optional(source, Field::MultipleOf, Null::Value, convert)?,
    ])
}

pub fn variant<B: Source>(mut source: B, kind: SchemaKind) -> Result<Decoded<B>, B::Error> {
    let shape = match kind {
        SchemaKind::Array => Shape::Array {
            items: optional(&mut source, Field::Items, Null::Absent, B::schemas_many)?,
            min: optional(&mut source, Field::MinItems, Null::Value, B::unsigned)?,
            max: optional(&mut source, Field::MaxItems, Null::Value, B::unsigned)?,
        },
        SchemaKind::Boolean => Shape::Boolean,
        SchemaKind::Number => Shape::Number(numeric(&mut source, B::float)?),
        SchemaKind::Integer => Shape::Integer(numeric(&mut source, B::integer)?),
        SchemaKind::Object => Shape::Object {
            properties: optional(&mut source, Field::Properties, Null::Value, B::schema_map)?,
            required: optional(&mut source, Field::Required, Null::Value, B::strings)?,
        },
        SchemaKind::String => Shape::String {
            min: optional(&mut source, Field::MinLength, Null::Value, B::unsigned)?,
            max: optional(&mut source, Field::MaxLength, Null::Value, B::unsigned)?,
            pattern: optional(&mut source, Field::Pattern, Null::Value, B::text)?,
            encoding: optional(&mut source, Field::ContentEncoding, Null::Value, B::text)?,
            media_type: optional(&mut source, Field::ContentMediaType, Null::Value, B::text)?,
        },
        SchemaKind::Null => Shape::Null,
    };
    Ok(Decoded {
        shape,
        context: source.remaining_context()?,
    })
}
