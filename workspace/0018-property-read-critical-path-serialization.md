# 0018 Property Read Critical-Path Serialization

Status: MIGRATED

Kind: owner-raised execution-risk investigation

Priority: HIGH

Target: the ordered WP-100 -> WP-200 -> WP-300 -> WP-400 Property Read vertical path

## Scope and authority

This topic records a Project Owner concern about whether the first executable Property Read vertical slice is constrained by necessary architectural dependencies or by additional serial boundaries that do not independently reduce technical risk.

The concern is an investigation input. It does not assert that the current ordering is wrong, does not authorize parallel implementation, and does not prescribe a different work-package or milestone structure. Codex owns the technical judgment from repository evidence.

## Repository observations

The repository records that:

- the WP-100 Property Read handler slice is complete;
- the WP-200 Planning slice remains before product-source admission;
- the WP-300 binding slice depends on the exact WP-200 slice;
- the WP-400 Servient slice depends on the exact WP-300 slice;
- the Property Read architecture gate is not complete until the ordered slices compose;
- broad milestone descriptions permit some parallel progress, while the executable Property Read path remains ordered in practice.

## Owner concern

The Project Owner is concerned that the project may report parallel milestone progress while the first end-to-end executable behavior remains dependent on a long serial chain. The concern is whether every boundary in that chain is technically necessary for the narrow Property Read proof and whether the repository makes the dominant critical path visible.

## Questions for investigation

1. What is the exact dependency graph from the completed WP-100 handler slice to an executable Property Read through Planning, Binding, and Servient?
2. Which edges are mandatory consequences of public contracts, ownership, lifecycle, resources, rollback, or validation truth?
3. Which edges arise from current work-package, review, checkpoint, or evidence organization rather than product architecture?
4. Can any preparation or independent evidence for later slices occur before the preceding source slice completes without claiming premature implementation admission?
5. Does the current milestone status accurately distinguish broad parallel work from the serial executable vertical path?
6. Are unrelated M1 or M2 closure activities able to block the Property Read path through shared artifacts or checks?
7. Does each narrow slice reduce uncertainty needed by the next slice, or can a slice complete locally while leaving the first cross-package construction risk unchanged?
8. What observable event marks each transition from one vertical slice to the next?
9. Does the current path contain any duplicated review or evidence boundary covering the same contract and rollback truth?
10. What repository evidence would prove that the present serialization is necessary and proportionate?
11. If unnecessary serialization exists, which authoritative owner must record the resulting decision?

## Constraints

- Do not assume that serial execution is inherently excessive.
- Do not assume that parallel work is safe or desirable.
- Do not weaken package dependencies, independent review, lifecycle ownership, resource bounds, or protocol-neutrality requirements under this topic.
- Do not prescribe a new sequencing model, tranche shape, or implementation schedule before investigation.
- Preserve the AI-led model: the Owner raises the concern, while Codex determines the technical answer.

## Expected decision output

Codex should determine:

1. the exact necessary and incidental dependencies on the Property Read vertical path;
2. whether the current roadmap and state accurately expose that path;
3. whether any serial boundary duplicates another boundary's contract, rollback, or validation truth;
4. whether authoritative records require correction;
5. the conditions for moving this topic through `OPEN -> DISCUSSING -> DECIDED -> MIGRATED`.

## Decision

The source dependency chain is necessary and remains:

```text
WP-100 handler
  -> WP-200 immutable logical plan and binding artifact
  -> WP-300 complete registration, route, accept, response, and cleanup SPI
  -> WP-400 Servient publication, route/handler selection, and orchestration
  -> PROPERTY-READ-ARCHITECTURE
```

Each edge transfers a public contract or exclusive lifecycle authority that
the next slice must consume. Removing an edge would require a fixture adapter
to impersonate a production plan, artifact, registration, route permit,
response opportunity, or cleanup owner, all of which the gate explicitly
forbids.

Preparation for a later slice may occur before its predecessor completes:
contract inspection, non-authoritative experiments, risk analysis, and draft
negative cases can reduce uncertainty. It cannot create the planned
architecture fixture root, obtain source admission, or count as vertical
progress. The transition events remain the predecessor's registered
completion evidence and the successor's own exact admission.

No duplicate source-review boundary remains. The WP-200 v2 review is a bounded
correction of admission evidence, not another review of the semantic contract.
WP-300 and WP-400 reviews cover distinct execution and orchestration ownership.
Disjoint M1/M2 work may proceed but cannot be reported as advancement of the
executable slice.

## Migration

The conclusion is migrated into D11 and the release-critical topology in
`PLAN.md`, the three-track and tranche-conversion rules in
`PROJECT_GOVERNANCE.md`, and the critical-path summary in
`PROJECT_STATE.md`. The existing gate DAG remains authoritative and unchanged.
