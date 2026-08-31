# 0063 Consumer Plan-Set Handoff Closure

Status: DECIDED

Kind: architecture decision and authority-migration proposal

Decision baseline: `86e4c67628e16d59e70e9943978965e34744d714`

Target: the smallest WP-200 -> WP-400 handoff that can publish one v5.1
Consumer Property Read generation without Servient TD interpretation or a
second binding execution path

## Decision effect

This decision was reconstructed after PR #60 completed Core-mediated Consumer
result sealing. It replaces the unresolved candidate set previously retained
in this topic. Closed PRs #56 and #57 remain useful counterexamples, but none of
their proposed carriers, staging types, per-registration pins, reservation
barriers, or fixture layouts are adopted by default.

This workspace decision is not active implementation authority. Until the
accepted conclusions are migrated into the registered specifications and work
packages, it does not:

- admit production source in Foundation, TD, Planning, Core, or Servient;
- reopen or supersede the completed WP-200 exact-coordinate tranche;
- change the source or completion status of the finished WP-300 Consumer
  binding tranche; the selected migration instead requires one narrow
  successor correction for its request identity;
- admit WP-400 Consumer implementation;
- register or pass the Consumer Property Read architecture gate; or
- change the v5.1 requirement set or roadmap milestone state.

No new design revision or ADR is required. The decision refines active
`DOC-RUNTIME-001`, `ADMIT-TXN-001`, `ADMIT-MEM-001`,
`CONSTRAINED-WORK-001`, `API-HOT-ID-001`, `PLAN-COST-003`,
`PLAN-REQUEST-001`, `PLAN-SET-001`, `PLAN-ARTIFACT-001`, and the existing v5.1
Consumer one-shot identities without reclassifying a requirement. The temporary
one-registration first-slice restriction belongs in work-package authority,
not in a durable cross-domain ADR.

## Current facts that control the decision

1. The completed WP-200 Consumer compiler deliberately owns one exact
   `(property name, property-form index)` coordinate and returns one logical
   plan, eager artifact envelope, and compact artifact reference. Its explicit
   coordinate semantics remain correct and useful as a leaf compiler.
2. `PlanBuildInput::new` still accepts `&Thing` under the name `validated_td`.
   Neither its type nor its constructor proves Basic validation, source
   footprint, resource applicability, or charged validation work.
3. The current exact compiler uses `WorkClass::BindingPolls` as its only
   progress allowance. That class correctly belongs to binding compiler/call
   progress; it does not accurately name aggregate enumeration, material
   construction, index sealing, or reconciliation.
4. The completed WP-300 registration now exposes only Core-sealed Consumer
   execution. Both Host and application-static paths validate complete
   registration/request/artifact identity and preserve every normal and late
   terminal result through Core validation.
5. A complete Host registration already contains its compiler and live client
   execution owner and is `Send + Sync` through its erased components. A
   complete static registration already contains the matching concrete
   compiler, server, client, and request-slot layout. No new WP-300 execution
   pin is necessary.
6. The current target Servient builder installs at most one complete Property
   Read registration. General multi-binding Consumer indexing is not required
   by the first v5.1 gate and `PLAN-INDEX-001` remains inactive.
7. The current public `consume(Thing)` path is synchronous, retains the TD for
   the application/legacy facade, and routes Property Read through legacy
   call-time Form scanning. The target path must replace only the admitted
   Property Read edge; unrelated legacy capability migration remains staged.
8. The accepted plan-set lifecycle and its ownership split are already
   sufficient: Planning owns semantic construction and an opaque sealed draft;
   Servient owns reservation, generation, publication, pins, operations,
   draining, and reclamation.
9. `OutboundRequest` currently owns a `ThingId`, whose `Clone` performs a deep
   `String` copy. The aggregate can retain the human-readable identity in its
   immutable source/plan/diagnostic storage, but the repeated-call path cannot
   construct the current request without cloning that static name. Core already
   owns the fixed-width generation-bearing `ThingSlotId`; no new identity type
   is needed.
10. `plan_compile_work_units_per_step_max` limits only one caller-driven step.
    A compiler-provided `BindingCompilerBounds::work` is not a policy ceiling:
    without a profile-owned per-coordinate cap and an admission-lifetime total,
    a fresh step budget can be supplied indefinitely and a declaration such as
    `u64::MAX` remains nominally valid.

These facts rule out both extremes: a singleton fixture coordinate cannot
back the existing `read_property(name, options)` surface, while a general
multi-binding capability index, fallback engine, lazy cache, or reusable
registration-generation system would solve capabilities that the first slice
does not need.

The decisive executable boundary is the current
`planning/src/{lib,property_read}.rs`,
`core/src/{binding,binding_compiler,identity,outbound}.rs`,
`foundation/src/budget.rs`, `docs/resource-limits.csv`,
`servient/src/{builder,servient,handle,property_read}.rs`, and TD validation /
default-resolution implementation, together with the passed
`docs/evidence/WP-200-consumer-property-read-planning-selection.toml` and
`docs/evidence/WP-300-consumer-property-read-binding-execution.toml` records.
The authority boundary is `API-HOT-ID-001`, `PLAN-REQUEST-001`,
`PLAN-SET-001`, `PLAN-ARTIFACT-001`, `PLAN-COST-003`,
`ADMIT-TXN-001`, `ADMIT-MEM-001`, and `CONSTRAINED-WORK-001` as projected by
the current Planning, runtime-safety, Foundation, and exact work-package
documents. A historical proposal statement that cannot be derived from those
sources has no weight in this decision.

