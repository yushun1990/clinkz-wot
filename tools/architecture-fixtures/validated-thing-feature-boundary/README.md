# Number feature-boundary prototype

Non-production impact evidence against master `8dd8973` (#84). The selected
future boundary belongs to the WP-100 validated-Thing admission record. This
standalone workspace models it without changing production TD or exporting a
ValidatedThing production cursor.

## Reproduce

```sh
rustup target add thumbv7em-none-eabihf
python3 tools/architecture-fixtures/validated-thing-feature-boundary/check.py
# All locked dependencies cached:
python3 tools/architecture-fixtures/validated-thing-feature-boundary/check.py --offline
```

The fixture lock selects serde_json 1.0.149, matching master. The prototype
declares `1.0.149` as a semver API floor (caret range), not an exact pin.
Upgrading within the range does not authorize reliance on private source.
The matrix's normal-dependency feature assertions accompany every result.

| Family | Cells | Execution |
| --- | ---: | --- |
| Host base/order/AP/combined × capability off/on | 8 | Source, typed parsing, actual current TD Basic witness, positivity; charged scan when on |
| Host no-default/async × capability off/on | 4 | Same runtime tests, no TD/serde std feature |
| thumb base/AP × async off/on × capability off/on | 8 | Actual no-std target library compilation |
| Host base/AP/order/combined × absent/sibling capability | 8 | Gated import E0432 or successful import |
| thumb base/AP × absent/sibling capability | 4 | Same negative/positive imports on actual target |

Each cell uses a **separate Cargo invocation**. One workspace-wide invocation
would unify features and invalidate negative evidence. Order/combined are
Host-only because upstream preserve_order enables std. Thumb checks are not a
link, runtime, allocator, atomic-free runtime-parity or full-signature proof.

## Construction

`td-boundary` gates its bounded module on its local `validated-thing`
capability and explicitly forwards AP. Within it, public `Number::as_str()`
is unconditional. `downstream` can independently enable serde AP/order while
the TD capability stays off. `surface` imports through a direct dependency
but can enable the capability through a sibling, exposing the distinction
between dependency-feature unification and a local capability cfg.

The public foundations are [Cargo dependency features and unification](https://doc.rust-lang.org/cargo/reference/features.html#dependency-features),
AP-gated `Number::as_str` in [serde_json's published API source](https://docs.rs/serde_json/1.0.149/src/serde_json/number.rs.html),
and public Number Display / core::fmt::Write for synchronous decimal delivery.
The fixture reads no dependency source, private tokens, Serialize callback
shapes, capacity or layout. A formatting sink accepts arbitrary callback
partitioning; it cannot call an externally sized callback one bounded unit.

`synchronous_decimal` streams Display without project-owned output. Fixed-state
`Positivity` consumes those bytes and the charged borrowed-text scan feeds the
same consumer. Tests collect byte vectors only as assertion oracles outside
the adapters. They compare every AP byte with the public borrow, cover
65,540-byte text, huge negative exponents, signed zero, both #82 cancellation
spellings, and base parsing rounding. Display has no step-latency guarantee and
is never called by Scan.

Actual existing TD Basic is executed for all four bound pairings on a String
schema. It currently accepts lower 9007199254740993 with upper 9007199254740992,
although an independent exact integer oracle rejects. Base synchronous Basic
must therefore migrate too. Underflow already lost by base Number parsing is
typed zero, not an opportunity to recover a discarded input lexeme.

## Limits

Scan only witnesses public byte access with a borrowed slice, fixed state,
existing WorkBudget and non-resettable lifetime. It charges before observation
and tests zero/small steps, exhaustion and sticky first cause. It does not
model normalized storage, comparisons, strict decoding, ledger or rollback.
Positivity is one predicate; the full arbitrary-exponent comparison kernel and
its synchronous/charged adapters remain required before readmission.

This supplies a feature decision, not completion of any of the eight
pre-readmission items, gate acceptance or production permission. #84 remains
historical evidence against the replaced boundary. The current work-package
states and public signature/resource constraints remain intact.
