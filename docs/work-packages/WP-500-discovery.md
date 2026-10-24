# WP-500 — Discovery clients — v6.0

## Target and dependency

Compose introduction/exploration/bounded retrieval and TD admission explicitly. Protocol adapters own native discovery; Directory interactions may reuse Consumer facets without requiring all discovery to do so. No implicit Directory service.

Exact package dependencies belong to [index.toml](index.toml). Runtime and optional
preparation are distinct: dynamic preparation integration needs WP-200, while
execution-only composition does not. Package completion is not a blanket start
barrier; exact available predecessors and affected authority must support the
chosen slice under ADR-0013. No new runtime tranche is admitted by this migration.

## Authority and completion

Applicable requirements: DISC-CLIENT-001, WOT-SEM-001, WOT-EXT-001, EXEC-SECURITY-001, SAFE-CANCEL-001, SAFE-CLEANUP-001.

Required evidence follows actual claims: Untrusted link/destination policy, bounded pages/results/continuations, cancellation/failure/cleanup and trust. In-memory backend functionality is not evidence for the complete target workflow.

Retained Property Read code/tests and historical tranches remain useful within
their recorded contracts. Their old status does not close this package under
v6. Exact successor source scope/API/state/resource changes, removals, checks and
independent review are determined from the current repository for each material
slice, not from a saved next-action list. New interaction families and Bindings
are outside this architecture migration.
