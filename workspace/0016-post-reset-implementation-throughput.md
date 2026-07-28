# 0016 Post-Reset Implementation Throughput and Executable Progress

Status: OPEN

Kind: implementation-throughput and governance-risk proposal

Target: the transition from the v5.0 authority reset into the Property Read vertical slice

## Scope and authority

This topic asks whether the current project process has a bounded and credible path from the v5.0 authority reset to executable Planning, Binding, and Servient progress.

It does not propose a process change, select a governance model, weaken an existing review or evidence requirement, admit implementation, redefine the Property Read architecture gate, or predetermine the resolution of `workspace/0014-property-read-plan-artifact-boundary.md`.

The Project Owner is raising a concern for AI investigation. The AI must determine from repository evidence whether the concern is valid, what causes it if valid, and whether any authoritative change is required.

## Repository observations

The current repository records the following facts:

- ADR-0018 reduces the active requirement set from 121 v4.9 identities to a bounded v5.0 core of 62 active requirements, with explicit dispositions for the other 59 identities.
- The exact v5.0 authority-reset candidate is constructed and registered for independent review, while active mainline authority remains v4.9 until separate review and activation complete.
- The next recorded critical-path blocker is `workspace/0014-property-read-plan-artifact-boundary.md`.
- That topic must resolve the host-erased and static Rust representation of the logical-plan, binding-artifact, compiler-input, and compiler-extension boundary before the WP-200 Property Read plan slice can be admitted truthfully.
- The Property Read architecture gate still requires ordered WP-100, WP-200, WP-300, and WP-400 slices, each with its own dependency-complete admission and evidence boundary.
- The WP-100 Property Read handler slice is implemented, but the corresponding Planning, Binding, and Servient slices remain blocked or planned.
- Recent progress has included substantial authority, candidate, audit, registry, checker, review, and completion-evidence work in addition to the implemented Core slices.
- The current Servient, binding, and subscription implementation remains migration source for several accepted target contracts, so the target executable vertical path is not yet present.

## Owner concern

The Project Owner is concerned that reducing the active requirement count may not, by itself, reduce the amount of pre-implementation process on the critical path.

The concern is that the project could complete the v5.0 authority reset and then enter another extended sequence of workspace investigation, contract freezing, candidate construction, audit registration, checker construction, independent review, admission checkpoints, and state migration before producing the next executable cross-package behavior.

If that pattern is structurally unbounded, the project may continue improving the precision of implementation readiness without converting that precision into an end-to-end executable architecture at a comparable rate.

The concern is not an instruction to bypass `workspace/0014`, weaken ADR-0013, remove independent review, reduce evidence quality, or start WP-200 implementation prematurely. It is a question about whether the current system distinguishes necessary risk control from a self-sustaining pre-implementation documentation loop.

## Problem to investigate

The repository does not currently make clear whether the v5.0 reset changes only the size of normative authority or also shortens the executable critical path.

The following possible problems require investigation without assuming that any of them is already proven:

1. the number of active requirements may fall while the number of required transition artifacts and serial checkpoints per implementation slice remains unchanged;
2. every newly exposed design ambiguity may create another workspace-to-ADR-to-candidate-to-review cycle before any executable composition can advance;
3. governance and evidence artifacts may individually be justified while their cumulative dependency chain has no explicit throughput or completion bound;
4. independent review boundaries may serialize work that is described as parallel at the milestone level;
5. the Property Read vertical slice may remain the stated critical path without being the dominant recipient of repository effort;
6. completed local Core slices may increase contract precision without reducing uncertainty at the first cross-package construction boundary;
7. checker and registry completeness may become a stronger visible progress signal than executable Planning, Binding, and Servient integration;
8. the project may lack a repository-grounded way to identify when further pre-implementation refinement has diminishing risk-reduction value;
9. the reset may defer inactive requirements successfully while leaving the same high-cost admission shape for each of the 62 active requirements;
10. repeated updates to `PLAN.md`, `PROJECT_STATE.md`, audits, and evidence may preserve continuity while making the next implementation boundary harder to see rather than easier.

## Questions the decision must answer

