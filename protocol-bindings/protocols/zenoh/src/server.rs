//! Zenoh server-side binding — serves exposed Things over a shared zenoh
//! session (baseline v3.0 §1, §12–§13, §10).
//!
//! [`ZenohServerBinding`] implements [`ServerBinding`](clinkz_wot_core::ServerBinding)
//! by declaring zenoh primitives on the same `zenoh::Session` used for outbound
//! interactions. Inbound operations are mapped per baseline §13:
//!
//! | WoT operation            | Server side (zenoh)   |
//! |--------------------------|-----------------------|
//! | `readproperty`           | `declare_queryable`   |
//! | `invokeaction`           | `declare_queryable`   |
//! | `writeproperty`          | put listener (subscriber) |
//! | `observeproperty`        | `PublisherSink` → `session.put` |
//! | `subscribeevent`         | `PublisherSink` → `session.put` |
//!
//! Event and observable-property publishing is wired through the shared
//! [`EventBroker`]: at `register_thing` time the binding registers a
//! [`ZenohPublisherSink`] for each event/observe key expression. When
//! `emit_event` or `observe_property` pushes a payload through the broker, the
//! sink calls `session.put` on the matching zenoh key expression, delivering
//! the sample to every remote subscriber.
//!
//! Route lifecycle is driven by the Servient: [`ServerBinding::register_thing`]
//! is called during `expose`, [`ServerBinding::unregister_thing`] during
//! `destroy`.

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[cfg(feature = "async")]
use clinkz_wot_core::AsyncServerBinding;
use clinkz_wot_core::identity::CorrelationId;
use clinkz_wot_core::{
    AffordanceTarget, AuthMaterial, EventBroker, EventName, InboundRequest, InboundResponse,
    InteractionInput, Payload, PublisherSink, ServerBinding, ThingId,
};
use clinkz_wot_td::data_type::Operation;
use clinkz_wot_td::form::Form;
use clinkz_wot_td::td_defaults::{FormContext, effective_form_operations};
use clinkz_wot_td::thing::Thing;
use zenoh::Wait;
use zenoh::bytes::Encoding;
use zenoh::pubsub::Subscriber;
use zenoh::query::{Query, Queryable};
use zenoh::sample::{Sample, SampleKind};

use crate::{
    ZenohBindingResult, ZenohFormTarget, ZenohOperationKind, ZenohOperationPlan,
    extract_zenoh_metadata, try_extract_zenoh_target, zenoh_operation_kind,
};

// ---------------------------------------------------------------------------
// Internal state
// ---------------------------------------------------------------------------

/// Static metadata captured per inbound route so the zenoh callback can build
/// a fully-formed [`InboundRequest`] without additional lookups.
#[derive(Clone)]
struct RouteMeta {
    thing_id: String,
    target: AffordanceTarget,
    operation: Operation,
    /// What the zenoh transport can extract for this route's effective
    /// security scheme. Resolved from the TD at route-planning time so the
    /// attachment is interpreted correctly (or refused) per route.
    auth_expectation: AuthExpectation,
}

/// What the zenoh transport can extract for a route's effective security
/// scheme.
///
/// The zenoh attachment is a single opaque byte buffer, so the transport can
/// only directly carry a bearer token. Routes that declare a non-bearer scheme
/// (Basic / OAuth2 / PSK / ApiKey / ...) cannot be authenticated via a zenoh
/// attachment and are reported as [`AuthExpectation::Unsupported`] — the
/// request is then treated as unauthenticated instead of misclassifying
/// arbitrary attachment bytes as a bearer token (the previous behavior, which
/// silently misfed verification).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AuthExpectation {
    /// No authentication required (NoSec or no security declared).
    None,
    /// Bearer scheme — attachment bytes are extracted as a bearer token.
    Bearer,
    /// A scheme the zenoh transport cannot extract from an attachment. The
    /// attachment is ignored and the request is treated as unauthenticated.
    Unsupported,
}

/// Upper bound on an accepted bearer-token attachment, to prevent unbounded
/// attachments from being wrapped as auth material.
const MAX_BEARER_TOKEN_BYTES: usize = 8 * 1024;

