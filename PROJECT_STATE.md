# Project State

Last updated: 2026-07-28

Repository basis: completed property-read handler implementation
`830f47ebe044b953a3c0c3214345968f0fb5e571` plus the current completion
evidence and continuation-state update. Its exact non-implementation candidate
`a8af29b29e41efb537ae8d6971edf79c42255b65` is the single child of frozen base
`2d7087d100d4a0d72cabb77476175bf60b0a7925`; its independent attestation is
`5bd9687507c790583452e16526a8842fe7560f67`, its admission checkpoint is
`5fdd4566b4fddf007945a1f26fd2eeb0de74ac12`, and its progress checkpoint is
`76ac45aff4070cccf17b72d8a750e2d478c6a93e`. Validation-hygiene checkpoint
`aea1677a8a02e2e2aea184ba87c1424814e1a073` routes the slice fixture into a
root target subdirectory so targeted and aggregate checks are order-independent.
The completed HandlerContext candidate
`27c8dc974fd1b337de4671c81719f9fa32410c56` is the single child of frozen base
`c42abcbc339167183c8c4bf9bd3bf584540073b4`; its independent attestation is
`3007a88b93155d063ed0d08c69dc3520defebee5`, its admission checkpoint is
`8c5693da3a3c18c81dceef1ae8620b2502817ac4`, and its progress checkpoint is
`0357271f0971d10c79d6d757edc573d454fe4acd`. The D3 design-authority
completion strategy is migrated at
`78b3a45fa521cfe97d49d6ae3907760bcca7a041`; D4 subscription receiver/control
ownership is migrated at
`e07ba5f796c7613e28da08b34903a77c00b5a2d8`. D5 property-read architecture
gate migration is recorded at
`d4800bff41442768d62c6531e5adee52a39b2a74`. ADR-0017 and the current D6
migration close the AR-004 pre-execution candidate-fallback design gap at
`7f2869ff03ceb8659ba01e69ba97d9882e944df3`. The dependency-ready D3
Foundation authority candidate
`2494f33fdfe49ec3c7ae850d20990e446e628865` is the single child of frozen base
`56fea9813df80fe29527755fcb2ce91d43cc5086` on
`candidate/d3-foundation-authority`; its exact scope and withheld activation
are registered in
`docs/audits/D3-foundation-authority-candidate.toml`.

## Current Objective

Obtain an independent review of the exact D3 Foundation authority candidate
`2494f33fdfe49ec3c7ae850d20990e446e628865` without changing its 21-path
boundary, integrating it, or self-attesting it in the candidate-building
session. Active authority remains unchanged until that review passes and a
separate integration checkpoint activates the candidate.

The preceding WP-200 inspection found that a property-read plan candidate is
not yet constructible: planning leaves the host-erased/static binding-artifact
representation open, binding registration lacks exact compiler/component Rust
schemas, and WP-200/WP-300 text duplicates implementation ownership despite the
package dependency order. Keep planning/Core/TD implementation, target/no-
atomic, request/storage, async/step handler traits, host execution, a WP-200
compile-contract root, and both planned architecture fixture roots outside
this review.

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

D3 now fixes the authority-completion model:

- `docs/requirements.csv::source_path` is the current normative owner;
- `docs/spec/decomposition.csv::target_path` is the final owner and its
  `depends_on` field is the target-domain migration DAG;
- every stable requirement occurs exactly once in each index;
- a target mapping never activates a planned or partial file;
- `docs/design.md` ultimately retains only revision, language, authority,
  change control, standards, source-manifest, and revision-record duties; and
- completion proceeds through several independently reviewed atomic
  target-domain migrations, with security and codecs separated because their
  ownership, side effects, resources, and evidence differ.

D4 fixes subscription application authority:

- one binding driver owns one protocol resource and receive cursor;
- Servient's private `SubscriptionRecord` owns installed-driver lifecycle;
- host `Subscription` and manual `StaticSubscription` are linear,
  non-`Clone` application capabilities;
- v1 exposes no separately cloneable receiver or per-subscription control
  handle;
- independent logical consumers require separately admitted
  `SubscriptionId`s; one facade provides neither competing-consumer
  distribution nor per-clone broadcast; and
- application-local fan-out after receipt is outside the subscription contract
  and owns its own bounds.

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

