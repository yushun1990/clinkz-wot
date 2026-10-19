# 0056 Target SPI External Validation and Cross-Protocol Neutrality

Status: DISCUSSING

Kind: owner-raised architecture investigation

Priority: HIGH

Target: conditional protocol independence for already supported WoT semantic capabilities and the evidence boundary of the public Binding/Servient SPI

The original investigation, decision and migration below record the pre-probe
feedback-order decision. That migrated history is preserved. The
[reassessment](#reassessment-conditional-protocol-independence) reopens the assurance
question in this topic, consolidating PR #205's former 0090 proposal with 0042.
The [decision refinement](#decision-refinement-bounded-property-read-contract)
below selects a bounded correction and projects its contract into the existing
Planning and Binding owners. It remains a candidate for independent acceptance;
source admission and empirical release evidence retain their existing owners.

## Scope and authority

This topic records a Project Owner concern, strengthened by an independent Ultra project review, that the current narrow Property Read slices and repository fixtures may establish internal consistency before the target Protocol Binding SPI has received enough feedback from real protocol implementations.

This is an investigation request, not an instruction to replace the current SPI, skip the Property Read architecture gate, adopt a particular alternative, or make Zenoh the protocol-neutrality proof. Accepted architecture remains implementation authority until Codex investigates the evidence and migrates any stable correction through normal architecture governance.

## Owner observation

The project has now proved important protocol-neutral ownership properties through narrow Planning, Binding, and Servient Property Read slices, while the real Zenoh implementation still follows the legacy path and the target binding interfaces are exercised primarily by compile/runtime fixtures and mocks.

An independent Ultra review therefore raised a credible counterexample to the current feedback order: continuing aggregate mock closure before using a real target binding may produce more evidence that the repository matches its declared model without testing whether that model is natural for actual protocol I/O, correlation, cancellation, buffering, readiness, cleanup, and concurrency.

The Owner agrees that a real Zenoh target slice is a useful first external probe, but explicitly rejects treating Zenoh alone as proof or disproof of protocol neutrality. Zenoh can test whether the current SPI naturally represents one real protocol family. A protocol-neutrality claim requires evidence across materially different protocol interaction models, or another justified method that can expose protocol-specific assumptions.

## Independent-review evidence to investigate

The Ultra review reported, among other findings:

- the TD -> Planning -> Binding -> Servient macro layering remains credible;
- immutable plan/artifact execution, Servient-owned handler selection/publication/cleanup, generation-bearing identity, bounded caller-driven progress, and protocol-owned I/O/correlation remain strong principles;
- target Host/static authoring and route behavior currently have substantial duplicated structure;
- prepare/readiness/activate/commit and other detailed lifecycle carriers were specified before a real target binding exercised them end to end;
- the real Zenoh target migration remains later than the current aggregate Property Read fixture path;
- narrow fixtures validate important ownership semantics but do not yet establish external validity for multiple real protocol models.

These findings are evidence inputs, not predetermined conclusions. Codex should verify them against the current fetched default branch and any newer D48 state before deciding.

## Questions for investigation

1. What architectural claims have the completed Property Read slices actually established, and which target SPI claims remain provisional until exercised by real bindings?
2. After D48 convergence, would the planned aggregate Property Read mock/fixture produce materially new architecture feedback before a real target binding, or mostly strengthen conformance to the existing model?
3. Should the roadmap place a minimal real target binding probe before, inside, or after the aggregate `PROPERTY-READ-ARCHITECTURE` gate?
4. What is the smallest real Zenoh Property Read slice that can test compiler/artifact construction, real I/O, correlation, readiness, failure, cancellation/drain, cleanup, multiple Things/routes/forms, and at least one real network round trip without turning the probe into full binding migration?
5. Which current public lifecycle stages and carrier types are genuinely required by that real Zenoh implementation, and which appear to be internal binding state or duplicated Host/static machinery?
6. Is one typed poll-driven kernel with profile-specific storage/erasure a credible simplification of the current Host/static split, or does repository evidence justify the present separation? If comparison is needed, what bounded spike would falsify either design?
7. What evidence is required before a target SPI type or lifecycle is considered stable/frozen rather than provisional?
8. What constitutes adequate evidence for protocol neutrality? In particular, what protocol-model differences must a second real binding expose beyond Zenoh so that shared abstractions are not merely Zenoh-shaped?
9. Should the second protocol be selected by transport name, or by deliberately different semantic properties such as request/response correlation, connection orientation, broker/topic routing, streaming/subscription behavior, server push, security/session ownership, constrained polling, or message-size/backpressure behavior?
10. Which candidate protocol family or families provide the highest-value contrast for v1 validation, and what is the minimum comparison needed before claiming the target SPI is protocol-neutral in practice?
11. How should real-binding feedback interact with the existing accepted ADRs, work-package gates, D48 validation machinery, and legacy/target coexistence without silently bypassing current authority?
12. What repository-visible result should end this investigation cycle: reaffirmation of the current SPI, a bounded comparative spike, a revised gate order, reopening of specific ADR/spec ownership, or another evidence-backed outcome?

## Constraints

- Preserve the protocol-neutral macro architecture unless evidence supports changing it.
- Do not equate successful Zenoh implementation with proof of protocol neutrality.
- Do not equate Zenoh friction alone with proof that the SPI is not protocol-neutral; distinguish a poor implementation, a protocol-specific mismatch, and a genuinely overfit abstraction.
- Cross-protocol evidence should be chosen for semantic diversity, not merely for adding another adapter with similar interaction mechanics.
- Do not require full production implementations merely to obtain architecture feedback; prefer the smallest real slices that can falsify important assumptions.
- Do not let a spike silently become authoritative implementation. Stable conclusions must migrate through the normal architecture-governance process.
- Do not discard boundedness, ownership, no_std constraints, cancellation/cleanup discipline, or protocol-neutral orchestration merely to reduce code volume.
- Do not continue freezing detailed SPI surface solely because existing fixtures and checkers can validate it.

## Expected decision output

Codex should determine:

1. whether current target-SPI external evidence is sufficient for the next planned aggregate gate;
2. whether a real Zenoh target probe should precede or alter that gate;
3. which current SPI details remain stable versus provisional;
4. whether a bounded alternative-kernel comparison is warranted;
5. the evidence standard for protocol neutrality and the semantic criteria for selecting at least one contrasting protocol model;
6. any required roadmap, work-package, ADR, specification, or gate changes; and
7. the exact next independently reviewable engineering claim, without beginning a materially distinct successor in the same review cycle.

## Decision

The completed narrow Property Read slices establish package-local
constructibility, ownership, generation, route identity, bounded progress,
permit-gated acceptance, response opportunity, and cleanup-transfer claims.
They do not establish that the detailed public Binding/Servient lifecycle is a
natural fit for a real protocol implementation. The planned aggregate mock
gate would strengthen internal cross-package conformance, but would mostly
exercise the same repository-authored model and therefore is not the highest
value next feedback step.

A bounded, non-authoritative **real-target Zenoh Property Read feedback probe**
must precede aggregate mock-gate candidate and source admission. It uses the
current target public Planning, Binding, and Servient boundaries and a real
Zenoh library/network path. At minimum it carries a real immutable plan and
route artifact into the first legal target entry, exposes and reads one
property across an actual network round trip, covers externally visible
readiness, correlation, response, failure, cancellation/drain, and terminal
cleanup, and includes enough multiple-Thing/route/form shape to expose hidden
single-fixture assumptions. It may use separate spike/fixture code but may not
replace an upstream output with a fixture-owned substitute or call legacy
selection/dispatch.

The probe is architecture evidence, not WP-600 product implementation,
progress, or source admission. It records the public declarations and helpers
needed, repeated workarounds, duplicate semantic transitions, diagnostics,
private/unsafe dependencies, generic/layout/code-size pressure, resource
mapping, and lifecycle stages that collapse into protocol-local state. A
concrete ownership, boundedness, portability, implementability, or systematic
authoring defect reopens the exact affected authority; the Executor preserves
evidence instead of bypassing it. A clean result supports, but does not itself
pass, the later aggregate mock gate.

The macro architecture and its semantic invariants remain frozen:
protocol-independent Core/Planning/Servient ownership, immutable artifacts,
protocol-owned I/O and correlation, Servient-owned dispatch/publication,
generation identity, route-scoped bounded progress, permit-gated acceptance,
and explicit cleanup transfer. Exact Host/static containers, detailed
prepare/readiness/activate/commit carriers, helper surface, and
operation-family signatures remain provisional until the probe supplies real
feedback. No alternative-kernel spike is justified before that evidence;
implementation friction may make such a comparison the next bounded claim.

Zenoh and zenoh-pico remain one protocol family. Their difference is valuable
cross-profile evidence, not independent protocol-shape neutrality. A general
empirical neutrality claim requires an independently authored fixture selected
for semantic contrast—such as materially different routing, correlation,
response, connection, push/stream, cancellation, or constrained-backpressure
behavior—not merely a different transport name. That contrast is not required
to expand v1 product scope: absent such evidence, the honest v1 claim is
protocol-independent engine ownership plus Zenoh-family operation.

## Migration

The reordered feedback ladder and maturity boundary are migrated to `PLAN.md`,
`ARCHITECTURE_GOVERNANCE.md`, `docs/spec/binding-spi.md`,
`docs/work-packages/WP-300-bindings.md`,
`docs/work-packages/PROPERTY-READ-ARCHITECTURE.md`, and
`docs/work-packages/WP-600-protocol-bindings.md`. The aggregate gate stays
`ready`; no fixture or product source is admitted in this decision cycle.

Displaced default: begin the aggregate mock candidate, then obtain real Zenoh
feedback later. New default: real target feedback, explicit SPI disposition,
then aggregate mock. First activation claim: a future Max-authored
`EXECUTION.md` for the bounded probe; this reset deliberately leaves that file
`IDLE`. Falsifier: aggregate candidate/source work begins without accepted
probe evidence, or release material treats Zenoh/zenoh-pico alone as general
protocol-shape neutrality. Such evidence reopens this topic.

## Reassessment: conditional protocol independence

### Boundary and conclusions

This assessment reconstructs production source and registered contracts at
`22047e07dbd71b97af38b5ddc5c8e3a3755e7352`. It examines PR #204's diagnostic at
`ad703fb2468b00b44ebbf509ff316e1f66ec88a4` and the independent review of PR #205
at `2d02fd74ab23160c7d89e1414e8574c28780343f`. GitHub owns subsequent PR and
review state. The earlier Zenoh feedback and completed HTTP Consumer evidence
are prior, scoped evidence, not new experiments in this assessment.

The conclusions are:

1. The architecture supports protocol-independent semantic and execution
   authority, but its ownership diagram alone does not establish substitutability
   through the actual public contracts. Information sufficiency, installable
   extension paths and lifecycle refinement are additional obligations.
2. PR #204 demonstrates a Producer information/coverage defect against accepted
   Planning semantics. Independently, Consumer compiler admission is deliberately
   closed. Therefore an unrestricted claim that any new protocol for Property
   Read needs only a new binding is unjustified on this baseline.
3. Protocol-private artifacts, state and correlation are credible extension
   boundaries. Their implementation conformance, resource declarations and real
   cleanup require evidence; an open trait cannot certify those behaviors.
4. Producer publication and cleanup authority do not logically require four
   native protocol phases. The mandatory public phase split deserves challenge,
   but the inspected contracts establish no general lifecycle impossibility
   warranting a replacement SPI now.
5. Retain the accepted macro architecture while correcting demonstrated defects
   through their existing owners. Use the conditions below to assess further
   defects. This assessment selects neither a compiled-program redesign nor a
   protocol experiment as the next architectural proof. Existing capability and
   tranche controls govern intersecting defects; no new broad-runtime prerequisite
   follows from opening this assurance question.

### Precise conditional property

Fix a supported semantic scope `K`: WoT operation, Producer or Consumer role,
security/result/error obligations, generation/publication/cancellation/cleanup
rules, supported profile and declared concurrency/resource envelope. On this
baseline the implemented one-shot Property Read paths supply different such
scopes. `K` describes observable obligations and policy, not today's sequence of
guard types. A future ObserveProperty capability is a different `K`.

For a protocol `P`, declare a nonempty input/configuration envelope `U` and a
mapping of native interactions to `K` before assessing compatibility. Shrinking
`U` after a counterexample explicitly narrows the claim. The external premises
are that:

- the mapping can express the promised WoT operation and preserve its observable
  values, source/target attribution, authorization and error meaning;
- protocol effects can be mediated without bypassing handler authority, and
  native resources can have truthful bounded ownership and terminal disposition
  under the selected profile, including declared external residuals;
- required environment services and fair application-driven progress are
  available for successful completion; failures retain the specified uncertainty
  and cleanup semantics.

These premises describe native feasibility against semantic obligations. They
must be assessed independently of the current Rust method names and guard types;
assuming that a binding already implements this SPI would assume the conclusion.

**Conditional independence means:** for each `P` satisfying these premises for
`K` and `U`, a binding and ordinary application composition can implement that
mapping through the public contracts, with no protocol-specific interpretation,
dispatch authority or lifecycle branch added to Core, Planning or Servient.
The adapter must accept supported inputs in `U`, reject unsupported declarations
before their interaction effects, and preserve the allowed `K` traces after
protocol-private steps are hidden. It may specialize native scheduling, storage,
session sharing and physical costs. A healthy supported case must complete;
rejecting everything cannot establish the property.

Equivalently, for inputs in `U`, the semantic projection of engine-plus-binding
traces must be contained in the allowed `K` traces under that native mapping;
under the stated health/progress premises, supported interactions must also
reach their specified result or terminal disposition. Trace inclusion alone
would allow an adapter that never progresses.

Trace preservation concerns publication, authorized dispatch, owned request/result
attribution, truthful errors and uncertainty, cancellation and cleanup disposition.
It does not require identical wire messages, timings, physical resource counts or
success against a failed peer. Exactly-once local settlement is not exactly-once
remote delivery. Closure of local acceptance is not atomic network publication or
reversal of a request already sent.

The architecture must discharge the six obligations below to derive this property.
They are proof obligations, not extra premises excluding every awkward protocol.
A protocol with a valid semantic mapping and admissible native implementation
that cannot traverse the public boundary is a counterexample. New shared WoT
semantics require their own capability decision; a generic contract repair must
be identified as a repair rather than described as extension success.

### Six obligations and their evidence

| Obligation | Structurally assessable guarantee | Implementation evidence and present limit |
| --- | --- | --- |
| Semantic information completeness | Before effects, each distinction needed for applicability or correct execution reaches the responsible component as resolved facts, preserved bounded extension data, or a trusted checked eligibility result. No required distinction may be silently erased. | Projection/default/security/coverage implementations must establish that relation for all admitted inputs. The Producer path fails it as described below. |
| Extension openness | Public payload/state types, compiler input and complete registration must permit external implementation in the claimed role/profile. A closed application enum is ordinary static composition; a Core-maintained list of permitted compiler implementations is a narrower boundary. | The actual installed paths below, including construction, rejection and execution, determine openness. Public trait visibility alone is insufficient. |
| Dependency direction | The neutral path depends on semantic contracts; the binding depends on those contracts and its SDK. Shared control flow need not know protocol addresses, headers, correlation rules or transport types. | Inspect manifests, enabled features and reachable source. Core and Planning have no concrete binding dependency. Servient still has the legacy shared binding dependency and optional Zenoh dependency under `test-zenoh`; this baseline cannot support a blanket crate-wide dependency-purity claim. |
| Protocol-private mechanisms | Immutable artifact payloads and typed/erased route or call state can represent protocol data without exposing its layout to neutral orchestration. A logical route owner need not equal a physical session owner. | Compilers must be pure; reactors, session references, queues and private mutation must preserve declared bounds, fair progress and final-owner cleanup. Types do not establish SDK behavior or safe shared-session retirement. |
| Execution authority | Selected generation-qualified artifacts, Servient-owned publication/selection, temporary route permits, owned response opportunities and Core result sealing constrain the ordinary target path. Binding callbacks receive no handler or registry authority. | Servient must issue/check the right permit, source, generation and target, and reject late/stale work. Binding code must obey the contract. Rust visibility cannot prevent arbitrary user code, ambient globals, unsafe code or a lying SDK adapter from violating it. |
| Lifecycle substitutability | Native internal states must refine the shared ownership/publication/cleanup obligations, including immediate or stuttering transitions, without making a protocol handshake a shared semantic rule. | Inspect every failure/cancellation boundary and any dependency on private peer progress. Current phase carriers constrain legal adaptation; they do not prove all feasible native protocols can be adapted. |

Relevant structural owners are [module boundaries](../docs/architecture/20-module-boundaries.md),
[plan lifecycle](../docs/architecture/30-compiled-plan-lifecycle.md),
[Planning](../docs/spec/planning.md) and [Binding SPI](../docs/spec/binding-spi.md).
The last two own `PLAN-ARTIFACT-001`, `PLAN-COST-003`, `FORM-COVERAGE-001`,
`BIND-REG-001`, `BIND-ROUTE-001`, `BIND-IO-001` and the active cancellation and
memory clauses. Source ownership is checked through
[plan values](../core/src/plan.rs), [compiler contracts](../core/src/binding_compiler.rs),
[binding contracts](../core/src/binding.rs), [Producer Planning](../planning/src/property_read.rs)
and [Producer orchestration](../servient/src/property_read.rs).

### Installed extension paths

| Role/profile | Actual public construction and execution path | Extensibility conclusion |
| --- | --- | --- |
| Host Producer Property Read | `BindingCompilerExtension` -> `HostBindingCompilerRegistration::new` -> `HostBindingRegistrationInput` with `RouteServerBinding` -> `HostBindingRegistration::new` -> `ServientBuilder::binding_registration` and the public produce/handler/expose/step/destroy path. | Generic compiler erasure is usable in a complete Producer installation. Compatibility, declarations and identity checks remain mandatory; Producer resources require `StrictV1`. Openness does not repair missing compiler information. |
| Static Producer Property Read | `StaticBindingCompilerRegistration<C>` and `PollServerBinding<Compiler = C>` -> `StaticBindingRegistration` -> `StaticServientBuilder`. Heterogeneous tables use application-owned matching compiler/cursor/artifact enums. | Application closure is not a shared protocol whitelist. Compiler work is currently restricted by `compiler_work_is_portable` to `BindingPolls`; arbitrary work declarations are not admitted. |
| Host Consumer Property Read | Complete Consumer-only `HostConsumerBindingRegistration` or eligible dual-role owner -> `try_into_consumer_compiler` -> checked `ConsumerCompilerRegistration<R>` -> Consumer Planning/runtime. `HostBindingCompilerRegistration::try_new_consumer` supplies the supported adapter. | Core's private supported `ResolvedTargetCompiler` adapter is required. Generic `new`, even wrapping that compiler, is ineligible. External clients can use the existing resolved-target artifact where sufficient; arbitrary external compiler lowerings cannot be installed. |
| Static Consumer Property Read | Complete Consumer-only or eligible dual-role static owner -> `try_into_consumer_compiler` -> checked Consumer Planning and `StaticConsumerPropertyReadRuntime`. | Core checks the actual compiler type/configuration against `ResolvedTargetCompiler`. An application enum wrapping it does not confer closed support. Consumer compiler work is also restricted to `BindingPolls`. |

[Closed Consumer support](../docs/spec/binding-spi.md#closed-consumer-compiler-support)
exists to substantiate bounded callback/storage/destruction behavior, not to
interpret a protocol. The fixed owned resolved-target primitive may support
several clients; its closure alone does not prove protocol-specific coupling.
It does refute unrestricted external compiler openness. Supporting a native
compiler outside that primitive needs separate reviewed support or a justified
generic admission change, even for the same WoT operation.

### Information preservation: PR #204

Let `E(d)` be a responsible downstream component's complete lawful input after
trusted upstream admission of document `d`, including its selected candidate
and fixed registration. A necessary condition is: if two admitted inputs have
identical `E`, every applicability or execution distinction that component must
make must be the same. Otherwise an upstream trusted check must separate or
reject them before reaching that boundary. This is sufficient information for
the support envelope, not a requirement to copy the entire TD.

PR #204's [0089 diagnostic](https://github.com/yushun1990/clinkz-wot/blob/ad703fb2468b00b44ebbf509ff316e1f66ec88a4/workspace/0089-http-producer-entry-readiness.md)
supplies a direct counterexample. `make_property_read_plan` retains target,
media and subprotocol; `LogicalInteractionPlan::try_property_read` initializes
`additional_form_metadata` to unknown and has no effective-security field.
`BindingCompilerInput` adds only candidate and role. Default versus explicit
method, response override, coding, present-empty scopes, Form/inherited security
and an additional advertised read property collapse to equal plans in the
documented eight Basic-valid mutations. Some distinctions can be equivalent
for a given envelope (default versus explicit GET); others demand different
eligibility or coverage decisions. Equality across the latter is the defect.

The Producer leaf also returns the first readable Form without proving whole-draft
coverage. The reported public mock path prepares and dispatches for each mutation.
The author and independent reviewer both executed that discriminator; this
assessment inspected its recipe, reports and production source rather than
rerunning it. Source equality already resolves the architectural question:
no deterministic downstream compiler with identical lawful input can recover
the erased distinctions. Rejecting the valid baseline along with every mutation
avoids unsafe acceptance but fails useful support.

A paid metadata-presence scalar can justify rejecting all extended Forms in a
narrow supported envelope. It cannot support interpretation of their different
contents. Future positive support requires sufficient resolved facts or bounded
preserved data; passing a raw TD or repeating TD defaults in the binding would
violate the existing boundary. Effective-security eligibility and advertised-route
coverage remain separate obligations; setting metadata to known-absent cannot
prove them. A trusted narrow eligibility check may suffice without a general
security/extension carrier. Topic 0089 owns the correction/admission choice.

This falsifies information preservation in the Producer implementation, not the
possibility of a protocol-independent boundary. It neither demonstrates a
required protocol-specific shared branch nor identifies a lifecycle defect.
Completed Consumer evidence concerns a different path and role.

### Compiled definitions, instance context and runtime ownership

[0081](0081-reusable-plan-and-thing-identity.md) remains the owner of program
reuse. For this assessment its three categories reveal different dependencies:

| Category | What must remain distinguishable | Consequence |
| --- | --- | --- |
| Reusable compiled definition | Immutable operation/schema/lowering data under complete compiler and semantic dependencies. | Reuse is valid only where equivalence is established; a binding may execute many definition values without a universal program format. |
| Instance context | Thing/target identity, resolved endpoint and parameters, authorization/source context and binding configuration. | Equal operation names or compatibility digests do not make these interchangeable. Credential material remains outside immutable compiler artifacts. |
| Runtime ownership | Publication generation, leases, calls, route/session ownership, reservations, cancellation and final cleanup. | Sharing immutable bytes or a transport does not merge independent authority or cleanup obligations. |

Current logical plans include Thing/plan/target context, and compiler inputs
allow dependence on all of it. Envelopes additionally qualify payload use by
generation, binding, configuration and role. The current
`ResolvedTargetArtifact` is an owned target, not evidence of a large reusable
instruction program. Thus source independence from the TD does not establish
cross-instance reuse; neither does per-generation ownership prove every
immutable byte must be compiled and copied per instance. Safe reuse could retain
distinct envelopes and funded owners, but needs its own dependency and lifetime
contract. Protocol independence requires sufficient context and private native
state, not a chosen physical layout or mandatory deduplication. No redesign is
inferred from #204's information loss.

### Producer lifecycle: challenge the mechanism

The minimum semantic obligations are a truthful frozen advertised contract,
owned and funded external work, no accepted application request before local
publication, dispatch under the exact live generation, closure of new acceptance
before drain, and retained cleanup ownership through failure and late completion.
Their current encoding is more specific:

| Current encoding | Justification and coupling question |
| --- | --- |
| Separate prepare, readiness, activate and committed-closed calls/guards | Preserves stage-specific ownership and recovery. These are mandatory public transitions, not four universally necessary native handshakes. An immediately ready protocol can complete transitions locally; a shared session can live behind route state. Neither needs a new socket per stage or Thing. Is the separate public call/state cost protecting an independently needed failure boundary? |
| All advertised routes commit before publication | Truthful immutable publication is semantic; requiring every advertised route to be available is a conservative v1 product policy, explicitly identified in Planning. It is not a consequence of protocol independence. A binding cannot silently remove a failing Form. |
| Temporary permit and one route accept cursor/waker | Protects dispatch/closure authority. Native demultiplexing can remain private, but fairness, independent route cancellation and bounded queues must survive that mapping. One cursor does not prove a private multiplexed implementation fits. |
| Same Host carrier across stages; explicit abort/shutdown settlement | Prevents lost/replaced ownership. It need not impose matching physical session lifetime. Final shared infrastructure release, pending calls and residual effects still need truthful owners under the applicable Producer contract. |

To establish substitutability, map native states/transitions to these semantic
obligations, then show each required SPI transition preserves ownership and
allows progress. Native transitions may refine several local stages or perform
no new native action. Check that readiness does not require a request which the
closed gate forbids, that native callbacks never dispatch, and that cancelling
one route does not kill sibling ownership. An immediate `Ready` or no-op commit
is legal only when its actual readiness/cleanup promises are true.

The strongest challenge to retaining this lifecycle is that stage-specific calls,
guards and cancellation successors can make authors reproduce neutral transition
machinery even when their native model has only one installation action. The
strongest defense is that different recovery owners remain necessary even when
successful native phases collapse. Current signatures and the corrected Host
carrier establish that defense's ownership benefit; they do not prove that every
separate callback is necessary or that emulation is cost-free. Repetition alone
does not demonstrate a defect. A legal semantic mapping that requires false
readiness, unowned effects, hidden dispatch, duplicate normative decisions or
unbounded retention would. Compare a narrower phase/helper correction with
retaining the current SPI before considering a new execution-family runtime.
Long-lived subscription sharing remains with 0082 and its separate capability
entry; it cannot falsify this fixed one-shot scope by changing the operation.

### Falsifiable correction and acceptance criteria

| Finding or unresolved claim | What would close or falsify it |
| --- | --- |
| Producer information and coverage defect | Valid default inputs remain useful; every mutation that changes eligibility/coverage is distinguished or rejected by trusted admission before prepare/handler/interaction effects. Preserve unknown versus absent, effective security and the whole-draft envelope. Charge projection, retained facts and rollback; zero/short budget and cancellation cannot bypass the barrier. Equivalent declarations may share output only with a justified equivalence. |
| Openness of a claimed installation path | Name role, profile, complete public owner, admitted compiler/artifact/configuration and successful runtime entry. A generic component or application assertion is insufficient. Claim only the existing closed Consumer primitive unless separately reviewed admission establishes more. |
| Unnecessary lifecycle coupling | Exhibit a native state/ownership mapping satisfying the semantic obligations, then identify the first mandatory public transition that prevents it and why immediate/private transitions cannot solve it honestly. If it maps with bounded ownership and progress, that specific incompatibility claim is falsified. |
| Proposed generic correction | Name exact affected authority/requirements, compare trusted eligibility rejection, additional sufficient facts and narrower lifecycle adaptation as applicable, and identify previously accepted evidence intersected by the change. Independent review must accept the contract/amendment and ADR-0013 admission before production edits. A protocol-name branch, TD rescan or caller promise does not close a neutral contract defect. |

No new executable discriminator is needed to establish #204's indistinguishability
or the closed Consumer type check. Where a particular reachability, state or
cleanup claim cannot be settled by signatures and source, use one focused public
compiler/lifecycle discriminator with named success/failure observations. A new
wire protocol implementation is justified only by that unresolved claim; adding
protocol examples is not the architectural proof method.

Structural separation and contract coverage can establish a conditional design
argument. Implementation-level semantic projection, callback behavior, phase
conformance, SDK boundedness and cleanup need their own scoped executable evidence.
No finite set of bindings, nor this argument, guarantees that arbitrary future
protocols supply the needed WoT mapping, resource/progress/security behavior or
unrepresented extension facts. Fixed current projections also cannot establish
information sufficiency for unknown future protocol vocabularies. Existing
empirical release requirements in the Binding SPI remain applicable in addition
to this conditional assessment.

Independent reassessment should attack the nonempty-envelope condition, the six
obligations, the exact role-specific paths, #204's information argument and the
lifecycle defense/counterargument. Assess the exact diff against current
authority and source. Record findings in PR comments. That assessment changed
workspace reasoning only. The decision refinement below now supplies the bounded
specification projection; implementation admission remains separate.

## Decision refinement: bounded Property Read contract

### Selected direction and scope

This decision reconstructs fetched master
`f324947bfa0b89eacd4d745129523ff1cfa4f1d0`, after PR #205. The assessment above
is reasoning history, not architectural closure. Select a narrow Producer
semantic-entry repair, retain closed Consumer compiler support for its actual
scope, retain the route lifecycle and execution authority, and preserve future
definition/context separation without implementing it.

These are conformance and claim-boundary refinements of existing requirements,
not a new lifecycle or compiler-admission architecture. Their normative owners
are [Planning](../docs/spec/planning.md#producer-semantic-entry-correction-boundary)
and [Binding SPI](../docs/spec/binding-spi.md#conditional-independence-and-admitted-extension-scope),
with navigation in architecture 40. No invariant reversal requiring another ADR,
parallel investigation, new gate or package graph is selected. Independent review
must accept this interpretation before it guides the successor admission.

Instantiate four scopes, rather than quantify over every visible WoT API.
For each protocol, declare its native target language and configuration family
from native semantics before assessment. The envelope contains all documents
within the row using that bounded target language, not only successful fixtures:

| Semantic scope | Input/configuration envelope | Implementation/evidence boundary |
| --- | --- | --- |
| Consumer Property Read, Host | One finalized Consumer registration; a Basic-valid borrowed Thing with a present `id`, at least one readable coordinate, [nonempty property names](../docs/spec/planning.md#logical-interaction-plan) for every retained readable coordinate, one effective NoSec definition per retained coordinate, default JSON Forms without additional metadata, literal targets of at most the configured `1..=64` UTF-8 bytes, no call URI variables, and sufficient admitted capacity. | The restricted target-copy primitive plus an external client. The aggregate may contain multiple properties/Forms; Runtime evidence covers one consumed generation and one active call. |
| Consumer Property Read, application-static | The same semantic envelope under its actual typed/manual representation and supported resource contract. | Actual compiler type capture and resident owners; portable compilation is not board or protocol qualification. |
| Producer Property Read, Host | One Basic-valid draft with a present `id`, advertising exactly one read route, exact handler coverage, literal target, default JSON, one effective NoSec definition, no additional Form metadata/scopes/URI-variable requirements, and sufficient `StrictV1` capacity. | This is the correction target, **not** the current entry guarantee. The compiler/server pair is open; semantic admission is defective. |
| Producer Property Read, application-static | The same one-route semantic envelope with typed state and its actual `StrictV1` costs. | The same missing entry guarantee; application-owned compiler enums remain possible. |

Fix actual configuration, limits, concurrency, deadline and resource/failure
policy before evaluating a mapping. The envelopes contain families of names,
targets, payload values and TD representations, not a post-hoc singleton or one
selected protocol. Explicit read and a defaulted read-only property may be
equivalent; default read/write is outside the one-read Producer envelope.
Consumer unrestricted/mock support remains evidence for its original finite
authors, not arbitrary Form semantics. Unsupported declarations cannot be
interpreted as defaults.

The observable contract is selected Property Read, opaque application payload
preservation and current success/result-origin checks, exact live identity,
local publication, first-cause cancellation, late-result handling and truthful
terminal cleanup. Core's seal is not general JSON/schema validation or proof of
peer authenticity. NoSec adds no credential/provider selection promise. Profiles
preserve semantic outcomes, not identical physical costs. Native control traffic
is distinct from accepting a WoT application call.

Legacy `ConsumedThingHandle` still selects/clones Forms at interaction time;
legacy `ServerBinding::serve` receives `Dispatch`. These are actual remaining
paths, not target-SPI evidence. Their writes, actions, subscriptions and
collections are neither silently certified nor removed here. The v5.1 deferred
domains and staged-removal roadmap retain authority. A public method or enum
variant alone establishes no compiled target capability.

### Conditional construction and its limits

The intended protocol class consists of native one-shot reads whose destination
and fixed request meaning are determined by a literal target and immutable
registration policy, whose values map to this payload/result contract, and
whose native state and local retirement are bounded under the chosen policy.
The native operation must permit mediated application acceptance and progress
under a fair available environment. These are native semantic properties,
independent of the current guards or compiler. They exclude per-Form instructions
absent from the declared envelope; they do not assume an SPI implementation.

For Consumer, the installed primitive copies the target exactly. An external
client can derive native syntax into its call/slot state before I/O, retaining
the supplied selected identity and deadline. This is protocol adaptation, not
TD interpretation or form reselection. Core need not understand that address
language or gain a protocol-specific compiler branch. Producer can compile its
route and canonical reservation through the open compiler after trusted
admission discharges omitted distinctions. Both keep correlation, framing,
native retries and physical session sharing private.

This source/contract construction establishes information and authority
placement, not an implemented adapter for every such protocol. Work, backing,
failure, readiness and retirement must fit the chosen representation. A feasible
mapping that cannot traverse a mandatory boundary remains a counterexample;
it cannot be excluded by redefining the class after failure. No guarantee for
arbitrary foreign compiler callbacks or every bounded native protocol is
established. A need for preserved Form contents or a different pre-I/O lowering
may intersect closed Consumer support and require a reviewed generic correction.

### Defects, restrictions and remedies

The Producer failure is stronger than missing maturity evidence. Its lawful
compiler input is plan/candidate/role; fixed registration cannot recover erased
method/response/security or whole-draft distinctions. Public expose roots freeze
the first leaf plan and check its selected handler without a whole-draft
barrier. Also, `property_read_plan` treats absent `op` as readable, while TD's
`default_property_operations` distinguishes write-only from read-only and
read/write. This is the same semantic-entry defect category, not a reason to
duplicate defaulting rules. The evidence is [Producer source](../planning/src/property_read.rs),
[orchestration](../servient/src/property_read.rs) and
[TD authority](../td/src/td_defaults.rs). #204 adds separately reported execution
of its eight mutations; no new protocol experiment is claimed here.

| Remedy | Decision |
| --- | --- |
| Paid whole-draft preflight and narrow rejection | Selected. Prove the complete one-route envelope before freeze/prepare, use TD semantics, preserve truthful metadata facts and charge continuation/rollback. Details and closure belong to Planning. |
| General security/schema/extension fields in every logical plan | Unnecessary for this envelope. Discharge eligibility upstream and retain only needed facts. More fields alone leave coverage and provenance unresolved. |
| Raw TD access, an HTTP branch, or a caller eligibility flag | Reject. These bypass semantic ownership or replace the real entry barrier with an assertion. |
| Metadata-presence bit alone | Insufficient for effective security, operation defaults and coverage. Existing `Option<bool>` suffices for the narrower fact when populated truthfully. |
| Multiple routes or silently reduced effective TD | Reject for this repair. Unsupported advertised operations must fail admission; broader positive support is another tranche. |
| Open Consumer compiler admission | Not selected. Closed support deliberately substantiates callback/storage/destruction behavior. It limits compiler authorability but proves no protocol-specific shared branch is required. Do not remove that proof or call it universal openness. |
| Replace four Producer phases with one universal kernel | Not justified. Native setup may be immediate/private; partial failure still needs an identified recovery owner. No lawful native-state counterexample defeats that refinement. Repeated wrappers alone are not such evidence. |
| Another protocol as primary architectural proof | Not selected. It tests one adapter, not the quantified property. A focused discriminator needs a named unresolved structural claim; the independent empirical release requirement remains. |

### Definition/context compatibility

Topic [0081](0081-reusable-plan-and-thing-identity.md) retains the reuse decision.
Do not turn transaction-local eligibility into durable program identity or put
credentials, route/session owners, handlers or cleanup into immutable artifacts.
Bind retained eligibility to the current draft/context and registration. A future
definition must declare static dependencies and recheck remaining instance
operands. Current Thing/plan/binding/generation envelopes still authorize each
execution; they are not content-equivalence keys. One binding can execute many
artifact values. Sharing a definition or session cannot merge publication or
last-owner cleanup. This permits a later split but establishes no reuse,
persistence, scalability benefit, cache format or runtime abstraction now.

### Authority, correction scope and closure

| Owner / requirements | Correction or reaffirmation |
| --- | --- |
| TD under ADR-0021; WP-100 | Reuse paid Basic/default/security/URI semantics. Do not mandate the Consumer aggregate's physical representation for Producer. |
| Planning: `PLAN-COST-003`, `PLAN-ARTIFACT-001`, `FORM-COVERAGE-001`; WP-200 | Whole-draft support/coverage preflight and truthful facts before freeze/prepare; no mandatory new public carrier. |
| Core/Binding: `BIND-REG-001`, `BIND-ROUTE-001`, `BIND-IO-001`, `BIND-OUT-001`; WP-300 | Actual role-specific installation limits; preserve artifact identity, result sealing, private adaptation and stage-specific recovery. |
| Servient: `ADMIT-TXN-001`, `LIFE-EXPOSE-001..003`; WP-400 | Both expose roots enforce the barrier and retain rollback. No new dispatch/publication authority or state machine. |
| Foundation/API/resource/performance projections | Reconcile paid work, actual layouts, continuations and rollback in correction admission. New schema/API rows are needed only if the selected source repair changes them. |
| WP-600 and #204 | Correct public Producer entry before real HTTP Producer admission. Consumer resource migration does not authorize a Producer policy change. |

Decision acceptance requires a fresh independent context to reconstruct the
source/contracts and challenge the four scopes, native class, preflight choice,
Consumer restriction and lifecycle refinement. This PR supplies the specification
decision boundary; it does not close the source defect. No package admission,
accepted gate or milestone changes in this decision.

The successor closes only when its reviewed ADR-0013 scope supplies both public
Host/static paths with useful baseline acceptance; pre-effect distinction or
rejection for #204's eligibility/coverage mutations, defaulted write-only and
read/write, and extra advertised operations; explicit-empty/unknown handling;
and paid zero/short-budget, cancellation, failure and rollback evidence. Recheck
actual source/identity/layout impact and affected Producer compiler/route/Servient
regressions. Existing mock route/permit/carrier traces remain original-scope
evidence, not semantic-admission certification. Completed Consumer/HTTP
qualification is disjoint unless changed shared carriers invalidate measured
costs or behavior; then revalidate the exact affected cases.

No finding here demands a general compiler-support issuer, lifecycle replacement
or program/context runtime before that correction. A later counterexample must
name the lost fact, rejected complete public installation or mandatory ownership
transition and its lawful native mapping. Topic 0056 and normal change control
remain the reopening path. General SPI substitutability, empirical protocol-shape
neutrality and production/release qualification remain separate unclosed claims.
Findings, checks and unresolved issues belong in PR comments, not a repository
task-state record.
