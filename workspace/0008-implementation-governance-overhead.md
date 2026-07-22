# 0008 Implementation Governance Overhead

Status: OPEN
Kind: development-process improvement
Target revision: v4.9 implementation convergence

## Scope and authority

This topic records a risk in the current implementation-admission process.

It does not change architecture, admit implementation work, or override
`PLAN.md`, the work-package DAG, ADRs, reviews, audits, or authoritative
specifications. Its purpose is to determine whether the current governance
mechanism is becoming more expensive than the implementation risk it is meant
to control.

The issue is not that the repository has too much documentation in general.
The issue is that small, local, additive implementation changes may require
coordinated edits across too many governance artifacts before useful runtime
evidence can be produced.

## Problem

The v4.9 closure work correctly introduced strong controls around:

- architecture authority;
- work-package admission;
- dependency ordering;
- evidence invalidation;
- ownership boundaries;
- review disposition;
- bounded-resource contracts; and
- migration from legacy implementation.

Those controls are appropriate for cross-cutting or invariant-sensitive work.

The risk is that the same level of ceremony may be applied to every small
implementation tranche, including passive values or mechanical migrations that:

- do not alter ownership;
- do not add runtime progress;
- do not change resource semantics;
- do not introduce protocol behavior;
- do not affect lifecycle transitions; and
- can be verified locally by compilation, tests, and narrow evidence.

When this happens, the project can spend more effort proving that work may
start than implementing and validating the work itself.

## Observed risk

The current process can encourage repeated subdivision of already narrow work
because every discovered issue is treated as a reason to reopen admission
rather than first determining whether the issue intersects the tranche.

This creates several failure modes.

### 1. Governance dominates implementation

A small implementation change may require synchronized changes to:

- work-package prose;
- machine-readable package indexes;
- review records;
- audit records;
- evidence manifests;
- dependency projections;
- workspace discussions; and
- `PLAN.md`.

The aggregate coordination cost can exceed the technical cost and risk of the
change.

### 2. Progress becomes document-shaped

Agents may optimize for producing complete governance projections rather than
producing executable evidence.

A tranche can appear highly controlled while no integrated runtime path has
been exercised.

### 3. Unrelated blockers propagate too far

A cross-cutting defect discovered near a tranche may block the entire tranche
even when part of the tranche is demonstrably disjoint.

The time-domain defect discovered around `Deadline` is an example of the
correct containment response: isolate the intersecting value and allow
clock-independent passive values to be reviewed separately. The lesson should
not become "split every value into its own package"; it should become "block
only the smallest scope actually intersected by the defect."

### 4. Recovery becomes harder

Many tiny packages increase:

- dependency edges;
- admission records;
- evidence relationships;
- invalidation paths;
- review surface; and
- the chance of projections drifting out of sync.

This can make the process less recoverable rather than more recoverable.

## Proposal

Adopt a risk-proportional implementation-admission policy.

The required governance depth should be determined by the semantic risk of a
change, not by a uniform package template.

### Category A — local additive implementation

Examples:

- passive value types;
- constructors and accessors;
- error-free conversions;
- local trait implementations;
- mechanical module moves;
- compile-time registration values with no lifecycle behavior.

Minimum required controls:

- authoritative contract already exists;
- exact scope is named;
- dependencies are satisfied;
- no unresolved finding intersects the scope;
- local tests and compilation evidence are defined;
- one recoverable Git checkpoint is created.

These changes should not require a new ADR, architecture review, or broad
evidence rewrite unless they reveal a real semantic conflict.

### Category B — cross-module contract implementation

Examples:

- handler entry;
- binding artifact boundaries;
- planner-to-binding compilation;
- Servient orchestration interfaces;
- cleanup ownership transfer;
- resource reservation APIs.

Required controls:

- explicit work package;
- exact dependency and ownership review;
- conformance fixtures;
- relevant review and audit projection;
- impact analysis against existing evidence.

### Category C — architecture or invariant change

Examples:

- changing ownership;
- changing lifecycle phases;
- changing time semantics;
- changing resource accounting;
- introducing a new execution path;
- changing protocol-neutral boundaries.

Required controls:

- workspace discussion;
- authoritative design or ADR migration;
- work-package revision;
- evidence invalidation and replacement;
- architecture review where required.

## Tranche sizing rule

A tranche should be split only when at least one of the following is true:

1. part of the tranche has a distinct unresolved semantic blocker;
2. parts have different ownership or lifecycle effects;
3. parts require different authoritative contracts;
4. one part can be completed and validated independently while another cannot;
5. failure or rollback boundaries materially differ; or
6. evidence for one part would otherwise falsely claim coverage of another.

A tranche should not be split merely because each type or trait can be named
individually.

## Admission question

Before creating another work package or workspace topic, ask:

> Does this issue change the semantics, ownership, lifecycle, resource model,
> or evidence truth of the proposed tranche?

If the answer is no, handle it inside the existing tranche.

If the answer is yes, isolate only the intersecting scope.

## Expected result

The project retains strong architecture governance while reducing process
amplification for low-risk implementation.

The intended hierarchy becomes:

```text
authoritative architecture
        ↓
risk-scoped implementation tranche
        ↓
implementation and executable evidence
        ↓
feedback
        ↓
authoritative correction when required
```

The process should make architecture violations difficult without making
ordinary implementation artificially expensive.

## Alternatives considered

### A. Keep one uniform admission process

This maximizes procedural consistency but does not distinguish a passive value
from a lifecycle or ownership change. It risks making governance cost
independent of technical risk.

Not recommended.

### B. Remove work-package governance

This would reduce overhead but would also remove the controls needed for the
v4.9 migration, especially around ownership, cleanup, bounded resources, and
evidence invalidation.

Not recommended.

### C. Use risk-proportional governance

This preserves strict controls for architecture-sensitive work while allowing
small additive implementation to proceed with narrow evidence.

Recommended.

## Open decisions

1. Which existing work-package fields are mandatory for Category A work?
2. Can Category A work be recorded as a tranche inside a parent package rather
   than as a new top-level package?
3. Which artifact owns the risk-category classification?
4. What evidence is sufficient to prove that a discovered issue is disjoint
   from a tranche?
5. Which existing closure checks must be changed to support the policy without
   weakening architecture enforcement?

## Migration

If accepted, migrate the stable policy into:

- `AGENTS.md` implementation workflow;
- the work-package authoring guidance;
- relevant machine-readable governance rules;
- `PLAN.md` only where current tranche boundaries change.

This topic should then be marked MIGRATED.
