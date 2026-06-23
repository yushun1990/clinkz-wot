//! Event fan-out and outbound subscription surfaces.
//!
//! Implements the v3.0 §9 / v3.1 §6.1 event data flow:
//!
//! - [`EventBroker`] holds a fan-out table keyed by `ThingId` and
//!   [`EventName`]. A broker-backed [`EventSink`] (see
//!   [`EventBroker::event_sink`]) is handed to a local
//!   [`EventSubscribeHandler::subscribe`](crate::EventSubscribeHandler::subscribe) call; each
//!   `emit` fans the payload out to every registered [`PublisherSink`], each of
//!   which wraps a server binding's publish channel and pushes the payload to a
//!   remote subscriber.
//! - [`Subscription`] is a bounded per-subscription queue for outbound
//!   (consumed) events with drop-oldest + overflow-counter backpressure. The
//!   caller drains it via [`Subscription::poll_next`] and stops it via
//!   [`Subscription::stop`].
//!
//! ## Queue primitive
//!
//! The bounded queue is implemented over [`alloc::collections::VecDeque`]. This
//! refines the addendum §6.1 primitive choice (`heapless::spsc::Queue` on
//! `no_std`, `flume`/`tokio::mpsc` on `std`) while preserving the full locked
//! behavioral contract: bounded capacity, drop-oldest eviction, an observable
//! overflow counter, per-subscription configurable capacity, and `Clone`-able
//! handles. Using `VecDeque` keeps the queue `no_std + alloc` with
//! *runtime-configurable* capacity on every build and avoids both a `heapless`
//! const-generic capacity constraint and a `std`-only channel dependency. The
//! interior-mutability wrapper is cfg-selected: [`core::cell::RefCell`] on
//! `no_std` (single-core MCU), [`std::sync::Mutex`] on `std`.
//!
//! The async [`core::task`] based `Stream` impl for [`Subscription`] is
//! deferred to the async driving-layer phase, which introduces the runtime
//! dependency; the synchronous [`Subscription::poll_next`] surface is the
//! primary drain primitive and is available on every build.

use alloc::{
    collections::{BTreeMap, VecDeque},
    string::String,
    sync::Arc,
    vec::Vec,
};
use core::fmt;

use crate::MapLock;
use crate::thing::EventSink;
use crate::{CoreError, CoreResult, Payload, ThingId};

/// Crate-default capacity for a [`Subscription`] queue when none is requested.
///
/// See addendum §6.1.
pub const DEFAULT_SUBSCRIPTION_CAPACITY: usize = 16;

/// Name of an event affordance within a [`ThingId`]'s scope.
///
/// Carried as a [`String`] newtype so it is `'static` and can key a fan-out
/// table alongside [`ThingId`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EventName(String);

impl EventName {
    /// Creates an event name from an owned string.
    pub fn new(name: String) -> Self {
        Self(name)
    }

    /// Returns the name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the underlying owned name string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for EventName {
    fn from(name: String) -> Self {
        Self(name)
    }
}

impl From<&str> for EventName {
    fn from(name: &str) -> Self {
        Self(String::from(name))
    }
}

impl AsRef<str> for EventName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EventName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Publisher side of an inbound (exposed) event subscription.
///
/// Each `PublisherSink` wraps a server binding's publish channel and pushes a
/// fanned-out payload to the remote subscriber it represents. Implementations
/// must not re-enter the [`EventBroker`] that owns them from within
/// [`publish`](Self::publish), since fan-out runs under the broker's lock.
pub trait PublisherSink {
    /// Publishes an event payload to the remote subscriber.
    fn publish(&self, payload: &Payload) -> CoreResult<()>;
}

/// Owned, erased publisher sink stored by [`EventBroker`].
///
/// `Send + Sync` on `std` (shareable across async tasks), `Send` on `no_std`.
#[cfg(not(feature = "std"))]
pub(crate) type SharedPublisherSink = Arc<dyn PublisherSink + Send>;
#[cfg(feature = "std")]
pub(crate) type SharedPublisherSink = Arc<dyn PublisherSink + Send + Sync>;

