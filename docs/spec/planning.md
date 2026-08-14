# Planning and Compiled Plan Sets

Status: active v5.0 authority. Only the eight requirement definitions
registered below are active; retained v4.9 deferred clauses are entry-review
input and carry no implementation authority.

This specification is the single normative owner of effective-form planning,
capability indexing, logical-plan construction, binding-compiler coordination,
compiled-plan-set publication, and plan reclamation. It refines
`docs/architecture/30-compiled-plan-lifecycle.md` and ADR-0008. The Protocol
Binding SPI specification owns transport execution, route progress, calls,
subscriptions, responses, and publication progress; this specification only
defines the immutable values that those operations consume.

This specification owns exactly `PLAN-COST-001`, `PLAN-COST-003`,
`PLAN-BOUND-001`, `PLAN-SET-001`, `PLAN-ARTIFACT-001`,
`FORM-FINALIZE-001`, `FORM-OWNER-001`, and `FORM-COVERAGE-001` in v5.0.
The retained clauses for `PLAN-COST-002`, `PLAN-INDEX-001`, `PLAN-LAZY-001`,
`PLAN-CACHE-001`, `PLAN-REQUEST-001`, and `FORM-FINALIZE-002` are explicitly
inactive until a planning-domain entry review re-adopts or replaces them.

## Normative requirements

`PLAN-COST-001`: Planning MUST use a two-level representation in which one
protocol-neutral logical plan is compiled once per effective form and operation
and shared by its binding candidates; a binding plan reference MUST contain
only binding identity, static capability results, an artifact reference, and
binding-specific data that cannot be shared, never a duplicate full logical
plan.

Historical v4.9 clause (`PLAN-COST-002`, inactive): A profile MAY choose eager, lazy, or hybrid compilation only
for eligible binding artifacts; the choice MUST preserve candidate order,
selection and failure semantics, active resource and work bounds, and a visible
distinction between admission-time and first-use compilation failure.

`PLAN-COST-003`: Plan construction MUST reject any document or build result that
exceeds an admitted form, candidate, probe, schema, security, logical-byte,
artifact-byte, compiler-cursor, temporary-byte, or work bound with a structured
limit error; it MUST NOT silently omit or truncate required planning data.

Historical v4.9 clause (`PLAN-INDEX-001`, inactive): Planning MUST build separate generation-bearing client and
server capability indexes from the captured registration snapshot, probe only
the indexed registrations and explicit admitted wildcards for each candidate,
reject declaration/support inconsistencies, and preserve O(`f + p + c`) normal
candidate construction rather than an implicit O(`f * b`) registration scan.

Historical v4.9 clause (`PLAN-LAZY-001`, inactive): Admission MUST eagerly compile all protocol-neutral metadata
required for safe selection and every Producer route/publication artifact; an
eligible Consumer artifact MAY be lazy only through a pre-reserved bounded slot
whose compiler is pure, deterministic, resumable, and unable to start protocol
work.

Historical v4.9 clause (`PLAN-CACHE-001`, inactive): Concurrent first use of one lazy artifact key and complete
dependency generation MUST be single flight, with one compiler lease, bounded
waiters or explicit backpressure, immutable Ready or deterministic Negative
publication, no callback under registry-wide or eviction locks, generation-
isolated reuse, and incremental reference-safe reclamation.

Historical v4.9 clause (`PLAN-REQUEST-001`, inactive): Per-call requests MUST reference immutable static target,
form, URI-template, schema, security, response, extension, and artifact data by
generation-bearing plan slots and MUST own only varying payload, URI-variable,
cancellation, deadline, correlation, committed-security, and protocol-status
data.

`PLAN-BOUND-001`: Every target-operation plan MUST enforce the admitted
`form_binding_candidates_per_operation_max`; fallback MUST examine each retained
candidate at most once and share one `provider_probes_per_interaction_max`
budget across all examined candidates, while strict selection MAY narrow but
MUST NOT bypass either bound.

`PLAN-SET-001`: Each produced or consumed handle generation MUST have one
Servient-owned aggregate compiled-plan-set record that alone owns lifecycle,
publication, pins, operation leases, lazy slots, cursors, and accounting;
publication MUST be atomic, draining MUST reject new pins while preserving old
leases, and incremental reclamation MUST reach zero retained plan/artifact
bytes only after every generation-bearing owner is terminal.

`PLAN-ARTIFACT-001`: Every protocol-specific planning artifact MUST be produced
by the compiler extension atomically associated with the candidate's complete
binding registration, remain immutable and generation checked, fit
pre-admitted final/cursor/temporary/work bounds, be eager for Producer use or
use the bounded Consumer lazy lifecycle, and contain no execution state,
credential, handler, external resource, or cleanup obligation.

`FORM-FINALIZE-001`: A server registration MAY provide one bounded,
deterministic, local, nonblocking, side-effect-free form contributor; planning
MUST invoke it only for indexed matching requirements, charge its work and
output to expose admission, and reject output that cannot locally and fully
describe generated forms, security/context additions, and endpoint reservation
identities without credentials or external-resource creation.

Historical v4.9 clause (`FORM-FINALIZE-002`, inactive): `expose` MUST transactionally validate the draft, collect
and merge contributions in captured order, validate effective forms, resolve
owners and collisions, freeze the effective TD, and eagerly compile all
Producer plans before the first binding side effect; after freeze, no form,
security/context definition, plan id, owner, collision identity, or endpoint
reservation key MAY change.

`FORM-OWNER-001`: Every compiled inbound and publication plan MUST have exactly
one owning `BindingId`; generated forms belong to their contributor, supplied
forms probe only indexed server registrations, zero owners and unresolved
multiple owners are errors, registration order MUST NOT resolve ambiguity, and
endpoint collision identity MUST remain (`CollisionDomainId`,
`EndpointReservationKey`) independently of registration generation.

`FORM-COVERAGE-001`: Every operation advertised by the frozen effective
Producer TD MUST have captured proof of an exact registered handler capability,
an applicable specified aggregation path, or an explicitly selected typed
late-handler policy; strict-at-expose is the default, planning MUST NOT invent
forms merely because handlers exist, and any allowed uncovered operation and
policy MUST remain visible in immutable diagnostics.

## Scope and invariants

Planning starts from a validated TD view or produced-Thing draft plus immutable
policy and registration snapshots. It ends with an admitted immutable plan set.
The following invariants apply to every profile:

- Planning applies the pure TD default rules supplied by `clinkz-wot-td`; it
  does not copy those rules into a binding.
- `base` resolution, operation defaulting, effective security inheritance,
  URI-template compilation, form identity, and candidate ordering happen once
  in shared planning.
- A binding compiler receives one already resolved and already selected
  candidate. It cannot select another form, operation, security expression, or
  binding owner.
- A compiler artifact is immutable planning data. It is not a socket, listener,
  session, task, route guard, subscription driver, request call, or cleanup
  obligation.
- No planning callback performs protocol I/O, invokes an application handler,
  reads credentials, starts an executor task, or acquires an external lease.
- Every plan value is immutable after construction. Aggregate lifecycle state,
  pins, compiler cursors, lazy slots, and reclamation cursors belong to the
  Servient-owned plan-set record.
- No plan is published until all mandatory counts, bytes, work bounds,
  identities, owners, and capacity reservations are complete.
- Every callback executes outside Servient registry locks and constrained
  critical sections.
- `no_std + alloc` planning uses caller-driven bounded progress. Host adapters
  may provide async syntax but cannot change planning outcomes or ownership.

Planning does not own binding execution, application handles, Servient
registries, runtime scheduling, protocol connection caches, handler dispatch,
or Directory service behavior.

## Ownership model

The ownership split is exact:

| Owner | Planning responsibility |
| --- | --- |
| `clinkz-wot-td` | Lossless TD/TM data, validation, and pure W3C default rules |
| `clinkz-wot-core` | Protocol-neutral immutable plan identities and values, binding capability and compiler-extension SPI values, and generation-bearing plan references |
| `clinkz-wot-planning` | Effective-form resolution, capability indexes, logical-plan construction, URI-template compilation, compiler coordination, deterministic ordering, and admitted build output |
| Concrete binding crate | Its capability declaration, compiler-extension implementation, and opaque protocol-specific artifact payload |
| `clinkz-wot-servient` | Build transaction, capacity reservation, plan-set record, publication, pins, lazy-slot state, draining, and reclamation |

An opaque binding artifact remains owned by its plan set for its entire stored
lifetime; binding execution never takes it out of the plan set. For v1
Producer-route preparation, Servient resolves the operation's compact artifact
reference directly inside the frozen plan set, validates the complete plan,
binding, generation, configuration, compatibility, role, and route identity,
and lends the matching envelope only for the initial preparation call. That
borrow cannot enter a pending call, guard, or route slot. The binding instead
derives bounded, route-local owned runtime state before its first protocol side
effect. The Servient's plan-set lease remains private lifecycle ownership and
is not an execution capability. Any other operation that needs retained
artifact access requires its own explicit specification; it is not inferred
from the Producer-route contract.

Artifact destruction releases memory only and cannot fail. It occurs outside
Servient locks. Any value whose destruction would require protocol cleanup is a
runtime binding object and is forbidden as a compiler artifact.

## Plan taxonomy

The taxonomy below is normative. Concrete Rust fields may be split into compact
tables or arenas, but the distinctions and references must remain observable in
diagnostics and conformance tests.

### Source and target identity

A plan source identity names:

- the admitted plan-set generation;
- the Thing root, property, action, event, or collection target context;
- the operation;
- the original form-array identity and form index;
- the resolved target identity; and
- the source document identity or redacted diagnostic identity selected by the
  source-retention policy.

Form indexes are scoped to their containing form array. A Thing-root form index
must never be interpreted as an affordance form index, or conversely. Plan ids
are stable only within one plan-set generation and are never reused to address a
different source identity in that generation.

### Logical interaction plan

`LogicalInteractionPlan` is the protocol-neutral result compiled once per
effective form and operation. It contains or references:

- source and target identity;
- operation and target context;
- a compiled absolute target URI template derived from `base` and `href`;
- effective media, response, subprotocol, and URI-variable rules;
- the structured effective security expression and scopes;
- schema and validation references;
- preserved extension views needed by candidate compilers; and
- immutable diagnostics required to explain selection.

One logical plan is shared by all binding candidates for that effective form and
operation. An implementation must not duplicate its target strings, schemas,
security trees, response metadata, URI-template program, or extension maps for
each binding.

The first Property Read tranche freezes this additive constructible projection:

```rust
impl LogicalInteractionPlan {
    pub fn try_property_read(
        plan_id: PlanId,
        thing_id: ThingId,
        property_name: Box<str>,
        form_index: u32,
        resolved_target: Box<str>,
        content_type: Option<Box<str>>,
        subprotocol: Option<Box<str>>,
    ) -> CoreResult<Self>;

    pub const fn plan_id(&self) -> PlanId;
    pub fn thing_id(&self) -> &ThingId;
    pub const fn operation(&self) -> Operation;
    pub fn property_name(&self) -> &str;
    pub const fn form_index(&self) -> u32;
    pub fn resolved_target(&self) -> &str;
    pub fn content_type(&self) -> Option<&str>;
    pub fn subprotocol(&self) -> Option<&str>;
}
```

`operation()` is exactly `Operation::ReadProperty` for a value returned by
`try_property_read`. The constructor rejects an empty property name or target.
The value owns every string above and contains no TD, form, source envelope, or
borrow from the build input. Its private representation may later add the
already-required security, schema, response, extension, and diagnostic
projections without changing this Property Read constructor.

### Binding candidate

`BindingCandidate` joins one logical plan to one candidate from the captured
registration snapshot. It carries the binding id, binding generation,
configuration identity, registration ordinal, static support result, compiler
compatibility identity, and candidate-order position. It carries no transport
state.

A candidate exists only after the applicable capability index returned the
registration and its side-effect-free support operation accepted the resolved
candidate. A compiler extension cannot manufacture an unindexed candidate.

The Property Read compiler input uses this exact constructible subset:

```rust
impl BindingCandidate {
    pub const fn new(
        binding_id: BindingId,
        binding_generation: BindingGeneration,
        configuration: BindingConfigurationDigest,
        compatibility: BindingArtifactCompatibility,
        registration_ordinal: u32,
        candidate_order: u32,
    ) -> Self;

    pub const fn binding_id(&self) -> BindingId;
    pub const fn binding_generation(&self) -> BindingGeneration;
    pub const fn configuration(&self) -> BindingConfigurationDigest;
    pub const fn compatibility(&self) -> BindingArtifactCompatibility;
    pub const fn registration_ordinal(&self) -> u32;
    pub const fn candidate_order(&self) -> u32;
}
```

Later capability-index work may add private proof material, but cannot permit a
caller to change these captured identities after construction.

### Binding artifact and binding plan reference

A binding artifact is bounded, immutable, protocol-specific data produced by
the candidate's compiler extension. Its envelope binds the payload to:

- one plan-set generation and logical plan id;
- one binding id and binding generation;
- one bounded configuration identity;
- one compiler and artifact compatibility identity; and
- one declared and measured lifetime footprint.

The opaque payload is accessible only through the execution half associated
with the same complete registration. A type, generation, configuration, or
compatibility mismatch is rejected before protocol work starts.

`BindingPlanRef` is the compact protocol-neutral join of a logical-plan slot,
candidate slot, and artifact slot. It does not expose another binding's opaque
payload. An eager reference addresses a ready artifact. A lazy reference
addresses a Servient-owned lazy slot whose capacity and maximum artifact
footprint were admitted before publication.

### Target-operation plan

A target-operation plan contains the ordered, bounded candidate references that
one application operation may examine. Thing-root operations are represented
as Thing-root targets; they are not attached to a synthetic affordance.

`observe_all_properties` and `subscribe_all_events` each use one compatible
Thing-root form, one selected binding plan, and one native or binding-coalesced
subscription. Planning never lowers either standard operation to per-affordance
fan-out. A compatible collection candidate must declare exact source
attribution and an admitted target bound through the typed collection
subscription capability.

The core-owned capability has this exact public schema:

```rust
pub struct CollectionSubscriptionCapability {
    topology: CollectionSubscriptionTopology,
    source_attribution: CollectionSourceAttribution,
    max_targets: NonZeroU32,
    teardown_mode: CollectionTeardownMode,
}

pub enum CollectionSubscriptionTopology {
    NativeMultiplexed,
    BindingCoalesced,
}

pub enum CollectionSourceAttribution {
    ExactAffordanceTarget,
    Unavailable,
}

pub enum CollectionTeardownMode {
    ImplicitDriverStop,
    RootForm,
    ImplicitOrRootForm,
}

impl CollectionSubscriptionCapability {
    pub const fn new(
        topology: CollectionSubscriptionTopology,
        source_attribution: CollectionSourceAttribution,
        max_targets: NonZeroU32,
        teardown_mode: CollectionTeardownMode,
    ) -> Self;
    pub const fn topology(&self) -> CollectionSubscriptionTopology;
    pub const fn source_attribution(&self) -> CollectionSourceAttribution;
    pub const fn max_targets(&self) -> NonZeroU32;
    pub const fn teardown_mode(&self) -> CollectionTeardownMode;
}
```

The fields are private. The capability and its enums implement `Copy`, `Clone`,
`Debug`, `Eq`, `PartialEq`, `Ord`, `PartialOrd`, and `Hash`. Only
`ObserveAllProperties` and `SubscribeAllEvents` are valid capability keys.
Standard collection planning accepts only `ExactAffordanceTarget`, rejects a
target count greater than `max_targets`, and retains topology and teardown mode
in the immutable selected plan. `BindingCoalesced` certifies one physical
subscription equivalent to the selected root operation; it does not authorize
engine-side per-affordance fan-out.

