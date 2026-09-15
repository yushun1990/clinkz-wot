# 0067 Bounded Exact-Decimal Number Impact

Status: SUPERSEDED IN PART BY 0069/0070

Kind: ADR-0013 scoped implementation-impact review / TD semantic authority amendment

Evidence baseline: master `c7bce96` (github-pr:81 and github-pr:82).

Impact and authority amendment review location: github-pr:83.

Supersession: github-pr:86 accepted workspace topic 0069's bounded-atomic
Number direction. Workspace topic 0070 and
`docs/amendments/WP-100-bounded-atomic-number-v1.md` supersede this topic's
exact finite-decimal Basic arithmetic, byte-resumable comparator, and related
progress/readmission obligations. The #81/#82 counterexamples remain current
evidence that an **unbounded** `Number::as_f64()` call cannot be hidden inside
a bounded cursor step and that private parser quirks must not become product
semantics. They are now negative resource-boundary witnesses rather than a
requirement to support arbitrary-length exact-decimal arithmetic.

The material below is retained as historical decision/evidence context. Where
it conflicts with 0069/0070 or the bounded-atomic amendment, the newer
authority wins.

## Finding and decision

The [Number boundary fixture](../tools/architecture-fixtures/validated-thing-number-boundary/README.md)
exposes two independent failures of the frozen stable-`Number::as_f64` route.
An arbitrarily long exponent is consumed synchronously before the public query
returns, even when the result is `Some(1.0)` or `Some(10.0)`. Decimal-cancellation
witnesses then show that an exact-value-only float replacement also changes the
current Basic result: legal spellings of mathematical one return `None` or
`Some(0.0)` from the observed query. Existing Basic validation can accept
those bounds where an exact comparison rejects them.

The selected direction at this historical review was **exact finite decimal
value** for the TD Basic predicates that inspect `serde_json::Value::Number` in
schema extension fields. That selection is no longer current where superseded
above. The source Number remains lossless and typed fieldwise equivalence
remains separate from arithmetic equality.

This was a planned semantic contract, not production behavior. The tranche
remains `planned` / `candidate` / `current`; the current detailed owner is the
amended [admission record](../docs/work-packages/WP-100-consumer-validated-thing-admission.md).

## Alternatives and impact boundary

The table records the state at the time of #83. Its exact-decimal selection is
historical after 0069/0070.

| Alternative | Historical disposition |
| --- | --- |
| Keep unbounded `as_f64` and charge its containing node or precharge the whole query | Rejected: without an input cap the stable call has no bounded cancellation interval. |
| Preserve the observed float result with a lexeme-aware resumable clone of private parsing | Rejected: it would make rustc/serde implementation quirks product semantics across supported versions. |
| Fold to exact decimal, then produce a correctly rounded `f64` | Rejected at #83 because it did not preserve every pathological historical result. |
| Compare exact decimal values in one resumable TD semantic kernel | Selected at #83; superseded by the later bounded-atomic finite-binary64 predicate boundary. |

The finding concerned numeric `minimum`, `exclusiveMinimum`, `maximum`,
`exclusiveMaximum`, and `multipleOf` extension fields, including extensions on
non-numeric schema variants that current validation inspects. It did not change
Number syntax, parsing acceptance, typed `NumberSchema` / `IntegerSchema`
field types or comparisons, JSON serialization, default operations,
URI/security rules, or public method signatures/error categories.

The affected requirement set remains `DOC-RUNTIME-001`, `ADMIT-MEM-001`,
`ADMIT-TXN-001`, `CONSTRAINED-WORK-001`, `CONSTRAINED-PROGRESS-001`, and
`CONSTRAINED-OWN-001`. The current 0070 migration adds exactly one named
`number_lexeme_bytes_max` resource row but still adds no new WorkClass, ledger
account, allocation category, lifecycle state, Planning input, or Servient
owner.

## Evidence and stop condition

The #81/#82 fixture remains evidence against the **unbounded** stable query and
against cloning private parser behavior as product semantics. Under the newer
bounded-atomic authority, very long Number witnesses are expected to hit the
named lexical `Limit`; short values still test deliberate binary64 rounding and
projection-failure behavior.

All eight pre-readmission items remain required and incomplete as a set. Their
current numeric obligations are defined only by the admission record, topic
0070, and the bounded-atomic amendment. This historical review creates no
completion evidence and readmits no tranche.
