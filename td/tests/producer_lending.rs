#![cfg(feature = "validated-thing")]
use clinkz_wot_foundation::{
    AdmissionLedger, GatewayDefaultV1, Generation, SlotIndex, StaticResourceProfile, WorkBudget,
    WorkClass,
};
use clinkz_wot_td::{
    ValidatedProducerEvent as Event, ValidatedProducerScope as Scope,
    ValidatedProducerStep as Step, ValidatedThing, ValidatedThingAdmissionConfig,
    ValidatedThingCursor, ValidatedThingProgress, thing::Thing,
};
use serde_json::json;
fn work(n: u64) -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, n))
}
fn proof(td: &Thing) -> ValidatedThing<'_> {
    let config = ValidatedThingAdmissionConfig::try_from_limits(&GatewayDefaultV1::LIMITS).unwrap();
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
            ValidatedThingProgress::Complete(p) => return p,
            ValidatedThingProgress::Failed(e) => panic!("{e:?}"),
        };
    }
    panic!("validation stalled")
}
fn td() -> Thing {
    serde_json::from_value(json!({"@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"whole Producer",
      "base":"http://127.0.0.1:38080/base/", "securityDefinitions":{"none":{"scheme":"nosec"}}, "security":["none"],
      "forms":[{"href":"../all", "op":["readallproperties"]}],
      "properties":{
        "a":{"type":"number","forms":[{"href":"a"},{"href":"a2","op":[]}]},
        "b":{"type":"number","writeOnly":true,"forms":[{"href":"b","security":[]}]},
        "c":{"type":"number","readOnly":true,"forms":[{"href":"c"}]},
        "empty":{"type":"number","forms":[]}},
      "actions":{"run":{"forms":[{"href":"run"}]}}, "events":{"alarm":{"forms":[{"href":"alarm"}]}}
    })).unwrap()
}
#[test]
fn all_scopes_forms_empty_sets_and_defaults_survive_without_read_filtering() {
    use clinkz_wot_td::data_type::Operation::*;
    let td = td();
    let mut cursor = proof(&td).into_producer();
    let mut scopes = Vec::new();
    let mut forms = Vec::new();
    for _ in 0..10_000 {
        match cursor.step(&mut work(1000), false).unwrap() {
            Step::Pending => continue,
            Step::Done => break,
            Step::Ready(Event::Scope { scope, forms }) => {
                scopes.push((format!("{scope:?}"), forms))
            }
            Step::Ready(Event::Form(f)) => {
                forms.push((
                    format!("{:?}", f.scope),
                    f.index,
                    f.effective_operations.to_vec(),
                    f.resolved_href.to_owned(),
                    f.declared_operations.is_some(),
                    f.effective_security.len(),
                ));
            }
        }
        cursor.acknowledge();
    }
    assert!(matches!(
        cursor.step(&mut work(1000), false).unwrap(),
        Step::Done
    ));
    assert_eq!(scopes.len(), 7);
    assert!(scopes.contains(&("Property(\"empty\")".into(), Some(0))));
    assert_eq!(
        forms.iter().map(|f| f.2.clone()).collect::<Vec<_>>(),
        vec![
            vec![ReadAllProperties],
            vec![ReadProperty, WriteProperty],
            vec![],
            vec![WriteProperty],
            vec![ReadProperty],
            vec![InvokeAction],
            vec![SubscribeEvent, UnsubscribeEvent]
        ]
    );
    assert_eq!(forms[0].3, "http://127.0.0.1:38080/all");
    assert_eq!(forms[1].3, "http://127.0.0.1:38080/base/a");
    assert!(forms[2].4);
    assert!(!forms[3].4);
    assert_eq!(forms[3].5, 0);
}
#[test]
fn zero_credit_stable_loans_and_cancellation_do_not_skip_declarations() {
    let td = td();
    let mut cursor = proof(&td).into_producer();
    for _ in 0..10 {
        assert!(matches!(
            cursor.step(&mut WorkBudget::new(), false).unwrap(),
            Step::Pending
        ));
    }
    assert!(matches!(
        cursor.step(&mut work(1000), false).unwrap(),
        Step::Pending
    ));
    for _ in 0..2 {
        assert!(matches!(
            cursor.step(&mut work(1000), false).unwrap(),
            Step::Ready(Event::Scope {
                scope: Scope::Thing,
                forms: Some(1)
            })
        ));
    }
    assert!(cursor.step(&mut WorkBudget::new(), true).is_err());
    assert!(cursor.step(&mut work(1000), false).is_err());
}
