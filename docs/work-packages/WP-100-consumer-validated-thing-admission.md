# WP-100 Consumer TD Admission

Status: AUTHORITY MIGRATED; NOT ADMITTED. `WP-100-CONSUMER-VALIDATED-THING`
remains `planned` / `candidate` / `current`. [ADR-0021](../ADRs/0021-borrowed-consumer-td-admission.org)
migrates the independently reviewed design and discriminator from
[workspace 0075](../../workspace/0075-consumer-borrowed-admission-construction.md)
/ github-pr:128. This record supplies no production implementation, readmission,
completion, successor admission or gate transition.

The normalized-owner authority from github-pr:78 and its later refinements is
superseded for typed Consumer admission. The github-pr:75 withdrawal after the
opaque-allocation counterexample remains history. Foundation/Context work from
#69/#70, typed arenas, shared semantics, strict decoding, RFC3339 and owned-handoff
fixtures remain evidence at their original boundaries. Git preserves prior
contracts; this record does not rewrite those observations as borrowed production
proof. The numeric amendment remains active; the literal strict contract below
moves with future ingestion. Independent acceptance of this exact authority
revision and a separate evidence-backed admission transition remain required.

## Tranche and authority

- id: `WP-100-CONSUMER-VALIDATED-THING`; package `WP-100`
- predecessor tranches: none; package dependency: complete `WP-000`
- owner packages: `clinkz-wot-foundation`, `clinkz-wot-td`
- profile cells: `no-default`, `async-no-std`, `std`; the TD surface explicitly
  requires `validated-thing` in each
- future evidence: `consumer-borrowed-td-admission` at
  `docs/evidence/WP-100-consumer-borrowed-td-admission.toml`

The active requirements are `DOC-RUNTIME-001`, `RES-LIMIT-001..003`,
`ADMIT-MEM-001`, `ADMIT-TXN-001`, `CONSTRAINED-WORK-001`,
`CONSTRAINED-PROGRESS-001`, `CONSTRAINED-OWN-001` and `API-RESOURCE-001`.
The registered specifications own those requirements; this record owns their
exact TD API, support, resource projection and evidence refinements. Planning
owns aggregate selection/compiler/output behavior; Servient owns the transaction.
No requirement enters or leaves the v5.1 set. All twelve WorkClasses and all
196 resource rows retain their identities, order and named values.

## Input, proof and complete validation

The bounded entry borrows `&Thing`. The caller owns and keeps the complete typed
value immutable/live through validation, semantic lending and Planning. The
proof retains that external loan; validation completion does **not** end it.
The consumed aggregate progress owner ends it structurally before returning
source-free completion. Failure ends input loans before returning a source-free output cleanup owner;
that owner remains live until terminal cleanup; source destruction is always the caller's responsibility.
Ordinary serde/authoring provisioning is outside the bounded admission claim.
An engine-owned upstream source keeps its existing global source charge while
lent; loan termination never frees a still-live physical owner.

TD first inspects the entire supplied value under structural/content/Number/work
limits, including private Context entries, every known field, schema child,
map association, sequence, metadata and opaque JSON subtree. It then executes
complete `ValidationLevel::Basic` through one TD rule/discovery kernel also used
by the public synchronous adapter. Security definitions/references, root schemas
and URI variables, all Properties/Actions/Events and all Forms participate,
including coordinates never read by this slice. Only then can TD mint
`ValidatedThing<'td>`. It is opaque, move-only and carries external input, checked
counts/support identity, controlled-state accounts and the same remaining TD
lifetime allowance. An open trait, caller assertion, serializer or raw constructor
cannot mint it. Basic still accepts missing ID; Consumer preflight rejects it.
No Profile/Full validation or new combo-cycle predicate is introduced.

Resource rejection precedes Basic and is distinct from Basic Invalid; inadmissible
structure has no promised Basic first-error order. Once Basic begins, preserve
the shared program's first rejection, schema OneOf-before-flags and child order.
Diagnostics are fixed inline rule/owner/field/index/ordinal facts, never an owned
recursive `ValidateError`. The existing public adapter may format its established
errors outside bounded admission. The five Number predicate amendment is the
only authorized Basic acceptance change.

The proof refers to the same immutable value, so there is no whole-TD copy or
post-copy equivalence pass. Required differential evidence instead proves complete
field discovery, Basic acceptance/first-error parity, effective-query semantics
and complete owned output. A Basic-valid serializer-failing Thing remains legal.
Neither serialization nor JSON length is an input/equivalence oracle.

## Typed-content and resource projection

This section owns the `typed-content-v1` oracle, the explicit typed operation
projection of `document_bytes_max`. It is logical content, not wire length,
capacity or physical RAM. The authority manifest registers resource interpretation
revision 2; numeric resource row ordering/values and the wire-byte ingestion
projection remain intact. Checked support/configuration identity and evidence
include this revision and operation; an old Snapshot interpretation or a wire
measurement cannot certify this typed projection. This is an explicit semantic
migration, not reuse of the old resource meaning under its discriminant.

Walk each occurrence in the supplied typed field model exactly once:

| Leaf / structure | Logical content bytes |
| --- | --- |
| String, map key, URI, operation/security vocabulary text, AP Number lexical text | UTF-8 `str.len()` once per occurrence |
| Present fixed boolean, integer, binary64 or null scalar | 8, independent of Rust width |
| Typed date | eight scalar components (year, month, day, hour, minute, second, nanosecond, signed offset-seconds), 64 total |
| Absent optional slot, structural/variant tag, known field label | 0 |
| Map/sequence/opaque Value | recursively sum keys and supplied leaves; container counts/depth are separate |

Present typed defaults count as fields of the supplied value; absent options do
not. Flattened known fields are discovered once in their owning field program,
not once per serde composition. Root metadata, Context values, links, responses,
schema definitions, URI variables and every affordance use this same traversal.
Semantic vocabulary leaves exposed by that program count their stable TD spelling
as text even when Rust represents the value as an enum; the tag that selects a
record/variant is not a second byte leaf. No serializer is needed to obtain those
fixed vocabulary facts.
Object storage order changes neither the total nor meaning. Values in distinct
positions count separately even when equal. Checked addition overflow is a fixed
arithmetic failure before further work. No wire spelling of dates or Number
projection is used to compute this oracle.

