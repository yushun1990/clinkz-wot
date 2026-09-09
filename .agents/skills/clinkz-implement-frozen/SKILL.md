---
name: clinkz-implement-frozen
description: Implement a ClinkZ-WoT task whose architecture, ownership/lifecycle boundary, and completion conditions are already sufficiently settled. Use for coding, debugging, tests, and evidence collection. Stop and reopen instead of silently redesigning accepted authority.
---

# ClinkZ Frozen Implementation

Use this workflow when implementation is admitted inside an already accepted
technical boundary.

The repository governance and accepted technical artifacts remain authority.
This skill does not grant permission to redesign them for implementation
convenience.

## Reconstruct the implementation contract

1. Fetch/reconcile the default branch and relevant task branch or pull request.
2. Read `PLAN.md` for roadmap context, then read only the smallest relevant
   accepted specification/ADR/work package and implementation/test surface.
3. Inspect the current source before planning edits.
4. State the implementation objective, accepted invariants, and falsifiable
   completion boundary.
5. Distinguish local implementation freedom from decisions that would alter
   semantics, public API, ownership, lifecycle, protocol neutrality, resource
   authority, persistence authority, or acceptance truth.

## Implement within the accepted boundary

Own ordinary implementation mechanics without asking for a separate design
plan when the boundary is clear. This includes local decomposition, helper
shape, file placement, Rust ownership mechanics, debugging strategy, focused
tests, and evidence collection.

Prefer the smallest coherent change that satisfies the accepted contract.
Preserve unrelated work and avoid speculative abstractions.

Do not introduce:

- a second semantic/runtime authority;
- shadow project or runtime state used only to make the implementation easier;
- undocumented compatibility behavior that changes the accepted contract;
- duplicate ownership/lifecycle paths;
- checker or orchestration machinery whose only purpose is task ceremony.

## Reopen on decision evidence

Implementation difficulty is not automatically an architecture problem. Resolve
ordinary local mechanics inside the implementation session.

Stop and surface a decision boundary when evidence shows that satisfying the
accepted design requires changing semantics, public API, ownership, lifecycle,
resource/protocol boundaries, persistence truth, acceptance evidence, or another
accepted invariant.

When that happens:

1. preserve the smallest useful reproduction/evidence;
2. explain exactly which accepted assumption was falsified;
3. do not hide the contradiction behind a workaround;
4. request/recommend reopening the relevant design or work-package authority;
5. do not continue source changes that depend on an unaccepted replacement
   design.

## Validate proportionally

Run focused validation first:

1. affected compile/check/test targets;
2. relevant integration or invariant tests;
3. broader workspace validation only when the semantic risk or gate requires it.

Avoid repeatedly feeding large unrelated test output back into the session.
Capture the smallest evidence needed to diagnose failures and prove the final
claim.

Passing tests is executable evidence, not independent gate acceptance.
Do not self-authorize an existing gate status transition that project governance
reserves for independent acceptance.

## Session boundaries

- Keep implementation serial with respect to unresolved architecture decisions.
- Do not use subagents or parallel write workers unless the Owner explicitly
  requests them and the work is genuinely isolated from shared authority.
- Do not create a continuation-state document to preserve the session.
- Do not perform the independent acceptance review as a continuation of this
  implementation context.

## Final output

Report concisely:

1. accepted boundary implemented;
2. material code/test changes;
3. validation executed and results;
4. any remaining evidence gap or reopen condition;
5. branch/PR state when repository-changing work was requested;
6. exact candidate now ready for independent review, when applicable.
