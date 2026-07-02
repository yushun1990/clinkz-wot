mod support;

use std::{
    cell::Cell,
    collections::BTreeMap,
    rc::Rc,
    sync::{Arc, Mutex},
};

use clinkz_wot_core::{EventName, InteractionInput, Payload, PublisherSink, ThingId};
use clinkz_wot_discovery::ThingFilter;
use clinkz_wot_protocol_bindings::{BindingError, FormSelectionCriteria};
use clinkz_wot_servient::{Servient, ServientError};
use clinkz_wot_td::{
    affordance::{InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    data_type::Operation,
    form::Form,
    thing::Thing,
};

use support::*;

#[test]
fn exposes_local_thing_and_dispatches_handlers_through_handle() {
    let (td, _) = thing("urn:thing:local-lamp", "Local Lamp");
    let servient = Servient::new();
    let handle = servient.expose(td).unwrap();
    let (status_read, status_write) = shared_status(Payload::new(b"off".to_vec(), "text/plain"));
    handle
        .set_property_read_handler("status", status_read)
        .unwrap();
    handle
        .set_property_write_handler("status", status_write)
        .unwrap();
    handle.set_action_handler("echo", EchoAction).unwrap();
    handle
        .set_event_subscribe_handler("startup", StartupEvent)
        .unwrap();

    let payload = handle
        .read_property("status", InteractionInput::empty())
        .unwrap()
        .payload
        .unwrap();
    assert_eq!(payload.body.as_ref(), b"off");
    assert_eq!(servient.list().total, 1);

    handle
        .write_property(
            "status",
            InteractionInput::with_payload(Payload::new(b"on".to_vec(), "text/plain")),
        )
        .unwrap();
    let payload = handle
        .read_property("status", InteractionInput::empty())
        .unwrap()
        .payload
        .unwrap();
    assert_eq!(payload.body.as_ref(), b"on");

    let payload = handle
        .invoke_action(
            "echo",
            InteractionInput::with_payload(Payload::new(b"hello".to_vec(), "text/plain")),
        )
        .unwrap()
        .payload
        .unwrap();
    assert_eq!(payload.body.as_ref(), b"hello");

    let mut sink = CollectSink::default();
    handle
        .subscribe_event("startup", InteractionInput::empty(), &mut sink)
        .unwrap();
    assert_eq!(sink.payloads[0].body.as_ref(), b"ready");
}

#[test]
fn consumed_handle_reads_remote_property_through_registered_binding() {
    let (td, _) = thing("urn:thing:remote-lamp", "Remote Lamp");
    let servient = Servient::builder()
        .binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"on".to_vec(), "text/plain"),
            })
        })
        .build();

    let consumed = servient.consume(td).unwrap();
    let output = consumed
        .read_property_with_criteria(
            "status",
            FormSelectionCriteria::new(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();

    assert_eq!(output.payload.unwrap().body.as_ref(), b"on");
}

#[test]
fn consumed_handle_routes_all_operations_through_registered_bindings() {
    let (td, _) = thing("urn:thing:remote-lamp", "Remote Lamp");
    let servient = Servient::builder()
        .binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"on".to_vec(), "text/plain"),
            })
        })
        .build();

    let consumed = servient.consume(td).unwrap();
    let read = consumed
        .read_property_with_criteria(
            "status",
            FormSelectionCriteria::new(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(read.payload.unwrap().body.as_ref(), b"on");

    consumed
        .write_property_with_criteria(
            "status",
            FormSelectionCriteria::new(Operation::WriteProperty).content_type("text/plain"),
            InteractionInput::with_payload(Payload::new(b"off".to_vec(), "text/plain")),
        )
        .unwrap();

    let action = consumed
        .invoke_action_with_criteria(
            "echo",
            FormSelectionCriteria::new(Operation::InvokeAction).content_type("text/plain"),
            InteractionInput::with_payload(Payload::new(b"hello".to_vec(), "text/plain")),
        )
        .unwrap();
    assert_eq!(action.payload.unwrap().body.as_ref(), b"hello");

    let event_sub = consumed
        .subscribe_event_with_criteria(
            "startup",
            FormSelectionCriteria::new(Operation::SubscribeEvent).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();
    let event_payload = event_sub
        .poll_next()
        .expect("subscription should have a sample");
    assert_eq!(event_payload.body.as_ref(), b"subscribed");
}

#[test]
fn payload_codecs_are_used_for_remote_interactions() {
    let encode_calls = Rc::new(Cell::new(0));
    let decode_calls = Rc::new(Cell::new(0));

    let (remote_td, _) = thing("urn:thing:remote-codec-lamp", "Remote Codec Lamp");
    let servient = Servient::builder()
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

    let consumed = servient.consume(remote_td).unwrap();
    let remote = consumed
        .read_property_with_criteria(
            "status",
            FormSelectionCriteria::new(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(remote.payload.unwrap().body.as_ref(), b"remote");
    assert_eq!(decode_calls.get(), 1);
    assert_eq!(encode_calls.get(), 1);
}

#[test]
fn normalize_payloads_false_skips_codec_round_trip() {
    let encode_calls = Rc::new(Cell::new(0));
    let decode_calls = Rc::new(Cell::new(0));

    let (remote_td, _) = thing("urn:thing:remote-no-norm", "Remote No-Norm Lamp");
    let servient = Servient::builder()
        .payload_codec(CountingCodec {
            encode_calls: encode_calls.clone(),
            decode_calls: decode_calls.clone(),
        })
        .normalize_payloads(false)
        .binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"remote".to_vec(), "text/plain"),
            })
        })
        .build();

    let consumed = servient.consume(remote_td).unwrap();
    let remote = consumed
        .read_property_with_criteria(
            "status",
            FormSelectionCriteria::new(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(remote.payload.unwrap().body.as_ref(), b"remote");
    assert_eq!(
        decode_calls.get(),
        0,
        "normalize_payloads(false) must skip the codec entirely"
    );
    assert_eq!(encode_calls.get(), 0);
}

#[test]
fn cbor_codec_canonicalizes_remote_application_cbor_payloads() {
    use clinkz_wot_codec_cbor::CborCodec;
    use clinkz_wot_td::affordance::PropertyAffordance;
    use clinkz_wot_td::data_schema::DataSchema;

    // Build a TD whose `status` property declares an `application/cbor` form
    // that the `test://` binding supports.
    let cbor_form = Form::read_property("test://things/cbor-lamp/properties/status")
        .content_type("application/cbor")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::string())
        .form(cbor_form)
        .build()
        .unwrap();
    let remote_td = Thing::builder("Remote CBOR Lamp")
        .id("urn:thing:remote-cbor-lamp")
        .nosec()
        .property("status", property)
        .build()
        .unwrap();

    // The remote binding returns a non-minimal CBOR integer (1 encoded with
    // the explicit one-byte-follow form 0x18 0x01 instead of the minimal
    // single-byte 0x01). The registered CborCodec must canonicalize the
    // response before the consumer sees it.
    let servient = Servient::builder()
        .payload_codec(CborCodec::new())
        .binding_factory(|| {
            Box::new(support::TestBinding {
                response: Payload::new(vec![0x18, 0x01], "application/cbor"),
            })
        })
        .build();

    let consumed = servient.consume(remote_td).unwrap();
    let remote = consumed
        .read_property_with_criteria(
            "status",
            FormSelectionCriteria::new(Operation::ReadProperty).content_type("application/cbor"),
            InteractionInput::empty(),
        )
        .unwrap();
    let payload = remote.payload.expect("CBOR response should be present");
    assert_eq!(payload.content_type, "application/cbor");
    // Non-minimal 0x18 0x01 must have been canonicalized to 0x01.
    assert_eq!(payload.body.as_ref(), &[0x01]);
}

#[test]
fn security_providers_are_used_for_remote_interactions() {
    let applied_calls = Rc::new(Cell::new(0));

    let (remote_td, remote_form) =
        secure_thing("urn:thing:remote-secure-lamp", "Remote Secure Lamp");
    let servient = Servient::builder()
        .security_provider(RecordingSecurityProvider {
            applied_calls: applied_calls.clone(),
        })
        .binding_factory(|| Box::new(AuthenticatedReadBinding))
        .build();

    let consumed = servient.consume(remote_td).unwrap();
    // The secure form is the only one, so default criteria selects it.
    let _ = remote_form;
    let remote = consumed
        .read_property("status", InteractionInput::empty())
        .unwrap();
    assert_eq!(remote.payload.unwrap().body.as_ref(), b"secure-remote");
    assert_eq!(applied_calls.get(), 1);
}

#[test]
fn consumed_handle_reports_binding_selection_errors() {
    let (td, _) = thing("urn:thing:remote-lamp", "Remote Lamp");
    let servient = Servient::builder()
        .binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"on".to_vec(), "text/plain"),
            })
        })
        .build();

    let consumed = servient.consume(td).unwrap();
    let err = consumed
        .read_property_with_criteria(
            "status",
            FormSelectionCriteria::new(Operation::ReadProperty).content_type("image/png"),
            InteractionInput::empty(),
        )
        .unwrap_err();

    assert!(matches!(
        err,
        clinkz_wot_servient::ServientError::Binding(BindingError::MetadataMismatch(_))
    ));
}

#[test]
fn consumed_handle_reuses_cached_binding_plans() {
    let (td, _) = thing("urn:thing:planned-lamp", "Planned Lamp");
    let unsupported_calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let unsupported_factory_calls = unsupported_calls.clone();
    let unsupported_probe_calls = unsupported_calls.clone();
    let servient = Servient::builder()
        .binding_factory_with_support(
            move || {
                Box::new(CountingUnsupportedBinding {
                    supports_calls: unsupported_factory_calls.clone(),
                })
            },
            move |_thing, _form, _operation| {
                unsupported_probe_calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                false
            },
        )
        .binding_factory_with_support(
            || {
                Box::new(CountingHrefBinding {
                    supports_calls: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                })
            },
            |_thing, form, operation| {
                form.href.as_str().starts_with("test://") && operation == Operation::ReadProperty
            },
        )
        .build();

    let consumed = servient.consume(td).unwrap();
    let criteria = FormSelectionCriteria::new(Operation::ReadProperty).content_type("text/plain");
    let read = consumed
        .read_property_with_criteria("status", criteria, InteractionInput::empty())
        .unwrap();
    assert_eq!(
        read.payload.unwrap().body.as_ref(),
        b"test://things/lamp/properties/status"
    );
    assert_eq!(
        unsupported_calls.load(std::sync::atomic::Ordering::Relaxed),
        1
    );

    let read = consumed
        .read_property_with_criteria("status", criteria, InteractionInput::empty())
        .unwrap();
    assert_eq!(
        read.payload.unwrap().body.as_ref(),
        b"test://things/lamp/properties/status"
    );
    assert_eq!(
        unsupported_calls.load(std::sync::atomic::Ordering::Relaxed),
        1,
        "cached plan should skip unsupported factories entirely"
    );
}

