# Executable Thing-root ReadAllProperties

Status: MIGRATED

## Finding and alternatives

The accepted EXEC-COLLECTION-001 contract already requires native collection
semantics. Implementation has actual root Form loans and complete Producer Form
walking, but no root schema closure, operation-scoped Consumer definition or
registered finite root behavior/native mapping. Individual Property eligibility
cannot establish a collection capability. HTTP already provides one native
GET-to-object mapping, matching TD 1.1 §6.3.9.6 and §8.3.1.

An implicit loop over ReadProperty is rejected: it changes the native interaction,
failure attribution and consistency. A universal collection/session framework is
also rejected: selected-member writes and long-lived groups have materially
different data roles and owners, and there is no executable need for their APIs
here. A new finite collection runtime would duplicate existing authority,
settlement and resource machinery without a different lifecycle. Reusing an
unconstrained object assertion would discard individual schemas and readability.

## Selected direction

Reuse the existing finite owners for this one finite operation. Carry Thing scope
explicitly through Consumer selection and HTTP native mappings. Root semantic
loans include Property schemas with their own contexts, without individual Forms.
Prepare owned readable-member names/type assertions, excluding writeOnly members;
require actual collection codec support and retain its services/configuration.
A separate root application behavior returns the combined result. HTTP owns the
single GET/200 JSON object mapping and native method/path collision checks.

The exact supported response requires every readable member once and validates
each original value. Missing, duplicate, unknown/writeOnly or invalid members
fail the entire interaction; application aggregation failure and unsupported
native partial statuses do not grant partial success. No atomic snapshot or
simultaneous sampling is inferred. Native aggregation, application consistency
and transport remain outside Engine orchestration.

Producer final qualification covers all root and individual Forms/operations,
original member meaning, actual behavior and native ownership after closed setup.
Finite cancellation/replacement/late-result/retirement rules apply without a new
state machine. Captured service traversal work includes the bounded member walk;
short credit retains the complete exchange before handler/codec entry. Schema
backing and bounded destruction retain original funding through final release.

All-versus-selected scope and read/write data roles remain distinct. A future
multiple read needs its chosen set; all writes concern all writable members;
partial write effects need explicit evidence. Long-lived collections must use the
appropriate continuing/notification owners and actual mapped controls. This
implementation creates none of those capabilities and requires no Binding to
support them.

## Affected authority and disposition

EXEC-COLLECTION-001/EXEC-DATA-001, PREP-SELECT-001/PREP-COVER-001, Binding SPI,
TD loans, runtime safety, API ownership and WP-200/300/400/600 now project this
bounded implementation. The HTTP README owns the concrete mapping/subset;
PLAN records only coarse capability coverage. No ADR reversal, gate, milestone,
work-package dependency or production qualification claim changes.

The real HTTP test target proves one Engine Consumer-to-Producer native request
for multiple Properties, without individual dispatch, and independent malformed
peer responses. Refusal, final qualification, captured service loss, unpaid work,
saturation, cancellation, replacement, retained late results and final release
are checked against the existing owners. Existing finite, Action and notification
regressions remain applicable. Independent acceptance reconstructs the exact diff
and executable evidence from repository authority; it is not granted by this
investigation's disposition.
