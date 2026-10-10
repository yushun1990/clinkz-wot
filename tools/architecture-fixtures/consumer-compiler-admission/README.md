# Consumer compiler admission observation

Small non-production reproduction for
[workspace topic 0077](../../../workspace/0077-consumer-compiler-admission-boundary.md).
It calls current Core Host erasure and the existing concrete `MockCompiler`
directly. It implements no TD semantics, aggregate, resource governor or compiler
replacement. This isolated non-publishable package does not alter the root
workspace, production dependencies, workflows or accepted evidence.

Run from the repository root using CI's Rust 1.95.0:

```sh
RUSTUP_TOOLCHAIN=1.95.0 cargo run --locked --manifest-path tools/architecture-fixtures/consumer-compiler-admission/Cargo.toml
RUSTUP_TOOLCHAIN=1.95.0 cargo fmt --manifest-path tools/architecture-fixtures/consumer-compiler-admission/Cargo.toml -- --check
RUSTUP_TOOLCHAIN=1.95.0 cargo tree --locked -p clinkz-wot-planning --no-default-features --edges normal,features
```

The fixture lock was seeded from the root lock and pruned by Cargo. All path
dependencies use the current checkout. `--offline` is optional after registry
dependencies are cached. The last command inspects the separate production
Planning graph; it currently requests no TD `validated-thing` capability.

The single-threaded binary provisions plans/registrations before observation.
Its System allocator observer records actual allocation/release size and
alignment in fixed atomic storage, with no allocation or formatting in the
observation windows. Assertions require equal returned static/Host bounds,
equal concrete target semantics, and these representation differences:

| Callback | Static | Core Host erasure |
|---|---|---|
| bounds | No allocation | No allocation; declaration forwarded unchanged |
| start | No allocation | One concrete cursor box |
| zero-credit Pending | Not part of the static comparison | Releases/reallocates the cursor box despite no underlying compiler progress |
| completing step | Exact target bytes | Exact target bytes plus concrete artifact payload box; cursor box released |
| output destruction | Target released | Target and payload box released with matching Layouts |

Recorded x86_64 output includes:

```text
bounds: target=25, cursor=1, temporary=0
Host complete: Requests { allocations: [(25, 1), (24, 8)], releases: [(1, 1)] }; Host output release: Requests { allocations: [], releases: [(25, 1), (24, 8)] }
Host artifact erasure delta: 24 requested bytes; no aggregate or admission claim
```

Type Layouts are derived from the actual compiled concrete types, not pinned as
ABI or invented artifact budgets. Tuple entries are `(size, alignment)`.
The probe fails if the observed allocation paths change; a legitimate Core
correction should replace these counterexample expectations with its new
admission invariants rather than preserve the old behavior.

This is an observation of existing lower-layer mechanics. There is no aggregate
zero-budget claim, allocation-failure injection, parent/global accounting,
peak/allocator-metadata measurement, variable cleanup, complete-registration
support attestation, bare-target execution, or acceptance claim. Source inspection
owns the observation that these current conversions are infallible `Box`
operations. The completed TD/fixed static join and exact-coordinate Planning
evidence remain valid within their narrower claims. A reviewed compiler support
contract must explain and admit these representation costs before they can be
used for the stronger aggregate contract.