## Selected boundary

The first Consumer aggregate is all readable Property Read coordinates from
one Basic-validated owned `Thing`, compiled eagerly through exactly one
complete Consumer-capable Property Read registration.

```text
owned Thing
  -> Servient private Thing slot + plan-set generation + input envelope
  -> TD-owned Basic validation + conservative source footprint
  -> Planning preflight for the one-registration aggregate
  -> Servient aggregate-capacity reservation
  -> Planning materialization + all-coordinate compiler-bounds barrier
  -> sequential eager compilation and aggregate seal
  -> Servient Frozen record
  -> one atomic Published consumed generation

read_property(name, options)
  -> plan-set lease carrying fixed-width ThingSlotId
  -> Planning-owned target lookup in the sealed draft
  -> exact plan/candidate/artifact selection
  -> retained complete registration
  -> OutboundRequest::property_read(ThingSlotId, ...)
  -> Core-sealed Host/static WP-300 execution
  -> terminal call settlement and plan-lease release
```

The aggregate is broader than the completed one-coordinate WP-200 output, but
narrower than the general Planning design. It has no second binding candidate,
automatic fallback, lazy artifact, cache, credential provider, subscription,
collection operation, or production-protocol claim.

The cross-crate semantic carrier graph has only three new owned values:

| Handoff | Minimum owned value | Content and exclusion |
| --- | --- | --- |
| TD -> Planning/Servient | opaque `ValidatedThing` | The one Thing plus validation/census facts; no public unchecked constructor or mutable Thing projection. |
| Planning preflight -> Servient reservation | opaque `ConsumerPropertyReadPreflight` | Checked shape and conservative reservation request; no TD borrow, registration owner, plan id, artifact, or runtime lease. |
| Planning build -> Servient publication | opaque `ConsumerPropertyReadDraft<A>` | Sealed plans, targets, candidates, artifacts/refs, lookup, and exact ledger; no TD borrow, binding execution object, or publication state. |

The preflight and build cursors are move-only progress values that return their
owned partial state on `Pending`/failure; they are not additional lifecycle
owners. After publication, lookup returns only a fixed selection containing
the target value and generation-checked row/artifact/registration coordinates.
The Servient resolves those coordinates while holding the plan-set lease; the
selection does not copy an artifact payload or expose an unleased raw pointer.
The lease supplies the existing `ThingSlotId`; neither the selection nor the
request reconstructs it from a human-readable Thing name. `ThingSlotId` is an
already-owned Core value, not a fourth semantic handoff carrier.
Exact Rust spellings may be adjusted by the authority diff, but adding another
semantic carrier requires a demonstrated ownership need.

## Validated input boundary

The raw `&Thing` spelling is not an admissible aggregate handoff. The required
predecessor output is an opaque move-only validated-Thing owner with these
semantics:

- it owns the exact `Thing` supplied to Consumer admission;
- only the TD crate can construct the successful state;
- construction proves `ValidationLevel::Basic` over the complete typed Thing;
- construction records a conservative physical source footprint plus the
  fixed-size counts/maxima needed to check every applicable current resource
  row;
- validation is resumable for the application-static profile and charges the
  caller's unique `WorkBudget` before each typed document-node or schema-node
  visit, using its matching work class;
- Host `consume` may drive the same pure cursor synchronously within the
  admitted maximum; it does not use a different validator;
- cancellation or validation failure retains the first cause and releases or
  returns the owned input without publishing any generation; and
- after success, Planning receives only an immutable borrow of this validated
  owner, never a raw `Thing` plus a caller assertion.

The source footprint is representation-aware. Serialized length and
`size_of::<Thing>()` are not sufficient for a caller-constructed value with
heap capacity and associative-container overhead. The TD-owned census must
either produce a proved conservative upper bound for the exact supported
representation or reject admission; it does not normalize or clone the Thing
merely to make accounting easier. Any future normalization alternative would
have to reserve both representations in peak-live accounting and is outside
this decision.

Typed TD traversal cannot be charged as parsed JSON merely because both inputs
describe a document. Authority migration therefore adds one
`WorkClass::DocumentNodes` counter for representation-neutral typed document
validation/census. `JsonSchemaNodes` retains parsed-JSON/schema work and cannot
be used for aggregate Planning. This is the smallest accurate validation-work
split.

The consumed generation retains the same validated Thing as an explicitly
charged source/application view. This preserves `thing_description()` and
staged legacy capability compatibility without cloning the TD. The target
Property Read lookup and execution path has no access edge back to that view.
The Planning draft remains fully TD-lifetime-free. Later removal or reduction
of retained source is a separate facade decision, not a prerequisite for the
first gate.

This validated input is a smaller predecessor because its owner, failure
boundary, and evidence differ from aggregate Planning and Servient runtime
execution. It should be admitted and implemented before the aggregate WP-200
tranche rather than hidden inside WP-400.

## Aggregate construction and lookup

Planning performs one deterministic preflight before Servient reserves final
storage:

1. Require the admitted human-readable Thing identity. Any owned `ThingId`
   projections remain admission-time immutable plan or diagnostic data and are
   fully accounted; they are never used to construct a per-call request.
2. Visit `Thing::properties` in the existing `BTreeMap` key order.
3. Visit each property's Forms in retained source order and use TD-owned
   effective-operation defaulting.
