# WP-400 Property-Read Servient Slice Entry Audit

Status: Passed

Design revision: v5.0

Admission scope: `WP-400-PROPERTY-READ-SERVIENT-SLICE`

Verdict: Implementation-ready

## Selected boundary

The verified route-reservation projection supplies the final missing upstream
output. This candidate composes, but does not reinterpret, the completed
Property Read handler, plan, binding, Producer-route, and reservation
contracts. The first legal binding side effect remains exactly:

```text
expose:Draft:expose->Preparing
  binding-route:Absent:begin_prepare->Preparing
```

Before that nested transition, Servient product code must own a complete
registration snapshot, compiled plan-set record and lease, produced Thing and
generation, matching handler view, route record, activation record, and every
route/ingress/guard/readiness/accept/response/status/cleanup reservation. Only
then may it derive `BindingRouteKey` and `PrepareInput` from the real plan
output and call the real binding registration.

No runner, test adapter, Planning code, or opaque binding payload may assemble
those WP-400-owned values.

## Active v5 public boundary

The narrow slice uses the frozen Servient types and exact lifecycle methods;
it adds no separate Property Read facade:

- `StaticServientBuilder::binding_registration` consumes one complete
  `StaticBindingRegistration<B>`; its Property Read handler input is one real
  `StaticHandlerRegistration<'h, H>` where `H: ReadPropertyHandler`.
  `StaticServient::begin_destroy` closes route selection before later
  caller-budgeted cleanup steps.
- `ServientBuilder::binding_registration` consumes one complete
  `HostBindingRegistration`. The existing zero-argument
  `ServientBuilder::new()` remains source-compatible;
  `ServientBuilder::resource_limits` installs the explicit narrow resource
  policy before `build`, and `build` retains its existing
  `ServientResult<Servient>` result. `Servient::produce_td`,
  `ExposedThingHandle::set_read_property_handler`, `begin_expose`, and
  `begin_destroy` form the exact host lifecycle boundary. WP-400 privately
  owns the necessary synchronous-handler erasure in
  `PropertyReadHandlerRecord`.
- `StaticServient::step` and `Servient::step` each accept an explicit
  `Context` and `WorkBudget` and return `StepStatus<()>`. The async/no-std
  projection is an adapter over the same bounded step contract and introduces
  no executor dependency.
- `ExposeState` and `CompiledPlanSetState` are read-only lifecycle views.

The v4.9 broad work-package input named `StepStatus<RuntimeEvent>` and
`HostHandlerRegistration`. Under ADR-0018, `CAP-STATUS-001` and
`API-SURFACE-001` are inactive domain-entry or retired authority, and neither
corresponding source type exists. This narrow active-v5 candidate therefore
does not activate either broad family. A later broad WP-300/WP-400 review may
replace the unit step value with an admitted status event and may introduce a
reusable Core host-handler registration. The narrow runtime exposes state and
observable fixture I/O, not a status-stream claim.

## Complete first-entry provenance

The candidate closes all nine D46 rows before any binding side effect:

| Input | Production owner/carrier | Required validation |
| --- | --- | --- |
| Producer logical route | `PlanBuildOutput.logical_plans` and `artifact_refs` | Selected plan, artifact, binding, and plan-set generations agree |
| Producer artifact metadata | real `BindingArtifactEnvelope` | Role is Producer-route and compiler-owned reservation is present |
| Complete binding registration | frozen `StaticBindingRegistration` or `HostBindingRegistration` snapshot | identity, compiler, server, resources, ingress, and status are complete |
| Produced Thing/generation | `ExposedThingRecord` and `ThingSlotId` | TD is read only during planning; generation is retained thereafter |
| Property Read handler coverage | `StaticHandlerRegistration` or private `PropertyReadHandlerRecord` | target and handler generation cover the selected plan exactly |
| Servient admission policy | builder-owned `ResourceLimits`, `Deadline`, clock identity, and caller-supplied `WorkBudget` | every required capacity is reserved before preparation |
| Compiled plan-set ownership | `CompiledPlanSetRecord` plus non-`Clone` `PlanSetLease` | plan set is frozen and pinned to the produced generation |
| Route preparation assembly | `BindingRouteRecord`, `BindingRouteKey`, and `PrepareInput` | route key uses only admitted artifact metadata and Servient-owned generations |
| Activation and cleanup ownership | `ServingActivationRecord`, `ServingActivationAuthority`, `RouteAcceptLease`, and cleanup reservations | authority remains unpublished until every route is committed closed |

