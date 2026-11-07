# Secure effectful Property Write

Status: MIGRATED

## Independent assessment

Basis: master e267ac6241c11692feef986e994e2f501425bd09, ADR-0025,
EXEC-SECURITY/OUTCOME/DATA-001, PREP-SELECT/COVER/REUSE-001,
SPI-INGRESS/DELIVERY-001 and SAFE-AUTH/CONCUR/CANCEL/CLEANUP-001.

The existing finite Write already validates structured input and qualifies the
whole Producer TD. Its HTTP PUT/204 mapping and retained native owners are
usable over TLS. The Basic compiler, access adapter and grant each impose a
Read-only restriction; that is an implementation limit, not Basic semantics.
Removing every operation restriction would incorrectly admit unproved families.
Select Property Read and Write explicitly; each issued grant must retain the
exact requested operation, instance, destination, registration and revision.

The Read security path verifies twice around authorization and rechecks the
original instance/publication/service before dispatch and successful send.
However, the final verifier can change application policy while returning the
same Principal. Repeating arbitrary callbacks forever cannot establish a stable
decision. Introduce passive invalidation revisions on the actual finite handler
and verifier, observed across each acceptance/delivery callback sequence. Mutable
implementations must advance revisions without reuse; immutable services use
zero. This is an explicit callback contract, not a new security manager. The
Host Producer dispatch unnecessarily borrowed the whole owner exclusively even
though lookup/acceptance only mutate the selected runtime. Use shared logical
owner access for finite dispatch, preserving exclusive runtime access. A one-way
ProducerRevocation handle allows callback-time instance revocation without
registry borrowing, retains original weak-cell funding, and cannot address a
successor. This is narrower than a checked-out Producer registry or universal
reentrant session owner. Publication revocation is already passive.
Finite Consumer callbacks already run outside the call registry; exercise that
boundary with effectful HTTPS calls and callback-time cancellation.

Property Write currently remains Unknown even after validated completion, while
root writes distinguish Completed. Extend the existing completion evidence to
Property Write. Preserve Unknown after behavior/native entry without validated
completion, NotInvoked before Producer behavior, and NotAttempted before Consumer
native progress. An HTTP refusal cannot prove remote nonexecution. Local policy
loss, cancellation or lost delivery cannot erase known completion or establish
rollback, atomicity, remote cancellation, physical-state confirmation or retry.

## Alternatives and falsification

Reject duplicating the Read or Write runtime, putting credentials in definitions,
putting HTTP statuses/TLS in Core, adding transactional receipts, and weakening
complete qualification. Retain the strict bounded empty-204/closing-connection
mapping, independently selected services, source-independent schemas, original
funding and finite native cleanup. Real Engine Consumer/Producer TLS writes and
independent TLS peers must expose validation/authentication/authorization refusal,
callback invalidation, lost responses after effects, cancellation, retained
completion, unsupported operations and exact final release.

Affected owners are Core, Binding SPI, runtime safety, HTTP capability and the
existing work packages. No gate or production/security milestone closes.

The selected boundary is migrated to the existing domain specifications, API
ownership projection, work packages and [scoped security evidence](../docs/evidence/td-security-https.md).
Prior Read and finite Write evidence is reaffirmed except the deliberately
superseded Unknown classification of a validated Property Write completion.
No new ADR is needed: this extends the existing finite authority and effect
contracts without adding a cross-domain ownership model.
