# Host execution across independent Consumer and Producer capabilities

Scoped Host integration evidence, not independent acceptance, architecture or
milestone closure, production qualification or a performance claim. The
[investigation](../../workspace/0129-host-producer-execution.md) records the
counterexample and alternatives; [Runtime Safety](../spec/runtime-safety.md#host-producer-execution-turns)
owns the detailed contract.

## Architectural assessment

| Concern | Finding and disposition |
| --- | --- |
| Independently published Things | Existing complete final-TD qualification, independent authority, publication/replacement and exact native membership suffice; no new Thing executor or publication hierarchy is required. |
| Consumer/Producer coexistence | Existing capacities and native owners can run in one supplied loop. The missing end-to-end connection was integration evidence; sharing one global work owner or protocol router is unnecessary. |
| Producer execution after storage erasure | A genuine API gap: retaining publication metadata and `Any` forced applications to recover the typed inventory and repeat Servient orchestration. Retain the typed Servient operation with its native member instead. |
| Revoked continuing Action cleanup | A genuine aggregate execution gap: normal progress requests cancellation but cannot finish a job requiring local close without application handle enumeration. The opt-in turn now funds and closes only this participant's revoked original work. |
| Closed native Action polls | HTTP returned Unsupported during legitimate drain. Closed polling now supplies maintenance/no candidate while logical work and unread results retain their separate owners. |
| Result observation and decisions | Query/cancel cadence, emission, response, retention, acknowledgement and reaping are application policy. A turn cannot safely invent them or discard their evidence. |
| Native topology and reusable preparation | Existing explicit HTTP instruction reuse, shared listener and separate finite capacity suffice. Definition/instance/work lifetimes remain separate; no universal compiled program, shared result registry or scheduler is needed. |

`OwnedProducerParticipation` is replaced directly with an owned native member
and its typed Servient turn. Its family constructors invoke the existing public
execution capabilities in Servient. The custom bounded turn seam supports linked
capabilities; no native callback gains a dispatcher or logical registry. Existing
callers migrate to the appropriate constructor. This adds one operation pointer
inside the existing box, identity/error storage in the fixed member vector and
their actual admission charges, without a second native owner or work registry.

Each `ComposedProducerExposure::step` checks original membership and supplies
independent participant/lane credit. It visits siblings after short work/errors
and reconciles native evidence at Servient. Native Action ingress cannot consume
the continuation/cleanup allowance. Errors and Pending cleanup stay observable;
no turn automatically publishes, acknowledges, reaps, emits or returns a complete
drain claim. The existing exact `retire` boundary remains binding. Low-level
manual operations remain useful independently of the opt-in aggregate policy.

## Real execution and adversarial evidence

The [joined Host case](../../protocol-bindings/protocols/mqtt/tests/support/host_execution.rs)
uses actual Engine Consumers and fully qualified Engine Producers with stock
HTTP TCP and an independent Mosquitto process. Its deployment supplies cyclic
Tokio maintenance turns; the Engine acquires no Tokio dependency.

- Two independently qualified/published Things share one Binding-owned HTTP
  listener and one finite capacity with one active and eight retained credits.
  The first advertises HTTP Property Read, continuing HTTP Action invoke/query/
  cancel and an MQTT Event; the second advertises its distinct finite Property.
  Entire final descriptions are prepared and checked without dropping Forms.
- One funded HTTP instruction compilation is checked and rebound for the second
  Thing and a successor. Their names/endpoints/behaviors/authority/funding remain
  independent. Final qualification/admission do not recompile. Zero-credit setup
  cannot acquire readiness or dispatch.
- Existing aggregate Consumer turns drive finite Read, a continuing Action and
  an Event subscription alongside the new aggregate Producer turns. A separate
  TCP peer reads the second Thing in the same loop. Original Producer results
  remain unread while later work proceeds; fresh policy denies another request
  before effects without closing the sibling. The original capacity sequences,
  recipients, effect/delivery and borrowed output remain distinct.
- A qualified original Producer emission reaches its subscribed Engine Consumer
  through Mosquitto. Local publication and observed delivery are separate facts.
  The occurrence has TD coordinates, not authenticated publisher provenance.
- With zero native ingress credit, original Action work still completes using
  its independent allowance. An explicit Consumer Query retrieves the validated
  completion; Producer and Consumer result acknowledgements remain separate.
- A second original Action stays Pending. Closing its publication triggers funded
  local close without an application cleanup-handle inventory. One close error,
  repeated Pending and a zero-credit turn retain the original owner/first close
  failure. Cancellation is requested once; no automatic remote CancelAction is
  sent. Unfinished original work prevents retirement.
- A staged successor publishes and serves its own value while the old Action
  still cannot close. Its original native MQTT lease is staged before the old
  last lease retires: the existing service deliberately cannot reopen after its
  final physical shutdown. This is explicit native setup/cutover integration,
  not a new reconnection or sharing abstraction. Original cleanup and results
  never acquire successor authority or change its behavior.
- Releasing the original close permits explicit Action acknowledgement and
  retirement; successor ownership keeps the shared native session alive.
  Consumer owner close/abandonment, Producer close and explicit acknowledgements
  drain the remaining owners. Both MQTT services disconnect. Actual HTTP ports
  can be rebound after native root disposal. Original Consumer/Producer results
  still retain their publication/work/service funding and valid original bytes;
  all accounts in all six banks are free after final result disposal. Immutable
  compilation backing releases independently when its own last user disappears.

Two [focused real-HTTP cases](../../protocol-bindings/protocols/http/tests/composed_thing/execution.rs)
challenge the aggregate boundary. A busy original listener records startup
failure while the next member still installs closed; no partial publication
occurs. Replacing a typed loan with another same-owner/key/configuration runtime
cannot pass admission or give that new identity progress, while the valid sibling completes actual
TCP I/O in the same error-returning turns. Restoring the original member permits
complete retirement; the first turn error remains observable. Final listeners
and original account reservations release.

Existing native authority, publication qualification, finite work/retained result,
collection semantics, continuing callback/late-result, security and service
declaration regressions retain their prior scoped oracles. Qualification-only
linked fixtures supply explicit custom turns; their manual native tests retain
their original evidence scope and do not become aggregate execution claims.
The [Property server](../../protocol-bindings/protocols/http/examples/serve_properties.rs),
[Action server](../../protocol-bindings/protocols/http/examples/serve_action.rs) and
[Event emitter](../../protocol-bindings/protocols/mqtt/examples/emit_event.rs)
now use the same aggregate Producer operation. Action cleanup enumerates only
results to acknowledge, rather than implementing per-invocation close/progress.

Focused commands (Mosquitto 2.x on PATH or `MOSQUITTO_BIN`):

```sh
cargo test --locked -p clinkz-wot-binding-mqtt --features broker-tests --test mixed_consumer host_turns_join
cargo test --locked -p clinkz-wot-binding-http --test composed_thing managed::execution
```

The PR records exact checked revisions and broader package, authority, feature
and example commands. Required GitHub `validation` proves diff hygiene only.

## Remaining architecture and product work

This supplies a usable bounded Host composition for the exercised capabilities,
not a finished v1 application API or all-family runtime. Concrete native setup
and prepared-definition/instance assembly remain verbose integration work.
Further constructors/examples can improve that without changing semantic or
native ownership. Independently awaited result tasks still need a qualified wake
contract; the current single driving task/cyclic loop is usable. Logical deadlines
install no timer, per-member budgets promise no general scheduling fairness and
each deployment remains responsible for turns and result disposal.

Shared continuing Producer work capacity and broader Action/notification compiler
reuse need separate representation/evidence work; the finite capacity/HTTP reuse
claims cannot be inherited by them. Mixed authentication/HTTPS Actions and
notifications, evidence-bearing notification source verification, broader schema/
media support and shared secure infrastructure require real capability/security
work, not more NoSec examples. Client Discovery-to-execution usability remains
separate from the existing bounded retrieval capability. Constrained continuing
execution, supported-profile conformance/resource/performance, Host HTTP/HTTPS
production delivery and independent v1 qualification remain open. Nothing here
establishes remote receipt, rollback, exactly-once effects or fleet-scale behavior.
