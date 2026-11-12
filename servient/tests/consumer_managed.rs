#![cfg(all(feature = "std", target_has_atomic = "ptr"))]
use clinkz_wot_core::{CoreError, host_accounts::HostResourceAccounts};
use clinkz_wot_foundation::*;
use clinkz_wot_servient::consumer::register_thing;
use clinkz_wot_servient_finite::consumer::ConsumerOwner;
use clinkz_wot_td::{thing::Thing, validate::Validate};
use serde_json::json;
use std::sync::Arc;

#[test]
fn anonymous_td_is_valid_but_id_registration_requires_an_explicit_unique_id() {
    let mut thing: Thing = serde_json::from_value(json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"pump",
        "security":["n"], "securityDefinitions":{"n":{"scheme":"nosec"}},
        "properties":{"pressure":{"type":"integer","readOnly":true,
            "forms":[{"href":"http://127.0.0.1/pressure"}]}}
    }))
    .unwrap();
    thing.validate().unwrap();
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let owner = ConsumerOwner::try_new(bank.clone(), 1, 0).unwrap();
    let mut budget = WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, 10000));
    let before = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    assert!(matches!(
        register_thing(&owner, &thing, &mut budget),
        Err(CoreError::Validation(_))
    ));
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        before
    );
    thing.id = Some(clinkz_wot_td::data_type::AbsoluteUri::parse("urn:pump").unwrap());
    drop(register_thing(&owner, &thing, &mut budget).unwrap());
    assert!(matches!(
        register_thing(&owner, &thing, &mut budget),
        Err(CoreError::Validation(_))
    ));
    drop(thing);
    owner.remove_thing("urn:pump", &mut budget).unwrap();
    drop(owner);
    for kind in ResourceKind::ALL {
        assert_eq!(bank.account(kind).used(), 0, "{kind:?}");
    }
}
