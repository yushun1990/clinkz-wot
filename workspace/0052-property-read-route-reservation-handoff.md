# 0052 Property Read Route-Reservation Handoff

Status: MIGRATED

Kind: cross-package successor-entry and compiler-metadata correction

Priority: CRITICAL PATH

Target: the canonical `RouteReservationIdentity` consumed when the first
WP-400 Property Read route is constructed

## Observation

The completed Producer-route Planning projection carries a real
`BindingArtifactRef` from the borrowed compiler of a complete WP-300
registration into the real Core `PrepareInput` type. Its fixture nevertheless
constructs `RouteReservationIdentity` directly from fixture constants after
Planning has completed. Repository search finds no implemented contributor,
support projection, compiler output, plan output, or artifact metadata from
which WP-400 can obtain that identity.

The narrow WP-300 slice intentionally excludes the broad
`ServerFormContributor` family. The current `BindingArtifact<A>` wrapper
retains compatibility, footprint, and payload only, while
`PlanBuildOutput<A>` retains logical plans, admitted artifact envelopes, and
compact references only. A future WP-400 fixture would therefore have to copy
the Producer-route fixture's constant or introduce a fixture-only adapter.
Either would hide the missing production handoff forbidden by D43 and the
successor-entry rule.

## Decision

Keep the completed Producer-route role/reference projection as a valid narrow
Planning claim, but revoke its projection that WP-400 candidate preparation is
fully released. Insert one exact Category C correction immediately after it:

`WP-200-PROPERTY-READ-ROUTE-RESERVATION-PROJECTION`.

The concrete binding compiler owns protocol-specific endpoint
canonicalization. For `BindingArtifactRole::ProducerRoute`, it returns the
pure, deterministic `RouteReservationIdentity` as Core-owned immutable
artifact metadata alongside its opaque payload. Core preserves the metadata
through static and host erasure and rejects a Producer-route artifact without
it or a non-route artifact that supplies it. Planning preserves the admitted
metadata in `PlanBuildOutput`; it does not hash a URI, restate a binding
configuration, or inspect a protocol payload. The Servient may later read the
admitted metadata by artifact slot when it constructs `BindingRouteKey`.

The change remains additive for existing artifact consumers: the established
three-part `BindingArtifact::into_parts` signature stays unchanged, while a
new `into_route_parts` consuming surface returns the optional reservation as
well. Core host erasure must use the complete surface so it cannot silently
drop the new metadata.

The route-reservation value creates no listener, lease, cleanup obligation, or
execution state. It is fixed-width Core wrapper metadata accounted with the
plan-set's structural metadata; `BindingArtifactFootprint` continues to measure
the binding-authored retained payload. Generated forms may later provide the same
identity through the frozen contributor contract; that broad path must agree
with the compiler result before freeze and is not implemented by this narrow
correction.

## Alternatives rejected

- Leaving the identity in the runner fixture is rejected because real WP-400
  source would have no corresponding production input.
- Hashing the resolved target in Planning or Servient is rejected because URI
  spelling is not a protocol's canonical collision domain or endpoint key.
- Adding the identity to `BindingRegistrationIdentity` is rejected because one
  registration may own multiple routes and endpoints.
- Implementing the complete `ServerFormContributor` surface is rejected here
  because generated forms, security/context merge, capability indexing, and
  collision-set validation have different blockers and broad WP-300 evidence.
- Hiding the identity only inside the opaque artifact payload is rejected
  because the Servient cannot inspect either a third-party static payload or a
  Core-erased host payload.
- Reopening the completed public Producer-route constructor tranche is
  rejected because its Planning source, role propagation, and reference
  evidence remain true; this correction changes different Core/compiler
  ownership, source paths, and completion evidence.

## Admission consequence

The correction changes a protocol-neutral public wrapper and a cross-package
identity invariant, so it requires a non-source candidate, independent review,
one exact pre-source admission checkpoint, and three-cell completion evidence.
Its product implementation is limited to `core/src/binding_compiler.rs` and
the existing Planning Property Read implementation/test path; its external
contract also updates the existing WP-300 mock compiler so the real complete
registration produces the metadata.

`WP-400-PROPERTY-READ-SERVIENT-SLICE` remains `planned`/`blocked`. Completion
and verified default-branch integration of this correction may release its
candidate/review preparation. Neither this topic nor the correction candidate
grants product-source authority.

## Migration

The decision is projected into the active Planning specification, primary
Producer data flow, WP-200 work package, Property Read architecture document
and manifest, work-package evidence index, candidate audit and executable
contracts, roadmap, and continuation state. This topic remains decision
history rather than normative authority.
