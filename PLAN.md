# ClinkZ-WoT Project Plan

## Plan Status

Plan revision: Owner review

Active design revision: v4.9 architecture-closure candidate

Release target: ClinkZ-WoT v1, a protocol-neutral W3C WoT runtime with a
stable Servient architecture and Zenoh binding support.

This plan records roadmap, milestones, dependencies, status, and acceptance
objectives. It does not define architecture or implementation admission.
Authoritative behavior and package-level execution contracts remain in the
registered specifications and `docs/work-packages/index.toml`.

The Owner must approve this revised milestone structure before M0 closes.
Until then, already accepted architecture decisions and the current
work-package DAG remain authoritative.

## Planning Baseline

The repository evidence establishes the following starting point:

- the v4.9 architecture backbone exists but is still a closure candidate;
- GATE-3, the Directory client boundary, is closed; GATE-1, GATE-2, GATE-4,
  GATE-5, and GATE-6 remain open;
- WP-000 is recorded complete, but its `time-and-generation-api` evidence is
  impacted by the unresolved finite-clock ordering conflict;
- the admitted `WP-100-FOUNDATION-REFRESH` tranche is complete;
- `WP-100-HANDLER-VALUE-PRIMITIVES` is the nearest scoped implementation
  candidate, but its independent entry re-review is pending;
- broad handler entry is blocked by time-domain semantics, incomplete workload
  oracles, request/context migration review, and no-atomic public-boundary
  evidence;
- WP-200 is blocked before admission by the unresolved constructible
  candidate-fallback policy and bounded diagnostics;
- WP-300 is blocked before admission by incomplete exact binding contracts,
  host/constrained authoring fixtures, and unresolved subscription receiver
  ownership;
- WP-400, WP-500, and WP-600 remain downstream of WP-300, and WP-700 joins
  those three branches;
- default workspace tests and the valid feature matrix pass;
- the aggregate design-artifact check currently fails because
  `workspace/0007-time-domain-and-deadline.md` is referenced by the
  work-package index but is not registered by the artifact registry.

Global closure and scoped implementation are intentionally not a single serial
track. ADR-0013 permits an independently reviewed, dependency-complete tranche
to proceed when it is disjoint from open global findings. All global gates must
still close before final integration and release conformance.

## Owner Decision Queue

These decisions are not yet accepted project direction. For each item, AI
prepares a bounded decision package with alternatives, recommendation, affected
requirements and packages, evidence impact, and the exact repository changes.
The Owner approves, rejects, or requests revision.

| ID | Decision | Planning consequence | Required by |
|---|---|---|---|
| D1 | Accept, revise, or reject risk-proportional implementation admission from `workspace/0008-implementation-governance-overhead.md` | Changes implementation governance and possibly tranche authoring, but not architecture | M0 exit |
| D2 | Freeze the time domain and Deadline direction from `workspace/0007-time-domain-and-deadline.md` | Unblocks corrective foundation/Core work and broad handler entry | M2 exit |
| D3 | Select the completion strategy for decomposing residual `docs/design.md` ownership from `workspace/0010-complete-design-decomposition.md` | Determines the remaining v4.9 authority-closure work | M1 exit |
| D4 | Freeze subscription receiver/control ownership and clone semantics from `workspace/0011-subscription-receiver-ownership.md` | Unblocks affected WP-300/WP-400 contracts | M4 entry |
| D5 | Accept, revise, or reject the mock-binding property-read architecture gate from `workspace/0009-minimal-end-to-end-architecture-validation.md` | If accepted, PLAN and the authoritative work-package DAG must gain reviewed narrow tranches and an integration gate before broad expansion | Before broad WP-200/WP-300/WP-400 expansion |

D5 does not currently alter milestone dependencies. Until the Owner accepts it
and the decision is migrated, the registered work-package DAG remains the
execution authority.

## Milestone Overview

