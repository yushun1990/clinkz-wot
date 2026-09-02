# Primary Data Flows

Status: active v5.1 authority.

## Canonical flow

```text
TD document or produced-Thing draft
        |
        v
parse + preserve extensions + validate W3C structure
        |
        v
capture immutable planning context
  (policy + startup registration view + dependency generations)
        |
        v
shared planner -----> logical plans -----> binding compiler extensions
        |                                       |
        +---------------------------------------+
                            |
                            v
                 admitted immutable plan set
                            |
          +-----------------+------------------+
          |                                    |
          v                                    v
 Consumer selection                       Producer exposure
          |                           prepare/readiness/activate/commit
          v                                    |
  selected OutboundRequest                     v
          |                         committed route + serving permit
          v                                    |
 Client Binding                         inbound / emission SPI
          |                                    |
          v                                    v
 validated result                     Servient dispatch/coordinator
          |                                    |
          +-------------------> application <--+
```

Every downward transition moves an owned, generation-bearing value or lease.
No stage reaches back into the full TD to rediscover a decision already present
in the plan.

## Cross-flow execution invariants

`CONCUR-LOCK-001`: Engine lock ordering is registry, Thing or handle state,
operation slot, binding-local state, then diagnostic/status state. Code MUST
NOT acquire an earlier class while holding a later class. Binding-internal
locks follow binding-local state and have a documented order. User callbacks
are not a lock class.

`CONCUR-USER-001`: Before calling a handler, provider, codec extension,
readiness driver, transport callback, or status sink, the engine MUST own the
minimum dispatch state and release every engine lock and critical-section
guard. Reentrancy observes the same published state as a new concurrent call
and cannot depend on recursive lock acquisition.

`CONCUR-CRIT-001`: A constrained critical section performs only bounded table,
slot, counter, or state transitions. It MUST NOT parse, allocate, validate,
expand a URI, call user code, poll transport work, or iterate a collection
whose maximum work is not fixed by the selected static profile.

`CONCUR-LIN-001`: Every public lifecycle or replacement operation documents one
linearization point. Replacement affects only dispatches selecting after
publication. Exposure publishes serving authority once; destroy closes new
admission once; subscription stop closes sample admission once; and every race
is classified on one side of its applicable point while retaining the old
owner for already admitted work.

## Consumer admission and interaction

1. `consume` captures the TD document, policy snapshot, credential/provider
   identities, and complete client-binding registration set.
2. The shared planner applies defaults, resolves targets and security, queries
   declared capabilities, and builds ordered logical candidates.
3. Each candidate's owning binding compiler creates a bounded protocol artifact
   or a bounded lazy-artifact descriptor.
4. Admission reserves the complete plan-set footprint and publishes one
   immutable consumed-handle generation.
5. An application operation selects within that plan set using explicit
   options and current security applicability.
6. Core/Servient constructs one `OutboundRequest` containing only selected
   execution facts and committed security material.
7. The selected client binding executes through an owned call or caller-owned
   constrained slot.
8. Shared response validation maps transport metadata and payload into the WoT
   result or a structured error before application delivery.

The first v5.1 Consumer Property Read aggregate is the narrower executable
projection of that broad flow:

```text
owned Thing
  -> TD-owned bounded Basic validation and representation-aware census
  -> Planning deterministic one-registration/all-readable preflight
  -> Servient conservative persistent-capacity reservation
  -> Planning complete plan/candidate materialization
  -> every compiler bounds evaluation
  -> completed all-bounds-before-start barrier
  -> sequential compiler start and eager compilation
  -> Planning-owned sealed TD-free aggregate draft
  -> Servient atomic publication with retained Thing and registration owner

read_property(name, options)
  -> lease the published plan set
  -> resolve the addressed property/Form row and eager artifact
  -> construct a name-free OutboundRequest
  -> execute only the Core-sealed complete registration
  -> settle the call and release the lease
```

Any validation, security, ledger, materialization, bounds, compilation,
reconciliation, cancellation, or seal failure terminates the unpublished
generation. In particular, materialization or bounds failure releases every
still-uncommitted reservation and causes zero compiler `start` calls. The
slice publishes neither a partial property/Form subset nor a handle whose
lookup is incomplete.

ADR-0017 permits fallback only before security commit and binding input:
side-effect-free security inapplicability or an exact deterministic lazy
compiler negative may skip one already admitted candidate in frozen order.
Binding input rejection, mutable health, transient failures, and every
post-acceptance result never trigger automatic fallback. Each eligible skip has
one fixed-width diagnostic bounded by the admitted candidate count. Fallback
never asks a binding to scan forms or compile an unbounded artifact during
transport execution.

## Producer finalization and exposure

1. A produced-Thing draft contains TD data and registered application handlers,
   not live protocol routes.
2. Captured form contributors deterministically add generated protocol forms
   and their endpoint reservation identities without opening listeners or
   contacting peers.
3. The shared planner validates the effective TD, assigns exactly one binding
   owner to every inbound plan and publication target, and eagerly invokes the
   selected Producer compiler. For an application-supplied form the compiler
   provides the canonical endpoint reservation identity; for a generated form
   its identity must equal the contributor output before freeze.
