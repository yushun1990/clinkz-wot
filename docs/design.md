# clinkz-wot Design

Status: v4.6 frozen implementation-ready target; coordinated implementation
proceeds only through the admitted work-package DAG.

This document is the authoritative project design for `clinkz-wot`. It is a
target design: implementation code may lag behind it. Previous architecture
baselines, implementation plans, audit notes, and target documents are
historical references under `docs/deprecated/`.

## Normative Language and Requirement Identity

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHOULD**, **SHOULD NOT**,
and **MAY** in this document are to be interpreted as described in RFC 2119 and
RFC 8174 when, and only when, they appear in uppercase. Lowercase uses describe
intent or guidance and are not independently testable requirements.

Stable requirement identifiers use a domain prefix and number, for example
`RES-LIMIT-001` or `LIFE-EXPOSE-001`. An identifier names the requirement, not
its paragraph location, and remains stable when text moves. A semantic change
to an identified requirement requires design review and updated verification
evidence. `DOC-GOV-001`: New design work SHOULD identify independently verified
requirements. Uppercase normative language is scoped to the nearest preceding
stable requirement id until another id or heading begins. Normative text under
a heading without a nearer id is mapped to that heading's requirement-family
row in the traceability matrix. This rule makes every normative statement
traceable without assigning a separate id to every sentence.

Code blocks are illustrative unless explicitly labelled as normative public
APIs. Type names described as a "minimum API" are normative names and
semantics; names described as "or equivalent" are architectural roles whose
concrete spelling may vary. A conforming implementation may deviate from a
SHOULD only when the rationale, affected profiles, and verification evidence
are recorded in an active design decision. It may not deviate from a MUST
without first changing this document.

Requirements use four independent classification axes:

- **compilation environment**: `std` or `no_std + alloc`;
- **execution model**: host-erased async integration or constrained manual
  poll/step progress;
- **resource profile**: dynamically configured bounded storage or a named static
  profile with caller-provided capacities;
- **capability role**: Producer, Consumer, Discovery/Directory client, or a
  composition such as a gateway.

`PROFILE-AXIS-001`: No compilation environment implies a hardware platform or
capability role. A constrained execution/resource profile MUST be usable under
`no_std + alloc` and MAY also be selected under `std`. A gateway MAY use either
execution model. Directory client integrations MAY require `std`, but
protocol-neutral Directory request, result, and progress types do not.
Requirements labelled **constrained**, **gateway**, or **directory-client**
below name execution, resource, or capability profiles, not processors,
operating systems, or Cargo features. A Directory service is outside the current
design scope.

### Design Authority and Change Control

`ARTIFACT-AUTH-001`: `docs/design.md` defines normative behavior and public
architecture. `docs/api-ownership.csv` is authoritative only for the defining
crate, module, public path, feature cells, and migration disposition of frozen
cross-crate items. `docs/artifacts.csv` enumerates the active artifact set and
its revision/schema identities. `docs/refactor-gates.csv` records
implementation-admission status. `docs/requirements.csv` indexes requirement
profile axes, package ownership, and evidence. The versioned files under
`docs/performance/`, their schemas, and the fixture lock define profile-specific
performance budgets and
stable measurement identities; `tools/performance-harness` checks and
orchestrates that contract. `docs/work-packages/index.toml` defines the
implementation dependency DAG, and its package documents define migration and
removal work without becoming a source of behavioral requirements. `PLAN.md`
selects the active revision but does not redefine it. When active artifacts
disagree, implementation MUST stop
at the conflicting requirement; neither code, a benchmark result, a work
package, nor an older document resolves the conflict implicitly. The design and
every affected supporting artifact MUST be corrected in one reviewed change
before implementation continues.

`API-OWNERSHIP-001`: Every frozen public type, trait, registration, operation
slot, lifecycle state record, and named profile MUST have exactly one defining
crate and public path in `docs/api-ownership.csv`. A higher crate MAY re-export
an item but MUST NOT redefine it. The matrix MUST be dependency-direction safe,
contain no undecided owner or path when its gate is closed, and identify whether
the current implementation keeps, relocates, adds, replaces, or removes the
item. Its checker MUST reject duplicate items and paths, unknown crates,
unindexed requirement ids, invalid feature cells, missing current paths for
keep/relocate entries, and placeholder ownership decisions.

`IMPL-CONFORM-001`: The coordinated implementation refactor is planned and
reviewed by requirement id. Every implementation work package MUST identify its
affected requirements, crates and feature cells, public API or data migration,
state and ownership changes, verification evidence, and applicable performance
workloads. Implementers MUST NOT preserve an obsolete implementation shape by
weakening this target design, invent observable behavior for an ambiguity, or
silently defer a mandatory requirement. A newly discovered ambiguity or
infeasible budget returns to design review before dependent implementation is
merged. Temporary nonconformance is allowed only on a private refactor branch or
behind an unavailable-by-default feature and must have an owner and removal
condition; it is not a releasable compatibility mode.

The admitted dependency order is `WP-000 -> WP-100 -> WP-200 -> WP-300`, then
`WP-400`, `WP-500`, and `WP-600` may proceed independently, and `WP-700`
depends on all three. The machine-readable package set MUST cover every indexed
requirement and performance workload, use actual or explicitly target Cargo
package names and feature cells, record old API removal and stable evidence
keys, and contain no dependency cycle or undeclared predecessor.

`CHANGE-CONTROL-001`: A semantic change to a frozen public API, lifecycle
transition, error category, resource default, complexity bound, or performance
budget requires a new design revision. The change records affected requirement
ids, compatibility and migration impact, and updated verification keys. Raising
a resource or performance limit is not a performance fix. A budget waiver is
time-bounded, names the affected profile and workload, includes measured cause
and risk, and cannot establish a new baseline unless this design and its
manifest are updated deliberately.

`REFACTOR-GATE-001`: The coordinated implementation refactor MUST NOT begin a
work package while any gate on which it depends is open. Gate closure requires
the evidence named in `docs/refactor-gates.csv` to be present and passing in the
same revision. Design-only edits, checkers, deterministic fixture generation,
performance-harness construction, and work-package authoring MAY proceed while
runtime migration is blocked. Reopening a gate blocks packages that have not
yet merged and requires an impact review for packages already completed.

### Revision Record

v4.6 is the frozen implementation-ready revision. It replaces the earlier
unqualified "implementation-ready" status with explicit admission gates, adds a
machine-readable API ownership contract, introduces a narrow foundation crate
for resource, work, time, and generation primitives, and makes work-package
dependency on gate closure explicit. It also resolves the cross-layer ownership
direction before any runtime API migration begins. All six gates are closed:
the lifecycle state machines terminate with owned cleanup; Directory is a
client-only contract; resource profiles are exhaustive; performance workloads,
fixtures, and result identities are executable; and the eight-package
implementation DAG covers every requirement and workload. Reopening any gate
removes implementation admission from its affected packages.

v4.5 completes the performance-contention pass of the 2026-07-13 design audit.
It adds `PLAN-CACHE-001`, `DIR-STREAM-001`, `PERF-ACCOUNT-001`, and
`PERF-FANOUT-002`. These requirements prevent lazy-compilation stampedes and
eager invalidation scans, require incremental Directory page admission, remove
global resource-accounting locks from interaction hot paths, and bound fan-out
progress without weakening per-affordance ordering. The performance manifests
add deterministic gates for these cases. These are target-design constraints;
they do not assert that the current implementation already conforms.

v4.4 closes implementation ambiguities found by the 2026-07-13 design audit.
It adds `JSONLD-PREFIX-001`, `DIR-AUTH-001`, `BIND-PROGRESS-001`, and
`PERF-PEAK-001`; extends `API-SURFACE-001`, `API-DIRECTORY-POLL-001`,
`SEC-PERF-001`, and the subscription/binding state contracts; and updates their
verification keys. The public migration is deliberate: constrained bindings
use active subscription and response slots, terminal-bearing process events,
and provider capability generations; Directory mutation calls use typed
publication authority. The gateway maximum-admission peak performance budget is
reduced from 256 MiB to 24 MiB and Directory client peak gates are explicit.
Because this repository treats the active document as target design rather than
the current implementation, v4.4 provides no temporary compatibility facade;
`IMPL-CONFORM-001` work packages must migrate the affected cross-crate surfaces
together before declaring conformance.

## References

Primary references:

- W3C Web of Things Architecture 1.1, W3C Recommendation 05 December 2023:
  <https://www.w3.org/TR/2023/REC-wot-architecture11-20231205/>
- W3C Web of Things Thing Description 1.1, W3C Recommendation 05 December
  2023:
  <https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/>
- W3C Web of Things Scripting API, W3C Group Note 03 October 2023:
  <https://www.w3.org/TR/2023/NOTE-wot-scripting-api-20231003/>
- W3C Web of Things Discovery, W3C Recommendation 05 December 2023:
  <https://www.w3.org/TR/2023/REC-wot-discovery-20231205/>

`STD-BASELINE-001`: The dated publications above, not their moving latest or
editor-draft URLs, are the conformance baseline. Published errata are reviewed
individually and recorded with affected requirements and fixtures before they
change engine behavior. A later W3C publication, editor draft, or TD 2.0 feature
does not alter the stable profile implicitly; experimental adoption remains
behind an explicit feature and design revision.

The Scripting API is treated as a semantic and method-catalogue target. Its
WebIDL and JavaScript shapes are mapped to Rust idioms rather than copied
verbatim.

## Purpose

`clinkz-wot` is a protocol-neutral Rust Web of Things engine for the Clinkz
platform. It uses W3C Thing Descriptions as the semantic contract, provides a
Rust application-facing API aligned with the WoT Scripting API, and keeps
transport behavior in optional protocol binding crates.

The engine targets:

- W3C WoT Thing Description 1.1 by default.
- W3C WoT Architecture 1.1 runtime and protocol binding separation.
- WoT Scripting API Consumer, Producer, and Discovery user-agent semantics.
- `no_std + alloc` support for data models, validation, interaction core,
  constrained local dispatch, and protocol planning.
- `std` host support for runtime conveniences, storage adapters, and concrete
  network backends.

TD 2.0 work is experimental and must remain behind `td2-preview`.

## Compatibility Profile

The engine has four compatibility layers:

- TD/TM:
  Target WoT TD 1.1. Parse, build, validate, serialize, and preserve unknown
  extensions.
- Runtime architecture:
  Target WoT Architecture 1.1. Keep Runtime, Servient, TD, and Protocol
  Binding boundaries separate.
- Scripting API:
  Target Consumer, Producer, and Discovery UA semantics. Provide Rust methods
  with equivalent behavior and documented deviations.
- Clinkz extensions:
  Target JSON-LD extension vocabulary. Keep Clinkz terms under a Clinkz
  namespace such as `cz:`.

The engine does not claim JavaScript WebIDL binary compatibility. It claims
semantic alignment: method catalogue, parameter meaning, lifecycle, error
categories, and interaction behavior are equivalent unless a deviation is
documented in this file.

The W3C WoT Scripting API is a W3C Group Note, not a W3C Recommendation. This
project treats the 2023-10-03 published Note as the Rust semantic target unless
this document explicitly adopts a later snapshot.

### Scripting API Mapping

Rust names use `snake_case`; JavaScript promise rejection maps to `Result`.
JavaScript streams/listeners map to Rust streams, subscriptions, or callbacks
as documented below.

- `WOT.consume(td)` maps to `Servient::consume(td)` and returns
  `ConsumedThingHandle`.
- `WOT.produce(init)` maps to `Servient::produce(init)` and returns
  `ExposedThingHandle`. The Scripting-compatible input is an
  `ExposedThingInit`/Partial TD value. `produce()` performs binding-independent
  expansion and `expose()` performs binding-dependent finalization before the
  effective TD is served. Rust-native APIs may additionally accept an already
  complete TD or source envelope through explicitly named methods.
- `WOT.discover(filter)` maps to `Servient::discover(filter)` and returns a
  lazy `ThingDiscoveryProcess` whose Scripting-compatible item view is a bare TD.
  Rust extension accessors or sibling APIs expose source metadata envelopes.
- `WOT.requestThingDescription(url)` maps to
  `Servient::request_thing_description(url)` and uses `AbsoluteUri`. The
  Scripting-compatible result is a bare TD view. `Servient::fetch_td(url)` may be
  provided only as a same-contract bare-TD convenience alias. Rust-native
  document APIs such as `fetch_td_document(url)` return the TD source envelope.
- `WOT.exploreDirectory(url, filter)` maps to a Scripting-compatible
  `Servient::explore_directory(url, filter)` or
  `Discoverer::explore_directory(url, filter)` surface using `ThingFilter`
  semantics and yielding bare TD views. Rust extension APIs may return source
  envelopes and may accept `DirectoryRef` plus `DirectoryQuery` for
  directory-native query features.
- `ConsumedThing.readProperty` maps to
  `ConsumedThingHandle::read_property`.
- `ConsumedThing.writeProperty` maps to
  `ConsumedThingHandle::write_property`.
- `ConsumedThing.invokeAction` maps to
  `ConsumedThingHandle::invoke_action`.
- TD action lifecycle operations `queryaction` and `cancelaction` map to
  `ConsumedThingHandle::query_action` and
  `ConsumedThingHandle::cancel_action`. They are Rust extensions to the
  Scripting API method catalogue, but they preserve TD 1.1 operation semantics.
- `ConsumedThing.observeProperty` maps to
  `ConsumedThingHandle::observe_property` and returns a Rust `Subscription`
  stream.
- `ConsumedThing.subscribeEvent` maps to
  `ConsumedThingHandle::subscribe_event` and returns a Rust `Subscription`
  stream.
- `Subscription.stop(options)` maps to `Subscription::stop`; Rust may expose a
  no-argument convenience method only when no TD-level teardown input is
  required.
- Handle-level `unobserve_property` and `unsubscribe_event`, if exposed, are
  Rust convenience extensions. They must require a subscription identity when
  more than one subscription can exist for the same affordance.
- `ConsumedThing.readAllProperties` maps to
  `ConsumedThingHandle::read_all_properties` and returns `PropertyReadMap` on
  success.
- `ConsumedThing.readMultipleProperties` maps to
  `ConsumedThingHandle::read_multiple_properties`.
- `ConsumedThing.writeMultipleProperties` maps to
  `ConsumedThingHandle::write_multiple_properties`.
- TD operation `writeallproperties`, which is not part of the stable
  Scripting API method catalogue, maps to Rust extension method
  `ConsumedThingHandle::write_all_properties`.
- `ConsumedThing.getThingDescription` maps to
  `ConsumedThingHandle::thing_description`.
- `ExposedThing.expose` maps to `ExposedThingHandle::expose`.
- `ExposedThing.destroy` maps to `ExposedThingHandle::destroy`.
- `ExposedThing.getThingDescription` maps to
  `ExposedThingHandle::thing_description`.
- Producer handler setters map to `set_*_handler` and
  `set_async_*_handler`.
- Producer event emit maps to `emit_event` and `emit_property_change`.

TD 1.1 operations that are not prominent in the Scripting API remain supported
as WoT operation semantics. They are treated as Rust extensions to the Scripting
API mapping rather than deviations.

### Documented Deviations

The following deviations are intentional:

1. `Result<T, E>` replaces JavaScript thrown exceptions and rejected promises.
2. Rust streams replace JavaScript listener callbacks for consumed
   `observe_property` and `subscribe_event`.
3. Rust may expose handle-level subscription teardown helpers in addition to the
   Scripting API `Subscription.stop()` model.
4. Payloads are byte-oriented with media metadata. JSON conversion helpers are
   convenience APIs, not the core interaction representation.
5. Handler registration uses Rust traits and owned values instead of WebIDL
   callback objects.
6. `InteractionOutput` exposes Rust-native byte, stream, and JSON helper
   methods instead of JavaScript `value()`, `arrayBuffer()`, and `dataUsed`
   names.
7. The Rust subscription model may expose an extended mode with multiple active
   subscriptions for one property or event. A Scripting-compatible mode must
   reject a second active subscription for the same property or event.
8. `Subscription::stop` may be implemented through an owned Rust teardown guard,
   but protocols that require explicit `unobserveproperty` or
   `unsubscribeevent` interactions must preserve those TD operation semantics.
9. Rust may expose directory-native query, projection, pagination, and watch
   options beyond the Scripting API `ThingFilter`. The Scripting-compatible
   discovery methods still accept or map from `ThingFilter` semantics.

Each deviation must preserve the interaction semantics: start, receive data,
receive errors, cancel, and release resources must remain observable by the
application.

## Design Principles

1. The engine is protocol-neutral. TD/TM, core runtime abstractions, discovery,
   and servient composition must not depend on zenoh-specific behavior.
2. W3C WoT vocabulary and Clinkz extensions are separate. Clinkz-specific
   binding, storage, compute, or platform metadata uses the Clinkz JSON-LD
   namespace, currently represented with the `cz:` prefix.
3. TD and TM crates own data models, builders, serialization,
   deserialization, validation, and round-trip preservation.
4. Protocol behavior belongs in binding crates.
5. Discovery and Servient/runtime behavior belong in dedicated crates.
6. `no_std + alloc` is a first-class contract where a crate responsibility
   permits it. Filesystems, sockets, threads, async runtimes, and process APIs
   stay behind `std` features or concrete runtime crates.
7. Unknown TD/TM extension fields are preserved through deserialize/serialize
   round trips unless an explicit validation mode rejects them.
8. `base` plus relative form `href` values are supported through shared form
   target resolution helpers.
9. Hot interaction paths must avoid repeated TD tree scans and avoid
   avoidable allocation.
10. Protocol bindings must be replaceable without changing TD, core,
    discovery, or servient crates.

## Workspace Crates

- `clinkz-wot-foundation` (`foundation`):
  Protocol-neutral resource limits, work budgets, monotonic time primitives,
  source timestamps, and generation primitives shared below TD and runtime
  semantics. Root crate supports `no_std + alloc`; it contains no TD vocabulary,
  interaction behavior, binding plan, discovery contract, or host runtime.
- `clinkz-wot-td` (`td`):
  TD/TM data models, builders, serde, validation, and URI helpers. Root crate
  supports `no_std + alloc`.
- `clinkz-wot-core` (`core`):
  Interaction core, handlers, locks, payloads, security, protocol-neutral
  logical and binding plan values, binding request/response data, and host and
  constrained binding contracts. Root crate supports `no_std + alloc`;
  host-erased traits and guard wrappers are feature-gated.
- `clinkz-wot-discovery` (`discovery`):
  Discovery data model, protocol-neutral Directory client contracts,
  publisher/watch client traits, and lazy discovery processes. Data model and
  portable poll contracts support `no_std + alloc`; async traits are behind
  `async`. Directory service composition, storage backends, server-side query
  execution, and endpoint hosting are deferred to a later design.
- `clinkz-wot-protocol-bindings` (`protocol-bindings/core`):
  Shared compilers and indexes for form selection, operation resolution, target
  resolution, security resolution, capability lookup, and URI templates. It
  consumes and produces `core` plan values but does not own binding execution
  traits or Servient registrations. Root crate supports `no_std + alloc`.
- `clinkz-wot-protocol-bindings-zenoh`
  (`protocol-bindings/protocols/zenoh`):
  Optional zenoh planning and runtime binding. Planning layer supports
  constrained builds; Rust zenoh backend is `std`; `zenoh-pico` is constrained
  runtime work.
- `clinkz-wot-servient` (`servient`):
  Application-facing Servient, produced/consumed handles, dispatch, and
  discovery facade. Registry and API abstractions are no_std-capable; host
  builder and concrete runtimes are feature gated.
- `clinkz-wot-codec-cbor` (`codecs/cbor`):
  CBOR payload codec. Root crate supports `no_std + alloc`.
- `clinkz-wot` (`clinkz-wot`):
  Umbrella crate that re-exports the application-facing API. Feature-composed.

## Feature Policy

The main feature groups are:

- `std`: host conveniences and standard-library integration.
- `async`: native async trait surface without implying an executor.
- `zenoh`: concrete Rust zenoh backend; this is a `std` runtime feature.
- `zenoh-pico`: constrained zenoh-pico backend surface.
- `td2-preview`: experimental TD 2.0 data-model additions.
- `cbor`: optional CBOR codec in the umbrella crate.

`std` may imply `async` when the exposed host surface requires async APIs.
`async` must not pull in a runtime such as Tokio by itself.
Host-erased binding registration with `Arc<dyn ServerBinding>` and
`Arc<dyn ClientBinding>` is a host API shape. `no_std + alloc` profiles must be
able to use static binding tables, indexes, or manually driven adapters without
requiring atomics, boxed futures, or host task spawning.

Every feature must be additive. Enabling `td2-preview` must not alter TD 1.1
round-trip behavior when TD 2.0 fields are absent.

Public application-facing types may have host-erased and constrained
representations behind feature gates, but their documented behavior must remain
equivalent for the supported operations. Documentation must not describe
`Arc<dyn ...>` ownership as the only representation of a handle that is also
claimed to be no_std-capable.

`FEATURE-MATRIX-001`: The following matrix is the minimum supported build and
API contract. A crate MAY expose more combinations, but removing a required
surface is a breaking design change. “Values” means owned protocol-neutral
request/result/error types; “poll” means the frozen manually driven traits in
the Implementation Contract; “host” means the object-safe erased integration
surface. Empty cells are unsupported rather than silently reduced APIs.

| Crate | `--no-default-features` | `async`, no `std` | `std` | Other required checks |
| --- | --- | --- | --- | --- |
| `foundation` | Resource/work/time/generation values | Same API; no executor | Host conversion conveniences only | No TD/runtime/protocol dependency |
| `td` | TD/TM values, builders, serde, validation, URI helpers | Same API; no executor | Host conveniences only | `td2-preview` additive |
| `core` | Interaction and plan values, local sync dispatch, poll binding contracts | Native async handler and binding traits without executor | Host locks, erased handlers, and erased binding traits | No higher-layer dependency |
| `protocol-bindings/core` | Plan compilers, capability indexes, form/security/URI helpers | Async compiler adapters without executor | Host compiler conveniences | No execution-trait or concrete protocol ownership |
| `discovery` | Values and poll Directory client/session/watch/publisher traits | Async twins | Host client adapters | No servient, Directory service, or storage-backend dependency |
| `servient` | `StaticServient` and manual progress API | Async twins over caller driver | `ServientBuilder` and host handles | Same validation/security semantics |
| `zenoh` binding | Planning and constrained adapter types | Constrained async adapter when enabled | Rust zenoh backend | `zenoh-pico` does not enable `std` |
| `codecs/cbor` | Bounded codec and incremental/poll decode | Same value API | Host I/O conveniences | No runtime dependency |
| umbrella | Re-export selected constrained surfaces | Re-export selected async surfaces | Feature-composed host API | No-default build is useful |

`std` MAY imply `async` only in crates whose host row exposes async operations.
The Cargo feature graph and public-surface compile fixtures MUST implement this
table. Unsupported combinations MUST fail through an explicit Cargo feature
constraint or be absent from the published matrix; they MUST NOT compile an
empty placeholder surface.

### Crate Dependency Direction

`CRATE-DEPS-001`: The normal dependency graph is acyclic and points in this
direction:

```text
foundation <- td
foundation + td <- core
foundation + td + core <- protocol-bindings/core <- protocol bindings
foundation + td + core <- discovery
foundation + td + core + protocol-bindings/core + discovery <- servient
core <- payload codec crates
all selected public components <- clinkz-wot
```

The diagram expresses allowed dependencies, not a requirement that every edge
exist. `foundation` MUST NOT depend on TD vocabulary, core, discovery, servient,
payload codecs, or a protocol binding. `td` MAY depend on `foundation` and MUST
NOT depend on core, discovery, servient, or a protocol binding.
`core` MUST NOT depend on discovery, servient, or a concrete protocol binding.
`protocol-bindings/core` MUST NOT depend on servient or a concrete protocol
binding. `discovery` MAY depend on `td` and protocol-neutral core value types,
but MUST NOT depend on servient, a concrete transport, a Directory service, or a
storage backend. `servient` composes the protocol-neutral crates and
registration traits; concrete protocol crates MUST NOT depend on servient merely
to implement a binding. Directory service and storage crates introduced by a
future design may consume the client-facing value contract, but they MUST NOT
move service behavior into `discovery` or become dependencies of `servient`.
The umbrella crate is the only crate expected to depend on every selected engine
component.

Feature unification MUST NOT make a lower layer acquire an optional dependency
on a higher layer. Shared types discovered during implementation go in the
lowest crate that owns their semantics; they are not duplicated to avoid an
otherwise invalid dependency edge. `foundation` is deliberately narrow: moving
an item there requires it to be independent of TD vocabulary, interaction error
semantics, binding execution, discovery, and lifecycle composition.

## Data Contract

`clinkz-wot-td` owns protocol-neutral TD/TM representation. Its boundary is:

- Build TD and TM documents through typed builders.
- Deserialize and serialize W3C TD 1.1 documents.
- Preserve unknown extension fields and JSON-LD context data.
- Validate TD/TM structure and defaults.
- Provide URI and operation types used by runtime crates.
- Preserve `base`, relative `href`, `security`, `scopes`, `op`,
  `contentType`, `response`, `additionalResponses`, `subprotocol`, and
  extension fields.

`AbsoluteUri` is exported at the crate root because discovery and servient APIs
use it directly.

TD/TM crates must not include concrete transport logic. Protocol-specific
metadata can be represented as extension fields, but interpretation belongs to
binding crates.

### Clinkz Extension Vocabulary

Clinkz-specific terms use the `cz` JSON-LD prefix. The canonical namespace IRI
for current generated documents is:

```text
https://clinkz.io/ns/wot#
```

The TD/TM data model preserves incoming `@context` values exactly in
round-trip mode, including alternate user-provided prefixes for the same IRI.
Builders that emit Clinkz extensions should add or reuse a `cz` context mapping
to the canonical namespace IRI only when that prefix is absent or already maps
to the canonical IRI. If an incoming document maps `cz` to another IRI,
round-trip mode preserves the incoming mapping, while builders that need to add
Clinkz terms must choose a non-conflicting prefix or report a structured
context-conflict error in strict emission modes. Binding and platform crates may
interpret terms in the canonical namespace, but W3C WoT terms remain modeled
independently from Clinkz extensions.

Prefix selection is deterministic. When a builder must emit a Clinkz term and
`cz` is unavailable, permissive emission tries `cz1`, `cz2`, and so on until it
finds a prefix that is absent from every object context in the active `@context`
array or map. It appends the new mapping to the last object context when that is
possible without changing existing mappings; otherwise it appends a new object
context at the end of the `@context` array. Strict emission modes fail with a
structured context-conflict error instead of inventing a new prefix unless the
caller explicitly enables prefix allocation. Duplicate incoming mappings are
preserved in round-trip mode and rejected only by validation modes that choose
to enforce context canonicality.

`JSONLD-PREFIX-001`: Prefix allocation builds one bounded set or sorted index of
occupied prefixes while the context is already being visited. It then chooses
the smallest available `czN` without rescanning every context object for each
candidate suffix. Work is O(context bytes + context entries + tested suffixes),
all prefix bytes and entries consume admission work units, and exhaustion of the
configured context-entry, string-byte, or work budget returns `LimitExceeded`.
An adversarial context containing `cz`, `cz1`, ... `czN` MUST NOT turn prefix
selection into quadratic repeated context scans.

### TD Source and JSON-LD Processing

TDs and TMs may come from untrusted devices, directories, files, or application
inputs. Parsing and validation must treat every document as untrusted until it
passes the validation level selected by the caller.

JSON-LD handling is deterministic by default:

- Known W3C and Clinkz context IRIs are resolved from an embedded or
  application-provided context registry.
- Parsing, validation, consuming, exposing, and discovery must not fetch remote
  JSON-LD contexts implicitly.
- Host applications may opt into remote context loading only through an
  explicit resolver with size, timeout, redirect, cache, and allow-list policy.
