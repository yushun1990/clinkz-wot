# 0060 Roadmap Execution Sequencing Review

Status: DISCUSSING
Kind: roadmap and implementation-sequencing investigation
Baseline: `594c1b36f1d529f2e57ee30ddbfd601f0682e52e`

## Scope and authority

This topic investigates whether the current v5 execution roadmap orders
implementation feedback correctly after the passed Producer Property Read
architecture gate.

It is not architecture authority, does not amend `PLAN.md`, does not change the
work-package DAG, does not reactivate any inactive requirement, does not create
or accept a gate, and does not authorize source implementation.

The investigation exists because the current package-level dependency model may
be broadly sound while the tranche-level execution order still delays important
cross-package feedback.

## Trigger

The registered `PROPERTY-READ-ARCHITECTURE` gate has passed independent
acceptance. The current `PLAN.md` then advances directly to broad WP-100,
WP-200, WP-300, and WP-400 completion before WP-500 and WP-600 product work.

A repository-wide roadmap audit challenged that transition. The audit did not
find evidence that the macro ownership architecture must be discarded. It did
find that the first executable composition proof covers the Producer one-shot
path only, while several materially different target execution topologies have
not yet received equivalent early composition feedback.

The core question is therefore not whether the package graph should be replaced
wholesale. It is whether broad package completion has been incorrectly used as
the primary execution sequence after one narrow vertical proof.

## Established evidence at the baseline

The following points are repository-grounded observations rather than proposed
roadmap decisions.

### v5 domain entry remains explicit

The active v5 authority contains 62 active requirements and 34 identities with
`inactive-domain-entry-review-required` disposition. Those inactive identities
must be reconsidered when their domains enter implementation; work-package prose
alone does not reactivate them.

Relevant deferred areas include Consumer request/output behavior, advanced
planning, binding progress, subscriptions, emissions, scheduling, observability,
Directory execution, validation, codecs, and profile defaults.

### The passed Property Read architecture proof is Producer-side

The first cross-package architecture proof validates a Producer Property Read
path across Planning, binding compilation, Producer route preparation,
Servient publication, handler execution, response sealing/delivery, generation,
and cleanup.

Its narrow scope deliberately excludes Consumer invocation, subscriptions,
emissions, broad multi-route behavior, Directory execution, and WP-600 product
completion. This exclusion was valid scope control for the first proof; it does
not establish those omitted execution topologies by implication.

### The target Consumer path is not yet the active implementation path

At the baseline revision:

- the public narrow Property Read planner entry is Producer-route oriented;
- the narrow complete registration advertises Producer Property Read server
  behavior rather than a target Consumer invocation surface;
- `Servient::consume` still enters the legacy consumed-Thing path; and
- the legacy Consumer path still selects forms/bindings at runtime and builds a
  request that carries TD/Form material into `ClientBinding`.

That implementation topology differs materially from the v5 architecture in
which admitted immutable plans select execution facts before an owned
`OutboundRequest` enters the chosen client binding.

### Long-lived interaction remains a different ownership topology

The current streaming path still contains legacy Core-owned subscription queue
and merge behavior. The target design instead assigns protocol-local flow
control and subscription progress to the binding, a linear application facade
to Servient, explicit stop/drop/cancel ownership, bounded backpressure, and
native collection behavior without implicit per-affordance fan-out.

A one-shot Producer Property Read cannot prove those long-lived ownership and
cleanup transitions.

### Existing implementation history justifies early composition feedback

Recent Property Read work found several defects only when real successor
handoffs or external implementation pressure were exercised, including:

- Producer-role projection needed after a general planning path produced a
  Consumer-oriented shape;
- route-reservation identity could not be safely synthesized by a fixture;
- Host erased route-state succession required correction after real Zenoh
  authoring exposed an ownership defect; and
- response success required one Core-owned sealing boundary after review found
  a semantic bypass.

These findings support a general concern that package-local consistency alone
can permit cross-package incompatibilities to survive too long.

### WP-400 already contains an early multi-owner checkpoint concept

Broad WP-400 explicitly calls for an early scenario with multiple Things,
multiple bindings/routes, a hot owner, a never-ready owner, cleanup/deadline
pressure, and unrelated-owner progress before later facade work accumulates.

Whether that checkpoint should run before, during, or after the proposed
Consumer one-shot proof is a sequencing question; it does not automatically
require a new global architecture gate.

### WP-700 currently contains first-time lifecycle claims that may be too late

The final integration package includes end-to-end Consumer lifecycle,
subscription/emission boundaries, native collection behavior, response
boundary composition, and final plan-set lifecycle verification.

WP-700 should be examined for validation that is appropriately final
aggregation versus validation that would discover a new architecture mismatch
too late, after its predecessor packages already claim completion.

## Problem statement

