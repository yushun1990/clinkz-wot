# Phase P2 — Binding Async

> Baseline: `docs/baseline/engine-architecture-baseline.md` (v4.0) §5.

## Goal

Make the protocol binding consume path genuinely async and remove the
dynamic-affordance binding surface. Adapt the shared and zenoh binding crates
to the P0 async `ClientBinding` / `ServerBinding` traits.

P2 leaves **`clinkz-wot-protocol-bindings` and
`clinkz-wot-protocol-bindings-zenoh` compiling and tested in isolation**.

## Entry Criteria

- P0 async `ClientBinding` / `ServerBinding` traits are stable (P0 open question
  resolved to "declare `invoke` async in P0").

## Current State (being changed)

- `protocol-bindings/core/src/binding.rs` → actually `core/src/binding.rs`
  defines `ClientBinding` (sync `invoke`) + `AsyncClientBinding`
  (`invoke_async`) + `as_async_binding()` shim + `SubscriptionGuard`. The
  sync/async split and the fake-async delegation originate here.
- `protocol-bindings/core/src/` (shared): `error.rs`, `error_status.rs`,
  `form.rs`, `uri_template.rs`, `lib.rs` — form selection, op resolution,
  target resolution, security metadata, `BindingError`, HTTP-like status
  mapping.
- `protocol-bindings/protocols/zenoh/src/`: `form.rs`, `zenoh.rs` (planning),
  `server.rs` (1260 lines, `ZenohServerBinding`), `runtime/{zenoh.rs,
  zenoh_pico.rs, selector.rs}`, `runtime.rs`, `error.rs`, `lib.rs`.

## Work Breakdown

### Step 2.1 — Shared binding (`protocol-bindings/core`)

Keep all shared utilities; targeted cleanups:

- Form selection, affordance form lookup, target resolution (`base` + `href`),
  `FormSelectionCriteria`, `AffordanceRef`, security metadata extraction,
  `error_status` mapping — **unchanged**.