`ExactAffordanceTarget` is the application-visible semantic identity. A
binding artifact may map several protocol-side topics, key expressions,
channels, or route instances to that target, but those protocol identifiers do
not enter `SubscriptionItem`. When physical-source provenance is useful for
diagnostics, it remains bounded binding-local status metadata. If two sources
have observably different WoT semantics, the TD or an explicitly designed
extension must model them as distinct logical targets.

### Producer plan projections

`InboundBindingPlan` is the Producer projection of one logical plan and its
unique server-binding owner. Publication-target projections identify one
selected binding publication target at a time. Route and publication artifacts
are planning artifacts only; route guards, listener state, emission calls, and
remote fan-out remain binding-execution state.

### Compiled plan set

A compiled plan set is the immutable admitted collection for one produced or
consumed handle generation. It contains compact indexes, logical plans,
candidate lists, binding plan references, eager artifacts, lazy-slot
descriptors, accounting ledgers, and immutable diagnostic metadata.

The set does not contain mutable lifecycle fields. The associated
Servient-owned plan-set record contains lifecycle state, pins, active-operation
leases, lazy-slot state, build and reclaim cursors, and publication identity.

## Planning input and context identity

`PlanBuildInput` is the concrete planning-context boundary. It captures or
pins immutable generation-qualified views for one build transaction; it does
not require a deep copy of the startup registration set and does not imply
runtime binding mutation.

One build transaction captures all of the following before candidate planning:

- the validated document or finalized Producer draft identity;
- the source-retention selection;
- the resource profile and planning policy generation;
- the complete startup registration snapshot;
- each binding id, generation, configuration identity, capability declaration,
  compiler compatibility identity, and execution compatibility identity;
- schema, codec, and other immutable dependency generations used by planning;
- deterministic application options that affect form ownership or candidate
  filtering; and
- the Consumer `CandidateFallbackPolicy`; Producer builds require
  `CandidateFallbackPolicy::Disabled`.

Credentials and per-call credential generations are not planning inputs and do
not invalidate an existing plan. Security applicability that depends on current
credentials is evaluated during selection from the precompiled effective
security expression.

V1 registration composition is startup-only. A handle pins the context used by
its build. A different registration or binding configuration requires a new
Servient instance. A new policy, compiler, or schema dependency generation may
affect a later build only through an explicitly captured immutable context; it
does not mutate or invalidate an existing handle in place.

Generation equality is part of every plan/artifact lookup. An entry from an
incompatible document, plan-set, policy, dependency, binding, configuration, or
compiler generation is a miss or stale-reference error, never a reusable cache
hit. Detecting a new generation is O(1) and must not scan or rewrite existing
plan sets.

## Candidate fallback policy and diagnostics

ADR-0017 fixes the only automatic Consumer fallback boundary. Planning owns
these exact public semantic values:

```rust
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CandidateFallbackPolicy {
    Disabled,
    #[default]
    PreExecution,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CandidateSkipReason {
    SecurityInapplicable,
    DeterministicLazyArtifactFailure,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CandidateSkipDiagnostic { /* private fixed-width fields */ }

impl CandidateSkipDiagnostic {
    pub const fn candidate_position(&self) -> u32;
    pub const fn plan_id(&self) -> PlanId;
    pub const fn form_index(&self) -> u32;
    pub const fn binding_id(&self) -> BindingId;
    pub const fn binding_generation(&self) -> BindingGeneration;
    pub const fn reason(&self) -> CandidateSkipReason;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateSelectionDiagnostics { /* private bounded sequence */ }

impl CandidateSelectionDiagnostics {
    pub const fn len(&self) -> usize;
    pub const fn is_empty(&self) -> bool;
    pub fn get(&self, index: usize) -> Option<&CandidateSkipDiagnostic>;
}
```

`PlanBuildInput` captures one immutable policy for the Consumer plan-set
generation. `PreExecution` is the Scripting-compatible and
`GatewayDefaultV1` default. An application may select `Disabled` when building
the consumed handle. A strict per-call form, binding, or security selection
narrows the immutable candidate set and disables automatic fallback for that
call; it cannot widen the plan or override a handle built with `Disabled`.
Producer owner selection never uses Consumer fallback and requires `Disabled`.

Under `PreExecution`, exactly two outcomes may skip one ordered candidate:

1. bounded, side-effect-free security-availability probes establish that the
   candidate's precompiled security expression is currently inapplicable; or
2. the exact lazy-artifact slot generation contains, or completes with, a
   deterministic cacheable compiler negative.

Provider-probe errors or budget exhaustion, cancellation, deadline exhaustion,
backpressure, transient resource failure, non-cacheable compiler failure,
stale or draining generations, security commit failure, request-construction
failure, and `BindingInputRejection<OutboundRequest>` are not eligible. They
terminate that interaction with their structured outcome. After a binding
accepts the request, no result can select another candidate.

Mutable runtime health is diagnostic-only in v1. It never reorders, removes, or
skips an admitted candidate. Stale, unavailable, or draining generations fail
through the generation and lifecycle contracts. A future health-aware policy
requires a separately admitted immutable health generation, bounds, fairness,
diagnostics, and evidence.

Every eligible skip appends exactly one `CandidateSkipDiagnostic` in candidate
order. The sequence is secret-free and fixed-width: it contains only candidate
position, plan id, original form index, binding id/generation, and the typed
reason. Capacity equal to the target-operation plan's admitted candidate count
is reserved before the first probe. Reservation failure therefore precedes all
probe and security-commit work; a completed sequence never truncates a skip and
its length cannot exceed `form_binding_candidates_per_operation_max`.

All candidates share one `provider_probes_per_interaction_max` allowance and
the caller's unique mutable `WorkBudget`. Neither allowance restarts after a
skip. Diagnostics are selection evidence, not free-form logging, and do not
change the chosen candidate or error.

## Capability indexes

Planning builds separate client and server capability indexes from the captured
registration snapshot. The primary key is resolved URI scheme. Declared
secondary keys may include protocol, subprotocol, operation class, media
family, Producer contribution role, and Thing-root collection capability.

Capability declarations are deterministic, side-effect free, and may
over-approximate support. They must not omit a key for which the registration's
support operation can return supported. Returning supported for a key absent
from the declaration is a binding contract violation and the candidate is not
admitted.

For each resolved form, planning probes only registrations returned by the
applicable index. Wildcard capability is an explicit declaration, occupies a
bounded wildcard index, and consumes the wildcard and total probe budgets.
Constrained profiles disable wildcard registrations unless their static
resource profile enables and bounds them.

The index preserves these ordering rules:

1. target context and operation determine the eligible form array;
2. forms retain document order within that array;
3. Consumer candidates for the same form retain captured client-registration
   order after explicit application filters; and
4. strict caller form or binding selection narrows the list without reordering
   it.

Registration order never resolves exclusive Producer ownership. More than one
supporting server registration remains an ambiguity unless an explicit,
typed application binding selection names the owner.

Let `f` be the number of effective form-operation pairs, `p` the number of
indexed support and contributor probes, and `c` the number of retained
candidates. Normal candidate construction is O(`f + p + c`). It must not scan
all `b` registrations for every form. Explicit wildcards may produce the
O(`f * b`) worst case only within the admitted total and wildcard probe limits.

## Shared build algorithm

Planning is incremental and transaction-local. A `PlanBuildInput` denotes the
captured immutable inputs, a build cursor owns resumable intermediate state,
and `PlanBuildOutput` contains the immutable set material plus its exact
accounting ledger. The shared `PlanCompiler` coordinates these stages:

1. validate the applicable document view and establish bounded source indexes;
2. enumerate target contexts and effective operations in deterministic order;
3. apply TD defaults and resolve `base`, `href`, media, response, URI-variable,
   security, scope, schema, and extension views;
4. query the applicable capability index and side-effect-free support methods;
5. build each shared logical plan once and retain ordered candidates;
6. obtain compiler bounds and compile every mandatory eager artifact;
7. construct pre-reserved lazy descriptors for permitted Consumer artifacts;
8. validate all plan, candidate, artifact, ownership, count, byte, and work
   bounds; and
