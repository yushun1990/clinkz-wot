# 0058 Host Route State Succession

Status: DISCUSSING

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

`docs/spec/binding-spi.md` remains the active contract until this question is
decided and migrated. The probe did not use a side table, leaked `Arc`, private
field access, or unsafe erasure to make the Host path appear executable.

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

## Questions for correction

1. Should erased Host guards keep one state allocation in place across stage
   changes, matching `ServerRouteSlot`, or expose an atomic operation that
   returns both the complete carrier and its downcast state?
2. Are separate prepared/active/committed erased guard types still buying a
   useful ownership proof, or are they duplicating a state transition already
   owned by the Servient route machine?
3. Can the correction share a typed lifecycle kernel with the static surface
   without weakening object safety, pinning, cancellation settlement, cleanup
   transfer, or constrained-profile ownership?
4. Which `docs/api-ownership.csv`, Binding SPI, state-machine, work-package,
   compile-fail, and Host external-authoring evidence must be invalidated and
   replaced when the successor shape is selected?

## Constraints

- Preserve Servient-owned orchestration, publication, dispatch, and cleanup.
- Preserve immutable artifacts, scoped artifact access, generation identity,
  permit-gated acceptance, and explicit terminal cleanup.
- Do not make the binding retain a plan-set lease or route side table.
- Do not reconstruct `PrepareInput` inside an external binding merely to work
  around an erased-guard API.
- Do not expand the correction into WP-600 or general operation-family work.

## Decision boundary

The exact Host route-state succession surface is reopened. Aggregate Property
Read source admission should wait for a bounded correction and external Host
revalidation. The application-static real-target probe remains valid evidence
for the disjoint static carrier and macro ownership path; it is not evidence
that the current Host carrier is implementable.
