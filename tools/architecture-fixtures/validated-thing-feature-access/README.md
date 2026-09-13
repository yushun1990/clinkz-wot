# ValidatedThing Number public-access boundary probe

This is non-production pre-readmission impact evidence. It is based on master
`446fe02` (github-pr:83), with `WP-100-CONSUMER-VALIDATED-THING` still
`planned` / `candidate` / `current`. It does not claim any of the eight
readmission items complete or authorize an admission transition.

## Reproduce

The standalone lockfile selects serde_json 1.0.149, matching master. The
commands are intentionally separate because Cargo unifies dependency features
across the packages selected in one invocation.

```sh
# Expected E0599: Number::as_str does not exist in the base graph.
cargo check --locked --offline --manifest-path tools/architecture-fixtures/validated-thing-feature-access/Cargo.toml -p validated-thing-feature-access-borrow

# Expected pass: a downstream direct dependency unifies arbitrary_precision,
# while the upstream TD-like package's local cfg remains false.
cargo test --locked --offline --manifest-path tools/architecture-fixtures/validated-thing-feature-access/Cargo.toml -p validated-thing-feature-access-unifier

# Expected pass: a graph-neutral callback experiment in the base graph.
cargo test --locked --offline --manifest-path tools/architecture-fixtures/validated-thing-feature-access/Cargo.toml -p validated-thing-feature-access-portable

# Expected passes on the real no_std + alloc target.
cargo check --locked --offline --manifest-path tools/architecture-fixtures/validated-thing-feature-access/Cargo.toml -p validated-thing-feature-access-portable --target thumbv7em-none-eabihf
cargo check --locked --offline --manifest-path tools/architecture-fixtures/validated-thing-feature-access/Cargo.toml -p validated-thing-feature-access-unifier --target thumbv7em-none-eabihf
```

## Exact finding

`Number::as_str` is the documented borrowed decimal-text API, and serde_json
gates it behind its own `arbitrary_precision` feature. The base graph fails
the unconditional call. A legal downstream unifier makes that call compile,
but does **not** set a local `arbitrary_precision` cfg in TD. TD currently has
no feature forwarding this dependency feature, and its Cargo manifest is not
one of the frozen future production paths. Thus TD cannot use a local cfg to
select `as_str` for all legal downstream feature unifications without a new
manifest/public feature boundary.

The `portable` experiment demonstrates why the failure is narrower than a
claim that every conceivable graph-neutral access is impossible. With the
current serde_json source, a custom `serde::Serializer` receives an integer or
float callback in the base graph and a `serialize_struct` / `serialize_field`
callback carrying a borrowed decimal string in the arbitrary-precision graph.
The probe reads one selected byte from that callback without allocation; for
the scalar callback it uses the original Number's bounded Display into a
64-byte inline buffer. It passes Host base and arbitrary-precision tests,
including a 65,540-byte exponent witness, and both real target compile cells.

That route is **not** a positive readmission proof. The arbitrary-precision
`Serialize` callback shape is based on serde_json's private `TOKEN` struct
representation (see `number.rs` in the selected dependency source), not on a
documented `Number` semantic-access contract. Its shape could change under
the admitted semver policy without changing the public `Number::as_str`
contract. The experiment also has no full Number cursor, exact-comparison
continuation, ledger, arena, or rollback proof. Direct `Display` of an
input-sized arbitrary-precision Number inside a charged step is expressly
disallowed by the admission record; `to_string` would allocate an additional
input-sized String. Existing scalar `as_f64` queries lose lexical and exact
decimal information, as the #81/#82 fixtures already show.

The current approved options therefore do not establish a stable, graph-
neutral public decimal cursor: direct `as_str` fails a required compile cell;
the only demonstrated common-source workaround depends on a private callback
shape; and the other public formatting/scalar routes violate the frozen
allocation, progress, or exact-decimal constraints. Accepting the callback
shape as authority, adding an explicit TD feature/manifest route, or changing
the supported graph/API policy needs an independent impact decision. This
fixture deliberately makes none of those changes.

## Readmission impact

Item 3 (lossless Number corpus), item 5 (all feature graphs), and item 6
(byte-charged Number continuation) cannot yet be accepted from this probe.
Items 1, 2, 4, 7, and 8 are not attempted. No production Rust, resource schema,
work-package state, predecessor evidence, or Producer gate is modified.
