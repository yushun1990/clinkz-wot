# Project State

Last updated: 2026-07-29

## Repository Basis

The active design revision is v5.0 bounded-core authority.

- ADR-0018 decision checkpoint:
  `eb145c5e86ec9e9db0a09194bd4e2868784a927f`.
- Exact non-implementation candidate:
  `b1916250a28ee133e8d0b12225c5b6311c975247`, the single child of the
  decision checkpoint and the tip of `candidate/v5-authority-reset`.
- Independent root-session attestation:
  `6d483a598e654f5c7043efb887074aba3a605f7a`.
- Exact activation merge:
  `30b845a4b17dd3eb56670da48c939b72daea7d59`, whose first parent is the
  attestation checkpoint and whose second parent is the reviewed candidate.
- Activation rollback point:
  `6d483a598e654f5c7043efb887074aba3a605f7a`.
- D9 bounded-conversion governance checkpoint:
  `a952e2b034b8939c0abdaf1662707eaef1d2fdc8`.
- Latest completed Property Read source slice:
  `830f47ebe044b953a3c0c3214345968f0fb5e571`.
- Exact WP-200 Property Read semantic candidate:
  `4a01b5010729cb42d6e8d51125103c8b5cda8707`, the single child of
  `525bb31b42efe299ed36d46acea1a1c4286e8bde`.
- Independent WP-200 v1 semantic-candidate attestation:
  `8a7aa198f5c983be8fbf5ef1a9750c90b5837703`.
- First WP-200 admission-evidence correction candidate:
  `f453f165c2ea775e5f0d10c36f1e419fcc1d79f3`, the exact seven-path
  single child of the v1 attestation. Independent review rejected its
  pre-source state projection.
- Second WP-200 admission-evidence correction base:
  `f453f165c2ea775e5f0d10c36f1e419fcc1d79f3`; the exact six-path
  single-child candidate is resolved by its future v2 attestation.

The activation candidate changed exactly 27 documentation/checker paths and no
Rust source, Cargo manifest, public API, or runtime behavior. Its independent
review passed the exact candidate checker, aggregate design/evidence suite,
default workspace tests, diff hygiene, and the 21-cell valid feature matrix.

## Current Objective

Obtain independent v2 review for the exact six-path second
`WP-200-PROPERTY-READ-PLAN-SLICE` admission-evidence correction. Do not change
product source before that review and the corrected five-file pre-source
checkpoint both pass.

D8 is migrated. Its one conversion packet contains:

1. one associated-type portable Core compiler/artifact contract;
2. an application-closed, typed static compiler/cursor/artifact enum;
3. Core-owned safe host erasure that returns original owned values on mismatch;
4. sole WP-200 implementation ownership, with WP-300 consuming the component
   only inside a complete installable registration;
5. paired external host and constrained third-party authoring fixtures;
6. an owned Property Read plan output with no TD lifetime; and
7. one exact Category C tranche candidate with nine implementation paths,
   seven prechecks, audit, exclusions, and completion key.

Candidate `4a01b5010729cb42d6e8d51125103c8b5cda8707` is the exact 25-path
single child of `525bb31b42efe299ed36d46acea1a1c4286e8bde`. A later independent
root session inspected the registered contract and both external authoring
forms, reran every precheck and aggregate baseline, mutation-tested all six
required rejection boundaries, and found no intersecting blocker. The review
attestation records that immutable semantic candidate ref.

The original four-file admission-ready simulation failed at the mandatory v5
authority-reset activation check because the changed Property Read gate no
longer matched its carried SHA-256. The first correction added the exact
carry-forward digest to a five-file checkpoint. Independent mutation review
then exposed a second intersecting evidence-truth defect: the design checker
required all implementation paths in the approved `in-progress` pre-source
state even though the same checker required the implementation commit to be
the next child of that checkpoint.

The second correction keeps all reviewed API, fixture, implementation,
exclusion, precheck, and five-file checkpoint semantics unchanged while:

1. preserving and validating the exact failed first-correction predecessor;
2. requiring implementation-path presence only in `complete`, while the
   existing topology checker owns admitted `in-progress` pre-source and
   implementation work;
3. making the new candidate the exact six-path single child of the failed
   first correction; and
4. retaining a separate v2 attestation before source admission.

No registered Core/Planning implementation path or cross-package Property Read
architecture fixture root is admitted.

## Active Milestones

- M0 Execution Baseline and Collaboration Reset — CLOSED.
- M1 v5.0 Authority Reset and Architecture Closure — IN_PROGRESS.
- M2 Foundation and Core Contract Stabilization — IN_PROGRESS.
- M3 Planning and Compilation Pipeline — OPEN; exact second
  admission-evidence correction candidate review pending.

