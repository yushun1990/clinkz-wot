# System Goals and Context

## Purpose

`clinkz-wot` is a Rust Web of Things engine for producing, consuming, and
discovering Things through replaceable Protocol Bindings. It provides a shared
semantic core and application runtime without turning one transport, executor,
or deployment topology into the WoT model itself.

This document is the single authoritative home for enduring product goals.
Architecture and ADRs choose how to pursue them; `PLAN.md` bounds delivery;
code, tests and scoped evidence establish current availability. None of those
silently redefines the mission. Existing code is evidence about migration cost
and defects, not the limit of the intended product.

Changing a product goal requires an explicit Owner decision, presented with
rationale and trade-offs. API removal, an accepted ADR, a v1 exclusion or a
failed implementation attempt is not such a decision.

## Compatibility baseline

- W3C WoT Thing Description 1.1 is the default compliance target.
- Preserve unknown extension associations through document round trips.
  [ADR-0020](../ADRs/0020-strict-json-value-decoding.org) specifies current
  ordinary-serde Value/RawValue exceptions and future literal strict ingestion;
  typed admission inspects its supplied immutable value. Those input contracts
  do not justify rejecting arbitrary extension vocabularies as a product limit.
- `base` and relative form `href` values retain consistent TD semantics across
  bindings; the current architecture assigns resolution to TD.
- Clinkz-specific metadata uses a Clinkz JSON-LD namespace and never masquerades
  as W3C vocabulary.
- Future TD 2.0 additions require an explicit reviewed capability/revision.
  The obsolete `td2-preview` gate for TD 1.1 Action `synchronous` is removed.

## Goals

The enduring objectives are:

- **Produce and consume Things:** expose application capabilities and interact
  with remote Things through Properties, Actions and Events. This includes
  Property read/write/observation, Action invocation and its described
  query/cancellation lifecycle, Event emission/subscription/unsubscription,
  and described collection operations.
- **Discover Things:** find Things and retrieve their descriptions, including
  interaction with Directory services as a client. Removing implicit Directory
  composition does not remove discovery from the product.
- **Secure interactions:** apply TD-described security and application policy,
  including authentication, authorization, credentials, scopes and transport
  trust where applicable. Unsupported selected security must refuse rather
  than silently downgrade. Preserving extension security data is distinct from
  being able to execute that scheme.
- **Preserve interoperability and extensibility:** provide protocol-neutral
  application semantics, deterministic and explainable form/operation/security/
  binding selection, and a practical third-party binding integration path.
  One transport's request shape must not define the WoT interaction model.
- **Support long-lived and collection interactions:** provide controllable
  observation/subscription, flow control, cancellation and terminal cleanup,
  and preserve protocol-native collection capabilities where available.
- **Make resource and lifecycle behavior dependable:** bound memory, work,
  fan-out, cleanup and diagnostics; keep activation, teardown and replacement
  under explicit ownership; prevent stale handles or late results from
  affecting a successor. Measure useful interaction performance on real workloads.
- **Serve host and constrained/embedded applications:** preserve equivalent
  WoT semantics for supported interactions while allowing different storage,
  progress, capacities and protocol backends. Host-first delivery does not
  withdraw embedded execution as an objective.
- **Make capability claims truthful:** identify unsupported combinations and
  validate interoperability, conformance, resource and performance claims with
  evidence appropriate to the advertised capability and environment.

These are capability-family objectives, not a claim that every binding or
device implements every operation, security scheme or deployment combination.

## Architectural decisions serving the goals

ADR-0025 selects a semantic Engine with separate execution capabilities over
Binding-owned native runtimes. Owned prepared data, independent instance
permission, truthful publication and retained cancellation/cleanup are the
means selected now. Eager plan sets, a plan-set epoch per Thing and route-scoped
native progress are no longer universal architecture requirements.

Likewise, executable evidence before tranche admission is an implementation
policy owned by [project governance](../../PROJECT_GOVERNANCE.md), not an
additional WoT capability. The context, profile and deployment choices below
describe the current architecture. Detailed ownership belongs to the
[module](20-module-boundaries.md), [Planning](30-compiled-plan-lifecycle.md),
[Binding](40-protocol-binding-spi-and-deployment.md) and
[Servient](50-servient-runtime-lifecycle.md) contracts.

## v1 delivery scope

[PLAN.md](../../PLAN.md) owns release scope and ordering. It selects Host
HTTP/1.1, the public Producer/Consumer loop and HTTPS/security qualification,
with portable shared layers and client-side Directory/Discovery work. Zenoh/
pico production and real embedded protocol/hardware qualification are deferred
separately. This scope neither reduces the goals above to Property Read nor
grants completion of the remaining interaction families.

