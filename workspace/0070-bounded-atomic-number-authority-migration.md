# 0070 Bounded Atomic Number Authority Migration

Historical migration record: the project-wide 256-byte ceiling and mandatory
M4 readmission gate described here were superseded by the authority migration
recorded in [topic 0071](0071-constrained-resource-authority-and-target-characterization.md).
Current requirements live in the cited `docs/` owners.

Status: AUTHORITY MIGRATION CANDIDATE / REVIEW REQUIRED

Kind: ADR-0013 scoped implementation-impact migration for `WP-100-CONSUMER-VALIDATED-THING`

Baseline: `master` at `d4cf245eb1bf79e607c4851bba4b16a58e83a04e` after github-pr:86.

Accepted impact proposal: [workspace topic 0069](0069-bounded-atomic-number-domain.md),
reviewed through exact head `07c59f993e85125e7a32747c013a006f5ce6e8f6` before github-pr:86 merged.

This migration does not readmit the tranche, authorize production `ValidatedThing`
Rust, complete any pre-readmission evidence item, or change a Consumer gate. It
replaces only the numeric authority that became unnecessarily strong in topics
0067/0068 while preserving the public lexical-access finding from topic 0068.

## Decision

WP-100 no longer requires byte-resumable arbitrary-length exact-decimal
comparison for the five Basic extension predicates. Instead it uses a bounded
atomic numeric boundary with separate storage and computation domains.

The selected boundary is:

1. `number_lexeme_bytes_max` is one append-only named resource limit for every
   Number retained or inspected by bounded `ValidatedThing` admission.
2. The project-wide hard ceiling is **256 bytes per Number lexeme**. A profile
   may configure a smaller value but MUST NOT configure a larger one. The
   initial named-profile projection is 256 bytes for gateway, `NA` for
   directory-client, and 64 bytes for the benchmark static reference profile.
   Its owner is Consumer `+validated-thing`; no Directory validation path owns it.
3. With configured limit `L` in `0..=256`, a strict JSON Number that reaches
   byte `L + 1` before the token ends terminates bounded admission as `Limit`
   before copying that byte; the decoder does not finish scanning the token.
   This includes 64/65, 256/257, and first-byte rejection when zero disables
   Number admission. A typed compatibility input checks borrowed
   `Number::as_str()` length under `td/validated-thing -> serde_json/arbitrary_precision` before any
   numeric projection or lossless copy.
4. Within the configured lexical ceiling, opaque Numbers such as `const`,
   `default`, and nested extension values remain losslessly retained even when
   they cannot project to finite binary64. The ceiling is a resource boundary,
   not a claim that every retained Number is an `f64`.
5. Existing typed `NumberSchema` comparisons remain the current `f64` behavior,
   including their current caller-constructed acceptance surface. Existing
   typed `IntegerSchema` comparisons remain `i64`. This migration does not
   strengthen either typed schema rule.
6. Only the five Basic extension predicates over `serde_json::Value::Number` —
   `minimum`, `exclusiveMinimum`, `maximum`, `exclusiveMaximum`, and
   `multipleOf` — use the new finite-binary64 projection rule. A non-Number
   value remains absent for that predicate as today. A Number within the
   lexical ceiling whose `as_f64()` projection is not finite/successful is
   `InvalidSchema`; it is never silently treated as an absent bound.
7. Binary64 rounding is the selected computation behavior for those five
   predicates. The project does not reproduce private rustc/serde float-parser
   quirks beyond the stable public projection contract, and does not promise
   arbitrary-precision arithmetic merely because AP preserves lexical text.

The 256-byte candidate permits useful exact opaque numeric constants as well
as ordinary binary64 spellings. The original assertion that this is "small
enough" did not justify its cancellation latency: Rust's public float parser
can enter a slower fallback with a 768-digit buffer even on shorter input.
Lexeme length bounds input, not cycles or stack. The correction supplies a
[declared constrained workload and non-production probe](../tools/architecture-fixtures/bounded-atomic-number/README.md)
with adversarial public projections, explicit cycle/stack rejection bounds,
and measurement instructions. The amendment owns the acceptance requirement.
256 remains a candidate ceiling until target results pass independent review;
neither a host result nor a thumb compile establishes target tolerability.
A failed bound reopens this choice; a later specialized numeric profile also
requires impact review rather than silently raising the ceiling.

## Work, progress, and cancellation

JSON lexing, lossless Number-byte capture, and byte copying remain charged and
resumable exactly as other input-sized codec work. Only the post-boundary
numeric projection/comparison ceases to be byte-resumable.

For one atomic numeric projection:

- the source lexeme length `n` is known first and satisfies `0 < n <= L <= 256`;
- before starting, the cursor debits `n` `CodecInputBytes` units from the
  current `WorkBudget` and the same `n` units from the non-resettable admission
  lifetime remainder;
- if the current step budget is insufficient, the operation does not start and
  returns `Pending` with no numeric progress;
- if the lifetime remainder is insufficient, the operation does not start and
  returns `Limit`; a later step cannot replenish that allowance;
- cancellation is checked immediately before and after the atomic projection;
- after projection, the floating comparison itself is constant-size scalar
  work under the containing schema-node charge.

Thus one cancellation interval may contain at most one <=256-byte numeric
projection. `CONSTRAINED-PROGRESS-001` requires the declared latency and stack
acceptance workload to pass as well as this lexical bound. Zero budget still
makes no progress, and no work begins before its complete debit.

