mod support;

use std::{cell::Cell, rc::Rc};

use clinkz_wot_core::{AffordanceTarget, ConsumedThing, InteractionInput, LocalThing, Payload};
use clinkz_wot_protocol_bindings::{BindingCoreError, FormSelectionCriteria};
use clinkz_wot_protocol_bindings_zenoh::{SharedZenohTransport, ZenohBinding};
use clinkz_wot_servient::{
    ConsumedThingCache, InMemoryBindingPlanCache, InMemoryConsumedThingCache,
    InMemorySelectedFormCache, SelectedFormCache, SelectedFormCacheAffordance,
    SelectedFormCacheKey, Servient, ServientError,
};
use clinkz_wot_td::data_type::Operation;

use support::*;

#[test]
fn exposes_local_thing_and_dispatches_handler() {
    let (td, _) = thing("urn:thing:local-lamp", "Local Lamp");
    let mut local = LocalThing::new(td);
    local.register_property_handler(
        "status",
        StatusProperty {
            value: Payload::new(b"off".to_vec(), "text/plain"),
        },
    );
    local.register_action_handler("echo", EchoAction);
    local.register_event_handler("startup", StartupEvent);

    let mut servient = Servient::new();
    servient.expose(local).unwrap();
    servient.start().unwrap();

    let payload = servient
        .read_property("urn:thing:local-lamp", "status", InteractionInput::empty())
        .unwrap()
        .payload
        .unwrap();

    assert!(servient.is_running());
    assert_eq!(payload.body, b"off");
    assert_eq!(servient.list().total, 1);

    servient
        .write_property(
            "urn:thing:local-lamp",
            "status",
            InteractionInput::with_payload(Payload::new(b"on".to_vec(), "text/plain")),
        )
        .unwrap();
    let payload = servient
        .read_property("urn:thing:local-lamp", "status", InteractionInput::empty())
        .unwrap()
        .payload
        .unwrap();
    assert_eq!(payload.body, b"on");

    let payload = servient
        .invoke_action(
            "urn:thing:local-lamp",
            "echo",
            InteractionInput::with_payload(Payload::new(b"hello".to_vec(), "text/plain")),
        )
        .unwrap()
        .payload
        .unwrap();
    assert_eq!(payload.body, b"hello");

    let mut sink = CollectSink::default();
    servient
        .subscribe_event(
            "urn:thing:local-lamp",
            "startup",
            InteractionInput::empty(),
            &mut sink,
        )
        .unwrap();
    assert_eq!(sink.payloads[0].body, b"ready");
}

#[test]
fn consumes_discovered_td_through_registered_binding_factory() {
    let (td, forms) = thing("urn:thing:remote-lamp", "Remote Lamp");
    let mut servient = Servient::builder()
        .binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"on".to_vec(), "text/plain"),
            })
        })
        .build();
    servient.register(td).unwrap();

    let mut consumed = servient.consume("urn:thing:remote-lamp").unwrap();
    let output = consumed
        .request(
            AffordanceTarget::Property("status"),
            Operation::ReadProperty,
            &forms.read_property,
            InteractionInput::empty(),
        )
        .unwrap();

    assert_eq!(output.payload.unwrap().body, b"on");
}

