#![cfg(feature = "validated-thing")]
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
use validated_thing_schema_kernel_probe::{
    schema_arena as arena,
    schema_fields::Shape,
    schema_kernel::{Field, Rule},
};
use validated_thing_value_construction_probe::{Cursor, Limits, OwnedValue};

#[path = "../../validated-thing-value-construction/tests/support/mod.rs"]
#[allow(dead_code)]
mod construction;

thread_local! { static COUNT: Cell<Option<usize>> = const { Cell::new(None) }; }
struct Observer;
// SAFETY: forward unchanged Layouts and pointer pairs. The thread-local counter
// owns only one scalar and never allocates or observes another test's thread.
unsafe impl GlobalAlloc for Observer {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = COUNT.try_with(|count| {
            if let Some(n) = count.get() {
                count.set(Some(n + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let _ = COUNT.try_with(|count| {
            if let Some(n) = count.get() {
                count.set(Some(n + 1));
            }
        });
        unsafe { System.realloc(pointer, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: Observer = Observer;

fn literal(input: &str, step: u64) -> OwnedValue {
    construction::drive(Cursor::from_json(input.as_bytes(), Limits::default()), step).unwrap()
}

mod candidate {
    use validated_thing_schema_kernel_probe as td_crate;
    include!("support/schema_field_cases.rs");
}
mod unchanged {
    use clinkz_wot_td as td_crate;
    include!("support/schema_field_cases.rs");
}

#[test]
fn duplicates_are_resolved_before_dispatch_and_field_errors() {
    let overwritten = r#"{"type":17,"type":"number","type":"integer","minimum":"wrong","minimum":1,"unit":null,"unit":"last","const":17,"const":null,"title":17,"ti\u0074le":"last"}"#;
    let canonical = r#"{"type":"integer","minimum":1,"unit":"last","const":null,"title":"last"}"#;
    for step in [1, 7, 4096] {
        let first = literal(overwritten, step);
        let last = literal(canonical, step);
        assert_eq!(arena::validate(first.view()), arena::validate(last.view()));
        assert_eq!(
            first.footprint().retained_requested_bytes,
            last.footprint().retained_requested_bytes
        );
        let fields = arena::decode(first.view()).unwrap();
        let Shape::Integer(values) = fields.shape else {
            panic!("last type")
        };
        assert_eq!(values[0], Some(1));
        assert_eq!(fields.context.unit, Some("last"));
        assert_eq!(fields.context.metadata.title, Some("last"));
        assert_eq!(
            fields.context.constant.unwrap().kind(),
            validated_thing_value_construction_probe::Kind::Null
        );
        let document = format!(
            r#"{{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"probe","security":"none","securityDefinitions":{{"none":{{"scheme":"nosec"}}}},"properties":{{"p":{{"forms":[],{}}}}}}}"#,
            &overwritten[1..overwritten.len() - 1]
        );
        let typed: clinkz_wot_td::thing::Thing = serde_json::from_str(&document).unwrap();
        unchanged::fields(first.view(), &typed.properties.unwrap()["p"]._schema);
        let typed: validated_thing_schema_kernel_probe::thing::Thing =
            serde_json::from_str(&document).unwrap();
        candidate::fields(first.view(), &typed.properties.unwrap()["p"]._schema);
    }
    // Entire overwritten syntax still belongs to #119's construction cursor.
    assert!(
        construction::drive(
            Cursor::from_json(br#"{"unit":[true,],"unit":"last"}"#, Limits::default()),
            1
        )
        .is_err()
    );
}

#[test]
fn extracted_ordinary_adapter_preserves_existing_repeated_representation_conversions() {
    let mut accepted = 0;
    let mut rejected = 0;
    for key in [
        "$serde_json::private::RawValue",
        "$serde_json::private::Number",
        "ordinary",
    ] {
        for payload in [
            "true",
            "null",
            "[true,false]",
            "1e309",
            "true trailing",
            r#""converted""#,
        ] {
            let single = format!(
                "{{{}:{}}}",
                serde_json::to_string(key).unwrap(),
                serde_json::to_string(payload).unwrap()
            );
            for wrapper in [
                single.clone(),
                format!(
                    r#"{{"ordinarySecond":true,{}}}"#,
                    &single[1..single.len() - 1]
                ),
                format!(
                    r#"{{{},"ordinarySecond":true}}"#,
                    &single[1..single.len() - 1]
                ),
            ] {
                for input in [
                    format!(
                        r#""const":{wrapper},"default":{wrapper},"enum":[{wrapper}],"opaque":{wrapper}"#
                    ),
                    format!(r#""minimum":{wrapper},"maximum":1,"multipleOf":{wrapper}"#),
                    format!(r#""title":{wrapper},"unit":{wrapper}"#),
                    format!(r#""readOnly":{wrapper}"#),
                    format!(r#""type":{wrapper}"#),
                    format!(r#""type":"array","items":{{"const":{wrapper},"default":{wrapper}}}"#),
                    format!(
                        r#""properties":{{"nested":{{"const":{wrapper},"default":{wrapper}}}}}"#
                    ),
                    format!(r#""oneOf":[{{"const":{wrapper},"default":{wrapper}}}]"#),
                ] {
                    let actual = candidate::ordinary_observation(&input);
                    assert_eq!(actual, unchanged::ordinary_observation(&input), "{input}");
                    if actual.is_ok() {
                        accepted += 1;
                    } else {
                        rejected += 1;
                    }
                }
            }
        }
    }
    assert!(accepted > 0 && rejected > 0);
    assert_eq!(accepted + rejected, 432);
}

#[test]
fn literal_collision_family_stays_opaque_or_rejects_known_scalar_kinds() {
    use validated_thing_value_construction_probe::Kind;
    let payload = format!("[{}]", vec!["true"; 257].join(","));
    for key in [
        "$serde_json::private::RawValue",
        "$serde_json::private::Number",
        "ordinary",
    ] {
        for text in [
            "true",
            "null",
            "[true,false]",
            "1e309",
            "true trailing",
            &payload,
        ] {
            let wrapper = format!(
                "{{{}:{}}}",
                serde_json::to_string(key).unwrap(),
                serde_json::to_string(text).unwrap()
            );
            for prefix in ["", r#""ordinarySecond":true,"#] {
                let wrapper = format!("{{{prefix}{}}}", &wrapper[1..wrapper.len() - 1]);
                for input in [
                    format!(
                        r#"{{"const":{wrapper},"default":{wrapper},"enum":[{wrapper}],"opaque":{wrapper},"minimum":{wrapper},"maximum":1,"multipleOf":{wrapper}}}"#
                    ),
                    format!(
                        r#"{{"type":"array","items":{{"const":{wrapper},"opaque":{wrapper}}}}}"#
                    ),
                    format!(
                        r#"{{"properties":{{"nested":{{"const":{wrapper},"opaque":{wrapper}}}}}}}"#
                    ),
                    format!(r#"{{"oneOf":[{{"const":{wrapper},"opaque":{wrapper}}}]}}"#),
                ] {
                    let owner = literal(&input, 7);
                    arena::validate(owner.view()).unwrap();
                    let mut schema = arena::decode(owner.view()).unwrap();
                    if let Some(children) = schema.context.one_of {
                        schema = arena::decode(children.get(0).unwrap()).unwrap();
                    } else if let Shape::Array {
                        items: Some(children),
                        ..
                    } = schema.shape
                    {
                        schema = arena::decode(children.get(0).unwrap()).unwrap();
                    } else if let Shape::Object {
                        properties: Some(children),
                        ..
                    } = schema.shape
                    {
                        schema = arena::decode(children.member(0).unwrap().1).unwrap();
                    }
                    assert_eq!(schema.context.constant.unwrap().kind(), Kind::Object);
                    let opaque = schema.context.extras.get("opaque").unwrap();
                    assert_eq!(opaque.kind(), Kind::Object);
                    assert_eq!(opaque.get(key).unwrap().text(), Some(text));
                }
            }
            for field in ["title", "unit", "@type", "readOnly", "type"] {
                let input = format!(r#"{{"{field}":{wrapper}}}"#);
                assert!(
                    matches!(
                        arena::validate(literal(&input, 7).view()),
                        Err(arena::Invalid::Field(_))
                    ),
                    "{input}"
                );
            }
        }
    }
    // Both member orders and escaped spelling are still just associations.
    for input in [
        r#"{"const":{"$serde_json::private::RawValue":"true","ordinary":true}}"#,
        r#"{"const":{"ordinary":true,"\u0024serde_json::private::RawValue":"true"}}"#,
    ] {
        let owner = literal(input, 1);
        let value = arena::decode(owner.view())
            .unwrap()
            .context
            .constant
            .unwrap();
        assert_eq!(value.kind(), Kind::Object);
        assert_eq!(value.len(), 2);
    }
    // Repeated wrappers contain strings, including JSON-looking document text.
    let mut wrapper = r#"{"$serde_json::private::RawValue":"[true,false]"}"#.to_owned();
    for _ in 0..3 {
        let text = wrapper;
        wrapper = format!(
            r#"{{"$serde_json::private::RawValue":{}}}"#,
            serde_json::to_string(&text).unwrap()
        );
        let owner = literal(&format!(r#"{{"const":{wrapper}}}"#), 1);
        let value = arena::decode(owner.view())
            .unwrap()
            .context
            .constant
            .unwrap();
        assert_eq!(
            value.get("$serde_json::private::RawValue").unwrap().kind(),
            Kind::String
        );
        assert_eq!(
            value.get("$serde_json::private::RawValue").unwrap().text(),
            Some(text.as_str())
        );
    }
}

#[test]
fn shared_basic_runs_on_actual_field_decisions_and_complete_conversion_precedes_it() {
    for (input, expected) in [
        (
            r#"{"minimum":2,"maximum":1}"#,
            Rule::Ordered(Field::Minimum, Field::Maximum),
        ),
        (r#"{"multipleOf":0}"#, Rule::Positive(Field::MultipleOf)),
        (
            r#"{"multipleOf":1e309}"#,
            Rule::FailedProjection(Field::MultipleOf),
        ),
        (
            r#"{"type":"integer","minimum":9007199254740993,"maximum":9007199254740992}"#,
            Rule::Ordered(Field::Minimum, Field::Maximum),
        ),
        (
            r#"{"type":"array","items":{"type":"string","minLength":2,"maxLength":1}}"#,
            Rule::Ordered(Field::MinLength, Field::MaxLength),
        ),
    ] {
        let owner = literal(input, 1);
        let Err(arena::Invalid::Basic(error)) = arena::validate(owner.view()) else {
            panic!("{input}")
        };
        assert_eq!(error.rule, expected);
    }
    let owner = literal(
        r#"{"readOnly":true,"writeOnly":true,"oneOf":[{"unit":null}]}"#,
        1,
    );
    assert!(matches!(
        arena::validate(owner.view()),
        Err(arena::Invalid::Field(_))
    ));
}

#[test]
fn inspection_and_basic_add_no_allocations_or_owning_graph() {
    let long = "λ".repeat(4096);
    for input in [
        format!(
            r#"{{"type":"array","title":{},"items":{{"type":"object","properties":{{"x":{{"type":"number","minimum":1.25}}}}}},"const":1e309,"minimum":1}}"#,
            serde_json::to_string(&long).unwrap()
        ),
        r#"{"properties":{"x":{"unit":null}}}"#.into(),
        r#"{"type":"array","items":{"readOnly":true,"writeOnly":true}}"#.into(),
    ] {
        let owner = literal(&input, 7);
        let footprint = owner.footprint();
        COUNT.with(|count| count.set(Some(0)));
        let result = arena::validate(owner.view());
        let count = COUNT.with(|count| count.replace(None).unwrap());
        assert_eq!(count, 0, "{result:?}");
        assert_eq!(owner.footprint(), footprint);
    }
    assert!(!std::mem::needs_drop::<
        validated_thing_schema_kernel_probe::schema_fields::Decoded<arena::Source<'static>>,
    >());
    // This is an observed Host inline capacity, not a constrained stack claim.
    println!(
        "arena field facts: {} bytes",
        std::mem::size_of::<
            validated_thing_schema_kernel_probe::schema_fields::Decoded<arena::Source<'static>>,
        >()
    );
}
