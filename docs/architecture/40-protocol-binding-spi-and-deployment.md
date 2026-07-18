# Protocol Binding SPI and Deployment

## V1 integration model

A Protocol Binding is an ordinary Rust crate compiled into the application or
firmware through Cargo. It exports constructors for a complete registration
bundle. The application explicitly supplies that bundle to `ServientBuilder`.

Official umbrella features are convenience composition only. A third-party
binding can depend directly on the public engine crates and does not need to be
added to this workspace or umbrella crate.

`build.rs` is not a binding discovery or dependency mechanism. Cargo resolves
Rust dependencies before build scripts run; recursively invoking Cargo would
create incompatible type identities, features, targets, profiles, and lock
ownership.

## Complete registration bundle

One logical bundle atomically associates:

- a unique `BindingId` and binding generation;
- a bounded configuration identity;
- side-effect-free capabilities and compiler extension;
- optional deterministic Producer form contribution;
- client and/or server execution registration;
- route, call, subscription, ingress, response, status, and cleanup footprint
  declarations; and
- host or constrained adapter metadata.

The exact Rust schema belongs to the binding SPI specification. The
architecture forbids independently registering compiler and execution halves
whose ids, generations, or configuration differ.

Builder validation rejects duplicate ids, incompatible capability overlap,
missing execution halves, unsupported profile cells, and a footprint that
cannot fit the selected resource profile. Registration order is stable for
diagnostics but never resolves ambiguous exclusive route ownership.

## Planning SPI

The planning half declares coarse indexed capabilities, answers bounded
side-effect-free support queries, and compiles only a candidate already resolved
by the shared planner. It cannot access a Servient, handler, credential, socket,
or executor.

Compiler artifacts are scoped to the complete bundle generation. An execution
half rejects a plan reference from another generation or configuration.

## Client execution SPI

The engine supplies one owned `OutboundRequest` and an admitted call/slot owner.
The binding supplies protocol I/O progress and a terminal WoT-facing result. It
does not choose another form or invoke application behavior.

Host calls are owned cancellation-aware objects retained by Servient across
caller drop and late completion. Constrained calls use caller-owned typed slots
with the same settlement semantics. Constructors are nonblocking and do not
start a side effect until ownership and capacity have transferred.

## Server execution SPI

The v1 server SPI is engine-orchestrated and route-scoped:

- prepare/readiness/activate/commit/shutdown operate on one generation-bearing
  route and preserve guard ownership on failure;
- successful commit returns a distinct committed-closed guard and never opens
  request admission;
- each `poll_accept` consumes one claim that exclusively borrows the route's
  unique accept lease into a non-cloneable, route-scoped permit for the
  currently serving produced generation;
- each serving route has one poll/waker lease for inbound acceptance;
- request, operational-error, and terminal events identify their route;
- route terminal state does not ambiguously terminate an entire registration;
- response delivery is bounded and retains the response opportunity on
  pre-acceptance backpressure; and
- readiness and cleanup are poll/step driven with explicit work, wake, deadline,
  footprint, and transfer rules.

A binding does not receive an application `Dispatch` capability. A host binding
may run a bounded protocol reactor that wakes route/call drivers, but it cannot
detach ownership, bypass engine admission, or call handlers directly.

The complete registration declares whether preparation is externally hidden or
visible. A visible route declares how closed-gate input is rejected,
backpressured, or buffered within admitted ingress limits. It cannot emit an
inbound request or report application acceptance before publication. A binding
that cannot enforce permit-gated acceptance is rejected.

The plan set, produced registry generation, and immutable serving activation
authority are published by one Servient transition after every required route
is committed-closed. The authority remains inside a private mutable Servient
record and is not passed to a binding. Servient validates that record, moves
the unique route lease into one claimed-call owner, and consumes the claim into
a short-lived permit for one generation-checked accept call. Drain stops new
claims before route shutdown. There is no per-route gate-opening callback and
no binding observation of registry state.

## Subscription and emission SPI

A successful subscription start transfers one pull-capable driver or typed
static slot. Its worst-case lifetime footprint is declared before start and its
actual immutable upper bound is verified before application publication.

Producer emission supplies one selected publication target at a time. Protocol
remote fan-out stays inside the binding; cross-binding scheduling and aggregate
results stay in Servient.

## Memory, cancellation, and cleanup

Every long-lived binding-owned object declares an immutable worst-case exclusive
retained footprint for its entire lifetime, including cancellation. Poll-time
temporary allocations and transport-library buffers are separately bounded and
charged. A footprint cannot shrink merely because an initial poll completed.

External-input growth is covered by explicit per-route/per-binding/global item
and byte limits. A binding cannot hide an unbounded pre-dispatch queue behind
`poll_accept`.

Cancellation uses an engine-created reservation carrying the complete bounded
identity seed. A phase-specific context binds the operation and independent
drain deadline at cancellation linearization. Start cancellation, active
subscription stop, route readiness cancellation, response cancellation, and
route cleanup use distinct operations. Pending cleanup is an ownership transfer
of the complete call/guard/driver, not only a status record.

## Host and constrained representations

Host builds may erase concrete types behind owned trait objects. Constrained
builds use associated state types, caller-owned slots, static tables, and
manual progress. They share request, result, identity, cancellation, cleanup,
and terminal semantics; they do not have to share allocation or executor
representation.

Both representations use the same permit-gated acceptance rule. A host runtime
pins the immutable serving generation while calling outside locks. A
constrained runtime claims a caller-owned route slot in a brief critical
section and passes a scoped permit to manual progress afterward. The permit
contract requires neither a shared atomic flag nor a host registry handle.

## Packaging and rollout

V1 rollout is process, container, or firmware replacement:

1. build an application containing the new binding crate/configuration;
2. start a new Servient instance and complete route readiness;
3. switch traffic or application ownership;
4. drain and explicitly shut down the old instance; and
5. roll back by restoring the prior binary/configuration when necessary.

Cargo features select compiled capabilities; they are not runtime rollout
switches. Compiling multiple bindings into one binary allows startup selection,
not safe code unload.

## Dynamic loading decision

Rust trait objects, `Arc`, `Box`, futures, wakers, enums, allocators, and panic
behavior do not form a stable cross-dynamic-library ABI. V1 does not pass core
Rust SPI objects across a `dylib` boundary and does not promise hot unload.

A future host-only plugin design requires a separately versioned stable ABI or
an out-of-process adapter. It must define ABI negotiation, allocation/free,
panic isolation, capability manifests, polling/cancellation, resource limits,
and unload only after every route, call, subscription, and cleanup owner is
terminal. That work is not inferred from the Rust crate SPI.

## Extension evidence

The conformance suite includes a fake third-party binding crate outside the
workspace member list. It must compile against public crates, construct a
complete registration, support consume/expose, and pass cancellation, cleanup,
generation, resource, and hot-path tests without modifying engine or umbrella
source.
