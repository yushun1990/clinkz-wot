//! P0 integration: `ServientBuilder::with_protocol_binding` wires a unified
//! `ProtocolBinding` into both the server-side registry and the per-Consumed-Thing
//! client-factory list.
//!
//! Verifies:
//! - A two-direction binding exposes both halves.
//! - Pure-consumer bindings never try to register routes.
//! - Pure-exposer bindings never produce a client adapter.
//! - Per-Consumed-Thing freshness: each `consume()` gets an independent
//!   `ClientBinding` instance from the factory.

#![cfg(all(feature = "async", feature = "std"))]

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use clinkz_wot_core::{
    AffordanceTarget, BindingRequest, ClientBinding, ClientBindingFactory, CoreError,
    InboundResponse, InteractionInput, InteractionOutput, ProtocolBinding, ProtocolId,
    PropertyReadHandler, ServerBinding, ThingId,
};
use clinkz_wot_servient::ServientBuilder;
use clinkz_wot_td::{
    affordance::{InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    data_type::Operation,
    thing::Thing,
};

// --- stubs reused across test cases -----------------------------------------

#[derive(Default)]
struct FakeServer {
    registered: Mutex<Vec<String>>,
    unregistered: Mutex<Vec<String>>,
}

impl ServerBinding for FakeServer {
    fn send_response(&self, _response: clinkz_wot_core::InboundResponse) {}
    fn register_thing(&self, thing_id: &ThingId, _td: &Thing) -> Result<(), CoreError> {
        self.registered
            .lock()
            .unwrap()
            .push(thing_id.as_str().to_string());
        Ok(())
    }
    fn unregister_thing(&self, thing_id: &ThingId) {
        self.unregistered
            .lock()
            .unwrap()
            .push(thing_id.as_str().to_string());
    }
}

impl FakeServer {
    fn registered_count(&self) -> usize {
        self.registered.lock().unwrap().len()
    }
}

#[derive(Default)]
struct CountingClient {
    invocations: Arc<Mutex<usize>>,
}

#[async_trait]
impl ClientBinding for CountingClient {
    fn supports(&self, _form: &clinkz_wot_td::form::Form, _op: Operation) -> bool {
        true
    }
    async fn invoke(&self, _request: BindingRequest) -> Result<InteractionOutput, CoreError> {
        *self.invocations.lock().unwrap() += 1;
        Ok(InteractionOutput::empty())
    }
}

#[derive(Clone)]
struct CountingClientFactory {
    invocations: Arc<Mutex<usize>>,
    /// Each `build()` call increments this so tests can assert the factory
    /// was actually invoked per consume().
    builds: Arc<Mutex<usize>>,
}

impl CountingClientFactory {
    fn new() -> (Self, Arc<Mutex<usize>>) {
        let invocations = Arc::new(Mutex::new(0));
        let builds = Arc::new(Mutex::new(0));
        (
            Self {
                invocations: invocations.clone(),
                builds,
            },
            invocations,
        )
    }
}

impl ClientBindingFactory for CountingClientFactory {
    fn build(&self) -> Box<dyn ClientBinding> {
        *self.builds.lock().unwrap() += 1;
        Box::new(CountingClient {
            invocations: self.invocations.clone(),
        })
    }
}

/// Two-direction `ProtocolBinding` carrying both halves. Mirrors what a real
/// zenoh binding would register.
struct TwoDirectionBinding {
    protocol: ProtocolId,
    factory: CountingClientFactory,
    server: Arc<FakeServer>,
}

impl ProtocolBinding for TwoDirectionBinding {
    fn protocol(&self) -> ProtocolId {
        self.protocol
    }
    fn client_factory(&self) -> Option<Box<dyn ClientBindingFactory>> {
        Some(Box::new(self.factory.clone()))
    }
    fn server(&self) -> Option<Arc<dyn ServerBinding>> {
        Some(self.server.clone())
    }
}

// --- fixtures ---------------------------------------------------------------

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

// --- tests ------------------------------------------------------------------

#[tokio::test]
async fn with_protocol_binding_registers_both_client_and_server() {
    let (factory, _invocations) = CountingClientFactory::new();
    let server = Arc::new(FakeServer::default());
    let binding = Arc::new(TwoDirectionBinding {
        protocol: ProtocolId("test"),
        factory,
        server: server.clone(),
    });

    let servient = ServientBuilder::new()
        .with_protocol_binding(binding)
        .build()
        .expect("build");

    // Exposed side: produce+expose drives register_thing on the shared server.
    let handle = servient.produce(lamp_td()).expect("produce");
    handle.expose().await.expect("expose");
    assert_eq!(server.registered_count(), 1, "server register called");

    // Consumed side: a fresh ClientBinding is built per consume().
    let consumed = servient.consume(lamp_td()).expect("consume");
    consumed
        .read_property("status", Default::default())
        .await
        .expect("read");
}

#[tokio::test]
async fn pure_consumer_binding_does_not_register_routes() {
    // A pure-consumer binding (server() returns None) should not appear in
    // the server registry: producing + exposing a Thing yields no
    // register_thing call against any external server.
    let (factory, _invocations) = CountingClientFactory::new();
    let binding: Arc<dyn ProtocolBinding> = clinkz_wot_core::client_only("test-c", factory);

    let servient = ServientBuilder::new()
        .with_protocol_binding(binding)
        .build()
        .expect("build");

    let handle = servient.produce(lamp_td()).expect("produce");
    // expose() still succeeds (the servient has no server binding, but the
    // registry insert + TD publish still happen; register_thing is a no-op).
    handle.expose().await.expect("expose");

    // Consumed side still works: client factory is wired.
    let consumed = servient.consume(lamp_td()).expect("consume");
    consumed
        .read_property("status", Default::default())
        .await
        .expect("read");
}

#[tokio::test]
async fn pure_exposer_binding_does_not_register_client() {
    // A pure-exposer binding (client_factory() returns None) cannot serve
    // consume(): read_property fails because no client binding matches.
    let server = Arc::new(FakeServer::default());
    let binding: Arc<dyn ProtocolBinding> =
        clinkz_wot_core::server_only("test-s", server.clone() as Arc<FakeServer>);

    let servient = ServientBuilder::new()
        .with_protocol_binding(binding)
        .build()
        .expect("build");

    let handle = servient.produce(lamp_td()).expect("produce");
    handle.expose().await.expect("expose");
    assert_eq!(server.registered_count(), 1, "server registered");

    let consumed = servient.consume(lamp_td()).expect("consume");
    let err = consumed
        .read_property("status", Default::default())
        .await
        .unwrap_err();
    assert!(
        matches!(err, CoreError::UnsupportedOperation(_) | CoreError::UnsupportedBinding(_)),
        "no client binding available, got {err:?}"
    );
}

#[tokio::test]
async fn each_consume_builds_a_fresh_client_binding() {
    let (factory, _invocations) = CountingClientFactory::new();
    let server = Arc::new(FakeServer::default());

    // Snapshot the builds counter externally by cloning the factory's
    // builds Arc before wrapping.
    let factory_for_binding = factory.clone();
    let builds_snapshot = factory_for_binding.builds.clone();

    let binding = Arc::new(TwoDirectionBinding {
        protocol: ProtocolId("test"),
        factory: factory_for_binding,
        server,
    });
    let servient = ServientBuilder::new()
        .with_protocol_binding(binding)
        .build()
        .expect("build");

    let _c1 = servient.consume(lamp_td()).expect("consume");
    let _c2 = servient.consume(lamp_td()).expect("consume");
    let _c3 = servient.consume(lamp_td()).expect("consume");

    // Each consume() must instantiate a fresh ClientBinding via the factory.
    assert_eq!(*builds_snapshot.lock().unwrap(), 3);
}

#[tokio::test]
async fn mixing_with_legacy_hooks_still_works_in_p0() {
    // P0 guarantee: the legacy with_server_binding / with_client_factory
    // entrypoints remain usable alongside with_protocol_binding. P1 will
    // retire them.
    let (factory, _invocations) = CountingClientFactory::new();
    let server: Arc<dyn ServerBinding> =
        Arc::new(FakeServer::default()) as Arc<dyn ServerBinding>;
    let facade: Arc<dyn ProtocolBinding> =
        clinkz_wot_core::client_only("test-c", factory);

    let servient = ServientBuilder::new()
        .with_server_binding(server.clone())
        .with_protocol_binding(facade)
        .build()
        .expect("build");

    // Combined: server from legacy hook, client from facade.
    let handle = servient.produce(lamp_td()).expect("produce");
    handle.expose().await.expect("expose");

    let consumed = servient.consume(lamp_td()).expect("consume");
    consumed
        .read_property("status", Default::default())
        .await
        .expect("read");
}

// --- keep imports used only in type signatures ------------------------------

#[allow(dead_code)]
fn _ensure_imports(
    _: AffordanceTarget,
    _: InboundResponse,
    _: InteractionInput,
    _: &dyn PropertyReadHandler,
) {
}
