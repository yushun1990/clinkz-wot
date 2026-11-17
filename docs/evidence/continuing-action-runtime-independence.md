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
actual configuration, supported meaning, continuation compatibility and complete
work/backing declarations. Sharing schema or transitions proves no native
compilation-work reuse.

## Executable boundary

The [caller-owned tests](../../tools/architecture-fixtures/constrained-consumer/src/action/tests.rs)
exercise:

- Invoke acceptance, explicit Pending Query, remote Cancel acknowledgement and
  later Query completion, including different Invoke/Query media and retired
  input-codec availability;
- independently authorized equal numeric slots sharing the same actual native
  definition, foreign/reused handle refusal and retained original results while
  a sibling completes;
- original input, opaque token, native output and exact lease ownership through
  unread settlement, disposal and slot reuse; all created native socket ports
  become bindable after their actual owners retire;
- one prepaid input view reused across authorization, validation and native
  start: policy-time selection of `99` cannot replace the validated `7` on the
  actual wire, input-view-time revocation fences an immediate-send Binding,
  and short admission credit obtains no input view or native effect;
- paid output loans over a safe-Rust Cell-selected native output: authorization
  after validation or at owner handoff can select invalid `99`, media/coding,
  status or response metadata, and each exact application loan refuses it;
  changing a selector after a successful loan cannot change that loan's checked
  bytes or metadata, while another loan must validate its own view;
- loan work separate from owner transfer: short view/codec credit retains all
  owners without callbacks or a sticky failure, unavailable codecs are not
  re-entered, invalid loans keep native evidence and first cause, and disposal
  returns original funding without native re-poll or new permission;
- cancellation before initial native progress and after Query starts, issuer
  revocation, Pending/failed physical close, retained late output and validation
  using original funding without effectful polling after local fencing;
- native poll error independently retained alongside late output and repeated
  cleanup errors; remote Cancel failure retains uncertainty, with no rollback
  claim and the same conservative Unknown effect as Host Actions;
- output shape, media, coding and DataSchema refusal with original output
  retention; unavailable original Query service is never re-entered;
- short callback/handoff credit, unchanged input/lease on refused admission,
  no automatic Query/Cancel, unused control declaration drift and pre-effect
  control refusal preserving unrelated Pending invocation authority;
- fresh paid unread-result permission, policy rotation within callbacks,
  output-validator-time revocation, deadlines/clock regression, and finite local
  Drop without a false close acknowledgement;
- an optional loan of the exact schemas inside a modern Host Action content
  owner to the different native representation, without schema recompilation.

The settlement compile-fail cases reject a result slice escaping its owner and
moving that owner into funding return while the result loan is still used.
Host callback regressions and the existing downstream/real HTTP Action suites
execute the extracted kernel under their original reentrant authority, wake,
service-funding, unread-result, late-output and physical-release obligations.
They are retained evidence, not replaced by matching synthetic traces.

Reproduce from the repository root:

```sh
cargo test --locked -p clinkz-wot-constrained-consumer-fixture
cargo test --locked -p clinkz-wot-constrained-consumer-fixture --features host-meaning
cargo check --locked -p clinkz-wot-constrained-consumer-fixture --target thumbv6m-none-eabi
cargo test --locked -p clinkz-wot-servient-finite --no-default-features --doc
cargo test --locked -p clinkz-wot-core -p clinkz-wot-planning -p clinkz-wot-servient-finite -p clinkz-wot-servient -p clinkz-wot-binding-http
cargo test --locked --manifest-path tools/architecture-fixtures/open-consumer/Cargo.toml
bash tools/check-design-artifacts.sh
```

No-atomic compilation establishes buildability of these contracts and fixture
owners, while hosted real datagrams establish execution and ownership behavior.
Neither establishes executable embedded hardware/SDK/allocator/real-time support.
Exact checked revisions and validation results belong to the PR.

## Output validation and owner handoff

At `fb8dee594f2df595cadcf01a8ad17317dad53f0c`, a conforming safe-Rust native output
could use `Cell` to select immutable backing. The real Query response supplied
`7`, which the original codec validated against maximum 50; a subsequent access
callback selected `99`, and the old unpaid `result()` exposed it successfully.
Retaining the same output owner did not establish payload/media/status/metadata
identity. This was a real application-validation gap.

The corrected `result(budget)` validates one captured view and returns that
same loan. Each successful call certifies its own bytes and metadata, without
promising immutable replay of every prior view. It uses the original returning
operation's schema/codec/bounds and charges its adapter and validation work.
Progress validation remains necessary for normal/late evidence. Moving the owned
settlement remains the permission/ownership handoff; obtaining an application
loan does no fresh authorization or native work. No payload or metadata-history
copy, self-reference, scheduler or shared execution storage is introduced.

The input selection regression additionally verifies the distinction between
the original input owner and a dispatched byte receipt: the peer receives `I7`
while `settlement.input()` later lends that owner's current `99` selection.

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
