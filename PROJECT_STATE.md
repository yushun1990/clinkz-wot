# Project State

Last updated: 2026-07-26

Repository basis: handler-context review-failure checkpoint
`64d3b79e39f3a922f090c14319718a1684f8dca4` plus replacement candidate
`27c8dc974fd1b337de4671c81719f9fa32410c56`. The replacement remains a single
child of frozen base `c42abcbc339167183c8c4bf9bd3bf584540073b4` and retains
the exact 17-path non-implementation boundary.

## Current Objective

Independently review replacement `WP-100-HANDLER-CONTEXT` candidate
`27c8dc974fd1b337de4671c81719f9fa32410c56` before changing Core source.
Preserve the completed handler-value and time tranches, keep the remaining
target/no-atomic and request-storage migrations separate, and do not fold
Producer or Servient integration into WP-100.

Active milestones:

- M0 Execution Baseline and Collaboration Reset - CLOSED;
- M1 v4.9 Architecture and Authority Closure - IN_PROGRESS;
- M2 Foundation and Core Contract Stabilization - IN_PROGRESS.

M0 is closed from current repository evidence:

- the Owner/AI decision model is corrected in `AGENTS.md`,
  `PROJECT_GOVERNANCE.md`, and `PLAN.md`;
- the baseline artifact-registry defect for
  `workspace/0007-time-domain-and-deadline.md` is fixed;
- D1 risk-proportional implementation admission is decided and migrated;
- `ARCHITECTURE_GOVERNANCE.md` now owns only technical convergence,
  architecture authority, and design-change control rather than duplicating
  execution planning;
- default tests, all 21 valid feature combinations, and the aggregate
  design-artifact checks pass against `18d85fe` plus the state/plan closure
  update;
- the first scoped Category A implementation tranche is now admitted,
  implemented, and independently evidenced.

## AI-led Collaboration Model

ClinkZ-WoT uses AI-led development.

Facts:

- AI owns technical architecture, API shape, work-package decomposition,
  implementation order, technical risk, evidence sufficiency, and technical
  milestone status.
- The Owner owns project vision, target outcomes, real-world constraints,
  unacceptable directions, product trade-offs, usage feedback, and actual
  public release execution.
- Owner questions, doubts, counterexamples, and usage feedback are evidence
  inputs. They are not direct technical instructions, not predetermined
  answers, and not automatic blockers for unrelated work.
- AI must investigate workspace topics against architecture, code, tests,
  specifications, audits, reviews, work packages, and evidence, then decide,
  record rationale, migrate conclusions, and verify results.
- AI asks the Owner only when a choice depends on goals, product trade-offs,
  real-world constraints, unacceptable directions, or irreversible external
  commitments beyond technical evidence.
- AI closes technical milestones from registered exit criteria and repository
  evidence. Later Owner-provided goal conflicts, missing constraints, or
  credible counterexamples can reopen a milestone or decision.
- AI determines technical release readiness. The Owner decides whether and
  when to execute an actual public release.

Workspace lifecycle:

- Owner or AI may create `OPEN` topics;
- AI investigates and advances topics through `DISCUSSING` to `DECIDED`;
- after projection into authoritative docs, work packages, source, tests, or
  governance, the topic becomes `MIGRATED`;
- later target constraints or credible counterexamples can reopen a topic or
  create a linked follow-up.

## Verified Project Model

The active target is the v4.9 architecture-closure candidate. v4.8 is migration
input, not active implementation authority.

The architecture direction already accepted by the registered sources is:

- immutable admitted plan sets separate logical plans from binding-owned
  artifacts;
- Protocol Bindings are ordinary Cargo-linked, application-registered crates;
- one Servient uses startup-only binding composition for v1;
- Servient owns activation, orchestration, plan-set lifetime, scheduling, and
  cleanup policy;
- bindings own protocol adaptation, I/O, correlation, and binding-local state,
  but do not select or dispatch handlers;
- every retained operation has generation ownership, bounds, cancellation, and
  terminal cleanup;
- host and `no_std + alloc` profiles preserve protocol-neutral semantics with
  different progress mechanisms;
- one live `ClockId` exposes non-wrapping extended logical ticks; raw wrap
  metadata is diagnostic only, and incomparable clock domains fail explicitly.

Authoritative implementation order remains:

`WP-000 -> WP-100 -> WP-200 -> WP-300 -> {WP-400, WP-500, WP-600} -> WP-700`.

ADR-0013 allows an exact, independently reviewed implementation tranche to
proceed while aggregate global gates are open when the tranche is proven
disjoint from their findings. Package status alone is not admission.

D1 adds risk-proportional admission authoring and review depth:

- Category A local additive work receives narrow scope/evidence review;
- Category B cross-module contract work keeps explicit work-package,
  ownership, dependency, fixture, audit, and impact review;
- Category C architecture or invariant changes require workspace
  investigation, authoritative migration, evidence invalidation or
  reaffirmation, and architecture review where required.

D1 does not weaken ADR-0013 or authorize runtime/public API work before the
specific tranche is admitted.

## Evidence and Status

Global gates:

- GATE-3 Directory client boundary - closed by carry-forward review;
- GATE-1 API ownership - open;
- GATE-2 lifecycle and cleanup - open;
- GATE-4 resource limits - open;
- GATE-5 performance contracts - open;
- GATE-6 implementation DAG/conformance - open.

Work packages and tranches:

- WP-000 is recorded complete; its historical `time-and-generation-api` file
  remains immutable;
- D2 is migrated through ADR-0016 and
  `docs/amendments/WP-100-time-domain-v1.md`; new WP-100 logical-time evidence
  replaces the historical time claims and reaffirms the generation claims;
- `WP-100-FOUNDATION-REFRESH` is approved, implemented, and complete;
- `WP-100-HANDLER-VALUE-PRIMITIVES` is approved, implemented, and complete
  under `docs/evidence/WP-100-handler-value-primitives.toml`;
- the registered handler-value candidate is commit
  `778c2b60eebc18895604485c4e546cad5bd5e101`, the single child of its frozen
  base, its independent review attestation is
  `db09e5f1635f9544af7d996919600c4b377875ff`, its admission checkpoint is
  `e009c760f915569186136b1f6541784a2fbde3b9`, and its implementation is
  `b5cb1a5392ab2ef35a9f23185731012f71e4ea05`;
- `WP-100-LOGICAL-TIME-CORRECTION` is approved, implemented, and complete
  under `docs/evidence/WP-100-logical-time-correction.toml`;
- `WP-100-DEADLINE-CLEANUP-TIMING` is approved, implemented, and complete
  under `docs/evidence/WP-100-deadline-cleanup-timing.toml`; its candidate is
  `0fc9c6eee2a37cd420d9cd8beae537f3ed54c369`, its independent attestation is
  `e37224fc9978ad16c0d76639bac58f0e221d824d`, its admission checkpoint is
  `1d85953d6c473006dee53ea1691702dd25c70f20`, its progress checkpoint is
  `65b8bdcc7182a6b65188e29a3cb99b04fd1c6d09`, and its implementation is
  `92d10f38065397222646ac9e36df256467a7f514`;
- the completed logical-time tranche's exact registered candidate is
  `88eebbba04d070fb423a9c2ec227ddc470790769`; the candidate is the single child
  of D2 base `0092e9c5f3c7e041ec979d18d462fd0e74ae2e0a`, changes exactly the
  14 registered non-implementation paths, its independent attestation is
  `8180e1916606b80518a0d975a8ce2fa2c2e93953`, its admission checkpoint is
  `a53b0fe07be1402c568808133a8289dfa6b04281`, its progress checkpoint is
  `9f3c354f4d743e1ce481d181b234e91883fb04fd`, and its implementation is
  `961fe9b22f241459ff374897d98d805e79af3f71`;
- `WP-100-HANDLER-CONTEXT` remains review-pending. Its Category B scope adds
  only the borrowed `HandlerContext<'a>` contract, permits implementation
  changes only in `core/src/handler.rs` and `core/src/lib.rs`, and has
  completion key `handler-context`. Replacement candidate
  `27c8dc974fd1b337de4671c81719f9fa32410c56` is the single child of base
  `c42abcbc339167183c8c4bf9bd3bf584540073b4`, changes exactly the 17
  registered non-implementation paths, and replaces the failed sampled
  compatibility fixture with an exhaustive 72-pairing matrix;
- `WP-100-HANDLER-ENTRY` remains blocked;
- WP-100 is in progress; WP-200 through WP-700 are planned.

The completed five-value tranche is exactly:

- `CancellationView`;
- `SubscriptionAcceptance`;
- `HandlerFootprint`;
- `HandlerStep`;
- `StaticHandlerRegistration`.

It excludes `Deadline`, time semantics, state machines, Servient, Protocol
Bindings, and performance workloads. Its implementation paths are exactly
`core/src/handler.rs` and the Core root re-export.

Risk classification: Category A implementation risk. The five values are
passive, additive, locally owned, and disjoint from the time-domain blocking
scope. Independent admission, implementation, and completion evidence all
passed without expanding the tranche.

## Current Blockers and Open Technical Decisions

AI-led open decisions:

- D3: decide the residual `docs/design.md` decomposition strategy;
- D4: freeze subscription receiver/control ownership and clone semantics;
- D5: decide whether and how to add the minimal mock-binding property-read
  integration gate.

Broad handler blockers:

- four handler workloads lack complete executable matrix oracles;
- the remaining `AffordanceTarget` relocation and replacement still needs its
  real incapable-target/no-atomic evidence boundary;
- `AcceptHint` resource admission and `InteractionInput` private request
  storage require a coordinated downstream construction-site migration;
- the 54 portable traits remain unadmitted until their exact dependency and
  external matrix boundary is frozen;
- the real no-atomic public boundary is not proven;
- Producer and Servient integration remain assigned to WP-300/WP-400.

Handler-context candidate facts:

- impact inspection separated three validation truths: additive dispatch
  identity, target/no-atomic replacement, and request/resource migration;
- `HandlerContext` is independently dependency-complete because it borrows the
  existing target and composes already implemented generation-bearing ids;
- the tranche validates exactly the 18 operation/target pairings and maps an
  incompatible pair to `Validation/Validate/Never` with fixed-size Thing slot,
  operation, plan, and optional binding context;
- it excludes `AffordanceTarget`, `AcceptHint`, `InteractionInput`, all 54
  traits, host erasure/storage, runtime callbacks, state machines, resources,
  performance workloads, Producer, Servient, and Protocol Bindings;
- its external fixture freezes private fields, absence of `Hash` and
  `Default`, three Core feature cells, and downstream compilation;
- the handler-value source checker remains exact for its five owned values and
  explicitly permits only the separately validated `HandlerContext` addition;
- superseded candidate `5b1feb64ffde42ac9104540b851cb72360f91426`
  tested all 18 valid pairings but only four of the 54 invalid pairings, while
  its source checker deliberately ignored method bodies;
- an isolated mutation against that candidate that additionally accepted
  `ReadProperty + AffordanceTarget::Action` passed the complete
  `tools/check-wp100-handler-context.sh`, proving that it could not support its
  exact compatibility-matrix completion claim;
- replacement candidate `27c8dc974fd1b337de4671c81719f9fa32410c56`
  tests all 72 pairings, accepts exactly 18, rejects exactly 54, and exercises
  both present and absent optional binding error context;
- the replacement fixture passed against a conforming isolated implementation
  and rejected the known incompatible-pair mutation; and
- no review attestation or approval exists for either candidate.

Completed handler-value tranche facts:

- the stale current-`HEAD` candidate coupling was corrected without rewriting
  published history or weakening the exact candidate/path checks;
- an independent root continuation review rechecked the candidate diff,
  schemas, authoritative owners, predecessor, time-scope partition, empty
  state/performance scope, compile contract, source validator, and prechecks;
- the approval and progress checkpoints preceded the exact two-file
  implementation commit;
- the completion checker proves the actual source projection, three required
  feature cells, semantic and negative UI contracts, redacted Debug behavior,
  and downstream Servient/Protocol Binding compilation;
- no intersecting finding or out-of-scope implementation need was found.

Migrated D2 facts:

- raw finite counters are extended inside the clock adapter; Core never
  reconstructs wrap epochs;
- reset, lost epoch state, scale change, or adapter restart retires the old
  non-reused clock id;
- positive-duration admission fails before logical `u64` overflow, while
  already admitted work may expire and drain at saturation;
- monotonic source timestamps compare only when clock id and scale match;
- caller-provided incomparability maps to
  `Validation/Admission/Never`; post-admission clock-domain loss maps to
  `InternalInvariant/Never` in the owning Handler, Binding, or Cleanup phase;
- `WP-100-LOGICAL-TIME-CORRECTION` precedes and is evidence-distinct from
  `WP-100-DEADLINE-CLEANUP-TIMING`;
- the completed logical-time and deadline/cleanup keys resolve the former
  time-domain blocker; broad handler entry remains blocked by its non-time
  admission and evidence gaps.

Completed logical-time tranche facts:

- the implementation preserves the four public time value/trait
  representations and the `RuntimeClock` method set;
- the only new public method is
  `SourceTimestamp::checked_cmp(self, other) -> Option<Ordering>`;
- `MonotonicInstant` and monotonic source timestamps now document extended
  logical ticks, while `wrap_period_ticks()` is raw-source diagnostic metadata
  only;
- SourceTimestamp comparison rejects mixed kinds, `Unknown`, different clock
  ids, and conflicting tick scales;
- the candidate, independent attestation, approval, progress, and exact
  one-file implementation checkpoints precede completion evidence;
