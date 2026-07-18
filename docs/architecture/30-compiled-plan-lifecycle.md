# Compiled-Plan Lifecycle

## Decision

The engine has an explicit compiled-plan-set lifecycle. Individual plan values
remain immutable; lifecycle state belongs to the Servient record that owns the
set, not to each value.

## Plan taxonomy

- A logical interaction plan contains resolved target context, operation,
  source form identity, media/schema rules, URI-variable rules, effective
  security, response classification, and diagnostic source identity.
- A binding candidate pairs a logical plan with one binding id/generation and
  side-effect-free capability result.
- A binding artifact contains bounded protocol-specific compiled data produced
  by that binding's compiler extension.
- A binding plan reference joins a logical plan, binding identity, and artifact
  slot without exposing protocol-local representation to other bindings.
- A compiled plan set is the immutable admitted collection owned by one
  produced or consumed handle generation.

## Plan-set lifecycle

```text
Building -> Frozen -> Published -> Draining -> Reclaimed
    |          |
    +-> Failed +-> Failed
```

- `Building` owns the document/policy/registration snapshot, admission
  transaction, compiler cursors, and provisional artifacts.
- `Frozen` means every mandatory plan, identity, footprint, and route owner is
  immutable and all required capacity is reserved. No binding side effect has
  started for a Consumer. A Producer may enter route preparation only from this
  state.
- `Published` means a handle generation can select and pin plans. Publication
  is one atomic Servient registry transition.
- `Draining` rejects new selection while existing operation, route,
  subscription, and cleanup leases finish.
- `Reclaimed` is reachable only when all leases, routes, lazy artifacts, and
  cleanup owners for that generation are terminal.
- `Failed` is an admission terminal before publication. Any provisional
  binding artifact is released outside engine locks.

Pin count and active-operation count are orthogonal retained fields, not extra
public states. A published plan value is never modified in place.

## Producer and Consumer timing

- Consumer plans are built, frozen, and published by the `consume` transaction.
  Transport execution starts only from the published set.
- Producer plans are built and frozen during `expose` before the first route
  side effect. Route preparation, readiness, activation, and commit bind that
  frozen plan set to active route generations. The set becomes published only
  with the serving registry transition.

The route lifecycle and plan-set lifecycle are orthogonal. A failed route
transaction rolls back resources and leaves the unpublished plan set failed;
it does not mutate individual plans.

## Binding compiler extension

Every complete binding registration contains one compiler extension tied to the
same binding id, generation, capability declaration, configuration digest, and
execution registrations.

The compiler extension:

- is deterministic for a fixed input snapshot;
- is local and side-effect free;
- consumes an explicit work and memory budget;
- returns a bounded artifact plus its immutable lifetime footprint;
- never receives credentials or application handler objects; and
- never selects a different form, operation, security branch, or binding.

Server route and publication artifacts required for exposure are eager.
Consumer artifacts may be lazy only when construction is pure, bounded, and
does not start protocol work.

## Lazy artifact lifecycle

```text
Empty -> Compiling -> Ready
                    -> Negative
Ready | Negative -> Reclaiming -> Empty
```

One generation has at most one compiler lease for a slot. Waiters observe the
same immutable result or bounded backpressure. `Negative` stores a bounded,
redacted deterministic failure and cannot hide a retryable transport failure.
No lazy state is shared across incompatible document, policy, binding, or
configuration generations.

Budget exhaustion retains a cursor and resumes; it does not restart compilation
or publish a partial artifact. Callbacks execute outside locks.

## Registration and generation rules

The v1 registration set is startup-only:

- `ServientBuilder` validates a complete set and assigns/captures binding
  generations before `build` returns.
- A produced or consumed handle pins that immutable registration snapshot.
- The Servient exposes no runtime add/remove/replace API in v1.
- Configuration or code rollout creates a new Servient/process generation.
- The old instance drains its handles, routes, calls, subscriptions, and cleanup
  before exit.

Therefore a binding-generation change never silently invalidates an existing
handle or mutates its cache. A future runtime replacement design must define
explicit rebuild, cutover, lease, and cleanup semantics before adding such an
API.

## Hot-path contract

After publication, an interaction may inspect only the bounded candidate list
for its target and operation. It does not:

- traverse the TD tree;
- resolve `base` or URI templates from source text again;
- probe unindexed bindings;
- redo W3C defaults or security inheritance;
- compile an unbounded artifact; or
- mutate a shared plan.

Protocol-native connection/session caches may exist in a binding, but they are
runtime resources keyed by admitted plan and binding generations, not a second
undocumented planning system.

## Resource and evidence requirements

Admission accounts for logical bytes, binding-artifact lifetime bytes, lazy
slot metadata, negative results, candidate counts, and compilation work.
Reclamation is incremental under `WorkBudget` for constrained profiles.

Evidence must cover cold maximum-plan compilation, repeated hot selection,
single-flight lazy compilation, deterministic negative caching, cancellation at
each build phase, maximum-generation drain, and zero retained plan/artifact
bytes after reclamation.
