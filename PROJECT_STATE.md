# Project State

Last updated: 2026-08-01

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
  `b889ae1dafa65ee66bfb331bebf9d537e1c29eee`, the exact nine-path WP-200
  Core/Planning implementation.
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
- The remote default branch was fetched on 2026-08-01 at
  `28d717c48a9b6598a93ae09f88503a695392400e`. It contains the pull-request #3
  correction and workspace topics 0042-0048.
- Independent review of the exact pull-request #3 correction and both
  registered next-state simulations passed locally. Branch
  `agent/review-wp300-admission-basis` records that review at
  `4ae812f` and adds
  `docs/audits/WP-300-admission-basis-correction-review.toml`.
  Draft pull request #13 targets `master` from that exact head; remote
  `mainline` run `30693434071` passed, the branch is mergeable, and no review
  thread is open. The checkpoint is not yet on the remote default branch.
- The workspace-topic 0042-0048 decision/migration checkpoint is
  `b8635059059b9f97aba38f6f44fbb59b1eab33b3`, the single child of the observed
  default checkpoint. Draft pull request #12 targets `master` from branch
  `agent/decide-workspace-topics-0042-0048`; exact-head `mainline` runs
  `30596434779` and `30597175091` passed, and no review thread is open.
- GitHub CLI authentication is valid through the host keyring. The active
  Ruleset `20009352` requires `validation`, but strict current-base checking
  and required review-thread resolution are both disabled. Repository policy
  therefore keeps pull requests #12 and #13 draft and forbids AI-enabled
  automatic integration until those prerequisites are enabled and reverified.

The activation candidate changed exactly 27 documentation/checker paths and no
Rust source, Cargo manifest, public API, or runtime behavior. Its independent
review passed the exact candidate checker, aggregate design/evidence suite,
default workspace tests, diff hygiene, and the 21-cell valid feature matrix.

## Current Objective

Integrate the independent WP-300 admission-basis correction review recorded at
`4ae812f`. Its exact-head remote validation passes in draft pull request #13,
but repository policy prohibits automatic integration while the active remote
Ruleset lacks strict current-base validation and required review-thread
resolution. The dependent five-file pre-source checkpoint must wait for
verified default-branch integration.

The decision/migration packet for workspace topics 0042-0048 is complete and
handed off in draft pull request #12. Its repository-grounded conclusions are
migrated into governance, specifications, architecture, work packages, the
plan, workspace index, and this continuation checkpoint. They strengthen
broad-package and release evidence without reopening the immutable narrow
WP-300 semantic candidate.

Product source remains unadmitted. Pull request #3 corrected the WP-300
admission-basis model on the remote default branch, and an exact independent
review now passes locally at `4ae812f`. That review checkpoint must be handed
off and integrated before the exact five-file combined pre-source checkpoint.
The pre-source checkpoint must bind `admission_base_ref` to the then-current
reviewed default-branch descendant and be its single child. Only its
implementation child may touch `core/src/binding.rs` and `core/src/lib.rs`.
The 0042-0048 migration packet is disjoint preparation and does not count as
WP-300 source admission.

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
absent-`core/src/binding.rs` completion boundary. No WP-300 implementation path
or cross-package Property Read architecture fixture root is admitted. The
semantic candidate review and admission-topology correction are complete. The
correction's independent review is locally recorded but not yet integrated.
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

Workspace issues 0042-0048 are decided and migrated locally. They establish a
six-rung Binding-SPI maturity ladder and honest Zenoh-family claim; a private
Servient owner graph and early cross-shard feedback checkpoint; cleanup
obligation coexistence and observable progress; one shared Host/constrained
kernel and trace oracle with normalized liveness; separation of atomic
publication from the conservative all-route v1 policy plus a bounded explicit
retry-facade contract; separation of the canonical resource schema from stable
authoring projections; and a default-branch reachability/content/validation
predicate for remote truth. Draft pull request #12 is the remaining integration
boundary.