- the historical WP-000 evidence blob remains unchanged; the new record
  replaces its `API-SOURCE-TIME-001` and `TIME-001` claims and reaffirms
  `API-TYPES-001` plus `CONSTRAINED-STORAGE-001` with unchanged generation
  source and a passing `tools/check-wp-000.sh`.

Completed deadline/cleanup tranche facts:

- the exact committed candidate contains no Core or Foundation implementation
  change;
- its direct predecessor is the complete logical-time tranche, whose checker
  passes from the candidate revision;
- the requirement, owner, feature-cell, API-item, implementation-path,
  contract-artifact, state/resource/performance/removal, and completion-key
  sets match the frozen time amendment;
- the nested fixture covers Deadline NONE/finite/delayed/incomparable results,
  CleanupRecord checked ordering, all four incomparable-clock dispositions,
  timeout linearization, and private Deadline storage;
- `tools/check-wp100-deadline-cleanup-timing-entry.sh --candidate` and the
  admission-ready mode both passed before implementation;
- the independent root review found no intersecting open finding or
  out-of-scope implementation need;
- the approval and exact two-file progress checkpoints precede the exact
  three-file Core implementation commit;
- `Deadline::NONE` never elapses, finite deadlines use checked extended logical
  ordering, and different clock ids return `None`;
- `CleanupRecord::try_with_timing` rejects clock mismatch, incomparable
  ordering, and retry instants after the terminal deadline; and
- the completion checker proves three Core feature cells, semantic and privacy
  fixtures, all four incomparable-clock dispositions, timeout linearization,
  Core unit tests, and the logical-time predecessor regression suite.

Downstream admission blockers:

- WP-200: constructible candidate-fallback policy, health rule,
  pre-side-effect failure set, and bounded diagnostics;
- WP-300: exact registration/compiler/constrained signatures, independent
  host/static authoring fixtures, and subscription receiver ownership;
- WP-400/WP-500/WP-600: their registered WP-300 predecessor;
- WP-700: WP-400, WP-500, and WP-600.

## Verification Baseline

Verified on 2026-07-26 through the deadline/cleanup implementation and
completion state:

- `tools/check-design-artifacts.sh` - passed;
- `tools/check-design-requirements.sh` - passed;
- `tools/check-api-ownership.sh` - passed;
- `tools/check-architecture-adrs.sh` - sixteen accepted decisions passed;
- `tools/check-wp100-handler-amendment.sh` - passed with the frozen Deadline
  projection;
- `cargo run --locked --quiet --manifest-path tools/design-check/Cargo.toml -- check-work-packages` - passed;
- `tools/check-wp100-handler-value-primitives-entry.sh --candidate` - passed;
- `tools/check-wp100-handler-value-primitives-entry.sh --admission-ready` -
  passed before implementation;
- `tools/check-wp100-handler-value-primitives.sh` - passed;
- `cargo fmt --manifest-path tools/design-check/Cargo.toml -- --check` - passed;
- `bash -n tools/check-architecture-adrs.sh` and
  `bash -n tools/check-wp100-handler-amendment.sh` - passed;
- `git diff --check` - passed;
- `cargo test --workspace --locked` - passed;
- `sh scripts/check-feature-matrix.sh` - 21 valid feature combinations passed.

Logical-time admission and completion verification on 2026-07-26:

- `tools/check-wp100-logical-time-correction-entry.sh --candidate` - passed;
- `tools/check-wp100-logical-time-correction-entry.sh --admission-ready` -
  passed before implementation;
- `tools/check-wp100-logical-time-correction.sh` - passed after implementation
  across all three Foundation feature cells, the independent semantic fixture,
  and the WP-000 regression suite;
- `cargo clippy --locked -p clinkz-wot-foundation --all-targets --all-features -- -D warnings` -
  passed;
- nested fixture metadata, lockfile, raw-wrap extension, delayed observation,
  reset, scale mismatch, and exhaustion checks - passed;
- the candidate/base diff contains the exact registered 14 paths and no
  `foundation/src/time.rs` change;
- the implementation commit changes only `foundation/src/time.rs`;
- the historical WP-000 evidence and generation source blobs are unchanged.

Deadline/cleanup admission and completion verification on 2026-07-26:

- `tools/check-wp100-deadline-cleanup-timing-entry.sh --candidate` - passed;
- `tools/check-wp100-deadline-cleanup-timing-entry.sh --admission-ready` -
  passed before implementation;
- `tools/check-wp100-deadline-cleanup-timing.sh` - passed after implementation
  across all three Core feature cells, the independent semantic and privacy
  fixtures, all 59 no-std Core tests, and the logical-time predecessor
  regression suite;
