# WP-100 Consumer Validated Thing Admission

Status: AUTHORITY MIGRATED; NOT ADMITTED. `WP-100-CONSUMER-VALIDATED-THING`
remains `planned` / `candidate` / `current`. This record freezes the
replacement authority selected by workspace topic 0065; it does not authorize
production Rust, a `candidate -> admitted` transition, completion evidence,
WP-200/WP-400 implementation, or a Consumer architecture gate.

Decision review: github-pr:76.

Authority migration review: github-pr:78.

Shared RFC3339 decode amendment: [workspace impact review 0066](../../workspace/0066-shared-rfc3339-decode-impact.md),
based on github-pr:79. The frozen decoder contract below and its permitted
future source path amend the migrated boundary without readmitting it.

Workspace topics 0067 and 0068 remain historical evidence for the Number
boundary and the explicit `validated-thing -> serde_json/arbitrary_precision`
feature-access finding. Their exact-decimal / byte-resumable arithmetic clauses
are superseded by [workspace topic 0069](../../workspace/0069-bounded-atomic-number-domain.md),
the [bounded-atomic Number amendment](../amendments/WP-100-bounded-atomic-number-v1.md),
and [workspace migration record 0070](../../workspace/0070-bounded-atomic-number-authority-migration.md).
The subsequent [resource-authority investigation 0071](../../workspace/0071-constrained-resource-authority-and-target-characterization.md)
supersedes 0070's project-wide 256-byte and physical-M4 readmission clauses.
The AP capability, semver floor, and Host/thumb/downstream feature matrix from
0068 remain current; AP is lexical-access authority rather than an
arbitrary-precision arithmetic promise.

Bounded-atomic Number authority migration review location: github-pr:87.

The prior exact-caller-`Thing`, serde_json representation-guard boundary
accepted by github-pr:72 is superseded. Github-pr:75 remains the impact review
that withdrew admission after github-pr:74's private-liballoc accounting was
falsified. Foundation changes already merged through github-pr:69 and the
Context inspection seam merged through github-pr:70 remain current code, but
they supply no production authority for this replacement tranche. The
github-pr:74 implementation, branch-only completion evidence, `complete`
status, and PLAN text never entered master.

## Tranche

- id: `WP-100-CONSUMER-VALIDATED-THING`
- work package: `WP-100`
- predecessor tranches: none
- package dependency: `WP-000` (complete)
- owner packages: `clinkz-wot-foundation`, `clinkz-wot-td`
- feature cells: `no-default`, `async-no-std`, `std`, each with the explicit
  TD `validated-thing` capability for the frozen validated surface
- admission state: `planned` / `candidate` / `current`
- future completion evidence key: `consumer-normalized-validated-thing`

The affected active requirements remain exactly:

- `DOC-RUNTIME-001`;
- `ADMIT-MEM-001`;
- `ADMIT-TXN-001`;
- `CONSTRAINED-WORK-001`;
- `CONSTRAINED-PROGRESS-001`; and
- `CONSTRAINED-OWN-001`.

One append-only named resource-limit row is added by this authority migration:
`number_lexeme_bytes_max`. The named 256/64 values are provisional profile
policy. Each selected Consumer profile must bind a finite value supported by
its projection implementation and complete work/temporary-resource envelope.
No new `WorkClass`, ledger account, allocation category, or state
machine is required. The existing `DocumentNodes`, `JsonSchemaNodes`,
`CodecInputBytes`, `CodecOutputBytes`, `UriBytes`, `SecurityBranches`, and
`CleanupItems` classes cover the complete work described below.
`PlanningItems` remains an already present successor class and no Planning code
may charge it in this tranche.

## Frozen retained representation

`Thing` remains the public authoring and interchange value. A successful
`ValidatedThing` owns no `Thing`, `BTreeMap`, `serde_json::Map`,
`serde_json::Number`, caller `String`/`Vec` capacity, caller pointer, or borrow
from its input. It owns one immutable project-controlled normalized snapshot.

The sealed snapshot has exactly three possible retained allocation sites:

1. an exact-length `RetainedNode` arena for typed TD and extension nodes;
2. an exact-length `RetainedEdge` arena for sequence entries, sorted map
   entries, and ranges; and
3. an exact-length byte arena for string contents, URI contents, and lossless
   number contents.

An empty arena performs no allocation. Every node and edge stores only scalar
values or ranges into those arenas; no arena element owns a nested allocation
or recursively dropped value. Map entries are sorted by semantic key. Forms,
Context entries, arrays, and every other semantically ordered sequence retain
their original order. Raw and TD-resolved Form URI values may both occupy the
byte arena; the resolved value is a TD-owned cached semantic query, not a plan
or a second validation authority.

