<div align="center">

# ClinkZ-WOT

**A protocol-neutral Rust Web of Things runtime based on the W3C WoT programming model.**

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![W3C WoT](https://img.shields.io/badge/W3C-Web%20of%20Things-005A9C)](https://www.w3.org/WoT/)
[![Project Status](https://img.shields.io/badge/status-early%20development-yellow)](#project-status)

[Goals](docs/architecture/00-system-goals-and-context.md) ·
[Architecture](#architecture) ·
[Repository](#repository-structure) ·
[Development Method](#development-method) ·
[Status](#project-status) ·
[Contributing](#contributing)


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
          |  Planning Layer   |                     Zenoh / MQTT / HTTP
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

- interaction routing
- lifecycle management
- handler execution
- activation and cleanup
- runtime state

Protocol Bindings provide communication capabilities only.

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

Interaction decisions are prepared before runtime execution.

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

The current concrete production slice is
[Host HTTP Consumer Property Read](protocol-bindings/protocols/http/README.md),
with an explicit cleartext-IP / NoSec envelope. Zenoh retains scoped native
transport and architecture probes; its legacy direct-dispatch facade is retired.
Neither establishes general operation or protocol parity.

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
registered evidence establish current capabilities. Removing legacy APIs does
not remove their corresponding product objectives. The
[migration evidence](docs/evidence/WP-200-preparation-execution-boundary.toml)
distinguishes retained Property Read behavior from withdrawn API implementations
and capabilities still awaiting replacements.

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