The only legal semantic roots are the TD, complete binding configuration,
root generation/slot input for the application-static profile, handler result,
and explicit resource/time policy. Host slot allocation is Servient-owned.
Prepared/readiness/active/committed guards, accepted requests, security
results, handler context/input/output, response opportunity, publication, and
drain state arise only after the first entry.

## Runtime ownership and linearization

One narrow produced generation owns one immutable plan set, one route, one
handler, and one `ServingActivationAuthority`. Exposure proceeds through the
registered `compiled-plan-set`, `expose`, and `binding-route` machines.
Publication occurs only at `expose:Committing:publish->Serving`, after the
route returns a committed-closed guard. The authority and route accept lease
become selectable atomically with that publication.

Each step claims the route lease, creates the borrowed activation permit,
polls the binding outside any Servient mutable-state boundary, and transfers a
complete `RouteInboundRequest` into one `InFlightRecord`. If the caller's
`HandlerSteps` allowance is exhausted after acceptance, that complete request
and its unique response opportunity remain in the record for a later step;
they are never dropped or re-polled from the binding. Servient constructs
`HandlerContext` from retained production identities, calls the selected
handler exactly once, and consumes the request's unique response opportunity
into `RouteInboundResponse`. Delivery acknowledgement releases the in-flight
and response reservations exactly once.

Destroy/deactivation first closes authority selection, rejects later accepts,
drives route shutdown, retains every incomplete cleanup object, and releases
route, request, response, plan-set, and cleanup counts only at their terminal
boundaries. This narrow slice makes no drop-transfer or external cleanup
executor claim.

## Implementation topology

The approved independent review and this distinct pre-source admission
checkpoint permit only these product implementation paths:

- `servient/Cargo.toml`;
- `servient/src/lib.rs`;
- `servient/src/builder.rs`;
- `servient/src/handle.rs`;
- `servient/src/registry.rs`;
- `servient/src/servient.rs`; and
- new `servient/src/property_read.rs`.

The immediate implementation child may also change exactly three non-product
support paths:

- root `Cargo.lock`, which must record the new Servient Foundation/Planning
  dependency edges;
- `tools/compile-contracts/wp400-property-read-servient-slice/Cargo.lock`,
  which must record the same path-dependency edge for the external contract;
  and
- `tools/compile-contracts/wp300-property-read-binding-slice/src/lib.rs`, so
  the existing complete mock registration can inject one protocol-neutral
  request and retain one delivered response in allowed deterministic I/O and
  instrumentation state.

The mock support object may create protocol frames, but it may not create a
plan, artifact, route key, `PrepareInput`, activation permit, handler value,
response opportunity, or cleanup owner. Both lockfile changes are metadata
closure for `--locked`; neither grants implementation authority.

Any other product or support source path revokes admission pending an
intersecting impact review.

## Executable contract

`tools/compile-contracts/wp400-property-read-servient-slice/` remains outside
the workspace. Its no-default manual cell, std host cell, and async/no-std
compile cell must enter the real Servient product API. They consume the real
TD, complete WP-300 mock registration, real handler registration, explicit
resource/time policy, and a caller-supplied budget/waker only.

The no-default static runtime cell and std host runtime cell must prove:

- TD access ends after planning and no runtime TD selector is used;
- one real compiler output reaches the first legal Servient route entry;
- no accept occurs before Servient publication;
- the route and handler are selected by Servient product code;
- the handler is called once with protocol-neutral input;
- the response opportunity is consumed once and the mock binding observes the
  exact output;
- an accepted request survives an exhausted handler-step allowance and resumes
  exactly once with a later allowance;
- deactivation rejects a later request; and
- all narrow route, in-flight, response, and cleanup counts return to zero.

The compile projection must prove that the mock binding has no Servient
dependency, all registration and runner inputs use public production types,
the activation permit cannot be constructed or retained by the runner, and
the async/no-std surface adds no executor.

The completion check must currently fail exactly because
`clinkz-wot-servient` has no Planning dependency or reviewed Property Read
module. Candidate preparation creates neither final architecture fixture root.

## Independent negative evidence

Independent review simulated the exact product/support transition and rejected
every registered mutation:

