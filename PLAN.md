# ClinkZ-WoT Project Plan

## Plan Status

Plan revision: active v5 bounded-core authority

Active design revision: v5.0 bounded-core authority

Convergence target: v5.0 bounded-core authority

Release target: ClinkZ-WoT v1, a protocol-neutral W3C WoT runtime for
producing, consuming, and discovering Things, with a stable Servient
architecture, a client-only Directory/Discovery boundary, and Zenoh plus
zenoh-pico binding support.

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

- the v5.0 bounded-core authority is active; aggregate architecture closure
  remains in progress;
- ADR-0018 supersedes lossless D3 residual decomposition as the convergence
  direction. The classified v5.0 target retains 62 active requirements,
  defers 34 declared-v1 requirements to their domain entry, preserves 15 as
  design input, retires four premature freezes, and discharges six duplicate
  requirement ids through stronger owners. Exact candidate
  `b1916250a28ee133e8d0b12225c5b6311c975247` passed independent review at
  `6d483a598e654f5c7043efb887074aba3a605f7a` and activated atomically at
  `30b845a4b17dd3eb56670da48c939b72daea7d59`; it changed no runtime or public
  API;
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
- `WP-100-HANDLER-CONTEXT` is implemented and has completion evidence for the
  borrowed dispatch-identity view and exact compatibility matrix;
- `WP-100-PROPERTY-READ-HANDLER-SLICE` is implemented and has completion
  evidence for the synchronous static property-read handler seam; no
  architecture fixture root was created;
- broad handler entry remains blocked by incomplete workload oracles,
  the remaining request/target migration, portable-trait admission, and
  no-atomic public-boundary evidence;
- ADR-0017 closes the WP-200 candidate-fallback design gap with a constructible
  pre-execution-only policy and bounded typed diagnostics. D8 freezes the
  host-erased/static binding-artifact representation, sole WP-200
  implementation ownership, and immutable planning-input/no-runtime-TD-read
  boundary. Exact implementation
  `b889ae1dafa65ee66bfb331bebf9d537e1c29eee` completes the narrow
  `WP-200-PROPERTY-READ-PLAN-SLICE` after the independently reviewed v2
  admission chain. Its Core/Planning source, host and constrained author
  fixtures, bounded progress, owned output, predecessor regressions, aggregate
  design check, locked workspace tests, and 21-cell feature matrix pass; no
  cross-package architecture fixture root was created;
- the exact WP-300 Property Read slice has a complete planner dependency, a
  finite migrated scope, and paired host/static authoring contracts. Its
  semantic review was recorded at
  `d5169ba34ad846b2d45d0841b5d57210ee4df0c1` and integrated through pull
  request #1 at `a8eac3504f7e4252e9c3ac66da5e3038cb532cfc`. Its
  evidence-topology correction and independent correction review are
  integrated through pull request #13 merge
  `ff86c1582138d9a231dd44068b35cf6464214cd5`. Pull request #12 merge
  `384461ba33a3639a0f89978159719aa6937e1f36` is the validated admission
  base. Exact five-file checkpoint
  `6aaf1ef1586428152e535b9cd75f7183ec764cd0` and immediate exact two-path
  implementation `89fb9f17ac961294032123173b29692a719a174c` complete the narrow
  tranche. Its registered three-cell authoring, route/response/cleanup,
  predecessor, and legacy-separation checks pass. The aggregate Property Read
  gate remains blocked pending WP-400; D4 has resolved subscription receiver
  ownership;
- the narrow WP-400 Property Read Servient slice is released to prepare its
  own candidate and review. Broad WP-400, WP-500, and WP-600 remain downstream
  of broad WP-300, and WP-700 joins those branches;
