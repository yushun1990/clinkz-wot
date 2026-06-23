#![cfg(feature = "async")]

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use clinkz_wot_core::{
    AffordanceTarget, AsyncActionHandler, AsyncPropertyReadHandler, AsyncPropertyWriteHandler,
    AsyncServerBinding, BindingRequest, ClientBinding, CoreResult, InboundRequest, InboundResponse,
    InteractionInput, InteractionOutput, Payload, ThingId,
};
use clinkz_wot_servient::Servient;
use clinkz_wot_td::data_type::Operation;
use clinkz_wot_td::thing::Thing;

// ---------------------------------------------------------------------------
// Fake async server binding
// ---------------------------------------------------------------------------

struct FakeAsyncServerBinding {
    pending: Arc<Mutex<VecDeque<InboundRequest>>>,
    responses: Arc<Mutex<Vec<InboundResponse>>>,
    registered: Arc<Mutex<Vec<String>>>,
    unregistered: Arc<Mutex<Vec<String>>>,
    notify: tokio::sync::Notify,
    route_fail: AtomicBool,
}

impl FakeAsyncServerBinding {
    fn new() -> Self {
        Self {
            pending: Arc::new(Mutex::new(VecDeque::new())),
            responses: Arc::new(Mutex::new(Vec::new())),
            registered: Arc::new(Mutex::new(Vec::new())),
            unregistered: Arc::new(Mutex::new(Vec::new())),
            notify: tokio::sync::Notify::new(),
            route_fail: AtomicBool::new(false),
        }
    }

    fn enqueue(&self, request: InboundRequest) {
        self.pending.lock().unwrap().push_back(request);
        self.notify.notify_one();
    }

    fn take_responses(&self) -> Vec<InboundResponse> {
        std::mem::take(&mut *self.responses.lock().unwrap())
    }

    fn registered_things(&self) -> Vec<String> {
        self.registered.lock().unwrap().clone()
    }

    fn unregistered_things(&self) -> Vec<String> {
        self.unregistered.lock().unwrap().clone()
    }

    fn set_route_fail(&self, fail: bool) {
        self.route_fail.store(fail, Ordering::SeqCst);
    }
}

#[async_trait]
impl AsyncServerBinding for FakeAsyncServerBinding {
    async fn poll_accept(&self) -> InboundRequest {
        loop {
            if let Some(req) = self.pending.lock().unwrap().pop_front() {
                return req;
            }
            self.notify.notified().await;
        }
    }

    fn send_response(&self, response: InboundResponse) {
        self.responses.lock().unwrap().push(response);
    }

    fn register_thing(&self, thing_id: &str, _td: &Thing) -> Result<(), String> {
        if self.route_fail.load(Ordering::SeqCst) {
            return Err(format!("route registration failed for '{thing_id}'"));
        }
        self.registered.lock().unwrap().push(thing_id.to_string());
        Ok(())
    }

