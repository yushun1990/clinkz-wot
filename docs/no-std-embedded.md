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

Embedded-ready crates should pass checks similar to:

```sh
cargo check -p clinkz-wot-td --no-default-features
cargo check -p clinkz-wot-core --no-default-features
cargo check -p clinkz-wot-protocol-bindings --no-default-features
cargo check -p clinkz-wot-protocol-bindings-zenoh --no-default-features
cargo check -p clinkz-wot-discovery --no-default-features
cargo check -p clinkz-wot-servient --no-default-features
```

When an explicit `alloc` feature is introduced, checks should include:

```sh
cargo check -p clinkz-wot-td --no-default-features --features alloc
```

Additional target checks should be added once the exact ESP32 Rust target and platform stack are selected.

## Design Notes

Embedded support should not force every binding to be embedded-compatible.

The engine should allow a device to expose local Thing behavior through a platform-provided adapter. If zenoh is available in a constrained deployment, the zenoh binding can be used. If not, another binding or adapter can be used without changing TD/TM/core logic.

`discovery` and `servient` keep embedded and host surfaces inside their own
crates. The embedded surface is exposed through an `embedded` module. The host
surface is exposed through a `host` module behind the default `std` feature.
The project avoids naming these modules `core` because `clinkz-wot-core`
already denotes the protocol-neutral engine trait crate.
