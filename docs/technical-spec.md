# Technical Specification

## Crate Layout

### `clinkz-wot-td`

Path: `td`.

Owns TD and TM data structures, builders, serialization, deserialization, validation, and extension preservation.

This crate must remain `no_std + alloc` compatible. It must not depend on networking, async runtimes, zenoh, databases, filesystems, or operating-system APIs.

### `clinkz-wot-core`

Path: `core`.

Defines protocol-neutral engine traits and local runtime abstractions
(v4.0 §4). Expected responsibilities:

- Exposed and consumed Thing abstractions (concrete `ExposedThing` /
  `ConsumedThing`; the single-impl traits are removed).
- Sync-primary handler traits (zero-alloc dispatch) + opt-in async twins for
  all nine interaction operations, behind `async` (v4.0 §4.2).
- Client (`ClientBinding` — async `invoke`/`subscribe`) and server
  (`ServerBinding` — sync `try_accept` + wholesale `register_thing`/
  `unregister_thing`) binding trait split (v4.0 §4.5; no `poll_accept`, no
  `AsyncServerBinding`).
- Inbound request/response model (`InboundRequest`, `InboundResponse`).
- Inbound dispatch contract (`InboundDispatcher`).
- Event broker and outbound subscription (`EventBroker`, `Subscription`).
- Identity types (`ThingId`, `CorrelationId`, `Principal`, `PrincipalId`).
- Security provider traits including inbound verification (`SecurityProvider::verify`).
- Payload codec traits.
- Transport adapter traits.
- Lock primitive `WotLock<T>` (replaces `MapLock`; `Arc`-backed `Clone`,
  std `RwLock` / no_std `critical_section::Mutex`) plus a copy-on-write
  snapshot helper for read-heavy-rare-write state (v4.0 §4.7).

This crate supports `no_std + alloc`.

### `clinkz-wot-protocol-bindings`

Path: `protocol-bindings/core`.

Defines common protocol binding utilities:

- Form selection.
- Operation-to-form resolution.
- Target URI resolution from `base` plus `href`.
- Shared binding error types.

This crate should avoid protocol-specific behavior.

### `clinkz-wot-protocol-bindings-zenoh`

Path: `protocol-bindings/protocols/zenoh`.

Implements the first concrete binding because Clinkz Platform uses zenoh as its
default communication bus.

The crate keeps its planning layer (`no_std + alloc`) separate from concrete
runtime backends. The `zenoh` feature provides the Rust `zenoh` (std) backend
with both outbound (`ZenohSessionTransport` implementing `ClientBinding`) and
inbound (`ZenohServerBinding` implementing `ServerBinding`) on a shared
`zenoh::Session`. The `zenoh-pico` feature exposes a `no_std + alloc`
platform-hook backend for constrained devices (outbound only in v1).

This crate is optional and must not be required by TD/TM/core crates.

### `clinkz-wot-discovery`

Implements W3C WoT Discovery concepts and Thing Description Directory behavior.

This crate supports `no_std + alloc` for the protocol-neutral Introduction/
Exploration model, directory reader/publisher/session traits, and the
in-memory reference backend (v4.0 §6). `watch` and `storage` adapters are
available only with the `std` feature. The old `ThingDirectory` CRUD
container, `DirectoryPage { offset, total }`, and the `local` module are
removed; the in-memory backend is a reference `DirectoryReader`/
`DirectoryPublisher`.

### `clinkz-wot-servient`

Composes TD/TM, bindings, discovery, security, and runtime services into a
usable WoT Servient (v4.0 §7).

The Servient is **non-generic** (the old `Servient<D>` is dropped); it holds
`Arc<dyn Discoverer>` + `Option<Arc<dyn DirectoryPublisher>>`. It is `Clone`
(cheap, `Arc`/snapshot-based) with all public methods taking `&self`. Typed
handles (`ExposedThingHandle`, `ConsumedThingHandle`) provide interaction APIs
aligned with the WoT Scripting API.

