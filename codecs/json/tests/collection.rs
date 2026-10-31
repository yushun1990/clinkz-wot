use clinkz_wot_codec_json::validate_property_map;
use clinkz_wot_core::{Payload, collection::*, value_type::*};
fn schema(names: &[&str]) -> PropertyCollectionSchema {
    PropertyCollectionSchema::try_new(
        names
            .iter()
            .map(|name| PropertyMemberSchema {
                name: (*name).into(),
                schema: TypeSchema::Type(ValueType::Integer),
            })
            .collect(),
    )
    .unwrap()
}
fn check(s: &PropertyCollectionSchema, bytes: &[u8]) -> bool {
    validate_property_map(s, &Payload::new(bytes, "application/json")).is_ok()
}
#[test]
fn empty_maximal_and_unicode_member_sets_validate_exactly() {
    assert!(check(&schema(&[]), b"{}"));
    assert!(!check(&schema(&[]), br#"{"extra":0}"#));
    assert!(!check(&schema(&[]), b"null"));
    let names: Vec<_> = (0..32).map(|i| format!("p{i}")).collect();
    let s = schema(&names.iter().map(String::as_str).collect::<Vec<_>>());
    let mut values = serde_json::Map::new();
    for name in &names {
        values.insert(name.clone(), serde_json::json!(7));
    }
    assert!(check(
        &s,
        serde_json::to_string(&values).unwrap().as_bytes()
    ));
    values.remove("p31");
    assert!(!check(
        &s,
        serde_json::to_string(&values).unwrap().as_bytes()
    ));
    let s = schema(&["", "温度", "\"", "😀"]);
    assert!(check(
        &s,
        br#"{"":7,"\u6e29\u5ea6":7.0,"\"":70e-1,"\uD83D\uDE00":7}"#
    ));
    assert!(!check(&s, br#"{"":7,"\u6e29\u5ea6":7,"\"":7,"\uD83D":7}"#));
}
#[test]
fn descriptor_bounds_and_decoded_duplicate_names_cannot_weaken_the_map() {
    let mut oversized_backing = Vec::with_capacity(64);
    oversized_backing.push(PropertyMemberSchema {
        name: "level".into(),
        schema: TypeSchema::Any,
    });
    assert!(PropertyCollectionSchema::try_new(oversized_backing).is_err());
    let s = schema(&["level"]);
    assert!(check(&s, br#"{"\u006cevel":7}"#));
    assert!(!check(&s, br#"{"level":7,"\u006cevel":8}"#));
    assert!(
        PropertyCollectionSchema::try_new(vec![PropertyMemberSchema {
            name: "x".repeat(257).into(),
            schema: TypeSchema::Any
        }])
        .is_err()
    );
    assert!(
        PropertyCollectionSchema::try_new(
            (0..33)
                .map(|i| PropertyMemberSchema {
                    name: format!("p{i}").into(),
                    schema: TypeSchema::Any
                })
                .collect()
        )
        .is_err()
    );
    assert!(
        PropertyCollectionSchema::try_new(
            (0..17)
                .map(|i| PropertyMemberSchema {
                    name: format!("{i:03}{}", "x".repeat(253)).into(),
                    schema: TypeSchema::Any
                })
                .collect()
        )
        .is_err()
    );
}
