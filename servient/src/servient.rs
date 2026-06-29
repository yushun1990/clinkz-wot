use alloc::{borrow::ToOwned, boxed::Box, string::String, sync::Arc, vec::Vec};

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[cfg(feature = "async")]
use clinkz_wot_core::AsyncServerBinding;
use clinkz_wot_core::{
    ClientBinding, EventBroker, LocalThing, MapLock, PayloadCodec, SecurityProvider, ServerBinding,
    ThingId,
};
use clinkz_wot_discovery::{
    DirectoryEntry, DirectoryPage, DirectoryQuery, InMemoryThingDirectory, ThingDirectory,
    ThingDiscovery, ThingFilter, discover as run_discovery,
};
use clinkz_wot_td::thing::Thing;

use crate::{
    ConsumedThingRegistry, ExposedThingRegistry, ServientBuilder, ServientError, ServientResult,
    handle::{ConsumedThingHandle, ExposedThingHandle},
};

mod bulk;
mod dispatch;
#[cfg(feature = "async")]
mod driving_async;
mod driving_sync;
mod security;

#[cfg(feature = "async")]
pub(crate) use driving_async::{AsyncAcceptState, DrivingState};

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

/// Lock-free (or interior-mutable) shared Servient state.
///
/// Interior-mutable shared state that does not need to coordinate with the
/// directory or the driving layer.
///
/// Built once by `ServientBuilder::build`. Holding these outside the directory
/// lock lets `interaction_runtime`, `dispatch_inbound`, and other hot paths
/// clone shared `Arc` references without acquiring the directory lock.
///
/// `sync_binding_cursor` is an `AtomicUsize` because it is driving-loop
/// private bookkeeping (round-robin fairness hint) and has no coordination
/// requirement with any other state — keeping it out of any `MapLock` lets the
/// driving loop update it without contending with directory operations or
/// route registration.
pub(crate) struct ServientShared {
    pub(crate) exposed_registry: Arc<ExposedThingRegistry>,
    pub(crate) consumed_registry: Arc<ConsumedThingRegistry>,
    pub(crate) binding_factories: BindingFactoryRegistry,
    pub(crate) payload_codecs: PayloadCodecRegistry,
    pub(crate) security_providers: SecurityProviderRegistry,
    pub(crate) event_broker: EventBroker,
    /// Snapshot of the consumed-interaction runtime, built once at
    /// `ServientBuilder::build` time. Every consumed interaction reads this
    /// single `&InteractionRuntime` instead of rebuilding a throwaway struct
    /// (4 `Arc` clones) per request. The registries inside it are the same
    /// `Arc<MapLock<…>>` handles stored alongside, so post-build mutations
    /// (`register_payload_codec`, `register_security_provider`, …) stay visible
    /// through this snapshot. It also owns `credential_store` and
    /// `normalize_payloads`, which are read-only after build.
    pub(crate) interaction: crate::interaction::InteractionRuntime,
    pub(crate) sync_server_bindings: Arc<MapLock<SyncServerBindingSnapshot>>,
    #[cfg(feature = "async")]
    pub(crate) async_server_bindings: Arc<MapLock<AsyncServerBindingSnapshot>>,
    /// Round-robin cursor for the synchronous driving loop. Pure fairness
    /// hint: the driving loop is the only reader/writer, and `% binding_count`
    /// absorbs binding-list size changes without coordination.
    pub(crate) sync_binding_cursor: AtomicUsize,
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
/// # Lock architecture (post lock-split)
///
/// State is split into three `Arc`-shared regions, each with its own
/// synchronization:
///
/// - [`ServientShared`] holds the registries, broker, server-binding
///   snapshots, and the sync driving cursor. Each registry has its own
///   interior mutability; `sync_binding_cursor` is a lock-free
///   `AtomicUsize`. No outer lock is acquired on the interaction or dispatch
///   hot paths.
/// - `directory: Arc<MapLock<D>>` carries the Thing Description Directory
///   behind its own lock. All `ThingDirectory` operations (`list`, `query`,
///   `register`, `update`, `unregister`) acquire only this lock — they no
///   longer coordinate with the driving layer.
/// - `driving: Arc<MapLock<DrivingState>>` (feature `async` only) holds the
///   async accept-state take-out/put-back region. The sync driving loop does
///   not touch it.
///
/// `expose()` / `destroy()` / runtime affordance mutation do **not** hold a
/// global state lock: route registration runs without any outer lock
/// (`register_thing`/`unregister_thing` use each binding's own interior
/// mutability), and the directory write acquires only `directory`. The brief
/// visibility window between "routes registered" and "TD published to the
/// directory" is documented in `docs/baseline/servient-design-baseline.md`
/// §10 and is acceptable for the W3C WoT Discovery model (the local driving
/// loop dispatches from the exposed-Thing registry, not from the directory,
/// so servability is never gated on directory publication).
pub struct Servient<D = InMemoryThingDirectory> {
    shared: Arc<ServientShared>,
    directory: Arc<MapLock<D>>,
    #[cfg(feature = "async")]
    driving: Arc<MapLock<DrivingState>>,
    shutdown: Arc<AtomicBool>,
    /// Wakes the async `serve` loop when async bindings are registered or
    /// shutdown is signaled, so it does not need a periodic timer (the previous
    /// 500 ms poll) and so a freshly registered binding starts accepting
    /// immediately instead of up to 500 ms later.
    #[cfg(feature = "async")]
    wake: Arc<tokio::sync::Notify>,
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
            directory: Arc::clone(&self.directory),
            #[cfg(feature = "async")]
            driving: Arc::clone(&self.driving),
            shutdown: Arc::clone(&self.shutdown),
            #[cfg(feature = "async")]
            wake: Arc::clone(&self.wake),
        }
    }
}

