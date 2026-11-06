# Interaction Core — v6.0

Core defines protocol-independent intent, data/security checks, identities and
observable ownership outcomes. It contains no protocol switch, TD parser,
universal RPC/session, scheduler or application registry.

`EXEC-ID-001`: A live reference MUST identify its issuer/registry, slot,
generation and admitted obligation, or carry an equivalent lifetime/ownership
proof. TD id, endpoint, content, Rust type or numeric equality alone is not
execution authority. Deferred observations, wakes, late results and replies MUST
remain tied to their original recipient, mapping, principal association and work
owner through settlement; slot replacement cannot redirect them. Bindings also
fence replaced sessions/correlation namespaces. Diagnostic context grants no
permission and MUST redact payloads/credentials under bounded formatting.

The Host Consumer implementation uses a move-only `ConsumerInstance` authority
and retained `ConsumerInstanceRef` identities. Equal numeric slots from separate
owners are distinct. A reference retains identity and passive currentness, not
revocation power or liveness after authority Drop. Finite calls and raw
notifications retain that same reference through their separate settlements.

The finite handoff distinguishes immutable `ConsumerExecutionContent`, a
qualified native `ConsumerDefinition`, and an instance-local `ConsumerBinding`.
Content records the exact affordance/operation/data roles; it grants neither
permission nor a service-support certificate. A definition retains its actual
mapping and data dependencies. Binding policy/accounts and attaching a current
logical authority remain independent steps. Calls and settlements retain these
original owners; no sibling or successor content/instance can substitute for them.

`EXEC-HANDLER-001`: Servient MUST select and capture actual registered behavior
for the accepted target/operation; Bindings MUST NOT call application handlers.
Requests contain dynamic data and context contains dispatch identity, not mutable
registry authority or credentials. Callbacks run outside locks/critical sections.
Synchronous callbacks are cooperative and non-preemptible: check cancellation/
deadline before entry and after return, retain late results, and never claim an
enforced execution deadline for code that does not return. Enforced bounded
progress requires a qualified cooperative poll/async behavior. Owning a future
or selecting async syntax alone does not prove cancellation or cleanup.

`EXEC-DATA-001`: Core MUST define media-typed payload and executable codec/
validation contracts; TD/Planning prepare shared DataSchema constraints for
source-independent execution. Structured values and opaque media bytes/streams
are permitted; no forced JSON conversion or per-recipient copy/decode exists.
Required bounded decoding/schema checks precede behavior input and claimed
validated delivery. Response classification includes the selected primary or
additional response and its declared schema/data role. Unsupported required
schema/encoding/stream combinations refuse the guarantee. An explicitly raw
API reports unvalidated data as such. Identity/shape sealing alone MUST NOT be
represented as schema conformance. Native status mapping remains Binding-owned.

Consumer data services validate a payload independently of direction. Prepared
`ConsumerSchemas` keep single-value input and output distinct; absence of a data
role differs from a present unconstrained value. `ConsumerFiniteSchemas` retains
those roles, derived Property collection member schemas or named Action output roles explicitly.
The current finite path supports Property Read/Write, explicitly synchronous
Action invocation (including empty data roles), and Thing-root
ReadAllProperties/ReadMultipleProperties/WriteAllProperties/WriteMultipleProperties and QueryAllActions. Shape/identity seals require terminal
primary application results and refuse continuing status/invocation references; captured codec/schema
checks separately precede input effects and validated result handoff.

A prepared Write requires the Property value as input and supports empty terminal
application completion. It cannot reuse a Read definition or result schema.
`readOnly` excludes Write and `writeOnly` excludes Read even with an explicit
contradictory Form operation. Explicit Forms never acquire omitted operations.
Required value constraints and effective security must be supported in full
before selecting that operation; unimplemented assertions are not ignored.

`data_schema::PreparedSchema` replaces the former Copy `TypeSchema`. It owns
bounded source-independent constraints and is borrowed by executable data
services. The shared semantic representation uses alloc without std or pointer
atomics; Host content owners choose their own shared/funded representation.
Core evaluates constraints through `ValueAccess`, whose codec-owned handles
supply actual kinds, exact numeric evidence, Unicode code-point lengths and
named child values. Encoding syntax, decoded keys, media and bounded access cost
remain codec responsibilities. No JSON tree or universal schema VM is required.
`integer` includes mathematically integral decimal/exponent encodings; binary
floating-point rounding cannot establish integrality or boundary comparisons.