/// Fan-out table keyed by Thing and event name.
type EventSinkMap = BTreeMap<ThingId, BTreeMap<EventName, Vec<SharedPublisherSink>>>;

/// Inbound (exposed) event fan-out broker.
///
/// Holds a nested map `ThingId -> EventName -> [PublisherSink]`. All methods
/// take `&self` and mutate through an interior-mutability wrapper, matching the
/// redesign's `&self` model (addendum §2.4 / baseline §7).
///
/// # Examples
///
/// ```
/// use clinkz_wot_core::{EventBroker, PublisherSink, Payload, CoreResult};
///
/// struct ConsoleSink;
/// impl PublisherSink for ConsoleSink {
///     fn publish(&self, payload: &Payload) -> CoreResult<()> {
///         // push to the remote subscriber channel ...
///         let _ = payload;
///         Ok(())
///     }
/// }
///
/// let broker = EventBroker::new();
/// broker.register("urn:thing:1", "update", ConsoleSink);
/// ```
pub struct EventBroker {
    sinks: Arc<MapLock<EventSinkMap>>,
}

impl Clone for EventBroker {
    fn clone(&self) -> Self {
        Self {
            sinks: Arc::clone(&self.sinks),
        }
    }
}

impl EventBroker {
    /// Creates an empty event broker.
    pub fn new() -> Self {
        Self {
            sinks: Arc::new(MapLock::new(BTreeMap::new())),
        }
    }

    /// Registers a publisher sink for the given Thing and event.
    ///
    /// Takes ownership and boxes the sink internally. Duplicate registrations
    /// for the same `(thing, event)` accumulate into the fan-out list.
    #[cfg(feature = "std")]
    pub fn register<S>(&self, thing: impl Into<ThingId>, event: impl Into<EventName>, sink: S)
    where
        S: PublisherSink + Send + Sync + 'static,
    {
        self.register_shared(thing, event, Arc::new(sink));
    }

    /// Registers a publisher sink for the given Thing and event (`no_std`).
    #[cfg(not(feature = "std"))]
    pub fn register<S>(&self, thing: impl Into<ThingId>, event: impl Into<EventName>, sink: S)
    where
        S: PublisherSink + Send + 'static,
    {
        self.register_shared(thing, event, Arc::new(sink));
    }

    fn register_shared(
        &self,
        thing: impl Into<ThingId>,
        event: impl Into<EventName>,
        sink: SharedPublisherSink,
    ) {
        self.sinks.with(|map| {
            map.entry(thing.into())
                .or_default()
                .entry(event.into())
                .or_default()
                .push(sink);
        });
    }

    /// Returns the number of registered publisher sinks for an event.
    pub fn subscriber_count(&self, thing: &ThingId, event: &EventName) -> usize {
        self.sinks.with(|map| {
            map.get(thing)
                .and_then(|events| events.get(event))
                .map_or(0, Vec::len)
        })
    }

    /// Removes all publisher sinks for a Thing (baseline §10 destroy step).
    ///
    /// Called by the Servient during `destroy` so that stale publisher sinks do
    /// not linger after a Thing is removed.
    pub fn remove_thing(&self, thing: &ThingId) {
        self.sinks.with(|map| {
            map.remove(thing);
        });
    }

    /// Removes publisher sinks for a single event on a Thing.
    pub fn remove_event(&self, thing: &ThingId, event: &EventName) {
        self.sinks.with(|map| {
            if let Some(events) = map.get_mut(thing) {
                events.remove(event);
            }
        });
    }