- D3 is migrated. The active checked completion index maps all 121 stable
  requirements across 14 final target domains: 34 are already at their final
  target, 84 remain residual in `docs/design.md`, and three remain under
  registered amendments. Exact Foundation candidate
  `2494f33fdfe49ec3c7ae850d20990e446e628865` is review-pending; its prospective
  44/76/1 map is not active authority;
- D4 is migrated into the canonical subscription flow, residual subscription
  contract, WP-400 evidence boundary, architecture checker, plan, and workspace
  decision record; it changes no runtime or public API;
- D5 is migrated into schema v3 of the work-package DAG. The first of its four
  exact property-read slices is complete; the remaining three are planned and
  blocked, and the integration gate still blocks broad handler, binding, and
  Servient entry without creating fixture placeholders;
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
- `WP-100-HANDLER-CONTEXT` is approved, implemented, and complete under
  `docs/evidence/WP-100-handler-context.toml`; its exact candidate is
  `27c8dc974fd1b337de4671c81719f9fa32410c56`, its independent attestation is
  `3007a88b93155d063ed0d08c69dc3520defebee5`, its admission checkpoint is
  `8c5693da3a3c18c81dceef1ae8620b2502817ac4`, its progress checkpoint is
  `0357271f0971d10c79d6d757edc573d454fe4acd`, and its implementation is
  `ae51858d82e8c1b9f621798674155015820c0535`;
- `WP-100-PROPERTY-READ-HANDLER-SLICE` is approved, implemented, and complete
  under `docs/evidence/WP-100-property-read-handler-slice.toml`; its exact
  candidate is `a8af29b29e41efb537ae8d6971edf79c42255b65`, its independent
  attestation is `5bd9687507c790583452e16526a8842fe7560f67`, its admission
  checkpoint is `5fdd4566b4fddf007945a1f26fd2eeb0de74ac12`, its progress
  checkpoint is `76ac45aff4070cccf17b72d8a750e2d478c6a93e`, and its
  implementation is `830f47ebe044b953a3c0c3214345968f0fb5e571`;
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

- None. Newly discovered technical questions enter `workspace/` and are
  resolved from repository evidence under the normal lifecycle.

D3 migration facts:

- the migration boundary is stable requirement identity, not Markdown headings
  or source modules;
- architecture module-boundary and execution-invariant requirements migrate
  first, followed by foundation, documents, interaction core, planning/codecs,
  security/binding/discovery, subscriptions, Servient, and cross-domain
  profiles/verification;
- existing planning and binding clauses remain active while their later
  reconciliation and remaining residual requirements follow the same DAG;
- current authority changes only when definitions, requirement source rows,
  artifact registration, prior detailed prose, amendments, affected
  projections, and evidence move atomically; and
- one monolithic move, placeholder target files, and overloading
  `docs/requirements.csv` with future ownership were rejected.

D4 migration facts:

- Core owns subscription values and SPI semantics, a binding owns protocol and
  cursor state, Servient owns the driver registry/lifecycle, and the
  application owns one linear facade or static slot token;
- passive metadata cloning conveys no receive, stop, or cleanup authority, and
  internal shared registry state creates no second public cursor;
- moving or externally serializing a `Send` facade still uses one cursor and
  defines no competing-consumer scheduling contract;
- a cloneable combined facade, a split cloneable control handle, per-clone
  broadcast, and competing-consumer semantics were rejected because they add
  observable teardown, retention, delivery, and fairness contracts without a
  demonstrated v1 requirement;
- current `core::Subscription` clone/share/merge behavior is nonconforming
  migration source already scheduled for WP-400 removal; and
- D4 does not close the remaining AR3 stop/cleanup signatures, constrained slot
  implementation, storage, performance, lifecycle evidence, or WP-300/WP-400
  admission.

D5 migration facts:

- `PROPERTY-READ-ARCHITECTURE` is a cross-package work-package gate rather than
  a product package, example, or replacement for package completion;
- its exact dependency chain is
  `WP-100-PROPERTY-READ-HANDLER-SLICE ->
  WP-200-PROPERTY-READ-PLAN-SLICE ->
  WP-300-PROPERTY-READ-BINDING-SLICE ->
  WP-400-PROPERTY-READ-SERVIENT-SLICE`;
- the WP-100 slice is complete with exact completion evidence; the remaining
  three slices remain `planned` and `blocked`. Each remaining slice needs its
  own independent ADR-0013 admission before implementation source or its
  planned architecture fixture root is created; candidate contract fixtures
  remain pre-implementation admission material;
