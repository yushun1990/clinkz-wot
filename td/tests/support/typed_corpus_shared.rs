//! Fixed typed input shared by the #96 tests and the snapshot prototype.

use super::td_crate::{
    context::Context,
    thing::Thing,
    validate::{Validate, ValidationLevel},
};
use serde_json::Value;

pub const CORPUS: &str = r##"{
    "@context": [
        "https://www.w3.org/2022/wot/td/v1.1",
        { "ex": "https://example.org/ns#" },
        "https://example.org/extra-context"
    ],
    "id": "urn:example:typed-corpus",
    "title": "Typed corpus",
    "titles": { "en": "Typed corpus", "fr": "Corpus typé" },
    "support": "https://example.org/support",
    "base": "https://example.org/things/",
    "security": ["none"],
    "securityDefinitions": { "none": { "scheme": "nosec" } },
    "properties": {
        "zeta": {
            "type": "string",
            "forms": [
                { "href": "zeta/first", "op": ["readproperty", "writeproperty"], "contentType": "text/plain" },
                { "href": "zeta/second", "op": "readproperty" }
            ]
        },
        "alpha": {
            "type": "boolean",
            "forms": [{ "href": "alpha", "op": "readproperty" }]
        }
    },
    "forms": [],
    "ex:payload": {
        "flag": true,
        "count": 123456789012345678901234567890,
        "empty": null,
        "items": ["first", { "left": "L", "right": "R" }]
    }
}"##;

pub fn typed_corpus() -> Thing {
    let mut thing: Thing = serde_json::from_str(CORPUS).expect("fixed TD input must decode");
    thing
        ._extra_fields
        .insert("ex:long".into(), Value::String("λ".repeat(2_048)));
    thing
        .validate_with_level(ValidationLevel::Basic)
        .expect("fixed typed corpus must pass Basic");
    thing
}

pub fn serializer_failure_thing() -> Thing {
    let mut thing = typed_corpus();
    thing.context =
        serde_json::from_str::<Context>(r#"["https://example.org/extension-only"]"#).unwrap();
    thing
        .validate_with_level(ValidationLevel::Basic)
        .expect("Basic accepts extension-only Context");
    thing
}
