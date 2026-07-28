# Project State

Last updated: 2026-07-28

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

The activation candidate changed exactly 27 documentation/checker paths and no
Rust source, Cargo manifest, public API, or runtime behavior. Its independent
review passed the exact candidate checker, aggregate design/evidence suite,
default workspace tests, diff hygiene, and the 21-cell valid feature matrix.

## Current Objective

Freeze and validate the exact non-implementation
`WP-200-PROPERTY-READ-PLAN-SLICE` candidate, register its immutable commit, and
stop at the independent-review boundary without self-attesting.

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

The candidate commit is the single child of
`525bb31b42efe299ed36d46acea1a1c4286e8bde`. Before review, that exact
single-child/path boundary identifies the candidate without a self-referential
hash-registration commit. The independent attestation records its immutable
hash.

The next source-changing event remains the independently reviewed and admitted
WP-200 slice. No registered Core/Planning implementation path or either
cross-package Property Read architecture fixture root is admitted before that
checkpoint.

## Active Milestones

- M0 Execution Baseline and Collaboration Reset — CLOSED.
- M1 v5.0 Authority Reset and Architecture Closure — IN_PROGRESS.
- M2 Foundation and Core Contract Stabilization — IN_PROGRESS.
- M3 Planning and Compilation Pipeline — OPEN; exact entry candidate review
  pending.

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

The exact candidate still requires immutable-ref registration and independent
review. Independent review, admission, implementation, and completion evidence
are execution states, not reopened design questions.

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

The candidate is ready for independent review. Its independent attestation,
not an intermediate metadata checkpoint, records the immutable candidate ref.

The intentionally invalid all-features combination enables mutually exclusive
Zenoh backends. Use `scripts/check-feature-matrix.sh`, not
`cargo test --all-features`, as the supported feature baseline.

## Next Safe Actions

1. Commit the exact D8/WP-200 non-implementation packet as the single child of
   its frozen base, run candidate and aggregate checks, and stop without
   creating a review attestation or hash-registration checkpoint.
2. In a later independent root continuation, review and mutation-test that
   exact commit; the attestation records its immutable hash.
3. Only after a passing attestation, record one combined, recoverable
   pre-source approval/in-progress checkpoint, implement exactly the nine named
   paths, and produce completion evidence.

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
- `tools/design-check/tests/wp200_binding_artifact_schema.rs`
