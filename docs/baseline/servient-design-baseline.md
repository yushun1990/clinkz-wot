# Servient Runtime Design Baseline (v3.0)

This document is the consolidated, authoritative design baseline for the
`clinkz-wot-servient` runtime redesign. It is implementation-ready: every
design decision in it is **LOCKED**. It supersedes the earlier v2.2 draft, in
which the concurrency model (OPEN-A), inbound security (OPEN-C), subscription
data flow (OPEN-D), `expose()` coordination (OPEN-E), and inbound request shape
(OPEN-F) were still open. v3.0 resolves all five.

The baseline supersedes the earlier Servient design assumptions embedded in
`PLAN.md` M6 and `docs/technical-spec.md`. The implementation plan under
`docs/plan/servient-runtime-redesign-plan.md` will be written against this
baseline.

Implementation-time type refinements (concrete types for the placeholders in
§8 and §11, the owned request/response model, the directory-invalidation
trigger, `expose()` handler sequencing, and the error taxonomy) are recorded
and locked in `docs/baseline/servient-design-baseline-addendum.md` (v3.1).
This document remains the authoritative decision reference; the addendum makes
its illustrative statements normative without reversing any LOCKED decision.

## 0. General Principles

- Target specifications: W3C WoT Architecture 1.1, WoT Discovery, and WoT
  Scripting API. The Scripting API is a W3C Group Note and is used as a
  semantic reference, not a normative contract.
- The engine stays protocol-neutral. Zenoh is the first optional protocol
  binding and is never a required dependency of the engine.
- The `clinkz-wot-td`, `clinkz-wot-core`, `clinkz-wot-protocol-bindings`
  (shared), and `clinkz-wot-servient` crate roots remain `no_std + alloc`.
  Concrete runtimes (Rust `zenoh`, `tokio`, `embassy`) stay behind feature flags
  or separate runtime crates.
- The redesign is a **one-shot breaking refactor**. M6 is still early and there
  are no downstream consumers, so the public API is changed directly to the
  target shape rather than migrated in compatibility-preserving stages.

## 1. Roles and Bindings [LOCKED]

A Servient may take on any combination of the Server, Client, Discovery, and
Intermediary roles. The W3C WoT Architecture explicitly permits these roles to
be active simultaneously, and the Consumer, Producer, and Discovery conformance
classes defined by the Scripting API are separable but may also be combined in
a single Servient instance. The Intermediary (gateway) pattern is therefore a
first-class use case for Clinkz.

The core crate defines two orthogonal traits:

- `ClientBinding` for outbound interactions (consuming remote Things).
- `ServerBinding` for inbound interactions (serving exposed Things).

A single concrete protocol binding (for example `ZenohBinding`) may implement
one or both traits and shares one protocol session across both directions. A
pure MCU sensor Servient exposes only the server side, a pure cloud application
Servient exposes only the client side, and a gateway Servient exposes both
against one shared session.

## 2. Core Trait Surface (no_std + alloc) [LOCKED]

### Outbound (retained, semantically renamed)

- `ClientBinding::invoke(&self, BindingRequest) -> CoreResult<InteractionOutput>`
- `BindingRequest { thing, target, operation, form, input }`

The `invoke` receiver is `&self`. Each concrete binding owns its own interior
mutability for I/O state, so outbound calls are issued through a shared
reference (see Section 7).

### Inbound (new)

- Protocol-neutral types `InboundRequest` and `InboundResponse` as defined in
  Section 11.
- An `InboundDispatcher` that maps an `InboundRequest` plus the exposed Thing
  registry to an `InboundResponse`. The dispatcher is synchronous and is the
  symmetric counterpart of the outbound path. It resolves the matched `Form`
  internally (for security scheme lookup) and never exposes it to handlers.
- A `ServerBinding` providing a dual accept surface (see Section 4):
  `poll_accept_sync(&self) -> Option<InboundRequest>` and
  `poll_accept(&self) -> impl Future<Output = InboundRequest>`.

### Interaction handle traits (already present, symmetric)

