# 0058 Host Route State Succession

Status: MIGRATED

Kind: real-target implementation counterexample

Priority: HIGH

Target: the public Host prepared -> active -> committed route-state carrier

## Scope and authority

The bounded real-target Zenoh Property Read probe required by workspace topic
0056 successfully implemented the application-static target path through real
Zenoh protocol I/O. While attempting the corresponding public Host authoring
path, it found a concrete ownership contradiction in the exact erased guard
succession API. This topic isolates that contradiction; it does not propose a
new Host/static kernel, change the frozen macro architecture, or admit WP-600
work.

The first application-static probe did not use a side table, leaked `Arc`,
private field access, or unsafe erasure to make the Host path appear
executable. The corrected paired probe applies the same constraints to the
public Host-erased path. Stable conclusions are migrated to the Binding SPI,
runtime-safety specification, state-machine registry, API ownership inventory,
WP-300, architecture gate, architecture governance, roadmap, code, and tests.

## Minimal counterexample

Real Zenoh preparation creates owned route state containing at least a
`zenoh::Session`, declared `Queryable`, protocol metadata, bounded ingress
state, correlation state, and a waker. The public Host guard accepts that state:

```rust
let prepared = HostPreparedRouteGuard::new(input, footprint, zenoh_state);
```

At activation, the binding has only two public operations relevant to that
state:

```rust
let zenoh_state = prepared.try_into_state::<ZenohRouteState>()?;
// HostActiveRouteGuard::new(prepared, zenoh_state) cannot compile:
// `prepared` was consumed by `try_into_state`.

let active = HostActiveRouteGuard::new(prepared, replacement_state);
// This compiles, but `HostActiveRouteGuard::new` discards and drops the
// prepared guard's erased `ZenohRouteState`.
```

The binding could copy the public parts of the original `PrepareInput` into its
private state and reconstruct a new `PrepareInput` after downcasting, or retain
an external state reference in a binding-owned side table. Both are workaround
shapes: the former duplicates and reconstructs the Core-owned carrier that the
guard claims to preserve, and the latter violates route-scoped ownership. The
probe constraints explicitly forbid both.

This conflicts with the specification statement that Host guards preserve the
complete preparation input while transferring private binding state exactly
once. `HostCommittedRouteGuard::try_state_pin_mut` is sufficient after commit,
but it does not solve prepared -> active or active -> committed succession.

## Real-target evidence retained outside this question

`protocol-bindings/protocols/zenoh/tests/target_property_read_feedback_probe.rs`
uses the public static route slot, where one owned `ZenohRouteState` remains in
place through prepare, readiness, activate, commit, accept, response, and
cleanup. That path therefore supplies the positive control: scoped artifact
metadata can naturally derive owned Zenoh route state without a hidden lease or
side table. The Host failure is specific to erased state succession, not to the
macro Planning/Binding/Servient ownership split.

## Decision

The counterexample holds. The exact falsified contract was not the three-stage
route machine or the macro ownership split. It was the public claim that a Host
guard preserved the complete preparation carrier and transferred private route
state exactly once, despite offering only these incompatible ownership paths:

- consuming `try_into_state` returned the state but destroyed access to the
  predecessor carrier, including the original `PrepareInput` and footprint;
- successor constructors accepted an independently supplied erased state and
  therefore could replace the predecessor state while dropping the state that
  the predecessor guard owned.

The correction keeps `HostPreparedRouteGuard`, `HostActiveRouteGuard`, and
`HostCommittedRouteGuard` as distinct linear stage owners. A Core-private
carrier is created once by preparation and owns the original `PrepareInput`,
immutable footprint, generation-bearing route identity, and one erased heap
allocation. Successor construction consumes only the predecessor guard and
moves the unchanged carrier. There is no consuming state extraction and no
replacement-state argument.

All three legal stages expose a type-checked borrowed mutable accessor for
matching `Unpin` state and a type-checked pinned mutable accessor for matching
state that may be `!Unpin`. Core alone projects the pinned carrier storage; a
binding receives a safe public borrow and needs no unsafe or private escape.
Failure, cancellation, and late-successor ownership retain the carrier. The
terminal cleanup or acknowledged residual owner disposes it exactly once.

The scoped-artifact rule remains unchanged. The binding validates the borrowed
immutable artifact during preparation and derives the values needed later into
the owned route-local state before the first protocol side effect. It neither
copies or reconstructs `PrepareInput` nor retains the artifact or a plan-set
lease. The footprint and state remain siblings in the same carrier rather than
independently reconstructible facts.

## Alternatives rejected

- Atomically extracting both carrier parts and concrete state still makes
  route state detachable and requires a public reassembly invariant. It offers
  replacement opportunities without adding a capability required by the real
  protocol.
- Collapsing the three guard types would remove useful phase ownership proofs
  and widen this bounded carrier correction into Servient lifecycle policy.
- A binding-owned route table or a strong shared `Arc` as primary ownership
  would move generation, liveness, and exactly-once release truth out of the
  guard. The probe permits only a non-owning `Weak` response alias that cannot
  keep the route alive.
- Unifying the Host and static lifecycle APIs would address mechanical callback
  duplication, not this ownership defect. The paired probe needs only shared
  protocol-local I/O helpers and does not duplicate Servient's normative
  transition policy, so a broad kernel refactor has no correction-sized
  justification.

## Constraints

- Preserve Servient-owned orchestration, publication, dispatch, and cleanup.
- Preserve immutable artifacts, scoped artifact access, generation identity,
  permit-gated acceptance, and explicit terminal cleanup.
- Do not make the binding retain a plan-set lease or route side table.
- Do not reconstruct `PrepareInput` inside an external binding merely to work
  around an erased-guard API.
- Do not expand the correction into WP-600 or general operation-family work.

## Migrated evidence and remaining boundary

Focused Core evidence keeps one `!Unpin`, non-`Clone` state allocation and one
footprint across prepared -> active -> committed, rejects a mismatched concrete
type, observes no predecessor-stage drop, and observes exactly one terminal
drop. The Servient Host fixtures now put their primary bounded ingress,
correlation, and cleanup state in the guard instead of a shared instrumentation
object. The real Zenoh probe pairs application-static and public Host-erased
success, readiness-failure, and pre-publication-cancellation scenarios. It
observes the same state address across successful Host stages, no early drop on
failure or cancellation, one terminal drop, and generation, permit,
correlation, response, and cleanup parity on the current Property Read overlap.

This clears the real-target prerequisite; it does not pass or implement the
aggregate Property Read gate. Multi-route resource modeling, complete
Zenoh/Tokio physical resource accounting, generic Host/static lifecycle
factoring, broad operation families, and WP-600 remain outside this correction.

Reopen the architecture rather than widening this correction if a legitimate
protocol implementation still requires binding-side primary state ownership,
carrier reconstruction, binding-side unsafe/private access, or a concrete
route-state type change between stages. None is required by the paired real
Zenoh evidence.
