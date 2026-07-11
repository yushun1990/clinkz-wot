# clinkz-wot Design

Status: current design reference for v4.1.

This document is the authoritative project design for `clinkz-wot`. Previous
architecture baselines, implementation plans, audit notes, and target documents
are historical references under `docs/deprecated/`.

## Purpose

`clinkz-wot` is a protocol-neutral Rust Web of Things engine for the Clinkz
platform. It uses W3C WoT Thing Descriptions as the semantic contract and keeps
transport behavior in optional protocol binding crates.

The engine targets:

- W3C WoT Thing Description 1.1 by default.
- W3C WoT Scripting API method semantics for Consumer, Producer, and Discovery
  user-agent flows.
- `no_std + alloc` support for data models, validation, interaction core, and
  embedded dispatch paths.
- `std` host support for runtime conveniences, storage adapters, and concrete
  network backends.

TD 2.0 work is experimental and must remain behind `td2-preview`.

## Design Principles

1. The engine is protocol-neutral. TD/TM, core runtime abstractions, discovery,
   and servient composition must not depend on zenoh-specific behavior.
2. W3C WoT vocabulary and Clinkz extensions are separate. Clinkz-specific
   binding, storage, compute, or platform metadata uses the Clinkz JSON-LD
   namespace, currently represented with the `cz:` prefix.
3. TD and TM crates own data models, builders, serialization,
   deserialization, validation, and round-trip preservation.
4. Protocol behavior belongs in binding crates.
5. Discovery and Servient/runtime behavior belong in dedicated crates.
6. `no_std + alloc` is a first-class contract where a crate responsibility
   permits it. Filesystems, sockets, threads, async runtimes, and process APIs
   stay behind `std` features or concrete runtime crates.
7. Unknown TD/TM extension fields are preserved through deserialize/serialize
   round trips unless an explicit validation mode rejects them.
8. `base` plus relative form `href` values are supported through shared form
   target resolution helpers.

## Workspace Crates

| Crate | Path | Role | `no_std + alloc` posture |
| --- | --- | --- | --- |
| `clinkz-wot-td` | `td` | TD/TM data models, builders, serde, validation, URI helpers. | Root crate supports it. |
| `clinkz-wot-core` | `core` | Interaction core, handlers, locks, payloads, security, inbound/outbound binding traits. | Root crate supports it. |
| `clinkz-wot-discovery` | `discovery` | WoT Discovery introduction, exploration, directory sessions, publisher/watch traits, in-memory backend. | Data model supports it; async traits behind `async`; storage under `std`. |
| `clinkz-wot-protocol-bindings` | `protocol-bindings/core` | Shared protocol-neutral binding helpers: form selection, op resolution, security resolution, URI templates. | Root crate supports it. |
| `clinkz-wot-protocol-bindings-zenoh` | `protocol-bindings/protocols/zenoh` | Optional zenoh planning and runtime binding. | Planning layer is protocol crate code; Rust zenoh backend is `std`; `zenoh-pico` is reserved for constrained runtime work. |
| `clinkz-wot-servient` | `servient` | Application-facing Servient composition, produced/consumed handles, dispatch, discovery facade. | Root registry primitives compile without `std`; full Servient surface is async-first. |
| `clinkz-wot-codec-cbor` | `codecs/cbor` | CBOR payload codec. | Root crate supports it. |
| `clinkz-wot` | `clinkz-wot` | Umbrella crate that re-exports the application-facing API. | Feature-composed. |

## Feature Policy

The main feature groups are:

- `std`: host conveniences and standard-library integration.
- `async`: native async trait surface without implying an executor.
- `zenoh`: concrete Rust zenoh backend; this is a `std` runtime feature.
- `zenoh-pico`: reserved constrained backend surface.
- `td2-preview`: experimental TD 2.0 data-model additions.
- `cbor`: optional CBOR codec in the umbrella crate.

`std` may imply `async` when the exposed host surface requires async APIs.
`async` must not pull in a runtime such as Tokio by itself.

## Data Contract

`clinkz-wot-td` owns protocol-neutral TD/TM representation. Its boundary is:

- Build TD and TM documents through typed builders.
- Deserialize and serialize W3C TD 1.1 documents.
- Preserve unknown extension fields and JSON-LD context data.
- Validate TD/TM structure and defaults.
- Provide URI and operation types used by runtime crates.

`AbsoluteUri` is exported at the crate root because discovery and servient APIs
use it directly.

TD/TM crates must not include concrete transport logic. Protocol-specific
metadata can be represented as extension fields, but interpretation belongs to
binding crates.

## Interaction Core

`clinkz-wot-core` owns protocol-neutral interaction semantics.

Key types:

- `ThingId` and `CorrelationId` identify Things and protocol request/response
  matching tokens.
