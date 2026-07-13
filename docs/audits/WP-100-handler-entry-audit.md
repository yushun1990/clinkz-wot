# WP-100 Handler Tranche Entry Audit

Status: Blocks handler implementation

Audit date: 2026-07-13

Design revision reviewed: v4.6 with amendments through
`WP-100-OUTPUT-API-001`

Affected gates: GATE-1, GATE-2, GATE-4, GATE-5, GATE-6

## Scope

This audit checks whether the next WP-100 tranche can replace the current
operation handlers, cancellation behavior, and sparse handler storage directly
from the active design. It reviews only active artifacts selected by `PLAN.md`.

## Verdict

The handler tranche is not implementation-ready. Implementing it from the
current documents would require local choices about public trait signatures,
subscription acceptance, cancellation ownership, late synchronous results,
resource admission, and migration staging. Those choices would cross crate and
work-package boundaries and could recreate the implementation divergence that
the refactor gates are intended to prevent.

No handler implementation migration may start until a normative amendment and
the affected artifacts close every item below.

## Blocking Findings

### H-1: Public ownership does not enumerate the handler surface

`HANDLER-API-001` says every operation has a synchronous handler trait, an async
twin, and an optional bounded poll/step form. `docs/api-ownership.csv` contains
rows only for `HandlerContext`, `CancellationView`, and `Deadline`. It contains
no rows for:

- the eighteen operation-specific synchronous traits;
- their async and bounded-step counterparts;
- typed handler input, output, acceptance, or step-state values;
- host-erased or constrained registration values;
- setter, replacement, removal, and clear operations; or
- internal sparse slot records and their public registration boundary.

The ownership matrix therefore cannot decide exact crate paths, feature cells,
or old-API removal for the next tranche.

### H-2: Normative prose leaves observable ABI alternatives open

The active design permits a `HandlerContext` to be owned or borrowed, an
observe/subscribe handler to return a guard or acceptance value, and write,
cancel, subscribe, or unsubscribe operations to return either a status or an
output. It does not freeze fields, constructors, getters, lifetimes, derives,
object-safety, future ownership, poll signatures, or per-operation input/result
types.

`API-PAYLOAD-001` also says `InteractionInput` owns the principal, resolved URI
variables, deadline/cancellation view, and correlation metadata, while
`HANDLER-API-001` assigns the same application-visible facts to
`HandlerContext`. The documents do not define one source of truth or the trust
boundary for constructing those values.

### H-3: Cancellation cannot retain a late synchronous handler safely

`HANDLER-CANCEL-001` correctly says a synchronous handler is non-preemptible and
its late result is discarded. The active in-flight machine can nevertheless
transition from `Admitted` through `Cancelled` to `Released` without a state or
owner that pins the selected handler, request data, plan generation, and
diagnostic identity until the synchronous call actually returns.

The async policy is also open: the design does not choose whether cancellation
signals and awaits the handler future, drops the future at a defined point, or
detaches it under retained ownership. `CancellationView` has no frozen live or
snapshot semantics, wake behavior, or ownership rules. No bounded-step handler
contract exists to prove `HANDLER-CANCEL-002`.

### H-4: Producer subscription setup has no state machine

The `subscription` machine in `docs/state-machines.toml` is owned by
`ConsumedThingHandle` and models the Consumer-side binding guard. It does not
model the Producer observe/subscribe transaction required by `HANDLER-SUB-001`:
application acceptance, local guard reservation, binding guard installation,
publication, rollback, and exactly-once application teardown.

The active model also does not close late setup success after cancellation,
drop during setup or teardown, setup failure after an external guard exists,
or retained terminal application teardown outcomes.

### H-5: Push removal and emission replacement form a DAG cycle

WP-100 currently requires removal of `PushFn`, `PublisherSink`, and other direct
push facades before WP-200 may start. WP-300, which depends on WP-200, owns
`ProducerEmission`, `BindingPublication`, `EmissionStatus`,
`ServerEmissionSlot`, and removal of direct binding publication paths.

Removing the old facades in WP-100 would leave current Servient and protocol
packages without their WP-300 replacement. Retaining them would violate the
current WP-100 completion text. The migration owner and temporary compatibility
boundary must be made acyclic before either removal is attempted.

### H-6: Handler storage and replacement are not resource-admitted

`HANDLER-STORAGE-001` requires storage proportional to admitted operations, but
the resource schema has no handler slot, handler object, async-wrapper, pending
handler call, or post-exposure replacement byte/count ceilings. The generic
compiled-runtime byte ceilings do not define which account owns dynamic handler
replacement or how rollback works when replacement exceeds it.

The design also does not give the destroy drain policy an exact cumulative
deadline or manual-step field for retaining late handler ownership.

### H-7: Evidence and performance identities do not prove the contract

The WP-100 evidence keys name handler cancellation, callback isolation, and
sparse storage, but no executable surface matrix or workload identity varies
registered operation density, sync/async/step storage, replacement, late
return, or reentrant callback behavior. Existing subscription and fan-out
workloads do not substitute for Producer handler setup/teardown evidence.

The exact evidence keys must be tied to the new state transitions, feature
cells, resource boundaries, and workload cases before implementation starts.

### H-8: The frozen poll skeleton contains an invalid signature

The normative `PollClientBinding::poll_subscription` skeleton contains two
consecutive `&mut self` parameters. The checker does not currently reject this
invalid Rust signature. The amendment and executable checker must repair and
lock the exact signature before a later binding tranche consumes it.

### H-9: `AffordanceTarget` keep semantics conflict with constrained ownership

The ownership matrix marks `AffordanceTarget` as `keep`, while its current
representation stores names in `Arc<str>`. The constrained contract says the
no-default surface must not require `Arc` or pointer-width atomics, and the
current no-default check compiles only for the host target. The design must say
whether `keep` freezes only the name and public path or also freezes this
representation, then add a build/evidence cell that can detect an accidental
atomic requirement.

## Required Closure Artifacts

The next normative revision must provide all of the following:

1. An exact operation matrix for every synchronous, async, and bounded-step
   handler trait, including paths, feature cells, inputs, results, and setter,
   replacement, removal, and clear operations.
2. Exact schemas and method surfaces for `HandlerContext`, `CancellationView`,
   `Deadline`, subscription acceptance, and any handler step state.
3. One ownership rule for application payload, principal, URI variables,
   correlation, deadline, cancellation, plan, binding, action invocation, and
   subscription identities across `InteractionInput` and `HandlerContext`.
4. Exact synchronous late-result retention and async/step cancellation
   transitions, owners, terminal outcomes, and generation-reuse rules.
5. A Producer observe/subscribe state machine covering setup publication,
   rollback, cancellation, drop, teardown, cleanup transfer, and late callback
   behavior.
6. An acyclic migration plan assigning `PushFn`, `PublisherSink`,
   `SubscriptionSender`, and `ProducerEmission` replacement/removal to explicit
   work-package checkpoints.
7. Exhaustive handler storage, pending-call, replacement, and drain limits with
   all named profile values and boundary evidence.
8. Executable API, state, resource, workload, and evidence checks, including a
   valid frozen `poll_subscription` signature.
9. An exact no-atomic representation rule for `AffordanceTarget` and a
   constrained compile check that proves it.

Only after these artifacts pass independent review may the affected gates
return to `closed` and handler implementation resume.
