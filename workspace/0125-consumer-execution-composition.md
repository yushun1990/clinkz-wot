# Managed Consumer execution composition

Status: MIGRATED

The existing `collection_coexistence` integration qualifies one Producer's HTTP
Actions/collection and MQTT Events, then manually drives an Event Consumer and a
finite QueryAllActions call. It uses a caller-owned Consumer authority and a test
notification wrapper. It proves independent native/semantic lifetimes and original
funding for those participants. It does not demonstrate a continuing Consumer
Action, managed Consumer removal/replacement or a common application progress loop.
The independently retained definitions, associations and capacities established
before this increment already supply the required lifetime separation. Neither
that split nor Review 10 F1/F2/F3 needs reopening to compose execution.

The concrete gap is asymmetric driving: finite and Action capacity owners can
visit admitted work without application handle inventories, while notification
applications must select each active cursor and separately arrange lost-cursor
cleanup. A native Waker also cannot notify passive owner loss or replenished work,
and a Ready native phase need not wake itself for its next phase. Treating these
manual APIs as self-scheduling Futures would hide missing progress obligations.

Selected change: add an aggregate notification turn using its existing bounded
slab/transitions, with separate passive abandonment intent, and a borrowed common
turn interface for existing Host capacity owners. Each supplied capacity receives
its own deployment-selected budget; none waits for another. This needs no new
work/result owner or execution state container. A tiny `ConsumerProgress` trait
allows heterogeneous capacity slices, including several notification Binding
types, without a protocol enum, fixed three-family bundle or task registry.
Applications may continue calling the original methods directly.

Live cursors preserve queued data and unread terminal reports. Aggregate driving
reclaims abandoned reports after original local retirement and paid disposal,
even when passive stop views survive. This deliberate opt-in mirrors finite/
Action handle abandonment. Applications requiring lost-cursor reports retain the
explicit cleanup/report path. Aggregate progress does not invent QueryAction,
CancelAction, response data or cancellation input; those remain application
decisions under original fresh policy.

Alternatives considered:

- Only add a mixed fixture or copy three manual loops into an example. This
  would leave active notification inventories and abandoned-work disposal in
  every application. The aggregate notification turn is the necessary improvement;
  the small borrowed helper is optional convenience over the resulting symmetric
  capacity APIs.
- Add a universal Runtime, per-Thing executor or managed all-family configuration
  registry. Existing original Consumer references already fence every family.
  Owning those capacities together would reintroduce the coupling just removed;
  a new registry has no demonstrated requirement here.
- Enumerate erased native jobs inside Servient. Bindings already own I/O, shared
  sessions, continuation and physical cleanup. A native task inventory adds a
  second owner and risks reversing the established authority boundary.
- Return a Future that depends only on native wakes, or self-wake on every short
  turn. The first can stall after passive lifecycle change, Ready phase or credit
  exhaustion; the second can spin indefinitely without useful work. Supply actual
  deployment maintenance pulses/command turns alongside native readiness instead.
- Add a Tokio scheduler, built-in timer thread or broadcast result subscription
  API. The existing single-task Host loop can observe application results after
  composed turns. Separate async result tasks would require their own qualified
  wake/ownership contract; that is not needed for this executable increment.

The accepted detailed owner is
[Runtime Safety](../docs/spec/runtime-safety.md#host-consumer-execution-turns),
under existing SAFE-AUTH/CANCEL/CLEANUP/DROP/CONCUR obligations. Module/SPI
navigation, API/state projections, WP-400/WP-700 and PLAN migrate that scope.
No new ADR, requirement, gate, dependency edge or milestone transition is needed.
[Scoped evidence](../docs/evidence/consumer-execution-composition.md) distinguishes
new managed Consumer integration from retained Producer evidence and unimplemented
security, source-provenance, Producer-reuse and embedded-profile capabilities.
