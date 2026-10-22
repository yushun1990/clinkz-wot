# Planning and Compiled Plan Sets

Status: active v5.1 authority. Nine requirement definitions are registered.
Retained deferred clauses remain entry-review input and carry no implementation
authority unless explicitly registered by the v5.1 manifest.

This specification is the single normative owner of effective-form selection,
capability indexing, logical-plan construction, binding-compiler coordination,
compiled-plan-set publication, and plan reclamation. It refines
`docs/architecture/30-compiled-plan-lifecycle.md` and ADR-0008. The Protocol
Binding SPI specification owns transport execution, route progress, calls,
subscriptions, responses, and publication progress; this specification only
defines the immutable values that those operations consume.

This specification owns exactly `PLAN-COST-001`, `PLAN-COST-003`,
`PLAN-BOUND-001`, `PLAN-SET-001`, `PLAN-ARTIFACT-001`, `PLAN-REQUEST-001`,
`FORM-FINALIZE-001`, `FORM-OWNER-001`, and `FORM-COVERAGE-001` in the v5.1
candidate. The retained clauses for `PLAN-COST-002`, `PLAN-INDEX-001`,
`PLAN-LAZY-001`, `PLAN-CACHE-001`, and `FORM-FINALIZE-002` remain explicitly
inactive until a later planning-domain entry review re-adopts or replaces them.

## Normative requirements

`PLAN-COST-001`: Planning MUST share protocol-neutral semantic work across
binding candidates. A persistent two-level logical/binding representation is
optional: minimal source-free execution entries MAY reference separately owned
immutable definitions. It MUST NOT duplicate full logical data merely for each
candidate or equate instance identity with program content identity.

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

`PLAN-BOUND-001`: Every target-operation plan MUST enforce the admitted
`form_binding_candidates_per_operation_max`; fallback MUST examine each retained
candidate at most once and share one `provider_probes_per_interaction_max`
budget across all examined candidates, while strict selection MAY narrow but
MUST NOT bypass either bound.

`PLAN-SET-001`: Servient MUST own each instance's publication, current
authority, generation, pins/operation leases, draining and accounting. A Plan Set
MAY collect entries and requires its own generation only when independently
replaceable or retained. Shared immutable definitions require a distinct funded
physical owner and admitted references; closing one instance MUST NOT reclaim
another's data. Reclamation follows actual final physical release. Existing
Consumer separate generations remain checked until deliberately migrated.

`PLAN-ARTIFACT-001`: Planning MUST pair the actual compiler and executor's
format/configuration/policy before effects. Native programs MUST be immutable,
source-free, fit funded final/cursor/temporary/work bounds, and contain no live
credential, handler, external resource or cleanup obligation. Producer coverage
and all required semantics/capacity MUST be established before effects and
publication. Qualified incremental pure construction MAY acquire capacity before
each step, retaining rollback ownership; it need not establish all native bounds
before any pure start. Existing Consumer all-bounds/error ordering remains its
exercised subset until its evidence is renewed. Runtime generation and authority
checks belong to each admitted entry, not to shared program content.

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