9. return the immutable material and reservations needed to freeze the set.

Budget exhaustion preserves the build cursor and returns pending progress. It
does not restart an earlier stage, publish a partial plan, or translate budget
exhaustion into an unsupported-form result. With zero available work units, a
step does not invoke a contributor, support method, or compiler extension and
does not advance externally observable state.

Any failure before freeze is an admission failure. Provisional artifacts and
temporary buffers are released outside locks, all reservations are rolled back,
and no handle, route, or binding side effect becomes visible.

## Binding compiler extension contract

The compiler extension and artifact obligation above is implemented through
one exact portable Rust contract owned by Core.

The fixed-width identities and passive accounting values are:

```rust
#[repr(transparent)]
pub struct BindingConfigurationDigest([u8; 32]);

#[repr(transparent)]
pub struct BindingArtifactCompatibility([u8; 16]);

pub enum BindingArtifactRole {
    ConsumerCall,
    ConsumerSubscription,
    ProducerRoute,
    ProducerPublication,
}

pub struct BindingArtifactFootprint {
    retained_items: u32,
    retained_bytes: u64,
}

pub struct BindingCompilerBounds {
    artifact: BindingArtifactFootprint,
    cursor_bytes: u64,
    temporary_bytes: u64,
    work: WorkBudget,
}

pub struct BindingArtifactIdentity {
    plan_set_generation: PlanSetGeneration,
    plan_id: PlanId,
    binding_id: BindingId,
    binding_generation: BindingGeneration,
    configuration: BindingConfigurationDigest,
    compatibility: BindingArtifactCompatibility,
    role: BindingArtifactRole,
}
```

The digest and compatibility wrappers provide `const new` and `as_bytes`
operations. `BindingArtifactFootprint` provides `const new`, field accessors,
and `fits_within`. `BindingCompilerBounds` provides `new`, immutable accessors,
and `into_work`. `BindingArtifactIdentity` provides `const new` plus one
accessor per field. `PlanSetGeneration` is a non-wrapping semantic wrapper over
Foundation `Generation`; it is not interchangeable with a binding generation.

The compiler input and ownership-bearing outcomes are exactly:

```rust
pub struct BindingCompilerInput<'a> {
    logical_plan: &'a LogicalInteractionPlan,
    candidate: BindingCandidate,
    role: BindingArtifactRole,
}

pub struct BindingArtifact<A> {
    compatibility: BindingArtifactCompatibility,
    footprint: BindingArtifactFootprint,
    route_reservation: Option<RouteReservationIdentity>,
    payload: A,
}

pub struct BindingCompilerOutput<A> {
    artifact: BindingArtifact<A>,
}

pub struct BindingCompilerFailure<C> {
    error: CoreError,
    cursor: C,
}

#[must_use]
pub enum BindingCompilerStep<C, A> {
    Pending(C),
    Complete(BindingCompilerOutput<A>),
    Failed(BindingCompilerFailure<C>),
}

pub trait BindingCompilerExtension {
    type Cursor;
    type Artifact;

    fn compatibility(&self) -> BindingArtifactCompatibility;
    fn bounds(
        &self,
        input: &BindingCompilerInput<'_>,
    ) -> CoreResult<BindingCompilerBounds>;
    fn start(
        &self,
        input: &BindingCompilerInput<'_>,
    ) -> CoreResult<Self::Cursor>;
    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact>;
    fn abort(&self, cursor: Self::Cursor);
}
```

`BindingCompilerInput::new` and its accessors expose exactly the three fields
shown. `BindingArtifact::new` creates an artifact without route metadata.
`BindingArtifact::producer_route` requires one complete
`RouteReservationIdentity`; `route_reservation` exposes it read-only.
The existing three-part `into_parts` surface remains unchanged;
`into_route_parts` returns compatibility, footprint, optional route
reservation, and payload for callers such as host erasure that must preserve
the added structural metadata. `compatibility`, `footprint`, `payload`, and
`into_payload` retain their existing meanings. The output and failure wrappers
continue to preserve their owned cursor or artifact on every resumable or
error branch; `into_route_parts` is the complete consuming path when route
metadata must survive payload conversion.

The route reservation is fixed-width Core wrapper metadata, not an external
lease or binding-authored payload allocation. It is accounted with the
compiled plan set's structural metadata; `BindingArtifactFootprint` continues
to measure the binding-authored retained payload. Only a concrete binding
compiler may derive the canonical collision domain and endpoint key from the
resolved logical plan and immutable local binding configuration. Planning and
Servient must not hash or otherwise reinterpret the target URI.

`start` creates only a pure caller-owned cursor; externally chargeable progress
occurs in `step`. `step` consumes the cursor and returns it in both `Pending`
and `Failed`. `abort` consumes pure in-memory state and creates no cleanup
obligation. The bound's work budget has a nonzero allowance only for
`WorkClass::BindingPolls`, and compiler progress charges only that class. With
no remaining `BindingPolls` unit, `step` returns `Pending(cursor)` without
changing it or any externally observable state.

Artifact admission is exact and ownership-preserving:

```rust
pub enum BindingArtifactRejectionReason {
    CompatibilityMismatch,
    FootprintExceeded,
    MissingRouteReservation,
    UnexpectedRouteReservation,
}

pub struct BindingArtifactRejection<A> {
    reason: BindingArtifactRejectionReason,
    artifact: BindingArtifact<A>,
}

impl<A> BindingArtifactEnvelope<A> {
    pub fn try_new(
        identity: BindingArtifactIdentity,
        admitted: BindingArtifactFootprint,
        artifact: BindingArtifact<A>,
    ) -> Result<Self, BindingArtifactRejection<A>>;

    pub const fn identity(&self) -> BindingArtifactIdentity;
    pub const fn admitted(&self) -> BindingArtifactFootprint;
    pub const fn route_reservation(&self) -> Option<RouteReservationIdentity>;
    pub fn artifact(&self) -> &BindingArtifact<A>;
    pub fn into_artifact(self) -> BindingArtifact<A>;
}

impl BindingArtifactRef {
    pub const fn new(
        identity: BindingArtifactIdentity,
        artifact_slot: SlotIndex,
    ) -> Self;

    pub const fn identity(&self) -> BindingArtifactIdentity;
    pub const fn artifact_slot(&self) -> SlotIndex;
}
```

Envelope construction compares the artifact compatibility with the complete
identity, compares the measured footprint with the admitted bound, requires
route metadata for `ProducerRoute`, and rejects route metadata for every other
role. A rejection returns the original typed artifact. The compact reference
is scoped by the identity's plan-set generation and can address only its
immutable artifact slot. A plan-set owner obtains the reservation through the
matched envelope; it does not inspect a static or erased protocol payload.

### Static compiler representation

The portable trait above is also the static representation. A constrained
third-party binding implements it with concrete cursor and artifact types. An
application that registers heterogeneous bindings defines one closed compiler
enum and matching closed cursor/artifact enums, then implements the trait by
variant dispatch:

```rust
pub struct StaticBindingCompilerRegistration<C> {
    compiler: C,
}

impl<C> StaticBindingCompilerRegistration<C> {
    pub const fn new(compiler: C) -> Self;
    pub const fn compiler(&self) -> &C;
    pub fn into_compiler(self) -> C;
}
```

Every entry in one application-static table therefore has the same application
enum type `C`, while each third-party compiler remains independently authored.
The representation requires no trait object, `Any`, `Arc`, allocation-backed
erasure, atomics, executor, or unsafe binding-authored cast. The artifact stays
typed through `BindingArtifactEnvelope<AppArtifact>`.

### Host compiler representation

Under `std`, Core alone provides safe cursor and artifact erasure:

```rust
pub struct HostBindingCompilerCursor { /* Core-private erasure */ }
pub struct HostBindingArtifact { /* Core-private erasure */ }
pub struct HostBindingCompilerRegistration { /* Core-private erasure */ }

impl HostBindingCompilerRegistration {
    pub fn new<C>(compiler: C) -> Self
    where
        C: BindingCompilerExtension + Send + Sync + 'static,
        C::Cursor: Send + 'static,
        C::Artifact: Send + Sync + 'static;

    pub fn compatibility(&self) -> BindingArtifactCompatibility;
    pub fn bounds(
        &self,
        input: &BindingCompilerInput<'_>,
    ) -> CoreResult<BindingCompilerBounds>;
    pub fn start(
        &self,
        input: &BindingCompilerInput<'_>,
    ) -> CoreResult<HostBindingCompilerCursor>;
    pub fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: HostBindingCompilerCursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<HostBindingCompilerCursor, HostBindingArtifact>;
    pub fn abort(
        &self,
        cursor: HostBindingCompilerCursor,
    ) -> Result<(), HostBindingCompilerCursor>;
}

impl BindingArtifact<HostBindingArtifact> {
    pub fn try_payload<T>(
        &self,
        expected: BindingArtifactCompatibility,
    ) -> Option<&T>
    where
        T: Send + Sync + 'static;

    pub fn try_into_payload<T>(
        self,
        expected: BindingArtifactCompatibility,
    ) -> Result<T, Self>
    where
        T: Send + Sync + 'static;
}
```

Core implements erasure with safe standard-library type inspection. A cursor
type mismatch makes `step` return `Failed` with the original erased cursor;
`abort` returns `Err(original_cursor)`. Compatibility or payload-type mismatch
returns `None` for borrowed access and `Err(original_artifact)` for consuming
access. Binding crates neither implement nor invoke an unsafe erasure hook.

### Complete-registration relationship

`HostBindingCompilerRegistration` and
`StaticBindingCompilerRegistration<C>` are constructible component values, not
Servient installation units. WP-200 implements them and the contract above.
WP-300 later consumes exactly one matching component into each complete
`HostBindingRegistration` or `StaticBindingRegistration<B>`. Only that complete
bundle can enter `ServientBuilder`, preserving atomic identity, capability,
compiler, artifact, and execution validation without making WP-200 wait for
the WP-300 execution API.

Compiler input is a read-only view containing only:

- the already resolved logical plan and source identity;
- the candidate binding, generation, configuration, and capability proof;
- the artifact role, such as Consumer call, Consumer subscription, Producer
  route, or Producer publication;
- preserved binding-relevant extension members.

It never contains credentials, application handlers, a Servient handle, a
socket, an executor, mutable TD storage, unrelated forms, or authority to change
the candidate.

For a fixed input snapshot and budget-independent completion, the extension
must produce equivalent artifact bytes or values, compatibility identity,
route reservation when the role is `ProducerRoute`, footprint, and diagnostic
outcome. It cannot use wall-clock time, random input, ambient mutable process
state, network state, or discovery state. Different step sizes must produce the
same completed artifact and failure classification.

The measured lifetime footprint must not exceed the pre-admitted bound. A bound
violation is a binding contract error; the artifact is rejected before any
execution operation receives it. Compiler failures identify their stage and
plan/candidate source without exposing credentials or secret configuration.

Compiler work does not include protocol connection/session establishment.
Protocol-native caches are runtime binding resources keyed by admitted plan and
binding generations and remain outside the planning cache.

## Constructible Property Read build boundary

The first WP-200 slice uses one generic registration-snapshot parameter so the
same Planning contract accepts a host slice or one application-static compiler
table:

```rust
pub struct PlanBuildInput<'a, R: ?Sized> {
    validated_td: &'a Thing,
    registrations: &'a R,
    plan_set_generation: PlanSetGeneration,
}

impl<'a, R: ?Sized> PlanBuildInput<'a, R> {
    pub const fn new(
        validated_td: &'a Thing,
        registrations: &'a R,
        plan_set_generation: PlanSetGeneration,
    ) -> Self;

    pub const fn validated_td(&self) -> &'a Thing;
    pub const fn registrations(&self) -> &'a R;
    pub const fn plan_set_generation(&self) -> PlanSetGeneration;
}

pub struct PlanBuildOutput<A> {
    /* owned logical plans, envelopes, and references */
}

pub struct PlanBuildFailure<C> {
    error: CoreError,
    cursor: C,
}

#[must_use]
pub enum PlanBuildStep<C, A> {
    Pending(C),
    Complete(PlanBuildOutput<A>),
    Failed(PlanBuildFailure<C>),
}

pub trait PlanCompiler<R: ?Sized> {
    type Cursor;
    type Artifact;

    fn start(
        &self,
        input: &PlanBuildInput<'_, R>,
    ) -> CoreResult<Self::Cursor>;
    fn step(
        &self,
        input: &PlanBuildInput<'_, R>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> PlanBuildStep<Self::Cursor, Self::Artifact>;
    fn abort(&self, cursor: Self::Cursor);
}
```

`PlanBuildOutput<A>` owns a bounded collection of
`LogicalInteractionPlan`, `BindingArtifactEnvelope<A>`, and
`BindingArtifactRef` values and provides slice accessors plus consuming
`into_parts`. It has no lifetime parameter and cannot contain a TD, form,
source envelope, registration, compiler cursor, handler, credential, Servient,
or execution trait object.

The Property Read completion fixture builds from a borrowed validated TD,
drops that TD and every registration/compiler input, and then uses only the
owned logical plan, envelope, reference, and typed or safely erased artifact
payload. That is the executable `DOC-RUNTIME-001` boundary for this tranche;
it does not claim the later full plan-set lifecycle or architecture gate.

## Producer form finalization

Producer finalization is part of the expose admission transaction and occurs
before logical plans are frozen.

### Contributor contract

A server registration may supply one deterministic `ServerFormContributor`
and a bounded `FormContributionCapability`. The contributor receives only the
binding-independent draft view, matching bounded affordance requirements, and
an immutable contribution context. It returns generated forms associated with
their exact targets, generated security definitions and JSON-LD context terms,
and canonical endpoint reservation identities for the forms it generates.
Application-supplied forms instead obtain their canonical reservation identity
from the selected Producer-route compiler output. If a generated form is also
compiled, the compiler result must equal the contributor's identity before the
plan set can freeze.

The contributor is local, nonblocking, deterministic, and side-effect free. It
must not open a listener, contact a peer, reserve an external lease, spawn work,
or wait for executor progress. It may inspect immutable local endpoint
configuration already captured by the registration. Contribution work and
returned bytes consume the expose admission budgets.

Generated forms include operation, target, media, subprotocol, security, scope,
and preserved extension metadata but never credentials or secret material. A
contributor that cannot determine all required metadata locally returns a
structured finalization error. Later route readiness cannot discover or rewrite
the effective TD.

### Finalization order

`expose` finalizes the effective contract in this exact order:

1. validate and index the binding-independent draft;
2. invoke matching contributors in captured registration order;
3. merge contributions transactionally, including deterministic Clinkz JSON-LD
   prefix allocation and duplicate security-definition checks;
4. validate every supplied and generated form and resolve one server owner;
5. compile every inbound and publication plan and artifact eagerly, obtaining
   the canonical reservation identity for supplied Producer routes and
   checking generated-route compiler identity against contributor output;
6. reject missing ownership, ambiguous ownership, endpoint collisions, invalid
   effective security, route-metadata mismatch, and resource-limit violations,
   then freeze the effective TD and unpublished plan set; and
7. hand the frozen unpublished Producer plan set to Servient route admission.

After step 6, no form, security definition, context term, plan id, binding
owner, collision identity, or endpoint reservation key can change. Binding
prepare, readiness, activate, commit, shutdown, and cleanup consume the frozen
plans but are not planning stages.

### Owner and collision rules

Every inbound and publication plan has exactly one owning `BindingId`.
Generated forms are owned by their contributor. Application-supplied forms are
probed only against server registrations returned by the server capability
index, including admitted wildcards.

Zero supporting registrations is a binding-selection error. Multiple
supporting registrations are an ambiguous-owner error unless an explicit typed
application option selected one binding id. Registration order is not an owner
selection policy, and a selected binding that does not support the form fails
closed.

