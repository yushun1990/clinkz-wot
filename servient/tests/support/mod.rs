use std::{borrow::Cow, cell::Cell, rc::Rc};

use clinkz_wot_core::{
    ActionHandler, AffordanceTarget, BindingRequest, CodecInput, CoreResult, EventHandler,
    EventSink, InteractionInput, InteractionOutput, LocalThing, Payload, PayloadCodec,
    PropertyHandler, ProtocolBinding, SecurityContext, SecurityProvider, TransportRequest,
};
use clinkz_wot_protocol_bindings_zenoh::{
    ZenohOperationKind, ZenohTransport, ZenohTransportRequest,
};
use clinkz_wot_servient::{
    ConsumedThingCache, ExposedThingRegistry, InMemoryConsumedThingCache,
    InMemoryExposedThingRegistry,
};
use clinkz_wot_td::{
    affordance::{ActionAffordance, EventAffordance, InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    data_type::Operation,
    form::Form,
    security_scheme::SecurityScheme,
    thing::Thing,
};

pub(crate) struct StatusProperty {
    pub(crate) value: Payload,
}

impl PropertyHandler for StatusProperty {
    fn read(&mut self, _input: InteractionInput) -> CoreResult<InteractionOutput> {
        Ok(InteractionOutput::with_payload(self.value.clone()))
    }

    fn write(&mut self, input: InteractionInput) -> CoreResult<InteractionOutput> {
        self.value = input.payload.expect("test write payload");
        Ok(InteractionOutput::empty())
    }
}

pub(crate) struct EchoAction;

impl ActionHandler for EchoAction {
    fn invoke(&mut self, input: InteractionInput) -> CoreResult<InteractionOutput> {
        Ok(InteractionOutput {
            payload: input.payload,
        })
    }
}

pub(crate) struct StartupEvent;

impl EventHandler for StartupEvent {
    fn subscribe(
        &mut self,
        _input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> CoreResult<InteractionOutput> {
        sink.emit(Payload::new(b"ready".to_vec(), "text/plain"))?;
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
        Ok(payload.body.clone())
    }
}

pub(crate) struct AuthenticatedStatusProperty;

impl PropertyHandler for AuthenticatedStatusProperty {
    fn read(&mut self, input: InteractionInput) -> CoreResult<InteractionOutput> {
        assert_eq!(input.parameters.get("auth").map(String::as_str), Some("ok"));
        Ok(InteractionOutput::with_payload(Payload::new(
            b"secure-local".to_vec(),
            "text/plain",
        )))
    }

    fn write(&mut self, _input: InteractionInput) -> CoreResult<InteractionOutput> {
        Ok(InteractionOutput::empty())
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
        &mut self,
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

impl ProtocolBinding for TestBinding {
    fn supports(&self, form: &Form, operation: Operation) -> bool {
        form.href.as_str().starts_with("test://")
            && matches!(
                operation,
                Operation::ReadProperty
                    | Operation::WriteProperty
                    | Operation::InvokeAction
                    | Operation::SubscribeEvent
            )
    }

    fn invoke(&mut self, request: BindingRequest<'_>) -> CoreResult<InteractionOutput> {
        match (request.target, request.operation) {
            (AffordanceTarget::Property("status"), Operation::ReadProperty) => {
                Ok(InteractionOutput::with_payload(self.response.clone()))
            }
            (AffordanceTarget::Property("status"), Operation::WriteProperty) => {
                assert_eq!(request.input.payload.unwrap().body, b"off");
                Ok(InteractionOutput::empty())
            }
            (AffordanceTarget::Action("echo"), Operation::InvokeAction) => Ok(InteractionOutput {
                payload: request.input.payload,
            }),
            (AffordanceTarget::Event("startup"), Operation::SubscribeEvent) => Ok(
                InteractionOutput::with_payload(Payload::new(b"subscribed".to_vec(), "text/plain")),
            ),
            _ => panic!("unexpected binding request"),
        }
    }
}

pub(crate) struct HrefBinding;

impl ProtocolBinding for HrefBinding {
    fn supports(&self, form: &Form, operation: Operation) -> bool {
        form.href.as_str().starts_with("test://") && operation == Operation::ReadProperty
    }

    fn invoke(&mut self, request: BindingRequest<'_>) -> CoreResult<InteractionOutput> {
        Ok(InteractionOutput::with_payload(Payload::new(
            request.form.href.as_str().as_bytes().to_vec(),
            "text/plain",
        )))
    }
}

pub(crate) struct AuthenticatedReadBinding;

impl ProtocolBinding for AuthenticatedReadBinding {
    fn supports(&self, form: &Form, operation: Operation) -> bool {
        form.href.as_str().starts_with("test://") && operation == Operation::ReadProperty
    }

    fn invoke(&mut self, request: BindingRequest<'_>) -> CoreResult<InteractionOutput> {
        assert_eq!(
            request.input.parameters.get("auth").map(String::as_str),
            Some("ok")
        );
        Ok(InteractionOutput::with_payload(Payload::new(
            b"secure-remote".to_vec(),
            "text/plain",
        )))
    }
}

pub(crate) struct CountingUnsupportedBinding {
    pub(crate) supports_calls: Rc<std::cell::RefCell<usize>>,
}

impl ProtocolBinding for CountingUnsupportedBinding {
    fn supports(&self, _form: &Form, _operation: Operation) -> bool {
        *self.supports_calls.borrow_mut() += 1;
        false
    }

    fn invoke(&mut self, _request: BindingRequest<'_>) -> CoreResult<InteractionOutput> {
        panic!("unsupported test binding should not be invoked")
    }
}

pub(crate) struct CountingHrefBinding {
    pub(crate) supports_calls: Rc<std::cell::RefCell<usize>>,
}

impl ProtocolBinding for CountingHrefBinding {
    fn supports(&self, form: &Form, operation: Operation) -> bool {
        *self.supports_calls.borrow_mut() += 1;
        form.href.as_str().starts_with("test://") && operation == Operation::ReadProperty
    }

    fn invoke(&mut self, request: BindingRequest<'_>) -> CoreResult<InteractionOutput> {
        Ok(InteractionOutput::with_payload(Payload::new(
            request.form.href.as_str().as_bytes().to_vec(),
            "text/plain",
        )))
    }
}

pub(crate) struct TestForms {
    pub(crate) read_property: Form,
    pub(crate) write_property: Form,
    pub(crate) invoke_action: Form,
    pub(crate) subscribe_event: Form,
}

#[derive(Default)]
pub(crate) struct ServientZenohTransport;

impl ZenohTransport for ServientZenohTransport {
    fn execute(&mut self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput> {
        match (request.plan.kind, request.plan.key_expr.as_str()) {
            (ZenohOperationKind::Query, "clinkz/things/lamp/properties/status") => Ok(
                InteractionOutput::with_payload(Payload::new(b"zenoh-on".to_vec(), "text/plain")),
            ),
            (ZenohOperationKind::Put, "clinkz/things/lamp/properties/status") => {
                assert_eq!(
                    request
                        .payload
                        .as_ref()
                        .map(|payload| payload.body.as_slice()),
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
}

#[derive(Default)]
pub(crate) struct TestExposedRegistry {
    pub(crate) inner: InMemoryExposedThingRegistry,
    pub(crate) inserted: usize,
    pub(crate) removed: usize,
}

impl ExposedThingRegistry for TestExposedRegistry {
    fn contains_id(&self, id: &str) -> bool {
        self.inner.contains_id(id)
    }

    fn insert(&mut self, id: String, thing: LocalThing) -> Option<LocalThing> {
        self.inserted += 1;
        self.inner.insert(id, thing)
    }

    fn remove(&mut self, id: &str) -> Option<LocalThing> {
        self.removed += 1;
        self.inner.remove(id)
    }

    fn get_mut(&mut self, id: &str) -> Option<&mut LocalThing> {
        self.inner.get_mut(id)
    }
}

#[derive(Default)]
pub(crate) struct TestConsumedCache {
    pub(crate) inner: InMemoryConsumedThingCache,
    pub(crate) inserted: usize,
    pub(crate) removed: usize,
}

impl ConsumedThingCache for TestConsumedCache {
    fn get(&self, id: &str) -> Option<Thing> {
        self.inner.get(id)
    }

    fn insert(&mut self, id: String, thing: Thing) -> Option<Thing> {
        self.inserted += 1;
        self.inner.insert(id, thing)
    }

    fn remove(&mut self, id: &str) -> Option<Thing> {
        self.removed += 1;
        self.inner.remove(id)
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

pub(crate) fn cacheable_thing(id: &str, title: &str) -> (Thing, Form, Form) {
    let first_form = Form::read_property("test://things/lamp/properties/status/first")
        .content_type("text/plain")
        .build()
        .unwrap();
    let cached_form = Form::read_property("test://things/lamp/properties/status/cached")
        .content_type("text/plain")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .forms([first_form.clone(), cached_form.clone()])
        .build()
        .unwrap();
    let thing = Thing::builder(title)
        .id(id)
        .nosec()
        .property("status", property)
        .build()
        .unwrap();

    (thing, first_form, cached_form)
}

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
