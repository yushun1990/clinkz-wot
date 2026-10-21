//! Production paid TD facts and closed compiler eligibility, without HTTP I/O.
use clinkz_wot_core::{
    BindingArtifactRole, BindingCandidate, BindingConfigurationDigest, BindingGeneration,
    BindingId, BindingResourceContract, ConsumerBindingConfiguration, CoreError,
    LogicalInteractionPlan, PlanId, ThingId,
};
use clinkz_wot_foundation::{
    AdmissionLedger, GatewayDefaultV1, Generation, SlotIndex, StaticResourceProfile, WorkBudget,
    WorkClass as W,
};
use clinkz_wot_planning::{binding_compiler::*, binding_registration::*};
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

fn lending(t: &Thing) -> clinkz_wot_td::ValidatedPropertyReadCursor<'_> {
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
    proof.into_property_read()
}
fn paid_plan(t: &Thing) -> LogicalInteractionPlan {
    let mut td = lending(t);
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
                .unwrap()
                .with_additional_form_metadata(f.has_additional_form_metadata());
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
fn paid_metadata_distinguishes_forms_and_restricted_compiler_rejects_them() {
    let baseline = paid_plan(&source(json!({})));
    assert_eq!(baseline.has_additional_form_metadata(), Some(false));
    let compiler = restricted_compiler();
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
        let plan = paid_plan(&t);
        assert_ne!(baseline, plan);
        assert_eq!(plan.has_additional_form_metadata(), Some(true));
        let input = BindingCompilerInput::new(&plan, candidate, BindingArtifactRole::ConsumerCall);
        assert!(matches!(
            compiler.bounds(&input),
            Err(CoreError::UnsupportedOperation(_))
        ));
        assert!(matches!(
            compiler.start(&input),
            Err(CoreError::UnsupportedOperation(_))
        ));
    }
}

#[test]
fn production_fact_is_paid_conservative_and_preserves_roundtrips() {
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
        let mut td = lending(&t);
        for pass in 0..2 {
            let mut ready = false;
            for _ in 0..100_000 {
                assert!(matches!(
                    td.step(&mut budget(0), false).unwrap(),
                    Step::Pending
                ));
                let mut credit = budget(10_000);
                match td.step(&mut credit, false).unwrap() {
                    Step::Ready(Event::Property { .. }) => td.acknowledge(),
                    Step::Ready(Event::Form(f)) => {
                        assert_eq!(f.has_additional_form_metadata(), expected);
                        assert_eq!(credit.remaining(W::DocumentNodes), 9_999);
                        ready = true;
                        break;
                    }
                    Step::Pending => {}
                    Step::Done => panic!("missing Form on pass {pass}"),
                }
            }
            assert!(ready);
            for _ in 0..3 {
                let mut credit = budget(0);
                let Step::Ready(Event::Form(f)) = td.step(&mut credit, false).unwrap() else {
                    panic!("lost Ready");
                };
                assert_eq!(f.has_additional_form_metadata(), expected);
                assert!(credit.is_exhausted());
            }
            td.acknowledge();
            while !matches!(td.step(&mut budget(10_000), false).unwrap(), Step::Done) {}
            td = td.rewind();
        }
        drop(td);
        let plan = paid_plan(&t);
        drop(restored);
        drop(t);
        for _ in 0..3 {
            assert_eq!(plan.has_additional_form_metadata(), Some(expected));
        }
    }
}

fn restricted_compiler() -> ResolvedTargetCompiler {
    ResolvedTargetCompiler::new_owned(
        ConsumerBindingConfiguration::try_new(
            BindingResourceContract::ConsumerOwnedV2,
            64,
            BindingConfigurationDigest::new([19; 32]),
            4096,
            32,
        )
        .unwrap()
        .with_default_json_forms()
        .unwrap(),
    )
}

#[test]
fn restriction_fails_closed_without_protocol_interpretation() {
    let t = source(json!({"href":"mock://peer/a"}));
    let plan = paid_plan(&t);
    assert_eq!(plan.resolved_target(), "mock://peer/a");
    drop(t);
    let compiler = restricted_compiler();
    let candidate = BindingCandidate::new(
        BindingId::new(1),
        BindingGeneration::INITIAL,
        compiler.configuration(),
        compiler.compatibility(),
        0,
        0,
    );
    assert!(
        compiler
            .bounds(&BindingCompilerInput::new(
                &plan,
                candidate,
                BindingArtifactRole::ConsumerCall,
            ))
            .is_ok()
    );
    for metadata in [None, Some(false), Some(true)] {
        for media in [
            None,
            Some("application/json"),
            Some("text/plain"),
            Some("application/json;charset=utf-8"),
        ] {
            let mut plan = LogicalInteractionPlan::try_property_read(
                plan.plan_id(),
                ThingId::from("urn:metadata"),
                "a".into(),
                0,
                "mock://peer/a".into(),
                media.map(Into::into),
                None,
            )
            .unwrap();
            assert_eq!(plan.has_additional_form_metadata(), None);
            if let Some(present) = metadata {
                plan = plan.with_additional_form_metadata(present);
            }
            let result = compiler.bounds(&BindingCompilerInput::new(
                &plan,
                candidate,
                BindingArtifactRole::ConsumerCall,
            ));
            assert_eq!(
                result.is_ok(),
                metadata == Some(false) && media == Some("application/json")
            );
        }
    }
}
