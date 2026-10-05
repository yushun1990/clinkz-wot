//! Literal DataSchema field facts over #119's actual owned arenas, followed by
//! #114's unchanged Basic rule body. No Thing/Value/Number/String is constructed.
//! Field inspection and Basic traversal remain synchronous and recursive.
use super::{
    schema_fields::{self as policy, Context, Decoded, Field, Metadata, Shape},
    schema_kernel::{self as basic, ChildSite, SchemaAccess, SchemaKind},
};
use serde::{
    Deserializer,
    de::{self, Visitor},
};
use validated_thing_value_construction_probe::{Kind, View};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodeError {
    pub field: Option<Field>,
}
impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("invalid schema field")
    }
}
impl core::error::Error for DecodeError {}
impl de::Error for DecodeError {
    fn custom<T: core::fmt::Display>(_: T) -> Self {
        Self { field: None }
    }
}

/// The fixed mask marks policy-owned keys, not physical arena indices. Arbitrary
/// extension count or source member order cannot overflow or change ownership.
pub struct Source<'a> {
    object: View<'a>,
    consumed: u64,
    active: Option<Field>,
}
impl<'a> Source<'a> {
    fn new(object: View<'a>) -> Result<Self, DecodeError> {
        if object.kind() != Kind::Object {
            return Err(DecodeError { field: None });
        }
        Ok(Self {
            object,
            consumed: 0,
            active: None,
        })
    }
    fn error(&self) -> DecodeError {
        DecodeError { field: self.active }
    }
    fn list(&self, value: View<'a>, one_or_many: bool) -> Result<List<'a>, DecodeError> {
        if value.kind() == Kind::Array {
            Ok(List {
                value,
                single: false,
            })
        } else if one_or_many {
            Ok(List {
                value,
                single: true,
            })
        } else {
            Err(self.error())
        }
    }
    fn scalar<T: for<'de> serde::Deserialize<'de>>(
        &self,
        value: View<'a>,
    ) -> Result<T, DecodeError> {
        T::deserialize(Scalar(value)).map_err(|_| self.error())
    }
}

#[derive(Clone, Copy)]
pub struct List<'a> {
    pub(crate) value: View<'a>,
    pub(crate) single: bool,
}
impl<'a> List<'a> {
    pub fn len(self) -> usize {
        if self.single { 1 } else { self.value.len() }
    }
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
    pub fn get(self, index: usize) -> Option<View<'a>> {
        if self.single {
            (index == 0).then_some(self.value)
        } else {
            self.value.child(index)
        }
    }
}

#[derive(Clone, Copy)]
pub struct Extras<'a> {
    pub(crate) object: View<'a>,
    pub(crate) consumed: u64,
    indexed: Option<[Option<View<'a>>; 29]>,
}
impl<'a> Extras<'a> {
    pub(crate) fn indexed(object: View<'a>, consumed: u64, index: &[Option<View<'a>>; 29]) -> Self {
        Self {
            object,
            consumed,
            indexed: Some(*index),
        }
    }
    /// Bounded admission uses only the fixed index prepared by schema_step.
    /// The synchronous reference retains its established object lookup.
    pub fn basic_field(self, field: basic::Field) -> Option<View<'a>> {
        if self.consumed & (1 << policy_field(field) as u8) != 0 {
            return None;
        }
        match self.indexed {
            Some(index) => index[policy_field(field) as usize],
            None => self.object.get(field.name()),
        }
    }
    /// Stable literal node identity selects consumed fields from the paid
    /// index; no second member-name scan is needed during canonical emission.
    pub(crate) fn consumed_value(self, value: View<'a>) -> bool {
        self.indexed
            .expect("charged index required")
            .into_iter()
            .enumerate()
            .any(|(field, candidate)| {
                self.consumed & (1 << field) != 0
                    && candidate.is_some_and(|candidate| candidate.same_node(value))
            })
    }
    pub fn get(self, key: &str) -> Option<View<'a>> {
        if Field::ALL
            .into_iter()
            .any(|field| self.consumed & (1 << field as u8) != 0 && field.name() == key)
        {
            return None;
        }
        self.object.get(key)
    }
    pub fn members(self) -> impl Iterator<Item = (&'a str, View<'a>)> {
        (0..self.object.len()).filter_map(move |i| {
            let (key, value) = self.object.member(i)?;
            self.get(key).map(|_| (key, value))
        })
    }
}
fn policy_field(field: basic::Field) -> Field {
    match field {
        basic::Field::MinItems => Field::MinItems,
        basic::Field::MaxItems => Field::MaxItems,
        basic::Field::MinLength => Field::MinLength,
        basic::Field::MaxLength => Field::MaxLength,
        basic::Field::Minimum => Field::Minimum,
        basic::Field::ExclusiveMinimum => Field::ExclusiveMinimum,
        basic::Field::Maximum => Field::Maximum,
        basic::Field::ExclusiveMaximum => Field::ExclusiveMaximum,
        basic::Field::MultipleOf => Field::MultipleOf,
    }
}