#[test]
fn servient_remote_convenience_methods_route_through_registered_bindings() {
    let (td, forms) = thing("urn:thing:remote-lamp", "Remote Lamp");
    let mut servient = Servient::builder()
        .binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"on".to_vec(), "text/plain"),
            })
        })
        .build();
    servient.register(td).unwrap();

    let read = servient
        .read_remote_property(
            "urn:thing:remote-lamp",
            "status",
            &forms.read_property,
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(read.payload.unwrap().body, b"on");

    servient
        .write_remote_property(
            "urn:thing:remote-lamp",
            "status",
            &forms.write_property,
            InteractionInput::with_payload(Payload::new(b"off".to_vec(), "text/plain")),
        )
        .unwrap();

    let action = servient
        .invoke_remote_action(
            "urn:thing:remote-lamp",
            "echo",
            &forms.invoke_action,
            InteractionInput::with_payload(Payload::new(b"hello".to_vec(), "text/plain")),
        )
        .unwrap();
    assert_eq!(action.payload.unwrap().body, b"hello");

    let event = servient
        .subscribe_remote_event(
            "urn:thing:remote-lamp",
            "startup",
            &forms.subscribe_event,
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(event.payload.unwrap().body, b"subscribed");
}

#[test]
fn payload_codecs_are_used_for_local_and_remote_interactions() {
    let encode_calls = Rc::new(Cell::new(0));
    let decode_calls = Rc::new(Cell::new(0));

    let (local_td, _) = thing("urn:thing:local-codec-lamp", "Local Codec Lamp");
    let mut local = LocalThing::new(local_td);
    local.register_property_handler(
        "status",
        StatusProperty {
            value: Payload::new(b"local".to_vec(), "text/plain"),
        },
    );

    let (remote_td, remote_forms) = thing("urn:thing:remote-codec-lamp", "Remote Codec Lamp");
    let mut servient = Servient::builder()
        .payload_codec(CountingCodec {
            encode_calls: encode_calls.clone(),
            decode_calls: decode_calls.clone(),
        })
        .binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"remote".to_vec(), "text/plain"),
            })
        })
        .build();

    servient.expose(local).unwrap();
    let local = servient
        .read_property(
            "urn:thing:local-codec-lamp",
            "status",
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(local.payload.unwrap().body, b"local");

    servient.register(remote_td).unwrap();
    let remote = servient
        .read_remote_property(
            "urn:thing:remote-codec-lamp",
            "status",
            &remote_forms.read_property,
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(remote.payload.unwrap().body, b"remote");
    assert_eq!(decode_calls.get(), 2);
    assert_eq!(encode_calls.get(), 2);
}

#[test]
fn security_providers_are_used_for_local_and_remote_interactions() {
    let applied_calls = Rc::new(Cell::new(0));

    let (local_td, _) = secure_thing("urn:thing:local-secure-lamp", "Local Secure Lamp");
    let mut local = LocalThing::new(local_td);
    local.register_property_handler("status", AuthenticatedStatusProperty);

    let (remote_td, remote_form) =
        secure_thing("urn:thing:remote-secure-lamp", "Remote Secure Lamp");
    let mut servient = Servient::builder()
        .security_provider(RecordingSecurityProvider {
            applied_calls: applied_calls.clone(),
        })
        .binding_factory(|| Box::new(AuthenticatedReadBinding))
        .build();

    servient.expose(local).unwrap();
    let local = servient
        .read_property(
            "urn:thing:local-secure-lamp",
            "status",
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(local.payload.unwrap().body, b"secure-local");

    servient.register(remote_td).unwrap();
    let remote = servient
        .read_remote_property(
            "urn:thing:remote-secure-lamp",
            "status",
            &remote_form,
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(remote.payload.unwrap().body, b"secure-remote");
    assert_eq!(applied_calls.get(), 2);
}

#[test]
fn late_codec_and_security_provider_registration_is_guarded_by_lifecycle() {
    let encode_calls = Rc::new(Cell::new(0));
    let decode_calls = Rc::new(Cell::new(0));
    let applied_calls = Rc::new(Cell::new(0));
    let mut servient = Servient::new();

    servient
        .register_payload_codec(CountingCodec {
            encode_calls: encode_calls.clone(),
            decode_calls: decode_calls.clone(),
        })
        .unwrap();
    servient
        .register_security_provider(RecordingSecurityProvider {
            applied_calls: applied_calls.clone(),
        })
        .unwrap();
    assert_eq!(servient.payload_codecs().len(), 1);
    assert_eq!(servient.security_providers().len(), 1);

    servient.start().unwrap();

    let err = servient
        .register_payload_codec(CountingCodec {
            encode_calls,
            decode_calls,
        })
        .unwrap_err();
    assert!(matches!(err, ServientError::Running));

    let err = servient
        .register_security_provider(RecordingSecurityProvider { applied_calls })
        .unwrap_err();
    assert!(matches!(err, ServientError::Running));
}

#[test]
fn servient_remote_criteria_methods_select_matching_forms() {
    let (td, _) = thing("urn:thing:remote-lamp", "Remote Lamp");
    let mut servient = Servient::builder()
        .binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"on".to_vec(), "text/plain"),
            })
        })
        .build();
    servient.register(td).unwrap();

    let read = servient
        .read_remote_property_with_criteria(
            "urn:thing:remote-lamp",
            "status",
            FormSelectionCriteria::operation(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(read.payload.unwrap().body, b"on");

    servient
        .write_remote_property_with_criteria(
            "urn:thing:remote-lamp",
            "status",
            FormSelectionCriteria::operation(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::with_payload(Payload::new(b"off".to_vec(), "text/plain")),
        )
        .unwrap();

    let action = servient
        .invoke_remote_action_with_criteria(
            "urn:thing:remote-lamp",
            "echo",
            FormSelectionCriteria::operation(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::with_payload(Payload::new(b"hello".to_vec(), "text/plain")),
        )
        .unwrap();
    assert_eq!(action.payload.unwrap().body, b"hello");

    let event = servient
        .subscribe_remote_event_with_criteria(
            "urn:thing:remote-lamp",
            "startup",
            FormSelectionCriteria::operation(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(event.payload.unwrap().body, b"subscribed");
    assert_eq!(servient.selected_form_cache().len(), 4);
}

#[test]
fn servient_remote_criteria_methods_report_binding_selection_errors() {
    let (td, _) = thing("urn:thing:remote-lamp", "Remote Lamp");
    let mut servient = Servient::builder()
        .binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"on".to_vec(), "text/plain"),
            })
        })
        .build();
    servient.register(td).unwrap();

    let err = servient
        .read_remote_property_with_criteria(
            "urn:thing:remote-lamp",
            "status",
            FormSelectionCriteria::operation(Operation::ReadProperty).content_type("image/png"),
            InteractionInput::empty(),
        )
        .unwrap_err();

    assert!(matches!(
        err,
        ServientError::Binding(BindingCoreError::MetadataMismatch(_))
    ));
}

