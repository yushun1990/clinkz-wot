# Finite Producer capacity after shared native HTTP publication

This is scoped Host/manual executable evidence, not independent acceptance,
milestone/package closure, production qualification or a performance claim.
[Topic 0128](../../workspace/0128-producer-finite-capacity.md) records the
counterexample and alternatives. Exact ordering and optional/default policy
belong to [Runtime Safety](../spec/runtime-safety.md#finite-producer-participation-and-work-capacity);
[Foundation](../spec/foundation.md) owns the original account lifetimes.

## Reassessment and resulting boundary

The post-#284 HTTP driver retires its connection and releases its native lease's
busy flag at successful send. The default Host runtime nevertheless guards
Serving with `settlement.is_some()`, and managed admission includes one maximum
input/result opportunity per participant. These are restrictions of the dedicated
finite policy, rather than requirements of continuous publication, native reply
identity or final-TD qualification. Retaining results may legitimately exhaust
bounded result storage; it need not consume the same credit as active execution.

| Concern | Result of this increment |
| --- | --- |
| Publication and native readiness | Persistent participant, complete independent qualification and local publication; no vacant shared work opportunity required |
| Incoming native parsing/buffering | Existing Binding ownership, native bounds, route leases and original accept deadline |
| Admitted Engine finite work | Original shared work bank reserves active and retained credit, complete input/result/cleanup before receive |
| Terminal native exchange | Releases active credit and ingress-item debit; no longer occupies native connection |
| Unread or caller-held outcome | Bounded original result, behavior, publication funding and work/cleanup leases survive until disposal |
| Default dedicated opportunity | Deliberately still requires acknowledgement before another receive |

`RawFiniteRuntime` remains the typed native participant and exclusive active
exchange owner. `ProducerFiniteCapacity` supplies optional accounting and capacity,
without owning a work/result registry, publication or dispatcher. Original
outcomes stay in bounded participant-local FIFOs; `take_retained_settlement`
transfers a complete owner with slice-only output and borrowed target coordinates.
Checked sequence numbers distinguish requests in their original capacity; all
native correlation/mapping checks still run inside the unchanged HTTP Binding.

Each reservation attempt reserves all six work accounts before allocating its
future outcome record and polling native receive. Refusal rolls back only that
attempt. Pending native parsing returns the temporary Engine opportunity while
native buffering remains covered by Binding policy. Terminal send/close disposes
the original input/reply before returning active/ingress credit. FIFO insertion
checks its actual unread-link count plus one before native terminal handoff,
then charges that allowance once on terminal success, before mutation. Drop order
keeps active work leases through physical Binding/configuration disposal; terminal
owners destroy result/behavior backing before releasing their original leases.
No capacity borrow or lock spans a linked callback or destructor.

Publication banks pay persistent description/behavior/prepared/native/membership
backing. Shared work banks pay the actual admitted opportunities; there is no
second idle input/result charge in the participant's managed declaration. Result
retention conservatively keeps its complete byte/cleanup envelope, outstanding
handler and response quotas, even after acknowledgement. Those quotas and the
separate retained limit intentionally bound progress. Counters are reservation
evidence, not allocator measurements or transitive process/OS/SDK RAM totals.

## Real execution

The seven [shared-listener capacity cases](../../protocol-bindings/protocols/http/tests/composed_thing/capacity.rs)
run actual preparation/validation, managed admission, complete retained final-TD
qualification, independent publication, production HTTP listener/driver and real
loopback TCP. Definition, publication and shared work banks are separate.

- With one shared native credit and one active slot, always poll the saturated
  receiving participant first, then the sibling holding the original reply.
  Refusal preserves the credit; the sibling sends and releases active capacity,
  and both original HTTP outcomes remain distinct and funded. At the original
  PR head `3c93c9da75e2696167341cbad626260a62c7d5fe`, four such turns instead
  left zero credit each time and the releasing sibling stayed Delivering.
- Publish a second Thing while the only active credit is occupied. A second
  request cannot dispatch until native settlement returns that credit. Then a
  sibling request and a later request to the same continuously published Thing
  complete with old outcomes unread. Three retained outcomes exhaust the result
  limit; taking one without disposing it does not permit another admission.
  FIFO, distinct original sequences, recipient/operation/results, and all-bank
  release are checked. A native poll without prepaid FIFO work makes no send.
- Saturate each of the six actual work accounts independently. No receive or
  handler runs; all partial reservations roll back and each refused turn retains
  its native poll credit. Publication remains current.
  Zero work also makes no acquisition. Fresh request policy denies before effects,
  and later independent allowed work progresses while that refusal is retained.
- Replace a Thing while its accepted reply remains pending. With two active
  credits, the successor's distinct native route accepts and completes before
  original close. Original zero-credit/Pending ownership, first cause, result,
  native delivery and sequence cannot move to the successor. Native close alone
  cannot retire an unread outcome. Acknowledged/shared outcomes keep original
  work/publication banks after all roots disappear; final native leases release
  the socket before those outcomes are disposed.
- Refuse foreign-owner, undersized and post-start capacity attachment while
  returning the complete native participation; invalid capacity construction
  leaves no charges or native effects.
- Incomplete original native headers consume no lasting Engine opportunity.
  Another Thing completes on the same listener; finishing the original input
  later dispatches only its original recipient. Both unread records still
  prevent their own final logical retirement until acknowledgement.
- Lose a participant with received or dispatched original work. Actual HTTP
  local abort releases the socket and work debit without dispatching/replaying
  anything new. Registry retirement and final bank release remain explicit.

The [effectful HTTP Write case](../../protocol-bindings/protocols/http/tests/property_write.rs)
uses the existing actual native adapter with controls that only pause send/close.
Success, a handler failure after committing a write, and an invalid result after
another committed write all remain unread while subsequent requests progress.
They retain Completed/Unknown effect evidence separately from delivery. A fourth
validated write enters Pending retirement: zero credits and paused close cannot
refund active work. Five Pending closes reuse the same four queue units; a final
close inserts behind three unread outcomes for exactly four units. Terminal abort
retains Completed with NotDelivered, the first local cancellation and original
result. No write is replayed. Returned outcomes
keep original behavior/publication/work funding through root loss and release
it only on disposal. This case uses an exclusive HTTP native listener; the seven
cases above establish the shared-listener execution boundary.

Three [controlled native accounting regressions](../../servient/finite/tests/native_ingress/capacity.rs)
separately isolate the terminal handoff. At the original PR head, three Pending
send polls spent all 96 queue units without inserting anything; a one-unit queue
allowance also prevented all close callbacks despite a retained limit of one.
The corrected cases exercise retained limits 1, 3 and 32; every FIFO insertion
cost from one through the limit; pop/reinsert counts and FIFO sequence; zero
native credit without acquisition; retained pressure before and after
acknowledgement; repeated Pending send/close with one shared queue allowance;
error send and Pending/error/terminal cleanup of both unaccepted input and an
accepted reply. A linked reply destructor asserts active/ingress and original
cleanup/response/handler funding are still live at disposal. Terminal results
retain the original bank after runtime/root loss and release it on final disposal.
Short queue credit preserves native/cleanup credit and makes no callback. These
probes establish accounting/ownership order, not additional protocol coverage.

Focused commands:

```sh
cargo test --locked -p clinkz-wot-binding-http --test composed_thing shared_work -- --nocapture
cargo test --locked -p clinkz-wot-binding-http --test property_write shared_producer_capacity -- --nocapture
cargo test --locked -p clinkz-wot-servient-finite --test native_ingress shared_capacity -- --nocapture
```

The existing default-policy Write/unread-result and shared-listener tests remain
regressions for their unchanged scopes. Package tests cover Core, Planning,
Servient, finite execution and HTTP; authority checks and no-default-feature
checks are recorded with the exact verified revision in the PR. Required GitHub
`validation` means diff hygiene only.

## Deliberately unchanged limits

Each HTTP participation still has one queued-or-active connection and one native
correlation at a time. Per-participant inline exchange bookkeeping remains; no
per-participant footprint reduction is claimed. The shared listener still bounds routes/unclassified heads,
requires compatible timeout/address/clock and retains original route-generation
isolation. Shared work/result saturation prevents receive progress; neither this
policy nor publication promises queueing, overload fairness, a wire refusal or
deadline maintenance without funded receiving turns. All progress, cleanup and
result observation remain caller-driven; no wake-only guarantee or timer appears.
An actual Pending native poll still spends its poll credit. Callers must provide
and distribute sufficient native/cleanup turns and the current FIFO insertion
allowance; preserving credit on capacity refusal is not general scheduling
fairness. The FIFO remains linear within its configured retained bound of 32.

The default raw/dedicated constructor and narrow single-Property exposure keep
their existing policy. Continuing Producer Actions and notifications, general
Producer Host-loop composition, shared secure native infrastructure, mixed-family
work banks, constrained profiles, performance/scaling and production qualification
remain separate. One work bank supplies shared bounds, not per-Thing hard isolation
or an aggregate quota across independent capacities. Local socket delivery does
not establish remote receipt, exactly-once effects or rollback.
