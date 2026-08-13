# Current Execution Contract

Status: IDLE

Contract revision: 0

Planning base: none

Task branch: none

Pull request: none

## Purpose

This is the repository's single replace-in-place carrier for the current
engineering claim. It gives the High Executor one frozen plan and gives a fresh
Max Acceptance Reviewer the same success boundary. It is not a roadmap,
session log, design authority, work-package manifest, or history archive.

`IDLE` means no implementation work is authorized. The next Max planning
context replaces the placeholders below; Git history retains the prior
contract. Keep this file at or below 200 lines.

## Roles for This Contract

- Technical Lead: Max
- Executor: High
- Acceptance Reviewer: fresh Max context, separate from implementation
- Plan Challenger: ChatGPT when the cycle meets the importance triggers in
  `PROJECT_GOVERNANCE.md`
- Periodic Auditor: Ultra at the repository-level checkpoints defined there
- Project Owner: goals, constraints, counterexamples, and external commitments

These are the current model mappings for stable capability roles. The roles,
not the product labels, own repository authority.

## Engineering Claim

No active claim.

## Authoritative Inputs

None selected. A planned contract names the smallest relevant specification,
work-package, source, test, audit, and workspace references; it does not copy
their contents.

## Scope

No implementation scope is authorized.

### In scope

- None.

### Out of scope

- All product, public API, runtime, fixture, and evidence implementation.

## Engineering Plan

No plan is active. Max must define ordered, outcome-oriented steps at a
specific fetched default-branch revision. High may choose local implementation
mechanics inside those constraints but may not change the engineering claim,
architecture authority, scope, non-goals, or acceptance criteria.

## Plan Challenge

Disposition: NOT REQUESTED

For an important cycle, this Lead-owned slot records the ChatGPT challenge
basis, concrete findings, and how the final plan incorporated or disposed of
them. It does not preserve a parallel proposed plan.

## Acceptance Criteria

No acceptance claim is active. A planned contract defines falsifiable outcomes,
required evidence, exact risk-appropriate checks, and any remote integration
predicate before High begins.

## Escalation and Stop Conditions

High stops and records evidence when implementation exposes an inconsistency,
unconstructible API, missing authority, scope expansion, invalid acceptance
criterion, or architecture defect. High must not work around it merely to make
the task pass. Max then revises the contract explicitly, increments its
revision, and returns it to `PLANNED`, or closes it as `BLOCKED`.

The cycle stops after accepted integration and reconciliation. It must not
silently begin the next roadmap item.

## Executor Handoff

Not started. While executing, High owns only this section plus status changes
to `EXECUTING`, `REVIEW_READY`, or `BLOCKED`. The handoff records the exact
implementation head, intended diff, checks run, evidence produced, deviations,
and unresolved findings without restating authoritative contracts.

## Acceptance Review

Verdict: NOT REVIEWED

A fresh Max context reconstructs the claim from repository authority, this
contract, the exact reviewed implementation head and diff, and executable
evidence. It records findings and one verdict: `ACCEPTED`, `CHANGES REQUIRED`,
or `BLOCKED`. Author-prepared summaries are navigation, not acceptance
authority.

## Lifecycle

```text
IDLE -> PLANNED -> EXECUTING -> REVIEW_READY -> ACCEPTED
                       |              |
                       v              v
                    BLOCKED       EXECUTING
```

- Max owns the Lead-authored sections and freezes them at `PLANNED`.
- Optional ChatGPT challenge occurs before execution; Max incorporates accepted
  corrections into this file.
- High implements and performs basic validation on the same task branch and
  leaves a draft pull request at `REVIEW_READY`.
- Fresh Max acceptance either returns concrete findings to High or records
  `ACCEPTED` and completes eligible integration and reconciliation.
- A later claim replaces this file; it does not append another contract.
