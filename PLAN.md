# ClinkZ-WoT roadmap

Design target: v6.0 semantic Engine over Binding-owned native runtimes, selected
by ADR-0025 and active on master. Architecture activation does not transfer
implementation acceptance or release qualification.

Release target remains v1 with both WoT roles and Property/Action/Event capability
families, described collections, security, client-only Discovery/Directory,
portable shared semantics and Host HTTP/HTTPS production delivery. Zenoh/pico and
real embedded protocol/hardware qualification remain separate future work.

## Delivery strategy: alternate semantic kernels and joined execution

The cleaned baseline retains narrow working Property Read paths and scoped
historical evidence. v6 replaces the universal Property Read-shaped SPI, eager
per-instance plan-set and route-scoped native model. Raw finite and Action kernels
(and independently reviewed notification work when accepted) provide semantic
execution primitives, not a general WoT operation. Joined whole-Thing Producer
composition now carries finite HTTP Property Read and continuing HTTP Actions
through preparation, closed setup, final qualification, local publication and
real I/O. That meaningful foothold has a deliberately restricted mapping,
schema and security envelope; it does not close the Producer families or v1.

The next engineering emphasis is **composition**: interleave semantic kernel
work with executable end-to-end paths, falsify interface/owner assumptions at
the seams, and correct wrong abstractions instead of accumulating independent
raw operation kernels. This is a priority rule, not an obligation to implement a
single protocol first or freeze a cross-family ordering.

### Capability horizons

| Horizon | Primary result | Completion evidence appropriate to that result |
| --- | --- | --- |
| A — Semantic ownership | Finite exchange, continuing Action, notification conversation and Producer publication retain distinct but composable controls and owners. | Executable operation/lifecycle cases, including stale identity, cancellation, late results, saturation and retained cleanup. Raw kernels alone make no TD or native I/O claim. |
| B — Joined Producer foothold | Whole-Thing composition supports finite Property Read and continuing Action mappings over actual native participants; broaden only with complete final support and retained ownership. | Public real-peer tests plus negative support, prepublication, cancellation, replacement and final-resource-release evidence, scoped to the implemented mappings. |
| C — Open Consumer and broader operations | Complete protocol-neutral Consumer operation and instance composition across distinct finite and notification lifecycles; continue Property/Action/Event and described collections in both roles. | Owned third-party preparation and multi-family finite I/O, shared logical authority across independent lifecycles, then TD-driven notifications. No family completes from a raw kernel or mock alone. |
| D — Interoperability and product qualification | Verify downstream third-party Binding composition, selected cross-protocol contrasts, Host HTTP/HTTPS and security, Discovery/Directory client, portable shared semantics and supported profiles. | Real native peers, advertised DataSchema/security/profile checks, scoped resource/performance measurements and client Discovery behavior. |
| E — v1 integration | Reconcile completed WP-400/500/600 capability claims through WP-700 without inheriting historical acceptance. | Independent conformance, security, integration and release evidence for the exact advertised v1 scope. |

Horizons express **recommended concentration, not sequential package gates**.
The immediate emphasis is **protocol-neutral Engine composition**, following the
open Consumer preparation/finite foothold. Challenge operation, schema-role and
logical-owner assumptions before extending a production protocol. Property Read
and explicitly synchronous Action invocation now exercise the finite preparation
boundary; finite and raw notification facets share one independent Consumer
authority while retaining distinct controls and cleanup. These are bounded Host
subsets, not completion of all capability families.

**HTTP SSE is deferred.** TD-driven notification selection, service validation and
control mappings remain valuable subsequent Engine work; production streaming
framing is not the immediate implementation priority. Independent test Bindings
and actual I/O may expose composition defects without introducing another
production protocol. Discovery and HTTPS/security retain their own dependencies.

### Joined Producer composition: retained boundary

Use the existing direct real-HTTP finite Producer implementation as an economical
integration participant where suitable, not as the universal SPI model or as a
mandatory protocol-specific predecessor. Another supported Binding is equally
valid if it better exposes an architectural defect. A deliberately limited TD is
permissible only if its **entire final retained description** is supported and
qualified; silently dropping unsupported Forms/operations is not a solution.

An integrated Producer claim requires the following observable composition:

1. TD interprets common WoT semantics, resolves extension vocabulary IRIs and
   preserves extension metadata; concrete Bindings interpret protocol
   vocabularies. Planning coordinates selection of **actual** registered
   Binding/configuration, codec, schema, security and behavior services and
   pure construction of owned source-independent execution facts.
2. Native preparation and closed installation retain real physical owners,
   acquire no application-dispatch authority, and allow endpoint-derived facts
   to be checked against the final TD.
