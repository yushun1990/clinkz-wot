# Logical subscriptions versus physical Broker subscriptions

Status: OPEN

## Question and motivating counterexample

Can ClinkZ-WoT preserve independent WoT logical consumption while using a
bounded, potentially much smaller number of native Zenoh/MQTT subscriptions
for many compatible physical sources and/or independent consumed Things?

Consider 10 million homogeneous electricity meters that publish under
device-qualified Zenoh key expressions or MQTT topics. A cloud application
may expose a fleet-level virtual Thing, individual Things, or both. It may
need collection-level observations, independently authorized per-meter
observations, or application-defined fan-out. Neither the physical device
count nor the Thing/ConsumerHandle count should *by itself* prescribe
10 million Broker subscribe calls, transport sessions, retained drivers,
or delivery queues. Conversely, a single wildcard subscription or one
fleet Thing does not automatically preserve per-device WoT source meaning,
security, isolation, delivery guarantees, or sufficient throughput.

The question differs from [topic 0081](0081-reusable-plan-and-thing-identity.md):
0081 addresses cross-Thing reuse of immutable *execution definition* versus
per-Thing generation ownership. This topic addresses reuse, coalescing or
partitioning of *live transport subscriptions* versus the application's
logical subscription identities and delivery contracts. One can be solved
without solving the other.

## Existing authority and evidence boundary

- [Binding SPI](../docs/spec/binding-spi.md) gives the concrete binding
  protocol-local client/session state, native multiplexing, routing and
  flow control. Servient retains semantic admission, subscription lifecycle,
  scheduling, resource governance and cleanup authority. Native sharing
  must not become independent binding-owned application dispatch.
- [Subscription flow](../docs/architecture/10-primary-data-flows.md)
  describes distinct `SubscriptionId`s, a linear application receive cursor,
  independently admitted logical subscriptions, driver/slot capacity and
  generation-safe stop/drain. It also states that standard Thing-root
  collection operations use one native/coalesced driver rather than silently
  expanding to N per-affordance subscriptions.
- [Planning](../docs/spec/planning.md) records
  `NativeMultiplexed`/`BindingCoalesced` collection topology and
  `ExactAffordanceTarget` attribution for eligible *Thing-root collection*
  operations. This does **not** prove arbitrary subscription sharing
  *across Things/ConsumerHandles*, nor an application-visible physical
  device identifier for many sources mapped to one logical target.
- [Historical source attribution topic 0003](0003-subscription-source-attribution.org)
  is MIGRATED: protocol topics/key expressions remain in binding artifacts
  and must not silently replace protocol-neutral application source identity.
  Different observable WoT semantics require distinct logical targets or a
  reviewed extension.
- [Subscription receiver topic 0011](0011-subscription-receiver-ownership.md)
  is MIGRATED: a single non-cloneable application receive capability has
  one cursor; independent logical consumers use distinct admitted identities.
  A shared transport must not accidentally create clone/broadcast semantics.
