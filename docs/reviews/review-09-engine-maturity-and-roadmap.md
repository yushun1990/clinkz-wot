# Architecture Review 09: Engine Maturity and the Next Engineering Roadmap

- Date: 2026-11-05
- Status: assessment retained; Owner-approved reuse and instance-binding direction
  reflected in the roadmap; first implementation increment scoped below
- Reviewed source and active authority: master
  [`072bf280b93d2956876536568215c9b4141aec4c`](https://github.com/yushun1990/clinkz-wot/tree/072bf280b93d2956876536568215c9b4141aec4c),
  including the finite Consumer callback correction in PR #258

This is a **non-normative planning assessment**. The Owner has approved making
the boundary between reusable prepared execution definitions and independently
bound logical Thing instances the next engineering priority. [PLAN.md](../../PLAN.md)
records the durable roadmap focus; this report supplies evidence, compatibility
analysis, alternatives and a falsifiable first increment. This documentation
change implements no runtime behavior, changes no detailed architecture contract
or work-package admission, and closes no milestone.

**Recommendation:** establish checked reuse and independent instance binding
through an existing real interaction path. Preserve correct authority,
interaction and native-infrastructure owners while separating their present
coupling to immutable prepared content. Shared validation meaning and broader
DataSchema execution belong on this boundary; implementing more constraints as
an isolated feature is not the next major objective. Security/HTTPS, composable
execution, scale, portable profiles and Discovery remain required delivery work.

The assessment starts from the enduring [product goals](../architecture/00-system-goals-and-context.md),
the [active v6 authority map](../spec/authority.toml), actual source, and executed
tests. Accepted ADRs, prior reviews, and passing checks are evidence to challenge,
not proof that an abstraction is correct or complete. Review 08 and workspace
topics 0056/0081/0082/0106 provide historical questions; their older diagnoses and
sequencing are not inherited as current conclusions.

**Current architectural maturity.** ClinkZ-WoT has reached a meaningful basic
interaction-execution milestone. It remains an incomplete reusable, secure, and
scalable Engine. The strongest implemented boundary separates logical authority,
accepted work, and native infrastructure. Finite exchanges, continuing Actions,
notifications, and publication retain distinct owners. Real HTTP, brokered MQTT,
and independently linked datagram implementations exercise those distinctions,
including cancellation, late results, uncertain effects, replacement, and retained
resource ownership.

The v6 responsibility split remains a reasonable direction: shared WoT meaning
in TD/Core, selection in Planning, logical execution in Servient, and native
mechanics in Bindings. This examination finds no demonstrated need to replace
that split wholesale. Its implementation is still a collection of bounded
capability paths rather than a complete general Engine. In particular, the
current restrictions must not become universal requirements merely because
existing Bindings can operate within them.

| Area | Demonstrated maturity | Remaining architectural limitation |
| --- | --- | --- |
| TD interpretation and extensions | Shared operation applicability/defaults, schema-role loans, URI resolution, bounded context expansion, and extension preservation. | The context profile is limited. Preservation does not establish executable extension support. General URI operands, response branches, and broader media combinations remain incomplete. |
| DataSchema execution | Captured services validate supported input, output, collection members, and notification data. | Execution represents only Any or a value type. Ordinary constraints, including nested properties/items, required fields, ranges, const/enum, and oneOf, are refused. A richer validator cannot be plugged into the current Consumer service boundary without changing it. |
| Security | TD security operands and unknown schemes are represented; NoSec still undergoes application authorization. | Joined preparation paths accept endpoint NoSec. Credential acquisition, verification, security composition, transport trust, scopes, and HTTPS lack joined production execution. |
| Preparation and reuse | Selected execution facts survive destruction of TD and compiler owners. Instance authority is independently represented. | Public runtime construction consumes definitions together with captured services and funding. Checked reuse across differently configured, independently funded instances and first-use preparation coordination are missing. |
| Binding extensibility | External Cargo-linked implementations work without a protocol whitelist. HTTP and MQTT exercise materially different native interactions. | Extensibility is strongest inside the supported semantic envelope. Shared type-only/NoSec restrictions constrain otherwise capable third-party Bindings. Notification selection currently uses a registry parameterized by one Binding type. |
| Servient composition | Multiple native participants can share one logical authority; publication qualifies the final TD before opening ingress. | Application composition remains spread across specialized preparation/runtime APIs. General concurrency, aggregate admission, and broader cross-facet lifecycle behavior are not fully established. |
| Scale | Bounded queues, reservations, native sharing, and focused fairness tests exist. | Current owners have small hard limits and linear searches. The Producer registry permits 32 instances and 16 finite behaviors per instance; HTTP Consumer registration permits one outstanding call through result disposal. These are implementation limits, not suitable universal Engine limits. |
| Host/static/embedded | Host execution works; a narrower static Property Read path exists. | The newer execution crate is std-gated, and Core execution modules require pointer atomics. Equivalent broader execution without mandatory Arc/atomics is missing. |
| Discovery | Explicit client interfaces and an in-memory backend exist. | Real bounded retrieval, Thing Link traversal, Directory interaction, trust, continuation, cancellation, and TD admission are not joined into the target client workflow. |

The source basis for these findings is directly inspectable:

- [TD semantic facts][td-facts] expose structured schemas, distinct data roles,
  URI variables, response metadata, and security operands. The
  [type-only schema compiler][type-schema] refuses assertions that the
  [Core type representation][value-type] cannot retain. This is a missing shared
  execution capability, not evidence of silent acceptance of those constraints.
- [Common finite preparation][common-services] restricts effective security and
  other common semantics before native selection. The
  [Action preparation facet][action-preparation] and
  [notification preparation facet][notification-preparation] retain separate
  admission transactions. Interpretation follows shared rules, but admission
  and semantic walks are repeated across those facets.
- [Consumer definitions][consumer-definitions] retain exact targets, actual
  services/configuration, and original funding. [Runtime construction][consumer-runtime]
  consumes them into an owner associated with one Consumer authority and the
  same account bank. Source independence alone does not provide checked
  bind-many reuse or endpoint parameterization.
- [Whole-Thing preparation][producer-preparation] and
  [composed publication][producer-publication] establish the stronger Producer
  final-advertisement boundary. [Independent Binding integration][external-binding]
  and [mixed collection execution][mixed-collections] provide concrete open
  composition evidence without qualifying every mapping or topology.
- [Producer registry limits][producer-runtime], the
  [HTTP Consumer policy][http-binding], and [MQTT native sharing][mqtt-native]
  separate logical capacity from protocol resource count. One Thing does not
  imply one connection, listener, or native subscription. Conversely, native
  sharing does not establish acceptable per-Thing preparation or record costs.
- The [execution crate's feature boundary][producer-runtime],
  [Core atomic gates][core-features], and [static Property Read implementation][static-runtime]
  establish the actual portability limits. [Discovery resolvers][discovery-resolver]
  and [sessions][discovery-session] remain distinct from a real client workflow.

**Standards interpretation.** The applicable baseline is the dated
[TD 1.1 Recommendation](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/),
[WoT Architecture 1.1 Recommendation](https://www.w3.org/TR/2023/REC-wot-architecture11-20231205/),
and [Discovery Recommendation](https://www.w3.org/TR/2023/REC-wot-discovery-20231205/).
They distinguish operation vocabulary from data, security, discovery, and
interoperability capabilities. They do not prescribe a Rust scheduler, cache,
program representation, or universal session model. The abstract Servient
implementation discussion is non-normative.

TD DataSchema is not a requirement to implement every JSON Schema feature or
general JSON-LD processing. Format validation is optional; annotations and
executable assertions need deliberate treatment. An Engine claiming required
validation must support or explicitly refuse the selected meaning. Individual
Binding mappings retain their actual scope and publication status. The
[2025 Binding Templates retirement notice](https://www.w3.org/TR/2025/NOTE-wot-binding-templates-20251104/)
and evolving mapping drafts are not production-conformance evidence.

**Classification of remaining work.** These categories have different
engineering consequences:

- **Demonstrated defect:** [continuing Consumer Action admission][action-runtime]
  retains a mutable registry borrow while invoking linked services. Reentry
  through authorization into close/progress conflicts with that borrow. The
  finite correction in #258 does not cover this distinct owner. This is a
  source-demonstrable conflict; no new Action reentry reproduction was executed
  for this assessment. Treat it as a prerequisite when extending that path,
  rather than the next strategic objective.
- **Missing capabilities:** executable DataSchema constraints, general security
  services, checked instance reuse, broader portable execution, and real
  Discovery clients. Explicit supported-subset refusal is not itself an
  unsoundness finding.
- **Unverified assumptions:** acceptable costs at many-Thing scale; compatible
  sharing across trust domains; heterogeneous notification composition; and
  bounded progress under sustained mixed workloads. Small passing tests cannot
  settle these. Generic typed notification registries permit useful external
  integration, but do not by themselves demonstrate selection across arbitrary
  heterogeneous Binding implementations.
- **Intentional deferrals and exclusions:** real embedded hardware/backend
  qualification and Zenoh/pico production remain future delivery work.
  Automatic caching/persistence is optional. Directory server implementation,
  dynamic Rust plugin loading, and hot unloading remain outside v1. These are
  distinct from the missing portable runtime and Discovery client capabilities.

Three assumptions deserve explicit challenge: source-free definitions are not
already reusable definitions; per-owner quotas do not demonstrate aggregate
scalability; and external Binding authorability does not demonstrate semantic
extensibility beyond the current schema/security envelope. The former closed
security deserializer is no longer a current defect: unknown scheme data is
preserved, while execution support remains separate.

**Selected engineering roadmap.** The near-term focus moves from basic operation
coverage to completing reusable, composable WoT Engine execution. The first
priority establishes the definition/instance boundary, rather than postponing it
until after DataSchema or native-artifact designs have hardened. Priorities
express concentration, not package-wide start barriers or a new completion gate.

| Priority | Major capability | Dependencies and rationale |
| --- | --- | --- |
| 1 | Reusable prepared definitions and checked instance binding | Separate eligible immutable execution meaning from actual instance binding, permission and accounting. Prove a useful shared definition with existing real I/O, while qualifying each native artifact against its real dependencies. Preserve correct existing owners rather than designing a universal program hierarchy. |
| 2 | Shared semantic and secure execution services | Broaden DataSchema/media/response meaning and TD security on that boundary. Deliver useful security schemes and Host HTTPS with runtime credentials, verified principals, authorization and transport trust. Actual destination checks cover URI operands and continuation targets. Narrow security/transport work can proceed alongside priority 1. |
| 3 | Composable runtime, scale and supported profiles | Extend the boundary through continuing Actions, notifications and Producer composition with their existing distinct lifecycles. Measure shared preparation, per-instance costs, active work and native sharing before choosing caches, indexes, pools or scheduling changes. Exercise caller-owned/static and no-atomic representations as supported capabilities grow. |
| 4 | Real Discovery/Directory clients | Join bounded retrieval and TD admission, reusing destination/trust services. Begin with configured URLs and a real Directory peer. Discovery need not wait for fleet optimization, universal reuse or every interaction mapping. |
| 5 | Integrated production and v1 qualification | Qualify the exact role/operation, HTTP/HTTPS, security, Discovery and portable shared-layer envelope using independent peers, sustained workloads, failure recovery and usable diagnostics. Hardware/backend qualification remains distinct future work. |

Schema constraints remain an important missing capability. Their compiled meaning
and actual validator support must fit the reusable definition and instance
boundary, with explicit roles, limits and ownership. Security work similarly
separates invariant requirements from mutable credentials and permission and
removes common NoSec eligibility restrictions as supporting services arrive.
URI operands, response/media branches and extension execution remain distinct
semantic work; broaden them with concrete mappings. Existing v1 requirements
are preserved, not replaced by a reuse-only delivery target.

The [historical performance manifests](../performance/gateway.toml) explicitly
retain characterization scope rather than successor acceptance. New scalability
work needs measurements of preparation cost, retained bytes, active-work cost,
throughput, service gaps, saturation, and final release over increasing Thing,
operation, and subscriber counts. Logical allowances are not physical RAM
measurements. Synchronous linked code remains cooperative; work credits do not
establish preemption or hard real-time behavior.

**Boundary to establish.** Existing [preparation/reuse contracts](../spec/planning.md),
[runtime safety](../spec/runtime-safety.md), [resource ownership](../spec/foundation.md)
and [Binding SPI](../spec/binding-spi.md) already distinguish these concerns.
They are not a requirement to introduce five new public abstractions.

| Concern | Existing implementation and correct boundary to retain | Reuse and qualification consequence |
| --- | --- | --- |
| Shared semantic definitions and validation meaning | Core already represents operation/data roles and named collection schemas; TD owns common interpretation. The Consumer collection schema is an owned member/name table, while the HTTP Producer program already shares its table through Arc. | Capture invariant meaning once in source-independent prepared content. Reuse the actual content, not just an equal type/name/hash. Its supported schema/media/security meaning is not current permission or an executable-service certificate. |
| Binding-specific compiled artifacts | The HTTP Consumer artifact embeds operation, complete target, method and headers. The Producer program embeds operation/Form, address, target/path and its schema association. Actual compiler/executor configuration and role are checked separately. | These complete artifacts are endpoint-dependent today. Different endpoints need separately qualified artifacts unless the concrete Binding deliberately implements checked operands. Artifact identity, configuration digest, Rust type or format equality alone cannot prove compatibility. |
| Per-instance identity, operands, policy, authority and accounts | ConsumerInstance already supplies a move-only authority and passive reference. ConsumerDefinition currently bundles exact mapping, services and funding, and runtime construction requires its bank to match the instance's bank. | Preserve independent authority. Bind an actual endpoint/mapping, supported services, mutable policy and per-instance capacity explicitly; keep the shared backing's original owner/account. An endpoint may be lent from its immutable mapping rather than duplicated as competing truth. |
| Per-interaction lifecycle and results | Finite calls already retain original definitions, controls, deadlines, native outcomes and settlements. Action invocation, notification and publication owners have separate lifetimes. | Keep these owners. A shared definition cannot become a shared call, cancellation flag, result slot or cleanup budget. Late/unread results retain their original instance association, captured services and backing. |
| Binding-owned native infrastructure | HTTP currently uses call-owned connections and a one-call-per-registration policy. MQTT already shares native services/interests through separate logical participation. | Definition reuse neither creates a connection pool nor requires one native resource per Thing. Sharing or specializing native infrastructure is a separate actual Binding policy with its own compatible leases, progress and retirement owner. |

The source distinction is material. [HttpArtifact][http-program] includes the
entire URL; the [native Consumer][http-consumer] parses and uses that captured
target. [HttpProducerProgram][http-producer-program] similarly captures native
address/target facts, and [HttpFiniteConfig][http-finite-config] separately carries
the logical Producer key. Retargeting either program by relabeling an artifact
envelope would bypass its existing support/compatibility assumptions. Conversely,
the [PropertyCollectionSchema][collection-schema] member table has no endpoint or
live authority and is eligible shared meaning when its exact members, roles and
supported assertions agree. Existing Producer schema sharing within one prepared
mapping is useful precedent, not proof of cross-instance accounting or reuse.

**First engineering increment: checked reuse of prepared ReadAllProperties
meaning across independently bound HTTP Consumers.** Use the existing
[real HTTP collection-read path][http-read-all] and its captured JSON/type-only,
endpoint-NoSec services. Its nontrivial owned member table gives a more useful
reuse demonstration than sharing a scalar type enum or a common marker. It
requires no new operation, protocol or DataSchema constraint.

Prepare one immutable Consumer root-read definition with its exact readable
members, data role and supported common meaning. For two concrete loopback
endpoints, pure preparation qualifies the actual HTTP mappings and all relevant
Form/response/extension facts against the actual registrations and services.
That endpoint preparation may construct distinct native artifacts; it should
borrow the original shared meaning rather than rebuild an independent retained
member table. Bind two logical Consumer instances, each with its actual endpoint
association, policy and accounts, and execute against two existing fully qualified
Engine Producers returning distinguishable values. Each root read remains one
native interaction, never a fallback to individual Property reads.

The first increment's sharing claim is deliberately precise: prepared common
execution meaning is reused across different endpoints; the present exact-target
HTTP artifacts remain specialized. An implementation may also share an unchanged
artifact when the actual Binding establishes all dependencies and associations,
but that is a separate supported case. This increment does not require splitting
HTTP bytes into a new parameter language or claiming every native artifact can
be shared across devices. Endpoint-specific pure preparation can finish before
instance binding; the first increment need not implement arbitrary first-use
compilation after source retirement.

Prefer separating the coupling in the present ConsumerDefinition/runtime handoff
and reusing the existing collection schema, actual registrations, authority and
call/settlement mechanisms. An owned shared schema view is an implementation
option, not a prescribed new trait hierarchy. Merely wrapping the entire current
definition in Arc would still couple per-instance policy/accounting, while
preparing every instance fully and interning equal outputs afterward would not
demonstrate preparation reuse.

Acceptance requires the following concrete outcomes:

1. **Observable prepared-content reuse:** construct the common operation/member
   meaning once and retain one shared member/name backing across both bound
   Consumers. Instrument construction and allocation/account ownership, not just
   byte equality. Any repeated endpoint-specific lowering or compatibility work
   is measured separately and remains bounded and funded.
2. **Real independent endpoint binding:** execute root reads at two different
   endpoints with distinct results, instance identities and independent policies.
   Denial, revocation, close or replacement of one instance cannot authorize,
   revoke, retarget or reclaim the other's work. Equal numeric slot values from
   different authority owners cannot collapse those identities.
3. **Complete actual qualification:** before effects, each selected Consumer
   operation passes actual Binding/configuration, endpoint, schema/media/security
   service, profile and capacity checks. Each Producer peer retains its existing
   complete final-TD qualification, behavior, collision/readiness and publication
   checks. Sharing never serves as blanket qualification; Consumer partial
   support remains distinct from Producer complete advertisement.
4. **Dependency counterexamples:** changed member/schema meaning requires new
   common preparation or refusal. Changed target/path/header or native
   configuration requires appropriate actual native preparation/checking; an old
   exact-target artifact cannot be relabeled. Foreign actual provider, unsupported
   security/media, unavailable validator, stale authority and changed captured
   bounds refuse before effects or fence the original work as applicable.
5. **Source-independent binding and execution:** after the two endpoint mappings
   are purely qualified, discard TD, semantic-arena and compiler owners, then
   bind/install and execute from the retained prepared content and actual runtime
   registrations. Binding or warm execution cannot recover meaning by reparsing
   the source or invoking a hidden TD callback.
6. **Exact independent funding:** the shared definition's physical owner retains
   its original reservations once. Each instance separately funds its reference,
   operands/services, records and interaction/result/cleanup obligations at its
   own account; compatible shared service backing also keeps its actual owner.
   Do not simply remove the existing foreign-bank guard or move charges to a new
   instance. Failed second-instance binding must return its owners and reservations
   without disturbing the first instance or shared content.
7. **Late-result and final-release truth:** keep an original result or late native
   outcome while its originating instance and preparation root are dropped and
   the sibling continues. Original validation, authority association, shared
   backing and both shared/per-interaction funding survive until their actual
   final uses. Release occurs exactly once after final disposal, including
   failure, cancellation, saturation and unread settlements. Independent-instance
   tests need not change HTTP's declared native concurrency policy.

The implementation boundary is the selected TD/Planning preparation capture,
Core prepared-content/instance handoff, finite Consumer runtime and the actual
HTTP preparation/execution association. Existing Producer composition supplies
the real peer and complete-advertisement regression evidence; Producer reuse is
not silently claimed. Reuse of the existing codec/validator is sufficient. Retain
callback isolation and external owned/incremental compiler eligibility. Any
exposed contract conflict must be corrected at its actual owner, rather than
hidden behind aliases or a second implementation path.

Do not generalize the first increment into an eager all-family migration,
automatic cache, serialized image, universal VM, scheduler or Engine session
manager. It makes no fleet-throughput, new media, new schema, general native
artifact sharing or embedded-runtime claim. Portable shared meaning must not
acquire a mandatory Host executor or new Arc requirement solely from the Host
binding representation. Extend the proved boundary to other capabilities and
profiles with their own real evidence; the known continuing Action callback
defect remains a prerequisite where that distinct path is touched.

**Principal risks and alternative directions.**

- Reuse can accidentally share authority, mutable policy, trust or accounting.
  Separating content from instance binding is therefore the first objective.
  Sharing the entire current ConsumerDefinition, a native service or an account
  bank is not a substitute for proving the actual independent owners.
- Native artifacts may remain specialized. Requiring an endpoint-independent
  universal program would invent compatibility the HTTP implementations do not
  currently have. Keeping every byte per instance would miss eligible common
  meaning. Select sharing granularity from actual dependencies and report the
  measured benefit without promising a fleet-scale reduction in advance.
- Schema expansion remains substantial later work. Existing TD numeric facets
  use machine numeric representations; exact values, annotations and supported
  assertions need deliberate policies before broader conformance claims. A
  larger validator cannot resolve the current instance/account coupling by
  itself, and unsupported assertions must still refuse.
- Host convenience can harden into architecture. The current manual local
  ownership model is useful, but synchronous callbacks and Rc/Arc representations
  cannot define every supported deployment. Exercise portable representations
  before interfaces spread further; do not add a universal executor merely to
  hide a composition problem.
- Concrete protocol subsets can distort the shared model. HTTP's single-call
  policy and MQTT's QoS 0/control restrictions remain Binding limits. Future
  mappings must preserve legitimate richer native behavior without inventing
  transactional, delivery, or rollback guarantees.
- A secure deployment deadline could justify advancing HTTPS/security alongside
  the first increment. Security is a direct product requirement, not optional
  cleanup after reuse. A committed large-fleet workload could justify earlier
  native parameterization, indexes or sharing measurements, but must not remove
  dependency checks or independent policy/accounting. Neither need is inferred
  merely from a desirable optimization.

Discovery remains a client capability rather than a Directory server project.
Its Introduction/Exploration workflow can be delivered progressively using
configured entry points and established credentials; it does not need a universal
discovery framework or completion of every introduction mechanism.

**Executable evidence and its limits.** The following commands were executed
against the reviewed source tree on Linux x86_64 with Rust 1.95.0. The local
implementation-branch tree was verified identical to the pinned master tree.
Counts include unit, integration, and documentation tests where applicable;
they are not counts of independent semantic requirements.

| Command | Result |
| --- | --- |
| `cargo test --locked --offline --workspace -- --test-threads=1` | 1,004 passed; 0 failed; 1 ignored across 113 test-result summaries. |
| `cargo test --locked --offline --manifest-path tools/architecture-fixtures/open-consumer/Cargo.toml -- --test-threads=1` | 47 passed; 0 failed across 6 summaries. |
| `MOSQUITTO_BIN=/tmp/mosquitto-2.0.22/src/mosquitto cargo test --locked --offline -p clinkz-wot-binding-mqtt --features broker-tests -- --test-threads=1` | 105 passed; 0 failed across 13 summaries, using real Mosquitto peers. The executable path is environment-specific. |
| `cargo test --locked --offline -p clinkz-wot-servient --no-default-features --test property_read` | 4 passed; 0 failed; narrow static Property Read execution on the Host toolchain. |

The first sandboxed workspace attempt could not open local sockets and failed
with permission errors. The full workspace command was rerun outside that
restriction and passed as reported above. No failure was attributed to Engine
behavior merely because the environment prohibited socket creation.

These runs support the existing execution baseline and refusal/lifecycle cases.
They do not establish fleet performance, hardware fit, all-profile parity,
complete DataSchema/security, or production interoperability. No new hardware or
cross-target execution was performed. The automatic required validation workflow
checks [diff hygiene](../../.github/workflows/mainline.yml), not those broader
claims.

These executed results are preserved as the assessment baseline. The revised
direction and first increment above are engineering conclusions from that source
and the Owner-approved priority; no checked-reuse implementation or new acceptance
result is claimed by this documentation revision.

[td-facts]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/td/src/semantic/facts.rs
[type-schema]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/planning/src/type_schema.rs#L138
[value-type]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/core/src/value_type.rs
[common-services]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/planning/src/open_consumer.rs#L817
[action-preparation]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/planning/src/open_consumer/action.rs
[notification-preparation]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/planning/src/open_consumer/notification.rs
[consumer-definitions]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/core/src/consumer.rs#L499
[consumer-runtime]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/servient/finite/src/consumer.rs#L467
[producer-preparation]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/planning/src/producer_composition.rs
[producer-publication]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/servient/src/producer_composition.rs
[external-binding]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/tools/architecture-fixtures/open-consumer/README.md
[mixed-collections]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/protocol-bindings/protocols/mqtt/tests/collection_coexistence.rs
[producer-runtime]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/servient/finite/src/lib.rs
[http-binding]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/protocol-bindings/protocols/http/README.md
[mqtt-native]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/protocol-bindings/protocols/mqtt/src/native.rs
[core-features]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/core/src/lib.rs
[static-runtime]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/servient/src/property_read.rs
[discovery-resolver]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/discovery/src/resolver.rs
[discovery-session]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/discovery/src/session.rs
[action-runtime]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/servient/finite/src/consumer_action.rs#L291
[http-program]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/protocol-bindings/protocols/http/src/program.rs
[http-consumer]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/protocol-bindings/protocols/http/src/lib.rs
[http-producer-program]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/protocol-bindings/protocols/http/src/producer.rs
[http-finite-config]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/protocol-bindings/protocols/http/src/finite_producer.rs
[collection-schema]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/core/src/collection.rs
[http-read-all]: https://github.com/yushun1990/clinkz-wot/blob/072bf280b93d2956876536568215c9b4141aec4c/protocol-bindings/protocols/http/tests/read_all_properties.rs
