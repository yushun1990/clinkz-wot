# Number feature-boundary prototype

Non-production impact evidence originating from master `8dd8973` (#84), now
aligned with the bounded-atomic Number authority merged through #87. The
selected future boundary belongs to the WP-100 validated-Thing admission
record. This standalone workspace models it without changing production TD
or exporting a ValidatedThing production cursor.

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
| Host base/order/AP/combined × capability off/on | 8 | Public finite-binary64 predicate projection, actual current TD Basic witness; borrowed source and precharged projection when on |
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
AP-gated `Number::as_str`, and graph-local `Number::as_f64` in
[serde_json's published API source](https://docs.rs/serde_json/1.0.149/src/serde_json/number.rs.html).
The fixture reads no dependency source, private tokens, Serialize callback
shapes, capacity or layout. The synchronous Basic projection uses only
`Value::as_number`, `Number::as_f64`, and `f64::is_finite` in every graph.
It classifies non-Numbers as absent and a failed/non-finite Number projection
as invalid. Binary64 rounding is intentional.

Actual existing public TD Basic runs for all four bound pairings on a String
schema. It accepts lower 9007199254740993 with upper 9007199254740992 in
base and AP graphs; the selected binary64 rule also accepts them after
rounding. An independent integer comparison remains a storage-distinction
witness, not the selected Basic oracle. Current AP TD Basic also silently
accepts a short `1e309` at each of the five extension predicates because
projection failure becomes absence. The selected shared Basic rule must
instead return `InvalidSchema` in both public Basic and future bounded
admission. Other
tested Basic cases retain their outcomes, including non-Number absence.
Base graph ordinary public Basic does not require borrowed lexical access,
and it remains synchronous.

With the capability on, `Scan` still demonstrates borrowed source delivery
under existing byte work and lifetime accounting. `project` witnesses the
separate bounded-atomic step using this fixture's 256-byte profile value:
check the configured lexical limit, precharge `CodecInputBytes` against step
and lifetime, then run the public projection
between two cancellation checks. Typed-borrow checks cover 64/65 and 256/257
bytes. This is a narrow source/feature witness, not strict tokenization or
an admitted cursor.

## Limits

Scan witnesses public byte access with a borrowed slice, fixed state, existing
WorkBudget and a non-resettable lifetime. The atomic witness does not prove
any target cycle/stack tolerance, strict JSON over-limit early
stop, normalized storage, complete Basic comparison traversal, ledger, or
rollback. The full pre-readmission evidence set remains required.

This reconciles the selected feature decision, not completion of any of the eight
pre-readmission items, gate acceptance or production permission. #84 remains
historical evidence against the replaced boundary. The current work-package
states and public signature/resource constraints remain intact.
