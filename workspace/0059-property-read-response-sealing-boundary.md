# 0059 Property Read Response-Sealing Boundary

Status: MIGRATED

Kind: public cross-package contract reopening

Priority: HIGH

Target: the narrow Property Read handler-result to response-delivery boundary

## Trigger and authority conflict

Independent architecture review found that the implemented target-generation
carrier did not enforce the active v5 ownership split. `RouteInboundResponse`
is the narrow protocol-neutral response-delivery value, but its public `new`
and `success` constructors accepted any `InteractionOutput`. Servient used that
unvalidated constructor after handler return. A concrete binding could
therefore receive a nominally successful Property Read response containing
binding-origin metadata, action-only status or references, an operation-status
payload role, or no payload.

The active `API-PAYLOAD-001`, `BIND-IO-001`, and `BIND-DELIVERY-001` owners put
protocol-neutral handler-result semantics in Core, Servient orchestration in
WP-400, and live route/correlation validation plus protocol mapping at the
concrete binding. A retained v4.6 staging amendment instead proposed a future
broad `InboundResponse::try_success`. That amendment is not an active source in
the v5 authority map, and the implemented narrow carrier already owns the
linear response opportunity. Treating the proposed broad envelope as a second
runtime value would duplicate that ownership.

## Decision

`RouteInboundResponse` remains the current narrow Property Read
protocol-neutral response boundary and linear response-delivery carrier.
Successful public construction is available only through one Core-owned
Property Read handler-result sealing operation. It consumes the original
`RouteResponseOpportunity` and a `CoreResult<InteractionOutput>`.

- A handler `Err` is retained unchanged.
- A successful Property Read output is accepted only when it contains a
  payload, has `InteractionStatus::Ok`, uses
  `ResponsePayloadRole::Application`, and contains neither
  `BindingResponseMetadata` nor `ActionInvocationRef`.
- An invalid successful output becomes `CoreError::Validation` inside the same
  deliverable carrier. The original route/correlation opportunity is retained.
- The unvalidated `new` and `success` constructors are not public.
- Servient calls the sealing operation and owns no duplicate semantic check.
- Static, Host, and real-target Zenoh evidence bindings validate the complete
  live route identity (including binding and route generations) and correlation
  before acceptance and protocol mapping. Identity rejection returns the
  complete response without consuming the live in-flight state. Bindings do
  not repeat handler-origin validation.

A future broad `InboundResponse` may rename or generalize this same carrier and
validation kernel after broad domain entry. It is not a second envelope and is
not activated by this correction.

## Alternatives rejected

- Implementing the historical broad `InboundResponse` now would silently
  activate deferred client/action/subscription contracts and create two
  response carriers during the narrow gate.
- Keeping public unvalidated success construction and relying on Servient call
  discipline would leave the public invariant unenforced for other callers.
- Validating in Servient would duplicate Core-owned output semantics across
  orchestration paths.
- Validating in each concrete binding would mix handler-origin rules with
  protocol mapping and permit Host/static or protocol-specific drift.
- Returning validation failure outside `RouteInboundResponse` would strand the
  already admitted single-use response opportunity and break exactly-once
  delivery.

## Migration and evidence boundary

The decision is migrated to the active interaction-core and Binding SPI
specifications, narrow architecture flow, API ownership and response-delivery
state projections, WP-300/WP-400 narrow contracts, and the aggregate Property
Read gate. The v4.6 output amendment and broad work-package projections are
marked as historical v5-inactive input where they previously conflicted.

Executable evidence owns accepted output, every rejected shape, unchanged
handler errors, opportunity retention, compile-fail bypass attempts,
static/Host delivery plus terminal cleanup, and static/real-target rejection
of stale binding-generation, route-generation, and correlation identities with
complete response preservation. The aggregate gate remains `ready`; this
correction neither implements its absent fixture roots nor claims broad
WP-300, broad WP-400, WP-600, or release progress.