- `cargo clippy --locked -p clinkz-wot-core --all-targets --all-features -- -A clippy::large_enum_variant -D warnings` -
  passed; the scoped allow is for the pre-existing frozen `HandlerStep<R>`
  representation and the admitted implementation added no lint finding;
- the candidate/base diff contains the exact registered 15 non-implementation
  paths;
- the implementation commit changes only `core/src/deadline.rs`,
  `core/src/lib.rs`, and `core/src/status.rs`;
- `tools/check-design-artifacts.sh`,
  `cargo run --quiet --manifest-path tools/design-check/Cargo.toml -- check-work-packages`,
  `cargo test --workspace --locked`, and the 21-cell valid feature matrix all
  passed in completion state; and
- the historical WP-000 evidence and both predecessor implementation commits
  are unchanged.

Handler-context candidate verification on 2026-07-26:

- `tools/check-wp100-handler-context-entry.sh --candidate` - passed after the
  exact candidate ref was registered;
- the candidate/base diff contains exactly the 17 registered
  non-implementation paths, and neither permitted Core implementation path
  changed;
- the design checker passed its 16 unit and four integration tests, while the
  completed handler-value checker remained green after its source projection
  was extended to permit only the separately validated `HandlerContext`;
- `tools/check-wp100-handler-context.sh` reached its intended pre-admission
  stop only because the Core `HandlerContext` implementation is absent;
- `tools/check-design-artifacts.sh` passed in registered-candidate state,
  validating governance and all six refactor gates; and
- isolated negative mutation verification proved that an unregistered invalid
  pairing could pass the superseded completion fixture;
- the replacement fixture passed against a conforming isolated implementation
  and rejected that mutation;
- the superseded candidate's independent review verdict is failed for evidence
  sufficiency; and
- no independent-review attestation, approval, progress checkpoint, or Core
  implementation exists.

`cargo test --workspace --all-features --locked` is not a valid project
baseline because it intentionally enables mutually exclusive `zenoh` and
`zenoh-pico` backends. Use the valid feature-matrix script instead.

## Stopping Point and Next Safe Actions

The failed `WP-100-HANDLER-CONTEXT` candidate has been superseded by exact
candidate `27c8dc974fd1b337de4671c81719f9fa32410c56`, which preserves the frozen
base and path boundary while closing the executable compatibility-matrix gap.
Core implementation remains unchanged, and the completed time and
handler-value evidence remains in force.

Next safe actions, in order:

1. run the registered candidate entry check for replacement
   `27c8dc974fd1b337de4671c81719f9fa32410c56`;
2. independently review the replacement candidate against base
   `c42abcbc339167183c8c4bf9bd3bf584540073b4`;
3. only if that review passes, record the attestation-only commit and exact
   three-file admission checkpoint before either Core source path changes;
4. create the exact two-file progress checkpoint, then implement and evidence
   `HandlerContext` only within the admitted scope;
5. continue D3, D4, and D5 as AI-led technical decisions, asking the Owner only
   for project-goal, constraint, or external-commitment clarification.

Important references:

- `AGENTS.md`;
- `PROJECT_GOVERNANCE.md`;
- `PLAN.md`;
- `ARCHITECTURE_GOVERNANCE.md`;
- `docs/architecture/README.md`;
- `docs/work-packages/index.toml`;
- `docs/audits/WP-100-handler-value-primitives-entry.md`;
- `docs/audits/WP-100-handler-value-primitives-review.toml`;
- `docs/evidence/WP-100-handler-value-primitives.toml`;
- `docs/ADRs/0016-extended-logical-monotonic-time.org`;
- `docs/amendments/WP-100-time-domain-v1.md`;
- `docs/audits/WP-100-logical-time-correction-entry.md`;
- `docs/audits/WP-100-logical-time-correction-review.toml`;
- `docs/evidence/WP-100-logical-time-correction.toml`;
- `docs/audits/WP-100-deadline-cleanup-timing-entry.md`;
- `docs/audits/WP-100-deadline-cleanup-timing-review.toml`;
- `docs/evidence/WP-100-deadline-cleanup-timing.toml`;
- `docs/audits/WP-100-handler-context-entry.md`;
- `tools/check-wp100-handler-context-entry.sh`;
- `tools/check-wp100-handler-context.sh`;
- `workspace/0007-time-domain-and-deadline.md`;
- `workspace/0008-implementation-governance-overhead.md`;
- `workspace/0009-minimal-end-to-end-architecture-validation.md`;
- `workspace/0010-complete-design-decomposition.md`;
- `workspace/0011-subscription-receiver-ownership.md`.
