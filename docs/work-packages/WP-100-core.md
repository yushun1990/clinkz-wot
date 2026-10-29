# WP-100 — TD and semantic execution contracts — v6.0

## Target and dependency

Maintain one TD interpreter; complete media/schema/security contracts below preparation, local identity and capability-specific interaction outcomes. Preserve Action invocation and Event control/response meaning without implementing a universal request/session.

Exact package dependencies belong to [index.toml](index.toml). Runtime and optional
preparation are distinct: dynamic preparation integration needs WP-200, while
execution-only composition does not. Package completion is not a blanket start
barrier; exact available predecessors and affected authority must support the
chosen slice under ADR-0013. No new runtime tranche is admitted by this migration.

## Authority and completion

Applicable requirements: MOD-DEPS-001, WOT-SEM-001, WOT-EXT-001, EXEC-ID-001, EXEC-HANDLER-001, EXEC-DATA-001, EXEC-SECURITY-001, EXEC-ACTION-001, EXEC-NOTIFY-001, EXEC-COLLECTION-001, EXEC-OUTCOME-001.

Required evidence follows actual claims: Shared semantic defaults and extension cases, source-free validation, missing/unknown versus unsupported, first-cause/late result and supported profile checks. No claimed no-atomic runtime from vocabulary compilation.

Retained Property Read code/tests and historical tranches remain useful within
their recorded contracts. Their old status does not close this package under
v6. Exact successor source scope/API/state/resource changes, removals, checks and
independent review are determined from the current repository for each material
slice, not from a saved next-action list. New interaction families and Bindings
are outside this architecture migration.

## Consumer composition scope

Core now carries operation-scoped finite definitions, distinct input/output
payload schema roles, and a separate move-only Consumer authority/passive
reference. The implementation and independent Binding evidence are scoped in
WP-200/WP-400. The type-only schema and NoSec service envelope remain supported
subsets; general DataSchema constraints, security composition/credentials,
continuing Consumer Actions and TD-driven Event/observation preparation remain
open. Existing raw Event control/response/retirement evidence is retained.
