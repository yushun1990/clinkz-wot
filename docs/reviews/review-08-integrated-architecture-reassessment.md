# Architecture Review 08: Preparation, Programs, Instances, and Protocol Infrastructure

- Date: 2026-10-20; review corrections 2026-10-21
- Status: reworked architecture candidate with constructibility evidence; pending independent architectural acceptance
- Reviewed source and authority: [`700cfdb1ccb03fabd8e27fdc27b2f2109a362d9a`](https://github.com/yushun1990/clinkz-wot/tree/700cfdb1ccb03fabd8e27fdc27b2f2109a362d9a), active v5.1 including the accepted Host HTTP Producer admission

This report preserves an independent reassessment from project goals, source,
accepted contracts and executable counterexamples. It is **non-normative**.
The proposed amendments linked below are also non-normative: they change no
requirement, API/state/resource schema, implementation admission, gate, package
or milestone status. Production implementation is outside this submission.
The original assessment is retained below. This revision answers an independent
critique of its dependency, reuse and shared-Producer boundaries. It selects a
concrete candidate and supplies small executable sketches; it is not independent
acceptance of its own corrections. Fresh architectural review and deliberate
authority/scope migration precede production implementation.

## Conclusion

Retain protocol-owned mapping and I/O, and complete the separation between
**preparation, immutable programs, instance authority and live protocol
infrastructure** before freezing the architecture. The recent responsibility
redesign improves the information boundary; it does not settle the ownership,
capability or environment boundaries. These concerns do not all belong in a
universal Binding SPI.

Keep shared WoT semantics, deterministic selection, source-free execution,
truthful advertisement, private dispatch authority, callbacks outside locks,
first-cause cancellation, retained late results, complete-object cleanup
transfer and original-account retention. Reconsider mandatory representations,
compiler eligibility, duplicated lifetimes and physical resource assumptions.

The first HTTP Producer entry/driver is a useful restricted design. Its real
implementation remains outstanding. Its `Copy` artifact, atomic compilation and
one-exposure/one-listener cardinalities do not establish a general architecture.

## Findings and evidence classification

| Finding | Evidence at the reviewed revision | Classification and implication |
| --- | --- | --- |
| F1: Producer semantic loss and incomplete coverage | [Planner](../../planning/src/property_read.rs) selects the first read-looking Form and treats absent `op` as readable. The [existing diagnostic](../../tools/architecture-fixtures/property-read-runner/tests/producer_admission.rs) reproduces thirteen Basic-valid mutations with absent semantic loans, equal plans, preparation/handler/response and complete mock cleanup. | Proven implementation defects, already acknowledged by the accepted Producer admission. Correct whole-description admission before native effects; this is not an HTTP authentication exploit or proof that the proposed driver works. |
| F2: Explicit HTTP method on a multi-operation Form | [HTTP lowering](../../protocol-bindings/protocols/http/src/compiler.rs) receives effective operations but does not apply their multiplicity to `htv:methodName`. The independent probe admits a Form with explicit `op: [readproperty, writeproperty]` and `htv:methodName: POST`. | Additional protocol-mapping defect. TD 1.1 prohibits this combination. Correct HTTP lowering, without an HTTP case in the Engine. The probe establishes compilation/admission, not a new wire-execution claim. |
| F3: General compiler erasure does not imply open Consumer admission | [Compiler contracts](../../core/src/binding_compiler.rs) support generic owned/resumable state, but `HostSemanticCompiler` requires an inline `Copy` artifact and one atomic callback. The [existing external compiler test](../../protocol-bindings/protocols/http/tests/semantic_compilation.rs) executes 512 funded steps and then proves exclusion from Consumer planning. | Proven extensibility restriction. Restricted inline authoring is useful, but cannot establish general Binding authorability. |
| F4: Extension security data is rejected upstream | [Security deserialization](../../td/src/components/security_scheme.rs) has a closed scheme discriminator. The independent probe adds an unused `ex:nonce` definition alongside selected NoSec and fails before Binding selection. | Proven extensibility barrier. TD must distinguish preserving an extension from knowing how to satisfy it. Preservation cannot grant authorization; selected unsupported security must still refuse. |
| F5: Execution and environment capabilities remain coupled | [Servient dependencies](../../servient/Cargo.toml) include TD, Planning, Discovery and the legacy compiler utility. Core base modules use `Arc`. Cortex-M4 no-default and async/no-std checks pass; Core fails on installed `riscv32imc-unknown-none-elf` because pointer atomics are unavailable. | Proven dependency and portability restrictions. They do not show that all embedded execution is impossible or that a new Contracts crate is necessary. |
| F6: Source representation restricts Consumer availability | The independent probe passes ordinary TD Basic validation with absent source id or an empty Property key, then observes Consumer admission refusal. [Planning](../../planning/src/consumer_property_read.rs) requires both. Original accounts return to baseline. | Availability restrictions, not failed cleanup. The new Producer admission already handles these cases; source provenance should not define live authority. |
| F7: Result sealing does not validate payload content | The independent probe supplies correctly attributed `application/json` bytes containing `not JSON` to the [Core result kernel](../../core/src/response.rs), which accepts their identity/shape. | Explicitly incomplete guarantee, not a new ownership defect. The current slice excludes broad schema validation. A sealed result is not proof of JSON syntax or conformance to the declared DataSchema. |

The independent probes are retained as
[test-only diagnostics](../../protocol-bindings/protocols/http/tests/architecture_reassessment.rs).
They assert current defects/restrictions or narrow evidence boundaries. Passing
means the observation was reproduced, not that the behavior is the target.
Future accepted corrections should invert or retire the corresponding diagnostic.

The standards baseline is the dated [TD 1.1 Recommendation](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/),
including [HTTP assertions](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#http-binding-assertions)
and context-extension security schemes. The [2024 Binding Templates](https://www.w3.org/TR/2024/NOTE-wot-binding-templates-20240528/)
is a Group Note supplying mapping guidance. These sources distinguish abstract
operations from native mappings; they do not select this report's Rust
representations, resource policies or compilation phases.

## Proposed architecture

### Responsibility and dependency boundary

| Component | Proposed responsibility |
| --- | --- |
| TD and semantic preparation | Preserve documents; validate and resolve names/context; compute shared WoT defaults, effective security and schema facts under explicit input profiles. |
| Planning | Filter requested operations, coordinate native lowering, select supported Consumer alternatives deterministically, check Producer coverage and construct executable entries. |
| Core | Small execution contracts: operation vocabulary, identities, values, shared result checks, ownership and lifecycle outcomes. |
| Servient | Install instance state; orchestrate runtime authorization, publication, dispatch, logical scheduling, cancellation and drain. |
| Concrete Binding | Validate and lower its native mapping; execute protocol I/O, correlation, multiplexing, flow control and infrastructure retirement. |
| Application/platform | Compose capabilities, credentials, clocks, executors and network services; select deployment limits, failure containment and recovery. |

Retain preparation as an independently useful component. It can run on a Host,
in a build tool, or on a device admitting dynamic inputs. Fixed execution does
not inherently require a TD parser or semantic resolver. Compiler facets can be
optional preparation dependencies while runtime facets execute compatible data.

Prefer slimming the existing Core into the execution-contract layer over
introducing a new Contracts package immediately. Moving shared operation values
out of TD and preparation/legacy behavior out of Core changes the dependency
direction deliberately. An extra package earns its place only if a remaining
cycle or independently useful distribution boundary requires it. Discovery and
codecs remain selected capabilities. Retire the builder's implicit Directory,
competing legacy TD-at-call-time execution, and obsolete compiler utility path;
do not preserve them as fallbacks for migrated capabilities.

The selected dependency direction is now explicit. In this adjacency list,
`A -> B` means **A depends on B**; optional preparation is an upward facet,
never a feature adding TD/Planning to Core:

```text
Core                  -> Foundation
TD                    -> Core, Foundation
Planning              -> TD, Core, Foundation
Binding runtime       -> Core, its native SDK
Binding / prepare     -> Binding runtime, Planning, TD
Servient execution    -> Core
Servient / prepare    -> Servient execution, Planning, TD
application           -> selected Servient, Bindings, Discovery, codecs, platform
```

The critical reversal is **TD -> Core**, after removing every Core -> TD edge.
This is a replacement of the current DAG, not an extra edge in it. Core cannot
retain `ResolvedBindingInput`, compiler authoring/erasure or legacy TD-dependent
client/security helpers under an optional feature. Compiler and runtime bundle
pairing is assembled above Core. Neither Planning nor Servient depends on a
concrete Binding; generic associated types connect them at the application.

This split follows responsibility, not today's file names. Runtime call
allocation, artifact compatibility and runtime erasure/storage helpers remain
below Planning even where they currently live in `core::binding_compiler`.
Compiler authoring/coordination moves upward. Core cannot re-export Planning to
preserve an old compiler path: that would restore the cycle. Registration
eligibility must also stop calling concrete compiler recognition from Core.

| Public contract/data | Selected defining owner and reason |
| --- | --- |
| Operation vocabulary, identities, payload/result and authorization/lifecycle contracts | Core; TD can use these without Core importing a document. TD owns its Serde wrapper/conversions for Core operation values, avoiding an orphan trait implementation or a parser dependency in Core. |
| Typed documents, resolved effective operations/security/URI facts, schema/context/extension trees and their paid loans | TD; this retains one semantic kernel. A resolved view is not a runtime record. |
| Selected-coordinate compiler input borrowing TD views; open compiler trait, pure cursors, outcomes, bounds and Host compiler erasure | Planning; a third-party preparation facet implements it. Moving the current public compiler contracts out of Core is necessary, not deferred placement. |
| Source-free common execution facts and generic owned definition/entry envelopes | Core; compiled validation facts do not contain `td::DataSchema`, a semantic arena or a source loan. Boolean result shape is the sketch's entire supported projection, not a replacement schema language. |
| Native program and typed operand representations, including compiler/executor mapping compatibility | Binding runtime module; the optional compiler produces this same type. No native type is defined by Planning or mentioned by Core. |
| Runtime registration/executor contract and private live instance/route records | Core contracts / Servient records; no compiler trait object is retained in an execution-only registration. Preparation captures and checks the real paired registration separately. |

The isolated [five-crate sketch](../../tools/architecture-fixtures/review08-boundaries/README.md)
compiles the critical Core/TD/Planning/Binding edges in distinct packages, using
language primitives in place of Foundation and std UDP in place of a native SDK.
It has no production crate dependency. Binding's
default runtime and Engine each have only Core as a normal dependency. Enabling
Binding preparation adds Planning/TD without changing Core's graph. Core, TD
and Planning sketches also compile on Cortex-M0 without pointer atomics. This
establishes feasibility of the proposed interfaces, not extraction or portability
of today's production Core. Exact public names, feature defaults, all legacy
removals and API/schema migration remain later reviewed changes. The projection is in
[module boundaries](../architecture/20-module-boundaries.md#proposed-preparationexecution-separation-review-08).

### Programs, instance context and live authority

An immutable program contains execution data. Checked instance context supplies
endpoint parameters, routing and security associations. Runtime records supply
current slot/generation authority, leases, calls and cleanup. Shared protocol
infrastructure has a separate physical owner. Credentials and live sockets,
handlers, tasks or cleanup obligations never become immutable program data.

A live generation-qualified execution entry can reference shared immutable
content without making its generation a content-equivalence key. Closing one
instance ends its rights; it cannot reclaim another instance's program or
infrastructure. A Plan Set remains useful as a collection. Its independent
generation/lifecycle should be required only where independently replaceable or
retained plan sets need it. Otherwise one instance owner suffices, with a
separate lifetime for shared immutable backing.

The architecture should permit preparing one definition and binding multiple
checked instances. It does not require parameterizing every program, a VM,
program registry, cache, interning, persistence or a serialization format.
Different endpoints may genuinely require distinct specialized programs. Same
model name, configuration digest or artifact format does not establish content
equivalence. Compare actual dependencies and operand contracts, including
schemas, security interpretation, templates and native mapping policy.

For N instances and K reusable definitions, measure definition compilation
separately from input validation, lookup, operand checking, installation and
retained physical bytes. Independent TD inputs still require validation; total
startup is not O(K). Interning after compiling each instance does not establish
reduced compilation. Persisted data would need a separate bounded format,
dependency/trust check and fresh installation; it cannot serialize authority.

#### Constructed definition and instance boundary

Select **explicit prepare-once / checked-bind-many**, with the compiler
declaring which inputs are definition dependencies and which are operands.
This corrects the earlier suggestion that a checked context alone could supply
reuse: there must be a callable binding boundary and a surviving program owner.
The sketch's minimal preparation interface is in
[Planning](../../tools/architecture-fixtures/review08-boundaries/planning/src/lib.rs):

```rust,ignore
trait ReadCompiler {
    type Program: 'static;  // owned native data, not necessarily Copy/Send
    type Operands: 'static;
    type Cursor: 'static;
    fn start(&self, definition: &td::ResolvedRead<'_>) -> Self::Cursor;
    fn step(&self, cursor: Self::Cursor, credits: &mut u8)
        -> Step<Self::Cursor, Self::Program>;
    fn bind(&self, program: &Self::Program, instance: &td::ResolvedInstance<'_>)
        -> Result<Self::Operands, BindError>;
}
```

The sketch's two-step native compiler produces a non-`Copy` boxed prefix plus
captured Boolean encoding policy. `bind` compares actual common/native dependencies
and policy, validates the suffix operand, and builds only an owned endpoint key.
It never calls `start`/`step`. The test destroys definition input and compiler,
then executes two instances with different operands and different Engine grants:
**one start, one completed compilation, two installed routes, one native socket**.
Changed dependencies, illegal operands, a different preparation policy and a
same-Rust-type/different-runtime-policy substitution all refuse before effects.
The type pairing alone is insufficient to establish configuration compatibility.

An explicit instantiation is not a claim that a second complete TD is equivalent.
For independent TDs, TD validation and dependency comparison must still run;
changed schema, effective security semantics, mapping policy or non-operand
target facts require refusal or a separately prepared definition. A selected
security association/credential may vary only within the definition's declared
operand contract. Current authorization always belongs to the Engine and is
checked independently per instance and again before accepting work; a program,
source id, parameter match or program reference grants no rights.

The ownership path is `definition owner -> immutable native program`, with each
instance/accepted call retaining a program lease plus its own operands and live
generation. The sketch uses `Rc` as a single-thread storage adapter: stopping A
cannot destroy B's backing, and an A reply retains the definition after both
routes stop. A weak reference and destructor counter show final program release
exactly once. This is explicit sharing, with no registry, lookup cache or interning.
Host thread-safe and caller-owned static adapters must enforce the same ownership
relations without making `Rc` or `Arc` a universal representation.

For funded production storage, the definition owner must retain its **original
allocation/account owner independently of the first instance**. Charge immutable
backing once there and charge each instance/call reference and operand at its
actual owner; release the backing charge only after physical destruction. Merely
cloning an `Arc` out of A's current per-instance account and returning A's allowance
at close is invalid. The sketch proves reference/drop ordering, not numeric
admission, original-account reconciliation or a qualified allocator.

[The lifecycle proposal](../architecture/30-compiled-plan-lifecycle.md#proposed-program-and-instance-lifetimes-review-08)
and [Planning amendment](../spec/planning.md#proposed-program-and-compilation-amendment-review-08)
project these changes. Topic [0081](../../workspace/0081-reusable-plan-and-thing-identity.md)
retains earlier reasoning, not a competing accepted contract.

### Open compilation without a universal mechanism

Keep compiler/executor pairing through native program types and checked semantic
and configuration compatibility. Consumer-only and Producer-only construction
remain possible. A different mapping for an existing WoT operation should not
require a Core compiler whitelist or protocol branch. A genuinely new semantic
operation or authorization contract can require an Engine capability change.

Inline `Copy` programs and atomic lowering are useful fast adapters, not the
general eligibility policy. Host preparation should admit owned immutable
programs and resumable pure jobs with qualified backing and cleanup. Static
profiles can require caller-owned storage and finite resumable work. Engine-held
native layouts and indirect backing remain funded. Linked callbacks are trusted
code: numeric work declarations cannot certify termination or hard real time.

Reconsider the universal all-bounds-before-any-start barrier. Pure compilation
can acquire funded provisional output sequentially and roll it back. Preserve
capacity before acquisition, reserved cleanup, no native effects during
compilation, full required checks before publication and atomic installation.
Error precedence and rollback costs change with sequencing and need explicit
review; the current barrier remains binding until that migration.

Consumer unsupported alternatives can be skipped; construction/resource failure
must remain distinct. Explicit selectors narrow the allowed result. Producers
must cover all retained advertised operations or explicitly construct a reduced
description before exposure. Neither compilation failure nor a returned request
authorizes automatic fallback after effects.

### Capabilities and environments

Separate `std`, async syntax, thread mobility, atomics, allocation/failure policy,
dynamic TD admission, role and hardware suitability. A Host adapter can require
`Send`; the portable execution contract need not. Static execution need not
include dynamic preparation or allocation-free ingestion. Shared semantic checks
can use borrowed views/common kernels while physical storage, synchronization
and scheduling differ. Features should not silently mutate globally named
payload semantics.

Prepared-data installation needs its own trust, compatibility and current-target
capacity checks; it cannot reuse a serialized validation proof. On-device
dynamic admission needs a supported finite preparation profile. Hardware fit,
latency and real embedded protocol production require separate measurements.

### Infrastructure ownership and resource policy

Things, programs, logical subscriptions, broker registrations, connections and
listeners have different cardinalities. Calls/routes can lease Binding-owned
infrastructure. Cancelling one releases its participation, without closing
resources another Consumer or Producer uses. Charge shared backing once at its
real owner, and references/wrappers at their respective owners. Last-user
retirement must include outstanding native jobs and callbacks, not just reference
counts, a closed flag or semantic completion.

This calls for an ownership contract, not a universal Engine session manager.
Bindings may compose shared or separate native owners. Permit-gated semantic
acceptance/dispatch can coexist with bounded native I/O and buffering outside a
route poll. A native callback still receives no handler-dispatch authority.

Distinguish call settlement, logical lease release, infrastructure retirement
and executor shutdown. Explicit asynchronous close retains its owner on Pending
or failure. A registration lacking asynchronous retirement supports a narrower
SDK arrangement; it does not prove every SDK should retire synchronously. Final
root loss needs a qualified local-disposal or owned-successor contract. Runtime
loss is an environment failure, not successful in-process cleanup. A residual
report never frees still-live local backing or proves remote rollback.

Retain ADR-0023's ownership-scoped target: Engine-controlled backing and promised
handoff capacity remain funded, Bindings enforce native input/concurrency/growth
policy, and deployment owns aggregate limits and failure containment. Generalize
this coherently across roles instead of accumulating permanent operation-specific
resource variants. Preserve real Layout/overlap and original-account evidence.
Optional quota banks/allocators remain deployment choices; a numeric reservation
is neither physical backing nor memory protection. Use applicable policy
projections rather than requiring the full schema in every runtime record.

The proposed observable contracts are in
[Binding SPI](../spec/binding-spi.md#proposed-role-and-infrastructure-amendment-review-08).

#### Constructed shared Producer boundary

The shared socket experiment requires a correction to **route-scoped progress**:
native ingress/retirement must have a separately owned progress domain. A shared
listener cannot require a permit for an arbitrarily chosen Thing before it can
discover the native request's destination. Retain route-scoped semantic acceptance
and response ownership; do not equate them with exclusive native-driver ownership.

The first rework incorrectly generalized the UDP adapter's equality-key -> one
recipient index. Native matching is a relation: overlapping or identical filters
can identify multiple independently authorized logical recipients. MQTT 3.1.1
allows overlapping subscriptions to be represented by one incoming packet;
its topic filter matching belongs to the protocol implementation. Logical
fan-out cannot be inferred from packet multiplicity.
[MQTT 3.1.1 §§3.3.5, 4.7](https://docs.oasis-open.org/mqtt/mqtt/v3.1.1/os/mqtt-v3.1.1-os.html).

The selected boundary is now **Binding-owned native matching over Engine-installed
registrations -> zero/many generation-pinned candidates and captures -> independent
Engine acceptance/dispatch**. The Engine owns the logical registration/generation
to handler table, current authorization and publication root. The Binding owns
native selectors/patterns, matching, capture extraction and native observation
storage. It receives read-only registration tokens minted from the actual Engine
records. A candidate carries that exact issuer/registration/generation, captured
native facts and a lease on its immutable packet. Matching grants no authority;
the Engine neither decodes patterns nor lets a Binding choose an authorized handler.

The [UDP Binding](../../tools/architecture-fixtures/review08-boundaries/binding/src/lib.rs)
and [exact-unicast Engine adapter](../../tools/architecture-fixtures/review08-boundaries/engine/src/lib.rs)
still demonstrate two routes sharing actual I/O. Equality lookup and unique-key
collision refusal are valid for that adapter's exclusive endpoints. They are
not required by the general matching boundary or by logical subscriptions.
Identical native filters may share a physical registration while retaining
distinct logical tokens, grants, deliveries and stop semantics. A capability
must declare its exclusivity/multiplicity; this does not authorize cloning a
one-shot response opportunity into several requests.

The construction is:

1. Check operand/program/runtime compatibility, current per-instance authorization,
   native collision/sharing semantics and capacities before effects. Install the
   Engine-minted logical tokens alongside Binding-owned native selectors; one
   filter need not mean one logical registration. A failed/partial installation
   retains its owner and cannot publish.
2. Install independent participation leases into the same physical owner. Ready
   routes retain read-only gates; only the Engine keeps the captured authority root.
   Publication changes the selected ready group without callback/yield. This
   proves a local single-thread boundary, not production concurrent publication.
3. Perform bounded native I/O/matching without a route permit. Enumerate all
   matching logical registrations within declared work, candidate and capture
   bounds. Retain one physical packet backing with separately bounded recipient
   references. Overflow has an explicit refusal/backpressure disposition; do not
   silently truncate recipients or substitute an unbounded broadcast.
4. For **each** candidate, recheck exact issuer/record/generation, publication,
   current authorization (including applicable captured facts) and that recipient's
   delivery capacity. Lend a non-`Clone` permit scoped to this accept. The Binding
   checks the captured token and returns this recipient's owned event/reply.
   Only the Engine selects/invokes its handler. Refusal/cancellation of A cannot
   consume B's participation or copy of the shared data. A newly created authority
   with identical numeric ids cannot impersonate the captured root.
5. A deferred candidate keeps its original token and captures; it is never matched
   again to a successor by filter/key after slot reuse. Stop A by revoking/removing
   its Engine record before removing its exact native logical registration.
   B remains eligible even with the same native filter. Already accepted A work
   retains its original handler/program/native data through settlement; it cannot
   authorize fresh ingress. Native acknowledgement/correlation stays at its actual
   Binding owner, not one acknowledgement per logical recipient.
6. **Only final route participation release initiates ordinary retirement internally.**
   A clone's `poll_retire` only progresses an already initiated retirement; an
   early poll returns Pending without changing ingress or attachment state.
   Observations, replies and SDK jobs/callbacks keep the retiring physical owner
   alive until terminal, then it closes once. Whole-infrastructure shutdown would
   require separate authority to revoke/drain all dependents first; this candidate
   adds no such bypass. Inert root handles are not users or shutdown authority.

Executable negatives cover prepublication/revoked admission, another issuer's
permit, cross-route permits, old-generation permits, buffered input after slot
reuse and send failure retaining the complete reply. The reviewer reproduced an
early poll suppressing B's ingress despite its live route. The retained UDP
regression failed before the correction, then passed with B still receiving,
replying and admitting a replacement sibling after that poll. Final observation,
reply and job drain still precede exactly one socket close.

The companion [Binding matcher](../../tools/architecture-fixtures/review08-boundaries/binding/src/matching.rs)
and [Engine recipient dispatcher](../../tools/architecture-fixtures/review08-boundaries/engine/src/matching.rs)
construct the relation without a protocol case in Engine. Two tests exercise
`meters/+/watts` with `meters/m1/#`, and identical `meters/+/watts` filters, over
one decoded packet. Captures, zero/many candidates, shared backing, independent
revocation/cancellation/removal, foreign issuer rejection and queued generation
replacement are checked. A one-event credit for A does not block B using the
second packet backing; aggregate exhaustion reports backpressure. Accepted A
work retains its old handler through replacement. Limits are two registrations,
two packet backings, at most two candidates per packet, four topic levels and
32-byte topic/payload bounds; captures are bounded by those inputs.
The probe's current grants are simple principal/slot pairs; authorization over
captured semantic facts remains a production security obligation.

These are **separate** evidence pieces: UDP supplies real exact-unicast I/O and
retirement, while the matching probe injects bounded decoded packet data. It
implements no MQTT wire protocol, broker session, acknowledgement/QoS, subscription
installation/retirement, or WoT `ObserveProperty` lifecycle. There is no combined
MQTT/SDK or general protocol-neutrality claim. The native job is a manually held
lifetime token. The compiler's step-credit probe does not establish funded
construction: start/bind/finish can allocate. Predebit for bounds/start/bind/step/
abort/erasure, owner-preserving failures, bounded cleanup and original-account
reconciliation remain mandatory future work.

The socket/retirement state has one physical owner; each route, observation,
response and job owns a bounded participation token. Production must fund this
owner and its eventual close destination before native acquisition, independently
of whichever route starts first. A route completion cannot release shared backing
or count as final native shutdown. The sketch's counters are ownership evidence,
not resource-schema measurements. A genuinely asynchronous SDK still needs a
funded retained closer and qualified final-root successor; a tombstone or last
`Arc` drop is insufficient. That API and SDK qualification remain unselected.

### Reassessment of the accepted HTTP Producer admission

Keep its whole-description coverage, source-free entry, retained setup/close
owner, private publication and borrowed permits. A single driver can preserve
the guarantees without four public native setup-phase families.

Its one route/listener/connection/response opportunity is a reasonable first
implementation limit. Before generalizing the public boundary, support owned
programs/program references beyond `Copy`; separate preparation from driver
construction; permit shared infrastructure leases; share entry/lifecycle
semantics with static execution; and handle optional source id and valid
Property keys consistently across roles. Keep distinct Consumer and Producer
operation topologies rather than forcing one universal execute trait.

The accepted [Producer scope](../work-packages/WP-600-http-producer-property-read.md)
is unchanged by this recommendation. Its current obligations, including bounded
input and final-root retirement, remain binding. A real public HTTP loop and
contrasting shared-infrastructure evidence are still outstanding. Architecture
review must decide the amendment before execution follows it.

## Alternatives challenged

| Alternative | Assessment |
| --- | --- |
| Repair only current methods and coverage, then freeze the current SPI | Necessary local repairs, but leave demonstrated compiler/TD extensibility and environment coupling unresolved. They can make the narrow path safe without completing the architecture. |
| Preserve mandatory logical plans, per-generation content ownership, all-bounds barriers and four setup phases universally | Existing evidence proves useful invariants through these mechanisms. It does not establish their necessity or acceptable authoring cost across different protocols/environments. |
| Move all TD interpretation/selection/dispatch into Bindings | Duplicates shared defaults, security and selection; weakens truthful advertisement and dispatch authority. Reject. |
| Keep public compiler traits in Core while making TD depend on Core | Core's resolved TD inputs immediately create Core -> TD -> Core. Reject. Move compiler traits/erasure and selected wrappers to Planning; keep TD's resolved views in TD. |
| Move the entire semantic tree into Core, or make TD depend on Planning | The former drags document/schema/preparation representation into the execution base; the latter cycles Planning -> TD -> Planning. Reject. Core stores only compiled execution facts. |
| Leave Core -> TD and place compilers in Planning | Acyclic and a credible narrower repair; retains TD on every fixed runtime path. Prefer the demonstrated reversal for the stated execution-only goal; it requires explicit legacy/security/API migration, not a feature alias. |
| Universal protocol IR/VM, extension-criticality framework or Engine session manager | Adds an unproved representation/policy framework. Native typed programs plus explicit owners provide the needed separation without selecting it. Reject for this target. |
| Split Host and embedded runtimes, or add a Contracts crate immediately | Credible if actual dependency/backend evidence requires it. First separate capabilities and slim Core; avoid duplicating semantic algorithms. |
| Deduplicate completed instance artifacts or share an instance generation as the program identity | Still repeats compilation or couples independent authority/reclamation. Reject as the prepare-once capability; explicit bind and independent program ownership solve the demonstrated case. |
| Poll one shared listener exclusively through one route's driver/permit, or add a universal Engine session manager | The former can block sibling traffic/retirement when its route stops; the latter unnecessarily centralizes native protocol policy. Select Binding-owned infrastructure progress with Engine-owned semantic routing and per-route permits. |
| Require one equality-key recipient, or decode native patterns in Engine | The former loses overlapping/identical-filter logical recipients; the latter moves protocol semantics into Engine. Keep equality as an exclusive-unicast adapter; select bounded Binding matching/captures plus separate Engine acceptance per pinned logical token. |
| Let any participation handle initiate retirement by polling | Can disable an authorized sibling before its lease drains. Reject. Final route release initiates ordinary retirement; polling grants progress only. An explicit whole-infrastructure shutdown would need separate authority. |
| Shared immutable program references, checked instance context and separate native infrastructure owners | Selected, constructed by the narrow five-crate/UDP and decoded-packet matching sketches. Production funding, concurrent publication, async SDK retirement and other operation/profile evidence remain independent obligations. |

## Change boundary and accepted authority

| Existing authority | Proposed change; accepted guarantee to retain |
| --- | --- |
| Module boundary / Core API ownership (`CRATE-DEPS-001`) | Reverse TD/Core only after removing Core's TD/legacy edges; compiler contracts/erasure move to Planning, resolved semantic loans stay TD-owned. Change exact public ownership/feature projections before source migration. |
| ADR-0008 and Planning's mandatory two-level/per-generation representation | Permit minimal execution entries and separately owned reusable content; retain source-free output, selection identity, publication, leases and final physical reclamation. The accepted Host Producer subset already makes a narrower exception. |
| Planning compiler eligibility and all-bounds ordering | Admit external owned/resumable jobs through generic pairing; permit funded incremental pure construction with reviewed error precedence and rollback. |
| Binding/Servient lifecycle and Host Producer admission | Permit bounded native matching/captures over Engine-installed registrations and independent acceptance per pinned candidate. Separate native progress, per-recipient delivery and retirement authority; keep exclusive route adapters. Retain private publication, first cause, late results, complete transfer and truthful terminal state. |
| Resource interpretation and Host/static contracts | Generalize ownership/disposal policy across roles without omitting Engine backing, weakening strict profiles or relabeling old measurements. |
| TD/security and interaction guarantees | Preserve extension data separately from execution support; keep runtime authorization mandatory; remove source-id/key availability assumptions deliberately. Sealing remains distinct from schema validation. |
| Work packages, state/API/resource matrices and evidence | Update affected projections only in a later accepted migration. Renew changed carrier/cost/trace evidence; preserve unrelated scoped evidence. No gate or package transition follows from this report. |

No accepted ADR is silently superseded, and no proposed identifier/type becomes
an API registration. Review 07 and topics 0056/0076/0081/0082 remain useful
inputs; this integrated recommendation challenges their sequencing/abstractions
without changing their acceptance or lifecycle state. Exact implementation
scope and authoritative artifacts should follow the chosen boundaries, not a
predetermined file or work-package structure.

## What can wait and what should happen next

Change the architectural boundaries now, before treating the current SPI as
final. The ensuing source scope includes F1/F2 repairs and retirement of
competing legacy entry points. This report authorizes none of those edits.

Defer automatic interning/eviction/persistence until a useful reusable definition
and measured compilation benefit exist. Defer general remote-context processing
until explicit bounded profiles prove insufficient. Defer automatic cross-Thing
subscription grouping until shared leases and independent stop semantics are
proved. Full protocol/operation parity, fleet throughput and embedded production
need their own evidence. Runtime code loading/hot unload remain outside v1 goals.
HTTPS/security remains a release requirement. Schema validation needs an explicit
capability/enforcement policy before claiming payload conformance.

Recommended sequence after this rework:

1. Independently review the integrated direction, then reconcile the affected
   accepted contracts and exact implementation scope. Avoid a new global gate
   or repeated current-state summaries.
2. Review the bounded prototype retained here, including its negative probes and
   limits. Renew production storage/account and static execution evidence when
   migrating the actual contracts; a miniaturized stand-in cannot establish those.
   Require an actual session/broker-shaped contrast before claiming general
   protocol-shape neutrality. No second production Binding is required here.
3. Implement the real public HTTP Consumer-to-Producer loop under the accepted
   corrections, including zero-effects refusal, source destruction, partial
   installation, cancellation, saturation and final-owner retirement. Qualify
   HTTPS/security before release. Renew only intersecting physical/semantic
   evidence when the contracts change.

Falsification matters: parameter-only changes should reuse a qualified definition
when it promises that capability; real dependency changes should create another
definition or reject. Closing one instance must preserve another's data, funding
and native service. Final physical release occurs exactly once. Preparation
refusal, native failure and exhaustion cannot publish partial authority.

## Reproduction and validation boundary

Original assessment environment: Linux x86_64, Rust 1.95.0. The source observations
and executed baseline checks below refer to `700cfdb1`. The rework adds proposed
text, diagnostics and an isolated sketch workspace; production code is unchanged.
PR-head checks belong to the PR's validation record.
No full workspace, real embedded execution, general interoperability, completed
HTTP Producer or architecture-closure result is inferred.

```sh
cargo test --locked --offline -p clinkz-wot-core -p clinkz-wot-planning -p clinkz-wot-servient
cargo test --locked --offline -p clinkz-wot-binding-http --test semantic_compilation --test producer_admission
cargo test --locked --offline -p clinkz-wot-property-read-architecture-runner --test producer_admission -- --nocapture
cargo test --locked --offline -p clinkz-wot-property-read-architecture-runner -p clinkz-wot-consumer-property-read-runtime-runner
cargo test --locked --offline -p clinkz-wot-property-read-architecture-runner -p clinkz-wot-consumer-property-read-runtime-runner --no-default-features
cargo check --locked --offline -p clinkz-wot-core -p clinkz-wot-planning -p clinkz-wot-servient --no-default-features --target thumbv7em-none-eabihf
cargo check --locked --offline -p clinkz-wot-core -p clinkz-wot-planning -p clinkz-wot-servient --no-default-features --features async --target thumbv7em-none-eabihf
```

These passed, with existing warnings in portable cells. The HTTP selection ran
14 semantic tests and one native Producer feasibility test. The Producer and
Consumer runners ran 19 Host and 14 no-default tests. The diagnostic separately
reproduced all thirteen Producer cases. The generic compiler restriction is a
passing counterexample test, not successful installation of that compiler.

The four independent probes originally ran outside the workspace against the
same source, with registry versions verified against the repository lockfile.
Their retained test target is reproducible without a new fixture workspace,
dependency, checker, workflow or manifest change:

```sh
cargo test --locked --offline -p clinkz-wot-binding-http --test architecture_reassessment -- --nocapture
```

With the target standard library installed, this command intentionally exposes
the current portability restriction; it is not a new expected-failure CI gate:

```sh
cargo check --locked --offline -p clinkz-wot-core --no-default-features --target riscv32imc-unknown-none-elf
```

It fails with `E0432` on `alloc::sync::Arc` in base Core modules and consequent
type errors. Cortex-M4 compilation is separate evidence and proves no board fit.
The required GitHub `validation` status checks diff hygiene only.

The constructibility rework ran on Linux x86_64, Rust 1.99.0. Reproduction commands
and exact evidence limits are in the
[sketch README](../../tools/architecture-fixtures/review08-boundaries/README.md).
The new tests exercise the proposed interfaces, not production contracts; the
original four diagnostics remain observations of unchanged production behavior.
