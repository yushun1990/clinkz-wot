# WP-600 — Concrete protocol integration — v6.0

## Target and dependency

Continue Host HTTP/HTTPS production direction; add useful supported capabilities against successor contracts. Zenoh/pico remain optional future Bindings. Keep concrete SDK/security/transport policy in the Binding.

Exact package dependencies belong to [index.toml](index.toml). Runtime and optional
preparation are distinct: dynamic preparation integration needs WP-200, while
execution-only composition does not. Package completion is not a blanket start
barrier; exact available predecessors and affected authority must support the
chosen slice under ADR-0013. No new runtime tranche is admitted by this migration.

## Authority and completion

Applicable requirements: SPI-COMPOSE-001, SPI-NATIVE-001, SPI-INGRESS-001, SPI-DELIVERY-001, SPI-RETIRE-001, PREP-COVER-001, EXEC-SECURITY-001.

Required evidence follows actual claims: Real peer interoperability, malformed/native input limits, publication, cancellation/late results, resource pressure and final physical release for each advertised mapping/profile. HTTP cleartext evidence is not HTTPS/security qualification; no hardware claim from Host tests.

Retained Property Read code/tests and historical tranches remain useful within
their recorded contracts. Their old status does not close this package under
v6. Exact successor source scope/API/state/resource changes, removals, checks and
independent review are determined from the current repository for each material
slice, not from a saved next-action list. New interaction families and Bindings
are outside this architecture migration.

## HTTP Producer data prerequisite

The JSON/type validator is shared in `codecs/json`; moving it out of HTTP does
not broaden its supported DataSchema subset or retire its existing regression
obligations. MQTT Event data uses the same actual decoder/type service.

The existing single-route HTTP service implements the prepared type-only schema
subset from WP-200 using its actual JSON decoder and Core's value-type relation.
The source-independent native program retains the assertion. Serving requires an
actual `ProducerOutputValidator` facet, borrowed from the captured provider/program;
numeric service limits alone cannot establish it. Installation-only providers
omit this facet. Schema/encoding
failure becomes a Core validation error before native delivery and maps to HTTP
500 with JSON null. The generic shape seal remains a separate check.

Preparation refuses unimplemented schema assertions before native setup. The
former extension-only recursive schema acceptance is removed, so nested/schema
facets accepted at older revisions are not inherited as supported behavior.
The HTTP policy/compatibility revision changes with this narrower executable
contract. Core/HTTP runtime-only dependency cuts remain unchanged.

Evidence is the real-peer `producer_serving` and pure `producer_preparation`
targets, with the existing installation/funding/Consumer loop regressions. The
changed codec runs under the existing 4096-byte result limit and manual dispatch
allowance; native parser/SDK backing retains its existing Host deployment policy.
No general validation performance, constrained execution or SDK maximum is claimed.
Historical evidence keeps its original revisions and no completion state changes.

This prerequisite alone does not implement qualified publication from the v6
shared logical runtime. Complete multi-participant qualification, aggregate
admission, final endpoint/dependency revalidation and the Servient-local final-TD
commit remain required under SAFE-PUBLISH-001.

## Direct shared finite HTTP participation

`HttpFiniteBinding` implements `FiniteIngressBinding` directly over the real
nonblocking listener and shared Hyper transport. It uses none of the old
Producer installation, accept-permit, request or sealed-response ABI. The native
owner contains no authority/handler/registry. `HttpFiniteConfig` captures the
owned prepared program/schema, exact native operands, candidate and timeout;
one actual logical behavior can serve independent participants. The runtime-only
normal graph still excludes TD, Planning and Servient.

The concrete limits remain loopback/nonzero HTTP/1.1 bodyless GET, exact path/Host,
one connection/correlation, 16 headers/8192 bytes, 4096 I/O bytes per native poll
and JSON results <=4096 bytes. Readiness keeps semantic acceptance closed.
Malformed native input disposes its connection without dispatch and preserves
the listener. Request timeouts span the original input/reply. Retirement aborts
the exact connection and listener; any started write reports Unknown delivery.
Drop disposes native owners before program/services, without network waits/tasks.
An opaque native issuer and monotonic correlation fence the original reply;
numeric Binding/instance equality cannot substitute a sibling's opportunity.