    /// Fans `payload` out to every publisher sink registered for the event.
    ///
    /// Every sink is attempted even if an earlier one errors; the first error
    /// encountered is returned (others are still delivered). Publishing to an
    /// unknown Thing or event succeeds as a no-op.
    pub fn publish(&self, thing: &ThingId, event: &EventName, payload: &Payload) -> CoreResult<()> {
        // Fast path: peek under the lock to see if there are any subscribers
        // for this (thing, event). If not, return without allocating the
        // snapshot Vec. The common case is "no subscribers" right after expose
        // and before any consumer has subscribed, or after all consumers have
        // unsubscribed.
        let has_subscribers = self.sinks.with(|map| {
            map.get(thing)
                .and_then(|events| events.get(event))
                .is_some_and(|sinks| !sinks.is_empty())
        });
        if !has_subscribers {
            return Ok(());
        }

        // Slow path: snapshot the sink list under a brief lock, then fan-out
        // outside the lock so blocking sinks (e.g. zenoh `session.put`) don't
        // hold the broker lock.
        let snapshot: Vec<SharedPublisherSink> = self.sinks.with(|map| {
            map.get(thing)
                .and_then(|events| events.get(event))
                .map_or_else(Vec::new, |sinks| sinks.clone())
        });

        let mut first_err: Option<CoreError> = None;
        for sink in &snapshot {
            if let Err(err) = sink.publish(payload)
                && first_err.is_none()
            {
                first_err = Some(err);
            }
        }
        match first_err {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    /// Builds a broker-backed [`EventSink`] for a local event source.
    ///
    /// Each [`EventSink::emit`] on the returned sink fans the payload out to
    /// the broker's registered sinks for `(thing, event)`. The sink borrows the
    /// broker for the duration of a `subscribe` call.
    pub fn event_sink(
        &self,
        thing: impl Into<ThingId>,
        event: impl Into<EventName>,
    ) -> BrokerEventSink<'_> {
        BrokerEventSink {
            broker: self,
            thing: thing.into(),
            event: event.into(),
        }
    }
}

impl Default for EventBroker {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for EventBroker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (events, total_sinks) = self.sinks.with(|map| {
            let events = map.values().map(BTreeMap::len).sum::<usize>();
            let total_sinks = map
                .values()
                .flat_map(BTreeMap::values)
                .map(Vec::len)
                .sum::<usize>();
            (events, total_sinks)
        });
        f.debug_struct("EventBroker")
            .field("events", &events)
            .field("total_sinks", &total_sinks)
            .finish()
    }
}

/// [`EventSink`] adapter that fans emitted payloads through an [`EventBroker`].
///
/// Returned by [`EventBroker::event_sink`]. Hand it to a local
/// [`EventSubscribeHandler::subscribe`](crate::EventSubscribeHandler::subscribe) call so that
/// locally emitted events reach every registered remote subscriber.
pub struct BrokerEventSink<'a> {
    broker: &'a EventBroker,
    thing: ThingId,
    event: EventName,
}

impl EventSink for BrokerEventSink<'_> {
    fn emit(&mut self, payload: Payload) -> CoreResult<()> {
        self.broker.publish(&self.thing, &self.event, &payload)
    }
}

impl fmt::Debug for BrokerEventSink<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BrokerEventSink")
            .field("thing", &self.thing)
            .field("event", &self.event)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Outbound (consumed) subscription bounded queue.
// ---------------------------------------------------------------------------

struct QueueInner {
    buffer: VecDeque<Payload>,
    capacity: usize,
    overflow_count: u64,
    stopped: bool,
}

/// Outbound (consumed) event subscription handle.
///
/// A bounded per-subscription queue with drop-oldest + overflow-counter
/// backpressure (baseline §9 / addendum §6.1). The consumer drains it via
/// [`poll_next`](Self::poll_next); the producer (a client binding) pushes remote
/// samples via the paired [`SubscriptionSender`]. The handle is [`Clone`] and
/// shares a single queue across clones.
///
/// `poll_next` returns `None` whenever the queue is empty; use
/// [`is_stopped`](Self::is_stopped) to distinguish "no data yet" from
/// "subscription finished".
#[derive(Clone)]
pub struct Subscription {
    inner: Arc<MapLock<QueueInner>>,
}

/// Producer side of a [`Subscription`] queue.
///
/// Held by a client binding that feeds remotely pushed samples into the queue.
/// Drop-oldest backpressure means the producer is never blocked: when the queue
/// is full the oldest sample is evicted and the overflow counter is
/// incremented. Pushes after [`stop`](Self::stop) are silently dropped.
#[derive(Clone)]
pub struct SubscriptionSender {
    inner: Arc<MapLock<QueueInner>>,
}