- `AffordanceTarget` identifies Thing, property, action, or event targets.
- `InteractionInput`, `InteractionOptions`, `InteractionOutput`, and
  `InteractionStatus` carry interaction payloads, URI variables, principals,
  media hints, and result metadata.
- `Payload` and `PayloadCodec` provide media-aware payload handling.
- `WotLock<T>` is the shared lock primitive. It is a cloneable handle backed by
  `std::sync` on host builds and `critical_section` on constrained builds.
- `EventBroker`, `Subscription`, and `SubscriptionGuard` support event and
  observable-property delivery.
- `SecurityProvider` and `CredentialStore` provide inbound verification and
  outbound request credential application.

### Produced Thing State

`ExposedThing` is a concrete core type that stores a TD plus per-affordance
handler slots. It is not the application-facing handle.

Handlers are sync-primary:

- Property: read, write, observe, unobserve.
- Action: invoke, query, cancel.
- Event: subscribe, unsubscribe.

Each operation has a synchronous handler trait. With the `async` feature, each
operation also has an async twin. Sync handlers are the zero-allocation hot path
for device and embedded workloads. Async handlers are opt-in for I/O-bound
gateway and cloud behavior.

Per-affordance handler setters replace the current slot. The last registered
sync or async handler wins for that operation.

### Consumed Thing State

`ConsumedThing` is a concrete core type that stores a TD plus a list of shared
`Arc<dyn ClientBinding>` references. All per-call context is carried in
`BindingRequest`, so a single client binding instance can serve many consumed
Things.

Before outbound invocation, the consumed path:

1. Selects an affordance form for the requested operation.
2. Applies outbound security through the configured `CredentialStore` and
   `SecurityProvider`.
3. Builds a `BindingRequest`.
4. Invokes the first registered `ClientBinding` that supports the form and
   operation.

## Protocol Binding Model

Bindings are extension points implemented by protocol crates.

### ServerBinding

`ServerBinding` owns inbound protocol lifecycle:

```rust
fn serve(&self, thing_id: &ThingId, td: &Thing, ctx: &BindingContext) -> CoreResult<()>;
fn shutdown(&self, thing_id: &ThingId);
fn try_accept(&self) -> Option<InboundRequest>;
fn send_response(&self, response: InboundResponse);
```

`serve` declares routes for one Thing and starts that binding's driving model.
On `std`, a binding may spawn a task that receives transport requests, calls
`ctx.dispatch.serve_request(req).await`, and sends the response back through
the protocol. On bare `no_std`, a super-loop can poll `try_accept`.

The Servient does not own a transport driving loop.

### ClientBinding

`ClientBinding` owns outbound protocol behavior:

```rust
fn supports(&self, form: &Form, operation: Operation) -> bool;
fn supports_with_thing(&self, thing: &Thing, form: &Form, operation: Operation) -> bool;
async fn invoke(&self, request: BindingRequest) -> CoreResult<InteractionOutput>;
async fn subscribe(
    &self,
    request: BindingRequest,
) -> CoreResult<(Subscription, Box<dyn SubscriptionGuard>)>;
```

`supports_with_thing` lets bindings account for Thing-level `base` when
resolving relative form targets. `subscribe` defaults to unsupported for
bindings that only implement one-shot request/response interactions.

### Removed Facades

v4.1 removes `ProtocolBinding` and `ClientBindingFactory`. Applications
register `Arc<dyn ServerBinding>` and `Arc<dyn ClientBinding>` directly with
`ServientBuilder`. A concrete protocol crate may still expose convenience
constructors that return those trait objects.

## Zenoh Binding

Zenoh is the first optional protocol binding, not a required engine dependency.

The zenoh crate provides:

- Form planning and validation for `zenoh:` targets.
- Clinkz extension metadata extraction for zenoh priority, congestion control,
  and QoS hints.
- A Rust zenoh runtime backend behind `zenoh`.
- Constructors such as `shared`, `server`, `client`, `client_pooled`, and
  `client_pooled_default` that return direct server/client binding trait
  objects.

The shared-session constructor is suitable when Producer and Consumer use one
pre-opened session. Pooled client construction is the preferred direction for
Consumers that need to reach Things through different TD-resolved authorities.

Zenoh-specific logic must not move into TD, discovery, core, or servient crates.

## Discovery

`clinkz-wot-discovery` models WoT Discovery as:

1. Introduction: obtain a discovery endpoint.
2. Exploration: query or navigate a Thing Directory.
3. Continuation: lazily drain a discovery session.

Primary public concepts:

- `DiscoveryEndpoint`, `IntroductionSource`, and `Introducer`.
- `DirectoryQuery`, filters, projections, counts, revisions, and lease tokens.
- `DirectoryReader` and `DirectorySession`.
- `DirectoryPublisher` for lease/revision-aware publication.
- `DirectoryWatch` and `DirectoryChange`.
- `ThingDescriptionResolver` and `ThingLinkResolver`.
- `Discoverer`, `DiscoveryFilter`, `DirectoryRef`, and
  `ThingDiscoveryProcess`.
- `InMemoryDirectory` and `SharedInMemoryDirectory` as reference backends.

Discovery remains protocol-neutral. Concrete discovery transports or directory
storage backends are optional integrations.

## Servient

`clinkz-wot-servient` is the application-facing composition root.

The Servient is non-generic. It holds:

- Registry of exposed Things.
- Registry/tracking for consumed Things.
- Default server bindings.
- Default client bindings under `async`.
- Security providers and optional credential store.
- A `Discoverer`.
- Shared `EventBroker`.

The Servient's responsibilities are intentionally narrow:

- `produce(td)` creates an `ExposedThingHandle`.
- `consume(td)` creates a `ConsumedThingHandle`.
- `discover(filter)` starts a lazy discovery process.
- `fetch_td(url)` resolves a TD through discovery.
- `Dispatch::serve_request(req)` resolves inbound requests from bindings,
  verifies security, and invokes handlers.

It does not own protocol driving loops.

### ExposedThingHandle

`ExposedThingHandle` is the Producer-facing handle. It owns cloned
`Arc<dyn ServerBinding>` references captured from the Servient defaults at
`produce()` time.

Lifecycle:

1. `produce(td)` creates a draft handle.
2. Handler setters attach or replace operation handlers.
3. `expose()` calls `ServerBinding::serve` on each handle-owned binding and
   inserts the Thing into the servable registry.
4. The TD affordance set is frozen after `expose()`.
5. `destroy()` calls `ServerBinding::shutdown`, marks the slot draining, and
   removes the registry entry.

Handlers may be replaced throughout the exposed lifetime. Dynamic affordance
add/remove after `expose()` is not part of v1.

### ConsumedThingHandle

`ConsumedThingHandle` is the Consumer-facing handle. It owns a `ConsumedThing`
that was populated with cloned `Arc<dyn ClientBinding>` references captured
from the Servient defaults at `consume()` time.

It provides async methods for Scripting API operations:

- `read_property`, `write_property`, and `observe_property`.
- `invoke_action`.
- `subscribe_event`.
- `read_all_properties`, `read_multiple_properties`, and
  `write_multiple_properties`.
- Subscription teardown through matching unobserve/unsubscribe methods.

Streaming operations return local subscription streams while the handle stores
wire-side guards so protocol resources are released on explicit teardown or
handle drop.

## Security

Inbound security is binding-assisted and core-verified:

1. A server binding extracts transport credentials into `AuthMaterial`.
2. The Servient dispatch path resolves the target Thing and its effective
   security requirements.
3. A matching `SecurityProvider` verifies credentials and produces a
   `Principal`.
4. The principal is attached to `InteractionInput` before handler dispatch.

Outbound security is request-applied:

1. `ConsumedThing` resolves the form's effective security.
2. It obtains credentials from `CredentialStore`.
3. A matching `SecurityProvider::apply` writes protocol-neutral request
   metadata into `BindingRequest::applied_security`.
4. The binding maps that metadata to protocol wire representation.

## `no_std + alloc` Boundary

Embedded support includes:

- TD/TM construction, serde, validation, and round-trip behavior.
- Core interaction types and local dispatch.
- Protocol-neutral form selection and URI-template helpers.
- Discovery data models.
- Async trait surfaces when enabled without a runtime dependency.
- Binding adapters that can be driven by a host integration or super-loop.

Embedded support excludes:

- Filesystem-backed storage.
- OS sockets and process APIs.
- Thread spawning.
- Tokio or other async runtimes.
- Concrete host network backends unless gated behind `std`.

## Validation and Verification

Expected checks before considering a design-affecting change complete:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets
cargo check -p clinkz-wot-td --no-default-features
cargo check -p clinkz-wot-core --no-default-features
cargo check -p clinkz-wot-discovery --no-default-features
cargo check -p clinkz-wot-protocol-bindings --no-default-features
cargo check -p clinkz-wot-servient --no-default-features
```

Feature-specific checks should cover:

- `async` without `std` where intended.
- `zenoh` runtime tests behind explicit opt-in.
- `td2-preview` as additive and isolated from TD 1.1 defaults.
- TD/TM round-trip fixtures with unknown extension preservation.
- Multiple forms per affordance, including relative `href` plus Thing `base`.
- Protocol bindings separately from protocol-neutral core logic.

## Deprecated Documents

All previous project documents are archived under `docs/deprecated/`. They are
not active design sources. Use them only to recover historical rationale,
implementation sequencing, or audit context.
