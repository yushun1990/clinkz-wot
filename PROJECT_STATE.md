# Project State

Last reconstructed: 2026-07-26

Repository basis: commit `a46ac13` plus the current uncommitted planning-state
changes to `PLAN.md` and this file.

## Current Objective

Obtain Owner review of the reconstructed milestone plan, restore a passing
execution baseline, and then continue the smallest independently admissible
v4.9 implementation tranche without hiding the global architecture blockers.

Active milestones:

- M0 Execution Baseline and Collaboration Reset — IN_PROGRESS;
- M1 v4.9 Architecture and Authority Closure — IN_PROGRESS;
- M2 Foundation and Core Contract Stabilization — IN_PROGRESS.

`PLAN.md` is in Owner review. The accepted architecture and
`docs/work-packages/index.toml` remain authoritative until the Owner approves a
plan or direction change.

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

## Evidence and Status

Global gates:

- GATE-3 Directory client boundary — closed by carry-forward review;
- GATE-1 API ownership — open;
- GATE-2 lifecycle and cleanup — open;
- GATE-4 resource limits — open;
- GATE-5 performance contracts — open;
- GATE-6 implementation DAG/conformance — open.

Work packages and tranches:

- WP-000 is recorded complete;
- its `time-and-generation-api` evidence is impacted and needs explicit
  replacement or reaffirmation after the time model is frozen;
- `WP-100-FOUNDATION-REFRESH` is approved, implemented, and complete;
- `WP-100-HANDLER-VALUE-PRIMITIVES` is pending with
  `admission_status = "review-pending"` and an entry audit whose verdict is
  independent re-review pending;
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

## Current Blockers

Immediate execution blocker:

- `tools/check-design-artifacts.sh` fails because
  `docs/work-packages/index.toml` references
  `workspace/0007-time-domain-and-deadline.md`, but that path is not registered
  by `docs/artifacts.csv` or a reviewed alias. The checker reports:
  `TIME-DOMAIN-AND-DEADLINE "blocking_topic" is not registered and present`.
  This must be corrected and rechecked before claiming a clean baseline or
  completing the handler-value entry review.

Broad handler blockers:

- finite raw clock wrap, Deadline ordering, SourceTimestamp, and CleanupRecord
  timing do not share one coherent comparison domain;
- the impacted WP-000 time evidence has no disposition;
- four handler workloads lack complete executable matrix oracles;
- request/target/context migration needs a scoped impact review;
- the real no-atomic public boundary is not proven;
- Producer and Servient integration remain assigned to WP-300/WP-400.

Downstream admission blockers:

- WP-200: constructible candidate-fallback policy, health rule,
  pre-side-effect failure set, and bounded diagnostics;
- WP-300: exact registration/compiler/constrained signatures, independent
  host/static authoring fixtures, and subscription receiver ownership;
- WP-400/WP-500/WP-600: their registered WP-300 predecessor;
- WP-700: WP-400, WP-500, and WP-600.

Governance/document debt:

- `ARCHITECTURE_GOVERNANCE.md` is titled and framed as an execution plan and
  duplicates material that belongs to `PLAN.md`, even though `AGENTS.md`
  assigns it technical-convergence governance;
- the risk-proportional admission proposal is open and cannot be applied until
  the Owner accepts it and it is migrated;
- `docs/design.md` still owns residual detailed contracts and has no accepted
  completion strategy for decomposition.

## Owner Decisions Awaited

The ordered decision queue is maintained in `PLAN.md`:

1. approve or revise the new plan and decide the risk-proportional admission
   proposal;
2. freeze the time-domain/Deadline direction;
3. select the residual `docs/design.md` decomposition strategy;
4. freeze subscription receiver/control ownership and clone semantics;
5. decide whether to add the minimal mock-binding property-read integration
   gate and narrow cross-package tranches.

No workspace proposal above is accepted merely because it is listed in the
plan.

## Verification Baseline

Verified on 2026-07-26:

- `cargo test --workspace --locked` — passed;
- `sh scripts/check-feature-matrix.sh` — 21 valid feature combinations passed;
- `tools/check-design-artifacts.sh` — failed only after its earlier checks
  passed, at the unregistered `TIME-DOMAIN-AND-DEADLINE` blocking topic.

The design-artifact run passed requirement indexing, API ownership,
architecture ADR registration, Directory scope, resource schema, WP-100
amendments, performance fixtures, and state/handler structural checks before
the failure.

`cargo test --workspace --all-features --locked` is not a valid project
baseline because it intentionally enables mutually exclusive `zenoh` and
`zenoh-pico` backends. It failed at the explicit mutual-exclusion guard. Use
the valid feature-matrix script instead.

## Stopping Point and Next Safe Actions

No source implementation was changed in this planning session.

Next safe actions, in order:

1. obtain Owner review of `PLAN.md`;
2. register or otherwise correctly project the time-domain blocking topic,
   then rerun the aggregate design-artifact check;
3. prepare the bounded Owner decision package for risk-proportional admission;
4. after the baseline passes, complete the independent
   `WP-100-HANDLER-VALUE-PRIMITIVES` entry re-review;
5. implement the five values only if the tranche becomes explicitly approved;
6. independently prepare the time-domain decision package without mixing it
   into the five-value tranche.

Important references:

- `PLAN.md`;
- `PROJECT_GOVERNANCE.md`;
- `ARCHITECTURE_GOVERNANCE.md`;
- `docs/architecture/README.md`;
- `docs/work-packages/index.toml`;
- `docs/reviews/review-05.org`;
- `docs/reviews/review-06.org`;
- `docs/audits/WP-100-handler-value-primitives-entry.md`;
- `workspace/0007-time-domain-and-deadline.md`;
- `workspace/0008-implementation-governance-overhead.md`;
- `workspace/0009-minimal-end-to-end-architecture-validation.md`;
- `workspace/0010-complete-design-decomposition.md`;
- `workspace/0011-subscription-receiver-ownership.md`.