#[test]
fn late_binding_factory_registration_is_used_by_consumed_handle() {
    let (td, _) = thing("urn:thing:remote-lamp", "Remote Lamp");
    let servient = Servient::new();
    servient
        .register_binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"late".to_vec(), "text/plain"),
            })
        })
        .unwrap();

    let consumed = servient.consume(td).unwrap();
    let output = consumed
        .read_property_with_criteria(
            "status",
            FormSelectionCriteria::new(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();

    assert_eq!(output.payload.unwrap().body.as_ref(), b"late");
}

#[test]
fn consumed_handle_reports_missing_bindings() {
    let (td, _) = thing("urn:thing:remote-lamp", "Remote Lamp");
    let servient = Servient::new();

    let consumed = servient.consume(td).unwrap();
    let err = consumed
        .read_property_with_criteria(
            "status",
            FormSelectionCriteria::new(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap_err();
    assert!(matches!(err, clinkz_wot_servient::ServientError::Serve(_)));
}

#[test]
fn unexposes_local_thing_and_removes_directory_entry() {
    let (td, _) = thing("urn:thing:local-lamp", "Local Lamp");
    let servient = Servient::new();
    let handle = servient.expose(td).unwrap();

    let removed_id = servient.unexpose(handle.thing_id()).unwrap();
    assert_eq!(removed_id, "urn:thing:local-lamp");
    assert_eq!(servient.list().total, 0);
}

#[test]
fn exposed_handle_is_clone_and_shares_live_state() {
    let (td, _) = thing("urn:thing:local-lamp", "Local Lamp");
    let servient = Servient::new();
    let handle = servient.expose(td).unwrap();
    handle
        .set_property_read_handler(
            "status",
            StatusRead {
                value: Payload::new(b"off".to_vec(), "text/plain"),
            },
        )
        .unwrap();

    // A cheap clone shares the live state: a handler attached through one clone
    // is visible to the other (baseline §6 / §7).
    let clone = handle.clone();
    let payload = clone
        .read_property("status", InteractionInput::empty())
        .unwrap()
        .payload
        .unwrap();
    assert_eq!(payload.body.as_ref(), b"off");
}

#[test]
fn servient_clone_shares_directory_and_bindings() {
    let (td, _) = thing("urn:thing:remote-lamp", "Remote Lamp");
    let servient = Servient::builder()
        .binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"on".to_vec(), "text/plain"),
            })
        })
        .build();

    let clone = servient.clone();
    let consumed = clone.consume(td).unwrap();
    let read = consumed
        .read_property_with_criteria(
            "status",
            FormSelectionCriteria::new(Operation::ReadProperty).content_type("text/plain"),
            InteractionInput::empty(),
        )
        .unwrap();
    assert_eq!(read.payload.unwrap().body.as_ref(), b"on");
}

#[test]
fn local_interaction_skips_transport_security() {
    // Local in-process interactions go directly to the handler without applying
    // transport security (baseline §6). A secure local Thing's handler runs
    // even though no security provider would set the expected parameter.
    let (td, _) = secure_thing("urn:thing:local-secure-lamp", "Local Secure Lamp");
    let servient = Servient::new();
    let handle = servient.expose(td).unwrap();
    handle
        .set_property_read_handler("status", LocalUnsecuredStatusProperty)
        .unwrap();

    let payload = handle
        .read_property("status", InteractionInput::empty())
        .unwrap()
        .payload
        .unwrap();
    assert_eq!(payload.body.as_ref(), b"local-direct");
}

#[test]
fn dispatch_to_unhandled_affordance_errors() {
    let (td, _) = thing("urn:thing:local-lamp", "Local Lamp");
    let servient = Servient::new();
    let handle = servient.expose(td).unwrap();
    // No property handler attached.

    let err = handle
        .read_property("status", InteractionInput::empty())
        .unwrap_err();
    assert!(matches!(
        err,
        clinkz_wot_servient::ServientError::Serve(
            clinkz_wot_core::CoreError::MissingHandler { .. }
        )
    ));
}

#[test]
fn dispatch_to_different_things_does_not_contend() {
    // Interactions against different Things use independent per-Thing locks
    // (baseline §7). Dispatching to two Things sequentially completes without
    // contention errors.
    let (td_a, _) = thing("urn:thing:lamp-a", "Lamp A");
    let (td_b, _) = thing("urn:thing:lamp-b", "Lamp B");
    let servient = Servient::new();

    let handle_a = servient.expose(td_a).unwrap();
    handle_a
        .set_property_read_handler(
            "status",
            StatusRead {
                value: Payload::new(b"a".to_vec(), "text/plain"),
            },
        )
        .unwrap();

    let handle_b = servient.expose(td_b).unwrap();
    handle_b
        .set_property_read_handler(
            "status",
            StatusRead {
                value: Payload::new(b"b".to_vec(), "text/plain"),
            },
        )
        .unwrap();

    // Dispatch to both — different Things, no contention.
    let payload_a = handle_a
        .read_property("status", InteractionInput::empty())
        .unwrap()
        .payload
        .unwrap();
    let payload_b = handle_b
        .read_property("status", InteractionInput::empty())
        .unwrap()
        .payload
        .unwrap();
    assert_eq!(payload_a.body.as_ref(), b"a");
    assert_eq!(payload_b.body.as_ref(), b"b");
}

#[test]
fn dispatch_within_one_thing_serializes() {
    // Interactions against the same Thing serialize through the per-Thing lock
    // (baseline §7). A handler that observes its own prior write confirms
    // sequential execution.
    let (td, _) = thing("urn:thing:counter", "Counter");
    let servient = Servient::new();
    let handle = servient.expose(td).unwrap();
    let (status_read, status_write) = shared_status(Payload::new(b"0".to_vec(), "text/plain"));
    handle
        .set_property_read_handler("status", status_read)
        .unwrap();
    handle
        .set_property_write_handler("status", status_write)
        .unwrap();

    // Write then read — the read must see the written value.
    handle
        .write_property(
            "status",
            InteractionInput::with_payload(Payload::new(b"42".to_vec(), "text/plain")),
        )
        .unwrap();
    let payload = handle
        .read_property("status", InteractionInput::empty())
        .unwrap()
        .payload
        .unwrap();
    assert_eq!(payload.body.as_ref(), b"42");
}

#[test]
fn destroy_from_within_handler_does_not_self_deadlock() {
    // A handler calling destroy(own_id) while its per-Thing lock is held must
    // not self-deadlock (baseline §7 edge case). The handler sets the drain
    // flag; the dispatch epilogue completes the removal.
    let (td, _) = thing("urn:thing:self-destroy", "Self Destroy");
    let servient = Servient::new();
    let handle = servient.expose(td).unwrap();
    let destroyed = Rc::new(Cell::new(false));
    handle
        .set_action_handler(
            "echo",
            SelfDestroyingAction {
                servient: servient.clone(),
                thing_id: handle.thing_id().to_string(),
                destroyed: destroyed.clone(),
            },
        )
        .unwrap();

    // Invoke the action — the handler calls destroy(own_id) inside.
    let payload = handle
        .invoke_action("echo", InteractionInput::empty())
        .unwrap()
        .payload
        .unwrap();
    assert_eq!(payload.body.as_ref(), b"destroyed");
    assert!(
        destroyed.get(),
        "destroy should succeed from within handler"
    );

    // After the handler returns, the Thing is gone: a subsequent dispatch
    // reports ExposedThingNotFound.
    let err = handle
        .read_property("status", InteractionInput::empty())
        .unwrap_err();
    assert!(matches!(
        err,
        clinkz_wot_servient::ServientError::ExposedThingNotFound(_)
    ));
}

