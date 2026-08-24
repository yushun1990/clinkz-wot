# Property Read Architecture Gate

Machine-readable gate status, planned fixture roots, dependencies, and current
evidence paths are defined only in
[`property-read-architecture-gate.toml`](property-read-architecture-gate.toml).

## Purpose

This gate is the first executable proof that the current Planning, Core
handler, Protocol Binding, and Servient boundaries compose without an
ownership shortcut. It is architecture evidence, not a production-protocol,
multi-route, workload, deployment, or release claim.

The package dependency is:

```text
WP-100 handler
    -> WP-200 property-read planning
    -> WP-300 complete binding registration and server lifecycle
    -> WP-200 Producer-route artifact and reservation projection
    -> WP-400 Servient lifecycle
    -> real-target Zenoh feedback probe
    -> aggregate Property Read architecture fixture
```

The real-target probe precedes the aggregate mock gate because it supplies
architecture feedback with actual protocol I/O. It does not itself claim
WP-600 progress.

## Current technical evidence

Completed slice history is retained by Git. Current regression ownership is
instead executable at the crate that can be broken:

- Core tests own the synchronous handler contract, bounded values, validated
  context, Property Read handler-result sealing, and compile-fail public type
  boundaries.
- Planning tests own immutable Property Read output, static/host parity,
  Producer-route identity, compiler-owned route reservation, and the real
  output-to-`PrepareInput` handoff.
- Servient tests own external binding authoring, preparation/readiness/
  activation/commit, permit-gated acceptance, retained requests, response
  delivery, destroy, cancellation input return, and cleanup.
- Core compiler tests own typed and erased preservation of Producer-route
  reservation metadata.

The exact evidence paths are registered in the gate manifest. They run through
workspace tests and focused no-default/default test cells.

The prerequisite real-target feedback is executable at
`protocol-bindings/protocols/zenoh/tests/target_property_read_feedback_probe.rs`.
It uses the actual Zenoh Rust SDK and loopback TCP for three varied
Thing/property/form shapes in both application-static and public Host-erased
representations. Each representation reaches a real handler and requester
response and completes terminal drain, readiness-failure rollback, and
pre-publication cancellation cleanup. Their paired assertions cover
generation, permit, correlation, response, and cleanup semantics. Its
technical disposition is recorded in `WP-300-bindings.md`.

The probe first falsified and then revalidated the Host prepared -> active ->
committed carrier. The corrected public surface moves one private carrier with
the original `PrepareInput`, footprint, generation identity, and non-`Clone`
Zenoh route state through the distinct stage guards. It offers only borrowed,
type-checked pinned shared state access and accepts no replacement successor
state. It exposes neither `&mut S` nor `Pin<&mut S>`; the real binding keeps
protocol mutation behind shared-state methods and cannot safely reproduce the
whole-state replacement counterexample. The binding uses no route table,
retained plan-set lease, private access, or unsafe erasure. Servient also keeps
the owned committed guard while `poll_accept` lends the binding only a shared
reference, closing safe whole-guard replacement or extraction. Workspace topic
0058 is migrated. This correction satisfies the real-target prerequisite; it
is not an aggregate gate pass. The aggregate gate remains `ready`, its planned
fixture roots remain absent, and no aggregate source has been admitted.

## Required scenario

Both application-static and host-erased cells exercise the same semantic path:

```text
Thing Description
  -> PropertyReadPlanCompiler
  -> immutable logical plan
  -> binding compiler
  -> Producer-route artifact with canonical reservation metadata
  -> complete binding registration
  -> prepared, ready, active, and committed route
  -> Servient publication
  -> permit-authorized request acceptance
  -> one ReadPropertyHandler call
  -> Core-sealed handler result on the original response opportunity
  -> one response delivery
  -> request and route cleanup
```

The static cell is manually progressed with caller-owned `Context` and
`WorkBudget`. The host cell uses the public erased registration and Servient
builder. Async/no-std is a compile cell unless an executor-backed runtime test
explicitly says otherwise.

## First-entry closure

Before the first binding preparation side effect, WP-400 must possess:

- the real Planning output and generation-bearing artifact reference;
- the compiler-produced Producer-route artifact and reservation identity;
- one complete, Core-validated binding registration;
- the produced Thing identity and generation;
- exact Property Read handler coverage and its declared footprint;
- explicit resource, work, deadline, status, and cleanup capacity;
- compiled-plan-set ownership and the matching plan-set lease; and
- the route, activation, acceptance, response, and cleanup ownership needed to
  roll back every later transition.

Tests may choose legal root inputs such as TD contents, initial generations,
mock protocol configuration, handler behavior, and resource policy. They may
not fabricate a logical plan, admitted artifact, route reservation, route key,
`PrepareInput`, handler context, response opportunity, or cleanup owner in
place of the production carrier under test.

## Aggregate fixture boundary

The gate manifest reserves two future aggregate roots:

- `property-read-binding`, containing deterministic protocol-local I/O and a
  complete external binding implementation; and
- `property-read-runner`, composing the TD, planner, registration, Servient,
  handler, request, response, and cleanup assertions.

Those roots remain absent until the aggregate scenario is separately admitted
and implemented. The real-target Host carrier prerequisite is satisfied;
package-local crate tests remain regression evidence after the aggregate
fixture is added.

The response-sealing prerequisite is package-local executable evidence for the
existing narrow carrier: Core accepts only a payload-bearing `Ok` application
success, converts every other nominal success into a deliverable validation
error, and exposes no unvalidated success constructor. Static and Host
Servient scenarios deliver that validation failure once and complete response
and route cleanup. This prerequisite does not implement either aggregate
fixture root or change the gate from `ready` to `passed`.

## Exclusions

This gate does not claim subscription/emission, broad handler families,
multi-route availability, production Zenoh-family maturity, Directory,
workload/performance closure, deployment, or release readiness. It also does
not turn package-local evidence into broad WP-100/WP-200/WP-300/WP-400
completion.
