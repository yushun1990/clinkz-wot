# ValidatedThing stable Number query boundary counterexample

This is non-production **blocking evidence**, not an implementation or
readmission of `WP-100-CONSUMER-VALIDATED-THING`. It challenges the number and
Basic-validation part of pre-readmission item 6, with implications for typed
parity (item 3) in a required `arbitrary_precision` graph (item 5).
The tranche stays `planned` / `candidate` / `current`.

The [decimal-cancellation follow-up](#decimal-cancellation-follow-up) below
adds a second blocker after github-pr:81: a bounded, exact-value reduction
does not necessarily preserve the actual stable query or Basic acceptance.

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


## Decimal-cancellation follow-up

Baseline: master `a268f5794a3b96c562de98eaaf5996e081fca7a5` (github-pr:81).
The original four tests and their counterexample remain intact. Two tests in
[`src/cancellation.rs`](src/cancellation.rs) investigate the simpler proposed
alternative: scan incrementally, preserve lossless source text separately, and
compute the float projection from the exact decimal value using bounded state.
The witness isolates exponent cancellation before any general rounding
algorithm is needed. It is not a production normalizer or a complete Number
parser.

For positive integer `n`, both of these legal decimal spellings have exact
mathematical value one:

```text
I(n) = "1" + "0" repeated n times + "e-" + decimal(n)
F(n) = "0." + "0" repeated (n-1) times + "1e+" + decimal(n)
```

They denote respectively `10^n * 10^-n` and `10^-n * 10^n`. Public
`serde_json::from_str::<Number>` accepts them and retains the spelling in the
required arbitrary-precision graph. No unchecked construction is involved.
On the recorded rustc 1.95.0 / serde_json 1.0.149 Host graph:

| Input | Bytes | Actual `Number::as_f64()` | Exact-value projection | Actual Basic | Basic using exact-value projection |
| --- | ---: | --- | --- | --- | --- |
| `I(655359)` | 655368 | `Some(1.0)` | `Some(1.0)` | reject | reject |
| `F(655359)` | 655369 | `Some(1.0)` | `Some(1.0)` | reject | reject |
| `I(655360)` | 655369 | `None` | `Some(1.0)` | accept | reject |
| `F(655360)` | 655370 | `Some(0.0)` | `Some(1.0)` | accept | reject |

Each actual Basic result comes from the public TD validator with the tested
Number as a StringSchema extension `minimum` and `maximum = 0.5`. The
comparison column calls that same validator with `Number::from(1)` as minimum;
it is a semantic oracle for the proposed cached float value, not a normalized
snapshot or an equality claim between the two Numbers. Basic's existing
optional numeric bounds skip a minimum whose query returns `None`; a zero
minimum passes. A minimum of one rejects as `InvalidSchema`. The test copies
no Basic rule body. Keeping original lossless bytes alongside an incorrect
cached projection would still produce this comparison mismatch.

The installed lexical observer explains the discrepancy. `parse_scientific`
updates its exponent only while the accumulated value is below `0x10000`.
For exponent digits `655359`, the accumulator reaches `65535` before the last
digit and then reaches `655359`. For `655360`, it reaches `65536` before the
last digit, which is consumed without updating the accumulator. Cancellation
against the mantissa length is therefore different. The observed resulting
lexical exponents are `-18`, `0`, `589806`, and `-589824` in table order;
observed lexical byte reads are `655399`, `1310749`, `655400`, and `1310751`.
The real public query produces positive infinity (filtered to `None`) or
positive zero in the two counterexamples. This source observation neither
pins those private rules as product authority nor replaces the actual-query
oracle. A future compiler changing this behavior should fail this finding's
explicit assertions and trigger re-evaluation, not force the product to keep
an old compiler.

### Bounded restricted witness and resource scope

`UnitFold` recognizes only positive unit-valued powers of ten from an already
parsed Number's borrowed stable `as_str()` content. It counts trailing and
fractional digits and accumulates the explicit exponent with checked scalar
arithmetic. Their final sum proves the exact value is one. `OutsideWitness`
means only that this restricted recognizer supplies no proof; it is not TD
syntax/Basic invalidity. No general integer, float-rounding, or strict JSON
parser claim is made.

The fold consumes one actual Foundation `CodecInputBytes` unit and one shared
lifetime unit before each byte observation. It stores continuation between
steps and does not run `as_f64`, Display, serde, a full-string query, or a
rescan in a charged step. At allowances 1, 7, and 128, tests assert exact byte
charges and `ceil(input_bytes / allowance)` productive calls. Interleaved zero
budgets preserve every cursor field. Fresh budgets cannot replenish lifetime;
exhaustion fixes `Limit`. Cancellation at every byte boundary of the short
`100000e-5` witness fixes `Cancelled`, and subsequent calls cannot replace
that first terminal cause. Final constant scalar arithmetic belongs to the
last charged byte. No work credits are accumulated for a later bulk call.

The restricted fold has a borrowed slice and fixed scalars only: 64 inline
bytes on this Host and no heap allocation sites or destructor. Its own
terminal/drop work requires no arena release, diagnostic allocation, or
cleanup record. This is a source-audited statement about the restricted fold,
not allocator instrumentation of the full probe. Input construction, real
Number parsing, original Basic validation/error allocation, and the lexical
source observer are deliberately outside the fold as setup/oracles. No
retained arena, lossless byte-copy path, ledger, peak/contiguous reservation,
Servient inline owner, or complete cleanup proof is implemented or claimed.
The inputs are below the existing Host default `document_bytes_max` of
1,048,576; the restricted scan is also below its default validation-work
allowance. Full normalization costs are not inferred from that narrow scan.
Resource limits may reject work normally, but this finding supplies no reason
to impose a new lexical cap or classify these Numbers as Basic-invalid.

### Result and stop boundary

There is a concrete impossibility for **exact-value-only** float projection
under the observed existing query semantics: `Number::from(1)` and each long
counterexample have the same exact rational value but different public float
projections. No algorithm depending solely on that rational value can match
all three. This falsifies value-preserving decimal compaction followed by
correct rounding as a sufficient replacement for the frozen query route,
even if it retains lossless source text separately and meets bounded progress.
It does **not** prove that every lexeme-aware resumable projection is
impossible. In particular, no such complete algorithm has been constructed
or accepted by this fixture.

The investigation stops without an authority migration. There is no positive
proof satisfying semantics, progress, resources, and supported features
together, so numeric boundary closure cannot be claimed. The general
rounding corpus, feature-neutral access to Number content, strict-entry
projection, and complete resource/target proof remain unresolved. Silently
substituting mathematically correct parsing would change existing Basic
acceptance. Freezing the observed private exponent threshold, copying
libcore into production, restricting supported compiler/serde graphs, or
allowing an uninterruptible original query would require a different boundary;
none is used to rescue this candidate. A future proposal must preserve actual
observable semantics across the supported policy or deliberately revisit
that policy through its authority owner.

The eight-item coverage table above remains unchanged: this adds a narrow
item-3 semantic counterexample and restricted item-6 progress trace, not a
completed item. Production Rust, authority, admission, resource schema,
public API, work-package statuses, and gate states are untouched. The shared
RFC3339 direction and prior Foundation/Context/Producer evidence are not
falsified by this numeric finding; their full validation is not claimed here.

Reproduce just the new finding (same rust-src requirement and locked Host
arbitrary-precision graph as the original fixture):

```sh
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-number-boundary/Cargo.toml cancellation:: -- --nocapture
```

The full fixture command above runs all six tests. Mainline CI does not run
this standalone observation fixture; green mainline CI is not a substitute
for this explicit reproduction command or independent acceptance.