`string_bytes_max` caps the aggregate supplied UTF-8 text/key/URI/vocabulary leaves,
excluding AP Number text (bounded by document and Number limits).
`extension_bytes_max` caps the same content oracle restricted to extension-map
keys/values and opaque JSON-valued const/default/enum subtrees, without double
counting nested opaque descendants. These totals are subsets of the document
total, not additional physical charges. `generated_effective_document_bytes_max`
caps supplied typed content plus derived URI text for one complete semantic pass;
repeated preflight/materialization passes check that same logical ceiling and
pay work again. Fixed/default operations and inherited security lend fixed or
existing supplied facts without creating stored document leaves. Planning-owned
copies remain bounded by its output/PlanFootprint controls. A future TD semantic
extension generating additional content must extend this oracle and account its
storage before admission. `uri_template_source_bytes_max` caps each original and resolved
Form target supplied to template compilation; `expanded_uri_bytes_max` retains
its per-interaction meaning and cannot be treated as a build-scratch allocation.

TD's checked operation catalog consists of:

- document/string/extension/effective-document/Number byte limits above;
- `json_nesting_depth_max`, `json_members_per_object_max`, `json_array_items_max`,
  `json_value_nodes_per_document_max`, applied to the typed structural traversal
  and every opaque subtree; root depth is 1, each nested container adds 1;
  typed records/maps/sequences consume their node/member/item counts too;
- `affordances_per_thing_max` (sum of Properties, Actions and Events),
  `forms_per_context_max`, `forms_per_thing_max`,
  `additional_responses_per_form_max`, `uri_variables_per_form_max`;
- `schema_nodes_per_document_max`, `schema_composition_depth_max`,
  `schema_reference_edges_per_document_max`, `security_expression_depth_max`,
  `security_branches_per_plan_max`, and `uri_template_source_bytes_max`;
- `document_validation_work_units_max` and physical temporary/per-admission/global
  live/peak/largest-request fields used by its actual controlled workspace.

The JSON node count includes each typed record/container/scalar/text occurrence
and each map association; optional absence and field labels consume bounded
structural work but no value node. Schema and security counts retain their named
semantic scopes; separate category totals are checked, not added to JSON nodes
as duplicate visits. The field/discovery implementation must make all counters
observable for boundary tests; the existing typed enumeration is reusable evidence,
not a normative dependence on fixture-private offsets.

`retained_source_bytes_*` and persistent-document accounts have zero new use in
this typed path. They remain controls for actual owned ingestion/source lifetimes.
TD frames and current derived bytes use temporary accounts; runtime output uses
persistent-runtime accounts. Planning and Servient project their own applicable
rows from the **same** immutable policy. Host-only pending-call fields, deferred
lazy/cache fields and unrelated `NA` values are outside TD's catalog. Full selected
role/cell validation remains the resource-profile owner, before transaction entry.
Missing applicable values or unsupported implementation envelopes are configuration
errors, before allowance, ledger, input inspection or progress; they are never
per-input Limit. No row may be treated as unlimited or implicitly inherited.

## Supported algorithms and bounded work

One bounded nonrecursive frame workspace retains actual external iterators or
slice positions. Suspended state has no loan into its own movable storage;
owned scratch uses private positions and fresh short loans. Map ordinal lookup
cannot rebuild a walk with `.nth`; no native hash/get/string comparison is an
unpaid shortcut. Structural enumeration never sorts or reconstructs opaque maps.
Properties retain Thing BTreeMap key order; Forms retain original slice indices.

For supported BTree and dense serde-map backends, predebit a conservative
`container.len() + 1` DocumentNodes envelope for iterator initialization/advance,
including seek/ascent and end detection. Slices/scalar field dispatch cost one
structural unit. Byte equality/ordering pays one CodecInputBytes unit per byte
actually compared, including both operands where read. Each security root/child
pays SecurityBranches; definition searches retain an iterator and byte position.
The production adapter must audit this envelope in every resolved dependency
backend. A backend not satisfying it fails support configuration, or obtains an
adapter with a proved envelope before use; private node-layout/pinning assumptions
cannot confer support. This logical primitive charge can be quadratic in a
container's length; it is not a target instruction/cycle claim. Each declared
structural pass is monotonic, with no prefix replay on Pending.

Production URI resolution is **byte-resumable**, using the shared TD meaning.
Its phases are component discovery, merge search, prefix emission, segment
inspection, output-path pop, query/fragment emission, UTF-8 validation and Ready.
Retain source spans, byte/segment positions, path/pop offsets and output length.
Reverse slash search, append/shift/grow, parse and final validation all pay
UriBytes before each bounded action and retain their positions across Pending.
Capacity transfer retains old/new charges. Only a completed valid UTF-8 target
is lent. The discriminator's small quadratic prepaid atomic resolver does not
admit the named 16-KiB/4-KiB target envelopes; it remains feasibility evidence.
A profile's supported URI lengths cannot be reduced by an input-time fallback.

Every accepted unit of TD inspect/Basic/semantic work also debits the same
non-resettable TD
`document_validation_work_units_max` remainder, including prepaid block release.
Structural inspect, Basic, preflight queries and materialization queries share
that remainder across proof/cursor movement and rewinds. CodecInputBytes pays
text/Number inspection/comparison, CodecOutputBytes pays TD-owned byte copies, UriBytes
pays URI meaning, JsonSchemaNodes pays schema visits, SecurityBranches pays
security work, DocumentNodes pays remaining field/frame work. No relabelling or
double charge solely for crossing a module boundary. Planning output copies and
item/cleanup operations debit its separate monotonic work remainder derived from
structural/output maxima, not a second debit for the same TD semantic action; compiler
coordinates own their declared non-resettable BindingPolls remainders.

Multi-class/atomic step and lifetime debits are checked together before any
counter changes or work. Insufficient step credit returns Pending with identical
state and no carried partial credit; lifetime exhaustion is terminal Limit.
Zero credit causes no source traversal, sizing scan, query, allocation or callback.
Cancellation may choose rollback at zero work. Supported configuration records
the largest required atomic debit in each class (Number `L`, native iterator
`max_count + 1`, fixed callbacks); Host/manual drivers must eventually offer it.
It must be representable and fit the implementation's admitted step/work/temporary
envelope. No profile promises progress from endlessly insufficient caller budgets.
Target cycle/stack claims require separate measurements for the named build.

## Bounded atomic Number semantics

Bounded admission applies one per-Number lexical resource boundary before
lossless retention or numeric projection:

- `number_lexeme_bytes_max` is a named per-item byte limit;
- the Consumer `+validated-thing` owner uses provisional gateway 256 and
  benchmark static reference 64 profile values; directory-client is `NA`
  because it has no validation owner;
- each selected profile's raw limits must first pass
  `ValidatedThingAdmissionConfig::try_from_limits`; its opaque result is the
  only value accepted by the typed validation entry; this projection checks
  only fields consumed by `ValidatedThing` admission, and missing required or
  finite implementation-unsupported `L` values fail before the progress machine
  as configuration errors, not per-input `Limit`;
