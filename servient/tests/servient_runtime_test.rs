use clinkz_wot_core::{
    ActionHandler, AffordanceTarget, BindingRequest, ConsumedThing, CoreResult, EventHandler,
    EventSink, InteractionInput, InteractionOutput, LocalThing, Payload, PropertyHandler,
    ProtocolBinding,
};
use clinkz_wot_protocol_bindings::{BindingCoreError, FormSelectionCriteria};
use clinkz_wot_servient::{
    ExposedThingRegistry, InMemoryExposedThingRegistry, Servient, ServientError,
};
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

struct TestForms {
    read_property: Form,
    write_property: Form,
    invoke_action: Form,
    subscribe_event: Form,
}

#[derive(Default)]
struct TestExposedRegistry {
    inner: InMemoryExposedThingRegistry,
    inserted: usize,
    removed: usize,
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

fn thing(id: &str, title: &str) -> (Thing, TestForms) {
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
