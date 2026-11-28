# Consumer notification composition evidence

Scope: Host manual Property/Event notification composition, including the existing
MQTT leaf and root collection mappings. This is capability evidence, not independent
acceptance, a package/milestone closure or production security qualification.
Contracts remain in [Planning](../spec/planning.md#open-consumer-notification-preparation),
[Core](../spec/interaction-core.md),
[Runtime Safety](../spec/runtime-safety.md#notification-associations-and-conversation-capacity)
and [Foundation](../spec/foundation.md). Decision alternatives are recorded in
[topic 0123](../../workspace/0123-consumer-notification-association-capacity.md).

The [public MQTT fixture](../../protocol-bindings/protocols/mqtt/tests/notification_composition.rs)
uses one actual MQTT registration and program. Its wrapper counts the real pure
compilation callback while delegating preparation, support and native participation
to the production Binding. One TD admission/compilation produces one immutable
endpoint-specialized definition; eight independent Consumer associations retain
it, with separately selected policies. Binding those existing
facts does not repeat TD admission/schema preparation/native lowering. This is not
compatibility checking for a newly supplied TD or a changed endpoint, and is not
cross-endpoint reuse of partially invariant lowering as in the HTTP extractor.

Two conversation slots admit two associations. Idle associations use no
conversation, report, queue or native interest slot. A third association is
refused while both slots are occupied. A locally closed conversation holds its
slot until collection. Holding the collected report still blocks the last
report slot even after the conversation slot is free. Dropping it allows a
successor to subscribe while the original sibling still holds the shared broker
interest. The fixture exercises withdrawal of a continuing grant, denial of
mapped cancellation, passive view loss and final release with partial native
writes. Mosquitto observes one SUBSCRIBE and one final UNSUBSCRIBE. The Binding
owns those controls, distribution, the socket and physical participation.

A second broker test fills the Binding's 32 participation slots using one
connected subscriber and 31 locally acquired interests. A separate capacity
with vacant conversation and report slots is refused. Completing the original
native close permits the contender to establish while the original report is
still held. The pre-effect refusal releases the contender's provisional report
slot, so that retry uses its only slot.

The policy sees the original Thing-root conversation and named `alarm`/`ready`
Event coordinates, including a permission change after queue admission. Only the
original permitted Consumer receives its queued value. These are TD Event labels,
not publisher identities. The independent peer is an anonymous MQTT publisher.
NoSec, the topic filter and an application meter label prove no source principal.

The authorization counterexamples reject `alarm` and subsequently deliver `ready`
on the same original logical subscription, with the sibling independently receiving
both. One case refuses at native candidate acceptance and verifies that native
polling/empty views never request Deliver without an occurrence. Another changes
policy after both items are queued, refuses the original head and verifies that
relaxing policy cannot resurrect it. After bounded disposal, the allowed queued
Event is delivered with no resubscribe, unsubscribe, first cause or native loss.
Both cases fail on the original PR head: the first observes an occurrence-free
Deliver call and the second gets terminal Security refusal from `front()`.

The raw reply-owner fixture refuses occurrences at acceptance, queued delivery,
response admission and delayed response send, including required replies.
Pending reply retirement retains the original payload and submitted response.
Retirement completes without a fresh delivery grant. The response input is then
released and the original payload stays owned until disposal. A later allowed
Event and reply use the same conversation. A retirement error fences that
conversation and retains the original job. Another case withdraws the
continuing grant inside an occurrence callback. The Maintain check refuses both
staged and queued handoffs, including when the item decision rejected the
occurrence. The independent Binding supplies these reply opportunities.

Review 10's notification F2 correction imports original data/access owners across
independently constructed registries. A fixture allocates 60,000 bytes of data
service backing and retains its one original bank charge while two mappings each
add the same separate record/program/schema charge at another bank. That per-map
charge is smaller than the service backing; physical service backing is not added
to either mapping. Work/instance/report lifetimes retain the original owners and
all account reservations reach zero after final disposal. These assertions measure
reservation topology; they are not an allocator/RSS or throughput benchmark.
Raw startup adoption separately recognizes same-object aliases within one registry;
sharing across registry roots requires explicitly propagating retained owners.

The dependency regressions change retained, temporary and body bounds between
preparation, installation and admission. They also exercise native support,
codec availability and qualification revisions for data and access. An observed
change after admission fences the original conversation before start. Restoring
the metadata before cleanup leaves that fence in place.

The response-validation regression mutates the prepared data service's body
bound inside the JSON callback for both accepted and refused payloads.
Ordinary malformed input returns the original input and leaves the conversation
Active. An observed dependency violation fences it even when validation fails.
Pending cleanup retains the exact occurrence and reply and sends no dataResponse.
The downstream UDP fixture tests final report and idle association disposal
against an attempted callback-driven Read. A later independent turn can send
the Read. [Mixed execution evidence](consumer-execution-composition.md)

The [raw lifecycle suite](../../servient/finite/tests/notification.rs) drives
one original slot per aggregate turn. It exercises bounded refusal, queue and
handoff ownership, pending and failed retirement, local abort, and first cause
when a native outcome arrives late. In the closing case, the physical
participation can retire while the reply is pending. A pending mapped stop
loses permission on a later poll and still completes local retirement. A held
report separately saturates report admission and returns its unit through
disposal. Zero-size local capacity and oversized inputs refuse before native
construction. The legacy global/per-Thing subscription accounts and execution
work factories have no callers on this path.

The [independent Cargo-linked fixture](../../tools/architecture-fixtures/open-consumer/tests/notification.rs)
retains its structured four-role and exclusive response-owner regressions. Its
additional delayed-response case checks that response admission and subsequent
send authorization receive the original Event coordinate. A later denial sends
nothing and retires the original reply/payload. This is deterministic native-owner
lifecycle evidence, not MQTT response support or new wire interoperability.
Existing independent datagram composition still supplies real non-MQTT I/O.

Reproduce with:

```sh
cargo test --locked -p clinkz-wot-core -p clinkz-wot-planning -p clinkz-wot-servient-finite
cargo test --locked -p clinkz-wot-binding-mqtt --features broker-tests
cargo test --locked --manifest-path tools/architecture-fixtures/open-consumer/Cargo.toml
tools/check-design-artifacts.sh
cargo check --locked -p clinkz-wot-core -p clinkz-wot-planning -p clinkz-wot-servient-finite --no-default-features
```

The broker command requires a real Mosquitto 2.x executable on PATH or
`MOSQUITTO_BIN`; it never silently skips. Exact tested revisions/results belong
to the PR and CI. No numeric fleet throughput or preparation-time improvement is
claimed by this increment.

Remaining architectural gaps include authenticated occurrence provenance with an
actual mapping/verifier and retained original trust/dependency context, including
forged/absent/ambiguous/stale evidence tests; changed-description/endpoint native
lowering reuse; constrained notification execution and authority wake integration.
The separate [continuing Action increment](consumer-action-composition.md)
addresses its F2/F3 and independent-capacity handoff. The subsequent
[Host turn composition](consumer-execution-composition.md) supplies optional
aggregate driving and mixed Consumer lifecycle integration; it does not add an
automatic authority-change wake or close the provenance/profile gaps.
A native MQTT topic or NoSec message cannot fill the provenance gap. General
wildcard coalescing, tenant/credential sharing and MQTT reciprocal Event responses
remain outside the declared mapping. Priority 3 and the broader v6 architecture
are not complete.

## Heterogeneous Event selection

One downstream application registers two independently implemented Event Bindings.
The stream subscriber opens a TCP connection and receives one-byte frames.
The datagram subscriber registers its ephemeral UDP port and can decode a hex-coded
occurrence before the data service validates the JSON integer. At the same
dual-listening test endpoint either implementation can qualify a Form without
a subprotocol. Pinning the provider executes its native control at its peer.
[Peer implementations](../../tools/architecture-fixtures/open-consumer/src/event.rs)

The public preparation tests execute the preferred stream Form and the later
coded datagram Form in both registration orders. Removing the earlier Form's
NoSec service makes the later Form win. Revoking the stream instance closes
its connection while the datagram instance receives another validated value.
Its mapped cancellation reaches the original peer and releases the port.
The same fixture exercises missing data, competing providers, a reload during
qualification, a new competitor after selection, short work and overlapping
mapping capacity. Original banks return to baseline on refusal and final release.
[Selection and execution tests](../../tools/architecture-fixtures/open-consumer/tests/event_selection.rs)

A second composition pairs the in-tree MQTT Binding and the downstream UDP
Binding on the same TD. An independent MQTT 3.1.1 wire peer observes the
subscription, delivers `5` and acknowledges unsubscribe. The selected UDP
instance subsequently receives `6` and sends its own cancellation.
[Mixed MQTT and datagram test](../../tools/architecture-fixtures/open-consumer/tests/event_selection.rs)
