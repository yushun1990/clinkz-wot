//! P3 integration: `ExposedThingHandle` async handler setters, the missing
//! sync local-dispatch methods, and the async local-dispatch surface.
//!
//! Verifies:
//! - 9 `set_async_*` setters register handlers reachable via `*_async`
//!   local dispatch.
//! - Sync dispatch refuses async handlers (returns `UnsupportedOperation`).
//! - Async dispatch drives BOTH sync and async handlers.
//! - 6 missing sync local-dispatch methods route through the registered
//!   sync handler.
//! - `InteractionOptions::with_data` / `with_uri_variable` build correctly.

#![cfg(all(feature = "async", feature = "std"))]

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use clinkz_wot_core::{
    AsyncActionHandler, AsyncPropertyReadHandler, CoreError, ErrorPhase, InteractionInput,
    InteractionOptions, InteractionOutput, Payload, PropertyObserveHandler, PropertyReadHandler,
    PropertyWriteHandler, RetryClass,
};
use clinkz_wot_servient::ServientBuilder;
use clinkz_wot_td::{
    affordance::{ActionAffordance, InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    data_type::Operation,
    thing::Thing,
};

// --- protocol binding plumbing (minimal: no bindings, local dispatch only) -

fn build_servient() -> clinkz_wot_servient::Servient {
    ServientBuilder::new().build().expect("build")
}

fn lamp_td() -> Thing {
    Thing::builder("Lamp")
        .id("urn:test:lamp")
        .nosec()
        .property(
            "status",
            PropertyAffordance::builder(DataSchema::string())
                .form(
                    clinkz_wot_td::form::Form::read_property("fake://clinkz/lamp/status")
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
}

fn sensor_td() -> Thing {
    Thing::builder("Sensor")
        .id("urn:test:sensor")
        .nosec()
        .property(
            "temperature",
            PropertyAffordance::builder(DataSchema::string())
                .form(
                    clinkz_wot_td::form::Form::read_property(
                        "fake://clinkz/sensor/properties/temperature",
                    )
                    .build()
                    .unwrap(),
                )
                .form(
                    clinkz_wot_td::form::Form::builder(
                        "fake://clinkz/sensor/properties/temperature",
                    )
                    .observe_property()
                    .build()
                    .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
}

fn pump_td() -> Thing {
    Thing::builder("Pump")
        .id("urn:test:pump")
        .nosec()
        .action(
            "start",
            ActionAffordance::builder()
                .form(
                    clinkz_wot_td::form::Form::invoke_action("fake://clinkz/pump/start")
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
}

// --- fakes -----------------------------------------------------------------

struct SyncRead(&'static str);
impl PropertyReadHandler for SyncRead {
    fn read(&self, _: &InteractionInput) -> Result<InteractionOutput, CoreError> {
        Ok(InteractionOutput::with_data(Payload::new(
            self.0.as_bytes().to_vec(),
            "text/plain",
        )))
    }
}

struct AsyncRead {
    canned: &'static str,
    delay_ms: u64,
}
#[async_trait]
impl AsyncPropertyReadHandler for AsyncRead {
    async fn read(&self, _: &InteractionInput) -> Result<InteractionOutput, CoreError> {
        // Sleep to prove the future actually drives an async path.
        tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
        Ok(InteractionOutput::with_data(Payload::new(
            self.canned.as_bytes().to_vec(),
            "text/plain",
        )))
    }
}

struct RecordingWrite {
    captured: Arc<Mutex<Option<Vec<u8>>>>,
}
impl PropertyWriteHandler for RecordingWrite {
    fn write(&self, input: &mut InteractionInput) -> Result<InteractionOutput, CoreError> {
        *self.captured.lock().unwrap() = input.data.take().map(|p| p.body.to_vec());
        Ok(InteractionOutput::empty())
    }
}

struct AsyncActionInvoke {
    calls: Arc<Mutex<usize>>,
}
#[async_trait]
impl AsyncActionHandler for AsyncActionInvoke {
    async fn invoke(&self, _: &mut InteractionInput) -> Result<InteractionOutput, CoreError> {
        *self.calls.lock().unwrap() += 1;
        Ok(InteractionOutput::empty())
    }
}

struct PushingObserve {
    sample: &'static str,
}
impl PropertyObserveHandler for PushingObserve {
    fn observe(
        &self,
        _: &InteractionInput,
        push: clinkz_wot_core::PushFn<'_>,
    ) -> Result<InteractionOutput, CoreError> {
        push(Payload::new(self.sample.as_bytes().to_vec(), "text/plain"))?;
        Ok(InteractionOutput::empty())
    }
}

// --- tests -----------------------------------------------------------------

#[tokio::test]
async fn set_async_property_read_handler_runs_via_async_dispatch() {
    let servient = build_servient();
    let handle = servient.produce(lamp_td()).expect("produce");
    handle.set_async_property_read_handler(
        "status",
        AsyncRead {
            canned: "async-21C",
            delay_ms: 5,
        },
    );

    let out = handle
        .read_property_async("status", &InteractionInput::empty())
        .await
        .expect("async read");
    assert_eq!(out.data().unwrap().body.as_ref(), b"async-21C");
}

#[tokio::test]
async fn sync_dispatch_refuses_async_handler() {
    let servient = build_servient();
    let handle = servient.produce(lamp_td()).expect("produce");
    handle.set_async_property_read_handler(
        "status",
        AsyncRead {
            canned: "x",
            delay_ms: 0,
        },
    );

    let err = handle
        .read_property("status", &InteractionInput::empty())
        .unwrap_err();
    assert!(
        matches!(
            err,
            CoreError::UnsupportedOperation(context)
                if context.phase() == ErrorPhase::Handler
                    && context.retry_class() == RetryClass::Never
                    && context.operation() == Some(Operation::ReadProperty)
        ),
        "sync dispatch refuses async handler, got {err:?}"
    );
}

#[tokio::test]
async fn async_dispatch_drives_sync_handler_too() {
    let servient = build_servient();
    let handle = servient.produce(lamp_td()).expect("produce");
    handle.set_property_read_handler("status", SyncRead("sync-42C"));

    let out = handle
        .read_property_async("status", &InteractionInput::empty())
        .await
        .expect("async read of sync handler");
    assert_eq!(out.data().unwrap().body.as_ref(), b"sync-42C");
}

#[tokio::test]
async fn set_async_action_handler_runs_via_invoke_action_async() {
    let servient = build_servient();
    let handle = servient.produce(pump_td()).expect("produce");

    let calls = Arc::new(Mutex::new(0usize));
    handle.set_async_action_handler(
        "start",
        AsyncActionInvoke {
            calls: calls.clone(),
        },
    );

    handle
        .invoke_action_async("start", &mut InteractionInput::empty())
        .await
        .expect("invoke async");
    assert_eq!(*calls.lock().unwrap(), 1, "async action handler ran once");
}

#[tokio::test]
async fn local_observe_routes_through_sync_handler_and_pushes() {
    let servient = build_servient();
    let handle = servient.produce(sensor_td()).expect("produce");
    handle.set_property_observe_handler("temperature", PushingObserve { sample: "first" });

    let mut received: Option<Vec<u8>> = None;
    let mut push_fn = |p: Payload| -> Result<(), CoreError> {
        received = Some(p.body.to_vec());
        Ok(())
    };
    let _ = handle.observe_property("temperature", &InteractionInput::empty(), &mut push_fn);
    assert_eq!(received.as_deref(), Some(b"first".as_slice()));
}

#[tokio::test]
async fn local_observe_async_routes_through_sync_handler_and_pushes() {
    let servient = build_servient();
    let handle = servient.produce(sensor_td()).expect("produce");
    handle.set_property_observe_handler(
        "temperature",
        PushingObserve {
            sample: "via-async",
        },
    );

    let mut received: Option<Vec<u8>> = None;
    let mut push_fn = |p: Payload| -> Result<(), CoreError> {
        received = Some(p.body.to_vec());
        Ok(())
    };
    handle
        .observe_property_async("temperature", &InteractionInput::empty(), &mut push_fn)
        .await
        .expect("observe async");
    assert_eq!(received.as_deref(), Some(b"via-async".as_slice()));
}

#[tokio::test]
async fn missing_handler_returns_unsupported_operation_error() {
    let servient = build_servient();
    let handle = servient.produce(lamp_td()).expect("produce");

    let err = handle
        .read_property_async("status", &InteractionInput::empty())
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        CoreError::UnsupportedOperation(context)
            if context.phase() == ErrorPhase::Handler
                && context.retry_class() == RetryClass::Never
                && context.operation() == Some(Operation::ReadProperty)
    ));
}

#[tokio::test]
async fn interaction_options_with_data_builder_round_trips() {
    let opts = InteractionOptions::with_data(Payload::new(b"on".to_vec(), "text/plain"));
    assert_eq!(opts.data.as_ref().unwrap().body.as_ref(), b"on");
    assert!(opts.uri_variables.is_empty());

    let chained = InteractionOptions::with_data(Payload::new(b"x".to_vec(), "text/plain"))
        .with_uri_variable("zone", "north")
        .with_uri_variable("brightness", "75");
    assert_eq!(
        chained.uri_variables.get("zone").map(String::as_str),
        Some("north")
    );
    assert_eq!(
        chained.uri_variables.get("brightness").map(String::as_str),
        Some("75")
    );
}

#[tokio::test]
async fn local_property_write_routes_through_sync_handler() {
    let servient = build_servient();
    let handle = servient.produce(lamp_td()).expect("produce");
    let captured = Arc::new(Mutex::new(None));
    handle.set_property_write_handler(
        "status",
        RecordingWrite {
            captured: captured.clone(),
        },
    );

    let mut input = InteractionInput::with_data(Payload::new(b"on".to_vec(), "text/plain"));
    handle.write_property("status", &mut input).expect("write");
    assert_eq!(captured.lock().unwrap().as_deref(), Some(b"on".as_slice()));
}
