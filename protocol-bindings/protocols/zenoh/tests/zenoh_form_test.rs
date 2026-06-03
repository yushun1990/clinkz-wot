use clinkz_wot_core::ProtocolBinding;
use clinkz_wot_protocol_bindings_zenoh::{
    CZ_ZENOH_KEY_EXPR, ZenohBinding, ZenohBindingError, extract_zenoh_target, is_zenoh_form,
};
use clinkz_wot_td::{data_type::Operation, form::Form, thing::Thing};
use serde_json::json;

#[test]
fn supports_forms_with_zenoh_href() {
    let form = Form::read_property("zenoh://clinkz/things/lamp/status")
        .build()
        .unwrap();
    let binding = ZenohBinding::new();

    assert!(is_zenoh_form(&form));
    assert!(binding.supports(&form, Operation::ReadProperty));
}

#[test]
fn supports_forms_with_explicit_key_expression_extension() {
    let form = Form::read_property("properties/status")
        .extra_field(CZ_ZENOH_KEY_EXPR, json!("clinkz/things/lamp/status"))
        .build()
        .unwrap();
    let binding = ZenohBinding::new();

    assert!(is_zenoh_form(&form));
    assert!(binding.supports(&form, Operation::ReadProperty));
}

#[test]
fn extracts_key_expression_from_zenoh_href() {
    let form = Form::read_property("zenoh://clinkz/things/lamp/status")
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp").nosec().build().unwrap();

    let target = extract_zenoh_target(&thing, &form).unwrap();

    assert_eq!(target.key_expr, "clinkz/things/lamp/status");
}

#[test]
fn explicit_key_expression_takes_precedence_over_href() {
    let form = Form::read_property("zenoh://fallback/key")
        .extra_field(CZ_ZENOH_KEY_EXPR, json!("clinkz/things/lamp/status"))
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp").nosec().build().unwrap();

    let target = extract_zenoh_target(&thing, &form).unwrap();

    assert_eq!(target.key_expr, "clinkz/things/lamp/status");
}

#[test]
fn rejects_non_string_key_expression_extension() {
    let form = Form::read_property("properties/status")
        .extra_field(CZ_ZENOH_KEY_EXPR, json!(42))
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp").nosec().build().unwrap();

    let err = extract_zenoh_target(&thing, &form).unwrap_err();

    assert_eq!(
        err,
        ZenohBindingError::Target("cz-zenoh:keyExpr must be a string".into())
    );
}

#[test]
fn does_not_support_unknown_operations_when_restricted() {
    let form = Form::write_property("zenoh://clinkz/things/lamp/status")
        .build()
        .unwrap();
    let binding = ZenohBinding::with_supported_operations([Operation::ReadProperty]);

    assert!(!binding.supports(&form, Operation::WriteProperty));
}