- A context loading failure is a structured validation or resolution error, not
  a reason to silently drop semantic terms.

TD ingestion APIs must keep source metadata outside the TD data model: source
URI, retrieval time, transport security status, directory revision, signature or
digest information when available, and validation level. Runtime decisions that
depend on source trust use this metadata explicitly instead of encoding it as
W3C vocabulary.

The design exposes this separation through a source envelope rather than by
adding source fields to `Thing`. The public envelope shape is
`ThingDocument<T, S>`, where `T` is the document value and `S` is the owned source
metadata type. Type aliases
`TdDocument = ThingDocument<ThingDescription, TdSourceInfo>` and
`TmDocument = ThingDocument<ThingModel, TmSourceInfo>` may be provided for
readability. A one-parameter `ThingDocument<T>` alias may exist only in API
contexts where the source metadata type is unambiguous. `TdDocument` is the
lossless source and admission contract; a compiled runtime handle is not
required to retain it. `TmDocument` is a tooling contract for TM construction,
validation, and round-trip preservation.

`ThingDocument<T, S>` has a stable minimum API:

- `thing()` and `thing_mut()` access the contained TD or TM value.
- `source_info()` and `source_info_mut()` access the owned metadata.
- `into_parts()` returns the contained value and source metadata without loss.
- `from_parts(value, source_info)` constructs an envelope from owned parts.
- `map_thing()` transforms the contained document value while preserving source
  metadata type `S`, unless the transformation explicitly records new derivation
  evidence through a differently named API that also transforms the metadata.

`TdSourceInfo` records, when available, source URI, retrieval time, source kind,
transport security status, directory identity, directory revision, lease or
freshness data, publisher identity, signature or digest evidence, validation
level, admission policy id, and whether the TD was application-supplied,
discovered, fetched, derived, or generated. Validation/admission APIs return a
new `TdDocument` value whose metadata records the validation level, policy id,
source evidence, and admission result used for that decision. They do not mutate
the TD value to encode trust.

Runtime admission, planning, consumption, and exposure accept TD values,
`TdDocument` values, or explicitly named TD source envelopes. The
Scripting-compatible `produce(init)` entry point is an initialization API: it
may accept `ExposedThingInit`/Partial TD input, but it must expand that input to
a TD candidate before runtime planning and must not admit a Thing Model or an
unresolved template as a runtime contract. TM-to-TD derivation remains a
separate tooling API. Rust-native discovery and directory APIs return TD
candidates as `TdDocument`. In this document, "discovered TD", "TD candidate",
or "matching TD" in a Rust-native API means `TdDocument` unless the text
explicitly says "bare TD". The envelope provides accessors and conversions for
the contained TD value so Scripting-compatible methods can expose the same TD
content while preserving source metadata for Rust callers.

`Servient::consume(td)` remains the Scripting-compatible Consumer convenience
entry point. It treats the input as application-supplied TD data with default
local/unknown source metadata and then applies the configured Servient admission
policy. `Servient::produce(init)` is the Scripting-compatible Producer
convenience entry point. It accepts an `ExposedThingInit`/Partial TD shape and
performs only binding-independent expansion: `@context`, `title`, version
metadata, affordance defaults, and explicitly configured security defaults that
do not depend on a concrete binding endpoint. Binding-dependent expansion, such
as generated forms, generated `href` values, binding-specific security material,
and endpoint collision checks, happens during `expose()` against the server
binding registration snapshot captured by the handle. Rust-native entry points
such as `consume_document(document)`, `produce_document(document)`, and
`produce_td(td)` accept complete TD documents or TD source envelopes directly,
preserving discovery metadata, directory revision, signature, freshness, and
validation state for trust decisions. They do not accept Thing Model documents directly;
applications or tooling must first derive a TD when all required instance values
are available. Runtime code that needs source trust must consume the envelope
metadata; it must not infer source trust from fields inside the W3C TD model.

### Document and Runtime Representation

`DOC-RUNTIME-001`: Lossless document ownership and compiled runtime ownership
are separate contracts. `ThingDocument` owns or references the TD-family source,
unknown extensions, JSON-LD spelling, member order where preserved, and source
evidence. `CompiledThing` is the architectural runtime role containing only the
identity, affordance/operation indexes, effective form metadata, compiled
schemas, security plans, binding references, and diagnostics selected by its
resource profile. A runtime handle MUST NOT retain a complete generic JSON DOM
or lossless `TdDocument` merely because admission started from one.

Admission policy chooses `SourceRetention::{None, MetadataOnly, RawDocument,
LosslessDocument}` or an equivalent explicit representation. `None` retains
only runtime-required trust decisions and a redacted source identity;
`MetadataOnly` retains bounded `TdSourceInfo`; `RawDocument` may retain or
reference the original serialized bytes; `LosslessDocument` retains the full
document API. Discovery/Directory storage and tooling MAY default to lossless
retention. Constrained and allocation-sensitive runtime profiles default to
`MetadataOnly`. Retention bytes are charged separately from compiled runtime
bytes.

`DOC-RUNTIME-002`: Parsing MAY use a generic JSON value tree only as bounded
temporary admission storage. Streaming, typed, raw-value, span/index, and
arena-backed parsers are conforming. Unknown extensions can be retained as raw
serialized values, source spans, compact typed values, or lossless side-table
entries. If source retention is disabled, unknown extensions not required by a
selected binding MAY be released after validation and plan compilation; this
does not change TD parsing or round-trip behavior of the `ThingDocument` API.

`DOC-RUNTIME-003`: The effective TD is a logical view over the admitted source
and compiled side tables. Defaults, inherited security, resolved targets, and
effective operations MUST NOT require a second complete TD tree. Borrowed or
streaming introspection is the baseline. An explicitly named owned
materialization API MAY construct a complete effective TD, charges temporary and
result bytes, and does not cache it by default. A profile MAY disable owned
materialization while still providing Scripting-compatible serialized or
borrowed introspection.

### Validation Levels

The target validation modes are:

- Round-trip:
  Preserve all known and unknown fields with minimal rejection.
- Basic TD 1.1:
  Enforce required fields, defaultable fields, structural shape, and known enum
  values.
- Full TD 1.1:
  Enforce cross-field references, security definition references, form
  operation compatibility, and URI/base validity.
- Binding preflight:
  Validate a TD or selected form for a specific binding without contaminating
  the TD data model.

Validation errors must be structured enums, not only strings, when callers can
reasonably recover or report a precise cause.

### Thing Model Scope

Thing Models are supported as first-class TD-family documents for construction,
serde, validation, and round-trip preservation, but they are not directly
consumable or exposable runtime contracts. Runtime planning starts from a Thing
Description instance. A Thing Model must first be instantiated or derived into a
TD by application or tooling code before `consume()` or `produce()` uses it.

The TM design target includes:

- preserving TM-specific vocabulary and extension fields, including
  `tm:optional`;
- representing TM placeholders without treating unresolved placeholders as TD
  runtime values;
- validating TM structural constraints separately from TD instance constraints;
- preserving import, extension, composition, and model-version metadata through
  round trips;
- providing builder support for model documents and for deriving TD candidates
  when all required instance values are supplied.

TM validation levels mirror the TD validation profile names but apply TM rules:
round-trip mode preserves unresolved templates with minimal rejection, basic TM
validation checks structural shape and known vocabulary values, and full TM
validation checks model references, required model metadata, placeholder
consistency, and composition/import references that are available to the caller.
Remote model imports are never fetched implicitly; they use the same explicit
resolver policy as JSON-LD contexts.

### Payload and Schema Validation

The engine must make payload validation an explicit boundary. It must never
silently accept a payload shape that contradicts the selected operation,
affordance schema, or media metadata when validation is enabled for that path.

Validation responsibilities:

- Form planning resolves operation input, output, event data, subscription, URI
  variable, form `response` media metadata, and `additionalResponses`
  schema/media metadata associated with each operation.
- Outbound Consumer calls validate caller payloads against the operation input
  schema before the binding sends them when the selected validation profile
  requires payload validation.
- Inbound Producer dispatch validates payload media type, content coding,
  operation/schema compatibility, and caller-provided URI variables before user
  handlers run when the selected validation profile requires payload validation.
- Binding request/response interactions are checked against primary `response`
  and `additionalResponses` metadata before being reported as successful outputs
  in strict profiles. Observable-property samples and event notifications are
  checked against the active subscription's affordance data schema: property
  value schema for property observations, and event `data`, `dataResponse`,
  `subscription`, or `cancellation` schemas as appropriate for the selected
  event operation. They are not validated against form response metadata unless
  the protocol actually delivers a subscription-start or teardown response.
- Payload validation is direction-sensitive. Payloads created by local callers or
  delivered to Producer handlers may fail closed when they contain data that is
  incompatible with the operation schema. Consumer-side responses must preserve
  TD 1.1 extensibility: object or array members that are not described by the TD
  data schema are retained and exposed to the caller unless the selected
  operation or an explicit Rust extension policy requires a narrower application
  type. A strict Consumer response profile may reject malformed required fields,
  incompatible primitive types, invalid media metadata, and failed
  `additionalResponses` checks, but it must not reject a successful response only
  because it contains extra object members or array entries allowed by TD 1.1
  response compatibility rules.
- An `additionalResponses` entry with `success: false` maps to a structured
  operation error, not to a successful `InteractionOutput`. The public
  `CoreError` retains only bounded plan, binding, phase, cause-code, and
  redacted-cause context; it does not retain the response payload, schema, or
  raw protocol status or error. An entry with `success: true` may be reported as a
  successful alternative output after schema/media validation. When `success`
  is absent, TD 1.1 defaulting treats it as `false`.
- Property `readOnly`, `writeOnly`, and `observable` defaults are enforced
  during form planning and dispatch; unsupported operations fail before handler
  invocation or binding I/O.

Validation profiles may trade cost for strictness:

- Fast path:
  Validate operation, media metadata, required envelope shape, and any caller
  input checks required by the active API compatibility mode.
- Strict schema:
  Validate JSON-compatible payloads against TD data schemas. Outbound and
  Producer-inbound payloads reject unknown or incompatible payload shapes
  according to the selected schema policy. Consumer responses reject incompatible
  schema violations but preserve TD-compatible extra response data unless the
  caller selected an explicitly narrower Rust extension policy.
- Binding delegated:
  A binding may perform protocol-specific validation, but it must report the
  result through structured errors and must not replace protocol-neutral
  operation and security validation.

Schema validation must be codec-aware. JSON Schema-like validation applies only
after a codec has produced a typed value or when the payload is already typed.
Opaque byte payloads carry media metadata and can be passed through only when
the caller selected a profile that allows deferred validation.

Security parameters carried in the payload body are handled as security-managed
wire fields, not as application-owned data. For outbound Consumer calls, the
engine validates the caller-provided application payload before credential
injection using an application-facing schema view where committed body-location
security fields are excluded or treated as provider-managed insertions. After the
effective security branch is selected and committed, the `SecurityProvider`
injects the body fields required by that branch, and strict profiles validate the
resulting wire payload against the effective TD schema and media metadata before
binding I/O. For inbound Producer dispatch, the binding supplies the wire
payload and only transport-native `TransportAuthMaterial`. After route and
generation validation, the core security pipeline decodes the wire payload once
through `PayloadCodec`, applies the compiled body-security plan through
`BodyAuthProjector`, and produces both body `AuthMaterial` and an
application-facing projection. Extraction, validation, and field
removal/redaction share that decoded representation or an overlay and MUST NOT
decode the payload a second time. Rust diagnostic APIs may expose the raw wire
payload only through deliberately named surfaces that preserve the same
secret-redaction and audit requirements as other security diagnostics.

Payload validation policy is selected separately from TD/TM document validation.
The host `ServientBuilder` sets the default policy for produced dispatch,
consumed interactions, discovery admission, and directory publication. `produce`
and `consume` may accept handle-level overrides, and `InteractionOptions` may
request a stricter per-call policy. A per-call option may not weaken a stricter
handle or Servient policy unless the caller explicitly uses an unsafe or
diagnostic escape hatch that is documented outside the Scripting-compatible API.
That escape hatch is not available through Scripting-compatible methods, is
disabled by default, and must be gated by a deliberately named Rust API or
feature such as `diagnostic-validation-bypass`. Using it produces an observable
diagnostic event or status item that records the Thing id, affordance target,
operation, validation checks bypassed, caller-selected reason, and plan id when
known, without exposing secrets. It may only bypass payload/schema checks for
diagnostic capture or interoperability workarounds; it must not bypass operation
support, form selection, security resolution, credential/scopes checks, URI
variable validation required for routing, or binding preflight checks needed to
avoid unsafe protocol behavior. Production profiles may disable this API
entirely.

The default Scripting-compatible host policy validates caller-provided
interaction payloads against the applicable TD data schema before outbound
Consumer sends and before Producer handlers run. Consumer responses preserve TD
1.1-compatible extra data as described above, while still rejecting malformed
required fields, incompatible primitive types, invalid media metadata, and
failed `additionalResponses` checks when the selected response profile requires
them. Directory admission and explicit validation APIs use strict schema
validation by default. Rust-native diagnostic or performance profiles may use a
weaker fast path for payload schemas only when that weaker behavior is selected
outside the Scripting-compatible API. Constrained runtimes may choose a cheaper
default, but they must document which payload checks are active and fail closed
for operations whose required validation cannot be performed.

`VALIDATE-COMPILE-001`: Repeated interaction validation uses compiled,
immutable validators or indexes rather than interpreting the TD schema tree on
every payload. Validator compilation is charged to eager admission or a bounded
lazy plan slot. Validators share schema nodes and media dispatch tables across
forms when their semantics are identical. A profile may select structural-only
validation, but it must not label an interpreted full-tree walk as a hot-path
optimization.

`VALIDATE-REUSE-001`: One interaction decodes a payload into a typed
representation at most once per codec unless the caller explicitly requests a
second independent representation. Application-view validation,
provider-managed body-field injection/removal, wire-view validation, handler
delivery, and response classification reuse that representation or validate
only the changed fields. Codec scratch, typed-value caches, and encoded output
are independently budgeted and released at the earliest safe phase.

## Interaction Core

`clinkz-wot-core` owns protocol-neutral interaction semantics.

Key types:

- `ThingId` and `CorrelationId` identify Things and protocol request/response
  matching tokens.
- `ActionInvocationRef` identifies a specific action execution when a protocol
  exposes asynchronous action status, query, or cancellation semantics.
- `AffordanceTarget` identifies Thing, property, action, or event targets.
- `InteractionInput`, `InteractionOptions`, `InteractionOutput`, and
  `InteractionStatus` carry interaction payloads, URI variables, principals,
  media hints, and result metadata.
- `Payload` and `PayloadCodec` provide media-aware payload handling.
- `WotLock<T>` is an internal `std` host shared-lock handle, not a frozen
  cross-crate public type. Constrained state uses uniquely owned cells or
  generation-bearing runtime slots protected by the caller's scoped
  critical-section boundary; the no-default surface does not make a lock
  cloneable by requiring `Arc` or atomics.
- `EventBroker`, `Subscription`, and `SubscriptionGuard` support event and
  observable-property delivery.
- `SecurityProvider` and `CredentialStore` provide inbound verification and
  outbound request credential application.

### Concurrency and Reentrancy

`CONCUR-LOCK-001`: Engine lock ordering is registry, Thing/handle state,
handler or subscription slot, binding-local state, then diagnostic/status
state. Code MUST NOT acquire an earlier class while holding a later class.
Bindings MAY define internal locks after binding-local state, but MUST document
their order. Credential providers and application callbacks are user code, not
an engine lock class.

`CONCUR-USER-001`: The engine MUST clone, copy, move, or otherwise own the
minimum dispatch state and release every engine lock and critical-section guard
before invoking a handler, credential provider, event callback, readiness
driver, transport callback, or user-supplied status sink. User code MAY reenter
the Servient unless the individual API documents a narrower rule. Reentrancy
MUST observe the same published state as a new concurrent call and MUST NOT
depend on a recursively acquired engine lock.

`CONCUR-LIN-001`: Public lifecycle and replacement APIs document a
linearization point. Handler replacement linearizes when the new slot is
published; an already selected dispatch retains the old handler. Subscription
stop linearizes when the guard registry marks the subscription stopping, after
which no new sample is admitted for that subscription. `expose()` linearizes at
the registry transition to serving. `destroy()` linearizes at the transition to
draining, after which no new dispatch is admitted. Event publication racing
with stop is either admitted before that stop point or rejected/loss-accounted
after it; it is never delivered without an owning live subscription record.

`CONCUR-CRIT-001`: A constrained critical section performs only bounded table,
slot, counter, or state transitions. It MUST NOT parse, allocate, validate a
schema, expand a URI, call user code, poll transport progress, or iterate a
collection whose maximum work is not fixed by the selected static profile.

### Produced Thing State

`ExposedThing` is a concrete core type that stores a TD plus per-affordance
handler slots. It is not the application-facing handle.

Handlers are sync-primary:

- Property: read, write, observe, unobserve.
- Action: invoke, query, cancel.
- Event: subscribe, unsubscribe.
- Thing-level and collection operations:
  read all properties, write all properties, read multiple properties, write
  multiple properties, observe all properties, unobserve all properties, query
  all actions, subscribe all events, and unsubscribe all events.

Each operation has a synchronous handler trait. With the `async` feature, each
operation also has an async twin. Sync handlers are the zero-allocation hot path
for bounded local work. Async or poll/step handlers are opt-in for I/O-bound
behavior.

`HANDLER-API-001`: Every operation-specific handler receives an owned or
call-lifetime-borrowed `HandlerContext` and the operation input appropriate to
the compiled plan. The context exposes Thing id/generation, target, operation,
plan id, correlation id, verified `Principal`, validated URI variables,
deadline/cancellation view, and binding metadata safe for applications. It never
exposes credentials, raw `AuthMaterial`, provider-managed body fields, or a
mutable registry. Read and query operations return `InteractionOutput`; write,
cancel, subscribe, and unsubscribe operations return an operation-specific
status or output required by the selected response schema. Observe/subscribe
handlers return a Producer-side guard or acceptance value; notification samples
are published through the broker/emission APIs rather than by returning an
unbounded stream from the handler.

The host sync registration accepts owned `Send + Sync + 'static` handler
objects. The host async twin returns one owned, cancellation-aware future whose
output has the same operation result; it does not borrow a handler-slot lock.
Constrained registration accepts caller-owned static slots or lifetime-bounded
handler references and does not require `Send`, `Sync`, `Arc`, or boxed futures
unless its selected execution environment does. Each concrete setter name fixes
one operation and therefore cannot register a handler with an incompatible
input/output type. Handler errors are converted to `CoreError` without losing
their bounded application error code and redacted message.

`HANDLER-SUB-001`: A Producer subscribe/observe handler authorizes and performs
application setup for one subscription request. Success is not published until
both the handler acceptance and binding/local subscription guard installation
succeed. The corresponding unsubscribe/unobserve handler is invoked at most once
for an explicitly matched active subscription when the compiled teardown
operation requires application behavior. Failure rolls back guards already
created for that start. Repeated wire teardown observes the retained terminal
outcome and does not invoke application teardown twice.

Per-affordance handler setters replace the current slot. The last registered
sync or async handler wins for that operation. Replacing a handler affects only
dispatches that select the slot after the replacement is published. A dispatch
that has already selected a handler keeps its cloned handler reference and runs
to completion, cancellation, or timeout according to the active drain policy.
Removing or clearing a handler is allowed only through an explicitly named API;
after removal, a matching request fails with structured
`UnsupportedOperation` even though the frozen TD still advertises the
operation. The error includes the Thing id, affordance target, operation, and
selected plan id when known.

`HANDLER-STORAGE-001`: Handler storage is proportional to operations admitted or
registered, not the Cartesian product of every handler trait and affordance.
Profiles may use compact operation bitsets plus sparse slot tables, fixed dense
tables only when their measured footprint fits the static profile, or generated
static dispatch. Unsupported operations consume no dynamic handler object or
async wrapper. Sync-only profiles do not reserve async handler storage.

Handler slot publication is atomic at the slot level. The implementation may
use locks, atomics, epochs, or caller-owned constrained storage, but dispatch
must never hold a global registry, binding lock, route guard, or long-lived
critical section while user code runs. If a setter races with dispatch, either
the old or the new handler is selected for that one dispatch; the same dispatch
must not observe a partially updated slot or switch handlers mid-call. Async
dispatch clones or otherwise owns the selected handler reference before the
first await.

Thing-level operation handlers are separate from per-affordance handlers. In
Scripting-compatible Producer mode, `readmultipleproperties`,
`readallproperties`, and `writemultipleproperties` requests may use the
registered per-property handlers when no explicit Thing-level handler is
registered. This mirrors the Scripting API server-side algorithms: the protocol
transaction remains one bulk request and one bulk response, while the
application-facing Producer implementation may service the request by invoking
the individual property behavior.

Producer-side per-property aggregation has an explicit execution policy. The
portable default is sequential execution in a deterministic property order,
because it is predictable for devices where property order is meaningful.
`readallproperties` uses the preserved TD property document order.
`readmultipleproperties` and `writemultipleproperties` use the caller-supplied
property order when the inbound protocol represents an ordered property list. If
the inbound representation is an unordered map, the portable fallback is the
preserved TD property document order filtered to the requested properties.
Applications may select concurrent aggregation with a bounded concurrency limit
when the targeted properties are independent. Every aggregation policy must
define authorization behavior, fail-fast versus structured partial-result
reporting, deadline and cancellation handling, and whether later properties
continue after one property fails.

The Scripting-compatible aggregation path must not let one blocking handler hold
global registries, binding locks, route guards, or critical sections. Synchronous
handlers are expected to finish promptly and avoid unbounded I/O waits. Blocking
or I/O-bound property work should use async handlers, a bounded concurrent
aggregation policy, or an explicit Thing-level bulk handler. When a handler
exceeds the configured deadline or observes cancellation, the following
execution contract applies.

`HANDLER-CANCEL-001`: A synchronous handler call is cooperative and
non-preemptible. The engine MUST check cancellation and deadline state before
entering it and after it returns, but MUST NOT claim that it can stop, bound, or
time out user code that does not return. A deadline crossed while the handler is
running makes its eventual result late; the runtime discards or reports that
result according to the active drain policy. It does not imply that the handler
was terminated.

`HANDLER-CANCEL-002`: An operation that requires an enforced execution deadline
MUST use an async handler that cooperates with cancellation or a poll/step
handler with a bounded per-step work contract. Aggregate timeout and
cancellation statuses describe the aggregate request and response lifecycle,
not preemption of synchronous user code. API documentation MUST state this
distinction.

Producer-side aggregation has an explicit response contract. A fail-fast policy
returns the first structured operation error with the Thing id, target property,
operation, and plan id. A structured partial-result policy returns a
Thing-level aggregate status that preserves each property success or failure
without flattening heterogeneous payloads into a single opaque value. Before the
binding reports a successful inbound response, the aggregate output is checked
against the selected Thing-level form's `response` or successful
`additionalResponses` metadata when the active validation profile requires
payload validation. Partial failures are represented as structured operation
errors or failure-status responses according to the selected TD response
metadata; they must not be reported as successful `InteractionOutput` values
unless the selected TD form explicitly defines that status shape as a successful
response.

For other Thing-level operations, and for Rust extension modes that disable the
Scripting-compatible aggregation policy, a Thing-level TD form with no matching
Thing-level handler or explicit producer-side aggregation policy fails with an
unsupported-operation error. The Producer path must not silently fan out a
Thing-level inbound request to multiple per-affordance handlers unless the
applicable compatibility mode or application configuration defines that
aggregation policy and its ordering, partial failure, cancellation, deadline,
and authorization behavior.

Handler dispatch must not hold global registries, binding locks, or long-lived
critical sections across user code. Async handler dispatch must clone the
selected handler reference out of shared state before awaiting.

### Consumed Thing State

`ConsumedThing` is a concrete core type that stores compact effective TD
metadata plus interaction indexes and plans. Source document retention follows
`SourceRetention`; it is not part of the required runtime representation. In the
host async profile, plans may refer to shared
`Arc<dyn ClientBinding>` references. In constrained profiles, the same
protocol-neutral plans must be usable with a poll-based binding table, static
binding indexes, or owned adapters. The plan representation must not require
boxed futures or `Arc` in order to compile for `no_std + alloc`. All per-call
context is carried in `BindingRequest`, so a single client binding instance can
serve many consumed Things.

`consume(td)` precomputes protocol-neutral work shared by calls:

1. Validate required TD shape for consumption.
2. Build an affordance index by target and operation.
3. Resolve operation defaults.
4. Resolve form targets against Thing `base`.
5. Resolve effective security metadata and scopes.
6. Ask registered client bindings whether they statically support each
   candidate form.
7. Store ordered logical candidates and lightweight binding references.

`PLAN-COST-001`: Planning MUST use a two-level representation. A logical plan
contains protocol-neutral metadata compiled once per form. A binding plan
refers to that logical plan and stores only binding identity, static capability
results, and binding-specific state that cannot be shared. Implementations MUST
NOT duplicate a full logical plan for every form-binding pair.

`PLAN-COST-002`: Heavy binding-specific compilation MAY be eager, lazy, or
hybrid. The selected policy MUST preserve candidate order and failure semantics,
MUST use the active resource budget, and MUST expose whether a failure occurred
during admission or first use. Lazy caches MUST be bounded and generation-aware
when binding registrations change.

`PLAN-COST-003`: Plan construction MUST reject input that exceeds the configured
form, binding-candidate, schema-node, security-branch, or compiled-byte budget.
It MUST return a structured limit error rather than silently omit candidates.

`PLAN-INDEX-001`: Separate generation-bearing capability indexes are built for
the captured client and server registration snapshots. They are keyed first by
resolved URI scheme and then, where declared, by protocol, subprotocol,
operation class, media family, and Producer contribution role. Server indexes
cover both form contribution and application-supplied-form ownership probes. A
form probes only registrations returned by the applicable index, including
declared wildcards. Capability declarations are side-effect-free and MAY
over-approximate support, but MUST NOT omit a key for which
`supports(candidate)` could return a supported result. An undeclared wildcard
is allowed only when admitted as an explicit wildcard candidate. Normal planning
complexity is O(`f + p + c`), where `p` is the number of indexed probes and `c`
is the supported candidate count; O(`f * b`) is only an admitted wildcard worst
case and remains bounded by `max_binding_probes_per_admission`.

`PLAN-LAZY-001`: Admission compiles compact logical metadata required for safe
selection. Heavy schema validators, protocol-specific route/client state, and
other derived artifacts MAY use eager, bounded lazy, or hybrid compilation.
Profiles MUST state which artifacts are eager. Lazy compilation uses a bounded
generation-aware slot/cache, publishes one immutable result for concurrent first
use, and reports first-use compilation failure distinctly. A static Producer MAY
select all-eager compilation; a runtime MUST NOT compile unused form-binding
combinations solely for API convenience.

`PLAN-CACHE-001`: Concurrent first use of the same plan key and dependency
generation is a single-flight operation: at most one compiler performs the
work, and waiters either observe its immutable result or receive bounded
backpressure. Compilation, provider callbacks, and binding callbacks MUST NOT
run while holding a registry-wide or cache-eviction lock. A deterministic
failure may be retained as a bounded negative entry for the same input and
dependency generations; transient, cancelled, resource-exhausted, and
deadline-dependent failures are not made permanent. Binding, provider,
credential, schema-registry, or policy generation changes invalidate by a
generation comparison or an equivalent O(1) epoch publication. They MUST NOT
synchronously scan every Thing or plan on the publishing thread. Stale entries
are reclaimed incrementally within explicit cleanup work and byte budgets.
Eviction never removes a referenced result and never causes more than one
concurrent recompilation for the same current key.

The outbound hot path should only:

1. Look up the target and operation plan.
2. Merge caller `InteractionOptions` into the request.
3. Apply outbound security material, including credential-derived URI, header,
   body, or protocol metadata.