Unsealed construction may use exactly four temporary allocation sites: mutable
node, edge, and byte build arenas plus one traversal-frame arena. A grow or seal
may temporarily retain the old and replacement allocation for one arena. It
must not allocate an additional map, sort buffer, string, number, recursive
task tree, or serializer output. Map ordering uses a resumable allocation-free
in-place sort. This catalog is exhaustive for the tranche; a required fifth
temporary category or fourth retained arena returns the tranche to impact
review before implementation expands it.

Every allocation uses a project-owned checked `Layout`. Build capacities are
charged at their actual requested capacity immediately. Sealing produces the
three exact-length retained arenas; mutable spare capacity does not cross the
`Complete` boundary.

## Typed semantic equivalence

The compatibility normalizer traverses the typed `Thing` directly. It must not
serialize the `Thing`, deserialize a serializer result, or use JSON round-trip
success as either normalization or equivalence. A serializer may reject a
typed value that the existing Basic validator accepts; that value remains a
legal compatibility input and must not become invalid merely because the
serializer is stricter.

Semantic equivalence is fieldwise equivalence of the typed `Thing` data model:

- every known field and present/absent distinction is equal;
- every string and URI has equal typed content;
- ordered sequences, including Context entries, Forms, and JSON arrays, retain
  order and original Form indices;
- maps retain exactly the same key/value associations, while object/map
  storage order remains non-semantic;
- nested extension null, boolean, string, array, object, and number values are
  equal without floating-point coercion; and
- the same TD-owned Basic, default-operation, URI-resolution, and security-
  inheritance rules produce the same result under the amended future Basic
  rule. The five numeric extension predicates deliberately change only where
  current `as_f64()` failure was silently treated as an absent bound; ordinary
  binary64 projection/rounding remains their computation model.

Strings are copied from stable `str` content, never capacity. Number content
is captured losslessly through public `Number::as_str()`: the explicit TD
`validated-thing` capability guarantees `serde_json/arbitrary_precision` in
every bounded-admission graph. Every source/emitted byte is charged. No scalar
formatting branch, dependency-feature detection, or private serde callback
representation is used. Before any bounded numeric projection, the borrowed
Number lexeme is checked against the configured `number_lexeme_bytes_max`,
which must be finite and validated for the selected projection implementation.
No intermediate `String`, second owned Number, or serializer output may be
allocated. The normalized form preserves the typed
Number and its lossless content; arithmetic projection used by Basic does not
collapse distinct typed/lexical Number values for fieldwise equivalence.

One private storage-neutral TD semantic-access kernel is shared by the public
`Thing` adapter and normalized snapshot adapter. The existing Basic rule set,
default-operation rules, URI rules, and security-reference/inheritance rules
remain TD-owned and singular, subject only to the numeric amendment below.
After implementation, compatibility conversion runs the amended Basic rules
against the typed input adapter and proves fieldwise equivalence with the
snapshot. The strict builder validates its completed snapshot through the same
rule kernel. The existing public `Thing::validate_with_level` adapter may still
materialize its established `ValidateError`; the normalization adapter instead
captures the same kernel's first rejection in the fixed inline
`ValidatedThingInvalid`. Neither path copies semantic rules into the normalizer
or Planning, and the diagnostic sink cannot change which inputs Basic accepts.

## Bounded atomic Number semantics

Bounded admission applies one per-Number lexical resource boundary before
lossless retention or numeric projection:

- `number_lexeme_bytes_max` is a named per-item byte limit;
- the Consumer `+validated-thing` owner uses provisional gateway 256 and
  benchmark static reference 64 profile values; directory-client is `NA`
  because it has no validation owner;
- each selected profile supplies a finite `L` validated before admission against
  the chosen implementation's maximum atomic input, representable complete
  step/lifetime debit, and bounded temporary-resource envelope; unsupported
  values are invalid configuration, not per-input `Limit`;
- for validated `L`, strict JSON decoding returns `Limit` as soon
  as byte `L + 1` of one Number token is observed, before copying that byte or
  finishing the scan (64/65 and 256/257 for the named profiles); zero rejects
  the first Number byte, including opaque Numbers; and
- typed compatibility admission checks borrowed `Number::as_str().len()` before
  projection or lossless copy.

The lexical ceiling is a resource boundary, not a claim that every Number is a
binary64 value. Any within-ceiling Number that belongs in the normalized
snapshot is retained losslessly, including opaque values such as `const`,
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
`Limit`, cancellation around one at-most-`L`-byte atomic projection, and strict/typed
parity wherever the same Number is constructible. The #81/#82 very long
witnesses are now negative resource-boundary cases rather than values the
runtime must successfully compare exactly.

## Frozen public API

TD does not depend on Core progress or error types. Every replacement item,
including views, diagnostics and methods, requires `td/validated-thing` in
each profile. Signatures, ownership and terminal behavior stay unchanged;
capability availability is the only public-surface delta. The replacement
surface is:

