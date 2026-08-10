# WP-400 Property-Read Servient Slice Entry Audit

Status: Review pending

Design revision: v5.0

Admission scope: `WP-400-PROPERTY-READ-SERVIENT-SLICE`

Verdict: Candidate ready for independent review

## Selected boundary

The verified route-reservation projection supplies the final missing upstream
output. This candidate composes, but does not reinterpret, the completed
Property Read handler, plan, binding, Producer-route, and reservation
contracts. The first legal binding side effect remains exactly:

```text
expose:Draft:expose->Preparing
  binding-route:Absent:begin_prepare->Preparing
```

Before that nested transition, Servient product code must own a complete
registration snapshot, compiled plan-set record and lease, produced Thing and
generation, matching handler view, route record, activation record, and every
route/ingress/guard/readiness/accept/response/status/cleanup reservation. Only
then may it derive `BindingRouteKey` and `PrepareInput` from the real plan
output and call the real binding registration.

No runner, test adapter, Planning code, or opaque binding payload may assemble
those WP-400-owned values.

## Active v5 public boundary

The narrow slice uses the already frozen public names and adds no one-off
Property Read facade:

- `StaticServientBuilder::binding_registration` consumes one complete
  `StaticBindingRegistration<B>`; its Property Read handler input is one real
  `StaticHandlerRegistration<'h, H>` where `H: ReadPropertyHandler`.
- `ServientBuilder::binding_registration` consumes one complete
  `HostBindingRegistration`. `ExposedThingHandle::set_read_property_handler`
  admits one synchronous `ReadPropertyHandler` plus its declared
  `HandlerFootprint`; WP-400 privately owns the necessary host erasure in
  `PropertyReadHandlerRecord`.
- `StaticServient::step` and `Servient::step` each accept an explicit
  `Context` and `WorkBudget` and return `StepStatus<()>`. The async/no-std
  projection is an adapter over the same bounded step contract and introduces
  no executor dependency.
- `ExposeState` and `CompiledPlanSetState` are read-only lifecycle views.

The v4.9 broad work-package input named `StepStatus<RuntimeEvent>` and
`HostHandlerRegistration`. Under ADR-0018, `CAP-STATUS-001` and
`API-SURFACE-001` are inactive domain-entry or retired authority, and neither
corresponding source type exists. This narrow active-v5 candidate therefore
does not activate either broad family. A later broad WP-300/WP-400 review may
replace the unit step value with an admitted status event and may introduce a
reusable Core host-handler registration. The narrow runtime exposes state and
observable fixture I/O, not a status-stream claim.

## Complete first-entry provenance

The candidate closes all nine D46 rows before any binding side effect:

| Input | Production owner/carrier | Required validation |
| --- | --- | --- |
| Producer logical route | `PlanBuildOutput.logical_plans` and `artifact_refs` | Selected plan, artifact, binding, and plan-set generations agree |
| Producer artifact metadata | real `BindingArtifactEnvelope` | Role is Producer-route and compiler-owned reservation is present |
| Complete binding registration | frozen `StaticBindingRegistration` or `HostBindingRegistration` snapshot | identity, compiler, server, resources, ingress, and status are complete |
| Produced Thing/generation | `ExposedThingRecord` and `ThingSlotId` | TD is read only during planning; generation is retained thereafter |
| Property Read handler coverage | `StaticHandlerRegistration` or private `PropertyReadHandlerRecord` | target and handler generation cover the selected plan exactly |
| Servient admission policy | builder-owned `ResourceLimits`, `Deadline`, clock identity, and caller-supplied `WorkBudget` | every required capacity is reserved before preparation |
| Compiled plan-set ownership | `CompiledPlanSetRecord` plus non-`Clone` `PlanSetLease` | plan set is frozen and pinned to the produced generation |
| Route preparation assembly | `BindingRouteRecord`, `BindingRouteKey`, and `PrepareInput` | route key uses only admitted artifact metadata and Servient-owned generations |
| Activation and cleanup ownership | `ServingActivationRecord`, `ServingActivationAuthority`, `RouteAcceptLease`, and cleanup reservations | authority remains unpublished until every route is committed closed |

The only legal semantic roots are the TD, complete binding configuration,
root generation/slot input for the application-static profile, handler result,
and explicit resource/time policy. Host slot allocation is Servient-owned.
Prepared/readiness/active/committed guards, accepted requests, security
results, handler context/input/output, response opportunity, publication, and
drain state arise only after the first entry.

## Runtime ownership and linearization

One narrow produced generation owns one immutable plan set, one route, one
handler, and one `ServingActivationAuthority`. Exposure proceeds through the
registered `compiled-plan-set`, `expose`, and `binding-route` machines.
Publication occurs only at `expose:Committing:publish->Serving`, after the
route returns a committed-closed guard. The authority and route accept lease
become selectable atomically with that publication.

