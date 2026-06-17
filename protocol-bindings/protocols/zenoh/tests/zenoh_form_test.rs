use clinkz_wot_core::{
    AffordanceTarget, BindingRequest, CoreError, CoreResult, InteractionInput, InteractionOutput,
    Payload, ProtocolBinding,
};
use clinkz_wot_protocol_bindings::{AffordanceRef, FormSelectionCriteria};
use clinkz_wot_protocol_bindings_zenoh::{
    extract_zenoh_metadata, extract_zenoh_target, is_zenoh_form, is_zenoh_form_target,
    plan_zenoh_affordance_operation, plan_zenoh_affordance_operation_with_criteria,
    plan_zenoh_operation, zenoh_operation_kind, ZenohBindingError, ZenohBindingTransport,
    ZenohOperationKind, ZenohTransport, ZenohTransportRequest, CZ_ZENOH_CONGESTION_CONTROL,
    CZ_ZENOH_PRIORITY, CZ_ZENOH_QOS,
};
use clinkz_wot_td::{
    affordance::{EventAffordance, InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    data_type::Operation,
    form::Form,
    thing::Thing,
};
use serde_json::json;

#[cfg(feature = "zenoh")]
use clinkz_wot_protocol_bindings_zenoh::SharedZenohTransport;

struct RecordingZenohTransport;

impl ZenohTransport for RecordingZenohTransport {
    fn execute(&mut self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput> {
        assert_eq!(request.plan.kind, ZenohOperationKind::Put);
        assert_eq!(request.plan.key_expr, "clinkz/things/lamp/status");
        assert_eq!(
            request.plan.metadata.content_type.as_deref(),
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

#[cfg(feature = "zenoh")]
#[derive(Default)]
struct CountingZenohTransport {
    calls: usize,
}

#[cfg(feature = "zenoh")]
impl ZenohTransport for CountingZenohTransport {
    fn execute(&mut self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput> {
        self.calls += 1;
        Ok(InteractionOutput::with_payload(Payload::new(
            format!("{}:{}", self.calls, request.plan.key_expr).into_bytes(),
            "text/plain",
        )))
    }
}

#[test]
fn supports_forms_with_zenoh_href() {
    let form = Form::read_property("zenoh://clinkz/things/lamp/status")
        .build()
        .unwrap();
    let binding = ZenohBindingTransport::with_transport(RecordingZenohTransport);

    assert!(is_zenoh_form(&form));
    assert!(binding.supports(&form, Operation::ReadProperty));
}

#[test]
fn supports_forms_with_relative_href_and_zenoh_base() {
    let form = Form::read_property("properties/status").build().unwrap();
    let thing = Thing::builder("Lamp")
        .base("zenoh://clinkz/things/lamp/")
        .nosec()
        .build()
        .unwrap();
    let binding = ZenohBindingTransport::with_transport(RecordingZenohTransport);

    assert!(!is_zenoh_form(&form));
    assert!(!binding.supports(&form, Operation::ReadProperty));
    assert!(binding.supports_with_thing(&thing, &form, Operation::ReadProperty));
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
fn builds_operation_plan_from_href_resolved_against_base() {
    let form = Form::invoke_action("actions/reboot").build().unwrap();
    let thing = Thing::builder("Lamp")
        .base("zenoh://clinkz/things/lamp/")
        .nosec()
        .build()
        .unwrap();

    let plan = plan_zenoh_operation(&thing, &form, Operation::InvokeAction).unwrap();

    assert_eq!(plan.key_expr, "clinkz/things/lamp/actions/reboot");
    assert_eq!(plan.kind, ZenohOperationKind::RequestReply);
    assert_eq!(
        plan.metadata,
        clinkz_wot_protocol_bindings_zenoh::ZenohFormMetadata {
            content_type: Some("application/json".into()),
            ..Default::default()
        }
    );
}

#[test]
fn extracts_zenoh_metadata_extensions() {
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

#[test]
fn includes_metadata_in_operation_plan() {
    let form = Form::write_property("zenoh://clinkz/things/lamp/status")
        .content_type("application/json")
        .extra_field(CZ_ZENOH_QOS, json!("express"))
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp").nosec().build().unwrap();

    let plan = plan_zenoh_operation(&thing, &form, Operation::WriteProperty).unwrap();

    assert_eq!(plan.key_expr, "clinkz/things/lamp/status");
    assert_eq!(plan.kind, ZenohOperationKind::Put);
    assert_eq!(
        plan.metadata.content_type.as_deref(),
        Some("application/json")
    );
    assert_eq!(plan.metadata.qos.as_deref(), Some("express"));
}

#[test]
fn runtime_binding_delegates_planned_operation_to_transport() {
    let form = Form::write_property("zenoh://clinkz/things/lamp/status")
        .content_type("application/json")
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
    let mut binding = ZenohBindingTransport::with_transport(RecordingZenohTransport);

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

#[cfg(feature = "zenoh")]
#[test]
fn shared_transport_reuses_underlying_runtime_state() {
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
    let shared = SharedZenohTransport::new(CountingZenohTransport::default());
    let mut first_binding = ZenohBindingTransport::with_transport(shared.clone());
    let mut second_binding = ZenohBindingTransport::with_transport(shared.clone());

    let first = first_binding
        .invoke(BindingRequest {
            thing: &thing,
            target: AffordanceTarget::Property("status"),
            operation: Operation::ReadProperty,
            form: &form,
            input: InteractionInput::empty(),
        })
        .unwrap();
    let second = second_binding
        .invoke(BindingRequest {
            thing: &thing,
            target: AffordanceTarget::Property("status"),
            operation: Operation::ReadProperty,
            form: &form,
            input: InteractionInput::empty(),
        })
        .unwrap();

    assert_eq!(first.payload.unwrap().body, b"1:clinkz/things/lamp/status");
    assert_eq!(second.payload.unwrap().body, b"2:clinkz/things/lamp/status");
    assert_eq!(shared.inner().lock().unwrap().calls, 2);
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
    let mut binding = ZenohBindingTransport::with_transport(RecordingZenohTransport);

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
    assert_eq!(
        plan.operation.key_expr,
        "clinkz/things/lamp/properties/status"
    );
    assert_eq!(
        plan.operation.metadata.content_type.as_deref(),
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
        "clinkz/things/lamp/properties/status/cbor"
    );
    assert_eq!(
        plan.operation.metadata.content_type.as_deref(),
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
        FormSelectionCriteria::new(Operation::SubscribeEvent).subprotocol("sse"),
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
fn rejects_relative_href_when_base_is_invalid() {
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

#[test]
fn runtime_binding_reports_invalid_interaction_for_unresolvable_target() {
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
    let mut binding = ZenohBindingTransport::with_transport(RecordingZenohTransport);

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
        CoreError::InvalidInteraction(message) => {
            assert!(message.contains("Cannot resolve form href"));
            assert!(message.contains("URI template base"));
        }
        other => panic!("expected invalid interaction error, got {:?}", other),
    }
}

#[test]
fn plans_operations_from_clinkz_extension_fixture() {
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
        "clinkz/things/targeted-roundtrip/properties/status"
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
        "clinkz/things/targeted-roundtrip/properties/status"
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
        "clinkz/things/targeted-roundtrip/events/alarm"
    );

    let read_all =
        plan_zenoh_affordance_operation(&thing, AffordanceRef::Thing, Operation::ReadAllProperties)
            .unwrap();
    assert_eq!(read_all.form_index, 0);
    assert_eq!(read_all.operation.kind, ZenohOperationKind::Query);
    assert_eq!(
        read_all.operation.key_expr,
        "clinkz/things/targeted-roundtrip/properties"
    );
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
        ZenohBindingError::InvalidExtension {
            term: CZ_ZENOH_QOS,
            message: "must be a string".into()
        }
    );
}

#[test]
fn extracts_default_content_type_into_zenoh_metadata() {
    let form = Form::read_property("zenoh://clinkz/things/lamp/status")
        .build()
        .unwrap();

    let metadata = extract_zenoh_metadata(&form).unwrap();

    assert_eq!(metadata.content_type.as_deref(), Some("application/json"));
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
    let binding = ZenohBindingTransport::with_transport_and_supported_operations(
        RecordingZenohTransport,
        [Operation::ReadProperty],
    );

    assert!(!binding.supports(&form, Operation::WriteProperty));
}
