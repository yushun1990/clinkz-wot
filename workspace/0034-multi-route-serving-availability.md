# 0034 Multi-Route Serving Availability

Status: OPEN

Kind: owner-raised lifecycle and availability-policy investigation

Priority: HIGH

Target: all-required-route commit and atomic produced-Thing serving publication under partial protocol availability

## Scope and authority

This topic records a Project Owner concern that the all-required-route publication rule prevents partial activation correctly but may make one unavailable protocol route block an otherwise usable Thing generation. It does not assert that partial publication is acceptable. Codex owns the lifecycle and product-semantics decision.

## Repository observations

- A produced Thing becomes serving only after every required route reaches `CommittedClosed`.
- Publication, plan selection, registry generation, and permit availability are one Servient transition.
- Sequential post-publication route opening is rejected because it exposes partial activation.
- The first Property Read gate uses one route and cannot exercise degraded multi-protocol availability.
- Preparation visibility and closed-ingress policy are declared per route.

## Questions for investigation

1. How are required, optional, redundant, and alternative routes represented in the compiled Producer plan set?
2. Is every form-backed Producer route necessarily required for one serving generation?
3. Can optional or alternative routes join later without violating immutable publication and generation authority?
4. What should happen when HTTP and Zenoh are ready but an MQTT route is unavailable?
5. Does a failed optional route prevent publication, remain absent for the generation, or require a new generation?
6. Which availability semantics are visible through status and application APIs?
7. What multi-route fixtures and failure cases must validate the decision?

## Constraints

- Do not allow a binding callback to mutate serving authority after publication.
- Do not permit accepted traffic on an uncommitted or stale route.
- Preserve one explicit generation and rollback boundary.
- Distinguish deliberate optionality from silent partial failure.

## Expected decision output

Codex should determine the required/optional/alternative route model, its planning and Servient ownership, publication and regeneration behavior under partial availability, and the exact state, status, resource, and workload evidence required.