4. Expand URI templates against caller variables and security-provided URI
   variables.
5. Invoke the selected client binding.

Per-call linear scans through the TD tree are not part of the target design.

`PLAN-REQUEST-001`: Static plan metadata remains in the immutable plan and is
referenced by compact plan, target, affordance, and binding slot ids. Per-call
requests own only varying data: payload, URI-variable values, cancellation and
deadline state, correlation/idempotency values, committed security material,
and protocol status. They MUST NOT clone static target strings, schemas,
security expressions, response metadata, or extension maps into every request.
Host erased calls may retain a shared plan reference; constrained calls use a
generation-bearing slot reference.

Consumed and exposed handles can expose two logical TD views when source
retention is enabled and the input document and effective runtime contract
differ:

- a preserved document view that retains incoming member order where practical,
  unknown extensions, JSON-LD context spelling, source metadata, and round-trip
  fidelity;
- an effective runtime view used for planning, validation, dispatch, and
  Scripting-compatible introspection, with TD defaults, relative targets,
  security inheritance, and other effective metadata resolved or made available
  through structured accessors.

`TD-MEM-001`: Two logical views do not require two complete independent TD
trees. When retained, the source document is the authoritative lossless
representation. Effective defaults and resolutions MUST use shared immutable
subtrees, indexes, overlays, or side tables rather than an always-resident cloned
tree. Compiled plans MUST share schema, extension, URI-template, and security
metadata where ownership permits.

`TD-MEM-002`: A constrained profile MUST document whether
`thing_description()` returns a borrowed view, a shared snapshot, or constructs
an owned value. It MUST NOT require two complete TD copies as an undocumented
baseline cost. Owned materialization is charged to the active byte budget.

`thing_description()` on `ConsumedThingHandle` and `ExposedThingHandle` returns
the effective runtime TD view used by the handle, matching the Scripting API
expectation that a consumed or exposed Thing is introspected through its
validated/expanded contract. Rust extension methods expose retained source
documents and source metadata when the selected `SourceRetention` mode provides
them; otherwise they return a structured unavailable status. Returning or
serializing the effective TD must not mutate retained source data.

## Form Selection and Binding Plans

Form selection is protocol-neutral and shared by all bindings.

### Effective Form Metadata

For every candidate form, the engine resolves:

- target context and operation;
- effective `op` values, including TD default operations;
- absolute target URI template, applying Thing `base` to relative form `href`;
- `contentType`, defaulting according to TD rules when absent;
- expected response media metadata;
- `subprotocol`;
- URI variables from Thing, affordance, and form scopes;
- effective security references and form-level scopes;
- security-scheme URI parameters that must participate in URI-template
  expansion and name-conflict validation;
- original TD form index and stable compiled plan id;
- extension fields preserved for binding-specific interpretation.

Form-level `security` overrides Thing-level `security`. A form without
`security` inherits Thing-level security.

### Thing-Level Forms and Meta Operations

Form planning treats the Thing root as a first-class form context in addition
to property, action, and event affordances. Thing-level forms are required for
TD operations whose context is the Thing or an affordance collection, including:

- `readallproperties`, `writeallproperties`, `readmultipleproperties`, and
  `writemultipleproperties`;
- `observeallproperties` and `unobserveallproperties`;
- `queryallactions`;
- `subscribeallevents` and `unsubscribeallevents`.

The plan model must not force these operations into a synthetic property,
action, or event. It stores a target context such as Thing root, property name,
action name, event name, or collection-level operation. Strict caller selection
by original form index must account for the form's context, because form index
values are scoped to the array that contained the form.

When a Scripting API-compatible method maps to a Thing-level TD operation, the
engine selects from Thing-level forms first. Per-affordance fan-out helpers are
separate Rust extensions and must not be used to pretend that a missing
Thing-level bulk form exists. For Consumer bulk property methods,
`readallproperties`, `readmultipleproperties`, and `writemultipleproperties`
are selected from the TD root `forms` array, and a caller-supplied `formIndex`
is interpreted as an index into that root array. The `formIndex` for a property,
action, or event form array must never be reused to select one of these
Thing-level bulk operations.

### Candidate Ordering

Candidate order is stable:

1. Forms are considered in TD document order within the selected form context.
2. A caller-supplied option may narrow acceptable media type, subprotocol,
   operation, binding protocol, or original TD `formIndex`.
3. Static binding support is evaluated without changing document order.
4. Per-call security applicability is evaluated after caller options and
   credentials are available. A candidate that cannot satisfy its effective
   security expression for this call is skipped only in selection modes that
   allow fallback; strict `formIndex` or strict security selection must fail
   closed instead of silently choosing another form.
5. The first supported and security-applicable plan is selected unless the
   caller requests a stricter selection mode.

Credential-driven fallback must never weaken security silently. In particular,
falling back from an authenticated form to an explicit `nosec` form is allowed
only when the caller or selection policy permits public-form fallback.

Selection and execution errors must remain distinct. Candidate selection errors
must distinguish:

- affordance missing;
- operation unsupported by the affordance;
- forms exist but no form supports the operation;
- target resolution failed;
- no registered binding supports the resolved form;
- security credentials or provider missing.

`PLAN-BOUND-001`: Each compiled target/operation plan has an explicit
`max_candidates_per_operation` admission limit covering the ordered
form-binding candidates that one call may inspect. The gateway default is 32.
Admission rejects a plan that exceeds the limit; it MUST NOT retain a larger
vector and rely on callers to avoid the slow path. Per-call fallback examines
each admitted candidate at most once, and security work for all examined
candidates shares the same `max_provider_probes_per_interaction` budget rather
than resetting the budget for every fallback. Strict form or binding selection
may reduce the scan but never bypasses these limits. Constrained profiles set
both values explicitly in `StaticResourceProfile`.

After a candidate has been selected, binding invocation, subscription start,
response parsing, response validation, cancellation, teardown, and backpressure
failures are execution errors. They must preserve the selected plan identity so
callers can diagnose the chosen form without confusing runtime failure with
candidate absence.

### Binding Plan Ownership

Bindings own protocol I/O, but not TD parsing policy. A binding may compile
binding-specific route or client metadata from a selected form. That compiled
metadata must live in the binding plan or binding state, not in the TD model.

Every compiled plan must retain enough source identity to explain behavior and
to support strict caller selection. At minimum this includes the affordance
or Thing-level target context, operation, original form index, resolved target
URI, effective security plan, and a stable plan id scoped to the consumed or
exposed Thing.

Binding plan construction happens at:

- `consume(td)` for outbound plans;
- `expose()` for protocol-neutral inbound route plans, followed by binding
  `prepare` for binding-specific route metadata;
- directory publication time for discovery metadata when needed.

## Protocol Binding Model

Bindings are extension points implemented by protocol crates.

The binding model has three layers:

- Protocol-neutral data types:
  `BindingCandidate`, `BindingRequest`, `InboundBindingPlan`,
  `BindingThingView`, `InboundRequest`, `InboundResponse`, support results,
  route matches, and security metadata. These types live in no_std-capable
  crates and must not require `Arc`, boxed futures, spawned tasks, or host
  synchronization.
- Constrained binding contracts:
  Poll-based or step-driven client and server traits use the same
  protocol-neutral data and are suitable for static binding tables, slot ids,
  and manually driven runtimes.
- Host-erased binding contracts:
  `ServerBinding` and `ClientBinding` are the host registration surfaces used
  by `ServientBuilder`, usually through `Arc<dyn ...>`. They may allocate for
  erased async futures, host guards, and runtime integration. Equivalent
  constrained APIs must preserve operation semantics without requiring this
  host-erased representation.

Host registration uses binding entries rather than bare trait objects at the
configuration boundary. A `ServerBindingRegistration` stores the binding id,
driving mode, optional route-readiness driver, runtime-event sink configuration,
overflow policy, optional form contributor, and `Arc<dyn ServerBinding>`. A
`ClientBindingRegistration` stores the binding id, client capability metadata,
and `Arc<dyn ClientBinding>`.
Convenience builder methods may accept bare trait objects and wrap them in
default registrations, but the explicit registration form is the design contract
for self-driving bindings, route readiness, and diagnostics.

### Producer Form Finalization and Route Ownership

The Producer path needs an explicit contract between a protocol-neutral draft
and binding-created endpoints. It must not infer generated forms by downcasting a
binding or by recognizing a protocol crate in servient.

`FORM-FINALIZE-001`: A `ServerBindingRegistration` MAY contain an object-safe
`ServerFormContributor`. The constrained registration contains an equivalent
static contributor slot. A contributor performs deterministic, local,
nonblocking finalization and has the following minimum semantic operation:

```rust
fn contribute_forms(
    &self,
    draft: BindingThingView<'_>,
    affordances: &[AffordanceFormRequirement<'_>],
    context: FormContributionContext<'_>,
) -> CoreResult<FormContribution>;
```

This block is a normative public API role; concrete borrowed view types may be
split without changing the input or output semantics. `FormContribution`
contains the contributing `BindingId`, zero or more generated forms associated
with their exact Thing/affordance target, any generated security definitions and
JSON-LD context terms, and a set of binding-local endpoint reservation keys.
Every generated form contains its operation set, absolute or base-relative
`href`, media metadata, subprotocol, effective security references, scopes, and
preserved extension members. It does not contain credentials or secret key
material.

Each contributor declares a bounded `FormContributionCapability` at
registration: URI schemes, operation/affordance classes, media families,
whether it contributes Thing-level forms, and whether it is a wildcard. The
Servient calls it only for matching requirements. A contributor MUST NOT receive
the full affordance set when its declaration cannot contribute to that set.
Wildcard contribution is explicit, consumes the same probe budget as wildcard
binding support, and is disabled by constrained profiles unless configured.

The contribution call MUST NOT open a listener, contact a peer, reserve an
external lease, spawn work, or depend on executor progress. It MAY inspect
already-open local endpoint configuration captured by the registration. The
same draft, registration configuration, binding generation, and resource policy
MUST produce equivalent contributions in stable order. Work and returned bytes
are charged to the expose admission transaction. A contributor that cannot
produce all required metadata locally returns a structured finalization error;
readiness is not used to discover or mutate the effective TD.

`FORM-FINALIZE-002`: `expose()` finalizes the TD in this order:

1. validate and index the binding-independent draft;
2. ask captured contributors in captured registration order for contributions;
3. merge contributions transactionally, allocating JSON-LD prefixes by the
   Clinkz context rules and rejecting incompatible duplicate security-definition
   names;
4. validate application-supplied and generated forms and resolve their owning
   server registrations;
5. reject missing required operations, duplicate route ownership, endpoint
   collisions, limits, and invalid effective security;
6. freeze the effective TD and compile all inbound plans;
7. publish `Preparing`, then enter the prepare/readiness/activate/commit
   lifecycle.

No form, security definition, route owner, plan id, or endpoint reservation key
changes after step 6. Readiness and activation may realize the frozen routes but
MUST NOT rewrite the served TD. Rollback releases every local endpoint
reservation made by the contribution transaction as well as prepared binding
resources.

`FORM-OWNER-001`: Each compiled inbound plan has exactly one owning
`BindingId`. A generated form is owned by its contributor. For an
application-supplied form, the Servient probes exactly the captured server
registrations returned by the server capability index, including declared
wildcards, with the same side-effect-free `supports(candidate)` categories used
for client planning. It MUST NOT probe a registration outside that candidate
set. Zero supporting registrations is a binding-selection error. More than one
supporting registration is an ambiguous-owner error unless the application
selected a binding id through an explicitly named Rust configuration field; the
engine MUST NOT resolve ambiguity by registration order. A selected binding that
does not support the form fails closed. The owner receives that plan in
`prepare`; non-owners do not.

Endpoint collision identity is the pair (`CollisionDomainId`,
`EndpointReservationKey`) and is independent of registration generation. The
binding registration supplies a canonical, bounded reservation key and
collision domain for every contributed or accepted route. Prepared, live,
draining, and cleanup-pending reservations in the same domain conflict across
binding generations. Route and binding generations distinguish ownership and
idempotent cleanup; they do not make the physical endpoint distinct. A
replacement may reuse a key only after the prior reservation is closed or
through an explicitly typed atomic-handoff contract. Alias declarations apply
only to compatible plans in one finalized route set, name one canonical route,
and are checked during finalization. Cross-binding keys do not collide when
their collision-domain ids differ.

`FORM-COVERAGE-001`: An affordance is not required to have every operation for
which a Producer handler exists. The frozen TD is authoritative. Conversely,
every operation advertised by an application-supplied or generated form MUST
have either a registered handler, a documented Scripting-compatible aggregation
path, or an explicit late-handler policy selected by the application. The
portable and host default policy is strict-at-expose: missing behavior rejects
`expose()`. A Rust-native `AllowLateHandlers` policy may admit the form, but a
request received before handler publication returns the structured
`UnsupportedOperation` error already defined, and selecting this policy is
visible in the effective handle diagnostics.

### ServerBinding

Host `ServerBinding` owns inbound protocol lifecycle:

This is the host-erased registration surface used behind
`Arc<dyn ServerBinding>`. It is object-safe. `ServerRouteGuard` and
`ActiveRouteGuard` are concrete type-erased host guard wrappers, not associated
types on `ServerBinding`; protocol crates store protocol-local guard state
inside those wrappers. Constrained server contracts use the frozen
`PollServerBinding` trait from the Implementation Contract with the same
protocol-neutral request, response, route, and lifecycle semantics. Constrained
traits may use associated guard types, slot ids, or caller-owned storage because
they are not the host-erased trait object surface.

The minimum constrained server surface mirrors the host lifecycle without boxed
futures or `Arc`: prepare a route set into caller-owned or binding-owned
prepared state, activate it into an active route slot, commit it after the
registry transition is ready to publish, poll or step for inbound requests,
send responses by correlation id, and shut down a route generation idempotently. A
poll-based implementation uses `Poll::Pending`, `Poll::Ready(Ok(Some(req)))`,
`Poll::Ready(Ok(None))`, and `Poll::Ready(Err(_))` with the same meanings as the
host surface. A step-driven implementation returns an equivalent progress enum:
idle, accepted request, graceful terminal, or structured error. Slot ids used by
constrained plans are stable for the lifetime of the constrained runtime table
and must not be reused while a prepared route, active route, request,
subscription, or cleanup operation can still reference them.

```rust
use core::task::{Context, Poll};

fn supports(&self, candidate: &BindingCandidate) -> BindingSupport;
fn prepare(
    &self,
    route: &BindingRouteKey,
    thing: BindingThingView<'_>,
    plans: &[InboundBindingPlan],
    ctx: BindingContext,
) -> CoreResult<ServerRouteGuard>;
fn activate(
    &self,
    route: &BindingRouteKey,
    guard: ServerRouteGuard,
) -> CoreResult<ActiveRouteGuard>;
fn commit(
    &self,
    route: &BindingRouteKey,
    guard: &ActiveRouteGuard,
) -> CoreResult<()>;
fn abort_prepared(
    &self,
    route: &BindingRouteKey,
    guard: &mut ServerRouteGuard,
    budget: &mut WorkBudget,
) -> CoreResult<CleanupOutcome>;
fn shutdown(
    &self,
    route: &BindingRouteKey,
    guard: &mut ActiveRouteGuard,
    budget: &mut WorkBudget,
) -> CoreResult<CleanupOutcome>;
fn poll_accept(
    &self,
    cx: &mut Context<'_>,
) -> Poll<CoreResult<Option<InboundRequest>>>;
fn send_response(&self, response: InboundResponse) -> CoreResult<()>;
fn publish(
    &self,
    emission: ProducerEmission,
) -> BindingFuture<'_, BindingPublication>;
```

`BindingRouteKey` contains the `ThingId`, owning `BindingId`, and immutable
handle/binding generations. It is the identity used by all lifecycle calls,
readiness tokens, late callbacks, runtime status, and shutdown. A textual Thing
id alone is not sufficient because a destroyed Thing may later be produced with
the same TD id. Registrations require unique live `BindingId` values; replacing
a registration creates a new binding generation. A stale lifecycle call or
callback returns or records `StaleHandle` and must not affect the newer route.

`BindingContext` is a cloneable owned handle. It must not borrow from the
`prepare` call stack because host bindings may keep it in a driver task. The
constrained server contract uses an equivalent owned context value that can be
kept in manually polled state.

`BindingThingView` is a restricted, protocol-neutral view of the exposed Thing
for binding setup. It contains stable Thing identity, human-readable metadata
needed for diagnostics, relevant root links, selected extension fields, and
source metadata that binding diagnostics may report. It does not require the
binding to receive the full TD tree, and bindings must not use it to redo
operation defaulting, `base` resolution, security inheritance, form selection, or
payload-schema selection. Those decisions are already represented in
`InboundBindingPlan`. If a concrete binding needs protocol-specific extension
metadata, the shared planning layer passes the preserved extension members
through the candidate or inbound plan for that form.

The core `ServerBinding` lifecycle surface is nonblocking. `prepare`,
`activate`, `commit`, `abort_prepared`, and `shutdown` may allocate, validate,
update local binding state, and make bounded cleanup progress, but they must not
wait on network I/O, executor progress, remote peers, or unbounded protocol
handshakes. Host
protocol crates that require asynchronous listener/session setup must expose
that setup in their host constructors before the binding is registered, or
through an explicit route-readiness driver stored in
`ServerBindingRegistration`. Constrained bindings use the same lifecycle rule
through their poll or step-driven contract: externally progressing work is
driven by poll or step methods, not hidden inside a lifecycle call.

A route-readiness driver is keyed by `BindingRouteKey` and the prepared route
identity owned by the `ServerRouteGuard`. The type-erased `ServerRouteGuard` wrapper must
therefore expose a stable prepared route identity or readiness key without
exposing protocol-local guard internals. A `ServerBindingRegistration` that
declares a route-readiness driver defines how the Servient drives or observes
that key to a terminal readiness state. After all bindings return prepared
guards and before any `activate` call, `expose()` must drive or observe every
registered readiness item until it reaches ready, failed, cancelled, or timeout.
Readiness failure is a prepare-phase failure: `expose()` rolls back every
prepared route, removes the preparing registry entry, and reports the binding id,
Thing id, prepared route identity, phase, and readiness error. A binding that
needs external progress but cannot expose a readiness driver must keep routes
unadvertised and perform only local fallible setup in `prepare`; it is not
allowed to block inside `prepare`, `activate`, or `commit`.

Host readiness driving is explicit and async-capable. `ExposedThingHandle::expose`
on the host surface is an async operation when any registered binding can require
readiness progress. A host `RouteReadinessDriver` is registered with the binding
entry and is driven by `expose()` for each prepared readiness key. The minimum
host driver surface is:

```rust
fn begin(
    &self,
    route: &BindingRouteKey,
    key: PreparedRouteKey,
    ctx: BindingContext,
) -> CoreResult<RouteReadinessToken>;
fn poll_ready(
    &self,
    token: &mut RouteReadinessToken,
    cx: &mut Context<'_>,
) -> Poll<CoreResult<RouteReadinessStatus>>;
fn cancel(
    &self,
    token: &mut RouteReadinessToken,
    budget: &mut WorkBudget,
) -> CoreResult<CleanupOutcome>;
```

`RouteReadinessStatus` has terminal states `Ready`, `Failed`, `Cancelled`, and
`TimedOut`, plus any nonterminal progress metadata a binding wants to report
through the runtime event sink. `begin` performs only local setup and returns a
token. `poll_ready` is the only readiness operation that waits for executor or
transport progress; it uses `Poll::Pending` instead of blocking. If `expose()`
is cancelled, times out, or a different binding fails, the Servient calls
`cancel` for every outstanding readiness token before rolling back prepared
routes. It retains a token until cancellation is complete, residual, or
transferred to the cleanup owner under the same rules as a route guard. A
binding that completes readiness during `prepare` may omit a driver or return an
already-ready key.

Constrained readiness uses the same states through a poll or step-driven
contract. The constrained `expose` equivalent either runs readiness to a terminal
state within the caller's manual-driver budget or returns a nonterminal progress
status that the caller must drive again. It must not spin indefinitely or hide
the need for external progress inside `prepare`, `activate`, or `commit`.

`BindingContext` carries the activation gate for the Thing route set. The gate
has at least three observable states: preparing, serving, and draining/closed.
Bindings may allocate routes, start local driver state, or register protocol
interest while the gate is preparing, but they must not deliver externally
visible requests to dispatch until the gate is serving. Dispatch must also check
the matched registry entry state and reject or drain requests for non-serving
entries, so the gate is enforced on both the binding and Servient sides. The
state transition to serving must be synchronized with publication of the
registry entry and active route guards; on host builds this uses the host
synchronization primitive selected by the Servient, and on constrained builds it
uses the same critical-section or manual-driver boundary that protects the
registry. A binding that cannot enforce this gate must keep routes unadvertised
until after `commit` and the final serving transition are complete.

`LIFE-EXPOSE-001`: The atomicity guaranteed by `expose()` is local publication
atomicity: the Servient registry and application dispatch observe either no
servable Thing or one complete servable Thing. This guarantee does not imply a
distributed transaction across protocol bindings or remote infrastructure.

`LIFE-EXPOSE-002`: A binding declares whether preparation is externally
quiescent. A `QuiescentPrepare` binding guarantees that no remote peer can
successfully interact with the route before the serving gate opens. A binding
that cannot make that guarantee MUST declare `CompensatingPrepare`; its setup
may leave externally observable state that requires compensation. Servient
policy MAY reject compensating bindings when strict external quiescence is
required.

`LIFE-EXPOSE-003`: Rollback produces a structured outcome:
`Complete`, `PendingCleanup`, or `ResidualExternalState`. Pending and residual
outcomes preserve binding id, Thing id, cleanup owner, retryability, and a
redacted cause. `expose()` MUST report them even when its primary failure came
from another binding. Process-crash recovery, remote lease expiry, and durable
cleanup are binding or host-integration responsibilities and MUST be documented
by bindings that create external state.

`abort_prepared` and `shutdown` are bounded cleanup-progress calls. `Complete`
permits the Servient to release the corresponding guard. `PendingCleanup` is
valid only after the registration, guard, and remaining cleanup work have been
atomically retained by the cleanup owner named in `CleanupRecord`.
`ResidualExternalState` means engine-local ownership has reached a terminal
state while externally observable residue remains; the durable status record
MUST retain that fact. An outer `CoreError` is reserved for an invalid or stale
call that did not transfer cleanup ownership. A `prepare` error guarantees that
the failing call left no caller-addressable or external resource requiring
compensation. A binding that cannot make that guarantee MUST return an
addressable prepared guard and surface the failure through readiness so rollback
can call `abort_prepared`.

For constrained cleanup, `Poll::Pending` means the caller still owns the same
operation and route generation. `Poll::Ready(Ok(PendingCleanup(_)))` is valid
only after cleanup ownership has transferred atomically to the named runtime
owner and consumes the caller's route generation; otherwise the implementation
MUST return `Poll::Pending`.

`prepare` declares routes for one Thing and returns a route guard that owns the
prepared binding resources. Prepared routes must not accept externally visible
requests until the final serving-state transition after all `commit` calls
succeed. This staged lifecycle lets `expose()` roll back all bindings without
locally publishing a half-serving Thing. Guard destruction is not the reporting
cleanup path and MUST NOT block. The Servient retains a prepared or active guard
until explicit cleanup reaches `Complete`, reaches `ResidualExternalState`, or
atomically transfers the guard and remaining work to reserved cleanup
ownership. On the success path, the Servient retains every active route guard in
the servable registry entry or in equivalent handle-owned state until
`destroy()` or handle drop starts explicit cleanup.

`activate` consumes a prepared route guard and returns an active route guard with
the same ownership semantics. It starts or arms that binding's driving model for
a prepared Thing, but the Thing must remain externally quiesced until the final
serving-state transition. All expected fallible route allocation, TD/form
validation, route collision detection, and protocol-local setup must happen
during `prepare` or the explicit route-readiness step before `activate`.
Fallible work that needs external progress must be represented by the
registration's route-readiness driver or by constrained poll/manual driver
state, rather than by blocking inside `prepare` or `activate`. After successful
readiness, `activate` is an idempotent activation step that should only fail for
errors that could not be detected before activation. If `activate` returns an
error, `expose()` rolls back every prepared or activated binding and removes the
registry entry. `abort_prepared` and `shutdown` are idempotent for the same route
generation and return `Complete` when that generation is already shut down.

After every binding has activated successfully, the Servient calls `commit` for
each active guard while the registry entry is still not serving. `commit`
confirms that the binding is ready to observe the final registry transition, but
it must not release externally visible request acceptance by itself. After every
`commit` succeeds, the Servient atomically marks the registry entry serving; that
single state transition releases any binding-local activation gate through the
`BindingContext` state visible to the binding. `commit` should be infallible in
normal operation; if it reports a late failure, `expose()` must roll back all
active bindings and remove the registry entry before any route becomes
externally serviceable. Bindings that do not need a local activation gate still
implement `commit` as an idempotent no-op. This keeps local Servient publication
atomic even when one binding activates successfully and a later binding fails
activation or commit. External cleanup still follows `LIFE-EXPOSE-002` and
`LIFE-EXPOSE-003`.

`poll_accept` uses `Poll::Pending` for "no request available yet",
`Poll::Ready(Ok(Some(req)))` for an accepted request,
`Poll::Ready(Ok(None))` for a graceful terminal state, and
`Poll::Ready(Err(_))` for binding, driver, transport, cancellation, or
backpressure failures that must be visible to the runtime. `send_response`
reports response delivery and backpressure errors through `CoreResult`. The
constrained poll server contract uses the same ready, pending, terminal, and
error meanings through its bounded response slot and start/poll operations;
runtimes that cannot use a real waker may use a no-op waker and drive the poll
surface from a super-loop.

Server bindings have two explicit driving modes:

- Externally driven bindings expose accepted requests through `poll_accept`.
  The host runtime or test harness owns polling, dispatch, response sending,
  and observation of `poll_accept`/`send_response` errors. Constrained runtimes
  use `start_response`/`poll_response` with caller-owned bounded slots for the
  same externally driven model.
- Self-driving host bindings may spawn or own protocol driver tasks that receive
  transport requests, call `ctx.dispatch.serve_request(req).await`, and send
  responses back through the protocol. They must still use the same
  `BindingContext` activation gate and route-match data, and they must report
  binding, transport, response-delivery, backpressure, and driver-task failures
  through a Servient-visible status or error sink with the same error categories
  as externally driven bindings.

The Servient-visible status sink is part of host binding registration. It is a
bounded runtime event channel or callback using the shared capacity policy and
reports `BindingRuntimeEvent` items containing the binding id, optional Thing id,
operation phase, plan id or form index when known, correlation id when known, and
a structured error or terminal status. Overflow behavior is explicit: the sink
either applies backpressure to the binding driver, drops according to a
configured policy while incrementing an observable lost-event counter, or shuts
the binding down with a structured backpressure error. Self-driving bindings
must not hide driver task panics, response-send failures, or queue overflow by
logging only.

Runtime events are classified before overflow policy is applied:

- Diagnostic events are best-effort observations such as transient progress,
  sampled metrics, and nonterminal protocol notes. They may use the configured
  drop policy.
- Operational status events describe recoverable interaction, subscription,
  response-delivery, backpressure, or teardown failures. They may be dropped
  only when the selected policy also increments loss counters and preserves a
  per-binding or per-Thing latest-status record.
- Critical lifecycle events describe driver task panic, route driver terminal
  failure, activation gate violation, commit failure, unexpected driver
  terminal state, binding shutdown caused by overflow, and response-send failure
  after a handler completed successfully. Their critical details must not be lost
  solely because the event queue is full.

