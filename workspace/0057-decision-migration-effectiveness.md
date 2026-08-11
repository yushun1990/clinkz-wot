# 0057 Decision Migration Effectiveness

Status: OPEN

Kind: owner-raised governance-effectiveness investigation

Priority: HIGH

Target: whether migrated decisions actually change later execution behavior

## Scope and authority

This topic records a Project Owner concern that some workspace decisions may be correctly investigated, decided, and migrated into authoritative repository artifacts without reliably changing the default behavior of later engineering work.

The concern is not that every decision must immediately modify product source, nor that a migrated decision is invalid unless its full downstream implementation lands in the same review cycle. Codex owns the technical and governance judgment from repository evidence.

The narrower question is how the repository distinguishes **decision durability** (the conclusion is recorded in the proper authority) from **decision effectiveness** (later work actually follows the changed rule or mechanism).

## Owner observation and repository counterexamples

The repository contains evidence that textual migration alone may be insufficient:

- workspace/0020 was historically decided and migrated with rules that support artifacts must not define technical truth in reverse or create independent refinement cycles, yet later route-reservation and WP-400 work still accumulated substantial tranche-specific checker machinery; 0020 had to be reopened to address the implementation-shape problem that the first migration had not bounded;
- `PROJECT_STATE.md` has long been governed as curated continuation memory with stale information replaced rather than accumulated, yet it repeatedly grew historical candidate/review/merge/workflow chains, which is now separately investigated by workspace/0055;
- governance states that one decision/conversion packet should not be serialized into separate cycles merely because workspace, specification, audit, registry, and work-package artifacts are different files, while historical execution has still shown repeated pressure toward mechanically separated governance/checkpoint work.

These are counterexamples for investigation, not proof that any specific rule was ignored intentionally. In some cases the original decision may have been underspecified, may have required a later distinct implementation claim, or may have conflicted with other local incentives.

## Questions for investigation

1. What does `MIGRATED` currently prove: authoritative textual projection only, or also operational adoption?
2. How should the project determine that a migrated decision has actually changed the default path followed by later Codex sessions?
3. Which past migrated workspace topics changed authoritative text but did not sufficiently change subsequent behavior, and why?
4. How should the repository distinguish a legitimate deferred implementation consequence from a migration that appears complete but leaves the old mechanism as the practical default?
5. When a decision changes execution rules rather than product behavior, what concrete repository evidence demonstrates effectiveness without adding another ceremonial governance layer?
6. Should a decision migration identify the old default, the intended new default, and the first later claim expected to exercise the new path?
7. Should later evidence that the old behavior continues automatically reopen the original topic, open a linked effectiveness topic, or be handled through another existing mechanism?
8. Are current `DECIDED -> MIGRATED` semantics sufficient if interpreted differently, or is a small governance clarification required?
9. How can this be solved without adding routine effectiveness reviews, new lifecycle states, mandatory Owner approvals, or another checker that merely validates governance prose?
10. What measurable evidence would show that decision effectiveness improves while governance overhead stays bounded?

## Constraints

- Preserve the existing AI-led technical decision model.
- Do not assume that `MIGRATED` needs a new lifecycle state such as EFFECTIVE or VERIFIED.
- Do not require every architecture/governance decision and its downstream implementation to occur in the same review cycle when they are genuinely distinct engineering claims.
- Do not equate lack of immediate code changes with failure to execute a decision.
- Do not add a generic compliance checker whose only purpose is to prove that prose references other prose.
- Prefer evidence from later real work over ceremonial reaffirmation.
- Preserve legitimate deferred implementation when the decision explicitly establishes a later next safe action.

## Expected decision output

Codex should determine:

1. whether the repository currently conflates durable migration with operational effectiveness;
2. which concrete historical cases demonstrate the problem and which are valid deferred implementations instead;
3. whether `MIGRATED` semantics, migration guidance, continuation projection, or another existing governance owner should change;
4. the minimum evidence needed to show that a decision changed later default behavior;
5. how ineffective migrations are detected and corrected without Owner micromanagement or recurring governance ceremonies; and
6. which authoritative owner should carry any stable correction.
