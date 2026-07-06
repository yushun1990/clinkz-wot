# User-Facing API — Implementation Plan

> Status: **P0 + P1 landed**. P2/P3 still pending. Sequences the migration
> delta in `docs/user-facing-api.md` §11 into four phases. Each phase
> compiles, passes tests, and ships independently.
>
> Reference docs:
> - `docs/user-facing-api.md` — frozen external boundary.
> - `docs/wot-compliance.md` — Scripting API conformance bar.
> - `AGENTS.md` — crate boundaries, `no_std` policy, style.

## Phase Summary

| Phase | Theme | Breaking? | Status |
|---|---|---|---|
| **P0** | `ProtocolBinding` facade lands | No | ✅ landed |
| **P1** | Old binding-config surface retired | Yes (cleanup only) | ✅ landed |
| **P2** | Consumer-side handle gaps closed | Additive | pending |
| **P3** | Exposed-side handle gaps closed + polish | Additive | pending |

P0 and P1 are serial (P1 depends on P0 trait). P2 and P3 are independent of
each other and may run in parallel after P1.

## Test Strategy (cross-cutting)

- **Every** phase adds a `no_std + alloc` compile check where it touches
  `no_std`-eligible code (`ProtocolBinding`, `Subscription`, `EventStream`,
  handler trait re-exports). See AGENTS.md "Testing Expectations".
- Each new handle method gets at least one fake-binding integration test
  under `servient/tests/` following the pattern in
  `servient/tests/servient_test.rs`.
- Form-selection behavior stays covered by fixtures with multiple forms per
  affordance (AGENTS.md testing rule).
- Clippy default groups clean on touched crates.

## P0 — `ProtocolBinding` Facade (Non-Breaking)

### Goal

Introduce the unified binding facade without disturbing any existing call
site. After P0, both `with_server_binding` + `with_client_factory` (legacy)
and `with_protocol_binding` (new) work side by side.

### Files

| File | Change |
|---|---|
| `core/src/binding_facade.rs` | **New.** Defines `ProtocolBinding` trait, `ClientOnly`/`ServerOnly` wrappers, `client_only`/`server_only` constructors. |
| `core/src/lib.rs` | `mod binding_facade;` + `pub use binding_facade::{ProtocolBinding, ClientOnly, ServerOnly, client_only, server_only};` |
| `servient/src/builder.rs` | Add `with_protocol_binding(Arc<dyn ProtocolBinding>) -> Self`. Internal: extract `client_factory()` and `server()`, push into existing `client_factories` / `server_bindings` slots. |
| `servient/src/lib.rs` | `pub use clinkz_wot_core::ProtocolBinding;` (re-export so users import from servient). |
| `protocol-bindings/protocols/zenoh/src/protocol_binding.rs` | **New.** `ZenohProtocolBinding` struct + `impl ProtocolBinding`. Wraps one `Arc<ZenohRuntimeTransport>` + optional `Arc<ZenohServerBinding>`. Internal factory struct impls `ClientBindingFactory` by cloning the shared transport into a fresh `ZenohBindingTransport` per consumed Thing. |
| `protocol-bindings/protocols/zenoh/src/lib.rs` | `pub use protocol_binding::ZenohProtocolBinding;` |
| `servient/tests/protocol_binding_test.rs` | **New.** Integration test using a `FakeProtocolBinding` that returns canned `ClientBindingFactory` + `ServerBinding`. Verifies that `with_protocol_binding` populates both slots and that `consume()`/`produce()` find the right adapter. |

### Key design decisions to lock in P0

1. `ProtocolBinding` is **not** a sealed trait — binding crates implement
   it externally.
2. `client_factory()` returns `Option<Box<dyn ClientBindingFactory>>`, not
   `Option<Box<dyn ClientBinding>>`. The factory indirection matches the
   existing per-Consumed-Thing instantiation pattern in
   `servient/src/servient.rs:111-113`.
3. `server()` returns `Option<Arc<dyn ServerBinding>>` — singleton, shared
   across all exposed Things.
