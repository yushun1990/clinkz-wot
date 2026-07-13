#![cfg(feature = "zenoh")]

use std::env;
use std::sync::Arc;
use std::time::Duration;

use clinkz_wot_core::{
    AffordanceTarget, BindingContext, CoreError, ErrorContext, ErrorPhase, EventBroker, EventName,
    InboundDispatcher, InboundRequest, InboundResponse, InteractionInput, InteractionOutput,
    Payload, RetryClass, ServerBinding, ThingId,
};
use clinkz_wot_protocol_bindings_zenoh::ZenohServerBinding;
use clinkz_wot_td::data_type::Operation;
use clinkz_wot_td::thing::Thing;
use zenoh::Wait;

const RUN_ZENOH_RUNTIME_TESTS: &str = "CLINKZ_WOT_RUN_ZENOH_RUNTIME_TESTS";
const REPLY_TIMEOUT: Duration = Duration::from_secs(5);
const PROPAGATION_DELAY: Duration = Duration::from_millis(500);

#[test]
fn runtime_server_binding_read_property_round_trip() {
    if !should_run() {
        return;
    }

    zenoh::init_log_from_env_or("error");
    let server_binding = ZenohServerBinding::open(zenoh::Config::default()).unwrap();
    let session = server_binding.session().clone();

    let td = test_td("urn:server:read");
    let ctx = default_ctx();
    server_binding
        .serve(&ThingId::from("urn:server:read"), &td, &ctx)
        .expect("serve");

    std::thread::sleep(PROPAGATION_DELAY);

    // Simulate a consumer querying the read-property key.
    let key_expr = "things/urn:server:read/properties/status";
    let query_thread = std::thread::spawn(move || {
        let replies = session.get(key_expr).wait().expect("send query");
        replies
            .recv_timeout(REPLY_TIMEOUT)
            .expect("receive reply timeout")
            .expect("no reply")
            .into_result()
            .expect("reply error")
    });

    // Drain the inbound request from the server side.
    let request = poll_with_timeout(&server_binding, REPLY_TIMEOUT, "query");

    assert_eq!(request.thing_id.as_str(), "urn:server:read");
    assert_eq!(request.target, AffordanceTarget::Property("status".into()));
    assert_eq!(request.operation, Operation::ReadProperty);

    let response = InboundResponse::new(
        InteractionOutput::with_data(Payload::new(b"on".to_vec(), "text/plain")),
        request.correlation,
    );
    server_binding.send_response(response);

    let sample = query_thread.join().expect("join query thread");
    assert_eq!(sample.payload().to_bytes().as_ref(), b"on");
}

#[test]
fn runtime_server_binding_invoke_action_round_trip() {
    if !should_run() {
        return;
    }

    zenoh::init_log_from_env_or("error");
    let server_binding = ZenohServerBinding::open(zenoh::Config::default()).unwrap();
    let session = server_binding.session().clone();

    let td = test_td("urn:server:invoke");
    let ctx = default_ctx();
    server_binding
        .serve(&ThingId::from("urn:server:invoke"), &td, &ctx)
        .expect("serve");

    std::thread::sleep(PROPAGATION_DELAY);

    let key_expr = "things/urn:server:invoke/actions/echo";
    let request_body = b"hello".to_vec();
    let query_thread = std::thread::spawn(move || {
        let replies = session
            .get(key_expr)
            .payload(request_body.clone())
            .wait()
            .expect("send action query");
        replies
            .recv_timeout(REPLY_TIMEOUT)
            .expect("receive reply timeout")
            .expect("no reply")
            .into_result()
            .expect("reply error")
    });

    let request = poll_with_timeout(&server_binding, REPLY_TIMEOUT, "action query");
    assert_eq!(request.target, AffordanceTarget::Action("echo".into()));
    assert_eq!(request.operation, Operation::InvokeAction);
    let input_body = request.input.data.map(|p| p.body).unwrap_or_default();
    assert_eq!(input_body.as_ref(), b"hello");

    let response = InboundResponse::new(
        InteractionOutput::with_data(Payload::new(b"hello-echo".to_vec(), "text/plain")),
        request.correlation,
    );
    server_binding.send_response(response);

    let sample = query_thread.join().expect("join action thread");
    assert_eq!(sample.payload().to_bytes().as_ref(), b"hello-echo");
}

