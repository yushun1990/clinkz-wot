# clinkz-wot

::: {align="center"}
## clinkz-wot

### A protocol-neutral Rust Web of Things runtime

Implementation of W3C WoT concepts with semantic interaction contracts
and pluggable Protocol Bindings.
:::

------------------------------------------------------------------------

## Overview

`clinkz-wot` is a Rust-based **Web of Things (WoT) runtime** designed
around the W3C WoT programming model.

The engine separates:

-   **semantic interaction models**
-   **runtime execution**
-   **protocol-specific communication**

Thing Descriptions define *what* a Thing provides, while Protocol
Bindings define *how* communication happens.

The goal is a runtime where applications depend on Things, not transport
protocols.

------------------------------------------------------------------------

## Architecture

``` text
              Thing Description
                      |
                      v
              +---------------+
              |    Planner    |
              +---------------+
                      |
                      v
            Immutable Plan Set
                      |
                      v
              +---------------+
              |   Servient    |
              |    Runtime    |
              +---------------+
                 ^       |
                 |       v
                 |  Application
                 |
        Protocol Binding SPI
                 |
                 v
          Zenoh / MQTT / HTTP
```

The Servient owns runtime orchestration. Protocol Bindings provide
transport integration.

------------------------------------------------------------------------

## Design Principles

### Semantic Contract First

W3C Thing Description is the semantic contract.

A TD defines:

-   Properties
-   Actions
-   Events
-   Data schemas
-   Interaction forms

The runtime operates on semantic interactions rather than protocol
messages.

------------------------------------------------------------------------

### Protocol Neutrality

The runtime core does not depend on a specific transport.

Protocol Bindings are responsible for:

-   protocol addressing
-   serialization and deserialization
-   transport sessions
-   correlation
-   protocol-local flow control

The runtime remains independent from:

-   Zenoh
-   MQTT
-   HTTP
-   WebSocket
-   other future protocols

------------------------------------------------------------------------

### Servient-Orchestrated Execution

The Servient owns:

-   lifecycle management
-   interaction routing
-   handler execution
-   activation
-   cleanup
-   operation lifetime

Protocol Bindings do **not** directly invoke application handlers.

``` text
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

------------------------------------------------------------------------

## Execution Model

Interaction decisions are prepared before runtime execution.

``` text
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

The runtime executes immutable admitted plans instead of repeatedly
discovering capabilities on the hot path.

------------------------------------------------------------------------

## Platform Targets

`clinkz-wot` targets both host and constrained environments.

  Target                        Status
  ----------------------------- ----------------
  Standard host environments    Supported
  Async host integration        Supported
  `no_std + alloc`              Supported
  Constrained execution model   In development

The same protocol-neutral semantics are preserved across platforms.

------------------------------------------------------------------------

## Protocol Bindings

Protocol Bindings extend the runtime with communication capabilities.

Current binding:

-   **Zenoh** (first implementation)

A Protocol Binding is responsible for protocol-specific behavior only.

It does not own:

-   application lifecycle
-   TD interpretation during execution
-   handler dispatch
-   runtime ownership rules

------------------------------------------------------------------------

## Workspace Structure

``` text
foundation/
    Resource and ownership foundations

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

discovery/
    WoT discovery support

docs/
    Architecture and specifications
```

------------------------------------------------------------------------

## Current Status

`clinkz-wot` is under active architectural development.

Current focus:

-   v4.9 architecture implementation
-   Servient lifecycle refinement
-   Protocol Binding SPI stabilization
-   Zenoh integration

The project is not yet a production release. Public APIs and module
boundaries may continue to evolve.

------------------------------------------------------------------------

## Documentation

Architecture:

    docs/architecture/

Design specification:

    docs/design.md

Architecture decisions:

    docs/ADRs/

------------------------------------------------------------------------

## Build

``` bash
git clone https://github.com/yushun1990/clinkz-wot.git

cd clinkz-wot

cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

------------------------------------------------------------------------

## License

Apache License 2.0