4. Retain every Form whose effective operations contain `ReadProperty`.
5. Require at least one retained coordinate in the whole aggregate.
6. Require the effective security of every retained coordinate to be exactly
   one locally resolved `NoSec` definition with no credential/provider or
   binding-carried material.
7. Measure target rows, plan rows, owned strings, candidates, artifact
   references, lookup material, diagnostics, bounds metadata, and
   reconciliation state with checked arithmetic, and derive a checked upper
   bound for every later phase's `PlanningItems` charges in this admission. The
   cursor reduces the applicable phase bound as those charges occur, so the
   all-compiler-bounds barrier compares only still-unperformed Planning work.

Every declared property receives one target entry with one shared
`AffordanceTarget::Property` identity. A request clones that already-owned
shared target identity; it does not allocate or copy the addressed name on the
hot path. It also copies the plan lease's fixed-width `ThingSlotId`, never a
`ThingId`. A property with no readable Form has an explicit empty plan range;
lookup therefore distinguishes a
missing property (`AffordanceMissing`) from an existing property with no
readable Form (`NoFormSupportsOperation`) without consulting the TD. A TD with
no readable Property Read coordinate fails admission and publishes no handle.

After reservation, Planning traverses the same immutable validated owner and
materializes every logical plan and candidate before compiler progress begins.
There is one final row per retained coordinate. Each row contains or resolves
exactly:

- one `LogicalInteractionPlan`;
- one `BindingCandidate` using registration ordinal `0` and candidate order
  `0`;
- one eager `ConsumerCall` artifact envelope;
- one aggregate-local `BindingArtifactRef`; and
- one compact plan/candidate/artifact join.

The existing exact-coordinate compiler remains the behavioral leaf, but the
aggregate does not nest its current `PlanCompiler::step` state machine because
that state machine calls `bounds` and `start` back-to-back. Planning instead
extracts and reuses a crate-private coordinate-preparation kernel: both paths
construct the same logical plan/candidate and use the same registration
projection, while the aggregate can collect every `bounds` result before any
`start`. This is an internal WP-200 refactor with regression evidence, not a
second public compiler API or a change to exact-coordinate semantics.

The aggregate stores target rows in property-key order and plan rows contiguously
in source Form order for each target. Omitted `form_index` selects the first
row in the addressed non-empty range. An explicit `form_index` must equal an
original Form-array index inside that addressed range. A sorted-table binary
search (or an equivalent bounded static index) finds the target; only its own
bounded Form range is then examined. Lookup never scans an unrelated target,
the TD, or a registration collection.

Every readable coordinate is mandatory. Compiler rejection or failure for any
one coordinate fails the entire unpublished aggregate. Publishing a successful
subset, silently skipping a non-NoSec coordinate, or choosing an arbitrary
first property is nonconforming.

The Planning output is one opaque sealed Consumer Property Read draft. Its
private physical layout may use parallel arrays or aggregate rows, but it must
provide checked lookup/resolution operations and an exact `PlanFootprint`.
Servient does not reconstruct candidates, rewrite artifact slots, rescan target
names, or repeat the seal invariants.

## Registration and execution-owner retention

The finalized Servient contains exactly one complete target Property Read
registration. The current builder's singular slot may be replaced before
`build`; only its final value is installed. That construction-time replacement
does not create a second registration ordinal or runtime replacement
lifecycle. A target Consumer admission with no finalized registration fails
without publication. Legacy binding collections may coexist for capabilities
not yet migrated, but they are not part of the target snapshot and cannot be
reached by target Property Read.

The single complete registration is the one-element startup snapshot:

- Planning receives its identity and compiler projection from that same owner;
- explicit installation is the first-slice owner selection, so Planning does
  not run a capability index or support probe; every exact compiler `bounds`
  result must accept its materialized plan, and the complete set of bounds must
  pass the aggregate work barrier before any compiler `start`;
- every candidate records ordinal `0` plus binding id/generation,
  configuration digest, compatibility, and candidate order `0`;
- the Frozen record retains that complete registration owner for its entire
  lifetime; and
- execution resolves ordinal `0`, compares the complete candidate, plan,
  artifact, request, and registration identities, then calls only the
  WP-300 sealed complete-registration operation.

No separately allocated `BindingRegistrationSnapshot` container is required
for this slice: the retained link to the one complete registration is the
entire snapshot semantics. There is no per-plan registration pin,
selected-client token, public raw client projection, or runtime binding scan.

For Host, the Servient startup configuration and every live consumed
generation share one `Arc<HostBindingRegistration>`-equivalent owner. A call
owner retains its plan-set lease; if cleanup ownership transfers, the complete
decorated call and that lease transfer together.

For application-static, one caller-owned root contains the concrete complete
registration, aggregate record, build/reclaim cursor, and Consumer request
slots. Plans and calls retain generation-bearing indices, not references into
the root. Progress obtains short `&mut` borrows from the root, so the design
requires no `Arc` ownership for the registration or plan-set record, no
self-reference, and no interior-mutable registration pins. This does not
redefine existing shared value fields such as the `Arc<str>` inside
`AffordanceTarget`.

## Identity and generation

