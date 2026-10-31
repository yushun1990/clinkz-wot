# ClinkZ-WoT roadmap

Design target: v6.0 semantic Engine over Binding-owned native runtimes, selected
by ADR-0025 and active on master. Architecture activation does not transfer
implementation acceptance or release qualification.

The enduring [product goals](docs/architecture/00-system-goals-and-context.md)
remain unchanged. Release target remains v1 with both WoT roles and
Property/Action/Event capability families, described collections, security,
client-only Discovery/Directory, portable shared semantics and Host HTTP/HTTPS
production delivery. Zenoh/pico and real embedded protocol/hardware qualification
remain separate future work.

## Delivery strategy: basic capability breadth through joined execution

The near-term engineering objective is **basic executable coverage of Property,
Action and Event in both Consumer and Producer roles, including described
collection operations**. Existing composition footholds are means to that result.
Interleave semantic kernels with public TD-driven paths and actual Binding I/O;
use those paths to challenge interface and ownership assumptions. An operation
enum, unit test or raw kernel cannot complete a WoT capability.

### Capability coverage

This coarse coverage is grounded in the linked source and integration fixtures.
The original baseline was recorded at
`dbf81f55536b8789c253b1f90199cee85d3ed6af`; implemented increments update their
affected cells. Source and scoped evidence remain the availability authority;
this is not a package acceptance record.

**Semantic representation:** [TD models and shared interpretation](td/src/semantic.rs)
cover all three families and Thing-root collection Forms, including operation
applicability/defaults and distinct schema roles. The
[operation vocabulary](core/src/operation.rs) represents all 18 TD 1.1 operations.
These facts do not establish executable support; schema and extension processing
still have supported-subset limits.

| Capability | Raw execution kernels | TD-driven Consumer composition | TD-driven Producer composition | Actual Binding I/O evidence |
| --- | --- | --- | --- | --- |
| Property read | Finite Consumer/Producer paths | Owned preparation and captured services | Qualified final TD and local publication | HTTP in both roles; independent test datagram Consumer |
| Property write | Finite Consumer/Producer paths | Operation-scoped value schema, captured Binding/data/access services | Complete qualified final TD, captured input/output validation and local publication | Engine Consumer-to-Producer HTTP PUT/204; Read/Write on one Property/listener; downstream test datagram Consumer |
| Property observe / unobserve | Consumer notification conversation/stop and typed Producer value publication | Selected observable Property Form and captured value/access services | Qualified observable sources, value mappings and publication services | Engine Producer-to-Engine Consumers through Mosquitto; mapped unobserve, independent authority and final release; downstream TD-prepared test datagram Consumer |
| Action invoke | Consumer finite call; Producer continuing invocation store | Explicit `synchronous: true` subset only | Continuing HTTP mapping with captured validation and controls | Test datagram synchronous Consumer; HTTP and test datagram Producer |
| Action query / cancel | Producer retained invocation controls | Open | Qualified described HTTP control routes | HTTP and test datagram Producer; no composed Consumer control path |
| Event subscribe / unsubscribe and Producer emission | Consumer Event conversation; distinct Producer emission and settlement | Selected MQTT Event Form, captured services, validated delivery and logical stop/unsubscribe | Qualified MQTT Event sources and emission; broker-native subscribe/unsubscribe without control data or dataResponse | Engine Producer-to-Engine Consumers through Mosquitto, shared native subscriptions, independent logical authority and final release; independent wire peers also exercised |
| Described collection operations | No collection execution path | Open | Open | No joined native collection evidence |

The [Property Write integration](protocol-bindings/protocols/http/tests/property_write.rs)
proves the bounded JSON/type-only, endpoint NoSec HTTP subset, including
pre-effect validation/authorization, ambiguous response refusal, cancellation,
replacement, retained late outcomes and final release. Unsupported required
schema/security assertions refuse preparation; no automatic uncertain-write
replay or readback confirmation is claimed.

Here **Open** means no joined implementation for that role/operation, even when
a raw primitive exists. Finite read/write evidence is in
[native ingress](servient/finite/tests/native_ingress.rs); composed Producer
read/Action evidence is in
[composed Thing](protocol-bindings/protocols/http/tests/composed_thing.rs) and
[HTTP Actions](protocol-bindings/protocols/http/tests/action_producer.rs).
[Producer Events](protocol-bindings/protocols/mqtt/tests/producer_events.rs)
exercise the [declared MQTT subset](protocol-bindings/protocols/mqtt/README.md)
through complete TD qualification and native broker delivery.
[Consumer Events](protocol-bindings/protocols/mqtt/tests/consumer_events.rs)
join the existing Producer to TD-selected Consumer subscriptions, per-item
validation, stop/unsubscribe and cleanup, with native matching/distribution and
shared connection ownership inside MQTT. Retained deliveries are discarded with
visible loss; no replay or reciprocal response capability is claimed.
[Property observation](protocol-bindings/protocols/mqtt/tests/property_observation.rs)
reuses the notification owner with Property value schemas, future-only native
values and mapped unobserve. Establishment performs no Read, invents no initial
value and grants no Event response right. Its declared loss policy reports drops
or terminates, without coalescing. [Property qualification](protocol-bindings/protocols/mqtt/tests/property_qualification.rs)
checks actual sources, full advertisement, supported schema/mapping and selected
Consumer services before effects.
[Downstream Consumer composition](tools/architecture-fixtures/open-consumer/tests/support/composition.rs)
exercises prepared finite I/O and TD-selected Property observation with an external test Binding; the
[notification fixtures](servient/finite/tests/notification.rs) exercise Event
lifecycle semantics in memory. Test Bindings are not production protocols.

