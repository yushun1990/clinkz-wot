# WP-700 Umbrella API Migration and Final Conformance

Status: Planned
Design revision: v4.6
Depends on: `WP-400`, `WP-500`, `WP-600`
Required gates: `GATE-1`, `GATE-2`, `GATE-3`, `GATE-4`, `GATE-5`, `GATE-6`
Owner packages: `clinkz-wot`, workspace

## Scope

Integrate the completed foundation, TD, core, planning, binding, Servient, Discovery client,
zenoh/zenoh-pico, and codec migrations into the application-facing `clinkz-wot` crate. Freeze the
intentional re-export surface, remove obsolete compatibility APIs, run the complete feature and
requirement evidence matrix, establish implementation baselines, and make the final conformance
decision.

This work package does not redesign behavior or create another implementation layer. A conflict,
missing owner, infeasible budget, or ambiguous state transition returns to design review under
`CHANGE-CONTROL-001`. Work begins only after `WP-400`, `WP-500`, and `WP-600` are complete and
all entry gates remain closed.

## Requirements

- `DOC-GOV-001`
- `ARTIFACT-AUTH-001`
- `IMPL-CONFORM-001`
- `CHANGE-CONTROL-001`
- `API-OWNERSHIP-001`
- `REFACTOR-GATE-001`
- `STD-BASELINE-001`
- `API-SURFACE-001`
- `PERF-BENCH-001`
- `PERF-BENCH-002`
- `PERF-BENCH-003`
- `PERF-BUDGET-001`
- `PERF-SCALE-001`

All other indexed requirements are transitive completion inputs. This package may aggregate their
evidence, but it cannot replace a focused package-level result with one broad integration test.

## Crates and Feature Cells

| Cargo package | Feature cell | Required surface |
| --- | --- | --- |
| `clinkz-wot` | `--no-default-features` | Useful constrained re-exports for foundation, TD, core, planning, Discovery client values/poll traits, codecs selected by feature, and `StaticServient` |
| `clinkz-wot` | `async`, no `std` | Native async adapters without an executor, host runtime, or concrete Rust zenoh dependency |
| `clinkz-wot` | `std` | Feature-composed `Servient`, host handles, Discovery client adapters, and host conveniences |
| `clinkz-wot` | `zenoh` | Optional Rust zenoh backend; implies its documented host requirements only |
| `clinkz-wot` | `zenoh-pico` | Optional constrained zenoh-pico backend without enabling `std` |
| `clinkz-wot` | `cbor` | Optional `clinkz-wot-codec-cbor` re-exports in every supported codec cell |
| `clinkz-wot` | `td2-preview` | Additive experimental TD 2.0 surface with unchanged TD 1.1 behavior when preview fields are absent |

The workspace compile matrix covers the actual Cargo packages
`clinkz-wot-foundation`, `clinkz-wot-td`, `clinkz-wot-core`,
`clinkz-wot-protocol-bindings`, `clinkz-wot-discovery`, `clinkz-wot-servient`,
`clinkz-wot-protocol-bindings-zenoh`, `clinkz-wot-codec-cbor`, and `clinkz-wot`.
Zenoh-pico is a feature of `clinkz-wot-protocol-bindings-zenoh`, not a separate package in this
revision.

## Public API and Data Migration

- Add `clinkz-wot-foundation` as a direct umbrella dependency and expose its frozen resource,
  work, time, generation, and named-profile types through an intentional module and selected
  prelude entries.
- Re-export each frozen item from its defining package and public path in
  `docs/api-ownership.csv`. The umbrella may provide a shorter path, but it must not define a
  second type, trait, registration, slot, state record, or profile with the same role.
- Replace broad accidental prelude exposure with a reviewed application-facing list covering
  documents, interaction values, handlers, Servient construction, produced/consumed handles,
  Discovery client values, resource policy, and selected optional bindings/codecs.
- Align `produce`, `consume`, `discover`, `explore_directory`, and
  `request_thing_description` with the Scripting-compatible result shapes. Keep source-aware and
  Directory-native extensions under explicitly named methods rather than changing the
  Scripting-compatible method contract.
- Add the umbrella `zenoh-pico` feature and forward it to the concrete package without `std`.
  Keep `zenoh` and `zenoh-pico` mutually exclusive and keep both off the default umbrella build.
- Forward `std`, `async`, `cbor`, and `td2-preview` only along dependency-safe edges. Feature
  unification must not make a lower crate depend on a higher crate or executor.
- Update examples, crate-level documentation, rustdoc links, and compile fixtures to use only the
  final paths. All committed technical text and error examples remain English.

## State and Ownership Migration

- Add no new lifecycle authority in the umbrella. `ExposeState`, subscription state,
  `DiscoveryProcessState`, binding route state, and operation slots remain owned by their defining
  packages and are only re-exported.
