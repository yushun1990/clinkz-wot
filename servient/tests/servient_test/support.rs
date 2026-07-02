#![allow(dead_code)]

use std::{
    borrow::Cow,
    cell::Cell,
    collections::VecDeque,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use clinkz_wot_core::{
    ActionHandler, AffordanceTarget, BindingRequest, ClientBinding, CodecInput, CoreResult,
    EventSink, EventSubscribeHandler, EventUnsubscribeHandler, InboundRequest, InboundResponse,
    InteractionInput, InteractionOutput, Payload, PayloadCodec, PropertyObserveHandler,
    PropertyReadHandler, PropertyWriteHandler, SecurityContext, SecurityProvider,
    ServerBinding as ServerBindingTrait, SubscriptionGuard, TransportRequest,
};

#[cfg(feature = "test-zenoh")]
use clinkz_wot_core::Subscription;
use clinkz_wot_td::{
    affordance::{ActionAffordance, EventAffordance, InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    data_type::Operation,
    form::Form,
    security_scheme::SecurityScheme,
    thing::Thing,
};

#[cfg(feature = "test-zenoh")]
use clinkz_wot_protocol_bindings_zenoh::{
    ZenohOperationKind, ZenohTransport, ZenohTransportRequest,
};

/// Shared-state property read/write handlers for tests that need both read and
/// write on the same property (split handler model requires shared state).
pub(crate) fn shared_status(value: Payload) -> (SharedStatusRead, SharedStatusWrite) {
    let shared = Arc::new(Mutex::new(value));
    (
        SharedStatusRead {
            value: Arc::clone(&shared),
        },
        SharedStatusWrite { value: shared },
    )
}

pub(crate) struct SharedStatusRead {
    value: Arc<Mutex<Payload>>,
}

impl PropertyReadHandler for SharedStatusRead {
    fn read(&self, _input: InteractionInput) -> CoreResult<InteractionOutput> {
        Ok(InteractionOutput::with_payload(
            self.value.lock().unwrap().clone(),
        ))
    }
}

pub(crate) struct SharedStatusWrite {
    value: Arc<Mutex<Payload>>,
}

impl PropertyWriteHandler for SharedStatusWrite {
    fn write(&self, input: InteractionInput) -> CoreResult<InteractionOutput> {
        *self.value.lock().unwrap() = input.payload.expect("test write payload");
        Ok(InteractionOutput::empty())
    }
}

/// Read-only status handler for tests that only need read property dispatch.
#[derive(Clone)]
pub(crate) struct StatusRead {
    pub(crate) value: Payload,
}

impl PropertyReadHandler for StatusRead {
    fn read(&self, _input: InteractionInput) -> CoreResult<InteractionOutput> {
        Ok(InteractionOutput::with_payload(self.value.clone()))
    }
}

/// Property handler that captures the principal from the interaction input,
/// verifying that inbound security context is threaded through to handlers.
pub(crate) struct PrincipalCapturingProperty {
    pub(crate) captured_principal: Rc<std::cell::RefCell<Option<clinkz_wot_core::Principal>>>,
}

impl PropertyReadHandler for PrincipalCapturingProperty {
    fn read(&self, input: InteractionInput) -> CoreResult<InteractionOutput> {
        *self.captured_principal.borrow_mut() = input.principal.clone();
        Ok(InteractionOutput::with_payload(Payload::new(
            b"ok".to_vec(),
            "text/plain",
        )))
    }
}

pub(crate) struct EchoAction;

impl ActionHandler for EchoAction {
    fn invoke(&self, input: InteractionInput) -> CoreResult<InteractionOutput> {
        Ok(InteractionOutput {
            payload: input.payload,
        })
    }
}

/// Action handler that calls `destroy(own_id)` during invocation, to verify
/// the deferred-removal path does not self-deadlock (baseline §7 edge case).
pub(crate) struct SelfDestroyingAction {
    pub(crate) servient: clinkz_wot_servient::Servient,
    pub(crate) thing_id: String,
    pub(crate) destroyed: Rc<Cell<bool>>,
}

impl ActionHandler for SelfDestroyingAction {
    fn invoke(&self, _input: InteractionInput) -> CoreResult<InteractionOutput> {
        // Destroy from within the handler — this must not deadlock.
        let result = self.servient.destroy(&self.thing_id);
        self.destroyed.set(result.is_ok());
        Ok(InteractionOutput::with_payload(Payload::new(
            b"destroyed".to_vec(),
            "text/plain",
        )))
    }
}

pub(crate) struct StartupEvent;

impl EventSubscribeHandler for StartupEvent {
    fn subscribe(
        &self,
        _input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> CoreResult<InteractionOutput> {
        sink.emit(Payload::new(b"ready".to_vec(), "text/plain"))?;
        Ok(InteractionOutput::empty())
    }
}

/// Event unsubscribe handler that records the unsubscribe call for testing.
pub(crate) struct RecordingUnsubscribe {
    pub(crate) called: Rc<Cell<bool>>,
}

impl EventUnsubscribeHandler for RecordingUnsubscribe {
    fn unsubscribe(&self, _input: InteractionInput) -> CoreResult<InteractionOutput> {
        self.called.set(true);
        Ok(InteractionOutput::empty())
    }
}

/// Property observe handler that emits an initial value through the sink.
pub(crate) struct ObserveInitial {
    pub(crate) initial: Payload,
}

impl PropertyObserveHandler for ObserveInitial {
    fn observe(
        &self,
        _input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> CoreResult<InteractionOutput> {
        sink.emit(self.initial.clone())?;
        Ok(InteractionOutput::empty())
    }
}

#[derive(Default)]
pub(crate) struct CollectSink {
    pub(crate) payloads: Vec<Payload>,
}

impl EventSink for CollectSink {
    fn emit(&mut self, payload: Payload) -> CoreResult<()> {
        self.payloads.push(payload);
        Ok(())
    }
}

pub(crate) struct CountingCodec {
    pub(crate) encode_calls: Rc<Cell<usize>>,
    pub(crate) decode_calls: Rc<Cell<usize>>,
}

impl PayloadCodec for CountingCodec {
    fn content_type(&self) -> Cow<'_, str> {
        "text/plain".into()
    }

    fn encode(&self, input: CodecInput<'_>) -> CoreResult<Payload> {
        self.encode_calls.set(self.encode_calls.get() + 1);
        Ok(Payload::new(input.body.to_vec(), "text/plain"))
    }

    fn decode(&self, payload: &Payload) -> CoreResult<Vec<u8>> {
        self.decode_calls.set(self.decode_calls.get() + 1);
        Ok(payload.body.as_ref().to_vec())
    }
}

/// Property handler that does not assume transport security was applied, for
/// testing the local direct-dispatch path (baseline §6).
pub(crate) struct LocalUnsecuredStatusProperty;

impl PropertyReadHandler for LocalUnsecuredStatusProperty {
    fn read(&self, _input: InteractionInput) -> CoreResult<InteractionOutput> {
        Ok(InteractionOutput::with_payload(Payload::new(
            b"local-direct".to_vec(),
            "text/plain",
        )))
    }
}

pub(crate) struct RecordingSecurityProvider {
    pub(crate) applied_calls: Rc<Cell<usize>>,
}

impl SecurityProvider for RecordingSecurityProvider {
    fn scheme_name(&self) -> &str {
        "token"
    }

    fn apply(
        &self,
        context: SecurityContext<'_>,
        request: &mut TransportRequest,
    ) -> CoreResult<()> {
        self.applied_calls.set(self.applied_calls.get() + 1);
        assert_eq!(context.scheme_name, "token");
        assert_eq!(context.scheme.scheme(), "bearer");
        request.metadata.insert("auth".to_owned(), "ok".to_owned());
        Ok(())
    }

    fn supports_scopes(&self, scopes: &[String]) -> bool {
        scopes.iter().all(|scope| scope == "read")
    }
}

pub(crate) struct TestBinding {
    pub(crate) response: Payload,
}

impl ClientBinding for TestBinding {
    fn supports(&self, form: &Form, operation: Operation) -> bool {
        form.href.as_str().starts_with("test://")
            && matches!(
                operation,
                Operation::ReadProperty
                    | Operation::WriteProperty
                    | Operation::InvokeAction
                    | Operation::SubscribeEvent
                    | Operation::ObserveProperty
            )
    }

    fn invoke(&self, request: BindingRequest) -> CoreResult<InteractionOutput> {
        match (request.target, request.operation) {
            (AffordanceTarget::Property(_), Operation::ReadProperty) => {
                Ok(InteractionOutput::with_payload(self.response.clone()))
            }
            (AffordanceTarget::Property(name), Operation::WriteProperty)
                if name.as_ref() == "status" =>
            {
                assert_eq!(request.input.payload.unwrap().body.as_ref(), b"off");
                Ok(InteractionOutput::empty())
            }
            (AffordanceTarget::Property(_), Operation::WriteProperty) => {
                Ok(InteractionOutput::empty())
            }
            (AffordanceTarget::Action(name), Operation::InvokeAction)
                if name.as_ref() == "echo" =>
            {
                Ok(InteractionOutput {
                    payload: request.input.payload,
                })
            }
            (AffordanceTarget::Event(_), Operation::SubscribeEvent) => {
                // Fallback for one-shot invoke path; streaming path uses subscribe().
                Ok(InteractionOutput::with_payload(Payload::new(
                    b"subscribed".to_vec(),
                    "text/plain",
                )))
            }
            _ => panic!("unexpected binding request"),
        }
    }

    fn subscribe(
        &self,
        request: BindingRequest,
    ) -> CoreResult<(clinkz_wot_core::Subscription, Box<dyn SubscriptionGuard>)> {
        match (request.target, request.operation) {
            (AffordanceTarget::Event(name), Operation::SubscribeEvent)
                if name.as_ref() == "startup" =>
            {
                let (sender, subscription) = clinkz_wot_core::Subscription::channel(0);
                // Push an initial sample simulating a remote event delivery.
                sender.push(Payload::new(b"subscribed".to_vec(), "text/plain"));
                Ok((subscription, Box::new(NoopGuard)))
            }
            (AffordanceTarget::Property(name), Operation::ObserveProperty)
                if name.as_ref() == "status" =>
            {
                let (sender, subscription) = clinkz_wot_core::Subscription::channel(0);
                sender.push(self.response.clone());
                Ok((subscription, Box::new(NoopGuard)))
            }
            _ => panic!("unexpected subscribe request"),
        }
    }
}

/// No-op subscription guard for test bindings.
struct NoopGuard;

impl SubscriptionGuard for NoopGuard {
    fn close(self: Box<Self>) {}
}

pub(crate) struct AuthenticatedReadBinding;

impl ClientBinding for AuthenticatedReadBinding {
    fn supports(&self, form: &Form, operation: Operation) -> bool {
        form.href.as_str().starts_with("test://") && operation == Operation::ReadProperty
    }

    fn invoke(&self, request: BindingRequest) -> CoreResult<InteractionOutput> {
        assert_eq!(
            request
                .input
                .security_metadata
                .get("auth")
                .map(String::as_str),
            Some("ok")
        );
        Ok(InteractionOutput::with_payload(Payload::new(
            b"secure-remote".to_vec(),
            "text/plain",
        )))
    }
}

pub(crate) struct CountingUnsupportedBinding {
    pub(crate) supports_calls: Arc<AtomicUsize>,
}

impl ClientBinding for CountingUnsupportedBinding {
    fn supports(&self, _form: &Form, _operation: Operation) -> bool {
        self.supports_calls.fetch_add(1, Ordering::Relaxed);
        false
    }

    fn invoke(&self, _request: BindingRequest) -> CoreResult<InteractionOutput> {
        panic!("unsupported test binding should not be invoked")
    }
}

pub(crate) struct CountingHrefBinding {
    pub(crate) supports_calls: Arc<AtomicUsize>,
}

impl ClientBinding for CountingHrefBinding {
    fn supports(&self, form: &Form, operation: Operation) -> bool {
        self.supports_calls.fetch_add(1, Ordering::Relaxed);
        form.href.as_str().starts_with("test://") && operation == Operation::ReadProperty
    }

    fn invoke(&self, request: BindingRequest) -> CoreResult<InteractionOutput> {
        Ok(InteractionOutput::with_payload(Payload::new(
            request.form.href.as_str().as_bytes().to_vec(),
            "text/plain",
        )))
    }
}

#[allow(dead_code)]
pub(crate) struct TestForms {
    pub(crate) read_property: Form,
    pub(crate) write_property: Form,
    pub(crate) invoke_action: Form,
    pub(crate) subscribe_event: Form,
}

#[cfg(feature = "test-zenoh")]
#[allow(dead_code)]
#[derive(Default)]
pub(crate) struct ServientZenohTransport;

#[cfg(feature = "test-zenoh")]
impl ZenohTransport for ServientZenohTransport {
    fn execute(&self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput> {
        match (request.plan.kind, request.plan.key_expr.as_str()) {
            (ZenohOperationKind::Query, "clinkz/things/lamp/properties/status") => Ok(
                InteractionOutput::with_payload(Payload::new(b"zenoh-on".to_vec(), "text/plain")),
            ),
            (ZenohOperationKind::Put, "clinkz/things/lamp/properties/status") => {
                assert_eq!(
                    request
                        .payload
                        .as_ref()
                        .map(|payload| payload.body.as_ref()),
                    Some(&b"zenoh-off"[..])
                );
                Ok(InteractionOutput::empty())
            }
            (ZenohOperationKind::RequestReply, "clinkz/things/lamp/actions/echo") => {
                Ok(InteractionOutput {
                    payload: request.payload,
                })
            }
            (ZenohOperationKind::Subscribe, "clinkz/things/lamp/events/startup") => {
                Ok(InteractionOutput::with_payload(Payload::new(
                    b"zenoh-subscribed".to_vec(),
                    "text/plain",
                )))
            }
            _ => panic!("unexpected zenoh transport request: {:?}", request),
        }
    }

    fn open_subscription(
        &self,
        request: ZenohTransportRequest,
    ) -> CoreResult<(Subscription, Box<dyn SubscriptionGuard>)> {
        match (request.plan.kind, request.plan.key_expr.as_str()) {
            (ZenohOperationKind::Subscribe, "clinkz/things/lamp/events/startup") => {
                let (sender, subscription) = Subscription::channel(0);
                sender.push(Payload::new(b"zenoh-subscribed".to_vec(), "text/plain"));
                Ok((subscription, Box::new(NoopZenohSubscriptionGuard)))
            }
            _ => panic!("unexpected zenoh subscription request: {:?}", request),
        }
    }
}

struct NoopZenohSubscriptionGuard;

impl SubscriptionGuard for NoopZenohSubscriptionGuard {
    fn close(self: Box<Self>) {}
}

#[cfg(feature = "test-zenoh")]
#[allow(dead_code)]
#[derive(Default)]
pub(crate) struct CountingServientZenohTransport {
    pub(crate) calls: Cell<usize>,
}

#[cfg(feature = "test-zenoh")]
impl ZenohTransport for CountingServientZenohTransport {
    fn execute(&self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput> {
        let count = self.calls.get().saturating_add(1);
        self.calls.set(count);
        match (request.plan.kind, request.plan.key_expr.as_str()) {
            (ZenohOperationKind::Query, "clinkz/things/lamp/properties/status") => {
                Ok(InteractionOutput::with_payload(Payload::new(
                    format!("zenoh-read-{}", count).into_bytes(),
                    "text/plain",
                )))
            }
            (ZenohOperationKind::Put, "clinkz/things/lamp/properties/status") => {
                assert_eq!(
                    request
                        .payload
                        .as_ref()
                        .map(|payload| payload.body.as_ref()),
                    Some(&b"zenoh-off"[..])
                );
                Ok(InteractionOutput::empty())
            }
            _ => panic!("unexpected shared zenoh transport request: {:?}", request),
        }
    }
}

pub(crate) fn thing(id: &str, title: &str) -> (Thing, TestForms) {
    let json_read_property = Form::read_property("other://things/lamp/properties/status")
        .content_type("application/json")
        .build()
        .unwrap();
    let read_property = Form::read_property("test://things/lamp/properties/status")
        .content_type("text/plain")
        .build()
        .unwrap();
    let write_property = Form::write_property("test://things/lamp/properties/status")
        .content_type("text/plain")
        .build()
        .unwrap();
    let observe_property = Form::builder("test://things/lamp/properties/status")
        .observe_property()
        .content_type("text/plain")
        .build()
        .unwrap();
    let invoke_action = Form::invoke_action("test://things/lamp/actions/echo")
        .content_type("text/plain")
        .build()
        .unwrap();
    let subscribe_event = Form::subscribe_event("test://things/lamp/events/startup")
        .content_type("text/plain")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .forms([
            json_read_property.clone(),
            read_property.clone(),
            write_property.clone(),
            observe_property,
        ])
        .build()
        .unwrap();
    let action = ActionAffordance::builder()
        .form(invoke_action.clone())
        .build()
        .unwrap();
    let event = EventAffordance::builder()
        .form(subscribe_event.clone())
        .build()
        .unwrap();
    let thing = Thing::builder(title)
        .id(id)
        .nosec()
        .property("status", property)
        .action("echo", action)
        .event("startup", event)
        .build()
        .unwrap();

    (
        thing,
        TestForms {
            read_property,
            write_property,
            invoke_action,
            subscribe_event,
        },
    )
}

pub(crate) fn secure_thing(id: &str, title: &str) -> (Thing, Form) {
    let read_property = Form::read_property("test://things/lamp/properties/status")
        .content_type("text/plain")
        .security(["token"])
        .scopes(["read"])
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(read_property.clone())
        .build()
        .unwrap();
    let thing = Thing::builder(title)
        .id(id)
        .security_named("token", SecurityScheme::bearer("Authorization"))
        .property("status", property)
        .build()
        .unwrap();

    (thing, read_property)
}

#[cfg(feature = "test-zenoh")]
#[allow(dead_code)]
pub(crate) fn zenoh_thing(id: &str, title: &str) -> Thing {
    let read_property = Form::read_property("zenoh://clinkz/things/lamp/properties/status")
        .content_type("text/plain")
        .build()
        .unwrap();
    let write_property = Form::write_property("zenoh://clinkz/things/lamp/properties/status")
        .content_type("text/plain")
        .build()
        .unwrap();
    let invoke_action = Form::invoke_action("zenoh://clinkz/things/lamp/actions/echo")
        .content_type("text/plain")
        .build()
        .unwrap();
    let subscribe_event = Form::subscribe_event("zenoh://clinkz/things/lamp/events/startup")
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

    Thing::builder(title)
        .id(id)
        .nosec()
        .property("status", property)
        .action("echo", action)
        .event("startup", event)
        .build()
        .unwrap()
}

/// Fake [`ServerBinding`] for testing the sync driving layer and expose/destroy
/// coordination (baseline §4 / §10).
#[derive(Default)]
pub(crate) struct FakeServerBinding {
    pub(crate) pending_requests: Mutex<VecDeque<InboundRequest>>,
    pub(crate) responses: Mutex<Vec<InboundResponse>>,
    pub(crate) registered_things: Mutex<Vec<String>>,
    pub(crate) unregistered_things: Mutex<Vec<String>>,
    pub(crate) registered_affordances: Mutex<Vec<String>>,
    pub(crate) unregistered_affordances: Mutex<Vec<String>>,
    pub(crate) route_registration_fails: bool,
}

impl FakeServerBinding {
    pub(crate) fn enqueue(&self, request: InboundRequest) {
        self.pending_requests.lock().unwrap().push_back(request);
    }

    pub(crate) fn take_responses(&self) -> Vec<InboundResponse> {
        std::mem::take(&mut *self.responses.lock().unwrap())
    }
}

impl ServerBindingTrait for FakeServerBinding {
    fn poll_accept_sync(&self) -> Option<InboundRequest> {
        self.pending_requests.lock().unwrap().pop_front()
    }

    fn send_response(&self, response: InboundResponse) {
        self.responses.lock().unwrap().push(response);
    }

    fn register_thing(&self, thing_id: &str, _td: &Thing) -> Result<(), String> {
        if self.route_registration_fails {
            return Err(format!("route registration failed for '{}'", thing_id));
        }
        self.registered_things
            .lock()
            .unwrap()
            .push(thing_id.to_owned());
        Ok(())
    }

    fn unregister_thing(&self, thing_id: &str) {
        self.unregistered_things
            .lock()
            .unwrap()
            .push(thing_id.to_owned());
    }
}
