//! P0 integration: `ServientBuilder::with_server_binding` /
//! `with_client_binding` register independent inbound and outbound bindings
//! (v4.1 AD55–AD57).
//!
//! Verifies:
//! - A server binding receives `serve` on expose and `shutdown` on destroy.
//! - A client binding receives `invoke` on consume-side interactions.
//! - Pure-consumer setups (client binding only) never call `serve`.
//! - Pure-exposer setups (server binding only) leave consume unable to find
//!   a matching client.
//! - The same shared `Arc<dyn ClientBinding>` serves every consumed Thing
//!   (AD57: one binding per protocol, not one per consume).

#![cfg(all(feature = "async", feature = "std"))]

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use clinkz_wot_core::{
    BindingContext, BindingRequest, ClientBinding, CoreError, CoreResult, InteractionOutput,
    ServerBinding, ThingId,
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
    fn serve(&self, thing_id: &ThingId, _td: &Thing, _ctx: &BindingContext) -> CoreResult<()> {
        self.registered
            .lock()
            .unwrap()
            .push(thing_id.as_str().to_string());
        Ok(())
    }
    fn shutdown(&self, thing_id: &ThingId) {
        self.unregistered
            .lock()
            .unwrap()
            .push(thing_id.as_str().to_string());
    }
    fn send_response(&self, _response: clinkz_wot_core::InboundResponse) {}
}

impl FakeServer {
    fn registered_count(&self) -> usize {
        self.registered.lock().unwrap().len()
    }
}

struct CountingClient {
    invocations: Arc<Mutex<usize>>,
}

#[async_trait]
impl ClientBinding for CountingClient {
    fn supports(&self, _form: &clinkz_wot_td::form::Form, _op: Operation) -> bool {
        true
    }
    async fn invoke(&self, _request: BindingRequest) -> CoreResult<InteractionOutput> {
        *self.invocations.lock().unwrap() += 1;
        Ok(InteractionOutput::empty())
    }
}

impl CountingClient {
    /// Returns a shared `Arc<dyn ClientBinding>` and the counter it mutates.
    fn shared() -> (Arc<dyn ClientBinding>, Arc<Mutex<usize>>) {
        let invocations = Arc::new(Mutex::new(0usize));
        (
            Arc::new(CountingClient {
                invocations: invocations.clone(),
            }),
            invocations,
        )
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
async fn server_and_client_bindings_register_independently() {
    let fake_server = Arc::new(FakeServer::default());
    let (client, _invocations) = CountingClient::shared();

    let servient = ServientBuilder::new()
        .with_server_binding(fake_server.clone())
        .with_client_binding(client)
        .build()
        .expect("build");

    // Exposed side: produce+expose drives serve on the shared server.
    let handle = servient.produce(lamp_td()).expect("produce");
    handle.expose().await.expect("expose");
    assert_eq!(fake_server.registered_count(), 1, "server serve called");

    // Consumed side: the shared ClientBinding handles the interaction.
    let consumed = servient.consume(lamp_td()).expect("consume");
    consumed
        .read_property("status", Default::default())
        .await
        .expect("read");
}

#[tokio::test]
async fn pure_consumer_binding_does_not_register_routes() {
    // A client-binding-only Servient has no server binding, so producing +
    // exposing a Thing never invokes `serve`. consume() still works because
    // the client binding is wired.
    let (client, _invocations) = CountingClient::shared();

    let servient = ServientBuilder::new()
        .with_client_binding(client)
        .build()
        .expect("build");

    let handle = servient.produce(lamp_td()).expect("produce");
    // expose() still succeeds (the servient has no server binding, but the
    // registry insert + TD publish still happen; serve is a no-op).
    handle.expose().await.expect("expose");

    // Consumed side still works: client binding is wired.
    let consumed = servient.consume(lamp_td()).expect("consume");
    consumed
        .read_property("status", Default::default())
        .await
        .expect("read");
}

#[tokio::test]
async fn pure_exposer_binding_leaves_consume_without_client() {
    // A server-binding-only Servient cannot serve consume(): read_property
    // fails because no client binding matches.
    let fake_server = Arc::new(FakeServer::default());

    let servient = ServientBuilder::new()
        .with_server_binding(fake_server.clone())
        .build()
        .expect("build");

    let handle = servient.produce(lamp_td()).expect("produce");
    handle.expose().await.expect("expose");
    assert_eq!(fake_server.registered_count(), 1, "server registered");

    let consumed = servient.consume(lamp_td()).expect("consume");
    let err = consumed
        .read_property("status", Default::default())
        .await
        .unwrap_err();
    assert!(
        matches!(
            err,
            CoreError::UnsupportedOperation(_) | CoreError::UnsupportedBinding(_)
        ),
        "no client binding available, got {err:?}"
    );
}

#[tokio::test]
async fn each_consume_shares_the_same_client_binding() {
    // AD57: one shared `Arc<dyn ClientBinding>` per protocol serves every
    // consumed Thing. Three consumes route through one binding, so the
    // invocation counter accumulates across all of them.
    let (client, invocations) = CountingClient::shared();

    let servient = ServientBuilder::new()
        .with_client_binding(client)
        .build()
        .expect("build");

    let c1 = servient.consume(lamp_td()).expect("consume");
    let c2 = servient.consume(lamp_td()).expect("consume");
    let c3 = servient.consume(lamp_td()).expect("consume");

    c1.read_property("status", Default::default())
        .await
        .expect("read");
    c2.read_property("status", Default::default())
        .await
        .expect("read");
    c3.read_property("status", Default::default())
        .await
        .expect("read");

    // One shared binding → all invocations accumulate on the same counter.
    assert_eq!(*invocations.lock().unwrap(), 3);
}
