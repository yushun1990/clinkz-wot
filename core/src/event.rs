//! Event fan-out and outbound subscription surfaces.
//!
//! Implements the v3.0 §9 / v3.1 §6.1 event data flow:
//!
//! - [`EventBroker`] holds a fan-out table keyed by `ThingId` and
//!   [`EventName`]. The Servient hands an observe/subscribe handler an ephemeral
//!   `FnMut(Payload)` closure (built over [`EventBroker::publish`]) so it can
//!   push initial payloads at establishment; each call fans the payload out to
//!   every registered [`PublisherSink`], each of which wraps a server binding's
//!   publish channel and pushes the payload to a remote subscriber.
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
//! On builds with the `async` feature, [`Subscription`] implements
//! [`futures_core::Stream`] so host/gateway consumers can drain it with
//! `while let Some(payload) = sub.next().await`. The `Stream` impl layers a
//! [`core::task::Waker`] notification on top of the same `VecDeque` queue, so
//! the synchronous [`Subscription::poll_next`] remains the primary drain
//! primitive on `no_std` builds and the `Stream` impl acts as a host-side push
//! adapter (see `docs/design.md`).

use alloc::{
    collections::{BTreeMap, VecDeque},
    string::String,
    sync::Arc,
    vec::Vec,
};
use core::fmt;

use crate::WotLock;
use crate::{CoreError, CoreResult, ErrorContext, ErrorPhase, Payload, RetryClass, ThingId};

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

impl From<Arc<str>> for EventName {
    fn from(name: Arc<str>) -> Self {
        Self((*name).into())
    }
}

impl AsRef<str> for EventName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl core::borrow::Borrow<str> for EventName {
    fn borrow(&self) -> &str {
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
/// `Send + Sync` on every build (v4.0 unified handler-object bounds): the
/// broker is shared across the driving loop / async tasks and its sinks must
/// be shareable across threads.
pub(crate) type SharedPublisherSink = Arc<dyn PublisherSink + Send + Sync>;

/// Fan-out table keyed by Thing and event name.
type PublisherSinkMap = BTreeMap<ThingId, BTreeMap<EventName, Arc<[SharedPublisherSink]>>>;

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
    sinks: WotLock<PublisherSinkMap>,
}

impl Clone for EventBroker {
    fn clone(&self) -> Self {
        Self {
            sinks: self.sinks.clone(),
        }
    }
}

impl EventBroker {
    /// Creates an empty event broker.
    pub fn new() -> Self {
        Self {
            sinks: WotLock::new(BTreeMap::new()),
        }
    }

    /// Registers a publisher sink for the given Thing and event.
    ///
    /// Takes ownership and boxes the sink internally. Duplicate registrations
    /// for the same `(thing, event)` accumulate into the fan-out list.
    pub fn register<S>(&self, thing: impl Into<ThingId>, event: impl Into<EventName>, sink: S)
    where
        S: PublisherSink + Send + Sync + 'static,
    {
        self.register_shared(thing, event, Arc::new(sink));
    }

    fn register_shared(
        &self,
        thing: impl Into<ThingId>,
        event: impl Into<EventName>,
        sink: SharedPublisherSink,
    ) {
        // `with` (not `with_recover`): on poison the registration is skipped
        // rather than written into potentially inconsistent state. A skipped
        // registration leaves the broker with its pre-poison fan-out table.
        self.sinks.with(|map| {
            let events = map.entry(thing.into()).or_default();
            let event = event.into();
            let snapshot = if let Some(sinks) = events.get(&event) {
                let mut sinks = sinks.to_vec();
                sinks.push(sink);
                Arc::from(sinks.into_boxed_slice())
            } else {
                let sinks = alloc::vec![sink];
                Arc::from(sinks.into_boxed_slice())
            };
            events.insert(event, snapshot);
        });
    }

