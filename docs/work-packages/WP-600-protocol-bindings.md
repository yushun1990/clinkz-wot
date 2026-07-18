# WP-600 Optional Zenoh and Zenoh-Pico Protocol Bindings

Status: Planned
Design revision: v4.9
Depends on: `WP-300`
Required gates: `GATE-1`, `GATE-2`, `GATE-3`, `GATE-4`, `GATE-5`, `GATE-6`
Owner packages: `clinkz-wot-planning`, `clinkz-wot-protocol-bindings-zenoh`

## Scope

Migrate the shared protocol-planning package and the first optional concrete binding to the frozen
planning, registration, lifecycle, request, response, subscription, emission, security, codec, and
cleanup contracts from `WP-200` and `WP-300`. The Rust zenoh backend is a host integration behind
the `zenoh` feature. The constrained zenoh-pico backend is exposed by the mutually exclusive
`zenoh-pico` feature and uses manually driven operation slots.

Zenoh remains optional. Neither `clinkz-wot-td`, `clinkz-wot-core`,
`clinkz-wot-discovery`, nor `clinkz-wot-servient` may acquire zenoh-specific behavior or a required
dependency on the concrete package. There is currently one concrete Cargo package,
`clinkz-wot-protocol-bindings-zenoh`; zenoh-pico is a feature/backend of that package, not a
separate Cargo package. Work may begin after `WP-300` and all entry gates are closed.

Concrete response metadata follows
`docs/amendments/WP-100-interaction-output-api-v1.md`: bindings populate the
untrusted fixed-size metadata channel, while shared WP-300 validation retains it
only after live identity and response-plan checks pass. Native status
provenance is proved by the concrete binding rather than inferred from its
opaque numeric value.

## Requirements

- `CRATE-DEPS-001`
- `FORM-COVERAGE-001`
- `BIND-IO-001`
- `BIND-OUT-001`
- `BIND-PROGRESS-001`
- `BIND-CALL-CANCEL-001`
- `BIND-HOST-CANCEL-001`
- `SUB-STORAGE-001`
- `SUB-DATA-001`
- `API-PAYLOAD-001`
- `API-SECURITY-001`
- `API-CODEC-001`
- `CONSTRAINED-PROGRESS-001`
- `PRODUCER-EMIT-001`
- `PERF-FANOUT-001`
- `PERF-FANOUT-002`

The package consumes `PLAN-INDEX-001`, `PLAN-REQUEST-001`, `STATE-BIND-001`,
`STATE-SUB-001`, `LIFE-EXPOSE-003`, and `PRODUCER-EMIT-001` without changing their semantics.

## Crates and Feature Cells

| Cargo package | Feature cell | Required surface |
| --- | --- | --- |
| `clinkz-wot-planning` | `--no-default-features` | `CapabilityIndex`, `PlanCompiler`, form/security/target resolution, and URI-template compilation without execution-trait ownership |
| `clinkz-wot-planning` | `async`, no `std` where provided | Async compiler adapters without an executor or concrete transport |
| `clinkz-wot-planning` | `std` | Host planning conveniences only; no concrete zenoh dependency |
| `clinkz-wot-protocol-bindings-zenoh` | `--no-default-features` | Zenoh form metadata, protocol-local compiler data, and constrained adapter types without a concrete runtime |
| `clinkz-wot-protocol-bindings-zenoh` | `zenoh` | Rust zenoh host backend implementing the host binding registrations and runtime status contract |
| `clinkz-wot-protocol-bindings-zenoh` | `zenoh-pico` | Constrained zenoh-pico client/server adapters implementing poll progress with caller-owned slots and no `std` |

The `zenoh` and `zenoh-pico` features remain mutually exclusive. `async` is syntax and adapter
surface only and must not enable Tokio, the Rust zenoh runtime, or another executor. The `zenoh`
feature may enable its host runtime dependencies; `zenoh-pico` must not enable `std`, Tokio,
`Arc`-only registration, or boxed-future-only progress.

The WP-600 feature-cell set is exactly `no-default`, `async-no-std`, and `std`.
The `no-default` cell is an independent baseline and is not implied by `async-no-std`.

## Public API and Data Migration

- Use `clinkz_wot_protocol_bindings::{CapabilityIndex, PlanCompiler, PlanBuildInput,
  PlanBuildOutput, CompiledUriTemplate, ResolvedFormTarget}` from `WP-200` as the shared planning
  boundary. Zenoh-specific compilation consumes `BindingCandidate` or `PlanBuildInput`; it does
  not take ownership of the TD tree or redefine operation defaulting and security inheritance.
