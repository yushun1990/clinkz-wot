# 0012 Property-Read Handler Slice Boundary

Status: MIGRATED

## Problem

`PROPERTY-READ-ARCHITECTURE` makes
`WP-100-PROPERTY-READ-HANDLER-SLICE` the only exception to the blocked broad
`WP-100-HANDLER-ENTRY`. The initial gate manifest also required
`real-no-atomic-boundary-proven` before that slice could proceed.

Those two rules form a dependency cycle. The frozen handler amendment requires
the broad `affordance-target-no-atomics` evidence to include a complete
no-default Core build for `thumbv6m-none-eabi` or an equivalently
atomic-incapable target. The current Core still contains unrelated legacy
`Arc` surfaces in payload, handler storage, inbound, outbound, event, and
synchronization paths. Migrating those surfaces belongs to later broad WP-100
work, but broad entry is blocked until the property-read architecture gate
passes.

The same slice cannot silently absorb final `InteractionInput` and
`AcceptHint` migration. Those values have resource-admission and downstream
construction-site impacts that differ from a single additive handler trait.

## Evidence

- `docs/amendments/WP-100-handler-api-v1.md` requires both an
  atomic-incapable Core build and source/API rejection of `Arc` for the broad
  no-atomic evidence.
- `docs/audits/WP-100-handler-context-entry.md` already records that target
  replacement/no-atomic proof intersects unrelated legacy Core `Arc` surfaces,
  while `AcceptHint` and `InteractionInput` require a coordinated storage and
  construction-site migration.
- The architecture gate's exact scenario needs one statically registered
  synchronous property-read handler. It does not exercise an async handler,
  bounded-step handler, host-erasure implementation, sparse handler store, or
  final request-storage schema.
- ADR-0013 permits a narrow tranche only when its claims, implementation paths,
  blockers, and completion evidence are truthful and independently reviewable.

## Decision

The exact WP-100 property-read handler slice is a synchronous composition seam:

- it adds only `clinkz_wot_core::ReadPropertyHandler`;
- its only implementation paths are `core/src/handler.rs` and
  `core/src/lib.rs`;
- it reuses the completed `HandlerContext` and
  `StaticHandlerRegistration` plus the existing production
  `InteractionInput` and `InteractionOutput`;
- its `async-no-std` cell proves the same synchronous trait and borrowed static
  registration remain available when the portable async feature is enabled;
  it does not add `AsyncReadPropertyHandler` or `StepReadPropertyHandler`; and
- its candidate must receive independent ADR-0013 review before either source
  path changes.

The slice explicitly does not claim:

- `AcceptHint` resource admission;
- final `InteractionInput` storage migration;
- `AffordanceTarget` relocation or broad no-atomic evidence;
- async or bounded-step handler traits; or
- host erasure, sparse storage, dispatch, cancellation, Producer, Binding, or
  Servient ownership.

The broad no-atomic and request-storage work remains blocked and separately
evidenced under WP-100. A later semantic change to any reused production value
triggers change-control impact review and reruns the property-read evidence.
This preserves evidence truth without making the early architecture proof
depend on broad entry.

## Rejected alternatives

- Retaining the broad no-atomic condition on the slice was rejected because it
  creates a cycle with the gate that blocks broad entry.
- Bundling every legacy `Arc`, `AffordanceTarget`, `AcceptHint`, and
  `InteractionInput` migration into the property-read slice was rejected
  because their ownership, implementation paths, resource behavior, and
  validation boundaries differ.
- Adding all sync, async, and step property-read traits was rejected because
  the first architecture scenario invokes only one synchronous static handler,
  while async and step contracts carry distinct cancellation and work-budget
  requirements.
- Using a fixture-only handler input or target was rejected because the gate
  forbids adapters that replace production ownership boundaries.

## Migration

The decision is projected into:

- `docs/work-packages/property-read-architecture-gate.toml`;
- `docs/work-packages/PROPERTY-READ-ARCHITECTURE.md`;
- `docs/work-packages/WP-100-core.md`;
- the executable work-package checker;
- `PLAN.md` and `PROJECT_STATE.md`; and
- `workspace/INDEX.org`.

This decision prepares an exact candidate boundary. It grants no source-edit
authority and creates neither planned architecture fixture root.