/// Resolves the route-level [`AuthExpectation`] from the TD's effective form
/// security. If multiple schemes are declared, Bearer wins (the transport can
/// extract it); any non-NoSec/non-Bearer scheme marks the route Unsupported.
fn resolve_auth_expectation(td: &Thing, form: &Form) -> AuthExpectation {
    use clinkz_wot_protocol_bindings::resolve_form_security;
    use clinkz_wot_td::security_scheme::SecurityScheme;

    let effective = resolve_form_security(td, form);
    let mut saw_unsupported = false;
    for name in effective.security {
        match td.security_definitions.get(name.as_str()) {
            Some(SecurityScheme::NoSec(_)) => continue,
            Some(SecurityScheme::Bearer(_)) => return AuthExpectation::Bearer,
            Some(_) => saw_unsupported = true,
            None => continue,
        }
    }
    if saw_unsupported {
        AuthExpectation::Unsupported
    } else {
        AuthExpectation::None
    }
}

/// How to deliver a response back to the zenoh requester.
enum ReplyTarget {
    /// Zenoh query expecting a reply (readproperty, invokeaction).
    Query { query: Query, key_expr: String },
    /// PUT sample — fire-and-forget, no reply expected (writeproperty).
    Put,
}

/// Maximum number of inbound requests buffered for the synchronous driving
/// loop. When exceeded, the oldest entry is dropped first (drop-oldest
/// backpressure) so a slow driver cannot grow the queue without bound.
const PENDING_QUEUE_CAPACITY: usize = 256;

/// Lifetime after which an unclaimed [`ReplyTarget`] is considered abandoned
/// and is evicted to release the underlying zenoh resource (e.g. a live
/// `zenoh::Query`). Measured from the instant the reply target is registered
/// (at callback arrival time). See [`sweep_expired_reply_targets`].
const REPLY_TARGET_TTL: Duration = Duration::from_secs(30);

/// Capacity of the tokio mpsc channel that feeds the async driving loop. The
/// channel is bounded so that an async-compiled binding that is driven
/// synchronously cannot leak unconsumed wakeups; under genuine async driving
/// the receiver drains it.
#[cfg(feature = "async")]
const ASYNC_CHANNEL_CAPACITY: usize = 256;

/// A registered [`ReplyTarget`] paired with the instant it was recorded, so
/// abandoned entries (handler error, panic, or a request dropped before being
/// polled) can be evicted via [`sweep_expired_reply_targets`] instead of
/// leaking the live zenoh resource forever.
struct ReplyTargetEntry {
    reply: ReplyTarget,
    inserted_at: Instant,
}

/// Declared zenoh handles for one Thing, stored for cleanup.
//
// The concrete handler types from zenoh's `.callback()` builder differ across
// zenoh versions; we erase them behind `Box<dyn Send>` and call `undeclare`
// through the type-erased `Undeclare` trait.
trait RouteHandle: Send {
    /// Undeclares the zenoh primitive, blocking on `.wait()`.
    ///
    /// This blocks the calling thread until zenoh acknowledges the
    /// undeclaration. Route cleanup is normally driven explicitly via
    /// [`ZenohServerBinding::unregister_thing`] (called from
    /// [`ServerBinding::unregister_thing`]); no `Drop` impl on the server
    /// binding relies on this path.
    fn undeclare_boxed(self: Box<Self>);
}

impl<H> RouteHandle for Queryable<H>
where
    H: Send,
{
    fn undeclare_boxed(self: Box<Self>) {
        if let Err(e) = Queryable::undeclare(*self).wait() {
            log::warn!("Zenoh server: failed to undeclare queryable route: {e}");
        }
    }
}

impl<H> RouteHandle for Subscriber<H>
where
    H: Send,
{
    fn undeclare_boxed(self: Box<Self>) {
        if let Err(e) = Subscriber::undeclare(*self).wait() {
            log::warn!("Zenoh server: failed to undeclare put-listener route: {e}");
        }
    }
}

enum DeclaredRoute {
    Queryable(Box<dyn RouteHandle>),
    PutListener(Box<dyn RouteHandle>),
}

/// Declared inbound routes for one Thing, keyed by affordance
/// (`affordance_key(target)`), so individual affordances can be
/// registered/unregistered at runtime (W3C dynamic affordance lifecycle)
/// without re-declaring the whole Thing's routes.
type ThingRoutes = BTreeMap<String, Vec<DeclaredRoute>>;

struct ServerState {
    routes: BTreeMap<String, ThingRoutes>,
    pending: VecDeque<InboundRequest>,
    reply_targets: BTreeMap<CorrelationId, ReplyTargetEntry>,
    next_correlation: AtomicU64,
    event_broker: Option<EventBroker>,
}

impl ServerState {
    fn new() -> Self {
        Self {
            routes: BTreeMap::new(),
            pending: VecDeque::new(),
            reply_targets: BTreeMap::new(),
            next_correlation: AtomicU64::new(1),
            event_broker: None,
        }
    }
}

