# WP-000 — Foundation — v6.0

## Target and dependency

Keep time/generation, linear work and capacity primitives; implement owner-scoped policies without universal per-family schemas. Numeric V1 profiles remain restricted adapters.

Exact package dependencies belong to [index.toml](index.toml). Runtime and optional
preparation are distinct: dynamic preparation integration needs WP-200, while
execution-only composition does not. Package completion is not a blanket start
barrier; exact available predecessors and affected authority must support the
chosen slice under ADR-0013. No new runtime tranche is admitted by this migration.

## Authority and completion

Applicable requirements: RES-OWNER-001, RES-HANDOFF-001, RES-WORK-001, RES-SCHEMA-001, RES-ENV-001, TIME-001, API-SOURCE-TIME-001.

Required evidence follows actual claims: Boundary exhaustion, original-account retention, reservation rollback, clock reset/wrap/exhaustion and bounded release. Actual layouts/allocator behavior are profile-specific.

Retained Property Read code/tests and historical tranches remain useful within
their recorded contracts. Their old status does not close this package under
v6. Exact successor source scope/API/state/resource changes, removals, checks and
independent review are determined from the current repository for each material
slice, not from a saved next-action list. New interaction families and Bindings
are outside this architecture migration.
