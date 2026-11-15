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

The dependency-scope reassessment rejects aggregate live invalidation of the
retained definition. A real HTTP counterexample against `306d3d9` changes only
an independent unused Cancel access revision: pending Invoke becomes Closed.
Retained membership is therefore not evidence of a dependency of every admitted
operation. Complete original qualification stays at new association/admission;
live work keeps original Invoke obligations plus its owned exchange/unread-result
operation. A consumed nonterminal control with fully retired exchange has no
remaining authority over pending execution. Explicit pre-effect control refusal
cannot withdraw an unrelated invocation grant, but still observes existing work's
callback-time authority/dependency loss. No declaration baseline is refreshed.

Keeping the aggregate check would preserve this counterexample. Dropping all
control checks would allow stale selected exchanges/results. Remembering every
ever-used operation would retain retired control authority without an owned
obligation; poisoning the shared association on refusal would cross independent
invocations. The existing single-exchange operation index and settlement state
already identify the relevant lifetime, so no dependency graph, grant bitmap,
new Runtime or scheduler is needed. Physical owners/funding remain retained
independently of this narrower validity scope.

Output validation exposes a second ownership conflation. The Action has one
input/output meaning, but its independently selected Forms/services own wire
representations and resource allowances. The real HTTP counterexample on
`485b5e5` admits a two-byte Invoke input, then rejects a valid four-byte Query
output under Invoke's two-byte limit despite Query's eight-byte allowance. The
converse must also refuse a two-byte result when Query permits only one byte,
even if Invoke permits 32. A distinct downstream mapping returns the same
semantic string as plain text while Invoke uses JSON: `ready` is valid text
output, not a JSON string token. Even with generous body bounds, an Invoke JSON
validator cannot validate those original Query bytes correctly.

The [TD 1.1 Recommendation](https://www.w3.org/TR/wot-thing-description11/)
places Action input/output schemas on ActionAffordance (§5.3.1.4) and media on
Forms (§5.3.4.2). Our inference is that shared semantic constraints cannot confer
ownership of every operation's representation on Invoke. TD does not prescribe
this runtime's codec API or guarantee that every query returns completed output;
the native Binding must actually lift a completed application value.

The simplest contract retains shared schemas once and validates with the
returning operation's original service/media/body/scratch/work declaration.
Invoke qualifies input and possible immediate output; Query qualifies output
without implementing unused input; Cancel status/ack remains native. Its own
exchange and unread output keep that codec dependency live. Once Invoke's
exchange retires, its input codec no longer governs a later Query representation;
original Invoke native continuation and access/authority still govern the
invocation. Normal and late result paths use the same producer operation index,
already retained through cleanup. Captured work is paid before poll/close that
may return output; original media and policy are checked around validation even
after application authority loss. No data declaration is refreshed.

Keeping Invoke validation would retain the demonstrated mismatch. Taking the
minimum or maximum of both policies would merge unrelated service obligations
and still not decode differing media. Requiring every Query to produce Invoke
bytes would impose unnecessary Binding transcoding and hide original late
evidence. A universal codec/value conversion layer has no demonstrated need:
Core's existing schema evaluation already supplies shared constraint meaning.
Dropping validation after failure would erase late evidence obligations. The
selected change therefore keeps the original output bytes, selected codec and
funding together without adding a runtime, conversion carrier or new authority.

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
