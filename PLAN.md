# ClinkZ-WoT Project Plan

## Plan Status

Plan revision: AI-led governance and architecture-authority baseline

Active design revision: v4.9 architecture-closure candidate

Release target: ClinkZ-WoT v1, a protocol-neutral W3C WoT runtime with a
stable Servient architecture and Zenoh binding support.

This plan records roadmap, milestones, dependencies, status, and acceptance
objectives. It does not define architecture or implementation admission.
Authoritative behavior and package-level execution contracts remain in the
registered specifications and `docs/work-packages/index.toml`.

The plan is executed under AI-led development. AI owns technical decisions,
milestone technical status, evidence sufficiency, and migration of stable
conclusions. Owner feedback is visibility, constraint, question, and
counterexample input; it is not a routine technical approval gate.

Owner visibility is continuous and non-blocking. AI must keep decisions,
evidence, and next safe actions visible, but must not pause solely to obtain
routine Owner acknowledgment. Owner feedback changes execution only when it
introduces a project-goal conflict, omitted real-world constraint, unacceptable
direction, credible counterexample, or irreversible external commitment.

## Planning Baseline

The repository evidence establishes the following starting point:

- the v4.9 architecture backbone exists but is still a closure candidate;
- GATE-3, the Directory client boundary, is closed; GATE-1, GATE-2, GATE-4,
  GATE-5, and GATE-6 remain open;
- WP-000 is recorded complete; ADR-0016 preserves its historical
  `time-and-generation-api` record, while completed WP-100 logical-time
  evidence replaces its time claims and reaffirms the disjoint generation
  claims;
- the admitted `WP-100-FOUNDATION-REFRESH` tranche is complete;
- `WP-100-HANDLER-VALUE-PRIMITIVES` is implemented and has completion evidence
  for the exact five-value scope;
- `WP-100-LOGICAL-TIME-CORRECTION` is implemented and has completion evidence
  for the extended logical-time domain;
- `WP-100-DEADLINE-CLEANUP-TIMING` is implemented and has completion evidence
  for the Core Deadline, cleanup ordering, and incomparable-clock disposition;
- `WP-100-HANDLER-CONTEXT` has an exact Category B admission candidate for the
  borrowed dispatch-identity view and remains review-pending; its Core source
  implementation is not admitted;
- broad handler entry remains blocked by incomplete workload oracles,
  the remaining request/target migration, portable-trait admission, and
  no-atomic public-boundary evidence;
- WP-200 is blocked before admission by the unresolved constructible
  candidate-fallback policy and bounded diagnostics;
- WP-300 is blocked before admission by incomplete exact binding contracts,
  host/constrained authoring fixtures, and unresolved subscription receiver
  ownership;
- WP-400, WP-500, and WP-600 remain downstream of WP-300, and WP-700 joins
  those three branches;
- default workspace tests, the valid feature matrix, and the aggregate
  design-artifact check pass after registering the time-domain workspace topic
  as a non-normative artifact.

Global closure and scoped implementation are intentionally not a single serial
track. ADR-0013 permits an independently reviewed, dependency-complete tranche
to proceed when it is disjoint from open global findings. All global gates must
still close before final integration and release conformance.

## AI-led Open Decision Queue

These technical directions remain unresolved or not yet fully migrated. For each item, AI
investigates the workspace topic and repository evidence, chooses a technical
direction when evidence is sufficient, records alternatives and rejected
approaches, then migrates the stable conclusion to its authoritative owner.

Owner input may add project constraints, target conflicts, unacceptable
directions, or credible counterexamples. It does not preselect the technical
answer and does not automatically block unrelated admitted work.

| ID | Status | AI decision to resolve | Planning consequence | Required by |
|---|---|---|---|---|
| D1 | MIGRATED | Adopt risk-proportional implementation admission from `workspace/0008-implementation-governance-overhead.md` as an authoring/review-depth policy, without weakening ADR-0013 tranche admission | Category A work receives narrow evidence and review; Category B/C retain stricter controls | M0 exit |
| D2 | MIGRATED | Use clock-source-owned non-wrapping extended logical ticks; retain raw wrap metadata as diagnostics; fail incomparable clock domains explicitly; correct Foundation before Core Deadline/cleanup timing | Completed `WP-100-LOGICAL-TIME-CORRECTION -> WP-100-DEADLINE-CLEANUP-TIMING`; the time blocker is resolved while independent broad-handler blockers remain | M2 exit |
| D3 | MIGRATED | Complete residual `docs/design.md` ownership through the requirement-indexed target DAG in `docs/spec/decomposition.csv` and several independently reviewed atomic domain migrations | 34 requirements are at final targets, 84 remain residual, and three remain in registered amendments; the checker now exposes exact remaining ownership | M1 exit |
| D4 | MIGRATED | Use one non-`Clone` linear `Subscription`/`StaticSubscription` receive capability; expose no cloneable receiver/control split, competing-consumer contract, or per-clone broadcast | WP-300 owns one binding driver/cursor; WP-400 owns the Servient record/facade and must provide negative `Clone` compile fixtures | M4 entry |
| D5 | OPEN | Decide whether and how to add the mock-binding property-read architecture gate from `workspace/0009-minimal-end-to-end-architecture-validation.md` | If adopted after AI evidence review, PLAN and the authoritative work-package DAG must gain reviewed narrow tranches and an integration gate before broad expansion | Before broad WP-200/WP-300/WP-400 expansion |