1. fixture-restated artifact or reservation;
2. dropped or mismatched produced, plan-set, binding, plan, or route generation;
3. Planning- or Servient-side reservation reconstruction;
4. host-erasure loss of artifact metadata;
5. unrelated reservation paired with the real artifact;
6. partial or bare binding registration;
7. binding preparation before resource and cleanup reservation; and
8. host/static semantic divergence.

It also rejected:

9. replacement of the existing zero-argument host builder constructor by the
   proposed one-argument spelling; and
10. a Servient dependency transition that omits either affected lockfile; and
11. a host resource-policy root that assumes a nonexistent
    `ResourceLimits::default()` implementation instead of selecting an
    existing named Foundation profile.
12. a host runner budget that names nonexistent `WorkClass::HandlerCalls`
    instead of the frozen `WorkClass::HandlerSteps` variant.
13. a portable contract root that imports std-only
    `HostBindingRegistration` without `#[cfg(feature = "std")]`, making both
    the no-default and async/no-std author cells structurally uncompilable.
14. an application-static boundary with no callable deactivation/cleanup
    entry despite claiming host/static lifecycle parity;
15. loss of an already accepted request and its response opportunity when the
    caller supplies no remaining `HandlerSteps` allowance;
16. replacement of the existing `ServientBuilder::build` result from
    `ServientResult<Servient>` to `CoreResult<Servient>`; and
17. omission from API ownership or tranche scope of the exact static handler,
    static destroy, host produce, host expose, or host destroy method required
    by the runner contract.

Review additionally rejected premature publication, accept without the unique
route lease/permit, direct runner-created handler context or response
opportunity, handler double-call, response double-delivery, cleanup-object
loss, source outside the exact path sets, and creation of either final
architecture fixture root.

## Exact exclusions

This candidate does not implement or claim:

- async or step handler families;
- Consumer calls, subscriptions, or Producer emissions;
- multiple Things, multiple routes, fairness, sharding, or workload bounds;
- dynamic binding registration, plan regeneration, lazy artifacts, fallback,
  forms contribution, or degraded publication;
- a runtime event/status stream;
- broad cleanup executor or drop transfer;
- a production protocol or Zenoh behavior; or
- completion of `PROPERTY-READ-ARCHITECTURE`.

## Candidate topology

Fetched, merge-validated default revision
`fcce9e69036459506a163ac73ef5542f92e5eb7f` is the original candidate base.
Exact 18-path non-source child
`2d63e151ac6f89ef294c089d5f48917e8e324773` passed the candidate checker,
aggregate design/evidence suite, locked workspace, 21-cell feature matrix, and
pull-request #22 validation run `31353096175`.

Independent next-state reconstruction nevertheless rejected that original
candidate before attestation:

- its host fixture replaced the existing zero-argument
  `ServientBuilder::new()` with `ServientBuilder::new(limits)`. Rust cannot
  overload inherent associated functions by arity, and 28 current
  valid source/test/example call sites use the zero-argument constructor; and
- adding the completion check's mandatory Foundation and Planning manifest
  dependencies makes both the root and external-contract lockfiles stale,
  while neither lockfile was registered in the proposed implementation
  topology. An isolated `cargo test --locked --no-run` stopped on that exact
  lockfile drift before source compilation.

The first corrective candidate is exact commit
`4456632367069fb5cdd20dd51aeade1035e3768b`, the 11-path single child of
`2d63e151ac6f89ef294c089d5f48917e8e324773`:

- `PLAN.md`;
- `PROJECT_STATE.md`;
- `docs/api-ownership.csv`;
- this audit;
- `docs/spec/v5-artifact-carry-forward.toml`;
- `docs/work-packages/property-read-architecture-gate.toml`;
- `tools/check-wp400-property-read-servient-slice-entry.sh`;
- `tools/check-wp400-property-read-servient-slice.sh`;
- `tools/compile-contracts/wp400-property-read-servient-slice/src/lib.rs`;
- `tools/design-check/src/main.rs`; and
- `tools/design-check/tests/wp400_property_read_servient_schema.rs`.

It preserves all nine D46 provenance rows, all seven product implementation
paths, and source absence. It changes the host fixture to zero-argument
construction followed by the frozen `resource_limits` method, registers the
two lockfiles as implementation support metadata, and adds both rejection
classes to executable schema evidence. Pull-request #22 exact-head validation
run `31354359944` passed for that immutable commit.