- broad `WP-100-HANDLER-ENTRY`, `WP-300-BROAD-ENTRY`, and
  `WP-400-BROAD-ENTRY` depend on the gate, with only their named property-read
  slices exempt from broad entry;
- one semantic scenario runs in `no-default-manual` and `std-host`, while
  `async-no-std` is compile-only and selects no executor;
- only protocol frames, deterministic I/O state, and instrumentation probes may
  be fixture adapters. Plans, artifacts, permits/guards, accepted requests,
  handler boundaries, response opportunities, and cleanup owners must be
  production types;
- mandatory evidence proves immutable planning with no runtime TD read,
  publication-before-acceptance, Servient-only selection, one handler call and
  response opportunity, deactivation rejection, generation consistency, zero
  retained ownership, dependency direction, public construction, forbidden
  capabilities, and portable compilation; and
- package-local-only validation, Zenoh as the first architecture proof, one
  opaque cross-package tranche, placeholder crates, and ownership-boundary
  adapters were rejected;
- `workspace/0012-property-read-handler-slice-boundary.md` resolves a cycle in
  the first WP-100 slice: requiring the broad atomic-incapable Core proof before
  the only broad-entry exemption could proceed would depend on legacy `Arc`
  migrations that broad entry itself blocks;
- the corrected slice adds only `ReadPropertyHandler`, reuses
  `HandlerContext`, `StaticHandlerRegistration`, `InteractionInput`, and
  `InteractionOutput`, and permits implementation changes only in
  `core/src/handler.rs` and `core/src/lib.rs`;
- its `async-no-std` cell checks availability of that synchronous/static seam
  with the async feature enabled; it does not add async or step handler traits;
  and
- broad `AffordanceTarget` no-atomic evidence, `AcceptHint` admission, final
  `InteractionInput` storage migration, host erasure/storage/execution, and
  downstream ownership remain excluded and separately blocking.

D6 candidate-fallback facts:

- ADR-0017 makes `clinkz_wot_planning::CandidateFallbackPolicy` the immutable
  Consumer policy captured by `PlanBuildInput`; `PreExecution` is the default
  and applications may select `Disabled` for a consumed-handle generation;
- strict per-call form, binding, or security selection disables automatic
  fallback and cannot widen the immutable plan or override `Disabled`;
- only side-effect-free security inapplicability and the exact lazy-slot
  generation's deterministic cacheable compiler negative may skip a candidate;
- provider errors or budget exhaustion, cancellation, deadlines, backpressure,
  transient or non-cacheable compiler failure, stale/draining generations,
  security commit, request construction, `BindingInputRejection`, and every
  post-acceptance outcome terminate that interaction without reselection;
- mutable runtime health is diagnostic-only in v1 and cannot reorder or skip an
  admitted candidate;
- every eligible skip has one ordered, fixed-width, secret-free diagnostic,
  with full capacity reserved to the target plan's admitted candidate count
  before the first probe; and
- all candidates share one provider-probe allowance and one unique mutable
  `WorkBudget`; neither restarts after a skip.

Completed property-read handler slice facts:

- the candidate is the single child of frozen base
  `2d7087d100d4a0d72cabb77476175bf60b0a7925`; its registered ref is
  `a8af29b29e41efb537ae8d6971edf79c42255b65` and it changes exactly the 16
  registered non-implementation paths;
- the audit freezes one object-safe synchronous trait with no supertrait,
  associated item, default body, allocation, executor, runtime, or thread
  bound;
- the external contract composes both public trait paths with borrowed
  `HandlerContext`, `InteractionInput`, and `StaticHandlerRegistration`, uses
  a deliberately non-`Send`/non-`Sync` implementation, and records exactly one
  call;
- three feature cells prove the synchronous/static seam under no-default,
  async-enabled no-std, and std configurations, while negative fixtures reject
  unscoped async and step traits;
- the fixture does not inspect `InteractionInput` storage and therefore does
  not freeze the separately excluded final request-storage migration;
- the exact source validator rejects widened traits or unregistered handler
  traits and composes with the completed handler-value and HandlerContext
  projections; and
- the gate checker permits pending approval without inventing a future
  admission SHA, then requires an exact three-file admission commit, exact
  two-file progress checkpoint, exact two-file implementation commit, and
  registered completion evidence with verified ancestry; and
- neither Core implementation path nor either planned architecture fixture
  root is present in the candidate diff;
