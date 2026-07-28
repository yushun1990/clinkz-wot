# 0013 Candidate Fallback Policy

Status: MIGRATED

## Problem

The planning specification preserves deterministic candidate order and forbids
fallback after binding acceptance, but its policy owner, default, eligible
pre-side-effect failures, binding-input-rejection treatment, runtime-health
rule, and skipped-candidate diagnostics were not constructible. AR-004 and the
WP-200 work package therefore correctly blocked implementation admission.

## Evidence

- `docs/spec/planning.md` already limits one interaction to the admitted
  candidate vector and one shared provider-probe budget.
- `docs/spec/binding-spi.md` constructs `OutboundRequest` only after security
  commit and returns `BindingInputRejection<OutboundRequest>` only when the
  binding did not accept protocol work.
- Security availability probes are required to be side-effect free; security
  commit may consume or mutate provider state.
- Deterministic lazy compiler negatives are pure planning outcomes, while
  cancellation, deadlines, transient resource failures, and transport failures
  are explicitly non-cacheable or operational.
- V1 binding registration and plan generations are startup-only and immutable;
  no health-generation or health-driven invalidation contract exists.
- Candidate count and provider-probe limits already bound selection work.

## Alternatives

1. Fall back after any result that precedes a protocol side effect.
2. Allow mutable binding health to reorder otherwise immutable candidates.
3. Disable automatic fallback entirely.
4. Permit fallback only for exact planning-owned side-effect-free outcomes.

The first alternative is unsafe because security commit precedes binding input
acceptance. The second creates an unowned mutable planning generation. The third
contradicts the retained ordered candidate contract and its registered
evidence. The fourth preserves deterministic order and the execution
exactly-once boundary.

## Decision

Adopt alternative 4 through ADR-0017:

- `CandidateFallbackPolicy::{Disabled, PreExecution}` is captured in
  `PlanBuildInput`, with `PreExecution` as the Consumer default;
- strict per-call selection disables automatic fallback;
- only side-effect-free security inapplicability and an exact deterministic
  lazy-artifact negative may skip a candidate;
- binding input rejection, health, stale/draining generations, security commit,
  budgets, cancellation, deadlines, backpressure, transient failures, and all
  post-acceptance outcomes are terminal for the interaction;
- runtime health remains diagnostic-only in v1; and
- every eligible skip produces one fixed-width, secret-free diagnostic in a
  sequence pre-reserved to the admitted candidate count.

## Migration

The decision is projected into:

- `docs/ADRs/0017-pre-execution-candidate-fallback.org`;
- `docs/spec/planning.md`;
- `docs/architecture/10-primary-data-flows.md`;
- `docs/spec/binding-spi.md`;
- `docs/api-ownership.csv`;
- `docs/work-packages/WP-200-planning.md`;
- the architecture checker, plan, and continuation state; and
- the workspace and artifact indexes.

The decision closes the AR-004 design gap but grants no WP-200 source-edit
authority. The exact property-read plan slice still needs a non-implementation
candidate and independent ADR-0013 review.