Independent reconstruction rejected that first correction before source
simulation. The std runtime cell calls `ResourceLimits::default()`, but the
frozen Foundation type has no `Default` implementation. A minimal external
no-std compile proof fails with `E0599`; replacing the expression with
`GatewayDefaultV1::LIMITS.clone()` compiles against the same Foundation
revision. The registered implementation paths cannot add the missing trait
implementation to Foundation, and doing so would silently invent a default
policy beyond this tranche's authority.

The second corrective candidate is exact commit
`8ce5b4426921f7343a298a5910b40fa5c87942d2`, the nine-path single child of
`4456632367069fb5cdd20dd51aeade1035e3768b`:

- `PLAN.md`;
- `PROJECT_STATE.md`;
- this audit;
- `docs/spec/v5-artifact-carry-forward.toml`;
- `docs/work-packages/property-read-architecture-gate.toml`;
- `tools/check-wp400-property-read-servient-slice-entry.sh`;
- `tools/compile-contracts/wp400-property-read-servient-slice/tests/host.rs`;
- `tools/design-check/src/main.rs`; and
- `tools/design-check/tests/wp400_property_read_servient_schema.rs`.

It changes only the host test's legal resource-policy root to the existing
Gateway profile, registers `GatewayDefaultV1` and `StaticResourceProfile` as
reused public inputs, and adds that precise rejection class to the entry/schema
checks. It preserves the first correction's compatible builder API, two
lockfile paths, all nine D46 rows, seven product paths, three support paths,
and source absence. Pull-request #22 exact-head validation run `31356400537`
passed for that immutable commit.

Independent reconstruction rejected the second correction before attestation.
Its host runner assigns handler work through `WorkClass::HandlerCalls`, but
the frozen Foundation enum has no such variant; the implemented public value
is `WorkClass::HandlerSteps`. A minimal external no-std compile proof of that
exact enum reference stops with `E0599` and recommends `HandlerSteps`. No choice inside the seven
registered Servient implementation paths can make a nonexistent public enum
variant compile, and adding a Foundation variant would be an out-of-scope
semantic expansion rather than a correction.

The third corrective candidate is the exact nine-path single child of
`8ce5b4426921f7343a298a5910b40fa5c87942d2`:

- `PLAN.md`;
- `PROJECT_STATE.md`;
- this audit;
- `docs/spec/v5-artifact-carry-forward.toml`;
- `docs/work-packages/property-read-architecture-gate.toml`;
- `tools/check-wp400-property-read-servient-slice-entry.sh`;
- `tools/compile-contracts/wp400-property-read-servient-slice/tests/host.rs`;
- `tools/design-check/src/main.rs`; and
- `tools/design-check/tests/wp400_property_read_servient_schema.rs`.

It replaces only the invalid work-class spelling with the frozen
`HandlerSteps` variant, registers `WorkClass` as a reused public input, and
adds the exact rejection to the entry/schema checks. It preserves every prior
constructor, resource-policy, lockfile, provenance, source-path, and
absent-source constraint. Its immutable object id is
`129af4349dbd29d0ca3212646020f7dfe59baf47`; pull-request #22 exact-head
validation run `31358644443` passed.

Independent reconstruction then installed only the registered seven product
and three support paths in a fresh detached checkout. The std host cell
compiled and completed one request, one handler call, one response, and full
route cleanup. Both portable cells stopped earlier with `E0432`, however,
because the external contract root imported std-only
`HostBindingRegistration` unconditionally. Core intentionally exports that
type only behind `feature = "std"`; no allowed Servient or support
implementation path can change the contract root or make a host-erased type
portable. The third correction is therefore rejected before attestation.

The fourth corrective candidate is the exact nine-path single child of
`129af4349dbd29d0ca3212646020f7dfe59baf47`:

- `PLAN.md`;
- `PROJECT_STATE.md`;
- this audit;
- `docs/spec/v5-artifact-carry-forward.toml`;
- `docs/work-packages/property-read-architecture-gate.toml`;
- `tools/check-wp400-property-read-servient-slice-entry.sh`;
- `tools/compile-contracts/wp400-property-read-servient-slice/src/lib.rs`;
- `tools/design-check/src/main.rs`; and
- `tools/design-check/tests/wp400_property_read_servient_schema.rs`.

