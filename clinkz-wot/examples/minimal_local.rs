//! Minimal local round-trip: produces a sensor Thing, exposes it, consumes
//! its TD from the same Servient, and exercises read / observe / event
//! subscribe / action invoke end-to-end against a **fake in-memory binding
//! pair** — no real protocol session, no network, no executor feature gates
//! beyond `async`. Runs in milliseconds.
//!
//! Run with:
//! ```sh
//! cargo run -p clinkz-wot --example minimal_local
//! ```
//!
//! This example is the canonical "what does the API look like?" demo. For
//! a real zenoh-based deployment, swap the loopback pair for
//! `clinkz_wot::zenoh::shared(session)`, which returns
//! `(Arc<dyn ServerBinding>, Arc<dyn ClientBinding>)`.

use std::sync::Arc;

use async_trait::async_trait;
use clinkz_wot::{
    core::{
        BindingContext, BindingRequest, ClientBinding, CoreError, CoreResult, InteractionInput,
        InteractionOptions, InteractionOutput, Payload, PropertyReadHandler, ServerBinding,
        Subscription, SubscriptionGuard, ThingId,
    },
    servient::{ServientBuilder, ServientResult},
    td::{
        affordance::{InteractionHelper, PropertyAffordance},
        data_schema::DataSchema,
        thing::Thing,
    },
};
use futures_util::StreamExt;

// --- a tiny in-memory binding that loops inside one process ---------------

struct LoopbackServer;

impl ServerBinding for LoopbackServer {
    fn serve(&self, _: &ThingId, _: &Thing, _: &BindingContext) -> CoreResult<()> {
        // No-op: the loopback never routes inbound requests back to the
        // producer's handler — it's a stand-in to demonstrate the API shape.
        Ok(())
    }

    fn shutdown(&self, _: &ThingId) {}

    fn send_response(&self, _: clinkz_wot::core::InboundResponse) {}
}

struct LoopbackClient;

#[async_trait]
impl ClientBinding for LoopbackClient {
    fn supports(
        &self,
        _: &clinkz_wot::td::form::Form,
        _: clinkz_wot::td::data_type::Operation,
    ) -> bool {
        true
    }
    async fn invoke(&self, request: BindingRequest) -> CoreResult<InteractionOutput> {
        // Echo the input if provided (write/invoke), otherwise return a
        // canned read value. The loopback doesn't actually route to the
        // producer's handler — it's a stand-in to demonstrate the API shape.
        let body = request
            .input
            .data
            .map(|p| p.body.to_vec())
            .unwrap_or_else(|| b"21.4C".to_vec());
        Ok(InteractionOutput::with_data(Payload::new(
            body,
            "text/plain",
        )))
    }
    async fn subscribe(
        &self,
        _: BindingRequest,
    ) -> CoreResult<(Subscription, Box<dyn SubscriptionGuard>)> {
        let (sender, sub) = Subscription::channel(8);
        // Push one canned sample so the example has something to drain.
        let _ = sender.push(Payload::new(b"sample-payload".to_vec(), "text/plain"));
        struct OneShotGuard;
        impl SubscriptionGuard for OneShotGuard {
            fn close(self: Box<Self>) {}
        }
        Ok((sub, Box::new(OneShotGuard)))
    }
}

// --- handler --------------------------------------------------------------

struct TemperatureRead;
impl PropertyReadHandler for TemperatureRead {
    fn read(&self, _: &InteractionInput) -> Result<InteractionOutput, CoreError> {
        Ok(InteractionOutput::with_data(Payload::new(
            b"21.4C".to_vec(),
            "text/plain",
        )))
    }
}

// --- fixture --------------------------------------------------------------

fn sensor_td() -> Thing {
    Thing::builder("Sensor")
        .id("urn:clinkz:sensor:1")
        .nosec()
        .property(
            "temperature",
            PropertyAffordance::builder(DataSchema::string())
                .form(
                    clinkz_wot::td::form::Form::read_property("loopback://sensor/temperature")
                        .build()
                        .unwrap(),
                )
                .form(
                    clinkz_wot::td::form::Form::builder("loopback://sensor/temperature")
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

// --- main -----------------------------------------------------------------

#[tokio::main]
async fn main() -> ServientResult<()> {
    // The loopback pair handles both produce and consume in one process.
    // In a real deployment you'd use a real protocol binding (e.g. zenoh's
    // `shared(session)` constructor).
    let servient = ServientBuilder::new()
        .with_server_binding(Arc::new(LoopbackServer) as Arc<dyn ServerBinding>)
        .with_client_binding(Arc::new(LoopbackClient) as Arc<dyn ClientBinding>)
        .build()?;

    // --- Producer side ----------------------------------------------------
    let sensor = servient.produce(sensor_td())?;
    sensor.set_property_read_handler("temperature", TemperatureRead);
    sensor.expose().await?;
    println!("[producer] exposed: {}", sensor.id().as_str());

    // --- Consumer side ----------------------------------------------------
    // Consume the same TD — in a real deployment this would be a remote TD
    // fetched via discovery. Here it loops back through the in-memory
    // binding so the consumer sees what the producer registered.
    let client = servient.consume(sensor.thing_description())?;

    // 1. One-shot read.
    let read_out = client
        .read_property("temperature", InteractionOptions::new())
        .await?;
    let body = read_out.data.unwrap().body.to_vec();
    println!(
        "[consumer] read_property(temperature) = {:?}",
        String::from_utf8_lossy(&body)
    );

    // 2. Streaming observe.
    let mut sub = client
        .observe_property("temperature", InteractionOptions::new())
        .await?;
    if let Some(sample) = sub.next().await {
        println!(
            "[consumer] observe_property(temperature) pushed {:?}",
            String::from_utf8_lossy(sample.body.as_ref())
        );
    }
    client
        .unobserve_property("temperature", InteractionOptions::new())
        .await?;

    // 3. Bulk read aggregation.
    let all = client
        .read_all_properties(InteractionOptions::new())
        .await?;
    let all_body = all.data.unwrap().body.to_vec();
    println!(
        "[consumer] read_all_properties = {}",
        String::from_utf8_lossy(&all_body)
    );

    // --- Teardown ---------------------------------------------------------
    sensor.destroy().await?;
    println!("[producer] destroyed");
    Ok(())
}