Every binding registration therefore has a small durable status record in
addition to the bounded event sink. The record stores a bounded critical-event
journal, the latest critical event summary, latest operational error, lost-event
counters by event class, and the terminal driver state when one exists. If the
bounded sink cannot accept a critical event, the runtime must update this durable
record before applying the overflow policy. If the critical-event journal itself
is full, the runtime must either apply backpressure, shut the binding down with a
structured overflow error, or compact the journal into an explicit
critical-events-compacted record that preserves the count, first and latest
critical event summaries, and affected Thing or binding identities. A drop
policy may drop the queued event copy, but the critical details remain available
through Servient status APIs. Critical event records must carry diagnostic
context without payload bytes, credentials, or redacted TD fields.

The selected driving mode is binding metadata known at registration or prepare
time. A binding must not silently switch modes for one Thing in a way that makes
runtime errors disappear from the Servient's observable lifecycle.

Inbound requests must carry a protocol-neutral route match produced from an
inbound binding plan. The route match includes the Thing id, affordance target,
operation, original form index, compiled plan id, correlation id, URI variable
values, inbound payload metadata, and transport-native
`TransportAuthMaterial`. Dispatch uses this route match to apply the exact
form-level security and scope semantics that were validated during `expose()`.

The Servient does not own a transport driving loop.

`BIND-IO-001`: `InboundRequest` owns its `BindingRouteKey`, route match,
correlation id, application-independent wire `Payload`, transport-native auth
material. `InboundResponse` owns the same route and correlation identities plus
either an output with validated `InteractionOutputMetadata` or a structured
error mapping. A binding cannot borrow either value from a transport receive
buffer after the call returns. Duplicate live correlation ids are rejected
within one binding route; ids may repeat on unrelated routes. Host
`send_response` and constrained
`start_response` validate that the route generation and correlation id still
name an admitted in-flight request. Host delivery consumes that response
opportunity in the call. Constrained delivery consumes it when start is accepted
into a response slot; failure before slot acceptance leaves the opportunity with
the caller so backpressure can be handled without invoking the application
handler again. After acceptance, success, failure, or cancellation reaches one
terminal result exactly once. Any protocol retry is binding-owned and never
calls the application handler again.

URI parsing and route matching are binding responsibilities; semantic
authorization and payload validation are core responsibilities. The binding
MUST reject a transport target that cannot be matched to exactly one compiled
inbound plan. It decodes only protocol framing and the fields needed to produce
the route match, URI variables, payload media metadata, and transport-native
authentication material. It MUST NOT interpret, remove, or redact TD
body-location security fields. Core dispatch then verifies the immutable plan
id/route generation pair, decodes the wire payload at most once, extracts body
authentication material through the compiled plan, verifies the combined
transport/body requirement and scopes, validates the application projection,
and invokes the handler with only the verified `Principal`. A binding must not
construct a route match from caller-controlled plan ids without checking it
against its prepared route table.

### ClientBinding

Host `ClientBinding` owns outbound protocol behavior:

```rust
pub type BindingFuture<'a, T> =
    Pin<Box<dyn Future<Output = CoreResult<T>> + Send + 'a>>;

fn supports(&self, candidate: &BindingCandidate) -> BindingSupport;
fn invoke(&self, request: BindingRequest) -> BindingFuture<'_, InteractionOutput>;
fn subscribe(
    &self,
    request: BindingRequest,
) -> BindingFuture<'_, HostSubscriptionStart>;
```

`BIND-OUT-001`: `BindingRequest` owns the selected `BindingId`, binding
generation, plan id, operation, resolved target, payload, media and response
metadata, URI variables after expansion, applied-security fields, correlation
id, deadline/cancellation view, and idempotency metadata when supplied. It does
not contain the full TD. The shared layer constructs it only after selection and
security commit. A binding MUST NOT select a different form, weaken security,
or reinterpret application payload fields as credentials.

`invoke` has exactly one terminal result. Successful transport exchange is not
automatically successful WoT output: the binding maps protocol status and bytes
into response metadata, then the shared response validator classifies the
primary or `additionalResponses` result. Implementations may place that shared
validation immediately above or below the trait call, but the public future
returns only a validated successful `InteractionOutput` or a structured error.
Cancellation before protocol commit returns `Cancelled`; cancellation after an
unknown or completed side effect preserves a `CallerDecision` retry class.

`subscribe` returns `HostSubscriptionStart` only after the remote or local start
operation has succeeded and its start response has passed validation. Samples
arriving before guard-registry installation are held in the same bounded queue
charged to the pending start; overflow applies the selected subscription policy.
If installation fails, no sample becomes visible through a public
`Subscription`, and pending samples are discarded with the guard cleanup.

`BindingCandidate` is the protocol-neutral, pre-resolved form view produced by
the shared planning layer. It contains the resolved target URI, effective
operation, media metadata, subprotocol, URI-variable declarations, effective
security plan, form index, plan id, and extension fields. Bindings must not
re-parse the TD tree to resolve `base`, default operations, security
inheritance, or content-type defaults. `BindingSupport` must distinguish
unsupported protocol, unsupported operation, unsupported media, unsupported
security, and binding-specific preflight failure when that information is
available.

`SubscriptionStart` is protocol-neutral start metadata containing the stable
`SubscriptionId`, selected plan and binding generations, and admitted queue and
overflow policy. `HostSubscriptionStart` contains that metadata plus the receive
side and wire-side `SubscriptionGuard`. The guard owns the binding's subscription resource and any
compiled teardown metadata needed to perform an explicit `unobserveproperty`,
`unobserveallproperties`, `unsubscribeevent`, or `unsubscribeallevents`
interaction. This metadata stores the selected teardown plan, route identity, and
credential lookup requirements, but it must not store secret bytes or committed
credential material longer than the binding protocol itself requires. The
`ConsumedThingHandle` installs the guard into its guard registry atomically
before constructing and returning the public `Subscription`. If guard
installation fails after the binding created a wire resource, the handle
immediately closes the guard or runs the compiled teardown path before returning
the installation error. A returned public `Subscription` therefore never owns a
second copy of the wire-side guard. `subscribe` defaults to unsupported for
bindings that only implement one-shot request/response interactions.

The registered `ClientBinding` surface is object-safe because applications
register `Arc<dyn ClientBinding>` for host async use. Network-bound binding
calls may allocate one boxed future per operation; local dispatch and
synchronous handler paths remain separate allocation-sensitive paths.

The object-safe async surface is not the only binding contract. Constrained
builds that cannot or should not allocate boxed futures use the frozen manually
driven `PollClientBinding` surface that accepts the same
`BindingCandidate` and `BindingRequest` data. Host adapters may wrap a
poll-based binding in the erased async `ClientBinding`; embedded adapters may
drive the poll surface directly from a super-loop.

The minimum constrained client surface starts an outbound request or
subscription against a compiled binding slot and returns a request token owned
by the constrained runtime. The runtime then polls or steps that token to one of
these terminal states: successful `InteractionOutput`, successful
`SubscriptionStart`, cancellation accepted, binding/transport error, response
validation error, or teardown/backpressure error. Cancellation and teardown are
explicit operations on the token or subscription slot. A constrained binding may
complete synchronously in the start call, but it must still report the same
selection, execution, cancellation, and teardown categories as the host
`ClientBinding`.

`BIND-PROGRESS-001`: A pending constrained subscription start transitions its
caller-owned `ClientSubscriptionSlot` to `Active` when `SubscriptionStart` is
returned; unlike a one-shot request slot, successful start does not consume that
slot. The public constrained subscription retains only the slot index and
generation. Samples, terminal status, explicit stop, and drop cleanup are driven
through `poll_subscription_item` and `poll_stop_subscription`; they do not
require a type-erased receive object or a binding-owned unbounded table. Exactly
one owner may transition the slot from active to stopping, and a terminal stop
consumes its generation only after the terminal outcome has been retained.

Constrained response delivery uses a caller-owned `ServerResponseSlot` whenever
the binding cannot accept a response synchronously. Starting delivery either
completes immediately or moves the owned `InboundResponse` into that bounded
slot; subsequent progress and cancellation use `poll_response` and
`poll_cancel_response`. Pool or slot exhaustion returns `Backpressure` before
the response opportunity is consumed. Once a start is accepted, the application
handler is never invoked again for retry, and a terminal delivery result consumes
the response opportunity exactly once. This makes response backpressure
progressable without an unbounded binding queue or a busy retry loop.

Constrained Producer publication uses a caller-owned `ServerEmissionSlot` under
the same rules. Start either completes or transfers the owned
`ProducerEmission` and its admitted result capacity into the slot. Poll resumes
the retained binding-target cursor, and cancel retains terminal outcomes for
already accepted publications before consuming the slot generation. Slot
exhaustion returns `Backpressure` before publication begins.

### Removed Facades

The design removes `ProtocolBinding` and `ClientBindingFactory`. Applications
register `ServerBindingRegistration` and `ClientBindingRegistration` values
with `ServientBuilder`. Convenience builder methods may accept
`Arc<dyn ServerBinding>` and `Arc<dyn ClientBinding>` directly when the default
registration metadata is sufficient. A concrete protocol crate may still expose
convenience constructors that return those trait objects or complete
registration values.

## Subscription Model

Consumed-side subscriptions use a Rust pull-stream model.

Target semantics:

- `observe_property` and `subscribe_event` return a `Subscription` that can be
  drained by polling or by `Stream` when `async` support is enabled.
- Collection-level subscription operations such as `observeallproperties` and
  `subscribeallevents` use the same `Subscription` model with a collection
  target context. Each delivered item identifies the originating property or
  event when that information is available from the protocol or payload.
- A Scripting-compatible mode permits at most one active subscription per
  property, event, or collection target and rejects a second active
  subscription for the same target. Rust extension modes may allow multiple
  active subscriptions for the same target.
- Every active subscription has a stable `SubscriptionId` scoped to the owning
  consumed Thing. The id is returned with or embedded in the `Subscription`,
  including in Scripting-compatible mode.
- Each active subscription has a wire-side `SubscriptionGuard`.
- The `ConsumedThingHandle` owns guards by subscription id so callers do not
  leak protocol resources by forgetting a guard.
- The returned `Subscription` owns the receive side, the `SubscriptionId`, and a
  teardown capability back to the handle-owned guard registry. It must not own a
  second copy of the wire-side guard.
- `Subscription::stop` is the explicit teardown API. It uses the teardown
  capability to remove and close the matching guard, may accept stop options
  when the selected protocol needs TD-level teardown input, and reports the
  resulting terminal status. Calling `stop` after the handle has already been
  dropped is idempotent and reports completion or a structured already-closed
  status.
- The handle and all returned subscriptions share a subscription inner state
  whose lifetime can outlive either side. The handle is the exclusive owner of
  wire-side guards while it is alive. Dropping the handle marks the shared state
  closed, closes every remaining guard, and records the terminal status for
  subscriptions that still exist. A later `Subscription::stop` observes this
  closed state and returns the recorded status instead of trying to access a
  dropped handle or duplicate guard.
- If an explicit teardown form is required, `Subscription::stop` selects an
  `unobserveproperty`, `unobserveallproperties`, `unsubscribeevent`, or
  `unsubscribeallevents` plan associated with the original subscription,
  respecting caller-provided teardown `formIndex`, URI variables, and event
  unsubscribe data. If the binding supports implicit teardown, the guard may
  close the wire resource directly without issuing a second TD operation.
- Dropping a `Subscription` without calling `stop` requests teardown through the
  same shared state, but it cannot provide new caller input or report a direct
  `Result`. The runtime must remove or schedule removal of the handle-owned
  guard exactly once, close the local receive side, and record an observable
  terminal status when the profile has a status channel. Drop cleanup may run a
  protocol-level teardown only when the original subscription plan already
  contains all required URI variables, form selection, cancellation payload, and
  a teardown security plan with credential lookup policy. Drop cleanup must
  reapply or refresh credentials through the configured provider when protocol
  teardown needs security material; it must not keep application secrets in the
  subscription solely to make later drop cleanup possible. If credentials are no
  longer available, have expired, require caller input, or the provider cannot
  safely commit them during drop cleanup, the runtime closes local resources,
  marks the subscription terminal, and records a structured teardown-not-run or
  teardown-auth-failed status. Documentation for each binding must state whether
  drop performs protocol-level teardown, local-only cleanup, or best-effort
  scheduled teardown.
- Rust extension methods such as `unobserve_property` and `unsubscribe_event`
  must either take an explicit `SubscriptionId` or fail when the target is
  ambiguous.
- Dropping the handle closes every remaining guard.

Backpressure must be explicit:

- Every subscription queue has a bounded capacity selected through the shared
  capacity policy.
- The host default subscription queue capacity is 16 samples. Constrained
  runtimes must require an explicit capacity or document a profile default.
- The default overflow policy is drop-oldest unless a binding or caller selects
  another policy.
- Dropped samples must be observable through a loss counter and through status
  or an error item. If the status/error channel is itself unavailable or full,
  the durable loss counter remains queryable and records the number lost when
  known.
- A slow consumer must not allow unbounded heap growth.

`SUB-STORAGE-001`: A logical queue per subscription does not require a distinct
heap allocation or a queue preallocated to its maximum payload size. Runtime
profiles define the physical storage strategy: shared sample descriptors,
fixed-size rings, latest-value mailboxes, rendezvous slots, and shared payload
slab/block pools are all valid. Per-subscription count and byte quotas plus
global descriptor and payload-pool quotas are enforced independently. Empty or
low-rate subscriptions MUST NOT reserve their maximum payload-byte allowance by
default. Oversized payload behavior is explicit and never triggers an
unbounded fallback allocation.

`SUB-DATA-001`: Subscription control and sample data paths are separate. Stop,
drop, and teardown use the guard registry; sample enqueue/dequeue uses a direct
generation-bearing subscription slot and its queue/loss counters. The sample
hot path MUST NOT acquire a Thing-wide guard-registry lock or copy teardown
metadata. Shared payload fan-out uses immutable references or slab blocks where
the execution environment permits, with reference operations defined by the
selected ownership model rather than requiring atomics.

Ordering is per subscription. The engine does not guarantee global ordering
across unrelated properties or events.

## Bulk Interaction Model

Bulk Consumer methods must preserve per-affordance structure.

Target result types:

- `PropertyReadMap`: property name to `InteractionOutput`.
- `PropertyWriteMap`: property name to write input payload/options.
- `PropertyStatusMap`: property name to success or structured `CoreError`.

The Scripting API-compatible methods use TD-level bulk operation forms when the
TD exposes `readallproperties`, `readmultipleproperties`, or
`writemultipleproperties`. These methods issue one selected form request. On
success, read methods return a `PropertyReadMap`; write methods return success
or a structured operation error according to the selected binding response. For
these Consumer methods, selected forms come from the TD root `forms` array;
`formIndex` options are scoped to that root array, not to any individual
affordance's `forms` array.

TD `writeallproperties` is exposed as the Rust extension method
`write_all_properties`, not as a Scripting-compatible method. It selects a
Thing-level `writeallproperties` form from the TD root `forms` array and uses
the same root-scoped `formIndex` rule as the Scripting-compatible bulk property
methods. If no compatible TD-level `writeallproperties` form exists, the method
reports that the operation is unavailable; fan-out write helpers remain
separately named Rust extensions.

When no compatible TD-level bulk form exists, the Scripting API-compatible bulk
method reports that no compatible bulk form is available. The engine may offer
explicit fan-out helpers that call individual property forms. Fan-out helpers
are Rust extensions, not silent fallbacks under the Scripting API-compatible
method names, and their names must make the fallback behavior visible.

Scripting-compatible `read_all_properties` and `read_multiple_properties`
return `Result<PropertyReadMap, CoreError>`. They do not encode per-property
failures inside `PropertyReadMap`; a TD-level response that reports partial
failure is surfaced as a structured operation error that preserves bounded
plan, binding, phase, cause-code, and redacted-cause context. It does not retain
the response payload, schema, or raw protocol status in `CoreError`. Rust
extension helpers may expose non-lossy partial-result maps, but those helpers
must use distinct names and must not be presented as the Scripting-compatible
methods.

Fan-out write helpers may run writes concurrently only when the caller does not
request sequential semantics. A sequential fan-out mode is required for Things
where property write order is meaningful. TD-level `writemultipleproperties`
uses one selected form request and does not fan out internally at the engine
layer.

Producer-side Scripting-compatible aggregation is different from Consumer
fan-out. A Producer may receive one TD-level `writemultipleproperties` request
and satisfy it by invoking registered per-property write handlers according to
the configured aggregation policy. That local handler aggregation is permitted
only because the protocol transaction is already a single bulk interaction. It
must not be reused by Consumer methods to simulate a missing TD-level bulk form.
Applications that need atomic device commits, register-block writes, or stronger
performance guarantees should register an explicit Thing-level bulk handler
instead of relying on per-property aggregation.

Bulk methods must not aggregate heterogeneous property payloads into a single
opaque JSON payload as the only API. JSON aggregation can exist as a helper.

## Zenoh Binding

Zenoh is the first optional protocol binding, not a required engine dependency.

The zenoh crate provides:

- Form planning and validation for `zenoh:` targets.
- Clinkz extension metadata extraction for zenoh priority, congestion control,
  and QoS hints.
- A Rust zenoh runtime backend behind `zenoh`.
- Constructors such as `shared`, `server`, `client`, `client_pooled`, and
  `client_pooled_default` that return direct server/client binding trait
  objects.

The shared-session constructor is suitable when Producer and Consumer use one
pre-opened session. Pooled client construction is the preferred direction for
Consumers that need to reach Things through different TD-resolved authorities.

Zenoh-specific logic must not move into TD, discovery, core, or servient crates.

## Discovery

`clinkz-wot-discovery` models WoT Discovery as:

1. Introduction: obtain a discovery endpoint.
2. Exploration: query or navigate a Thing Directory.
3. Continuation: lazily drain a discovery session.

Primary public concepts:

- `DiscoveryEndpoint`, `IntroductionSource`, and `Introducer`.
- `DirectoryQuery`, filters, projections, counts, revisions, and lease tokens.
- `DirectoryReader` and `DirectorySession`.
- `DirectoryPublisher` for lease/revision-aware publication.
- `DirectoryWatch` and `DirectoryChange`.
- `ThingDescriptionResolver` and `ThingLinkResolver`.
- `Discoverer`, `DiscoveryFilter`, `DirectoryRef`, and
  `ThingDiscoveryProcess`.

The protocol-neutral Directory contracts are engine-facing remote-client
contracts. They define what the engine sends, receives, polls, cancels, and
reports; they are not a storage SPI or a server implementation contract.

### Directory Scope and Deferred Service Design

`DIR-SCOPE-001`: The current design includes only the engine-to-Directory
interaction boundary: endpoint references, query and publication requests,
owned result values, sessions, watches, cancellation, timeout, pagination,
revision and lease tokens, trust metadata, and portable poll/async adapters.
Constructing a `Servient` MUST NOT create an in-process Directory, and
`clinkz-wot-servient` and `clinkz-wot-discovery` MUST NOT depend on Directory
service composition or storage crates.

The following concerns are explicitly deferred to a later Directory service
design and are not implementation requirements for this engine revision:

- service process topology, endpoint hosting, and deployment lifecycle;
- storage-backend SPI, persistence, replication, and high availability;
- server-side authorization/redaction orchestration and policy storage;
- query planner, physical indexes, snapshot storage, compaction, and lease
  reclamation implementation;
- server-side watch fan-out and publication transaction implementation;
- reference in-memory backends and production service SLOs.

Non-normative design inputs for that later revision are retained in
`docs/future/directory-service.md`; that file is not an active engine artifact.

The interaction types preserve enough protocol information for a later service
to implement the contract without putting service behavior into the engine.
They do not prescribe how a server satisfies a query, stores revisions, creates
page tokens, or retains snapshots. A future Directory service design may add
service and backend crates, but it must treat this engine API as a client
boundary and must not make a Directory service a Servient dependency.

`DirectoryQuery`, publication requests, snapshot options, and watch options
express client request intent and client-observable response invariants. This
revision assigns no query evaluation, hidden-field authorization, redaction
ordering, compare-and-set enforcement, lease-token validation, snapshot
retention, compaction, or watch fan-out implementation to
`clinkz-wot-discovery` or `clinkz-wot-servient`.

`DiscoveryFilter` is the Rust carrier type for the Scripting API `ThingFilter`
subset used by `discover(filter)` and Scripting-compatible
`explore_directory(url, filter)`. `ThingFilter` names the W3C semantic filter
model in this document; `DiscoveryFilter` is the concrete Rust API type unless a
lower-level API explicitly exposes a differently named adapter.

The Scripting-compatible `ThingFilter` subset is intentionally small. An omitted
filter is an empty filter. `DiscoveryFilter::fragment` is serialized losslessly
as a request for W3C fragment-filter semantics over the endpoint-authorized
searchable view. A client adapter MUST NOT broaden, remove, or reinterpret the
fragment. It MUST NOT emulate unsupported fragment filtering by fetching a
broader canonical result set and filtering it locally. When endpoint capability
is known to be insufficient, or the endpoint reports the operation unsupported,
the client returns a structured unsupported-filter result. Server-side
authorization, searchable-view construction, redaction order, fragment
evaluation details, and hidden-field oracle prevention belong to the future
Directory service design.
Semantic query languages, projection, pagination, security posture filters,
lease constraints, and watch options are Rust extensions represented by
`DirectoryQuery` or other explicitly named types.

Discovery remains protocol-neutral. Concrete discovery transports are optional
client integrations outside the `discovery` core crate. Directory service and
storage design is deferred rather than implicitly supplied by this workspace.

### Discovery Scripting API Mapping

The Discovery UA target requires:

- `discover(filter)`: returns a lazy process over matching bare TD views.
- `explore_directory(url, filter)`: returns a lazy process over a TD Directory
  query using `ThingFilter` semantics and yielding bare TD views.
- `request_thing_description(url)`: resolves one bare TD view by URL.

The Rust-native discovery and directory targets additionally include document
surfaces such as `discover_documents(filter)`,
`explore_directory_documents(url, filter)`, `fetch_td_document(url)`, and
`explore_directory_query(directory_ref, query)`. These surfaces return TD source
envelopes, or processes over envelopes, where `directory_ref` can represent
non-URL directory handles or previously introduced endpoints and `query` is a
`DirectoryQuery` superset.

The Scripting-compatible surface accepts `ThingFilter` semantics for
`discover(filter)` and `explore_directory(url, filter)`. Rust APIs may expose a
`DirectoryQuery` superset for directory-native filters, projections,
pagination, counts, revision constraints, lease handling, and query-language
selection. When a Scripting API method is mapped to Rust, an omitted filter maps
to an empty `ThingFilter`, and a `ThingFilter` maps losslessly into the
corresponding `DirectoryQuery` subset. Query features that cannot be represented
by `ThingFilter` are Rust extensions and must be named or typed as such.

`ThingDiscoveryProcess` yields discovered bare TD views on the
Scripting-compatible surface. Rust-native document processes yield the
corresponding TD source envelopes. Both process shapes must support:

- lazy fetching;
- cancellation/suppression;
- terminal error reporting;
- optional timeout;
- bounded buffering according to the shared capacity policy.

The portable poll view yields `ProcessEvent<T>` rather than using `Option<T>`
as an implicit end marker. Host `Stream` adapters may expose items through the
conventional stream interface, but they MUST retain the one `ProcessTerminal`
value for an explicit `terminal_status()` accessor or completion future;
`None` alone is not the error, timeout, cancellation, or overflow contract.

Cancellation semantics are explicit. Calling stop or dropping the discovery
process requests cancellation, prevents new directory/network work from being
started, and closes discovery-session producer resources. Already buffered TD
candidates may be drained only when the caller selected a drain-on-cancel
policy; the default policy suppresses remaining buffered candidates and reports
a terminal cancelled status. Timeout is reported as a distinct terminal status
from caller cancellation, and terminal directory, transport, validation,
timeout, and cancellation statuses must not be collapsed into a generic
end-of-stream.

Discovery overflow is also terminal unless the selected profile explicitly
permits loss. When a discovery producer cannot be backpressured and the bounded
process buffer is full, the default non-lossy behavior is to stop starting new
directory or network work, cancel the underlying discovery session where
possible, and report a structured overflow terminal status. A lossy discovery
profile may drop newest candidates only when that behavior was selected
explicitly and the terminal or status item records the number of candidates lost
when known. No discovery profile may recover from a full buffer by switching to
an unbounded queue or by silently broadening, truncating, or hiding results.

`DirectoryQuery` can express protocol-neutral filters over TD metadata,
security posture, capabilities, and semantic terms. A client adapter declares
which request capabilities it can encode. Unsupported query languages or filter
forms fail explicitly before a broader request is sent; an adapter never
silently removes a requested predicate.

### Discovery Trust, Freshness, and Privacy

Rust-native discovery APIs return TD candidates plus source metadata.
Scripting-compatible discovery APIs yield bare TD views, but the runtime still
retains source metadata internally for admission and diagnostics where the API
surface allows it. A discovered TD is not automatically trusted for `consume()`
or republication just because it came from a directory or introducer. Callers
can configure the minimum validation level, accepted source schemes, accepted
directory identities, freshness requirements, and whether unsigned or
unauthenticated TDs are acceptable.

The engine-facing Directory contract preserves these observable invariants:

- Publication requests carry the caller-selected validation evidence and source
  metadata; publication results state the accepted revision, lease, and digest.
- Returned entries carry revision, lease or expiry status, publisher identity
  when available, validation level, and digest or signature metadata when
  available.
- A response that declares an entry current cannot simultaneously mark its lease
  stale or its revision superseded; historical results require an explicit query
  option and status.
- Watches report ordered revision changes per directory entry and expose
  compaction or missed-update status when a watcher falls behind.
- Client adapters apply bounded result-page and watch-buffer limits and reject a
  remote response that exceeds the admitted count or byte budget.

Discovery filters must fail closed when a requested security posture,
capability, semantic term, or query language is unsupported. Returning a broad
unfiltered result set for an unsupported filter is a design defect because it
can disclose TD metadata and cause Consumers to select unintended Things.

TD privacy remains part of the client boundary without assigning server policy
to the engine. The client treats each returned `TdDocument` as the view supplied
by the remote endpoint and preserves its source and policy-generation metadata.
It MAY apply an additional local presentation projection, but that projection is
not an authorization boundary and does not establish that the remote endpoint
performed correct redaction. Client diagnostics and errors MUST redact lease
tokens, credentials, caller authorization material, and fields hidden by a
selected local presentation policy. Endpoint-side authorization, canonical
storage, searchable-view construction, and hidden-field oracle prevention are
requirements of the future Directory service design.

### Directory Consistency and Public Trait Contract

`DIR-CONTRACT-001`: Protocol-neutral Directory operations use owned request and
result values and expose these minimum semantic operations. Portable poll traits
are fixed by `API-DIRECTORY-POLL-001`; the `async` feature adds adapters and
`std` may add documented blocking adapters under the execution-adapter contract
defined later in this document.

- `DirectoryReader::query(DirectoryQuery) -> DirectorySession`;
- `DirectoryReader::watch(DirectoryQuery, WatchStart) -> DirectoryWatch`;
- `DirectorySession::next_page() -> DirectoryPage` and `cancel()`;
- `DirectoryPublisher::create(TdDocument, PublishOptions) -> Publication`;
- `DirectoryPublisher::replace(EntryId, ExpectedRevision, TdDocument,
  PublishOptions) -> Publication`;
- `DirectoryPublisher::renew(EntryId, ExpectedRevision,
  PublicationAuthority, LeaseRequest) -> Publication` and
  `delete(EntryId, ExpectedRevision, PublicationAuthority)`;
- `DirectoryWatch::next_change()` and `cancel()`;
- `ThingDescriptionResolver::resolve(AbsoluteUri) -> TdDocument`.

Every construction and step operation above returns a structured directory
result; arrows omit `CoreResult` only for readability. A page item is a
`TdDocument`, not a bare TD. Session and watch end-of-stream is distinguishable
from cancellation, timeout, compaction, authorization change, backend failure,
and normal completion. Sync, async, and poll twins share the same owned request,
item, error, and terminal-status types.

