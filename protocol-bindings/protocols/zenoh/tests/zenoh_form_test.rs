use clinkz_wot_core::{
    AffordanceTarget, BindingRequest, CoreError, CoreResult, InteractionInput, InteractionOutput,
    Payload, ProtocolBinding,
};
use clinkz_wot_protocol_bindings::{AffordanceRef, FormSelectionCriteria};
use clinkz_wot_protocol_bindings_zenoh::{
    CZ_ZENOH_CONGESTION_CONTROL, CZ_ZENOH_ENCODING, CZ_ZENOH_KEY_EXPR, CZ_ZENOH_PRIORITY,
    CZ_ZENOH_QOS, ZenohBinding, ZenohBindingError, ZenohOperationKind, ZenohTransport,
    ZenohTransportRequest, extract_zenoh_metadata, extract_zenoh_target, is_zenoh_form,
    is_zenoh_form_target, plan_zenoh_affordance_operation,
    plan_zenoh_affordance_operation_with_criteria, plan_zenoh_operation, zenoh_operation_kind,
};
use clinkz_wot_td::{
    affordance::{EventAffordance, InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    data_type::Operation,
    form::Form,
    thing::Thing,
};
use serde_json::json;

struct RecordingZenohTransport;

impl ZenohTransport for RecordingZenohTransport {
    fn execute(&mut self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput> {
        assert_eq!(request.plan.kind, ZenohOperationKind::Put);
        assert_eq!(request.plan.key_expr, "clinkz/things/lamp/status");
        assert_eq!(
            request.plan.metadata.encoding.as_deref(),
            Some("application/json")
        );
        assert_eq!(
            request
                .payload
                .as_ref()
                .map(|payload| payload.body.as_slice()),
            Some(&b"on"[..])
        );
        assert_eq!(
            request.parameters.get("source").map(String::as_str),
            Some("test")
        );

        Ok(InteractionOutput::with_payload(Payload::new(
            b"accepted".to_vec(),
            "text/plain",
        )))
    }
}

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
fn builds_operation_plan_from_key_expression_extension() {
    let form = Form::invoke_action("actions/reboot")
        .extra_field(
            CZ_ZENOH_KEY_EXPR,
            json!("clinkz/things/lamp/actions/reboot"),
        )
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp").nosec().build().unwrap();

    let plan = plan_zenoh_operation(&thing, &form, Operation::InvokeAction).unwrap();

    assert_eq!(plan.key_expr, "clinkz/things/lamp/actions/reboot");
    assert_eq!(plan.kind, ZenohOperationKind::RequestReply);
    assert_eq!(plan.metadata, Default::default());
}

#[test]
fn extracts_zenoh_metadata_extensions() {
    let form = Form::write_property("zenoh://clinkz/things/lamp/status")
        .extra_field(CZ_ZENOH_ENCODING, json!("application/json"))
        .extra_field(CZ_ZENOH_QOS, json!("express"))
        .extra_field(CZ_ZENOH_PRIORITY, json!("real-time"))
        .extra_field(CZ_ZENOH_CONGESTION_CONTROL, json!("block"))
        .build()
        .unwrap();

    let metadata = extract_zenoh_metadata(&form).unwrap();

    assert_eq!(metadata.encoding.as_deref(), Some("application/json"));
    assert_eq!(metadata.qos.as_deref(), Some("express"));
    assert_eq!(metadata.priority.as_deref(), Some("real-time"));
    assert_eq!(metadata.congestion_control.as_deref(), Some("block"));
}

#[test]
fn includes_metadata_in_operation_plan() {
    let form = Form::write_property("zenoh://clinkz/things/lamp/status")
        .extra_field(CZ_ZENOH_ENCODING, json!("application/json"))
        .extra_field(CZ_ZENOH_QOS, json!("express"))
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp").nosec().build().unwrap();

    let plan = plan_zenoh_operation(&thing, &form, Operation::WriteProperty).unwrap();

    assert_eq!(plan.key_expr, "clinkz/things/lamp/status");
    assert_eq!(plan.kind, ZenohOperationKind::Put);
    assert_eq!(plan.metadata.encoding.as_deref(), Some("application/json"));
    assert_eq!(plan.metadata.qos.as_deref(), Some("express"));
}

#[test]
fn runtime_binding_delegates_planned_operation_to_transport() {
    let form = Form::write_property("zenoh://clinkz/things/lamp/status")
        .extra_field(CZ_ZENOH_ENCODING, json!("application/json"))
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(form.clone())
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .nosec()
        .property("status", property)
        .build()
        .unwrap();
    let mut input =
        InteractionInput::with_payload(Payload::new(b"on".to_vec(), "application/json"));
    input.parameters.insert("source".into(), "test".into());
    let mut binding = ZenohBinding::with_transport(RecordingZenohTransport);

    let output = binding
        .invoke(BindingRequest {
            thing: &thing,
            target: AffordanceTarget::Property("status"),
            operation: Operation::WriteProperty,
            form: &form,
            input,
        })
        .unwrap();

    assert_eq!(output.payload.unwrap().body, b"accepted");
}

#[test]
fn runtime_binding_without_transport_reports_unavailable() {
    let form = Form::read_property("zenoh://clinkz/things/lamp/status")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(form.clone())
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .nosec()
        .property("status", property)
        .build()
        .unwrap();
    let mut binding = ZenohBinding::new();

    let err = binding
        .invoke(BindingRequest {
            thing: &thing,
            target: AffordanceTarget::Property("status"),
            operation: Operation::ReadProperty,
            form: &form,
            input: InteractionInput::empty(),
        })
        .unwrap_err();

    match err {
        CoreError::Transport(message) => {
            assert!(message.contains("Zenoh transport unavailable"));
            assert!(message.contains("clinkz/things/lamp/status"));
        }
        other => panic!("expected transport error, got {:?}", other),
    }
}

#[test]
fn runtime_binding_rejects_form_that_does_not_support_requested_operation() {
    let form = Form::read_property("zenoh://clinkz/things/lamp/status")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(form.clone())
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .nosec()
        .property("status", property)
        .build()
        .unwrap();
    let mut binding = ZenohBinding::with_transport(RecordingZenohTransport);

    let err = binding
        .invoke(BindingRequest {
            thing: &thing,
            target: AffordanceTarget::Property("status"),
            operation: Operation::WriteProperty,
            form: &form,
            input: InteractionInput::empty(),
        })
        .unwrap_err();

    assert_eq!(
        err,
        CoreError::UnsupportedOperation("Selected form does not support WriteProperty".into())
    );
}

#[test]
fn plans_zenoh_affordance_operation_from_matching_form() {
    let http_form = Form::read_property("https://example.com/things/lamp/properties/status")
        .build()
        .unwrap();
    let zenoh_form = Form::read_property("zenoh://clinkz/things/lamp/properties/status")
        .extra_field(CZ_ZENOH_ENCODING, json!("application/json"))
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
    assert_eq!(
        plan.operation.key_expr,
        "clinkz/things/lamp/properties/status"
    );
    assert_eq!(
        plan.operation.metadata.encoding.as_deref(),
        Some("application/json")
    );
}

#[test]
fn plans_zenoh_affordance_operation_with_metadata_criteria() {
    let json_form = Form::read_property("zenoh://clinkz/things/lamp/properties/status/json")
        .content_type("application/json")
        .build()
        .unwrap();
    let cbor_form = Form::read_property("zenoh://clinkz/things/lamp/properties/status/cbor")
        .content_type("application/cbor")
        .extra_field(CZ_ZENOH_ENCODING, json!("application/cbor"))
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
        FormSelectionCriteria::operation(Operation::ReadProperty).content_type("application/cbor"),
    )
    .unwrap();

    assert_eq!(plan.form_index, 1);
    assert_eq!(
        plan.operation.key_expr,
        "clinkz/things/lamp/properties/status/cbor"
    );
    assert_eq!(
        plan.operation.metadata.encoding.as_deref(),
        Some("application/cbor")
    );
}

#[test]
fn plans_zenoh_affordance_operation_with_subprotocol_criteria() {
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
        FormSelectionCriteria::operation(Operation::SubscribeEvent).subprotocol("sse"),
    )
    .unwrap();

    assert_eq!(plan.form_index, 1);
    assert_eq!(plan.operation.kind, ZenohOperationKind::Subscribe);
    assert_eq!(
        plan.operation.key_expr,
        "clinkz/things/lamp/events/status-change/sse"
    );
}