## Non-goals for v1

The following are deliberately outside the v1 engine contract:

- a Directory service, storage topology, or server-side query engine;
- a durable message broker or append log in core;
- implicit per-affordance lowering of collection subscriptions;
- runtime discovery or loading of arbitrary Rust dynamic libraries;
- hot unloading of binding code with live routes or calls;
- a universal async executor or transport reactor;
- hidden unbounded task, queue, cache, or retry ownership; and
- transparent migration of a live handle to a new binding registration.

These exclusions delimit v1, without deciding future product scope. The
enduring resource and lifecycle goals still apply to later designs. In particular,
excluding a Directory service does not exclude Directory clients, and excluding
a durable broker in Core does not exclude bindings to broker/session protocols.

## Implemented capabilities and migration claims

Availability is established by public code and executed evidence, not by this
goal list, operation enums, planned API rows or an accepted ADR. The
[Preparation/Execution migration evidence](../evidence/WP-200-preparation-execution-boundary.toml)
owns the exact retained, removed and unimplemented scope. It retains narrow
Producer/Consumer Property Read paths; the concrete HTTP path now includes Host,
cleartext-IP, NoSec Consumer reads and a bounded loopback Producer service. Broad legacy Action/Event, write and
subscription facades have no claimed replacement in that migration.
[Discovery](../../discovery/src/lib.rs) remains separately available; removal
of Servient convenience methods does not complete the
[target Directory client contract](../work-packages/WP-500-discovery.md).

The pre-v1 cleanup also withdraws orphaned Core event/subscription APIs and the
deferred Zenoh/pico transport stack. Their removal adds no replacement capability;
the [retained implementation boundary](20-module-boundaries.md#retained-pre-v1-implementation-baseline)
identifies current coverage. Strict remote JSON ingestion remains future work
after retirement of its non-production prototypes.

Deleting legacy Producer/Consumer, Action/Event, subscription, discovery,
security or embedded-related APIs withdraws those API implementations and any
unmatched current availability, not the corresponding enduring objectives.
Deleted tests are not evidence of migrated capability. The current pointer-
atomic requirement for Core execution is a representation gap, not a decision
to exclude devices without atomics; portable TD/tooling compilation is not
embedded runtime qualification. Runtime principal/auth/scope values alone do
not establish authentication or HTTPS support.

[Review 08's diagnostics](../reviews/review-08-integrated-architecture-reassessment.md#findings-and-evidence-classification)
also identify semantic/mapping and extension-security limitations in existing
code. They remain gaps against the goals; they cannot redefine interoperability
or security as whatever the current implementation accepts.

## System context

```text
TD semantic interpretation -> optional Planning/preparation -> owned mapping
Application <-> Servient admission/dispatch <-> Binding native runtimes <-> peers
                     Core semantic / data / security contracts
Discovery client -> bounded retrieval -> TD admission
```

The application chooses which concrete binding crates are linked into its
binary and passes their complete registrations to the Servient builder. The
Servient never discovers a binding by scanning the process or TD protocol
strings.

## Execution profiles

The architecture separates independent axes:

- compilation: `std` or `no_std + alloc`;
- progress: host-erased integration or caller-driven poll/step; and
- resources: dynamically configured bounded capacities or application-static
  storage;
- allocation, thread mobility and pointer atomics; and
- dynamic preparation, selected capabilities and native backend/hardware support.

A gateway may use either progress model. `async` describes an API syntax
surface and never implies Tokio or another executor. Concrete host bindings may
integrate a reactor internally, with an explicit physical owner, bounded ingress and retained cleanup. Logical
acceptance remains independently Engine-owned.

## Deployment units

- A Rust crate is the compile-time extension unit.
- A complete binding registration is the startup composition unit.
- A Servient instance is a logical authority and accepted-work ownership unit;
  compatible physical services may be shared through explicit leases.
- A process, container, or firmware image is the v1 rollout unit.

A Servient is not a memory-protection or process-crash boundary. Host process
OOM/abort/kill, runtime availability, OS limits and restart belong to deployment;
in-process admission and protocol retirement retain their explicit guarantees.
[ADR-0022](../ADRs/0022-first-production-binding-and-runtime-environments.org)
selects Host HTTP/1.1 plus HTTPS as the first production direction, and separates
portable semantic support from real embedded protocol/hardware qualification.

Binding registration is distinct from dynamic code loading. Multiple binding
instances may be linked into one binary, but one Servient's registration set is
frozen at build completion. Reconfiguration uses a new Servient instance and a
drain/cutover transaction outside the old instance.
