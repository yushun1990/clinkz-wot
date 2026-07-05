//! `Servient` — composition root (baseline v4.0 §7 / phase-p3 §3.1–§3.2, §3.5,
//! §3.10–§3.11). Non-generic; holds registries, bindings, the inbound fan-in
//! channel, a `Discoverer`, and the driving primitives.

use alloc::{boxed::Box, format, sync::Arc, vec::Vec};

#[cfg(feature = "std")]
use clinkz_wot_core::FanInSender;
use clinkz_wot_core::{
    ClientBinding, Dispatch, EventBroker, EventName, ExposedThing, InboundRequest, InboundResponse,
    InteractionOutput, Payload, ServerBinding, ThingId, WotLock,
};
use clinkz_wot_discovery::{Discoverer, DiscoveryFilter, ProcessState, ThingDiscoveryProcess};
use clinkz_wot_td::{AbsoluteUri, thing::Thing};

use crate::handle::{ConsumedThingHandle, ExposedThingHandle};
use crate::registry::{ConsumedThingRegistry, ExposedThingRegistry, ExposedThingSlot};
use crate::{ServientError, ServientResult};

/// Constructs a fresh [`ClientBinding`] for a consumed Thing. The Servient holds
/// a list of factories; `consume()` builds one binding per factory and
/// registers them into the [`clinkz_wot_core::ConsumedThing`].
#[cfg(feature = "async")]
pub trait ClientBindingFactory: Send + Sync {
    /// Builds a fresh client binding instance.
    fn build(&self) -> Box<dyn ClientBinding>;
}

/// Drives the shutdown flag for graceful teardown.
#[derive(Clone)]
pub struct ShutdownHandle {
    flag: Arc<core::sync::atomic::AtomicBool>,
}

impl ShutdownHandle {
    /// Signals the serving loops to stop after their current iteration.
    pub fn shutdown(&self) {
        self.flag.store(true, core::sync::atomic::Ordering::SeqCst);
    }

    /// Whether shutdown has been requested.
    #[allow(dead_code)]
    pub fn is_shutdown(&self) -> bool {
        self.flag.load(core::sync::atomic::Ordering::SeqCst)
    }
}

/// The Servient: composes exposed/consumed Things, server/client bindings, and
/// discovery into a single runtime. `Clone` (cheap, `Arc`/`WotLock` clones),
/// all methods `&self`, `Send + Sync`.
#[derive(Clone)]
pub struct Servient {
    pub(crate) exposed: ExposedThingRegistry,
    #[allow(dead_code)]
    consumed_registry: ConsumedThingRegistry,
    pub(crate) server_bindings: Arc<[Arc<dyn ServerBinding>]>,
    #[cfg(feature = "async")]
    pub(crate) client_factories: Arc<[Arc<dyn ClientBindingFactory>]>,
    pub(crate) discoverer: Arc<dyn Discoverer>,
    pub(crate) event_broker: EventBroker,
    shutdown: Arc<core::sync::atomic::AtomicBool>,
    /// std-only inbound fan-in receiver (bindings self-push via `set_request_sink`).
    #[cfg(feature = "std")]
    inbound_rx: Arc<async_channel::Receiver<InboundRequest>>,
    /// std-only fan-in sender; cloned into each binding at registration.
    #[cfg(feature = "std")]
    inbound_tx: FanInSender<InboundRequest>,
    /// no_std rotation cursor for `try_accept` poll fairness (AD7).
    #[allow(dead_code)]
    rotation: Arc<core::sync::atomic::AtomicUsize>,
}

