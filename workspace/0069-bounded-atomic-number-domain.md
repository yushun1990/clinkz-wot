# 0069 Bounded Atomic Number Domain

Status: ACCEPTED IMPACT; AUTHORITY MIGRATION IN 0070

Kind: ADR-0013 scoped impact decision for `WP-100-CONSUMER-VALIDATED-THING`

Baseline: master after github-pr:85.

Initial proposal exact head: `6e2c89282e828394fd83e5827a68d2262bd4e65d`.
First corrected exact head: `db325b78cc6735cef7a2ee4513c0e933a13f68e7`.
Final accepted exact head: `07c59f993e85125e7a32747c013a006f5ce6e8f6`.
Accepted by independent review before github-pr:86 was squash-merged as
`d4cf245eb1bf79e607c4851bba4b16a58e83a04e`.

Github-pr:86 accepted the bounded-atomic direction after two review-driven
scope corrections:

1. binary64 projection applies only to the five `serde_json::Value::Number`
   Basic extension predicates; existing typed `NumberSchema` and
   `IntegerSchema` comparison behavior is not amended; and
2. insufficient current step budget returns `Pending`, while insufficient
   non-resettable lifetime allowance returns `Limit` before atomic work starts.

This impact decision did not itself change authoritative Basic semantics,
readmit the tranche, or authorize production Rust. The accepted direction is
being projected into authority by [workspace topic 0070](0070-bounded-atomic-number-authority-migration.md)
and `docs/amendments/WP-100-bounded-atomic-number-v1.md` in github-pr:87.

## Problem

Github-pr:81 correctly proved that `serde_json::Number::as_f64()` cannot be
used as an unbounded charged cursor primitive when a legal Number lexeme may be
arbitrarily long. Github-pr:82 then correctly proved that replacing the query
with an exact-value-to-float projection cannot reproduce every observed
`as_f64`/Basic result for pathological long spellings. Topics 0067 and 0068
therefore selected exact finite-decimal Basic semantics, resumable byte-charged
comparison, and an explicit `validated-thing -> arbitrary_precision` source
capability.

The product question is earlier than those implementation choices:

> Must a general-purpose WoT runtime make arbitrarily long Number text a
> byte-resumable arithmetic problem merely so bounded admission can inspect the
> five Basic extension predicates?

For ClinkZ-WoT the accepted answer is **no**. A constrained runtime already
rejects otherwise syntactically legal documents when they exceed admitted
memory, work, depth, URI, or structural limits. Number text receives an
explicit admitted resource bound as well. Once that bound is known before a
numeric projection starts, the projection/comparison itself may be a bounded
atomic operation.

This does **not** make every WoT/JSON Number a binary64 value:

- typed `NumberSchema` bounds remain their existing public `f64` values and
  retain their current Basic comparison behavior, including existing behavior
  for caller-constructed non-finite `f64` values;
- typed `IntegerSchema` bounds remain `i64` and retain their current comparison
  behavior;
- extension/document values such as `const`, `default`, and nested JSON values
  remain `serde_json::Value` and may need lossless retention without any
  arithmetic projection; and
- only the five Basic extension predicates over `serde_json::Value::Number`
  (`minimum`, `exclusiveMinimum`, `maximum`, `exclusiveMaximum`, and
  `multipleOf`) receive the bounded binary64 projection rule.

A short AP-backed opaque value such as `const: 1e309` is therefore not invalid
merely because it has no finite `f64` projection. If Basic does not compute
with that value, normalized admission preserves it losslessly subject to the
per-Number lexical resource ceiling and the rest of admission's resource
limits.

## Accepted direction

Treat conversion/comparison for the five Basic extension predicates as a
**bounded atomic operation**, while preserving the rest of the typed model and
lossless Number storage semantics.

1. Admission defines one append-only named hard maximum Number lexical size.
   Profiles may lower this ceiling but may not raise it beyond the project-wide
   atomic bound. The authority migration freezes the constant and corresponding
   atomic work quantum.
2. The lexical ceiling is a resource/admission rule, not a JSON syntax or
   mathematical-validity rule. A Number whose retained textual representation
   exceeds it terminates bounded admission with `Limit`. Strict JSON admission
   charges/checks the token while lexing; typed compatibility checks borrowed
   public Number text under the existing validated-Thing capability.
3. A Number at or below the ceiling may be retained losslessly even when it
   cannot project to finite binary64, if no affected Basic predicate computes
   with it. `td/validated-thing -> serde_json/arbitrary_precision` remains the
   stable borrowed lexical-access mechanism; it does not promise
   arbitrary-precision arithmetic.
4. Existing typed `NumberSchema` `f64` comparisons are unchanged. No new finite
   rejection is added to caller-constructed typed bounds. Existing typed
   `IntegerSchema` `i64` comparisons are also unchanged.
