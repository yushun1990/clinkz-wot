# Composable capabilities and runtime boundaries

Status: DISCUSSING

## Engineering question and authority

Should ClinkZ-WoT evolve toward composable Producer/Consumer capabilities,
TD-independent execution contracts, optional dynamic TD admission, and distinct
Host/embedded storage and execution representations? Which changes earn their
migration cost through actual applications, rather than a cleaner diagram?

This is an unresolved investigation. The candidate below preserves the design
exploration following [PR #134](https://github.com/yushun1990/clinkz-wot/pull/134).
That PR owns the independent product/runtime review, including its detailed
dependency, resource-layout and legacy-call-path audit; this topic does not
duplicate or accept that review. The evidence baseline for both investigations
is [`6e804d2`](https://github.com/yushun1990/clinkz-wot/tree/6e804d2d46ffc6a1a59b316f7e6e729cf0277512).
The additional reproducible experiments live in
[product-runtime-boundaries](../tools/architecture-fixtures/product-runtime-boundaries/README.md).

The active v5.1 owners remain authoritative under
[Architecture Governance](../ARCHITECTURE_GOVERNANCE.md#architecture-authority-model).
No candidate crate, API, generated image, new platform commitment, work-package
admission or gate acceptance is established here.

## Applications before platform labels

| Application | Minimum useful capabilities | Dynamic admission and complete Servient? | Application/platform responsibilities |
|---|---|---|---|
| TD editor, validator, generator or catalog | TD models, serialization, Basic/schema/default/security/URI meaning; Planning only for executable configurations | Neither runtime nor mandatory Planning. TD already stands alone. | Editing, persistence, catalogs and build orchestration |
| Producer-only service | Advertised operations, handlers, one server protocol, security/payload enforcement, activation and cleanup | Dynamic admission optional for fixed configurations; Consumer and Discovery unnecessary | Business logic, deployment, credential storage |
| Consumer-only client | Selected requests, credentials, result validation, deadlines, cancellation and cleanup | Dynamic preparation for unknown TDs; fixed peers may be prepared earlier. No necessary server. | Application retry and provisioning policy |
| Industrial gateway | Selected roles/protocols, isolation, shared resource governance and scheduling | Dynamic Host runtime often useful for changing topology, not implied by Linux alone | Fleet management, persistence, hard real-time and safety control |
| Intermediary | Upstream Consumer and downstream Producer with separate trust/lifecycle contexts | Fixed mappings may be prepared; two handles can share one runtime/governor | Transformation, caching, reconciliation and cross-system transaction policy |
| Embedded firmware | Selected operations, buffers, bounded state and protocol/platform progress | Fixed firmware may need neither on-device admission nor a WoT executor | HAL, RTOS/executor, networking integration and control loops |

Subscriptions, collection operations and Discovery are additional capabilities,
not automatic consequences of a role. A complete Servient could remain a
convenient composition sharing protocol sessions and resource governance without
being every application's minimum construction unit.

Native protocol firmware with a correctly generated or externally hosted TD is
a real alternative: it forgoes engine-provided orchestration guarantees but may
already satisfy the deployment. The WoT alternative-implementation rationale
is referenced in PR #134. An embedded executor must offer useful lifecycle,
Consumer or resource behavior to justify its costs.

## Evidence motivating the question, and qualifications

- [Core](../core/Cargo.toml) combines target execution values with TD-bearing
  legacy things/security, and depends on TD. Codecs and bindings inherit that
  surface. [Servient](../servient/Cargo.toml) and the
  [umbrella](../clinkz-wot/Cargo.toml) include Discovery. Cargo dependencies
  demonstrate API/build coupling, not necessarily retained TD RAM or linked
  parser code.
- [The public legacy Consumer](../servient/src/servient.rs) constructs
  `ConsumedThing`; [handle calls](../servient/src/handle.rs) and
  [legacy request execution](../core/src/thing.rs) still inspect Forms. The
  target compiler and frozen selector are a distinct execution path. Repeated
  calls to shared TD helpers are duplicated work, not necessarily divergent
  semantic algorithms. Target-to-legacy retirement already belongs to the
  roadmap; it must not wait until a new facade conceals both paths.
- [Target registration](../core/src/binding.rs) currently requires
  `B: PollServerBinding` in `StaticBindingRegistrationInput`; Host validation
  requires `supports_producer_property_read()` even with a client component.
  Legacy client-only construction is less restrictive. This is a concrete
  impediment to target Consumer-only products, but a first-slice implementation
  restriction is not proof that accepted product authority requires it.
- [The legacy builder](../servient/src/builder.rs) installs
  `LocalDiscoverer(InMemoryDirectory)` by default and constructs an event broker.
  Removing implicit Directory service is already an accepted direction, not a
  new consequence of this proposal.
- [The frozen selector](../planning/src/property_read.rs) consumes already
  prepared plans but lives in TD-dependent Planning and takes `PlanBuildOutput`.
  The fixture isolates its body over slice observations without a second
  selection algorithm.
- Current Core imports `alloc::sync::Arc` unconditionally and fails the
  Cortex-M0 comparison. Extracted predicates/value dependencies compile there.
  This establishes a limited dependency/representation opportunity, not a
  Cortex-M0 support requirement or impossibility of WoT on other MCUs.
- ADR-0001 and ADR-0009 already permit distinct Host/constrained representations.
  ADR-0021 already removes TD source retention from Published state. The premise
  that one complete physical runtime or resident TD is mandated is too strong.
  Build-time preparation may save admission work/peak memory more than steady
  state, and has not yet demonstrated whole-firmware savings.

## Credible alternatives

| Candidate | Dependency/construction model | Strongest case, cost and disconfirming evidence |
|---|---|---|
| Retain accepted boundaries and finish migration | TD tooling uses TD; applications use Servient; static builder accepts a Thing. Public `consume(Thing)` remains legacy until replaced. | Least disruption; already separates semantic owners, immutable plans and typed/erased execution. Prefer it if realistic products fit and extraction adds more adapters than practical benefits. Current TD/Arc coupling remains. |
| Improve features and facades only | Role-specific builder methods, optional Discovery, selected Producer/Consumer features in existing Servient/Core | Useful product improvement with small migration; hiding methods alone does not isolate TD or change payload/atomic requirements. Can be an intermediate or sufficient outcome. |
| Capability runtime with a leaf Contracts package and optional admission | Explicit role handles, Host/embedded modules in one Servient package; TD/Planning only for dynamic admission or build tools | Prior design's preferred candidate. Reuses rules and lifecycle code while separating physical storage. Adds a leaf package, view adapters, compatibility period and admission/image boundary work. Must demonstrate usefulness beyond the fixture. |
| Separate Host and embedded execution packages | `clinkz-wot-host::Consumer` / `clinkz-wot-embedded::Producer`, sharing Contracts and preparation libraries | Credible if actual backends diverge substantially; adds release, packaging and integration boundaries before necessity is demonstrated. Module separation first preserves the option. |
| Build-time preparation for all products | Planning is a build dependency; execution accepts only prepared images | Useful for fixed firmware or fixed Host services; inadequate for previously unknown TDs/changing topology. Candidate mode, not universal design. |
| Independent Host/MCU semantics and planners | Separate product libraries interpret TDs and select operations independently | Rejected in the prior exploration: optimizing representations does not justify duplicated defaults/security/URI/Planning algorithms and parallel fixes. |
| Native protocol implementation plus generated TD | TD tooling/build dependencies; no embedded engine runtime | Potentially simplest correct fixed Producer. Compare against the engine using the same application; fewer engine guarantees may be acceptable. |

The preferred candidate is provisional. Separate Producer/Consumer crates,
a universal allocator/queue/scheduler generic framework, a binary plan format,
and an immediate Host/MCU rewrite have no demonstrated justification.

## Candidate package and API design

These names/features are design sketches, not existing exports or commitments.

| Package | Candidate responsibility and dependency boundary |
|---|---|
| Foundation | Existing time/generation/resource/work primitives; no WoT vocabulary or runtime policy |
| **Contracts, new leaf** | Operation vocabulary; qualified identities/errors; plan/artifact observations; binding contracts and shared execution predicates with their linear authority. Foundation dependency; portable base without TD, allocation or atomics |
| TD | One authoritative Basic/default/security/URI program and trusted source adapters. Foundation + Contracts, never legacy Core aggregate |
| Planning | One preparation/selection/compiler-barrier program over trusted TD inputs; TD + Contracts + Foundation. Optional build-tool emission |
| Servient | Publication, runtime owners/governor, registrations, role handles, scheduling and cleanup; Contracts/Foundation base. Optional `dynamic-td` adds TD/Planning |
| Bindings and codecs | Contracts plus selected SDK/codec dependencies. Compiler modules implement a Contracts SPI; compiler-only use must not select a transport SDK or own Planning rules |
| Discovery | Explicit client capability; TD-bearing results allowed, no implicit Directory service or mandatory Servient |
| Existing Core / umbrella | Transitional reexports for unchanged moved types; intentional optional composition. No second runtime behavior or target dependency on legacy orchestration |

A leaf extraction solves a specific dependency cycle: moving `Operation` into
existing Core and making TD depend on Core conflicts with Core's current TD
dependency. Slimming Core after legacy retirement is a competing way to solve
it; the extra package earns its place only if independent consumers and gradual
migration require it.

The proposed Contracts scope includes executable result/identity/lifecycle
enforcement, not just traits. It excludes concrete registries, queues,
schedulers, application orchestration and TD interpretation. The response
predicate and immutable selector have narrow executable evidence. Moving the
complete binding SPI, private seals, cancellation/late-result handling and
portable slots is **not** proved by that extraction. These must migrate
together where authority cannot safely be separated.

Candidate application constructions:

```rust,ignore
// TD tooling: depend on clinkz-wot-td; add Planning only if needed.
let td: Thing = serde_json::from_slice(input)?;
td.validate()?; // ordinary tooling ingestion, not bounded external ingestion

// Host Consumer: Servient features std + consumer + dynamic-td + async;
// selected binding client/backend. A Producer selects producer + server instead.
let runtime = host::Runtime::builder()
    .client(zenoh::client(config)?)
    .profile(GatewayDefaultV1)
    .build()?;
runtime.run(async |api| {
    let sensor = api.consumer().consume(&td).await?;
    sensor.read_property("temperature", InteractionOptions::default()).await
}).await?;

// Gateway/Intermediary: one builder with client(upstream).server(downstream).
// Application code supplies mapping and independent security contexts.
// Discovery is explicit: resolve TD, then pass it to the Consumer capability.
```

`consume(&td)` would borrow only through preparation and return a handle owning
source-independent execution facts. Supervised `run` would own progress/drain;
an advanced API could lend a unique manual driver. Producer-only construction
would not allocate a consumed registry or Consumer request pool. `std` should
not imply Tokio; the current graph already selects Tokio through concrete
Zenoh. Features should add capabilities/types, not mutate a globally named
payload layout under Cargo feature unification.

```toml
# Candidate fixed-firmware composition; these features do not exist today.
[dependencies]
clinkz-wot-servient = { version = "NEXT", default-features = false, features = ["producer"] }
clinkz-wot-protocol-bindings-zenoh = { version = "NEXT", default-features = false, features = ["pico"] }
[build-dependencies]
clinkz-wot-planning = { version = "NEXT", features = ["emit-rust"] }
clinkz-wot-protocol-bindings-zenoh = { version = "NEXT", default-features = false, features = ["compiler"] }
```

```rust,ignore
let mut node = embedded::Producer::bind(
    &generated::IMAGE, pico_server, &mut storage, handlers, &DEVICE_PROFILE,
)?;
node.begin_expose()?;
loop {
    platform.progress();
    node.step(&mut cx, &mut step_budget);
}
```

Fixed Host services could also install prepared facts; a capable embedded
deployment could choose dynamic admission. Platform, role and preparation mode
are independent dimensions. Narrow affordance preparation might reduce output,
but must preserve complete Basic validation and would need successor scope;
the active Consumer aggregate's complete lookup/preflight is unchanged.

## Sharing and generated-image boundary

Preserve one authoritative implementation of TD semantics, Planning rules and
shared execution correctness predicates. Host Vec/Box/Arc versus embedded
static slices/slot leases need not share a physical representation. Share
selection, identity/result checks, reusable lifecycle decisions and resource
accounting meaning; platform adapters may differ in scheduling, storage,
synchronization and protocol I/O. Each still needs fairness, isolation,
boundedness and cleanup evidence. A view observes facts; it cannot mint
admission, publication or validated-result authority.

If generated images are pursued, the initial candidate is Cargo-linked Rust
data for an explicitly supported operation/artifact subset:

1. Run the same TD semantic and Planning programs; emission changes storage,
   not selection. Binding compilers must provide concrete supported image data;
   `Box<dyn Any>` and memory dumps are not image formats.
2. Include logical facts and compatibility/configuration identities. Exclude
   credentials, SDK handles, pointers, live generations and lifecycle state.
3. Bind fresh runtime generations, check compatibility/configuration, and admit
   target storage/resources before activation. Host layouts cannot admit MCU
   slots. Trusted linked output needs explicit structural checks; it cannot
   silently inherit `ValidatedThing` authority. Downloaded/untrusted plan blobs
   are outside this candidate.
4. Retain runtime authorization, result/payload validation, cancellation,
   cleanup and route activation. TD schema-definition validation is not proof
   of application-payload validation; broad payload schemas are outside the
   current Consumer response slice. Future evaluators must remain singular.

Embedded storage loans must survive pending protocol work and late cleanup;
dropping a ticket cannot release its slot early. Borrowed payloads cannot point
into a binding callback's stack. Reexports can preserve unchanged moved types,
not substitute for a new lifetime/ownership surface.

## Affected owners and conflicts to resolve if selected

| Owner | Precise effect or qualification |
|---|---|
| [ADR-0001](../docs/ADRs/0001-crate-and-module-boundaries.org), [module boundaries](../docs/architecture/20-module-boundaries.md), `CRATE-DEPS-001` and API ownership | Contracts extraction would change the accepted Core/package owner and dependency graph. This is an actual authority change to assess; it is not implemented by the probe. |
| [ADR-0009](../docs/ADRs/0009-protocol-binding-integration-and-deployment.org), [binding SPI](../docs/spec/binding-spi.md) | Different representations already allowed. Consumer-only complete registration would need exact compiler/client pairing, applicability and sealed-output rules without dummy Producer state. Current code restriction is not a newly discovered normative mandate. |
| [ADR-0021](../docs/ADRs/0021-borrowed-consumer-td-admission.org), [Planning](../docs/spec/planning.md), [runtime lifecycle](../docs/architecture/50-servient-runtime-lifecycle.md) | Keep admitted borrowed-input/owned-output transaction. Generated-image installation is a new trust/admission path, not the same proof with validation skipped. |
| [Interaction Core](../docs/spec/interaction-core.md), [runtime safety](../docs/spec/runtime-safety.md) | Preserve private registration-minted single-use seals, normal/late-success validation, no raw installed-client bypass and exactly-once settlement. No application-supplied identity tuple becomes authority. |
| [Foundation](../docs/spec/foundation.md), resource schema and target characterization | One schema/accounting meaning; profile projection is not a whole-device fit guarantee. Distinct physical costs require measured admission/progress evidence. |

No accepted owner was found to require every application to be a complete
Servient or every MCU to dynamically admit TDs. The candidate's broader API and
image claims go beyond demonstrated behavior and require deliberate authority
migration if selected. Existing typed Host carriers, generation checks,
activation permits and terminal ownership are reusable guarantees, not reasons
to freeze every mechanism forever.

## Reevaluation timing and possible migration

Continue the admitted Consumer Property Read critical path. The useful
reevaluation point is after the first real Host **target** Consumer integration,
before broad public API and legacy-path expansion increases reversal cost.
Check repeated reads and cancellation/failure cleanup through frozen plans;
legacy success and the real-target Producer probe do not answer that question.

Escalate earlier if implementation exposes a concrete conflict with role-only
construction, accepted ownership, resource/lifecycle assumptions or other
product boundaries. Topic [0077](0077-consumer-compiler-admission-boundary.md)
applies that rule to compiler admission now: shared Planning semantics and
representation-specific support must be distinguished before aggregate
admission. This does not select the broader Contracts/image proposal here or
make a crate/runtime split its prerequisite. Use the existing
[decision behavior](../PROJECT_GOVERNANCE.md#technical-decision-behavior),
[implementation escalation](../PROJECT_GOVERNANCE.md#risk-proportional-implementation-admission)
and [architecture change control](../ARCHITECTURE_GOVERNANCE.md#architecture-change-control).
This is no additional mandatory gate, recurring ceremony or next-action record.

If the candidate earns adoption, proportionate stages would be: establish
genuine Consumer-only construction and explicit optional services; extract only
the justified contracts with compatibility reexports; compare one fixed
firmware application using dynamic/manual execution, prepared execution and
native protocol plus generated TD; retire legacy paths capability by capability
only when replacements exist. Keep ObserveProperty and broader lifecycle
evidence separate. No generator or crate rewrite belongs on the admitted
Consumer transaction's critical path.

`PLAN.md` needs no amendment for this investigation: its existing critical path
already puts real Host Consumer evidence before broad expansion. No milestone,
package dependency, admission or gate status has changed. `AGENTS.md` and
Architecture Governance already supply escalation and migration mechanisms;
Project Governance only gains index navigation for session reconstruction.

## Evidence still needed to decide

The [fixture documentation](../tools/architecture-fixtures/product-runtime-boundaries/README.md)
owns commands, measured results and recovery limitations. Predicate parity and
target compilation do not establish complete runtime feasibility or savings.
The next evaluation needs real Host construction/call-path evidence and, for
embedded claims, the same realistic application on hardware with a real
protocol backend: RAM/flash/stack, simultaneous application/engine/protocol
buffers, fragmentation, latency/progress, cancellation/reconnect/cleanup,
global accounting, fairness and shutdown. Start from a deployment or the
existing STM32F407 reference; Cortex-M0 compilation is a discriminator, not a
new product commitment.

Real Pico C ownership/blocking behavior, artifact emission/binding contracts,
a materially contrasting protocol and the need for unknown TDs after
commissioning remain unknown. Neither async adaptation nor code generation
preempts an uncooperative handler or bounds a blocking C call. If the existing
runtime fits the deployment and extraction/image complexity outweighs measured
benefits, retaining it is the preferable outcome.