| ID | Milestone | Status | Dependency |
|---|---|---|---|
| M0 | Execution Baseline and Collaboration Reset | IN_PROGRESS | None |
| M1 | v4.9 Architecture and Authority Closure | IN_PROGRESS | M0 for closure |
| M2 | Foundation and Core Contract Stabilization | IN_PROGRESS | WP-000; scoped admission may run alongside M1 |
| M3 | Planning and Compilation Pipeline | OPEN | WP-100 |
| M4 | Protocol Binding SPI and Lifecycle | OPEN | WP-200 |
| M5A | Servient Runtime and Application Lifecycle | OPEN | WP-300 |
| M5B | Directory and Discovery Client Runtime | OPEN | WP-300 |
| M5C | Zenoh and zenoh-pico Binding Migration | OPEN | WP-300 |
| M6 | Umbrella Integration and Final Conformance | OPEN | WP-400, WP-500, WP-600 |
| M7 | v1 Release Review | OPEN | M6 |

M1 and admitted parts of M2 may progress in parallel. M5A, M5B, and M5C may
progress independently after their shared WP-300 dependency is complete.

## M0 — Execution Baseline and Collaboration Reset

Status: IN_PROGRESS

Objective: establish one trusted execution baseline and an explicit
Owner/AI review loop before further substantial implementation.

Scope:

- obtain Owner review of this plan, its release target, milestone ordering, and
  decision queue;
- initialize and maintain `PROJECT_STATE.md` as the continuation checkpoint;
- resolve the current artifact-registry/checker inconsistency;
- decide D1 and, if accepted, migrate the risk-proportional admission policy
  into its proper governance and authoring owners;
- remove or explicitly assign duplicated execution-planning responsibility in
  the root governance documents without changing accepted architecture;
- record the baseline verification commands that future milestones must
  preserve.

AI deliverable:

- a review-ready plan and state checkpoint;
- a focused D1 decision package;
- passing baseline governance checks or an explicit, Owner-visible blocker;
- exact evidence links and next safe action after every substantial change.

Owner checkpoint:

- approve or revise the plan;
- decide D1;
- confirm that the collaboration mechanism is sufficient for routine
  execution and milestone review.

Exit criteria:

- the Owner has approved the active plan;
- root artifact responsibilities no longer conflict or duplicate execution
  authority ambiguously;
- `PROJECT_STATE.md` is current and sufficient for a fresh session;
- the default workspace tests, valid feature matrix, and aggregate
  design-artifact check pass;
- the next implementation candidate and its admission state are unambiguous.

## M1 — v4.9 Architecture and Authority Closure

Status: IN_PROGRESS

Objective: turn the v4.9 architecture-closure candidate into one coherent,
single-owner, independently reviewed design revision.

Scope:

- close or supersede every remaining Architecture Review 03 finding;
- decide D2, D3, and D4 where their cross-domain consequences affect closure;
- complete the requirement-owned decomposition of residual
  `docs/design.md` contracts;
- reconcile accepted ADRs, architecture, domain specifications, API ownership,
  state machines, resources, performance contracts, requirements, and the
  work-package DAG;
- keep historical v4.8 material as migration input only;
- run all registered checks and obtain an independent same-revision closure
  review.

AI deliverable:

- decision packages with concrete alternatives and repository impact;
- migrated authoritative specifications after Owner decisions;
- updated checkers, registries, and review evidence;
- a closure evidence index with no unresolved conflict hidden by precedence.

Owner checkpoint:

- approve significant architecture and normative-ownership decisions;
- validate the independent closure review;
- confirm transition from REVIEW to CLOSED.

Exit criteria:

- every accepted ADR has one non-conflicting authoritative projection;
- every active detailed requirement has one registered normative owner;
- API, state, resource, performance, requirement, and work-package artifacts
  identify the same v4.9 revision;
- GATE-1 through GATE-6 are closed with same-revision evidence;
- an independent review finds no remaining architecture conflict;
- the Owner closes the milestone.

## M2 — Foundation and Core Contract Stabilization

Status: IN_PROGRESS

Authoritative package scope: WP-000 and WP-100.

Objective: complete the protocol-neutral foundation and Core contracts needed
by planning, bindings, and Servient without protocol-specific assumptions.