3. Before Serving, the final retained TD, all effective operations/Forms,
   behaviors, services, collisions and required readiness pass complete
   qualification. A single local, non-fallible publication commit binds the
   final description and current logical authority; prepublication ingress
   cannot dispatch.
4. Real native ingress passes fresh recipient, policy, payload and capacity
   checks; Servient captures the actual behavior; validated result and native
   delivery retain original identity, effect certainty and owners.
5. Refusal, cancellation/timeout, replacement, late results and close exercise
   first-cause retention, bounded cleanup/acknowledgement and actual final
   physical release. A working HTTP exchange or raw local activation alone
   proves none of the missing publication obligations.

Broaden this foothold in independently useful changes. Prove each claimed
boundary with proportionate tests; do not invent a global all-protocol proof
gate, a universal runtime/session/RPC abstraction, or certification paperwork
for unimplemented combinations. If existing preparation or runtime APIs impede
the joined path, replace them rather than maintain compatibility facades.

### Continuing capability growth

Consumer growth follows demonstrated architectural gaps rather than a fixed
finite-then-SSE sequence. Operation-scoped preparation and shared instance
authority belong to [WP-200](docs/work-packages/WP-200-planning.md#consumer-operation-composition)
and [WP-400](docs/work-packages/WP-400-servient.md#joined-consumer-execution),
with separate native facets in [WP-300](docs/work-packages/WP-300-bindings.md#consumer-capability-composition).
Deferred production observation scope remains in
[WP-600](docs/work-packages/WP-600-protocol-bindings.md#td-driven-consumer-observation).

After composition has an executable foothold, extend one operation/lifecycle
at a time using its own semantic obligations. In particular, Action invocation
outlives initiating finite transport exchanges; notification establishment,
flow/loss, reciprocal Event responses and stop differ from finite calls.
Property observation has no fabricated initial value. Described collection
operations preserve native group semantics instead of implicit N-call lowering.
Bindings keep native matching, multicast/distribution, correlation, session
sharing, retries and physical infrastructure; the Engine owns independent
logical acceptance and application dispatch.

Introduce a materially different protocol interaction where it tests an
unproven boundary (for example shared broker-native subscription or multi-party
distribution), not to satisfy an arbitrary protocol count. Check third-party
Cargo-linked, startup-registered capabilities without a Core whitelist. Prepared
definitions may be reused under proved configuration compatibility; one Thing
never implies one physical connection, one native subscription or one plan-set.

Shared DataSchema/media validation, security selection and authorization,
resource/work ownership, Host/constrained support and diagnostics accompany
the operations that use them; they are not postponed until an all-family gate.
Client-only Discovery/Directory and Host HTTPS have separate deliverable paths.
Real embedded hardware/transport and Zenoh/pico production remain outside the
selected v1 release but inside the enduring product goals.

Exact package/capability dependencies remain in
[the work-package DAG](docs/work-packages/index.toml). The responsibility
contracts are [Planning](docs/spec/planning.md),
[Binding SPI](docs/spec/binding-spi.md), [Core](docs/spec/interaction-core.md) and
[Runtime safety](docs/spec/runtime-safety.md). This roadmap adds **no** new
architecture authority, API freeze, PR-specific continuation state, mandatory
review gate or milestone status transition.

## Milestones

| Milestone | Status | Completion boundary |
| --- | --- | --- |
| M0 — Repository-grounded governance | CLOSED | Stable ownership and recoverable Git/remote workflow |
| M1 — Architecture authority and closure | IN_PROGRESS | One revision, explicit prior dispositions, checks and independent review |
| M2 — Foundation and semantic Core/TD | IN_PROGRESS | Shared semantics, data/security, owner/work/time and supported profiles |
| M3 — Preparation | IN_PROGRESS | Actual pairing, pure/deferred construction, coverage and reuse obligations |
| M4 — Binding capability/lifecycle | IN_PROGRESS | Open native facets, recipient authority and physical retirement |
| M5A — Servient runtime | IN_PROGRESS | Logical publication, dispatch, accepted work and cleanup |
| M5B — Discovery client | OPEN | Bounded trusted retrieval/continuation and exact client scope |
| M5C — Host HTTP/HTTPS production | OPEN | Real supported mapping and security/lifetime qualification |
| M6 — Integration/conformance | OPEN | Joined capability, profile, resource/performance and removal evidence |
| M7 — v1 release | OPEN | Independent technical release acceptance after M6 |

No milestone closes because this target is accepted or historical CI is green.
Source, tests and exact review evidence establish implemented/admitted capability;
GitHub owns live branch/PR/CI state. This roadmap stores no task continuation,
execution checklist or next action.