#[test]
fn handler_reentering_registry_does_not_self_deadlock() {
    // C7 regression: a sync handler that calls back into the Servient for the
    // SAME Thing — here, a property read handler that reads another property
    // through the handle — must not self-deadlock. Before the reentrancy fix
    // the handler ran under the per-Thing slot lock, so re-entering the
    // registry deadlocked on `std` / panicked on `no_std`.
    use clinkz_wot_core::{CoreError, CoreResult, InteractionOutput, PropertyReadHandler};
    use clinkz_wot_discovery::InMemoryThingDirectory;
    use clinkz_wot_servient::ExposedThingHandle;

    type Handle = ExposedThingHandle<InMemoryThingDirectory>;

    let make_prop = || {
        PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
            .form(
                Form::read_property("test://t/properties/x")
                    .content_type("text/plain")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
    };
    let td = Thing::builder("Reentrant")
        .id("urn:thing:reentrant")
        .nosec()
        .property("a", make_prop())
        .property("b", make_prop())
        .build()
        .unwrap();

    let servient = Servient::new();
    let handle: Handle = servient.expose(td).unwrap();

    // Reading "b" returns a fixed value.
    handle
        .set_property_read_handler(
            "b",
            StatusRead {
                value: Payload::new(b"from-b".to_vec(), "text/plain"),
            },
        )
        .unwrap();

    // Reading "a" re-enters the registry to read "b" on the same Thing.
    struct ReentrantReader {
        handle: Handle,
    }
    impl PropertyReadHandler for ReentrantReader {
        fn read(&self, _input: InteractionInput) -> CoreResult<InteractionOutput> {
            self.handle
                .read_property("b", InteractionInput::empty())
                .map_err(|e| CoreError::InboundDispatch(e.to_string()))
        }
    }
    handle
        .set_property_read_handler(
            "a",
            ReentrantReader {
                handle: handle.clone(),
            },
        )
        .unwrap();

    let body = handle
        .read_property("a", InteractionInput::empty())
        .expect("reentrant read must not deadlock")
        .payload
        .unwrap()
        .body;
    assert_eq!(body.as_ref(), b"from-b");
}

#[test]
fn repeated_consume_shares_interned_instance() {
    // Baseline v3.0 §5.1: consume() of the same Thing returns handles that
    // share one canonical live entry. A binding plan cached during an
    // interaction through one handle must be reused (not recomputed) when a
    // second handle to the same Thing interacts.
    let (td, _) = thing("urn:thing:interned-lamp", "Interned Lamp");
    let unsupported_calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let unsupported_factory_calls = unsupported_calls.clone();
    let unsupported_probe_calls = unsupported_calls.clone();
    let servient = Servient::builder()
        .binding_factory_with_support(
            move || {
                Box::new(CountingUnsupportedBinding {
                    supports_calls: unsupported_factory_calls.clone(),
                })
            },
            move |_thing, _form, _operation| {
                unsupported_probe_calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                false
            },
        )
        .binding_factory_with_support(
            || {
                Box::new(CountingHrefBinding {
                    supports_calls: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                })
            },
            |_thing, form, operation| {
                form.href.as_str().starts_with("test://") && operation == Operation::ReadProperty
            },
        )
        .build();

    let consumed_a = servient.consume(td.clone()).unwrap();
    let consumed_b = servient.consume(td).unwrap();

    let criteria = FormSelectionCriteria::new(Operation::ReadProperty).content_type("text/plain");

    // First interaction caches the binding plan in the shared entry.
    consumed_a
        .read_property_with_criteria("status", criteria, InteractionInput::empty())
        .unwrap();
    assert_eq!(
        unsupported_calls.load(std::sync::atomic::Ordering::Relaxed),
        1
    );

    // Second interaction through a different handle reuses the cached plan
    // from the same interned entry — the unsupported factory is not probed
    // again.
    consumed_b
        .read_property_with_criteria("status", criteria, InteractionInput::empty())
        .unwrap();
    assert_eq!(
        unsupported_calls.load(std::sync::atomic::Ordering::Relaxed),
        1,
        "second handle should share the interned entry's cached plan"
    );
}

#[test]
fn invalidate_clears_interned_entry() {
    // Baseline v3.0 §5.2: invalidate(id) removes the interned entry so the
    // next consume() rebuilds form selections and binding plans.
    let (td, _) = thing("urn:thing:invalidated-lamp", "Invalidated Lamp");
    let unsupported_calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let unsupported_factory_calls = unsupported_calls.clone();
    let unsupported_probe_calls = unsupported_calls.clone();
    let servient = Servient::builder()
        .binding_factory_with_support(
            move || {
                Box::new(CountingUnsupportedBinding {
                    supports_calls: unsupported_factory_calls.clone(),
                })
            },
            move |_thing, _form, _operation| {
                unsupported_probe_calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                false
            },
        )
        .binding_factory_with_support(
            || {
                Box::new(CountingHrefBinding {
                    supports_calls: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                })
            },
            |_thing, form, operation| {
                form.href.as_str().starts_with("test://") && operation == Operation::ReadProperty
            },
        )
        .build();

    let consumed = servient.consume(td.clone()).unwrap();
    let criteria = FormSelectionCriteria::new(Operation::ReadProperty).content_type("text/plain");

    // First interaction caches the binding plan.
    consumed
        .read_property_with_criteria("status", criteria, InteractionInput::empty())
        .unwrap();
    assert_eq!(
        unsupported_calls.load(std::sync::atomic::Ordering::Relaxed),
        1
    );

    // Invalidate the interned entry.
    servient.invalidate(consumed.thing_id());

    // Re-consume creates a fresh entry — the binding plan is recomputed.
    let consumed_new = servient.consume(td).unwrap();
    consumed_new
        .read_property_with_criteria("status", criteria, InteractionInput::empty())
        .unwrap();
    assert_eq!(
        unsupported_calls.load(std::sync::atomic::Ordering::Relaxed),
        2,
        "invalidate should force recompute on re-consume"
    );
}

// ===========================================================================
// SR-P2: Driving layer and expose/destroy coordination tests
// ===========================================================================

use clinkz_wot_core::{AffordanceTarget, CorrelationId, InboundRequest};

fn fake_server() -> std::sync::Arc<FakeServerBinding> {
    #[allow(clippy::arc_with_non_send_sync)]
    {
        std::sync::Arc::new(FakeServerBinding::default())
    }
}

fn fake_server_failing_routes() -> std::sync::Arc<FakeServerBinding> {
    #[allow(clippy::arc_with_non_send_sync)]
    {
        std::sync::Arc::new(FakeServerBinding {
            route_registration_fails: true,
            ..Default::default()
        })
    }
}

#[test]
fn poll_serve_sync_dispatches_read_property() {
    let (td, _) = thing("urn:thing:driving-1", "Driving Test 1");
    let server_binding = fake_server();
    let servient = Servient::builder()
        .server_binding(server_binding.clone())
        .build();
    let handle = servient.expose(td).unwrap();
    handle
        .set_property_read_handler(
            "status",
            StatusRead {
                value: Payload::new(b"on".to_vec(), "text/plain"),
            },
        )
        .unwrap();

    server_binding.enqueue(InboundRequest::new(
        ThingId::from("urn:thing:driving-1"),
        AffordanceTarget::Property("status".into()),
        Operation::ReadProperty,
        InteractionInput::empty(),
    ));

    servient.poll_serve_sync().unwrap();

    let responses = server_binding.take_responses();
    assert_eq!(responses.len(), 1);
    let response = &responses[0];
    assert!(response.error.is_none());
    assert_eq!(response.output.payload.as_ref().unwrap().body.as_ref(), b"on");
}

#[test]
fn poll_serve_sync_dispatches_readallproperties_as_combined_bulk_response() {
    // A Thing with two properties plus a Thing-level `readallproperties` form.
    // The inbound `ReadAllProperties` request must fan out across the property
    // read handlers and combine their outputs into one JSON-object response.
    let td = Thing::builder("Bulk Read Server")
        .id("urn:thing:bulk-read-server")
        .nosec()
        .property(
            "a",
            PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
                .form(
                    Form::read_property("test://things/bulk-read/properties/a")
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .property(
            "b",
            PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
                .form(
                    Form::read_property("test://things/bulk-read/properties/b")
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .form(
            Form::builder("test://things/bulk-read/properties")
                .op([Operation::ReadAllProperties])
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let server_binding = fake_server();
    let servient = Servient::builder()
        .server_binding(server_binding.clone())
        .build();
    let handle = servient.expose(td).unwrap();
    handle
        .set_property_read_handler(
            "a",
            StatusRead {
                value: Payload::new(b"1".to_vec(), "application/json"),
            },
        )
        .unwrap();
    handle
        .set_property_read_handler(
            "b",
            StatusRead {
                value: Payload::new(b"2".to_vec(), "application/json"),
            },
        )
        .unwrap();

    server_binding.enqueue(InboundRequest::new(
        ThingId::from("urn:thing:bulk-read-server"),
        AffordanceTarget::Thing,
        Operation::ReadAllProperties,
        InteractionInput::empty(),
    ));

    servient.poll_serve_sync().unwrap();

    let responses = server_binding.take_responses();
    assert_eq!(responses.len(), 1);
    let response = &responses[0];
    assert!(response.error.is_none(), "got error: {:?}", response.error);
    let combined: serde_json::Value = serde_json::from_slice(
        response
            .output
            .payload
            .as_ref()
            .expect("combined payload")
            .body
            .as_ref(),
    )
    .expect("bulk response is valid JSON");
    let map = combined
        .as_object()
        .expect("bulk response is a JSON object");
    assert_eq!(map.get("a"), Some(&serde_json::json!(1)));
    assert_eq!(map.get("b"), Some(&serde_json::json!(2)));
}

#[test]
fn poll_serve_sync_returns_missing_handler_for_unhandled_affordance() {
    let (td, _) = thing("urn:thing:driving-2", "Driving Test 2");
    let server_binding = fake_server();
    let servient = Servient::builder()
        .server_binding(server_binding.clone())
        .build();
    servient.expose(td).unwrap();

    server_binding.enqueue(InboundRequest::new(
        ThingId::from("urn:thing:driving-2"),
        AffordanceTarget::Property("status".into()),
        Operation::ReadProperty,
        InteractionInput::empty(),
    ));

    servient.poll_serve_sync().unwrap();

    let responses = server_binding.take_responses();
    assert_eq!(responses.len(), 1);
    assert!(matches!(
        responses[0].error,
        Some(clinkz_wot_core::CoreError::MissingHandler { .. })
    ));
}

#[test]
fn poll_serve_sync_returns_error_for_unknown_thing() {
    let server_binding = fake_server();
    let servient = Servient::builder()
        .server_binding(server_binding.clone())
        .build();

    server_binding.enqueue(InboundRequest::new(
        ThingId::from("urn:thing:nonexistent"),
        AffordanceTarget::Property("status".into()),
        Operation::ReadProperty,
        InteractionInput::empty(),
    ));

    servient.poll_serve_sync().unwrap();

    let responses = server_binding.take_responses();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].error.is_some());
    assert!(responses[0].output.payload.is_none());
}

#[test]
fn poll_serve_sync_echoes_correlation_id() {
    let (td, _) = thing("urn:thing:driving-3", "Driving Test 3");
    let server_binding = fake_server();
    let servient = Servient::builder()
        .server_binding(server_binding.clone())
        .build();
    let handle = servient.expose(td).unwrap();
    handle
        .set_property_read_handler(
            "status",
            StatusRead {
                value: Payload::new(b"ok".to_vec(), "text/plain"),
            },
        )
        .unwrap();

    let correlation = CorrelationId::from(42u64);
    server_binding.enqueue(InboundRequest::new(
        ThingId::from("urn:thing:driving-3"),
        AffordanceTarget::Property("status".into()),
        Operation::ReadProperty,
        InteractionInput::empty(),
    ));
    // Override correlation via struct mutation
    {
        let mut req = server_binding.pending_requests.lock().unwrap();
        req.front_mut().unwrap().correlation = correlation.clone();
    }

    servient.poll_serve_sync().unwrap();

    let responses = server_binding.take_responses();
    assert_eq!(responses[0].correlation, correlation);
}

#[test]
fn expose_registers_routes_on_server_binding() {
    let (td, _) = thing("urn:thing:driving-4", "Driving Test 4");
    let server_binding = fake_server();
    let servient = Servient::builder()
        .server_binding(server_binding.clone())
        .build();
    servient.expose(td).unwrap();

    let registered = server_binding.registered_things.lock().unwrap();
    assert_eq!(registered.len(), 1);
    assert_eq!(registered[0], "urn:thing:driving-4");
}

#[test]
fn destroy_unregisters_routes_on_server_binding() {
    let (td, _) = thing("urn:thing:driving-5", "Driving Test 5");
    let server_binding = fake_server();
    let servient = Servient::builder()
        .server_binding(server_binding.clone())
        .build();
    servient.expose(td).unwrap();
    servient.destroy("urn:thing:driving-5").unwrap();

    let unregistered = server_binding.unregistered_things.lock().unwrap();
    assert_eq!(unregistered.len(), 1);
    assert_eq!(unregistered[0], "urn:thing:driving-5");
}

#[test]
fn expose_rolls_back_on_route_registration_failure() {
    let (td, _) = thing("urn:thing:driving-6", "Driving Test 6");
    let server_binding = fake_server_failing_routes();
    let servient = Servient::builder()
        .server_binding(server_binding.clone())
        .build();

    let result = servient.expose(td);
    assert!(matches!(
        result,
        Err(clinkz_wot_servient::ServientError::RouteRegistration(_))
    ));

    // The Thing should not be exposed.
    let directory = servient.list();
    assert_eq!(directory.entries.len(), 0);
}

#[test]
fn produce_creates_thing_without_registering_routes() {
    let (td, _) = thing("urn:thing:produce-1", "Produce Test 1");
    let server_binding = fake_server();
    let servient = Servient::builder()
        .server_binding(server_binding.clone())
        .build();

    let handle = servient.produce(td).unwrap();

    // Routes are NOT registered after produce alone.
    let registered = server_binding.registered_things.lock().unwrap();
    assert!(
        registered.is_empty(),
        "produce must not register routes on server bindings"
    );
    drop(registered);

    // Directory is NOT populated after produce alone.
    let directory = servient.list();
    assert_eq!(directory.entries.len(), 0);

    // Local interactions work after produce (no handler yet → MissingHandler
    // is expected, but the Thing is found, not "unknown").
    let result = handle.read_property("status", InteractionInput::empty());
    assert!(result.is_err());
}

#[test]
fn produce_then_expose_registers_routes_and_publishes_directory() {
    let (td, _) = thing("urn:thing:produce-2", "Produce Test 2");
    let server_binding = fake_server();
    let servient = Servient::builder()
        .server_binding(server_binding.clone())
        .build();

    let handle = servient.produce(td).unwrap();

    // Register a handler BEFORE expose — the Thing is not yet
    // network-reachable, so there is no window where remote callers hit an
    // unhandlered Thing.
    handle
        .set_property_read_handler(
            "status",
            StatusRead {
                value: Payload::new(b"on".to_vec(), "text/plain"),
            },
        )
        .unwrap();

    // Start serving.
    handle.expose().unwrap();

    // Now routes are registered.
    let registered = server_binding.registered_things.lock().unwrap();
    assert_eq!(registered.len(), 1);
    assert_eq!(registered[0], "urn:thing:produce-2");
    drop(registered);

    // Directory is populated.
    let directory = servient.list();
    assert_eq!(directory.entries.len(), 1);

    // Local interaction works with the handler registered before expose.
    let output = handle
        .read_property("status", InteractionInput::empty())
        .unwrap();
    assert_eq!(output.payload.unwrap().body.as_ref(), b"on");
}

#[test]
fn handle_expose_rolls_back_on_route_registration_failure() {
    let (td, _) = thing("urn:thing:produce-3", "Produce Test 3");
    let server_binding = fake_server_failing_routes();
    let servient = Servient::builder()
        .server_binding(server_binding.clone())
        .build();

    let handle = servient.produce(td).unwrap();

    let result = handle.expose();
    assert!(matches!(
        result,
        Err(clinkz_wot_servient::ServientError::RouteRegistration(_))
    ));

    // The Thing is still in the registry (produce succeeded), but routes are
    // not registered and directory is not populated.
    assert_eq!(servient.list().entries.len(), 0);
}

// ===========================================================================
// SR-P3: Directory-driven invalidation tests (baseline addendum §3)
// ===========================================================================

/// Shared helper: set up a Servient with a counting binding factory and
/// return (servient, unsupported_calls_rc).
fn invalidation_test_servient() -> (Servient, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
    let unsupported_calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let factory_calls = unsupported_calls.clone();
    let probe_calls = unsupported_calls.clone();
    let servient = Servient::builder()
        .binding_factory_with_support(
            move || {
                Box::new(CountingUnsupportedBinding {
                    supports_calls: factory_calls.clone(),
                })
            },
            move |_thing, _form, _operation| {
                probe_calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                false
            },
        )
        .binding_factory_with_support(
            || {
                Box::new(CountingHrefBinding {
                    supports_calls: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                })
            },
            |_thing, form, operation| {
                form.href.as_str().starts_with("test://") && operation == Operation::ReadProperty
            },
        )
        .build();
    (servient, unsupported_calls)
}

#[test]
fn directory_update_invalidates_consumed_thing() {
    let (td, _) = thing("urn:thing:dir-update", "Dir Update Lamp");
    let (servient, unsupported_calls) = invalidation_test_servient();

    // Register in directory first so update can find it.
    servient.register(td.clone()).unwrap();

    let consumed = servient.consume(td.clone()).unwrap();
    let criteria = FormSelectionCriteria::new(Operation::ReadProperty).content_type("text/plain");

    // First interaction caches the binding plan.
    consumed
        .read_property_with_criteria("status", criteria, InteractionInput::empty())
        .unwrap();
    assert_eq!(
        unsupported_calls.load(std::sync::atomic::Ordering::Relaxed),
        1
    );

    // Directory update triggers invalidation (addendum §3).
    servient.update(td).unwrap();

    // Re-consume creates a fresh entry — binding plan is recomputed.
    let consumed_new = servient
        .consume(thing("urn:thing:dir-update", "Dir Update Lamp").0)
        .unwrap();
    consumed_new
        .read_property_with_criteria("status", criteria, InteractionInput::empty())
        .unwrap();
    assert_eq!(
        unsupported_calls.load(std::sync::atomic::Ordering::Relaxed),
        2,
        "directory update should invalidate consumed entry"
    );
}

#[test]
fn directory_unregister_invalidates_consumed_thing() {
    let (td, _) = thing("urn:thing:dir-delete", "Dir Delete Lamp");
    let (servient, unsupported_calls) = invalidation_test_servient();

    // Register the TD in the directory first so unregister can find it.
    servient.register(td.clone()).unwrap();

    let consumed = servient.consume(td).unwrap();
    let criteria = FormSelectionCriteria::new(Operation::ReadProperty).content_type("text/plain");

    consumed
        .read_property_with_criteria("status", criteria, InteractionInput::empty())
        .unwrap();
    assert_eq!(
        unsupported_calls.load(std::sync::atomic::Ordering::Relaxed),
        1
    );

    // Directory delete triggers invalidation (addendum §3).
    servient.unregister("urn:thing:dir-delete").unwrap();

    // Re-consume creates a fresh entry.
    let consumed_new = servient
        .consume(thing("urn:thing:dir-delete", "Dir Delete Lamp").0)
        .unwrap();
    consumed_new
        .read_property_with_criteria("status", criteria, InteractionInput::empty())
        .unwrap();
    assert_eq!(
        unsupported_calls.load(std::sync::atomic::Ordering::Relaxed),
        2,
        "directory delete should invalidate consumed entry"
    );
}

#[test]
fn destroy_invalidates_consumed_thing() {
    let (td, _) = thing("urn:thing:destroy-inv", "Destroy Invalidate Lamp");
    let (servient, unsupported_calls) = invalidation_test_servient();

    // Expose the Thing (also publishes to directory).
    servient.expose(td.clone()).unwrap();

    // Consume the same Thing (gateway scenario).
    let consumed = servient.consume(td).unwrap();
    let criteria = FormSelectionCriteria::new(Operation::ReadProperty).content_type("text/plain");

    consumed
        .read_property_with_criteria("status", criteria, InteractionInput::empty())
        .unwrap();
    assert_eq!(
        unsupported_calls.load(std::sync::atomic::Ordering::Relaxed),
        1
    );

    // Destroy the exposed Thing — directory delete triggers invalidation.
    servient.destroy("urn:thing:destroy-inv").unwrap();

    // Re-consume creates a fresh entry.
    let consumed_new = servient
        .consume(thing("urn:thing:destroy-inv", "Destroy Invalidate Lamp").0)
        .unwrap();
    consumed_new
        .read_property_with_criteria("status", criteria, InteractionInput::empty())
        .unwrap();
    assert_eq!(
        unsupported_calls.load(std::sync::atomic::Ordering::Relaxed),
        2,
        "destroy should invalidate consumed entry"
    );
}

// ---------------------------------------------------------------------------
// Event pipeline tests (T1: EventBroker wiring + emit_event).
// ---------------------------------------------------------------------------

/// [`PublisherSink`] that records every published payload for test assertions.
#[derive(Clone)]
struct RecordingPublisherSink {
    received: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl RecordingPublisherSink {
    fn new() -> Self {
        Self {
            received: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn bodies(&self) -> Vec<Vec<u8>> {
        self.received.lock().unwrap().clone()
    }
}

impl PublisherSink for RecordingPublisherSink {
    fn publish(&self, payload: &Payload) -> clinkz_wot_core::CoreResult<()> {
        self.received.lock().unwrap().push(payload.body.as_ref().to_vec());
        Ok(())
    }
}

#[test]
fn emit_event_delivers_to_registered_publisher_sink() {
    let (td, _) = thing("urn:thing:event-emit", "Event Emit Lamp");
    let servient = Servient::new();
    let handle = servient.expose(td).unwrap();

    // Register a publisher sink on the broker for the "startup" event.
    let sink = RecordingPublisherSink::new();
    servient.event_broker().register(
        ThingId::from("urn:thing:event-emit"),
        EventName::from("startup"),
        sink.clone(),
    );

    // Emit through the handle.
    handle
        .emit_event("startup", Payload::new(b"hello".to_vec(), "text/plain"))
        .unwrap();

    assert_eq!(sink.bodies(), vec![b"hello".to_vec()]);
}

#[test]
fn emit_event_to_unknown_event_is_noop() {
    let (td, _) = thing("urn:thing:event-noop", "Event Noop Lamp");
    let servient = Servient::new();
    let handle = servient.expose(td).unwrap();

    // No sink registered — publish must succeed as no-op.
    handle
        .emit_event("unknown", Payload::new(b"data".to_vec(), "text/plain"))
        .unwrap();
}

#[test]
fn emit_property_change_routes_through_broker() {
    let (td, _) = thing("urn:thing:prop-emit", "Prop Emit Lamp");
    let servient = Servient::new();
    let handle = servient.expose(td).unwrap();

    let sink = RecordingPublisherSink::new();
    servient.event_broker().register(
        ThingId::from("urn:thing:prop-emit"),
        EventName::from("status"),
        sink.clone(),
    );

    handle
        .emit_property_change("status", Payload::new(b"42".to_vec(), "text/plain"))
        .unwrap();

    assert_eq!(sink.bodies(), vec![b"42".to_vec()]);
}

#[test]
fn destroy_removes_publisher_sinks() {
    let (td, _) = thing("urn:thing:event-destroy", "Event Destroy Lamp");
    let servient = Servient::new();
    let handle = servient.expose(td).unwrap();

    let sink = RecordingPublisherSink::new();
    servient.event_broker().register(
        ThingId::from("urn:thing:event-destroy"),
        EventName::from("startup"),
        sink.clone(),
    );

    handle
        .emit_event("startup", Payload::new(b"before".to_vec(), "text/plain"))
        .unwrap();
    assert_eq!(sink.bodies(), vec![b"before".to_vec()]);

    // Destroy removes all sinks for this Thing.
    servient.destroy("urn:thing:event-destroy").unwrap();

    // Emitting after destroy is a no-op (handle still usable but no sinks).
    // The handle's event_broker is a clone that shares state, so the removal
    // is visible.
    handle
        .emit_event("startup", Payload::new(b"after".to_vec(), "text/plain"))
        .unwrap();
    assert_eq!(sink.bodies(), vec![b"before".to_vec()]); // unchanged
}

#[test]
fn dispatcher_routes_subscribe_event_through_broker() {
    let (td, _) = thing("urn:thing:event-dispatch", "Event Dispatch Lamp");
    let servient = Servient::new();
    let handle = servient.expose(td).unwrap();
    handle
        .set_event_subscribe_handler("startup", StartupEvent)
        .unwrap();

    // Register a publisher sink to verify broker fan-out during dispatch.
    let sink = RecordingPublisherSink::new();
    servient.event_broker().register(
        ThingId::from("urn:thing:event-dispatch"),
        EventName::from("startup"),
        sink.clone(),
    );

    // Enqueue a SubscribeEvent inbound request via the FakeServerBinding.
    let server_binding = Arc::new(FakeServerBinding::default());
    servient
        .register_server_binding(server_binding.clone())
        .unwrap();

    server_binding.enqueue(InboundRequest::new(
        ThingId::from("urn:thing:event-dispatch"),
        clinkz_wot_core::AffordanceTarget::Event("startup".into()),
        Operation::SubscribeEvent,
        InteractionInput::empty(),
    ));

    servient.poll_serve_sync().unwrap();

    // The StartupEvent handler emits "ready" through the BrokerEventSink,
    // which fans out through the broker to our recording sink.
    assert_eq!(sink.bodies(), vec![b"ready".to_vec()]);

    let responses = server_binding.take_responses();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].error.is_none());
}

#[test]
fn dispatcher_routes_observe_property_through_broker() {
    let (td, _) = thing("urn:thing:observe-dispatch", "Observe Dispatch Lamp");
    let servient = Servient::new();
    let handle = servient.expose(td).unwrap();
    handle
        .set_property_read_handler(
            "status",
            StatusRead {
                value: Payload::new(b"on".to_vec(), "text/plain"),
            },
        )
        .unwrap();

    // Register a publisher sink on the property name for observe fan-out.
    let sink = RecordingPublisherSink::new();
    servient.event_broker().register(
        ThingId::from("urn:thing:observe-dispatch"),
        EventName::from("status"),
        sink.clone(),
    );

    let server_binding = Arc::new(FakeServerBinding::default());
    servient
        .register_server_binding(server_binding.clone())
        .unwrap();

    server_binding.enqueue(InboundRequest::new(
        ThingId::from("urn:thing:observe-dispatch"),
        clinkz_wot_core::AffordanceTarget::Property("status".into()),
        Operation::ObserveProperty,
        InteractionInput::empty(),
    ));

    servient.poll_serve_sync().unwrap();

    // ObserveProperty reads the current value and emits through broker.
    assert_eq!(sink.bodies(), vec![b"on".to_vec()]);

    let responses = server_binding.take_responses();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].error.is_none());
    // The response also carries the read value.
    assert_eq!(responses[0].output.payload.as_ref().unwrap().body.as_ref(), b"on");
}

#[test]
fn dispatcher_acknowledges_unsubscribe_and_unobserve() {
    let (td, _) = thing("urn:thing:unsub-dispatch", "Unsub Dispatch Lamp");
    let servient = Servient::new();
    servient.expose(td).unwrap();

    let server_binding = Arc::new(FakeServerBinding::default());
    servient
        .register_server_binding(server_binding.clone())
        .unwrap();

    // UnsubscribeEvent — ack only.
    server_binding.enqueue(InboundRequest::new(
        ThingId::from("urn:thing:unsub-dispatch"),
        clinkz_wot_core::AffordanceTarget::Event("startup".into()),
        Operation::UnsubscribeEvent,
        InteractionInput::empty(),
    ));

    // UnobserveProperty — ack only.
    server_binding.enqueue(InboundRequest::new(
        ThingId::from("urn:thing:unsub-dispatch"),
        clinkz_wot_core::AffordanceTarget::Property("status".into()),
        Operation::UnobserveProperty,
        InteractionInput::empty(),
    ));

    servient.poll_serve_sync().unwrap();

    let responses = server_binding.take_responses();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].error.is_none());

    servient.poll_serve_sync().unwrap();

    let responses = server_binding.take_responses();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].error.is_none());
}

