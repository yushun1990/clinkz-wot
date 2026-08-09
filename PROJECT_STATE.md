# Project State

Last updated: 2026-08-09

## Repository Basis

The active design revision is v5.0 bounded-core authority.

- ADR-0018 decision checkpoint:
  `eb145c5e86ec9e9db0a09194bd4e2868784a927f`.
- Exact non-implementation candidate:
  `b1916250a28ee133e8d0b12225c5b6311c975247`, the single child of the
  decision checkpoint and the tip of `candidate/v5-authority-reset`.
- Independent root-session attestation:
  `6d483a598e654f5c7043efb887074aba3a605f7a`.
- Exact activation merge:
  `30b845a4b17dd3eb56670da48c939b72daea7d59`, whose first parent is the
  attestation checkpoint and whose second parent is the reviewed candidate.
- Activation rollback point:
  `6d483a598e654f5c7043efb887074aba3a605f7a`.
- D9 bounded-conversion governance checkpoint:
  `a952e2b034b8939c0abdaf1662707eaef1d2fdc8`.
- Latest completed Property Read source slice:
  `42fca7a63cefd3a5916a7edc6582044d2c04845e`, the exact two-path WP-200
  Producer-route Planning implementation.
- Exact WP-200 Property Read semantic candidate:
  `4a01b5010729cb42d6e8d51125103c8b5cda8707`, the single child of
  `525bb31b42efe299ed36d46acea1a1c4286e8bde`.
- Independent WP-200 v1 semantic-candidate attestation:
  `8a7aa198f5c983be8fbf5ef1a9750c90b5837703`.
- First WP-200 admission-evidence correction candidate:
  `f453f165c2ea775e5f0d10c36f1e419fcc1d79f3`, the exact seven-path
  single child of the v1 attestation. Independent review rejected its
  pre-source state projection.
- Exact second WP-200 admission-evidence correction candidate:
  `d2dcf2e9d2e19c7c2dfa234f96c5303cc3aee24a`, the exact six-path
  single child of `f453f165c2ea775e5f0d10c36f1e419fcc1d79f3`.
- Independent WP-200 v2 attestation:
  `4f3bdeff604e30eecfbba9c8c12e6dd0b23cc87f`, the exact three-path
  review/registry/continuation checkpoint for that immutable candidate.
- Exact WP-200 pre-source checkpoint:
  `ce1e4ae448617458251f4f437e66e77fb652e86b`, the five-file child of the v2
  attestation that changes the tranche to `in-progress`/`approved`.
- Exact WP-200 implementation:
  `b889ae1dafa65ee66bfb331bebf9d537e1c29eee`, the nine-file child of that
  pre-source checkpoint.
- Design-check runtime-root correction:
  `eacaaf1242a41861758ebc78a40ada2d88d15bba`.
- WP-300 issue-decision and migration checkpoint:
  `d8ed500ddba85997d380adc5071818a90150858b`, which migrates workspace
  issues 0025-0029 without product-source changes.
- Exact WP-300 candidate:
  `e31b975b329fa147bbccf71e5bc6be4254902d89`, the single child of
  `d8ed500ddba85997d380adc5071818a90150858b`; it changes exactly the 20
  registered candidate paths and retains
  `candidate_ref = "register-after-candidate-commit"` so the commit does not
  attempt to contain its own object id.
- Independent WP-300 candidate review: passed on 2026-07-30 with no
  intersecting finding. Attestation checkpoint
  `d5169ba34ad846b2d45d0841b5d57210ee4df0c1` adds exactly
  `docs/artifacts.csv` and
  `docs/audits/WP-300-property-read-binding-slice-review.toml`.
- Remote merge `a8eac3504f7e4252e9c3ac66da5e3038cb532cfc` integrated pull
  request #1 with first parent
  `9082ff4eb24d96572ae1124096185aa20abb3472` and reviewed second parent
  `d5169ba34ad846b2d45d0841b5d57210ee4df0c1`.
- Remote merge `2250d1e7ef1b2a65b52edceabce312e344682374` integrated pull
  request #2 and introduced workspace topics 0030-0041.
- Remote merge `14acdf3ddf19bdab52a2f03901cfa02c34750477` integrated pull
  request #3. Its reviewed content head is
  `90b385f4ae82ba10187d4f67f7656185a577125f`, its actual base is
  `2250d1e7ef1b2a65b52edceabce312e344682374`, and exact-head workflow run
  `30524546209` passed.
- Independent post-integration review of that correction is exact checkpoint
  `4ae812fd603820c48f5be03264cb2a3f2a39e3e0`. Pull request #13 integrated it
  through merge commit `ff86c1582138d9a231dd44068b35cf6464214cd5`, whose
  first parent is `28d717c48a9b6598a93ae09f88503a695392400e` and whose
  second parent is the exact review checkpoint. The merge tree equals the
  reviewed tree, changes the expected five paths, and default-branch
  `mainline` run `30697858866` passed on that merge revision.
- The workspace-topic 0042-0048 decision/migration checkpoint is
  `b8635059059b9f97aba38f6f44fbb59b1eab33b3`; continuation checkpoint
  `26800cc4605b4b1b3198de13a2f156c8d8868523` records its first remote
  handoff. Pull request #12 exact head
  `8a9c5395cd0b70412a4b1c67f6652a1811a1b19e` passed `mainline` run
  `30698464368` and was integrated by merge
  `384461ba33a3639a0f89978159719aa6937e1f36`, whose first parent is
  `ff86c1582138d9a231dd44068b35cf6464214cd5` and second parent is that
  exact head. Its merge tree equals the reviewed head, changes the expected
  24 paths, and default-branch `mainline` run `30698804288` passed.
- Historical WP-300 admission basis was fetched on 2026-08-01 at
  `384461ba33a3639a0f89978159719aa6937e1f36`. It is the reviewed and
  validated `admission_base_ref` for the WP-300 five-file pre-source
  transition. Exact checkpoint
  `6aaf1ef1586428152e535b9cd75f7183ec764cd0` is its single child and changes
  only the five registered admission paths. Exact implementation
  `89fb9f17ac961294032123173b29692a719a174c` is the immediate single child of
  that checkpoint and changes only `core/src/binding.rs` and
  `core/src/lib.rs`.
- WP-300 completion evidence is exact checkpoint
  `c5b59d60c237b275e0121092006bd9f5e1f3c54d`. Pull request #14 exact-head
  run `30701146704` passed and merge
  `b2adf0756c06cc41be5d809c33211d7c20f86aba` integrated its exact reviewed
  tree; default-branch run `30701503019` passed on that merge revision.
- Pull request #16 integrated workspace topics 0050 and 0051 through merge
  `6dcc0df5ddd8f6461670bb8b1e572bda95a68300`, whose expected topic content is
  present on `master`. Default-branch `mainline` run `30741610515` passed on
  that exact merge revision.
- Pull request #17 integrated the 0050/0051 decisions through exact head
  `67e57e09afa8ced0fa5c8cdad72e253c900ad99d` and merge
  `5352e5b00c6f94a7633e4e71c8f80ddd3a7fd80b`. Exact-head run `30742571841`
  and default-branch run `30742976937` passed; the fetched merge contains the
  expected eight-path decision packet.
- Draft pull request #15 preserves original Producer-route semantic candidate
  `613ee18d11b8f60e93d0792fcc76b83a00569044`, the exact 22-path single child
  of `b2adf0756c06cc41be5d809c33211d7c20f86aba`; its exact-head run
  `30705872070` passed. Independent review nevertheless rejected its evidence
  sufficiency because the next-state fixture did not compile, omitted the
  borrowed host projection and cursor-opacity negative contract, and did not
  prove the compiler borrow ended before mutable server preparation.
- Corrective candidate `376ee84f80ea27c8d3faa4b1840ce7b68d61f23f`
  is the exact eight-path single child of the original candidate. Independent
  root-session review on 2026-08-02 passed its corrected next-state source
  simulation, full Producer-route completion check, and declared negative
  mutations. Review checkpoint
  `56ee6b990373c12b83aac26a0377e3489fbde194` changes exactly the registered
  attestation, artifact row, and continuation projection. That checkpoint
  admitted no source; it was later preserved in pull request #15's integrated
  implementation chain.
