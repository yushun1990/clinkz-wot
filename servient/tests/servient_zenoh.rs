#[path = "servient_test/support.rs"]
mod support;

use clinkz_wot_core::{InteractionInput, Payload};
use clinkz_wot_protocol_bindings::FormSelectionCriteria;
use clinkz_wot_protocol_bindings_zenoh::{SharedZenohTransport, ZenohBinding};
use clinkz_wot_servient::Servient;
use clinkz_wot_td::data_type::Operation;

use support::{CountingServientZenohTransport, ServientZenohTransport, zenoh_thing};

#[test]
fn servient_routes_remote_requests_through_zenoh_binding_transport() {
    let td = zenoh_thing("urn:thing:zenoh-lamp", "Zenoh Lamp");
    let mut servient = Servient::builder()
        .binding_factory(|| Box::new(ZenohBinding::with_transport(ServientZenohTransport)))
        .build();
    servient.register(td).unwrap();

    let read = servient
        .read_remote_property_with_criteria(
            "urn:thing:zenoh-lamp",
            "status",
            FormSelectionCriteria::new(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(read.payload.unwrap().body, b"zenoh-on");

    servient
        .write_remote_property_with_criteria(
            "urn:thing:zenoh-lamp",
            "status",
            FormSelectionCriteria::new(Operation::WriteProperty).content_type("text/plain"),
            InteractionInput::with_payload(Payload::new(b"zenoh-off".to_vec(), "text/plain")),
        )
        .unwrap();

    let action = servient
        .invoke_remote_action_with_criteria(
            "urn:thing:zenoh-lamp",
            "echo",
            FormSelectionCriteria::new(Operation::InvokeAction).content_type("text/plain"),
            InteractionInput::with_payload(Payload::new(b"zenoh-echo".to_vec(), "text/plain")),
        )
        .unwrap();
    assert_eq!(action.payload.unwrap().body, b"zenoh-echo");

    let event = servient
        .subscribe_remote_event_with_criteria(
            "urn:thing:zenoh-lamp",
            "startup",
            FormSelectionCriteria::new(Operation::SubscribeEvent).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(event.payload.unwrap().body, b"zenoh-subscribed");
}

#[test]
fn servient_binding_factories_can_share_zenoh_transport_state() {
    let td = zenoh_thing("urn:thing:shared-zenoh-lamp", "Shared Zenoh Lamp");
    let shared = SharedZenohTransport::new(CountingServientZenohTransport::default());
    let factory_transport = shared.clone();
    let mut servient = Servient::builder()
        .binding_factory(move || Box::new(ZenohBinding::with_transport(factory_transport.clone())))
        .build();
    servient.register(td).unwrap();

    let first = servient
        .read_remote_property_with_criteria(
            "urn:thing:shared-zenoh-lamp",
            "status",
            FormSelectionCriteria::new(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(first.payload.unwrap().body, b"zenoh-read-1");

    servient
        .write_remote_property_with_criteria(
            "urn:thing:shared-zenoh-lamp",
            "status",
            FormSelectionCriteria::new(Operation::WriteProperty).content_type("text/plain"),
            InteractionInput::with_payload(Payload::new(b"zenoh-off".to_vec(), "text/plain")),
        )
        .unwrap();

    let second = servient
        .read_remote_property_with_criteria(
            "urn:thing:shared-zenoh-lamp",
            "status",
            FormSelectionCriteria::new(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(second.payload.unwrap().body, b"zenoh-read-3");
    assert_eq!(shared.inner().lock().unwrap().calls, 3);
}
