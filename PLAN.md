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

Current status:

- Started `clinkz-wot-core` as a `no_std + alloc` workspace crate.
- Added the first protocol-neutral trait surface for payload codecs, exposed
  Things, consumed Things, protocol bindings, security providers, and transport
  adapters.
- Added protocol-neutral local Thing handler traits and a reusable dispatcher
  for property reads/writes, action invocation, and event subscription.
- Added a protocol-neutral consumed Thing dispatcher that validates selected
  affordance forms against TD effective operations and routes requests to
  matching bindings.
- Kept form selection, target URI resolution, and concrete protocol behavior
  outside the core crate for the later protocol-bindings milestone.
- Verified `clinkz-wot-core` with `cargo check -p clinkz-wot-core
  --no-default-features`.

Entry criteria:

- TD/TM public types expose effective operation, target, and security metadata
  needed by protocol binding consumers.
- The core trait surface remains independent of zenoh and other concrete
  transports.

### M4: Protocol Bindings and Zenoh Binding

Add shared binding utilities and implement zenoh as the first optional protocol
binding without making it a dependency of TD, TM, or core runtime crates.

Current status:

- Organized protocol binding crates under `protocol-bindings/`.
- Started `clinkz-wot-protocol-bindings` as a `no_std + alloc` workspace crate
  for shared protocol binding utilities.
- Added shared form selection based on TD effective operations and affordance
  context.
- Added shared form target resolution using the TD crate's `base` plus `href`
  helper.
- Added shared affordance-level form selection and target resolution for Thing,
  property, action, and event forms, including unknown-affordance errors.
- Added protocol-neutral form selection criteria for content type and
  subprotocol matching while preserving the existing operation-only selection
  API.
- Kept zenoh and other concrete protocol behavior out of
  `clinkz-wot-protocol-bindings`.
- Started `clinkz-wot-protocol-bindings-zenoh` as the first optional concrete
  protocol binding crate under `protocol-bindings/protocols/zenoh`.
- Added first-pass zenoh form support that recognizes `zenoh://` form targets
  and `cz-zenoh:keyExpr` extension metadata, extracts key expressions, and
  implements the shared `ProtocolBinding` support check without introducing a
  required zenoh runtime dependency.
- Added zenoh operation planning that maps WoT form operations to
  transport-level zenoh operation kinds while still avoiding a required zenoh
  runtime dependency.
- Extended zenoh operation planning with first-pass Clinkz extension metadata
  parsing for encoding, QoS, priority, and congestion control hints.
- Added shared predicate-based form selection and a zenoh affordance operation
  planner so concrete bindings can choose protocol-supported forms from
  multi-form affordances before runtime transport execution is wired in.
- Extended the zenoh affordance planner to accept shared form selection
  criteria, with zenoh planner coverage for content type and subprotocol
  filters.
- Improved shared form selection diagnostics so operation mismatches are
  distinguished from metadata or caller-filter mismatches.
- Added an injected zenoh transport adapter boundary to the generic
  `ZenohBinding<T>` so planned zenoh operations can be executed by host or test
  integrations without adding a required zenoh runtime dependency.

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