#[test]
fn poll_serve_sync_round_robins_across_server_bindings() {
    let (td, _) = thing("urn:thing:sync-round-robin", "Sync Round Robin Lamp");
    let servient = Servient::new();
    let handle = servient.expose(td).unwrap();
    handle
        .set_property_read_handler(
            "status",
            StatusRead {
                value: Payload::new(b"on".to_vec(), "text/plain"),
            },
        )
        .unwrap();

    let first_binding = Arc::new(FakeServerBinding::default());
    let second_binding = Arc::new(FakeServerBinding::default());
    servient
        .register_server_binding(first_binding.clone())
        .unwrap();
    servient
        .register_server_binding(second_binding.clone())
        .unwrap();

    let mut first_request_a = InboundRequest::new(
        ThingId::from("urn:thing:sync-round-robin"),
        clinkz_wot_core::AffordanceTarget::Property("status".into()),
        Operation::ReadProperty,
        InteractionInput::empty(),
    );
    first_request_a.correlation = clinkz_wot_core::CorrelationId::from(1u64);
    let mut first_request_b = InboundRequest::new(
        ThingId::from("urn:thing:sync-round-robin"),
        clinkz_wot_core::AffordanceTarget::Property("status".into()),
        Operation::ReadProperty,
        InteractionInput::empty(),
    );
    first_request_b.correlation = clinkz_wot_core::CorrelationId::from(2u64);
    let mut second_request = InboundRequest::new(
        ThingId::from("urn:thing:sync-round-robin"),
        clinkz_wot_core::AffordanceTarget::Property("status".into()),
        Operation::ReadProperty,
        InteractionInput::empty(),
    );
    second_request.correlation = clinkz_wot_core::CorrelationId::from(3u64);

    first_binding.enqueue(first_request_a);
    first_binding.enqueue(first_request_b);
    second_binding.enqueue(second_request);

    servient.poll_serve_sync().unwrap();
    let first_responses = first_binding.take_responses();
    assert_eq!(first_responses.len(), 1);
    assert_eq!(
        first_responses[0].correlation,
        clinkz_wot_core::CorrelationId::from(1u64)
    );
    assert!(second_binding.take_responses().is_empty());

    servient.poll_serve_sync().unwrap();
    let second_responses = second_binding.take_responses();
    assert_eq!(second_responses.len(), 1);
    assert_eq!(
        second_responses[0].correlation,
        clinkz_wot_core::CorrelationId::from(3u64)
    );
    assert!(first_binding.take_responses().is_empty());

    servient.poll_serve_sync().unwrap();
    let first_responses = first_binding.take_responses();
    assert_eq!(first_responses.len(), 1);
    assert_eq!(
        first_responses[0].correlation,
        clinkz_wot_core::CorrelationId::from(2u64)
    );
}

