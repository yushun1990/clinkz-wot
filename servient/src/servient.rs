use alloc::{borrow::ToOwned, boxed::Box, format, string::String, sync::Arc, vec::Vec};

#[cfg(feature = "async")]
use core::future::Future;
#[cfg(feature = "async")]
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "async")]
use core::time::Duration;

#[cfg(feature = "td2-preview")]
use clinkz_wot_core::ActionCancelHandler;
#[cfg(feature = "async")]
use clinkz_wot_core::AsyncServerBinding;
use clinkz_wot_core::{
    ActionHandler, ActionQueryHandler, ClientBinding, CoreError, CoreResult, CredentialStore,
    EventBroker, EventName, EventSink, EventSubscribeHandler, EventUnsubscribeHandler,
    InboundRequest, InboundResponse, InteractionInput, InteractionOutput, LocalThing, MapLock,
    Payload, PayloadCodec, PropertyObserveHandler, PropertyReadHandler, PropertyUnobserveHandler,
    PropertyWriteHandler, SecurityProvider, ServerBinding, ThingId,
};
use clinkz_wot_discovery::{
    DirectoryEntry, DirectoryPage, DirectoryQuery, InMemoryThingDirectory, ThingDirectory,
    ThingDiscovery, ThingFilter, discover as run_discovery,
};
use clinkz_wot_td::{data_type::Operation, thing::Thing};
#[cfg(feature = "async")]
use futures_util::stream::{FuturesUnordered, StreamExt};

use crate::{
    ConsumedThingRegistry, ExposedThingRegistry, ServientBuilder, ServientError, ServientResult,
    handle::{ConsumedThingHandle, ExposedThingHandle},
    interaction::InteractionRuntime,
    registry::ResolvedInboundSecurity,
};

pub(crate) type BindingFactory =
    Box<dyn Fn() -> Box<dyn clinkz_wot_core::ClientBinding + Send + Sync>>;
pub(crate) type BindingFactorySupports =
    Arc<dyn Fn(&Thing, &clinkz_wot_td::form::Form, clinkz_wot_td::data_type::Operation) -> bool>;

pub(crate) struct BindingFactoryEntry {
    pub(crate) make: BindingFactory,
    pub(crate) supports: BindingFactorySupports,
}

pub(crate) type PayloadCodecRegistry = Arc<MapLock<Vec<Arc<dyn PayloadCodec>>>>;
pub(crate) type SecurityProviderRegistry = Arc<MapLock<Arc<Vec<Arc<dyn SecurityProvider>>>>>;
pub(crate) type SyncServerBindingSnapshot = Arc<[Arc<dyn ServerBinding>]>;
#[cfg(feature = "async")]
pub(crate) type AsyncServerBindingSnapshot = Arc<[Arc<dyn AsyncServerBinding>]>;

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
    factories: Vec<BindingFactoryEntry>,
    generation: u64,
}

impl BindingFactoryRegistry {
    /// Wraps a pre-built factory vec (used by `ServientBuilder::build`).
    pub(crate) fn from_factories(factories: Vec<BindingFactoryEntry>) -> Self {
        Self {
            #[allow(clippy::arc_with_non_send_sync)]
            state: Arc::new(MapLock::new(BindingFactoryState {
                factories,
                generation: 0,
            })),
        }
    }

