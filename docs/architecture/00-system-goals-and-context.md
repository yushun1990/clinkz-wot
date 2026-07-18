# System Goals and Context

## Purpose

`clinkz-wot` is a Rust Web of Things engine for producing, consuming, and
discovering Things through replaceable Protocol Bindings. It provides a shared
semantic core and application runtime without turning one transport, executor,
or deployment topology into the WoT model itself.

The architecture is derived from the WoT contract and explicit operational
goals. Existing code is evidence about migration cost and defects, not design
authority.

## Compatibility baseline

- W3C WoT Thing Description 1.1 is the default compliance target.
- Unknown extension members survive deserialize/serialize round trips.
- `base` and relative form `href` values are resolved by one shared planner.
- Clinkz-specific metadata uses a Clinkz JSON-LD namespace and never masquerades
  as W3C vocabulary.
- TD 2.0 additions remain behind `td2-preview` and do not change TD 1.1 behavior
  when absent.

## Goals

The engine must provide:

- deterministic form, operation, security, and binding selection;
- immutable precompiled plans on interaction hot paths;
- protocol-neutral application APIs and core SPI values;
- bounded memory, work, fan-out, cleanup, and diagnostics;
- generation-safe replacement of handles and external resources;
- explicit Producer exposure and destroy transactions;
- pull-capable subscriptions and protocol-native collection operations;
- host and constrained execution profiles with equivalent outcomes;
- a simple third-party Protocol Binding integration path; and
- executable conformance and performance evidence before implementation
  tranches are admitted.

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

## System context

```text
Application
    |
    | application-facing API
    v
Servient ------------------------------------------------ Discovery client
    |                         plan snapshots                  |
    | lifecycle, dispatch, scheduling, cleanup                | Directory/API
    v                                                         v
Protocol-neutral core <------ shared planning ------> TD/TM documents
    |
    | planning extension + execution SPI
    v
Concrete Protocol Binding ------ protocol I/O ------ remote Things/peers
```

The application chooses which concrete binding crates are linked into its
binary and passes their complete registrations to the Servient builder. The
Servient never discovers a binding by scanning the process or TD protocol
strings.

## Execution profiles

The architecture separates three independent axes:

- compilation: `std` or `no_std + alloc`;
- progress: host-erased integration or caller-driven poll/step; and
- resources: dynamically configured bounded capacities or application-static
  storage.

A gateway may use either progress model. `async` describes an API syntax
surface and never implies Tokio or another executor. Concrete host bindings may
integrate a reactor internally, but their observable work remains owned through
the engine SPI and its bounded records.

## Deployment units

- A Rust crate is the compile-time extension unit.
- A complete binding registration is the startup composition unit.
- A Servient instance is the runtime ownership and failure-isolation unit.
- A process, container, or firmware image is the v1 rollout unit.

Binding registration is distinct from dynamic code loading. Multiple binding
instances may be linked into one binary, but one Servient's registration set is
frozen at build completion. Reconfiguration uses a new Servient instance and a
drain/cutover transaction outside the old instance.
