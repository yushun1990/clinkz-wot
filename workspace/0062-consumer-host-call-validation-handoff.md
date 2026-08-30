# 0062 Consumer Host Call Validation Handoff

Status: DISCUSSING

Kind: implementation-discovered architecture correction candidate

Priority: HIGH

Target: the first unconstructible WP-300 -> WP-400 Consumer handoff: retaining
Core-owned response-validation authority after Host binding-call acceptance

## Scope and authority

This topic originally investigated the aggregate Planning input needed by the
public Consumer `consume(td)` transaction. Reconstructing the active v5.1
authority and the completed WP-100, WP-200, and WP-300 implementations exposed
an earlier defect that is independent of aggregate TD enumeration: the Host
client path transfers the only `OutboundRequest` to a binding call, but the
only public Core response validator requires borrowing that same request after
the call returns.

This revision records the smaller predecessor finding and a correction
candidate. The earlier aggregate-admission investigation remains useful later
history in Git, but its conclusion that bounded validation, aggregate
construction, and registration-snapshot pinning must all be solved before any
WP-400 Consumer composition work is too broad.

This document is non-authoritative while `DISCUSSING`. It does not change
active v5.1 authority, reopen or reaffirm a completed tranche, admit production
source, register the Consumer architecture gate, change a gate status, or
unblock WP-400 or WP-600.

## Reconstructed repository facts

The controlling facts come from active authority and current source, not from
the prior 0062 reviews or the closed PRs #56 and #57.

1. ADR-0019 and the Core/Binding specifications require every binding-origin
   Property Read success to remain untrusted until Core checks its binding id,
   binding generation, plan id, response branch, status, payload role, and
   action-reference shape against the live selected call.
2. The completed WP-300 contract exposes exactly one public wrapper for that
   check:

   ```rust
   pub fn validate_untrusted_binding_output(
       request: &OutboundRequest,
       output: InteractionOutput,
   ) -> CoreResult<InteractionOutput>;
   ```

3. The target Host `ClientBinding::invoke` consumes the request and returns a
   call whose terminal value contains only `CoreResult<InteractionOutput>`:

   ```rust
   fn invoke(
       &self,
       request: OutboundRequest,
       artifact: &BindingArtifactEnvelope<HostBindingArtifact>,
   ) -> Result<
       HostBindingCallBox<CoreResult<InteractionOutput>>,
       BindingInputRejection<OutboundRequest>,
   >;
   ```

4. After acceptance, neither `HostBindingCallBox` nor its terminal or
   cancellation-settlement result exposes the accepted request or a
   Core-issued validation authority derived from it.
5. The application-static path does not have this exact loss: an occupied
   `ClientRequestSlot` retains the request while the binding's request state is
   pending. A static owner can borrow `slot.request()` before acknowledgement
   and clear.
6. Current WP-300 tests prove Host invocation/cancellation and response
   validation in separate tests. They do not transfer one real request into a
   Host call and then validate that call's returned output against the same
   call authority.
7. WP-400 explicitly owns the missing composition: invoke the selected binding,
   validate the binding-origin response, retain caller-drop/cancellation/late
   completion, and release call and plan ownership exactly once.

The minimal type-level reproduction is therefore:

```rust,ignore
let request = OutboundRequest::property_read(/* selected values */)?;
let mut call = registration
    .client()
    .expect("Consumer capability has a client")
    .invoke(request, artifact)?;
let output = poll_to_terminal(&mut call)?;

// `request` was moved into `invoke`; this conforming validation cannot compile.
let output = validate_untrusted_binding_output(&request, output)?;
```

The source can reconstruct a second equal-looking `OutboundRequest` from its
public accessors before the move, but that is not the live accepted request. It
duplicates owned call state, introduces an extra resource owner, and makes
equality depend on caller discipline. The active contract deliberately made
the request non-`Clone` and prohibited independently supplied validation ids to
avoid that class of workaround.

## Why this is the next distinct correction

The defect appears with one already-built logical plan, one eager artifact,
one selected registration, and one Host call. It needs no aggregate target
index, public `consume(td)` implementation, TD validation provenance, multi-plan
identity allocation, or general registration-snapshot collection.

