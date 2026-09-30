# ValidatedThing arena layout prototype

This fixture library is non-production source evidence for item 2 of
[`WP-100-CONSUMER-VALIDATED-THING`](../../../docs/work-packages/WP-100-consumer-validated-thing-admission.md#evidence-required-before-separate-readmission).
The TD test-only consumer is also partial item-3 evidence for typed storage and
the narrow shared semantic-kernel slice described below. Neither admits the
tranche nor implements production typed TD conversion. The prototype
deliberately does not claim complete semantic equivalence, work charging,
cleanup prepayment, or a validated admission configuration.

## Reproduce

From the repository root:

```sh
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-arena-layout/Cargo.toml
cargo check --locked --target thumbv7em-none-eabihf --manifest-path tools/architecture-fixtures/validated-thing-arena-layout/Cargo.toml
cargo fmt --manifest-path tools/architecture-fixtures/validated-thing-arena-layout/Cargo.toml -- --check
```

The thumb command compiles the `no_std + alloc` library; it does not execute a
target allocator or establish target memory availability.

## Allocation catalog and formulas

The fixed `Prototype` owns exactly four temporary arena handles: mutable
`RetainedNode`, `RetainedEdge`, and byte build arrays, plus a traversal-frame
array. Sealing retains exactly three possible arrays: exact-length node, edge,
and byte arrays. Empty arrays allocate zero bytes and retain no allocation.
All element types are explicit `Copy` scalar records with no destructor or
nested owning value. The prototype calls `alloc` and `dealloc` itself with
one checked `Layout::array::<T>(capacity)` for each nonempty array.

For one proposed allocation, let `r = Layout::size()`, `S` be live source
requests, and `T` be live temporary requests. Before the allocator call, the
prototype checks:

```text
r <= largest_contiguous_allocation_bytes_max
S + T + r <= peak_live_bytes_per_admission_max
S + r <= retained_source_bytes_per_owner_max       (source request)
T + r <= admission_temporary_bytes_per_operation_max (temporary request)
```

Every sum and the `Layout` construction is checked. Exactly one Foundation
`AdmissionLedger` reservation of `r` bytes precedes exactly one allocator
request of that `Layout`; a rejected reservation performs no allocation.
After successful allocation, the reservation commits. Growth copies the
initialized prefix to the new array, deallocates the old array, then releases
its old charge. Seal makes a source request with capacity equal to the used
length while the corresponding build array is still charged, copies that
prefix, and releases the build array. This records old/new overlap in both
the temporary peak (on growth) and conversion peak (on growth and seal).

The sealed footprint uses the sum of the three live source `Layout::size()`
values, the count of nonempty retained arrays, the ledger's largest actual
single request, the simultaneous temporary peak, and the ledger's total live
peak. The worked test has a 40-to-80-byte node grow, a 120-byte temporary
overlap peak, a 137-byte conversion peak, and a sealed 39-byte/three-allocation
footprint. Tests also cover zero and spare-capacity inputs, a rejected grow
that preserves the old initialized array, source and peak rejection before
seal, contiguous-request rejection, and checked `Layout` overflow.

## Scope of the evidence

This demonstrates that the frozen allocation catalog can be implemented using
the current Foundation ledger without treating an aggregate as one physical
request. It does not prove the complete TD `Thing` traversal fits that catalog.
In particular, typed-field decoding, map ordering, complete URI parsing and
work charging, Basic semantic sharing beyond the narrow slices below, all
structural/work charges, and real-target allocator observations remain
separate pre-readmission obligations. The URI slice below demonstrates a
direct-to-byte-arena resolution path but does not implement its resumable
production cursor. A required fifth temporary allocation category or fourth
retained category in the full conversion still returns the tranche to impact
review.

## Typed corpus storage slice

`td/tests/support/normalized_snapshot_probe.rs` imports this arena source in
TD's test-only `Context` child module. It uses the exact same input constructor
as `td/tests/validated_thing_typed_corpus.rs`. The prototype walks typed
`Thing`, `ContextEntry`, `Form`, and extension `Value` directly and seals one
node, one edge, and one byte arena. Reserved contiguous edge ranges retain
sequence indices and map key/value pairs. Byte ranges retain URI and string
content and AP Number text. No snapshot element owns another allocation. The
input `Thing` is outside the snapshot ownership boundary.

Run `cargo test --locked -p clinkz-wot-td` from the repository root to execute
both consumers of the fixed corpus. The TD test dependency selects serde
arbitrary precision so borrowed `Number::as_str()` is available; the
no-default library remains unchanged.

The test-only placement allows inspection of private `Context.entries`
without expanding the production Context or ValidatedThing API. The fixed
headroom and recursive call stack are prototype mechanics; the full bounded
cursor, traversal-frame use, all known fields/variants, allocation growth,
work charging, complete URI input classification, and complete storage-neutral
Basic validation remain open. Existing Basic is used while constructing the
fixed typed input and as the oracle for the narrow shared semantic slices.
This is partial item-3 evidence, not readmission or item-3 completion.

## Remaining Thing root-field slice

The shared typed corpus and snapshot probe also cover the root metadata fields
`@type`, `description`, and `descriptions`; version instance/model/extensions;
the ordered `profile` URI sequence; and the `schemaDefinitions` and
`uriVariables` schema maps. The probe stores those values directly from the
typed model in the same node, edge, and byte arenas. Tests read every included
field back, preserve tag and profile order, distinguish absent from present
values and present-empty schema maps, preserve schema key/value associations,
and keep map insertion history non-semantic.

This remains a corpus-scoped part of item 3. The recursive fixed-headroom
traversal also remains non-production and does not establish exact work
charging, resumability, a shared semantic kernel, strict entry, or readmission.

## Complete SecurityScheme typed-storage slice

The shared corpus now includes every current typed security variant: `nosec`,
`auto`, `combo`, `basic`, `digest`, `apikey`, `bearer`, `psk`, and `oauth2`.
It exercises the common semantic tags, description, multilingual descriptions,
proxy, discriminator, and extension fields, plus every variant-specific field.
The `combo` references and OAuth2 scopes retain source order; URI fields remain
typed URI text; optional sequences distinguish absent from present-empty.

The snapshot stores each variant discriminator, common context, and typed
variant payload directly in the existing node, edge, and byte arenas. Tests
read all fields back, distinguish representative mutations of every payload,
preserve `oneOf`/`allOf` and scope order, keep definition key/value
associations, normalize map insertion history, and still retain exactly three
sealed allocations. No serializer round trip, opaque security object, nested
owner, or new allocation category is introduced.

This section remains typed-storage evidence. The test-only semantic slice below
adds root/Form/combo name-reference checks, effective Thing/Form inheritance,
and Planning-shaped scheme lookup. Complete SecurityScheme Basic validation,
exact work charging, resumability, and the strict builder remain outside it.

## Complete Form typed-storage slice

The shared corpus now includes one rich Property Form and one minimal Form.
The snapshot retains every current typed Form field: raw `href`, content type,
optional content coding, ordered security names and scopes, primary response
content and extensions, ordered additional responses with optional content
type/schema and success state, subprotocol, ordered operations, and Form
extensions. Expected-response and additional-response records use only scalar
nodes, ranges, and normalized extension values in the same three arenas.

Tests read both Forms back field by field, retain their original indices, and
show that content, ordered lists, response/additional-response content,
subprotocol, extensions, and absent-versus-present-empty distinctions affect
the sealed snapshot. The test-only semantic slice below additionally exercises
Property operation defaults, inherited/overridden security, and raw/resolved
URI queries. Response-content defaults remain outside the probe.

## Complete PropertyAffordance typed-storage slice

The rich Property now includes `observable: true` and a two-entry
`uriVariables` map, while the second Property retains the false/absent shape.
Together with the already retained DataSchema and complete ordered Form list,
the snapshot now covers every current typed `PropertyAffordance` and nested
`InteractionAffordance` field. URI-variable schemas reuse the same recursive
DataSchema representation in the three sealed arenas.

Tests read both Properties back field by field, distinguish the observable
flag, preserve URI-variable key/schema associations, ignore BTreeMap insertion
history, and distinguish absent from present-empty URI-variable maps. The
narrow semantic slice below now covers effective Property operations and
security. The URI slice below additionally covers raw and resolved Property
Form targets; URI-template expansion remains a later Planning/runtime concern.

## Shared Property/default/security semantic-kernel slice

`td/tests/support/semantic_kernel_probe.rs` is included only by the existing TD
snapshot test module. It defines one private borrowed semantic-access contract,
one adapter for the current typed `Thing`, and one adapter for the sealed
node/edge/byte Snapshot. Adapter methods expose storage facts only. One shared
rule implementation owns:

- Property default operations for all `readOnly`/`writeOnly` combinations,
  including the current neutral fallback for the Basic-invalid both-true case;
- preservation of explicit operation order and explicit-empty operations;
- Form security override, Thing security inheritance, and explicit-empty Form
  security without accidental inheritance; and
- required nonempty Thing security plus Thing, every Form, and combo
  `oneOf`/`allOf` name-reference lookup. Combo reference lookup follows the
  mutable typed `scheme` discriminator: a non-Combo Rust variant whose scheme
  is `combo` reads `oneOf`/`allOf` from its extension fields, matching current
  Basic behavior.

The typed adapter is checked against today's production `td_defaults` helpers
and Basic validator. The same fixed typed corpus and semantic mutations are
then run through both adapters. Positive and negative tests cover inherited,
overridden, empty, and undefined names at Thing, Property Form, and combo
definition sites, including a discriminator/enum-variant mismatch mutation.
Borrowed effective-operation and effective-security iterators are
`Clone + ExactSizeIterator` and preserve Property ordinal and original Form
index. A local Planning-shaped probe enumerates a non-first Property/Form,
filters `ReadProperty`, and resolves exactly one NoSec definition without
receiving `&Thing`-specific fields or raw arena ranges.

A thread-local Host counting allocator brackets only the completed query,
reference-validation, fingerprint, and local Planning-probe intervals after
both input representations exist. Both typed and Snapshot paths observe zero
allocation calls and equal semantic summaries. This establishes an
allocation-free implementation path for this narrow query slice; it is not a
real-target allocator observation, work/stack bound, or claim that
normalization allocates nothing.

Run the focused proof with:

```sh
cargo test --locked -p clinkz-wot-td --lib semantic_kernel
```

The following remain explicitly unresolved across these slices:

- complete Basic validation, including schema/affordance/security-scheme
  constraints and complete first-error/diagnostic parity;
- strict-entry URI classification and a resumable, exactly charged production
  implementation of the URI kernel below;
- strict JSON decoding, strict-entry validation, and typed/direct decode
  agreement;
- resumable progress, work charging, lifetime/step budgets, cancellation, and
  bounded rollback; and
- the required external Planning crate fixture using the future public
  `ValidatedThingView`. The source-isolated consumer below checks the frozen
  method shape and information sufficiency, but the public type does not exist
  in production and this probe cannot complete item 4.

No production source, public API, allocation catalog, admission/status record,
or prior evidence conclusion is changed.

## Shared URI and borrowed Planning-view slice

`td/tests/support/uri_semantic_kernel_probe.rs` adds one test-only borrowed URI
kernel. Typed `Thing` and sealed Snapshot adapters provide only the already
classified concrete/template base and reference/template href plus borrowed
text. Both call the same kernel. Concrete URI parsing still uses
`fluent_uri`; relative resolution writes directly into a caller-supplied sink,
including path merging, percent-encoded dot-segment removal, query replacement
or inheritance, and fragment handling. The Snapshot sink is its existing
mutable byte arena. It creates no temporary `String`, URI owner, vector, map,
or additional arena.

The differential corpus compares both adapters with today's production
`resolve_form_href` result for:

- absolute targets, relative references, no base, network-path references,
  and opaque bases;
- template href preservation, template-base rejection, and an absolute href
  that bypasses a template base;
- merged and absolute path normalization, including encoded dot segments;
- inherited and replaced queries, fragments, query-plus-fragment references,
  base-fragment failure, and the RFC 3986 normal/abnormal reference matrix.

The production helper remains the oracle in this PR; it is not modified. A
future production implementation must move the existing owned helper and the
Snapshot builder onto one shared sink-based TD kernel. Keeping this test copy
as a second production rule implementation would not satisfy the evidence.

### Why the frozen borrow requires retained derived bytes

For base `https://base.example/a/b/c/` and href
`../d/./e?mode=full#part`, the result is
`https://base.example/a/b/d/e?mode=full#part`. That result combines both inputs
and removes path segments; it is not a contiguous slice of either input. A
query-local parser or fixed buffer therefore cannot return the frozen
`ValidatedFormHref::Reference(&'a str)` with the owner's lifetime.

The evaluated alternatives are:

| Strategy | Query allocation | Frozen API/lifetime | Disposition |
| --- | ---: | --- | --- |
| call today's owned `resolve_form_href` during each query | nonzero in the measured composite case | returns a new owner, not `&'a str` | incompatible |
| parse into query-local fixed scratch | zero | result dies with the query | incompatible |
| return segments or accept caller scratch | zero | changes the frozen API | requires authority review |
| retain parsed URI owner objects | query can be zero | adds nested owners/allocation responsibility | outside the frozen catalog |
| emit resolved bytes during normalization | zero | returned text borrows the Snapshot byte arena | compatible prototype |

The compatible prototype does not require retained parsed objects. Template,
absolute, and no-base results alias the raw href node. Only a composite result
adds one URI node and its bytes; an error adds one scalar error node. Every
Form adds one edge for the cached result. These remain node, edge, and byte
arena contents, not new allocation sites.

For the fixed corpus, the exact incremental retained request is 496 bytes:
9 Form edges × 8 bytes, 7 derived URI/error nodes × 20 bytes, and 284 resolved
URI bytes. The lifetime is exactly the sealed Snapshot lifetime. Physical
responsibility remains the existing three retained arenas and four temporary
arenas. Semantic work remains `UriBytes` for source/output bytes plus
`CodecOutputBytes` for retained output and `DocumentNodes` for emitted
node/edge work; physical bytes remain subject to the frozen source, temporary,
peak-live, and largest-request limits. The prototype adds no WorkClass, limit
row, ledger account, or allocation category.

The Host allocator probe separates intervals:

- constructing and sealing the nonempty Snapshot observes six allocation
  calls: the three existing build arenas and the three exact retained arenas;
- today's owned composite resolution observes a nonzero query allocation;
- direct resolution into an inline/arena sink observes zero allocations; and
- the completed borrowed Planning query observes zero allocations after the
  Snapshot exists.

`planning_view_consumer.rs` is a source-isolated consumer of test-only wrappers
with the exact frozen method set. It can receive only `ValidatedThingView` and
related views. It enumerates Properties, selects ordinal 1 / original Form
index 1, reads raw and resolved URI plus content metadata, verifies effective
`ReadProperty`, resolves inherited security to NoSec, and returns only borrows
and scalars. It has no `Thing`, Snapshot/arena handle, URI resolver, or copied
default/security rule. The complete interval observes zero Host allocation
calls.

Run the focused proof with:

```sh
cargo test --locked -p clinkz-wot-td --lib semantic_kernel
```

This is feasibility evidence only. It does not change the frozen View API,
allocation catalog, resource authority, work-package state, or admission
status. Exact `UriBytes`/`CodecOutputBytes` charging, resumability,
cancellation, strict-entry classification, rollback, actual thumb execution,
and an external crate consuming the eventual public View remain open.

The separate [post-Planning handoff witness](../validated-thing-planning-handoff/README.md)
reuses this borrowed query and Snapshot to prove a concretely owned output can
survive physical Snapshot release without a TD or registration borrow. It
supplies the prototype post-drop part of item 4 after the build-scoped lifecycle
migration; the eventual public View and complete resource/publication proof
remain required.

## Complete ActionAffordance typed-storage slice

The shared corpus now contains a rich Action and a minimal Action. The sealed
snapshot retains all five metadata fields, the complete ordered Form list,
affordance-level URI-variable schemas, optional input and output schemas,
`safe` and `idempotent`, extension fields, and the optional `synchronous` state
when `td2-preview` is enabled. The preview-disabled representation carries an
explicit absent slot, so enabling the feature cannot silently reuse an
unrelated extension path.

Tests read both Actions back field by field and distinguish metadata/Form
order, input/output presence, both flags, extensions, URI-variable
associations, absent versus present-empty maps, and—under `td2-preview`—false
versus absent `synchronous`. BTreeMap insertion history remains non-semantic.
This proves typed storage only; operation defaults, URI-template expansion,
effective security, and shared Basic/equivalence rules remain outside the
prototype.

## Complete EventAffordance typed-storage slice

The shared corpus now contains a rich Event and a minimal Event. The sealed
snapshot retains all five metadata fields, the complete ordered Form list,
affordance-level URI-variable schemas, optional `subscription`, `data`,
`dataResponse`, and `cancellation` schemas, plus Event extension fields. Every
schema reuses the same recursive DataSchema representation in the three sealed
arenas.

Tests read both Events back field by field and distinguish metadata/Form order,
presence of each optional schema, extensions, URI-variable key/schema
associations, and absent versus present-empty URI-variable maps. BTreeMap
insertion history remains non-semantic. This is typed-storage evidence only;
subscription lifecycle, event delivery, URI-template expansion, effective
operations/security, and shared Basic/equivalence rules remain outside the
prototype.

## Complete Link typed-storage slice

The shared corpus now contains a rich Link and a minimal Link in an ordered
root `links` sequence. The sealed snapshot retains the required raw `href`,
optional media type, relation, raw anchor, sizes, ordered `hreflang` values,
and Link extension fields. URI references remain stored as their typed raw
text; this slice does not resolve them against the Thing base.

Tests read both Links back field by field, retain their original sequence
indices, distinguish each optional field and extension, preserve `hreflang`
order, and distinguish absent from present-empty `hreflang` and root Link
sequences. This is typed-storage evidence only; relation interpretation, URI
resolution, language negotiation, effective security, and shared
Basic/equivalence rules remain outside the prototype.

## Complete timestamp typed-storage slice

The shared corpus now includes both root `created` and `modified` timestamps,
including leap-day, nanosecond, non-UTC offset-with-seconds, lowercase
separator, space-separator, and lowercase-UTC spellings accepted by TD's
existing private RFC3339 owner. The typed snapshot stores each resulting
`OffsetDateTime` as fixed scalar year, month, day, hour, minute, second,
nanosecond, and explicit offset-second fields in the same three arenas. It does
not format or reparse the typed values.

Tests read every retained component back and distinguish each timestamp's
presence, local date/time and nanosecond changes, plus a same-instant value
expressed with a different offset. Original lexical spelling and fractional
digits beyond the typed nanosecond result are deliberately not retained by
typed entry. This is storage evidence only: shared resumable RFC3339 decode,
strict-entry progress/charging, lexical error parity, and shared
Basic/equivalence rules remain outside the prototype.

## Nested JSON Object ordering slice

The snapshot probe now reserves each JSON Object's map edges, stores scalar
source-member indices in those slots, and insertion-sorts the slots by key
before emitting any child node or byte. Each sorted index selects its original
key and value together. This keeps nested Object and Object-in-Array layouts
identical across insertion histories, including `preserve_order`; Array and
Form/Context sequence edges retain their original indices. The sort uses only
scalar slots in the existing edge arena. Its input `Thing` and JSON maps are
caller-owned, and no sort buffer or additional allocation category is added.
The test checks full node/edge/byte equality, sorted keys, nested associations,
array-order sensitivity, opaque Number text, equal footprints, and exactly
three retained arena allocations.

Run the probe in separate Host invocations:

```sh
cargo test --locked -p clinkz-wot-td --lib normalized_snapshot_probe
cargo test --locked -p clinkz-wot-td --lib normalized_snapshot_probe --features serde_json/preserve_order
cargo test --locked -p clinkz-wot-td --lib normalized_snapshot_probe --no-default-features
cargo test --locked -p clinkz-wot-td --lib normalized_snapshot_probe --no-default-features --features td2-preview
```

TD's test dependency enables AP in all four invocations, so the first is AP,
the second is AP + order, and the last proves the preview-only Action field in
the no-default graph. The separate feature-boundary matrix checks Host
base/order/AP/combined requests and actual resolved serde features; base and
order without the validated capability remain ordinary TD/Basic graphs and do
not run the AP-dependent snapshot probe. Exact work charging, bounded
resumable sorting, complete Basic/URI/strict-decode equivalence, the public
external Planning fixture, and complete item-3 equivalence remain open.