D5 does not currently alter milestone dependencies. Until AI decides it and
the decision is migrated, the registered work-package DAG remains the execution
authority.

## Milestone Overview

| ID | Milestone | Status | Dependency |
|---|---|---|---|
| M0 | Execution Baseline and Collaboration Reset | CLOSED | None |
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

Status: CLOSED

Objective: establish one trusted execution baseline and an explicit AI-led
collaboration loop before further substantial implementation.

Scope:

- publish this plan, its release target, milestone ordering, and AI-owned open
  decision queue for Owner visibility and feedback;
- initialize and maintain `PROJECT_STATE.md` as the continuation checkpoint;
- resolve the current artifact-registry/checker inconsistency;
- keep D1 migrated as the risk-proportional admission authoring policy in
  `PROJECT_GOVERNANCE.md`, `AGENTS.md`, and the workspace lifecycle records;
- keep root artifact responsibilities distinct, with
  `ARCHITECTURE_GOVERNANCE.md` limited to architecture authority,
  convergence, and design-change control;
- record the baseline verification commands that future milestones must
  preserve.

AI deliverable:

- a review-ready plan and state checkpoint;
- a migrated D1 decision record;
- passing baseline governance checks or an explicit visible blocker;
- exact evidence links and next safe action after every substantial change.

Owner feedback focus (non-blocking):

- flag any project-goal conflict, missing constraint, unacceptable direction,
  or credible counterexample in the plan or collaboration model.

Exit criteria:

- `AGENTS.md`, `PROJECT_GOVERNANCE.md`, `PLAN.md`, and `PROJECT_STATE.md`
  consistently describe the AI-led collaboration model;
- root artifact responsibilities no longer conflict or duplicate execution
  authority ambiguously;
- `PROJECT_STATE.md` is current and sufficient for a fresh session;
- the default workspace tests, valid feature matrix, and aggregate
  design-artifact check pass;
- D1 is decided and migrated;
- the next implementation candidate and its admission state are unambiguous.

## M1 — v4.9 Architecture and Authority Closure

Status: IN_PROGRESS

Objective: turn the v4.9 architecture-closure candidate into one coherent,
single-owner, independently reviewed design revision.

Scope:

- close or supersede every remaining Architecture Review 03 finding;
- apply the migrated D2, D3, and D4 directions where their cross-domain
  consequences affect closure;
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
- migrated authoritative specifications after AI decisions;
- updated checkers, registries, and review evidence;
- a closure evidence index with no unresolved conflict hidden by precedence.

Owner feedback focus (non-blocking):

- flag any project-goal conflict, omitted real-world constraint,
  unacceptable direction, or credible counterexample in the v4.9 closure
  evidence.

Exit criteria:

- every accepted ADR has one non-conflicting authoritative projection;
- every active detailed requirement has one registered normative owner;
- API, state, resource, performance, requirement, and work-package artifacts
  identify the same v4.9 revision;
- GATE-1 through GATE-6 are closed with same-revision evidence;
- an independent review finds no remaining architecture conflict;
- AI closes the milestone from registered evidence.

## M2 — Foundation and Core Contract Stabilization

Status: IN_PROGRESS

Authoritative package scope: WP-000 and WP-100.

Objective: complete the protocol-neutral foundation and Core contracts needed
by planning, bindings, and Servient without protocol-specific assumptions.

Completed evidence:

- WP-000 is recorded complete;
- `WP-100-FOUNDATION-REFRESH` is implemented and has completion evidence.
- `WP-100-HANDLER-VALUE-PRIMITIVES` is implemented and has completion
  evidence.
