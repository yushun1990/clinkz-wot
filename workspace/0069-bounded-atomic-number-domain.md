# 0069 Bounded Atomic Number Domain

Status: PROPOSAL / REVIEW CORRECTION APPLIED

Kind: ADR-0013 scoped impact proposal for `WP-100-CONSUMER-VALIDATED-THING`

Baseline: current `master` after github-pr:85.

Initial proposal exact head: `6e2c89282e828394fd83e5827a68d2262bd4e65d`.

Independent review of that exact head accepted the bounded-atomic direction with
one required correction: binary64 is a WP-100 **computation-domain** boundary,
not a claim that every WoT/JSON Number must project to finite `f64`. This
revision applies that correction. It still does **not** change authoritative
Basic semantics, readmit the tranche, authorize production Rust, or supersede
workspace topics 0067/0068.

## Problem

Github-pr:81 correctly proved that `serde_json::Number::as_f64()` cannot be
used as an unbounded charged cursor primitive when a legal Number lexeme may be
arbitrarily long. Github-pr:82 then correctly proved that replacing the query
with an exact-value-to-float projection cannot reproduce every observed
`as_f64`/Basic result for pathological long spellings. Topics 0067 and 0068
therefore selected exact finite-decimal Basic semantics, resumable byte-charged
comparison, and an explicit `validated-thing -> arbitrary_precision` source
capability.

The open product question is earlier than those implementation choices:

> Must a general-purpose WoT runtime admit arbitrary-length Number text and
> make every numerically represented JSON value part of an arbitrary-precision
> computation domain merely so bounded admission can inspect it?

For ClinkZ-WoT the proposed answer is **no**. A constrained runtime already
rejects otherwise syntactically legal documents when they exceed admitted
memory, work, depth, URI, or structural limits. Numeric text should have an
explicit admitted resource bound as well. Requiring byte-level resumability
solely so one Number may contain hundreds of thousands of characters turns a
resource-policy case into a general arbitrary-precision arithmetic problem.

This does **not** mean every Number is a binary64 value. The existing TD model
already distinguishes computational numeric fields from opaque JSON values:

- typed `NumberSchema` bounds use `f64`;
- typed `IntegerSchema` bounds use `i64`;
- extension/document values such as `const`, `default`, and nested JSON values
  remain `serde_json::Value` and may need lossless retention without any Basic
  arithmetic projection.

A short AP-backed value such as `const: 1e309` is therefore not invalid merely
because it has no finite `f64` projection. If Basic does not compute with that
value, normalized admission preserves it losslessly subject to ordinary
resource limits.

## Candidate direction

Treat the numeric comparisons that WP-100 actually computes as **bounded atomic
operations**, while preserving non-computational Number values losslessly.

1. The generic numeric computation domain for typed `NumberSchema` floating
   bounds and the five Basic numeric extension predicates
   (`minimum`, `exclusiveMinimum`, `maximum`, `exclusiveMaximum`, and
   `multipleOf`) is finite IEEE-754 binary64 unless a future specialized
   profile explicitly introduces a wider numeric computation type. Existing
   typed `IntegerSchema` comparisons remain `i64`; opaque JSON Number values do
   not acquire an `f64` requirement merely by being Numbers.
2. Admission defines one explicit hard maximum Number lexical size. Profiles
   may lower this ceiling but may not raise it beyond the project-wide atomic
   bound. The exact constant and justified atomic work quantum must be frozen by
   the authority migration rather than guessed by this proposal.
3. The lexical ceiling applies before input-sized numeric conversion. A Number
   whose representation exceeds it is not declared invalid JSON or
   mathematically invalid; bounded admission terminates with a truthful named
   resource `Limit`. Strict JSON admission charges/checks the Number token as it
   is lexed. Typed-`Thing` compatibility checks borrowed public Number text.
4. A Number at or below the ceiling may still be retained losslessly when no
   arithmetic rule uses it, including values outside finite binary64 range.
   `td/validated-thing -> serde_json/arbitrary_precision` remains useful for
   stable borrowed lossless Number text and feature-graph consistency; it is
   not a promise of arbitrary-precision arithmetic.
5. Before an atomic conversion/comparison starts, the cursor must debit its
   full bounded work quantum and corresponding non-resettable lifetime
   allowance. If the current step budget is insufficient, it returns `Pending`
   without starting that atomic operation. Cancellation is checked before and
   after it. The hard lexical ceiling therefore supplies an explicit worst-case
   cancellation latency and work interval.
6. When one of the five Basic predicates requires a floating projection, an
   at-or-below-ceiling Number whose finite binary64 projection fails is
   `InvalidSchema`; it is never silently treated as an absent bound. Normal
   binary64 rounding is part of this selected computation domain. Values such
   as `9007199254740993` do not acquire arbitrary-precision comparison semantics
   merely because their mathematical integer value exceeds binary64 exact
   integer precision.
7. Typed integer comparison semantics remain unchanged. Lossless normalized
   storage also remains distinct from arithmetic projection: retaining exact
   Number text does not imply exact-decimal Basic arithmetic.

