# WP-100 Property-Read Handler Slice Entry Audit

Status: Review pending

Design revision: v4.9

Admission scope: `WP-100-PROPERTY-READ-HANDLER-SLICE`

Verdict: Candidate ready for independent review

## Scope

This tranche adds exactly one public Core trait:

- `ReadPropertyHandler`.

Permitted implementation paths are exactly:

- `core/src/handler.rs`; and
- `core/src/lib.rs`.

The affected integration requirements are exactly:

- `HANDLER-API-001`;
- `HANDLER-VALUE-001`;
- `API-PAYLOAD-001`; and
- `CLEANUP-RECORD-001`.

The completion key is exactly `property-read-handler-slice`.

The trait composes four existing production values:

- completed `HandlerContext`;
- completed `StaticHandlerRegistration`;
- existing `InteractionInput`; and
- existing `InteractionOutput`.

It replaces none of those values and changes no handler execution or storage.

## Why this is the exact integration slice

`PROPERTY-READ-ARCHITECTURE` needs one statically registered synchronous
property-read handler before planner, binding, and Servient slices can compose
the first end-to-end scenario. A generic static registration already exists,
and HandlerContext already validates property-read target compatibility. The
only missing WP-100 seam for that scenario is the operation-specific
synchronous trait.

Impact review found that a wider tranche would merge distinct blockers and
evidence boundaries:

- final `InteractionInput` plus `AcceptHint` requires resource admission and a
  coordinated downstream construction-site migration;
- `AffordanceTarget` relocation and the atomic-incapable Core build intersect
  unrelated legacy `Arc` surfaces;
- async and bounded-step traits add cancellation/future/work-budget contracts;
  and
- host erasure, sparse storage, selection, and call ownership require runtime
  state and workload evidence.

`workspace/0012-property-read-handler-slice-boundary.md` migrated the narrow
decision into the architecture-gate manifest and executable checker. The full
no-atomic condition remains a broad WP-100 blocker; placing it on this slice
would make the only broad-entry exemption depend on work broad entry blocks.

## Frozen public contract

The exact trait is:

```rust
pub trait ReadPropertyHandler {
    fn handle(
        &self,
        context: HandlerContext<'_>,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput>;
}
```

The trait:

- is root-re-exported as `clinkz_wot_core::ReadPropertyHandler`;
- remains available at
  `clinkz_wot_core::handler::ReadPropertyHandler`;
- is object-safe;
- has no `Send`, `Sync`, `'static`, allocation, executor, runtime, or other
  supertrait;
- takes `&self`, a by-value borrowed `HandlerContext`, and a shared
  `&InteractionInput`;
- returns exactly `CoreResult<InteractionOutput>`; and
- has no associated type, constant, default method body, second method, or
  Core-side implementation, including no blanket implementation.

The generic `StaticHandlerRegistration<'h, H>` remains unchanged. A static
registration applies the `ReadPropertyHandler` bound only where the generated
table or dispatch call consumes it.

## Exact exclusions

This tranche does not:

- change `InteractionInput`, `InteractionOutput`, `AcceptHint`, payload,
  principal, correlation, deadline, cancellation, action, or subscription
  ownership;
- relocate or replace `AffordanceKind` or `AffordanceTarget`;
- claim `affordance-target-no-atomics` or an atomic-incapable Core build;
- add `AsyncReadPropertyHandler`, `StepReadPropertyHandler`, or any other
  operation handler trait;
- add host erasure, `HostHandlerRegistration`, registration ingress, sparse
  storage, replacement, selection, execution owners, callbacks, or reducers;
- add a Handler slot, queue, lock, resource reservation, state machine,
  performance workload, or old-API removal;
- add planner, Binding, Servient, Producer, or protocol behavior; or
- create either planned property-read architecture fixture root.

Discovering a required source change outside the two registered implementation
paths revokes admission pending impact review.

## Dependency and ownership verdict

The exact predecessors are:

- `WP-100-FOUNDATION-REFRESH`;
- `WP-100-HANDLER-VALUE-PRIMITIVES`;
- `WP-100-LOGICAL-TIME-CORRECTION`;
- `WP-100-DEADLINE-CLEANUP-TIMING`; and
- `WP-100-HANDLER-CONTEXT`.

All five are complete with passing evidence. Core owns the protocol-neutral
handler contract. The slice does not move planner, Binding, Servient, Producer,
or application-facing setter ownership.

The candidate reruns every predecessor completion check because the handler
source projection must be widened only for the separately validated
`ReadPropertyHandler` trait.

## Risk, resources, and performance

This is Category B implementation risk. The change is additive and local, but
it freezes a public object-safe handler boundary used by later packages. It
therefore receives exact source projection, a real external contract fixture,
feature-cell and negative-scope checks, impact analysis, and independent
review.

The trait itself has no storage, allocation, reservation, state, callback
implementation, or runtime behavior. No resource schema, workload identity,
or performance budget changes. `HandlerFootprint` remains passive metadata on
the reused registration value.

## Contract fixture

The nested fixture under
`tools/compile-contracts/wp100-property-read-handler-slice/` consumes the real
Core, Foundation, and TD crates. It proves:

- root and handler-module trait paths;
- the exact borrowed method signature and result;
- object safety;
- implementation by a deliberately non-`Send`, non-`Sync` handler;
- borrowed static registration and exactly one direct invocation;
- compilation in `no-default`, `async-no-std`, and `std` cells;
- absence of unscoped async and step property-read traits; and
- no architecture fixture root exists.

The actual-source validator rejects supertraits, extra methods, associated
items, default bodies, signature changes, and additional unregistered handler
traits. Existing HandlerContext and five-value source validators remain exact.

## Candidate and independent review

The gate manifest owns the exact candidate base, candidate commit, changed path
set, implementation paths, contract artifacts, prechecks, audit, entry check,
completion key, reused values, and excluded claims. The candidate commit must
be a single child of its registered base and must not change either
implementation path.

The independent review must inspect the registered commit rather than an
uncommitted worktree. A later root continuation session that did not author
the candidate records
`reviewer_attestation_kind = "independent-root-session"` and
`reviewer_id = "codex-agent:/root"`. A separately spawned reviewer records
`reviewer_attestation_kind = "separate-agent-task"` and its real canonical
child-task id. The attestation commit is limited to its TOML record and the
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
- `wp100-logical-time-correction-check`;
- `wp100-deadline-cleanup-timing-check`; and
- `wp100-handler-context-check`.

Before implementation, the completion checker must fail only because the real
Core `ReadPropertyHandler` declaration is absent.

## Completion evidence

Before this tranche becomes complete:

- its independent review attestation and exact three-file approval checkpoint
  must exist; the approval checkpoint changes only `PLAN.md`, this audit, and
  the property-read architecture-gate manifest;
- the exact two-file progress checkpoint changes only `PLAN.md` and the
  property-read architecture-gate manifest and must precede source changes;
- `tools/check-wp100-property-read-handler-slice.sh` must pass;
- `docs/evidence/WP-100-property-read-handler-slice.toml` must record passed
  `property-read-handler-slice` evidence and the exact implementation commit;
- the implementation commit must change only the two registered Core source
  paths;
- exact source projection, three feature cells, object safety, the
  non-thread-safe implementation, borrowed static registration, negative scope
  fixtures, Core unit tests, downstream compilation, and all predecessor
  regressions must pass; and
- later changes to any reused production value must trigger change-control
  impact review and rerun this evidence.

This candidate does not admit broad `WP-100-HANDLER-ENTRY` or create either
planned architecture fixture root.