The driving layer is **async-first** (v4.0 §7.2): `poll_serve` (one request per
call) / `serve` (loop) are the canonical primitives; `poll_serve_once` is the
bare-`no_std` manual-poll super-loop primitive. Inbound accept is a **single
bounded fan-in channel** on std (bindings enqueue via sync `try_send` from
zenoh callbacks; on `Full` request/response is rejected with an explicit error
reply, streaming/events drop-oldest) and a sync `try_accept` poll with a
rotation cursor on no_std. There is no `select_all`, no boxed `poll_accept`,
no `AsyncServerBinding`, no `poll_serve_sync`/`serve_sync` (v4.0 §4.5/§7.2;
audit defects AD1/AD6a/AD6b/AD7/AD9).

The Servient keeps hot-path runtime state in lock-free `Arc` snapshots
(registries, handler tables, binding list, EventBroker fan-out) so reads never
disable interrupts on `no_std`; `WotLock` is reserved for read-write-frequent
state (v4.0 §4.7; AD2). Consumed-Thing entries intern both the selected form
and the live binding instance, and the form cache stores `Arc<Form>` so
repeated consumed interactions avoid deep TD clones.

Late binding factories may optionally provide a lightweight support predicate
in addition to the factory constructor. Servient uses that predicate to skip
instantiating bindings that cannot handle the selected form and operation,
which shortens binding selection on consumed interactions.

The shared `EventBroker` (baseline §9) is wired into the Servient inner state
and fed to each `ServerBinding` via `set_event_broker` during build and late
registration. `ExposedThingHandle::emit_event` fans event payloads through the
broker to all registered `PublisherSink`s, each of which publishes to its
remote subscriber. Inbound `SubscribeEvent` / `UnsubscribeEvent` /
`ObserveProperty` / `UnobserveProperty` operations are routed through the
broker-backed sink. The verified `Principal` from inbound security verification
is threaded into handler `InteractionInput` so handlers can authorize
per-caller.

Consumer-side streaming is provided through
`ConsumedThingHandle::subscribe_event` / `observe_property`, which return a
long-lived `Subscription` for draining pushed samples. The underlying
`ClientBinding::subscribe` method opens a wire subscription and returns a
`SubscriptionGuard` for protocol-specific cleanup. `unsubscribe_event` /
`unobserve_property` stop the wire subscription and release resources.

Handler traits follow the W3C Scripting API split: `PropertyReadHandler`,
`PropertyWriteHandler`, `PropertyObserveHandler`, `EventSubscribeHandler`, and
`EventUnsubscribeHandler` are registered independently, allowing read-only,
write-only, and observable affordances. Bulk property operations
(`read_multiple_properties`, `read_all_properties`, `write_multiple_properties`)
are available on both handles. On the consumed side they prefer a single
Thing-level form declaring the matching bulk meta-operation
(`readallproperties`, `readmultipleproperties`, `writemultipleproperties`;
W3C TD §6.3.3) when the consumed TD advertises one, splitting the combined
JSON-object response into a per-property map; otherwise they fall back to one
round trip per property. The inbound serving path dispatches the same
meta-operations by fanning out across the exposed property handlers and
combining their outputs into a single JSON-object response, so a Thing-level
bulk form is servable end-to-end.

Discovery follows the W3C Scripting API §5 process model: `Servient::discover`
accepts a `DiscoveryFilter` and returns a lazy `ThingDiscoveryProcess` (v4.0 §9.5
— replaces the transitional `ThingFilter`/`ThingDiscovery`). Callers drain the
process asynchronously with `ThingDiscoveryProcess::next()` (the only drain
primitive; the old `next_now()` is removed). v1 is local-only (in-memory
backend); transport-backed discovery remains protocol-specific (deferred per E6).

