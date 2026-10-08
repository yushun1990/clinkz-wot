# Product/runtime boundary experiments

Non-production investigation evidence for
[workspace topic 0076](../../../workspace/0076-composable-capabilities-and-runtime-boundaries.md).
This isolated Cargo workspace is not part of the production dependency graph.
It implements neither a new product Contracts package nor an embedded runtime.

Recovered from `/tmp/clinkz-next-architecture-cjj6jpcj/` and its README, sources,
lockfile, provenance hashes, compile log and generated `plan.rs` output. Original
review baseline: `6e804d2d46ffc6a1a59b316f7e6e729cf0277512`. The probes now compile
the checkout's current production code, rather than preserve a frozen copy.

## Reproduce from a clean checkout

Use Rust/Cargo with edition 2024 support and the repository's dependency MSRVs;
mainline pins Rust 1.95.0. Install the Cortex-M0 standard library target. No board,
C SDK, protocol server, rust-src component or cross linker is needed: the target
command type-checks a library, not a firmware executable.

Run from the repository root:

```sh
rustup target add thumbv6m-none-eabi
cargo run --locked --manifest-path tools/architecture-fixtures/product-runtime-boundaries/Cargo.toml -p runtime-boundary-compare
cargo check --locked --manifest-path tools/architecture-fixtures/product-runtime-boundaries/Cargo.toml -p static-plan-probe --target thumbv6m-none-eabi
cargo tree --locked --manifest-path tools/architecture-fixtures/product-runtime-boundaries/Cargo.toml -p static-plan-probe --target thumbv6m-none-eabi -e normal --prefix none
cargo fmt --manifest-path tools/architecture-fixtures/product-runtime-boundaries/Cargo.toml -p runtime-contracts-probe -p static-plan-probe -p runtime-boundary-compare -- --check
```

Cargo downloads the locked registry dependencies on the first run; add
`--offline` only after caching them. All repository path dependencies are
relative. The fixture lockfile was reduced from the master lockfile so shared
registry dependencies retain master versions. The scratch lock had newer
independently resolved versions; its successful run was reproduced separately.
The mainline workflow executes the three positive evidence commands above.
They are investigation regression checks, not an architecture acceptance gate.

Expected Host output on the recorded x86_64 toolchain:

```text
frozen selection: 15 cases match owned production output; generation mismatch rejected
response cases=3456; identical acceptance and diagnostics
unreachable output combinations rejected by existing constructor=1152
shared build/runtime leaf plan: readproperty form=1 target=zenoh+tcp://127.0.0.1:7447/sensor/temperature?unit=C
Host sizes (error,borrowed output,image)=[232, 56, 72]
```

The normal target graph must be read separately from build/dev dependencies:

```text
static-plan-probe -> runtime-contracts-probe -> clinkz-wot-foundation
```

TD, Core, Planning and the existing mock compiler are build-side dependencies
of `static-plan-probe`; the Host comparator also uses them normally. Successful
target checking alone would not establish this separation; inspect the normal
tree. An added normal dependency invalidates the recorded isolation claim even
if compilation still succeeds.

For the baseline counterexample, run this separately in the repository root:

```sh
cargo check --locked -p clinkz-wot-core --no-default-features --target thumbv6m-none-eabi
```

At the recorded baseline it fails with E0432 on unconditional `alloc::sync::Arc`
imports in `event.rs`, `outbound.rs`, `payload.rs`, `sync.rs` and `thing.rs`.
This negative observation is deliberately not an expected-failure CI rule:
future Core portability improvements may legitimately make it pass. It neither
proves all MCUs unsuitable nor expands supported platform requirements.

## What each probe establishes

| Probe | Production reuse and observed result | Evidence boundary |
|---|---|---|
| `contracts/build.rs` + borrowed views | Reads identity macros/values, errors, operation vocabulary, response metadata, artifact identities/references, response predicate and frozen selector directly from current production source | Only source projection plus observations. No complete registration, slot implementation, scheduler or sealing extraction |
| `compare/src/main.rs` | Calls real `validate_untrusted_binding_output` on a real `OutboundRequest` versus the projected predicate. Eight shape/identity bits × six native statuses × three normalized statuses = 4,608 inputs; 1,152 rejected by production output construction, 3,456 compared, including six successful inputs. Acceptance and exact Debug error diagnostics agree | Finite corpus, not exhaustive equivalence. Native numeric statuses remain opaque. Constructor-rejected shapes are not predicate comparisons |
| `plan_builder.rs` + `prepared/build.rs` | Runs real typed TD Basic validation, exact-coordinate `PropertyReadPlanCompiler` and existing `MockCompiler` in a Cargo build script. Explicitly selects original Form index 1 and resolves the relative URI above. Emits six logical/mock facts as Rust data | Leaf, not Consumer aggregate or production semantic lending. No emitter for real binding artifacts or deployable image |
| `compare/src/selection.rs` | Compares extracted selector with actual Planning selector over 15 property/Form cases; compares returned references or exact Debug errors. A separately corrupted plan-set/plan generation reference is rejected by both | Frozen leaf shape only. The scratch tested corruption only on the extracted view; this fixture adds the matching production negative case |
| Target check and normal tree | Both projected predicates/value dependencies and generated projection type-check on `thumbv6m-none-eabi` with only Foundation on the normal path | No `alloc`, TD, Serde or atomic dependency in that path. No execution, linking, board fit, RAM/flash/WCET or protocol proof |

