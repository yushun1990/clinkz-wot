# MQTT Producer and Consumer notifications

This bounded Host mapping joins observable Properties and Events declared in a
TD to actual MQTT value/data delivery between Engine roles. Pure preparation,
closed native setup, complete final qualification and one logical publication
capture the actual sources, mappings and services. A TD-selected Consumer owns
establishment, validated delivery, stop/unobserve or unsubscribe and final
physical cleanup. Property values retain their own DataSchema; Event data and
control/response roles stay distinct. This does not establish complete family,
production, security or constrained-profile qualification.

The [MQTT Binding draft](https://w3c.github.io/wot-binding-templates/bindings/protocols/mqtt/index.html)
is an evolving work in progress, not a W3C Recommendation. This implementation
uses its separate broker URL and `mqv:filter` convention and default
subscribe/unsubscribe and observe/unobserve mappings. In particular, its
non-retained Property mapping is observation-only: `observeproperty` uses
SUBSCRIBE and `unobserveproperty` uses UNSUBSCRIBE. It supplies future values
without an implicit Read or fabricated initial value. Wire behavior is
[MQTT 3.1.1](https://docs.oasis-open.org/mqtt/mqtt/v3.1.1/os/mqtt-v3.1.1-os.html).

Supported TD subset:

- Local `mqtt://IP:port` loopback broker, optional trailing slash; no DNS, TLS,
  credentials, URI variables, query, path topic or fragment.
- Observable Property value schemas (`observable: true`) or Event `data` with
  the existing complete type-only DataSchema subset, JSON
  content, at most 4096 encoded bytes; no content coding. JSON syntax, Unicode
  scalar validity and mathematical integrality are checked by the shared JSON
  codec service also used by HTTP. All unsupported assertions refuse.
- Exact `mqv:filter`, 1–128 ASCII graphic bytes, no wildcard or `$` topic.
  An explicit `mqv:topic` must equal that filter. One observe/subscribe mapping
  per source; distinct Property/Event sources and independent logical slots
  cannot collide on a topic in
  the same native service. A successor can reuse its slot's topic with a fresh
  mapping identity.
- Property `observeproperty` and optional `unobserveproperty`; Event
  `subscribeevent` and optional `unsubscribeevent` (including Event TD defaults).
  Property operations must be explicit: default read/write operations do not
  acquire an observation mapping. An observation source does not supply a Read
  or Write handler; Producer qualification checks every advertised operation.
  Omitted control packets follow the operation default. Explicit control packet
  values require a single matching operation. `mqv:qos` absent, `0` or `"0"`;
  `mqv:retain` absent or false. Actual publication is one non-retained QoS 0
  PUBLISH independent of subscriber count; the broker handles native controls.
- One effective NoSec branch and actual captured JSON/NoSec services. The
  separately configured broker must permit anonymous operation. Unknown
  extensions (except class annotations), profile declarations, response branches,
  subscription/cancellation data, and **any** dataResponse schema refuse rather
  than becoming ignored metadata. MQTT protocol ACKs are not Event dataResponse.

For example, a Property with no MQTT Read/snapshot capability:

```json
{
  "@context": ["https://www.w3.org/2022/wot/td/v1.1", {
    "mqv": "https://www.w3.org/2019/wot/mqtt#"
  }],
  "title": "Observable temperature",
  "security": ["n"],
  "securityDefinitions": {"n": {"scheme": "nosec"}},
  "properties": {"temperature": {
    "type": "integer",
    "observable": true,
    "readOnly": true,
    "forms": [{
      "href": "mqtt://127.0.0.1:1883",
      "op": ["observeproperty", "unobserveproperty"],
      "mqv:filter": "things/temperature",
      "mqv:qos": "0",
      "mqv:retain": false
    }]
  }}
}
```

`MqttNotificationProducer` is an immutable actual startup registration implementing
`ProducerCapability`. `prepare_thing` owns every selected definition independently
of the source. `MqttNotificationService::participation` checks the exact registration and
constructs an unstarted native owner for `ComposedProducerExposure`. The exposure
reserves its controlled layout, data/frame/result/cleanup opportunity and final
qualification scratch at the Host bank before any managed setup. The raw logical
root and startup registrations remain caller-provisioned, as in existing composed
Producer paths. Services cannot be swapped at publication or per emission.

The native service is application-owned infrastructure with a fixed policy:
at most 32 participation leases, each with at most 16 notification sources/32 operation
definitions; one shared publication slot; 2-second setup timeout; clean session,
keepalive disabled; no reconnect, retry, replay or idle retention. Native setup
and health, emission and retirement require explicit caller progress. Native
socket/SDK state and this fixed service policy have their own Binding/deployment
owner; Host bank charges are not total process RAM claims. One service can share
one connection across Things and compatible NoSec Servients. A participation loan
cannot request whole-service shutdown. Last actual lease/work disposal closes
the socket even if the application still owns the service object.

`RawProducerServient::publish_notification` accepts an explicit Property/Event
target and `PublicationData::PropertyValue` or `EventData`. A mismatched family
refuses before native effects and returns the original carrier. Applications
register actual `ProducerNotificationSource::property` / `event` sources;
Property registration is independent of any finite Read behavior. It checks
the current qualified Property/Event mapping, backing, policy and validator before native
admission. Refusal returns the original data. Accepted work retains the original
instance/mapping, actual payload and capacity through native retirement and
settlement acknowledgement. A full slot returns Backpressure; there is no hidden
queue, coalescing, subscriber replication or loss disguised as a newer value.

The result distinguishes first local cause from native effect: `NotPublished`,
`Uncertain` or `LocallyPublished`. The latter means the complete packet was
locally written; it proves neither broker acceptance nor subscriber delivery.
Zero subscribers still permits local success. Revocation, replacement, deadline,
root loss and local cancellation fence pending emissions; escaped writes cannot
be rolled back. Cancelling/dropping a partial packet closes the shared connection
to preserve framing, reports uncertainty and invalidates dependent availability.
Native failure is terminal for that service and never silently replays data.
Unread settlements retain the slot and original funding. Closing one Thing
preserves sibling leases; retained emissions prevent premature logical retirement
and final physical release. Drop reports no successful drain.

Run the [qualified application example](examples/emit_event.rs) with a local
anonymous broker and an independently subscribed MQTT peer:

```sh
cargo run --locked -p clinkz-wot-binding-mqtt --example emit_event -- 127.0.0.1:1883 things/alarm 42
```

The default qualification tests need no broker. The real-peer suite requires
Mosquitto 2.x on PATH, or `MOSQUITTO_BIN` pointing to its executable, and fails
explicitly if unavailable:

```sh
cargo test --locked -p clinkz-wot-binding-mqtt --features broker-tests
```

These tests start isolated real broker processes and use independent TCP MQTT
subscribers. They exercise matching/distribution, unsubscribe, absent subscribers,
no initial/replayed value, refusal/validation/policy, actual service capture,
saturation, cancellation/deadline, replacement, late effects, broker failure,
shared participation, original-bank retention and broker-observed final socket
release. The opt-in manual workspace/feature/full CI runs this suite explicitly.

## TD-selected Consumer

`MqttConsumerService` provisions a distinct Consumer connection service and
supplies the actual startup-linked `MqttNotificationConsumer` registration. Typed
`open_consumer::ConsumerNotificationRegistry` prepares one named observable
Property or Event, selecting
the first completely supported observe/subscribe Form in TD order, or the explicit
Binding/Form selector. Multiple actual providers for one Form refuse. It uses
the common admitted TD semantics and the same MQTT vocabulary interpretation as
the Producer. The registry captures the actual `ConsumerDataService` (for example
`MqttJsonData`) and application `ConsumerAccessService`; NoSec is not destination
or application authorization. Registrations may come from external linked crates.
The Consumer NoSec security definition admits no extra semantic fields.
No dummy finite capability, native setup or logical permission is needed to compile.

The owned `ConsumerNotificationDefinition` contains a discriminated
`ConsumerNotificationSchemas`: the Property's own value schema or all four
Event schema roles. It also owns the actual native program/configuration,
family-qualified target, services and original funding. TD, semantic arena and registry can be destroyed before execution.
The current MQTT subset requires the Property's supported value schema or
Event `data`, and rejects subscription and
cancellation schemas, **any** dataResponse schema, response branches and the
other unsupported assertions listed above. Mere presence of a dataResponse
schema never creates a response right. A caller requiring native Event responses
is refused before native acquisition; protocol SUBACK is not a dataResponse.

`ConsumerNotificationRuntime` consumes the definition and an independent
`ConsumerInstance` reference. It composes the existing raw notification lifecycle;
it is not another execution kernel. Every logical observation/subscription gets its own move-only
cursor, identity, policy checks, queue, loss/stop state and settlement. The
captured JSON/schema service validates each candidate before logical queue
acceptance. Original authority is checked after native/validation callbacks and
again at application delivery. No codec, executor or configuration is supplied
per operation. Runtime construction reserves the owner-scoped Engine queue and
validation envelope at the original Host bank; these conservative root leases
last until runtime disposal. Pure compilation requires a per-step Planning work
allowance of at least 4096 (the stock Gateway preset has 256); callers must
explicitly provision that allowance, compilation temporary capacity and runtime
limits. This does not qualify the unmodified Gateway resource profile.

Native policy is fixed and bounded: at most 32 actual participation leases and
32 exact topics per service, four queued payloads per participant, 4096 payload
bytes per item, a bounded frame, one control in flight and two-second connect/
control timeouts. Native socket, queues, frame, waker and control backing belong
to this Binding policy, separate from Engine bank accounting. Compatible logical
subscribers share a single native topic subscription and connection. The Binding
matches and distributes received payloads (sharing immutable body storage);
the Engine does no subscriber loop. Each recipient still validates independently.
Native queue overflow drops the newest value/data and reports loss to that
recipient; Engine queue overflow follows its explicit DropNewest/Terminate
policy. Property coalescing is not supplied by this subset, and Event data is
never treated as coalescible state. A pinned cursor is not replaced by newer data.

Native establishment waits for successful QoS 0 SUBACK. It performs no Read,
creates no initial Property value or Event, and caches no latest value. Broker
deliveries with RETAIN set are discarded and counted as loss;
Terminate therefore closes on such loss. Under MQTT 3.1.1 a live retained
publication is forwarded to existing subscribers with RETAIN cleared, so this
policy cannot identify the original publisher's retained flag. A later logical
join to an existing subscription receives no sibling's queued data. Matching is
by reception at the native service, without a cross-incarnation wire-history,
publisher identity, global ordering, replay or loss-recovery guarantee.

Stop fences local delivery immediately, even with zero work or denied/malformed
mapped cancellation. An empty `CancellationInput` requests the selected described
unobserve/unsubscribe; application data is refused with its original input. The
current mapping supports that request only when the selected observe/subscribe
Form also includes `unobserveproperty` / `unsubscribeevent` respectively
(including Event TD defaults); separate control Forms are outside
this Consumer subset. Local stop remains available otherwise. Cancellation is
reauthorized before native control acceptance using the original target's stop
operation. Establishment and delivery authorize ObserveProperty or SubscribeEvent
respectively, without crossing schema or authority families. Native lease cleanup releases
interest without application control data: the final topic interest sends
UNSUBSCRIBE and waits for UNSUBACK, while a sibling retains the shared interest.
`Confirmed` refers to that original mapped removal's ACK, `Unknown` preserves an
attempt without a known ACK, and `NotAttempted` covers local-only cleanup or a
shared interest that required no mapped broker unsubscribe. Infrastructure
housekeeping alone never claims authorized application cancellation.

Control frames remain owned by the native service if their initiating logical
owner drops; siblings can finish them without corrupting framing. Connection/
protocol failure is terminal and is propagated to dependents. There is no retry,
reconnect, replay or idle pool. Last actual participation disposal closes the
socket even while service objects, definitions or settlements remain retained.
A later explicit subscription may start a fresh clean connection on a healthy
service; it resumes no old conversation or queue. A failed service requires a
fresh service owner. Root Drop aborts
locally without claiming remote drain. Manual progress is required.
Progress runs inside a Tokio runtime with I/O and time enabled.

`tests/property_observation.rs` and `tests/consumer_events.rs` use the existing
qualified Engine Producer and a real
Mosquitto broker for joined delivery, independent shared Consumers, replacement,
schema mismatch, overflow and final release. Additional real broker wire inputs
challenge invalid JSON and retained data. Focused cases also cover authorization,
pre-effect refusal/stop, cancellation acceptance, partial-control cleanup, broker
failure and original funding. `tests/consumer_qualification.rs` exercises pure
selection, actual service pairing and unsupported descriptions without a broker.
`tests/property_qualification.rs` additionally checks observable declaration,
actual Property source/services, complete final description, value schema,
Form selection and family substitution refusal. Both families execute the same
Consumer lifecycle regression cases. Property-specific Producer cases cover
policy/validation, cancellation, deadline, replacement, unread funding, partial
publication and a Thing with same-named Property/Event sources. The same
`broker-tests` command and manual CI step run both roles/families.

General DataSchema/security, QoS 1/2, TLS, broader loss/reconnect policies,
dataResponse with actual response cardinality, separate control mappings,
collections and constrained profiles remain separate work.