- Migrate zenoh server integration to `ServerBindingRegistration`, `ServerBinding`,
  `ServerFormContributor`, `RouteReadinessDriver`, and bounded runtime-event sink metadata from
  `clinkz-wot-core`. Route guards contain protocol-local resources behind the core's erased host
  wrappers.
- Migrate zenoh client integration to `ClientBindingRegistration` and `ClientBinding` using an
  owned `OutboundRequest`. The binding must use the selected plan, route and binding generations,
  applied security, correlation id, and response validation contract without selecting another
  form.
- Preserve the host convenience constructors `shared`, `server`, `client`, `client_pooled`, and
  `client_pooled_default`, but make their result type an explicit host binding object or a complete
  registration. Builder convenience wrapping must not discard binding id, generation, driving
  mode, readiness, form contribution, status, or overflow metadata.
- Implement zenoh-pico through `PollClientBinding` and `PollServerBinding`, including
  `ClientRequestSlot`, `ClientSubscriptionSlot`, `ServerResponseSlot`, and
  `BindingEmissionSlot`. `ZenohPicoTransport` and its platform callbacks may remain
  protocol-specific, but all engine-visible progress and terminal values use core types.
- Map transport credentials only into `TransportAuthMaterial`; core owns body authentication,
  security branch verification, scope checks, and the application payload projection. Outbound
  zenoh metadata comes only from `OutboundRequest::applied_security` after provider commit.
- Implement the host subscription receive and teardown path as a binding-owned
  `HostSubscriptionDriver`. Zenoh and zenoh-pico own protocol credit, callback ingress, prefetch,
  and any bounded protocol-local storage; neither returns a core queue or public sender.
- Advertise a typed native capability for root-form `subscribe_all_events` and
  `observe_all_properties` only when the selected route provides exact source attribution and
  bounded teardown. The concrete compiler maps Zenoh wildcard or selector syntax; core and
  Servient never interpret it.

## State and Ownership Migration

- Key every prepared, active, subscription, request, response, and emission resource by
  `BindingRouteKey` and binding generation. Late zenoh callbacks carry that generation and cannot
  mutate a replacement route.
- Replace listener declaration during a monolithic serve call with deterministic form
  contribution, local `prepare`, explicit readiness, `activate`, `commit`, and bounded
  abort/shutdown. No lifecycle call waits on network or executor progress.
- Keep prepared and active resources addressable until `CleanupOutcome` is terminal or ownership
  transfers through a `CleanupRecord`. `PendingCleanup` never means that an untracked zenoh query,
  subscription, listener, or lease remains.
- Enforce the Servient activation gate before an inbound request reaches dispatch. Self-driving
  host tasks report transport, delivery, backpressure, panic, and terminal state through the
  configured runtime event and durable status paths.
- Move an accepted inbound transport buffer into an owned `InboundRequest`; responses retain the
  same route and correlation identities. Duplicate live correlations are rejected within one
  route, while unrelated route generations remain independent.
- For zenoh-pico, retain progress cursors and owned buffers in caller-visible generation-bearing
  slots. Budget exhaustion returns pending work without restarting decode, fan-out, response, or
  cleanup from the beginning.
- Consume only WP-300 `ProducerEmission` and `BindingPublication` at the engine boundary. Host
  zenoh and zenoh-pico preserve emission target, route, subscription, binding generation, payload
  lease, overflow result, and retained cursor without re-entering a legacy `PublisherSink`.
- Treat one `BindingEmissionSlot` as one selected binding publication. Remote subscriber fan-out
  behind a Zenoh key expression remains internal to that binding and does not create one engine
  sink or payload copy per remote subscriber.

## Old API Removal

- Remove any `ProtocolBinding` and `ClientBindingFactory` facade and any documentation or example
  that presents either as the registration boundary.
- Remove `ServerBinding::serve(&ThingId, &Thing, &BindingContext)` and
  `shutdown(&ThingId)` implementations from `ZenohServerBinding`, together with tests that treat
  guard drop or textual Thing id as lifecycle ownership.
- Remove zenoh runtime paths that accept a complete `Thing` and re-run form selection, TD default
  operations, `base` resolution, security inheritance, or schema selection. Runtime execution
  receives compiled protocol-neutral candidates and inbound plans.
- Remove public direct-TD planning helpers such as `plan_zenoh_operation` and
  `plan_zenoh_affordance_operation` after equivalent `PlanCompiler` entry points and migration
  fixtures exist. Protocol-local inspection helpers may remain private to the compiler.
- Remove hidden busy-retry loops, binding-owned unbounded request/subscription tables, and any
  zenoh-pico path that requires `std`, Tokio, `Arc<dyn ...>`, or a boxed future to make progress.
