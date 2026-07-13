# clinkz-wot Current Documentation Plan

The active repository-wide design revision is:

- `docs/design.md` (v4.6 frozen implementation-ready revision)

Normative implementation-support artifacts referenced by that design are:

- `docs/artifacts.csv`
- `docs/api-ownership.csv`
- `docs/refactor-gates.csv`
- `docs/resource-limits.csv`
- `docs/state-machines.toml`
- `docs/performance/manifest.schema.json`
- `docs/performance/result.schema.json`
- `docs/performance/fixtures.lock.toml`
- `docs/performance/fixture-generator.md`
- `docs/performance/constrained.toml`
- `docs/performance/gateway.toml`
- `docs/performance/directory.toml`
- `docs/requirements.csv`
- `docs/work-packages/index.toml`
- `docs/work-packages/*.md`
- `docs/amendments/WP-100-error-cleanup-v1.md`
- `docs/amendments/WP-100-error-disposition-v1.md`

The executable performance contract checker and deterministic fixture
generator are in `tools/performance-harness`.

The v4.6 revision is frozen for coordinated implementation: every row in
`docs/refactor-gates.csv` is closed. Runtime API migration proceeds only through
the dependency order in `docs/work-packages/index.toml`. Reopening a gate blocks
affected packages as specified by `REFACTOR-GATE-001`.

The frozen WP-100 error, retry, correlation, and cleanup Rust schemas are closed
by normative amendment `WP-100-ERR-CLEANUP-001`. The amendment resolves schema
details left open by the base prose without changing the implementation DAG.
The success/error boundary, shared wire disposition, handler-absence mapping,
and legacy Servient predicate removal are closed by normative amendment
`WP-100-ERR-DISPOSITION-001`.

That amendment adds `additional_responses_per_form_max`. The resulting `WP-000`
refresh now covers all 118 resource fields in the generated foundation schema,
profile snapshots, boundary tests, and evidence. `WP-000` is complete and
`WP-100` has resumed.

The Directory performance artifact covers only the engine-side Directory client
contract. Directory service topology, storage backends, server-side query
execution, and production service SLOs are deferred to a later design.
Non-normative inputs retained for that future design are in
`docs/future/directory-service.md`; they are not active engine requirements.

Implementation refactoring starts from the requirement-scoped `WP-000`
foundation package and follows the `IMPL-CONFORM-001` DAG. Existing code is not
a competing design source, and partial implementation compatibility must not
weaken the target design. Cross-crate API, state, ownership, resource, and
performance changes are coordinated before a conforming release is declared.

Historical baselines, implementation plans, target notes, audit follow-ups, and
the previous root `PLAN.md` are archived under:

- `docs/deprecated/`

For new task sessions, read `docs/design.md` first, followed by the active
artifact relevant to the task. Open deprecated documents only when historical
rationale or migration context is explicitly needed.
