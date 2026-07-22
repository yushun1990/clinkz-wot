# 0006 Handler Entry Decomposition

Status: MIGRATED

## Problem

The completed `WP-100-FOUNDATION-REFRESH` removes the first prerequisite from
the broad `WP-100-HANDLER-ENTRY`, but it does not make the entire handler
migration implementation-ready. Treating that entrypoint as one implementation
unit would combine portable value contracts, 54 handler traits, host erasure,
sparse replacement, cancellation reducers, Producer subscription integration,
Servient setters, and downstream binding work.

The v4.9 re-review also found that a passing syntactic checker was hiding two
different issues:

- the handler amendment repeated an obsolete `poll_subscription` signature
  while the binding domain owns `poll_subscription_start`; and
- the promised no-atomic `AffordanceTarget` evidence cannot be claimed while
  the real Core implementation still contains the legacy `Arc<str>` shape and
  has no incapable-target build.

The four Core handler performance workloads also list matrix dimensions without
an executable complete-coverage oracle. They cannot yet support admission of
the full storage and cancellation implementation.

## Decision

Keep `WP-100-HANDLER-ENTRY` blocked. Its global readiness command remains the
broad coordinated entry check; ADR-0013 scoped work proceeds through explicit
child tranche records instead of weakening that command.

The next candidate is `WP-100-HANDLER-VALUE-PRIMITIVES`. It is purely additive
and contains exactly:

- `CancellationView`;
- `SubscriptionAcceptance`;
- `HandlerFootprint`;
- `HandlerStep<R>`; and
- `StaticHandlerRegistration<'h, H>`.

It changes only the Core handler module and root re-exports. Its external nested
contract fixture and source validator are frozen before implementation. It
excludes `Deadline`, `AcceptHint`, `InteractionInput`, `HandlerContext`,
`AffordanceTarget`, all 54 handler traits, host erasure/ingress, sparse storage,
execution owners, Producer state, Servient APIs, binding APIs, old API removal,
and performance workloads.

The five values have no queue, scan, allocation policy, callback, state
transition, or runtime scheduling algorithm. Their completion evidence is a
three-cell public-surface build plus exact value/ownership tests, not a
performance measurement.

The independent entry review discovered that `Deadline` cannot compare a raw
finite-width clock across an arbitrary manual-poll gap. Review 06 and
`workspace/0007-time-domain-and-deadline.md` therefore record it, the
foundation clock domain, cleanup timing, and impacted WP-000 time evidence in
the separate `TIME-DOMAIN-AND-DEADLINE` blocking scope. That scope is an impact
placeholder, not a defined corrective tranche or a dependency of the five-value
candidate. The identity, ownership, dependencies, completion contract, and
evidence disposition of future corrective work remain open. This five-value
decision is disjoint from that topic; the broad handler entry remains
separately blocked by it.

## Later decomposition

The remaining handler work is intentionally not admitted by this decision:

1. the time-domain correction and `Deadline`;
2. request/target/context values and the real no-atomic boundary;
3. the 54 portable handler traits and real external compile matrix;
4. host opaque registration, sparse storage, replacement, and execution owners;
5. executable storage/cancellation coverage matrices;
6. WP-300 binding and Producer primitives;
7. WP-400 Servient setters and Producer coordination; and
8. downstream legacy push-surface removals at their frozen checkpoints.

## Migration

The conclusion is projected into `docs/reviews/review-05.org`,
`docs/requirements.csv`, `docs/api-ownership.csv`,
`docs/work-packages/index.toml`, `docs/work-packages/WP-100-core.md`,
`docs/audits/WP-100-handler-value-primitives-entry.md`, the governance checker
registry, and `PLAN.md`.
