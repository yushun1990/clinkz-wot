# WP-200 Property-Read Route-Reservation Projection Entry Audit

Status: Review pending

Design revision: v5.0

Admission scope: `WP-200-PROPERTY-READ-ROUTE-RESERVATION-PROJECTION`

Verdict: Candidate ready for independent review

## Finding and evidence truth

The completed `WP-200-PROPERTY-READ-PRODUCER-ROUTE-PROJECTION` correctly
publishes a bounded Producer-route planner and carries its real artifact
reference into the real WP-300 preparation type. It does not publish the
canonical endpoint reservation identity needed to construct the accompanying
`BindingRouteKey`. Its completion fixture currently supplies that identity
with direct fixture constants, and no implemented production boundary can
supply the same value to WP-400.

This finding leaves the completed role/reference projection intact but revokes
its downstream-release projection. WP-400 remains blocked behind the exact
correction registered here.

## Selected public boundary

The proposed addition extends the existing Core-owned `BindingArtifact<A>`
wrapper with one optional, private route-reservation field and exactly these
public operations:

```rust
impl<A> BindingArtifact<A> {
    pub const fn producer_route(
        compatibility: BindingArtifactCompatibility,
        footprint: BindingArtifactFootprint,
        reservation: RouteReservationIdentity,
        payload: A,
    ) -> Self;

    pub const fn route_reservation(&self) -> Option<RouteReservationIdentity>;

    pub fn into_route_parts(
        self,
    ) -> (
        BindingArtifactCompatibility,
        BindingArtifactFootprint,
        Option<RouteReservationIdentity>,
        A,
    );
}
```

`BindingArtifact::new` remains the constructor for artifacts without a route
reservation, and the existing three-part `into_parts` surface remains
unchanged for compatibility. Callers that must retain the added structural
metadata use `into_route_parts`; host erasure uses that complete consuming
surface. `BindingArtifactEnvelope::try_new` validates the artifact role
against the optional metadata and adds two ownership-preserving rejection
classes: `MissingRouteReservation` and `UnexpectedRouteReservation`. The
envelope exposes the admitted reservation through a read-only accessor so a
Servient plan-set owner need not inspect a typed or erased protocol payload.
The changed public API-item set is exactly `BindingArtifact`,
`BindingArtifactEnvelope`, and `BindingArtifactRejectionReason`; the identity,
role, reference, route-key, compiler-registration, plan-output, preparation,
and reservation wrapper types are reused unchanged.

Host erasure preserves the reservation exactly while erasing only the opaque
payload. A consuming mismatch continues to return the complete artifact.

## Ownership and resource boundary

The concrete compiler derives the canonical collision domain and endpoint key
from the already resolved logical plan plus immutable local binding
configuration. It performs no protocol I/O and creates no external lease. Core
owns the protocol-neutral wrapper and role validation. Planning coordinates
the compiler and retains the admitted envelope without interpreting the
identity. WP-400 later owns local route-capacity reservation and
`BindingRouteKey` construction.

The reservation is fixed-width Core wrapper metadata accounted with the
plan-set's structural metadata. `BindingArtifactFootprint` continues to measure
binding-authored retained payload. This candidate adds no resource kind,
executor work, cursor state, external cleanup, or runtime fallback.

## Implementation topology

After independent review and a combined pre-source admission checkpoint, the
only permitted product implementation paths are:

- `core/src/binding_compiler.rs`;
- `planning/src/property_read.rs`.

The same implementation checkpoint also updates the registered external mock
compiler at
`tools/compile-contracts/wp300-property-read-binding-slice/src/lib.rs`; that
support path is not product authority. Any other product-source path revokes
admission pending an intersecting impact review.

## Executable contract

`tools/compile-contracts/wp200-property-read-route-reservation/` stays outside
the workspace member list. It consumes the real TD builder, the real borrowed
compiler projection from the complete WP-300 static registration, the real
Producer-route planner, the real admitted artifact envelope/reference, and the
real WP-300 preparation boundary.

Its runtime proof must obtain the reservation only from the admitted envelope,
end all TD and compiler borrows, construct `BindingRouteKey`, and start the
real mock server. Direct construction of `RouteReservationIdentity` or either
component identity is forbidden in the runner fixture. The compiler-owning
WP-300 fixture may construct the deterministic mock protocol identity.

The contract covers no-default, async-no-std, and std compilation plus a std
runtime test. The executable schema rejects missing Producer-route metadata,
unexpected Consumer metadata, loss through host erasure, and fixture-side
identity construction. The completion check
must currently fail exactly because the reviewed Core artifact constructor is
absent.

## Exact exclusions

This correction does not:

- implement `ServerFormContributor`, capability indexes, generated forms, or
  full collision-set validation;
- derive protocol collision identity in Planning or Servient;
- put reservation identity in a registration shared by multiple routes;
- open a listener, reserve an external endpoint, or add cleanup state during
  compilation;
- add Consumer, subscription, Producer-publication, fallback, or lazy behavior;
- add Servient registry, publication, dispatch, or lifecycle source;
- create either Property Read architecture fixture root; or
- implement or claim a production protocol or Zenoh behavior.

## Candidate and review topology

The immutable candidate base is reconciliation checkpoint
`5a53a82a5d68a336b56b19e2e8f4c27f87492731`, whose parent is verified fetched
default merge `30485b1a51470f328e79453ba0e82e3358c14f79`. The candidate changes only
the exact paths registered in the Property Read gate and contains no product
source.

Independent review must reconstruct that single-child candidate, run all
registered prechecks, simulate the exact three-path implementation transition,
and mutation-test at least:

- missing `ProducerRoute` reservation metadata;
- reservation metadata on `ConsumerCall`;
- loss of metadata through host erasure or `into_route_parts`;
- caller- or runner-created collision domain or endpoint key;
- Planning/Servient URI hashing or opaque-payload inspection;
- product source outside the exact two-path boundary;
- support source outside the one registered WP-300 compiler path; and
- premature WP-400 or architecture-fixture source.

Review writes one new attestation and registry row. The admission checkpoint
then changes only the registered governance/state paths, binds the fetched
review integration as `admission_base_ref`, and changes this tranche to
`in-progress`/`approved`. No source is admitted before that checkpoint.