At Consumer admission entry, Servient reserves one private plan-set build slot
and the next value from its Servient-local, non-wrapping Consumer
`PlanSetGeneration` allocator before validation work. The slot/generation pair
owns the input/validation ledger from its first charge and becomes the same
record identity if publication succeeds; no separate admission generation is
introduced. A plan-set generation is never issued to a second Consumer
generation in the same Servient, including after validation, preflight, build,
or publication failure and after reclamation. This decision does not replace
the accepted Producer identity allocator; role and exact plan-set ownership
keep the two paths distinct.

```text
AdmissionLedger.owner = (record slot, PlanSetGeneration.get())
ThingSlotId.slot       = record slot
ThingSlotId.generation = PlanSetGeneration.get()
```

The `ThingSlotId` is the fixed-width identity of this consumed Thing record.
It is allocated once with the private record capability, retained by the
published record/lease, and copied into calls. It is not derived from or looked
up by `ThingId`. The human-readable `ThingId` remains only in admitted
immutable source, plan, or diagnostic storage where the existing Core logical
plan API requires it.

Every plan id in that aggregate is derived, not independently allocated:

```text
PlanId.slot       = dense zero-based plan-row ordinal
PlanId.generation = PlanSetGeneration.get()
```

The Rust wrappers remain distinct types; equality of their underlying
generation values is an invariant of this aggregate, not interchangeability.
All artifact identities, references, target ranges, selections, requests, and
diagnostics resolve under the same retained plan-set owner.

The narrow Core successor changes only the selected-request Thing field:

```rust
impl OutboundRequest {
    pub fn property_read(
        thing_slot: ThingSlotId,
        target: AffordanceTarget,
        artifact: BindingArtifactRef,
        uri_variables: BTreeMap<String, String>,
        deadline: Option<Deadline>,
    ) -> CoreResult<Self>;

    pub const fn thing_slot(&self) -> ThingSlotId;
}
```

The constructor rejects a `thing_slot.generation()` that differs from
`artifact.plan_set_generation().get()` and rejects an artifact whose
`PlanId::generation()` differs from the same plan-set generation. It retains
the existing Property target and `ConsumerCall` role checks. It has no
`ThingId` parameter, field, accessor, conversion, or fallback lookup. Existing
Host/static result sealing continues to derive binding and plan identity from
the same artifact reference; no sealing algorithm or execution-owner API
changes.

The exact plan-set owner/lease is still required before resolving a raw
plan/artifact reference, but uniqueness does not rely only on that private
pointer discipline: two Consumer generations in one Servient cannot
accidentally produce equal artifact identities from the same dense plan slot
and binding identity.

Any failed admission spends its reserved plan-set generation. Terminal
settlement releases the private Thing/record slot, and any reuse stamps that
slot with a later plan-set generation. The storage slot has no second
generation allocator: a private Host/static record capability is exactly the
`ThingSlotId` whose generation equals the record's `PlanSetGeneration`.
Consumer allocator exhaustion rejects new admission; it never wraps. A static
root owns the same one counter and bounded record slots in caller-provided
storage.

This removes the historical need for independent PlanId-generation and
storage-slot-generation allocators while preserving stale-reference rejection.

## Reservation, resource, and work accounting

The minimum transaction uses one semantic preflight/reservation barrier, not
the two general-purpose barriers proposed in prior candidates:

```text
reserve private build slot, plan-set generation, and generic input/validation envelope
  -> validate and measure source
  -> Planning preflight and exact aggregate shape
  -> reserve persistent shape, compiler memory ceilings, and reclaim state
  -> materialize all logical plans/candidates
  -> collect and admit every compiler bound; no compiler has started
  -> drive eager compilers sequentially
  -> reconcile actual ledger and release unused capacity
  -> seal -> Frozen
  -> final cancellation check -> atomic publication
```

The private slot/generation owns the entry envelope, which is the existing
admission safety boundary for an untrusted owned input and validation scratch;
it is not a second Planning -> Servient shape handshake. An over-limit
caller-constructed Thing is returned or dropped without registry publication,
the private slot is released, and its generation remains spent. Successful
census reconciles the conservative source charge before the aggregate
reservation, and peak-live accounting keeps the retained source,
validation/preflight scratch, and later final allocation overlap explicit.

Final plan/index allocation cannot begin before the shape reservation. After
all logical plans and candidates exist, Planning calls the pure `bounds`
operation exactly once for every coordinate and stores its fixed metadata. It
does not call any compiler `start` until the last coordinate has passed all of
these checks:

- artifact, cursor, and temporary declarations fit the already held per-item,
  per-Thing, and applicable global ceilings;
- every non-`BindingPolls` work counter is zero;
- the declared `BindingPolls` count is nonzero and no greater than the
  profile's Consumer per-coordinate compiler-work limit;
- checked addition of all coordinate declarations does not overflow; and
- that sum plus the preflight-proved upper bound for all remaining
  `PlanningItems` fits the unspent profile-owned Consumer admission-work total.

Failure at this barrier is a structured limit or compiler-contract error with
zero `start` calls and zero published state. A compiler does not need a second
Servient callback merely to reserve its exact declared memory amount; the
transaction debits it from the already held aggregate ceilings. After the
barrier, each stored declaration becomes that coordinate's non-resettable
progress allowance. Limit diagnostics name the exact row, configured and
requested/observed value, coordinate when applicable, and admission phase;
checked-sum overflow is attributed to the admission-total row.

The first slice obtains the memory ceilings without invoking a compiler:
checked preflight counts reserve the configured worst-case retained-artifact
ceiling for every coordinate and only one live compiler cursor/temporary
ceiling because compilation is sequential. The later all-bounds barrier may
release unused retained capacity, but it never requests more. This may
conservatively reject an aggregate whose binding would have declared smaller
artifacts, but it avoids a second Planning -> Servient handshake. A later
tighter admission algorithm is not part of this decision.

