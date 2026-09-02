# 0063 Consumer Plan-Set Handoff Closure

Status: DECIDED

Kind: architecture decision and executable migration/admission-path correction

Reassessment baseline: `a1029ae04fd24e1d38d58e0dc8378af192a8dd98`

Superseded decision baselines:

- `c7bb2b602b900c087f2b50cf649af8562fd8bcbd` (PR #62);
- `76170bc1d6ae5551aa72c9ea0b5b5dab84acf61a` (PR #63 source commit).

Target: the smallest end-to-end executable path from this `DECIDED` topic to
one admitted and implemented v5.1 Consumer Property Read generation, without
Servient TD interpretation, an unadmitted source exception, or a speculative
future-tranche registry.

## Reassessment verdict

The prior 0063 direction is withdrawn where it required either:

- two new global resource-limit rows and an authority-plus-generated-source
  migration outside any ADR-0013 tranche;
- simultaneous registration of four future tranches whose predecessors were
  not complete; or
- a new hot-Thing-identity successor while the already completed WP-300
  request tranche and its evidence remained current.

Those choices cannot form one legal path through current repository authority.
PR #63 fixed the first observed docs-only migration failure, but its generated-
projection exception exposed the next contradiction instead of closing the
path.

The minimum executable correction is:

1. keep the one-Thing, one-registration, all-readable-Property-Read aggregate;
2. use the existing resource schema and add only the two missing `WorkClass`
   discriminants through an admitted WP-100 source tranche;
3. remove static human-readable Thing and target identities from
   `OutboundRequest` rather than replacing `ThingId` with `ThingSlotId`;
4. reopen and readmit the existing WP-300 Consumer binding tranche for that
   public request correction;
5. migrate architecture and work-package authority without registering new
   future tranche nodes; and
6. register each new tranche only when all of its predecessor nodes are
   `complete/current`, exactly as ADR-0013 and the executable index checker
   require; then
7. register the Consumer architecture gate only after its source prerequisites
   and aggregate fixtures exist, without replacing the already passed Producer
   gate.

This remains a v5.1 conformance correction under the already active
`DOC-RUNTIME-001`, `ADMIT-MEM-001`, `CONSTRAINED-WORK-001`,
`API-HOT-ID-001`, `PLAN-COST-003`, `PLAN-REQUEST-001`, `PLAN-SET-001`,
`PLAN-ARTIFACT-001`, and `BIND-OUT-001` requirements. It does not activate a
new requirement, design revision, operation family, general capability index,
or fallback policy. No new ADR is required.

## Repository-grounded stop conditions

### Generated projection has no legal source owner

`docs/resource-limits.csv` is both the exhaustive resource authority and a
Cargo build input. Appending the two rows selected by PR #63 would necessarily
change generated public Foundation API and named-profile arrays. PR #63
therefore allowed changes to `foundation/build.rs`,
`foundation/src/resource.rs`, and `tools/check-resource-limits.sh` while saying
that the cohort was not implementation and was not completion evidence for any
tranche.

ADR-0013 instead states that every implementation change belongs to exactly
one recorded tranche. The proposed Foundation/build changes alter generated
product API, but none of the four proposed successor tranches owned them. The
coupled resource checker merely tracks that source-visible schema mutation; it
does not supply admission, predecessor closure, implementation paths, or
completion evidence. A separate projection tranche would add a fifth boundary
and still face a contract-before-code cycle around the authoritative CSV.

The executable correction is to require no new resource field for this slice.
The 195-field schema and its generated projection remain unchanged.

### `index.toml` is not a future-work DAG

The current tranche validator permits a `current` node to depend only on a
`complete/current` tranche. This is not merely a checker limitation: ADR-0013
condition 2 says an unimplemented predecessor contract cannot be used as if it
were complete.

The prior migration proposed registering WP-100, WP-200, WP-300, and WP-400
future nodes together. The WP-200 node would depend on a planned WP-100 node,
and WP-400 would depend on planned WP-200/WP-300 nodes. That graph is rejected
by the current executable admission semantics and would be non-admissible even
if the checker were weakened.

`docs/work-packages/index.toml` is the registry of exact admission and current
dependency truth, not a speculative planning manifest. Future technical
boundaries may be defined in their owning work-package documents, but a new
index node is added only by its own admission review after every predecessor is
complete.

### The current integration-gate registry is singular

The current `index.toml` has one `integration_gate_manifest`, pointing to the
passed v5.0 Producer `PROPERTY-READ-ARCHITECTURE` gate. The design checker also
validates exactly that path, schema revision, design revision, id, and fixture
set. Its dependency validation currently requires only a nonempty set of
unique, known, `covered` entries with existing evidence; it does not freeze the
Producer dependency-id set. Replacing the singular field with a Consumer
manifest would erase the current Producer registration, while merely adding a
second file would leave the Consumer gate unregistered and unchecked.

ADR-0019 and `PLAN.md` deliberately say that the Consumer gate is not
registered until its source prerequisites are assembled. The initial authority
migration therefore leaves the singular Producer registration and checker
unchanged. The later WP-400 source tranche predeclares and produces the
cross-package Host/static fixture as completion evidence. Only after all
component tranches are `complete/current` does a separate gate-registration
candidate:

- preserve the Producer manifest as the first registered gate;
- evolve the work-package schema from the singular field to an ordered exact
  manifest list and append `CONSUMER-PROPERTY-READ-ARCHITECTURE` in `ready`;
- generalize the non-normative design checker to validate both manifests and
  add manifest-specific exact dependency-id/evidence assertions rather than
  replacing or weakening its Producer assertions;
- register the new gate document/manifest in `docs/artifacts.csv` and update
  the gate-registration statement in `docs/spec/README.md`; and
- point only to already existing fixture and tranche evidence.

That candidate is an isolated governance/checker projection. It contains no
product source, generated API, fixture implementation, or tranche admission,
and it grants no functional edit authority. This follows the repository's
existing crate-level registration of `tools/design-check/Cargo.toml` as a
`non-normative-checker`, not a generated product projection; the source file
does not claim a separate artifact classification. The subsequent
`ready -> passed` transition remains a separate status-only independent
acceptance action under `PROJECT_GOVERNANCE.md`.

### The completed WP-300 tranche cannot remain current

The current `OutboundRequest` owns both a deep-cloned `ThingId` and an
`AffordanceTarget`. The latter uses `Arc<str>`, but it is still a static target
identity retained in the request. Active `API-HOT-ID-001` and
`PLAN-REQUEST-001` require human-readable names to remain at API/admission or in
immutable plan/diagnostic storage, and require static target data to remain
behind the pinned plan/artifact reference.

Changing that public request shape affects the completed
`WP-300-CONSUMER-PROPERTY-READ-BINDING` tranche and its completion evidence.
ADR-0013 requires an affected completed tranche to enter impact review and be
reaffirmed or reopened. A new successor cannot silently supersede its public
schema while the original node and evidence remain `current/passed`.

The repository already exercised the correct recovery for result sealing:
reopen the original node, supersede its evidence, independently readmit the
corrected boundary, implement it, and replace completion evidence. The hot
request correction uses the same path.

### Workspace state was already inconsistent

After PR #62, this file said `Status: DECIDED` while `workspace/INDEX.org`
continued to list 0063 under `DISCUSSING`. This reassessment restores one
workspace lifecycle classification. Workspace state does not authorize
implementation, but its own index must not contradict the topic it indexes.

## Minimum end-to-end technical boundary

The first Consumer aggregate contains every effective Property Read coordinate
from one owned Basic-validated `Thing`, compiled eagerly through exactly one
finalized complete Consumer-capable Property Read registration.

```text
owned Thing
  -> Servient private admission record and conservative retained-source charge
  -> TD-owned bounded Basic validation and representation-aware census
  -> Planning deterministic preflight and compiler-bounds barrier
  -> Servient persistent-runtime reservation
  -> Planning eager materialization and compilation
  -> Planning-owned sealed TD-free aggregate draft
  -> Servient atomic publication with retained Thing and registration owner

read_property(name, options)
  -> lease one published plan-set record
  -> lookup the addressed property and selected Form row in that record
  -> resolve one artifact and the retained complete registration
  -> construct a name-free `OutboundRequest`
  -> execute only the Core-sealed complete-registration path
  -> settle the call and release the plan-set lease
```

The first slice has:

- one Thing and one published consumed generation;
- exactly one complete registration and registration ordinal `0`;
- all declared properties represented in the lookup, including an empty range
  for a property with no readable Form;
- every effective readable Property Read Form compiled eagerly and retained in
  original per-property Form order;
- one eager `ConsumerCall` artifact per retained coordinate;
- no partial publication: any validation, security, bounds, compiler, ledger,
  cancellation, or seal failure fails the unpublished generation; and
- a NoSec-only predicate: every retained coordinate must resolve to exactly one
  local NoSec definition with no credential/provider or binding-carried
  security material.

The aggregate does not activate multiple registrations, automatic fallback,
lazy artifacts, caches, `PLAN-INDEX-001`, credentials, subscriptions,
collections, emissions, production protocol behavior, or another operation.

## Validated input and retained source

`PlanBuildInput::new(&Thing, ...)` does not prove validation. The predecessor
must therefore expose one TD-owned move-only validated input whose successful
construction proves:

- ownership of the exact input `Thing`;
- complete `ValidationLevel::Basic` validation;
- checked structural limits for the typed representation;
- a conservative representation-aware retained-source footprint and the
  counts needed by Planning preflight;
- bounded Host and application-static progress with cancellation; and
- no public unchecked constructor or mutable raw-Thing projection.

The same owned Thing becomes the retained application/source view after
publication. It is not cloned or normalized for accounting. Servient first
reserves a conservative source envelope from the existing
`retained_source_bytes_*`, document, peak-live, and largest-contiguous limits,
then reconciles unused capacity after the TD-owned census. At freeze, one
narrow checked `AdmissionLedger` operation reclassifies those same live bytes
from source to persistent-document accounting. It checks the destination limit
before changing either account and leaves `live_bytes`, peak-live bytes, and
largest-contiguous allocation unchanged. Failure leaves the source charge and
owned Thing available for ordinary rollback. This is an ownership-preserving
account transfer, not a second reservation or a second Thing representation.

Validation consumes a new appended `WorkClass::DocumentNodes`. Its
non-resettable admission-lifetime allowance comes from the existing
`document_validation_work_units_max`. Host may drive the same pure cursor to
completion synchronously; application-static callers may resume it, but fresh
per-step budgets cannot replace the cursor-owned lifetime remainder.

`DocumentNodes` covers only typed-document traversal not already owned by a
more specific existing class. Typed schema-node visits remain
`JsonSchemaNodes`; URI-template bytes and security branches remain `UriBytes`
and `SecurityBranches`. One unit is not relabelled or double charged merely
because it occurs during validation, and every class derives its allowance
from its existing applicable resource limit.

Basic TD validation currently permits an absent Thing ID, while the completed
exact Planning leaf requires one. The Consumer preflight therefore preserves
that existing distinction: a Basic-valid Thing without an ID is rejected
before compiler bounds or `start`, rather than synthesizing an ID or changing
Basic validation globally.

## Aggregate construction, lookup, and work

Planning owns both semantic enumeration and the sealed aggregate. It visits
properties in the current `BTreeMap` key order, visits each property's Forms in
retained source order, and applies TD-owned effective-operation defaulting.
The preflight records checked target/coordinate counts, the exact one-
registration projection, conservative allocation requirements, and the
remaining deterministic work needed after reservation.

After Servient reserves the declared persistent runtime envelope, Planning:

1. materializes one logical plan and candidate for every readable coordinate;
2. calls the pure compiler `bounds` operation exactly once for every
   coordinate;
3. rejects every non-`BindingPolls` compiler work declaration;
4. requires each coordinate's declared `BindingPolls` total to be nonzero and
   fit within the existing `plan_compile_work_units_per_step_max` value as a
   deliberately conservative first-slice eligibility rule;
5. completes the all-bounds barrier before the first compiler `start`;
6. drives compilers sequentially with one non-resettable coordinate remainder;
7. reconciles actual artifact and temporary footprints against the held
   reservations; and
8. seals one TD-lifetime-free aggregate draft.

Using `plan_compile_work_units_per_step_max` as a first-slice eligibility cap
does not redefine that global field as a per-plan or per-admission resource.
It means only that this narrow slice accepts compilers whose total work fits in
one configured step quantum. The maximum aggregate compiler work is therefore
checked `readable_coordinate_count * plan_compile_work_units_per_step_max`.
The coordinate count is already bounded by `forms_per_context_max` and
`forms_per_thing_max`.

Pure enumeration, row construction, lookup sealing, reconciliation, and
reclamation consume a new appended `WorkClass::PlanningItems`. Their lifetime
is bounded structurally: a monotonic cursor visits each admitted property,
Form, row, and artifact a fixed number of times. A replenished caller step
budget permits later progress but cannot revisit completed state. The existing
per-step ceiling bounds one call; structural maxima and cursor monotonicity
bound the complete admission. No new global per-plan or per-admission work row
is needed.

The lookup contains one target row per declared property and a contiguous Form
range per target. An omitted `form_index` selects the first readable row for
that property. An explicit index must match an original Form-array index in the
same property's range. A missing property returns `AffordanceMissing`; an
existing property with an empty readable range returns
`NoFormSupportsOperation`. Lookup never scans the TD, another target, or a
registration collection.

The exact-coordinate `PropertyReadPlanCompiler::consumer_call` remains the
behavioral leaf. Aggregate implementation may extract a crate-private
coordinate-preparation kernel so it can run the all-bounds barrier, but it must
preserve the completed exact-coordinate semantics and regression evidence.

## Request, identity, and execution-owner correction

The first-slice request needs no human-readable Thing or target identity. The
selected plan/artifact already owns the static target and protocol facts, while
the Servient call owner retains the plan-set lease and runtime Thing slot.

The corrected request boundary is semantically:

```rust
impl OutboundRequest {
    pub fn property_read(
        artifact: BindingArtifactRef,
        uri_variables: BTreeMap<String, String>,
        deadline: Option<Deadline>,
    ) -> CoreResult<Self>;
}
```

It retains no `ThingId`, `ThingSlotId`, `AffordanceTarget`, TD, Form,
`InteractionOptions`, registration owner, candidate list, or fallback
authority. `operation()` remains fixed to `ReadProperty`; binding, binding
generation, configuration, plan-set generation, plan id, compatibility, and
role derive from the artifact reference. Construction rejects a non-
`ConsumerCall` role and a `PlanId` generation that differs from the artifact's
`PlanSetGeneration`.

Servient owns a separate generation-bearing `ThingSlotId` for the consumed
record and a non-wrapping `PlanSetGeneration` for its plan set. Their generation
values are not required to be equal. Dense `PlanId` slots use the plan-set
generation, and all aggregate artifact identities resolve under the retained
plan-set lease. The lease, not an accidental equality between unrelated
allocators or globally unique numeric value, proves which record may resolve an
artifact.

For Host, the consumed record retains a shared owner of the one complete
registration. For application-static, one caller-owned root retains the typed
registration, aggregate record, progress state, and request slots. Both forms
reach only `start_consumer_property_read` on the complete registration; raw
client authoring SPIs remain unreachable from installed runtime state. The
existing Core result-sealing algorithm and cleanup classifications are
re-executed as replacement WP-300 evidence but are not redesigned.

## Publication, cancellation, and reclamation

Validation, preflight, bounds collection, compilation, and reconciliation are
private unpublished phases. Cancellation is checked before external/compiler
callbacks, at bounded pure-work intervals, and at the publication
linearization point. A failure or cancellation fixes the first cause, starts no
new compiler work, aborts the one live pure compiler cursor exactly once,
releases every reservation idempotently, spends but never reuses the reserved
plan-set generation, and returns no handle or partial lookup.

Successful Host publication atomically installs the complete record and returns
the consumed handle. Application-static publication changes the caller-owned
root to its published state only after the same seal and final cancellation
check. The representations share semantic plans, lookup, requests, failures,
and terminal outcomes, but do not need the same container, synchronization, or
public progress API.

Close first stops new leases/calls, then drains already admitted calls and
cleanup owners. Reclamation begins only after all leases, calls, and cleanup
owners are terminal and progresses monotonically under `PlanningItems` and the
existing `plan_reclaim_bytes_per_step_max`.

## Authority-migration boundary

The next migration candidate is docs-only. It must contain no Rust source,
resource CSV row, generated projection, functional test, or future tranche
registration.

It projects the decision into these authoritative owners:

| Owner | Required migration |
| --- | --- |
| `docs/spec/foundation.md` | Append `DocumentNodes` and `PlanningItems` without changing existing discriminants; bind validation lifetime to `document_validation_work_units_max`; freeze the narrow source-to-persistent-document ledger transfer; and record the structural/per-step derivation above without adding a resource row. |
| `docs/spec/runtime-safety.md` | Freeze the owned Basic-validated Thing, retained-representation accounting, bounded profile-specific progress, and unpublished cancellation boundary. |
| `docs/spec/planning.md` | Freeze the one-registration all-readable aggregate, deterministic lookup, all-bounds-before-start barrier, structurally bounded Planning work, sealed draft, and no-partial-publication rule. |
| `docs/spec/interaction-core.md` | Require the first-slice `OutboundRequest` to carry no human-readable Thing or target identity and to reject a mismatched plan/plan-set generation. |
| `docs/spec/binding-spi.md` | Project the name-free request through the existing complete Host/static registration paths and reaffirm Core-mediated result sealing. |
| `docs/architecture/10-primary-data-flows.md` | Project validated input -> Planning aggregate -> Servient publication -> name-free selected execution. |
| `docs/architecture/20-module-boundaries.md` | Project TD validation/census ownership, Planning semantic construction, and Servient reservation/publication ownership. |
| `docs/architecture/30-compiled-plan-lifecycle.md` | Project independent Thing-slot and plan-set generations, plan-set lease resolution, retained source/registration, drain, and reclaim. |
| `docs/architecture/50-servient-runtime-lifecycle.md` | Project Host shared-registration and application-static root ownership without merging their physical APIs. |
| `docs/api-ownership.csv` | Project the new cross-crate `AdmissionLedger` source-to-persistent-document reclassification operation plus only the validated-input, aggregate preflight/draft/selection, and Servient facade items actually required; update the request contract and removal of `OutboundRequest::thing_id`/`target`; add no resource getter, general index, registration snapshot, or per-entry pin. |
| `docs/work-packages/WP-100-core.md` | Define the unadmitted validated-Thing/work-class tranche and its exact Foundation/TD paths and evidence, including a Producer-gate impact disposition for the append-only `WorkClass::ALL` change. |
| `docs/work-packages/WP-200-planning.md` | Define the unadmitted aggregate tranche, exact-coordinate regression boundary, existing-limit derivation, and evidence. |
| `docs/work-packages/WP-300-bindings.md` and `docs/work-packages/WP-300-consumer-property-read-binding-admission.md` | Reopen the existing Consumer binding tranche for the name-free request correction and replacement evidence; record the removed constructor parameters and `thing_id`/`target` accessors in `old_api_removals`; require an explicit Producer-gate impact disposition before source because that gate registers `core/src/binding.rs` and full Core/Servient commands; do not create a successor id. |
| `docs/work-packages/WP-400-servient.md` | Define the later unadmitted runtime tranche, its two exact predecessor boundaries, and the cross-package Host/static fixture source and assertions that its completion evidence must produce before gate registration. |
| `docs/work-packages/index.toml` and current WP-300 evidence | Reopen only the already registered affected WP-300 node and mark its current evidence superseded, following the existing impact-review lifecycle. Register no new future node. |

The migration leaves unchanged:

- `docs/resource-limits.csv` and all 195 generated resource fields;
- `foundation/build.rs`, generated resource assertions, and
  `tools/check-resource-limits.sh`;
- the singular Producer `integration_gate_manifest`, its passed manifest, and
  `docs/artifacts.csv`, `docs/spec/README.md`, and
  `tools/design-check/src/main.rs`;
- the active 65-requirement set and design revision;
- ADR-0013, ADR-0015, and ADR-0019;
- `PLAN.md` milestone/frontier state;
- the completed WP-100 Consumer call-values tranche;
- the completed exact-coordinate WP-200 tranche;
- the Producer Property Read gate's current manifest/status. The migration
  records, but does not pre-judge, mandatory impact reviews before both the
  later request correction and WorkClass append. The former intersects its
  registered `core/src/binding.rs` evidence and full Core/Servient commands;
  the latter must prove its fixed `[u64; 10]` cleanup snapshot intentionally
  covers the unchanged first ten `WorkClass::ALL` entries and that Producer
  evidence never consumes the two appended Consumer classes; and
- Consumer architecture-gate registration/status.

After independent acceptance and merge of that exact docs-only migration, 0063
may move to `MIGRATED`. Future admission and implementation state then belongs
only to work-package authority, `index.toml`, source, evidence, Git/GitHub, and
CI; this topic must not become a continuation log.

## Executable tranche and admission path

Future work-package documents may name all boundaries during migration, but
`index.toml` materializes them only in this order:

| Step | Registry action | Preconditions at the registered revision | Source/evidence boundary |
| --- | --- | --- | --- |
| 1A | Independently readmit the reopened `WP-300-CONSUMER-PROPERTY-READ-BINDING` node | Its existing WP-200 predecessor is `complete/current`; the migrated request contract, replacement evidence criteria, and Producer-gate impact-review boundary are complete | Correct `core/src/outbound.rs` and affected Core tests/projections; prove the removed parameters/accessors are absent; rerun the complete Host/static sealing matrix and every intersecting registered Producer-gate command/evidence; record the exact-head impact disposition and independently reopen the Producer gate before merge if its claim is invalidated; replace the superseded WP-300 evidence |
| 1B | Register/admit `WP-100-CONSUMER-VALIDATED-THING` | WP-000 is complete; the migration is merged; pre-code checks and the Producer-gate WorkClass impact-review boundary are complete | Append the two work classes, add the narrow ledger account transfer, and implement TD-owned validated input/census in exact Foundation/TD paths; rerun the Producer fixture's ten-class cleanup-prefix regression; record the exact-head impact disposition and independently reopen the Producer gate before merge if invalidated; no resource-schema change |
| 2 | Register/admit `WP-200-CONSUMER-PROPERTY-READ-AGGREGATE` | Step 1B and the existing exact-coordinate WP-200 tranche are both `complete/current` | Implement preflight, all-bounds barrier, aggregate draft, lookup, and structural work proof in Planning paths |
| 3 | Register/admit `WP-400-CONSUMER-PROPERTY-READ-RUNTIME` | Step 1A and Step 2 are both `complete/current` | Implement Host/static reservation, publication, retained registration/source, name-free execution, drain, and reclaim in Servient paths; produce the predeclared cross-package Host/static fixture and completion evidence |
| 4A | Register `CONSUMER-PROPERTY-READ-ARCHITECTURE` in `ready` through the isolated gate-registry/checker projection | Step 3 and every component tranche are `complete/current`, their completion evidence is `passed`, and the exact fixture source already exists | Preserve the passed Producer gate; atomically generalize the index/checker to two exact manifests; add only the Consumer gate document/manifest and artifact/spec registration; change no product or fixture source |
| 4B | Independently accept the exact Consumer gate candidate | The registered `ready` head and every listed command/evidence pass | A separate reviewer-controlled status-only change moves `ready -> passed`; later real Host Zenoh remains separate WP-600 production evidence |

Steps 1A and 1B may proceed independently after migration. Step 2 does not
depend on the request correction. Step 3 is the first join and cannot be
registered until both branches are complete. Step 4A registers no tranche and
points only to already completed source/evidence. At no point does a current
index node depend on a planned or in-progress predecessor.

Each admission is a separate docs-only reviewed revision before its functional
source. Each implementation stays within its recorded paths and produces the
predeclared completion evidence before becoming complete. This is the existing
ADR-0013 workflow; no new admission state or checker exception is needed.

## Falsifiable closure boundary

Authority review and later tranche evidence must be able to falsify all of the
following:

- the authority migration changes no production source, resource row,
  generated resource projection, or new tranche registration;
- `index.toml` accepts every intermediate state without weakening the rule that
  a current dependency is `complete/current`;
- the affected existing WP-300 node and evidence are not reported current while
  their public request contract is superseded;
- all existing WorkClass discriminants remain stable and only
  `DocumentNodes`/`PlanningItems` are appended by the admitted WP-100 source;
- the passed Producer gate remains current across the request correction only
  if explicit impact review reaffirms every intersecting registered source and
  command; otherwise an independent gate-control action reopens it before the
  WP-300 source can merge;
- it likewise remains current across the WorkClass append only if impact
  review reaffirms its fixed ten-class cleanup evidence as the unchanged
  prefix and the complete Producer fixture still passes; otherwise it is
  independently reopened before the WP-100 source can merge;
- typed schema, URI, and security validation work retains its existing work
  class instead of being hidden in or double charged as `DocumentNodes`;
- validation cannot be entered with an unchecked `Thing`, exceed its existing
  structural/memory/work limits, or reset its lifetime budget across steps;
- a Basic-valid Thing without an ID fails Consumer preflight before compiler
  bounds or `start`;
- source-to-persistent reclassification neither duplicates the Thing nor
  changes total live/peak bytes, and a failed destination check leaves the
  source charge intact;
- no resource field, named-profile value, or generated getter is needed for
  the complete first-slice bound;
- no compiler starts until every coordinate has passed bounds and reservation;
- zero or per-coordinate compiler work over the existing step ceiling fails
  before every compiler `start`, and aggregate lifetime is bounded by checked
  structural derivation;
- lookup distinguishes missing property from no readable Form without TD or
  registration scanning;
- one failed coordinate publishes no subset;
- `OutboundRequest` contains no `ThingId`, `ThingSlotId`,
  `AffordanceTarget`, TD, Form, options, or other static human-readable target
  data;
- its removed constructor parameters and `thing_id`/`target` accessors cannot
  compile and are recorded as old-API removals in the reopened tranche;
- request/artifact/registration/plan-set identities and generations are checked
  before protocol work;
- installed execution can reach only the complete Core-sealed registration;
- Host and static cancellation, late results, cleanup, and reclamation retain
  the same semantic outcomes; and
- no general multi-binding, fallback, lazy/cache, credential, subscription,
  collection, emission, protocol-production, or milestone claim enters through
  this path.

The later gate boundary additionally proves that the passed Producer manifest
remains registered unchanged, the Consumer manifest is appended in `ready`
only after all component evidence exists, the checker validates both exact
gates, and no gate fixture or product source is smuggled into registration or
the status-only acceptance change.

## Rejected alternatives

### Keep the PR #63 generated-projection exception

Rejected. It creates source changes with no ADR-0013 tranche, implementation
path, or completion evidence and therefore does not close the admission path.

### Add a fifth generated-resource projection tranche

Rejected. The two new rows are unnecessary for the first slice. A dedicated
projection tranche would add ordering and evidence solely to support avoidable
global configuration growth.

### Weaken `index.toml` to register the future DAG

Rejected. The current dependency rule implements ADR-0013's requirement that
predecessors be complete. Allowing current candidate nodes to depend on future
contracts would turn the admission registry into a second planning system and
make its entries non-executable.

### Replace the passed Producer gate with the Consumer gate

Rejected. The singular current field and hard-coded checker do not make the
Producer claim disposable. Gate registration must preserve that manifest and
evolve to an exact two-manifest registry only after the Consumer prerequisites
exist.

### Pre-register the Consumer gate during authority migration

Rejected. ADR-0019 and `PLAN.md` deliberately defer registration until source
prerequisites are assembled. Registering it early would recreate a speculative
future graph; adding fixture source during later gate registration would also
leave that source outside its predeclared WP-400 tranche.

### Keep two new resource rows but land them with WP-100 or WP-200

Rejected. Existing document, structural, artifact, memory, and per-step limits
already give a conservative finite bound. Moving the rows between tranches
does not justify their global configuration/API cost.

### Add `ThingSlotId` to `OutboundRequest`

Rejected. The binding does not need a Thing-record lookup capability. The
Servient call owner and plan-set lease already retain that lifecycle identity;
adding it to the request creates a second join without removing the also-static
target name.

### Preserve `ThingId` or `AffordanceTarget` because cloning is cheap enough

Rejected. `ThingId` performs a deep string clone, while `AffordanceTarget`
shares its string allocation; both remain static human-readable identity that
active `API-HOT-ID-001` and `PLAN-REQUEST-001` place outside the request. The
artifact/plan already owns the needed static facts.

### Add a WP-300 hot-identity successor while retaining current evidence

Rejected. A successor id cannot make a contradicted completed public request
schema remain current. Reopen/readmit/replace is the existing repository
recovery model.

### Collapse all implementation into WP-400

Rejected. TD validation/work primitives, Planning semantic construction, Core
request conformance, and Servient lifecycle have different owners, source
paths, predecessors, and falsifiable evidence.

### Publish a singleton coordinate or a successful subset

Rejected. The existing `read_property(name, options)` facade addresses every
declared property. A singleton or subset would require arbitrary hidden
selection or call-time TD fallback and would not be the admitted consumed plan
set.

## Migration condition

This topic remains `DECIDED`, not `MIGRATED`. A fresh independent architecture
review must accept the exact docs-only authority migration described above.
That candidate must neither carry generated/functional source nor pre-register
new future tranches. Once the accepted authority, work-package decomposition,
and existing WP-300 impact state are merged, this topic can move to
`MIGRATED`; the first new source begins only after its own independently
reviewed ADR-0013 admission.
