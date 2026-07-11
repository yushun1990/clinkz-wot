# Protocol Bindings

## Protocol-Neutral Engine

`clinkz-wot` is a protocol-neutral WoT engine.

The engine core must not prefer zenoh, HTTP, CoAP, MQTT, Modbus, BLE, or any other protocol. Protocol choice is deployment policy and should be expressed through TD forms and binding configuration.

## Binding Model

Every protocol binding consumes TD forms and maps them to concrete transport behavior.

Relevant form fields include:

- `href`
- `op`
- `contentType`
- `contentCoding`
- `subprotocol`
- `security`
- `scopes`
- protocol-specific extension terms

Bindings must use the same protocol-neutral trait surface.

## Crate Organization

Protocol binding crates are grouped under `protocol-bindings/`:

- `core`: shared protocol binding utilities published as
  `clinkz-wot-protocol-bindings`.
- `protocols/zenoh`: the concrete zenoh binding published as
  `clinkz-wot-protocol-bindings-zenoh`.

The shared protocol binding crate owns form selection, affordance form lookup,
and target resolution helpers. Concrete protocol crates own transport-specific
metadata parsing and operation mapping.

Runtime crates should own concrete transport sessions and platform integration.
For zenoh, the planning crate remains independent from both the Rust `zenoh`
runtime and `zenoh-pico`; backend selection belongs in runtime adapters that
implement the shared transport trait.

## Zenoh Binding

Zenoh is the first implemented binding because Clinkz Platform uses zenoh as its default communication bus.

Zenoh is not a required dependency of the engine. It belongs in
`clinkz-wot-protocol-bindings-zenoh` or an equivalent optional crate.

The `clinkz-wot-protocol-bindings-zenoh` crate is a protocol binding planning
crate, not a concrete session runtime. It recognizes zenoh TD forms, resolves
resolved targets, parses `cz-zenoh` metadata, maps WoT operations to zenoh
operation kinds, and exposes a `ZenohTransport` adapter boundary. It must stay
usable under `no_std + alloc`.

Host runtimes can wrap a concrete `ZenohTransport` implementation in
`SharedZenohTransport<T>` to reuse one session, connection pool, or runtime
adapter across multiple binding instances created by Servient binding
factories. The shared handle is available only with the planning crate's `std`
feature and does not affect `no_std + alloc` checks.

The feature-selected public runtime alias is `ZenohRuntimeTransport`. When the
crate is built with the `zenoh` feature, that alias resolves to
`ZenohSessionTransport`, which wraps a concrete `zenoh::Session`, supports put,
get/request-reply, and one-shot subscribe execution through the protocol-neutral
`ZenohTransport` trait, and also exposes `ZenohSubscription` for std runtimes
that need explicit subscription receive and undeclare lifecycle control. It
maps form `contentType`, express QoS, priority, and congestion control metadata
onto put and get/request-reply builders. The default crate build remains free
of the Rust `zenoh` dependency.

Concrete zenoh execution should be added through optional runtime backends:

- A Rust `zenoh` backend for std deployments. This backend is `std` because
  the Rust `zenoh` runtime depends on async and socket capabilities.
- A `zenoh-pico` backend for constrained deployments. This backend should live
  behind its own feature or crate and handle C ABI, platform I/O, memory, and
  polling concerns without adding them to TD, core, or shared binding crates.

Concrete runtime backend features use the `runtime-*` prefix:

- `zenoh`: Rust `zenoh` backend.
- `zenoh-pico`: constrained `zenoh-pico` platform-hook backend.

The `zenoh-pico` backend provides a `no_std + alloc` adapter boundary through
`ZenohPicoPlatform` and `ZenohRuntimeTransport`, which resolves to
`ZenohPicoTransport` under the `zenoh-pico` feature. Target-specific code still
owns the real zenoh-pico C ABI calls, session handle, polling, timeout
handling, and buffer ownership. The constrained backend target is documented
in `docs/zenoh-pico-runtime-target.md`.

Concrete backend features are mutually exclusive. The shared planning surface
remains available without selecting either backend.