The ledger keeps these accounts distinct:

- source/input: the one owned validated Thing and its validation facts;
- phase temporary: validation/preflight state, the current compiler cursor,
  compiler temporary ceiling, and bounded failure-settlement cursor;
- persistent document: the explicitly retained Thing view after publication;
- persistent runtime: plan-set record, targets, plans, candidates, joins,
  artifacts, references, one registration-owner link, plan leases/pins, and
  reclaim metadata;
- diagnostics: the bounded first failure or selection diagnostic capacity; and
- cleanup: only real post-acceptance call/cleanup owners. Pure pre-publication
  compiler abort contributes zero external cleanup records.

The unchanged owned Thing moves from the source account to the persistent-
document account by checked ledger reclassification; it is not reserved as if
a second copy existed. If the current `AdmissionLedger` cannot express that
account transition without changing `live_bytes`, the WP-100 predecessor adds
one narrow ownership-preserving transfer operation. The destination limit is
checked before the source charge changes, and failure leaves the source charge
and owned Thing available for ordinary rollback.

Peak simultaneously live bytes and largest contiguous allocation are checked
against the physical Host or static representation. Shared registration bytes
are charged once at startup; each plan set charges only its registration-owner
link and record slot, not a fictitious copy of the registration.

The decision adds exactly two work classes, appended without changing existing
discriminants:

- `DocumentNodes` charges representation-neutral typed TD validation/census;
- `PlanningItems` charges before each registration/target/Form visit
  attributable to Planning, each pure compiler-bounds invocation, aggregate row
  construction, lookup sealing, invariant reconciliation, budgeted immutable
  lookup comparison, and plan-record reclamation.

URI bytes, security branches, parsed JSON/schema nodes, and cleanup keep their
existing classes. One actual binding compiler `step` callback is one
`BindingPolls` unit; validation, Planning, cleanup, and schema work cannot be
relabelled as binding polling.

The existing bounded document/form counts and
`document_validation_work_units_max` bound validation lifetime. They do not
bound aggregate compilation, and a compiler's own declaration is evidence of
need rather than policy authority. Authority migration therefore appends these
two Consumer-only rows to the exhaustive resource schema:

| Field | Scope | Enforced meaning |
| --- | --- | --- |
| `consumer_binding_compile_work_units_per_plan_max` | per-plan | Maximum `BindingPolls` a compiler may declare for one Consumer logical-plan/Form coordinate. |
| `consumer_plan_compile_work_units_per_admission_max` | per-admission | Maximum combined `PlanningItems` and `BindingPolls` over the complete Consumer plan-set admission. |

Both rows have `resource_kind=plan`, `unit=work-units`, Consumer applicability,
disabled-zero semantics, finite nonzero values strictly below `u64::MAX` in
`GatewayDefaultV1` and `BenchmarkStaticReferenceV1`, and `NA` in
`DirectoryClientDefaultV1`. The authority migration must supply explicit
reviewed values in both applicable profiles; a compiler declaration, omitted
field, `None`, or a caller budget cannot supply or enlarge either value.

`plan_compile_work_units_per_step_max` remains a third, orthogonal ceiling: it
limits the sum of `PlanningItems` and `BindingPolls` charged by one aggregate
`step` call. The cursor uses its deterministic current phase rather than a
caller-selected split. A larger caller budget grants no more than this limit;
zero applicable caller work returns `Pending` without a callback.

The aggregate cursor owns the admission-total remainder plus one declared
remainder for every coordinate. Before one Planning item, it checks and debits
the caller's `PlanningItems`, the local step remainder, and the aggregate
remainder. Before one compiler callback, it checks and debits the caller's
`BindingPolls`, the local step remainder, the coordinate remainder, and the
aggregate remainder, then supplies the compiler a fresh one-unit
`BindingPolls` child budget. That child is created only after the unique caller
budget has been debited, is non-refundable because the callback itself is the
charged unit, and cannot authorize a second poll. This is a bounded partition,
not a copied allowance.

A compiler that remains `Pending` when its declared coordinate remainder is
exhausted fails before another callback and its live cursor is aborted once.
Unused declaration is discarded on completion. Repeated Host loop iterations
or static calls may provide new per-step budgets, but they cannot replace the
cursor-owned coordinate or admission remainders. Thus an at-limit declaration
is constructible, an over-one declaration fails before every compiler `start`,
and no number of per-step refills can turn synchronous `consume` into unbounded
work. `plan_reclaim_bytes_per_step_max` continues to bound reclamation
separately.

Host target lookup is a non-incremental bounded responsibility over the sorted
target table and one addressed Form range. Application-static lookup consumes
`PlanningItems` from its caller-supplied step budget. Neither form may hide a
global plan, registration, or TD scan; authority migration must add a distinct
lookup ceiling only if the existing affordance/Form limits cannot prove this
bound on the executable representation.

The sealed draft carries the completed ledger. Servient compares its generation
and ledger to the held reservations and commits them; it does not rescan,
recount, remeasure, or reinterpret Planning output.

Before one binding call is accepted, WP-400 must additionally reserve the
operation slot, declared Host-call or static-slot footprint, result capacity,
cleanup owner, deadline/cancellation state, and one plan pin. These runtime
charges are not aggregate-build charges and remain owned by the later WP-400
tranche.

