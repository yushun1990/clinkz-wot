# WP-600 Optional Zenoh and Zenoh-Pico Protocol Bindings

Machine-readable package status, design revision, dependencies, document path,
and owner crates are defined only in [`index.toml`](index.toml). This document
specifies technical scope and acceptance boundaries.

## Scope

Migrate the shared planning package and the first optional concrete binding to the frozen planning,
complete-registration, compiler-extension, committed-route activation-permit lifecycle, request,
response, subscription, emission, security, codec, memory, and cleanup contracts from `WP-200`
and `WP-300`. The Rust zenoh backend is a host integration behind the `zenoh` feature. The
constrained zenoh-pico backend is exposed by the mutually exclusive `zenoh-pico` feature and uses
manually driven associated-state operation slots.

Zenoh remains optional. Neither `clinkz-wot-td`, `clinkz-wot-core`,
`clinkz-wot-discovery`, nor `clinkz-wot-servient` may acquire zenoh-specific behavior or a required
dependency on the concrete package. There is currently one concrete Cargo package,
`clinkz-wot-protocol-bindings-zenoh`; zenoh-pico is a feature/backend of that package, not a
separate Cargo package. WP-600 package completion still depends on `WP-300`. An exact WP-600
capability tranche may enter before broad WP-300 completion only through ADR-0013 when its
matching WP-200/WP-300 predecessors, any required WP-400 runtime tranche, applicable domain-entry
authority, and applicable architecture proof are complete.

The first production-oriented WP-600 tranche is real Host Zenoh Consumer
Property Read after the Consumer Property Read architecture proof and its
matching shared/runtime tranches are stable. It must reuse the admitted consumed
plan, selected `OutboundRequest`, client-call ownership, response-validation,
generation, cancellation, and cleanup boundaries with no legacy form or binding
selection. Producer Property Read remains required regression evidence and joins
that Consumer path in the first bidirectional Zenoh Property Read loop; it is not
repeated as the only new production proof.

A bounded real-target Zenoh Producer Property Read feedback probe already ran
after the narrow WP-300 slice and before the aggregate mock gate. It exercises
public target authoring plus actual protocol I/O, correlation, readiness,
cancellation/drain, cleanup, multiple Thing/route/form shape, and at least one
network round trip. That historical probe remains architecture evidence rather
than retroactive WP-600 product progress. Future real Host Zenoh paths count as
WP-600 production progress only when they use admitted public target contracts
without test-only or legacy selection/dispatch edges. Real zenoh-pico
associated-state/backend evidence may follow the Host path, but it is mandatory
before constrained parity or completion is claimed for the corresponding common
capability.

Zenoh and zenoh-pico are two representations of one protocol family. Their
shared runtime cases establish production-backed Host/constrained parity for the
explicitly declared common capability intersection; they do not by themselves
establish protocol-shape neutrality. Profile-only, compile-only, unsupported,
and not-applicable capabilities remain explicit registration and evidence
dispositions rather than silently omitted cases.

The broad consumer-response metadata flow in
`docs/amendments/WP-100-interaction-output-api-v1.md` is v5-inactive domain-entry
input. If broad WP-600 later re-adopts it, concrete bindings populate untrusted
metadata and validate it against live identity and compiled response facts at
the applicable consumer boundary. For the current Producer Property Read
carrier, concrete bindings instead receive a Core-sealed handler result,
validate only the live route/generation/correlation, and map the result to the
protocol. They do not repeat handler-origin validation. Native status
provenance remains concrete binding evidence rather than an inference from an
opaque numeric value.

## Requirements

