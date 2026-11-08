# Security composition and qualified execution

Status: MIGRATED

## Independent boundary reassessment

Basis: master `3e28af8ae3e9475d710d1e6cba70c96604d3e284`, ADR-0025,
EXEC-SECURITY-001, PREP-SELECT/COVER/REUSE-001, SPI-NATIVE/INGRESS-001,
SAFE-AUTH/CANCEL/CLEANUP/PROFILE-001. PR #271's borrowed finite meaning and
caller-owned execution remain the portable boundary; this increment adds no
Embedded capability or runtime.

TD interpretation already resolves Form overrides and recursively exposes Combo
definitions. Planning's singleton restriction and Core's static scheme/parameter
pair are provisional implementation limits. They cannot distinguish a selected
Consumer conjunction from complete Producer qualification, carry variable scheme
operands, or establish that independent native and credential services agree.
The HTTP `basic` flag also conflates authenticated transport with one credential
scheme. Preserving these representations would make further support conditional
on single-scheme workarounds.

The W3C TD 1.1 Recommendation defines security lists as conjunctions; Combo
allOf requires every member, and oneOf permits choosing one member. Its
[security vocabulary](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#security-vocabulary-definition)
requires exactly one combination operator. A bounded projection must detect
cycles and invalid/empty combinations, preserve unsupported branches, and refuse
expansion overflow rather than silently drop alternatives. Schema oneOf's
exclusive validation semantics do not apply to authentication alternatives.

## Selected direction

Replace the singleton with owned, protocol-independent immutable composition
and borrowed inspection, without requiring Arc or threads. Preparation projects
complete effective requirements, then enumerates bounded complete conjunctions.
Uninterpreted leaves remain explicitly unavailable; they cannot become NoSec.
Consumer selection tries alternatives in declared order against the actual
credential service and native compiler, records the selected complete conjunction,
and repeats compatibility at instance bind. Reuse compares selected meaning;
credentials, audience, policy, authority and trust remain independent live owners.
Producer preparation and final qualification cover every advertised conjunction,
including unsupported alternatives. Failed qualification performs no native I/O.

Demonstrate Basic/header Authorization, API key/header X-Api-Key, both together,
NoSec, and nested alternatives on finite HTTPS Property Read/Write. Distinct
verifiers must establish the same Principal including claims for a conjunction;
do not synthesize identity or union privileges. Evidence presence selects a
complete native branch before verification. Invalid or extraneous evidence
cannot fall through to an anonymous or weaker branch. Multiple credentials
competing for one native placement refuse qualification.

Existing per-instance destination/operation/registration checks, non-reused
credential and policy invalidation revisions, fresh authorization, funded
verification, original reply ownership and final cleanup remain obligations.
Every verifier in a conjunction participates in work/readiness/revision checks.
Transport chain/endpoint verification precedes credential placement and is not
proof of application identity or permission.

## Rejected alternatives and falsification

Reject merely adding a Vec of scheme names, accepting independent scheme tags
as combination support, choosing alternatives independently in the compiler and
credential provider, accepting one supported Producer alternative, credential
storage in definitions, silent NoSec fallback, claim union, a universal manager,
and a new shared session/transport abstraction. Reject unbounded distributive
expansion and mandatory reference counting. Keep native placement and protocol
operation limits inside HTTP; OAuth/Digest/PSK/proxy/other placements remain
explicitly unsupported execution, not discarded semantics.

Real Engine Consumer/Producer TLS I/O must demonstrate branch selection,
conjunction failure when either component fails, authorization, conflicting
identities, instance/audience isolation, verifier and credential revocation,
callback-time policy invalidation, cancellation/late results and final original
resource release. Prior singleton-only negative tests must be revised where
their assumptions are intentionally superseded. No performance or production,
general scheme, milestone, or new constrained execution claim follows.


## Pre-implementation challenge and disposition

A fresh Max review independently reconstructed master and excluded concurrent
implementation edits. Its four material findings are incorporated: preserve
absent/empty Combo operators in TD, refuse indistinguishable native alternatives,
check static credential-header collisions, and validate all verifier revisions
across the complete conjunction. Pure security qualification precedes native I/O;
existing final qualification after permitted closed setup retains its cleanup
contract. No new ADR is needed: this implements the existing composition and
independent-authority obligations without introducing a new runtime owner.
Selected conclusions are migrated into the TD/Core/Planning/Binding domain owners,
the HTTP capability contract and [the scoped evidence](../docs/evidence/security-composition.md).
Initial singleton-only refusal assumptions are superseded; prior finite authority,
data validation, original-owner cleanup and PR #271 portability obligations remain.
