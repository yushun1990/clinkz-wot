# WP-100 Handler Context Entry Audit

Status: Pending

Design revision: v4.9

Admission scope: `WP-100-HANDLER-CONTEXT`

Verdict: Independent re-review pending

The frozen handler amendment defines one call-lifetime dispatch-identity view,
but broad handler entry still combines unrelated request storage, target
representation, portable traits, host erasure, storage, cancellation, and
runtime ownership. This audit freezes the smallest dependency-complete
cross-module handler boundary for independent review.

## Scope

This tranche adds exactly one public Core value:

- `HandlerContext<'a>`.

Permitted implementation paths are exactly:

- `core/src/handler.rs`; and
- `core/src/lib.rs`.

The affected requirements are exactly:

- `API-SURFACE-001`; and
- `HANDLER-API-001`.

The completion key is exactly `handler-context`.

`HandlerContext` composes the already implemented `ThingId`, `ThingSlotId`,
`AffordanceTarget`, `PlanId`, `BindingId`, and `BindingGeneration` values plus
the TD-owned `Operation` vocabulary. It does not replace any of those values.

## Why this is a separate tranche

Repository impact inspection found three different evidence boundaries:

- `HandlerContext` is an additive borrowed identity view whose only behavior is
  the fixed operation/target compatibility check;
- `AffordanceTarget` replacement owns the separate incapable-target and
  no-atomic proof, which currently intersects unrelated legacy Core `Arc`
  surfaces; and
- `AcceptHint` plus `InteractionInput` own resource admission and a coordinated
  migration of existing Core, Servient, and protocol-binding construction
  sites.

Those parts have different blockers, implementation paths, validation
independence, and evidence truth. Splitting `HandlerContext` is therefore based
on the project tranche rule rather than on the fact that it is a separately
named type. The target/no-atomic and request-storage scopes remain unadmitted.

## Frozen public contract