Endpoint collision identity is the pair (`CollisionDomainId`,
`EndpointReservationKey`) and is independent of registration generation.
Prepared, active, draining, and cleanup-pending reservations conflict in the
same domain across generations. A key may be reused only after the old owner is
closed or through a separately specified typed atomic-handoff contract.
Generation numbers distinguish owners and cleanup attempts; they do not make a
physical endpoint distinct.

Form contribution reserves no external endpoint. The finalization transaction
may reserve only local accounting entries. Servient reserves runtime route and
cleanup capacity after freeze and before asking a binding to create external
state.

### Advertised behavior coverage

The frozen effective TD is authoritative about advertised Producer behavior. A
registered handler does not require planning to invent or advertise a
corresponding operation. Conversely, every operation advertised by an
application-supplied or generated form must have one of these coverage proofs
in the captured expose input:

- a registered handler capability for the exact target and operation;
- a separately specified Scripting-compatible aggregation path applicable to
  that exact operation; or
- an explicit typed late-handler policy selected by the application.

Planning receives only immutable behavior-presence and aggregation-capability
metadata; it never receives or invokes handler objects. The portable and host
default is strict-at-expose: missing coverage rejects finalization before the
plan set freezes. A Rust-native `AllowLateHandlers` policy MAY admit the plan,
but the policy and uncovered target-operation identities MUST be retained in
the immutable diagnostics. Runtime dispatch before handler publication returns
the separately specified structured `UnsupportedOperation` outcome; this
specification does not redefine handler lookup or dispatch.

Every Producer route implied by every form retained in the frozen effective TD
is required for that produced generation. Planning and expose admission do not
label advertised routes optional, redundant, or alternative at runtime. If any
required route cannot be reserved, prepared, made ready, or committed closed,
the generation is not published and all prepared state follows rollback.
Applications that want to omit an unavailable form must construct and admit a
new effective TD and plan-set generation without that form. V1 has no
late-joining route. A future availability group or redundant-route policy
requires a versioned TD/plan contract plus explicit lifecycle, resource, and
evidence rules.

This all-advertised-route conjunction is a conservative v1 product policy, not
a logical consequence of atomic publication. Atomicity requires that whichever
effective TD and route set is selected be published as one truthful immutable
generation. The engine does not derive a reduced TD after route failure.
Applications or deployment platforms own any replacement source/effective TD,
new generation, attempt/backoff bound, signing and Directory update, and later
restoration of the full route set after the failed generation's conflicting
endpoint and cleanup ownership is settled.

## Consumer plan construction and selection

Consumer admission eagerly compiles all protocol-neutral metadata needed for
safe lookup and selection. It may leave only eligible heavy, pure binding
artifacts lazy. The consume transaction freezes and atomically publishes its
plan set before returning the consumed handle. No transport operation starts
during planning.

An interaction hot path:

1. looks up one target-operation plan through its compact index;
2. applies strict caller filters without reordering candidates;
3. examines current security applicability within the shared provider-probe
   budget;
4. obtains the eager artifact or resolves its admitted lazy slot;
5. commits one selected plan identity and per-call security material; and
6. constructs an `OutboundRequest` for the selected binding execution SPI.

It does not traverse the TD, resolve source URI text, redo defaulting or
security inheritance, query an unindexed registration, or compile an unbounded
artifact.

Each candidate is examined at most once per interaction. The first applicable
candidate is selected unless a strict option requires a particular form or
binding. `CandidateFallbackPolicy::PreExecution` may continue only for the two
eligible planning-owned outcomes defined above and records every skip in the
pre-reserved diagnostics. `Disabled` and every strict selection return the
first failure. Once security commit or binding input construction fails, or a
binding execution operation accepts or rejects a request, that interaction
never triggers implicit fallback that could repeat a side effect.

An explicit retry is a new interaction. The caller may combine the returned
`RetryClass` with its own idempotency and security policy and select a strict
form or binding for the new call. That call receives a fresh deadline,
`WorkBudget`, provider-probe allowance, and security commit; no prior
side-effect assumption is carried forward. Diagnostics identify the failed
selection/execution phase and why another admitted form was not attempted.
Multiple forms are ordered candidates, not a runtime failover pool.

`RetryClass` is advice about one terminal error; by itself it is not a complete
availability action. Before a broad Consumer or Gateway facade claims reusable
retry or failover behavior, its domain-entry contract must freeze bounded
values for:

- execution certainty (proved no protocol side effect, result known, or result
  possibly committed/unknown);
- permitted next action (same candidate, another admitted candidate, rebuild
  the consumed handle/generation, caller decision, or no retry);
- operation/idempotency proof required for that action;
- strict plan-set, candidate, form, binding, and generation identity;
- one product-operation correlation identity over fresh per-attempt
  correlation ids;
- overall attempt, elapsed-time, work, provider-probe, and retained-diagnostic
  bounds; and
- cancellation and a bounded final attempt history.

Until that facade and its negative evidence exist, the v1 engine offers
structured failure and low-level strict selection, not automatic failover.
No classification may authorize another candidate after acceptance or when
execution certainty is unknown.

Selection errors remain distinct from execution errors. They identify missing
targets or operations, absent compatible forms, target-resolution failures,
unsupported bindings, unavailable security applicability, strict-option
mismatch, and eager or first-use compilation failure. Execution errors retain
the selected plan identity.

## Static request data

`PLAN-REQUEST-001` requires per-call requests to reference immutable static
metadata by generation-bearing plan, target, affordance, candidate, and
artifact slots. A request owns only varying data, including payload,
URI-variable values, cancellation/deadline state, correlation or idempotency
values, committed security material, and protocol status.

A request must not clone static target strings, URI-template programs, schemas,
security expressions, response metadata, or extension maps. Host erased calls
may retain a shared plan-set lease. Constrained calls use a generation-bearing
slot reference whose owner retains the same plan-set lease. Neither
representation gives a binding access to the full TD or another candidate.

## Eager and lazy artifact policy

The profile and complete registration determine which eligible Consumer
artifact roles are eager or lazy. The policy is immutable for one plan-set
generation and appears in diagnostics.

The following are always eager:

- all logical-plan metadata needed for lookup, security applicability, request
  construction, validation, and diagnostics;
- every Producer route and publication artifact;
- any artifact whose construction is not pure, deterministic, bounded, and
  side-effect free; and
- any artifact whose final and compiler-cursor footprint cannot be
  conservatively reserved before publication.

A Consumer artifact may be lazy only when its slot metadata, maximum artifact
bytes, maximum compiler-cursor bytes, negative-entry bytes, and waiter capacity
are all admitted at freeze. Lazy compilation starts before binding execution,
never starts protocol work, and cannot make a published set exceed its admitted
lifetime footprint.

A constrained static Producer may require all-eager compilation. Planning must
not eagerly compile otherwise eligible unused Consumer form-binding
combinations solely to simplify an API.

## Lazy artifact state and single flight

Lazy state belongs to the Servient plan-set record, not the immutable plan.
One slot follows this state model:

```text
Empty -> Compiling -> Ready
                    -> Negative
Compiling ---------> Empty       (non-cacheable abort or failure)
Ready | Negative -> Reclaiming -> Empty
```

- `Empty` owns admitted capacity but no artifact or compiler cursor.
- `Compiling` owns the unique compiler lease, resumable cursor, reservation,
  and bounded notification state.
- `Ready` owns one immutable artifact and its measured footprint.
- `Negative` owns one bounded, redacted deterministic failure.
- `Reclaiming` prevents new attachment while a published result or diagnostic
  is released incrementally.

At most one compiler lease exists for a slot generation. Concurrent users
attach through bounded waiter/notification capacity and observe the same Ready
artifact, the same Negative result, or explicit backpressure. Compilation and
notifications run outside registry-wide and eviction locks.

Budget exhaustion retains the same cursor. A partial artifact is never
published. A deterministic incompatibility for the fixed input and dependency
generations may enter `Negative`. Cancellation, deadline dependence, transient
resource exhaustion, and internal retryable failure are not retained as
Negative; after releasing unpublished state they leave the slot Empty.

