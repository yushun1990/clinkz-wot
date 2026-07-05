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
