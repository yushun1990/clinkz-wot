# 0020 Governance and Checker Complexity

Status: OPEN

Kind: owner-raised governance-risk investigation

Priority: HIGH

Target: the cumulative interaction among governance rules, registries, audits, manifests, continuation state, and executable checkers

## Scope and authority

This topic records a Project Owner concern about whether the repository's governance and checker system remains proportionate to the implementation risks it is intended to control, or whether the control system has developed independent complexity that can create new blockers and maintenance obligations.

The concern is an investigation input. It does not classify any existing artifact or checker as unnecessary, does not request weaker governance, and does not prescribe simplification. Codex owns the technical and governance judgment from repository evidence.

## Repository observations

The repository records that:

- architecture authority, project governance, planning state, continuation state, work-package records, audits, manifests, registries, and executable checkers each have distinct stated responsibilities;
- recent critical-path work has included substantial non-product changes to those artifacts;
- some checks protect runtime, ownership, lifecycle, authority, or evidence invariants;
- some checks validate consistency among other process artifacts;
- D9 states that continuity, registry, audit, and checker changes should travel with the checkpoint whose truth they record and should not become independent critical-path prerequisites;
- a new checker is intended to protect a stable invariant not already proved by an existing executable check.

## Owner concern

The Project Owner is concerned that individually justified governance artifacts may form a cumulative dependency system whose complexity approaches or exceeds the product change it controls. The concern includes whether process-consistency checks can recursively generate correction work without independently protecting implementation behavior or architecture authority.

## Questions for investigation

1. What stable invariant is protected by each governance, registry, audit, manifest, state, and checker artifact currently on the active critical path?
2. Which invariants can be violated by product implementation, architecture change, or authority migration?
3. Which checks primarily verify consistency among other process artifacts?
4. Are any invariants proved more than once by overlapping executable checks?
5. Are any artifacts authoritative owners of truth that is also independently restated elsewhere?
6. Can a process artifact change force updates across a broad graph even when product semantics, ownership, lifecycle, resources, and public APIs are unchanged?
7. Has any checker or registry rule become an independent source of critical-path failure rather than a detector of an underlying project defect?
8. Does D9's rule for attaching support artifacts to owning checkpoints operate effectively in current commits?
9. Can a fresh Codex session identify which artifacts are mandatory for a specific tranche without reconstructing the full governance graph?
10. Does the current system distinguish traceability value, authority protection, implementation safety, and status bookkeeping?
11. What evidence would show that the cumulative governance burden is proportionate and bounded?
12. If the concern is supported, which authoritative governance owner must record the decision?

## Constraints

- Do not use raw document, line, commit, or checker counts as sufficient evidence.
- Do not classify all non-code work as overhead.
- Do not classify an artifact as necessary merely because another artifact references it.
- Do not weaken architecture authority, independent review, rollback evidence, lifecycle safety, resource bounds, or durable continuation under this topic.
- Do not prescribe consolidation, deletion, quotas, or a new governance model before investigation.
- Preserve the AI-led model.

## Expected decision output

Codex should determine:

1. the invariant and authority map for the active governance/checker system;
2. whether duplicated, circular, recursively generated, or independently blocking obligations exist;
3. whether D9 currently bounds support-artifact work as intended;
4. whether any authoritative responsibility or progress claim requires correction;
5. the conditions for moving this topic through its workspace lifecycle.