## Cancellation, failure, and publication

Validation, preflight, materialization, compilation, reconciliation, and
settlement are private `Building` phases. They do not add public plan-set
states.

Cancellation is checked before every external/compiler callback, at bounded
Planning intervals, immediately before `Frozen`, and at the publication
linearization point. On failure or cancellation, the transaction:

1. retains the first cause;
2. starts no new compiler work;
3. passes the exact live cursor to its matching compiler `abort` once;
4. drops completed unpublished artifacts and partial aggregate rows outside
   Servient locks;
5. releases source/temporary/persistent/diagnostic reservations idempotently;
6. releases the registration and plan-set leases;
7. releases the unpublished record slot while leaving its reserved plan-set
   generation spent; and
8. exposes only terminal `Failed`, with no handle or partial lookup entry.

The Host first-slice `consume` remains a bounded synchronous construction call.
Its only pre-return external cancellation source is Servient shutdown; there is
no invented admission future or public cancellation token. The implementation
checks the existing shared shutdown authority between bounded steps and before
publication.

The static root owns cancellation directly through its exclusive mutable
control path. `begin_destroy()` or the equivalent pre-publication close records
the first cause, and later `step` calls settle the owned transaction. It does
not borrow an immutable cancellation view from itself and does not assume a
Host shutdown object exists.

After publication, handle/root close transitions the record to `Draining`
before rejecting new plan pins. Already admitted Host call owners or static
slots retain the generation through Core-sealed terminal settlement. Reclaim
begins only when pins, calls, and cleanup owners are terminal and progresses
under `PlanningItems` plus the configured reclaim-byte ceiling.

## Host and static publication ownership

Host publication installs one complete record into the consumed-generation
registry and creates the returned handle in one exclusive transition. The
handle and operation owners share the record; no reader can observe `Building`
or `Frozen`. Registration/compiler/binding callbacks and artifact destruction
run outside registry locks.

Application-static publication changes the caller-owned root from `Frozen` to
`Published` only after the same seal and final cancellation check. A returned
generation token or facade is an index/generation capability into that root,
not an independently owning pointer. The application supplies storage and
drives progress explicitly.

The two representations must produce equal semantic plans, selections,
requests, failures, and terminal outcomes for equal inputs. They need not share
the same container, cancellation primitive, synchronization mechanism, or
public lifecycle API.

## Required admission decomposition

The architecture is decided, but implementation must be split at four real
ownership/evidence boundaries:

1. **WP-100 Consumer admission-primitives/validated-TD predecessor** — TD-owned
   Basic validation, move-only validated owner, conservative source
   footprint/census, bounded Host/static progress, cancellation,
   `DocumentNodes`/`PlanningItems`, the two Consumer compile-total resource
   rows, and the narrow ledger transfer if required by implementation.
2. **WP-200 Consumer aggregate draft** — deterministic preflight, one
   registration, all readable coordinates, NoSec predicate, dense shared
   generation, all-bounds-before-start admission, aggregate construction,
   indexed lookup, non-resettable work remainders, exact ledger, cancellation,
   and sealed TD-free draft. It depends on both the new WP-100 predecessor and
   the completed exact-coordinate WP-200 tranche.
3. **`WP-300-CONSUMER-HOT-THING-IDENTITY`** — a narrow Core successor to the
   completed Consumer binding tranche. It replaces only the `OutboundRequest`
   Thing field/constructor/accessor with `ThingSlotId`, adds the generation
   join checks, updates all three Core feature-cell request/binding tests, and
   re-runs result-sealing regressions under `API-HOT-ID-001`,
   `PLAN-REQUEST-001`, `PLAN-ARTIFACT-001`, and `BIND-OUT-001`. It needs no
   Planning aggregate or Servient implementation and may complete independently
   of items 1-2.
4. **WP-400 Consumer Property Read runtime** — reservation, Thing/plan-set
   owner, publication, handle/static facade, registration retention, call
   owners, plan pins, Core-sealed execution, cancellation, drain, and reclaim.
   It depends on both the new aggregate WP-200 tranche and the hot-Thing
   identity correction; result sealing remains supplied by the completed
   WP-300 Consumer binding tranche.

This is not a reason to split 0063 into more unresolved architecture topics:
the handoff semantics above are one coherent decision. The extra WP-300
predecessor is nevertheless a real smaller implementation decision because
the current public Core request type itself makes the desired hot path
unconstructible, while its correction can be implemented and falsified without
WP-200 aggregate or WP-400 lifecycle code. TD/Foundation and aggregate
Planning likewise must not be hidden inside a monolithic WP-400 implementation.

The completed `WP-200-CONSUMER-PROPERTY-READ-PLANNING` tranche remains current:
its exact-coordinate output and immutable singleton selection are unchanged.
The new aggregate is additive composition and must retain regression evidence.
The completed `WP-300-CONSUMER-PROPERTY-READ-BINDING` tranche is reaffirmed for
registration, ownership, cancellation settlement, and Core result sealing.
Its historical `ThingId` request-field projection is superseded only by the
new successor's completion evidence; the prior evidence file remains immutable
history and is not rewritten. No registration or result-sealing algorithm is
reopened.

## Authority-migration preparation

One reviewed docs-only migration should project this decision into the existing
owners before any new production source is admitted:

| Owner | Required migration |
| --- | --- |
| `docs/spec/foundation.md` | Add `DocumentNodes` and `PlanningItems` without changing existing discriminants, freeze the per-step/per-coordinate/per-admission hierarchy and one-unit pre-debited compiler partition, and freeze the narrow source-to-persistent ledger reclassification if current accounting cannot express it. |
| `docs/spec/interaction-core.md` | Replace the Consumer request's human Thing name with `ThingSlotId`, require its generation join to the selected plan set/artifact, and retain the no-hot-name-copy invariant. |
| `docs/spec/binding-spi.md` | Project the corrected selected-request signature through Host/static complete registrations while explicitly reaffirming the existing private result-sealing paths. |
| `docs/spec/runtime-safety.md` | Freeze the move-only validated input, explicit retained-source charge, reserve-build-publish ordering, and profile-specific cancellation ownership. |
| `docs/spec/planning.md` | Freeze the one-registration Consumer aggregate, deterministic coordinate set, target ranges, shared Thing/plan-set generation rule, all-bounds-before-start barrier, non-resettable coordinate/admission work totals, sealed draft, lookup, ledger, and no-subset failure semantics. |
| `docs/architecture/10-primary-data-flows.md` | Project the exact Consumer admission and selected execution flow. |
| `docs/architecture/20-module-boundaries.md` | Project TD ownership of validated input, Planning ownership of the sealed draft, and Servient ownership of publication/runtime state. |
| `docs/architecture/30-compiled-plan-lifecycle.md` | Project the `ThingSlotId`/record-slot/plan-set generation relation, source/registration retention, publication, drain, and reclaim rules. |
| `docs/architecture/50-servient-runtime-lifecycle.md` | Project Host shared-registration ownership and static root ownership without merging their physical APIs. |
| `docs/api-ownership.csv` | Update `OutboundRequest` ownership/signature and register only the exact ledger-transfer (if needed), validated-input, preflight/progress, aggregate-draft/selection, generated resource fields, and Servient facade items selected by the migration; do not pre-register a new plan-set identity, general indexes, or per-entry pins. |
| `docs/resource-limits.csv` and named profiles | Append `consumer_binding_compile_work_units_per_plan_max` and `consumer_plan_compile_work_units_per_admission_max`; assign explicit finite Gateway/static values and Directory non-applicability, then project them through generated `ResourceLimits`/named profiles and schema-count/order validation. |
| `docs/work-packages/WP-100-core.md` | Define the admission-primitives/typed-TD predecessor, the resource-row implementation paths, and its evidence boundary. |
| `docs/work-packages/WP-200-planning.md` | Define the additive aggregate-draft tranche, crate-private coordinate-kernel reuse, all-bounds barrier, work-limit enforcement, and exact-compiler regression boundary. |
| `docs/work-packages/WP-300-bindings.md` and `docs/work-packages/WP-300-consumer-property-read-binding-admission.md` | Admit `WP-300-CONSUMER-HOT-THING-IDENTITY` as a successor whose only production path is `core/src/outbound.rs`; update request/binding tests and preserve complete-registration and result-sealing semantics. |
| `docs/work-packages/WP-400-servient.md` | Define the later runtime tranche without admitting it in the decision PR. |
| `docs/work-packages/index.toml` | Register the four exact tranche boundaries and make WP-400 depend on both the aggregate WP-200 tranche and the hot-Thing WP-300 correction. |
| completion evidence | Keep `docs/evidence/WP-300-consumer-property-read-binding-execution.toml` immutable; require a distinct hot-Thing identity completion record that covers the corrected request schema and re-runs the sealing matrix. Require aggregate evidence to cover both work-limit rows and the no-start barrier. |

`PLAN.md`, the active requirement count, the Producer Property Read gate, and
the historical WP-300 completion record do not change in this decision PR. The
authority migration may update the coarse Consumer dependency wording in
`PLAN.md` only if the registered four-boundary DAG is otherwise ambiguous; it
must not turn the roadmap into a tranche tracker. A new Consumer architecture
gate is registered only with the later WP-400 implementation/evidence
candidate, not as empty ceremony during authority migration.

## Rejected alternatives

### Preserve #56 or #57 as the base design

Rejected. They use more stages and evidence-specific carriers than the current
one-registration slice needs. In particular, a general registration snapshot,
per-entry execution pin, independently allocated plan generations, two
Servient-to-Planning reservation handshakes, or discarding the only TD view
before the staged facade can use it are not required by current authority.

### Publish one arbitrary or caller-hidden coordinate

Rejected. The existing `read_property(name, options)` surface must distinguish
all declared properties and explicit Form indexes without a TD scan. A
singleton fixture would require an arbitrary build-time choice or a different
public API and would not close the aggregate handoff.

### Activate general multi-binding indexing or fallback

Rejected. The target builder already has one complete registration and the
first gate excludes multi-binding fairness, fallback, and `PLAN-INDEX-001`.
Keeping only the finalized singular builder slot is smaller and explicit.

### Retain only artifact identity and look up a live binding later

Rejected. Identity is not an execution owner. The exact complete registration
must remain reachable for the generation lifetime; a later binding scan could
select a different code/configuration owner.

### Keep `ThingId` in `OutboundRequest` or make it reference counted

Rejected. The owned `String` forces a deep copy per request. Changing the
human-readable identity globally to `Arc` would avoid byte copies but still
would not provide a stale-safe record/plan-set coordinate and would widen an
unrelated Core identity representation. The existing `ThingSlotId` exactly
matches the required fixed-width record capability; the human name remains in
immutable admitted storage for API and diagnostics.