impl<D> Servient<D> {
    /// Wraps already-assembled shared state, directory, and (async only)
    /// driving state in a shared, clone-able Servient.
    pub(crate) fn from_parts(
        shared: ServientShared,
        directory: D,
        #[cfg(feature = "async")] driving: DrivingState,
    ) -> Self {
        Self {
            #[allow(clippy::arc_with_non_send_sync)]
            shared: Arc::new(shared),
            #[allow(clippy::arc_with_non_send_sync)]
            directory: Arc::new(MapLock::new(directory)),
            #[cfg(feature = "async")]
            #[allow(clippy::arc_with_non_send_sync)]
            driving: Arc::new(MapLock::new(driving)),
            shutdown: Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "async")]
            wake: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Acquires the directory lock exclusively and runs `f` against the owned
    /// directory.
    ///
    /// Used by mutating operations (`register`/`update`/`delete`/publish).
    /// Read-only operations (`list`/`query`/`discover`) should use
    /// [`with_directory_read`](Self::with_directory_read) so concurrent readers
    /// proceed in parallel on the `std` build instead of serializing through a
    /// write lock.
    pub(crate) fn with_directory<R>(&self, f: impl FnOnce(&mut D) -> R) -> R {
        self.directory.with_recover(f)
    }

    /// Acquires a **read** directory lock and runs `f` against the directory.
    ///
    /// Dedicated to read-only operations (`list`/`query`/`discover`): on the
    /// `std` build this takes a shared read lock, so directory reads do not
    /// serialize against each other and only block against directory writers.
    pub(crate) fn with_directory_read<R>(&self, f: impl FnOnce(&D) -> R) -> R {
        self.directory.with_read_recover(f)
    }

    /// Acquires the async-driving lock and runs `f` against the driving state.
    ///
    /// Only available with the `async` feature. Used by `poll_serve`'s
    /// take-out / `.await` / put-back discipline and by
    /// `register_async_server_binding` to bump the generation counter.
    #[cfg(feature = "async")]
    pub(crate) fn with_driving<R>(&self, f: impl FnOnce(&mut DrivingState) -> R) -> R {
        self.driving.with_recover(f)
    }

