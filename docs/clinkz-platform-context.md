# Clinkz Platform Context

## Platform Vision

Clinkz is a semantic IoT platform built around the Web of Things model.

In Clinkz, devices, databases, APIs, services, gateways, and compute tasks can all be represented as Things. TD and TM documents provide the semantic contract for these Things.

## Role of clinkz-wot

`clinkz-wot` is the WoT engine layer for the platform.

It provides:

- TD and TM modeling.
- Validation and serialization.
- Protocol-neutral operation dispatch.
- Protocol binding integration.
- Discovery integration.
- Servient runtime building blocks.

## Zenoh in Clinkz

Clinkz Platform uses zenoh as its default communication bus.

This platform choice does not make zenoh mandatory for the WoT engine. The engine remains protocol-neutral, and zenoh is implemented as the first optional binding.

Zenoh may be used to connect distributed storage, computation, gateways, and physical devices. It may also replace or front legacy protocols in deployments where that is beneficial.

## Thing Types

The platform should be able to model:

- Physical device Things.
- Gateway Things.
- Database Things.
- API Things.
- Compute Things.
- Virtual composite Things.
- Directory and registry Things.

All of these should use the same TD/TM foundation.

## Extension Policy

Clinkz-specific terms should be expressed through JSON-LD extension vocabulary.

The extension vocabulary should cover platform concepts such as:

- Distributed resource identifiers.
- Zenoh key expressions.
- Storage and query hints.
- Compute placement hints.
- Platform ownership and tenancy metadata.

These extensions must not replace standard WoT TD semantics when a standard term already exists.

## Deployment Patterns

### Pattern 1: Embedded Servient (devices, gateways)

The Servient runs as the primary runtime on a device or gateway. Handlers
implement device business logic (sensor reads, actuator control, sub-device
proxying). The Servient manages protocol primitives (zenoh queryables, put
listeners, event fan-out), security verification, and discovery integration.

This is the core deployment pattern for Clinkz edge devices and gateways.
Multiple Things can be exposed from a single Servient instance sharing one
zenoh session.

### Pattern 2: TD-only (microservices, third-party systems)

REST/gRPC microservices do not need to embed the WoT engine. They publish a TD
that describes their existing HTTP/gRPC endpoints via standard WoT forms
(`href: "https://api.example.com/sensors/temp"`). WoT consumers discover the TD
through the TDD and call the microservice's native endpoints directly via the
ConsumedThing path with an HTTP binding.

The microservice has no runtime dependency on the WoT engine. The TD is a data
contract (serializable, publishable, queryable), not a runtime.

### Pattern 3: ConsumedThing SDK (cloud consumers)

Cloud applications that interact with Things use a lightweight ConsumedThing
client: read a TD, select a form, invoke the protocol binding (HTTP, zenoh).
No full Servient is required on the consumer side — only form selection,
security material application, and a protocol client.

## Edge Governance (emergent)

Servient + Zenoh + TDD naturally provide IoT/edge governance capabilities
without requiring the engine to implement governance features:

| Governance capability | How it emerges |
|---|---|
| Device registration | `Servient::expose` → TDD `register(td)` |
| Semantic discovery | TDD query by TD fields (`@type`, properties, semantics) |
| Cross-network routing | zenoh router infrastructure (protocol-level, not engine) |
| Event streaming | zenoh pub/sub via `EventBroker` + `PublisherSink` |
| Lifecycle management | TDD entry presence = registered; `destroy` → TDD `unregister` |

This is an **emergent** property of the WoT architecture: the TD carries
capability semantics (not just address), zenoh carries cross-network routing,
and the TDD provides registration/discovery. Together they cover the core
IoT/edge governance surface — device-to-cloud, gateway aggregation, and
semantic discovery — without the engine itself containing governance logic.

The engine does NOT implement governance features that are invasive to the
protocol or dispatch flow (load balancing, circuit breaking, rate limiting,
active health probing, distributed tracing). These belong to the **Clinkz
platform layer** (above the engine) or to the **protocol layer** (zenoh router
configuration, service mesh).

## Architectural Boundary

The WoT engine (`clinkz-wot`) is protocol-neutral and focused on:

- TD/TM data model, validation, serialization, round-trip fidelity.
- Protocol-neutral operation dispatch (form selection, security verification,
  payload codec).
- Servient runtime composition (expose, consume, driving loop, event broker,
  runtime TD mutation lifecycle).
- Discovery directory traits (protocol-neutral registration, query, listing).

The engine deliberately does NOT include:

- Protocol-specific governance (load balancing, circuit breaking, rate
  limiting, traffic shaping).
- Active health monitoring / TTL-based liveness probing.
- Platform-level security policy (mTLS mesh, token rotation, audit logging).
- Observability infrastructure (distributed tracing, metrics export).
- Configuration management / secret rotation.

These are **platform-layer** responsibilities, implemented above the engine
using its trait surfaces (`ServerBinding`, `ThingDirectory`,
`SecurityProvider`, etc.) and the zenoh protocol's own capabilities (routing,
QoS, attachment-based auth).
