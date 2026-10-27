#![cfg(all(feature = "std", feature = "validated-thing"))]
use clinkz_wot_core::{
    BindingGeneration, BindingId, Operation, ThingSlotId, producer::ProducerInstanceAuthority,
};
use clinkz_wot_foundation::{
    GatewayDefaultV1, Generation, SlotIndex, StaticResourceProfile, WorkBudget, WorkClass,
};
use clinkz_wot_planning::producer_composition::*;
use clinkz_wot_td::{
    ValidatedProducerScope,
    semantic::{FormOwner, SemanticScope},
    validate::{Validate, ValidationLevel},
};
use std::{any::Any, boxed::Box};

fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, k| b.with_remaining(k, 1_000_000))
}
fn key() -> clinkz_wot_core::producer::ProducerInstanceKey {
    ProducerInstanceAuthority::try_new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL))
        .unwrap()
        .key()
}
fn td() -> String {
    serde_json::json!({"@context":["https://www.w3.org/2022/wot/td/v1.1", {"ex":"urn:test:","method":"htv:methodName"}],
        "title":"many", "security":["n"],"securityDefinitions":{"n":{"scheme":"nosec"},"unused":{"scheme":"custom", "ex:option":1}},
        "properties":{"p":{"type":"number","readOnly":true,"forms":[{"href":"http://127.0.0.1:1234/p"},{"href":"http://127.0.0.1:1235/p"}]}},
        "actions":{"run":{"input":{"type":"string"},"output":{"type":"string"},"forms":[
          {"href":"http://127.0.0.1:1236/run","method":"POST"},
          {"href":"http://127.0.0.1:1236/status","op":"queryaction"},
          {"href":"http://127.0.0.1:1236/stop","op":"cancelaction"}]}},
        "events":{"alarm":{"data":{"type":"number"},"forms":[{"href":"http://127.0.0.1:1237/events","op":["subscribeevent","unsubscribeevent"]}]}},
        "forms":[{"href":"http://127.0.0.1:1238/all", "op":"readallproperties"}]
    }).to_string()
}
struct Provider(u32, &'static str);
impl ProducerCapability for Provider {
    fn identity(&self) -> (BindingId, BindingGeneration) {
        (BindingId::new(self.0), BindingGeneration::INITIAL)
    }
    fn definition_bytes(&self, definition: &dyn Any) -> Option<usize> {
        definition
            .downcast_ref::<String>()
            .map(|s| core::mem::size_of::<String>() + s.capacity())
    }
    fn prepare(
        &self,
        input: ProducerOperationInput<'_>,
        _: &mut WorkBudget,
    ) -> clinkz_wot_core::CoreResult<Option<Box<dyn Any>>> {
        if match (&self.1, input.site) {
            (&"property", FormSite::Property(_)) => true,
            (&"action", FormSite::Action(_)) => true,
            (&"event", FormSite::Event(_)) => true,
            (&"collection", FormSite::Thing) => true,
            _ => false,
        } {
            let facts = input.document.form(input.site.owner(), input.form).unwrap();
            assert!(facts.effective_operations.contains(&input.operation));
            assert_eq!(facts.effective_security, ["n"]);
            if self.1 == "action" {
                assert_eq!(input.document.action_synchronous("run"), Some(None));
                assert!(
                    input
                        .document
                        .affordance_schema(FormOwner::Action("run"), "input")
                        .is_some()
                );
                if input.form == 0 {
                    assert!(
                        input
                            .document
                            .scope(SemanticScope::AffordanceForm {
                                owner: FormOwner::Action("run"),
                                index: 0
                            })
                            .unwrap()
                            .field("http://www.w3.org/2011/http#methodName")
                            .is_some()
                    );
                }
            }
            if self.1 == "event" {
                assert!(
                    input
                        .document
                        .affordance_schema(FormOwner::Event("alarm"), "data")
                        .is_some()
                );
            }
            if self.1 == "collection" {
                assert!(input.document.scope(SemanticScope::Thing).is_some());
            }
            return Ok(Some(Box::new(input.target.to_owned())));
        }
        Ok(None)
    }
    fn check_final(
        &self,
        input: ProducerOperationInput<'_>,
        definition: &dyn Any,
        budget: &mut WorkBudget,
    ) -> clinkz_wot_core::CoreResult<bool> {
        Ok(self
            .prepare(input, budget)?
            .and_then(|d| d.downcast::<String>().ok())
            .as_deref()
            == definition.downcast_ref::<String>())
    }
}
fn covered(_: ValidatedProducerScope<'_>, _: Operation) -> bool {
    true
}

#[test]
fn each_scope_form_and_effective_operation_selects_actual_registration() {
    let providers = [
        Provider(1, "property"),
        Provider(2, "action"),
        Provider(3, "event"),
        Provider(4, "collection"),
    ];
    let registrations: Vec<&dyn ProducerCapability> = providers
        .iter()
        .map(|p| p as &dyn ProducerCapability)
        .collect();
    let text = td();
    let prepared = prepare_thing(
        &text,
        key(),
        &GatewayDefaultV1::LIMITS,
        &mut work(),
        &registrations,
        covered,
    )
    .unwrap();
    assert_eq!(prepared.operations().len(), 8);
    assert!(
        prepared
            .operations()
            .iter()
            .any(|o| o.site == FormSite::Thing && o.operation == Operation::ReadAllProperties)
    );
    assert_eq!(
        prepared
            .operations()
            .iter()
            .filter(|o| o.site == FormSite::Action("run".into()))
            .count(),
        3
    );
    assert_eq!(
        prepared
            .operations()
            .iter()
            .filter(|o| o.site == FormSite::Event("alarm".into()))
            .count(),
        2
    );
    check_final_thing(
        &text,
        key(),
        &GatewayDefaultV1::LIMITS,
        &mut work(),
        &registrations,
        &prepared,
        covered,
    )
    .unwrap();
    for mutation in ["queryaction", "unsubscribeevent", "readallproperties"] {
        let value = match mutation {
            "queryaction" => text.replace("queryaction", "invokeaction"),
            "unsubscribeevent" => text.replace("unsubscribeevent", "subscribeevent"),
            _ => text.replace("readallproperties", "queryallactions"),
        };
        assert!(
            check_final_thing(
                &value,
                key(),
                &GatewayDefaultV1::LIMITS,
                &mut work(),
                &registrations,
                &prepared,
                covered
            )
            .is_err()
        );
    }
}

#[test]
fn absent_ambiguous_and_unregistered_controls_refuse_purely() {
    let p = Provider(1, "property");
    let a = Provider(2, "action");
    let text = td();
    assert!(
        prepare_thing(
            &text,
            key(),
            &GatewayDefaultV1::LIMITS,
            &mut work(),
            &[&p, &a],
            covered
        )
        .is_err()
    );
    let e = Provider(3, "event");
    let c = Provider(4, "collection");
    assert!(
        prepare_thing(
            &text,
            key(),
            &GatewayDefaultV1::LIMITS,
            &mut work(),
            &[&p, &a, &e, &c, &p],
            covered
        )
        .is_err()
    );
    assert!(
        prepare_thing(
            &text,
            key(),
            &GatewayDefaultV1::LIMITS,
            &mut work(),
            &[&p, &a, &e, &c],
            |scope, op| !(matches!(scope, ValidatedProducerScope::Action(_))
                && op == Operation::CancelAction)
        )
        .is_err()
    );
    let no = text.replace(
        "\"n\":{\"scheme\":\"nosec\"}",
        "\"n\":{\"scheme\":\"custom\"}",
    );
    // TD preservation is not support. A real provider must refuse the selected
    // extension scheme; this fixture's permissive provider is not a security service.
    assert_ne!(no, text);
    let extended: clinkz_wot_td::thing::Thing = serde_json::from_str(&no).unwrap();
    assert!(extended.validate_with_level(ValidationLevel::Basic).is_ok());
}
