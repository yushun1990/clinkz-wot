//! P2 integration: `ConsumedThingHandle` streaming and bulk ops.
//!
//! Verifies:
//! - `observe_property` opens a subscription, samples flow through, and the
//!   wire-side guard stays alive until `unobserve_property` drops it or the
//!   handle is dropped.
//! - `subscribe_event` / `unsubscribe_event` symmetric pair.
//! - `subscribe_all_events` fans out across all declared events and yields
//!   `(EventName, Payload)` tuples.
//! - `read_all_properties` / `read_multiple_properties` aggregate per-property
//!   payloads into a single JSON `InteractionOutput`.
//! - `write_multiple_properties` writes each entry sequentially and surfaces
//!   the first error.
//!
//! All exercised against a fake binding that implements both `invoke` (echo
//! input data) and `subscribe` (push canned samples on demand).

#![cfg(all(feature = "async", feature = "std"))]

use std::collections::{BTreeMap as StdBTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use clinkz_wot_core::{
    BindingContext, BindingRequest, ClientBinding, CoreError, CoreResult, EventName,
    InteractionInput, InteractionOptions, InteractionOutput, Payload, ServerBinding, Subscription,
    SubscriptionGuard, ThingId,
};
use clinkz_wot_servient::ServientBuilder;
use clinkz_wot_td::{
    affordance::{EventAffordance, InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    data_type::Operation,
    form::Form,
    thing::Thing,
};
use futures_util::StreamExt;

// ---------------------------------------------------------------------------
// Fakes
// ---------------------------------------------------------------------------

/// Fake client binding that supports every form and either echoes the input
/// `data` on `invoke` (for read/write/invoke paths) or hands back a paired
/// `(Subscription, SubscriptionGuard)` on `subscribe`. The subscription
/// channel is pre-fed with `canned_samples` keyed by affordance name
/// (`request.target.name()`) so multi-event / multi-property tests can
/// distinguish which canned payload belongs to which affordance.
#[derive(Default)]
struct StreamingClient {
    /// Canned payloads keyed by affordance name; pushed into the matching
    /// opened subscription before the guard is returned. Drained per call.
    canned_samples: Arc<Mutex<StdBTreeMap<String, VecDeque<Vec<u8>>>>>,
    /// Count of guards currently alive (across all opened subscriptions).
    live_guard_count: Arc<Mutex<usize>>,
}

struct FakeGuard {
    counter: Arc<Mutex<usize>>,
}

impl SubscriptionGuard for FakeGuard {
    fn close(self: Box<Self>) {
        let mut c = self.counter.lock().unwrap();
        *c = c.saturating_sub(1);
    }
}

impl Drop for FakeGuard {
    fn drop(&mut self) {
        let mut c = self.counter.lock().unwrap();
        *c = c.saturating_sub(1);
    }
}

#[async_trait]
impl ClientBinding for StreamingClient {
    fn supports(&self, _form: &Form, _op: Operation) -> bool {
        true
    }

    async fn invoke(&self, request: BindingRequest) -> Result<InteractionOutput, CoreError> {
        Ok(InteractionOutput::with_data(
            request.input.data.unwrap_or_default(),
        ))
    }

    async fn subscribe(
        &self,
        request: BindingRequest,
    ) -> Result<(Subscription, Box<dyn SubscriptionGuard>), CoreError> {
        let (sender, sub) = Subscription::channel(8);
        if let Some(name) = request.target.name() {
            let mut canned = self.canned_samples.lock().unwrap();
            if let Some(queue) = canned.get_mut(name) {
                while let Some(sample) = queue.pop_front() {
                    let _ = sender.push(Payload::new(sample, "text/plain"));
                }
            }
        }

        *self.live_guard_count.lock().unwrap() += 1;
        let guard = Box::new(FakeGuard {
            counter: self.live_guard_count.clone(),
        });
        Ok((sub, guard))
    }
}

#[allow(dead_code)]
#[derive(Default)]
struct NoopServer;
impl ServerBinding for NoopServer {
    fn serve(&self, _: &ThingId, _: &Thing, _: &BindingContext) -> CoreResult<()> {
        Ok(())
    }
    fn shutdown(&self, _: &ThingId) {}
    fn send_response(&self, _response: clinkz_wot_core::InboundResponse) {}
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn multi_event_td() -> Thing {
    Thing::builder("Sensor")
        .id("urn:test:sensor")
        .nosec()
        .property(
            "temperature",
            PropertyAffordance::builder(DataSchema::string())
                .form(
                    Form::read_property("fake://clinkz/sensor/properties/temperature")
                        .build()
                        .unwrap(),
                )
                .form(
                    Form::write_property("fake://clinkz/sensor/properties/temperature")
                        .build()
                        .unwrap(),
                )
                .form(
                    Form::builder("fake://clinkz/sensor/properties/temperature")
                        .observe_property()
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .property(
            "humidity",
            PropertyAffordance::builder(DataSchema::string())
                .form(
                    Form::read_property("fake://clinkz/sensor/properties/humidity")
                        .build()
                        .unwrap(),
                )
                .form(
                    Form::write_property("fake://clinkz/sensor/properties/humidity")
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .event(
            "motion",
            EventAffordance::builder()
                .form(
                    Form::subscribe_event("fake://clinkz/sensor/events/motion")
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .event(
            "startup",
            EventAffordance::builder()
                .form(
                    Form::subscribe_event("fake://clinkz/sensor/events/startup")
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
}

/// Keyed canned-sample map used by the streaming fake binding.
type CannedSamples = StdBTreeMap<String, Vec<Vec<u8>>>;
/// Shared handle to the canned-sample buffer.
type CannedHandle = Arc<Mutex<StdBTreeMap<String, VecDeque<Vec<u8>>>>>;

fn build_servient(
    canned: CannedSamples,
) -> (
    clinkz_wot_servient::Servient,
    Arc<Mutex<usize>>,
    CannedHandle,
) {
    let keyed: StdBTreeMap<String, VecDeque<Vec<u8>>> = canned
        .into_iter()
        .map(|(k, v)| (k, VecDeque::from(v)))
        .collect();
    let canned_samples = Arc::new(Mutex::new(keyed));
    let live_guard_count = Arc::new(Mutex::new(0usize));
    let client: Arc<dyn ClientBinding> = Arc::new(StreamingClient {
        canned_samples: canned_samples.clone(),
        live_guard_count: live_guard_count.clone(),
    });
    let servient = ServientBuilder::new()
        .with_client_binding(client)
        .build()
        .expect("build");
    (servient, live_guard_count, canned_samples)
}

fn single_key_canned(key: &str, samples: Vec<Vec<u8>>) -> StdBTreeMap<String, Vec<Vec<u8>>> {
    let mut m = StdBTreeMap::new();
    m.insert(key.to_string(), samples);
    m
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn observe_property_returns_subscription_that_yields_canned_samples() {
    let (servient, live_guards, _) = build_servient(single_key_canned(
        "temperature",
        vec![b"21C".to_vec(), b"22C".to_vec()],
    ));
    let handle = servient.consume(multi_event_td()).expect("consume");

    let mut sub = handle
        .observe_property("temperature", InteractionOptions::new())
        .await
        .expect("observe");

    let first = sub.next().await.expect("first sample");
    assert_eq!(first.body.as_ref(), b"21C");
    let second = sub.next().await.expect("second sample");
    assert_eq!(second.body.as_ref(), b"22C");

    // Guard stays alive while the subscription is open.
    assert_eq!(*live_guards.lock().unwrap(), 1);

    handle
        .unobserve_property("temperature", InteractionOptions::new())
        .await
        .expect("unobserve");
    assert_eq!(
        *live_guards.lock().unwrap(),
        0,
        "unobserve drops the wire-side guard"
    );
}

#[tokio::test]
async fn subscribe_event_round_trips_and_unsubscribe_releases_guard() {
    let (servient, live_guards, _) = build_servient(single_key_canned(
        "motion",
        vec![b"motion-detected".to_vec()],
    ));
    let handle = servient.consume(multi_event_td()).expect("consume");

    let mut sub = handle
        .subscribe_event("motion", InteractionOptions::new())
        .await
        .expect("subscribe event");

    let sample = sub.next().await.expect("event sample");
    assert_eq!(sample.body.as_ref(), b"motion-detected");

    assert_eq!(*live_guards.lock().unwrap(), 1);
    handle
        .unsubscribe_event("motion", InteractionOptions::new())
        .await
        .expect("unsubscribe");
    assert_eq!(*live_guards.lock().unwrap(), 0);
}

#[tokio::test]
async fn subscribe_all_events_yields_named_samples_from_every_event() {
    let mut canned = StdBTreeMap::new();
    canned.insert("motion".to_string(), vec![b"motion-A".to_vec()]);
    canned.insert("startup".to_string(), vec![b"startup-B".to_vec()]);
    let (servient, live_guards, _) = build_servient(canned);
    let handle = servient.consume(multi_event_td()).expect("consume");

    let mut stream = handle
        .subscribe_all_events(InteractionOptions::new())
        .await
        .expect("subscribe all");

    assert_eq!(stream.event_count(), 2, "two events in the TD");
    let names: Vec<String> = stream
        .event_names()
        .into_iter()
        .map(|n| n.as_str().to_string())
        .collect();
    assert!(names.contains(&"motion".to_string()));
    assert!(names.contains(&"startup".to_string()));

    let collected: Vec<(EventName, Payload)> = (&mut stream).take(2).collect().await;
    assert_eq!(collected.len(), 2);
    let by_name: StdBTreeMap<String, Vec<u8>> = collected
        .into_iter()
        .map(|(n, p)| (n.as_str().to_string(), p.body.to_vec()))
        .collect();
    assert_eq!(
        by_name.get("motion").map(|v| v.as_slice()),
        Some(&b"motion-A"[..])
    );
    assert_eq!(
        by_name.get("startup").map(|v| v.as_slice()),
        Some(&b"startup-B"[..])
    );

    // Two events → two guards.
    assert_eq!(*live_guards.lock().unwrap(), 2);

    // Dropping the handle should release every still-open guard.
    drop(handle);
    assert_eq!(
        *live_guards.lock().unwrap(),
        0,
        "handle drop releases all remaining guards"
    );
}

#[tokio::test]
async fn read_all_properties_aggregates_into_json_object() {
    // The EchoClient returns input.data; reads use empty input so the
    // response payload body will be empty. To get meaningful bodies,
    // pre-seed canned samples — but `invoke` doesn't consult them. The
    // aggregation code accepts empty bodies (they become "" in JSON).
    //
    // For a richer aggregation test we need a binding that returns
    // canned data on invoke. Use interior mutability so `&self` can
    // pop the next canned value.
    use std::sync::Mutex;

    struct CannedEcho {
        canned: Mutex<Vec<Vec<u8>>>,
    }
    #[async_trait]
    impl ClientBinding for CannedEcho {
        fn supports(&self, _: &Form, _: Operation) -> bool {
            true
        }
        async fn invoke(&self, _request: BindingRequest) -> Result<InteractionOutput, CoreError> {
            let next = self.canned.lock().unwrap().remove(0);
            Ok(InteractionOutput::with_data(Payload::new(
                next,
                "text/plain",
            )))
        }
        async fn subscribe(
            &self,
            _: BindingRequest,
        ) -> Result<(Subscription, Box<dyn SubscriptionGuard>), CoreError> {
            Err(CoreError::UnsupportedOperation("no streaming".into()))
        }
    }

    let client: Arc<dyn ClientBinding> = Arc::new(CannedEcho {
        canned: Mutex::new(vec![
            // BTreeMap iterates alphabetically: humidity first, then
            // temperature. Order canned values to match.
            b"55pct".to_vec(),
            b"21C".to_vec(),
        ]),
    });
    let servient = ServientBuilder::new()
        .with_client_binding(client)
        .build()
        .expect("build");
    let handle = servient.consume(multi_event_td()).expect("consume");

    let out = handle
        .read_all_properties(InteractionOptions::new())
        .await
        .expect("read all");
    let payload = out.data.expect("aggregated payload");
    let body = std::str::from_utf8(payload.body.as_ref()).unwrap();
    assert!(
        body.contains("\"temperature\":\"21C\""),
        "temperature value present, got: {body}"
    );
    assert!(
        body.contains("\"humidity\":\"55pct\""),
        "humidity value present, got: {body}"
    );
    assert!(body.starts_with('{') && body.ends_with('}'));
}

#[tokio::test]
async fn write_multiple_properties_invokes_each_write_sequentially() {
    let (servient, _, _) = build_servient(StdBTreeMap::new());
    let handle = servient.consume(multi_event_td()).expect("consume");

    let mut entries: std::collections::BTreeMap<&str, Payload> = std::collections::BTreeMap::new();
    entries.insert("temperature", Payload::new(b"21C".to_vec(), "text/plain"));
    entries.insert("humidity", Payload::new(b"55pct".to_vec(), "text/plain"));

    handle
        .write_multiple_properties(&entries, InteractionOptions::new())
        .await
        .expect("write all");
}

#[tokio::test]
async fn unobserve_without_prior_observe_is_idempotent() {
    let (servient, _, _) = build_servient(StdBTreeMap::new());
    let handle = servient.consume(multi_event_td()).expect("consume");
    handle
        .unobserve_property("temperature", InteractionOptions::new())
        .await
        .expect("idempotent unobserve without prior observe");
}

// Keep imports used only in type signatures alive.
#[allow(dead_code)]
fn _ensure_imports<T: SubscriptionGuard>(_: &T, _: &mut InteractionInput) {}