impl Subscription {
    /// Creates a bounded channel with the given capacity.
    ///
    /// Returns the producer sender and the consumer handle sharing one queue. A
    /// `capacity` of `0` selects [`DEFAULT_SUBSCRIPTION_CAPACITY`].
    pub fn channel(capacity: usize) -> (SubscriptionSender, Self) {
        let cap = if capacity == 0 {
            DEFAULT_SUBSCRIPTION_CAPACITY
        } else {
            capacity
        };
        let inner = Arc::new(MapLock::new(QueueInner {
            buffer: VecDeque::new(),
            capacity: cap,
            overflow_count: 0,
            stopped: false,
        }));
        (
            SubscriptionSender {
                inner: Arc::clone(&inner),
            },
            Self { inner },
        )
    }

    /// Drains the next buffered payload, or `None` if the queue is empty.
    pub fn poll_next(&self) -> Option<Payload> {
        self.inner.with(|q| q.buffer.pop_front())
    }

    /// Marks the subscription as stopped.
    ///
    /// Prevents further producer pushes but leaves already-buffered samples
    /// drainable via [`poll_next`](Self::poll_next).
    pub fn stop(&self) {
        self.inner.with(|q| q.stopped = true);
    }

    /// Returns whether the subscription has been stopped.
    pub fn is_stopped(&self) -> bool {
        self.inner.with(|q| q.stopped)
    }

    /// Returns the number of samples dropped by overflow backpressure.
    pub fn overflow_count(&self) -> u64 {
        self.inner.with(|q| q.overflow_count)
    }

    /// Returns the configured queue capacity.
    pub fn capacity(&self) -> usize {
        self.inner.with(|q| q.capacity)
    }

    /// Returns the number of currently buffered samples.
    pub fn len(&self) -> usize {
        self.inner.with(|q| q.buffer.len())
    }

    /// Returns whether no samples are currently buffered.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl fmt::Debug for Subscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (len, capacity, overflow_count, stopped) = self
            .inner
            .with(|q| (q.buffer.len(), q.capacity, q.overflow_count, q.stopped));
        f.debug_struct("Subscription")
            .field("capacity", &capacity)
            .field("len", &len)
            .field("overflow_count", &overflow_count)
            .field("stopped", &stopped)
            .finish()
    }
}

impl SubscriptionSender {
    /// Pushes a remote sample into the queue with drop-oldest backpressure.
    ///
    /// When the queue is full the oldest sample is evicted and the overflow
    /// counter is incremented; the producer is never blocked. Pushes after a
    /// [`stop`](Self::stop) are silently dropped and do not count as overflow.
    pub fn push(&self, payload: Payload) {
        self.inner.with(|q| {
            if q.stopped {
                return;
            }
            if q.buffer.len() >= q.capacity {
                q.buffer.pop_front();
                q.overflow_count = q.overflow_count.saturating_add(1);
            }
            q.buffer.push_back(payload);
        });
    }

    /// Marks the subscription as stopped.
    pub fn stop(&self) {
        self.inner.with(|q| q.stopped = true);
    }

    /// Returns whether the subscription has been stopped.
    pub fn is_stopped(&self) -> bool {
        self.inner.with(|q| q.stopped)
    }

    /// Returns the number of samples dropped by overflow backpressure.
    pub fn overflow_count(&self) -> u64 {
        self.inner.with(|q| q.overflow_count)
    }
}