- a later independent root continuation rechecked the exact candidate,
  ownership, exclusions, all twelve prechecks, a conforming isolated
  implementation, and mutations adding `Send` or a Core-side implementation;
- the attestation, exact three-file approval, and exact two-file progress
  checkpoints precede the exact two-file Core implementation commit;
- the implementation adds only the object-safe synchronous trait and its Core
  root re-export, with no runtime owner, storage, allocation, thread bound,
  async/step trait, or downstream behavior; and
- the completion checker proves all three feature cells, two semantic tests,
  negative async/step scope, Core tests, downstream compilation, and both
  handler predecessor suites without creating either architecture fixture.

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

Completed handler-context tranche facts:

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
- a later independent root continuation review rechecked the exact candidate,
  authority, scope, dependency, ownership, fixtures, all eleven prechecks, and
  the known incompatible-pair mutation before attesting it;
- the attestation, exact three-file approval, and exact two-file progress
  checkpoints all precede the exact two-file Core implementation commit;
- the implementation is allocation-free, validates compatibility only, retains
  no lifecycle or resource ownership, and preserves optional binding identity
  in its fixed-size validation error; and
- the completion checker proves exact source shape, all three Core feature
  cells, the exhaustive compatibility/error matrix, private fields, negative
  traits, Core unit tests, downstream compilation, and handler-value regression.

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

- WP-200: one exact host-erased/static binding-artifact and compiler-extension
  Rust representation, single WP-200/WP-300 implementation owner, paired
  authoring fixtures, then the property-read logical-plan/build-input
  candidate, no-runtime-TD-read proof, and independent tranche review;
  ADR-0017 has closed the fallback/health/diagnostic design gap;
- WP-300: exact registration/compiler/constrained signatures, independent
  host/static authoring fixtures, and subscription receiver ownership;
- WP-400/WP-500/WP-600: their registered WP-300 predecessor;
- WP-700: WP-400, WP-500, and WP-600.

## Verification Baseline

Verified on 2026-07-26 through the handler-context implementation and
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

Handler-context admission and completion verification on 2026-07-26:

- `tools/check-wp100-handler-context-entry.sh --candidate` - passed after the
  exact candidate ref was registered;
- the candidate/base diff contains exactly the 17 registered
  non-implementation paths, and neither permitted Core implementation path
  changed;
- the design checker passed its 16 unit and four integration tests, while the
  completed handler-value checker remained green after its source projection
  was extended to permit only the separately validated `HandlerContext`;
- `tools/check-wp100-handler-context.sh` reached its intended pre-admission
  stop only because the Core `HandlerContext` implementation was absent;
- `tools/check-design-artifacts.sh` passed in registered-candidate state,
  validating governance and all six refactor gates; and
- isolated negative mutation verification proved that an unregistered invalid
  pairing could pass the superseded completion fixture;
- the replacement fixture passed against a conforming isolated implementation
  and rejected that mutation;
- `tools/check-wp100-handler-context-entry.sh --candidate` passed after
  replacement registration, including the exact candidate ancestry/path check
  and all eleven predecessor/pre-implementation checks;
- `tools/check-wp100-handler-context-entry.sh --admission-ready` passed before
  implementation;
- the independent review attestation, three-file approval checkpoint, and
  two-file progress checkpoint precede the exact two-file implementation
  commit;
- `tools/check-wp100-handler-context.sh` passed after implementation across all
  three Core feature cells, all 72 operation/target pairings, negative trait
  and private-field fixtures, the Core unit suite, downstream Servient and
  Protocol Binding compilation, and the handler-value predecessor suite;
- `cargo clippy --locked -p clinkz-wot-core --all-targets --all-features -- -A clippy::large_enum_variant -D warnings` -
  passed with only the established scoped allow for the pre-existing frozen
  `HandlerStep<R>` representation;
- `tools/check-design-artifacts.sh`,
  `cargo run --locked --quiet --manifest-path tools/design-check/Cargo.toml -- check-work-packages`,
  `cargo test --workspace --locked`, and the 21-cell valid feature matrix all
  passed in completion state;
- the implementation commit changes only `core/src/handler.rs` and
  `core/src/lib.rs`; and
- the superseded candidate remains failed historical evidence and was not
  attested or admitted.

D3 authority-decomposition verification on 2026-07-26:

- `docs/spec/decomposition.csv` expands to all 121 registered requirements
  exactly once across 14 unique target domains;
