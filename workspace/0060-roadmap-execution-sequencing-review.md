# 0060 Roadmap Execution Sequencing Review

Status: DECIDED
Kind: roadmap and implementation-sequencing investigation
Baseline: `594c1b36f1d529f2e57ee30ddbfd601f0682e52e`

## Scope and authority

This topic decides how the v5 implementation roadmap should sequence executable
feedback after the passed Producer Property Read architecture gate.

It is not itself architecture authority. It does not amend `PLAN.md`, the
work-package DAG, active requirements, gate manifests, or source code. The
conclusions below must be migrated to their authoritative owners before they can
authorize implementation.

The decision keeps the macro ownership architecture. The defect is in execution
sequencing: one Producer one-shot composition proof is not sufficient reason to
expand all remaining work packages horizontally before the next materially
different ownership topologies receive executable feedback.

## Repository evidence controlling the decision

The following facts control the result.

- The active v5 authority contains 62 active requirements and 34
  `inactive-domain-entry-review-required` identities. Inactive identities must
  be reconsidered at domain entry and cannot authorize implementation merely
  because a work package names them.
- ADR-0013 makes the tranche the smallest implementation-admission unit. Parent
  work-package status alone neither admits nor rejects a source change; exact
  predecessor tranches and implementation dependencies must be complete.
- `PROPERTY-READ-ARCHITECTURE` proves a Producer Property Read path across real
  Planning output, complete binding registration, Producer route lifecycle,
  Servient publication, handler execution, response sealing/delivery, and
  cleanup. It deliberately does not prove Consumer invocation, subscriptions,
  emissions, Directory execution, or broad multi-route behavior.
- The current Consumer implementation remains the legacy v4.1 topology:
  `BindingRequest` carries `Thing` and `Form`; `ClientBinding` performs
  `supports_with_thing`; `ConsumedThing` scans bindings, applies security from
  the TD at runtime, and invokes the selected legacy binding. This differs
  materially from the v5 target in which Planning selects immutable execution
  facts and an owned `OutboundRequest` enters the chosen client binding.
- The current long-lived path remains legacy: Core-owned subscription queues and
  guards plus `EventStream`/per-affordance fan-out are still present, while the
  target architecture assigns subscription progress and protocol-local flow
  control to a binding-owned driver/slot and application lifecycle ownership to
  Servient.
- Broad WP-400 already requires an early multi-Thing/multi-binding/multi-route
  scheduling checkpoint before subscription, emission, and broad facade work
  accumulate. That evidence owner already exists and does not need another
  global gate.
- Active module authority places Discovery on
  `foundation + td + core <- discovery`; Discovery does not depend on Planning,
  concrete bindings, or Servient. WP-500 itself is client-only and forbids a
  Servient or concrete-transport dependency.
- WP-600 consumes WP-200/WP-300 shared planning and binding contracts and must
  implement real concrete protocol behavior. Earlier real Zenoh feedback has
  already demonstrated that concrete authoring can expose shared-SPI defects
  which package-local fixtures miss.
- The active architecture already requires staged legacy separation: target
  artifacts must not flow back through legacy selection; concrete legacy edges
  disappear as their owners migrate; WP-700 proves final absence.

## Decision table

| Question | Verdict | Decision |
| --- | --- | --- |
| Q1 Consumer Property Read gate | ACCEPT | Add one formal Consumer one-shot cross-package architecture gate before broad Consumer expansion. |
| Q2 Long-lived interaction gate | ACCEPT | Add one formal long-lived gate, represented by a minimal `ObserveProperty` closed loop. |
| Q3 WP-400 early checkpoint | ACCEPT | Run the existing checkpoint in parallel with Consumer domain-entry work; keep it WP-400 evidence, not a new gate. |
| Q4 WP-500 dependency | MODIFY | Remove the coarse `WP-500 -> WP-300` dependency; WP-500 depends on WP-100 at package level and exact admitted Core/TD prerequisites at tranche entry. |
| Q5 WP-600 evidence timing | MODIFY | Move real protocol evidence to each stable capability boundary; Host Zenoh enters first, while real zenoh-pico evidence is mandatory before the corresponding constrained capability/WP-600 completion claim. |
| Q6 Legacy removal | ACCEPT | Remove migrated target-to-legacy backflow capability-by-capability; WP-700 proves final public/source absence. |

## Q1 — Consumer Property Read architecture gate

### Decision

Create one formal Consumer Property Read cross-package architecture gate before
broad Consumer Planning, Binding, and Servient expansion.