Repeated projection of the same Number, if an implementation cannot retain a
scalar projection in fixed inline state, must debit the same bounded cost each
time. No uncharged rescan is authorized.

Ordinary public `Thing::validate_with_level(Basic)` remains synchronous. It
shares the five-predicate binary64 acceptance rule but is not itself bounded
admission and therefore does not expose `Pending`/`Limit` or apply the
per-admission lexical resource terminal. Capability-off public Basic may use
its ordinary public Number projection synchronously; capability-on bounded
admission uses borrowed Number text to enforce the resource ceiling before the
atomic projection. These adapters must agree on Basic semantics for values that
reach the predicate; resource rejection is separate from Basic invalidity.

## Feature boundary

Topic 0068 remains authoritative for the explicit
`td/validated-thing -> serde_json/arbitrary_precision` capability and supported
feature/target matrix. Its purpose is narrowed: AP guarantees stable borrowed,
lossless Number lexical access for bounded admission and normalized retention.
It does **not** imply exact-decimal Basic arithmetic or arbitrary-precision
numeric computation.

The synchronous Display-driven exact-decimal comparison responsibility from
0068 is superseded. Capability-off public Basic does not need lossless lexical
access merely to perform the five binary64 predicates.

## Superseded authority

This migration supersedes these parts of topic 0067 and their projections:

- exact finite-decimal arithmetic as the Basic meaning of the five extension
  predicates;
- byte-resumable multi-pass decimal comparison;
- fixed-state exact-decimal comparator constructibility as a readmission
  blocker;
- the requirement that every comparison pass charge every inspected Number
  byte one at a time; and
- the #81/#82 long-Number witnesses as values the runtime must successfully
  compute.

Those fixtures remain valuable as negative resource-boundary and semantic
regression evidence. In particular, an over-ceiling witness must stop as
`Limit`, and a short within-ceiling overflow such as `1e309` in one of the five
predicates must be `InvalidSchema` rather than absent.

This migration supersedes these parts of topic 0068:

- synchronous Display-driven exact-decimal comparison in base graphs; and
- any wording that makes AP a semantic prerequisite for exact arithmetic.

It preserves topic 0068's public-access finding, explicit `validated-thing`
capability, AP feature edge, semver floor, and Host/thumb/downstream feature
matrix.

## Resource projection

The authority migration reserves one append-only resource field:

`number_lexeme_bytes_max, document, bytes, per-item, consumer, disabled, 256, NA, 64`

The generated `ResourceKind`/getter is implementation plumbing and does not by
itself readmit WP-100. The configured value MUST be in `0..=256`; zero disables
Number admission in the same sense as other disabled resources. A nonzero
profile value above 256 is invalid configuration rather than permission to
expand the atomic boundary.

The row owns only the per-Number lexical ceiling. Existing document/source,
temporary, retained-byte, peak, and work limits continue to own aggregate
resource use.

## Pre-readmission evidence delta

The eight pre-readmission items remain required as a set, but numeric portions
of items 3 and 6 change.

Item 3 must prove:

- all unaffected Basic rules retain their current oracle;
- the five extension predicates use the selected binary64 projection in public
  Basic and both future admission entries;
- non-Number values remain absent for those predicates;
- within-ceiling projection failure is `InvalidSchema`;
- opaque within-ceiling Numbers are retained losslessly regardless of finite
  `f64` projection;
- typed `NumberSchema` and `IntegerSchema` behavior is unchanged; and
- strict/typed/capability graphs agree wherever the same typed Number is
  constructible.

Item 6 must prove:

- exact configured `L - 1`/`L`/`L + 1` thresholds, including 63/64/65 and
  255/256/257 lexical bytes, and first-byte rejection for zero-disabled;
- strict decode stops over-ceiling input without an unbounded finishing scan;
- typed AP-backed length checking precedes conversion/copy;
- insufficient current step budget returns `Pending` without starting;
- insufficient lifetime allowance returns `Limit` without starting;
- a started atomic projection is fully precharged and has one <=256-byte
  cancellation interval with accepted target cycle/stack workload results;
- zero budget makes no numeric progress;
- repeated projections are charged each time;
- short overflow/underflow/rounding boundary cases have deliberate results;
- #81/#82 long witnesses are resource-limit witnesses rather than successful
  exact-decimal comparisons; and
- Host and real constrained/thumb feature cells exercise the same boundary.

Items 2, 5, 7, and 8 still cover allocation-site constructibility,
feature/target graphs, resource sufficiency, and exact-head reaffirmation. The
remaining items are unchanged except where they reference the old numeric
algorithm.

## Migration scope

The authority PR that adopts this record should update:

- `docs/work-packages/WP-100-consumer-validated-thing-admission.md`;
- `docs/spec/runtime-safety.md`;
- `docs/spec/foundation.md`;
- `docs/work-packages/WP-100-core.md`;
- the resource-schema projection and tranche index; and
- topic 0067/0068 disposition text as needed to prevent stale authority.

It must not implement production TD normalization, readmit the tranche, or
claim any completion evidence item finished.

## Stop condition

If the 256-byte hard ceiling cannot be enforced before every bounded numeric
projection in the supported strict/typed feature graphs, or if a supported
constrained target cannot tolerate one precharged <=256-byte public projection
as an atomic cancellation interval, stop and reopen impact review. Do not fall
back to unbounded `as_f64()`, exact-decimal byte resumability, or a larger
profile-specific ceiling silently.
