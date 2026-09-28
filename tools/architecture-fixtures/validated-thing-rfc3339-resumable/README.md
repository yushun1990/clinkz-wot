# Shared resumable RFC3339 prototype

This is non-production, partial readmission evidence for
[`WP-100-CONSUMER-VALIDATED-THING`](../../../docs/work-packages/WP-100-consumer-validated-thing-admission.md#shared-resumable-rfc3339-decode).
It follows the blocking counterexample in
[`validated-thing-decode-boundary`](../validated-thing-decode-boundary/README.md)
and the migrated decision in
[`workspace/0066`](../../../workspace/0066-shared-rfc3339-decode-impact.md).
It does not change the production parser, expose a decoder API, admit the
tranche, complete any one of the eight readmission items, or create completion
evidence.

## Reproduce

From the repository root:

```sh
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-rfc3339-resumable/Cargo.toml
cargo check --locked --target thumbv7em-none-eabihf --manifest-path tools/architecture-fixtures/validated-thing-rfc3339-resumable/Cargo.toml
cargo fmt --manifest-path tools/architecture-fixtures/validated-thing-rfc3339-resumable/Cargo.toml -- --check
```

The standalone lockfile retains the current counterexample baseline's relevant
versions: serde 1.0.228, serde_json 1.0.149, and time 0.3.47. The target command
compiles the prototype as `no_std`; it is not target execution or an allocator,
cycle, or stack measurement.

## Candidate shape and semantic ownership

`build.rs` reads the current production `td/src/rfc3339.rs` and checks the
expected parser and fraction-scanner seams. It emits two test modules:

- `existing.rs` is the real source with only inner documentation comments
  converted so `include!` can compile it. It is the independent oracle.
- `prototype.rs` retains that source's serializer, serde visitor, formatter,
  parse-error text, and unit tests. It replaces only the private synchronous
  `parse_rfc3339`/`Cursor` region with `src/resumable_parser.rs`.

The replacement contains one date rule body, `Decoder`. The retained ordinary
serde adapter synchronously loops over the input and then finishes that decoder.
The strict JSON-string cursor feeds the same decoder as charged JSON units are
unescaped. It does not contain a second date parser, shorten the fraction,
canonicalize a prefix, or call the old parser after accumulating credit. The
production parser is compiled beside it solely for differential tests.

The differential corpus compares exact `OffsetDateTime` values or exact serde
error text. It covers the production tests, accepted separator/offset forms,
fraction widths 1/8/9/10/4,096/65,536, component range failures, error-precedence
cases, 124,416 structured component/fraction/offset combinations, and
deletion/replacement of every byte in a representative timestamp.
The 65,536-digit case completes through both adapters with the same result; no
date-length ceiling is added. This is strong constructibility evidence, not an
exhaustive proof over every possible byte string or dependency version.

## Bounded progress

`StrictDateCursor::step` owns the borrowed token position, JSON escape state,
the shared date continuation, first terminal result, and a non-resettable
lifetime remainder. Before reading each source byte it requires and consumes:

1. one current-step `CodecInputBytes` unit; and
2. one unit from the cursor's lifetime remainder.

An empty input has no charged source byte to own its EOF rejection. That
standalone EOF transition therefore consumes one `CodecInputBytes` unit from
the current step and one unit from the lifetime remainder before it returns a
terminal JSON error. With either unit unavailable it returns `Pending` or
`Limit` under the normal precedence. A non-empty token's EOF check remains
fixed work owned by its last charged source byte.

The cursor stores no step credit. A fresh budget therefore cannot pay earlier
and trigger a later bulk scan. For `w` charged progress units in one step, the
instrumented implementation consumes at most `w` source bytes, performs
exactly one JSON transition per source byte, at most `4w` date-byte transitions
(the maximum UTF-8 expansion of one completed JSON escape/scalar), and at most
`w` fixed finalizations. Tests assert the conservative bound
`json + date + finalization <= 6w` after every step. Ordinarily `w` equals the
number of source bytes; the standalone empty-input EOF transition has `w = 1`
and no source byte. All inner loops have a fixed maximum of four;
calendar/time/offset construction is fixed-size work performed while
processing the charged closing quote.

Executable traces cover:

- a valid 65,536-digit fraction with fresh one-byte budgets across every step;
- 1 through 8 byte allowances over long plain and escaped inputs;
- zero budget with unchanged position, state, lifetime, and work trace;
- pauses inside `\uXXXX`, surrogate-pair, UTF-8, fraction, delimiter, and offset
  state;
- every continuation boundary of raw valid two-, three-, and four-byte UTF-8,
  plus malformed continuation and overlong sequences, under one-byte
  allowances;
- a late `x` after 4,096 fraction digits and trailing data after a complete
  offset, with production error parity and no prefix acceptance;
- valid and invalid JSON escape combinations compared with serde_json plus the
  real date decoder;
- lifetime exhaustion across fresh step budgets, including exhaustion before
  the charged closing-quote finalization; and
- cancellation before the next step, preserving position and trace and
  consuming no current-step allowance.

This proves the prototype's work bound in units of its instrumented scalar
transitions. It does not claim a target cycle limit. Cancellation is exercised
at the frozen `step(..., cancel_requested: bool)` call boundary; integration
with complete builder rollback remains outside this fixture.

## State, allocation, and cleanup

Both continuations contain only enums, counters, component scalars, one
four-byte UTF-8 buffer, a borrowed input slice, and fixed result/error state.
Host tests assert that `Decoder` is at most 128 bytes,
`StrictDateCursor<'static>` is at most 256 bytes, neither needs drop, and size
does not change while progress advances. A thread-local counting allocator
observes zero allocations for strict success, synchronous success, and
unfinished-cursor drop. The real thumb target compiles the same library.

These results support the existing authority classification:

- input work uses `CodecInputBytes` and the existing shared lifetime remainder;
- date and JSON continuation state is fixed-size and allocation-free under the
  fixture's measured Host bounds; no authoritative builder or Servient
  inline-owner capacity is asserted;
- the date seam needs no retained allocation, temporary allocation, diagnostic
  allocation, cleanup record, or new allocation category; and
- dropping or cancelling this date-local cursor has no recursive or
  input-sized cleanup.

The complete strict builder may still materialize decoded bytes in its already
authorized byte-build arena. This fixture does not implement that arena, its
`CodecOutputBytes` charging, the other three temporary arenas, grow/seal
overlap, ledger reservations, or rollback. It therefore cannot prove the full
three-retained/four-temporary catalog or its peak formulas.

## Evidence classification

### Proved by this prototype

- One private semantic state machine can preserve ordinary synchronous serde
  behavior while also supporting strict budgeted pause/resume.
- The tested long fractions, offsets, suffixes, ranges, and JSON escape
  combinations preserve the current decoder's values and error text.
- Every tested strict step performs work proportional to that step's consumed
  allowance; zero allowance performs none, and no credit is accumulated.
- Date-local state is fixed-size, allocation-free, trivially dropped, and
  compilable in the real `thumbv7em-none-eabihf` graph.

### Partially proved

- Semantic preservation is broad for the private date owner but is not an
  exhaustive language-equivalence proof or a complete typed-TD corpus.
- JSON/date composition is executable for a complete string token, including
  escapes and malformed-escape cases. Full-document parsing, diagnostics, and
  precedence between an early field error and malformed later document input
  are not modeled.
- Date-local cancellation and lifetime behavior are proved; complete builder
  rollback, prepaid cleanup, and first-cause behavior across other phases are
  not.
- Existing work and allocation categories fit this local seam; complete
  pipeline resource sufficiency is not proved.

### Not proved

- Production API shape, opaque configuration projection, normalization,
  sorting, URI/number work, seal/equivalence, all rollback causes, or Planning
  view behavior.
- The full capability matrix, downstream feature unification, constrained
  runtime execution, allocator traces on target, physical stack, cycles, or
  interrupt latency.
- A supported implementation maximum `M`, full source/temporary/peak/contiguous
  resource formulas, or Servient inline-owner capacity.
- Reaffirmation of Foundation/Context behavior, WP-200/WP-300 evidence, or the
  passed Producer Property Read gate at this head.

### Counterexample disposition

No newly accepted design claim was falsified. The earlier counterexample
remains valid against the unchanged production parser: node-only charging,
whole-parser atomic precharge, accumulated credit followed by one synchronous
scan, prefix truncation, and a new date-length cap still fail the contract.
This fixture supplies a constructible replacement mechanism without weakening
that finding.

## Relation to the eight pre-readmission items

| Item | Evidence supplied here | Status after this fixture |
| ---: | --- | --- |
| 1 | No production/public surface is added; no compile API fixture is supplied. | Not proved |
| 2 | Date/JSON state is fixed-size, allocation-free, non-dropping, and needs no separate temporary or retained allocation category; no concrete builder/Servient owner capacity is tested. | Partial; complete arenas/formulas/ledger order and owner capacity remain open |
| 3 | Differential date values and exact error text, including long fractions and range/error precedence. | Partial; full typed semantic corpus and numeric predicates remain open |
| 4 | None. | Not proved |
| 5 | Host execution plus real thumb `no_std` compilation of this probe. | Partial; the full capability/feature matrix and target execution remain open |
| 6 | Exact date-token step traces for budget, resume, JSON escapes, lifetime, cancellation, and late errors. | Partial; all other phases, rollback, Number, and cleanup traces remain open |
| 7 | Fixed state, zero local allocations, bounded scalar transition formula, and existing category mapping for this seam. | Partial; complete resource proof and supported `M` remain open |
| 8 | None; no prior gate/runtime command is claimed reaffirmed. | Not proved |
