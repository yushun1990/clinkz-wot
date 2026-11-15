# Continuing Consumer Action association and capacity

Status: MIGRATED

Review 10 F2/F3 expose a shared ownership defect: the former Action preparation
charged physical services per operation, while `ConsumerActionRuntime` consumed
definitions and captured fresh declarations beside per-Consumer vacant slots.
A definition, Consumer association, invocation and capacity therefore could not
be retained or reused independently. The real HTTP path already preserves a
continuing invocation beyond each exchange; keeping that path is the smallest
useful way to correct the boundary.

The selected increment removes the Runtime, retains endpoint-specialized
qualified mappings/data in the existing definition, binds explicit access owners
per operation in an independent Consumer association, and admits original
invocation owners into separate family-specific capacity. Startup composition
imports the existing `RetainedConsumerServices` owners, so selection never adopts
or charges raw backing again. Newly bound access is explicitly qualified; it is
not permission embedded in reusable definition bytes. Prepared data/native
metadata remains the original qualification and funding baseline. The old settlement's `&Payload` and
`&InteractionOutput` loans also allowed cloning original body Arcs after funding
refund. The affected result boundary now lends existing Core slice views; sharing
the complete settlement retains the original bytes and their funding together.
Capacity slots hold references to separately funded invocation records. Keeping
inline records while also reserving admitted record bytes would charge the same
physical storage twice and obscure the record's settlement lifetime. One bounded
allocation per admitted invocation preserves the existing checkout/control state
without reserving that complete record in every vacant slot. Caller-owned/static
storage remains a separately qualified representation, not an additional facade.

Alternatives considered:

- Deduplicate the old reservation arithmetic and add a construction check.
  This could repair two counterexamples while leaving definitions consumable by
  only one Consumer/slot container. It would preserve the defect's ownership cause.
- Keep the old Runtime behind a compatibility adapter. Pre-v1 migration is small
  enough to update all live callers, and an adapter would retain competing
  ownership semantics.
- Share the entire old definition, including its access policy. This would make
  cross-instance reuse depend on preparation-time policy and obscure explicit
  independent Invoke/Query/Cancel access binding.
- Copy Notification's raw conversation owner. Action already has the necessary
  unique handle, occupied checkout, opaque continuing token and terminal evidence
  state. Its exchanges and continuing invocation need different lifetimes; they
  do not need Notification's queue, occurrence or response model.
- Build a universal Runtime, scheduler, cache or native pool. None is needed to
  distinguish these owners or falsify the HTTP claims. Physical HTTP occupancy
  remains a Binding policy.

This applies the definition/authority split of 0081 and the independent-capacity
principle of 0121/0123. It does not import their representation as a universal
carrier. The normative migrations are [Planning](../docs/spec/planning.md#continuing-consumer-action-preparation)
(PREP-ISOLATION/SELECT/REUSE), [Core](../docs/spec/interaction-core.md)
(EXEC-ACTION/DATA/OUTCOME), [Runtime Safety](../docs/spec/runtime-safety.md#continuing-action-associations-and-invocation-capacity)
(SAFE-AUTH/CANCEL/CLEANUP/CONCUR), and [Binding SPI](../docs/spec/binding-spi.md)
(SPI-COMPOSE/DELIVERY/RETIRE), under unchanged Foundation original-owner
obligations. API/state/lifecycle and WP-200/300/400 projections follow those owners.
No new ADR, requirement identity, gate or milestone closure is selected.

[Executable evidence](../docs/evidence/consumer-action-composition.md) distinguishes
actual same-definition reuse from cross-endpoint lowering, and funded account
reservations from heap/performance claims. Mixed HTTP/MQTT composition, reusable
qualified Producer exposures, automatic integration and broader profiles remain
outside this increment. Acceptance is Owner-directed; this record supplies no
independent acceptance claim.