// ---------------------------------------------------------------------------
// T3: Principal threading tests.
// ---------------------------------------------------------------------------

/// Inbound security provider that authenticates a known bearer token and
/// grants the "read" scope, used to verify principal threading.
struct InboundBearerProvider {
    valid_token: Vec<u8>,
}

impl clinkz_wot_core::SecurityProvider for InboundBearerProvider {
    fn scheme_name(&self) -> &str {
        "token"
    }

    fn apply(
        &self,
        _: clinkz_wot_core::SecurityContext<'_>,
        _: &mut clinkz_wot_core::TransportRequest,
    ) -> clinkz_wot_core::CoreResult<()> {
        Ok(())
    }

    fn verify(
        &self,
        request: &clinkz_wot_core::InboundRequest,
        _scheme: &clinkz_wot_td::security_scheme::SecurityScheme,
    ) -> Result<clinkz_wot_core::Principal, clinkz_wot_core::SecurityError> {
        match &request.auth {
            Some(clinkz_wot_core::AuthMaterial::BearerToken(t)) if t == &self.valid_token => {
                Ok(clinkz_wot_core::Principal {
                    id: clinkz_wot_core::PrincipalId::from("verified-caller"),
                    scopes: vec!["read".into()],
                })
            }
            _ => Err(clinkz_wot_core::SecurityError::InvalidCredentials),
        }
    }

    fn supports_scopes(&self, scopes: &[String]) -> bool {
        scopes.iter().all(|s| s == "read")
    }
}

#[test]
fn handler_receives_verified_principal_from_inbound_dispatch() {
    let (td, _) = secure_thing("urn:thing:principal-test", "Principal Test Lamp");
    let servient = Servient::builder()
        .security_provider(InboundBearerProvider {
            valid_token: b"secret-token".to_vec(),
        })
        .build();

    let handle = servient.expose(td).unwrap();
    let captured: Rc<std::cell::RefCell<Option<clinkz_wot_core::Principal>>> =
        Rc::new(std::cell::RefCell::new(None));
    handle
        .set_property_read_handler(
            "status",
            PrincipalCapturingProperty {
                captured_principal: captured.clone(),
            },
        )
        .unwrap();

    let server_binding = Arc::new(FakeServerBinding::default());
    servient
        .register_server_binding(server_binding.clone())
        .unwrap();

    // Enqueue a read request with valid bearer token auth.
    let mut request = InboundRequest::new(
        clinkz_wot_core::ThingId::from("urn:thing:principal-test"),
        clinkz_wot_core::AffordanceTarget::Property("status".into()),
        Operation::ReadProperty,
        InteractionInput::empty(),
    );
    request.auth = Some(clinkz_wot_core::AuthMaterial::BearerToken(
        b"secret-token".to_vec(),
    ));
    server_binding.enqueue(request);

    servient.poll_serve_sync().unwrap();

    // Handler should have received the verified principal.
    let captured = captured.borrow();
    assert!(captured.is_some(), "handler should receive a principal");
    assert_eq!(captured.as_ref().unwrap().id.as_str(), "verified-caller");
    assert_eq!(captured.as_ref().unwrap().scopes, vec!["read".to_string()]);

    let responses = server_binding.take_responses();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].error.is_none());
}

