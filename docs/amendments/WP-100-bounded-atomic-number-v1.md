# WP-100 Bounded Atomic Number v1

Status: authority candidate for `WP-100-CONSUMER-VALIDATED-THING`; not admitted.

Decision source: workspace topic 0069 / github-pr:86.
Migration record: workspace topic 0070.

This amendment owns the WP-100 numeric boundary that supersedes the affected
exact-decimal / byte-resumable clauses previously projected from workspace
0067 and 0068. It does not authorize production Rust, readmit the tranche, or
change existing Consumer gates.

## Scope

This amendment applies only to bounded `ValidatedThing` admission and the
shared Basic semantics for the five extension predicates that inspect a
`serde_json::Value::Number` under the keys:

- `minimum`;
- `exclusiveMinimum`;
- `maximum`;
- `exclusiveMaximum`; and
- `multipleOf`.

It does not redefine every JSON Number as binary64. Lossless Number storage,
typed `NumberSchema`, typed `IntegerSchema`, and opaque JSON-valued fields are
separate concerns.

## Hard lexical resource boundary

Bounded admission has one named per-Number lexical limit:
`number_lexeme_bytes_max`. Its owner is the Consumer role's
`+validated-thing` admission surface. The row is provisional authority input
until that owner is implemented; generated getters do not enforce admission.

- project hard maximum: **256 bytes**;
- gateway profile: 256 bytes;
- directory-client profile: `NA` (no Directory-client validation owner);
- benchmark static reference profile: 64 bytes;
- profiles may lower the value but may not raise it above 256;
- zero disables Number admission under the normal disabled-resource rule.

Let `L` be the configured value, validated in `0..=256` before admission.
The limit is checked before any input-sized numeric projection. Strict JSON
admission stops as `Limit` when byte `L + 1` of one Number token is observed,
before copying that byte or finishing the token scan. Thus the boundary is
64/65 for the benchmark profile and 256/257 at the hard maximum. At `L = 0`,
the first Number byte returns `Limit`; a document with no Numbers is not
rejected by this resource. Typed compatibility admission uses borrowed public
`Number::as_str()` under the
explicit `td/validated-thing -> serde_json/arbitrary_precision` capability and
checks `as_str().len() <= L` before projection or lossless copy. A value above
256 is invalid configuration, not a per-input `Limit`. This validation belongs
to the future owning builder; raw Foundation `ResourceLimits` remains low-level
assembly and is not that builder.

Over-ceiling Number text is a resource `Limit`, not `InvalidSchema` and not a
claim that the JSON syntax is invalid.

## Storage domain

Every at-or-below-ceiling Number that belongs in the normalized snapshot is
retained losslessly in the project-owned byte arena. No finite-`f64` projection
is required merely to retain a Number.

Consequently values such as an AP-backed `const: 1e309` may be retained when
Basic performs no arithmetic on that value. `const`, `default`, nested
extension Numbers, and other opaque JSON values do not become binary64 merely
because the runtime stores them.

The existing fieldwise semantic-equivalence rule continues to compare retained
Number content losslessly rather than through floating-point equality.

## Computation domain

Existing typed numeric behavior is preserved:

- typed `NumberSchema` comparisons remain the current `f64` behavior and
  acceptance surface;
- typed `IntegerSchema` comparisons remain `i64`;
- this amendment adds no NaN/finite strengthening to either typed schema rule.

For the five extension predicates only, when the extension value is a Number:

1. obtain the stable public `Number::as_f64()` projection used by the selected
   serde graph;
2. require the projection to exist and be finite;
3. otherwise return Basic `InvalidSchema` rather than treating the Number as
   absent;
4. use ordinary binary64 ordering/positivity for the existing predicate; and
5. keep current non-Number-as-absent behavior.

`multipleOf` retains only its existing Basic strict-positivity check; this
amendment does not add divisibility validation.

Binary64 rounding is deliberately part of these five predicates. The runtime
does not promise arbitrary-precision arithmetic or reproduce private parser
quirks as product semantics.

Ordinary public `Thing::validate_with_level(Basic)` remains synchronous and
shares this five-predicate acceptance rule. It is not bounded admission and
therefore does not expose `Pending` or the lexical resource `Limit` terminal.
Resource admission and Basic semantic validity remain distinct.

