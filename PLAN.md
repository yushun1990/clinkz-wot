# ClinkZ-WoT Project Plan

## Plan Status

Plan revision: active v5 bounded-core roadmap

Active design revision: v5.1 Consumer one-shot authority

Release target: ClinkZ-WoT v1, with protocol-independent Core, Planning, and
Servient ownership; a client-only Directory/Discovery boundary; and a production
Host HTTP/1.1 binding with HTTPS/security qualification. Concrete bindings remain
optional Cargo dependencies. Zenoh and zenoh-pico are deferred future bindings;
their SDK repair, parity and hardware qualification are not v1 dependencies.
Portable Core/Planning/Servient support remains required. Embedded production
and empirical protocol-shape neutrality are claimed only from their own executed
evidence, separately from Host HTTP operation.

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

The v5.1 Consumer one-shot authority is active. The domain-entry review is
complete, and the first WP-100 Consumer source tranche is complete. The borrowed
TD tranche is complete within its admitted ADR-0013 boundary; the broader WP-100
package remains open. [Its admission record](docs/work-packages/WP-100-consumer-validated-thing-admission.md)
owns the exact implementation and evidence boundary. ADR-0021 replaces the mandatory
normalized Snapshot with borrowed immutable typed Thing, bounded complete TD
validation/semantic lending, owned Planning output and Servient transaction
publication. Strict bounded JSON ingestion is a separate future capability,
with its literal-value and field-decoding authority preserved. It is no longer
a prerequisite of the first Consumer Property Read integration.
The Consumer Property Read architecture gate has passed independent acceptance.
All six
narrow Producer Property Read tranches and D48's
generic transition-validation convergence are integrated. The aggregate
`PROPERTY-READ-ARCHITECTURE` implementation and registered external mock
binding/runner evidence are assembled in both application-static and public
Host-erased representations; the gate has passed independent acceptance.

The bounded real-target Zenoh Property Read probe required by workspace topic
0056 now exercises the application-static and public Host-erased target
Planning, Binding, and Servient paths through actual protocol I/O and network
round trips. It reaffirmed the frozen macro ownership boundary and the typed
static route-state carrier. The Host counterexample recorded by workspace topic
0058 is corrected by one nonreplaceable carrier whose original input,
footprint, generation identity, and erased route state survive the three
linear stage owners through terminal cleanup; its public state projection is
shared and pinned rather than whole-state mutable, and accept polling receives
only a shared guard borrow while Servient retains the committed owner.
The accepted aggregate proof preserves that correction and supplies the first
executable cross-package composition proof. The real-target probe remains
architecture feedback, not WP-600 product progress or protocol-shape-neutrality
evidence.

The passed Producer and Consumer proofs do not release undifferentiated broad
expansion. [ADR-0022](docs/ADRs/0022-first-production-binding-and-runtime-environments.org)
selects real Host HTTP Consumer evidence as the production frontier, preserving
the Zenoh findings while correcting universal SDK allocation-recovery assumptions.
A separate minimal `ObserveProperty` proof remains required before broad
subscription/emission expansion. The existing WP-400 multi-owner/multi-route
checkpoint is package evidence and is not another global architecture gate.

The admitted `WP-600-HTTP-CONSUMER-PROPERTY-READ` tranche is complete at its
independently accepted exact head. The WP-600 package remains planned in the
work-package index. The source-free HTTP Producer route and public
Consumer-to-Producer loop are implemented within their admitted single-route
scope; independent tranche completion acceptance, followed by separate
HTTPS/security evidence, remains on the critical path. Embedded qualification
and v1 release readiness remain separate.

Before extending the public Binding/Servient SPI into a general multi-route,
multi-operation or long-lived execution surface, derive and independently
challenge one coherent architecture from the enduring WoT goals, normative
operation semantics, and contrasting native communication models. Existing
single-route APIs, successful tests, architecture fixtures, and accepted narrow
tranches are evidence and migration inputs, not a design template or reasons to
preserve an inadequate abstraction. Separate logical operation/authority,
compiled definition/checked instance, protocol-native matching/infrastructure,
and delivery/lifecycle responsibilities without presupposing one universal trait,
runtime manager, storage representation, or physical-resource cardinality.
The accepted ADR-0024 dependency direction is a foundation, not completed
general-SPI acceptance; revise its remaining assumptions deliberately if needed.
This architecture-first dependency adds no new gate and does not block separately
qualified HTTP HTTPS/security work.

The roadmap identifies durable ordering, not a stored executable next action.
A task session must still inspect the current repository and decide how to
advance the frontier safely.

## Critical Path

1. The admitted [ADR-0023 resource-ownership migration](docs/ADRs/0023-resource-ownership-and-policy-boundaries.org)
   for the narrow Consumer path and the first Host HTTP Consumer Property Read
   tranche are complete, with engine admission, cleanup funding and the
   independent production-completion evidence preserved. Concrete HTTP SDKs
   have been selected and qualified for this Consumer scope. The existing
   WP-400 multi-owner/scheduler checkpoint may proceed in parallel.