1. After the exact v5.0 candidate is activated, what is the complete repository dependency chain before the first WP-200 Property Read implementation change may be admitted?
2. Which steps in that chain are mandatory consequences of current architecture and safety contracts, and which exist because of the current governance implementation?
3. Does `workspace/0014` have a bounded decision boundary, or can resolving one Rust representation question recursively expose additional pre-admission questions with no defined stopping condition?
4. Can the project currently state, from repository evidence, what observable event marks the end of design preparation and the beginning of WP-200 implementation?
5. Does ADR-0013 define sufficient criteria for refusing additional decomposition once a tranche is dependency-complete, or only criteria for admitting work after decomposition has already converged?
6. Has D1 risk-proportional admission materially reduced the artifact and review burden of completed Category A work, and can that result predict the burden of the Category B WP-200 boundary?
7. Are the M1, M2, and Property Read gate tracks actually progressing in parallel, or do their exact review and authority dependencies make the critical path serial in practice?
8. What proportion of repository change since the Property Read handler implementation has advanced executable behavior, and what proportion has maintained or changed governance, authority, audit, and evidence machinery?
9. Did those non-implementation changes remove specific blockers that could not have been removed with a smaller artifact boundary?
10. Does the v5.0 reset eliminate repeated authority-only migrations, or does it merely replace them with domain-entry re-adoption and tranche-specific candidate cycles of similar cost?
11. Can the project complete the WP-200 Property Read plan slice while leaving unrelated active requirements unresolved, or do global gate and evidence interactions still pull unrelated domains back onto the critical path?
12. Is the present level of independent candidate review proportionate to the reversibility and implementation risk of each boundary, particularly when a candidate changes no runtime or public API?
13. Are checker additions proving stable invariants that implementation could violate, or are some checkers primarily proving the consistency of other process artifacts?
14. Does `PROJECT_STATE.md` expose one unambiguous next executable objective, or can a fresh session validly continue with further process refinement without confronting the missing vertical slice?
15. What evidence would prove that the concern is false and that the current process already has a bounded conversion from design uncertainty to executable progress?
16. If the concern is valid, which exact repository mechanism creates the bottleneck: normative authority, work-package decomposition, tranche admission, independent review, evidence registration, checker composition, continuation-state maintenance, or an interaction among them?
17. Does resolving the problem require an authoritative governance or architecture change, or is it an execution-state issue under rules that are already sufficient?
18. How should the project measure progress toward v1 when local contract completion, architecture closure, and executable vertical integration advance at different rates?

## Required evidence and analysis

The investigation must be grounded in the current repository rather than the Owner's impression alone. It must inspect at least:

- the exact active dependency path in `PLAN.md`, `PROJECT_STATE.md`, and `docs/work-packages/index.toml`;
- ADR-0013, ADR-0018, D1 admission policy, and the current Property Read architecture gate;
- `workspace/0014-property-read-plan-artifact-boundary.md` and every authority or work-package edge that prevents its direct resolution;
- the commit and path history from the WP-100 Property Read handler implementation through the v5.0 candidate registration;
- the source, test, fixture, audit, evidence, registry, checker, and state changes in that interval;
- the blockers each non-implementation change claims to remove and whether those blockers were on the Property Read executable critical path;
- the current Planning, protocol-binding, and Servient source to distinguish missing implementation from unresolved contract;
- the exact admission material that would still be required after `workspace/0014` reaches a technical decision;
- whether any current rule permits repeated refinement without an explicit decision or implementation boundary;
- whether existing checks already prevent the risks that further proposed process artifacts would claim to address.

The result must distinguish:

- necessary architecture discovery;
- necessary implementation admission evidence;
- necessary independent review;
- continuity and traceability maintenance;
- duplicated or recursively generated process obligations, if any;
- executable behavior added or unblocked;
- uncertainty that remains despite the completed process work.

## Constraints

- Do not treat this concern as proof that the current process is excessive.
- Do not assume that implementation speed is more important than correctness, portability, lifecycle safety, resource bounds, or protocol neutrality.
- Do not assume that existing governance must be preserved merely because it is already implemented.
- Do not propose or implement a remedy before identifying the exact cause from repository evidence.
- Do not use raw document count, line count, commit count, or elapsed time as sufficient evidence without relating it to dependency removal and executable progress.
- Do not classify all non-code work as overhead; determine what risk or authority boundary each artifact actually closes.
- Do not classify all independently reviewed artifacts as necessary merely because current process requests them.
- Do not admit WP-200, create planned architecture fixture roots, or alter runtime/public API under this topic.
- Do not pre-decide `workspace/0014`; its technical answer remains independently owned by its investigation.
- Do not answer only with a statement that implementation will resume after the current review. Determine whether the repository mechanism makes that transition bounded and enforceable.
- Preserve the AI-led model: the Owner supplies the concern, while AI owns the technical judgment and any resulting decision.

## Expected decision output

The AI should determine:

1. whether the claimed risk of a recurring pre-implementation documentation loop is supported by repository evidence;
2. the exact mechanism or interaction responsible if the risk exists;
3. whether the issue is already bounded by current governance and work-package rules;
4. whether the v5.0 reset materially changes executable critical-path length rather than only normative-set size;
5. the exact effect of the current process on the transition from `workspace/0014` to the WP-200 Property Read plan slice;
6. whether an authoritative decision or migration is required;
7. which existing claims, plans, states, or evidence must be corrected if the current repository overstates or understates executable progress;
8. the conditions under which this topic can move from `OPEN` to `DECIDED`, and from `DECIDED` to `MIGRATED` if a repository change is required.

This topic deliberately provides no preferred solution, alternative set, target process, artifact limit, review threshold, or implementation schedule. The decision belongs to the AI after repository-grounded investigation.
