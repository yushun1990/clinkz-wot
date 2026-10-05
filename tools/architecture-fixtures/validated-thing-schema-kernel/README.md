# Shared Thing semantics and DataSchema field policy

Non-production evidence for
[`WP-100-CONSUMER-VALIDATED-THING`](../../../docs/work-packages/WP-100-consumer-validated-thing-admission.md#evidence-required-before-separate-readmission),
primarily items 3 and 6. The tranche remains `planned` / `candidate`.

The full Basic semantic composition is now exercised by one storage-neutral
rule source over the existing typed Thing, three-arena Snapshot and actually
constructed canonical Thing. It combines
the DataSchema and numeric predicate proof from #114 with root, affordance,
security-scheme/reference checks and the default/security seam from #110.
The literal arena/field-policy composition below extends that proof. No
production ValidatedThing or admission builder is added.

The uncertainty addressed here is whether a future admission cursor must invent
another Basic rule set or error precedence while combining those local proofs.
The candidate uses shared component predicates and shared whole-Thing order;
its adapters contain only borrowed representation facts. Strict Thing
construction and whole-admission resource and terminal ownership remain
separate obligations. The canonical Thing slice below composes paid whole
Basic with typed construction under the same owner and lifetime. The existing
[URI query proof](../validated-thing-arena-layout/README.md#shared-uri-and-borrowed-planning-view-slice)
and [post-drop Planning witness](../validated-thing-planning-handoff/README.md)
remain reusable; Basic introduces no URI-resolution acceptance predicate.

## Reproduce

From the repository root:

```sh
cargo test --locked -p clinkz-wot-td --lib normalized_snapshot_probe
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --features validated-thing
cargo check --locked --target thumbv7em-none-eabihf --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --no-default-features --features validated-thing
```

Mainline CI repeats the independent public-source candidate in Host base, AP,
order, and combined requests with capability off/on; no-default and Foundation
async with capability off/on; preview; and actual thumb base/AP/async requests
with capability off/on. Snapshot adapter execution is repeated by the existing
default/no-default/order/preview TD test cells, all of which resolve AP via the
test dependency. These runs do not establish a complete frozen public surface,
sibling/downstream surface absence, or target runtime parity.

The standalone lockfile preserves the workspace baseline's relevant versions:
serde 1.0.228, serde_json 1.0.149, time 0.3.47, and serde_with 3.18.0. It is
reproducibility evidence, not an exact-version product requirement.

## Shared value-decoding boundary finding

`tests/decode_boundary.rs` compiles one adversarial wire corpus against both
unchanged production TD and this existing Basic candidate. It exposes a
shared strict/typed decoding question upstream of Basic: public decoding can
reinterpret literal extension objects and embedded strings, with graph,
member-order and repeated-field-conversion effects. The investigation,
alternatives, authority impact and exact limits belong to
[workspace topic 0073](../../../workspace/0073-shared-json-value-decode-boundary.md).
Its selected strict/ordinary agreement contract is now owned by the
[WP-100 admission record](../../../docs/work-packages/WP-100-consumer-validated-thing-admission.md#strict-json-value-decoding-contract),
with rationale in ADR-0020; migrated candidate authority still needs independent
review and supplies no readmission.
These observations do not make private serde keys a product contract or
implement a strict decoder. The existing CI matrix automatically executes
them in its Host cells; thumb library checks do not execute these tests.

Run only the finding with `--test decode_boundary`, with the same feature
requests as the commands above. The full commands also retain all existing
schema/whole-Basic evidence.

`tests/value_decode_contract.rs` tests the decision's additional discriminators:
AP Number/map public-event ambiguity, known-field collisions, decoded duplicate
names, field-specific null/default behavior, and AP Number-content spelling
and expansion. One synchronous public RawValue/scalar reference classifies
literal wrappers without special-key dispatch or embedded parsing. It allocates
a recursive Value graph and implements neither the full TD field decoder nor
bounded admission; it is a semantic reference only. The production/public-source
candidate and its generator remain unchanged. Run it with
`--test value_decode_contract` in the same cells. These are Host executions;
thumb library checks do not execute either decoding test corpus.

## One rule source, two representations, two diagnostic sinks

`src/kernel.rs` owns the DataSchema Basic rules once. It visits explicit type,
ordered `oneOf` children, read/write flags, extension size bounds, extension
numeric predicates, typed variant constraints, and Array/Object children in
the existing validator's order. All seven variants are supported. Object
children follow semantic key order; sequence indices remain ordered. Opaque
`const`, `default`, `enum`, and unrelated extension values gain no validation
predicate. Typed NumberSchema keeps its existing f64 comparisons, including
NaN/Infinity acceptance where those comparisons permit it; IntegerSchema
keeps i64 comparisons without binary64 coercion.

`src/typed_access.rs` supplies only typed representation facts. TD's test-only
`schema_kernel_probe.rs` supplies the corresponding borrowed Snapshot facts;
it reconstructs no Thing, Value, or owned Number. The rule source supports an
inline diagnostic sink and the public formatting sink without giving either
sink control over acceptance or first-error order. The inline result contains
only a rule and deterministic schema-local traversal ordinal. It is not yet
the full document's frozen `ValidatedThingInvalid` coordinate.

`src/basic_kernel.rs` composes that same schema visitor with every existing
Thing Basic check. Its order is:

1. Required title, nonempty root security, and ordered root security references.
2. Each security definition in key order: local scheme constraints, then its
   combo oneOf/allOf references. The complete local shape is checked before
   any reference, while an earlier definition's references precede a later
   definition's shape.
3. Root schemaDefinitions, then root uriVariables, each in key order.
4. Properties in key order: Property schema, interaction URI variables, all
   explicit operations, then all Form security references. Actions visit URI
   variables, input, output, operations, then Form security; Events visit URI
   variables, subscription, data, dataResponse, cancellation, operations, then
   Form security. Each whole affordance precedes the next affordance.
5. All Thing Form security references, then all Thing Form operations.

The API-key/OAuth/combo predicates dispatch from the mutable scheme string.
Typed API-key name, typed OAuth flow/endpoints, and typed combo members retain
precedence over similarly named extension fields. Other variants use the
existing string/filtering fallback. Combo duplicates and self-references remain
accepted; no graph-cycle or security-policy requirement is added. Missing and
explicit-empty operations remain accepted; operations are checked against the
form's owning context, without validating inferred defaults or adding a
readOnly/observable eligibility rule. Basic still does not require an ID,
standard WoT context, interaction presence, or resolvable additional-response
schema references. Typed URI parsing remains the URI shape boundary.

`src/basic_typed.rs` and TD's test-only `basic_kernel_probe.rs` supply the two
adapters. The latter reuses #114's Snapshot schema adapter and #110's borrowed
security-name/definition/operation access. The existing query witness and the
public-source candidate now also call the same Property-default, explicit-empty
security inheritance, required-root-security, combo-dispatch, and undefined-name
helpers; the old focused reference witness is not a full Basic validator.

`build.rs` copies the current TD source into Cargo's build directory. It projects
the private DataSchema, affordance, security-scheme and default/security seams
onto these rules, and delegates the public Thing Basic entry to the composition.
Unreachable extracted component helpers are removed from the candidate.
Profile/Full retain their existing enclosing checks; Minimal still bypasses
validation. Real fields, builders, serde decoding, and URI behavior are retained.
The unchanged production TD crate compiles alongside it as the independent
oracle; no generated source is committed or consumed by production TD.

`tests/thing_basic.rs` compiles the same fixed mutation corpus against both
independent TD Rust models, without converting either through serialization.
More than 170 positive/negative cases pin acceptance, public error taxonomy,
complete payloads and exact Display text at every validation level, including
standalone component APIs and public default/security queries. A 24-stage
multi-fault suffix corpus removes one earlier fault at a time, pinning every
cross-phase first error and original Form/reference/operation indices. The
existing #114 nested/schema/numeric corpus is retained, not duplicated.
Existing rich typed/nested/serializer-failure Things now also pass whole Basic
through both storage adapters.

`src/basic_diagnostics.rs` formats the established public error, including the
existing Thing.forms.forms[index].security context. The fixed inline sink holds
only a Basic/schema rule, owner/field/map-or-Form/member coordinates, and an
optional schema-local ordinal. TD tests compare both sinks over both storage
forms and compare public errors directly with the unchanged validator.
The numeric amendment participates in whole-document precedence: title and
root security still precede a schema Number's failed projection. The inline
coordinate is a prototype locator, not the future document-node ordinal or
public ValidatedThingInvalid API. Internal coordinates use machine-width
indices so the public synchronous Basic candidate acquires no admission-only
u32 ceiling. A future bounded adapter must check its own normalized-coordinate
envelope. Neither sink chooses checks or traversal.

## Number projection and bounded continuation

The five numeric extension predicates distinguish absence/non-Number from a
Number without a finite public projection. The latter rejects as
InvalidSchema in the candidate. Tests explicitly retain the unchanged
production validator's failed-projection-as-absent result as the deliberate
delta; it is never reported as ordinary parity.

The typed adapter calls public `Number::as_f64()`. The AP Snapshot adapter
projects borrowed retained text through public `str::parse::<f64>()`, filtering
non-finite results. Differential tests reuse the complete existing
[`bounded-atomic-number` workload](../bounded-atomic-number/README.md), compare
exact binary64 bits or rejection, and observe zero Host allocations in both
projection intervals. No private serde/rustc parser is copied, and no second
owned Number is created. This is tested adapter equivalence in the resolved
graph, not an independent promise about dependency-private parser algorithms.

`NumericCursor` is one fixed scalar continuation for all five predicates.
Four bound projections precede the four pair comparisons; `multipleOf` is
visited only if they pass. Missing/non-Number fields remain absent. Immutable
input handles are resolved before the isolated numeric step and are kept
unchanged across its polls. Map lookup and schema traversal are deliberately
outside that step's measured projection interval; a zero projection budget
cannot disguise them as paid numeric work.

The existing Number feature fixture and this probe use the same extracted
`projection_step.rs` implementation: lexical ceiling, whole-lexeme current
step and lifetime precharge, cancellation immediately before/after projection,
and no credit accumulation. The extraction preserves the Number fixture's
configuration, feature boundary, and results. Tests cover:

- 63/64/65, 255/256/257, application 256/257/258, and zero-disabled lengths;
- zero/small allowances, Pending/resume, unchanged lifetime on unpaid work,
  and lifetime exhaustion across fresh budgets;
- three predicates advancing separately without reprojection on resume,
  repeated validation paying again, and terminal polling doing no projection;
- short failed projection, binary64 integer rounding and underflow,
  comparison failure before a later failed projection, and pre/post
  cancellation with both typed and Snapshot projection; and
- lossless retention of opaque overflow Numbers, without arithmetic rejection.

Charges use the actual borrowed typed/retained Number text. An ordinary TD
decode can produce text whose spelling differs from raw JSON; strict/typed
resource observations must not invent identical raw lengths. Complete direct
decode and its typed-field spelling agreement remain open.

## Resource and progress boundary

The inline semantic and projection intervals observe zero Host heap allocations.
The Snapshot retains the same three arenas, and the new adapters/sinks add no
temporary or retained allocation category. Host tests bound NumericCursor at
128 bytes and its schema-only diagnostic at 16 bytes. The complete Basic
inline diagnostic is at most 64 Host bytes and requires no drop.
These are local state bounds, not builder/Servient inline-owner capacity or
target stack measurements. The scalar Number continuation needs no arena of
its own; future nested traversal must use the already authorized frame arena.

The complete Basic visitor and nested schema visitor are intentionally
synchronous; schema recursion and index-based map/sequence lookup are not
resumable or budgeted. Schema-node, map/key lookup, unsigned extension
projection, byte comparison, nested traversal charging, nonrecursive
continuation, global document ordinals, depth handling, complete
first-cause/rollback ownership, and prepaid cursor destruction are **not**
proved. A numeric Limit/Cancelled result belongs to the future outer rollback
owner; this local helper does not implement that owner. Full admission still
must apply the lexical ceiling to every retained Number before copy, including
opaque Numbers. This probe budgets predicate projection only.

No full configuration catalog or supported maximum M is derived here. The
Number fixture's existing narrow configuration proof remains narrow; the
threshold tests receive fixture ceilings and do not authorize raw limits at a
future public entry. No cycle/stack or slow-parser-path coverage claim is made.

## Readmission contribution

The table describes the earlier whole-Basic proof. The additional field-policy
contribution and its narrower construction limits follow below.

| Item | Contribution and remaining boundary |
| ---: | --- |
| 1 | None. Full frozen API/configuration and removed-surface positive/negative fixtures remain required. |
| 2 | Both semantic adapters use existing Snapshot storage and no extra allocation site. Full charged traversal/sort/construction and frame-arena integration remain required. |
| 3 | Complete Thing Basic rule composition, shared DataSchema/numeric predicates, root/affordance/security checks, typed/Snapshot parity, whole-document first-error order and public taxonomy/text. Existing fieldwise/nested/lossless/serializer-failure corpus is reused. Strict direct-entry construction/equivalence and admitted public integration remain required. |
| 4 | Existing URI/default/security and post-drop Planning proofs remain usable; this fixture adds no Planning claim. Eventual public View/construction still must repeat them. |
| 5 | Full Basic public-source candidate reuses the existing 13 Host runtime / 8 actual thumb compile cells; Snapshot executes the existing four AP graphs. Full frozen API/construction/sibling/downstream and target runtime evidence remain required. |
| 6 | Whole Basic first-error/diagnostic selection is shared; existing atomic numeric progress evidence still drives the same schema predicates. Whole document/schema work charging, opaque complete configuration, direct decode, sorting, URI, seal, cancellation/rollback/prepaid drop remain required. |
| 7 | Complete inline Basic validation observes zero local Host allocations in both representations; fixed diagnostics and the same three retained arenas introduce no category. Complete supported M, source/temporary/peak/contiguous accounting, simultaneous Snapshot/plan residency, source release, Published and owner capacity remain required. |
| 8 | No authority, resource schema, Foundation/Context implementation, completed successor evidence, or gate status changes. Full impact reaffirmation and the future admitted Foundation method removal remain required. |

This fixture closes the full Basic semantic-composition gap at prototype level. It
does not complete any readmission item as a whole, perform an admission
transition, implement production normalization, or register a Consumer gate.

## DataSchema field interpretation over literal arenas

The literal value constructor from #119 and the typed Snapshot/Basic evidence
from #96–#115 were separate paths. A JSON-kind proof cannot decide whether
flattened field ownership, null, one-or-many, dispatch and scalar conversions
give the same TD fields. Conversely more typed field readback cannot exercise
strict field interpretation. This seam is a prerequisite for composing those
paths into full TD construction.

`src/schema_fields.rs` is one representation-neutral field-policy body for the
complete DataSchema family: its context, all seven variants, nested `oneOf`,
Array `items`, Object `properties`, metadata and preserved extensions. The
metadata rows generate both the candidate's existing serde-derived declaration
and its literal reader. Variant ownership, metadata flattening, field-specific
null, absent false flags, one-or-many selection and type dispatch are shared.
Absent/unrecognized type dispatches to Object and retains the type; Basic still
decides validity. `const/default: null` retain a value, metadata null is absent,
and presence-only string/list/numeric fields retain their current rejection.
Array `items: null` is absent. Variant-specific fields remain extensions on
other variants.

`src/schema_serde.rs` supplies the current owning representation facts and
conversions. `build.rs` projects the current public DataSchema deserializers
onto this body, while keeping RawValue/TypePeek, map buffering, metadata
draining, per-element `items` conversion and `from_remaining` re-entry. Even
`const/default` retain their established `from_value::<Value>` conversion.
Production source remains the independent oracle. No generated source enters
production TD.

`src/schema_arena.rs` supplies literal kinds and borrowed ranges from #119's
actual three-arena owner. Its field facts contain only scalars and borrowed
views. The fixed ownership mask names the 29 policy fields, not source member
indices; arbitrary extension counts/order cannot change ownership. Primitive
serde visitors supply the existing integer/f64/flexible-bool conversions without
an owned Number. The flexible-bool rule body is the actual TD helper. Literal
objects never become scalars and strings never open documents. Typed numeric
fields preserve existing integer/binary64 results, including typed Infinity;
only the already amended extension predicates require a finite projection.

Inspection converts every schema-bearing child before Basic starts, without
interpreting opaque `const/default/enum` or extension subtrees as schemas.
The Basic adapter then calls #114's unchanged rule source, not a new validator.
The completed owner is still a **literal JSON owner**, not the future typed
normalized Snapshot. Its inline field facts are borrowed semantic projections;
they do not prove compatibility Thing traversal or equal normalized footprints
for different wire spellings with equal defaulted TD fields.

### Executable composition

`tests/schema_fields.rs` runs on actual literal construction under 1/17/4096
work-unit steps for the rich field corpus, rather than a signature mock. Its
shared fieldwise assertions compile against both unchanged TD and the extracted
candidate. They inspect every variant/context field, optional distinctions,
ordered metadata/required/schema lists, map associations, nested values and
lossless public AP content. No typed Thing/schema serialization is used to
construct an engine input or prove equivalence.

- 822 conversion cases per public model check acceptance against real serde
  and compare every field plus unaffected Basic outcomes on success. They
  include null/kind distinctions, flexible booleans, unsigned/i64 range,
  fractions/exponents, underflow, overflow and binary64 bits.
- Twelve rich inputs exercise all variants and nested children under three
  construction step sizes. The generic sorted-map associations remain ordered
  independently of input field/member order.
- Escaped duplicate names and overwritten wrong-kind fields reach real
  duplicate resolution before policy. The last type/numeric/null decisions
  agree with both actual **Thing Property** decoders. Standalone DataSchema's
  ordinary RawValue/TypePeek duplicate behavior is not used as the Thing oracle.
- The #116/#117 collision family is classified in schema root/nested
  `const/default/enum`, opaque extensions and numeric predicates, with both
  member orders, escaped spelling, three wrappers and a 257-item array-looking
  string. Known string/list/bool/type fields reject literal Objects. Actual
  Number bounds/zero/overflow still reach the shared Basic rules; wrapper
  Objects remain non-Numbers.
- 432 additional ordinary-wire observations compare successful complete typed
  field Debug values and rejection text between the extracted and unchanged
  **Thing** models. Both accepted and rejected cases are required. This guards
  the existing repeated Value/RawValue conversion boundary; the fieldwise
  literal proof above does not depend on Debug or serialization.
- An independent fixed thread-local allocator counter observes zero allocation
  or reallocation calls in field inspection plus Basic, for long-string,
  success, field-error and Basic-error inputs. Owner footprint is unchanged.
  The field-fact product has no destructor and occupies 800 bytes on the tested
  64-bit Host. This is local inline capacity, not target stack or Servient size.

The new dependency is optional behind this fixture's `validated-thing` feature;
capability-off source graphs do not acquire AP from it. The existing CI matrix
executes the new tests in its seven capability-on Host cells and retains the
six off cells; all eight existing thumb library cells include the extracted
ordinary source, with literal arena access in four on cells. No new fixture
crate, matrix family, checker, resource row or work class is added.

```sh
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --features validated-thing --test schema_fields
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --features validated-thing,order
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --no-default-features --features async,validated-thing
```

### Additional contribution and remaining risks

Item 3 advances from separate value and typed semantic witnesses to actual
shared DataSchema field interpretation and Basic over constructed literal
arenas. Items 5/6 advance this composed source's existing capability compilation
and field-conversion-before-Basic traces. Item 7 gains local evidence that the
semantic seam requires no additional allocation category. None is complete as
a whole; item 2's complete typed construction is not claimed.

The original `schema_arena.rs` field lookups/comparisons, list/map inspection,
scalar conversion and Basic remain **synchronous, recursive and unbudgeted**.
Repeated Basic queries repeat field extraction. The charged per-node pass below
addresses field projection only; it does not make these original adapters or
Basic bounded. Neither path can be installed into production admission unchanged.
The fixed error names a schema field only, not the frozen global semantic
ordinal/input-offset/phase diagnostic. Complete first-cause/rollback ownership
and prepaid whole-cursor cleanup are not supplied by this semantic pass.

Whole Thing/affordance/Form/security field extraction, Context/RFC3339/URI
composition, complete typed construction, shared lifetime charging, normalized
seal/equivalence, opaque full configuration/supported M, target execution,
final View/owned Planning integration and full owner/parent resource release
remain required. #93/#108/#110–#115/#119 remain reusable at their documented
boundaries. Another field-storage witness, another Basic rule proof, API-only
surface expansion or a complete configuration claim ahead of the missing
construction/projection envelope would not resolve this field seam.

No authority, admission/completion manifest, gate status or PLAN is changed.
`WP-100-CONSUMER-VALIDATED-THING` remains `planned / candidate / current` and
requires fresh independent exact-head acceptance of the complete eight-item
set before any separate admission-only transition.

## Charged one-node DataSchema field projection

`src/schema_step.rs` resolves a different dependency from the earlier field
parity proof: can **one actual literal schema node** be interpreted through the
same policy while pausing inside field access and conversion, without another
owning graph, temporary site, or unbounded helper? Expanding the field table to
other TD families would not answer that execution question. A whole admission
cursor would also combine it with recursive traversal, Basic, URI/date work,
normalization, configuration and lifecycle obligations that remain separate.

This pass reuses #120's `schema_fields.rs` unchanged in its context/variant
field decisions. Its source adapter has a fixed index and fixed conversion
cache. The ordinary candidate and synchronous literal reference retain their
existing behavior. The shared dispatch vocabulary now also derives its maximum
selector length; there is no separately copied discriminator table.

### Mechanism and work boundary

- A single pass indexes the actual #119 Object's surviving members. Candidate
  names come from `Field::ALL`/`Field::name()`, not another ownership table.
  Member/candidate visits pay `DocumentNodes`; each source key byte actually
  compared pays `CodecInputBytes`. Length mismatch needs no content scan.
  Arbitrarily many extensions cannot enlarge the 29-slot index.
- Dispatch copies at most the shared vocabulary's seven-byte maximum into
  fixed inline state, one paid byte at a time, then uses the existing dispatch
  function. A longer unrecognized type selects Object without inspecting its
  content, but its **entire original string remains the context type**. This
  adds no input length/validity limit.
- The source executes the existing `variant`/`context`/metadata policy. A
  conversion without a cached fact yields a private request. The cursor
  services it under charges, then replays the policy over constant-time facts.
  Each replay pays one structural unit and has a fixed 29-field bound; there
  are at most 30 runs per node. No map scan, list loop, scalar parse, or source
  string comparison occurs during replay. This is bounded local replay, not
  accumulated credit followed by an externally sized scan.
- Text and opaque values return validated borrowed ranges. String lists and
  language maps check one element kind per structural unit. Schema lists/maps
  return child handles only: they do **not** convert or visit child schemas.
  Flexible-bool strings of at most five bytes are copied incrementally into
  fixed state before the real TD visitor runs; longer strings reach that same
  visitor's fixed-spelling length rejection without a content scan.
- Numeric scalar conversion preserves #120's public primitive events. Generic
  conversion may try `i64`, then `u64`, then `f64`; typed floating fields go
  directly to `f64`. **Each actual attempt**, including a failed attempt, uses
  #93/#114's existing lexical guard, whole-lexeme step/lifetime debit and
  pre/post cancellation helper. A resumed stage cannot repeat an earlier
  parse. A fresh validation must pay again. Primitive conversion then uses
  the same public serde and TD flexible-bool visitors as the reference.
  Typed Infinity and integer rejection/precision behavior are unchanged;
  the five extension Basic predicates are not executed by this pass.
- A local field/node transition pays `DocumentNodes`; the node entry also pays
  one `JsonSchemaNodes`. Accepted class units all debit the caller's borrowed
  lifetime remainder. There is no owned allowance, stored step credit, new
  WorkClass, cleanup record or heap request in the field cursor.

The value owner's fixture-only `admission_parts()` splits a borrowed arena
view from its actual unspent construction remainder. Seal no longer discards
that remainder. Tests drive field projection and repeated inspection through
that same mutable scalar. This is executable lifetime composition, **not** a
frozen API/configuration claim; the fixture handoff is not proposed as a public
production method or an unforgeable coordinator. The cancellation closure is
fixture instrumentation for pre/post atomic checks, not a replacement for the
frozen production step's `cancel_requested: bool` or a user callback boundary.

The former `View::text()` called `from_utf8` on the complete range on every
lookup, hiding an uncharged string-sized scan. It now borrows validated UTF-8
without revalidation. The safety invariant is owned by the value constructor:
wire strings are syntax/UTF-8/scalar checked before emission, typed strings
come from `str`, Number emission is ASCII, compaction moves complete ranges,
and private sealed storage is immutable. No public unchecked value constructor
or mutable storage is added. Existing value construction/grammar/allocator
regressions and additional UTF-8/surrogate tests protect this premise.

### Falsifiable evidence

`tests/schema_step.rs` adds fourteen tests in capability-on graphs:

- The **same** #120 fieldwise corpus compiles against both real public models,
  now with the charged literal driver: 822 conversion cases per model and all
  rich variants/context/nested fields. Recursive orchestration in this test
  support is explicitly outside the one-node engine and its measured interval.
  Existing ordinary-wire/collision/whole-Basic regressions remain intact.
- A wide/long input includes 128 opaque members, a 24,576-byte multibyte key,
  title and unknown type, 512 tags, language metadata and opaque overflow
  Numbers. One-unit and interleaved class budgets produce the same exact trace
  as large steps. Byte debits and structural/list work are checked after each
  step; no field pass scans long text just to borrow it.
- Numeric traces separately witness all three primitive parse attempts, u64
  fallback, preserved typed Infinity, per-attempt Pending/lifetime exhaustion,
  63/64/65 and 255/256/257 limits, zero-disabled conversion and decoded `1e0`
  content length four. No byte debit or parse occurs on step/lifetime shortage.
- Zero work and cancellation are checked at **every Pending boundary** of a
  successful rich input, covering all five local phases. Cancellation before
  and after an actual parse preserves the exact debit/attempt observations.
- Actual value construction plus field projection plus repeated projection
  share one remainder. A lifetime one unit below the combined work completes
  value construction but fails field projection as resource exhaustion. The
  outer owner still owns all three source allocations; dropping it releases
  each once. The local field failure is not represented as whole rollback.
- Decoded duplicates, last-wins type/numeric/null decisions and literal
  collision objects reuse actual construction. Opaque Numbers/JSON-looking
  strings do not become primitive conversion work. A nested-invalid schema
  test pins the non-recursive boundary rather than accidentally claiming full
  conversion-before-Basic from a single-node pass.
- An independent fixed thread-local allocator observes zero alloc/dealloc/
  realloc calls in projection, local failure/cancellation and every Pending
  abandonment of its fixed inputs. Source footprint is unchanged. Cursor and
  returned facts need no destructor. On the tested 64-bit Host the field
  cursor is **1,592 inline bytes**, and the result is **800 bytes** with the
  canonical construction's full paid field index below. Policy
  replay also uses fixed local state; these are not target stack, supported-M,
  future Servient owner capacity, or whole-cursor peak claims.

```sh
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --features validated-thing --test schema_step
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --no-default-features --features async,validated-thing
cargo check --locked --target thumbv7em-none-eabihf --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --no-default-features --features validated-thing
```

The existing 13 Host / eight thumb matrix automatically includes this source;
new tests run in its seven capability-on Host cells and the local engine
compiles in four on thumb cells. The other cells continue to protect ordinary
source behavior and capability-off dependencies. No matrix, fixture crate,
manifest, lock, checker or CI orchestration is added.

### Readmission advance and exclusions

| Item | New contribution / remaining boundary |
| ---: | --- |
| 1 | None. Full frozen signatures/removals, sole opaque full configuration and negatives remain. |
| 2 | One schema node's field access/conversion fits fixed non-dropping state and the existing arenas, with no new allocation site. Whole typed TD construction, canonical normalization and frame-based traversal remain. |
| 3 | Existing field parity now survives actual charged/resumable conversion, preserving all context/variant fields and primitive events. Whole Thing field extraction, compatibility traversal, normalized Snapshot equivalence and whole query parity remain. |
| 4 | No new Planning claim; #111/#113 remain reusable. |
| 5 | Existing matrix covers the additional local source; full frozen surface, sibling/downstream construction and constrained execution remain. |
| 6 | Per-node field indexing/list/selector/scalar traces and actual value-to-field lifetime composition advance. Whole schema/Thing nonrecursive traversal, Basic/date/URI/equivalence, complete configuration, global diagnostics and first-cause rollback remain. |
| 7 | No local heap/category/drop cost, unchanged actual source footprint and exact shared-work debit; original allocator/accounting proofs remain. Complete M, whole work/stack/owner envelope, Snapshot/plan overlap, parent/global release and Published/permit/install evidence remain. |
| 8 | Production/authority/resource schema/successor and gate inputs are unchanged. Full independent exact-head reaffirmation and future admitted API removal remain. |

The pass **borrows a completed literal value owner** and returns borrowed field
facts. It neither constructs a typed canonical Snapshot nor proves equal final
footprints for different wire spellings with the same defaulted TD fields.
This staged prototype does not choose production decode/seal ordering. It has
no recursive schema visitor, charged Basic, RFC3339/URI conversion, whole Thing
projection, equivalence pass, normalized seal, full configuration or global
input-offset/node-ordinal diagnostic. The original Basic adapter still repeats
synchronous extraction and cannot be used as a paid shortcut. Numeric support
is tested at local ceilings, not derived as the complete supported maximum M;
no target execution, cycle/stack or parser-internal slow-path claim is made.

No readmission item is complete as a whole. Admission remains
`planned / candidate / current`; production implementation and the separate
admission-only transition still require independent acceptance of the complete
eight-item evidence set. This slice establishes the bounded field seam before
widening it or composing a whole admission cursor, not a new production design.

## Charged nested DataSchema decoding and Basic composition

`src/schema_tree.rs` addresses the composition gap left by #121: a complete
schema subtree must convert every known child, then run the shared Basic rules
in their existing discovery order, while field work, semantic work and traversal
storage all consume the actual value owner's remaining resources. The older
Basic adapter's recursive, repeated extraction is never used by this engine.
Another isolated field conversion or numeric rule case would not establish this
boundary. This is non-production pre-readmission evidence, not an admitted
builder or a typed canonical Snapshot.

### Existing source composed

- #119 supplies the actual immutable literal arenas, checked Accounting/ledger,
  source footprint, and construction's non-resettable lifetime remainder.
- #120 supplies the shared metadata/context/variant policy and primitive TD
  visitors. No field ownership, null/default or discriminator rule is copied.
- #121 supplies charged indexing, conversion and bounded policy replay. Its
  `Machine` now accepts a work remainder at each step; the existing borrowing
  `Cursor` facade uses that **same machine** and retains its original contract.
  Completed facts retain the fixed paid 29-field index and consumed-key mask.
  Basic selects its nine fields from that index; canonical emission identifies
  consumed values by internal node identity without scanning keys again. Basic does not call
  `View::get`, `Extras::get`, synchronous `decode`, or an unbudgeted map scan.
- #114 supplies `NumericCursor` and the atomic projection helper. The same
  `kernel.rs` now also owns `Walk` and the local checks. The synchronous public
  candidate and the bounded traversal both use that rule/discovery source.
  #115's whole-Thing composition continues to use the same schema kernel.

### Work and storage mechanism

The first iterative pass decodes the entire DataSchema subtree, traversing
ordered `oneOf`, then Array items or Object properties in semantic key order.
No Basic check runs during this pass. Thus a malformed later child is not hidden
by a Basic-invalid parent or earlier sibling. The second pass decodes each
visited node once again and executes shared `Walk`: type, oneOf children, flags,
unsigned extension bounds, numeric extension predicates, typed constraints,
and variant children. Both passes pay every field extraction and scalar parse;
there is no free re-extraction on a Basic accessor. Only the active ancestry's
field facts are stored, not a second document graph. This deliberate second
paid pass is a construction witness, not a throughput claim or a production
choice of decode/seal order.

Each traversal transition pays `DocumentNodes`; each field-machine node entry
also pays `JsonSchemaNodes`. Short type comparisons copy paid bytes into the
shared vocabulary's bounded inline buffer. Extension `u64` inspection and
`f64` projection each use the existing full-lexeme debit and pre/post
cancellation helper, including failed attempts. Non-Number predicates remain
absent; opaque values never become schemas or arithmetic predicates. Local rule
comparisons run under their containing structural debit. Every accepted work
unit, including cleanup prepayment, consumes the same lifetime scalar as literal
construction. No step credit is retained.

Traversal frames use the existing fourth temporary site. The sealed arrays are
grouped privately so an immutable `Sealed` borrow can coexist with mutable
accounting. `OwnedValue::inspection_parts()` lends that source borrow, its actual
lifetime and a frame workspace backed by its **actual Accounting and ledger**.
It does not create a separate allowance or change source classification.
Each empty replacement is allocated through the original checked Layout and
reservation body after one prepaid `CleanupItems`/lifetime debit. Old and new
frame blocks remain simultaneously charged. Each frame copy is a separate paid
move; mutation/extraction is forbidden during transfer. All frame elements are
non-dropping. Completion, local failure, cancellation and abandonment release
at most two frame allocations, with no recursive drop or fallible cleanup.

The local pass borrows the source owner. Its terminal releases frames **before**
returning and leaves the original source arenas live. The outer value owner
still owns their release. This establishes the shared child ledger and bounded
local cleanup, not full admission rollback, parent/global release, publication,
or installation. The cancellation closure is fixture instrumentation; it does
not alter the frozen production boolean step parameter.

### Falsifiable witnesses and reproduction

`tests/schema_tree.rs` runs nine tests in each capability-on Host cell:

- 180 comparisons (45 semantic cases across root, oneOf, Array items and
  Object properties) agree with the generated candidate's real public Basic
  entry and the existing literal reference, covering all variants, the five
  extension predicates, finite
  failure, rounding, underflow, u64-sized bounds and typed integer/Infinity
  behavior. Separate tests pin malformed child kinds and conversion-before-
  Basic conflicts, plus exact first-rule/schema-local ordinals after suspension.
- Small/interleaved and large budgets yield identical complete traces. A
  seven-node witness pays field projection twice per node, three Basic numeric
  projections, three frame allocations and three frame moves. Repeated validation
  pays again; construction plus subtree work at the exact lifetime boundary
  succeeds, while one unit less fails as resource exhaustion.
- A Basic Number whose step or lifetime debit is short does not parse or debit.
  Resume does not repeat a completed predicate. Pre/post cancellation pins the
  actual parse/debit, and a fixed Basic rejection is not replaced by a later
  cancellation during cleanup. A size-bound pair rejects before any later pair's
  scalar conversion; a one-unit witness reaches that rejection despite a later
  20-byte Number. AP decoded `1e+309` is charged as six bytes.
- An independent thread-local allocator matches successful frame requests,
  old/new overlap, peak and largest request against the real source ledger.
  Every actual frame request is fault-injected. Temporary, actual-contiguous
  and combined source/frame peak ceilings are tested below/equal/above; rejected
  requests are not allocated. Terminals have zero temporary charge and exactly
  the original source charge. Source drop still releases its three exact arenas.
- Every Pending boundary of a nested witness preserves zero-work state,
  phase and lifetime; cancellation or abandonment releases all frames, including
  both sides of an unfinished grow, without allocation or recursive cleanup.
- A 512-level schema completes with explicit frames, and its first excessive
  semantic depth is rejected before entering the extra node. Long metadata and
  literal collision/opaque overflow objects introduce no hidden schema visit,
  scalar parse or embedded document interpretation.

On the tested 64-bit Host, the subtree cursor is 1,944 inline bytes and one
frame is 832 bytes. The seven-node witness retains 1,321 source bytes; semantic
frame requests peak at 4,992 bytes, the largest frame request is 3,328 bytes,
and their live overlap with source reaches 6,313 bytes. The owner's recorded
conversion peak is the maximum of construction's prior peak and this new
overlap, not a reset. These are fixture observations, not profile ceilings,
target stack bounds or supported-M claims. Instrumented trace/error records
are fixed inline test records, not the frozen production diagnostics.

```sh
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --features validated-thing --test schema_tree -- --nocapture
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --no-default-features --features async,validated-thing
cargo check --locked --target thumbv7em-none-eabihf --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --no-default-features --features validated-thing
```

The unchanged CI matrix executes all original tests plus this source in 13
Host cells, with subtree and canonical tests in seven capability-on cells,
and compiles eight thumb cells. Thumb compilation is not constrained execution.
Existing value construction, typed Snapshot/Basic, Planning handoff and authority
checks remain required; no checker, manifest, lock or CI orchestration is added.

### Exact pre-readmission advance and remaining boundary

Items 2/3/6 advance from one-node field projection plus isolated recursive Basic
to a charged, nonrecursive **complete DataSchema subtree**, with actual shared
lifetime, rule order and first-cause local cleanup. Item 7 gains actual source/
semantic-frame overlap and request/release observations under the same ledger;
item 5 covers the composed source in the existing matrix. No item is complete
as a whole. Items 1/4/8 gain no new surface, Planning or acceptance claim.

The strict Thing/affordance/Form/security field-policy extraction and traversal,
date/URI/default/security execution,
typed canonical normalization/seal/equivalence and default-equivalent footprints
remain. So do complete configuration and derived M, frozen surface negatives,
global diagnostic coordinates, whole first-cause source rollback, constrained
execution and target stack evidence, final View/owned Planning repetition,
and owner/parent/global/publication resource lifecycle. Existing #108 and
#110–#115 are reusable local dependencies, not proof of these composed duties.

Admission stays `planned / candidate / current`. No production, authority,
work-package status, PLAN, successor, resource row, WorkClass or gate is changed.
Independent exact-head acceptance of the complete eight-item set and a separate
admission-only transition are still required before production implementation.

## Charged canonical DataSchema construction and owner transfer

`src/schema_build.rs` connects the subtree proof to an actual typed canonical
result. The prior paths stopped at borrowed literal field facts or used a
separate recursive, fixed-capacity typed Snapshot. This slice removes that
DataSchema construction/lifecycle gap before widening Thing/Form/security
field policy. It adds no production source or authority change.

The fixture driver executes one transaction:

1. Construct the strict literal owner with the established value cursor.
2. Decode the complete DataSchema subtree and execute shared Basic with the
   existing charged subtree cursor. Malformed later children still precede
   earlier Basic invalidity.
3. Lend that same source, lifetime remainder and Accounting to canonical
   emission in the original three empty node/edge/byte build sites and fourth
   traversal site. All input/output/frame/grow overlap remains charged together.
4. End emission with a second charged equivalence traversal while the complete
   literal input and canonical build arrays remain live. End every source borrow.
5. Reseal into exact-length arrays, releasing each old source array and build
   array with its original charge. Return a typed Schema owner that has no
   input lifetime. Any failure fixes its cause and releases both graphs through
   the outer owner; cleanup observes no further cancellation or allocation.

Each schema has one fixed record with 29 optional field slots and an extension
map slot. Slot identifiers come from the existing shared `Field::ALL`; emission
reads **decoded typed facts**, not a new field-policy table. Absent slots use a
checked sentinel. Boolean defaults are materialized, one-or-many sequences
are canonical arrays, optional empty collections remain distinct from absence,
and f64/i64/u32 values are exact scalar bits. Text, languages, opaque JSON and
lossless Number content occupy the same arenas. Maps retain semantic key order;
sequences retain original indices. No owned Number, serializer, second TD graph
or recursive emitter is created.

Completed field facts now retain the full paid index, with its consumed mask.
Extension selection uses internal literal node identity in that fixed index,
so emitting extensions does not hide another source key scan. This enlarges
field/frame inline state; the current subtree observations above reflect that
change. Internal engine identity is not caller pointer/history semantics.

Emission and equivalence each service the unchanged charged field machine;
every repeated scalar parse and input/output byte is paid again. Equivalence
regenerates the typed structural stream, comparing every node, edge target,
original index, absence and byte against the built result, without another
buffer. A deliberate output corruption rejects as SemanticMismatch. Paid
element seal moves preserve that compared graph. The post-seal `Access` adapter
also runs the same synchronous Basic source over the actual canonical result,
without literal field decoding. That query is an external semantic oracle;
**a charged Basic traversal over the canonical result is not a constructor
phase**. The transaction's charged Basic checks precede canonical emission and
the subsequent equivalence proof.

`from_json` is a fixture driver that services borrowing continuations in lexical
scopes and returns the owned result. Its budget/cancellation hook is test
instrumentation, not the frozen owning admission cursor API. No self-reference
or lifetime erasure is used. Canonical reconstruction directly from a typed
Thing, including its serializer-incompatible values, is still unimplemented.

### Allocation and progress ordering corrections and falsifiable evidence

Independent regressions against the prior source reproduce three defects:
zero Number returns Pending after its first byte; an impossible frame request
yields for cleanup credit before reporting Memory; and a null-returning grow
reports a 120-byte reservation peak although only 40 bytes ever lived.
The value cursor now fixes zero rejection in the observing wire transition.
Grow/seal preflight is pure and precedes cleanup/lifetime debit in both value
entries, semantic frames and canonical rebuild. Physical peaks advance only
after successful allocation; actual request maximum and logical reservation
high water remain separately observable. Foundation ledger semantics are
unchanged.

Canonical emission, equivalence and grow copies check every required step
class and the complete lifetime debit together before doing a transition.
Missing byte or cleanup credit cannot spend a structural/lifetime unit merely
for polling the blocked state. Growth preflight still precedes both structural
and cleanup debit. Repeated partial-credit polls retain the same work trace and
exact transaction lifetime as a sufficient schedule. The wrapper obtains
terminal phase/pass from each cursor's failure, because one step may cross
several phases before fixing a cause.

`tests/schema_build.rs` covers all context fields and seven variants, exact
integer/f64 bits including typed Infinity, optional/empty/null/default
distinctions, nested schema map associations, ordered sequences, opaque
overflow Numbers and collision Objects, mixed small/large budgets, zero-work
boundaries, paid/falsifiable equivalence, exact combined lifetime, cancellation
in every transaction phase, and a 256-level nonrecursive result. Every
canonical Pending boundary can abandon the whole owner with no allocation and
at most eight block releases, including unfinished grow overlap.

The independent allocator runs the complete transaction and injects every
actual request failure. All 65 requests in the rich Host witness are covered,
including initial construction, semantic frames, canonical build and reseal.
Each failure releases all source/output storage; physical peaks match actual
successful requests and logical reservation peaks are distinct. Below/equal/
above temporary, contiguous and total peak limits execute with actual literal
and canonical output residency, rather than independent fixture allowances.
The rich witness retains 4,208 bytes, peaks physically at 18,342 bytes, has a
6,656-byte largest request, and consumes 27,180 work units under its fixed mixed
schedule. Its canonical cursor is 2,000 inline Host bytes. These are observed
fixture costs, not derived M, supported-profile or constrained stack claims.

```sh
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --features validated-thing --test schema_build -- --nocapture
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --features validated-thing,order
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --no-default-features --features async,validated-thing
cargo check --locked --target thumbv7em-none-eabihf --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --no-default-features --features async,validated-thing
```

### Eight obligations remain distinct

| Readmission item | Evidence boundary after this slice |
| ---: | --- |
| 1 | No complete frozen signature/removal or opaque configuration proof. Fixture APIs are not proposed production surface. |
| 2 | Actual literal-to-canonical DataSchema build, grow, source/output overlap, equivalence and exact reseal share one ledger/catalog. The typed Thing slice below extends this emitter; strict Thing construction remains. |
| 3 | Strict DataSchema yields a typed canonical graph with fieldwise public typed parity and shared Basic queries. The typed Thing slice below adds compatibility construction; complete strict differential/query composition remains. |
| 4 | Existing URI/Planning/post-drop witnesses remain separate. This Schema owner cannot enumerate Thing Property/Form coordinates or prove the final Planning handoff. |
| 5 | The existing 13 Host/8 thumb source matrix includes this complete local path. Thumb is compile evidence; full frozen surface/downstream/sibling and constrained execution duties remain. |
| 6 | Whole local schema transaction now owns first-cause rollback across literal construction, Basic, canonical build, equivalence and seal. Whole Thing dates/URI, configuration, canonical-result charged Basic and global diagnostics remain. |
| 7 | Actual requests, logical reservations and physical peaks are distinguished on every failure; literal/output/frame overlap and local release are executed. Derived M, full owner envelope, complete plan/artifact overlap, parent/global release and permit/install remain. |
| 8 | Production Foundation/Context, resource schema, successor evidence and Producer gate inputs are unchanged and registered validation remains applicable. Independent exact-head reaffirmation and the future admitted method removal remain required. |

The tranche remains `planned / candidate / current`; none of the eight is
declared complete as a whole. This constructor produces DataSchema, not a
ValidatedThing. Local green tests cannot readmit production or accept a gate.

## Direct typed Thing construction envelope

The canonical DataSchema result proved its local storage/ownership boundary,
but could not construct the Thing owner consumed by the whole-Basic and
Property/Form query proofs. The new slice composes that missing envelope:
borrowed typed Thing -> charged canonical emission -> charged fieldwise
equivalence -> exact reseal -> independently owned `NormalizedThing`.
It produces construction evidence, not a ValidatedThing admission result.

`src/thing_build.rs` contains borrowed representation facts for every typed
Thing field, Context entry, Property/Action/Event, Form and both response
families, Link, Version, date component, security context and all nine security
variants. All seven DataSchema variants use the established canonical schema
layout inside that owner. No per-schema allocation or independent child owner
is introduced. Typed f64/i64 fields retain their exact bits. Absent slots,
explicit empty collections, opaque nulls, ordered sequences, original Form
indices, map associations and public AP Number contents remain distinct.
The generator adds a private Context borrowing seam only to the build-directory
candidate. The production Context and TD sources are unchanged.

The existing `schema_build::Cursor` supplies the construction/equivalence
mechanics to both entries. `OwnedValue::empty_for_fixture` starts the typed
transaction with empty source arenas, zero allocations, one Accounting owner
and one lifetime remainder. It does not manufacture a literal source or run
a parser. Each typed scalar/header, schema entry, iterator transition, copied
or compared byte, frame transfer and arena transfer is charged. Raw typed URI
copy/comparison pays both source/output `UriBytes` plus codec work; this does
not execute or prove URI parse/resolution. The fixed two-unit URI debit checks
the complete class allowance before spending any credit. All cursors borrow
their source and work owner in lexical scopes; no self-reference, erased input
lifetime or recursive owning task tree is needed.

Caller BTreeMaps use borrowed iterators. JSON extension maps, including a
downstream `preserve_order` map, select their next semantic key with a scalar
continuation that charges each candidate transition and compared byte. They
emit sorted entries directly without a key/sort-buffer allocation. This local
prototype uses selection rather than the authority's future in-place sorting
algorithm; it does **not** discharge that algorithm's construction/progress
duty. Its quadratic work is bounded by the same non-resettable lifetime, and
is observable under varied and partial budgets. It is evidence for the common
storage envelope, not a production algorithm selection or complexity claim.

The sealed result still has exactly three retained allocations, and construction
still uses only node, edge, byte and frame temporary sites. Grow overlap,
cleanup prepayment and one checked Layout per reservation remain the existing
implementation. Every failure is fixed before outer rollback releases the
whole owner. No aggregate footprint is submitted as a contiguous request.
The whole-Thing inline cursor is 2,008 bytes on the measured 64-bit Host;
extending the common cursor also changes the earlier schema-only inline size.
The older schema witness's 2,000-byte observation describes its earlier layout,
not a current capacity guarantee.

`thing_build::Basic` reads only canonical typed facts and invokes the existing
whole-Thing Basic kernel, including its shared schema rules. The existing
170-plus fault corpus executes on the actually constructed owner and compares
complete inline first errors with the typed adapter. This oracle is deliberately
synchronous and external to this lower-level construction trace. The paid
result-Basic slice below includes it in the typed transaction; Basic over the
strict completed Thing remains an unresolved admission duty.
The shared default/security helpers also run on this constructed owner.

`tests/thing_build.rs` supplies eleven executable evidence categories:

- Independent public-field readback of the rich, nested-schema and
  Basic-valid serializer-failure corpora, including every typed field family.
- Sparse envelope fields and explicit-empty maps/sequences/Form overrides
  retain their distinctions, including through shared Basic.
- Identical complete construction traces under mixed 1/7/4096 allowances;
  exact lifetime, one-less failure and repeated withholding of each class.
- Independently counted key comparisons charge both input operands before
  either read. Repeated one-byte polls preserve all credit and lifetime; two
  equal-length keys require twelve byte units per key byte across both passes.
- The existing whole-Basic first-error corpus on the constructed canonical
  graph, with no second predicate or diagnostic authority.
- All 41 rich transaction allocator failures, actual peaks/largest requests,
  zero reallocations, and exact three-block completed-owner release.
- Below/equal/above source, temporary, peak, contiguous, node, edge and byte
  limits; frame/depth rejection; cancellation at build, equivalence and seal.
- Paid output-corruption rejection, zero work and every mixed-schedule
  construction/equivalence Pending abandonment through outer owner cleanup.
- Public AP lexical ceilings, including opaque overflow content and
  zero-disabled rejection before Number output/copy.
- Caller spare capacity, reverse JSON-map insertion and 256-deep opaque
  values, with identical canonical fields and retained arenas.
- An external allocation-free non-first Property/Form query after caller
  Thing drop, with metadata, original index, shared defaults and inherited
  NoSec, then owned selection facts surviving physical normalized-owner drop.

The rich nested-schema 64-bit Host witness retains 34,220 requested bytes,
peaks at 62,464 physical bytes, requests at most 20,480 contiguous bytes and
spends 105,564 construction/equivalence/seal work units. These are fixture
observations, not supported M, profile ceilings, target runtime/stack evidence,
complete plan/artifact residency or Servient owner-capacity claims.

```sh
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --features validated-thing --test thing_build -- --nocapture
```

Existing CI discovers these tests in every capability-on Host invocation and
compiles the entire library in its capability-on thumb cells. No manifest,
lockfile, feature default, resource row, WorkClass or CI orchestration is added.
Thumb remains compilation evidence.

| Readmission item | Contribution / remaining boundary |
| ---: | --- |
| 1 | No frozen API/configuration/removal proof; raw fixture readers and `NormalizedThing` are not proposed production surfaces. |
| 2 | Complete typed envelope and nested schemas construct/grow/compare/reseal under one ledger and the original catalog. Strict Thing decode and the admitted sorting algorithm remain. |
| 3 | Complete typed-field readback, serializer-failure acceptance, canonical whole-Basic first-error parity and default/security queries now consume actual construction. Strict whole-Thing field-policy/differential corpus and full URI/query composition remain. |
| 4 | The constructed owner yields non-first Property/Form coordinates and shared queries after input drop; a small owned selection survives its drop. Cached resolved URI and complete independent Planning output remain with the existing separate witnesses. |
| 5 | The existing matrix compiles/runs this typed envelope. Full frozen surface, downstream/sibling absence and constrained execution remain. |
| 6 | Typed construction/equivalence/seal have exact shared work, lexical limits, cancellation, rollback and prepaid drop evidence. Charged whole-Basic, strict Thing decode, configuration, resumable URI/date handling and global diagnostics remain. |
| 7 | Physical complete typed-owner peak, actual requests, source footprint and release replace independent per-schema allowances. Derived M, complete plan/artifact overlap, parent/global release, Published/permit/install and owner capacity remain. |
| 8 | Production owners, authority and accepted gate inputs are unchanged; independent exact-head reaffirmation and future admitted method removal remain. |

All eight remain incomplete as a set. The tranche remains
`planned / candidate / current`; this slice makes no readmission, production,
Consumer-gate, successor-package or completion transition.

## Paid whole-Thing Basic in the canonical construction transaction

Constructing the complete typed envelope removes the per-schema owner gap,
but a synchronous Basic query after construction does not prove an admission
transaction. It can scan keys, follow references, project Numbers and recurse
outside that transaction's work and temporary-resource envelope. The next
composition boundary is therefore Basic on the actual completed Thing owner.

`thing_build::from_thing_basic` now executes:

```text
borrowed typed Thing -> paid canonical construction -> paid equivalence
                    -> exact reseal -> paid whole Basic -> owned NormalizedThing
```

The lower-level `from_thing` remains useful for constructing invalid parity
inputs. The new path returns only after Basic completes. Both paths retain
the original Accounting owner, lifetime remainder, three source arenas and
four temporary categories. Basic failure fixes its cause and coordinates,
ends all source/frame borrows, then rolls back the complete transaction.
There is no new ledger, independent per-schema allowance, retained owner,
scratch vector, recursive task tree or allocation category.

`basic_kernel::Walk` now owns the whole-Thing discovery order for both the
synchronous reference and `thing_step::Cursor`. Required-field, scheme/flow,
combo shape, operation and DataSchema rules are shared predicates. The paid
driver supplies only canonical representation access and continuations:

- Header/continuation discovery pays DocumentNodes. Security name/group
  traversal also pays SecurityBranches; schema entry also pays JsonSchemaNodes.
- Discriminator and lookup bytes pay CodecInputBytes before inspection. A
  reference lookup compares two retained input operands, caching one paid
  byte before separately paying for the other. Static grammar bytes are not
  input bytes. Length mismatch does not inspect a payload.
- Extension-field lookup is resumable instead of delegating to synchronous
  `Basic`/`SchemaAccess` map searches. Typed fields are constant-size facts.
  A complete local combo shape precedes its references; a known oneOf shape
  error returns before allOf lookup/scanning can consume further credit.
- Schema traversal uses the existing trivially destructible frame category,
  paid capacity transfers and preflight before cleanup/lifetime prepayment.
  Frames overlap the actual retained source in the same ledger, persist across
  schema roots and release physically on completion, failure or Pending drop.
- Unsigned and five binary64 extension projections use the existing bounded
  atomic precharge and NumericCursor. The full actual public Number content
  must fit this poll's byte allowance and remaining lifetime. Cancellation
  before/after projection and failed finite projection retain their distinct
  terminal behavior. Opaque Numbers acquire no Basic projection.

`tests/thing_step.rs` adds nine executable categories:

- The fixed 170-plus Basic corpus, including cross-phase faults, mutated
  discriminators, filtered fallback names and non-first coordinates, runs
  through construction, equivalence, seal and paid result Basic. Complete
  inline first errors match the shared typed reference under varied schedules.
- Basic reads the serializer-failure owner after the caller Thing is dropped;
  each class debit, zero-budget pause and lifetime remainder is observable.
- Exact total lifetime and one-less failure span the entire transaction.
  Repeatedly withholding each class preserves the same complete trace.
- All 44 actual nested-corpus allocation failures, including Basic frame
  requests, match an independent allocator's physical peak and largest actual
  request and release the entire owner. Reservation peak is recorded separately.
- Every mixed-schedule Basic Pending boundary can cancel or abandon with
  prepaid temporary release. The enclosing Basic-stage cancellation rolls back
  source too.
- 256 nested schemas compose with unsigned and binary64 extension projections;
  a narrower standalone frame bound fails without recursive Basic traversal.
- Actual Number contents at 63/64/65 and 255/256/257 exercise standalone Basic
  thresholds; accepted lengths also run through the complete transaction with
  that same limit. Zero, non-Number absence, short overflow, repeated projection,
  partial atomic credit, insufficient projection lifetime and immediate
  pre/post-projection cancellation are observed. The standalone cursor's
  narrower-limit tests are not frozen configuration/admission evidence.
- Singleton oneOf with 0/1/4096 later allOf members preserves the same first
  error and Basic trace, including with exactly the spent lifetime remaining.
- 1/128/512-byte security names independently require two input byte units per
  compared byte, including under one-byte polls, without allocating lookup storage.

The measured 64-bit Host nested corpus spends 107,161 total work units;
Basic contributes `[1236, 300, 31, 27, 3]` in DocumentNodes, CodecInputBytes,
JsonSchemaNodes, SecurityBranches and CleanupItems respectively. It visits 31
schemas and requests three frame blocks. This corpus has no extension Number
projections; the dedicated numeric/deep cases above cover them. Retained source
is still 34,220 bytes and whole physical peak is still 62,464 bytes, with a
20,480-byte largest actual request. The Basic borrowing cursor is 752 inline
bytes on that Host. These observations establish no supported M, production
owner capacity, target stack/runtime or complete plan/artifact residency claim.

```sh
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --features validated-thing --test thing_step -- --nocapture
```

The existing 13 Host runtime and eight thumb library matrix cells discover
the shared kernel and capability-on driver. Thumb remains compilation evidence.
No production source, manifest, lockfile, feature default, WorkClass, resource
row, PLAN, gate or CI orchestration changes.

| Readmission item | Advance / remaining boundary |
| ---: | --- |
| 1 | No frozen owning cursor/configuration/removal or negative surface proof. The borrowing fixture cursor and NormalizedThing remain prototype-only. |
| 2 | Typed build/equivalence/reseal/result-Basic compose within the original catalog/ledger. Complete strict Thing construction and prescribed in-place sorting remain. |
| 3 | Paid canonical result Basic preserves the fixed typed first-error corpus and numeric rules. Complete strict Thing differential field-policy and resolved URI/query composition remain. |
| 4 | The same owned canonical result is now Basic checked under budget. The cached URI view and complete owned Planning/artifact handoff remain separate witnesses. |
| 5 | The current matrix covers this composition. Frozen/downstream/sibling public surface and supported-cell evidence remain incomplete. |
| 6 | Whole typed construction and paid result Basic share exact work/lifetime, Number progress, failure and prepaid cleanup. Strict Thing decode, frozen configuration, in-place sorting, resumable URI/date handling and global diagnostics remain. |
| 7 | Basic/source physical overlap, actual failed requests and complete rollback are measured under one ledger. Derived M, full plan/artifact residency and post-plan release, parent/global/Published accounting and owner capacity remain. |
| 8 | Production authority and accepted predecessor/gate inputs are unchanged. Independent exact-head reaffirmation and future admitted removals remain required. |

All eight remain incomplete as a set. This slice preserves
`planned / candidate / current`; it supplies no readmission or completion.

## Owning suspension shared by strict and compatibility results

The post-construction semantic pass must survive moving its source owner across
`Pending`. The earlier borrowing cursor retained `View`, `&str`, frame-accounting
and lifetime borrows into a stationary owner. Its synchronous driver ended those
borrows before returning, so it did not establish an owning admission cursor.
Pairing that cursor with its source and returning both fails Rust's E0515/E0505
checks. This is a prerequisite to composing the strict path with the existing
whole-result evidence, independently of how many Thing fields are decoded.

`thing_step` now uses one discovery/rule driver with private node indices and
byte ranges in every persistent state and frame. Static grammar references are
`&'static str`; continuation handles retain no source pointer. There is no
lifetime erasure, Pin, boxed cursor, second rule body, or per-Pending arena
traversal. Only a step's transient runner
binds the handles to immutable arena borrows. It ends all those borrows before
returning a movable continuation.

The original frame site can now belong to the same movable source/accounting
owner. `staged::Inspection` owns that workspace, including any old/replacement
frame overlap, and lends a facade for each step. Ending a loan preserves the
workspace; destroying its owner releases both frame blocks before the source.
The existing borrowed facade still owns and releases its workspace. Both use
the same checked allocation, paid transfer and exact release implementation.
`OwnedInspection` moves the original lifetime remainder with that same ledger;
none is recreated or cloned when the cursor is suspended or resumed.

The compatibility construction transaction now actually moves an `OwningCursor`
through its paid whole-Thing Basic pass. Strict `schema_build::from_json_basic`
composes literal JSON decoding, duplicate resolution/compaction, shared schema
field policy, literal Basic, canonical construction/equivalence and exact reseal
with that same owning result-Basic driver. Schema scope starts at the canonical
schema root and stops there; Thing scope uses the existing whole-Thing discovery
program. No typed Thing or serde graph is needed by the strict transaction.

`tests/owning_basic.rs` exercises six categories:

- Both source paths produce a `'static` post-construction cursor after the
  original input is destroyed. Address-observable inline slots move the complete
  cursor on every Pending; varied schedules preserve debits, first cause and
  lifetime. Public typed field readback checks the strict canonical result.
- Strict construction through owning result Basic uses one lifetime and original
  allocation owner. Exact total lifetime succeeds; one-less fails in result
  Basic. An independent allocator checks all 62 observed request failures,
  physical peaks, largest attempted requests, rollback and the three final
  source allocations.
- Every Pending in a nested strict witness can cancel or abandon the whole owner,
  including an incomplete frame growth. Releases are prepaid, fixed by the
  arena catalog, and do not visit the nested semantic graph.
- A 256-level strict schema continues after its input is dropped, including real
  unsigned and binary64 predicates, frame growth/copies and owner moves.
- Literal Number/RawValue-looking objects remain Objects through canonical
  construction and result Basic. JSON-looking strings remain strings. Escaped
  duplicate field names use the last value for dispatch; malformed overwritten
  bytes still fail. The ordinary Number-wrapper observation is separately
  classified as a wire-value difference, not a different Basic rule.
- Actual Number content at 63/64/65 and 255/256/257 spans strict input limits and
  the owning result's atomic projection. Repeated partial polls preserve unused
  credit and lifetime; sufficient credit charges the full lexeme before parse.

The existing nine whole-Thing categories run the owning transaction too,
including the 170-plus first-error corpus, numeric cancellation and every
whole-transaction allocation failure. The borrowed inspection tests remain
useful independent scheduling/resource stresses over the same driver.

On the measured 64-bit Host, the fixed strict witness spends 12,036 total work
units, retains 2,753 bytes, peaks at 13,249 physical bytes and attempts a largest
6,656-byte request. Result Basic visits five schemas, projects two unsigned and
two binary64 values, and contributes `[222, 71, 5, 0, 3]` work units. Its owning
cursor occupies 1,336 inline bytes; the borrowing runner occupies 752 bytes.
The original whole-Thing witness retains its 107,161 work, 34,220-byte source,
62,464-byte peak and 44 requests. These are fixture observations; inline cursor
bytes remain an owner-capacity duty, not an additional allocation request.

```sh
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --features validated-thing --test owning_basic -- --nocapture
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --no-default-features --features async,validated-thing
cargo check --locked --target thumbv7em-none-eabihf --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --no-default-features --features async,validated-thing
```

The registered 13 Host/eight thumb cells include this composition. Thumb remains
compilation evidence. The unchanged arena and literal-value suites cover the
borrowed facade and prior catalog responsibilities.

This establishes the common **post-construction** ownership seam only. Earlier
strict field projection, schema traversal, canonical construction/equivalence
and reseal still use borrowing drivers. The full frozen owning entries have not
been assembled, and this post-construction facade is not a proposed production
API or proof of the authority's complete phase order. Complete strict Thing,
Form, Context, security, Link and response field policies remain absent.
Canonical typed-map construction still uses selection rather than the prescribed
in-place sort. The shared date decoder and URI/default/security query witnesses
have not been joined to these complete construction owners. The opaque admission
configuration, derived supported Number maximum M, full diagnostics, complete
Planning/artifact residency and release, parent/global pairing, publication,
negative frozen-surface matrix and constrained execution also remain unproved.

This slice advances ownership constructibility in item 1, the shared catalog in
item 2, strict canonical/result-Basic composition in item 3, feature evidence in
item 5, movable work/cancellation/drop in item 6 and physical overlap/release in
item 7. It completes no readmission item as a whole; items 4 and 8 retain their
separate duties. No fifth temporary category, fourth retained arena, new ledger,
WorkClass or changed architectural contract was needed at this seam. Earlier
accepted borrowing evidence remains valid within its stated boundary; its
extension to a complete owning entry remains unproved. Admission remains
`planned / candidate / current`, with fresh independent review required.
