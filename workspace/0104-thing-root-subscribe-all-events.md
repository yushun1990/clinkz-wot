# TD-driven Thing-root Event collections

Status: MIGRATED

## Finding and decision

TD 1.1 defines SubscribeAllEvents/UnsubscribeAllEvents as single root
interactions. Its combined Property meta-interaction schema is specific to
Properties; it supplies no universal Event collection wire envelope. Event
subscription, data, dataResponse and cancellation remain separate roles.

PR #255's explicit notification kind is useful, but its complete Property map
cannot represent individual Event occurrences. The current native candidate and
cursor expose only recipient/payload/response. Two Events with different schemas
lose identity there. Fix that boundary with an explicit occurrence target using
the existing AffordanceTarget, distinct from the conversation's Thing target.
Capture a bounded table of exact Event names and all four original schema roles.
Validate and retain the original occurrence target through delivery and response
retirement; native topics remain inside the Binding.

Select a concrete MQTT 3.1.1 single-level wildcard mapping: one combined root
Form with mqv:filter ending in /+, non-retained QoS 0, JSON data. Its final
topic level is the exact Event name. All names must fit this Binding's explicit
single-level ASCII subset; every Event needs supported data and no subscription,
cancellation or dataResponse schema. Unsupported roles refuse preparation rather
than treating protocol ACKs as application channels. Each publication selects one
known Event, validates its schema and sends one PUBLISH. Matching, compatible
filter sharing, queues, unsubscribe correlation and last-owner socket disposal
remain native. The root source and publication authority are distinct from leaf
sources; no leaf emission or Engine fan-out is synthesized.

## Alternatives

- Complete Property-style maps conflate occurrences and require unrelated Event
  data; reject.
- N leaf subscriptions violate the root operation's single-interaction meaning;
  reject.
- One exact aggregate topic with a new JSON envelope is implementable, but adds
  serialization/attribution policy when MQTT already conveys topic identity.
- HTTP SSE can carry native event identity and has relevant WoT Profile guidance;
  it remains a useful later contrast, but requires another Producer streaming
  runtime to establish this increment.
- Arbitrary wildcard extraction, overlapping filters, shared groups and fleet
  pools introduce unproved topology and delivery policy. Support only a terminal
  /+ and reject overlapping distinct filters in one Consumer service. Compatible
  identical filters may share without merging logical authorities or schemas.
- No universal collection control codec or delivery-guarantee abstraction is
  justified. Preserve per-Event roles and decline unsupported aggregate controls.

## Completion boundary

Both roles use public TD-driven selection/complete final publication, captured
actual services, independent source-free owners and real broker I/O. Evidence
must falsify wrong/unknown Event identity and cross-schema data, unsupported
roles/forms/services, family substitution, loss/overflow, authority replacement,
stop before establishment, independent shared stop, uncertain native controls,
late/unread work and final physical/account release. Existing leaf and Property
collection evidence remains regression evidence, not proof of Event collections.

Affected owners: TD semantic closure; EXEC-NOTIFY-001/EXEC-COLLECTION-001;
PREP-SELECT-001/PREP-COVER-001; SPI-DELIVERY-001/SPI-RETIRE-001;
SAFE-CANCEL-001/SAFE-CLEANUP-001/SAFE-OVERFLOW-001; API/state/resource projections;
WP-200/300/400/600 and roadmap capability coverage. No gate or milestone status
transition follows. Independent fresh-context acceptance is required.

## Disposition

The occurrence target, retained member roles and declared native mapping are
projected into TD, Core, Planning, Binding SPI and Runtime Safety contracts,
API/state ownership, work packages and capability coverage. Linked qualification,
real-broker and downstream response fixtures own executable evidence. Existing
leaf Property/Event, collection-map and raw lifecycle suites remain regression
obligations. No milestone, architecture gate, release or fleet-topology claim
closes; independent acceptance remains separate.

## Delivery ownership counterexample

Integration re-review demonstrated the inherited notification result escape on
accepted #256 and the integrated tree: `front()` lent a Payload and Arc-backed
occurrence name, permitting original-allocation clones to survive advance and
successful mapped unsubscribe after every original account returned to zero.
Acceptance of the native MQTT mapping does not establish this allocation claim.

Correct the public delivery boundary directly: `RawNotificationView` uses the
existing Core `PayloadView` and `AffordanceSite`. All variable data is borrowed
from the original queued occurrence under the exclusive root loan, which prevents
advance/cleanup while the data remains in use. Raw and composed callers migrate
directly. Explicit caller-owned copies may survive disposal; the original
allocation may not escape through a view. Native candidates, replies, queues,
matching/distribution and captured validation retain their current ownership.

A new shared delivery/session owner or lease-bearing Payload representation
would change queue admission, advance, codec and native handoffs beyond the
demonstrated defect. Reusing the existing shared slice views closes the payload
and name escapes without extending notification lifetime or changing native
controls. Compile-fail, exact-allocation raw lifetime, real-broker repeated-advance
and downstream reply-retention evidence reaffirm the affected claims. Exact
contracts are migrated to Interaction Core, Binding SPI and Runtime Safety;
independent acceptance remains a separate review of the corrected revision.

Sources: [TD 1.1](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#form),
[MQTT guidance](https://w3c.github.io/wot-binding-templates/bindings/protocols/mqtt/index.html),
[MQTT 3.1.1](https://docs.oasis-open.org/mqtt/mqtt/v3.1.1/os/mqtt-v3.1.1-os.html).
