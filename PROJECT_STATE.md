# Project State

Last updated: 2026-07-30

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
  `b889ae1dafa65ee66bfb331bebf9d537e1c29eee`, the exact nine-path WP-200
  Core/Planning implementation.
- Exact WP-200 Property Read semantic candidate:
  `4a01b5010729cb42d6e8d51125103c8b5cda8707`, the single child of
  `525bb31b42efe299ed36d46acea1a1c4286e8bde`.
- Independent WP-200 v1 semantic-candidate attestation:
  `8a7aa198f5c983be8fbf5ef1a9750c90b5837703`.
- First WP-200 admission-evidence correction candidate:
  `f453f165c2ea775e5f0d10c36f1e419fcc1d79f3`, the exact seven-path
  single child of the v1 attestation. Independent review rejected its
  pre-source state projection.
- Exact second WP-200 admission-evidence correction candidate:
  `d2dcf2e9d2e19c7c2dfa234f96c5303cc3aee24a`, the exact six-path
  single child of `f453f165c2ea775e5f0d10c36f1e419fcc1d79f3`.
- Independent WP-200 v2 attestation:
  `4f3bdeff604e30eecfbba9c8c12e6dd0b23cc87f`, the exact three-path
  review/registry/continuation checkpoint for that immutable candidate.
- Exact WP-200 pre-source checkpoint:
  `ce1e4ae448617458251f4f437e66e77fb652e86b`, the five-file child of the v2
  attestation that changes the tranche to `in-progress`/`approved`.
- Exact WP-200 implementation:
  `b889ae1dafa65ee66bfb331bebf9d537e1c29eee`, the nine-file child of that
  pre-source checkpoint.
- Design-check runtime-root correction:
  `eacaaf1242a41861758ebc78a40ada2d88d15bba`.

The activation candidate changed exactly 27 documentation/checker paths and no
Rust source, Cargo manifest, public API, or runtime behavior. Its independent
review passed the exact candidate checker, aggregate design/evidence suite,
default workspace tests, diff hygiene, and the 21-cell valid feature matrix.

## Current Objective

Prepare the exact `WP-300-PROPERTY-READ-BINDING-SLICE` admission candidate,
public host/constrained authoring fixtures, executable lifecycle schema, entry
check, and completion boundary without editing WP-300 product source.

The narrow WP-200 plan slice is complete. Its handoff to WP-300 contains:

1. one associated-type portable Core compiler/artifact contract;
2. an application-closed typed static compiler/cursor/artifact composition;
3. Core-owned safe host erasure that returns original owned values on mismatch;
4. an immutable owned Property Read plan and artifact set with no TD lifetime;
5. bounded, deterministic planning progress in all three feature cells; and
6. sole WP-200 implementation ownership, which WP-300 may consume only inside
   a complete installable registration.

No WP-300 implementation path or cross-package Property Read architecture
fixture root is admitted. The validation prerequisite is resolved:
`tools/design-check` now selects an explicit runtime worktree root from its
callers, falls back only to runtime ancestor discovery, and rejects an invalid
explicit root instead of silently checking another worktree.

Workspace issues 0025-0029 are decided and migrated. They establish a finite
Producer Property Read slice, two distinct downstream release events, strict
no-backflow into legacy selection, explicit global-gate impact mapping, and
reuse of the existing immutable candidate/transition machinery. Candidate
preparation is now the next critical-path action.

## Active Milestones

- M0 Execution Baseline and Collaboration Reset — CLOSED.
- M1 v5.0 Authority Reset and Architecture Closure — IN_PROGRESS.
- M2 Foundation and Core Contract Stabilization — IN_PROGRESS.
- M3 Planning and Compilation Pipeline — IN_PROGRESS; the exact WP-200
  Property Read plan slice is complete, while broad WP-200 exits remain open.
- M4 Protocol Binding SPI and Lifecycle — OPEN; the exact WP-300 Property Read
  slice dependency is satisfied but no candidate is admitted.

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

D18-D22 add the WP-300 conversion boundary:

- the narrow tranche installs one complete bundle advertising only Producer
  Property Read and covers immediate/external readiness, committed-closed
  routes, permit-gated acceptance, one response, cleanup, and host/static
  authoring;
- optional interface methods may reject unadvertised roles before state or
  side effects, but do not complete client, subscription, emission,
  contribution, broad cancellation, workload, Servient, or concrete-protocol
  behavior;
- narrow WP-300 completion releases only the narrow WP-400 Property Read
  tranche, while broad WP-300 completion releases broad WP-400, WP-500, and
  WP-600;
- target Planning/Core/WP-300 code cannot call a legacy selector, rescan a TD,
  or send a target request into legacy dispatch; WP-600 removes concrete call
  edges and WP-700 proves final absence;
- global findings affect scoped evidence only through an explicit mapped
  intersection or named revalidation; and
- WP-300 reuses immutable candidate, independent scoped review, exact
  next-state simulation, combined pre-source admission, and completion
  evidence rules.

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