- Exact current-default reconciliation merge
  `2a469b1d4a3579d4db6115351b76867a0a531db8` preserves the original,
  correction, and review checkpoints and includes fetched default revision
  `5352e5b00c6f94a7633e4e71c8f80ddd3a7fd80b`. It is the immutable
  `admission_base_ref` for the five-file Producer-route pre-source transition.
- Exact pre-source checkpoint
  `342f93f4ef6a1322864456454be958821beb2802` is its five-path single child.
  Exact implementation `42fca7a63cefd3a5916a7edc6582044d2c04845e`
  is the immediate child and changes only `planning/src/lib.rs` and
  `planning/src/property_read.rs`.
- Producer-route completion checkpoint
  `d26f21cb3d3f3f499a770ebf70fbc32bd3f1d24f` was integrated by pull request
  #15 at merge `30485b1a51470f328e79453ba0e82e3358c14f79`. The fetched merge tree
  contains the expected implementation/evidence, and default-branch
  `mainline` run `30758506555` passed on that exact merge revision.
- Pull request #19 integrated workspace issue 0053 at fetched default revision
  `e8e339635f2448b4f377ea6a0c3ba2566134828e`; the expected topic and index
  entry are present and the merge is descended from the verified Producer-route
  integration. Default-branch `mainline` run `31303954629` passed on that
  exact merge revision.
- Draft pull request #18 remains open at exact remote head
  `b4fedb61a63d6eab6b1ca77c0e9a4595a4ed9d8c`, targeting `master` from
  `agent/wp200-route-reservation-correction-candidate`. Exact-head
  `mainline` run `31298103735` passed. GitHub currently reports it draft,
  unreviewed, and non-mergeable against the advanced default branch; no
  independent attestation exists. Its compiler/Core/Planning correction is
  not default-branch authority and does not release WP-400. That branch already
  assigns D45 to the route-reservation decision, so the disjoint 0053 packet
  intentionally uses D46 even though D45 is not yet present on `master`.
- GitHub CLI, SSH fetch/push, and the connected GitHub application are
  available as of 2026-08-09. Active Ruleset `20009352` was
  fetched on 2026-08-02: it targets the default branch, requires strict
  current-base `validation` and resolved review threads, has no bypass actor,
  and permits merge commits. Native automatic integration remains eligible
  only while every terminal predicate is freshly true for the exact head.

The activation candidate changed exactly 27 documentation/checker paths and no
Rust source, Cargo manifest, public API, or runtime behavior. Its independent
review passed the exact candidate checker, aggregate design/evidence suite,
default workspace tests, diff hygiene, and the 21-cell valid feature matrix.

## Continuation Projection

Observed default branch: `e8e339635f2448b4f377ea6a0c3ba2566134828e`

Observed on: 2026-08-09 after fetch plus default ancestry/content inspection.
The fetched tree contains pull request #19's open workspace issue 0053 and is
descended from verified Producer-route merge
`30485b1a51470f328e79453ba0e82e3358c14f79`; default-branch `mainline` run
`31303954629` passed on the observed revision.

Projection mode: conditional remote handoff

The bounded task decides and migrates workspace issue 0053 as a one-time
consumer-backward closure review inside the existing
`WP-400-PROPERTY-READ-SERVIENT-SLICE` candidate/admission boundary. It changes
no product source, does not establish a reusable global gate, and does not
review or widen pull request #18. WP-400 remains blocked.

### Before verified integration

The exact 0053 decision packet on
`agent/decide-wp400-first-entry-closure` passes its Property Read checker,
aggregate design suite in a clean target, locked workspace tests, 21-cell
feature matrix, formatting, and diff hygiene. Commit and push only that scope,
open one draft pull request against `master`, and verify exact-head remote
`validation`. Keep pull request #18 independently draft/unreviewed and keep
WP-400 candidate and source blocked.

### After verified integration

Fetch `master` and verify the 0053 decision pull request's actual base, merge
ancestry, expected decision content, and passing merge-revision `validation`.
Then reconcile pull request #18's immutable route-reservation candidate onto
that fetched default without widening its upstream source/evidence scope, rerun
its exact candidate checks, and obtain the still-missing independent review.
Only independently reviewed implementation and verified default integration of
that correction may release the WP-400 candidate. The WP-400 candidate must
then include D46's first-entry closure evidence in its normal independent
review before any pre-source admission.

The narrow WP-200 plan slice is complete. Its handoff to WP-300 contains:

1. one associated-type portable Core compiler/artifact contract;
2. an application-closed typed static compiler/cursor/artifact composition;
3. Core-owned safe host erasure that returns original owned values on mismatch;
4. an immutable owned Property Read plan and artifact set with no TD lifetime;
5. bounded, deterministic planning progress in all three feature cells; and
6. sole WP-200 implementation ownership, which WP-300 may consume only inside
   a complete installable registration.

The exact non-product-source WP-300 candidate contains paired public
host/static authoring contracts, a five-test executable lifecycle schema,
entry/completion checks, and the expected
absent-`core/src/binding.rs` completion boundary. The candidate itself contains
no implementation or cross-package Property Read architecture fixture root;
the later exact pre-source and implementation checkpoints preserve its
registered two-path boundary.
The semantic candidate review and admission-topology correction are complete. The
correction's independent review is integrated and default-branch validated at
`ff86c1582138d9a231dd44068b35cf6464214cd5`.
The earlier validation prerequisite remains resolved:
`tools/design-check` now selects an explicit runtime worktree root from its
callers, falls back only to runtime ancestor discovery, and rejects an invalid
explicit root instead of silently checking another worktree.

Workspace issues 0025-0029 are decided and migrated. They establish a finite
Producer Property Read slice, two distinct downstream release events, strict
no-backflow into legacy selection, explicit global-gate impact mapping, and
reuse of the existing immutable candidate/transition machinery. Independent
candidate review is complete.

Workspace issues 0030-0041 are also decided and migrated. They establish
terminal-only automatic integration eligibility; an explicit evidence-claim
ladder; a production-authoring spike; sharded Servient orchestration; required
all-route publication; one complete-object cleanup kernel; host/constrained
semantic trace parity; generated projections over the exhaustive resource
schema; build/deploy plugin semantics; explicit-retry rather than failover;
remote/repository/state truth separation; and staged no-backflow evidence.

Workspace issues 0042-0048 are decided, migrated, and integrated. They establish a
six-rung Binding-SPI maturity ladder and honest Zenoh-family claim; a private
Servient owner graph and early cross-shard feedback checkpoint; cleanup
obligation coexistence and observable progress; one shared Host/constrained
kernel and trace oracle with normalized liveness; separation of atomic
publication from the conservative all-route v1 policy plus a bounded explicit
retry-facade contract; separation of the canonical resource schema from stable
authoring projections; and a default-branch reachability/content/validation
predicate for remote truth. Pull request #12 and its merge-revision validation
close that disjoint integration boundary.

Workspace issue 0049 is decided and migrated in the active correction
candidate. It records the concrete Producer-route planning reachability gap,
preserves the completed Consumer-call and Producer-registration claims, and
inserts one exact independently reviewed correction before WP-400. The topic
does not itself admit either correction source or WP-400 source.

Workspace issues 0050 and 0051 are decided, migrated, and integrated. They
establish that tranche completion is package-local, predecessor
completion releases successor preparation rather than source, and declared
cross-package handoffs are proved by real upstream output at the successor's
first legal entry. They also replace an unconditional current objective with a
merge-stable continuation envelope whose observed local basis and two
conditional actions are aggregate-checked.

## Active Milestones

- M0 Execution Baseline and Collaboration Reset — CLOSED.
- M1 v5.0 Authority Reset and Architecture Closure — IN_PROGRESS.
- M2 Foundation and Core Contract Stabilization — IN_PROGRESS.
- M3 Planning and Compilation Pipeline — IN_PROGRESS; the exact WP-200
  Property Read plan slice is complete. Its exact Producer-route projection
  correction is also `complete`/`approved`; broad WP-200 exits remain open.
- M4 Protocol Binding SPI and Lifecycle — IN_PROGRESS; the exact WP-300
  Property Read binding slice is `complete`/`approved`. Broad WP-300 exits
  remain open. The completed Producer-route correction releases only the
  route-reservation correction; narrow WP-400 candidate/review preparation
  remains blocked until that correction is independently reviewed, integrated,
  and verified on the default branch.

