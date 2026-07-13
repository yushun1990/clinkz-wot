# clinkz-wot Current Documentation Plan

The active repository-wide design revision is:

- `docs/design.md` (v4.6 design-closeout revision)

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

The executable performance contract checker and deterministic fixture
generator are in `tools/performance-harness`.

The v4.6 revision is not frozen for the coordinated implementation refactor
until every row in `docs/refactor-gates.csv` is closed. Design-only work,
verification tooling, fixture generation, and work-package preparation may
proceed while a gate is open; dependent runtime API migration may not.

The Directory performance artifact covers only the engine-side Directory client
contract. Directory service topology, storage backends, server-side query
execution, and production service SLOs are deferred to a later design.
Non-normative inputs retained for that future design are in
`docs/future/directory-service.md`; they are not active engine requirements.

Implementation refactoring starts from requirement-scoped work packages under
`IMPL-CONFORM-001` after their design dependencies are closed. Existing code is
not a competing design source, and partial implementation compatibility must
not weaken the target design. Cross-crate API, state, ownership, resource, and
performance changes are coordinated before a conforming release is declared.

Historical baselines, implementation plans, target notes, audit follow-ups, and
the previous root `PLAN.md` are archived under:

- `docs/deprecated/`

For new task sessions, read `docs/design.md` first, followed by the active
artifact relevant to the task. Open deprecated documents only when historical
rationale or migration context is explicitly needed.
