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
- M3 protocol-neutral core has the first trait and dispatcher surface needed by
  binding crates.
- M4 protocol binding hardening is complete for the current shared and zenoh
  binding scope.
- M5 Discovery has the first embedded-ready in-memory Thing Description
  Directory and query surface needed by runtime crates.
- M6 Servient runtime composition now has the first embedded-ready runtime
  surface that wires Discovery, local Things, consumed Things, injected
  protocol bindings, runtime registries, TD/form/binding-plan caches, payload
  codecs, and security providers.
- Keep M7 conformance and embedded checks running across every crate that
  claims `no_std + alloc` support.

Immediate next sequence:

1. Keep M7 checks and compatibility documentation aligned with the current
   TD/TM, core, protocol binding, Discovery, and Servient surfaces.
2. Start the next concrete runtime/backend increment only after documenting the
   acceptance target and keeping the existing embedded checks green.

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

Plan: `docs/plan/protocol-bindings-development-plan.md`.

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
- Added shared validation for caller-selected affordance forms and wired zenoh
  runtime invocation to reject forms that do not belong to the requested
  affordance or do not support the requested effective operation before
  transport execution.
- Added shared protocol-neutral helpers for selected-form security references
  and scopes, including inherited Thing-level security, form-level overrides,
  and nosec coverage.
- Documented the first Clinkz zenoh extension vocabulary, including stable
  `cz-zenoh:keyExpr`, experimental metadata hints, string validation, and
  `keyExpr` precedence over `href`.
- Expanded zenoh planning coverage for Thing-level forms, bulk property and
  event operations, content type and subprotocol criteria, and relative `href`
  values resolved against a zenoh `base`.
- Kept concrete zenoh execution optional behind `ZenohTransport`, with tests
  for fake transport propagation and the default no-transport error path.
- Stabilized shared selected-form validation coverage for Thing-level forms,
  property forms, action defaults, event defaults, copied selected form values,
  and selected-form mismatch cases.
- Finalized shared binding diagnostics with distinct errors for unknown
  affordances, unsupported operations, metadata mismatches, caller-filter
  mismatches, target resolution failures, and selected forms outside the
  requested affordance.
- Documented the runtime backend policy for future Rust `zenoh` and
  `zenoh-pico` adapters: the current zenoh planning crate remains
  `no_std + alloc`, while concrete runtime backends stay optional and
  feature-gated or crate-separated.
- Added the first concrete Rust `zenoh` host runtime backend behind the
  explicit `runtime-zenoh` feature while keeping the default and
  `--no-default-features` builds free of a concrete zenoh runtime dependency.
- Hardened the Rust `zenoh` host runtime backend with request/reply selector
  parameter validation, subscription lifecycle metadata and undeclaration, and
  first-pass metadata mapping for encoding, express QoS, priority, and
  congestion control.
- Added a std-only shared zenoh transport handle so Servient binding factories
  can reuse a session, pool, or runtime adapter across cloned bindings.

Completion notes:

- `clinkz-wot-protocol-bindings` and
  `clinkz-wot-protocol-bindings-zenoh` pass `cargo test`.
- Both protocol binding crates pass `cargo check --no-default-features`.
- Zenoh-specific code remains outside TD, TM, and core crates.

Exit criteria:

- `clinkz-wot-protocol-bindings` and
  `clinkz-wot-protocol-bindings-zenoh` pass `cargo test`.
- Both protocol binding crates pass `cargo check --no-default-features`.
- Shared utilities cover form selection, target resolution, selected-form
  validation, diagnostics, and security metadata extraction needed by runtime
  crates.
- Zenoh-specific code remains outside TD, TM, and core crates.

### M5: Discovery and TDD

Implement W3C Discovery concepts and Thing Description Directory behavior for
registration, lookup, update, deletion, and query flows.

Current status:

- Started `clinkz-wot-discovery` as a workspace crate.
- Added protocol-neutral Thing Description Directory traits for registration,
  retrieval, update, deletion, listing, and query.
- Added backend-portable structured directory queries with exact-match filters,
  conjunctive matching, pagination metadata, and owned result entries suitable
  for memory, SQL, RDF/SPARQL, HTTP, or other runtime backends.
- Added a deterministic in-memory directory backend with configurable TD
  validation level and a local predicate-query convenience API for tests and
  host-local filtering.
- Added focused tests for duplicate registration, update behavior, deletion,
  lookup by id, deterministic listing, structured query predicates, pagination,
  missing ids, validation failures, and owned result cloning.

Entry criteria:

- M4 shared binding APIs are stable enough for Discovery and Servient crates to
  refer to TD forms without duplicating form selection or target resolution.
- TD validation exposes the Basic checks needed to reject invalid directory
  entries explicitly instead of during deserialization.

Planned work:

- Add `clinkz-wot-discovery` as a workspace crate with `no_std + alloc`
  embedded APIs and `std` host extension points.
- Define protocol-neutral directory traits for registration, retrieval, update,
  deletion, listing, and query.
- Implement a deterministic in-memory directory backend first.
- Keep concrete production storage backends separate from the shared Discovery
  query model, following the same adapter boundary used by protocol bindings.
- Validate TD inputs explicitly at configurable validation levels before
  registration and update.
- Preserve full TD round-trip data, including unknown extension fields and
  JSON-LD contexts, through directory storage.
- Add tests for duplicate registration policy, overwrite/update behavior,
  deletion, lookup by id, listing, and basic query predicates.

Exit criteria:

- The discovery crate has focused tests for CRUD and query behavior.
- Discovery does not depend on zenoh or any concrete binding.
- Directory storage keeps TD/TM semantic data and extension fields intact.

### M6: Servient Runtime

Compose TD/TM, protocol bindings, discovery, security, and observability into a
host/runtime Servient that supports exposed and consumed Things.