#[test]
fn plans_thing_level_zenoh_form() {
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
    assert_eq!(plan.operation.key_expr, "clinkz/things/lamp");
}

#[test]
fn plans_bulk_property_operation_from_thing_level_form() {
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
    assert_eq!(plan.operation.key_expr, "clinkz/things/lamp/properties");
}

#[test]
fn plans_bulk_event_operation_from_thing_level_form() {
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
    assert_eq!(plan.operation.key_expr, "clinkz/things/lamp/events");
}

#[test]
fn plans_relative_href_against_zenoh_base() {
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

    assert_eq!(
        plan.operation.key_expr,
        "clinkz/things/lamp/properties/status"
    );
    assert_eq!(plan.operation.kind, ZenohOperationKind::Query);
}

#[test]
fn reports_selection_error_when_affordance_has_no_zenoh_form() {
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
        ZenohBindingError::Selection(
            "Caller filter mismatch: No form matches FormSelectionCriteria { operation: ReadProperty, content_type: None, subprotocol: None } after applying caller filter".into()
        )
    );
}

#[test]
fn rejects_non_string_zenoh_metadata_extension() {
    let form = Form::read_property("zenoh://clinkz/things/lamp/status")
        .extra_field(CZ_ZENOH_QOS, json!(true))
        .build()
        .unwrap();

    let err = extract_zenoh_metadata(&form).unwrap_err();

    assert_eq!(
        err,
        ZenohBindingError::Target("cz-zenoh:qos must be a string".into())
    );
}

#[test]
fn rejects_empty_zenoh_metadata_extension() {
    let form = Form::read_property("zenoh://clinkz/things/lamp/status")
        .extra_field(CZ_ZENOH_ENCODING, json!(""))
        .build()
        .unwrap();

    let err = extract_zenoh_metadata(&form).unwrap_err();

    assert_eq!(
        err,
        ZenohBindingError::Target("cz-zenoh:encoding must not be empty".into())
    );
}

#[test]
fn maps_wot_operations_to_zenoh_operation_kinds() {
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

#[test]
fn maps_bulk_wot_operations_to_zenoh_operation_kinds() {
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
        zenoh_operation_kind(Operation::QueryAllActions),
        ZenohOperationKind::Query
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
