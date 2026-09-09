# WP-100 Consumer Validated Thing Admission

Status: AUTHORITY MIGRATED; NOT ADMITTED. `WP-100-CONSUMER-VALIDATED-THING`
remains `planned` / `candidate` / `current`. This record freezes the
replacement authority selected by workspace topic 0065; it does not authorize
production Rust, a `candidate -> admitted` transition, completion evidence,
WP-200/WP-400 implementation, or a Consumer architecture gate.

Decision review: github-pr:76.

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
- feature cells: `no-default`, `async-no-std`, `std`
- admission state: `planned` / `candidate` / `current`
- future completion evidence key: `consumer-normalized-validated-thing`

The affected active requirements remain exactly:

- `DOC-RUNTIME-001`;
- `ADMIT-MEM-001`;
- `ADMIT-TXN-001`;
- `CONSTRAINED-WORK-001`;
- `CONSTRAINED-PROGRESS-001`; and
- `CONSTRAINED-OWN-001`.

No new requirement, resource-limit row, generated resource projection, or
`WorkClass` is required. The existing `DocumentNodes`, `JsonSchemaNodes`,
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
  inheritance rules produce the same result.

Strings are copied from stable `str` content, never capacity. Integers and
representable floating numbers are captured from stable typed Number queries;
otherwise Number's stable display implementation writes directly into the
charged byte arena. That operation is not Thing/JSON serialization and may not
allocate an intermediate `String`. The normalized numeric form must preserve
the typed `serde_json::Number` value accepted in the resolved feature graph.

One private storage-neutral TD semantic-access kernel is shared by the public
`Thing` adapter and normalized snapshot adapter. The existing Basic rule set,
default-operation rules, URI rules, and security-reference/inheritance rules
remain TD-owned and singular. Compatibility conversion runs Basic validation
against the typed input adapter and proves fieldwise equivalence with the
snapshot. The strict builder validates its completed snapshot through the same
rule kernel. The existing public `Thing::validate_with_level` adapter may still
materialize its established `ValidateError`; the normalization adapter instead
captures the same kernel's first rejection in the fixed inline
`ValidatedThingInvalid`. Neither path copies semantic rules into the normalizer
or Planning, and the diagnostic sink cannot change which inputs Basic accepts.

## Frozen public API

TD does not depend on Core progress or error types. The public replacement
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
| Strict JSON token and typed-field decode | `CodecInputBytes` per consumed input byte plus the applicable structural/specific class | mutable node/edge/byte build arenas are `admission_temporary_bytes_*`; the borrowed input buffer is caller-owned and bounded by `document_bytes_max` |
| Compatibility typed-field inspection | `DocumentNodes` per generic field/node; `CodecInputBytes` per string/number byte read | traversal frames use the one temporary frame arena; the borrowed `Thing` graph is the entry baseline |
| Basic validation | `DocumentNodes` per generic node, `JsonSchemaNodes` per schema node, `UriBytes` per URI byte, and `SecurityBranches` per root/child/reference | no allocation in the normalization adapter; the first failure is fixed inline |
| Defaults, effective operations, and security inheritance/lookup | `DocumentNodes` per Form/default/operation item and `SecurityBranches` per security branch/reference | no allocation; cached resolved results use already counted node/edge/byte arena entries |
| Checked counts and range/offset calculation | applicable structural class per visited item | fixed cursor scalars only; overflow records fixed `CheckedArithmetic` and enters rollback |
| Deterministic map ordering | `DocumentNodes` per comparison/move and `CodecInputBytes` per compared key byte | resumable in-place sort; no map or sort-scratch allocation |
| String and lossless number normalization | `CodecInputBytes` per source byte and `CodecOutputBytes` per emitted byte | current byte-build or exact retained byte arena; no intermediate `String` or serializer output |
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
`WorkBudget` cannot reset that remainder. Exact checked work can therefore
reach a resource `Limit` even when a semantic/byte structural ceiling still
fits, without reclassifying that input as Basic-invalid. The table covers both
entries; the difference between them is the baseline described by their public
guarantees, not an uncharged helper path.

## Normalization, reservation, and seal order

Both paths use one monotonic cursor and these ordered phases:

1. **Input and semantic inspection.** Check applicable limits and charge each
   byte/node/specific semantic unit before processing it. Compatibility reads
   typed fields directly. Strict JSON decoding charges `CodecInputBytes`, emits
   directly into the charged mutable arenas, and shares the same typed field-
   decoding authority. Basic validation uses the single TD semantic kernel.
   No allocation or externally sized scan occurs before its work and structural
   charge.
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

The product boundary follows the workspace semver dependency and ordinary
stable Rust/MSRV policy. It does not pin exact serde_json source, rustc,
liballoc, target layout, or allocator internals. Legal downstream
`preserve_order`, `arbitrary_precision`, and combined serde_json feature
unification must compile and normalize through stable public semantic APIs.
Object storage order and caller capacity disappear; arbitrary-precision number
content remains lossless.

The required compatibility cells are:

- Host `x86_64-unknown-linux-gnu`: TD default, `preserve_order`,
  `arbitrary_precision`, and combined feature graphs;
- real `no_std + alloc` `thumbv7em-none-eabihf`: the same four feature graphs
  using a fixture allocator and no host runtime; and
