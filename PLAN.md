# ClinkZ-WoT roadmap

Design target: v6.0 semantic Engine over Binding-owned native runtimes, selected
by ADR-0025 and active on master. Architecture activation does not transfer
implementation acceptance or release qualification.

The enduring [product goals](docs/architecture/00-system-goals-and-context.md)
remain unchanged. Release target remains v1 with both WoT roles and
Property/Action/Event capability families, described collections, security,
client-only Discovery/Directory, portable shared semantics and Host HTTP/HTTPS
production delivery. Zenoh/pico and real embedded protocol/hardware qualification
remain separate future work.

## Delivery strategy: reusable, composable Engine execution

The near-term engineering objective is **complete the reusable, composable WoT
Engine execution architecture**, starting with checked reuse of eligible prepared
execution content across independently bound logical Thing instances. Basic
Property/Action/Event and described collection coverage in both roles has reached
a meaningful milestone. The real execution paths below are the foundation for
this work; they do not establish architecture completion or v1 qualification.

Separate invariant preparation from instance binding and accepted interaction
work without prescribing a universal program format or new public type hierarchy.
Broader DataSchema validation and security services must develop on that correct
boundary, rather than make isolated schema constraints the next major objective.
Use existing real Bindings to expose compatibility and ownership assumptions,
then refine the shared architecture with executable evidence.

### Capability coverage

This coarse coverage is grounded in the linked source and integration fixtures.
The original baseline was recorded at
`dbf81f55536b8789c253b1f90199cee85d3ed6af`; implemented increments update their
affected cells. Source and scoped evidence remain the availability authority;
this is not a package acceptance record.

**Semantic representation:** [TD models and shared interpretation](td/src/semantic.rs)
cover all three families and Thing-root collection Forms, including operation
applicability/defaults and distinct schema roles. The
[operation vocabulary](core/src/operation.rs) represents all 18 TD 1.1 operations.
These facts do not establish executable support; schema and extension processing
still have supported-subset limits.

