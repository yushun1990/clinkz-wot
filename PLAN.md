# clinkz-wot Implementation Plan

## Summary

`clinkz-wot` is a protocol-neutral Rust implementation of a W3C Web of Things engine for the Clinkz platform.

The engine uses TD and TM as the semantic contract layer. Protocol bindings are pluggable adapters. Zenoh is implemented first because Clinkz Platform uses it as the default communication bus, but zenoh is not a required dependency of the engine.

The default specification target is W3C WoT TD 1.1, Architecture 1.1, Discovery, and Profile. TD 2.0 is tracked as experimental work behind a feature flag.

## Milestones

### M1: TD 1.1 Hardening

- Complete TD 1.1 data model coverage.
- Preserve unknown extension fields.
- Preserve round-trip fidelity for existing and new fixtures.
- Strengthen validation for required fields, `OneOrMany`, operations, URI references, URI templates, defaults, and security definitions.
- Add shared support for resolving `base` plus relative form `href` values.

### M2: Thing Model Support

- Add Thing Model types and builders.
- Support TM parsing, serialization, validation, and extension fields.
- Support TM inheritance and optional affordance metadata.
- Add a TM-to-TD generation path for reducing authoring repetition.

### M3: Protocol-Neutral Core

- Add core traits for exposed Things, consumed Things, interaction handlers, protocol bindings, payload codecs, security providers, and transport adapters.
- Keep the core crate compatible with `no_std + alloc`.
- Define operation dispatch by WoT semantic intent, not by protocol-specific assumptions.

### M4: Protocol Binding Core and Zenoh Binding

- Add binding traits and form selection utilities.
- Implement `clinkz-wot-binding-zenoh` as the first optional binding.
- Define Clinkz zenoh extension vocabulary under a `cz:` namespace.
- Map WoT operations to zenoh query, put, publish, subscribe, and reply patterns.
- Keep zenoh dependency out of TD/TM/core crates.

### M5: Discovery and TDD

- Implement W3C Discovery concepts: Introduction and Exploration.
- Provide Thing Description Directory repository traits.
- Add in-memory and platform-backed repository implementations.
- Support TD registration, update, delete, lookup, and query flows.
- Allow Clinkz Platform to publish discovery information over zenoh without making zenoh mandatory for the engine.

### M6: Servient Runtime

- Compose TD/TM, protocol bindings, discovery, security providers, and runtime observability.
- Support exposed and consumed Things.
- Provide platform APIs for devices, databases, external APIs, and compute tasks as Things.
- Keep host/cloud runtime features in `std` crates or behind `std` features.

### M7: Conformance and Embedded Support

- Add W3C example and fixture compatibility tests.
- Add TD 1.0/1.1 compatibility tests where practical.
- Add `no_std + alloc` checks for embedded-ready crates.
- Add embedded-oriented tests for local Thing registration, dispatch, validation, and serialization.
- Keep TD 2.0 support experimental until the specification stabilizes.

## Acceptance Criteria

- Core TD/TM documents can be parsed, validated, serialized, and round-tripped without losing extension data.
- The TD/TM/core crates compile without `std` when built with the embedded feature set.
- The engine core has no dependency on zenoh.
- The zenoh binding can be enabled as an optional crate or feature.
- Protocol bindings all use the same protocol-neutral trait surface.
- Technical documentation and comments are English-only.

## Current Repository Notes

- The existing `clinkz-wot-td` crate already starts from a `no_std` layout and uses `alloc`.
- The TD model already includes `base` and form `href`.
- The current implementation stores `base` and relative `href` values but should add a shared resolution helper for runtime and binding use.
- The current context model supports JSON-LD context objects, which can represent prefixes such as `cz`.