The exact value is:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HandlerContext<'a> {
    thing_id: &'a ThingId,
    thing_slot: ThingSlotId,
    target: &'a AffordanceTarget,
    operation: clinkz_wot_td::data_type::Operation,
    plan_id: PlanId,
    binding: Option<(BindingId, BindingGeneration)>,
}
```

Its only constructor is `HandlerContext::try_new`, with the exact argument
order and `CoreResult<Self>` result frozen by
`docs/amendments/WP-100-handler-api-v1.md`. Its six getters are `const`, take
`self`, and return the exact borrowed or copyable field values.

The type implements exactly `Clone`, `Copy`, `Debug`, `Eq`, and `PartialEq`.
It does not implement `Default`, `Hash`, ordering, allocation, owned request
storage, or a live-identity proof. Fields remain private.

## Compatibility and failure behavior

`try_new` accepts exactly:

- `ReadProperty`, `WriteProperty`, `ObserveProperty`, and
  `UnobserveProperty` for `AffordanceTarget::Property`;
- `InvokeAction`, `QueryAction`, and `CancelAction` for
  `AffordanceTarget::Action`;
- `SubscribeEvent` and `UnsubscribeEvent` for
  `AffordanceTarget::Event`; and
- all nine Thing/collection operations for `AffordanceTarget::Thing`.

Every other operation/target pairing returns `CoreError::Validation` with
`ErrorPhase::Validate` and `RetryClass::Never`. The error context carries the
Thing slot, operation, plan id, and optional binding identity supplied to the
constructor. It does not copy `ThingId` or an affordance name into the error.

The constructor validates compatibility only. It does not prove that an id,
slot, generation, plan, binding, Thing, or affordance is live or admitted.

## Exact exclusions

This tranche does not:

- replace, relocate, or otherwise change `AffordanceKind` or
  `AffordanceTarget`;
- claim the incapable-target or `affordance-target-no-atomics` evidence;
- change `AcceptHint`, `InteractionInput`, `InteractionOptions`, payload,
  principal, correlation, deadline, cancellation, action, or subscription
  ownership;
- add any of the 54 sync, async, or step handler traits;
- add host erasure, registration ingress, sparse storage, replacement,
  execution owners, callback invocation, or cancellation reducers;
- add Producer, Servient, Protocol Binding, planner, scheduler, queue, lock, or
  state-machine behavior;
- remove or rename an existing API; or
- claim a handler performance workload.

Discovering a required source change outside the two registered implementation
paths revokes admission pending impact review.

## Dependency and ownership verdict

The direct semantic predecessor is `WP-100-FOUNDATION-REFRESH`, which supplies
the generation-bearing identifier domain consumed by the context. The current
candidate is also a Git descendant of all four completed WP-100 tranches and
reruns their completion checks because its checker update must not weaken
their evidence.

Core owns the protocol-neutral handler call boundary. Servient still owns
application-facing setter and orchestration APIs; bindings still own protocol
adaptation and I/O. A borrowed identity view moves neither responsibility.

## Risk, resources, and performance

This is Category B implementation risk. The change is locally implemented but
projects a public handler contract and validation boundary across Core and TD
types. It therefore receives an explicit tranche record, ownership and
dependency review, an external conformance fixture, impact analysis, and an
independent admission review.

Construction and getters are fixed-size and allocation-free. The value owns no
variable storage, resource reservation, queue entry, callback, lock, or
retained lifecycle state. No resource-schema or performance-workload change
applies.

## Contract fixture

The nested fixture under
`tools/compile-contracts/wp100-handler-context/` consumes the real Core,
Foundation, and TD surfaces. It freezes:

- the root public path and the exact borrowed/copyable method signatures;
- all 18 valid operation/target pairings and representative invalid
  cross-kind pairings;
- the exact validation category, phase, retry class, and fixed-size context;
- private fields;
- absence of `Hash` and `Default`;
- all three Core feature cells; and
- downstream Servient and Protocol Binding compilation.

The Core source validator freezes the exact attributes, private fields,
generic lifetime, constructor, and getter signatures. The existing
handler-value validator is narrowed to its five owned declarations while still
rejecting unknown handler-module items; its completion checker is rerun as a
predecessor regression.

## Authoritative artifacts

- `docs/ADRs/0013-work-package-scoped-implementation-admission.org`
- `docs/ADRs/0014-transitional-normative-ownership.org`
- `docs/amendments/WP-100-handler-api-v1.md`
- `docs/api-ownership.csv`
- `docs/design.md`
- `docs/requirements.csv`
- `docs/work-packages/WP-100-core.md`
- `docs/work-packages/index.toml`

## Candidate and independent review

The work-package index owns the exact candidate base, candidate commit, changed
path set, implementation paths, contract artifacts, prechecks, audit, entry
check, and completion key. The candidate commit must be a single child of its
registered base and must not change either implementation path.

The independent review must inspect the registered commit rather than an
uncommitted worktree. A later root continuation session that did not author
the candidate records
`reviewer_attestation_kind = "independent-root-session"` and
`reviewer_id = "codex-agent:/root"`. A separately spawned reviewer records
`reviewer_attestation_kind = "separate-agent-task"` and its real canonical
child task id. The attestation commit is limited to its TOML record and the
artifact-registry row.

## Pre-implementation checks

The candidate entry check reruns:

- `api-ownership-check`;
- `architecture-adr-check`;
- `design-requirement-check`;
- `resource-profile-check`;
- `work-package-dag-check`;
- `wp100-amendment-check`;
- `wp100-handler-amendment-check`;
- `wp100-foundation-refresh-check`;
- `wp100-handler-value-primitives-check`;
- `wp100-logical-time-correction-check`; and
- `wp100-deadline-cleanup-timing-check`.

Before implementation, the completion checker must fail only because the real
Core `HandlerContext` declaration is absent.

## Completion evidence

Before this tranche becomes complete:

- its independent review attestation and exact three-file approval checkpoint
  must exist;
- the exact two-file progress checkpoint must precede source changes;
- `tools/check-wp100-handler-context.sh` must pass;
- `docs/evidence/WP-100-handler-context.toml` must record passed
  `handler-context` evidence and the exact implementation commit;
- the implementation commit must change only the two registered Core source
  paths;
- the exact source projection, three feature cells, semantic fixture, private
  field and negative-trait fixtures, Core unit tests, downstream compilation,
  and handler-value predecessor regression must pass; and
- `PROJECT_STATE.md` must name the next remaining handler boundary.

This candidate does not admit broad `WP-100-HANDLER-ENTRY`.