`EntryId`, `DirectoryRevision`, `EntryRevision`, `LeaseToken`, `PageToken`, and
watch cursor are distinct opaque types. Publication results contain entry id,
new entry revision, directory revision, canonical digest, effective lease
expiry, and a newly issued or rotated `LeaseToken` when the Directory uses
lease-capability authorization. `ExpectedRevision` is either `Any` or `Exact`; host defaults use
`Exact` for replacement, renewal, and deletion. The client maps a reported
mismatch to a structured conflict and does not retry by weakening the request to
`Any`. Lease tokens are secret-bearing capabilities: Debug, Display, errors,
and discovery envelopes redact them.

`DIR-AUTH-001`: `PublicationAuthority` is a non-exhaustive typed choice between
the request's authenticated publisher authority and an owned `LeaseToken`; it is
not an `Option`, string, or ambiguous boolean. A client sends exactly one typed
authority and never silently substitutes authenticated publisher authority for
a lease token. It adopts a rotated token only from a successful publication
result. It MUST NOT retry an unknown renewal or deletion outcome with
`ExpectedRevision::Any` or a different authority. A client adapter MUST NOT copy
a lease token into `TdSourceInfo`, page items, watch changes, diagnostics, or
retryable error context. Retry after an unknown renewal outcome is
`CallerDecision` until revision lookup establishes whether the token was
rotated. Remote token validation, rotation, and invalidation policy are deferred
service behavior.

`DIR-SNAPSHOT-001`: A query requests one directory snapshot or explicitly
documented weak snapshot. `DirectoryPage` contains ordered TD documents, the
declared snapshot revision, an optional next-page token, and optional total
count only when requested and returned as authorized metadata. Stable portable
ordering is entry id ascending unless the query selects another supported
order. The client records endpoint identity, query digest, authorization-context
generation, projection, and declared snapshot mode in the session slot. A page
token is opaque and bounded. The client refuses to reuse it after any recorded
input changes, validates response metadata and ordering consistency, and rejects
an empty intermediate page because it could spin without progress. Invalid,
stale, unauthorized, or cross-query token statuses from the endpoint remain
distinct structured errors. Token generation, cryptographic binding, snapshot
storage, and expiry enforcement are deferred service concerns.

A remote Directory that cannot hold a snapshot across pages declares
weak-snapshot semantics in the first session result. Such pages still carry
revisions and may report duplicates or missed entries only through an explicit
consistency status. The client rejects weak-snapshot sessions when the caller
requires snapshot isolation. Snapshot storage, retained-version budgets, and
server publication backpressure belong to the deferred service design.

`DIR-WATCH-001`: A watch starts from an explicit directory revision or from a
snapshot returned by a query. Changes are ordered by directory revision; changes
to one entry also carry monotonically increasing entry revisions. Each item is
`Created`, `Replaced`, `Deleted`, or `LeaseRenewed`. Compaction is the terminal
`DirectoryTerminal::Compacted` status; it names the oldest resumable revision
and terminates the current gap-free view without requiring a synthetic change
item. The caller must query a new snapshot before claiming continuity. Redaction
is an endpoint concern. The adapter enqueues only the response view received for
the active authorization-context generation. A reported policy-generation
change terminates or requires resnapshot before later items become visible.
Lease expiry reported by the endpoint is represented as deletion with an expiry
cause. Cancellation and overflow follow the shared state and capacity contracts.

`DIR-STREAM-001`: Directory response decoding and admission are incremental
across transport bytes and page items. A client MUST NOT require the complete
encoded page, a generic decoded page DOM, and lossless admitted copies of every
TD to coexist. Each item is size-checked, decoded, validated, and transferred
into eventual result storage or a bounded response slot before the next item
consumes its full admission budget. That storage remains private until page
metadata and ordering are structurally complete; failure discards the partial
page and publishes no truncated successful page. Poll adapters
return `Pending` when input, output capacity, or work budget is exhausted and
resume from a bounded generation-bearing cursor rather than reparsing accepted
bytes or items. Host convenience adapters MAY materialize an owned
`DirectoryPage`, but that result storage is charged once and does not permit a
second full-page staging copy.

`DirectoryTerminal` contains `Compacted { oldest_resumable_revision }` and
`AuthorizationChanged { policy_generation }` domain outcomes. It carries only
bounded opaque revisions or generations and no redacted query data. Normal
remote completion uses the common `Completed` status; transport, protocol, and
backend failures use `Failed(CoreError)`.

`API-DIRECTORY-POLL-001`: The portable Directory client surface uses
caller-owned, bounded, generation-bearing operation slots. The following is a
normative public API skeleton; concrete request-view names may be split, but the
start, poll, cancel, ownership, and terminal semantics remain unchanged:

```rust
pub trait PollDirectoryClient {
    fn start_query(
        &mut self,
        request: DirectoryQueryRequest,
        slot: &mut DirectorySessionSlot,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<DirectoryPage>>;
    fn poll_next_page(
        &mut self,
        cx: &mut Context<'_>,
        slot: &mut DirectorySessionSlot,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<DirectoryPage>>;
    fn poll_cancel_session(
        &mut self,
        cx: &mut Context<'_>,
        slot: &mut DirectorySessionSlot,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<CleanupOutcome>>;

    fn start_watch(
        &mut self,
        request: DirectoryWatchRequest,
        slot: &mut DirectoryWatchSlot,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<ProcessEvent<DirectoryChange, DirectoryTerminal>>>;
    fn poll_change(
        &mut self,
        cx: &mut Context<'_>,
        slot: &mut DirectoryWatchSlot,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<ProcessEvent<DirectoryChange, DirectoryTerminal>>>;
    fn poll_cancel_watch(
        &mut self,
        cx: &mut Context<'_>,
        slot: &mut DirectoryWatchSlot,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<CleanupOutcome>>;

    fn start_publication(
        &mut self,
        request: DirectoryPublicationRequest,
        slot: &mut DirectoryPublicationSlot,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<Publication>>;
    fn poll_publication(
        &mut self,
        cx: &mut Context<'_>,
        slot: &mut DirectoryPublicationSlot,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<Publication>>;
    fn poll_cancel_publication(
        &mut self,
        cx: &mut Context<'_>,
        slot: &mut DirectoryPublicationSlot,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<CleanupOutcome>>;

    fn start_resolve(
        &mut self,
        request: ThingResolveRequest,
        slot: &mut ThingResolveSlot,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<TdDocument>>;
    fn poll_resolve(
        &mut self,
        cx: &mut Context<'_>,
        slot: &mut ThingResolveSlot,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<TdDocument>>;
    fn poll_cancel_resolve(
        &mut self,
        cx: &mut Context<'_>,
        slot: &mut ThingResolveSlot,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<CleanupOutcome>>;
}
```

`DirectoryQueryRequest`, `DirectoryWatchRequest`, and
`DirectoryPublicationRequest` own the endpoint reference, operation input,
deadline/cancellation view, security selection, and resource-profile identity.
`ThingResolveRequest` owns the absolute source URI and the same policy context.
A start failure leaves its slot empty. A synchronous `Ready`, terminal poll, or
cancellation consumes the slot generation. Query `Ready` returns the first page
and leaves the slot active only when that page contains a next-page token;
otherwise the slot becomes terminal. Watch `Ready(Item(change))` returns the
first change and leaves the watch active; `Ready(Terminal(status))` represents a
watch that terminates before its first item. Later watch completion,
cancellation, timeout, compaction, authorization change, overflow, and backend
failure are returned exactly once as `ProcessEvent::Terminal` rather than being
collapsed into `Pending`, an empty change, or an implicit end-of-stream.
Directory-specific compaction and authorization-change details use
`ProcessTerminal::Domain(DirectoryTerminal)` so the common process type does
not create a core-to-discovery dependency.
Publication and resolution each have exactly one
terminal result. A client adapter never needs access to Directory storage,
indexes, or server policy objects. Host `DirectoryReader`, `DirectorySession`,
`DirectoryPublisher`, `DirectoryWatch`, and `ThingDescriptionResolver` methods
are async adapters over these same state machines, not independent
implementations.

## Servient

`clinkz-wot-servient` is the application-facing composition root.

The host `Servient` is non-generic. It holds:

- Registry of exposed Things.
- Registry/tracking for consumed Things.
- Default server binding registrations.
- Default client binding registrations under `async`.
- Security providers and optional credential store.
- A `Discoverer`.
- Shared `EventBroker`.
- Per-binding or sharded bounded runtime event sinks plus an aggregate drain
  facade for host self-driving bindings.

The Servient's responsibilities are intentionally narrow:

- `produce(init)` creates an `ExposedThingHandle` from an `ExposedThingInit` or
  Partial TD value accepted by the configured Producer expansion policy.
- `consume(td)` creates a `ConsumedThingHandle`.
- `discover(filter)` starts a lazy Scripting-compatible discovery process over
  bare TD views.
- `explore_directory(url, filter)` starts a Scripting-compatible directory
  exploration.
- `explore_directory_query(directory_ref, query)` starts a directory-native
  exploration.
- `request_thing_description(url)` resolves one bare TD view through discovery.
  `fetch_td(url)` may exist only as a convenience alias with the same bare-TD
  result contract. Rust-native callers that need source metadata use
  `fetch_td_document(url)` or another explicitly named document API.
- Rust-native variants such as `produce_td`, `produce_document`,
  `consume_document`, and the document discovery/fetch methods accept complete
  TD inputs or expose TD source envelopes without changing the
  Scripting-compatible method contracts.
- Runtime status APIs let host applications drain or subscribe to binding
  runtime events without making the Servient own protocol driving loops.
- `Dispatch::serve_request(req)` resolves inbound requests from bindings,
  verifies security, and invokes handlers.

It does not own protocol driving loops.

Constrained profiles use a separate constrained runtime facade or construction
API rather than the host `Servient` shape above. That constrained surface is
allowed to be generic over caller-owned storage, lifetimes, binding slot tables,
or manual drivers, but it must preserve the same operation semantics for the
features it supports. In this document, unqualified `Servient` refers to the
host application-facing type unless the section explicitly discusses the
constrained construction target. Cross-profile documentation must describe the
host and constrained handle representations separately instead of implying that
the host `Arc<dyn ...>` registries are available in `no_std + alloc`.

### ExposedThingHandle

`ExposedThingHandle` is the Producer-facing handle. In the host async profile it
owns cloned `ServerBindingRegistration` entries captured from the Servient
defaults at `produce()` time, including the `Arc<dyn ServerBinding>` plus binding
id, generation, driving mode, form contributor, route-readiness driver,
runtime-event sink configuration, and overflow policy. In constrained profiles,
the equivalent handle uses binding
slots, static binding tables, or manually driven adapters instead of `Arc`. The
captured binding registration set is a snapshot: server bindings registered or
removed from the Servient after `produce()` do not affect that handle unless an
explicit Rust extension API creates a new handle or replaces the handle's binding
set before exposure.

Lifecycle:

1. `produce(init)` captures the server binding registration snapshot, performs
   binding-independent expansion, and validates the accepted
   `ExposedThingInit` or Partial TD input enough to create a draft handle with
   an effective TD candidate and preserved source view. Rust-native producer
   entry points such as `produce_td` and `produce_document` create the same
   handle shape from complete TD inputs or TD source envelopes without changing
   the Scripting-compatible `produce(init)` contract.
2. Handler setters attach or replace operation handlers.
3. `expose()` performs binding-dependent finalization against the captured
   binding registration snapshot, including generated forms, generated `href`
   values, binding-specific security material, endpoint collision checks, and
   strict validation of any application-supplied forms. It then freezes the TD,
   builds protocol-neutral inbound plans, inserts the Thing into the servable
   registry in a preparing state that dispatch can resolve for validation but
   must not treat as serving, and calls `ServerBinding::prepare` on each
   handle-owned binding.
4. After every binding prepares successfully, `expose()` drives or observes any
   registration-provided route-readiness items for those prepared guards until
   they are ready or fail.
5. After every required route-readiness item is ready, `expose()` calls
   `ServerBinding::activate` with each prepared guard, stores the returned active
   guards in the registry entry or equivalent handle-owned state, and keeps
   activated bindings externally quiesced.
6. After every binding activates successfully, `expose()` calls
   `ServerBinding::commit` on each active guard while the registry entry remains
   not serving.
7. After every binding commits successfully, `expose()` atomically marks the
   registry entry serving. Bindings release any activation gates by observing
   that final registry state through `BindingContext`.
8. If any prepare, readiness, activate, or commit step fails, successful
   bindings are aborted or shut down through the explicit cleanup surface.
   Guards remain retained until cleanup completes, becomes residual, or
   transfers to the reserved cleanup owner. The non-serving registry entry is
   removed while the retained cleanup/status record remains addressable.
9. `destroy()` marks the registry entry draining before binding shutdown,
   rejects new inbound requests, waits for already-dispatched handlers only up to
   the configured drain policy, drives `ServerBinding::shutdown` within the
   cleanup budget, and releases stored active route guards only after cleanup
   completes or transfers to a retained cleanup owner. It removes the registry
   entry after local dispatch ownership is closed while retaining any required
   cleanup/status record.

The destroy drain policy is part of the exposed handle configuration. The
portable default is bounded draining: requests already dispatched before the
draining transition may finish until a configured deadline or manual-driver step
budget expires. Results are sent only while the matched route and binding remain
able to accept responses; otherwise the result is discarded and a structured
runtime status records the Thing id, operation, correlation id when known, and
discard reason. When the drain budget expires, the runtime marks remaining
in-flight requests cancelled, drops response delivery for them, and proceeds with
binding shutdown. The engine cannot forcibly stop arbitrary user code, so handler
traits and host adapters must observe cancellation state where they can block or
perform long-running work. Constrained runtimes express the same policy through
manual-driver progress and bounded cleanup steps.

The TD affordance set is frozen after `expose()`. Handlers may be replaced
throughout the exposed lifetime. Dynamic affordance add/remove after
`expose()` is not part of v1.

The lifecycle order is part of the design contract. It prevents a binding from
receiving a request for a Thing that the Servient cannot yet resolve, and it
prevents new requests from entering once `destroy()` starts.

`HANDLE-DROP-001`: `destroy()` is the only Producer API that can report full
drain and cleanup results. Dropping a draft handle releases its private
reservations synchronously. Dropping a preparing or serving host handle requests
the applicable `Cancelling` or `Draining` transition exactly once and transfers
cleanup ownership to the Servient cleanup executor or manual runtime; it MUST
NOT block the destructor on user code or network progress. If no cleanup
executor is configured, a host
builder that permits exposure MUST retain cleanup work in a bounded Servient
queue that applications drive explicitly. Exhausting that reserved queue makes
the initiating lifecycle operation return `Cleanup`/`LimitExceeded`; it never
causes a live route guard to be forgotten. Constrained drop follows
`CONSTRAINED-PROGRESS-001` and leaves a generation-bearing cleanup handle or
runtime-owned work item. Status APIs retain the final `CleanupOutcome` after the
public handle is gone.

`PRODUCER-EMIT-001`: `emit_property_change(name, payload)` and
`emit_event(name, payload)` are valid only for an effective TD affordance that
advertises the corresponding observable property or event data semantics. They
validate the application payload before publication under the handle policy,
snapshot the active local subscriber set without holding engine locks across
codec or binding calls, and enqueue one logically ordered item per matching
subscription. Property change emission does not invoke the property read
handler. Event emission does not invoke subscribe handlers. A missing
affordance, non-observable property, incompatible payload, non-serving handle,
or exhausted fail-closed budget returns a structured error.

Local enqueue and protocol publication outcomes are represented by a bounded
`EmissionStatus`: accepted subscriber count, bindings accepted, bindings failed,
and loss counts when known. The Scripting-compatible convenience call succeeds
only when every required binding publication was accepted; Rust-native methods
may return the non-lossy per-binding status. Subscriber queues apply their own
configured overflow policies, so drop-oldest sample loss is observable but does
not retroactively turn a successfully accepted emission into an unclassified
error. Concurrent emissions for the same Thing and affordance are serialized at
one publication sequence point; cross-affordance ordering is unspecified.

Exposure compiles the required publication targets from the owning inbound
plans. Emission invokes only binding generations that own an applicable
observable-property or event target; it does not scan every registered binding.
The host `ServerBinding::publish` surface and the constrained
`start_emission`/`poll_emission` surface return one `BindingPublication` per
targeted binding generation.

Capacity for the complete bounded per-binding result set is admitted before the
first binding publication begins. A Rust-native emission MUST NOT truncate
binding outcomes. Partial acceptance after admission is returned in
`EmissionStatus`. The source payload remains in one immutable emission-owned
storage allocation while local subscribers and binding publications reference
it. Host adapters MAY use shared host storage; constrained adapters use a
caller-owned arena lease or an equivalent bounded representation. Neither
representation may copy payload bytes once per subscriber or binding target.

A pending constrained emission retains generation-safe local-subscriber and
binding-target cursors in its `ServerEmissionSlot`. A later step resumes from
those cursors, and a later emission for the same Thing and affordance MUST NOT
pass it. Cancelling the slot follows the common cleanup ownership contract and
retains a terminal per-binding result for every publication already accepted.

### ConsumedThingHandle

`ConsumedThingHandle` is the Consumer-facing handle. It owns a `ConsumedThing`
that was populated with precompiled interaction plans. In the host async profile,
those plans may include cloned `Arc<dyn ClientBinding>` references captured from
the Servient defaults at `consume()` time. In constrained profiles, the same
logical plans refer to binding slots, static binding indexes, or owned/manual
adapters.

It provides async methods for Scripting API operations:

- `read_property`, `write_property`, and `observe_property`.
- `invoke_action`, plus Rust extension methods `query_action` and
  `cancel_action` for TD action lifecycle operations.
- `subscribe_event`.
- `observe_all_properties` and `subscribe_all_events` as Rust extension methods
  for collection-level subscription operations.
- `read_all_properties`, `read_multiple_properties`, and
  `write_multiple_properties`.
- Rust extension method `write_all_properties` for TD `writeallproperties`
  forms.
- Subscription teardown through `Subscription::stop` and optional handle-level
  Rust convenience methods.

`invoke_action` returns an `InteractionOutput`. Its normalized
`InteractionStatus` is separate from the single response payload. When the
selected binding or TD operation exposes asynchronous action tracking,
`InteractionOutputMetadata` contains an `ActionInvocationRef`.
`query_action` and `cancel_action` take the action name plus either an
`ActionInvocationRef` or caller-supplied
`InteractionOptions` data required by the selected `queryaction` or
`cancelaction` form. `query_action` and `cancel_action` return a validated
`InteractionOutput`; an action-domain status response uses the single payload
with `ResponsePayloadRole::OperationStatus`. Their `InteractionStatus` describes
only successful request completion, and request-level failure is a `CoreError`.
If a TD exposes query or cancel forms but no invocation identity can be derived
from the original action interaction or caller options, the method fails with a
structured missing-action-reference error.

Streaming operations return local subscription streams while the handle stores
wire-side guards so protocol resources are released on explicit teardown or
handle drop. Collection-level teardown is normally performed through
`Subscription::stop`; optional handle-level helpers for
`unobserve_all_properties` and `unsubscribe_all_events` follow the same
`SubscriptionId` ambiguity rules as single-affordance helpers.

## Security

Security resolution must follow TD 1.1 inheritance rules and must be shared by
inbound and outbound paths.

### Effective Security

For every selected form:

1. If the form declares `security`, use the form's security references.
2. Otherwise inherit Thing-level `security`.
3. Resolve each reference against `securityDefinitions`.
4. Preserve form-level `scopes`.
5. Preserve security scheme type and definition name.

Provider matching rules:

- The security definition name identifies the TD requirement and credential
  lookup key.
- The security scheme type identifies which `SecurityProvider` implementation
  can process the requirement.
- A provider may additionally opt into specific definition names.
- Provider capability checks are side-effect-free. A provider must expose, or be
  wrapped by, an equivalent probe operation that can answer whether it can verify
  or apply a security requirement without mutating credential state, consuming
  one-time material, refreshing tokens, writing request metadata, or recording a
  successful authorization.
- Committed verification or application is a separate step performed only after
  the effective security branch has been selected for the request.

Multiple named security definitions on a Thing or form are interpreted as an
AND requirement. Combo security schemes preserve their `allOf` and `oneOf`
semantics. Unsupported combo semantics, missing referenced definitions, and
missing provider capability for a required scheme must fail explicitly during
validation or plan construction instead of weakening security at dispatch time.
Concrete credential availability is a per-call selection concern unless the
caller selected a static credential admission profile that intentionally requires
credentials to exist before a handle can be constructed.

For TD 1.1 compatibility, the engine accepts multiple names in a `security`
array and evaluates them as an AND requirement. New generated TDs should prefer
combo security schemes for explicit AND/OR structure, while preserving incoming
documents round-trip.

`nosec` is a real security scheme, not the absence of security. It must be
represented in the plan so bindings and dispatch can distinguish "public by
design" from "invalid or missing security metadata".

Security planning expands combo schemes recursively into a structured effective
security expression. Inbound verification evaluates that expression against
available `AuthMaterial`; outbound application evaluates it against available
credentials. `allOf` requires every child expression to pass. `oneOf` requires
one child expression to be chosen for use and must fail if no supported and
credentialed child can be applied. Selection is deterministic: explicit caller
selection wins when exposed by a Rust option, otherwise children are considered
in TD declaration order after binding support, provider capability, and
side-effect-free credential-availability probes are applied. The immutable plan
stores the expression and deterministic branch order. The selected branch path
is stored in the per-call request or verification context; it may be cached in a
plan only under a static credential admission profile that guarantees the choice
cannot vary between calls. `oneOf` is not an instruction to merge credentials
from multiple branches. Inbound verification probes branches without provider
side effects and authorizes the first branch, in declaration order, that fully
verifies and satisfies scopes. `nosec` passes without credentials only when it
appears as an explicit resolved scheme in the effective expression. A successful
`nosec` verification yields an anonymous public `Principal` with no granted
scopes; scope checks still run and fail closed when scopes are required but
unavailable.

### Inbound Security

Inbound security is binding-assisted and core-verified:

1. A server binding maps transport-native credentials into
   `TransportAuthMaterial` without interpreting payload body fields.
2. The dispatch path resolves the immutable matched Thing, affordance,
   operation, form, effective security, and scopes.
3. Core decodes the payload once and projects body-location authentication
   material plus the application payload view through the compiled plan.
4. A matching `SecurityProvider` verifies the combined transport and body
   material and produces a `Principal`.
5. The dispatcher checks required scopes against the `Principal`.
6. The principal and application payload projection are attached to
   `InteractionInput` before handler dispatch; raw authentication material is
   not.

Bindings may reject requests earlier when a protocol cannot carry the required
credentials, but the protocol-neutral dispatch path remains the authority for
TD security semantics.

### Outbound Security

Outbound security is request-applied:

1. The consumed interaction plan carries effective form security.
2. Binding and provider capability are checked without side effects. For
   `oneOf` without an explicit caller choice, the credential store performs a
   side-effect-free availability probe for candidate branches in TD declaration
   order. This probe may report availability and credential properties needed
   for selection, but it must not return secret bytes, consume one-time
   credentials, refresh tokens, or update credential state.
3. After a branch is selected, the credential store retrieves or commits the
   credentials required by that branch. For `allOf`, this means every leaf
   requirement. For `oneOf`, no unselected branch is committed. A retrieval or
   commit race that makes the selected credentials unavailable fails that call;
   automatic fallback is allowed only by the caller's selection policy and must
   restart side-effect-free selection without having committed the failed
   branch.
4. Matching `SecurityProvider::apply` calls write protocol-neutral request
   metadata into `BindingRequest::applied_security`. This metadata can include
   URI-template values for security schemes whose location is `uri`, header or
   body fields, and protocol-specific opaque data.
5. The shared planning layer expands the resolved target URI template after
   caller URI variables and credential-derived URI variables are available.
6. The binding maps the applied metadata to protocol wire representation.

Private security data must not be exposed through application-facing Thing
handles. Application code can register providers and credential stores, but it
cannot inspect stored secrets through the WoT API surface.

Credential lookup and provider application must avoid side effects while
probing `oneOf` branches. A failed branch probe must not mutate credential
state, consume one-time credentials, refresh tokens, write request metadata, or
emit audit success records. Side effects occur only after the selected branch is
committed for the request. Providers that cannot separate probe from commit are
not eligible for automatic `oneOf` fallback; callers must select such a branch
explicitly or plan construction must fail closed for that security expression.
Credential stores used for automatic `oneOf` selection must provide the same
probe/commit separation. A store that cannot do so is eligible only for explicit
branch selection or a static credential admission profile.

`SEC-PERF-001`: Provider capability, supported definition names, security
expression structure, branch order, and schema/location metadata are resolved
once per provider/plan generation. Per-call automatic selection probes only
credential or principal material whose availability can actually change. A
generation-aware availability or selected-branch cache MAY be used when the
credential store publishes invalidation generations. Cache misses and invalid
entries fail or re-probe within the per-interaction provider-probe budget; they
never cause an unbounded provider registry scan.

Provider capability caching is keyed by both provider registration generation
and `SecurityProvider::generation()`. An immutable provider returns a constant
generation. A provider whose supported schemes, definition names, algorithms,
or probe semantics can change MUST increment its generation before publishing
the change. A generation change invalidates affected capability and branch
caches before another call can use them; credential availability remains keyed
separately by `CredentialStore::generation()`.

## `no_std + alloc` Boundary

The `no_std + alloc` compilation environment includes:

- TD/TM construction, serde, validation, and round-trip behavior.
- Core interaction types and local dispatch.
- Protocol-neutral form selection and URI-template helpers.
- Discovery data models.
- Async trait surfaces when enabled without a runtime dependency.
- Binding adapters that can be driven by an integration or manual progress loop.
- Poll-based or otherwise manually driven client/server binding surfaces for
  constrained builds. Boxed-future `Arc<dyn ClientBinding>` registration is the
  host erased surface, not the only embedded binding contract.

The `no_std + alloc` compilation environment excludes required dependencies on:

- Filesystem-backed storage.
- OS sockets and process APIs.
- Thread spawning.
- Tokio or other async runtimes.
- Concrete host network backends unless gated behind `std`.

The full host `ServientBuilder` can be `std`-only. A constrained runtime must
still have a documented construction path for the pieces it supports, such as
static registries, local dispatch, manually driven bindings, and precompiled
plans.

The constrained construction target is explicit even if the exact type names
evolve:

- Applications provide static or caller-owned registries for Things, handlers,
  consumed plans, subscriptions, and binding slots.
- Server and client bindings are registered by stable slot ids or indexes into
  caller-owned tables, not by `Arc<dyn ...>`.
- Precompiled inbound and outbound plans store binding slot ids plus compiled
  form metadata. They must not store boxed futures, spawned task handles, or
  host-only synchronization primitives.
- A manually driven runtime exposes poll or step methods for accepting inbound
  requests, dispatching local handlers, driving outbound requests, draining
  subscriptions, and running cleanup.
- Handles reference registry entries and binding slots through bounded ids or
  lifetimes owned by the constrained runtime. Dropping a handle schedules or
  performs bounded cleanup through the same manual driver.
- The constrained API may omit host conveniences such as dynamic binding
  registration and background discovery watchers, but supported operations must
  preserve the same validation, security, form-selection, and lifecycle
  semantics as the host API.

No-default-feature builds for crates that claim `no_std + alloc` support must expose a
useful constrained surface, not only an empty crate root. At minimum, the
relevant crate roots must keep the protocol-neutral data types, TD/TM
construction and validation types, plan construction inputs and outputs, local
dispatch entry points, poll or step-driven binding traits, bounded identifiers,
capacity policy types, and error/status enums that constrained applications need
to build static registries and manually driven runtimes. Host-only builders,
`Arc<dyn ...>` registration, spawned drivers, filesystem storage, sockets, and
concrete network backends remain behind `std` or protocol-specific runtime
features.

