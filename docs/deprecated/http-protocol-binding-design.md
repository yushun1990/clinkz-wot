# HTTP Protocol Binding — Design Document

> Status: **Design draft v0.1**. No implementation code is proposed here. This
> document grounds every recommendation in the existing zenoh binding (the
> reference implementation under `protocol-bindings/protocols/zenoh/`) and the
> shared binding utilities under `protocol-bindings/core/`.

## 0. Reference Points

The zenoh binding (`clinkz-wot-protocol-bindings-zenoh`) is the reference for
crate layout, module split, feature flags, and trait surface. Key files:

| File | Role | HTTP analog |
|---|---|---|
| `protocols/zenoh/Cargo.toml` | Dependency + feature surface | `protocols/http/Cargo.toml` |
| `src/lib.rs` (53 lines) | Module wiring + `pub use` surface | `src/lib.rs` |
| `src/form.rs` (257 lines) | Form parsing, plan building | `src/form.rs` |
| `src/zenoh.rs` (395 lines) | `ZenohBindingTransport<T>` ClientBinding impl | `src/client.rs` |
| `src/runtime.rs` + `src/runtime/*` (16+431 lines) | Concrete transport adapters | `src/runtime.rs` |
| `src/server.rs` (1103 lines) | `ServerBinding` impl (sync fan-in) | `src/server.rs` |
| `src/protocol_binding.rs` (187 lines) | `ProtocolBinding` facade | `src/protocol_binding.rs` |
| `src/error.rs` (60 lines) | Binding-local error type | `src/error.rs` |

Shared utilities the HTTP binding will reuse without forking:

- `protocol-bindings/core/src/form.rs:121-331` — `select_affordance_form*`,
  `select_form_with_result_filter`, `validate_form_operation`,
  `resolve_form_target`.
- `protocol-bindings/core/src/error_status.rs:14-35` — `error_status()` maps
  `CoreError` to an HTTP-like status code. The mapping is already
  HTTP-aligned (404 / 400 / 401 / 403 / 406 / 500 / 501 / 502 / 504).
- `protocol-bindings/core/src/uri_template.rs` — RFC 6570 Level 1–3 expansion
  for `uriVariables` form hrefs.

Core trait surface (`clinkz-wot-core`):

- `binding_facade.rs:81-101` — `ProtocolBinding { protocol(), client_factory(), server() }`.
- `binding.rs:50-103` — `ClientBinding { supports, supports_with_thing, async invoke, async subscribe }` (gated `async`).
- `binding.rs:117-120` — `ClientBindingFactory::build()`.
- `inbound.rs:132-152` — `ServerBinding { configure, try_accept, send_response, register_thing, unregister_thing }`.
- `inbound.rs:164-169` — `Dispatch::serve_request(InboundRequest) -> InboundResponse` (async, direct-dispatch handle).

The Servient already provides the direct-dispatch path. `servient/src/builder.rs:86-94`
constructs `BindingContext { event_broker, fanin_sender: None, dispatch: Some(Arc<dyn Dispatch>) }`
and calls `configure(&ctx)` on every server binding. The zenoh `server.rs:481-483`
TODO confirms the fan-in path is legacy and direct dispatch is the canonical
model for new bindings.

---

## 1. Scope for v0.1

### 1.1 In scope

A two-direction `ProtocolBinding` over HTTP/1.1, std-only, tokio-driven:

- **Client side** (`ClientBinding`): outbound read / write / invoke over a
  shared HTTP client, plus streaming observe/subscribe over Server-Sent Events.
- **Server side** (`ServerBinding`): inbound route registration that maps each
  affordance form to an HTTP method + path, served by an HTTP server using the
  **direct-dispatch model** (no fan-in channel, no `try_accept`/`send_response`
  reply-target table).
- **Facade** (`HttpProtocolBinding`): single `ProtocolBinding` impl wrapping
  both, constructible in client-only, server-only, and shared-listener modes.

### 1.2 Operation coverage for v0.1

| Operation | v0.1 client | v0.1 server |
|---|---|---|
| `readproperty` | ✅ | ✅ |
| `writeproperty` | ✅ | ✅ |
| `invokeaction` | ✅ | ✅ |
| `queryaction`, `cancelaction` | defer | defer |
| `observeproperty` / `unobserveproperty` | SSE | SSE |
| `subscribeevent` / `unsubscribeevent` | SSE | SSE |
| `readallproperties` | GET on Thing form | GET on Thing form |
| `readmultipleproperties` | GET on Thing form | GET on Thing form |
| `writeallproperties` / `writemultipleproperties` | defer (compound payload) | defer |
| `observeallproperties` / `subscribeallevents` | defer | defer |

### 1.3 Explicitly deferred (post-v0.1)

- OAuth2 / OAuth1 / Digest / mTLS credential flows. v0.1 honors `NoSec` and
  `Bearer` only (mirroring the zenoh `AuthExpectation::Bearer` ceiling at
  `server.rs:84-93`); other schemes are accepted-but-not-extracted.
- WebSocket subprotocol (`subprotocol: "webhook"` / `"ws"`).
- Long-poll fallback for environments that block SSE.
- Multipart payloads, content negotiation beyond `Content-Type`, `Accept`
  handling, conditional requests, `ETag`/`If-Match` optimistic concurrency.
