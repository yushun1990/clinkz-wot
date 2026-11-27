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
| Producer publication | An application supplies a TD, behaviors and policy. Linked finite services assemble the selected members. The Host qualifies the actual final address and every advertised operation before opening ingress. An assigned-endpoint loser releases its socket and returns the original handlers. [Public Host](servient/src/host_producer.rs) · [Wire and contention evidence](docs/evidence/finite-producer-publication.md) | Continuing Actions and MQTT still use the composed caller with explicitly assembled participants. [Mixed caller](protocol-bindings/protocols/mqtt/tests/collection_coexistence.rs) |
| Finite Properties and collections | Both roles execute HTTP Read/Write and root ReadAll, ReadMultiple, WriteAll and WriteMultiple. Direct verified HTTPS supports selected data and security combinations. With shared Host capacity an expired, accepted HTTP PUT can release its connection while the same Thing and a sibling keep serving and the caller keeps its outcome. [Root ReadMultiple](protocol-bindings/protocols/http/tests/read_multiple_properties.rs) · [Root WriteMultiple](protocol-bindings/protocols/http/tests/write_multiple_properties.rs) · [Structured values](docs/evidence/structured-dataschema.md) · [Composed security](docs/evidence/security-composition.md) · [Retirement over HTTP](protocol-bindings/protocols/http/tests/support/accepted_timeout.rs) | HTTP serves one native exchange per finite participant. The caller-owned Read supports multiple active replies on one member. Per-effect credential and data-direction work continues. [Independent reply retirement](servient/finite/tests/borrowed_producer.rs) |
| Continuing Actions | HTTP Invoke, Query and Cancel retain the original invocation across exchanges. QueryAllActions observes a root inventory with per-invocation identity, output and visibility. A source-free Consumer can drive an invocation with caller-owned storage. [HTTP root status](protocol-bindings/protocols/http/tests/query_all_actions.rs) · [Caller-owned Action](docs/evidence/continuing-action-runtime-independence.md) | Producer retention reads process time. The aggregate walk must bound handle construction as well as lookup. The Consumer close path still prepays access and validation work. |
| Observations and Events | Both roles execute MQTT observe/unobserve, subscribe/unsubscribe and the corresponding root collections. One root conversation preserves member identity and schema. A completed QoS 0 packet releases the shared writer while its result is held. A downstream Consumer can reply to an occurrence. [Root Properties](protocol-bindings/protocols/mqtt/tests/observe_all_properties.rs) · [Root Events](protocol-bindings/protocols/mqtt/tests/subscribe_all_events.rs) · [Writer lifetime](protocol-bindings/protocols/mqtt/tests/producer_events.rs) · [Occurrence replies](tools/architecture-fixtures/open-consumer/tests/notification.rs) | A recoverable emission cleanup error revokes a healthy Thing. Application-controlled subscriptions also need setup acceptance, a returned control operand and per-delivery ownership in both roles. [Emission settlement](servient/finite/src/notification_publication.rs) |
| Definition reuse and deployment | Compatible HTTP Reads in both roles can reuse prepared instructions. Mixed Host turns drive both roles. Caller-owned Consumers execute Read and continuing Action. A caller-owned Producer serves a trusted final Read with supplied storage, time and manual progress. [Consumer reuse](docs/evidence/consumer-execution-reuse.md) · [Producer reuse](docs/evidence/producer-preparation-reuse.md) · [Finite deployments](docs/evidence/finite-producer-publication.md) | Continuing definitions can still specialize endpoints. Costs on the ordinary deployment turn, a second reusable mapping and caller-owned notification lifetimes remain open. |

## Engineering priorities and dependencies

### Retire the superseded resource architecture

