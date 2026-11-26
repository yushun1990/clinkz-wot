# Managed Host Consumer execution composition

This increment establishes optional execution composition for the supported Host
Consumer subsets. It does not claim independent acceptance, package/milestone
closure or production qualification. The exact contract is
[Runtime Safety](../spec/runtime-safety.md#host-consumer-execution-turns);
[topic 0125](../../workspace/0125-consumer-execution-composition.md) records the
problem boundary and rejected alternatives.

## Existing mixed evidence and the missing boundary

The [collection coexistence fixture](../../protocol-bindings/protocols/mqtt/tests/collection_coexistence.rs)
already qualifies one Producer with HTTP Action and MQTT Event collection Forms,
keeps same-named Action/Event schemas distinct, and proves independent lifecycle
and funding through actual I/O. Its Consumer combines a finite QueryAllActions
call with manually selected notification progress, under a raw Consumer authority
and a test notification adapter. It does not execute continuing Consumer Actions
or managed mixed-family removal/replacement, and its separate polling loops are
not an application execution-composition boundary.

Finite and continuing Action capacity owners already aggregate their retained
work. Notification capacity needed a corresponding turn for active conversations
and abandoned cleanup. Its existing slab, validation/policy and native retirement
transitions suffice. The deployment now calls each capacity directly. It can
omit a finite turn while continuing Action and notification work progresses.
Definitions, original associations, live work and physical resources keep their
established owners; Review 10 F1/F2/F3 are preserved, not reinterpreted.

## Executable managed HTTP/MQTT composition

The [mixed Consumer fixture](../../protocol-bindings/protocols/mqtt/tests/mixed_consumer.rs)
uses actual HTTP finite/Action Consumer Bindings, a scripted independent TCP peer
and a real Mosquitto process. The peer is protocol/lifecycle evidence, not a new
qualified Engine Producer. TD-driven preparation is discarded before execution;
original prepared definitions are bound to separate managed Consumers with
independent instance-scoped access services. A capacity has no Thing cardinality
relationship: eight managed identities coexist with one finite call slot, one
Action invocation slot and two notification conversation slots.

One Host task supplies native Wakers, a real maintenance timer, monotonic time
and independent turns. It collects finite results in that same task after
passively observing native settlement. The fixture exercises the following cases.

- Refused finite permission on one Consumer leaves the sibling finite call,
  continuing Action and MQTT subscriptions usable. Omitting the finite turn
  still permits Action acceptance and MQTT delivery. Driving the finite capacity
  completes its original call.
- With one outstanding unit shared by finite and continuing calls, holding either
  taken result refuses the other family. Disposal permits its next admission.
- A finite Read and continuing Action each try to drive the other handle from
  an authorization callback. Both resume under independent turns and return
  their original HTTP results.
- Invoke/Query complete with original Consumer identity and validated output.
  Explicit mapped Cancel reports Requested while the Action remains pending;
  a later explicit Query completes. Local cancellation wakes the registered Host
  driver and sends no hidden remote CancelAction.
- Stopping one notification preserves its live cursor's report and the sibling's
  actual shared MQTT connection. Queued data and another delivery remain usable
  by the independently authorized sibling.
- Managed replacement is a passive fence. Old finite/Action native completions
  retained before handoff remain with their original identity and funding, while
  successful application handoff is refused. A successor binds the same actual
  Action/notification definitions and cannot acquire the original continuation.
  Old notification data retires without reaching the successor; the sibling
  retains its own queued value and physical connection.
- Abandonment reclaims a silent finite socket, continuing Action and MQTT cursor
  through capacity turns without retaining their application handles/identities.
  A surviving stop view does not pin a reclaimed conversation. The sibling remains
  independently usable. A delayed owner-Drop command during otherwise silent I/O
  fences original work without a Waker notification; supplied maintenance turns
  complete original notification retirement.
- Actual timer turns observe finite and continuing Action deadlines while a
  finite peer is silent and an Action is remotely pending. Original first causes
  are TimedOut, no remote Cancel is invented, and MQTT continues delivering.

The HTTP peer observes client socket retirement, MQTT reports actual connection
release after its final participant, and every Engine account reaches zero after
all original results/associations/definitions are disposed. Returned settlements
deliberately retain original funding beyond controller loss. These are lifecycle
and reservation assertions, not process-RAM, performance or arbitrary SDK claims.

## Notification aggregation and retained regression evidence

The [notification owner tests](../../servient/finite/tests/notification.rs) add
three aggregate-turn counterexamples: rotating one-poll credit between a stalled
and healthy conversation; blocked/failed reply retirement after cursor abandonment
while a stop view survives; and zero-credit observation of passive parent loss.
Sibling delivery continues under failed cleanup. Live cursors keep their terminal
reports until explicit handoff. No response is fabricated, and prepaid backing
cannot be released before complete original response/participant retirement.

The downstream observation Binding attempts a finite Read from its native
receive callback under both aggregate and per-conversation progress. The test
checks the UDP peer after the callback and receives the request only after an
independent finite turn. [Downstream composition fixture](../../tools/architecture-fixtures/open-consumer/tests/support/composition.rs)

Aggregate driving intentionally discards a fully retired abandoned cursor's report.
Keep the cursor for a required report, or use the existing manual cleanup/report
API when collecting reports after cursor loss. Existing per-handle/per-conversation
polling remains supported. Existing Action callback/late-output/retirement tests,
notification occurrence/response/loss tests and original F1/F2/F3 service/declaration
regressions remain applicable; the helper delegates those original transitions.
The new HTTP replacement test preserves unread original native output; it does
not claim that the stock HTTP mapping can retrieve output after dropping its
socket during local close.

The [runnable mixed Consumer example](../../protocol-bindings/protocols/mqtt/examples/mixed_consumer.rs)
prepares the three actual capabilities, registers the TD ID, explicitly binds
continuing associations and supplies one Tokio loop with a maintenance pulse.
It is a deployment example; the Engine acquires no Tokio dependency. Application
Query cadence, result handling and shutdown policy remain explicit. After the
ten-second application window it fences managed acceptance, abandons remaining
handles and continues composed cleanup until the capacities are idle.

Scoped validation includes the Servient-finite, HTTP and MQTT/broker suites,
the downstream open-Consumer fixture, design projections, applicable no-default
checks, and compilation/execution of the example. Commands and exact checked
revisions belong in the PR. Required GitHub validation establishes diff hygiene
only; it does not certify these runtime claims.

## Remaining boundaries

This implements mixed **Consumer** integration. Reusable qualified Producer
exposures, complete final-TD publication reuse and general Producer execution
composition need their own evidence. The prior qualified mixed Producer fixture
retains its existing scope.

The new mixed path is bounded Host NoSec/JSON with IP-literal HTTP and the existing
loopback MQTT subset. Separate finite HTTPS/security evidence does not qualify
authenticated mixed Actions/notifications. MQTT Event attribution is a TD label,
not verified publisher/source provenance; proof verification, freshness and trust
dependency retention remain open. No embedded continuing executor, hardware
transport, fleet-scale performance or broader profile qualification is established.

The deployment must continue supplying native and maintenance turns, clocks and
credit. No universal Runtime, scheduler, per-Thing executor, background drain,
automatic semantic Query/Cancel, broadcast result wake API or deadline timer is
created. Application result observation belongs in the same driving loop; separate
async waiter tasks need explicit coordination under the retained manual contract.
Idle capacities exclude separately retained settlements and physical owners.
Cross-endpoint compilation reuse, automatic caching and a general all-family
configuration registry are not required or introduced by this increment.