- HTTPS/TLS. v0.1 ships HTTP only; TLS arrives with a `tls` feature that
  pulls in `rustls` + a `rustls::ServerConfig`, gated off by default to keep
  the dependency footprint minimal.
- Chunked transfer encoding for streaming action outputs (action handlers
  that yield progressively).
- CORS preflight handling. Servient-level concern, not binding-local.

### 1.4 Why the server side is much smaller than zenoh's

Zenoh's `server.rs` is 1103 lines because zenoh callbacks are **synchronous**
(callback receives a `Query`/`Sample` that must be replied to via `.wait()`).
The binding maintains a reply-target table (`server.rs:235-254`), a TTL sweeper
(`server.rs:341-371`), a drop-oldest pending queue (`server.rs:725-755`), and
type-erased route undeclaration (`server.rs:175-211`).

HTTP handlers are `async fn`. The direct-dispatch path
(`Dispatch::serve_request(req).await` → `InboundResponse`) collapses all of
that into a single request-scoped future. No reply-target table, no TTL sweep,
no bounded queue — the HTTP server's own connection pool provides backpressure.
Expected `server.rs` size: **~250–350 lines** including route planning.

---

## 2. Crate Structure

### 2.1 Directory layout

```
protocol-bindings/protocols/http/
├── Cargo.toml
├── src/
│   ├── lib.rs              # module wiring + pub use surface
│   ├── error.rs            # HttpBindingError, HttpBindingResult
│   ├── form.rs             # scheme detection, plan building, op→method mapping
│   ├── client.rs           # HttpClientBinding: ClientBinding impl + plan cache
│   ├── server.rs           # HttpServerBinding: ServerBinding impl (direct dispatch)
│   ├── protocol_binding.rs # HttpProtocolBinding facade + factory
│   └── runtime.rs          # concrete hyper client + server adapters
└── tests/
    ├── http_form_test.rs                # form parsing unit tests
    ├── http_protocol_binding_test.rs    # facade construction
    └── http_round_trip_smoke_test.rs    # opt-in client↔server end-to-end
```

Workspace `Cargo.toml` gains one entry in the `members` array:

```toml
members = [
    # ...
    "protocol-bindings/protocols/http",
    # ...
]
```

### 2.2 Why a flat split, not zenoh's nested `runtime/`

Zenoh splits `runtime/` into `zenoh.rs`, `zenoh_pico.rs`, `selector.rs`,
`sample.rs`, `metadata.rs` because it has **two mutually exclusive concrete
backends** (std Rust zenoh vs constrained zenoh-pico) plus shared metadata
parsing. HTTP v0.1 has a single concrete backend (hyper), so a single
`runtime.rs` is sufficient. If a `no_std` HTTP-over-something backend ever
becomes viable (it is not — see §3.2), it can be split out then following
zenoh's `runtime/<backend>.rs` pattern.

### 2.3 Module responsibilities (mirrors zenoh)

| Module | Mirrors zenoh's | Owns |
|---|---|---|
| `form.rs` | `zenoh/src/form.rs` | `HTTP_SCHEME`/`HTTPS_SCHEME` constants, `is_http_form(_target)`, `extract_http_target`, `HttpOperationPlan` (method + resolved URL), `plan_http_operation`, `http_method_for_operation` |
| `client.rs` | `zenoh/src/zenoh.rs` | `HttpClientBinding<T>` generic over a `HttpTransport` adapter, per-form plan cache keyed by `Arc<Form>` address (same pattern as `zenoh/src/zenoh.rs:77-261`) |
| `server.rs` | `zenoh/src/server.rs` | `HttpServerBinding` — direct-dispatch via `ctx.dispatch.serve_request(req).await`; `register_thing`/`unregister_thing` build/dismantle route tables; no reply-target table |
| `runtime.rs` | `zenoh/src/runtime/zenoh.rs` | `HyperTransport` (the concrete `HttpTransport` impl) — owns a `hyper::client::Client` and a `hyper::server::Connection` driver |
| `protocol_binding.rs` | `zenoh/src/protocol_binding.rs` | `HttpProtocolBinding` facade + `HttpClientFactory` |
| `error.rs` | `zenoh/src/error.rs` | `HttpBindingError` wrapping `BindingError` plus HTTP-specific variants |

### 2.4 Crate root module gating

Mirror `zenoh/src/lib.rs:1-26`:

```rust
#![no_std]  // CRITICAL: see §3.2 — this becomes std-only when `http` is on.

#[cfg(feature = "http")]
extern crate std;
extern crate alloc;

mod error;
mod form;
#[cfg(feature = "http")]
mod client;
#[cfg(feature = "http")]
mod server;
#[cfg(feature = "http")]
mod protocol_binding;
#[cfg(feature = "http")]
mod runtime;
```

The crate keeps `form.rs` and `error.rs` (the protocol-planning surface)
available under `no_std + alloc` even with no backend selected — this
preserves the property that "TD form parsing is no_std-capable" which
zenoh has at `zenoh/src/lib.rs:18-19`. The actual client/server impls
require `feature = "http"`.

---

## 3. Dependencies

### 3.1 Recommended HTTP stack

