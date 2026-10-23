#![cfg(feature = "prepare")]
//! Production semantic checkpoint; no mock server/handler or native driver.
use clinkz_wot_binding_http::{HttpProducerConfig, HttpProducerProgram};
use clinkz_wot_foundation::{
    AdmissionLedger, GatewayDefaultV1, Generation, SlotIndex, StaticResourceProfile, WorkBudget,
    WorkClass,
};
use clinkz_wot_planning::{ProducerPropertyReadAdmission, ProducerPropertyReadStep};
use clinkz_wot_td::{
    ValidatedThingAdmissionConfig, ValidatedThingCursor, ValidatedThingProgress, thing::Thing,
};
use serde_json::{Value, json};
fn work(n: u64) -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, n))
}
fn document() -> Value {
    json!({"@context":["https://www.w3.org/2022/wot/td/v1.1", {"htv":"http://www.w3.org/2011/http#"}],
       "title":"Producer", "securityDefinitions":{"none":{"scheme":"nosec"},"basic":{"scheme":"basic"}}, "security":["none"],
       "properties":{"level":{"type":"number", "readOnly":true,"forms":[{"href":"http://127.0.0.1:38080/level"}]}}})
}
fn admission<'a, 'h>(td: &'a Thing, handler: &'h str) -> ProducerPropertyReadAdmission<'a, 'h> {
    let config = ValidatedThingAdmissionConfig::try_from_limits(GatewayDefaultV1::LIMITS).unwrap();
    let ledger = AdmissionLedger::new(
        SlotIndex::new(1),
        Generation::INITIAL,
        0,
        8 << 20,
        0,
        0,
        0,
        0,
    );
    let mut cursor = ValidatedThingCursor::from_thing(td, &config, ledger);
    for _ in 0..100_000 {
        cursor = match cursor.step(&mut work(1_000_000), false) {
            ValidatedThingProgress::Pending(c) => c,
            ValidatedThingProgress::Complete(p) => {
                return ProducerPropertyReadAdmission::new(p, handler);
            }
            ValidatedThingProgress::Failed(e) => panic!("Basic admission failed: {e:?}"),
        };
    }
    panic!("Basic stalled")
}
fn prepare(value: Value, handler: &str) -> Result<HttpProducerProgram, clinkz_wot_core::CoreError> {
    let td: Thing = serde_json::from_value(value).unwrap();
    let mut build = admission(&td, handler);
    for _ in 0..100_000 {
        if build.step(&mut work(1_000_000), false)? == ProducerPropertyReadStep::Covered {
            let config = HttpProducerConfig::for_loopback("127.0.0.1:38080".parse().unwrap())?;
            let program = build
                .with_input(&mut work(1_000_000), |input, budget| {
                    config.compile_property_read(input, budget)
                })?
                .unwrap();
            drop(build);
            drop(td);
            return Ok(program);
        }
    }
    panic!("coverage stalled")
}
#[test]
fn default_explicit_and_aliased_get_compile_without_id_or_nonempty_key_and_outlive_source() {
    let p = prepare(document(), "level").unwrap();
    assert_eq!(p.target(), "http://127.0.0.1:38080/level");
    assert_eq!(p.authority(), "127.0.0.1:38080");
    assert_eq!(p.path_and_query(), "/level");
    let mut value = document();
    value["@context"][1]["method"] = json!("htv:methodName");
    value["@context"][1]["status"] = json!("htv:statusCodeValue");
    value["@context"][1]["ex"] = json!("https://example.org/");
    value["@type"] = json!(["ex:Thing"]);
    value["properties"]["level"]["@type"] = json!(["ex:Reading"]);
    value["base"] = json!("http://127.0.0.1:38080/base/");
    value["properties"]["level"]["forms"][0] = json!({"href":"../level?q=1", "op":["readproperty"], "method":"GET", "response":{"contentType":"application/json", "status":200}});
    // Explicit read op supports writable schema too; effective op owns coverage.
    value["properties"]["level"]
        .as_object_mut()
        .unwrap()
        .remove("readOnly");
    let property = value["properties"]
        .as_object_mut()
        .unwrap()
        .remove("level")
        .unwrap();
    value["properties"][""] = property;
    assert_eq!(prepare(value, "").unwrap().path_and_query(), "/level?q=1");
}
#[test]
fn thirteen_old_lost_semantics_and_uncovered_siblings_refuse_the_whole_draft() {
    for (label, field, value) in [
        ("post", "htv:methodName", json!("POST")),
        (
            "vendor",
            "https://vendor.example/requireConfirmation",
            json!(true),
        ),
        ("media", "response", json!({"contentType":"text/plain"})),
        (
            "status",
            "response",
            json!({"contentType":"application/json","htv:statusCodeValue":201}),
        ),
        ("coding", "contentCoding", json!("gzip")),
        ("scopes", "scopes", json!([])),
        ("form-basic", "security", json!(["basic"])),
        ("root-basic", "root-security", json!(["basic"])),
        ("second-form", "second-form", Value::Null),
        ("second-property", "second-property", Value::Null),
        ("readwrite", "readwrite", Value::Null),
        ("writeonly", "writeonly", Value::Null),
        ("profile", "profile", json!("urn:unsupported:profile")),
        ("action", "action", Value::Null),
        ("event", "event", Value::Null),
        (
            "root-form",
            "forms",
            json!([{"href":"http://127.0.0.1:38080/all", "op":["readallproperties"]}]),
        ),
        ("root-empty", "forms", json!([])),
        ("empty-op", "op", json!([])),
        (
            "read-and-write",
            "op",
            json!(["readproperty", "writeproperty"]),
        ),
        ("no-security", "security", json!([])),
        ("headers", "htv:headers", json!([])),
        ("additional", "additionalResponses", json!([])),
        ("subprotocol", "subprotocol", json!("other")),
    ] {
        let mut td = document();
        match field {
            "root-security" => td["security"] = value,
            "profile" => td["profile"] = value,
            "forms" => td["forms"] = value,
            "second-form" => {
                let form = td["properties"]["level"]["forms"][0].clone();
                td["properties"]["level"]["forms"]
                    .as_array_mut()
                    .unwrap()
                    .push(form);
            }
            "second-property" => {
                td["properties"]["other"] = json!({"type":"number","readOnly":true,"forms":[]})
            }
            "readwrite" => {
                td["properties"]["level"]
                    .as_object_mut()
                    .unwrap()
                    .remove("readOnly");
            }
            "writeonly" => {
                td["properties"]["level"]
                    .as_object_mut()
                    .unwrap()
                    .remove("readOnly");
                td["properties"]["level"]["writeOnly"] = json!(true);
            }
            "action" => td["actions"] = json!({"run":{"forms":[]}}),
            "event" => td["events"] = json!({"alarm":{"forms":[]}}),
            _ => td["properties"]["level"]["forms"][0][field] = value,
        }
        assert!(prepare(td, "level").is_err(), "{label}");
    }
    assert!(prepare(document(), "wrong-handler").is_err());
}
#[test]
fn inherited_security_can_be_overridden_but_proxy_and_unknown_scoped_instructions_refuse() {
    let mut td = document();
    td["security"] = json!(["basic"]);
    td["properties"]["level"]["forms"][0]["security"] = json!(["none"]);
    assert!(prepare(td, "level").is_ok());
    for scope in ["root", "property", "security", "response", "nested-schema"] {
        let mut td = document();
        let instruction = "https://vendor.example/instruction";
        match scope {
            "root" => td[instruction] = json!(true),
            "property" => td["properties"]["level"][instruction] = json!(true),
            "security" => td["securityDefinitions"]["none"][instruction] = json!(true),
            "response" => {
                td["properties"]["level"]["forms"][0]["response"] =
                    json!({"contentType":"application/json", instruction:true})
            }
            _ => {
                td["properties"]["level"]["type"] = json!("object");
                td["properties"]["level"]["properties"] =
                    json!({"child":{"type":"number", instruction:true}});
            }
        }
        assert!(prepare(td, "level").is_err(), "{scope}");
    }
    let mut td = document();
    td["securityDefinitions"]["none"]["proxy"] = json!("http://127.0.0.1:9");
    assert!(prepare(td, "level").is_err());
}
#[test]
fn zero_credit_missing_coverage_cancel_and_listen_mismatch_never_issue_a_program() {
    let td: Thing = serde_json::from_value(document()).unwrap();
    let mut build = admission(&td, "level");
    assert!(
        build
            .with_input::<()>(&mut work(1_000_000), |_, _| panic!("uncovered callback"))
            .is_err()
    );
    for _ in 0..10 {
        assert_eq!(
            build.step(&mut WorkBudget::new(), false).unwrap(),
            ProducerPropertyReadStep::Pending
        );
    }
    assert!(build.step(&mut WorkBudget::new(), true).is_err());
    assert!(build.step(&mut work(1_000_000), false).is_err());
    let mut build = admission(&td, "level");
    while build.step(&mut work(1_000_000), false).unwrap() != ProducerPropertyReadStep::Covered {}
    assert!(
        build
            .with_input::<()>(&mut WorkBudget::new(), |_, _| panic!("unpaid callback"))
            .unwrap()
            .is_none()
    );
    let config = HttpProducerConfig::for_loopback("127.0.0.1:38080".parse().unwrap()).unwrap();
    let mut no_lowering = work(1_000_000);
    no_lowering.set_remaining(WorkClass::BindingPolls, 0);
    assert!(
        build
            .with_input(&mut no_lowering, |input, budget| config
                .compile_property_read(input, budget))
            .unwrap()
            .is_none()
    );
    let config = HttpProducerConfig::for_loopback("127.0.0.1:38081".parse().unwrap()).unwrap();
    assert!(
        build
            .with_input(&mut work(1_000_000), |input, budget| config
                .compile_property_read(input, budget))
            .is_err()
    );
    assert!(HttpProducerConfig::for_loopback("127.0.0.1:0".parse().unwrap()).is_err());
    assert!(HttpProducerConfig::for_loopback("192.0.2.1:80".parse().unwrap()).is_err());
}

#[test]
fn standard_nested_schema_is_supported_but_extension_aliases_cannot_replace_typed_branches() {
    let mut td = document();
    td["properties"]["level"]["type"] = json!("object");
    td["properties"]["level"]["properties"] =
        json!({"values":{"type":"array","items":{"type":"number"}}});
    assert!(prepare(td.clone(), "level").is_ok());
    td["@context"][1]["shadow"] = json!("https://www.w3.org/2019/wot/json-schema#properties");
    td["properties"]["level"]["shadow"] = json!({});
    assert!(prepare(td, "level").is_err());
}

#[test]
fn concrete_http_target_limit_is_exact_and_does_not_change_td_coverage() {
    let prefix = "http://127.0.0.1:38080/";
    for size in [64, 65] {
        let mut td = document();
        td["properties"]["level"]["forms"][0]["href"] =
            json!(format!("{prefix}{}", "a".repeat(size - prefix.len())));
        assert_eq!(prepare(td, "level").is_ok(), size == 64);
    }
    let mut td = document();
    td["properties"]["level"]["forms"][0]["href"] = json!("http://127.0.0.1:38080/other#fragment");
    assert!(prepare(td, "level").is_err());
}