- Verify an end-to-end Producer lifecycle from form contribution through serving, emission,
  draining, and retained cleanup without an ownership handoff that is absent from
  `docs/state-machines.toml`.
- Verify an end-to-end Consumer and Directory-client lifecycle from plan selection through request
  or subscription progress, terminal status, cancellation, and cleanup with stable binding and
  slot generations.
- Verify that a source document, compiled plan, registration, route guard, payload lease,
  subscription guard, Directory slot, cleanup record, and performance result each have exactly one
  live owner at every cross-crate handoff.
- Run the lock/reentrancy and dependency inspections across the composed workspace so an umbrella
  convenience adapter cannot reintroduce a global hot-path mutex, a callback under lock, or a
  forbidden concrete transport dependency.

## Old API Removal

- Remove `ProtocolBinding` and `ClientBindingFactory` names, documentation, prelude exports, and
  examples. Applications register `ServerBindingRegistration` and
  `ClientBindingRegistration`, with explicit convenience wrapping where appropriate.
- Remove umbrella exposure of Directory backends, storage adapters, `InMemoryDirectory`,
  `LocalDiscoverer`, local Directory defaults, and all service-shaped query/publication types
  removed by `WP-500`.
- Remove exports of the old monolithic `ServerBinding::serve`/`shutdown` lifecycle, implicit
  subscription/discovery end markers, and current binding registration methods superseded by
  generation-bearing registrations and slots.
- Remove or rename ambiguous `produce(Thing)` and source-envelope methods whose result shape does
  not match their documented Scripting-compatible contract. Complete TD and source-document paths
  remain available only under their explicit names.
- Remove legacy security, payload, error, and resource aliases that define a second cross-crate
  role instead of re-exporting the frozen owner. A transitional alias may not remain enabled in a
  releasable feature cell.
- Remove stale crate documentation claiming that the default `std` feature installs Tokio or a
  concrete transport when the actual feature graph does not do so.

The old-API compile-fail suite names every removed public path and proves that no default,
no-default, async, or optional binding cell restores it.

## Evidence

- `umbrella-public-surface`: positive compile fixtures for every intended module, prelude entry,
  feature cell, and optional integration, with defining-type identity checks.
- `old-api-removal`: negative compile fixtures and source/reference inspection for every removed
  API and compatibility facade.
- `workspace-feature-matrix`: Cargo feature/dependency checks for all required package cells,
  no-std targets, optional bindings/codecs, mutual exclusion, and `td2-preview` additivity.
- `requirement-evidence-completeness`: machine-readable proof that every applicable expanded
  requirement has current focused evidence and that all artifact, ownership, state, resource,
  gate, and work-package checks pass in one revision.
- `performance-baseline-and-regression-gates`: fixture-locked numeric baselines, absolute-budget
  results, regression comparisons, and approved runner identities for every gating workload.

Release evidence also includes formatting, Clippy, rustdoc, unit, integration, round-trip,
failure-injection, no-std compile, and dependency-direction checks across the full workspace.

## Performance Workloads

- Gateway: `PERF-GW-001..019` (`PERF-GW-001` through `PERF-GW-019`).
- Directory client: `PERF-DIR-001..011` (`PERF-DIR-001` through `PERF-DIR-011`).
- Constrained: `PERF-CS-001..014` (`PERF-CS-001` through `PERF-CS-014`).

Run every workload through `tools/performance-harness` with the manifest, fixture, measurement,
profile, feature, target, toolchain, allocator, runner, and workload identities locked. Populate
the implementation-completion baselines required by `PERF-BENCH-002`; characterization workloads
remain report-only. A result must satisfy both the absolute budget or structural invariant and the
applicable regression gate. Mismatched identities form a separate series and cannot close the
reference gate.

## Completion Conditions

- `WP-400`, `WP-500`, and `WP-600` are complete; all six refactor gates remain closed; and the
  work-package DAG checker reports every predecessor complete.
- The ownership matrix matches the implemented defining and umbrella paths, contains no temporary
  owner or obsolete migration disposition, and the Cargo graph follows `CRATE-DEPS-001` in every
  feature cell.
- Every positive and negative public-surface fixture passes, including useful no-default and async
  no-std umbrella builds, optional `zenoh`, optional `zenoh-pico`, `cbor`, and additive
  `td2-preview`.
- Every applicable requirement has current focused evidence; no waiver, open design ambiguity,
  temporary nonconformance feature, or unrecorded old API remains.
- All gating performance workloads have accepted numeric baselines and pass their absolute,
  structural, resource-ledger, peak, and regression gates on the approved result series.
- Workspace formatting, Clippy, rustdoc, tests, state models, artifact checks, no-std checks, and
  old-API inspections pass from a clean tree.
- The release notes identify the intentional breaking migrations and final public paths. Only then
  may the coordinated implementation refactor be declared conforming to design revision v4.6.