```rust
use clinkz_wot_foundation::{AdmissionLedger, ResourceKind, ResourceLimits, WorkBudget};

pub struct ValidatedThingCursor<'a> {
    /* borrows the compatibility Thing; owns normalized build state and ledger */
}

pub struct ValidatedThingBuilder<'a> {
    /* borrows JSON input; owns project-controlled decode/build state and ledger */
}

#[must_use]
pub enum ValidatedThingProgress<C> {
    Pending(C),
    Complete(ValidatedThing),
    Invalid(ValidatedThingInvalid),
    Limit(ValidatedThingLimit),
    Cancelled { phase: ValidatedThingPhase },
    ConversionFailed(ValidatedThingConversionError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidatedThingPhase {
    Input,
    Validation,
    Normalization,
    Seal,
    Equivalence,
    Rollback,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidatedThingInvalidKind {
    InputSyntax,
    MissingRequiredField,
    InvalidOperation,
    InvalidSchema,
    InvalidSecurity,
    InvalidUri,
    InvalidReference,
    InvalidContext,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedThingInvalid {
    /* kind, cause phase, optional strict-input byte offset,
       and optional deterministic semantic-node ordinal */
}

impl ValidatedThingInvalid {
    pub const fn kind(&self) -> ValidatedThingInvalidKind;
    pub const fn phase(&self) -> ValidatedThingPhase;
    pub const fn input_offset(&self) -> Option<u64>;
    pub const fn node_ordinal(&self) -> Option<u64>;
}

pub struct ValidatedThingLimit {
    /* ResourceKind, configured/observed amounts, and ValidatedThingPhase */
}

impl ValidatedThingLimit {
    pub const fn kind(&self) -> ResourceKind;
    pub const fn configured(&self) -> Option<u64>;
    pub const fn observed(&self) -> Option<u64>;
    pub const fn phase(&self) -> ValidatedThingPhase;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidatedThingConversionErrorKind {
    CheckedArithmetic,
    AllocationFailed,
    SemanticMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedThingConversionError {
    /* kind, cause phase, and optional allocation request bytes */
}

impl ValidatedThingConversionError {
    pub const fn kind(&self) -> ValidatedThingConversionErrorKind;
    pub const fn phase(&self) -> ValidatedThingPhase;
    pub const fn requested_bytes(&self) -> Option<u64>;
}

impl<'a> ValidatedThingCursor<'a> {
    pub fn from_thing(
        thing: &'a Thing,
        limits: &ResourceLimits,
        ledger: AdmissionLedger,
    ) -> Self;

    pub fn step(
        self,
        budget: &mut WorkBudget,
        cancel_requested: bool,
    ) -> ValidatedThingProgress<Self>;
}

impl<'a> ValidatedThingBuilder<'a> {
    pub fn from_json(
        input: &'a [u8],
        limits: &ResourceLimits,
        ledger: AdmissionLedger,
    ) -> Self;

    pub fn step(
        self,
        budget: &mut WorkBudget,
        cancel_requested: bool,
    ) -> ValidatedThingProgress<Self>;
}
```

Both entry constructors copy only their fixed applicable limit values and move
one unpublished `AdmissionLedger`; they perform no externally influenced work
or allocation. The two cursor types and `ValidatedThing` implement neither
`Clone` nor `Copy`. There is no unchecked constructor, `thing()`, `into_thing`,
mutable view, raw-arena view, or public storage offset.

The moved ledger owns the per-owner/per-admission physical accounts. Before a
constructor is called, the admission coordinator separately commits the
applicable parent/global allowance and caps this operation from the same
immutable `ResourceLimits`. That outer owner remains paired with the cursor,
reconciles from `ValidatedThingFootprint` on `Complete`, and releases on every
other terminal or cursor drop. TD never treats the child ledger as a global
aggregator. The future Servient implementation owns this pairing; external
pre-code fixtures must provide an equivalent parent owner. This docs migration
does not implement or admit that Servient work.

`from_thing` is the compatibility entry. The input remains immutably borrowed
until terminal progress and is never retained by `ValidatedThing`. Its resource
guarantee is the exact additional project-owned peak over the entry baseline;
it cannot bound the caller's already allocated `Thing` graph.

`from_json` is the strict project-owned builder/decoder entry. The borrowed
input bytes are checked against `document_bytes_max`; all project-owned decode,
normalization, validation, diagnostic, rollback, and retained allocations are
charged from the first controlled allocation. It therefore provides absolute
engine-owned input-processing-through-retention admission. Caller-owned input
buffer memory remains caller responsibility. The decoder shares TD's typed
field-decoding and semantic kernel; it is not a second TD interpretation.

The strict entry is the only direct builder/decoder source admitted here.
Adding a second strict format or a field-by-field public builder requires its
own impact review. Host callers may use either entry. Application-static
callers requiring an absolute engine-owned bound use `from_json`.

## Shared resumable RFC3339 decode

