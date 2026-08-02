# WP-200 Property-Read Producer-Route Projection Entry Audit

Status: Passed

Design revision: v5.0

Admission scope: `WP-200-PROPERTY-READ-PRODUCER-ROUTE-PROJECTION`

Verdict: Implementation-ready

## Finding and exact scope

The completed Property Read planner is externally unreachable and hard-codes
`BindingArtifactRole::ConsumerCall`. The completed narrow WP-300 bundle is a
Producer Property Read server registration whose route preparation consumes a
matching `BindingArtifactRef`. Neither package-local completion fixture joins
those facts, and the WP-400 architecture fixture is forbidden from replacing
the plan or artifact with fixture-owned values.

The completed WP-200 Consumer-call behavior and WP-300 registration/lifecycle
behavior remain valid. This candidate adds a distinct corrective tranche
rather than rewriting either completion claim.

## Public API boundary

The proposed public additions are exactly:

```rust
pub struct PropertyReadPlanCompiler { /* private fields */ }

impl PropertyReadPlanCompiler {
    pub const fn producer_route(
        plan_id: PlanId,
        registration: BindingRegistrationIdentity,
        registration_index: u32,
        candidate_order: u32,
    ) -> Self;
}

pub struct PropertyReadBuildCursor<C, A> { /* private state */ }
```

`PropertyReadPlanCompiler` implements the existing
`PlanCompiler<[R]>` contract for the already supported typed static and
Core-erased host compiler projections, including borrowed projections from a
complete registration. The associated cursor is public only so an external
caller can retain and resume bounded progress; its state and constructors stay
private.

The public constructor fixes the artifact role to `ProducerRoute`. It copies
binding id, binding generation, configuration digest, and compatibility from
one `BindingRegistrationIdentity`. There is no arbitrary-role constructor and
no new compiler, artifact, registration, server, or erasure trait.

## Implementation topology

After independent review and a combined pre-source admission checkpoint, the
only permitted product implementation paths are:

- `planning/src/lib.rs`;
- `planning/src/property_read.rs`.

The first path re-exports the two reviewed types. The second reuses the
existing Property Read selection/build algorithm, carries the selected role
through every compiler input and final artifact identity, keeps the cursor
opaque, and accepts borrowed complete-registration compiler projections.

Any product change outside those two paths revokes admission pending an
intersecting impact review.

## Executable contract

`tools/compile-contracts/wp200-property-read-producer-route/` is outside the
workspace member list. It consumes:

- the real TD builder and Planning `PlanCompiler` contract;
- the real WP-300 static complete registration fixture and both the static and
  host complete registrations' borrowed compiler projections;
- the real `BindingArtifactEnvelope`, `BindingArtifactRef`,
  `BindingRouteKey`, and `PrepareInput`; and
- the real WP-300 `PollServerBinding::start_prepare` boundary.

It must pass no-default, async-no-std, and std compilation and a std runtime
test. The runtime assertion drops the TD and compiler-projection borrow before
route preparation and proves:

1. zero `BindingPolls` budget returns pending without consuming the cursor;
2. the compiler input, envelope identity, and compact reference all carry
   `ProducerRoute`;
3. the artifact identity matches the complete registration and plan-set
   generations; and
4. the resulting reference constructs `PrepareInput` and starts the real mock
   server's route preparation.

The planning scope is lexical: the TD, plan input, and borrowed registration
compiler projection leave scope before the mutable server borrow begins. A
`compile_fail` doctest rejects readable/forgeable public cursor state. The
fixture constructs the real public `PrepareInput`; the forbidden case is a
fixture-owned substitute that would sever the production type handoff.

`tools/design-check/tests/wp200_property_read_producer_route_schema.rs`
provides a pre-source executable model and negative Consumer-call mutation.
The completion check must currently fail exactly because the reviewed public
Producer-route planner marker is absent.

## Exact exclusions

This correction does not:

- expose a general artifact-role selector;
- make the existing Consumer-call constructor public;
- implement Consumer subscription or Producer publication planning;
- add fallback, lazy compilation, capability-index, or multi-plan behavior;
- change Core, TD, WP-300 registration, or binding lifecycle source;
- add Servient registry, publication, selection, dispatch, or cleanup source;
- create either Property Read architecture fixture root;
- implement a concrete protocol or claim Zenoh behavior; or
- complete WP-400 or the aggregate Property Read architecture gate.

