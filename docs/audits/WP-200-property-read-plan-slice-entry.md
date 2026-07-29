# WP-200 Property-Read Plan Slice Entry Audit

Status: Review pending

Design revision: v5.0

Admission scope: `WP-200-PROPERTY-READ-PLAN-SLICE`

Verdict: Candidate ready for independent review

Independent root review at
`8a7aa198f5c983be8fbf5ef1a9750c90b5837703` confirmed exact candidate
`4a01b5010729cb42d6e8d51125103c8b5cda8707`, all seven prechecks, the
executable schema, both external authoring forms, and the expected absent-source
boundary. It also rejected the required host compatibility/type/cursor,
footprint, static-variant, premature-source, and premature-fixture mutations.
That review remains immutable predecessor evidence for the semantic contract.

The subsequent admission-ready simulation found an intersecting evidence-truth
defect: changing the Property Read gate without changing
`docs/spec/v5-artifact-carry-forward.toml` leaves the carried SHA-256 stale, so
the mandatory `v5-authority-reset-candidate-check` rejects the planned
four-file checkpoint. Exact first correction
`f453f165c2ea775e5f0d10c36f1e419fcc1d79f3` changed the future pre-source
boundary from four files to five and made the carried digest atomic with the
gate transition.

Independent review passed that commit's static diff, candidate checks, and
aggregate baselines, then rejected its required exact-five-file mutation. Once
the digest matched, `check-work-packages` required
`core/src/binding_compiler.rs` in the approved `in-progress` pre-source state,
even though the registered topology requires the nine-path implementation
commit to follow that checkpoint. No v2 attestation was created.

The second correction keeps the reviewed API, implementation paths, fixtures,
exclusions, prechecks, and five-file checkpoint unchanged. It preserves the
failed predecessor topology, makes implementation-path presence mandatory only
for `complete`, and leaves the existing exact topology checker responsible for
`in-progress` pre-source and implementation states.

## Decision and exact scope

Workspace issue 0014 is decided and migrated. The tranche uses one portable
associated-type compiler contract, an application-closed static
compiler/cursor/artifact enum, and Core-owned safe host erasure.

WP-200 is the sole implementation owner of the Core compiler/artifact SPI and
the Planning coordination surface. WP-300 later consumes exactly one compiler
component in a complete registration. A compiler component is constructible
and independently testable, but cannot be installed in `ServientBuilder`.

The tranche adds exactly these public items:

- `BindingConfigurationDigest`;
- `PlanSetGeneration`;
- `LogicalInteractionPlan`;
- `BindingCandidate`;
- `BindingArtifactCompatibility`;
- `BindingArtifactRole`;
- `BindingArtifactFootprint`;
- `BindingCompilerBounds`;
- `BindingArtifactIdentity`;
- `BindingCompilerInput`;
- `BindingArtifact<A>`;
- `BindingArtifactEnvelope<A>`;
- `BindingArtifactRef`;
- `BindingArtifactRejectionReason`;
- `BindingArtifactRejection<A>`;
- `BindingCompilerOutput<A>`;
- `BindingCompilerFailure<C>`;
- `BindingCompilerStep<C, A>`;
- `BindingCompilerExtension`;
- `StaticBindingCompilerRegistration<C>`;
- `HostBindingArtifact`;
- `HostBindingCompilerCursor`;
- `HostBindingCompilerRegistration`;
- `PlanBuildInput<'a, R>`;
- `PlanBuildOutput<A>`;
- `PlanBuildFailure<C>`;
- `PlanBuildStep<C, A>`; and
- `PlanCompiler<R>`.

The exact implementation paths are:

- `Cargo.lock`;
- `Cargo.toml`;
- `core/src/binding_compiler.rs`;
- `core/src/identity.rs`;
- `core/src/lib.rs`;
- `core/src/plan.rs`;
- `planning/Cargo.toml`;
- `planning/src/lib.rs`; and
- `planning/src/property_read.rs`.

Any required product change outside those paths revokes admission pending
impact review.

## Portable compiler ownership

`BindingCompilerExtension` has associated `Cursor` and `Artifact` types.
`start` creates pure cursor state. `step` consumes the cursor and returns
exactly `Pending(cursor)`, `Complete(output)`, or `Failed(error, cursor)`.
`abort` consumes pure memory and has no protocol cleanup obligation.

Bounds declare final artifact items/bytes, cursor bytes, temporary bytes, and
typed work. Only `WorkClass::BindingPolls` may be nonzero or charged by compiler
progress. Planning admits those bounds before progress and rejects a measured
artifact that exceeds them.

The artifact identity includes:

- plan-set generation;
- logical plan id;
- binding id and generation;
- binding configuration digest;
- compiler/artifact compatibility identity; and
- Consumer-call, Consumer-subscription, Producer-route, or
  Producer-publication role.

Envelope construction checks compatibility and footprint before execution can
borrow a payload. Rejection returns the original typed artifact.

## Static representation

A constrained third-party binding implements the portable trait with its own
concrete cursor and artifact. The application composes several bindings by
defining:

- one closed compiler enum;
- one matching closed cursor enum; and
- one matching closed artifact enum.

The application enum delegates each trait operation to the selected
third-party compiler. Every entry in its static table can therefore use
`StaticBindingCompilerRegistration<AppCompiler>` without a trait object,
`Any`, `Arc`, allocator-backed erasure, atomics, executor, or unsafe
binding-authored cast.

The static artifact stays typed through
`BindingArtifactEnvelope<AppArtifact>`.

## Host representation

Under `std`, `HostBindingCompilerRegistration::new` accepts a portable compiler
whose compiler, cursor, and artifact satisfy the frozen thread/lifetime bounds.
Core performs safe standard-library type erasure.

Borrowed host payload access checks compatibility and concrete type.
Consuming access returns `Err(original_artifact)` on either mismatch. A cursor
type mismatch returns the original erased cursor through the failed step or
abort result. No binding crate provides an unsafe vtable, raw pointer,
integer-slot convention, or unchecked downcast.

## Property Read planning boundary

`LogicalInteractionPlan::try_property_read` owns its plan id, Thing id,
property name, original form index, resolved target, content type, and
subprotocol. It returns `Operation::ReadProperty` and contains no TD, form,
source envelope, or input lifetime.

`PlanBuildInput<'a, R>` borrows a validated TD, immutable registration
snapshot, and plan-set generation. `PlanBuildOutput<A>` owns bounded vectors of
logical plans, artifact envelopes, and compact references and has no lifetime
parameter.

Completion must prove:

- the planner reads the TD only while the build input is borrowed;
- dropping the TD, registration snapshot, and compiler input leaves the output
  usable;
- a static typed artifact and a host-erased artifact carry the same identity
  and measured footprint;
- execution preparation can consume the compiled mock payload without reading
  a TD or form; and
- zero `BindingPolls` budget returns the unchanged cursor without observable
  progress.

## Exact exclusions

This tranche does not:

- implement `HostBindingRegistration`, `StaticBindingRegistration<B>`,
  `ServientBuilder`, or any independently installable compiler half;
- implement client/server execution, routes, request acceptance, responses,
  subscriptions, emission, form contribution, status, ingress, cleanup, or
  protocol I/O;
- implement broad capability indexes, selection fallback, lazy compilation,
  negative caches, collection operations, or Producer form finalization;
- publish, drain, reclaim, or otherwise own a Servient plan-set record;
- migrate the TD source-envelope or lossless-document model;
- change existing handler, payload, security, request, binding, Servient, or
  concrete-protocol source;
- remove a legacy API or claim a performance workload;
- complete `WP-300-PROPERTY-READ-BINDING-SLICE`,
  `WP-400-PROPERTY-READ-SERVIENT-SLICE`, or the aggregate
  `PROPERTY-READ-ARCHITECTURE` gate; or
- create either planned cross-package architecture fixture root.

## Dependency and ownership verdict

The exact predecessor is the completed
`WP-100-PROPERTY-READ-HANDLER-SLICE`. Its completion check remains a mandatory
precheck.

The Core crate owns protocol-neutral identities, logical plan values, compiler
progress, artifact identity/accounting, safe host erasure, and typed static
registration. The new Planning crate owns the generic build input/output and
Property Read planning algorithm. TD remains a borrowed validated input.
WP-300 owns the later complete registration and execution SPI. No dependency
edge changes.

## Risk and evidence depth

This is Category C work because it introduces public cross-crate ownership,
generation, resource-bound, and host/static representation contracts. It
therefore requires:

- exact candidate/base/path checking;
- independent review of the committed candidate;
- paired external authoring fixtures;
- a self-contained executable Rust schema;
- all three supported Core/Planning feature cells;
- mismatch and footprint rejection tests;
- no-runtime-TD ownership proof;
- a separate admission checkpoint before source; and
- exact implementation-commit and completion-evidence checking.

No runtime state machine, old API removal, or performance workload belongs to
this narrow slice.

## Candidate contract fixtures

`tools/design-check/tests/wp200_binding_artifact_schema.rs` is executable
before product implementation. It models the exact public signatures and
proves:

- consumed-cursor ownership on every step result;
- typed static artifact access;
- application-closed heterogeneous static dispatch;
- safe host cursor/artifact erasure;
- mismatch return of the original owned value;
- compatibility and footprint envelope rejection; and
- an owned plan output with no TD lifetime.

`tools/compile-contracts/wp200-property-read-plan-slice/` is the external
production contract. Its library is `no_std + alloc` and authors the static
third-party compiler and application enums. Its `std` test authors the same
compiler through Core host erasure. It intentionally cannot compile before the
registered Core implementation exists; the entry check requires the completion
checker to stop at that exact absent-source boundary.

Neither fixture is a cross-package architecture fixture and neither grants
runtime or source authority.

## Candidate and independent review

The original semantic candidate is
`4a01b5010729cb42d6e8d51125103c8b5cda8707`, the single child of
`525bb31b42efe299ed36d46acea1a1c4286e8bde`; its v1 attestation is
`8a7aa198f5c983be8fbf5ef1a9750c90b5837703`.

The failed first correction is exact seven-path single child
`f453f165c2ea775e5f0d10c36f1e419fcc1d79f3` of the v1 attestation. The second
corrective candidate base is that failed commit. The gate manifest owns the
new candidate's exact six paths:

- `PLAN.md`;
- `PROJECT_STATE.md`;
- this audit;
- `docs/spec/v5-artifact-carry-forward.toml`;
- `docs/work-packages/property-read-architecture-gate.toml`;
- `tools/design-check/src/main.rs`.

The corrective authoring commit must be the single child of that base and must
not change an implementation path. Before review, the frozen base, exact path
set, and single-child rule identify that commit without a self-referential
hash-registration checkpoint. The independent v2 attestation records the
immutable corrective candidate ref.

The independent reviewer must inspect the failed first correction and the
registered second corrective commit, revalidate the original v1 attestation,
run every precheck and the executable schema, confirm that the
API/fixture/implementation contract is unchanged, and mutation-test at least:

- omitting `docs/spec/v5-artifact-carry-forward.toml` from the pre-source
  boundary is rejected;
- the exact five-file pre-source boundary is accepted with no implementation
  source when its carried digest matches the gate;
- the implementation worktree/commit remains restricted to exactly the nine
  registered paths;
- premature source or architecture-fixture creation remains rejected; and
- the original host/static mismatch, footprint, and ownership rejection
  boundaries remain represented by the frozen v1 evidence.

A later root continuation that did not author this candidate may use
`reviewer_attestation_kind = "independent-root-session"` and
`reviewer_id = "codex-agent:/root"`. A separate child task may use
`reviewer_attestation_kind = "separate-agent-task"` with its real canonical
task id.

The v2 attestation commit changes only
`docs/audits/WP-200-property-read-plan-slice-review-v2.toml`, its artifact
registry row, and `PROJECT_STATE.md`. The continuation update records the
reviewed immutable ref and the remaining combined pre-source checkpoint; it is
not a separate critical-path commit. The original v1 attestation remains
registered predecessor evidence.

## Pre-implementation checks

The candidate entry check reruns exactly:

- `api-ownership-check`;
- `architecture-adr-check`;
- `design-requirement-check`;
- `resource-profile-check`;
- `v5-authority-reset-candidate-check`;
- `work-package-dag-check`; and
- `wp100-property-read-handler-slice-check`.

Before implementation, the completion checker must fail only because
`core/src/binding_compiler.rs` is absent.

## Admission and completion

After a passing independent v2 attestation, one combined approval/in-progress
checkpoint may change only:

- `PLAN.md`;
- `PROJECT_STATE.md`;
- this audit; and
- `docs/spec/v5-artifact-carry-forward.toml`; and
- `docs/work-packages/property-read-architecture-gate.toml`.

That single recoverable checkpoint changes the tranche directly from
`pending`/`review-pending` to `in-progress`/`approved`; no separate approval,
progress, or admission-hash registration checkpoint is required before source.
The carry-forward record must contain the exact digest of the changed gate in
that same checkpoint.

The implementation commit must change exactly the nine registered
implementation paths. Completion requires:

- `tools/check-wp200-property-read-plan-slice.sh` passing;
- no-default, async-no-std, and std Core/Planning checks;
- the external static and host authoring fixtures;
- exact identity, mismatch, footprint, zero-budget, deterministic-step, and
  no-runtime-TD tests;
- the completed handler predecessor regression;
- no planned cross-package architecture fixture root; and
- registered
  `docs/evidence/WP-200-property-read-plan-slice.toml` identifying the exact
  implementation commit and passing `property-read-plan-slice` evidence.

This closes only the package-local Property Read planning slice. The binding,
Servient, and aggregate vertical-integration slices remain separate.