Completed WP-200 narrow-slice work includes:

- the `clinkz-wot-planning` crate in the root workspace;
- Core logical-plan, generation, compiler, typed static registration, safe
  host-erasure, artifact identity/envelope, compatibility, role, footprint,
  and rejection values;
- a bounded Property Read `PlanCompiler` that resolves the selected form and
  target only during planning;
- owned plan output that outlives its borrowed TD and registration inputs; and
- external constrained/static and host authoring fixtures with mismatch,
  footprint, zero-budget, and deterministic-step evidence.

The legacy form-selection implementation still exists in
`protocol-bindings/core`; Servient still stores `Arc<dyn ClientBinding>` and
`Arc<dyn ServerBinding>` directly; and existing concrete bindings still use
the legacy execution boundary. D12 assigns those one-way migrations to
WP-300, WP-400, WP-600, and final WP-700 removal evidence, so their presence
does not weaken the completed narrow WP-200 claim.

## Open Decisions and Blockers

### Focused execution-risk decisions / workspace issues 0017-0029

Status: twelve decisions migrated; one remote-enforcement action remains.

- 0017: the WP-200 admission path has a finite stopping condition. Independent
  review now includes the exact next-state simulation; the five-file
  pre-source transition passes, omission of its carry-forward update fails,
  out-of-scope implementation work fails, and the exact nine-path child now
  has completion evidence.
- 0018: the four Property Read source slices remain necessarily serial because
  each next slice consumes the preceding public ownership/lifecycle boundary.
  Later preparation is allowed but is neither admission nor vertical progress.
- 0019: the legacy migration map is owned by WP-200, WP-300, WP-400, WP-600,
  and WP-700. Coexistence is permitted only at their named one-way adapters;
  final absence is a WP-700 claim, not a narrow-gate claim.
- 0020: governance artifacts form a directed
  owner -> projection -> evidence -> validation graph. The two WP-200
  correction cycles were real transition-validation defects, not architecture
  changes; transition reviews must now execute their declared next state.
- 0021: D8 is reaffirmed for admission. The external host and
  `no_std + alloc` fixtures use public dependencies and preserve ownership;
  both compile at WP-200 completion, while the WP-600 production compilers own
  the remaining production-author usability evidence.
- 0022: independent review is scoped by defect class. Session separation alone
  does not close runtime, lifecycle, resource, workload, performance, or
  production-author claims.
- 0023: the local/remote integrity concern is confirmed. GitHub now reports
  `master` protected, and mainline workflow run `30503733056` passed for
  `9082ff4eb24d96572ae1124096185aa20abb3472`, but the branch API reports
  required-status-check enforcement `off` with no contexts/checks. Do not
  report mechanically protected validation until the remote rule requires
  `mainline / validation`.
- 0024: the full v1 target remains coherent and bounded by registered package
  and evidence exits. The critical path reaches the Property Read gate and
  broad WP-300 completion, then WP-400/WP-500/WP-600 branch and rejoin at
  WP-700 before M7.
- 0025: the WP-300 Property Read slice is finite. It owns one complete
  Producer Property Read registration, both readiness forms, permit-gated
  request acceptance, response delivery, cleanup, and paired host/static
  authors; optional interface presence is not a broad behavior claim.
- 0026: narrow WP-300 completion releases only narrow WP-400 Property Read.
  Broad WP-300 completion remains the necessary release event for broad
  WP-400, WP-500, and WP-600; downstream preparation is allowed but is not
  source admission or vertical progress.
- 0027: no current target request can leak because WP-300 target execution is
  absent, but the negative boundary needed strengthening. New target code may
  not call legacy selectors or dispatch; WP-600 removes concrete calls and
  WP-700 proves final public selector/adapter absence.
- 0028: ADR-0013 remains unchanged. A candidate's requirement, artifact,
  state, resource, dependency, exclusion, and evidence map owns the impact
  boundary; intersecting findings block/reopen, compatible changes receive
  named revalidation, and disjoint findings cannot cause broad re-review.
- 0029: WP-300 reuses the WP-200 immutable candidate and transition model.
  Only contract-specific lifecycle/authoring checks are new. Preparation ends
  at passing prechecks, executable schema, expected absent-source completion
  failure, and reviewed next-state simulation.

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

The semantic candidate passed independent v1 review, the exact
evidence-boundary correction passed independent v2 review, and the exact
pre-source/implementation topology now has completion evidence. The
representation and package ownership decisions are not reopened.

### WP-200 completion boundary

Status: NARROW PROPERTY READ PLAN SLICE COMPLETE.

The immutable topology is:

`4f3bdeff604e30eecfbba9c8c12e6dd0b23cc87f`
`-> ce1e4ae448617458251f4f437e66e77fb652e86b`
`-> b889ae1dafa65ee66bfb331bebf9d537e1c29eee`.

The middle checkpoint changes exactly the five registered pre-source paths.
The implementation child changes exactly the nine registered Core/Planning
paths. `docs/evidence/WP-200-property-read-plan-slice.toml` binds that
implementation ref to the completion checker. The integration gate still has
WP-300 and WP-400 planned/blocked and remains globally `blocked`.