// ---------------------------------------------------------------------------
// ZenohServerBinding
// ---------------------------------------------------------------------------

/// Zenoh server binding sharing a [`zenoh::Session`] for both inbound serving
/// and outbound interactions (baseline v3.0 §1, §13).
///
/// During `Servient::expose` the Servient calls [`ServerBinding::register_thing`]
/// for each Thing, causing zenoh queryables and put-listeners to be declared on
/// the shared session. The Servient driving loop polls
/// [`poll_accept_sync`](ServerBinding::poll_accept_sync) to drain inbound
/// requests; responses are written back via
/// [`send_response`](ServerBinding::send_response).
pub struct ZenohServerBinding {
    session: zenoh::Session,
    routes: Arc<Mutex<BTreeMap<String, ThingRoutes>>>,
    pending: Arc<Mutex<VecDeque<InboundRequest>>>,
    reply_targets: Arc<Mutex<BTreeMap<CorrelationId, ReplyTargetEntry>>>,
    event_broker: Arc<Mutex<Option<EventBroker>>>,
    next_correlation: Arc<AtomicU64>,
    #[cfg(feature = "async")]
    async_tx: Arc<tokio::sync::mpsc::Sender<InboundRequest>>,
    #[cfg(feature = "async")]
    async_rx: Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<InboundRequest>>>,
}

impl ZenohServerBinding {
    /// Creates a server binding from an existing zenoh session.
    ///
    /// The session is shared — the same session can be used for outbound
    /// interactions via [`crate::ZenohSessionTransport`].
    pub fn new(session: zenoh::Session) -> Self {
        let state = ServerState::new();
        #[cfg(feature = "async")]
        let (async_tx, async_rx) = tokio::sync::mpsc::channel(ASYNC_CHANNEL_CAPACITY);
        Self {
            session,
            routes: Arc::new(Mutex::new(state.routes)),
            pending: Arc::new(Mutex::new(state.pending)),
            reply_targets: Arc::new(Mutex::new(state.reply_targets)),
            event_broker: Arc::new(Mutex::new(state.event_broker)),
            next_correlation: Arc::new(state.next_correlation),
            #[cfg(feature = "async")]
            async_tx: Arc::new(async_tx),
            #[cfg(feature = "async")]
            async_rx: Arc::new(tokio::sync::Mutex::new(async_rx)),
        }
    }

    /// Opens a zenoh session and creates a server binding from it.
    pub fn open(config: zenoh::Config) -> std::io::Result<Self> {
        let session = zenoh::open(config).wait().map_err(io_error)?;
        Ok(Self::new(session))
    }

    /// Returns a reference to the underlying zenoh session.
    pub fn session(&self) -> &zenoh::Session {
        &self.session
    }
}

// ---------------------------------------------------------------------------
// ServerBinding implementation
// ---------------------------------------------------------------------------

impl ServerBinding for ZenohServerBinding {
    fn poll_accept_sync(&self) -> Option<InboundRequest> {
        if let Ok(mut reply_targets) = self.reply_targets.lock() {
            sweep_expired_reply_targets(&mut reply_targets);
        }
        self.pending.lock().ok()?.pop_front()
    }

    fn send_response(&self, response: InboundResponse) {
        let reply_target = match self.reply_targets.lock() {
            Ok(mut reply_targets) => {
                sweep_expired_reply_targets(&mut reply_targets);
                reply_targets
                    .remove(&response.correlation)
                    .map(|entry| entry.reply)
            }
            Err(_) => return,
        };

        match reply_target {
            Some(ReplyTarget::Query { query, key_expr }) => {
                if let Some(err) = response.error {
                    let status = clinkz_wot_protocol_bindings::error_status(&err);
                    if let Err(e) = query.reply_err(format!("[{}] {}", status, err)).wait() {
                        log::warn!("Zenoh server: failed to send error reply: {e}");
                    }
                } else {
                    let (payload_body, content_type) = match response.output.payload {
                        Some(payload) => (payload.body, payload.content_type),
                        None => (Vec::new(), String::new()),
                    };
                    let mut builder = query.reply(key_expr.as_str(), payload_body);
                    if !content_type.is_empty() {
                        builder = builder.encoding(Encoding::from(content_type.as_str()));
                    }
                    if let Err(e) = builder.wait() {
                        log::warn!("Zenoh server: failed to send reply: {e}");
                    }
                }
            }
            Some(ReplyTarget::Put) | None => { /* no reply needed */ }
        }
    }