- its strictly increasing dependency phases make the target graph acyclic;
- `tools/check-design-requirements.sh` joins current and final ownership and
  reports 34 requirements at final targets, 84 residual in `docs/design.md`,
  and three in registered amendments; and
- isolated checker mutations were rejected for duplicate/missing requirement
  ownership, a backward dependency phase, and changing an active specification
  away from its current final target;
- `tools/check-design-artifacts.sh` passes with the new normative migration
  index, including all registered design checks and the design checker's 16
  unit and four integration tests; and
- `workspace/0010-complete-design-decomposition.md` is `MIGRATED`, with the
  stable conclusion projected into the design manifest, architecture and
  specification indexes, artifact registry, checker, plan, and continuation
  state.

D3 Foundation authority candidate verification on 2026-07-28:

- candidate `2494f33fdfe49ec3c7ae850d20990e446e628865` is the single child of
  frozen base `56fea9813df80fe29527755fcb2ce91d43cc5086` on durable local branch
  `candidate/d3-foundation-authority`;
- it changes exactly the 21 documentation and checker paths registered in
  `docs/audits/D3-foundation-authority-candidate.toml`, with no source, Cargo,
  compile-fixture, resource-schema, API-ownership, state-machine, performance,
  or evidence-file change;
- new final owner `docs/spec/foundation.md` defines exactly
  `API-RESOURCE-001`, `API-SOURCE-TIME-001`, `TIME-001`,
  `CONSTRAINED-STORAGE-001`, `CONSTRAINED-STORAGE-002`,
  `CONSTRAINED-WORK-001`, `RES-LIMIT-001`, `RES-LIMIT-002`,
  `RES-LIMIT-003`, and `ADMIT-MEM-001`;
- the candidate removes duplicate definitions and affected-requirement claims
  from `docs/design.md` and five active WP-100 amendments while preserving
  their residual Core and handler scope and historical corrective provenance;
- the candidate map is 44 final, 76 residual, and one amendment requirement,
  while the active mainline map remains 34/84/3 until integration;
- completed historical tranche authoritative sets were intentionally left
  unchanged because their admission reviews predate `foundation.md`; the new
  D3 entry audit owns this later authority migration;
- candidate-mode boundary validation, requirement ownership, API ownership,
  resource limits, all seventeen ADRs, amendment checks, Foundation/time
  predecessor checks, the design-check suite, and
  `tools/check-design-artifacts.sh` all passed against the immutable candidate;
- changed shell paths pass `bash -n`, and changed Rust path
  `tools/design-check/src/main.rs` passes
  `rustfmt --edition 2024 --check`; full-workspace formatting still reports
  unrelated existing differences in Core and Zenoh paths; and
- authority activation is withheld. A different root session must inspect and
  rerun the exact candidate, then record independent attestation before a
  separate integration checkpoint may activate it.

D4 subscription-ownership verification on 2026-07-26:

- ADR-0003, `docs/spec/binding-spi.md`, `docs/design.md`, API ownership, and the
  WP-300/WP-400 split agree on one driver and one Servient-owned linear facade;
- repository inspection confirms the current Core `Clone`/shared-queue/merge
  behavior is the explicitly removed legacy path rather than target authority;
- `tools/check-architecture-adrs.sh` now requires non-`Clone` host/manual
  facades, absence of split cloneable authority, distinct ids for multiple
  consumers, and WP-400 negative compile evidence; and
- an isolated architecture mutation that removed the non-`Clone` facade rule
  was rejected by that checker; and
- `workspace/0011-subscription-receiver-ownership.md` is `MIGRATED` while
  explicitly preserving unrelated AR3 and implementation gates.

D5 property-read architecture-gate verification on 2026-07-26:

- `docs/work-packages/property-read-architecture-gate.toml` records exactly
  four ordered slices, two broad expansion entry points, three profile cells,
  allowed and forbidden fixture adapters, 21 affected requirements, and the
  mandatory runtime/compile assertions;
- schema v3 of `docs/work-packages/index.toml` assigns the four slice evidence
  keys to WP-100 through WP-400 and the gate evidence key to WP-700;
- the work-package checker validates the manifest path, registered artifacts,
  exact slice dependencies and sequence, package evidence ownership, broad
  entry dependencies/exemptions, the exact one-item/two-path WP-100 slice,
  reused API values, excluded claims, absence of fixture placeholders, and the
  human gate document;
