# TD-driven Thing-root WriteAllProperties

Status: MIGRATED

The selected capability follows the merged all/selected-read and Property Write
slices. TD 1.1 requires every writable (non-readOnly) Property in one root write.
Reuse the existing finite owners and role-neutral member descriptor; derive the
writable input independently from readable output. Complete input validation and
group authorization precede application effects. A distinct root behavior and one
actual native PUT preserve group intent. Implicit per-member WriteProperty calls
would change semantics and are rejected. A universal collection runtime adds no
needed lifecycle and is rejected. Future selected writes can separately retain
a caller-selected subset of the writable eligibility without weakening all-write.

One request does not prove transactional application. A root handler's empty
success means reported completion of the entire group, not acceptance or partial
success. Add Completed evidence for this group to the existing conservative effect
carriers, while preserving independent local/native outcomes and delivery.
Failures after entry remain possibly partially applied; lost/ambiguous native
completion remains remote-uncertain. No automatic replay, rollback or per-member
receipt can be inferred. Reusing Unknown alone plus a local result would obscure
confirmed native completion after local revocation of an unread result.

HTTP owns PUT/204 and the existing strict Write framing/EOF completion boundary;
no HTTP fields enter Engine semantics. The captured map validator, work and schema
cleanup use existing funding. The real integration fixture deliberately applies
only one member then fails, so absence of atomicity is executable evidence.

Affected authority: Core collection/data/outcome, TD scoped loans, Planning,
Binding SPI, runtime safety, API ownership, WP-200/300/400/600 and the HTTP mapping.
Their projections own the migrated details. Tests and the implementation PR own
exact runnable evidence; this investigation grants no independent acceptance.
