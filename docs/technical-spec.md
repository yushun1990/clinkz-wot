# Technical Specification

## Crate Layout

### `clinkz-wot-td`

Path: `td`.

Owns TD and TM data structures, builders, serialization, deserialization, validation, and extension preservation.

This crate must remain `no_std + alloc` compatible. It must not depend on networking, async runtimes, zenoh, databases, filesystems, or operating-system APIs.

### `clinkz-wot-core`

Path: `core`.

Defines protocol-neutral engine traits and local runtime abstractions.

Expected responsibilities:

- Exposed and consumed Thing abstractions.
- Property, action, and event handler traits.
- Client (`ClientBinding`) and server (`ServerBinding`) binding trait split.
- Inbound request/response model (`InboundRequest`, `InboundResponse`).
- Inbound dispatch contract (`InboundDispatcher`).
- Event broker and outbound subscription (`EventBroker`, `Subscription`).
- Identity types (`ThingId`, `CorrelationId`, `Principal`, `PrincipalId`).
- Security provider traits including inbound verification (`SecurityProvider::verify`).
- Payload codec traits.
- Transport adapter traits.
- Shared locking primitive (`MapLock`).

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

This crate supports `no_std + alloc` for the protocol-neutral query model,
directory traits, and deterministic in-memory directory. The `local` module
contains no-std local directory capabilities. The `storage` module is available
only with the `std` feature for shared storage adapters and future production
storage extension points.

### `clinkz-wot-servient`

Composes TD/TM, bindings, discovery, security, and runtime services into a
usable WoT Servient.

The Servient is a single-generic `Servient<D>` (parameterized by the `ThingDirectory`
type `D`). It is `Clone` (cheap, `Arc`-based) with all public methods taking
`&self`. Typed handles (`ExposedThingHandle`, `ConsumedThingHandle`) provide
interaction APIs aligned with the WoT Scripting API.

The sync driving layer (`poll_serve_sync`, `serve_sync`) polls registered
`ServerBinding`s and dispatches inbound requests through the `InboundDispatcher`.
`poll_serve_sync` is stepwise: each call processes at most one inbound request
across all registered sync bindings and rotates the polling start point to keep
multi-binding scheduling fair. `serve_sync` is the std-host convenience loop on
top of that primitive and may apply idle backoff when no request is available.
The native-async driving layer (`poll_serve`, `serve`) is gated behind the
`async` feature and uses `AsyncServerBinding` (dyn-compatible via
`#[async_trait]`) with a persistent per-binding accept set to race all async
bindings concurrently without rebuilding the whole wait set on each iteration
(baseline v3.0 §4, addendum §2.4 / §6.2).

The Servient keeps hot-path runtime state in snapshots rather than repeatedly
cloning live vectors. Registered sync and async server bindings are mirrored in
`Arc<[...]>` snapshots so the driving loops can clone a single shared pointer
per poll. The shared `EventBroker` uses the same snapshot style for
`ThingId`/`EventName` fan-out tables so `publish` can release the broker lock
before delivering to sinks. Consumed-Thing entries intern both the selected
form and the selected binding plan, and the form cache stores `Arc<Form>` so
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
accepts a fragment-oriented `ThingFilter` and returns a `ThingDiscovery`
process object. Callers drain the process synchronously with
`ThingDiscovery::next_now()` or asynchronously with `ThingDiscovery::next()`.
The current implementation uses the in-memory directory backend; transport-
backed discovery remains protocol-specific.

Security metadata is separated from URI variables:
`InteractionInput.security_metadata` carries transport-level auth headers
applied by `SecurityProvider::apply`; bindings SHOULD send these as
protocol-level headers or zenoh attachments. Inbound auth material is extracted
from zenoh query/sample attachments and surfaced as `AuthMaterial::BearerToken`
in `InboundRequest.auth`.

Error mapping is shared via `clinkz_wot_protocol_bindings::error_status`, which
maps `CoreError` variants to HTTP-like status codes. Bindings include the status
in error replies.

Graceful shutdown is provided by `Servient::shutdown_handle()`, returning a
`Clone`-able `ShutdownHandle` that signals `serve_sync`, `serve`, and
`poll_serve_sync` to exit after the current iteration.

A credential vault (`CredentialStore` trait, `InMemoryCredentialStore`) provides
protocol-neutral secret storage. `SecurityContext.credentials` passes the store
to `SecurityProvider::apply` so providers retrieve stored credentials by Thing
ID and scheme name instead of capturing them in closures.

Runtime TD mutation (`add_property` / `add_action` / `add_event` and their
`remove_*` counterparts on `ExposedThingHandle`) allows dynamic affordance
lifecycle after `expose`.

Async consumer methods (`read_property_async`, `write_property_async`,
`invoke_action_async`, `subscribe_event_async`, `observe_property_async`) are
available behind the `async` feature. The current implementation delegates to
the synchronous path, providing a forward-compatible API for future native
async bindings.

Async handler traits (`AsyncPropertyReadHandler`, `AsyncPropertyWriteHandler`,
`AsyncActionHandler`) are available behind the `async` feature. The async
driving loop uses a take-out / await / return dispatch pattern that avoids
holding the thing slot lock across `.await`, allowing async handlers to perform
async I/O without blocking the driving loop. When no async handler is
registered for an affordance, the async dispatch falls back to the synchronous
handler.

The crate supports `no_std + alloc` for runtime composition through the crate
root. Concrete std-only sessions, filesystems, async runtimes, databases, and
observability integrations stay behind the crate's `std` feature.

## Feature Policy

- `default = ["std"]` may be used for std runtime and cloud convenience.
- `alloc` enables dynamic data structures in `no_std` environments.
- `std` enables networking, filesystems, async runtimes, integration tests, and richer diagnostics.
- `async` enables the native-async driving layer (`poll_serve` / `serve`),
  `AsyncServerBinding` trait (dyn-compatible via `#[async_trait]`), and
  `Send + Sync` lock primitives.
- `zenoh` enables the Rust `zenoh` (std) backend including `ZenohServerBinding`.
- `zenoh-pico` enables the constrained `no_std + alloc` platform-hook backend
  (mutually exclusive with `zenoh`).
- `td2-preview` enables experimental TD 2.0 tracking.

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
