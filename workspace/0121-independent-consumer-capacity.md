# Persistent Consumer associations and finite capacity

Status: MIGRATED

Review 10's independent-capacity direction is implemented first through the
existing finite HTTP path. The relevant obligations are independent: revoking a
logical association must fence its work without destroying a sibling, retaining
prepared readiness requires no vacant Call Slot, and an unread result still
requires its original backing after the execution slot is reused.

The selected boundary replaces the bundled `ConsumerRuntime` constructor with
`ConsumerAssociation` and explicitly supplied `ConsumerCalls`. The finite slot
registry survives as a work owner because its reserved-slot checkout, passive
controls, unwind restoration and orphan cleanup already implement useful finite
mechanisms. It no longer owns one instance or its definition inventory. The
association owns checked policy/mapping references and passive root authority;
selected binding records fund their own lifetime rather than pinning unselected
mappings through a whole-root reference. Work snapshots original operands and
retains its work bank and any separate instance outstanding-call debit.

Compared alternatives: constructing the old root for each call still bundles
root loss with work and repeats binding/storage construction. Growing an
association-local call registry on demand removes idle allocation but preserves
the capacity topology. Returning only self-contained call handles would simplify
storage but remove the existing independent cleanup owner after handle loss.
A universal executor or mandatory fleet pool adds no needed capability. An
explicit finite work owner permits dedicated, shared or on-demand capacity while
retaining the already qualified finite lifecycle; caller-owned finite Read remains
a separate supported representation.

Cross-family challenge: a continuing Action must retain invoke/control mappings,
continuation and future control/result/cleanup funding after its initial exchange;
returning a finite slot is not that invocation's retirement. An established
notification conversation needs continuing cursor, loss, stop and response state
without traffic; passive association readiness is not an active subscription or
an occurrence-policy expression. A Producer exposure requires complete final-TD
qualification, behavior and closed native readiness before publication; neither
finite association construction nor spare work capacity establishes that claim.
The shared ConsumerInstance identity remains usable across facets, while these
owners remain family-specific. No common session or work-state hierarchy is
introduced. F1/F2/F3 remain outside this finite handoff.

The exact contracts are migrated into Runtime Safety, Foundation, Core, Planning,
Binding SPI and their existing architecture/API/state/work-package projections.
WP-400 links the real HTTP capability regressions. Historical scaling measurements
remain historical, not evidence for the replacement layout. No new governance
layer, architecture gate, milestone transition or acceptance claim is added.

## Completing Host lifecycle control

The passive split exposed two independent gaps: revoking an association could
leave a live I/O-driven controller asleep, and losing aggregate control could
leave an abandoned call retained solely by another call's shared backing.
The selected correction retains independent associations/capacity and finite
checkout, with active source/recipient relationships and qualified local abort.

Relationships coalesce by original source and actual progress recipient, with
explicit source-wide limits. They contain qualified Host signals, not calls or
Rust Wakers. A call reserves both association/instance dependencies and any
distinct polling/aggregate-cleanup recipients before effects. Fixed driver
domains, per-call observer inventories and a global supervisor are unnecessary.
The borrowed/manual representation retains its own driving contract.

| Transition | Linearization and retained ownership |
| --- | --- |
| Instance/association invalidation | Original currentness becomes false before notification; repeated invalidation cannot reactivate or redirect old work. |
| Dependency acquisition | A source-gated relationship insertion/count increment, followed by original currentness checks before effects; refusal retires acquired dependencies and returns input. |
| Notification ownership | Closing takes the source gate or commits work to its existing guard. The guard pins/detaches the finite batch before releasing the gate. Posting occurs outside the gate; a suspended guard can delay delivery. |
| Dependency release | Its count decrement; structural unlink under the gate and final record/endpoint/funding retirement after all notification and lease references disappear. Retiring records still consume the source quota. |
| Driver acknowledgement | Drain native readiness and acquire published notice state before surveying original work; later notices remain observable. A receipt is not funded cleanup. |
| Call handoff | Fresh original finite checks and complete funded slot-to-settlement transfer; notification dependencies cease, result backing remains charged. |
| Aggregate controller loss | Distinct from explicit close. Vacate abandoned original slots before qualified local destruction; a checked-out call remains in its lease until restoration. |

Source mutation is private, non-suspending, allocation/callback-free and bounded
by the configured record limit. An atomic busy/dirty/closed gate hands close and
last-release duties to an existing guard without waiting. No request flag alone
is a notification owner. Native backend failure that establishes no readiness
can leave a driver asleep and cannot be reported as delivered notification.
The Host backend and new disposal/reentry timings require executable evidence;
neither a metadata tag nor HTTP alone certifies third-party destructors.