`ExposedThing` and `ConsumedThing` both expose
`read_property` / `write_property` / `invoke_action` / `subscribe_event`.

### Events (new)

- An `EventBroker` in core (no_std) fans event emissions from local handlers
  out to the server bindings that serve remote subscribers. Its data flow is
  specified in Section 9.
- A `Subscription` handle supports `poll_next()` (sync) or a `Stream` (async)
  plus an explicit `stop()`.

## 3. Lifecycle [LOCKED]

There is no global `Servient::start()` or `Servient::stop()`. The WoT Scripting
API has no such concept; lifecycle is managed per Thing through
`expose(td)` and `destroy(id)`.

Calling `expose(td)` immediately registers the inbound serving work for that
Thing. Calling `destroy(id)` unregisters it. The coordination semantics between
the exposed Thing registry, the discovery directory, and the server binding —
including failure rollback — are specified in Section 10.

## 4. Driving Layer (WoT API is strictly separated from platform driving) [LOCKED]

Engine startup and execution management is **not** a WoT concern. It is purely
a Rust platform-integration concern. The unifying contract is Rust's native
`Future` trait, which `tokio`, `async-std`, and `embassy` all implement;
`embassy` proves that `Future` works on `no_std` MCU targets.

No custom `ServeRunner` trait is introduced. The engine exposes its serving
work as a runtime-agnostic `Future` and a synchronous primitive. Callers drive
it with whatever runtime they already use.

Both flavors are retained. They are paired consistently and never crossed:

```rust
// Sync flavor (default; bare no_std super-loop)
poll_serve_sync(&self) -> ServientResult<()>      // one step: at most one inbound request
serve_sync(&self) -> !                            // std host wrapper around repeated poll_serve_sync()
// ServerBinding::poll_accept_sync(&self) -> Option<InboundRequest>  (non-blocking, immediate)

// Async flavor (async feature; embassy no_std / tokio std / async-std)
poll_serve(&self) -> impl Future<Output = ServientResult<()>> + Send   // native async, Waker suspension
serve(self)      -> impl Future<Output = ()> + Send                     // loop { poll_serve().await }
// ServerBinding::poll_accept(&self) -> impl Future<Output = InboundRequest>  (Pending until woken)
```

Note: `serve` takes `self` by value (consuming an owned, cheaply-cloned
`Servient`) so the returned future is `'static + Send` and spawnable, e.g.
`tokio::spawn(svc.clone().serve())`. `poll_serve`, `poll_serve_sync`, and
`serve_sync` take `&self`, consistent with the interior-mutability model in
Section 7.

Key constraints:

1. A sync build uses only the sync pair; an async build uses only the async
   pair. There is no `poll_serve_sync` plus `serve` crossing.
2. The async variant is native async (Waker-based suspension and wake-up),
   never a synchronous wrapper. The two flavors share only the
   `InboundDispatcher` and the `InboundRequest` type.
3. Naming convention: the `_sync` suffix denotes the synchronous variant; the
   bare name denotes the asynchronous variant.
4. `poll_serve_sync` is **stepwise**, not draining: each call processes at
   most one inbound request across all registered sync bindings, matching the
   stepwise intent of `poll_serve()`.
5. Sync fairness is rotation-based: the next `poll_serve_sync` call resumes
   polling from the binding after the one that just produced work, avoiding
   starvation when one binding stays continuously busy.
6. `serve_sync` is a std-host convenience loop. Implementations may apply idle
   backoff such as `yield_now()` / short sleep when no request is available, so
   the host flavor does not degenerate into a pure busy-spin while preserving
   the non-blocking `poll_accept_sync()` contract.

## 5. Storage Ownership [LOCKED]

- The `ExposedThingRegistry` (concrete type) is the single authority for
  locally exposed Things; it owns the TD and the live handler together.
- The `Directory` (the single generic parameter `D` and the only injectable
  backend) is a discovery publication target.
- The `ConsumedThingRegistry` (concrete type; renamed from the earlier
  `ConsumedThingCache`) is the outbound live-instance registry. See §5.1 for its
  precise purpose and persistence policy.