The v5 authority switch is complete, but M1 remains open because GATE-1,
GATE-2, GATE-4, GATE-5, and GATE-6 still require their registered closure
evidence. GATE-3 remains closed.

## Accepted Technical Model

Active v5 authority contains 62 requirements:

- 41 indispensable architecture/safety requirements; and
- 21 requirements protecting the first Property Read vertical slice.

The other 59 inherited v4.9 identities have checked inactive dispositions:

- 34 are mandatory domain-entry review input;
- 15 are historical design input;
- four premature or superseded identities are retired; and
- six redundant identities defer to stronger owners.

The package order remains:

`WP-000 -> WP-100 -> WP-200 -> WP-300 -> {WP-400, WP-500, WP-600} -> WP-700`.

ADR-0013 permits a dependency-complete, independently reviewed tranche to
proceed while disjoint global gates remain open. Package status alone never
admits source work.

D9 adds these execution rules:

- the active critical path names one executable objective, finite blockers,
  an observable closure event, and the next source event;
- decision, authoritative migration, authoring fixtures, and admission share
  one conversion packet when contract, rollback, and validation truth match;
- post-closure refinement may block only on an explicit intersecting semantic,
  ownership, lifecycle, resource, dependency, or evidence-truth finding;
- continuity, registry, audit, and checker changes travel with the checkpoint
  whose truth they record; and
- authority closure, package-local completion, and executable vertical
  integration are reported separately.

D18-D22 add the WP-300 conversion boundary:

- the narrow tranche installs one complete bundle advertising only Producer
  Property Read and covers immediate/external readiness, committed-closed
  routes, permit-gated acceptance, one response, cleanup, and host/static
  authoring;
- optional interface methods may reject unadvertised roles before state or
  side effects, but do not complete client, subscription, emission,
  contribution, broad cancellation, workload, Servient, or concrete-protocol
  behavior;
- narrow WP-300 completion releases only the exact Producer-route planning
  correction; that correction's completion releases narrow WP-400, while
  broad WP-300 completion releases broad WP-400, WP-500, and WP-600;
- target Planning/Core/WP-300 code cannot call a legacy selector, rescan a TD,
  or send a target request into legacy dispatch; WP-600 removes concrete call
  edges and WP-700 proves final absence;
- global findings affect scoped evidence only through an explicit mapped
  intersection or named revalidation; and
- WP-300 reuses immutable candidate, independent scoped review, exact
  next-state simulation, combined pre-source admission, and completion
  evidence rules.

D23-D34 add these execution and realism boundaries:

- native auto-merge is terminal-only and remains disabled until exact-head,
  current evidence, strict up-to-date validation, conversation resolution,
  non-stacked/conflict-free state, and Owner-boundary conditions are freshly
  verified;
- evidence claims progress from package-local slice to mock cross-package gate
  to real Zenoh smoke to workload/release readiness; no earlier rung implies a
  later one;
- a bounded external Zenoh authoring spike follows narrow WP-300 and can reopen
  the SPI only on a concrete ownership, portability, resource, unsafe, or
  implementability defect;
- Servient keeps semantic authority but shards host progress by
  Thing/generation and route/slot, using bounded cursors, brief critical
  sections, and callbacks outside locks;
- every advertised Producer route is required for one frozen generation;
  omission requires a new effective TD and generation;
- cleanup uses one complete-object offer/acknowledge-or-return kernel, with
  `NoCleanupSuccessor` for simple synchronous authors;
- host and constrained profiles share trace ids, outcomes, and resource
  deltas; compile-only cells make no runtime claim;
- the exhaustive resource schema remains authoritative, with generated
  profile/role builders as checked complete projections;
- v1 plugin deployment is build/install/deploy composition, not dynamic
  loading; external configuration still creates a new Servient generation;
- forms are candidates rather than failover routes, and explicit retry is a
  fresh call with strict selection and fresh time/work/security budgets;
- remote integration facts, repository technical truth, and curated state are
  separate projections that fresh sessions reconcile; and
- no-backflow evidence advances from WP-300 poisoned exits through WP-400 zero
  calls and WP-600 concrete-edge removal to WP-700 final absence.

D35-D41 add these maturity and continuation boundaries:

- Binding-SPI evidence advances through immutable consistency, narrow
  constructibility, external Zenoh authoring, mock cross-package composition,
  production Zenoh-family execution, and final release evidence; Zenoh and
  zenoh-pico alone do not prove protocol-shape neutrality;
- broad Servient admission requires a private owner/dependency graph,
  complete-object cross-shard handoff, distinct bounded scheduling domains,
  one shared semantic kernel, and an early multi-Thing/multi-binding feedback
  checkpoint;
- cleanup admission requires an operation/obligation coexistence matrix,
  observable pending progress, and an explicit v1 in-instance residual
  durability boundary;
- Host/constrained parity uses one code-level transition kernel and trace
  oracle, compares semantic resources and normalized liveness, and separately
  bounds profile-specific physical costs;
- atomic publication truth is distinct from the conservative v1 all-route
  policy, while degraded recovery and bounded multi-attempt retry remain
  explicit application/platform contracts;
- the exhaustive flat resource schema remains canonical authority rather than
  a stable public authoring surface; broad maturity requires executable
  applicability, typed projections, diagnostics, revision/digest identity,
  and field-lifecycle discipline; and
- remote integration requires actual-base inspection, default-branch
  reachability, expected content, and relevant validation, while
  `PROJECT_STATE.md` records the last observed projection and `PLAN.md` retains
  only durable roadmap/package truth.

D43-D44 add these release and continuation boundaries:

- tranche completion proves its registered package-local contract only;
  successor preparation and successor source admission are distinct events;
- a declared cross-package handoff is owned by successor-entry evidence that
  carries real upstream output into the first legal downstream entry without
  fixture-owned substitutes; and
- every handoff checkpoint records one fetched-default basis plus explicit
  pre-integration and post-integration actions, and aggregate validation proves
  the envelope shape and local ancestry without claiming live GitHub truth.

D8 selects this exact representation:

- `BindingCompilerExtension` owns associated `Cursor` and `Artifact` types;
- `step` consumes the cursor and returns it on pending or failure;
- constrained applications compose heterogeneous third-party compilers with
  one closed application enum and matching cursor/artifact enums;
- `HostBindingCompilerRegistration` is a Core-erased `std` component with safe
  borrowed and consuming payload access;
- `StaticBindingCompilerRegistration<C>` is typed in every feature cell;
- neither compiler component is independently installable;
- `BindingArtifactEnvelope<A>` checks full generation/configuration/
  compatibility/role identity and measured footprint; and
- `PlanBuildOutput<A>` owns logical plans, envelopes, and compact references
  without retaining the borrowed TD or registration snapshot.

## Implementation Truth

Completed and independently evidenced WP-100 work includes:

- Foundation refresh;
- handler value primitives;
- extended logical time;
- Deadline and cleanup timing;
- borrowed `HandlerContext`; and
- synchronous static `ReadPropertyHandler`.

Completed WP-200 narrow-slice work includes:

- the `clinkz-wot-planning` crate in the root workspace;
- Core logical-plan, generation, compiler, typed static registration, safe
  host-erasure, artifact identity/envelope, compatibility, role, footprint,
  and rejection values;
- a bounded Property Read `PlanCompiler` that resolves the selected form and
  target only during planning;
- owned plan output that outlives its borrowed TD and registration inputs; and
- external constrained/static and host authoring fixtures with mismatch,
  footprint, zero-budget, and deterministic-step evidence.

The legacy form-selection implementation still exists in
`protocol-bindings/core`; Servient still stores `Arc<dyn ClientBinding>` and
`Arc<dyn ServerBinding>` directly; and existing concrete bindings still use
the legacy execution boundary. D12 assigns those one-way migrations to
WP-300, WP-400, WP-600, and final WP-700 removal evidence, so their presence
does not weaken the completed narrow WP-200 claim.

## Open Decisions and Blockers

### Focused execution-risk decisions / workspace issues 0017-0051 and 0053

Status: issues 0017-0051 are migrated and integrated; issue 0053 is migrated
in the current decision packet and awaits remote integration. Issue 0052 and
its route-reservation migration remain confined to unreviewed draft pull
request #18 and are not default-branch authority.

