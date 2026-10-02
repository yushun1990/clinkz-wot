//! Decision probes, not a bounded decoder or an admission construction proof.
//!
//! The literal reference uses public RawValue/scalar APIs synchronously and
//! allocates a recursive Value graph. It deliberately cannot serve as strict
//! production code. No dependency source or private dispatch is used.

#[cfg(any(feature = "ap", feature = "validated-thing"))]
use serde::{Deserialize, Deserializer, de::Visitor};

#[path = "support/literal_value_reference.rs"]
mod literal_value_reference;
use literal_value_reference::literal;

// A generic public Visitor records the events it actually receives. In an AP
// graph, a large Number token and one literal object have identical events.
// This defeats an always-object visit_map shortcut, not every possible public
// API construction: a project lexer can retain wire provenance separately.
#[cfg(any(feature = "ap", feature = "validated-thing"))]
#[derive(Debug, PartialEq)]
enum Events {
    String(String),
    Map(Vec<(String, Events)>),
}

#[cfg(any(feature = "ap", feature = "validated-thing"))]
impl<'de> Deserialize<'de> for Events {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EventVisitor;
        impl<'de> Visitor<'de> for EventVisitor {
            type Value = Events;
            fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.write_str("the probe's string or map events")
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Events, E> {
                Ok(Events::String(value.into()))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut access: A,
            ) -> Result<Events, A::Error> {
                let mut events = Vec::new();
                while let Some(entry) = access.next_entry()? {
                    events.push(entry);
                }
                Ok(Events::Map(events))
            }
        }
        deserializer.deserialize_any(EventVisitor)
    }
}

#[cfg(any(feature = "ap", feature = "validated-thing"))]
#[test]
fn generic_public_events_do_not_identify_number_token_provenance() {
    for token in ["1e+309", "18446744073709551616"] {
        let object = format!(r#"{{"$serde_json::private::Number":"{token}"}}"#);
        assert_eq!(
            serde_json::from_str::<Events>(token).unwrap(),
            serde_json::from_str::<Events>(&object).unwrap(),
        );
        assert!(literal(token).unwrap().is_number());
        assert!(literal(&object).unwrap().is_object());
    }
}

mod unchanged_td {
    use clinkz_wot_td as td;
    include!("support/value_decode_contract_cases.rs");
}

mod shared_basic_candidate {
    use validated_thing_schema_kernel_probe as td;
    include!("support/value_decode_contract_cases.rs");
}
