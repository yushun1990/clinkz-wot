# WP-000 — Foundation — v6.0

## Target and dependency

Maintain the time and generation primitives. Move concrete limits and any
necessary cumulative work counters to their enforcing owners. Migrate the
callers of the flat resource schema, generic receipts and work vector. [Resource contracts](../spec/foundation.md)

Exact package dependencies belong to [index.toml](index.toml). Runtime and optional
preparation are distinct: dynamic preparation integration needs WP-200, while
execution-only composition does not. Package completion is not a blanket start
barrier; exact available predecessors and affected authority must support the
chosen slice under ADR-0013. No new runtime tranche is admitted by this migration.

## Authority and completion

Applicable requirements: RES-OWNER-001, RES-HANDOFF-001, RES-WORK-001, RES-SCHEMA-001, RES-ENV-001, TIME-001, API-SOURCE-TIME-001.

A fixed slot supplies its own backing. Where a deployment retains an aggregate
ceiling, exercise its actual participants and final release. Fill the last unit
with an old result, take and hold it, and refuse a new caller with a free native
slot. Dispose the result and race two contenders. Exactly one takes the unit
when the policy promises a shared remainder. For an allowed partition check
return to the original share. A fenced cutover drains or transfers every counted
claim once. An explicitly retired ceiling needs no shadow counter. Preserve the
clock reset, wrap and exhaustion oracles.

Retire `ResourceKind`, `ResourceLimits`, `ResourceProfileId`, `StaticResourceProfile`,
the named profiles and `AdmissionLedger` with their last consumers. Replace
`ResourceAccount` and `ResourceReservation` handoffs with actual storage or the
specific deployment permit required by a retained promise. Remove `WorkBudget`,
`WorkClass` and `BudgetExceeded` after the public and internal drivers migrate.

Retained Property Read code/tests and historical tranches remain useful within
their recorded contracts. Their old status does not close this package under
v6. Exact successor source scope/API/state/resource changes, removals, checks and
independent review are determined from the current repository for each material
slice, not from a saved next-action list. New interaction families and Bindings
are outside this architecture migration.
