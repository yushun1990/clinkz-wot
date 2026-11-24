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
| Consumer choice and instance authority | A managed Host Consumer can retain named finite mappings and use a separate call pool. Continuing Actions choose a compatible Invoke/Query/Cancel tuple in Form order. Events can select either of two independent Bindings. Both continuing searches account for competing providers under one effective configuration. [Managed Consumer](servient/finite/src/consumer_managed.rs) · [HTTP Query Form orders](protocol-bindings/protocols/http/tests/action_consumer.rs) · [Event selection](tools/architecture-fixtures/open-consumer/tests/event_selection.rs) | Finite preparation can refuse a usable Read because two providers support a later Form. Continuing preparation filters some URI-variable, response and security meaning before actual providers can qualify it. [Finite selection](planning/src/open_consumer.rs) |
| Producer publication | The finite Host entry exposes a supplied TD or resolves its Forms against a setup-assigned base. A Read reaches the advertised HTTP address alongside an independently registered datagram Property. Ambiguous final ownership rolls back the candidate. One MQTT participant serves 33 funded Event mappings on the composed path. [Public exposure](protocol-bindings/protocols/http/tests/producer_exposure.rs) · [Mapping capacity](protocol-bindings/protocols/mqtt/tests/support/notification_capacity.rs) | Source-free caller-owned Producer installation and independently retiring concurrent requests on one participation remain open. Applications combining finite and continuing families still assemble native members. [Mixed caller](protocol-bindings/protocols/mqtt/tests/collection_coexistence.rs) |
| Finite Properties and collections | Both roles execute HTTP Read/Write and root ReadAll, ReadMultiple, WriteAll and WriteMultiple. Direct verified HTTPS supports selected data and security combinations. With shared Host capacity an expired, accepted HTTP PUT can release its connection while the same Thing and a sibling keep serving and the caller keeps its outcome. [Root ReadMultiple](protocol-bindings/protocols/http/tests/read_multiple_properties.rs) · [Root WriteMultiple](protocol-bindings/protocols/http/tests/write_multiple_properties.rs) · [Structured values](docs/evidence/structured-dataschema.md) · [Composed security](docs/evidence/security-composition.md) · [Retirement over HTTP](protocol-bindings/protocols/http/tests/support/accepted_timeout.rs) | Each finite Producer participant still drives one native exchange. Phase-specific cleanup, checked outgoing data and authorization through the actual credential use remain incomplete. |
| Continuing Actions | HTTP Invoke, Query and Cancel retain the original invocation across exchanges. QueryAllActions observes a root inventory with per-invocation identity, output and visibility. A source-free Consumer can drive an invocation with caller-owned storage. [HTTP root status](protocol-bindings/protocols/http/tests/query_all_actions.rs) · [Caller-owned Action](docs/evidence/continuing-action-runtime-independence.md) | Producer retention reads process time. Aggregate handle construction starts before its work is charged. The Consumer close path still prepays access and validation work. |
| Observations and Events | Both roles execute MQTT observe/unobserve, subscribe/unsubscribe and the corresponding root collections. One root conversation preserves member identity and schema. A completed QoS 0 packet releases the shared writer while its result is held. A downstream Consumer can reply to an occurrence. [Root Properties](protocol-bindings/protocols/mqtt/tests/observe_all_properties.rs) · [Root Events](protocol-bindings/protocols/mqtt/tests/subscribe_all_events.rs) · [Writer lifetime](protocol-bindings/protocols/mqtt/tests/producer_events.rs) · [Occurrence replies](tools/architecture-fixtures/open-consumer/tests/notification.rs) | A recoverable emission cleanup error revokes a healthy Thing. Application-controlled subscriptions also need setup acceptance, a returned control operand and per-delivery ownership in both roles. [Emission settlement](servient/finite/src/notification_publication.rs) |
| Definition reuse and deployment | HTTP Read in both roles can skip actual invariant lowering when binding a compatible instance. Mixed Host turns drive both roles. Caller-owned finite Read and continuing Consumer Action execute with supplied storage and time. [Consumer costs](docs/evidence/consumer-execution-reuse.md#measured-costs-and-benefit) · [Producer reuse](docs/evidence/producer-preparation-reuse.md) · [Host turns](docs/evidence/host-execution-integration.md) · [Caller-owned Read](docs/evidence/constrained-finite-execution.md) | Continuing definitions can still specialize endpoints. A second mapping, costs on the normal deployment turn, and caller-owned Producer and notification lifetimes remain open. |

## Engineering priorities and dependencies

Public Producer exposure, Consumer choice and delivery failure isolation can
advance on existing paths. Callback subscriptions bring them together when one
application accepts a subscription that another application can control.
[Dependencies of the selected design](workspace/0132-engine-lifecycle-design.md#convergence-by-dependency)

### Near-term capability growth and priority

**Expose a finite Property through the ordinary Producer API, serve its
advertised address and retire each accepted exchange independently.** This
remains the priority because the finite driver owns one active reply per
participant and the general source-free Producer handoff is still open. Longer-lived
Actions and subscriptions need a truthful publication and a way to release
each request.
[Installation interface](core/src/publication.rs) · [Finite driver](servient/finite/src/host.rs)

Start with the application that exposes the Thing. Deployment registers linked
Bindings, services and capacity at startup. For each exposure the application
supplies its TD, behaviors and policy. Each selected Binding assembles its own
closed native participants. Servient publishes the final TD, exposes revocation
and lets the application drive accepted work through its own handles. The HTTP
finite example uses this operation. The same entry serves a TD with Forms on
both HTTP and an independently registered datagram service. Each adapter owns
its concrete definitions and native assembly.
[Host application](protocol-bindings/protocols/http/examples/serve_properties.rs) · [Mixed exposure](protocol-bindings/protocols/mqtt/tests/collection_coexistence.rs)

Two deployments constrain that same exposure at different points.

- **Host with an address learned during setup.** Check known requirements,
  including behavior coverage, during funded draft preparation. Reserve
  acquisition, finalization and rollback before taking a native lease.
  The Binding reports the endpoint it acquired with ingress closed. TD resolves
  relative Forms against the final base. While the lease is held, Planning
  builds any newly required mappings and qualifies every advertised Form,
  effective operation, alternative and behavior. The Binding checks native
  mappings, collisions and readiness. Servient verifies the exact members and
  holds those facts, selected services and competitors' answers current while
  publishing the TD and opening ingress in one callback-free commit.
  Read the Property at the address in the published TD. On refusal or
  cancellation after acquisition the candidate keeps ingress closed and
  releases its original lease while a coexisting incumbent keeps serving.
  A fixed endpoint can reuse coverage while the final TD and its dependencies
  stay current.
  [Whole-publication qualification](docs/spec/planning.md) · [Local publication and rollback](docs/spec/runtime-safety.md)
- **Caller-owned Producer with a trusted final TD.** Verify its coverage
  against the actual deployment through the same commit. Changed facts require
  fresh preparation or refusal. A downstream Binding lends bounded native
  slots over real I/O. Servient checks current membership, permission, behavior
  and capacity at dispatch. With supplied storage, clock and progress, two
  accepted exchanges on one participation can finish out of order. An unfunded
  offer keeps its original reply right and deadline. One reply can retire on
  native and cleanup credit while its sibling stalls and its result awaits collection.
  Revocation fences new admissions. A retained result releases only its own
  buffer generation across replacement. [Source-free publication](workspace/0132-engine-lifecycle-design.md#representation-and-public-entry-points) · [Admission and release](workspace/0132-engine-lifecycle-design.md#admission-and-release)

The two cases can proceed together. The first determines how closed native
facts reach final qualification and who retains the lease on refusal. The second
distinguishes a moved reply, a funded loan and a funded copy by their actual
release points. Evolve the composed Host path and the caller-owned entry around
one Planning coverage decision and the same Servient rules for publication,
admission, revocation and settlement. Revise the SPI and runtime clauses
where their current types impose Host ownership. Typed static inputs, Host
queues and a serial HTTP driver can each implement these contracts.
[Binding and deployment boundary](docs/spec/binding-spi.md) · [Finite capacity contract](docs/spec/runtime-safety.md#finite-producer-participation-and-work-capacity)

### Qualify the interaction through its first effect

From one admitted TD, let an application choose a named finite operation,
Action or Event in TD Form order or pin a permitted choice. Planning selects
the first complete viable mapping or control tuple under one effective
configuration. Ambiguity on a later unused Form or a different affordance
leaves a uniquely selected operation callable without a pin. If multiple owners
qualify at the preferred viable choice, selecting that operation or explicitly
requiring a set that includes it returns ambiguity. Default admission requires
at least one statically supported interaction and reports support per operation.
A separately funded inventory locates assessed ambiguity by affordance,
operation and Form. It distinguishes coarse or deferred answers from qualified
support. Unsupported meaning, selected ambiguity, exhausted credit and live
denial have distinct outcomes.
[Finite preparation](planning/src/open_consumer.rs) · [Deferred and continuing preparation](docs/spec/planning.md)

Lend resolved meaning to the actual Binding, codec and access service for all
required controls, response branches and security operands. A competitor
arriving after selection leaves the original association usable while its
services and permission remain valid. An uncertain effect precludes automatic
replay. Correct the 2019/2022 context-pair defect in the shared TD resolver.

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
- **Failure isolation for deliveries.** Establish the shared rule on existing
  MQTT emission before callbacks depend on it. Preserve each delivery's first
  cause, native effect, cleanup error and funding through physical retirement
  and result disposal. A peer can receive the Event, cleanup can fail and then
  recover, and the same healthy Thing must still emit. A failed subscription
  can close its own conversation. Actual loss of a required installed member
  must fence its original publication as soon as the loss is observed, even
  by an emission. A damaged shared stream fences its dependents while an
  independent service continues. Revise Runtime Safety's current notification
  revocation rule with the corresponding Binding and Servient handoffs.
  [Emission ownership](servient/finite/src/notification_publication.rs) · [MQTT failure controls](protocol-bindings/protocols/mqtt/tests/producer_events.rs) · [Runtime Safety](docs/spec/runtime-safety.md)
- **Two-ended subscriptions.** Join qualified setup, control and destination
  permission to exact Producer publication, funded admission and delivery
  isolation. The Consumer offers its callback address. The Producer can deny
  it or accept a funded obligation and return the ID used
  for separately authorized cancellation. A missing or malformed ID preserves
  remote uncertainty. Validate the occurrence and any application reply with
  their own target, authority and owner. A delayed cancellation must leave a
  same-subscriber successor on the same route able to receive and stop with its
  own ID. Retain broker-owned fan-out even with zero Engine subscribers, one
  root control across distinct members and refusal when required source
  evidence is unavailable. Exercise the notification and its response with both
  caller-owned storage and Host driving. [Consumer participation](core/src/notification.rs) · [Producer emission](core/src/publication.rs) · [Selected subscription contract](workspace/0132-engine-lifecycle-design.md#when-a-producer-accepts-a-subscription)

### Measure staging and reuse on the public paths

Concurrent finite Host candidates can commit or roll back independently on one
bank. Measure actual backing alongside reservations at peak and after commit
before choosing shared scratch. Candidate, serving instance, accepted work and shared native
service each release their backing at their own last use. [Producer preparation](planning/src/producer_composition.rs) · [Resource ownership](docs/spec/foundation.md)

Measure compilation reuse as soon as a mapping's invariant inputs are known.
Compare fresh specialization with checked reuse on HTTP and a worthwhile second
mapping, for example MQTT root Event member/filter instructions bound to
different prefixes. Count actual lowerings and compare source-to-ready work,
peak and retained backing, warm dispatch and the ordinary deployment turn for
one and many instances. Hold active work fixed while varying idle Things.
Use the comparison to decide where specialized mappings or indexes pay for
themselves. [Measured HTTP baseline](docs/evidence/consumer-execution-reuse.md#measured-costs-and-benefit) · [Scale criteria](workspace/0132-engine-lifecycle-design.md#compilation-reuse-and-the-cost-of-scale)

Extend the public paths as each interaction becomes usable. A retained or bound
Consumer Read uses the same checked association as a named operation and draws
from separately supplied call capacity. Migrate restricted and static callers
before retiring their Read façade, Plan-Set and route-stage builders. Retain
the raw layers used by composition. Foundation completion requires the joined
public paths, consistent domain contracts, representative Host and caller-owned
execution, and independent review at one revision. This includes the
execution-only dependency graph and the supported no-pointer-atomics build.
[Public caller census](docs/reviews/review-11-engine-architecture-audit.md#public-api-and-caller-census) · [Foundation completion criteria](workspace/0132-engine-lifecycle-design.md#foundation-completion-and-product-development) · [Work-package dependencies](docs/work-packages/index.toml)

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
