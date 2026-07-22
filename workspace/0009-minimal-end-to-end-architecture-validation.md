# 0009 Minimal End-to-End Architecture Validation

Status: OPEN
Kind: implementation-readiness proposal
Target revision: v4.9 implementation convergence

## Scope and authority

This topic proposes a minimal executable vertical slice for validating the
current architecture.

It does not replace the work-package DAG and does not authorize broad runtime
migration. It asks whether the project should require one deliberately narrow
end-to-end path before continuing to expand isolated foundations and SPI
surfaces.

The target is not a demonstration application. The target is architecture
evidence.

## Problem

The v4.9 architecture is increasingly well specified in documents, ADRs,
reviews, work packages, ownership projections, and machine-readable governance
artifacts.

However, the most important architectural claims remain primarily validated
through document consistency rather than through an executable path crossing
the actual module boundaries.

Those claims include:

- planning produces immutable protocol-neutral logical plans;
- binding compilers produce binding-owned artifacts without rescanning TD;
- Servient owns activation and runtime orchestration;
- protocol bindings do not dispatch application handlers directly;
- inbound protocol data becomes a protocol-neutral request;
- handler execution is selected through admitted routing state;
- response data returns through a protocol-neutral boundary;
- cleanup and generation ownership are explicit; and
- no hidden direct execution path bypasses Servient.

Each individual contract can look correct while their composition is wrong.

## Risk

### 1. Integration defects are discovered too late

Planning, binding SPI, handler, Servient, and cleanup work can each advance
independently. A mismatch may not appear until a real binding migration begins,
at which point many supposedly completed contracts may need revision.

### 2. Architecture can converge only on paper

Documents may agree while implementations still require:

- TD rescanning inside a binding;
- PB-owned handler lookup;
- hidden callback dispatch;
- mutable runtime plan state;
- lifecycle shortcuts;
- unowned cleanup;
- protocol-specific leakage into Core.

Without a full path, these pressures remain hypothetical and cannot be judged
against real APIs.

### 3. The first real binding becomes the architecture test

If Zenoh migration is the first end-to-end validation, protocol complexity and
architecture complexity arrive together. Failures become difficult to
attribute.

### 4. Work-package completion can overstate readiness

A set of package-local tests may prove each package in isolation but not prove
that the architecture is implementable as a coherent runtime.

## Proposal

Require one minimal property-read vertical slice before broad binding or
handler expansion.

The slice should exercise exactly this path:

```text
Thing Description fixture
        ↓
shared planner
        ↓
immutable logical property-read plan
        ↓
mock binding compiler
        ↓
binding-owned artifact
        ↓
prepared producer route
        ↓
Servient-owned activation
        ↓
mock binding accepts one inbound request
        ↓
protocol-neutral accepted request
        ↓
Servient route and handler selection
        ↓
one static property-read handler
        ↓
protocol-neutral response
        ↓
mock binding response delivery
        ↓
route/request cleanup
```

## Why property read

Property read is the smallest useful interaction that still crosses the major
boundaries:

- TD parsing and planning;
- form/candidate ownership;
- binding compilation;
- server route preparation;
- atomic activation;
- inbound acceptance;
- handler dispatch;
- response delivery;
- generation and cleanup.

It avoids the additional complexity of:

- subscription lifetime;
- event fan-out;
- observable-property streams;
- write payload validation;
- action progress;
- cancellation races;
- multi-response interactions.

## Required constraints

### Use a mock Protocol Binding

The slice should use a deterministic in-process binding fixture.

The mock binding should implement only the minimum required producer-side
contract and should not contain Zenoh, MQTT, HTTP, sockets, broker sessions, or
async runtime integration.

This isolates architecture correctness from protocol behavior.

### One interaction only

The slice should support:

- one Thing;
- one readable property;
- one compiled route;
- one handler;
- one request;
- one response.

No generic registry expansion is required beyond what the architecture itself
requires.

### No hidden dispatch

The mock binding may accept and emit protocol-shaped fixture data, but it must
not:

- select a handler;
- hold application handler references;
- call a dispatch callback;
- rescan TD;
- construct a new logical plan;
- mutate the admitted plan set.

It must hand a protocol-neutral accepted item to Servient-controlled progress.

### Use real boundary types

The slice should use the actual planned Core and SPI boundary types wherever
they exist.