| Crate | Role | Why |
|---|---|---|
| `hyper` (1.x) | Both client and server HTTP/1.1 | Single dependency, no wrapper layers. `hyper::server::conn::http1` + `hyper::client::conn::http1` cover the v0.1 surface. |
| `http-body-util` | `Full<Bytes>` / `Empty<Bytes>` body helpers | hyper 1.x requires explicit body types; this is the canonical helper crate. |
| `bytes` | `Bytes` body buffers | Already a transitive hyper dep; declared explicitly because we construct `Bytes` directly. |
| `http` | `Request`/`Response`/`Method`/`StatusCode` types | Decouples request/response types from hyper the same way hyper itself does. |
| `tokio` | Async runtime (already in workspace via zenoh) | hyper 1.x is runtime-agnostic but needs a TCP adapter; `tokio::net::TcpListener` is the conventional choice. |

**Explicitly not added for v0.1:**

- `reqwest` — wraps hyper + adds a higher-level API + its own connection pool.
  Not needed; `hyper::client::Client` is enough and keeps the dep surface
  minimal. Reconsider if/when cookie jars, redirect policy, or multipart
  support are needed.
- `axum` — wraps hyper with a tower-style routing layer. Tempting for the
  server side because it handles path-parameter extraction (`/things/{id}/properties/{name}`),
  but it pulls `tower`, `tower-http`, `matchit`, etc. v0.1 routes are a flat
  `BTreeMap<route_key, RouteMeta>` lookup (one entry per affordance form);
  manual routing is one match arm, not worth ~15 transitive crates.
- `rustls` / `native-tls` — TLS is deferred (see §1.3).

### 3.2 `no_std` posture

**The HTTP binding is std-only.** HTTP requires:

- TCP sockets (`tokio::net::TcpListener`, `tokio::net::TcpStream`).
- TLS for any realistic deployment (post-v0.1).
- A long-lived async runtime.

None of these exist on `no_std + alloc`. This is a hard constraint, not a
deferred feature. The crate's planning surface (`form.rs`, `error.rs`) stays
`no_std + alloc` compatible for symmetry with zenoh's planning surface, but
the binding cannot be used on bare-metal targets.

This is consistent with AGENTS.md's policy: *"Put host/cloud runtime
functionality behind `std` features or in separate `std` crates."* The crate
root stays `#![no_std]` and the `http` feature pulls `std` in (matching
zenoh's pattern at `zenoh/src/lib.rs:1, 14`).

### 3.3 Interaction with tokio

Zenoh's `Cargo.toml:13` already pulls `tokio = { version = "1", features =
["sync", "macros", "rt", "rt-multi-thread", "time"] }`. The HTTP crate needs
the same plus `tokio/net`:

```toml
tokio = { version = "1", optional = true, features = ["sync", "macros", "rt", "rt-multi-thread", "net", "time"] }
```

No new runtime is introduced. The HTTP binding's `async fn invoke` runs on
the same tokio runtime the Servient was launched on. `HttpServerBinding::configure`
captures `Arc<dyn Dispatch>` from `BindingContext` and the server task is
spawned lazily on the first `register_thing` (so a pure-consumer Servient
that never calls `expose()` does not bind a port).

### 3.4 Feature flags

Mirror zenoh's feature shape (`zenoh/Cargo.toml:19-40`):

```toml
[features]
default = ["http"]
http = [
    "async",
    "clinkz-wot-core/std",
    "clinkz-wot-protocol-bindings/std",
    "clinkz-wot-td/std",
    "serde_json/std",
    "dep:hyper",
    "dep:http-body-util",
    "dep:bytes",
    "dep:http",
    "dep:tokio",
]
async = [
    "dep:async-trait",
    "dep:tokio",
    "clinkz-wot-core/async",
]
```

The `http` feature is the analog of zenoh's `zenoh` feature. There is no
equivalent of `zenoh-pico` because no constrained HTTP backend is planned.

---

## 4. TD Form Parsing

### 4.1 What HTTP forms look like

Standard W3C TD 1.1 HTTP binding vocabulary (W3C WoT Binding Templates — HTTP
Core vocabulary) uses `http://` or `https://` schemes. The operation-to-method
mapping is **conventional** — there is no required extension vocabulary for
the core mapping. Example form from a compliant TD:

```json
{
  "base": "https://example.com/things/lamp/",
  "href": "properties/status",
  "op": ["readproperty", "writeproperty"],
  "contentType": "application/json"
}
```

The resolved target (`https://example.com/things/lamp/properties/status`) +
the operation (`readproperty` or `writeproperty`) is sufficient to derive
`GET` or `PUT`. No extension terms required.

### 4.2 Recommendation: no `cz-http:` vocabulary for v0.1

Unlike zenoh (which needs `cz-zenoh:qos`, `cz-zenoh:priority`,
`cz-zenoh:congestionControl` because zenoh has no native TD expression for
those transport hints — see `zenoh/src/form.rs:16-21`), HTTP has no transport
metadata that lacks a standard expression. Specifically:

- Method → derivable from `op` (see §5).
- Content type → standard `contentType` field.
- Body encoding → standard `contentCoding` field.
- URI variables → standard `uriVariables` on the affordance + RFC 6570 href
  templates (already handled by `protocol-bindings/core/src/uri_template.rs`).
