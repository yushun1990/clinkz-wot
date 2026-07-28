# 0014 Property-Read Plan Artifact Boundary

Status: OPEN

## Problem

`WP-200-PROPERTY-READ-PLAN-SLICE` must prove one production
`LogicalInteractionPlan`, `BindingArtifact`, `BindingArtifactEnvelope`,
`BindingArtifactRef`, `BindingCompilerInput`, and
`BindingCompilerExtension` boundary before the later mock binding and Servient
slices can compose it. The current authority fixes the semantic ownership and
invariants, but not one constructible Rust representation shared by host-erased
and static registrations.

## Evidence

- `docs/spec/planning.md` requires an opaque immutable artifact and says its
  representation may use an erased host payload or a registered static slot,
  but does not freeze the public Rust schema or compiler trait signatures.
- `docs/spec/binding-spi.md` requires the compiler extension, artifact
  compatibility, and execution half to be installed atomically in
  `HostBindingRegistration` or `StaticBindingRegistration<B>`, while their
  complete constructor and component schemas remain unspecified.
- `docs/reviews/review-03.org` AR3-02 and AR3-06 explicitly find that an
  independent third-party binding cannot construct the complete bundle or
  determine how generic `B` supplies heterogeneous artifact/compiler state.
- `docs/work-packages/index.toml` assigns `PLAN-ARTIFACT-001` and
  `binding-compiler-extension` evidence to WP-200, and WP-300 depends on WP-200.
  However, both `WP-200-planning.md` and `WP-300-bindings.md` currently say to
  implement the same Core compiler-extension values.
- The property-read architecture gate forbids fixture-local logical-plan and
  binding-artifact adapters. A slice fixture therefore cannot hide the missing
  production representation.

The TD side is not the blocking condition. Current residual document authority
allows planning to borrow a validated bare TD or source envelope, and
`DOC-RUNTIME-001` can be proven by returning owned compact plan material that
contains no TD or lossless document.

## Required closure

Before a truthful property-read plan candidate exists:

1. freeze exact host-erased and static compiler/artifact Rust representations,
   including compiler progress, payload access, identity checks, footprints,
   and ownership on every failure;
2. reconcile WP-200 as the sole implementation owner of the Core
   compiler/artifact SPI with WP-300 as the complete-registration consumer, or
   explicitly revise the package DAG if the boundary cannot be separated;
3. validate the result with independent `std` host-erased and
   `no_std + alloc` static third-party authoring fixtures; and
4. only then define the property-read logical-plan/build-input candidate and
   its no-runtime-TD-read evidence.

Until that closure, do not create a WP-200 compile-contract root, either
planned property-read architecture fixture root, or planning/Core/TD
implementation source. The blocker is disjoint from the dependency-ready D3
foundation authority migration, which may continue.