    fn register_thing(&self, thing_id: &str, td: &Thing) -> Result<(), String> {
        let routes = plan_inbound_routes(thing_id, td)?;
        let broker = self.event_broker.lock().map_err(|e| e.to_string())?.clone();
        let mut by_affordance: ThingRoutes = BTreeMap::new();

        for route in routes {
            let key = affordance_key(&route.meta.target);
            match self.declare_planned_route(route, thing_id, &broker) {
                Ok(Some(declared)) => by_affordance.entry(key).or_default().push(declared),
                Ok(None) => {}
                Err(err) => {
                    for (_, declared) in by_affordance {
                        undeclare_routes(declared);
                    }
                    return Err(err);
                }
            }
        }

        self.routes
            .lock()
            .map_err(|e| e.to_string())?
            .insert(thing_id.to_string(), by_affordance);
        Ok(())
    }

    fn unregister_thing(&self, thing_id: &str) {
        let broker = match self.event_broker.lock() {
            Ok(broker) => broker.clone(),
            Err(_) => return,
        };
        if let Some(ref broker) = broker {
            broker.remove_thing(&ThingId::from(thing_id));
        }
        let by_affordance = match self.routes.lock() {
            Ok(mut routes) => routes.remove(thing_id),
            Err(_) => return,
        };
        if let Some(by_affordance) = by_affordance {
            for (_, declared) in by_affordance {
                undeclare_routes(declared);
            }
        }
    }

    fn register_affordance(
        &self,
        thing_id: &str,
        target: &AffordanceTarget,
        td: &Thing,
    ) -> Result<(), String> {
        let key = affordance_key(target);
        let broker = self.event_broker.lock().map_err(|e| e.to_string())?.clone();
        // Plan all routes, keep only those belonging to this affordance.
        let planned = plan_inbound_routes(thing_id, td)?
            .into_iter()
            .filter(|r| affordance_key(&r.meta.target) == key);
        let mut declared = Vec::new();
        for route in planned {
            match self.declare_planned_route(route, thing_id, &broker) {
                Ok(Some(d)) => declared.push(d),
                Ok(None) => {}
                Err(err) => {
                    undeclare_routes(declared);
                    return Err(err);
                }
            }
        }
        // Insert/replace under a brief lock. `insert` returns any prior entry
        // for this affordance, which we undeclare to avoid leaking it.
        let prior = self
            .routes
            .lock()
            .map_err(|e| e.to_string())?
            .entry(thing_id.to_string())
            .or_default()
            .insert(key, declared);
        if let Some(prior) = prior {
            undeclare_routes(prior);
        }
        Ok(())
    }

    fn unregister_affordance(&self, thing_id: &str, target: &AffordanceTarget) {
        let key = affordance_key(target);
        // Drop broker sinks for event / observable-property affordances.
        let broker_name = match target {
            AffordanceTarget::Event(name) | AffordanceTarget::Property(name) => Some(name.as_str()),
            _ => None,
        };
        if let Some(name) = broker_name
            && let Ok(broker) = self.event_broker.lock()
            && let Some(broker) = broker.as_ref()
        {
            broker.remove_event(&ThingId::from(thing_id), &EventName::from(name));
        }
        let removed = self.routes.lock().ok().and_then(|mut routes| {
            routes
                .get_mut(thing_id)
                .and_then(|thing_routes| thing_routes.remove(&key))
        });
        if let Some(removed) = removed {
            undeclare_routes(removed);
        }
    }

    fn set_event_broker(&self, broker: EventBroker) {
        if let Ok(mut event_broker) = self.event_broker.lock() {
            *event_broker = Some(broker);
        }
    }
}