5. When one of the five Basic extension predicates consumes a
   `serde_json::Value::Number`, that Number is projected into finite IEEE-754
   binary64 within the admitted lexical ceiling. Normal binary64 rounding is
   part of the predicate computation rule. Failure to obtain a finite
   projection for an at-or-below-ceiling Number used by one of these predicates
   is `InvalidSchema`; it is never silently treated as an absent bound.
6. Before one atomic numeric projection/comparison begins, the cursor determines
   and debits its complete bounded work cost. If the **current step budget** is
   insufficient, the step returns `Pending` with no numeric progress.
7. The same atomic operation must also fit the shared **non-resettable lifetime
   remainder** before it starts. If that allowance is insufficient, admission
   terminates with the existing structured resource `Limit`.
8. Cancellation is checked before and after the atomic operation. The hard
   lexical ceiling plus frozen atomic work quantum supplies an explicit
   worst-case cancellation interval. JSON lexing, lossless Number-byte capture,
   and other input-sized copying remain ordinary charged/resumable work; only
   the bounded projection/comparison ceases to require byte-level continuation.

## Three separate domains

The accepted decision keeps three concerns distinct:

- **storage domain:** which legal typed/JSON Number values the normalized
  snapshot preserves losslessly;
- **computation domain:** which representation a particular TD rule uses when
  it actually compares a value; and
- **execution grain:** what bounded operation may execute atomically between
  cancellation points.

The change narrows only the second and third concerns for the five Basic
extension predicates. It does not redefine all WoT Numbers, all typed numeric
fields, or the normalized snapshot as binary64.

A bounded atomic operation is compatible with `CONSTRAINED-PROGRESS-001` when
its maximum admitted input and complete work debit are known before execution.
Resumability remains required where an admitted worst-case operation exceeds
the chosen atomic bound; it is not required merely because the implementation
internally loops over bytes.

## Independent review history

The independent review of `6e2c89282e828394fd83e5827a68d2262bd4e65d`
accepted the bounded-atomic direction but rejected a blanket finite-f64 claim.
It required the storage/computation distinction and preservation of opaque
Numbers such as `const: 1e309`.

The follow-up review of `db325b78cc6735cef7a2ee4513c0e933a13f68e7`
confirmed that correction and identified two remaining gaps:

- the finite binary64 rule must not amend existing typed `NumberSchema` Basic
  behavior; it applies only to the five `Value::Number` Basic predicates; and
- step-budget exhaustion and non-resettable lifetime exhaustion have different
  terminal behavior: `Pending` versus `Limit` respectively.

Those findings were corrected at
`07c59f993e85125e7a32747c013a006f5ce6e8f6`, which received final independent
acceptance before #86 merged.

The reviews also established that:

- github-pr:81/github-pr:82 prove unbounded `as_f64()` is not cursor work, but
  do not prove byte-level numeric resumability is a product requirement after a
  small hard lexical cap;
- github-pr:84/github-pr:85's AP/public lexical-access finding remains useful;
- long #81/#82 Numbers become negative resource-limit witnesses rather than
  required successful arithmetic inputs; and
- the normalized snapshot, public signatures, unrelated Basic rules, existing
  typed numeric behavior, and existing gates remain independent of this
  decision.

## Required authority migration

Workspace topic 0070 / github-pr:87 owns the migration. It must:

- replace the admission record's exact-decimal rule, synchronous Display
  comparison, Number work table, and affected portions of pre-readmission items
  3 and 6;
- add an append-only named per-Number lexical resource limit with one hard
  project ceiling that profiles may lower but not raise;
- freeze and justify an atomic numeric work quantum and corresponding bounded
  cancellation latency;
- distinguish insufficient step budget (`Pending`) from insufficient
  non-resettable lifetime remainder (`Limit`);
- classify over-ceiling Number text as `Limit`;
- classify a within-ceiling Number used by one of the five Basic predicates
  whose finite binary64 projection fails as `InvalidSchema`;
- preserve current typed `NumberSchema` `f64` and `IntegerSchema` `i64`
  comparison behavior;
- preserve lossless storage of within-ceiling opaque Numbers even when outside
  finite binary64 range;
- retain `td/validated-thing -> serde_json/arbitrary_precision` as stable
  lexical-access capability, not arbitrary-precision arithmetic authority;
- update Foundation work/resource authority, runtime-safety, WP-100 core,
  resource schema, and tranche-index projections; and
- require threshold/boundary, binary64 rounding, short overflow, opaque
  out-of-range retention, strict/typed parity, step-budget `Pending`, lifetime
  `Limit`, cancellation-latency, and Host/thumb evidence before later
  readmission.

That migration must not implement production `ValidatedThing`, readmit the
tranche, or claim the new evidence complete.

## Stop condition

If authority migration cannot enforce a hard lexical ceiling before bounded
numeric projection across every supported strict/typed feature graph, or a
supported constrained target cannot tolerate one fully precharged bounded
projection as an atomic cancellation interval, reopen impact review. Do not
silently return to unbounded `as_f64()`, private parser authority,
byte-resumable exact-decimal arithmetic, or a larger profile-specific ceiling.
