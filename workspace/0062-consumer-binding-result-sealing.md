# 0062 Consumer Binding Result Sealing

Status: MIGRATED

Kind: independently confirmed architecture correction

Priority: HIGH

Target: Core-mediated, no-bypass sealing of every Consumer Property Read
binding result through the complete registration in both Host-erased and
application-static execution

## Scope and authority

The first investigation of this topic identified a real Host-only ownership
defect. Independent review of PR #58 confirmed that defect and found the same
loss on the allowed synchronous application-static path. The correct boundary
is therefore not a Host carrier with later static follow-up. It is one
profile-neutral semantic rule implemented by two profile-appropriate physical
carriers: every result from an installed Consumer binding is sealed by Core
before it can reach WP-400 or application code.

PR #58 recorded the independently confirmed defect, reopened the affected
WP-300 Consumer tranche, and superseded its prior evidence. The correction
below is now projected into the Binding SPI, interaction and deployment
architecture, API ownership matrix, and the WP-300 readmission record. Those
authoritative artifacts, rather than this migrated investigation, govern the
next implementation. The migration does not change an architecture gate,
unblock WP-400, or change the v5.1 requirement set.

The separate aggregate Planning -> Servient investigation is preserved in
`workspace/0063-consumer-plan-set-handoff-closure.md`. Its unresolved questions
are downstream of this one-call result-sealing boundary.

## Confirmed repository facts

The following facts come from active v5.1 authority and current source/tests,
not from the prior 0062 proposal or closed PRs #56 and #57.

1. `docs/spec/interaction-core.md` requires a binding-origin success to remain
   untrusted until Core checks its metadata and Property Read output shape
   against the live selected request. Only then may the output reach
   application code.
2. The only admitted public wrapper borrows that request:

   ```rust
   pub fn validate_untrusted_binding_output(
       request: &OutboundRequest,
       output: InteractionOutput,
   ) -> CoreResult<InteractionOutput>;
   ```

3. The target Host `ClientBinding::invoke` consumes the request and returns a
   raw call whose normal and late-return branches contain only
   `CoreResult<InteractionOutput>`. After acceptance, neither the call nor its
   settlement exposes the request or a Core-derived validation authority.
4. An application-static pending request is retained in
   `ClientRequestSlot`, so pending normal and cancellation-late results can in
   principle be validated against `slot.request()` before acknowledgement.
5. The static contract also permits synchronous completion. The current
   external-author fixture returns `StartStatus::Ready(Ok(output))` without
   initializing the slot, and its test confirms that the slot remains vacant.
   The moved request is unavailable on that valid path.
6. Existing WP-300 tests prove request validation, Host execution,
   Host cancellation, static synchronous/pending execution, and static
   cancellation as separate facts. They do not prove one composed path in
   either profile that transfers a real request and seals every successful
   terminal branch before delivery.
7. The validated complete registrations expose raw execution projections:
   Host through `HostBindingRegistration::client()` and static through
   `StaticBindingRegistration::server_mut()` followed by
   `StaticBindingComponents::client_mut()`. Adding an optional sealing helper
   while leaving those as WP-400 execution routes would not create a
   no-bypass boundary.

The minimal Host failure is:

```rust,ignore
let request = OutboundRequest::property_read(/* selected values */)?;
let mut call = registration
    .client()
    .expect("Consumer capability has a client")
    .invoke(request, artifact)?;
let output = poll_to_terminal(&mut call)?;

// `request` was moved into `invoke`.
let output = validate_untrusted_binding_output(&request, output)?;
```

The static synchronous failure is equivalent:

```rust,ignore
let request = OutboundRequest::property_read(/* selected values */)?;
let result = registration
    .server_mut()
    .client_mut()
    .start_request(request, artifact, &mut slot, budget)?;

// `Ready(Ok(output))` is allowed while `slot` is vacant and `request` moved.
```

Reconstructing a second equal-looking request, passing expected ids separately,
or trusting the binding to validate its own metadata would violate the accepted
live-call/no-bypass ownership contract.

## Correction invariant

Every installed Consumer Property Read execution must enter through one
Core-mediated complete-registration operation. Immediately before transferring
the request to the binding, Core derives a private, non-`Clone`, single-use
validation seal from that exact request. The seal contains only the immutable
facts required by the existing private WP-100 kernel. It is not a second
request, plan lease, public token, candidate, registration snapshot, or bag of
caller-supplied expected ids.

After acceptance, no successful `InteractionOutput` may escape the complete
registration's runtime projection without consuming that sealing authority or
being checked against the exact live slot request. Binding-origin failures pass
through unchanged. A validation failure replaces only the nominal success
inside the same terminal branch; it must not become cancellation, erase a late
return, or alter cleanup ownership.

This is a thin Core decorator around the already accepted binding lifecycle,
not a second protocol call state machine. WP-400 continues to own the plan-set
lease, caller interest, deadline/cancellation choice, result delivery, cleanup
reservation, drain, and reclamation.

