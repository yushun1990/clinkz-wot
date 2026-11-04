# Thing-root ObserveAllProperties / UnobserveAllProperties

Status: MIGRATED

## Counterexample and selected boundary

A root notification cannot be represented by the existing Property/Event-only
kind, scalar schema, or named Producer source. Relabeling it as an individual
Property would misstate authority, policy operations and data roles. The existing
logical conversation, original-instance fencing, independent cursor, bounded
queue and Binding participation owners remain applicable. Correct their target
and schema representation; do not create a parallel subscription framework.

The old inference of notification kind from target scope also fails at the root:
TD 1.1 defines both Property and Event collection Forms there. Capture kind
explicitly in selection, raw requests and publication routes, and validate its
pairing with scope/program/data. Native publication route lookup also takes kind;
target alone cannot select a root family. Do not globally equate Thing with Properties.

TD 1.1 root Forms explicitly describe the collection operation. Its Property
meta-interaction schema combines named Property schemas in an ObjectSchema.
The selected bounded mapping delivers complete readable Property maps on one
exact MQTT topic. Every sample contains every non-writeOnly Property exactly
once, under its original TD name and schema. Root authority is independent of
individual observe Forms and the per-Property observable hint. No native
SUBACK, initial Read, per-Property driver, changed-member inference or atomic
snapshot guarantee is introduced.

## Alternatives

- N ObserveProperty calls: rejected; not the described native collection.
- Wildcard topic-to-member extraction: useful for some peers, but MQTT vocabulary
  provides no standard TD extraction rule. It requires a concrete mapping and
  extra routing/attribution evidence that this full-map peer does not need.
- Per-update named deltas: narrower payloads, but require an explicitly supported
  partial-map contract and validation service. Full named maps suffice here.
- HTTP SSE/CoAP Observe: defensible future mappings; introducing another native
  runtime adds no necessary evidence to this increment.
- Binding-local exact-topic sharing: retain the existing checked registration,
  independent interests and native last-owner disposal. One logical root
  observation does not imply one connection, nor one driver per member.

The MQTT guidance is evolving, not a Recommendation, and its default table does
not define observe-all. This Binding explicitly adds the collection mapping:
one combined root Form declaring both operations, exact mqv:filter, JSON,
non-retained QoS 0; SUBSCRIBE / UNSUBSCRIBE implement the two operations. It
refuses conflicting controlPacket declarations. Separate control Forms, wildcards,
shared groups, TLS, credentials, replay/reconnect and partial maps are outside
this mapping. Engine contracts remain independent of this restriction.

## Required evidence and affected authority

Real Engine Producer -> Engine Consumer maps through Mosquitto, independent
wire peers, complete final-TD qualification, member/schema/media/duplicate
refusal, root-only authority, independent stop and policy, loss/saturation,
replacement, late owned work, pending/failed unsubscribe and physical release.
Fund schema retention/construction/disposal and captured validation before
effects. Reaffirm existing Event and single-Property evidence.

Affect EXEC-NOTIFY-001, EXEC-COLLECTION-001, Planning, Binding SPI, runtime safety,
TD schema loans, API ownership, WP-200/300/400/600 and capability coverage.
Topic 0082 remains open: this proves bounded exact-topic sharing and one
collection, not arbitrary wildcard attribution, tenant compatibility, Host/static
parity, large-fleet throughput or a universal subscription pool.

Sources: [TD 1.1 Recommendation](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#thing),
[MQTT Binding work in progress](https://w3c.github.io/wot-binding-templates/bindings/protocols/mqtt/index.html),
[MQTT 3.1.1](https://docs.oasis-open.org/mqtt/mqtt/v3.1.1/os/mqtt-v3.1.1-os.html).

## Disposition

The kind/scope correction, retained collection schemas, explicit root Producer
source/data and MQTT mapping are implemented and projected into the affected
domain contracts, API/state ownership, work packages and PLAN coverage. The
qualification and real-broker fixtures linked there own executable evidence;
existing single-Property/Event and downstream Consumer fixtures remain regression
obligations. No milestone, fleet-topology or production qualification closes.