The finite structured subset captures object properties and required names,
including required names with otherwise unconstrained values; omitted properties
remain optional and unspecified members remain unconstrained. It captures signed
64-bit IntegerSchema inclusive/exclusive bounds and StringSchema code-point
min/max lengths, plus the existing unconstrained primitive kinds. NumberSchema
bounds are refused because the current TD facts use f64; integer payload values
are compared exactly even beyond i64, without truncation or float conversion.
`multipleOf`, const/enum, oneOf, constrained arrays, pattern/format/content
transformations and unrecognized assertions remain unsupported. Assertions on
an omitted type refuse rather than silently disappear. See
[TD 1.1 Data Schema vocabulary](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#sec-data-schema-vocabulary).

Preparation bounds each schema to 64 nodes, depth 8, 32 fields per object,
256 UTF-8 bytes per name and 4096 total name bytes. Collection preparation
bounds the combined member constraints as well. Dynamic field/name backing and
bounded destruction belong to the original content owner. Actual alloc-only
semantic compilation is not proof of a static or non-atomic Engine runtime.
The JSON participant retains its 4096-byte envelope, bounds encoded nesting to
32, lends raw subvalues and temporarily decodes requested keys/strings under its
funded scratch policy. Duplicate described names, including escaped aliases,
refuse. Unknown unconstrained members do not acquire new validation assertions.
Callbacks remain bounded atomic cooperative work: prepay their declared complete
work and scratch; check permission/currentness before and after; preserve late
results and original funding. No mid-callback preemption or recoverable process
OOM is claimed.

Actual scalar validators explicitly qualify the complete borrowed schema;
`ConsumerDataService::supports_schema` defaults to primitive type-only support.
Collection/status facets independently qualify their complete named schemas;
they do not require an unused bare-scalar validation facet. A marker, successful
JSON parse or matching outer kind alone cannot qualify structured constraints.
The real HTTP finite Read/Write and ReadAllProperties integrations exercise this
subset, source retirement, checked reuse, invalid input before behavior and
normal/late result validation. Continuing Action/notification reuse, non-JSON
media interoperability, general schema/security and constrained execution remain
separate capabilities.

The restricted Host Producer executor must check output
with its captured program/services before successful delivery. Finite raw mode
remains explicitly unvalidated. The shared finite owner can require the actual
captured provider's `FiniteValidator` input/output facet, replacing the finite
output-only interface. It checks the original candidate's input before fresh
application authorization and behavior entry, and complete result shape/backing
before output validation. Both roles belong to the captured configuration and
remain tied to the original operation through native delivery/retirement.
This proves only that participant's data checks, not
complete TD support or publication. Missing/lost required services fence progress.
Serving requires an actual validation-service facet; numeric service limits do
not establish it. Native installation alone need not provide that facet. Result
shape/backing bounds precede the external codec/validation callback.

`EXEC-SECURITY-001`: Shared interpretation MUST resolve effective security,
Form override and allOf/oneOf composition. Security services evaluate selected
schemes, establish verified principals and apply application policy; Bindings
perform native trust/handshake/placement. Raw authentication evidence is not a
principal. Outbound policy authorizes the attempted destination/security action
before credential use; inbound bounded parsing/challenge I/O may precede final
authorization but cannot dispatch behavior. Recheck mutable policy at semantic
acceptance and applicable long-lived delivery. Credential capabilities are tied
to destination/audience, operation, instance and policy; they remain outside
prepared content, logs, behavior data and diagnostics. Body/URI credential
projection is bounded and avoids unaccounted decoding. Refresh has a retained
owner. Session sharing respects trust/tenant/credential domains. NoSec does not
bypass application policy. Extending a scheme MUST NOT require a protocol enum
in Core or silently broaden a prepared security branch.

The finite implementation carries `SecurityRequirement` as immutable
scheme/parameter operands and `OutboundSecurity` as an opaque per-call Binding
capability. Core does not interpret concrete scheme names, HTTP headers, TLS or
passwords. Actual access services explicitly qualify requirements, issue bounded
capabilities under their declared work and expose a passive live dependency
revision. Revocation/rotation never mutates prepared meaning or refreshes old
work. Authentication evidence stays with the original native reply; only a
verified Principal enters application input. Authentication carriers and opaque
capabilities redact formatting. The selected executable subset and limits are
owned by [the HTTP Binding](../../protocol-bindings/protocols/http/README.md).

`EXEC-ACTION-001`: Action invocation lifetime MUST be independent of the initiating
exchange. Completion may yield output/error or, where the mapping supports it,
a continuing invocation reference retaining its exact mapping, opaque continuation,
identity, policy and cleanup. Query/cancel use that association, their described
operations and fresh permission; no fabricated generic URL or session is required.
Producer application invocation state is distinct from Binding wire correlation.
Local abandonment/cancellation and known remote terminal state MUST be reported
separately. `synchronous` absence makes no completion claim. Unknown effect
MUST NOT authorize implicit replay of an Action or write.

The implemented raw Host Producer kernel uses a single Servient logical registry
for finite and Action behavior, but a separate bounded invocation store for
continuing Action work. It captures separate selected invoke/query/cancel route
identities (possibly different Bindings), the established principal id and the
original behavior. Query/cancel require a live local handle, original route,
current instance and fresh application permission. Cooperative cancellation,
local abandonment, application result and native delivery are separate outcomes;
an error after behavior entry is effect-uncertain. Alone this raw kernel makes
no TD or native-I/O claim; explicitly raw registrations remain unvalidated
beyond shape/backing checks. A published Action requires a captured validation
service for each selected invoke mapping and captured query/cancel mappings for
each advertised control Form at publication, owned by the same Servient
incarnation as behavior and currentness. Native requests cannot omit or
replace invoke validation or invent control routes: input is checked before
behavior and immediate/late output before successful terminal exposure,
retaining the original service through settlement.
The composed HTTP Producer supplies source-independent JSON/type validation and
separately described query/cancel routes. Its 202 means locally admitted work,
not remote terminal success or rollback. Other qualified Bindings must provide
their actual validation services during publication, not at individual ingress.

The Host `consumer_action::ConsumerActionRuntime` implements the continuing
Consumer association separately from finite results. Its unique local handle
pins the original Consumer incarnation and selected mappings; the Binding's
opaque token is retained independently of each finite native exchange and never
becomes logical authority. Invoke acceptance exposes Pending, not a completed
result. Explicit query/cancel can use different actual mappings and captured
access services. The original invoke data service validates completion, including
late output after cancellation; native output, validation error, first local cause,
cleanup error and effect uncertainty remain independently observable.

Native completion does not seal unread application success. `Completed` remains
revocable until `take_settlement(now, budget)` performs paid, fresh checks of
time, original instance/root authority and captured services/access. Progress and
local cancellation continue to fence unread completions. Short handoff credit
retains the complete original result and slot; an earlier progress check cannot
serve as cached authorization. Fencing preserves native output as evidence in a
failed settlement. Successful handoff is the local application acceptance edge;
subsequent revocation does not retract a value already handed to the caller.

A native poll error is retained immediately as `native_error()` and the first
local cause when none exists. Retirement still owns its original response
opportunity. Any late output returned by `poll_close` is retained in
`native_result()` and checked by the captured invoke validator; it cannot erase
the earlier error, skip validation or establish application success.

Cancellation request acknowledgement and already-terminal acknowledgement grant
no terminal application result. A subsequent query must obtain and validate it.
Native cancel failure or loss leaves remote cancellation uncertain. Local cancel,
abandonment, root close, deadline, policy/service loss or instance replacement
fences acceptance and retires local work; none sends an undescribed cancel or
asserts remote rollback. No progress/control path reinvokes an Action. Unread
terminal settlement retains the original slot; acknowledgement returns a complete
settlement owner, and stale handles cannot address a reused slot. The settlement
retains original services, native evidence and funding until final disposal.

A Thing-root **QueryAllActions** is a finite status observation, independent of
any Consumer invocation handle. Action definitions, invocation instances and
invocation output MUST remain distinct. One definition can have zero or multiple
concurrent instances and retained terminal records. The query MUST preserve each
record's Action attribution, native invocation identity, state and any supported
completed output/error evidence; one value per definition cannot represent it.
Status-resource links are native evidence, not Engine invocation authority or
permission to follow them. Query success does not imply invocation success,
remote cancellation, a complete historical ledger or a distributed snapshot.
TD 1.1 defines the root interaction, not a universal envelope/retention policy.
Concrete status vocabulary, representation, ordering and retention require an
explicit supported mapping. Unsupported native evidence MUST NOT be silently
collapsed into an unconstrained Object or a single Action result.

The Host Consumer retains `ActionCollectionSchema` separately from scalar data
roles. Its actual `ConsumerDataService` MUST explicitly support and validate the
selected status representation, including each named Action's own output schema;
scalar Object support is insufficient. The original complete native payload and
response metadata survive validation failure, cancellation and late settlement.
No aggregate observation creates or replaces local continuing invocation handles.

The Host Producer's `query_raw_actions` is one bounded borrowed observation of
a qualified Binding's original inventory. A distinct root `ActionQueryPolicy`
and captured Thing/QueryAllActions route are required; per-record original
instance/principal and fresh Action-handler permission additionally apply.
Member Security denial filters that record; root denial or another error refuses
the query. Authorization cannot confer control of a successor or change the
original handle association. Native inventory completeness and correlation remain
Binding obligations; the Engine neither performs N QueryAction calls nor
progresses application continuations. Raw use alone establishes no TD/native
completeness. Acknowledgement transfers a complete Action settlement owner;
original result/target/error storage and validation service remain owned until
that owner is disposed. Query and settlement output use slice-only
Core `InteractionOutputView`/`PayloadView`; they expose neither InteractionOutput,
Payload nor the original body's Arc. Target access similarly lends an affordance
site/name. Shared original data require retaining the settlement owner; an
explicit byte copy creates separate caller-owned backing. Returning a reference
to a cloneable Payload would not establish this ownership guarantee.

Each admitted invocation owns a non-cloneable result lease that moves into its
acknowledged settlement. `RawActionLimits::invocations` bounds live invocations
plus acknowledged results still retained by callers. Acknowledgement removes
logical/native records but does not free this result capacity. Managed publication
captures the actual original account bank; admission reserves each result's
maximum output/settlement backing and cleanup before behavior entry. Refusal
returns provisional reservations without consuming a slot or entering behavior.
Only actual disposal returns the result slot and byte/cleanup leases. Raw callers
supply their own bounded backing and receive the same retained-slot guarantee.

`EXEC-NOTIFY-001`: A logical notification conversation MUST own establishment,
control inputs, delivery, optional reciprocal responses, stop and cleanup. Property
observation carries Property values. Event subscription/data/dataResponse/
cancellation schemas keep their distinct roles. A dataResponse schema does not
create a native response channel; required response semantics need an actually
supported opportunity/cardinality before acceptance. Protocol ACK is distinct.
Application pull cursors/async adapters may sit over Binding push/callback/poll
mechanisms. Each Engine-owned subscription has independent permission, cursor,
limits and stop state; broker remote subscribers need not be local records.
Credits MUST NOT claim wire backpressure unsupported by the protocol. Explicit
Property coalescing is permitted; Event loss cannot be disguised as a newer value.
Loss/ordering/replay/initial-value guarantees are capability properties. Overflow
must report loss or terminate safely. Stop revokes local delivery first, then
progresses mapped removal/release; it remains possible when remote cancellation
is unsupported. Shared native subscriptions/backing retain compatible owners;
one stop cannot release a sibling. Sources may have zero subscribers; publish
success alone proves neither subscription lifetime nor recipient delivery.
Cancellation or funding failure before native establishment releases the retained
reservation and reports its first local cause without claiming remote effects;
after native setup starts, its owner remains responsible for bounded cleanup.

The raw Host Consumer facet implements these roles in `core::notification` and
`servient-finite::notification`. One source-free capability registration may serve many
independent logical subscriptions; separate roots may share compatible native
infrastructure. A non-cloneable cursor borrows retained raw data until explicit
advance; cloneable stop intent grants no receive/response or shutdown authority.
`RawNotificationView` lends Core `PayloadView` byte/media slices and an
`AffordanceSite` occurrence coordinate. It exposes neither a Payload/body Arc
nor a cloneable owner of the original occurrence name. The loan is tied to the
exclusive root borrow, so advance, progress/cleanup and root disposal cannot
invalidate data still in use. Explicit byte/name copies have separate caller-owned
backing and may outlive the loan; no detached shared delivery owner is introduced.
Event control inputs and reciprocal responses have distinct carriers. Required
responses refuse absent native support; each actual response opportunity has a
separate retained delivery/retirement owner. An exposed unused optional
opportunity can be explicitly declined without sending a dataResponse or stopping
the subscription. Pending/error retains its
reply owner, payload and capacity; only successful native retirement permits
cursor advance. Required and already submitted responses cannot be declined.
No initial Property value, replay, wire backpressure, TD qualification or schema
validation is inferred. The same owner also admits Thing-root Property collection
observations; constrained execution remains unimplemented by this Host slice.
Fixed loss status and one prepaid overflow/late handoff preserve cleanup when
the item/byte queue is full. Explicit owner cleanup works after every cursor is
lost; terminal acknowledgement releases the logical reservation. Native physical
retirement and remote cancellation/response outcomes remain separate.

The composed `ConsumerNotificationRuntime` consumes the selected source-free
Property, Property collection, Event or Event collection definition and captures actual services around that same raw
lifecycle.
It exposes the same paid `decline_response` operation for admitted optional
opportunities, retaining native retirement before advance without sending or
stopping; required and submitted responses remain non-declinable.
Its immutable validation hook distinguishes a Property's value schema from
Event subscription/data/dataResponse/cancellation roles, prepays data validation
before native handoff, and validates each
candidate before logical queue acceptance. Invalid or late data remains owned
through cleanup and is never exposed. Currentness is rechecked after validation
callbacks as well as native/policy callbacks. Each composed root prepays its
Engine queue/validation envelope from the original bank; native queue/connection
policy retains its separate Binding owner. This supplies TD-driven Property
observation and Event execution for the
declared MQTT subset. Property establishment has empty control input, fabricates
no initial value and performs no implicit Read. Mapped stop authorizes
`UnobserveProperty`; shared native cleanup does not revoke a sibling. Current
queues report dropped values or terminate, without coalescing. Raw calls gain no
TD qualification. Collection observations use the distinct retained schema and
root operations below; native Event response rights remain outside this subset.

Producer notification publication has its own Host facet in `core::publication`
and `servient-finite::notification_publication`; it remains separate from finite
requests and Consumer conversations. An application registers a
`ProducerNotificationSource::Property`, `AllProperties`, `Event` or `AllEvents` on the same logical Producer
instance as other behavior. Complete publication captures each selected observe
or subscribe mapping and its actual data/security services. A Property source is
not a Read handler, and an Event of the same name cannot satisfy it.
`publish_notification` checks current authority, exact captured target/mapping,
backing, fresh policy and validation before native acceptance. `PublicationData`
distinguishes PropertyValue, PropertyValues, EventData and EventOccurrence; mismatched family/data refuse before
native acceptance and return the original carrier. Source registration alone
cannot bypass complete TD qualification or captured services. No Engine record
is created for a remote broker subscriber and no Engine subscriber loop
replicates a publication.

Each admitted emission retains its original instance, native publication and
capacity until native retirement and settlement acknowledgement (or bounded
local disposal). The settlement preserves first local cause separately from
native effect: not published, uncertain, or locally published. Local publication
does not prove broker acceptance, subscriber presence or recipient delivery.
Cancellation/replacement/root loss stops pending work without redirecting it;
already escaped native publication retains its original effect. Native failure
fences affected authority and keeps its failure/cleanup evidence. The concrete
MQTT mapping's supported subset and real-peer evidence are recorded in
[its README](../../protocol-bindings/protocols/mqtt/README.md); it establishes no
general MQTT interoperability or Event dataResponse path.

`EXEC-COLLECTION-001`: Described root collection operations MUST preserve native
semantics and logical attribution, including partial failure and supported
atomicity. They MUST NOT silently become N individual calls/subscriptions.
Explicit application fan-out is a separate bounded capability with its own
promised guarantees. Native group distribution and matching stay Binding-owned.

The implemented finite collection subset is TD-driven **ReadAllProperties** and
**ReadMultipleProperties**, plus **WriteAllProperties** and **WriteMultipleProperties**, in both
roles. Its coordinate is `AffordanceTarget::Thing`, with its own actual root
Form and operation; individual Property Forms supply no collection mapping or
behavior. In accordance with [TD 1.1 §5.3.1.1](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#thing),
the ReadAllProperties output includes all described readable Properties and
excludes writeOnly Properties. `collection::PropertyCollectionSchema` owns those names and their
complete supported type-only assertions after source destruction. ReadOnly
Properties remain readable. Unsupported assertions on any included member refuse
preparation; an excluded writeOnly member supplies no output schema.

ReadMultipleProperties prepares the same eligible readable-member names and
schemas, independently of its per-invocation input. The Consumer accepts a bounded
set of exact names, rejects duplicates/unknown/writeOnly names before native
effects, and retains the selection with the original definition through settlement.
An empty selection requires an empty map. According to
[TD 1.1 §5.3.1.1](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#thing),
the default request is a name array serialized using the selected Form's media
type. The actual captured data service must explicitly support both encoding that
request and validating exactly the requested output members. A missing member,
an unrequested member (including another readable Property), a duplicate or an
invalid value refuses the whole result. The static schema is never narrowed by
one call, and a later call cannot replace the original requested set.

The actual captured data service must explicitly support the complete named map;
scalar or object type support alone is insufficient. The concrete codec checks
every member, with no silent omission, unknown/writeOnly member or duplicate.
Native response representation, status/failure mapping and transport remain in
the Binding. The initial HTTP mapping requires one complete successful object;
a partial group result or per-member failure has no successful partial-settlement
mapping. Application aggregation failure is an error for the original group
interaction. Engine dispatch enters one registered root behavior and never
synthesizes individual reads. Neither dispatch nor a complete valid response
establishes an atomic snapshot, simultaneous sampling or remote effect certainty.
Any stronger guarantee needs actual application/Binding support and evidence.

WriteAllProperties has a distinct writable-member **input** schema, independent
of either readable output map. Every writable (non-readOnly) Property, including
writeOnly members, MUST appear exactly once with its own schema-valid value.
ReadOnly, unknown, duplicate and missing members refuse the whole input. An empty
writable set permits only an empty map. The supported implementation is type-only;
unsupported assertions on any included member refuse preparation rather than
silently weaken validation. Individual Property Forms grant no collection support.
Complete input validation and group authorization MUST precede application
effects. Engine dispatch enters one distinct root behavior once, and native
execution performs one actual collection interaction; no N-WriteProperty lowering
is allowed. An empty successful root result reports completed application of the
whole group. The handler MUST NOT use success to mean merely accepted, partially
applied or individually successful members. Neither this completion nor validation
proves atomic commit, ordering, rollback or independent physical-state confirmation.

WriteMultipleProperties retains the same complete writable eligibility, but its
per-call input is a map of **selected** member values. The actual captured data
service validates every supplied name/value and derives the exact selection;
omitted writable members are not input obligations. Unknown/readOnly members,
duplicates (including decoded aliases), malformed data or any invalid value
refuse the entire input before effects. The bounded subset permits `{}` and
still performs one native interaction. The invocation retains its selection
independently of native input ownership and later calls. Unsupported assertions
on any eligible member refuse preparation, even if a caller might omit it.

`ConsumerAccessService::authorize_property_write` receives the original immutable
schema and validated selection before native admission and at each subsequent
permission edge, including unread handoff. Its default delegates to group
permission covering all members; a member-restricting policy checks the entire
set within its captured `work()` allowance. Producer authorization receives the
complete validated input before any member effect; discovering a denied member
cannot justify applying an earlier allowed member first.

Empty terminal success remains a whole-group completion option, not the only
protocol-independent write result. `InteractionOutputMetadata` may instead carry
`collection::PropertyWriteReport` for either root write. Its original-schema
indices identify the exact requested set, each member's Completed, NotApplied or
Unknown effect, optional opaque 16-bit native status, and Unspecified/Atomic/
NonAtomic evidence. Unclassified members remain Unknown; native status never
establishes effect by itself. Disjoint masks and exact original-set association
are required. An Atomic report with a known member effect must classify the
whole set consistently; mixed partial effects contradict atomicity. Neither
one exchange nor all-complete outcomes supply an atomicity claim. No arbitrary
native diagnostic tree, transaction coordinator or general protocol receipt
schema is introduced. Bindings needing richer diagnostics need an explicit
supported contract rather than silently truncating them into this carrier.

Member reports are terminal native evidence even when incomplete. Consumer
`native_outcome()` retains the report; `result()` refuses whole-group success for
an incomplete report using an Application error with CallerDecision retry advice.
A first local cause still takes precedence. Invalid original-set association
retains the untrusted report in `rejected_output()`. Scalar writes, Reads and
Actions cannot acquire this collection-only result. Producer mappings validate
reports against their original input and must encode the supported evidence or
refuse it, never replace an incomplete report with an empty success.
`ConsumerSettlement::property_write_members()` exposes the retained original
schema and invocation set, so report indices remain interpretable after TD,
registry and runtime destruction, including for rejected or late reports. A
Producer interprets the report against its captured original application/native
mapping; an index report never describes a successor schema or an arbitrary TD.

Collection outcome evidence has independent axes:

| Evidence | Meaning |
| --- | --- |
| Producer `FiniteEffect::NotInvoked` | Validation/authorization/admission refused before behavior entry; no application effect attempted. |
| Producer `FiniteEffect::Unknown` | Behavior entered without a valid complete result; some, all or no members may have changed. An error does not undo effects. |
| Producer `FiniteEffect::Completed` | Root behavior reported whole-group completion and passed result validation. Native delivery is separate. |
| Producer `FiniteEffect::Reported` | A validated incomplete member report remains in the original result. No whole-group completion is claimed. |
| Consumer `ConsumerEffect::NotAttempted` / local admission refusal | No native progress was attempted. |
| Consumer `ConsumerEffect::Unknown` | Native progress lacks validated completion evidence; remote refusal, partial application or full completion remain possible. |
| Consumer `ConsumerEffect::Completed` | Selected native mapping returned a validated whole-group completion. Local handoff still requires original authority and fresh policy. |
| Consumer `ConsumerEffect::Reported` | The original native output contains validated incomplete member evidence; consult that report for exact effects and atomicity. |

Completed/Reported are emitted only for the two root writes. Existing finite Read/Property Write/Action effect reporting
is unchanged. A completed unread native result keeps its completion evidence if
local cancellation, replacement, timeout or policy loss subsequently refuses
handoff. Original native outcome, first cause, result and delivery remain separate.
No owner automatically replays an effect-uncertain collection write or claims
per-member success from it. A Producer pre-effect refusal does not become known
to a Consumer through a generic native error response without a supported mapping.

These finite operations add no universal collection lifecycle. All-versus-selected
scope and read-versus-write data roles remain distinct.

**ObserveAllProperties / UnobserveAllProperties** reuse the notification
conversation under one `AffordanceTarget::Thing` and the distinct
`NotificationKind::ObserveAllProperties`. One logical identity, move-only cursor,
queue, loss policy, stop and settlement cover the collection. Individual Property
observations neither establish nor cancel it. Native subscription/session count
and sharing remain Binding-owned. Mapped cancellation authorizes the original
`UnobserveAllProperties`; local stop fences delivery immediately even if mapped
cancellation is denied, malformed, unsupported or uncertain.

Notification kind is explicit in preparation, raw requests and captured Producer
routes; a target's scope does not determine its operation family. Kind/scope,
prepared program and publication-data pairing are checked before effects. A
Thing target is not a standing alias for every root notification operation.

The implemented data subset is a complete named readable Property map per sample.
`ConsumerNotificationSchemas::ObserveAllProperties` retains the original member
schemas in shared owned backing; scalar `data()` is absent for this distinct
role. A captured data service must explicitly support/validate Property maps.
Each included value has its exact TD Property name and supported DataSchema;
writeOnly members are excluded, while readOnly members remain eligible. A root
Form supplies its own authority independently of the individual `observable`
hint or observe Forms. Unknown, missing, duplicate (including escaped aliases)
or invalid members refuse the entire sample before logical queue admission.
Application delivery retains the complete map, with no inferred changed-member
identity or atomic snapshot. Group authorization grants the whole collection;
partial member permission cannot be used as authority for this operation.

Producer registration requires a distinct `AllProperties` source and captured
root route/services. It publishes `PublicationData::PropertyValues`; leaf sources
and single-value/Event carriers cannot stand in for it. The application supplies
a complete map when any included Property changes; publishing a leaf value does
not fabricate a collection sample or trigger hidden Reads. Both roles retain
empty establishment/cancellation data, no response right, and future-only
delivery for the [declared MQTT mapping](../../protocol-bindings/protocols/mqtt/README.md#thing-root-property-observation).
Validation work and variable schema backing are funded before handoff; overflow
reports whole-sample loss or terminates, without coalescing or losing cleanup.
Replacement and late publication retain original authority, mapping and funding
through retirement and acknowledgement. No initial value, replay, atomicity,
wildcard attribution or fleet scalability is established.

Root Event collections retain their own notification conversation and mapped
controls, independently of the finite Action status observation described above.

**SubscribeAllEvents / UnsubscribeAllEvents** retain one root conversation under
`NotificationKind::SubscribeAllEvents`. An occurrence is one named Event with its
own optional data, not a complete Property-style map or one occurrence from each
Event. `NativeNotification::target` retains the occurrence's
`AffordanceTarget::Event`; `RawNotificationView::target` lends its
`AffordanceSite::Event` name, separately from the conversation's Thing
target and original recipient. Leaf candidates must equal their original target;
root Event candidates must name an Event. The composed validator also requires
membership in the original `EventCollectionSchema`, and validates data and each
submitted dataResponse against that specific member's original role. Unknown
names, wrong-family targets and wrong-member schemas refuse before queue admission
or response acceptance; the original candidate/reply owner remains through cleanup.

`ConsumerNotificationSchemas::SubscribeAllEvents` shares a bounded table of exact
Event names and separate subscription/data/dataResponse/cancellation schemas.
Absent data differs from a JSON null schema. The common schema envelope permits
32 members, 256 bytes per name and 4096 total name bytes; a concrete Binding or
the bounded TD semantic closure may refuse earlier. A scalar data/schema accessor
is not the aggregate schema or evidence that all members lack a role. This joined
facet supplies no universal aggregate subscription/cancellation encoding; it
refuses such member control data. Actual response opportunities can vary per
occurrence and keep their own owners; a member schema never creates one.

Producer `AllEvents` authority and `PublicationData::EventOccurrence { event,
data }` select one original Event under a separately captured root route. All
advertised leaf operations still require their actual sources/mappings. The
captured validator checks the selected name/schema before native acceptance;
wrong-family carriers, unknown names and invalid data return their original
backing. Publication has one native outcome; it implies no recipient receipt,
transactional group delivery or per-Event acknowledgement. Original occurrence
name/data backing, cleanup and unread settlements remain funded.

TD 1.1 specifies the root operations but does not prescribe a universal Event
collection payload envelope. The implemented [MQTT mapping](../../protocol-bindings/protocols/mqtt/README.md#thing-root-event-subscription)
uses a single terminal /+ filter, attributing native topic names inside the
Binding. Every known Event's data validates independently. Described member
controls/responses decline because this QoS 0 mapping has no application channel
for them. One logical subscription has one cursor/queue/loss/stop/settlement;
compatible native sharing never merges logical grants. The final actual interest
owns original unsubscribe/ACK and local socket disposal. Unknown native Event
names are not silently dropped. Native loss is visible and cannot coalesce Events.
No per-member subscription fallback or speculative universal notification runtime
is introduced.

An Event occurrence and an Action invocation remain distinct identities even
when their affordance names are equal. Their data roles are independent: Event
data uses the named Event schema; Action completion in a status observation uses
the named Action output schema. Sharing a Thing-root coordinate confers no
cross-family dispatch, cancellation, subscription or result ownership.

## Error and retry contract

`EXEC-OUTCOME-001`: Outcomes MUST distinguish invalid description, unsupported
combination, denied authority, exhausted capacity, failed communication, payload
validation, cancellation/timeout, stale identity, cleanup and internal defects.
External malformed input is not an internal invariant. Preserve bounded first
cause, selected identity and effect certainty. Pre-effect candidate fallback
and post-effect retry are different decisions. Unknown or possibly committed
write/Action/publication effects require an explicit new policy decision;
transient failure and TD safe/idempotent flags alone do not prove safe replay.

The finite Consumer settlement retains `ConsumerEffect` separately from its
first local cause, original native outcome and validated application result.
Native progress conservatively marks remote execution uncertain; cancellation
or a communication/validation failure does not establish remote rollback.
`result()` and `rejected_output()` lend Core `InteractionOutputView`/`PayloadView`
slices under that settlement's original backing and cleanup reservations.
`native_outcome()` returns a `ConsumerNativeOutcome`: normal/late output uses the
same slice view, while cancellation lends the exact retry/cleanup disposition.
Neither path exposes an owning output, Payload, or the original body's Arc.
The original affordance name is likewise borrowed as an `AffordanceSite`.
Sharing original data requires retaining the settlement owner; explicit byte
copies create separate caller-owned backing. Dropping a call or runtime cannot
release funding still held by a settlement. Final disposal destroys original
native/result/definition backing before releasing its reservations.
An admitted native success is not by itself a successful application outcome:
`native_outcome()` retains normal/late native results independently of
`result()`, which also requires terminal shape, captured validation, live original
authority and fresh policy. A successful Write result reports completion under
the selected mapping; it is not a Read or independent confirmation of physical
state. Producer `FiniteEffect::NotInvoked/Unknown/Completed`, application result and native
`FiniteDelivery` are separate axes. A handler error or invalid result after entry
cannot downgrade Unknown to NotInvoked. No finite owner replays a write.

Exact v5.1 payload/error/handler carrier schemas remain restricted implementation
records, not a universal envelope or an active eighteen-handler ABI. The public
ownership matrix distinguishes stable primitives, restricted implementations and
withdrawn speculative paths. Raw Action/notification slices above do not qualify
the corresponding complete TD capability families.