#[test]
fn runtime_server_binding_write_property_put_listener() {
    if !should_run() {
        return;
    }

    zenoh::init_log_from_env_or("error");
    let server_binding = ZenohServerBinding::open(zenoh::Config::default()).unwrap();
    let session = server_binding.session().clone();

    let td = test_td("urn:server:write");
    let ctx = default_ctx();
    server_binding
        .serve(&ThingId::from("urn:server:write"), &td, &ctx)
        .expect("serve");

    std::thread::sleep(PROPAGATION_DELAY);

    // Send a PUT on the write-property key.
    let key_expr = "things/urn:server:write/properties/status".to_string();
    std::thread::spawn(move || {
        session
            .put(key_expr.as_str(), "off")
            .wait()
            .expect("send put");
    });

    let request = poll_with_timeout(&server_binding, REPLY_TIMEOUT, "put");

    assert_eq!(request.target, AffordanceTarget::Property("status".into()));
    assert_eq!(request.operation, Operation::WriteProperty);
    let body = request.input.data.map(|p| p.body).unwrap_or_default();
    assert_eq!(body.as_ref(), b"off");

    // Write-property is fire-and-forget — send_response is a no-op but must not panic.
    server_binding.send_response(InboundResponse::new(
        InteractionOutput::empty(),
        request.correlation,
    ));
}

#[test]
fn runtime_server_binding_unregister_stops_receiving_queries() {
    if !should_run() {
        return;
    }

    zenoh::init_log_from_env_or("error");
    let server_binding = ZenohServerBinding::open(zenoh::Config::default()).unwrap();

    let td = test_td("urn:server:unreg");
    let ctx = default_ctx();
    server_binding
        .serve(&ThingId::from("urn:server:unreg"), &td, &ctx)
        .expect("serve");

    std::thread::sleep(PROPAGATION_DELAY);

    server_binding.shutdown(&ThingId::from("urn:server:unreg"));

    std::thread::sleep(PROPAGATION_DELAY);

    // After shutdown, try_accept should not produce requests.
    std::thread::sleep(PROPAGATION_DELAY);
    assert!(server_binding.try_accept().is_none());
}

#[test]
fn runtime_server_binding_error_reply() {
    if !should_run() {
        return;
    }

    zenoh::init_log_from_env_or("error");
    let server_binding = ZenohServerBinding::open(zenoh::Config::default()).unwrap();
    let session = server_binding.session().clone();

    let td = test_td("urn:server:error");
    let ctx = default_ctx();
    server_binding
        .serve(&ThingId::from("urn:server:error"), &td, &ctx)
        .expect("serve");

    std::thread::sleep(PROPAGATION_DELAY);

    let key_expr = "things/urn:server:error/properties/status";
    let query_thread = std::thread::spawn(move || {
        let replies = session.get(key_expr).wait().expect("send query");
        replies
            .recv_timeout(REPLY_TIMEOUT)
            .expect("receive reply")
            .expect("no reply")
    });

    let request = poll_with_timeout(&server_binding, REPLY_TIMEOUT, "error query");

    let response = InboundResponse::error(
        request.correlation,
        CoreError::UnsupportedOperation(
            ErrorContext::new(ErrorPhase::Handler, RetryClass::Never)
                .with_operation(Operation::ReadAllProperties),
        ),
    );
    server_binding.send_response(response);

    let reply = query_thread.join().expect("join error thread");
    assert!(reply.into_result().is_err(), "expected error reply");
}

// ---------------------------------------------------------------------------
// Event publishing (H2)
// ---------------------------------------------------------------------------

