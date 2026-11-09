# TD-described finite response execution

Status: MIGRATED

Basis: master after #272 (`678395337e40bcd5559c6a9bb244854f1981b92a`),
ADR-0025, EXEC-DATA/OUTCOME-001, PREP-SELECT/COVER/REUSE-001,
SPI-NATIVE/DELIVERY-001 and SAFE-CANCEL/CLEANUP/PROFILE-001.

The [TD 1.1 Recommendation](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#response-related-terms-usage)
distinguishes primary response metadata from additional alternatives. Additional
success defaults to false; media defaults to the enclosing Form, independently
of primary response media. A named schema replaces the operation's ordinary
output schema. Each response needs a distinguishable native representation;
TD does not require HTTP status codes or a universal numeric discriminator.

Existing common preparation rejects different response media and additional
responses. The finite seal rejects every additional branch. Native status is
mandatory metadata despite being opaque to Core. Write success shape would
also reject an application-error payload. These are provisional implementation
boundaries, inconsistent with the intended common semantics.

Selected correction: common immutable alternatives retain exact order, media,
data role and success meaning. A present unconstrained value differs from no
data. Native mapping and discrimination stay with the concrete Binding. Capture
the finite admitted branch/shape expectation before native handoff; metadata
cannot add a branch. Validate each selected branch against its own retained
schema and actual service. A declared application failure is a validated output,
distinct from Core execution errors, and does not certify no remote effects.

Consumer selection must qualify every branch of the selected Form/operation;
Producer final qualification must cover every retained advertised alternative.
Reuse compares complete common response meaning. Existing Security composition,
authority, first cause, late results, native retirement and physical funding stay
with their original owners. Shared values and shape checks remain alloc/no_std
without pointer atomics; Host owners need not become the portable representation.

Alternatives rejected: accepting arbitrary indices at runtime; using native
status classes as WoT success; returning application errors as transport errors;
validating every result against the primary schema; deriving absence of effects
from a failure response; adding a response manager or generic codec hierarchy.
The concrete increment will use bounded JSON Property responses over actual
HTTP I/O in both roles. Other mappings must decline unsupported alternatives.

Affected owners: TD facts, Core data/outcome semantics, Planning selected/full
qualification, Binding native responsibility, runtime effect certainty, public
API projections and WP-100/200/300/400/600. Prior primary-only evidence remains
regression evidence; it cannot establish alternatives or broad conformance.

## Migration

The domain owners now define exact response, selection and outcome behavior in
[Core](../docs/spec/interaction-core.md#td-described-finite-response-alternatives),
[Planning](../docs/spec/planning.md), [TD](../docs/spec/td.md),
[Binding SPI](../docs/spec/binding-spi.md#td-described-finite-response-identification)
and [runtime safety](../docs/spec/runtime-safety.md#declared-finite-failures).
API/work-package projections and the concrete HTTP profile follow those owners.
The [execution evidence](../docs/evidence/td-responses.md) owns demonstrated scope
and remaining limitations. The correction concretizes ADR-0025's existing shared
semantic responsibility; it introduces no new universal runtime or architecture
activation. Prior primary-only restrictions are superseded only for the new
qualified finite Property facet. Historical primary, Security, collection and
portable Read evidence remains regression evidence within its original scope;
none becomes acceptance for additional responses by inheritance.

## Original response identity counterexample

Two native Forms for one Property/operation can declare different additional
schemas and branch order. The existing HTTP configuration validator selected
its first target/operation match; a payload valid only under the first Form
could pass Producer validation and be delivered under the second. A real-I/O
regression failed before correction. Binding-owned validation now retains the
exact original matched mapping with the exclusive exchange/reply. A pure
FiniteValidator loan supplies that exact common response set to HandlerContext;
one behavior chooses scoped branches without TD lookup. Normal, invalid and
sibling-reordered exchanges test both Consumer and Producer execution.

Reject a global Engine Form/mapping token or a restriction requiring all sibling
Forms to share response meaning. HTTP already owns native correlation and one
exclusive exchange; retaining that original attribution is its responsibility.
This does not prove every other protocol's concurrency/lifetime strategy.
