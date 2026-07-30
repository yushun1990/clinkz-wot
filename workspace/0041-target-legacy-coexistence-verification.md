# 0041 Target and Legacy Coexistence Verification

Status: MIGRATED

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

## Decision

Distinct target types are necessary but do not alone prove no backflow. The
staged evidence is:

1. WP-300 compiles the target registration/request/response surface without a
   protocol-bindings or Servient dependency, source-scans the target module for
   legacy selectors/dispatch, exposes no target-to-legacy conversion, and
   rejects mismatched compiler, artifact, binding, configuration, route, and
   plan generations.
2. WP-400's target runner imports only target types and arms legacy selector,
   `ServerBinding::serve`, and `Dispatch` paths with poison counters/panics;
   the target Property Read trace must complete with every poison count zero.
3. WP-600 removes concrete selector and legacy execution calls for Zenoh and
   zenoh-pico and proves compiled targets enter only the target SPI.
4. WP-700 uses negative public compile fixtures, dependency/source inspection,
   and the full feature matrix to prove the selector family, legacy root
   exports, and every named adapter edge are absent.

The only temporary positive adapter remains the already named one-way legacy
handler-publication to `ProducerEmission` bridge. WP-300 owns its contract,
WP-400 and WP-600 remove their respective ends, no new caller may enter it,
and no adapter may translate a target artifact/request back to a TD, form, or
legacy server call.

Tests claiming target-generation coverage must construct a target plan,
artifact, registration, route, permit, request, response opportunity, and
generation chain. A legacy fixture cannot satisfy that claim. Generation
mixing negatives are required at each owning package.

Issue 0027 is reopened only by concrete evidence that a required target
operation cannot be expressed without information owned solely by the legacy
path, that identity/ownership cannot survive the one-way migration, or that a
production binding cannot implement the target SPI. Convenience or existing
legacy test coverage is not such evidence.

## Migration

The staged evidence and poison-boundary requirements are projected into the
Property Read gate and WP-300/WP-400/WP-600/WP-700 work-package evidence. This
topic is `MIGRATED`.