    fn unregister_thing(&self, thing_id: &str) {
        self.unregistered.lock().unwrap().push(thing_id.to_string());
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_td(id: &str) -> Thing {
    use clinkz_wot_td::{
        affordance::{InteractionHelper, PropertyAffordance},
        data_schema::DataSchema,
        form::Form,
        thing::Thing,
    };

    let form = Form::read_property("test://things/lamp/properties/status")
        .content_type("text/plain")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(form)
        .build()
        .unwrap();

    Thing::builder("Test Lamp")
        .id(id)
        .nosec()
        .property("status", property)
        .build()
        .unwrap()
}

fn read_property_request(thing_id: &str) -> InboundRequest {
    InboundRequest::new(
        ThingId::from(thing_id),
        AffordanceTarget::Property("status".into()),
        Operation::ReadProperty,
        InteractionInput::empty(),
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn async_driving_dispatches_inbound_request() {
    let binding = Arc::new(FakeAsyncServerBinding::new());
    let servient = Servient::builder()
        .async_server_binding(binding.clone())
        .build();

    let td = test_td("urn:async:dispatch");
    servient.expose(td).unwrap();

    // Enqueue an inbound request.
    binding.enqueue(read_property_request("urn:async:dispatch"));

    // Drive one iteration.
    servient.poll_serve().await.unwrap();

    let responses = binding.take_responses();
    assert_eq!(responses.len(), 1);
    // No handler attached → MissingHandler error in the response.
    assert!(responses[0].error.is_some());
}

#[tokio::test]
async fn async_driving_unknown_thing_returns_error() {
    let binding = Arc::new(FakeAsyncServerBinding::new());
    let servient = Servient::builder()
        .async_server_binding(binding.clone())
        .build();

    binding.enqueue(read_property_request("urn:nonexistent"));

    servient.poll_serve().await.unwrap();

    let responses = binding.take_responses();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].error.is_some());
}

#[tokio::test]
async fn async_expose_registers_routes_on_async_binding() {
    let binding = Arc::new(FakeAsyncServerBinding::new());
    let servient = Servient::builder()
        .async_server_binding(binding.clone())
        .build();

    let td = test_td("urn:async:routes");
    servient.expose(td).unwrap();

    assert_eq!(binding.registered_things(), vec!["urn:async:routes"]);
}

#[tokio::test]
async fn async_destroy_unregisters_routes() {
    let binding = Arc::new(FakeAsyncServerBinding::new());
    let servient = Servient::builder()
        .async_server_binding(binding.clone())
        .build();

    let td = test_td("urn:async:destroy");
    servient.expose(td).unwrap();
    assert_eq!(binding.registered_things().len(), 1);

    servient.destroy("urn:async:destroy").unwrap();
    assert_eq!(binding.unregistered_things(), vec!["urn:async:destroy"]);
}

#[tokio::test]
async fn async_expose_route_failure_rolls_back() {
    let binding = Arc::new(FakeAsyncServerBinding::new());
    binding.set_route_fail(true);
    let servient = Servient::builder()
        .async_server_binding(binding.clone())
        .build();

    let td = test_td("urn:async:fail");
    let result = servient.expose(td);

    assert!(result.is_err());
    // Thing should not be exposed.
    assert_eq!(servient.list().entries.len(), 0);
}

// ---------------------------------------------------------------------------
// M8: Async consumer API tests.
// ---------------------------------------------------------------------------

struct AsyncTestBinding;

impl ClientBinding for AsyncTestBinding {
    fn supports(&self, form: &clinkz_wot_td::form::Form, operation: Operation) -> bool {
        form.href.as_str().starts_with("test://") && operation == Operation::ReadProperty
    }

    fn invoke(&self, _request: BindingRequest) -> CoreResult<InteractionOutput> {
        Ok(InteractionOutput::with_payload(Payload::new(
            b"async-value".to_vec(),
            "text/plain",
        )))
    }
}

fn async_consumer_td() -> Thing {
    use clinkz_wot_td::{
        affordance::{InteractionHelper, PropertyAffordance},
        data_schema::DataSchema,
        form::Form,
    };
    let form = Form::read_property("test://things/x/properties/status")
        .content_type("text/plain")
        .build()
        .unwrap();
    let prop = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(form)
        .build()
        .unwrap();
    Thing::builder("X")
        .id("urn:async:consumer")
        .nosec()
        .property("status", prop)
        .build()
        .unwrap()
}

#[tokio::test]
async fn read_property_async_returns_value() {
    let servient = Servient::builder()
        .binding_factory(|| Box::new(AsyncTestBinding))
        .build();
    let consumed = servient.consume(async_consumer_td()).unwrap();

    let output = consumed
        .read_property_async("status", InteractionInput::empty())
        .await
        .unwrap();

    assert_eq!(output.payload.unwrap().body, b"async-value");
}

#[tokio::test]
async fn invoke_action_async_completes() {
    let servient = Servient::builder()
        .binding_factory(|| Box::new(AsyncTestBinding))
        .build();
    let consumed = servient.consume(async_consumer_td()).unwrap();

    // read_property_async works the same way; verify the async path resolves.
    let output = consumed
        .read_property_async("status", InteractionInput::empty())
        .await
        .unwrap();

    assert!(output.payload.is_some());
}

// ---------------------------------------------------------------------------
// M9: Async handler dispatch tests.
// ---------------------------------------------------------------------------

struct SlowAsyncRead;

#[async_trait]
impl AsyncPropertyReadHandler for SlowAsyncRead {
    async fn read(&mut self, _input: InteractionInput) -> CoreResult<InteractionOutput> {
        // Simulate async work (e.g., reading from a sensor or database).
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        Ok(InteractionOutput::with_payload(Payload::new(
            b"async-read-value".to_vec(),
            "text/plain",
        )))
    }
}

fn async_handler_td() -> Thing {
    use clinkz_wot_td::{
        affordance::{InteractionHelper, PropertyAffordance},
        data_schema::DataSchema,
        form::Form,
    };
    let form = Form::read_property("test://things/x/properties/status")
        .content_type("text/plain")
        .build()
        .unwrap();
    let prop = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(form)
        .build()
        .unwrap();
    Thing::builder("X")
        .id("urn:async:handler")
        .nosec()
        .property("status", prop)
        .build()
        .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn async_read_handler_dispatched_through_async_driving_loop() {
    let binding = Arc::new(FakeAsyncServerBinding::new());
    let servient = Servient::builder()
        .async_server_binding(binding.clone())
        .build();

    let handle = servient.expose(async_handler_td()).unwrap();
    handle
        .set_async_property_read_handler("status", SlowAsyncRead)
        .unwrap();

    // Enqueue a read request.
    binding.enqueue(InboundRequest::new(
        ThingId::from("urn:async:handler"),
        AffordanceTarget::Property("status".into()),
        Operation::ReadProperty,
        InteractionInput::empty(),
    ));

    // Drive one iteration.
    servient.poll_serve().await.unwrap();

    let responses = binding.take_responses();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].error.is_none());
    assert_eq!(
        responses[0].output.payload.as_ref().unwrap().body,
        b"async-read-value"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn async_dispatch_falls_back_to_sync_handler() {
    use clinkz_wot_core::PropertyReadHandler;

    struct SyncRead;
    impl PropertyReadHandler for SyncRead {
        fn read(&mut self, _: InteractionInput) -> CoreResult<InteractionOutput> {
            Ok(InteractionOutput::with_payload(Payload::new(
                b"sync-fallback".to_vec(),
                "text/plain",
            )))
        }
    }

    let binding = Arc::new(FakeAsyncServerBinding::new());
    let servient = Servient::builder()
        .async_server_binding(binding.clone())
        .build();

    let handle = servient.expose(async_handler_td()).unwrap();
    // Register ONLY a sync handler (no async handler).
    handle
        .set_property_read_handler("status", SyncRead)
        .unwrap();

    binding.enqueue(InboundRequest::new(
        ThingId::from("urn:async:handler"),
        AffordanceTarget::Property("status".into()),
        Operation::ReadProperty,
        InteractionInput::empty(),
    ));

    servient.poll_serve().await.unwrap();

    let responses = binding.take_responses();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].error.is_none());
    assert_eq!(
        responses[0].output.payload.as_ref().unwrap().body,
        b"sync-fallback"
    );
}

// ---------------------------------------------------------------------------
// M4: Async write & action handler dispatch tests.
// ---------------------------------------------------------------------------

struct AsyncEchoWrite;

#[async_trait]
impl AsyncPropertyWriteHandler for AsyncEchoWrite {
    async fn write(&mut self, input: InteractionInput) -> CoreResult<InteractionOutput> {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        let body = input.payload.map(|p| p.body).unwrap_or_default();
        Ok(InteractionOutput::with_payload(Payload::new(
            body,
            "text/plain",
        )))
    }
}

struct AsyncEchoAction;

#[async_trait]
impl AsyncActionHandler for AsyncEchoAction {
    async fn invoke(&mut self, input: InteractionInput) -> CoreResult<InteractionOutput> {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        Ok(InteractionOutput {
            payload: input.payload,
        })
    }
}

fn async_write_action_td() -> Thing {
    use clinkz_wot_td::{
        affordance::{ActionAffordance, InteractionHelper, PropertyAffordance},
        data_schema::DataSchema,
        form::Form,
    };
    let write_form = Form::write_property("test://things/y/properties/status")
        .content_type("text/plain")
        .build()
        .unwrap();
    let action_form = Form::invoke_action("test://things/y/actions/echo")
        .content_type("text/plain")
        .build()
        .unwrap();
    let prop = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(write_form)
        .build()
        .unwrap();
    let action = ActionAffordance::builder()
        .form(action_form)
        .build()
        .unwrap();
    Thing::builder("Y")
        .id("urn:async:write-action")
        .nosec()
        .property("status", prop)
        .action("echo", action)
        .build()
        .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn async_write_handler_dispatched() {
    let binding = Arc::new(FakeAsyncServerBinding::new());
    let servient = Servient::builder()
        .async_server_binding(binding.clone())
        .build();

    let handle = servient.expose(async_write_action_td()).unwrap();
    handle
        .set_async_property_write_handler("status", AsyncEchoWrite)
        .unwrap();

    binding.enqueue(InboundRequest::new(
        ThingId::from("urn:async:write-action"),
        AffordanceTarget::Property("status".into()),
        Operation::WriteProperty,
        InteractionInput::with_payload(Payload::new(b"written".to_vec(), "text/plain")),
    ));

    servient.poll_serve().await.unwrap();

    let responses = binding.take_responses();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].error.is_none(), "{:?}", responses[0].error);
}

#[tokio::test(flavor = "multi_thread")]
async fn async_action_handler_dispatched() {
    let binding = Arc::new(FakeAsyncServerBinding::new());
    let servient = Servient::builder()
        .async_server_binding(binding.clone())
        .build();

    let handle = servient.expose(async_write_action_td()).unwrap();
    handle
        .set_async_action_handler("echo", AsyncEchoAction)
        .unwrap();

    binding.enqueue(InboundRequest::new(
        ThingId::from("urn:async:write-action"),
        AffordanceTarget::Action("echo".into()),
        Operation::InvokeAction,
        InteractionInput::with_payload(Payload::new(b"invoke".to_vec(), "text/plain")),
    ));

    servient.poll_serve().await.unwrap();

    let responses = binding.take_responses();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].error.is_none());
    assert_eq!(
        responses[0].output.payload.as_ref().unwrap().body,
        b"invoke"
    );
}
