# 0042 Protocol Binding SPI Validation and Freeze Risk

Status: MIGRATED

Kind: owner-raised architecture-validation and public-SPI maturity investigation

Priority: HIGH

Target: the progression of the public Protocol Binding SPI from the narrow WP-300 Property Read candidate through broad WP-300, WP-400 composition, and WP-600 production bindings

## Scope and authority

This topic records a Project Owner concern about whether the current evidence order, public-surface freeze points, reopening predicates, and validation claims are sufficient to establish an effective Protocol Binding SPI.

It does not assert that the current SPI is defective, prescribe an alternative SPI, authorize bypassing the current work-package order, or block the reviewed narrow WP-300 source path merely by existing. Codex owns the repository-grounded technical decision.

## Repository observations

- The narrow WP-300 candidate defines an exact public Property Read registration and execution surface before its product source and the first cross-package target-generation runtime composition exist.
- The current evidence ladder distinguishes package-local constructibility, mock cross-package Property Read composition, real Zenoh Property Read operation, and final release readiness.
- The planned external Zenoh authoring spike follows narrow WP-300 completion and precedes broad WP-300 admission; the first authoritative production Zenoh runtime smoke belongs to WP-600 after broad WP-300 releases that package.
- The current SPI reopening predicate names concrete ownership loss, impossible portable representation, unaccounted resources, unimplementable lifecycle, or required unsafe/private dependencies. Repetitive declarations are currently treated as helper or generation concerns rather than semantic defects.
- Zenoh and zenoh-pico are the planned production bindings. They exercise host and constrained representations of the same protocol family.
- The narrow Property Read slice excludes client invocation, subscriptions, Producer emission, collection capabilities, broad cancellation races, multi-route behavior, production transport behavior, and Servient scheduling.
- Host and constrained profiles are required to share semantic transitions, trace identifiers, observable outcomes, and resource deltas while their storage, wake, executor, and critical-section mechanisms may differ.
- Candidate checkers, lifecycle schemas, negative mutations, feature matrices, and independent reviews provide strong internal consistency evidence, while production protocol and independent third-party author evidence remain downstream.

## Questions for investigation

### Public-surface freeze versus executable feedback

1. Which parts of the current Protocol Binding SPI are semantic invariants that should already be treated as frozen, and which Rust signatures remain provisional until executable feedback exists?
2. Does the current sequence obtain real protocol-library feedback early enough to prevent broad WP-300 from depending extensively on an interface that has only mock constructibility evidence?
3. What exact claim is justified when narrow WP-300 source completes but no cross-package runner or production transport has executed?
4. What exact claim is justified by the planned external Zenoh authoring spike, and what remains unproved even if that spike compiles successfully?
5. At which checkpoint would changing a public Rust signature be a normal candidate correction, and at which checkpoint would it constitute reopening a stable SPI decision?

### SPI reopening threshold and practical usability

6. Are the current reopening predicates sufficient to identify an SPI that is technically implementable but systematically difficult or error-prone for third-party authors?
7. What repository evidence distinguishes mechanical boilerplate that belongs in helpers or generators from public-surface complexity that indicates a semantic decomposition problem?
8. Can repeated identity, lifecycle, resource, cleanup, or profile declarations remain individually correct while collectively creating an unacceptable implementation burden?
9. What observable authoring failures, diagnostics, maintenance costs, or repeated workarounds would justify reopening the public SPI without proving outright impossibility?
10. How should Codex distinguish an awkward API that is valuable design feedback from unavoidable complexity required by the ownership and resource model?

### Protocol neutrality

11. What evidence is required before the public SPI may claim protocol neutrality rather than compatibility with the Zenoh protocol family and repository-owned mock bindings?
12. Do Zenoh and zenoh-pico provide independent protocol-shape evidence, or primarily representation evidence for one protocol model?
13. Which protocol characteristics could expose assumptions in route readiness, committed-closed guards, permit-gated acceptance, correlation, reactor progress, response opportunity, or cleanup ownership?
14. At what project stage must protocol-shape diversity be evaluated to avoid both premature abstraction and late public-SPI correction?
15. Which conclusions about HTTP-like, broker-mediated, datagram, constrained link, or other transport models are currently unsupported by repository evidence?

### Narrow Property Read influence on broad SPI

16. Which current types and transitions are truly operation-neutral, and which may be shaped specifically by one-shot Producer Property Read?
17. What evidence is required before the Property Read route, response, settlement, and cleanup model can be generalized to client calls, long-lived subscriptions, Producer emission, collection capabilities, and action invocation?
18. Could broad optional interfaces preserve source compatibility while still forcing unrelated operation families into an unsuitable Property Read lifecycle model?
19. What domain-entry findings would require a local extension, a new operation-specific component, or reconsideration of a shared semantic kernel?
20. How will broad WP-300 prevent completion of the narrow slice from being interpreted as validation of inactive operation families?