This is not a symmetry exercise for its own sake. Consumer one-shot execution
introduces ownership transitions not exercised by the passed Producer gate:

```text
TD
 -> consumed plan build/publication
 -> selected immutable Consumer plan
 -> owned OutboundRequest
 -> selected ClientBinding call/slot
 -> protocol result
 -> shared response validation
 -> InteractionOutput
 -> caller / late completion / cleanup
```

The gate must prove at least:

- no runtime TD/Form rescan or `supports_with_thing` selection on the target
  path;
- one selected binding/plan generation and one admitted call owner;
- `OutboundRequest` is sufficient for concrete execution without re-selection;
- caller drop, cancellation, timeout, late result, and terminal settlement keep
  ownership explicit;
- response validation has one semantic owner and cannot be bypassed by a
  binding;
- host-erased and constrained/static representations preserve the same semantic
  outcomes; and
- terminal completion leaves no retained call, cleanup, or plan lease.

The first gate deliberately excludes fallback, lazy compilation, subscriptions,
collection operations, broad multi-binding fairness, write/action breadth, and
retry policy beyond what the one selected call needs.

### Authority consequence

Before implementation, the Consumer one-shot domain must review the exact
inactive identities it needs. The expected entry set includes at least
`PLAN-REQUEST-001`, `BIND-OUT-001`, and `API-OPTIONS-001`, plus any response,
cancellation, status, or validation identity actually required by the admitted
slice. The entry review may re-adopt, replace, split, or retire them; this topic
does not reactivate them.

Only after that authority is migrated may the new gate manifest and narrow
implementation tranches be admitted.

## Q2 — Long-lived interaction architecture gate

### Decision

Create exactly one additional formal cross-package architecture proof for the
long-lived topology before broad subscription/emission expansion.

Use `ObserveProperty` as the representative scenario.

`ObserveProperty` is preferred over `SubscribeEvent` because it changes the
fewest semantic axes after Property Read: the same property affordance family
is retained while the test adds the new facts that actually need proof — a
long-lived client driver, Producer observation/emission, repeated delivery,
stop/drop/cancel, backpressure, drain, and cleanup transfer.

The minimum proof is intentionally small:

- one Thing;
- one property;
- one binding;
- one observer;
- one binding-owned subscription driver/static slot;
- at least two delivered samples so the proof cannot collapse into a one-shot
  call;
- one explicit unobserve/stop path; and
- terminal cleanup with zero retained ownership.

It must cover start-pending cancellation, driver installation, bounded
backpressure/overflow behavior, receive-versus-stop ordering, remote terminal,
Servient drain, Producer emission-to-binding handoff, and cleanup ownership.

It explicitly excludes all-properties observation, event collections,
multi-subscriber fan-out, multi-target emission, broad fallback, general
streaming APIs, and arbitrary-scale scheduling.

Later `SubscribeEvent`, collection subscriptions, and related operation families
reuse this lifecycle topology and receive package/integration evidence unless
new implementation evidence proves that they introduce another materially
different ownership boundary.

### Authority consequence

Subscription/emission inactive identities are reviewed as one coherent domain
entry immediately before this gate. The likely inputs include
`HANDLER-SUB-001`, `BIND-PROGRESS-001`, `SUB-STORAGE-001`, `SUB-DATA-001`,
`STATE-SUB-001`, and `PRODUCER-EMIT-001`; exact disposition remains a separate
authority decision.

## Q3 — WP-400 early multi-owner checkpoint

### Decision

Run the already-specified WP-400 multi-owner/scheduler checkpoint in parallel
with Consumer domain-entry preparation.

It remains WP-400 evidence and is not promoted to a third architecture gate.
The early checkpoint may use the already-proven Producer route path to exercise
at least two Things, two bindings, multiple routes, a hot owner, a never-ready
owner, cleanup/deadline pressure, and unrelated progress.

Its purpose is to falsify scheduler decomposition, route-owner isolation,
sharding, work budgeting, and cleanup progress early. It must not claim to
freeze Consumer call ownership, subscription/emission scheduling, or broad
public facade structure. Those remain free to refine when Consumer and
long-lived evidence arrive.

Private scheduler/container/helper representation remains implementation
feedback, not a public contract.

## Q4 — WP-500 package dependency

### Decision

The current coarse `WP-500 -> WP-300` dependency is not justified by the active
module boundary and should be corrected during roadmap migration.

The package-level dependency should become:

```text
WP-100 -> WP-500
```

