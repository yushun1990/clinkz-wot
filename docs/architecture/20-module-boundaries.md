# Module Boundaries

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

The v4.9 target renames the current shared
`clinkz-wot-protocol-bindings` compiler crate to `clinkz-wot-planning`.
Planning is not a protocol implementation, and its crate name must not suggest
that it owns binding execution. Concrete Protocol Binding crates remain
separate dependencies of the application or umbrella crate, never of Servient.

## Responsibility map

| Layer | Owns | Produces | Must not own |
| --- | --- | --- | --- |
| `clinkz-wot-foundation` | Resource reservations, work budgets, monotonic time, generations, profile-independent accounting | Bounded primitive values | TD vocabulary, interaction semantics, plans, registries, queues, protocol behavior |
| `clinkz-wot-td` | Lossless TD/TM models, builders, Serde, W3C defaults, validation, URI values | Validated documents and pure views | Form/binding selection, runtime caches, transport behavior |
| `clinkz-wot-core` | Protocol-neutral IDs, errors, payloads, handlers, security/codec contracts, immutable plan values, binding SPI values/traits, lifecycle outcomes | Semantic values and execution contracts | Application handles, plan compiler algorithms, global schedulers, universal subscription queues, protocol I/O |
| `clinkz-wot-planning` | Effective-form resolution, capability indexes, logical-plan construction, binding compiler coordination, URI templates | Admitted-plan build output | Binding execution, Servient registries, runtime queues, concrete protocol I/O |
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