impl ZenohServerBinding {
    /// Declares a single planned zenoh route. Returns `Ok(Some(route))` for
    /// Queryable/PutListener (which need explicit undeclaration), `Ok(None)`
    /// for Publisher routes (broker-managed via `PublisherSink` registration).
    fn declare_planned_route(
        &self,
        route: PlannedRoute,
        thing_id: &str,
        broker: &Option<EventBroker>,
    ) -> Result<Option<DeclaredRoute>, String> {
        match route.kind {
            RouteKind::Queryable { key_expr } => {
                let pending = Arc::clone(&self.pending);
                let reply_targets = Arc::clone(&self.reply_targets);
                let next_correlation = Arc::clone(&self.next_correlation);
                let meta = route.meta.clone();
                let key_for_reply = key_expr.clone();
                #[cfg(feature = "async")]
                let async_tx = Arc::clone(&self.async_tx);

                let queryable = self
                    .session
                    .declare_queryable(key_expr.as_str())
                    .callback(move |query| {
                        if let Some(request) = handle_query(
                            &reply_targets,
                            &next_correlation,
                            &meta,
                            &key_for_reply,
                            query,
                        ) {
                            #[cfg(feature = "async")]
                            {
                                if let Err(e) = async_tx.try_send(request) {
                                    log::warn!(
                                        "Zenoh server: failed to enqueue inbound request \
                                         (channel full): {e}"
                                    );
                                }
                            }
                            #[cfg(not(feature = "async"))]
                            {
                                if let Ok(mut pending) = pending.lock()
                                    && let Some(dropped) = push_bounded(&mut pending, request)
                                {
                                    log::warn!(
                                        "Zenoh server: pending queue full; dropping oldest request"
                                    );
                                    send_drop_reply(&reply_targets, &dropped.correlation);
                                }
                            }
                        }
                    })
                    .wait()
                    .map_err(|err| format!("zenoh queryable declaration failed: {err}"))?;
                Ok(Some(DeclaredRoute::Queryable(Box::new(queryable))))
            }
            RouteKind::PutListener { key_expr } => {
                let pending = Arc::clone(&self.pending);
                let reply_targets = Arc::clone(&self.reply_targets);
                let next_correlation = Arc::clone(&self.next_correlation);
                let meta = route.meta.clone();
                #[cfg(feature = "async")]
                let async_tx = Arc::clone(&self.async_tx);

                let subscriber = self
                    .session
                    .declare_subscriber(key_expr.as_str())
                    .callback(move |sample| {
                        if let Some(request) =
                            handle_put_sample(&reply_targets, &next_correlation, &meta, sample)
                        {
                            #[cfg(feature = "async")]
                            {
                                if let Err(e) = async_tx.try_send(request) {
                                    log::warn!(
                                        "Zenoh server: failed to enqueue inbound request \
                                         (channel full): {e}"
                                    );
                                }
                            }
                            #[cfg(not(feature = "async"))]
                            {
                                if let Ok(mut pending) = pending.lock()
                                    && let Some(dropped) = push_bounded(&mut pending, request)
                                {
                                    log::warn!(
                                        "Zenoh server: pending queue full; dropping oldest request"
                                    );
                                    send_drop_reply(&reply_targets, &dropped.correlation);
                                }
                            }
                        }
                    })
                    .wait()
                    .map_err(|err| format!("zenoh put-listener declaration failed: {err}"))?;
                Ok(Some(DeclaredRoute::PutListener(Box::new(subscriber))))
            }
            RouteKind::Publisher { key_expr } => {
                // Register a PublisherSink with the EventBroker so emit_event /
                // observe_property deliveries reach remote zenoh subscribers.
                if let Some(broker) = broker {
                    let event_name = match &route.meta.target {
                        AffordanceTarget::Event(name) => name.clone(),
                        AffordanceTarget::Property(name) => name.clone(),
                        _ => return Ok(None),
                    };
                    let sink = ZenohPublisherSink {
                        session: self.session.clone(),
                        key_expr,
                    };
                    broker.register(thing_id.to_string(), event_name, sink);
                }
                Ok(None)
            }
        }
    }
}

/// Stable affordance identity used as the per-affordance route key, so a single
/// affordance's routes can be incrementally registered/unregistered.
fn affordance_key(target: &AffordanceTarget) -> String {
    match target {
        AffordanceTarget::Thing => String::from("thing"),
        AffordanceTarget::Property(name) => format!("property:{name}"),
        AffordanceTarget::Action(name) => format!("action:{name}"),
        AffordanceTarget::Event(name) => format!("event:{name}"),
    }
}

// ---------------------------------------------------------------------------
// Zenoh callback handlers
// ---------------------------------------------------------------------------

fn handle_query(
    reply_targets: &Mutex<BTreeMap<CorrelationId, ReplyTargetEntry>>,
    next_correlation: &AtomicU64,
    meta: &RouteMeta,
    key_expr: &str,
    query: Query,
) -> Option<InboundRequest> {
    let correlation = CorrelationId::from(next_correlation.fetch_add(1, Ordering::Relaxed));
    let input = query_to_input(&query);
    let auth = attachment_to_auth(query.attachment(), meta.auth_expectation);
    let request = InboundRequest {
        thing_id: ThingId::from(meta.thing_id.as_str()),
        target: meta.target.clone(),
        operation: meta.operation,
        input,
        auth,
        correlation,
    };
    let entry = ReplyTargetEntry {
        reply: ReplyTarget::Query {
            query,
            key_expr: key_expr.to_string(),
        },
        inserted_at: Instant::now(),
    };
    {
        let Ok(mut reply_targets) = reply_targets.lock() else {
            return None;
        };
        reply_targets.insert(request.correlation.clone(), entry);
    }
    // The caller pushes the request to the sync pending queue or the async
    // channel — no clone needed here (the request is moved, not duplicated).
    Some(request)
}

