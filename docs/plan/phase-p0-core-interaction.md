# Phase P0 — Core Interaction Surface Rewrite

> **⚠ Status — implemented; expanded post-v4.0.** The core interaction
> surface landed per this plan and was later expanded (P0–P3 user-facing
> API work, commits `39bcb4d` → `b6f5a97`) with streaming ops, bulk ops,
> async handler setters, and the `ProtocolBinding` facade. See
> `docs/user-facing-api.md` for the current external boundary.

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

### Step 0.0 — `clinkz-wot-td` internal cleanups (Tier 0, audit D17)

P0 owns the td data-contract cleanups (baseline §3), since td is the foundation
layer and P1 depends on the `AbsoluteUri` re-export:

- Split `td/src/core/data_type.rs` into cohesive modules
  (`core/uri.rs`, `core/metadata.rs`, `core/version.rs`, `core/response.rs`,
  `core/operation.rs`).
- Extract a shared `FormData` core (deduplicate `ThingModelForm`/`Form`).
- Extract shared Thing/ThingModel affordance validation helpers.
- **Re-export `AbsoluteUri` at the td crate root**
  (`pub use core::data_type::AbsoluteUri;`) — AD11, P1 hard prerequisite.
- Convert free-form `String` error messages to structured enum variants.

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
- Remove `multithread` from `core/Cargo.toml` features. **Also remove the
  `multithread` feature from `servient/Cargo.toml`** (audit F6 — it forwards to
  `clinkz-wot-core/multithread`; deleting the core feature invalidates the
  servient forward). This is a P0 task even though P3 rewrites servient, so the
  feature does not dangle.
- **Registry read path per build (AD2, corrected C1/AD54):**
  - **std**: lock-free `arc_swap::ArcSwap<Arc<im::HashMap<K,V>>>` snapshot — read
    = `load()` (lock-free Guard); write = `store(Arc::new(map.insert(..)))`
    (O(log n) structural-sharing). Add `arc-swap` + `im` to `core/Cargo.toml`
    behind the `std` feature.
  - **no_std**: `WotLock<BTreeMap<K,V>>` + clone-out dispatch discipline — read =
    `wotlock.with_read(|m| m.get(&id).cloned())` (brief CS ~500ns: BTreeMap::get
    + Arc clone); handler invocation **outside** any lock. **Zero external deps.**
    `arc-swap` and `im` are NOT stable-`no_std` and are excluded entirely.
  - This is for the registries and handler tables. `WotLock` is also used for
    read-write-frequent / exclusive-semantics state on every build
    (see P3 §3.1/§3.6).

### Step 0.2 — Identity and correlation types

Retain `core/src/identity.rs` (`ThingId`, `CorrelationId`) unchanged (v3.1 §1.1).
Audit that no `MapLock` references remain in core.

### Step 0.3 — Concrete Thing types

- Remove the `ExposedThing` and `ConsumedThing` traits (`core/src/thing.rs`).
- Introduce concrete `ExposedThing` (produced Thing + handler sets) and
  `ConsumedThing` (consumed Thing + resolved binding plan).
- These live in core; `Servient` wraps them in `Arc` handles (P3).
- **`LocalThing` affordance-mutation primitives (audit F9 — decision):** the
  existing `core/src/thing.rs` `LocalThing::{add,remove}_{property,action,event}`
  (not the deleted `ExposedThingHandle` ones) are **retained as produce-time TD
  builders**. D2 freezes the affordance set only *after* `expose()`; pre-expose
  TD assembly (`produce` → mutate → `expose`) legitimately needs them. They are
  NOT reachable from an exposed handle post-expose (AD12 removed that surface),
  so retaining the core primitives does not reopen the dynamic lifecycle.

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
- `ExposedThing` holds `Map<AffordanceName, Arc<HandlerSet>>` per kind
  (audit H1 — **single model**: std = `im::OrdMap`+`ArcSwap`; no_std =
  `BTreeMap`+`WotLock` clone-out). Each affordance's `HandlerSet` is a plain
  `Arc<HandlerSet>` **value in the map**, NOT a separate per-affordance
  `ArcSwap` cell. Dispatch clones the `Arc<HandlerSet>` and invokes outside any
  lock (clone-out / snapshot load).
- Rationale: inbound handler invocation is the device hot path; sync dispatch
  is a direct virtual call with **no `Box`**. Async handlers pay one
  `async_trait` `Box` per call — acceptable only because the handler is
  I/O-bound (v4.0 §4.2).
- **Handler-swap granularity (audit round-2 P-2/AD51, H1 unified):** v1 swaps a
  slot by rebuilding the ONE affected `Arc<HandlerSet>` (one alloc) + one map
  insert (O(log n)); other affordances are untouched. If runtime handler-swapping
  later proves hot, the documented escape hatch is per-slot
  `ArcSwapOption<Arc<dyn …>>` (swap one slot without rebuilding the struct).
  Deferred.

