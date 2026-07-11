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
- Sync inbound driving via `ServerBinding::try_accept` polled from the MCU
  super-loop, dispatching via `Dispatch::serve_request` (v4.0 §4.5/§7.3).
  The driving loop is **binding-owned**, not on the Servient: each binding
  decides whether to push inbound requests into a fan-in channel (zenoh),
  call `Dispatch::serve_request` directly from an async handler
  (HTTP/CoAP), or expose `try_accept` for a super-loop to poll (bare
  `no_std`). There is no `poll_serve` / `serve` / `poll_serve_once` on the
  Servient — those primitives were removed when the driving loop moved
  out of the Servient (commit c03de58).
- `ServerBinding` exposes a **synchronous `try_accept`** (no boxed
  `poll_accept`, no `select_all`) and wholesale `register_thing`/
  `unregister_thing` (v4.0 §4.5).
- `WotLock<T>` locking primitive in `clinkz-wot-core` usable across core and
  servient. Read-heavy-rare-write state (registries, handler tables) uses
  lock-free `Arc`-snapshot reads; `WotLock` (std `RwLock` / no_std
  `critical_section::Mutex`) is reserved for read-write-frequent state.
- Handlers are **synchronous and primary** (zero per-call allocation on the
  inbound hot path); opt-in async twins behind the `async` feature serve
  I/O-bound cloud/gateway handlers (v4.0 §4.2).
- **Multi-thread RTOS support is inherent, not feature-gated.** The unified
  lock primitive is always thread-safe (std `RwLock` / no_std
  `critical_section::Mutex`); the prior `multithread` feature, `RefCell`/
  `UnsafeCell` split, and `DrainFlag`/`Cell` toggles are removed (v4.0 §4.7).
  The engine runs safely across RTOS tasks (FreeRTOS, Zephyr) on multi-core MCU
  gateways without `std` or an async runtime.

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

`servient` exposes no-std Servient APIs through the crate root. `Servient`
(non-generic in v4.0; the old `Servient<D>` is dropped) is `Clone` with `&self`
methods, using `WotLock<T>` from `clinkz-wot-core` for interior mutability and
lock-free `Arc`-snapshot reads for the read-heavy registries/handler tables.
**Dispatch is binding-owned** (commit c03de58): the Servient exposes only
`Dispatch::serve_request(req).await` for bindings to call from whichever
driving model fits their transport — tokio task draining a fan-in channel
(zenoh), direct async route handler (HTTP/CoAP), or super-loop polling
`ServerBinding::try_accept` (bare `no_std`). The `poll_serve` / `serve` /
`poll_serve_once` primitives that used to live on the Servient have been
removed. Std-only Servient conveniences (`ServientBuilder`, host conveniences)
stay behind the `std` feature. The project avoids naming these modules `core`
because `clinkz-wot-core` already denotes the protocol-neutral engine trait
crate.

## MCU Gateway Path: Two-Layer Plan

For MCU gateways (ESP32, STM32, nRF52) that serve multiple Things and need
concurrent request handling across sub-devices (BLE, Modbus, SPI), the
following layers close the gap. (v4.0 collapses the prior three-layer plan:
multi-thread safety is now inherent — no feature flag — so Layer 1 is gone.)

### Layer 1: zenoh-pico concrete platform — DEFERRED (hardware-specific)

The `ZenohPicoPlatform` trait in
`protocol-bindings/protocols/zenoh/src/runtime/zenoh_pico.rs` defines the
platform hook for constrained zenoh-pico C ABI integrations. The pico backend
**will** implement `ServerBinding::try_accept` (synchronous-readiness, polled
by the MCU super-loop, then dispatched via `Dispatch::serve_request`) and the
async `ClientBinding` — but this is **deferred** (P2 §2.7, AD16): the
server-side `try_accept` model and the pico server module are not yet
implemented/finalized. A concrete implementation requires:

- Target MCU selection (ESP32, STM32, nRF52, etc.).
- zenoh-pico C library compiled for the target.
- Rust C ABI bindings (FFI) for session, get, put, subscribe, undeclare.
- Buffer management and polling model (cooperative vs preemptive).
- A `critical_section::Impl` registration for the target's interrupt model.

Multi-thread safety across RTOS tasks is inherent (`WotLock` is always
thread-safe; no `multithread` feature). Blocked by: PLAN.md "Defer `zenoh-pico`
runtime injection until the target hardware platform, C ABI strategy, and
polling model are confirmed."

### Layer 2: Embassy async (concurrent dispatch on MCU) — DEFERRED

With a concrete zenoh-pico platform (Layer 1), the engine compiles and runs on
`no_std + alloc`, driven from the super-loop via `ServerBinding::try_accept` →
`Dispatch::serve_request`. Layer 2 adds the `embassy` async runtime for
concurrent dispatch:

- New `embassy` feature on `clinkz-wot-servient` (alongside existing `async`
  which uses tokio).
- Binding-owned driving tasks ported to embassy primitives
  (`embassy_futures::select`, `embassy_time::Timer`).
- No `tokio::select!` or `tokio::time::sleep` — embassy equivalents.
- Enables cross-Thing concurrent dispatch on MCU (same fan-in +
  `FuturesUnordered` pattern as the tokio path, but on no_std).

Blocked by: requires embassy executor + target (or simulator) for testing.
The binding-owned driving design is runtime-agnostic and portable to embassy
once the target is available.