- the design-checker unit/integration suite and the positive work-package DAG
  check pass with the new schema; isolated mutations that reversed the binding
  slice dependency and removed `cleanup-owner` from the forbidden fixture
  adapters were both rejected; and
- `workspace/0009-minimal-end-to-end-architecture-validation.md` is
  `MIGRATED`; no runtime or public API changed.

Property-read handler admission and completion verification on 2026-07-28:

- candidate `a8af29b29e41efb537ae8d6971edf79c42255b65` is the single child of
  frozen base `2d7087d100d4a0d72cabb77476175bf60b0a7925` and changes exactly the
  16 registered non-implementation paths;
- registration checkpoint
  `0406692e3b7b1f3c3f9c02240b44c761dea5065c` changes only the gate manifest
  and continuation state and records the full candidate ref;
- `tools/check-wp100-property-read-handler-slice-entry.sh --candidate` passed
  all 12 predecessor/pre-implementation checks and stopped the completion
  checker only at the expected absent Core trait boundary;
- the design checker passed 19 unit tests and four handler-schema integration
  tests, and `tools/check-design-artifacts.sh` passed all governance and six
  refactor gates;
- an isolated conforming two-file implementation passed the exact source
  projections, no-default/async-no-std/std fixture cells, two semantic tests,
  unresolved async/step negative fixtures, Core tests, downstream Servient and
  Protocol Bindings compilation, and the HandlerContext predecessor suite;
- isolated mutations adding a Core-side `ReadPropertyHandler` implementation
  or a `Send` supertrait were both rejected at the exact source boundary;
- independent attestation
  `5bd9687507c790583452e16526a8842fe7560f67`, exact three-file approval
  `5fdd4566b4fddf007945a1f26fd2eeb0de74ac12`, and exact two-file
  progress checkpoint `76ac45aff4070cccf17b72d8a750e2d478c6a93e` all precede
  implementation;
- `tools/check-wp100-property-read-handler-slice-entry.sh --admission-ready`
  passed before source implementation;
- `tools/check-wp100-property-read-handler-slice.sh` passed after
  implementation across the exact source projection, three feature cells,
  object safety, deliberately non-thread-safe handler, two semantic tests,
  negative async/step scope, Core tests, downstream compilation, and
  predecessor regressions;
- `cargo clippy --locked -p clinkz-wot-core --all-targets --all-features --
  -A clippy::large_enum_variant -D warnings` passed; and
- implementation `830f47ebe044b953a3c0c3214345968f0fb5e571` changes only
  `core/src/handler.rs` and `core/src/lib.rs`; neither planned architecture
  fixture root exists; and
- `tools/check-design-artifacts.sh`, `cargo test --workspace --locked`, and
  all 21 valid feature combinations pass in completion state; and
- after validation-hygiene checkpoint
  `aea1677a8a02e2e2aea184ba87c1424814e1a073`,
  `tools/check-wp100-property-read-handler-slice.sh` followed immediately by
  `tools/check-design-artifacts.sh` passes without cleanup and creates no
  nested fixture `target/`.

D6 candidate-fallback migration verification on 2026-07-28:

- migration checkpoint `7f2869ff03ceb8659ba01e69ba97d9882e944df3`
  contains the accepted ADR and all authoritative projections;
- `tools/check-api-ownership.sh` passed with 690 frozen items, including the
  four planning-owned fallback and diagnostic values;
- `tools/check-design-requirements.sh` passed with all 121 requirements
  indexed exactly once: 34 at final targets, 84 residual, and three in
  registered amendments;
- `tools/check-architecture-adrs.sh` passed with all seventeen accepted
  decisions and the v4.9 backbone registered;
- `cargo run --locked --quiet --manifest-path tools/design-check/Cargo.toml --
  check-work-packages` passed the predecessor suites and authoritative
  work-package DAG;
- `tools/check-design-artifacts.sh` passed all governance and six refactor
  gates after the ADR, API ownership, planning, binding, plan, workspace, and
  continuation projections; and
- no implementation source or planned property-read architecture fixture root
  changed; `WP-200-PROPERTY-READ-PLAN-SLICE` remains blocked until its exact
  candidate and independent review exist.

`cargo test --workspace --all-features --locked` is not a valid project
baseline because it intentionally enables mutually exclusive `zenoh` and
`zenoh-pico` backends. Use the valid feature-matrix script instead.

## Stopping Point and Next Safe Actions