- 0017: the WP-200 admission path has a finite stopping condition. Independent
  review now includes the exact next-state simulation; the five-file
  pre-source transition passes, omission of its carry-forward update fails,
  out-of-scope implementation work fails, and the exact nine-path child now
  has completion evidence.
- 0018: the Property Read source path remains necessarily serial where each
  next slice consumes a preceding public ownership/lifecycle boundary. Later
  preparation is allowed but is neither admission nor vertical progress, and a
  newly proved handoff gap may insert one owner-scoped correction.
- 0019: the legacy migration map is owned by WP-200, WP-300, WP-400, WP-600,
  and WP-700. Coexistence is permitted only at their named one-way adapters;
  final absence is a WP-700 claim, not a narrow-gate claim.
- 0020: governance artifacts form a directed
  owner -> projection -> evidence -> validation graph. The two WP-200
  correction cycles were real transition-validation defects, not architecture
  changes; transition reviews must now execute their declared next state.
- 0021: D8 is reaffirmed for admission. The external host and
  `no_std + alloc` fixtures use public dependencies and preserve ownership;
  both compile at WP-200 completion, while the WP-600 production compilers own
  the remaining production-author usability evidence.
- 0022: independent review is scoped by defect class. Session separation alone
  does not close runtime, lifecycle, resource, workload, performance, or
  production-author claims.
- 0023: the local/remote integrity concern is confirmed and migrated. Active
  repository Ruleset `20009352` targets the default branch and requires the
  `validation` job from GitHub Actions; that exact check passed in mainline
  workflow run `30503733056` for remote `master`
  `9082ff4eb24d96572ae1124096185aa20abb3472`. The classic branch-protection
  summary's `off` value does not describe effective Ruleset enforcement and
  must not be used alone for this audit.
- 0024: the full v1 target remains coherent and bounded by registered package
  and evidence exits. The critical path reaches the Property Read gate and
  broad WP-300 completion, then WP-400/WP-500/WP-600 branch and rejoin at
  WP-700 before M7.
- 0025: the WP-300 Property Read slice is finite. It owns one complete
  Producer Property Read registration, both readiness forms, permit-gated
  request acceptance, response delivery, cleanup, and paired host/static
  authors; optional interface presence is not a broad behavior claim.
- 0026: narrow WP-300 completion releases only narrow WP-400 Property Read
  entry preparation. The concrete 0049 handoff defect refines that edge
  through one exact Producer-route Planning correction. Broad WP-300
  completion analogously releases broad successor preparation; every
  successor still requires its own source admission and declared handoff
  evidence.
- 0027: no current target request can leak because WP-300 target execution is
  absent, but the negative boundary needed strengthening. New target code may
  not call legacy selectors or dispatch; WP-600 removes concrete calls and
  WP-700 proves final public selector/adapter absence.
- 0028: ADR-0013 remains unchanged. A candidate's requirement, artifact,
  state, resource, dependency, exclusion, and evidence map owns the impact
  boundary; intersecting findings block/reopen, compatible changes receive
  named revalidation, and disjoint findings cannot cause broad re-review.
- 0029: WP-300 reuses the WP-200 immutable candidate and transition model.
  Only contract-specific lifecycle/authoring checks are new. Preparation ends
  at passing prechecks, executable schema, expected absent-source completion
  failure, and reviewed next-state simulation.
- 0030: automatic integration is allowed only as a verified terminal action;
  handoff stays draft until current remote ruleset prerequisites are proven.
- 0031: the shortest valid feedback path is narrow WP-300, the exact 0049
  Producer-route correction, narrow WP-400, the mock Property Read gate, then
  a real Zenoh smoke and broader claims.
- 0032: constructibility fixtures are followed by a bounded Zenoh authoring
  spike; only concrete defects reopen the SPI.
- 0033: Servient orchestration is semantically central but physically sharded
  by Thing/generation and route/slot.
- 0034: all advertised routes are required; degraded publication and late join
  are not v1 semantics.
- 0035: cleanup is one complete-object transition with acknowledged transfer
  or unchanged manual return.
- 0036: host and constrained implementations share semantic traces and
  resource deltas; only representation mechanics differ.
- 0037: generated complete builders project the exhaustive flat resource
  schema; they do not create a second authority.
- 0038: startup plugins are Cargo/package/deployment composition; the platform
  and engine ownership boundary is explicit.
- 0039: candidates are not failover; explicit retry is a fresh caller action
  with strict selection and fresh budgets/security.
- 0040: remote, repository, and continuation facts are reconciled explicitly;
  the WP-300 review/admission ref conflation is corrected and mainline runs the
  registered work-package checker.
- 0041: no-backflow proof is staged from poisoned target exits to zero calls,
  concrete-edge removal, and final public/source absence.
- 0042: the narrow Binding-SPI candidate proves constructibility only.
  Maturity now has six evidence rungs; the Zenoh authoring spike has concrete
  reopening predicates, and Zenoh plus zenoh-pico remain one protocol family.
- 0043: broad Servient admission requires a private owner graph, one-way
  dependencies, complete-object cross-shard handoff, bounded scheduling
  domains, one shared trace kernel, and early multi-route feedback. Narrow
  Property Read remains admissible.
- 0044: complete-object cleanup remains frozen while exact Rust layout stays
  provisional. Broad admission now owns obligation coexistence, pending
  progress observability, unique progress authority, and an honest
  in-instance residual durability boundary.
- 0045: common Host/constrained capabilities use one code-level kernel and
  trace oracle. Semantic resources and normalized liveness must match;
  profile-specific physical costs remain separately bounded, and compile-only
  cells make no runtime claim.
- 0046: atomic publication and all-route-required are distinct. The latter is
  a conservative v1 availability policy; recovery generation construction is
  application/platform-owned, and `RetryClass` alone is not a bounded retry
  facade.
- 0047: the flat exhaustive resource schema remains canonical but is not yet a
  stable authoring surface. Broad maturity requires executable applicability,
  typed role projections, diagnostics, revision/digest identity, and field
  lifecycle evidence.
- 0048: dependent work proves remote truth from actual base, default
  reachability, expected content, and validation. Dangerous continuation drift
  is corrected before the next transition; PLAN no longer owns transient
  branch or handoff facts.
- 0049: the completed Producer-route projection carries a real
  `BindingArtifactRef` into `PrepareInput` and is integrated at
  `30485b1a51470f328e79453ba0e82e3358c14f79`; its local role/reference claim
  remains valid despite the later reservation finding.
- 0050: completion is package-local. Successor preparation and source
  admission are distinct; real upstream output must reach the first legal
  downstream entry when a handoff is declared. The confirmed Property Read
  gap blocks WP-400 source without revoking WP-200/WP-300 local completion.
- 0051: continuation state is a merge-stable conditional envelope, not live
  remote truth. It records one fetched-default basis and both integration
  branches; aggregate validation rejects the stale unconditional shape and
  checks local ancestry only.
- 0053: D43 proves declared handoffs one at a time and does not close a
  successor's complete input set. The exact WP-400 boundary is
  `binding-route:Absent:begin_prepare->Preparing`; one machine-readable
  consumer-backward provenance table now belongs to the existing WP-400
  candidate/review. It is one-time, does not widen or validate pull request
  #18, and blocks WP-400 source on any unowned/synthetic/recomputed value,
  generation mismatch, missing pre-side-effect reservation, or profile
  divergence.

Candidate preparation also resolved one Rust staging constraint. The existing
legacy `core::inbound::ServerBinding` owns `shutdown(&ThingId)`, while the
target route lifecycle needs `shutdown(RouteShutdownInput)`; Rust traits cannot
overload those methods. The narrow target therefore uses uniquely named
`RouteServerBinding`, `RouteInboundRequest`, `RouteResponseOpportunity`, and
`RouteInboundResponse` in future `core/src/binding.rs`. Legacy
`core/src/inbound.rs::{ServerBinding, InboundRequest, InboundResponse,
BindingContext}` remains unchanged and cannot be called from the target
generation. WP-700 removes the legacy exports only after WP-400/WP-600 have
migrated.

### D8 / workspace issue 0014

Status: MIGRATED. No technical representation or package-ownership decision
remains open.

