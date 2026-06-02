# clinkz-wot Implementation Plan

## Summary

`clinkz-wot` is a protocol-neutral Rust implementation of a W3C Web of Things engine for the Clinkz platform.

The engine uses TD and TM as the semantic contract layer. Protocol bindings are pluggable adapters. Zenoh is implemented first because Clinkz Platform uses it as the default communication bus, but zenoh is not a required dependency of the engine.

The default specification target is W3C WoT TD 1.1, Architecture 1.1, Discovery, and Profile. TD 2.0 is tracked as experimental work behind a feature flag.

## Scope

This file is the repository-level blueprint and milestone index. It defines the
intended delivery sequence and repository-wide acceptance criteria.

Detailed development tasks belong in module plans under `docs/plan/`. Milestone
sections reference those module plans when they exist.

## Milestones

Current focus:

- TD 1.1 hardening in M1 is complete for the current crate scope.
- Thing Model support has a first complete TD-crate implementation.
- Start M3 protocol-neutral core now that the TD/TM contract surface is
  reliable enough for runtime and binding crates to consume.

### M1: TD 1.1 Hardening

Plan: `docs/plan/wot-td-development-plan.md`.

Harden the TD 1.1 data model, validation, extension preservation, and
round-trip compatibility in `clinkz-wot-td`.

Current status:

- Foundation work is complete for `no_std + alloc`, URI field typing, builder
  error reporting, validation levels, security reference checks, field coverage
  audit, and shared `base` plus form `href` target resolution.
- Fixture expansion, shared TD default helpers, DataSchema Basic validation,
  and SecurityScheme Basic validation are complete.

### M2: Thing Model Support

Plan: `docs/plan/wot-td-development-plan.md`.

Status: complete for the first TD-crate pass.

Add Thing Model modeling, validation, extension preservation, and a future path
from reusable TM templates to concrete TD documents.

Entry criteria:

- M1 fixture expansion and TD Basic validation hardening are complete.
- TM data structures can reuse TD component patterns without changing protocol
  boundaries or adding `std`-only dependencies.

Completion notes:

- Added `ThingModel` data structures and builders in `clinkz-wot-td`.
- TM parsing, serialization, validation, and extension preservation are covered
  by focused tests.
- TM support compiles under the same `no_std + alloc` check as TD.

### M3: Protocol-Neutral Core

Define the protocol-neutral engine trait surface for exposed Things, consumed
Things, interaction handlers, bindings, payload codecs, security providers, and
transport adapters.

Entry criteria:

- TD/TM public types expose effective operation, target, and security metadata
  needed by binding-core consumers.
- The core trait surface remains independent of zenoh and other concrete
  transports.

### M4: Protocol Binding Core and Zenoh Binding

Add shared binding utilities and implement zenoh as the first optional protocol
binding without making it a dependency of TD, TM, or core runtime crates.

### M5: Discovery and TDD

Implement W3C Discovery concepts and Thing Description Directory behavior for
registration, lookup, update, deletion, and query flows.

### M6: Servient Runtime

Compose TD/TM, protocol bindings, discovery, security, and observability into a
host/runtime Servient that supports exposed and consumed Things.

### M7: Conformance and Embedded Support

TD/TM plan: `docs/plan/wot-td-development-plan.md`.

Add W3C compatibility checks, fixture coverage, and embedded-oriented
`no_std + alloc` verification for crates that claim embedded support.

## Acceptance Criteria

- Core TD/TM documents can be parsed, validated, serialized, and round-tripped without losing extension data.
- The TD/TM/core crates compile without `std` when built with the embedded feature set.
- The engine core has no dependency on zenoh.
- The zenoh binding can be enabled as an optional crate or feature.
- Protocol bindings all use the same protocol-neutral trait surface.
- Technical documentation and comments are English-only.
