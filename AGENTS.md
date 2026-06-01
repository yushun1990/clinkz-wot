# Agent Guidance

This repository implements `clinkz-wot`, a Rust Web of Things engine for the Clinkz platform.

## Language Policy

- All technical specifications, Rust doc comments, inline comments, public API documentation, examples, and error messages must be written in English.
- Product discussions may happen in other languages, but committed technical artifacts should stay English-only.

## Architecture Boundaries

- Keep the engine protocol-neutral.
- Do not add zenoh-specific logic to TD, TM, or core runtime crates.
- Treat zenoh as the first optional protocol binding, not as a required engine dependency.
- Keep W3C WoT vocabulary separate from Clinkz extensions.
- Use a Clinkz JSON-LD namespace, such as `cz:`, for Clinkz-specific binding, storage, compute, or platform metadata.

## no_std Policy

- TD, TM, and core runtime abstractions must support `no_std + alloc`.
- Avoid filesystem, sockets, threads, async runtimes, process APIs, and OS-only APIs in `no_std` crates.
- Put host/cloud runtime functionality behind `std` features or in separate `std` crates.
- Embedded support means TD/TM construction, serialization, validation, and local Thing dispatch with abstract transport adapters.

## W3C Compatibility

- Use W3C WoT TD 1.1 as the default compliance target.
- Keep TD 2.0 work behind an experimental feature such as `td2-preview`.
- Preserve unknown extension fields during deserialization and serialization.
- Preserve round-trip fidelity for TD/TM documents unless a validation mode explicitly rejects them.
- Support `base` plus relative form `href` values; binding implementations should resolve form targets through a shared helper instead of duplicating resolution logic.

## Implementation Style

- Prefer the existing crate and module patterns before adding new abstractions.
- Keep TD/TM crates focused on data models, builders, serialization, deserialization, and validation.
- Put protocol behavior in binding crates.
- Put Discovery and Servient/runtime behavior in dedicated crates.
- Use feature flags to separate embedded, experimental, and host/runtime capabilities.

## Testing Expectations

- Add `no_std + alloc` compile checks for crates that claim embedded support.
- Keep round-trip fixture tests for TD/TM compatibility.
- Test protocol bindings separately from protocol-neutral core logic.
- Add fixtures with multiple forms per affordance to verify protocol-neutral selection behavior.