- the repository `no-default`, `async-no-std`, and `std` package cells.

The same cursor algorithms, semantic outcomes, footprint formulas, and public
views apply on Host and `no_std + alloc`. Pointer width and allocator overhead
may change physical evidence, but no pointer-width atomics, global counting
allocator, or host executor enters the TD contract.

## Permitted future implementation paths

After a separate accepted readmission, production implementation may change
only:

- `td/src/validated.rs` (new normalized owner, cursor, builder, view, footprint,
  and allocation catalog);
- `td/src/validate.rs` (the shared storage-neutral Basic rule kernel);
- `td/src/td_defaults.rs` (shared storage-neutral default/security queries);
- `td/src/thing.rs` and `td/src/lib.rs` (Thing adapter and public exports);
- `td/src/flat.rs` (shared typed field decoding used by the strict JSON entry);
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

Changes outside `td/src/validated.rs` may only extract storage-neutral semantic
access/decoding used by both existing `Thing` behavior and the normalized
snapshot. They may not change public `Thing` fields, builders, serialization,
deserialization, Basic-valid input set, defaults, or URI/security semantics.
Tests, external compile fixtures, and the registered future evidence file may
be added outside those production paths. A required production change outside
this list returns the tranche to impact review.

No Foundation production change is expected: the two WorkClasses and ledger
reclassification operation already exist. A discovered need to change a
Foundation public method, add an account, add a resource row, or add a work
class stops implementation for architecture review.

## Explicit exclusions

This migration and the later WP-100 tranche do not implement or claim:

- any production Rust before separate readmission;
- a caller-allocation-history, pointer-identity, exact-rustc, liballoc-layout,
  or allocator-overhead contract;
- JSON serialize/deserialize normalization of `Thing`;
- reconstruction of an owned `Thing` from `ValidatedThing`;
- aggregate Planning implementation or `PlanningItems` charging;
- Servient reservation/publication/execution/lifecycle implementation;
- WP-200 or WP-400 admission/status/evidence changes;
- Consumer architecture-gate registration;
- any resource row, generated getter, or new work class;
- global Basic-validation strengthening or Consumer ID synthesis; or
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
4. An external Planning contract fixture that uses only
   `ValidatedThingView` to enumerate a non-first Property/Form coordinate,
   inspect raw/resolved URI and content metadata, apply effective operations,
   resolve effective security to NoSec, and retain original Form indices,
   without `&Thing`, snapshot parsing, allocation, or copied TD rules.
5. Host and real `thumbv7em-none-eabihf` `no_std + alloc` compile prototypes for
   default, `preserve_order`, `arbitrary_precision`, and combined serde_json
   feature unification, all expected to succeed.
6. Exact progress traces for Basic validation, typed/direct decode,
   normalization, sorting, URI/string/number handling, seal, diagnostics,
   cancellation, every rollback cause, zero-budget no-progress, lifetime-
   budget non-reset, and prepaid bounded cursor drop.
7. A resource proof mapping source, temporary, peak, actual contiguous request,
   diagnostics, cleanup, reclassification, and inline Servient-owner capacity
   to existing authority without treating aggregate capacity as one allocation.
8. Reaffirmation that github-pr:69 Foundation behavior, github-pr:70's Context
   seam, the active resource schema, completed WP-200/WP-300 evidence, and the
   passed Producer Property Read gate remain unchanged by the future source
   boundary; any falsified evidence must be reopened by its owner.

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
2. Compact and over-capacity serde Maps and short, long, and spare-capacity
   arbitrary-precision Numbers pass in all legal feature graphs; normalized
   footprints follow semantic content rather than caller capacity/history.
3. Every typed map and nested extension reachability path enters only the three
   retained arenas, and successful source inspection finds no opaque standard
   or serde allocation graph.
4. Typed fieldwise equivalence, Basic positive/negative parity through one rule
   kernel, ordered sequence/Form-index retention, deterministic map iteration,
   URI resolution, effective operations/security, and lossless numbers.
5. Maximum additional live bytes through inspect/build/grow/seal/equivalence,
   including old/new overlap, failure, cancellation, rollback, and cursor drop;
   every observed value and actual largest request fits its reservation.
6. Below/equal/above and checked-arithmetic cases for every applicable source,
   temporary, peak, actual-contiguous, structural, diagnostic, cleanup, and
   lifetime-work boundary. Rejected allocation/work never occurs first.
7. Terminal ownership for invalid input, Basic invalidity, limit, cancellation,
   allocation/arithmetic/equivalence failure, and deep-input rejection, with no
   recursive unbudgeted drop and all ledger accounts returning to baseline.
8. Equal Host/application-static semantic outcomes under varied step sizes,
   zero-budget no-progress, and non-resettable lifetime work.
9. Execution of the Host and real `no_std + alloc` matrix, including allocator
   tracing on the constrained target rather than a compile-only parity claim.
10. The external allocation-free Planning-view consumer, normal locked
    workspace/authority checks, and every registered Producer Property Read
    gate command at the exact implementation head.

This evidence must not readmit the tranche, enter WP-200/WP-400, register or
pass the Consumer gate, claim another operation family, or claim broad WP-100
completion.
