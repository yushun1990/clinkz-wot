//! Integration tests for the rewritten core interaction surface (P0).
//!
//! - Sync [`ExposedThing`] dispatch (default features): handler
//!   registration + read/write/invoke/subscribe round-trip.
//! - Consumed form selection + async dispatch (`async` feature): exercises
//!   [`ConsumedThing`] and the async [`ClientBinding`] path.
//! - Error mapping.

#![cfg(test)]

use std::sync::Arc;

use clinkz_wot_core::{
    AffordanceKind, AffordanceTarget, CoreError, CoreResult, ExposedThing, InteractionInput,
    InteractionOutput, InteractionStatus, Payload,
};
use clinkz_wot_td::{
    affordance::{ActionAffordance, EventAffordance, InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    data_type::Operation,
    form::Form,
    thing::Thing,
};

// ---------------------------------------------------------------------------
// Test handlers (sync).
// ---------------------------------------------------------------------------

struct StoredRead {
    value: Arc<std::sync::Mutex<Payload>>,
}

impl clinkz_wot_core::PropertyReadHandler for StoredRead {
    fn read(&self, _input: &InteractionInput) -> CoreResult<InteractionOutput> {
        Ok(InteractionOutput::with_data(
            self.value.lock().unwrap().clone(),
        ))
    }
}

struct StoredWrite {
    value: Arc<std::sync::Mutex<Payload>>,
}

impl clinkz_wot_core::PropertyWriteHandler for StoredWrite {
    fn write(&self, input: &mut InteractionInput) -> CoreResult<InteractionOutput> {
        let payload = input
            .data
            .take()
            .ok_or_else(|| CoreError::InvalidInteraction("Missing property payload".into()))?;
        *self.value.lock().unwrap() = payload;
        Ok(InteractionOutput::empty())
    }
}

struct EchoAction;

impl clinkz_wot_core::ActionHandler for EchoAction {
    fn invoke(&self, input: &mut InteractionInput) -> CoreResult<InteractionOutput> {
        Ok(InteractionOutput {
            data: input.data.take(),
            status: InteractionStatus::Ok,
        })
    }
}

struct StartupEvent;

impl clinkz_wot_core::EventSubscribeHandler for StartupEvent {
    fn subscribe(
        &self,
        _input: &InteractionInput,
        push: &mut dyn FnMut(Payload) -> CoreResult<()>,
    ) -> CoreResult<InteractionOutput> {
        push(Payload::new(b"ready".to_vec(), "text/plain"))?;
        Ok(InteractionOutput::empty())
    }
}

// ---------------------------------------------------------------------------
// Thing Description fixtures.
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Sync ExposedThing dispatch (primary inbound path).
// ---------------------------------------------------------------------------

#[test]
fn local_exposed_thing_dispatches_registered_handlers() {
    let mut thing = ExposedThing::new(local_thing_description());
    let shared = Arc::new(std::sync::Mutex::new(Payload::new(
        b"off".to_vec(),
        "text/plain",
    )));
    thing.set_property_read_handler(
        "status",
        StoredRead {
            value: Arc::clone(&shared),
        },
    );
    thing.set_property_write_handler(
        "status",
        StoredWrite {
            value: Arc::clone(&shared),
        },
    );
    thing.set_action_handler("echo", EchoAction);
    thing.set_event_subscribe_handler("startup", StartupEvent);

    let status = thing
        .read_property("status", &InteractionInput::empty())
        .unwrap()
        .data
        .unwrap();
    assert_eq!(status.body.as_ref(), b"off");

    let mut input = InteractionInput::with_data(Payload::new(b"on".to_vec(), "text/plain"));
    thing.write_property("status", &mut input).unwrap();
    let status = thing
        .read_property("status", &InteractionInput::empty())
        .unwrap()
        .data
        .unwrap();
    assert_eq!(status.body.as_ref(), b"on");

    let mut action_input =
        InteractionInput::with_data(Payload::new(b"hello".to_vec(), "text/plain"));
    let action = thing
        .invoke_action("echo", &mut action_input)
        .unwrap()
        .data
        .unwrap();
    assert_eq!(action.body.as_ref(), b"hello");

    let mut received: Vec<Payload> = Vec::new();
    thing
        .subscribe_event("startup", &InteractionInput::empty(), &mut |p| {
            received.push(p);
            Ok(())
        })
        .unwrap();
    assert_eq!(received[0].body.as_ref(), b"ready");
}

#[test]
fn local_exposed_thing_rejects_unknown_affordance_before_dispatch() {
    let mut thing = ExposedThing::new(local_thing_description());
    thing.set_property_read_handler(
        "missing",
        StoredRead {
            value: Arc::new(std::sync::Mutex::new(Payload::new(
                b"value".to_vec(),
                "text/plain",
            ))),
        },
    );

    let err = thing
        .read_property("missing", &InteractionInput::empty())
        .unwrap_err();
    assert_eq!(
        err,
        CoreError::UnknownAffordance {
            kind: AffordanceKind::Property,
            name: "missing".into()
        }
    );
}

#[test]
fn local_exposed_thing_reports_missing_registered_handler() {
    let thing = ExposedThing::new(local_thing_description());
    let err = thing
        .invoke_action("echo", &mut InteractionInput::empty())
        .unwrap_err();
    assert!(matches!(
        err,
        CoreError::MissingHandler {
            target: AffordanceTarget::Action(_),
            operation: Operation::InvokeAction,
        }
    ));
}

#[test]
fn local_thing_affordance_mutation_pre_expose() {
    // LocalThing affordance mutation is retained as a produce-time TD builder
    // (audit F9). The TD affordance set is frozen at expose(); pre-expose
    // mutation is legitimate.
    let mut local = clinkz_wot_core::LocalThing::new(local_thing_description());
    assert!(local.ensure_property_affordance("status").is_ok());
    local
        .add_property(
            "level",
            PropertyAffordance::builder(DataSchema::number())
                .form(Form::read_property("/properties/level").build().unwrap())
                .build()
                .unwrap(),
        )
        .unwrap();
    assert!(local.ensure_property_affordance("level").is_ok());
    local.remove_property("level");
    assert!(local.ensure_property_affordance("level").is_err());
}

#[test]
fn core_error_display_is_english() {
    let err = CoreError::UnsupportedBinding("no matching form".into());
    assert_eq!(err.to_string(), "Unsupported binding: no matching form");
}

#[test]
fn interaction_output_defaults_to_ok_status() {
    let out = InteractionOutput::with_data(Payload::new(b"x".to_vec(), "text/plain"));
    assert_eq!(out.status, InteractionStatus::Ok);
}

// ---------------------------------------------------------------------------
// Consumed form selection + async dispatch (requires `async` feature).
// ---------------------------------------------------------------------------

#[cfg(feature = "async")]
mod consumed_async {
    use super::*;
    use clinkz_wot_core::interaction::InteractionOutput;
    use clinkz_wot_core::{BindingRequest, ClientBinding, ConsumedThing, SubscriptionGuard};
    use std::cell::RefCell;

    struct RecordingBinding {
        content_type: &'static str,
        response: Payload,
    }

    #[async_trait::async_trait]
    impl ClientBinding for RecordingBinding {
        fn supports(&self, form: &Form, operation: Operation) -> bool {
            form.content_type == self.content_type && operation == Operation::ReadProperty
        }

        async fn invoke(&self, request: BindingRequest) -> CoreResult<InteractionOutput> {
            assert!(
                matches!(request.target, AffordanceTarget::Property(ref name) if name.as_ref() == "status")
            );
            assert_eq!(request.operation, Operation::ReadProperty);
            assert_eq!(
                request.thing._metadata.title.as_deref(),
                Some("Remote Lamp")
            );
            Ok(InteractionOutput::with_data(self.response.clone()))
        }
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
        let property =
            PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
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

    #[tokio::test]
    async fn consumed_dispatches_selected_form_to_matching_binding() {
        let (td, read_form) = remote_thing_description();
        let mut thing = ConsumedThing::new(td);
        thing.register_binding(RecordingBinding {
            content_type: "application/octet-stream",
            response: Payload::new(b"on".to_vec(), "text/plain"),
        });

        let output = thing
            .request(
                AffordanceTarget::Property("status".into()),
                Operation::ReadProperty,
                Arc::new(read_form.clone()),
                InteractionInput::empty(),
            )
            .await
            .unwrap();

        assert_eq!(output.data.unwrap().body.as_ref(), b"on");
    }

    #[tokio::test]
    async fn consumed_rejects_form_not_in_affordance() {
        let (td, _valid_form) = remote_thing_description();
        let consumed = ConsumedThing::new(td);
        let foreign_form = Arc::new(
            Form::read_property("wot://other/properties/x")
                .content_type("application/octet-stream")
                .build()
                .unwrap(),
        );
        let err = consumed
            .request(
                AffordanceTarget::Property("status".into()),
                Operation::ReadProperty,
                foreign_form,
                InteractionInput::empty(),
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, CoreError::InvalidInteraction(_)),
            "expected InvalidInteraction for a foreign form, got {err:?}"
        );
    }

    #[tokio::test]
    async fn consumed_rejects_unknown_affordance_before_binding_dispatch() {
        let (td, read_form) = remote_thing_description();
        let mut thing = ConsumedThing::new(td);
        thing.register_binding(RecordingBinding {
            content_type: "application/octet-stream",
            response: Payload::new(b"on".to_vec(), "text/plain"),
        });
        let err = thing
            .request(
                AffordanceTarget::Property("missing".into()),
                Operation::ReadProperty,
                Arc::new(read_form.clone()),
                InteractionInput::empty(),
            )
            .await
            .unwrap_err();
        assert_eq!(
            err,
            CoreError::UnknownAffordance {
                kind: AffordanceKind::Property,
                name: "missing".into()
            }
        );
    }

    #[tokio::test]
    async fn consumed_rejects_operation_not_declared_by_selected_form() {
        let read_form = Form::read_property("wot://thing/properties/status")
            .content_type("application/octet-stream")
            .build()
            .unwrap();
        let property =
            PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
                .form(read_form.clone())
                .build()
                .unwrap();
        let td = Thing::builder("Remote Lamp")
            .nosec()
            .property("status", property)
            .build()
            .unwrap();
        let thing = ConsumedThing::new(td);
        let err = thing
            .request(
                AffordanceTarget::Property("status".into()),
                Operation::WriteProperty,
                Arc::new(read_form.clone()),
                InteractionInput::empty(),
            )
            .await
            .unwrap_err();
        assert_eq!(
            err,
            CoreError::UnsupportedOperation("Form does not support writeproperty".into())
        );
    }

    #[tokio::test]
    async fn consumed_reports_missing_matching_binding() {
        let (td, read_form) = remote_thing_description();
        let thing = ConsumedThing::new(td);
        let err = thing
            .request(
                AffordanceTarget::Property("status".into()),
                Operation::ReadProperty,
                Arc::new(read_form.clone()),
                InteractionInput::empty(),
            )
            .await
            .unwrap_err();
        assert_eq!(
            err,
            CoreError::UnsupportedBinding(
                "No binding supports readproperty for wot://thing/properties/status".into()
            )
        );
    }

    // Suppress unused-import warnings for fixtures only used in some tests.
    #[allow(dead_code)]
    fn _retain(_v: &RefCell<()>) {}
    #[allow(dead_code)]
    fn _guard(_g: Box<dyn SubscriptionGuard>) {}
}

// ---------------------------------------------------------------------------
// Opt-in async handler dispatch round-trip (`async` feature).
// ---------------------------------------------------------------------------

#[cfg(feature = "async")]
mod async_dispatch {
    use super::*;
    use clinkz_wot_core::{AsyncActionHandler, AsyncPropertyReadHandler, ExposedThing};

    struct AsyncEchoRead;

    #[async_trait::async_trait]
    impl AsyncPropertyReadHandler for AsyncEchoRead {
        async fn read(&self, input: &InteractionInput) -> CoreResult<InteractionOutput> {
            Ok(InteractionOutput::with_data(
                input.data.clone().unwrap_or_default(),
            ))
        }
    }

    struct AsyncEchoAction;

    #[async_trait::async_trait]
    impl AsyncActionHandler for AsyncEchoAction {
        async fn invoke(&self, input: &mut InteractionInput) -> CoreResult<InteractionOutput> {
            Ok(InteractionOutput {
                data: input.data.take(),
                status: InteractionStatus::Ok,
            })
        }
    }

    #[tokio::test]
    async fn async_read_handler_dispatches_via_async_path() {
        let mut thing = ExposedThing::new(local_thing_description());
        thing.set_async_property_read_handler("status", AsyncEchoRead);
        let out = thing
            .read_property_async(
                "status",
                &InteractionInput::with_data(Payload::new(b"hi".to_vec(), "text/plain")),
            )
            .await
            .unwrap();
        assert_eq!(out.data.unwrap().body.as_ref(), b"hi");
    }

    #[tokio::test]
    async fn async_action_handler_dispatches_via_async_path() {
        let mut thing = ExposedThing::new(local_thing_description());
        thing.set_async_action_handler("echo", AsyncEchoAction);
        let mut input = InteractionInput::with_data(Payload::new(b"yo".to_vec(), "text/plain"));
        let out = thing.invoke_action_async("echo", &mut input).await.unwrap();
        assert_eq!(out.data.unwrap().body.as_ref(), b"yo");
    }

    #[tokio::test]
    async fn sync_handler_runs_inline_via_async_path() {
        // An async dispatch against a sync handler runs the sync handler inline
        // (the std driving loop's inline model).
        let mut thing = ExposedThing::new(local_thing_description());
        thing.set_property_read_handler(
            "status",
            StoredRead {
                value: Arc::new(std::sync::Mutex::new(Payload::new(
                    b"v".to_vec(),
                    "text/plain",
                ))),
            },
        );
        let out = thing
            .read_property_async("status", &InteractionInput::empty())
            .await
            .unwrap();
        assert_eq!(out.data.unwrap().body.as_ref(), b"v");
    }
}