4. The legacy `with_server_binding` / `with_client_factory` builders stay
   `pub` and unchanged in P0 so the existing zenoh integration tests in
   `protocol-bindings/protocols/zenoh/src/server.rs` and the servient test
   suite keep passing without modification.

### Acceptance Criteria

- [ ] `cargo build -p clinkz-wot-core --no-default-features` succeeds
  (verifies `no_std + alloc` compatibility of `ProtocolBinding`).
- [ ] `cargo test -p clinkz-wot-servient` passes including the new
  `protocol_binding_test.rs`.
- [ ] New test exercises a `ProtocolBinding` returning both `Some` client
  and `Some` server, and one returning `Some`/`None` for each asymmetric
  case.
- [ ] No public API removed; `with_server_binding` / `with_client_factory`
  still callable.

### Risks

- **Object-safety:** `ProtocolBinding` has no generic methods, returns
  `Box<dyn ...>` / `Arc<dyn ...>`, takes `&self`. Object-safe by
  construction. Verify with `let b: &dyn ProtocolBinding = ...;` in a test.
- **Zenoh session sharing:** the factory must hand out fresh
  `ZenohBindingTransport` clones that share the underlying session via
  `Arc`. `ZenohBindingTransport::with_transport` already takes owned `T`;
  P0 either changes the transport to `Arc<T>` internally or wraps it once
  at the factory boundary. Prefer the latter (smaller blast radius).

## P1 — Old Binding-Config Surface Retired

### Goal

After P0 lands and consumers migrate, demote the legacy hooks so the public
API has a single binding-config entrypoint.

### Prerequisite

- All in-tree callers (zenoh binding tests, servient integration tests,
  examples) updated to use `with_protocol_binding`. P0's
  `protocol_binding_test.rs` plus a new `zenoh_protocol_binding_test.rs`
  in the zenoh crate demonstrate the migration pattern.

### Files

| File | Change |
|---|---|
| `servient/src/builder.rs` | `with_server_binding` / `with_client_factory` visibility: `pub` → `pub(crate)`. Mark `#[doc(hidden)]` if external mock-test support is required (decide in §12.5 of the API doc). |
| `servient/src/lib.rs` | Remove `pub use servient::ClientBindingFactory;` (line 34). `ClientBindingFactory` stays `pub` in `clinkz_wot_core` for binding authors. |
| `servient/src/servient.rs` | `ClientBindingFactory` trait itself stays `pub` (binding authors still need it via core re-export); just stop re-exporting from servient. |
| `docs/user-facing-api.md` | Strike the "P1 prerequisite" note; mark the migration row in §11 as done. |
| `docs/wot-compliance.md` | Update the "Naming posture" paragraph to reflect that `ClientBindingFactory` is no longer in the application-facing re-export set. |

### Acceptance Criteria

- [ ] `cargo build -p clinkz-wot-servient` succeeds with no public
  `with_server_binding` / `with_client_factory` / `ClientBindingFactory`
  re-export.
- [ ] `cargo doc -p clinkz-wot-servient --no-deps` shows only
  `with_protocol_binding` in the `ServientBuilder` docs.
- [ ] All servient / zenoh tests still pass after migration.
- [ ] A grep for `with_server_binding\|with_client_factory` in
  non-`pub(crate)` code returns zero hits.

### Risks

- **External mock tests broken.** If we go `pub(crate)`, downstream crates
  cannot register mock `ServerBinding`s without going through a
  `ProtocolBinding` wrapper. Mitigation: provide a `MockProtocolBinding` in
  `clinkz_wot_servient` under a `test-support` feature flag, or accept the
  wrapper pattern as the cost of the cleaner surface. Recommend the latter
  for now; revisit if it hurts test ergonomics.

## P2 — Consumer-Side Handle Gaps

### Goal

Close all missing `ConsumedThingHandle` methods listed in
`docs/user-facing-api.md` §6.1. The plumbing (`ClientBinding::subscribe`,
`Subscription`, `Subscription::merge`) already exists; this phase wires the
handle surface.

### Sub-phases (can be separate PRs)

#### P2.1 — `ConsumedThing` subscribe passthrough