Raw output also has to fit the concrete HTTP encoding. Malformed JSON, unsupported
media/coding or a body above the native 4096-byte limit refuses before any response
is queued or written, even if the Engine's result envelope is larger. The native
mapping error closes through the retained owner and settles the original value/
effect as NotDelivered. Only an original error result maps to the fixed 500/null;
a substituted error cannot certify delivery of an original successful output.

The `finite_producer` target uses actual HTTP peers and the public Consumer, with
sources destroyed before execution, shared mutable behavior, type/Unicode/exact
integer rejection, zero/short/exact credits, failed setup/cleanup, original-result
retention and socket reuse. The old funded public root and its account/transport
regressions remain restricted independent evidence. It is not wrapped by the new
path; retiring it requires replacement of its funded execution guarantees plus
the successor final-description commit. No old API becomes successor authority.
Neither successful HTTP I/O nor required output validation is qualified TD
publication. Multi-route HTTP matching, HTTPS/security, constrained execution,
aggregate capacity and release qualification retain their separate boundaries.

## Integration priority after the Producer foothold

Whole-Thing composition and the HTTP finite/Action participants now supply
qualified publication with real I/O, within their supported envelopes.
[Open Consumer integration](WP-200-planning.md#open-consumer-integration) also has
an executable finite foothold. Further protocol work follows
[PLAN.md's capability priorities](../../PLAN.md#near-term-capability-growth-and-priority)
across both roles and Property/Action/Event, retaining ownership/publication
regressions. Choose a mapping for the capability or interoperability gap it
addresses; a convenient protocol feature is not its own roadmap priority.
HTTPS/security and other work retain their dependencies and qualification scope.

## TD-driven Consumer observation

TD-driven notification selection/preparation, captured validation/security and
control mappings remain open capabilities in WP-200/WP-400. A raw notification
participant alone does not establish them. Their implementation follows the
roadmap's basic coverage objective; production HTTP streaming is not a
prerequisite for that Engine work. Preserve separate finite and notification
lifecycles. Event subscription/response and Producer notifications retain their
own semantics and delivery increments.

### Optional HTTP SSE mapping

HTTP SSE is one candidate for Consumer Property observation, not the prescribed
next slice or a dependency of other Affordance families. The concrete scope below
applies if this mapping is selected against the roadmap's remaining gaps.

The existing [HTTP compiler](../../protocol-bindings/protocols/http/src/compiler.rs)
accepts only `ReadProperty` without a subprotocol. Its
[transport](../../protocol-bindings/protocols/http/src/transport.rs) validates a
bounded JSON response and collects it to EOF. Reuse suitable HTTP/Hyper I/O
mechanics, not that finite body collector or its total-response wire cap as the
stream model. The Binding needs bounded streaming framing/decoding and retained
native progress/retirement; the Engine keeps logical delivery and visible loss.

Executable evidence for this mapping when selected:

- Consume a TD, select the actual registered observation capability, retain
  resolved URL/method/subprotocol, schema/media/security facts and stop mapping,
  then destroy the source before opening a real loopback TCP stream. Unsupported
  Forms remain explicit; ordinary HTTP reads still run through the same common
  selection path. No hard-coded program bypasses TD preparation.
- Deliver multiple validated Property values while the connection remains open,
  including fragmented UTF-8/lines, multiline data, comments, malformed/oversize
  input and incomplete EOF. Check SSE wire `text/event-stream` separately from
  the supported data encoding/Property schema. Start with a declared cleartext
  NoSec/type-only subset; reject unsupported selected schemas/security before
  effects. Establishment fabricates no initial value and performs no hidden read.
- Bound per-poll work, frame/line/backing and queue items/bytes without a lifetime
  byte cap on a valid continuing stream. Exercise slow consumers, visible
  loss/termination, real peer failure, stop before/after establishment, late data,
  policy/replacement races and lost owners. Retire the original socket/parser
  and release accounts; two live observations demonstrate independent stop.
- Interpret explicit `UnobserveProperty` support or a documented native implicit
  removal mapping; do not invent DELETE from an observe URL. Report local release
  separately from remote cancellation certainty. SSE has no application
  dataResponse; retain the general Event response/cardinality regressions.

Standards basis: [TD 1.1 Recommendation, PropertyAffordance](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#propertyaffordance)
defines observation semantics without guaranteeing an initial value. The
[HTTP Binding draft's SSE example](https://w3c.github.io/wot-binding-templates/bindings/protocols/http/index.html)
is mapping guidance, not a Recommendation or an Engine SPI. Use the
[HTML event-stream parsing rules](https://html.spec.whatwg.org/multipage/server-sent-events.html#parsing-an-event-stream)
for the concrete framing. Record the supported mapping and limits with its code;
do not claim browser EventSource or WoT Profile conformance from this subset.

Redirects, reconnect/replay/Last-Event-ID policy, TLS/authentication, broader
DataSchema, Event subscribe/response, Producer notifications, broker-native
sharing and constrained execution are outside this proposed SSE subset; their
roadmap priority is independent of it. The proposed mapping terminates
observably on disconnect and follows no redirects; it does not promise replay or
remote receipt. These are concrete Binding limits, not new Consumer SPI rules.

## Joined HTTP finite participant

`HttpFiniteProducer` is the optional preparation facet for `HttpFiniteBinding`.
It constructs the direct native configuration with the actual pure HTTP compiler
and rechecks the exact mapping and actual `bound_address` after closed setup.
Its normal runtime graph retains no TD, Planning or Servient dependency.
HTTP protocol vocabulary, GET/path/Host matching, native correlation/reply,
JSON/type checking and physical retirement remain in the Binding.

The first supported joined TD retains one Property read, one Form, endpoint
NoSec, JSON and primary 200 with the existing type-only schema subset. Supplied
nonzero loopback endpoints must match the immutable listen operands. Native
readiness is not publication: closed setup accepts/parses no stream and buffers
only the OS-owned bounded listen backlog requested at 128; backlog saturation
may wait/refuse peers. No native callback receives behavior/dispatch power.
Publication occurs solely at WP-400's final local commit after complete
requalification. Port-0 TD rewriting is refused, not silently substituted.

The public `serve_property` example now uses this joined path. Real raw peers
and the public Consumer in `joined_producer` test final TD truth, original result
and delivery, before/after-publication cancellation, occupied-port failure,
dependency loss, replacement, partial-write uncertainty and retained cleanup.
The original raw HTTP and legacy installer tests continue to test their own
ownership/resource/framing contracts; their prior acceptance does not transfer.
HTTP write completion describes local delivery only. No remote receipt, TLS,
authentication, shared HTTP listener, multi-Form/operation or production
resource/performance qualification follows from these tests.

## TD-driven Producer Event emission

The bounded Host [MQTT Event mapping](../../protocol-bindings/protocols/mqtt/README.md)
adds pure operation preparation, actual startup-registration pairing, closed
native installation and complete final TD qualification to the distinct
Producer Event emission capability. It captures actual JSON/type and NoSec
services and native publication identities before the local commit. Every
advertised Event operation/Form is supported or publication refuses; control
schemas, dataResponse and required response branches are not silently dropped.

MQTT is selected for native broker matching/distribution: one application Event
publication reaches zero or many independent subscribers without Engine fan-out.
One application-owned native service can share a connection across Things;
logical close preserves siblings and pending work pins physical resources.
The shared JSON validator is extracted from HTTP without broadening its subset.
This is basic executable capability evidence, not WP-600 production closure or
a change to the selected Host HTTP/HTTPS release direction.

`cargo test --locked -p clinkz-wot-binding-mqtt --features broker-tests` starts
real Mosquitto peers and exercises actual delivery, native unsubscribe, NoSec/
data refusal, actual service capture/substitution, prepublication fencing,
slot/account saturation, cancellation/deadline, replacement, partial-write
uncertainty, late success, broker failure and final socket/account release.
Default qualification tests need no broker; execution-only compilation retains
no Planning/TD dependency. The runnable `emit_event` example uses the same
qualified public composition.

TD-driven Consumer subscription establishment, selected native Consumer services,
per-item validation, logical cursors/permission, unsubscribe/stop and cleanup
remain required for a complete Engine Producer-to-Consumer Event path. Broader
schema/security, TLS, QoS 1/2, replay/reconnect, Event responses with real native
cardinality, collections and constrained execution remain outside this slice.
