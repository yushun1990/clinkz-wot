//! Synchronous allocating test oracle from #117; never an engine decoder.

use serde::{Deserialize, Deserializer, de::Visitor};
use serde_json::{Map, Value, value::RawValue};

struct Members(Vec<(String, Box<RawValue>)>);

impl<'de> Deserialize<'de> for Members {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor;
        impl<'de> Visitor<'de> for ObjectVisitor {
            type Value = Members;
            fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.write_str("a JSON object")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut access: A,
            ) -> Result<Members, A::Error> {
                let mut members = Vec::new();
                while let Some(entry) = access.next_entry()? {
                    members.push(entry);
                }
                Ok(Members(members))
            }
        }
        deserializer.deserialize_map(ObjectVisitor)
    }
}

pub(crate) fn literal(text: &str) -> Result<Value, serde_json::Error> {
    let raw: Box<RawValue> = serde_json::from_str(text)?;
    let text = raw.get();
    match text.as_bytes()[0] {
        b'{' => {
            let mut map = Map::new();
            for (key, value) in serde_json::from_str::<Members>(text)?.0 {
                // Resolve decoded duplicate names in source order. Every value
                // is parsed, including the overwritten occurrence.
                map.insert(key, literal(value.get())?);
            }
            Ok(Value::Object(map))
        }
        b'[' => serde_json::from_str::<Vec<Box<RawValue>>>(text)?
            .into_iter()
            .map(|value| literal(value.get()))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        b'"' => serde_json::from_str(text).map(Value::String),
        b't' | b'f' => serde_json::from_str(text).map(Value::Bool),
        b'n' => Ok(Value::Null),
        _ => serde_json::from_str(text).map(Value::Number),
    }
}
