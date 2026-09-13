#![no_std]

extern crate alloc;

// Test-only probe of one possible graph-neutral public Number access path.
// A callback reads one byte from a borrowed serde string; scalar graphs use
// Number Display into a fixed inline buffer only after scalar dispatch.

use core::fmt::{self, Write};
use serde::Serialize;
use serde::ser::{Impossible, SerializeStruct, Serializer};
use serde_json::Number;

#[derive(Debug)]
pub struct ProbeError;

impl fmt::Display for ProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unsupported Number serialization callback")
    }
}

impl serde::ser::Error for ProbeError {
    fn custom<T: fmt::Display>(_: T) -> Self {
        Self
    }
}

struct Inspector {
    position: usize,
    byte: Option<u8>,
}

impl SerializeStruct for &mut Inspector {
    type Ok = bool;
    type Error = ProbeError;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        if value.serialize(&mut **self)? {
            return Err(ProbeError);
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(false)
    }
}

impl Serializer for &mut Inspector {
    type Ok = bool;
    type Error = ProbeError;
    type SerializeSeq = Impossible<bool, ProbeError>;
    type SerializeTuple = Impossible<bool, ProbeError>;
    type SerializeTupleStruct = Impossible<bool, ProbeError>;
    type SerializeTupleVariant = Impossible<bool, ProbeError>;
    type SerializeMap = Impossible<bool, ProbeError>;
    type SerializeStruct = Self;
    type SerializeStructVariant = Impossible<bool, ProbeError>;

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        self.byte = value.as_bytes().get(self.position).copied();
        Ok(false)
    }

    fn serialize_i8(self, _: i8) -> Result<Self::Ok, Self::Error> {
        Ok(true)
    }
    fn serialize_i16(self, _: i16) -> Result<Self::Ok, Self::Error> {
        Ok(true)
    }
    fn serialize_i32(self, _: i32) -> Result<Self::Ok, Self::Error> {
        Ok(true)
    }
    fn serialize_i64(self, _: i64) -> Result<Self::Ok, Self::Error> {
        Ok(true)
    }
    fn serialize_i128(self, _: i128) -> Result<Self::Ok, Self::Error> {
        Ok(true)
    }
    fn serialize_u8(self, _: u8) -> Result<Self::Ok, Self::Error> {
        Ok(true)
    }
    fn serialize_u16(self, _: u16) -> Result<Self::Ok, Self::Error> {
        Ok(true)
    }
    fn serialize_u32(self, _: u32) -> Result<Self::Ok, Self::Error> {
        Ok(true)
    }
    fn serialize_u64(self, _: u64) -> Result<Self::Ok, Self::Error> {
        Ok(true)
    }
    fn serialize_u128(self, _: u128) -> Result<Self::Ok, Self::Error> {
        Ok(true)
    }
    fn serialize_f32(self, _: f32) -> Result<Self::Ok, Self::Error> {
        Ok(true)
    }
    fn serialize_f64(self, _: f64) -> Result<Self::Ok, Self::Error> {
        Ok(true)
    }
    fn serialize_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(self)
    }

    fn serialize_bool(self, _: bool) -> Result<Self::Ok, Self::Error> {
        Err(ProbeError)
    }
    fn serialize_char(self, _: char) -> Result<Self::Ok, Self::Error> {
        Err(ProbeError)
    }
    fn serialize_bytes(self, _: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(ProbeError)
    }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(ProbeError)
    }
    fn serialize_some<T: ?Sized + Serialize>(self, _: &T) -> Result<Self::Ok, Self::Error> {
        Err(ProbeError)
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(ProbeError)
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(ProbeError)
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ProbeError)
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ProbeError)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ProbeError)
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(ProbeError)
    }
    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(ProbeError)
    }
    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(ProbeError)
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(ProbeError)
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(ProbeError)
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(ProbeError)
    }
}

struct InlineText {
    bytes: [u8; 64],
    len: usize,
}

impl InlineText {
    const fn new() -> Self {
        Self {
            bytes: [0; 64],
            len: 0,
        }
    }
    fn byte(&self, position: usize) -> Option<u8> {
        self.bytes
            .get(position)
            .filter(|_| position < self.len)
            .copied()
    }
}

impl Write for InlineText {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len.checked_add(s.len()).ok_or(fmt::Error)?;
        let target = self.bytes.get_mut(self.len..end).ok_or(fmt::Error)?;
        target.copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

/// Each call inspects at most one input byte in the borrowed-string branch.
/// This source probe does not claim a stable upstream serialization shape or
/// the full cursor's WorkBudget/ledger proof.
pub fn decimal_byte(number: &Number, position: usize) -> Result<Option<u8>, ProbeError> {
    let mut inspector = Inspector {
        position,
        byte: None,
    };
    let scalar = number.serialize(&mut inspector)?;
    if scalar {
        let mut text = InlineText::new();
        write!(&mut text, "{number}").map_err(|_| ProbeError)?;
        Ok(text.byte(position))
    } else {
        Ok(inspector.byte)
    }
}

#[cfg(test)]
mod tests {
    use super::decimal_byte;
    use alloc::string::ToString;
    use serde_json::Number;

    #[test]
    fn base_graph_scalar_presentation_is_bounded_inline() {
        for source in ["0", "-0.0", "9007199254740993.0", "1e-4000", "1e+30"] {
            let Ok(number) = serde_json::from_str::<Number>(source) else {
                continue;
            };
            let expected = number.to_string();
            for (position, byte) in expected.bytes().enumerate() {
                assert_eq!(decimal_byte(&number, position).unwrap(), Some(byte));
            }
            assert_eq!(decimal_byte(&number, expected.len()).unwrap(), None);
        }
    }
}
