//! Fixed Basic boundary mutations, consumed by the real-source candidate and
//! both typed/Snapshot adapters. Expected outcomes do not call the new kernel.
extern crate alloc;
use super::td_crate::{
    data_schema::{DataSchema, NullSchema},
    data_type::Operation,
    form::Form,
    security_scheme::{SecurityScheme, SecuritySchemeContext},
    thing::Thing,
};
use alloc::{collections::BTreeMap, format, string::String, vec, vec::Vec};

pub struct First {
    pub owner: &'static str,
    pub ordinal: usize,
    pub field: &'static str,
    pub index: usize,
    pub member: usize,
}

pub struct Case {
    pub label: String,
    pub thing: Thing,
    pub valid: bool,
    pub first: Option<First>,
}

fn base() -> Thing {
    serde_json::from_str(r#"{
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"Basic boundary",
        "security":["none"], "securityDefinitions":{"none":{"scheme":"nosec"},"other":{"scheme":"nosec"}},
        "properties":{"a":{"type":"null","forms":[{"href":"p/0"},{"href":"p/1"}]}},
        "actions":{"a":{"forms":[{"href":"a/0"},{"href":"a/1"}]}},
        "events":{"a":{"forms":[{"href":"e/0"},{"href":"e/1"}]}},
        "forms":[{"href":"all/0"},{"href":"all/1"}]
    }"#).unwrap()
}

fn invalid_schema() -> DataSchema {
    let mut value = NullSchema::default();
    value._context.read_only = true;
    value._context.write_only = true;
    DataSchema::Null(value)
}

fn schema_map() -> BTreeMap<String, DataSchema> {
    BTreeMap::from([("a".into(), invalid_schema())])
}

fn forms(thing: &mut Thing, kind: usize) -> &mut [Form] {
    match kind {
        0 => {
            &mut thing
                .properties
                .as_mut()
                .unwrap()
                .get_mut("a")
                .unwrap()
                ._interaction
                .forms
        }
        1 => {
            &mut thing
                .actions
                .as_mut()
                .unwrap()
                .get_mut("a")
                .unwrap()
                ._interaction
                .forms
        }
        2 => {
            &mut thing
                .events
                .as_mut()
                .unwrap()
                .get_mut("a")
                .unwrap()
                ._interaction
                .forms
        }
        3 => thing.forms.as_mut().unwrap(),
        _ => unreachable!(),
    }
}

fn context_mut(scheme: &mut SecurityScheme) -> &mut SecuritySchemeContext {
    match scheme {
        SecurityScheme::NoSec(v) => &mut v._context,
        SecurityScheme::Auto(v) => &mut v._context,
        SecurityScheme::Combo(v) => &mut v._context,
        SecurityScheme::Basic(v) => &mut v._context,
        SecurityScheme::Digest(v) => &mut v._context,
        SecurityScheme::APIKey(v) => &mut v._context,
        SecurityScheme::Bearer(v) => &mut v._context,
        SecurityScheme::PSK(v) => &mut v._context,
        SecurityScheme::OAuth2(v) => &mut v._context,
    }
}

fn add(
    cases: &mut Vec<Case>,
    label: impl Into<String>,
    valid: bool,
    mutate: impl FnOnce(&mut Thing),
) {
    let mut thing = base();
    mutate(&mut thing);
    cases.push(Case {
        label: label.into(),
        thing,
        valid,
        first: None,
    });
}

// Earlier faults are repaired one at a time. Every suffix still contains all
// later faults, so each row falsifies a cross-phase precedence inversion.
const LADDER: [(&str, &str); 24] = [
    ("Thing", "Title"),
    ("Thing", "Security"),
    ("SecurityDefinition", "Scheme"),
    ("SecurityDefinition", "OneOf"),
    ("Thing", "SchemaDefinitions"),
    ("Thing", "UriVariables"),
    ("Property", "PropertySchema"),
    ("Property", "UriVariables"),
    ("Property", "FormOperation"),
    ("Property", "FormSecurity"),
    ("Action", "UriVariables"),
    ("Action", "Input"),
    ("Action", "Output"),
    ("Action", "FormOperation"),
    ("Action", "FormSecurity"),
    ("Event", "UriVariables"),
    ("Event", "Subscription"),
    ("Event", "Data"),
    ("Event", "DataResponse"),
    ("Event", "Cancellation"),
    ("Event", "FormOperation"),
    ("Event", "FormSecurity"),
    ("Thing", "FormSecurity"),
    ("Thing", "FormOperation"),
];

