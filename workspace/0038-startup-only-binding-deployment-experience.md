# 0038 Startup-Only Binding Deployment Experience

Status: MIGRATED

Kind: owner-raised product-boundary and deployment-workflow investigation

Priority: MEDIUM

Target: the relationship between v1 Cargo-linked startup-only Binding composition and ClinkZ's user-facing plugin installation experience

## Scope and authority

This topic records a Project Owner concern that the safe v1 decision against runtime Rust dynamic loading may leave an unowned gap between the engine contract and the platform promise of installing a new protocol capability. It does not request unsafe Rust ABI loading or runtime unload. Codex owns the engine/platform boundary judgment.

## Repository observations

- V1 bindings are Cargo-linked crates registered before `ServientBuilder::build`.
- A running Servient cannot add, replace, remove, or unload Binding code.
- Rollout uses a new binary, process, container, firmware image, or application generation followed by readiness, cutover, drain, and shutdown.
- Rust dynamic-library ABI and arbitrary `build.rs` dependency discovery are explicitly rejected.
- The wider ClinkZ product may still present a one-click protocol installation experience through automated build and deployment.

## Questions for investigation

1. Which responsibilities belong to clinkz-wot and which belong to a ClinkZ service/application manager?
2. What stable metadata must a Binding crate expose for automated dependency selection, registration generation, compatibility checks, and rollout?
3. How are source trust, version pinning, lockfiles, feature cells, target triples, and supply-chain policy handled?
4. What readiness, health, migration, rollback, and drain evidence is required before publishing a newly compiled generation?
5. Can users update a Binding configuration without recompilation, and which changes require a new artifact generation?
6. Should this repository define only an integration contract or also provide a reference assembly tool?
7. How is the user-facing word "plugin" documented without implying runtime dynamic loading?

## Constraints

- Do not introduce arbitrary Rust `dylib` loading into v1.
- Preserve complete startup registration and generation-safe rollout.
- Do not make clinkz-wot depend on one platform deployment system.
- Keep supply-chain and rollback concerns explicit rather than hiding compilation behind a button.

## Expected decision output

Codex should define the engine/platform ownership boundary, required Binding package metadata and assembly contract, rollout and rollback expectations, terminology, and any future repository or platform work needed to make startup-only composition usable as a plugin experience.

## Decision

clinkz-wot owns the Cargo-linkable binding contract, complete runtime
registration, compatibility/resource validation, readiness, drain, status, and
generation-safe shutdown. A ClinkZ service/application manager owns package
discovery, source trust, version and checksum pinning, lockfiles, feature and
target resolution, toolchain/build isolation, artifact signing, deployment,
health observation, cutover, and rollback.

A binding package intended for automated assembly must expose build-time
metadata for crate/source identity and version, supported engine contract,
targets/profile cells/features, configuration-schema version, advertised
roles, and the generated registration-constructor hook. That manifest helps an
assembler select dependencies but is not runtime authority; the complete
registration still validates its actual binding/configuration generations,
compatibility, capabilities, and footprints.

External configuration may change without recompiling only when the already
linked crate accepts that schema. It still creates a new application/Servient
generation and follows readiness, cutover, drain, and rollback. Changing code,
crate version, Cargo feature, target, or compatibility requires a rebuild.

This repository defines the integration contract and conformance metadata, not
a platform-specific package registry or one-click assembly service. User-facing
“plugin” means an install/build/deploy workflow, not in-process dynamic loading
or hot unload.

## Migration

The boundary, metadata contract, configuration rule, and terminology are
projected into ADR-0009 and
`docs/architecture/40-protocol-binding-spi-and-deployment.md`. This topic is
`MIGRATED`.
