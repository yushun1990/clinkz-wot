# Reusable Plan semantics versus Thing-generation ownership

Status: DISCUSSING

ADR-0024 migrates the preparation/execution, definition/instance and native
infrastructure direction into active authority. The first source extraction is
scoped to dependency/compiler ownership; production reuse, funding and shared
retirement remain unproved. Earlier sequencing below is history where it
conflicts with that decision. It does not block the dependency extraction.

The completed [integrated Review 08](../docs/reviews/review-08-integrated-architecture-reassessment.md#programs-instance-context-and-live-authority)
reassesses reuse with current compilation, role and infrastructure boundaries.
It recommends separating definition content, checked instance context and live
authority now, while deferring automatic caching/persistence until benefit and
ownership are proved. Its review-first sequence challenges the older narrow-fix
sequencing below without retrospectively changing accepted implementation or
activating a shared-program contract. The topic remains DISCUSSING.

## Question and counterexample

Does the current one-Plan-Set-per-produced/consumed-handle-generation contract
silently turn a sound **lifecycle owner** into a mandatory **physical-copy and
compilation unit**? A Plan describes executable interaction semantics; a Thing
identifies a logical physical or virtual affordance-bearing object. They can
coincide for a small deployment without being intrinsically the same identity.

Consider 10 million otherwise homogeneous electricity meters reporting through
Zenoh. An application may expose fleet-level virtual Things, individual meter
Things, or both, depending on discovery, permissions and interaction semantics.
The engine must not prescribe a physical-device-to-Thing cardinality, nor infer
one transport session or one independent complete compiled image per meter.
Conversely, using one fleet Thing must not erase per-meter identity, provenance,
authorization, routing or independently observable WoT meaning.

The scalability question is not whether a correct single-Thing Runtime can
allocate and release its Plan Set. It is whether its public/retained identity
structure makes identical immutable execution work inherently unreusable when
many compatible logical instances are needed.

## Current evidence and boundary

- [Planning's active `PLAN-COST-001` and `PLAN-SET-001`](../docs/spec/planning.md)
  compile one logical plan per effective form/operation, sharing it among
  binding candidates **within the build**, while assigning each published
  handle generation one Servient-owned Plan Set lifecycle record. Neither
  clause demonstrates or forbids separate cross-Thing physical storage reuse.
- The [compiled-plan lifecycle](../docs/architecture/30-compiled-plan-lifecycle.md)
  correctly scopes publication, pin/operation leases, draining and reclamation
  to an owning generation. Plan identities and artifact references carry
  generation/target/binding context. Moving invariant bytes into a shared object
  must not accidentally share these distinct authorities.
- The accepted [WP-200 Consumer aggregate evidence](../docs/evidence/WP-200-consumer-property-read-aggregate.toml)
  measured 4,550 retained Host output bytes (4,238 static) for **four properties
  and three coordinates**, in one unpublished fixture. At 100,000 separately
  materialized identical-size Host outputs, that conditional multiplication is
  455 MB decimal, *before* Servient records, registrations, transport, active
  calls or allocator overhead. This is not a deployed per-device measurement,
  a generic Plan size, or proof that a fleet needs 100,000 Things.
- The [Binding SPI](../docs/spec/binding-spi.md) already assigns native session
  multiplexing, protocol-side routing and correlation to a binding. Thing
  count, transport connections, active operations and physical device count
  are different dimensions.
- The [WP-400 Consumer Runtime evidence](../docs/evidence/WP-400-consumer-property-read-runtime.toml)
  deliberately proves **one consumed generation and one active call**.
  Its scope cannot establish or rule out multi-Thing program sharing or fleet
  deployment costs.

## Architectural risk

A design can satisfy every per-Thing resource cap and exact cleanup invariant
yet impose O(N) repeated immutable program bytes for N Thing instances,
where only O(N) minimal instance metadata is intrinsically necessary.
Encoding Thing/generation identity, authority, or concrete endpoint ownership
directly into every reusable byte may make that duplication hard to remove later.

The missing potential boundary is between:

1. **Reusable execution definition**: operation/form semantics and immutable
   compiled portions that are demonstrably identical under a checked
   compiler/schema/security/binding/configuration dependency identity.
2. **Execution binding/context**: logical Thing identity, resolved per-instance
   target and variable values, authorization/credentials, routing, and
   configuration or source-dependent details that cannot safely be shared.
3. **Runtime ownership**: plan-set publication, exact generation, leases, calls,
   capacity reservations, cancellation, cleanup and retirement.

These are investigation categories, **not** selected Rust types, API changes, or
a finding that any particular existing field is misplaced.

## Reassessment against the owned-configuration finding

Reconstruction baseline: master
`2fa5a856b9c41590f169cbc33ef744bfc8ee8951`. This reassessment challenges both
the existing architecture and the sharing hypothesis using production source,
the registered strict evidence, ADR-0008/0021/0023 and
[topic 0086](0086-consumer-owned-artifact-configuration.md). A **compiled
program** below means potentially reusable immutable execution data, not Rust
machine code, a current public type or a transport session. Persistence would
be an optional representation of that data, evaluated separately below.

**Sequencing decision: prepare the narrow 0086 contract/admission repair first;
do not require a foundational program-sharing correction before it.** The
current model permits a later deliberate separation of reusable data from
per-generation authority, but does not already provide safe cross-Thing reuse.
That separation needs its own accepted contract before implementation. Neither
this proposal nor 0086 authorizes production work under the withdrawn scope.

### Concrete coupling and its consequences

| Boundary | Production / authority evidence | Consequence for reuse and evolution |
| --- | --- | --- |
| Compiler input | [`LogicalInteractionPlan`](../core/src/plan.rs) owns `PlanId`, `ThingId`, property name, original form index, absolute resolved target, media and subprotocol. [`BindingCompilerInput`](../core/src/binding_compiler.rs) exposes that entire plan, candidate binding/generation/configuration/order and role. | The generic compiler can depend on instance identity as well as semantics. Determinism for the **same snapshot** does not prove invariance under a different Thing, plan id, endpoint or registration ordinal. No declared dependency projection or parameter-binding contract establishes such invariance. |
| Actual supported payload | `ResolvedTargetCompiler::step` copies the complete resolved target into `ResolvedTargetArtifact` (`[u8; 64]` and length). Its continuation captures `PlanId` and the complete candidate. | This is endpoint data, not a substantial generic instruction program. Different meter endpoints produce different payloads even with identical compiler configuration/compatibility. Identical targets can produce equal native payload values under different Thing/plan identities; that observation supplies no shared owner or permission to reuse an envelope. |
| Artifact identity | `BindingArtifactIdentity` contains plan-set generation, plan id, binding id/generation, configuration, compatibility and role; `BindingArtifactRef` adds a local slot. [`Planning materialization`](../planning/src/consumer_property_read.rs) creates dense plan ids using the admitted plan-set generation. | These identify an admitted use of a payload. They are unsuitable as a cross-Thing content key. Removing generations or substituting another registration would break the existing selection/request seals. Conversely, different envelopes could eventually reference equal immutable content without sharing execution authority. |
| Configuration and eligibility | `ResolvedTargetCompiler::configuration` encodes only capacity/version. Native `check`, Core's static downcast, the private Host adapter and `ConsumerCompilerSupport::configuration` enforce that same key. Planning's registration trait is sealed. | This is the concrete 0086 blocker. Adding an opaque artifact type or wrapping one in `Arc` cannot bypass closed support or establish callback/destructor eligibility. Configuration repair must reach the actual compiler, support descriptor and complete owner in both representations. |
| Physical output owner | Planning's `ConsumerPropertyReadOwnedOutput` uniquely owns the source-free manual root and is neither Clone nor Sync. [`Host GenerationOwner`](../servient/src/consumer_property_read/host.rs) owns that output and its original charge; the static runtime retains its typed build/output storage. | Sharing Host handles to one generation is not sharing programs across independent generations. The existing output includes instance strings, indexes, artifacts, accounting and reclaim state. Sharing the whole root would also share lifecycle state and exclusive cleanup authority. |
| Accounting and reclamation | [`PLAN-SET-001` and Planning ownership/publication](../docs/spec/planning.md) assign one aggregate owner per handle generation, retain artifacts inside it, transfer output once and reclaim after its leases end. `PlanFootprint` forbids double-counting shared immutable storage but supplies no cross-generation physical owner. | Current closed output is charged and reclaimed per generation. A future shared store needs a funded physical owner, bounded per-generation references and final release only after the last user. A departing generation cannot release another generation's backing. The no-double-counting clause alone does not implement this protocol. |

The compiler-input and physical-owner couplings are material future changes,
not proof that `PlanId` or one Plan Set per handle is itself wrong. The
[lifecycle authority](../docs/architecture/30-compiled-plan-lifecycle.md)
explicitly makes the live record lease, rather than globally unique numeric
ids, the lookup authority. Independent roots can have equal numeric ids.
A future store must preserve that namespace boundary; hashing bare ids cannot
create global program identity.

The normative logical plan also includes security/schema/template/extension
semantics beyond this small implemented projection. A future reuse key must
cover their actual dependencies rather than infer support from the narrow
NoSec Property Read evidence. Credentials remain runtime inputs, excluded from
compiler artifacts under the existing contract. Neither reuse nor a new key
may move credentials, route guards, calls or SDK cleanup into immutable data.

### Challenge to the proposed sharing model

Three different questions need separate answers:

1. **Can an execution registration consume this artifact?** The complete
   binding/configuration/compatibility/role and admitted generation checks
   answer that today. They must remain exact at execution.
2. **Would compilation produce equivalent reusable content?** Neither
   `BindingConfigurationDigest` nor `BindingArtifactCompatibility` answers
   this. They omit the selected target and other logical inputs. A future
   compiler must define a complete semantic/dependency identity or a checked
   parameterized representation. Equal type/model names or format markers
   are insufficient. Different binding policies may allow equal pure bytes,
   but sharing those bytes cannot make the registrations interchangeable.
3. **Who retains and pays for that content?** A shared immutable payload still
   requires a physical owner, reference admission and last-user destruction.
   `Arc` does not supply parent-account provenance, bounded destruction or a
   static/no-atomic representation. Mutable lazy slots, compile cancellation,
   negative results and publication remain generation-scoped unless separately
   redesigned; immutable content sharing does not require sharing them.

Whole-artifact deduplication and parameterized compilation are different
proposals. Full-input equivalence can share identical endpoint data where
proved; it saves nothing for the current payload when each meter has a
different target. Sharing an instruction/template body while binding each
meter's endpoint/security/source context requires a new compiler/runtime
operand boundary. Merely omitting those inputs from a cache key is unsound.
Likewise, deduplicating after independent compilation may save retained bytes
without saving validation, materialization or compilation work. Compilation
reuse needs evidence for its own dependency and admission rules.

The earlier 455 MB multiplication remains a conditional cost warning. It
does not identify 4,550 shareable bytes: the measured aggregate contains
instance-specific targets, names, identities, storage capacity and accounting.
No existing fleet measurement establishes the reusable fraction, a dominant
compiler cost or the instance/transport cardinality. A generic program layer
could cost more than this closed target-copy primitive saves.

### Explicit target model: hypothesis and counterexamples

The proposed target is **one compatible Protocol Binding executing multiple
Compiled Programs; one immutable program serving many compatible Thing
instances; an Instance Context supplying per-Thing identity, endpoint
parameters, authorization and routing; and a Plan Set retaining independent
lifecycle, generation, admission and cleanup authority**. Its purpose is to
reuse compilation work, especially at service cold start, rather than merely
deduplicate outputs after compiling every Thing. These names describe a
hypothesis, not accepted types or a required implementation.

| Proposed relation | Assessment against current evidence | What could falsify or limit it |
| --- | --- | --- |
| One Binding executes many programs | Coherent and structurally aligned with the SPI. `PollClientBinding::start_request` and `ClientBinding::invoke` receive the selected artifact per invocation; an associated compiler/artifact **type** does not imply a single program **value**. The accepted aggregate already compiles three coordinates through one registration. | Each program must fit the actual binding's supported format, role, configuration and operation envelope. A binding must not retain a global mutable "current program" that changes another call's interpretation. Existing single-call Runtime evidence proves neither concurrent multi-program scheduling nor multi-Thing deployment. |
| One immutable program serves many Things | Plausible for a checked execution definition whose remaining variability is expressed as typed operands. Reusing a definition does not require reusing Thing, plan or binding generations. | Same model name or schema shape does not establish equivalence. Endpoint scheme/template shape, protocol extensions, media/codec, schema or security-expression differences can require a distinct definition. If a compiler specializes on every concrete endpoint or source identity, there may be no useful shared program, or it needs an explicit parameter boundary first. |
| Instance Context supplies identity, endpoint parameters, authorization and routing | Coherent as the instance/runtime side of that boundary. Values and any runtime references must be bound and checked for this Thing and selected program before effects; necessary per-call state remains per-call. | Moving a value into a context does not prove the program is independent of it. URI encoding/template constraints, target restrictions, authority/tenant isolation and route mapping must still be enforced. Static security expressions may be shared only when semantically equivalent; current credentials, applicability and committed authorization remain runtime checks. Context identity alone cannot grant authorization, and this hypothesis does not put credentials/providers into `OutboundRequest` or compiler data. |
| Plan Sets retain independent authority | Retain this invariant in any future model. A Plan Set would admit its context, generation-qualified envelope and bounded program reference; a separate physical owner would fund shared immutable storage. Closing one generation ends its rights without closing another's. | A context or program reference that bypasses the live record lease, spends another generation's capacity, or frees the physical program before the last user contradicts the model. Session/SDK owners and their retirement remain Binding/runtime concerns. Producer route reservation and atomic activation would still need fresh instance-specific checks. |

Thus the model is a credible evolution hypothesis, **not falsified by the
present one-Plan-Set-per-generation rule**, and **not established by today's
implementation**. Compatibility must mean both a valid semantic/operand
contract and an eligible current execution registration, not equality of one
format marker. A reusable program may combine shared logical data with a
binding-specific lowering; this implies no universal artifact executable by
every binding. The current generation-qualified envelope can remain the
execution authority even if a future payload references a reusable definition.

### Cold-start objective: compilation by definition, binding by instance

Let `N` be the number of Thing instances and `K` the number of distinct
compatible execution definitions actually required by their operations,
binding/compiler dependencies and parameter contracts. `K` is not the number
of model names; one Thing can require several definitions, and specialization
can make `K` grow with `N`. The desired cost decomposition is:

```text
startup work = sum over K definitions of definition compilation
             + sum over N instances of context checking/binding/admission
             + supplied-input validation + reuse lookup/verification
```

This is a target decomposition, not a measured complexity claim. Count any
repeated logical/schema/template compilation as definition work too; moving it
out of the binding compiler must not hide that it still runs once per Thing.
If each instance supplies an independent TD, the current borrowed-input
contract still requires paid validation/semantic processing of that input.
Recognizing a shared definition has its own cost. A future trusted common
definition plus checked instance parameters might avoid repeated work, but
cannot obtain validated-TD authority by trusting a supplied type name or hash.
Identity, endpoint validation, authorization, capacity reservation and
publication remain necessary for every instance. Total startup is not O(`K`)
when it must at least inspect and admit `N` distinct contexts.

Post-compilation interning alone does not meet this compilation objective.
Neither lazy compilation that eventually repeats per instance nor many Things
sharing one transport session establishes it. A future solution must locate
reusable work **before** repeating definition compilation. If concurrent
materialization is supported, avoid duplicate definition work with bounded
waiting/refusal and cancellation that preserves other users. This does not
reactivate today's inactive lazy/cache requirements or require a cache service.
Explicit startup grouping, a supplied immutable definition, or a build-produced
definition are alternatives to evaluate alongside runtime lookup.

The falsifiable experiment is to hold `K` and definition sizes fixed while
increasing `N`, then vary `K` independently. Count logical and binding compiler
invocations/work separately from input processing, context instantiation,
admission and retained physical bytes. Parameter-only changes should reuse
the definition; changes to a real dependency must produce another definition
or reject before effects. Check identical behavior against independent builds,
including endpoint escaping and cross-tenant authorization negatives. Measure
both elapsed cold-start time and work: a smaller payload or equal final bytes
does not establish reduced compilation. If most startup cost is unavoidable
per-input work, definitions specialize per endpoint, or operand checks cost as
much as the removed compilation, the expected cold-start benefit is limited.
The current small target-copy primitive cannot establish or refute savings
for a future substantial schema/template/protocol program.

### Optional persistence across process restarts

Persisting a pure definition without runtime ownership is plausible, separately
from in-process sharing. A service could load and verify `K` stored programs,
then bind/admit `N` fresh contexts and Plan Sets. Disk reuse can survive the
last in-memory user; the disk artifact is not a surviving Plan Set lease or
proof that the new process owns funded memory. A program could support sharing
without supporting serialization, and persistence without sharing one loaded
copy. Neither feature is a prerequisite for the other.

Today's types provide no such contract: artifact references contain local
generations/slots, and Host artifacts use erased native storage. Snapshotting
a Plan Set, `Box<dyn Any>`, native pointers or an ownership root cannot create
a reusable file format. Topic 0076's
[generated-data candidate](0076-composable-capabilities-and-runtime-boundaries.md#sharing-and-generated-image-boundary)
already distinguishes pure image data from fresh installation authority; its
Cargo-linked subset does not admit loading arbitrary persisted files.

A future persistence proposal must settle these boundaries:

- **Stable data and dependencies.** Use an explicit versioned encoding of
  immutable definition/parameter contracts and a complete compiler/format/
  binding semantic dependency manifest, including applicable schema/codec/
  security interpretation and configuration projections. Runtime `BindingId`, Plan/Thing generations,
  registration order, addresses and parent-account identities cannot be its
  durable identity. A new process must match current registration semantics to
  those dependencies, then mint fresh envelopes; it never treats old numeric
  identity equality as compatibility. Target/ABI/features/endian dependence
  must be explicit if the encoding uses native representations.
- **No serialized authority.** Do not persist credentials or authorization
  grants, provider/SDK handles, active route/permit owners, sockets, calls, pins,
  reservations, leases, runtime counters, compiler continuations or cleanup
  obligations as program data. Bind current identity, endpoint parameters,
  authorization and routing afresh. Recompute actual installed footprint and admit current target
  capacity; a saved footprint or successful prior run grants no allocation,
  publication, route-activation or validated-result authority.
- **Trust and failure.** A content digest alone does not authenticate a
  compiler or restore semantic validation authority. Specify the trusted
  producer/provenance or required revalidation, bounded structural decoding,
  dependency/version checks and integrity handling. Corrupt, truncated or
  incompatible data must fail before effects, or trigger fresh compilation
  from an available trusted source within current admission. If neither usable
  cached data nor a trusted source exists, refuse rather than coerce an old
  artifact. An accepted trusted-program installation path might avoid loading
  the original TD; it cannot serialize today's `ValidatedThing` proof. Current
  credential and result checks remain active even for trusted program data.
- **Actual benefit.** Measure load/verification/decoding/relocation separately
  from compilation and context admission, across same-version restarts and
  dependency/configuration upgrades. If necessary verification reconstructs
  the expensive program, persistence may not improve cold start. The loader
  must preserve the same semantics as fresh compilation without inventing a
  second TD/security interpretation or preserving old runtime ownership.

This is a bounded architectural feasibility assessment, not selection of a
serialized IR, executable-code loader, persistence API or cache lifecycle.
The active startup-only registration and generation-isolation rules remain
binding until a future projection reconciles this new admission path.

### Alternatives and disposition

| Alternative | Disposition and reason |
| --- | --- |
| Preserve independent materialized Plan Sets, repair 0086 configuration | Selected near-term sequence. Keeps the accepted lifetime and source-free transaction; corrects a demonstrated inability to represent required binding operands. Does not claim fleet scalability or program reuse. |
| Share today's complete Plan Set/output or reuse artifact references across Things | Reject. It conflates source identities, generation seals, mutable record/reclaim authority and original capacity. It also fails to parameterize the endpoint payload. |
| Introduce shared immutable programs, separate instance bindings and generation records before 0086 | Credible future architecture, not a prerequisite now. It changes compiler inputs, output ownership, artifact resolution, resources and Host/static evidence together, without a measured shareable program or production binding to justify that cost. The required configuration distinction is independent of this split. |
| Factor definition compilation from instance binding, with reuse before compilation | Preferred hypothesis to test for cold-start savings. Define dependency/operand equivalence and keep fresh contexts, generation envelopes and exact registration seals. Compare explicit startup grouping/supplied definitions with bounded runtime lookup; interning after independent builds is only a memory optimization. |
| Use one virtual fleet Thing or binding session multiplexing | Valid application/protocol choices when they preserve externally required identity, authorization and source attribution. Neither solves separately meaningful Things' immutable-data duplication. Transport sharing and compiled-data sharing have separate owners. |
| Add lazy compilation/eviction or persisted/generated programs now | Defer implementation. Persistence could reduce repeated compilation across restarts, subject to the data/trust/reinstallation assessment above; it cannot restore runtime authority. Laziness alone does not establish equivalence or fix 0086. Inactive PLAN-LAZY/CACHE requirements are not reactivated. Topic 0076 owns the broader image/package investigation. |

### Constraints on the narrow repair

The repair should amend the accepted successor/API/admission projection for
the existing closed compiler, not introduce a shared-program runtime. Before
source work resumes it must settle:

- A versioned immutable binding-configuration projection covering the actual
  capacity/format, resource-contract variant, concrete binding-private policy
  identity, result bounds and supported cells. The private-policy input and
  its correspondence to the actual complete execution owner must be explicit.
  An arbitrary caller digest is not evidence of either correspondence or
  bounded callbacks. Preserve the strict constructor/key behavior.
- Exact forwarding through native and supported Host constructors, support
  capture, bounds/start/continuation checks and candidate/artifact envelopes.
  A descriptor must not reconstruct the old capacity-only identity for a newly
  configured compiler. Preserve complete-owner recovery and all-bounds-before-
  start. Choose the compatibility version from actual payload interpretation;
  a matching format marker never waives configuration checks.
- Separation from the complete **engine-policy** key. Do not include unrelated
  engine quotas, parent account/lease identities, Thing/plan generations or
  endpoint strings in binding configuration merely to obtain uniqueness.
  Endpoints remain compiler inputs; the checked engine policy still captures
  all of its own constructor operands. Strict/owned artifact substitution must
  fail even when target bytes and numerical limits are equal.
- Preserve the ability for one compatible registration to execute multiple
  definitions and for later instance binding to use a shared immutable
  definition. Do not make a program's reusable identity depend on runtime
  generations or a process nonce, or require recompilation on a restart merely
  because runtime ownership is fresh. This is an evolution constraint, not a
  requirement to add program identities, parameterization or persistence to
  the narrow repair. Its complete registration digest is still an admission
  identity, not a future durable content key.
- Proportional impact review and new layout/work/failure measurements for the
  changed compiler/support/Host adapter/cursor and affected aggregate/runtime
  carriers. Preserve source-free output, full engine charging of artifacts,
  monotonic rollback and original-account release order. Existing strict
  evidence cannot silently certify enlarged layouts or owned semantics.

These changes need no decision about shared-program identity or physical
storage. Extending a fixed configuration captured once per registration/support
descriptor has bounded impact. It does not require putting the private policy
or runtime owner into each artifact payload, nor change the compiler's target-
copy algorithm. Future program reuse can retain exact admission envelopes and
introduce a distinct content/dependency identity when justified. It must not
reinterpret this repaired binding digest as that identity.

The narrow configuration projection is now specified in the
[successor contract](../docs/spec/consumer-resource-ownership.md#closed-consumer-binding-configuration);
its [admission record](../docs/work-packages/WP-400-consumer-resource-ownership-admission.md#reconstructed-baseline-and-impact)
owns the explicit non-blocking reconciliation of this topic. It remains a
candidate pending independent review. This investigation stays DISCUSSING;
no shared-program, physical-storage or persistence contract is adopted.

### Sequence and falsification boundary

1. Prepare and independently review 0086's concrete contract/API/scope amendment,
   including configuration operands and encoding. This reassessment selects
   that work, not an exact Rust constructor or encoding. The current
   `candidate` admission/source stop remains until that review permits changes.
2. Implement and independently accept the scoped resource-ownership successor
   with its six existing evidence obligations and affected strict/Producer
   regressions. Continue the production HTTP frontier under its own admission.
3. Test the explicit target with real binding/program and multi-Thing evidence
   before broad scalability claims or a frozen shared representation. Vary
   `N` and `K` independently and measure definition compilation separately from
   instance/input/admission work and physical bytes. If persistence is proposed,
   compare fresh compile with load/verify plus fresh installation across
   restarts/upgrades. Host/static costs and irreducible instance costs remain.

Reopen this sequence **before** implementing the narrow repair if its concrete
API requires per-Thing compiler registration/policy duplication, equates a
binding configuration with globally reusable program content, puts execution
or cleanup owners into artifacts, weakens generation checks, or prevents a
payload from later referencing separately owned immutable data. No such
requirement follows from the inspected repair boundary. Existing exclusive
output ownership will require deliberate migration for sharing, but the
configuration repair need not deepen it.

Any later sharing proposal must falsifiably demonstrate equivalent content
under its declared dependency/parameter rules; distinct live Thing/generation/
registration envelopes; mismatch rejection before effects; one cancellation
or drain preserving another user's data and funding; and exactly one final
physical reclamation. It must reconcile Planning's artifact ownership,
PLAN-SET-001/PlanFootprint, Servient leases and Binding access in their existing
authoritative owners. Fresh independent acceptance is required for that change.

This topic remains a non-authoritative proposal, with the broader reuse design
unresolved. It changes no accepted architecture, work-package admission,
requirement/gate/milestone status or production code. Verification commands and
exact review revisions belong to the PR. Related
[topic 0076](0076-composable-capabilities-and-runtime-boundaries.md) owns the
broader capability/package/image proposals; 0086 owns the configuration blocker.
