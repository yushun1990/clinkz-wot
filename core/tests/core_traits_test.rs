use std::{borrow::Cow, cell::RefCell, sync::Arc};

use clinkz_wot_core::{
    ActionHandler, AffordanceTarget, BindingRequest, BoundConsumedThing, ClientBinding, CodecInput,
    ConsumedThing, CoreError, CoreResult, EventSink, EventSubscribeHandler, ExposedThing,
    InteractionInput, InteractionOutput, LocalThing, Payload, PayloadCodec, PropertyReadHandler,
    PropertyWriteHandler, TransportAdapter, TransportRequest, TransportResponse,
};
use clinkz_wot_td::{
    affordance::{ActionAffordance, EventAffordance, InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    data_type::Operation,
    form::Form,
    security_scheme::{NoSecurityScheme, SecurityScheme},
    thing::Thing,
    validate::Validate,
};

struct EchoCodec;

impl PayloadCodec for EchoCodec {
    fn content_type(&self) -> Cow<'_, str> {
        "application/octet-stream".into()
    }

    fn encode(&self, input: CodecInput<'_>) -> CoreResult<Payload> {
        Ok(Payload::new(
            input.body.to_vec(),
            self.content_type().into_owned(),
        ))
    }

    fn decode(&self, payload: &Payload) -> CoreResult<Vec<u8>> {
        Ok(payload.body.clone())
    }
}

struct EchoTransport;

impl TransportAdapter for EchoTransport {
    fn exchange(&mut self, request: TransportRequest) -> CoreResult<TransportResponse> {
        Ok(TransportResponse {
            metadata: request.metadata,
            payload: request.payload,
        })
    }
}

struct EchoBinding {
    transport: RefCell<EchoTransport>,
}

impl ClientBinding for EchoBinding {
    fn supports(&self, form: &Form, operation: Operation) -> bool {
        form.content_type == "application/octet-stream" && operation == Operation::InvokeAction
    }

    fn invoke(&self, request: BindingRequest) -> CoreResult<InteractionOutput> {
        let payload = request.input.payload;
        let response = self.transport.borrow_mut().exchange(
            TransportRequest::new(request.form.href.as_str(), "invoke").with_payload(payload),
        )?;
        Ok(InteractionOutput {
            payload: response.payload,
        })
    }
}

struct RecordingBinding {
    content_type: &'static str,
    response: Payload,
}

impl ClientBinding for RecordingBinding {
    fn supports(&self, form: &Form, operation: Operation) -> bool {
        form.content_type == self.content_type && operation == Operation::ReadProperty
    }

    fn invoke(&self, request: BindingRequest) -> CoreResult<InteractionOutput> {
        assert!(matches!(request.target, AffordanceTarget::Property(ref name) if name == "status"));
        assert_eq!(request.operation, Operation::ReadProperty);
        assert_eq!(
            request.thing._metadata.title.as_deref(),
            Some("Remote Lamp")
        );
        Ok(InteractionOutput::with_payload(self.response.clone()))
    }
}

trait RequestPayloadExt {
    fn with_payload(self, payload: Option<Payload>) -> Self;
}

impl RequestPayloadExt for TransportRequest {
    fn with_payload(mut self, payload: Option<Payload>) -> Self {
        self.payload = payload;
        self
    }
}

struct StoredRead {
    value: Arc<std::sync::Mutex<Payload>>,
}

impl PropertyReadHandler for StoredRead {
    fn read(&mut self, _input: InteractionInput) -> CoreResult<InteractionOutput> {
        Ok(InteractionOutput::with_payload(
            self.value.lock().unwrap().clone(),
        ))
    }
}

struct StoredWrite {
    value: Arc<std::sync::Mutex<Payload>>,
}

impl PropertyWriteHandler for StoredWrite {
    fn write(&mut self, input: InteractionInput) -> CoreResult<InteractionOutput> {
        *self.value.lock().unwrap() = input
            .payload
            .ok_or_else(|| CoreError::InvalidInteraction("Missing property payload".into()))?;
        Ok(InteractionOutput::empty())
    }
}

struct EchoAction;