| File | Change |
|---|---|
| `core/src/thing.rs` (`ConsumedThing`) | Add `pub async fn subscribe(&self, target, operation, form, input) -> CoreResult<(Subscription, Box<dyn SubscriptionGuard>)>`. Mirrors `request` (line 1122) but calls `binding.subscribe(...)` instead of `binding.invoke(...)`. Form selection identical. |

#### P2.2 — `observe_property` / `unobserve_property`

| File | Change |
|---|---|
| `servient/src/handle.rs` (`ConsumedThingHandle`) | Add `observe_property(name, options) -> CoreResult<Subscription>` and `unobserve_property(name, options) -> CoreResult<()>`. `observe` calls `ConsumedThing::subscribe` with `Operation::ObserveProperty`; `unobserve` drops the guard returned alongside the subscription (so signature returns `()` once the guard is dropped). |
| `servient/tests/consumed_streaming_test.rs` | **New.** Fake `ClientBinding` that supports `subscribe` for `ObserveProperty`. Verifies pushed samples flow through the returned `Subscription`. |

#### P2.3 — `subscribe_event` / `unsubscribe_event` / `subscribe_all_events`

| File | Change |
|---|---|
| `core/src/event.rs` | Add `EventStream` struct: holds a `Subscription` (merged) + `BTreeMap<String, SubscriptionGuard>` for per-event cleanup. Implements `futures_core::Stream<Item = (EventName, Payload)>`. |
| `core/src/lib.rs` | `pub use event::EventStream;` |
| `servient/src/handle.rs` | Add `subscribe_event`, `unsubscribe_event`, `subscribe_all_events`. `subscribe_all_events` iterates `thing.events`, calls `subscribe_event` per event, merges into one `EventStream`. |
| `servient/tests/consumed_streaming_test.rs` | Extend: event subscribe, multi-event subscribe_all with merged stream. |

#### P2.4 — Bulk property ops

| File | Change |
|---|---|
| `servient/src/handle.rs` | Add `read_all_properties`, `read_multiple_properties`, `write_multiple_properties`. Per Scripting API §6.5: `read_all` and `read_multiple` return a single `InteractionOutput` whose payload is a JSON map `{name: value}`; `write_multiple` takes `&BTreeMap<&str, Payload>` and returns `()`. |
| `servient/tests/consumed_bulk_test.rs` | **New.** Fake binding supporting multiple `ReadProperty` ops; verify result aggregation. |

### Open design point (resolve in P2.1, before code)

- **`unobserve` signature:** return `()` (drop-guard model) vs
  `CoreResult<InteractionOutput>` (Scripting API returns the
  cancellation ack). Recommend `CoreResult<()>` — the cancellation ack
  has no useful payload and a `Result` lets the binding surface transport
  errors. Record as a Scripting-API deviation in `docs/wot-compliance.md`
  §9.

### Acceptance Criteria

- [ ] All 8 new methods present and exercised by integration tests.
- [ ] `EventStream` implements `futures_core::Stream` and round-trips
  `(EventName, Payload)` through a fake binding.
- [ ] `cargo build -p clinkz-wot-core --no-default-features` succeeds
  (`EventStream` is `no_std + alloc`).
- [ ] `subscribe_all_events` cleans up all per-event guards on drop.

### Risks

- **Bulk op aggregation semantics:** Scripting API leaves the result map
  format loose. Pick JSON object keyed by property name; document in
  `docs/wot-compliance.md` as a clinkz convention.
- **Form selection in `subscribe_all_events`:** if a Thing declares
  events that no registered binding can serve, fail-fast (return error
  listing the unsupported events) or skip silently? Recommend fail-fast;
  surface as `CoreError::UnsupportedOperation`.

## P3 — Exposed-Side Handle Gaps + Polish

### Goal

Close `ExposedThingHandle` gaps (§6.2 of API doc) and pick up the small
polish items.

### Sub-phases

#### P3.1 — Async handler setters on the handle

