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

The F2 counterexample now imports one actual retained data owner and one access
owner into three operation registrations and two independently constructed
registries. Each prepares its own definition at a separate bank. Adding 60,000
bytes of actual backing to either service increases the service bank reservation
by exactly 60,000 bytes; neither definition's reservation changes. The three
operations and both definitions reference the same physical data owner. Physical
backing dies under its original bank charge, and all banks return to zero after
final release. These are resource-account measurements, not allocator heap,
throughput or preparation-time measurements.

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
actual output validation. Each observed violation of the original Invoke or retained Query dependencies
irreversibly fences its own invocation. Restoring declarations cannot enable query or successful handoff;
original identity and native output remain in the failed settlement. Original
invoke validation still runs for late output when cancellation/access loss has
already fenced application acceptance, under its original data allowance.

The operation-scope HTTP counterexample uses three independent operation data/
access services and native mappings. Changing only unused Cancel access revision,
work, retained-backing declaration, native bounds or schema support leaves pending Invoke and its
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
The access-callback regression uses independent control codecs: loss of the
original Invoke validator must be observed and fenced before a control starts,
even though that control's own data service remains available. Negative cases change the actually admitted Query/Cancel revision with native work
owned: the invocation stays irreversibly fenced through short-credit/pending
retirement, preserves a late validated Query output and uncertain Cancel status,
and releases only its original owners. The unread-result HTTP cases use independent
Query services to ensure narrowing cannot bypass its selected grant/bounds.

The [raw callback suite](../../servient/finite/tests/consumer_action_callbacks.rs)
retains 24 deterministic occupied-slot/reentry, time, wake, unwind, cancellation,
late-result and paid disposal cases. The independent
[downstream datagram fixture](../../tools/architecture-fixtures/open-consumer/tests/action.rs)
continues to use different actual invoke/control Binding types and blocked/error
retirement with funded late validation. The
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
Managed mixed HTTP/MQTT capabilities, reusable qualified Producer exposures,
constrained continuing Actions, scheduler/event-loop integration and production
qualification remain open. Review 10's retained probe is historical against its
recorded API; the corrected live counterexamples are these executable fixtures.