#[test]
fn servient_remote_criteria_methods_reuse_cached_selected_forms() {
    let (td, _first_form, cached_form) = cacheable_thing("urn:thing:cached-lamp", "Cached Lamp");
    let mut servient = Servient::builder()
        .with_selected_form_cache(InMemorySelectedFormCache::new())
        .binding_factory(|| Box::new(HrefBinding))
        .build();
    servient.register(td).unwrap();
    servient.selected_form_cache().insert(
        SelectedFormCacheKey::new(
            "urn:thing:cached-lamp",
            SelectedFormCacheAffordance::Property("status".to_owned()),
            FormSelectionCriteria::operation(Operation::ReadProperty).content_type("text/plain"),
        ),
        cached_form,
    );

    let read = servient
        .read_remote_property_with_criteria(
            "urn:thing:cached-lamp",
            "status",
            FormSelectionCriteria::operation(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();

    assert_eq!(
        read.payload.unwrap().body,
        b"test://things/lamp/properties/status/cached"
    );
    assert_eq!(servient.selected_form_cache().len(), 1);
}

#[test]
fn servient_remote_criteria_methods_reuse_cached_binding_plans() {
    let (td, _) = thing("urn:thing:planned-lamp", "Planned Lamp");
    let unsupported_calls = std::rc::Rc::new(std::cell::RefCell::new(0));
    let supported_calls = std::rc::Rc::new(std::cell::RefCell::new(0));
    let unsupported_factory_calls = unsupported_calls.clone();
    let supported_factory_calls = supported_calls.clone();
    let mut servient = Servient::builder()
        .with_binding_plan_cache(InMemoryBindingPlanCache::new())
        .binding_factory(move || {
            Box::new(CountingUnsupportedBinding {
                supports_calls: unsupported_factory_calls.clone(),
            })
        })
        .binding_factory(move || {
            Box::new(CountingHrefBinding {
                supports_calls: supported_factory_calls.clone(),
            })
        })
        .build();
    servient.register(td).unwrap();

    let read = servient
        .read_remote_property_with_criteria(
            "urn:thing:planned-lamp",
            "status",
            FormSelectionCriteria::operation(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(
        read.payload.unwrap().body,
        b"test://things/lamp/properties/status"
    );
    assert_eq!(*unsupported_calls.borrow(), 1);
    assert_eq!(servient.binding_plan_cache().len(), 1);

    let read = servient
        .read_remote_property_with_criteria(
            "urn:thing:planned-lamp",
            "status",
            FormSelectionCriteria::operation(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(
        read.payload.unwrap().body,
        b"test://things/lamp/properties/status"
    );
    assert_eq!(
        *unsupported_calls.borrow(),
        1,
        "cached plan should skip probing earlier unsupported factories"
    );
    assert_eq!(servient.binding_plan_cache().len(), 1);
    assert!(*supported_calls.borrow() >= 2);
}

#[test]
fn servient_invalidates_binding_plan_cache_on_td_update() {
    let (td, _) = thing("urn:thing:planned-lamp", "Planned Lamp");
    let (updated_td, _) = thing("urn:thing:planned-lamp", "Updated Planned Lamp");
    let mut servient = Servient::builder()
        .binding_factory(|| Box::new(HrefBinding))
        .build();
    servient.register(td).unwrap();

    servient
        .read_remote_property_with_criteria(
            "urn:thing:planned-lamp",
            "status",
            FormSelectionCriteria::operation(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(servient.selected_form_cache().len(), 1);
    assert_eq!(servient.binding_plan_cache().len(), 1);

    servient.update(updated_td).unwrap();

    assert!(servient.selected_form_cache().is_empty());
    assert!(servient.binding_plan_cache().is_empty());
}

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
            FormSelectionCriteria::operation(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(read.payload.unwrap().body, b"zenoh-on");

    servient
        .write_remote_property_with_criteria(
            "urn:thing:zenoh-lamp",
            "status",
            FormSelectionCriteria::operation(Operation::WriteProperty).content_type("text/plain"),
            InteractionInput::with_payload(Payload::new(b"zenoh-off".to_vec(), "text/plain")),
        )
        .unwrap();

    let action = servient
        .invoke_remote_action_with_criteria(
            "urn:thing:zenoh-lamp",
            "echo",
            FormSelectionCriteria::operation(Operation::InvokeAction).content_type("text/plain"),
            InteractionInput::with_payload(Payload::new(b"zenoh-echo".to_vec(), "text/plain")),
        )
        .unwrap();
    assert_eq!(action.payload.unwrap().body, b"zenoh-echo");

    let event = servient
        .subscribe_remote_event_with_criteria(
            "urn:thing:zenoh-lamp",
            "startup",
            FormSelectionCriteria::operation(Operation::SubscribeEvent).content_type("text/plain"),
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
            FormSelectionCriteria::operation(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(first.payload.unwrap().body, b"zenoh-read-1");

    servient
        .write_remote_property_with_criteria(
            "urn:thing:shared-zenoh-lamp",
            "status",
            FormSelectionCriteria::operation(Operation::WriteProperty).content_type("text/plain"),
            InteractionInput::with_payload(Payload::new(b"zenoh-off".to_vec(), "text/plain")),
        )
        .unwrap();

    let second = servient
        .read_remote_property_with_criteria(
            "urn:thing:shared-zenoh-lamp",
            "status",
            FormSelectionCriteria::operation(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(second.payload.unwrap().body, b"zenoh-read-3");
    assert_eq!(shared.inner().lock().unwrap().calls, 3);
}

#[test]
fn late_binding_factory_registration_is_used_by_new_consumed_requests() {
    let (td, forms) = thing("urn:thing:remote-lamp", "Remote Lamp");
    let mut servient = Servient::new();
    servient.register(td).unwrap();
    servient
        .register_binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"late".to_vec(), "text/plain"),
            })
        })
        .unwrap();

    let output = servient
        .read_remote_property(
            "urn:thing:remote-lamp",
            "status",
            &forms.read_property,
            InteractionInput::empty(),
        )
        .unwrap();

    assert_eq!(output.payload.unwrap().body, b"late");
}

#[test]
fn remote_requests_report_missing_bindings_and_unknown_things() {
    let (td, forms) = thing("urn:thing:remote-lamp", "Remote Lamp");
    let mut servient = Servient::new();
    servient.register(td).unwrap();

    let err = servient
        .read_remote_property(
            "urn:thing:remote-lamp",
            "status",
            &forms.read_property,
            InteractionInput::empty(),
        )
        .unwrap_err();
    assert!(matches!(err, ServientError::Core(_)));

    let err = servient
        .read_remote_property(
            "urn:thing:missing",
            "status",
            &forms.read_property,
            InteractionInput::empty(),
        )
        .unwrap_err();
    assert!(matches!(err, ServientError::Discovery(_)));
}

#[test]
fn unexposes_local_thing_and_removes_directory_entry() {
    let (td, _) = thing("urn:thing:local-lamp", "Local Lamp");
    let local = LocalThing::new(td);
    let mut servient = Servient::new();
    servient.expose(local).unwrap();

    let removed = servient.unexpose("urn:thing:local-lamp").unwrap();

    assert_eq!(
        removed.thing_description().id.as_ref().unwrap().as_str(),
        "urn:thing:local-lamp"
    );
    let err = match servient.consume("urn:thing:local-lamp") {
        Ok(_) => panic!("removed Thing should not be consumable"),
        Err(err) => err,
    };
    assert!(matches!(err, ServientError::Discovery(_)));
}

#[test]
fn servient_uses_injected_exposed_thing_registry() {
    let (td, _) = thing("urn:thing:local-lamp", "Local Lamp");
    let mut local = LocalThing::new(td);
    local.register_property_handler(
        "status",
        StatusProperty {
            value: Payload::new(b"off".to_vec(), "text/plain"),
        },
    );

    let mut servient = Servient::builder()
        .with_exposed_registry(TestExposedRegistry::default())
        .build();
    servient.expose(local).unwrap();

    let payload = servient
        .read_property("urn:thing:local-lamp", "status", InteractionInput::empty())
        .unwrap()
        .payload
        .unwrap();
    assert_eq!(payload.body, b"off");
    assert_eq!(servient.exposed_registry().inserted, 1);

    servient.unexpose("urn:thing:local-lamp").unwrap();
    assert_eq!(servient.exposed_registry().removed, 1);
}

#[test]
fn servient_syncs_consumed_cache_with_directory_mutations() {
    let (td, _) = thing("urn:thing:remote-lamp", "Remote Lamp");
    let (updated_td, _) = thing("urn:thing:remote-lamp", "Updated Remote Lamp");
    let mut servient = Servient::builder()
        .with_consumed_cache(TestConsumedCache::default())
        .build();

    servient.register(td).unwrap();
    assert_eq!(servient.consumed_cache().inserted, 1);
    assert_eq!(servient.consumed_cache().inner.len(), 1);

    let consumed = servient.consume("urn:thing:remote-lamp").unwrap();
    assert_eq!(
        consumed.thing_description()._metadata.title.as_deref(),
        Some("Remote Lamp")
    );

    servient.update(updated_td).unwrap();
    assert_eq!(servient.consumed_cache().inserted, 2);
    let consumed = servient.consume("urn:thing:remote-lamp").unwrap();
    assert_eq!(
        consumed.thing_description()._metadata.title.as_deref(),
        Some("Updated Remote Lamp")
    );

    servient.unregister("urn:thing:remote-lamp").unwrap();
    assert_eq!(servient.consumed_cache().removed, 1);
    assert!(servient.consumed_cache().inner.is_empty());
    let err = match servient.consume("urn:thing:remote-lamp") {
        Ok(_) => panic!("unregistered Thing should not be consumable"),
        Err(err) => err,
    };
    assert!(matches!(err, ServientError::Discovery(_)));
}

#[test]
fn consume_prefers_cached_td_when_present() {
    let (directory_td, _) = thing("urn:thing:remote-lamp", "Directory Lamp");
    let (cached_td, _) = thing("urn:thing:remote-lamp", "Cached Lamp");
    let mut cache = InMemoryConsumedThingCache::new();
    cache.insert("urn:thing:remote-lamp".to_owned(), cached_td);
    let mut servient = Servient::builder().with_consumed_cache(cache).build();
    servient.register(directory_td).unwrap();
    let (cached_td, _) = thing("urn:thing:remote-lamp", "Cached Lamp");
    servient
        .consumed_cache_mut()
        .insert("urn:thing:remote-lamp".to_owned(), cached_td);

    let consumed = servient.consume("urn:thing:remote-lamp").unwrap();

    assert_eq!(
        consumed.thing_description()._metadata.title.as_deref(),
        Some("Cached Lamp")
    );
}

#[test]
fn lifecycle_start_stop_are_idempotent_and_guard_runtime_composition() {
    let (td, _) = thing("urn:thing:remote-lamp", "Remote Lamp");
    let (updated_td, _) = thing("urn:thing:remote-lamp", "Updated Remote Lamp");
    let (local_td, _) = thing("urn:thing:local-lamp", "Local Lamp");
    let (new_td, _) = thing("urn:thing:new-lamp", "New Lamp");
    let mut servient = Servient::new();

    servient.register(td).unwrap();
    servient.expose(LocalThing::new(local_td.clone())).unwrap();
    servient.start().unwrap();
    servient.start().unwrap();
    assert!(servient.is_running());

    let err = servient.register(new_td).unwrap_err();
    assert!(matches!(err, ServientError::Running));

    let err = servient.update(updated_td).unwrap_err();
    assert!(matches!(err, ServientError::Running));

    let err = servient.unregister("urn:thing:remote-lamp").unwrap_err();
    assert!(matches!(err, ServientError::Running));

    let err = servient.expose(LocalThing::new(local_td)).unwrap_err();
    assert!(matches!(err, ServientError::Running));

    let err = match servient.unexpose("urn:thing:local-lamp") {
        Ok(_) => panic!("running Servient should reject unexpose"),
        Err(err) => err,
    };
    assert!(matches!(err, ServientError::Running));

    let err = servient
        .register_binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"late".to_vec(), "text/plain"),
            })
        })
        .unwrap_err();
    assert!(matches!(err, ServientError::Running));

    servient.stop().unwrap();
    servient.stop().unwrap();
    assert!(!servient.is_running());

    servient.unregister("urn:thing:remote-lamp").unwrap();
    servient.unexpose("urn:thing:local-lamp").unwrap();
}