fn fault(thing: &mut Thing, stage: usize) {
    match stage {
        0 => thing._metadata.title = None,
        1 => thing.security = vec!["none".into(), "missing-root".into()],
        2 => {
            thing.security_definitions.insert(
                "a".into(),
                SecurityScheme::combo_one_of(Vec::<String>::new()),
            );
        }
        3 => {
            thing.security_definitions.insert(
                "z".into(),
                serde_json::from_str(r#"{"scheme":"combo","oneOf":["none","missing-combo"]}"#)
                    .unwrap(),
            );
        }
        4 => thing.schema_definitions = Some(schema_map()),
        5 => thing.uri_variables = Some(schema_map()),
        6 => {
            thing
                .properties
                .as_mut()
                .unwrap()
                .get_mut("a")
                .unwrap()
                ._schema = invalid_schema()
        }
        7 => {
            thing
                .properties
                .as_mut()
                .unwrap()
                .get_mut("a")
                .unwrap()
                ._interaction
                .uri_variables = Some(schema_map())
        }
        8 | 13 | 20 => {
            let kind = if stage == 8 {
                0
            } else if stage == 13 {
                1
            } else {
                2
            };
            forms(thing, kind)[1].op = Some(vec![Operation::ReadAllProperties]);
        }
        9 | 14 | 21 => {
            let kind = if stage == 9 {
                0
            } else if stage == 14 {
                1
            } else {
                2
            };
            forms(thing, kind)[0].security = Some(vec!["none".into(), "missing-form".into()]);
        }
        10 => {
            thing
                .actions
                .as_mut()
                .unwrap()
                .get_mut("a")
                .unwrap()
                ._interaction
                .uri_variables = Some(schema_map())
        }
        11 => thing.actions.as_mut().unwrap().get_mut("a").unwrap().input = Some(invalid_schema()),
        12 => thing.actions.as_mut().unwrap().get_mut("a").unwrap().output = Some(invalid_schema()),
        15 => {
            thing
                .events
                .as_mut()
                .unwrap()
                .get_mut("a")
                .unwrap()
                ._interaction
                .uri_variables = Some(schema_map())
        }
        16 => {
            thing
                .events
                .as_mut()
                .unwrap()
                .get_mut("a")
                .unwrap()
                .subscription = Some(invalid_schema())
        }
        17 => thing.events.as_mut().unwrap().get_mut("a").unwrap().data = Some(invalid_schema()),
        18 => {
            thing
                .events
                .as_mut()
                .unwrap()
                .get_mut("a")
                .unwrap()
                .data_response = Some(invalid_schema())
        }
        19 => {
            thing
                .events
                .as_mut()
                .unwrap()
                .get_mut("a")
                .unwrap()
                .cancellation = Some(invalid_schema())
        }
        22 => forms(thing, 3)[1].security = Some(vec!["missing-root-form".into()]),
        23 => forms(thing, 3)[0].op = Some(vec![Operation::ReadProperty]),
        _ => unreachable!(),
    }
}

pub fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for start in 0..=LADDER.len() {
        let mut thing = base();
        for stage in start..LADDER.len() {
            fault(&mut thing, stage);
        }
        let first = LADDER.get(start).map(|&(owner, field)| First {
            owner,
            ordinal: if start == 3 { 2 } else { 0 },
            field,
            index: if [8, 13, 20, 22].contains(&start) {
                1
            } else {
                0
            },
            member: if [1, 3, 9, 14, 21].contains(&start) {
                1
            } else {
                0
            },
        });
        cases.push(Case {
            label: format!("first-error suffix {start}"),
            thing,
            valid: first.is_none(),
            first,
        });
    }
    add(&mut cases, "empty title", false, |t| {
        t._metadata.title = Some("".into())
    });
    add(
        &mut cases,
        "whitespace title remains Basic-valid",
        true,
        |t| t._metadata.title = Some(" ".into()),
    );
    add(&mut cases, "empty root security", false, |t| {
        t.security.clear()
    });
    add(
        &mut cases,
        "missing root reference in ordered names",
        false,
        |t| t.security = vec!["none".into(), "missing".into(), "absent".into()],
    );
    add(
        &mut cases,
        "Basic does not require id, interactions or WoT context",
        true,
        |t| {
            t.id = None;
            t.context = serde_json::from_str(r#""https://example.org/extension-only""#).unwrap();
            t.properties = None;
            t.actions = None;
            t.events = None;
            t.forms = None;
        },
    );
    add(
        &mut cases,
        "Basic leaves additional-response references lenient",
        true,
        |t| {
            forms(t, 0)[0].additional_responses = Some(vec![
                serde_json::from_str(r#"{"schema":"missing"}"#).unwrap(),
            ]);
        },
    );

    for (input, valid) in [
        (r#"{"scheme":"combo"}"#, false),
        (r#"{"scheme":"combo","oneOf":["none"]}"#, false),
        (r#"{"scheme":"combo","allOf":["none"]}"#, false),
        (r#"{"scheme":"combo","oneOf":["none",""]}"#, false),
        (r#"{"scheme":"combo","allOf":["","none"]}"#, false),
        (
            r#"{"scheme":"combo","oneOf":["none","missing"],"allOf":["none"]}"#,
            false,
        ),
        (
            r#"{"scheme":"combo","oneOf":["none","other"],"allOf":["missing","none"]}"#,
            false,
        ),
        (
            r#"{"scheme":"combo","oneOf":["none","other"],"allOf":["other","none"]}"#,
            true,
        ),
        (r#"{"scheme":"combo","oneOf":["none","none"]}"#, true),
        (r#"{"scheme":"combo","oneOf":["s","none"]}"#, true),
        (r#"{"scheme":"apikey"}"#, false),
        (r#"{"scheme":"apikey","name":""}"#, false),
        (r#"{"scheme":"apikey","name":"key"}"#, true),
        (r#"{"scheme":"oauth2","flow":"code"}"#, false),
        (
            r#"{"scheme":"oauth2","flow":"code","authorization":"https://a.example/"}"#,
            false,
        ),
        (
            r#"{"scheme":"oauth2","flow":"code","authorization":"https://a.example/","token":"https://a.example/token"}"#,
            true,
        ),
        (r#"{"scheme":"oauth2","flow":"client"}"#, true),
        (r#"{"scheme":"oauth2","flow":"device"}"#, true),
        (r#"{"scheme":"oauth2","flow":"implicit"}"#, false),
    ] {
        add(&mut cases, input, valid, |t| {
            t.security_definitions
                .insert("s".into(), serde_json::from_str(input).unwrap());
        });
    }
    for discriminator in [
        "unknown", "combo", "apikey", "oauth2", "nosec", "basic", "digest", "bearer", "psk", "auto",
    ] {
        for extra in [
            serde_json::json!({}),
            serde_json::json!({"oneOf":["none",false,"other",7],"name":"key","flow":"code","authorization":"not a URI","token":"present"}),
        ] {
            let valid = match discriminator {
                "unknown" => false,
                "combo" | "apikey" | "oauth2" => !extra.as_object().unwrap().is_empty(),
                _ => true,
            };
            add(
                &mut cases,
                format!("mutable Auto discriminator {discriminator}/{extra}"),
                valid,
                |t| {
                    let mut scheme = SecurityScheme::auto();
                    let context = context_mut(&mut scheme);
                    context.scheme = discriminator.into();
                    context._extra_fields = extra
                        .as_object()
                        .unwrap()
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    t.security_definitions.insert("s".into(), scheme);
                },
            );
        }
    }
    for (extra, valid) in [
        (serde_json::json!("none"), false),
        (serde_json::json!([false, 7]), false),
        (serde_json::json!(["none", "other", null]), true),
    ] {
        add(
            &mut cases,
            format!("filtered combo fallback {extra}"),
            valid,
            |t| {
                let scheme = t.security_definitions.get_mut("none").unwrap();
                let context = context_mut(scheme);
                context.scheme = "combo".into();
                context._extra_fields.insert("oneOf".into(), extra);
            },
        );
    }
    add(
        &mut cases,
        "typed API key absence wins over extension name",
        false,
        |t| {
            let mut scheme = SecurityScheme::apikey("");
            context_mut(&mut scheme)
                ._extra_fields
                .insert("name".into(), "extension-name".into());
            t.security_definitions.insert("s".into(), scheme);
        },
    );
    add(
        &mut cases,
        "mutable Combo switched to NoSec ignores member fields",
        true,
        |t| {
            let mut scheme = SecurityScheme::combo_one_of(Vec::<String>::new());
            context_mut(&mut scheme).scheme = "nosec".into();
            t.security_definitions.insert("s".into(), scheme);
        },
    );
    add(
        &mut cases,
        "typed Combo fields take precedence over extension members",
        false,
        |t| {
            let mut scheme = SecurityScheme::combo_one_of(Vec::<String>::new());
            context_mut(&mut scheme)
                ._extra_fields
                .insert("oneOf".into(), serde_json::json!(["none", "other"]));
            t.security_definitions.insert("s".into(), scheme);
        },
    );
    add(
        &mut cases,
        "typed OAuth flow takes precedence over extension flow",
        false,
        |t| {
            let mut scheme = SecurityScheme::oauth2("unsupported");
            context_mut(&mut scheme)
                ._extra_fields
                .insert("flow".into(), "client".into());
            t.security_definitions.insert("s".into(), scheme);
        },
    );
    add(
        &mut cases,
        "typed OAuth endpoint absence takes precedence over extension endpoint",
        false,
        |t| {
            let mut scheme = SecurityScheme::oauth2("code");
            context_mut(&mut scheme)
                ._extra_fields
                .insert("authorization".into(), "present".into());
            context_mut(&mut scheme)
                ._extra_fields
                .insert("token".into(), "present".into());
            t.security_definitions.insert("s".into(), scheme);
        },
    );
    add(
        &mut cases,
        "earlier security reference precedes later definition shape",
        false,
        |t| {
            t.security_definitions.insert(
                "a".into(),
                serde_json::from_str(r#"{"scheme":"combo","oneOf":["none","missing-a"]}"#).unwrap(),
            );
            t.security_definitions.insert(
                "z".into(),
                SecurityScheme::combo_one_of(Vec::<String>::new()),
            );
        },
    );
    add(
        &mut cases,
        "oneOf references precede allOf references",
        false,
        |t| {
            t.security_definitions.insert("s".into(), serde_json::from_str(r#"{"scheme":"combo","oneOf":["none","missing-one"],"allOf":["other","missing-all"]}"#).unwrap());
        },
    );
    add(
        &mut cases,
        "empty root reference resolves to an empty definition key",
        true,
        |t| {
            t.security_definitions
                .insert("".into(), SecurityScheme::nosec());
            t.security = vec!["".into()];
        },
    );
    add(
        &mut cases,
        "Basic accepts affordances with empty forms",
        true,
        |t| {
            t.properties
                .as_mut()
                .unwrap()
                .get_mut("a")
                .unwrap()
                ._interaction
                .forms
                .clear();
            t.actions
                .as_mut()
                .unwrap()
                .get_mut("a")
                .unwrap()
                ._interaction
                .forms
                .clear();
            t.events
                .as_mut()
                .unwrap()
                .get_mut("a")
                .unwrap()
                ._interaction
                .forms
                .clear();
        },
    );
    // A frozen independent vocabulary table: Property [0,4), Action [4,7),
    // Event [7,9), root meta-operations [9,18).
    use Operation::*;
    let vocabulary = [
        ReadProperty,
        WriteProperty,
        ObserveProperty,
        UnobserveProperty,
        InvokeAction,
        QueryAction,
        CancelAction,
        SubscribeEvent,
        UnsubscribeEvent,
        ReadAllProperties,
        WriteAllProperties,
        ReadMultipleProperties,
        WriteMultipleProperties,
        ObserveAllProperties,
        UnobserveAllProperties,
        QueryAllActions,
        SubscribeAllEvents,
        UnsubscribeAllEvents,
    ];
    for (kind, range) in [(0, 0..4), (1, 4..7), (2, 7..9), (3, 9..18)] {
        for (index, op) in vocabulary.iter().enumerate() {
            add(
                &mut cases,
                format!("operation {kind}/{op:?}"),
                range.contains(&index),
                |t| forms(t, kind)[1].op = Some(vec![*op]),
            );
        }
        for explicit in [None, Some(vec![])] {
            add(
                &mut cases,
                format!("empty/absent operations {kind}/{explicit:?}"),
                true,
                |t| forms(t, kind)[0].op = explicit,
            );
        }
        for explicit in [
            None,
            Some(vec![]),
            Some(vec!["none".into()]),
            Some(vec!["missing".into()]),
        ] {
            let valid = explicit
                .as_ref()
                .is_none_or(|v| v.is_empty() || v[0] == "none");
            add(
                &mut cases,
                format!("form security {kind}/{explicit:?}"),
                valid,
                |t| forms(t, kind)[1].security = explicit,
            );
        }
    }
    add(
        &mut cases,
        "earlier Property Form reference precedes later Property schema",
        false,
        |t| {
            forms(t, 0)[1].security = Some(vec!["missing".into()]);
            let mut later = t.properties.as_ref().unwrap()["a"].clone();
            later._schema = invalid_schema();
            t.properties.as_mut().unwrap().insert("z".into(), later);
        },
    );
    add(
        &mut cases,
        "semantic map order is independent of insertion order",
        false,
        |t| {
            let mut later = t.properties.as_ref().unwrap()["a"].clone();
            later._schema = invalid_schema();
            let mut earlier = later.clone();
            earlier._schema =
                serde_json::from_str(r#"{"type":"array","minItems":2,"maxItems":1}"#).unwrap();
            t.properties = Some(BTreeMap::from([("z".into(), later), ("a".into(), earlier)]));
        },
    );
    cases
}