fn handle_put_sample(
    reply_targets: &Mutex<BTreeMap<CorrelationId, ReplyTargetEntry>>,
    next_correlation: &AtomicU64,
    meta: &RouteMeta,
    sample: Sample,
) -> Option<InboundRequest> {
    if sample.kind() != SampleKind::Put {
        return None;
    }

    let correlation = CorrelationId::from(next_correlation.fetch_add(1, Ordering::Relaxed));
    let input = sample_to_input(&sample);
    let auth = attachment_to_auth(sample.attachment(), meta.auth_expectation);
    let request = InboundRequest {
        thing_id: ThingId::from(meta.thing_id.as_str()),
        target: meta.target.clone(),
        operation: meta.operation,
        input,
        auth,
        correlation,
    };
    let entry = ReplyTargetEntry {
        reply: ReplyTarget::Put,
        inserted_at: Instant::now(),
    };
    {
        let Ok(mut reply_targets) = reply_targets.lock() else {
            return None;
        };
        reply_targets.insert(request.correlation.clone(), entry);
    }
    Some(request)
}

fn undeclare_routes(routes: Vec<DeclaredRoute>) {
    for route in routes {
        match route {
            DeclaredRoute::Queryable(handle) | DeclaredRoute::PutListener(handle) => {
                handle.undeclare_boxed();
            }
        }
    }
}

/// Pushes a request onto the pending queue with drop-oldest backpressure,
/// enforcing [`PENDING_QUEUE_CAPACITY`] so the queue cannot grow without bound
/// when the synchronous driving loop falls behind. Returns the evicted request
/// (if any) so the caller can fail it fast instead of waiting for the reply
/// TTL to expire.
fn push_bounded(
    queue: &mut VecDeque<InboundRequest>,
    request: InboundRequest,
) -> Option<InboundRequest> {
    let dropped = if queue.len() >= PENDING_QUEUE_CAPACITY {
        queue.pop_front()
    } else {
        None
    };
    queue.push_back(request);
    dropped
}

/// Fails a dropped request immediately by replying to its zenoh `Query` with an
/// error, instead of leaving it to time out after [`REPLY_TARGET_TTL`].
///
/// Query-kind targets get a `server busy` error reply; Put-kind targets are
/// fire-and-forget (no reply expected) and are simply dropped.
fn send_drop_reply(
    reply_targets: &Mutex<BTreeMap<CorrelationId, ReplyTargetEntry>>,
    correlation: &CorrelationId,
) {
    let Ok(mut reply_targets) = reply_targets.lock() else {
        return;
    };
    if let Some(ReplyTargetEntry {
        reply: ReplyTarget::Query { query, .. },
        ..
    }) = reply_targets.remove(correlation)
        && let Err(e) = query.reply_err("server busy: pending queue full").wait()
    {
        log::warn!("Zenoh server: failed to send drop reply: {e}");
    }
}

/// Removes [`ReplyTargetEntry`]s older than [`REPLY_TARGET_TTL`]. For each
/// evicted zenoh query, an error reply is sent so the underlying `Query`
/// resource is released instead of leaking (e.g. when the handler errors or
/// panics and [`ZenohServerBinding::send_response`] is never called).
fn sweep_expired_reply_targets(reply_targets: &mut BTreeMap<CorrelationId, ReplyTargetEntry>) {
    let now = Instant::now();
    let expired: Vec<CorrelationId> = reply_targets
        .iter()
        .filter(|(_, entry)| now.duration_since(entry.inserted_at) > REPLY_TARGET_TTL)
        .map(|(id, _)| id.clone())
        .collect();
    for id in expired {
        if let Some(ReplyTargetEntry {
            reply: ReplyTarget::Query { query, .. },
            ..
        }) = reply_targets.remove(&id)
            && let Err(e) = query.reply_err("timeout").wait()
        {
            log::warn!("Zenoh server: failed to send timeout reply: {e}");
        }
    }
}

