# Project State

Last updated: 2026-07-26

Repository basis: D2 migration checkpoint `0092e9c` plus the current
review-pending logical-time correction admission candidate.

## Current Objective

Complete the independent review of the registered Foundation-owned
`WP-100-LOGICAL-TIME-CORRECTION` candidate. Do not begin source implementation
until the exact ADR-0013 attestation and approval checkpoint pass.

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
  `docs/amendments/WP-100-time-domain-v1.md`; new WP-100 evidence must replace
  the historical time claims and reaffirm the generation claims;
- `WP-100-FOUNDATION-REFRESH` is approved, implemented, and complete;
- `WP-100-HANDLER-VALUE-PRIMITIVES` is approved, implemented, and complete
  under `docs/evidence/WP-100-handler-value-primitives.toml`;
- the registered handler-value candidate is commit
  `778c2b60eebc18895604485c4e546cad5bd5e101`, the single child of its frozen
  base, its independent review attestation is
  `db09e5f1635f9544af7d996919600c4b377875ff`, its admission checkpoint is
  `e009c760f915569186136b1f6541784a2fbde3b9`, and its implementation is
  `b5cb1a5392ab2ef35a9f23185731012f71e4ea05`;
- `WP-100-LOGICAL-TIME-CORRECTION` and
  `WP-100-DEADLINE-CLEANUP-TIMING` have frozen identities, ordering,
  ownership, and completion keys;
- the logical-time tranche now has a review-pending candidate, one permitted
  implementation path (`foundation/src/time.rs`), an independent nested
  contract fixture, and executable entry/completion checks, but is not
  admitted;
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

- the decided logical-time correction and dependent Deadline/cleanup tranche
  are not yet admitted or complete;
- four handler workloads lack complete executable matrix oracles;
- request/target/context migration needs a scoped impact review;
- the real no-atomic public boundary is not proven;
- Producer and Servient integration remain assigned to WP-300/WP-400.

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
- broad handler entry remains blocked until both completion keys pass.

Logical-time admission facts:

- the implementation preserves the four public time value/trait
  representations and the `RuntimeClock` method set;
- the only new public method is
  `SourceTimestamp::checked_cmp(self, other) -> Option<Ordering>`;
- the candidate changes no implementation path and its completion checker must
  stop at the missing-method boundary before approval;
- the historical WP-000 evidence blob is frozen and the completion contract
  reruns `tools/check-wp-000.sh` while leaving generation source unchanged;
- the current root session authored the candidate and cannot provide its
  independent review attestation.

Downstream admission blockers:

- WP-200: constructible candidate-fallback policy, health rule,
  pre-side-effect failure set, and bounded diagnostics;
- WP-300: exact registration/compiler/constrained signatures, independent
  host/static authoring fixtures, and subscription receiver ownership;
- WP-400/WP-500/WP-600: their registered WP-300 predecessor;
- WP-700: WP-400, WP-500, and WP-600.

## Verification Baseline

Verified on 2026-07-26 through the D2 authority migration:

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

`cargo test --workspace --all-features --locked` is not a valid project
baseline because it intentionally enables mutually exclusive `zenoh` and
`zenoh-pico` backends. Use the valid feature-matrix script instead.

## Stopping Point and Next Safe Actions

D2 is migrated. The exact logical-time correction admission package is authored
without modifying `foundation/src/time.rs`; its independent review remains
pending. The historical WP-000 evidence file was not rewritten.

Next safe actions, in order:

1. independently review the exact registered logical-time candidate and record
   either findings or the bounded attestation;
2. admit and implement the tranche only after that review evidence;
3. then admit `WP-100-DEADLINE-CLEANUP-TIMING`;
4. continue D3, D4, and D5 as AI-led technical decisions, asking the Owner only
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
- `workspace/0007-time-domain-and-deadline.md`;
- `workspace/0008-implementation-governance-overhead.md`;
- `workspace/0009-minimal-end-to-end-architecture-validation.md`;
- `workspace/0010-complete-design-decomposition.md`;
- `workspace/0011-subscription-receiver-ownership.md`.