## Active Milestones

- M0 Execution Baseline and Collaboration Reset — CLOSED.
- M1 v5.0 Authority Reset and Architecture Closure — IN_PROGRESS.
- M2 Foundation and Core Contract Stabilization — IN_PROGRESS.
- M3 Planning and Compilation Pipeline — IN_PROGRESS; the exact WP-200
  Property Read plan slice is complete, while broad WP-200 exits remain open.
- M4 Protocol Binding SPI and Lifecycle — OPEN; the exact WP-300 Property Read
  slice dependency, semantic review, and correction are satisfied. The exact
  correction review passes locally but is not integrated; no product source is
  admitted.

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
- narrow WP-300 completion releases only the narrow WP-400 Property Read
  tranche, while broad WP-300 completion releases broad WP-400, WP-500, and
  WP-600;
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

### Focused execution-risk decisions / workspace issues 0017-0048

Status: all thirty-two decisions migrated.

- 0017: the WP-200 admission path has a finite stopping condition. Independent
  review now includes the exact next-state simulation; the five-file
  pre-source transition passes, omission of its carry-forward update fails,
  out-of-scope implementation work fails, and the exact nine-path child now
  has completion evidence.
- 0018: the four Property Read source slices remain necessarily serial because
  each next slice consumes the preceding public ownership/lifecycle boundary.
  Later preparation is allowed but is neither admission nor vertical progress.
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
- 0026: narrow WP-300 completion releases only narrow WP-400 Property Read.
  Broad WP-300 completion remains the necessary release event for broad
  WP-400, WP-500, and WP-600; downstream preparation is allowed but is not
  source admission or vertical progress.
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
- 0031: the shortest feedback path remains narrow WP-300, narrow WP-400, the
  mock Property Read gate, then a real Zenoh smoke and broader claims.
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
implementation ref to the completion checker. The integration gate still has
WP-300 `pending`/`review-pending`, WP-400 planned/blocked, and remains
globally `blocked`.

### WP-300 admission candidate

Status: SEMANTIC REVIEW PASSED; ADMISSION-BASIS CORRECTION INTEGRATED;
CORRECTION REVIEW PASSED LOCALLY BUT NOT INTEGRATED; SOURCE NOT ADMITTED.

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
`4ae812f` records that result. Source admission waits for that checkpoint's
remote integration and a fresh default-base reconciliation.

Future implementation scope remains exactly `core/src/binding.rs` and
`core/src/lib.rs`; existing `core/src/inbound.rs` is not an admitted path.

### Aggregate design-check worktree-root defect

Status: RESOLVED.

Commit `eacaaf1242a41861758ebc78a40ada2d88d15bba` removes the compile-time
repository root. Design-check callers export
`CLINKZ_WOT_REPOSITORY_ROOT`; direct runs may discover the nearest runtime
ancestor; explicit invalid roots fail closed. Three unit tests own precedence,
ancestor discovery, and invalid-root rejection. A real shared-target
regression built the checker in a detached temporary worktree, removed that
worktree, and then executed the cached binary from `/tmp` against the current
root successfully. The temporary worktree and 160.9 MiB test target were
removed.

### Disjoint downstream blockers

- Broad WP-100 handler entry still lacks its remaining request/target
  migration, portable async/step admission, no-atomic public-boundary proof,
  and workload/resource evidence.
- The exact WP-300 Property Read slice has its plan dependency and immutable
  semantic review. Its correction is integrated and independently reviewed
  locally, but the review checkpoint must be handed off and integrated before
  the combined pre-source checkpoint. Broad WP-300 also waits on external
  authoring, cleanup-coexistence, shared-parity-oracle, and resource-authoring
  evidence.
- WP-400, WP-500, and WP-600 depend on WP-300; WP-700 joins those branches.
- Draft pull request #13 has passed exact-head validation but cannot be
  automatically integrated under repository policy because Ruleset `20009352`
  does not require strict current-base validation or review-thread resolution.
  This is an external integration blocker, not an architecture finding, and it
  does not authorize source admission.

