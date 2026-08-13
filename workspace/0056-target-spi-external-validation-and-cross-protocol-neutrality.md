# 0056 Target SPI External Validation and Cross-Protocol Neutrality

Status: OPEN

Kind: owner-raised architecture investigation

Priority: HIGH

Target: the external-validation boundary for the v5 target binding/Servient SPI before further aggregate mock closure or API freezing

## Scope and authority

This topic records a Project Owner concern, strengthened by an independent Ultra project review, that the current narrow Property Read slices and repository fixtures may establish internal consistency before the target Protocol Binding SPI has received enough feedback from real protocol implementations.

This is an investigation request, not an instruction to replace the current SPI, skip the Property Read architecture gate, adopt a particular alternative, or make Zenoh the protocol-neutrality proof. Accepted architecture remains implementation authority until Codex investigates the evidence and migrates any stable correction through normal architecture governance.

## Owner observation

The project has now proved important protocol-neutral ownership properties through narrow Planning, Binding, and Servient Property Read slices, while the real Zenoh implementation still follows the legacy path and the target binding interfaces are exercised primarily by compile/runtime fixtures and mocks.

An independent Ultra review therefore raised a credible counterexample to the current feedback order: continuing aggregate mock closure before using a real target binding may produce more evidence that the repository matches its declared model without testing whether that model is natural for actual protocol I/O, correlation, cancellation, buffering, readiness, cleanup, and concurrency.

The Owner agrees that a real Zenoh target slice is a useful first external probe, but explicitly rejects treating Zenoh alone as proof or disproof of protocol neutrality. Zenoh can test whether the current SPI naturally represents one real protocol family. A protocol-neutrality claim requires evidence across materially different protocol interaction models, or another justified method that can expose protocol-specific assumptions.

## Independent-review evidence to investigate

The Ultra review reported, among other findings:

- the TD -> Planning -> Binding -> Servient macro layering remains credible;
- immutable plan/artifact execution, Servient-owned handler selection/publication/cleanup, generation-bearing identity, bounded caller-driven progress, and protocol-owned I/O/correlation remain strong principles;
- target Host/static authoring and route behavior currently have substantial duplicated structure;
- prepare/readiness/activate/commit and other detailed lifecycle carriers were specified before a real target binding exercised them end to end;
- the real Zenoh target migration remains later than the current aggregate Property Read fixture path;
- narrow fixtures validate important ownership semantics but do not yet establish external validity for multiple real protocol models.

These findings are evidence inputs, not predetermined conclusions. Codex should verify them against the current fetched default branch and any newer D48 state before deciding.

## Questions for investigation

1. What architectural claims have the completed Property Read slices actually established, and which target SPI claims remain provisional until exercised by real bindings?
2. After D48 convergence, would the planned aggregate Property Read mock/fixture produce materially new architecture feedback before a real target binding, or mostly strengthen conformance to the existing model?
3. Should the roadmap place a minimal real target binding probe before, inside, or after the aggregate `PROPERTY-READ-ARCHITECTURE` gate?
4. What is the smallest real Zenoh Property Read slice that can test compiler/artifact construction, real I/O, correlation, readiness, failure, cancellation/drain, cleanup, multiple Things/routes/forms, and at least one real network round trip without turning the probe into full binding migration?
5. Which current public lifecycle stages and carrier types are genuinely required by that real Zenoh implementation, and which appear to be internal binding state or duplicated Host/static machinery?
6. Is one typed poll-driven kernel with profile-specific storage/erasure a credible simplification of the current Host/static split, or does repository evidence justify the present separation? If comparison is needed, what bounded spike would falsify either design?
7. What evidence is required before a target SPI type or lifecycle is considered stable/frozen rather than provisional?
8. What constitutes adequate evidence for protocol neutrality? In particular, what protocol-model differences must a second real binding expose beyond Zenoh so that shared abstractions are not merely Zenoh-shaped?
9. Should the second protocol be selected by transport name, or by deliberately different semantic properties such as request/response correlation, connection orientation, broker/topic routing, streaming/subscription behavior, server push, security/session ownership, constrained polling, or message-size/backpressure behavior?
10. Which candidate protocol family or families provide the highest-value contrast for v1 validation, and what is the minimum comparison needed before claiming the target SPI is protocol-neutral in practice?
11. How should real-binding feedback interact with the existing accepted ADRs, work-package gates, D48 validation machinery, and legacy/target coexistence without silently bypassing current authority?
12. What repository-visible result should end this investigation cycle: reaffirmation of the current SPI, a bounded comparative spike, a revised gate order, reopening of specific ADR/spec ownership, or another evidence-backed outcome?

## Constraints

- Preserve the protocol-neutral macro architecture unless evidence supports changing it.
- Do not equate successful Zenoh implementation with proof of protocol neutrality.
- Do not equate Zenoh friction alone with proof that the SPI is not protocol-neutral; distinguish a poor implementation, a protocol-specific mismatch, and a genuinely overfit abstraction.
- Cross-protocol evidence should be chosen for semantic diversity, not merely for adding another adapter with similar interaction mechanics.
- Do not require full production implementations merely to obtain architecture feedback; prefer the smallest real slices that can falsify important assumptions.
- Do not let a spike silently become authoritative implementation. Stable conclusions must migrate through the normal architecture-governance process.
- Do not discard boundedness, ownership, no_std constraints, cancellation/cleanup discipline, or protocol-neutral orchestration merely to reduce code volume.
- Do not continue freezing detailed SPI surface solely because existing fixtures and checkers can validate it.

## Expected decision output

Codex should determine:

1. whether current target-SPI external evidence is sufficient for the next planned aggregate gate;
2. whether a real Zenoh target probe should precede or alter that gate;
3. which current SPI details remain stable versus provisional;
4. whether a bounded alternative-kernel comparison is warranted;
5. the evidence standard for protocol neutrality and the semantic criteria for selecting at least one contrasting protocol model;
6. any required roadmap, work-package, ADR, specification, or gate changes; and
7. the exact next independently reviewable engineering claim, without beginning a materially distinct successor in the same review cycle.
