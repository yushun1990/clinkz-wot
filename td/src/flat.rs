//! Field-flattening helpers used in place of `#[serde(flatten)]`.
//!
//! `#[serde(flatten)]` forces serde to buffer the entire struct through an
//! internal `Content` layer on deserialization, which (a) is a per-field cost
//! across every flatten-using struct and (b) blocks the cheaper
//! `Box<RawValue>` buffering in `DataSchema`/`SecurityScheme` — `RawValue`
//! cannot be captured from a `Content` deserializer. These helpers let each
//! struct buffer once into a [`serde_json::Map`] instead (never serde's
//! `Content`), so nested typed deserialization runs against serde_json's native
//! deserializer and `RawValue` works once flatten is gone from the ancestor
//! chain.
//!
//! The per-struct pattern is uniform:
//!
//! ```ignore
//! impl<'de> Deserialize<'de> for Foo {
//!     fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
//!         let mut map = flat::deserialize_map(d)?;
//!         let a = flat::take_required(&mut map, "a")?;
//!         let b = flat::take::<B>(&mut map, "b")?;
//!         Ok(Foo { a, b, _extra_fields: flat::into_extras(map) })
//!     }
//! }
//! ```

use alloc::string::String;
use alloc::vec::Vec;

use serde::Deserialize;
use serde::Serialize;
use serde::Serializer;
use serde::de::DeserializeOwned;

use crate::data_type::ExtensionMap;

/// A JSON object map, used as the single per-struct buffer.
pub(crate) type JsonMap = serde_json::Map<String, serde_json::Value>;

/// Deserializes a JSON object map directly from the deserializer (no `Content`).
pub(crate) fn deserialize_map<'de, D>(deserializer: D) -> Result<JsonMap, D::Error>
where
    D: serde::Deserializer<'de>,
{
    serde_json::Map::deserialize(deserializer)
}

/// Removes `key` from `map` and deserializes its value into `T`.
///
/// Returns `None` when the key is absent. Field-level `serde_as` adapters are
/// applied by deserializing through `T`'s derived `Deserialize` (so pass a
/// helper newtype when the field carries `serde_as`).
pub(crate) fn take<T, E>(map: &mut JsonMap, key: &str) -> Result<Option<T>, E>
where
    T: DeserializeOwned,
    E: serde::de::Error,
{
    map.remove(key)
        .map(serde_json::from_value)
        .transpose()
        .map_err(E::custom)
}

/// Removes `key` from `map` and deserializes its value into `Option<Vec<T>>`
/// using the W3C TD one-or-many convention: a JSON array yields one element per
/// item; any other JSON value yields a single-element vec; `null` and a missing
/// key yield `None`.
///
/// This replicates `serde_with::OneOrMany` for element types that need to
/// deserialize through `Box<RawValue>` (e.g. [`DataSchema`](crate::data_schema)).
/// `serde_with::OneOrMany` buffers its input through serde's internal `Content`
/// layer, which — like `#[serde(flatten)]` — prevents `RawValue` capture. This
/// helper instead deserializes each element directly through `serde_json`
/// (via [`from_value`](serde_json::from_value)), so the element type's own
/// deserializer runs against serde_json's native, raw-capable deserializer.
pub(crate) fn take_one_or_many<T, E>(map: &mut JsonMap, key: &str) -> Result<Option<Vec<T>>, E>
where
    T: DeserializeOwned,
    E: serde::de::Error,
{
    match map.remove(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Array(arr)) => {
            let mut out = Vec::with_capacity(arr.len());
            for value in arr {
                out.push(serde_json::from_value(value).map_err(E::custom)?);
            }
            Ok(Some(out))
        }
        Some(other) => Ok(Some(alloc::vec![
            serde_json::from_value(other).map_err(E::custom)?
        ])),
    }
}

/// Removes a required `key` from `map`; errors if absent.
pub(crate) fn take_required<T, E>(map: &mut JsonMap, key: &str) -> Result<T, E>
where
    T: DeserializeOwned,
    E: serde::de::Error,
{
    take(map, key)?.ok_or_else(|| E::custom(alloc::format!("missing field `{key}`")))
}

/// Consumes the leftover map entries as extension fields.
pub(crate) fn into_extras(map: JsonMap) -> ExtensionMap {
    map.into_iter().collect()
}

/// Serializes a slice using the same one-or-many convention as
/// `serde_with::OneOrMany`: a single-element slice is emitted as a bare value,
/// anything else as a JSON array. Used by structs whose fields previously
/// carried `#[serde_as(as = "Option<OneOrMany<_>>")]` so the serialized form
/// preserves single-vs-array round-trip fidelity.
pub(crate) struct OneOrManyRef<'a, T: Serialize>(pub &'a [T]);

impl<T: Serialize> Serialize for OneOrManyRef<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            [single] => single.serialize(serializer),
            rest => rest.serialize(serializer),
        }
    }
}

/// Drains `keys` out of `map` into a fresh sub-object and deserializes it into
/// `T` via `from_value` (serde_json-backed, so `T`'s derived `Deserialize`
/// applies its `serde_as`/`deserialize_with` adapters, and nested types that
/// were themselves un-flattened can use `Box<RawValue>`).
///
/// Used for flattened composition fields like `Metadata` whose keys live at the
/// same level as the parent's own fields.
pub(crate) fn drain_substruct<T, E>(map: &mut JsonMap, keys: &[&str]) -> Result<T, E>
where
    T: DeserializeOwned,
    E: serde::de::Error,
{
    let mut sub = serde_json::Map::new();
    for key in keys {
        if let Some(value) = map.remove(*key) {
            sub.insert((*key).into(), value);
        }
    }
    serde_json::from_value(serde_json::Value::Object(sub)).map_err(E::custom)
}

/// Deserializes the remaining map entries into `T` via `from_value`
/// (serde_json-backed). Used when a flattened field (e.g. `DataSchema`) owns
/// the rest of the object — its discriminator-peek deserializer then runs
/// against serde_json and can use `Box<RawValue>`.
pub(crate) fn from_remaining<T, E>(map: JsonMap) -> Result<T, E>
where
    T: DeserializeOwned,
    E: serde::de::Error,
{
    serde_json::from_value(serde_json::Value::Object(map)).map_err(E::custom)
}
