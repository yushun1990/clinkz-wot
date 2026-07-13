# WP-100 Interaction Output API and Staging Amendment

Status: Frozen

Base design revision: v4.6

Amendment id: WP-100-OUTPUT-API-001

Affected requirements: API-PAYLOAD-001, API-SURFACE-001,
API-OWNERSHIP-001, HANDLER-API-001, BIND-IO-001, BIND-OUT-001,
RES-LIMIT-001, IMPL-CONFORM-001

## Purpose

This normative amendment closes the public method signatures and work-package
ownership left implicit by `WP-100-ERR-DISPOSITION-001`. It does not change the
logical schemas, success semantics, response validation rules, or resource
limits frozen by that amendment.

The clarification prevents the WP-100 value migration from inventing a
temporary public response envelope before the route and binding identities
owned by WP-200 and WP-300 exist. It also prevents constructors from implying
that caller-supplied binding response metadata is trusted.

## Binding Response Metadata API

The exact v1 constructor and getter surface is:

```rust
impl BindingResponseMetadata {
    pub const fn primary(
        binding_id: BindingId,
        binding_generation: BindingGeneration,
        plan_id: PlanId,
        status_code: u16,
    ) -> Self;

    pub fn try_additional(
        binding_id: BindingId,
        binding_generation: BindingGeneration,
        plan_id: PlanId,
        index: u16,
        status_code: u16,
        limits: &ResourceLimits,
    ) -> Option<Self>;

    pub const fn binding_id(&self) -> BindingId;
    pub const fn binding_generation(&self) -> BindingGeneration;
    pub const fn plan_id(&self) -> PlanId;
    pub const fn selection(&self) -> ResponseSelection;
    pub const fn status_code(&self) -> u16;
}
```

`primary` constructs `ResponseSelection::Primary` and does not consult the
additional-response limit. `try_additional` constructs
`ResponseSelection::Additional(index)` only when
`limits.additional_responses_per_form_max()` is `Some(limit)` and
`u64::from(index) < limit`. `None` and `Some(0)` both reject every additional
index. The existing `ResourceLimits` schema rejects limits above `65_536`, so
the maximum accepted pair is limit `65_536` with index `65_535`.

Both constructors establish bounded shape only. They do not prove that the
binding, generation, plan, response branch, or status code belongs to a live
request.

## Interaction Output Metadata API

The exact v1 builder and getter surface is:

```rust
impl InteractionOutputMetadata {
    pub const fn with_action_invocation(
        self,
        action_invocation: ActionInvocationRef,
    ) -> Self;

    pub const fn with_payload_role(self, payload_role: ResponsePayloadRole) -> Self;

    pub const fn with_untrusted_binding_response(
        self,
        binding_response: BindingResponseMetadata,
    ) -> Self;

    pub const fn action_invocation(&self) -> Option<ActionInvocationRef>;
    pub const fn binding_response(&self) -> Option<BindingResponseMetadata>;
    pub const fn payload_role(&self) -> ResponsePayloadRole;
}
```

The word `untrusted` is part of the public builder name. No v1 constructor or
builder names binding response metadata as validated or trusted. Validation
retains the same fixed-size value only after checking it against the live
request and compiled response plan.

## Final Inbound Response Envelope

The final v1 `InboundResponse` schema is owned by
`clinkz_wot_core::binding`, re-exported as
`clinkz_wot_core::InboundResponse`, and implemented in WP-300 after
`BindingRouteKey` exists:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InboundResponse {
    route: BindingRouteKey,
    correlation: CorrelationId,
    result: CoreResult<InteractionOutput>,
}
```

Its exact public surface is:

```rust
impl InboundResponse {
    pub fn try_success(
        route: BindingRouteKey,
        correlation: CorrelationId,
        operation: clinkz_wot_td::data_type::Operation,
        output: InteractionOutput,
    ) -> CoreResult<Self>;

    pub fn failure(
        route: BindingRouteKey,
        correlation: CorrelationId,
        error: CoreError,
    ) -> Self;

