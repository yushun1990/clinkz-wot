# 0035 Cleanup Protocol Implementability

Status: OPEN

Kind: owner-raised lifecycle API and implementation-risk investigation

Priority: HIGH

Target: cleanup reservation, phase binding, complete-object transfer, manual fallback, durable residual state, and terminal acknowledgement

## Scope and authority

This topic records a Project Owner concern that the cleanup ownership model is rigorous but may impose excessive implementation and API complexity on Servient and Binding authors. It does not authorize destructor-only cleanup, record-only transfer, or loss of complete owned work. Codex owns the technical decision.

## Repository observations

- Cleanup capacity and identity are reserved before side effects.
- Each cancellation, abort, shutdown, or transfer phase fixes its own operation, first cause, deadline, generations, work, and footprint.
- `Pending` is legal only after the complete work object reaches an acknowledged owner.
- Rejected handoff must return the identical complete object to a pre-reserved manual owner.
- Durable residual state is not successful cleanup, and dropping a live object is not the cleanup protocol.

## Questions for investigation

1. What is the smallest reusable Rust abstraction that preserves these invariants across calls, routes, readiness, responses, subscriptions, and emissions?
2. Which cleanup distinctions must be public to Binding authors and which can remain engine-owned helpers?
3. Can the host and constrained forms share one semantic transition implementation?
4. How are executor rejection, lost wake prevention, deadline progress, and late results tested deterministically?
5. Does every simple synchronous Binding need to implement the full transfer surface, or can bounded no-successor helpers prove a smaller path?
6. Where can generic nesting or successor enums become unusable or produce excessive code size?
7. What concrete implementation evidence would justify revising the public cleanup surface?

## Constraints

- Preserve complete-object ownership until acknowledged transfer.
- Preserve first-cause, deadline, resource, and residual-state truth.
- Do not treat `CleanupRecord` as progress-capable work.
- Do not weaken cleanup solely to improve syntax; require concrete implementability evidence.

## Expected decision output

Codex should define the reusable implementation pattern, public-versus-private cleanup surface, mandatory negative/runtime tests, simple-binding helpers, and any API corrections revealed by the first WP-300 implementation.