- for validated `L`, future strict JSON decoding returns `Limit` as soon
  as byte `L + 1` of one Number token is observed, before copying that byte or
  finishing the scan (64/65 and 256/257 for the named profiles); zero rejects
  the first Number byte, including opaque Numbers; and
- typed admission checks borrowed `Number::as_str().len()` before
  projection or lossless copy.

The lexical ceiling is a resource boundary, not a claim that every Number is a
binary64 value. Every within-ceiling supplied Number is inspected losslessly, including
opaque values such as `const`,
`default`, or nested extension Numbers that do not project to finite `f64`.
A value such as AP-backed `const: 1e309` therefore remains a legal storage value
when no Basic arithmetic rule uses it.

Existing typed numeric behavior is unchanged: typed `NumberSchema` fields keep
their current `f64` comparisons and caller-constructed acceptance surface, and
typed `IntegerSchema` fields keep their current `i64` comparisons. This
migration does not add a finite/NaN rule to those typed fields.

Only `serde_json::Value::Number` values in extension fields named `minimum`,
`exclusiveMinimum`, `maximum`, `exclusiveMaximum`, or `multipleOf` use the
amended Basic computation rule. For those five predicates:

- a non-Number value remains absent for that numeric predicate as before;
- a Number is projected through the stable public `Number::as_f64()` behavior;
- the projection must exist and be finite, otherwise Basic returns
  `InvalidSchema` rather than silently treating the Number as absent;
- the four bound fields retain their existing pairwise order checks and use
  ordinary binary64 ordering;
- `multipleOf` retains only its current strict-positivity predicate and does
  not gain divisibility validation; and
- binary64 rounding is deliberate product behavior for these predicates.

The project does not promise arbitrary-precision arithmetic merely because AP
preserves lexical text, and it does not clone private rustc/serde parser quirks
as a separate semantic authority.

### Atomic progress and cancellation

JSON lexing, lossless Number-byte capture, and byte copying remain ordinary
charged/resumable work. Numeric projection/comparison after the lexical bound is
one bounded atomic operation.

For one Number lexeme of length `n`, with `0 < n <= L`, before projection starts:

1. debit `n` `CodecInputBytes` units from the current `WorkBudget`;
2. debit the same `n` units from the shared non-resettable admission lifetime
   remainder;
3. if the current step budget is insufficient, return `Pending` without
   starting and make no numeric progress;
4. if the lifetime remainder is insufficient, return `Limit` without starting;
5. check cancellation immediately before and after the projection; and
6. perform only constant-size scalar comparison after projection under the
   containing schema-node charge.

Thus one uninterrupted numeric projection has the selected profile's validated
finite input bound. Cycle latency and stack margins for a particular target are
product characterization, not generic admission conditions. Zero budget still
makes no numeric progress. If a Number must be
projected again, the repeated projection is charged again; no rescan is free.

Ordinary public `Thing::validate_with_level(Basic)` remains synchronous and
keeps its existing signature and error category. It shares the five-predicate
binary64 acceptance rule, including failed projection -> `InvalidSchema`, but
is not bounded admission and therefore does not expose `Pending` or the
per-admission lexical resource `Limit`. Resource rejection and Basic invalidity
remain distinct.

### Synchronous base-graph responsibility

Without `td/validated-thing`, public Thing and schema `Validate` adapters remain
available and must implement the same five-predicate binary64 Basic rule. They
must not silently skip a Number after failed projection. They do not need
lossless borrowed lexical access merely to perform this synchronous Basic rule,
and no Display-driven exact-decimal comparator is authorized.
The shared rule must distinguish a non-Number value from a Number whose public
`as_f64()` projection fails or is non-finite; an optional `Value::as_f64()`
result alone cannot make that distinction. This applies in base, downstream-AP,
and capability-on graphs.

Capability-off public Basic therefore uses the stable public Number projection
available in its resolved serde graph. Capability-on bounded admission uses
borrowed AP text first to enforce the resource ceiling and then performs the
same projection semantics. These adapters must agree on Basic acceptance for
values that reach the predicate; only bounded admission can terminate earlier
with the lexical `Limit`.

Before readmission, numeric evidence must cover configured `L - 1`/`L`/`L + 1`
thresholds, including 63/64/65 and 255/256/257, zero-disabled first-byte rejection,
short overflow such as predicate `1e309`, binary64 rounding near exact-integer
precision, ordinary underflow behavior in each resolved graph, repeated
projection charging, zero/small budgets, step-budget `Pending`, lifetime
`Limit`, cancellation around one at-most-`L`-byte atomic projection and typed/public
Basic parity wherever the same Number is constructible. Strict parity is a
future-ingestion obligation. The #81/#82 very long
witnesses are now negative resource-boundary cases rather than values the
runtime must successfully compare exactly.

## Opaque proof and lending API

TD depends on Foundation, never Core/Planning/Servient. This is the frozen
portable boundary under `td/validated-thing`; field layout/helper decomposition
are private. `ValidatedThingAdmissionConfig` now means the TD transaction
projection above, not a Snapshot configuration or the application facade.
Its sole checked constructor preserves the numeric supported-prefix check and
adds traversal/URI/workspace support. Servient's checked Consumer policy joins
that projection to Planning/runtime applicability and compiler declarations.

