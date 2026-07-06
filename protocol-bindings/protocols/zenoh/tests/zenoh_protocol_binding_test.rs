//! P1 migration pattern: how a zenoh-based application registers both
//! sides of the protocol through the unified `ProtocolBinding` facade.
//!
//! Replaces the pre-P1 pattern of `ServientBuilder::new().with_server_binding(...)
//! .with_client_factory(...)` (now retired). The single entry point is
//! `ServientBuilder::with_protocol_binding(Arc<dyn ProtocolBinding>)`.
//!
//! Gated on `CLINKZ_WOT_RUN_ZENOH_RUNTIME_TESTS=1` because opening a zenoh
//! session is a real network call.

#![cfg(feature = "zenoh")]

use std::{env, sync::Arc};

use clinkz_wot_core::ProtocolBinding;
use clinkz_wot_protocol_bindings_zenoh::{
    ZenohProtocolBinding, ZenohSessionTransport,
};
use clinkz_wot_servient::ServientBuilder;
use clinkz_wot_td::{
    affordance::{InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    thing::Thing,
};

const RUN_ZENOH_RUNTIME_TESTS: &str = "CLINKZ_WOT_RUN_ZENOH_RUNTIME_TESTS";

#[tokio::test]
async fn zenoh_protocol_binding_shared_composes_with_servient() {
    if !should_run() {
        return;
    }

    zenoh::init_log_from_env_or("error");

    // Canonical shared-session topology: one zenoh::Session drives both
    // ZenohServerBinding and ZenohRuntimeTransport. The
    // `ZenohProtocolBinding::shared` constructor wraps both into a single
    // `Arc<dyn ProtocolBinding>`.
    let session = ZenohSessionTransport::open(zenoh::Config::default())
        .expect("open zenoh session")
        .session()
        .clone();
    let binding: Arc<dyn ProtocolBinding> = ZenohProtocolBinding::shared(session);

    // Single entry point: with_protocol_binding. The Servient extracts
    // the client factory and server singleton internally.
    let servient = ServientBuilder::new()
        .with_protocol_binding(binding)
        .build()
        .expect("build servient");

    // produce() accepts a TD; consume() accepts the same TD. Both sides
    // share the protocol binding infrastructure without further wiring.
    let td = lamp_td();
    let _exposed = servient.produce(td.clone()).expect("produce");
    let _consumed = servient.consume(td).expect("consume");
}

#[tokio::test]
async fn zenoh_protocol_binding_client_only_via_new_constructor() {
    if !should_run() {
        return;
    }

    zenoh::init_log_from_env_or("error");

    // Pure-consumer topology: `ZenohProtocolBinding::new(transport)` yields
    // a client-only facade (server() returns None). Useful for cloud
    // controllers that never expose local Things.
    let session = ZenohSessionTransport::open(zenoh::Config::default())
        .expect("open zenoh session")
        .session()
        .clone();
    let transport = ZenohSessionTransport::new(session);
    let binding: Arc<dyn ProtocolBinding> = Arc::new(ZenohProtocolBinding::new(transport));

    assert!(binding.server().is_none(), "client-only facade has no server");

    let servient = ServientBuilder::new()
        .with_protocol_binding(binding)
        .build()
        .expect("build servient");

    // consume() works without a registered server.
    let _consumed = servient.consume(lamp_td()).expect("consume");
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