### Host and constrained parity

21. Does the current parity rule constrain semantic outcomes only, or does the planned Rust surface also impose unnecessary structural symmetry between host and constrained authors?
22. Which differences in async facade, manual polling, state storage, wake integration, executor use, and cancellation driving are compatible with one semantic kernel?
23. Could enforcing identical trace outcomes and resource deltas cause either profile to model capabilities or costs that do not exist naturally on that platform?
24. What evidence would show that host convenience layers remain faithful projections rather than a second effective SPI?
25. What evidence would show that constrained support is more than compile-only surface availability while avoiding unsupported runtime claims?

### Internal consistency versus external validity

26. Which current checks prove conformance to the selected model, and which checks provide evidence that the selected model matches real protocol and third-party implementation constraints?
27. Could candidate schemas, negative mutations, and independent reviews all pass while omitting an external assumption that only a production library or independent author would expose?
28. What minimum external evidence is required at narrow WP-300 completion, broad WP-300 entry, PROPERTY-READ-ARCHITECTURE closure, WP-600 production smoke, and final release review?
29. How should progress reporting prevent internal consistency evidence from being described as production implementability, protocol neutrality, author ergonomics, or v1 stability?
30. Do any current milestone, work-package, specification, or continuation statements overstate the maturity of the Protocol Binding SPI relative to their actual evidence?

## Constraints

- Do not infer that the SPI is defective solely because production bindings and downstream work are incomplete.
- Do not block the reviewed narrow WP-300 source path merely because this topic is OPEN.
- Do not weaken ownership, lifecycle, generation, resource, cleanup, or no-hidden-dispatch requirements without repository-grounded evidence.
- Do not treat mock success as production-protocol, protocol-neutrality, author-ergonomics, or release-readiness evidence.
- Do not treat field count, subjective complexity, or preference for a shorter API as sufficient proof of a semantic defect.
- Do not prescribe a particular protocol prototype, helper design, conformance framework, or implementation sequence before Codex classifies the questions against current repository evidence.
- Do not add a new blocking gate unless it protects a distinct falsifiable claim not already owned by an existing work package or gate.

## Expected decision output

Codex should:

1. classify which concerns are already resolved by D24, D25, D29, existing specifications, and registered work-package evidence;
2. identify any remaining confirmed risk, unresolved uncertainty, or unsupported maturity claim;
3. define the exact maturity and claim boundary of the SPI at each existing checkpoint;
4. decide whether the current evidence order and reopening predicates are sufficient;
5. identify any authoritative specification, architecture, work-package, plan, checker, fixture, audit, or continuation projection that requires correction; and
6. migrate only conclusions supported by repository evidence.

## Decision

The existing narrow WP-300 order remains valid because it freezes one finite
Property Read candidate and then executes exactly that package-local
constructibility claim. It does not prove production operation, broad
operation-family fit, third-party ergonomics, protocol-shape neutrality,
cross-package orchestration, or release stability.

SPI maturity advances through six named evidence levels: immutable internal
consistency, narrow package-local constructibility, external Zenoh authoring,
mock cross-package composition, production Zenoh-family execution, and final
release evidence. Public semantic invariants—generation identity, complete
ownership, permit-gated acceptance, bounded progress, cleanup transfer, and no
hidden dispatch—remain frozen. Exact Rust containers and operation-family
signatures remain normal broad-candidate correction points until their
external-authoring/domain evidence closes; later changes require explicit
reopening and migration.

The Zenoh spike stays before broad WP-300. Its reopening predicate is expanded
beyond impossibility: repeated workarounds that lose semantic truth, duplicate
normative transitions, require unsafe/private dependencies, produce unusable
diagnostics, or exceed declared generic/layout/code-size bounds are concrete
defects. Mechanical repetition or field count alone still justifies
helpers/generation rather than semantic relaxation.

Zenoh and zenoh-pico are one protocol family. They provide meaningful Host
versus constrained representation evidence but not independent protocol-shape
neutrality. A release claim of empirical protocol neutrality therefore needs a
materially different independently authored route/correlation/response/
cancellation fixture; otherwise the honest claim is protocol-independent
engine ownership plus Zenoh-family operation.

## Migration

The maturity ladder, broader reopening predicate, protocol-family limitation,
and release claim boundary are projected into `PLAN.md`,
`docs/spec/binding-spi.md`, `docs/work-packages/WP-300-bindings.md`,
`docs/work-packages/WP-600-protocol-bindings.md`, and
`docs/work-packages/WP-700-integration.md`. Narrow WP-300 remains unblocked.
This topic is `MIGRATED`.
