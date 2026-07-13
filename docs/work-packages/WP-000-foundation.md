# WP-000 Foundation Primitives

Status: Complete

Design revision: v4.6

Depends on: None

Required gates: GATE-1, GATE-2, GATE-3, GATE-4, GATE-5, GATE-6

Owner packages: clinkz-wot-foundation, workspace

## Scope

Create the `clinkz-wot-foundation` Cargo package and make it the only owner of
protocol-neutral resource, work-budget, monotonic-time, source-time, and generation
primitives. Establish the dependency edge below `clinkz-wot-td` and `clinkz-wot-core`
without moving TD vocabulary, interaction semantics, binding plans, discovery contracts, or
host runtime behavior into the package.

This package establishes the types and invariants consumed by every later work package. It
does not migrate TD ingestion, core dispatch, binding execution, Directory clients, or
Servient lifecycle orchestration.

## Requirements

- `PROFILE-AXIS-001`, `FEATURE-MATRIX-001`, and `CRATE-DEPS-001` define the portable build
  surface and downward-only dependency direction.
- `API-SURFACE-001`, `API-TYPES-001`, `API-SOURCE-TIME-001`, and `TIME-001` freeze work,
  generation, clock, instant, and source timestamp semantics.
- `CONSTRAINED-STORAGE-001`, `CONSTRAINED-STORAGE-002`, `CONSTRAINED-WORK-001`, and
  `CONSTRAINED-OWN-001` define static profiles, generation-safe slots, and typed budgets.
- `RES-LIMIT-001`, `RES-LIMIT-002`, `RES-LIMIT-003`, `RES-LIMIT-004`, `RES-PROFILE-001`, and
  `API-RESOURCE-001` define the exhaustive limit schema, named profiles, accounts,
  reservations, and ledgers.
- `ADMIT-TXN-001` and `ADMIT-MEM-001` define reservation rollback, phase ownership, and peak
  memory accounting.
- `PERF-ACCOUNT-001` requires bounded hierarchical accounting without a global interaction
  hot-path lock.
- `HOST-DEFAULT-001` and `HOST-DEFAULT-002` apply only to the named profile values owned here;
  host scheduling and runtime policy are implemented by later packages.

## Crates and Feature Cells

- Add package `clinkz-wot-foundation` at `foundation/` with a root that supports
  `--no-default-features` as `no_std + alloc`.
- The `no-default`, `async-no-std`, and `std` cells expose the same protocol-neutral value
  types. The `async` cell adds no executor and no alternate ownership model.
- The `std` cell may add conversions to host duration or clock values, but the stored public
  representation remains independent of `std::time::Instant` and wall-clock time.
- Update `clinkz-wot-td` and `clinkz-wot-core` dependency declarations only far enough to
  prove `foundation <- td` and `foundation + td <- core`. No reverse dependency is allowed.
- Add compile fixtures for the three cells and a dependency inspection that rejects TD,
  runtime, protocol, socket, filesystem, thread, or executor dependencies in foundation.

## Public API and Data Migration

Implement these frozen public paths exactly as owned by `docs/api-ownership.csv`:

- `clinkz_wot_foundation::WorkClass`, `WorkBudget`, and `BudgetExceeded` in `budget`;
- `ResourceProfileId`, `ResourceLimits`, `StaticResourceProfile`, `ResourceKind`,
  `ResourceAccount`, `ResourceReservation`, and `AdmissionLedger` in `resource`;
- `GatewayDefaultV1`, `DirectoryClientDefaultV1`, and `BenchmarkStaticReferenceV1` in
  `resource`;
- `ClockId`, `MonotonicInstant`, `RuntimeClock`, and `SourceTimestamp` in `time`;
- `Generation` and `SlotIndex` in `generation`.

Generate or validate every `ResourceLimits` field and named-profile value against
`docs/resource-limits.csv`; a Rust-side field list or default table must not become a second
schema. Represent `NA` as a typed non-applicable value and reject omission, inheritance
markers, and implicit infinity. Keep counters, ids, and units as newtypes where mixing them
could bypass admission or work accounting.

