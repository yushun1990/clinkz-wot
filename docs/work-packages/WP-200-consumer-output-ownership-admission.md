# Consumer output ownership admission

[`index.toml`](index.toml) owns the state of
`WP-200-CONSUMER-OUTPUT-OWNERSHIP`. This record defines its exact v5.1 ADR-0013
implementation and evidence envelope.

[Independent admission review of PR #156](https://github.com/yushun1990/clinkz-wot/pull/156#issuecomment-6137329268)
accepted base `e725de484d62b17e07fccd19e7cf5f8e437c6f20` to head
`7923a966238e2c4efb5dde5afd5538448f0da793` with no blocking findings, subject
to successful required CI. This separate admission transition follows that
acceptance and satisfied CI condition. Admission covers this envelope only;
production completion still requires the evidence and independent acceptance
defined below.

## Decision and bounded claim

Add a consuming, allocation-free ownership bridge for one completed Consumer
Property Read draft and its **matching physical manual cleanup root**, and for
the corresponding source-free variable rollback graph. The bridge moves the
existing backing and accounting; it neither copies plans/artifacts nor changes
their selection or reclamation program. Building and TD progress remain local.

The [construction investigation](../../workspace/0076-composable-capabilities-and-runtime-boundaries.md#bounded-host-construction-result)
compares a local driver, quiescent transfer and a shared carrier against current
source. A local driver works when built and retained on one thread, but adds
command/delivery/shutdown ownership if used to serve transferable Host handles.
Transfer of a complete quiescent graph is a smaller lower-layer correction.
Wrapping the existing graph in `Arc`/`Mutex` fails its auto-trait contracts;
replacing the whole build/root protocol with shared synchronization is unnecessary
for this claim. Host synchronization and scheduling remain WP-400 decisions.

The [existing external fixture](../../tools/architecture-fixtures/consumer-runtime-entry/README.md#construction-boundary-experiments)
compiles an unchanged copy of the current Planning program with a private
experimental adapter appended. It demonstrates feasibility, not the existence
or acceptance of the proposed public API. The production aggregate's accepted
local/manual-root contract remains valid. The correction preserves architecture
ownership and semantics and needs no new ADR, crate, scheduler or resource schema.

## Admitted public surface and invariants

The additive advanced surface is `std` only, in `clinkz-wot-planning`:

```rust,ignore
pub struct ConsumerPropertyReadOwnedOutput<A: 'static> { /* private */ }
pub struct ConsumerPropertyReadOwnedRollback<A: 'static> { /* private */ }

impl<A: 'static> ConsumerPropertyReadOwnedOutput<A> {
    pub fn try_from_local(
        draft: ConsumerPropertyReadDraft<A>,
        root: ConsumerPropertyReadCleanupSlot<A>,
        budget: &mut WorkBudget,
    ) -> Result<Self, (
        ConsumerPropertyReadDraft<A>, ConsumerPropertyReadCleanupSlot<A>, CoreError,
    )>;
    pub fn into_local(self) -> (
        ConsumerPropertyReadDraft<A>, ConsumerPropertyReadCleanupSlot<A>,
    );
}
impl<A: 'static> ConsumerPropertyReadOwnedRollback<A> {
    pub fn try_from_local(
        rollback: ConsumerPropertyReadRollback<A>,
        root: ConsumerPropertyReadCleanupSlot<A>,
        budget: &mut WorkBudget,
    ) -> Result<Self, (
        ConsumerPropertyReadRollback<A>, ConsumerPropertyReadCleanupSlot<A>, CoreError,
    )>;
    pub fn into_local(self) -> (
        ConsumerPropertyReadRollback<A>, ConsumerPropertyReadCleanupSlot<A>,
    );
}
```

Both bridge owners are unique, non-Clone and structurally `Send` when `A: Send`.
They promise no `Sync` capability. No unsafe `Send`/`Sync` implementation is
permitted. The current build, draft, slot and rollback types retain their local
auto traits. No registration, TD, compiler, source or stack loan enters either
bridge owner. A borrowed selection cannot survive consuming its draft.

`try_from_local` checks the actual common allocation, phase, absence of fixed
cleanup and the exclusive two-reference draft/root or rollback/root condition.
Equal public reservation tuples, identities, ceilings or footprints are
insufficient. Wrong root/phase returns both original owners and a fixed,
allocation-free validation cause without consuming work. A fixed pre-allocation
rollback is outside this bridge and is returned unchanged as unsupported; it
continues to use the existing local cleanup path.

One `PlanningItems` credit prepays the complete fixed detach/reconstitution
pair before changing ownership. Short credit returns both originals with a
fixed Backpressure cause and no progress. `into_local` consumes that prepaid
owner, reconstructing exactly one matching root/lease pair without allocation,
callback, fallible release or a second charge. Neither method drives selection,
compiler or variable cleanup. Repeated bridging requires fresh credit from the
coordinator's retained allowance, not refilled Planning lifetime work.

Retain the original node, rows, coordinates, artifact addresses, identities,
runtime ledger, peak history, first cause, partial reclaim credit and cleanup
reservation metadata. Once local again, existing selection, abandonment and
`Rollback::step` remain the only algorithms. Private helper extraction is allowed
to implement these invariants; a second selector/reclaimer or whole-root atomic
rewrite is outside scope.

The bridge owner is the complete advanced manual root while detached. Dropping
it with live variable output does not perform variable destruction or claim
recovery. As with the existing root, an advanced caller must retain and explicitly
drain it. WP-400 must retain this owner in its generation/cleanup record, never
make a delivery view its sole owner, and prove executor/manual fallback and root
shutdown separately. This API supplies no publication or parent-capacity permit.

## Dependencies, source and resource envelope

The completed/current aggregate and Consumer-only complete-registration tranches
are direct predecessors. Their passed evidence already owns TD semantics,
compiler barriers, concrete artifacts, installed Core sealing and local reclaim.
The role-only predecessor is used for the real client-only completion fixture.

Permitted production paths are only `planning/src/consumer_property_read.rs`
and `planning/src/lib.rs`. Focused tests may extend Planning tests and the
existing Consumer runtime-entry fixture. Core, Foundation, Servient, protocol,
dependency/feature manifests and all legacy code are outside implementation scope.

No new allocation, copied output, parent account, resource kind or `WorkClass`
is introduced. Existing output footprint and allocation history remain unchanged.
Measure the actual native owner/return/inline transport Layouts and any overlap;
keep them within the already reserved cleanup envelope, with checked layout
arithmetic. The original committed cleanup `ResourceAccount` remains a separate
coordinator-owned object and must travel or remain live with its children.
The bridge must neither infer its provenance from public metadata nor release it.
Changing the required capacity or work envelope beyond this projection stops
implementation for scope amendment and review.

There is no new performance workload: the added operations are fixed ownership
conversions with zero allocation and zero variable traversal, measured by the
focused harness. Existing bounded aggregate/reclaim workload semantics remain.

## Pre-implementation checks and impact

Use Rust 1.95.0 and the exact commands in `index.toml`: current authority checks,
locked workspace tests, the existing runtime-entry std/no-default/async tests
and both ARM compile cells. The std construction tests cover local-worker and
quiescent alternatives, wrong-root/short-credit rejection, partially paid cleanup,
original backing release, real parent cleanup reservation and isolated fatal Host
allocation failures. Current public auto-trait negatives remain applicable.

Topic 0076's ownership subquestion is migrated into this record and Planning's
projection. Its still-open call allocation, callback support, runtime parent
pairing, shutdown, deployment and broader capability questions are disjoint from
this no-callback/no-allocation bridge. They are **not** disjoint from the future
WP-400 runtime. No accepted predecessor evidence is invalidated. Existing
predecessor admission/completion/impact and gate states remain unchanged.

The linked independent admission review covers this exact record and executable
experiment, the selected boundary, unsafe allocation ownership proof and
manual-retention limitation. Production completion requires the public evidence
below and separate independent acceptance.

## Required production completion

Stable key: `consumer-output-ownership`. Output:
`docs/evidence/WP-200-consumer-output-ownership.toml`. Record the implementation
revision, commands and actual native Layouts. Required evidence:

1. Use the **public production** bridge with actual paid aggregate output from
   complete Consumer-only registrations. End TD/build/compiler loans, move the
   graph to another thread, use the original selector and concrete Host artifact
   through the complete Core sealed call path, and reclaim to zero retained
   output/backing with exactly one final original-node release.
2. Preserve actual artifact addresses, complete generation/registration identity,
   footprint, ledger peaks, first cause, root metadata and partially paid byte
   progress across draft and rollback transfers. Exercise repeated round trips,
   draft abandonment, rollback abandonment and zero/short/exact credit.
3. Reject unrelated equal-looking roots, vacant/occupied/fixed roots and wrong
   phases with unchanged owners, charge/history and no work/allocation/callback.
   Compile contracts prevent outstanding selection loans from crossing a transfer,
   forging/cloning owners and transferring non-Send artifacts; wrappers must not
   silently change current local auto traits.
4. Observe zero allocation and variable traversal during both conversions, no
   second output/root charge, and original child-before-cleanup-account release.
   Re-execute existing public aggregate, static/Host sealed execution and portable
   compile/runtime evidence. The private experiment cannot substitute for these
   production tests.

This completion cannot establish a Published record, generic thread-safe build,
fallible Host call backing, arbitrary callback bound, global governor, executor
drop recovery, Consumer runtime admission or architecture-gate acceptance.
