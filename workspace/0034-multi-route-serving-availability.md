# 0034 Multi-Route Serving Availability

Status: MIGRATED

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

## Decision

In v1 every route represented by an advertised Producer form in the frozen
effective TD and immutable plan set is required for that serving generation.
“Optional”, “redundant”, and “alternative” are not runtime route labels. A
failed HTTP, Zenoh, or MQTT route therefore prevents publication of a
generation that advertises all three; silently publishing the other two would
make the served TD and plan authority false.

Deliberate optionality is expressed before planning by constructing a
different effective TD/application configuration that omits the capability.
After a readiness failure, the application may build and expose a new
generation without that form, but the failed generation is rolled back first.
An absent route cannot join a published generation later. A configuration-only
change may reuse the binary, but it still creates a new Servient/application
generation; startup-only binding composition remains unchanged.

Status exposes the failed route/binding/generation, readiness phase, primary
failure, and cleanup disposition. It does not report a partially serving
generation. A future availability-group model would require an immutable
versioned plan/TD policy, bounded variant selection, honest TD publication,
resource accounting, regeneration behavior, and new failure/workload
evidence. It is deferred rather than inferred from multiple forms.

## Migration

The exact meaning of “required route” and the rejected partial-publication
model are projected into ADR-0012, the Planning and Binding specifications,
and the WP-400 exposure contract. This topic is `MIGRATED`.
