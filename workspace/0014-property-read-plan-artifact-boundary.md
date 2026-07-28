# 0014 Property-Read Plan Artifact Boundary

Status: MIGRATED

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

## Decision

Use one generic, portable Core compiler contract with associated `Cursor` and
`Artifact` types. Its step operation consumes the cursor and returns exactly
one of:

- `Pending(cursor)`;
- `Complete(BindingCompilerOutput<artifact>)`; or
- `Failed(BindingCompilerFailure<cursor>)`.

The cursor therefore has one owner on every branch. Compiler artifacts are
pure immutable memory: they create no socket, task, route, cleanup record, or
other external obligation.

The two deployment representations preserve that semantic contract without
pretending that they have the same storage mechanism:

- constrained applications use
  `StaticBindingCompilerRegistration<C>` and an application-closed compiler
  enum; its associated cursor and artifact types are matching closed sum
  enums, so a heterogeneous static registry remains typed and needs no trait
  object, `Any`, allocator-backed erasure, atomics, executor, or unsafe
  binding-authored cast; and
- `std` hosts use `HostBindingCompilerRegistration`, whose Core-provided
  adapter safely erases the compiler cursor and artifact payload. Borrowed
  payload access checks both compatibility and concrete type. Consuming access
  returns the complete erased value unchanged on mismatch.

WP-200 is the sole implementation owner of the Core compiler/artifact SPI and
the Planning coordination surface. It introduces constructible compiler
component registrations so a third-party binding can author and test its
compiler before the rest of the execution SPI exists.

WP-300 consumes exactly one host or static compiler component when it later
constructs `HostBindingRegistration` or `StaticBindingRegistration<B>`.
Neither compiler component is independently installable in a
`ServientBuilder`; only the complete bundle remains an atomic installation
unit. This separation removes the duplicated implementation claim without
changing the package DAG or weakening `BIND-REG-001`.

Artifact admission uses an exact identity containing the plan-set generation,
plan id, binding id and generation, configuration digest, compatibility id,
and artifact role. Planning compares measured item/byte footprint with the
admitted bound before constructing an envelope. Static envelopes retain a
typed payload. Host envelopes use only Core-owned safe erasure. A
generation/configuration/compatibility/type mismatch is rejected before
execution.

`PlanBuildInput<'a, R>` borrows the validated TD and an immutable compiler
registration snapshot. `PlanBuildOutput<A>` owns all logical-plan and artifact
material and has no source lifetime. The Property Read slice must prove that
the TD can be dropped after build and that the completed artifact remains
sufficient for execution preparation.

## Rationale

An associated-type portable trait makes ownership and bounded progress
compile-visible in every feature cell. A closed static enum is the smallest
constructible way for an application to store several third-party compiler
implementations without host erasure. Core-owned host erasure keeps unsafe
downcasts and lossy mismatch behavior out of binding crates.

Separating a constructible compiler component from an installable complete
registration resolves the apparent WP-200/WP-300 cycle. Atomic installation
governs what enters a Servient, not whether an individual public component can
be constructed and tested.

## Rejected alternatives

- A host-only `dyn BindingCompilerExtension` contract is rejected because it
  cannot compile or store application-static associated state.
- Binding-authored `Any`, raw-pointer, integer-slot, or unsafe downcast
  conventions are rejected because they cannot guarantee mismatch ownership
  or cross-crate type safety.
- One heap-erased representation in every feature cell is rejected because it
  would make allocation and host runtime assumptions part of the portable SPI.
- A distinct public compiler trait for host and static cells is rejected
  because it would create two semantic contracts and allow their outcomes to
  diverge.
- Moving the compiler/artifact SPI back to WP-300 or adding a reverse
  Planning-to-Binding dependency is rejected because the existing WP-200
  evidence and package order already assign planning coordination to WP-200.
- Making compiler components independently installable is rejected because it
  would permit identity, capability, compiler, and execution halves to drift.

## Migration

The exact Rust contract, ownership split, Property Read tranche candidate,
paired authoring fixtures, audit, and executable entry/completion checks are
migrated together into:

- `docs/spec/planning.md`;
- `docs/spec/binding-spi.md`;
- `docs/architecture/30-compiled-plan-lifecycle.md`;
- `docs/architecture/40-protocol-binding-spi-and-deployment.md`;
- `docs/api-ownership.csv`;
- `docs/work-packages/WP-200-planning.md`;
- `docs/work-packages/WP-300-bindings.md`;
- `docs/work-packages/property-read-architecture-gate.toml`; and
- the registered WP-200 audit and contract-check artifacts.

The candidate is non-implementation work. Product source remains blocked until
the exact commit receives independent review and a separate pre-source
admission checkpoint.

## Former required closure

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

That closure is now represented by the decision/admission packet. The paired
WP-200 compile-contract root is permitted as review input, but neither planned
cross-package architecture fixture root nor Planning/Core/TD implementation
source is admitted before independent review.