Completed evidence:

- WP-000 is recorded complete;
- `WP-100-FOUNDATION-REFRESH` is implemented and has completion evidence.

Next execution order:

1. repair the current artifact/check registration defect;
2. complete independent entry re-review of
   `WP-100-HANDLER-VALUE-PRIMITIVES`;
3. if approved, implement and verify exactly the five admitted passive values;
4. decide D2, define a corrective time-domain tranche, and replace or reaffirm
   impacted WP-000 time evidence;
5. define and admit the next bounded handler tranches for request/context
   values, portable traits, host erasure/storage, security, codecs, errors, and
   callback isolation;
6. complete the real handler matrix, no-atomic boundary, cancellation,
   storage/replacement, resource, and performance evidence;
7. retain Producer and Servient integration in WP-300 and WP-400.

AI deliverable:

- exact tranche admission material before code;
- implementation constrained to admitted paths;
- completion evidence and updated `PROJECT_STATE.md`;
- immediate escalation when an ambiguity changes semantics, ownership,
  lifecycle, resources, or evidence truth.

Owner checkpoint:

- approve D2 and any resulting architecture change;
- review any proposed expansion or reordering of WP-100 scope;
- validate WP-100 completion evidence.

Exit criteria:

- impacted WP-000 time evidence is replaced or reaffirmed under one frozen
  clock model;
- all WP-100 public contracts exist at their frozen owners and feature cells;
- handler callbacks execute outside engine locks and have explicit bounded
  cancellation and cleanup behavior;
- required Core workloads and feature/no-atomic fixtures pass;
- obsolete Core surfaces assigned to WP-100 are removed;
- WP-100 completion evidence is complete and independently reviewable;
- the Owner closes the milestone.

## M3 — Planning and Compilation Pipeline

Status: OPEN

Authoritative package scope: WP-200.

Dependency: WP-100.

Objective: produce immutable, bounded logical plans, binding plans, capability
indexes, and compiled-plan sets without runtime TD rescanning.

Entry conditions:

- WP-100 dependencies required by the proposed tranche are complete;
- the AR-004 candidate-fallback policy, health rule, pre-side-effect failure
  set, and bounded diagnostics are constructible and independently reviewed;
- D5 has been decided; if accepted, its narrow planner tranche and integration
  dependency are present in the authoritative DAG;
- the exact tranche is admitted under the active governance policy.

AI deliverable:

- deterministic planner/compiler implementation;
- admission rollback, bound, generation, and complexity evidence;
- no hidden binding execution or Servient lifecycle ownership in planning.

Owner checkpoint:

- approve any selection-policy change that alters project direction or public
  semantics;
- validate WP-200 completion.

Exit criteria:

- a TD produces an immutable admitted plan set through registered capability
  and compiler boundaries;
- selection, fallback, caching, lazy compilation, bounds, rollback, and
  generation isolation pass their evidence contracts;
- no interaction hot path rescans or re-defaults the TD;
- WP-200 is complete and independently reviewed.

## M4 — Protocol Binding SPI and Lifecycle

Status: OPEN

Authoritative package scope: WP-300.

Dependency: WP-200.

Objective: provide a constructible protocol-neutral client/server binding SPI
with route-scoped progress, explicit ownership, and bounded cleanup.

Entry conditions:

- WP-200 is complete;
- D4 is accepted and migrated into authoritative subscription contracts;
- exact complete-registration, compiler, route, cancellation, response,
  subscription, emission, and constrained-progress signatures are frozen;
- independent host and `no_std + alloc` binding-authoring fixtures pass;
- D5 has been reflected if accepted;
- the exact tranche is admitted.

AI deliverable:

- implementation and authoring fixtures that prove a binding need not know
  handler internals;
- lifecycle, memory, flow-control, response-validation, generation, and cleanup
  evidence;
- removal staging that does not create a migration cycle.

Owner checkpoint:

- approve subscription ownership and any SPI direction change;
- validate that the SPI is usable by realistic host and constrained bindings;
- validate WP-300 completion.

