use alloc::{borrow::ToOwned, boxed::Box, format, string::String, sync::Arc, vec::Vec};

#[cfg(feature = "async")]
use core::future::Future;
#[cfg(feature = "async")]
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "async")]
use clinkz_wot_core::AsyncServerBinding;
use clinkz_wot_core::{
    ClientBinding, CoreError, CoreResult, CredentialStore, EventBroker, EventName, EventSink,
    InboundRequest, InboundResponse, InteractionInput, InteractionOutput, LocalThing, MapLock,
    PayloadCodec, SecurityProvider, ServerBinding, ThingId,
};
use clinkz_wot_discovery::{
    DirectoryEntry, DirectoryPage, DirectoryQuery, InMemoryThingDirectory, ThingDirectory,
    ThingDiscovery, ThingFilter, discover as run_discovery,
};
use clinkz_wot_td::thing::Thing;
#[cfg(feature = "async")]
use futures_util::stream::{FuturesUnordered, StreamExt};

use crate::{
    ConsumedThingRegistry, InMemoryExposedThingRegistry, ServientBuilder, ServientError,
    ServientResult,
    handle::{ConsumedThingHandle, ExposedThingHandle},
    interaction::InteractionRuntime,
    registry::ResolvedInboundSecurity,
};

pub(crate) type BindingFactory = Box<dyn Fn() -> Box<dyn ClientBinding>>;

pub(crate) type PayloadCodecRegistry = Arc<MapLock<Vec<Box<dyn PayloadCodec>>>>;
pub(crate) type SecurityProviderRegistry = Arc<MapLock<Vec<Box<dyn SecurityProvider>>>>;

/// Interior-mutable registry of protocol binding factories with generation
/// tracking.
///
/// The generation counter bumps on every mutation, so callers can cache derived
/// state (e.g. validated binding plans) keyed by generation and skip
/// revalidation when the registry has not changed since the cache entry was
/// built.
#[derive(Clone)]
pub(crate) struct BindingFactoryRegistry {
    state: Arc<MapLock<BindingFactoryState>>,
}

struct BindingFactoryState {
    factories: Vec<BindingFactory>,
    generation: u64,
}

impl BindingFactoryRegistry {
    /// Wraps a pre-built factory vec (used by `ServientBuilder::build`).
    pub(crate) fn from_factories(factories: Vec<BindingFactory>) -> Self {
        Self {
            #[allow(clippy::arc_with_non_send_sync)]
            state: Arc::new(MapLock::new(BindingFactoryState {
                factories,
                generation: 0,
            })),
        }
    }

    /// Appends a factory and bumps the generation counter.
    pub(crate) fn push(&self, factory: BindingFactory) {
        self.state.with(|s| {
            s.factories.push(factory);
            s.generation = s.generation.wrapping_add(1);
        });
    }

    /// Returns the current generation counter.
    ///
    /// Cached binding plans store the generation observed at validation time;
    /// if the current generation matches, the cached plan is still valid and
    /// the caller can skip revalidation.
    pub(crate) fn generation(&self) -> u64 {
        self.state.with(|s| s.generation)
    }

    /// Constructs a fresh `Box<dyn ClientBinding>` from the factory at `index`.
    ///
    /// Returns `None` when `index` is out of bounds (the factory was removed
    /// or the registry shrank).
    pub(crate) fn make_binding(&self, index: usize) -> Option<Box<dyn ClientBinding>> {
        self.state.with(|s| s.factories.get(index).map(|f| f()))
    }

    /// Iterates factories to find one whose constructed binding supports the
    /// given form/operation, returning `(factory_index, binding)`.
    pub(crate) fn find_supporting(
        &self,
        thing: &Thing,
        form: &clinkz_wot_td::form::Form,
        operation: clinkz_wot_td::data_type::Operation,
    ) -> Option<(usize, Box<dyn ClientBinding>)> {
        self.state.with(|s| {
            for (index, factory) in s.factories.iter().enumerate() {
                let binding = factory();
                if binding.supports_with_thing(thing, form, operation) {
                    return Some((index, binding));
                }
            }
            None
        })
    }
}

#[cfg(feature = "async")]
type AcceptFuture =
    Pin<Box<dyn Future<Output = (Arc<dyn AsyncServerBinding>, InboundRequest)> + Send>>;

#[cfg(feature = "async")]
pub(crate) struct AsyncAcceptState {
    generation: u64,
    pending: FuturesUnordered<AcceptFuture>,
}

#[cfg(feature = "async")]
impl AsyncAcceptState {
    pub(crate) fn new() -> Self {
        Self {
            generation: 0,
            pending: FuturesUnordered::new(),
        }
    }

    fn rebuild(&mut self, generation: u64, bindings: &[Arc<dyn AsyncServerBinding>]) {
        self.generation = generation;
        self.pending = bindings
            .iter()
            .cloned()
            .map(accept_future_for_binding)
            .collect();
    }
}