The current roadmap may conflate two different concepts:

1. package completion dependency; and
2. the safest order in which implementation tranches should receive executable
   cross-package feedback.

The package graph can remain mostly correct while the execution strategy still
needs vertical checkpoints between broad expansions.

The failure mode under review is:

```text
one narrow architecture proof
        -> broad package expansion
        -> first Consumer/streaming/product composition much later
```

The alternative hypothesis is:

```text
prove one ownership topology
        -> controlled broadening
        -> prove the next materially different topology
        -> controlled broadening
        -> package closure
```

The investigation must determine whether that alternative reduces rework enough
to justify the added review/gate overhead.

## Candidate execution principles

The following are candidate principles from the roadmap audit. They are not yet
accepted decisions.

- Organize early vertical proofs around materially different lifecycle and
  ownership topologies rather than around every WoT operation.
- Add a formal cross-package gate only where package-local evidence cannot
  validate a real production handoff early enough.
- Require real upstream values to enter real downstream first-entry boundaries;
  fixtures may not synthesize an identity or owner that production code must
  provide.
- Keep deterministic external fixtures for exhaustive state semantics and use
  real protocol implementations early enough to expose authoring/runtime
  friction before shared SPIs are called mature.
- Re-enter inactive v5 requirements by coherent domain waves rather than by one
  giant broad reactivation or one authority cycle per method.
- Remove target-to-legacy backflow as each capability is productionized where
  practical, rather than relying exclusively on WP-700 for late cleanup.
- Keep gate count intentionally small. A new gate must prove that it catches a
  class of composition defect that focused package evidence cannot catch at the
  same useful time.

## Candidate roadmap shape under review

One audit candidate proposes the following broad sequence. This section records
an alternative for review; it is not a decision.

```text
passed Producer one-shot proof
        |
        +--> early WP-400 multi-owner/scheduler checkpoint
        |
        v
Consumer one-shot domain entry
        v
Consumer Property Read architecture proof
        v
matching real-protocol Consumer/Producer feedback
        v
long-lived interaction domain entry
        v
long-lived interaction architecture proof
        v
capability-family expansion and package closure
        v
WP-500 / WP-600 product completion as their real entry dependencies permit
        v
WP-700 final aggregation and release conformance
```

Under this candidate, only two additional formal cross-package architecture
proofs are contemplated:

1. a Consumer Property Read one-shot proof; and
2. one representative long-lived interaction proof.

Multi-route scheduling, native collection, multi-target emission, Directory
client behavior, and later operation families would retain focused package or
integration evidence unless review demonstrates a distinct cross-package
ownership gap that requires another gate.

## Questions requiring independent decision

The review should resolve each item independently instead of accepting or
rejecting the audit as one package.

### Q1. Consumer Property Read gate

Should a target Consumer Property Read vertical proof become a formal
cross-package architecture gate before broad Consumer planning/binding/Servient
expansion?

The candidate proof would cover at minimum consumed plan publication and
selection, owned `OutboundRequest`, selected client-binding execution,
caller-drop/cancel/timeout/late-result ownership, response validation, plan and
binding generations, cleanup transfer, and terminal zero-owner state.

A rejection should explain which existing focused evidence can falsify those
cross-package mismatches early enough without such a gate.

### Q2. Long-lived interaction gate

Is one additional formal proof required for the long-lived topology before broad
subscription/emission implementation?

If yes, review must choose the smallest representative semantic scenario. The
candidate should exercise driver installation, multiple deliveries,
backpressure/overflow, stop/drop/cancel races, remote terminal behavior,
Servient drain, emission/subscription handoff, and cleanup ownership without
becoming a general event framework.

The exact representative operation remains open. `ObserveProperty`,
`SubscribeEvent`, or a deliberately protocol-neutral fixture may each have
advantages; this topic does not choose one yet.

### Q3. WP-400 early multi-owner checkpoint ordering

Should the already-specified WP-400 multi-Thing/multi-binding/multi-route
checkpoint proceed in parallel with Consumer domain-entry work before the
Consumer architecture gate?

The candidate argument is that it validates the shared owner graph and
scheduling skeleton without opening broad subscription/emission facades. The
counter-risk is that it may stabilize runtime structure before Consumer call
ownership has supplied enough feedback.

This checkpoint should not become another formal global gate unless existing
WP-400 evidence ownership is demonstrably insufficient.

### Q4. WP-500 package dependency

Should WP-500 continue to depend on broad WP-300 completion, or should its
package completion/start dependency be narrowed?

This question is intentionally unresolved.

A crate-level dependency argument is not sufficient. Review must distinguish:

- Cargo/module dependency direction;
- required shared Core/TD/Foundation primitives;
- cancellation/status/resource semantics currently staged in other work
  packages;