This does **not** mean that broad WP-100 completion automatically admits all
WP-500 source work, nor that Discovery may start against unfinished Core
contracts. ADR-0013 still controls implementation entry: every WP-500 tranche
must name exact dependency-complete Core/TD predecessors and pass its own
independent admission.

The reason for removing WP-300 is technical rather than merely Cargo-level:

- Discovery owns a protocol-neutral Directory client state machine;
- it may consume Foundation/TD/Core values;
- it does not select WoT forms through Planning;
- it does not execute Protocol Binding client/server SPI;
- it must not depend on Servient or a concrete transport; and
- its cancellation/status/resource requirements are Core contracts and should
  be admitted at their actual Core owner, not indirectly through broad WP-300
  completion.

A later Servient/umbrella integration test is still required, but that is an
integration dependency, not justification for making the entire Binding
package a predecessor of the Directory client package.

The old WP-500 prose saying work may begin only after WP-300 must therefore be
removed or rewritten when this decision migrates.

## Q5 — WP-600 production evidence timing

### Decision

Do not wait for broad WP-300 completion before allowing every concrete-protocol
attempt. Real protocol pressure should follow each stable matching capability
boundary.

The progression is:

```text
matching WP-200/WP-300 tranche
 -> matching WP-400 runtime tranche where required
 -> applicable architecture proof
 -> real Host Zenoh production path
 -> constrained zenoh-pico production/parity evidence before the corresponding
    constrained capability or WP-600 completion claim
```

This does not allow Zenoh to bypass or own shared semantics. A concrete binding
receives only the already-selected shared contract; if real authoring exposes an
unconstructible or semantically wrong SPI, the shared owner is reopened and
reviewed.

The existing distinction between feedback and product evidence remains useful:

- a bounded probe against incomplete/non-production target surfaces is
  architecture feedback and grants no WP-600 completion credit;
- a real protocol path using admitted public target contracts with no
  test-only/legacy selection or dispatch edge is WP-600 production progress;
- Host Zenoh should enter that production evidence as soon as the matching
  capability boundary is stable rather than waiting for unrelated broad
  WP-300 features; and
- zenoh-pico compile-only feature coverage is not constrained-runtime evidence.
  Real associated-state/backend/platform evidence is required before claiming
  constrained parity or completing the relevant common-capability slice.

Host and constrained evidence therefore need not be artificially synchronized
at the first feedback moment, but WP-600 cannot close a claimed common
capability intersection using Host-only runtime evidence.

The first production slice should be reconsidered accordingly: Consumer
Property Read is higher-value new protocol pressure than repeating the already
well-exercised Producer-only path. Producer Property Read remains required
regression evidence and participates in the later bidirectional closed loop.

## Q6 — Legacy-removal timing

### Decision

Remove target-to-legacy backflow edges as each capability becomes production
complete. Do not defer routine migration cleanup to WP-700.

The removal rule is staged:

1. when a target capability has a production path, remove any source edge that
   converts its plan/artifact/request back into legacy TD/Form selection or
   legacy execution;
2. keep a temporary compatibility type or public export only while a legitimate
   unmigrated downstream owner still requires it;
3. remove that compatibility surface immediately after its final legitimate
   caller migrates; and
4. let WP-700 prove final public exports, source call edges, umbrella re-exports,
   and named compatibility families are absent.

This is consistent with the active staged-legacy-separation architecture. It
reduces the risk that a new target path accidentally reuses `BindingRequest`,
legacy selectors, Core queues, `SubscriptionGuard`, `EventStream`,
`PublisherSink`, or equivalent compatibility machinery merely because those
symbols remain convenient.

## Gate-count decision

Exactly two additional formal cross-package architecture gates are justified:

1. Consumer Property Read one-shot; and
2. one `ObserveProperty` long-lived interaction proof.

The passed Producer Property Read gate remains the first baseline proof.

No third global gate is justified now.

The following areas remain executable evidence rather than new architecture
gates unless implementation produces a counterexample:

- WP-400 multi-owner/multi-route scheduling;
- native collection planning/driver behavior;
- multi-target emission and slow-lane isolation;
- Directory client state machines plus one Servient/umbrella integration path;
- additional one-shot write/action families; and
- Zenoh/zenoh-pico production and parity evidence.

Directory has a materially different client process model, but its engine-side
boundary is intentionally isolated and can be falsified with package-local
state-machine evidence plus one composition path. Collection and multi-target
emission add fan-out/aggregation pressure but reuse the long-lived ownership
kernel. None currently demonstrates the same need for another global
cross-package gate.

## Decided execution sequence

The durable roadmap should migrate toward the following execution strategy:

