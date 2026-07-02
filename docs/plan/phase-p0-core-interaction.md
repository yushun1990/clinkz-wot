# Phase P0 — Core Interaction Surface Rewrite

> Baseline: `docs/baseline/engine-architecture-baseline.md` (v4.0) §4.

## Goal

Rewrite `clinkz-wot-core` to a single async interaction surface aligned with the
WoT Scripting API. This phase is foundational: every other crate depends on
core's public types.

P0 leaves **`clinkz-wot-core` compiling and tested in isolation**. Core's direct
dependents (`protocol-bindings-zenoh`, `discovery`, `servient`) are temporarily
broken until P1–P3 adapt them. This is the accepted cost of the one-shot
breaking refactor.

## Entry Criteria

- v4.0 baseline is locked. ✅

## Work Breakdown

### Step 0.1 — Lock primitive: `MapLock` → `WotLock`

Rewrite `core/src/sync.rs`:

- `WotLock<T>`: `Arc`-backed, `Clone` (cheap refcount bump), interior-mutable.
- Backends: `std::sync::RwLock<T>` (std) / `critical_section::Mutex<T>` (no_std).
  (No blocking RwLock exists for no_std — `critical_section` is the primitive;
  `embassy_sync`'s RwLock is async-only and thus the wrong tool for these brief
  synchronous holds.)
- API: `WotLock::new(value)`, `with(|&mut T| R) -> R` (exclusive),
  `with_read(|&T| R) -> R` (shared on std / exclusive on no_std), `with_recover`
  (panic-healing) retained.
- Remove the `multithread` feature, `RefCell` backend, `UnsafeCell` backend,
  and the `MapLockError` std-poison surface (or retain minimal).
- Remove `multithread` from `core/Cargo.toml` features.
- **Snapshot-read primitive for hot read-heavy state** (audit defect AD2):
  alongside `WotLock`, expose a copy-on-write snapshot helper
  (`Arc<ImmutableMap>` publish + atomic-load read) for the registries and
  handler tables whose reads must not disable interrupts on `no_std`. `WotLock`
  is reserved for read-write-frequent / exclusive-semantics state (see P3
  §3.1/§3.6).

### Step 0.2 — Identity and correlation types

Retain `core/src/identity.rs` (`ThingId`, `CorrelationId`) unchanged (v3.1 §1.1).
Audit that no `MapLock` references remain in core.

### Step 0.3 — Concrete Thing types

- Remove the `ExposedThing` and `ConsumedThing` traits (`core/src/thing.rs`).
- Introduce concrete `LocalExposedThing` (produced Thing + handler sets) and
  `BoundConsumedThing` (consumed Thing + resolved binding plan).
- These live in core; `Servient` wraps them in `Arc` handles (P3).

### Step 0.4 — Handler trait set (sync primary, opt-in async)

- Define **synchronous** handler traits as the primary path (plain `fn`,
  `Send + Sync`, zero per-call allocation): `PropertyReadHandler`,
  `PropertyWriteHandler`, `PropertyObserveHandler`, `PropertyUnsubscribeHandler`,
  `ActionHandler`, `ActionQueryHandler`, `ActionCancelHandler`,
  `EventSubscribeHandler`, `EventUnsubscribeHandler`.
- Define **opt-in async twins for ALL 9 operations** behind the `async`
  feature (`#[async_trait]`, `+ Send + Sync`): `AsyncPropertyReadHandler`,
  `AsyncPropertyWriteHandler`, `AsyncPropertyObserveHandler`,
  `AsyncPropertyUnsubscribeHandler`, `AsyncActionHandler`,
  `AsyncActionQueryHandler`, `AsyncActionCancelHandler`,
  `AsyncEventSubscribeHandler`, `AsyncEventUnsubscribeHandler` — observe/
  unobserve, query/cancel, event subscribe/unsubscribe included, not just
  read/write/invoke (audit defect AD4: partial coverage would force cloud/
  gateway handlers on the uncovered ops to block the executor).
- Remove the old nine sync traits' mechanical duplication (consolidated storage
  replaces it); the sync traits above ARE the primary surface.
- Define consolidated handler-set storage: `PropertyHandlerSet`,
  `ActionHandlerSet`, `EventHandlerSet`, each slot an enum
  `Sync(Arc<dyn …>) | Async(Arc<dyn Async…>)` (async arm feature-gated).
- `LocalExposedThing` holds `BTreeMap<AffordanceName, …HandlerSet>` per kind.
- Rationale: inbound handler invocation is the device hot path; sync dispatch
  is a direct virtual call with **no `Box`**. Async handlers pay one
  `async_trait` `Box` per call — acceptable only because the handler is
  I/O-bound (v4.0 §4.2).

### Step 0.5 — Interaction I/O (Scripting API §7.1)

- Rework `InteractionInput` → keep handler-facing fields: `payload`, `parameters`
  (uri variables), `principal`. Remove `security_metadata` (moves to binding/
  transport layer).
- Introduce `InteractionOptions { uri_variables, form_index, data, timeout }`
  for the consumed-side call surface.
- Rework `InteractionOutput { data, status }`.