Consequently:

- the bounded-validation and aggregate resource questions found by the earlier
  0062 investigation remain real for the later general public `consume(td)`
  transaction;
- the full aggregate candidates in closed PRs #56 and #57 do not repair this
  accepted-call validation loss;
- solving those broader questions first would allow substantial Planning and
  admission design to accumulate on top of a Host call boundary that still
  cannot satisfy `API-PAYLOAD-001`, `BIND-IO-001`, and `BIND-DELIVERY-001`; and
- the separately required WP-400 multi-owner/scheduler checkpoint remains
  useful parallel work, but it does not make this exact Consumer call
  ownership valid.

The smallest sound progression unit is therefore a correction of the Host
accepted-call carrier and its WP-300 completion evidence before WP-400 Consumer
admission or aggregate design resumes.

## Decision candidate

Core should retain one single-use response-validation authority at the exact
Host call-acceptance boundary. That authority is derived inside Core from the
same selected `OutboundRequest` immediately before the request is transferred
to the binding. It contains only the immutable facts needed by the active
Property Read validator; it is not a second request, candidate, plan lease, or
public bag of independently supplied ids.

The preferred correction is a Core-owned accepted Host client-call carrier,
constructed through the complete `HostBindingRegistration` rather than by
letting WP-400 call its raw client component directly. Its semantic contents
are:

```text
accepted Host client call
  = binding-authored HostBindingCallBox<CoreResult<InteractionOutput>>
  + Core-derived single-use response-validation authority
```

The complete-registration start operation must:

1. verify that the request, selected artifact envelope, and registration agree
   on every identity each pair shares: binding, binding generation,
   configuration, compatibility, plan-set generation, plan, and Consumer-call
   role, before protocol work;
2. derive the private validation authority from that exact request;
3. invoke the selected client and return the unchanged request on
   pre-acceptance rejection;
4. on acceptance, return one owned carrier before the first protocol side
   effect; and
5. retain no artifact borrow, TD, Form, `InteractionOptions`, candidate list,
   registration snapshot, or plan-set owner inside the Core carrier.

That start operation becomes the complete registration's Consumer execution
projection. The current public `client()` projection must not remain WP-400's
accepted-call path, because retaining it as an equally valid execution route
would make validation bypassable. Whether the raw projection is removed,
restricted, or retained only for a compatibility interval is part of the
required public-API impact review; the public authoring trait itself remains
available for third-party implementations.

The accepted carrier must mediate normal result polling and cancellation. It
passes binding failures through unchanged, but every successful
`InteractionOutput`—including a late `BindingCallSettlement::Returned`
success—must pass through the existing private Core validation kernel before
the carrier can expose it to WP-400. The validation authority is consumed or
terminally discarded exactly once and follows the complete call through any
cleanup transfer or manual fallback.

WP-400 remains responsible for the plan-set lease, caller-interest
state, deadline/cancellation choice, cleanup reservation, result delivery, and
drain/reclamation ordering. The Core carrier does not absorb those Servient
responsibilities.

For application-static execution, the existing occupied
`ClientRequestSlot` remains the accepted-request owner. The correction must add
or prove one Core-mediated terminal path that validates against
`slot.request()` before acknowledgement/clear and prevents an unvalidated
successful output from reaching the application. Host and static physical
representations need not share a wrapper type, but their terminal semantic
trace must be identical.

The existing public `validate_untrusted_binding_output(&OutboundRequest, ...)`
may remain as the static/external composition primitive. The Host target path
must not require reconstructing a request merely to call it. Exact Rust type
names and private field layout remain implementation choices; the frozen
correction is the retained Core validation authority and the no-bypass terminal
boundary.

## Alternatives rejected for this correction

### Finish aggregate Consumer admission first

Rejected as the immediate progression unit because the Host handoff fails even
when a test supplies one valid, already-completed WP-200 output. Aggregate TD
work cannot make the consumed request available after `invoke`.

### Clone or reconstruct the request in WP-400

Rejected because it creates two owned request-shaped values, adds unaccounted
retention, and makes the validator depend on manually keeping copies equal.
Making `OutboundRequest: Clone` would weaken rather than repair the ownership
contract.

