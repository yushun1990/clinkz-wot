# Continuing Action execution across ownership representations

This increment advances SAFE-PROFILE-001 from caller-owned finite Read to one
source-free continuing Consumer Action path. It is executable ownership evidence,
not constrained TD preparation, production Binding qualification or independent
architecture/package acceptance. Investigation and alternatives are preserved in
[topic 0130](../../workspace/0130-continuing-action-runtime-independence.md).
The normative owners are [Core](../spec/interaction-core.md#continuing-execution-across-representations),
[Runtime Safety](../spec/runtime-safety.md#caller-owned-continuing-action) and
[Binding SPI](../spec/binding-spi.md).

## Reassessment after Host integration

Reconstructed baseline: `dbf904527d3d727c10a5d760a0d468db52c7b6cd`.
The enduring goal is a protocol-neutral WoT semantic Engine over actual
Binding-owned native execution, with equivalent supported Host/constrained
meaning and independent logical authority. Topic 0081's current authority is
PREP-REUSE-001, ADR-0025 and the compiled-definition lifecycle, rather than its
historical Plan-Set proposal. SAFE-PROFILE-001 separates meaning/ownership from
storage, erasure, atomics and driving.

| Guarantee | Implemented baseline | What it does not establish |
| --- | --- | --- |
| Compilation reuse | Explicit HTTP lowering once and checked instance binding; independently retained specialized Action/notification definitions; scoped Producer HTTP response/validation reuse | General cross-family or cross-endpoint invariant native lowering; identical artifacts across incompatible environments |
| Logical instance independence | Independent Host Consumer authority/associations/capacities, Producer publication, permission and original results, with separately owned native infrastructure | Independence from Host storage and account structures |
| Executor independence | Manual poll/step and optional borrowed Host Consumer/Producer turns, with deployment clocks, maintenance and native wakes | Execution with a different work/result/identity representation |
| Runtime/storage representation independence | Shared finite kernel plus caller-owned NoSec Property Read with real datagrams, inline output and separate no-atomic compilation | Continuing Action, notification, Producer or general constrained execution |

Host Action definitions retain Box mappings, Any continuations, Arc data service
owners and Host account reservations in
[`core::consumer_action`](../../core/src/consumer_action.rs). Their module is
std/pointer-atomic gated. The actual
[Servient owner](../../servient/finite/src/consumer_action.rs) additionally uses
Rc authority/control identity, RefCell registry slots, Vec capacity and Waker
interest. Host `ConsumerInstance` uses Arc/AtomicBool identity/currentness;
`HostResourceAccounts` uses atomic used/peak counters, and `ConsumerServiceOwner`
retains Arc services and their original bank funding. The
[finite Host owner](../../servient/finite/src/consumer.rs) likewise retains
Rc/RefCell work/control/association state and Arc definitions/accounts despite
using the common finite kernel. Host integration turns preserve these owners; manual driving
does not remove those requirements. Caller-owned Read previously demonstrated a
different representation only for its finite lifetime.

## Selected correction

The finite kernel concludes one call at native settlement. Reusing its lifecycle
for Action acceptance would conflate exchange and invocation lifetime. A caller
application loop around the Host Action would keep the same representation
dependency. Notifications and Producers remain useful successors but introduce
publication/distribution responsibilities beyond this specific counterexample.

The existing Host Action transition algorithm now lives in
[`execution::action::step_action`](../../core/src/execution/action.rs), and both
the actual Host `ConsumerActions` owner and
[`borrowed_action::ActionSlot`](../../servient/finite/src/borrowed_action.rs)
execute it. `ActionProgress` holds only per-invocation semantic state/errors.
There is no shared mutable work/result container, universal Runtime or scheduler.
Each representation independently owns native exchange, continuation, output,
services, authority, input and funding. Core also supplies one borrowed Action
schema meaning, one output-shape check, shared `ConsumerEffect` vocabulary and
`ValueValidator`; finite Read now uses that same value validator contract.

The new typed native facet is supplied by a downstream Cargo-linked
[test datagram Binding](../../tools/architecture-fixtures/constrained-consumer/src/action.rs).
Its production fixture code uses concrete native exchange/token types, Cell
instrumentation and inline output, with no std/reference counts/atomics/lock.
Host tests supply actual UDP I/O to that same code. This requires no conversion
from the Host Box/Any/Arc artifacts. Each native representation qualifies its
actual configuration, supported meaning, continuation compatibility and original
backing. Sharing schema or transitions proves no native
compilation-work reuse.

## Owner-enforced backing and bounded turns

The replacement keeps one Action kernel and gives native polling, retirement
and semantic settlement separate turns. An acknowledged close returns the
exchange to its Binding while the invocation stays pending or its result
remains unread. Terminal settlement releases the continuation. A retained
result keeps its input, actual returning destination and result backing.

The [caller-owned datagram suite](../../tools/architecture-fixtures/constrained-consumer/src/action/tests.rs)
uses one exclusive typed result slot per admission. A held result excludes
reuse of that cell while an independent cell remains usable. The Binding,
validators and access services are supplied at installation of the prepared
program. Siblings share a drive cell. An access callback attempting to poll a
sibling gets a readiness lifecycle error before observing its clock, and
passive cancellation still fences the sibling's next turn. The peer receives
one datagram from the outer turn.

The [Host callback suite](../../servient/finite/tests/consumer_action_callbacks.rs)
exercises original first cause, a failed and blocked close, terminal token
disposal under the Host execution turn, and the original output after
callback unwind. The [independent Host datagram fixture](../../tools/architecture-fixtures/open-consumer/tests/action.rs)
uses different Invoke and control Bindings, and validates Query output in its
own media before delivery. The [real HTTP Action suite](../../protocol-bindings/protocols/http/tests/action_consumer.rs)
exercises the last response-buffer quota with a held result. A later Invoke
and Cancel proceed while a Query that needs that quota is refused before I/O.
The Query succeeds when the original value is disposed.

The [HTTP allocation fixture](../../protocol-bindings/protocols/http/tests/allocation_trace.rs)
withholds the fallible Query output-buffer allocation. The original
invocation and expanded control remain usable. On retry, a 514-byte JSON
string with a length constraint can be validated and delivered with shared
pools exhausted. The validation and handoff turns make no allocation request
of 512 bytes or larger. A finite Read sharing the last outstanding-call unit
is refused while the Action settlement is held and succeeds on the same
public path after disposal.

Native poll failure, failed close, late output and first cause are independent
records. When a fence is already active, retirement keeps raw output without
entering the access or data service. The typed result loan checks current
permission and validates the exact bytes, media, coding, shape and metadata
it returns. Changing a native selector after validation can affect a later
view, while a previously lent view retains the original evidence. The input
adapter follows the same exact-view rule before Invoke. These checks use real
UDP requests alongside safe interior-mutability test adapters.

Reproduce from the repository root.

```sh
cargo test --locked -p clinkz-wot-constrained-consumer-fixture --features host-meaning,prepare
cargo check --locked -p clinkz-wot-constrained-consumer-fixture --target thumbv6m-none-eabi
cargo test --locked -p clinkz-wot-servient-finite --no-default-features --doc
cargo test --locked -p clinkz-wot-servient-finite --test consumer_action_callbacks
cargo test --locked -p clinkz-wot-binding-http --test action_consumer --test allocation_trace
cargo test --locked --manifest-path tools/architecture-fixtures/open-consumer/Cargo.toml --test action
```

No-pointer-atomics compilation verifies that the typed execution path builds
without the Host representation. Hardware and hard resource bounds require
target-specific evidence. Exact checked revisions belong to the PR.

## Remaining 0081 and runtime-independence scope

0081's definition/instance/authority decision remains active. Broader useful
native lowering reuse still needs concrete complete dependency/operand contracts,
changed-dependency refusal, independent authority/funding and actual compilation
cost distinction from admission or specialization. Action/notification definitions
remain endpoint-specialized. Further Producer families/Bindings and optional
cache/persistence need their own evidence; none requires a universal program or
identical native artifact format across profiles. This increment adds execution
representation independence, not another compilation reuse claim.

Runtime/storage independence remains incomplete beyond the scoped caller-owned
Read and Action paths. Modern constrained TD preparation/registration, security
credentials, notifications/response controls/loss, Producer publication and
behavior/result ownership, concrete embedded I/O, wake integration and physical
platform qualification remain open. General structured schema storage still uses
the existing alloc representation; arbitrary caller-provided schema arenas are
not supplied. The local slot is one invocation opportunity, not the Host managed
registry, shared multi-invocation capacity or an independently awaited result
wake contract. Host owners remain legitimate implementations for their profiles.
No package, gate, milestone, runtime-independence or release closure is claimed.
