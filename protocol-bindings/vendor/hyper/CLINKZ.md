# Private HTTP SDK correction

This is Hyper **1.12.0-clinkz.1**, a repository-pinned dependency for the
Host HTTP Consumer binding. It is not an upstream release. The MIT license is
retained in `LICENSE`. Source: the crates.io `hyper-1.12.0.crate` archive,
SHA-256 `2c3e324da4c95177d6291d4c8730197c0d1822f8a9766814a4a44fa5ab797c9c`.

All 64 upstream `src/` files are included. Exactly three differ:

- `src/client/conn/http1.rs` adds `Builder::chunked_limits(fields, bytes,
  extensions)`, independent of the main response-head settings.
- `src/proto/h1/conn.rs` carries that immutable configuration into each decoder.
- `src/proto/h1/decode.rs` applies it, checks trailer bytes before appending
  (inclusive maximum), refuses a field beyond the count at its first byte,
  and uses a configurable, non-resetting extension counter. Default extension
  behavior remains the upstream 16383 accepted bytes. Decoder test argument
  construction includes the new counter ceiling.

`Cargo.toml` changes only the package version and adds a standalone workspace
boundary. Original manifest, README and license are retained byte-for-byte.
The standalone `Cargo.lock` pins the SDK's test graph; the root lock pins the
production graph. Compare with an extracted checksum-verified archive using
`diff -ru`. No root `[patch]` override or downstream patch configuration is
required. The HTTP crate's direct, exact-version path dependency selects this
source; ordinary workspace members do not select it.

Qualification is for `default-features = false, features = ["client", "http1"]`.
The binding implements Hyper's I/O traits on its existing ingress wrapper,
using Tokio for the underlying socket. `hyper-util` and unmodified Hyper remain
dev dependencies only for the original, independently executable SDK
counterexamples in `tests/sdk_limits.rs`.

Run the production network/ownership cases with:

```sh
cargo test --locked --offline -p clinkz-wot-binding-http --all-targets -- --test-threads=1 --nocapture
```

The upstream decoder unit tests require the upstream test harness's `full`
feature graph, even when filtering to HTTP/1 decoder tests:

```sh
cargo test --locked --offline --manifest-path protocol-bindings/vendor/hyper/Cargo.toml --lib --no-default-features --features full proto::h1::decode -- --test-threads=1
```

That test-only graph does not enable HTTP/2 or server features in the production
binding. Only the executed HTTP Consumer subset is qualified; other Hyper
features, platforms, native stack maxima, arbitrary SDK allocation failures
and universal disposal bounds are not established. Dependency maintenance,
upstreaming/replacement and registry distribution of this private version
remain explicit obligations before a published release. The binding README
and its existing WP-600 evidence owner describe deployment and measured costs.