Caller cancellation releases only that caller's waiter. The slot owns an
accepted compile lease, so another waiter can still receive the result. Drain or
eviction may abort pure unpublished compiler work and return the slot to Empty
after releasing its cursor; it never creates a protocol cleanup obligation.

Eviction cannot reclaim a referenced Ready result. Reuse after eviction creates
at most one new compiler lease for the current generation. A generation
mismatch never attaches to an old slot. Stale slots are reclaimed incrementally
under explicit item, byte, and work budgets; no publication thread scans all
Things or plan sets.

## Compiled-plan-set lifecycle

The aggregate plan-set obligation above uses exactly the lifecycle below.

The Servient plan-set record has exactly these public lifecycle states:

```text
Building -> Frozen -> Published -> Draining -> Reclaimed
    |          |
    +-> Failed +-> Failed
```

The allowed transitions are:

| Current | Event | Next | Contract |
| --- | --- | --- | --- |
| `Building` | all mandatory build work and reservations complete | `Frozen` | Immutable set material and full accounting are installed; no plan is selectable |
| `Building` | admission failure or cancellation | `Failed` | Nothing was published and provisional storage is rolled back |
| `Frozen` | Consumer registry publication | `Published` | One consumed-handle generation becomes atomically selectable |
| `Frozen` | Producer serving commit | `Published` | Plan publication and serving-registry publication are one atomic Servient transition |
| `Frozen` | expose route failure or cancellation | `Failed` | No serving entry is published; set storage remains pinned only until route rollback owners are terminal |
| `Published` | handle/Thing drain begins | `Draining` | New selection is rejected; existing leases retain immutable plans |
| `Draining` | all pins, operations, routes, lazy work, and cleanup owners are terminal and reclaim work completes | `Reclaimed` | The generation retains no plan or artifact bytes |

No other transition is valid. `Failed` is an unpublished admission terminal;
its bounded diagnostic may be retained independently, but its plan material is
released after any expose rollback owners finish. `Reclaimed` is terminal.

Pin count and active-operation count are orthogonal retained fields, not public
states. A pin can be acquired only from Published. Starting Draining closes the
pin-acquisition gate before the state becomes observable. A lease acquired
before that linearization remains valid until released.

### Consumer timing

`consume` performs `Building -> Frozen -> Published` without transport side
effects. Returning a consumed handle proves that its plan set is Published. A
failed consume publishes neither a handle nor a partial registry entry.

### Producer timing

`expose` completes `Building -> Frozen` before the first route side effect.
Servient route preparation, readiness, activation, and commit bind that frozen
set to generation-bearing route records. Only the successful serving-registry
commit transitions the set to Published. A route failure transitions the
unpublished set to Failed and invokes the separately specified route rollback;
it never mutates an individual plan.

Inbound route matching and Producer emission selection pin the Published set.
Draining closes route ingress through the Servient route lifecycle and retains
plans until every accepted request, response, emission, route, and cleanup
owner that references them is terminal.

## Resource and work contract

All profiles enforce the exact resource fields registered in
`docs/resource-limits.csv`. For planning, the following existing fields are
mandatory controls rather than advisory telemetry:

- `document_bytes_max`, `forms_per_context_max`, `forms_per_thing_max`,
  `schema_nodes_per_document_max`, `security_expression_depth_max`, and
  `security_branches_per_plan_max`;
- `binding_and_contributor_probes_per_admission_max` and
  `wildcard_binding_and_contributor_probes_per_admission_max`;
- `form_binding_candidates_per_operation_max`;
- `compiled_plan_bytes_max`, `compiled_runtime_bytes_per_thing_max`, and
  `compiled_runtime_bytes_global_max`;
- `lazy_plan_slots_per_thing_max` and `lazy_plan_slots_global_max`; and
- the registered cache entry, generation, byte, and reclamation-step limits.

Zero never means unbounded. Unsupported capability cells are disabled rather
than represented by an unlimited value.

Each build output has an exact `PlanFootprint` ledger, whether represented by
one Rust struct or equivalent private records. It separately accounts for
logical-plan bytes, candidate/index bytes, eager-artifact lifetime bytes,
reserved lazy-artifact bytes, lazy metadata and negative-result capacity,
compiler-cursor capacity, retained source/extension bytes, and reclamation
metadata. The sum is checked against per-Thing and global reservations without
double-counting shared immutable storage.

Every compiler extension reports conservative final, cursor, temporary, and
work bounds before compilation. Temporary memory is charged while live but is
not misreported as lifetime storage. Actual final and cursor footprints are
checked against both the extension bound and the transaction reservation.

Plan construction rejects one-over-limit input with a structured limit error.
It never silently omits a form, candidate, security branch, schema node,
extension, or artifact to fit. In particular, an operation retains no more than
`form_binding_candidates_per_operation_max` ordered candidates. The gateway
default remains 32; constrained profiles state an explicit static value.

All candidate fallback for one interaction shares one
`provider_probes_per_interaction_max` budget. The budget is not reset for each
candidate. Strict selection can reduce work but cannot bypass admission or
provider-probe limits.

Planning traversal, each indexed support or contributor call, URI-template and
security/schema compilation, binding compiler progress, and reclamation consume
explicit `WorkBudget`. Maximum-document builds are resumable. Work charging is
monotonic, and changing a step's budget size cannot change the final plan,
candidate order, artifact, or error classification.

## Complexity contract

For an admitted document:

- source validation and logical compilation are linear in the visited bounded
  document, schema, security, URI-template, and extension structures;
- capability lookup and candidate construction are O(`f + p + c`) outside the
  explicit wildcard worst case;
- logical memory is O(`l`), not O(`l * c`), where `l` is shared logical-plan
  data and `c` is the binding-candidate count;
- artifact memory is the sum of admitted per-candidate artifact footprints;
- hot target-operation lookup uses a prebuilt index and examines at most the
  admitted candidates for that operation;
- strict form or binding lookup does not scan unrelated targets or all
  registrations; and
- reclamation is incremental and bounded per step, not one stop-the-world scan
  proportional to all handles.

Hash tables are not required. Static sorted tables, perfect indexes, arenas, or
bounded maps are conforming if they preserve the same bounds, ordering,
generation checks, and worst-case evidence.

## Publication and consumer contracts

The planning build output is transferred exactly once into a Servient plan-set
record. Publication is a single registry transition; readers observe either no
generation or the complete immutable generation. They never observe Building,
partially populated indexes, provisional artifacts, or a mixture of old and
new generation fields.

Consumers of plan data are restricted as follows:

- application handles receive selection and diagnostic views, not mutable plan
  storage;
- Servient selection receives target-operation indexes and candidate refs;
- a binding compiler receives one resolved candidate input;
- a binding execution operation receives one checked binding plan/artifact ref;
- Producer route admission receives only plans owned by that registration; and
- emission coordination receives immutable selected publication targets, not a
  capability to rediscover bindings.

No consumer may retain a raw reference without a plan-set pin or generation-
checked slot lease. No consumer may write through a plan reference.

## Error classification

Planning errors are structured by stage and retain bounded source identity.
They distinguish at least:

- invalid or over-limit document input;
- target, URI-template, media, schema, or security resolution failure;
- no indexed or supporting binding;
- ambiguous Producer owner or endpoint collision;
- contributor or compiler contract violation;
- eager compiler failure;
- first-use lazy compiler failure;
- stale plan, binding, configuration, or artifact generation;
- candidate or provider-probe limit exhaustion; and
- cancelled or deadline-exhausted build progress.

A redacted deterministic compiler failure may be cached only in the matching
lazy slot generation. Transport, authentication-attempt, cancellation,
deadline, and runtime resource failures are execution outcomes and are never
turned into permanent planning negatives.

`BindingInputRejection<OutboundRequest>` is also not a planning negative. It
returns the exact post-security-commit request before binding acceptance, but it
does not prove that choosing and committing another candidate would be
side-effect free.

## Property Read Producer-route projection