```rust
pub struct ValidatedThing<'td> { /* private input loan, counts, policy, debit, accounts */ }
pub struct ValidatedThingCursor<'td> { /* external loan, owned continuation */ }
pub struct ValidatedThingAdmissionConfig { /* checked fixed TD projection */ }
pub enum ValidatedThingConfigErrorKind { MissingAdmissionLimit, UnsupportedLimit }
pub struct ValidatedThingConfigError { /* kind, resource, configured, supported */ }
pub enum ValidatedThingPhase { Inspect, Basic, Semantics, Rollback }
pub enum ValidatedThingInvalidKind {
    MissingRequiredField, InvalidOperation, InvalidSchema, InvalidSecurity,
    InvalidUri, InvalidReference, InvalidContext,
}
pub struct ValidatedThingInvalid { /* fixed shared rule and original coordinate */ }
pub struct ValidatedThingLimit { /* resource, configured/observed, phase */ }
pub enum ValidatedThingFailureKind { CheckedArithmetic, AllocationFailed }
pub struct ValidatedThingFailure { /* kind, phase, requested bytes */ }
pub enum ValidatedThingCause {
    Invalid(ValidatedThingInvalid), Limit(ValidatedThingLimit),
    Cancelled { phase: ValidatedThingPhase }, Failed(ValidatedThingFailure),
}
pub enum ValidatedThingProgress<'td> {
    Pending(ValidatedThingCursor<'td>), Complete(ValidatedThing<'td>),
    Failed(ValidatedThingCause),
}
impl ValidatedThingAdmissionConfig {
    pub fn try_from_limits(limits: &ResourceLimits) -> Result<Self, ValidatedThingConfigError>;
}
impl<'td> ValidatedThingCursor<'td> {
    pub fn from_thing(thing: &'td Thing, config: &ValidatedThingAdmissionConfig,
                      ledger: AdmissionLedger) -> Self;
    pub fn step(self, budget: &mut WorkBudget, cancel_requested: bool)
        -> ValidatedThingProgress<'td>;
}
pub struct ValidatedPropertyReadCursor<'td> { /* owns proof, current semantic state */ }
pub enum ValidatedPropertyReadStep<'step> {
    Pending, Ready(ValidatedPropertyReadEvent<'step>), Done,
}
pub enum ValidatedPropertyReadEvent<'step> {
    Property { ordinal: u32, name: &'step str },
    Form(ValidatedPropertyReadForm<'step>),
}
pub struct ValidatedPropertyReadForm<'step> { /* immutable semantic facts */ }
impl<'step> ValidatedPropertyReadForm<'step> {
    pub fn property_ordinal(&self) -> u32;
    pub fn property_name(&self) -> &'step str;
    pub fn original_index(&self) -> u32;
    pub fn href(&self) -> &'step str;
    pub fn resolved_href(&self) -> &'step str;
    pub fn content_type(&self) -> &'step str;
    pub fn content_coding(&self) -> Option<&'step str>;
    pub fn subprotocol(&self) -> Option<&'step str>;
    pub fn scopes(&self) -> ValidatedTextSequence<'step>;
    pub fn readable(&self) -> bool;
    pub fn security_count(&self) -> u64;
    pub fn security_name(&self) -> Option<&'step str>;
    pub fn security_scheme(&self) -> Option<&'step str>;
    pub fn copy_bytes(&self) -> u64;
}
pub struct ValidatedTextSequence<'step> { /* private source, paid count/byte sum */ }
impl<'step> ValidatedTextSequence<'step> {
    pub fn len(&self) -> usize;
    pub fn byte_len(&self) -> u64;
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &'step str> + 'step;
}
impl<'td> ValidatedThing<'td> {
    pub fn into_property_read(self) -> ValidatedPropertyReadCursor<'td>;
}
impl<'td> ValidatedPropertyReadCursor<'td> {
    pub fn id(&self) -> Option<&str>;
    pub fn step(&mut self, budget: &mut WorkBudget, cancel_requested: bool)
        -> Result<ValidatedPropertyReadStep<'_>, ValidatedThingCause>;
    pub fn acknowledge(&mut self);
    pub fn rewind(self) -> Self;
}
```

Config diagnostics retain constant `kind`, `resource_kind`, `configured`,
`supported_max` accessors; Invalid/Limit/Failure retain fixed kind/phase/coordinate
or requested/observed accessors. Invalid has `kind`, `phase`, `node_ordinal`; Limit has `kind`, `configured`,
`observed`, `phase`; Failure has `kind`, `phase`, `requested_bytes`. All diagnostics
are fixed-size values with no nested allocation. No formatting is implicit.
Constructors/rewind/acknowledgement do fixed state work and allocate nothing.
`acknowledge` changes only Ready to the next scalar phase; outside Ready it is a
no-op. `rewind` is legal only at Done and otherwise preserves state; it never
refills work or clones the proof. Property/Form events are short loans ending
before acknowledgement, another poll or moving the owner. At Ready, step only
re-lends paid completed facts; insufficient Planning credit leaves the event
Ready. Done is stable until rewind or owner release. On failure the cursor
fixes the first cause and subsequent polls return it without semantic work;
its fixed blocks remain owned until prepaid release. Validation failure returns
Failed only after TD's fixed cleanup is terminal. Proof/cursors are neither
Copy nor Clone and have no raw Thing/storage/mutable accessor.

Each property emits one Property event; only effective readable original Forms
emit Form events. Security name/scheme are Some only for a single effective
reference; other counts let Planning reject eligibility without a second query.
`copy_bytes` is the checked total of all source strings/scopes this slice copies
(including ID/property/raw/resolved targets and metadata), completed before Ready.
An acknowledgement uses its already-paid fixed Ready transition; it never pays
for or advances unpaid discovery. A Form exposes constant-time getters for property ordinal/name, original Form
index, raw and resolved target, content type/coding, subprotocol, scopes,
effective ReadProperty membership, effective-security count/name/scheme and
checked copy byte totals. TD derives all those facts before Ready, including
UTF-8 validity, security lookup and scope count/byte sum. Explicit empty ops
remain empty; explicit empty security overrides inheritance. The sequence's
`len`, `byte_len` and iterator creation are constant-time; Planning charges each
iterator advance/copy before using it. No unrestricted synchronous URI/security
lookup or raw-map iterator is exposed. Planning alone decides exactly-one-NoSec
eligibility. A failed copy debit cannot repeat source sizing/UTF-8 validation or
accumulate credit. Extended operation facts enter only through an admitted TD
semantic extension.

The direct TD entry is an advanced child boundary. Its caller supplies an
unpublished child ledger already capped/paired with parent allowances; the
Servient transaction enforces pairing. TD copies only checked fixed projection
values, makes no input scan/allocation in the constructor and retains its local
physical account until the controlled blocks die. The obsolete Builder, Snapshot
Footprint/View and ConversionError/SemanticMismatch surfaces are removed from
the frozen target; no public strict constructor is frozen for the first slice.
A future ingestion backend uses a closed internal source-access adapter and
cannot offer a downstream proof factory.

## Allocation, lifetime and cleanup catalog

This is the replacement catalog. Every concrete supported representation must
instantiate these formulas with checked target Layouts and binding declarations
before admission; fixture sizes are not profile constants.

