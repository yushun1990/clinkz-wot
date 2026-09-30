# Shared DataSchema Basic convergence prototype

Non-production evidence for
[`WP-100-CONSUMER-VALIDATED-THING`](../../../docs/work-packages/WP-100-consumer-validated-thing-admission.md#evidence-required-before-separate-readmission),
primarily items 3 and 6. The tranche remains `planned` / `candidate`.

The existing typed corpus and three-arena Snapshot already retain every schema
variant, nested schema, typed scalar, and extension Number. Their storage
readback does not establish that those two representations accept the same
schemas. The [default/security probe](../../../td/tests/support/semantic_kernel_probe.rs)
and URI probe answer different semantic questions; the
[post-drop Planning witness](../validated-thing-planning-handoff/README.md)
answers an ownership question. None supplies DataSchema Basic validation.

This fixture connects the existing representations to one rule source and
connects its numeric predicates to the existing atomic precharge mechanism.
Another field-storage slice would not falsify that missing connection. Full
Thing Basic, direct JSON construction, and whole-admission resource/progress
proof remain separate obligations.

## Reproduce

From the repository root:

```sh
cargo test --locked -p clinkz-wot-td --lib schema_kernel
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

`build.rs` copies the current TD source into Cargo's build directory. It
replaces only the final private DataSchema validation region with an adapter
to this kernel, leaving the real public Thing/component validators, builders,
deserializers, field layout, and remaining semantics in that candidate. The
unchanged production TD crate is compiled beside it as the independent oracle.
No generated source is committed, and the production crate never uses it.
The candidate is a normal library plus the explicit public integration test;
upstream TD unit-test harnesses are run in the repository's real crate instead.

The public-source tests compare unaffected acceptance and exact public error
text, exercise the existing rich/nested/serializer-failure corpus in graphs
where its typed inputs are constructible (the nested corpus needs AP), and check
the five-predicate delta at root schemaDefinitions/uriVariables, Property
schemas and URI variables, nested oneOf/items/object children, Action
input/output/URI variables, and every Event schema/URI-variable location.
Minimal still bypasses Basic. No production public behavior is changed.

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

The semantic and projection intervals observe zero Host heap allocations.
The Snapshot retains the same three arenas, and the new adapters/sinks add no
temporary or retained allocation category. Host tests bound NumericCursor at
128 bytes and its inline diagnostic at 16 bytes, with neither requiring drop.
These are local state bounds, not builder/Servient inline-owner capacity or
target stack measurements. The scalar Number continuation needs no arena of
its own; future nested traversal must use the already authorized frame arena.

The complete schema visitor is intentionally synchronous and recursive.
Schema-node, map/key lookup, unsigned extension projection, byte comparison,
and nested traversal charging,
nonrecursive continuation, global document ordinals, depth handling, complete
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
| 3 | One complete DataSchema Basic rule body, typed/Snapshot parity, public-source candidate call graph, unaffected first-error text, numeric amendment and opaque storage. Full Thing Basic/security/affordance rules and direct strict entry remain required. |
| 4 | Existing URI/default/security and post-drop Planning proofs remain usable; this fixture adds no Planning claim. Eventual public View/construction still must repeat them. |
| 5 | Public-source candidate compiles/runs across the described cells; Snapshot tests execute their existing AP graphs. Complete API/construction/sibling/downstream and target execution remain required. |
| 6 | The existing atomic numeric precharge now drives shared schema predicates in both representations. Whole schema/admission progress, configuration, sorting, URI, seal, cancellation/rollback/drop remain required. |
| 7 | Allocation-free local rule/projection intervals and fixed scalar state; no new category. Complete supported M, source/temporary/peak/contiguous accounting, parent/global release, Published and owner capacity remain required. |
| 8 | No authority, resource schema, Foundation/Context implementation, completed successor evidence, or gate status changes. Full impact reaffirmation and the future admitted Foundation method removal remain required. |

This fixture closes a schema-semantic composition gap at prototype level. It
does not complete any readmission item as a whole, perform an admission
transition, implement production normalization, or register a Consumer gate.