For a concrete binding, omitted protocol-significant form facts must never be
interpreted as defaults merely because the current compiler input lacks them.
TD lends bounded immutable facts; Planning preserves the facts or their paid
eligibility proof needed for the registration's declared support envelope.
The binding compiler interprets protocol vocabulary without raw TD access or
execution-time rescan. The original HTTP entry rejected form extensions through
a minimum presence projection. The [Host semantic amendment](#host-semantic-form-dispatch)
replaces that shortcut with resolved content and native lowering. Existing mock
evidence retains its original scope; it proves no HTTP vocabulary interpretation.

Information sufficiency is relative to a declared support envelope. If two
admitted documents produce identical complete lawful inputs to a component,
that component must not be required to distinguish their applicability or
execution meaning. A trusted upstream check may instead reject one document
or establish their equivalence before effects. This refines `PLAN-COST-003`,
`PLAN-ARTIFACT-001` and `FORM-COVERAGE-001`; it requires neither a complete TD
copy nor one new logical-plan field per source field. Facts may be discharged
by admission, retained as bounded resolved data, or lowered by the responsible
compiler. A public plan constructor or metadata setter is a data constructor,
not evidence that trusted source admission occurred.

Planning starts from a TD-owned validated semantic view or produced-Thing draft
plus immutable policy and registration snapshots. For the first Consumer
aggregate, TD supplies paid short semantic loans from its opaque validated
immutable Thing proof; Planning never receives raw input/storage access. It ends with an admitted
immutable plan set.
The following invariants apply to every profile:

- Planning consumes TD-owned default/effective results. The completed direct-
  `Thing` leaf calls the pure `clinkz-wot-td::td_defaults` helpers; the future
  borrowed Consumer aggregate reads the same authority through its paid
  semantic lending cursor. Planning does not copy those rules into its own walker
  or a binding.
- `base` resolution, operation defaulting, and effective-security inheritance
  belong solely to TD; each required pass pays its semantic work. URI-template
  compilation, form identity, and candidate ordering happen once in Planning.
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
| `clinkz-wot-core` | Protocol-neutral immutable plan identities and values, binding capability/runtime artifact SPI values, and generation-bearing plan references |
| `clinkz-wot-planning` | Compiler authoring/erasure and preparation pairing, selection over TD-owned effective forms/targets/security, capability indexes, logical-plan construction, URI-template compilation, compiler coordination, deterministic ordering, and admitted build output |
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

The [scoped Consumer ownership configuration](consumer-resource-ownership.md#closed-consumer-binding-configuration)
uses the same complete registration digest in native/Host compiler checks,
candidates and artifact envelopes. Planning forwards it unchanged; no engine-
policy key or translated capacity key substitutes. This eligibility identity is
not a content-equivalence or storage-sharing key. The extension changes no
Planning algorithm, public input, generation or all-bounds-before-start rule.

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

Under `std`, Planning provides compiler/cursor erasure and Core provides completed
runtime artifact storage:

```rust
pub struct HostBindingCompilerCursor { /* Planning-private erasure */ }
pub struct HostBindingArtifact { /* Core-private erasure */ }
pub struct HostBindingCompilerRegistration { /* Planning-private erasure */ }

impl HostBindingCompilerRegistration {
    pub fn new<C>(compiler: C) -> Self
    where
        C: BindingCompilerExtension + Send + Sync + 'static,
        C::Cursor: Send + 'static,
        C::Artifact: Send + Sync + 'static;

    pub fn try_new<C>(compiler: C) -> Result<Self, BindingInputRejection<C>>
    where
        C: BindingCompilerExtension + Send + Sync + 'static,
        C::Cursor: Send + 'static,
        C::Artifact: Send + Sync + 'static;

    pub fn slot_layout(&self) -> core::alloc::Layout;

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

impl HostBindingArtifact {
    pub fn storage_overhead(&self) -> u64;
}
```

Planning implements compiler/cursor erasure and Core implements completed artifact
access with safe standard-library type inspection. A cursor
type mismatch makes `step` return `Failed` with the original erased cursor;
`abort` returns `Err(original_cursor)`. Compatibility or payload-type mismatch
returns `None` for borrowed access and `Err(original_artifact)` for consuming
access. Binding crates neither implement nor invoke an unsafe erasure hook.

Both generic and closed Consumer erasure use one typed slot holding either the
native cursor or the completed output. `start` acquires its complete actual
`slot_layout()` fallibly before the native callback. Pending, Failed and Complete
reuse that allocation; artifact extraction also requires no new allocation.
`try_new` returns the original compiler on startup allocation refusal without a
native callback. `new` retains the Host deployment-failure policy for startup.
The slot records the concrete compiler type, so sharing associated cursor and
artifact types does not authorize a different compiler body to consume a job.
This is type pairing, not live registration or generation validation.

The native artifact footprint still covers its inline payload and private
backing. `storage_overhead()` reports the
slot's bytes beyond the inline native payload, including tag, output metadata,
padding and retained cursor capacity. Host Producer retained-output admission
adds this overhead once to compiled/runtime storage; native artifact quotas
continue to measure the native payload. Closed Consumer admission already charges
its full slot separately and must not add this overhead again. Native private
backing, outer owners, callback work and transient overlap remain separate.
Static retained-output accounting excludes the native inline payload from the
outer-envelope charge because that payload is already in its native footprint.
These physical storage rules do not open closed Consumer eligibility or qualify
arbitrary callbacks, destructors or complete generic compiler admission.

### Complete-registration relationship

`HostBindingCompilerRegistration` and
`StaticBindingCompilerRegistration<C>` are constructible component values, not
Servient installation units. WP-200 implements them and the contract above.
Planning consumes one matching component and a Core runtime registration into
`HostBindingPreparation` or `StaticBindingPreparation<B>` (and the Consumer-only
equivalents). The current `ServientBuilder` accepts this complete preparation
bundle. `into_execution` releases its compiler and returns the validated Core
runtime registration. Actual identity, capability, native format and policy are
checked before effects; a runtime registration never requires a compiler.

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

This frozen `PlanBuildInput<&Thing>` shape remains the completed exact-
coordinate Producer/Consumer leaf contract and its existing evidence remains
current. It is not the input contract for the later all-readable Consumer
aggregate. That successor must adapt the same behavioral leaf to TD's paid semantic
lending cursor before source admission; it may not recover raw `&Thing` from
the opaque proof. This authority
migration changes no existing Planning source or completed tranche status.

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

A preparation registration may supply one deterministic `ServerFormContributor`
and a bounded `FormContributionCapability`. The contributor receives only the
binding-independent draft view, matching bounded affordance requirements, and
an immutable contribution context. It returns generated forms associated with
their exact targets, generated security definitions and JSON-LD context terms,
and canonical endpoint reservation identities for the forms it generates.
The contributor and document contribution values belong to Planning's preparation
surface; runtime Core owns only the source-free endpoint reservation identity.
These broad contribution APIs remain unimplemented.
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

`PLAN-REQUEST-001`: Per-call requests MUST reference immutable static target,
form, URI-template, schema, security, response, extension, and artifact data by
generation-bearing plan, target, affordance, candidate, and artifact slots. A
request MUST own only facts that genuinely vary for that call, including
payload, URI-variable values, cancellation/deadline state, correlation or
idempotency values, committed security material, and protocol-status data where
that data belongs to the selected call boundary.

A request MUST NOT clone static target strings, URI-template programs, schemas,
security expressions, response metadata, or extension maps. Host erased calls
may retain a shared plan-set lease. Constrained calls use a generation-bearing
slot reference whose owner retains the same plan-set lease. Neither
representation gives a binding access to the full TD or another candidate.

The v5.1 Consumer Property Read architecture slice uses an eager admitted
Consumer artifact. Activating `PLAN-REQUEST-001` does not activate
`PLAN-LAZY-001`, `PLAN-CACHE-001`, `PLAN-INDEX-001`, or `PLAN-COST-002`, and the
first Consumer gate MUST NOT depend on those deferred mechanisms.

### First Consumer Property Read aggregate

#### Shared semantics and representation applicability

This is a dynamic preparation capability with application-static and public
Host-erased representations. Application-static describes typed dispatch and
storage ownership, not a hardware target, heapless execution, or a requirement
to run Planning on embedded devices. Host/static completion covers the declared
common capability; it does not establish every embedded deployment. Fixed-image
preparation/installation remains a separate, unadmitted trust boundary.

The shared contract is TD-owned meaning, deterministic coordinate/candidate
selection, complete preflight, all bounds before any start, owned immutable
output, qualified identities, monotonic work, whole-transaction failure and
bounded terminal ownership. Concrete binding compilers lower resolved inputs
into protocol artifacts; they do not repeat TD semantics or select candidates.
Planning owns safe Host compiler/cursor erasure; Core owns completed artifact
storage. Allocation, storage Layouts, cursor transport and
destruction primitives are supported per preparation representation. Servient
owns policy capture, parent/global capacity pairing and publication. Sharing
the Planning program requires none of a common allocator implementation,
identical output containers, identical physical costs or separate Runtime crates.

Host/static parity compares logical facts, selected identities, common supported
outcomes and lifecycle/resource invariants. Each representation independently
proves its physical capacity, work and cleanup costs. An unsupported compiler or
representation fails eligibility before callbacks; representation-specific
capacity exhaustion need not occur at the same byte ceiling. No Host allocation
or destructor proof is inherited by a static implementation, or vice versa.

#### HTTP entry amendment

[WP-600 HTTP admission](../work-packages/WP-600-http-property-read-target.md#paid-metadata-and-protocol-neutral-compiler-amendment)
owns the admitted paid-metadata scalar and restricted closed-compiler projection.
Carry that scalar in the existing owned logical plan; retain whole-transaction
all-bounds-before-start and rollback. This permits no raw TD/protocol branch and
limits source scope to that admission. Existing unrestricted support/evidence
remains valid for its original scope; changed carrier costs require successor completion.

#### Host semantic Form dispatch

For `HostSemanticCompiler`, preparation resolves a semantic closure for each
scheme-eligible Form immediately before native start. The closure contains Thing,
the selected property's typed schema and resolved extensions, that Form and its
responses, effective security definitions, scoped URI-variable schemas and named
additional-response schema dependencies. Other properties/Forms/unused security definitions are not
visited by this pass. Basic TD validation and common security constraints still
apply to the complete input. This replaces whole-document semantic preparation:
a valid MQTT Form's unsupported local context or its large annotation must not
make an independent HTTP Form unavailable.

Engine capacity for one pair of arena buffers is reserved before allocation and
reused as a temporary allowance across coordinates. Type/configuration-based
bounds are collected before any compiler starts; they do not inspect extensions.
The selected closure survives that coordinate's start/step callbacks, then is
released. Only one closure is live; none enters published plans or requests. TD
reserves the atomic work maximum, then settles only actual lookup/node/byte work
and returns unused credit. All actual work consumes the original TD lifetime.

Coarse scheme declarations supplied by the captured Binding filter effective
readable Forms before materialization. Planning contains no protocol enum or
HTTP/MQTT term branch. Operation defaults, security inheritance, the current
exactly-one-NoSec constraint and engine authority remain common. The bounded
aggregate still captures one complete registration; arbitrary multi-registration
discovery and automatic same-protocol competition are not implemented.

A semantic profile/capacity/unsupported-dependency refusal or native
`UnsupportedOperation` makes only
the affected coordinate unavailable. A live compiler slot is released when
present. A shared Thing/property context restriction can affect every Form in
that scope; this remains a resolver-profile limitation, not TD invalidity or a
W3C prohibition. Basic invalidity, security refusal, allocation failure and
exhaustion of the original TD work lifetime still abort the transaction.
Default selection chooses the first supported coordinate;
a strict Form index cannot fall through to another Form. Scheme-excluded Forms
are unavailable to that registration. No fallback occurs after native effects.
An unsupported coordinate creates no request or I/O, and retained negative
coordinates participate in the same funded plan cleanup. Whole-draft Producer
coverage/collision/readiness rules are unchanged and are not proved by this
Consumer selection behavior.

This is an incremental compilation-boundary migration. `LogicalInteractionPlan`
remains the existing engine runtime identity/selection record; the Host compiler
receives the separate resolved input. Retiring runtime containers or replacing
Producer installation stages requires evidence at those boundaries.

#### Aggregate construction

The first aggregate contains every effective readable Property Read Form from
one TD-owned validated immutable Thing loan and exactly one finalized complete
Consumer-capable Property Read registration. Properties use Thing's BTreeMap key
order; Forms retain original array indices. TD's paid lending cursor supplies
raw/resolved targets, content type/coding, subprotocol/scopes, effective operations
and security-definition meaning. Planning receives no raw Thing/map/storage
access and owns no Basic/default/security/URI rules. Each property gets a lookup
row, including an empty readable range. Each retained coordinate has registration
ordinal/candidate order zero, one eager ConsumerCall artifact, no credentials and
exactly one TD-resolved NoSec definition. Explicit empty ops/security remain empty.

Planning owns `ConsumerPropertyReadPreflight` and
`ConsumerPropertyReadDraft<A: 'static>`, both without TD/environment lifetimes.
Preflight has checked property/coordinate counts, captured registration identity,
conservative output/cursor/temporary/cleanup capacities and a remaining Planning
work envelope. It has no plan ID/artifact/lease or registration owner. It is associated with
its originating proof/policy/registration/transaction nonce; its counts cannot
authorize another build. Draft owns all logical plans/candidates, eager concrete
artifacts/references, names/ranges/lookup, required execution programs and exact
PlanFootprint. It has no source or registration callback/payload loan, publication
state or mutable index-rebuild authority. Opaque unrelated metadata is inspected,
not retained. The first slice preserves its existing narrow response-sealing rule;
deferred operation-specific schema/template programs need owned-output evidence
when their operation enters, never runtime schema loans.

A Basic-valid missing ID fails preflight before persistent reservation,
materialization, bounds or start. Preflight and materialization each enumerate
through TD's monotonic semantic cursor and debit the same remaining TD lifetime;
rewind at Done never refills it. One Property/Form event stays Ready until Planning
has fully checked copy/item/cleanup credit. TD has already paid URI UTF-8 validity,
security lookup and scope-count/byte-total facts. Planning retries read those
scalars without source sizing/semantic replay, then charge each sequence advance
and copy before actual materialization. Zero/short credit cannot accumulate across
calls or acknowledge a partial event.

After preflight, Servient reserves runtime/record/slot/rollback envelopes. Planning
then materializes **every** coordinate and owned logical input before collecting
all pure compiler bounds. No native source scan or synchronous public Thing helper
is an aggregate entry. The existing exact-coordinate `PlanBuildInput<&Thing>` is
narrow leaf regression authority and proves no validated trust boundary.

The portable consumed build boundary is frozen as follows (private layouts and
helper modules are implementation mechanics):

```rust
pub trait ConsumerPropertyReadRegistration: private::Sealed {
    type Artifact: 'static;
    // Projection from Planning's checked ConsumerCompilerRegistration<R>, which
    // retains the complete Host/static registration and its closed support.
}
pub struct ConsumerPropertyReadBuildConfig { /* checked fixed Planning projection */ }
pub struct ConsumerPropertyReadRollback<A: 'static> { /* owned output, accounts, fixed cause/positions */ }
pub struct ConsumerPropertyReadCleanupSlot<A: 'static> { /* reserved complete-object destination */ }
pub enum ConsumerPropertyReadRollbackStep<A: 'static> {
    Pending(ConsumerPropertyReadRollback<A>), Complete(CoreError),
}
impl<A: 'static> ConsumerPropertyReadCleanupSlot<A> {
    pub fn try_new<'capacity>(config: &ConsumerPropertyReadBuildConfig,
                   capacity: ResourceReservation<'capacity>)
        -> Result<Self, (ResourceReservation<'capacity>, CoreError)>;
    pub fn take(&mut self) -> Option<ConsumerPropertyReadRollback<A>>;
}
impl<A: 'static> ConsumerPropertyReadRollback<A> {
    pub fn step(self, budget: &mut WorkBudget) -> ConsumerPropertyReadRollbackStep<A>;
}
pub struct ConsumerPropertyReadBuild<'td, 'environment, R: ConsumerPropertyReadRegistration> {
    /* owns TD cursor, accounts, nonce, draft and scalar phase; borrows external R */
}
pub enum ConsumerPropertyReadBuildStep<'td, 'environment, R: ConsumerPropertyReadRegistration> {
    Pending(ConsumerPropertyReadBuild<'td, 'environment, R>),
    Preflight { build: ConsumerPropertyReadBuild<'td, 'environment, R>,
                requirements: ConsumerPropertyReadPreflight },
    Complete(ConsumerPropertyReadDraft<R::Artifact>),
    Failed { cause: CoreError,
             cleanup: Option<ConsumerPropertyReadRollback<R::Artifact>> },
}
impl ConsumerPropertyReadBuildConfig {
    pub fn try_from_limits(limits: &ResourceLimits) -> Result<Self, CoreError>;
}
pub fn build_consumer_property_read_aggregate<'td, 'environment, R: ConsumerPropertyReadRegistration>(
    td: ValidatedThing<'td>, registration: &'environment R,
    config: &ConsumerPropertyReadBuildConfig, generation: PlanSetGeneration,
    ledger: AdmissionLedger,
    cleanup: &'environment mut ConsumerPropertyReadCleanupSlot<R::Artifact>,
) -> Result<ConsumerPropertyReadBuild<'td, 'environment, R>, CoreError>;
impl<'td, 'environment, R: ConsumerPropertyReadRegistration>
    ConsumerPropertyReadBuild<'td, 'environment, R>
{
    pub fn step(self, budget: &mut WorkBudget, cancel_requested: bool)
        -> ConsumerPropertyReadBuildStep<'td, 'environment, R>;
    pub fn admit_runtime(self, ledger: AdmissionLedger)
        -> Result<Self, (Self, AdmissionLedger, CoreError)>;
}
```

The checked Planning projection covers only existing structural/candidate/output,
artifact/cursor/temporary, compile-step and reclaim fields consumed by this slice.
It rejects missing values/arithmetic failure before progress and certifies only
that limits projection, not compiler behavior. Complete
Consumer role/profile/registration eligibility is Servient's checked capture.
Config constructors allocate/scan no external input. The build copies fixed config
and captures complete registration identity; its entry does no variable work.
The sealed registration adapter projects Core's checked
`ConsumerCompilerRegistration<R>` with the actual artifact/cursor types. Raw
complete-registration forwarding remains insufficient for aggregate eligibility.
The checked Consumer policy must bind support
to the actual compiler implementation, complete registration identity and
selected preparation representation/configuration before any aggregate compiler callback.
Bounds/start/step/abort primitive costs, cursor/temporary storage, allocation
behavior and artifact destruction are part of that support. A limits projection,
compatibility identifier or complete-registration validity alone is not a support
attestation. [Planning's closed support contract](binding-spi.md#closed-consumer-compiler-support)
owns the consuming complete-owner construction, actual immutable compiler
configuration and selected Host adapter. The original eligible implementation is
the fixed resolved-target copy primitive. The Host semantic contract above
supersedes exact-compiler eligibility only for its declared representation.
The [precursor](../work-packages/WP-200-consumer-compiler-support-admission.md)
must complete its public Host/static evidence before aggregate admission. The
existing build signature takes the checked owner as `R`; its limits-only config
does not mint support. No downstream trusted proof factory or installable
compiler-only component is introduced by this requirement.
No concrete protocol dependency or unsafe type identification enters Planning.
Binding authors still use the Planning preparation pair and Core execution registration.

The additive [Consumer-only registration precursor](../work-packages/WP-300-consumer-only-registration-admission.md)
extends this sealed adapter to checked complete static/Host Consumer-only owners.
It changes no aggregate algorithm, eligibility rule, public build signature,
lookup, rollback or parent-provenance authority. Existing Producer and dual-role
owners remain separate supported registrations. The new adapters require that
precursor's independent admission; they are not authorized by prior aggregate
completion.

Preflight is emitted only through the consumed `ConsumerPropertyReadBuild::step`
result; there is no separate standalone preflight entry with a second transaction
owner.

Preflight is emitted once, leaving the returned owner waiting for runtime
capacity. The coordinator associates that immutable requirement with its live
transaction, policy, registration, generation and paired parent allowances before
calling `admit_runtime`. That method moves the coordinator's capped child output
ledger into the unique waiting build. A wrong phase returns the unchanged build
and ledger with a fixed error, without work or lost capacity. Waiting polls return
Pending without source replay.

An `AdmissionLedger` is local accounting, not a transaction/policy certificate.
It has no public origin, policy or nonce inspection and can be constructed by a
caller. Planning must not claim to authenticate parent capacity or a ledger's
transaction provenance, infer it from equal numeric ceilings, or introduce a
synthetic reservation to inspect private ledger identity. Its private phase owner
keeps preflight attached to the original TD continuation, config and registration;
it does not accept a detached preflight as authority for another build. Planning
enforces its copied policy and every actual Layout/local charge before allocation.
An insufficient child account fails the unpublished build and follows rollback;
it does not trigger a smaller successful subset. Servient separately proves the
matching transaction and simultaneous parent/global reservation. This advanced
Planning handoff neither publishes nor independently certifies that pairing.
Complete consumes/destroys all input/config/registration/compiler-bearing
state before exposing the source-free draft payload; destructuring Complete leaves
no generic terminal cursor keeping the TD loan alive. Failed first ends TD/external-input loans and performs the supported prepaid
fixed exactly-once abort, then preserves cause and returns all remaining output/
account objects in source-free Rollback. Its Pending/terminal cleanup shape is
`ConsumerPropertyReadRollbackStep<A> { Pending(Rollback<A>), Complete(CoreError) }`;
`Rollback::step(self, &mut WorkBudget)` uses monotonic item/byte positions.
The fixed cause/diagnostic has admitted bounded storage; a compiler CoreError is
reduced to that category without uncharged formatting or an unbounded destructor.
Cleanup completion is not admission success.

The cleanup-slot constructor checks the cleanup resource kind and sufficient
bytes for its actual fixed inline destination, captures the reservation's owner,
generation and amount as fixed metadata, and commits the guard on success. Its
Rust storage supplies the destination's alignment. Rejection returns the original
uncommitted guard and a fixed error. Committing ends the temporary mutable account
loan; it does not release the charge or transfer the account into the slot.
The coordinator retains that account and its paired parent allowance until the
actual slot is released and every deposited/taken cleanup child is terminal.
The coordinator validates transaction/policy association before construction;
the public Foundation guard alone cannot certify it. No allocator request or
synthetic contiguous allocation is made for this inline logical reservation.
The slot is vacant and reserved before entry/output construction, never allocated
on drop. `CleanupSlot::take(&mut self) -> Option<Rollback<A>>` moves the complete
cleanup object to its driver, and `Rollback::step` consumes/returns its owner.
The slot owns complete pending output/accounts, not merely a cleanup record.
The advanced build entry borrows the external slot; Host runtime and static root
retain the actual reserved slot owner. Abandonment first ends all caller/config/
registration/compiler loans and executes prepaid bounded abort/fixed TD release,
then moves all source-free output/accounts into that destination with fixed work.
The deposited Rollback contains no references to input, registration or the build.
If another live owner occupies the destination the entry is rejected before work;
no fallback allocation or lost owner is allowed. No detached cleanup retains a
caller input loan, and no variable collection Drop is hidden in abandonment.

The additive [terminal ownership bridge](../work-packages/WP-200-consumer-output-ownership-admission.md)
projects a consuming transfer of the actual matching draft/root or variable
rollback/root allocation into a unique source-free owner. It preserves the
original output, ledger and bounded reclaimer, derives `Send` structurally when
the artifact is Send, and promises no Sync capability. Existing build and local
handle contracts remain unchanged. Physical root identity, fixed prepaid work,
unchanged-owner rejection, original cleanup-account retention and exact
signatures are owned by that admission record; implementation is limited to its
ADR-0013 envelope. No public ledger observation becomes a parent permit,
and no wrapper around the existing local graph is claimed to make it transferable.

Compiler bounds/start/step/abort callbacks are themselves supported bounded
primitives. Predebit their admitted callback cost, including the cost to compute
bounds, before calling them; returned bounds cannot retrospectively pay for the
callback. Obtain conservative cursor/temporary storage before start and check
actual requests before allocation, not after an infallible callback has exceeded
capacity. The admitted binding set provides an allocation/cleanup adapter or
source-level proof for every callback, specific to the selected representation.
An inline/static compiler may prove fixed storage and bounded destruction without
an allocator adapter. An allocating compiler or erasure adapter must either
preserve ownership on allocation failure or prove backing storage cannot fail
under its held reservation. A logical ledger allowance alone does not reserve
allocator storage. Checking a returned artifact after infallible allocation is
not such a proof. No universal allocator trait or rewrite of all compiler
implementations follows from this obligation. A `'static` artifact bound is necessary;
concrete no-source-pointer/callback, declaration honesty and post-destruction
execution evidence are also required. Each callback receives a short owned-input
loan; suspended compiler state owns data or checked positions, never a loan into
movable transaction output.

Every bound must declare only `WorkClass::BindingPolls`, with a nonzero total
no greater than the existing `plan_compile_work_units_per_step_max`. All
coordinate bounds must succeed and a completed all-bounds barrier must exist
before the first compiler `start`. Compilers then run sequentially with one
non-resettable coordinate remainder. The aggregate compiler-work maximum is the
checked product of readable-coordinate count and that existing per-step value;
the count is already bounded by `forms_per_context_max` and
`forms_per_thing_max`. This narrow eligibility rule does not reinterpret the
global field as a per-plan or per-admission limit and introduces no resource
row.

The persistent reservation remains uncommitted through materialization and
bounds evaluation. Any failure there releases every still-uncommitted
reservation, produces zero compiler `start` calls, and cannot enter
publication. Compilation failure, reconciliation failure, cancellation, or a
seal failure likewise fails the whole unpublished aggregate; no successful
subset is published. The existing exact-coordinate
`PropertyReadPlanCompiler::consumer_call` remains the behavioral leaf. A
crate-private preparation kernel may be shared to support the barrier, but the
completed exact-coordinate semantics and evidence remain unchanged.

Lookup addresses one property row and only its contiguous readable-Form range.
Omitted `form_index` selects the first readable row; an explicit index must
equal an original Form-array index in the same range. A missing property is
`AffordanceMissing`, while a present property with an empty range is
`NoFormSupportsOperation`. Lookup scans neither the TD, another target, nor a
registration collection. Its fixed selection resolves the plan, artifact, and
retained complete registration only while the caller holds the matching
plan-set lease.

Aggregate enumeration, row construction, lookup sealing, reconciliation, and
later reclamation consume `WorkClass::PlanningItems` through monotonic cursors.
Owned byte copies pay CodecOutputBytes; sequence advances pay their declared
bounded primitive/item cost; cleanup pays CleanupItems and its byte work.
Before each phase, derive its checked non-resettable item/byte/cleanup envelope
from the fixed pass count, admitted structural counts, preflight byte/program
capacity and constructed output/artifact capacity. These are private projections
of existing limits, not new ResourceKinds. Every accepted unit debits that
phase's remainder before work; atomic multi-class debits cannot partially commit,
and movement, fresh budgets or retries cannot refill it. Preflight source queries
still debit the same TD lifetime. Compiler progress uses its separate declared
coordinate remainder. Exhaustion fails the unpublished aggregate with a bounded
limit cause; cleanup's reserved capacity and lifetime allowance remain available
to terminal release under its explicit per-step budget.
Existing structural/output maxima bound lifetime work, the existing step limits bound
one call, and `plan_reclaim_bytes_per_step_max` bounds reclamation. The sealed
draft is transferred once to Servient; Servient does not recount semantics,
rewrite artifact slots, or construct a second lookup.

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

The Consumer transaction performs `Building -> Frozen -> Published` without
transport side effects. First-Consumer Frozen output is complete/source-independent;
input-bearing build state is consumed and bounded scratch cleanup finishes before
final actual-account/identity/generation/registration/cancellation/slot checks.
A private permit closes cancellation under exclusive installation authority; the
preallocated complete record is atomically installed without allocation, callback,
yield, source query, fallible release or further cancellation decision. Returning
a handle proves Published; failure publishes neither a handle nor a subset.

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

The accepted [Consumer resource migration](consumer-resource-ownership.md)
reaffirms full compiler/cursor/artifact/output admission. It changes no Planning
API, all-bounds barrier or source-independent ownership contract; shared Core
metadata Layout changes still require fresh affected measurements.

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

For this first slice retained-source and lazy-slot fields are zero. Exact output
accounting separates logical strings/programs, table capacity, candidates/lookup,
artifacts, compiler/cursor/temporary overlap, registration/erasure/record deltas,
inline capacity and rollback/reclaim metadata. Persistent output allowances are
logical sums; largest-contiguous observes only individual actual checked Layouts.
BindingArtifactFootprint measures the binding-authored payload. Core erasure and
storage-owner costs are composed separately into the selected representation's
admission/PlanFootprint, without changing that payload declaration or counting
inline payload storage twice. Equal binding bounds therefore do not imply equal
physical admission totals. A Host preparation tool cannot authorize target storage
from Host Layouts; any future generated-image path must prove target capacity at
its own installation boundary.
A variable output rollback/reclaimer retains monotonic item/byte positions, owns
complete artifacts, admits bounded destructor primitives and releases children
before parents. `Vec` nesting is not evidence of bounded Drop.

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

- structural enumeration and logical materialization make a fixed number of
  monotonic passes over supplied structures; every actual semantic lookup/byte
  comparison and URI path action is bounded and charged through TD. The supported
  iterator envelope and repeated security/URI work can exceed linear charged work
  in unique input size; no free native lookup or Pending prefix replay is allowed;
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

For the first Consumer Property Read record, that immutable generation contains
the execution plans and lookup, the complete binding registration owner, and
runtime lifecycle/resource records. It contains no TD proof, input/derived loan or TD source/document charge
attributable to this borrowed build.

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

## Host Producer Property Read entry

The accepted ADR-0013 [Host HTTP Producer admission](../work-packages/WP-600-http-producer-property-read.md)
selects this bounded replacement for Host Producer Property Read. Its recorded
admission grants only that implementation scope; existing source is not claimed
conforming. Other roles and static representations retain their
stated contracts. This section is the scoped replacement of the mandatory
persistent logical-plan part of `PLAN-COST-001` and separate plan-set identity
in `PLAN-SET-001`: one immutable entry is owned by one produced instance, with
no independently replaceable plan-set generation. Lease/reclamation and
registration/route stale-use invariants remain mandatory.

TD owns whole-input Basic validation and effective operations/security/URI
resolution. Its Producer semantic continuation must reuse the shared validation,
default and bounded resolved-tree kernel; it must not call the Consumer's
first-read selection or require a source id. Pure private construction may
precede rejection. Before install, Planning examines **all** declared Property,
Action, Event and root Forms and their effective operation sets, including
omitted/defaulted and empty declarations. The first scope rejects unsupported
shape/capability rather than selecting a supported leaf of an uncovered draft.
Each retained Form/operation needs exactly one paired owner, executable mapping,
supported effective security and exact handler capability. No Binding callback
may obtain a handler object. No effects occur from compilation.

`ResolvedProducerInput<'a>` lends the chosen coordinate's effective operation,
resolved target, media/subprotocol and the existing `ResolvedSemanticDocument`
(schema, Form/response, security/proxy, URI-variable and scoped extension facts).
Unknown extension data is preserved for the paired compiler, not certified
as supported behavior by mere preservation. The first Producer support policy
explicitly refuses unhandled predicates and Profiles before installation.
Source id is optional. Producer-only construction does not go through
`BindingCompilerInput::resolved`'s logical-plan/id check. All source inspection,
arena construction and compilation consume original lifetime/step credit before
work, with explicit limit/dependency refusal. Do not reparse serialized TD or
JSON-LD in Planning or the Binding. The existing 128-node/32-scope/depth-16,
65,536-unit reservation is an initial Host resolver limit, not a WoT limit.

`ProducerExecutionEntry<A>` owns only the Engine's immutable dispatch/validation
facts (Property key, operation, media/response constraints and source coordinate)
and the paired Binding's source-free native artifact. Private fields tie it to
the actual captured registration and route. `ThingSlotId` is live local authority;
optional TD id is provenance. Existing `HandlerContext::thing_id()` uses the
source id when present and an explicitly local generated id otherwise; authority
checks always use slot/generation. The published source is not assigned that id.
Empty TD-valid Property keys require no sentinel encoding. The entry's own backing, checked operands and accounts follow its instance.
Referenced immutable programs retain a distinct physical/account owner until
all users drain. Compiler input
never needs a persistent `LogicalInteractionPlan` or native resource.

A privately completed entry plus exact complete-coverage result permits driver
construction only after Engine, ingress/response and cleanup capacity is funded.
Drop/cancel/refusal rolls back original-account scratch and partial output.
Neither a publicly constructible fact nor a configuration digest is a coverage
certificate. Source loans end before install; source ownership, if requested for
TD retrieval, is explicit and separate from runtime execution.

The semantic preparation checkpoint uses `ValidatedThing::into_producer` to
visit root, Property, Action and Event scopes and every Form. The TD continuation
lends full effective operation sets, original empty declarations, inherited
security and resolved targets using the original validation work/account owner.
`ProducerPropertyReadAdmission` checks the entire draft and exact supplied
handler key before `with_input` lends a `ResolvedProducerInput` to native
lowering. Its initial single-Property/single-Form restriction is local to this
adapter. The TD continuation itself does not filter by protocol or operation.
The Host semantic arena remains charged to the original ledger until physical
release, including refusal and callback unwinding.

The semantic continuation alone yields pure source-free native data. `Covered`
certifies shape and key matching, not effective security, Binding support,
registered handler existence or installation authority.

`ProducerBindingPreparation<C,E>` pairs an open `ProducerCompiler<E>` with the
actual `ProducerExecutor`: associated native program type, complete concrete
configuration equality, captured registration identity/policy and executor
operand validation. The compiler supplies the owned configuration snapshot
during pure preparation; the executor compares that captured value by passive,
allocation-free `configuration_matches` queries, including before runtime
funding. It exposes no mutable executor loan. Only complete
coverage plus the exact owned `ProducerHandlerRegistration<H>` can produce its
privately constructed `PreparedProducer<E,H>`. Refusal returns the pair and
registered capability. Insufficient credit returns them for retry. The Property
key moves into the Core `ProducerExecutionEntry`; empty keys need no sentinel.
Registration requires a `ProducerInstanceAuthority` borrow and retains its
opaque incarnation key as well as `ThingSlotId`. The entry retains that same
incarnation key. Runtime admission consumes the identical move-only authority;
a fresh authority with copied slot/generation values cannot reuse an old entry.
Incarnations refuse on counter exhaustion rather than wrapping. Keys are lookup
data, not publication permits, source ids or physical program identities. This
checkpoint does not use a source TD id at all.

`PreparedProducer::into_execution` consumes the preparation proof into Core's
source-free `ProducerExecutionRegistration`. Only the latter is retained during
native execution. The compiler and all TD/arena loans end before runtime
admission. Core data constructors are not coverage certificates: they cannot
forge Planning's completed wrapper. Configuration/policy and native operands
are rechecked before construction and installation. Inline/Copy programs and
an independently replaceable logical plan-set remain unnecessary.

The [installation contract](binding-spi.md#host-producer-installation-checkpoint)
funds this entry and a closed native installation. It does not yet supply the
request/response bridge or replace the public staged Host expose facade.
The first-leaf Producer projection below is not a fallback for this checkpoint.

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

<a id="proposed-replacement-of-the-compilation-boundary"></a>
### Selected native compilation boundary

ADR-0024 adopts the separation. TD supplies bounded effective operations,
security, schema/response, URI and context/extension loans. Planning selects
Consumer alternatives deterministically and checks whole-description Producer
coverage. Binding performs native lowering without raw-TD defaults, handler
access, credential access or I/O. Lossless metadata preservation is not proof of
execution support. Unsupported advertised Producer behavior refuses before
effects; an unsupported Consumer alternative need not hide another supported
alternative. Explicit selection does not silently fall back.

Minimal source-free executable entries may reference owned native definitions
and checked operands. Preparation ends every source/semantic loan before runtime
installation. Capacity precedes acquisition and rollback retains partial pure
construction. Producer semantics/coverage and runtime/cleanup funding precede
native effects and publication. Shared definition ownership and instance grants
are distinct. The program-definition contract below owns checked reuse.

The current closed Consumer all-bounds and inline Host eligibility remain
measured subsets. General qualified owned/resumable lowering, shared production
backing and reusable programs remain implementation work; no sketch is promoted
to resource/authorization evidence by this extraction.

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

<a id="proposed-program-and-compilation-amendment-review-08"></a>
## Preparation capabilities

Generic compiler authoring and preparation registration are available without
activating the borrowed Consumer aggregate. Planning's `validated-thing` feature
selects that aggregate and forwards TD's explicit capability; the default
Planning product includes it and Servient requests it explicitly. A compiler-only
dependency must not implicitly activate TD loans or `arbitrary_precision` merely
to implement the native compiler contract. Syntax and capability are separate.

## Program definitions and checked instance binding

ADR-0024 adopts explicit definition preparation and checked binding as the reuse
boundary. Compiler dependencies include common/native schema, effective security,
template/mapping and policy facts except declared operands. `bind` must validate
those dependencies and operands without restarting definition compilation.
Independent TD inputs still undergo validation. Changed dependencies or actual
compiler/executor policy refuse before effects or require a new definition.
A program reference, source name or matching Rust type grants no authority.

Planning owns `BindingCompilerExtension`, selected-coordinate inputs, cursors,
steps, bounds and Host compiler erasure. `ResolvedBindingInput` borrows TD-owned
semantic views. Core owns runtime artifact envelopes, fallible allocation and
completed erased storage through `HostArtifactStorage`; it does not own compiler
APIs. Native program types are defined in Binding runtime modules. Optional
preparation registrations pair actual executors and can be consumed into Core
runtime registrations, dropping their compiler. No upward compatibility export
is permitted. The inline Host semantic adapter and closed target-copy helper
retain only their bounded existing support, not a universal eligibility rule.

Definition backing/accounts survive independently of the first instance. Accepted
work retains its data through drain, and final release occurs once. No cache,
automatic equivalence, persistence or VM is admitted. Production reuse remains
unimplemented by this dependency migration; the Review 08 sketches do not prove
its physical funding or throughput.

## API roles

The API ownership matrix freezes public paths and exact Rust schemas. It must
project these roles without moving their behavior to another crate:

| Role | Defining owner | Required purpose |
| --- | --- | --- |
| `LogicalInteractionPlan`, `BindingCandidate`, `BindingPlanRef`, `InboundBindingPlan` | `clinkz-wot-core` | Immutable protocol-neutral plan and execution-reference values |
| Semantic plan, source, target, binding, artifact, and plan-set generation ids | `clinkz-wot-core` | Static distinctions and stale-reference rejection; they may contain the foundation-owned generic `Generation` primitive without exposing it as an interchangeable semantic id |
| Artifact envelope/ref, compatibility identity and footprint | `clinkz-wot-core` runtime SPI | Source-free data consumed by execution |
| Binding compiler input, outcome and compiler-extension trait | `clinkz-wot-planning` | Pure native preparation paired with the actual runtime registration |
| `CapabilityIndex`, `PlanCompiler`, `PlanBuildInput`, `PlanBuildOutput`, build cursor, `CompiledUriTemplate`, and `ResolvedFormTarget` | `clinkz-wot-planning` | Shared deterministic planning algorithms and resumable build surface |
| `PropertyReadPlanCompiler` and opaque `PropertyReadBuildCursor` | `clinkz-wot-planning` | Exact bounded Producer-route projection from one complete registration identity/compiler view; no arbitrary role construction |
| `CandidateFallbackPolicy`, `CandidateSkipReason`, `CandidateSkipDiagnostic`, and `CandidateSelectionDiagnostics` | `clinkz-wot-planning` | Immutable Consumer fallback policy plus bounded, typed pre-execution selection diagnostics |
| `ServerFormContributor` and form-contribution values | `clinkz-wot-planning` preparation SPI | Side-effect-free Producer form finalization input/output; unimplemented |
| Compiled plan-set record, plan-set lease, lazy-artifact slot, compiler lease, and reclaim cursor | `clinkz-wot-servient` | Aggregate lifecycle and mutable runtime ownership; these records are not binding APIs |
| Concrete artifact payload and compiler-extension implementation | Concrete binding crate | Protocol-specific immutable planning data |

The complete preparation bundle pairs the compiler extension with a validated
Core runtime registration. The Binding SPI specification owns its construction
semantics; Planning owns the preparation API. Planning consumes its immutable
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