- Caching, conditional requests, idempotency keys → out of scope for v0.1.

**If** a future phase needs method overrides (e.g. a server that accepts
`writeproperty` via `POST` instead of `PUT`, or a WebDAV-style binding), a
`cz-http:method` extension term should be introduced at that point —
following the same shape as `cz-zenoh:*` (string-valued, optional, validated
by `extension_string` — see `zenoh/src/form.rs:244-257` for the canonical
helper). **Defer all of this past v0.1.**

### 4.3 Form parsing API surface

The HTTP `form.rs` exposes the same shape as `zenoh/src/form.rs:81-219`:

```rust
pub const HTTP_SCHEME: &str  = "http://";
pub const HTTPS_SCHEME: &str = "https://";

pub fn is_http_form(form: &Form) -> bool;
pub fn is_http_form_target(thing: &Thing, form: &Form) -> bool;
pub fn try_extract_http_target(thing: &Thing, form: &Form)
    -> HttpBindingResult<Option<HttpFormTarget>>;
pub fn extract_http_target(thing: &Thing, form: &Form)
    -> HttpBindingResult<HttpFormTarget>;
pub fn plan_http_operation(thing: &Thing, form: &Form, operation: Operation)
    -> HttpBindingResult<HttpOperationPlan>;
pub fn plan_http_affordance_operation<'a>(...) -> HttpBindingResult<HttpAffordanceOperationPlan<'a>>;
pub fn http_method_for_operation(operation: Operation) -> HttpMethod;  // see §5
```

`HttpFormTarget` is just the resolved URL string plus the parsed
`http::Uri`. `HttpOperationPlan` carries `{ url, method, content_type }`
— the minimum a transport needs to issue the request.

The `try_extract_http_target` helper mirrors
`zenoh/src/form.rs:100-112` exactly: resolve the form target once
(via `resolve_form_target` from `protocol-bindings/core/src/form.rs:201-205`),
check the scheme, and either return the parsed target or `None`.

---

## 5. Operation Mapping

### 5.1 WoT operation → HTTP method

The mapping follows W3C WoT Profile §6.2 (HTTP binding defaults). Implemented
as `http_method_for_operation(Operation) -> http::Method` in `form.rs`:

| WoT operation | HTTP method | Notes |
|---|---|---|
| `readproperty` | `GET` | Body-less request, response carries the property value. |
| `writeproperty` | `PUT` | Idempotent overwrite; `POST` is allowed by the spec for non-idempotent writes, but v0.1 always uses `PUT`. |
| `invokeaction` | `POST` | Action input in request body, action output in response body. `202 Accepted` for long-running actions is a v0.2 concern. |
| `queryaction` | `GET` | On the action's own form (deferred). |
| `cancelaction` | `DELETE` | On the action's own form (deferred). |
| `observeproperty` / `subscribeevent` | `GET` with `Accept: text/event-stream` | SSE. See §5.2. |
| `unobserveproperty` / `unsubscribeevent` | (client closes the SSE stream) | No HTTP request is issued — the client half-closes the connection. The server's SSE handler drop-cancels the broker subscription. |
| `readallproperties` / `readmultipleproperties` | `GET` | On the Thing-level form (`AffordanceTarget::Thing`). `readmultipleproperties` carries the property names as a query string. |
| `writeallproperties` / `writemultipleproperties` | `PUT` (deferred) | Compound JSON object body. |
| `observeallproperties` / `subscribeallevents` | `GET` with `Accept: text/event-stream` (deferred) | Single SSE stream multiplexing all events/properties. |

This mirrors `zenoh/src/form.rs:201-219` (`zenoh_operation_kind`) in shape:
one match arm per operation, returning a typed transport primitive
(`http::Method` instead of `ZenohOperationKind`).

### 5.2 Observe/subscribe: SSE, not WebSocket, not long-poll

**SSE is the v0.1 streaming mechanism.** Rationale:

- Unidirectional (server → client), which matches `observeproperty` /
  `subscribeevent` semantics exactly. WebSocket is bidirectional and
  over-engineered for this.
- Plain HTTP/1.1 with `Content-Type: text/event-stream`; no upgrade dance.
- Native `EventSource` browser support (matters when a future JS consumer
  calls into a Clinkz Thing).
- Maps cleanly onto the `EventBroker` + `PublisherSink` model already in
  the engine — the same path zenoh uses at `server.rs:580-596`, just with
  an SSE writer as the sink instead of `session.put`.
- TD 1.1 has a standard `subprotocol: "sse"` value (W3C WoT Binding
  Templates). Forms that declare `subprotocol: "sse"` are preferred for
  observe/subscribe; forms without `subprotocol` but with an HTTP scheme
  and an observe/subscribe op are also accepted in v0.1.

Long-poll is deferred (see §1.3). It would be modeled as a separate
`HttpSubscriptionMode::LongPoll` variant if it ever returns; for v0.1
there is one mode: `Sse`.

### 5.3 Reusing shared form selection

The HTTP binding MUST reuse `protocol-bindings/core/src/form.rs` rather than
re-implementing selection. Concretely:

