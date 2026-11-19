# Target architecture for the ClinkZ-WoT Engine

Status: DISCUSSING

A Servient should be able to expose two Things on one listener, let a request to
the first expire after its handler runs, and serve the next request to **either**
Thing while the application still holds the first outcome. It should also be
able to consume many Things using one compiled mapping definition and a small
pool of calls, with independent permission and cleanup for every instance. The
first example tests how far a failure travels. The second tests what can be
shared. Give Servient authority over logical instances and accepted work, give
each Binding authority over its native resources, and let each admission
borrow or own exactly the qualified mapping, capacity and reply rights it uses.

This is a proposed refinement of ADR-0025. Review 11 audited production revision
`d6a4c86b`. The merge at `4ad10ece` added the report. The independent review
of `7930cf58` also reproduced a publication assembled across incompatible
configurations and an HTTP Consumer sending an undescribed object member. Its
other three findings concern which subscription lifetimes, credential audiences
and Binding choices the architecture must represent. The convergence plan gives
the execution evidence still needed. [Product goals](../docs/architecture/00-system-goals-and-context.md) · [ADR-0025](../docs/ADRs/0025-semantic-engine-and-binding-runtimes.org) · [Review 11](../docs/reviews/review-11-engine-architecture-audit.md) · [Independent review of the proposal](https://github.com/yushun1990/clinkz-wot/pull/289#pullrequestreview-5970405546)

## Start with the promised interaction

A Property exposes state, an Action invokes behavior and an Event offers
occurrences. A Form describes a way to perform an operation. The same Thing can
offer different Forms and protocols. A described Thing-root collection has its
own Form, member semantics and result. A Consumer chooses a usable interaction.
A Producer publishing a TD promises the interactions it advertises. Discovery
can supply the description and its retrieval provenance. Access to the actual
interaction requires its own security and policy decision. These are reasons
to share interpretation across both roles while giving selection, publication
and live dispatch different responsibilities. [WoT Architecture 1.1](https://www.w3.org/TR/2023/REC-wot-architecture11-20231205/) · [TD 1.1 Forms](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#form) · [WoT Discovery](https://www.w3.org/TR/2023/REC-wot-discovery-20231205/)

TD 1.1 and WoT Architecture 1.1 are Recommendations. A protocol vocabulary
and its published mapping determine the wire interpretation within their
stated scope. Binding Templates is a retired Group Note. ClinkZ must choose
its own Rust interfaces, progress policy and resource owners. Its goals demand
a real extension path, bounded work and equivalent supported semantics on Host
and constrained deployments. [Binding Templates status](https://www.w3.org/TR/2025/NOTE-wot-binding-templates-20251104/#sotd) · [Delivery scope](../PLAN.md)

## Ownership and lifetime

Each row answers who can establish a fact or release a resource. Several rows
can share one Rust value where their lifetimes coincide. A difference in
authority or last use must be representable even on a deployment using one slot.

| What | Owner, relationship and release |
| --- | --- |
| Reusable definition | TD supplies shared meaning. A Binding can lower the invariant part to immutable instructions and declare the operands it accepts. One registration can execute many definitions. Many compatible mappings can reference one definition. Credentials, handlers and open sessions enter at later lifetimes. The caller, build image or preparation owner supplies the backing. Dynamic backing is charged once to its original owner through last use. |
| Qualified mapping | For TD-driven selection, Planning brings the selected site, Form, operation, schemas and security branch together with the actual Binding registration, configuration, data and access services. The Binding checks its concrete operands. A source-free install can use the same qualified facts. They establish executable support. Permission to use them belongs to the instance. Identical immutable selections can be shared. Changing a service, endpoint or selection requires checking the affected facts again. |
| Logical instance and Producer publication | Servient issues the local identity and incarnation. A Consumer association attaches selected mappings and local policy. A Producer additionally associates behaviors, a final TD and its exact native participations. Replacement fences the old incarnation before admitting work on the new one. A TD `id` is an optional discovery/lookup attribute. Several views may intentionally share a revocation domain. Independently installed instances have distinct authority even when the descriptions or addresses agree. [TD 1.1 Thing](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#thing) |
| Execution capacity | Deployment supplies bounded pools or caller slots and may impose per-instance quotas as well. Active exchanges, invocations, conversations, queued items, retained results and draining work consume the capacities they actually need. Many idle instances can share a pool. A one-opportunity pool is a valid deployment choice. |
| Native service and participation | The Binding owns listeners, sessions, SDK buffers, routes, filters and physical release. Each participation identifies one installed contribution. A Thing can need several. Many participations can share one service when configuration, trust and account isolation permit it. A service can yield several exclusive opportunities or serialize them. Its owner decides when the last lease drains, when to retain an idle pool and when to shut it down. |
| Admitted work | Servient accepts work for an exact incarnation, mapping and recipient, retaining authority, operands, funding and the native opportunity. A continuing Action invocation and an application-accepted Producer subscription can each outlive the establishing exchange. Deliveries, optional application replies and emission attempts have their own lifetimes. The Binding owns native correlation and progress. A Producer can emit with zero Engine-owned subscribers. |
| Retained result | At settlement, transfer the value or late evidence, actual backing, original attribution, effect, delivery and first cause into funded retention. Release each active permit and native lane when its own work retires. An SDK-backed value can still pin its specific buffer. Charge that buffer until the view is released. Disposal returns only the result's original charges. |
| Retirement | Each outstanding obligation keeps one owner able to advance local close or transfer the complete funded work to an owner that acknowledges it. Stop new admissions, settle or drain original work, and reclaim each last reference independently. An old result or delayed native close can outlive a new incarnation. Neither has authority over the successor. |

Core supplies the semantic contracts for value roles, authority, evidence and
the family-specific transitions. Servient owns decisions about live instances,
handlers, recipients and publication. A Binding recognizes and correlates its
native traffic, distributes it as its protocol requires and reports physical
readiness, effects and failures. The application supplies behaviors, policy,
capacity, compatible service sharing, monotonic time and progress. This also
allows a source-free caller to supply checked prepared data directly. Execution
needs only the facts and owners used by that interaction. Putting dispatch in
each Binding would duplicate policy and leave a cross-Binding Action without
a common invocation owner. Putting route matching in Servient would duplicate
the native lease and filter decisions that actually determine who received the
traffic. [Interaction Core](../docs/spec/interaction-core.md) · [Binding SPI](../docs/spec/binding-spi.md) · [Runtime safety](../docs/spec/runtime-safety.md)

## Qualify the claim that will actually be used

**Interpret once per admitted input.** TD owns operation applicability and
defaults, URI resolution and variable declarations, context expansion, schema
roles and security inheritance/composition. Admit the immutable input and lend
the selected Form and its semantic dependencies under a work bound. Start with
an index of sites and Forms and resolve detailed facts as needed. A caller
preparing several operations from one Form can use the same resolution. Fully
normalizing and retaining a second document costs memory even for one selected
Read. Measure the benefit before choosing that representation. Separate invalid
input, unsupported selected meaning, ambiguous support, exhausted allowance
and live denial in the outcome. [TD contract](../docs/spec/td.md) · [Planning contract](../docs/spec/planning.md)

**Let an actual implementation qualify a profile.** Common Planning passes a
bounded view of the complete selected meaning to registered implementations,
including resolved extension names, values and their location, coding, URI
variables, response alternatives, security operands and schema roles. Keep
unknown associations on round trip. The Binding interprets protocol terms and
qualifies addressing, native identification and control compatibility. The
actual codec, validator and access service qualify the parts they implement.
Core and Servient still enforce required data checks, credential scope and
authorization. A provider's ability to parse a field is insufficient to claim
the complete interaction. An unhandled selected requirement refuses that
candidate. An unrelated annotation or unused security definition can coexist
with a supported selection. A Binding registers the roles and capabilities it
implements. This applies equally to a third party and to HTTP/MQTT. It
requires neither a vocabulary of mandatory extensions nor
an Engine protocol switch. [TD extensions](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#sec-context-extensions) · [Consumer pre-provider refusal and Producer comparison](../docs/reviews/review-11-engine-architecture-audit.md#f12-consumer-planning-fixes-the-profile-before-a-third-party-binding-can-qualify-it)

**Qualify who generates the data.** TD 1.1 §8.2 lets a Thing return additional
object members and requires the Consumer to accept them. It constrains the
Consumer to generate data described for that interaction. Thus a Read of
`{"level":3,"extra":1}` can succeed against an object schema describing only
the integer `level`. Submitting those bytes as a Write must refuse before I/O.
A wrong type for `level` still fails in either direction. Apply the rule at
nested objects and, when supported, to extra items in tuple-style arrays.
Required fields, bounded decoding, the selected collection members and the
recipient's permission still have their own checks. An extra value supplies
no authority to act on another Property. A Producer's choice to accept
additional input is a separate application contract. [TD 1.1 data behavior](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#behavior-data) · [Paired HTTP Read/Write observation](https://github.com/yushun1990/clinkz-wot/pull/289#discussion_r4625168387)

Use one schema for common constraints. Core identifies the operation's data
role and generating party. The Consumer generates a Write value, an Action
input, subscription/cancellation data and an Event application response. It
receives the Thing's values, Action outputs and Event data. The actual data
service qualifies the used role and direction. A direction argument or separate
encode/validate entries suffice. Already encoded caller input owes the same
outgoing check. The current HTTP callback takes only schema and payload, and
the object evaluator visits only described names. An outgoing implementation
must be able to examine the supplied members or construct the value from
checked fields. Indexing or a streaming codec remains a cost decision. [Current Consumer contract](../docs/spec/interaction-core.md?plain=1#L159) · [Object evaluator](../core/src/data_schema.rs#L336) · [HTTP validator](../protocol-bindings/protocols/http/src/lib.rs#L819)

**Select a complete usable alternative.** A Consumer can narrow the operation,
Form, Binding and permitted security choices. From one ordinary preparation
entry, Planning considers every applicable implementation in the application's
registered set under one allowance. The current notification registry is
parameterized by one concrete Binding type. Running two such registries
independently leaves the choice and competing-provider ambiguity to the caller.
A bounded visit across typed registrations, retaining the chosen program and
executor together, suffices. Host may erase the candidate interface. Neither
a shared native program type nor a dynamic plugin loader is required.
[Notification registration and selection](../planning/src/open_consumer/notification.rs#L26)

Preserve required conjunctions and the advertised one-of choices.
Qualify all response roles and each control required for the
chosen lifecycle. For a continuing Action, join Invoke with compatible Query
and Cancel mappings, including across registrations that can interpret the
original continuation. For automatic selection, prefer the first viable tuple
in TD Form order, considering Invoke then its required controls in a fixed
order. This is ClinkZ's default preference. A tuple must satisfy fixed operands,
services and selection policy. Registration enumeration order conveys no preference.
An explicit selection fails if its constraints admit no complete tuple.
Ambiguous providers for the same selected mapping refuse. Exhausting the
allowance reports a capacity limit.
No side effects occur during this search. After admission, an uncertain Write,
Invoke or control never triggers automatic replay through another Form. [Action selection](../planning/src/open_consumer/action.rs#L219) · [TD security definitions](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#securityscheme) · [Downstream split Invoke/control execution](../tools/architecture-fixtures/open-consumer/tests/action.rs#L403)

**Make qualified use valid through the first effect.** A mapping retains the
actual service and the configuration under which qualification succeeded. Use
an immutable snapshot where available. A reloadable provider can lend a scoped
guard or check the expected revision and perform the dependent operation in
one serialized entry. A revision sampled in Servient before an independent
provider callback leaves a check-to-use gap. Recheck current instance permission
after reentrant callbacks and before each dependent effect or success handoff.
Check dynamic URI values, selected members and continuations when available.
If an effect wins a race with revocation, retain the original work and its
evidence. [Qualification through first native effect](../docs/reviews/review-11-engine-architecture-audit.md#from-qualification-to-the-first-native-effect)

**Scope each credential to its actual use.** A Form's logical target, the next
network hop and the party authorized to receive a credential can differ. In a
proxy-plus-endpoint conjunction, TD and Planning preserve each scheme's
audience and obligations. Before each credential is issued or presented, the
access service authorizes the original instance, operation, intended target,
actual hop and credential audience/scope. The Binding establishes peer trust,
uses the credential with the qualified placement and protects it through any
intermediary as required. A changed redirect, callback or continuation
destination needs fresh authorization before dependent use. An unresolved
audience or missing mechanism refuses that use. Immutable definitions can
describe these requirements. Live secrets belong to the authorized exchange
or session. Direct HTTP may satisfy all checks with one destination. An opaque,
audience-scoped credential handle per use suffices for a multi-peer Binding.
[TD 1.1 combined schemes](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#combosecurityscheme) · [Planning destination policy](../docs/spec/planning.md?plain=1#L138) · [Current access-service entry](../core/src/consumer.rs#L162)

Likewise, authenticating a broker establishes its transport identity. If
delivery policy requires the publisher's identity, it needs verified evidence
bound to that occurrence and source, or delivery refuses. The review found an
assumption in the proposed single-destination contract. It reported no
credential disclosure. [Occurrence authorization scope](../core/src/notification.rs#L412)

### Qualify the whole publication at one local commit

Stage with closed ingress and fund provisional output, scratch and rollback
before acquisition. Closed native setup can determine the final addresses.
Planning must establish that every retained affordance has Forms and every
final Form, effective operation, advertised alternative and behavior is covered
under **one configuration effective at commit**. This includes the registration
set, the selected providers and the configurations of all eligible competitors
whose negative answers establish uniqueness. The current final walk asks each
provider about one Form at a time. A competitor can be eligible for A in one
configuration and B in another, yet decline each when it reloads between
probes. Combining those answers certifies a configuration that never existed.
Repeating that walk has the same defect. [Final provider loop](../planning/src/producer_composition.rs#L375) · [Independent publication comparison](https://github.com/yushun1990/clinkz-wot/pull/289#discussion_r4625168375)

Prefer an immutable registration set with immutable qualification configurations
installed together for this publication. Reuse a staged answer only for the
same final TD and dependencies. A deployment permitting in-place reload must
order **every change affecting that proof** against commit. For example, qualify
against a non-reused revision of the whole set, then compare and publish in the
same local serialized step respected by every writer. A changed revision
invalidates the attempt. Retry within the staging allowance or leave ingress
closed. Separate last-look checks leave a gap before commit, as does a lock
that a provider can bypass. A fixed deployment can establish the guarantee
entirely by construction.

Servient checks the actual installed operands, collisions, ready member set and
current publication authority, then makes the exact final TD and logical
ingress current together. Partial setup retains its native retirement owner.
After commit, release scratch and preparation-only references at last use.
A registration added to the next immutable set applies to future selections.
Per-use permission and credentials stay live. Withdrawal of a required installed
capability fences the affected promise. This commit orders local publication,
with independent network availability and peer caches. [Publication contract](../docs/spec/runtime-safety.md?plain=1#L493) · [Local commit](../servient/finite/src/publication.rs#L604)

## Admission, failure and independent cleanup

Before acceptance, the Binding owns an offered packet, stream or SDK slot,
its bounded buffering and its native refusal path. It can wait for capacity,
apply its documented loss/refusal policy, or lend a complete offer. An Engine
refusal leaves the original input and exclusive opportunity with their owner.
Before accepting, acquire active capacity, bounded terminal backing and a
cleanup path. A native receive that requires a pre-reserved slot may use one.
An empty poll need not allocate a settlement for each idle Thing. A Binding can
hand off a movable reply owner or a generation-checked loan into its own slots.
Both must support original correlation, refusal and individual close. A serial
device can supply one outstanding loan. A multiplexed listener can have several.

The Binding identifies the original native recipient and preserves the reply
right. Servient checks issuer, unique logical candidate, publication membership
as applicable, incarnation, behavior, input and current policy before dispatch.
A candidate alone grants no dispatch right. The HTTP listener already captures
lease eligibility at acceptance. A slow request spanning replacement is still
attributable to its original lease. Keep this trust boundary. Introduce extra
logical membership evidence only if a conforming supported Binding can present
an otherwise acceptable candidate to the wrong instance. [Finite ingress and exclusive reply](../core/src/finite.rs#L151) · [F4 attribution evidence](../docs/reviews/review-11-engine-architecture-audit.md#f4-finite-dispatch-trusts-the-binding-for-original-native-recipient-attribution)

| Event | Smallest owner that must act | Observable result |
| --- | --- | --- |
| Active or retained capacity exhausted before acceptance | The offered exchange and its admission pool | Backpressure or declared native refusal. The publication stays current. A pending native offer remains bounded and keeps its original deadline. |
| Accepted request expires, reply fails locally or caller cancels | That exchange, its original reply opportunity and its funded outcome | Record first cause, whether the handler acted, what is known about delivery and any late value. Retire the exchange. A healthy participation can accept another request on the same Thing even with the first outcome held, subject to actual capacity. |
| A required installed member loses readiness irrecoverably, or the owner replaces the Thing | Binding reports the affected participation. Servient fences the corresponding publication or commits a separately qualified replacement. | Drain old requests against their original owners. An independent sibling continues on a healthy service. Refusing a staged successor leaves a still-serving incumbent current. Exclusive reuse of the same native route can require explicit withdrawal first. |
| MQTT finishes a QoS 0 packet on a healthy stream | The original emission releases exclusive writer use as soon as its write is complete and safe. | An unread result retains its own bytes and funding. A sibling can write while the first application still holds its result. Late disposal cannot unlock a successor's write. Local publication establishes no subscriber delivery. |
| A partial packet makes a shared stream unusable, or its physical owner shuts it down | The Binding fences that stream and reports the affected dependents. Servient reconciles the promises that depended on it. | A genuine shared failure can span Things. Independent services continue. Report uncertain remote effects and retain the original cleanup owners. |

Retirement has its own progress obligation. Once a terminal cause fences fresh
effects, native close needs its original owner, native/cleanup allowance and
any genuinely required I/O. It can advance with zero new permission, codec or
result-queue credit. Reserve a bounded place for terminal evidence at admission
so a close can record it there before queue insertion or application handoff.
A full queue can block new admissions. A late output can wait there for its
original validation service. Check live permission before handing out success.
A mapped remote Cancel/Unsubscribe can have its own authorization and uncertain
outcome. Local release must still be drivable when that control is denied.
If a native API needs more progress, keep the work and its charges through
retry or acknowledged transfer. A cleanup error accompanies the first cause.
Drop can perform its bounded local disposal. Report remote completion only on
evidence from the mapping. [Finite close prepayment](../servient/finite/src/host.rs#L1170) · [Action work prepayment](../core/src/execution/action.rs#L196) · [MQTT retirement and destruction](../protocol-bindings/protocols/mqtt/src/native.rs#L816)

### When a Producer accepts a subscription

An Event source can publish to a broker that manages all remote subscriptions.
Another mapping can ask the application to accept a subscriber's filter and
callback address, return an identifier and later accept a cancellation. The
TD webhook example illustrates that second lifetime. Today's Producer facet
admits emissions. Its Consumer counterpart can send a reciprocal response.
Neither entry gives the Producer application authority over establishment or
cancellation. [TD Event roles](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#eventaffordance) · [Webhook example](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#webhook-example-serialization) · [Producer notification facet](../core/src/publication.rs#L86)

For a mapping offering application-controlled subscriptions, the Binding
offers the original establishment or cancellation input and any mapped reply
opportunity. Servient checks the current publication, Event, subscriber
authority, declared control data, filter and actual callback destination
before the application accepts. Refusal returns the opportunity for native
refusal and grants no delivery rights. Reserve capacity for the subscription
and its cleanup before acceptance. It belongs to the original incarnation
and survives the setup reply. The Binding maps any wire identifier back to
it and owns native routing, correlation and distribution. Servient authorizes a
cancellation against the original subscriber and subscription, and authorizes
later callback delivery against the actual destination. A foreign ID or denied
cancellation leaves that subscription and its siblings intact. An undelivered
setup reply can coexist with an accepted subscription that still needs cleanup.

Each callback delivery retains its own work. Where the mapping offers an
application reply, bind it to the original occurrence and validate the actual
`dataResponse` role before handing it to the Producer application. A transport
acknowledgement proves only its mapped transport outcome. On stop, revocation
or replacement, fence new deliveries and drain late callbacks, replies and
results with their original funding. Reuse ordinary admission and retirement
for the control exchanges and retain only the extra subscription state that
survives them. Publishing this mapping requires actual establishment,
cancellation and reply support wherever the final TD promises them.
Broker-owned MQTT needs no local subscriber table. A production
HTTP webhook service is one possible implementation of the conditional
contract. [MQTT control-data scope](../protocol-bindings/protocols/mqtt/src/preparation.rs#L166) · [Consumer reply fixture](../tools/architecture-fixtures/open-consumer/tests/notification.rs#L428)

## Compilation reuse and the cost of scale

0081 settled the independence of definition, instance authority and physical
resources. Its remaining engineering question is how much compilation can be
avoided **before** lowering. Let K count distinct compatible definitions, M
the selected mappings and I the local instances. One Thing may use several
definitions, and different Things may share an entire mapping set or only its
invariant instructions. Startup pays for distinct supplied inputs, K actual
lowerings, dependency comparisons and operand checks for M mappings, admission
of I instances, and necessary native installation. K can grow with M. An idle
instance needs its identity, revocation state and actual mapping references.
It can have zero active calls, sockets or retained results. [0081 disposition](0081-reusable-plan-and-thing-identity.md) · [Current preparation authority](../docs/architecture/30-compiled-plan-lifecycle.md)

Prefer an explicitly supplied definition and checked binding. Each Binding uses
the same inputs its lowerer observes to establish equivalence, including data
direction, operation, schema/response and credential-audience requirements,
vocabulary, compiler format and configuration wherever they affect instructions.
It checks variable operands such as a destination or topic prefix against that
definition and the actual execution registration. Compare against fresh specialization. Varying
only an operand should skip invariant work. Changing a dependency should
refuse the binding or require explicit compilation before effects. A digest,
TD identifier, shared socket or byte deduplication after compilation cannot
establish that an expensive step was skipped. A genuinely specialized mapping
can use direct preparation. An automatic cache would additionally own
concurrent construction, failure, eviction and last use. The explicit path
needs none of those services. [Checked HTTP Consumer lowering](../docs/evidence/consumer-execution-reuse.md#explicit-http-definition-compilation) · [Producer reuse and collision evidence](../docs/evidence/producer-preparation-reuse.md#executable-evidence)

The HTTP Consumer experiment reports one lowering across 32 mappings, 55,388
versus 91,680 retained heap bytes, and similar total debug preparation times.
The single-mapping case used 660 extra heap bytes with sharing. HTTP Producer
Read also reused lowering across endpoints and independently refused a route
collision. These are working controls. The archived 10,000-instance HTTP run
used dedicated slot provisioning and direct admitted-call polling. Master
already lets independent Consumer associations share one call pool. Neither
study measures today's association-only memory slope, a mixed Producer driver
or a constrained device. [Measurement revisions and limits](../docs/reviews/review-11-engine-architecture-audit.md#what-the-recorded-measurements-establish)

Measure K, M, I, mapping width, active capacity, concurrent staging, native
lanes, fan-out and retained/draining work independently on the affected path.
Compare total source-to-ready work and time, entries into the actual lowerer,
heap traffic, peak/live bytes, original account reservations and release.
Measure a selected dispatch and the normal deployment turn over quiet owners,
including service maintenance. For load, distinguish latency after admission
from latency including queueing. With mapping, payload and service held fixed,
adding idle Things must not raise the required work allowance for a retained
operation merely to search them. A sweep of configured slots,
native route collision checks and per-recipient permission have legitimate
costs. Name and bound them at their owner. Small tables or a linear scan can
win for a small profile. Introduce an index, streaming validator or service
wake list when the corresponding workload justifies its storage and complexity.

## What the audit findings change

The observations and controls belong to Review 11. The dispositions below
identify which design choice each observation can support. [Findings F1–F16](../docs/reviews/review-11-engine-architecture-audit.md#pass-2--findings-challenged-against-callers-and-contracts) · [Cost and findings F17–F20](../docs/reviews/review-11-engine-architecture-audit.md#performance-and-scalability-on-master)

| Findings | Cause and consequence for the design |
| --- | --- |
| F1, F2 | Two reproduced local correctness defects. The context resolver seeds only once and rejects the standard 2019/2022 pair. The Producer walk can pass every visited operation and miss an affordance with zero Forms. Correct context processing and scope-level structural validation at their existing owners. These defects invalidate reuse of the affected answers. |
| F3, F12 | Structural selection defects. An early per-operation preference discards the only compatible Action control family. Generic Consumer gates discard coding, security or other selected metadata before a downstream provider can qualify it. Qualify complete alternatives through actual services. The Producer already lends semantic input to providers. The downstream coding comparison locates a refusal point and establishes no support for the coding itself. |
| F4 | An assurance boundary. Native attribution is trusted and logical authorization remains in Servient. The audit found no conforming finite misdelivery. Keep the HTTP lease and replacement evidence and the explicit counterexample for reopening this decision. |
| F6, F7 | Uneven reuse is a capability gap. Repeated Basic admission and support-via-lowering are avoidable work. Service qualification is an ownership issue where the service that acts can change. Reuse complete invariant facts, pin their qualified use, and keep live policy, negotiated readiness and late operands at the effect edge. No observed HTTP retargeting underlies this decision. |
| F5, F8, F17 | Work and funding begin or end at the wrong phase. F5 is a local ordering defect. Pay for aggregate Action handle allocation and inventory lookups before doing them, preserving per-member visibility. F8 leaves the payer before managed admission unclear. F17 establishes a real ceiling of at most 16 simultaneously staged exposures on one default Gateway bank, each reserving at least 16 MiB of its 256 MiB temporary quota. Serving releases that reservation but retains preparation records and provider references. Physical heap retention depends on sharing. Fund preparation explicitly and release its backing at last use. |
| F9, F10, F15, F16 | Unrelated owners constrain progress. F9 mixes real resource pressure with accidental dimensions such as a participant limit on notification routes, whole-store Action scans and speculative work on empty receives. F10 prevents even a native close poll without enough queue credit. With **full** credit F15 still revokes the Thing after an accepted exchange expires. F16 holds a healthy MQTT writer until result acknowledgement. These last three require distinct changes to phase funding, failure scope and physical occupancy. Raising limits or driving harder cannot establish those invariants. |
| F11 | A capability gap with a structural cause. Caller-owned Read and continuing Consumer Action already run. Producer installation exposes Host ownership types, and Producer Action retention reads `std::time::Instant` internally. Let the deployment supply storage, clock and progress for these lifetimes. |
| F13, F14 | Public surface debt with different evidence. The prelude's `Servient` names a restricted Producer Read runtime. It and the static adapters have real callers. Replace the façade and restricted Plan-Set/route-stage builders as callers migrate, preserving their funding, service checks and cleanup. Keep raw finite/notification layers used by composed paths. Remove `ProcessEvent`, `ProcessTerminal` and `SubscriptionSlotId` with their inventory entries after checking the cited contracts. The census found no in-tree execution caller for these three. |
| F18, F19, F20 | Confirmed traversal, conditional performance cost. F18 repeats m(m−1)/2 uniqueness comparisons per association with m distinct entries. F19 can scan an encoded object once per described field. Finding undescribed outgoing members is a correctness obligation regardless of the scan's speed. F20 can revisit every HTTP lease per empty receive and MQTT interests per filter per quiet poll. Measure a shared checked set, a bounded object traversal and service-owned maintenance against the small bounded implementations. Native matching and fan-out stay in the Binding. |

## Representation and public entry points

Make the ordinary Consumer entry an association with checked mappings. A caller
can select by name or retain a checked operation reference for repeated use,
then submit work against a supplied capacity. Fold the bound Property Read
fast path into that contract. Replace preparation that requires a live access
owner and bank in every reusable definition. Bind actual services and funding
where selection or use depends on them. For a Producer, supply behaviors and
closed participations, publish once qualified, and drive accepted work through
its own handles. Separate closed installation, per-exchange retirement and
service shutdown in the native SPI. Increasing the concurrency of the current
participant-wide state machine would still give each exchange the power to
close the participant.

For a one-off operation, a direct, fully qualified specialized mapping is
sufficient. For repeated sets, try one checked immutable set with a small
per-instance reference and only the differing operands. Eager per-operation
records buy direct lookup at a cost proportional to instances times mappings.
An instance that uses only one operation may do better with a bounded selection
and a retained reference for that operation. Keep the representation local
until source-inclusive time and retained bytes distinguish them. An Engine-wide
VM, dynamic plugin loader or mandatory installed-operation object would add
format, lifetime and dispatch costs to each backend. Registration of linked
capability implementations and ordinary typed calls already allow independent
Bindings. [Existing association construction](../servient/finite/src/consumer.rs#L1477) · [Public caller census](../docs/reviews/review-11-engine-architecture-audit.md#public-api-and-caller-census)

For a source-free Producer, a trusted preparation source can supply the exact
final TD and its coverage for a specific immutable deployment configuration.
At closed installation check the actual registration set, including competing
providers, selected services, behaviors, endpoints and ready members against
that coverage through commit. A changed provider set can invalidate uniqueness
even when the bytes and address agree. Return changed facts to preparation
or refuse publication. A device with a fixed build can establish the same
correspondence without a runtime parser. A raw participant can serve under
its narrower contract.

Host owners may use reference counts, queues, registries and an async executor.
A caller-owned implementation can use borrowed definitions, typed native slots,
generation-checked references, explicit result storage and a cooperative poll.
Both enforce the same acceptance, callback invalidation, result and retirement
edges. A caller can supply monotonic time for request deadlines **and** Action
retention. The deployment arranges progress on readiness, deadline, revocation
and replenished credit. Release shared registry borrows before invoking
application or provider code. Keep enough original state to process an accepted
callback even when it revokes its own instance. Execution-only dependencies
exclude TD parsing and compilers. A no-std build on `thumbv6m-none-eabi`
(no pointer atomics) plus real caller-driven I/O on Host can establish the
supported representation. Physical
device timing, RAM and transport behavior need deployment evidence. [Caller-owned execution scope](../docs/evidence/continuing-action-runtime-independence.md#remaining-0081-and-runtime-independence-scope) · [Current Producer installation interface](../core/src/publication.rs#L198)

## Convergence by dependency

The earlier plan would have migrated authority across domains up front, chosen
Host installation and work interfaces, and left caller-owned storage until the
end. Begin with two independent questions. Can the selected services execute
the described roles with the right authority? Can a caller-owned Producer admit
and retire native work independently? Their results meet at whole-publication
commit. A Producer-controlled subscription then uses both contracts and adds a
logical lifetime between control exchanges. Action migration can proceed on
its own dependencies. [Earlier proposal](https://github.com/yushun1990/clinkz-wot/blob/3076f23f7e0c28bd81d0d8cf8bcd5cf6c03e67bd/workspace/0132-engine-lifecycle-design.md) · [Architecture change control](../ARCHITECTURE_GOVERNANCE.md#architecture-change-control)

### Selection and qualified use

Use two unrelated Binding implementations together through one ordinary Consumer
entry and one Event TD with alternative Forms. Select each implementation in
turn and observe its peer. With the first Form ineligible, automatic selection
must execute the later viable Form in either registration order. Make both
providers support the same selected Form and observe ambiguity. Pin a Form
whose required service is missing and observe refusal. Exhaust search credit
before uniqueness is known and observe a capacity refusal with no native start.
Keep the split-Binding Invoke/control case as a separate witness to compatible
Action selection. A downstream continuing profile must also be able to qualify
and enforce selected metadata previously rejected before its provider ran.

- Use the same object schema and bytes on a real HTTP Read and Write. Accept
  the extra field on receipt. On generation, reject it with zero native
  requests and zero handler entries. Keep a conforming Write and a wrongly
  typed described field as controls.
- Use two distinguishable credentials, a proxy peer and an endpoint peer.
  Observe each credential at its authorized recipient, with the endpoint
  material protected from the proxy where required. Change a hop or scope,
  or remove the verifier. The affected credential must never reach the wrong
  peer. Rotate a provider just before native entry and verify that only the
  qualified configuration can act. A downstream mapping with controlled peers
  suffices to exercise the contract.

This work establishes the input contract for 0081. As soon as a Binding can
identify which inputs its lowering observes, compare explicit reuse with fresh
specialization at fixed K and increasing I, then vary K. Candidate selection
and this experiment can use existing execution paths.

### Admission and release

In parallel, use a curated source-free finite Producer mapping. Exercise closed
installation, publication, acceptance, revocation and final release using
caller-owned storage and clock over real I/O **before choosing the shared
installation SPI**. A downstream Binding lending bounded native slots must be
able to refuse an offer under pressure, complete two accepted replies out of
order, and retire one while its sibling stalls. Supply native and cleanup
credit alone. Repeat the full-credit deadline after handler entry and serve
the next request to the same Thing and to a sibling while the result is held.
In the broker control, allow a sibling to write while the first completed
result is held and preserve partial-packet fencing. These checks can use
existing prepared inputs.

Exercise **finite Consumer close, continuing Action close and finite Producer
close separately** after a terminal cause. Give each only native and cleanup
credit. Observe the original native owner advance. Then supply the missing
validation or queue credit and recover its late value or settlement. Each
path has its own prepayment and handoff today. [Finite Consumer close](../core/src/execution.rs#L230) · [Action close](../core/src/execution/action.rs#L195) · [Finite Producer close](../servient/finite/src/host.rs#L1170)

### Join at instance binding and publication

Once both handoffs work, exercise normal finite Consumer and composed Producer
execution from a valid TD through HTTP I/O. Change the final endpoint, cause a
native collision and replace an instance while old work drains. For publication
consistency use two Forms and a competitor that overlaps A in configuration 1
and B in configuration 2. Each fixed configuration must refuse ambiguity.
Switch between probes so each Form appears unique. Ingress must stay closed.
Use a third, genuinely unique configuration as the successful on-wire control.
Then invalidate that effective configuration after its last probe and before
commit. The change must either be ordered after commit or prevent publication
under the stale proof. A source-free installation presented with the same TD
and different eligible registrations must also refuse stale coverage.

Hold several exposures in closed staging on one bank. Complete or refuse them
independently and account for preparation-only backing through last use. On the
normal driver, hold admitted work fixed while varying idle instances and
mapping width. Shared definitions must preserve independent permission, refusal,
funding and final physical release.

Before adopting a cross-family representation, extend the existing HTTP Read
evidence to a worthwhile second mapping. One candidate is MQTT
SubscribeAllEvents. Check whether member/filter instructions can be compiled
once across two valid prefixes, retaining prefix/member byte checks at binding.
Compare actual lowerings and total startup cost with fresh specialization. If
sharing moves the same work into operand checks or saves little, retain
specialization for that mapping. For physical sharing, use identical filters.
The native interest must survive one recipient's stop through last use by the other.
[MQTT member and prefix checks](../protocol-bindings/protocols/mqtt/src/preparation.rs#L257)

### Apply the contracts to longer lifetimes

Retain the Action invocation beyond its first exchange, authorize each original
control, validate output against the returning operation and fund aggregate
inventory/visibility work before lookup. Advance only the supplied clock to
expire and release a retained Producer Action. This distinguishes deployment
control from a poll loop that still reads process time. [Producer retention clock](../servient/finite/src/action.rs#L727) · [Action control evidence](../docs/evidence/consumer-action-composition.md)

Before fixing the Producer notification interface, use a downstream callback
mapping with separate establish and cancel Forms. This depends on directional
control data, actual destination authorization, coherent publication and funded
work that can outlive its exchange. Deny an offered callback address before
acceptance or callback I/O. For an accepted subscription, deliver an occurrence,
accept a conforming application reply and reject a malformed one before
application handoff. Cancel via the separate Form. Replace the Thing while
another delivery or reply is outstanding. Late replies and old cancellations
cannot reach the successor. Observe final release and service to a sibling. Preserve broker-owned
MQTT as the independent check on fan-out, loss and stop without local subscriber
records. Use caller-supplied time and storage for representative continuing
work and retain the mixed Host progress controls. [Mixed Host evidence](../docs/evidence/host-execution-integration.md#real-execution-and-adversarial-evidence)

### Migrate authority and callers at the affected boundary

Correct context handling, affordance validation and pre-credit work under
their existing contracts once implementation resumes. Changes to public
ownership have a different prerequisite. Record each stable cross-domain
decision in the ADR and affected registered owners before changing governed
production behavior. Supersede Interaction Core's direction-independent
validation clause. Specify whole-set commit consistency in Planning/Runtime
Safety, per-audience use in the security/Binding contract, and conditional
Producer subscription ownership in Core, Servient and its native handoff.
Supersede the finite Producer's one-exchange driver and mandatory queue-credit
precheck, and shorten the MQTT writer lifetime while preserving funding for
every accepted outcome. [Interaction Core](../docs/spec/interaction-core.md) · [Finite Producer capacity contract](../docs/spec/runtime-safety.md#finite-producer-participation-and-work-capacity) · [MQTT publication profile](../protocol-bindings/protocols/mqtt/README.md)

For each viable replacement, migrate normal callers, examples and live
restricted/static uses, reconcile its API and work-package projections, and
re-establish affected execution evidence. This can accompany each handoff.
The complete architecture claim depends on their integration and independent
review. [Authority owners](../docs/spec/authority.toml) · [Engineering priorities](../PLAN.md#engineering-priorities-and-dependencies)

## Decisions that still need evidence

| Decision | Working choice and what would change it |
| --- | --- |
| How a reloadable service guarantees qualified use | Prefer an immutable snapshot. Before adopting the qualification interface, rotate a real provider between check and entry. If a snapshot cannot cover its first effect, the provider must serialize the check and use or refuse the operation. A Servient lock cannot order a reload outside that lock. |
| How a native backend transfers an offer and its buffer | Prefer moving an exclusive opportunity when the SDK permits it. Before adopting the installation interface, exercise a backend lending its own slots. Compare holding a paid slot with making a bounded, funded copy at acceptance when a result can outlive the exchange. Late disposal must release the correct slot generation. |
| How much to retain between selection and service | Start with one admitted input, selected facts and paid bounded scratch. Incomplete coverage needs fresh qualification against a configuration valid through commit. Compare eager tables with selected loans or shared indexes. Measure concurrent staging and peak/post-commit bytes before choosing a shared scratch pool. |
| Does finite recipient attribution need an extra logical check? | Retain Binding correlation plus current Servient checks. Reopen on a concrete conforming multiplexed topology that can misattribute an accepted request across replacement. A copied candidate, a nonconforming Binding or duplication of HTTP path matching would settle a different question. |
| Does managed lookup need duplicate or anonymous TD IDs? | Use fresh local authority for independent installs. Keep the TD-ID convenience API's unique-ID rule. A requirement to recover independently revocable instances from one registry when IDs coincide or are absent would justify an application key. |
| Which costs justify more machinery? | Choose small bounded tables and explicit progress as initial controls. Measure mapping width, payload width, quiet leases per service, funded backlogs and latency on the normal driver. Adopt a ready-work index, single-pass object API or shared maintenance turn where it improves the intended workload while preserving attribution, duplicate validation, fair progress and bounded disposal. |
| Which deployment constraints change delivery scope? | Use the roadmap's Host-first delivery and source-free constrained execution for foundational evidence. On-device TD admission needs a specific provisioning and memory budget. A production authenticated notification profile needs a named source and trust mechanism able to attest the original occurrence. Those product commitments determine which concrete profiles to qualify. |
| Must v1 keep an identical exclusive route continuously available through failed replacement? | The proposed baseline permits a documented service gap when the old native lease must close first. The HTTP fixture explicitly closes it before starting the successor and preserves the old result. An Owner requirement for uninterrupted service would require a Binding with a qualified native handoff and separate failed-cutover evidence. [Same-route replacement](../protocol-bindings/protocols/http/tests/composed_thing/shared_listener.rs#L260) |

## Foundation completion and product development

The foundation is complete when one reviewed revision supports these claims
through the normal public entry points and an independently implemented
Binding. They are completion criteria for the Engine design and its execution
model. The proposal itself is at the design-review stage.

- A Consumer selects across unrelated implementations through one ordinary
  entry. On the same TD it can execute a later viable Form, explain ambiguity
  or missing support, and report exhausted search credit. The chosen provider
  enforces the complete selected meaning through real I/O.
- A Consumer accepts permitted additions to received data and prevents their
  transmission in a role that cannot generate them. Each credential is used
  only for its authorized audience and actual effect, demonstrated with two
  distinct recipients and a refused mis-scoped use.
- A Producer publishes only when coverage of the whole final TD, including
  provider uniqueness, is true under one effective configuration through local
  commit. A changing competitor cannot open ingress by supplying incompatible
  negative answers. A source-free install verifies the same deployment claim.
- Through an independent mapping, Servient and the Producer application can
  deny a callback, own an accepted subscription beyond setup, authorize its
  cancellation, validate a supported application reply and retire late work
  after replacement. A broker-managed source can publish without taking on
  that subscriber state.
- A definition can do useful invariant compilation once for compatible
  instances. Checked mapping, independent permission, actual capacity, native
  specialization and final backing release remain observable. Measured
  source-to-ready costs and warm dispatch expose the price of both shared and
  specialized representations, including the single-instance case.
- Two Things sharing real infrastructure can accept, refuse, replace, fail and
  drain independently. A local failure stays with the obligation whose promise
  failed. A genuinely shared failure reaches the affected dependents. Unread
  outcomes and stalled cleanup keep their original charges. Unrelated new
  permission or queue work cannot prevent local close.
- Finite exchange, continuing invocation, notification/response and Producer
  publication/retirement lifetimes work through representative Host and
  source-free caller-owned paths. Exercise alternate driving, supplied
  clock/storage and native I/O, including failure and short credit. Verify the
  supported constrained build and execution-only dependency graph. Compilation
  reuse, instance independence, executor independence and storage independence
  each have their own evidence.
- The architecture, domain contracts, public API and applicable work packages
  describe the same ownership rules. Retired entry points have a disposition
  and scoped successor evidence for the obligations they carried. Independent
  same-revision review can trace each material claim to code and execution.
  [Architecture closure policy](../ARCHITECTURE_GOVERNANCE.md#architecture-closure)

Production webhook hosting, proxy support, authenticated notification profiles,
new codecs and broader schema coverage are product capabilities. The foundation
needs representative executions of the ownership, direction, audience and
selection contracts above. A deployment can decline a concrete profile before
publication or use. Discovery/Directory clients and disjoint HTTPS work can
proceed when their own dependencies allow. Strict ingestion, on-device
preparation, hardware qualification, sustained workload targets and release
readiness each need evidence for the advertised deployment. [Enduring goals](../docs/architecture/00-system-goals-and-context.md) · [Roadmap and release milestones](../PLAN.md#milestones)
