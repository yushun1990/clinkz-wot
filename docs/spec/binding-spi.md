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
Terminal reconnect failure is a separate outcome: fence affected acceptance,
report the first cause and dependent availability/terminal outcomes, retain native
jobs and backing until bounded retirement or acknowledged whole-owner transfer,
and report remote effect uncertainty. It does not imply a shutdown request or
authorize implicit replay of semantic effects.

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
Their actual safety checks MUST NOT be weakened while retained. The authority
migration itself introduced no new facade or interaction-family implementation;
later composed HTTP Action delivery uses the successor boundary below. Historical
contracts/evidence remain available at the pinned predecessor.

The optional composed Host Producer handoff supplies a closed participation
lease for each independently installed native contribution. It checks the exact
owned definition/operation against one unambiguous participant, plus readiness,
and requires every participant to cover at least one operation of that same
qualified exposure. Finite readiness also checks the exact logical instance,
requires input/output validation to be enabled and verifies its captured service;
an explicitly raw runtime cannot qualify a published exposure. An unrelated
closed runtime cannot be opened as an extra participant. The
handoff installs separately owned closed ingress gates before commit and flips
them inside the Servient publication boundary. Third-party `open` callbacks
after logical commit are not permitted: reentrancy or partial opening would
leave a multi-participant publication observably half-open despite a non-fallible
contract. Gate loans and checks finish before commit; no linked callback,
allocation or fallible native operation occurs while flipping them. Revocation
closes all original gates without calling the native drivers. A lease is not a
permanent receive grant: a Serving participant checks its gate before every
new semantic ingress, including after an earlier exchange. Owned replies and
native cleanup can still finish after the gate closes. A lease is not a
native connection or per-Thing Binding runtime: one Binding owner
can retain many leases and Things, and closing one releases only that participation.
The HTTP finite participant can share one listener across up to 32 independently
prepared Property Read/Write mappings; native method/path collisions refuse before
setup. `FiniteIngressBinding::validator` supplies one captured `FiniteValidator`
that checks the original candidate's separate input/output roles. There is no
HTTP method, status or retry case in this SPI. A linked
test-only UDP Action participant exercises shared infrastructure and continuing
work over actual I/O. The HTTP Action participant adds a production loopback
 mapping with independent invoke/status/cancel Forms, publication-captured JSON/type
 validation, retained native correlation and manually driven continuing work.
  The composed handoff captures the actual invoke-route validation service and
  each described query/cancel mapping from selected native participants before
  publication. Missing services or control routes refuse qualification; after
  commit the Servient checks the captured Action and operation routes, so a raw
  request cannot downgrade published execution or invent a control Form.
UDP remains test-only; HTTP's one-listener-per-Action implementation does not
constrain other native resource layouts or cross-Binding Action routes.

Producer notification participants additionally supply their exact emission mapping
and actual validation/security services for each selected Property observe or Event subscribe Form before
publication. These services and the complete data allowance are recaptured and
compared before commit; missing or substituted services refuse. Native notification
publication has separate start, progress and retirement controls, not a finite
reply opportunity. Native subscriber matching, subscribe/unsubscribe control and
distribution stay at the Binding/broker; no physical subscription cardinality is
inferred from logical source or recipient counts.

The current manual Host publication bounds the captured notification mapping table at
32 entries, independently of its 16-source per-instance registration envelope.
Larger mappings require a separately bounded lookup/progress representation.

The bounded MQTT Host participant uses one application-owned service across
Thing participations, with one shared in-flight publication slot retained through
unread settlement acknowledgement. Native resource policy permits no reconnect,
replay or idle pooling. Dropping/retiring one participation preserves siblings;
last actual participation/work disposal releases the connection. Partial TCP
frame cancellation invalidates the shared native service, reports uncertainty
and fences dependents rather than appending a sibling frame to truncated bytes.
Only its unique service owner can explicitly shut down the whole infrastructure.
QoS 0 local write completion carries no subscriber-delivery acknowledgement.
This is a concrete mapping policy, not a requirement on all notification Bindings.

The manual Host publication records bounded opaque identities of the original
native participation owners. Publication and retirement reject duplicate,
foreign, omitted or substituted participants. These identities retain no logical
permission or native shutdown authority. An optional owned composition handoff
retains the actual participants and their transferred capacity owner; typed native
progress remains specific to each capability. A managed participation must
declare its additional controlled backing before effects and retain the original
reservation through native/result disposal, including when moved out of a typed
loan. Its Drop owes the existing finite local-disposal contract. Shared native
infrastructure keeps its Binding-owned policy and final-use lifetime.


Open finite Consumer registration pairs the actual portable `ConsumerCompiler`
with the immutable `HostConsumerBindingRegistration`; refusal returns both
original owners. Typed/static pairing has the corresponding public generic path.
Eligibility does not downcast to a built-in compiler, require private inline
support, or require a Copy artifact. Generic compiler erasure keeps its original
owned cursor/output/failure-return mechanisms. A compiler's inspected footprint
includes variable backing and destruction, not merely its erased slot Layout.
The actual data and destination/access services remain captured through execution;
a caller supplies no replacement executor, codec, validator or configuration to a
finite operation. `ClientBinding::supports_operation` is only coarse eligibility;
the actual compiler and captured services prove complete Form support. The Host
Consumer entry is `start_finite`; the incorrect Property Read-only entry is
removed there. Retained dual-role/static Property Read adapters still reject other
operations explicitly. Finite requests carry their selected operation and optional
application input; they do not represent continuing Action or notification
conversations. Existing inline aggregate and raw finite APIs retain their
restricted purposes. Notification preparation uses a separate typed facet:
`ConsumerNotificationPreparation` compiles owned programs on the actual
`ConsumerNotificationBinding` registration. Its operation-scoped Property/Event registry
captures actual services and configuration without adding finite methods or
passing TD/compiler owners into execution. MQTT supplies that real native facet;
its shared topic interests, queues, control correlation, wakers and final socket
release remain Binding-owned. Each Engine recipient retains independent logical
acceptance, value/data validation, loss and stop. A native program retains its
Property/Event kind, so source-free pairing cannot relabel an Event subscription
as Property observation. The Property DataSchema is distinct from all Event
roles; no native subscription ACK is a Property value. Separate control/response capabilities
must be supported explicitly; protocol ACKs cannot stand in for application data.