| Owner/site | Capacity and account formula | Release/control |
| --- | --- | --- |
| TD traversal frames | checked `Layout::array::<Frame>(capacity)`; depth plus fixed pending-transfer slots, geometric growth capped by checked depth; temporary | old/new both live during item-budgeted transfer; trivially destructible external loans/scalars; one prepaid CleanupItems per nonempty block |
| TD current derived URI | checked byte capacity sufficient for `uri_template_source_bytes_max`; inline reserved slot or one charged byte block, temporary | one current target, private byte positions; bounded growth/shift and prepaid block release; no complete derived table |
| TD proof/diagnostic/policy state | `size_of` plus alignment in its reserved owner slot | includes fixed cause, remaining counters and current facts; no diagnostic heap allocation |
| Planning logical/lookup/candidate/bounds/ref tables | checked array Layout for each complete count; string/byte capacities from paid preflight plus exact emitted programs | persistent-runtime envelope before materialization; spare capacity charged; rollback cursor owns each constructed element |
| Compiler cursor/temporary/artifact | supported registration's conservative final/cursor/temporary declarations plus actual Layout requests | declarations acquired before callback/start; actual excess fails unpublished; one live cursor, exactly-once abort |
| Host/static registration, erasure, registry slot/record | actual shared ownership or caller-reserved typed capacity, wrapper layouts/alignment | startup already-charged storage is not charged twice; attributable retention/erasure delta and record charged before publication |
| Cleanup/reclaim owner | fixed progress/first-cause/position metadata plus complete-object transfer capacity, declared maximum coexistence | reserved at entry/before output acquisition; no allocation on abandonment; owns unpublished or drained output until terminal |

There is no required site count or exact-length TD seal. Each actual nonempty
allocation obtains matching local and parent capacity for its checked Layout
before allocator entry. Allocation count, live total, peak simultaneous total,
largest actual request, temporary peak and additional controlled admission peak
are separate observations. Aggregate reservations are logical envelopes; they
must never be passed as one physical request to a largest-request ledger.
Inline capacity, unused output slots, movement/return overlap, alignment and
selected allocator surcharge remain real costs. A synchronous Host driver must
use the same accounted owner; it cannot hide another continuation/result on stack.

`live = TD frames/current scratch + Planning owner/temporaries/output + live
compiler/artifacts + attributable registration/erasure/record/slot + diagnostics
+ cleanup/reclaim`. Global live additionally includes still-live upstream engine
sources and other owners. Peak is the maximum simultaneous sum, never the sum
of unrelated phase maxima. Typed caller allocation history is outside the
additional-controlled-state claim, not a fictitious zero-sized system input.
Physical source counts are not derived from collection lengths or byte oracle.

TD release touches only its fixed catalog of trivial frame/byte blocks, with
prepaid deallocation and child-before-parent release. Planning variable output
is **not** an unbounded `Vec<Plan>` Drop: one nonrecursive rollback/reclaim owner
walks tables and owned byte/program/artifact blocks with monotonic item/byte
positions under PlanningItems, CleanupItems and the applicable byte-step limits.
The supported binding set declares bounded cursor abort and artifact destruction
primitives; an extension with unbounded destructor/callback is ineligible until
it supplies a bounded cleanup adapter. No recursive caller Thing drop occurs.

On failure, preserve the first cause, stop new compiler starts, abort the single
live compiler exactly once, and release constructed output incrementally. Cleanup
failure cannot replace admission failure or publish a subset. Explicit progress
returns Pending with the same owner; abandoning an owner either performs only
proved fixed prepaid release or ends TD/external input loans and the supported prepaid fixed compiler abort, then
moves **all** remaining source-free output/account objects into its
pre-reserved Servient/manual cleanup destination. Failure to transfer keeps an
addressable cleanup owner. Root shutdown drives terminal cleanup; dropping the
root is not a completion report. Generation/allowance owners remain live until
all protected children are physically released.

## Planning and transaction handoff