The v5 authority switch is complete, but M1 remains open because GATE-1,
GATE-2, GATE-4, GATE-5, and GATE-6 still require their registered closure
evidence. GATE-3 remains closed.

## Accepted Technical Model

Active v5 authority contains 62 requirements:

- 41 indispensable architecture/safety requirements; and
- 21 requirements protecting the first Property Read vertical slice.

The other 59 inherited v4.9 identities have checked inactive dispositions:

- 34 are mandatory domain-entry review input;
- 15 are historical design input;
- four premature or superseded identities are retired; and
- six redundant identities defer to stronger owners.

The package order remains:

`WP-000 -> WP-100 -> WP-200 -> WP-300 -> {WP-400, WP-500, WP-600} -> WP-700`.

ADR-0013 permits a dependency-complete, independently reviewed tranche to
proceed while disjoint global gates remain open. Package status alone never
admits source work.

D9 adds these execution rules:

- the active critical path names one executable objective, finite blockers,
  an observable closure event, and the next source event;
- decision, authoritative migration, authoring fixtures, and admission share
  one conversion packet when contract, rollback, and validation truth match;
- post-closure refinement may block only on an explicit intersecting semantic,
  ownership, lifecycle, resource, dependency, or evidence-truth finding;
- continuity, registry, audit, and checker changes travel with the checkpoint
  whose truth they record; and
- authority closure, package-local completion, and executable vertical
  integration are reported separately.

D8 selects this exact representation:

- `BindingCompilerExtension` owns associated `Cursor` and `Artifact` types;
- `step` consumes the cursor and returns it on pending or failure;
- constrained applications compose heterogeneous third-party compilers with
  one closed application enum and matching cursor/artifact enums;
- `HostBindingCompilerRegistration` is a Core-erased `std` component with safe
  borrowed and consuming payload access;
- `StaticBindingCompilerRegistration<C>` is typed in every feature cell;
- neither compiler component is independently installable;
- `BindingArtifactEnvelope<A>` checks full generation/configuration/
  compatibility/role identity and measured footprint; and
- `PlanBuildOutput<A>` owns logical plans, envelopes, and compact references
  without retaining the borrowed TD or registration snapshot.

## Implementation Truth

Completed and independently evidenced WP-100 work includes:

- Foundation refresh;
- handler value primitives;
- extended logical time;
- Deadline and cleanup timing;
- borrowed `HandlerContext`; and
- synchronous static `ReadPropertyHandler`.

The planned WP-200 architecture is not implemented:

- no `clinkz-wot-planning` crate exists;
- `LogicalInteractionPlan`, `BindingArtifact*`, `BindingCompiler*`,
  `PlanBuildInput`, `PlanCompiler`, `HostBindingRegistration`, and
  `StaticBindingRegistration` do not exist in product Rust;
- current form selection remains in `protocol-bindings/core`;
- Servient still stores `Arc<dyn ClientBinding>` and
  `Arc<dyn ServerBinding>` directly; and
- existing protocol binding paths still reflect the legacy direct execution
  boundary rather than the planned compiler-artifact/Servient orchestration
  split.

Those facts are implementation evidence for D8, not authority to preserve the
legacy boundary.

## Open Decisions and Blockers

### D8 / workspace issue 0014

Status: MIGRATED. No technical representation or package-ownership decision
remains open.

Required authoritative consumers:

- `docs/spec/planning.md`;
- `docs/spec/binding-spi.md`;
- the relevant architecture flow/module projections;
- `docs/api-ownership.csv`;
- `docs/work-packages/index.toml`;
- `docs/work-packages/WP-200-planning.md`;
- `docs/work-packages/WP-300-protocol-binding-spi.md`;
- `docs/work-packages/property-read-architecture-gate.toml`;
- paired public compile-contract fixtures;
- the tranche audit/checker; and
- PLAN, workspace lifecycle, artifact registry, and this state checkpoint.

The semantic candidate passed independent v1 review. Admission remains blocked
only by independent v2 review of the exact second evidence-boundary correction
and the corrected five-file pre-source checkpoint. The representation and
package ownership decisions are not reopened.

### WP-200 admission evidence boundary

Status: SECOND CORRECTION CANDIDATE REVIEW PENDING.

The attempted command
`tools/check-wp200-property-read-plan-slice-entry.sh --admission-ready`
rejected the original four-file checkpoint with:

`v5 authority reset activation check: carried artifact
'docs/work-packages/property-read-architecture-gate.toml' changed without
disposition update`