#[test]
fn handler_receives_anonymous_principal_for_nosec() {
    let (td, _) = thing("urn:thing:nosec-principal", "NoSec Principal Lamp");
    let servient = Servient::new();
    let handle = servient.expose(td).unwrap();

    let captured: Rc<std::cell::RefCell<Option<clinkz_wot_core::Principal>>> =
        Rc::new(std::cell::RefCell::new(None));
    handle
        .set_property_read_handler(
            "status",
            PrincipalCapturingProperty {
                captured_principal: captured.clone(),
            },
        )
        .unwrap();

    let server_binding = Arc::new(FakeServerBinding::default());
    servient
        .register_server_binding(server_binding.clone())
        .unwrap();

    server_binding.enqueue(InboundRequest::new(
        clinkz_wot_core::ThingId::from("urn:thing:nosec-principal"),
        clinkz_wot_core::AffordanceTarget::Property("status".into()),
        Operation::ReadProperty,
        InteractionInput::empty(),
    ));

    servient.poll_serve_sync().unwrap();

    // NoSec → anonymous principal.
    let captured = captured.borrow();
    assert!(captured.is_some(), "handler should receive a principal");
    assert_eq!(captured.as_ref().unwrap().id.as_str(), "anonymous");
    assert!(captured.as_ref().unwrap().scopes.is_empty());
}

// ---------------------------------------------------------------------------
// T2: Consumer streaming subscription tests.
// ---------------------------------------------------------------------------

#[test]
fn subscribe_event_returns_streaming_subscription() {
    let (td, _) = thing("urn:thing:stream-sub", "Stream Sub Lamp");
    let servient = Servient::builder()
        .binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"on".to_vec(), "text/plain"),
            })
        })
        .build();

    let consumed = servient.consume(td).unwrap();

    let subscription = consumed
        .subscribe_event("startup", InteractionInput::empty())
        .unwrap();

    // The TestBinding::subscribe pushes an initial sample.
    let payload = subscription.poll_next().expect("should have a sample");
    assert_eq!(payload.body.as_ref(), b"subscribed");

    // Queue is now empty.
    assert!(subscription.poll_next().is_none());
}

#[test]
fn observe_property_returns_streaming_subscription() {
    let (td, _) = thing("urn:thing:stream-obs", "Stream Observe Lamp");
    let servient = Servient::builder()
        .binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"observed-value".to_vec(), "text/plain"),
            })
        })
        .build();

    let consumed = servient.consume(td).unwrap();

    let subscription = consumed
        .observe_property("status", InteractionInput::empty())
        .unwrap();

    let payload = subscription.poll_next().expect("should have a sample");
    assert_eq!(payload.body.as_ref(), b"observed-value");
}

#[test]
fn unsubscribe_event_stops_wire_subscription() {
    let (td, _) = thing("urn:thing:unsub", "Unsub Lamp");
    let servient = Servient::builder()
        .binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"on".to_vec(), "text/plain"),
            })
        })
        .build();

    let consumed = servient.consume(td).unwrap();

    let _subscription = consumed
        .subscribe_event("startup", InteractionInput::empty())
        .unwrap();

    // unsubscribe_event should clean up wire resources without panic.
    consumed.unsubscribe_event("startup");
}

#[test]
fn unobserve_property_stops_wire_subscription() {
    let (td, _) = thing("urn:thing:unobs", "Unobs Lamp");
    let servient = Servient::builder()
        .binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"on".to_vec(), "text/plain"),
            })
        })
        .build();

    let consumed = servient.consume(td).unwrap();

    let _subscription = consumed
        .observe_property("status", InteractionInput::empty())
        .unwrap();

    // unobserve_property should clean up wire resources without panic.
    consumed.unobserve_property("status");
}

#[test]
fn subscription_supports_poll_next_and_stop() {
    let (td, _) = thing("urn:thing:sub-poll", "Sub Poll Lamp");
    let servient = Servient::builder()
        .binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"data".to_vec(), "text/plain"),
            })
        })
        .build();

    let consumed = servient.consume(td).unwrap();

    let subscription = consumed
        .subscribe_event("startup", InteractionInput::empty())
        .unwrap();

    // Drain initial sample.
    assert!(subscription.poll_next().is_some());
    assert!(subscription.is_empty());

    // Stop the consumer-side queue.
    subscription.stop();
    assert!(subscription.is_stopped());
}

// ---------------------------------------------------------------------------
// C5: Split handler trait tests.
// ---------------------------------------------------------------------------

#[test]
fn read_only_property_works_without_write_handler() {
    let (td, _) = thing("urn:thing:read-only", "Read Only Lamp");
    let servient = Servient::new();
    let handle = servient.expose(td).unwrap();

    // Only register a read handler — no write handler needed.
    handle
        .set_property_read_handler(
            "status",
            StatusRead {
                value: Payload::new(b"read-only-value".to_vec(), "text/plain"),
            },
        )
        .unwrap();

    let payload = handle
        .read_property("status", InteractionInput::empty())
        .unwrap()
        .payload
        .unwrap();
    assert_eq!(payload.body.as_ref(), b"read-only-value");

    // Write must fail with MissingHandler.
    let err = handle
        .write_property(
            "status",
            InteractionInput::with_payload(Payload::new(b"ignored".to_vec(), "text/plain")),
        )
        .unwrap_err();
    assert!(
        matches!(
            err,
            crate::ServientError::Serve(clinkz_wot_core::CoreError::MissingHandler { .. })
        ),
        "write to read-only property must fail with MissingHandler"
    );
}

#[test]
fn observe_handler_invoked_on_observe_dispatch() {
    let (td, _) = thing("urn:thing:observe-handler", "Observe Handler Lamp");
    let servient = Servient::new();
    let handle = servient.expose(td).unwrap();

    let sink = RecordingPublisherSink::new();
    servient.event_broker().register(
        ThingId::from("urn:thing:observe-handler"),
        EventName::from("status"),
        sink.clone(),
    );

    handle
        .set_property_observe_handler(
            "status",
            ObserveInitial {
                initial: Payload::new(b"initial-observe".to_vec(), "text/plain"),
            },
        )
        .unwrap();

    let server_binding = Arc::new(FakeServerBinding::default());
    servient
        .register_server_binding(server_binding.clone())
        .unwrap();

    server_binding.enqueue(InboundRequest::new(
        ThingId::from("urn:thing:observe-handler"),
        clinkz_wot_core::AffordanceTarget::Property("status".into()),
        Operation::ObserveProperty,
        InteractionInput::empty(),
    ));

    servient.poll_serve_sync().unwrap();

    // The ObserveInitial handler emits through the broker sink.
    assert_eq!(sink.bodies(), vec![b"initial-observe".to_vec()]);
}

#[test]
fn unsubscribe_handler_invoked_on_unsubscribe_dispatch() {
    let (td, _) = thing("urn:thing:unsub-handler", "Unsub Handler Lamp");
    let servient = Servient::new();

    let called = Rc::new(Cell::new(false));
    let handle = servient.expose(td).unwrap();
    handle
        .set_event_unsubscribe_handler(
            "startup",
            support::RecordingUnsubscribe {
                called: Rc::clone(&called),
            },
        )
        .unwrap();

    let server_binding = Arc::new(FakeServerBinding::default());
    servient
        .register_server_binding(server_binding.clone())
        .unwrap();

    server_binding.enqueue(InboundRequest::new(
        ThingId::from("urn:thing:unsub-handler"),
        clinkz_wot_core::AffordanceTarget::Event("startup".into()),
        Operation::UnsubscribeEvent,
        InteractionInput::empty(),
    ));

    servient.poll_serve_sync().unwrap();
    assert!(called.get(), "unsubscribe handler must be called");
}

// ---------------------------------------------------------------------------
// C6: Bulk property operation tests.
// ---------------------------------------------------------------------------

#[test]
fn read_multiple_properties_returns_all_values() {
    let td = {
        let read_temperature = Form::read_property("test://things/sensor/properties/temperature")
            .content_type("text/plain")
            .build()
            .unwrap();
        let read_humidity = Form::read_property("test://things/sensor/properties/humidity")
            .content_type("text/plain")
            .build()
            .unwrap();
        let temp = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
            .form(read_temperature)
            .build()
            .unwrap();
        let humid = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
            .form(read_humidity)
            .build()
            .unwrap();
        Thing::builder("Multi-Sensor")
            .id("urn:thing:multi-read")
            .nosec()
            .property("temperature", temp)
            .property("humidity", humid)
            .build()
            .unwrap()
    };
    let servient = Servient::builder()
        .binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"42".to_vec(), "text/plain"),
            })
        })
        .build();
    let consumed = servient.consume(td).unwrap();

    let results = consumed
        .read_multiple_properties(&["temperature", "humidity"])
        .unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results["temperature"].payload.as_ref().unwrap().body.as_ref(), b"42");
    assert_eq!(results["humidity"].payload.as_ref().unwrap().body.as_ref(), b"42");
}

#[test]
fn read_all_properties_returns_every_td_property() {
    let td = {
        let form1 = Form::read_property("test://things/x/properties/a")
            .content_type("text/plain")
            .build()
            .unwrap();
        let form2 = Form::read_property("test://things/x/properties/b")
            .content_type("text/plain")
            .build()
            .unwrap();
        let prop_a = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
            .form(form1)
            .build()
            .unwrap();
        let prop_b = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
            .form(form2)
            .build()
            .unwrap();
        Thing::builder("X")
            .id("urn:thing:read-all")
            .nosec()
            .property("a", prop_a)
            .property("b", prop_b)
            .build()
            .unwrap()
    };
    let servient = Servient::builder()
        .binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"ok".to_vec(), "text/plain"),
            })
        })
        .build();
    let consumed = servient.consume(td).unwrap();

    let results = consumed.read_all_properties().unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.contains_key("a"));
    assert!(results.contains_key("b"));
}

// ---------------------------------------------------------------------------
// Bulk property operations via Thing-level forms (W3C TD §6.3.3).
//
// When the consumed TD declares a Thing-level `readallproperties` /
// `writemultipleproperties` form, the consumer should route through it in a
// single round trip rather than fanning out across per-property forms.
// ---------------------------------------------------------------------------

