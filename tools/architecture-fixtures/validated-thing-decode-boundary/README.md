# ValidatedThing shared decode boundary counterexample

This is non-production **blocking evidence**, not a ValidatedThing prototype
completion claim or a readmission request. It investigates pre-readmission
item 6 (typed/direct decode progress) in
[`WP-100-CONSUMER-VALIDATED-THING`](../../../docs/work-packages/WP-100-consumer-validated-thing-admission.md).
The tranche remains `planned` / `candidate` / `current`.

## Reproduce

From the repository root:

```sh
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-decode-boundary/Cargo.toml -- --nocapture
cargo fmt --manifest-path tools/architecture-fixtures/validated-thing-decode-boundary/Cargo.toml -- --check
```

The standalone lockfile retains the relevant versions from the master lockfile
(including serde_json 1.0.149, serde 1.0.228, time 0.3.47 and serde_with 3.18.0).
This is a Host observation fixture; it claims no constrained-target execution.

## Finding

The frozen strict entry must share TD typed-field decoding authority, perform
byte-charged resumable work, and preserve existing decoding semantics. The
future production source list omits `td/src/rfc3339.rs`, which owns the
`created` / `modified` decoder called by `Rfc3339DateTimeField` in
`td/src/thing.rs`. Its private `parse_rfc3339`, `Cursor`, and
`Cursor::optional_fraction` form one synchronous call with no continuation
interface. Fractional seconds have no lexical length bound: digits after nine
are still checked, although their numeric contribution is truncated.

The probe invokes that actual parser through serde's borrowed
`StrDeserializer`. `build.rs` reads the current production file and inserts
only an observer at its sole byte-access expression. It does not replace a
rule, return value, loop, or error. The observer uses fixed thread-local scalar
counters, never an input-sized log. Its output is checked against the real
production `Thing` decoder and Basic validation. The production parser's five
existing unit tests also execute on the instrumented source.

Measured valid-input traces (one synchronous parser call each):

| Fraction digits | Input bytes | Byte reads | Furthest byte reached |
| ---: | ---: | ---: | ---: |
| 1 | 22 | 26 | 22 |
| 9 | 30 | 42 | 30 |
| 10 | 31 | 44 | 31 |
| 4096 | 4117 | 8216 | 4117 |
| 65536 | 65557 | 131096 | 65557 |

The read count includes repeat peeks, not just consumed bytes; it is an
implementation-work observation, **not a proposed new WorkClass formula**.
The scan grows with the external input in either measure. These are legal
typed-decoding/Basic inputs, not an assertion that they fit every resource
profile. Limits may reject them as resource limits; their length cannot be
made a new syntax or Basic restriction.

The late-invalid input puts `x` after 4096 fraction digits, before `Z`. The
parser reads 8215 bytes and rejects it. Parsing only a shortened valid prefix
would lose that rejection.

Two deliberately rejected adapter approaches make the progress issue concrete:

- Charging one `DocumentNodes` unit before the existing call permits 8216
  observed byte reads with a zero byte budget. This is not conforming decode
  evidence.
- Even granting an adapter foreknowledge of the exact read cost, atomically
  charging the entire call leaves all work pending with fresh step allowances
  of 0, 1, 9, 128, or 8215. Only a sufficient single-call allowance starts the
  synchronous parser. This demonstrates the absence of a resumable seam; it
  does not claim that insufficient-budget `Pending` is itself illegal.

Precharging across calls and doing the scan in the last call would move work
out of its executing step, not make the scan resumable. Implementing a separate
fraction/date parser or truncation rule in the strict normalizer would copy
typed-decoding semantics instead of sharing their owner. The final lexical
check cannot be skipped just because the retained timestamp has fixed size.

## Source-boundary conclusion and scope of the stop

The ordinary extraction needed for a shared resumable date decoder changes
`td/src/rfc3339.rs`: expose resumable scalar parse state and drive that same
state from both the existing serde adapter and the strict decoder. That file
is outside the frozen future production list. The work package explicitly
says: “A required production change outside this list returns the tranche to
impact review.” This evidence therefore stops at that source-boundary review.

This is **not** a proof that normalized storage or a resumable date parser is
intrinsically impossible. It falsifies treating the current source boundary
and existing shared decoder as sufficient without further extraction review.
Rerouting `mod rfc3339` through an allowed file while leaving its old parser
orphaned would relocate the semantic owner to work around the omission; this
probe does not treat that as implicit authority. The narrow proposed remedy
for independent impact review is to permit extraction in the actual parser
owner, preserving its lexical/typed outcomes and using one shared rule body.
No remedy or authority amendment is implemented here.

Pre-readmission coverage is intentionally incomplete:

| Item | Evidence from this fixture |
| --- | --- |
| 1, frozen API | Not established |
| 2, allocation catalog | Not established |
| 3, full typed-equivalence corpus | Not established; only the date decode oracle above |
| 4, external Planning view | Not established |
| 5, supported feature/target matrix | Not established |
| 6, full progress/rollback traces | Blocked at the shared date decoder; other required traces not established |
| 7, full resource proof | Not established |
| 8, prior evidence reaffirmation | No production, authority, resource, or gate artifact is changed; prior runtime commands are not re-executed or claimed reaffirmed |

In particular, this does not establish or falsify the three-retained /
four-temporary catalog, sorting, URI/number algorithms, or prepaid bounded
drop. Those claims remain unproven here. No completion evidence manifest, gate
status, work-package admission, or roadmap state is changed.