Exit criteria:

- one binding can be authored through the frozen SPI in host and constrained
  profiles;
- bindings own protocol syntax, I/O, correlation, and local flow control but
  cannot dispatch handlers or reinterpret TDs;
- activation, cancellation, subscription, response, emission, and cleanup
  state transitions are executable and bounded;
- WP-300 is complete and independently reviewed.

## M5A — Servient Runtime and Application Lifecycle

Status: OPEN

Authoritative package scope: WP-400.

Dependency: WP-300.

Objective: complete Servient-owned startup composition, activation,
orchestration, scheduling, application facades, and cleanup.

Exit criteria:

- one Servient instance owns plan-set lifetime and atomic serving activation;
- route acceptance cannot precede publication and deactivation prevents new
  acceptance;
- handler selection and progress have no binding-owned shortcut;
- host and constrained scheduling, fairness, cleanup, and resource-ledger
  evidence pass;
- application-facing producer/consumer/subscription facades have their frozen
  ownership and semantics;
- WP-400 is complete and independently reviewed.

Owner checkpoint: validate the usable application lifecycle and WP-400
completion.

## M5B — Directory and Discovery Client Runtime

Status: OPEN

Authoritative package scope: WP-500.

Dependency: WP-300.

Objective: converge the Directory and Discovery client runtime while
preserving the already reviewed client-only boundary.

Exit criteria:

- public client surfaces, lazy progress, incremental admission, terminal state,
  cancellation, overflow, and bounded result handling pass;
- no Directory service scope enters the active v1 target;
- WP-500 is complete and independently reviewed.

Owner checkpoint: validate Directory/Discovery scope and WP-500 completion.

## M5C — Zenoh and zenoh-pico Binding Migration

Status: OPEN

Authoritative package scope: WP-600.

Dependency: WP-300.

Objective: migrate Zenoh and zenoh-pico to the frozen planning and binding
contracts as the first production protocol bindings.

Exit criteria:

- Core and Planning remain protocol-neutral;
- Zenoh registrations, form compilation, routes, request/response correlation,
  subscriptions, emissions, activation, cleanup, and bounds pass;
- both valid backend feature cells pass without relying on their intentionally
  invalid simultaneous configuration;
- realistic end-to-end Thing interaction works through Zenoh;
- WP-600 is complete and independently reviewed.

Owner checkpoint: validate production-binding usability and WP-600 completion.

## M6 — Umbrella Integration and Final Conformance

Status: OPEN

Authoritative package scope: WP-700.

Dependencies: WP-400, WP-500, and WP-600.

Objective: compose the v4.9 implementation through the umbrella crate, remove
obsolete APIs, and close final cross-package evidence.

Exit criteria:

- umbrella public APIs expose only the intended v1 surface;
- all staged old APIs and hidden legacy execution paths are removed;
- workspace feature, `no_std + alloc`, TD compatibility, integration,
  architecture-boundary, resource, and performance checks pass;
- requirement-to-evidence coverage is complete;
- the accepted D5 integration gate passes if adopted;
- WP-700 and all global conformance evidence are complete;
- an independent release-candidate review is ready for the Owner.

Owner checkpoint: validate integration completeness and admit the release
candidate to M7.

## M7 — v1 Release Review

Status: OPEN

Dependency: M6.

Objective: make the final release decision from reproducible evidence rather
than milestone labels.

Scope:

- release documentation and examples;
- public API and compatibility review;
- reproducible build and verification commands;
- known limitation and deferred-scope review;
- release artifact and version readiness.

Exit criteria:

- all prior milestones are CLOSED;
- no open blocker intersects the v1 release scope;
- release checks and examples pass from a clean checkout;
- deferred items are explicit and do not contradict v1 claims;
- the Owner confirms release readiness and closes v1.

## Progress Update Rule

Milestone status changes only from repository evidence and follows
`PROJECT_GOVERNANCE.md`. AI prepares the evidence and updates this plan and
`PROJECT_STATE.md`; the Owner validates significant direction changes,
milestone completion, and release readiness.