These do not extend the D8 packet unless repository evidence shows a direct
contract, rollback, or validation intersection.

## Rejected or Superseded Approaches

- Lossless D3 domain-by-domain authority migration is superseded by ADR-0018.
- Foundation candidate
  `2494f33fdfe49ec3c7ae850d20990e446e628865` remains historical input and must
  not be activated.
- Partial v5 activation or piecemeal rollback is prohibited.
- A separate documentation/review cycle for each artifact in one semantic
  conversion packet is rejected by D9.
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
continuation correction. Draft pull request #13 contains that exact head;
remote `mainline` run `30693434071` passed on 2026-08-01, the branch is
mergeable against `master` at
`28d717c48a9b6598a93ae09f88503a695392400e`, and no review thread is open.

The 24-path workspace-topic 0042-0048 decision/migration packet changes no
product source. On 2026-07-30 it passes:

- `tools/check-design-artifacts.sh`, including revalidated state-machine and
  work-package carry-forward digests;
- `tools/check-wp300-property-read-binding-slice-entry.sh --candidate`, proving
  the broad maturity decisions do not reopen the narrow candidate;
- `cargo test --workspace --locked`;
- `sh scripts/check-feature-matrix.sh` — 21 passed, 0 failed; and
- `git diff --check`.

Draft pull request #12 contains exact migration checkpoint
`b8635059059b9f97aba38f6f44fbb59b1eab33b3`; remote `mainline` runs
`30596434779` and `30597175091` passed, and no review thread is open.

The 2026-08-01 continuation-only remote reconciliation update passes
`tools/check-design-artifacts.sh`,
`tools/check-wp300-property-read-binding-slice-entry.sh --candidate`,
`cargo test --workspace --locked`, the 21-cell supported feature matrix, and
diff hygiene.

The static and host compile contracts deliberately do not compile against
product source before admission; the completion checker owns their three-cell
compile/runtime validation after the exact two-path implementation child.

The intentionally invalid all-features combination enables mutually exclusive
Zenoh backends. Use `scripts/check-feature-matrix.sh`, not
`cargo test --all-features`, as the supported feature baseline.

## Next Safe Actions

1. Keep draft pull requests #12 and #13 unmerged while Ruleset `20009352`
   lacks strict current-base validation and required review-thread resolution.
   Changing that external repository policy requires explicit Owner direction.
2. After those prerequisites are enabled and freshly verified, recheck the
   exact head, current-base validation, diff, independent evidence,
   mergeability, reviews, and conversations; then make pull request #13 ready
   and enable native merge-commit auto-merge with expected-head protection.
   Pull request #12 remains an independently integrable disjoint packet.
3. Fetch the resulting default branch and reconcile actual ancestry, expected
   review content, and default-head validation. From that reviewed descendant,
   create the exact five-file combined pre-source checkpoint with
   `admission_base_ref = <that commit>`.
4. Only after the pre-source checkpoint passes `--admission-ready`, implement
   the exact child touching `core/src/binding.rs` and `core/src/lib.rs`.

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
- `docs/spec/planning.md`
- `docs/spec/binding-spi.md`
- `docs/work-packages/property-read-architecture-gate.toml`
- `docs/work-packages/WP-200-planning.md`
- `docs/work-packages/WP-300-bindings.md`
- `docs/audits/WP-300-property-read-binding-slice-entry.md`
- `docs/audits/WP-200-property-read-plan-slice-entry.md`
- `docs/audits/WP-200-property-read-plan-slice-review.toml`
- `docs/audits/WP-200-property-read-plan-slice-review-v2.toml`
- `docs/evidence/WP-200-property-read-plan-slice.toml`
- `tools/design-check/tests/wp200_binding_artifact_schema.rs`
- `tools/design-check/tests/wp300_property_read_binding_schema.rs`
