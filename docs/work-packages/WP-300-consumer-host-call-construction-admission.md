# Consumer Host call construction admission candidate

`index.toml` owns `WP-300-CONSUMER-HOST-CALL-CONSTRUCTION`, currently
**planned/candidate/current** under v5.1 ADR-0013. Independent admission review
and a separate admission transition are required before production changes.
The patches in the existing runtime-entry fixture are disposable experiments,
not production implementation or completion evidence.

## Decision

Keep `ClientBinding::invoke`, `HostBindingCall`, `HostBindingCallBox` and both
installed registration start signatures. Before invoking a raw Consumer client,
fallibly allocate **one private, complete Core call envelope** able to hold either
existing seal mediator. Populate it after acceptance and erase/pin the already
allocated box without another allocation. Add only the optional
`HostBindingCallBox::try_new<B>(B) -> Result<Self, B>` constructor, reusing Core's
existing exact-Layout fallible allocation primitive. Keep `new` available; its
infallible semantics do not acquire a bounded-runtime guarantee.

This is a representation and failure-ownership correction inside Core's current
sealing boundary. It adds no result validator, seal issuer, public call-support
certificate, lifecycle, protocol, resource kind, WorkClass or scheduler. No new
ADR is needed: ADR-0009's distinct representations and the existing sealing
authority remain intact. The [investigation](../../workspace/0080-consumer-host-call-construction.md)
owns the alternatives and their production-source rationale; the
[fixture](../../tools/architecture-fixtures/consumer-runtime-entry/README.md#host-call-construction-candidate)
owns reproduction commands and measured experimental evidence.

## Production path and invariants

Both `HostBindingRegistration` and `HostConsumerBindingRegistration` already
share `start_host_consumer_property_read`. Preserve its identity/capability
barrier before invoking the private raw client. Build the existing fixed error
context and request-derived `ConsumerResultSeal` without allocation. Envelope
allocation failure returns `BindingInputRejection<OutboundRequest>` with the
**same complete request** and structured Binding-phase Backpressure. The raw
client is not called; no accepted ownership or protocol side effect exists.

Raw rejection still returns the exact request, destroys the vacant envelope,
and issues no seal. Successful raw acceptance moves one owned pinned call into
the envelope. Preserve raw `lifetime_footprint()` inspection and checked combined
accounting, then select the normal sealed or accepted-overrun branch. Neither
branch may allocate, fail to retain the raw call, synthesize request rejection,
or poll the raw operation during construction. Never move pinned concrete raw
state; moving its owning `Pin<Box<_>>` is allowed.

Keep the existing `SealedHostConsumerCall` algorithm: consume its single-use
private seal only for successful normal/late output, run the existing Core
validator, preserve structured raw failures and normal-versus-Returned
classification, and delegate deadlines and cancellation. The existing
`RejectedAcceptedHostConsumerCall` retains raw state and unused seal and reports
the accepted resource error without raw polling. Preserve its single-use
terminal behavior and saturated footprint on arithmetic overflow. An accepted
resource error does not authorize abandoning raw ownership or unbounded Drop.

Cleanup transfer moves or returns the **whole** decorated call, unused seal,
first cause and accounting. Caller disinterest is not cancellation. Core does
not gain a cleanup driver or permission to dispose of a pending call. The
runtime must keep the accepted owner until settlement/transfer and actual final
release. Application-static slots, terminal acknowledgement, work debits and
cleanup semantics remain separate and unchanged.

`try_new` attempts exactly one nonzero concrete Layout allocation (zero for a
ZST), initializes/pins it only after success, and returns the exact concrete
owner on null. It does not call protocol code or issue an acceptance guarantee.
A binding must retain the request inside that recoverable owner or outside
construction, recover it before rejecting, and bound any other allocations and
failed-state destruction. No helper can recover an allocation that already
aborted in `B`'s construction. Generic boxed authoring remains supported but is
not automatically eligible for a bounded Consumer runtime.

## Callback eligibility and cleanup boundary

Core registration currently checks identity, representation and declarations;
`ConsumerCompilerSupport` attests compiler behavior, not client calls. Neither
fact proves a raw client's synchronous constructor, `lifetime_footprint`,
deadline, result/cancellation callbacks or destructor bounded. `invoke` and
footprint/deadline inspection have no WorkBudget parameter; budgeted polls also
depend on cooperative concrete implementation. A new generic trait/budget
parameter would not preempt a blocking callback or certify hidden allocations.

Use concrete implementation evidence, bound to actual complete registration,
configuration, representation and admitted input limits. Its attestation must
cover construction allocations, state/transient Layouts, fixed callback entry
cost, budget/progress/wake behavior, normal/late/cancel/transfer settlements,
failed-state destruction and terminal release. WP-400 owns the private policy
that accepts that evidence and rejects unsupported bindings before runtime work.
This precursor supplies construction mechanics and a finite test witness; it
does **not** invent a production binding eligibility registry or attest arbitrary
clients. Existing compiler support remains compilation-only authority.

The witness prepares a single fixed response before observation, accepts one
request, attempts one recoverable concrete raw box, uses nonblocking one-slot
access, reports a fixed footprint, and debits one BindingPolls unit per raw
result/cancel step. It performs no protocol I/O. Its finite response/request
shape and destructors can be inspected. Larger URI maps, protocol libraries,
blocking callbacks, transport queues and variable destruction need their own
input/work/resource proof; this witness cannot certify them.

## Dependencies, implementation and impact

Direct completed/current predecessors are
`WP-300-CONSUMER-PROPERTY-READ-BINDING`,
`WP-300-CONSUMER-ONLY-REGISTRATION` and
`WP-200-CONSUMER-COMPILER-SUPPORT`. Their passed records supply the existing
validator, complete owners and reusable fallible compiler allocation primitive.
The Planning ownership bridge is complete but disjoint; no change to it is needed.

Production scope is only `core/src/binding.rs` and the private allocator-helper
visibility in `core/src/binding_compiler.rs`. Tests may extend the existing Core
Consumer binding/support tests and runtime-entry fixture. No crate/dependency/
feature manifest, Core export, Foundation, TD, Planning, Servient, protocol,
Producer implementation or legacy path change is permitted. Implementation
discovery requiring any additional API/ownership/resource change stops for an
amended independent review.

One envelope allocation replaces the original branch-specific Core allocation;
its maximum enum Layout, including tag, raw owner, seal, fixed error and vacant
state, is charged once as Core call backing. The normal branch may therefore
cost more bytes than its old smaller wrapper. Include vacant-envelope/raw
construction overlap and failed-construction destruction in the transient
envelope; no exact physical capacity may be inferred from a logical ceiling.
Measure boxed carrier and return/inline transport Layouts separately. Keep the
Host carrier and installed result shape unchanged; no new static slot overhead.

Requirements: `BIND-REG-001`, `BIND-MEM-001`, `BIND-IO-001`, `BIND-OUT-001`,
`BIND-DELIVERY-001`, `BIND-CALL-CANCEL-001`, `BIND-HOST-CANCEL-001`,
`API-PAYLOAD-001`, `PLAN-ARTIFACT-001`. Applicable existing performance categories
are `PERF-GW-009`, `PERF-GW-010` and `PERF-GW-028`; these focused discriminators
measure construction and invariant behavior, not full workload completion.
No resource profile or state-machine edit is proposed.

Old Binding completion evidence remains valid at its historical implementation
boundary; it never claimed recoverable call construction. After admitted
production changes, replace/reaffirm affected sealing/accounting/cancellation
evidence at the exact new head. Do not carry old normal-wrapper byte measurements
forward silently. Topic 0076's broader runtime/product issues remain open but are
disjoint from this mechanics-only precursor; its call subquestion is migrated.

## Checks and exact remaining obligations

The index records focused pre-implementation commands. The new CI policy's
required `validation` context checks diff hygiene only; no full manual workflow
is required for these scoped candidate experiments. Run authority validation,
baseline/candidate discriminators and the inline comparator; retain their exact
source boundary and toolchain. No full workspace success is asserted.

Before admission: independent review must reconstruct this candidate's exact
diff, challenge the alternative selection and raw callback limitations, reproduce
both accepted branches and allocation failures, check authoritative projections
and required CI, then record an explicit ADR-0013 admission revision. This PR
does not supply that independent review or transition.

After admission, completion evidence `consumer-host-call-construction` must prove:

1. Actual public `try_new` handles nonzero, zero-sized, over-aligned and pinned
   concrete states; null returns the exact owner and original request with zero
   protocol side effects and exactly one final release. All other raw construction
   allocations are absent, recoverable or explicitly pre-backed in the supported
   concrete witness.
2. Core backing failure occurs before raw invocation and returns the exact
   request. Raw rejection/failure releases the vacant envelope exactly once;
   accepted normal, ceiling-minus-one/exact/plus-one and arithmetic-overflow
   branches retain the same raw owner with zero post-acceptance allocation.
3. Normal and both late-return paths, raw errors, Pending/zero-credit, deadlines,
   cancellation and accepted/rejected cleanup transfer use unchanged Core sealing
   and settlement semantics in both complete Host registration forms. Rejected
   accepted ownership remains intact until explicit terminal release.
4. Actual native allocation/owner/return/transient Layouts and overlap are
   recorded, charged once, and independently reviewed against declaration and
   parent-capacity obligations. No allocator-metadata, whole-process peak or
   device-fit claim follows from Layout observation alone.
5. Concrete constructor/footprint/deadline/poll/cancel/destructor costs and finite
   input limits are attested. Generic `new`, registration validity and compiler
   support never imply runtime call eligibility. Static/no-default/async-no-std
   behavior and Producer source/behavior compatibility remain valid.
6. Exact-head production review, focused validation and required CI pass; a
   separate completion transition may then update this tranche's evidence/state.

Still outside this precursor: WP-400 policy and actual protocol eligibility,
paired parent/global reservations, publication/cancellation linearization,
generation/plan/call leases, delivery abandonment, drain, executor/manual fallback,
root shutdown, variable cleanup scheduling, real Zenoh, Consumer architecture
gate acceptance and all gate/milestone transitions.