`docs/spec/v5-artifact-carry-forward.toml` owns the exact digest of that gate.
Therefore any honest gate-status transition must update the gate and its
carried digest atomically. Exact first correction
`f453f165c2ea775e5f0d10c36f1e419fcc1d79f3` did so, but independent review
found that its five-file simulation then failed with:

`design structure check: WP-200-PROPERTY-READ-PLAN-SLICE in-progress state
lacks implementation path "core/src/binding_compiler.rs"`

The failure contradicts the registered topology in which the approved
pre-source checkpoint precedes the exact nine-path implementation commit. The
second correction changes only:

- `PLAN.md`;
- `PROJECT_STATE.md`;
- `docs/audits/WP-200-property-read-plan-slice-entry.md`;
- `docs/spec/v5-artifact-carry-forward.toml`;
- `docs/work-packages/property-read-architecture-gate.toml`;
- `tools/design-check/src/main.rs`.

A later independent root must review that single child of
`f453f165c2ea775e5f0d10c36f1e419fcc1d79f3` and, if it passes, record
`docs/audits/WP-200-property-read-plan-slice-review-v2.toml`.

### Disjoint downstream blockers

- Broad WP-100 handler entry still lacks its remaining request/target
  migration, portable async/step admission, no-atomic public-boundary proof,
  and workload/resource evidence.
- Broad WP-300 remains blocked by exact registration/execution contracts and
  later binding/Servient integration evidence.
- WP-400, WP-500, and WP-600 depend on WP-300; WP-700 joins those branches.

These do not extend the D8 packet unless repository evidence shows a direct
contract, rollback, or validation intersection.

## Rejected or Superseded Approaches

- Lossless D3 domain-by-domain authority migration is superseded by ADR-0018.
- Foundation candidate
  `2494f33fdfe49ec3c7ae850d20990e446e628865` remains historical input and must
  not be activated.
- Partial v5 activation or piecemeal rollback is prohibited.
- A separate documentation/review cycle for each artifact in one semantic
  conversion packet is rejected by D9.
- Protocol Bindings selecting handlers, rescanning TDs at runtime, or owning
  Servient orchestration remains outside the frozen direction.
- A representation that works only through in-repository private types, or
  only for `std` trait objects, cannot close issue 0014.
- Binding-authored unsafe erasure, a heap-erased representation in every
  feature cell, separate host/static public compiler traits, independently
  installable compiler halves, and a WP-300 duplicate implementation owner are
  rejected by D8.
- Proceeding with the original four-file pre-source checkpoint is rejected
  because it makes the authoritative carry-forward digest stale.
- Dropping or weakening `v5-authority-reset-candidate-check` is rejected; the
  exact carried digest is active v5 evidence, not optional bookkeeping.
- Entering product source on the v1 semantic attestation alone is rejected
  because the discovered evidence-truth defect intersects admission.
- Attesting first correction
  `f453f165c2ea775e5f0d10c36f1e419fcc1d79f3` is rejected because its exact
  five-file pre-source simulation requires implementation source before the
  registered implementation commit.

## Verification Baseline

Independent review of candidate
`b1916250a28ee133e8d0b12225c5b6311c975247` on 2026-07-28 passed:

- `tools/check-v5-authority-reset-candidate.sh`;
- `tools/check-design-artifacts.sh`;
- `cargo test --workspace --locked`;
- `sh scripts/check-feature-matrix.sh` — 21 passed, 0 failed; and
- `git diff --check
  eb145c5e86ec9e9db0a09194bd4e2868784a927f..b1916250a28ee133e8d0b12225c5b6311c975247`.

The activation merge was additionally checked to have exact parents and zero
content difference from the candidate across all 27 candidate paths.
Post-activation status reconciliation passed:

- `tools/check-v5-authority-reset-candidate.sh`;
- `tools/check-design-artifacts.sh`;
- `cargo test --workspace --locked`;
- `sh scripts/check-feature-matrix.sh` — 21 passed, 0 failed; and
- `git diff --check`.

These checks preserve the active-owner, carry-forward, completed-evidence,
workspace-test, and feature-matrix baselines.

Author-side D8 schema validation on 2026-07-28 passed:

- the 19 design-check unit tests;
- four existing handler API schema tests; and
- four new WP-200 compiler/artifact schema tests covering closed static
  dispatch, zero-budget cursor preservation, safe host mismatch recovery,
  artifact footprint rejection, and TD-lifetime-free owned output.

Author-side exact candidate validation also passed:

- `tools/check-wp200-property-read-plan-slice-entry.sh --candidate`, including
  the exact base/parent/path gate and the expected absent-implementation-source
  boundary;