- `ClientBinding::invoke` calls `validate_form_operation`
  (`form.rs:355-371`) to check the selected form supports the requested
  operation, exactly as `zenoh/src/zenoh.rs:168-181` does.
- The form's `subprotocol: "sse"` is matched via `FormSelectionCriteria::subprotocol`
  (`form.rs:52-53`) for observe/subscribe operations. This is the
  protocol-neutral hook the shared selector provides for exactly this case.
- Target resolution uses `resolve_form_target` (`form.rs:201-205`) — no
  HTTP-local URL resolution code.

---

## 6. ServerBinding Implementation

### 6.1 The decision: direct dispatch

Two server-driving models are described in `core/src/inbound.rs:106-169`:

1. **Fan-in channel** (`try_accept` + `send_response`): the binding pushes
   `InboundRequest`s onto a bounded channel; the Servient drains them; the
   Servient returns `InboundResponse`s via `send_response`, which the binding
   matches back to its transport via the echoed `CorrelationId`.
2. **Direct dispatch** (`Dispatch::serve_request`): the binding calls
   `ctx.dispatch.serve_request(req).await` from inside its async handler and
   receives the `InboundResponse` directly. No channel, no correlation
   matching.

**Recommendation: direct dispatch.** Reasons:

- HTTP handlers are already async (`async fn` in hyper). The fan-in model
  exists for protocols whose callbacks are **synchronous** and cannot
  `.await` (zenoh's `.callback(move |query| ...)` at `server.rs:509` cannot
  `await`). HTTP has no such constraint.
- The fan-in model requires a reply-target table (`zenoh/src/server.rs:219-266`),
  a TTL sweeper for abandoned entries (`server.rs:341-371`), and a bounded
  queue with drop-oldest backpressure (`server.rs:725-755`). All of that
  disappears with direct dispatch: the request lifetime is bounded by the
  HTTP connection's own lifetime, and hyper's connection pool provides
  backpressure.
- The Servient already provides `BindingContext { dispatch: Some(Arc<dyn Dispatch>), fanin_sender: None }`
  (`servient/src/builder.rs:87-91`). The direct-dispatch path is the one
  the Servient actually wires up today.

### 6.2 `HttpServerBinding` shape

```rust
pub struct HttpServerBinding {
    inner: Arc<ServerShared>,
}

struct ServerShared {
    /// Routes registered per Thing: thing_id → (route_key → RouteMeta).
    /// Mirrors zenoh's `routes: BTreeMap<ThingId, ThingRoutes>`
    /// (zenoh/src/server.rs:283) but the values are plain metadata, not
    /// declared zenoh primitives.
    routes: Mutex<BTreeMap<ThingId, BTreeMap<String, RouteMeta>>>,
    /// Injected by `configure(&BindingContext)` — see §6.3.
    dispatch: Mutex<Option<Arc<dyn Dispatch>>>,
    event_broker: Mutex<Option<EventBroker>>,
    /// The configured listen address. The TCP listener is bound lazily on
    /// first register_thing, so a pure-consumer Servient never opens a port.
    addr: SocketAddr,
    /// The bound listener, guarded so register_thing/unregister_thing can
    /// check whether the server task needs starting.
    listener: tokio::sync::Mutex<Option<tokio::net::TcpListener>>,
    /// A shutdown signal sent when the binding is dropped or via a
    /// shutdown() method. Matches the graceful-shutdown expectation in
    /// user-facing-api.md §5.2.
    shutdown: tokio::sync::Notify,
}
```

`RouteMeta` carries the `thing_id`, `AffordanceTarget`, `Operation`, and
the `AuthExpectation` (same enum shape as zenoh's
`server.rs:84-93` — `None` / `Bearer` / `Unsupported`). It does **not**
carry a hyper request handle or a reply target.

### 6.3 `configure` captures the dispatch handle

```rust
fn configure(&self, ctx: &BindingContext) {
    #[cfg(feature = "async")]
    {
        if let Some(dispatch) = ctx.dispatch.clone() {
            *self.inner.dispatch.lock().unwrap() = Some(dispatch);
        }
    }
    if let Ok(mut broker) = self.inner.event_broker.lock() {
        *broker = Some(ctx.event_broker.clone());
    }
}
```

This replaces the zenoh `server.rs:477-483` TODO — instead of wiring a
fan-in sender, we capture the dispatch handle directly.

### 6.4 `register_thing` plans routes without declaring them

