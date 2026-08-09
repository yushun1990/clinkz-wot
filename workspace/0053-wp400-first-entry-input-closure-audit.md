# 0053 WP-400 First-Legal-Entry Input Closure Audit

Status: OPEN

Kind: Owner review concern on cross-package integration completeness and feedback latency

Priority: CRITICAL PATH REVIEW

Target: the first legal WP-400 Property Read Servient entry after the route-reservation correction represented by draft pull request #18

## Owner concern

The Property Read vertical slice has now exposed two successive cross-package handoff gaps immediately before WP-400:

1. the completed WP-300 Producer registration could not feed a real `ProducerRoute` artifact from Planning into the real `PrepareInput` boundary, which required the later `WP-200-PROPERTY-READ-PRODUCER-ROUTE-PROJECTION`; and
2. reconstruction of the WP-400 candidate then found that `RouteReservationIdentity` was still created by fixture constants rather than carried by a production output, leading to the route-reservation correction currently represented by draft pull request #18.

Both findings are valuable and appear to demonstrate that the existing independent-review and successor-entry machinery can reject false integration claims. The concern is not that either correction is unnecessary. The concern is that the current validation approach may still discover the WP-400 input contract one missing field or identity at a time.

If the route-reservation correction is completed and the next WP-400 candidate again discovers another fixture-owned value, reconstructed identity, missing generation-bearing value, hidden resource fact, or lifecycle prerequisite, the project may continue to insert narrow corrective tranches between already-completed package-local slices. That can preserve local correctness while increasing feedback latency and evidence/governance complexity faster than product capability.

The Owner therefore asks AI to investigate whether the project should perform one bounded, consumer-backward completeness review of the first legal WP-400 Property Read entry before allowing the next source-admission attempt.

This topic records the concern and questions only. It does not prescribe a new gate, reopen any completed tranche, invalidate pull request #18, or pre-decide the technical mechanism.

## Repository observations to investigate

AI should verify these observations against the fetched default branch, pull request #18, specifications, work-package contracts, fixtures, and source rather than accepting them as conclusions:

- the first Producer-route correction proved a real `ProducerRoute` / `BindingArtifactRef` reaches `PrepareInput`, but that proof did not reveal the missing canonical route-reservation producer until WP-400 reconstruction;
- the current route-reservation correction proposes compiler-owned canonicalization and preservation through Core/Planning so WP-400 can construct a real route key without fixture synthesis;
- the route-reservation candidate is intentionally narrow and should not be widened merely to answer unrelated future WP-400 concerns;
- current successor-entry evidence focuses strongly on carrying a declared upstream output into the first legal downstream type, but it may not yet prove that the downstream consumer's complete required input closure is production-reachable;
- the project already has substantial candidate/review/admission/completion machinery, so any stronger completeness proof must justify its cost and avoid becoming another unbounded governance layer.

## Questions for AI decision

### 1. Is there a distinct missing evidence concept?

- Does the repository already contain an authoritative requirement that effectively proves the complete input closure of a successor's first legal entry, making this concern only an execution mistake?
- Or does D43/successor-entry evidence prove one declared handoff at a time without requiring consumer-complete enumeration?
- Is the repeated discovery of `ProducerRoute` and then `RouteReservationIdentity` evidence of a systematic validation gap, or are these unrelated defects that do not justify a new review shape?
- What exact evidence would distinguish a real systematic gap from normal vertical-slice discovery?

### 2. What exactly is the first legal WP-400 entry?

AI should identify the concrete production boundary rather than reason from fixture names.

- Which WP-400 function/type/state transition is the first point where a Servient can legally begin constructing or admitting one Property Read Producer route?
- Which values must be available at that boundary for the operation to proceed without reading the TD again, calling legacy selection, inspecting opaque protocol payloads, or synthesizing test-only state?
- Which inputs are mandatory immediately, which may be produced later by WP-400 itself, and which belong to later lifecycle/activation stages?
- Is `PrepareInput` itself the complete first-entry boundary, only one ingredient of it, or an earlier WP-300 execution boundary that should not be used as a proxy for WP-400 completeness?

### 3. Consumer-backward input closure

Starting from the real first WP-400 entry, should AI enumerate every value required to construct the first legal route and trace each value backward to one production owner?

Candidate classes to investigate include, without assuming all are required at the same point:

- Producer-route artifact and artifact slot/reference;
- canonical route reservation / route identity;
- binding registration identity;
- binding id and binding generation;
- plan id and plan-set generation;
- configuration / compatibility identity;
- artifact role;
- Thing/generation identity;
- operation or handler target identity;
- route key components;
- readiness / acceptance metadata such as any `AcceptHint`-like input;
- resource-admission facts;
- cleanup/lifecycle ownership facts;
- activation/publication authority inputs;
- any security/context state required before a route may become observable.

For each real requirement, AI should determine:

1. who owns semantic creation of the value;
2. which production object carries it;
3. whether it is owned, borrowed, generated, or recomputed;
4. how generation identity is preserved;
5. whether host-erased and static/constrained paths preserve the same semantic value;
6. whether the current fixture obtains it from real production output or manufactures/restates it;
7. whether any protocol-neutral layer is interpreting protocol-specific data to recreate it;
8. whether resource/lifecycle accounting follows the same path.

### 4. What counts as unacceptable fixture synthesis?