### Constrained Reference Storage Model

`CONSTRAINED-STORAGE-001`: The reference constrained runtime is built from
caller-owned bounded arenas or tables for Things, logical plans, binding slots,
handler slots, subscriptions, pending requests, and cleanup work. Each externally
retained slot reference contains an index and generation. Removing a slot
increments its generation before reuse; a generation mismatch returns a stale
handle error and never aliases a new resource. An implementation with a finite
generation counter MUST stop reusing the slot before wraparound can make a live
stale handle valid again.

`CONSTRAINED-STORAGE-002`: Construction reserves all table capacities from a
`StaticResourceProfile`. Admission reserves every slot and byte budget needed to
publish a Thing before publication. Exhaustion returns `LimitExceeded` without
evicting a live object. Variable-size TD, extension, schema, plan, and payload
data may use `alloc`, but every allocation is charged to a profile byte budget;
the design does not claim a heapless profile.

`CONSTRAINED-PROGRESS-001`: A manual runtime step accepts an explicit work
budget expressed as maximum state transitions and typed work units. It
returns `Idle`, `Progress { value, pending }`, or a structured terminal/error
state. `value` carries at most one event produced by the step; `pending` is an
`Option<PendingWork>` and `PendingWork` is nonempty whenever present. `None` is
valid when the call completed a transition or produced an event but no
maintained state reports remaining work. An event and pending work can therefore
be reported by the same step without an extra table scan. Cleanup uses a
reserved bounded queue that is part of construction. If
the queue is full, an explicit call performs cleanup synchronously within the
caller's supplied work budget or returns a cleanup-pending handle that retains
ownership; cleanup is never silently dropped.

`CONSTRAINED-WORK-001`: A work unit names its bounded cost class rather than an
arbitrary implementation transition. At minimum profiles bound JSON/schema
nodes, codec input/output bytes, URI output bytes, security branches and
provider probes, queue operations, binding progress calls, and cleanup items per
step. A poll/step implementation MUST NOT decode an unbounded payload, walk an
unbounded collection, expand an entire unbounded target, or complete unrelated
queued work inside one charged unit. Non-incremental cryptography, codec calls,
or application handlers declare their maximum admitted input and external
worst-case execution responsibility.

`CONSTRAINED-SCHED-001`: Manual progress is fair across binding input, response
delivery, outbound requests, subscription data, timers, and cleanup. The runtime
retains round-robin or equivalent generation-safe cursors and reserves profile
quota for response delivery and cleanup before admitting new work. A perpetually
ready binding or Thing MUST NOT starve another admitted owner. `Progress`
reports the class of pending work so an application can schedule the next step
without scanning every table.

`CONSTRAINED-OWN-001`: Constrained handles use lifetimes, unique ownership, or
slot references into the runtime tables and MUST NOT require `Arc` or pointer-
width atomic instructions. Sharing across execution contexts is supplied by the
application through its chosen critical-section or message-passing boundary.
The runtime never holds a critical section while allocating or calling user
code.

## Resource Limits and Admission Budgets

Every TD, TM, payload, discovery result, security expression, URI template, and
extension value is untrusted variable-size input. Queue item counts alone are
not a resource bound.

`RES-LIMIT-001`: Public ingestion and runtime construction surfaces MUST accept
or inherit a `ResourceLimits` policy. `docs/resource-limits.csv` is the
exhaustive field schema and named-profile value source. The categories below
explain that schema; they are not a second field list:

- document, payload, extension, generated effective-document, and compiled-plan
  byte limits;
- retained source, admission temporary, peak live, largest contiguous
  allocation, validator/cache, and global compiled-runtime byte limits;
- JSON nesting depth, members per object, array length, and total value nodes;
- Thing affordance count, forms per context, total forms, and URI variables;
- schema nodes, reference/composition depth, and validation work units;
- security expression depth, branches, provider probes per interaction, and
  ordered form-binding candidates per operation;
- URI-template length, variable count, and expanded target length;
- Things, bindings, sessions, subscriptions, discovery processes, publishers,
  watchers, and concurrent queries at per-owner and global scopes;
- binding and form-contributor probes per admission, wildcard candidates, and
  lazy compiled-plan slots;
- queue item count, total queued bytes, and maximum bytes per item;
- cleanup work items and bytes, endpoint reservations, page-token bytes, and
  outstanding response opportunities;
- cache and durable-status-record entries, bytes, generations, and retention;
- Directory query bytes, depth, nodes, terms, strings, pages, watches, and
  incremental decode scratch;
- Producer emission slots, bounded per-binding results, fan-out cursors, and
  subscribers/bindings processed per step;
- hierarchical accounting batch, idle-reservation, reconciliation work, and
  reconciliation interval or step limits.

`RES-LIMIT-002`: A limit violation MUST return a structured `LimitExceeded`
category identifying the resource kind, configured limit, observed or requested
amount when safely known, and processing phase. Parsing or validation MUST stop
once its work budget is exhausted. It MUST NOT partially admit a document or
silently truncate candidates, schemas, security branches, or extension data.

`RES-LIMIT-003`: Limits compose hierarchically: per item, per Thing or
Directory client, per principal or publisher when known, per binding or adapter, and
global. Admission MUST reserve capacity before publishing externally reachable
state and release the reservation idempotently during rollback or cleanup.

`RES-LIMIT-004`: Host profiles provide documented conservative defaults.
Constrained construction MUST receive explicit limits or select a named static
profile. A zero byte or count limit disables that resource unless the field is
explicitly documented as rendezvous capacity. No zero value means unbounded.

The default gateway profile uses the following selected admission ceilings.
This table is a readable summary; `docs/resource-limits.csv` is exhaustive and
authoritative when a field is not shown here. Applications may lower values
directly and may raise them only through explicit configuration:

| Resource | Default gateway ceiling |
| --- | ---: |
| TD/TM document bytes | 1 MiB |
| payload bytes per item | 1 MiB |
| JSON nesting depth | 64 |
| JSON value nodes per document | 65,536 |
| members per object or items per array | 8,192 |
| total affordances per Thing | 1,024 |
| forms per context / total forms | 32 / 4,096 |
| schema nodes / composition depth | 65,536 / 32 |
| document/schema validation work units per admission | 1,048,576 |
| payload validation nodes per interaction | 65,536 |
| security expression depth / branches | 16 / 64 |
| provider probes per interaction | 64 |
| form-binding candidates per operation | 32 |
| URI variables / expanded URI bytes | 64 / 16 KiB |
| extension bytes per document | 256 KiB |
| generated effective-document bytes per Thing | 2 MiB |
| retained source bytes per Thing / global | 1 MiB / 512 MiB |
| admission temporary bytes per operation / global | 8 MiB / 256 MiB |
| admission peak live bytes per operation / aggregate | 24 MiB / 512 MiB |
| total engine live bytes global | 2 GiB |
| compiled runtime bytes per Thing / global | 4 MiB / 1 GiB |
| validator/cache bytes per Thing / global | 2 MiB / 256 MiB |
| largest contiguous engine allocation | 8 MiB |
| binding/contributor probes per admission | 16,384 |
| wildcard binding/contributor probes per admission | 64 |
| concurrent lazy plan slots per Thing / global | 32 / 4,096 |
| active Things / bindings | 4,096 / 64 |
| active Directory sessions per principal / global | 16 / 1,024 |
| active Directory publications per principal / global | 16 / 1,024 |
| active subscriptions per Thing / global | 1,024 / 65,536 |
| active discovery processes per principal / global | 16 / 1,024 |
| active watchers per principal / global | 64 / 8,192 |
| pending cleanup items / bytes | 1,024 / 4 MiB |
| endpoint reservations per Thing / global | 4,096 / 262,144 |
| page token bytes | 4 KiB |
| URI template source bytes | 16 KiB |
| remote resolver requests / redirects / bytes | 8 / 4 / 1 MiB |
| in-flight responses per Thing / global | 1,024 / 65,536 |
| subscription queued bytes per subscription / global | 1 MiB / 512 MiB |
| cache entries per Thing / global | 4,096 / 262,144 |
| durable status entries and bytes per binding | 64 / 64 KiB |
| durable status bytes global / retention | 16 MiB / 24 hours |

Peak limits count engine-owned bytes that are actually live, including arena,
pool, heap, and caller-provided capacity while it is exclusively reserved for
the engine. They do not count a logical quota that has not acquired physical
storage. `peak_live_bytes_per_admission_max` is attributed to one active
admission transaction and includes its source, temporary, and not-yet-published
persistent state. `admission_peak_live_bytes_global_max` aggregates those bytes
across all simultaneously active admissions.
`engine_live_bytes_global_max` covers all live engine-owned accounts, including
published documents/runtime state, queues, caches, diagnostics, cleanup, and
active admission. Individual ledger ceilings remain independently enforced even
when this total has capacity. A reserved virtual address range or static array
is charged by committed physical storage on host profiles and by its full linked
or caller-dedicated capacity on static profiles; benchmark results state which
representation applies.

`RES-PROFILE-001`: A named host profile is represented by an exhaustive,
versioned `ResourceLimits` value. Every field is a concrete bounded value or an
explicit `NotApplicable` for a capability the profile does not expose; neither
field omission nor zero means unlimited. Fields summarized in the table above
use those values directly. The CSV spelling `NA` maps only to the typed
`NotApplicable` value; empty cells, `inherit`, and `unbounded` are invalid.
Queue item and byte defaults are explicit CSV fields rather than a multiplication
rule. Hierarchical scopes that are not independently configured inherit the
stricter applicable parent or global ceiling at runtime, but every named-profile
snapshot still contains a concrete value for every applicable field. A
compile-time profile snapshot test MUST fail when a new `ResourceLimits` field
lacks a value in any named profile.

`DirectoryClientDefaultV1` is the engine-side remote Directory adapter profile.
Its CSV snapshot repeats every applicable document and payload structural value
rather than relying on implicit inheritance. It admits 16 active sessions,
queries, and publications per principal, 1,024 of each globally, 64 active
watches per principal, 8,192 globally, a maximum page of 128 TD documents, and
16 MiB of total buffered Directory response bytes. It also bounds query bytes,
depth, nodes, terms, strings, watch/change bytes, and incremental decode scratch.
It does not define Directory storage, snapshot retention, query execution, or
service capacity. Those limits belong to the deferred service design.

These values are admission quotas, not instructions to preallocate
`item_capacity * maximum_item_size` for every owner. Physical allocation follows
`SUB-STORAGE-001` and the equivalent discovery, watch, and event-pool policy.

No universal constrained numeric default is implied: target memory budgets vary
too widely. A constrained constructor therefore MUST receive every capacity or
select a named application-defined `StaticResourceProfile`; omission is a
construction error. Implementations MAY provide example profiles, but MUST NOT
silently select one.

`BenchmarkStaticReferenceV1` in the constrained performance manifest is a
reproducible verification fixture, not a runtime default. Its numeric capacities
are the exhaustive snapshot in `docs/resource-limits.csv`; the manifest
references that profile and MUST NOT carry a divergent partial copy. The values
define the maximum benchmark case and MUST be recorded with the result. A
product may reuse it only by selecting it explicitly as an application profile.

Schema and security work budgets count deterministic local units such as visited
nodes, evaluated branches, provider probes, and reference edges. Wall-clock
deadlines MAY supplement these budgets on host systems but MUST NOT be the only
denial-of-service control. Remote context or model resolution has separate size,
redirect, request-count, and elapsed-time limits.

## Performance Contract

Performance is a design requirement, not an implementation afterthought.

### Hot Path Rules

Inbound dispatch hot path:

1. Correlation id lookup and route metadata lookup are expected O(1), or worst
   case O(log n) for an ordered bounded table.
2. Thing registry lookup is O(log n) or better.
3. Affordance/operation lookup uses precompiled indexes.
4. Handler dispatch supports the zero-allocation configuration defined by
   `PERF-ALLOC-001`.
5. No lock or critical section spans user handler execution unless the handler
   explicitly owns that lock.

Outbound interaction hot path:

1. Target/operation lookup uses a precompiled plan.
2. Binding selection uses precomputed candidate ordering.
3. Static security selection metadata is reused; provider-created wire metadata
   is charged to the per-interaction allocation and byte budget.
4. Payload body bytes are moved or reference-counted, not copied by default.
5. URI variable expansion writes into caller-provided scratch storage when
   available, otherwise it allocates at most one final target string within the
   expanded-target byte limit.

Subscription hot path:

1. Queues are bounded.
2. Backpressure behavior is explicit.
3. Static metadata is compiled into the subscription plan and is not reparsed
   per sample.

### Host Concurrency Topology

`HOST-SHARD-001`: Host concurrency MUST NOT serialize unrelated Things or
bindings behind one mutable registry, subscription, in-flight, or runtime-event
lock. Registry structural mutation is separate from per-Thing dispatch state.
After lookup, dispatch retains a generation-safe entry and releases the
structural registry guard before in-flight admission, security work, payload
validation, status publication, or user code. Each Thing owns or addresses its
own lifecycle/in-flight state; handler slots are independently publishable.

`HOST-SHARD-002`: Runtime events, durable status, and loss counters are stored
per binding or per bounded shard. A public Servient drain API MAY merge those
streams, but producers do not contend on one mandatory global queue. Critical
lifecycle state updates its fixed/bounded durable record directly before any
best-effort aggregate notification. Diagnostics from one binding MUST NOT apply
backpressure to unrelated binding interaction data unless an explicit global
shutdown policy was selected.

`HOST-ASYNC-001`: The object-safe boxed-future binding API is a compatibility
surface, not the only host execution path. Bindings MAY expose native async,
poll-based, or reusable operation-slot adapters that avoid one heap allocation
per interaction. Benchmark manifests report allocation count and bytes for both
the erased convenience path and every advertised allocation-sensitive path.
Request/response pools are bounded and generation-safe; pool exhaustion applies
backpressure rather than falling back to unbounded allocation.

### Allocation Policy

`PERF-ALLOC-001`: The following successful paths MUST support a zero engine-heap
allocation configuration after setup when using sync handlers, already-owned
payload buffers, preallocated queue/status capacity, no URI expansion, and
non-allocating metrics:

- local property read;
- local property write;
- local action invoke;
- local event emit into already-allocated broker capacity;
- inbound dispatch to sync handlers.

Handler-created output and caller-requested owned TD materialization are outside
the engine-allocation count. Reference-count operations are not allocations when
they do not grow storage. Error, overflow, URI-expansion, security-material
creation, multi-subscriber fan-out that requires new storage, and diagnostics
paths have separately measured allocation budgets and are not covered by
`PERF-ALLOC-001`.

Network-bound client calls may allocate for protocol framing, async trait
dispatch, and runtime integration only within their manifest allocation budget.
I/O latency alone is not evidence that allocator contention or tail-latency cost
is acceptable.

### Benchmark Targets

The verification suite MUST include benchmark or measurement hooks for:

- local sync handler dispatch latency;
- inbound request dispatch latency excluding transport I/O;
- consumed plan lookup latency;
- bounded candidate and security-branch selection latency;
- composed interaction latency with codec, strict schema validation, security
  selection/application, and dispatch measured in one operation;
- subscription enqueue/dequeue throughput and multi-subscriber fan-out latency;
- Directory client request planning, page/change ingestion, cancellation, and
  bounded-buffer overflow latency;
- lazy-plan cold-start single-flight and generation invalidation work;
- hierarchical resource-accounting contention under maximum concurrency;
- incremental Directory page peak residency and bounded fan-out resume work;
- heap allocations per hot-path operation;
- code size for selected `no_std + alloc` profiles.

`PERF-BENCH-001`: The repository MUST define benchmark manifests for constrained,
gateway, and Directory-client profiles. Each manifest records target
architecture, toolchain, feature set, allocator, fixture-generator version and
digest, TD node/form/binding/schema/security scale, payload sizes, concurrency,
warm-up, sample count, and reported statistics. Each workload defines the
operation start and stop events, whether setup and codec work are included, the
allocation-accounting boundary, and the units for peak-live bytes and lock wait,
either directly or by an explicit inherited manifest measurement block.
Latency benchmarks report at least median and p95; host concurrency benchmarks
also report p99. A gating workload MUST contain at least one absolute numeric
budget or deterministic structural invariant. A report-only workload is marked
as characterization and cannot satisfy a requirement's performance gate.
Throughput minima regress when values decrease; latency, work, allocation,
memory, lock-wait, and code-size maxima regress when values increase. Integer
zero budgets remain exact and are not widened by percentage tolerances.
Every workload and contention case has a globally unique stable id, positive
version, requirement-id set, fixture id, harness case, and gating or
characterization marker. `docs/performance/manifest.schema.json` defines the
manifest shape, `docs/performance/fixtures.lock.toml` locks generated bytes,
and `docs/performance/result.schema.json` defines adapter results. The
deterministic generator and orchestrator in `tools/performance-harness` MUST
regenerate and hash actual document, payload, page, actor, and subscriber
bytes; reject identity, boundary, or requirement drift; and reject a result
whose manifest, fixture, workload, measurement, profile, feature, toolchain,
allocator, or runner identity does not match.

`PERF-BENCH-002`: Before design implementation is declared complete, each
manifest MUST record numeric baselines for latency, throughput, peak live bytes,
allocation count, plan-construction time and bytes, and applicable code size.
A change fails the performance gate when a metric regresses by more than the
manifest's declared tolerance without an approved baseline update. The default
tolerance SHOULD be 10% for latency, throughput, allocation count, and peak
memory. The default code-size tolerance is also 10%; a manifest may choose a
stricter value. Design freeze requires the
manifest schema, workloads, metrics, and tolerances; measured numeric baselines
are an implementation-completion artifact because they depend on the selected
toolchain, target, allocator, and implementation.

`PERF-BENCH-003`: Plan benchmarks MUST include small, maximum-admitted, and
adversarial documents. They report scaling against forms, bindings, schema
nodes, and security branches so eager compilation or accidental form-binding
Cartesian duplication is visible.

`PERF-SCALE-001`: Performance verification separates byte size from structural
node count. Each profile includes one-axis fixtures for document bytes, string
bytes, extension bytes, URI-template input and expanded output bytes, payload
bytes, value/schema nodes, forms, matching binding probes, security branches,
candidates, and subscribers where applicable. Deterministic counters verify the
declared complexity and pass bounds: bytes consumed and produced, nodes and
reference edges visited, candidates visited, provider and binding probes,
payload copies, logical plans, binding-specific states, and persistent bytes.
Wall-clock or cycle budgets supplement these counters; they cannot by themselves
prove that an algorithm is linear or that static metadata is not duplicated.
The fixture digest covers generated bytes, not only generator parameters, and
every randomized fixture records a seed.

`PERF-BUDGET-001`: The versioned benchmark manifests under
`docs/performance/` are normative design budgets. Baselines record observed
values; budgets are maximum acceptable values or minimum throughput values and
exist before implementation. A result must satisfy both its absolute budget and
the regression gate. The reference target used for host latency budgets is a
dedicated x86_64 Linux runner with at least four physical cores, fixed
performance governor, isolated benchmark process, and the toolchain recorded in
the result. A different runner establishes a separately approved baseline and
does not overwrite reference results.

Budget results are comparable only when manifest schema, profile, workload,
fixture digest, feature set, target, toolchain, allocator, measurement boundary,
and runner class match. A mismatched result creates a separate result series.
Warm-up or calibration samples are excluded from reported samples. Failed,
cancelled, overflowed, and outlier samples are never silently removed; the
result records their counts and any predeclared statistical exclusion rule.

The Directory-client manifest measures only engine-side request construction,
adapter progress, response admission/decoding, session/watch state, cancellation,
and overflow handling against a deterministic scripted transport. Network
latency and Directory service execution are excluded and reported separately by
an integration when available. No service-backend identity, storage SLO, query
planner, or snapshot-retention result is implied by this manifest.

The initial reference budgets are deliberately broad enough for portable Rust
implementations but strict enough to catch structural regressions:

| Workload | Gateway budget | Constrained budget | Directory-client budget |
| --- | ---: | ---: | ---: |
| Local sync read/write/invoke, warm p95 | 25 us | 100,000 target cycles | n/a |
| Inbound dispatch excluding transport, warm p99 | 100 us | 250,000 target cycles | n/a |
| Compiled plan lookup, warm p95 | 5 us | 20,000 target cycles | n/a |
| Subscription enqueue+dequeue, one subscriber | 250 ns minimum 4 Mops/s | 5,000 target cycles | n/a |
| Candidate selection, 32 admitted candidates, warm p95 | 10 us | 50,000 target cycles | n/a |
| Security selection, 64 local side-effect-free probes, warm p95 | 100 us | 250,000 target cycles | n/a |
| Composed 1 KiB strict interaction, 8 candidates/8 security branches, warm p99 | 250 us | 750,000 target cycles | n/a |
| Shared-payload fan-out, 1,024 subscribers, warm p99 | 1 ms | n/a | n/a |
| Shared-payload fan-out, 256 subscribers, warm p95 | n/a | 500,000 target cycles | n/a |
| Directory request planning, warm p95 | n/a | n/a | 25 us |
| Directory page admission, 128 results, 1 KiB each, warm p95 | n/a | n/a | 2 ms |
| Directory watch-change admission, 1 KiB, warm p99 | n/a | n/a | 100 us |
| Directory cancellation to local terminal state, warm p99 | n/a | n/a | 100 us |
| Maximum-admitted TD planning | 2 s and 24 MiB peak | manifest-sized profile | n/a |

Zero-allocation paths have an absolute engine allocation budget of zero after
setup. Gateway network-bound erased calls default to at most two engine heap
allocations and 1 KiB of engine-owned metadata per operation; a native/poll path
defaults to zero allocations when caller slots and protocol buffers are
provided. Candidate-selection benchmarks cover 1, 8, and 32 candidates;
security benchmarks cover 1, 8, and 64 branches with credential cache hit,
miss, and generation invalidation. Fan-out benchmarks cover 1, 16, 256, and
1,024 subscribers. Contention benchmarks separately measure unrelated Things,
different affordances of one Thing, and one hot subscription.

`PERF-ADMISSION-001`: Admission implementations MUST provide phase peak-memory
instrumentation. With `SourceRetention::MetadataOnly`, parser DOM or raw-value
indexes, validation scratch, contributor scratch, and candidate probe storage
are released before publication unless a documented dependency proves they are
still needed. A generic JSON DOM MUST NOT coexist with a fully materialized
effective TD and complete compiled plans. String, URI, schema, and security
metadata shared by more than one plan are interned or arena-referenced; a
benchmark structural assertion rejects owned duplicate copies.

`PERF-PEAK-001`: A performance budget never overrides a `ResourceLimits`
account. Every benchmark runs with the named profile and fails immediately if
any source, temporary, persistent, validator/cache, largest-contiguous, or peak
live reservation is exceeded, even when the measured latency and manifest peak
budget would otherwise pass. The gateway maximum-admitted planning workload
uses `MetadataOnly`, disables owned effective-TD materialization, runs one
admission in an otherwise quiescent process, and has a 24 MiB peak-live ceiling.
This ceiling is the maximum simultaneous engine-owned memory for that operation,
not permission to move bytes from an exhausted ledger account into another
account. Directory page and publication workloads apply the same rule to
`DirectoryClientDefaultV1`. Measurement distinguishes the operation-attributed
admission peak, aggregate concurrent-admission peak, and total engine live
bytes exactly as defined by the resource schema. Reserved logical capacity is
reported separately and cannot be substituted for an actual-live-byte metric.

`PERF-CALL-001`: The successful interaction path decodes a payload at most once,
performs no registry-wide provider or binding scan, and acquires no more than
one structural registry read guard before reaching generation-safe per-Thing
state. Credential-store and provider probes are counted. A profile MAY set a
lower probe limit than `ResourceLimits`; exceeding it fails selection rather
than increasing tail latency without bound.

`PERF-ACCOUNT-001`: Resource, in-flight, loss, and metrics accounting on an
admitted interaction uses a per-Thing, per-binding, per-shard, reserved local,
or lock-free counter path. It MUST NOT acquire a mandatory global ledger or
metrics mutex for each request, response, or subscription sample. Global limits
are enforced by bounded hierarchical reservations obtained in batches or at
owner admission and reconciled without oversubscription; a local reservation
exhaustion applies backpressure or obtains another bounded batch outside user
code and binding callbacks. Reconciliation, snapshot reporting, and owner
teardown may aggregate shards, but they MUST NOT stop unrelated interaction
progress for work proportional to every active owner. Exact externally visible
capacity and loss totals remain correct despite batching.

Every profile bounds the maximum reservation batch, the unused reservation one
owner may retain while idle, owners reconciled per step, and either a host
reconciliation interval or constrained reconciliation-step deadline. A batch is
charged to the global parent before becoming locally visible. Returning idle
quota and reconciliation use the same fair scheduling class as cleanup; a hot
owner cannot indefinitely retain quota needed by another admitted owner.

`PERF-FANOUT-001`: Subscription fan-out reserves descriptors in bulk where the
storage backend supports it and shares immutable payload storage. It MUST NOT
copy payload bytes per subscriber. Loss-counter and overflow-summary updates use
fixed storage and MUST NOT take a Thing-wide lock. Physical pool capacity is
configured globally and per shard; per-subscription logical quotas never cause
up-front maximum-payload allocation.

`PERF-FANOUT-002`: Fan-out snapshots or generation-pins the applicable
subscriber set without holding a Thing-wide lock while enqueuing. A slow, full,
or cancelling subscriber cannot block delivery to unrelated subscribers beyond
the selected bounded overflow operation. Host profiles shard or batch large
subscriber sets so cancellation, cleanup, and unrelated emissions continue
during a hot fan-out. Constrained profiles retain a generation-safe fan-out
cursor and process at most the caller's subscriber/queue work units per step;
they MUST NOT restart the scan at subscriber zero after budget exhaustion.
Per-affordance publication order is preserved across batches, and a reused slot
generation never receives an item from an older cursor.

## Reliability Contract

Reliability requirements apply to both host and constrained profiles:

- Plan construction is atomic. `produce(init)`, `consume(td)`, and `expose()`
  either publish a complete validated draft, plan, handle, or registry entry, or
  leave no partially usable state behind.
- Compiled plans are immutable after publication. Runtime state such as
  handlers, guards, credentials, and metrics may change behind explicit
  synchronization, but TD-derived route and client plans do not mutate on hot
  paths.
- Lifecycle operations are idempotent where repeated calls are plausible.
  Calling `destroy()`, binding `shutdown`, or subscription teardown more than
  once must not leak resources or panic.
- Library code must not panic for malformed TDs, unsupported operations,
  missing credentials, transport errors, cancellation, or backpressure. These
  conditions return structured errors or observable status items.
- Resource ownership is explicit. Every opened server route, client
  subscription, discovery session, queue, and guard has a documented owner and
  release path.
- Cancellation starts cleanup immediately and prevents new work from entering
  through that handle. Engine-owned async or poll/step cleanup is cooperative
  and bounded by a configured progress budget. Synchronous user handlers are
  non-preemptible as specified by `HANDLER-CANCEL-001`; the engine may stop
  waiting for and discard their late responses but cannot guarantee their
  execution has ended.
- Error categories preserve enough context for diagnosis without exposing
  secrets: Thing id, affordance target, operation, form index or plan id,
  binding protocol, and security definition name where applicable.
- Security never silently degrades. Unsupported schemes, failed credential
  lookup, missing scopes, and unsupported combo semantics fail closed.
- All queues, caches, and registries that can grow from external input have an
  explicit capacity policy or documented backpressure behavior.
- Host integrations may add retries, reconnects, and deadlines, but the core
  engine must expose the original failure cause and must not retry
  non-idempotent interactions unless the caller or binding explicitly opts in.

## Capacity and Overflow Policy