- `CRATE-DEPS-001`
- `FORM-COVERAGE-001`
- `BIND-REG-001`
- `BIND-ROUTE-001`
- `BIND-STORAGE-001`
- `BIND-MEM-001`
- `BIND-DELIVERY-001`
- `BIND-IO-001`
- `BIND-OUT-001`
- `BIND-PROGRESS-001`
- `BIND-CALL-CANCEL-001`
- `BIND-HOST-CANCEL-001`
- `LIFE-EXPOSE-002`
- `LIFE-EXPOSE-003`
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
`STATE-SUB-001`, and `PRODUCER-EMIT-001` without changing their semantics.

## Crates and Feature Cells

| Cargo package | Feature cell | Required surface |
| --- | --- | --- |
| `clinkz-wot-planning` | `--no-default-features` | `CapabilityIndex`, `PlanCompiler`, form/security/target resolution, and URI-template compilation coordinated over the core compiler-extension SPI without execution-trait ownership |
| `clinkz-wot-planning` | `async`, no `std` where provided | Async compiler adapters without an executor or concrete transport |
| `clinkz-wot-planning` | `std` | Host planning conveniences only; no concrete zenoh dependency |
| `clinkz-wot-protocol-bindings-zenoh` | `--no-default-features` | Zenoh form metadata, protocol-local compiler data, and constrained adapter types without a concrete runtime |
| `clinkz-wot-protocol-bindings-zenoh` | `zenoh` | Rust zenoh host backend constructing one complete host bundle and implementing route-scoped progress and runtime status |
| `clinkz-wot-protocol-bindings-zenoh` | `zenoh-pico` | Constrained zenoh-pico bundle implementing poll progress with caller-owned associated-state slots and no `std` |

The `zenoh` and `zenoh-pico` features remain mutually exclusive. `async` is syntax and adapter
surface only and must not enable Tokio, the Rust zenoh runtime, or another executor. The `zenoh`
feature may enable its host runtime dependencies; `zenoh-pico` must not enable `std`, Tokio,
`Arc`-only registration, or boxed-future-only progress.

The WP-600 feature-cell set is exactly `no-default`, `async-no-std`, and `std`.
The `no-default` cell is an independent baseline and is not implied by `async-no-std`.

## Public API and Data Migration

- Use `clinkz_wot_planning::{CapabilityIndex, PlanCompiler, PlanBuildInput, PlanBuildOutput,
  CompiledUriTemplate, ResolvedFormTarget}` from `WP-200` for shared coordination and
  `clinkz_wot_core::{BindingCandidate, BindingCompilerExtension, BindingCompilerInput,
  BindingArtifactEnvelope, BindingArtifactRef}` for the compiler/artifact SPI. Zenoh-specific
  compilation consumes an already resolved candidate; it does not take ownership of the TD tree
  or redefine operation defaulting and security inheritance.
- Migrate zenoh server integration to the server execution component, deterministic
  `ServerFormContributor`, owned prepared, active, and committed-closed route guards/calls, and
  route-scoped readiness and accept event contracts from `clinkz-wot-core`. Route guards contain
  protocol-local resources behind the core's erased host wrappers. Every serving committed route
  owns exactly one accept cursor and waker lease, and every `poll_accept` receives a fresh borrowed
  `RouteActivationPermit<'_>` for that exact route.
- Migrate zenoh client integration to the client execution component using an owned
  `OutboundRequest`. The binding must use the selected plan, route and binding generations,
  applied security, correlation id, and response validation contract without selecting another
  form.
- Preserve the host convenience constructors `shared`, `server`, `client`, `client_pooled`, and
  `client_pooled_default` only where they return a complete `HostBindingRegistration` or an
  explicitly named component builder that cannot be installed. Every installable result includes
  the compiler, compatible execution halves, binding/configuration generations, capability and
  form contribution, readiness, reactor, ingress, footprint, status, overflow, and cleanup
  metadata, preparation visibility, and closed-ingress policy. No bare host component receives
  synthesized defaults at Servient registration time.