**Production qualification:** narrow HTTP Property Read evidence retains its
[recorded scope](docs/evidence/WP-600-http-consumer-property-read.toml).
Current joined paths are bounded Host/manual, type-only schema and NoSec subsets;
their real I/O does not establish complete family, general DataSchema/security,
HTTPS, constrained-profile or v1 qualification. Those remain delivery work.

### Capability horizons

| Horizon | Primary result | Completion evidence appropriate to that result |
| --- | --- | --- |
| A — Semantic ownership | Finite exchange, continuing Action, notification conversation and Producer publication retain distinct but composable controls and owners. | Executable operation/lifecycle cases, including stale identity, cancellation, late results, saturation and retained cleanup. Raw kernels alone make no TD or native I/O claim. |
| B — Joined Producer foothold | Whole-Thing composition supports finite Property Read and continuing Action mappings over actual native participants; broaden only with complete final support and retained ownership. | Public real-peer tests plus negative support, prepublication, cancellation, replacement and final-resource-release evidence, scoped to the implemented mappings. |
| C — Open Consumer and broader operations | Establish basic executable Property/Action/Event and described collection coverage in both roles through open composition with distinct finite, Action and notification lifecycles. | TD-driven selection or qualified publication, actual services and real Binding I/O for each claimed role/operation subset, with its control and cleanup evidence. Raw kernels and mocks alone do not complete a family. |
| D — Interoperability and product qualification | Verify downstream third-party Binding composition, selected cross-protocol contrasts, Host HTTP/HTTPS and security, Discovery/Directory client, portable shared semantics and supported profiles. | Real native peers, advertised DataSchema/security/profile checks, scoped resource/performance measurements and client Discovery behavior. |
| E — v1 integration | Reconcile completed WP-400/500/600 capability claims through WP-700 without inheriting historical acceptance. | Independent conformance, security, integration and release evidence for the exact advertised v1 scope. |

Horizons express **recommended concentration, not sequential package gates**.
The present frontier spans A–C: extend working composition into the missing
role/operation paths while retaining existing lifecycle guarantees. Basic coverage
is narrower than D's production qualification and E's release integration.

### Near-term capability growth and priority

The objective covers the following distinct semantics, delivered in coherent,
independently reviewable increments:

- **Property:** retain reads and add TD-driven write and observation/stop paths
  in both roles. Validate written and delivered values; observation establishment
  fabricates no initial value and performs no hidden read.
- **Action:** extend the synchronous Consumer subset to continuing invocation
  and described query/cancel, preserving Producer controls. Retain the original
  mapping, principal, result and cleanup independently of the initiating exchange;
  local cancellation does not prove remote rollback.
- **Event:** join Producer emission and Consumer subscription/unsubscription to
  TD-driven preparation/publication and real I/O. Keep subscription, data,
  optional dataResponse and cancellation roles distinct; described reciprocal
  responses require actual native support and cardinality. Event loss is not
  Property coalescing.
- **Collections:** deliver described Thing-root operations alongside family
  growth: Property read/write all or multiple and observe/unobserve all, Action
  query all, and Event subscribe/unsubscribe all. Preserve all-versus-selected
  scope, combined schemas, logical attribution, partial failure and supported
  atomicity; never silently lower a native group operation into N calls or
  subscriptions. [TD 1.1 collection Forms](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#thing)
  and [Core collection semantics](docs/spec/interaction-core.md) retain authority.

For a basic executable claim, use public TD-driven selection or complete Producer
qualification, captured actual services, source-independent execution and a real
Binding peer for the declared subset. Exercise the relevant refusal, validation,
cancellation, replacement, late-result and final-release paths. Semantic breadth,
raw lifecycle evidence and production qualification remain separate claims. The
objective neither requires all operations in one PR nor every Binding to support
every operation.

Select subsequent increments against the **whole roadmap and remaining product
gaps**, using the coverage above and exact work-package dependencies. Prefer
missing executable roles/operations over another mapping of an already exercised
path unless a concrete interoperability need or defect justifies that choice.
The implementation PR should explain the capability advanced, why it is valuable
relative to remaining gaps, its dependencies and the scope of its evidence.
Correct prerequisite schema, security or ownership defects with the affected
slice; do not accumulate speculative kernels or universal runtime abstractions.
This does not impose a fixed cross-family sequence or postpone independent
Discovery, HTTPS/security and portable-profile work until every operation exists.

**HTTP SSE is an optional notification mapping**, selected when it advances the
chosen capability. It is neither a mandatory predecessor nor a standing next
task. Existing HTTP or independent test Bindings with real I/O may be economical
participants; other native interactions may better expose an unproved boundary.
Protocol choice follows capability and interoperability needs, while Host
HTTP/HTTPS remains the selected first production direction.

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

Operation-scoped preparation and shared instance authority belong to
[WP-200](docs/work-packages/WP-200-planning.md#consumer-operation-composition)
and [WP-400](docs/work-packages/WP-400-servient.md#joined-consumer-execution),
with separate native facets in [WP-300](docs/work-packages/WP-300-bindings.md#consumer-capability-composition).
The optional HTTP observation mapping remains in
[WP-600](docs/work-packages/WP-600-protocol-bindings.md#td-driven-consumer-observation).

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