- default workspace tests, the 21-cell valid feature matrix, and the aggregate
  design-artifact check pass with the WP-300 completion evidence present;
  exact-head remote validation remains required before integration.

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
| D3 | SUPERSEDED | The former lossless residual-decomposition direction remains historical decision input | Do not integrate Foundation candidate `2494f33fdfe49ec3c7ae850d20990e446e628865` or open another D3 domain migration; ADR-0018/D7 replace its activation path | Superseded by D7 |
| D4 | MIGRATED | Use one non-`Clone` linear `Subscription`/`StaticSubscription` receive capability; expose no cloneable receiver/control split, competing-consumer contract, or per-clone broadcast | WP-300 owns one binding driver/cursor; WP-400 owns the Servient record/facade and must provide negative `Clone` compile fixtures | M4 entry |
| D5 | MIGRATED | Adopt `PROPERTY-READ-ARCHITECTURE` as the first executable cross-package composition proof, using one property read in host and manual runtime cells plus an async/no-std compile projection | Four exact ordered tranches form `WP-100 -> WP-200 -> WP-300 -> WP-400`; the first three are complete and WP-400 remains pending its own candidate, review, admission, and implementation | Before broad WP-100/WP-300/WP-400 expansion |
| D6 | MIGRATED | Use `CandidateFallbackPolicy::PreExecution` by default; permit only side-effect-free security inapplicability and exact deterministic lazy-artifact negatives to skip candidates; prohibit binding-input, health, transient, security-commit, and post-acceptance fallback | ADR-0017 makes the policy constructible and bounds one fixed-width diagnostic per eligible skip; the WP-200 Property Read review reaffirmed it while fallback/lazy implementation remains outside the frozen narrow scope | M3 entry |
| D7 | MIGRATED | Adopt ADR-0018's bounded v5.0 authority reset: 62 active requirements, explicit inactive classifications for the other 59, and domain-entry re-adoption for later v1 obligations | Exact candidate `b1916250a28ee133e8d0b12225c5b6311c975247` was independently attested and integrated as the unchanged second parent of activation checkpoint `30b845a4b17dd3eb56670da48c939b72daea7d59`; v5.0 authority is active | M1 exit and WP-200 resume |
| D8 | MIGRATED | Use one associated-type portable compiler contract, an application-closed static compiler/cursor/artifact enum, and Core-owned safe host erasure; WP-200 solely implements the compiler/artifact components and WP-300 consumes them only inside a complete installable bundle | The exact nine-path WP-200 implementation and both public author profiles pass completion evidence; WP-300 may consume but must not duplicate this ownership | M3 entry |
| D9 | MIGRATED | Adopt bounded design-to-implementation conversion from `workspace/0016-post-reset-implementation-throughput.md` | Once D8's declared closure boundary is satisfied, refuse unrelated refinement and proceed through one exact review, one pre-source admission checkpoint, implementation, and completion evidence; track authority, local contracts, and vertical integration separately | Continuous execution |
| D10 | MIGRATED | Treat the WP-200 admission path as finite only after review exercises the complete next-state transition, not merely the candidate state | The reviewed v2 transition produced the exact five-file pre-source checkpoint, exact nine-path implementation, and registered completion evidence without another design loop | M3 entry |
| D11 | MIGRATED | Preserve the exact WP-100 -> WP-200 -> WP-300 -> WP-400 source dependency chain while allowing non-authoritative preparation for later slices | Later preparation may reduce uncertainty but cannot claim admission or vertical progress before its predecessor's completion event | Property Read gate |
| D12 | MIGRATED | Use one staged legacy-to-target authority map: Planning owns selection/artifacts, WP-300 owns execution SPI, WP-400 owns orchestration, WP-600 migrates concrete Zenoh paths, and WP-700 proves final removal | Old and new paths may coexist only at named one-way migration adapters; no generation may have two selection, dispatch, or activation authorities | M3-M6 |
| D13 | MIGRATED | Keep the directed owner/projection/evidence/checker model and require transition checks to bind immutable candidates and exercise their next state | Support-artifact failures block only when they falsify an owned technical or evidence claim; no additional WP-200 refinement cycle is admitted | Continuous execution |
| D14 | MIGRATED | Reaffirm D8 constructibility for admission from the paired external public-boundary fixtures, while reserving runtime ergonomics and production-author claims for implementation and WP-600 evidence | WP-200 completion compiles both profiles; real Zenoh and zenoh-pico authoring may reopen the API only on a concrete ownership, resource, or portability defect | M3 and M5C |
| D15 | MIGRATED | Define review effectiveness by independently falsifiable evidence classes, not session separation alone | The completed v2 review closed only the evidence-boundary transition; later completion reviews must add real compile, runtime, lifecycle, resource, workload, and integration evidence as applicable | Continuous execution |
| D16 | MIGRATED | Establish one remote mainline validation status covering diff hygiene, aggregate design evidence, locked workspace tests, and the supported feature matrix | Active repository Ruleset `20009352` targets the default branch and requires GitHub Actions context `validation`; the matching check passed on remote `master`. The classic branch-protection summary is not sufficient Ruleset evidence | Before remote source integration |
| D17 | MIGRATED | Retain the full v1 target and make its critical path and post-WP-300 branch join explicit | Directory/Discovery client work is mandatory client scope, the Directory service remains excluded, and WP-400/WP-500/WP-600 rejoin at WP-700 before release review | M7 exit |
| D18 | MIGRATED | Bound the WP-300 Property Read slice to one complete Producer Property Read registration, two readiness shapes, permit-gated acceptance, one response, cleanup, and paired host/static authors | The exact package-local slice is complete; subscription, emission, client, Servient, production protocol, workload, and broad cancellation behavior remain excluded | M4 entry |
| D19 | MIGRATED | Distinguish narrow and broad WP-300 release events instead of inferring progress from package status | Narrow WP-300 completion has released only the WP-400 Property Read slice; broad WP-300 completion still releases broad WP-400, WP-500, and WP-600 | M4-M5 entry |
| D20 | MIGRATED | Enforce source- and generation-segregated legacy coexistence with no target-artifact backflow into legacy selection | New Planning/Core/WP-300 code cannot call legacy selectors or dispatch; WP-600 removes concrete selector/execution calls and WP-700 proves final public selector and adapter absence | M3-M6 |
| D21 | MIGRATED | Make global-gate impact on scoped tranches an explicit requirement/artifact/state/resource/evidence mapping | Intersecting findings block or reopen, compatible changes receive named revalidation, and disjoint findings cannot trigger undifferentiated re-review; aggregate gates remain required for WP-700/release | Continuous execution |
| D22 | MIGRATED | Reuse the immutable-candidate, scoped-review, exact-transition, pre-source, and completion machinery for WP-300 | Add only WP-300-specific contract/lifecycle fixtures and checks; preparation ends at the passing candidate plus expected absent-source boundary and reviewed next-state simulation | M4 entry |
| D23 | MIGRATED | Permit native GitHub auto-merge only as a terminal integration action after exact-head, current-evidence, strict up-to-date validation, resolved-conversation, non-stacked, conflict-free, and Owner-boundary checks all pass | Keep automatic handoffs draft by default; use merge-commit mode with expected-head protection only after the remote ruleset prerequisites are freshly verified | Continuous remote integration |
| D24 | MIGRATED | Preserve the shortest Property Read feedback path and separate local slice, mock cross-package, real Zenoh smoke, and release-readiness claims | Do not add support-only refinement between reviewed tranches; make a real Zenoh Property Read smoke the first executable WP-600 tranche after broad WP-300 | M4-M5C |
| D25 | MIGRATED | Treat the narrow mock authors as constructibility evidence, then run one bounded external Zenoh authoring spike before broad WP-300 admission | Helpers may group/generate declarations but cannot hide ownership or resources; reopen the SPI only for a concrete ownership, portability, resource, unsafe, or implementability defect | Broad M4 entry |
| D26 | MIGRATED | Keep Servient semantic authority while sharding host storage and scheduling by Thing/generation and route/slot | Bounded queues/cursors and brief critical sections must isolate never-ready, hot, slow, draining, and cleanup-heavy owners; callbacks execute outside locks | M5A |
| D27 | MIGRATED | Treat every route represented by every advertised Producer form as required for one frozen generation | Any route failure blocks publication; omission requires a new effective TD and generation. Optional/redundant/late-join policy is deferred until versioned lifecycle/resource evidence exists | M5A |
| D28 | MIGRATED | Use one complete-object cleanup transition kernel with acknowledged transfer or unchanged manual return | Synchronous bindings use `NoCleanupSuccessor` without an executor; Servient owns executor/manual queues and durable fallback | M4-M5A |
| D29 | MIGRATED | Require host and constrained profiles to share semantic transitions, trace ids, outcomes, and resource deltas | Representation may differ only in storage, dispatch, wake, executor, and critical-section mechanics; compile-only cells make no runtime claim | M4-M6 |
| D30 | MIGRATED | Retain the exhaustive flat resource schema and generate checked named-profile and role builders as projections | Every applicable field remains explicit, typed `NA` is the only omission, and representation change requires measured evidence rather than field count | M2-M6 |
| D31 | MIGRATED | Define v1 binding “plugins” as build/install/deploy composition, not dynamic loading | Binding crates publish build metadata; the engine owns readiness/drain, while the platform owns trust/build/sign/cutover/rollback. External config still creates a new Servient generation | M5C-M7 |
| D32 | MIGRATED | Keep multiple forms as ordered candidates rather than runtime failover; make explicit retry a fresh application call | Retry combines `RetryClass` with caller idempotency/security policy, strict selection, and fresh deadline/work/security budgets; diagnostics explain non-attempted forms | M3-M6 |
| D33 | MIGRATED | Separate remote integration facts, repository technical truth, and curated continuation projections; reconcile them at session entry | Fix WP-300 by separating immutable `review_attestation_ref` from `admission_base_ref`, and include the registered work-package check in mainline validation | Continuous execution |
| D34 | MIGRATED | Verify legacy coexistence with staged no-backflow evidence rather than one final grep | WP-300 poisons legacy target exits, WP-400 records zero target calls, WP-600 removes concrete edges, and WP-700 proves public/source absence; only the one-way legacy-publication adapter is temporarily allowed | M4-M6 |
| D35 | MIGRATED | Treat narrow WP-300 as package-local constructibility, then advance through external authoring, mock composition, production-family execution, and release evidence without overstating protocol neutrality or stable third-party ergonomics | The completed narrow slice releases mock composition only; the Zenoh authoring spike still precedes broad WP-300 and concrete authoring defects may reopen the provisional Rust surface | Broad M4 through M6 |
| D36 | MIGRATED | Close broad Servient architecture with explicit private owners, dependency direction, scheduling domains, complete-object cross-shard handoff, and one shared semantic-kernel/trace owner | The narrow Property Read Servient slice remains unblocked; broad WP-400 must freeze the internal owner graph and run an early multi-owner/multi-shard feedback tranche before feature breadth accumulates | Broad M5A entry |
| D37 | MIGRATED | Preserve the complete-object cleanup kernel while treating operation-specific Rust containers as provisional and making reservation coexistence, progress ownership, observability, scheduling isolation, and v1 residual durability explicit | Reserve simultaneously live obligations rather than every named phase additively; v1 residual durability is bounded in-instance status and final shutdown reporting, not restart persistence | Broad M4-M5A |
| D38 | MIGRATED | Require one code-owned semantic kernel and machine-readable trace oracle per shared Host/constrained capability, comparing safety and normalized liveness while separating semantic resource units from physical profile costs | Capability applicability and Host-default expansion are explicit; compile-only async/no-std makes no runtime claim, and production-backed parity is required before constrained runtime maturity is claimed | M4-M6 |
| D39 | MIGRATED | Keep atomic publication distinct from the conservative v1 all-advertised-route policy, and keep explicit retry distinct from product failover | Applications/platforms build a new effective TD/generation after rollback; broad Consumer/Gateway availability claims require execution-certainty/action taxonomy, overall attempt bounds, and attempt correlation beyond `RetryClass` alone | M5A-M7 |
| D40 | MIGRATED | Keep the exhaustive flat resource schema as canonical authority, not as the stable external authoring surface | Before broad public resource/SPI maturity, add typed applicability and lifecycle/classification metadata, complete role builders, schema revision/digest identity, field-admission/retirement discipline, and evidence-backed default maturity | Broad M4-M6 |
| D41 | MIGRATED | Count remote integration only from actual base, fetched default-branch ancestry, expected content, and validation coverage; treat an impossible objective or false blocker as dangerous projection drift | `PROJECT_STATE.md` records its observed basis and next bounded task; `PLAN.md` retains roadmap/package state rather than transient PR workflow, and stacked/repair PRs do not release dependent work merely because `merged = true` | Continuous execution |