`td/src/rfc3339.rs` remains the single private semantic owner of `created` and
`modified` lexical decoding to `time::OffsetDateTime`. After separate
readmission, extraction may replace its synchronous parser internals with one
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
- `ValidatedThingBuilder::from_json` remains a fixed-work constructor. During
  `step`, the strict builder owns the decoder continuation, borrowed-input
  position, first cause, per-step `&mut WorkBudget`, and the one non-resettable
  `document_validation_work_units_max` remainder. The decoder owns parse
  semantics and resumable scalar state; it cannot create/reset an allowance or
  perform an externally sized scan outside its caller's charged step.
- `ValidatedThingCursor::from_thing` receives already typed timestamps. It
  inspects/copies/compares those fixed-size values under the existing generic
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
completed snapshot. JSON escape state is likewise fixed; any materialized
decoded bytes belong to the existing charged byte-build arena. No new string,
vector, map, scratch arena, recursive task, diagnostic, or cleanup allocation
category is permitted. Inline state belongs to its existing cursor/owner
capacity, not an invented physical allocation request. The three retained /
four temporary allocation catalog and prepaid bounded release remain intact.

This amendment resolves only the source-boundary omission. Readmission item 6
still requires executable shared-decoder traces, and items 2, 3, 5, and 7 still
require allocation, typed parity, supported-cell, and resource proof covering
this extraction. None of the eight items is completed here. A required new
WorkClass, public API, allocation category, or broader ownership/lifecycle
change stops work for a new impact review. The one `number_lexeme_bytes_max`
resource row is the only newly authorized resource-schema addition.

## Frozen footprint and semantic view

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedThingFootprint {
    /* retained requested bytes, retained allocation count,
       largest actual allocation request, peak temporary requested bytes,
       and additional conversion peak bytes */
}

impl ValidatedThing {
    pub fn view(&self) -> ValidatedThingView<'_>;
    pub const fn footprint(&self) -> ValidatedThingFootprint;
    pub const fn retained_source_bytes(&self) -> u64;
    pub const fn property_count(&self) -> u64;
    pub const fn readable_property_form_count(&self) -> u64;
    pub fn reclassify_source_to_persistent_document(&mut self) -> bool;
}