All buffers that can grow because of external input use an explicit capacity
policy. The policy records the logical queue kind, item capacity, byte capacity,
maximum item bytes, owner and global quota, overflow behavior, and whether
overflow is reported inline, through a status stream, through metrics, or
through both status and metrics.

Host defaults are:

- subscription queue: 16 items, drop-oldest with a lost-sample status or metric;
- discovery process buffer: 16 TD candidates, backpressure to the discovery
  producer where possible and otherwise structured terminal overflow; lossy
  profiles may choose drop-newest only when explicitly selected and observable
  through status or metrics;
- each binding runtime event sink: 64 queued event copies, drop-oldest for diagnostic
  and eligible operational events with a lost-event counter unless the
  registration selects backpressure or shutdown-on-overflow; critical lifecycle
  event details are always preserved in the durable binding status record or its
  compacted critical-event summary before any queued copy is dropped;
- directory watch buffer: 64 changes per watcher, compact with an observable
  missed-update status when a watcher falls behind;
- directory result page: 128 entries unless the directory backend sets a lower
  limit;
- Servient cleanup queue: 1,024 items and 4 MiB, fail admission of work that
  cannot retain cleanup ownership.

The subscription item default is a logical per-subscription ceiling. Storage is
drawn on demand from the bounded global descriptor and payload pools and remains
subject to the 512 MiB global queued-byte ceiling; an empty subscription does
not reserve sixteen maximum-size payloads.

These item-count defaults are active only together with the host
`ResourceLimits` byte and concurrency defaults. An item larger than the maximum
item size is rejected before enqueue. Admission fails when either count or byte
capacity would be exceeded; a drop policy releases the dropped item's byte
reservation before admitting its replacement.

`CAP-OVERFLOW-001`: Reporting overflow MUST NOT enqueue another event into the
same full queue. Loss counters and the latest overflow summary use fixed-capacity
or overwrite-in-place storage. If shutdown-on-overflow is selected, shutdown
progress MUST NOT depend exclusively on the blocked producer whose queue
overflowed.

`CAP-STATUS-001`: Durable binding status has explicit per-binding and global
journal entry and byte limits, bounded identity and message fields, retention or
generation-based reclamation after a Thing is removed, and a fixed-size compact
summary fallback. Compaction never allocates from the exhausted queue budget.

`OBS-PROFILE-001`: Every runtime retains fixed-size loss counters and the latest
terminal lifecycle status required for correctness. Historical critical-event
journals, detailed source chains, sampled metrics, and aggregate diagnostic
streams are optional observability profile capabilities. Disabling them does not
remove structured operation errors or latest terminal state. Constrained and
allocation-sensitive profiles default to counters plus latest status; host
profiles may enable bounded journals. Cleanup, response, and interaction data
paths MUST NOT allocate detailed diagnostics when their diagnostic budget is
exhausted.

Constrained profiles must either require capacities during construction or
document smaller profile defaults. A capacity of zero is valid only when it is
explicitly documented as rendezvous/backpressure behavior; it must not mean an
unbounded queue. Overflow status must include the queue kind, Thing id or
directory id when known, binding id when relevant, number of lost items when
known, selected overflow policy, and whether the operation continued, was
cancelled, or was shut down. Overflow reporting must not expose payload bytes,
credentials, or redacted TD fields.

## Implementation Contract

This section closes cross-cutting choices that otherwise force implementers to
invent observable behavior. Names in this section are normative minimum public
API names; implementations may add convenience APIs without weakening these
semantics.

### Frozen Cross-Crate Surface

`API-SURFACE-001`: The following names, ownership rules, and operation shapes
are normative public API. Minor signatures may add generic error conversion or
borrowed convenience overloads, but implementations MUST NOT substitute an
unrelated execution model. Host async adapters wrap these contracts; they do
not replace the constrained contract.

```rust
pub struct WorkBudget { /* typed remaining units */ }
pub struct PendingWork { /* bounded nonempty work-class summary */ }
pub enum StartStatus<T> { Ready(T), Pending }
pub enum ProcessEvent<T, D = ()> { Item(T), Terminal(ProcessTerminal<D>) }
pub enum ProcessTerminal<D = ()> {
    Completed,
    Cancelled,
    TimedOut,
    Overflowed { lost: Option<u64> },
    Domain(D),
    Failed(CoreError),
}
pub enum StepStatus<T> {
    Idle,
    Progress { value: Option<T>, pending: Option<PendingWork> },
    Terminal(T),
}
pub enum CleanupOutcome {
    Complete,
    PendingCleanup(CleanupRecord),
    ResidualExternalState(CleanupRecord),
}
pub struct CleanupRecord {
    /* subject, owner, retry class, and bounded redacted cause */
}
pub struct ProducerEmission {
    /* route, target, sequence, kind, and immutable payload lease */
}
pub enum EmissionKind { PropertyChange, Event }
pub struct BindingPublication {
    /* binding generation, target count, and acceptance outcome */
}
pub struct EmissionStatus {
    /* sequence, local outcome, and bounded per-binding outcomes */
}
pub trait RuntimeClock {
    fn now(&self) -> MonotonicInstant;
    fn ticks_per_second(&self) -> NonZeroU64;
}

pub trait PollClientBinding {
    fn start_request(
        &mut self,
        request: BindingRequest,
        slot: &mut ClientRequestSlot,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<InteractionOutput>>;
    fn poll_request(
        &mut self,
        cx: &mut Context<'_>,
        request: &mut ClientRequestSlot,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<InteractionOutput>>;
    fn start_subscription(
        &mut self,
        request: BindingRequest,
        slot: &mut ClientSubscriptionSlot,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<SubscriptionStart>>;
    fn poll_subscription(
        &mut self,
        cx: &mut Context<'_>,
        subscription: &mut ClientSubscriptionSlot,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<SubscriptionStart>>;
    fn poll_subscription_item(
        &mut self,
        cx: &mut Context<'_>,
        subscription: &mut ClientSubscriptionSlot,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<ProcessEvent<SubscriptionItem>>>;
    fn poll_cancel_request(
        &mut self,
        cx: &mut Context<'_>,
        request: &mut ClientRequestSlot,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<CleanupOutcome>>;
    fn poll_cancel_subscription_start(
        &mut self,
        cx: &mut Context<'_>,
        subscription: &mut ClientSubscriptionSlot,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<CleanupOutcome>>;
    fn poll_stop_subscription(
        &mut self,
        cx: &mut Context<'_>,
        subscription: &mut ClientSubscriptionSlot,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<CleanupOutcome>>;
}

pub trait PollServerBinding {
    fn prepare(&mut self, input: PrepareInput<'_>) -> CoreResult<PreparedRouteId>;
    fn poll_ready(
        &mut self,
        cx: &mut Context<'_>,
        route: PreparedRouteId,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<RouteReadinessStatus>>;
    fn activate(&mut self, route: PreparedRouteId) -> CoreResult<ActiveRouteId>;
    fn poll_abort_prepared(
        &mut self,
        cx: &mut Context<'_>,
        route: PreparedRouteId,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<CleanupOutcome>>;
    fn commit(&mut self, route: ActiveRouteId) -> CoreResult<()>;
    fn poll_accept(
        &mut self,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<Option<InboundRequest>>>;
    fn start_response(
        &mut self,
        response: InboundResponse,
        slot: &mut ServerResponseSlot,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<ResponseDelivery>>;
    fn poll_response(
        &mut self,
        cx: &mut Context<'_>,
        response: &mut ServerResponseSlot,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<ResponseDelivery>>;
    fn poll_cancel_response(
        &mut self,
        cx: &mut Context<'_>,
        response: &mut ServerResponseSlot,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<CleanupOutcome>>;
    fn start_emission(
        &mut self,
        emission: ProducerEmission,
        slot: &mut ServerEmissionSlot,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingPublication>>;
    fn poll_emission(
        &mut self,
        cx: &mut Context<'_>,
        emission: &mut ServerEmissionSlot,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingPublication>>;
    fn poll_cancel_emission(
        &mut self,
        cx: &mut Context<'_>,
        emission: &mut ServerEmissionSlot,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<CleanupOutcome>>;
    fn poll_shutdown(
        &mut self,
        cx: &mut Context<'_>,
        route: ActiveRouteId,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<CleanupOutcome>>;
}

pub trait StaticServient {
    fn step(&mut self, budget: &mut WorkBudget) -> CoreResult<StepStatus<RuntimeEvent>>;
}
```

The block above is a normative public API skeleton. `Context`, `Poll`, input
views, slot ids, and outcomes are `core`/`alloc` compatible. Implementations MAY
split preparation and request slots into more specific types, but the public
adapter MUST provide this lifecycle, typed budget, and terminal behavior.
`ClientRequestSlot`, `ClientSubscriptionSlot`, `ServerResponseSlot`, and
`ServerEmissionSlot` are caller-owned, bounded, generation-bearing operation
slots. A start method
initializes an empty reserved slot exactly once and either returns a synchronous
result or `Pending`; request/response progress polling and start cancellation
require a pending slot, while an active subscription is polled for samples or
stopped through its active slot. Start
failure leaves the slot empty. A synchronous request or response `Ready`,
terminal poll, or cancellation consumes that slot generation before it can be
reused. A successful subscription start instead transitions its slot to
`Active`; sample polling and stop retain that generation until terminal cleanup.
Subscription start becomes public only
after the guard-install
transaction defined by `STATE-SUB-001`; cancellation before that point closes
pending binding resources without publishing samples. No constrained start,
poll, or cancel operation requires a boxed future or binding-owned unbounded
request table. A pending emission slot retains the immutable payload lease and
generation-safe local-subscriber and binding-target cursors. A later step
resumes those cursors; it does not restart fan-out or binding publication at
target zero.
`ProcessEvent` is the common terminal-bearing poll value for subscriptions,
discovery processes, and Directory watches. A terminal event is retained by the
owning slot or process and emitted at most once; status accessors remain
available after emission. Polling after emission returns a documented
terminal-slot error and never restarts backend work; callers use the accessor to
read the retained status again. `Failed(CoreError)` is terminal and must not also
be returned as an outer error. The outer `CoreResult` is reserved for invalid
calls that do not change process state, such as a stale or mismatched slot.
`PreparedRouteId` remains owned by the caller until `activate` consumes it.
Readiness failure, timeout, expose cancellation, or failure of another binding
before activation is cleaned up through `poll_abort_prepared`. A terminal abort
consumes the prepared generation. `poll_shutdown` handles active routes,
including routes that activated but never committed. Neither cleanup operation
may require the caller to drop an unaddressable binding-owned prepared state.
`WorkBudget` has distinct counters for JSON/schema nodes, codec input bytes,
codec output bytes, URI
bytes, security branches, provider probes, queue operations, binding polls, and
cleanup items; decrementing an exhausted counter returns `LimitExceeded` before
the work begins. Codec byte counters are exact byte counts, not
implementation-defined blocks: consuming `n` input bytes and producing `m`
output bytes charges `n` and `m`, including bytes processed across incremental
calls. `PendingWork` reports only work already known from maintained ready
queues, cursors, or slot state; constructing it MUST NOT scan every runtime
table. It is nonempty whenever present and includes emission fan-out, binding
publication, response delivery, subscription cancellation, route readiness,
and route cleanup classes when maintained state reports them ready. `Idle`
means no work or event was observed within the supplied budget. `Progress`
means at least one transition occurred, one event was produced, or known work
remains. `Progress { pending: None, .. }` is valid when the call made progress
but no maintained queue, cursor, or slot reports remaining work; even
`Progress { value: None, pending: None }` is therefore distinct from `Idle`.
`Terminal` is used only when the driven facade itself has a terminal state; an
ordinary quiescent `StaticServient` returns `Idle`.
`MonotonicInstant` contains an opaque `ClockId` and an unsigned tick value.
Values are ordered or subtracted only when their clock ids match. The associated
`RuntimeClock::ticks_per_second()` is immutable for that clock id and defines
checked conversion between ticks and durations. A finite-width clock declares
its wrap period; the admitted maximum deadline and lease duration MUST be less
than half that period so modular comparisons are unambiguous. Core types never
depend on `std::time::Instant` or wall-clock time.

`API-SECURITY-001`: Security selection uses the following mandatory separation:

```rust
pub trait SecurityProvider {
    fn generation(&self) -> SecurityProviderGeneration;
    fn capability(&self, requirement: SecurityRequirementView<'_>)
        -> SecurityCapability;
    fn verify_probe(&self, input: VerificationProbe<'_>) -> CoreResult<ProbeResult>;
    fn verify_commit(&self, input: VerificationCommit<'_>) -> CoreResult<Principal>;
    fn apply_probe(&self, input: ApplicationProbe<'_>) -> CoreResult<ProbeResult>;
    fn apply_commit(&self, input: ApplicationCommit<'_>)
        -> CoreResult<AppliedSecurity>;
}

pub trait BodyAuthProjector {
    fn extract(
        &self,
        plan: BodySecurityPlanView<'_>,
        payload: &mut DecodedPayload,
        auth: &mut BodyAuthSlot,
        budget: &mut WorkBudget,
    ) -> CoreResult<ApplicationPayloadProjection>;
}

pub trait CredentialStore {
    fn probe(&self, key: CredentialKeyView<'_>) -> CoreResult<CredentialAvailability>;
    fn commit(&self, key: CredentialKeyView<'_>) -> CoreResult<CredentialLease>;
    fn generation(&self) -> CredentialGeneration;
}
```

Probe methods MUST be side-effect-free as defined by the Security section.
`CredentialLease` is secret-bearing, zeroizes provider-owned temporary material
where the representation permits it, is never `Debug`/`Display`, and cannot be
stored in immutable plans. Providers that cannot implement probe/commit remain
usable only through explicit branch selection.

`API-CODEC-001`: `PayloadCodec` exposes bounded whole-value helpers plus an
incremental decoder/encoder state API. Every call accepts `ResourceLimits` and
`WorkBudget`, reports consumed input and produced output bytes, and can return a
nonterminal need-input/need-output state. Validation operates on the one decoded
typed representation owned by the interaction. Application-view validation,
security-field injection, and wire-view validation share that representation or
a field overlay; they MUST NOT decode the same payload twice.

`API-RESOURCE-001`: `ResourceLimits`, `ResourceReservation`, and
`AdmissionLedger` are public protocol-neutral types. A reservation is move-only,
records owner, resource kind, count and bytes, and releases idempotently unless
committed into a published owner. `AdmissionLedger` has explicit source,
temporary, persistent-document, persistent-runtime, diagnostic, and cleanup
accounts plus `peak_live_bytes()` and `largest_contiguous_allocation()`.
Admission, directory, runtime, binding, codec, and security entry points accept
`&ResourceLimits` or a handle that identifies an immutable named profile; no
entry point interprets a missing policy as unbounded.

`API-DISCOVERY-EXEC-001`: Discovery always provides the owned value types and
poll traits in `no_std + alloc`. The `async` feature provides async extension
traits implemented as adapters over the same state machines. `std` provides
blocking adapters only for operations documented as safe to block. Concrete
client adapters implement at least one progress trait and MUST NOT implement
independent sync/async semantics. This resolves the “sync, async, or poll” choice in
`DIR-CONTRACT-001`; poll is the portable contract, async is the preferred host
contract, and blocking sync is only a convenience adapter.

`API-SOURCE-TIME-001`: Source retrieval and lease times use
`SourceTimestamp::{Monotonic { clock_id, ticks, ticks_per_second },
UnixMillis(i64), Unknown}`. `ticks_per_second` is nonzero and immutable for one
`clock_id`; conflicting scales for the same id are invalid source metadata.
`no_std` producers use `Monotonic` or `Unknown`; host resolvers MAY record
`UnixMillis`. Ordering across different `clock_id` values is undefined and
freshness policy MUST reject or explicitly accept incomparable timestamps.
Duration arithmetic uses checked integer scaling with a documented rounding
direction: expiry rounds toward earlier expiry and remaining-lifetime reporting
rounds toward zero.

### Common Value and Error Types

`API-TYPES-001`: Identifiers exposed outside one call (`ThingId`, `BindingId`,
`PlanId`, `SubscriptionId`, `CorrelationId`, `ActionInvocationRef`, arena slot
ids, revisions, and readiness keys) are opaque newtypes, not interchangeable
integers or strings. They implement `Clone`, `Eq`, `Hash`, and `Debug`; bounded
copyable representations also implement `Copy` and `Ord`. Debug and Display
output for identifiers and errors never includes payloads or credentials.
Generation-bearing ids compare and hash both index and generation. A `PlanId`
is stable for one immutable handle generation and is not a persistent identity
across replanning.

`API-HOT-ID-001`: Runtime lookup and per-call data use bounded slot/generation
ids for Things, affordances, bindings, handlers, subscriptions, and plans.
Human-readable names remain at API/admission boundaries and in immutable plan or
diagnostic tables; hot-path request, response, sample, and cleanup records MUST
NOT clone them. Formatting a name or detailed error is lazy and consumes only
the selected diagnostic budget.

`API-PAYLOAD-001`: `Payload` owns or shares an immutable byte sequence plus a
parsed media type and optional content coding. Moving a payload never copies its
body. Borrowed inspection is always available; typed decoding is explicit,
fallible, codec-selected, size-budgeted, and cached only when the cache is
charged to the owning budget. `InteractionInput` owns the application payload,
resolved URI variables, principal, deadline/cancellation view, and correlation
metadata. `InteractionOutput` owns one response payload, a normalized
`InteractionStatus`, and fixed-size `InteractionOutputMetadata` containing the
payload role plus optional action-invocation and validated binding-response
metadata. None of these types borrows a binding call stack.

`API-OPTIONS-001`: `InteractionOptions` is an owned, non-exhaustive options
value. It distinguishes omission from explicit selection for form index,
binding id, media type, subprotocol, security branch, deadline, cancellation,
URI variables, and validation profile. Merging is deterministic: per-call
values replace handle defaults by field; maps reject duplicate keys at their
input boundary; a per-call validation value may strengthen but not weaken the
effective policy. Unknown option extensions are preserved by Rust-native
builders or rejected explicitly; they are never silently ignored.

`ERR-TAXONOMY-001`: The public `CoreError` is a non-exhaustive structured enum
with the following top-level categories: `InvalidDocument`, `Validation`,
`LimitExceeded`, `NotFound`, `UnsupportedOperation`, `Selection`, `Security`,
`Binding`, `Payload`, `Backpressure`, `Cancelled`, `TimedOut`, `StaleHandle`,
`Lifecycle`, `Cleanup`, and `InternalInvariant`. Every variant carries a
bounded `ErrorContext` with applicable Thing, target, operation, form index,
plan, binding, correlation, phase, and redacted source cause. Selection errors
use a nested reason enum matching the candidate failures listed in Form
Selection; execution errors retain the chosen plan id. `InternalInvariant` is
for detected engine defects, not malformed external input. Error construction
itself has a bounded fallback representation and must not panic when diagnostic
allocation fails.

`CLEANUP-RECORD-001`: A queued cleanup item contains only generation-bearing
owner/plan references, cleanup operation, deadline/retry state, and a bounded
status code. Teardown forms, URI templates, security expressions, and diagnostic
names remain in their owning guard or immutable plan arena until cleanup reaches
a terminal state. Enqueueing cleanup MUST NOT clone a complete teardown plan or
payload. If retained plan ownership would exceed its cleanup budget, the
initiating admission or lifecycle operation fails before ownership can be lost.

`ERR-RETRY-001`: Errors expose `RetryClass::{Never, Safe, CallerDecision}` and
an optional retry-after hint. Validation, limits, stale handles, unsupported
operations, and authentication/authorization failures default to `Never`.
Read-only operations may be `Safe` only when the binding establishes that no
side effect was committed. Writes, action invocation, publication, and teardown
default to `CallerDecision` unless an idempotency key or protocol acknowledgement
proves a safe retry. The engine never retries automatically merely because an
error is transient.

The exact WP-100 Rust schemas for error categories and payloads, retry context,
correlation identity, cleanup operations, and cleanup records are frozen by
`docs/amendments/WP-100-error-cleanup-v1.md`. That normative amendment resolves
representation choices omitted from the prose above; an implementation must
satisfy both documents. Where the amendment is more specific, it supersedes the
category and representation sentences in `ERR-TAXONOMY-001`,
`ERR-RETRY-001`, and `CLEANUP-RECORD-001`.

The successful-interaction boundary, shared default error disposition,
handler-absence mapping, and removal of legacy Servient Boolean error
predicates are frozen by
`docs/amendments/WP-100-error-disposition-v1.md`. The shared disposition is a
binding-adapter default and does not introduce HTTP types into protocol-neutral
core.

The exact binding-response metadata method surface, final inbound response
envelope, and response-validation work-package ownership are frozen by
`docs/amendments/WP-100-interaction-output-api-v1.md`. That amendment preserves
the logical schemas above while preventing a temporary route-free public
response envelope from crossing the WP-100/WP-300 boundary.

### Admission Transaction and Complexity

`ADMIT-TXN-001`: Parsing, validation, effective-view construction, plan
construction, and registry publication use a reserve-build-publish transaction.
The implementation first charges deterministic work and temporary bytes, then
reserves persistent counts and bytes, builds only private state, and finally
publishes with one registry transition. Every failure releases reservations
idempotently. Temporary and persistent accounting are separate so a rollback
cannot double-release. Admission cancellation is checked at bounded work-unit
intervals and before publication.

`ADMIT-MEM-001`: Admission accounts input/source bytes, temporary working bytes,
persistent document-retention bytes, and persistent compiled-runtime bytes as
separate ledgers. It records or can measure peak simultaneously live bytes and
largest contiguous allocation. Completion releases phase-local parser,
validator, contributor, and candidate storage at the earliest safe boundary;
atomic publication MUST NOT be implemented by retaining every phase's complete
representation until commit.

Implementations SHOULD use arena checkpoints, reservation ledgers, immutable
base plus overlays, or generation-scoped bulk release so rollback metadata does
not duplicate the resources it protects. An admission profile defines a peak
multiplier or an absolute temporary-byte ceiling in addition to final persistent
ceilings. Exceeding temporary capacity returns `LimitExceeded` without changing
published state.

`PERF-COMPLEXITY-001`: Complexity includes bytes as well as node counts. Let
`d` be input document bytes, `n` parsed value nodes, `x` total retained extension
bytes, `f` forms, `t` URI-template source plus expanded-output bytes, `s` schema
nodes and reference edges, `q` security-expression nodes, `p` indexed matching
and explicit wildcard probes, `c` retained form-binding candidates, `v` typed
payload value nodes, `y` payload input plus output bytes, and `r` subscribers.
Protocol-neutral parsing is O(`d`) time. Local document validation and logical
plan construction are O(`d + n + x + f + t + s + q + p`) time, excluding bytes
of explicitly resolved external resources, which are charged to the same byte
and request budgets when admitted. Capability indexing makes `p` independent of
unmatched registrations in normal profiles. The admitted wildcard worst case is
O(`f * b`) probes for `b` registrations but is terminated by the probe budget.
Persistent compiled storage is O(`n_runtime + x_runtime + f + t + s + q + c`),
where only runtime-retained nodes and extension bytes are counted; a logical
plan, schema, string, or extension value is never copied per candidate.

One interaction performs O(`c_call + q_call + t_output + y + v`) bounded work,
where the per-call candidate and security terms never exceed their admitted
budgets and codec/schema passes are fixed by the selected validation profile.
Fan-out performs O(`r + y`) work and O(`r + y`) additional descriptor/shared-
payload storage; it is not O(`r * y`) in payload bytes. Each input/output byte,
node, probe, candidate, subscriber descriptor, and reference edge consumes the
corresponding typed work unit. Recursive document, schema, security, JSON-LD,
and TM algorithms MUST use checked depth or an explicit stack bounded by
`ResourceLimits`; untrusted input must not cause native stack overflow. A
complexity claim that counts nodes but omits variable-length strings, encoded
bytes, URI output, or extension bytes is nonconforming.

`PERF-INDEX-001`: Published registries and target/operation indexes use hashed
lookup with a documented denial-of-service-resistant hasher on host profiles,
or ordered/bounded lookup on constrained profiles. No public complexity claim
assumes attacker-controlled hashing remains O(1). Candidate vectors preserve TD
order and selection scans at most the pre-admitted candidate count; per-call
work is therefore bounded by the plan's candidate budget.

### Normative Lifecycle State Machines

`STATE-EXPOSE-001`: An exposed handle has states `Draft`, `Preparing`,
`ReadyPendingActivation`, `Activating`, `Committing`, `Serving`, `Cancelling`,
`Draining`, `CleanupPending`, `Cancelled`, `Destroyed`, and `Failed`. A
successful expose follows the preparation states in order and linearizes at
`Committing -> Serving`. Failure before serving enters `CleanupPending` when
cleanup remains, otherwise `Failed`; neither is dispatchable. Destroy
linearizes at `Serving -> Draining`, or performs private cleanup from `Draft`.

Cancellation of the host expose future, `destroy()` during exposure, and handle
drop may request cancellation from `Preparing`, `ReadyPendingActivation`,
`Activating`, or `Committing`. The cancellation request and
`Committing -> Serving` publication use the same lifecycle linearization
boundary. If cancellation wins, the handle enters `Cancelling` and the Thing
never becomes dispatchable. If publication wins, the request follows
`Serving -> Draining`.

`Cancelling` cancels every outstanding readiness token, aborts every prepared
route, and shuts down every active or committed route. It reaches `Cancelled`
after complete cleanup or enters `CleanupPending` while ownership is retained or
transferred. `CleanupPending` records its intended terminal state so
cancellation, exposure failure, and destruction cannot be confused. Dropping an
expose future or handle MUST NOT orphan a readiness token, route guard, endpoint
reservation, or cleanup result. Host drop transfers them to the reserved
Servient cleanup owner without blocking; constrained drop leaves a
generation-bearing runtime work item. A cancelled exposure is not retryable on
the same handle.

`destroy()` on `Cancelling`, `Draining`, `CleanupPending`, `Cancelled`, or
`Destroyed` joins or reports the same cleanup outcome. `expose()` outside
`Draft` returns a lifecycle error and does not start a second transaction.
Cleanup retry is allowed only for items whose `CleanupOutcome` says retryable,
and never republishes the Thing.

`STATE-SUB-001`: A subscription has states `Starting`, `CancellingStart`,
`Active`, `Stopping`, `CleanupPending`, `Closed`, and `Failed`. The
guard-registry insertion linearizes `Starting -> Active`; failure before it
closes the pending wire guard and never returns a public active subscription.
Cancellation or drop while `Starting` linearizes against guard installation. If
cancellation wins, the slot enters `CancellingStart`, no public `Subscription`
is created, pending samples remain invisible, and start cancellation closes the
pending wire resource. If installation wins, the same request observes an
active subscription and proceeds through `Active -> Stopping`.

Stop linearizes at `Active -> Stopping`; exactly one owner removes and closes
the wire guard. Samples are admitted only in `Active`. `CleanupPending` retains
whether successful cleanup leads to `Closed` or whether an already observed
failure leads to `Failed`. Repeated cancellation, stop, and drop operations join
the same retained outcome. Drop cannot manufacture missing teardown input. If
protocol teardown requires caller input that was not captured and is
unavailable, local sample admission still closes exactly once and the retained
result is `ResidualExternalState` with a bounded teardown-input-required cause.
Terminal state and lost-sample count remain observable while either the handle
or subscription view exists.

`STATE-DISC-001`: A discovery process has states `Created`, `Running`,
`Stopping`, `Completed`, `Cancelled`, `TimedOut`, `Overflowed`, and `Failed`.
The first poll or explicit start linearizes `Created -> Running`. Stop/drop
linearizes to `Stopping`, prevents new backend work, and owns cancellation of
the session. Exactly one terminal status is emitted and retained even when the
item buffer is full. `Completed` is used only for normal source exhaustion.
Terminal states are not retryable; callers create a new process. The configured
cancel policy alone decides whether pre-cancel buffered items drain.

