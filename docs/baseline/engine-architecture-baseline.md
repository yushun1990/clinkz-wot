# clinkz-wot Engine Architecture Baseline (v4.0)

This document is the consolidated, authoritative **engine-wide** architecture
baseline for `clinkz-wot`. It supersedes the Servient-only baselines
`docs/baseline/servient-design-baseline.md` (v3.0) and
`docs/baseline/servient-design-baseline-addendum.md` (v3.1) as the primary
design reference. Those two documents remain useful as historical record of the
concurrency and inbound-surface decisions that v4.0 inherits; where v4.0
diverges, the divergence is explicit and LOCKED here.

v4.0 is a **one-shot breaking refactor** triggered by three direction decisions
that collapse most of the accumulated complexity:

1. **Full WoT Scripting API alignment** — the engine targets Consumer, Producer,
   and Discovery *User Agent conformance* rather than the previous "Native WoT
   Runtime, Scripting API as design reference only" stance
   (`docs/wot-compliance.md` §Scripting API Boundary). This reverses the old
   positioning.
2. **No dynamic affordance lifecycle in v1** — a Thing Description is frozen at
   `expose()` time. `add_property` / `remove_*` after `expose`, and the per-
   affordance `register_affordance` / `unregister_affordance` binding surface,
   are removed. They return in a later iteration behind an explicit feature.
3. **Async-first, sync as a super-loop adapter** — handler traits and the
   driving layer are async. The four-way sync/async handler and driving
   duplication is collapsed to a single async surface plus a manual-poll
   primitive for bare `no_std` super-loops.

Every design decision below is **LOCKED**.

## 0. Specification Targets and Conformance Posture

Target specifications (normative):

- W3C WoT Architecture 1.1.
- W3C WoT Thing Description 1.1 (TD 2.0 stays behind `td2-preview`).
- W3C WoT Discovery.
- W3C WoT Profile.
- W3C WoT Scripting API — now a **conformance target**, not merely a design
  reference. The engine aims for the Consumer, Producer, and Discovery
  conformance classes defined by the Scripting API. Rust idiom (Result instead
  of throw, `impl Future` instead of Promise, owned buffers) is the *syntax*;
  the *method set, parameter semantics, and error model* follow the Scripting
  API.

Consequences of the posture change:

- The `WoT` facade, `ExposedThing`, `ConsumedThing`, and `ThingDiscovery`
  surfaces are defined against the Scripting API method catalogue. The
  conformance bar for a Thing is a conformant TD *plus* the protocol behavior
  declared by its forms *plus* faithful Scripting API interaction semantics.
- Engine-specific deviations from the Scripting API (notably the pull-queue
  subscription delivery model, §9) are documented as such and are the minimum
  set required for `no_std + alloc` safety. They do not invalidate the
  interaction *semantics*.

**Engineering-priorities posture.** Scripting API alignment is a *target*, not
a constraint that overrides engineering judgment. The primary criteria for
design decisions are **performance, stability, extensibility, and code
reasonableness**; where strict Scripting API adherence (naming or structure)
conflicts with these, the engineering-best choice wins and the divergence is
recorded as an explicit §9 deviation with rationale. The §10 conformance map is
therefore the *default* surface, amended by the §9 list — not a contract that
overrides engineering concerns. (Worked example: the per-operation handler
trait model — nine sync traits plus nine async twins, one per interaction
operation — is retained because the nine operations have genuinely
heterogeneous signatures: observe/subscribe carry a `PushFn` initial-value
callback that read/write/invoke/query/cancel/unobserve/unsubscribe do not, so
collapsing them into one multi-method trait would require either a fat-trait
ISP violation or a god-signature request struct. Per-op traits are the
engineering-cleanest shape; this also happens to be Scripting-API-aligned, so
no §9 deviation arises there.)

**Naming and idiom posture.** Scripting API conformance is *method-catalogue +
parameter-semantics + error-model* alignment, expressed in Rust idiom. It is
**not** verbatim replication of the JS API's type or method names. Concretely:

- The **Servient User-Agent surface** is the Scripting-API-aligned layer: the
  `WoT`/`Servient` facade (`produce`/`consume`/`discover`/`fetch_td`), the
  handles (`ExposedThingHandle`, `ConsumedThingHandle`, `ThingDiscoveryProcess`),
  and the method catalogue (`read_property`, `write_property`, `invoke_action`,
  `observe_property`/`unobserve_property`, `subscribe_event`/`unsubscribe_event`,
  `query_action`/`cancel_action`, `emit_event`/`emit_property_change`,
  `expose`/`destroy`, the `set_*_handler` family) — Rust `snake_case` renderings
  of the Scripting API method names, mapped 1:1 in §10.
- The **engine-internal concrete types** (`ExposedThing`, `ConsumedThing`,
  `LocalThing`, `InteractionInput`/`InteractionOutput`/
  `InteractionOptions`/`InteractionStatus`, `EventBroker`, `PushFn`, the
  `*Handler` traits, `WotLock`) are Rust-idiomatic engine types. They implement
  the Scripting API *semantics* but are shaped to Rust, not to JS. The Servient
  wraps `ExposedThing`/`ConsumedThing` in `ExposedThingHandle`/
  `ConsumedThingHandle` (the application-facing, `Arc`-clonable surface); that
  `*Handle` layer is what corresponds 1:1 to the Scripting API's
  `ExposedThing`/`ConsumedThing`, while the core `ExposedThing`/`ConsumedThing`
  nouns denote the concrete thing state the handles wrap (deliberate
  `Thing` + `Handle` Rust-idiom split). `LocalThing` keeps its `Local` qualifier
  to disambiguate from the data-model `clinkz_wot_td::Thing`. These naming
  choices are governed by this §0 posture and the Rust API Guidelines; they are
  **not** §9 behavioral deviations and do not require individual §9 entries.
- Engine-specific handler-signature details the Scripting API does not pin down
  (e.g. the observe/subscribe handler's `PushFn` initial-value callback, which
  models CoAP-Observe-style "current value on subscribe") are implementation
  shapes, not interaction-semantics deviations.

## 1. Design Principles

1. **Layering is non-negotiable.** Data contract (TD/TM) knows nothing of
   transport. Interaction core knows nothing of concrete protocols. Bindings
   know nothing of discovery or servient composition. Servient composes; it
   owns no domain logic.
2. **Interaction semantics are the primary abstraction; forms are transport.**
   `read_property` / `write_property` / `invoke_action` / `subscribe_event` are
   the engine's verbs. Form selection and protocol op mapping are machinery the
   core never sees.
3. **Async is the canonical execution model.** `no_std` super-loops drive the
   same async futures by manual polling. There is no parallel synchronous trait
   hierarchy.
4. **One lock primitive, always correct.** `WotLock<T>` (replacing the misnamed
   `MapLock<T>`) is always multi-thread safe: `std::sync` on `std`,
   `critical_section` on `no_std`. The `RefCell` / `UnsafeCell` / `multithread`-
   feature three-way bifurcation (`core/src/sync.rs`) is removed. `WotLock<T>`
   is itself a cheaply-cloneable (`Clone`) `Arc`-backed handle, eliminating the
   ubiquitous `Arc<MapLock<T>>` nesting.
5. **`no_std + alloc` is the baseline contract.** Every crate whose
   responsibility permits it compiles `no_std + alloc`. Networking, async
   runtimes, filesystems, and OS APIs live behind `std` features or in separate
   runtime crates.
6. **Stable unknown-field round-trip fidelity.** TD/TM documents are preserved
   verbatim through deserialization and serialization, including extension
   vocabulary and JSON-LD contexts.

## 2. Crate and Module Map

The crate boundaries are sound and are kept. The rewrites are *inside* the
crates.

| Crate | Path | `no_std+alloc` | v4.0 change |
|---|---|---|---|
| `clinkz-wot-td` | `td` | yes | Keep. Internal cleanups only. |
| `clinkz-wot-core` | `core` | yes | **Rewrite.** Single async interaction surface; concrete Thing types; single lock. |
| `clinkz-wot-protocol-bindings` | `protocol-bindings/core` | yes | Keep. No external change. |
| `clinkz-wot-protocol-bindings-zenoh` | `protocol-bindings/protocols/zenoh` | planning yes / runtime std | Real async consume; drop dynamic-affordance API. |
| `clinkz-wot-discovery` | `discovery` | yes (local) / std (storage) | **Rewrite** per `docs/plan/discovery-directory-refactor-plan.md`. |
| `clinkz-wot-servient` | `servient` | crate root yes | **Simplify.** Drop `Servient<D>`; async-only driving; frozen-TD lifecycle. |
| `clinkz-wot-codecs-cbor` | `codecs/cbor` | yes | Keep. |

## 3. Tier 0 — `clinkz-wot-td` (Data Contract)

Largely healthy. No public API change. Internal cleanups (tracked in
`docs/deferred-design-followups.md`):

- Split `td/src/core/data_type.rs` (957 lines, catch-all) into cohesive
  modules: `core/uri.rs`, `core/metadata.rs`, `core/version.rs`,
  `core/response.rs`, `core/operation.rs`.