## Atomic work and cancellation

JSON tokenization, lossless Number-byte capture, and byte copying remain
ordinary charged/resumable work. Numeric projection/comparison after the hard
lexical boundary is a bounded atomic operation.

For one projection of a Number with lexical length `n`, where `0 < n <= L <= 256`:

- debit `n` `CodecInputBytes` units from the current step budget before
  projection starts;
- debit the same `n` units from the shared non-resettable admission lifetime
  remainder before projection starts;
- if the current step budget is insufficient, return `Pending` before starting
  and make no numeric progress;
- if the lifetime remainder is insufficient, return `Limit` before starting;
- check cancellation immediately before and after the projection; and
- perform only constant-size scalar comparison after projection under the
  containing schema-node charge.

The maximum uninterrupted numeric projection consumes a Number of at most 256
lexical bytes. Input finiteness alone does not establish acceptable latency or
stack use. The [constrained atomic Number workload](../../tools/architecture-fixtures/bounded-atomic-number/README.md#constrained-acceptance-workload)
defines the falsifiable target acceptance boundary for
`CONSTRAINED-PROGRESS-001`: every measured projection interval on the declared
168 MHz Cortex-M4 must fit 168,000 cycles (1 ms), with at most 4,096 additional
stack bytes including callees. This is an admission requirement, not a measured
result. The 256 ceiling remains a candidate until this workload passes and its
evidence is independently accepted. Failure requires reopening the ceiling or
projection decision, not increasing these budgets silently.

If a Number must be projected again, the repeated projection is charged again.
No replayed scan is free merely because the source is already retained.

## Feature boundary

The explicit `td/validated-thing -> serde_json/arbitrary_precision` edge from
topic 0068 remains required for stable borrowed, lossless lexical access in
bounded admission and for normalized Number retention.

It is not an arbitrary-precision computation promise. The previous
Display-driven exact-decimal base-graph comparison path is superseded.
Capability-off public Basic can perform its synchronous binary64 predicate
without borrowed lexical access because it is not the bounded admission path.

The previously accepted Host/thumb/downstream feature matrix and semver floor
remain unchanged.

## Evidence required before readmission

The full WP-100 pre-readmission set remains required. Numeric evidence must now
include:

- configured `L - 1`/`L`/`L + 1` thresholds for nonzero `L`, including
  63/64/65 and 255/256/257, plus zero-disabled first-byte rejection;
- over-ceiling strict input returning `Limit` without finishing an unbounded
  token scan;
- typed AP-backed length check before projection/copy;
- opaque within-ceiling non-finite-projecting Numbers retained losslessly;
- short within-ceiling projection failure such as predicate `1e309` returning
  `InvalidSchema`;
- normal binary64 rounding cases, including values beyond exact integer
  precision;
- unchanged typed `NumberSchema`/`IntegerSchema` behavior;
- strict/typed/capability parity for the five predicates where the same typed
  Number is constructible;
- insufficient current step budget -> `Pending` before work;
- insufficient lifetime remainder -> `Limit` before work;
- zero-budget no-progress;
- repeated projection charging;
- one <=256-byte bounded cancellation interval satisfying the declared
  constrained latency/stack workload, including slower fallback inputs; and
- Host plus real constrained/thumb coverage.

The #81/#82 very long Number witnesses become negative resource-boundary tests,
not required successful arithmetic inputs. Topic 0068 lexical-access tests
remain relevant.

## Supersession

This amendment supersedes the following prior authority wherever it conflicts:

- workspace 0067 exact finite-decimal arithmetic for the five extension
  predicates;
- workspace 0067 byte-resumable exact-decimal comparator requirements;
- workspace 0068 synchronous Display-driven exact-decimal comparison; and
- derived wording in the WP-100 admission, Foundation work projection,
  runtime-safety projection, and WP-100 package projection that requires those
  algorithms.

It preserves all unrelated Basic rules, the normalized three-retained /
four-temporary arena design, rollback and accounting, storage-independent
`ValidatedThingView`, public signatures, the explicit AP capability boundary,
and existing feature/target matrices.

The tranche remains `planned` / `candidate` / `current`. Independent exact-head
authority review, completion and independent acceptance of all eight
pre-readmission evidence items, and only then a separate docs-only
`candidate -> admitted` transition are required before production implementation.