## Host physical boundary

The validated `HostBindingRegistration` must expose a Consumer start operation
with this behavior:

1. Before protocol work, check the registration, selected artifact envelope,
   and request identities wherever they overlap: binding id/generation,
   configuration, compatibility, plan-set generation, plan, and Consumer-call
   role.
2. Derive the private validation seal from the exact request.
3. Invoke the author-supplied raw `ClientBinding`; on pre-acceptance rejection,
   return the unchanged request and discard the unused seal.
4. On acceptance, return one sealed Host call before the first protocol side
   effect. The artifact borrow does not enter the returned call.
5. Delegate `lifetime_footprint`, `poll_result`, `start_cancel`, `poll_cancel`,
   and `next_deadline` to the underlying `HostBindingCall` while accounting for
   the decorator and seal.

The decorator intercepts only successful output values:

```text
normal Ok(output)
  -> validate with the private seal
  -> Ok(validated) | Err(CoreError::Validation)

BindingCallSettlement::Returned(Ok(output))
  -> validate with the private seal
  -> BindingCallSettlement::Returned(
         Ok(validated) | Err(CoreError::Validation)
     )
```

Existing binding errors, `Cancelled` settlements, retry classification,
complete/transfer-required/residual cleanup disposition, and deadlines keep
their original classification. When complete cleanup work is offered to
another owner, the underlying call, seal, decorator state, and their accounting
move or return together as one work object.

## Application-static physical boundary

The validated dual-role `StaticBindingRegistration` must mediate the same
semantic boundary without pretending that every call occupies a slot:

1. Before calling `PollClientBinding::start_request`, validate shared
   registration/request/artifact identity and derive private sealing authority.
2. Pre-acceptance rejection returns the exact request, leaves the slot vacant,
   and discards the seal.
3. `StartStatus::Ready(Ok(output))` is sealed immediately with the authority
   captured before the move. A synchronous binding error passes through.
4. `StartStatus::Pending` is accepted only after the mediator verifies that the
   occupied slot request matches the authority captured before the move.
   Normal pending and cancellation-late `Returned(Ok(output))` results are then
   sealed against `slot.request()` before the terminal value is exposed.
5. A validation failure remains a normal or late returned
   `Err(CoreError::Validation)` in the same terminal classification.
6. Acknowledgement and clear are available to the installed runtime only after
   a sealed terminal result or an admitted terminal cancellation disposition
   has been retained. They cannot be used to discard an unvalidated success.
7. Slot reuse still occurs only after binding-private state drop in caller
   context, and zero-budget progress remains a no-op.

The static mediator may need a private sealing/disposition bit or equivalent
linear carrier, but it does not duplicate the binding's request lifecycle
state machine. Its retained/transient memory and work costs must be included in
the registration's existing admitted resource accounts.

### Static cleanup-transfer exclusion

The active one-shot static Consumer boundary has no cleanup-transfer owner or
target. Its caller-owned request slot is the manual cleanup owner. For request
cancellation, the installed static runtime supplies a
`CleanupPhaseContext::bind(...)` context whose `transfer_owner()` is `None`,
not a Host-style `bind_with_transfer_owner(...)` context.

`CleanupTransferRequest` has private fields and no direct public constructor.
The only admitted derivation consumes the unchanged phase with a
production-provided named owner through `try_into_transfer_request()`. For the
static Consumer phase above, that call must return the unchanged phase as
`Err(context)`. No authorized
`BindingCancellationDisposition::TransferRequired` branch is therefore
reachable for the live request. The runtime retains the slot and phase under
manual progress until a sealed `Returned` value, `Complete`, or
`ResidualExternalState` disposition.

A raw authoring implementation could violate the SPI by discarding the
supplied phase and manufacturing an unrelated one. Such a nominal value is not
production-authorized transfer authority: the admitted static runtime has no
owner or target that could accept it. It must not activate transfer machinery
or permit the slot to be discarded.

The correction must prove this reachability boundary; it must not add a static
cleanup transfer reservation, `CleanupTransferEnvelope<ClientRequestSlot<_>>`,
`CleanupTransferTarget`, executor, or new transfer state. The generic
`BindingCallSettlement` spelling remains shared representation and does not by
itself activate every variant for every profile.

## Frozen no-bypass boundary

Raw `ClientBinding` and `PollClientBinding` remain public authoring SPIs so
third-party implementations can be constructed and tested before installation.
They are not installed runtime projections.

For an installed complete registration:

- WP-400 obtains only the Core-mediated sealed Consumer start/progress path;
- `HostBindingRegistration::client()` cannot remain an alternative raw
  execution route;
- a validated dual-role static registration cannot expose the Consumer client
  through `server_mut().client_mut()`; and