| Capability | Raw execution kernels | TD-driven Consumer composition | TD-driven Producer composition | Actual Binding I/O evidence |
| --- | --- | --- | --- | --- |
| Property read | Finite Consumer/Producer paths | Owned preparation and captured services | Qualified final TD and local publication | HTTP in both roles; independent test datagram Consumer |
| Property write | Finite Consumer/Producer paths | Operation-scoped value schema, captured Binding/data/access services | Complete qualified final TD, captured input/output validation and local publication | Engine Consumer-to-Producer HTTP PUT/204; Read/Write on one Property/listener; downstream test datagram Consumer |
| Property observe / unobserve | Consumer notification conversation/stop and typed Producer value publication | Selected observable Property Form and captured value/access services | Qualified observable sources, value mappings and publication services | Engine Producer-to-Engine Consumers through Mosquitto; mapped unobserve, independent authority and final release; downstream TD-prepared test datagram Consumer |
| Action invoke | Separate finite Consumer and continuing invocation owners; Producer continuing store | Explicit synchronous subset plus retained continuing invocation with captured mappings/services | Continuing HTTP mapping with captured validation and controls | Engine Consumer-to-Producer HTTP acceptance, pending and validated completion; independent downstream datagram Consumer |
| Action query / cancel | Retained Consumer/Producer invocation controls | Independently selected described Forms, opaque native continuation and original instance authority | Qualified described HTTP control routes | Engine Consumer-to-Producer HTTP query/cancel, uncertainty, replacement and final release; distinct downstream invoke/control Bindings over real datagrams |
| Event subscribe / unsubscribe and Producer emission | Consumer Event conversation; distinct Producer emission and settlement | Selected MQTT Event Form, captured services, validated delivery and logical stop/unsubscribe | Qualified MQTT Event sources and emission; broker-native subscribe/unsubscribe without control data or dataResponse | Engine Producer-to-Engine Consumers through Mosquitto, shared native subscriptions, independent logical authority and final release; independent wire peers also exercised |
| Thing-root ReadAllProperties | Finite root operation with derived readable Property map | Actual root Form/Binding and captured schema/data/access services | Complete final-TD qualification and a distinct root behavior | Engine Consumer-to-Producer HTTP GET; multiple Properties in one native request, schema/refusal/lifecycle/resource cases |
| Thing-root ReadMultipleProperties | Finite root operation with retained readable schemas and separate per-call member set | Actual root Form/Binding; captured request encoder and exact selected-map validator | Complete final-TD qualification; original input supplied to root result validation | [Engine Consumer-to-Producer tests](protocol-bindings/protocols/http/tests/read_multiple_properties.rs): explicit HTTP POST/200 JSON name array; different subsets from one TD, malformed/partial results and lifecycle/resource cases |
| Thing-root WriteAllProperties | Finite root write with independent writable input schema and optional bounded member report | Complete map validation and group authorization before native effects | Complete final-TD qualification, captured validator and one root behavior | [Engine Consumer-to-Producer HTTP PUT/204](protocol-bindings/protocols/http/tests/write_all_properties.rs); real partial effects, uncertain completion, no replay, cancellation/replacement and final release |
| Thing-root WriteMultipleProperties | Finite root write with retained per-call writable set and optional bounded member report | Actual root Form/Binding, captured map validation and set-aware permission | Complete final-TD qualification and one distinct root behavior | [Engine Consumer-to-Producer HTTP PUT/204](protocol-bindings/protocols/http/tests/write_multiple_properties.rs); changing subsets, partial effects and lifecycle/resource evidence; independent test datagrams exercise richer member/atomicity reports with TD-driven Consumer and raw Producer |
| Thing-root ObserveAllProperties / UnobserveAllProperties | One root notification authority, independent cursor/stop and complete named Property maps | Explicit collection kind, selected root Form, original readable schemas and captured map/access services | Complete final-TD qualification and a distinct root publication source | [Engine Producer-to-Consumers through Mosquitto](protocol-bindings/protocols/mqtt/tests/observe_all_properties.rs); one native subscription, member validation, shared stop, loss/replacement, late work and physical release |
| Thing-root QueryAllActions | One root status observation over original Producer invocation owners; finite Consumer settlement | Selected root Form, named Action output schemas and actual status/access services | Complete final-TD qualification, root policy/route and complete native inventory | [Engine Consumer-to-Producer HTTP GET/200](protocol-bindings/protocols/http/tests/query_all_actions.rs); concurrent same-Action instances, native status identity, failed/retained outcomes, authority, slice-only Consumer/Producer results and original funding through final owner disposal |
| Thing-root SubscribeAllEvents / UnsubscribeAllEvents | One root conversation with per-occurrence Event identity and independent logical stop | Selected root Form, exact named Event roles and actual captured data/access services | Complete final-TD qualification and distinct AllEvents source/route | [Engine Producer-to-Consumers through Mosquitto](protocol-bindings/protocols/mqtt/tests/subscribe_all_events.rs); one native wildcard subscription, heterogeneous schemas, adversarial identity, loss/replacement, uncertain controls, borrowed payload/name delivery and original funding through disposal |

The [continuing Action integration](protocol-bindings/protocols/http/tests/action_consumer.rs)
proves the bounded HTTP JSON/type-only, endpoint NoSec Consumer-to-Producer
mapping. Acceptance, pending execution, validated completion, local cancellation
and remote cancellation uncertainty remain separate. Described controls use
independently selected actual registrations; native continuation and original
services survive the initiating exchange. The
[downstream Action fixture](tools/architecture-fixtures/open-consumer/tests/action.rs)
adds real datagrams, different invoke/control Binding implementations and funded
late validation/retirement evidence. These are Host/manual execution claims;
general security/schema, constrained execution and production qualification remain open.

The [Property Write integration](protocol-bindings/protocols/http/tests/property_write.rs)
proves the bounded JSON/type-only, endpoint NoSec HTTP subset, including
pre-effect validation/authorization, ambiguous response refusal, cancellation,
replacement, retained late outcomes and final release. Unsupported required
schema/security assertions refuse preparation; no automatic uncertain-write
replay or readback confirmation is claimed.