- Convert remaining free-form `String` `BindingError` messages to structured
  variants so callers match programmatically (deferred #8). Keep the existing
  structured `UnknownAffordance { kind, name }`.
- `uri_template.rs` percent-encoder (direct `%XX` buffer writes, retained from
  hardening) — unchanged.

### Step 2.2 — Adopt P0 async `ClientBinding`

P0 declares `ClientBinding::invoke` and `subscribe` as `async fn`. The shared
crate consumes the trait; no shared-side change beyond updating any
trait-bound references. The `as_async_binding()` shim and `AsyncClientBinding`
trait (current `core/src/binding.rs`) are gone after P0; P2 removes all
references in the binding crates.

### Step 2.3 — Zenoh planning layer (`form.rs`, `zenoh.rs`, `error.rs`)

**Unchanged** and stays `no_std + alloc`:

- `zenoh://` form recognition, relative-`href`-against-`base` resolution.
- WoT op → zenoh operation-kind mapping.
- `cz-zenoh` extension metadata parsing (qos/priority/congestionControl) as
  experimental hints.
- `ZenohOperationPlan`, `ZenohBinding<T>` generic planning surface.
- Predicate-based form selection integration (`FormSelectionCriteria`).
- Thing-level forms, bulk property/event planning, selector parameter
  validation.

### Step 2.4 — Zenoh runtime: real async consume (`runtime/zenoh.rs`)

`ZenohSessionTransport` (std, `zenoh` feature) becomes genuinely async:

- `async fn invoke(&self, request: BindingRequest) -> CoreResult<InteractionOutput>`
  driving the real `zenoh::Session`:
  - readproperty / invokeaction → `session.get(query).await` (request/reply).
  - writeproperty → `session.put(key, payload).await`.
  - Maps `contentType`/encoding, express QoS, priority, congestion control from
    the cached `ZenohOperationPlan`.
- `async fn subscribe(...)` → `open_subscription` using
  `session.declare_subscriber` with a callback pushing samples into a
  `SubscriptionSender`; returns `(Subscription, Box<dyn SubscriptionGuard>)`.
- `ZenohSubscription` retained for explicit receive + undeclare lifecycle.
- Remove the `as_async_binding()` override and any "delegates to sync" path.

The cached `ZenohOperationPlan` (keyed by `(Arc<Form> pointer, Operation)` with
`Weak<Form>` eviction, from hardening) is retained so the steady-state hot path
skips target resolution and key-expr allocation.

### Step 2.5 — Zenoh server binding (`server.rs`)

`ZenohServerBinding` implements the P0 `ServerBinding` surface. **Main std path:
direct push into the Servient's single bounded fan-in channel — no
binding-internal accept queue** (audit defect AD1 + the unbounded-buffer risk
AD6a). `try_accept` is the **no_std polled path only**.

- readproperty / invokeaction via `declare_queryable`.
- writeproperty via put-listener.
- observeproperty / subscribeevent via publisher on key (`PublisherSink`
  wrapping `session.put`), fed by the `EventBroker`.
- Route planning reuses the `no_std + alloc` planner (inbound direction).
- **std accept (main path):** at registration the Servient calls the trait's
  `set_request_sink(sender)` (AD13) to hand the binding a `FanInSender`
  clone (the Servient fan-in channel tx) — this is a formalized trait method,
  not prose-only coupling. zenoh queryable/put-listener callbacks are **synchronous closures** (`move |query|
  { … }` / `move |sample| { … }` — see current `server.rs:558,601`); they
  **cannot `.await`**. So they build the `InboundRequest` and call
  **`fanin_tx.try_send(req)` synchronously** (matches the existing
  `async_tx.try_send(request)` + `handle_async_enqueue_result` pattern). The
  channel is **bounded** (capacity configurable, sane default); on `Full` the
  binding applies a **policy split by interaction kind** (audit defect AD9 — a
  blanket drop-oldest is wrong for request/reply):
  - **Request/response** (readproperty / writeproperty / invokeaction /
    queryaction): the binding REJECTS with an explicit error reply synthesized
    from the zenoh reply target and mapped via `error_status` (503-style), so
    the client gets **immediate overload feedback** — NOT a silent drop/timeout
    that would unfairly sacrifice the longest-waiting request.
  - **Streaming / fire-and-forget** (events, property observes): drop-oldest +
    overflow counter (latest is most valuable; consistent with the
    `Subscription` model in v4.0 §4.6; observable for diagnostics).
  **No `await` in the callback, no async-bridge task** (a bridge would
  reintroduce the intermediate buffer AD6a removed), **no binding-internal
  queue** — the bounded fan-in channel is the single buffer.
- **Deferred reply-handle storage (audit M1):** the zenoh callback returns
  immediately after `try_send`, but the **zenoh `Query` reply-handle** must
  survive until `send_response` replies. The binding stores it in a
  `CorrelationId`-keyed `reply_targets` map (`ReplyTarget::Query { query,
  key_expr }`) — this IS the current code pattern (`server.rs:128/400`), and
  zenoh 1.x natively supports holding a `Query` and calling `query.reply()` /
  `query.reply_err()` later (the `Query` owns the reply sender). A throttled
  TTL sweep evicts orphaned entries (a request that never got a `send_response`
  because it was dropped at capacity) and replies with a server-busy error so
  the client is not left hanging. **P2 exit requires a smoke test** verifying
  the end-to-end flow: queryable callback → `try_send` → `send_response` →
  `query.reply()` under the `zenoh` feature gate.
- **no_std accept (polled path):** no executor ⇒ no callback push. The binding
  implements `try_accept(&self) -> Option<InboundRequest>` that drains one
  ready request directly from its transport poll. The Servient super-loop
  polls `try_accept` per binding (see P3 §3.5).

### Step 2.6 — Remove dynamic-affordance API

Delete from `ZenohServerBinding` (and any concrete binding):

- `register_affordance(thing_id, target, td)` / `unregister_affordance(thing_id, target)`.
- Per-affordance route tracking (`BTreeMap<thing_id, BTreeMap<affordance_key, routes>>`).
- Per-affordance broker sink register/remove.

Replace with **wholesale** route lifecycle:

- `register_thing(thing_id, td)` — declares all routes for every affordance in
  the TD; registers all event/observable `PublisherSink`s.
- `unregister_thing(thing_id)` — undeclares all routes; removes all sinks via
  `EventBroker::remove_thing`.

This is the v3.0 §10 model (expose registers all routes; destroy unregisters
all), restored. The P0 `ServerBinding` trait carries only the wholesale pair.

### Step 2.7 — zenoh-pico backend (`runtime/zenoh_pico.rs`)

Retained at the platform-hook boundary (`ZenohPicoPlatform` trait,
`ZenohPicoTransport`, `ZenohPicoRequest`). Adopt the async `ClientBinding`
signature (the platform hook returns a future resolved by the platform's
polling model). **The `ServerBinding::try_accept` model for the pico backend
is deferred** (server-side accept API on bare no_std is TBD): its poll-driven
(synchronous-readiness) shape will be specified when the target hardware
platform and C ABI polling model are confirmed (per PLAN.md "Defer zenoh-pico
runtime injection"). Update `scripts/check-reserved-features.sh` expectations
if the feature surface moves.

### Step 2.8 — Feature policy

- `zenoh` (Rust std backend) and `zenoh-pico` (constrained `no_std+alloc`
  platform-hook) remain **mutually exclusive**.
- Planning layer (`form.rs`, `zenoh.rs`, `error.rs`) independent of both —
  always `no_std + alloc`.
- `runtime/zenoh.rs` behind `zenoh`; `runtime/zenoh_pico.rs` behind `zenoh-pico`.
- `ZenohRuntimeTransport` type alias resolves per feature (retained).

### Step 2.9 — Shared zenoh transport handle

`SharedZenohTransport<T>` (std, retained from hardening) lets Servient binding
factories reuse one session across cloned bindings. Keep; verify it hands out
the async `invoke`.

## Resolved Decisions

- **AD1 / A3 (inbound accept = bounded fan-in, not boxed `poll_accept` +
  `select_all`; AD6a unbounded-buffer risk).** Audit defect AD1 overturned the
  earlier "boxed `poll_accept` future + `select_all`" design; the
  unbounded-buffer follow-up (AD6a) removes the binding-internal queue
  entirely. The single source of truth is the **Servient's bounded fan-in
  channel**:
  - **std zenoh backend (main path):** the binding receives a
    `Sender<InboundRequest>` clone at registration and enqueues from its
    **synchronous** zenoh callbacks via **`fanin_tx.try_send(req)`** — zenoh
    callbacks are sync closures (`move |query| { … }`, `server.rs:558,601`) and
    cannot `await`. Bounded capacity ⇒ on `Full` a **policy split by interaction
    kind** (AD9): request/response is **rejected with an explicit error reply**
    (mapped via `error_status`, immediate client feedback — not silent
    drop/timeout); streaming/events use drop-oldest + overflow counter. **No
    `await` in the callback, no async-bridge task (would reintroduce an
    intermediate buffer), no binding-internal queue.** `try_accept` is unused on
    std.
  - **no_std zenoh-pico backend:** `try_accept(&self) -> Option<InboundRequest>`
    drains one ready request directly from the transport poll (no executor, no
    callback push); the Servient super-loop polls it per binding. Model
    **deferred** with the hardware platform.
  No per-binding boxed accept future, no `select_all`, no unbounded buffer.

### Open Questions

1. **Selector validation on the async path.** The current zenoh runtime
   validates request/reply selector parameters. Keep that validation in the
   async `invoke` (fail fast with a structured `BindingError` before
   `session.get`). Confirm.

## Deliverables

- Async `ClientBinding`/`ServerBinding` adoption across both binding crates.
- Real async zenoh consume (read/write/invoke) + async subscription.
- Wholesale route lifecycle (`register_thing`/`unregister_thing`) replacing
  per-affordance tracking.

## Exit Criteria

- `clinkz-wot-protocol-bindings` and `clinkz-wot-protocol-bindings-zenoh` pass
  `cargo test` and `cargo check --no-default-features`.
- Opt-in smoke test passes: `CLINKZ_WOT_RUN_ZENOH_RUNTIME_TESTS=1 cargo test -p
  clinkz-wot-protocol-bindings-zenoh --features zenoh` covering real async
  put/get/subscribe **and** the deferred-reply-handle flow (audit M1:
  queryable callback → `try_send` → `send_response` → `query.reply()`).
- No `register_affordance`/`unregister_affordance`/`as_async_binding`/
  `AsyncClientBinding` references remain.

## Risks

- Zenoh API drift: the `zenoh` crate's `get`/`put`/`declare_subscriber` builder
  API evolves between versions. Pin the zenoh version in the workspace and
  record it; isolate builder calls in `runtime/zenoh.rs`.
- `async_trait` on `invoke` boxes the future per outbound call. The cached plan
  avoids re-planning, but the per-call `Box` remains. Accepted (v4.0 §4.2);
  revisit only if profiling on a hot consume loop shows it matters.