- `tools/check-design-artifacts.sh`;
- `cargo test --workspace --locked`;
- the supported feature matrix, 21 passed and 0 failed; and
- staged/committed diff hygiene and the exact 25-path candidate comparison.

Independent root-session review of exact candidate
`4a01b5010729cb42d6e8d51125103c8b5cda8707` on 2026-07-29 passed:

- `tools/check-wp200-property-read-plan-slice-entry.sh --candidate`;
- inspection of the full 25-path diff, executable schema, and paired external
  static/host authoring fixtures;
- all seven registered prechecks and the expected absent-source completion
  boundary;
- `tools/check-design-artifacts.sh`;
- `cargo test --workspace --locked`;
- `sh scripts/check-feature-matrix.sh` — 21 passed, 0 failed; and
- `git diff --check
  525bb31b42efe299ed36d46acea1a1c4286e8bde..4a01b5010729cb42d6e8d51125103c8b5cda8707`.

The review additionally mutation-tested and rejected host payload
compatibility mismatch, host payload concrete-type mismatch, host cursor
concrete-type mismatch, artifact footprint overflow, static enum variant
mismatch, premature product-source creation, and premature cross-package
architecture-fixture creation. No finding intersects the tranche.

Review attestation
`8a7aa198f5c983be8fbf5ef1a9750c90b5837703` is the single child of the
candidate, changes exactly the registered three review paths, and passes the
aggregate design and default workspace baselines.

The subsequent original four-file admission-ready simulation intentionally
failed at `v5-authority-reset-candidate-check`: the gate changed while its
carried digest/disposition did not. That failure is the evidence for the
first seven-file correction candidate and corrected five-file pre-source
boundary.

Independent review of exact first correction
`f453f165c2ea775e5f0d10c36f1e419fcc1d79f3` on 2026-07-29 passed its full
seven-path inspection, `--candidate`, aggregate design checks, default
workspace tests, the 21-cell feature matrix, and diff hygiene. Its required
pre-source mutations then proved:

- omitting `docs/spec/v5-artifact-carry-forward.toml` is rejected as expected;
- the exact five-file checkpoint passes the carried-digest check; but
- that checkpoint is then incorrectly rejected because the `in-progress`
  status requires absent `core/src/binding_compiler.rs`.

No v2 attestation was created. The second correction narrows implementation
presence to `complete` while retaining the existing exact pre-source and
implementation topology checks for `in-progress`.

The intentionally invalid all-features combination enables mutually exclusive
Zenoh backends. Use `scripts/check-feature-matrix.sh`, not
`cargo test --all-features`, as the supported feature baseline.

## Next Safe Actions

1. In a later independent root session, review the exact six-path second
   correction candidate, rerun `--candidate` and aggregate baselines,
   mutation-test
   omission of the carry-forward manifest versus the exact five-file boundary,
   and create only the v2 attestation/registry/state checkpoint if it passes.
2. From the v2 attestation, record one exact five-file
   `pending`/`review-pending` to `in-progress`/`approved` checkpoint with the
   gate's matching carried digest, then run `--admission-ready`.
3. Only then implement exactly the nine registered WP-200 paths, run the
   completion checker and supported feature cells, and register completion
   evidence without advancing WP-300, WP-400, or the aggregate gate.

Ask the Project Owner only if the investigation reaches a product-goal,
real-world constraint, unacceptable direction, or irreversible external
commitment that repository evidence cannot resolve.

## Primary Continuation References

- `AGENTS.md`
- `PROJECT_GOVERNANCE.md`
- `ARCHITECTURE_GOVERNANCE.md`
- `PLAN.md`
- `docs/design.md`
- `docs/ADRs/0013-work-package-scoped-implementation-admission.org`
- `docs/ADRs/0018-bounded-v5-normative-authority-reset.org`
- `docs/spec/v5-authority-reset.toml`
- `docs/audits/D7-v5-authority-reset-candidate.toml`
- `docs/audits/D7-v5-authority-reset-review.toml`
- `workspace/0014-property-read-plan-artifact-boundary.md`
- `workspace/0016-post-reset-implementation-throughput.md`
- `docs/spec/planning.md`
- `docs/spec/binding-spi.md`
- `docs/work-packages/property-read-architecture-gate.toml`
- `docs/work-packages/WP-200-planning.md`
- `docs/work-packages/WP-300-bindings.md`
- `docs/audits/WP-200-property-read-plan-slice-entry.md`
- `docs/audits/WP-200-property-read-plan-slice-review.toml`
- `docs/audits/WP-200-property-read-plan-slice-review-v2.toml` (future)
- `tools/design-check/tests/wp200_binding_artifact_schema.rs`
