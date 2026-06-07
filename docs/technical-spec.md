# Technical Specification

## Crate Layout

### `clinkz-wot-td`

Path: `td`.

Owns TD and TM data structures, builders, serialization, deserialization, validation, and extension preservation.

This crate must remain `no_std + alloc` compatible. It must not depend on networking, async runtimes, zenoh, databases, filesystems, or operating-system APIs.

### `clinkz-wot-core`

Path: `core`.

Defines protocol-neutral engine traits and local runtime abstractions.

Expected responsibilities:

- Exposed and consumed Thing abstractions.
- Property, action, and event handler traits.
- Protocol binding traits.
- Payload codec traits.
- Security provider traits.
- Transport adapter traits.

This crate should also support `no_std + alloc`.

### `clinkz-wot-protocol-bindings`

Path: `protocol-bindings/core`.

Defines common protocol binding utilities:

- Form selection.
- Operation-to-form resolution.
- Target URI resolution from `base` plus `href`.
- Shared binding error types.

This crate should avoid protocol-specific behavior.

### `clinkz-wot-protocol-bindings-zenoh`

Path: `protocol-bindings/protocols/zenoh`.

Implements the first concrete binding because Clinkz Platform uses zenoh as its default communication bus.

This crate is optional and must not be required by TD/TM/core crates.

### `clinkz-wot-discovery`

Implements W3C WoT Discovery concepts and Thing Description Directory behavior.

This crate supports `no_std + alloc` for the protocol-neutral query model,
directory traits, and deterministic in-memory directory. The `local` module
contains no-std local directory capabilities. The `storage` module is available
only with the `std` feature for shared storage adapters and future production
storage extension points.

### `clinkz-wot-servient`

Composes TD/TM, bindings, discovery, security, and runtime services into a usable WoT Servient.

This crate supports `no_std + alloc` for runtime composition through the crate
root. Concrete std-only sessions, filesystems, async runtimes, databases, and
observability integrations should stay behind the crate's `std` feature.

## Feature Policy

- `default = ["std"]` may be used for std runtime and cloud convenience.
- `alloc` enables dynamic data structures in `no_std` environments.
- `std` enables networking, filesystems, async runtimes, integration tests, and richer diagnostics.
- `td2-preview` enables experimental TD 2.0 tracking.

Crates that expose both embedded-ready and std-only surfaces should keep both
surfaces in the same crate when the split is only a feature boundary. Use
module names that describe the capability or backend rather than naming a
module solely after `std` or `no_std` availability. Avoid a module named `core`
because `clinkz-wot-core` already names the protocol-neutral engine trait
crate.

## Validation Levels

- Minimal validation: serde shape and basic document structure.
- Basic validation: TD/TM required fields, type constraints, operation context, URI references, URI templates, default handling, and `OneOrMany`.
- Profile validation: WoT Profile compatibility checks.
- Full validation: semantic and behavioral assertions where practical.

Validation should be explicit. Deserialization should not reject documents merely because a stronger validation profile would reject them.

## Serialization Policy

- Preserve unknown extension fields.
- Preserve JSON-LD context entries.
- Preserve compact `OneOrMany` forms semantically.
- Default serialization should target TD 1.1.
- TD 2.0 serialization should be gated behind an experimental feature.

## Error Policy

- Public errors must be stable enough for downstream users.
- Error messages must be written in English.
- Protocol-specific error details belong in binding crates.
- TD/TM validation errors should not depend on runtime or transport concepts.
