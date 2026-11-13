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
Engine execution architecture** through correct protocol/Engine authority,
independent logical associations, checked reusable preparation and separately
funded admitted work. The finite Host Consumer now has managed TD-ID
registrations, non-revoking association views and independent `ConsumerCalls`
capacity (PR #276); this completes a scoped finite handoff, not continuing
Action/notification composition, Producer authority extraction or all-profile
execution. Basic Property/Action/Event and described collection coverage in
both roles has reached a meaningful milestone. These real execution paths do
not establish architecture completion or v1 qualification.

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
The [finite structured DataSchema increment](docs/evidence/structured-dataschema.md)
adds reusable nested object/required-field constraints, integer boundaries and
Unicode string lengths through actual HTTP Consumer/Producer execution. Broader
schema/media/profile coverage remains open.
The [TD security/HTTPS increment](docs/evidence/td-security-https.md) now executes
finite Property Read and structured Property Write through verified TLS.
The [composition correction](docs/evidence/security-composition.md) adds Basic,
header API keys, their conjunction and distinguishable alternatives with actual
independent services, selected Consumer meaning and complete Producer qualification.
Other authentication mechanisms/operation families and production security
qualification remain open.

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
proves bounded structured DataSchema input/output through the HTTP JSON,
endpoint NoSec Consumer-to-Producer mapping. Acceptance, pending execution,
validated completion, local cancellation
and remote cancellation uncertainty remain separate. Described controls use
independently selected actual registrations; native continuation and original
services survive the initiating exchange. The
[downstream Action fixture](tools/architecture-fixtures/open-consumer/tests/action.rs)
adds real datagrams, different invoke/control Binding implementations and funded
late validation/retirement evidence. These are Host/manual execution claims;
general security/schema, cross-instance continuing definition reuse, constrained
execution and production qualification remain open.

The [Property Write integration](protocol-bindings/protocols/http/tests/property_write.rs)
proves the bounded endpoint NoSec HTTP subset, including
pre-effect validation/authorization, ambiguous response refusal, cancellation,
replacement, retained late outcomes and final release. The
[secure Write integration](protocol-bindings/protocols/http/tests/security_write.rs)
adds structured values, verified HTTPS/Basic and independent effect/delivery
evidence. Unsupported required
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
[Structured leaf notifications](docs/evidence/structured-dataschema.md#notification-execution)
apply the common structured DataSchema meaning in both MQTT roles, while actual
collection/control/response/security support remains separately qualified.
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
Current joined paths are bounded Host/manual subsets, mostly NoSec/type-only
with the structured and security increments scoped above;
their real I/O does not establish complete family, general DataSchema/security,
production HTTPS, constrained-profile or v1 qualification. Those remain delivery work.

### Engineering priorities and dependencies

| Priority | Product and architectural outcome | Evidence appropriate to the outcome |
| --- | --- | --- |
| 1 — Producer native/semantic authority boundary | Remove the Binding-to-Servient execution authority leak (Review 10 F1). Binding owns native matching, correlation, I/O and retirement; Servient alone decides logical admission, dispatch and revocation. Begin with the existing real HTTP continuing Action path; do not prescribe a universal RPC/session or a callback facade that secretly forwards Engine authority. | HTTP Action invoke/query/cancel and original continuation/result/cleanup work through native candidates/opportunities and a protocol-neutral Servient handoff. The HTTP Binding no longer depends on `servient-finite` to dispatch application work; failure before/after publication, late outcomes and independent native release remain truthful. |
| 2 — Continuing Consumer notification composition | Extend independent logical association and capacity ownership to established conversations; retain exact per-occurrence target and qualified source/verification context for fresh policy where required. Preserve native sharing/stop/response rights and fix Review 10 F2/F3 at affected service handoffs. | Real MQTT notification conversations remain independently authorized under shared compatible native interests. A concrete evidence-bearing source mapping tests per-occurrence policy without treating an unauthenticated topic as a verified principal; queued delivery, stop, loss, late work and physical cleanup retain their original owners. |
| 3 — Continuing Action and multi-capability reuse | Decouple continuing invocation capacity and checked prepared-service ownership from per-instance Runtime construction. Integrate finite, Action, notification and qualified Producer capabilities under their common logical authority without a mega-Runtime; repair F2/F3 and obsolete adapters as the affected boundaries are changed. | Original invoke/query/cancel mappings and late outcomes survive initial exchange, replacement and retirement. Actual shared content/services are funded once, prepared declarations cannot silently change during runtime construction, and mixed HTTP/MQTT capabilities retain independent progress and cleanup. |
| 4 — Secure execution and Discovery clients | Broaden actual DataSchema/media/security execution, Host HTTPS and explicit bounded trusted Discovery/Directory retrieval where their dependencies are ready; these can proceed independently of the cross-family sequence. | Supported assertions, security branches and trusted retrieval execute through actual providers and peers without source-TD callbacks, silent weakening, credential leakage or fabricated interoperability claims. |
| 5 — Product integration and supported profiles | Converge practical application composition, Host HTTP/HTTPS delivery, selected portable/static representations and scoped v1 qualification without prematurely freezing a universal facade or an automatic scheduler. | Supported role/operation/profile combinations have actual integration, conformance, resource/performance and lifecycle evidence for the advertised scope; unsupported combinations remain explicit. |

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

The ownership direction in
[Review 10](docs/reviews/review-10-engine-execution-architecture.md) supersedes
the earlier ReadAllProperties-first implementation sequence. Existing checked
content/mapping reuse is a foundation, not a reason to preserve per-instance
Runtime containers or prepaid idle Call Slots. Persistent usability must be
independent of simultaneous execution capacity. Each admitted obligation retains
its original instance, checked mapping, operands, accounts, result and cleanup.
An endpoint specialized into a native artifact remains specialized.

The finite Consumer association/capacity separation is implemented by PR #276:
the managed owner resolves TD IDs/configurations without retaining application
handles or prepaying vacant finite call slots. It still provides neither a
universal multi-family owner nor automatic scheduling/physical drain.

Prioritize correcting **Producer execution authority (Review 10 F1)** before
broadening Producer composition. Current HTTP Action and MQTT native runtimes
receive `RawProducerServient` and can invoke, acknowledge or revoke logical
work; this contradicts the Binding/Servient dependency and authority boundary.
Use real HTTP Action invoke/query/cancel to establish native candidates,
opportunities and failure handoff into Servient-owned authorization, dispatch
and revocation. Preserve original native correlation, continuing Action and
result/cleanup ownership. Remove the actual reverse dependency; do not simply
hide it in a dispatch callback or add a universal RPC/session abstraction.
The precise trait and carrier design must follow source-level implementation
analysis; the responsibility decision does not preselect its ABI.

Then extend the same independently retained association/capacity principles
where continuing Consumer Notification and Action lifetimes need them. Original
notification occurrence context must reach the relevant fresh policy without
equating a native topic with authenticated identity; compatible native sharing
must keep independent logical rights and physical leases. Repair F2/F3 shared
service charging and prepared-declaration fencing when those handoffs are
changed. Preserve family-specific state and explicit application scheduling;
do not build a mandatory mega-Runtime, automatic wake subsystem or global
subscription pool. These are independently useful capabilities, not an
all-family migration gate.

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
| M5B — Discovery client | IN_PROGRESS | Bounded trusted retrieval/continuation and exact client scope |
| M5C — Host HTTP/HTTPS production | OPEN | Real supported mapping and security/lifetime qualification |
| M6 — Integration/conformance | OPEN | Joined capability, profile, resource/performance and removal evidence |
| M7 — v1 release | OPEN | Independent technical release acceptance after M6 |

No milestone closes because this target is accepted or historical CI is green.
Source, tests and exact review evidence establish implemented/admitted capability;
GitHub owns live branch/PR/CI state. This roadmap stores no task continuation,
execution checklist or next action.