#[cfg(feature = "async")]
impl Default for AsyncAcceptState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "async")]
fn accept_future_for_binding(binding: Arc<dyn AsyncServerBinding>) -> AcceptFuture {
    Box::pin(async move {
        let request = binding.poll_accept().await;
        (binding, request)
    })
}

/// Lock-free shared Servient state.
///
/// Every field either has its own interior mutability (registries behind their
/// own `MapLock`, broker behind its internal `Arc<MapLock<…>>`) or is immutable
/// after `ServientBuilder::build`. Holding these outside the state lock lets
/// `interaction_runtime`, `dispatch_inbound`, and other hot paths clone shared
/// `Arc` references without acquiring the outer Servient lock.
pub(crate) struct ServientShared {
    pub(crate) exposed_registry: Arc<InMemoryExposedThingRegistry>,
    pub(crate) consumed_registry: Arc<ConsumedThingRegistry>,
    pub(crate) binding_factories: BindingFactoryRegistry,
    pub(crate) payload_codecs: PayloadCodecRegistry,
    pub(crate) security_providers: SecurityProviderRegistry,
    pub(crate) credential_store: Option<Arc<dyn CredentialStore>>,
    pub(crate) event_broker: EventBroker,
}

/// Stateful Servient state protected by a single outer lock.
///
/// Combines the directory (mutable, exclusive access) with the driving layer
/// (server bindings, cursor, async accept state). Kept under one lock so
/// `expose` / `destroy` can sequence "register routes → publish to directory"
/// atomically from the driving loop's perspective.
pub(crate) struct ServientState<D> {
    pub(crate) directory: D,
    pub(crate) server_bindings: Vec<Arc<dyn ServerBinding>>,
    pub(crate) sync_binding_cursor: usize,
    #[cfg(feature = "async")]
    pub(crate) async_server_bindings: Vec<Arc<dyn AsyncServerBinding>>,
    #[cfg(feature = "async")]
    pub(crate) async_binding_generation: u64,
    #[cfg(feature = "async")]
    pub(crate) async_accept_state: AsyncAcceptState,
}

/// Web of Things Servient that composes discovery, exposed Things, and consumed
/// Things.
///
/// `Servient<D>` keeps a single generic parameter `D` for the discovery
/// directory (baseline §5 / §6). It is cheaply [`Clone`] (the live state is
/// shared behind an `Arc`) and every public method takes `&self`. There is no
/// `running` lifecycle flag: composition and interactions are always live and
/// mutate shared state through interior mutability.
///
/// Interactions are performed through typed handles returned by
/// [`expose`](Self::expose) and [`consume](Self::consume) (baseline §6).
///
/// # Lock architecture
///
/// State is split into two `Arc`-shared regions:
///
/// - [`ServientShared`] holds the lock-free registries (binding factories,
///   payload codecs, security providers, credential store), the exposed and
///   consumed Thing registries, and the event broker. Each of these has its
///   own interior mutability, so cloning an `Arc` reference is enough to use
///   them — no outer lock is acquired on the interaction or dispatch hot paths.
/// - [`ServientState`] holds the directory and driving layer (server bindings,
///   cursor, async accept state) behind a single `MapLock`. Directory mutations
///   and binding registration are sequenced here.
pub struct Servient<D = InMemoryThingDirectory> {
    shared: Arc<ServientShared>,
    state: Arc<MapLock<ServientState<D>>>,
    shutdown: Arc<AtomicBool>,
}

impl Servient<InMemoryThingDirectory> {
    /// Creates a builder using an in-memory Thing Description Directory.
    pub fn builder() -> ServientBuilder<InMemoryThingDirectory> {
        ServientBuilder::new()
    }

    /// Creates a default in-memory Servient.
    pub fn new() -> Self {
        Self::builder().build()
    }
}

impl Default for Servient<InMemoryThingDirectory> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D> Clone for Servient<D> {
    fn clone(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
            state: Arc::clone(&self.state),
            shutdown: Arc::clone(&self.shutdown),
        }
    }
}

