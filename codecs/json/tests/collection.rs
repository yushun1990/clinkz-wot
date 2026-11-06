use clinkz_wot_codec_json::{
    decode_property_selection, encode_property_selection, validate_property_map,
    validate_property_write_map, validate_selected_property_map,
};
use clinkz_wot_core::{Payload, collection::*, data_schema::*};
fn schema(names: &[&str]) -> PropertyCollectionSchema {
    PropertyCollectionSchema::try_new(
        names
            .iter()
            .map(|name| PropertyMemberSchema {
                name: (*name).into(),
                schema: PreparedSchema::Type(ValueType::Integer),
            })
            .collect(),
    )
    .unwrap()
}
fn check(s: &PropertyCollectionSchema, bytes: &[u8]) -> bool {
    validate_property_map(s, &Payload::new(bytes, "application/json")).is_ok()
}

#[test]
fn selected_write_derives_only_validated_members_with_independent_schemas() {
    let s = PropertyCollectionSchema::try_new(vec![
        PropertyMemberSchema {
            name: "on".into(),
            schema: PreparedSchema::Type(ValueType::Boolean),
        },
        PropertyMemberSchema {
            name: "secret".into(),
            schema: PreparedSchema::Type(ValueType::String),
        },
        PropertyMemberSchema {
            name: "level".into(),
            schema: PreparedSchema::Type(ValueType::Integer),
        },
    ])
    .unwrap();
    for (bytes, names) in [
        (b"{}".as_slice(), vec![]),
        (br#"{"on":true}"#, vec!["on"]),
        (br#"{"secret":"x","level":70e-1}"#, vec!["secret", "level"]),
        (
            br#"{"\u006fn":false,"secret":"x","level":7.0}"#,
            vec!["on", "secret", "level"],
        ),
    ] {
        let selected =
            validate_property_write_map(&s, &Payload::new(bytes, "application/json")).unwrap();
        assert_eq!(selected, s.select(&names).unwrap());
    }
    for bytes in [
        br#"{"on":1}"#.as_slice(),
        br#"{"secret":true}"#,
        br#"{"level":7.1}"#,
        br#"{"on":true,"on":false}"#,
        br#"{"on":true,"\u006fn":false}"#,
        br#"{"on":true,"secret":"\uD800"}"#,
        br#"{"extra":null}"#,
        b"[]",
        b"{} {}",
    ] {
        assert!(validate_property_write_map(&s, &Payload::new(bytes, "application/json")).is_err());
    }
    let names: Vec<_> = (0..32).map(|i| format!("p{i}")).collect();
    let s = schema(&names.iter().map(String::as_str).collect::<Vec<_>>());
    let p = Payload::new(br#"{"p31":1,"p0":2}"#.as_slice(), "application/json");
    assert_eq!(
        validate_property_write_map(&s, &p).unwrap().bits(),
        0x80000001
    );
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
        schema: PreparedSchema::Any,
    });
    assert!(PropertyCollectionSchema::try_new(oversized_backing).is_err());
    let s = schema(&["level"]);
    assert!(check(&s, br#"{"\u006cevel":7}"#));
    assert!(!check(&s, br#"{"level":7,"\u006cevel":8}"#));
    assert!(
        PropertyCollectionSchema::try_new(vec![PropertyMemberSchema {
            name: "x".repeat(257).into(),
            schema: PreparedSchema::Any
        }])
        .is_err()
    );
    assert!(
        PropertyCollectionSchema::try_new(
            (0..33)
                .map(|i| PropertyMemberSchema {
                    name: format!("p{i}").into(),
                    schema: PreparedSchema::Any
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
                    schema: PreparedSchema::Any
                })
                .collect()
        )
        .is_err()
    );
}

#[test]
fn selection_round_trips_exact_names_and_refuses_duplicate_or_unreadable_input() {
    let s = schema(&["", "温度", "\"", "😀", "level"]);
    let selected = s.select(&["😀", "", "level"]).unwrap();
    let encoded = encode_property_selection(&s, selected).unwrap();
    assert_eq!(decode_property_selection(&s, &encoded).unwrap(), selected);
    let good = Payload::new(
        br#"{"":7,"\uD83D\uDE00":70e-1,"level":7.0}"#.as_slice(),
        "application/json",
    );
    assert!(validate_selected_property_map(&s, selected, &good).is_ok());
    assert!(validate_property_map(&s, &good).is_err());
    for bytes in [
        br#"["level","\u006cevel"]"#.as_slice(),
        br#"["secret"]"#,
        b"[1]",
        b"null",
        b"{}",
        br#"["\uD800"]"#,
        b"[] []",
    ] {
        assert!(decode_property_selection(&s, &Payload::new(bytes, "application/json")).is_err());
    }
    for bytes in [
        br#"{"level":7}"#.as_slice(),
        br#"{"":7,"\uD83D\uDE00":7,"level":7,"\u6e29\u5ea6":7}"#,
        br#"{"":7,"\uD83D\uDE00":7,"level":7,"\u006cevel":7}"#,
        br#"{"":7,"\uD83D\uDE00":7,"level":7e-100}"#,
    ] {
        assert!(
            validate_selected_property_map(&s, selected, &Payload::new(bytes, "application/json"))
                .is_err()
        );
    }
    assert!(s.select(&["level", "level"]).is_err());
    assert!(s.select(&["level"; 33]).is_err());
    let empty = s.select(&[]).unwrap();
    assert_eq!(
        encode_property_selection(&s, empty).unwrap().body.as_ref(),
        b"[]"
    );
    assert!(
        validate_selected_property_map(
            &s,
            empty,
            &Payload::new(b"{}".as_slice(), "application/json")
        )
        .is_ok()
    );
}

#[test]
fn maximal_selection_and_encoded_escaping_have_bounded_backing() {
    let names: Vec<_> = (0..32).map(|i| format!("p{i}")).collect();
    let s = schema(&names.iter().map(String::as_str).collect::<Vec<_>>());
    let selected = s
        .select(&names.iter().map(String::as_str).collect::<Vec<_>>())
        .unwrap();
    assert_eq!(selected.bits(), u32::MAX);
    assert_eq!(
        decode_property_selection(&s, &encode_property_selection(&s, selected).unwrap()).unwrap(),
        selected
    );
    assert!(encode_property_selection(&schema(&["p0"]), selected).is_err());
    // Names fit the static descriptor while their escaped request exceeds the
    // native body envelope. Encoding must fail within its funded buffer.
    let names: Vec<_> = (0..17)
        .map(|i| format!("{i:02}{}", "\n".repeat(238)))
        .collect();
    let s = schema(&names.iter().map(String::as_str).collect::<Vec<_>>());
    assert!(encode_property_selection(&s, s.all()).is_err());
}