- [Architecture roadmap](../PLAN.md) requires explicit long-lived
  subscription/emission domain-entry review and one minimal `ObserveProperty`
  cross-package proof *before* broad subscription/emission expansion. The
  current Consumer Property Read one-shot tranche and [PR #168](https://github.com/yushun1990/clinkz-wot/pull/168)
  do not prove a retained or cross-Thing subscription topology.

The historical/deferred long-lived SPI describes one driver with a receive
cursor, protocol resource and binding-local cleanup state. Whether such a
driver may own a *lease* or an independently accounted virtual cursor into
shared native state, without weakening its unique lifecycle, is unproved.
No current text should be interpreted as empirical evidence for automatic
cross-Thing sharing, nor as requiring one physical Broker subscribe call
per logical subscription.

## Identity and cost dimensions that must not be conflated

- Physical devices and authenticated protocol sources.
- Logical WoT Things, affordance targets, and per-source application meaning.
- Consumed handle and plan-set generations, including optional reuse of
  immutable compiled bytes (topic 0081).
- Admitted logical subscriptions, their independent receive cursors,
  delivery semantics, authorization, cancellations and cleanup.
- Native transport sessions/connections, Broker subscribe registrations,
  wildcard expressions, shared groups/partitions and binding receive state.
- Concurrent application processing, bytes/items in flight, fan-out,
  backpressure, work, CPU and memory costs.

The number of native subscriptions is not simply the number of Things,
handles, devices or concurrent handlers. It is constrained by protocol and
Broker capabilities, matching rules, tenant/security boundaries,
throughput, scheduling and configured limits.

## Candidate responsibilities to investigate, not freeze

1. **Application/platform user** specifies which logical sources/interactions
   to consume, isolation and delivery requirements, application processing
   and deployment policy/limits. It should not have to implement routine
   protocol-specific subscription pooling for ordinary WoT consumption.
2. **Planning** validates/compiles admitted matching and source-mapping facts,
   including URI/key/topic variable extraction where applicable, without
   creating sessions, subscribing to a Broker or assigning runtime cleanup.
   It must not invent a new application-visible protocol identifier.
3. **Servient** owns each logical subscription's admission, generation,
   lease/cursor, delivery and stop semantics; coordinates bounded scheduling,
   lifecycle/resource accounting and cleanup. It may govern shared capacity
   without owning protocol-specific matching or network calls.
4. **Concrete Binding** owns native subscribe/unsubscribe, sessions, protocol
   matching and multiplexing/coalescing, local routing/correlation and bounded
   receive resources. Any physical sharing must preserve the separately
   admitted logical obligations; protocol I/O never grants hidden permission
   to dispatch an application handler.
5. **Broker/network** determines which wildcard/group/partition and delivery
   behaviors actually exist. MQTT wildcard matching and MQTT shared
   subscription groups have distinct fan-out/load-balancing semantics;
   Zenoh key-expression matches do not inherit MQTT group semantics.
   Do not infer one transport subscription is safe, sufficient or desirable
   merely because one pattern covers every source.

These are investigation boundaries derived from existing owners, not
a selected subscription-pool type, mandatory global dispatcher, new public
facade, or active normative requirement.

## Concrete falsification scenarios

- **Independent logical stop:** two separately admitted Things/handles or
  logical subscriptions share one compatible physical source subscription.
  Dropping/stopping one cannot revoke the other's active delivery; stopping
  the last logical owner must eventually release or explicitly account for
  the physical resource. Test races with late messages, cancellation, drain
  and protocol unsubscribe/cleanup failure.
- **Accurate source attribution:** a message on
  `meters/123456/telemetry` or matching Zenoh key is parsed using bounded,
  compiled matching rules; source/instance variables are associated with
  the proper logical target. An unauthenticated topic string alone cannot
  authorize or identify a tenant or meter. Reject ambiguous, malformed,
  spoofed or stale-generation mappings; account for parsing work and memory.
- **Independent consumers vs broadcast:** two logical subscribers to the
  same source must receive the delivery semantics they requested. A binding
  must not silently translate MQTT shared-group competing-consumer delivery
  into broadcast (or vice versa), nor treat overlapping wildcards as
  exactly-once delivery without evidence. Each logical receive cursor,
  item/byte bound and loss/backpressure policy must remain explicit.
- **Tenant/security/config compatibility:** two identical meter models
  with different credentials, security policy, endpoint namespace,
  binding generation, subscriber options or delivery semantics must not
  share physical resources unless a checked compatibility proof permits it.
  No cross-tenant payload leaks or privilege escalation from merged routes.
- **One fleet target versus individual Things:** demonstrate when a single
  collection-level logical subscription with source attribution is sufficient,
  and when millions of independently addressable WoT targets genuinely
  require more logical state. The standard `AffordanceTarget` alone must not
  be presumed to carry arbitrary device instance identity; reconcile that
  gap with the migrated 0003 decision rather than silently exposing raw
  protocol topics in `SubscriptionItem`.
- **Bounded scaling:** compare 1, 1,000 and 100,000 logical sources/consumers
  and, where practical, a 10-million-identity simulated fleet. Record
  physical subscribe calls, sessions, routing table/wildcard costs,
  per-instance/cursor memory, callbacks, buffers, work, cleanup and
  throughput. State separately the counts that grow with device identities,
  logical subscriptions, physical subscriptions, partitions and delivered
  samples. Demonstrate backpressure/fairness under a hot source, idle peers,
  wildcard overlap and reconnect storms. Do not present simulated counts as
  actual deployed throughput evidence.
- **Profiles and reclamation:** challenge both Host and constrained/static
  capability projections to the extent that each profile claims support.
  No unbounded global pool, hidden side table, automatic worker, uncharged
  shared allocation, retained dangling subscriber or premature last-owner
  cleanup. A profile may explicitly reject unsupported topology.

## Alternatives to compare

- One physical subscription/driver per admitted logical subscription:
  baseline with straightforward lifetime semantics; measure unnecessary
  session/registration and duplicated receive costs.
- One native collection/wildcard subscription per compatible logical group:
  fewer Broker registrations but bounded route demultiplexing and source
  attribution are necessary.
- Binding-local refcounted/leased physical subscription with independently
  admitted logical drivers/cursors: potentially broad reuse, but requires
  exact ownership, compatibility, accounting, stop and residual-cleanup proofs.
- Sharded, partitioned or application-selected acquisition topology:
  may be necessary for throughput, distribution, security or protocol
  capability, even when one wildcard matches all devices.
- Explicit application fan-out from one logical subscription:
  legitimate application design, but not a substitute for independent engine
  logical subscription semantics when those are requested.

No candidate is selected by this OPEN topic.

## Revisit trigger, scope and governance

Revisit **during the long-lived ObserveProperty/subscription domain-entry
architecture review and before freezing cross-Thing subscription-sharing
APIs, binding driver ownership or fleet scalability claims**. The review
must examine current normative sources, actual binding implementations,
protocol behavior and negative evidence, then decide whether to migrate
requirements, change ownership, constrain supported topology or leave
application-defined fan-out as the sufficient v1 solution.

This is a non-authoritative, non-blocking counterexample. It does not reopen
MIGRATED topics 0003/0011; activate inactive subscription requirements;
change `PLAN-SET-001`, `SubscriptionId`, `AffordanceTarget`, driver contracts,
resource defaults, public API, package admission, performance budgets or CI
gates; or block the independently reviewed one-shot Consumer Property Read
tranche/PR #168. A demonstrated *positive contradiction* with an irreversible
API/ownership decision must be raised through
[architecture governance](../ARCHITECTURE_GOVERNANCE.md), not silently patched.

Related: [0081 Plan reuse](0081-reusable-plan-and-thing-identity.md)
and [0076 runtime/product boundaries](0076-composable-capabilities-and-runtime-boundaries.md).
