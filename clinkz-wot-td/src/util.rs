use serde::{de, Deserializer};

/// 支持从布尔值、数字 (0/1) 或字符串 ("true"/"false"/"1"/"0") 反序列化为 bool
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

        // 处理原生布尔值: true, false
        fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> {
            Ok(v)
        }

        // 处理数字: 1 -> true, 0 -> false
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

        // 处理字符串 (可选，增加鲁棒性): "1"/"0", "true"/"false"
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