impl ValidatedThingFootprint {
    pub const fn retained_requested_bytes(self) -> u64;
    pub const fn retained_allocation_count(self) -> u64;
    pub const fn largest_allocation_request_bytes(self) -> u64;
    pub const fn peak_temporary_requested_bytes(self) -> u64;
    pub const fn additional_conversion_peak_bytes(self) -> u64;
}
```

`retained_requested_bytes` is the sum of `Layout::size()` for the live sealed
node, edge, and byte arenas. `retained_allocation_count` is the number of those
non-empty arenas. `largest_allocation_request_bytes` is the largest actual
single project allocation request across build, grow, resolve, seal, and final
retention. `peak_temporary_requested_bytes` is the maximum simultaneously live
temporary allocation total. `additional_conversion_peak_bytes` is the maximum
simultaneously live project-owned source plus temporary allocation total above
the selected entry's baseline, including old/new grow or seal overlap.

`retained_source_bytes()` is a compatibility alias for
`footprint().retained_requested_bytes()`. Inline `ValidatedThing` and Servient
record bytes are not allocation requests and are charged by their owning slot
or runtime-record capacity, not folded into an invented contiguous request.
Allocator headers, bins, and rounding are outside the portable footprint; a
profile that governs them owns an explicit allocator-specific surcharge.

Every `AdmissionLedger::try_reserve_source` or `try_reserve_temporary` call in
this tranche corresponds to one actual checked `Layout` request. The ledger is
never given an aggregate footprint as if it were one physical allocation.
Consequently its largest-contiguous observation is the maximum actual request,
while account usage and live/peak observations are sums. The completed owner
keeps the ledger. `reclassify_source_to_persistent_document()` delegates the
existing Foundation operation for exactly `retained_requested_bytes`; it
changes account classification only and changes no physical allocation,
allocation count, live total, conversion peak, or largest request.

The storage-independent borrowed Planning view is:

```rust
#[derive(Clone, Copy)]
pub struct ValidatedThingView<'a> { /* private */ }
#[derive(Clone, Copy)]
pub struct ValidatedPropertyView<'a> { /* private */ }
#[derive(Clone, Copy)]
pub struct ValidatedFormView<'a> { /* private */ }
#[derive(Clone, Copy)]
pub struct ValidatedSecuritySchemeView<'a> { /* private */ }

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidatedFormHref<'a> {
    Reference(&'a str),
    Template(&'a str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidatedFormHrefError {
    TemplateBase,
    Resolution,
}

impl<'a> ValidatedThingView<'a> {
    pub fn id(self) -> Option<&'a str>;
    pub fn property(self, name: &str) -> Option<ValidatedPropertyView<'a>>;
    pub fn properties(
        self,
    ) -> impl ExactSizeIterator<Item = ValidatedPropertyView<'a>> + Clone + 'a;
    pub fn security_definition(
        self,
        name: &str,
    ) -> Option<ValidatedSecuritySchemeView<'a>>;
}

impl<'a> ValidatedPropertyView<'a> {
    pub const fn ordinal(self) -> u32;
    pub fn name(self) -> &'a str;
    pub fn form(self, original_index: u32) -> Option<ValidatedFormView<'a>>;
    pub fn forms(
        self,
    ) -> impl ExactSizeIterator<Item = ValidatedFormView<'a>> + Clone + 'a;
}

impl<'a> ValidatedFormView<'a> {
    pub const fn original_index(self) -> u32;
    pub fn href(self) -> ValidatedFormHref<'a>;
    pub fn resolved_href(
        self,
    ) -> Result<ValidatedFormHref<'a>, ValidatedFormHrefError>;
    pub fn content_type(self) -> &'a str;
    pub fn content_coding(self) -> Option<&'a str>;
    pub fn subprotocol(self) -> Option<&'a str>;
    pub fn scopes(self) -> impl ExactSizeIterator<Item = &'a str> + Clone + 'a;
    pub fn effective_operations(
        self,
    ) -> impl ExactSizeIterator<Item = Operation> + Clone + 'a;
    pub fn effective_security(
        self,
    ) -> impl ExactSizeIterator<Item = &'a str> + Clone + 'a;
}

impl<'a> ValidatedSecuritySchemeView<'a> {
    pub fn name(self) -> &'a str;
    pub fn scheme(self) -> &'a str;
}
```

Property `ordinal()` is the deterministic normalized key-order index; it is not
source object-member order. `original_index()` is the exact zero-based index in
the property's original Form array. The URI methods, effective operations,
effective security names, and security-definition lookup are allocation-free
TD-owned queries. They cover the first Consumer Property Read aggregate's real
Planning inputs: identity, exact Property/Form coordinates, raw and resolved
URI, content metadata, effective ReadProperty membership, and proof that each
effective security name resolves to the required scheme. Planning must not
reconstruct `Thing`, parse snapshot storage, resolve defaults/security itself,
or receive a raw node/edge/byte range.

The snapshot preserves all other typed Thing semantics even though this first
public view exposes only the Planning queries above. Extending the view for a
later operation family is an additive TD API review; it does not expose the
physical arenas.

## Complete work and allocation responsibility

The complete admission flow has these owners; an implementation may split a
cursor helper but may not move work or allocation outside this table.

| Work | Work charge before execution | Allocation/account responsibility |
| --- | --- | --- |
| Strict JSON token and typed-field decode | `CodecInputBytes` per consumed input byte plus the applicable structural/specific class; Number token length is checked against `number_lexeme_bytes_max` while lexing | mutable node/edge/byte build arenas are `admission_temporary_bytes_*`; the borrowed input buffer is caller-owned and bounded by `document_bytes_max` |
| Compatibility typed-field inspection | `DocumentNodes` per generic field/node; `CodecInputBytes` per string/number byte read | traversal frames use the one temporary frame arena; the borrowed `Thing` graph is the entry baseline |
| Basic validation | `DocumentNodes` per generic node, `JsonSchemaNodes` per schema node, one fully precharged `CodecInputBytes` debit equal to a predicate Number's bounded lexeme length before each atomic projection, `UriBytes` per URI byte, and `SecurityBranches` per root/child/reference | no allocation in the normalization adapter; the first failure is fixed inline |
| Defaults, effective operations, and security inheritance/lookup | `DocumentNodes` per Form/default/operation item and `SecurityBranches` per security branch/reference | no allocation; cached resolved results use already counted node/edge/byte arena entries |
| Checked counts and range/offset calculation | applicable structural class per visited item | fixed cursor scalars only; overflow records fixed `CheckedArithmetic` and enters rollback |
| Deterministic map ordering | `DocumentNodes` per comparison/move and `CodecInputBytes` per compared key byte | resumable in-place sort; no map or sort-scratch allocation |
| String and lossless number normalization | `CodecInputBytes` per source byte and `CodecOutputBytes` per emitted byte; predicate projection work is charged separately as the bounded atomic debit above | current byte-build or exact retained byte arena; no intermediate `String`, owned Number, or serializer output |
| URI parse/resolution and copied URI output | `UriBytes` per source/output byte, plus `CodecOutputBytes` for retained bytes | byte arena only; no URI-owned nested allocation survives emission |
| Node/edge normalization and grow copies | `DocumentNodes` per emitted or copied element; byte copies retain their byte charges | one mutable arena allocation per category at a time, with charged old/new overlap during growth |
| Seal | `DocumentNodes` per node/edge copied and `CodecInputBytes`/`CodecOutputBytes` per byte copied | reserve each exact retained arena as source while its temporary predecessor remains live; release predecessor only after transfer |
| Typed semantic equivalence | same generic/specific charges as the compared fields, with byte charges for string/number comparisons | allocation-free and serializer-free |
| Limit/invalid/cancel/conversion diagnostic | the charge that discovered the cause has already occurred; no diagnostic formatting work is required | fixed inline category, phase, and numeric coordinate; diagnostic ledger use is zero |
| Failure, cancellation, abandonment, and final destruction | one `CleanupItems` unit prepaid before each live arena allocation | deallocate the fixed arena catalog and release its source/temporary charge; cleanup-account bytes are zero because no separate cleanup record is allocated |

Every accepted WorkClass unit in the table also consumes one unit from the
cursor's shared non-resettable lifetime remainder captured from
`document_validation_work_units_max`; byte classes consume one unit per byte.
Prepaid cleanup consumes both its `CleanupItems` unit and shared lifetime unit
before the corresponding allocation becomes live. A fresh per-step
`WorkBudget` cannot reset that remainder. Predicate Number projection begins
only after its complete bounded byte debit has succeeded against both the
current step and lifetime remainder. Exact checked work can therefore reach a
resource `Limit` even when a semantic/byte structural ceiling still fits,
without reclassifying that input as Basic-invalid. The table covers both
entries; the difference between them is the baseline described by their public
guarantees, not an uncharged helper path.

## Normalization, reservation, and seal order

Both paths use one monotonic cursor and these ordered phases:

1. **Input and semantic inspection.** Check applicable limits and charge each
   byte/node/specific semantic unit before processing it. Compatibility reads
   typed fields directly. Strict JSON decoding charges `CodecInputBytes`, emits
   directly into the charged mutable arenas, checks every Number token against
   `number_lexeme_bytes_max`, and shares the same typed field-decoding authority.
   Basic validation uses the single TD semantic kernel. No allocation or
   externally sized scan occurs before its work and structural charge.
2. **Count and normalize.** Compatibility computes checked exact final arena
   sizes before final allocation. The strict builder canonicalizes only its
   three charged mutable build arenas. Input and emitted string/number bytes use
   `CodecInputBytes` and `CodecOutputBytes`; explicit Form operations and map-
   sort comparisons use `DocumentNodes`; schema, URI, and security work retain
   their more specific classes.
3. **Reserve and allocate.** Before each allocation, check the source or
   temporary account, per-admission and global peak, and actual contiguous
   request. Charge one prepaid `CleanupItems` unit for that live allocation,
   then reserve its exact `Layout::size()` and allocate. Rejected charges do no
   allocation or work.
4. **Seal.** Reserve a replacement exact-length retained arena while its mutable
   predecessor remains charged, allocate/copy under work budget, then release
   the predecessor. All old/new overlap is included in temporary and
   conversion peaks. No mutable spare capacity remains.
5. **Equivalence and completion.** Compatibility performs the typed fieldwise
   equivalence proof. Final counts and footprint are checked, temporary storage
   is released, and only then may `Complete` expose `ValidatedThing`.

The final source arenas remain in the source ledger account until the later
Servient owner successfully reclassifies exactly their requested bytes after
its persistent destination check. Aggregate preflight reservations are logical
capacity and must never be reported as a physical contiguous allocation.

## Terminal ownership, rollback, and cleanup

`docs/state-machines.toml` owns the exact
`validated-thing-normalization` lifecycle. `Invalid`, `Limit`, `Cancelled`, and
`ConversionFailed` are not observable until the cursor has entered rollback,
dropped every partial project-owned allocation, and released every source,
temporary, and peak charge idempotently. Diagnostic-account and cleanup-account
byte use stay zero for this tranche. The first cause and its bounded inline
diagnostic are fixed before rollback and cannot be replaced by a later
cancellation or cleanup result.

Every retained or temporary arena element is trivially destructible. Terminal
cleanup performs a bounded number of allocation releases from the exhaustive
catalog and consumes the already charged `CleanupItems` units. It never
recursively drops one node per input depth. Deep invalid input therefore cannot
bypass the work contract through `Vec`/map/JSON recursive destruction.

The compatibility input is borrowed, so its destruction remains with the
caller and outside the additional-peak claim. The strict JSON input is also
borrowed and owns no nested TD graph. Dropping an unfinished cursor invokes only
the same prepaid fixed-allocation rollback; it cannot publish, leak a ledger
charge, run semantic work, or recursively destroy caller input. A future change
that requires fallible or unbounded destructor work must introduce an explicit
cleanup owner through architecture review rather than hide it in `Drop`.

The terminal diagnostic is fixed inline and performs no allocation or recursive
drop. It preserves whether the first cause was input syntax, one Basic-invalid
category, a named `ResourceKind` limit, cancellation, or one conversion-failure
category, together with its phase and numeric coordinate where applicable.

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
validated view must explicitly request the capability in their own admitted
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
footprint formulas and views with the capability on; pointer width may change
physical evidence without introducing atomics, a Host executor or a global
allocator contract. Ordinary parsing across base and AP graphs may produce
different typed Numbers; parity compares the resolved typed value, not an
invented graph-invariant parsing result. Object storage order and caller
capacity remain non-semantic; retained Number content remains lossless.

## Permitted future implementation paths

After a separate accepted readmission, production implementation may change
only:

- `td/Cargo.toml` (only the explicit capability edge and semver API floor
  above; no default/std change, exact-version pin, fork or unrelated feature);
- `td/src/validated.rs` (new normalized owner, cursor, builder, view, footprint,
  and allocation catalog);
- `td/src/validate.rs` (the shared storage-neutral Basic rule kernel);
- `td/src/td_defaults.rs` (shared storage-neutral default/security queries);
- `td/src/thing.rs` and `td/src/lib.rs` (Thing adapter and public exports);
- `td/src/flat.rs` (shared typed field decoding used by the strict JSON entry);
- `td/src/rfc3339.rs` (the shared resumable date decoder and its synchronous
  serde adapter, limited to the contract above);
- `td/src/components/affordance.rs`;
- `td/src/components/context.rs`;
- `td/src/components/data_schema.rs`;
- `td/src/components/form.rs`;
- `td/src/components/link.rs`;
- `td/src/components/security_scheme.rs`;
- `td/src/core/data_type.rs`;
- `td/src/core/data_type/metadata.rs`;
- `td/src/core/data_type/response.rs`;
- `td/src/core/data_type/uri.rs`; and
- `td/src/core/data_type/version.rs`.

Apart from the explicit manifest boundary and capability-gated exports,
changes outside `td/src/validated.rs` may only extract storage-neutral semantic
access/decoding used by both `Thing` behavior and the normalized snapshot,
plus the explicitly amended bounded-binary64 Basic predicates in the existing
TD semantic owner. They may not change public `Thing` fields, builders,
serialization, deserialization, other Basic-valid input sets, defaults, or
URI/security semantics. The five numeric extension predicates are the sole
authorized Basic acceptance change and must reach the public Thing adapter and
both admission entries together. Tests, external compile fixtures, and the
registered future evidence file may be added outside those production paths. A
required production change outside this list returns the tranche to impact
review.

Foundation implementation may add only the generated/resource-schema plumbing
for `number_lexeme_bytes_max`; no new account or WorkClass is authorized. Any
other Foundation public-method or ownership change stops implementation for
architecture review.

## Explicit exclusions

This migration and the later WP-100 tranche do not implement or claim:

- any production TD normalization Rust before separate readmission;
- a caller-allocation-history, pointer-identity, exact-rustc, liballoc-layout,
  or allocator-overhead contract;
- JSON serialize/deserialize normalization of `Thing`;
- reconstruction of an owned `Thing` from `ValidatedThing`;
- aggregate Planning implementation or `PlanningItems` charging;
- Servient reservation/publication/execution/lifecycle implementation;
- WP-200 or WP-400 admission/status/evidence changes;
- Consumer architecture-gate registration;
- any resource row beyond `number_lexeme_bytes_max`, any new ledger account,
  or any new WorkClass;
- any Basic-validation change beyond the five bounded-binary64 numeric
  extension predicates, or Consumer ID synthesis; or
- another operation family, strict input format, or broad WP-100 completion.

## Evidence required before separate readmission

An independent exact-head review must accept all of the following before any
`candidate -> admitted` transition:

1. Compile-only proof of every frozen public signature and compile-fail proof
   of the removed `new(Thing)`, `ValidatedThingStep`, `thing()`, mutable/raw
   storage, and unchecked-construction surfaces.
2. A source-level prototype of the three retained and four temporary allocation
   sites, checked formulas, grow/seal overlap, trivially destructible arena
   elements, and one-reservation-per-actual-`Layout` ledger ordering.
3. A fixed typed-semantic equivalence corpus covering all known fields,
   optional distinctions, Context order, Form original indices, map-key
   associations, nested extension values, long strings, and lossless numbers.
   It must include a Basic-valid typed `Thing` that cannot complete the chosen
   serializer path and prove compatibility normalization still accepts it.
   For the five amended numeric extension predicates, prove public Thing /
   compatibility / strict-entry agreement for the selected binary64 projection
   in every supported capability graph, preserve non-Number-as-absent behavior,
   make within-ceiling failed projection `InvalidSchema`, preserve opaque
   within-ceiling Numbers losslessly, and preserve existing typed
   `NumberSchema`/`IntegerSchema` results. Record deliberate deltas from today's
   failed-`as_f64`-as-absent behavior without treating binary64 rounding as a
   parity failure. Preserve all unaffected Basic results.
4. An external Planning contract fixture that uses only
   `ValidatedThingView` to enumerate a non-first Property/Form coordinate,
   inspect raw/resolved URI and content metadata, apply effective operations,
   resolve effective security to NoSec, and retain original Form indices,
   without `&Thing`, snapshot parsing, allocation, or copied TD rules.
5. Compile full prototypes for the entire capability off/on matrix above,
   including actual thumb target, serde-only downstream and sibling-capability
   unification, and negative public-surface fixtures. Repeat source-access
   evidence with all frozen signatures and the complete construction model;
   the feature prototype alone does not complete this item. Order/combined
   remain Host-only because the upstream feature enables std.
6. Exact progress traces for Basic validation, typed/direct decode,
   normalization, sorting, URI/string/number handling, seal, diagnostics,
   cancellation, every rollback cause, zero-budget no-progress, lifetime-
   budget non-reset, and prepaid bounded cursor drop. Number traces must cover
   configured `L - 1`/`L`/`L + 1` thresholds, including 63/64/65 and 255/256/257,
   zero-disabled first-byte rejection; strict over-limit stop without a finishing
   scan; typed AP length check before projection/copy; short failed projection;
   rounding; repeated projection charging; step-budget `Pending`; lifetime
   `Limit`; cancellation immediately before/after one projection at the
   selected finite `L`, with a supported atomic work/temporary-resource
   envelope and slower projection-path coverage in the chosen implementation;
   and #81/#82 long witnesses as resource-limit cases rather than
   successful comparisons. A host run or thumb compile alone does not complete
   the supported-cell evidence in item 5 or this progress proof; physical M4
   cycle/stack measurements are required only for a separately declared target
   or product claim.
7. A resource proof mapping source, temporary, peak, actual contiguous request,
   diagnostics, cleanup, reclassification, the new per-Number lexical limit,
   and inline Servient-owner capacity to authority without treating aggregate
   capacity as one allocation.
8. Reaffirmation that github-pr:69 Foundation behavior, github-pr:70's Context
   seam, the active resource schema plus the one appended Number field,
   completed WP-200/WP-300 evidence, and the passed Producer Property Read gate
   remain unchanged by the future source boundary; any falsified evidence must
   be reopened by its owner.

The readmission change is docs-only and separate from this authority migration
and from production implementation.

## Completion evidence required after future implementation

The replacement evidence belongs at
`docs/evidence/WP-100-consumer-normalized-validated-thing.toml`, claims only
`WP-100-CONSUMER-VALIDATED-THING`, and records the exact implementation head.
It must prove:

1. Fresh-empty and retained-empty-root BTreeMap inputs normalize to semantically
   equal outputs with equal retained footprints; allocator-observed live arena
   requests do not exceed the report and return to baseline on owner drop.
2. Compact and over-capacity Maps plus short, long and spare-capacity Numbers
   normalize in every capability-on graph above. All resolve AP; a base-request
   capability cell is not scalar-serde normalization evidence. Separately
   preserve ordinary TD API availability and prove amended synchronous Basic
   in every capability-off cell, including downstream AP. Normalized footprints
   follow semantic content rather than caller capacity/history, and over-limit
   Number lexemes terminate with the named `Limit` before unbounded projection.
3. Every typed map and nested extension reachability path enters only the three
   retained arenas, and successful source inspection finds no opaque standard
   or serde allocation graph.
4. Typed fieldwise equivalence, Basic positive/negative parity through one rule
   kernel, ordered sequence/Form-index retention, deterministic map iteration,
   URI resolution, effective operations/security, lossless Numbers, and the
   bounded-binary64 predicate deltas.
5. Maximum additional live bytes through inspect/build/grow/seal/equivalence,
   including old/new overlap, failure, cancellation, rollback, and cursor drop;
   every observed value and actual largest request fits its reservation.
6. Below/equal/above and checked-arithmetic cases for every applicable source,
   temporary, peak, actual-contiguous, structural, diagnostic, cleanup,
   lifetime-work, and `number_lexeme_bytes_max` boundary. Rejected allocation
   or work never occurs first.
7. Terminal ownership for invalid input, Basic invalidity, limit, cancellation,
   allocation/arithmetic/equivalence failure, and deep-input rejection, with no
   recursive unbudgeted drop and all ledger accounts returning to baseline.
8. Equal Host/application-static semantic outcomes under varied step sizes,
   zero-budget no-progress, non-resettable lifetime work, and bounded atomic
   numeric projection.
9. Execution of the Host and real `no_std + alloc` matrix, including allocator
   tracing on the constrained target rather than a compile-only parity claim.
10. The external allocation-free Planning-view consumer, normal locked
    workspace/authority checks, and every registered Producer Property Read
    gate command at the exact implementation head.

This evidence must not readmit the tranche, enter WP-200/WP-400, register or
pass the Consumer gate, claim another operation family, or claim broad WP-100
completion.