    /// Returns a [`ShutdownHandle`] that can signal the serving loops to stop.
    ///
    /// The driving loops (`serve_sync`, `serve`) check this flag between
    /// iterations and exit when set.
    pub fn shutdown_handle(&self) -> ShutdownHandle {
        ShutdownHandle {
            flag: Arc::clone(&self.shutdown),
            #[cfg(feature = "async")]
            wake: Arc::clone(&self.wake),
        }
    }

    /// Returns the cached [`InteractionRuntime`] snapshot.
    ///
    /// The runtime is constructed once at build time from the shared registry
    /// `Arc` handles and stored in [`ServientShared`]; every consumed
    /// interaction borrows this single instance instead of rebuilding a
    /// throwaway struct (which previously cost 4 `Arc` clones per request).
    pub(crate) fn interaction_runtime(&self) -> &crate::interaction::InteractionRuntime {
        &self.shared.interaction
    }

    // -----------------------------------------------------------------------
    // Directory operations.
    // -----------------------------------------------------------------------

    /// Lists directory entries in deterministic backend order.
    pub fn list(&self) -> DirectoryPage
    where
        D: ThingDirectory,
    {
        self.with_directory_read(|directory| directory.list())
    }

    /// Queries directory entries with the shared Discovery query model.
    pub fn query(&self, query: DirectoryQuery) -> DirectoryPage
    where
        D: ThingDirectory,
    {
        self.with_directory_read(|directory| directory.query(query))
    }

    /// Starts a discovery process.
    ///
    /// Returns a [`ThingDiscovery`] process object that the caller drains via
    /// [`ThingDiscovery::next_now`] or [`ThingDiscovery::next`].
    pub fn discover(&self, filter: ThingFilter) -> ServientResult<ThingDiscovery>
    where
        D: ThingDirectory,
    {
        self.with_directory_read(|directory| {
            run_discovery(directory, filter).map_err(ServientError::from)
        })
    }