`WorkBudget` contains the frozen typed counters, decrements before work starts, and returns a
structured exhaustion result without wrapping. `MonotonicInstant` retains `ClockId`; checked
ordering and subtraction reject different clocks. Profile snapshot constructors return the
complete versioned limit value and do not silently select a constrained default.

## State and Ownership Migration

- Make `ResourceReservation` move-only. It owns an uncommitted charge, releases that charge
  idempotently, and can be committed only into a published owner or another explicit account.
- Make `AdmissionLedger` own the source, temporary, persistent-document,
  persistent-runtime, diagnostic, and cleanup accounts. Preserve peak-live and largest-
  contiguous-allocation observations across phase releases.
- Represent parent and global ceilings without a process-wide hot-path lock. A child account
  may batch against a reserved parent allowance only within the limits frozen in the active
  profile.
- Make `Generation` changes explicit at removal/reuse boundaries. `SlotIndex` alone never
  identifies a reusable object.
- Keep clock identity and wrap policy with the clock source; do not infer comparable time from
  a raw tick count.

## Old API Removal

There is no supported legacy foundation public surface. Do not add compatibility aliases in
`clinkz-wot-core`, `clinkz-wot-td`, or the umbrella crate for duplicate budget, limit, clock,
or generation definitions.

As downstream entry points migrate, remove raw `usize` capacity bundles, raw integer
generation/slot pairs, and `std::time::Instant` values that cross protocol-neutral package
boundaries. Remove any temporary duplicate profile snapshot immediately after the final
consumer adopts `clinkz_wot_foundation`; a duplicate schema or default table is a completion
blocker, not a compatibility mode.

## Evidence

Produce these package evidence keys exactly as indexed by the work-package DAG:

- `foundation-feature-matrix` for the three compilation cells and forbidden dependencies;
- `resource-limit-boundaries` for every field and exact/one-over boundary;
- `resource-profile-snapshots` for exhaustive versioned named profiles;
- `resource-accounting-rollback` for reservation, commit, rollback, and phase accounting;
- `time-and-generation-api` for clock identity, wrap, and slot reuse semantics.

These records satisfy the corresponding requirement-index evidence families:

- `profile-axis-matrix` and `feature-public-surface` for all required feature cells;
- `cargo-dependency-direction` for the package graph and forbidden-dependency inspection;
- `frozen-cross-crate-surface` and `common-public-types` for public path and trait checks;
- `resource-boundaries` for every field, exact-limit, one-over-limit, and named snapshot case;
- `manual-runtime` for typed budget exhaustion and generation-safe constrained values;
- `admission-ledger` for reserve, commit, rollback, phase release, peak, and contiguous bytes;
- `defaults-and-timeout-races` for profile snapshots and clock comparison/wrap behavior.

Each evidence record names WP-000, its requirement ids, the compilation cell, and the selected
resource profile. Evidence from a later integration package may supplement but not replace the
foundation unit and compile evidence.

## Performance Workloads

- `PERF-GW-013`, `PERF-CS-011`, and `PERF-DIR-004` exercise hierarchical accounting without
  a global interaction-path lock.
WP-000 supplies counters, profiles, and result identity plumbing. Runtime adapters for these
workloads may land later, but no later package may replace the accounting or profile semantics
to satisfy a budget.

## Completion Conditions

- `clinkz-wot-foundation` builds and documents a useful public surface in all three required
  feature cells, including a no-std compile fixture.
- Every ownership-matrix item assigned to foundation exists at its frozen public path and no
  duplicate public definition exists elsewhere.
- Every resource field and named profile has an exact snapshot test tied to
  `docs/resource-limits.csv`.
- Reservation rollback, peak accounting, work exhaustion, clock mismatch, and generation
  reuse tests pass without panic or counter wrap.
- Cargo dependency inspection proves the required downward graph and absence of host/runtime
  dependencies.
- All listed stable evidence keys have current WP-000 records; no temporary compatibility
  type or duplicate profile schema remains.
