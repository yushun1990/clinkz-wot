use clinkz_wot_core::{
    ActionHandler, AffordanceTarget, BindingRequest, ConsumedThing, CoreResult, EventHandler,
    EventSink, InteractionInput, InteractionOutput, LocalThing, Payload, PropertyHandler,
    ProtocolBinding,
};
use clinkz_wot_servient::{Servient, ServientError};
use clinkz_wot_td::{
    affordance::{ActionAffordance, EventAffordance, InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    data_type::Operation,
    form::Form,
    thing::Thing,
};

struct StatusProperty {
    value: Payload,
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

struct EchoAction;

impl ActionHandler for EchoAction {
    fn invoke(&mut self, input: InteractionInput) -> CoreResult<InteractionOutput> {
        Ok(InteractionOutput {
            payload: input.payload,
        })
    }
}

struct StartupEvent;

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
struct CollectSink {
    payloads: Vec<Payload>,
}

impl EventSink for CollectSink {
    fn emit(&mut self, payload: Payload) -> CoreResult<()> {
        self.payloads.push(payload);
        Ok(())
    }
}

struct TestBinding {
    response: Payload,
}

impl ProtocolBinding for TestBinding {
    fn supports(&self, form: &Form, operation: Operation) -> bool {
        form.href.as_str().starts_with("test://") && operation == Operation::ReadProperty
    }

    fn invoke(&mut self, request: BindingRequest<'_>) -> CoreResult<InteractionOutput> {
        assert!(matches!(
            request.target,
            AffordanceTarget::Property("status")
        ));
        Ok(InteractionOutput::with_payload(self.response.clone()))
    }
}

fn thing(id: &str, title: &str) -> (Thing, Form) {
    let form = Form::read_property("test://things/lamp/properties/status")
        .content_type("text/plain")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(form.clone())
        .build()
        .unwrap();
    let action = ActionAffordance::builder()
        .form(
            Form::invoke_action("test://things/lamp/actions/echo")
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let event = EventAffordance::builder()
        .form(
            Form::subscribe_event("test://things/lamp/events/startup")
                .build()
                .unwrap(),
        )
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

    (thing, form)
}

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
    servient.start().unwrap();
    servient.expose(local).unwrap();

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
    let (td, form) = thing("urn:thing:remote-lamp", "Remote Lamp");
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
            &form,
            InteractionInput::empty(),
        )
        .unwrap();

    assert_eq!(output.payload.unwrap().body, b"on");
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
