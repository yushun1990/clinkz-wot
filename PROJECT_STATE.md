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

Resolve workspace issue 0014 as D8 and convert the result directly into one
exact `WP-200-PROPERTY-READ-PLAN-SLICE` decision/admission packet.

The finite blocker set is:

1. one constructible Core-owned portable binding compiler/artifact SPI;
2. one host-erased representation and one allocation-free static
   representation of that same semantic contract;
3. one implementation owner for each Core SPI and Planning compiler value,
   eliminating the WP-200/WP-300 duplication;
4. paired third-party host and constrained authoring fixtures using only
   public contracts;
5. an immutable Property Read planning input and artifact proving no runtime
   TD read; and
6. one exact tranche candidate with source boundary, feature cells,
   dependencies, exclusions, prechecks, audit, and completion key.

Design closure occurs when those six outputs coexist in the authoritative
planning/binding/work-package projections and their pre-implementation checks
pass. Under D9 they share one conversion packet and one scoped independent
review unless investigation proves a distinct rollback or evidence boundary.

The next source-changing event is the independently admitted WP-200 Property
Read planning slice: creation of the exact Core SPI and Planning compiler
implementation paths named by that packet. No Planning, Core, TD, Binding,
Servient, or architecture-fixture source is admitted before that checkpoint.

## Active Milestones

- M0 Execution Baseline and Collaboration Reset — CLOSED.
- M1 v5.0 Authority Reset and Architecture Closure — IN_PROGRESS.
- M2 Foundation and Core Contract Stabilization — IN_PROGRESS.
- M3 Planning and Compilation Pipeline — OPEN; D8 is its entry blocker.

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

Status: OPEN and the sole design blocker on the executable WP-200 critical
path.

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

The investigation must select a constructible public Rust representation. It
must not merely rename the existing open prose.

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

The intentionally invalid all-features combination enables mutually exclusive
Zenoh backends. Use `scripts/check-feature-matrix.sh`, not
`cargo test --all-features`, as the supported feature baseline.

## Next Safe Actions

1. Decide issue 0014 from repository evidence.
2. Migrate D8 and construct its paired authoring fixtures, exact tranche
   candidate, audit, and checker in one conversion packet.
3. Obtain independent review of that exact packet.
4. Record one pre-source admission checkpoint, then implement only the named
   WP-200 source paths and produce completion evidence.

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
- `docs/work-packages/WP-300-protocol-binding-spi.md`