Current status:

- Started `clinkz-wot-servient` as a workspace crate with embedded-ready
  runtime composition and `std` host extension points.
- Added a host Servient builder backed by an injectable Thing Directory and
  protocol binding factories.
- Added lifecycle APIs for start and stop.
- Defined first lifecycle semantics: `start` and `stop` are idempotent, while
  directory, exposed Thing, and binding factory mutations are rejected while the
  Servient is running.
- Added directory APIs for register, update, unregister, list, and query.
- Added local Thing exposure and unexposure flows that keep the directory in
  sync with exposed TDs.
- Extracted the in-memory exposed Thing map behind an injectable registry
  boundary so runtime services can replace local Thing storage without changing
  TD, Discovery, or core crates.
- Added Servient-level dispatch APIs for property reads, property writes, action
  invocation, and event subscription on locally exposed Things.
- Added consumed Thing creation from directory entries or direct TDs, with
  registered protocol bindings injected into each consumed dispatcher.
- Added Servient-level consumed Thing convenience APIs for remote property
  reads, property writes, action invocation, and event subscription when callers
  already have a selected form.
- Added Servient-level consumed Thing convenience APIs that select remote
  property, action, and event forms from shared `FormSelectionCriteria` while
  preserving selected-form calls for callers that cache form choices.
- Added post-build protocol binding factory registration for runtime
  composition flows that cannot provide all bindings at builder construction
  time.
- Added a boxed `ProtocolBinding` forwarding implementation in core so runtime
  crates can pass protocol-neutral binding instances without knowing concrete
  binding types.
- Added an injectable consumed Thing TD cache boundary in Servient, with a
  deterministic in-memory default and synchronization from register, update,
  expose, unregister, and unexpose flows.
- Added an injectable selected form cache boundary in Servient, with a
  deterministic in-memory default, cache invalidation on TD lifecycle
  mutations, and criteria-based remote invocation reuse.
- Added an injectable binding plan cache boundary in Servient, with a
  deterministic in-memory default, cache invalidation on TD lifecycle
  mutations, and criteria-based remote invocation reuse of both selected TD
  forms and selected protocol binding factories.
- Added Servient builder and post-build slots for protocol-neutral payload
  codecs and security providers, with hooks for local interactions and
  Servient-level consumed Thing calls.
- Split Servient implementation into focused builder, cache, error, registry,
  runtime, and interaction modules while keeping the crate-root public API
  stable.
- Added integration tests for exposing a local Thing, consuming a discovered TD,
  dispatching all local interaction kinds, and invoking through an injected test
  binding.
- Added runtime integration coverage for remote property writes, remote action
  invocation, remote event subscription, late binding factory registration,
  unknown Thing ids, and missing binding diagnostics.
- Added runtime integration coverage for consumed TD cache synchronization,
  cache-preferred consumption, directory update, and unregister flows.
- Added a Servient integration test that routes remote property reads, property
  writes, action invocation, and event subscription through the optional zenoh
  binding with an injected fake transport, keeping zenoh out of Servient's
  required dependencies.
- Added shared zenoh transport ownership support so binding factories can reuse
  host sessions or connection pools without requiring concrete protocol types
  in Servient.

Entry criteria:

- M5 provides a usable directory abstraction and in-memory backend.
- Core consumed/exposed Thing dispatch can route through protocol bindings with
  validated forms.

Planned work:

- Keep low-level `BoundConsumedThing` access available for callers that need to
  cache TDs, selected forms, or dispatchers directly.

Exit criteria:

- Servient runtime can expose and consume Things through protocol-neutral core
  traits.
- Zenoh remains optional and can be omitted from Servient builds.
- Runtime integration tests cover discovery plus binding dispatch.

### M7: Conformance and Embedded Support

TD/TM plan: `docs/plan/wot-td-development-plan.md`.

Add W3C compatibility checks, fixture coverage, and embedded-oriented
`no_std + alloc` verification for crates that claim embedded support.

Planned work:

- Keep TD/TM round-trip fixtures aligned with TD 1.1 field coverage and
  extension preservation requirements.
- Add workspace-level verification documentation for `cargo test` and
  `--no-default-features` checks.
- Add no-default-features checks for every embedded-ready crate as it is added
  or changed.
- Keep `scripts/check-embedded.sh` aligned with every crate that claims
  `no_std + alloc` support.
- Add conformance-oriented fixtures for multi-form affordances, form security,
  `base` plus relative `href`, JSON-LD context preservation, and Clinkz
  extension terms.
- Keep TD 2.0 behavior behind an experimental feature and out of default
  compatibility expectations.

Current status:

- `scripts/check-embedded.sh` covers every current workspace crate that claims
  `no_std + alloc` support: TD, core, shared protocol bindings, zenoh binding,
  Discovery, and Servient.
- The current protocol binding M7 verification path passes:
  - `cargo fmt --check`
  - `cargo test -p clinkz-wot-protocol-bindings -p clinkz-wot-protocol-bindings-zenoh`
  - `scripts/check-embedded.sh`

Exit criteria:

- All crates that claim embedded support pass no-default-features checks.
- Public fixtures cover the TD/TM, core dispatch, binding selection, discovery,
  and Servient flows needed for a first compatibility baseline.
- No protocol-specific behavior leaks into TD/TM/core crates.

## Acceptance Criteria

- Core TD/TM documents can be parsed, validated, serialized, and round-tripped without losing extension data.
- The TD/TM/core crates compile without `std` when built with the embedded feature set.
- The engine core has no dependency on zenoh.
- The zenoh binding can be enabled as an optional crate or feature.
- Protocol bindings all use the same protocol-neutral trait surface.
- Technical documentation and comments are English-only.