2. ADR-0024's dependency cut and the admitted HTTP Producer route/permit/response
   path and public bidirectional loop are implemented. Reconcile their exact
   independently accepted tranche completion without reopening narrow evidence.
   HTTPS/trust/security is a separate v1 release obligation and may progress in
   parallel with general SPI design; do not freeze a one-route physical model
   as the general interface to deliver HTTPS.
3. Before broad public Binding/Servient SPI expansion, design the complete
   protocol-independent interaction and ownership boundary from WoT semantics
   and meaningfully different native communication shapes: request/reply,
   unsolicited input, subscriptions/streams, native matching and collection
   delivery. Judge alternative responsibilities and executable data flows first;
   challenge inherited assumptions, permit pre-v1 breaking replacement, and
   reconcile accepted architecture only after a defensible design decision.
   Do not use mock proofs, gate construction or prototype convenience as a
   substitute for this design. Reuse the relevant existing architecture topics
   and Review 08 rather than creating parallel governance artifacts.
4. Following the design decision, implement a coherent production vertical
   integration that exercises multi-route/multi-instance semantics and at least
   one materially contrasting native interaction model. Use focused prototypes
   only to resolve concrete design feasibility doubts; then verify actual
   implemented matching, logical authority, independent cancellation, shared
   physical retirement, and resource bounds. Do not widen the general SPI by
   layering compatibility facades over the single-route HTTP adapter.
5. Broaden Core, Planning, Binding, and Servient capabilities in useful admitted
   slices. Resolve the long-lived domain entry and minimal `ObserveProperty`
   boundary before broad subscription/emission expansion. Reusable compiled
   definitions and checked instance binding require real ownership/compatibility
   design; automatic caches and persistence are not prerequisites.
6. Expand remaining capability families and Directory/Discovery with staged
   legacy removal. WP-500 retains its WP-100 dependency and WP-600 its WP-300
   dependency. Optional Zenoh/pico production and real embedded qualification
   remain separate from Host HTTP delivery, without shrinking product goals.
7. Join completed WP-400, WP-500, and WP-600 through WP-700 for final
   aggregation, absence, feature/conformance, and release evidence, then perform
   the v1 technical release review.

## Architecture Evolution Checkpoints

These checkpoints refine the critical path without adding admission gates,
independent implementation tracks, or a second source of task/PR status.
They are triggered by concrete integration or deployment evidence, not by
calendar deadlines. Existing architecture authority and ADR-0013 change control
still govern any ensuing source or contract changes. Reuse the evidence and
open questions in [Review 07](docs/reviews/review-07-product-runtime-architecture.md)
rather than restarting the same general review.

- **After the admitted Consumer resource-ownership implementation completes:**
  audit the existing architecture fixtures for independent falsification value,
  direct production-contract coverage, and redundant parallel implementations.
  Prefer a bounded review and focused retirement/consolidation only when proof
  obligations remain covered. Preserve unique negative probes and regressions;
  raw fixture LOC is not a sufficient reason for deletion. This audit may
  proceed alongside HTTP admission and is not an extra HTTP predecessor unless
  it finds a concrete intersecting defect.
- **After a usable target Host HTTP Consumer Property Read path exists:**
  examine the legacy/public facade versus the target call path, map old public
  capabilities to tested replacements, and stage removal of superseded
  selection, binding, and lifecycle paths. Do not claim migrated capabilities
  based on test-local adapters or delete unmatched behavior without an explicit
  disposition. Avoid a simultaneous workspace-wide crate rewrite.
- **Before implementing a second one-shot operation family:** challenge the
  risk of duplicating Property Read's carrier, generation, lease, settlement,
  cancellation, and cleanup mechanisms. Use contrasting operation semantics
  and actual production evidence to identify reusable lifecycle invariants
  versus operation-specific effects. Preserve the distinct Host and embedded
  storage/progress representations; do not preselect a universal OneShot
  runtime based on one operation slice alone.
- **When a concrete embedded firmware/device target is selected:** qualify
  actual stack/SRAM/flash, atomics, allocator and protocol-backend behavior
  against its application workload. Investigate a compact projection from the
  single authoritative resource schema instead of mandating all policy fields
  in every fixed firmware representation. Portable/no-std compilation is not
  hardware-fit evidence, and this qualification is not a prerequisite for
  the Host HTTP path.

## Milestone Overview

| Milestone | Status | Depends on |
|---|---|---|
| M0 — Execution Baseline and Collaboration Reset | CLOSED | — |
| M1 — v5.0 Authority Reset and Architecture Closure | IN_PROGRESS | M0 |
| M2 — Foundation and Core Contract Stabilization | IN_PROGRESS | M1 authority |
| M3 — Planning and Compilation Pipeline | IN_PROGRESS | M2 tranche dependencies |
| M4 — Protocol Binding SPI and Lifecycle | IN_PROGRESS | M3 tranche dependencies |
| M5A — Servient Runtime and Application Lifecycle | IN_PROGRESS | M4 tranche dependencies |
| M5B — Directory and Discovery Client Runtime | OPEN | M2 package dependency; exact Core/TD tranche entry |
| M5C — Host HTTP Production Binding | OPEN | M4 package dependency; matching capability tranche entry |
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
- the 65 active v5.1 requirements retain one registered owner and all inactive
  identities retain one checked disposition;