The exact D3 Foundation authority candidate is constructed, committed,
machine-validated, and registered at
`2494f33fdfe49ec3c7ae850d20990e446e628865`. It remains isolated on
`candidate/d3-foundation-authority`; active mainline authority has not switched,
and this candidate-building session must not independently attest or integrate
it.

`WP-100-PROPERTY-READ-HANDLER-SLICE` remains independently reviewed, admitted,
implemented, and complete. Neither planned architecture fixture root exists,
and broad handler entry remains blocked by the separate target/no-atomic,
request/storage, remaining portable-trait, workload, resource, and performance
boundaries.

Next safe actions, in order:

1. in a different root session, review exact candidate
   `2494f33fdfe49ec3c7ae850d20990e446e628865`, rerun its registered checks and
   candidate-boundary mode against base
   `56fea9813df80fe29527755fcb2ce91d43cc5086`, and record a real independent
   attestation only if every result passes;
2. only after that attestation, create a separate integration checkpoint that
   activates the exact candidate without changing its 21-path boundary, then
   update active authority counts from 34/84/3 to 44/76/1;
3. later resolve
   `workspace/0014-property-read-plan-artifact-boundary.md` with one exact
   host/static representation and paired authoring fixtures before preparing
   the WP-200 property-read plan candidate;
4. independently admit every later property-read slice before its source or
   planned architecture fixture root is created;
5. carry the migrated D4, D5, and D6 contracts into future subscription and
   property-read slice work;
6. decompose the next dependency-complete portable-trait or remaining
   request/target tranche without merging its evidence boundary into the
   property-read slice; and
7. ask the Owner only for project-goal, constraint, or external-commitment
   clarification.

Important references:

- `AGENTS.md`;
- `PROJECT_GOVERNANCE.md`;
- `PLAN.md`;
- `ARCHITECTURE_GOVERNANCE.md`;
- `docs/audits/D3-foundation-authority-candidate.toml`;
- candidate
  `2494f33fdfe49ec3c7ae850d20990e446e628865`:
  `docs/audits/D3-foundation-authority-entry.md`,
  `docs/spec/foundation.md`, and
  `tools/check-d3-foundation-authority.sh`;
- `docs/architecture/README.md`;
- `docs/work-packages/index.toml`;
- `docs/work-packages/property-read-architecture-gate.toml`;
- `docs/work-packages/PROPERTY-READ-ARCHITECTURE.md`;
- `docs/audits/WP-100-handler-value-primitives-entry.md`;
- `docs/audits/WP-100-handler-value-primitives-review.toml`;
- `docs/evidence/WP-100-handler-value-primitives.toml`;
- `docs/ADRs/0016-extended-logical-monotonic-time.org`;
- `docs/ADRs/0017-pre-execution-candidate-fallback.org`;
- `docs/amendments/WP-100-time-domain-v1.md`;
- `docs/audits/WP-100-logical-time-correction-entry.md`;
- `docs/audits/WP-100-logical-time-correction-review.toml`;
- `docs/evidence/WP-100-logical-time-correction.toml`;
- `docs/audits/WP-100-deadline-cleanup-timing-entry.md`;
- `docs/audits/WP-100-deadline-cleanup-timing-review.toml`;
- `docs/evidence/WP-100-deadline-cleanup-timing.toml`;
- `docs/audits/WP-100-handler-context-entry.md`;
- `docs/audits/WP-100-handler-context-review.toml`;
- `docs/evidence/WP-100-handler-context.toml`;
- `tools/check-wp100-handler-context-entry.sh`;
- `tools/check-wp100-handler-context.sh`;
- `docs/audits/WP-100-property-read-handler-slice-entry.md`;
- `docs/audits/WP-100-property-read-handler-slice-review.toml`;
- `docs/evidence/WP-100-property-read-handler-slice.toml`;
- `tools/check-wp100-property-read-handler-slice-entry.sh`;
- `tools/check-wp100-property-read-handler-slice.sh`;
- `workspace/0007-time-domain-and-deadline.md`;
- `workspace/0008-implementation-governance-overhead.md`;
- `workspace/0009-minimal-end-to-end-architecture-validation.md`;
- `workspace/0010-complete-design-decomposition.md`;
- `workspace/0011-subscription-receiver-ownership.md`;
- `workspace/0012-property-read-handler-slice-boundary.md`;
- `workspace/0013-candidate-fallback-policy.md`;
- `workspace/0014-property-read-plan-artifact-boundary.md`.
