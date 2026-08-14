# ClinkZ-WoT Project Plan

## Plan Status

Plan revision: active v5 bounded-core roadmap

Active design revision: v5.0 bounded-core authority

Release target: ClinkZ-WoT v1, with protocol-independent Core, Planning, and
Servient ownership; a client-only Directory/Discovery boundary; and optional
Zenoh plus zenoh-pico bindings. Release material may claim empirical
protocol-shape neutrality only if a materially contrasting protocol fixture has
passed. Otherwise it must state the narrower evidence-backed claim:
protocol-independent engine ownership plus Zenoh-family operation.

This file owns only the durable roadmap: milestones, dependencies, coarse
status, objectives, and exit goals. It does not own the current engineering
task, detailed implementation plan, session state, design authority,
work-package admission, or evidence history.

Use:

- registered `docs/` and work-package manifests for technical authority,
  dependency, admission, and completion truth;
- source code and tests for implementation truth;
- Git, GitHub, audits, and CI for current remote state, history, review, and
  executable evidence; and
- `PROJECT_GOVERNANCE.md` for task-session and progression rules.

Current task state and the next action are derived from those sources in the
active task session; they are not stored in this roadmap.

## Roadmap Frontier

The v5.0 authority is active. All six narrow Property Read tranches and D48's
generic transition-validation convergence are integrated and validated. The
aggregate `PROPERTY-READ-ARCHITECTURE` gate is `ready`, not `passed`, and its
planned fixture roots remain absent.

The bounded real-target Zenoh Property Read probe required by workspace topic
0056 has exercised the application-static target Planning, Binding, and
Servient path through actual protocol I/O and a network round trip. It
reaffirmed the frozen macro ownership boundary and the typed static route-state
carrier, but falsified the exact Host prepared -> active -> committed erased
state succession surface. Workspace topic 0058 isolates that correction.
Aggregate mock source admission waits for the Host carrier correction and
external revalidation. The probe remains architecture feedback, not WP-600
product progress or protocol-shape-neutrality evidence.

The roadmap identifies durable ordering, not a stored executable next action.
A task session must still inspect the current repository and decide how to
advance the frontier safely.

## Critical Path

1. Correct and independently revalidate the falsified Host route-state
   succession surface while preserving the exercised static boundary.
2. Complete the aggregate mock `PROPERTY-READ-ARCHITECTURE` gate.
3. Complete the remaining broad WP-100, WP-200, WP-300, and WP-400 contracts;
   broad WP-300 releases WP-500 and WP-600 product work.
4. Join WP-400, WP-500, and WP-600 through WP-700 and close all global gates.
5. Perform the v1 technical release review; the Owner decides actual release.

## Milestone Overview

| Milestone | Status | Depends on |
|---|---|---|
| M0 — Execution Baseline and Collaboration Reset | CLOSED | — |
| M1 — v5.0 Authority Reset and Architecture Closure | IN_PROGRESS | M0 |
| M2 — Foundation and Core Contract Stabilization | IN_PROGRESS | M1 authority |
| M3 — Planning and Compilation Pipeline | IN_PROGRESS | M2 tranche dependencies |
| M4 — Protocol Binding SPI and Lifecycle | IN_PROGRESS | M3 tranche dependencies |
| M5A — Servient Runtime and Application Lifecycle | IN_PROGRESS | M4 tranche dependencies |
| M5B — Directory and Discovery Client Runtime | OPEN | broad M4 |
| M5C — Zenoh and zenoh-pico Binding Migration | OPEN | broad M4 |
| M6 — Umbrella Integration and Final Conformance | OPEN | M5A, M5B, M5C |
| M7 — v1 Release Review | OPEN | M6 |

## M0 — Execution Baseline and Collaboration Reset

Objective: maintain an AI-led process whose durable truth lives in the
repository authorities, implementation, Git/GitHub, and executable evidence,
without requiring a parallel current-state or conversational-memory system.

Exit goals:

- governance and artifact responsibilities are non-conflicting;
- task sessions reconstruct current state from repository and remote truth;
- model/profile choice is proportional to technical uncertainty rather than a
  mandatory role choreography; and
- default-branch validation and remote reconciliation remain reproducible.

## M1 — v5.0 Authority Reset and Architecture Closure

Objective: keep one coherent active v5.0 authority while closing registered
architecture gates from same-revision evidence.

Exit goals:

- every accepted ADR has one non-conflicting authoritative projection;
- the 62 active v5 requirements retain one registered owner and all inactive
  identities retain one checked disposition;
- GATE-1 through GATE-6 close with current evidence; and
- an independent same-revision review finds no unresolved architecture
  conflict.

