# Persistent Producer participation and finite work capacity

Status: MIGRATED

## Counterexample and affected authority

After #284, independent HTTP publications share native infrastructure, but
`RawFiniteRuntime::step` still stops receiving while its one settlement is
unread. HTTP has already retired that exchange and released native occupancy.
The managed participant's admission declaration also reserves its maximum
input/result opportunity for the entire publication. Thus a continuously
published Thing implies a dedicated Engine work opportunity, and observing one
old result is a prerequisite to processing an otherwise independent request.

One exclusive active exchange per HTTP participation is a concrete bounded native
policy: its parser, accepted mapping and reply correlation are singular. Complete
publication/readiness and original native retirement also require a persistent
participant. Those facts do not require reserving execution/result backing for
every idle publication or blocking work behind a completed unread result.

Affected owners are Runtime Safety's finite Producer policy, Foundation's
capacity-before-acquisition/result lifetime contract, Core's raw settlement
description, the finite publication admission projection, the API ownership
projection, WP-400 and PLAN. SPI-NATIVE/INGRESS/DELIVERY/RETIRE and independent
publication remain binding; no native SPI or protocol interpretation changes.

## Alternatives and selected increment

| Alternative | Disposition |
| --- | --- |
| Require prompt acknowledgement or one runtime per request | Reject as the only composition: observation is not native retirement, and publication cannot be rebuilt per request |
| Add only a larger runtime-local result queue | Removes one obstruction but still permanently dedicates maximum work backing to each publication |
| Copy Consumer associations/call handles | Reject: native incoming input/reply opportunities, closed publication and native leases have different lifetimes from outgoing calls |
| Remove RawFiniteRuntime or add a universal Runtime/scheduler | Reject: the typed participant already orders native exchange access and implements useful qualification/retirement obligations |
| Add concurrent HTTP exchanges per participant | Defer: would change native parser/correlation/buffering policy; sequential requests and independent participants suffice to falsify the Engine coupling |
| Optional independent finite work capacity on existing participants | Select: shared or dedicated work banks, no idle exclusive slot, bounded retained outcomes independent of active credit |

`ProducerFiniteCapacity` belongs to one logical owner and an explicit original
work bank. Persistent participants may attach before setup and publish even
when no work opportunity is vacant. Before native receive, reserve active and
retained credit plus complete Engine input/result/cleanup backing. Pending
native parsing remains Binding-owned and releases the temporary Engine credit;
the Binding's native deadline, occupancy and buffering bounds remain unchanged.
Accepted candidates, dispatch, refusal and native close keep the original permit.

Native terminal settlement returns active credit and retains the original
outcome, behavior and publication funding in a bounded participant-local FIFO.
Acknowledgement transfers that complete owner. Only final disposal refunds
retained-work/result/cleanup capacity. Slice-only output/target views prevent
unfunded original backing from escaping. Local sequence numbers distinguish
results within the original capacity; native request/reply correlation remains
entirely Binding-owned. No shared result registry, owner/queue reference cycle,
automatic driving, scheduling or new dispatch authority is introduced.

The existing constructor deliberately retains its single dedicated opportunity
and raw result handoff. It is an explicit small bounded policy, not a universal
Producer invariant. The optional narrow `ProducerExposure` facade and continuing
Action/notification capacity policies are unchanged. The new shared path must
prove native I/O, independent publication/permission, active and retained pressure,
original results through replacement, exact bank lifetime and final retirement.

## Migration and evidence disposition

The selected optional policy is implemented in `servient/finite`; HTTP keeps its
existing production native driver and listener unchanged. Detailed ordering and
limits live in Runtime Safety; Foundation owns exact account and reservation
meaning. Core's carrier comment, Binding/native policy, architecture/API/state
projections, WP-400 and PLAN now agree. No new ADR is needed to replace existing
cross-domain ownership: this increment applies the existing separate logical,
native, work and retained-result owners through an optional finite policy.

[Executable evidence](../docs/evidence/producer-finite-capacity.md) establishes
fully qualified publication without vacant work, active limits of one and two,
same-participant and sibling progress behind unread outcomes, retained-pressure
refusal, all six work-account failure points, incomplete native input, original
replacement/results and bounded loss/cleanup. Actual HTTP Writes preserve
Completed/Unknown evidence across failures and Pending retirement. Historical
shared-listener evidence remains valid for its native topology and dedicated
policy; its old observation restriction is not a universal Producer contract.
No package/gate/milestone closure or independent acceptance is asserted.

## Shared-budget counterexample and local accounting correction

The initial PR implementation charged `BindingPolls` before attempting work
admission. With one active slot occupied by A's original reply, an idle published
B polled first under each one-credit shared turn consumed the credit, failed
admission and invoked no Binding callback. A could never send and release the
slot. This is an accounting-induced progress cycle even with an otherwise ready
native send; larger independent per-participant test budgets concealed it.

The initial terminal path also prepaid the global 32-link FIFO bound on every
send/close attempt. Pending/error polls inserted no outcome yet repeatedly spent
that allowance; a one-slot capacity could not settle under a one-unit queue
budget. This evidence invalidates the unconditional charge, not the separation
of publication, original work and retained results.

Select local prerequisite checks and exact mutation charging. Check native
credit without acquiring on zero; acquire all original work funding before
consuming the receive poll credit immediately before its callback. Track only
the participant FIFO's unread count, check count + 1 queue units before a
potentially terminal native poll, and charge once on actual terminal insertion.
Exclusive runtime/budget borrowing keeps the checked cost available across
callbacks. Retain existing original work, first cause and disposal/refund order.

Merely increasing budgets hides both defects. A scheduler/rotating caller order
would disguise the lost credit without correcting its ownership. Charging the
configured maximum once per work would remove repeated charging but still bill
unperformed traversal for short queues. Replacing the bounded FIFO with a new
container or shared result registry adds no necessary capability. A local count
preserves the existing allocation/ownership topology and exact FIFO behavior.
Runtime Safety owns the corrected ordering; the existing evidence record links
the deterministic red/green cases and their execution limits.