fn query_to_input(query: &Query) -> InteractionInput {
    match query.payload() {
        Some(payload) => {
            let body = payload.to_bytes().into_owned();
            let content_type = query.encoding().map(|e| e.to_string()).unwrap_or_default();
            InteractionInput {
                payload: Some(Payload::new(body, content_type)),
                parameters: BTreeMap::new(),
                principal: None,
                security_metadata: BTreeMap::new(),
            }
        }
        None => InteractionInput::empty(),
    }
}

/// Extracts [`AuthMaterial`] from a zenoh attachment (`ZBytes`).
///
/// Extracts transport-level auth material from a zenoh attachment, interpreted
/// according to the route's [`AuthExpectation`].
///
/// For [`AuthExpectation::Bearer`] the attachment bytes are wrapped as a
/// [`AuthMaterial::BearerToken`] (with a size bound). For
/// [`AuthExpectation::None`] (NoSec) or [`AuthExpectation::Unsupported`]
/// (Basic/OAuth2/PSK/…) the attachment is **ignored** and `None` is returned —
/// the request is treated as unauthenticated. This replaces the previous
/// behavior of unconditionally wrapping arbitrary attachment bytes as a bearer
/// token, which misfed verification for non-bearer schemes.
fn attachment_to_auth(
    attachment: Option<&zenoh::bytes::ZBytes>,
    expectation: AuthExpectation,
) -> Option<AuthMaterial> {
    match expectation {
        AuthExpectation::Bearer => {
            let zbytes = attachment?;
            let bytes = zbytes.to_bytes().into_owned();
            if bytes.is_empty() {
                return None;
            }
            if bytes.len() > MAX_BEARER_TOKEN_BYTES {
                log::warn!(
                    "Zenoh inbound: bearer attachment ({} bytes) exceeds {}, dropping",
                    bytes.len(),
                    MAX_BEARER_TOKEN_BYTES
                );
                return None;
            }
            Some(AuthMaterial::BearerToken(bytes))
        }
        AuthExpectation::Unsupported => {
            log::warn!(
                "Zenoh inbound: route uses a security scheme the zenoh transport \
                 cannot extract from an attachment; attachment ignored"
            );
            None
        }
        AuthExpectation::None => None,
    }
}