- Implement zenoh-pico through `PollClientBinding` and `PollServerBinding`, including
  associated `RequestState`, `SubscriptionState`, `RouteState`, `ReadinessState`,
  `ResponseState`, and `EmissionState` types plus the corresponding generic caller-owned slots.
  Each state publishes its `BindingStateLayout`, lifetime/transient footprint, construction/drop
  contract, and generation checks. `ZenohPicoTransport` and its platform callbacks may remain
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
- Construct one complete startup bundle per selected backend. The bundle atomically pairs the
  concrete compiler extension and artifact compatibility identity with all client/server
  execution roles, form contribution, footprint and ingress declarations, reactor/wake policy,
  status/overflow policy, cleanup contract, and supported profile cells. No independently
  installable half or runtime registration mutation remains.

## State and Ownership Migration

- Key every prepared, active, committed-closed, subscription, request, response, and emission
  resource by `BindingRouteKey` and binding generation. Late zenoh callbacks carry that generation
  and cannot mutate a replacement route.
- Replace listener declaration during a monolithic serve call with deterministic form
  contribution, local `prepare`, explicit readiness, `activate`, `commit`, and bounded
  abort/shutdown. Successful commit returns a retained committed-closed guard and does not open
  admission. Acceptance is polled through that guard with a fresh route-scoped permit, never
  through an active guard or one registration-wide cursor. No lifecycle call waits on network or
  executor progress.
- Keep prepared, active, and committed-closed resources addressable until `CleanupOutcome` is
  terminal or the complete protocol work object transfers through an acknowledged cleanup
  envelope. `PendingCleanup` never means that an untracked zenoh query, subscription, listener,
  guard, driver, or lease remains. Shutdown accepts either the active or committed-closed guard,
  including committed guards returned after cancellation.
- Accept inbound work only while Servient supplies the permit obtained by consuming the
  exclusive claim over that route driver's `RouteAcceptLease` and the one serving activation
  authority for the exact Thing, produced generation, plan-set generation, and route.
  The binding validates the permit against its route call and may not retain it in a guard,
  associated-state slot, reactor queue, or detached task. A bounded protocol reactor may advance
  protocol-local I/O and wake the route, but it receives no application dispatch authority.
  Route-scoped polling reports requests, operational errors, and one terminal event through the
  configured runtime event and durable status paths.
- Enforce the declared preparation visibility and closed-ingress policy. An externally visible
  prepared route uses exactly one of reject, backpressure, or buffer-within-admitted-limits. Before
  publication it cannot emit an `InboundRequest`, create an engine response opportunity, or report
  application acceptance; any admitted buffer remains route-owned through rollback or shutdown.
- Move an accepted inbound transport buffer into an owned `InboundRequest`; responses retain the
  same route and correlation identities. Duplicate live correlations are rejected within one
  route, while unrelated route generations remain independent.
- For zenoh-pico, retain progress cursors and owned buffers in caller-visible generation-bearing
  associated-state slots. Budget exhaustion returns pending work without restarting decode,
  remote fan-out, response, or cleanup from the beginning. State construction and destruction
  follow the declared layout and happen only after terminal acknowledgement. The route slot records
  the `CommittedClosed` stage, but it never retains a `RouteActivationPermit<'_>` between polls.
- Consume only WP-300 `ProducerEmission` and `BindingPublication` at the engine boundary. Host
  zenoh and zenoh-pico preserve emission target, route, subscription, binding generation, payload
  lease, overflow result, and retained cursor without re-entering a legacy `PublisherSink`.
- Treat one `BindingEmissionSlot` as one selected binding publication. Remote subscriber fan-out
  behind a Zenoh key expression remains internal to that binding and does not create one engine
  sink or payload copy per remote subscriber.
- Return `BindingInputRejection<InboundResponse>` or
  `BindingInputRejection<BindingPublication>` with the complete input on every failure before
  delivery acceptance. Once accepted, retain the input and opportunity through exactly one
  terminal result, late-result classification, acknowledged transfer of the complete call or
  slot, or durable residual state. A `CleanupRecord` without the protocol work object is not a
  transfer.
