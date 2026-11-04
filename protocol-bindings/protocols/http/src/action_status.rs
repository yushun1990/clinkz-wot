//! The selected HTTP status-array representation. Core retains definitions and
//! media bytes; this module alone interprets native status resources/JSON.
use crate::*;
use clinkz_wot_core::action::ActionCollectionSchema;
use serde::{
    Deserialize,
    de::{DeserializeSeed, MapAccess, SeqAccess, Visitor},
};
use std::borrow::Cow;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Status<'a> {
    #[serde(borrow)]
    status: Cow<'a, str>,
    #[serde(borrow)]
    href: Cow<'a, str>,
    #[serde(default, borrow, deserialize_with = "present")]
    output: Option<&'a serde_json::value::RawValue>,
}
fn present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<&'de serde_json::value::RawValue>, D::Error> {
    <&serde_json::value::RawValue>::deserialize(d).map(Some)
}

pub(crate) fn validate(schema: &ActionCollectionSchema, payload: &Payload) -> CoreResult<()> {
    producer_validation::validate_payload(
        value_type::TypeSchema::Type(value_type::ValueType::Object),
        payload,
    )?;
    struct Members<'a>(&'a ActionCollectionSchema);
    struct Records<'a> {
        output: Option<value_type::TypeSchema>,
        identities: &'a mut Vec<String>,
    }
    impl<'de> DeserializeSeed<'de> for Records<'_> {
        type Value = ();
        fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
            d.deserialize_seq(self)
        }
    }
    impl<'de> Visitor<'de> for Records<'_> {
        type Value = ();
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("bounded invocation status records")
        }
        fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> Result<(), S::Error> {
            use serde::de::Error;
            while let Some(record) = seq.next_element::<Status<'de>>()? {
                let href = record.href.as_ref();
                let uri = href
                    .parse::<hyper::Uri>()
                    .map_err(|_| S::Error::custom("invalid status identity"))?;
                if self.identities.len() == 128
                    || href.len() > 256
                    || !matches!(uri.scheme_str(), Some("http" | "https"))
                    || uri.host().is_none_or(str::is_empty)
                    || uri.authority().is_none_or(|a| a.as_str().contains('@'))
                    || href.contains('#')
                    || self.identities.iter().any(|id| id == href)
                {
                    return Err(S::Error::custom(
                        "missing, duplicate or invalid status identity",
                    ));
                }
                match (record.status.as_ref(), self.output, record.output) {
                    ("pending" | "running" | "failed", _, None) | ("completed", None, None) => {}
                    ("completed", Some(schema), Some(output)) => {
                        producer_validation::validate_json(schema, output.get().as_bytes())
                            .map_err(|_| S::Error::custom("invalid completed output"))?
                    }
                    _ => return Err(S::Error::custom("invalid status or output role")),
                }
                self.identities.push(href.into());
            }
            Ok(())
        }
    }
    impl<'de> Visitor<'de> for Members<'_> {
        type Value = ();
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("an Action status map")
        }
        fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<(), M::Error> {
            use serde::de::Error;
            let mut seen = 0u32;
            let mut identities = Vec::with_capacity(128);
            while let Some(name) = map.next_key::<Cow<'de, str>>()? {
                let index = self
                    .0
                    .members()
                    .iter()
                    .position(|m| m.name.as_ref() == name.as_ref())
                    .ok_or_else(|| M::Error::custom("unknown Action"))?;
                let bit = 1u32 << index;
                if seen & bit != 0 {
                    return Err(M::Error::custom("duplicate Action"));
                }
                seen |= bit;
                map.next_value_seed(Records {
                    output: self.0.members()[index].output,
                    identities: &mut identities,
                })?;
            }
            Ok(())
        }
    }
    let mut decoder = serde_json::Deserializer::from_slice(&payload.body);
    serde::Deserializer::deserialize_map(&mut decoder, Members(schema)).map_err(|_| {
        CoreError::Validation(ErrorContext::new(ErrorPhase::Validate, RetryClass::Never))
    })?;
    decoder.end().map_err(|_| {
        CoreError::Validation(ErrorContext::new(ErrorPhase::Validate, RetryClass::Never))
    })
}
