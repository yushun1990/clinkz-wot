# Consumer Action composition evidence

Scope: continuing Host Consumer Actions with manual execution. Contracts remain
in [Planning](../spec/planning.md#continuing-consumer-action-preparation),
[Core](../spec/interaction-core.md),
[Runtime Safety](../spec/runtime-safety.md#continuing-action-associations-and-invocation-capacity)
and [Binding SPI](../spec/binding-spi.md). Alternatives are recorded in
[0124](../../workspace/0124-consumer-action-association-capacity.md).
This is implementation evidence for Owner-directed acceptance, not independent
review, architecture/package/milestone closure or production qualification.

The [composition fixture](../../protocol-bindings/protocols/http/tests/support/action_composition.rs)
wraps the actual HTTP preparation, mappings and services, with real additional
service backing and mutable passive declarations. It delegates native tokens,
control destinations, framing and I/O to the production Binding. The existing
[Consumer-to-Producer suite](../../protocol-bindings/protocols/http/tests/action_consumer.rs)
continues to exercise acceptance, pending/query/completion, distinct selected
control registrations/Forms, cancellation acknowledgement and uncertainty, both
role replacements, late results, resource refusal and original cleanup ownership.

For automatic selection, a live HTTP Producer returns the same completed value
as a pinned compatible Query in either Query Form order. Pinning the incompatible
listener refuses before Invoke, and that listener receives zero connections.
The external Consumer fixture registers HTTP alongside independent datagram Invoke
and control implementations. With either registration order, it sends the original
HTTP token to the HTTP Query and receives `42`. Removing that Query selects the
datagram pair, whose Query receives its own token and returns `24`. Two Invoke
providers qualify on that datagram Form. Only one returns the numeric continuation
accepted by its control.
[HTTP integration](../../protocol-bindings/protocols/http/tests/action_consumer.rs) · [Mixed-Binding fixture](../../tools/architecture-fixtures/open-consumer/tests/support/action_selection.rs)

Two datagram Forms isolate the unit of qualification. Form 0 offers Invoke and
Form 1 offers Invoke and Query. Pinning Invoke to Form 0 and selecting
automatically both choose Invoke 0 and Query 1 with 128 compiler units. With
127 units preparation refuses before I/O and releases its reservations. The
automatically selected Query sends the original token `7` and returns `24`.
Automatic selection also fits the minimum temporary capacity measured for the
pinned choice and refuses one byte below that boundary. In a second case, both
Forms offer Cancel. Form 1's Cancel compiler fails.
Automatic selection uses Query 1 with Cancel 0, and pinning Cancel 1 exposes
the original Binding error. All three regression checks failed on `005ac0b6`.
[Downstream qualification tests](../../tools/architecture-fixtures/open-consumer/tests/support/action_selection.rs)

With three single-operation Forms, two Invoke providers offer distinct native
tokens. Query 1 accepts only the numeric token. Query 2's compiler fails. On
`6baed041` pinning Query 1 succeeds and automatic preparation returns the later
error. In either registration order, the corrected automatic choice sends
`QUERY 7` and receives `24` with exactly 192 compiler units. With 191 it refuses
before I/O and refunds its reservations. Pinning Query 2 still returns its error.

A second datagram TD has an earlier Query with a later compatible Cancel and a
later Query with an earlier Cancel. The selected Invoke returns an ASCII token.
The selected Cancel and Query send it to the peer. A hard failure compiling
that required Cancel propagates even when pinning the later Query succeeds.
Competing Invoke providers and competing controls for one Invoke both produce
ambiguity at the preferred complete combination. Multiple Query providers for
an Invoke that cannot use any Cancel leave a unique complete assignment usable.
[Provider preference and ambiguity fixtures](../../tools/architecture-fixtures/open-consumer/tests/support/action_selection.rs)

An exhausted Invoke offering an ASCII token releases its cleanup slot before
the next Invoke offers a numeric token. With two cleanup slots and the pinned
choice's temporary allowance, automatic preparation retains the numeric Invoke
and the cached Query. Their UDP exchange returns `24`. Pinning Query to an
Invoke-only Form or to an unregistered Binding spends zero compiler credit.
With Invoke 0, Query 1 and Invoke 2, a definitive empty Query stops automatic
selection at 64 compiler units, even if Invoke 2 would fail. A second case
exhausts two Cancel Forms before reporting the same result. With three Invoke registrations,
two successful 32-unit compilers establish ambiguity at 64 units. At 63 units
the outcome is still undecided and preparation returns backpressure. The
fixture also checks early ambiguity when Query or Cancel completes the tuple.
[Selection resource and ambiguity regressions](../../tools/architecture-fixtures/open-consumer/tests/support/action_selection.rs)

With two Invoke Forms and eight declined Forms before each usable Query and
Cancel, automatic selection fits the temporary peak measured for the pinned
controls. Twenty native compiler calls cover the twenty Form/operation pairs.
The second Invoke reuses the first Invoke’s Query probes. Its numeric token
completes Cancel and Query over UDP and returns `24`.
[Empty control cache regression](../../tools/architecture-fixtures/open-consumer/tests/support/action_selection.rs)

A reload between two competing negative compiler observations refuses selection.
Stable competing providers report ambiguity, and short probe credit refuses before
native start. A pinned provider and a previously selected, still-authorized
association each complete a real datagram Invoke after a competitor becomes eligible.
Revoking another instance of that same definition starts no exchange. The fixture
also releases all original accounts. [Configuration and instance fixture](../../tools/architecture-fixtures/open-consumer/tests/support/action_selection.rs)

The F2 counterexample now imports one actual retained data owner and one access
owner into three operation registrations and two independently constructed
registries. Each prepares its own definition at a separate bank. Adding 60,000
bytes of actual backing to either service increases the service bank reservation
by exactly 60,000 bytes; neither definition's reservation changes. The three
operations and both definitions reference the same physical data owner. Physical
backing dies under its original bank charge, and all banks return to zero after
final release. These are resource-account measurements, not allocator heap,
throughput or preparation-time measurements.

The [preparation accounting regression](../../planning/tests/consumer_action_ownership.rs)
uses an external test Binding with matching retained data services and 16-byte
versus 4,143-byte media strings. Each definition owns a distinct media snapshot;
its definition-bank reservation grows by exactly the 4,127-byte difference, while
the original service-bank charge stays fixed during preparation. A definition
bank one byte short of the complete copy reservation refuses before native
compilation and releases partial preparation funding. With each admission
temporary or peak limit set to 64 KiB in turn, the compiler allocates scratch
at its exact remaining allowance. One extra byte refuses before compiler entry
and identifies the limiting resource. Final owner disposal returns both banks
to zero. A 1 KiB independent charge in a 64 KiB shared peak bank survives
preparation. Raising the local temporary ceiling from 48 KiB to 1 MiB gives
the same measured Engine peak and still admits the mapping.

In the downstream UDP fixture each native mapping allocates 36 KiB. One mapping
fits a 64 KiB admission peak. A pair is refused before the second allocation
under either peak ceiling, with the original bank fully released. At 192 KiB
the pair retains 72 KiB of native buffers after preparation and returns `24`
using its original continuation. A separate case discards an incompatible
36 KiB Invoke, reuses the funded Query and completes with the later Invoke.
[UDP selection and accounting tests](../../tools/architecture-fixtures/open-consumer/tests/support/action_selection.rs)

A closing revision observer samples three mappings with 36 KiB of native
buffers and a structured output schema before publication. The admission peak covers their full Engine
charge at that instant. A stable revision releases the peak at handoff. With
a changed or unavailable revision, the native destructor observes the same
charge before the bank returns to zero.
[Revision and disposal regression](../../tools/architecture-fixtures/open-consumer/tests/support/action_selection.rs)

The F3 cases prepare once, then change data backing, temporary bytes, validation
work, media or schema support, or native exchange bytes, poll work, registration
identity or operation. Association construction refuses and returns the exact
original definition/access owners. Already bound associations also refuse new
work, preserving the exact caller input and vacant capacity without entering
native start. Changed access backing cannot be adopted under its old funding.
A compiler callback that changes the original data work declaration refuses
preparation and releases partial definition funding. An access callback that
changes the compiler allowance refuses before that compiler is entered. Capacity construction takes
no definition and can neither consume nor refresh one.

A real HTTP fixture prepares three Invoke/Query/Cancel mappings once, destroys
TD/registry owners and binds the same actual definition to three independent
Consumers. Association construction performs no further compilation and allocates
no invocation quota or vacant slots. Vacant slots hold registry/control references; admitted records are separately
funded rather than counted again inside slot storage. Two independently funded one-slot capacities
admit different Consumers. Saturation at one refuses exact input while the other
accepts, and closing the first leaves the second's original identity, remote
cancellation and validated completion usable. Dropping a passive association
view does not cancel its admitted invocation. A third association subsequently
uses the second capacity with no new preparation. Independent access policies
remain bound to their original associations; native control destinations and
selected registrations remain original.

Service, definition, three instance and two execution banks remain separate.
Outstanding instance quotas survive settlement handoff until each original
result is disposed. A one-outstanding-work instance refuses admission through
another empty capacity while either its invocation or handed-off result remains
retained; disposal permits that same association to use the other capacity.
After all public definition/association/capacity/handle views
are dropped, settlements retain their original services, definition and account
owners. Final result disposal releases every account; physical service destructors
observe original funding. Result access lends existing Core slice views, with
compile-fail examples rejecting detached normal/native/input body Arcs and escaped
loans. A weak reference to the actual HTTP output stays live while a shared
settlement survives and expires on its last disposal, with original accounts
retained until that point. HTTP still allows only one unretired finite exchange
per actual registration. Independent Engine capacity does not increase that
native allowance or imply one connection per Consumer.

Additional real HTTP cases change an access revision/work declaration, prepared
data work or native bounds after unread completion, and change data work inside
actual Query output validation. Each observed violation of the relevant original native/authority or retained Query data dependencies
irreversibly fences its own invocation. Restoring declarations cannot enable query or successful handoff;
original identity and native output remain in the failed settlement. Original
returning-operation validation still runs for late output when cancellation/access
loss has already fenced application acceptance, under that service's original allowance.

The operation-scope HTTP counterexample uses three independent operation data/
access services and native mappings. Changing only unused Cancel access revision,
work, retained-backing declaration or native byte/work bounds leaves pending Invoke and its
independently authorized Query result usable. Per-instance Cancel denial likewise
refuses before effects. Cases cover both never-used Cancel and an acknowledged,
fully retired cancellation: a failed later request preserves original identity,
continuation and known cancellation status. No refused request starts native work.
Metadata changes still refuse admission through the original association; explicit
new access binding may qualify a new revision/work, while changed prepared native/
data declarations or unfunded access backing still refuse that binding.

This counterexample fails against `306d3d9` with Closed instead of Pending after
only the unused Cancel revision changes. No funding is released because a control
becomes unavailable; the original complete definition/services remain retained.
A raw immediate Invoke completion similarly survives an independent unused Query
revision change before completion or handoff, without entering Query/Cancel policy.
The access-callback regression uses independent control codecs: losing the
retired Invoke codec during control access leaves the selected Query/Cancel
usable, while full qualification still refuses new invocation admission. Losing
Query's actual codec before Query admission refuses without native effects or
revoking pending Invoke; an independently authorized Cancel remains usable.
Negative cases change the actually admitted Query/Cancel revision with native work
owned: the invocation stays irreversibly fenced through short-credit/pending
retirement, preserves a late validated Query output and uncertain Cancel status,
and releases only its original owners. The unread-result HTTP cases use independent
Query services to ensure narrowing cannot bypass its selected grant/bounds.

The [raw callback suite](../../servient/finite/tests/consumer_action_callbacks.rs)
retains deterministic occupied-slot/reentry, time, wake, unwind, cancellation,
late-result and paid disposal cases. The independent
[downstream datagram fixture](../../tools/architecture-fixtures/open-consumer/tests/action.rs)
continues to use different actual invoke/control Binding types and blocked/error
retirement with funded late validation.

The HTTP body-allowance counterexample fails against `485b5e5`: Invoke accepts
input `12` under a two-byte body policy, then a Query with an eight-byte policy
returns valid integer `2400`. Invoke-centered validation closes it. The corrected
path completes with exactly one Invoke input validation and one Query output
validation. Conversely, Invoke's 32-byte policy cannot grant Query permission to
validate two-byte `24` under Query's one-byte allowance: the native output remains
retained, validation fails before codec entry, and settlement cannot succeed.
Independent service, definition, association and execution banks all reach zero
only after original settlement disposal.

The downstream representation counterexamples use integer JSON Invoke input
and a shared five-character string output schema. Invoke uses a JSON service
with a two- or 64-byte policy; Query uses an independent plain-text service with
a 64-byte policy, 1024-byte scratch and seven work units, versus Invoke's 64-byte
scratch and 47 units. Both services can support both schemas in the media-only
case. A separate case lets Query implement only its actual output role; Cancel's
service implements neither application schema. Real datagrams preserve `ready`
as `text/plain` through settlement, without JSON quotes or conversion. Query
completes with Invoke available in the media-only case and after its retired
codec becomes unavailable in other cases, with its exact seven work units;
six units enter no native poll. Control start and Cancel progress use
zero application codec credit. Query's own too-small body limit and a valid text
value violating the shared string constraint both refuse application success.
The Query validator observes the original scratch reservation during callback.
Admission reserves exactly its 1024-byte maximum used-role scratch, omitting
the Cancel carrier's unused 2048-byte application codec declaration.

Late text output is returned by the original exchange's `poll_close` after a
native poll error, blocked/failed retirement, local cancellation, Consumer
revocation and root loss. It is validated once by Query under its captured
allowance; Invoke is not reentered. The settlement retains original identity,
poll/cleanup errors, raw text/media and failed application outcome. Changing
Query's declared work while cleanup is pending prevents validation entry under
the stale allowance, retains a distinct validation error and the exact native
output, and never refreshes funding or the qualification baseline. Native
destructors observe original cleanup funding; all accounts clear after final
settlement disposal. Existing cancellation/reentry/uncertain-control cases remain
in the HTTP and raw suites.

The
[root QueryAllActions integration](../../protocol-bindings/protocols/http/tests/query_all_actions.rs)
uses two continuing associations and independent continuing/finite capacity under
one original Consumer authority against the actual HTTP Producer.

Validation commands and exact tested revisions belong in the PR. Focused checks
cover Core, Planning, Servient finite, HTTP and the independent downstream fixture,
with design projections and applicable no-default compilation. Required GitHub
`validation` establishes diff hygiene only.

Limits: reuse binds the same already qualified endpoint/target/registration/
configuration-specialized definition. It does not compare new TDs, reuse partial
Action lowering across endpoints, share invocation authority or provide general
security support beyond the existing NoSec mapping. Explicit access binding may
qualify a new policy revision for new work; it cannot retarget admitted work.
Managed mixed HTTP/MQTT execution was outside this increment; the subsequent
[Host turn composition](consumer-execution-composition.md) supplies its scoped
Consumer integration. TD-driven Action preparation here runs on Host. The
[caller-owned continuing Action contract](../spec/runtime-safety.md#caller-owned-continuing-action)
covers its separate execution path. Reusable qualified Producer exposures and
production qualification remain open.
Review 10's retained probe is historical against its
recorded API; the corrected live counterexamples are these executable fixtures.
