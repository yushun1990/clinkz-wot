---
name: clinkz-gate-review
description: Independently review a ClinkZ-WoT implementation candidate, architecture gate, milestone closure, or consequential PR against repository authority, the exact diff, and executable evidence. Use from a fresh review context; do not inherit the implementer's conclusion or modify the candidate while judging it.
---

# ClinkZ Independent Gate Review

Use this workflow for an independent acceptance decision or consequential review.

The review session is evidence-gathering and judgment only. Repository authority,
not the implementer's narrative and not this skill, defines what the candidate
must satisfy.

## Reconstruct independently

Start from a fresh context when independence matters.

1. Fetch/reconcile the default branch and the exact candidate branch, commit, or
   pull request under review.
2. Read `PLAN.md` only for durable milestone context.
3. Read the smallest relevant accepted architecture/specification/ADR/work
   package/gate material needed to know the intended contract.
4. Inspect the exact diff and the affected implementation directly.
5. Inspect registered tests, focused validation evidence, and relevant CI.
6. Treat PR descriptions, implementation summaries, prior model conclusions,
   and handoff prose as leads only; verify material claims from authoritative
   sources.

## Review the candidate

Check whether the candidate actually satisfies the accepted boundary, including
where applicable:

- public/API and protocol-facing contracts;
- ownership, lifecycle, cancellation, concurrency, and resource invariants;
- semantic/runtime authority and absence of unintended second authorities;
- persistence and reconstruction truth;
- protocol-neutral versus binding-specific responsibility;
- compatibility behavior and hidden fallback paths;
- error semantics and failure containment;
- work-package or gate closure conditions;
- test/evidence coverage for falsifiable invariants;
- unrelated scope expansion or speculative machinery.

Search specifically for implementation conveniences that weaken or duplicate the
accepted authority even when tests are green.

Distinguish defects in the implementation from evidence that the accepted design
itself is wrong. The latter is a reopen condition, not a request to patch around
the design during review.

## Preserve review independence

- Do not modify implementation while performing the acceptance judgment.
- Do not continue the implementer's session when a fresh independent review is
  required.
- Do not accept a claim merely because the implementation model made it or CI is
  green.
- Do not use subagents or parallel reviewers unless the Owner explicitly asks
  for them. Multiple model opinions do not vote architecture into correctness.
- If additional evidence is required, state the minimal evidence needed and keep
  the candidate unaccepted until it exists.

## Gate decision

Classify findings by consequence, not style:

- **blocker**: acceptance claim is false or materially unproven;
- **non-blocking**: improvement that does not falsify the current acceptance
  boundary;
- **reopen**: implementation evidence falsifies or materially undermines an
  accepted architecture/design assumption.

For an existing registered gate, preserve the status authority defined by
`PROJECT_GOVERNANCE.md`. A successful technical review can justify a separate
status-only transition, but the review must not mix implementation changes into
that transition.

## Final output

Report concisely:

1. exact candidate reviewed;
2. authority and executable evidence inspected;
3. blocker findings first, with precise file/evidence references;
4. non-blocking findings separately;
5. explicit conclusion: accept, reject/repair, or reopen architecture;
6. if accepted, the exact scope of the acceptance claim and any separate gate
   status action still required.
