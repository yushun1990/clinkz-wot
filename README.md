# clinkz-wot

<div align="center">

# ClinkZ

### Simple Links. Infinite Possibilities.

**A protocol-neutral Rust Web of Things runtime based on W3C WoT**

[![Rust](https://img.shields.io/badge/Rust-orange?logo=rust)](#)
[![WoT](https://img.shields.io/badge/W3C-WoT-blue)](#)
[![License](https://img.shields.io/badge/license-Apache--2.0-green)](#)

</div>


## Overview

`clinkz-wot` is a protocol-neutral Rust **Web of Things (WoT) runtime**.

ClinkZ uses **W3C Thing Description (TD)** as the semantic contract between:

- Things
- edge systems
- cloud services
- applications

The goal is simple:

> Applications should understand Things, not communication protocols.


## Architecture

```text
                 Thing Description
                         |
                         v
              +--------------------+
              | Logical Planner    |
              +--------------------+
                         |
                         v
              Immutable Plan Set
                         |
                         v
              +--------------------+
              | Protocol Binding   |
              +--------------------+
                         |
                         v
              +--------------------+
              |     Servient       |
              +--------------------+
                    /        \
                   v          v
            Application     Transport
             Handler        Network
```

The runtime separates semantic interaction from protocol transport.

---

## Core Ideas

### Semantic First

W3C WoT Thing Description is the contract.

A TD defines:

- Properties
- Actions
- Events
- Schemas
- Interaction forms


### Protocol Neutral

Applications do not depend on:

- MQTT
- HTTP
- Zenoh
- WebSocket
- future protocols

Protocol-specific logic lives inside independent **Protocol Bindings**.


### Servient Orchestration

The Servient owns:

- lifecycle
- routing
- activation
- handler execution
- cleanup

Protocol Bindings own:

- network communication
- encoding/decoding
- transport state
- correlation


A Protocol Binding never directly calls application handlers.

```text
Protocol Message
       |
       v
Protocol Binding
       |
       v
Servient
       |
       v
Application
```


## Execution Model

ClinkZ moves decisions from runtime execution into planning.

```text
TD
 |
 v
Parse
 |
 v
Plan
 |
 v
Compile Binding
 |
 v
Execute
```

Runtime executes immutable plans instead of repeatedly discovering capabilities.


## Platform

Designed for:

| Environment | Support |
|---|---|
| Host systems | ✅ |
| Async runtime | ✅ |
| no_std + alloc | ✅ |
| Constrained devices | 🚧 |


## Protocol Bindings

Zenoh is the first concrete Protocol Binding.

Bindings are responsible for:

- protocol communication
- data conversion
- sessions
- correlation
- protocol-local flow control

The core runtime remains protocol independent.


## Workspace

```text
foundation/          Runtime foundations
td/                  Thing Description model
core/                Protocol-neutral contracts
planning/            Planning engine
protocol-bindings/   Protocol integrations
servient/            Runtime orchestration
docs/                Architecture and specifications
```


## Project Status

ClinkZ is under active development.

Current focus:

- v4.9 architecture implementation
- Servient lifecycle refinement
- Protocol Binding SPI stabilization
- Zenoh integration


⚠️ Not yet a production release.


## Documentation

| Document | Location |
|---|---|
| Architecture | `docs/architecture/` |
| Design | `docs/design.md` |
| Decisions | `docs/ADRs/` |


## Build

```bash
git clone https://github.com/yushun1990/clinkz-wot.git

cd clinkz-wot

cargo test --workspace
```


## Design Philosophy

- Semantic models before protocols
- Explicit ownership over hidden behavior
- Compile-time planning over runtime guessing
- Bounded resources over unlimited assumptions
- Same architecture from cloud to constrained devices


## License

Apache License 2.0