    /// Returns the number of registered publisher sinks for an event.
    pub fn subscriber_count(&self, thing: &ThingId, event: &EventName) -> usize {
        self.sinks.with_recover(|map| {
            map.get(thing)
                .and_then(|events| events.get(event))
                .map_or(0, |sinks| sinks.len())
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
    /// Every sink is attempted even if an earlier one errors. A bounded
    /// publication failure is returned after fan-out completes; it uses
    /// [`RetryClass::CallerDecision`] because a failed fan-out may already have
    /// committed delivery to another sink. Publishing to an unknown Thing or
    /// event succeeds as a no-op.
    pub fn publish(&self, thing: &ThingId, event: &EventName, payload: &Payload) -> CoreResult<()> {
        // Snapshot the sink list under a brief lock, then fan-out outside the
        // lock so blocking sinks (e.g. zenoh `session.put`) don't hold the
        // broker lock.
        let snapshot: Option<Arc<[SharedPublisherSink]>> = self
            .sinks
            .with_read(|map| map.get(thing).and_then(|events| events.get(event)).cloned());
        let Some(snapshot) = snapshot else {
            return Ok(());
        };

        let mut failed = false;
        for sink in snapshot.iter() {
            if sink.publish(payload).is_err() {
                failed = true;
            }
        }
        if failed {
            Err(CoreError::Binding(ErrorContext::new(
                ErrorPhase::Delivery,
                RetryClass::CallerDecision,
            )))
        } else {
            Ok(())
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
        let (events, total_sinks) = self.sinks.with_recover(|map| {
            let events = map.values().map(BTreeMap::len).sum::<usize>();
            let total_sinks = map
                .values()
                .flat_map(BTreeMap::values)
                .map(|sinks| sinks.len())
                .sum::<usize>();
            (events, total_sinks)
        });
        f.debug_struct("EventBroker")
            .field("events", &events)
            .field("total_sinks", &total_sinks)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Outbound (consumed) subscription bounded queue.
// ---------------------------------------------------------------------------

pub(crate) struct QueueInner {
    buffer: VecDeque<Payload>,
    capacity: usize,
    overflow_count: u64,
    stopped: bool,
    /// Optional task waker notified on push/stop so the `async` `Stream` impl
    /// can park until data is available. Stays `None` whenever no task is
    /// parked on the queue (e.g. every `no_std` build without the `async`
    /// feature, or a drained-then-repolled consumer). Waking is a no-op when
    /// `None`, so keeping the field unconditional avoids `cfg` churn in the
    /// push/stop hot paths.
    waker: Option<core::task::Waker>,
}

/// Outbound (consumed) event subscription handle.
///
/// A bounded per-subscription queue with drop-oldest + overflow-counter
/// backpressure (baseline §9 / addendum §6.1). The consumer drains it via
/// [`poll_next`](Self::poll_next); the producer (a client binding) pushes remote
/// samples via the paired [`SubscriptionSender`]. The handle is [`Clone`] and
/// shares a single queue across clones.
///
/// A subscription created via [`Subscription::merge`] multiplexes across
/// multiple underlying queues (used by `subscribeAllEvents` /
/// `observeAllProperties` fan-out), draining each round-robin on
/// [`poll_next`](Self::poll_next).
///
/// `poll_next` returns `None` whenever every underlying queue is empty; use
/// [`is_stopped`](Self::is_stopped) to distinguish "no data yet" from
/// "subscription finished".
#[derive(Clone)]
pub struct Subscription {
    inner: SubscriptionInner,
}

#[derive(Clone)]
enum SubscriptionInner {
    /// Single bounded queue (the common case).
    Single(WotLock<QueueInner>),
    /// Multiplexed set of subscriptions (fan-out / "all" operations).
    Merged(Vec<Subscription>),
}

/// Producer side of a [`Subscription`] queue.
///
/// Held by a client binding that feeds remotely pushed samples into the queue.
/// Drop-oldest backpressure means the producer is never blocked: when the queue
/// is full the oldest sample is evicted and the overflow counter is
/// incremented. Pushes after [`stop`](Self::stop) are silently dropped.
#[derive(Clone)]
pub struct SubscriptionSender {
    inner: WotLock<QueueInner>,
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
        let inner = WotLock::new(QueueInner {
            buffer: VecDeque::new(),
            capacity: cap,
            overflow_count: 0,
            stopped: false,
            waker: None,
        });
        (
            SubscriptionSender {
                inner: inner.clone(),
            },
            Self {
                inner: SubscriptionInner::Single(inner),
            },
        )
    }

    /// Creates a merged subscription that multiplexes across `subs`, draining
    /// each round-robin on [`poll_next`](Self::poll_next).
    ///
    /// Used by fan-out "all" operations (`subscribeAllEvents`,
    /// `observeAllProperties`) when no Thing-level form declares the matching
    /// meta-operation and the caller fans out across individual affordances.
    /// Stopping the merged subscription stops every underlying subscription.
    /// Returns an empty (already-stopped) subscription when `subs` is empty.
    pub fn merge(subs: Vec<Subscription>) -> Self {
        if subs.is_empty() {
            let (_sender, stopped) = Self::channel(1);
            stopped.stop();
            return stopped;
        }
        if subs.len() == 1 {
            return subs.into_iter().next().expect("exactly one");
        }
        Self {
            inner: SubscriptionInner::Merged(subs),
        }
    }

    /// Drains the next buffered payload, or `None` if every underlying queue is
    /// empty.
    pub fn poll_next(&self) -> Option<Payload> {
        match &self.inner {
            SubscriptionInner::Single(inner) => inner.with_recover(|q| q.buffer.pop_front()),
            SubscriptionInner::Merged(subs) => {
                for sub in subs {
                    if let Some(payload) = sub.poll_next() {
                        return Some(payload);
                    }
                }
                None
            }
        }
    }

    /// Marks the subscription (and every underlying subscription in a merge) as
    /// stopped.
    ///
    /// Prevents further producer pushes but leaves already-buffered samples
    /// drainable via [`poll_next`](Self::poll_next).
    pub fn stop(&self) {
        match &self.inner {
            SubscriptionInner::Single(inner) => {
                let waker = inner.with_recover(|q| {
                    q.stopped = true;
                    q.waker.take()
                });
                if let Some(waker) = waker {
                    waker.wake();
                }
            }
            SubscriptionInner::Merged(subs) => {
                for sub in subs {
                    sub.stop();
                }
            }
        }
    }

    /// Returns whether the subscription has been stopped (every underlying
    /// subscription in a merge).
    pub fn is_stopped(&self) -> bool {
        match &self.inner {
            SubscriptionInner::Single(inner) => inner.with_recover(|q| q.stopped),
            SubscriptionInner::Merged(subs) => subs.iter().all(Self::is_stopped),
        }
    }

    /// Returns the number of samples dropped by overflow backpressure.
    pub fn overflow_count(&self) -> u64 {
        match &self.inner {
            SubscriptionInner::Single(inner) => inner.with_recover(|q| q.overflow_count),
            SubscriptionInner::Merged(subs) => subs.iter().map(Self::overflow_count).sum(),
        }
    }

    /// Returns the configured queue capacity.
    ///
    /// For a merged subscription this is the sum of the underlying capacities.
    pub fn capacity(&self) -> usize {
        match &self.inner {
            SubscriptionInner::Single(inner) => inner.with_recover(|q| q.capacity),
            SubscriptionInner::Merged(subs) => subs.iter().map(Self::capacity).sum(),
        }
    }

    /// Returns the number of currently buffered samples.
    pub fn len(&self) -> usize {
        match &self.inner {
            SubscriptionInner::Single(inner) => inner.with_recover(|q| q.buffer.len()),
            SubscriptionInner::Merged(subs) => subs.iter().map(Self::len).sum(),
        }
    }

    /// Returns whether no samples are currently buffered.
    ///
    /// Short-circuits on the first non-empty underlying queue so a merged
    /// subscription does not lock every queue and sum lengths.
    pub fn is_empty(&self) -> bool {
        match &self.inner {
            SubscriptionInner::Single(inner) => inner.with_read_recover(|q| q.buffer.is_empty()),
            SubscriptionInner::Merged(subs) => subs.iter().all(Self::has_buffered),
        }
    }
}

impl Subscription {
    /// Returns true when the subscription has at least one buffered sample.
    fn has_buffered(sub: &Subscription) -> bool {
        !sub.is_empty()
    }
}

impl fmt::Debug for Subscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.inner {
            SubscriptionInner::Single(inner) => {
                let (len, capacity, overflow_count, stopped) = inner
                    .with_recover(|q| (q.buffer.len(), q.capacity, q.overflow_count, q.stopped));
                f.debug_struct("Subscription")
                    .field("capacity", &capacity)
                    .field("len", &len)
                    .field("overflow_count", &overflow_count)
                    .field("stopped", &stopped)
                    .finish()
            }
            SubscriptionInner::Merged(subs) => f
                .debug_struct("Subscription")
                .field("merged", &subs.len())
                .field("len", &self.len())
                .field("stopped", &self.is_stopped())
                .finish_non_exhaustive(),
        }
    }
}

impl SubscriptionSender {
    /// Pushes a remote sample into the queue with drop-oldest backpressure.
    ///
    /// When the queue is full the oldest sample is evicted and the overflow
    /// counter is incremented; the producer is never blocked. Pushes after a
    /// [`stop`](Self::stop) are silently dropped and do not count as overflow.
    /// If a task is parked on the `async` `Stream` impl, it is woken so it can
    /// drain the pushed sample.
    pub fn push(&self, payload: Payload) {
        let waker = self.inner.with_recover(|q| {
            if q.stopped {
                return None;
            }
            if q.buffer.len() >= q.capacity {
                q.buffer.pop_front();
                q.overflow_count = q.overflow_count.saturating_add(1);
            }
            q.buffer.push_back(payload);
            q.waker.clone()
        });
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    /// Marks the subscription as stopped.
    ///
    /// Wakes any task parked on the `async` `Stream` impl so it observes the
    /// terminal state instead of parking forever.
    pub fn stop(&self) {
        let waker = self.inner.with_recover(|q| {
            q.stopped = true;
            q.waker.take()
        });
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    /// Returns whether the subscription has been stopped.
    pub fn is_stopped(&self) -> bool {
        self.inner.with_recover(|q| q.stopped)
    }

    /// Returns the number of samples dropped by overflow backpressure.
    pub fn overflow_count(&self) -> u64 {
        self.inner.with_recover(|q| q.overflow_count)
    }
}

impl fmt::Debug for SubscriptionSender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (capacity, overflow_count, stopped) = self
            .inner
            .with_recover(|q| (q.capacity, q.overflow_count, q.stopped));
        f.debug_struct("SubscriptionSender")
            .field("capacity", &capacity)
            .field("overflow_count", &overflow_count)
            .field("stopped", &stopped)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// `async` `Stream` adapter for outbound subscriptions.
// ---------------------------------------------------------------------------

/// Host-side push adapter layered on top of the synchronous
/// [`Subscription::poll_next`] queue.
///
/// Behind the `async` feature, [`Subscription`] implements
/// [`futures_core::Stream`]. The implementation parks a task by registering a
/// [`core::task::Waker`] under the same [`WotLock`] that guards the queue, and
/// [`SubscriptionSender::push`] / [`Subscription::stop`] wake it. This keeps the
/// queue the single source of truth (no second channel primitive) and leaves the
/// synchronous surface usable on `no_std`.
#[cfg(feature = "async")]
impl Subscription {
    /// Collects every underlying single queue, flattening nested merges.
    ///
    /// Async-only helper shared by `Subscription`'s own `Stream` impl and by
    /// `EventStream`'s `Stream` impl. Returns one [`WotLock`] handle per
    /// underlying leaf queue so callers can register a waker / drain samples
    /// without re-walking the merge tree on every poll.
    pub(crate) fn collect_leaves(&self, out: &mut alloc::vec::Vec<WotLock<QueueInner>>) {
        match &self.inner {
            SubscriptionInner::Single(inner) => out.push(inner.clone()),
            SubscriptionInner::Merged(subs) => {
                for sub in subs {
                    sub.collect_leaves(out);
                }
            }
        }
    }
}

#[cfg(feature = "async")]
mod stream_impl {
    use alloc::vec::Vec;
    use core::pin::Pin;
    use core::task::{Context, Poll};

    use futures_core::Stream;

    use crate::WotLock;
    use crate::payload::Payload;

    use super::{QueueInner, Subscription, SubscriptionInner};

    impl Subscription {
        // `collect_leaves` is now defined at the top level of the async
        // section below; both `stream_impl` and `event_stream_impl` reach
        // it through `crate::event::Subscription::collect_leaves`.
    }

    impl Stream for Subscription {
        type Item = Payload;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            // Fast path: the overwhelmingly common single-queue case skips the
            // per-poll `Vec` allocation entirely.
            if let SubscriptionInner::Single(inner) = &self.inner {
                return poll_single(inner, cx);
            }

            let mut leaves: Vec<WotLock<QueueInner>> = Vec::new();
            self.collect_leaves(&mut leaves);
            poll_leaves(&leaves, cx)
        }
    }

    fn poll_single(inner: &WotLock<QueueInner>, cx: &mut Context<'_>) -> Poll<Option<Payload>> {
        let drained = inner.with_recover(|q| {
            if let Some(payload) = q.buffer.pop_front() {
                q.waker = None;
                Some(payload)
            } else {
                None
            }
        });
        if let Some(payload) = drained {
            return Poll::Ready(Some(payload));
        }
        let mut all_stopped = true;
        inner.with_recover(|q| {
            let needs_register = q.waker.as_ref().is_none_or(|w| !w.will_wake(cx.waker()));
            if needs_register {
                q.waker = Some(cx.waker().clone());
            }
            if !q.stopped {
                all_stopped = false;
            }
        });
        if all_stopped {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }

    fn poll_leaves(leaves: &[WotLock<QueueInner>], cx: &mut Context<'_>) -> Poll<Option<Payload>> {
        // Round-robin drain: if any leaf has a buffered sample, return it
        // and drop the wake slot on that queue (progress was made).
        for inner in leaves {
            let drained = inner.with_recover(|q| {
                if let Some(payload) = q.buffer.pop_front() {
                    q.waker = None;
                    Some(payload)
                } else {
                    None
                }
            });
            if let Some(payload) = drained {
                return Poll::Ready(Some(payload));
            }
        }

        // Every leaf is empty. Register the caller's waker on each leaf (so
        // the next push to any queue wakes this task) and determine whether
        // the whole subscription is terminal. `will_wake` avoids redundant
        // waker clones on repeated polls with the same task.
        let mut all_stopped = true;
        for inner in leaves {
            inner.with_recover(|q| {
                let needs_register = q.waker.as_ref().is_none_or(|w| !w.will_wake(cx.waker()));
                if needs_register {
                    q.waker = Some(cx.waker().clone());
                }
                if !q.stopped {
                    all_stopped = false;
                }
            });
        }

        if all_stopped {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

// ---------------------------------------------------------------------------
// `EventStream`: merged event-name-tagged subscription stream (P2).
// ---------------------------------------------------------------------------

/// Merged event-name-tagged stream returned by
/// `ConsumedThingHandle::subscribe_all_events`.
///
/// Wraps a `Vec<(EventName, Subscription)>` and yields `(EventName, Payload)`
/// tuples in round-robin order across the underlying event streams. Distinct
/// from [`Subscription::merge`] (which yields bare `Payload`) because the
/// caller of `subscribe_all_events` needs to know which event each sample
/// belongs to.
///
/// Like [`Subscription`], this type is `no_std + alloc` compatible in its
/// synchronous surface ([`poll_next`](Self::poll_next),
/// [`stop`](Self::stop)); the
/// [`futures_core::Stream`] implementation is available behind the `async`
/// feature.
#[cfg(feature = "async")]
pub struct EventStream {
    entries: alloc::vec::Vec<(EventName, Subscription)>,
}

#[cfg(feature = "async")]
impl EventStream {
    /// Creates an event stream from already-paired `(EventName, Subscription)`
    /// entries.
    ///
    /// Used by `ConsumedThingHandle::subscribe_all_events`, which produces
    /// one entry per event declared by the consumed Thing. An empty `entries`
    /// yields a stream that immediately returns `None`.
    pub fn new(entries: alloc::vec::Vec<(EventName, Subscription)>) -> Self {
        Self { entries }
    }

    /// Drains the next tagged payload across all underlying subscriptions,
    /// round-robin, or `None` when every queue is empty.
    ///
    /// Use [`is_stopped`](Self::is_stopped) to distinguish "no data yet"
    /// from "every underlying subscription has stopped".
    pub fn poll_next(&self) -> Option<(EventName, Payload)> {
        for (name, sub) in &self.entries {
            if let Some(payload) = sub.poll_next() {
                return Some((name.clone(), payload));
            }
        }
        None
    }

    /// Stops every underlying subscription.
    ///
    /// Already-buffered samples remain drainable via
    /// [`poll_next`](Self::poll_next); only future producer pushes are
    /// suppressed.
    pub fn stop(&self) {
        for (_, sub) in &self.entries {
            sub.stop();
        }
    }

    /// Returns `true` when every underlying subscription is stopped.
    pub fn is_stopped(&self) -> bool {
        self.entries.iter().all(|(_, s)| s.is_stopped())
    }

    /// Returns the number of underlying event subscriptions in this stream.
    pub fn event_count(&self) -> usize {
        self.entries.len()
    }

    /// Returns the names of the events multiplexed by this stream, in
    /// construction order.
    pub fn event_names(&self) -> alloc::vec::Vec<&EventName> {
        self.entries.iter().map(|(n, _)| n).collect()
    }
}

#[cfg(feature = "async")]
impl core::fmt::Debug for EventStream {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EventStream")
            .field("event_count", &self.entries.len())
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "async")]
mod event_stream_impl {
    use super::EventStream;
    use crate::payload::Payload;

    use alloc::vec::Vec;
    use core::pin::Pin;
    use core::task::{Context, Poll};

    use futures_core::Stream;

    impl Stream for EventStream {
        type Item = (super::EventName, Payload);

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            // Drain ready samples first; only register the waker when no
            // underlying queue had a sample this pass.
            for (name, sub) in &self.entries {
                if let Some(payload) = sub.poll_next() {
                    return Poll::Ready(Some((name.clone(), payload)));
                }
            }
            // Register the waker on every underlying queue so any future
            // push wakes this task.
            let mut leaves: Vec<crate::WotLock<super::QueueInner>> = Vec::new();
            for (_, sub) in &self.entries {
                sub.collect_leaves(&mut leaves);
            }
            let mut all_stopped = true;
            for inner in &leaves {
                inner.with_recover(|q| {
                    let needs_register = q.waker.as_ref().is_none_or(|w| !w.will_wake(cx.waker()));
                    if needs_register {
                        q.waker = Some(cx.waker().clone());
                    }
                    if !q.stopped {
                        all_stopped = false;
                    }
                });
            }
            if all_stopped {
                Poll::Ready(None)
            } else {
                Poll::Pending
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_SUBSCRIPTION_CAPACITY, EventBroker, EventName, PublisherSink, Subscription,
    };
    use alloc::{string::String, string::ToString, vec, vec::Vec};
    use std::sync::{Arc, Mutex};

    use crate::{CoreError, CoreResult, ErrorContext, ErrorPhase, Payload, RetryClass, ThingId};

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
            self.rec
                .received
                .lock()
                .unwrap()
                .push(p.body.as_ref().to_vec());
            Ok(())
        }
    }

    struct FailingSink;

    impl PublisherSink for FailingSink {
        fn publish(&self, _: &Payload) -> CoreResult<()> {
            Err(CoreError::Binding(ErrorContext::new(
                ErrorPhase::Delivery,
                RetryClass::Safe,
            )))
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
    fn publish_continues_after_sink_error_and_requires_caller_retry_decision() {
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
        assert!(matches!(result, Err(CoreError::Binding(context))
            if context.phase() == ErrorPhase::Delivery
                && context.retry_class() == RetryClass::CallerDecision));
        assert_eq!(rec.bodies(), vec![vec![9]]);
    }

    #[test]
    fn publish_returns_a_bounded_failure_after_attempting_all_sinks() {
        let broker = EventBroker::new();
        // The public error remains bounded even when multiple subscribers fail.
        broker.register("urn:t:1", "update", FailingSink);
        broker.register("urn:t:1", "update", FailingSink);

        let err = broker
            .publish(
                &ThingId::from("urn:t:1"),
                &EventName::from("update"),
                &payload(&[1]),
            )
            .expect_err("two failing sinks must surface an error");
        assert!(matches!(err, CoreError::Binding(context)
            if context.phase() == ErrorPhase::Delivery
                && context.retry_class() == RetryClass::CallerDecision));
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

#[cfg(all(test, feature = "async"))]
mod stream_tests {
    use alloc::vec;
    use core::pin::Pin;
    use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    use futures_core::Stream;
    use futures_util::StreamExt;

    use super::Subscription;
    use crate::Payload;

    fn payload(body: &[u8]) -> Payload {
        Payload::new(body.to_vec(), "application/octet-stream")
    }

    /// Builds a no-op [`Waker`] so [`Subscription`] can be polled without an
    /// async runtime. Sufficient for asserting `Ready`/`Pending` returns; the
    /// cross-task wake path is covered by the `#[tokio::test]` cases below.
    fn noop_waker() -> Waker {
        fn clone(ptr: *const ()) -> RawWaker {
            RawWaker::new(ptr, &VTABLE)
        }
        fn wake(_: *const ()) {}
        fn wake_by_ref(_: *const ()) {}
        fn drop(_: *const ()) {}
        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
        // Safety: the vtable functions are valid no-ops and the stored pointer
        // is never dereferenced.
        unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &VTABLE)) }
    }

    fn poll_once(sub: &mut Subscription) -> Poll<Option<Payload>> {
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        // Fully-qualified call: the inherent `Subscription::poll_next(&self)`
        // would otherwise shadow the `Stream::poll_next` trait method.
        Stream::poll_next(Pin::new(sub), &mut cx)
    }

    #[test]
    fn poll_returns_ready_with_buffered_item() {
        let (sender, mut sub) = Subscription::channel(4);
        sender.push(payload(&[1]));
        sender.push(payload(&[2]));

        assert_eq!(poll_once(&mut sub), Poll::Ready(Some(payload(&[1]))));
        assert_eq!(poll_once(&mut sub), Poll::Ready(Some(payload(&[2]))));
        // Buffer now empty but subscription is live.
        assert_eq!(poll_once(&mut sub), Poll::Pending);
    }

    #[test]
    fn poll_returns_pending_until_pushed() {
        let (sender, mut sub) = Subscription::channel(4);
        assert_eq!(poll_once(&mut sub), Poll::Pending);

        sender.push(payload(&[7]));
        assert_eq!(poll_once(&mut sub), Poll::Ready(Some(payload(&[7]))));
        assert_eq!(poll_once(&mut sub), Poll::Pending);
    }

    #[test]
    fn poll_returns_none_when_stopped_and_empty() {
        let (sender, mut sub) = Subscription::channel(4);
        sender.push(payload(&[1]));

        // Buffered items remain drainable after stop (matches poll_next).
        assert_eq!(poll_once(&mut sub), Poll::Ready(Some(payload(&[1]))));
        sender.stop();
        assert_eq!(poll_once(&mut sub), Poll::Ready(None));
    }

    #[test]
    fn poll_returns_none_immediately_for_empty_stopped_channel() {
        let (_sender, mut sub) = Subscription::channel(4);
        sub.stop();
        assert_eq!(poll_once(&mut sub), Poll::Ready(None));
    }

    #[tokio::test]
    async fn push_wakes_parked_async_consumer() {
        let (sender, mut sub) = Subscription::channel(4);

        // Park a consumer in a spawned task, then push from the main task.
        let join = tokio::spawn(async move { sub.next().await });

        // Give the spawned consumer a chance to reach the parked `.await`.
        tokio::time::sleep(core::time::Duration::from_millis(5)).await;

        sender.push(payload(&[9]));
        assert_eq!(join.await.unwrap(), Some(payload(&[9])));
    }

    #[tokio::test]
    async fn stop_wakes_parked_async_consumer() {
        let (sender, mut sub) = Subscription::channel(4);

        let join = tokio::spawn(async move { sub.next().await });

        tokio::time::sleep(core::time::Duration::from_millis(5)).await;

        sender.stop();
        assert_eq!(join.await.unwrap(), None);
    }

    #[tokio::test]
    async fn merged_stream_round_robins_and_terminates() {
        let (s1, sub1) = Subscription::channel(4);
        let (s2, sub2) = Subscription::channel(4);
        let mut merged = Subscription::merge(vec![sub1, sub2]);

        s1.push(payload(&[10]));
        s2.push(payload(&[20]));

        // Round-robin drains leaf 0 before leaf 1.
        assert_eq!(merged.next().await, Some(payload(&[10])));
        assert_eq!(merged.next().await, Some(payload(&[20])));

        // Both leaves empty -> parks. Pushing to either wakes the merged task.
        let s1_clone = s1.clone();
        let push_task = tokio::spawn(async move {
            tokio::time::sleep(core::time::Duration::from_millis(5)).await;
            s1_clone.push(payload(&[30]));
        });

        assert_eq!(merged.next().await, Some(payload(&[30])));
        push_task.await.unwrap();

        // Stopping both leaves terminates the merged stream when empty.
        s1.stop();
        s2.stop();
        assert_eq!(merged.next().await, None);
    }

    #[tokio::test]
    async fn merged_stream_park_wakes_from_either_leaf() {
        let (s2, sub2) = Subscription::channel(4);
        let (_s1, sub1) = Subscription::channel(4);
        let mut merged = Subscription::merge(vec![sub1, sub2]);

        let s2_clone = s2.clone();
        let push_task = tokio::spawn(async move {
            tokio::time::sleep(core::time::Duration::from_millis(5)).await;
            // Push to the second leaf; the merged task must wake even though the
            // first leaf is the one polled first.
            s2_clone.push(payload(&[40]));
        });

        assert_eq!(merged.next().await, Some(payload(&[40])));
        push_task.await.unwrap();
    }
}