impl fmt::Debug for SubscriptionSender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (capacity, overflow_count, stopped) = self
            .inner
            .with(|q| (q.capacity, q.overflow_count, q.stopped));
        f.debug_struct("SubscriptionSender")
            .field("capacity", &capacity)
            .field("overflow_count", &overflow_count)
            .field("stopped", &stopped)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BrokerEventSink, DEFAULT_SUBSCRIPTION_CAPACITY, EventBroker, EventName, PublisherSink,
        Subscription,
    };
    use alloc::{string::String, string::ToString, vec, vec::Vec};
    use std::sync::{Arc, Mutex};

    use crate::thing::EventSink;
    use crate::{CoreError, CoreResult, Payload, ThingId};

    fn payload(body: &[u8]) -> Payload {
        Payload::new(body.to_vec(), "application/octet-stream")
    }

    /// Shareable recorder so a test can read a sink's deliveries after it has
    /// been boxed and moved into a broker.
    #[derive(Clone)]
    struct Recorder {
        received: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    impl Recorder {
        fn new() -> Self {
            Self {
                received: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn bodies(&self) -> Vec<Vec<u8>> {
            self.received.lock().unwrap().clone()
        }
    }

    struct RecorderSink {
        rec: Recorder,
    }

    impl PublisherSink for RecorderSink {
        fn publish(&self, p: &Payload) -> CoreResult<()> {
            self.rec.received.lock().unwrap().push(p.body.clone());
            Ok(())
        }
    }

    struct FailingSink;

    impl PublisherSink for FailingSink {
        fn publish(&self, _: &Payload) -> CoreResult<()> {
            Err(CoreError::Transport("publish failed".into()))
        }
    }

    #[test]
    fn event_name_converts_and_displays() {
        let n = EventName::from("update");
        assert_eq!(n.as_str(), "update");
        assert_eq!(n.to_string(), "update");
        assert_eq!(EventName::new(String::from("x")).into_string(), "x");
        assert_eq!(n.as_ref(), "update");
    }

    #[test]
    fn fan_out_delivers_to_all_registered_sinks() {
        let broker = EventBroker::new();
        let rec_a = Recorder::new();
        let rec_b = Recorder::new();
        broker.register(
            ThingId::from("urn:t:1"),
            "update",
            RecorderSink { rec: rec_a.clone() },
        );
        broker.register(
            ThingId::from("urn:t:1"),
            "update",
            RecorderSink { rec: rec_b.clone() },
        );

        let thing = ThingId::from("urn:t:1");
        let event = EventName::from("update");
        assert_eq!(broker.subscriber_count(&thing, &event), 2);

        broker
            .publish(&thing, &event, &payload(&[1, 2, 3]))
            .unwrap();

        assert_eq!(rec_a.bodies(), vec![vec![1, 2, 3]]);
        assert_eq!(rec_b.bodies(), vec![vec![1, 2, 3]]);
    }

    #[test]
    fn publish_to_unknown_thing_or_event_is_ok() {
        let broker = EventBroker::new();
        broker
            .publish(
                &ThingId::from("nope"),
                &EventName::from("none"),
                &payload(&[]),
            )
            .unwrap();
    }

    #[test]
    fn publish_continues_after_sink_error_and_returns_first_error() {
        let broker = EventBroker::new();
        let rec = Recorder::new();
        // Failing sink registered first; the recorder must still receive.
        broker.register("urn:t:1", "update", FailingSink);
        broker.register("urn:t:1", "update", RecorderSink { rec: rec.clone() });

        let result = broker.publish(
            &ThingId::from("urn:t:1"),
            &EventName::from("update"),
            &payload(&[9]),
        );
        assert!(result.is_err());
        assert_eq!(rec.bodies(), vec![vec![9]]);
    }

    #[test]
    fn broker_event_sink_bridges_emit_to_fanout() {
        let broker = EventBroker::new();
        let rec = Recorder::new();
        broker.register("urn:t:1", "update", RecorderSink { rec: rec.clone() });

        let mut sink: BrokerEventSink<'_> = broker.event_sink("urn:t:1", "update");
        sink.emit(payload(&[7])).unwrap();

        assert_eq!(rec.bodies(), vec![vec![7]]);
    }

    #[test]
    fn subscription_drop_oldest_increments_overflow() {
        let (sender, sub) = Subscription::channel(2);
        assert_eq!(sub.capacity(), 2);
        sender.push(payload(&[1]));
        sender.push(payload(&[2]));
        sender.push(payload(&[3])); // evict [1]
        sender.push(payload(&[4])); // evict [2]

        assert_eq!(sub.overflow_count(), 2);
        assert_eq!(sub.len(), 2);
        assert_eq!(sub.poll_next(), Some(payload(&[3])));
        assert_eq!(sub.poll_next(), Some(payload(&[4])));
        assert_eq!(sub.poll_next(), None);
        assert!(sub.is_empty());
    }

    #[test]
    fn subscription_clone_shares_one_queue() {
        let (sender, sub) = Subscription::channel(4);
        sender.push(payload(&[1]));
        let sub2 = sub.clone();

        // Draining from one clone empties the shared buffer for the other.
        assert_eq!(sub.poll_next(), Some(payload(&[1])));
        assert_eq!(sub2.poll_next(), None);
    }

    #[test]
    fn subscription_stop_prevents_push_but_allows_drain() {
        let (sender, sub) = Subscription::channel(4);
        sender.push(payload(&[1]));
        sub.stop();
        assert!(sub.is_stopped());

        sender.push(payload(&[2])); // silently dropped
        assert_eq!(sub.len(), 1);
        assert_eq!(sub.poll_next(), Some(payload(&[1])));
        assert_eq!(sub.poll_next(), None);
        assert_eq!(sender.overflow_count(), 0);
    }

    #[test]
    fn subscription_zero_capacity_uses_default() {
        let (_sender, sub) = Subscription::channel(0);
        assert_eq!(sub.capacity(), DEFAULT_SUBSCRIPTION_CAPACITY);
    }

    #[test]
    fn subscription_counts_every_overflow_drop() {
        let (sender, sub) = Subscription::channel(1);
        for byte in 0..100u8 {
            sender.push(payload(&[byte]));
        }
        assert_eq!(sub.overflow_count(), 99);
        assert_eq!(sub.poll_next(), Some(payload(&[99])));
        assert_eq!(sub.poll_next(), None);
    }

    #[test]
    fn broker_clone_shares_state() {
        let broker = EventBroker::new();
        let broker2 = broker.clone();
        let rec = Recorder::new();
        broker.register("urn:t:1", "update", RecorderSink { rec: rec.clone() });

        // Publish through the clone; the sink registered on the original sees it.
        broker2
            .publish(
                &ThingId::from("urn:t:1"),
                &EventName::from("update"),
                &payload(&[5]),
            )
            .unwrap();
        assert_eq!(rec.bodies(), vec![vec![5]]);
    }

    #[test]
    fn remove_thing_clears_all_sinks() {
        let broker = EventBroker::new();
        let rec = Recorder::new();
        broker.register("urn:t:1", "update", RecorderSink { rec: rec.clone() });
        broker.register("urn:t:1", "alert", RecorderSink { rec: rec.clone() });

        let thing = ThingId::from("urn:t:1");
        assert_eq!(
            broker.subscriber_count(&thing, &EventName::from("update")),
            1
        );
        assert_eq!(
            broker.subscriber_count(&thing, &EventName::from("alert")),
            1
        );

        broker.remove_thing(&thing);
        assert_eq!(
            broker.subscriber_count(&thing, &EventName::from("update")),
            0
        );
        assert_eq!(
            broker.subscriber_count(&thing, &EventName::from("alert")),
            0
        );

        // Publishing after removal is a no-op.
        broker
            .publish(&thing, &EventName::from("update"), &payload(&[1]))
            .unwrap();
        assert!(rec.bodies().is_empty());
    }

    #[test]
    fn remove_event_clears_single_event() {
        let broker = EventBroker::new();
        let rec = Recorder::new();
        broker.register("urn:t:1", "update", RecorderSink { rec: rec.clone() });
        broker.register("urn:t:1", "alert", RecorderSink { rec: rec.clone() });

        let thing = ThingId::from("urn:t:1");
        broker.remove_event(&thing, &EventName::from("update"));

        assert_eq!(
            broker.subscriber_count(&thing, &EventName::from("update")),
            0
        );
        assert_eq!(
            broker.subscriber_count(&thing, &EventName::from("alert")),
            1
        );

        broker
            .publish(&thing, &EventName::from("alert"), &payload(&[2]))
            .unwrap();
        assert_eq!(rec.bodies(), vec![vec![2]]);
    }
}
