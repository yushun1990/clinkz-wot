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

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[cfg(feature = "async")]
use clinkz_wot_core::AsyncServerBinding;
use clinkz_wot_core::identity::CorrelationId;
use clinkz_wot_core::{
    AffordanceTarget, AuthMaterial, EventBroker, InboundRequest, InboundResponse, InteractionInput,
    Payload, PublisherSink, ServerBinding, ThingId,
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

use crate::{ZenohOperationKind, is_zenoh_form_target, plan_zenoh_operation};

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
}

/// How to deliver a response back to the zenoh requester.
enum ReplyTarget {
    /// Zenoh query expecting a reply (readproperty, invokeaction).
    Query { query: Query, key_expr: String },
    /// PUT sample — fire-and-forget, no reply expected (writeproperty).
    Put,
}

/// A pending inbound request paired with its reply mechanism.
struct PendingRequest {
    request: InboundRequest,
    reply: ReplyTarget,
}

/// Declared zenoh handles for one Thing, stored for cleanup.
//
// The concrete handler types from zenoh's `.callback()` builder differ across
// zenoh versions; we erase them behind `Box<dyn Send>` and call `undeclare`
// through the type-erased `Undeclare` trait.
trait RouteHandle: Send {
    fn undeclare_boxed(self: Box<Self>);
}

impl<H> RouteHandle for Queryable<H>
where
    H: Send,
{
    fn undeclare_boxed(self: Box<Self>) {
        let _ = Queryable::undeclare(*self).wait();
    }
}

impl<H> RouteHandle for Subscriber<H>
where
    H: Send,
{
    fn undeclare_boxed(self: Box<Self>) {
        let _ = Subscriber::undeclare(*self).wait();
    }
}

enum DeclaredRoute {
    Queryable(Box<dyn RouteHandle>),
    PutListener(Box<dyn RouteHandle>),
}

struct ServerState {
    routes: BTreeMap<String, Vec<DeclaredRoute>>,
    pending: VecDeque<PendingRequest>,
    reply_targets: BTreeMap<CorrelationId, ReplyTarget>,
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
    routes: Arc<Mutex<BTreeMap<String, Vec<DeclaredRoute>>>>,
    pending: Arc<Mutex<VecDeque<PendingRequest>>>,
    reply_targets: Arc<Mutex<BTreeMap<CorrelationId, ReplyTarget>>>,
    event_broker: Arc<Mutex<Option<EventBroker>>>,
    next_correlation: Arc<AtomicU64>,
    #[cfg(feature = "async")]
    notify: Arc<tokio::sync::Notify>,
}