### Step 0.6 — Affordance addressing, inbound, binding requests

- Retain `AffordanceTarget` (`Arc<str>`, owned, `'static`), `AffordanceKind`.
- Retain owned `InboundRequest`, `InboundResponse`, `BindingRequest` (v3.1 §2).
- `ClientBinding::invoke` and `ClientBinding::subscribe` are declared
  `async fn` **in P0** (resolved decision A1). Core defines the async traits;
  bindings adapt their implementations in P2. P0's core tests use a fake
  binding that implements the async traits via `#[async_trait]`.

### Step 0.7 — Binding traits (core definition)

- `ClientBinding { supports, async invoke, async subscribe }` (resolved A1) —
  outbound path; the `async_trait` `Box` per outbound call is acceptable
  (network-amortized), unlike the inbound handler path which stays sync.
- `ServerBinding` exposes a **synchronous, non-blocking `try_accept`** — no
  boxed `poll_accept` future, no `select_all` (audit defect AD1):
  `fn try_accept(&self) -> Option<InboundRequest>`, plus wholesale
  `register_thing(thing_id, td)` / `unregister_thing(thing_id)`, plus a
  **formalized std fan-in injection point**
  `#[cfg(feature="std")] fn set_request_sink(&self, sender: FanInSender<InboundRequest>)`
  (audit defect AD13 — the std main path is binding→channel `try_send`, so the
  sender injection must be on the trait surface, not prose). The Servient calls
  `set_request_sink` at registration (std) to hand each binding a sender clone;
  on no_std there is no channel and the loop polls `try_accept` per binding —
  see P3 §3.5. Remove `poll_accept_sync`, `AsyncServerBinding`, and the
  boxed-`poll_accept` surface entirely.
- Remove `register_affordance` / `unregister_affordance` (decision 2); the
  wholesale `register_thing` / `unregister_thing` declares/undeclares all
  routes for the Thing at once.
- `InboundDispatcher` maps `InboundRequest` → handler dispatch →
  `InboundResponse`; calls the sync handler directly (no allocation) or awaits
  an opt-in async handler (§0.4).

### Step 0.8 — Event / subscription primitives

- Retain `EventBroker`, `PublisherSink`, `Subscription`, `SubscriptionGuard`,
  `EventSink` (`core/src/event.rs`). Adapt to async handler dispatch.

### Step 0.9 — Security primitives

- Retain `SecurityProvider` (`verify` inbound, `apply` outbound), `Principal`,
  `PrincipalId`, `AuthMaterial`, `SecurityError`, `CredentialStore`,
  `InMemoryCredentialStore`, `check_scopes` (`core/src/security.rs`).
- Change `apply` to return the metadata it added (deferred #4), removing the
  post-apply diff.

### Step 0.10 — Payload / codec / transport

- Retain `Payload` (`Arc<[u8]>` body), `PayloadCodec`, `TransportAdapter`,
  `TransportRequest`, `TransportResponse`. Adapt to the new `InteractionOutput`.

### Step 0.11 — Core error taxonomy

- `CoreError`: retain `MissingHandler { target, operation }`, `Security`,
  `InboundDispatch`. Drop variants tied to removed surfaces.

### Step 0.12 — `core/src/lib.rs` public surface

- Re-export the new types. Remove re-exports of removed traits/handlers.
- `#![no_std]` retained; `extern crate alloc` retained.

## Resolved Decisions

- **A1 (outbound async; handlers stay sync).** `ClientBinding::invoke` and
  `subscribe` are declared `async fn` in P0 itself (outbound path; one
  `async_trait` `Box` per call, accepted because each call is a network
  round-trip). Rationale: P0 rewrites core's public surface and breaks all
  dependents anyway, so the outbound async move is done once; P2 only migrates
  binding *implementations*. **Inbound handler traits stay synchronous** (zero
  per-call allocation) with opt-in async twins behind `async` — the inbound hot
  path must not `Box` per interaction on MCU (v4.0 §4.2). P0's fake binding is
  async from the start; P0's fake handler is sync.

## Deliverables

- `clinkz-wot-core` rewritten per above.
- `cargo check -p clinkz-wot-core` and `--no-default-features` pass.
- `cargo test -p clinkz-wot-core` covers: handler registration, synthetic
  dispatch round-trip (sync handler read/write/invoke/subscribe), opt-in async
  handler dispatch, `WotLock` concurrency, inbound security verify, error
  mapping.

## Exit Criteria

- `clinkz-wot-core` compiles `no_std + alloc` and `std`.
- The public surface matches v4.0 §4.
- Core tests pass.
- Direct dependents are known-broken (tracked in P1/P2/P3).

## Risks

- The sync-primary handler model means a handler that performs long blocking
  work inside the async driving loop will block the executor. Document that
  sync handlers must be non-blocking (or short); I/O-bound handlers must use
  the opt-in async variant. This is the trade-off for the zero-alloc inbound
  hot path (v4.0 §4.2).
- `critical_section` dependency adds a critical-section impl registration
  requirement on bare targets — documented in `docs/no-std-embedded.md`.