The exact aggregate contract is [Planning](../spec/planning.md#first-consumer-property-read-aggregate).
Its owning consumed build API returns Pending with the same lifetime-bearing
owner, one owned preflight with that owner, source-free Complete, or first-cause
failure with source-free remaining cleanup ownership. The build receives a
pre-reserved cleanup slot at entry. Abandonment ends TD loans and performs
exactly-once prepaid fixed abort before moving all unpublished output/accounts
into that slot; it cannot transfer a caller borrow to a detached runtime. Completion consumes/destroys the
terminal input-bearing owner before returning the draft; callers need no special
drop order. Compiler Pending state never borrows transaction-owned logical input:
each callback receives a fresh short loan, while its suspended state is owned or
uses checked positions. Artifacts satisfy `A: 'static` at aggregate admission
plus concrete no-source-pointer/callback evidence; that bound alone is insufficient.

Servient captures policy/registration identities, pairs allowances and drives:
inspect -> complete Basic -> paid preflight -> runtime/record/slot reservation
-> complete materialization -> all bounds -> sequential compile/reconcile/freeze
-> end all input/derived/config/compiler loans -> finish build scratch cleanup
-> reconcile actual owners -> final identity/generation/resource/cancellation/slot
checks -> private permit -> atomic install. Preflight is associated privately
with the same input proof, policy, registration and transaction nonce; it cannot
be reused for another input. A new input or captured identity starts a new build.

Every property receives a deterministic lookup row, including empty ranges;
original Form indices survive. No ID synthesis, credential/provider commit,
protocol I/O, handler/task, lazy slot or external-operation lease occurs while
building. All materialization and all bounds precede every compiler start;
later-coordinate failures reject the whole aggregate. A complete registration
is retained separately and joined only by checked identity/generation.
Published has zero TD/source/document-retention charge attributable to this
borrowed build and contains only owned runtime material and its lifecycle accounts.

Every variable/fallible release finishes **before** the last final checks and
permit. Permit/install execute under exclusive slot mutation authority with
cancellation linearized at the last check. No callback, allocation, source query,
yield, new cancellation decision, fallible operation or retry occurs in that
interval. Readers see absent or complete. Close stops new leases; existing calls,
leases and cleanup retain the immutable aggregate/registration until drain and
bounded reclamation reach zero. Servient does no semantic recount or second lookup.

## Strict JSON value-decoding contract

This section is the sole detailed owner of the future ingestion capability's value semantics
and its agreement with ordinary serde. ADR-0020 owns the rationale. The
typed loan inspects the **already typed** `Thing` directly, including literal objects manually constructed by a caller and
Numbers produced by ordinary serde's string/Value conversions. It neither
re-decodes that value nor attempts to recover its original JSON.

### Literal values and field interpretation

Strict input is one complete UTF-8 JSON object, with JSON token, escape, string,
array and object grammar. Trailing non-whitespace, invalid escapes, malformed
UTF-8 or invalid Unicode scalar decoding are `Invalid`; configured structural,
byte and work limits remain `Limit`. There is no embedded JSON format. Before
TD field interpretation, values have the JSON kinds Null, Boolean, String,
Number, Array and Object:

- Every object member name is a decoded string. No spelling, prefix, escaped
  spelling or member position makes an object a scalar or opens another input
  document. There is no reserved serde-key namespace or collision blacklist.
- A JSON string remains string content. Text resembling JSON inside it is not
  parsed again. This applies recursively, including `const`, `default`, `enum`
  and every extension value, and inside known-field objects.
- Arrays retain element order. Objects retain decoded key/value associations.
  Duplicate decoded names use the **last source occurrence**, matching current
  public Thing map buffering. An earlier occurrence is still syntax-checked
  and charged, but cannot determine a field type, default, discriminator or
  Basic result after it has been overwritten. Escaped and plain equal names
  are duplicates. Rejecting all duplicate keys is not authorized.
- A Number originates only from a JSON Number token. Its logical lossless
  content agrees with public AP `serde_json::Number` decoding of that token in
  the resolved supported graph; original-token byte identity is not promised.
  For example, in the selected lock `-0` yields `0`, `1E0` yields `1e+0`, and
  `1.00` remains `1.00`. This is the existing typed Number-content boundary,
  not permission to extract private dependency code or allocate an opaque
  Number as strict build state.

TD then interprets those literal values through **one shared field-policy
source**, used by ordinary serde adapters and the strict controlled-storage adapter. Shared
policy includes field ownership and flattened composition, required/optional
presence, field-specific null handling, one-or-many conversion, flexible
booleans, defaults, DataSchema/security dispatch, scalar conversion, RFC3339,
and URI types. Strict may not invent a cleaner TD vocabulary or replace
current accepted field behavior. The ordinary adapter retains its existing
Value/RawValue representation conversions around that policy; the strict
adapter supplies literal controlled values and performs no such re-interpretation.
An intermediate serde graph, JSON round-trip, or separately copied field rule
table is not the shared extraction.

In particular, absent and null are not universally interchangeable:
DataSchema `const: null` is `Some(Null)` while absent `const` is `None`;
metadata `title: null` and `profile: null` are `None`; `id: null`,
`properties: null`, schema `unit: null`, `observable: null`, and Form
`contentType: null` reject. Absent Form content type defaults to
`application/json`; null Form `op` and `security` use their existing optional
list behavior. Missing/unrecognized schema type follows existing Object
dispatch, preserving the type field; Basic still decides its validity.
Recognized schema types and security schemes use their existing variants.
This contract changes no field predicate, default or Basic acceptance rule.

Source order is retained until duplicate resolution and the applicable field
decisions are fixed. Only then may object storage be sorted by semantic key.
Neither a first member nor a later sorted-map replay decides a value's kind.
Storage order is non-semantic; association equality, ordered sequences and
original Form indices remain required. No exact-length TD reseal is required. Decoder failures use the frozen inline
diagnostics; serde error-string identity is not a strict-entry guarantee.

### Agreement and deliberate wire differences

For the same decoded logical field values, both adapters must produce the same
typed field decisions, and the typed and ingestion backends must supply identical Basic/default/URI/security
results and field decisions, subject to their
separate source/resource observations. Ordinary serde and strict input need
not have equal raw Number lengths or equal resource limits. Typed admission is
always measured against its supplied typed value, including serializer-failing
Basic-valid values.

Unconditional `from_json(bytes) == validate_borrowed(serde(bytes))` is superseded.
The permitted semantic difference is precisely **literal JSON kinds versus
serde Value/RawValue representation-driven re-interpretation**, including its
propagation through repeated field composition. It is not a blanket exception
for any input containing a suspect key. A field, null, default, variant, scalar
or Basic mismatch when the adapters receive the same logical values remains
a defect. No runtime collision detector or private-token list is required.

The #116 corpus is classified as follows; the private-looking names below are
test input, not parser dispatch vocabulary:

| Wire value/context | Strict result before shared TD policy | Ordinary serde / compatibility |
| --- | --- | --- |
| One-member RawValue/Number-looking object with a string payload | Object with that string member, even when payload text is invalid JSON | Keep current graph-specific Boolean/Array/Number result or decode rejection; inspect a successfully supplied typed value directly |
| The same object with an ordinary second member, in either order or with escaped spelling | Same associations after duplicate resolution; root extension and Property `const` retain Object | Keep current first-key and repeated-composition/order-backend observations |
| Three wrappers or a string containing a 257-element array | Objects and strings; no semantic children are created from string content | Keep current embedded parsing; typed inspection counts and bounds the resulting typed graph |
| Such an object in generic schema `minimum` or `multipleOf` | Non-Number; the unchanged shared predicate treats it as absent | A decoded Number reaches the same predicate, including invalid bound/positivity or amended failed-projection results |
| Such an object supplied to a known string/list/bool/numeric field | Apply the existing field's Object-kind handling, usually decode rejection | Keep any current acceptance caused by conversion to the expected kind |
| An actual Number token, plain string, ordinary object, or unaffected known field | Existing logical scalar/field result | Equality is required wherever the decoded logical values agree |

The apparent Basic difference for a wrapper is therefore a wire-value
difference, not another Basic rule. Direct `minimum: 2` with `maximum: 1`,
`multipleOf: 0`, and an actual predicate Number `1e309` still reach the existing
shared/amended rules. A literal wrapper carrying `1e309` has no Number and
does not invoke numeric projection.

### Construction and resource obligations

The future decoder constructs controlled literal/typed storage directly, without
an ordinary owned Thing or serde graph as intermediate state. Its separately
admitted backend declares exhaustive allocation/capacity/overlap/release formulas;
the former three/four-site prescription is historical evidence, not a required
representation. Shared ordinary field adapters may remain synchronous but cannot
be called as a bounded shortcut. Unescaping, duplicate resolution, field dispatch,
byte comparisons/copies, repeated inspection and discarded-value reclamation
remain nonrecursive, charged and resumable. A completed controlled source lends
the same closed TD semantic program, with its physical charges retained until
physical release before publication.

`number_lexeme_bytes_max` bounds both raw Number-token length and the decoded
Number content retained or projected. The existing lexer stops at raw byte
`L + 1` before copy or a finishing scan. Decoded content can be longer (`1e0`
has three wire bytes but four public AP content bytes); that length must also
be checked before over-limit output/copy or projection. All scans and emitted
bytes pay their existing classes and the same lifetime remainder. Configuration
must account for this transformation in its supported work/temporary-resource
envelope. In strict input, wrapper strings pay string/document limits rather
than Number limits. In compatibility input, a Number produced from a string
by ordinary serde still undergoes the borrowed Number-length guard first.

This selects a precise semantic boundary; it does not demonstrate the
complete implementation. Before ingestion admission, one public-source field-policy
extraction and one charged controlled construction must exercise this contract,
classify all #116 counterexamples, prove unaffected field parity and public AP
Number-content agreement, and integrate configuration, diagnostics and
terminal ownership. A test-only synchronous literal reference proves only the
selected value algebra, not those construction/resource/progress obligations.

## Shared resumable RFC3339 decode

`td/src/rfc3339.rs` remains the single private semantic owner of `created` and
`modified` lexical decoding to `time::OffsetDateTime`. After separate ingestion
admission, extraction may replace its synchronous parser internals with one
resumable decoder used by both the existing serde adapter and strict JSON
entry. Grammar, component range checks, fractional precision, offset handling,
and complete-input rejection must have one rule implementation. A second
parser in `validated.rs`, a shortened-prefix parser, or module rerouting that
orphans the current owner is not permitted. Private helper signatures and
decomposition remain implementation choices; no public decoder API is added.

### Entry and progress responsibility

- Ordinary serde deserialization drives that same decoder synchronously to
  completion and preserves its existing return/error behavior. It acquires no
  `WorkBudget` parameter, admission guarantee, or new input limit. Its caller
  retains external worst-case input/work responsibility; this adapter cannot
  be used as a bounded strict-builder shortcut.
- The future strict decoder constructor remains a fixed-work constructor. During
  `step`, the strict builder owns the decoder continuation, borrowed-input
  position, first cause, per-step `&mut WorkBudget`, and the one non-resettable
  `document_validation_work_units_max` remainder. The decoder owns parse
  semantics and resumable scalar state; it cannot create/reset an allowance or
  perform an externally sized scan outside its caller's charged step.
- `ValidatedThingCursor::from_thing` receives already typed timestamps. It
  inspects those fixed-size values under the existing generic
  field charges; it does not format them, reparse them, or reject a Basic-valid
  typed value because its spelling would fail a serializer or lexical parser.

Strict decoding charges `CodecInputBytes` and the shared lifetime remainder
before each input-byte processing unit, with existing structural charges for
field/node visits. JSON unescaping and date decoding must compose resumably:
an already charged byte may feed bounded decoder state directly; a separate
scan of decoded bytes must carry its own byte charges. A cached lookahead is
fixed state, not permission for an uncharged rescan. Work is not double charged
merely for crossing a helper boundary, and the counterexample's instrumented
read count is not a new WorkClass formula. Emitted/copied arena bytes retain
their existing `CodecOutputBytes` charges.

The continuation must resume inside arbitrarily long fractional seconds and
at delimiters, offsets, and end-of-input, including late-invalid suffixes.
Exhausting the current step allowance yields `Pending` before the uncharged
unit; repeated sufficient small allowances advance without restarting the
prefix. Zero allowance permits no decode progress. Exhausting the lifetime
remainder produces the existing resource `Limit` through rollback, never a
syntax/Basic rejection. Cancellation uses the existing bounded checks,
first-cause preservation, and rollback contract. Node-only charging, an atomic
whole-parser precharge, accumulating credits across steps for a final bulk
scan, and uncharged finish/error scans do not satisfy this contract. Fixed
component finalization must belong to bounded charged progress, not an
unaccounted terminal helper.

### Semantic and storage preservation

The oracle is existing TD decoding behavior, including its accepted subset
and extensions, not a newly selected RFC profile. Preserve four-digit years;
`T`, `t`, or space date/time separators; `Z`/`z`; signed offsets with optional
seconds; the current calendar/time/offset range checks; and full-input syntax,
out-of-range, and trailing-input rejection behavior. Fractions require at
least one digit, pad fewer than nine digits to nanoseconds, truncate numerical
contribution after nine, and still inspect every remaining digit and suffix.
No lexical fraction-length cap, new rounding, timezone normalization, or
Basic-validity restriction is introduced. `OffsetDateTime` component,
nanosecond, and offset results and existing serde error classification/text
remain equivalent. Serialization and formatting behavior remain unchanged.
Strict-entry failures use the already frozen inline diagnostics; they do not
allocate serde error strings.

Decoder state is fixed-size inline scalar state: phase, checked position,
bounded component/precision accumulators, optional cached input unit, and
fixed result/error data. Fraction precision accumulation saturates after nine
digits while input position continues monotonically; no fraction-sized counter
or buffer is needed. State may reference only the builder's existing borrowed
input or charged build arenas and is never retained as an input borrow in the
completed controlled source. JSON escape state is likewise fixed; any materialized
decoded bytes belong to the backend's charged byte storage. Every materialized
string/byte/frame allocation belongs to the ingestion
backend's declared catalog and release proof. Inline state is charged to owner
capacity, never invented as a contiguous allocation request. Private positions
into owned storage replace self-borrows across Pending.

This decoder is future-ingestion authority. Existing shared RFC3339 and literal
construction fixtures remain reusable evidence at those boundaries. Typed
Consumer readmission requires neither a lexical date decoder nor complete strict
field construction. Ingestion admission must prove shared-decoder traces,
semantic/field/Number agreement, supported cells, allocation/work/cancellation
and terminal cleanup before any bounded external-input or Directory claim.

## Legal serde feature unification and supported cells

The product boundary uses stable public dependencies under ordinary semver and
Rust/MSRV policy. It does not pin exact dependency source, rustc, liballoc,
target layout, or allocator internals. Future TD declares the verified public
API floor `serde_json = "1.0.149"` (a caret requirement allowing compatible
upgrades, not `=1.0.149`), preserving `default-features = false` and the
existing `alloc` / `raw_value` features. This TD-local declaration replaces
workspace inheritance; the workspace's other dependency declarations need no
edit. No claim is made that this is the earliest release containing the API.
The fixture lock is reproducibility evidence, not the compatibility boundary.

The future TD manifest adds this explicit opt-in edge:

```toml
[features]
validated-thing = ["serde_json/arbitrary_precision"]
```

TD default/std feature sets do not change. The feature gates the entire frozen
validated surface. Enabling only serde AP downstream cannot enable it. If a
normal sibling dependency enables the TD capability, Cargo unifies it and the
serde edge for users of that TD package instance. Future consumers of the
semantic cursor must explicitly request the capability in their own admitted
manifest changes; ambient workspace unification is insufficient.

Required requested/resolved graphs are:

| Profile / downstream serde request | Capability off: ordinary TD and synchronous Basic | Capability on: full validated surface and Basic |
| --- | --- | --- |
| Host base/default | base serde | AP serde |
| Host `preserve_order` | order serde | AP + order serde |
| Host `arbitrary_precision` | AP serde; validated surface absent | AP serde |
| Host combined | AP + order; validated surface absent | AP + order serde |
| real `thumbv7em-none-eabihf` base `no_std + alloc` | base serde | AP + alloc |
| real target with downstream AP | AP + alloc; validated surface absent | AP + alloc |

Every positive cell must compile. Host runtime tests also cover no-default and
Foundation async-no-std composition with capability off/on. Real target
compilation covers those no-std compositions; TD gains no async feature.
Independent negative imports prove absence under serde-only unification;
positive imports prove sibling TD-capability activation. Select each graph in
a separate Cargo invocation, and inspect resolved features. API-ownership cells
use `profile+validated-thing`; tranche `feature_cells` retains the orthogonal
profile axis.

The [non-production feature-graph prototype](../../tools/architecture-fixtures/validated-thing-feature-boundary/README.md)
exercises these requested/resolved graphs and the selected public Number
projection. Its current-code Basic oracle identifies the AP failed-projection
delta. It does not implement the future shared Basic kernel or complete a
pre-readmission evidence item.

In the resolved dependency `preserve_order` enables std, so it and combined
are Host-only. Legal feature unification does not require an upstream std-only
feature to compile under no_std. Both profiles use the same bounded algorithms,
accounting formulas and loans with the capability on; pointer width may change
physical evidence without introducing atomics, a Host executor or a global
allocator contract. Ordinary parsing across base and AP graphs may produce
different typed Numbers; parity compares the resolved typed value, not an
invented graph-invariant parsing result. Object storage order and caller
capacity remain non-semantic; borrowed Number content remains lossless.

## Permitted future implementation paths

After separate readmission, the TD tranche may change the paths registered in
`index.toml`: Foundation generated Number-row plumbing and the already orphaned
source-to-persistent-document method removal; TD's explicit feature/API floor,
new validation/proof/lending module, shared Basic/default/URI kernels, private
typed inspection in the listed model components and exports. Ordinary Thing
fields/builders/serde and all unaffected Basic semantics remain unchanged.
`td/src/flat.rs`, full strict field construction and lexical RFC3339 extraction
are future-ingestion scope, not this tranche's production duties. Tests/compile
fixtures may be added for the exact admitted boundary. A required production
path or public semantic change outside the registered scope returns to impact
review. No new Foundation account, WorkClass or resource row is authorized.

## Evidence required before separate readmission

These replacement obligations supersede the eight Snapshot-specific items.
Independent exact-head acceptance must establish the **complete TD tranche**
proof before a separate docs-only candidate-to-admitted transition. Reusable
historical witnesses count only where their actual claim matches the obligation;
passing this migration's authority checks discharges none of the production joins.

1. **Capability and lifetimes.** Compile the exact proof/progress/lending boundary
   and full capability off/on matrix; negative fixtures prohibit raw Thing
   aggregate input, forged proof/config, retained scratch loans across moves,
   caller input destruction before the build ends, cloned lifetime allowances,
   obsolete Snapshot APIs and ambient serde-only capability activation.
   Consuming completion returns a draft usable after source/registration destruction.
2. **Complete typed meaning.** Instrument whole-field resource discovery and
   complete shared Basic/first-error order, including unrelated Actions/Events,
   Context/private fields, every schema/response/security/opaque reachability path,
   serializer-failing valid input, absent ID, explicit-empty/default distinctions,
   long/common-prefix names and Number amendment parity. No copy-equivalence or
   lexical date decoder is needed; all supplied contents must be visited.
3. **Support and progress.** Construct the operation projection and supported
   BTree/serde adapters, resumable URI algorithm and AP atomic Number envelope.
   Prove missing/unsupported configuration before allowance/input/progress,
   ignored unrelated NA, named gateway 256/static 64, selected `L-1/L/L+1`, zero,
   public Basic failed/non-finite predicate projection, numeric slow paths,
   whole-debit Pending, lifetime Limit, cancellation, moves/growth and no replay.
   Instrument Ready URI UTF-8 and scope visits under repeated zero/short TD,
   Planning-copy and cleanup credit; unchanged counters alone are insufficient.
4. **Actual resources and terminal ownership.** Instantiate every TD/inline
   catalog formula on supported layouts; match allocator-observed requests,
   transfer overlap, peak/largest/count, pre-allocation rejection and every injected
   allocation failure. Prove fixed prepaid block cleanup, original first cause,
   every cancellation/drop position and child-before-parent release. Prove
   typed-content/string/extension/depth/count/Number/work thresholds and zero
   invented source charge. A real upstream source must retain its original charge.
5. **External semantic join.** An external Planning witness accepts only the
   trusted cursor, traverses non-first original coordinates and owns actual Core
   plans and concrete artifacts from a complete registration. It exercises
   paid operations/security/relative URI, complete output/empty lookup, later
   materialization/bounds failure with zero starts, actual input/registration
   destruction, and bounded fixture rollback. Its fixed catalog is not a claim
   of production variable-output cleanup or Servient race acceptance.
6. **Disjoint evidence and impact.** Reaffirm Foundation #69 and Context #70,
   general ledger/work primitives, the unchanged row/value schema, completed
   exact WP-100 call/response, WP-200 leaf, WP-300 execution and Producer gate.
   Remove the orphaned reclassification method only in its future admitted
   source change. Semantic/strict/date/arena/handoff fixtures remain at their
   narrower boundaries. Reopen any actually falsified owner; don't erase history.

The #128 discriminator supplies reusable local evidence for parts of these
obligations; it lacks full policy projection, production resumable URI support,
complete parent/global composition and variable output cleanup. Strict literal
wire construction, AP decoded spelling growth and resumable RFC3339 proof are
future-ingestion prerequisites rather than typed readmission items.

## Completion and successor evidence

TD completion must repeat the readmission claims through production public
boundaries at the exact implementation head, including actual Host and real
`no_std + alloc` allocator/runtime traces, not compile-only parity. Verify
capability graphs, shared public Basic, every structural/resource/work threshold,
all source inspection/query/cancel/drop/grow causes and zero controlled bytes
on terminal cleanup. This evidence claims only the TD tranche.

WP-200 must separately prove the complete aggregate, concrete source-free
artifact/callback and compiler-Pending ownership, all-bounds-before-start,
pre-admitted/actual final/cursor/temporary/work costs, exactly-once abort,
variable rollback/reclaim and exact PlanFootprint. WP-400 must prove simultaneous
local/global accounting including upstream sources, real slot/generation and
cancellation races, registration/Host erasure, cleanup transfer, source-free
consuming completion, final permit/install, leases/drain and zero runtime bytes
on reclaim in Host/static profiles. Their existing statuses do not change.

Only assembled passed component evidence permits later Consumer gate
registration/acceptance. Real Host Zenoh and constrained target/product cycle,
stack/RAM promises remain separate production evidence. Future bounded ingestion
must prove the strict value/field/Number/date contract, first-allocation source
accounting, complete decoding/work/cancellation/discard/release and shared TD
semantics before Directory/Discovery or release claims rely on external runtime
JSON. Ordinary parsing followed by borrowing cannot satisfy that capability.