The package-local Property Read plan slice initially completed a private
Consumer-call algorithm. The completed narrow WP-300 registration advertises
Producer Property Read and exposes its compiler only as the immutable
projection of one complete registration. Before WP-400 may prepare a route,
Planning therefore provides one exact public Producer-route projection.

`PropertyReadPlanCompiler::producer_route` consumes a `PlanId`, one complete
`BindingRegistrationIdentity`, a registration index, and a deterministic
candidate order. It derives binding id, binding generation, configuration
digest, and compatibility from that identity and fixes every compiler input,
artifact envelope, and compact artifact reference to
`BindingArtifactRole::ProducerRoute`. No public constructor accepts an
arbitrary role, and the existing Consumer-call constructor remains
crate-private.

The existing `PlanCompiler` contract determines the concrete cursor type.
`PropertyReadBuildCursor<C, A>` is therefore public but opaque: external code
may own, move, and resume it, while only Planning may construct or inspect its
state. Static and host callers borrow the compiler projection from a complete
registration; they neither move the compiler out of that bundle nor build a
second installable compiler half.

The correction does not add another selection algorithm. It reuses the same
validated-TD read, immutable logical plan, bounded compiler progress, admitted
artifact envelope, and TD-lifetime-free output. Completion evidence joins the
real output to the real WP-300 boundary:

```text
validated TD + complete Producer registration projection
  -> PropertyReadPlanCompiler::producer_route
  -> BindingArtifactRef(role = ProducerRoute)
  -> PrepareInput
  -> Servient-checked BindingArtifactEnvelope borrow
  -> PollServerBinding::start_prepare
```

The evidence must preserve identity and role across that chain, prove the
compiler-produced nontrivial payload reaches matching preparation, prove zero
budget makes no progress, and end all TD and compiler-projection borrows before
route preparation. The envelope borrow ends when initial preparation returns;
pending preparation and every later route phase use only binding-owned bounded
runtime state. A fixture-created logical plan, artifact, or preparation
substitute is not evidence for this contract.

## Property Read route-reservation projection

The Producer-route role/reference projection alone does not supply the
canonical collision identity needed by WP-400 to construct `BindingRouteKey`.
Its package-local fixture may construct the real `PrepareInput` type with a
fixed mock reservation, but that value is not a production output and therefore
does not release successor source or the cross-package architecture fixture.

`WP-200-PROPERTY-READ-ROUTE-RESERVATION-PROJECTION` closes that distinct
boundary. A `ProducerRoute` compiler completes with
`BindingArtifact::producer_route`; Core preserves its
`RouteReservationIdentity` through typed and host-erased payload paths and
artifact admission. Missing metadata and metadata attached to a non-route role
fail without losing the artifact. Planning retains the admitted envelope and
the future Servient obtains the identity from the artifact slot paired with its
real `BindingArtifactRef`.

The correction does not implement form contribution, capability indexing, a
collision table, Servient capacity reservation, route lifecycle, or protocol
I/O. It supplies the previously missing immutable value at the existing legal
compiler/artifact boundary.

## API roles

The API ownership matrix freezes public paths and exact Rust schemas. It must
project these roles without moving their behavior to another crate:

| Role | Defining owner | Required purpose |
| --- | --- | --- |
| `LogicalInteractionPlan`, `BindingCandidate`, `BindingPlanRef`, `InboundBindingPlan` | `clinkz-wot-core` | Immutable protocol-neutral plan and execution-reference values |
| Semantic plan, source, target, binding, artifact, and plan-set generation ids | `clinkz-wot-core` | Static distinctions and stale-reference rejection; they may contain the foundation-owned generic `Generation` primitive without exposing it as an interchangeable semantic id |
| Binding compiler input, artifact envelope/ref, compatibility identity, footprint, outcome, and compiler-extension trait | `clinkz-wot-core` binding SPI | Portable contract between shared planning and one complete registration |
| `CapabilityIndex`, `PlanCompiler`, `PlanBuildInput`, `PlanBuildOutput`, build cursor, `CompiledUriTemplate`, and `ResolvedFormTarget` | `clinkz-wot-planning` | Shared deterministic planning algorithms and resumable build surface |
| `PropertyReadPlanCompiler` and opaque `PropertyReadBuildCursor` | `clinkz-wot-planning` | Exact bounded Producer-route projection from one complete registration identity/compiler view; no arbitrary role construction |
| `CandidateFallbackPolicy`, `CandidateSkipReason`, `CandidateSkipDiagnostic`, and `CandidateSelectionDiagnostics` | `clinkz-wot-planning` | Immutable Consumer fallback policy plus bounded, typed pre-execution selection diagnostics |
| `ServerFormContributor` and form-contribution values | `clinkz-wot-core` binding SPI | Side-effect-free Producer form finalization input/output |
| Compiled plan-set record, plan-set lease, lazy-artifact slot, compiler lease, and reclaim cursor | `clinkz-wot-servient` | Aggregate lifecycle and mutable runtime ownership; these records are not binding APIs |
| Concrete artifact payload and compiler-extension implementation | Concrete binding crate | Protocol-specific immutable planning data |

The complete registration bundle atomically pairs the compiler extension with
its capability and execution registrations. The binding SPI specification owns
that bundle's exact construction API. Planning consumes its immutable
projection and does not define client calls, route guards, request acceptance,
responses, subscriptions, emissions, or cleanup APIs.

## Required evidence

Evidence is executable and uses fixed manifest, fixture, feature/profile,
binding-set, policy, allocator, clock, and workload identities. A prose review
or benchmark without those identities does not close a gate.

| Evidence key | Required assertions |
| --- | --- |
| `plan-cost-and-limits` | One shared logical plan for multiple bindings; measured footprint ledger; eager/lazy equivalence; one-over form, candidate, schema, security, artifact, cursor, and byte failures; no silent omission |
| `index-lazy-request-size` | Indexed probes only; explicit wildcard bound; stable ordering; no hot TD/all-binding scan; no static target/schema/security/extension clone in `OutboundRequest`; Producer artifacts eager and unused eligible Consumer artifacts lazy |
| `lazy-cache-single-flight-generation` | One compiler lease under concurrent first use; bounded waiters/backpressure; deterministic Negative classification; non-cacheable cancellation/resource/deadline results; generation isolation; eviction with a referenced result; incremental zero-byte reclamation |
| `per-operation-candidate-bound` | Admission at limit and rejection at limit plus one; 1/8/32 candidate selection scaling; exact `Disabled`/`PreExecution` outcomes; only the two eligible skip reasons; complete ordered fixed-width diagnostics; binding-input, health, transient, security-commit, and post-acceptance non-fallback; each candidate examined once; one shared provider-probe budget; strict lookup does not bypass limits |
| `form-finalization` | Contributor capability pruning, deterministic stable output, side-effect detector, exact merge/freeze order, generated and supplied owner resolution, explicit ambiguity, behavior coverage and late-handler diagnostics, collision across generations, immutable post-freeze TD, and rollback with no external contribution state |
| `compiled-plan-lifecycle` | Consumer atomic publication, Producer freeze-before-side-effect and publication-with-serving, cancellation in every build phase, pin/drain races, late route rollback pinning, lazy-work drain, and zero retained plan/artifact bytes after terminal reclamation |
| `binding-compiler-extension` | Third-party compiler implementation, compatibility mismatch, deterministic output across step sizes, zero-budget no-progress, declared-versus-actual footprint violation, callback outside locks, no credentials/handlers, and no protocol side effect during compile |

Performance manifests additionally cover cold maximum-plan compilation,
repeated hot target-operation selection, 1/8/32 candidate scaling, single-flight
first use, deterministic negative hits, maximum-generation drain, and bounded
incremental reclamation. Reports include work units, indexed probes, compiler
steps, logical bytes, artifact bytes, lazy and negative bytes, peak temporary
bytes, selection latency, and retained bytes after reclamation.

Constrained compile evidence builds planning and its protocol-neutral values
with `no_std + alloc`, caller-owned cursors, explicit static limits, and no host
executor dependency. A fake third-party binding outside the workspace member
list must compile a bounded artifact and consume it through the matching
execution registration without access to Servient internals.
