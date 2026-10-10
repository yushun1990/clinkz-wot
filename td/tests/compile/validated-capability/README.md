# Production TD capability boundary

This normal downstream dependency graph exercises the implemented frozen
configuration/progress/proof/lending surface of `td/validated-thing`. It does
not import the source-projection candidates.

Use separate Cargo invocations for each requested graph. `positive` imports
must compile with `capability` or `sibling`; `negative` imports must fail without
either, including when downstream serde enables AP. `ap` and `order` request
serde's independent axes. `std`, `--no-default-features`, and `async` select
Host/default, portable alloc, and Foundation async composition. `order` is
Host-only. Inspect `cargo tree -e normal,build,features` for the resolved normal
dependency features; TD must add its AP edge only when its own capability is enabled.

Example positive and expected negative:

```sh
cargo check --locked --manifest-path td/tests/compile/validated-capability/Cargo.toml --features capability,positive,order
cargo check --locked --manifest-path td/tests/compile/validated-capability/Cargo.toml --features negative,ap
```

The second command must report unresolved TD validated exports. Proof/config
forgery, cloned proof and premature input destruction are production TD
`compile_fail` doctests, run with `cargo test -p clinkz-wot-td --features validated-thing --doc`.
Semantic loan and scratch retention negatives are production TD doctests;
the external semantic join also prohibits raw aggregate input and premature
source/registration destruction. Each Host graph also runs public Thing/schema
Basic against the resolved normal dependencies, including binary64 rounding,
underflow, non-Number predicates and all five failed projections wherever AP
Numbers are constructible. A lexical preservation assertion distinguishes base
and AP resolution without TD dev-dependency unification. The mainline capability
step runs all 24 Host off/direct/sibling graph cells and all 12 corresponding
portable/async ARM cells in separate invocations, with resolved feature trees.
The independent lock records this graph, not a dependency pin.