Complete the ownership and bounded-progress migration before expanding interaction
coverage. The finite Producer already demonstrates closed acquisition, rollback
and individually retired replies. Its Host preparation and composed participants
still use legacy accounting. The remaining dependencies determine the order.
[Accepted migration](workspace/0133-resource-management-proposal.md#disposition-of-308-and-the-roadmap) · [Owner and progress contracts](docs/spec/foundation.md)

1. **Finite Consumer Read convergence.** Host Reads use the same finite calls
   as Write and the collection operations. Source-enabled Host preparation and
   source-free mappings share that execution path. Caller-owned Reads lend
   prepared meaning and keep their typed result slot through disposal. The
   restricted Read façade, schema-2 policy and inline compiler are retired.
   Shared Host outstanding-call, response, scratch and cleanup accounts still
   cover multiple families. Host registration and preparation still consume
   footprint and artifact declarations. The remaining route-stage plan adapter
   serves Producer paths.
   [Host execution](servient/finite/src/consumer.rs) · [Caller-owned evidence](docs/evidence/constrained-finite-execution.md)
2. **Continuing and published work.** Migrate Consumer Invoke/Query/Cancel and
   notification conversations, then Producer candidate, invocation, delivery and
   reply owners with their Bindings and deployment loops. Secure real continuation,
   correlation, reply and handoff storage before effects. Drive local retirement
   under saturation and distinguish isolated cleanup failure from lost ingress.
   The shared validator and access work declarations still serve continuing
   Actions, notifications or preparation. Delete them with their last consuming
   chain, along with the remaining per-family credit factories.
3. **Preparation and Discovery.** Move TD traversal, Planning search and Discovery
   retrieval to the jobs that repeat visits, delegate, rewind or retry. Bound
   per-turn visits and cumulative work through exhaustion. Preserve page, byte,
   deadline and input ownership limits. This also releases dependencies of the
   finite Producer staging and Consumer registration paths.
4. **Shared quotas and final removal.** At each cutover identify the promised
   participants, unit and last holder. Conserve any retained aggregate across
   old and new participants, including taken results, through their final
   release. Where availability is promised in advance, hold actual supply at
   its owner and exercise the same path with the remaining pool exhausted. Retire obsolete ceilings at their last caller. Remove the flat
   resource profiles, global work classes and budgets, generic reservations,
   footprint declarations, generators, overlays and digest checks as their
   consumers disappear. [Quota cutover](docs/spec/foundation.md) · [Package dependencies](docs/work-packages/index.toml)

**Retirement is complete** when supported public paths, examples, fixtures,
benchmarks, build features and CI compile and run without the superseded APIs,
generated tables or work factories. Current specifications and public inventories
must describe the enforcing owners. Verify real Host and independent Binding I/O,
caller-owned execution and the no-pointer-atomics build, bounded callbacks and
refusal of unsupported reentry, job exhaustion, first cause and late evidence,
and reuse after final release.
For each continuing shared ceiling, test last-unit contention with a held result.
Review that evidence and the execution-only dependency graph at one revision.
[Runtime safety](docs/spec/runtime-safety.md) · [Integration obligations](docs/work-packages/WP-700-integration.md)

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
support. Unsupported meaning, selected ambiguity, exhausted search limits and live
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

- **Continuing Actions.** Claim the invocation and initiating reply before
  handler entry. Secure wire correlation when the mapping promises an
  addressable identity, later control or root status lookup. Query and Cancel
  use the original mapping and fresh permission. Under saturation local close
  can retire the original exchange and preserve a late value for authorized,
  validated handoff. Use the supplied monotonic clock for Producer retention
  and bound the aggregate inventory walk, including handle construction. [Core driver](core/src/execution/action.rs) · [Producer store](servient/finite/src/action.rs)
- **Failure isolation for deliveries.** Extend the finite Producer’s isolated
  reply cleanup and required-member loss cases to MQTT emissions. Preserve each delivery's first cause, native effect, cleanup
  error and backing through physical retirement and result disposal. A peer can
  receive the Event, cleanup can fail and then recover, and the same healthy
  Thing must still emit. A failed subscription
  can close its own conversation. Actual loss of a required installed member
  must fence its original publication as soon as the loss is observed, even
  by an emission. A damaged shared stream fences its dependents while an
  independent service continues. Migrate the Binding and Servient handoffs
  together under the delivery failure contract.
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

On the public paths, stage concurrent candidates and measure actual allocator
backing at peak, after commit and while results are held. Record headroom for
advertised quotas, useful completions and local retirements per bounded turn.
Candidate, serving instance, accepted work and shared service release at their
respective last use. [Resource ownership](docs/spec/foundation.md)

Measure compilation reuse as soon as a mapping's invariant inputs are known.
Compare fresh specialization with checked reuse on HTTP and a worthwhile second
mapping, for example MQTT root Event member/filter instructions bound to
different prefixes. Count actual lowerings and compare source-to-ready work,
peak and retained backing, warm dispatch and the ordinary deployment turn for
one and many instances. Hold active work fixed while varying idle Things.
Use the comparison to decide where specialized mappings or indexes pay for
themselves. [Measured HTTP baseline](docs/evidence/consumer-execution-reuse.md#measured-costs-and-benefit) · [Scale criteria](workspace/0132-engine-lifecycle-design.md#compilation-reuse-and-the-cost-of-scale)

Foundation completion includes joined public paths, consistent domain contracts,
representative Host and caller-owned execution, and independent review of the
execution-only dependency graph and the supported no-pointer-atomics build.
[Public caller census](docs/reviews/review-11-engine-architecture-audit.md#public-api-and-caller-census) · [Foundation completion criteria](workspace/0132-engine-lifecycle-design.md#foundation-completion-and-product-development) · [Work-package dependencies](docs/work-packages/index.toml)

After resource retirement, product webhook hosting, proxy support,
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