## Candidate and review topology

The immutable original candidate base is fetched and validated default-branch
merge `b2adf0756c06cc41be5d809c33211d7c20f86aba`, which integrated WP-300
through pull request #14 and passed default-branch validation run
`30701503019`. Exact 22-path single child
`613ee18d11b8f60e93d0792fcc76b83a00569044` is the original semantic
candidate.

Independent review on 2026-08-02 rejected that original candidate's evidence,
not its bounded public API direction. A next-state source simulation found:

- the fixture imported `BindingRouteKey` from a Core root that does not
  re-export it;
- registration construction used `.expect(...)` even though the preserved
  rejected input is intentionally not `Debug`;
- only the borrowed static compiler projection was compiled, so omission of
  the borrowed host projection remained green;
- cursor opacity had no negative compile contract; and
- dropping a `Copy` array of borrowed compiler references did not prove the
  borrow ended before mutable server preparation.

The corrective candidate is the exact eight-path single child of
`613ee18d11b8f60e93d0792fcc76b83a00569044`:

- `PLAN.md`;
- `PROJECT_STATE.md`;
- this audit;
- `docs/spec/v5-artifact-carry-forward.toml`;
- `docs/work-packages/property-read-architecture-gate.toml`;
- `tools/check-wp200-property-read-producer-route-entry.sh`;
- `tools/compile-contracts/wp200-property-read-producer-route/src/lib.rs`; and
- `tools/design-check/src/main.rs`.

It changes no product source, public API proposal, implementation owner, or
future two-path source boundary. The gate uses
`candidate_ref = "resolved-by-review-attestation"`; independent review records
the immutable corrective candidate object id in
`docs/audits/WP-200-property-read-producer-route-review.toml`.

Candidate validation may run with `HEAD` at either the exact corrective commit
or a tree-equivalent two-parent execution wrapper whose first parent is the
registered corrective base. Once an attestation exists, all later checks
resolve the reviewed correction from its `reviewed_ref` instead of
reclassifying a merge or continuation head as the candidate.

The reviewer must reconstruct both immutable candidates, inspect the exact
diffs, run every precheck and candidate check, execute the corrected next-state
source simulation, and mutation-test at least:

- `ConsumerCall` substituted for `ProducerRoute`;
- caller-restated registration identity in place of the complete identity;
- public arbitrary-role construction;
- a public/forgeable cursor state;
- omission of the borrowed static or host compiler projection;
- fixture-created logical plan, artifact, or preparation input substitute;
- progress under zero budget;
- any product-source path outside the exact two-path topology; and
- premature WP-400 or architecture-fixture source.

The review attestation checkpoint changes exactly the attestation, its artifact
registry row, and `PROJECT_STATE.md`. It may report `passed` only after the
corrected compile/runtime contract and all declared mutations pass.

Independent root-session review passed corrective candidate
`376ee84f80ea27c8d3faa4b1840ce7b68d61f23f`. Exact review checkpoint
`56ee6b990373c12b83aac26a0377e3489fbde194` changes only the registered three
review paths and records all eight prechecks as passed. The isolated next-state
simulation completed the real handoff, and every declared negative mutation
failed closed.

## Admission and completion

This combined pre-source checkpoint changes only `PLAN.md`, `PROJECT_STATE.md`,
this audit,
`docs/spec/v5-artifact-carry-forward.toml`, and
`docs/work-packages/property-read-architecture-gate.toml`. It is based on exact
current-default reconciliation merge
`2a469b1d4a3579d4db6115351b76867a0a531db8`, changes the
tranche from `pending`/`review-pending` to
`in-progress`/`approved`, records the immutable review ref, and binds
`admission_base_ref` to that exact reviewed basis.

The immediate implementation child must change exactly the two Planning
paths. Completion requires the correction checker, all three feature cells,
the real cross-slice compile/runtime fixture, both predecessor completion
checks, aggregate design/workspace/feature validation, exact completion
evidence, and no Property Read architecture fixture root.