impl ZenohServerBinding {
    /// Creates a server binding from an existing zenoh session.
    ///
    /// The session is shared — the same session can be used for outbound
    /// interactions via [`crate::ZenohSessionTransport`].
    pub fn new(session: zenoh::Session) -> Self {
        let state = ServerState::new();
        Self {
            session,
            routes: Arc::new(Mutex::new(state.routes)),
            pending: Arc::new(Mutex::new(state.pending)),
            reply_targets: Arc::new(Mutex::new(state.reply_targets)),
            event_broker: Arc::new(Mutex::new(state.event_broker)),
            next_correlation: Arc::new(state.next_correlation),
            #[cfg(feature = "async")]
            notify: Arc::new(tokio::sync::Notify::new()),
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
        let pending = self.pending.lock().ok()?.pop_front()?;
        let correlation = pending.request.correlation.clone();
        self.reply_targets
            .lock()
            .ok()?
            .insert(correlation, pending.reply);
        Some(pending.request)
    }

    fn send_response(&self, response: InboundResponse) {
        let reply_target = match self.reply_targets.lock() {
            Ok(mut reply_targets) => reply_targets.remove(&response.correlation),
            Err(_) => return,
        };

        match reply_target {
            Some(ReplyTarget::Query { query, key_expr }) => {
                if let Some(err) = response.error {
                    let status = clinkz_wot_protocol_bindings::error_status(&err);
                    let _ = query.reply_err(format!("[{}] {}", status, err)).wait();
                } else {
                    let (payload_body, content_type) = match response.output.payload {
                        Some(payload) => (payload.body, payload.content_type),
                        None => (Vec::new(), String::new()),
                    };
                    let mut builder = query.reply(key_expr.as_str(), payload_body);
                    if !content_type.is_empty() {
                        builder = builder.encoding(Encoding::from(content_type.as_str()));
                    }
                    let _ = builder.wait();
                }
            }
            Some(ReplyTarget::Put) | None => { /* no reply needed */ }
        }
    }

    fn register_thing(&self, thing_id: &str, td: &Thing) -> Result<(), String> {
        let routes = plan_inbound_routes(thing_id, td)?;
        let broker = self.event_broker.lock().map_err(|e| e.to_string())?.clone();
        let mut declared = Vec::with_capacity(routes.len());

        for route in routes {
            match route.kind {
                RouteKind::Queryable { key_expr } => {
                    let pending = Arc::clone(&self.pending);
                    let next_correlation = Arc::clone(&self.next_correlation);
                    let meta = route.meta.clone();
                    let key_for_reply = key_expr.clone();
                    #[cfg(feature = "async")]
                    let notify_handle = self.notify.clone();

                    let queryable = match self
                        .session
                        .declare_queryable(key_expr.as_str())
                        .callback(move |query| {
                            handle_query(&pending, &next_correlation, &meta, &key_for_reply, query);
                            #[cfg(feature = "async")]
                            notify_handle.notify_one();
                        })
                        .wait()
                    {
                        Ok(queryable) => queryable,
                        Err(err) => {
                            undeclare_routes(declared);
                            return Err(format!("zenoh queryable declaration failed: {err}"));
                        }
                    };

                    declared.push(DeclaredRoute::Queryable(Box::new(queryable)));
                }
                RouteKind::PutListener { key_expr } => {
                    let pending = Arc::clone(&self.pending);
                    let next_correlation = Arc::clone(&self.next_correlation);
                    let meta = route.meta.clone();
                    #[cfg(feature = "async")]
                    let notify_handle = self.notify.clone();

                    let subscriber = match self
                        .session
                        .declare_subscriber(key_expr.as_str())
                        .callback(move |sample| {
                            handle_put_sample(&pending, &next_correlation, &meta, sample);
                            #[cfg(feature = "async")]
                            notify_handle.notify_one();
                        })
                        .wait()
                    {
                        Ok(subscriber) => subscriber,
                        Err(err) => {
                            undeclare_routes(declared);
                            return Err(format!("zenoh put-listener declaration failed: {err}"));
                        }
                    };

                    declared.push(DeclaredRoute::PutListener(Box::new(subscriber)));
                }
                RouteKind::Publisher { key_expr } => {
                    // Register a PublisherSink with the EventBroker so that
                    // emit_event / observe_property deliveries reach remote
                    // zenoh subscribers via session.put.
                    if let Some(ref broker) = broker {
                        let event_name = match &route.meta.target {
                            AffordanceTarget::Event(name) => name.clone(),
                            AffordanceTarget::Property(name) => name.clone(),
                            _ => continue,
                        };
                        let sink = ZenohPublisherSink {
                            session: self.session.clone(),
                            key_expr,
                        };
                        broker.register(thing_id.to_string(), event_name, sink);
                    }
                    // No DeclaredRoute — cleanup is via broker.remove_thing
                    // during unregister_thing.
                }
            }
        }

        self.routes
            .lock()
            .map_err(|e| e.to_string())?
            .insert(thing_id.to_string(), declared);
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
        let routes = match self.routes.lock() {
            Ok(mut routes) => routes.remove(thing_id),
            Err(_) => return,
        };

        if let Some(routes) = routes {
            undeclare_routes(routes);
        }
    }

    fn set_event_broker(&self, broker: EventBroker) {
        if let Ok(mut event_broker) = self.event_broker.lock() {
            *event_broker = Some(broker);
        }
    }
}

// ---------------------------------------------------------------------------
// Zenoh callback handlers
// ---------------------------------------------------------------------------

fn handle_query(
    pending: &Mutex<VecDeque<PendingRequest>>,
    next_correlation: &AtomicU64,
    meta: &RouteMeta,
    key_expr: &str,
    query: Query,
) {
    let correlation = CorrelationId::from(next_correlation.fetch_add(1, Ordering::Relaxed));
    let input = query_to_input(&query);
    let auth = attachment_to_auth(query.attachment());
    let request = InboundRequest {
        thing_id: ThingId::from(meta.thing_id.as_str()),
        target: meta.target.clone(),
        operation: meta.operation,
        input,
        auth,
        correlation,
    };
    let Ok(mut pending) = pending.lock() else {
        return;
    };
    pending.push_back(PendingRequest {
        request,
        reply: ReplyTarget::Query {
            query,
            key_expr: key_expr.to_string(),
        },
    });
}

fn handle_put_sample(
    pending: &Mutex<VecDeque<PendingRequest>>,
    next_correlation: &AtomicU64,
    meta: &RouteMeta,
    sample: Sample,
) {
    if sample.kind() != SampleKind::Put {
        return;
    }

    let correlation = CorrelationId::from(next_correlation.fetch_add(1, Ordering::Relaxed));
    let input = sample_to_input(&sample);
    let auth = attachment_to_auth(sample.attachment());
    let request = InboundRequest {
        thing_id: ThingId::from(meta.thing_id.as_str()),
        target: meta.target.clone(),
        operation: meta.operation,
        input,
        auth,
        correlation,
    };
    let Ok(mut pending) = pending.lock() else {
        return;
    };
    pending.push_back(PendingRequest {
        request,
        reply: ReplyTarget::Put,
    });
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
/// The attachment bytes are interpreted as a bearer token. If the attachment
/// is absent, `None` is returned and the request will be treated as
/// unauthenticated (suitable for NoSec schemes).
fn attachment_to_auth(attachment: Option<&zenoh::bytes::ZBytes>) -> Option<AuthMaterial> {
    let zbytes = attachment?;
    let bytes = zbytes.to_bytes().into_owned();
    if bytes.is_empty() {
        return None;
    }
    Some(AuthMaterial::BearerToken(bytes))
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

    for (target, operation, form) in iter_zenoh_affordance_forms(td) {
        let plan = plan_zenoh_operation(td, form, operation).map_err(|e| e.to_string())?;

        let meta = RouteMeta {
            thing_id: thing_id.to_string(),
            target,
            operation,
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
/// `(target, operation, form)` triples.
fn iter_zenoh_affordance_forms(td: &Thing) -> Vec<(AffordanceTarget, Operation, &Form)> {
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
            );
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
            );
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
            );
        }
    }

