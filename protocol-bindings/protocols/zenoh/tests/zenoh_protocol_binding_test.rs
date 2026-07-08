//! v4.1 binding ownership: how a zenoh-based application constructs the
//! server and client bindings through the dedicated `shared` / `server` /
//! `client` constructors and registers them via `ServientBuilder`.
//!
//! Replaces the v4.0 `ZenohProtocolBinding` facade (and the single
//! `ServientBuilder::with_protocol_binding` entry point). The canonical
//! shared-session topology is `clinkz_wot_protocol_bindings_zenoh::shared`,
//! which returns a ready-to-register `(Arc<dyn ServerBinding>,
//! Arc<dyn ClientBinding>)` pair.
//!
//! Gated on `CLINKZ_WOT_RUN_ZENOH_RUNTIME_TESTS=1` because opening a zenoh
//! session is a real network call.

#![cfg(feature = "zenoh")]

use std::{env, sync::Arc};

use clinkz_wot_core::{ClientBinding, ServerBinding};
use clinkz_wot_protocol_bindings_zenoh::{ZenohSessionTransport, client, shared};
use clinkz_wot_servient::ServientBuilder;
use clinkz_wot_td::{
    affordance::{InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    thing::Thing,
};
use zenoh::Wait;

const RUN_ZENOH_RUNTIME_TESTS: &str = "CLINKZ_WOT_RUN_ZENOH_RUNTIME_TESTS";

#[tokio::test]
async fn zenoh_shared_constructor_composes_with_servient() {
    if !should_run() {
        return;
    }

    zenoh::init_log_from_env_or("error");

    // Canonical shared-session topology: one zenoh::Session drives both
    // the server binding and the client transport. `shared` returns a
    // ready-to-register `(Arc<dyn ServerBinding>, Arc<dyn ClientBinding>)`.
    let session = zenoh::open(zenoh::Config::default())
        .wait()
        .expect("open zenoh session");
    let (server, client) = shared(session);

    // Register each side through its dedicated builder entry point.
    let servient = ServientBuilder::new()
        .with_server_binding(server)
        .with_client_binding(client)
        .build()
        .expect("build servient");

    // produce() accepts a TD; consume() accepts the same TD. Both sides
    // share the underlying session without further wiring.
    let td = lamp_td();
    let _exposed = servient.produce(td.clone()).expect("produce");
    let _consumed = servient.consume(td).expect("consume");
}

#[tokio::test]
async fn zenoh_client_only_constructor_skips_server_registration() {
    if !should_run() {
        return;
    }

    zenoh::init_log_from_env_or("error");

    // Pure-consumer topology: `client(transport)` yields a client-only
    // `Arc<dyn ClientBinding>`. Useful for cloud controllers that never
    // expose local Things.
    let transport =
        ZenohSessionTransport::open(zenoh::Config::default()).expect("open zenoh session");
    let binding: Arc<dyn ClientBinding> = client(transport);

    // No server binding is registered — consume() works without one.
    let servient = ServientBuilder::new()
        .with_client_binding(binding)
        .build()
        .expect("build servient");

    let _consumed = servient.consume(lamp_td()).expect("consume");
}

/// Verifies the `shared` constructor returns types that satisfy both the
/// `ServerBinding` and `ClientBinding` contracts. This does not open a real
/// zenoh session (it is type-checked only) so it runs in the default test
/// suite without `CLINKZ_WOT_RUN_ZENOH_RUNTIME_TESTS`.
#[allow(dead_code)]
fn shared_constructor_typechecks(session: zenoh::Session) {
    let (server, client): (Arc<dyn ServerBinding>, Arc<dyn ClientBinding>) = shared(session);
    let _ = server;
    let _ = client;
}

/// Verifies the `client` constructor returns an `Arc<dyn ClientBinding>`.
#[allow(dead_code)]
fn client_constructor_typechecks(transport: ZenohSessionTransport) {
    let _binding: Arc<dyn ClientBinding> = client(transport);
}

// --- fixtures ---------------------------------------------------------------

fn lamp_td() -> Thing {
    Thing::builder("Lamp")
        .id("urn:clinkz:lamp")
        .nosec()
        .property(
            "status",
            PropertyAffordance::builder(DataSchema::string())
                .form(
                    clinkz_wot_td::form::Form::read_property(
                        "zenoh://clinkz/things/urn:clinkz:lamp/properties/status",
                    )
                    .build()
                    .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
}

fn should_run() -> bool {
    if env::var(RUN_ZENOH_RUNTIME_TESTS).ok().as_deref() == Some("1") {
        true
    } else {
        eprintln!("skipping real zenoh runtime test; set {RUN_ZENOH_RUNTIME_TESTS}=1");
        false
    }
}