Required authoritative consumers:

- `docs/spec/planning.md`;
- `docs/spec/binding-spi.md`;
- the relevant architecture flow/module projections;
- `docs/api-ownership.csv`;
- `docs/work-packages/index.toml`;
- `docs/work-packages/WP-200-planning.md`;
- `docs/work-packages/WP-300-protocol-binding-spi.md`;
- `docs/work-packages/property-read-architecture-gate.toml`;
- paired public compile-contract fixtures;
- the tranche audit/checker; and
- PLAN, workspace lifecycle, artifact registry, and this state checkpoint.

The semantic candidate passed independent v1 review, the exact
evidence-boundary correction passed independent v2 review, and the exact
pre-source/implementation topology now has completion evidence. The
representation and package ownership decisions are not reopened.

### WP-200 completion boundary

Status: NARROW PROPERTY READ PLAN SLICE COMPLETE.

The immutable topology is:

`4f3bdeff604e30eecfbba9c8c12e6dd0b23cc87f`
`-> ce1e4ae448617458251f4f437e66e77fb652e86b`
`-> b889ae1dafa65ee66bfb331bebf9d537e1c29eee`.

The middle checkpoint changes exactly the five registered pre-source paths.
The implementation child changes exactly the nine registered Core/Planning
paths. `docs/evidence/WP-200-property-read-plan-slice.toml` binds that
implementation ref to the completion checker. The integration gate now has
WP-300 `complete`/`approved`, WP-400 planned/blocked, and remains globally
`blocked`.

### WP-300 Property Read binding slice

Status: NARROW PROPERTY READ BINDING SLICE COMPLETE.

The candidate is the exact 20-path single child of decision checkpoint
`d8ed500ddba85997d380adc5071818a90150858b`. It owns:

- the exact active-requirement, API, state-machine, implementation-path,
  exclusion, evidence, and transition projection;
- paired external static and host authoring contracts;
- five executable ownership/lifecycle schema tests;
- an entry check with immutable-candidate and exact five-file next-state
  boundaries; and
- a completion check that fails first and exactly while
  `core/src/binding.rs` is absent.

The semantic candidate changes no product source. Its immutable review
attestation is present at
`d5169ba34ad846b2d45d0841b5d57210ee4df0c1`. The aggregate checker formerly
substituted current `HEAD` for that ref and required a later pre-source commit
to be its direct child, which is false after valid merge integration. Pull
request #3 integrated the correction that binds `review_attestation_ref`
separately from a deferred `admission_base_ref`, requires the attestation to be
an ancestor of the base, and makes the exact five-file checkpoint a single
child of the base.

Independent inspection of the exact pull-request #3 head found no intersecting
semantic change. It also reproduced the passing five-file
`--admission-ready` transition and the accepted exact two-path implementation
topology while rejecting an incomplete completion claim. Local checkpoint
`4ae812f` records that result; merge checkpoint
`ff86c1582138d9a231dd44068b35cf6464214cd5` integrates and validates it on the
default branch. Pull request #12 merge
`384461ba33a3639a0f89978159719aa6937e1f36` is the later reviewed default
descendant selected as the admission base. Exact five-file checkpoint
`6aaf1ef1586428152e535b9cd75f7183ec764cd0` binds that base. Its immediate
single child `89fb9f17ac961294032123173b29692a719a174c` implements exactly
`core/src/binding.rs` and `core/src/lib.rs`; existing `core/src/inbound.rs`
remains outside the admitted and completed path set.

`docs/evidence/WP-300-property-read-binding-slice.toml` binds the implementation
ref to the registered completion checker and all 27 active narrow-slice
requirements. Pull request #14 integrates that completion at
`b2adf0756c06cc41be5d809c33211d7c20f86aba`, and merge-revision validation
run `30701503019` passed. This completion released successor-entry preparation,
which exposed the real Producer-route gap and therefore released only the
exact Planning correction's candidate/review work. It does not release WP-400
source. Broad binding roles, production protocol execution, Servient
composition, workload proof, and cross-package completion remain separate.

### WP-200 Property Read Producer-route projection

Status: NARROW PRODUCER-ROUTE PROJECTION COMPLETE.

Original candidate `613ee18d11b8f60e93d0792fcc76b83a00569044` owns 22
registered non-product-source paths and passed exact-head remote run
`30705872070`; independent review rejected only its next-state evidence.
Corrective candidate `376ee84f80ea27c8d3faa4b1840ce7b68d61f23f`
changes exactly eight registered support paths, grants no source authority,
and passed independent review on 2026-08-02. Exact three-path review checkpoint
`56ee6b990373c12b83aac26a0377e3489fbde194` records the immutable correction
and all eight prechecks.

Exact pre-source checkpoint `342f93f4ef6a1322864456454be958821beb2802`
is the five-path single child of admission base
`2a469b1d4a3579d4db6115351b76867a0a531db8`. Its immediate implementation
child `42fca7a63cefd3a5916a7edc6582044d2c04845e` changes exactly
`planning/src/property_read.rs` and `planning/src/lib.rs`. The review rejected
Consumer-role substitution, caller-restated registration identity,
arbitrary-role/public Consumer construction, forgeable cursor state, omitted
borrowed static/host projections, fixture-owned preparation, zero-budget
progress, source outside the two-path boundary, and premature WP-400 fixtures.

Author-side schema tests, ownership/design/ADR/resource/authority checks,
formatting, shell syntax, carry-forward digests, corrected candidate topology,
full next-state completion, and diff hygiene pass. The real implementation now
passes the same three-cell completion checker. Reconciliation merge
`2a469b1d4a3579d4db6115351b76867a0a531db8` includes fetched default checkpoint
`5352e5b00c6f94a7633e4e71c8f80ddd3a7fd80b` without rewriting the original,
correction, or review checkpoints. Completion evidence binds the exact
implementation and six active requirements. Pull request #15 integrated the
complete chain at `30485b1a51470f328e79453ba0e82e3358c14f79` and its exact
merge-revision validation passed.

Completion preparation exposed one support-only evidence defect: the tranche
selected a handler-scoped generic parser that hard-coded `WP-100`. The
Producer-route-specific parser now requires honest `WP-200` ownership, exact
review/admission/implementation ancestry, the five-path pre-source checkpoint,
the two-path implementation, and the registered completion command. This
changes no product source or public API.

The honest completion tree passes `check-work-packages`. An isolated mutation
that relabels the completion evidence as `WP-100` is rejected with the exact
Producer-route ownership mismatch before completion can be accepted; the
mutation was then discarded.

### WP-200 route-reservation projection and WP-400 first-entry closure

Status: ROUTE-RESERVATION CANDIDATE UNREVIEWED; WP-400 BLOCKED.

WP-400 reconstruction found that the completed Producer-route fixture obtains
`RouteReservationIdentity` from direct fixture constants. No default-branch
compiler output, artifact metadata, or plan output supplies the canonical
reservation to a real Servient. Draft pull request #18 proposes an exact
compiler/Core/Planning metadata correction at remote head
`b4fedb61a63d6eab6b1ca77c0e9a4595a4ed9d8c`; its validation passed, but it
has no independent review and GitHub reports it non-mergeable after `master`
advanced. Its contract is not widened or technically validated by issue 0053.

Issue 0053 confirms a distinct evidence gap: D43 validates each declared
handoff but did not require the consumer's complete first-entry input set. D46
therefore adds one bounded review inside the existing WP-400 candidate. The
exact boundary is
`binding-route:Absent:begin_prepare->Preparing`, not `PrepareInput` alone.
The existing Property Read gate now carries nine checked provenance rows for
the real plan/artifact, complete registration, produced Thing/generation,
handler coverage, policy, plan-set ownership, route assembly, and
activation/cleanup ownership. It also declares the finite negative mutations.

No third upstream correction is currently required after the proposed
reservation carrier: remaining required values are either existing production
outputs/root inputs or WP-400-owned derivations. Guards, accepted requests,
security results, handler/response values, and publication state arise later;
`AcceptHint`, final `InteractionInput` storage, subscription/emission,
multi-route, protocol, and workload claims remain broad exclusions.