### Let each binding validate its own output

Rejected because the binding is the source of the untrusted metadata. Shared
WoT response semantics belong to Core, and a third-party implementation must
not be able to bypass them by returning a nominal success directly.

### Expose three expected ids or a freely assembled validation key

Rejected because it recreates the mismatch-prone public authority that the
v5.1 wrapper intentionally removed. A private Core-derived authority may use
the selected artifact identity internally; callers do not assemble its fields.

### Return the whole request with every Host terminal result

This is constructible but not the smallest retained boundary. It forces the
binding call to keep URI-variable and deadline input after deriving all
protocol-local state, expands lifetime-footprint accounting, and still needs a
trusted comparison against the selected call owner if a binding returns the
wrong request. Retaining only the Core validation authority preserves the
active validator semantics with less state.

### Change Host invocation to borrow the request

Rejected for this correction because it changes the accepted owned-transfer
contract and forces every Host binding to copy all call-varying input into its
private state. The existing owned input/rejection boundary remains otherwise
constructible.

## Work-package and authority impact

- `WP-100-CONSUMER-CALL-VALUES-VALIDATOR`: the private validation algorithm and
  its negative cases remain valid; its completion claim is reaffirmable after
  the corrected carrier consumes the same kernel.
- `WP-200-CONSUMER-PROPERTY-READ-PLANNING`: this finding does not change its
  one-plan construction or selection behavior. Aggregate-output questions are
  separate later impact evidence.
- `WP-300-CONSUMER-PROPERTY-READ-BINDING`: the completed implementation and
  evidence are affected. The claim that Host invocation plus live-request
  validation is usable by WP-400 has not been proved and is false at the
  current API boundary. ADR-0013 requires an explicit impact review and a
  separately admitted correction before production source changes.
- `WP-400` Consumer Property Read: source admission remains blocked because its
  required response-validation step cannot consume the completed Host
  predecessor. This finding does not block disjoint Producer scheduler
  evidence.
- Consumer architecture-gate registration, accepted Producer gate status,
  production Zenoh, active requirement counts, and milestone status do not
  change in this candidate.

No v5.2 requirement-set revision is indicated. The correction preserves the
active v5.1 semantics and repairs an unconstructible public carrier. Migration
would update the Binding SPI authority, API ownership, WP-300 admission and
completion projections, and the work-package index through a reviewed
ADR-0013 correction tranche.

## Falsifiable correction boundary

Before this topic can migrate, executable evidence for the corrected source
must demonstrate all of the following in the same composed call:

- a real WP-200 selected artifact and request enter the matching complete Host
  registration;
- pre-acceptance rejection returns that exact request and creates no retained
  validation or protocol owner;
- acceptance produces one Core-owned carrier before protocol side effects and
  retains no artifact borrow;
- a valid terminal success is checked by Core and delivered once;
- mismatched binding id, binding generation, plan id, response selection,
  status, payload shape/role, or action reference is rejected after the
  original request has been transferred;
- caller-interest drop, timeout, cancellation, and late returned success keep
  the validation authority with the call until one terminal disposition;
- cleanup transfer acceptance/rejection moves or returns the complete call and
  validation authority without duplication or loss;
- request/validation/call retained bytes have one explicit accounting owner;
- the application-static slot proves the same validation and terminal outcomes
  before acknowledgement and reuse; and
- no test reconstructs/clones an `OutboundRequest`, supplies expected ids
  independently, calls a legacy binding path, or claims aggregate
  `consume(td)`, the Consumer architecture gate, or production protocol
  progress.

## Review and migration condition

A fresh independent architecture review must reconstruct this finding from the
active v5.1 authority, the exact WP-300 source/API, its completion evidence, and
the proposed diff. It should try to identify an existing conforming way for
WP-400 to validate a Host result after request transfer and should compare the
Core-owned carrier against the rejected alternatives above.

Until that review accepts the correction boundary, this topic remains
`DISCUSSING`. No authoritative migration, tranche impact transition,
production implementation, gate registration, or dependent Consumer work is
authorized by this document.