    if let Some(forms) = &td.forms {
        collect_zenoh_forms(
            td,
            forms,
            FormContext::Thing,
            AffordanceTarget::Thing,
            &mut result,
        );
    }

    result
}

fn collect_zenoh_forms<'a>(
    td: &Thing,
    forms: &'a [Form],
    context: FormContext<'_>,
    target: AffordanceTarget,
    out: &mut Vec<(AffordanceTarget, Operation, &'a Form)>,
) {
    for form in forms {
        if !is_zenoh_form_target(td, form) {
            continue;
        }
        for operation in effective_form_operations(context, form).iter() {
            out.push((target.clone(), *operation, form));
        }
    }
}

// ---------------------------------------------------------------------------
// AsyncServerBinding (baseline v3.0 §4 / addendum §2.4)
// ---------------------------------------------------------------------------

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl AsyncServerBinding for ZenohServerBinding {
    async fn poll_accept(&self) -> InboundRequest {
        loop {
            if let Some(request) = ServerBinding::poll_accept_sync(self) {
                return request;
            }
            self.notify.notified().await;
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
            .put(self.key_expr.as_str(), payload.body.clone());
        if !payload.content_type.is_empty() {
            builder = builder.encoding(Encoding::from(payload.content_type.as_str()));
        }
        builder
            .wait()
            .map_err(|e| clinkz_wot_core::CoreError::Transport(e.to_string()))
    }
}
