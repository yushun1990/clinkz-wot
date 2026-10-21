#![cfg(feature = "async")]

use clinkz_wot_planning::{AffordanceRef, FormSelectionCriteria};
use clinkz_wot_protocol_bindings_zenoh::{
    CZ_ZENOH_CONGESTION_CONTROL, CZ_ZENOH_PRIORITY, CZ_ZENOH_QOS, ZenohBindingError,
    ZenohOperationKind, extract_zenoh_metadata, extract_zenoh_target, is_zenoh_form_target,
    plan_zenoh_affordance_operation, plan_zenoh_affordance_operation_with_criteria,
    plan_zenoh_operation, zenoh_operation_kind,
};
use clinkz_wot_td::{
    affordance::{EventAffordance, InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    data_type::Operation,
    form::Form,
    thing::Thing,
};
use serde_json::json;

#[tokio::test]
async fn extracts_key_expression_from_zenoh_href() {
    let form = Form::read_property("zenoh://clinkz/things/lamp/status")
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp").nosec().build().unwrap();

    let target = extract_zenoh_target(&thing, &form).unwrap();

    assert_eq!(target.transport, "tcp");
    assert_eq!(target.authority, "clinkz");
    assert_eq!(target.key_expr, "things/lamp/status");
}

#[tokio::test]
async fn extracts_transport_suffix_from_scheme() {
    let udp_form = Form::read_property("zenoh+udp://router-a:7447/things/lamp/status")
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp").nosec().build().unwrap();

    let target = extract_zenoh_target(&thing, &udp_form).unwrap();

    assert_eq!(target.transport, "udp");
    assert_eq!(target.authority, "router-a:7447");
    assert_eq!(target.key_expr, "things/lamp/status");

    let tcp_form = Form::read_property("zenoh+tcp://router-a:7447/things/lamp/status")
        .build()
        .unwrap();
    let target = extract_zenoh_target(&thing, &tcp_form).unwrap();
    assert_eq!(target.transport, "tcp");
}

#[tokio::test]
async fn rejects_empty_authority() {
    let form = Form::read_property("zenoh:///things/lamp/status")
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp").nosec().build().unwrap();

    let err = extract_zenoh_target(&thing, &form).unwrap_err();

    assert!(matches!(err, ZenohBindingError::MissingAuthority(_)));
}

#[tokio::test]
async fn builds_operation_plan_from_href_resolved_against_base() {
    let form = Form::invoke_action("actions/reboot").build().unwrap();
    let thing = Thing::builder("Lamp")
        .base("zenoh://clinkz/things/lamp/")
        .nosec()
        .build()
        .unwrap();

    let plan = plan_zenoh_operation(&thing, &form, Operation::InvokeAction).unwrap();

    assert_eq!(plan.transport, "tcp");
    assert_eq!(plan.authority, "clinkz");
    assert_eq!(plan.key_expr, "things/lamp/actions/reboot");
    assert_eq!(plan.kind, ZenohOperationKind::RequestReply);
    assert_eq!(
        plan.metadata,
        clinkz_wot_protocol_bindings_zenoh::ZenohFormMetadata {
            content_type: Some("application/json".into()),
            ..Default::default()
        }
    );
}

#[tokio::test]
async fn extracts_zenoh_metadata_extensions() {
    let form = Form::write_property("zenoh://clinkz/things/lamp/status")
        .content_type("application/json")
        .extra_field(CZ_ZENOH_QOS, json!("express"))
        .extra_field(CZ_ZENOH_PRIORITY, json!("real-time"))
        .extra_field(CZ_ZENOH_CONGESTION_CONTROL, json!("block"))
        .build()
        .unwrap();

    let metadata = extract_zenoh_metadata(&form).unwrap();

    assert_eq!(metadata.content_type.as_deref(), Some("application/json"));
    assert_eq!(metadata.qos.as_deref(), Some("express"));
    assert_eq!(metadata.priority.as_deref(), Some("real-time"));
    assert_eq!(metadata.congestion_control.as_deref(), Some("block"));
}

#[tokio::test]
async fn includes_metadata_in_operation_plan() {
    let form = Form::write_property("zenoh://clinkz/things/lamp/status")
        .content_type("application/json")
        .extra_field(CZ_ZENOH_QOS, json!("express"))
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp").nosec().build().unwrap();

    let plan = plan_zenoh_operation(&thing, &form, Operation::WriteProperty).unwrap();

    assert_eq!(plan.key_expr, "things/lamp/status");
    assert_eq!(plan.kind, ZenohOperationKind::Put);
    assert_eq!(
        plan.metadata.content_type.as_deref(),
        Some("application/json")
    );
    assert_eq!(plan.metadata.qos.as_deref(), Some("express"));
}

#[tokio::test]
async fn plans_zenoh_affordance_operation_from_matching_form() {
    let http_form = Form::read_property("https://example.com/things/lamp/properties/status")
        .build()
        .unwrap();
    let zenoh_form = Form::read_property("zenoh://clinkz/things/lamp/properties/status")
        .content_type("application/json")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .forms([http_form, zenoh_form])
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .nosec()
        .property("status", property)
        .build()
        .unwrap();

    let plan = plan_zenoh_affordance_operation(
        &thing,
        AffordanceRef::Property("status"),
        Operation::ReadProperty,
    )
    .unwrap();

    assert_eq!(plan.affordance, AffordanceRef::Property("status"));
    assert_eq!(plan.form_index, 1);
    assert_eq!(plan.operation.kind, ZenohOperationKind::Query);
    assert_eq!(plan.operation.key_expr, "things/lamp/properties/status");
    assert_eq!(
        plan.operation.metadata.content_type.as_deref(),
        Some("application/json")
    );
}

#[tokio::test]
async fn plans_zenoh_affordance_operation_with_metadata_criteria() {
    let json_form = Form::read_property("zenoh://clinkz/things/lamp/properties/status/json")
        .content_type("application/json")
        .build()
        .unwrap();
    let cbor_form = Form::read_property("zenoh://clinkz/things/lamp/properties/status/cbor")
        .content_type("application/cbor")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .forms([json_form, cbor_form])
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .nosec()
        .property("status", property)
        .build()
        .unwrap();

    let plan = plan_zenoh_affordance_operation_with_criteria(
        &thing,
        AffordanceRef::Property("status"),
        FormSelectionCriteria::new(Operation::ReadProperty).content_type("application/cbor"),
    )
    .unwrap();

    assert_eq!(plan.form_index, 1);
    assert_eq!(
        plan.operation.key_expr,
        "things/lamp/properties/status/cbor"
    );
    assert_eq!(
        plan.operation.metadata.content_type.as_deref(),
        Some("application/cbor")
    );
}

#[tokio::test]
async fn plans_zenoh_affordance_operation_with_subprotocol_criteria() {
    let longpoll_form =
        Form::subscribe_event("zenoh://clinkz/things/lamp/events/status-change/longpoll")
            .content_type("application/json")
            .subprotocol("longpoll")
            .build()
            .unwrap();
    let sse_form = Form::subscribe_event("zenoh://clinkz/things/lamp/events/status-change/sse")
        .content_type("text/event-stream")
        .subprotocol("sse")
        .build()
        .unwrap();
    let event = EventAffordance::builder()
        .forms([longpoll_form, sse_form])
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .nosec()
        .event("status-change", event)
        .build()
        .unwrap();

    let plan = plan_zenoh_affordance_operation_with_criteria(
        &thing,
        AffordanceRef::Event("status-change"),
        FormSelectionCriteria::new(Operation::SubscribeEvent).subprotocol("sse"),
    )
    .unwrap();

    assert_eq!(plan.form_index, 1);
    assert_eq!(plan.operation.kind, ZenohOperationKind::Subscribe);
    assert_eq!(
        plan.operation.key_expr,
        "things/lamp/events/status-change/sse"
    );
}

#[tokio::test]
async fn plans_thing_level_zenoh_form() {
    let form = Form::builder("zenoh://clinkz/things/lamp")
        .op([Operation::ReadAllProperties])
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp").nosec().form(form).build().unwrap();

    let plan =
        plan_zenoh_affordance_operation(&thing, AffordanceRef::Thing, Operation::ReadAllProperties)
            .unwrap();

    assert_eq!(plan.affordance, AffordanceRef::Thing);
    assert_eq!(plan.form_index, 0);
    assert_eq!(plan.operation.kind, ZenohOperationKind::Query);
    assert_eq!(plan.operation.key_expr, "things/lamp");
}

#[tokio::test]
async fn plans_bulk_property_operation_from_thing_level_form() {
    let read_form = Form::builder("zenoh://clinkz/things/lamp/properties")
        .op([Operation::ReadAllProperties])
        .build()
        .unwrap();
    let write_form = Form::builder("zenoh://clinkz/things/lamp/properties")
        .op([Operation::WriteMultipleProperties])
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .nosec()
        .forms([read_form, write_form])
        .build()
        .unwrap();

    let plan = plan_zenoh_affordance_operation(
        &thing,
        AffordanceRef::Thing,
        Operation::WriteMultipleProperties,
    )
    .unwrap();

    assert_eq!(plan.form_index, 1);
    assert_eq!(plan.operation.kind, ZenohOperationKind::Put);
    assert_eq!(plan.operation.key_expr, "things/lamp/properties");
}

/// `subscribeallevents` / `unsubscribeallevents` are TD 1.1 event
/// meta-operations; this planning test covers the Thing-level bulk form path.
#[tokio::test]
async fn plans_bulk_event_operation_from_thing_level_form() {
    let subscribe_form = Form::builder("zenoh://clinkz/things/lamp/events")
        .op([Operation::SubscribeAllEvents])
        .build()
        .unwrap();
    let unsubscribe_form = Form::builder("zenoh://clinkz/things/lamp/events")
        .op([Operation::UnsubscribeAllEvents])
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .nosec()
        .forms([subscribe_form, unsubscribe_form])
        .build()
        .unwrap();

    let plan = plan_zenoh_affordance_operation(
        &thing,
        AffordanceRef::Thing,
        Operation::UnsubscribeAllEvents,
    )
    .unwrap();

    assert_eq!(plan.form_index, 1);
    assert_eq!(plan.operation.kind, ZenohOperationKind::Unsubscribe);
    assert_eq!(plan.operation.key_expr, "things/lamp/events");
}

#[tokio::test]
async fn plans_relative_href_against_zenoh_base() {
    let form = Form::read_property("properties/status").build().unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(form.clone())
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .base("zenoh://clinkz/things/lamp/")
        .nosec()
        .property("status", property)
        .build()
        .unwrap();

    assert!(is_zenoh_form_target(&thing, &form));

    let plan = plan_zenoh_affordance_operation(
        &thing,
        AffordanceRef::Property("status"),
        Operation::ReadProperty,
    )
    .unwrap();

    assert_eq!(plan.operation.key_expr, "things/lamp/properties/status");
    assert_eq!(plan.operation.kind, ZenohOperationKind::Query);
}

#[tokio::test]
async fn rejects_relative_href_when_base_is_invalid() {
    let form = Form::read_property("properties/status").build().unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(form.clone())
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .base("https://example.com/{tenant}/")
        .nosec()
        .property("status", property)
        .build()
        .unwrap();

    let err = plan_zenoh_affordance_operation(
        &thing,
        AffordanceRef::Property("status"),
        Operation::ReadProperty,
    )
    .unwrap_err();

    assert!(matches!(
        err,
        ZenohBindingError::Target(clinkz_wot_td::data_type::ResolveFormHrefError::TemplateBase(_))
    ));
}

#[tokio::test]
async fn plans_operations_from_clinkz_extension_fixture() {
    let thing: Thing = serde_json::from_str(include_str!(
        "../../../../td/tests/fixtures/clinkz-extension-defaults.td.jsonld"
    ))
    .expect("fixture TD should deserialize");

    let read = plan_zenoh_affordance_operation(
        &thing,
        AffordanceRef::Property("status"),
        Operation::ReadProperty,
    )
    .unwrap();
    assert_eq!(read.form_index, 0);
    assert_eq!(read.operation.kind, ZenohOperationKind::Query);
    assert_eq!(
        read.operation.key_expr,
        "things/targeted-roundtrip/properties/status"
    );

    let write = plan_zenoh_affordance_operation_with_criteria(
        &thing,
        AffordanceRef::Property("status"),
        FormSelectionCriteria::new(Operation::WriteProperty).content_type("application/cbor"),
    )
    .unwrap();
    assert_eq!(write.form_index, 1);
    assert_eq!(write.operation.kind, ZenohOperationKind::Put);
    assert_eq!(
        write.operation.key_expr,
        "things/targeted-roundtrip/properties/status"
    );

    let unsubscribe = plan_zenoh_affordance_operation(
        &thing,
        AffordanceRef::Event("alarm"),
        Operation::UnsubscribeEvent,
    )
    .unwrap();
    assert_eq!(unsubscribe.form_index, 1);
    assert_eq!(unsubscribe.operation.kind, ZenohOperationKind::Unsubscribe);
    assert_eq!(
        unsubscribe.operation.key_expr,
        "things/targeted-roundtrip/events/alarm"
    );

    let read_all =
        plan_zenoh_affordance_operation(&thing, AffordanceRef::Thing, Operation::ReadAllProperties)
            .unwrap();
    assert_eq!(read_all.form_index, 0);
    assert_eq!(read_all.operation.kind, ZenohOperationKind::Query);
    assert_eq!(
        read_all.operation.key_expr,
        "things/targeted-roundtrip/properties"
    );
}

#[tokio::test]
async fn reports_selection_error_when_affordance_has_no_zenoh_form() {
    let form = Form::read_property("https://example.com/things/lamp/properties/status")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(form)
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .nosec()
        .property("status", property)
        .build()
        .unwrap();

    let err = plan_zenoh_affordance_operation(
        &thing,
        AffordanceRef::Property("status"),
        Operation::ReadProperty,
    )
    .unwrap_err();

    assert_eq!(
        err,
        ZenohBindingError::Shared(clinkz_wot_planning::BindingError::CallerFilterMismatch(
            "No form matches FormSelectionCriteria { operation: ReadProperty, content_type: None, subprotocol: None } after applying caller filter".into()
        ))
    );
}

#[tokio::test]
async fn rejects_non_string_zenoh_metadata_extension() {
    let form = Form::read_property("zenoh://clinkz/things/lamp/status")
        .extra_field(CZ_ZENOH_QOS, json!(true))
        .build()
        .unwrap();

    let err = extract_zenoh_metadata(&form).unwrap_err();

    assert_eq!(
        err,
        ZenohBindingError::InvalidExtension {
            term: CZ_ZENOH_QOS,
            message: "must be a string".into()
        }
    );
}

#[tokio::test]
async fn extracts_default_content_type_into_zenoh_metadata() {
    let form = Form::read_property("zenoh://clinkz/things/lamp/status")
        .build()
        .unwrap();

    let metadata = extract_zenoh_metadata(&form).unwrap();

    assert_eq!(metadata.content_type.as_deref(), Some("application/json"));
}

#[tokio::test]
async fn maps_wot_operations_to_zenoh_operation_kinds() {
    assert_eq!(
        zenoh_operation_kind(Operation::ReadProperty),
        ZenohOperationKind::Query
    );
    assert_eq!(
        zenoh_operation_kind(Operation::WriteProperty),
        ZenohOperationKind::Put
    );
    assert_eq!(
        zenoh_operation_kind(Operation::SubscribeEvent),
        ZenohOperationKind::Subscribe
    );
    assert_eq!(
        zenoh_operation_kind(Operation::UnsubscribeEvent),
        ZenohOperationKind::Unsubscribe
    );
    assert_eq!(
        zenoh_operation_kind(Operation::CancelAction),
        ZenohOperationKind::RequestReply
    );
}

#[tokio::test]
async fn maps_bulk_wot_operations_to_zenoh_operation_kinds() {
    assert_eq!(
        zenoh_operation_kind(Operation::ReadAllProperties),
        ZenohOperationKind::Query
    );
    assert_eq!(
        zenoh_operation_kind(Operation::WriteMultipleProperties),
        ZenohOperationKind::Put
    );
    assert_eq!(
        zenoh_operation_kind(Operation::ObserveAllProperties),
        ZenohOperationKind::Subscribe
    );
    assert_eq!(
        zenoh_operation_kind(Operation::UnsubscribeAllEvents),
        ZenohOperationKind::Unsubscribe
    );
    assert_eq!(
        zenoh_operation_kind(Operation::SubscribeAllEvents),
        ZenohOperationKind::Subscribe
    );
    assert_eq!(
        zenoh_operation_kind(Operation::QueryAllActions),
        ZenohOperationKind::Query
    );
}
