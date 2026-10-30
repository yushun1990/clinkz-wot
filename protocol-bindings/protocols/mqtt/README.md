# MQTT Producer Events

This bounded Host mapping joins an Event declared in a TD to pure preparation,
closed native setup, complete final qualification, logical publication,
application emission and actual MQTT broker distribution. It is the first
executable Producer Event slice, not complete Event-family or production
qualification. The Engine has no remote subscriber registry or fan-out loop.

The [MQTT Binding draft](https://w3c.github.io/wot-binding-templates/bindings/protocols/mqtt/index.html)
is an evolving work in progress, not a W3C Recommendation. This implementation
uses its separate broker URL and `mqv:filter` convention and default
subscribe/unsubscribe mappings. Wire behavior is
[MQTT 3.1.1](https://docs.oasis-open.org/mqtt/mqtt/v3.1.1/os/mqtt-v3.1.1-os.html).

Supported TD subset:

- Local `mqtt://IP:port` loopback broker, optional trailing slash; no DNS, TLS,
  credentials, URI variables, query, path topic or fragment.
- Event `data` with the existing complete type-only DataSchema subset, JSON
  content, at most 4096 encoded bytes; no content coding. JSON syntax, Unicode
  scalar validity and mathematical integrality are checked by the shared JSON
  codec service also used by HTTP. All unsupported assertions refuse.
- Exact `mqv:filter`, 1–128 ASCII graphic bytes, no wildcard or `$` topic.
  An explicit `mqv:topic` must equal that filter. One subscribe mapping per Event;
  distinct Events and independent logical slots cannot collide on a topic in
  the same native service. A successor can reuse its slot's topic with a fresh
  mapping identity.
- `subscribeevent` and optional `unsubscribeevent` only, including TD defaults.
  Omitted control packets follow the operation default. Explicit control packet
  values require a single matching operation. `mqv:qos` absent, `0` or `"0"`;
  `mqv:retain` absent or false. Actual publication is one non-retained QoS 0
  PUBLISH independent of subscriber count; the broker handles native controls.
- One effective NoSec branch and actual captured JSON/NoSec services. The
  separately configured broker must permit anonymous operation. Unknown
  extensions (except class annotations), profile declarations, response branches,
  subscription/cancellation data, and **any** dataResponse schema refuse rather
  than becoming ignored metadata. MQTT protocol ACKs are not Event dataResponse.

For example:

```json
{
  "@context": ["https://www.w3.org/2022/wot/td/v1.1", {
    "mqv": "https://www.w3.org/2019/wot/mqtt#"
  }],
  "title": "Alarm source",
  "security": ["n"],
  "securityDefinitions": {"n": {"scheme": "nosec"}},
  "events": {"alarm": {
    "data": {"type": "integer"},
    "forms": [{
      "href": "mqtt://127.0.0.1:1883",
      "op": ["subscribeevent", "unsubscribeevent"],
      "mqv:filter": "things/alarm",
      "mqv:qos": "0",
      "mqv:retain": false
    }]
  }}
}
```

`MqttEventProducer` is an immutable actual startup registration implementing
`ProducerCapability`. `prepare_thing` owns every selected definition independently
of the source. `MqttEventService::participation` checks the exact registration and
constructs an unstarted native owner for `ComposedProducerExposure`. The exposure
reserves its controlled layout, data/frame/result/cleanup opportunity and final
qualification scratch at the Host bank before any managed setup. The raw logical
root and startup registrations remain caller-provisioned, as in existing composed
Producer paths. Services cannot be swapped at publication or per emission.

The native service is application-owned infrastructure with a fixed policy:
at most 32 participation leases, each with at most 16 Event sources/32 operation
definitions; one shared publication slot; 2-second setup timeout; clean session,
keepalive disabled; no reconnect, retry, replay or idle retention. Native setup
and health, emission and retirement require explicit caller progress. Native
socket/SDK state and this fixed service policy have their own Binding/deployment
owner; Host bank charges are not total process RAM claims. One service can share
one connection across Things and compatible NoSec Servients. A participation loan
cannot request whole-service shutdown. Last actual lease/work disposal closes
the socket even if the application still owns the service object.

`RawProducerServient::emit_event` is a distinct application capability. It checks
the current qualified Event mapping, backing, policy and validator before native
admission. Refusal returns the original data. Accepted work retains the original
instance/mapping, actual payload and capacity through native retirement and
settlement acknowledgement. A full slot returns Backpressure; there is no hidden
queue, coalescing, subscriber replication or loss disguised as a newer Event.

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

A complete Producer-to-Consumer Event path still needs TD-driven Consumer
notification preparation with the actual selected Binding/services, a real native
Consumer subscription participant, establishment and per-item data validation,
independent logical permission/cursors, unsubscribe/stop and retained cleanup.
The existing raw Consumer notification kernel supplies lifecycle roles, not that
integration. General DataSchema/security, QoS 1/2, TLS, reconnect/loss policies,
dataResponse with actual response cardinality, collections and constrained
profiles remain separate work. Native peer subscription/unsubscription here
does not claim the missing Engine Consumer path.