### Add a new Consumer plan-set reference type

Rejected. The selected record already has one Thing slot and one
`PlanSetGeneration`. Storing that same pair in another public wrapper would add
an identity join without distinguishing another lifecycle owner.

### Give every plan or registration an independent lifetime pin

Rejected. One immutable plan-set owner and one complete-registration owner
already dominate all selected rows. Per-entry pins duplicate lifetime state and
are particularly harmful to static ownership.

### Let Servient concatenate singleton outputs and build the lookup

Rejected. That would make Servient interpret target/Form structure and repeat
Planning invariants. Planning must return the already sealed aggregate draft.

### Retain no TD at all in the first consumed generation

Rejected for this staged slice. It would force an unrelated
`thing_description()`/legacy-facade migration or a duplicate source copy.
Explicitly retaining and charging the one owned validated Thing is simpler;
the target Property Read path is still structurally unable to scan it.

### Charge aggregate work as binding, cleanup, or schema work

Rejected. `BindingPolls` remains binding progress, `CleanupItems` remains
cleanup ownership, and `JsonSchemaNodes` remains parsed-JSON/schema work.
`DocumentNodes` and `PlanningItems` are the minimum accurate additions because
typed TD validation and aggregate plan construction have different owners and
units.

### Treat the per-step limit or compiler declaration as the total work bound

Rejected. A per-step limit can be replenished indefinitely, while a compiler
declaration is untrusted demand and can be `u64::MAX`. The two profile-owned
total rows and cursor-owned remainders are required even though the current
exact compiler normally needs one poll.

### Start each coordinate immediately after obtaining its bounds

Rejected. A later coordinate can make the checked sum overflow or exceed the
admission total after earlier compilers have already started. Materializing the
plans and storing their small bounds records permits one deterministic barrier
with no second Servient reservation callback and only one live compiler cursor.

## Falsifiable migration and implementation boundary

Independent authority review and later implementation evidence must be able to
falsify all of the following:

- no aggregate build can be entered with a raw `Thing` or forged validation
  proof;
- validation/source footprint and work remain bounded under Host and static
  progress, including heap-capacity/container overhead, cancellation, and
  one-over-limit inputs;
- a multi-property, multi-readable-Form TD produces every coordinate in exact
  key/Form order, including explicit empty target ranges;
- a non-first property and non-first explicit Form index resolve without a TD
  or registration scan;
- zero total coordinates, non-NoSec effective security, no finalized target
  registration, any attempt to expose more than the one finalized target
  registration, any compiler failure, and every checked overflow publish
  nothing;
- validation, preflight, and build failures each spend their private
  plan-set generation while releasing the reusable record slot;
- the published record's `ThingSlotId` uses that record slot and the same
  underlying generation as `PlanSetGeneration`; a mismatched Thing/artifact or
  plan/artifact generation is rejected before binding acceptance;
- plan ids are dense, use the plan-set generation, and become stale together
  after failed-build settlement or reclamation without wraparound;
- the public `OutboundRequest` construction/accessor surface accepts no
  `ThingId`; repeated requests copy only `ThingSlotId` and the existing shared
  target value, perform no human Thing-name allocation/clone, and retain every
  existing Host/static result-sealing outcome;
- compiler spies prove that all coordinates at their per-plan limit and the
  aggregate admission limit can complete, while either limit plus one, a
  `u64::MAX` declaration, or checked-sum overflow fails with zero compiler
  `start` calls and zero publication;
- a compiler that remains `Pending` after its admitted coordinate polls is
  aborted exactly once, and repeated fresh per-step budgets cannot reset either
  its coordinate remainder or the aggregate admission remainder;
- no aggregate step consumes more than
  `plan_compile_work_units_per_step_max` across `PlanningItems` and
  `BindingPolls`; zero budget invokes no bounds/compiler callback, and changing
  the valid step partition does not change the sealed output;
- the sealed draft's plan/candidate/artifact/ref/target joins are one-to-one and
  its completed ledger fits the held reservations;
- source-to-persistent reclassification neither duplicates the Thing nor
  changes total live bytes, and every failure restores exact account totals;
- Servient performs no semantic recount or index construction after the draft
  seal;
- the complete Host registration survives builder/build-input destruction
  through every call/cleanup owner, while static uses only caller-owned root
  storage and generation-bearing indices;
- the target lookup/selection type boundary accepts no `Thing`, Form, or
  registration input, and the target path succeeds with legacy support probes,
  raw installed client projection, and legacy `BindingRequest` poisoned;
- cancellation before publication leaves no plan/artifact/source/registration
  owner-link reservation, and cancellation after publication drains without
  invalidating an admitted call's plan lease;
- normal, binding-error, validation-error, cancellation, cancellation-late,
  cleanup-transfer, static manual-cleanup, and terminal reclaim outcomes retain
  the WP-300 sealing classifications; and
- Host and static traces agree semantically while retaining their distinct
  physical owners.

## Migration condition

This topic is `DECIDED`, not `MIGRATED`. A fresh independent architecture
review must accept the exact docs-only authority diff before the registered
owners or tranche DAG change. Production implementation starts only after the
corresponding predecessor/tranche admission review. Once every listed
authoritative projection is accepted and merged, this topic may move to
`MIGRATED`; it must not become a parallel current-state summary.
