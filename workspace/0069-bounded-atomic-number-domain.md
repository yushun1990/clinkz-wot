# 0069 Bounded Atomic Number Domain

Status: PROPOSAL / REVIEW REQUIRED

Kind: ADR-0013 scoped impact proposal for `WP-100-CONSUMER-VALIDATED-THING`

Baseline: current `master` after github-pr:85.

This proposal does **not** change authoritative Basic semantics, readmit the
tranche, authorize production Rust, or supersede workspace topics 0067/0068.
It asks whether their unbounded exact-decimal/resumable-Number direction solved
a stronger problem than ClinkZ-WoT actually needs to support.

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

> Must a general-purpose WoT runtime admit arbitrary-length JSON Number text
> and arbitrary-precision decimal values into its bounded runtime domain?

For ClinkZ-WoT the proposed answer is **no**. A constrained runtime already
rejects otherwise syntactically legal documents when they exceed admitted
memory, work, depth, URI, or structural limits. Numeric input should have an
explicit admitted domain as well. Requiring byte-level resumability solely so a
single Number may contain hundreds of thousands of characters appears to turn
a resource-policy case into a general arbitrary-precision numeric engine.

## Candidate direction

Treat numeric work as a **bounded atomic operation**, not as intrinsically
byte-resumable work.

1. The ordinary WoT numeric computation domain is finite IEEE-754 binary64
   (`f64`) unless a future specialized profile explicitly introduces a wider
   numeric type.
2. Admission defines an explicit maximum Number lexical size for any operation
   that must inspect textual Number content. The exact value and whether it is
   profile-configurable are review questions; this proposal deliberately does
   not guess a constant.
3. A Number whose textual representation exceeds that bound is not declared
   invalid JSON or mathematically invalid. Bounded admission terminates with a
   resource/domain limit before invoking an input-sized numeric conversion.
4. Once the Number is within the admitted lexical bound, conversion and the
   corresponding bound comparison may execute atomically. Work may be charged
   for the admitted Number before the call; cancellation is checked before and
   after the atomic operation. The lexical ceiling supplies the missing worst-
   case cancellation/work interval that github-pr:81 correctly found absent.
5. A numeric value that cannot be represented as a finite binary64 value is
   outside the ordinary WoT numeric domain. Review must decide whether this is
   expressed by Basic `InvalidSchema`, by an admission-domain terminal, or by an
   existing resource-limit category; it must not be silently treated as an
   absent bound merely because `as_f64()` returns `None`.
6. Normal binary64 rounding is part of the selected numeric domain. Values such
   as `9007199254740993` need not acquire arbitrary-precision semantics merely
   because their mathematical integer value is distinguishable beyond binary64
   precision. If a use case requires such precision, it belongs to a
   specialized numeric system/profile rather than the generic WoT runtime.

This proposal distinguishes two independent questions that topics 0067/0068
currently couple:

- **semantic domain:** what numeric values does the generic WoT runtime promise
  to compute exactly enough for its declared type; and
- **execution grain:** what bounded operation may run atomically between two
  cancellation points.

A bounded atomic operation is compatible with `CONSTRAINED-PROGRESS-001` if its
worst-case input size is explicitly bounded. Resumability is required when an
operation's admitted worst-case cost exceeds the selected atomic-work bound,
not merely because its implementation loops over bytes internally.

## Consequences if accepted

Acceptance would require a new authority migration rather than editing this
proposal in place. That migration should reassess, not automatically preserve,
the following parts of the current 0067/0068 direction:

- the exact finite-decimal amendment for the five Basic extension predicates;
- byte-resumable multi-pass decimal comparison;
- per-byte cancellation/progress obligations for Number comparison;
- synchronous Display-driven exact-decimal comparison in capability-off Basic;
- `serde_json/arbitrary_precision` as a semantic requirement rather than, at
  most, a stable lexical-access mechanism for bounded admission; and
- the #81/#82 pathological long-Number witnesses as required supported values
  rather than negative resource-boundary tests.

The proposal does **not** reopen the normalized retained-snapshot decision from
0065. Project-owned retained arenas, allocation accounting, rollback,
fieldwise semantic equivalence, Planning's storage-independent view, and the
rest of WP-100 remain independent of this numeric execution-grain question.

It also does not imply that every library call is automatically an acceptable
atomic primitive. An atomic operation must have an admitted worst-case input
bound small enough for the supported constrained profiles, and that bound must
be enforceable before the operation begins.

## Review questions

An independent review should try to falsify this direction before any authority
migration:

1. Is a bounded binary64 numeric domain consistent with the actual W3C WoT TD
   numeric contract and the project's existing public typed model?
2. Can strict JSON admission enforce a per-Number lexical ceiling before any
   unbounded numeric conversion occurs?
3. Can typed-`Thing` compatibility admission enforce the same ceiling using
   stable public `serde_json::Number` APIs across the supported feature graphs,
   without reviving private-layout authority?
4. Does a precharged bounded atomic numeric conversion satisfy the actual
   intent of `CONSTRAINED-WORK-001`, `CONSTRAINED-PROGRESS-001`, and
   `ADMIT-TXN-001`, or is byte-level resumability genuinely required by a
   product requirement rather than by the current wording?
5. What terminal should distinguish (a) syntactically legal but over-limit
   Number text from (b) a value outside the supported finite-binary64 domain?
6. Does keeping binary64 rounding preserve a coherent Basic contract, and what
   deliberate delta from current `None`-as-absent behavior is necessary?
7. If `td/validated-thing -> serde_json/arbitrary_precision` is still useful
   solely for stable borrowed lexical access, can that capability be retained
   without making arbitrary precision part of the accepted numeric domain?
8. Which 0067/0068 evidence and fixtures remain useful as negative boundary
   tests, and which obligations disappear if the bounded atomic model is
   accepted?

## Stop condition

Do not implement production Rust, change Basic semantics, alter resource rows,
remove the current feature capability, or readmit `WP-100-CONSUMER-VALIDATED-THING`
from this proposal alone.

If independent review finds that arbitrary-length exact-decimal support is an
actual WoT/product requirement, keep the current authority and continue the
0067/0068 constructibility proof. If not, perform a separate docs-only
migration that explicitly supersedes the affected numeric authority before
resuming WP-100 implementation.
