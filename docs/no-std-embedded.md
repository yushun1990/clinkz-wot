# no_std and Embedded Support

## Target

`clinkz-wot` should support constrained gateway deployments, including future ESP32-class environments.

The embedded target is `no_std + alloc` for TD/TM, protocol-neutral core
runtime abstractions, shared binding utilities, Discovery, and Servient runtime
composition.

## Supported Embedded Capabilities

Embedded-ready crates should support:

- TD construction.
- TM construction.
- TD/TM serialization and deserialization using allocation-backed buffers.
- Minimal and basic validation.
- Local Thing registration and allocation-backed Thing Directory storage.
- Local property, action, and event dispatch.
- Embedded Servient composition with injected protocol bindings, payload
  codecs, security providers, and caches.
- Abstract transport adapters supplied by the platform.
- Owned inbound interaction model (`InboundRequest`, `InboundResponse`,
  `AffordanceTarget`, `BindingRequest`) that is `'static` and usable across
  spawnable boundaries (baseline v3.1 §2).
- Sync inbound driving via `poll_serve_sync` on MCU super-loops (baseline
  addendum §6.2). The native-async driving layer (`async` feature) is deferred.
- `ServerBinding` trait is dyn-compatible, allowing `Vec<Arc<dyn ServerBinding>>`
  storage in both sync and (future) async builds.
- `MapLock` shared locking primitive in `clinkz-wot-core` usable across core
  and servient.

## Non-Goals for v1

The initial embedded target does not require:

- A hard dependency on zenoh in embedded builds.
- Remote JSON-LD context fetching.
- Full JSON-LD expansion on-device.
- Filesystem-backed storage.
- Cloud-oriented observability stacks.
- Host-owned protocol sessions, async runtimes, sockets, or database-backed
  directories inside embedded-ready crates.

## Dependency Rules

- Use `alloc` types such as `String`, `Vec`, and `BTreeMap` where needed.
- Avoid `std` imports in embedded-ready crates.
- Keep async runtime dependencies out of embedded-ready crates.
- Keep networking dependencies behind binding crates or platform adapters.
- Avoid hidden feature defaults that pull in `std`.

## Checks

Embedded-ready crates should pass no-std checks similar to:

```sh
scripts/check-no-std.sh
```

The repository-level verification path is documented in
`docs/verification.md`.

The current no-std check script covers:

- `clinkz-wot-td`
- `clinkz-wot-core`
- `clinkz-wot-protocol-bindings`
- `clinkz-wot-protocol-bindings-zenoh`
- `clinkz-wot-discovery`
- `clinkz-wot-servient`
- `clinkz-wot-core --features async` (async `no_std` flavor)
- `clinkz-wot-servient --features async` (async `no_std` flavor)

When an explicit `alloc` feature is introduced, checks should include:

```sh
cargo check -p clinkz-wot-td --no-default-features --features alloc
```

Additional target checks should be added once the exact ESP32 Rust target and platform stack are selected.

## Design Notes

Embedded support should not force every binding to be embedded-compatible.

The engine should allow a device to expose local Thing behavior through a platform-provided adapter. If zenoh is available in a constrained deployment, the zenoh binding can be used. If not, another binding or adapter can be used without changing TD/TM/core logic.

`discovery` separates its `no_std + alloc` and `std` capabilities by
responsibility rather than by environment labels. The crate root keeps the
shared directory and query model. `discovery::local` exposes local
allocation-backed directory capabilities usable without `std`.
`discovery::storage` is available only with the `std` feature for shared
storage adapters and future production storage extension points.

`servient` exposes no-std Servient APIs through the crate root. The
single-generic `Servient<D>` is `Clone` with `&self` methods, using `MapLock`
from `clinkz-wot-core` for interior mutability. The sync driving layer
(`poll_serve_sync`) is available without `std` and is intended to be called as
a stepwise primitive from the MCU super-loop: one call processes at most one
inbound request. Std-only Servient integrations (`serve_sync`,
`std::eprintln!` diagnostics, host idle backoff) stay behind the `std` feature.
This keeps the sync API usable in both embedded and host deployments without
forcing the no-std super-loop semantics onto host runtimes. The native-async
driving layer and `Send + Sync` lock primitives are deferred behind the
`async` feature (SR-P2.2). The project avoids naming these modules `core`
because `clinkz-wot-core` already denotes the
protocol-neutral engine trait crate.
