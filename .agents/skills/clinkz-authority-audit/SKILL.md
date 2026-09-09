---
name: clinkz-authority-audit
description: Audit and converge an architecture, ownership, lifecycle, protocol, runtime-authority, or semantic-authority question in ClinkZ-WoT before implementation. Use when a decision boundary must be reconstructed and challenged. Do not use for routine implementation or final acceptance review.
---

# ClinkZ Authority Audit

Use this workflow to investigate a consequential technical decision before
implementation is admitted.

The repository governance is authoritative. This skill is only a procedure for
one investigation session.

## Reconstruct the decision boundary

1. Fetch/reconcile the default branch and the relevant task branch or pull
   request when one exists.
2. Read `PLAN.md` only for durable roadmap and milestone context.
3. Read the smallest relevant set of `ARCHITECTURE_GOVERNANCE.md`, accepted
   specifications/ADRs/work packages, workspace topics, audits, source, and
   tests needed to reconstruct the question.
4. Inspect current implementation before making claims about current behavior
   or ownership.
5. State the exact decision boundary in technical terms. Separate settled
   authority from assumptions, proposals, compatibility surfaces, and open
   evidence.

## Audit the authority

Trace the relevant data, ownership, lifecycle, resource, protocol, and runtime
paths end to end. Identify:

- the current authoritative representation or state owner;
- admission/validation boundaries and irreversible transitions;
- public/API or protocol-neutral contracts that constrain the choice;
- execution/runtime owners versus derived views or compatibility surfaces;
- duplicated semantic truth, shadow state, caches, or representations that
  could become a second authority;
- lifecycle, cancellation, resource-accounting, concurrency, or persistence
  consequences;
- evidence gaps where tests or code do not support the current claim.

Do not merely prove the current proposal internally consistent. Challenge the
existing inclination and compare credible simpler alternatives against project
goals and current implementation evidence.

## Converge or stop

If the evidence supports a stable conclusion:

- state the selected technical direction;
- state which artifact should own the durable decision;
- state the invariants and falsifiable completion boundary needed by an
  implementation session;
- update only the authoritative design/work-package documentation necessary to
  record the accepted conclusion when the task includes repository changes.

If the decision remains materially uncertain, keep it open in the appropriate
investigation space and stop before implementation. Do not hide uncertainty in
code, helpers, compatibility layers, or provisional runtime state.

## Session boundaries

- Do not implement the design during the audit unless the Owner explicitly
  changes the task boundary after the decision is settled.
- Do not create a new project-state or continuation document for the session.
- Do not use subagents or parallel workers unless the Owner explicitly requests
  them or the task clearly contains bounded, independent, read-only evidence
  gathering that cannot affect the architecture sequence.
- If delegation is used, the primary session remains responsible for the final
  integrated judgment; delegated results are evidence, not authority.

## Final output

Report concisely:

1. decision boundary;
2. authoritative evidence inspected;
3. material findings and rejected alternatives;
4. selected conclusion or unresolved blocker;
5. durable artifact changed or to be changed;
6. exact next admissible engineering action.