- Which fixture constants are harmless test inputs and which conceal missing production handoffs?
- Should a fixture be allowed to construct an identity when production code would also receive that identity from the application at the same legal boundary?
- How should the check distinguish legitimate root inputs from values that must have been produced upstream?
- Is a value still synthetic if the fixture restates bytes that can be derived from a real upstream object but deliberately bypasses that object?
- Should tests require object provenance or only semantic equality?

### 5. Completeness versus over-freezing

- Can a bounded audit prove only the input closure needed for the first Property Read route without freezing broad WP-400 internals prematurely?
- Which Servient decisions must remain private implementation details until WP-400 source exists?
- Could an exhaustive pre-source schema accidentally force speculative types for later subscription, event, action, multi-route, security, or publication behavior?
- What is the smallest closure boundary that gives fast architectural feedback while preserving room for implementation learning?

### 6. Relationship to pull request #18

- Should this investigation remain disjoint from the current route-reservation candidate so PR #18 can complete its own independently reviewed scope unchanged?
- Should the resulting completeness review happen only after PR #18 is integrated and its production metadata path exists?
- Can useful non-authoritative analysis proceed before integration without claiming that the dependent WP-400 path has advanced?
- If the audit discovers another route-reservation-adjacent omission, what criteria decide whether PR #18 should be reopened versus a distinct correction being warranted?

### 7. Relationship to WP-400 admission

- Should a successful consumer-backward closure review become evidence required for the WP-400 Property Read candidate, for pre-source admission, or only an optional review technique?
- Would making it a permanent global governance rule be disproportionate when the risk is concentrated at the first cross-package vertical slice?
- Can it be a one-time `PROPERTY-READ-ARCHITECTURE` feedback artifact rather than a new project-wide gate?
- What exact failure should block WP-400 source: an unowned required value, fixture-only provenance, illegal recomputation, missing generation, impossible resource ownership, or something else?

### 8. Evidence and complexity budget

- Can existing compile-contract fixtures, architecture-gate manifests, audits, and checkers express the result without creating another large parallel evidence system?
- Which existing artifact should own the closure result if the concern is valid?
- Can one generated/structured input table replace many prose/checker projections?
- Should the project measure or at least report evidence-to-product-source growth for corrective tranches as a warning signal rather than a blocker?
- At what point does additional admission machinery provide less risk reduction than an executable mock or real vertical integration fixture?

### 9. Negative mutations / falsifiability

If a completeness audit is adopted, what finite mutations demonstrate that it is actually capable of catching the class of error that motivated this topic?

Examples for AI to evaluate:

- replace one production-carried identity with a fixture constant;
- drop a generation component while retaining otherwise matching ids;
- reconstruct canonical route identity in Planning or Servient;
- lose metadata during host erasure while static mode still passes;
- hide required metadata only inside an opaque binding payload;
- provide a real artifact reference but an unrelated route reservation;
- satisfy a type-level field while leaving no production owner capable of constructing it;
- pass one feature cell while constrained/static or host semantics diverge.

The test set should remain finite and risk-proportional rather than attempting to mutate every field mechanically.

## Constraints

Any decision should preserve these project constraints unless repository evidence justifies changing their authoritative owner:

- completed package-local claims remain valid unless a newly discovered defect intersects what they actually claimed;
- no successor source is admitted merely because a predecessor merged;
- no fixture-owned substitute may stand in for a declared production cross-package handoff;
- Planning must not regain protocol-specific endpoint interpretation merely to fill a downstream identity;
- Servient must not inspect opaque binding payloads or recreate protocol canonicalization that belongs to the concrete binding/compiler;
- protocol-specific execution authority must not leak back into Planning;
- the investigation must not force broad WP-400 design or unrelated affordance behavior before the first Property Read entry needs it;
- host and constrained/static profiles must preserve shared semantic identity even when physical representation differs;
- no new permanent gate/checker/document family should be introduced unless existing evidence owners cannot express the required claim;
- the purpose is to shorten architectural feedback and prevent serial corrective discovery, not to require certainty about the entire v1 design before implementation.

## Expected AI output

AI should investigate the repository and decide one of the following, or a better evidence-supported alternative:

1. **No new mechanism:** existing successor-entry evidence is sufficient; identify the concrete execution mistake that allowed the two gaps and correct only that practice.
2. **One-time bounded WP-400 closure review:** define the exact first-entry boundary, enumerate its production-required inputs, prove backward provenance, and place the result in an existing Property Read architecture/admission artifact.
3. **Reusable successor input-closure rule:** if evidence shows the same risk applies systematically across cross-package entries, define the smallest reusable rule and migrate it to the existing governance/evidence owner without creating redundant machinery.

The decision should explicitly state:

- the real first legal WP-400 boundary;
- the complete required input set at that boundary;
- production owner/provenance for each required input;
- what fixture synthesis is permitted versus forbidden;
- whether PR #18 is affected;
- whether and when WP-400 candidate/source preparation is blocked;
- the smallest executable/negative evidence needed;
- how the decision avoids another open-ended evidence expansion cycle.

## Non-goals

This topic does not ask AI to:

- redesign the entire Servient before WP-400 begins;
- reopen the completed WP-100, original WP-200 plan, WP-300 binding, or Producer-route projection solely because a later consumer has more inputs;
- add all future subscription/event/action/publication requirements to the Property Read slice;
- replace executable vertical integration with documentation;
- prohibit test constants generally;
- preselect a specific table, schema, checker, work-package tranche, or ADR shape;
- declare PR #18 technically correct or incorrect before its independent review;
- delay unrelated, dependency-complete work.