- Declare immutable maximum lifetime footprints for compiler cursors/artifacts, prepared, active,
  and committed-closed route guards, calls, drivers, associated states, response/cancellation
  buffers, reactor queues, and closed-ingress buffers before side effects. Enforce ingress item and
  byte bounds per route, per binding, per Thing where applicable, and globally without hiding a
  transport-runtime queue.

## Old API Removal

- Remove any `ProtocolBinding` and `ClientBindingFactory` facade and any documentation or example
  that presents either as the registration boundary.
- Remove `ServerBinding::serve(&ThingId, &Thing, &BindingContext)` and
  `shutdown(&ThingId)` implementations from `ZenohServerBinding`, together with tests that treat
  guard drop or textual Thing id as lifecycle ownership.
- Remove zenoh runtime paths that accept a complete `Thing` and re-run form selection, TD default
  operations, `base` resolution, security inheritance, or schema selection. Runtime execution
  receives compiled protocol-neutral candidates and inbound plans.
- Remove every concrete call to
  `select_affordance_form_selection_with_result_filter` and the other legacy
  `select_form*`/`select_affordance_form*` family. No adapter may translate a
  WP-200 artifact back into a TD/form input for those functions.
- Remove public direct-TD planning helpers such as `plan_zenoh_operation` and
  `plan_zenoh_affordance_operation` after equivalent `PlanCompiler` entry points and migration
  fixtures exist. Protocol-local inspection helpers may remain private to the compiler.
- Remove hidden busy-retry loops, binding-owned unbounded request/subscription tables, and any
  zenoh-pico path that requires `std`, Tokio, `Arc<dyn ...>`, or a boxed future to make progress.
- Remove split installable client/server registrations, separately installed compiler or
  contributor components, runtime event-sink configuration objects, driving-mode switches,
  registration-wide acceptance, and concrete opaque core slots. Both backends enter Servient
  only through their complete startup bundle and progress through route-scoped or associated-state
  contracts.
- Remove per-route `open_gate` or `release_gate` callbacks, `RouteCommitOutcome::Serving`,
  `poll_accept` over an active guard or without a borrowed activation permit, and any binding view
  of Servient registry state. A previously observed wake or protocol frame is not serving
  authority.
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
- `zenoh-complete-registration`: bundle construction and rejection fixtures covering compiler and
  execution compatibility, all required policies and maxima, profile cells, startup-only
  publication, and absence of independently installable components.
- `zenoh-property-read-smoke`: real Host Zenoh Consumer Property Read through immutable consumed
  plan, selected `OutboundRequest`, client execution, validated response, cancellation/late-result
  settlement, and terminal cleanup, plus Producer Property Read regression through the admitted
  route/permit/response path. Neither side may use a mock transport or legacy selection/dispatch
  edge.
- `zenoh-author-usability`: an external binding-author fixture recording the
  complete required declarations, helper use, diagnostics, repeated
  workaround classes, cleanup-library mapping, generic/layout/code-size cost,
  and unsafe/private dependencies; systematic semantic-error patterns or
  bounded-cost violations can reopen the shared SPI as well as outright
  ownership, portability, resource, unsafe, or implementability defects.
- `zenoh-cross-profile-parity`: both backends consume one versioned
  machine-readable trace oracle for their common capability intersection,
  compare semantic transitions, normalized liveness and semantic resource
  deltas, and report physical allocation/layout/reactor/wake costs separately.
- `zenoh-route-scoped-progress`: prepare/readiness/activate/commit/accept/drain ownership, one
  accept cursor and waker per serving committed route, reactor wake isolation, no direct handler
  dispatch, and route-terminal isolation for the host backend.
