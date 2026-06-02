# wot-td Subplan

## Parent Plan Relationship

This document is a crate-level subplan under the repository-level
`PLAN.md`. It refines the parts of the main plan that belong specifically to
`clinkz-wot-td`.

Parent milestones covered by this subplan:

- M1: TD 1.1 Hardening.
- M2: Thing Model Support.
- M7: Conformance and Embedded Support, only for checks and fixtures owned by
  the TD/TM crate.

Parent milestones not covered by this subplan:

- M3: Protocol-Neutral Core.
- M4: Protocol Binding Core and Zenoh Binding.
- M5: Discovery and TDD.
- M6: Servient Runtime.

## Scope

`clinkz-wot-td` is the protocol-neutral crate for Thing Description and Thing
Model data structures, builders, serialization, deserialization, validation,
and extension preservation.

The crate must remain compatible with `no_std + alloc`. It must not depend on
networking, async runtimes, filesystems, zenoh, databases, process APIs, or
other host-only runtime facilities.

Protocol behavior belongs in binding crates. Discovery, Servient runtime
composition, and operation dispatch belong in dedicated crates and are only
mentioned here when TD/TM data model choices must support them later.

## Current Baseline

The current TD crate already provides:

- A `no_std` crate layout with `alloc`.
- TD data structures for Things, affordances, forms, links, security schemes,
  and data schemas.
- Builder APIs for common TD structures.
- Serde round-trip support over the existing fixture corpus.
- Unknown extension field preservation through extension maps.
- Field-specific URI types:
  - `UriReference` for RFC 3986 URI references.
  - `FormHref` for form targets that may be URI references or URI templates.
  - `AbsoluteUri` for fields requiring absolute URIs.
  - `BaseUri` for Thing-level base values, including absolute URI templates.
- `cargo check -p clinkz-wot-td --no-default-features` passes after replacing
  the `Thing.created` and `Thing.modified` RFC3339 serde adapter with the
  `time` crate's `no_std + alloc` compatible RFC3339 option helper.

Known baseline gaps:

- Validation is present but not yet separated into explicit validation levels.
- TD 1.1 model coverage has not yet been audited field by field.
- Thing Model support has not yet been introduced.

## TD-P0: Stabilize the Foundation

### TD-P0.1 Fix `no_std + alloc` Compilation

Status: complete.

Goal: the TD crate must compile without default features.

Required check:

```sh
cargo check -p clinkz-wot-td --no-default-features
```

Work items:

- Replace the current `serde_as(as = "Option<Rfc3339>")` usage for
  `Thing.created` and `Thing.modified`.
- Add crate-local serde helpers or a TD-specific date-time newtype that works
  under `no_std + alloc`.
- Keep public error messages in English.
- Add the no-default-features check to the regular verification path.

Acceptance criteria:

- `cargo test -p clinkz-wot-td` passes.
- `cargo check -p clinkz-wot-td --no-default-features` passes.
- No `std` imports are introduced in the TD crate outside `#[cfg(feature =
  "std")]` sections.

Completion notes:

- `Thing.created` and `Thing.modified` now use
  `time::serde::rfc3339::option` with serde defaults, preserving optional field
  behavior while avoiding the `serde_with` RFC3339 adapter that failed without
  default features.
- Verified with:
  - `cargo fmt --check`
  - `cargo check -p clinkz-wot-td --no-default-features`
  - `cargo test -p clinkz-wot-td`

### TD-P0.2 Finish URI Type Model Cleanup

Goal: URI type names and invariants must match the TD field semantics.

Current intended mapping:

- `Thing.id`: `AbsoluteUri`
- `Thing.support`: `AbsoluteUri`
- `Thing.base`: `BaseUri`
- `Thing.profile`: `Vec<AbsoluteUri>`
- JSON-LD context URI entries: `AbsoluteUri`
- Security endpoint fields: `AbsoluteUri`
- `Form.href`: `FormHref`
- `Link.href`: `UriReference`
- `Link.anchor`: `UriReference`

Work items:

- Keep `AnyUri` out of public field types.
- Keep `AbsoluteUri` strict during both parsing and deserialization.
- Keep `BaseUri` strict enough to reject relative references while allowing
  absolute URI templates seen in real TD documents.
- Add focused fixtures for URI field behavior.

Acceptance criteria:

- URI constraint tests cover absolute URI, base URI, form href, and link href.
- Round-trip fixture tests continue to pass.
- No field uses a wider URI type than its TD semantics require.

### TD-P0.3 Stop Silent Builder Error Loss

Status: complete.

Goal: builders must not silently ignore invalid typed values.

Current risk:

Some builder methods parse strings and call `.ok()`, which drops invalid input
without surfacing an error.

Work items:

- Audit all builder methods that parse `AbsoluteUri`, `BaseUri`,
  `UriReference`, `FormHref`, and other constrained types.
- Prefer storing pending raw inputs and returning errors from `build()`, or
  introduce fallible builder methods where a local pattern already supports it.
- Add tests proving invalid builder input produces an error.

Acceptance criteria:

- Invalid URI values cannot be silently omitted by builder APIs.
- Existing successful builder flows remain ergonomic.

Completion notes:

- `ThingBuilder` now records invalid `id`, `support`, `base`, and `profile`
  URI inputs and returns the first error from `build()`.
- `ContextBuilder` now returns `Result<Context, ValidateError>` so invalid
  context URIs are reported.
- `LinkBuilder` now parses `anchor` during `build()` together with `href`
  instead of discarding invalid anchor input.
- Security scheme builders now return `Result<_, ValidateError>` and report
  invalid `proxy`, `authorization`, `token`, and `refresh` URI inputs.
- Added focused builder tests for invalid URI inputs.

## TD-P1: Harden TD 1.1

### TD-P1.1 TD 1.1 Field Coverage Audit

Goal: confirm that the Rust model covers W3C WoT TD 1.1 with correct field
types and extension preservation.

Audit targets:

- `Thing`
- `InteractionAffordance`
- `PropertyAffordance`
- `ActionAffordance`
- `EventAffordance`
- `Form`
- `Link`
- `SecurityScheme`
- `DataSchema`
- `ExpectedResponse`
- `AdditionalExpectedResponse`
- `VersionInfo`
- JSON-LD `@context`

For each field, record:

- TD term name.
- Rust field name.
- Rust type.
- Whether it is required.
- Whether `OneOrMany` applies.
- Whether defaults apply.
- Whether extension fields are preserved.
- Validation requirements.

Acceptance criteria:

- A TD 1.1 coverage matrix exists in `docs/` or crate-level developer notes.
- Missing or weakly typed fields are tracked as explicit follow-up tasks.

### TD-P1.2 Explicit Validation Levels

Goal: keep parsing tolerant and make validation explicit.

Validation levels:

- `Minimal`: serde shape and basic document structure.
- `Basic`: TD required fields, type constraints, URI constraints, operation
  context, security references, defaults, and `OneOrMany` semantics.
- `Profile`: WoT Profile compatibility checks.
- `Full`: semantic checks where practical.

Work items:

- Add a validation API that takes a validation level or profile.
- Keep deserialization independent from strong validation.
- Preserve unknown extension fields unless the selected validation mode rejects
  them explicitly.
- Expand validation errors without coupling them to runtime or protocol
  concepts.

Acceptance criteria:

- Existing `Validate` behavior is either mapped to a default level or replaced
  with a level-aware API.
- Tests cover operation constraints per affordance type.
- Tests cover security name references against `securityDefinitions`.

### TD-P1.3 Round-Trip and Fixture Expansion

Goal: protect compatibility as the model becomes stricter.

Required fixture cases:

- `base` plus relative form `href`.
- Absolute URI template `base`.
- Form `href` URI templates.
- Link `href` URI references.
- JSON-LD context as string, array, and object entries.
- Unknown extension fields at Thing, affordance, form, schema, and security
  levels.
- Clinkz extension namespace entries such as `cz:`.
- Single-value and array `OneOrMany` forms.
- Form-level security override and Thing-level security inheritance.
- Multiple forms per affordance.

Acceptance criteria:

- Round-trip fixture tests preserve unknown terms and compact forms.
- New targeted fixtures fail when field-specific URI constraints regress.

## TD-P2: Complete Data Schema and Security Semantics

### TD-P2.1 DataSchema TD Subset

Goal: model the TD 1.1 DataSchema vocabulary accurately without pretending to
be a full JSON Schema implementation.

Work items:

- Audit scalar, object, array, numeric, string, enum, const, and composition
  fields.
- Preserve extension fields.
- Validate obvious constraint conflicts at the `Basic` level.
- Avoid adding a full JSON Schema validator unless a later milestone requires
  it.

### TD-P2.2 Security Scheme Validation

Goal: make security metadata structurally reliable while remaining
protocol-neutral.

Work items:

- Validate scheme-specific required fields.
- Validate `oneOf` and `allOf` references in combo schemes.
- Validate OAuth2 endpoint URI fields using `AbsoluteUri`.
- Keep protocol-specific behavior out of the TD crate.

## TD-P3: Thing Model Support

Goal: add Thing Model support after TD 1.1 parsing, serialization, URI typing,
and validation are stable.

Work items:

- Add Thing Model data structures.
- Add Thing Model builders.
- Support TM parsing, serialization, validation, and extension fields.
- Add TM fixture round-trip tests.
- Add a later TM-to-TD generation path for platform tooling.

Non-goals for the first TM pass:

- Network fetching of referenced models.
- Full JSON-LD expansion.
- Protocol-specific form generation.

## TD-P4: Prepare Binding-Core Consumers

Goal: expose clean TD data types that binding crates can consume without
duplicating field semantics.

The TD crate should not implement binding behavior. Later binding-core work
should consume TD types for:

- Form selection.
- Operation-to-form matching.
- Target URI resolution from `base` plus `href`.
- Security inheritance resolution.
- Content type and subprotocol selection.

Acceptance criteria:

- TD public types make form and link targets unambiguous.
- No zenoh, HTTP, CoAP, MQTT, or other protocol behavior leaks into the TD
  crate.

## Recommended Next Tasks

1. Add the TD 1.1 field coverage matrix.
2. Introduce validation levels.
3. Expand targeted fixtures for URI, context, security, defaults, and
   `OneOrMany`.
4. Start Thing Model support only after TD 1.1 hardening is stable.
