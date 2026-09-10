# ValidatedThing stable Number query boundary counterexample

This is non-production **blocking evidence**, not an implementation or
readmission of `WP-100-CONSUMER-VALIDATED-THING`. It challenges the number and
Basic-validation part of pre-readmission item 6, with implications for typed
parity (item 3) in a required `arbitrary_precision` graph (item 5).
The tranche stays `planned` / `candidate` / `current`.

Authority examined: the admission record at master commit
`8f958b5413e2c26406ab4aac0da4fda9ff6b6495`, including the shared RFC3339
amendment from github-pr:80. That amendment addresses date decoding. This
finding concerns an independent numeric conversion path.

## Reproduce

From the repository root, with `rust-src` installed for the selected compiler:

```sh
rustc -Vv
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-number-boundary/Cargo.toml -- --nocapture
cargo fmt --manifest-path tools/architecture-fixtures/validated-thing-number-boundary/Cargo.toml -- --check
```

The recorded observation uses rustc 1.95.0 (`59807616e1fa2540724bfbac14d7976d7e4a3860`),
Host `x86_64-unknown-linux-gnu`, serde_json 1.0.149, and the standalone
lockfile. Only this observation fixture requires `rust-src`. Production keeps
its ordinary stable Rust and dependency policy. A changed observation seam
fails the fixture explicitly and requires adapting the observer, not pinning
the product to private Rust source.

The fixture's direct dependency legally unifies serde_json
`arbitrary_precision` into the existing TD dependency. It uses ordinary
`serde_json::from_str::<Number>`; it never constructs an unchecked Number,
accesses its private storage, or relies on its allocation layout.

## Executable finding

For every tested prefix length, construct these two Numbers:

```text
1e+000...0000  -> as_f64() = Some(1.0)
1e+000...0001  -> as_f64() = Some(10.0)
```

All exponent digits survive ordinary Number parsing in this graph. Both
values are exactly representable finite floats; this is not an overflow case
that could simply bypass the typed query. The final byte determines the
projection despite the arbitrarily long common prefix.

The test inserts each Number into a real typed Thing's StringSchema extension
`minimum`, with extension `maximum = 2`. The actual public
`Thing::validate_with_level(Basic)` accepts the first and rejects the second
as `InvalidSchema`. This exercises the existing rule, including its deliberate
inspection of numeric extensions on non-numeric schema variants.

The production call chain is:

1. [`DataSchema::validate_with_level`](../../../td/src/components/data_schema.rs)
   calls `validate_schema_context` and `validate_json_number_bounds`.
2. `value_as_f64` calls the public `serde_json::Value::as_f64`.
3. `Number::as_f64`, under `arbitrary_precision`, executes
   `self.n.parse::<f64>().ok().filter(|float| float.is_finite())` synchronously.
4. The installed Rust `dec2flt` implementation calls `parse_number`; its
   `parse_scientific` consumes every exponent digit before returning.

The same stable query is relevant to the admission record's frozen numeric
capture direction. A typed compatibility input has already decoded
timestamps, but it has **not** necessarily precomputed its Numbers' binary
numeric projections. A pure lossless Display-to-byte-arena copy does not
supply the numeric result that existing Basic rules use.

## Observation scope and exact traces

`build.rs` reads the selected compiler's `rust-src` and compiles the finite
lexical front end of `dec2flt/parse.rs`, its original `Decimal` fields, and
`common.rs`. It changes module import paths and inserts observers at byte
accesses only. It excludes the non-finite spelling helper and subsequent
decimal-to-binary arithmetic. The observer has fixed thread-local counters;
it does not allocate an input-sized trace.

This does **not** intercept the compiled production `Number::as_f64` call.
The real Number query and real TD validation execute separately as semantic
oracles. The observed front end establishes a source-executed lower bound
on the synchronous query's work, not a total count for all float conversion
stages. In particular, these read counts are not a new WorkClass formula.

| Leading exponent zeros | Number bytes | Observed lexical byte reads | Furthest byte reached |
| ---: | ---: | ---: | ---: |
| 1 | 5 | 8 | 5 |
| 9 | 13 | 24 | 13 |
| 4096 | 4100 | 4111 | 4100 |
| 65536 | 65540 | 65551 | 65540 |

Both members of each pair have exactly the same observed read count. Tests
assert these counts, the lexical mantissa/exponent, the real `as_f64` results,
and the differing Basic acceptance results. No timing threshold is used.

The negative adapter fixtures show:

- A single `JsonSchemaNodes` charge allows the stable query to run with zero
  `CodecInputBytes` allowance. Its separately observed lexical lower bound is
  4111 byte reads over the 4100-byte value.
- Even granting an adapter a lower-bound cost oracle of 4100 byte units,
  five fresh steps at each allowance of 0, 1, 9, 128, or 4099 execute zero
  queries. There is no continuation inside the stable query. This is a
  rejected adapter example, not a claim that every insufficient-budget
  `Pending` violates progress.
- Text retention alone does not preserve the Basic numeric projection:
  `9007199254740993.0` rounds to `9007199254740992.0`, `1e-4000` projects to
  zero, and `1e+4000` projects to `None` because of finite filtering.
  These are asserted against the real stable query, not substituted rules.

Input construction, public serde parsing, and ordinary synchronous Basic
validation here are oracle/setup work. None is presented as a bounded
normalization implementation or included in a fictitious arena footprint.

## Authority conflict and stop boundary

The [admission record](../../../docs/work-packages/WP-100-consumer-validated-thing-admission.md)
freezes all of the following together:

- "Integers and representable floating numbers are captured from stable typed
  Number queries; otherwise Number's stable display implementation writes
  directly into the charged byte arena."
- The existing Basic rule set remains singular, with unchanged acceptance.
- Basic validation, typed numeric reads, and normalization execute under
  the cursor's work and lifetime budgets, with no uncharged helper path.

[`CONSTRAINED-WORK-001`](../../../docs/spec/foundation.md) also says a step
must not hide an unbounded decode; non-incremental calls must declare their
maximum admitted input and external worst-case execution responsibility.
[`ADMIT-TXN-001`](../../../docs/spec/runtime-safety.md) requires cancellation
checks at bounded work intervals. This cursor has no declared exception for
an arbitrarily long synchronous Number-to-float query.

The frozen stable-query route therefore cannot be treated as a bounded
primitive in the required feature graph. Moving this call before a charged
step, charging only its containing schema node, accumulating credits before
executing a bulk scan, or repeatedly calling the whole parser would not make
it a resumable primitive. A whole-query atomic allowance would instead need
an explicit non-incremental work/cancellation contract, which this tranche
does not have. The date-specific resumable extraction in github-pr:80 does
not change this numeric call chain.

Potential alternatives need an authority decision before implementation:

- Define a shared resumable numeric projection with the exact existing
  rounding, underflow, overflow/finite filtering, and feature-dependent
  semantics, then amend the frozen stable-Number-query direction and prove
  its allocation/work boundary. A TD-local implementation may be possible
  within existing TD files; it would still replace the frozen query mechanism
  and needs its own parity evidence. This fixture does not claim that such an
  algorithm is intrinsically impossible or necessarily needs another arena.
- Explicitly admit a bounded synchronous query policy and its external
  work/cancellation responsibility. No such relaxation is applied here.

Rejecting long legal Numbers as syntax/Basic-invalid, dropping the required
`arbitrary_precision` cell, or changing numeric Basic comparisons would change
accepted semantics. Retaining an owned Number would violate the retained
catalog. Copying or modifying serde_json/libcore internals in production would
exceed the permitted production paths and dependency policy. None is used.

Work stops at this conflict. No numeric replacement, RFC3339 replacement,
authority amendment, production Rust, new account/WorkClass/resource row,
public API, or admission transition is included.

## Pre-readmission coverage

| Required item | Evidence established here |
| --- | --- |
| 1 — Frozen API and forbidden surfaces | Not established |
| 2 — Allocation catalog and Layout ordering | Not established; no arena prototype claim |
| 3 — Full typed-equivalence corpus | Not established; the narrow actual Basic numeric oracle above is supplied |
| 4 — External Planning view | Not established |
| 5 — Supported feature/target matrix | Only Host with `arbitrary_precision` executes; the complete matrix is not established |
| 6 — Complete progress/rollback traces | Blocked at stable numeric projection; negative adapter and lexical traces above do not complete this item |
| 7 — Complete resource proof | Not established |
| 8 — Prior evidence reaffirmation | No production, authority, resource, or gate artifact changes; no broad reaffirmation claim |

In particular, this fixture does not reopen the Foundation behavior from
github-pr:69, the Context seam from github-pr:70, completed WP-200/WP-300
evidence, or the passed Producer Property Read gate. It does not falsify any
of those claims. Their full registered commands are not asserted re-executed
by this fixture. There is no future completion evidence manifest and no claim
that all eight pre-readmission obligations have been satisfied.