    /// Appends a factory and bumps the generation counter.
    pub(crate) fn push(&self, factory: BindingFactoryEntry) {
        self.state.with_recover(|s| {
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
        self.state.with_recover(|s| s.generation)
    }

    /// Constructs a fresh `Box<dyn ClientBinding>` from the factory at `index`.
    ///
    /// Returns `None` when `index` is out of bounds (the factory was removed
    /// or the registry shrank).
    pub(crate) fn make_binding(
        &self,
        index: usize,
    ) -> Option<Box<dyn ClientBinding + Send + Sync>> {
        self.state
            .with_recover(|s| s.factories.get(index).map(|f| (f.make)()))
    }

    /// Iterates factories to find one whose support predicate accepts the
    /// given form/operation, returning the factory index.
    pub(crate) fn find_supporting_index(
        &self,
        thing: &Thing,
        form: &clinkz_wot_td::form::Form,
        operation: clinkz_wot_td::data_type::Operation,
    ) -> Option<usize> {
        // Snapshot the support predicates under a brief lock (cheap Arc
        // clones), then run them outside the factory lock so a non-trivial /
        // user-supplied predicate does not block factory registration or other
        // lookups.
        let predicates: Vec<(usize, BindingFactorySupports)> = self.state.with_recover(|s| {
            s.factories
                .iter()
                .enumerate()
                .map(|(i, factory)| (i, Arc::clone(&factory.supports)))
                .collect()
        });
        for (index, supports) in predicates {
            if supports(thing, form, operation) {
                return Some(index);
            }
        }
        None
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
    pub(crate) exposed_registry: Arc<ExposedThingRegistry>,
    pub(crate) consumed_registry: Arc<ConsumedThingRegistry>,
    pub(crate) binding_factories: BindingFactoryRegistry,
    pub(crate) payload_codecs: PayloadCodecRegistry,
    pub(crate) security_providers: SecurityProviderRegistry,
    pub(crate) credential_store: Option<Arc<dyn CredentialStore>>,
    pub(crate) event_broker: EventBroker,
    pub(crate) normalize_payloads: bool,
    pub(crate) sync_server_bindings: Arc<MapLock<SyncServerBindingSnapshot>>,
    #[cfg(feature = "async")]
    pub(crate) async_server_bindings: Arc<MapLock<AsyncServerBindingSnapshot>>,
}

/// Stateful Servient state protected by a single outer lock.
///
/// Combines the directory (mutable, exclusive access) with the driving layer
/// (server bindings, cursor, async accept state). Kept under one lock so
/// `expose` / `destroy` can sequence "register routes → publish to directory"
/// atomically from the driving loop's perspective.
pub(crate) struct ServientState<D> {
    pub(crate) directory: D,
    pub(crate) sync_binding_cursor: usize,
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
        self.state.with_recover(f)
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
            self.shared.normalize_payloads,
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

    /// Creates an exposed Thing without starting inbound serving (W3C WoT
    /// Scripting API `WoT.produce`).
    ///
    /// The returned [`ExposedThingHandle`] can receive local in-process
    /// interactions and allows handler registration, but is **not** yet
    /// network-reachable or discoverable. Call
    /// [`ExposedThingHandle::expose`](crate::ExposedThingHandle::expose) to
    /// register inbound routes on server bindings and publish the TD to the
    /// directory.
    ///
    /// This two-phase lifecycle eliminates the window where a Thing is remotely
    /// addressable but has no handlers attached: register handlers between
    /// `produce` and `expose`, then the Thing goes live fully wired.
    pub fn produce(&self, td: Thing) -> ServientResult<ExposedThingHandle<D>> {
        let id = thing_id(&td)?;

        // §10 step 2: insert into the exposed registry (own lock — separate
        // from the state lock used by start_serving).
        self.shared
            .exposed_registry
            .insert(id.clone(), LocalThing::new(td))?;

        Ok(ExposedThingHandle::new(
            self.clone(),
            Arc::clone(&self.shared.exposed_registry),
            self.shared.event_broker.clone(),
            id,
        ))
    }

    /// Exposes a local Thing and immediately starts serving, combining
    /// [`produce`](Self::produce) +
    /// [`ExposedThingHandle::expose`](crate::ExposedThingHandle::expose) in one
    /// step (backward-compatible convenience).
    ///
    /// Handlers are attached after `expose`, through the returned
    /// [`ExposedThingHandle`]. When you need to register handlers *before* the
    /// Thing becomes network-reachable, use [`produce`](Self::produce) +
    /// [`ExposedThingHandle::expose`](crate::ExposedThingHandle::expose)
    /// instead.
    pub fn expose(&self, td: Thing) -> ServientResult<ExposedThingHandle<D>>
    where
        D: ThingDirectory,
    {
        let handle = self.produce(td)?;
        if let Err(err) = handle.expose() {
            // Rollback: destroy the registry entry on serving-start failure.
            let _ = self.shared.exposed_registry.destroy(handle.thing_id());
            return Err(err);
        }
        Ok(handle)
    }

    /// Registers inbound routes on all server bindings and publishes the TD to
    /// the directory (W3C WoT Scripting API `ExposedThing.expose`).
    ///
    /// Called by [`ExposedThingHandle::expose`](crate::ExposedThingHandle::expose)
    /// and by the convenience [`expose`](Self::expose) wrapper. Route-
    /// registration failure is fatal (returns `Err` with rollback of partially
    /// registered routes); directory-publish failure is best-effort.
    pub(crate) fn start_serving(&self, id: &str) -> ServientResult<()>
    where
        D: ThingDirectory,
    {
        let td = self
            .shared
            .exposed_registry
            .thing_description(id)
            .ok_or_else(|| ServientError::ExposedThingNotFound(id.to_owned()))?;

        // Snapshot binding from the single authoritative source so
        // start_serving and the driving loop never observe divergent sets.
        let sync_bindings = self.shared.sync_server_bindings.with(|s| s.clone())?;
        #[cfg(feature = "async")]
        let async_bindings = self.shared.async_server_bindings.with(|s| s.clone())?;

        self.with_state(|state| {
            for binding in sync_bindings.iter() {
                if let Err(message) = binding.register_thing(id, &td) {
                    return Err(ServientError::RouteRegistration(message));
                }
            }
            #[cfg(feature = "async")]
            {
                for binding in async_bindings.iter() {
                    if let Err(message) = binding.register_thing(id, &td) {
                        // Rollback sync bindings that succeeded.
                        for b in sync_bindings.iter() {
                            b.unregister_thing(id);
                        }
                        return Err(ServientError::RouteRegistration(message));
                    }
                }
            }

            // §10 step 4: publish to directory. Non-fatal on failure.
            if let Err(directory_err) = state.directory.register(td).map_err(ServientError::from) {
                #[cfg(feature = "std")]
                std::eprintln!(
                    "clinkz-wot expose: non-fatal directory publish failure: {}",
                    directory_err
                );
                #[cfg(not(feature = "std"))]
                let _ = directory_err;
            }

            Ok(())
        })
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

        // §10 destroy step 1: unregister inbound routes. The binding list comes
        // from the single authoritative source (no second copy to diverge).
        let sync_bindings = self.shared.sync_server_bindings.with(|s| s.clone())?;
        #[cfg(feature = "async")]
        let async_bindings = self.shared.async_server_bindings.with(|s| s.clone())?;
        for binding in sync_bindings.iter() {
            binding.unregister_thing(id);
        }
        #[cfg(feature = "async")]
        for binding in async_bindings.iter() {
            binding.unregister_thing(id);
        }

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

    /// Propagates a runtime-added affordance to the network side: registers the
    /// affordance's routes on every server binding and re-publishes the
    /// post-mutation TD to the directory (W3C Scripting API dynamic affordance
    /// lifecycle). Without this, a new affordance is locally visible but not
    /// remotely reachable or discoverable.
    ///
    /// Route-registration failure is fatal (returns `Err`) so the caller can
    /// roll back the local TD mutation; directory-update failure is best-effort.
    pub(crate) fn sync_added_affordance(
        &self,
        thing_id: &str,
        target: &clinkz_wot_core::AffordanceTarget,
    ) -> ServientResult<()>
    where
        D: ThingDirectory,
    {
        let td = self
            .shared
            .exposed_registry
            .thing_description(thing_id)
            .ok_or_else(|| ServientError::ExposedThingNotFound(thing_id.to_owned()))?;

        let sync_bindings = self.shared.sync_server_bindings.with(|s| s.clone())?;
        #[cfg(feature = "async")]
        let async_bindings = self.shared.async_server_bindings.with(|s| s.clone())?;
        for binding in sync_bindings.iter() {
            binding
                .register_affordance(thing_id, target, &td)
                .map_err(ServientError::RouteRegistration)?;
        }
        #[cfg(feature = "async")]
        for binding in async_bindings.iter() {
            binding
                .register_affordance(thing_id, target, &td)
                .map_err(ServientError::RouteRegistration)?;
        }

        // Best-effort directory re-publish of the post-mutation TD.
        if let Err(err) = self
            .with_state(|state| state.directory.update(td))
            .map_err(ServientError::from)
        {
            #[cfg(feature = "std")]
            std::eprintln!(
                "clinkz-wot: non-fatal directory update after affordance add: {}",
                err
            );
            #[cfg(not(feature = "std"))]
            let _ = err;
        }
        Ok(())
    }

    /// Propagates a runtime-removed affordance: unregisters its routes on every
    /// server binding and re-publishes the post-mutation TD to the directory.
    pub(crate) fn sync_removed_affordance(
        &self,
        thing_id: &str,
        target: &clinkz_wot_core::AffordanceTarget,
    ) where
        D: ThingDirectory,
    {
        let sync_bindings = self
            .shared
            .sync_server_bindings
            .with(|s| s.clone())
            .unwrap_or_default();
        #[cfg(feature = "async")]
        let async_bindings = self
            .shared
            .async_server_bindings
            .with(|s| s.clone())
            .unwrap_or_default();
        for binding in sync_bindings.iter() {
            binding.unregister_affordance(thing_id, target);
        }
        #[cfg(feature = "async")]
        for binding in async_bindings.iter() {
            binding.unregister_affordance(thing_id, target);
        }

        // Best-effort directory re-publish of the post-mutation TD.
        if let Some(td) = self.shared.exposed_registry.thing_description(thing_id)
            && let Err(err) = self
                .with_state(|state| state.directory.update(td))
                .map_err(ServientError::from)
        {
            #[cfg(feature = "std")]
            std::eprintln!(
                "clinkz-wot: non-fatal directory update after affordance remove: {}",
                err
            );
            #[cfg(not(feature = "std"))]
            let _ = err;
        }
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
        let entry = self
            .shared
            .consumed_registry
            .get_or_insert(id.clone(), td)?;
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
        F: Fn() -> Box<dyn ClientBinding + Send + Sync> + 'static,
    {
        self.shared.binding_factories.push(BindingFactoryEntry {
            make: Box::new(factory),
            supports: Arc::new(|_, _, _| true),
        });
        Ok(())
    }

    /// Registers a protocol binding factory with a lightweight support
    /// predicate.
    pub fn register_binding_factory_with_support<F, S>(
        &self,
        factory: F,
        supports: S,
    ) -> ServientResult<()>
    where
        F: Fn() -> Box<dyn ClientBinding + Send + Sync> + 'static,
        S: Fn(&Thing, &clinkz_wot_td::form::Form, clinkz_wot_td::data_type::Operation) -> bool
            + 'static,
    {
        self.shared.binding_factories.push(BindingFactoryEntry {
            make: Box::new(factory),
            supports: Arc::new(supports),
        });
        Ok(())
    }

    /// Registers a payload codec.
    pub fn register_payload_codec(&self, codec: impl PayloadCodec + 'static) -> ServientResult<()> {
        self.shared
            .payload_codecs
            .with(|codecs| codecs.push(Arc::new(codec)))?;
        Ok(())
    }

    /// Registers a security provider.
    pub fn register_security_provider(
        &self,
        provider: impl SecurityProvider + 'static,
    ) -> ServientResult<()> {
        self.shared.security_providers.with(|snapshot| {
            let mut vec = (**snapshot).clone();
            vec.push(Arc::new(provider));
            #[allow(clippy::arc_with_non_send_sync)]
            {
                *snapshot = Arc::new(vec);
            }
        })?;
        Ok(())
    }

    /// Registers a server binding for inbound interactions (baseline §1 / §4).
    ///
    /// The shared [`EventBroker`] is handed to the binding so it can register
    /// [`PublisherSink`](clinkz_wot_core::PublisherSink)s during subsequent
    /// `expose` calls.
    pub fn register_server_binding(&self, binding: Arc<dyn ServerBinding>) -> ServientResult<()> {
        binding.set_event_broker(self.shared.event_broker.clone());
        // Copy-on-write: clone the snapshot slice to a Vec, push, re-wrap in
        // Arc<[...]>. Register is a cold (setup-time) operation; the hot-path
        // poll benefits from a single Arc clone instead of N.
        self.shared.sync_server_bindings.with(|snapshot| {
            let mut vec: Vec<_> = snapshot.to_vec();
            vec.push(binding);
            *snapshot = Arc::from(vec);
        })?;
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
        // Push to the authoritative list first, THEN bump the generation so the
        // driving loop only rebuilds its accept state once the new binding is
        // visible (the previous order bumped generation before the list push,
        // letting the loop rebuild against a stale list).
        self.shared.async_server_bindings.with(|snapshot| {
            let mut vec: Vec<_> = snapshot.to_vec();
            vec.push(binding);
            *snapshot = Arc::from(vec);
        })?;
        self.with_state(|state| {
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
        // Clone the binding list (N Arc refcount bumps) outside the hot path.
        let bindings = self
            .shared
            .sync_server_bindings
            .with(|snapshot| snapshot.clone())?;
        let binding_count = bindings.len();
        if binding_count == 0 {
            self.with_state(|state| {
                state.sync_binding_cursor = 0;
            });
            return Ok(false);
        }

        let start_cursor = self.with_state(|state| state.sync_binding_cursor);

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
        let broker = &self.shared.event_broker;

        // Clone the handler `Arc` out under a brief slot lock and invoke it
        // with the slot lock released (held only by the driving-loop
        // serialization lock `sync_lock`), so the handler may re-enter the
        // Servient for the same Thing without self-deadlock (C7). Emitted
        // payloads are buffered under the run and drained through the broker
        // afterwards.
        let output: Option<CoreResult<InteractionOutput>> =
            registry.slot_for(request.thing_id.as_str()).map(|slot| {
                slot.with_sync_serialization(|| {
                    let mut emitted: Vec<Payload> = Vec::new();
                    let prepared = slot
                        .with_thing(|thing| {
                            let mut input = request.input.clone();
                            input.principal = Some(principal);
                            PreparedDispatch::prepare(thing, &request, input)
                        })
                        .ok_or(CoreError::MissingHandler)?;
                    let prepared = prepared?;
                    let result = prepared.run(&mut BufferingEventSink {
                        buffer: &mut emitted,
                    })?;
                    drain_emitted(broker, &request.thing_id, &request.target, emitted);
                    drain_tagged_emissions(broker, &request.thing_id, result.tagged_emissions);
                    Ok(result.output)
                })
            });

        match output {
            Some(Ok(out)) => InboundResponse::new(out, correlation),
            Some(Err(core_err)) => InboundResponse::error(correlation, core_err),
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
    ///
    /// # Concurrency model
    ///
    /// `poll_serve` dispatches **one request at a time** (it awaits the
    /// dispatch before returning). For **concurrent cross-Thing dispatch**,
    /// use [`serve`](Self::serve), which interleaves accept and dispatch via
    /// `select!` + `FuturesUnordered` — no `tokio::spawn`, no `Send`
    /// requirement. Within a single Thing, dispatch is always serialized by
    /// the per-Thing `async_lock` ([`crate::registry::ThingSlot`]).
    pub async fn poll_serve(&self) -> ServientResult<()> {
        let mut accept_state = self.with_state(|state| {
            if state.async_accept_state.generation != state.async_binding_generation {
                let bindings = self
                    .shared
                    .async_server_bindings
                    .with_recover(|snapshot| snapshot.clone());
                state
                    .async_accept_state
                    .rebuild(state.async_binding_generation, &bindings);
            }
            core::mem::take(&mut state.async_accept_state)
        });

        if accept_state.pending.is_empty() {
            return Ok(());
        }

        // `next().await` on a non-empty `FuturesUnordered` either blocks
        // (Pending) or yields an item (Ready(Some)). It only returns `None`
        // when the collection becomes empty — which the `is_empty()` guard
        // above prevents. The `let-else` is required by syntax; the branch is
        // unreachable in practice.
        let Some((binding, request)) = accept_state.pending.next().await else {
            unreachable!("is_empty guard above guarantees pending is non-empty");
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

    /// Infinite-loop wrapper that accepts requests and dispatches them
    /// **concurrently** (baseline §4).
    ///
    /// Uses `select!` to interleave accept and dispatch: while one dispatch
    /// `.await`s (e.g. a slow async handler), the loop continues accepting new
    /// requests and polling other in-flight dispatches. This gives cross-Thing
    /// async concurrency **without `tokio::spawn`** — the dispatch futures run
    /// on the same task, so `Servient` does not need to be `Send`. Within a
    /// single Thing, dispatch is still serialized by the per-Thing `async_lock`
    /// (baseline §7).
    ///
    /// Takes `self` by value so the loop owns the Servient for its lifetime;
    /// each in-flight dispatch receives a cheap `Servient` clone (Arc bumps).
    pub async fn serve(self) {
        // Build a LOCAL accept state owned by this serve loop (poll_serve
        // uses the one in ServientState for its take-out / put-back model).
        let generation = self.with_state(|s| s.async_binding_generation);
        let initial_bindings = self
            .shared
            .async_server_bindings
            .with_recover(|s| s.clone());
        let mut accept_state = AsyncAcceptState::new();
        accept_state.rebuild(generation, &initial_bindings);

        let mut in_flight: FuturesUnordered<Pin<Box<dyn Future<Output = ()>>>> =
            FuturesUnordered::new();

        loop {
            if self.shutdown.load(Ordering::Relaxed) {
                break;
            }

            tokio::select! {
                // Accept a new request from any binding.
                Some((binding, request)) = accept_state.pending.next() => {
                    // Replenish so this binding keeps accepting.
                    accept_state
                        .pending
                        .push(accept_future_for_binding(binding.clone()));
                    // Dispatch concurrently — push to in-flight instead of
                    // awaiting inline. Multiple dispatches interleave via the
                    // async runtime; a slow handler does not block other
                    // Things' requests.
                    let servient = self.clone();
                    in_flight.push(Box::pin(async move {
                        let response = servient.dispatch_inbound_async(request).await;
                        binding.send_response(response);
                    }));
                }
                // Drain a completed dispatch (response already sent).
                _ = in_flight.next() => {}
                // Periodic generation check + prevents hang when idle.
                _ = tokio::time::sleep(Duration::from_millis(500)) => {
                    let current_gen = self.with_state(|s| s.async_binding_generation);
                    if current_gen != accept_state.generation {
                        let bindings = self
                            .shared
                            .async_server_bindings
                            .with_recover(|s| s.clone());
                        accept_state.rebuild(current_gen, &bindings);
                    }
                }
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
        let broker = &self.shared.event_broker;

        // Phase 2: Async dispatch (no ServientInner lock held). The input is
        // cloned exactly once per inbound request: the principal moves in and
        // the owned `input` is handed to whichever handler branch fires.
        let output = match dispatch_to_handler_async(&registry, &request, principal, broker).await {
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
    registry: &ExposedThingRegistry,
    request: &InboundRequest,
    principal: clinkz_wot_core::Principal,
    broker: &EventBroker,
) -> Option<CoreResult<InteractionOutput>> {
    let slot = registry.slot_for(request.thing_id.as_str())?;

    // Serialize interactions within one Thing (baseline §7). The async lock
    // guard is held across `.await`, which is safe for tokio::sync::Mutex.
    let _async_guard = slot.lock_async().await;

    // Build the per-call input once: clone the request input and inject the
    // verified principal. This is the single `input` clone per inbound request.
    let build_input = || {
        let mut input = request.input.clone();
        input.principal = Some(principal.clone());
        input
    };

    // Try async handlers first (read/write/invoke). When present, they run
    // outside the slot lock (the `Arc` was cloned out under a brief lock).
    match (&request.target, request.operation) {
        (clinkz_wot_core::AffordanceTarget::Property(name), Operation::ReadProperty) => {
            if let Some(handler) = slot.with_thing(|t| t.async_read_handler(name)).flatten() {
                return Some(handler.read(build_input()).await);
            }
        }
        (clinkz_wot_core::AffordanceTarget::Property(name), Operation::WriteProperty) => {
            if let Some(handler) = slot.with_thing(|t| t.async_write_handler(name)).flatten() {
                return Some(handler.write(build_input()).await);
            }
        }
        (clinkz_wot_core::AffordanceTarget::Action(name), Operation::InvokeAction) => {
            if let Some(handler) = slot.with_thing(|t| t.async_action_handler(name)).flatten() {
                return Some(handler.invoke(build_input()).await);
            }
        }
        _ => {}
    }

    // Sync fallback: prepare (clone handler `Arc` out under the brief slot
    // lock) and run outside that lock (async_lock still held), so the handler
    // may re-enter the Servient without self-deadlock (C7). Covers sync
    // read/write/invoke when no async handler is registered, plus the
    // registration-style observe/subscribe/unsubscribe operations.
    let mut emitted: Vec<Payload> = Vec::new();
    let prepared =
        slot.with_thing(|thing| PreparedDispatch::prepare(thing, request, build_input()));
    let result = match prepared {
        None => Err(CoreError::MissingHandler),
        Some(Ok(dispatch)) => dispatch.run(&mut BufferingEventSink {
            buffer: &mut emitted,
        }),
        Some(Err(err)) => Err(err),
    };
    drain_emitted(broker, &request.thing_id, &request.target, emitted);
    Some(result.map(|r| {
        drain_tagged_emissions(broker, &request.thing_id, r.tagged_emissions);
        r.output
    }))
}

/// A handler cloned out under the brief slot lock, ready to run with the slot
/// lock released (C7 reentrancy fix).
///
/// [`PreparedDispatch::prepare`] runs under the per-Thing `thing` lock and
/// captures the handler `Arc` plus its input; [`PreparedDispatch::run`] invokes
/// the handler outside that lock (only the driving-loop serialization lock may
/// remain held), so handler code may re-enter the Servient without self-deadlock.
enum PreparedDispatch {
    Read(Arc<dyn PropertyReadHandler>, InteractionInput),
    Write(Arc<dyn PropertyWriteHandler>, InteractionInput),
    Invoke(Arc<dyn ActionHandler>, InteractionInput),
    Subscribe(Arc<dyn EventSubscribeHandler>, InteractionInput),
    Unsubscribe(Arc<dyn EventUnsubscribeHandler>, InteractionInput),
    Observe(Arc<dyn PropertyObserveHandler>, InteractionInput),
    Unobserve(Arc<dyn PropertyUnobserveHandler>, InteractionInput),
    /// Action query (W3C TD `queryaction`).
    ActionQuery(Arc<dyn ActionQueryHandler>, InteractionInput),
    /// Action cancel with a registered handler (W3C TD `cancelaction`).
    #[cfg(feature = "td2-preview")]
    ActionCancel(Arc<dyn ActionCancelHandler>, InteractionInput),
    /// No unsubscribe handler registered — acknowledge the request inline.
    UnsubscribeAck,
    /// No observe handler registered — acknowledge the request inline.
    UnobserveAck,
    /// No cancel handler registered — acknowledge the request inline.
    #[cfg(feature = "td2-preview")]
    ActionCancelAck,
    /// No observe handler registered — fall back to read + emit initial value.
    ObserveFallbackRead(Arc<dyn PropertyReadHandler>, InteractionInput),
    /// Fan out a `readallproperties` / `readmultipleproperties` request across
    /// the listed property read handlers and combine the results into a single
    /// JSON-object payload (W3C TD §6.3.3).
    BulkReadProperties(
        Vec<(String, Arc<dyn PropertyReadHandler>)>,
        InteractionInput,
    ),
    /// Fan out a `writeallproperties` / `writemultipleproperties` request
    /// across the listed property write handlers, each fed its slice of the
    /// JSON-object request payload (W3C TD §6.3.3).
    BulkWriteProperties(Vec<(String, Arc<dyn PropertyWriteHandler>, InteractionInput)>),
    /// Fan out an `observeallproperties` request across the listed property
    /// observe handlers. Each handler emits through a per-property buffering
    /// sink so emissions route to the correct broker key (W3C TD §6.3.3).
    BulkObserveProperties(
        Vec<(String, Arc<dyn PropertyObserveHandler>)>,
        InteractionInput,
    ),
    /// Fan out an `unobserveallproperties` request across the listed property
    /// unobserve handlers (side-effect only, no streaming output).
    BulkUnobserveProperties(Vec<(String, Arc<dyn PropertyUnobserveHandler>, InteractionInput)>),
    /// Fan out a `subscribeallevents` request across the listed event
    /// subscribe handlers. Each handler emits through a per-event buffering
    /// sink (TD 2.0; requires `td2-preview`).
    #[cfg(feature = "td2-preview")]
    BulkSubscribeEvents(
        Vec<(String, Arc<dyn EventSubscribeHandler>)>,
        InteractionInput,
    ),
    /// Fan out an `unsubscribeallevents` request across the listed event
    /// unsubscribe handlers (side-effect only; TD 2.0, requires `td2-preview`).
    #[cfg(feature = "td2-preview")]
    BulkUnsubscribeEvents(Vec<(String, Arc<dyn EventUnsubscribeHandler>, InteractionInput)>),
    /// Fan out a `queryallactions` request across the listed action query
    /// handlers and combine the results into a single JSON-object payload
    /// (W3C TD §6.3.3).
    BulkQueryActions(Vec<(String, Arc<dyn ActionQueryHandler>)>, InteractionInput),
}

/// Output of [`PreparedDispatch::run`].
struct DispatchResult {
    /// Interaction output (empty for ack / side-effect-only operations).
    output: InteractionOutput,
    /// Per-affordance emissions for bulk streaming fan-out operations
    /// (`observeallproperties`, `subscribeallevents`). Each `(name, payloads)`
    /// pair is drained through the broker keyed by `(thing_id, name)`. Empty
    /// for single-affordance operations whose emissions go through the passed-in
    /// `sink`.
    tagged_emissions: Vec<(String, Vec<Payload>)>,
}

impl PreparedDispatch {
    /// Resolves the affordance + clones the handler `Arc` under the slot lock.
    fn prepare(
        thing: &LocalThing,
        request: &InboundRequest,
        input: InteractionInput,
    ) -> CoreResult<Self> {
        match (&request.target, request.operation) {
            (clinkz_wot_core::AffordanceTarget::Property(name), Operation::ReadProperty) => {
                thing.ensure_property_affordance(name)?;
                let handler = thing.read_handler(name).ok_or(CoreError::MissingHandler)?;
                Ok(Self::Read(handler, input))
            }
            (clinkz_wot_core::AffordanceTarget::Property(name), Operation::WriteProperty) => {
                thing.ensure_property_affordance(name)?;
                let handler = thing.write_handler(name).ok_or(CoreError::MissingHandler)?;
                Ok(Self::Write(handler, input))
            }
            (clinkz_wot_core::AffordanceTarget::Action(name), Operation::InvokeAction) => {
                thing.ensure_action_affordance(name)?;
                let handler = thing
                    .action_handler(name)
                    .ok_or(CoreError::MissingHandler)?;
                Ok(Self::Invoke(handler, input))
            }
            (clinkz_wot_core::AffordanceTarget::Event(name), Operation::SubscribeEvent) => {
                thing.ensure_event_affordance(name)?;
                let handler = thing
                    .subscribe_handler(name)
                    .ok_or(CoreError::MissingHandler)?;
                Ok(Self::Subscribe(handler, input))
            }
            (clinkz_wot_core::AffordanceTarget::Event(name), Operation::UnsubscribeEvent) => {
                thing.ensure_event_affordance(name)?;
                match thing.unsubscribe_handler(name) {
                    Some(handler) => Ok(Self::Unsubscribe(handler, input)),
                    None => Ok(Self::UnsubscribeAck),
                }
            }
            (clinkz_wot_core::AffordanceTarget::Property(name), Operation::ObserveProperty) => {
                thing.ensure_property_affordance(name)?;
                match thing.observe_handler(name) {
                    Some(handler) => Ok(Self::Observe(handler, input)),
                    None => {
                        let handler = thing.read_handler(name).ok_or(CoreError::MissingHandler)?;
                        Ok(Self::ObserveFallbackRead(handler, input))
                    }
                }
            }
            (clinkz_wot_core::AffordanceTarget::Property(name), Operation::UnobserveProperty) => {
                thing.ensure_property_affordance(name)?;
                match thing.unobserve_handler(name) {
                    Some(handler) => Ok(Self::Unobserve(handler, input)),
                    None => Ok(Self::UnobserveAck),
                }
            }
            (clinkz_wot_core::AffordanceTarget::Action(name), Operation::QueryAction) => {
                thing.ensure_action_affordance(name)?;
                let handler = thing
                    .action_query_handler(name)
                    .ok_or(CoreError::MissingHandler)?;
                Ok(Self::ActionQuery(handler, input))
            }
            // `cancelaction` is a TD 2.0 operation.
            #[cfg(feature = "td2-preview")]
            (clinkz_wot_core::AffordanceTarget::Action(name), Operation::CancelAction) => {
                thing.ensure_action_affordance(name)?;
                match thing.action_cancel_handler(name) {
                    Some(handler) => Ok(Self::ActionCancel(handler, input)),
                    None => Ok(Self::ActionCancelAck),
                }
            }
            // Bulk property reads (W3C TD §6.3.3). Fan out across the property
            // read handlers and combine the results into a single JSON-object
            // payload. `readallproperties` targets every declared property;
            // `readmultipleproperties` targets the names carried by the request
            // payload as a JSON array (e.g. `["temp","hum"]`).
            (clinkz_wot_core::AffordanceTarget::Thing, Operation::ReadAllProperties) => {
                let names = thing
                    .thing_description()
                    .properties
                    .as_ref()
                    .map(|props| props.keys().cloned().collect::<Vec<_>>())
                    .unwrap_or_default();
                Ok(Self::BulkReadProperties(
                    collect_read_handlers(thing, &names)?,
                    input,
                ))
            }
            (clinkz_wot_core::AffordanceTarget::Thing, Operation::ReadMultipleProperties) => {
                let names = parse_read_multiple_names(&input)?;
                Ok(Self::BulkReadProperties(
                    collect_read_handlers(thing, &names)?,
                    input,
                ))
            }
            // Bulk property writes (W3C TD §6.3.3). The request payload is a
            // JSON object mapping property names to their new values. Each
            // (name, value) pair is dispatched to its write handler with the
            // value serialized as the per-property interaction input payload.
            (clinkz_wot_core::AffordanceTarget::Thing, Operation::WriteAllProperties)
            | (clinkz_wot_core::AffordanceTarget::Thing, Operation::WriteMultipleProperties) => {
                let content_type = bulk_content_type(&input);
                Ok(Self::BulkWriteProperties(collect_write_handlers(
                    thing,
                    &input,
                    content_type.as_str(),
                )?))
            }
            // Bulk observe (W3C TD §6.3.3). Fan out across the property
            // observe handlers for every observable property.
            (clinkz_wot_core::AffordanceTarget::Thing, Operation::ObserveAllProperties) => {
                let names = observable_property_names(thing.thing_description());
                Ok(Self::BulkObserveProperties(
                    collect_observe_handlers(thing, &names)?,
                    input,
                ))
            }
            // Bulk unobserve (W3C TD §6.3.3). Fan out across the property
            // unobserve handlers; unobserved properties without a handler are
            // acked.
            (clinkz_wot_core::AffordanceTarget::Thing, Operation::UnobserveAllProperties) => {
                let names = observable_property_names(thing.thing_description());
                Ok(Self::BulkUnobserveProperties(collect_unobserve_handlers(
                    thing, &names, &input,
                )))
            }
            // Bulk subscribe (`subscribeallevents`) and unsubscribe
            // (`unsubscribeallevents`) are TD 2.0 event meta-operations.
            #[cfg(feature = "td2-preview")]
            (clinkz_wot_core::AffordanceTarget::Thing, Operation::SubscribeAllEvents) => {
                let names = event_names(thing.thing_description());
                Ok(Self::BulkSubscribeEvents(
                    collect_subscribe_handlers(thing, &names)?,
                    input,
                ))
            }
            #[cfg(feature = "td2-preview")]
            (clinkz_wot_core::AffordanceTarget::Thing, Operation::UnsubscribeAllEvents) => {
                let names = event_names(thing.thing_description());
                Ok(Self::BulkUnsubscribeEvents(collect_unsubscribe_handlers(
                    thing, &names, &input,
                )))
            }
            // Bulk query actions (W3C TD §6.3.3). Fan out across action query
            // handlers and combine results. Actions without a query handler are
            // skipped.
            (clinkz_wot_core::AffordanceTarget::Thing, Operation::QueryAllActions) => {
                let names = action_names(thing.thing_description());
                Ok(Self::BulkQueryActions(
                    collect_action_query_handlers(thing, &names),
                    input,
                ))
            }
            _ => Err(CoreError::UnsupportedOperation(alloc::format!(
                "Inbound dispatch does not support {:?} on {:?}",
                request.operation,
                request.target
            ))),
        }
    }

    /// Invokes the handler outside the slot lock. Single-affordance emissions go
    /// through `sink`; bulk streaming fan-out emissions are returned as tagged
    /// `(affordance_name, payloads)` pairs in [`DispatchResult`].
    fn run(self, sink: &mut dyn EventSink) -> CoreResult<DispatchResult> {
        let empty_emissions = Vec::new();
        match self {
            Self::Read(handler, input) => Ok(DispatchResult {
                output: handler.read(input)?,
                tagged_emissions: empty_emissions,
            }),
            Self::Write(handler, input) => Ok(DispatchResult {
                output: handler.write(input)?,
                tagged_emissions: empty_emissions,
            }),
            Self::Invoke(handler, input) => Ok(DispatchResult {
                output: handler.invoke(input)?,
                tagged_emissions: empty_emissions,
            }),
            Self::Subscribe(handler, input) => Ok(DispatchResult {
                output: handler.subscribe(input, sink)?,
                tagged_emissions: empty_emissions,
            }),
            Self::Unsubscribe(handler, input) => Ok(DispatchResult {
                output: handler.unsubscribe(input)?,
                tagged_emissions: empty_emissions,
            }),
            Self::Observe(handler, input) => Ok(DispatchResult {
                output: handler.observe(input, sink)?,
                tagged_emissions: empty_emissions,
            }),
            Self::Unobserve(handler, input) => Ok(DispatchResult {
                output: handler.unobserve(input)?,
                tagged_emissions: empty_emissions,
            }),
            Self::ActionQuery(handler, input) => Ok(DispatchResult {
                output: handler.query(input)?,
                tagged_emissions: empty_emissions,
            }),
            #[cfg(feature = "td2-preview")]
            Self::ActionCancel(handler, input) => Ok(DispatchResult {
                output: handler.cancel(input)?,
                tagged_emissions: empty_emissions,
            }),
            Self::UnsubscribeAck | Self::UnobserveAck => Ok(DispatchResult {
                output: InteractionOutput::empty(),
                tagged_emissions: empty_emissions,
            }),
            #[cfg(feature = "td2-preview")]
            Self::ActionCancelAck => Ok(DispatchResult {
                output: InteractionOutput::empty(),
                tagged_emissions: empty_emissions,
            }),
            Self::ObserveFallbackRead(handler, input) => {
                let output = handler.read(input)?;
                if let Some(ref payload) = output.payload {
                    let _ = sink.emit(payload.clone());
                }
                Ok(DispatchResult {
                    output,
                    tagged_emissions: empty_emissions,
                })
            }
            Self::BulkReadProperties(entries, input) => Ok(DispatchResult {
                output: run_bulk_read(entries, input)?,
                tagged_emissions: empty_emissions,
            }),
            Self::BulkWriteProperties(entries) => {
                for (_name, handler, value_input) in entries {
                    handler.write(value_input)?;
                }
                Ok(DispatchResult {
                    output: InteractionOutput::empty(),
                    tagged_emissions: empty_emissions,
                })
            }
            Self::BulkObserveProperties(entries, input) => {
                run_bulk_streaming(entries, input, |handler, input, sink| {
                    handler.observe(input, sink)
                })
            }
            Self::BulkUnobserveProperties(entries) => {
                for (_name, handler, value_input) in entries {
                    handler.unobserve(value_input)?;
                }
                Ok(DispatchResult {
                    output: InteractionOutput::empty(),
                    tagged_emissions: empty_emissions,
                })
            }
            #[cfg(feature = "td2-preview")]
            Self::BulkSubscribeEvents(entries, input) => {
                run_bulk_streaming(entries, input, |handler, input, sink| {
                    handler.subscribe(input, sink)
                })
            }
            #[cfg(feature = "td2-preview")]
            Self::BulkUnsubscribeEvents(entries) => {
                for (_name, handler, value_input) in entries {
                    handler.unsubscribe(value_input)?;
                }
                Ok(DispatchResult {
                    output: InteractionOutput::empty(),
                    tagged_emissions: empty_emissions,
                })
            }
            Self::BulkQueryActions(entries, input) => Ok(DispatchResult {
                output: run_bulk_query_actions(entries, input)?,
                tagged_emissions: empty_emissions,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Bulk property operation helpers (W3C TD §6.3.3).
//
// `readallproperties` / `readmultipleproperties` fan out across property read
// handlers and combine their outputs into a single JSON-object payload keyed by
// property name. `writeallproperties` / `writemultipleproperties` split a
// JSON-object request payload into per-property write inputs. Both directions
// treat the bulk payload as `application/json` (the TD default content type)
// when no other content type is declared on the request.
// ---------------------------------------------------------------------------

/// Default content type used when assembling or parsing bulk payloads.
const BULK_CONTENT_TYPE: &str = "application/json";

/// A single `(name, write handler, per-property input)` entry prepared for a
/// bulk write dispatch.
type PreparedBulkWriteEntry = (String, Arc<dyn PropertyWriteHandler>, InteractionInput);

/// Returns the content type carried by the bulk request payload, falling back
/// to the WoT default when none is declared.
fn bulk_content_type(input: &InteractionInput) -> String {
    input
        .payload
        .as_ref()
        .map(|payload| payload.content_type.clone())
        .filter(|content_type| !content_type.is_empty())
        .unwrap_or_else(|| String::from(BULK_CONTENT_TYPE))
}

/// Collects `(name, read handler)` pairs for the listed property names.
///
/// A property without a registered read handler is skipped rather than failing
/// the whole bulk request, matching the tolerant fan-out semantics of
/// [`ExposedThingHandle::read_all_properties`]. Returns `MissingHandler` only
/// when no listed property has a handler at all.
fn collect_read_handlers(
    thing: &LocalThing,
    names: &[String],
) -> CoreResult<Vec<(String, Arc<dyn PropertyReadHandler>)>> {
    let mut entries = Vec::new();
    for name in names {
        if let Some(handler) = thing.read_handler(name) {
            entries.push((name.clone(), handler));
        }
    }
    if entries.is_empty() {
        return Err(CoreError::MissingHandler);
    }
    Ok(entries)
}

/// Parses a `readmultipleproperties` request payload (a JSON array of property
/// names, e.g. `["temp","hum"]`) into an owned name list.
///
/// When the payload is missing or not a JSON array, an empty name list is
/// returned so `collect_read_handlers` surfaces a clear `MissingHandler` error
/// instead of a confusing deserialization failure.
fn parse_read_multiple_names(input: &InteractionInput) -> CoreResult<Vec<String>> {
    let Some(payload) = input.payload.as_ref() else {
        return Ok(Vec::new());
    };
    let names: Vec<String> = serde_json::from_slice(payload.body.as_slice()).map_err(|err| {
        CoreError::InvalidInteraction(alloc::format!(
            "readmultipleproperties request payload is not a JSON array of names: {err}"
        ))
    })?;
    Ok(names)
}

/// Collects `(name, write handler, per-property input)` triples for a bulk
/// write request.
///
/// The request payload is a JSON object mapping property names to their new
/// values. Each value is re-serialized into a standalone
/// [`clinkz_wot_core::Payload`] and wrapped in an [`InteractionInput`] that
/// preserves the caller's URI variables and security metadata.
fn collect_write_handlers(
    thing: &LocalThing,
    input: &InteractionInput,
    content_type: &str,
) -> CoreResult<Vec<PreparedBulkWriteEntry>> {
    let Some(payload) = input.payload.as_ref() else {
        return Err(CoreError::InvalidInteraction(alloc::format!(
            "{} request payload is missing",
            "writeallproperties/writemultipleproperties"
        )));
    };
    let map: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(payload.body.as_slice()).map_err(|err| {
            CoreError::InvalidInteraction(alloc::format!(
                "bulk write request payload is not a JSON object: {err}"
            ))
        })?;

    let mut entries = Vec::new();
    for (name, value) in map {
        let Some(handler) = thing.write_handler(&name) else {
            continue;
        };
        let body = serde_json::to_vec(&value).map_err(|err| {
            CoreError::InvalidInteraction(alloc::format!(
                "failed to serialize bulk write value for '{name}': {err}"
            ))
        })?;
        let value_input = InteractionInput {
            payload: Some(Payload::new(body, content_type)),
            parameters: input.parameters.clone(),
            principal: input.principal.clone(),
            security_metadata: input.security_metadata.clone(),
        };
        entries.push((name, handler, value_input));
    }

    if entries.is_empty() {
        return Err(CoreError::MissingHandler);
    }
    Ok(entries)
}

/// Runs a bulk read, combining each handler's output payload into a single
/// JSON-object response keyed by property name.
///
/// Each handler output is parsed as a JSON value; non-JSON payloads are wrapped
/// in a JSON string so the combined object stays valid JSON. An empty handler
/// output contributes a JSON `null` entry.
fn run_bulk_read(
    entries: Vec<(String, Arc<dyn PropertyReadHandler>)>,
    input: InteractionInput,
) -> CoreResult<InteractionOutput> {
    let mut combined = serde_json::Map::new();
    for (name, handler) in entries {
        let output = handler.read(input.clone())?;
        let value = match output.payload {
            Some(payload) if !payload.body.is_empty() => {
                serde_json::from_slice::<serde_json::Value>(payload.body.as_slice()).unwrap_or_else(
                    |_| {
                        serde_json::Value::String(
                            alloc::string::String::from_utf8_lossy(payload.body.as_slice())
                                .into_owned(),
                        )
                    },
                )
            }
            _ => serde_json::Value::Null,
        };
        combined.insert(name, value);
    }

    let body = serde_json::to_vec(&serde_json::Value::Object(combined)).map_err(|err| {
        CoreError::InvalidInteraction(alloc::format!(
            "failed to serialize bulk read response: {err}"
        ))
    })?;
    Ok(InteractionOutput::with_payload(Payload::new(
        body,
        BULK_CONTENT_TYPE,
    )))
}

/// [`EventSink`] that buffers emitted payloads for deferred fan-out.
///
/// Used by the inbound dispatch path to collect payloads emitted while the
/// per-Thing slot lock is held, so they can be pushed through the
/// [`EventBroker`] after the lock is released.
struct BufferingEventSink<'a> {
    buffer: &'a mut Vec<Payload>,
}

impl<'a> EventSink for BufferingEventSink<'a> {
    fn emit(&mut self, payload: Payload) -> CoreResult<()> {
        self.buffer.push(payload);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Bulk streaming and action query helpers (W3C TD §6.3.3).
// ---------------------------------------------------------------------------

/// Returns the names of all observable properties declared in the TD.
fn observable_property_names(thing: &Thing) -> Vec<String> {
    thing
        .properties
        .as_ref()
        .map(|props| {
            props
                .iter()
                .filter(|(_, p)| p.observable)
                .map(|(name, _)| name.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Returns the names of all events declared in the TD.
#[cfg(feature = "td2-preview")]
fn event_names(thing: &Thing) -> Vec<String> {
    thing
        .events
        .as_ref()
        .map(|events| events.keys().cloned().collect())
        .unwrap_or_default()
}

/// Returns the names of all actions declared in the TD.
fn action_names(thing: &Thing) -> Vec<String> {
    thing
        .actions
        .as_ref()
        .map(|actions| actions.keys().cloned().collect())
        .unwrap_or_default()
}

/// Collects `(name, observe handler)` pairs for the listed property names.
///
/// A property without a registered observe handler is skipped. Returns
/// `MissingHandler` only when no listed property has a handler at all.
fn collect_observe_handlers(
    thing: &LocalThing,
    names: &[String],
) -> CoreResult<Vec<(String, Arc<dyn PropertyObserveHandler>)>> {
    let mut entries = Vec::new();
    for name in names {
        if let Some(handler) = thing.observe_handler(name) {
            entries.push((name.clone(), handler));
        }
    }
    if entries.is_empty() {
        return Err(CoreError::MissingHandler);
    }
    Ok(entries)
}

/// Collects `(name, unobserve handler, input)` triples for the listed property
/// names. Properties without an unobserve handler produce no entry (the inbound
/// dispatcher acks those inline).
fn collect_unobserve_handlers(
    thing: &LocalThing,
    names: &[String],
    input: &InteractionInput,
) -> Vec<(String, Arc<dyn PropertyUnobserveHandler>, InteractionInput)> {
    let mut entries = Vec::new();
    for name in names {
        if let Some(handler) = thing.unobserve_handler(name) {
            entries.push((name.clone(), handler, input.clone()));
        }
    }
    entries
}

/// Collects `(name, subscribe handler)` pairs for the listed event names.
#[cfg(feature = "td2-preview")]
fn collect_subscribe_handlers(
    thing: &LocalThing,
    names: &[String],
) -> CoreResult<Vec<(String, Arc<dyn EventSubscribeHandler>)>> {
    let mut entries = Vec::new();
    for name in names {
        if let Some(handler) = thing.subscribe_handler(name) {
            entries.push((name.clone(), handler));
        }
    }
    if entries.is_empty() {
        return Err(CoreError::MissingHandler);
    }
    Ok(entries)
}

/// Collects `(name, unsubscribe handler, input)` triples for the listed event
/// names.
#[cfg(feature = "td2-preview")]
fn collect_unsubscribe_handlers(
    thing: &LocalThing,
    names: &[String],
    input: &InteractionInput,
) -> Vec<(String, Arc<dyn EventUnsubscribeHandler>, InteractionInput)> {
    let mut entries = Vec::new();
    for name in names {
        if let Some(handler) = thing.unsubscribe_handler(name) {
            entries.push((name.clone(), handler, input.clone()));
        }
    }
    entries
}

/// Collects `(name, query handler)` pairs for the listed action names.
fn collect_action_query_handlers(
    thing: &LocalThing,
    names: &[String],
) -> Vec<(String, Arc<dyn ActionQueryHandler>)> {
    let mut entries = Vec::new();
    for name in names {
        if let Some(handler) = thing.action_query_handler(name) {
            entries.push((name.clone(), handler));
        }
    }
    entries
}

/// Runs a bulk streaming fan-out (`observeallproperties` /
/// `subscribeallevents`), invoking each handler through a per-affordance
/// buffering sink so emissions are tagged with the correct affordance name for
/// broker routing.
fn run_bulk_streaming<H>(
    entries: Vec<(String, Arc<H>)>,
    input: InteractionInput,
    invoke: fn(&Arc<H>, InteractionInput, &mut dyn EventSink) -> CoreResult<InteractionOutput>,
) -> CoreResult<DispatchResult>
where
    H: ?Sized,
{
    let mut tagged_emissions: Vec<(String, Vec<Payload>)> = Vec::new();
    for (name, handler) in entries {
        let mut emitted: Vec<Payload> = Vec::new();
        invoke(
            &handler,
            input.clone(),
            &mut BufferingEventSink {
                buffer: &mut emitted,
            },
        )?;
        if !emitted.is_empty() {
            tagged_emissions.push((name, emitted));
        }
    }
    Ok(DispatchResult {
        output: InteractionOutput::empty(),
        tagged_emissions,
    })
}

/// Runs a bulk action query (`queryallactions`), combining each handler's
/// output payload into a single JSON-object response keyed by action name.
/// When no query handlers are registered, returns an empty JSON object.
fn run_bulk_query_actions(
    entries: Vec<(String, Arc<dyn ActionQueryHandler>)>,
    input: InteractionInput,
) -> CoreResult<InteractionOutput> {
    let mut combined = serde_json::Map::new();
    for (name, handler) in entries {
        let output = handler.query(input.clone())?;
        let value = match output.payload {
            Some(payload) if !payload.body.is_empty() => {
                serde_json::from_slice::<serde_json::Value>(payload.body.as_slice()).unwrap_or_else(
                    |_| {
                        serde_json::Value::String(
                            alloc::string::String::from_utf8_lossy(payload.body.as_slice())
                                .into_owned(),
                        )
                    },
                )
            }
            _ => serde_json::Value::Null,
        };
        combined.insert(name, value);
    }

    let body = serde_json::to_vec(&serde_json::Value::Object(combined)).map_err(|err| {
        CoreError::InvalidInteraction(alloc::format!(
            "failed to serialize queryallactions response: {err}"
        ))
    })?;
    Ok(InteractionOutput::with_payload(Payload::new(
        body,
        BULK_CONTENT_TYPE,
    )))
}

/// Returns the broker event-name key for an affordance target, if it is one
/// that emits through the broker (events and observed properties).
fn event_name_for_target(target: &clinkz_wot_core::AffordanceTarget) -> Option<&str> {
    match target {
        clinkz_wot_core::AffordanceTarget::Event(name)
        | clinkz_wot_core::AffordanceTarget::Property(name) => Some(name.as_str()),
        _ => None,
    }
}

/// Drains buffered payloads through the broker, keyed by the request target.
///
/// Only subscribe/observe operations emit; for any other target the buffer is
/// expected to be empty and this is a no-op.
fn drain_emitted(
    broker: &EventBroker,
    thing_id: &ThingId,
    target: &clinkz_wot_core::AffordanceTarget,
    emitted: Vec<Payload>,
) {
    if emitted.is_empty() {
        return;
    }
    let Some(name) = event_name_for_target(target) else {
        return;
    };
    let event = EventName::from(name);
    for payload in emitted {
        let _ = broker.publish(thing_id, &event, &payload);
    }
}

/// Drains per-affordance tagged emissions through the broker.
///
/// Each `(name, payloads)` pair is published to the broker under
/// `(thing_id, name)` so the correct per-affordance `PublisherSink` receives
/// the payloads.
fn drain_tagged_emissions(
    broker: &EventBroker,
    thing_id: &ThingId,
    tagged: Vec<(String, Vec<Payload>)>,
) {
    for (name, payloads) in tagged {
        let event = EventName::from(name);
        for payload in payloads {
            let _ = broker.publish(thing_id, &event, &payload);
        }
    }
}

fn verify_inbound(
    security_providers: &SecurityProviderRegistry,
    request: &InboundRequest,
    resolved_security: &ResolvedInboundSecurity,
) -> Result<clinkz_wot_core::Principal, CoreError> {
    use clinkz_wot_core::{Principal, PrincipalId, SecurityError, check_scopes};
    use clinkz_wot_td::security_scheme::SecurityScheme;

    // Snapshot provider handles under a brief lock, then verify *outside* the
    // registry lock so a slow provider (e.g. JWT key fetch, network retrieval)
    // does not serialize every inbound request across every Thing.
    let providers = security_providers.with(|snapshot| Arc::clone(snapshot))?;

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
