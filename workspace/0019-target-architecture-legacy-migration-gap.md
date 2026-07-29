# 0019 Target Architecture and Legacy Migration Gap

Status: MIGRATED

Kind: owner-raised architecture-migration investigation

Priority: HIGH

Target: migration from the current legacy execution boundaries to the accepted v5 Planning, Protocol Binding, and Servient model

## Scope and authority

This topic records a Project Owner concern about the gap between the accepted v5 target architecture and the current product implementation. It asks whether the migration path is sufficiently explicit to avoid preserving incompatible legacy execution paths or creating an intermediate architecture with unclear ownership.

The concern is an investigation input. It does not assert that the accepted target is wrong, does not select a migration strategy, and does not instruct Codex to preserve or remove any specific legacy type. Codex owns the repository-grounded decision.

## Repository observations

The repository records that:

- the planned Planning compiler and binding-artifact contracts are not yet implemented in product Rust;
- current form selection remains in the legacy Protocol Binding core path;
- Servient still stores direct client and server binding trait objects;
- existing binding paths still reflect direct execution and dispatch boundaries that differ from the accepted target;
- the accepted direction assigns immutable plan and artifact production to Planning, protocol syntax and I/O to bindings, and orchestration and handler progress to Servient;
- WP-200, WP-300, and WP-400 are expected to migrate these boundaries in ordered slices.

## Owner concern

The Project Owner is concerned that implementing the new slices incrementally may leave old and new execution paths active at the same time without a clearly bounded compatibility, removal, or authority model. The concern includes whether legacy helpers could silently remain authoritative after the target architecture appears to exist.

## Questions for investigation

1. What exact current product types and call paths embody the legacy form-selection, binding-registration, dispatch, and Servient ownership model?
2. Which of those paths are intended migration inputs, temporary compatibility surfaces, or obsolete target conflicts?
3. At what exact tranche does each legacy responsibility move to its accepted owner?
4. Can old and new plan, selection, registration, dispatch, or activation paths coexist, and if so, what authority and lifecycle rules distinguish them?
5. Is there any route by which a Protocol Binding can continue selecting or dispatching handlers after the new SPI is introduced?
6. Is there any route by which runtime code can continue rescanning or reinterpreting TD forms after immutable planning artifacts exist?
7. How are generation, configuration, compatibility, role identity, cancellation, and cleanup preserved across each migration boundary?
8. Do the registered WP-200, WP-300, and WP-400 slices identify all source owners and removal obligations needed for the transition?
9. Could a narrow Property Read path pass while a conflicting legacy path remains available elsewhere?
10. What evidence would prove that the migration has one coherent authority at every intermediate checkpoint?
11. If the migration boundary is incomplete, which authoritative specifications, work packages, tests, or completion evidence must consume the decision?

## Constraints

- Do not assume that all legacy code must be removed immediately.
- Do not assume that compatibility is required merely because current code exists.
- Do not prescribe a flag day, adapter layer, dual-path period, or removal sequence before investigation.
- Do not reopen accepted Planning, Binding, or Servient ownership solely because implementation has not yet migrated.
- Do not treat passing narrow fixtures as proof that all legacy conflicts are resolved unless repository evidence supports that conclusion.
- Preserve the AI-led model.

## Expected decision output

Codex should determine:

1. the exact legacy-to-target migration map;
2. whether any unowned, duplicated, or conflicting intermediate execution authority exists;
3. whether current work-package and completion boundaries fully cover migration and removal truth;
4. what evidence closes the migration risk for the Property Read slice and later broad entry;
5. the conditions for moving this topic through its workspace lifecycle.

## Decision

The migration has one staged authority map:

| Current legacy responsibility | Target owner and closure |
| --- | --- |
| TD form selection and target/security resolution in `protocol-bindings/core/src/form.rs` | WP-200 Planning owns selection and immutable artifacts; broad WP-200 completion removes the shared legacy selection authority |
| `core/src/inbound.rs` / `outbound.rs` direct `ServerBinding`, `ClientBinding`, `BindingContext`, and dispatch/call shapes | WP-300 replaces them with complete registration, route-scoped progress, owned calls, and explicit cleanup |
| Servient storage of bare binding trait objects and direct serve/shutdown/dispatch integration | WP-400 installs only complete startup bundles and owns publication, selection, progress, and cleanup |
| Zenoh and zenoh-pico TD/form rescanning and legacy binding implementations | WP-600 compiles protocol-local artifacts and migrates both concrete execution paths |
| Umbrella aliases and remaining compatibility entry points | WP-700 negative fixtures and source inspection prove final absence |

Old and new code may coexist only while a named downstream caller is
unmigrated and only through the one-way adapters registered by WP-300,
WP-400, or WP-600. For one Servient generation there is exactly one planning,
registration, dispatch, and activation authority. A binding cannot select a
handler, reinterpret a TD, or bypass the Servient merely because its legacy
helper still exists for an unmigrated path.

The narrow Property Read gate intentionally does not prove repository-wide
legacy removal. It proves that one target path composes without a shortcut.
Broad package completion owns removal at each boundary, and WP-700 owns the
final no-legacy claim. This prevents a passing narrow fixture from overstating
migration completion.

## Migration

The selected map is already projected in the Old API Removal and completion
sections of WP-200, WP-300, WP-400, WP-600, and WP-700. D12 in `PLAN.md` and
the continuation summary in `PROJECT_STATE.md` now make the intermediate
authority rule explicit. No target architecture decision is reopened.
