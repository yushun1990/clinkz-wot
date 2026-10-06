# Module Boundaries

Status: active v5.1 authority.

## Target dependency direction

```text
foundation <- td
foundation + td <- core
foundation + td + core <- planning <- concrete Protocol Bindings
foundation + td + core <- discovery
foundation + td + core + planning + discovery <- servient
core <- codec crates
selected public crates <- clinkz-wot umbrella
```

The v5.0 target carries forward the planned rename of the current shared
`clinkz-wot-protocol-bindings` compiler crate to `clinkz-wot-planning`.
Planning is not a protocol implementation, and its crate name must not suggest
that it owns binding execution. Concrete Protocol Binding crates remain
separate dependencies of the application or umbrella crate, never of Servient.

`CRATE-DEPS-001`: The dependency graph MUST remain acyclic and directed from
Foundation to TD/Core, from TD/Core to Planning and Discovery, and from those
protocol-neutral layers to Servient composition. Codec crates depend on Core;
concrete Protocol Bindings depend on Planning/Core contracts but MUST NOT
become Servient dependencies merely to implement an SPI. Lower layers MUST NOT
gain an optional higher-layer policy through feature unification. Foundation
owns no TD/runtime/protocol vocabulary; Core owns no Servient, Discovery, or
concrete protocol; Discovery owns no Directory service or storage backend; and
only the umbrella is expected to compose every selected public crate.

## Responsibility map

| Layer | Owns | Produces | Must not own |
| --- | --- | --- | --- |
| `clinkz-wot-foundation` | Resource reservations, work budgets, monotonic time, generations, profile-independent accounting | Bounded primitive values | TD vocabulary, interaction semantics, plans, registries, queues, protocol behavior |
| `clinkz-wot-td` | Lossless TD/TM models, builders, Serde, one Basic/default/security semantic kernel, bounded typed inspection, trusted semantic lending and URI values | Lifetime-bearing validation proofs and paid semantic loans | Form/binding selection, runtime caches, transport behavior |
| `clinkz-wot-core` | Protocol-neutral IDs, errors, payloads, handlers, security/codec contracts, immutable plan values, binding SPI values/traits, lifecycle outcomes | Semantic values and execution contracts | Application handles, plan compiler algorithms, global schedulers, universal subscription queues, protocol I/O |
| `clinkz-wot-planning` | Candidate/Form selection from TD-owned semantic inputs, capability indexes, logical-plan construction, binding compiler coordination, URI-template compilation | Admitted-plan build output | TD Basic/default/security reinterpretation, binding execution, Servient registries, runtime queues, concrete protocol I/O |
| `clinkz-wot-servient` | Application facade, registration snapshot, plan-set ownership, admission, handler/security orchestration, route lifecycle, scheduling, cleanup, status | Produced/consumed handles and runtime events | Protocol syntax, transport I/O, TD reparsing, implicit Directory service |
| `clinkz-wot-discovery` | Discovery/Directory client values, sessions, watches, publisher client, source envelopes | Source-bearing TD documents and client progress | Directory service, storage backend, server query/redaction policy, endpoint hosting |
| Concrete binding crate | Compiler extension, capability declaration, protocol route/client artifacts, I/O, correlation, native flow control/multiplexing, auth extraction, cleanup | Complete binding registration bundle | Servient registry, handlers, shared TD defaulting, cross-binding scheduling, hidden unbounded tasks |
| Codec crate | Bounded codecs and incremental state | Decoded/encoded payloads | Runtime or transport policy |
| Umbrella crate | Feature composition and deliberate re-exports | Application import surface | Runtime behavior or duplicate definitions |

## Core internal boundaries

Core separates at least:

- `identity` and generation-bearing references;
- `error`, retry, cleanup, progress, and status values;
- `handler` context, cancellation, traits, and portable slots;
- `security` and `codec` semantic contracts;
- immutable `plan` values and source identity;
- `binding/client`, `binding/server`, `binding/subscription`,
  `binding/emission`, and registration values; and
- local protocol-neutral dispatch semantics.