impl Servient {
    /// Assembles a `Servient` from its pieces (called by `ServientBuilder`).
    #[cfg(feature = "std")]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn assemble(
        exposed: ExposedThingRegistry,
        consumed_registry: ConsumedThingRegistry,
        server_bindings: Arc<[Arc<dyn ServerBinding>]>,
        client_factories: Arc<[Arc<dyn ClientBindingFactory>]>,
        discoverer: Arc<dyn Discoverer>,
        event_broker: EventBroker,
        inbound_tx: FanInSender<InboundRequest>,
        inbound_rx: Arc<async_channel::Receiver<InboundRequest>>,
    ) -> Self {
        Self {
            exposed,
            consumed_registry,
            server_bindings,
            client_factories,
            discoverer,
            event_broker,
            shutdown: Arc::new(core::sync::atomic::AtomicBool::new(false)),
            inbound_rx,
            inbound_tx,
            rotation: Arc::new(core::sync::atomic::AtomicUsize::new(0)),
        }
    }

    /// Returns the inbound fan-in sender (std only).
    #[cfg(feature = "std")]
    #[allow(dead_code)]
    pub(crate) fn inbound_sender(&self) -> FanInSender<InboundRequest> {
        self.inbound_tx.clone()
    }

    /// Returns the shutdown handle.
    pub fn shutdown_handle(&self) -> ShutdownHandle {
        ShutdownHandle {
            flag: Arc::clone(&self.shutdown),
        }
    }

    // --- facade (WoT surface) ---

    /// Produces a draft [`ExposedThingHandle`] from a TD (not yet remotely
    /// servable). `id` is required (E18).
    pub fn produce(&self, td: Thing) -> ServientResult<ExposedThingHandle> {
        let id = td
            .id
            .as_ref()
            .map(|u| ThingId::from(u.as_str()))
            .ok_or(ServientError::MissingThingId)?;
        let slot = Arc::new(WotLock::new(ExposedThingSlot::new(ExposedThing::new(td))));
        Ok(ExposedThingHandle::new(self.clone(), slot, id))
    }

    /// Consumes a TD: builds a [`clinkz_wot_core::ConsumedThing`] with fresh
    /// client bindings and returns a [`ConsumedThingHandle`].
    #[cfg(feature = "async")]
    pub fn consume(&self, td: Thing) -> ServientResult<ConsumedThingHandle> {
        use clinkz_wot_core::ConsumedThing;
        let id = td
            .id
            .as_ref()
            .map(|u| ThingId::from(u.as_str()))
            .ok_or(ServientError::MissingThingId)?;
        let mut consumed = ConsumedThing::new(td);
        for factory in self.client_factories.iter() {
            // `Box<dyn ClientBinding>` implements `ClientBinding` (core's blanket
            // impl), so it satisfies the `impl ClientBinding` registration entry.
            consumed.register_binding(factory.build());
        }
        self.consumed_registry.track(id.clone());
        Ok(ConsumedThingHandle::new(self.clone(), consumed, id))
    }

    /// Synchronous discovery: returns a lazy [`ThingDiscoveryProcess`] (no
    /// network/directory work until the first `next()`, AD10).
    #[cfg(feature = "async")]
    pub fn discover(&self, filter: DiscoveryFilter) -> ThingDiscoveryProcess {
        match self.discoverer.discover(filter) {
            Ok(process) => process,
            Err(err) => {
                // Bridge a fallible discover() into the infallible Scripting API
                // shape via a terminal Done(err) process (D5).
                ThingDiscoveryProcess::new(Box::new(ProcessState::done(Some(err))))
            }
        }
    }

    /// Fetches a Thing Description by URL (delegates to the Discoverer).
    #[cfg(feature = "async")]
    pub async fn fetch_td(&self, url: &AbsoluteUri) -> ServientResult<Thing> {
        Ok(self.discoverer.request_thing_description(url).await?)
    }

    // --- lifecycle hooks (called by ExposedThingHandle) ---

    /// Registers a Thing on every server binding (wholesale), inserts into the
    /// servable registry, and publishes the TD. Multi-binding rollback (E12/AD27).
    pub(crate) async fn expose_thing(
        &self,
        id: ThingId,
        slot: Arc<WotLock<ExposedThingSlot>>,
    ) -> ServientResult<()> {
        if self.exposed.contains(&id) {
            return Err(ServientError::AlreadyExposed(id));
        }
        let td = slot.with_read(|s| s.thing.thing_description().clone());
        // 1. register on ALL bindings (deterministic order); rollback on failure.
        let mut registered: Vec<usize> = Vec::new();
        for (i, binding) in self.server_bindings.iter().enumerate() {
            if let Err(err) = binding.register_thing(&id, &td) {
                for &j in registered.iter().rev() {
                    self.server_bindings[j].unregister_thing(&id);
                }
                return Err(ServientError::RouteRegistration(err));
            }
            registered.push(i);
        }
        // 2. insert into the registry (now remotely servable).
        if self.exposed.insert(id.clone(), slot).is_err() {
            for binding in self.server_bindings.iter() {
                binding.unregister_thing(&id);
            }
            return Err(ServientError::AlreadyExposed(id));
        }
        // 3. publish TD via the directory publisher (best-effort) — none in v1 MVP.
        Ok(())
    }

    /// Quiescing teardown (AD15). Idempotent (AD27/E13).
    pub(crate) async fn destroy_thing(&self, id: &ThingId) -> ServientResult<()> {
        let Some(slot) = self.exposed.get(id) else {
            return Ok(()); // idempotent: never-exposed/already-removed → Ok
        };
        // 1. routes-first: no new requests can arrive.
        for binding in self.server_bindings.iter() {
            binding.unregister_thing(id);
        }
        // 2. set draining: not-yet-dispatched requests are rejected.
        slot.with(|s| {
            s.draining.store(true, core::sync::atomic::Ordering::SeqCst);
        });
        // 3-4. remove the registry entry (MVP: immediate; full in-flight tracking
        //      is a refinement).
        self.exposed.remove(id);
        Ok(())
    }

    /// Fans an event/property-change payload out to registered subscribers.
    pub(crate) fn emit_event(
        &self,
        thing: &ThingId,
        name: &str,
        payload: Payload,
    ) -> Result<(), clinkz_wot_core::CoreError> {
        self.event_broker
            .publish(thing, &EventName::from(name), &payload)
    }

    // --- driving (§3.5) ---

    /// Processes at most ONE inbound request, then returns (AD6b). Native async.
    #[cfg(feature = "std")]
    pub async fn poll_serve(&self) -> ServientResult<()> {
        use core::sync::atomic::Ordering;
        if self.shutdown.load(Ordering::SeqCst) {
            return Ok(());
        }
        let request = self.inbound_rx.recv().await.map_err(|_| {
            ServientError::Serve(clinkz_wot_core::CoreError::InboundDispatch(
                "fan-in channel closed".into(),
            ))
        })?;
        self.serve_one(request).await;
        Ok(())
    }

    /// `while !shutdown { poll_serve().await }`.
    #[cfg(feature = "std")]
    pub async fn serve(&self) {
        use core::sync::atomic::Ordering;
        while !self.shutdown.load(Ordering::SeqCst) {
            if let Err(err) = self.poll_serve().await {
                log::warn!("serve step failed: {err}");
            }
        }
    }

    /// Dispatches one request and replies via each server binding's
    /// `send_response` (matched by `CorrelationId`).
    pub(crate) async fn serve_one(&self, request: InboundRequest) {
        let response = self.dispatch(request).await;
        for binding in self.server_bindings.iter() {
            binding.send_response(response.clone());
        }
    }

    /// Bare-`no_std` super-loop step (AD16: compile-time architecture only in v1).
    /// Polls `try_accept` round-robin; the async dispatch path requires an
    /// executor and is exercised on std/embassy only.
    #[cfg(not(feature = "std"))]
    pub fn poll_serve_once(
        &self,
        _cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<ServientResult<()>> {
        let n = self.server_bindings.len();
        if n == 0 {
            return core::task::Poll::Ready(Ok(()));
        }
        let start = self
            .rotation
            .fetch_add(1, core::sync::atomic::Ordering::Relaxed)
            % n;
        for i in 0..n {
            if self.server_bindings[(start + i) % n].try_accept().is_some() {
                // v1 no_std driving is compile-only (AD16); runtime dispatch
                // lands with a concrete no_std binding (zenoh-pico, P2 §2.7).
                return core::task::Poll::Ready(Ok(()));
            }
        }
        core::task::Poll::Pending
    }

    /// Dispatch routing (§3.6). Resolves the exposed Thing, checks draining,
    /// routes by operation to the handler (sync dispatch path; async-handler
    /// dispatch is a documented MVP boundary).
    pub(crate) async fn dispatch(&self, request: InboundRequest) -> InboundResponse {
        use clinkz_wot_core::CoreError;
        use clinkz_wot_td::data_type::Operation;
        let correlation = request.correlation.clone();
        let Some(slot) = self.exposed.get(&request.thing_id) else {
            return InboundResponse::error(
                correlation,
                CoreError::InboundDispatch("Thing gone".into()),
            );
        };
        if slot.with_read(|s| s.draining.load(core::sync::atomic::Ordering::SeqCst)) {
            return InboundResponse::error(
                correlation,
                CoreError::InboundDispatch("Thing gone".into()),
            );
        }
        let mut input = request.input;
        let result = slot.with_read(|s| -> Result<InteractionOutput, CoreError> {
            let name = request.target.name().unwrap_or("");
            match request.operation {
                Operation::ReadProperty => s.thing.read_property(name, &input),
                Operation::WriteProperty => s.thing.write_property(name, &mut input),
                Operation::InvokeAction => s.thing.invoke_action(name, &mut input),
                Operation::QueryAction => s.thing.query_action(name, &input),
                Operation::CancelAction => s.thing.cancel_action(name, &mut input),
                Operation::SubscribeEvent => s.thing.subscribe_event(name, &input, &mut |_| Ok(())),
                Operation::ObserveProperty => {
                    s.thing.observe_property(name, &input, &mut |_| Ok(()))
                }
                Operation::UnsubscribeEvent => s.thing.unsubscribe_event(name, &input),
                Operation::UnobserveProperty => s.thing.unobserve_property(name, &input),
                _ => Err(CoreError::UnsupportedOperation(format!(
                    "operation {:?} not handled by MVP dispatch",
                    request.operation
                ))),
            }
        });
        match result {
            Ok(output) => InboundResponse::new(output, correlation),
            Err(err) => InboundResponse::error(correlation, err),
        }
    }
}

/// Direct-dispatch implementation: lets bindings with async handlers (HTTP,
/// CoAP) dispatch requests directly without the fan-in channel + driving loop.
/// The binding calls `serve_request(request).await` inside its async handler
/// and gets the response — the transport's own concurrency model provides
/// backpressure.
#[cfg(feature = "async")]
#[async_trait::async_trait]
impl Dispatch for Servient {
    async fn serve_request(&self, request: InboundRequest) -> InboundResponse {
        self.dispatch(request).await
    }
}