- Remove transport-specific security interpretation that bypasses `TransportAuthMaterial`,
  `SecurityProvider` probe/commit, or the shared response validation path.
- Remove `PublisherSink` and the WP-300 protocol-side compatibility adapter after both zenoh and
  zenoh-pico publish exclusively through `ProducerEmission`. No concrete binding may call
  `PushFn`, `SubscriptionSender`, a Servient handler setter, or an application handler directly.
- Remove `BindingRequest`, core queue construction, `SubscriptionGuard`, `EventStream`, and any
  binding path that asks Servient to synthesize a collection subscription by opening N event
  subscriptions.

No compatibility feature may reintroduce zenoh into a protocol-neutral crate.

## Evidence

- `protocol-neutral-core-dependencies`: Cargo metadata and source inspection proving that lower
  engine crates contain no zenoh dependency or zenoh-specific branch.
- `zenoh-form-and-route-compilation`: multi-form, relative target, operation, media, extension,
  security, form-owner, collision, and deterministic contribution fixtures.
- `zenoh-binding-lifecycle`: host prepare/readiness/activate/commit/serve/drain/cleanup failure
  injection with activation-gate and durable-status evidence.
- `zenoh-pico-bounded-progress`: no-std compile fixtures plus request, response, subscription,
  emission, cancellation, cleanup, byte-budget, and work-budget resume tests.
- `binding-generation-and-correlation`: stale callback, route replacement, duplicate correlation,
  response opportunity, and idempotent cleanup evidence for both backends.
- `binding-response-provenance`: protocol-native status/branch extraction, untrusted metadata
  construction, shared validation, and structured failure translation for both backends.
- `binding-owned-flow-control`: driver polling, protocol credit/prefetch, admitted storage,
  overflow/loss accounting, exact source items, and stop/drop teardown for both backends.
- `zenoh-native-collection-subscriptions`: one root-form start, exact source attribution, native
  multiplexing, bounded cleanup, and negative capability cases without implicit fan-out.

The `producer-emission-migration` evidence owned by WP-300 is consumed here with concrete source
inspection proving that both backend features have removed their adapter exit and every
`PublisherSink` reference.

Feature evidence must include `--no-default-features`, host `zenoh`, constrained `zenoh-pico`, and
an expected compile failure when both concrete backend features are selected.

## Performance Workloads

- `PERF-GW-009`: erased host network-call metadata allocations.
- `PERF-GW-010`: allocation-sensitive poll/native network-call metadata.
- `PERF-CS-002`: constrained inbound dispatch excluding transport I/O.
- `PERF-GW-007`, `PERF-GW-018`, `PERF-GW-019`, and `PERF-CS-007` cover binding-owned
  subscription start, receive, cancellation, and stop progress.
- `PERF-GW-008`, `PERF-CS-008`, and `PERF-CS-009` cover binding-local publication and remote
  fan-out without per-subscriber engine payload copies.
- `PERF-GW-023`, `PERF-GW-024`, `PERF-GW-025`, `PERF-GW-026`, and `PERF-GW-027` cover compiled
  Zenoh targets, binding scaling and isolation, exposure construction, and native collection
  behavior. `PERF-CS-018` and `PERF-CS-019` cover the corresponding zenoh-pico retained progress
  and native collection paths.

Adapter results must identify the backend feature, target, toolchain, allocator, runner, manifest,
fixture, and workload. Transport I/O is outside the two metadata workload boundaries unless the
manifest explicitly includes it; a host result cannot stand in for zenoh-pico evidence.

## Completion Conditions

- `WP-300` is complete, all entry gates remain closed, and the only concrete package is optional
  from every protocol-neutral crate and from the umbrella default feature set.
- Shared planning and both concrete backend features compile in their required cells with no
  reverse dependency or executor leakage.
- Host zenoh and constrained zenoh-pico pass the lifecycle, ownership, progress, security, codec,
  generation, correlation, cancellation, and cleanup evidence above.
- The host constructors preserve complete registration metadata, while zenoh-pico exposes a useful
  caller-driven surface without `std` or erased host traits.
- Removed monolithic lifecycle and direct-TD runtime planning APIs are absent from public compile
  fixtures and production call sites.
- `PublisherSink` and the protocol-side emission adapter are absent from both concrete feature
  cells; all Producer publication reaches the WP-300 bounded emission state.
- Native collection tests use one selected root route and one driver, while missing or inexact
  capability returns no-compatible-form without Servient-side fan-out.
- The listed performance workloads satisfy their fixture-locked budgets and structural invariants
  through `tools/performance-harness`.
