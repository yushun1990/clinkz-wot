# clinkz-wot Implementation Plan

## Summary

`clinkz-wot` is a Rust implementation of a W3C Web of Things engine for the
Clinkz platform. The engine targets **strict WoT specification compliance and
WoT Scripting API conformance**, runs on both `std` and `no_std + alloc`, and
serves both device Thing integration and application-layer integration.

The authoritative engine-wide design reference is
`docs/baseline/engine-architecture-baseline.md` (v4.0). This file is the
repository-level blueprint, milestone index, and acceptance-criteria source.
Detailed per-phase implementation tasks live under `docs/plan/`.

## Scope

This file defines the delivery sequence and repository-wide acceptance criteria.
v4.0 mandates three direction decisions that drive the plan:

1. **Full WoT Scripting API alignment** — Consumer, Producer, and Discovery UA
   conformance (reverses the prior "Native Runtime" positioning).
2. **Frozen TD in v1** — no dynamic affordance add/remove after `expose()`.
3. **Async-first driving, sync-primary handlers** — the driving/transport
   layer is async; inbound handlers are synchronous (zero-allocation hot path)
   with opt-in async twins for I/O-bound cloud handlers; `no_std` super-loops
   drive the same futures by manual polling.

The three earlier Servient baselines (`servient-design-baseline.md` v3.0 and
its addendum v3.1) and the per-milestone plans that sequenced them
(`servient-runtime-redesign-plan.md`, the transitional `discovery-directory-
refactor-plan.md` execution, etc.) are **superseded** by v4.0 and the phase
plans below.

## Phases