The former D3 Foundation candidate is the exact single child of
`56fea9813df80fe29527755fcb2ce91d43cc5086`, changes only its registered
21-path documentation/checker boundary, and changes no implementation or
historical evidence file. ADR-0018 abandons it as an activation candidate. Its
content remains v5.0 migration input and Git history; its 44/76/1 authority
transition must not be integrated.

D7 keeps the Property Read critical path explicit. The v5.0 reset is active,
and `workspace/0014-property-read-plan-artifact-boundary.md` is migrated. Its
associated-type compiler contract, closed static representation, Core-owned
host erasure, single WP-200 implementation ownership, and paired authoring
fixtures passed the immutable v1 semantic review and v2 transition review.
The resulting exact pre-source and implementation topology is now complete at
`b889ae1dafa65ee66bfb331bebf9d537e1c29eee`.

D9 made that conversion finite: the exact nine-path implementation passed its
completion checker, supported public author profiles, aggregate design check,
locked workspace tests, and 21-cell feature matrix. The completion record
claims only the narrow plan slice; fallback/lazy planning, broad WP-200
completion, binding execution, Servient lifecycle, and cross-package
integration remain downstream work.

D5 preserves package completion order but adds a cross-package integration
dependency in the registered work-package DAG. The WP-100 handler, WP-200
plan, and WP-300 binding slices are complete. The exact WP-300 topology is
default-branch admission base `384461ba33a3639a0f89978159719aa6937e1f36`
`-> 6aaf1ef1586428152e535b9cd75f7183ec764cd0`
`-> 89fb9f17ac961294032123173b29692a719a174c`. WP-400 remains
planned/blocked until its own exact candidate and review; narrow WP-300
completion satisfies only its source dependency. Broad
`WP-100-HANDLER-ENTRY`, `WP-300-BROAD-ENTRY`, and `WP-400-BROAD-ENTRY` remain
blocked until the gate passes. The gate exception does not claim final
`InteractionInput` storage, `AcceptHint` resource admission,
`AffordanceTarget` no-atomic evidence, async/step handler traits, complete
binding registration/storage, or execution.

