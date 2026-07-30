# 0031 End-to-End Architecture Feedback Latency

Status: MIGRATED

Kind: owner-raised execution and architecture-validation investigation

Priority: HIGH

Target: the first executable Property Read vertical proof and the delay between public-contract freeze and cross-package runtime feedback

## Scope and authority

This topic records a Project Owner concern that the target architecture may freeze substantial Core, Planning, Binding, and Servient contracts before one complete target-generation Property Read has executed. It does not assert that the ordered tranche DAG is wrong or authorize bypassing admitted ownership boundaries. Codex owns the repository-grounded decision.

## Repository observations

- WP-100 and the narrow WP-200 Property Read slices are implemented.
- The WP-300 candidate is reviewed, but its product source and WP-400 orchestration are not yet implemented.
- `PROPERTY-READ-ARCHITECTURE` remains the first cross-package executable proof.
- That gate intentionally excludes subscriptions, production protocols, security execution, fallback, cancellation races, multi-route behavior, and performance workloads.
- Narrow WP-300 completion releases only narrow WP-400; broad Binding risks remain later work.

## Questions for investigation

1. Which public contracts remain unvalidated by any target-generation runtime composition?
2. Can the WP-300 and WP-400 path reach executable feedback without another support-only refinement cycle?
3. What conclusions may legitimately be claimed when the narrow Property Read gate passes?
4. Which broad risks must remain explicitly open after that gate passes?
5. Should a minimal production-protocol smoke proof follow immediately, and which package owns it?
6. Are any current milestone or progress statements likely to overstate executable maturity?
7. What observable events should distinguish local slice completion, vertical composition, production-binding validation, and release readiness?

## Constraints

- Do not weaken the ordered ownership chain or replace production boundaries with fixture-owned substitutes.
- Do not treat mock runtime success as production-protocol or broad Binding completion.
- Do not add a new gate unless it protects a distinct falsifiable claim.
- Avoid further pre-source design work unless a new intersecting semantic, ownership, lifecycle, resource, dependency, or evidence-truth defect is identified.

## Expected decision output

Codex should determine the shortest safe path to target-generation runtime feedback, the exact claim boundary of the first Property Read gate, any required follow-on production smoke evidence, and the authoritative progress records that need correction.

## Decision

The shortest safe path remains the existing direct chain:

`WP-300 Property Read -> WP-400 Property Read -> PROPERTY-READ-ARCHITECTURE`.

No additional support-only refinement or protocol spike may enter that chain
without a concrete intersecting ownership, lifecycle, resource, dependency, or
evidence-truth defect. The first gate proves one target-generation mock route
from planning through Servient dispatch, response, and cleanup in the host and
manual runtime cells, plus the executor-neutral async/no-std compile
projection. It proves cross-package constructibility and ownership flow only.
It does not prove multi-route availability, broad fairness, production
transport behavior, production-author ergonomics, security/fallback/retry,
subscriptions, cancellation races, workloads, or release readiness.

Progress must use four distinct observable events:

1. package-local slice completion;
2. the passed mock cross-package Property Read gate;
3. a real Zenoh Property Read authoring/runtime smoke owned by WP-600; and
4. final release readiness after the full branch join and global gates.

The Zenoh smoke is the first executable WP-600 tranche after broad WP-300
releases that package. Non-authoritative preparation may begin earlier but
cannot block or take progress credit from the narrow WP-300/WP-400 chain.

## Migration

The claim ladder and immediate critical path are projected into `PLAN.md`,
`docs/work-packages/PROPERTY-READ-ARCHITECTURE.md`, and
`docs/work-packages/WP-600-protocol-bindings.md`. This topic is `MIGRATED`.