| File | Change |
|---|---|
| `servient/src/handle.rs` (`ExposedThingHandle`) | Add 9 `set_async_*_handler` methods mirroring `core::ExposedThing` setters (thing.rs:549-635). Each delegates to `slot.with(|s| s.thing.set_async_*_handler(...))`. Gated on `#[cfg(feature = "async")]`. |
| `servient/tests/exposed_async_handler_test.rs` | **New.** Registers `AsyncPropertyReadHandler`, fires local `read_property_async`, verifies the async path is taken. |

#### P3.2 — Local dispatch surface

| File | Change |
|---|---|
| `servient/src/handle.rs` (`ExposedThingHandle`) | Add sync local dispatch: `query_action`, `cancel_action`, `observe_property`, `unobserve_property`, `subscribe_event`, `unsubscribe_event` (6 methods). Delegate to the existing `core::ExposedThing` methods (thing.rs:691-840). |
| `servient/src/handle.rs` | Add 9 `*_async` local-dispatch methods mirroring `core::ExposedThing::*_async` (thing.rs:905-1060). Gated on `#[cfg(feature = "async")]`. |
| `servient/tests/exposed_local_dispatch_test.rs` | **New.** Round-trip: register handlers for each affordance kind, call every local-dispatch method, verify the handler runs. |

#### P3.3 — `InteractionOptions` builder conveniences

| File | Change |
|---|---|
| `core/src/interaction.rs` | Add `InteractionOptions::with_data(Payload) -> Self` and `with_uri_variable(k: &str, v: &str) -> Self`. Keep `new()` and bare-field construction. |
| `core/src/interaction.rs` | Unit tests for the new builders. |

#### P3.4 — Compliance docs sync

| File | Change |
|---|---|
| `docs/wot-compliance.md` | Update §10 conformance map to mark every newly-added method as covered. Record deviations decided in P2 (`unobserve` return shape, bulk result format, `subscribe_all_events` fail-fast). |
| `docs/user-facing-api.md` | Strike §11 migration rows that landed; strike resolved §12 open questions; keep the deferred ones (umbrella crate, error unification). |

### Acceptance Criteria

- [ ] Every Scripting API §7 method on `ExposedThing` is reachable through
  `ExposedThingHandle` (sync or async variant).
- [ ] Async handlers actually drive the `*_async` dispatch path; sync
  handlers drive the sync path. Verified by a test that asserts which
  handler ran.
- [ ] `InteractionOptions::with_data(...)` reads cleanly at call sites;
  added to the API doc's example in §5.3.
- [ ] `docs/wot-compliance.md` §9 lists every deviation with rationale.

### Risks

- **Sync vs async dispatch ambiguity on the handle:** if a user registers
  an async handler and calls the sync `read_property`, what happens?
  Recommend: sync methods return `CoreError::InvalidInteraction` when the
  registered handler is async. Document explicitly; covered by test.

## Out of Scope (deferred past v0.1)

These items from `docs/user-facing-api.md` §12 are **not** in this plan:

1. Unified facade error tree (`CoreError` / `ServientError` /
   `BindingError` / `DiscoveryError` consolidation).
2. Top-level `clinkz_wot` umbrella crate decision.
3. `read_all_properties` parallel fan-out vs sequential.
4. `ProtocolBinding::protocol()` typed `ProtocolId` newtype.
5. `with_*_binding` legacy hooks' final visibility (`pub(crate)` vs
   `#[doc(hidden)] pub`).

Each will get its own design note when prioritized.

## Suggested Branch / PR Sequence

```
main
  └─ p0/protocol-binding-facade        (P0 — single PR)
       └─ p1/retire-legacy-binding-hooks (P1 — single PR)
            ├─ p2.1/consumed-thing-subscribe
            ├─ p2.2/observe-property
            ├─ p2.3/subscribe-event
            ├─ p2.4/bulk-properties
            ├─ p3.1/exposed-async-handlers
            ├─ p3.2/exposed-local-dispatch
            └─ p3.3/interaction-options-builders
```

P2.* and P3.* sub-PRs are independent and may be opened in any order after
P1 lands. Each sub-PR is small enough for a focused review (target:
≤500 LOC excluding tests).