impl<D> Servient<D> {
    /// Wraps already-assembled shared and stateful state in a shared,
    /// clone-able Servient.
    pub(crate) fn from_parts(shared: ServientShared, state: ServientState<D>) -> Self {
        Self {
            #[allow(clippy::arc_with_non_send_sync)]
            shared: Arc::new(shared),
            #[allow(clippy::arc_with_non_send_sync)]
            state: Arc::new(MapLock::new(state)),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Returns a [`ShutdownHandle`] that can signal the serving loops to stop.
    ///
    /// The driving loops (`serve_sync`, `serve`) check this flag between
    /// iterations and exit when set.
    pub fn shutdown_handle(&self) -> ShutdownHandle {
        ShutdownHandle {
            flag: Arc::clone(&self.shutdown),
        }
    }

    /// Runs a closure with exclusive access to the stateful Servient state
    /// (directory + driving layer).
    ///
    /// The lock-free shared registries are accessed directly through
    /// `self.shared.*` without this lock. The closure must not re-enter the
    /// Servient through another `with_state` call: on the sync flavor that
    /// would panic on a double `RefCell` borrow, and on the std flavor it
    /// would deadlock.
    pub(crate) fn with_state<R>(&self, f: impl FnOnce(&mut ServientState<D>) -> R) -> R {
        self.state.with(f)
    }

    /// Constructs an [`InteractionRuntime`] snapshot without acquiring the
    /// state lock — all four registries are kept in the lock-free shared region
    /// and cloning their `Arc` handles is enough.
    pub(crate) fn interaction_runtime(&self) -> InteractionRuntime {
        InteractionRuntime::new(
            self.shared.binding_factories.clone(),
            Arc::clone(&self.shared.payload_codecs),
            Arc::clone(&self.shared.security_providers),
            self.shared.credential_store.clone(),
        )
    }

    // -----------------------------------------------------------------------
    // Directory operations.
    // -----------------------------------------------------------------------

    /// Lists directory entries in deterministic backend order.
    pub fn list(&self) -> DirectoryPage
    where
        D: ThingDirectory,
    {
        self.with_state(|state| state.directory.list())
    }

    /// Queries directory entries with the shared Discovery query model.
    pub fn query(&self, query: DirectoryQuery) -> DirectoryPage
    where
        D: ThingDirectory,
    {
        self.with_state(|state| state.directory.query(query))
    }

    /// Starts a discovery process.
    ///
    /// Returns a [`ThingDiscovery`] process object that the caller drains via
    /// [`ThingDiscovery::next_now`] or [`ThingDiscovery::next`].
    pub fn discover(&self, filter: ThingFilter) -> ServientResult<ThingDiscovery>
    where
        D: ThingDirectory,
    {
        self.with_state(|state| {
            run_discovery(&state.directory, filter).map_err(ServientError::from)
        })
    }

    /// Registers a TD in the directory without exposing local handlers.
    pub fn register(&self, thing: Thing) -> ServientResult<DirectoryEntry>
    where
        D: ThingDirectory,
    {
        self.with_state(|state| state.directory.register(thing).map_err(Into::into))
    }

    /// Updates a TD in the directory.
    ///
    /// After a successful update, any interned consumed-Thing entry for the same
    /// id is invalidated so the next interaction rebuilds form selections and
    /// binding plans from the updated TD (baseline addendum §3 / v3.0 §5.2).
    pub fn update(&self, thing: Thing) -> ServientResult<DirectoryEntry>
    where
        D: ThingDirectory,
    {
        let entry =
            self.with_state(|state| state.directory.update(thing).map_err(ServientError::from))?;
        // Invalidate outside the state lock — consumed_registry has its own
        // interior mutability, so we don't need to keep the directory lock held
        // during invalidation.
        self.shared.consumed_registry.invalidate(&entry.id);
        Ok(entry)
    }

    /// Removes a TD from the directory.
    ///
    /// After a successful removal, any interned consumed-Thing entry for the
    /// same id is invalidated (baseline addendum §3 / v3.0 §5.2).
    pub fn unregister(&self, id: &str) -> ServientResult<Thing>
    where
        D: ThingDirectory,
    {
        let thing =
            self.with_state(|state| state.directory.delete(id).map_err(ServientError::from))?;
        self.shared.consumed_registry.invalidate(id);
        Ok(thing)
    }

    // -----------------------------------------------------------------------
    // Expose / consume / destroy (baseline §6, §10 / addendum §4).
    // -----------------------------------------------------------------------

    /// Exposes a local Thing, immediately registering its inbound serving work
    /// and publishing its TD to the directory (baseline §3 / §6, addendum §4).
    ///
    /// Handlers are attached after `expose`, through the returned
    /// [`ExposedThingHandle`]. Handler completeness is not a gate at `expose`
    /// time.
    pub fn expose(&self, td: Thing) -> ServientResult<ExposedThingHandle<D>>
    where
        D: ThingDirectory,
    {
        let id = thing_id(&td)?;

        // §10 step 2: insert into the exposed registry (own lock — separate
        // from the state lock below).
        self.shared
            .exposed_registry
            .insert(id.clone(), LocalThing::new(td.clone()))?;

        // §10 step 3 + 4: register inbound routes and publish to directory
        // under the state lock so the sequence is consistent from the driving
        // loop's perspective. `td` is moved into the closure (used by reference
        // for binding registration, then moved into the directory).
        let registration: Result<(), ServientError> = self.with_state(|state| {
            for binding in &state.server_bindings {
                if let Err(message) = binding.register_thing(&id, &td) {
                    return Err(ServientError::RouteRegistration(message));
                }
            }
            #[cfg(feature = "async")]
            for binding in &state.async_server_bindings {
                if let Err(message) = binding.register_thing(&id, &td) {
                    // Rollback sync bindings that succeeded.
                    for b in &state.server_bindings {
                        b.unregister_thing(&id);
                    }
                    return Err(ServientError::RouteRegistration(message));
                }
            }

            // §10 step 4: publish to directory. Non-fatal on failure.
            if let Err(directory_err) = state.directory.register(td).map_err(ServientError::from) {
                // The Thing remains locally exposed and servable.
                #[cfg(feature = "std")]
                std::eprintln!(
                    "clinkz-wot expose: non-fatal directory publish failure: {}",
                    directory_err
                );
                #[cfg(not(feature = "std"))]
                let _ = directory_err;
            }

            Ok(())
        });
        if let Err(err) = registration {
            // Rollback step 2.
            self.shared.exposed_registry.destroy(&id);
            return Err(err);
        }

        Ok(ExposedThingHandle::new(
            self.clone(),
            Arc::clone(&self.shared.exposed_registry),
            self.shared.event_broker.clone(),
            id,
        ))
    }

    /// Removes a locally exposed Thing and its directory entry (baseline §10).
    ///
    /// Implements the deferred-removal discipline (baseline §7 edge case): if a
    /// handler for this Thing is in flight (e.g. the handler itself called
    /// `destroy(own_id)`), the entry is marked draining and removed from the
    /// map immediately (preventing new dispatches); the in-flight handler
    /// completes normally and its dispatch epilogue drops the slot.
    ///
    /// Returns the removed Thing id on success. The `LocalThing` is not
    /// returned in the deferred case (it is dropped with the slot when the
    /// in-flight handler finishes).
    pub fn destroy(&self, id: &str) -> ServientResult<String>
    where
        D: ThingDirectory,
    {
        // Remove all event publisher sinks for this Thing (shared broker).
        self.shared.event_broker.remove_thing(&ThingId::from(id));

        // §10 destroy step 1: unregister inbound routes first (state lock).
        self.with_state(|state| {
            for binding in &state.server_bindings {
                binding.unregister_thing(id);
            }
            #[cfg(feature = "async")]
            for binding in &state.async_server_bindings {
                binding.unregister_thing(id);
            }
        });

        // §10 destroy step 2: remove the exposed-registry entry (own lock).
        if !self.shared.exposed_registry.destroy(id) {
            return Err(ServientError::ExposedThingNotFound(id.to_owned()));
        }

        // §10 destroy step 3: unpublish from directory (best-effort).
        if let Err(directory_err) = self
            .with_state(|state| state.directory.delete(id))
            .map_err(ServientError::from)
        {
            #[cfg(feature = "std")]
            std::eprintln!(
                "clinkz-wot destroy: non-fatal directory unpublish failure: {}",
                directory_err
            );
            #[cfg(not(feature = "std"))]
            let _ = directory_err;
        }

        // Invalidate any interned consumed-Thing entry (baseline addendum §3).
        self.shared.consumed_registry.invalidate(id);

        Ok(id.to_owned())
    }

    /// Removes a locally exposed Thing and its directory entry.
    ///
    /// Alias for [`destroy`](Self::destroy) retained for compatibility.
    pub fn unexpose(&self, id: &str) -> ServientResult<String>
    where
        D: ThingDirectory,
    {
        self.destroy(id)
    }

    /// Consumes a remote Thing, returning a handle for outbound interactions
    /// (baseline §6).
    ///
    /// Identity interning: repeated `consume()` of the same Thing id returns
    /// handles that share one canonical live entry, so form selections and
    /// binding plans are computed once and reused (baseline v3.0 §5.1).
    pub fn consume(&self, td: Thing) -> ServientResult<ConsumedThingHandle<D>> {
        let id = thing_id(&td)?;
        // No state lock needed — consumed_registry has its own interior
        // mutability.
        let entry = self.shared.consumed_registry.get_or_insert(id.clone(), td);
        Ok(ConsumedThingHandle::new(self.clone(), id, entry))
    }

    /// Invalidates the interned consumed-Thing entry for `id` (baseline v3.0
    /// §5.2).
    ///
    /// The next `consume()` of the same Thing rebuilds form selections and
    /// binding plans from the updated TD. Used internally after directory
    /// writes (SR-P3) and available as an explicit programmatic entry point.
    pub fn invalidate(&self, id: &str) {
        self.shared.consumed_registry.invalidate(id);
    }

    // -----------------------------------------------------------------------
    // Late composition registration.
    // -----------------------------------------------------------------------

    /// Registers a protocol binding factory.
    pub fn register_binding_factory<F>(&self, factory: F) -> ServientResult<()>
    where
        F: Fn() -> Box<dyn ClientBinding> + 'static,
    {
        self.shared.binding_factories.push(Box::new(factory));
        Ok(())
    }

    /// Registers a payload codec.
    pub fn register_payload_codec(&self, codec: impl PayloadCodec + 'static) -> ServientResult<()> {
        self.shared
            .payload_codecs
            .with(|codecs| codecs.push(Box::new(codec)));
        Ok(())
    }

    /// Registers a security provider.
    pub fn register_security_provider(
        &self,
        provider: impl SecurityProvider + 'static,
    ) -> ServientResult<()> {
        self.shared
            .security_providers
            .with(|providers| providers.push(Box::new(provider)));
        Ok(())
    }

    /// Registers a server binding for inbound interactions (baseline §1 / §4).
    ///
    /// The shared [`EventBroker`] is handed to the binding so it can register
    /// [`PublisherSink`](clinkz_wot_core::PublisherSink)s during subsequent
    /// `expose` calls.
    pub fn register_server_binding(&self, binding: Arc<dyn ServerBinding>) -> ServientResult<()> {
        binding.set_event_broker(self.shared.event_broker.clone());
        self.with_state(|state| state.server_bindings.push(binding));
        Ok(())
    }

    /// Registers an async server binding for native-async inbound driving
    /// (baseline §4, addendum §2.4).
    #[cfg(feature = "async")]
    pub fn register_async_server_binding(
        &self,
        binding: Arc<dyn AsyncServerBinding>,
    ) -> ServientResult<()> {
        binding.set_event_broker(self.shared.event_broker.clone());
        self.with_state(|state| {
            state.async_server_bindings.push(binding);
            state.async_binding_generation = state.async_binding_generation.wrapping_add(1);
        });
        Ok(())
    }

    /// Returns a clone of the shared [`EventBroker`] (baseline §9).
    ///
    /// Advanced callers can use this to register custom
    /// [`PublisherSink`](clinkz_wot_core::PublisherSink)s or inspect subscriber
    /// counts. Normal event emission goes through
    /// [`ExposedThingHandle::emit_event`](crate::ExposedThingHandle::emit_event).
    pub fn event_broker(&self) -> EventBroker {
        self.shared.event_broker.clone()
    }

    // -----------------------------------------------------------------------
    // Driving layer (baseline §4 / addendum §6.2).
    // -----------------------------------------------------------------------

    /// Performs one synchronous driving iteration (baseline §4).
    ///
    /// Polls each registered [`ServerBinding::poll_accept_sync`], dispatches
    /// inbound requests against the exposed Thing registry, and writes
    /// [`InboundResponse`] back through [`ServerBinding::send_response`].
    /// Each call processes at most one inbound request across all registered
    /// bindings, mirroring the stepwise semantics of the async
    /// [`poll_serve`](Self::poll_serve) path.
    ///
    /// On bare `no_std` MCU super-loops, this is the primary driving primitive —
    /// call it once per super-loop iteration alongside other work.
    ///
    /// The outer Servient lock is held only briefly to snapshot the binding list
    /// and to extract per-request dependencies (security/registry/broker). The
    /// user handler and `send_response` run **outside** the outer lock (mirrors
    /// the async take-out / await / return discipline).
    pub fn poll_serve_sync(&self) -> ServientResult<()>
    where
        D: ThingDirectory,
    {
        if self.shutdown.load(Ordering::Relaxed) {
            return Ok(());
        }
        self.poll_serve_sync_step().map(|_| ())
    }

    /// Infinite-loop wrapper around [`poll_serve_sync`](Self::poll_serve_sync)
    /// (baseline §4 / addendum §6.2).
    ///
    /// Intended for std host/cloud single-purpose runtimes that dedicate a
    /// thread to serving. On bare `no_std` MCU super-loops, use
    /// `poll_serve_sync` directly instead.
    #[cfg(feature = "std")]
    pub fn serve_sync(&self)
    where
        D: ThingDirectory,
    {
        let mut idle_streak = 0usize;
        loop {
            if self.shutdown.load(Ordering::Relaxed) {
                break;
            }
            match self.poll_serve_sync_step() {
                Ok(did_work) => {
                    if did_work {
                        idle_streak = 0;
                    } else {
                        idle_streak = idle_streak.saturating_add(1);
                        if idle_streak <= 8 {
                            std::thread::yield_now();
                        } else {
                            std::thread::sleep(std::time::Duration::from_millis(1));
                        }
                    }
                }
                Err(err) => {
                    idle_streak = 0;
                    std::eprintln!("clinkz-wot serve_sync error: {}", err);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Synchronous driving step + dispatch (baseline §4, addendum §6.2).
//
// Mirrors the async take-out / await / return discipline: the outer Servient
// lock is held only briefly to snapshot per-request dependencies; the user
// handler and `send_response` run outside the lock so a slow handler cannot
// block unrelated Servient operations.
// ---------------------------------------------------------------------------

impl<D> Servient<D> {
    /// Runs one synchronous driving step without holding the outer Servient
    /// lock across handler dispatch or `send_response`.
    fn poll_serve_sync_step(&self) -> ServientResult<bool> {
        // Brief lock: snapshot the binding list (cheap Arc clone per binding)
        // and the current cursor.
        let (bindings, start_cursor) =
            self.with_state(|state| (state.server_bindings.clone(), state.sync_binding_cursor));
        let binding_count = bindings.len();
        if binding_count == 0 {
            self.with_state(|state| state.sync_binding_cursor = 0);
            return Ok(false);
        }

        let start = start_cursor % binding_count;
        for offset in 0..binding_count {
            let index = (start + offset) % binding_count;
            if let Some(request) = bindings[index].poll_accept_sync() {
                // Dispatch + send_response run with no outer lock held.
                let response = self.dispatch_inbound(request);
                bindings[index].send_response(response);
                self.with_state(|state| state.sync_binding_cursor = (index + 1) % binding_count);
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Synchronous inbound dispatch (baseline §4).
    ///
    /// Reads lock-free shared state (registries, broker) directly; the handler
    /// runs through `registry.dispatch`, which holds only its own per-Thing slot
    /// lock — never the outer Servient lock.
    fn dispatch_inbound(&self, request: InboundRequest) -> InboundResponse {
        let correlation = request.correlation.clone();

        let resolved_security = match self.shared.exposed_registry.resolve_inbound_security(
            request.thing_id.as_str(),
            &request.target,
            request.operation,
        ) {
            Some(Ok(resolved_security)) => resolved_security,
            Some(Err(core_error)) => {
                return InboundResponse::error(correlation, core_error);
            }
            None => {
                return InboundResponse::error(
                    correlation,
                    CoreError::InboundDispatch(alloc::format!(
                        "Unknown Thing id '{}'",
                        request.thing_id
                    )),
                );
            }
        };

        let principal = match verify_inbound(
            &self.shared.security_providers,
            &request,
            &resolved_security,
        ) {
            Ok(principal) => principal,
            Err(core_err) => return InboundResponse::error(correlation, core_err),
        };

        let registry = Arc::clone(&self.shared.exposed_registry);
        let broker = self.shared.event_broker.clone();

        // Inject the verified principal inside the dispatch closure so the
        // request input is cloned exactly once per inbound request (instead of
        // once for principal injection and again when handed to the handler).
        let output = registry.dispatch(request.thing_id.as_str(), |thing| {
            let mut input = request.input.clone();
            input.principal = Some(principal);
            dispatch_to_handler(thing, &request, input, &broker)
        });

        match output {
            Some(result) => match result {
                Ok(out) => InboundResponse::new(out, correlation),
                Err(core_err) => InboundResponse::error(correlation, core_err),
            },
            None => InboundResponse::error(correlation, CoreError::MissingHandler),
        }
    }
}

// ---------------------------------------------------------------------------
// Async driving layer (baseline §4, addendum §2.4 / §6.2).
// ---------------------------------------------------------------------------

#[cfg(feature = "async")]
impl<D> Servient<D>
where
    D: ThingDirectory + Send + Sync + 'static,
{
    /// Performs one native-async driving iteration (baseline §4).
    ///
    /// Keeps one pending accept future per binding in a persistent
    /// [`FuturesUnordered`], avoiding per-iteration future reconstruction.
    /// When a request arrives from any binding, the accept future for that
    /// binding is replenished, the request is dispatched, and the response is
    /// written back.
    pub async fn poll_serve(&self) -> ServientResult<()> {
        let mut accept_state = self.with_state(|state| {
            if state.async_accept_state.generation != state.async_binding_generation {
                let bindings = state.async_server_bindings.clone();
                state
                    .async_accept_state
                    .rebuild(state.async_binding_generation, &bindings);
            }
            core::mem::take(&mut state.async_accept_state)
        });

        if accept_state.pending.is_empty() {
            return Ok(());
        }

        let Some((binding, request)) = accept_state.pending.next().await else {
            return Ok(());
        };
        accept_state
            .pending
            .push(accept_future_for_binding(binding.clone()));

        self.with_state(|state| {
            state.async_accept_state = accept_state;
        });

        let response = self.dispatch_inbound_async(request).await;
        binding.send_response(response);
        Ok(())
    }

    /// Infinite-loop wrapper around [`poll_serve`](Self::poll_serve)
    /// (baseline §4).
    ///
    /// Takes `self` by value for `'static + Send` spawning.
    pub async fn serve(self) {
        loop {
            if self.shutdown.load(Ordering::Relaxed) {
                break;
            }
            if let Err(_err) = self.poll_serve().await {
                #[cfg(feature = "std")]
                std::eprintln!("clinkz-wot serve error: {}", _err);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Async dispatch (M9 — take-out / await / return pattern).
// ---------------------------------------------------------------------------

#[cfg(feature = "async")]
impl<D> Servient<D>
where
    D: ThingDirectory + Send + Sync + 'static,
{
    /// Async inbound dispatch: verifies security, then dispatches to async
    /// handlers when available, falling back to sync handlers otherwise.
    ///
    /// Uses the take-out / await / return pattern to avoid holding the thing
    /// slot lock across `.await`:
    /// 1. Lock slot → take async handler out → release lock.
    /// 2. Await the handler (no lock held).
    /// 3. Lock slot → return handler → release lock.
    async fn dispatch_inbound_async(&self, request: InboundRequest) -> InboundResponse {
        let correlation = request.correlation.clone();

        // Phase 1: Security verification reading lock-free shared state.
        let resolved_security = match self.shared.exposed_registry.resolve_inbound_security(
            request.thing_id.as_str(),
            &request.target,
            request.operation,
        ) {
            Some(Ok(resolved_security)) => resolved_security,
            Some(Err(core_error)) => {
                return InboundResponse::error(correlation, core_error);
            }
            None => {
                return InboundResponse::error(
                    correlation,
                    CoreError::InboundDispatch(alloc::format!(
                        "Unknown Thing id '{}'",
                        request.thing_id
                    )),
                );
            }
        };

        let principal = match verify_inbound(
            &self.shared.security_providers,
            &request,
            &resolved_security,
        ) {
            Ok(p) => p,
            Err(e) => return InboundResponse::error(correlation, e),
        };

        let registry = Arc::clone(&self.shared.exposed_registry);
        let broker = self.shared.event_broker.clone();

        // Phase 2: Async dispatch (no ServientInner lock held). The input is
        // cloned exactly once per inbound request: the principal moves in and
        // the owned `input` is handed to whichever handler branch fires.
        let output = match dispatch_to_handler_async(&registry, &request, principal, &broker).await
        {
            Some(result) => result,
            None => Err(CoreError::MissingHandler),
        };

        match output {
            Ok(out) => InboundResponse::new(out, correlation),
            Err(core_err) => InboundResponse::error(correlation, core_err),
        }
    }
}

#[cfg(feature = "async")]
async fn dispatch_to_handler_async(
    registry: &InMemoryExposedThingRegistry,
    request: &InboundRequest,
    principal: clinkz_wot_core::Principal,
    broker: &EventBroker,
) -> Option<CoreResult<InteractionOutput>> {
    use clinkz_wot_core::ExposedThing;

    let slot = registry.slot_for(request.thing_id.as_str())?;

    // Build the per-call input once: clone the request input and inject the
    // verified principal. This is the single `input` clone per inbound request.
    let build_input = || {
        let mut input = request.input.clone();
        input.principal = Some(principal.clone());
        input
    };

    match (&request.target, request.operation) {
        (
            clinkz_wot_core::AffordanceTarget::Property(name),
            clinkz_wot_td::data_type::Operation::ReadProperty,
        ) => {
            let handler = slot
                .with_thing(|thing| thing.take_async_read_handler(name))
                .flatten();
            if let Some(mut handler) = handler {
                let result = handler.read(build_input()).await;
                slot.with_thing(|thing| thing.return_async_read_handler(name, handler));
                return Some(result);
            }
            slot.with_thing(|thing| thing.read_property(name, build_input()))
        }
        (
            clinkz_wot_core::AffordanceTarget::Property(name),
            clinkz_wot_td::data_type::Operation::WriteProperty,
        ) => {
            let handler = slot
                .with_thing(|thing| thing.take_async_write_handler(name))
                .flatten();
            if let Some(mut handler) = handler {
                let result = handler.write(build_input()).await;
                slot.with_thing(|thing| thing.return_async_write_handler(name, handler));
                return Some(result);
            }
            slot.with_thing(|thing| thing.write_property(name, build_input()))
        }
        (
            clinkz_wot_core::AffordanceTarget::Action(name),
            clinkz_wot_td::data_type::Operation::InvokeAction,
        ) => {
            let handler = slot
                .with_thing(|thing| thing.take_async_action_handler(name))
                .flatten();
            if let Some(mut handler) = handler {
                let result = handler.invoke(build_input()).await;
                slot.with_thing(|thing| thing.return_async_action_handler(name, handler));
                return Some(result);
            }
            slot.with_thing(|thing| thing.invoke_action(name, build_input()))
        }
        // ObserveProperty, SubscribeEvent, UnsubscribeEvent, UnobserveProperty
        // intentionally fall through to the sync dispatch path. These
        // operations are registration-style (setup/teardown, not long-running
        // I/O), so async handler variants are not provided. The sync handlers
        // complete quickly and do not meaningfully block the async loop.
        _ => slot.with_thing(|thing| dispatch_to_handler(thing, request, build_input(), broker)),
    }
}

fn dispatch_to_handler(
    thing: &mut LocalThing,
    request: &InboundRequest,
    input: InteractionInput,
    broker: &EventBroker,
) -> CoreResult<InteractionOutput> {
    use clinkz_wot_core::ExposedThing;

    match (&request.target, request.operation) {
        (
            clinkz_wot_core::AffordanceTarget::Property(name),
            clinkz_wot_td::data_type::Operation::ReadProperty,
        ) => thing.read_property(name, input),
        (
            clinkz_wot_core::AffordanceTarget::Property(name),
            clinkz_wot_td::data_type::Operation::WriteProperty,
        ) => thing.write_property(name, input),
        (
            clinkz_wot_core::AffordanceTarget::Action(name),
            clinkz_wot_td::data_type::Operation::InvokeAction,
        ) => thing.invoke_action(name, input),
        (
            clinkz_wot_core::AffordanceTarget::Event(name),
            clinkz_wot_td::data_type::Operation::SubscribeEvent,
        ) => {
            let mut sink =
                broker.event_sink(request.thing_id.clone(), EventName::from(name.clone()));
            thing.subscribe_event(name, input, &mut sink)
        }
        (
            clinkz_wot_core::AffordanceTarget::Event(name),
            clinkz_wot_td::data_type::Operation::UnsubscribeEvent,
        ) => {
            // Try the unsubscribe handler; fall back to ack if none registered.
            match thing.unsubscribe_event(name, input) {
                Ok(output) => Ok(output),
                Err(CoreError::MissingHandler) => Ok(InteractionOutput::empty()),
                Err(e) => Err(e),
            }
        }
        (
            clinkz_wot_core::AffordanceTarget::Property(name),
            clinkz_wot_td::data_type::Operation::ObserveProperty,
        ) => {
            let mut sink =
                broker.event_sink(request.thing_id.clone(), EventName::from(name.clone()));
            // Try a dedicated observe handler; fall back to read + emit. The
            // clone is only paid on the observe call so the fallback can move
            // the original `input` into `read_property`.
            match thing.observe_property(name, input.clone(), &mut sink) {
                Ok(output) => Ok(output),
                Err(CoreError::MissingHandler) => {
                    let output = thing.read_property(name, input)?;
                    if let Some(ref payload) = output.payload {
                        let _ = sink.emit(payload.clone());
                    }
                    Ok(output)
                }
                Err(e) => Err(e),
            }
        }
        (
            clinkz_wot_core::AffordanceTarget::Property(_),
            clinkz_wot_td::data_type::Operation::UnobserveProperty,
        ) => Ok(InteractionOutput::empty()),
        _ => Err(CoreError::UnsupportedOperation(format!(
            "Inbound dispatch does not support {:?} on {:?}",
            request.operation, request.target
        ))),
    }
}

fn verify_inbound(
    security_providers: &SecurityProviderRegistry,
    request: &InboundRequest,
    resolved_security: &ResolvedInboundSecurity,
) -> Result<clinkz_wot_core::Principal, CoreError> {
    use clinkz_wot_core::{Principal, PrincipalId, SecurityError, check_scopes};
    use clinkz_wot_td::security_scheme::SecurityScheme;

    security_providers.with(|providers| {
        let mut resolved_principal: Option<Principal> = None;

        for (scheme_name, scheme) in &resolved_security.schemes {
            if matches!(scheme, SecurityScheme::NoSec(_)) {
                continue;
            }

            let provider = providers
                .iter()
                .find(|provider| provider.scheme_name() == scheme_name.as_str())
                .ok_or_else(|| {
                    CoreError::Security(SecurityError::SchemeFailure(format!(
                        "No security provider registered for '{}'",
                        scheme_name
                    )))
                })?;

            if !provider.supports_scopes(&resolved_security.scopes) {
                return Err(CoreError::Security(SecurityError::SchemeFailure(format!(
                    "Security provider '{}' does not support scopes {:?}",
                    scheme_name, resolved_security.scopes
                ))));
            }

            let principal = provider.verify(request, scheme).map_err(CoreError::from)?;
            check_scopes(&resolved_security.scopes, &principal.scopes).map_err(CoreError::from)?;
            resolved_principal = Some(principal);
        }

        Ok(resolved_principal.unwrap_or(Principal {
            id: PrincipalId::from("anonymous"),
            scopes: alloc::vec::Vec::new(),
        }))
    })
}

fn thing_id(thing: &Thing) -> ServientResult<String> {
    thing
        .id
        .as_ref()
        .map(|id| id.as_str().to_owned())
        .ok_or(ServientError::MissingThingId)
}

/// Handle for signaling graceful shutdown to [`Servient::serve_sync`] and
/// [`Servient::serve`].
///
/// Created by [`Servient::shutdown_handle`]. The handle is `Clone` so multiple
/// callers (e.g. signal handler + timeout) can signal shutdown.
#[derive(Debug, Clone)]
pub struct ShutdownHandle {
    flag: Arc<AtomicBool>,
}

impl ShutdownHandle {
    /// Signals the serving loops to stop after the current iteration.
    pub fn shutdown(&self) {
        self.flag.store(true, Ordering::Relaxed);
    }

    /// Returns `true` if shutdown has been signaled.
    pub fn is_shutdown(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }
}
