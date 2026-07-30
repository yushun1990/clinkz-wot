# 0038 Startup-Only Binding Deployment Experience

Status: OPEN

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