## M2 — Foundation and Core Contract Stabilization

Authoritative package scope: WP-000 and WP-100.

Objective: complete protocol-independent Foundation and Core contracts for
planning, bindings, and Servient across host and constrained profiles.

Current status: WP-000, Foundation refresh, handler values, logical time,
deadline/cleanup timing, handler context, and the narrow Property Read handler
slice are complete. Broad handler, request/target, cancellation, resource,
workload, and no-atomic closure remains.

Exit goals: frozen public contracts at their registered owners; bounded
cancellation and cleanup; passing required workload and feature evidence; and
independently reviewable WP-100 completion.

## M3 — Planning and Compilation Pipeline

Authoritative package scope: WP-200. Dependency: WP-100.

Objective: produce immutable bounded plan sets and binding artifacts without
runtime Thing Description rescanning or protocol execution ownership.

Current status: the narrow Property Read plan, Producer-route, and
route-reservation projections are complete. Broad selection, fallback,
caching, lazy compilation, bounds, and generation evidence remains.

Exit goals: deterministic admitted plans; complete ownership, rollback,
complexity, and generation evidence; no runtime TD reinterpretation; and
independently reviewed WP-200 completion.

## M4 — Protocol Binding SPI and Lifecycle

Authoritative package scope: WP-300. Dependency: WP-200.

Objective: provide a constructible protocol-independent binding architecture
with protocol-owned I/O and correlation, route-scoped progress, explicit
ownership, and bounded cleanup.

Current status: the narrow Property Read binding slice is complete. The
real-target Zenoh probe has reaffirmed its macro ownership invariants and
application-static carrier through real network I/O. The exact Host erased
guard succession is reopened by workspace topic 0058 because the current
public constructors cannot naturally transfer one prepared protocol state
into the active and committed guards. The aggregate mock gate and broad WP-300
exit remain open.

Exit goals: externally exercised authoring and execution boundaries; explicit
profile applicability and resource costs; executable lifecycle, failure, and
cleanup evidence; no binding-owned dispatch; and independently reviewed
WP-300 completion.

## M5A — Servient Runtime and Application Lifecycle

Authoritative package scope: WP-400. Dependency: WP-300.

Objective: complete Servient-owned composition, activation, orchestration,
scheduling, application facades, and cleanup.

Current status: the narrow Property Read Servient slice is complete and
integrated. Aggregate and broad runtime evidence remains.

Exit goals: atomic serving activation; Servient-owned selection and progress;
bounded host/constrained scheduling and cleanup; explicit v1 availability
limits; and independently reviewed WP-400 completion.

## M5B — Directory and Discovery Client Runtime

Authoritative package scope: WP-500. Dependency: broad WP-300.

Objective: complete the Directory and Discovery client runtime while
preserving the reviewed client-only boundary.

Exit goals: bounded lazy progress, admission, cancellation, overflow, and
result handling; no Directory service scope; and independently reviewed
WP-500 completion.

## M5C — Zenoh and zenoh-pico Binding Migration

Authoritative package scope: WP-600. Dependency: broad WP-300.

Objective: migrate Zenoh and zenoh-pico to the admitted planning and binding
contracts as one production protocol family.

The earlier real-target Property Read probe supplies architecture feedback but
does not satisfy this milestone's source admission or production evidence.

Exit goals: real Zenoh-family plan, route, request/response, subscription,
emission, activation, cancellation, cleanup, and bounds evidence; valid host
and constrained feature cells; and realistic end-to-end interaction without
legacy selection or dispatch.

## M6 — Umbrella Integration and Final Conformance

Authoritative package scope: WP-700. Dependencies: WP-400, WP-500, WP-600.

Objective: compose the v5 implementation, remove obsolete paths, and close
cross-package conformance.

Exit goals: intentional v1 public surface; no target-to-legacy backflow;
passing workspace, feature, constrained, compatibility, resource, workload,
and performance evidence; current requirement coverage; passed aggregate
gates; and release claims no broader than executed evidence.

## M7 — v1 Release Review

Dependency: M6.

Objective: determine technical release readiness from clean, reproducible
evidence while leaving the actual public release decision to the Owner.

Exit goals: all prior milestones closed; no intersecting blocker; passing
release checks and examples; explicit limitations and deferred scope; an
independent release-candidate review; and an AI technical-readiness verdict.

## Progress Rule

Milestone status changes only from registered repository evidence under
`PROJECT_GOVERNANCE.md`. Detailed work progress belongs to its owning source,
work-package record, tests, audit/evidence, Git branch, and pull request. It is
summarized here only when a milestone or durable roadmap dependency changes.