4. The Servient reserves plan, route, readiness, ingress, response, status, and
   cleanup capacity before the first binding side effect.
5. The immutable Producer plan set is frozen before route preparation.
6. Each selected server binding progresses a route-scoped
   prepare/readiness/activate/commit transaction. Successful commit returns a
   distinct committed-closed guard and does not open request admission.
7. After every route is committed-closed, the Servient performs one
   generation-checked transition that publishes the plan set and produced
   registry generation and makes their shared serving activation authority
   available for route-admission claims.

Failure before publication rolls back every prepared, active, or
committed-closed route. No partially serving Thing becomes visible through the
local registry.

## Inbound request dispatch

1. The Servient validates the private serving record, moves the unique accept
   lease for one committed route into the claimed-call owner, and consumes that
   claim into a route-scoped activation permit.
2. The binding may produce one owned `RouteInboundRequest` only while
   `poll_accept` holds that permit. The request carries the route, plan, form,
   correlation, payload, and transport-auth identities.
3. The Servient validates the route generation and admits an in-flight response
   opportunity before invoking application behavior.
4. Shared security, codec, schema, URI-variable, and scope processing executes
   from the immutable inbound plan.
5. The Servient invokes the selected handler outside registry locks.
6. Core consumes the request's unique `RouteResponseOpportunity` and seals the
   handler result into one `RouteInboundResponse`. A valid Property Read
   success contains one application payload with `Ok` status and no binding or
   action metadata; an invalid success becomes a deliverable validation error,
   while a handler error remains unchanged.
7. The owning binding revalidates the live route, generation, and correlation,
   maps the sealed result to the protocol, and sends it through bounded
   progress. It does not repeat handler-origin validation, and retry never
   reinvokes the application handler.

The v1 binding model is engine-orchestrated. A binding does not receive a
general-purpose `Dispatch` handle and does not call handlers from a hidden task.
It also does not observe the Servient registry. A bounded host reactor may wake
a route or retain admitted protocol-local ingress, but a wake or queued frame is
not serving authority.

## Subscription flow

1. Selection and admission reserve a `SubscriptionId`, plan generation,
   driver/slot footprint, item/byte capacity, and cleanup capacity.
2. The binding start operation returns one pull-capable driver or activates one
   caller-owned slot.
3. The Servient installs the driver before publishing the application facade.
4. The application subscription capability is linear. The host `Subscription`
   facade owns exclusive access to one registry driver cursor; the manual
   `StaticSubscription` token owns one generation-bearing caller slot identity.
   Both facades are non-`Clone`.
5. Binding-local flow control or an explicitly selected bounded adapter owns
   pre-delivery buffering. Core does not impose one queue implementation.
6. Explicit stop, remote terminal, deadline, handle drop, and Servient drain
   converge on the same generation-safe teardown path.

There is no separately cloneable public receive view or per-subscription
control handle in v1. Multiple application consumers require separately
admitted subscriptions with distinct `SubscriptionId`s. Moving or externally
serializing one facade still polls one cursor; it does not create independent,
competing-consumer, or broadcast semantics. Application-local fan-out after
receipt is outside the subscription abstraction and owns its own bounds.

`observe_all_properties` and `subscribe_all_events` execute one selected
Thing-level plan and one native/coalesced driver. They are not silently lowered
to N subscriptions.

## Producer emission flow

1. The produced handle validates the target and payload against its effective
   TD and immutable plan set.
2. Servient creates one bounded emission record with a retained payload lease,
   local-subscriber cursor, selected binding-publication targets, result cells,
   and cleanup capacity.
3. Local application delivery and each binding publication progress under a
   profile-specific Servient policy.
4. A concrete binding owns protocol-native remote fan-out; Servient owns
   cross-binding scheduling and aggregate status.
5. Per-target outcomes remain attributable. One slow binding cannot consume an
   unrelated binding's lane or erase its result.

Core defines emission values and one-binding progress semantics only. It does
not contain an `EventBroker` or global dispatcher.

## Discovery-to-consume flow

Discovery produces source-bearing TD documents through a client contract. It
does not host an implicit Directory service. A selected discovery result enters
the same consume admission path as an application-supplied document; source,
freshness, trust, and redaction evidence is preserved through validation and
plan construction.

`DIR-SCOPE-001`: The engine contains only the remote Directory client
boundary: owned request/result values, sessions, watches, cancellation,
pagination, revision/lease identities, trust metadata, and portable progress
adapters. Constructing a Servient MUST NOT create an in-process Directory, and
Servient and Discovery MUST NOT depend on Directory service composition,
storage, server query planning, replication, redaction orchestration, or
endpoint-hosting crates. Those concerns require a later domain-entry review.

## Ownership checkpoints

At each checkpoint, failure is atomic or leaves an addressable cleanup owner:

- document accepted;
- registration snapshot captured;
- plan footprint admitted;
- plan set published;
- route side effect started;
- binding call accepted;
- subscription driver installed;
- response opportunity accepted; and
- cleanup transferred or terminally recorded.

Detailed state machines must name these boundaries explicitly. A destructor is
never the only owner of fallible cleanup.