- The current manual three-way synchronization between directory, exposed
  registry, and consumed registry is removed.

### 5.1 ConsumedThingRegistry — Purpose and Persistence [LOCKED]

`ConsumedThingRegistry` is the **interning map of live `ConsumedThing`
instances**, keyed by Thing identity. Its purpose is deliberately narrow:

1. **Identity interning** — `consume()` of the same Thing returns the same
   `ConsumedThingHandle`, so multiple components of a gateway consuming one
   remote Thing share one canonical live instance.
2. **Resource reuse** — one open binding session and one set of event
   subscriptions per remote Thing, instead of duplicate connections per caller.
3. **Derived-computation memoization, internalized** — form selection, binding
   plan, and key-expression mapping are computed once and cached **inside** the
   interned instance (replacing the per-call recomputation in the current
   `BoundConsumedThing::request()`, `core/src/thing.rs:219`), not as a separate
   cache.

`ConsumedThingRegistry` is **not**:

- A value cache — it never caches interaction **results** (property values);
  staleness is an application-layer concern.
- A TD store — storing TDs fetched via Discovery is the `Directory` (`D`) job.
- A persistence layer — see below.

**Persistence policy.** The registry holds **live runtime resources** (open
binding sessions, active subscriptions, zenoh session handles) that are bound to
a live process and cannot be serialized or reattached after a restart. Therefore
the registry is **always in-memory and never persisted**; persisting it would be
a category error. What may persist is the *input* that rebuilds it:

- The set of consumed TDs, whose persistence belongs to the **TD source layer**:
  Directory-sourced Things persist via `D` when `D` is a durable backend;
  non-Directory-sourced Things (P2P introduction, manual configuration) persist
  via an optional durable consumed-TD store (injectable, behind the `std` feature
  or an MCU flash abstraction).
- The application's own consume-list / boot configuration, which re-issues
  `consume()` at startup. Per the WoT Scripting API, `consume()` is explicitly
  application-driven; the engine does not autonomously restore consumed Things.

On restart the registry is **lazily rebuilt in-memory** as the application calls
`consume()` again. Directory-driven invalidation (§5.2) then keeps it fresh.

### 5.2 Invalidation [LOCKED]

When the `Directory` reports a TD change for a Thing identity, the corresponding
interned entry in `ConsumedThingRegistry` is invalidated and rebuilt, because its
internal form selection and binding plan derive from that TD. Invalidation
granularity is **per Thing identity** (not per affordance) in v1. The registry
exposes an explicit `invalidate(id)` entry point for programmatic use.

## 6. API and Typed Handles [LOCKED]

- `Servient<D>` keeps a single generic parameter `D` for the directory. The
  previous registry, consumed registry, selected form cache, and binding plan
  cache generic parameters collapse into internal concrete types.
- `Servient<D>` is `Clone` (cheap, `Arc`-based). All public methods take
  `&self`; none take `&mut self`. In an async build it is `Send + Sync`; in a
  sync build it is `!Send` (single-threaded super-loop assumption).
- The interaction API is exposed through typed handles:
  `expose(&self, td) -> ExposedThingHandle` and
  `consume(&self, td) -> ConsumedThingHandle`. Both handle types use the same
  interaction method names, aligned with the WoT Scripting API sections 8 and 9.
  Handles hold `Arc` clones of the relevant stores plus ids; they are `Clone`
  and (async build) `Send + Sync`.
- Local in-process interactions go directly to the handler. They do not run
  form selection or apply transport security providers, because security
  schemes protect protocol hops, not in-memory calls.

## 7. Concurrency and Sharing Model [LOCKED]

This section resolves OPEN-A. It is the keystone that shapes every public API
signature.

### Decision: shared interior-mutable Servient

All mutable state moves behind interior mutability. Every `Servient` method
takes `&self` (never `&mut self`). This eliminates the borrow conflict between
a running `serve()` task and a caller invoking `consume()` or `expose()`.

An actor / message-passing model was considered and rejected: it would force
`expose()` / `consume()` to be asynchronous even in the sync flavor, violating
the LOCKED principle that the sync flavor is truly synchronous and zero-overhead.

