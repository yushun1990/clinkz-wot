# Project State

Last updated: 2026-07-26

Repository basis: clean session-entry commit
`18d85fefec6beba2a027315e7a684e05df6e0e61`.

## Current Objective

Proceed with the smallest evidence-admissible v4.9 implementation work without
waiting for Owner technical approvals.

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
- the next implementation candidate and its `review-pending` admission state
  are unambiguous.

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
  different progress mechanisms.

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

- WP-000 is recorded complete;
- its `time-and-generation-api` evidence is impacted and needs explicit
  replacement or reaffirmation after the time model is frozen;
- `WP-100-FOUNDATION-REFRESH` is approved, implemented, and complete;
- `WP-100-HANDLER-VALUE-PRIMITIVES` is pending with
  `admission_status = "review-pending"` and an entry audit whose verdict is
  independent re-review pending;
- the registered handler-value candidate is commit
  `778c2b60eebc18895604485c4e546cad5bd5e101`, the single child of its frozen
  base, but the entry checker incorrectly requires that historical candidate
  to remain `HEAD`; at current `HEAD` it fails before the six prechecks;
- `WP-100-HANDLER-ENTRY` remains blocked;
- WP-100 is in progress; WP-200 through WP-700 are planned.

The five-value candidate is exactly:

- `CancellationView`;
- `SubscriptionAcceptance`;
- `HandlerFootprint`;
- `HandlerStep`;
- `StaticHandlerRegistration`.

It excludes `Deadline`, time semantics, state machines, Servient, Protocol
Bindings, and performance workloads. Its intended implementation paths are
`core/src/handler.rs` and the Core root re-export only.

Risk classification: Category A implementation risk, because the five values
are passive, additive, locally owned, and disjoint from the time-domain
blocking scope. The tranche still cannot be implemented until its registered
admission state changes from `review-pending` to `approved`.

## Current Blockers and Open Technical Decisions

Immediate baseline defect:

- resolved. `docs/artifacts.csv` now registers
  `workspace/0007-time-domain-and-deadline.md` as a non-normative workspace
  artifact so the `TIME-DOMAIN-AND-DEADLINE` blocking topic is registered and
  present.

AI-led open decisions:

- D2: freeze the time-domain/Deadline direction and decide the corrective
  tranche/evidence disposition;
- D3: decide the residual `docs/design.md` decomposition strategy;
- D4: freeze subscription receiver/control ownership and clone semantics;
- D5: decide whether and how to add the minimal mock-binding property-read
  integration gate.

Broad handler blockers:

- finite raw clock wrap, Deadline ordering, SourceTimestamp, and CleanupRecord
  timing do not share one coherent comparison domain;
- the impacted WP-000 time evidence has no disposition;
- four handler workloads lack complete executable matrix oracles;
- request/target/context migration needs a scoped impact review;
- the real no-atomic public boundary is not proven;
- Producer and Servient integration remain assigned to WP-300/WP-400.

Handler-value admission checkpoint drift:

- `tools/check-wp100-handler-value-primitives-entry.sh --candidate` fails with
  `candidate commit must have parent ...` because the checker compares the
  current `HEAD` to the historical candidate base;
- the reviewed candidate commit and exact path set remain recoverable in Git,
  but the direct-child/HEAD coupling cannot survive later governance commits;
- do not rewrite published history to recreate the old checkout shape;
- refresh or supersede the admission checkpoint mechanics while preserving the
  exact five-value scope, exclusions, predecessor, executable contract, and
  honest independent-review requirement.

Downstream admission blockers:

- WP-200: constructible candidate-fallback policy, health rule,
  pre-side-effect failure set, and bounded diagnostics;
- WP-300: exact registration/compiler/constrained signatures, independent
  host/static authoring fixtures, and subscription receiver ownership;
- WP-400/WP-500/WP-600: their registered WP-300 predecessor;
- WP-700: WP-400, WP-500, and WP-600.

## Verification Baseline

Verified on 2026-07-26 against `18d85fe` plus the M0 state/plan closure update:

- `tools/check-design-artifacts.sh` - passed;
- `tools/check-design-requirements.sh` - passed;
- `tools/check-api-ownership.sh` - passed;
- `cargo run --locked --quiet --manifest-path tools/design-check/Cargo.toml -- check-work-packages` - passed;
- `git diff --check` - passed;
- `cargo test --workspace --locked` - passed;
- `sh scripts/check-feature-matrix.sh` - 21 valid feature combinations passed.

`cargo test --workspace --all-features --locked` is not a valid project
baseline because it intentionally enables mutually exclusive `zenoh` and
`zenoh-pico` backends. Use the valid feature-matrix script instead.

## Stopping Point and Next Safe Actions

No source implementation has been changed since the latest governance
checkpoint.

Next safe actions, in order:

1. repair the stale, history-coupled candidate checkpoint without weakening
   the tranche's semantic admission controls or rewriting published history;
2. complete the AI-led technical admission review for
   `WP-100-HANDLER-VALUE-PRIMITIVES` and produce the registered attestation or
   record why the tranche remains pending;
3. if the tranche becomes admitted, implement exactly the five values in
   `core/src/handler.rs` and `core/src/lib.rs`, then run the registered
   completion evidence;
4. independently process D2 time-domain/Deadline and replace or reaffirm
   impacted WP-000 time evidence;
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
- `workspace/0007-time-domain-and-deadline.md`;
- `workspace/0008-implementation-governance-overhead.md`;
- `workspace/0009-minimal-end-to-end-architecture-validation.md`;
- `workspace/0010-complete-design-decomposition.md`;
- `workspace/0011-subscription-receiver-ownership.md`.