#[test]
fn read_all_properties_uses_thing_level_form_when_available() {
    // A binding that answers Thing-level bulk reads with a combined JSON
    // object and panics if a per-property read reaches it — proving the
    // consumer took the bulk path instead of fanning out.
    struct BulkBinding {
        seen_bulk: Arc<Mutex<bool>>,
    }
    impl clinkz_wot_core::ClientBinding for BulkBinding {
        fn supports(&self, form: &Form, operation: Operation) -> bool {
            (form.href.as_str().starts_with("test://"))
                && matches!(
                    operation,
                    Operation::ReadAllProperties | Operation::ReadMultipleProperties
                )
        }
        fn invoke(
            &self,
            request: clinkz_wot_core::BindingRequest,
        ) -> Result<clinkz_wot_core::InteractionOutput, clinkz_wot_core::CoreError> {
            match (request.target.clone(), request.operation) {
                (clinkz_wot_core::AffordanceTarget::Thing, Operation::ReadAllProperties) => {
                    *self.seen_bulk.lock().unwrap() = true;
                    let body = br#"{"a":1,"b":2}"#.to_vec();
                    Ok(clinkz_wot_core::InteractionOutput::with_payload(
                        Payload::new(body, "application/json"),
                    ))
                }
                _ => panic!("expected a single Thing-level bulk read, got fan-out"),
            }
        }
    }

    let seen_bulk = Arc::new(Mutex::new(false));
    let td = {
        let prop_a = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
            .form(
                Form::read_property("test://things/x/properties/a")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let prop_b = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
            .form(
                Form::read_property("test://things/x/properties/b")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        Thing::builder("BulkRead")
            .id("urn:thing:bulk-read")
            .nosec()
            .property("a", prop_a)
            .property("b", prop_b)
            .form(
                Form::builder("test://things/x/properties")
                    .op([Operation::ReadAllProperties])
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
    };
    let servient = Servient::builder()
        .binding_factory({
            let seen = Arc::clone(&seen_bulk);
            move || {
                Box::new(BulkBinding {
                    seen_bulk: Arc::clone(&seen),
                })
            }
        })
        .build();
    let consumed = servient.consume(td).unwrap();

    let results = consumed.read_all_properties().unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results["a"].payload.as_ref().unwrap().body.as_ref(), b"1");
    assert_eq!(results["b"].payload.as_ref().unwrap().body.as_ref(), b"2");
    assert!(
        *seen_bulk.lock().unwrap(),
        "the Thing-level readallproperties form must be used"
    );
}

#[test]
fn write_multiple_properties_uses_thing_level_form_when_available() {
    // Records the body of the single bulk write request so the test can prove
    // the consumer sent one combined `{name: value}` payload instead of N
    // per-property writes.
    let recorded_body = Arc::new(Mutex::new(Vec::<u8>::new()));
    struct BulkWriteBinding {
        recorded_body: Arc<Mutex<Vec<u8>>>,
    }
    impl clinkz_wot_core::ClientBinding for BulkWriteBinding {
        fn supports(&self, form: &Form, operation: Operation) -> bool {
            form.href.as_str().starts_with("test://")
                && operation == Operation::WriteMultipleProperties
        }
        fn invoke(
            &self,
            request: clinkz_wot_core::BindingRequest,
        ) -> Result<clinkz_wot_core::InteractionOutput, clinkz_wot_core::CoreError> {
            match (request.target.clone(), request.operation) {
                (clinkz_wot_core::AffordanceTarget::Thing, Operation::WriteMultipleProperties) => {
                    if let Some(payload) = request.input.payload {
                        *self.recorded_body.lock().unwrap() = payload.body.as_ref().to_vec();
                    }
                    Ok(clinkz_wot_core::InteractionOutput::empty())
                }
                _ => panic!("expected a single Thing-level bulk write, got fan-out"),
            }
        }
    }

    let td = {
        Thing::builder("BulkWrite")
            .id("urn:thing:bulk-write")
            .nosec()
            .form(
                Form::builder("test://things/x/properties")
                    .op([Operation::WriteMultipleProperties])
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
    };
    let servient = Servient::builder()
        .binding_factory({
            let recorded = Arc::clone(&recorded_body);
            move || {
                Box::new(BulkWriteBinding {
                    recorded_body: Arc::clone(&recorded),
                })
            }
        })
        .build();
    let consumed = servient.consume(td).unwrap();

    let mut values = BTreeMap::new();
    values.insert(
        String::from("a"),
        InteractionInput::with_payload(Payload::new(b"1".to_vec(), "application/json")),
    );
    values.insert(
        String::from("b"),
        InteractionInput::with_payload(Payload::new(b"2".to_vec(), "application/json")),
    );
    consumed.write_multiple_properties(&values).unwrap();

    let body = recorded_body.lock().unwrap().clone();
    let parsed: serde_json::Value =
        serde_json::from_slice(&body).expect("bulk write payload must be a JSON object");
    let map = parsed.as_object().expect("bulk payload is an object");
    assert_eq!(map.get("a"), Some(&serde_json::json!(1)));
    assert_eq!(map.get("b"), Some(&serde_json::json!(2)));
}

// ---------------------------------------------------------------------------
// M7: Credential store tests.
// ---------------------------------------------------------------------------

#[test]
fn in_memory_credential_store_stores_and_retrieves() {
    use clinkz_wot_core::{CredentialStore, Credentials, InMemoryCredentialStore};

    let store = InMemoryCredentialStore::new();
    store
        .put(
            "urn:thing:1",
            "bearer",
            Credentials::BearerToken(b"tok123".to_vec()),
        )
        .unwrap();

    let creds = store
        .get("urn:thing:1", "bearer")
        .expect("credentials stored");
    assert_eq!(creds, Credentials::BearerToken(b"tok123".to_vec()));

    assert!(store.get("urn:thing:1", "unknown").is_none());
    assert!(store.get("urn:thing:other", "bearer").is_none());
}

#[test]
fn credential_store_remove_clears_entry() {
    use clinkz_wot_core::{CredentialStore, Credentials, InMemoryCredentialStore};

    let store = InMemoryCredentialStore::new();
    store
        .put(
            "urn:thing:1",
            "apikey",
            Credentials::ApiKey("key123".into()),
        )
        .unwrap();

    store.remove("urn:thing:1", "apikey").unwrap();
    assert!(store.get("urn:thing:1", "apikey").is_none());
}

#[test]
fn write_multiple_properties_dispatches_each() {
    let td = {
        let form1 = Form::write_property("test://things/x/properties/a")
            .content_type("text/plain")
            .build()
            .unwrap();
        let form2 = Form::write_property("test://things/x/properties/b")
            .content_type("text/plain")
            .build()
            .unwrap();
        let prop_a = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
            .form(form1)
            .build()
            .unwrap();
        let prop_b = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
            .form(form2)
            .build()
            .unwrap();
        Thing::builder("X")
            .id("urn:thing:multi-write")
            .nosec()
            .property("a", prop_a)
            .property("b", prop_b)
            .build()
            .unwrap()
    };
    let servient = Servient::builder()
        .binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"".to_vec(), "text/plain"),
            })
        })
        .build();
    let consumed = servient.consume(td).unwrap();

    let mut values = BTreeMap::new();
    values.insert(
        "a".to_string(),
        InteractionInput::with_payload(Payload::new(b"1".to_vec(), "text/plain")),
    );
    values.insert(
        "b".to_string(),
        InteractionInput::with_payload(Payload::new(b"2".to_vec(), "text/plain")),
    );
    consumed.write_multiple_properties(&values).unwrap();
}

// ---------------------------------------------------------------------------
// C7: Discovery API tests.
// ---------------------------------------------------------------------------

#[test]
fn discover_local_returns_all_directory_entries() {
    let servient = Servient::new();
    servient
        .register(
            Thing::builder("Lamp A")
                .id("urn:thing:disc-a")
                .nosec()
                .build()
                .unwrap(),
        )
        .unwrap();
    servient
        .register(
            Thing::builder("Lamp B")
                .id("urn:thing:disc-b")
                .nosec()
                .build()
                .unwrap(),
        )
        .unwrap();

    let mut discovery = servient.discover(ThingFilter::new()).unwrap();

    assert!(!discovery.is_done());
    let t1 = discovery.next_now().expect("first result");
    assert_eq!(t1._metadata.title.as_deref(), Some("Lamp A"));
    let t2 = discovery.next_now().expect("second result");
    assert_eq!(t2._metadata.title.as_deref(), Some("Lamp B"));
    assert!(discovery.next_now().is_none());
    assert!(discovery.is_done());
}

#[test]
fn discover_with_fragment_filter_narrows_results() {
    let servient = Servient::new();
    servient
        .register(
            Thing::builder("Sensor")
                .id("urn:thing:disc-sensor")
                .nosec()
                .build()
                .unwrap(),
        )
        .unwrap();
    servient
        .register(
            Thing::builder("Lamp")
                .id("urn:thing:disc-lamp")
                .nosec()
                .build()
                .unwrap(),
        )
        .unwrap();

    let mut discovery = servient
        .discover(ThingFilter::new().fragment_field("title", serde_json::json!("Sensor")))
        .unwrap();

    let t = discovery.next_now().expect("one result");
    assert_eq!(t._metadata.title.as_deref(), Some("Sensor"));
    assert!(discovery.next_now().is_none());
}

#[test]
fn discover_stop_discards_remaining_results() {
    let servient = Servient::new();
    servient
        .register(
            Thing::builder("A")
                .id("urn:thing:disc-stop-a")
                .nosec()
                .build()
                .unwrap(),
        )
        .unwrap();
    servient
        .register(
            Thing::builder("B")
                .id("urn:thing:disc-stop-b")
                .nosec()
                .build()
                .unwrap(),
        )
        .unwrap();

    let mut discovery = servient.discover(ThingFilter::new()).unwrap();
    assert_eq!(discovery.remaining(), 2);

    discovery.stop();
    assert!(discovery.is_done());
    assert!(discovery.next_now().is_none());
    assert_eq!(discovery.remaining(), 0);
}

#[test]
fn discover_empty_filter_works_like_local() {
    let servient = Servient::new();
    servient
        .register(
            Thing::builder("Thing")
                .id("urn:thing:disc-everything")
                .nosec()
                .build()
                .unwrap(),
        )
        .unwrap();

    let mut discovery = servient.discover(ThingFilter::new()).unwrap();

    assert!(discovery.next_now().is_some());
    assert!(discovery.next_now().is_none());
}

// ---------------------------------------------------------------------------
// M12: Graceful shutdown tests.
// ---------------------------------------------------------------------------

#[test]
fn shutdown_handle_signals_shutdown() {
    let servient = Servient::new();
    let handle = servient.shutdown_handle();
    assert!(!handle.is_shutdown());
    handle.shutdown();
    assert!(handle.is_shutdown());
}

#[test]
fn shutdown_handle_is_cloneable() {
    let servient = Servient::new();
    let handle = servient.shutdown_handle();
    let handle2 = handle.clone();
    handle.shutdown();
    assert!(handle2.is_shutdown());
}

#[test]
fn poll_serve_sync_returns_after_shutdown() {
    let (td, _) = thing("urn:thing:shutdown-test", "Shutdown Lamp");
    let servient = Servient::new();
    servient.expose(td).unwrap();

    let handle = servient.shutdown_handle();
    handle.shutdown();

    // With shutdown set, poll_serve_sync should return immediately (Ok).
    servient.poll_serve_sync().unwrap();
}

