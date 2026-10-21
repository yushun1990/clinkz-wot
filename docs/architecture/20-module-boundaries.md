# Module Boundaries

Status: active v5.1 authority.

## Target dependency direction

```text
Core execution        -> Foundation
TD                    -> Core, Foundation
Planning              -> TD, Core, Foundation
Binding runtime       -> Core, native SDK
Binding / prepare     -> Binding runtime, Planning, TD
Servient execution    -> Core
Servient / prepare    -> execution, Planning, TD
application           -> selected capabilities, Discovery, codecs, platform
```

`CRATE-DEPS-001`: The graph MUST be acyclic. Core MUST have no normal or optional
TD/Planning/Discovery/concrete-Binding dependency or upward re-export. TD owns
its document encoding of Core operation vocabulary. Planning owns compiler
contracts, selected inputs, pure jobs and compiler erasure; TD owns the resolved
semantic trees and paid loans they borrow. Runtime allocation, artifact storage,
format compatibility, executor registration and result checks remain in Core.
Native executable types MUST be defined by the Binding runtime, not Planning.
Optional preparation MUST produce those same native types and capture/check the
actual executor policy before effects. No concrete Binding becomes a Servient
dependency. Only application/umbrella composition selects all capabilities.

[ADR-0024](../ADRs/0024-preparation-and-execution-boundary.org) records the change.
The first production extraction establishes Core -> Foundation, TD -> Core,
Planning compiler ownership and HTTP's optional `prepare` feature. The current
Servient package still composes preparation and execution: its Consumer retained
output/rollback types and Producer build state name Planning. A future execution-
only Servient feature must first separate those physical output/reclaim owners;
a feature alias cannot remove that dependency. This limitation does not license
an upward Core edge or claim the full target graph is already extracted.

The old `clinkz-wot-protocol-bindings` utility crate and TD-at-call-time
ClientBinding/ServerBinding/Dispatch facade are removed. Deferred Zenoh native
transport and preparation helpers retain their scoped evidence, without a
legacy dispatch entry. The umbrella names Planning directly. Servient does not
construct an implicit Directory or supply a legacy fallback.

Host/static storage, std, async syntax, thread mobility, pointer atomics and
resource/disposal policy remain distinct capabilities. No Contracts crate,
platform-wide allocator/reactor or split runtime packages are selected.

<a id="proposed-preparationexecution-separation-review-08"></a>
## Core internal boundaries

Core separates at least:

- `identity` and generation-bearing references;
- `error`, retry, cleanup, progress, and status values;
- `handler` context, cancellation, traits, and portable slots;
- `security` and `codec` semantic contracts;
- immutable `plan` values and source identity;
- `binding/client`, `binding/server`, `binding/subscription`,
  `binding/emission`, and registration values; and
- local protocol-neutral dispatch semantics.

Core does not use a catch-all event module for queues, merge policy, binding
publication, and application streams.

## Planning boundaries

Planning receives a validated document view, immutable policy, resource budget,
and checked preparation registration snapshot. It may call side-effect-free
capability and compiler-extension methods. It returns values and admitted
footprints; it never opens a route, sends a request, or retains a Servient
handle.

Logical plans share protocol-neutral work across candidates. Native programs contain protocol-specific instructions and may be shared under
explicit ownership and accounting; instance entries retain current authority and
varying operands. A binding compiler
does not receive authority to reinterpret W3C defaults, choose a different
operation, or access credentials.

For the first Consumer Property Read aggregate, TD owns bounded complete typed
inspection/Basic and an opaque `ValidatedThing<'td>` external loan. Its movable
semantic cursor lends one paid Property/Form coordinate until acknowledged;
current derived URI state stays TD-owned and every event loan is short. Shared
Basic/default/security/URI meaning never moves into Planning. There is no mandatory
complete normalized TD or unrestricted synchronous semantic view.

Planning owns complete preflight, owned required facts/lookup/candidates, every
compiler bound before start, sequential eager compilation, exact output footprint
and a source-independent Frozen draft. It retains no loan/pointer into scratch
across Pending and consumes input-bearing state before returning the draft.
Concrete artifacts satisfy owned-lifetime and cleanup eligibility, beyond a generic
wrapper's appearance. The completed exact-coordinate leaf remains narrower
regression evidence; its raw `PlanBuildInput<&Thing>` is not an aggregate trust entry.