## Milestone Overview

| ID | Milestone | Status | Dependency |
|---|---|---|---|
| M0 | Execution Baseline and Collaboration Reset | CLOSED | None |
| M1 | v5.0 Authority Reset and Architecture Closure | IN_PROGRESS | M0 for closure |
| M2 | Foundation and Core Contract Stabilization | IN_PROGRESS | WP-000; scoped admission may run alongside M1 |
| M3 | Planning and Compilation Pipeline | IN_PROGRESS | WP-100 |
| M4 | Protocol Binding SPI and Lifecycle | IN_PROGRESS | WP-200 |
| M5A | Servient Runtime and Application Lifecycle | OPEN | WP-300 |
| M5B | Directory and Discovery Client Runtime | OPEN | WP-300 |
| M5C | Zenoh and zenoh-pico Binding Migration | OPEN | WP-300 |
| M6 | Umbrella Integration and Final Conformance | OPEN | WP-400, WP-500, WP-600 |
| M7 | v1 Release Review | OPEN | M6 |

M1 and admitted parts of M2 may progress in parallel. M5A, M5B, and M5C may
progress independently after their shared WP-300 dependency is complete.

The dominant executable and release path is:

```text
WP-100 Property Read complete
  -> WP-200 Property Read complete
  -> WP-300 Property Read complete
  -> WP-400 Property Read
  -> PROPERTY-READ-ARCHITECTURE
  -> remaining WP-100/WP-200/WP-300 package completion
  -> {WP-400, WP-500, WP-600}
  -> WP-700 plus all global gates
  -> M7 technical release review
```