`register_thing` walks every HTTP-targeting affordance form (same iteration
shape as `zenoh/src/server.rs:981-1056`'s `iter_zenoh_affordance_forms`),
builds a `RouteMeta` per `(target, operation, form)` tuple, and stores them
in the route map. No socket declaration happens here — that is zenoh-specific
(`declare_queryable` / `declare_subscriber`).

The server task (one per binding, started lazily) owns the
`tokio::net::TcpListener` and accept loop. Each accepted connection runs an
async handler that:

1. Parses the `Request<IncomingBody>`.
2. Looks up the matching `RouteMeta` by `(method, path)`.
3. Extracts the body, content type, and bearer token (if `AuthExpectation::Bearer`).
4. Builds an `InboundRequest` with a synthesized `CorrelationId` (any
   monotonically-unique token — it is local to the handler's future).
5. Calls `dispatch.serve_request(req).await`.
6. Converts the returned `InboundResponse` to an HTTP response:
   - On `error: Some(e)`: status from `error_status(&e)` (already HTTP-shaped),
     body = `e.to_string()`.
   - On success: `200 OK` (or `201 Created` / `202 Accepted` from
     `InteractionStatus` once that field propagates — currently `InteractionOutput::status`
     is in `user-facing-api.md §9.1` but not yet in `core/src/interaction.rs`;
     treat as v0.2).
   - Content-Type from `output.data.content_type`.

For SSE: the handler parses `Accept: text/event-stream`, opens a
`tokio::sync::broadcast::Receiver` against the `EventBroker`, and writes
`data: <base64-payload>\n\n` frames as payloads arrive. The future ends
when the client half-closes (hyper returns `Err` on the body write) or the
broker drops the subscription (Thing destroyed). This replaces zenoh's
`PublisherSink` registration at `server.rs:580-596` with an SSE writer
registered the same way against `EventBroker::register`.

### 6.5 `try_accept` / `send_response` are no-ops

`try_accept` returns `None` unconditionally. `send_response` is a no-op (or
panics in debug builds). The direct-dispatch path never invokes them. This
is the contract `core/src/inbound.rs:139-145` permits — `try_accept` has a
default `None` impl, and `send_response` exists only for the fan-in model.

### 6.6 `unregister_thing` removes the route map entry

Same shape as `zenoh/src/server.rs:458-475` minus the `undeclare_routes`
call. There is nothing to undeclare — the server task routes by the map,
so removing the map entry is enough. In-flight handlers for that Thing
return `404`/`503` on their next map lookup (the `InboundRequest` is built
after the lookup; if the entry is gone, the handler returns `404 Not Found`
without dispatching).

---

## 7. ClientBinding Implementation

### 7.1 `HttpClientBinding<T>` generic over a transport adapter

Mirror `zenoh/src/zenoh.rs:77-200` exactly:

```rust
pub struct HttpClientBinding<T> {
    supported_operations: u32,
    transport: T,
    plan_cache: WotLock<BTreeMap<PlanCacheKey, PlanCacheEntry>>,
}
```

`T: HttpTransport`. `HttpTransport` is the analog of `ZenohTransport`
(`zenoh/src/zenoh.rs:38-56`):

```rust
#[async_trait::async_trait]
pub trait HttpTransport: Send + Sync {
    /// Executes a one-shot HTTP request and returns the response body.
    async fn execute(&self, request: HttpTransportRequest)
        -> CoreResult<InteractionOutput>;

    /// Opens a long-lived SSE subscription.
    async fn open_subscription(
        &self,
        request: HttpTransportRequest,
    ) -> CoreResult<(Subscription, Box<dyn SubscriptionGuard>)>;
}
```

`HttpTransportRequest` carries `plan: Arc<HttpOperationPlan>` plus
`payload: Option<Payload>` plus `uri_variables: BTreeMap<String, String>`
— same shape as `ZenohTransportRequest` at `zenoh/src/zenoh.rs:20-30`.

### 7.2 The plan cache

Reuse the exact pattern from `zenoh/src/zenoh.rs:211-261`:

- Keyed by `(Arc<Form> pointer, Operation)`.
- Entry holds a `Weak<Form>` so dropped forms are pruned.
- Read lock on hit, write lock on miss.
- Dead-entry pruning on miss (`prune_dead_plan_cache_entries`).

This is the engine's amortization strategy for repeated consumer
interactions (PLAN.md §"Performance Hardening": *"Outbound form/binding
plan interned in the consumed registry entry"*). The cache is per-binding
(per-Consumed-Thing), matching `ClientBindingFactory::build` producing a
fresh binding per consume.

### 7.3 `ClientBinding::invoke` flow

```text
1. validate_form_operation(thing, affordance_ref(target), form, op)
   → on error: map BindingError → CoreError (already implemented in
     protocol-bindings/core/src/error.rs:90-104) and return.
2. plan_for(thing, form, op)
   → cache hit: reuse Arc<HttpOperationPlan>.
   → cache miss: plan_http_operation(thing, form, op), insert.
3. Expand uriVariables into the URL using expand_uri_template
   (protocol-bindings/core/src/uri_template.rs).
4. transport.execute(HttpTransportRequest { plan, payload: input.data, uri_variables })
   → returns InteractionOutput.
```

Identical control flow to `zenoh/src/zenoh.rs:168-181`.

### 7.4 `ClientBinding::subscribe` flow

Same as invoke but calls `transport.open_subscription(...)`. The transport's
SSE implementation:

1. Issues `GET <url>` with `Accept: text/event-stream`.
2. Spawns a task that parses `data: ...\n\n` frames and pushes the decoded
   `Payload` into a `Subscription::channel(0)` sender (same channel
   mechanism as `zenoh/src/runtime/zenoh.rs:162-178`).
3. Returns a `SubscriptionGuard` whose `close()` aborts the task and drops
   the connection.

### 7.5 Content-Type handling

- Outbound: `request.input.data.content_type` becomes the `Content-Type`
  header for `PUT`/`POST`. `GET` has no body, so `Content-Type` is omitted.
- Inbound (response → `InteractionOutput`): the response's `Content-Type`
  header populates `InteractionOutput.data.content_type`. If the server
  omits the header, fall back to the form's declared `contentType` (this
  matches what zenoh does via `content_type_hint` at
  `zenoh/src/runtime/zenoh.rs:163`).

### 7.6 Error status mapping (inbound side)

`protocol-bindings/core/src/error_status.rs:14-35` already maps every
`CoreError` variant to an HTTP-shaped status code. The HTTP client uses
this in reverse:

- HTTP status `2xx` → success, body becomes `InteractionOutput.data`.
- HTTP status `4xx`/`5xx` → `CoreError::Transport(format!("HTTP {status}: {body}"))`
  for v0.1. A more granular reverse mapping (`404 → UnknownAffordance`,
  `401 → Security(MissingCredentials)`, etc.) is deferred — the lossy
  `Transport` variant preserves the status text for callers.

The server side uses `error_status()` directly: each `InboundResponse.error`
becomes `StatusCode::from_u16(error_status(&e)).unwrap_or(500)`.

---

## 8. ProtocolBinding Facade

### 8.1 The analog of `ZenohProtocolBinding::shared`

Zenoh's canonical constructor (`zenoh/src/protocol_binding.rs:78-82`) is:

```rust
pub fn shared(session: zenoh::Session) -> Arc<dyn ProtocolBinding> {
    let transport = ZenohRuntimeTransport::new(session.clone());
    let server = Arc::new(ZenohServerBinding::new(session));
    Arc::new(Self::new(transport).with_server(server))
}
```

The HTTP analog needs an **address**, not a session, because the HTTP
server has to bind a TCP port. The client side has no equivalent of a
`Session` to share — a `hyper::client::Client` is cheap to clone and
internally pooled, so it is constructed inside the factory:

```rust
impl HttpProtocolBinding {
    /// Canonical shared-listener constructor. Analogous to
    /// `ZenohProtocolBinding::shared(session)`.
    pub fn bind(addr: impl Into<SocketAddr>) -> std::io::Result<Arc<dyn ProtocolBinding>> {
        let addr = addr.into();
        let transport = HyperTransport::new();         // builds the hyper client
        let server = Arc::new(HttpServerBinding::new(addr));
        Ok(Arc::new(Self::new(transport).with_server(server)))
    }

    /// Client-only facade (cloud controller topology).
    pub fn client() -> Arc<dyn ProtocolBinding> {
        let transport = HyperTransport::new();
        Arc::new(Self::new(transport))
    }

    /// Server-only facade (sensor topology).
    pub fn server(addr: impl Into<SocketAddr>) -> std::io::Result<Arc<dyn ProtocolBinding>> {
        let server = Arc::new(HttpServerBinding::new(addr.into()));
        Ok(Arc::new(Self::with_server_only(server)))
    }
}
```

### 8.2 The three topologies

This matches the three usage patterns documented in
`user-facing-api.md §5.3` and the test patterns in
`zenoh/tests/zenoh_protocol_binding_test.rs`:

| Topology | Constructor | Use case |
|---|---|---|
| Two-direction | `HttpProtocolBinding::bind(addr)` | Default: a Thing that both consumes remote Things and exposes local Things. |
| Client-only | `HttpProtocolBinding::client()` | Cloud controller that never exposes local Things. No port bound. |
| Server-only | `HttpProtocolBinding::server(addr)` | Sensor that never consumes remote Things. No HTTP client constructed. |

### 8.3 Facade trait impl

```rust
impl ProtocolBinding for HttpProtocolBinding {
    fn protocol(&self) -> ProtocolId { ProtocolId("http") }

    fn client_factory(&self) -> Option<Box<dyn ClientBindingFactory>> {
        // Always present in two-direction and client-only modes.
        // Absent in server-only mode.
        self.client_transport.as_ref()
            .map(|t| Box::new(HttpClientFactory { transport: t.clone() }) as Box<dyn ClientBindingFactory>)
    }

    fn server(&self) -> Option<Arc<dyn ServerBinding>> {
        self.server.as_ref()
            .map(|s| s.clone() as Arc<dyn ServerBinding>)
    }
}
```

Shape identical to `zenoh/src/protocol_binding.rs:98-116`.

### 8.4 `HttpClientFactory`

Mirror `zenoh/src/protocol_binding.rs:124-135`:

```rust
#[derive(Clone)]
struct HttpClientFactory {
    transport: HyperTransport,
}

impl ClientBindingFactory for HttpClientFactory {
    fn build(&self) -> Box<dyn ClientBinding> {
        Box::new(HttpClientBinding::with_transport(self.transport.clone()))
    }
}
```

`HyperTransport` is `Clone` (its `hyper::client::Client` is internally
`Arc`-wrapped, same as `zenoh::Session`).

---

## 9. Effort Estimate

### 9.1 Lines of code

Based on the zenoh binding's actual sizes
(`wc -l protocol-bindings/protocols/zenoh/src/*.rs`):

| Module | Zenoh size | HTTP estimate | Rationale |
|---|---|---|---|
| `lib.rs` | 53 | ~40 | Fewer re-exports (no pico variant). |
| `error.rs` | 60 | ~50 | Same shape. |
| `form.rs` | 257 | ~200 | No `cz-http:` extension terms to parse; method mapping is one match. |
| `client.rs` (zenoh: `zenoh.rs`) | 395 | ~350 | Same plan cache, same invoke/subscribe shape; SSE frame parsing adds ~50 lines. |
| `server.rs` | **1103** | **~300** | Direct dispatch eliminates reply-target table, TTL sweep, drop-oldest queue, type-erased route handles, fan-in channel. Route planning + handler is the bulk. |
| `protocol_binding.rs` | 187 | ~180 | Same shape, three constructors instead of one. |
| `runtime.rs` (zenoh: `runtime/zenoh.rs`) | 431 | ~250 | hyper client + server driver; no `Shared*` wrapper needed (hyper client is already `Arc`-shared). |
| **Total src/** | **~2526** | **~1370** | ~46% of zenoh, mostly because of the direct-dispatch simplification. |

Tests add roughly 30–50% on top, matching zenoh's ratio.

### 9.2 Files

8 source files (mirrors zenoh's 8, minus the `runtime/` subdirectory which
collapses to a single `runtime.rs`). 3–4 test files (form, protocol binding,
round-trip smoke gated on `CLINKZ_WOT_RUN_HTTP_TESTS=1`, mirroring zenoh's
`CLINKZ_WOT_RUN_ZENOH_RUNTIME_TESTS`).

### 9.3 Phasing

**Phase A — Client-only (≈ 60% of the work, lands first):**
1. `Cargo.toml` skeleton + `lib.rs` module wiring + `error.rs`.
2. `form.rs`: scheme detection, method mapping, plan building, unit tests.
3. `runtime.rs`: `HyperTransport` with `execute()` only (no SSE yet).
4. `client.rs`: `HttpClientBinding<T>` + plan cache + `invoke` impl.
5. `protocol_binding.rs`: `HttpProtocolBinding::client()` facade.
6. Smoke test: client against a known HTTP endpoint (or a hyper test server
   spun up in-process).

**Phase B — Server (≈ 30%, lands second):**
7. `server.rs`: route map, `register_thing`/`unregister_thing`, accept loop.
8. `runtime.rs`: server task using `hyper::server::conn::http1`.
9. `protocol_binding.rs`: `bind(addr)` and `server(addr)` constructors.
10. Round-trip smoke test: one process exposes a Thing, another (same
    process, fresh binding) consumes it and round-trips a read/write/invoke.

**Phase C — Streaming (≈ 10%, lands last):**
11. `runtime.rs`: SSE frame parser + SSE response writer.
12. `client.rs`: `subscribe` impl.
13. `server.rs`: SSE handler that registers a sink with `EventBroker`.
14. SSE round-trip smoke test for `observeproperty` and `subscribeevent`.

This phasing lets the binding become useful (consume any HTTP Thing) at
the end of Phase A, before the server exists. Phase B unlocks exposing
Things; Phase C unlocks observability.

### 9.4 Risks

| Risk | Mitigation |
|---|---|
| hyper 1.x API churn between minor versions | Pin hyper at a specific 1.x minor in `Cargo.toml`; test against that version. |
| SSE parser correctness (line buffering, `retry:` field, multi-line `data:`) | Use the `eventsource-stream` crate or hand-roll against the WHATWG spec; cover with fixture tests before the smoke test. |
| Route matching for `uriVariables` (e.g. `/things/{id}/properties/{name}`) | Use `protocol-bindings/core/src/uri_template.rs` for *outbound* expansion. For *inbound* matching, the v0.1 server matches on the **resolved href template** with simple `{var}` substitution against incoming paths — defer full RFC 6570 inbound parsing until needed. |
| `InteractionOutput::status` not yet plumbed through core | v0.1 always returns `200 OK` on success and `error_status(&e)` on failure; the `Created`/`Accepted` distinction arrives when `InteractionStatus` propagates from `user-facing-api.md §9.1` into `core/src/interaction.rs`. |
| Plan-cache memory growth under high affordance churn | Same `Weak<Form>` pruning as zenoh (`zenoh/src/zenoh.rs:263-265`); add a size cap in v0.2 if profiling warrants. |

---

## 10. Open Questions (non-blocking for v0.1)

1. Should the HTTP binding publish a Thing's TD at a well-known path
   (e.g. `/.well-known/wot`)? Likely yes for v0.2 — needed for HTTP-based
   discovery — but discovery is out of scope for v0.1.
2. `InteractionStatus::Accepted` for long-running actions: surface via
   `202 Accepted` + `Location` header for action status polling? Defer to
   when the action lifecycle (query/cancel) is in scope.
3. Should `bind(addr)` start the listener eagerly (returning
   `io::Result`) or lazily on first `register_thing`? **Recommendation:
   eagerly**, so a port-binding failure surfaces at construction time
   rather than at first expose.
4. Multiple HTTP bindings on different ports (e.g. one HTTP, one HTTPS):
   supported naturally by constructing two `HttpProtocolBinding` values
   and registering both via `with_protocol_binding`. Confirm with a
   multi-binding integration test in Phase B.
5. Whether `HttpProtocolBinding::bind` should accept a `hyper::server::conn::http1::Builder`
   for tuning (max headers, keep-alive timeout). Defer — defaults are
   fine for v0.1.