### Two-level locking for the exposed Thing registry

```rust
ExposedThingRegistry =
    Arc<MapLock<BTreeMap<ThingId, Arc<ThingLock<ThingEntry>>>>>
```

- **Outer map lock** (`MapLock`): held only for insert, remove, and enumerate —
  i.e. `expose()` and `destroy()`. Coarse and infrequent. Holding it across the
  whole `expose` sequence yields registry + route + directory atomicity for free
  (rollback = remove on failure). See Section 10.
- **Inner per-Thing lock** (`ThingLock`): each `ThingEntry` is individually
  wrapped. Held only across a single handler call. Interactions against
  different Things never contend; within one Thing, interactions serialize,
  which matches typical device semantics.

### Dispatch discipline (reentrancy-safe)

```text
lock map -> clone Arc<ThingEntry> -> drop map lock
lock ThingEntry -> run handler (sync) -> drop ThingEntry lock
```

Locks are **never held across an `.await` point**, and **never held across a
handler that calls back into the Servient**. A handler emitting an event touches
the `EventBroker` (its own lock); a handler calling `destroy()` touches the map
lock. Neither contends with the per-Thing lock already held, so there is no
self-deadlock. This keeps the protocol-neutral core traits synchronous.

### Lock primitive (internal, not public-generic)

A small `pub(crate)` newtype selects the backing primitive per feature, so the
engine-wide API carries no extra generic:

| Build | `MapLock` / `ThingLock` primitive | Notes |
|---|---|---|
| sync (default) | `core::cell::RefCell` | Single-threaded; zero-cost; no interrupt disable during (potentially slow) handler calls. |
| async + std | `std::sync::Mutex` | Brief hold; never awaited while held. |
| async + embassy | `critical_section::Mutex` / `embassy_sync` | no_std-safe; brief hold. |

Critical sections are always short and never span `.await` or handler dispatch.

### Handler and binding bounds

- Stored handlers and bindings are `Send` in every build (zero-cost on a single
  thread; honest about capability).
- In the **async** build they are additionally `Sync`, enabling multi-thread
  `tokio` spawning. Trait objects therefore become
  `Box<dyn PropertyHandler + Send + Sync>` in async, `+ Send` in sync.

### serve() / poll_serve() ownership

`serve(self)` consumes an owned `Servient` clone so the returned future is
`'static + Send` and spawnable. `poll_serve(&self)`, `poll_serve_sync(&self)`,
and `serve_sync(&self)` borrow shared.

### Edge case: destroy() from within a handler

A handler calling `destroy(own_id)` while its own per-Thing lock is held is
resolved by **deferred removal**: `destroy()` sets a `draining` flag on the
entry and completes the actual map removal after the in-flight handler returns.
The Thing is observable as "gone after the current interaction." Non-reentrant
locks are preserved.

## 8. Security [LOCKED]

This section resolves OPEN-C.

The current `SecurityProvider` (in `core/src/security.rs`) only covers outbound
interactions via `apply(...)`. Inbound interactions receive a symmetric entry
point. The existing provider registry (matched by `scheme_name()`) is reused,
so the inbound and outbound paths share one security configuration surface.

```rust
pub struct Principal {
    // Identity established for the inbound caller.
    pub id: /* opaque identity */,
    // Scopes/claims carried for authorization, if any.
    pub scopes: Vec<String>,
}

pub trait SecurityProvider {
    // existing outbound members retained ...

    /// Verifies an inbound request before it is dispatched to a handler.
    /// `scheme` is the security scheme of the form that received the request.
    fn verify(
        &self,
        request: &InboundRequest,
        scheme: &SecurityScheme,
    ) -> Result<Principal, SecurityError>;
}
```

- `verify` runs **before** the dispatcher routes a request to a local handler.
- The matched `Form`'s security scheme is resolved internally by the dispatcher
  and passed to `verify`; handlers never see it.
- v1 scope: authenticate plus an optional scope match against affordance
  `security` / `scopes`. A full per-affordance policy engine is deferred.
