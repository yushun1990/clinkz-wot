# 0041 Target and Legacy Coexistence Verification

Status: OPEN

Kind: owner-raised migration and implementation-conformance investigation

Priority: HIGH

Target: the first target-generation Binding and Servient source introduced while legacy selector, inbound, dispatch, and concrete Binding paths remain present

## Scope and authority

This topic records a Project Owner concern that the migration rules decided in workspace topic 0027 must become executable verification once target WP-300/WP-400 code actually exists. It does not reopen the no-backflow decision without new evidence and does not require immediate removal of legacy APIs. Codex owns the implementation-conformance judgment.

## Repository observations

- Legacy form selection, inbound traits, direct Binding storage, and concrete protocol execution remain in the repository.
- Target Planning artifacts already exist, while target WP-300 execution source is the next planned implementation.
- The target path uses distinct `RouteServerBinding`, `RouteInboundRequest`, `RouteResponseOpportunity`, and `RouteInboundResponse` names.
- New target code is forbidden from rescanning TDs, calling legacy selectors, entering legacy `ServerBinding::serve`, or using hidden dispatch.
- WP-600 removes concrete call edges and WP-700 proves final public legacy/adapter absence.

## Questions for investigation

1. Which compile, dependency, source, and runtime checks prove the first target request cannot enter a legacy selector or dispatch path?
2. Are distinct type names sufficient, or are sealed modules, visibility boundaries, dependency rules, or compile-fail fixtures required?
3. Which temporary adapters are permitted, who owns them, and how are new callers prevented?
4. How are tests prevented from accidentally exercising legacy behavior while claiming target-generation coverage?
5. What generation and identity checks reject mixing a target artifact with a legacy registration or request?
6. What evidence must accompany WP-300, WP-400, WP-600, and WP-700 respectively?
7. What concrete finding would justify reopening the migrated 0027 decision?

## Constraints

- Do not remove legacy paths before their named migration consumers are ready.
- Do not allow bidirectional adapters or target-to-legacy execution fallback.
- Preserve one selection, dispatch, and activation authority per generation.
- Reuse existing completion and architecture-gate checks where they prove the same invariant.

## Expected decision output

Codex should define the staged executable no-backflow evidence, permitted adapter ownership, test isolation rules, generation-mixing negatives, and exact removal proofs for WP-300 through WP-700.