The refactor is sequenced P0 through P4. **P0–P2 are target-crate isolation**
(each target crate compiles/tests alone; the workspace is NOT whole mid-refactor
because P0 rewrites core's public surface); **the workspace is made whole again
at P3** (see §Dependency shape below). Each phase has a dedicated plan document
under `docs/plan/`.

| Phase | Goal | Plan |
|---|---|---|
| P0 | Core interaction surface rewrite (sync-primary handlers, concrete Thing types, `WotLock`, Scripting-API I/O) | `docs/plan/phase-p0-core-interaction.md` |
| P1 | Discovery rewrite (Introduction/Exploration/session; `Discoverer`/`DirectoryPublisher`) | `docs/plan/phase-p1-discovery.md` |
| P2 | Binding async (real async `ClientBinding`; zenoh async consume; drop dynamic-affordance API) | `docs/plan/phase-p2-binding-async.md` |
| P3 | Servient rewire (drop `Servient<D>`; async-only driving; frozen-TD lifecycle; real async consume) | `docs/plan/phase-p3-servient.md` |
| P4 | Compliance and verification (Scripting API conformance map tests, feature matrix, fixtures, no-std checks) | `docs/plan/phase-p4-compliance.md` |

Dependency shape and compile boundaries: P0 is foundational (core types
everyone depends on). Because P0 rewrites core's public surface, it breaks
core's direct dependents (binding, discovery, servient) until they adapt. The
phases are therefore sequenced bottom-up along the dependency graph: **P0–P2
each leave their own target crate compiling and tested in isolation; the
workspace is made whole again at P3.** This is the accepted cost of a one-shot
breaking refactor with no downstream consumers. P1 and P2 may overlap in time
once P0's public surface is stable. P4 finalizes compliance.

### P0: Core Interaction Surface Rewrite

Plan: `docs/plan/phase-p0-core-interaction.md`.

Rewrite `clinkz-wot-core` to a single async interaction surface aligned with the
WoT Scripting API. **P0 also owns the `clinkz-wot-td` internal cleanups**
(baseline §3 / Tier 0 — the foundation data-contract layer): split
`data_type.rs`, deduplicate `Form`/`ThingModelForm`, extract shared validation
helpers, and **re-export `AbsoluteUri` at the td crate root** (AD11, P1's hard
prerequisite). These td edits are part of P0, not unassigned.

Scope:

- Concrete `LocalExposedThing` / `BoundConsumedThing` (remove single-impl
  `ExposedThing` / `ConsumedThing` traits).
- Sync-primary handler trait set (`PropertyReadHandler`,
  `PropertyWriteHandler`, …) with opt-in async twins behind `async`, and
  consolidated per-affordance handler-set storage (`PropertyHandlerSet`,
  `ActionHandlerSet`, `EventHandlerSet`).
- `InteractionOptions` / `InteractionOutput` rework (Scripting API §7.1);
  remove `InteractionInput.security_metadata`.
- Rename `MapLock<T>` → `WotLock<T>`; make it the `Clone`-able `Arc`-backed
  handle; remove `multithread` feature and `RefCell`/`UnsafeCell` backends.
- Retain owned `ThingId`, `CorrelationId`, `AffordanceTarget`, `InboundRequest`,
  `InboundResponse`, `BindingRequest` (v3.1 §1–§2).
- Remove `ServerBinding::register_affordance` / `unregister_affordance`.
- `ServerBinding` exposes a **sync non-blocking `try_accept`** + wholesale
  `register_thing`/`unregister_thing` (drop `poll_accept`/`poll_accept_sync`/
  `AsyncServerBinding`); the fan-in is a bounded channel (std) / `try_accept`
  poll (no_std) — v4.0 §4.5.

Entry criteria:

- v4.0 baseline is locked.

Exit criteria:

- `clinkz-wot-core` compiles `no_std + alloc` and `std`.
- `cargo check -p clinkz-wot-core --no-default-features` passes.
- Handler registration and a synthetic async dispatch round-trip are covered by
  tests.

### P1: Discovery Rewrite

Plan: `docs/plan/phase-p1-discovery.md`.

Execute `docs/plan/discovery-directory-refactor-plan.md` against the v4.0
surface. Discovery becomes Introduction → Exploration → continuation session.

Scope:

- `DiscoveryEndpoint`, `ThingDescriptionResolver`, `ThingLinkResolver`,
  `DirectoryReader`, `DirectorySession`, `DirectoryBatch`, `ContinuationToken`.
- `DirectoryPublisher` (lease/revision-aware) and `DirectoryWatch`.
- `Discoverer` trait (`discover` / `explore_directory` /
  `request_thing_description`).
- `ThingDiscoveryProcess` lazy session (replaces buffered `ThingDiscovery`).
- In-memory backend as reference implementation of `DirectoryReader` /
  `DirectoryPublisher`.
- Remove `ThingDirectory` CRUD container, `DirectoryPage { offset, total }`,
  `ThingFilter.method` model, `DiscoveryMethod` enum.

Entry criteria:

- P0 core identity types (`ThingId`) are stable.
- td re-exports `AbsoluteUri` at its crate root (`pub use
  core::data_type::AbsoluteUri;`, AD11) — P1's public discovery surface
  (`DiscoveryEndpoint`, `DirectoryRef`, `DirectoryQuery`) depends on it.

Exit criteria:

- `clinkz-wot-discovery` compiles `no_std + alloc` (crate root) and `std`
  (storage).
- A local continuation-session discovery round-trip is covered by tests.

### P2: Binding Async

Plan: `docs/plan/phase-p2-binding-async.md`.

Make the protocol binding consume path genuinely async and remove the
dynamic-affordance binding surface.

Scope:

- `ClientBinding::invoke` becomes `async fn` driving a real transport.
- `ZenohSessionTransport` async consume via `zenoh::Session` (`get`, `put`).
- Remove fake-async consumer delegation (PLAN-old M8).
- Remove `register_affordance` / `unregister_affordance` from concrete bindings.
- Keep shared form selection, op resolution, target resolution, security
  metadata extraction unchanged.
- Keep the `runtime-*` feature split (planning `no_std+alloc` vs `zenoh` /
  `zenoh-pico` backends).

Entry criteria:

- P0 async `ClientBinding` / `ServerBinding` traits are stable.

Exit criteria:

- `clinkz-wot-protocol-bindings` and `clinkz-wot-protocol-bindings-zenoh` pass
  `cargo test` and `--no-default-features`.
- A real async zenoh consume round-trip (read/write/invoke) is covered by an
  opt-in smoke test.

### P3: Servient Rewire

Plan: `docs/plan/phase-p3-servient.md`.

Rewire the Servient on top of the P0–P2 surfaces: drop the directory generic,
async-only driving, frozen-TD lifecycle, real async consumer.

Scope:

- `Servient` holds `Arc<dyn Discoverer>` + `Option<Arc<dyn DirectoryPublisher>>`
  (drop `Servient<D>`).
- Async-only driving: `poll_serve` / `serve` / `poll_serve_once` (manual-poll
  for bare `no_std` super-loops). Remove `driving_sync.rs` /
  `driving_async.rs` / `DrivingState` / `AsyncAcceptState` split.
- `produce` / `consume` / `discover` / `fetch_td` facade; `expose` / `destroy`
  with frozen TD; remove `add_*` / `remove_*` post-expose mutation.
- Real async `ConsumedThingHandle` (remove fake-async delegation).
- Retain `EventBroker`, bulk operations, security verification, credential
  store, graceful shutdown.

Entry criteria:

- P0, P1, P2 surfaces are stable.

Exit criteria:

- `clinkz-wot-servient` compiles `no_std + alloc` (crate root) and `std`.
- Integration tests cover produce→expose→read/write/invoke/subscribe and
  consume→read/write/invoke/subscribe end-to-end through a fake binding and
  (opt-in) the zenoh binding.
- **`destroy()` quiescing lifecycle** covered (AD15): in-flight requests
  rejected/dropped after the draining flag, in-flight handlers complete with
  results discarded, self-`destroy` from a handler uses deferred removal.

### P4: Compliance and Verification

Plan: `docs/plan/phase-p4-compliance.md`.

Lock the compliance surface and the verification baseline.

Scope:

- Scripting API conformance map tests (§10 of v4.0).
- Feature-matrix verification (`std`, `async`, `zenoh`, `zenoh-pico`,
  `td2-preview`).
- `scripts/check-no-std.sh` updated for the new crate surfaces and `WotLock`.
- TD 1.1 fixture round-trip and multi-form affordance coverage.
- Clippy clean across the workspace.
- Update `docs/technical-spec.md`, `docs/wot-compliance.md`,
  `docs/no-std-embedded.md`, `docs/verification.md` to reflect v4.0.

Entry criteria:

- P0–P3 are complete.

Exit criteria:

- The full workspace M-baseline passes: `cargo fmt --check`,
  `cargo test --workspace`, `cargo clippy --workspace --all-targets`,
  `scripts/check-no-std.sh`, `scripts/check-m7.sh` (renamed/updated).

## Acceptance Criteria

- TD/TM documents parse, validate, serialize, and round-trip without losing
  extension data, on `no_std + alloc`.
- The engine core has no dependency on zenoh or any concrete transport.
- The zenoh binding is optional and feature-gated.
- All protocol bindings use the same protocol-neutral `ClientBinding` /
  `ServerBinding` trait surface.
- The `WoT` facade, `ExposedThing`, `ConsumedThing`, and `ThingDiscovery`
  surfaces follow the WoT Scripting API method catalogue (v4.0 §10).
- `no_std + alloc` crates compile without `std`.
- The only documented deviations from the Scripting API are those listed in
  v4.0 §9.

## Performance Hardening

The per-request hot path must remain allocation-light and lock-bounded under
v4.0. Targets (carried from the prior hardening pass, re-validated against the
async surface):

- Affordance addressing via `Arc<str>` (retained).
- Handler invocation clones one `Arc<dyn Handler>` out of a per-Thing handler-
  set map under a brief lock, releases, then `.await`s. One `async_trait`
  `Box` per call is the accepted cost.
- Outbound form/binding plan interned in the consumed registry entry; repeated
  consumed interactions reuse the cached binding instance via `Arc` clone.
- Event fan-out shares `Payload` bytes via `Arc<[u8]>`; media metadata may move
  to `Arc<str>` if profiling warrants.
- Lock contention bounded by the two-level model; `WotLock` removes the
  `multithread` feature coordination cost.
- Directory queries are continuation-based (one batch + token), not
  `offset+total` full-table scan.

## Deprecated Documentation

The following are superseded by v4.0 and retained only as historical record
(each carries a SUPERSEDED banner):

- `docs/baseline/servient-design-baseline.md` (v3.0)
- `docs/baseline/servient-design-baseline-addendum.md` (v3.1)
- `docs/plan/servient-runtime-redesign-plan.md` (sequenced v3.0/v3.1)
- `docs/plan/wot-td-development-plan.md` (M1/M2 hardening — mostly complete,
  remaining cleanups folded into P0 §3 TD internal splits)
- `docs/plan/protocol-bindings-development-plan.md` (M4 — superseded by P2)
- `docs/plan/discovery-directory-refactor-plan.md` (folded into P1 as the
  design source; P1 is the implementation plan)

The technical-spec, wot-compliance, and no-std-embedded docs are updated in P4
to match v4.0.