#[test]
fn runtime_server_binding_event_publish_to_subscriber() {
    if !should_run() {
        return;
    }

    zenoh::init_log_from_env_or("error");
    let server_binding = ZenohServerBinding::open(zenoh::Config::default()).unwrap();
    let session = server_binding.session().clone();

    // Build a context carrying an EventBroker *before* serving the thing
    // so that publisher sinks are wired during serve.
    let broker = EventBroker::new();
    let ctx = BindingContext {
        event_broker: broker.clone(),
        dispatch: None,
    };

    let td = test_td("urn:server:event");
    server_binding
        .serve(&ThingId::from("urn:server:event"), &td, &ctx)
        .expect("serve");

    std::thread::sleep(PROPAGATION_DELAY);

    // Subscribe on the client side.
    let key_expr = "things/urn:server:event/events/startup";
    let received = Arc::new(std::sync::Mutex::new(None::<Vec<u8>>));
    let received_clone = received.clone();
    let _subscriber = session
        .declare_subscriber(key_expr)
        .callback(move |sample| {
            *received_clone.lock().unwrap() = Some(sample.payload().to_bytes().into_owned());
        })
        .wait()
        .expect("declare subscriber");

    std::thread::sleep(PROPAGATION_DELAY);

    // Publish the event via the broker — this should fan-out to the
    // ZenohPublisherSink which calls session.put on the key expression.
    let payload = Payload::new(b"event-fired".to_vec(), "text/plain");
    broker
        .publish(
            &ThingId::from("urn:server:event"),
            &EventName::from("startup"),
            &payload,
        )
        .expect("publish event");

    // Wait for zenoh delivery.
    let deadline = std::time::Instant::now() + REPLY_TIMEOUT;
    loop {
        if received.lock().unwrap().is_some() {
            break;
        }
        if std::time::Instant::now() > deadline {
            panic!("timed out waiting for event delivery");
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let guard = received.lock().unwrap();
    assert_eq!(
        guard.as_deref(),
        Some(b"event-fired".as_slice()),
        "subscriber should receive the published event payload"
    );
}

// ---------------------------------------------------------------------------
// Async driving layer — removed: `AsyncServerBinding` was deleted in P0 (the
// v4.0 `ServerBinding` is the single inbound contract; async driving lives in
// the Servient, P3, draining its bounded fan-in channel). The per-binding
// async `poll_accept` this test exercised no longer exists; the equivalent
// coverage (Servient fan-in driving) lands with P3.
// ---------------------------------------------------------------------------

// (removed: runtime_async_server_binding_poll_accept — tested a deleted surface)

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn default_ctx() -> BindingContext {
    BindingContext {
        event_broker: EventBroker::new(),
        dispatch: None,
    }
}

fn poll_with_timeout(
    binding: &ZenohServerBinding,
    timeout: Duration,
    label: &str,
) -> InboundRequest {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if let Some(request) = binding.try_accept() {
            return request;
        }
        if std::time::Instant::now() > deadline {
            panic!("timed out waiting for {label}");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn test_td(id: &str) -> Thing {
    use clinkz_wot_td::{
        affordance::{ActionAffordance, EventAffordance, InteractionHelper, PropertyAffordance},
        data_schema::DataSchema,
        form::Form,
        thing::Thing,
    };

    // Use the thing id as a uniqueness prefix so parallel tests don't collide.
    let prefix = format!("clinkz/things/{id}");
    let read_href = format!("zenoh://{prefix}/properties/status");
    let write_href = format!("zenoh://{prefix}/properties/status");
    let action_href = format!("zenoh://{prefix}/actions/echo");
    let event_href = format!("zenoh://{prefix}/events/startup");

    let read_property = Form::read_property(&read_href)
        .content_type("text/plain")
        .build()
        .unwrap();
    let write_property = Form::write_property(&write_href)
        .content_type("text/plain")
        .build()
        .unwrap();
    let invoke_action = Form::invoke_action(&action_href)
        .content_type("text/plain")
        .build()
        .unwrap();
    let subscribe_event = Form::subscribe_event(&event_href)
        .content_type("text/plain")
        .build()
        .unwrap();

    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .forms([read_property, write_property])
        .build()
        .unwrap();
    let action = ActionAffordance::builder()
        .form(invoke_action)
        .build()
        .unwrap();
    let event = EventAffordance::builder()
        .form(subscribe_event)
        .build()
        .unwrap();

    Thing::builder("Test Lamp")
        .id(id)
        .nosec()
        .property("status", property)
        .action("echo", action)
        .event("startup", event)
        .build()
        .unwrap()
}

fn should_run() -> bool {
    if env::var(RUN_ZENOH_RUNTIME_TESTS).ok().as_deref() == Some("1") {
        true
    } else {
        eprintln!("skipping zenoh server binding smoke test; set {RUN_ZENOH_RUNTIME_TESTS}=1");
        false
    }
}

// Suppress unused import warnings when tests are skipped.
#[allow(dead_code)]
fn _unused() {
    let _ = ThingId::from("");
    let _: Arc<dyn InboundDispatcher> = Arc::new(StubDispatcher);
    let _ = InboundRequest::new(
        ThingId::from(""),
        AffordanceTarget::Thing,
        Operation::ReadProperty,
        InteractionInput::empty(),
    );
    let _ = InboundResponse::new(
        InteractionOutput::empty(),
        clinkz_wot_core::identity::CorrelationId::empty(),
    );
}

struct StubDispatcher;
impl InboundDispatcher for StubDispatcher {
    fn dispatch(&self, _request: InboundRequest) -> clinkz_wot_core::CoreResult<InboundResponse> {
        Ok(InboundResponse::new(
            InteractionOutput::empty(),
            clinkz_wot_core::identity::CorrelationId::empty(),
        ))
    }
}