Security metadata belongs to the binding/transport layer, not handler inputs
(v4.0 §4.3/AD21 — `InteractionInput.security_metadata` is **removed**).
Outbound security application stays on the `SecurityProvider::apply` path
(bindings send the applied headers as protocol-level headers or zenoh
attachments). Inbound auth material is extracted from zenoh query/sample
attachments and surfaced as `AuthMaterial::BearerToken` in `InboundRequest.auth`;
the verified `Principal` is injected into the handler-facing `InteractionInput`.

Error mapping is shared via `clinkz_wot_protocol_bindings::error_status`, which
maps `CoreError` variants to HTTP-like status codes. Bindings include the status
in error replies.

Graceful shutdown is provided by `Servient::shutdown_handle()`, returning a
`Clone`-able `ShutdownHandle` that signals `serve` / `poll_serve` /
`poll_serve_once` to exit after the current iteration.

A credential vault (`CredentialStore` trait, `InMemoryCredentialStore`) provides
protocol-neutral secret storage. `SecurityContext.credentials` passes the store
to `SecurityProvider::apply` so providers retrieve stored credentials by Thing
ID and scheme name instead of capturing them in closures.

**No runtime TD mutation after `expose()`** (v4.0 decision 2 / AD8): the TD is
frozen at `expose()`; `add_property`/`add_action`/`add_event`/`remove_*` and
dynamic-affordance network propagation are removed. The lifecycle is
`produce` (draft, no registry) → configure handlers → `expose` (single registry
insert) → `destroy` (single removal, gone).

`ConsumedThingHandle` methods are natively async (real async `ClientBinding` on
std; the fake "delegates to sync" path is removed). Handler dispatch is
sync-primary (zero-alloc); opt-in async handler twins for all nine interaction
operations are available behind `async` (v4.0 §4.2). When no async handler is
registered, the dispatcher calls the sync handler directly.

The crate supports `no_std + alloc` for runtime composition through the crate
root. Concrete std-only sessions, filesystems, async runtimes, databases, and
observability integrations stay behind the crate's `std` feature.

## Feature Policy

- `default = ["std"]` may be used for std runtime and cloud convenience.
- `alloc` enables dynamic data structures in `no_std` environments.
- `std` enables networking, filesystems, async runtimes, integration tests, and richer diagnostics.
- `async` enables the native-async driving layer (`poll_serve` / `serve`), the
  opt-in async handler twins (all nine ops), and native async `ClientBinding`.
  On `no_std`, `poll_serve_once` drives the same surface from a bare super-loop.
- `zenoh` enables the Rust `zenoh` (std) backend including `ZenohServerBinding`.
- `zenoh-pico` enables the constrained `no_std + alloc` platform-hook backend
  (mutually exclusive with `zenoh`).
- `td2-preview` enables experimental TD 2.0 tracking (currently the
  `ActionAffordance.synchronous` field; the full TD 1.1 `op` vocabulary
  including `cancelaction`, `subscribeallevents`, and `unsubscribeallevents`
  is always available).

Crates that expose both embedded-ready and std-only surfaces should keep both
surfaces in the same crate when the split is only a feature boundary. Use
module names that describe the capability or backend rather than naming a
module solely after `std` or `no_std` availability. Avoid a module named `core`
because `clinkz-wot-core` already names the protocol-neutral engine trait
crate.

## Validation Levels

- Minimal validation: serde shape and basic document structure.
- Basic validation: TD/TM required fields, type constraints, operation context, URI references, URI templates, default handling, and `OneOrMany`.
- Profile validation: WoT Profile compatibility checks.
- Full validation: semantic and behavioral assertions where practical.

Validation should be explicit. Deserialization should not reject documents merely because a stronger validation profile would reject them.

## Serialization Policy

- Preserve unknown extension fields.
- Preserve JSON-LD context entries.
- Preserve compact `OneOrMany` forms semantically.
- Default serialization should target TD 1.1.
- TD 2.0 serialization should be gated behind an experimental feature.

## Error Policy

- Public errors must be stable enough for downstream users.
- Error messages must be written in English.
- Protocol-specific error details belong in binding crates.
- TD/TM validation errors should not depend on runtime or transport concepts.