impl ActionHandler for EchoAction {
    fn invoke(&mut self, input: InteractionInput) -> CoreResult<InteractionOutput> {
        Ok(InteractionOutput {
            payload: input.payload,
        })
    }
}

struct StartupEvent;

impl EventSubscribeHandler for StartupEvent {
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
struct CollectSink {
    payloads: Vec<Payload>,
}

impl EventSink for CollectSink {
    fn emit(&mut self, payload: Payload) -> CoreResult<()> {
        self.payloads.push(payload);
        Ok(())
    }
}

fn local_thing_description() -> Thing {
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(
            Form::builder("wot://thing/properties/status")
                .op([Operation::ReadProperty, Operation::WriteProperty])
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let action = ActionAffordance::builder()
        .form(
            Form::builder("wot://thing/actions/echo")
                .op([Operation::InvokeAction])
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let event = EventAffordance::builder()
        .form(
            Form::builder("wot://thing/events/startup")
                .op([Operation::SubscribeEvent])
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    Thing::builder("Local Lamp")
        .nosec()
        .property("status", property)
        .action("echo", action)
        .event("startup", event)
        .build()
        .unwrap()
}

fn remote_thing_description() -> (Thing, Form) {
    let read_form = Form::builder("wot://thing/properties/status")
        .content_type("application/octet-stream")
        .build()
        .unwrap();
    let write_form = Form::write_property("wot://thing/properties/status")
        .content_type("application/json")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .forms([read_form.clone(), write_form])
        .build()
        .unwrap();

    (
        Thing::builder("Remote Lamp")
            .nosec()
            .property("status", property)
            .build()
            .unwrap(),
        read_form,
    )
}

#[test]
fn codec_round_trips_payload_bytes() {
    let codec = EchoCodec;

    let payload = codec
        .encode(CodecInput {
            body: b"hello",
            data_type: None,
        })
        .unwrap();

    assert_eq!(payload.content_type, "application/octet-stream");
    assert_eq!(codec.decode(&payload).unwrap(), b"hello");
}

#[test]
fn binding_invokes_selected_form_without_protocol_assumptions() {
    let form = Form::builder("wot://thing/actions/ping")
        .content_type("application/octet-stream")
        .op([Operation::InvokeAction])
        .build()
        .unwrap();
    let action = clinkz_wot_td::affordance::ActionAffordance::builder()
        .form(form.clone())
        .input(DataSchema::String(DataSchema::string().build()))
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .security(SecurityScheme::NoSec(
            NoSecurityScheme::builder().build().unwrap(),
        ))
        .action("ping", action)
        .build()
        .unwrap();
    thing.validate().unwrap();

    let binding = EchoBinding {
        transport: RefCell::new(EchoTransport),
    };
    assert!(binding.supports(&form, Operation::InvokeAction));

    let output = binding
        .invoke(BindingRequest {
            thing: Arc::new(thing.clone()),
            target: AffordanceTarget::Action("ping".into()),
            operation: Operation::InvokeAction,
            form: Arc::new(form.clone()),
            input: InteractionInput::with_payload(Payload::new(
                b"payload".to_vec(),
                "application/octet-stream",
            )),
        })
        .unwrap();

    assert_eq!(output.payload.unwrap().body, b"payload");
}

#[test]
fn consumed_thing_dispatches_selected_form_to_matching_binding() {
    let (td, read_form) = remote_thing_description();
    let mut thing = BoundConsumedThing::new(td);
    thing.register_binding(RecordingBinding {
        content_type: "application/octet-stream",
        response: Payload::new(b"on".to_vec(), "text/plain"),
    });

    let output = thing
        .request(
            AffordanceTarget::Property("status".into()),
            Operation::ReadProperty,
            &read_form,
            InteractionInput::empty(),
        )
        .unwrap();

    assert_eq!(output.payload.unwrap().body, b"on");
}

#[test]
fn consumed_thing_rejects_unknown_affordance_before_binding_dispatch() {
    let (td, read_form) = remote_thing_description();
    let mut thing = BoundConsumedThing::new(td);
    thing.register_binding(RecordingBinding {
        content_type: "application/octet-stream",
        response: Payload::new(b"on".to_vec(), "text/plain"),
    });

    let err = thing
        .request(
            AffordanceTarget::Property("missing".into()),
            Operation::ReadProperty,
            &read_form,
            InteractionInput::empty(),
        )
        .unwrap_err();

    assert_eq!(
        err,
        CoreError::UnknownAffordance {
            kind: "property",
            name: "missing".into()
        }
    );
}

#[test]
fn consumed_thing_rejects_operation_not_declared_by_selected_form() {
    let read_form = Form::read_property("wot://thing/properties/status")
        .content_type("application/octet-stream")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(read_form.clone())
        .build()
        .unwrap();
    let td = Thing::builder("Remote Lamp")
        .nosec()
        .property("status", property)
        .build()
        .unwrap();
    let mut thing = BoundConsumedThing::new(td);

    let err = thing
        .request(
            AffordanceTarget::Property("status".into()),
            Operation::WriteProperty,
            &read_form,
            InteractionInput::empty(),
        )
        .unwrap_err();

    assert_eq!(
        err,
        CoreError::UnsupportedOperation("Form does not support WriteProperty".into())
    );
}

#[test]
fn consumed_thing_reports_missing_matching_binding() {
    let (td, read_form) = remote_thing_description();
    let mut thing = BoundConsumedThing::new(td);

    let err = thing
        .request(
            AffordanceTarget::Property("status".into()),
            Operation::ReadProperty,
            &read_form,
            InteractionInput::empty(),
        )
        .unwrap_err();

    assert_eq!(
        err,
        CoreError::UnsupportedBinding(
            "No binding supports ReadProperty for wot://thing/properties/status".into()
        )
    );
}

#[test]
fn local_thing_dispatches_registered_handlers() {
    let mut thing = LocalThing::new(local_thing_description());
    let shared = Arc::new(std::sync::Mutex::new(Payload::new(
        b"off".to_vec(),
        "text/plain",
    )));
    thing.register_property_read_handler(
        "status",
        StoredRead {
            value: Arc::clone(&shared),
        },
    );
    thing.register_property_write_handler(
        "status",
        StoredWrite {
            value: Arc::clone(&shared),
        },
    );
    thing.register_action_handler("echo", EchoAction);
    thing.register_event_subscribe_handler("startup", StartupEvent);

    let status = thing
        .read_property("status", InteractionInput::empty())
        .unwrap()
        .payload
        .unwrap();
    assert_eq!(status.body, b"off");

    thing
        .write_property(
            "status",
            InteractionInput::with_payload(Payload::new(b"on".to_vec(), "text/plain")),
        )
        .unwrap();
    let status = thing
        .read_property("status", InteractionInput::empty())
        .unwrap()
        .payload
        .unwrap();
    assert_eq!(status.body, b"on");

    let action = thing
        .invoke_action(
            "echo",
            InteractionInput::with_payload(Payload::new(b"hello".to_vec(), "text/plain")),
        )
        .unwrap()
        .payload
        .unwrap();
    assert_eq!(action.body, b"hello");

    let mut sink = CollectSink::default();
    thing
        .subscribe_event("startup", InteractionInput::empty(), &mut sink)
        .unwrap();
    assert_eq!(sink.payloads[0].body, b"ready");
}

#[test]
fn local_thing_rejects_unknown_affordance_before_dispatch() {
    let mut thing = LocalThing::new(local_thing_description());
    thing.register_property_read_handler(
        "missing",
        StoredRead {
            value: Arc::new(std::sync::Mutex::new(Payload::new(
                b"value".to_vec(),
                "text/plain",
            ))),
        },
    );

    let err = thing
        .read_property("missing", InteractionInput::empty())
        .unwrap_err();

    assert_eq!(
        err,
        CoreError::UnknownAffordance {
            kind: "property",
            name: "missing".into()
        }
    );
}

#[test]
fn local_thing_reports_missing_registered_handler() {
    let mut thing = LocalThing::new(local_thing_description());

    let err = thing
        .invoke_action("echo", InteractionInput::empty())
        .unwrap_err();

    assert_eq!(err, CoreError::MissingHandler);
}

#[test]
fn core_error_display_is_english() {
    let err = CoreError::UnsupportedBinding("no matching form".into());

    assert_eq!(err.to_string(), "Unsupported binding: no matching form");
}
