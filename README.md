# clinkz-wot

<p align="center">
  <b>Simple Links. Infinite Possibilities.</b>
</p>

<p align="center">
  A protocol-neutral Rust Web of Things runtime based on W3C WoT.
</p>

## Overview

`clinkz-wot` is a protocol-neutral Rust implementation of a Web of Things (WoT) runtime.

ClinkZ uses **W3C Thing Description (TD)** as the semantic contract between Things, services, and applications.

It separates:
- semantic interaction models
- runtime execution
- protocol communication

## Why ClinkZ?

Modern IoT systems use many incompatible protocols:

- MQTT devices
- HTTP services
- industrial gateways
- edge nodes
- cloud applications

ClinkZ provides a common semantic runtime:

```text
Different Protocols
        |
        v
Protocol Binding Layer
        |
        v
Protocol-Neutral Runtime
        |
        v
Application
```

## Core Concepts

### Thing Description as the Contract

ClinkZ treats W3C WoT Thing Description as the primary semantic model.

A TD describes:
- Properties
- Actions
- Events
- Data schemas
- Interaction forms

### Protocol-Neutral Runtime

Applications interact with Things, not transport protocols.

Protocol-specific logic remains isolated inside Protocol Bindings.

### Servient-Orchestrated Runtime

The Servient owns:

- lifecycle management
- routing
- handler execution
- activation
- cleanup

Protocol Bindings own:

- transport communication
- encoding/decoding
- connection management
- correlation handling

A Protocol Binding never directly invokes application handlers.

```text
Protocol Traffic
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

## Planned Interaction Execution

ClinkZ compiles interaction decisions before runtime execution.

```text
Thing Description
        |
        v
Parse & Validate
        |
        v
Logical Planner
        |
        v
Immutable Plan Set
        |
        v
Binding Compilation
        |
        v
Runtime Execution
```

## Architecture

```text
Thing Description

        |
        v

Logical Planner

        |
        v

Immutable Plan Set

        |
        v

Protocol Bindings

        |
        v

Servient

        |
        +-------------+
        |             |
        v             v

 Application     Transport
 Handler         Network
```

## Platform Support

ClinkZ targets:

- Standard host environments
- Async runtime integration
- `no_std + alloc`
- Constrained environments

## Current Protocol Binding

Zenoh is the first concrete Protocol Binding.

## Workspace Structure

```text
foundation/          Resource foundations
td/                  Thing Description model
core/                Runtime contracts
planning/            Logical planning engine
protocol-bindings/   Protocol integrations
servient/            Runtime orchestration
docs/                Architecture documentation
```

## Project Status

ClinkZ is under active architectural development.

Current focus:

- v4.9 architecture implementation
- Servient lifecycle refinement
- Protocol Binding SPI stabilization
- Zenoh integration

The project is not yet a production release.

## Documentation

- docs/architecture/
- docs/design.md
- docs/ADRs/

## Build

```bash
git clone https://github.com/yushun1990/clinkz-wot.git

cd clinkz-wot

cargo test --workspace
```

## Development Philosophy

- Semantic models before protocols
- Explicit ownership over hidden behavior
- Compile-time planning over runtime guessing
- Bounded resources over unlimited assumptions
- Same architecture from cloud to constrained devices

## License

Apache License 2.0