### Zenoh URI Convention and Session Model

The zenoh binding uses an authority-based URI convention: TD forms resolve to
`zenoh[+<transport>]://<authority>/<key-expr>`, where the authority (RFC 3986
`host[:port]`) names the zenoh router and the path is the key expression. The
authority is mandatory — a TD with an empty authority is a configuration
error, not a default-session fallback. The transport is encoded in the URI
scheme suffix (`zenoh`, `zenoh+tcp`, `zenoh+udp`), following the RFC 8323
`coap+tcp` precedent.

The `std` backend aggregates sessions per authority (`ZenohSessionPool`), so a
Consumer can reach Things on multiple routers from one Servient. The `no_std`
backend uses a single platform-injected session and validates each form's
authority against it. Client and server sessions are never shared, even in a
combined Servient.

The full specification — form parsing, session pool design, credential
acquisition, migration path, and `no_std` handling — lives in
[`docs/zenoh-binding-template.md`](zenoh-binding-template.md).

Expected operation mapping:

- Property read maps to zenoh query or get behavior.
- Property write maps to zenoh put or query-with-reply behavior.
- Property observe maps to zenoh subscribe behavior.
- Action invoke maps to request/reply behavior.
- Event subscribe maps to zenoh subscribe behavior.
- Bulk operations map to key-expression based group operations where appropriate.

## Clinkz Extension Namespace

Clinkz-specific binding terms should use a JSON-LD namespace such as:

```json
{
  "cz": "https://clinkz.io/wot#"
}
```

Zenoh-specific terms may use a more specific namespace if needed:

```json
{
  "cz-zenoh": "https://clinkz.io/wot/zenoh#"
}
```

## Zenoh Extension Vocabulary

Zenoh-specific extension terms belong to the `cz-zenoh` namespace:

```json
{
  "cz-zenoh": "https://clinkz.io/wot/zenoh#"
}
```

These terms are valid on TD forms. The zenoh binding treats every term below as
an optional string-valued extension and rejects non-string or empty string
values.

| Term | Status | JSON type | Purpose |
| --- | --- | --- | --- |
| `cz-zenoh:qos` | Experimental hint | string | Preferred zenoh QoS metadata. |
| `cz-zenoh:priority` | Experimental hint | string | Preferred zenoh priority metadata. |
| `cz-zenoh:congestionControl` | Experimental hint | string | Preferred zenoh congestion control metadata. |

The resolved `href` remains authoritative for the concrete target. Relative
`href` values are resolved against Thing-level `base` (RFC 3986) before the
binding splits the result into a zenoh router authority and a key expression.
The full URI convention — mandatory authority, `+transport` scheme suffix,
session aggregation, and the `no_std` backend — is specified in
[`docs/zenoh-binding-template.md`](zenoh-binding-template.md).

The metadata hint terms are parsed and preserved in the zenoh operation plan,
but the shared engine does not assign mandatory runtime behavior to them. Host
runtime adapters may choose how to translate these hints to a concrete zenoh
session or publication API.

Example form (authority `router.example.com:7447` carries the router address;
the path under it is the key expression):

```json
{
  "base": "zenoh://router.example.com:7447/clinkz/things/lamp/",
  "href": "properties/status",
  "op": "readproperty",
  "contentType": "application/json",
  "cz-zenoh:qos": "express"
}
```

This resolves to
`zenoh://router.example.com:7447/clinkz/things/lamp/properties/status` →
authority `router.example.com:7447`, key expression
`clinkz/things/lamp/properties/status`. A non-TCP transport uses a scheme
suffix, e.g. `zenoh+udp://router.example.com:7447/...` (following the RFC 8323
`coap+tcp` precedent). An empty authority is a TD configuration error.

## Future Bindings

Future bindings should use the same core traits:

- HTTP
- CoAP
- MQTT
- Modbus TCP
- Modbus RTU
- BLE
- OPC UA
- Custom industrial protocols

Zenoh may also be used as a bridge or replacement transport for constrained or legacy environments when the deployment makes that appropriate. This is a platform choice, not an engine-level assumption.