- every architecture/refactor gate required by the active v5 roadmap has
  current same-revision evidence and its applicable acceptance state; and
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
route-reservation projections and the Consumer Property Read aggregate/output
ownership are complete within their admitted boundaries. The narrow Consumer
Servient integration and aggregate gate are accepted. Broad selection, fallback, caching, lazy
compilation, bounds, and generation evidence remains.

Exit goals: deterministic admitted plans; complete ownership, rollback,
complexity, and generation evidence; no runtime TD reinterpretation; and
independently reviewed WP-200 completion.

## M4 — Protocol Binding SPI and Lifecycle

Authoritative package scope: WP-300. Dependency: WP-200.

Objective: provide a constructible protocol-independent binding architecture
with protocol-owned I/O and correlation, route-scoped progress, explicit
ownership, and bounded cleanup.

Current status: the narrow Property Read binding implementation candidate and
its executable evidence are assembled. The real-target Zenoh probe has
reaffirmed its macro ownership invariants and both
the application-static and corrected Host-erased carriers through real network
I/O. Workspace topic 0058 is migrated: public Host succession moves one
unchanged carrier across the distinct prepared, active, and committed owners,
with borrowed typed state access and exact terminal release. The aggregate
Property Read architecture gate has passed independent acceptance; broad WP-300
exit remains open.

Exit goals: one coherent general SPI responsibility and composition design,
independently challenged against WoT semantics and contrasting protocol shapes;
externally exercised authoring and execution boundaries; explicit profile
applicability and resource costs; real lifecycle, failure, and cleanup evidence;
no binding-owned application dispatch; and independently reviewed WP-300
completion. Successful narrow-route proofs alone do not satisfy this goal.

## M5A — Servient Runtime and Application Lifecycle

Authoritative package scope: WP-400. Dependency: WP-300.

Objective: complete Servient-owned composition, activation, orchestration,
scheduling, application facades, and cleanup.

Current status: the narrow Property Read Servient implementation and aggregate
architecture evidence have passed independent acceptance. The existing early
multi-owner/multi-route scheduler checkpoint is the next broad WP-400 evidence
that may proceed in parallel with Consumer domain-entry preparation. Broad
runtime evidence remains.

Exit goals: atomic serving activation; independently authorized multi-route
logical dispatch over binding-owned native infrastructure; bounded
host/constrained scheduling and cleanup; explicit v1 availability limits; and
independently reviewed WP-400 completion. One typed Property Read root is not
the general Servient composition interface.

## M5B — Directory and Discovery Client Runtime

Authoritative package scope: WP-500. Package dependency: WP-100.

Objective: complete the Directory and Discovery client runtime while
preserving the reviewed client-only boundary.

Implementation entry is tranche-scoped under ADR-0013: a WP-500 tranche may
proceed only after its exact Core/TD predecessors and Directory domain-entry
authority are complete. Broad WP-300 completion is not a prerequisite because
Discovery does not consume Planning or Protocol Binding execution semantics.

Exit goals: bounded lazy progress, admission, cancellation, overflow, and
result handling; no Directory service scope; and independently reviewed
WP-500 completion.

## M5C — Host HTTP Production Binding

Authoritative package scope: WP-600. Package dependency: WP-300.

Objective: deliver a useful production Host HTTP/1.1/HTTPS binding through the
admitted Planning, Binding and Servient contracts. The first Consumer tranche
is followed by a production Producer loop and HTTPS/security qualification.

Exit goals: real selected-request and permit-gated Producer interaction;
bounded protocol input/state and truthful cancellation/cleanup; explicit
supported capabilities and deployment requirements; matching admitted
capability/workload evidence; and no legacy selection or dispatch. Zenoh probes
remain architecture feedback and deferred SDK evidence. Neither portable
compilation nor Host HTTP completion is embedded production or protocol parity.

## M6 — Umbrella Integration and Final Conformance

Authoritative package scope: WP-700. Dependencies: WP-400, WP-500, WP-600.

Objective: aggregate the completed v5 implementation, finish only remaining
umbrella/public-surface cleanup, and close final cross-package conformance. WP-700
must not be the first executable proof of Consumer, long-lived interaction,
collection, emission, or protocol production ownership.

Exit goals: intentional v1 public surface; final absence of target-to-legacy
backflow after staged capability migration; passing workspace, feature,
portable/constrained-core, compatibility, resource, workload, and performance evidence;
current requirement coverage; passed aggregate gates; and release claims no
broader than executed evidence. Deferred legacy Zenoh execution is excluded or
retired from the release surface; its replacement is not a release dependency.

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