Preparation and disjoint M1/M2 closure may proceed alongside this chain, but
they are not executable vertical progress. After WP-300, the Servient,
Directory/Discovery client, and concrete-binding branches are independently
validatable and must all rejoin at WP-700. A late global-gate finding
invalidates earlier evidence only through the normal explicit impact and
revalidation rules.

The exact release events inside that path are asymmetric:

- `WP-300-PROPERTY-READ-BINDING-SLICE` completion releases
  `WP-400-PROPERTY-READ-SERVIENT-SLICE`; and
- broad WP-300 completion releases broad WP-400, WP-500, and WP-600.

Package-level blocking therefore does not serialize the narrow Property Read
proof, while a narrow server slice does not pretend to stabilize the complete
client, subscription, emission, and concrete-binding contract.

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

## M1 — v5.0 Authority Reset and Architecture Closure

Status: IN_PROGRESS

Objective: replace the disproportional v4.9 residual-decomposition model with
one coherent, bounded, independently reviewed v5.0 authority revision while
preserving current safety, Property Read, and completed-evidence truth.

Scope:

- close or supersede every remaining Architecture Review 03 finding;
- apply migrated D2, D4, D5, D6, and D7 directions where their cross-domain
  consequences affect closure;
- activate the ADR-0018 62-requirement core and classify all 59 inactive v4.9
  identities without residual authority;