The closure is one-time and uses the existing manifest, design checker,
work-package documents, candidate review, and future architecture runner. It
adds no global governance rule, new tranche, audit family, or independent
review cycle. WP-400 source fails closed on an unowned or fixture-only value,
illegal recomputation, generation mismatch, missing pre-side-effect resource
or cleanup reservation, or host/static semantic divergence.

The current decision worktree passes the design-check unit suite,
`check-work-packages`, the active-v5 aggregate design suite in a clean isolated
Cargo target, `cargo test --workspace --locked`, the supported 21-cell feature
matrix, formatting, carry-forward digests, and diff hygiene. An isolated
mutation that relabels `producer-route-artifact-metadata` from
`upstream-output` to `root-input` is rejected with the exact provenance
mismatch; the mutation was then discarded.

### Aggregate design-check worktree-root defect

Status: DESIGN CHECK RESOLVED; PERFORMANCE HARNESS LIMITATION CONFIRMED.

Commit `eacaaf1242a41861758ebc78a40ada2d88d15bba` removes the compile-time
repository root. Design-check callers export
`CLINKZ_WOT_REPOSITORY_ROOT`; direct runs may discover the nearest runtime
ancestor; explicit invalid roots fail closed. Three unit tests own precedence,
ancestor discovery, and invalid-root rejection. A real shared-target
regression built the checker in a detached temporary worktree, removed that
worktree, and then executed the cached binary from `/tmp` against the current
root successfully. The temporary worktree and 160.9 MiB test target were
removed.

The aggregate script also invokes `tools/performance-harness`, which still
derives its repository root from compile-time `CARGO_MANIFEST_DIR`. On
2026-08-09 the shared default target reused a harness binary built in a deleted
pull-request #18 validation worktree and therefore failed before reading the
current performance schemas. Rebuilding in a clean isolated Cargo target
verified all 66 fixtures/cases and allowed the complete aggregate suite to
pass. This pre-existing support-tool cache defect is disjoint from issue 0053,
does not affect a clean remote runner, and should be corrected in a separate
bounded task rather than widening this decision packet or pull request #18.

### Disjoint downstream blockers

- Broad WP-100 handler entry still lacks its remaining request/target
  migration, portable async/step admission, no-atomic public-boundary proof,
  and workload/resource evidence.
- The exact narrow WP-300 Property Read slice is complete. Broad WP-300 still
  waits on external authoring, cleanup-coexistence, shared-parity-oracle, and
  resource-authoring evidence.
- Narrow WP-400 candidate/review preparation is blocked until the
  route-reservation correction is independently reviewed, implemented,
  remotely integrated, and default-branch validated. Its candidate must then
  close D46's one-time first-entry table before its own pre-source admission.
  Broad WP-400, WP-500, and WP-600 still depend on broad WP-300; WP-700 joins
  those branches.

These do not extend the D8 packet unless repository evidence shows a direct
contract, rollback, or validation intersection.

### Remote integration controls

Status: RULESET VERIFIED; CLI AUTHENTICATION AVAILABLE.

Ruleset `20009352` was fetched on 2026-08-02 and targets the default branch
with strict current-base `validation` plus resolved review threads, no bypass
actor, and merge commits allowed. `gh auth status` passed for account
`yushun1990` on 2026-08-09; the connected GitHub application and SSH Git
transport are also available. Pull request #13 previously exercised the full
terminal path; Producer-route merge-revision run `30758506555` confirms that
integrated predecessor.

## Rejected or Superseded Approaches

- Lossless D3 domain-by-domain authority migration is superseded by ADR-0018.
- Foundation candidate
  `2494f33fdfe49ec3c7ae850d20990e446e628865` remains historical input and must
  not be activated.
- Partial v5 activation or piecemeal rollback is prohibited.
- A separate documentation/review cycle for each artifact in one semantic
  conversion packet is rejected by D9.
- Treating package-local completion as implicit successor source admission is
  rejected; a declared handoff is a separately falsifiable successor-entry
  claim.
- An unconditional continuation objective, pre-written merge success, a
  write-capable post-merge state bot, and recursive state-only pull requests
  are rejected. The merge-stable conditional envelope closes the gap without
  claiming live remote truth.
- Protocol Bindings selecting handlers, rescanning TDs at runtime, or owning
  Servient orchestration remains outside the frozen direction.
- A representation that works only through in-repository private types, or
  only for `std` trait objects, cannot close issue 0014.
- Binding-authored unsafe erasure, a heap-erased representation in every
  feature cell, separate host/static public compiler traits, independently
  installable compiler halves, and a WP-300 duplicate implementation owner are
  rejected by D8.
- Proceeding with the original four-file pre-source checkpoint is rejected
  because it makes the authoritative carry-forward digest stale.
- Dropping or weakening `v5-authority-reset-candidate-check` is rejected; the
  exact carried digest is active v5 evidence, not optional bookkeeping.
- Entering product source on the v1 semantic attestation alone is rejected
  because the discovered evidence-truth defect intersects admission.
- Attesting first correction
  `f453f165c2ea775e5f0d10c36f1e419fcc1d79f3` is rejected because its exact
  five-file pre-source simulation requires implementation source before the
  registered implementation commit.

## Verification Baseline

Independent review of candidate
`b1916250a28ee133e8d0b12225c5b6311c975247` on 2026-07-28 passed:

- `tools/check-v5-authority-reset-candidate.sh`;
- `tools/check-design-artifacts.sh`;
- `cargo test --workspace --locked`;
- `sh scripts/check-feature-matrix.sh` — 21 passed, 0 failed; and
- `git diff --check
  eb145c5e86ec9e9db0a09194bd4e2868784a927f..b1916250a28ee133e8d0b12225c5b6311c975247`.

The activation merge was additionally checked to have exact parents and zero
content difference from the candidate across all 27 candidate paths.
Post-activation status reconciliation passed:

- `tools/check-v5-authority-reset-candidate.sh`;
- `tools/check-design-artifacts.sh`;
- `cargo test --workspace --locked`;
- `sh scripts/check-feature-matrix.sh` — 21 passed, 0 failed; and
- `git diff --check`.

These checks preserve the active-owner, carry-forward, completed-evidence,
workspace-test, and feature-matrix baselines.

Author-side D8 schema validation on 2026-07-28 passed:

- the 19 design-check unit tests;
- four existing handler API schema tests; and
- four new WP-200 compiler/artifact schema tests covering closed static
  dispatch, zero-budget cursor preservation, safe host mismatch recovery,
  artifact footprint rejection, and TD-lifetime-free owned output.

Author-side exact candidate validation also passed:

- `tools/check-wp200-property-read-plan-slice-entry.sh --candidate`, including
  the exact base/parent/path gate and the expected absent-implementation-source
  boundary;
- `tools/check-design-artifacts.sh`;
- `cargo test --workspace --locked`;
- the supported feature matrix, 21 passed and 0 failed; and
- staged/committed diff hygiene and the exact 25-path candidate comparison.

Independent root-session review of exact candidate
`4a01b5010729cb42d6e8d51125103c8b5cda8707` on 2026-07-29 passed:

- `tools/check-wp200-property-read-plan-slice-entry.sh --candidate`;
- inspection of the full 25-path diff, executable schema, and paired external
  static/host authoring fixtures;
- all seven registered prechecks and the expected absent-source completion
  boundary;
- `tools/check-design-artifacts.sh`;
- `cargo test --workspace --locked`;
- `sh scripts/check-feature-matrix.sh` — 21 passed, 0 failed; and
- `git diff --check
  525bb31b42efe299ed36d46acea1a1c4286e8bde..4a01b5010729cb42d6e8d51125103c8b5cda8707`.

The review additionally mutation-tested and rejected host payload
compatibility mismatch, host payload concrete-type mismatch, host cursor
concrete-type mismatch, artifact footprint overflow, static enum variant
mismatch, premature product-source creation, and premature cross-package
architecture-fixture creation. No finding intersects the tranche.

Review attestation
`8a7aa198f5c983be8fbf5ef1a9750c90b5837703` is the single child of the
candidate, changes exactly the registered three review paths, and passes the
aggregate design and default workspace baselines.

The subsequent original four-file admission-ready simulation intentionally
failed at `v5-authority-reset-candidate-check`: the gate changed while its
carried digest/disposition did not. That failure is the evidence for the
first seven-file correction candidate and corrected five-file pre-source
boundary.