`STATE-BIND-001`: Each binding route follows `Absent -> Prepared -> Ready ->
Active -> Committed -> Serving -> Draining -> Closed`, with `CleanupPending`
reachable from any state after `Prepared`. The Servient owns state transitions;
the binding guard owns protocol resources. Prepare, activate, commit, shutdown,
readiness cancel, prepared-route abort, and cleanup retry are idempotent for the
same Thing/binding generation. `CleanupPending` retains the guard or records its
atomic transfer to a named cleanup owner; guard drop is not a state transition.
A late callback includes its generation and is
discarded as stale after closure. A route cannot return from `Draining` or
`Closed` to `Serving`.

`STATE-INFLIGHT-001`: Dispatch reserves an in-flight slot only after confirming
`Serving`, then rechecks the registry generation while publishing the slot. The
destroy transition and slot admission share the registry synchronization
boundary, so each request is unambiguously rejected or counted as in flight.
Completion releases the slot exactly once. After drain expiry, the response gate
closes before binding shutdown; late handler results are discarded and reported
without retaining the registry entry indefinitely.

### Default Host Runtime Policy

`HOST-DEFAULT-001`: The named `GatewayDefaultV1` profile is the gateway limits
table in Resource Limits plus the queue defaults in Capacity and Overflow. It
also sets a 30-second interaction deadline, 10-second expose-readiness deadline,
5-second destroy drain deadline, 5-second subscription teardown deadline, and
30-second discovery timeout. Deadlines are monotonic durations. A caller may
disable a convenience deadline explicitly, but work-unit, count, and byte limits
remain mandatory. The default aggregation policy is sequential, fail-fast, and
TD/caller order as specified above. Automatic interaction retries are disabled.
Its runtime `SourceRetention` default is `MetadataOnly`; Directory publication,
explicit tooling/document handles, and callers requesting lossless source
access select `RawDocument` or `LosslessDocument` separately. It enables bounded
per-binding operational status and latest critical status, while historical
diagnostic journals are opt-in.

`HOST-DEFAULT-002`: The named `DirectoryClientDefaultV1` profile uses the
engine-side Directory client limits above, a 30-second query timeout, 10-second
publisher operation timeout, and 5-second session/watch cancellation deadline.
Result and watch overflow follow the non-lossy and compacting policies already
specified. It defines no Directory service default. Host defaults are versioned
names; changing a numeric value requires a new profile name or a documented
breaking configuration change.

`TIME-001`: Timeout races use the operation's linearization point. If terminal
success is published before timeout cancellation, success wins; otherwise the
caller receives `TimedOut` and any later success is treated as a late result.
Durations use a monotonic clock supplied by the host runtime. Constrained
profiles receive caller clock/tick values and explicit step budgets; no core
crate reads a wall clock.

## Validation and Verification

Expected checks before considering a design-affecting change complete:

```sh
tools/check-design-requirements.sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets
cargo check -p clinkz-wot-td --no-default-features
cargo check -p clinkz-wot-core --no-default-features
cargo check -p clinkz-wot-discovery --no-default-features
cargo check -p clinkz-wot-protocol-bindings --no-default-features
cargo check -p clinkz-wot-protocol-bindings-zenoh --no-default-features
cargo check -p clinkz-wot-servient --no-default-features
cargo check -p clinkz-wot-codec-cbor --no-default-features
cargo check -p clinkz-wot --no-default-features
```

The no-default-feature checks must verify that the documented constrained
surface remains available. Passing by compiling an effectively empty crate root
does not satisfy the `no_std + alloc` support contract.

Feature-specific checks should cover:

- `async` without `std` where intended.
- `zenoh` runtime tests behind explicit opt-in.
- `zenoh-pico` constrained planning/build surface.
- `td2-preview` as additive and isolated from TD 1.1 defaults.
- TD/TM round-trip fixtures with unknown extension preservation.
- TM-specific fixtures for unresolved placeholders, `tm:optional`, model
  composition/import metadata, and TD derivation with required instance values.
- Clinkz extension fixtures that preserve incoming `@context` values and emit
  the canonical `cz` namespace mapping when builders add Clinkz terms,
  including deterministic `cz1`/`cz2` prefix allocation and strict
  context-conflict errors. Adversarial consecutive prefixes verify one-pass
  indexing and linear byte/entry work rather than repeated context scans.
- Multiple forms per affordance, including relative `href` plus Thing `base`.
- Security-provided URI variables, including name-conflict rejection and
  expansion after credential application.
- TD source-trust metadata, deterministic JSON-LD context resolution, remote
  context resolver policy, and `cz` prefix conflict handling.
- Source-envelope APIs for discovered/admitted TDs, bare-TD
  Scripting-compatible discovery/fetch surfaces, plain `consume(td)` default
  source metadata, `produce(init)` binding-independent Partial TD expansion,
  `expose()` binding-dependent generated form finalization from the captured
  server-binding registration snapshot, and Rust-native document-consuming entry
  points that preserve validation and source evidence. Tests must keep bare-TD
  aliases such as `fetch_td(url)` distinct from document APIs such as
  `fetch_td_document(url)`. Envelope tests must cover the stable
  `ThingDocument<T, S>` accessor, mutation, `from_parts`, `into_parts`, and
  `map_thing` contract, plus `TdSourceInfo` validation/admission evidence.
- Handle TD introspection fixtures proving `thing_description()` returns the
  effective runtime TD view while extension APIs preserve the original
  round-trip document and source metadata.
- Thing-level forms for bulk and meta operations, including strict `formIndex`
  selection scoped to the containing form array.
- Payload/schema validation profiles covering outbound inputs, inbound handler
  inputs, primary responses, additional responses, read/write defaults, and
  opaque payload pass-through policy. Additional-response tests must cover
  `success: true`, `success: false`, and omitted `success` defaulting to error.
  Subscription sample tests must validate observable-property and event samples
  against affordance data/subscription schemas rather than unrelated form
  response metadata. Consumer response tests must prove TD-compatible extra
  object members or array entries are preserved rather than rejected only because
  they are extra.
- Body-location security tests must cover outbound application-payload validation
  before credential injection, wire-payload validation after provider commit,
  inbound `AuthMaterial` extraction before handler dispatch, and default removal
  or redaction of provider-managed body fields from handler-visible payloads.
- Payload validation policy selection across Servient defaults, handle-level
  overrides, per-call stricter options, and constrained-runtime documented
  defaults.
- Diagnostic validation-bypass tests proving the bypass is unavailable through
  Scripting-compatible APIs, disabled by default, observable when used, and
  unable to bypass operation support, security, scopes, routing URI variables,
  or binding preflight safety checks.
- Protocol bindings separately from protocol-neutral core logic.
- Object-safe host binding registration through `ServerBindingRegistration` and
  `ClientBindingRegistration`, including default wrapping of
  `Arc<dyn ServerBinding>` and `Arc<dyn ClientBinding>`, driving mode metadata,
  optional route-readiness drivers, runtime-event sink configuration, and
  overflow policy.
- Producer form-contributor tests covering deterministic generated forms,
  contributor budgets, security/context merge conflicts, application-supplied
  form probing, ambiguous owners, explicit owner selection, endpoint collision
  aliases, frozen-TD behavior after readiness, and reservation rollback.
- Server binding preparation tests proving bindings receive `BindingThingView`
  plus precompiled inbound plans and do not need access to the full TD tree for
  defaulting, target resolution, security inheritance, or form selection.
- Route-readiness driver tests covering async host `begin`/`poll_ready`/`cancel`,
  constrained nonterminal progress, timeout, expose cancellation, readiness
  failure rollback, and the rule that lifecycle calls do not block on external
  progress.
- Constrained binding construction that uses stable binding slots and
  poll/manual adapters without requiring boxed futures, host task spawning, or
  `Arc`, including slot lifetime, cancellation, teardown, and terminal progress
  states. Compile and transition tests cover synchronous and pending request and
  subscription starts, start failure leaving a slot empty, terminal generation
  consumption, active subscription sample/terminal polling, cancellation races,
  bounded pending response delivery, response-slot backpressure, and
  `StepStatus` values that report an event and pending work together without
  scanning all runtime tables.
- Binding I/O ownership tests covering route generations, duplicate correlation
  ids, stale callbacks, exactly-once response opportunities, response
  classification, and samples received before subscription installation.

Semantic verification must include:

- TD 1.1 field coverage and default resolution fixtures.
- Security inheritance fixtures: Thing-level, form-level override, `nosec`,
  scopes, security URI variables, missing definitions, unsupported schemes,
  multiple named schemes as AND, deterministic `oneOf` branch selection, and
  combo schemes.
- Security provider and credential-store probe/commit tests proving `oneOf`
  branch probing has no credential, request-metadata, token-refresh, or
  audit-success side effects; per-call branch choices do not mutate immutable
  plans; static admission profiles may cache a branch only while their
  credential invariant holds; and a post-selection retrieval race fails closed
  without committing or silently falling back from the failed branch.
- Security tests for body-location credentials must prove that application-facing
  payload APIs do not require callers to provide provider-managed secret fields
  and do not expose those fields to handlers or diagnostics by default.
- Inbound route-match fixtures proving form-level security and scopes are
  selected by compiled plan id or form index.
- Scripting API mapping tests for Consumer, Producer, and Discovery surfaces,
  including Producer `ExposedThingInit` expansion and Discovery bare-TD results
  distinct from Rust-native source-envelope results.
- Action lifecycle tests for `invoke_action` status metadata,
  `ActionInvocationRef` propagation, `query_action`, `cancel_action`, and
  missing-action-reference errors.
- Error taxonomy tests proving candidate selection failures are distinguishable
  from binding execution, response validation, teardown, cancellation, and
  backpressure failures.
- TD-level bulk operation tests and explicit fan-out helper tests.
- Rust extension `write_all_properties` tests for TD `writeallproperties`
  root-form selection, root-scoped `formIndex`, unsupported-operation errors,
  and separation from Scripting-compatible method names.
- Thing-level operation tests for `writeallproperties`,
  `observeallproperties`, `unobserveallproperties`, `queryallactions`,
  `subscribeallevents`, and `unsubscribeallevents`.
- Producer-side Thing-level handler tests, including Scripting-compatible
  per-property aggregation for `readmultipleproperties`, `readallproperties`,
  and `writemultipleproperties`; sequential and bounded-concurrent aggregation
  policies; deterministic TD-order and caller-order defaults; fail-fast,
  structured partial-result, deadline, and cancellation behavior; explicit
  Thing-level bulk handler precedence; and
  unsupported-operation errors when no matching handler or explicit aggregation
  policy exists.
- Handler slot replacement tests proving replacement affects only later
  dispatches, in-flight dispatch keeps the selected handler, clearing a handler
  yields structured `UnsupportedOperation`, and async dispatch owns the selected
  handler before awaiting.
- Subscription lifecycle tests: observe, subscribe, `Subscription::stop`,
  Scripting-compatible duplicate-subscription rejection, explicit
  `unobserveproperty`, `unobserveallproperties`, `unsubscribeevent`, and
  `unsubscribeallevents` teardown forms, collection-level subscriptions,
  optional handle-level teardown helpers, drop cleanup, teardown-not-run status
  when caller input is required but unavailable, teardown-auth-failed status when
  teardown credentials cannot be reacquired, proof that subscription guards do
  not retain secret credential material solely for later drop cleanup,
  backpressure, error delivery, and durable lost-sample counters when the
  status/error channel is unavailable or full.
- Bulk property tests for complete success, Scripting-compatible partial failure
  as a structured operation error, fail-fast extension helpers, and separately
  named structured partial-result extension helpers.
- Lifecycle tests for expose rollback after prepare, route-readiness, activate,
  or commit failure, activation gate release ordering, binding runtime error
  reporting, runtime event sink overflow policy, response backpressure reporting,
  critical lifecycle event journal preservation, critical-event compaction or
  shutdown-on-overflow behavior, and destroy quiescing with bounded drain budgets
  and discarded-response status.
- Handle-drop and Producer-emission tests covering draft drop, serving drop with
  transferred cleanup ownership, full cleanup queues, per-affordance emission
  ordering, validation, partial binding publication, and subscriber loss
  accounting.
- Reliability tests for idempotent cleanup, cancellation, shared capacity
  defaults, overflow observability, bounded buffering, and failure context
  preservation without secret disclosure.
- Discovery admission, freshness, revision ordering, watch compaction,
  lossless `ThingFilter.fragment` request serialization, refusal to emulate an
  unsupported filter with a broader local query, returned-view projection and
  secret redaction, unsupported-filter fail-closed behavior, discovery-process
  overflow terminal status when producers cannot be backpressured, explicit
  lossy-profile drop-newest reporting, and discovered TD trust policy tests.
- Directory scope tests proving Servient construction does not create or depend
  on an in-process Directory and that discovery exposes no service or storage
  SPI.
- Scripted remote Directory contract tests for exact-revision request encoding,
  typed lease authority and successful rotation adoption, token redaction,
  stable page-order validation, refusal to reuse tokens across query or
  authorization contexts, snapshot and weak-snapshot response handling,
  empty-page rejection, watch resume, compaction, reported lease expiry, and
  policy-generation terminal handling. Server-side compare-and-set execution,
  token issuance/validation, hidden-field authorization, lease expiry execution,
  and redaction policy are excluded until the Directory service design.
- Directory poll-client tests covering empty-slot start, synchronous completion,
  pending query/watch/publication/resolution, page and change delivery,
  cancellation races, explicit terminal watch events before and after the first
  item, terminal generation consumption, response count/byte
  limits, scripted remote errors, and proof that async adapters share the poll
  state machine.
- Runtime representation tests proving admission can release a generic JSON
  tree and lossless source, effective introspection does not require a second TD
  tree, source-retention modes account bytes separately, and owned effective TD
  materialization is explicit and non-caching.
- Plan/storage performance tests covering capability-index pruning, wildcard
  probe limits, bounded concurrent lazy compilation, compact per-call records,
  sparse handler slots, shared subscription slabs, and cleanup records that do
  not clone plans.
- Host contention tests proving unrelated Things and bindings make progress
  independently, plus allocation comparisons between erased boxed-future and
  allocation-sensitive binding paths.
- Validation/security reuse tests proving one codec decode per interaction,
  compiled validator reuse, provider capability caching, and generation-based
  invalidation.
- Constrained scheduling tests covering byte/node work-unit limits, round-robin
  fairness, reserved response/cleanup progress, and a perpetually ready binding.
- Performance smoke checks for allocation-sensitive hot paths.

### Requirement Traceability Matrix

The matrix is the design-time verification contract. The implementation plan
adds evidence links without redefining requirements. `All` means constrained,
gateway, and Directory-client where that capability exists; a profile may mark
a row not applicable only with a documented reason. “Model” means state-transition or
ownership review with exhaustive transition tests; “inspection” includes API,
feature, and documentation checks.

The machine-readable ownership and evidence index is
`docs/requirements.csv`. It records compilation cells, execution models,
resource profiles, capability roles, actual or target Cargo owner packages,
evidence kinds, evidence key, and source path as independent columns. Its
`requirement` expressions use an inclusive `..` range only for identifiers with
the same prefix and a `|` separator for an explicit set. CI MUST validate every
axis and package token, expand those expressions, reject an unknown or duplicate
stable requirement, reject a normative id missing from the CSV, and reject a
CSV id missing from this document. Each `evidence_key` becomes a stable test,
compile fixture, model, inspection, or benchmark result key; implementation
completion requires at least one current evidence record for every applicable
expanded row.

| Requirement id or family | Profiles | Required verification |
| --- | --- | --- |
| `DOC-GOV-001` | All | Inspection: normative-id and broken-reference checker |
| `ARTIFACT-AUTH-001`, `IMPL-CONFORM-001`, `CHANGE-CONTROL-001` | All | Artifact-conflict, work-package, migration, waiver, and revision-control inspection |
| `API-OWNERSHIP-001`, `REFACTOR-GATE-001` | All | Ownership uniqueness, dependency-direction, gate-evidence, and refactor-ready inspection |
| `STD-BASELINE-001` | All | Pinned-publication and adopted-errata inspection |
| `PROFILE-AXIS-001` | All | Feature/execution/resource/capability matrix inspection and compile fixtures |
| `FEATURE-MATRIX-001` | All | Compile every required crate/feature/public-surface cell |
| `CRATE-DEPS-001` | All | Cargo metadata dependency and feature-direction inspection |
| Data/TD/TM and validation family | All | Round-trip, field/default, extension, TM, URI, schema, source-envelope fixtures |
| `DOC-RUNTIME-001` through `DOC-RUNTIME-003` | All | Retention-mode, no-resident-DOM, logical-view, materialization, and peak-byte tests |
| `JSONLD-PREFIX-001` | All | Deterministic conflict fixtures plus adversarial prefix-count and context-byte scaling |
| Feature and `no_std` family | Constrained | Feature-matrix compile checks and public-surface compile fixture |
| `CONCUR-LOCK-001`, `CONCUR-USER-001`, `CONCUR-CRIT-001` | All | Lock-order model review, reentrant callback tests, constrained critical-section instrumentation |
| `CONCUR-LIN-001` | All | Race tests at every documented linearization point |
| `HANDLER-CANCEL-001`, `HANDLER-CANCEL-002` | All | Late sync result, cooperative async cancellation, bounded-step tests |
| `HANDLER-STORAGE-001` | Gateway, constrained | Sparse/dense profile footprint and unused-operation storage tests |
| `PLAN-COST-001` through `PLAN-COST-003` | All | Structural memory assertion, adversarial limit tests, plan benchmarks |
| `PLAN-INDEX-001`, `PLAN-LAZY-001`, `PLAN-REQUEST-001` | All | Capability pruning, wildcard limit, lazy race/cache, and per-call size tests |
| `PLAN-CACHE-001` | All | Single-flight compile, negative-cache classification, generation invalidation, bounded reclamation, and no-global-lock tests |
| `PLAN-BOUND-001` | All | One-over candidate admission, bounded fallback/probe accounting, and 1/8/32 candidate scaling tests |
| `TD-MEM-001`, `TD-MEM-002` | All | Ownership inspection and peak-live-byte measurements |
| Form selection and binding-plan family | All | Ordered multi-form, strict selection, relative URI, and error-category fixtures |
| `LIFE-EXPOSE-001` through `LIFE-EXPOSE-003` | Gateway, constrained | Failure injection at every binding phase and compensating-cleanup outcomes |
| Protocol binding family | Gateway, constrained | Object-safety compile test, poll/async contract tests, readiness and stale-generation tests |
| `FORM-FINALIZE-001`, `FORM-FINALIZE-002`, `FORM-OWNER-001`, `FORM-COVERAGE-001` | Gateway, constrained | Contributor determinism, owner/collision, freeze, limit, and rollback tests |
| `BIND-IO-001`, `BIND-OUT-001`, `BIND-PROGRESS-001` | Gateway, constrained | Ownership, generation, correlation, bounded subscription/response/emission progress, validation, cancellation, and start-install race tests |
| `HANDLER-API-001`, `HANDLER-SUB-001` | Gateway, constrained | Handler type/ownership compile tests and subscribe rollback/exactly-once teardown tests |
| Subscription and bulk families | Gateway, constrained | State/race, exactly-once teardown, overflow, ordering, partial-result, and root-form tests |
| `SUB-STORAGE-001`, `SUB-DATA-001` | Gateway, constrained | Shared-pool quota, empty-subscription footprint, direct-slot, and overflow tests |
| Discovery family | Directory-client, gateway | Lazy start, terminal-state, cancellation, freshness, privacy, paging, watch, and overflow tests |
| `DIR-SCOPE-001` | Directory-client, gateway | Dependency and public-surface inspection proving no service/storage implementation is in scope |
| `DIR-CONTRACT-001`, `DIR-AUTH-001`, `DIR-SNAPSHOT-001`, `DIR-WATCH-001`, `API-DIRECTORY-POLL-001` | Directory-client, gateway, constrained | Scripted-client owned-value, slot lifecycle, exact-revision/authority encoding, token-context reuse prevention, returned-view redaction, page/watch metadata validation, cancellation, and adapter-equivalence tests |
| `DIR-STREAM-001` | Directory-client, gateway, constrained | Fragmented-input, one-over-limit, bounded-resume, partial-page rollback, and peak-residency tests |
| Security family | All | Inheritance, combo expression, probe/commit side-effect, scope, redaction, and race tests |
| `SEC-PERF-001`, `VALIDATE-COMPILE-001`, `VALIDATE-REUSE-001` | All | Generation cache, compiled validator, single-decode, and injection-view reuse tests |
| `CONSTRAINED-STORAGE-001`, `CONSTRAINED-STORAGE-002` | Constrained | Arena generation/wrap prevention, reservation rollback, exhaustion tests |
| `CONSTRAINED-PROGRESS-001`, `CONSTRAINED-OWN-001` | Constrained | Bounded-step/cleanup tests and dependency/type inspection for atomics/`Arc` |
| `CONSTRAINED-WORK-001`, `CONSTRAINED-SCHED-001` | Constrained | Typed work-unit bounds, fairness, starvation, and reserved-progress tests |
| `RES-LIMIT-001` through `RES-LIMIT-004`, `RES-PROFILE-001` | All | Boundary and one-over-limit tests for every field; exhaustive named-profile snapshots; atomic rollback accounting |
| `PERF-ALLOC-001` | Gateway, constrained | Allocation-count harness for every listed path |
| `PERF-BENCH-001` through `PERF-BENCH-003`, `PERF-SCALE-001` | All | Versioned manifests, reproducible reports, deterministic work counters, one-axis scaling, baselines, and regression gate |
| `PERF-BUDGET-001`, `PERF-ADMISSION-001`, `PERF-PEAK-001`, `PERF-CALL-001`, `PERF-FANOUT-001` | All applicable profiles | Absolute budget, profile-ledger and peak-memory, call-path, and fan-out structural benchmarks |
| `PERF-ACCOUNT-001`, `PERF-FANOUT-002` | All applicable profiles | Accounting-contention, bounded aggregation, fan-out cursor, cancellation-progress, and stale-generation tests |
| `CAP-OVERFLOW-001`, `CAP-STATUS-001` | Gateway, directory-client | Full-queue recursion, byte/count accounting, compaction, and shutdown-progress tests |
| `OBS-PROFILE-001` | All | Minimal/latest-status profile and exhausted-diagnostic-budget tests |
| `API-TYPES-001`, `API-PAYLOAD-001`, `API-OPTIONS-001` | All | Public API compile tests, move/copy allocation tests, merge property tests |
| `API-SURFACE-001`, `API-SECURITY-001`, `API-CODEC-001`, `API-RESOURCE-001` | All | Frozen surface compile tests and ownership/side-effect tests |
| `API-DISCOVERY-EXEC-001`, `API-SOURCE-TIME-001` | All applicable profiles | Execution-adapter compile tests and timestamp comparability tests |
| `API-HOT-ID-001`, `CLEANUP-RECORD-001` | Gateway, constrained | Hot-record size/allocation and cleanup-plan non-duplication tests |
| `ERR-TAXONOMY-001`, `ERR-RETRY-001` | All | Exhaustive producer-to-category mapping and redaction/retry policy tests |
| `ADMIT-TXN-001`, `ADMIT-MEM-001` | All | Failure injection, reservation leaks, phase release, peak live bytes, and contiguous-allocation tests |
| `PERF-COMPLEXITY-001`, `PERF-INDEX-001` | All | Scaling benchmarks, adversarial hashing/depth tests, structural inspection |
| `STATE-EXPOSE-001`, `STATE-SUB-001`, `STATE-DISC-001`, `STATE-BIND-001`, `STATE-INFLIGHT-001` | Applicable profiles | Model review plus exhaustive legal/illegal transition and race tests |
| `HANDLE-DROP-001`, `PRODUCER-EMIT-001` | Gateway, constrained | Cleanup ownership/exhaustion and emission validation/order/partial-outcome tests |
| `HOST-SHARD-001`, `HOST-SHARD-002`, `HOST-ASYNC-001` | Gateway, directory-client | Independent-progress contention tests and erased/native allocation benchmarks |
| `HOST-DEFAULT-001`, `HOST-DEFAULT-002`, `TIME-001` | Gateway, directory-client | Snapshot tests of every default and deterministic timeout-race tests |
| Reliability family | All | Panic-freedom fuzzing, idempotent cleanup, cancellation, failure-context tests |

Every family row covers normative statements in its named section that do not
carry a more specific id. Evidence must name the exact paragraph or API tested;
a single broad integration test is not sufficient evidence for an entire row.

## Design Exit Criteria

The design is frozen for implementation only when all of the following hold:

1. Every uppercase normative requirement has a stable id or is mapped to a
   named requirement-family row in the traceability table.
2. Every stable requirement has applicable profiles and a verification method.
3. Public normative APIs are distinguished from illustrative code and no
   unresolved type or ownership choice changes observable semantics.
4. Host and constrained `ResourceLimits` profiles define numeric defaults or
   require explicit caller values; no externally influenced collection is
   implicitly unbounded.
5. State machines for expose/destroy, subscription teardown, discovery
   cancellation, and binding cleanup define linearization points, owners,
   retryability, and terminal outcomes.
6. Concurrency documentation defines lock ordering, reentrancy policy, and the
   rule that no engine lock spans user code.
7. Constrained design includes a reference storage layout covering arenas or
   tables, slot generations, capacity exhaustion, manual progress, and cleanup
   without atomic reference counting.
8. Benchmark manifests define reproducible workloads, metrics, and approved
   regression tolerances. Numeric baselines are required before implementation
   completion, not before design freeze.
9. The active design has no unresolved open question classified as blocking.
10. Lossless source retention, compiled runtime state, temporary admission
    storage, and owned effective-TD materialization have separate ownership and
    byte accounting; no profile requires a resident generic JSON DOM or two
    complete TD trees.
11. Host concurrency tests demonstrate independent progress for unrelated
    Things and bindings, and constrained tests demonstrate bounded typed work
    units plus fair manual scheduling.
12. Queue count limits are paired with physical descriptor/payload pool limits;
    logical per-owner capacity does not imply maximum-size preallocation.
13. The current Directory scope contains only engine-side interaction values and
    progress adapters; Servient has no service/storage dependency, and service
    topology, backends, query execution, snapshot storage, and service SLOs are
    explicitly deferred.
14. The conformance baseline uses dated standards, active artifacts have an
    explicit authority order, and every implementation work package is scoped by
    requirement ids with migration and evidence obligations.
15. Complexity and performance gates cover both structural counts and byte-heavy
    inputs, and every gating workload has an absolute budget or deterministic
    invariant rather than report-only metrics.
16. Cache cold starts, generation invalidation, resource accounting, Directory
    page admission, and subscription fan-out have bounded progress and
    contention gates; none relies on a global scan or per-operation global
    mutex.

## Open Questions and Accepted Risks

There are no silently open design choices. A newly discovered question is added
here with an owner, blocking or non-blocking classification, affected
requirement ids, and resolution condition before implementation relies on an
assumption.

Accepted risks at this design revision are:

- synchronous user code can run past cancellation or deadline; the engine
  isolates and discards late results but cannot preempt safe Rust code
  (`HANDLER-CANCEL-001`);
- bindings classified as `CompensatingPrepare` can leave temporary or residual
  external state after failed expose; strict deployments reject that class,
  while other deployments surface structured cleanup outcomes
  (`LIFE-EXPOSE-002`, `LIFE-EXPOSE-003`);
- network-bound and erased async calls may allocate within benchmarked budgets;
  only the precisely scoped paths in `PERF-ALLOC-001` promise a zero-allocation
  configuration;
- the Scripting API target is a W3C Group Note that its publisher describes as
  unstable; the dated semantic baseline prevents silent drift, and later
  changes require explicit compatibility review (`STD-BASELINE-001`).

## Deprecated Documents

All previous project documents are archived under `docs/deprecated/`. They are
not active design sources. Use them only to recover historical rationale,
implementation sequencing, or audit context.
