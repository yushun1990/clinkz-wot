# Property Read Architecture Gate

Machine-readable gate status, fixture roots, dependencies, and current
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
0058 is migrated. This correction satisfies the real-target prerequisite but
does not substitute for the aggregate result.

The aggregate fixture executes successfully at the two registered roots; the
gate remains `ready` until independent acceptance. The external
binding crate supplies one deterministic protocol-local ingress slot,
correlation owner, response delivery path, compiler, static server, Host
server, lifecycle guards, and cleanup implementation. Its ingress declaration
admits one item and 1,024 retained bytes at route, binding, and global scope.
The static queue, live correlation, and byte charge reside in the caller-owned
typed route state; the probe and binding keep only weak access to that owner.
The Host route owns the matching one-item channel receiver and retained-byte/
correlation state, while its protocol-facing sender enforces the same byte
ceiling before enqueue. Both owners release the charge only at response
settlement or terminal cleanup. The runner supplies only legal root inputs and
executes both representations through the production Planning and Servient
path. Its success cells prove one compiler-produced
Producer-route artifact and canonical reservation reaches preparation, one
request is retained across an exhausted handler budget, one production-created
handler context reaches exactly one callback, and one real application payload
is sealed and settled on the binding edge with its exact bytes, media type, and
accepted correlation. A paired invalid handler-origin success is sealed by Core
into exactly one deliverable validation failure on that original response
opportunity. Both cases prove one handler invocation, one response settlement,
and zero terminal route, request, response, artifact, and cleanup ownership in
both representations. The Host assertions also preserve one route-state
allocation and footprint through prepared, active, and committed ownership.

The aggregate negative cells prove that insufficient ingress item or byte
capacity at every route, binding, and global scope fails before buffering,
compiler-carrier checks, or binding preparation side effects. Incomplete
cleanup/resource policy has the same pre-side-effect result. Servient retains
committed accounts for the real plan/artifact, handler, route/readiness/
response, ingress, and Host/static storage. Every narrow Host call—prepare,
readiness, activation, commit, response delivery, abort, and shutdown—is
checked against the pre-admitted normal call ceiling before its first poll.
Host first entry also reserves a distinct recovery ceiling, bounded by the
applicable cleanup-item, cleanup-byte, cancel-buffer, transfer-byte, and Host
call limits. A call that misses normal admission may be retained or cancelled
only when its complete declared item/byte footprint fits that recovery
reservation. Fixture constructors only retain their complete input;
activation, commit, delivery, and cleanup mutation begins inside admitted call
execution. An oversized activation call that fits the separately admitted
recovery ceiling receives no operational poll, returns its unchanged
prepared-route carrier through cancellation, and reaches the normal abort
owner with no premature route-state drop. If the configured recovery ceiling
cannot even cover the registration's ordinary call declaration, first entry
fails before preparation construction, operational polling, cancellation
polling, or lifecycle mutation.

Call cancellation, later route rollback, cleanup-call cancellation, and a
named transfer owner reserve four distinct Host cleanup subjects with additive
item/byte/status/transfer capacity. The cancellation matrix stops
prepare, readiness, activate, and commit calls in both representations before
the stage-appropriate abort or shutdown. For every Host case, executable
evidence compares the reservation, first cause, operation, deadline, work,
and lifetime footprint before `Pending` with the terminal settlement. A
pending response-delivery cancellation proves the independent
`CancelResponseDelivery` context and exactly one cancelled response
settlement before shutdown. A separately reserved cancellation path for a
pending cleanup call proves both its original route-cleanup context and its
own cancellation context reach terminal settlement without one masking the
other. A delivery identity-validation error returns the complete response to
the call without consuming its cleanup context; a later cancellation poll
uses that same context without panic and reaches one terminal response
settlement.

`TransferRequired` is represented explicitly as a source-owned
`CleanupTransferEnvelope` containing the complete call and request. Servient
does not re-poll it while source-owned. Each Host `CleanupPhaseContext`
receives the named owner directly from Servient's separately admitted transfer
reservation, and the binding consumes that carrier into its request without
knowing Servient slot layout. The separately reserved named owner
checks the unchanged call footprint, request phase footprint, requested owner,
and durable-record capacity at the atomic `CleanupTransferTarget` boundary.
Only `CleanupTransferAcceptance::Accepted` moves the envelope into the
acknowledged state. Rejection returns the identical envelope and exact phase
context to the pre-reserved manual owner, which retains them across Pending or
callback error until terminal settlement instead of re-offering blindly.
Acknowledged lifecycle, response-delivery, and cleanup-call transfer cases
then progress the complete call to legal terminal settlement while retaining
the request and exact cleanup context across Pending and callback errors.
Readiness failure also
returns the real prepared route through abort or retrying abort cleanup.
Immediately before the first preparation call, Servient validates its private complete first-entry closure,
including the frozen plan-set lease, real artifact/reference, canonical route,
validated registration/profile cell, Thing/generation, exact handler coverage
and footprint, status and cleanup capacity, activation authority/accept lease,
and vacant request/response owners.

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
builder. The async/no-std compile cell constructs a real TD, external static
binding registration, handler registration, and production Static Servient;
it makes no executor-backed runtime claim.

## First-entry closure

Before the first binding preparation side effect, WP-400 must possess:

- the real Planning output and generation-bearing artifact reference;
- the compiler-produced Producer-route artifact and reservation identity;
- one complete, Core-validated binding registration;
- the produced Thing identity and generation;
- exact Property Read handler coverage and its declared footprint;
- committed resource, work, deadline, and status accounts;
- independent call-cancellation and route-rollback cleanup reservations, plus
  Host cleanup-call cancellation and named transfer-owner reservations whose
  admitted footprint covers every call they are allowed to retain;
- compiled-plan-set ownership and the matching plan-set lease; and
- the route, activation, acceptance, response, and cleanup ownership needed to
  roll back every later transition.

Tests may choose legal root inputs such as TD contents, initial generations,
mock protocol configuration, handler behavior, and resource policy. They may
not fabricate a logical plan, admitted artifact, route reservation, route key,
`PrepareInput`, handler context, response opportunity, or cleanup owner in
place of the production carrier under test.

## Aggregate fixture boundary

The gate manifest registers two aggregate roots:

- `property-read-binding`, containing deterministic protocol-local I/O and a
  complete external binding implementation; and
- `property-read-runner`, composing the TD, planner, registration, Servient,
  handler, request, response, and cleanup assertions.

The runner contains no constructor call for a logical plan, admitted artifact,
artifact reference, route reservation, route key, `PrepareInput`, handler
context, response opportunity, or cleanup reservation. An executable source
boundary check protects that negative claim. Those values are created only by
the mock compiler/binding where protocol ownership requires it or by the
production Planning, Core, and Servient path. Package-local crate tests remain
regression evidence and are not treated as the aggregate pass by themselves.

The response-sealing prerequisite is package-local executable evidence for the
existing narrow carrier: Core accepts only a payload-bearing `Ok` application
success, converts every other nominal success into a deliverable validation
error, and exposes no unvalidated success constructor. Static and Host
Servient scenarios deliver that validation failure once and complete response
and route cleanup. The aggregate fixture consumes that same boundary: its
external binding validates the full live route identity and correlation before
acceptance, observes the real successful payload and media type, and delivers
the sealed invalid-success failure exactly once without repeating Core's
handler-origin validation.

## Exclusions

This gate does not claim subscription/emission, broad handler families,
multi-route availability, production Zenoh-family maturity, Directory,
workload/performance closure, deployment, or release readiness. It also does
not turn package-local evidence into broad WP-100/WP-200/WP-300/WP-400
completion.
