# WP-200 Property-Read Producer-Route Projection Entry Audit

Status: Review pending

Design revision: v5.0

Admission scope: `WP-200-PROPERTY-READ-PRODUCER-ROUTE-PROJECTION`

Verdict: Candidate ready for independent review

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
- the real WP-300 static complete registration fixture;
- the complete registration's borrowed compiler projection;
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

The immutable candidate base is fetched and validated default-branch merge
`b2adf0756c06cc41be5d809c33211d7c20f86aba`, which integrated WP-300 through
pull request #14 and passed default-branch validation run `30701503019`.

The candidate is the single child of that base and changes exactly the
registered non-product-source candidate paths. Its manifest uses
`candidate_ref = "register-after-candidate-commit"`; independent review must
record the actual immutable candidate object id in
`docs/audits/WP-200-property-read-producer-route-review.toml`.

Candidate validation may run with `HEAD` at either that exact commit or a
GitHub pull-request merge checkout. A merge checkout is only an execution
wrapper: it must have the registered candidate base as first parent, the exact
single-child candidate as second parent, and a tree identical to that
candidate. The checker unwraps and validates the second parent; it never
treats the two-parent wrapper as the reviewed candidate object.

The reviewer must reconstruct the candidate from the base, inspect the exact
diff, run every precheck and candidate check, and mutation-test at least:

- `ConsumerCall` substituted for `ProducerRoute`;
- caller-restated registration identity in place of the complete identity;
- public arbitrary-role construction;
- a public/forgeable cursor state;
- omission of the borrowed static or host compiler projection;
- fixture-created logical plan, artifact, or preparation input substitute;
- progress under zero budget;
- any product-source path outside the exact two-path topology; and
- premature WP-400 or architecture-fixture source.

## Admission and completion

After a passing independent attestation, one combined pre-source checkpoint
may change only `PLAN.md`, `PROJECT_STATE.md`, this audit,
`docs/spec/v5-artifact-carry-forward.toml`, and
`docs/work-packages/property-read-architecture-gate.toml`. It changes the
tranche from `pending`/`review-pending` to
`in-progress`/`approved`, records the immutable review ref, and binds
`admission_base_ref` to the exact reviewed default-branch basis from which the
five-file checkpoint is created.

The immediate implementation child must change exactly the two Planning
paths. Completion requires the correction checker, all three feature cells,
the real cross-slice compile/runtime fixture, both predecessor completion
checks, aggregate design/workspace/feature validation, exact completion
evidence, and no Property Read architecture fixture root.