This proposal distinguishes three independent questions that topics 0067/0068
currently couple:

- **storage domain:** which legal typed/JSON Number values the normalized
  snapshot preserves losslessly;
- **computation domain:** which numeric representation a particular TD rule uses
  when it actually performs arithmetic/comparison; and
- **execution grain:** what bounded operation may run atomically between two
  cancellation points.

A bounded atomic operation is compatible with `CONSTRAINED-PROGRESS-001` when
its worst-case input size and full work debit are known before it starts.
Resumability is required when an operation's admitted worst-case cost exceeds
the selected atomic-work bound, not merely because its implementation loops
over bytes internally.

JSON lexing, lossless Number-byte capture, and other input-sized copying remain
ordinary charged/resumable work. The proposal changes only the need to make the
**conversion/comparison itself** byte-resumable once its source is below the
hard atomic ceiling.

## Independent review result on the initial exact head

The independent review of `6e2c89282e828394fd83e5827a68d2262bd4e65d`
found the bounded atomic direction sound for WP-100 numeric comparisons, with
this revision's storage/computation distinction required.

The review also established the following migration constraints:

- github-pr:81 and github-pr:82 prove that unbounded `as_f64()` cannot be cursor
  work; they do not prove byte-level resumability is required after a small hard
  lexical cap;
- strict decode can charge/check Number token length before conversion, while
  typed compatibility can use borrowed `Number::as_str()` under the existing
  `td/validated-thing -> arbitrary_precision` capability;
- the current small-positive-budget Number guarantee must be replaced: a step
  with insufficient budget for one atomic numeric operation may return
  `Pending` with no numeric progress;
- the authority migration must state the bounded cancellation latency implied
  by the chosen hard ceiling and work quantum;
- github-pr:81/github-pr:82 long-Number cases remain useful as negative
  limit-boundary witnesses, while github-pr:84/github-pr:85's public-access
  findings remain relevant;
- the normalized snapshot, public signatures, typed integer comparisons, opaque
  JSON-value retention, unrelated Basic rules, and existing gates do not
  change.

Because this revision changes the reviewed exact head, a final exact-head
acceptance still belongs to review before this proposal is merged or migrated.

## Consequences if accepted

Acceptance requires a separate authority migration rather than treating this
proposal as authority. That migration should explicitly supersede only the
affected parts of topics 0067/0068 and preserve 0068's AP access/feature-matrix
finding.

At minimum it must:

- replace the admission record's exact-decimal rule, synchronous Display
  comparison, Number work table, and affected portions of pre-readmission items
  3 and 6;
- define an append-only named per-Number lexical resource limit, with one hard
  project ceiling that profiles may lower but not raise, plus a justified
  atomic work quantum;
- classify over-ceiling Number text as `Limit`, and a within-ceiling Number used
  by one of the five Basic predicates whose finite binary64 projection fails as
  `InvalidSchema`;
- preserve lossless storage of within-ceiling opaque Numbers even when they are
  outside finite binary64 range;
- retain `td/validated-thing -> serde_json/arbitrary_precision` only as the
  stable lexical-access capability unless another independent requirement
  justifies broader semantics;
- update the corresponding runtime-safety, Foundation work rule, WP-100 core,
  resource schema, and tranche-index projections; and
- require threshold, rounding, short-overflow, strict/typed parity, insufficient
  budget, atomic cancellation-latency, Host/thumb, and lossless opaque-Number
  evidence before any later readmission.

The proposal does **not** reopen the normalized retained-snapshot decision from
0065. Project-owned retained arenas, allocation accounting, rollback,
fieldwise semantic equivalence, Planning's storage-independent view, and the
rest of WP-100 remain independent of this numeric execution-grain question.

It also does not imply that every library call is automatically an acceptable
atomic primitive. An atomic operation must have an admitted worst-case input
bound small enough for the supported constrained profiles, and its complete
work/lifetime cost must be enforceable before the operation begins.

## Remaining review questions

A final exact-head review should focus only on whether this corrected boundary
is coherent:

1. Is the storage/computation split faithful to the current typed TD model and
   actual WoT contract?
2. Is one hard append-only Number lexical limit, lowerable by profiles but not
   raisable, sufficient to bound atomic conversion/cancellation without
   imposing binary64 semantics on opaque Numbers?
3. Is `Limit` for over-ceiling Number text and `InvalidSchema` only for a
   within-ceiling value that a Basic predicate must project but cannot project
   to finite binary64 the correct terminal split?
4. Can the existing AP capability safely remain as a lexical-access mechanism
   without recreating an arbitrary-precision arithmetic promise?
5. Is any product requirement left that still forces numeric
   conversion/comparison itself to be byte-resumable?

## Stop condition

Do not implement production Rust, change Basic semantics, alter resource rows,
remove the current feature capability, or readmit `WP-100-CONSUMER-VALIDATED-THING`
from this proposal alone.

If final exact-head review rejects the corrected bounded-atomic boundary, keep
current authority and return to impact review. If it accepts it, perform a
separate docs-only authority migration that explicitly supersedes the affected
numeric authority before resuming WP-100 implementation.