- `verify` is synchronous, matching `apply`.

## 9. Subscription Data Flow [LOCKED]

This section resolves OPEN-D.

### Inbound (exposed) events

The `EventBroker` holds a fan-out table:

```text
Map<(ThingId, EventName), Vec<PublisherSink>>
```

A local `EventHandler::subscribe` is invoked **once** with a broker-backed
`EventSink`. Each subsequent `EventSink::emit(payload)` fans the payload out to
every registered `PublisherSink` for that event, each of which wraps a server
binding's publish channel and pushes the payload to remote subscribers. Remote
subscribe / unsubscribe maps to adding and removing a `PublisherSink`.

### Outbound (consumed) events

`ConsumedThingHandle::subscribe_event()` calls the client binding to subscribe
remotely and returns a `Subscription` handle. Remotely pushed samples flow:

```text
remote -> client binding -> per-subscription bounded queue -> caller
```

The caller drains the queue via `Subscription::poll_next() -> Option<Payload>`
(sync) or via a `Stream` impl (async), and stops it with `stop()`. A
`Subscription` holds an `Arc` to its queue and is `Clone` and (async build)
`Send + Sync`.

The queue primitive is feature-gated:

| Build | Queue primitive |
|---|---|
| no_std | `heapless::spsc::Queue` |
| std | `flume` or `tokio::mpsc` |

### Backpressure

When a bounded outbound queue is full the policy is **drop-oldest with an
overflow counter**: the oldest sample is evicted, a saturated-counter is
incremented, and the producer (client binding) is never blocked. This bounds
latency and avoids head-of-line blocking across subscriptions. The overflow
counter is observable on the `Subscription` for diagnostics.

## 10. expose() / destroy() Coordination [LOCKED]

This section resolves OPEN-E. Most of it falls directly out of the two-level
locking in Section 7.

### expose(td)

Performed under the **outer map lock**:

1. Validate the TD and handler set.
2. Insert the `ThingEntry` into the registry (wrapped in its own `ThingLock`).
3. Register inbound routes in each `ServerBinding` (affordance form -> protocol
   key). Route registration **reuses the existing zenoh planner** in the reverse
   (inbound) direction.
4. Publish the TD to the `Directory`.

Rollback / severity:

- A **binding route-registration failure** is **fatal**: the registry entry
  inserted in step 2 is removed and `expose()` returns `Err`. The Thing cannot
  be served, so it is not exposed.
- A **Directory publish failure** is **non-fatal**: the Thing remains locally
  exposed and servable; a warning is surfaced. WoT Discovery is optional per
  spec, so local serving is not coupled to discovery availability.

### destroy(id)

Order matters for clean teardown:

1. **Unregister inbound routes** in each `ServerBinding` (so no new requests can
   arrive mid-teardown).
2. **Remove the entry** from the registry.
3. **Unpublish** the TD from the `Directory` (best-effort; failure is logged).

A `destroy(own_id)` invoked from within that Thing's own handler uses the
deferred-removal rule from Section 7.

## 11. Inbound Request / Response Model [LOCKED]

This section resolves OPEN-F.

```rust
pub struct InboundRequest {
    pub thing_id: ThingId,
    pub target: AffordanceTarget,        // Property / Action / Event / Thing
    pub operation: Operation,
    pub input: InteractionInput,
    pub auth: Option<AuthMaterial>,      // peer id / token / cert fingerprint (Section 8)
    pub correlation: CorrelationId,      // opaque, echoed in the response
}

pub struct InboundResponse {
    pub output: InteractionOutput,
    pub correlation: CorrelationId,
}
```

- `CorrelationId` is an opaque newtype the binding defines (for example a zenoh
  query id). It is echoed unchanged in `InboundResponse` so the binding can
  match a response to its request.
- The matched `Form` is resolved **internally** by the dispatcher (used for
  security scheme lookup via Section 8). It is not carried on `InboundRequest`
  and is not exposed to handlers.
- `AuthMaterial` carries transport-level credentials extracted by the binding
  (peer locator, bearer token, certificate fingerprint, etc.) and is consumed
  by `SecurityProvider::verify`.