Host sizes describe only three inline values, excluding allocations and nested
owners. They are informational prints, not pinned ABI, a target measurement,
summable runtime costs or evidence of firmware savings.

## Source drift and handwritten limits

`contracts/build.rs` extracts named, uniquely checked source ranges into Cargo's
`OUT_DIR` and declares every production input with `rerun-if-changed`. It removes
the allocation-bearing `ThingId`, Serde derives/imports and unrelated test
modules. It changes imports and function signatures for borrowed views. The
response predicate body and frozen-selector/error bodies are unchanged; the
selector keeps its Core qualifier through a local alias.

No copied rule program or generated source is checked in. A changed/missing
seam fails extraction, incompatible observations fail compilation, and changed
behavior can fail the production comparison or fixed corpus/result assertions.
This follows the existing source-projection fixture pattern. There is no
snapshot hash to update automatically and thereby disguise rule drift.

`contracts/src/views.rs` is handwritten: borrowed output, three slice-backed
plan/artifact observations, a Form-index option and the six-field static
projection. They intentionally do not implement production output construction,
admission, resource bounds or ownership/lifecycle machinery. They must be
reviewed if production observations acquire new semantics; the finite corpus
cannot catch every possible mismatch or a bug shared by both comparisons.

The projected expected-identity validation function is public **only inside
this experiment**. Production must preserve private registration-minted,
non-Clone single-use result authority, normal and cancellation-late validation,
and absence of raw installed-client bypasses. This seam does not authorize a
public application identity bag. The borrowed payload points into stable
caller storage for a synchronous call; it proves no runtime slot lease.

The generated `plan.rs` uses compiler output through Rust Debug string escaping;
the comparator checks every emitted field against another Host call to the
same production compiler. Generation/configuration compatibility binding,
credentials, runtime activation, payload schemas and real SDK handles are
deliberately absent. The 32-step harness ceiling only detects a stuck leaf; it
is not an engine work/resource limit.

## Recovery disposition and verification

The original scratch Host run, M0 check and normal tree reproduced before
migration on Rust/Cargo 1.99.0 (`b940084d7` / `5f94df478`), x86_64 Linux.

| Temporary material | Repository disposition |
|---|---|
| Comparator, selection corpus, shared plan builder and build-script emitter | Retained, formatted and renamed as fixture packages; relative paths; corpus/result assertions and finite harness progress added |
| Copied `contracts/{identity,error,operation,metadata,response,frozen}.rs` | Replaced by live production-source projection plus small handwritten views; no independent rule copies |
| `predicate-provenance.txt` | Inspected; static hashes superseded by extracting current bodies on every relevant source change |
| Scratch Cargo manifests/lock | Same role/dependency split retained; isolated workspace and non-publishable packages. Lock rebuilt from repository pins, not the scratch's newer resolution |
| Generated `plan.rs`, compile logs, target directories and binaries | Inspected as outputs, not committed; recreated by documented commands |
| `/tmp/clinkz-wot-architecture-design.md` | Used as investigation input; candidate design/rationale retained in topic 0076, not copied as a second completed review or current-state document |

No source needed for these probes was missing. Earlier PR #134's broad graph,
size/cache and Pico probes remain scoped to that independent review; this
fixture does not claim to preserve or rerun every experiment from that PR.

Local verification of this preservation change, on the same Rust/Cargo 1.99.0
Host and baseline above:

| Command | Observed result |
|---|---|
| Fixture `cargo run`, M0 `cargo check` and normal `cargo tree` above (with cached dependencies and `--offline`) | Passed; comparison counts/target and three-package normal graph shown above |
| Fixture package-scoped `cargo fmt ... -- --check` above | Passed |
| `cargo test --locked -p clinkz-wot-core --lib response::tests` | 9 passed |
| `cargo test --locked -p clinkz-wot-servient --no-default-features --test property_read` | 4 passed; existing unused legacy registry warnings |
| `tools/check-design-artifacts.sh` | Passed; active authority/DAG/API/ADR checks, 196 resource fields / 3 profiles, 66 performance cases |
| `cargo test --workspace --locked` | 582 passed, 0 failed, 3 existing ignored doctests; existing unused `FIELDS` and atomic-method deprecation warnings |
| Baseline Core M0 command above | Exit 101, E0432 `alloc::sync` imports plus cascading E0282 inference errors; reproduced counterexample, not a fixture failure |
| `git diff --check` and local Markdown file-link resolution | Passed |

The fixture Host run, M0 check and normal tree also passed in a fresh detached
checkout with empty build directories and only cached registry dependencies.

Interpret failures at their actual boundary: a missing target is setup; a source
seam panic means the projection needs review; a comparison/assertion failure
invalidates the recorded behavior or its adapter; a target type/dependency error
invalidates portability of the projected subset. None automatically selects a
replacement architecture. Core becoming portable would supersede its negative
observation. The ignored doctests are two legacy `InteractionOptions` examples
and the TD flat-module sketch; no new ignored tests are introduced.

Compilation/mock evidence does not establish complete embedded runtime
feasibility, full result sealing, aggregate Consumer admission, global accounting,
scheduling, real protocol I/O or hardware savings. No new hardware/protocol run
was attempted. See topic 0076 for the unresolved evidence needs.