- `WP-100-LOGICAL-TIME-CORRECTION` is implemented and has completion evidence.
- `WP-100-DEADLINE-CLEANUP-TIMING` is implemented and has completion evidence.

Next execution order:

1. independently review and, if the exact candidate passes, admit and complete
   `WP-100-HANDLER-CONTEXT`;
2. decompose the next dependency-complete portable-trait or remaining
   request/target tranche, keeping `AcceptHint` resource admission and
   `InteractionInput` downstream migration explicit;
3. complete the real handler matrix, no-atomic boundary, cancellation,
   storage/replacement, resource, and performance evidence;
4. retain Producer and Servient integration in WP-300 and WP-400.

AI deliverable:

- exact tranche admission material before code;
- implementation constrained to admitted paths;
- completion evidence and updated `PROJECT_STATE.md`;
- immediate Owner clarification only when an ambiguity depends on project
  goals, product trade-offs, real-world constraints, unacceptable directions,
  or irreversible external commitments.

Owner feedback focus (non-blocking):

- flag project-goal conflicts, omitted constraints, unacceptable directions, or
  credible counterexamples in D2, WP-100 scope, or completion evidence.

Exit criteria:

- impacted WP-000 time evidence is replaced or reaffirmed under one frozen
  clock model;
- all WP-100 public contracts exist at their frozen owners and feature cells;
- handler callbacks execute outside engine locks and have explicit bounded
  cancellation and cleanup behavior;
- required Core workloads and feature/no-atomic fixtures pass;
- obsolete Core surfaces assigned to WP-100 are removed;
- WP-100 completion evidence is complete and independently reviewable;
- AI closes the milestone from registered evidence.

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
- D5 has been decided; if adopted by AI evidence review, its narrow planner
  tranche and integration dependency are present in the authoritative DAG;
- the exact tranche is admitted under the active governance policy.

AI deliverable:

- deterministic planner/compiler implementation;
- admission rollback, bound, generation, and complexity evidence;
- no hidden binding execution or Servient lifecycle ownership in planning.

Owner feedback focus (non-blocking):

- flag project-goal conflicts, omitted constraints, unacceptable directions, or
  credible counterexamples in selection policy, public semantics, or WP-200
  completion evidence.

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
- D4 is decided and migrated into authoritative subscription contracts;
- exact complete-registration, compiler, route, cancellation, response,
  subscription, emission, and constrained-progress signatures are frozen;
- independent host and `no_std + alloc` binding-authoring fixtures pass;
- D5 has been reflected if adopted;
- the exact tranche is admitted.

AI deliverable:

- implementation and authoring fixtures that prove a binding need not know
  handler internals;
- lifecycle, memory, flow-control, response-validation, generation, and cleanup
  evidence;
- removal staging that does not create a migration cycle.

Owner feedback focus (non-blocking):

- flag project-goal conflicts, omitted constraints, unacceptable directions, or
  credible counterexamples in subscription ownership, SPI usability, or WP-300
  completion evidence.

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

Owner feedback focus (non-blocking): flag project-goal conflicts, omitted
constraints, unacceptable directions, or credible counterexamples in the
application lifecycle or WP-400 completion evidence.

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

Owner feedback focus (non-blocking): flag project-goal conflicts, omitted
constraints, unacceptable directions, or credible counterexamples in
Directory/Discovery scope or WP-500 completion evidence.

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

Owner feedback focus (non-blocking): flag project-goal conflicts, omitted
constraints, unacceptable directions, or credible counterexamples in
production-binding usability or WP-600 completion evidence.

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
- the D5 integration gate passes if adopted;
- WP-700 and all global conformance evidence are complete;
- an independent release-candidate review is complete and ready for release
  readiness assessment.

Owner feedback focus (non-blocking): flag project-goal conflicts, omitted
constraints, unacceptable directions, or credible counterexamples in umbrella
integration or release-candidate evidence.

## M7 — v1 Release Review

Status: OPEN

Dependency: M6.

Objective: determine technical v1 release readiness from reproducible evidence
and separate that evidence judgment from the Owner's actual public release
decision.

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
- AI determines technical release readiness from registered evidence;
- the Owner decides whether and when to execute the actual public release.

## Progress Update Rule

Milestone status changes only from repository evidence and follows
`PROJECT_GOVERNANCE.md`. AI prepares the evidence, makes technical milestone
and release-readiness judgments, and updates this plan and `PROJECT_STATE.md`.
Owner feedback can reopen a milestone or decision when it identifies a
project-goal conflict, omitted constraint, unacceptable direction, or credible
counterexample. AI does not pause for routine acknowledgment. The Owner
decides actual public release execution.