Independent review of exact first correction
`f453f165c2ea775e5f0d10c36f1e419fcc1d79f3` on 2026-07-29 passed its full
seven-path inspection, `--candidate`, aggregate design checks, default
workspace tests, the 21-cell feature matrix, and diff hygiene. Its required
pre-source mutations then proved:

- omitting `docs/spec/v5-artifact-carry-forward.toml` is rejected as expected;
- the exact five-file checkpoint passes the carried-digest check; but
- that checkpoint is then incorrectly rejected because the `in-progress`
  status requires absent `core/src/binding_compiler.rs`.

Independent v2 review of exact second correction
`d2dcf2e9d2e19c7c2dfa234f96c5303cc3aee24a` on 2026-07-29 passed:

- exact single-parent and six-path topology inspection;
- unchanged semantic specification, public authoring fixtures, work-package
  contract, and registered implementation scope relative to the v1 candidate;
- `tools/check-wp200-property-read-plan-slice-entry.sh --candidate`;
- `tools/check-design-artifacts.sh`;
- `cargo test --workspace --locked`;
- `sh scripts/check-feature-matrix.sh` — 21 passed, 0 failed; and
- `git diff --check
  f453f165c2ea775e5f0d10c36f1e419fcc1d79f3..d2dcf2e9d2e19c7c2dfa234f96c5303cc3aee24a`.

The review also executed the exact next-state transition: the five-file
pre-source checkpoint passed `--admission-ready`; omitting the carry-forward
manifest and introducing an out-of-scope implementation path were both
rejected. Those transition simulations were temporary review mutations and
are not repository evidence commits.

The real five-file pre-source checkpoint from attestation
`4f3bdeff604e30eecfbba9c8c12e6dd0b23cc87f` also passes
`tools/check-wp200-property-read-plan-slice-entry.sh --admission-ready` with no
product source present. Its gate and carried SHA-256 change atomically.

Exact implementation `b889ae1dafa65ee66bfb331bebf9d537e1c29eee` passed on
2026-07-29:

- `tools/check-wp200-property-read-plan-slice.sh`, including Core/Planning
  no-default, async/no-std, and std profiles, both external authoring fixtures,
  and predecessor regressions;
- `tools/check-design-artifacts.sh`;
- `cargo test --workspace --locked`;
- `sh scripts/check-feature-matrix.sh` — 21 passed, 0 failed; and
- exact committed path/topology inspection and diff hygiene.

The shared-target root defect described above caused one false aggregate-check
failure before the design-check package artifact was rebuilt from the current
worktree. The clean rebuild, runtime-root correction, 22 design-check unit
tests, deleted-build-worktree cache regression, and aggregate rerun pass; no
product source changed during that diagnosis.

Author-side WP-300 candidate validation on 2026-07-30 passes:

- the five executable registration, route, readiness, response-ownership,
  cleanup-transfer, and host-erasure schema tests;
- `tools/check-wp300-property-read-binding-slice-entry.sh --candidate`,
  including all eight registered prechecks and the exact absent-source
  completion boundary;
- `tools/check-design-artifacts.sh`;
- `cargo test --workspace --locked`;
- `sh scripts/check-feature-matrix.sh` — 21 passed and 0 failed; and
- exact single-parent/20-path topology plus diff hygiene.

Independent root-session review of exact candidate
`e31b975b329fa147bbccf71e5bc6be4254902d89` on 2026-07-30 passed:

- exact single-parent/20-path reconstruction and diff hygiene;
- `tools/check-wp300-property-read-binding-slice-entry.sh --candidate`,
  including all eight registered prechecks, five lifecycle-schema tests, and
  the expected absent-source completion boundary;
- mutation rejection for compiler/server compatibility mismatch, zero-budget
  progress, response-input loss, permit lifetime escape, rejected cleanup
  handoff, premature product source, and premature cross-package fixture roots;
- exact two-path review-attestation topology;
- an exact five-file combined pre-source simulation that passed
  `--admission-ready`;
- an exact `core/src/binding.rs` plus `core/src/lib.rs` implementation-child
  topology accepted by the work-package checker; and
- a simulated passed completion record rejected by the real external
  authoring compile contract while the implementation was incomplete.

The transition and mutation changes existed only in an isolated review
worktree. No simulated product source, admission state, or completion evidence
was retained. The aggregate design-artifact suite, locked workspace tests, and
21-cell feature matrix also pass on the review branch.

Fresh-session reconciliation on default-branch checkpoint
`2250d1e7ef1b2a65b52edceabce312e344682374` reproduced the issue-0040 defect:
`check-state` passed, while `check-work-packages` failed because it treated the
three-parent current merge tip as the review-attestation commit. Inspection
confirmed the immutable attestation is instead the single-parent commit
`d5169ba34ad846b2d45d0841b5d57210ee4df0c1`. The correction's 22
design-check unit tests and 13 schema/integration tests pass; aggregate and
default-branch packet validation also passes:

- `tools/check-wp300-property-read-binding-slice-entry.sh --candidate`,
  including all registered prechecks and the expected absent-source boundary;
- `tools/check-design-artifacts.sh`, including the corrected registered
  work-package check;
- `cargo test --workspace --locked`;
- `sh scripts/check-feature-matrix.sh` — 21 passed, 0 failed;
- `rustfmt --edition 2024 --check tools/design-check/src/main.rs`;
- `bash -n tools/check-wp300-property-read-binding-slice-entry.sh`; and
- `git diff --check`.

An isolated worktree at correction commit
`08f7c6b5498b875ffce8d8e76367bd9deb6a26e1` then changed exactly the five
registered pre-source paths, bound that commit as `admission_base_ref`, updated
the carried gate digest, and passed
`tools/check-wp300-property-read-binding-slice-entry.sh --admission-ready`.
The checker reported `implementation admission ready`; the simulated state was
discarded and no product source was created.

Remote reconciliation then proved pull request #3's actual base, exact content
head, merge reachability from `master`, and successful exact-head workflow run
`30524546209`. Independent review of that correction on 2026-07-30 passed:

- full correction-diff and topology inspection;
- the exact five-file `--admission-ready` simulation, with gate SHA-256
  `a679bd3ac055d519740d0771603bd12382181f64e4c00e72343413511447b00a`;
- the exact two-path implementation topology simulation;
- rejection of an incomplete passed-completion claim;
- `tools/check-wp300-property-read-binding-slice-entry.sh --candidate`;
- `tools/check-design-artifacts.sh`;
- `cargo test --workspace --locked`;
- `sh scripts/check-feature-matrix.sh` — 21 passed, 0 failed; and
- diff hygiene.

Local checkpoint `4ae812f` contains the resulting registered audit and
continuation correction. Pull request #13 exact-head `mainline` run
`30693434071` passed; merge commit
`ff86c1582138d9a231dd44068b35cf6464214cd5` has first parent
`28d717c48a9b6598a93ae09f88503a695392400e`, exact reviewed second parent
`4ae812fd603820c48f5be03264cb2a3f2a39e3e0`, an equal merge/review tree, and
passed default-branch `mainline` run `30697858866`.

The 24-path workspace-topic 0042-0048 decision/migration packet changes no
product source. On 2026-07-30 it passes:

- `tools/check-design-artifacts.sh`, including revalidated state-machine and
  work-package carry-forward digests;
- `tools/check-wp300-property-read-binding-slice-entry.sh --candidate`, proving
  the broad maturity decisions do not reopen the narrow candidate;
- `cargo test --workspace --locked`;
- `sh scripts/check-feature-matrix.sh` — 21 passed, 0 failed; and
- `git diff --check`.

Pull request #12 exact head
`8a9c5395cd0b70412a4b1c67f6652a1811a1b19e` passed remote `mainline` run
`30698464368`. Merge commit
`384461ba33a3639a0f89978159719aa6937e1f36` has first parent
`ff86c1582138d9a231dd44068b35cf6464214cd5`, exact second parent
`8a9c5395cd0b70412a4b1c67f6652a1811a1b19e`, an equal merge/review tree,
and passed default-branch `mainline` run `30698804288`.

The 2026-08-01 continuation-only remote reconciliation update passes
`tools/check-design-artifacts.sh`,
`tools/check-wp300-property-read-binding-slice-entry.sh --candidate`,
`cargo test --workspace --locked`, the 21-cell supported feature matrix, and
diff hygiene.

