# Shared logical Producer ownership

Status: MIGRATED

## Evidence and affected owners

At `a402bf07cffe09ee6fffbaf97f3ab210c72f1347`, the finite Host
`RawFiniteRuntime<B>` owns `Vec<RawFiniteInstance>` and closing one driver
revokes every instance in that vector. The unique Core authority and move-only
handler set therefore cannot participate in two differently typed runtimes.
The older preparation-dependent Producer root has the same narrower ownership
shape; extending it would also import optional TD/Planning into execution.

This is an implementation restriction, contrary to the general separation in
ADR-0025, SAFE-AUTH-001, SAFE-CONCUR-001, SPI-NATIVE-001 and SPI-RETIRE-001.
EXEC-ID-001, SPI-DELIVERY-001, SAFE-CANCEL-001, SAFE-CLEANUP-001,
RES-OWNER-001, RES-HANDOFF-001 and RES-WORK-001 constrain its replacement.
Core owns identities, raw Property result checks and settlement contracts;
Servient owns current grants, behavior and accepted work; each concrete Binding
owns native opportunities, progress and physical retirement.

## Alternatives

- Copy instance keys and handlers into each Binding runtime: rejects the unique
  authority requirement and permits divergent revocation/replacement.
- Put the registry behind a shared mutex and clone access into runtimes: can
  serialize authority but creates lock/reentrancy and callback capture machinery
  unnecessary for this manually driven synchronous slice. It also encourages
  native ownership to retain dispatch access.
- Make Servient erase and own all native drivers behind a universal service or
  session: unnecessary. Native reply/configuration types already have the proper
  concrete owner; independent retirement needs no transport common denominator.
- Borrow one Servient owner for semantic progress, leaving each typed runtime
  independently owned: selected. Exclusive mutable access orders acceptance,
  revocation and replacement; no locks surround callbacks. Native retirement
  remains drivable even after the logical owner is dropped.

## Selected executable boundary

`RawFiniteServient` owns one bounded registry. Its private shared instance backing
contains the sole move-only Core authority and actual handler set. Registry entry
currentness is never copied into a runtime. A runtime has only an opaque weak
issuer association, its captured Binding/configuration, one exchange/result/
settlement opportunity and, after acceptance, a retention reference to the original
instance. Retention grants no dispatch or shutdown permission.

The single-thread manual Host facet uses `Rc` for retention rather than requiring
`Sync` callbacks or adding a global lock. Thread mobility, async handlers and
constrained execution require separate qualification, not compatibility wrappers.
Native progress never calls a handler. Servient progress checks issuer, actual
Binding pair, current key, operation, policy and bounded capacity before invoking
the captured behavior. Complete count/name-byte lookup work is prepaid. Native
steps, dispatch and cleanup each consume their own linear allowance.

Replacement validates a complete same-slot successor before changing the entry.
The entry update is the linearization point; old native candidates fail even if
their numeric slot/generation is reused. Already accepted exchanges pin the
original instance through delivery/close and settlement acknowledgement. Terminal
settlement includes the originating Binding identity and accepted target, owned
late output, effect and delivery separately. Retiring or dropping one runtime
neither revokes a logical instance nor closes another Binding. Dropping Servient
fences new acceptance; accepted work and native cleanup retain their owners.

Each runtime slot and result opportunity is caller-provisioned and independently
bounded; the registry has no growing runtime/job list. This does not assert a
process-wide RAM limit, scheduler fairness, forced callback preemption or native
drain on Drop. Native providers still owe finite local disposal and bounded polls.

## Migration and falsification

Replace the runtime-owned registry API directly. Update its callers, API ownership,
module projection and WP-400 scope. ADR-0025's ownership decision is reaffirmed;
no new protocol, authority revision, milestone or historical gate is introduced.
PR #229 evidence remains evidence for its exact original revision; rerun changed
finite-ingress regressions rather than inherit acceptance.

Required executable counterexamples: one behavior with mutable application state
served through independently typed native I/O; retiring/stalling/failing one
Binding while another progresses; revoke/replace between receive and acceptance;
replacement during pending accepted send and failed close; original owner/result
retained through acknowledgement; wrong-issuer refusal; logical-owner Drop;
zero/short/exact dispatch and cleanup budgets; rejected replacement returns its
complete original input without changing authority. TD publication, preparation,
Action/Event lifetime and production protocol qualification stay separate.
