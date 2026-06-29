#[path = "servient_test/support.rs"]
mod support;

use clinkz_wot_core::{InteractionInput, Payload};
use clinkz_wot_protocol_bindings::FormSelectionCriteria;
use clinkz_wot_protocol_bindings_zenoh::{SharedZenohTransport, ZenohBindingTransport};
use clinkz_wot_servient::Servient;
use clinkz_wot_td::data_type::Operation;

use support::{CountingServientZenohTransport, ServientZenohTransport, zenoh_thing};

#[test]
fn consumed_handle_routes_remote_requests_through_zenoh_binding_transport() {
    let td = zenoh_thing("urn:thing:zenoh-lamp", "Zenoh Lamp");
    let servient = Servient::builder()
        .binding_factory_with_support(
            || {
                Box::new(ZenohBindingTransport::with_transport(
                    ServientZenohTransport,
                ))
            },
            |thing, form, operation| {
                matches!(
                    operation,
                    Operation::ReadProperty
                        | Operation::WriteProperty
                        | Operation::InvokeAction
                        | Operation::SubscribeEvent
                        | Operation::ObserveProperty
                ) && clinkz_wot_protocol_bindings_zenoh::is_zenoh_form_target(thing, form)
            },
        )
        .build();

    let consumed = servient.consume(td).unwrap();
    let read = consumed
        .read_property_with_criteria(
            "status",
            FormSelectionCriteria::new(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(read.payload.unwrap().body.as_ref(), b"zenoh-on");

    consumed
        .write_property_with_criteria(
            "status",
            FormSelectionCriteria::new(Operation::WriteProperty).content_type("text/plain"),
            InteractionInput::with_payload(Payload::new(b"zenoh-off".to_vec(), "text/plain")),
        )
        .unwrap();

    let action = consumed
        .invoke_action_with_criteria(
            "echo",
            FormSelectionCriteria::new(Operation::InvokeAction).content_type("text/plain"),
            InteractionInput::with_payload(Payload::new(b"zenoh-echo".to_vec(), "text/plain")),
        )
        .unwrap();
    assert_eq!(action.payload.unwrap().body.as_ref(), b"zenoh-echo");

    let event = consumed
        .subscribe_event_with_criteria(
            "startup",
            FormSelectionCriteria::new(Operation::SubscribeEvent).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(
        event.poll_next().expect("subscription sample").body.as_ref(),
        b"zenoh-subscribed"
    );
}

#[test]
fn consumed_handle_shares_zenoh_transport_state_across_requests() {
    let td = zenoh_thing("urn:thing:shared-zenoh-lamp", "Shared Zenoh Lamp");
    let shared = SharedZenohTransport::new(CountingServientZenohTransport::default());
    let factory_transport = shared.clone();
    let servient = Servient::builder()
        .binding_factory_with_support(
            move || {
                Box::new(ZenohBindingTransport::with_transport(
                    factory_transport.clone(),
                ))
            },
            |thing, form, operation| {
                matches!(
                    operation,
                    Operation::ReadProperty
                        | Operation::WriteProperty
                        | Operation::InvokeAction
                        | Operation::SubscribeEvent
                        | Operation::ObserveProperty
                ) && clinkz_wot_protocol_bindings_zenoh::is_zenoh_form_target(thing, form)
            },
        )
        .build();

    let consumed = servient.consume(td).unwrap();
    let criteria = FormSelectionCriteria::new(Operation::ReadProperty).content_type("text/plain");
    let first = consumed
        .read_property_with_criteria("status", criteria, InteractionInput::empty())
        .unwrap();
    assert_eq!(first.payload.unwrap().body.as_ref(), b"zenoh-read-1");

    consumed
        .write_property_with_criteria(
            "status",
            FormSelectionCriteria::new(Operation::WriteProperty).content_type("text/plain"),
            InteractionInput::with_payload(Payload::new(b"zenoh-off".to_vec(), "text/plain")),
        )
        .unwrap();

    let second = consumed
        .read_property_with_criteria("status", criteria, InteractionInput::empty())
        .unwrap();
    assert_eq!(second.payload.unwrap().body.as_ref(), b"zenoh-read-3");
    assert_eq!(shared.inner().calls.get(), 3);
}
