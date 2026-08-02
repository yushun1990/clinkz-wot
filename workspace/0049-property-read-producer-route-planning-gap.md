# 0049 Property Read Producer-Route Planning Reachability Gap

Status: MIGRATED

Kind: intersecting planning/binding handoff and completion-evidence investigation

Priority: CRITICAL PATH

Target: the exact Property Read planning output consumed by the completed
Producer server registration before the WP-400 Servient slice may begin

## Observation

The completed WP-200 Property Read algorithm in
`planning/src/property_read.rs` is private and constructs every
`BindingCompilerInput` and `BindingArtifactIdentity` with
`BindingArtifactRole::ConsumerCall`. The completed WP-300 registration
advertises only Producer Property Read server execution, and its route
preparation input consumes a `BindingArtifactRef` for the prepared route.

Repository search finds no target-source or external-fixture construction of
`BindingArtifactRole::ProducerRoute` and no real `PrepareInput::new` call.
The WP-200 completion fixture exercises compiler components with
`ConsumerCall`; the WP-300 fixture constructs registration and lifecycle
values without consuming a real plan output. Both package-local claims remain
valid, but their Producer-route handoff is unreachable.

The architecture gate forbids the future WP-400 fixture from synthesizing a
logical plan, binding artifact, or route guard. A fixture-local replacement
would therefore hide rather than close the gap.

## Decision

Keep the completed Consumer-call plan slice and completed Producer server
registration intact. Insert one exact corrective tranche between WP-300 and
WP-400:

`WP-200-PROPERTY-READ-PRODUCER-ROUTE-PROJECTION`.

The correction is owned by Planning, depends on the completed WP-300 slice,
and adds only:

- a public bounded `PropertyReadPlanCompiler` constructor for the
  `ProducerRoute` role;
- a public opaque `PropertyReadBuildCursor<C, A>` required by the existing
  `PlanCompiler` associated-type contract; and
- support for borrowing the compiler projection of a complete static or host
  registration without moving or cloning the registration.

The constructor consumes one `BindingRegistrationIdentity`; it does not ask a
caller to restate binding id, generation, configuration, or compatibility.
No public arbitrary-role constructor is admitted. Consumer subscription,
Producer publication, fallback/lazy planning, Servient lifecycle, concrete
protocol behavior, and broad planning remain excluded.

The correction completion fixture must use the real WP-300 static complete
registration, the real TD and Planning implementation, and the real Core
`PrepareInput`. It must prove that the compiler sees `ProducerRoute`, the
artifact envelope and compact reference retain `ProducerRoute`, registration
identity is preserved, TD/registration borrows end before route preparation,
zero budget makes no progress, and the resulting reference starts real route
preparation.

## Admission consequence

This is Category C because it adds a public cross-crate planning entry point
and corrects a cross-package ownership handoff. Its non-source candidate,
negative mutations, exact two-path source topology, and next-state transition
require independent review before implementation.

WP-400 remains `planned`/`blocked`. Completion of WP-300 releases this
correction; completion of this correction releases the exact WP-400 Servient
candidate. Broad package dependencies and the aggregate gate remain
unchanged.

## Migration

The decision is projected into the planning specification, WP-200 work
package, Property Read gate document and manifest, roadmap, ownership matrix,
contract fixtures, checks, audit, and continuation state in the same
non-source candidate. This topic grants no product-source authority.
