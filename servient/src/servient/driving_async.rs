use alloc::{boxed::Box, format, sync::Arc, vec::Vec};

use clinkz_wot_core::{
    AsyncServerBinding, CoreError, CoreResult, EventBroker, InboundRequest, InboundResponse,
    InteractionOutput, Payload,
};
use clinkz_wot_discovery::ThingDirectory;
use clinkz_wot_td::data_type::Operation;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::Ordering;
use futures_util::stream::{FuturesUnordered, StreamExt};

use crate::{ExposedThingRegistry, ServientResult};

use super::Servient;
use super::dispatch::{
    BufferingEventSink, PreparedDispatch, drain_emitted, drain_tagged_emissions,
};
use super::security::verify_inbound;

type AcceptFuture =
    Pin<Box<dyn Future<Output = (Arc<dyn AsyncServerBinding>, InboundRequest)> + Send>>;

pub(crate) struct AsyncAcceptState {
    generation: u64,
    pending: FuturesUnordered<AcceptFuture>,
}

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

impl Default for AsyncAcceptState {
    fn default() -> Self {
        Self::new()
    }
}

fn accept_future_for_binding(binding: Arc<dyn AsyncServerBinding>) -> AcceptFuture {
    Box::pin(async move {
        let request = binding.poll_accept().await;
        (binding, request)
    })
}

/// Async-driving state protected by its own lock.
///
/// This used to share a lock with the directory and the sync driving cursor.
/// After the lock-split refactor:
/// - the directory moved to `Servient::<D>::directory` with its own
///   `MapLock<D>`;
/// - the sync driving cursor moved to `ServientShared::sync_binding_cursor`
///   as a lock-free `AtomicUsize`;
/// - only the async accept-state and its generation counter remain here,
///   because the take-out / `.await` / put-back discipline in `poll_serve`
///   needs a brief mutual exclusion with `register_async_server_binding`.
///
/// The struct only exists when the `async` feature is enabled.
pub(crate) struct DrivingState {
    pub(crate) async_binding_generation: u64,
    pub(crate) async_accept_state: AsyncAcceptState,
}

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
        let mut accept_state = self.with_driving(|driving| {
            if driving.async_accept_state.generation != driving.async_binding_generation {
                let bindings = self
                    .shared
                    .async_server_bindings
                    .with_read_recover(|snapshot| snapshot.clone());
                driving
                    .async_accept_state
                    .rebuild(driving.async_binding_generation, &bindings);
            }
            core::mem::take(&mut driving.async_accept_state)
        });

        if accept_state.pending.is_empty() {
            return Ok(());
        }

        // `next().await` on a non-empty `FuturesUnordered` either blocks
        // (Pending) or yields an item (Ready(Some)). It only returns `None`
        // when the collection becomes empty — which the `is_empty()` guard
        // above prevents. Treat the (defensive) `None` as "nothing to drive
        // right now" instead of panicking, so a future refactor that breaks
        // the invariant degrades gracefully rather than taking down the
        // process.
        let Some((binding, request)) = accept_state.pending.next().await else {
            log::warn!(
                "clinkz-wot poll_serve: accept pending drained despite non-empty \
                 guard; yielding this turn"
            );
            return Ok(());
        };
        accept_state
            .pending
            .push(accept_future_for_binding(binding.clone()));

        self.with_driving(|driving| {
            driving.async_accept_state = accept_state;
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
        // uses the one in DrivingState for its take-out / put-back model).
        let generation = self.with_driving(|s| s.async_binding_generation);
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
                // Wake on async-binding registration or shutdown so the loop
                // rebuilds its accept state promptly (or exits). Replaces the
                // previous 500 ms timer, which woke twice per second even when
                // idle and delayed new bindings by up to 500 ms.
                _ = self.wake.notified() => {
                    if self.shutdown.load(Ordering::Relaxed) {
                        break;
                    }
                    let current_gen = self.with_driving(|s| s.async_binding_generation);
                    if current_gen != accept_state.generation {
                        let bindings = self
                            .shared
                            .async_server_bindings
                            .with_read_recover(|s| s.clone());
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
                    CoreError::InboundDispatch(format!("Unknown Thing id '{}'", request.thing_id)),
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
            None => Err(CoreError::MissingHandler {
                target: request.target.clone(),
                operation: request.operation,
            }),
        };

        match output {
            Ok(out) => InboundResponse::new(out, correlation),
            Err(core_err) => InboundResponse::error(correlation, core_err),
        }
    }
}

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
    let prepared = slot.with_thing(|thing| {
        PreparedDispatch::prepare(thing, &request.target, request.operation, build_input())
    });
    let result = match prepared {
        None => Err(CoreError::MissingHandler {
            target: request.target.clone(),
            operation: request.operation,
        }),
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
