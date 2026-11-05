# Architecture Review 09: Engine Maturity and the Next Engineering Roadmap

- Date: 2026-11-05
- Status: assessment complete; proposed roadmap and objective pending Owner decision
- Reviewed source and active authority: master
  [`072bf280b93d2956876536568215c9b4141aec4c`](https://github.com/yushun1990/clinkz-wot/tree/072bf280b93d2956876536568215c9b4141aec4c),
  including the finite Consumer callback correction in PR #258

This is a **non-normative planning assessment** for Owner review. It proposes
capability priorities and a falsifiable next objective. It changes no accepted
architecture, roadmap, work-package admission, milestone, or implementation.
Publishing or merging this report does not itself approve the proposed objective
or authorize implementation; an explicit Owner decision remains outstanding.

**Recommendation:** build shared DataSchema preparation and validation next,
with reusable definitions kept separate from live Thing authority. Follow closely
with TD-described security and production HTTPS. Retain broader instance reuse,
scalable Servient composition, portable execution, and Discovery as distinct
architectural capabilities rather than assuming operation coverage completes them.

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

**Proposed engineering roadmap.** Move from operation breadth to semantic depth,
secure execution, and reusable deployment. Priorities express concentration,
not package-wide start barriers or a new completion gate.

| Priority | Major capability | Dependencies and rationale |
| --- | --- | --- |
| 1 | Shared executable DataSchema services | Unlock ordinary structured device data across interaction families. Replace the type-only boundary with bounded prepared validation definitions and explicit role/media support. Address a shared restriction before more Bindings duplicate it. |
| 2 | TD-described security and production HTTPS | Provide prepared security requirements plus runtime credential, principal, authorization, and trust services. Start with a useful supported scheme over real HTTPS in both roles. Destination checks must cover URI expansion and continuation targets. Narrow transport work can proceed alongside priority 1. |
| 3 | Reusable Thing definitions and scalable Servient composition | Provide checked binding of immutable content to independent instance contexts, aggregate capacity, and practical mixed-family application composition. Measure duplicated preparation/backing before choosing caches, indexes, pools, or sharding. |
| 4 | Real Discovery/Directory clients | Join bounded retrieval and TD admission, reusing destination/trust services. Begin with configured URLs and a real Directory peer. Discovery need not wait for fleet optimization or every interaction mapping. |
| 5 | Broader portable execution | Extend supported semantics through caller-owned/static storage and no-atomic representations. Introduce portability checks during priorities 1-3; complete broader runtime parity as a dedicated capability. Hardware qualification remains separate. |
| 6 | Integrated v1 production qualification | Qualify the exact advertised HTTP/HTTPS, security, Discovery, resource, and profile envelope with independent peers, sustained workloads, failure recovery, and usable diagnostics. Add mappings where they test an interoperability need or unresolved architectural boundary. |

Security, reuse, and portability should influence interfaces built in priority 1.
They should not require completing a universal security framework, fleet runtime,
or embedded backend before useful schema support can ship. URI operands and
response/media branches remain distinct semantic work; broaden their executable
support with concrete mappings rather than pretending schema validation alone
completes TD interpretation. Security work must remove common NoSec eligibility
restrictions as actual security services become available.

The [historical performance manifests](../performance/gateway.toml) explicitly
retain characterization scope rather than successor acceptance. New scalability
work needs measurements of preparation cost, retained bytes, active-work cost,
throughput, service gaps, saturation, and final release over increasing Thing,
operation, and subscriber counts. Logical allowances are not physical RAM
measurements. Synchronous linked code remains cooperative; work credits do not
establish preemption or hard real-time behavior.

**Next major objective: execute ordinary TD-described structured data through
shared, reusable validation definitions.** The product outcome is concrete: an
application can describe a structured sensor value or actuator command with
meaningful constraints, then consume, expose, observe, or publish it through
existing Bindings with consistent validation.

The architectural outcome separates the following lifetimes:

- Immutable schema meaning and prepared validation belong to reusable
  definitions.
- Thing identity, current permission, behavior, and per-instance operands belong
  to instance context.
- Calls, notifications, results, and cleanup retain their own owners.
- Native encoding and transport remain Binding/codec responsibilities.

This supplies a concrete first use of reusable prepared content without
committing to a universal compiled program format or claiming general native
definition reuse. Different endpoint/schema/security/profile dependencies may
still require specialized native preparation.

The initial scope should include nested objects and arrays, required members,
numeric bounds, const/enum, and length constraints under explicit limits.
Composition and further assertions can follow in coherent increments. Numeric
precision, annotation handling, format, and unsupported assertions need explicit
policies; silently weakening a schema is unacceptable. The exact supported
subset is an engineering decision inside this objective, not a promise of full
JSON Schema or general extension execution.

Acceptance requires:

1. **Useful execution:** real HTTP Consumer-to-Producer input/output validation
   and real MQTT notification validation using structured constrained values,
   including invalid peer input.
2. **Shared semantics:** Property values, Action roles, Event roles, and
   collection members use the same underlying schema machinery while retaining
   their distinct meanings. Family integration can land in coherent increments.
3. **Source independence:** execution continues after TD, semantic arena, and
   preparation owners are destroyed.
4. **Safe reuse:** at least two independent instances reuse one prepared
   validation definition. Revocation, replacement, or disposal of one cannot
   alter the other's permission or prematurely release shared backing.
5. **Truthful support:** unsupported assertions remain distinguishable from
   invalid descriptions and exhausted capacity. Consumer alternatives remain
   selectable; Producer final advertisement remains completely qualified.
6. **Bounded ownership:** compilation, validation, scratch, retained definitions,
   late results, and destruction have tested limits and original-account
   retention under failure and cancellation.
7. **Preserved portability:** the new semantic representation introduces no
   mandatory Host executor, protocol case, or Arc requirement into its portable
   core. Portable kernel evidence remains distinct from embedded runtime or
   hardware qualification.

The implementation boundary spans TD facts, Core data contracts, Planning,
codecs, and affected validation adapters. Replace the restrictive type-only path
where necessary; do not preserve it as a mandatory compatibility facade. Correct
the known Action callback ownership prerequisite before routing richer linked
services through that owner. Retain the existing distinct finite, Action,
notification, and publication lifecycles.

General endpoint parameterization, automatic caches, persistence, broad media
support, and fleet scheduling belong to later work. Existing HTTP and MQTT
participants are sufficient to begin. They both use JSON in these paths, so
success with them does not establish general media independence. Another
encoding or protocol should earn its place through a concrete use or
counterexample, rather than a protocol-count requirement.

**Principal risks and alternative directions.**

- Schema work can expand indefinitely. Deliver a bounded useful subset first.
  Existing TD numeric facets use machine numeric representations; exact value
  handling requires deliberate limits or correction before claiming broader
  numeric conformance. Unsupported precision or assertions must refuse
  explicitly.
- Reuse can accidentally share authority or trust. Shared schema content is a
  useful starting point. General native-definition reuse must additionally prove
  endpoint, configuration, media, security, and profile compatibility, including
  original physical-owner and account retention.
- Host convenience can harden into architecture. The current manual local
  ownership model is useful, but synchronous callbacks and Rc/Arc representations
  cannot define every supported deployment. Exercise portable representations
  before interfaces spread further; do not add a universal executor merely to
  hide a composition problem.
- Concrete protocol subsets can distort the shared model. HTTP's single-call
  policy and MQTT's QoS 0/control restrictions remain Binding limits. Future
  mappings must preserve legitimate richer native behavior without inventing
  transactional, delivery, or rollback guarantees.
- A secure deployment deadline could justify security first. This is the
  strongest alternative ordering because authentication and trust are direct
  product requirements. A committed large-fleet integration could similarly
  justify bringing checked instance reuse forward. Without either external
  commitment, schema execution offers the broadest immediate capability benefit.

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

The proposed Owner decision is to adopt shared executable DataSchema preparation
and validation as the next major objective, followed closely by security/HTTPS,
with broader instance reuse and scaling retained as separate architectural work.
Implementation awaits an explicit decision.

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
