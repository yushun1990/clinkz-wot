# Binding capabilities and native runtimes — v6.0

These contracts define responsibility and acceptance boundaries. Exact trait/
enum/erasure layouts are implementation choices within them, not a universal
transport API. Finite Consumer conversations, notifications, Producer exposure
and Discovery have different controls. A Binding can implement several facets
through one private driver without dummy symmetric roles.

`SPI-COMPOSE-001`: Applications MUST be able to Cargo-link and startup-register
complete conforming third-party capability implementations without a built-in
compiler/protocol whitelist or workspace membership. A registration captures
actual identity/configuration, supported combinations, native progress/ingress
limits, visibility/delivery/retirement policies and required services. Runtime-only
registration MUST NOT retain a compiler or TD dependency. Preparation pairs the
actual executor/configuration/format/policy, not just a tag/type. A registration
receives no application-dispatch handle. One Servient's set is frozen after
construction; code loading/hot unloading is not selected.

`SPI-NATIVE-001`: Bindings MUST own protocol mapping/defaults, native matching/
captures, correlation, conversations, I/O, flow control, distribution and physical
infrastructure. Core/Planning/Servient MUST NOT add protocol cases or a universal
RPC/session/matcher. One logical Thing MUST NOT imply one native resource.
Native progress has its own owner/budget and may receive/match traffic without
any candidate Thing's serving authority. Concrete callbacks may enqueue bounded
observations/wake progress but cannot select/call application behavior.

`SPI-INGRESS-001`: Matching MAY yield zero/many original logical candidates plus
native captures and retained input backing. Each candidate MUST pass independent
Engine identity/publication/policy/data/capacity checks before semantic acceptance.
Exclusive request ownership ambiguity refuses; supported multicast can have
multiple recipients. Mapping support declares reply opportunities/cardinality
and enforces response-bearing topology before acceptance. Engine MUST NOT invent
a universal unique-replier, aggregation, arbitration or broadcast layer. Deferred
input cannot retarget a replacement generation. Prepublication/draining input
uses a declared bounded reject/buffer/control policy, never early dispatch.
A capability unable to enforce that local boundary is unsupported.

`SPI-DELIVERY-001`: Before acceptance, rejection MUST return the complete input
and reply/publication opportunity. After acceptance an explicit owner MUST retain
pending work, original recipient/mapping, result/late value, effect certainty and
cleanup path. Byte views MUST retain actual physical backing across callbacks;
no borrow may outlive a native stack/buffer. Immutable backing MAY be shared
without body copy/fresh decode per recipient, with actual recipient costs paid.
Binding-native correlation, ACKs, retries inside an admitted mapping and wire
status conversion remain native responsibilities, bounded by declared policy.
They MUST NOT silently replay uncertain semantic effects.

`SPI-RETIRE-001`: Every native resource MUST have one physical owner and explicit
compatible participation/work leases. Logical close releases only its participation
and cannot stop siblings. Jobs, packets, replies, callbacks and SDK work retain
the actual retirement owner until terminal disposal. Final destruction waits for
final use. Idle retention/pools, reconnect, failure and explicit shutdown are
distinct bounded policies; last-route release need not trigger immediate close.
Only an authorized infrastructure owner may initiate whole-service shutdown,
revoking/draining affected dependents. Polling or a participation clone grants no
shutdown authority. Cross-Servient sharing requires an application-owned service
with distinct compatible trust/account leases, not an Engine session manager.

## Publication and security collaboration

Producer publication follows [PREP-COVER-001](planning.md) and SAFE-PUBLISH-001.
Native setup may be externally visible while semantic ingress remains closed.
Readiness means required exposure participation is ready, not all future
connections/subscriptions/results preallocated. The Binding supplies endpoint
and collision facts; Engine publishes only final qualified meaning and actual
behavior. Remote registration/cached TD visibility is not globally atomic.

Bounded authentication parsing/handshake may occur before final authorization;
only configured verification establishes a principal. Native source evidence,
captures and actual destinations feed Engine policy. Credential services may
use typed/opaque capabilities; no protocol security enum belongs in Core.

## Resource and progress representations

Engine-controlled state/promised handoffs follow Foundation. Bindings enforce
native input/growth/concurrency/pool/retry limits at their real owners. Deployment
owns OS/SDK failure containment. Cooperative bounded callbacks are contracts of
trusted linked code, not preemption/sandboxing guarantees. Static associated
state/caller-owned slots and Host erasure/owned calls may differ physically;
both must meet supported semantic ownership/progress/cancellation outcomes.

## Removed general contracts and retained adapters

ADR-0025 supersedes ADR-0002/0003/0005/0008/0009/0010/0012 physical APIs and
ADR-0024's mandatory final-route retirement trigger. The prior route-stage,
route-scoped poll_accept permit, equality-key collision and common outbound
request inventories are no longer general SPI authority. The same single-owner,
closed semantic acceptance, cancellation-late-value and acknowledged transfer
invariants remain mandatory under the requirements above and Runtime Safety.

Existing Core route carriers, HostBindingCall, Consumer registrations and the
single synchronous Property Read Producer remain restricted working adapters.
Their actual safety checks MUST NOT be weakened while retained. No new facade,
new Binding or interaction-family implementation is introduced. Their exact
historical contracts/evidence remain available at the pinned predecessor.
