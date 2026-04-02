use core::fmt;

use serde::{Deserializer, de::{self, Visitor}};

/// Supports deserializing from boolean, numbers (0/1), or strings ("true"/"false"/"1"/"0")
/// into bool.
pub fn deserialize_bool_flexible<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    struct BoolVisitor;

    impl<'de> de::Visitor<'de> for BoolVisitor {
        type Value = bool;

        fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
            formatter.write_str("a boolean, an integer (0 or 1), or a string representation")
        }

        // Handle bool value: true, false
        fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> {
            Ok(v)
        }

        // Handle number value: 1 -> true, 0 -> false
        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match v {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(de::Error::invalid_value(de::Unexpected::Unsigned(v), &"0 or 1")),
            }
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match v {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(de::Error::invalid_value(de::Unexpected::Signed(v), &"0 or 1")),
            }
        }

        // Handle string value: "1"/"0", "true"/"false"
        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match v {
                "true" | "1" => Ok(true),
                "false" | "0" => Ok(false),
                _ => Err(de::Error::unknown_variant(v, &["true", "false", "1", "0"])),
            }
        }
    }

    deserializer.deserialize_any(BoolVisitor)
}


/// Supports deserializing from boolean, numbers (0/1), or strings ("true"/"false"/"1"/"0")
/// into Option<bool>.
pub fn deserialize_option_bool_flexible<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptionBoolVisitor;

    impl<'de> Visitor<'de> for OptionBoolVisitor {
        type Value = Option<bool>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a boolean, an integer (0 or 1), a string representation, or null")
        }

        // Handle null values (None)
        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        // Handle Some(value) when the deserializer provides an option wrapper
        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(self)
        }

        // Handle native booleans: true, false
        fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(v))
        }

        // Handle unsigned integers: 1 -> Some(true), 0 -> Some(false)
        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match v {
                0 => Ok(Some(false)),
                1 => Ok(Some(true)),
                _ => Err(de::Error::invalid_value(de::Unexpected::Unsigned(v), &"0 or 1")),
            }
        }

        // Handle signed integers: 1 -> Some(true), 0 -> Some(false)
        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match v {
                0 => Ok(Some(false)),
                1 => Ok(Some(true)),
                _ => Err(de::Error::invalid_value(de::Unexpected::Signed(v), &"0 or 1")),
            }
        }

        // Handle strings: "1"/"0", "true"/"false"
        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match v {
                "true" | "1" => Ok(Some(true)),
                "false" | "0" => Ok(Some(false)),
                _ => Err(de::Error::unknown_variant(v, &["true", "false", "1", "0"])),
            }
        }
    }

    // We use deserialize_any to allow the visitor to decide based on the input type
    deserializer.deserialize_any(OptionBoolVisitor)
}
