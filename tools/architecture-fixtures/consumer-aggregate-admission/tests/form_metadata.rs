//! Pre-code HTTP admission discriminators, not a production eligibility proof.
//! Reuse typed TD validation/lending and the existing closed Core compiler.
use clinkz_wot_core::{
    BindingArtifactRole, BindingCandidate, BindingCompilerExtension, BindingCompilerInput,
    BindingGeneration, BindingId, LogicalInteractionPlan, PlanId, ResolvedTargetCompiler, ThingId,
};
use clinkz_wot_foundation::{
    AdmissionLedger, GatewayDefaultV1, Generation, SlotIndex, StaticResourceProfile, WorkBudget,
    WorkClass as W,
};
use clinkz_wot_td::{
    ValidatedPropertyReadEvent as Event, ValidatedPropertyReadStep as Step,
    ValidatedThingAdmissionConfig, ValidatedThingCursor, ValidatedThingProgress as Progress,
    form::Form, thing::Thing,
};
use serde_json::{Value, json};

fn budget(n: u64) -> WorkBudget {
    W::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, n))
}
fn source(fields: Value) -> Thing {
    let mut form = json!({"href":"http://127.0.0.1:8080/a"});
    form.as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    serde_json::from_value(json!({
        "@context":["https://www.w3.org/2022/wot/td/v1.1", {"htv":"http://www.w3.org/2011/http#"}],
        "title":"metadata discriminator", "id":"urn:metadata",
        "security":["none"], "securityDefinitions":{"none":{"scheme":"nosec"}},
        "properties":{"a":{"forms":[form]}}
    }))
    .unwrap()
}
fn form(t: &Thing) -> &Form {
    &t.properties.as_ref().unwrap()["a"]._interaction.forms[0]
}

// Proposed scalar only. This test-side projection has raw Form access; the
// future production getter must store the paid result before lending Ready.
// It must not copy this source lookup into a binding/compiler callback.
fn proposed_paid_fact(f: &Form, credit: &mut WorkBudget) -> Option<bool> {
    credit.consume(W::DocumentNodes, 1).ok()?;
    Some(
        !f._extra_fields.is_empty()
            || f.response.is_some()
            || f.additional_responses.is_some()
            || f.content_coding.is_some()
            || f.subprotocol.is_some()
            || f.scopes.is_some(),
    )
}
fn restricted_support(metadata: Option<bool>, media: Option<&str>) -> bool {
    metadata == Some(false) && media == Some("application/json")
}

// Actual current public TD proof/lending -> current owned Core input. This
// deliberately has no proposed getter or modified product source available.
fn current_plan(t: &Thing) -> LogicalInteractionPlan {
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
    let mut c = ValidatedThingCursor::from_thing(t, &config, ledger);
    let proof = loop {
        match c.step(&mut budget(10_000), false) {
            Progress::Pending(next) => c = next,
            Progress::Complete(p) => break p,
            Progress::Failed(cause) => panic!("validation: {cause:?}"),
        }
    };
    let mut td = proof.into_property_read();
    for _ in 0..100_000 {
        match td.step(&mut budget(10_000), false).unwrap() {
            Step::Ready(Event::Form(f)) => {
                return LogicalInteractionPlan::try_property_read(
                    PlanId::new(SlotIndex::new(0), Generation::INITIAL),
                    ThingId::from("urn:metadata"),
                    f.property_name().into(),
                    f.original_index(),
                    f.resolved_href().into(),
                    Some(f.content_type().into()),
                    f.subprotocol().map(Into::into),
                )
                .unwrap();
            }
            Step::Ready(Event::Property { .. }) => td.acknowledge(),
            Step::Pending => {}
            Step::Done => panic!("missing readable form"),
        }
    }
    panic!("lending did not progress")
}
fn unprojected_cases() -> Vec<Value> {
    vec![
        json!({"htv:methodName":"GET"}),
        json!({"htv:methodName":"POST"}),
        json!({"htv:headers":[{"htv:fieldName":"X-Test","htv:fieldValue":"value"}]}),
        json!({"unknown:metadata":{"nested":[1,2,3]}}),
        json!({"response":{"contentType":"text/plain"}}),
        json!({"response":{"contentType":"application/json"}}),
        json!({"additionalResponses":[]}),
        json!({"additionalResponses":[{"success":false,"contentType":"text/plain"}]}),
    ]
}

#[test]
fn current_lending_and_closed_compiler_lose_observable_metadata() {
    let baseline = current_plan(&source(json!({})));
    let compiler = ResolvedTargetCompiler::try_new(64).unwrap();
    let candidate = BindingCandidate::new(
        BindingId::new(1),
        BindingGeneration::INITIAL,
        compiler.configuration(),
        compiler.compatibility(),
        0,
        0,
    );
    for fields in unprojected_cases() {
        let t = source(fields);
        let plan = current_plan(&t);
        assert_eq!(
            baseline, plan,
            "current compiler input cannot distinguish this Form"
        );
        assert!(
            compiler
                .bounds(&BindingCompilerInput::new(
                    &plan,
                    candidate,
                    BindingArtifactRole::ConsumerCall
                ))
                .is_ok()
        );
        assert_eq!(proposed_paid_fact(form(&t), &mut budget(1)), Some(true));
    }
}

#[test]
fn proposed_fact_is_paid_conservative_and_preserves_roundtrips() {
    let mut cases = vec![(json!({}), false)];
    cases.extend(unprojected_cases().into_iter().map(|v| (v, true)));
    cases.extend([
        (json!({"contentCoding":"identity"}), true),
        (json!({"subprotocol":""}), true),
        (json!({"scopes":[]}), true),
        (json!({"scopes":["scope"]}), true),
    ]);
    for (fields, expected) in cases {
        let t = source(fields);
        let restored: Thing = serde_json::from_value(serde_json::to_value(&t).unwrap()).unwrap();
        assert_eq!(
            form(&restored),
            form(&t),
            "preserve typed fields/extensions"
        );
        let mut credit = budget(0);
        assert_eq!(proposed_paid_fact(form(&t), &mut credit), None);
        assert_eq!(credit.remaining(W::DocumentNodes), 0);
        let mut credit = budget(1);
        let fact = proposed_paid_fact(form(&t), &mut credit);
        assert_eq!(fact, Some(expected));
        assert_eq!(credit.remaining(W::DocumentNodes), 0);
        // An owned scalar remains usable without source or further source work.
        drop(restored);
        drop(t);
        for _ in 0..3 {
            assert_eq!(fact, Some(expected));
            assert_eq!(credit.remaining(W::DocumentNodes), 0);
        }
    }
}

#[test]
fn proposed_restriction_fails_closed_without_protocol_interpretation() {
    assert!(restricted_support(Some(false), Some("application/json")));
    for metadata in [None, Some(true)] {
        assert!(!restricted_support(metadata, Some("application/json")));
    }
    for media in [
        None,
        Some("text/plain"),
        Some("application/json;charset=utf-8"),
    ] {
        assert!(!restricted_support(Some(false), media));
    }
    let t = source(json!({"href":"mock://peer/a"}));
    let plan = current_plan(&t);
    let fact = proposed_paid_fact(form(&t), &mut budget(1));
    assert_eq!(plan.resolved_target(), "mock://peer/a");
    drop(t);
    assert!(restricted_support(fact, plan.content_type()));
}