The explicit TD `validated-thing` capability/AP boundary remains as specified by
[the TD admission record](../work-packages/WP-100-consumer-validated-thing-admission.md).
Ordinary typed APIs stay available without it. Strict literal JSON and shared field/
RFC3339 decoding are a separate future TD ingestion capability under ADR-0020;
its eventual controlled backend lends the same closed semantic program.

## Servient boundaries

Servient owns the transaction that composes modules. In particular it owns:

- the immutable binding registration snapshot;
- plan-set publication and retirement;
- generation-safe produced/consumed registries;
- callback leases and lock-free callback invocation;
- cross-binding fairness and isolation;
- application-visible subscription and emission facades;
- cleanup reservation, progress, and durable in-instance status; and
- host/static policy selection.

It schedules binding SPI progress but does not implement protocol I/O.

In the first Consumer slice, Servient owns the transaction and immutable checked
policy/registration capture, child/parent capacity pairing, runtime reservation
between preflight and materialization, independent Thing-slot/plan-set generations,
cleanup transfer, final checks, permit/install, registration retention, leases/drain
and reclaim. Build scratch and all input/config/compiler loans are terminal before
final checks and permit. Actual capacity and overlap remain charged until physical
release; an upstream source keeps its own charge. Aggregate allowances are not
contiguous allocation observations.

The private permit closes cancellation under exclusive slot mutation authority.
It installs preallocated complete runtime material without callback, allocation,
yield, source query or fallible work. Published owns execution output, one complete
registration and lifecycle/resource records only. Servient does no Basic/default/
security/URI work, semantic recount, lookup rebuild or TD retention. Legacy by-value
consume/Arc<Thing> ergonomics cannot define the bounded borrowed facade.

## Binding boundaries

A concrete binding receives only compiled, selected values. It may retain
protocol-local buffers and reactor handles only within its declared and admitted
footprint. External-input queues use registration-declared item/byte limits and
the shared overflow contract.

A binding may use a protocol runtime internally to make I/O ready and wake an
engine-owned call or route driver. It may not detach semantic ownership, call
application handlers directly, or report terminal state only through logs.

## Discovery boundary

The engine-side Discovery crate is a client. Storage engines, Directory server
query execution, redaction services, replication, and service SLOs belong to
future service crates or deployments. No Servient default silently constructs a
Directory service.

## Feature boundary

Host and embedded are deployment environments, not mandatory complete products.
TD tooling, Consumer-only and Producer-only applications may construct only
their admitted capabilities. The accepted Consumer-only registration and manual
runtime already do this without a dummy server or Directory. Dynamic typed TD
admission, static typed execution and any future generated-image installation
are different capabilities; the last requires its own trust/validation evidence.
ADR-0022 keeps this graph and shared semantic owners. Contracts extraction, a
Host/embedded crate split and a universal allocator/reactor are not prerequisites
of the first HTTP binding. Runtime, HAL/RTOS, network and reset mechanisms remain
environment-specific dependencies with explicit lifetime and qualification.

TD, planning values, core semantics, constrained binding contracts, and static
Servient abstractions support `no_std + alloc`. Filesystem, sockets, threads,
processes, dynamic libraries, and executor ownership remain in concrete host
adapters or higher deployment layers.

Operation vocabulary is available without pointer atomics. The current Core
execution representations require pointer atomics and expose no runtime API on
other targets; TD tooling uses the vocabulary independently. Cortex-M0 TD checks
therefore do not establish no-atomic Core/Planning/Servient execution. The latter
remains a separate unresolved representation task; qualified static execution
continues to name its actual target (currently Cortex-M4 compilation evidence).

Planning's `validated-thing` capability selects the borrowed Consumer aggregate
and explicitly activates TD's matching capability. Generic compiler authoring,
resolved semantic inputs and registration pairing do not implicitly activate
that loan/precision surface. The default Planning product includes it; Servient
selects it explicitly. Compiler-only Binding/probe dependencies may omit it.
This preserves TD's independently checked capability and Serde feature boundary.