Here **Open** means no joined implementation for that role/operation, even when
a raw primitive exists. Finite read/write evidence is in
[native ingress](servient/finite/tests/native_ingress.rs); composed Producer
read/Action evidence is in
[composed Thing](protocol-bindings/protocols/http/tests/composed_thing.rs) and
[HTTP Actions](protocol-bindings/protocols/http/tests/action_producer.rs).
[Producer Events](protocol-bindings/protocols/mqtt/tests/producer_events.rs)
exercise the [declared MQTT subset](protocol-bindings/protocols/mqtt/README.md)
through complete TD qualification and native broker delivery.
[Consumer Events](protocol-bindings/protocols/mqtt/tests/consumer_events.rs)
join the existing Producer to TD-selected Consumer subscriptions, per-item
validation, stop/unsubscribe and cleanup, with native matching/distribution and
shared connection ownership inside MQTT. Retained deliveries are discarded with
visible loss; no replay or reciprocal response capability is claimed.
[Property observation](protocol-bindings/protocols/mqtt/tests/property_observation.rs)
reuses the notification owner with Property value schemas, future-only native
values and mapped unobserve. Establishment performs no Read, invents no initial
value and grants no Event response right. Its declared loss policy reports drops
or terminates, without coalescing. [Property qualification](protocol-bindings/protocols/mqtt/tests/property_qualification.rs)
checks actual sources, full advertisement, supported schema/mapping and selected
Consumer services before effects.
[Collection observation](protocol-bindings/protocols/mqtt/tests/observe_all_properties.rs)
adds future-only complete readable maps on one exact root topic. Named members
retain their individual schemas; one root conversation never becomes N leaf
observations. [Qualification](protocol-bindings/protocols/mqtt/tests/collection_qualification.rs)
checks root support, full member meaning, real services and final advertisement.
The [declared mapping](protocol-bindings/protocols/mqtt/README.md#thing-root-property-observation)
requires a combined observe/unobserve Form and does not provide deltas, atomic
snapshots, arbitrary wildcard extraction or fleet-scale evidence.
[Event collection subscription](protocol-bindings/protocols/mqtt/tests/subscribe_all_events.rs)
adds one native wildcard interaction with retained individual Event identity and
schemas, independent cursors and stop, explicit control/response refusal and
original resource release. Its [qualification](protocol-bindings/protocols/mqtt/tests/event_collection_qualification.rs)
and [mapping](protocol-bindings/protocols/mqtt/README.md#thing-root-event-subscription)
are bounded Host/manual NoSec, type-only, non-retained QoS 0 evidence. It adds no
arbitrary wildcard, general response-channel, replay or fleet-scale claim.
[Combined collection evidence](protocol-bindings/protocols/mqtt/tests/collection_coexistence.rs)
qualifies both root families on one Thing, with distinct same-named Action/Event
schemas, independent HTTP/MQTT lifecycles and original funding through result disposal.
[Downstream Consumer composition](tools/architecture-fixtures/open-consumer/tests/support/composition.rs)
exercises prepared finite I/O and TD-selected Property observation with an external test Binding; the
[notification fixtures](servient/finite/tests/notification.rs) exercise Event
lifecycle semantics in memory. Test Bindings are not production protocols.

**Production qualification:** narrow HTTP Property Read evidence retains its
[recorded scope](docs/evidence/WP-600-http-consumer-property-read.toml).
Current joined paths are bounded Host/manual, type-only schema and NoSec subsets;
their real I/O does not establish complete family, general DataSchema/security,
HTTPS, constrained-profile or v1 qualification. Those remain delivery work.

### Engineering priorities and dependencies

| Priority | Product and architectural outcome | Evidence appropriate to the outcome |
| --- | --- | --- |
| 1 — Reusable definitions and checked instance binding | Prepare eligible invariant content once and bind independent logical Things with their actual endpoints, services, policy, authority and accounts. Retain existing correct interaction and native ownership boundaries. | Existing real interaction paths demonstrate actual shared content, complete selected support, incompatible-bind refusal, independent revocation/replacement, late results and exact final release. |
| 2 — Shared semantic and secure execution services | Broaden DataSchema/media/response meaning and TD-described security on the definition/instance boundary; deliver Host HTTPS, authentication, authorization and transport trust. | Supported assertions and security branches execute through actual captured services and native peers, without source-TD callbacks, silent weakening or credential leakage. |
| 3 — Composable runtime and supported profiles | Extend checked reuse and application composition across finite, Action, notification and Producer capabilities; prove bounded progress, native sharing and portable representations. | Mixed real Binding work retains separate lifecycles and authority; Host/static/no-atomic evidence is scoped to supported capabilities. Measure preparation, per-instance and active-work costs before selecting caches, indexes or scheduling changes. |
| 4 — Discovery/Directory clients | Compose bounded trusted retrieval, links/continuations and TD admission as an explicit client workflow. | Real endpoints and Directory peers, destination/credential policy, input/result/work limits, cancellation and final cleanup. |
| 5 — Production and v1 integration | Qualify the retained role/operation, Host HTTP/HTTPS, security, Discovery and portable shared-layer scope through WP-700. | Independent interoperability, conformance, resource/performance, feature/profile and release evidence for the exact advertised scope. |

Priorities express concentration, not sequential package gates. Security/HTTPS,
Discovery and portable-profile work can advance with available dependencies;
they need not wait for universal reuse or all-family architectural closure.
Basic operation evidence remains valuable without closing a package or milestone.

### Near-term capability growth and priority

The roadmap distinguishes shared semantic definitions and validation meaning;
Binding-specific compiled artifacts with their actual compatibility dependencies;
per-instance identity, endpoint operands, policy, authority and accounting;
per-interaction lifecycle and result ownership; and Binding-owned native
infrastructure that may be shared independently of Thing cardinality. These are
responsibility and lifetime distinctions, not five mandatory public abstractions.
Detailed contracts remain in [Planning](docs/spec/planning.md),
[Core](docs/spec/interaction-core.md), [Binding SPI](docs/spec/binding-spi.md),
[Runtime safety](docs/spec/runtime-safety.md) and [Foundation](docs/spec/foundation.md).

The first increment should demonstrate checked reuse in the existing Host HTTP
ReadAllProperties path: share its eligible prepared member/data meaning across
independent Consumer instances, qualify each actual endpoint/native mapping,
and retain separate policy, capacity, calls and results. Exact ownership,
compatibility analysis and falsifiable acceptance are scoped in
[Review 09](docs/reviews/review-09-engine-maturity-and-roadmap.md). This establishes
the boundary using existing validation, without requiring new DataSchema
constraints, automatic caches or universal native-artifact reuse. An endpoint
embedded in a native artifact cannot become a free instance operand by assertion.

Architecture acceptance requires an actual reusable-content/instance-binding
seam in the preparation-to-execution handoff, not merely an `Arc` around a
Property member table, an interned copy of fully prepared per-instance output,
or two unchanged runtimes reading different URLs. The shared content must be
prepared independently of instance authority; each instance must separately
qualify and bind actual native mappings, policy, operands and funding without
repeating that shared semantic preparation. The boundary must remain suitable
for later shared DataSchema/security meaning without making a protocol-specific
or ReadAllProperties-only framework. Demonstrate the distinct lifetimes, resource
owners and measurable sharing benefit; do not require universal native reuse.

Select subsequent increments against remaining product capabilities and the
actual assumptions exposed by those integrations. Preserve distinct Property,
continuing Action, Event/control/response and collection semantics. A native
collection interaction never silently becomes N leaf calls or subscriptions;
sharing content never shares current permission or changes effect certainty.
Complete Producer qualification remains mandatory for the entire retained final
TD, while Consumer partial support still requires complete support of each
selected interaction. Correct local defects as prerequisites where relevant;
do not substitute another isolated repair for the architectural capability.

**HTTP SSE is an optional notification mapping**, selected when it advances the
chosen capability. It is neither a mandatory predecessor nor a standing next
task. Existing HTTP or independent test Bindings with real I/O may be economical
participants; other native interactions may better expose an unproved boundary.
Protocol choice follows capability and interoperability needs, while Host
HTTP/HTTPS remains the selected first production direction.

### Joined Producer composition: retained boundary

Use the existing direct real-HTTP finite Producer implementation as an economical
integration participant where suitable, not as the universal SPI model or as a
mandatory protocol-specific predecessor. Another supported Binding is equally
valid if it better exposes an architectural defect. A deliberately limited TD is
permissible only if its **entire final retained description** is supported and
qualified; silently dropping unsupported Forms/operations is not a solution.

An integrated Producer claim requires the following observable composition:

1. TD interprets common WoT semantics, resolves extension vocabulary IRIs and
   preserves extension metadata; concrete Bindings interpret protocol
   vocabularies. Planning coordinates selection of **actual** registered
   Binding/configuration, codec, schema, security and behavior services and
   pure construction of owned source-independent execution facts.
2. Native preparation and closed installation retain real physical owners,
   acquire no application-dispatch authority, and allow endpoint-derived facts
   to be checked against the final TD.
3. Before Serving, the final retained TD, all effective operations/Forms,
   behaviors, services, collisions and required readiness pass complete
   qualification. A single local, non-fallible publication commit binds the
   final description and current logical authority; prepublication ingress
   cannot dispatch.
4. Real native ingress passes fresh recipient, policy, payload and capacity
   checks; Servient captures the actual behavior; validated result and native
   delivery retain original identity, effect certainty and owners.
5. Refusal, cancellation/timeout, replacement, late results and close exercise
   first-cause retention, bounded cleanup/acknowledgement and actual final
   physical release. A working HTTP exchange or raw local activation alone
   proves none of the missing publication obligations.

Broaden this foothold in independently useful changes. Prove each claimed
boundary with proportionate tests; do not invent a global all-protocol proof
gate, a universal runtime/session/RPC abstraction, or certification paperwork
for unimplemented combinations. If existing preparation or runtime APIs impede
the joined path, replace them rather than maintain compatibility facades.

### Continuing capability growth

Operation-scoped preparation and shared instance authority belong to
[WP-200](docs/work-packages/WP-200-planning.md#consumer-operation-composition)
and [WP-400](docs/work-packages/WP-400-servient.md#joined-consumer-execution),
with separate native facets in [WP-300](docs/work-packages/WP-300-bindings.md#consumer-capability-composition).
The optional HTTP observation mapping remains in
[WP-600](docs/work-packages/WP-600-protocol-bindings.md#td-driven-consumer-observation).

Bindings keep native matching, multicast/distribution, correlation, session
sharing, retries and physical infrastructure; the Engine owns independent
logical acceptance and application dispatch.

Introduce a materially different protocol interaction where it tests an
unproven boundary (for example shared broker-native subscription or multi-party
distribution), not to satisfy an arbitrary protocol count. Check third-party
Cargo-linked, startup-registered capabilities without a Core whitelist. Prepared
definitions may be reused under proved configuration compatibility; one Thing
never implies one physical connection, one native subscription or one plan-set.

Shared DataSchema/media validation, security selection and authorization,
resource/work ownership, Host/constrained support and diagnostics accompany
the operations that use them; they are not postponed until an all-family gate.
Client-only Discovery/Directory and Host HTTPS have separate deliverable paths.
Real embedded hardware/transport and Zenoh/pico production remain outside the
selected v1 release but inside the enduring product goals.

Exact package/capability dependencies remain in
[the work-package DAG](docs/work-packages/index.toml). The responsibility
contracts are [Planning](docs/spec/planning.md),
[Binding SPI](docs/spec/binding-spi.md), [Core](docs/spec/interaction-core.md) and
[Runtime safety](docs/spec/runtime-safety.md). This roadmap adds **no** new
architecture authority, API freeze, PR-specific continuation state, mandatory
review gate or milestone status transition.

## Milestones

| Milestone | Status | Completion boundary |
| --- | --- | --- |
| M0 — Repository-grounded governance | CLOSED | Stable ownership and recoverable Git/remote workflow |
| M1 — Architecture authority and closure | IN_PROGRESS | One revision, explicit prior dispositions, checks and independent review |
| M2 — Foundation and semantic Core/TD | IN_PROGRESS | Shared semantics, data/security, owner/work/time and supported profiles |
| M3 — Preparation | IN_PROGRESS | Actual pairing, pure/deferred construction, coverage and reuse obligations |
| M4 — Binding capability/lifecycle | IN_PROGRESS | Open native facets, recipient authority and physical retirement |
| M5A — Servient runtime | IN_PROGRESS | Logical publication, dispatch, accepted work and cleanup |
| M5B — Discovery client | OPEN | Bounded trusted retrieval/continuation and exact client scope |
| M5C — Host HTTP/HTTPS production | OPEN | Real supported mapping and security/lifetime qualification |
| M6 — Integration/conformance | OPEN | Joined capability, profile, resource/performance and removal evidence |
| M7 — v1 release | OPEN | Independent technical release acceptance after M6 |

No milestone closes because this target is accepted or historical CI is green.
Source, tests and exact review evidence establish implemented/admitted capability;
GitHub owns live branch/PR/CI state. This roadmap stores no task continuation,
execution checklist or next action.
