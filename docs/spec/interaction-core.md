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

Exact v5.1 payload/error/handler carrier schemas remain restricted implementation
records, not a universal envelope or an active eighteen-handler ABI. The public
ownership matrix distinguishes stable primitives, restricted implementations and
withdrawn speculative paths. No replacement Action/Event runtime is claimed.