- Producer server progress remains available through explicit Producer
  projections that do not reveal the installed Consumer half.

A compatibility interval may preserve source spellings only outside the
installed runtime capability. It must not leave an unsealed route reachable by
WP-400. Exact Rust type and method names remain implementation choices, but
this access boundary is part of the correction rather than a deferred API
question.

## Alternatives rejected

### Host-only sealing

Rejected because synchronous static completion loses the moved request while
leaving `ClientRequestSlot` vacant. Pending-only static behavior is
constructible; the advertised static contract as a whole is not.

### Separately held public artifact reference or validation token

Rejected because it lets the validation authority be separated or swapped
between calls and makes normal and cancellation-late sealing, plus Host cleanup
transfer sealing, a WP-400 convention. The private seal must remain paired with
the accepted Host work object or the complete-registration static mediator.

### Clone or reconstruct `OutboundRequest`

Rejected because it creates duplicate owned request state and makes authority
depend on caller discipline. Making `OutboundRequest: Clone` weakens rather
than repairs its linear selected-call role.

### Binding-side or Servient-side validation

Rejected because bindings author the untrusted metadata and Servient must not
reimplement Core semantics. Both would create a bypassable second validator.

### Return the whole request on every terminal branch

Constructible but larger than necessary. It retains URI-variable and deadline
input after protocol-local derivation, increases lifetime accounting, and
still requires a trusted pairing rule. A private minimal seal preserves the
same validation semantics with less retained state.

### Borrow the request for every binding call

Rejected for this correction because it changes the accepted owned-transfer
contract and forces each Host binding to copy all call-varying input into its
own state. Owned pre-acceptance rejection remains otherwise constructible.

### Finish aggregate admission first

Rejected as the immediate progression unit. The sealing failure occurs with
one valid already-built plan, artifact, request, and complete registration.
Aggregate enumeration cannot repair either moved-request path.

## Work-package and authority impact

- `WP-100-CONSUMER-CALL-VALUES-VALIDATOR` is explicitly reaffirmed for this
  finding. Its private kernel and negative cases are correct; the defect is
  that accepted execution does not retain a Core path to that kernel.
- `WP-200-CONSUMER-PROPERTY-READ-PLANNING` is unaffected by this one-call
  finding. Its separate aggregate limitations remain owned by workspace 0063.
- `WP-300-CONSUMER-PROPERTY-READ-BINDING` is affected and reopened. Its prior
  evidence proves execution and validation separately, so it is retained as
  superseded history rather than current completion evidence. A corrected
  admission and replacement evidence require independent review.
- WP-400 Consumer Property Read remains blocked. It must consume only the
  sealed complete-registration paths after WP-300 is readmitted and completed.
- The accepted Producer Property Read architecture gate, disjoint Producer
  scheduler evidence, production Zenoh status, milestones, and active
  requirement counts do not change.

No v5.2 requirement-set revision is indicated. The correction preserves active
v5.1 semantics and repairs their unconstructible execution carrier.

## Falsifiable correction boundary

Replacement WP-300 evidence must prove in both Host and application-static
representations:

- matching request/artifact/registration acceptance and exact-request
  pre-acceptance rejection;
- synchronous success, pending success, synchronous/pending binding failure,
  explicit cancellation, and cancellation-late returned success;
- valid output delivery exactly once after Core validation;
- binding id, binding generation, plan id, response selection, status, payload
  presence/role, and action-reference negatives after the original request has
  moved;
- validation failure preserving normal versus late-return terminal
  classification and cleanup truth;
- caller-interest drop and cleanup transfer retaining the complete Host call
  plus seal without duplication or loss;
- static cancellation receiving no named transfer owner,
  `try_into_transfer_request()` returning the unchanged phase, and no static
  transfer envelope, target, or executor becoming reachable;
- static acknowledgement/clear refusing to bypass an unsealed success and slot
  reuse only after a terminal disposition;
- no installed raw Host or static client projection reachable by WP-400;
- explicit retained/transient bytes and work accounting for the decorator,
  seal, and static mediation state;
- no artifact borrow, TD, Form, `InteractionOptions`, candidate list, or
  plan-set owner retained in the sealing carrier; and
- no reconstructed/cloned request, independently assembled expected ids,
  legacy binding path, aggregate `consume(td)` claim, Consumer architecture
  gate claim, or production protocol claim.

## Migration record

The stable correction is migrated by the WP-300 Consumer result-sealing
readmission change into `docs/spec/binding-spi.md`,
`docs/spec/interaction-core.md`,
`docs/architecture/40-protocol-binding-spi-and-deployment.md`,
`docs/api-ownership.csv`, `docs/work-packages/WP-300-bindings.md`, and
`docs/work-packages/WP-300-consumer-property-read-binding-admission.md`.
That exact readmission revision still requires fresh independent review before
production implementation starts. This migrated history does not independently
accept its own correction or change any gate status.
