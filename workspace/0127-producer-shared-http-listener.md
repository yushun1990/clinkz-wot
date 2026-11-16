# Independent Producer Things on shared native HTTP infrastructure

Status: MIGRATED

## Reassessment after reusable preparation

PR #283 separates invariant HTTP response/schema instructions from checked
per-Thing mappings. It leaves physical listeners independent. Code inspection
finds no one-Thing/one-listener rule in the current Producer SPI: SPI-NATIVE and
SPI-RETIRE explicitly permit shared infrastructure; `FiniteIngressBinding`
returns original candidates and exclusive replies without dispatch authority.
Complete final-TD qualification and publication gates already operate on
independent native participation. A single native driver can name several
Things, but that is not required to demonstrate independent publications.

The concrete restriction is `HttpFiniteBinding`: its configuration has one
instance key and its start directly binds an exclusive socket. Simply sharing
that driver through two `ComposedProducerExposure` objects would conflate
readiness, ingress gates, original membership, accepted work and retirement.
Those are actual restrictions of the existing single-participant runtime's
representation, not evidence that the Engine needs routing or server ownership.

## Alternatives and selection

| Alternative | Disposition |
| --- | --- |
| One shared complete prepared Thing | Reject: description coordinates, behavior and qualification remain independent |
| Share one runtime/publication object across Things | Reject: conflates ingress gates, original membership, work and close |
| Generalize the SPI with a universal server/session manager | Reject: existing native ownership and participation contracts already admit this topology |
| Add an Engine router or callback into Servient | Reject: native matching and listener ownership belong to HTTP |
| Require an application Runtime per Thing | Reject: no semantic or native requirement; existing typed runtime instances here are participation adapters |
| Add optional compatible HTTP listener participation below the existing SPI | Select: one native owner, independent route leases, existing complete qualification/publication |

`HttpFiniteListener` is a caller-provisioned native owner. Atomic route acquisition
refuses collisions across participants. Its bounded head preselection stays in
HTTP and hands a connection to only its original participant; the existing native
parser still performs full request checks. An accept-time lease horizon prevents
partial old input moving to a later registration without retaining retired Thing
backing in unclassified sockets. Sibling progress does not require a staged or
unread-result recipient to grant semantic authority.

Same-route replacement closes the old native lease before acquiring the route
again, retaining old result ownership through acknowledgement. Different routes
may stage concurrently with accepted old work. Seamless overlapping takeover of
one method/path has no proved need in this increment and would require an explicit
native handoff policy; no generic authority or implicit collision exception is
introduced. Last native participation release closes the physical listener.

The listener's retry Waker is an external callback and may synchronously reenter
native configuration. Complete the queued connection handoff and release the
listener guard before waking. Its existing idle self-wake remains an advisory
retry under caller-paced manual progress; it supplies neither socket readiness,
recipient wake distribution nor work/time credit. This local callback-boundary
correction requires no scheduler or Producer execution-capacity change.

## Migration and scope

Exact native compatibility/buffering/release policy belongs to the
[Binding contract](../docs/spec/binding-spi.md#independent-http-listener-participation).
The small runtime/API projections describe the affected ownership surface;
Planning, Core and Engine implementations remain unchanged. WP-600 and PLAN
record the scoped capability. [Executable evidence](../docs/evidence/producer-shared-http-listener.md)
owns the claims and limitations. No general Producer redesign, milestone/gate
closure, independent review or merge follows from this implementation.
