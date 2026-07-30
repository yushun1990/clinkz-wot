# 0027 Legacy Migration Boundary Leakage

Status: MIGRATED

Kind: owner-raised architecture-migration investigation

Priority: HIGH

Target: the one-way migration boundaries between the legacy runtime paths and the active v5 target architecture

## Scope and authority

This topic records a Project Owner question about whether the staged coexistence of legacy runtime code and the active v5 architecture is fully bounded by explicit one-way adapters, and whether any implementation path could accidentally reintroduce a legacy decision after the new authority has already acted.

The question does not assert that the legacy code remains normative or that the repository currently has two architectural authorities. The active authority map is already defined by D12. Codex owns the repository-grounded judgment about whether the implementation and migration records enforce that map completely.

## Repository observations

The current repository records that:

- legacy form selection still exists in `protocol-bindings/core`;
- Servient still stores legacy `Arc<dyn ClientBinding>` and `Arc<dyn ServerBinding>` values directly;
- concrete bindings still use the legacy execution boundary;
- Planning owns selection and artifacts, WP-300 owns execution SPI, WP-400 owns orchestration, WP-600 owns concrete binding migration, and WP-700 owns final removal evidence;
- old and new paths may coexist only at named one-way migration adapters;
- an adapter is not a second public contract and may not accept new callers after its owning package completes;
- no generation may have two selection, dispatch, or activation authorities.

## Questions for investigation

1. Are all currently required legacy-to-target adapters explicitly named in authoritative work-package or specification records?
2. For each adapter, which side owns its input, output, lifecycle, generation identity, and eventual removal?
3. Can any new WP-200 artifact flow reach legacy form selection after Planning has already selected the form and target?
4. Can any new WP-300 accepted request reach a legacy binding-owned handler-dispatch path after Servient orchestration becomes active?
5. Can any legacy route or binding state observe, retain, or reinterpret the new Servient activation authority or route permit?
6. Are compatibility adapters structurally prevented from gaining new public callers?
7. Are the adapters one-way in code and dependency direction, or only described as one-way in documentation?
8. Can old and new execution paths operate on the same binding generation, plan generation, route generation, or request identity?
9. What compile, runtime, or source evidence proves that one generation has only one selection, dispatch, and activation authority during each migration stage?
10. Are removal owners and removal checkpoints complete for every legacy API and call site that remains reachable?
11. Could a legacy path remain reachable after its replacement package reports completion without violating an existing checker?
12. Does the current WP-300 Property Read slice require any adapter whose ownership or removal condition is still ambiguous?
13. What evidence would falsify the concern that a migration boundary can leak legacy behavior into a target-generation request?
14. If leakage is possible, which exact adapter, call edge, or authority record permits it?

## Constraints

- Do not treat the presence of legacy code as evidence that it remains normative.
- Do not claim a current double-authority defect without tracing one exact generation and call path.
- Do not require immediate removal of legacy code merely because its target replacement is designed.
- Do not introduce a second public compatibility contract under this topic.
- Preserve the staged migration and AI-led decision model while Codex determines whether the current boundaries are sufficient.

## Decision

No current target-generation request can leak into a legacy authority because
the WP-300 target execution path does not yet exist. The concern nevertheless
found a real enforcement gap: the migration records did not explicitly forbid
new Core/WP-300 code from calling the still-public
`clinkz-wot-protocol-bindings` form selectors, and the final removal list did
not name that selector family.

The boundary is source- and generation-segregated:

- WP-200 Planning selects the form, resolves the target, and produces the
  artifact exactly once;
- a target WP-300 registration consumes only that plan/artifact identity and
  must not depend on or call a legacy TD/form selector;
- a target request is accepted only under its target binding, plan, route, and
  serving generations and is never passed to legacy `serve`, `Dispatch`, or
  binding-owned handler lookup;
- the existing Zenoh selector call remains a legacy-generation path until
  WP-600 replaces it with compiled input; and
- WP-700 proves final absence of the legacy selector exports and all other
  named compatibility surfaces.

This is not a per-request adapter from the new artifact back into the legacy
selector. The only positive compatibility adapter retained by WP-300 is the
already named one-way legacy publication-to-`ProducerEmission` boundary; its
handler-side and protocol-side ends remain removed by WP-400 and WP-600.

## Migration

The no-backflow rule and selector-removal owner are projected into
`docs/architecture/40-protocol-binding-spi-and-deployment.md`,
`docs/spec/binding-spi.md`, WP-300/WP-600/WP-700 work-package records, D20 in
`PLAN.md`, and `PROJECT_STATE.md`. The WP-300 candidate must add source and
dependency evidence for the negative edge.