The pull-request #12 merge resolution against validated default checkpoint
`ff86c1582138d9a231dd44068b35cf6464214cd5` passes the same aggregate design,
WP-300 candidate, locked workspace, 21-cell feature-matrix, and diff-hygiene
suite before its merge commit.

Exact WP-300 source topology on 2026-08-01 is:

`384461ba33a3639a0f89978159719aa6937e1f36`
`-> 6aaf1ef1586428152e535b9cd75f7183ec764cd0`
`-> 89fb9f17ac961294032123173b29692a719a174c`.

The middle checkpoint passes
`tools/check-wp300-property-read-binding-slice-entry.sh --admission-ready` and
changes exactly five registered paths. The implementation changes exactly the
two registered Core paths and passes
`tools/check-wp300-property-read-binding-slice.sh`, including no-default,
async/no-std, and std external fixtures, std host execution, lifecycle and
resource checks, Core/Servient/protocol regressions, and WP-100/WP-200
predecessor checks. With completion records present, the tree also passes
`tools/check-design-artifacts.sh`, `cargo test --workspace --locked`,
`sh scripts/check-feature-matrix.sh` (21 passed, 0 failed), and diff hygiene.
Completion commit `c5b59d60c237b275e0121092006bd9f5e1f3c54d`, exact-head run
`30701146704`, pull request #14, merge
`b2adf0756c06cc41be5d809c33211d7c20f86aba`, and merge-revision run
`30701503019` close the remote integration boundary.

The static and host compile contracts deliberately do not compile against
product source before admission; the completion checker owns their three-cell
compile/runtime validation after the exact two-path implementation child.

Independent root-session review of corrective candidate
`376ee84f80ea27c8d3faa4b1840ce7b68d61f23f` on 2026-08-02 passed:

- exact single-parent/eight-path topology and all eight registered prechecks;
- the corrected external contract for borrowed static and host projections,
  real public Core imports, opaque cursor state, and lexical borrow release;
- an isolated exact two-path Planning source simulation through the full
  completion checker, including the real plan-output-to-`PrepareInput`
  runtime handoff in all three feature cells; and
- negative mutation rejection for Consumer-role substitution, public cursor
  state, missing borrowed host/static support, zero-budget progress,
  caller-restated identity, arbitrary public role selection, fixture-owned
  preparation, out-of-scope source, and premature WP-400 fixture creation.

The review simulation and mutations were isolated and discarded. Review
checkpoint `56ee6b990373c12b83aac26a0377e3489fbde194` contains no product
source; the later exact admitted implementation is separately bound by its
completion evidence.

The exact admitted implementation and completion tree also pass the registered
three-cell Producer-route checker and `check-work-packages`. The latter now
uses the dedicated Producer-route evidence parser; a temporary evidence
mutation from `work_package = "WP-200"` to `"WP-100"` fails with the expected
ownership mismatch. No mutation remains in the worktree.

The completed 2026-08-02 worktree additionally passes:

- `tools/check-design-artifacts.sh`;
- `cargo test --workspace --locked`;
- `sh scripts/check-feature-matrix.sh` — 21 passed, 0 failed;
- `rustfmt --edition 2024 --check` on the two Planning implementation paths
  and `tools/design-check/src/main.rs`;
- shell syntax checks for the registered design, entry, completion, and
  feature-matrix scripts; and
- `git diff --check`.

Workspace-wide `cargo fmt --all -- --check` remains blocked by pre-existing,
out-of-scope formatting differences in `core/src/handler.rs` and existing
Zenoh source. The three Rust paths owned by this task are clean, and none of
the out-of-scope files is modified by this branch.

The intentionally invalid all-features combination enables mutually exclusive
Zenoh backends. Use `scripts/check-feature-matrix.sh`, not
`cargo test --all-features`, as the supported feature baseline.

The 2026-08-02 workspace-topic 0050/0051 decision packet changes no product
source. Its current tree passes:

- `tools/check-design-artifacts.sh`, including the new merge-stable
  continuation envelope and local observed-basis ancestry check;
- an isolated negative mutation that replaces the envelope with the former
  unconditional `Current Objective` heading and is rejected before the
  aggregate suite runs;
- `cargo test --workspace --locked`;
- `sh scripts/check-feature-matrix.sh` — 21 passed, 0 failed;
- `bash -n tools/check-design-artifacts.sh`; and
- `git diff --check`.

## Next Safe Actions

1. Commit and push only the locally validated issue-0053 decision scope from
   `agent/decide-wp400-first-entry-closure`, open one draft pull request against
   `master`, and verify exact-head remote `validation`. Keep it draft unless
   every registered terminal integration predicate is current.
2. After that pull request is verified on fetched `master`, reconcile draft
   pull request #18 onto the new default while preserving its exact
   route-reservation scope. Rerun its candidate checks and obtain independent
   review; do not infer correctness from the issue-0053 decision.
3. Only after the route-reservation correction is reviewed, implemented,
   integrated, and merge-revision validated may WP-400 construct its exact
   candidate. That candidate's normal independent review must execute D46's
   complete first-entry closure and negative mutations before pre-source
   admission.
4. Correct the performance harness's compile-time repository-root capture in
   a separate support-tool task; until then, use a clean Cargo target when a
   shared target may contain binaries compiled from deleted worktrees.

Ask the Project Owner only if the investigation reaches a product-goal,
real-world constraint, unacceptable direction, or irreversible external
commitment that repository evidence cannot resolve.

## Primary Continuation References

- `AGENTS.md`
- `PROJECT_GOVERNANCE.md`
- `ARCHITECTURE_GOVERNANCE.md`
- `PLAN.md`
- `docs/design.md`
- `docs/ADRs/0013-work-package-scoped-implementation-admission.org`
- `docs/ADRs/0018-bounded-v5-normative-authority-reset.org`
- `docs/spec/v5-authority-reset.toml`
- `docs/audits/D7-v5-authority-reset-candidate.toml`
- `docs/audits/D7-v5-authority-reset-review.toml`
- `docs/work-packages/PROPERTY-READ-ARCHITECTURE.md`
- `docs/work-packages/WP-400-servient.md`
- `docs/work-packages/property-read-architecture-gate.toml`
- `workspace/0053-wp400-first-entry-input-closure-audit.md`
- `workspace/0014-property-read-plan-artifact-boundary.md`
- `workspace/0016-post-reset-implementation-throughput.md`
- `workspace/0025-wp300-property-read-slice-scope.md`
- `workspace/0026-wp300-critical-path-concentration.md`
- `workspace/0027-legacy-migration-boundary-leakage.md`
- `workspace/0028-global-gates-and-scoped-implementation.md`
- `workspace/0029-wp300-admission-machinery-regression.md`
- `workspace/0030-automatic-pull-request-integration.md` through
  `workspace/0041-target-legacy-coexistence-verification.md`
- `workspace/0042-protocol-binding-spi-validation-risk.md` through
  `workspace/0048-repository-truth-reachability-and-state-projection-drift.md`
- `workspace/0049-property-read-producer-route-planning-gap.md`
- `workspace/0050-tranche-release-and-successor-entry-evidence.md`
- `workspace/0051-project-state-merge-transition-drift.md`
- `docs/spec/planning.md`
- `docs/spec/binding-spi.md`
- `docs/work-packages/property-read-architecture-gate.toml`
- `docs/work-packages/WP-200-planning.md`
- `docs/work-packages/WP-300-bindings.md`
- `docs/audits/WP-300-property-read-binding-slice-entry.md`
- `docs/audits/WP-200-property-read-plan-slice-entry.md`
- `docs/audits/WP-200-property-read-plan-slice-review.toml`
- `docs/audits/WP-200-property-read-plan-slice-review-v2.toml`
- `docs/audits/WP-200-property-read-producer-route-entry.md`
- `docs/evidence/WP-200-property-read-plan-slice.toml`
- `docs/evidence/WP-300-property-read-binding-slice.toml`
- `tools/design-check/tests/wp200_binding_artifact_schema.rs`
- `tools/design-check/tests/wp300_property_read_binding_schema.rs`
- `tools/design-check/tests/wp200_property_read_producer_route_schema.rs`
