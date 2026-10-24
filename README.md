<div align="center">

# ClinkZ-WOT

**A protocol-neutral Rust Web of Things runtime based on the W3C WoT programming model.**

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![W3C WoT](https://img.shields.io/badge/W3C-Web%20of%20Things-005A9C)](https://www.w3.org/WoT/)
[![Project Status](https://img.shields.io/badge/status-early%20development-yellow)](#current-status)

[Goals](docs/architecture/00-system-goals-and-context.md) ·
[Architecture](#architecture) ·
[Repository](Cargo.toml) ·
[Development Method](PROJECT_GOVERNANCE.md) ·
[Status](#current-status) ·
[Contributing](PROJECT_GOVERNANCE.md#git-and-pull-request-workflow)


</div>

---

## Overview

`clinkz-wot` provides a **protocol-neutral runtime layer** for building interoperable IoT systems.

The runtime separates:

- **Thing semantics**
- **interaction execution**
- **protocol communication**

A Thing Description (TD) defines **what a Thing provides**.

Protocol Bindings define **how communication happens**.

Applications interact with Things instead of depending on specific transports.

```text
             Thing Description
                    |                              Protocol Binding SPI
                    v                                        |
          +-------------------+                              v
          |  Planning Layer   |                     HTTP Property Read
          +-------------------+
                    |
                    v
          +-------------------+
          | Servient Runtime  |
          +-------------------+
                    |
                    v
              Application

```

---

# Architecture

## Servient-Orchestrated Runtime

The Servient is the runtime authority.

It owns:

- logical recipient admission and behavior dispatch
- lifecycle management
- handler execution
- activation and cleanup
- runtime state

Protocol Bindings own native mapping, matching, correlation, distribution, I/O
and physical infrastructure. Native progress and logical acceptance have independent
authority and resource owners. A Thing does not imply one connection or listener.

```text
            Protocol Message
                    |
                    v
            Protocol Binding
                    |
                    v
            Servient Runtime
                    |
                    v
            Application Handler
```

Protocol Bindings do **not** directly dispatch application handlers.

---

# Design Principles

## Semantic First

The runtime is built around the W3C Thing Description model.

A TD describes:

- Properties
- Actions
- Events
- Data schemas
- Interaction forms

The runtime operates on semantic interactions rather than protocol messages.

## Protocol Neutrality

The core runtime does not depend on any transport protocol.

Protocol Bindings are responsible for:

- protocol addressing
- serialization and deserialization
- transport sessions
- message exchange
- protocol-specific flow control

## Compiled Execution Model

Shared TD meaning is normalized once; selected mapping preparation completes
before dependent execution. Consumer native preparation may occur on first use;
Producer publication requires complete final support and required readiness.
Prepared content grants no current execution permission.

The [v6.0 authority map](docs/spec/authority.toml) and
[ADR-0025](docs/ADRs/0025-semantic-engine-and-binding-runtimes.org) own the target.
Current Property Read carriers are restricted adapters, without a universal
compiled-plan-set, RPC or session requirement.

```text
            Thing Description
                    |
                    v
            Parse & Validate
                    |
                    v
            Logical Planning
                    |
                    v
            Binding Compilation
                    |
                    v
            Runtime Execution
```

---

# Protocol Binding SPI

Protocol Bindings extend the runtime with protocol capabilities.

``` text
            Protocol Payload
                    |
            Protocol Binding
                    |
            Protocol-neutral Interaction
                    |
            Servient Runtime
```

The current concrete binding is
[Host HTTP Producer and Consumer Property Read](protocol-bindings/protocols/http/README.md),
with an explicit cleartext-IP / NoSec envelope, source-free execution and manual
Servient progress. The Producer serves a bounded loopback GET endpoint. The
Consumer also supports the documented explicit method/header mappings.
HTTPS/security qualification and broader operation families remain delivery work.
Zenoh/pico implementations and historical prototypes have been withdrawn; Git
retains their source and results. HTTP is an optional direct Cargo dependency.

---

# Platform Targets

| Target | Status |
| --- | --- |
| Host | Narrow Property Read paths; concrete binding limits apply |
| Async syntax | Optional; does not select an executor |
| `no_std + alloc` | Scoped portable layers and static execution; Core execution requires pointer atomics |
| Embedded production | Requires separate backend, workload and hardware qualification |

---

# Current Status

`clinkz-wot` is under active architectural development.

The [system goals](docs/architecture/00-system-goals-and-context.md) own the
enduring mission, [PLAN.md](PLAN.md) owns v1 delivery scope, and code/tests plus
registered evidence establish current capabilities. The retained baseline includes:

- TD/TM models, builders, extension-preserving round trips, validation and bounded
  borrowed semantic preparation for Properties, Actions, Events and security data;
- Host HTTP Property Read Producer/Consumer execution and its real-I/O regressions;
- protocol-neutral static Property Read runtimes and bounded ownership/cleanup;
- independently composed discovery clients and their in-memory reference backend;
- payload codecs, logical clocks, resource accounting and portable shared layers.

Legacy Core event fan-out, queue/merge subscriptions, `SubscriptionAcceptance`,
payload-bearing `InteractionOptions`, old form-selection/template helpers, and
the Zenoh transport/session/pico APIs are removed without compatibility wrappers.
General Property write/observation, Action and Event execution, subscriptions,
secure transport and embedded production remain product objectives; the removed
implementations are not claimed replaced. See the
[module boundary](docs/architecture/20-module-boundaries.md) for the implementation
disposition and [PLAN.md](PLAN.md) for delivery scope.

---

# Build

```bash
git clone https://github.com/yushun1990/clinkz-wot.git
cd clinkz-wot
cargo test --workspace
```

---

# License

Apache License 2.0