Temporary fixture-only adapters are acceptable only when:

- the missing production type belongs to a later admitted package;
- the adapter is explicitly named as temporary;
- the adapter does not conceal an ownership decision; and
- replacement is recorded as a dependency.

### Prove cleanup

The slice is incomplete unless it demonstrates:

- request/accept permit release;
- route generation ownership;
- response completion;
- shutdown or deactivation cleanup; and
- absence of retained hidden handler or request references.

## Non-goals

The slice must not expand into:

- full handler trait coverage;
- a generic server framework;
- Zenoh binding migration;
- dynamic binding loading;
- runtime binding registration;
- multi-form fallback;
- retry policy;
- subscriptions;
- events;
- observable properties;
- actions;
- collection interactions;
- security-scheme execution;
- production networking;
- performance benchmarking.

These belong to later work packages.

## Completion evidence

The vertical slice should produce executable evidence for the following
statements.

### Planning

- the TD fixture is read only during planning;
- the logical plan is immutable after admission;
- the binding compiler receives bounded protocol-neutral planning input;
- the binding artifact contains everything needed by the mock binding at
  runtime.

### Binding boundary

- the binding owns protocol adaptation and transport-facing state only;
- the binding cannot access application handlers;
- the binding does not select or dispatch handlers;
- the binding returns a protocol-neutral accepted request;
- the binding receives a protocol-neutral response for emission.

### Servient orchestration

- Servient owns route activation;
- no request is accepted before activation;
- Servient selects the admitted route and handler;
- progress occurs only through the declared orchestration path;
- deactivation prevents new acceptance.

### Handler

- the handler is statically registered;
- the handler sees only the intended protocol-neutral interaction input;
- handler completion produces the expected response value;
- no binding-specific value leaks into the handler API.

### Cleanup

- generation-bearing values remain associated with the correct plan/route
  generation;
- request-scoped ownership is released exactly once;
- shutdown drains or rejects remaining progress according to the frozen
  lifecycle contract;
- the test leaves no active route or request resource.

## Placement in the execution plan

The vertical slice should be represented as an integration/conformance gate,
not as a replacement for foundation, planner, binding, handler, or Servient
packages.

Its dependencies should include only the minimum narrow tranches required to
construct the path.

Packages that expand the relevant surfaces should not claim architecture-level
integration readiness until this gate passes.

A suitable position is:

```text
minimum foundation values
        +
minimum planner property-read path
        +
minimum binding producer SPI
        +
minimum Servient activation/orchestration
        +
minimum static handler entry
        ↓
property-read vertical-slice gate
        ↓
broader handler and binding expansion
```

## Expected result

The project gains an executable architecture proof before investing in broad
surface completion.

The slice should expose whether the current design is:

- implementable without ownership shortcuts;
- ergonomic enough for a trivial binding;
- capable of preserving immutable planning boundaries;
- able to hide internal lifecycle machinery from ordinary PB code;
- compatible with explicit cleanup and bounded resources.

Any difficulty discovered by the slice becomes concrete design feedback rather
than speculation.

## Alternatives considered

### A. Continue package-local implementation only

This preserves the current DAG but delays cross-boundary feedback.

Not recommended as the sole validation method.

### B. Use Zenoh as the first vertical slice

This provides realistic protocol evidence but mixes protocol-specific session,
routing, payload, and async concerns with architecture validation.

Not recommended for the first slice.

### C. Build a mock-binding property-read slice

This provides the smallest executable proof of the primary producer flow while
keeping failures attributable to architecture boundaries.

Recommended.

## Open decisions

1. Which exact existing or planned work package owns the integration gate?
2. Which temporary adapters are permitted before their production types land?
3. Must the slice run under both host/std and constrained/manual-poll profiles?
4. Which generation and cleanup assertions are mandatory in the first version?
5. Does passing the slice become a precondition for WP-300 binding expansion,
   broad handler entry, or both?
6. Which architecture claims should be encoded as compile-fail tests versus
   runtime tests?

## Migration

If accepted:

- add the integration gate to `PLAN.md` and the work-package DAG;
- define its exact dependency set and completion evidence;
- link it from the relevant planner, binding, Servient, and handler packages;
- record failures as scoped design feedback rather than silently adding
  shortcuts.

This topic should be marked MIGRATED after the gate is represented in the
authoritative execution artifacts.
