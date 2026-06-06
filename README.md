# clinkz-wot

`clinkz-wot` is a protocol-neutral Rust Web of Things engine for the Clinkz
platform.

The project uses W3C WoT Thing Descriptions (TD) and Thing Models (TM) as the
semantic contract layer. Protocol bindings are pluggable adapters. Zenoh is the
first concrete binding because Clinkz uses it as a default communication bus,
but zenoh is not required by the TD, TM, or protocol-neutral core crates.

## Project Status

This workspace is in an early `0.1.0` development stage.

Current implementation highlights:

- TD 1.1 modeling, serde support, explicit validation, default handling, URI
  typing, `base` plus relative form `href` resolution, and extension-field
  preservation are implemented for the current TD crate scope.
- Thing Model support has a first complete implementation in the TD crate.
- The protocol-neutral core exposes the first trait surface for local Things,
  consumed Things, protocol bindings, payload codecs, security providers, and
  transport adapters.
- Shared protocol binding utilities cover form selection, target resolution,
  selected-form validation, diagnostics, and security metadata extraction.
- The zenoh binding is an optional planning and adapter crate. It recognizes
  zenoh TD forms and `cz-zenoh` extension metadata without depending on a
  concrete zenoh runtime.
- Discovery includes an embedded-ready query model and deterministic in-memory
  Thing Description Directory.
- Servient includes embedded-ready runtime composition for discovery, local
  exposure, remote consumption, binding factories, caches, payload codecs, and
  security provider hooks.

Next focus areas are production-oriented shared transport ownership, optional
concrete zenoh runtime backends, and continued conformance plus embedded checks.

## Workspace Crates

| Crate | Path | Role | Runtime profile |
| --- | --- | --- | --- |
| `clinkz-wot-td` | `td` | TD/TM data models, builders, serde, validation, defaults, URI helpers, extension preservation. | `no_std + alloc`, `std` by default |
| `clinkz-wot-core` | `core` | Protocol-neutral engine traits and local/consumed Thing dispatch abstractions. | `no_std + alloc`, `std` by default |
| `clinkz-wot-protocol-bindings` | `protocol-bindings/core` | Shared form selection, target resolution, selected-form validation, diagnostics, and security helpers. | `no_std + alloc`, `std` by default |
| `clinkz-wot-protocol-bindings-zenoh` | `protocol-bindings/protocols/zenoh` | Optional zenoh form parsing, operation planning, metadata extraction, and injected transport boundary. | `no_std + alloc`, `std` by default |
| `clinkz-wot-discovery` | `discovery` | Protocol-neutral Discovery and Thing Description Directory traits with an in-memory backend. | `no_std + alloc`, `std` by default |
| `clinkz-wot-servient` | `servient` | Servient runtime composition for discovery, local exposure, remote consumption, caches, and injected bindings. | `no_std + alloc`, `std` by default |

## Architecture Principles

- Keep the engine protocol-neutral.
- Keep W3C WoT vocabulary separate from Clinkz extensions.
- Express Clinkz-specific metadata through JSON-LD extension namespaces such as
  `cz:` and zenoh-specific terms through `cz-zenoh:`.
- Keep zenoh-specific behavior in optional protocol binding crates.
- Keep TD, TM, and core runtime abstractions compatible with `no_std + alloc`.
- Put host/runtime capabilities, storage backends, sockets, and concrete
  protocol sessions behind `std` boundaries or separate runtime adapters.

## Quick Start

Use a Rust toolchain that supports edition 2024.

```sh
git clone git@github.com:yushun1990/clinkz-wot.git
cd clinkz-wot
cargo test --workspace
```

Run the embedded-oriented checks for crates that claim `no_std + alloc`
support:

```sh
scripts/check-embedded.sh
```

`discovery` and `servient` keep embedded and host surfaces inside their own
crates. Use the `embedded` modules for `no_std + alloc` composition and the
`host` modules, available with the default `std` feature, for future
host-only backends. The project avoids naming these modules `core` because
`clinkz-wot-core` already owns the protocol-neutral engine trait surface.

Run Clippy when changing Rust code:

```sh
cargo clippy --workspace --all-targets
```

## Minimal TD Example

```rust
use clinkz_wot_td::{
    affordance::{InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    form::Form,
    thing::Thing,
    validate::Validate,
};

fn build_lamp_td() -> Result<Thing, String> {
    let status_form = Form::read_property("/properties/status")
        .build()
        .map_err(|error| error.to_string())?;

    let status = PropertyAffordance::builder(DataSchema::string())
        .form(status_form)
        .build()
        .map_err(|error| error.to_string())?;

    let thing = Thing::builder("Lamp")
        .id("urn:dev:ops:lamp-001")
        .base("zenoh://clinkz/things/lamp-001/")
        .nosec()
        .property("status", status)
        .build()
        .map_err(|error| error.to_string())?;

    thing.validate().map_err(|error| error.to_string())?;
    Ok(thing)
}
```

## Protocol Bindings

Protocol bindings consume TD forms and map them to concrete transport behavior.
The shared binding crate handles protocol-neutral concerns such as operation
matching, affordance-level form lookup, `base` plus `href` target resolution,
selected-form validation, and security metadata extraction.

The zenoh crate currently acts as a planning and adapter layer. It supports
`zenoh://` targets and `cz-zenoh:keyExpr` metadata, maps WoT operations to
zenoh operation kinds, and exposes an injected `ZenohTransport` boundary for
host or test integrations. Concrete Rust `zenoh` and `zenoh-pico` runtime
backends should remain optional and feature-gated or crate-separated.

## Documentation

- [Implementation plan](PLAN.md)
- [Technical specification](docs/technical-spec.md)
- [WoT compliance notes](docs/wot-compliance.md)
- [Protocol bindings](docs/protocol-bindings.md)
- [TD API convenience surface](docs/td-api-convenience.md)
- [TD 1.1 field coverage](docs/td-1.1-field-coverage.md)
- [no_std and embedded support](docs/no-std-embedded.md)
- [Clinkz platform context](docs/clinkz-platform-context.md)
- [TD/TM development plan](docs/plan/wot-td-development-plan.md)
- [Protocol binding development plan](docs/plan/protocol-bindings-development-plan.md)

## License

This project is licensed under the MIT License. Portions of the software are
derived from `wot-td`. See [LICENSES/MIT.txt](LICENSES/MIT.txt) for details.