fn sample_to_input(sample: &Sample) -> InteractionInput {
    let body = sample.payload().to_bytes().into_owned();
    let content_type = sample.encoding().to_string();
    if body.is_empty() && content_type.is_empty() {
        InteractionInput::empty()
    } else {
        InteractionInput {
            payload: Some(Payload::new(body, content_type)),
            parameters: BTreeMap::new(),
            principal: None,
            security_metadata: BTreeMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Route planning
// ---------------------------------------------------------------------------

enum RouteKind {
    Queryable { key_expr: String },
    PutListener { key_expr: String },
    Publisher { key_expr: String },
}

struct PlannedRoute {
    meta: RouteMeta,
    kind: RouteKind,
}

fn plan_inbound_routes(thing_id: &str, td: &Thing) -> Result<Vec<PlannedRoute>, String> {
    let mut routes = Vec::new();

    for (target, operation, form, zenoh_target) in
        iter_zenoh_affordance_forms(td).map_err(|e| e.to_string())?
    {
        // Build the execution plan from the already-resolved zenoh target to
        // avoid resolving the form target a second time.
        let plan = ZenohOperationPlan {
            key_expr: zenoh_target.key_expr,
            kind: zenoh_operation_kind(operation),
            metadata: extract_zenoh_metadata(form).map_err(|e| e.to_string())?,
        };

        let meta = RouteMeta {
            thing_id: thing_id.to_string(),
            target,
            operation,
            auth_expectation: resolve_auth_expectation(td, form),
        };

        let kind = match plan.kind {
            ZenohOperationKind::Query | ZenohOperationKind::RequestReply => RouteKind::Queryable {
                key_expr: plan.key_expr,
            },
            ZenohOperationKind::Put => RouteKind::PutListener {
                key_expr: plan.key_expr,
            },
            ZenohOperationKind::Subscribe => RouteKind::Publisher {
                key_expr: plan.key_expr,
            },
            ZenohOperationKind::Unsubscribe => {
                // No route or publisher needed — cleanup is broker-managed.
                continue;
            }
        };

        routes.push(PlannedRoute { meta, kind });
    }

    Ok(routes)
}

/// Iterates over all zenoh-targeting affordance forms in a TD, yielding
/// `(target, operation, form, zenoh_target)` tuples.
///
/// The resolved [`ZenohFormTarget`] is produced alongside each form so that
/// callers do not need to resolve the form target a second time.
fn iter_zenoh_affordance_forms(
    td: &Thing,
) -> ZenohBindingResult<Vec<(AffordanceTarget, Operation, &Form, ZenohFormTarget)>> {
    let mut result = Vec::new();

    if let Some(properties) = &td.properties {
        for (name, property) in properties {
            let context = FormContext::Property(property);
            collect_zenoh_forms(
                td,
                &property._interaction.forms,
                context,
                AffordanceTarget::Property(name.clone()),
                &mut result,
            )?;
        }
    }

    if let Some(actions) = &td.actions {
        for (name, action) in actions {
            let context = FormContext::Action(action);
            collect_zenoh_forms(
                td,
                &action._interaction.forms,
                context,
                AffordanceTarget::Action(name.clone()),
                &mut result,
            )?;
        }
    }

    if let Some(events) = &td.events {
        for (name, event) in events {
            let context = FormContext::Event(event);
            collect_zenoh_forms(
                td,
                &event._interaction.forms,
                context,
                AffordanceTarget::Event(name.clone()),
                &mut result,
            )?;
        }
    }

    if let Some(forms) = &td.forms {
        collect_zenoh_forms(
            td,
            forms,
            FormContext::Thing,
            AffordanceTarget::Thing,
            &mut result,
        )?;
    }

    Ok(result)
}

fn collect_zenoh_forms<'a>(
    td: &Thing,
    forms: &'a [Form],
    context: FormContext<'_>,
    target: AffordanceTarget,
    out: &mut Vec<(AffordanceTarget, Operation, &'a Form, ZenohFormTarget)>,
) -> ZenohBindingResult<()> {
    for form in forms {
        // Resolve the form target once: check the zenoh scheme and extract the
        // key expression in a single resolution pass.
        let Some(zenoh_target) = try_extract_zenoh_target(td, form)? else {
            continue;
        };
        for operation in effective_form_operations(context, form).iter() {
            out.push((target.clone(), *operation, form, zenoh_target.clone()));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// AsyncServerBinding (baseline v3.0 §4 / addendum §2.4)
// ---------------------------------------------------------------------------

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl AsyncServerBinding for ZenohServerBinding {
    async fn poll_accept(&self) -> InboundRequest {
        loop {
            // Drain the async channel directly. Unlike `poll_accept_sync`,
            // this never touches the `std::sync::Mutex`-guarded pending queue
            // and therefore does not stall the tokio worker thread.
            let request = {
                let mut rx = self.async_rx.lock().await;
                rx.recv().await
            };
            if let Some(request) = request {
                if let Ok(mut reply_targets) = self.reply_targets.lock() {
                    sweep_expired_reply_targets(&mut reply_targets);
                }
                return request;
            }
            // All senders were dropped (e.g. no routes are declared yet). Yield
            // instead of busy-spinning, then wait for a sender to reappear.
            tokio::task::yield_now().await;
        }
    }

    fn send_response(&self, response: InboundResponse) {
        ServerBinding::send_response(self, response);
    }

    fn register_thing(&self, thing_id: &str, td: &Thing) -> Result<(), String> {
        ServerBinding::register_thing(self, thing_id, td)
    }

    fn unregister_thing(&self, thing_id: &str) {
        ServerBinding::unregister_thing(self, thing_id);
    }
}

// ---------------------------------------------------------------------------

fn io_error(err: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::other(err.to_string())
}

// ---------------------------------------------------------------------------
// ZenohPublisherSink — bridges EventBroker fan-out to zenoh session.put
// ---------------------------------------------------------------------------

/// [`PublisherSink`] that publishes event payloads to a zenoh key expression.
///
/// Registered with the [`EventBroker`] during
/// [`ZenohServerBinding::register_thing`] for each event and observable
/// property form. When the broker fans out a payload, the sink calls
/// `session.put` on its key expression, delivering the sample to every remote
/// zenoh subscriber.
struct ZenohPublisherSink {
    session: zenoh::Session,
    key_expr: String,
}

impl PublisherSink for ZenohPublisherSink {
    fn publish(&self, payload: &Payload) -> clinkz_wot_core::CoreResult<()> {
        let mut builder = self
            .session
            .put(self.key_expr.as_str(), payload.body.as_slice());
        if !payload.content_type.is_empty() {
            builder = builder.encoding(Encoding::from(payload.content_type.as_str()));
        }
        builder
            .wait()
            .map_err(|e| clinkz_wot_core::CoreError::Transport(e.to_string()))
    }
}