- Extract a shared `FormData` core to deduplicate `ThingModelForm` from `Form`
  (deferred #6).
- Extract shared Thing/ThingModel affordance validation helpers (deferred #7).
- Convert free-form `String` error messages to structured enum variants where
  callers match programmatically (deferred #8).
- **Re-export `AbsoluteUri` at the td crate root**
  (`pub use core::data_type::AbsoluteUri;`) — audit defect AD11: P1 discovery
  uses `AbsoluteUri` as a public type (`DiscoveryEndpoint`, `DirectoryRef`,
  `DirectoryQuery`); it is already defined at `core/data_type.rs:86` and
  reachable via `data_type::AbsoluteUri`, but the root re-export is a hard P1
  prerequisite, not an open question.

## 4. Tier 1 — `clinkz-wot-core` (Interaction Core) — REWRITE

This is where the divergence and complexity concentrate. v4.0 rewrites the
public surface.

### 4.1 Thing types become concrete

The single-impl `ExposedThing` and `ConsumedThing` **traits**
(`core/src/thing.rs`) are removed (deferred #3). The names are reused for the
two concrete types `core` now owns (the trait indirection is gone; these are
the concrete thing-state types the Servient wraps in `*Handle`s):

- `ExposedThing` — a produced Thing plus its handler set. Lives in core so
  the protocol-neutral dispatcher can drive it.
- `ConsumedThing` — a consumed Thing plus its resolved binding plan. Lives
  in core so the consumed dispatch path can invoke it.

`Servient` wraps these in `Arc`-based handles (`ExposedThingHandle`,
`ConsumedThingHandle`), exactly as today, but the indirection trait is gone.

### 4.2 Handler model — sync primary, opt-in async

The nine synchronous single-method handler traits (`core/src/thing.rs`) are
collapsed to a **coherent, consolidated handler model**: one trait per
interaction operation, with **synchronous handlers as the primary,
zero-allocation path** and **an async twin per operation** (all nine, not a
subset) as an opt-in variant for I/O-bound cloud/gateway handlers.

**Why sync-primary.** A handler invocation is the inbound hot path — every
remote property read / event subscription triggers one. On an always-on MCU
gateway doing thousands of interactions per second, an `async_trait` `Box` per
call would fragment the heap over time and add WCET. Handlers are semantically
short callbacks (read a register, return a value), naturally synchronous for
the dominant device case. So the primary handler traits are plain synchronous
`fn`s stored as `Arc<dyn …>`: dispatch is a direct virtual call, **zero
per-interaction heap allocation**.

```rust
// Primary: synchronous, plain trait, zero-alloc dispatch.
pub trait PropertyReadHandler {
    fn read(&self, input: &InteractionInput) -> CoreResult<InteractionOutput>;
}
pub trait PropertyWriteHandler {
    fn write(&self, input: &mut InteractionInput) -> CoreResult<InteractionOutput>;
}
// PropertyObserveHandler, PropertyUnsubscribeHandler,
// ActionHandler (invoke), ActionQueryHandler, ActionCancelHandler,
// EventSubscribeHandler, EventUnsubscribeHandler — all plain sync `fn`.
```

**Opt-in async variant (all nine operations).** A handler that legitimately
needs to await (a cloud handler querying a DB, setting up a downstream
subscription, calling another service) cannot block the executor. **Every one
of the nine operations has an async twin** behind the `async` feature
(`#[async_trait]`, `+ Send + Sync`) — observe/unobserve, query/cancel, and
event subscribe/unsubscribe included, not just read/write/invoke. Partial
coverage would force cloud/gateway handlers on the uncovered interactions to
block the executor or bypass the unified abstraction. Registration offers both
flavors per slot; at most one occupies a slot. The async path pays one
`async_trait` `Box` per call, which is acceptable because the handler is
I/O-bound (the Box is noise next to the awaited work).

```rust
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait AsyncPropertyReadHandler: Send + Sync {
    async fn read(&self, input: &InteractionInput) -> CoreResult<InteractionOutput>;
}
// Async twins for ALL nine operations, behind `async`:
// AsyncPropertyRead/Write/Observe/UnsubscribeHandler,
// AsyncActionHandler (invoke) + AsyncActionQuery/CancelHandler,
// AsyncEventSubscribe/UnsubscribeHandler.
```

**Consolidated storage.** One handler-set struct per affordance, with one slot
per operation, each slot holding whichever flavor was registered. This
eliminates the nine parallel trait-object maps:

```rust
pub enum ReadHandler {
    Sync(Arc<dyn PropertyReadHandler>),       // zero-alloc dispatch (primary)
    #[cfg(feature = "async")]
    Async(Arc<dyn AsyncPropertyReadHandler>), // Box per call (opt-in)
}
pub struct PropertyHandlerSet {
    pub read:    Option<ReadHandler>,
    pub write:   Option<WriteHandler>,
    pub observe: Option<ObserveHandler>,
    pub unobserve: Option<UnobserveHandler>,
}
pub struct ActionHandlerSet { invoke, query, cancel }
pub struct EventHandlerSet  { subscribe, unsubscribe }
```

`ExposedThing` holds `Map<AffordanceName, Arc<HandlerSet>>` (audit H1 —
**single unified model**: std = `im::OrdMap` behind `ArcSwap` snapshot; no_std =
`BTreeMap` behind `WotLock`+clone-out), with Action/Event equivalents. Each
affordance's handlers are a plain `Arc<HandlerSet>` **value in the map** — NOT a
separate per-affordance `ArcSwap` cell (the earlier "via `ArcSwap`" wording was
ambiguous and is withdrawn). Registration methods (`set_property_read_handler`)
rebuild the ONE affected `Arc<HandlerSet>` (one alloc, one slot changed) and do a
single map insert (O(log n)); other affordances' `Arc<HandlerSet>` are shared,
not rebuilt. Dispatch looks up the affordance, clones the `Arc<HandlerSet>`,
reads the slot, and invokes — all outside any lock (clone-out / snapshot load).
An absent slot yields `CoreError::MissingHandler`.

Bounds: sync handler trait objects are `Send + Sync` (so `Arc` clones share a
handler across concurrent dispatches and the driving loop stays `Send`). The
current divergence where sync handler trait objects are non-`Send` (addendum
§9.3) is thereby resolved.

**Panic safety in dispatch (audit G1 — locked; std-only contract — audit
round-2 AD36/C1).** A handler is user code; it may panic. The lock is already
safe on every build (the handler `Arc` is cloned out under a brief lock and the
lock released *before* the invoke, and `WotLock::with_recover` heals lock state
across a panic), so a handler panic does **not** poison the registry/handler-
table locks. The fate of the panic itself is **feature-split**, because
`std::panic::catch_unwind` is std-only and bare-metal is conventionally
`panic = "abort"` (no unwinding to catch):

- **`std` builds:** the `InboundDispatcher` wraps every handler invocation
  (sync direct call and async `.await`) in `std::panic::catch_unwind`,
  converting a caught panic into an `InboundResponse::error`
  (`CoreError::HandlerPanic { target, operation }`, status-mapped via
  `error_status` to a 5xx-class reply) so the offending request fails cleanly
  and the driving loop keeps serving other Things. This is the panic→error-reply
  guarantee.
- **`no_std` builds:** `catch_unwind` is unavailable. The engine guarantees
  only **lock integrity** — `with_recover` heals lock state and the handler
  `Arc` was cloned before the invoke, so a panic never corrupts registry/
  handler-table state — but the panic itself propagates to the **platform panic
  handler** (`panic = "abort"` ⇒ reset/restart). The panic→error-reply guarantee
  is a **`std`-only** contract, deliberately not pretended on `no_std`. `no_std`
  deployments needing panic containment must keep handlers panic-free by
  construction (the dominant device case — handlers are short register reads)
  or run on a `std` host.

So the contract is honest about the split: handler authors should avoid panics
everywhere; on `std` the engine is additionally robust to one via
`catch_unwind`, on `no_std` it is robust only at the lock-state level. This
rescopes AD30 from the earlier blanket "every invocation" wording, which was
unmeetable on the very `no_std` hot path that sync-primary handlers exist to
serve.

**Non-blocking rule covers the whole inbound hot path (audit round-2 O2/AD43).**
The "must not block the executor" rule is not handler-only.
`SecurityProvider::verify` (§7.5) runs synchronously on the inbound dispatch
path **before** the handler, and expensive crypto there (JWT/signature
validation) blocks the executor exactly as a blocking handler would. The same
contract therefore applies to `verify`: a sync `verify` must be non-blocking/
short; deployments whose verification is genuinely I/O-bound or CPU-heavy need
an async twin. An `AsyncSecurityProvider` (`verify`/`apply` async twins) is
recorded as a deferred follow-up (`docs/deferred-design-followups.md`); until it
lands, `verify` is treated as part of the sync hot-path budget and must respect
it.

### 4.3 Interaction I/O aligned to Scripting API

`InteractionInput` / `InteractionOutput` are reworked to mirror the Scripting
API's `InteractionOptions` (Scripting API §7.1) and response shapes:

```rust
pub struct InteractionOptions {
    pub uri_variables: BTreeMap<String, String>,
    pub form_index: Option<usize>,
    pub data: Option<Payload>,
    pub timeout: Option<Duration>,
}

pub struct InteractionOutput {
    pub data: Option<Payload>,
    pub status: InteractionStatus,
}
#[non_exhaustive]
pub enum InteractionStatus {
    /// Normal completion (default; HTTP/CoAP 200-equivalent).
    Ok,
    /// A new resource was created (201-equivalent).
    Created,
    /// An async action was accepted, not yet complete (202-equivalent; future).
    Accepted,
}
```

**Naming consistency (audit D3).** The payload-bearing field is named `data`
everywhere — `InteractionInput.data` (handler-facing, inbound), `InteractionOptions.data`
(caller-facing, outbound/consumed), `InteractionOutput.data`. The prior
`InteractionInput.payload` is renamed `data`. URI-template variables are named
`uri_variables` everywhere — `InteractionInput.uri_variables` (renamed from
`parameters`) and `InteractionOptions.uri_variables`. (`InteractionInput` keeps
its inbound-only `principal` field **and gains an inbound-only `accept:
Option<AcceptHint>`** (audit round-2 O7/AD48 — see encoding boundary below);
`InteractionOptions` keeps its outbound-only `form_index`/`timeout`. The two
types differ by context but share field names for the concepts they have in
common.)

The current `InteractionInput.security_metadata` field is removed from the
handler-facing type. Security material belongs to the binding/transport layer,
not to handler inputs. Outbound security application stays on the
`SecurityProvider`/binding path; the verified `Principal` remains on the
inbound handler input (addendum §T3 is kept).

**Encoding boundary — handlers are byte-level (audit G3 — locked).** Both
`InteractionInput.data` and `InteractionOutput.data` carry a `Payload` whose
body is `Arc<[u8]>` (already-encoded bytes) plus media metadata. Handlers are
therefore **byte-level on both sides**: an inbound handler receives the
request's decoded-to-bytes payload and returns an already-encoded payload; it
does not deal in schema values, and the runtime does **not** auto-encode a
logical value the way the Scripting API's value-returning handlers do. The
`PayloadCodecRegistry` (§7.1 `codecs`) is applied at the **transport edge**: a
binding decodes the wire body to the `Payload` bytes the handler reads and, on
the outbound/consumed path, encodes the caller's `Payload` to the wire format
matched to the form's `contentType`.

**Content-negotiation hint on the byte-level input (audit round-2 O7/AD48,
**E1 corrected).** Because the handler emits already-encoded bytes with no value
type in between, it must itself pick an output content type. Doing that blind
(without knowing the client's `Accept`) risks emitting a type the client cannot
consume. To close that gap the inbound `InteractionInput` carries an `accept:
Option<AcceptHint>` (a compact representation of the request's
`Accept`/content-type preferences, populated by the binding at the edge). A
byte-level handler reads `accept` to choose an output `Payload` content type the
client will accept in one encode pass. **If the handler ignores the hint and
emits a mismatched content type, the edge does NOT transcode** (audit E1: the
engine has no value type to transcode through — deviation #4/AD32; "decode→
re-encode" would require an intermediate value type that does not exist).
Instead, the mismatch is an **error**: the binding returns
`CoreError::ContentTypeMismatch` (status-mapped) so the client gets explicit
feedback. Transcoding, if ever needed, is an **application-layer codec** the
caller registers explicitly (a `PayloadCodec` that produces a value type), not
an engine-layer fallback. `AcceptHint` is a small, protocol-neutral struct
(a preferred `MediaType` plus an optional ordered list), `no_std + alloc`-safe;
it carries no protocol headers. This byte-level handler contract is the
companion to §9 deviation #4.
needing value semantics encode/decode inside their handler.

### 4.4 Affordance addressing and correlation

Retained from v3.1 §1/§2: `ThingId`, `CorrelationId`, `AffordanceTarget`
(`Arc<str>`-backed, owned, `'static`), `InboundRequest`, `InboundResponse`,
`BindingRequest` (owned, `Arc<Thing>` / `Arc<Form>`). These are correct and
unchanged.

### 4.5 Binding trait split

Retained: `ClientBinding` (outbound) and `ServerBinding` (inbound), both `&self`
with interior mutability (v3.0 §2, v3.1 §2.4). The dynamic-affordance methods
`register_affordance` / `unregister_affordance` added in addendum §9.2 are
**removed** (decision 2). A binding registers a Thing's routes wholesale during
`expose()` and unregisters them during `destroy()`.

`ClientBinding::invoke` / `subscribe` are `async fn` (resolved A1) — the
outbound path; one `async_trait` `Box` per call, accepted as network-amortized.

**Inbound accept uses a fan-in channel, not `select_all` over boxed
`poll_accept` futures** (audit defect 1). `ServerBinding` exposes a single
**synchronous, non-blocking** `try_accept`:

```rust
pub trait ServerBinding: Send + Sync {
    /// Non-blocking drain of one currently-ready inbound request, or `None`.
    /// No `async_trait`, no `Box` — a plain virtual call. (no_std polled path.)
    /// Default `None` (audit F8): a std-only binding that self-pushes via
    /// `set_request_sink` never has `try_accept` called and need not override it.
    fn try_accept(&self) -> Option<InboundRequest> { None }
    /// The reply path (audit F1): `InboundRequest` carries no reply handle, so
    /// the dispatcher's `InboundResponse` is returned via `send_response`,
    /// matched back to the requester by `CorrelationId`. Required by AD9's
    /// "overload → explicit error reply" semantics. No default — every binding
    /// that accepts requests must implement it.
    fn send_response(&self, response: InboundResponse);
    /// EventBroker injection (audit F1): the Servient calls this at registration
    /// so the binding can register `PublisherSink`s for event/observable fan-out
    /// during `register_thing`. Default no-op for bindings without event publish.
    fn set_event_broker(&self, _broker: EventBroker) {}
    /// std fan-in injection (audit defect AD13): the Servient hands each
    /// binding a clone of the bounded fan-in sender at registration; the
    /// binding `try_send`s from its sync transport callbacks. Formalized on
    /// the trait so the std main path is not prose-only implicit coupling.
    #[cfg(feature = "std")]
    fn set_request_sink(&self, sender: FanInSender<InboundRequest>);
    /// Wholesale route registration for one Thing during `expose()`. Returns
    /// `Result<(), CoreError>` so the multi-binding rollback (E12/AD27) can
    /// detect a binding `k+1` failure, `unregister_thing` the succeeded
    /// `1..k`, and surface a fatal `Err` (audit round-2 C3/AD38 — the earlier
    /// `()` sketch could not fail, contradicting the rollback contract). A
    /// binding reports a structural failure (cannot register routes for this
    /// TD) via a structured `CoreError` (mapped from its `BindingError`),
    /// never a `String`, so it threads through `error_status`/`ServientError`.
    fn register_thing(&self, thing_id: &ThingId, td: &Thing) -> Result<(), CoreError>;
    /// Wholesale route removal during `destroy()`. Returns `()` — `destroy()`
    /// is idempotent (AD27/E13) and best-effort across bindings: a failure to
    /// unregister one binding does not abort teardown of the rest (logged, not
    /// fatal), matching the "Thing gone" end state.
    fn unregister_thing(&self, thing_id: &ThingId);
}
```

The driving loop never builds a `select_all` wait set over per-binding boxed
futures. Instead it uses a **single bounded fan-in channel** as the one and
only inbound buffer:

- **std path (main):** the Servient owns one **bounded** fan-in channel
  (`FanInSender<InboundRequest>` / `Receiver`). At registration the Servient
  calls `ServerBinding::set_request_sink(sender)` (AD13) to hand each binding a
  sender clone; the binding enqueues inbound requests from its **synchronous**
  transport callbacks via **`fanin_tx.try_send(req)`** — zenoh callbacks are
  sync closures (`move |query| { … }`, `server.rs:558,601`) and cannot `.await`.
  Bounded capacity ⇒ on `Full` a **policy split by interaction kind** (audit
  defect AD9): request/response is **rejected with an explicit error reply**
  (mapped via `error_status`, immediate client feedback — not silent
  drop/timeout); streaming/events use drop-oldest + overflow counter — there is
  **no binding-internal accept queue** (audit defect AD6a) and no async-bridge
  task. The driving loop is `receiver.recv().await` — **O(1) per step, zero
  per-binding boxing, one request per step**.
- **no_std path:** there is no executor, so bindings cannot self-push; the
  driving loop takes **one** request per tick with a **rotation cursor** so no
  binding is starved:
  `let start = cursor.fetch_add(1) % n; for i in 0..n { let b = snapshot[(start+i)%n]; if let Some(r) = b.try_accept() { dispatch(r); break; } }`
  — the start offset advances each tick, delivering round-robin fairness;
  strict one request per tick, no backlog drain (audit defects AD6b/AD7).
  O(N_bindings) per tick but N is the protocol-binding count (typically 1–5),
  each poll a plain sync virtual call.

This removes the `poll_accept_sync` / `AsyncServerBinding` / boxed-`poll_accept`
surface entirely (addendum §6.2, §9.6 superseded). On std, `try_accept` is
unused (direct push is the main path); on no_std, the zenoh-pico backend's
`try_accept` polls its transport and returns one ready request. The sync
driving primitive (§7.2) drives the same one-step loop.

**`FanInSender` definition (audit D16).** `FanInSender<InboundRequest>` is a
core-defined, **std-only** type alias for the bounded fan-in channel sender —
concretely `async_channel::Sender<InboundRequest>` (runtime-neutral: works
under tokio/async-std/embassy-std; its `try_send` is synchronous, matching the
sync zenoh-callback enqueue). Defined in `clinkz-wot-core` behind `#[cfg(feature
= "std")]`; the Servient constructs the `async_channel::channel(capacity)` pair
and owns the `Receiver`. no_std has no `FanInSender` (no channel — the loop
polls `try_accept`).

### 4.6 Subscription primitives

Retained: `EventBroker` (inbound event fan-out) and `Subscription`
(outbound pull-queue with drop-oldest + overflow counter). The queue capacity
model (v3.1 §6.1) is retained. The pull-queue delivery model is the documented
deviation from the Scripting API's listener callback (§9).

**Async `Stream` waker (audit E17).** The `Subscription` queue owns an
`Option<core::task::Waker>`. The producer side (a sync zenoh callback that
`try_push`es a sample into the queue) calls `wake()` on the stored waker after a
successful push — the callback need not `.await`, it only touches the
`Option<Waker>` under the queue's brief lock. The async consumer
(`Subscription::next().await` as a `Stream`) registers its `Waker` when the
queue is empty (returns `Pending`); the next push wakes it. So the
sync-callback-producer / async-consumer concurrency is well-defined: no
executor is needed on the producer side, only on the consumer side.

**`InteractionOptions.timeout` executor (audit E16/H2 — build-time cfg,
fail-closed per AD45).** Outbound timeout is applied via **build-time cfg**
inside the Servient outbound path, NOT a runtime `dyn` trait (audit H2: the
earlier `OutboundTimeout` trait had a generic method and was not object-safe):
- **std** (tokio): the outbound path wraps `tokio::time::timeout(dur,
  binding.invoke(req))` when `options.timeout.is_some()`.
- **no_std + async** (embassy): `embassy_time::with_timeout` behind the
  `embassy` feature.
- **bare no_std**: no timer available — see fail-closed below.
No trait object, no per-call boxing.

On a build **without** a timer (bare `no_std`),
a caller-supplied `InteractionOptions.timeout` is **never silently ignored**:
the outbound call returns `Err(CoreError::TimeoutUnsupported)` immediately
(fail-closed) instead of hanging indefinitely. A `timeout` left `None` behaves
normally (no timeout enforced). Silently dropping a caller's explicit safety
bound would let a deadlocked `consume` block forever with the caller believing
it was bounded. The earlier "no-op, retained for API symmetry" wording is
withdrawn (audit round-2 O4).

**Inbound async handler — no timeout bound (audit M6, known boundary).**
`InteractionOptions.timeout` is **outbound-only**. An inbound opt-in async
handler has no per-handler timeout: if it hangs (bug/malice), its future never
completes, permanently occupying a `max_inflight` slot (AD42/H4). The primary
defense is the §4.2 **non-blocking contract** (handlers must not block), but
there is no enforcement against a malicious/buggy handler that violates it. A
per-handler timeout is a **future hardening item**; v1 documents this as a
known boundary.

### 4.7 Single lock primitive — `WotLock<T>`

The `MapLock<T>` name (which implied it locked a `Map`, yet appeared as
`MapLock<()>`, `MapLock<Vec<…>>`, `MapLock<BindingFactoryState>`) is renamed to
`WotLock<T>`: the WoT engine's portable, always-thread-safe, interior-mutable
lock container. The name is domain-scoped ("WoT" spans every layer of the
engine, so it does not tie a core primitive to a higher layer the way
`ServientLock` would) and handles the pure-lock case `WotLock<()>` naturally
(which a "Cell" name would not). It is also reworked to be itself a
cheaply-cloneable `Arc`-backed handle (`Clone`), so the pervasive
`Arc<MapLock<T>>` nesting becomes plain `WotLock<T>`.

`core/src/sync.rs` becomes:

| Build | `WotLock<T>` backing |
|---|---|
| `std` | `std::sync::RwLock<T>` (read-mostly) / `std::sync::Mutex<T>` (exclusive) |
| `no_std` | `critical_section::Mutex<T>` (there is no blocking `RwLock` for `no_std` — `critical_section` is the primitive; `embassy_sync`'s RwLock is async-only and thus the wrong tool for these always-synchronous brief holds) |

The `RefCell` single-thread backend and the `multithread` feature are removed.
On a bare single-thread `no_std` target, `critical_section` resolves to a
disable-interrupt / no-op implementation that is correct and cheap. This
removes the entire `sync_lock` / `async_lock` / `DrainFlag` / `multithread`
matrix of addendum §9.1.

API shape:

```rust
pub struct WotLock<T> { /* Arc<RwLock<T> | Mutex<T>> */ }
impl<T> WotLock<T> {
    pub fn new(value: T) -> Self;
    pub fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R;        // exclusive
    pub fn with_read<R>(&self, f: impl FnOnce(&T) -> R) -> R;       // shared (exclusive on no_std)
}
impl<T> Clone for WotLock<T> { /* refcount bump */ }
```

Two-level locking is retained (v3.0 §7): an outer registry `WotLock` for
insert/remove/enumerate, an inner per-Thing `WotLock` held only across a single
handler call. The reentrancy discipline (clone handler `Arc` out under a brief
lock, release, then invoke) is retained.

**Read-heavy-rare-write state — per-build strategy (AD2, corrected by C1/AD54).**
The hot read path must not hold a lock during handler invocation (AD2's intent).
How this is achieved depends on the build:

- **std (tokio host/cloud): lock-free `arc-swap` snapshot.** The registry is
  `arc_swap::ArcSwap<Arc<im::HashMap<ThingId, Arc<ThingSlot>>>>` — a write does
  `arc_swap.store(Arc::new(map.insert(..)))` (atomic, lock-free, O(1) amortized
  insert via HAMT structural sharing); a read does `arc_swap.load()` (lock-free
  `Guard`, zero contention across tokio threads). **`HashMap` (not `OrdMap`) is
  chosen for the registry** (audit M2: registry lookup is exact-key by
  `ThingId`, not range scan; `HashMap`'s HAMT is O(1) amortized with wider
  branching vs `OrdMap`'s O(log n) B-tree with ~10 levels at gateway scale +
  cache-unfriendly pointer chasing). Per-Thing handler tables (small N ≈ 10–50
  affordances) keep `OrdMap`/`BTreeMap` where ordering or small-N simplicity
  matters.
- **no_std (ESP32/STM32/etc.): `WotLock` + clone-out dispatch discipline.** The
  registry is `WotLock<BTreeMap<ThingId, Arc<ThingSlot>>>`. The hot read does:
  `wotlock.with_read(|m| m.get(&id).cloned())` — a **brief critical_section**
  covering only the BTreeMap::get (O(log n) ≈ 7 comparisons for n=100 ≈ 200ns)
  + Arc clone (refcount incr ≈ 10ns); then the lock is released and the handler
  is invoked **outside any lock**. The CS window is ~500ns — well within typical
  MCU real-time interrupt-disable budgets (1–10μs). **No external deps needed**
  (`WotLock` + `BTreeMap` are both in-scope `alloc` types); no `arc-swap`, no
  `im`, no in-tree `AtomicPtr`+CS snapshot primitive. The savings from a
  snapshot over WotLock+clone-out on a 240 MHz ESP32 are ~450ns/dispatch =
  ~100 cycles = negligible.

This split resolves both dependency gaps (C1: `arc-swap` and `im` are std-only,
NOT stable-`no_std`) while serving ESP32-class gateways that aggregate dozens to
hundreds of sub-device — `BTreeMap::get` is O(log n) and fast enough under the
brief CS regardless of registry size.

`WotLock` is used for the registry on `no_std` and for genuinely
read-write-frequent / exclusive-semantics state on every build (driving state,
credential store, binding-factory generation counter).

**Handler-set swap granularity (audit H1 unified, round-2 P-2/AD51).** The
single model is `Map<Name, Arc<HandlerSet>>`: each affordance's handlers are a
plain `Arc<HandlerSet>` value in the map. Replacing one slot rebuilds that ONE
`Arc<HandlerSet>` (one alloc) + one map insert (O(log n) structural sharing on
std; O(log n) insert under WotLock on no_std). Other affordances' `Arc<HandlerSet>`
are shared, not rebuilt. This is off the per-request path (handler swaps are
setup-phase wiring + occasional runtime re-attachment per AD14). If profiling
later shows per-affordance handler swapping to be a hot allocation, the
documented escape hatch is per-slot `ArcSwapOption<Arc<dyn …>>` (std only) so
one slot swaps without rebuilding the struct; v1 ships the simpler whole-HandlerSet
rebuild. Recorded in `docs/deferred-design-followups.md`.

### 4.8 Trait sealing (audit D15)

Two classes, decided explicitly (AGENTS.md favors sealing extensible traits;
deferred #8 had left this open):

- **Stable extension points — NOT sealed** (downstream crates/users implement
  these): `ClientBinding`, `ServerBinding`, the 9 sync handler traits + their
  async twins, `PayloadCodec`, `SecurityProvider`, `CredentialStore`,
  `Discoverer`, `DirectoryReader`, `DirectoryPublisher`,
  `ThingDescriptionResolver`, `ThingLinkResolver`. Documented as the public
  extension surface.
- **Engine-internal — sealed or `pub(crate)`** (no external impls):
  `DiscoverySession`, `DirectorySession`, `EventSink`, `InboundDispatcher`,
  the consolidated `*HandlerSet` storage types, `ProcessState`. These are
  implementation details; sealing prevents downstream from depending on their
  shape.

## 5. Tier 2 — Protocol Bindings

### 5.1 Shared binding (`clinkz-wot-protocol-bindings`)

Healthy. No external change. Form selection, op→form resolution, target
resolution, security metadata extraction, and the structured `BindingError`
taxonomy are kept. Minor: convert remaining free-form `String` `BindingError`
messages to structured variants (deferred #8).

**Multi-form selection priority (audit E20; form_index placement — audit
round-2 O6/AD47).** When an affordance advertises multiple forms, the shared
selector chooses by, in order: **(0) explicit `InteractionOptions.form_index`**
— if the caller pins a form by index, that form is used **directly, bypassing
`supports`**; the caller takes responsibility for the choice. A `form_index`
that points at a form no registered binding can drive returns
`CoreError::UnsupportedForm` (mapped from `BindingError`) rather than silently
falling through, so an explicit wrong choice is a loud error; then, when no
`form_index` is given: (1) the concrete binding's `supports` predicate (protocol
the binding can drive), (2) caller `FormSelectionCriteria` (content type /
subprotocol), (3) operation match. The tie-break order among equally-matching
forms (e.g. two zenoh forms with the same content type) is **deterministic by
TD declaration order** (first wins) — documented here as the v1 rule; a richer
priority policy is deferred.

**Cross-crate error interop (audit E1 — locked).** Four error types span the
crates: `CoreError` (core), `BindingError` (protocol-bindings),
`DiscoveryError` (discovery), `ServientError` (servient). The load-bearing
conversion chain (crate-boundary contract):

- `impl From<BindingError> for CoreError` — a binding's `invoke`/`subscribe`
  returns `CoreResult` (= `Result<_, CoreError>`); `BindingError` flows in via
  this conversion.
- `impl From<CoreError> for ServientError`, `impl From<BindingError> for
  ServientError` (via CoreError), `impl From<DiscoveryError> for
  ServientError` — servient methods return `ServientResult`.
- **Protocol status mapping**: `error_status(&CoreError) -> u16` (shared
  binding crate) is the single status source. Since `BindingError → CoreError`,
  binding failures map through `CoreError`. `DiscoveryError` is an
  **application-layer** error surfaced via the `ThingDiscoveryProcess` (its
  `error()`/`next()`), NOT as a protocol reply status — it does not flow through
  `error_status`. `ServientError` is unwrapped to its inner `CoreError` for
  status mapping on the inbound reply path.
- Direction: conversions go **inward** (BindingError→CoreError→ServientError);
  the inverse is not provided (no `CoreError→BindingError`), preserving layering.

### 5.2 Zenoh binding (`clinkz-wot-protocol-bindings-zenoh`)

Two changes:

1. **Real async consume.** The fake-async consumer surface (PLAN M8, "delegates
   to sync path") is replaced. `ZenohSessionTransport` exposes
   `async fn invoke(request: BindingRequest) -> CoreResult<InteractionOutput>`
   that drives the real `zenoh::Session` (`session.get`, `session.put`). The
   client binding trait becomes async, matching §4.5.
2. **Drop dynamic-affordance API.** The per-affordance
   `register_affordance`/`unregister_affordance` route tracking (addendum §9.2)
   is removed. `expose()` declares all routes for the Thing; `destroy()`
   undeclares them.

The `runtime-*` feature split (planning layer `no_std+alloc` vs concrete
`zenoh`/`zenoh-pico` backends) is retained.

## 6. Tier 3 — Discovery (`clinkz-wot-discovery`) — REWRITE

Execute `docs/plan/discovery-directory-refactor-plan.md` in full. Summary of
the target shape:

- **Introduction** discovers `DiscoveryEndpoint`s (not Things).
- **Exploration** resolves endpoints into TDs or directory sessions via
  `ThingDescriptionResolver`, `ThingLinkResolver`, `DirectoryReader`.
- **Directory** is an Exploration service with continuation-based
  `DirectorySession`, `CountMode`, `ProjectionMode` — not a local CRUD
  container. **v1 ships `ConsistencyMode::Live` only** (audit defect 3);
  `SessionStable` (snapshot-at-open) is deferred — it would re-introduce the
  large-result-set materialization cost that lazy continuation was meant to
  remove, especially for remote/large directories. `ConsistencyMode` stays
  `#[non_exhaustive]` so `SessionStable` is added non-breakingly once its
  snapshot semantics and remote-backend cost are resolved.
- **Discovery process** (`ThingDiscoveryProcess`) is a lazy session handle, not
  a buffered `VecDeque<Thing>`.
- **Publisher side** is lease/revision-aware (`DirectoryPublisher`).
- **Watch** (`DirectoryWatch`) is separate from search pagination.

`Servient` no longer carries `Servient<D>`; it holds
`Arc<dyn Discoverer>` + `Option<Arc<dyn DirectoryPublisher>>`. The in-memory
backend is demoted to a reference implementation of those traits.

`discovery/src/scripting.rs` (the transitional `ThingFilter.method` model,
`DiscoveryMethod::Local/Directory/Multicast/Everything`) is replaced by the
`Discoverer` trait surface (`discover` / `explore_directory` /
`request_thing_description`).

## 7. Tier 4 — Servient (`clinkz-wot-servient`) — SIMPLIFY

### 7.1 Shape

```rust
pub struct Servient {
    exposed: ExposedThingRegistry,
    consumed: ConsumedThingRegistry,
    server_bindings: BindingList,            // Arc-snapshot of registered ServerBindings
    client_factories: BindingFactoryRegistry,
    discoverer: Arc<dyn Discoverer>,
    directory_publisher: Option<Arc<dyn DirectoryPublisher>>,
    security: SecurityContext,
    codecs: PayloadCodecRegistry,
    event_broker: EventBroker,
    shutdown: Arc<AtomicBool>,
}
```

`Servient` is `Clone` (cheap, `Arc`-based), all methods `&self`, `Send + Sync`.

### 7.2 Driving layer — async only

The four-way sync/async duplication collapses:

```rust
impl Servient {
    /// One step: accept at most one inbound request across all bindings and
    /// dispatch it. Native async; suspends on Waker when no request is ready.
    pub async fn poll_serve(&self) -> ServientResult<()>;

    /// Convenience loop: `while !shutdown { poll_serve().await; }`.
    pub async fn serve(&self);

    /// Manual-poll primitive for bare no_std super-loops without an executor.
    /// Dual implementation by feature (C5/AD40, **E2 corrected**): on
    /// `no_std + async` (embassy) it **stores a pinned reusable `poll_serve`
    /// future** (created once, polled each tick — NOT recreated per call, E2:
    /// recreating drops the `recv().await` Pending state); on bare
    /// `no_std` (no `async` feature) it runs a **purely synchronous**
    /// accept→dispatch→reply step with no async future involved. Both return
    /// `Poll<ServientResult<()>>`; `Pending` means no request was ready.
    pub fn poll_serve_once(&self, cx: &mut core::task::Context<'_>)
        -> core::task::Poll<ServientResult<()>>;
}
```

All three driving primitives take `&self` (resolved A4), forming a consistent
family: `poll_serve_once(&self)` reuses one `Servient` across super-loop
iterations and shares it with other work, so `&self` is required there, and
`serve`/`poll_serve` match for consistency. `serve()` is spawnable on
tokio/embassy via `tokio::spawn(async move { svc.clone().serve().await })` —
the `async move` block owns the cheaply-cloned `Servient` and `serve(&self)`
borrows it (Pin makes the self-referential future sound).

**Driving primitive × feature matrix (audit D4 — locked).** `poll_serve` and
`serve` are `async fn` ⇒ gated behind the `async` feature (and need an executor:
tokio on std, embassy on no_std). `poll_serve_once` is a plain sync `fn`
available on every build — it is the bare-`no_std` super-loop primitive.

| Primitive | `std` | `no_std` (no `async`) | `no_std` + `async` (embassy) |
|---|---|---|---|
| `poll_serve_once` (sync) | yes | **yes** (super-loop) | yes |
| `poll_serve` (async) | yes (tokio) | **no** (no executor) | yes (embassy) |
| `serve` (async loop) | yes (tokio host loop, std-gated idle backoff) | **no** | yes (embassy task) |

So a bare `no_std` build (no `async` feature) exposes **only** `poll_serve_once`;
the async driving primitives require the `async` feature + an executor.

**Step contract — at most one inbound *accept* per call** (audit defect AD6b;
concurrency model — audit round-2 O1/AD42). `poll_serve` and `poll_serve_once`
each **accept** at most one request per call — they never drain a ready backlog,
so a bare super-loop stays cooperative (one request per tick, interleaved with
other work). AD6b bounds the **accept rate**, not the **completion concurrency**:

- **`std` builds:** a local `FuturesUnordered` of in-flight dispatches is
  retained so a slow opt-in async handler on one Thing does not stall
  accept/dispatch of other Things; sync handlers run inline (fast). The
  `FuturesUnordered` is bounded by a **`max_inflight` cap with poll-before-accept
  discipline** (H4 — `FuturesUnordered` itself is unbounded; the earlier
  "bounded FuturesUnordered" was false): before accepting, the loop checks
  `in_flight < max_inflight`; at capacity it polls-only. The fan-in channel fills
  → bindings backpressure per AD9. `poll_serve` accepts ≤1 new request per step
  *and* polls the in-flight set one step.
- **`no_std + async` (embassy):** the same `FuturesUnordered` + `max_inflight`
  model with `futures-util` `alloc` (cooperative, no `tokio`).
- **bare `no_std`:** strictly **serial** — one accept→sync-handler→reply per
  tick, no `FuturesUnordered`, no in-flight concept. There is no executor to
  concurrently drive multiple futures, so concurrent completion is not a thing
  here; a slow sync handler monopolizes the tick (the §4.2 non-blocking contract
  is what keeps this bounded).

This resolves the earlier tension between "inline `.await` the handler"
(P3 §3.6) and "FuturesUnordered of in-flight dispatches" (P3 §3.5): inline refers
to *how* a single dispatch runs its handler, while FuturesUnordered allows
*multiple* dispatches to be in flight concurrently on std/embassy.

**`poll_serve` / `poll_serve_once` Err semantics (audit M3).** Per-request
handler errors do NOT surface as `poll_serve`'s `Err` — they go through
`InboundResponse.error` → `send_response` → `Ok(())`. `poll_serve` returns
`Err(ServientError)` only for **infrastructure-level** failures that cannot be
attributed to a specific request: the fan-in channel is closed/disconnected
(all bindings unregistered or shutdown), or an unrecoverable driving-loop panic.
Normal operation: `Ok(())` after each step (whether a request was dispatched or
the step was idle).

**Global shutdown quiescing (audit G2 — locked).** Per-Thing `destroy()`
quiescing is AD15; the **global** `shutdown` flag (§7.1) has a parallel,
simpler contract. `serve = while !shutdown { poll_serve().await }` checks the
flag **between** iterations, so the semantics are: (1) the currently-running
`poll_serve` step finishes — the one request it accepted is dispatched and its
handler(s) run to completion (an async handler is `.await`ed, not cancelled);
(2) once that step returns and the flag is observed set, `serve` exits; (3)
any further requests already sitting in the bounded fan-in channel are **not**
drained — they are dropped when the `Servient`/fan-in channel is dropped
(callers see a transport-level connection-close, not a WoT error reply). This
is "finish-current, drop-queued", deliberately not a full drain: a host
shutting down is expected to stop accepting at the transport (bindings close
their listeners) so the queue drains to empty quickly; a long drain could
block shutdown indefinitely. For per-Thing polite teardown use `destroy()`
(AD15 gives in-flight handlers + error replies); reserve global `shutdown` for
process exit. On `no_std`, `poll_serve_once` callers honor the flag the same
way between super-loop ticks.

**Accept is a single bounded fan-in channel, not `select_all`** (audit defect
AD1/AD6a, see §4.5). The driving step does NOT build a `select_all` over
per-binding boxed `poll_accept` futures and there is **no binding-internal
accept queue**. On std the binding enqueues from its **synchronous** zenoh
callbacks via `fanin_tx.try_send(req)` (zenoh callbacks cannot `.await`;
bounded capacity — on `Full` request/response is rejected with an explicit
error reply and streaming/events drop-oldest + overflow, see AD9); the loop
`receiver.recv().await`s the single bounded fan-in channel (O(1), one request
per step). On bare no_std it takes one request per tick
`for b in snapshot.rotate_from(cursor) { if let Some(r) = b.try_accept() {
dispatch(r); cursor.advance_past(b); break; } }` (rotation cursor below;
O(N_bindings), sync, no boxing, no drain).

The bare super-loop usage:

```rust
// no_std cooperative super-loop, no executor
loop {
    let waker = noop_waker();
    let mut cx = core::task::Context::from_waker(&waker);
    let _ = svc.poll_serve_once(&mut cx);
    // ...other super-loop work (sensor reads, sub-device polling)...
}
```

The current `driving_sync.rs` / `driving_async.rs` / `DrivingState` /
`AsyncAcceptState` split is replaced by a single driving module.

### 7.3 Lifecycle — frozen TD (decision 2)

```rust
impl Servient {
    pub async fn produce(&self, td: Thing) -> CoreResult<ExposedThingHandle>;
    pub async fn consume(&self, td: Thing) -> CoreResult<ConsumedThingHandle>;
    pub fn discover(&self, filter: DiscoveryFilter) -> ThingDiscoveryProcess;
    pub async fn fetch_td(&self, url: &AbsoluteUri) -> ServientResult<Thing>;
}
```

**`fetch_td` returns `ServientResult<Thing>`** (audit round-2 C2/AD37).
`fetch_td` delegates to `Discoverer::request_thing_description`, whose return
type is `DiscoveryResult<Thing>` (P1 §1.9). The cross-crate error chain (AD25)
deliberately provides `From<DiscoveryError> for ServientError` but **not**
`From<DiscoveryError> for CoreError` — `DiscoveryError` is an application-layer
error surfaced via the discovery process, not a protocol status, and `core`
does not depend on `discovery` (layering forbids the reverse impl). The earlier
`CoreResult<Thing>` signature therefore had no legal conversion and was a
compile-level contradiction. Returning `ServientResult<Thing>` uses the existing
`From<DiscoveryError> for ServientError`; AD25 stands unchanged.

**`ThingId` uniqueness and collision (audit G5/H6 — locked).** The exposed and
consumed registries key by `ThingId`. Uniqueness is **not** synthesized by the
engine: `ThingId` is whatever the TD's `id` states (E18 — the TD must carry
one). A `produce()`/`expose()` whose `ThingId` already exists in the servable
exposed registry is **rejected** with `ServientError` (`AlreadyExposed`) rather
than silently overwriting — `destroy()` the existing Thing first. `consume()`
with a duplicate `ThingId` **reuses** the existing consumed entry (refreshing
its TD).

**Atomic check-and-insert (audit H6):** concurrent `expose`/`destroy` against
the same `ThingId` must be atomic (check + insert/remove as one operation):
- **std** (ArcSwap): a **CAS loop** — `load → check absent/present → build new
  map → compare_and_swap(current, new) → retry on mismatch`. `ArcSwap::rcu` or
  `compare_and_swap` provides this. `AlreadyExposed` / idempotent-destroy are
  correct under CAS (a concurrent writer that wins the CAS forces the loser to
  retry and see the new state).
- **no_std** (WotLock): the `with` (exclusive) critical section makes
  check + insert/remove **inherently atomic** under one lock hold.

The earlier "concurrent destroy serialize via the registry lock" wording (E13)
was std-inaccurate (std has no lock — the CAS loop is the mechanism); corrected
here to per-build atomicity.

Cross-directory/cross-origin id collision is **out of scope for v1**: a
`ThingId` is only as globally unique as the TD's `id` asserts; a deployment that
merges directories is responsible for disambiguating before expose/consume.


impl ExposedThingHandle {
    pub fn set_property_read_handler(&self, name, handler);
    pub fn set_property_write_handler(&self, name, handler);
    // ... set_action_handler, set_event_subscribe_handler, etc.
    pub async fn expose(&self) -> ServientResult<()>;   // registers routes + publishes TD; TD frozen after
    pub async fn destroy(&self) -> ServientResult<()>;  // unregisters routes + unpublishes TD
    pub async fn read_property(&self, name, options) -> ...;  // server-side local read
    pub async fn write_property(...);
    pub async fn emit_event(&self, name, data);
    pub async fn emit_property_change(&self, name, data);
}
```

There is **no** `add_property` / `remove_property` / `add_action` / `add_event`
after `expose()` — the **TD affordance set is frozen** at expose (decision 2).
**Handlers, however, may be attached or replaced throughout the exposed
lifetime** (audit defect AD14 — the earlier "handlers only between produce and
expose" wording conflicted with P3 and with the Scripting API). Rationale: a
handler is runtime behavior for an already-declared affordance, not TD
structure, and the Scripting API allows `setPropertyReadHandler` etc. at any
time. An affordance whose handler slot is still `None` returns
`CoreError::MissingHandler` — a **designed-in** semantic for an exposed-but-
unwired affordance, not an error condition. (Handler swap publishes a new
`Arc` handler-set snapshot; an in-flight dispatch keeps the handler `Arc` it
cloned out.) **Lifecycle state machine (AD8):** `produce()` creates a draft
handle whose `Arc` state (TD + handler slots) lives in **no registry**;
`expose()` is the **single** insertion into the servable exposed registry
(ThingSlot wrapping that `Arc` state) + route registration + TD publish;
`destroy()` is the **single** removal (Thing **gone**, not back to draft —
re-`produce` to re-expose). One insertion, one removal, no second "becomes a
registry thing" point. `expose()` registers all inbound routes wholesale and
publishes the TD; the TD affordance set is immutable thereafter until
`destroy()`.

**`expose()` sub-step ordering (audit M4 — pinned).** Correctness (rollback +
no stale-route window) requires a precise order:
1. `register_thing` on **ALL** server bindings (deterministic order). If any
   binding fails, `unregister_thing` the already-registered ones (reverse) and
   return `Err` — the registry is **not yet touched**, so no registry rollback
   needed.
2. All bindings registered OK → **insert into the exposed registry** (CAS/WotLock
   atomic). Now the Thing is dispatchable.
3. `DirectoryPublisher::register` (best-effort; failure is non-fatal/warn).
This eliminates the "routes exist but registry doesn't" window: the registry
entry appears only after all routes are live.

**`destroy()` quiescing (audit defect AD15).** Teardown is more than
routes-first; it defines the fate of every in-flight request:

1. `ServerBinding::unregister_thing` on every binding (routes-first → no **new**
   requests can arrive).
2. Set the ThingSlot `draining` flag. The driving loop honors it: any
   not-yet-dispatched request already in the fan-in channel (or accepted via
   `try_accept`) that targets this Thing is **rejected** — request/response
   gets a synthesized "Thing gone" error reply (status-mapped via
   `error_status`, 410-style); streaming/events are dropped.
3. **In-flight handlers already executing are allowed to complete** (they hold
   a handler `Arc` cloned out before draining); their results are **discarded**
   if the Thing is already removed (the response goes nowhere). Async handlers
   are not cancelled mid-`.await`.
4. Once no in-flight dispatch remains (quiesce point), remove the registry
   entry.
5. `DirectoryPublisher::unregister` (best-effort).

`destroy(own_id)` from within the Thing's own handler is the special case: the
in-flight handler is step 3 itself, so removal is **deferred** until it returns
(v3.0 §7 deferred-removal rule, retained).

The dynamic-affordance network propagation (addendum §9.2), directory
re-publish-on-mutation, and `register_affordance`/`unregister_affordance` are
all removed.

**`discover()` sync/async boundary (audit defect AD10).**
`Servient::discover(&self, filter) -> ThingDiscoveryProcess` is **synchronous
and returns immediately**, and so is `Discoverer::discover()` — both are sync
entry points. The `ThingDiscoveryProcess` is lazy: it stashes the reader +
query (`Pending`), and the real async work (`DirectoryReader::open_search().await`
+ Introduction/Exploration) happens in the **first `next()`** on the process
(which is async; `Pending`→`Open` on first call). No network/directory work at
construction (matches the WoT Scripting API `discover()` → lazy `ThingDiscovery`
model). `Discoverer::request_thing_description()` stays async (a concrete TD
fetch IS a network round-trip).

### 7.4 ConsumedThing — real async

```rust
impl ConsumedThingHandle {
    pub async fn read_property(&self, name, options) -> CoreResult<InteractionOutput>;
    pub async fn write_property(&self, name, value, options) -> ...;
    pub async fn invoke_action(&self, name, params, options) -> ...;
    pub async fn query_action(&self, name, options) -> CoreResult<InteractionOutput>;   // queryaction (E14)
    pub async fn cancel_action(&self, name, options) -> CoreResult<InteractionOutput>;  // cancelaction (E14)
    pub async fn observe_property(&self, name, options) -> CoreResult<Subscription>;
    pub async fn unobserve_property(&self, name, sub) -> ...;
    pub async fn subscribe_event(&self, name, options) -> CoreResult<Subscription>;
    pub async fn unsubscribe_event(&self, name, sub) -> ...;
    // bulk: read_all_properties, write_all_properties, read_multiple_properties,
    //       write_multiple_properties, subscribe_all_events, unsubscribe_all_events
}
```

The consumer surface is **symmetric with the 9-op producer model** (audit E14):
`query_action` / `cancel_action` are first-class consumer methods (TD 1.1
`queryaction`/`cancelaction` are first-class ops), matching the producer's
`ActionQueryHandler`/`ActionCancelHandler`. All methods drive the real async
`ClientBinding`. The fake "delegates to sync" consumer surface (M8) is removed.
Bulk operations prefer a Thing-level meta-operation form when the TD advertises
one (W3C TD §6.3.3), otherwise fan out (behavior retained from PLAN C6).
**Bulk fan-out concurrency (audit round-2 P-3/AD52):** when fanning out, `std`
builds drive the per-property `invoke`s through a **bounded**
`futures::stream::iter(..).map(invoke).buffer_unordered(bound)` (one
`async_trait` `Box` per property, bounded to avoid unbounded allocation/N-way
network storms); `no_std` builds fan out **serially** (no concurrent network).
The default bound is the property count; a configurable bound is a deferred
follow-up. **Bulk reads honor `readOnly`/`writeOnly`** (audit E24): `read_all`/`read_multiple`
exclude `writeOnly` properties; `write_all`/`write_multiple` exclude `readOnly`.
Partial-failure semantics (AD26) hold under both: one property's `Err` does not
fail the batch.

**Async action completion — v1 scope (audit E15).** v1 supports **synchronous
actions only**: `invoke_action` awaits the handler and returns its result in the
`InteractionOutput` (`InteractionStatus::Ok`). The async-action completion model
(HTTP/CoAP 202 `Accepted` + later result retrieval via poll/observe-action-state)
is **deferred** — `InteractionStatus::Accepted` is reserved for that future
model but no result-retrieval/subscription mechanism is defined in v1. This is a
declared v1 scope boundary (not a §9 Scripting-API deviation; it is a
feature-completeness gap recorded here and in `deferred-design-followups.md`).

### 7.5 Security and credentials

Retained: `SecurityProvider` (with `verify` for inbound, `apply` for outbound),
`Principal`/`PrincipalId`, `CredentialStore`/`InMemoryCredentialStore`,
inbound `AuthMaterial` extraction. The `apply_security` post-apply diff is
replaced by having `apply` return the metadata it added (deferred #4).

**Combo schemes (audit E5).** TD 1.1 `ComboSecurityScheme` (`security`/`compose`
— AND/OR of sub-schemes) is **not decomposed by the engine in v1**: a
`SecurityProvider` returns `UnsupportedScheme` for a combo scheme. v1 supports
the basic schemes only. A future `ComboSecurityProvider` will decompose AND
(all sub-schemes must `apply`/`verify`) and OR (any) — tracked as a follow-up,
not a §9 deviation (it is a scheme-coverage gap, recorded here).

## 8. Feature Policy

| Feature | Effect |
|---|---|
| `default = ["std"]` | std runtime + tokio convenience. |
| `alloc` | dynamic data on `no_std`. |
| `std` | networking, filesystem, async runtime, host convenience (`serve` loop, idle backoff). **Implies `alloc` + `async`** (`Cargo.toml`: `std = ["alloc", "async"]` — audit M7; `--features std` always enables async driving + `FanInSender`). |
| `async` | native-async driving (always on for `std`; required for the canonical model). On `no_std`, driving is manual-poll by default and native-async suspension requires an executor (embassy). |
| `zenoh` | Rust `zenoh` std backend (real async consume + inbound). |
| `zenoh-pico` | constrained `no_std+alloc` platform-hook backend (mutually exclusive with `zenoh`). |
| `td2-preview` | experimental TD 2.0 fields. |

The `multithread` feature is **removed** — the lock primitive is always
thread-safe.

## 9. Documented Deviations from the Scripting API

These are the minimum deviations required for `no_std + alloc` and are
documented, not hidden. This section lists **behavioral / semantic** deviations
only — naming and Rust-idiom choices for engine-internal types are governed by
the §0 "Naming and idiom posture" and are not enumerated here:

1. **Subscription delivery is a pull queue, not a push callback.** A
   `ConsumedThingHandle::subscribe_event` returns a `Subscription` drained by
   `poll_next` (sync) or a `Stream` impl (async). Rationale: a callback fired
   from inside the protocol poll can self-deadlock or block the super-loop on a
   bare MCU; decoupling arrival from handling is the safe model. The semantic
   contract (the subscriber eventually observes the event) is preserved.
2. **Errors are `Result`, not thrown exceptions.** Rust idiom.
3. **`fetchTD` / directory exploration are trait objects (`Discoverer`),** not a
   built-in `fetch` — the engine is protocol-neutral and the concrete transport
   is injected.
4. **No implicit server-side property value store (audit E2).** The engine is
   **handler-driven**: `ExposedThing` is "Thing + handler set", with no
   internal property-value map. `read_property` dispatches to the read handler;
   an affordance with no read handler returns `MissingHandler`. The Scripting
   API's `ExposedThing` keeps an internal value (readable without a handler,
   set by `writeProperty`/initial TD) — clinkz-wot does **not** replicate that.
   Rationale: a handler-driven model is unambiguous (no value/handler race),
   zero-extra-state, and matches the device/gateway use case. Applications
   wanting value-store semantics implement a read handler backed by their own
   state.
5. **`DiscoveryFilter` replaces `ThingFilter` (audit E9).** The Scripting API
   `discover(filter: ThingFilter)` (with `method` enum + `query`) is replaced by
   `Servient::discover(filter: DiscoveryFilter)` (P1 §1.9). The
   `DiscoveryMethod`/`ThingFilter.query` vocabulary is folded into
   `DiscoveryFilter` + `DirectoryFilter`; remote `Directory`/`Multicast` methods
   are v1-unsupported (see §6 / E6).

No other behavioral deviations are permitted without an explicit entry here
(naming/idiom choices for engine-internal types are governed by the §0
posture, not this section).

## 10. Scripting API Conformance Map

| Scripting API | clinkz-wot surface | Notes |
|---|---|---|
| `WoT.produce(td)` | `Servient::produce(td)` | returns `ExposedThingHandle` |
| `WoT.consume(td)` | `Servient::consume(td)` | returns `ConsumedThingHandle` |
| `WoT.discover(filter)` | `Servient::discover(filter)` | returns `ThingDiscoveryProcess` (lazy session) |
| `WoT.fetchTD(url)` | `Servient::fetch_td(url)` | async; **direct fetch, does not follow `ThingLink`** (audit E21 — link-following is a separate `ThingLinkResolver` path, §6) |
| `ExposedThing.setPropertyReadHandler` | `ExposedThingHandle::set_property_read_handler` | |
| `ExposedThing.setPropertyWriteHandler` | `set_property_write_handler` | |
| `ExposedThing.setPropertyObserveHandler` | `set_property_observe_handler` | |
| (property unobserve) | `set_property_unobserve_handler` | TD §5.3.4.2 op |
| `ExposedThing.setActionHandler` | `set_action_handler` | invoke op |
| (action query) | `set_action_query_handler` | `queryaction` op |
| (action cancel) | `set_action_cancel_handler` | `cancelaction` op |
| `ExposedThing.setEventSubscribeHandler` | `set_event_subscribe_handler` | |
| (event unsubscribe) | `set_event_unsubscribe_handler` | TD §5.3.4.2 op |
| `ExposedThing.readProperty`/`writeProperty` | `read_property`/`write_property` (server-side local) | |
| `ExposedThing.emitEvent`/`emitPropertyChange` | `emit_event`/`emit_property_change` | |
| `ExposedThing.expose()`/`destroy()` | `expose()`/`destroy()` | TD frozen after expose |
| `ConsumedThing.readProperty` | `read_property(name, options)` | async, real binding |
| `ConsumedThing.writeProperty` | `write_property` | |
| `ConsumedThing.invokeAction` | `invoke_action` | |
| (action query) | `query_action` | `queryaction` consumer op (E14) |
| (action cancel) | `cancel_action` | `cancelaction` consumer op (E14) |
| `ConsumedThing.observeProperty`/`unobserveProperty` | `observe_property`/`unobserve_property` | returns `Subscription` (deviation §9.1) |
| `ConsumedThing.subscribeEvent`/`unsubscribeEvent` | `subscribe_event`/`unsubscribe_event` | returns `Subscription` (deviation §9.1) |
| `ConsumedThing.readAllProperties`/`writeAllProperties`/`readMultipleProperties`/`writeMultipleProperties`/`subscribeAllEvents`/`unsubscribeAllEvents` | bulk methods | retained from PLAN C6; honor `readOnly`/`writeOnly` (E24) |
| `ThingDiscovery.start/next/stop` | `ThingDiscoveryProcess` (async session) | lazy, continuation-based; `start()` folded into first `next()` (AD10, E19) |

## 11. Performance Targets

The per-interaction hot path must be allocation-light and lock-bounded:

- **Affordance addressing** uses `Arc<str>` (already done, retained).
- **Handler invocation** clones one `Arc<dyn Handler>` from a per-Thing
  handler-set **snapshot** (`Arc<HandlerSet>`, lock-free atomic load — audit
  defect 2), then invokes. The primary sync handler path is a direct virtual
  call — **zero per-interaction heap allocation**. The opt-in async handler
  path pays one `async_trait` `Box` per call (acceptable: the handler is
  I/O-bound).
- **Inbound accept** is a single **bounded** fan-in channel on std (O(1)
  `recv`, zero boxing; binding enqueues via sync `try_send` from zenoh
  callbacks — they cannot `await`; on `Full` request/response is rejected with
  an explicit error reply, streaming/events drop-oldest + overflow — no
  binding-internal queue, AD6a/AD9) and a sync `try_accept` poll on no_std (one
  request per tick, rotation cursor, O(N_bindings), no boxing — AD6b). No
  `select_all`, no per-binding boxed `poll_accept` future (audit defect AD1).
- **Registry / handler-table / subscription-state reads** are per-build (AD2/C1):
  std = lock-free `arc_swap` snapshot load; no_std = `WotLock` + clone-out
  (`with_read(|m| m.get(&id).cloned())` — brief CS ~500ns, handler invocation
  outside the lock). No `WotLock::with_read` covering handler invocation on any
  build. std registry snapshots are `Arc<im::HashMap<…>>` (O(1) amortized insert,
  O(1) publish); no_std uses `BTreeMap` (O(log n) get, no snapshot publish).
- **Outbound form/binding plan** is interned in the consumed registry entry
  (addendum §9.4 retained); repeated consumed interactions reuse the cached
  binding instance via `Arc` clone — no `make_binding`, no plan recompute.
- **Event fan-out** shares `Payload` bytes via `Arc<[u8]>` (retained); media
  metadata may move to `Arc<str>` if profiling warrants (deferred #1).
- **Lock contention** is bounded: `WotLock` is reserved for read-write-frequent
  / exclusive-semantics state; read-heavy-rare-write state uses snapshots.
- **Directory queries** are continuation-based (one batch + token), `Live`
  consistency only in v1, not full-table scan with `offset+total` (discovery
  refactor; audit defect 3).

**Performance acceptance level (audit E8).** v4.0's performance claims are
**design-level and structurally verified**, not benchmark-gated: the zero-alloc
inbound hot path and O(1) fan-in are guaranteed by the architecture (sync
handler dispatch, `Arc`-snapshot reads, bounded single fan-in channel), and P4
verifies them by **code review + allocation-shape audit**, not by a `criterion`
suite. A quantified regression benchmark is a **deferred follow-up** (recorded
in `docs/deferred-design-followups.md`), added once the P0–P3 code lands and a
representative workload exists. P4 exit does **not** require a numeric
threshold.

## 12. Sequencing

The refactor is sequenced for **target-crate isolation through P2, workspace
whole at P3** (audit defect AD17 — unifies with `PLAN.md` §Dependency shape;
the earlier "keep the workspace compiling at each phase" wording was wrong
because P0 rewrites core's public surface and breaks core's dependents until
they adapt):

- **P0 — Core interaction surface rewrite.** Sync-primary handler trait set
  with opt-in async twins; consolidated handler storage; concrete
  `ExposedThing` / `ConsumedThing`; `WotLock`; `InteractionOptions`/
  `InteractionOutput` rework. `no_std+alloc` verified.
- **P1 — Discovery rewrite.** Introduction/Exploration/session traits; in-memory
  backend as reference impl; `Discoverer`/`DirectoryPublisher`/
  `DirectoryWatch`. Execute the discovery refactor plan.
- **P2 — Binding async.** Real async `ClientBinding::invoke`; zenoh
  `ZenohSessionTransport` async consume; remove dynamic-affordance API.
- **P3 — Servient rewire.** Drop `Servient<D>`; async-only driving
  (`poll_serve`/`serve`/`poll_serve_once`); frozen-TD lifecycle; real async
  `ConsumedThingHandle`; remove `add_*`/`remove_*` and the sync driving
  modules.
- **P4 — Compliance and verification.** Scripting API conformance map tests;
  feature-matrix checks; `check-no-std.sh`; fixtures; Clippy. Update
  `PLAN.md`, `docs/technical-spec.md`, `docs/wot-compliance.md`,
  `docs/no-std-embedded.md`, `docs/verification.md`.

Each phase is independently shippable behind the workspace build.

## 13. What This Supersedes

- `docs/baseline/servient-design-baseline.md` (v3.0) — retained as historical
  record; v4.0 inherits §1 roles, §5 storage ownership, §7 two-level locking,
  §8 security, §9 subscription flow, §10 expose/destroy coordination, §11
  inbound request shape. v4.0 reverses the async/sync duality (§4) and the
  dynamic-affordance surface (§9.2 of the addendum).
- `docs/baseline/servient-design-baseline-addendum.md` (v3.1) — retained as
  historical record; v4.0 inherits §1 concrete types, §2 owned request model,
  §3 directory-invalidation trigger, §5 error taxonomy. v4.0 reverses §9.1
  sync-lock / §9.2 dynamic affordance / §9.3 Send-bound divergence.
- `docs/wot-compliance.md` §Scripting API Boundary — the "Native Runtime, not a
  Scripting API UA" positioning is reversed. The subscription-deviation note
  is preserved as §9.1 here.
- `docs/no-std-embedded.md` MCU three-layer plan — Layer 1 (`multithread`
  feature) is superseded by the unified lock primitive; Layer 2 (zenoh-pico)
  and Layer 3 (embassy) boundaries are retained.

## 14. Decision Index

| Decision | Topic | Resolution |
|---|---|---|
| D1 | Scripting API alignment | Full Consumer/Producer/Discovery UA conformance target. (§0) |
| D2 | Dynamic affordance lifecycle | Removed in v1; TD frozen at expose. (§4.5, §7.3) |
| D3 | Async/sync model | Async driving/transport layer; sync handlers primary (zero-alloc hot path) with opt-in async handlers (feature/cloud); sync driving is a manual-poll super-loop adapter. (§4.2, §7.2) |
| D4 | Lock primitive | `WotLock<T>`: `Arc`-backed portable handle, `std::sync` / `critical_section`; renames `MapLock`; `multithread` feature removed. (§4.7) |
| D5 | Thing abstractions | Concrete `ExposedThing`/`ConsumedThing`; single-impl traits removed. (§4.1) |
| D6 | Handler storage | One consolidated handler-set per affordance; sync traits primary, async twins (all 9 ops) opt-in per Scripting API method. (§4.2) |
| D7 | Discovery | Execute the Introduction/Exploration/session refactor; `Servient` holds `Discoverer` trait object. (§6, §7.1) |

### Audit defect resolutions (locked)

| Defect | Topic | Resolution |
|---|---|---|
| AD1 | Inbound accept fan-in | Drop boxed `poll_accept` + `select_all`; fan-in channel (std, O(1)) + sync `try_accept` (no_std, O(N_bindings), no boxing). (§4.5, §7.2) |
| AD6a | Unbounded accept buffer | Single **bounded** fan-in channel (capacity configurable); std bindings enqueue from **synchronous** zenoh callbacks via `try_send` (callbacks cannot `await`); on `Full` the policy is split by interaction kind (AD9); **no binding-internal queue**, no async-bridge task. (§4.5, §7.2) |
| AD6b | `poll_serve_once` step semantics | Strict bounded step: at most ONE inbound request **accepted** per `poll_serve`/`poll_serve_once` call; no backlog drain (no_std `if let … break`, not `while let`). AD6b bounds the **accept rate**, not completion concurrency (O1/AD42/H4: std/embassy use `FuturesUnordered` + `max_inflight` cap; bare `no_std` is strictly serial). (§7.2) |
| AD7 | no_std poll-loop fairness | Restore a lightweight `AtomicUsize` rotation cursor for the no_std `try_accept` poll loop (the old "select_all-inherent fairness" rationale died with `select_all`); start offset advances each tick. (§4.5, §7.2) |
| AD8 | produce/expose registry insertion | `produce()` creates a draft handle only (no registry insert); `expose()` is the SINGLE insertion into the servable exposed registry. Closes the lifecycle state machine: draft → exposed → removed. (§7.3) |
| AD9 | Overload policy for request/reply | On fan-in `Full`: request/response interactions are **rejected with an explicit error reply** (mapped via `error_status`, immediate client feedback); only streaming/events use drop-oldest + overflow. No silent drop/timeout as the request/reply default. (§4.5, §11) |
| AD10 | `discover()` sync/async boundary | `Servient::discover()` AND `Discoverer::discover()` are both **sync**, returning a lazy `ThingDiscoveryProcess`; the async `DirectoryReader::open_search()` is deferred to the first async `next()`. No network/directory work at construction. `request_thing_description()` stays async (real network fetch). (§6, §7.3) |
| AD11 | `AbsoluteUri` exposure | td re-exports `AbsoluteUri` at its crate root as a hard P1 prerequisite (it was a P1 open question; P1's independent-compile promise rested on it). (§3) |
| AD12 | Dynamic affordance surface removed from code | The `register_affordance`/`unregister_affordance` binding trait methods, the `ExposedThingHandle::{add,remove}_{property,action,event}` methods, their Servient propagation (`sync_added/sync_removed_affordance`), the zenoh per-affordance impls, and the dedicated tests are **deleted from the current code** (not just docs), closing the code↔baseline divergence. Workspace `cargo check --all-targets` and `cargo test --workspace` pass. |
| AD13 | Fan-in sender injection formalized | The std fan-in `Sender` injection is a **trait method** `ServerBinding::set_request_sink(sender)` (std-gated), called by the Servient at registration — not prose-only "the binding receives a Sender clone". The driving layer drains; it does not own the overload policy (that stays the binding's AD9 contract). (§4.5, §7.2) |
| AD14 | Handler lifecycle vs TD freeze | The TD **affordance set** is frozen at `expose()` (decision 2), but **handlers may be attached/replaced throughout the exposed lifetime** (Scripting API aligned). `MissingHandler` is the designed-in semantic for an exposed-but-unwired affordance. Resolves the baseline-vs-P3 conflict. (§7.3) |
| AD15 | `destroy()` quiescing | Teardown = routes-first + `draining` flag (pending requests rejected: request/reply → "Thing gone" error, streaming dropped) + in-flight handlers allowed to complete (results discarded) + entry removed at quiesce + unpublish. Self-`destroy` from a handler = deferred removal. (§7.3) |
| AD16 | no_std driving = compile-time architecture only | P3's no_std path is compile-only in v1; runtime validation is gated on zenoh-pico (P2 §2.7). P3 depends on the `try_accept` trait *shape*, not on pico's server-side runtime being finalized. (§7.2, P3 §3.12) |
| AD17 | Phase compile boundary | P0–P2 are target-crate isolation (each target crate compiles/tests alone); the workspace is made whole at P3. Unifies baseline §12 with `PLAN.md`. |
| AD18 | `ProjectionMode` vs `ThingDiscoveryProcess` | `ThingDiscoveryProcess` (Scripting-API surface yielding full `Thing`s) **forces `FullThingDescription`**; `IdOnly`/`Summary` are confined to the lower-level `DirectorySession`/`DirectoryItem` API (directory-admin use) and do not flow into the Scripting process. (`docs/plan/phase-p1-discovery.md` §1.4/§1.6) |
| AD19 | `ServerBinding` trait surface completeness | The trait carries **all** load-bearing methods: `try_accept` (default `None` — std-only bindings self-push and never have it called), `send_response` (the reply path — required by AD9 overload error replies; `InboundRequest` has no reply handle), `set_event_broker` (EventBroker injection, default no-op), `set_request_sink` (std, AD13), `register_thing`/`unregister_thing`. The earlier §4.5 snippet omitted `send_response`/`set_event_broker`; both are retained from the current code. **M1:** the binding stores zenoh `Query` reply-handles in a `CorrelationId`-keyed map for deferred `send_response` (current code `server.rs:128/400`; zenoh 1.x natively supports this). (§4.5) |
| AD6c | no_std verification overclaim | `check-no-std.sh` is compile-only; runtime no_std driving deferred with zenoh-pico. (§7.2, `docs/plan/phase-p3-servient.md`, `phase-p4-compliance.md`) |
| AD2 | `WotLock` no_std read degradation | **std**: registry uses lock-free `arc_swap::ArcSwap<Arc<im::HashMap>>` snapshot reads (M2: HashMap for exact-key lookup, not OrdMap). **no_std**: registry uses `WotLock<BTreeMap>` + clone-out dispatch discipline (CS covers only the BTreeMap::get + Arc clone, ~500ns; handler invocation is outside any lock). AD2's intent — "don't hold the lock during handler invocation" — is satisfied on both builds; "zero interrupt-disable on no_std" is **withdrawn** (C1/AD54). (§4.7, §11) |
| AD3 | `SessionStable` snapshot cost | v1 ships `ConsistencyMode::Live` only; `SessionStable` deferred (`#[non_exhaustive]`). (§6) |
| AD4 | Async handler coverage | Async twins for ALL 9 interaction operations, not just read/write/invoke. (§4.2) |
| AD5 | Conservative compliance matrix | P4 build-checks all valid feature combinations per crate; tests a representative subset. (`docs/plan/phase-p4-compliance.md`) |
| AD20 | Driving primitive feature matrix + `FanInSender` | `poll_serve_once` (sync) on every build; `poll_serve`/`serve` (async) gated behind `async` + need an executor (tokio/embassy) — bare `no_std` exposes only `poll_serve_once`. `FanInSender<T>` = core std-only alias for `async_channel::Sender<T>` (runtime-neutral; sync `try_send`). Outbound timeout is build-time cfg (AD39/H2), not a trait. (§4.5, §4.6, §7.2) |
| AD21 | Interaction I/O naming consistency | Payload field is `data` and URI-template vars are `uri_variables` across `InteractionInput`/`Options`/`Output`; `InteractionStatus { Ok, Created, Accepted }` (`#[non_exhaustive]`). (§4.3) |
| AD22 | `ThingDiscoveryProcess` struct + discover error bridging | `{ inner: Box<dyn DiscoverySession> }` where concrete inner is `ProcessState { Pending, Open(DirectorySession), Done(err) }` implementing `DiscoverySession`. Infallible `Servient::discover()` bridges a fallible `Discoverer::discover()` by constructing `Done(err)`. **H5 correction:** v1 Introduction is trivially resolved in `discover()` (local endpoint = reader, no async Introduction); `next()` does Exploration only (`open_search`). The earlier "Introduction deferred to next()" was aspirational for future remote Introduction; deferred with E6. (§6, `phase-p1-discovery.md` §1.6/§1.9) |
| AD23 | td cleanup owned by P0 | The Tier-0 td cleanups (data_type split, Form dedup, validation helpers, AbsoluteUri root re-export) are assigned to P0 (Step 0.0), closing the phase-ownership hole. (§3, §12, `phase-p0-core-interaction.md` §0.0) |
| AD24 | Trait sealing | Extension-point traits (bindings, handlers, codecs, security, discovery reader/publisher/resolver) NOT sealed; engine-internal traits (`DiscoverySession`, `DirectorySession`, `EventSink`, `InboundDispatcher`, `*HandlerSet`, `ProcessState`) sealed/`pub(crate)`. (§4.8) |
| AD25 | Cross-crate error interop | `From<BindingError> for CoreError`; `From<{CoreError,BindingError,DiscoveryError}> for ServientError`; `error_status(&CoreError)` is the single protocol-status source (binding errors flow through CoreError; DiscoveryError is app-layer via the process, not a status). Inward-only direction. **`fetch_td` therefore returns `ServientResult<Thing>`** (audit round-2 C2/AD37): `From<DiscoveryError> for CoreError` is layering-blocked (core does not depend on discovery), so the Servient-level conversion is the only legal one. (§5.1, §7.3) |
| AD26 | Bulk operation partial-failure | `readAll`/`readMultiple`/`writeAll`/`writeMultiple` return `BTreeMap<PropertyName, Result<InteractionOutput, CoreError>>`; `subscribeAll`/`unsubscribeAll` return per-event `Result<Subscription, _>`. One property's error does NOT fail the batch (Scripting-API aligned). (§7.4, P3 §3.6) |
| AD27 | `expose()` rollback + `destroy()` idempotency | `expose()` registers bindings in order; on binding `k+1` failure it `unregister_thing`s the succeeded `1..k` (reverse), rolls back the registry insert, returns fatal `Err` (E12). Requires `register_thing` to return `Result<(), CoreError>` (audit round-2 C3/AD38). `destroy()` is idempotent — on an already-removed/never-exposed Thing it no-ops returning `Ok`; concurrent destroys are safe via CAS (std) / WotLock exclusive (no_std) (E13/H6). (P3 §3.4) |
| AD28 | Consumer 9-op symmetry | `ConsumedThingHandle` has `query_action`/`cancel_action` matching the producer's `ActionQueryHandler`/`ActionCancelHandler` — TD 1.1 `queryaction`/`cancelaction` are first-class on both sides. (§7.4, §10) |
| AD29 | Async-action completion — v1 scope | v1 = synchronous actions only (`invoke_action` awaits + returns `Ok`); the 202 `Accepted` + result-retrieval/observe-action model is deferred (`InteractionStatus::Accepted` reserved). Declared scope boundary, not a §9 deviation. (§7.4) |
| AD30 | Handler panic safety (G1) | **std-only panic→reply; no_std lock-integrity only** (audit round-2 C1). `std`: `InboundDispatcher` wraps every handler invocation in `catch_unwind`; a panic becomes `CoreError::HandlerPanic { target, operation }` → 5xx reply, the request fails cleanly, the `serve` loop keeps running. `no_std`: `catch_unwind` unavailable (bare metal is `panic=abort`); the engine guarantees only **lock integrity** (handler `Arc` cloned before invoke; `with_recover` heals locks) — the panic itself goes to the platform panic handler. Locks stay unpoisoned on every build. (§4.2) |
| AD31 | Global shutdown quiescing (G2) | `shutdown` flag checked between `poll_serve` steps: the in-flight request completes (handler awaited, not cancelled); queued fan-in requests are dropped on `Servient` drop (not drained — full drain could block shutdown). Per-Thing polite teardown is `destroy()` (AD15). (§7.2) |
| AD32 | Byte-level handler encoding (G3) | Handlers are byte-level on both sides (`InteractionInput/Output.data: Option<Payload>`, body `Arc<[u8]>`); the runtime does not auto-encode logical values. `PayloadCodecRegistry` applies at the transport edge (wire↔Payload). **`InteractionInput.accept: Option<AcceptHint>`** lets a byte-level handler pick a matching output content type and avoid a mismatch-driven double codec (audit round-2 O7/AD48). Companion to §9 deviation #4. (§4.3) |
| AD33 (G5/H6) | `ThingId` uniqueness + atomic check-and-insert | Registries key by `ThingId` (= TD's `id`). Duplicate `expose` rejected (`AlreadyExposed`); duplicate `consume` reuses. **Atomicity (H6):** std = CAS loop on ArcSwap (load→check→build→compare_and_swap→retry); no_std = WotLock exclusive `with` (check+insert atomic). The earlier "registry lock" wording was std-inaccurate; corrected to per-build mechanism. (§7.3) |
| AD34 | Binding trait `Send + Sync` (G4) | `ServerBinding: Send + Sync` and `ClientBinding` trait objects are `Send + Sync` so the `serve` future is `Send` and spawnable on tokio/embassy. (§4.5) |
| AD35 | `ServientBuilder` API shape (G6) | Move-fluent consuming builder (`with_*` → `build()`); required ≥1 server binding + ≥1 client factory; omitted discoverer defaults to `LocalDiscoverer`; `build()` wires `set_event_broker`/`set_request_sink` into every binding. **`with_fanin_capacity(usize)`** configures the bounded inbound fan-in channel capacity (audit round-2 O5/AD46) — the AD6a "configurable capacity" had no setter before. (P3 §3.11) |

### Audit round-2 defect resolutions (locked)

These resolve the second design-audit pass (contradictions, omissions, and
performance-bottleneck findings against the v4.0 baseline and phase plans).
Each is a locked amendment; the affected body sections and existing AD entries
above carry cross-references to the matching AD36–AD53 entry.

| Defect | Topic | Resolution |
|---|---|---|
| AD36 (C1) | `catch_unwind` vs `no_std` | `catch_unwind` is std-only; AD30 rescoped to a **std-only** panic→reply contract. `no_std` gets **lock-integrity only** (`with_recover` + handler `Arc` cloned pre-invoke); the panic itself goes to the platform panic handler (`panic=abort` ⇒ reset). (§4.2, AD30) |
| AD37 (C2) | `fetch_td` error chain | `fetch_td` returns **`ServientResult<Thing>`**. `From<DiscoveryError> for CoreError` is layering-blocked (core ↛ discovery), so the Servient-level conversion is the only legal one; AD25 unchanged. (§7.3, AD25) |
| AD38 (C3) | `register_thing` return type | `register_thing(&self, ..) -> Result<(), CoreError>` — required so `expose()` rollback (E12/AD27) can detect binding `k+1` failure. `unregister_thing` stays `()` (idempotent/best-effort teardown). (§4.5, AD27) |
| AD39 (C4/H2) | Outbound timeout — build-time cfg, not trait | **No `OutboundTimeout` trait** (H2: generic-method trait is not object-safe; `Arc<dyn OutboundTimeout>` was invalid). Timeout is **build-time cfg** in the Servient outbound path: std = `tokio::time::timeout`; no_std+embassy = `embassy_time::with_timeout`; bare no_std = fail-closed `Err(TimeoutUnsupported)`. (§4.6, AD45) |
| AD40 (C5/E2) | `poll_serve_once` on bare no_std | Dual implementation by feature: no_std+async **stores a pinned reusable `poll_serve` future** (created once, polled each tick — E2: NOT recreated per call, which would drop the `recv().await` Pending state); **bare no_std runs a purely synchronous** accept→dispatch→reply step (no async future exists there). (§7.2) |
| AD41 (C6) | Lock-free snapshot atomic primitive (std-only) | On **std**, the concrete primitive is `arc_swap::ArcSwap<Arc<im::HashMap>>` (M2: HashMap for exact-key lookup; genuinely lock-free reads). On **no_std**, `arc-swap` and `im` are NOT available (C1/AD54); the registry uses `WotLock<BTreeMap>` + clone-out instead — no snapshot primitive needed. (§4.7, AD2) |
| AD42 (O1/H4) | `FuturesUnordered` + `max_inflight` cap | AD6b bounds **accept** (≤1/step), not **completion concurrency**. std/embassy use `FuturesUnordered` for cross-Thing async concurrency — but it is **inherently unbounded** (H4: "bounded FuturesUnordered" was a false claim). The concrete bound is **`max_inflight`**: before accepting, the loop checks `in_flight < max_inflight`; at capacity, poll-only → fan-in fills → AD9 backpressure. Default configurable (e.g., 64). bare no_std is strictly serial (no executor, no `FuturesUnordered`). (§7.2, AD6b/AD9) |
| AD43 (O2) | `verify` non-blocking | The non-blocking rule extends from handlers to the sync `SecurityProvider::verify` on the inbound hot path. `AsyncSecurityProvider` deferred. (§4.2, §7.5, `deferred-design-followups.md`) |
| AD44 (O3/H3) | Live-session cursor model | In-memory `Live` session uses a **sorted-id cursor** (last-emitted id in BTreeMap key order), NOT a revision high-water-mark (H3: revision cursor re-emitted updated items, violating Live Semantics rule 4). Each `next()` reads `id > cursor` under a brief shared lock, advances cursor. Updates to already-emitted ids do NOT re-emit (go to `DirectoryWatch`). O(1) cursor memory. (P1 §1.5/§1.10) |
| AD45 (O4) | timeout silent no-op on no_std | A set `timeout` on bare no_std (no timer) returns **`Err(CoreError::TimeoutUnsupported)`** (fail-closed) — never silently ignored. On std/embassy the timeout is enforced via build-time cfg (AD39/H2). (§4.6, AD39) |
| AD46 (O5) | Fan-in capacity setter | `ServientBuilder::with_fanin_capacity(usize)` added; AD6a's "configurable capacity" now has a setter. (AD35, P3 §3.11) |
| AD47 (O6) | `form_index` priority | `InteractionOptions.form_index` is the **highest-priority** selection key (bypasses `supports`; caller takes responsibility); an unsupported pinned form → `CoreError::UnsupportedForm`. (§5.1, E20) |
| AD48 (O7/E1) | Byte-level content negotiation | `InteractionInput.accept: Option<AcceptHint>` lets a byte-level handler pick a client-acceptable output content type. **E1 correction:** if the handler emits a mismatched type, the edge returns `CoreError::ContentTypeMismatch` — it does **NOT** transcode (the engine has no value type per deviation #4/AD32). Transcoding is an explicit application-layer codec, not an engine fallback. (§4.3, AD32) |
| AD49 (S1) | `DirectoryPatch` JSON coupling | `DirectoryPatch` is **`{ body: Vec<u8>, content_type: MediaType }`** (protocol-neutral bytes), not `pub serde_json::Value`; serialization to JSON/CBOR moves to the backend. Keeps the no_std discovery root JSON-free. (P1 §1.4) |
| AD50 (P-1/M2) | Registry container per build | **std**: `Arc<im::HashMap>` (HAMT structural sharing: O(1) amortized insert, O(1) publish — M2: HashMap for exact-key lookup, not OrdMap). **no_std**: `WotLock<BTreeMap>` (no snapshot; O(log n) get under brief CS — for ESP32-class gateways; no external deps). (§4.7, §11, AD2) |
| AD51 (P-2/H1) | Handler-set swap granularity — unified | **Single model**: `Map<Name, Arc<HandlerSet>>`. A slot swap rebuilds ONE `Arc<HandlerSet>` (one alloc) + one map insert (O(log n)); NOT a per-affordance `ArcSwap` cell. The earlier "via `ArcSwap`" per-affordance wording is withdrawn (H1). Per-slot `ArcSwapOption` remains the std-only escape hatch if hot. (§4.2, §4.7) |
| AD52 (P-3) | Bulk fan-out concurrency | std: bounded `buffer_unordered`; no_std: serial. Default bound = property count; configurable bound deferred. Partial-failure (AD26) holds under both. (§7.4, P3 §3.6) |
| AD53 (P-4) | Directory-driven consumed invalidation cost | std: `im::HashMap` (AD50/M2) → O(1) amortized rebuild. no_std: `WotLock<BTreeMap>` → O(log n) get, O(n) full rebuild on invalidation (rare; acceptable for ESP32-class registries). Churn is coalesced/debounced. (P3 §3.7) |
| **AD54 (C1)** 🔴 | **arc-swap + im NOT stable-`no_std` — no_std uses WotLock+clone-out** | The prior claim "`arc-swap` is `no_std`-compatible" was **false** (`arc-swap` 1.9 `no_std` needs nightly `experimental-thread-local`; `im` is std-oriented). **Final correction:** no_std registry uses **`WotLock<BTreeMap>` + clone-out dispatch** (CS covers only BTreeMap::get + Arc clone ≈ 500ns; handler invocation outside any lock). **Zero external deps on no_std.** std keeps `arc_swap::ArcSwap<Arc<im::HashMap>>` (lock-free; both deps available on std). AD2 intent preserved (no locked invocation); "zero interrupt-disable on no_std reads" **withdrawn**. (§4.7, §11, AD2/AD41/AD50) |