    pub const fn route(&self) -> &BindingRouteKey;
    pub const fn correlation(&self) -> CorrelationId;
    pub fn result(&self) -> Result<&InteractionOutput, &CoreError>;
    pub fn into_result(self) -> CoreResult<InteractionOutput>;
    pub fn into_parts(
        self,
    ) -> (BindingRouteKey, CorrelationId, CoreResult<InteractionOutput>);
}
```

The private `CoreResult<InteractionOutput>` is the only terminal channel. A
failure contains no empty or default successful output, and a success contains
no `CoreError`. There are no public `output` or `error` fields, no constructor
that accepts both, and no `Default` implementation. Moving the result out of
the envelope does not clone its payload or error.

`try_success` is the producer/handler-origin metadata and action-shape check. It
rejects binding-response metadata and checks the `Created`, `Accepted`,
`OperationStatus`, payload, action-reference, and supplied-operation
combinations frozen by `WP-100-ERR-DISPOSITION-001`. It does not prove that its
publicly supplied route, correlation, or operation names an admitted request.
WP-400 derives the operation from the admitted `InboundRouteMatch` and rechecks
the in-flight response opportunity; WP-300 response delivery rechecks the route
generation and correlation. Schema/media classification and required-output
validation have already succeeded before this call. `failure` cannot construct
or retain an `InteractionOutput`.

The current correlation-only `InboundResponse` in `core/src/inbound.rs` is a
legacy implementation location and shape. WP-100 must not replace it with a
second temporary public sum type. It is replaced once, in WP-300, together with
`BindingRouteKey`, final `InboundRequest`, response delivery, and correlation
ownership. Until then it cannot satisfy `BIND-IO-001` or the end-to-end
success/error evidence.

## Validation Ownership and Work-Package Order

The response contract is implemented without crossing the frozen DAG:

1. WP-100 implements the six frozen interaction values, the exact APIs above,
   and the local `InteractionOutput::try_with_metadata` shape check. It does not
   claim binding-response authenticity, final `InboundResponse`, or end-to-end
   success/error evidence.
2. WP-200 compiles immutable response classification facts, including primary
   and additional branch identity, schema/media classification, and actual
   additional-response count. It does not publish a response.
3. WP-300 owns the final core-owned binding request and response envelopes and
   the shared consumer/binding-origin validator at
   `clinkz_wot_protocol_bindings::validate_untrusted_binding_output`:

   ```rust
   pub fn validate_untrusted_binding_output(
       request: &BindingRequest,
       output: InteractionOutput,
       limits: &ResourceLimits,
   ) -> CoreResult<InteractionOutput>;
   ```

   `BindingRequest` supplies the selected live binding, generation, plan, and
   compiled response facts. The function checks those identities, the
   classified branch, actual plan count, active resource limit, schema/media
   result, and operation-specific output invariants before returning success.
   It is the only public shared validation entry point for a client binding's
   untrusted output.
4. WP-400 passes application-handler output through
   `InboundResponse::try_success`; it does not implement a second copy of the
   metadata or action-invariant rules.
5. WP-600 concrete bindings populate untrusted metadata directly from the
   protocol-native response without treating the opaque status code as an HTTP
   code in core. Status provenance is a concrete producer conformance property;
   it is not inferred later from the `u16` value or from diagnostic text.
6. WP-700 closes the end-to-end evidence that a success never carries a
   `CoreError` and a failure never carries an `InteractionOutput`.

Existing aggregate property helpers that discard per-item status or metadata
are compatibility code and are not response-model evidence for this amendment.

## Evidence

The exact package evidence ownership is:

- WP-100 `core-public-surface`: derives, paths, private fields, method
  signatures, defaults, metadata round trips, additional-index boundaries, and
  the local no-payload `OperationStatus` shape rejection in every core feature
  cell. It does not mark `BIND-IO-001` complete.
- WP-200 `logical-plan-footprint`: immutable primary/additional response facts,
  actual additional count, schema/media classification identity, and bounded
  admission ownership.
- WP-300 `binding-response-validation`: final response XOR, binding/plan/branch
  mismatch rejection, resource and actual-count boundaries, action-status
  combinations, and exactly-one response terminal.
- WP-400 `servient-response-validation`: handler binding-metadata rejection and
  use of `InboundResponse::try_success` for every producer response path, with
  the supplied operation derived from the admitted route match and the route,
  generation, and correlation rechecked against the response opportunity.
- WP-600 `binding-response-provenance`: protocol status and branch metadata are
  populated from the native response and survive shared validation without
  string or diagnostic inspection.
- WP-700 `end-to-end-response-boundary`: stale correlation handling and proof
  that every composed success contains only an `InteractionOutput` and every
  composed failure contains only a `CoreError`.
