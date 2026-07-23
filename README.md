# clinkz-wot

> A protocol-neutral Rust Web of Things runtime based on the W3C WoT programming model.

`clinkz-wot` is a Rust implementation of Web of Things concepts with semantic interaction contracts and pluggable Protocol Bindings.

[![Rust](https://img.shields.io/badge/language-Rust-orange)](https://www.rust-lang.org/)
[![WoT](https://img.shields.io/badge/model-W3C%20WoT-blue)](https://www.w3.org/WoT/)
[![License](https://img.shields.io/badge/license-Apache--2.0-green)](LICENSE)

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
                    |
                    v
          +-------------------+
          |  Planning Layer   |
          +-------------------+
                    |
                    v
          +-------------------+
          | Servient Runtime  |
          +-------------------+
                    |
                    v
              Application


          Protocol Binding SPI

                    |
                    v

          Zenoh / MQTT / HTTP
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

```
Protocol Payload

        |

Protocol Binding

        |

Protocol-neutral Interaction

        |

Servient Runtime
```

Current implementation:

- Zenoh Binding

---

# Platform Targets

| Target | Status |
| --- | --- |
| Standard Rust environments | Supported |
| Async runtime integration | Supported |
| `no_std + alloc` | Supported |
| Embedded execution model | In development |

---

# Workspace Structure

```text
foundation/
    Ownership and resource foundations

td/
    W3C Thing Description model

core/
    Protocol-neutral runtime contracts

planning/
    Logical planner and capability matching

protocol-bindings/
    Protocol integration layer

servient/
    Runtime orchestration engine

docs/
    Architecture and specifications
```

---

# Current Status

`clinkz-wot` is under active architectural development.

Current focus:

- Servient lifecycle refinement
- Protocol Binding SPI stabilization
- Immutable plan execution model
- Zenoh integration

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
