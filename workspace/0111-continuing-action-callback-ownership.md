# Continuing Consumer Action callback ownership

Status: MIGRATED

Basis: master `7f8c02250a83604455b2824910c5db59d4de625c` after PR #264.
This investigates the distinct owner identified in Review 09 and the existing
SAFE-CONCUR-001 obligation. The source inspected was
`servient/finite/src/consumer_action.rs`, including admission, handles,
progress, control, settlement and retirement; the finite correction was evidence
for comparison, not implementation authority for Action lifecycle semantics.

## Findings and limits

The original owner held `RefMut<State>` across actual access, data-validation,
native destination/start/poll/close and destruction callbacks. Public-API
regressions in `servient/finite/tests/consumer_action_callbacks.rs` reproduce
borrow panics in admission, query/cancel authorization, progress, settlement and
abandoned token destruction. Separately executed native poll, retirement,
validator and native disposal reentry also panic on the basis. The initial
reproduction is a separate Git checkpoint, before the implementation correction.

The original implementation does **not** silently double-poll or overwrite a slot
in those executions: the borrow panic prevents the nested operation. Merely
releasing the borrow or removing an invocation temporarily would expose those
hazards. Nested capacity, identity, single-poll and stale-handle tests establish
the correction's obligations rather than misreporting baseline corruption.

Atomic ConsumerInstance revocation during authorization already refuses before
native start. The positive-control test passes on both the basis and corrected
owner. Existing native token, described-control, cancellation uncertainty and
late-retirement semantics need preservation, not replacement.

Additional regressions fail on the basis after a control access callback removes
the data service: restoring the service permits another query instead of retaining
the observed failure as a fence. An injected validator unwind also loses the
native output because it is still a callback-local value. The correction records
control dependency failure and owns native output before validator entry. These
are existing first-cause/evidence obligations, not new remote guarantees.

## Alternatives and decision

- Retain the global borrow or replace it with a lock: same execution/registry
  coupling; nested entry panics or deadlocks.
- Vacate the slot, silently ignore failed borrows, or retry the original invoke:
  loses capacity, control intent or original work; can duplicate remote effects.
- Put each complete invocation behind its own borrow and refuse every nested
  access: isolates siblings but prevents same-invocation cancellation during a
  callback unless passive control is separated anyway.
- Introduce a shared finite/Action runtime or scheduler: unnecessary and obscures
  the continuing token, explicit query/cancel exchanges and retained lifecycle.
- **Selected:** short registry transactions, an occupied reservation during
  admission/checkout, a scoped original-owner lease, and separately retained
  passive invocation control. Continuing state, opaque native token and complete
  evidence stay in the existing Invocation. No new public carrier or executor.

The control records first cause, abandonment, latest entry time and wake interest
without accessing native ownership. Busy operations return Backpressure;
progress requests a retry on restoration. Rechecks fence callback-time changes
before effects/handoff. Returned native work and late results retain their
original leases. Original cleanup capacity includes the control backing, and
its independent live-byte lease lasts through the last handle/owner. Native
result ownership precedes validation; restoration on unwind is not a promise
that arbitrary panicking callbacks can resume.

## Migration and evidence boundary

The normative owner is [Runtime Safety](../docs/spec/runtime-safety.md), with
its affected lifecycle, API requirement and accepted-work projections updated.
[WP-400](../docs/work-packages/WP-400-servient.md) links implementation evidence.
Core Action identity, opaque token, mapping SPI and remote outcome semantics
remain authoritative and unchanged; no new ADR, gate or roadmap frontier is
needed for this local ownership correction.

Deterministic public-API tests cover callback reentry, slot retention, wakes,
nested time, cancellation, settlement, short work, unwind and funded destruction.
Actual HTTP Consumer-to-Producer tests exercise reentrant access, native and
validation boundaries with retained completion. Existing independent datagram
invoke/control Bindings test native ownership and final physical release.
Exact executed revisions and commands belong in Git/PR validation evidence.
These do not establish all-family callback isolation, constrained execution,
production qualification or independent acceptance of this candidate.