- retire ADR-0014/D3 migration machinery without destroying its Git history;
- reconcile accepted ADRs, architecture, domain specifications, API ownership,
  state machines, resources, performance contracts, requirements, and the
  work-package DAG;
- keep historical v4.8/v4.9 material and the abandoned Foundation candidate
  as migration input only;
- run all registered checks and obtain an independent same-revision closure
  review.

Current progress: v5.0 authority is active with 62 exact single owners, 59
inactive dispositions, explicit machine-artifact and completed-evidence
carry-forward manifests, active-only gate requirement sets, and an immutable
review/activation chain. Architecture closure remains open on the registered
global gates; the exact `WP-300-PROPERTY-READ-BINDING-SLICE` is complete and
the executable critical path now proceeds to an exact
`WP-400-PROPERTY-READ-SERVIENT-SLICE` candidate and independent admission.

AI deliverable:

- decision packages with concrete alternatives and repository impact;
- migrated authoritative specifications after AI decisions;
- updated checkers, registries, and review evidence;
- a closure evidence index with no unresolved conflict hidden by precedence.

Owner feedback focus (non-blocking):

- flag any project-goal conflict, omitted real-world constraint,
  unacceptable direction, or credible counterexample in the v5.0 closure
  evidence.

Exit criteria:

- every accepted ADR has one non-conflicting authoritative projection;
- exactly 62 active detailed requirements have one registered normative owner,
  and all other v4.9 identities have one checked inactive disposition;
- API, state, resource, performance, requirement, and work-package artifacts
  identify or explicitly carry forward into the same v5.0 revision;
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
- `WP-100-HANDLER-CONTEXT` is implemented and has completion evidence.
- `WP-100-PROPERTY-READ-HANDLER-SLICE` is implemented and has completion
  evidence.

Next execution order:

1. decompose the next dependency-complete portable-trait or remaining
   request/target tranche, keeping `AcceptHint` resource admission and
   `InteractionInput` downstream migration explicit;
2. complete the real handler matrix, no-atomic boundary, cancellation,
   storage/replacement, resource, and performance evidence;
3. retain Producer and Servient integration in WP-300 and WP-400.

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

Status: IN_PROGRESS; the exact Property Read plan slice is complete while
broad WP-200 package exits remain open.

Authoritative package scope: WP-200.

Dependency: WP-100.

Objective: produce immutable, bounded logical plans, binding plans, capability
indexes, and compiled-plan sets without runtime TD rescanning.

Entry conditions:

- WP-100 dependencies required by the proposed tranche are complete;
- ADR-0017's candidate-fallback policy, health rule, pre-side-effect failure
  set, and bounded diagnostics are constructible and reaffirmed by the exact
  tranche's independent review;
- the migrated host-erased and static binding-artifact/compiler representation
  has one implementation owner, exact Rust contracts, and paired independent
  authoring fixtures; and
- D5 is migrated; the exact `WP-200-PROPERTY-READ-PLAN-SLICE` and its
  dependency on the completed handler slice are present in the authoritative
  DAG;
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

Status: IN_PROGRESS; the exact Property Read binding slice is complete while
broad WP-300 package exits remain open.

Authoritative package scope: WP-300.

Dependency: WP-200.

Objective: provide a constructible protocol-neutral client/server binding
architecture with route-scoped progress, explicit ownership, and bounded
cleanup, while keeping empirical maturity claims scoped to the protocol and
profile evidence actually executed.

