# Thing-root QueryAllActions

Status: MIGRATED

## Semantic challenge and decision

TD 1.1 Recommendation §5.3.4.2 defines one Thing-root interaction obtaining
the status of all Actions. An ActionAffordance describes executable behavior;
it is not an invocation or its current status. One definition may have zero,
one or several concurrent invocations. Action output is a completed invocation's
application value, not a schema for a status resource. TD supplies neither a
universal status envelope nor a universal retention/snapshot guarantee.

The dated [HTTP Profile Working Draft, 2025-11-04,
§6.2.2.4](https://www.w3.org/TR/2025/WD-wot-profile-20251104/#queryallactions)
resolves the executable representation: one GET/200 JSON object, Action names
mapping to newest-first arrays of ActionStatus objects. Its retention note and
example include completed/failed records despite the prose's word "ongoing".
This is draft guidance, not a TD Recommendation requirement or a claim of full
profile conformance. Existing 202 acceptance and described {?id} controls remain
the bounded HTTP mapping; they are not the draft's 201 invocation contract.

Use that native aggregate shape, with a required bounded unique href for every
record, optional completed output, and pending/running/completed/failed states.
The Producer derives href from its original native correlation lease. No copied
href grants Engine control authority. Missing names mean no visible retained
records; empty arrays are allowed. Required output data, if any, is validated
against its own Action's retained schema. Failed records are successful query
observations, not failed query exchanges. This subset does not carry optional
timestamps/problem details; unsupported status fields refuse rather than disappear.

Keep native aggregation, ordering, correlation, retention and encoding inside
HTTP. Extend its participant to several Actions plus a root query on one actual
listener. All declared Actions must belong to that participant before it can
advertise complete aggregation; another listener/Binding cannot be silently
omitted. No remote fetch loop, hidden leaf query, application reinvocation,
snapshot across distributed stores or unbounded history is promised.

The Engine's structural gap is its exclusively per-invocation query permission
surface and value/Property-only finite validation. Correct those boundaries
directly: capture a distinct root route and application query policy; expose one
bounded read of original invocation owners with fresh root/member permission,
currentness and principal checks. Member policy denial filters a record; root
denial fails the exchange. Neither operation requests cancellation or progresses
application work. Retain named Action output schemas and a separate actual
status-validation service in the finite Consumer. The complete native payload
and response provenance remain observable through normal/late settlement.

## Alternatives

- N QueryAction calls lose native completeness/order and exclude invocations
  unknown to the Consumer; rejected.
- One state/value per Action destroys concurrent invocation identity; rejected.
- A generic Object schema cannot validate identity, status or member output;
  rejected. An additional normalized universal invocation/session store would
  duplicate native correlation and authority; rejected.
- MQTT request/reply or another real Binding could implement a group query, but
  supplies no clearer standardized status representation and adds a new server
  runtime without exposing a needed semantic distinction; deferred.
- Full HTTP Profile migration would also change acceptance, cancellation and
  status-retention semantics; unnecessary for this explicit bounded mapping.

## Completion boundary

Complete TD-selected Consumer and final-TD-qualified Producer execution over
real HTTP, with two definitions and concurrent same-definition invocations,
pending/completed/failed records, exact original identity, authorization,
validation/refusal, cancellation uncertainty, replacement/late work, saturation,
retention acknowledgement and final native/resource release. Migrate affected
Core/SPI/Planning/runtime/TD contracts, API ownership and capability coverage.
Independent architecture and implementation acceptance remains required; no
milestone, architecture gate or release qualification changes.

## Migration and evidence disposition

Semantic/data and root authority decisions are migrated to Interaction Core,
Planning, Binding SPI, Runtime Safety and TD; the HTTP README owns the concrete
subset and native limits. Work packages 200/300/400/600 project implementation
scope, API/state ownership projects the boundary, and PLAN updates coverage.
The new native-control counterexample also requires exact record-to-Action
attribution when Form ordinals coincide. The funding regression demonstrates
that acknowledgement is an ownership transfer, so settlements now pin their
original instance and validator until disposal and expose retained data by borrow
instead of detachable public owned fields. Existing callers migrate directly. Existing raw/continuing Action and composed
Producer evidence is reaffirmed only by rerunning affected fixtures; old results
are not inherited. These corrections add no new architecture target, ADR-level
platform choice, global gate or compatibility wrapper. Independent acceptance
remains a separate fresh-context review of the exact proposed revision.

## Review counterexamples: result ownership and response closure

The review reproduced three defects in the first implementation: a borrowed
InteractionOutput still exposes a cloneable payload Arc; retaining one fixed
instance reservation does not fund arbitrarily many acknowledged results; and
Consumer response qualification ignored non-HTTP extension assertions.

Correct the bounded Action owner directly. Public query and settlement results
lend byte/media slices and fixed metadata, never an InteractionOutput/Payload or
the original allocation's Arc. Sharing requires retaining the settlement owner;
copying bytes creates separately caller-owned backing. A per-admission result
lease covers the live invocation and transfers unchanged into its settlement.
Live plus acknowledged-but-retained results count against the original store's
capacity, including after root loss. Managed publication also captures its actual
original account bank: each result's maximum backing/cleanup reservations are
acquired before behavior entry and stay owned until disposal. Reuse cannot
renew either capacity for free. Query views cannot bypass this boundary before
acknowledgement. Native inventory, correlation and retirement remain unchanged.

A global Payload/Arc redesign would affect unrelated finite/notification APIs;
it is unnecessary for this bounded Action facet because a slice view closes all
its public result escape paths. Holding only a shared fixed reservation fails
the review's repeated-acknowledgement counterexample. A second per-Binding result
store would duplicate the actual Servient owner. Select the bounded result lease
and exact views, with no compatibility facade. The root Consumer's selected
response must additionally qualify every resolved assertion under the concrete
Binding's supported response vocabulary/annotations.
