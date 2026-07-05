//! P3 integration: produce→expose→dispatch round-trip and consume→invoke via
//! fake bindings, plus frozen-TD lifecycle (expose/destroy).

#![cfg(feature = "async")]

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use clinkz_wot_core::{
    AffordanceTarget, BindingRequest, ClientBinding, CoreError, EventBroker, FanInSender,
    InboundRequest, InboundResponse, InteractionInput, InteractionOptions, InteractionOutput,
    Payload, PropertyReadHandler, ServerBinding, ThingId,
};
use clinkz_wot_servient::{ClientBindingFactory, ServientBuilder};
use clinkz_wot_td::{
    affordance::{InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    data_type::Operation,
    thing::Thing,
};

// --- fake server binding ---

#[derive(Default)]
struct FakeServer {
    sink: Mutex<Option<FanInSender<InboundRequest>>>,
    responses: Mutex<Vec<InboundResponse>>,
    registered: Mutex<Vec<String>>,
}

impl ServerBinding for FakeServer {
    fn try_accept(&self) -> Option<InboundRequest> {
        None
    }
    fn send_response(&self, response: InboundResponse) {
        self.responses.lock().unwrap().push(response);
    }
    fn set_event_broker(&self, _broker: EventBroker) {}
    fn set_request_sink(&self, sender: FanInSender<InboundRequest>) {
        *self.sink.lock().unwrap() = Some(sender);
    }
    fn register_thing(&self, thing_id: &ThingId, _td: &Thing) -> Result<(), CoreError> {
        self.registered
            .lock()
            .unwrap()
            .push(thing_id.as_str().to_string());
        Ok(())
    }
    fn unregister_thing(&self, thing_id: &ThingId) {
        self.registered
            .lock()
            .unwrap()
            .retain(|s| s != thing_id.as_str());
    }
}

// --- fake client binding ---

struct EchoClient;
#[async_trait]
impl ClientBinding for EchoClient {
    fn supports(&self, _form: &clinkz_wot_td::form::Form, _op: Operation) -> bool {
        true
    }
    async fn invoke(&self, request: BindingRequest) -> Result<InteractionOutput, CoreError> {
        Ok(InteractionOutput::with_data(
            request.input.data.unwrap_or_default(),
        ))
    }
}

struct EchoClientFactory;
impl ClientBindingFactory for EchoClientFactory {
    fn build(&self) -> Box<dyn ClientBinding> {
        Box::new(EchoClient)
    }
}

// --- fixtures ---

fn lamp_td() -> Thing {
    Thing::builder("Lamp")
        .id("urn:test:lamp")
        .nosec()
        .property(
            "status",
            PropertyAffordance::builder(DataSchema::string())
                .form(
                    clinkz_wot_td::form::Form::read_property("zenoh://clinkz/lamp/status")
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
}

struct StoredRead(Arc<Mutex<Payload>>);
impl PropertyReadHandler for StoredRead {
    fn read(
        &self,
        _input: &clinkz_wot_core::InteractionInput,
    ) -> Result<InteractionOutput, CoreError> {
        Ok(InteractionOutput::with_data(self.0.lock().unwrap().clone()))
    }
}

#[tokio::test]
async fn produce_expose_registers_and_dispatches() {
    let fake_server = Arc::new(FakeServer::default());
    let servient = ServientBuilder::new()
        .with_server_binding(fake_server.clone())
        .with_client_factory(Arc::new(EchoClientFactory))
        .build()
        .expect("build servient");

    let value = Arc::new(Mutex::new(Payload::new(b"on".to_vec(), "text/plain")));
    let handle = servient.produce(lamp_td()).expect("produce");
    handle.set_property_read_handler("status", StoredRead(value.clone()));
    handle.expose().await.expect("expose");

    // expose() registered the Thing on the fake binding.
    assert_eq!(fake_server.registered.lock().unwrap().len(), 1);

    // Simulate a remote read: push an InboundRequest via the fan-in sender.
    let sender = fake_server.sink.lock().unwrap().clone().expect("sink set");
    let request = InboundRequest::new(
        ThingId::from("urn:test:lamp"),
        AffordanceTarget::Property("status".into()),
        Operation::ReadProperty,
        InteractionInput::empty(),
    );
    sender.send(request).await.expect("send inbound");

    // Drive one step: dispatches and replies via send_response.
    servient.poll_serve().await.expect("poll_serve");

    // The handler's value reached the response.
    let responses = fake_server.responses.lock().unwrap();
    assert_eq!(responses.len(), 1);
    let body = responses[0].output.data.as_ref().unwrap().body.as_ref();
    assert_eq!(body, b"on");
}

#[tokio::test]
async fn consume_invokes_via_client_binding() {
    let fake_server = Arc::new(FakeServer::default());
    let servient = ServientBuilder::new()
        .with_server_binding(fake_server.clone())
        .with_client_factory(Arc::new(EchoClientFactory))
        .build()
        .expect("build servient");

    let handle = servient.consume(lamp_td()).expect("consume");
    let out = handle
        .read_property("status", InteractionOptions::new())
        .await
        .expect("read");
    // EchoClient returns the input data (empty here); no error means the
    // form selection + async invoke path worked end-to-end.
    let _ = out;
}

#[tokio::test]
async fn destroy_unregisters() {
    let fake_server = Arc::new(FakeServer::default());
    let servient = ServientBuilder::new()
        .with_server_binding(fake_server.clone())
        .with_client_factory(Arc::new(EchoClientFactory))
        .build()
        .expect("build servient");

    let handle = servient.produce(lamp_td()).expect("produce");
    handle.expose().await.expect("expose");
    assert_eq!(fake_server.registered.lock().unwrap().len(), 1);

    handle.destroy().await.expect("destroy");
    assert!(fake_server.registered.lock().unwrap().is_empty());

    // Idempotent destroy (AD27/E13): second destroy is a no-op Ok.
    handle.destroy().await.expect("idempotent destroy");
}

// --- Scripting API conformance map expansion (P4 §4.1) ---

#[tokio::test]
async fn producer_write_property_local() {
    let fake_server = Arc::new(FakeServer::default());
    let servient = ServientBuilder::new()
        .with_server_binding(fake_server)
        .with_client_factory(Arc::new(EchoClientFactory))
        .build()
        .expect("build");

    let value = Arc::new(Mutex::new(Payload::new(b"off".to_vec(), "text/plain")));
    let handle = servient.produce(lamp_td()).expect("produce");
    handle.set_property_write_handler("status", StoredWrite(value.clone()));
    handle.expose().await.expect("expose");

    let mut input =
        InteractionInput::with_data(Payload::new(b"on".to_vec(), "text/plain"));
    handle.write_property("status", &mut input).expect("write");
    assert_eq!(
        value.lock().unwrap().body.as_ref(),
        b"on"
    );
}

struct StoredWrite(Arc<Mutex<Payload>>);
impl clinkz_wot_core::PropertyWriteHandler for StoredWrite {
    fn write(
        &self,
        input: &mut clinkz_wot_core::InteractionInput,
    ) -> Result<InteractionOutput, CoreError> {
        *self.0.lock().unwrap() = input.data.take().unwrap();
        Ok(InteractionOutput::empty())
    }
}

#[tokio::test]
async fn missing_handler_on_exposed_but_unwired_affordance() {
    let fake_server = Arc::new(FakeServer::default());
    let servient = ServientBuilder::new()
        .with_server_binding(fake_server)
        .with_client_factory(Arc::new(EchoClientFactory))
        .build()
        .expect("build");

    let handle = servient.produce(lamp_td()).expect("produce");
    handle.expose().await.expect("expose");

    // No read handler set → MissingHandler (AD14: designed-in semantic for
    // exposed-but-unwired).
    let err = handle
        .read_property("status", &InteractionInput::empty())
        .unwrap_err();
    assert!(matches!(
        err,
        CoreError::MissingHandler { .. }
    ));
}

#[tokio::test]
async fn producer_emit_event_succeeds() {
    let fake_server = Arc::new(FakeServer::default());
    let servient = ServientBuilder::new()
        .with_server_binding(fake_server)
        .with_client_factory(Arc::new(EchoClientFactory))
        .build()
        .expect("build");

    let handle = servient.produce(lamp_td()).expect("produce");
    handle.expose().await.expect("expose");

    // emit_event publishes via the broker; succeeds even with no subscribers
    // (no-op fan-out).
    handle
        .emit_event("status", Payload::new(b"change".to_vec(), "text/plain"))
        .expect("emit");
    handle
        .emit_property_change("status", Payload::new(b"v2".to_vec(), "text/plain"))
        .expect("emit change");
}

#[tokio::test]
async fn discover_returns_lazy_process() {
    let fake_server = Arc::new(FakeServer::default());
    let servient = ServientBuilder::new()
        .with_server_binding(fake_server)
        .with_client_factory(Arc::new(EchoClientFactory))
        .build()
        .expect("build");

    // discover() is sync and returns immediately (AD10). With an empty
    // directory, the first next() yields None.
    let mut process = servient.discover(clinkz_wot_discovery::DiscoveryFilter::all());
    let result = process.next().await.expect("next should not error");
    assert!(result.is_none(), "empty directory → clean end");
}

#[tokio::test]
async fn all_producer_handler_setters_compile_and_register() {
    // Smoke-test: every set_*_handler variant compiles and the handler is
    // registered (read dispatches successfully; others are no-ops if unwired).
    let fake_server = Arc::new(FakeServer::default());
    let servient = ServientBuilder::new()
        .with_server_binding(fake_server)
        .with_client_factory(Arc::new(EchoClientFactory))
        .build()
        .expect("build");

    let handle = servient.produce(lamp_td()).expect("produce");
    handle.set_property_read_handler("status", StoredRead(Arc::new(Mutex::new(
        Payload::new(b"x".to_vec(), "text/plain"),
    ))));
    handle.set_property_write_handler("status", StoredWrite(Arc::new(Mutex::new(
        Payload::new(b"y".to_vec(), "text/plain"),
    ))));
    handle.set_property_observe_handler(
        "status",
        struct_observe(),
    );
    handle.set_property_unobserve_handler("status", struct_unobserve());
    handle.set_action_handler("status", struct_action());
    handle.set_action_query_handler("status", struct_query());
    handle.set_action_cancel_handler("status", struct_cancel());
    handle.set_event_subscribe_handler("status", struct_subscribe());
    handle.set_event_unsubscribe_handler("status", struct_unsubscribe());
    handle.expose().await.expect("expose");

    // Read succeeds (handler was set).
    let out = handle
        .read_property("status", &InteractionInput::empty())
        .expect("read");
    assert_eq!(out.data.unwrap().body.as_ref(), b"x");
}

// Trivial handler stubs for the compile-and-register smoke test.
fn struct_observe() -> impl clinkz_wot_core::PropertyObserveHandler {
    struct H;
    impl clinkz_wot_core::PropertyObserveHandler for H {
        fn observe(
            &self,
            _: &clinkz_wot_core::InteractionInput,
            _push: &mut dyn FnMut(Payload) -> Result<(), CoreError>,
        ) -> Result<InteractionOutput, CoreError> {
            Ok(InteractionOutput::empty())
        }
    }
    H
}
fn struct_unobserve() -> impl clinkz_wot_core::PropertyUnobserveHandler {
    struct H;
    impl clinkz_wot_core::PropertyUnobserveHandler for H {
        fn unobserve(
            &self,
            _: &clinkz_wot_core::InteractionInput,
        ) -> Result<InteractionOutput, CoreError> {
            Ok(InteractionOutput::empty())
        }
    }
    H
}
fn struct_action() -> impl clinkz_wot_core::ActionHandler {
    struct H;
    impl clinkz_wot_core::ActionHandler for H {
        fn invoke(
            &self,
            _: &mut clinkz_wot_core::InteractionInput,
        ) -> Result<InteractionOutput, CoreError> {
            Ok(InteractionOutput::empty())
        }
    }
    H
}
fn struct_query() -> impl clinkz_wot_core::ActionQueryHandler {
    struct H;
    impl clinkz_wot_core::ActionQueryHandler for H {
        fn query(
            &self,
            _: &clinkz_wot_core::InteractionInput,
        ) -> Result<InteractionOutput, CoreError> {
            Ok(InteractionOutput::empty())
        }
    }
    H
}
fn struct_cancel() -> impl clinkz_wot_core::ActionCancelHandler {
    struct H;
    impl clinkz_wot_core::ActionCancelHandler for H {
        fn cancel(
            &self,
            _: &mut clinkz_wot_core::InteractionInput,
        ) -> Result<InteractionOutput, CoreError> {
            Ok(InteractionOutput::empty())
        }
    }
    H
}
fn struct_subscribe() -> impl clinkz_wot_core::EventSubscribeHandler {
    struct H;
    impl clinkz_wot_core::EventSubscribeHandler for H {
        fn subscribe(
            &self,
            _: &clinkz_wot_core::InteractionInput,
            _push: &mut dyn FnMut(Payload) -> Result<(), CoreError>,
        ) -> Result<InteractionOutput, CoreError> {
            Ok(InteractionOutput::empty())
        }
    }
    H
}
fn struct_unsubscribe() -> impl clinkz_wot_core::EventUnsubscribeHandler {
    struct H;
    impl clinkz_wot_core::EventUnsubscribeHandler for H {
        fn unsubscribe(
            &self,
            _: &clinkz_wot_core::InteractionInput,
        ) -> Result<InteractionOutput, CoreError> {
            Ok(InteractionOutput::empty())
        }
    }
    H
}