It moves `HandlerFootprint` and `HostBindingRegistration` into an exact
std-gated import while leaving the static imports available in all cells, and
adds the thirteenth executable rejection class. No product or support source
is admitted. Its immutable object id is
`43db15247279660ef910fdd13757e2767801fd94`; pull-request #22 exact-head
validation run `31361336829` passed.

Independent reconstruction installed only the registered seven product and
three support paths and completed the entire WP-400 completion matrix in an
isolated checkout. Source-level negative evidence still rejected the fourth
correction before attestation:

- `StaticServient` exposed only `step`; an external deactivation proof stopped
  with `E0599` because `begin_destroy` was unreachable, while the candidate
  requires the no-default runtime cell to close its route and cleanup counts;
- after a binding returned an owned `RouteInboundRequest`, exhausting
  `HandlerSteps` dropped the request and response opportunity. A later funded
  step observed zero handler calls and zero delivered responses, leaving the
  binding's in-flight count live; and
- the simulated source could compile only by changing the existing
  `ServientBuilder::build` result type and adding required public lifecycle
  methods that were absent from API ownership and tranche scope.

The fifth corrective candidate is the exact twelve-path single child of
`43db15247279660ef910fdd13757e2767801fd94`:

- `PLAN.md`;
- `PROJECT_STATE.md`;
- `docs/api-ownership.csv`;
- this audit;
- `docs/spec/v5-artifact-carry-forward.toml`;
- `docs/work-packages/property-read-architecture-gate.toml`;
- `tools/check-wp400-property-read-servient-slice-entry.sh`;
- `tools/check-wp400-property-read-servient-slice.sh`;
- `tools/compile-contracts/wp400-property-read-servient-slice/src/lib.rs`;
- `tools/compile-contracts/wp400-property-read-servient-slice/tests/host.rs`;
- `tools/design-check/src/main.rs`; and
- `tools/design-check/tests/wp400_property_read_servient_schema.rs`.

It registers the five previously unowned public methods, preserves the host
builder's application-facing result type, adds a real no-default static
runtime test, and requires both runtime profiles to retain an accepted request
across handler-budget exhaustion before completing response delivery and
deactivation. It adds four exact rejection classes, bringing the executable
schema to one positive closure plus seventeen negative mutations. No product
or implementation-support source is admitted; the gate keeps
`candidate_ref = "resolved-by-review-attestation"`.

Independent root-session review bound exact fifth correction
`43e2669117f6e2a550e862837f3ab55cc5f956fb`, reconstructed all six candidates,
executed the eleven registered prechecks, exercised the no-default static and
std host runtime cells plus the async/no-std compile projection, and rejected
all seventeen registered mutations in an isolated checkout. Exact three-path
review checkpoint `f04814c31a5a5f0ba4144d3bedcb3956ad00ab44` records that
result; exact three-path checkpoint
`d5acc49c84302f33cc18fd60c2a6e3a544e23529` registers its immutable reference.

Source-level probes additionally retained every returned guard, request,
response, cleanup input, first cause, and residual record across readiness,
abort, shutdown, cancellation-before-publication, exhausted-handler, and
cleanup-retry paths. Each static and host failure path returned route,
in-flight, response, and cleanup counts to zero. The transient probes were
removed after execution, and no simulated source entered the review branch.

Pull request #22 integrated the exact candidate/review chain at merge
`1652976500941f0f61e7578391d8d2ce8fcee862`. Its first parent is fetched
default `fcce9e69036459506a163ac73ef5542f92e5eb7f`, its second parent is exact
PR head `d5acc49c84302f33cc18fd60c2a6e3a544e23529`, and its merge tree equals the
reviewed head tree. Exact-head pull-request validation run `31464618610` and
default-branch validation run `31465621460` both passed.

This combined pre-source checkpoint changes only `PLAN.md`,
`PROJECT_STATE.md`, this audit,
`docs/spec/v5-artifact-carry-forward.toml`, and
`docs/work-packages/property-read-architecture-gate.toml`. It binds exact
fetched/default-validated merge `1652976500941f0f61e7578391d8d2ce8fcee862`
as `admission_base_ref` and changes this tranche to
`in-progress`/`approved`. All seven product and three support implementation
paths remain unchanged at this checkpoint.