Core does not use a catch-all event module for queues, merge policy, binding
publication, and application streams.

## Planning boundaries

Planning receives a validated document view, immutable policy, resource budget,
and complete binding registration snapshot. It may call side-effect-free
capability and compiler-extension methods. It returns values and admitted
footprints; it never opens a route, sends a request, or retains a Servient
handle.

Logical plans share protocol-neutral work across candidates. Binding artifacts
contain only protocol-specific data that cannot be shared. A binding compiler
does not receive authority to reinterpret W3C defaults, choose a different
operation, or access credentials.

For the first Consumer Property Read aggregate, TD owns bounded complete typed
inspection/Basic and an opaque `ValidatedThing<'td>` external loan. Its movable
semantic cursor lends one paid Property/Form coordinate until acknowledged;
current derived URI state stays TD-owned and every event loan is short. Shared
Basic/default/security/URI meaning never moves into Planning. There is no mandatory
complete normalized TD or unrestricted synchronous semantic view.

Planning owns complete preflight, owned required facts/lookup/candidates, every
compiler bound before start, sequential eager compilation, exact output footprint
and a source-independent Frozen draft. It retains no loan/pointer into scratch
across Pending and consumes input-bearing state before returning the draft.
Concrete artifacts satisfy owned-lifetime and cleanup eligibility, beyond a generic
wrapper's appearance. The completed exact-coordinate leaf remains narrower
regression evidence; its raw `PlanBuildInput<&Thing>` is not an aggregate trust entry.

The explicit TD `validated-thing` capability/AP boundary remains as specified by
[the TD admission record](../work-packages/WP-100-consumer-validated-thing-admission.md).
Ordinary typed APIs stay available without it. Strict literal JSON and shared field/
RFC3339 decoding are a separate future TD ingestion capability under ADR-0020;
its eventual controlled backend lends the same closed semantic program.

## Servient boundaries

Servient owns the transaction that composes modules. In particular it owns:

- the immutable binding registration snapshot;
- plan-set publication and retirement;
- generation-safe produced/consumed registries;
- callback leases and lock-free callback invocation;
- cross-binding fairness and isolation;
- application-visible subscription and emission facades;
- cleanup reservation, progress, and durable in-instance status; and
- host/static policy selection.

It schedules binding SPI progress but does not implement protocol I/O.

In the first Consumer slice, Servient owns the transaction and immutable checked
policy/registration capture, child/parent capacity pairing, runtime reservation
between preflight and materialization, independent Thing-slot/plan-set generations,
cleanup transfer, final checks, permit/install, registration retention, leases/drain
and reclaim. Build scratch and all input/config/compiler loans are terminal before
final checks and permit. Actual capacity and overlap remain charged until physical
release; an upstream source keeps its own charge. Aggregate allowances are not
contiguous allocation observations.

The private permit closes cancellation under exclusive slot mutation authority.
It installs preallocated complete runtime material without callback, allocation,
yield, source query or fallible work. Published owns execution output, one complete
registration and lifecycle/resource records only. Servient does no Basic/default/
security/URI work, semantic recount, lookup rebuild or TD retention. Legacy by-value
consume/Arc<Thing> ergonomics cannot define the bounded borrowed facade.

## Binding boundaries

A concrete binding receives only compiled, selected values. It may retain
protocol-local buffers and reactor handles only within its declared and admitted
footprint. External-input queues use registration-declared item/byte limits and
the shared overflow contract.

A binding may use a protocol runtime internally to make I/O ready and wake an
engine-owned call or route driver. It may not detach semantic ownership, call
application handlers directly, or report terminal state only through logs.

## Discovery boundary

The engine-side Discovery crate is a client. Storage engines, Directory server
query execution, redaction services, replication, and service SLOs belong to
future service crates or deployments. No Servient default silently constructs a
Directory service.

## Feature boundary

TD, planning values, core semantics, constrained binding contracts, and static
Servient abstractions support `no_std + alloc`. Filesystem, sockets, threads,
processes, dynamic libraries, and executor ownership remain in concrete host
adapters or higher deployment layers.