// ---------------------------------------------------------------------------
// URI template expansion (Finding #3 from WoT compliance audit).
//
// Verifies that consumed interactions expand RFC 6570 URI templates in form
// hrefs using the caller-supplied uriVariables (InteractionInput.parameters)
// before handing the form to the binding.
// ---------------------------------------------------------------------------

#[test]
fn consumed_request_expands_uri_template_form_href() {
    // Build a Thing whose property form uses a URI template.
    let template_form = Form::builder("test://things/{thing_id}/properties/{prop}")
        .read_property()
        .content_type("text/plain")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(template_form)
        .build()
        .unwrap();
    let td = Thing::builder("Template Thing")
        .id("urn:thing:template-thing")
        .nosec()
        .property("temperature", property)
        .build()
        .unwrap();

    let servient = Servient::builder()
        .binding_factory(|| {
            Box::new(CountingHrefBinding {
                supports_calls: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            })
        })
        .build();

    let consumed = servient.consume(td).unwrap();

    // Supply uriVariables through InteractionInput.parameters.
    let mut input = InteractionInput::empty();
    input
        .parameters
        .insert("thing_id".to_string(), "gw001".to_string());
    input
        .parameters
        .insert("prop".to_string(), "temperature".to_string());

    let output = consumed.read_property("temperature", input).unwrap();

    // CountingHrefBinding returns the href it received; verify it was expanded.
    let payload = output.payload.unwrap();
    let received_href = std::str::from_utf8(&payload.body).unwrap();
    assert_eq!(
        received_href, "test://things/gw001/properties/temperature",
        "binding should receive the expanded URI, not the template"
    );
}

#[test]
fn consumed_request_passes_non_template_form_unchanged() {
    // Build a Thing with only concrete test:// forms (no other:// variants).
    let read_form = Form::read_property("test://things/lamp/properties/status")
        .content_type("text/plain")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(read_form)
        .build()
        .unwrap();
    let td = Thing::builder("Concrete Href Thing")
        .id("urn:thing:concrete-href")
        .nosec()
        .property("status", property)
        .build()
        .unwrap();

    let servient = Servient::builder()
        .binding_factory(|| {
            Box::new(CountingHrefBinding {
                supports_calls: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            })
        })
        .build();

    let consumed = servient.consume(td).unwrap();
    let output = consumed
        .read_property("status", InteractionInput::empty())
        .unwrap();

    let payload = output.payload.unwrap();
    let received_href = std::str::from_utf8(&payload.body).unwrap();
    assert_eq!(
        received_href, "test://things/lamp/properties/status",
        "non-template href should pass through unchanged"
    );
}

// ---------------------------------------------------------------------------
// W3C WoT Scripting API compliance: new handler traits and meta-operations.
// ---------------------------------------------------------------------------

/// Records whether the unobserve handler was called.
struct RecordingUnobserveHandler {
    called: Arc<std::sync::atomic::AtomicBool>,
}

impl clinkz_wot_core::PropertyUnobserveHandler for RecordingUnobserveHandler {
    fn unobserve(
        &self,
        _input: InteractionInput,
    ) -> clinkz_wot_core::CoreResult<clinkz_wot_core::InteractionOutput> {
        self.called
            .store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(clinkz_wot_core::InteractionOutput::empty())
    }
}

#[test]
fn dispatcher_routes_unobserve_property_through_handler() {
    let (td, _) = thing("urn:thing:unobserve-handler", "Unobserve Handler Lamp");
    let servient = Servient::new();
    let handle = servient.expose(td).unwrap();

    let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    handle
        .set_property_unobserve_handler(
            "status",
            RecordingUnobserveHandler {
                called: Arc::clone(&called),
            },
        )
        .unwrap();

    let server_binding = Arc::new(FakeServerBinding::default());
    servient
        .register_server_binding(server_binding.clone())
        .unwrap();

    server_binding.enqueue(InboundRequest::new(
        ThingId::from("urn:thing:unobserve-handler"),
        clinkz_wot_core::AffordanceTarget::Property("status".into()),
        Operation::UnobserveProperty,
        InteractionInput::empty(),
    ));

    servient.poll_serve_sync().unwrap();

    assert!(
        called.load(std::sync::atomic::Ordering::Relaxed),
        "unobserve handler should have been called"
    );
    let responses = server_binding.take_responses();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].error.is_none());
}

#[test]
fn dispatcher_routes_queryaction_through_handler() {
    let (td, _) = thing("urn:thing:query-action", "Query Action Lamp");
    let servient = Servient::new();
    let handle = servient.expose(td).unwrap();

    // QueryAction without a handler returns MissingHandler.
    let server_binding = Arc::new(FakeServerBinding::default());
    servient
        .register_server_binding(server_binding.clone())
        .unwrap();

    server_binding.enqueue(InboundRequest::new(
        ThingId::from("urn:thing:query-action"),
        clinkz_wot_core::AffordanceTarget::Action("echo".into()),
        Operation::QueryAction,
        InteractionInput::empty(),
    ));

    servient.poll_serve_sync().unwrap();
    let responses = server_binding.take_responses();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].error.is_some());

    // Now register a query handler and re-test.
    struct EchoQuery;
    impl clinkz_wot_core::ActionQueryHandler for EchoQuery {
        fn query(
            &self,
            _input: InteractionInput,
        ) -> clinkz_wot_core::CoreResult<clinkz_wot_core::InteractionOutput> {
            Ok(clinkz_wot_core::InteractionOutput::with_payload(
                Payload::new(b"\"idle\"".to_vec(), "application/json"),
            ))
        }
    }
    handle.set_action_query_handler("echo", EchoQuery).unwrap();

    server_binding.enqueue(InboundRequest::new(
        ThingId::from("urn:thing:query-action"),
        clinkz_wot_core::AffordanceTarget::Action("echo".into()),
        Operation::QueryAction,
        InteractionInput::empty(),
    ));

    servient.poll_serve_sync().unwrap();
    let responses = server_binding.take_responses();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].error.is_none());
    assert_eq!(
        responses[0].output.payload.as_ref().unwrap().body.as_ref(),
        b"\"idle\""
    );
}

/// `cancelaction` is a TD 1.1 operation; this dispatch test verifies the
/// no-handler acknowledgement path.
#[test]
fn dispatcher_acknowledges_cancelaction_without_handler() {
    let (td, _) = thing("urn:thing:cancel-action", "Cancel Action Lamp");
    let servient = Servient::new();
    servient.expose(td).unwrap();

    let server_binding = Arc::new(FakeServerBinding::default());
    servient
        .register_server_binding(server_binding.clone())
        .unwrap();

    server_binding.enqueue(InboundRequest::new(
        ThingId::from("urn:thing:cancel-action"),
        clinkz_wot_core::AffordanceTarget::Action("echo".into()),
        Operation::CancelAction,
        InteractionInput::empty(),
    ));

    servient.poll_serve_sync().unwrap();
    let responses = server_binding.take_responses();
    assert_eq!(responses.len(), 1);
    assert!(
        responses[0].error.is_none(),
        "CancelAction without a handler should ack"
    );
}

#[test]
fn dispatcher_dispatches_queryallactions_as_combined_response() {
    let (td, _) = thing("urn:thing:query-all-actions", "Query All Actions Lamp");
    let servient = Servient::new();
    let handle = servient.expose(td).unwrap();

    struct EchoQuery;
    impl clinkz_wot_core::ActionQueryHandler for EchoQuery {
        fn query(
            &self,
            _input: InteractionInput,
        ) -> clinkz_wot_core::CoreResult<clinkz_wot_core::InteractionOutput> {
            Ok(clinkz_wot_core::InteractionOutput::with_payload(
                Payload::new(b"\"running\"".to_vec(), "application/json"),
            ))
        }
    }
    handle.set_action_query_handler("echo", EchoQuery).unwrap();

    let server_binding = Arc::new(FakeServerBinding::default());
    servient
        .register_server_binding(server_binding.clone())
        .unwrap();

    server_binding.enqueue(InboundRequest::new(
        ThingId::from("urn:thing:query-all-actions"),
        clinkz_wot_core::AffordanceTarget::Thing,
        Operation::QueryAllActions,
        InteractionInput::empty(),
    ));

    servient.poll_serve_sync().unwrap();
    let responses = server_binding.take_responses();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].error.is_none());
    let body = &responses[0].output.payload.as_ref().unwrap().body;
    let json: serde_json::Value = serde_json::from_slice(body).unwrap();
    assert_eq!(json["echo"], "running");
}

#[test]
fn dispatcher_dispatches_observeallproperties_through_broker() {
    // Build a Thing with an observable property.
    let observe_form = Form::builder("test://things/lamp/properties/status")
        .observe_property()
        .content_type("text/plain")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .observable(true)
        .form(observe_form)
        .build()
        .unwrap();
    let td = Thing::builder("Observe All Props Lamp")
        .id("urn:thing:observe-all-props")
        .nosec()
        .property("status", property)
        .build()
        .unwrap();

    let servient = Servient::new();
    let handle = servient.expose(td).unwrap();
    handle
        .set_property_observe_handler(
            "status",
            ObserveInitial {
                initial: Payload::new(b"on".to_vec(), "text/plain"),
            },
        )
        .unwrap();

    let sink = RecordingPublisherSink::new();
    servient.event_broker().register(
        ThingId::from("urn:thing:observe-all-props"),
        EventName::from("status"),
        sink.clone(),
    );

    let server_binding = Arc::new(FakeServerBinding::default());
    servient
        .register_server_binding(server_binding.clone())
        .unwrap();

    server_binding.enqueue(InboundRequest::new(
        ThingId::from("urn:thing:observe-all-props"),
        clinkz_wot_core::AffordanceTarget::Thing,
        Operation::ObserveAllProperties,
        InteractionInput::empty(),
    ));

    servient.poll_serve_sync().unwrap();
    assert_eq!(
        sink.bodies(),
        vec![b"on".to_vec()],
        "observeallproperties should fan out through the broker"
    );
}

#[test]
fn consumed_write_all_properties_dispatches_each() {
    let (td, _forms) = thing("urn:thing:write-all-props", "Write All Props Lamp");
    let servient = Servient::builder()
        .binding_factory(|| {
            Box::new(TestBinding {
                response: Payload::new(b"ok".to_vec(), "text/plain"),
            })
        })
        .build();
    let consumed = servient.consume(td).unwrap();

    let mut values = BTreeMap::new();
    values.insert(
        "status".to_string(),
        InteractionInput::with_payload(Payload::new(b"off".to_vec(), "text/plain")),
    );
    consumed.write_all_properties(&values).unwrap();
}