```text
Producer Property Read architecture proof              PASSED
                |
                +--> WP-400 early multi-owner checkpoint
                |      (existing WP-400 evidence, parallel)
                |
                v
Consumer one-shot domain-entry authority review
                v
Consumer Property Read architecture gate
                v
real Host Zenoh Consumer/Producer capability evidence
                v
controlled one-shot Core/Planning/Binding/Servient broadening
                v
long-lived subscription/emission domain-entry review
                v
ObserveProperty long-lived architecture gate
                v
capability-family expansion + staged legacy removal
                |
                +--> WP-500 after exact Core/TD prerequisites
                +--> WP-600 capability-by-capability real protocol evidence
                v
broad WP-100/200/300/400 closure and WP-500/WP-600 completion
                v
WP-700 final aggregation / absence / conformance
                v
v1 release review
```

This is an execution strategy, not permission to skip package dependencies or
tranche admission. Package closure and implementation entry remain distinct.

## Domain-entry sequencing

The 34 inactive identities should not be reactivated as one batch. The roadmap
migration should establish coherent review waves:

1. Consumer one-shot request/call/options/response prerequisites;
2. runtime scheduling/profile/status/resource prerequisites as the WP-400
   checkpoint and Consumer runtime require them;
3. codec/validation/security prerequisites when the selected Consumer boundary
   requires them;
4. streaming/subscription/emission prerequisites before the long-lived gate;
5. Directory client prerequisites before the first WP-500 tranche; and
6. advanced planning features such as cache/lazy/index/cost only when a real
   admitted capability requires them.

Each wave may re-adopt, replace, split, or retire historical identities. This
topic does not predetermine those dispositions.

## Required roadmap migration

Before any intersecting implementation tranche is admitted, a docs-only
roadmap/authority change should migrate this decision to the correct owners.
That change should at minimum evaluate and update:

- `PLAN.md` Critical Path and milestone dependency wording;
- `docs/design.md` package order;
- `docs/work-packages/index.toml`, including the WP-500 dependency and support
  for more than one integration-gate manifest if needed;
- WP-300 stale aggregate-gate wording;
- WP-400 checkpoint sequencing language if clarification is needed;
- WP-500 predecessor/start wording;
- WP-600 first production-tranche and feedback/product-evidence wording;
- WP-700's role as final aggregation rather than first architecture discovery,
  including its stale v4.9 completion wording; and
- the architecture/spec projection that states the progression from the first
  composition proof to later materially different ownership-topology proofs.

The migration must not silently activate Consumer or streaming requirements.
Those enter through their own reviewed domain-entry decisions.

Because this migration affects package ordering and cross-package architecture
gates, its final diff requires an independent architecture-level acceptance
review before merge.

## Rejected alternatives

### Continue broad horizontal expansion now

Rejected because it leaves the legacy Consumer and long-lived ownership
boundaries untested until too much package-local implementation has accumulated.

### Add only Consumer one-shot validation

Rejected because subscription/emission introduces a different lifetime,
progress, backpressure, stop/drop, and cleanup topology that cannot be inferred
from one-shot request/response.

### Gate every capability family

Rejected because write, action, collection, Directory, fan-out, and individual
protocol capabilities do not currently demonstrate distinct cross-package
ownership risks large enough to justify parallel governance machinery.

### Keep WP-500 behind WP-300 for sequencing convenience

Rejected because it encodes an unrelated Binding package as a prerequisite for
a client-only Discovery package whose active dependency boundary is
Foundation/TD/Core. Exact tranche dependencies provide the correct safety
mechanism without distorting the package DAG.

### Delay concrete protocol work until broad shared-SPI completion

Rejected because recent Zenoh feedback has already shown that real authoring and
runtime pressure can expose shared-SPI ownership defects that deterministic
package-local fixtures do not reveal early enough.

## Non-goals

This decision does not:

- design the Rust shape of `OutboundRequest`, `ClientBinding`, subscription
  drivers, response metadata, or Servient internal records;
- reactivate any inactive v5 requirement;
- create or accept either new gate;
- implement the WP-400 checkpoint;
- implement Consumer Property Read or `ObserveProperty`;
- modify package authority outside this workspace file;
- claim WP-500 or WP-600 implementation admission; or
- claim release readiness.

## Migration condition

This topic remains `DECIDED` until its durable conclusions are represented in
the appropriate roadmap, architecture/specification, work-package, and machine
projection owners through a reviewed repository change.

Only after that migration is integrated should this topic become `MIGRATED`.
