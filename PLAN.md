# ClinkZ-WoT roadmap

The v1 delivery target is a Rust Engine that consumes and exposes Things through
Properties, Actions, Events and described collections, with secure interactions,
client-side Discovery/Directory, portable shared semantics and Host HTTP/HTTPS
production delivery. Zenoh/pico and real embedded protocol and hardware
qualification have separate delivery horizons. [Enduring product goals](docs/architecture/00-system-goals-and-context.md)

## Delivery strategy

One application should be able to publish independent Things on a shared service,
consume Things through unrelated Bindings, and continue working while earlier
results await collection. TD owns shared meaning. Planning qualifies a usable
Consumer interaction or the entire final Producer description against actual
services. Core defines the interaction, data and security semantics. Servient
owns instance authority, publication, application dispatch and admitted work.
The Binding owns native matching, correlation, distribution and physical release.
Compatible instances can share compiled instructions and a pool of calls.
Servient checks the original instance’s current permission. Active work, native
reply rights and retained results each have their own release point. [ADR-0025](docs/ADRs/0025-semantic-engine-and-binding-runtimes.org) · [Selected Engine target](workspace/0132-engine-lifecycle-design.md#ownership-and-lifetime)

### Capability coverage

| Capability | What an application can exercise | Remaining foundation work |
| --- | --- | --- |
| Consumer choice and instance authority | A managed Host Consumer can retain named finite mappings and draw from a separate call pool. Continuing Actions select a complete Invoke/Query/Cancel combination in Form order across compatible registrations. Events can select either of two independent Bindings. Both searches account for competing providers under one effective configuration. [Managed Consumer](servient/finite/src/consumer_managed.rs) · [HTTP Query Form orders](protocol-bindings/protocols/http/tests/action_consumer.rs) · [Downstream Action](tools/architecture-fixtures/open-consumer/tests/action.rs) · [Event selection](tools/architecture-fixtures/open-consumer/tests/event_selection.rs) | Continuing preparation still filters some URI-variable, response and security meaning before actual providers can qualify it. Repeated preparations from one admitted TD need a shared source of resolved facts. |
| Producer publication | On Host, qualification rejects empty affordance Forms and checks each advertised operation, behavior and provider. Closed setup verifies the exact native members and endpoints. Changes in eligibility or readiness can prevent commit. A single MQTT participant serves 33 funded Event mappings with the finite pool occupied. Mapping and participant shortages refuse independently. [Host publication](servient/src/producer_composition.rs) · [Coherence and source-free commit](protocol-bindings/protocols/http/tests/composed_thing/coherent.rs) · [Mapping capacity](protocol-bindings/protocols/mqtt/tests/support/notification_capacity.rs) | The draft-to-staging payer, concurrent candidates on one bank, and peak versus post-commit backing need evidence. The installation interface uses Host ownership and synchronization. |
| Finite Properties and collections | Both roles execute HTTP Read/Write and root ReadAll, ReadMultiple, WriteAll and WriteMultiple. Direct verified HTTPS supports selected data and security combinations. With shared Host capacity an expired, accepted HTTP PUT can release its connection while the same Thing and a sibling keep serving and the caller keeps its outcome. [Root ReadMultiple](protocol-bindings/protocols/http/tests/read_multiple_properties.rs) · [Root WriteMultiple](protocol-bindings/protocols/http/tests/write_multiple_properties.rs) · [Structured values](docs/evidence/structured-dataschema.md) · [Composed security](docs/evidence/security-composition.md) · [Retirement over HTTP](protocol-bindings/protocols/http/tests/support/accepted_timeout.rs) | Each finite Producer participant still drives one native exchange. Phase-specific cleanup, checked outgoing data and authorization through the actual credential use remain incomplete. |
| Continuing Actions | HTTP Invoke, Query and Cancel retain the original invocation across exchanges. QueryAllActions observes a root inventory with per-invocation identity, output and visibility. A source-free Consumer can drive an invocation with caller-owned storage. [HTTP root status](protocol-bindings/protocols/http/tests/query_all_actions.rs) · [Caller-owned Action](docs/evidence/continuing-action-runtime-independence.md) | Producer retention reads process time. Aggregate handle construction starts before its work is charged. The Consumer close path still prepays access and validation work. |
| Observations and Events | Both roles execute MQTT observe/unobserve, subscribe/unsubscribe and the corresponding root collections. One root conversation preserves member identity and schema. The broker owns distribution. A completed QoS 0 packet releases the shared writer while its result is held. A downstream Consumer can reply to an occurrence. [Root Properties](protocol-bindings/protocols/mqtt/tests/observe_all_properties.rs) · [Root Events](protocol-bindings/protocols/mqtt/tests/subscribe_all_events.rs) · [Writer lifetime](protocol-bindings/protocols/mqtt/tests/producer_events.rs) · [Occurrence replies](tools/architecture-fixtures/open-consumer/tests/notification.rs) | Application-controlled Producer subscriptions need setup acceptance, a returned control operand and per-delivery reply ownership in both roles. |
| Definition reuse and deployment | HTTP Read in both roles can skip actual invariant lowering when binding a compatible instance. Mixed Host turns drive both roles. Caller-owned finite Read and continuing Consumer Action execute with supplied storage and time. [Consumer costs](docs/evidence/consumer-execution-reuse.md#measured-costs-and-benefit) · [Producer reuse](docs/evidence/producer-preparation-reuse.md) · [Host turns](docs/evidence/host-execution-integration.md) · [Caller-owned Read](docs/evidence/constrained-finite-execution.md) | Continuing definitions can still specialize endpoints. A second mapping, costs on the normal deployment turn, and caller-owned Producer and notification lifetimes remain open. |

## Engineering priorities and dependencies

Finite Producer admission can use already qualified inputs. Consumer
qualification can use existing finite and continuing interactions. They meet
when a Producer accepts a callback subscription that its Consumer can later
control. [Dependencies of the selected design](workspace/0132-engine-lifecycle-design.md#convergence-by-dependency)

### Near-term capability growth and priority

**Publish a Thing with caller-owned storage and retire each admitted finite
exchange independently.** A serial HTTP participant can discard one expired
reply. The general driver still keeps its ingress, reply, deadline and active
permit on the participant. It checks result-queue credit before entering native
retirement.
The publication interface also carries Host owner tokens, reference-counted gates
and retained owners. Increasing the pool size leaves both constraints in place.
A caller-owned Producer tests the two decisions together before more families
come to depend on them. [Finite driver](servient/finite/src/host.rs) · [Installation interface](core/src/publication.rs) · [Current finite capacity contract](docs/spec/runtime-safety.md#finite-producer-participation-and-work-capacity)

Use a curated, source-free Property mapping with a complete final TD, caller
slots, supplied monotonic time and a downstream Binding that lends native
buffers over real I/O. Install closed and check the prepared coverage against
the actual deployment and its ready members before publishing. Every dispatch
needs current membership, permission, behavior and capacity. Under pressure
the original owner keeps the unaccepted offer, reply right and deadline. Two
accepted requests on one participation can complete out of order. One can
retire with native and cleanup credit while its sibling stalls. A later turn
can hand off the original result with its first cause, effect and delivery
evidence. Revoking the Thing fences new work. Dropping an old result after
replacement releases only its own backing. Compare moving an opportunity,
retaining a funded loan, and copying into funded result storage where the
native lifetime requires it. Use
the comparison to settle the common handoff and revise the affected SPI and
runtime contracts before migrating the composed Host path. HTTP can keep its
serial native policy. [Selected admission and release boundary](workspace/0132-engine-lifecycle-design.md#admission-and-release)

### Qualify the interaction through its first effect

Lend resolved TD facts to the actual Binding, codec and access service. Let
each qualify its part of a complete interaction, including control Forms,
response alternatives and all required security operands. Correct the shared
resolver's rejection of the standard 2019/2022 context pair at TD admission.
[Continuing preparation](docs/spec/planning.md#continuing-consumer-action-preparation)

Carry the generating party and data role into validation. For the same object
bytes, a Consumer must accept a permitted additional field on Read and refuse to
send it on Write. This requires revising Core's direction-independent validation
contract. For credentials, distinguish the logical target, next hop and
authorized audience at each use. Use two controlled peers to observe where each
credential goes. Rotate a provider immediately before entry to determine whether
an immutable snapshot suffices or the provider must serialize the check and
effect. Recheck permission after reentrant callbacks and when a control or
callback supplies its destination. [Data and security contracts](docs/spec/interaction-core.md) · [TD 1.1 data behavior](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#behavior-data)

### Carry admitted work across exchanges

- **Continuing Actions.** Use the existing invocation and its original control
  mappings to separate paid remote Query/Cancel from local retirement. Finite
  Consumer and continuing Action close each need to progress with native and
  cleanup credit, then validate or hand off the late value when that phase is
  funded. Supply the Producer retention clock and pay for aggregate inventory
  work before allocating handles or looking them up. This can advance on the
  existing Action path. [Core drivers](core/src/execution.rs) · [Action progress](core/src/execution/action.rs) · [Producer store](servient/finite/src/action.rs)
- **Two-ended subscriptions.** Join qualified setup and control to Producer
  admission and retained work. The Consumer offers its callback address. The
  Producer can deny it or accept a funded obligation and return the ID used
  for separately authorized cancellation. A missing or malformed ID preserves
  remote uncertainty. Validate the occurrence and any application reply with
  their own target, authority and owner. A delayed cancellation must leave a
  same-subscriber successor on the same route able to receive and stop with its
  own ID. Retain broker-owned fan-out, one root control across distinct members
  and explicit refusal when required source evidence is unavailable. Run the
  representative notification/response lifetime with caller-owned storage as
  well as Host driving. [Consumer participation](core/src/notification.rs) · [Producer emission](core/src/publication.rs) · [Selected subscription contract](workspace/0132-engine-lifecycle-design.md#when-a-producer-accepts-a-subscription)

### Fund preparation, measure reuse and migrate callers

Whole-publication correctness gives staging a stable observable outcome.
Account for provisional preparation, closed installation and rollback through
their actual last use. Concurrent candidates on one bank must be able to
commit or refuse independently. Measure reservations and actual backing during
staging and after commit before choosing shared scratch. Keep candidate funding
separate from the serving instance, the work it accepts and the resources a
Binding can share. [Producer preparation](planning/src/producer_composition.rs) · [Resource ownership](docs/spec/foundation.md)

Measure compilation reuse as soon as a mapping's invariant inputs are known.
Compare fresh specialization with checked reuse on HTTP and a worthwhile second
mapping, for example MQTT root Event member/filter instructions bound to
different prefixes. Count actual lowerings and compare source-to-ready work,
peak and retained backing, warm dispatch and the ordinary deployment turn for
one and many instances. Hold active work fixed while varying idle Things.
Use the comparison to decide where specialized mappings or indexes pay for
themselves. [Measured HTTP baseline](docs/evidence/consumer-execution-reuse.md#measured-costs-and-benefit) · [Scale criteria](workspace/0132-engine-lifecycle-design.md#compilation-reuse-and-the-cost-of-scale)

Migrate ordinary callers with each usable successor. Consumers need named
operations and a bound Read fast path on checked associations with separate
capacity. Producers need behavior binding, qualified publication and handles
for admitted work. Migrate restricted and static callers before retiring their
Read façade, Plan-Set and route-stage builders. Retain the raw layers used by
composition. Foundation completion requires the joined public paths, consistent
domain contracts, representative Host and caller-owned execution, and independent
review at one revision, including the execution-only dependency graph and
the supported no-pointer-atomics build. [Public caller census](docs/reviews/review-11-engine-architecture-audit.md#public-api-and-caller-census) · [Foundation completion criteria](workspace/0132-engine-lifecycle-design.md#foundation-completion-and-product-development) · [Work-package dependencies](docs/work-packages/index.toml)

Bounded trusted Discovery/Directory retrieval and disjoint Host HTTPS work can
advance on their own dependencies. Product webhook hosting, proxy support,
authenticated notification profiles, strict ingestion and sustained workload
qualification require evidence for the advertised deployment. The selected
baseline permits a service gap for an exclusive route that must close before
replacement. A v1 commitment to continuous service through failed replacement
is an open product decision. [Discovery client evidence](docs/evidence/remote-discovery.md) · [Deployment decisions](workspace/0132-engine-lifecycle-design.md#decisions-that-still-need-evidence)

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