- migration sequencing and legacy removal obligations; and
- whether a narrower tranche-level prerequisite is enough even if the coarse
  package DAG remains unchanged.

No `WP-500 -> WP-100` or other dependency rewrite is accepted by this topic
without that analysis.

### Q5. WP-600 product-evidence timing

Should real Zenoh/zenoh-pico work remain blocked until broad WP-300 completion,
or should each matched capability enter production protocol evidence as soon as
its required WP-300/WP-400 tranches and architecture proof are stable?

The candidate interpretation is not permission to bypass WP-300. It is a
proposal to shorten the interval between a shared SPI becoming usable for one
capability and a real protocol attempting to implement that exact capability.

Review must preserve the rule that a concrete protocol may falsify a shared SPI
but must not become the unreviewed owner of shared semantics.

### Q6. Legacy-removal timing

Should target/legacy migration edges be removed capability-by-capability once a
production target path replaces them, with WP-700 proving final absence, or
should most legacy deletion remain concentrated in WP-700?

The review should balance backflow risk against the risk of deleting compatibility
surfaces before all legitimate downstream migration users have moved.

## Related domain-entry waves to evaluate

The roadmap audit grouped deferred v5 identities by likely entry domain. The
grouping is useful review input but carries no authority and reactivates
nothing.

Possible waves include:

- Consumer one-shot: request/output/options and selected client-call contracts;
- runtime/scheduling/profile: scheduling, status, profile and Host shard
  semantics;
- codec/validation/security performance;
- streaming/emission: subscription storage/data, binding progress, subscription
  state, Producer emission and handler-subscription behavior;
- Directory client execution; and
- advanced planning: indexes, lazy compilation, cache/cost and broad form
  finalization.

Independent review may merge, split, reorder, replace, or retire these candidate
waves.

## Alternatives

### A. Keep the current broad-expansion sequence

After the passed Producer Property Read gate, continue directly through broad
WP-100/WP-200/WP-300/WP-400 completion and rely on later package and WP-700
integration evidence.

This minimizes new roadmap governance but accepts a longer interval before
Consumer and long-lived target composition is exercised.

### B. Add only a Consumer one-shot proof

Treat the current omission as a Producer/Consumer symmetry gap only, then return
to broad expansion after Consumer Property Read succeeds.

This is simpler than the full audit candidate but assumes long-lived driver and
emission ownership can be safely derived during broad implementation without a
separate early architecture proof.

### C. Staged lifecycle-topology validation

Add a Consumer one-shot proof and one long-lived proof, reuse WP-400's existing
multi-owner checkpoint, and keep later capabilities under focused package
and integration evidence.

This is the current audit candidate because it attempts to catch the two most
materially different unproven ownership topologies without creating an
operation-by-operation gate system.

### D. Gate every major capability family

Add architecture gates for write, actions, events, collection, Directory,
emission, and other domains.

This is not recommended as the default. It risks converting architecture
validation into a parallel project-management layer and requires a much
stronger falsifiable need than is currently demonstrated.

## Non-goals

This investigation does not:

- redesign `OutboundRequest`, `ClientBinding`, response metadata, subscription
  drivers, or Servient internals;
- reactivate `PLAN-REQUEST-001`, `BIND-OUT-001`, `BIND-PROGRESS-001`,
  subscription/emission identities, Directory identities, or any other inactive
  requirement;
- create Consumer or long-lived gate manifests;
- change any gate status;
- change WP-500 or WP-600 dependencies;
- implement a Consumer Property Read slice;
- implement the WP-400 multi-owner checkpoint;
- modify `PLAN.md`, work-package documents, architecture sources, ADRs, or
  source code; or
- encode Max/Ultra/High as permanent repository roles.

## Required independent review output

A fresh architecture-level review should reconstruct the claim from repository
authority and implementation rather than inherit this topic's preferred
alternative.

For each Q1 through Q6 it should return one of:

- `ACCEPT`;
- `REJECT`; or
- `MODIFY`.

Each verdict should identify:

- the repository evidence that controls the decision;
- the earliest executable evidence able to falsify the proposed sequencing;
- the cost or rework risk of delaying that evidence;
- any authority/domain-entry consequence; and
- the smallest roadmap change, if any, needed to implement the verdict.

The review should also explicitly challenge whether two new formal gates are
already too many, too few, or correctly scoped.

## Migration condition

This topic may become `DECIDED` only after the six questions above have stable
technical dispositions and the resulting roadmap does not conflict with active
v5 authority.

If the decision changes durable ordering, package dependencies, architecture
gate structure, or domain-entry sequencing, those changes must be migrated to
their proper owners through normal reviewed repository changes. Only after the
accepted conclusions are represented in the relevant `PLAN.md`, work-package,
architecture/specification, ADR, or machine projection should this topic become
`MIGRATED`.
