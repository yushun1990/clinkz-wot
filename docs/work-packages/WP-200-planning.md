# WP-200 — Optional preparation — v6.0

## Target and dependency

Build deterministic complete selected support and pure source-independent construction. Permit owned incremental native output, Consumer first use, optional checked reuse and fully qualified Producer deferred construction. Relocate physical output/reclaim owners before execution-only Servient claims.

Exact package dependencies belong to [index.toml](index.toml). Runtime and optional
preparation are distinct: dynamic preparation integration needs WP-200, while
execution-only composition does not. Package completion is not a blanket start
barrier; exact available predecessors and affected authority must support the
chosen slice under ADR-0013. No new runtime tranche is admitted by this migration.

## Authority and completion

Applicable requirements: PREP-ISOLATION-001, PREP-SELECT-001, PREP-DEFER-001, PREP-REUSE-001, PREP-COVER-001.

Required evidence follows actual claims: Downstream compiler/executor pairing without whitelists; source destruction before execution; pure construction capacity/cancellation/rollback; selected support and qualification/materialization consistency. Existing Copy/eager tests remain adapter regressions.

Retained Property Read code/tests and historical tranches remain useful within
their recorded contracts. Their old status does not close this package under
v6. Exact successor source scope/API/state/resource changes, removals, checks and
independent review are determined from the current repository for each material
slice, not from a saved next-action list. New interaction families and Bindings
are outside this architecture migration.

## Type-only schema prerequisite

The Host-only `type_schema::prepare_type_schema` prepares the optional type
assertion from TD-resolved `SchemaView` facts into Core-owned fixed-size
`TypeSchema`. It refuses
const/enum/composition, nested properties/items, required/range/multiple/length,
pattern/format/content and unknown extension assertions; this is a supported
subset rather than a promise to validate arbitrary DataSchema. Metadata, default,
unit and read/write annotations impose no value assertion. Missing type is
unconstrained only when no unsupported assertions remain.

The HTTP Producer captures this result with native operands. The actual executor
performs the corresponding codec/validation check before success sealing. The
preparation pair requires an actual validation-service facet for a serving
provider; a numeric response policy is insufficient. The
extension-only `supported_schema` predicate is removed. Complete final-TD
participation/behavior/readiness qualification remains distinct from the existing
single-Property `Covered` shape marker and `PreparedProducer` adapter.