Each step claims the route lease, creates the borrowed activation permit,
polls the binding outside any Servient mutable-state boundary, and transfers a
complete `RouteInboundRequest` into one `InFlightRecord`. Servient constructs
`HandlerContext` from retained production identities, calls the selected
handler exactly once, and consumes the request's unique response opportunity
into `RouteInboundResponse`. Delivery acknowledgement releases the in-flight
and response reservations exactly once.

Destroy/deactivation first closes authority selection, rejects later accepts,
drives route shutdown, retains every incomplete cleanup object, and releases
route, request, response, plan-set, and cleanup counts only at their terminal
boundaries. This narrow slice makes no drop-transfer or external cleanup
executor claim.

## Implementation topology

After an approved independent review and a distinct pre-source admission
checkpoint, the only permitted product implementation paths are:

- `servient/Cargo.toml`;
- `servient/src/lib.rs`;
- `servient/src/builder.rs`;
- `servient/src/handle.rs`;
- `servient/src/registry.rs`;
- `servient/src/servient.rs`; and
- new `servient/src/property_read.rs`.

The implementation checkpoint may also extend exactly one non-product mock
support path,
`tools/compile-contracts/wp300-property-read-binding-slice/src/lib.rs`, so the
existing complete mock registration can inject one protocol-neutral request
and retain one delivered response in allowed deterministic I/O and
instrumentation state. That support object may create protocol frames, but it
may not create a plan, artifact, route key, `PrepareInput`, activation permit,
handler value, response opportunity, or cleanup owner.

Any other product or support source path revokes admission pending an
intersecting impact review.

## Executable contract

`tools/compile-contracts/wp400-property-read-servient-slice/` remains outside
the workspace. Its no-default manual cell, std host cell, and async/no-std
compile cell must enter the real Servient product API. They consume the real
TD, complete WP-300 mock registration, real handler registration, explicit
resource/time policy, and a caller-supplied budget/waker only.

The runtime cells must prove:

- TD access ends after planning and no runtime TD selector is used;
- one real compiler output reaches the first legal Servient route entry;
- no accept occurs before Servient publication;
- the route and handler are selected by Servient product code;
- the handler is called once with protocol-neutral input;
- the response opportunity is consumed once and the mock binding observes the
  exact output;
- deactivation rejects a later request; and
- all narrow route, in-flight, response, and cleanup counts return to zero.

The compile projection must prove that the mock binding has no Servient
dependency, all registration and runner inputs use public production types,
the activation permit cannot be constructed or retained by the runner, and
the async/no-std surface adds no executor.

The completion check must currently fail exactly because
`clinkz-wot-servient` has no Planning dependency or reviewed Property Read
module. Candidate preparation creates neither final architecture fixture root.

## Required independent negative evidence

Review must simulate the exact product/support transition and reject every
registered mutation:

1. fixture-restated artifact or reservation;
2. dropped or mismatched produced, plan-set, binding, plan, or route generation;
3. Planning- or Servient-side reservation reconstruction;
4. host-erasure loss of artifact metadata;
5. unrelated reservation paired with the real artifact;
6. partial or bare binding registration;
7. binding preparation before resource and cleanup reservation; and
8. host/static semantic divergence.

It must additionally reject premature publication, accept without the unique
route lease/permit, direct runner-created handler context or response
opportunity, handler double-call, response double-delivery, cleanup-object
loss, source outside the exact path sets, and creation of either final
architecture fixture root.

## Exact exclusions

This candidate does not implement or claim:

- async or step handler families;
- Consumer calls, subscriptions, or Producer emissions;
- multiple Things, multiple routes, fairness, sharding, or workload bounds;
- dynamic binding registration, plan regeneration, lazy artifacts, fallback,
  forms contribution, or degraded publication;
- a runtime event/status stream;
- broad cleanup executor or drop transfer;
- a production protocol or Zenoh behavior; or
- completion of `PROPERTY-READ-ARCHITECTURE`.

## Candidate topology

The immutable candidate base is fetched, merge-validated default revision
`fcce9e69036459506a163ac73ef5542f92e5eb7f`. The candidate is its exact
single child, changes only the 18 paths registered in the Property Read gate,
and changes no product or support source. Its commit id is resolved only after
the candidate commit is created.

Independent review must bind that immutable commit, execute all eleven
registered prechecks, exercise the three positive cells and all negative
mutations in an isolated checkout, and record a separate attestation commit.
Only then may a distinct admission checkpoint switch this tranche to
`in-progress`/`approved` and bind the exact default-integrated admission base.