Entry conditions:

- broad package entry requires WP-200 to be complete;
- D4 is decided and migrated into authoritative subscription contracts;
- the broad `WP-300-BROAD-ENTRY` remains blocked until
  `PROPERTY-READ-ARCHITECTURE` passes; only
  `WP-300-PROPERTY-READ-BINDING-SLICE` may seek independent admission after
  its exact planner-slice dependency completes even while the remaining WP-200
  package is incomplete;
- for the narrow Property Read tranche, exact complete-registration, compiler,
  route, cancellation, response, cleanup-transfer, and constrained-progress
  signatures are frozen; subscription, emission, client, form, collection,
  Servient, and production-protocol behavior remain excluded;
- independent host and `no_std + alloc` binding-authoring contracts and the
  executable lifecycle schema are present in the immutable candidate;
- the exact tranche is admitted.

Current narrow progress: implementation
`89fb9f17ac961294032123173b29692a719a174c` provides the complete registration,
route, response, cleanup-transfer, typed static, and host-erased boundary in
the two registered Core paths. Its completion evidence releases only the
narrow WP-400 Servient slice; the broad entry requirements below remain open.

Before broad WP-300 admission, the external Zenoh authoring spike must record
authoring, cleanup, resource, diagnostic, generic/layout, and unsafe/private
dependency evidence; the resource authoring and capability-applicability gaps
in D38/D40 must have an exact owner and closure tranche. Narrow Property Read
completion remains a constructibility claim and is not blocked by those broad
entry requirements.

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
- shared Host/constrained capabilities use one semantic kernel and one
  machine-readable trace oracle, while profile-specific physical costs and
  applicability remain explicit;
- WP-300 is complete and independently reviewed.

## M5A — Servient Runtime and Application Lifecycle

Status: OPEN

Authoritative package scope: WP-400.

Dependency: WP-300.

Objective: complete Servient-owned startup composition, activation,
orchestration, scheduling, application facades, and cleanup.

Entry conditions:

- broad `WP-400-BROAD-ENTRY` remains blocked until
  `PROPERTY-READ-ARCHITECTURE` passes;
- only `WP-400-PROPERTY-READ-SERVIENT-SLICE` may seek independent admission
  after its exact binding-slice dependency completes; and
- before broad source admission, the private owner/dependency graph,
  scheduling domains, cross-shard transfer rules, shared semantic-kernel
  owner, and first multi-owner/multi-shard feedback tranche are frozen; and
- package completion order and the narrow WP-400 slice remain unchanged.

Exit criteria:

- one Servient instance owns plan-set lifetime and atomic serving activation;
- route acceptance cannot precede publication and deactivation prevents new
  acceptance;
- handler selection and progress have no binding-owned shortcut;
- host and constrained scheduling, fairness, cleanup, and resource-ledger
  evidence pass;
- publication and retry facades make the conservative v1 availability
  limitations explicit and do not claim degraded service or failover;
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
- production-backed parity covers the common Zenoh/zenoh-pico capability
  intersection while making profile-only and unsupported cells explicit;
- realistic end-to-end Thing interaction works through Zenoh;
- WP-600 is complete and independently reviewed.

Owner feedback focus (non-blocking): flag project-goal conflicts, omitted
constraints, unacceptable directions, or credible counterexamples in
production-binding usability or WP-600 completion evidence.

## M6 — Umbrella Integration and Final Conformance

Status: OPEN

Authoritative package scope: WP-700.

Dependencies: WP-400, WP-500, and WP-600.

Objective: compose the active v5.0 implementation through the umbrella crate, remove
obsolete APIs, and close final cross-package evidence.

Exit criteria:

- umbrella public APIs expose only the intended v1 surface;
- all staged old APIs and hidden legacy execution paths are removed;
- workspace feature, `no_std + alloc`, TD compatibility, integration,
  architecture-boundary, resource, and performance checks pass;
- requirement-to-evidence coverage is complete;
- protocol-neutrality, constrained-runtime maturity, resource-authoring, and
  availability claims are no broader than their executed protocol-shape,
  profile, authoring, and recovery evidence;
- `PROPERTY-READ-ARCHITECTURE` passes and its evidence remains current;
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
