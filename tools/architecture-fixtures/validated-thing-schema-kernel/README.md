# Shared Thing Basic convergence prototype

Non-production evidence for
[`WP-100-CONSUMER-VALIDATED-THING`](../../../docs/work-packages/WP-100-consumer-validated-thing-admission.md#evidence-required-before-separate-readmission),
primarily items 3 and 6. The tranche remains `planned` / `candidate`.

The full Basic semantic composition is now exercised by one storage-neutral
rule source over the existing typed Thing and three-arena Snapshot. It combines
the DataSchema and numeric predicate proof from #114 with root, affordance,
security-scheme/reference checks and the default/security seam from #110.
No field-storage slice, production ValidatedThing, or admission builder is added.

The uncertainty addressed here is whether a future admission cursor must invent
another Basic rule set or error precedence while combining those local proofs.
The candidate uses shared component predicates and shared whole-Thing order;
its adapters contain only borrowed representation facts. Strict JSON
construction, charged nonrecursive traversal, and whole-admission resource and
terminal ownership remain separate obligations. The existing
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
public ValidatedThingInvalid API. Neither sink chooses checks or traversal.

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