- **v1 actions are synchronous**: the handler blocks and returns its result in
  the immediate `InboundResponse`. Asynchronous action completion (HTTP/CoAP
  202-style) is deferred to a later iteration.

## 12. Crate Layout and Feature Matrix [LOCKED]

| Crate | no_std | Key change |
|---|---|---|
| `clinkz-wot-td` | yes | unchanged |
| `clinkz-wot-core` | yes | new inbound surface (`InboundRequest`/`InboundResponse`, `InboundDispatcher`), `ClientBinding`/`ServerBinding` split, `EventBroker`, `Subscription`, `SecurityProvider::verify` + `Principal` |
| `clinkz-wot-protocol-bindings` | yes | unchanged |
| `clinkz-wot-protocol-bindings-zenoh` | planning layer yes / runtime layer std | new server side (queryable, put listener, publisher) sharing the client session; dual backend zenoh-pico (no_std) and zenoh (std) |
| `clinkz-wot-discovery` | yes (local) / std (storage) | unchanged |
| `clinkz-wot-servient` | crate root yes | `Servient<D>` generic collapse; interior-mutability `&self` API; typed handles; sync/async driving layer; global start/stop removed; two-level registry locking |

Feature matrix for `clinkz-wot-servient`:

| Combination | Available driving API | Concurrency | Platform |
|---|---|---|---|
| default (sync flavor) | `poll_serve_sync` + `serve_sync` | `RefCell`, `!Send` | bare no_std super-loop |
| `async` (no_std via embassy) | `poll_serve` + `serve` | `critical_section`/`embassy_sync`, `Send + Sync` | embassy MCU |
| `std` + `async` | `poll_serve` + `serve` | `std::sync::Mutex`, `Send + Sync` | tokio / async-std |

## 13. Zenoh Operation Mapping (validates feasibility) [LOCKED]

| WoT operation | Server side (zenoh) | Client side (zenoh) |
|---|---|---|
| readproperty | `declare_queryable` | `get` |
| writeproperty | listen for `put` on key | `put` |
| invokeaction | `declare_queryable` | `get` with payload |
| observeproperty / subscribeevent | `publisher` on key | `subscribe` |

## 14. Resolved Decisions Index

All items that were open in v2.2 are now locked.

| Item | Topic | Resolution |
|---|---|---|
| OPEN-A | Concurrency & sharing model | Shared interior-mutable Servient; `&self` everywhere; two-level (map + per-Thing) locking; `Send` always, `+ Sync` for async; `serve(self)` for `'static + Send` spawn; deferred `destroy(own_id)` removal. (Section 7) |
| OPEN-C | Inbound security | Symmetric `SecurityProvider::verify(&InboundRequest, &SecurityScheme) -> Result<Principal, SecurityError>` before dispatch; form resolved internally; authenticate + optional scope check. (Section 8) |
| OPEN-D | Subscription data flow | `EventBroker` fan-out to `PublisherSink`s for inbound events; bounded per-subscription queue for outbound events; drop-oldest + overflow counter. (Section 9) |
| OPEN-E | `expose()` coordination | Map-lock atomicity; binding route failure fatal, Directory publish failure non-fatal; routes-first `destroy()`; reuse zenoh planner inbound. (Section 10) |
| OPEN-F | Inbound request shape | `InboundRequest { thing_id, target, operation, input, auth, correlation }` + `InboundResponse { output, correlation }`; internal form resolution; synchronous v1 actions. (Section 11) |

## 15. Relationship to Existing Documentation

This baseline is the authoritative Servient design reference. When the
implementation plan is written under `docs/plan/servient-runtime-redesign-plan.md`,
it must be validated against this document. The following existing documents
will need follow-up updates as the redesign lands:

- `PLAN.md` M6 section (status, current scope, exit criteria).
- `docs/technical-spec.md` Servient and feature policy sections.
- `docs/no-std-embedded.md` embedded capabilities and feature policy.
- `docs/verification.md` and `scripts/check-no-std.sh` once the new crate
  surfaces and feature flags stabilize.