### Aggregate design-check worktree-root defect

Status: RESOLVED.

Commit `eacaaf1242a41861758ebc78a40ada2d88d15bba` removes the compile-time
repository root. Design-check callers export
`CLINKZ_WOT_REPOSITORY_ROOT`; direct runs may discover the nearest runtime
ancestor; explicit invalid roots fail closed. Three unit tests own precedence,
ancestor discovery, and invalid-root rejection. A real shared-target
regression built the checker in a detached temporary worktree, removed that
worktree, and then executed the cached binary from `/tmp` against the current
root successfully. The temporary worktree and 160.9 MiB test target were
removed.

### Disjoint downstream blockers

- Broad WP-100 handler entry still lacks its remaining request/target
  migration, portable async/step admission, no-atomic public-boundary proof,
  and workload/resource evidence.
- The exact WP-300 Property Read slice has its plan dependency but remains
  blocked by exact registration/execution contracts and public authoring
  fixtures; broad WP-300 also waits on later binding/Servient integration
  evidence.
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

Independent v2 review of exact second correction
`d2dcf2e9d2e19c7c2dfa234f96c5303cc3aee24a` on 2026-07-29 passed:

- exact single-parent and six-path topology inspection;
- unchanged semantic specification, public authoring fixtures, work-package
  contract, and registered implementation scope relative to the v1 candidate;
- `tools/check-wp200-property-read-plan-slice-entry.sh --candidate`;
- `tools/check-design-artifacts.sh`;
- `cargo test --workspace --locked`;
- `sh scripts/check-feature-matrix.sh` — 21 passed, 0 failed; and
- `git diff --check
  f453f165c2ea775e5f0d10c36f1e419fcc1d79f3..d2dcf2e9d2e19c7c2dfa234f96c5303cc3aee24a`.

The review also executed the exact next-state transition: the five-file
pre-source checkpoint passed `--admission-ready`; omitting the carry-forward
manifest and introducing an out-of-scope implementation path were both
rejected. Those transition simulations were temporary review mutations and
are not repository evidence commits.

The real five-file pre-source checkpoint from attestation
`4f3bdeff604e30eecfbba9c8c12e6dd0b23cc87f` also passes
`tools/check-wp200-property-read-plan-slice-entry.sh --admission-ready` with no
product source present. Its gate and carried SHA-256 change atomically.

Exact implementation `b889ae1dafa65ee66bfb331bebf9d537e1c29eee` passed on
2026-07-29:

- `tools/check-wp200-property-read-plan-slice.sh`, including Core/Planning
  no-default, async/no-std, and std profiles, both external authoring fixtures,
  and predecessor regressions;
- `tools/check-design-artifacts.sh`;
- `cargo test --workspace --locked`;
- `sh scripts/check-feature-matrix.sh` — 21 passed, 0 failed; and
- exact committed path/topology inspection and diff hygiene.

The shared-target root defect described above caused one false aggregate-check
failure before the design-check package artifact was rebuilt from the current
worktree. The clean rebuild, runtime-root correction, 22 design-check unit
tests, deleted-build-worktree cache regression, and aggregate rerun pass; no
product source changed during that diagnosis.

The intentionally invalid all-features combination enables mutually exclusive
Zenoh backends. Use `scripts/check-feature-matrix.sh`, not
`cargo test --all-features`, as the supported feature baseline.

## Next Safe Actions

1. Inspect the exact WP-300 Property Read binding-slice contract, then prepare
   its non-implementation candidate, public authoring fixtures, schema, entry
   check, completion boundary, and next-state topology under ADR-0013. Do not
   edit WP-300 product source before admission.
2. Independently review the immutable candidate and simulate its exact
   pre-source and implementation transitions before attestation.
3. Before remote source integration, change the GitHub rule so
   `mainline / validation` is a required status and verify that remote
   enforcement. A successful workflow already exists; status enforcement is
   the sole unmigrated part of issue 0023.

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
- `workspace/0025-wp300-property-read-slice-scope.md`
- `workspace/0026-wp300-critical-path-concentration.md`
- `workspace/0027-legacy-migration-boundary-leakage.md`
- `workspace/0028-global-gates-and-scoped-implementation.md`
- `workspace/0029-wp300-admission-machinery-regression.md`
- `docs/spec/planning.md`
- `docs/spec/binding-spi.md`
- `docs/work-packages/property-read-architecture-gate.toml`
- `docs/work-packages/WP-200-planning.md`
- `docs/work-packages/WP-300-bindings.md`
- `docs/audits/WP-200-property-read-plan-slice-entry.md`
- `docs/audits/WP-200-property-read-plan-slice-review.toml`
- `docs/audits/WP-200-property-read-plan-slice-review-v2.toml`
- `docs/evidence/WP-200-property-read-plan-slice.toml`
- `tools/design-check/tests/wp200_binding_artifact_schema.rs`
