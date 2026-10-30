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
`ConsumerSchemas` keep finite input and output distinct; absence of a data role
is different from a present unconstrained value. The current finite path supports
Property Read and explicitly synchronous Action invocation, including empty Action
input/output roles. Shape/identity seals require terminal primary application
results and refuse continuing status/invocation references; captured codec/schema
checks separately precede input effects and validated result handoff.

The implemented `value_type::TypeSchema` is a fixed-size type-only subset, not
the general schema representation. Planning must refuse every required assertion
it cannot capture. Concrete codecs classify values; Core evaluates the prepared
type relation. `integer` includes mathematically integral decimal/exponent
encodings and is a subset of `number`; binary floating-point rounding cannot
establish integrality. The restricted Host Producer executor must check output
with its captured program/services before successful delivery. Finite raw mode
remains explicitly unvalidated. The shared finite owner can require the actual
captured provider's output-validation facet: it checks complete result shape/
backing before entering that service and retains its result through native
delivery/retirement. This proves only that participant's output check, not
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
Event control inputs and reciprocal responses have distinct carriers. Required
responses refuse absent native support; each actual response opportunity has a
separate retained delivery/retirement owner. An exposed unused optional
opportunity can be explicitly declined without sending a dataResponse or stopping
the subscription. Pending/error retains its
reply owner, payload and capacity; only successful native retirement permits
cursor advance. Required and already submitted responses cannot be declined.
No initial Property value, replay, wire backpressure, TD qualification or schema
validation is inferred. Collection operations and constrained execution remain
unimplemented by this Host slice.
Fixed loss status and one prepaid overflow/late handoff preserve cleanup when
the item/byte queue is full. Explicit owner cleanup works after every cursor is
lost; terminal acknowledgement releases the logical reservation. Native physical
retirement and remote cancellation/response outcomes remain separate.

Producer emission has its own Host facet in `core::event` and
`servient-finite::event`; it is neither a finite request nor a Consumer
subscription conversation. An application registers an Event source on the
same logical Producer instance as other behavior. Complete publication captures
each selected native emission mapping and its actual data/security services.
`emit_event` checks current instance authority, exact captured mapping, backing
allowance, fresh policy and data validation before native acceptance. Explicitly
raw registries cannot bypass qualified Event service capture. No Engine local
record is created for a remote broker subscriber, and no Engine subscriber loop
replicates an emission.

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
TD-driven Consumer subscription/dataResponse path.

`EXEC-COLLECTION-001`: Described root collection operations MUST preserve native
semantics and logical attribution, including partial failure and supported
atomicity. They MUST NOT silently become N individual calls/subscriptions.
Explicit application fan-out is a separate bounded capability with its own
promised guarantees. Native group distribution and matching stay Binding-owned.

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

Exact v5.1 payload/error/handler carrier schemas remain restricted implementation
records, not a universal envelope or an active eighteen-handler ABI. The public
ownership matrix distinguishes stable primitives, restricted implementations and
withdrawn speculative paths. Raw Action/notification slices above do not qualify
the corresponding complete TD capability families.