impl<'a> policy::Source for Source<'a> {
    type Value = View<'a>;
    type Text = &'a str;
    type Values = View<'a>;
    type Strings = List<'a>;
    type Languages = View<'a>;
    type Schemas = List<'a>;
    type SchemaMap = View<'a>;
    type Extras = Extras<'a>;
    type Error = DecodeError;
    fn take(&mut self, field: Field) -> Option<View<'a>> {
        self.active = Some(field);
        self.consumed |= 1 << field as u8;
        self.object.get(field.name())
    }
    fn is_null(value: &View<'a>) -> bool {
        value.kind() == Kind::Null
    }
    fn literal(&mut self, value: View<'a>) -> Result<View<'a>, DecodeError> {
        Ok(value)
    }
    fn text(&mut self, value: View<'a>) -> Result<&'a str, DecodeError> {
        if value.kind() == Kind::String {
            Ok(value.text().unwrap())
        } else {
            Err(self.error())
        }
    }
    fn values(&mut self, value: View<'a>) -> Result<View<'a>, DecodeError> {
        if value.kind() == Kind::Array {
            Ok(value)
        } else {
            Err(self.error())
        }
    }
    fn strings(&mut self, value: View<'a>) -> Result<List<'a>, DecodeError> {
        let list = self.list(value, false)?;
        for i in 0..list.len() {
            self.text(list.get(i).unwrap())?;
        }
        Ok(list)
    }
    fn strings_many(&mut self, value: View<'a>) -> Result<List<'a>, DecodeError> {
        let list = self.list(value, true)?;
        for i in 0..list.len() {
            self.text(list.get(i).unwrap())?;
        }
        Ok(list)
    }
    fn languages(&mut self, value: View<'a>) -> Result<View<'a>, DecodeError> {
        if value.kind() != Kind::Object {
            return Err(self.error());
        }
        for i in 0..value.len() {
            self.text(value.member(i).unwrap().1)?;
        }
        Ok(value)
    }
    fn schemas(&mut self, value: View<'a>) -> Result<List<'a>, DecodeError> {
        self.list(value, false)
    }
    fn schemas_many(&mut self, value: View<'a>) -> Result<List<'a>, DecodeError> {
        self.list(value, true)
    }
    fn schema_map(&mut self, value: View<'a>) -> Result<View<'a>, DecodeError> {
        if value.kind() == Kind::Object {
            Ok(value)
        } else {
            Err(self.error())
        }
    }
    fn boolean(&mut self, value: View<'a>) -> Result<bool, DecodeError> {
        crate::components::util::deserialize_bool_flexible(Scalar(value)).map_err(|_| self.error())
    }
    fn unsigned(&mut self, value: View<'a>) -> Result<u32, DecodeError> {
        self.scalar(value)
    }
    fn float(&mut self, value: View<'a>) -> Result<f64, DecodeError> {
        self.scalar(value)
    }
    fn integer(&mut self, value: View<'a>) -> Result<i64, DecodeError> {
        self.scalar(value)
    }
    fn metadata(&mut self) -> Result<Metadata<Self>, DecodeError> {
        policy::metadata(self)
    }
    fn remaining_context(self) -> Result<Context<Self>, DecodeError> {
        policy::context(self)
    }
    fn extras(self) -> Extras<'a> {
        Extras {
            object: self.object,
            consumed: self.consumed,
            indexed: None,
        }
    }
}

/// Only primitive public serde visitors are used. Object/Array cannot become a
/// scalar, and strings never open another document. Number text is already
/// guarded and AP-normalized by construction; no owned Number is made here.
struct Scalar<'a>(View<'a>);
impl<'de> Deserializer<'de> for Scalar<'de> {
    type Error = DecodeError;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DecodeError> {
        match self.0.kind() {
            Kind::Null => Primitive::Null.deserialize_any(visitor),
            Kind::False => Primitive::Boolean(false).deserialize_any(visitor),
            Kind::True => Primitive::Boolean(true).deserialize_any(visitor),
            Kind::String => Primitive::Text(self.0.text().unwrap()).deserialize_any(visitor),
            Kind::Number => {
                let text = self.0.text().unwrap();
                let primitive = if let Ok(value) = text.parse::<i64>() {
                    Primitive::Signed(value)
                } else if let Ok(value) = text.parse::<u64>() {
                    Primitive::Unsigned(value)
                } else {
                    Primitive::Float(text.parse().map_err(|_| DecodeError { field: None })?)
                };
                primitive.deserialize_any(visitor)
            }
            _ => Err(DecodeError { field: None }),
        }
    }
    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DecodeError> {
        if self.0.kind() == Kind::Number {
            visitor.visit_f64(
                self.0
                    .text()
                    .unwrap()
                    .parse()
                    .map_err(|_| DecodeError { field: None })?,
            )
        } else {
            self.deserialize_any(visitor)
        }
    }
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 char str string bytes byte_buf
        option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any
    }
}

/// Already selected public primitive events. Both the synchronous adapter and
/// charged adapter use the same serde/flexible-bool visitors; the latter pays
/// each Number parse before constructing an event, instead of parsing again.
pub(crate) enum Primitive<'a> {
    Null,
    Boolean(bool),
    Text(&'a str),
    Signed(i64),
    Unsigned(u64),
    Float(f64),
}
impl<'de> Deserializer<'de> for Primitive<'de> {
    type Error = DecodeError;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DecodeError> {
        match self {
            Self::Null => visitor.visit_unit(),
            Self::Boolean(value) => visitor.visit_bool(value),
            Self::Text(value) => visitor.visit_borrowed_str(value),
            Self::Signed(value) => visitor.visit_i64(value),
            Self::Unsigned(value) => visitor.visit_u64(value),
            Self::Float(value) => visitor.visit_f64(value),
        }
    }
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf
        option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any
    }
}

pub fn decode(value: View<'_>) -> Result<Decoded<Source<'_>>, DecodeError> {
    policy::decode(Source::new(value)?)
}

/// Match serde's complete field conversion before Basic starts. Traversal only
/// enters schema-bearing fields; opaque const/default/enum/extensions are not
/// recursively reinterpreted as schemas. This is deliberately not budgeted.
pub fn inspect(value: View<'_>) -> Result<(), DecodeError> {
    let decoded = decode(value)?;
    if let Some(children) = decoded.context.one_of {
        for i in 0..children.len() {
            inspect(children.get(i).unwrap())?;
        }
    }
    match decoded.shape {
        Shape::Array {
            items: Some(children),
            ..
        } => {
            for i in 0..children.len() {
                inspect(children.get(i).unwrap())?;
            }
        }
        Shape::Object {
            properties: Some(children),
            ..
        } => {
            for i in 0..children.len() {
                inspect(children.member(i).unwrap().1)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub struct Access;
impl<'a> SchemaAccess<'a> for Access {
    type Node = View<'a>;
    type Number = &'a str;
    fn kind(&self, node: View<'a>) -> SchemaKind {
        let value = decode(node).expect("inspect before Basic");
        match value.shape {
            Shape::Array { .. } => SchemaKind::Array,
            Shape::Boolean => SchemaKind::Boolean,
            Shape::Number(_) => SchemaKind::Number,
            Shape::Integer(_) => SchemaKind::Integer,
            Shape::Object { .. } => SchemaKind::Object,
            Shape::String { .. } => SchemaKind::String,
            Shape::Null => SchemaKind::Null,
        }
    }
    fn data_type(&self, node: View<'a>) -> Option<&'a str> {
        decode(node).unwrap().context.data_type
    }
    fn flags(&self, node: View<'a>) -> (bool, bool) {
        let context = decode(node).unwrap().context;
        (context.read_only, context.write_only)
    }
    fn one_of_count(&self, node: View<'a>) -> usize {
        decode(node).unwrap().context.one_of.map_or(0, List::len)
    }
    fn one_of(&self, node: View<'a>, index: usize) -> View<'a> {
        decode(node)
            .unwrap()
            .context
            .one_of
            .unwrap()
            .get(index)
            .unwrap()
    }
    fn child_count(&self, node: View<'a>) -> usize {
        match decode(node).unwrap().shape {
            Shape::Array { items, .. } => items.map_or(0, List::len),
            Shape::Object { properties, .. } => properties.map_or(0, View::len),
            _ => 0,
        }
    }
    fn child(&self, node: View<'a>, index: usize) -> (ChildSite<'a>, View<'a>) {
        match decode(node).unwrap().shape {
            Shape::Array { items, .. } => (
                ChildSite::Indexed(index),
                items.unwrap().get(index).unwrap(),
            ),
            Shape::Object { properties, .. } => {
                let (key, value) = properties.unwrap().member(index).unwrap();
                (ChildSite::Property(key), value)
            }
            _ => unreachable!(),
        }
    }
    fn unsigned_extension(&self, node: View<'a>, field: basic::Field) -> Option<u64> {
        let value = decode(node).unwrap().context.extras.get(field.name())?;
        if value.kind() == Kind::Number {
            value.text()?.parse().ok()
        } else {
            None
        }
    }
    fn number_extension(&self, node: View<'a>, field: basic::Field) -> Option<&'a str> {
        let value = decode(node).unwrap().context.extras.get(field.name())?;
        if value.kind() == Kind::Number {
            value.text()
        } else {
            None
        }
    }
    fn project_number(&self, number: &'a str) -> Option<f64> {
        number.parse().ok()
    }
    fn typed_unsigned(&self, node: View<'a>) -> (Option<u32>, Option<u32>) {
        match decode(node).unwrap().shape {
            Shape::Array { min, max, .. } | Shape::String { min, max, .. } => (min, max),
            _ => (None, None),
        }
    }
    fn typed_float(&self, node: View<'a>) -> [Option<f64>; 5] {
        let Shape::Number(values) = decode(node).unwrap().shape else {
            unreachable!()
        };
        values
    }
    fn typed_integer(&self, node: View<'a>) -> [Option<i64>; 5] {
        let Shape::Integer(values) = decode(node).unwrap().shape else {
            unreachable!()
        };
        values
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Invalid {
    Field(DecodeError),
    Basic(basic::InlineInvalid),
}
pub fn validate(value: View<'_>) -> Result<(), Invalid> {
    inspect(value).map_err(Invalid::Field)?;
    basic::validate(&Access, value, &basic::InlineSink).map_err(Invalid::Basic)
}

const _: () = assert!(!core::mem::needs_drop::<Decoded<Source<'static>>>());