- `zenoh-serving-activation`: host and constrained trace evidence that commit returns a complete
  committed-closed guard, all-route publication is atomic, and each `poll_accept` consumes one
  exclusive claim over its exact `RouteAcceptLease` into one fresh borrowed non-retained permit.
  Failure injection covers pre-publication traffic, Nth-route commit failure, both
  publication/cancellation orderings, stale and mismatched permits, duplicate concurrent claims,
  unclaimed-permit attempts, both drain/accept-claim orderings, late committed guards, and reject,
  backpressure, and bounded-buffer closed-ingress policies. The evidence records zero unclaimed
  permits, duplicate claims, partial admissions, pre-publication requests or response opportunities,
  post-drain claims, lost guards, or cleanup work objects.
- `zenoh-associated-state-storage`: every zenoh-pico associated state at its declared size and
  alignment, typed slot construction/drop, zero-budget retention, stale generations, and reuse
  after terminal acknowledgement.
- `zenoh-lifetime-ingress-bounds`: lifetime/transient footprint maxima, reactor and transport
  hidden-buffer inspection, ingress saturation at every required scope, rollback, and unrelated
  route progress.
- `zenoh-input-preservation`: typed response/publication rejection before acceptance, aligned
  host/static terminal classifications, late result handling, complete cleanup-work transfer,
  handoff fallback, and residual commitment.
- `zenoh-form-and-route-compilation`: multi-form, relative target, operation, media, extension,
  security, form-owner, collision, and deterministic contribution fixtures.
- `zenoh-binding-lifecycle`: host prepare/readiness/activate/commit/permit-gated
  accept/drain/cleanup failure injection with committed-guard and durable-status evidence.
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

The same source inspection proves that every concrete Zenoh call to the legacy
selector family, monolithic `ServerBinding::serve`, and target-to-legacy
execution adapter is absent. This is the concrete-edge stage of the no-backflow
proof; WP-700 owns final public and source absence.

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
- `PERF-GW-028`, `PERF-GW-029`, `PERF-GW-030`, `PERF-GW-031`, and `PERF-GW-032` cover owned-call
  cancellation, plan-set generations, route readiness, complete Zenoh registration, and bounded
  ingress. `PERF-CS-020`, `PERF-CS-021`, `PERF-CS-022`, and `PERF-CS-023` cover the corresponding
  typed-slot, plan-set, route, and ingress behavior for zenoh-pico. `PERF-GW-030` and
  `PERF-CS-022` additionally run the same serving-activation trace oracle and case ordering for
  atomic publication, permit non-retention, stale/pre-publication/post-drain rejection, and all
  three externally visible closed-ingress policies.

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
- Their common capability intersection and every profile-only, compile-only,
  unsupported, or not-applicable cell are explicit; the completion claim is
  Zenoh-family production evidence rather than proof of general
  protocol-shape neutrality.
- The host constructors preserve complete registration metadata, while zenoh-pico exposes a useful
  caller-driven surface without `std` or erased host traits.
- Route progress is engine-orchestrated and route-scoped in both backends; no protocol reactor has
  application dispatch authority, and one route cannot consume a sibling route's wake or terminal
  event.
- Successful route commit remains closed until Servient's atomic publication. Every accept poll
  uses a fresh exact-route borrowed permit, drain stops new permit issuance before it becomes
  observable, and neither backend retains or reconstructs serving authority.
- Every constrained protocol state uses its associated typed slot, every pre-acceptance delivery
  failure returns the complete input, and cleanup transfers the complete call/guard/driver/slot
  rather than status alone.
- Removed monolithic lifecycle and direct-TD runtime planning APIs are absent from public compile
  fixtures and production call sites.
- `PublisherSink` and the protocol-side emission adapter are absent from both concrete feature
  cells; all Producer publication reaches the WP-300 bounded emission state.
- Native collection tests use one selected root route and one driver, while missing or inexact
  capability returns no-compatible-form without Servient-side fan-out.
- The listed performance workloads satisfy their fixture-locked budgets and structural invariants
  through `tools/performance-harness`.