    /// Registers a TD in the directory without exposing local handlers.
    pub fn register(&self, thing: Thing) -> ServientResult<DirectoryEntry>
    where
        D: ThingDirectory,
    {
        self.with_directory(|directory| directory.register(thing).map_err(Into::into))
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
            self.with_directory(|directory| directory.update(thing).map_err(ServientError::from))?;
        // Invalidate outside the directory lock — consumed_registry has its
        // own interior mutability, so we don't need to keep the directory lock
        // held during invalidation.
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
            self.with_directory(|directory| directory.delete(id).map_err(ServientError::from))?;
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

        // Snapshot bindings from the single authoritative source so
        // start_serving and the driving loop never observe divergent sets.
        // A *read* lock is sufficient — cloning the Vec<Arc<...>> never
        // mutates the source, and concurrent exposes/destroys of other
        // Things should not serialize against each other here.
        let sync_bindings = self
            .shared
            .sync_server_bindings
            .with_read_recover(|s| s.clone());
        #[cfg(feature = "async")]
        let async_bindings = self
            .shared
            .async_server_bindings
            .with_read_recover(|s| s.clone());

        // Phase 1: register inbound routes. Each binding carries its own
        // interior mutability, so no outer Servient lock is needed. On
        // failure, roll back the routes that were already registered so the
        // Thing is not half-exposed.
        //
        // We deliberately do NOT hold the directory lock across these calls:
        // `register_thing` may perform network I/O (e.g. zenoh declare), and
        // blocking directory reads/writes for the duration would reintroduce
        // the very serialization the lock-split refactor removed.
        let mut registered_sync: Vec<&Arc<dyn ServerBinding>> = Vec::new();
        for binding in sync_bindings.iter() {
            if let Err(message) = binding.register_thing(id, &td) {
                for registered in registered_sync.iter() {
                    registered.unregister_thing(id);
                }
                return Err(ServientError::RouteRegistration(message));
            }
            registered_sync.push(binding);
        }
        #[cfg(feature = "async")]
        {
            let mut registered_async: Vec<&Arc<dyn AsyncServerBinding>> = Vec::new();
            for binding in async_bindings.iter() {
                if let Err(message) = binding.register_thing(id, &td) {
                    for registered in registered_async.iter() {
                        registered.unregister_thing(id);
                    }
                    for registered in registered_sync.iter() {
                        registered.unregister_thing(id);
                    }
                    return Err(ServientError::RouteRegistration(message));
                }
                registered_async.push(binding);
            }
        }

        // Phase 2: publish to the directory. The directory lock is acquired
        // only here, briefly, and only for the directory write itself. A
        // brief visibility window exists between "routes registered" and "TD
        // discoverable": during it, the local driving loop can already
        // dispatch requests (it consults the exposed-Thing registry, not the
        // directory), but remote discovery cannot yet find the Thing. This
        // matches the W3C WoT Discovery eventual-consistency model.
        //
        // Directory publication failure is best-effort: the Thing remains
        // servable locally even if the directory write fails.
        if let Err(directory_err) =
            self.with_directory(|directory| directory.register(td).map_err(ServientError::from))
        {
            // Directory publication failure is best-effort: the Thing remains
            // servable locally even if the directory write fails.
            log::warn!(
                "clinkz-wot expose: non-fatal directory publish failure: {}",
                directory_err
            );
        }

        Ok(())
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
        // Read lock — snapshotting the Vec<Arc<...>> is a read-only operation.
        let sync_bindings = self
            .shared
            .sync_server_bindings
            .with_read_recover(|s| s.clone());
        #[cfg(feature = "async")]
        let async_bindings = self
            .shared
            .async_server_bindings
            .with_read_recover(|s| s.clone());
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
            .with_directory(|directory| directory.delete(id))
            .map_err(ServientError::from)
        {
            log::warn!(
                "clinkz-wot destroy: non-fatal directory unpublish failure: {}",
                directory_err
            );
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

        let sync_bindings = self
            .shared
            .sync_server_bindings
            .with_read_recover(|s| s.clone());
        #[cfg(feature = "async")]
        let async_bindings = self
            .shared
            .async_server_bindings
            .with_read_recover(|s| s.clone());
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
            .with_directory(|directory| directory.update(td))
            .map_err(ServientError::from)
        {
            log::warn!(
                "clinkz-wot: non-fatal directory update after affordance add: {}",
                err
            );
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
            .with_read_recover(|s| s.clone());
        #[cfg(feature = "async")]
        let async_bindings = self
            .shared
            .async_server_bindings
            .with_read_recover(|s| s.clone());
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
                .with_directory(|directory| directory.update(td))
                .map_err(ServientError::from)
        {
            log::warn!(
                "clinkz-wot: non-fatal directory update after affordance remove: {}",
                err
            );
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
        self.with_driving(|driving| {
            driving.async_binding_generation = driving.async_binding_generation.wrapping_add(1);
        });
        // Wake the async serve loop so it rebuilds its accept state and starts
        // accepting on the new binding immediately (replaces the previous 500 ms
        // poll, which could drop up to 500 ms of requests to a freshly
        // registered binding).
        self.wake.notify_one();
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
    //
    // See `servient/driving_sync.rs` (sync) and `servient/driving_async.rs`
    // (async).
    // -----------------------------------------------------------------------
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
    #[cfg(feature = "async")]
    wake: Arc<tokio::sync::Notify>,
}

impl ShutdownHandle {
    /// Signals the serving loops to stop after the current iteration.
    pub fn shutdown(&self) {
        self.flag.store(true, Ordering::Relaxed);
        // Wake a parked async `serve` loop immediately so shutdown is observed
        // without waiting for the next accept / in-flight completion.
        #[cfg(feature = "async")]
        self.wake.notify_one();
    }

    /// Returns `true` if shutdown has been signaled.
    pub fn is_shutdown(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }
}