### Step 0.5 — Interaction I/O (Scripting API §7.1)

- Rework `InteractionInput` handler-facing fields to **`data`** (renamed from
  `payload`) + **`uri_variables`** (renamed from `parameters`) + `principal`
  (audit D3 — naming consistency across `InteractionInput`/`Options`/`Output`).
  Remove `security_metadata` (moves to binding/transport layer). **Add
  `accept: Option<AcceptHint>`** (audit round-2 O7/AD48) — a protocol-neutral
  view of the request's `Accept`/content-type preferences, populated by the
  binding at the edge, so a byte-level handler can choose a client-acceptable
  output content type and avoid a mismatch-driven double codec. `AcceptHint` is
  a small `no_std + alloc`-safe struct (preferred `MediaType` + optional ordered
  list), carrying no protocol headers.
- Introduce `InteractionOptions { uri_variables, form_index, data, timeout }`
  for the consumed-side call surface.
- Rework `InteractionOutput { data, status }`; enumerate
  `InteractionStatus { Ok, Created, Accepted }` (`#[non_exhaustive]`,
  v4.0 §4.3).

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
- `ServerBinding` exposes a **synchronous, non-blocking `try_accept`** (default
  `None` — audit F8: std-only bindings self-push and never have it called) — no
  boxed `poll_accept` future, no `select_all` (audit defect AD1):
  `fn try_accept(&self) -> Option<InboundRequest> { None }`, the **reply path
  `send_response(InboundResponse)`** (audit F1 — required by AD9 overload error
  replies; `InboundRequest` carries no reply handle), **`set_event_broker`**
  (audit F1 — EventBroker injection, default no-op), wholesale
  `register_thing(thing_id, td) -> Result<(), CoreError>` /
  `unregister_thing(thing_id)` (audit round-2 C3/AD38 — `register_thing` must be
  fallible so `expose()` rollback E12/AD27 can detect a binding `k+1` failure;
  `unregister_thing` stays infallible since `destroy()` is idempotent/best-
  effort), plus a **formalized std fan-in injection point**
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
- **`verify` is on the sync inbound hot path (audit round-2 O2/AD43):** it runs
  before the handler on every dispatch, so the same non-blocking rule that
  governs sync handlers governs `verify` — it must be non-blocking/short.
  Expensive crypto (JWT/signature validation) belongs in an async twin; an
  `AsyncSecurityProvider` (`verify`/`apply` async twins) is a deferred
  follow-up (`docs/deferred-design-followups.md`), not a v1 surface.

### Step 0.10 — Payload / codec / transport

- Retain `Payload` (`Arc<[u8]>` body), `PayloadCodec`, `TransportAdapter`,
  `TransportRequest`, `TransportResponse`. Adapt to the new `InteractionOutput`.
- **Outbound timeout — build-time cfg, NOT a trait (audit H2):** the earlier
  `OutboundTimeout` trait with a generic `fn timeout<F: Future>(...)` was **not
  object-safe** (generic methods cannot produce `dyn`), so `Arc<dyn
  OutboundTimeout>` was invalid. Correction: timeout is a **build-time cfg**
  inside the Servient outbound path (P3), not a runtime-injected trait:
  - **std** (tokio): `tokio::time::timeout(dur, binding.invoke(req))` when
    `options.timeout.is_some()`.
  - **no_std + async** (embassy): `embassy_time::with_timeout` behind the
    `embassy` feature.
  - **bare no_std**: a set `options.timeout` returns
    `Err(CoreError::TimeoutUnsupported)` (AD45 — fail-closed, never silent).
  - No trait, no `dyn`, no per-call boxing. `CoreError::Timeout` and
    `TimeoutUnsupported` variants are retained (§0.11).

### Step 0.11 — Core error taxonomy

- `CoreError`: retain `MissingHandler { target, operation }`, `Security`,
  `InboundDispatch`. Drop variants tied to removed surfaces. **Add the variants
  the round-2 resolutions require:** `HandlerPanic { target, operation }`
  (AD30 — std-only panic→reply), `Timeout` (AD39 — outbound timeout expired),
  `TimeoutUnsupported` (AD45 — a `timeout` was requested but this build has no
  timer cfg; fail-closed), and
  `UnsupportedForm` (AD47 — a caller-pinned `form_index` points at a form no
  binding can drive). Each is a structured variant (no free-form `String`) so
  it threads through `error_status`/`ServientError`.

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
  hot path (v4.0 §4.2). The same non-blocking budget applies to
  `SecurityProvider::verify`, which is on the same inbound hot path before the
  handler (audit round-2 O2/AD43).
- `critical_section` dependency adds a critical-section impl registration
  requirement on bare targets — documented in `docs/no-std-embedded.md`.
