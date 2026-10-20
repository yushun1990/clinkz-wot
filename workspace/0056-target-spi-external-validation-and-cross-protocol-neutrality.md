# 0056 Target SPI External Validation and Cross-Protocol Neutrality

Status: DISCUSSING

The completed [integrated Review 08](../docs/reviews/review-08-integrated-architecture-reassessment.md)
reassesses this topic together with reusable programs, capability/environment
boundaries and infrastructure ownership at `700cfdb1`. It recommends separating
those responsibilities before freezing a general SPI. Its proposed projections
are marked in the existing architecture/specification owners; they neither
activate the earlier broad replacement nor alter accepted Producer admission.
This topic remains DISCUSSING pending the integrated direction review.

Kind: owner-raised architecture investigation

Priority: HIGH

Target: conditional protocol independence for already supported WoT semantic capabilities and the evidence boundary of the public Binding/Servient SPI

The [semantic-boundary investigation](#semantic-boundary-investigation) below
challenges the replacement proposal's unresolved input assumption with standards
and disposable HTTP/MQTT lowering evidence. It is the current recommendation
for that question, pending independent architectural review and Owner approval.
It adopts no replacement SPI or architecture gate.
The [responsibility reassessment](#responsibility-reassessment-without-application-allowlists)
supersedes the experiment's application-supplied annotation-role policy. Ordinary
applications select supported profiles and deployment/security policy; they do
not classify vocabulary terms or maintain domain-class allowlists.
The [standard-profile counterexample](#standard-thingprofile-advertisement-activation-and-coverage)
distinguishes `Thing.profile` claims, trusted interpretation, native support and
Engine selection. Annotation interoperability is demonstrated for class
assertions only; unfamiliar annotation predicates still cause refusal.

The original investigation, decision and migration below record the pre-probe
feedback-order decision. That migrated history is preserved. The
[reassessment](#reassessment-conditional-protocol-independence) reopens the assurance
question in this topic, consolidating PR #205's former 0090 proposal with 0042.
The [replacement decision](#decision-refinement-protocol-independent-compilation-and-execution)
below reassesses the complete architecture under the Owner's explicit pre-v1
breaking-change authorization. It supersedes the earlier recommendation to retain
closed Consumer lowering and the public route phase split. It is a proposal for
independent review, not implementation admission or an activated specification
migration. Existing source contracts remain effective until deliberately replaced.

## Producer admission subset

The [WP-600 Host Producer candidate](../docs/work-packages/WP-600-http-producer-property-read.md)
migrates a bounded subset of this investigation into the existing Planning,
Binding, runtime/architecture and ADR owners for independent ADR-0013 admission:
resolved Producer facts, complete advertised coverage, one instance-owned entry,
a retained Host endpoint driver and real HTTP ownership evidence. The admission
PR resolves those design questions only for its one-route Host scope; it does
not treat this DISCUSSING topic or #206/#211 as implementation authority.
General Consumer compiler openness, shared/multi-route sessions, profile
interpretation, reuse and cross-protocol claims remain open and are outside
this tranche. The old Producer counterexamples are executable prechecks,
not an assertion that the proposed production replacement already works.

## Scope and authority

The following scope and probe history describe the original investigation. The
current reassessment and replacement decision below use the Owner's broader
authorization; the original restriction on what was being investigated is not a
restriction on this proposal.

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

Historical assessment from PR #205. Its source findings and conditional-property
formulation remain useful; its recommendation to preserve the macro architecture
is superseded by the replacement decision below.

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
workspace reasoning only. The replacement decision below changes the proposed
architecture direction; authority migration and implementation remain separate.

## Decision refinement: protocol-independent compilation and execution

### Verdict and review boundary

**Replace the current compilation/admission boundary and simplify the execution
SPI. The present target is not a defensible protocol-independent extension
architecture for the stated goal.** Preserve correct WoT semantics and ownership
properties where useful; do not preserve `ResolvedTargetCompiler`,
`LogicalInteractionPlan`, exact compiler-type admission, or the current runtime
decomposition as architectural requirements.

This reassessment reconstructs master
`f324947bfa0b89eacd4d745129523ff1cfa4f1d0` and the previous PR #206 proposal at
`c611bbcae4293a595b91c5e360d3a8c538c5ade7`. The Owner explicitly authorizes breaking
changes and a clean-slate design in this pre-v1 project. Compatibility, completed
proofs, implementation cost already incurred, and accepted decisions carry no
presumption of architectural correctness. The earlier proposal's narrow Producer
preflight could repair a safety defect; retaining closed Consumer lowering and
waiting for a future protocol counterexample would still fail the requested goal.

Select an **open Binding-owned lowering boundary, small immutable execution
entries, and engine-owned transactions with Binding-owned native drivers**.
Rewrite the intersecting Core/Planning/Servient contracts rather than extending
the closed helper one exception at a time. This is a cross-domain architecture
replacement proposal, not a clarification of existing admission.

PR #206 and this existing topic remain the review boundary. The Planning and
Binding specification additions describe the proposed migration, explicitly
without activating it. No production change, new topic, gate transition, package
admission, roadmap closure, approval, or merge follows from this decision.
Independent acceptance must challenge the selected architecture itself.

### First principles and the required extension property

Fix a supported WoT semantic capability, role, security/result contract, execution
profile, and declared resource/failure policy. For each protocol with a native
mapping that satisfies those observable obligations, a third party must be able
to implement and install the mapping through public generic contracts and normal
application composition. Core, Planning, and Servient must need no protocol name,
vocabulary case, permitted compiler type, or protocol-specific lifecycle branch.
A Binding must complete healthy supported interactions, not merely reject them.

Native feasibility means that the protocol can express the semantic operation,
mediate application dispatch, and retain bounded accountable work under the
selected profile. It does not mean that it already fits today's Rust SPI. A new
WoT operation or new authorization semantics can require an engine capability
change. A different wire method, option, address representation, response code,
or protocol setup sequence for an existing capability cannot be reclassified as
new WoT semantics merely to excuse a closed extension boundary.

This distinction follows the standards' separation of abstract operations from
protocol adaptation. [TD 1.1 sections 7.2 and 8.3](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#protocol-bindings)
allow binding vocabulary in Forms; its HTTP section includes explicit method
selection. The [Binding Templates Note](https://www.w3.org/TR/2024/NOTE-wot-binding-templates-20240528/#protocol-binding-templates)
illustrates different protocol mappings for the same operation. These sources
support an information/extension requirement, not this proposal's Rust design.

The semantic requirements are deterministic selection; correct operation,
security and result meaning; truthful advertised capability; local publication
and dispatch authority; live identity and attribution; overload behavior; and
accountable cancellation/retirement. Neither WoT nor those requirements imply a
universal persisted logical-plan structure, a Core-owned compiler implementation,
four native setup phases, or the same physical runtime on Host and embedded.

### Fundamental findings

| Finding | Source evidence and architectural consequence |
| --- | --- |
| Consumer compiler installation is closed by implementation identity. | [`static_consumer_support`](../core/src/binding.rs) downcasts to the exact `ResolvedTargetCompiler`. [`HostCompilerStorage` and `consumer_support`](../core/src/binding_compiler.rs) recognize only the private Consumer adapter; generic erasure of even the same compiler is ineligible. [`ConsumerPropertyReadRegistration`](../planning/src/consumer_property_read.rs) is sealed around that capture. A conforming independently authored lowering cannot reach the real Consumer path. This is a structural failure of the required extension property, regardless of whether the whitelist contains protocol names. |
| Necessary information is erased before the Binding can compile it. | [`LogicalInteractionPlan`](../core/src/plan.rs) retains target/media/subprotocol and a metadata-presence flag; [`BindingCompilerInput`](../core/src/binding_compiler.rs) adds candidate/role. Effective security, response details and extension values are absent. Even [`ValidatedPropertyReadForm`](../td/src/validated.rs) exposes metadata presence without its extension contents. Merely unsealing the compiler does not restore them. |
| Producer semantic admission is incorrect. | [`property_read_plan`](../planning/src/property_read.rs) returns the first read-looking Form and treats absent `op` as read support. [`TD defaulting`](../td/src/td_defaults.rs) distinguishes write-only, read-only and read/write. Both [`Servient expose paths`](../servient/src/property_read.rs) check the selected handler, not complete advertised coverage. #204's eight Basic-valid mutations collapse to equal plans and still reach preparation/dispatch. |
| A representation proof has become the public extension policy. | [`ConsumerCompilerSupport`](../core/src/binding_compiler.rs) derives callback costs and storage from one known implementation and fixed 64-byte target payload. Core configuration also carries the default-JSON restriction. Known costs explain the original construction proof; they do not justify making every new lowering depend on a Core release. The current HTTP client must parse/derive its target in [`invoke`](../protocol-bindings/protocols/http/src/lib.rs) instead of using an independently compiled native artifact. |
| Runtime mechanisms have been promoted to semantic requirements. | [`PollServerBinding`](../core/src/binding.rs), the Host route guards, and [`HostRouteState`](../servient/src/property_read.rs) expose prepare/readiness/activate/commit plus phase-specific cancellation, transfer and joining. They protect real ownership, but each exposed phase multiplies recovery cases. No semantic requirement establishes that four public phase families are necessary. The absence of a native impossibility counterexample does not justify their cost. |
| Immutable content, instance data and live authority are coupled. | Compiler input includes Thing/plan/candidate generations and concrete targets; envelopes and per-generation output repeat identity and storage ownership. [0081](0081-reusable-plan-and-thing-identity.md) explains why these cannot be treated as content-equivalence keys. This is not proof that a cache would help, but a rewrite should stop requiring live authority inside reusable immutable content. |
| The product still has competing execution truths. | [`ConsumedThingHandle`](../servient/src/handle.rs) selects/clones Forms at interaction time; legacy server dispatch hands authority to bindings. [`Servient's manifest`](../servient/Cargo.toml) also contains the concrete optional Zenoh edge. These paths cannot establish the new extension property or remain implicit fallbacks for a migrated capability. |

The information defect is not just a missing struct field. If two admitted
sources have identical complete lawful compiler inputs but require different
native behavior, no downstream compiler can recover the distinction. A trusted
upstream rejection can make a narrow implementation safe; repeatedly rejecting
all newly needed vocabulary is not a protocol-independent architecture. Target
length and inline storage may be Binding/profile limits, never universal limits
on a semantic Property Read capability.

The existing finite tests remain useful: their passing whitelist rejections
establish actual behavior, not desirable extensibility. PR #204's diagnostic
uses a mock registration and demonstrates lost input/coverage, not HTTP wire
conformance or a demonstrated lifecycle impossibility.

### Selected target: resolve, lower, install, execute

```text
immutable source + policy + linked registrations
        |
TD semantic resolution + bounded source/context views
        |
Planning: enumerate candidates -> Binding-owned pure lowering
        |                         (opaque native artifact)
        v
immutable execution entries + generic selection/coverage indexes
        |
Servient: reserve/build/publish, dispatch, leases and retirement
        |
Binding-owned client operations / endpoint and shared-session drivers
        |
native protocol I/O
```

**Resolution and compilation.** Keep one implementation of TD validity,
operation defaulting, security inheritance and generic URI semantics. Give the
Binding compiler a bounded borrowed view of the selected interaction and all
applicable source context: effective operation and role, resolved target or
validated template, schemas/media/response/security requirements, and preserved
Form/affordance/root extension data with namespace/context identity. Preserve
absent, explicit-empty and explicit values where semantically distinct.

This is compile-time access, not runtime TD retention or authority to reselect
another Form. A generic bounded value/context view must carry future vocabulary
without adding a Core field per protocol. TD owns interpretation of shared TD
terms; the Binding owns its protocol vocabulary and protocol-specific defaults,
even when those defaults are documented by W3C. Unknown relevant instructions
must produce explicit unsupported/error outcomes. Namespace interpretation uses
bounded local or explicitly supplied immutable context; no implicit remote
context fetching is needed. Do not mandate a second complete normalized TD tree.

Use public role-specific compiler/executor contracts paired by one registration,
with associated native artifact/state types. Consumer-only registration requires
no dummy server, and Producer-only requires no client. The compiler is pure:
no socket, handler, credential acquisition, reactor task or native subscription.
It returns supported artifact, candidate rejection, or failure. Planning performs
generic deterministic ranking, supported security/codec selection and coverage;
it contains no protocol-specific rejection logic. An unsupported alternative
Form does not automatically invalidate a Consumer's supported alternative.

Every advertised Producer operation/Form must have matching installed routes
and handler/security coverage before publication. Initially require complete
coverage of the submitted advertised TD. A future API may explicitly construct
a reduced description before exposure, but silently publishing an unchanged TD
while serving its first route is never acceptable. Pure compilation may proceed
privately before the final coverage check; that check must precede native route
effects and publication. A separate whole-draft preflight may optimize failure
cost, but is not a required architecture layer.

**Artifacts and execution entries.** Remove the requirement to materialize and
retain a `LogicalInteractionPlan` for every effective Form/operation. Compile
directly from the resolved view into an immutable entry containing only engine-used semantic
selection/validation/dispatch data and the Binding's opaque artifact. Share
neutral data where useful. Native parsed targets, wire instructions and response
mapping belong in the artifact; they need not be reparsed at every invocation.
Artifacts contain no live protocol resources, credentials or cleanup jobs.

Use one unpublished build owner and one published instance-generation owner for
the execution table. Publication transfers ownership; closure stops new leases,
drains existing work and reclaims the table. Frozen describes completed private
construction, not a required extra runtime manager. Separate plan-set generations
are needed only if plans can be replaced independently of the instance; do not
require that machinery for an immutable per-instance table. Call/route slots
still need their own stale-use protection where they are reused. Select eager
compilation initially; lazy compilation, negative caches and persistence are
optional optimizations with separate evidence, not fundamental WoT requirements.

Keep publication/generation/registration authority in an engine-owned record and
its nonforgeable handles. Bind artifact use to the exact installed registration
and role through typed composition or its generic erased equivalent. Keep any
instance parameters distinct from immutable definition content where they truly
vary. This allows one Binding to execute many artifacts and one shared native
session to serve many Things. It mandates no universal program, bytecode,
serialization format, content hash, cache, persistence or deduplication service.
Existing numeric digests are not certificates of source behavior or authority.

Issue local instance/slot identity independently of the optional source TD `id`.
Retain that source identity when present for its actual semantic/provenance use;
do not require it merely to construct an ownership record. Treat affordance names
as source keys, including empty keys accepted by the TD semantic kernel, rather
than imposing `LogicalInteractionPlan`'s nonempty-name constructor restriction.
Any narrower product policy needs its own reason. These old prerequisites are
implementation restrictions to remove in the rewrite, not safety invariants.

**Open admission with real bounds.** Delete exact-type/private-adapter eligibility
and the source-known `ConsumerCompilerSupport` descriptor as installation
requirements. Derive physical slot Layouts, alignment and wrapper/erasure overhead
from actual generic types. Supply budgeted compilation/storage access and an
owned compile job whose partial output can always be rolled back. Bound initial
construction, progress, allocation, failure and disposal, not just `step` after
an unbudgeted `bounds` callback. Returned costs alone cannot prepay that callback.

A generic Host adapter owns one complete typed job through Pending, failure,
completion and release; it must not consume/rebox state on every Pending or
allocate an unretainable output after completion. Static composition uses inline
slots, application-owned enums or equivalent typed storage without mandatory
atomics. Capacity is acquired before controlled growth and retained until actual
release. Preallocating a whole job is one simple mechanism; its correctness must
be established generically, not by recognizing a compiler name.

Finite callback/destructor behavior is an explicit conformance obligation of a
trusted linked Binding, just as for its runtime callbacks and SDK. An application
can qualify external source and execute the public conformance suite without a
Core source change or centrally issued support token. Engine APIs enforce actual
controlled storage, handle provenance and accounting; Rust types cannot prove
that arbitrary native code terminates or reports honestly. This does not weaken
the behavioral bound or certify a self-reported cost. A stricter embedded product
must qualify its actual implementation/input maxima; hostile in-process code
requires isolation, not a compiler whitelist.

Do not require all candidate bounds before the first pure compile job. Reserve
and compile candidates sequentially under aggregate caps and funded rollback;
complete selection/coverage and all promised capacity before publication/native
effects. Preserve a complete pre-reservation mode where a product actually
promises worst-case success. This removes a global barrier and duplicate census
machinery without removing admission or permitting unowned partial output.

**Execution and ownership.** Servient owns semantic transactions, selection,
security/handler dispatch, local publication, active leases, cancellation cause
and cleanup policy. Binding owns native I/O, framing, correlation, retries within
policy, authentication adaptation, buffers, multiplexing and shared sessions.
An endpoint is a logical WoT owner, not a mandatory socket/session.

Replace the mandatory public prepare/readiness/activate/commit families with one
owned endpoint installation driver: Installing -> LocallyReady -> Serving ->
Closing -> Closed. Servient retains one wrapper/lease throughout; native setup
states and partial recovery remain inside the Binding driver. Installation may
finish immediately. LocallyReady means local installation is ready to obey the
engine gate, not guaranteed peer reachability or completion of a user request.
An error/cancel at any point leaves the same owner available for close.

Servient's publication step opens the local dispatch gate after complete coverage
and resource checks. The Binding may receive bounded traffic/control messages
earlier, but cannot dispatch a handler before publication or after closure.
This promises atomic local dispatch visibility, not atomic remote advertisement
or rollback of already-visible native effects. Endpoint collision identity stays
Binding-derived and engine-checked; it does not require public native commit.

A registration/session driver may multiplex ingress across endpoints and return
bounded semantic events carrying engine route identity and an owned reply
opportunity. Servient validates them and dispatches. Client operations and replies
retain owned progress/cancel/retire state. There is no Binding-held handler or
registry capability. Host may use its reactor under an explicitly retained native
owner; embedded may poll manually. Do not force one route poller per physical
session or a universal executor. Close one route without killing siblings;
retire shared infrastructure only after its final user and callbacks are gone.

Cancellation fixes the first cause, closes new acceptance, and progresses the
same owned operation toward terminal retirement. A late result is validated and
released while its owner/lease remains live. If the ordinary driver is abandoned,
transfer the whole owner into a prebacked cleanup destination or retain an
explicit manual close handle. Report uncertain remote effects distinctly from
local retirement. Do not use Drop, a closed flag, a deadline, or a residual record
as proof that surviving tasks and buffers are gone. Generic owned progress may
serve multiple operations, but do not invent a universal semantic state machine
from Property Read alone; subscriptions retain distinct long-lived obligations.

**Deployment and module boundaries.** Keep concrete protocols/SDKs outside the
normal Core/Planning/Servient dependency graph; the application or umbrella
selects them. Keep startup composition as the simple default: open third-party
authoring requires neither dynamic code loading nor hot replacement. A bounded
registration set can contain multiple external implementations. Runtime
replacement, if ever needed, is a separate capability.

Separate execution contracts and data from dynamic TD admission/compilation so
a fixed Consumer or Producer need not link a whole dynamic runtime. Host can
compose both; firmware can use build-time preparation with the same semantic
compiler, or opt into on-device admission where justified. Binding-specific typed
definitions are sufficient for build-time output; installation still checks the
current instance/configuration and issues fresh live authority. No universal
artifact image or saved generation seal follows. Share semantic definitions and
rule bodies, not necessarily storage or scheduling implementations. A packaging
split must serve those dependencies; no mandatory new crate hierarchy is selected.

### Safety invariants versus replaceable machinery

| Essential guarantee retained | Machinery not required by that guarantee |
| --- | --- |
| Trusted TD meaning; sufficient information for each lowering; no hot-path source interpretation. | A fixed owned `LogicalInteractionPlan`, metadata bits instead of vocabulary, or a second normalized TD. |
| Deterministic supported-candidate selection and truthful advertised coverage. | One Core compiler for every Consumer, all-readable candidate success, first-Form exposure, or a one-route/default-JSON architecture envelope. |
| Controlled storage bounded before growth; paid work; overload and rollback ownership. | Core-known compiler bodies, a central source-admission list, identical Host/static bytes, or all bounds collected before pure compilation. |
| Complete local publication/closure and authorized dispatch. | Four public native setup phases and stage-specific carrier types. |
| Exact live registration/route/call provenance; stale results cannot execute. | Every immutable payload embedding live generation fields, independently versioned plan-set machinery without independent replacement, or mandatory cryptographic digests of every constructor setting. |
| A retained owner through late completion, cancellation and cleanup; shared last-user retirement. | One physical session per Thing, nonreplaceable stage succession as a public API, or a universal cleanup-transfer taxonomy. |
| Bounded cleanup with funded fallback; truthful remote uncertainty. | Universal recovery from Host process OOM or allocation-free destruction inside every Host SDK. |
| Portable semantics and predictable constrained ownership. | Dynamic TD admission on every firmware, one scheduler/allocator representation, or mandatory Arc/atomics. |

Use ADR-0023's ownership allocation consistently: engine-controlled storage and
promised handoffs are admitted; Binding policy bounds native input/state; the
application composes deployment resources. Inline native types held in engine
slots still count, and private backing is never an excuse for ownerless work.
Host process failure is outside recoverable in-process capacity promises.
Embedded fit requires actual RAM/stack/flash/backend evidence. Neither a numeric
quota nor a portable compile test provides that evidence.

Prefer synchronous bounded operations for explicitly capped small inputs when
one paid atomic step suffices. Keep incremental work for variable inputs and
required latency/cancellation bounds. Derive capability-specific limits from
real owners rather than requiring every future schema field in every product.
Do not weaken bounds by calling them estimates; distinguish enforced limits,
qualified finite maxima, measured costs and deployment assumptions explicitly.

### Alternatives under the same requirements

| Design | Assessment |
| --- | --- |
| Previous PR #206: narrow Producer preflight, closed Consumer compiler, existing public phases. | Can safely support its small literal-target class. Fails independent lowering for the same semantic capability and freezes known exclusions. Reject as the architecture target. |
| Remove downcasts and keep the rest. | Opens syntax, not a constructible contract. Missing vocabulary, generic erasure costs and unbounded callback/disposal gaps remain. Reject. |
| Open resolved-input compiler, retain all existing plan/lifecycle containers. | Can meet protocol independence after substantial information/storage repairs. Credible fallback if a simplified driver loses a required invariant, but carries redundant persistent projections and phase/recovery multiplication without a demonstrated benefit. |
| Open lowering, minimal execution entries, owned native drivers, thin engine transactions. | Selected clean-slate target. Separates semantics from protocol mechanism and safety from representation proofs; permits generic Host and static authoring. Requires a deliberate contract/runtime rewrite and new behavioral evidence. |
| Put TD selection/handlers in bindings, interpret TD on each call, or introduce a universal protocol VM/allocator. | Reject. The first choices duplicate semantics or lose authority/performance; the latter adds a language/runtime and authoring burden without a need. |
| Rewrite TD semantics and every Foundation primitive too. | Unnecessary. Existing default/security/URI kernels, time, linear reservations and useful tests can be retained when they satisfy the selected contracts. Reuse is judged by fit, not sunk cost. |

The selected design is smaller in obligatory public concepts, not a measured LOC
or throughput claim. External conformance carries a real qualification burden;
open trusted code cannot supply mechanical proof of every callback. Collapsing
public phases hides native failure detail inside the driver, so bounded/redacted
phase diagnostics and adversarial partial-installation tests are needed. These
are explicit tradeoffs, not reasons to retain a known closed extension boundary.

### Proposed rewrite boundary and independent acceptance

The replacement boundary spans the supported Property Read compilation and both
Consumer/Producer installation/execution roots. Do not implement another
protocol-specific compiler exception or a Producer-only preflight as closure of
this decision. Keep HTTP's native transport tests and useful semantic kernels;
replace their shared integration contract where necessary.

| Affected owners | Required migration before replacement production code |
| --- | --- |
| Architecture 10/20/30/40/50; ADR-0008/0009/0010/0012/0019/0021/0023 | Record deliberate supersession of fixed logical projection, closed compiler admission and public route phases; retain useful semantic/resource rationale. Use existing architecture change control, not another parallel investigation. |
| TD/WP-100; `DOC-RUNTIME-001`, `PLAN-COST-003` | Define bounded complete compile-time source/context views, shared semantic meaning and source-independent output. Reuse Basic/default/URI kernels; remove metadata-only information bottlenecks. |
| Planning/WP-200; `PLAN-ARTIFACT-001`, `PLAN-BOUND-001`, `FORM-COVERAGE-001` | Open candidate lowering, minimal entries, deterministic selection, coverage and sequential funded construction/rollback. Delete required per-coordinate logical-plan duplication and closed support dependence. |
| Core/Binding/WP-300; `BIND-REG-001`, `BIND-IO-001`, `BIND-ROUTE-001`, `BIND-OUT-001` | Generic paired compiler/executor registrations and actual-layout storage adapters; replace `ConsumerCompilerSupport` and exact-type checks. Remove `ResolvedTargetCompiler` from Core's required path; retire it or move any useful literal-target helper into optional Binding support. Replace stage-specific public route families with an owned endpoint driver. |
| Servient/WP-400; `ADMIT-TXN-001`, `STATE-EXPOSE-001`, `STATE-BIND-001`, `HANDLE-DROP-001`, `LIFE-EXPOSE-001..003` | Replace intersecting per-phase orchestration with admission/publication/dispatch/lease/retirement owners in Host and static representations. Preserve first-cause and late-value semantics; remove implicit legacy fallback for migrated capabilities. |
| Foundation/resource/API/state/performance projections; WP-600/700 | Replace representation-specific cost/generation/status contracts where the new owner model changes them. Requalify HTTP integration; preserve distinct Host/embedded and protocol-ingress evidence. Revise scoped work-package dependencies and release surface deliberately. |

Do not run two competing production dispatch/admission systems indefinitely.
Implement the replacement as coherent capability cuts after authority review,
then delete the displaced target implementation and its exclusive proof
machinery. A temporary development adapter must have a removal boundary and
cannot certify the replacement. Unmigrated pre-v1 legacy operations need an
explicit retire/defer/replace disposition; backward compatibility does not
require preserving them. Independent acceptance must invalidate or scope affected
old claims, not automatically carry accepted gates into a different design.
Unchanged semantic/HTTP transport regressions keep their original evidence value.

Before treating the target as constructible, require ordinary external packages
to author the compiler and execution sides without engine patches or private
tokens. Use at least two contrasting Property Read mappings: default/explicit
HTTP instructions and a non-URI-copy native artifact using per-Form vocabulary
and shared-session setup. Demonstrate distinct native artifacts/behavior for
distinct relevant metadata; valid alternative selection; Producer full coverage;
and cross-registration, stale-generation and wrong-role rejection. Execute the
same semantic contract through Host erasure and static composition, including an
application enum containing externally authored compilers.

Failure evidence must cover unpaid/short work, acquisition refusal, compiler
failure and abandonment, every partial installation boundary, cancellation racing
with publication/result delivery, late results, full cleanup destinations,
multiplexed sibling isolation and final-owner retirement. Measure actual carrier
backing, erasure/rollback overhead and retained release at those owners. One real
public Consumer-to-Producer protocol round trip is needed for useful product
integration; contrasting native adaptation evidence is needed for the extension
boundary. No finite test suite proves every conceivable protocol, and no passing
old gate supplies replacement acceptance. A later ObserveProperty proof must
exercise long-lived semantics rather than infer them from reads.

These are falsifiable completion criteria for the proposed rewrite, not a new
gate or permission to start it. Independent review should challenge whether the
resolved views expose enough context, external admission is honestly bounded,
one endpoint owner covers every failure/recovery case, and any retained public
concept is necessary. Findings and exact executed checks belong directly in
PR #206. Production implementation, authority activation and merge remain outside
this task.

## Semantic-boundary investigation

Status: DISCUSSING; recommendation for independent review and final Owner
approval, not an accepted replacement contract.

### Protocol-owned mapping reassessment

The same-protocol matching experiment was reverted by #213; its findings and
review corrections remain in #212, not a retained framework. The preferred
production boundary is protocol-owned native mapping/defaults/vendor handling,
with Engine-owned common WoT semantics, Form selection, security and lifecycle.
Earlier annotation refusal, shared interpreter/coverage and matching proposals
below are historical alternatives, not implementation requirements. TD 1.1
specifies neither a universal extension criticality marker nor vocabulary scoring.

The bounded Host implementation migrates the compilation seam first: TD resolves
supported local extension syntax once, generic scheme/Form dispatch supplies a
native compiler, and HTTP executes its own immutable artifact. Reusing the current
runtime records preserves tested identity/security/lease/cleanup responsibilities;
requiring them as native input would preserve the information-loss defect.
A simultaneous Producer endpoint/lifecycle rewrite is unnecessary for this slice.
The exact contracts are migrated to the existing TD, Planning, Binding and HTTP
owners. Producer coverage/collision evidence and the immediate production HTTP
Producer loop remain necessary. Independent architecture acceptance is still
required; the Rust examples do not prove arbitrary-protocol conformance.

### Answer and claim boundary

**A useful protocol-neutral semantic boundary is feasible and worth pursuing,
but neither today's logical plan nor a universally sufficient expanded graph
is that boundary.** The defensible design is an Engine-owned semantic service:
resolved WoT facts plus structured, context-resolved extension values and
explicit loss/feature diagnostics, accessed through bounded queries. Bindings own
the meaning of their protocol vocabulary. This selects responsibilities and
information obligations, not a universal Rust type, graph store or VM.

The executable evidence proves useful common-interface lowering for two
contrasting mappings. It also falsifies the stronger hypothesis that expanded
values plus shallow source-presence metadata preserve every future extension's
inputs. **The tested interface is not ready to replace production contracts.**
Prefer an explicit input profile first: extensions express ordering through
semantic lists and opaque JSON payload structure through JSON literals;
unsupported constructs are reported before execution. Ordinary JSON-LD arrays
are unordered, so the counterexample is not a semantic failure for such
vocabularies. A complete source overlay is a broader alternative whose need,
sufficiency and cost remain unverified. This restricts representational
features, not protocol names or approved compiler implementations.

There is no information-theoretic obstacle to preserving a finite input tree
and its resolved identities. There is a fundamental obstacle to deriving the
operational meaning of an arbitrary new vocabulary from its IRIs alone. A
generic resolver can carry `vendor:confirmMode`; only its interpreter can know
whether it changes framing, completion, authorization, or something the Engine
cannot implement. A new mapping of an existing WoT operation need not change
the Engine; a new WoT operation or authorization/lifecycle guarantee may.

The [role-responsibility reassessment](#responsibility-reassessment-without-application-allowlists)
rejects application-maintained class lists. Its preferred common-rule/profile
combination is conditional: open descriptive classes and arbitrary undisclosed
class-implied execution obligations cannot both be guaranteed.

### Reconstructed baseline and prior evidence

Inspected source baseline: fetched master
`d904987caca33e8970e810116d06294c1aa435ce`. The links below identify the
relevant implementation, not evidence that its architecture is optimal.

| Area | Observed implementation and consequence |
| --- | --- |
| TD representation | [Thing](../td/src/thing.rs), [Form](../td/src/components/form.rs), [Context](../td/src/components/context.rs) and response/schema/security models retain many unknown values as extension maps. Context entries retain JSON objects/URIs; they do not resolve extension identities. Form deserialization already collapses omitted and explicit default `contentType`. “Lossless” must name which distinctions matter, rather than promise original syntax. |
| Validation/defaults | [Validation](../td/src/validate.rs) checks typed TD semantics at project-defined levels. Profile/Full context checks recognize official context placement; they are not JSON-LD expansion. [Default helpers](../td/src/td_defaults.rs) centralize effective operations, security inheritance and additional-response media. [Validated lending](../td/src/validated.rs) budgets inspection/resolution but exposes extension presence rather than a resolved extension tree. Project validation levels must not be mistaken for a completed standards-wide semantic resolver. |
| Earlier extension rejection | [SecurityScheme deserialization](../td/src/components/security_scheme.rs) accepts a fixed scheme discriminator set and rejects other names. A future `scheme: "ex:Ticket"` cannot reach any downstream semantic view through that model. Preservation must start at ingestion; an extensible view after a closed parser is insufficient. Supporting the new authorization behavior additionally needs a suitable security provider, not just retained metadata. |
| Compilation bottleneck | [LogicalInteractionPlan](../core/src/plan.rs) contains a Property Read projection, target/media/subprotocol and metadata-presence state. [BindingCompilerInput](../core/src/binding_compiler.rs) adds candidate/role, not extension values or complete effective security/response data. At equal registration, changing only the native method or MQTT topic cannot be lowered differently from otherwise equal inputs. Opening a trait does not recover an erased input. |
| Matching/selection | [Consumer aggregate](../planning/src/consumer_property_read.rs) materializes readable coordinates for one captured registration, requires its closed compiler support, and selects the first coordinate or a strict Form index. It does not implement the broader [specified capability index](../docs/spec/planning.md#capability-indexes). A compiler rejection can fail construction rather than classify an unsupported alternative. [Producer planning](../planning/src/property_read.rs) still takes the first read-looking Form and treats absent `op` as readable in that loop. |
| Registration | [Static capture](../core/src/binding.rs) downcasts to `ResolvedTargetCompiler`; Host supported capture also remains closed. Master includes #209's typed retained Host job improvement, not #210's external admission. [HTTP registration](../protocol-bindings/protocols/http/src/lib.rs) pairs that compiler and its Host client; invocation parses the copied target. These are useful restricted paths, not independent general lowering. |
| Ownership/resource split | [Planning's output](../planning/src/lib.rs), [compiler envelopes](../core/src/binding_compiler.rs), [Binding carriers](../core/src/binding.rs) and [Servient](../servient/src/property_read.rs) carry live identities, storage, leases and cleanup. They establish constraints worth preserving, not a reason to make every native program contain Engine generations or to expose Engine accounting mechanics to its author. |
| Separate legacy path | [Protocol-binding helpers](../protocol-bindings/core/src/form.rs) expose raw Forms and shared default/selection helpers. They demonstrate existing semantic work, but cannot silently serve as a fallback that restores raw TD interpretation inside a replacement Binding. |

[PR #206](https://github.com/yushun1990/clinkz-wot/pull/206) proposed open
lowering and smaller runtime entries. Its
[independent review](https://github.com/yushun1990/clinkz-wot/pull/206#pullrequestreview-5567741696)
left namespace/parameter sufficiency, session retirement and non-equivalence
collision relations as obligations. This investigation tests the first; it
does not claim to close the latter two. The current normative logical-plan
model is one plan per effective Form/operation, shared across candidates; the
replacement rationale is corrected here accordingly. The reason to reconsider
it is mandatory persistence and insufficient information, not assumed
per-candidate plan duplication.

[Closed PR #210](https://github.com/yushun1990/clinkz-wot/pull/210), inspected at
`1219bbd70e51e76c193d594ccd2deb0753118a14`, lends original Thing, Property and
Form objects and inherited security names to its compiler. That demonstrates
external inline compiler admission, with actual-layout storage and rollback
work. It also makes the downstream author responsible for recovering meaning
from raw extension/context structures. It does not meet this task's boundary.
Its [review probe](https://github.com/yushun1990/clinkz-wot/pull/210#issuecomment-6264001657)
found per-step cap and error-order issues at an earlier revision; its final
description reports repairs. Those runtime reports were inspected, not rerun
or adopted as semantic evidence. The
[closure finding](https://github.com/yushun1990/clinkz-wot/pull/210#issuecomment-6266271563)
is consistent with the source. No #210 code is imported here.

### Standards versus project choices

Consulted the specifications directly on 2026-10-19, including the following
sections. Fixed publication URLs separate stable requirements from evolving
binding guidance.

| Source | Standards contribution; limit of that contribution |
| --- | --- |
| [TD 1.1 Recommendation](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#sec-default-values), §§5.3–5.4, 6.3–6.5, 7, 8.3 and Appendix D | Defines WoT defaults/inheritance, Form requests, extension contexts and HTTP method rules. It supplies no Engine-to-Binding SPI. Core semantic resolution is more than JSON-LD expansion. |
| [WoT Architecture 1.1](https://www.w3.org/TR/2023/REC-wot-architecture11-20231205/#protocol-bindings), §§6.7, 7.5 and 8.6 | Separates abstract interactions and concrete protocol messages. Scheme alone cannot identify every subprotocol/platform mapping. It does not prescribe this project's plan ownership, storage or registration types. |
| [WoT Profiles, November 2025 Working Draft](https://www.w3.org/TR/2025/WD-wot-profile-20251104/#sec-profiling-mechanism), §§4–6 | Defines profile conformance claims, shared constraints and native mapping rules. This is a Working Draft, not a Recommendation. Its standard `Thing.profile` carrier is tested below; neither the URI nor the advertisement supplies trusted executable interpretation. |
| [Binding Templates, May 2024 Note](https://www.w3.org/TR/2024/NOTE-wot-binding-templates-20240528/#protocol-binding-templates), §4 | Describes protocol, payload and platform binding dimensions. The [November 2025 Note](https://www.w3.org/TR/2025/NOTE-wot-binding-templates-20251104/#sotd) retires the main document and redirects future work. Neither is a universal operational interpreter. |
| [HTTP binding](https://w3c.github.io/wot-binding-templates/bindings/protocols/http/) and [MQTT binding](https://w3c.github.io/wot-binding-templates/bindings/protocols/mqtt/) | Consulted the respective mapping/vocabulary sections. These evolve; HTTP examples include a newer TD context, and MQTT explicitly remains work in progress. The probe uses TD 1.1 HTTP rules and an explicitly declared illustrative MQTT vocabulary, not a new normative MQTT profile. |
| [JSON-LD 1.1](https://www.w3.org/TR/json-ld11/#scoped-contexts), §§4.1, 4.2 and 9 | Term definitions can affect keys, values, scoped contexts, datatypes, lists and JSON literals. Identity expansion supplies no protocol semantics or ontology entailment. Lexically different aliases can denote the same property. |
| [JSON-LD algorithms](https://www.w3.org/TR/json-ld11-api/#expansion-algorithm), §§4–5 and [9.4](https://www.w3.org/TR/json-ld11-api/#remote-document-and-context-retrieval) | Expansion can discard unmapped properties/nulls and normalize value containers. A custom loader can supply context documents. A resolved graph alone is not a lossless source representation. |

The project's stronger obligations—no Binding TD rescans, deterministic final
selection, complete Producer advertisement, source-free execution, bounded
resource ownership and portable deployment—are architectural choices to
justify. They do not follow automatically from standards conformance.

### Alternatives challenged

| Candidate | Falsifier or cost | Disposition |
| --- | --- | --- |
| Raw TD/context loan, as in #210 | Each author needs TD navigation and context/value/default semantics. Two extension aliases can become different native requests. | Reject as the semantic boundary, even if borrowing is safe. |
| Larger fixed resolved operation record | Adding HTTP method and MQTT fields works until another vocabulary needs a nested rule, schema reference or response predicate. | Retain a small shared WoT projection, reject a protocol-field union as the extension mechanism. |
| Flat IRI-to-JSON map | `verb` under a property-scoped context, an IRI-coerced string and a JSON literal require different processing. Prefix substitution and arbitrary string expansion misinterpret them. | Insufficient. |
| Expanded JSON-LD graph/tree alone | Source coordinates, omitted/empty/null distinctions and some container structure disappear. It also leaves WoT defaulting undone. | Insufficient as a complete compiler input. |
| Expanded values plus shallow presence/shape | Handles useful aliases and typed values cheaply for an author, but `[null,"A"]` and `["A",null]` collide in the executable probe. | Universal source preservation falsified; plausible semantic subset when vocabularies declare lists/JSON literals. |
| Shared WoT facts plus structured extension values and bounded queries, under a declared feature profile | Must specify scope/linking, value types, loss diagnostics and budgets. Unsupported representational features are explicit. | Recommended minimum responsibility boundary; not yet a complete production contract. |
| Add a complete resolved source overlay | Must annotate nested source occurrences before expansion loses them, including null positions and lexical containers. | Credible broader alternative for demonstrated syntax-sensitive mappings; not mandatory without that need. |
| General SPARQL/reasoning service | Adds graph/entailment machinery and uncertain work bounds without resolving syntax preservation or protocol meaning. | Unnecessary. Queries should initially be scope/member/reference access. |
| Require preprocessed binding definitions | Host tooling resolves TD/contexts and invokes the Binding compiler ahead of runtime. Limits dynamic admission; useful for fixed firmware. | Credible deployment of the recommended boundary, not a second semantic interpretation. |
| Move whole TD processing to each Binding | Simplifies the Engine by transferring exactly the duplication the Owner wants removed. | Reject under the stated goal. |

An extensible data algebra is sufficient in principle; a query facade need not
materialize a second full TD. It can borrow indexed source nodes with resolved
annotations, traverse a prepared compact table, or use an admitted cursor.
Representation should follow measured cost and semantics. A different public
Rust struct is not itself an architectural improvement.

### Proposed semantic contract and its limits

**Shared resolution (questions 1–4).** The Engine interprets standard WoT
terms once: effective operation and its complete source Form operation set,
access flags, schemas, security requirement expressions/references and defaults,
media/response relationships, URI bases/templates and variable declarations.
Keep origin/presence when it affects meaning or diagnostics. Retrieval base,
TD `base`, JSON-LD `@base` and optional source `id` are distinct; using one as
another is not a generic resolution rule.

The extension side exposes named scopes for Thing, affordance, Form, schemas,
security definitions and responses, with generic references to other admitted
nodes where needed. Terms have expanded identities; values distinguish node
references, literals with datatype/language/direction, ordered lists, value
collections and opaque JSON literals. Do not combine root/affordance/Form
extensions into an implicit override map: protocol-specific precedence belongs
to the relevant Binding/platform interpreter. An unknown extension at a parent
scope can matter to a child operation.

Define **semantic structure preservation** against the declared vocabulary
contract. Semantic lists retain order; JSON literals retain their JSON data;
ordinary value collections carry no order guarantee. Native request defaults
must not hide malformed/dropped instructions, so preserve occurrence/error
facts where a vocabulary's validity/defaulting depends on them. The probe
conservatively refuses explicit empty/null method values instead of turning
them into an absent method. It does not validate every nested vocabulary shape.

If an admitted mapping really requires source member/container order or null
positions outside semantic lists/JSON literals, either reject that input feature
or deliberately add a complete resolved source overlay. Such an overlay must
correlate source occurrences with resolved identities throughout nested objects;
raw TD/context lending is not a repair. Original bytes and signatures are
separate provenance capabilities. Do not require a second full source model
merely because the shallow sidecar cannot meet that stronger contract.

| Context/extension case | Required behavior |
| --- | --- |
| Different prefixes, aliases, equivalent supplied contexts | Resolve to the same semantic identities and lowering, within the declared processing profile. Lexical spelling remains diagnostic provenance. |
| Property/type scoped contexts and keyword aliases | Engine applies them before exposing values. Never let a Binding implement a second context processor. TD core aliases/remappings require deliberate TD serialization/validation support; the probe qualifies extension aliases only. |
| Undefined simple terms | With a vocabulary mapping they may expand to an unfamiliar IRI; without one they may be dropped by ordinary expansion. Preserve/report that condition rather than infer a protocol default. |
| Unmapped compact-looking term | `oops:methodName` may itself be an absolute IRI; it is not thereby HTTP's method term. No guessed namespace repair. |
| Remote contexts | Use an immutable, supplied dependency closure with URLs, document bases and content identity. Missing dependencies are resolution failures. Fetching, if a Host application opts into it, belongs to an explicit acquisition policy outside pure lowering. |
| Unsupported JSON-LD features | Name the unsupported feature/scope. Do not interpret it approximately. Reverse/named-graph/propagation/import support can be added generically, independently of a protocol crate. |
| Unknown extension | Preserve it. Apply the selected common execution contract or a trusted interpreter, then require support for its operational requirements. Unknown does not mean optional; an IRI alone supplies no criticality information. Parent classes under the proposed descriptive contract are a defined exception, not automatically inferred safe terms. |

Conservative refusal of unknown instructions is an availability trade-off.
Ignoring all unfamiliar terms is an incorrect extensibility story; rejecting
every extended TD globally is also too restrictive. Classify support at the
smallest sound scope. A missing root context can invalidate resolution of the
whole document. A proven isolated unsupported Form extension can make only
that candidate unavailable. The probe expands the entire document and does
not demonstrate recovery from an unavailable Form-local context.

### Annotation roles and composability

**Historical experiment.** `AnnotationProfile` is retained as counterexample and
comparison evidence, not the recommended application API. The
[responsibility reassessment](#responsibility-reassessment-without-application-allowlists)
below incorporates the Owner's prohibition on application-maintained role lists.

The empty-profile refusal policy has a concrete availability defect: adding
the review's Thing-level `@type: "https://schema.org/TemperatureSensor"` to a
healthy HTTP description makes its lowerer reject. This is not a satisfactory
general default for an extensible WoT product. [TD semantic annotations](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#semantic-annotations)
include domain metadata, and the [SAREF TemperatureSensor class](https://saref.etsi.org/core/TemperatureSensor)
supplies a real domain-class example. Neither the existence of a class IRI nor
its prefix proves a role in native execution.

| Interpretation policy | Disposition |
| --- | --- |
| Reject every unknown at every scope | Reproduced by the empty-profile test. Preserves conservative refusal but prevents unrelated domain annotations from composing with both mappings. Insufficient as an unqualified product recommendation. |
| Ignore all unknowns, a whole namespace, or everything on a parent scope | Reject. A parent `requireConfirmation` instruction can affect every child interaction. Namespace membership or scope alone does not establish inertness. |
| Treat every Thing/affordance `@type` as descriptive by contract | Credible simpler alternative if classes alone cannot activate native/security requirements. An explicitly activated interpreter can still consume the class context. That restriction must be explicit; it cannot be inferred from arbitrary future vocabulary. |
| Explicit scoped vocabulary-role profile | Selected for the narrow experiment. Trusted declarations classify exact type IRIs at Thing/affordance scope as non-operational metadata. Unclassified values remain available to interpreters and unsupported instructions remain refused. No protocol-specific Engine branch or per-Binding annotation list is required. |
| Separate dialect/interpreter capability manifests | Credible way to supply/check such declarations without every application author maintaining lists. Conflict, completeness, trust and resource costs remain unverified; no manifest framework is implemented here. |

`AnnotationProfile` is an experimental application-supplied declaration, not
semantic inference or a conformance certificate. It accepts exact scope/type
pairs, never wildcards. Declared types remain in the public
`Interaction.annotations` facts after classification, and the full schema still
retains its classes. A property's standard data type and additional semantic
class are separated using their actual context-resolved origins, so a numeric
schema can also carry a domain annotation. The native lowerers are unchanged.

The side-by-side tests use both the review's example IRI and SAREF at Thing
and typed-property scope, including a class alias with its own scoped context.
Both lowerers produce
the same programs as the unannotated inputs when that interpretation profile
is explicitly supplied. A fixture-defined mandatory `requireConfirmation`
instruction remains unsupported at Thing, affordance and Form scope. The same
domain type on a Form, an undeclared type in the same namespace, and a type
declared only for another scope are not exempted. This experiment cannot
classify Form/response/security statements or arbitrary annotation properties.

The earlier experiment left ownership of these declarations unresolved. Requiring
applications to supply them is now ruled out: successful compilation after an
application author labels classes inert does not establish practical semantic
interoperability. An erroneous declaration can invalidate execution, regardless
of whether an application, Binding or central list supplies it. The experiment
does establish preservation and the cost of blanket refusal. Any future compiled
reuse must include its actual interpretation contract/dependencies, not just a
human-readable profile name.

### Responsibility reassessment without application allowlists

**Preferred direction:** a small, versioned common execution contract for domain
class assertions, combined with explicit activation of protocol/profile
requirements and optional shared interpreters for requirements that need them.
Do not build a universal role classifier. Ordinary applications install Bindings
and supported semantic profiles and set security/deployment policy; they never
decide whether an unfamiliar class or predicate is harmless.

This revises the responsibility recommendation, not production authority.
Neither independent review of the earlier probe nor the Owner's clarification
adopts this particular mechanism. The new executable cases test distinctions
between alternatives; they do not implement the proposed profile-composition
or requirement-accounting contract.

#### Assumptions challenged

1. Refusing every unfamiliar term is not a neutral safety policy: it makes
   independent metadata an availability dependency of every native mapping.
   The original default refuses a harmless-to-the-mapping SAREF assertion.
2. A context expands names/values; it does not tell a consumer which operational
   rules it must implement. An ontology, an IRI, a namespace and a role label
   are not by themselves executable interpretation or a trust certificate.
3. Moving the exact inert-class list into every Binding duplicates a common
   problem. Moving it into a centrally approved list changes its administrator,
   not its logical sufficiency or extensibility cost.
4. A role declaration needs an interpretation contract, provenance, version,
   activation condition and applicable scope. A naked `descriptive=true` flag
   cannot override native or security requirements safely.
5. Registration support and semantic obligation are different. An installed
   Binding may understand a vocabulary without imposing that vocabulary's
   rules on every other mapping. Installing another Binding must not silently
   reinterpret an already admitted description.

#### Alternatives and developer costs

| Alternative | Constructive value and falsifier | Assessment |
| --- | --- | --- |
| Common descriptive-class contract | One shared rule accepts new parent class assertions without class inventories. It cannot discover an unstated nonce/authentication obligation implied solely by a foreign class. | Preferred base, conditional on the explicit execution contract below. Not proof that arbitrary classes are intrinsically inert. |
| Reusable trusted vocabulary/role definitions for every extension | One vocabulary/profile provider can supply shared interpretation to several Bindings. Missing definitions for every new domain class become admission blockers; a role label without an interpreter can still lose required behavior. | Useful for selected operational profiles and annotation predicates outside common rules; unnecessary for every descriptive class. A universal prerequisite recreates the original availability problem. |
| Binding-declared interpretation capabilities alone | Native authors already know their dialects, inputs and unsupported instructions. Independent Binding declarations can disagree on whether a parent requirement is optional, leaving a less capable Binding as an unsafe fallback. | Necessary support evidence, insufficient as global role authority. Capabilities must satisfy requirements fixed independently of candidate ranking. |
| Common rules plus optional shared profile/vocabulary interpreters | No domain-class enumeration for base mappings. A selected dialect can use parent context; shared codecs/security/profile helpers interpret common rules once, outside native adapters. Costs arise only for the additional semantics actually selected. | Preferred combination. Ordinary Cargo libraries/typed helpers suffice as a starting hypothesis; no manifest framework or central protocol registry is justified. |
| Conservative refusal of every class with an unavailable role definition | Can support a closed semantic profile that refuses ambiguity. It rejects new SAREF-like metadata even when the native Form is complete. | Credible optional strict/legacy profile, not the general WoT default. The application selects a complete supported profile, not per-IRI exceptions. |
| Ignore all unknown parent metadata or rely on namespace/scope alone | A Thing-level `requireConfirmation` member can govern all Forms. A Form annotation can be descriptive too, while a parent extension can be operational. | Reject as a general inference rule. A normative contract can assign roles; location or spelling cannot prove harmlessness. |

The preferred combination avoids universal vocabulary coverage: common rules
handle descriptive class assertions; an actual extension interpreter handles
the additional semantics it claims. A complete new native protocol mapping
still needs its own protocol code, not a Core vocabulary case. A common payload,
security or profile vocabulary should have a reusable helper/interpreter shared
by those mappings, rather than independent copies of its defaulting logic.

#### What treating parent `@type` as descriptive actually means

Under the proposed **base execution contract**, extension class assertions on
Thing and affordance nodes are preserved semantic descriptions. They do not,
by themselves, add mandatory protocol steps, change authorization, select a
different native dialect or change WoT operation semantics. No list of SAREF,
schema.org or future class IRIs is needed. The rule belongs in common semantic
preparation/SDK code and applies identically to all base Bindings.

This defines the proposed execution contract, **not an inference that a term
is safe to ignore because it occurs at a parent scope**. It does not alter the
standard DataSchema `type`, effective WoT security, Form instructions, response
rules or required codec semantics. Those remain operational requirements.
Arbitrary unknown parent predicates remain unresolved requirements unless a
common rule or selected interpreter gives them another meaning. In particular,
the proposal does not solve arbitrary annotation-property interoperability.
This classification happens after context processing: even a descriptive class
can select a type-scoped JSON-LD context that changes expansion of operational
values. Removing it before resolution would corrupt those values.

The rule excludes an unconstrained legacy mapping that says only
`Thing.@type = ex:ConfirmedDevice` while silently requiring an extra handshake,
or `ex:AuthenticatedDevice` while omitting the relevant security requirements.
The Engine cannot discover these implications from the class assertion alone.
A trusted, supported semantic profile can accommodate such a mapping only when
its activation and complete interpretation are established before candidate
selection. A closed legacy profile may require all relevant role definitions
and refuse unknown classes; it cannot inherit the base contract's openness and
also promise discovery of arbitrary implicit obligations.

Merely selecting the base contract locally does not prove that an arbitrary
legacy producer followed it. Admission needs a supported producer/profile
contract or deployment qualification establishing that operational requirements
are explicit. The exact assertion/qualification mechanism remains unresolved.
If admission must accept unconstrained TDs while guaranteeing every foreign
class implication, the stated combination is impossible; conservative refusal
with a known complete profile is the narrow credible alternative. Neither a
context URL nor an application-supplied per-term exception supplies that proof.

Descriptive assertions **may be inputs** to an explicitly activated interpreter.
For example, a Form's known versioned dialect can require a confirmation step
whose parameters depend on a Thing/Property class or member. The dialect,
not the ambient class alone, activates the obligation. Every eligible Binding
must implement it or reject that candidate; a plain HTTP Binding cannot fall
back to a GET that omits it. The class remains preserved and queryable.

[TD 1.1's Thing and Form models](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#form)
describe semantic tags, profile associations and subprotocol metadata. They do
not provide this proposed criticality contract. A context URL is not a
must-understand declaration. `Thing.profile` is a standard conformance claim
with potentially operational consequences, not a universal interpreter-loading
instruction. The earlier `subprotocol` fixture proves one local dialect only;
the standard carrier and its interpretation boundary are examined below.

#### Responsibility, activation and dependency ownership

| Owner | Obligation and ordinary cost |
| --- | --- |
| Engine/TD semantic layer and common SDK | Implement standard WoT meaning, context resolution, the selected common contract, preservation and generic traversal/coverage diagnostics. Keep execution requirements distinct from declared descriptive assertions. Native vocabulary meanings stay outside Core. |
| Vocabulary/profile/codec/security library maintainer | Supply versioned interpretation for its additional semantics, including activation, shared defaults/validation, scope and required dependencies. Ship it with a supported Binding/profile package or ordinary shared library. Descriptive-only class IRIs need no such package under the base contract. |
| Binding author | Implement native lowering and declare supported operation/role/dialect/profile capabilities and actual interpretation dependencies. Return unsupported for unsatisfied requirements. Reuse common semantic and vocabulary helpers; do not maintain an inventory of unrelated domain classes or mirror a second hand-written registry of every parser branch. |
| Application developer | Choose installed Bindings, supported whole profiles, credentials/security policy and deployment limits. Normal dependency/version selection is allowed; manual per-term role decisions are not. |
| Deployment/tooling | Acquire approved immutable dependencies before admission and qualify the selected environment. Host preparation can emit checked definitions for constrained execution; runtime network fetching is not required. |

Establish the active interpretation contract/dependency closure before lowering.
Resolve explicit caller/property/operation/Form/Binding constraints as already
demonstrated. Derive required semantics using common rules and the selected
trusted interpreter; then test each candidate's support. A candidate cannot
erase a requirement merely because it does not understand it. Final selection
and Producer coverage remain Engine-owned, with explainable unsupported/missing
dependency/conflicting-declaration outcomes. Do not use registration order to
settle semantic conflicts. Different explicitly selected profiles may coexist
without sharing one global vocabulary-role map.

The additional metadata, where needed, comes from the supported interpreter's
maintainer, not an application's hand-authored allowlist or an arbitrary
document retrieved at compile time. Its smallest credible form is normal code
and package data documenting a fixed interpretation/version and its dependencies.
A source/schema/profile author must identify the relevant operational dialect
according to that contract. A role-only definition cannot implement required
behavior without matching interpretation and native support.

Validate dependency identity/composition when preparing the installed profile
and validate applicability/completeness against the actual TD before exposing
support. Pin context URL, effective document/base URL, exact content identity,
semantic-contract/vocabulary version and interpreter implementation dependency.
Context version alone is insufficient when operational meaning changes without
new term IRIs. A digest identifies content; package provenance/qualification
establishes trust. Conflicting active meanings or missing required dependencies
fail preparation or the affected candidate before effects. A bare claim of
support is a linked-code conformance obligation, not proof from metadata.

No public-network access is implicit. Contexts needed for key/value expansion
must exist in the supplied closure, or resolution fails. An absolute unknown
class IRI can be preserved under the descriptive contract without dereferencing
its vocabulary. An unknown Form instruction, unfulfilled operational profile,
or unknown security requirement is unsupported when its required interpreter
is unavailable. An optional definition must not be needed to decide whether
an otherwise invisible mandatory obligation exists: that would recreate the
ambiguity rather than solve it.

#### Counterexamples that distinguish the alternatives

| Case | Independently checkable consequence |
| --- | --- |
| SAREF class on a Thing and numeric Property | The original empty `AnnotationProfile` path rejects. The added common-rule experiment preserves the class and NumberSchema and produces unchanged HTTP/MQTT programs without an IRI list. A second Binding requires no new domain policy. |
| Mandatory unknown `ex:requireConfirmation` on a Form | Both the original and common-rule paths refuse it. Knowing the HTTP scheme/method or a harmless parent class does not discharge this instruction. A namespace-wide exemption would admit it incorrectly. |
| Same required predicate on Thing/Property | The common-rule path still refuses it. The proposal does not infer that all parent metadata is descriptive. |
| Parent context drives a known selected dialect | In the added fixture, `probe-confirmed-read-v1` requires a parent `ConfirmedDevice` class and compiles a confirmation-bearing native definition. Ordinary HTTP rejects that dialect. Missing context, an unknown dialect or another unknown Form step prevents selection. This is lowering evidence, not a handshake runtime. |
| Operational meaning implied only by a class | Remove the dialect selector from the preceding input: the base mapping emits its ordinary GET. The passing test deliberately records this limitation. A foreign vocabulary requiring confirmation or authentication from that class alone is outside this base contract. |
| Two Bindings share a domain vocabulary | A single common rule handles its class assertions; neither lowerer contains the vocabulary. If they instead share an operational codec/profile vocabulary, both can call one maintained interpreter. Reusing logic does not prove once-per-document work or shared runtime ownership. |
| Conflicting active declarations | Within selected profile P/v1, one dependency declares R descriptive, another declares R requires a nonce. A planner that accepts the first declaration or a less capable Binding can omit the nonce. Refuse the inconsistent composition. P/v1 and a separate explicit P/v2 are different contracts, not automatically conflicting global entries. No conflict manager is implemented here. |
| Incomplete declaration | A package labels R operational but lacks its referenced nonce schema/interpreter, or Binding A implements its obligation while B merely recognizes R's name. Neither the label nor B's name recognition can justify support. Require complete interpretation/dependencies; B rejects. |
| Completely unknown vocabulary, no local definition | A resolved parent class assertion is descriptive under the chosen base contract; an unknown Form/parent instruction or required profile is unsupported. If even its context needed for expansion is missing, fail resolution. Fetching an ontology cannot replace an absent execution contract. |

There is an unavoidable ambiguity outside a contract: two authors can supply
the same otherwise complete HTTP TD with `@type = urn:unknown:X`. One vocabulary
means only a domain classification; the other also demands a nonce before
success. The plain server can reject an unsolicited nonce exchange, while the
other rejects a GET before that exchange; always adding a handshake does not
repair the ambiguity. With no definition/activation information, all observable
TD input is identical but required native behavior differs. No algorithm, new Rust type or
relocation of an allowlist can make both interpretations correct. The base
contract deliberately admits the first meaning and requires an explicit
operational profile for the second. This is not universal semantic discovery.

#### Standard `Thing.profile`: advertisement, activation and coverage

**Standards reconstruction.** [TD 1.1 §5.3.1.1](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#thing)
makes `profile` optional, accepting one URI or an array, identifying mechanisms
followed by the TD and Thing. [WoT Profiles §4](https://www.w3.org/TR/2025/WD-wot-profile-20251104/#sec-profiling-mechanism)
uses it to claim conformance; profile conformance includes the profile's
normative assertions. The draft permits extra bindings and cautions that a
claim is not proof of conformance. It recommends avoiding changed defaults or
unexpected behavior for non-profile consumers; these are SHOULD NOT statements,
not a universal guarantee that unknown profiles impose no requirements.

The same draft includes common security/error constraints and an HTTP Basic
`readproperty` mapping with GET, JSON acceptance and a successful 200 JSON
response. Profile meaning therefore exceeds a protocol scheme or a Form's
`subprotocol`. Listing several profiles claims several conformances; it is not
an ordered list from which a compiler can choose the weakest interpretation.
Nothing in these specifications installs trusted code from a URI or proves a
Consumer's profile conformance from one successful operation.

**Actual trace at the earlier `c9067d4` probe boundary.** `ROOT_KEYS` omits
`profile`; the pinned official context expands its key to
`https://www.w3.org/2019/wot/td#followsProfile`. That context does not `@id`-coerce
its values: the example is `Member(..., (Atom(profile_uri),), ...)`, not a
`Reference`. `Trace.scope` retains it in `Interaction.root_extensions`.
`DescriptiveParentTypes` changes classes only. HTTP reaches `common_support`,
which rejects the field as unknown. MQTT does likewise for an MQTT Form, or
rejects the HTTP scheme first. Thus preservation works; standard-profile
activation did not. Production [Thing](../td/src/thing.rs) already stores profile
URIs; this is a limitation of the disposable semantic adapter, not evidence that
master lacks the standard field.

Simply adding `profile` to `ROOT_KEYS` would silence that rejection while
carrying neither advertisement nor requirements. That is not a repair. The
existing Form dialect experiment is also not a substitute for interpreting
document-level profile constraints.

**Preferred responsibility boundary, still proposed.** Treat the standard
advertisement as an input to preparation, not as trusted implementation or a
candidate Binding choice:

1. Shared TD processing preserves and validates advertised profile identities
   separately from active requirements. Context processing does not interpret
   the associated specifications. No profile URL is fetched as executable code.
2. Already installed, versioned profile interpretation code establishes the
   obligations of the selected admission contract from those claims and the
   resolved TD. The provider comes from a supported profile/Binding package or
   shared library, with pinned specification/context/code dependencies. Applications
   choose such packages/whole contracts; they never enumerate annotation roles.
3. Before native candidate compilation, fix requirements for each operation/Form
   coordinate, including document-wide constraints. Interpretation can inspect
   resolved Form facts to define scope, but cannot depend on which Binding wins.
   Known profile composition must be consistent; unsupported dependencies or
   unresolved scope cause refusal. Candidate removal, priority or strict Binding
   selection cannot weaken these requirements.
4. Bindings lower against that fixed input and report which assigned obligations
   they implement. The Engine checks complete coverage before ranking supported
   candidates. It compares identities/coverage generically; profile/native
   libraries interpret the obligations. Security/codec/Engine obligations require
   their appropriate providers too, rather than making every native Binding
   implement an entire profile. Strict selection cannot waive missing coverage.

Requirements can be a pure function of the fixed contract, advertised profiles,
resolved TD and operation/role/Form coordinate. This does not require eagerly
storing a second requirement table for every Form. Evaluating only requested
coordinates must give the same requirements regardless of candidate registration
or ranking.

This needs no universal protocol registry: ordinary linked libraries can supply
a local interpretation function before selection. A small opaque requirement
token suffices for the counterexample, not for every future requirement or Rust
SPI. Token recognition and a compiler's coverage claim are not conformance proof;
qualified interpretation, actual native behavior and dependency evidence are
still necessary. Returning success only when every assigned requirement is
implemented could carry the same contract; the test's explicit coverage set does
not mandate a second manually maintained SDK ledger. This investigation adds no manifest grammar, generic
profile dispatcher or production registration obligation.

**Small executable distinction.** [Five added cases](probes/td-binding-semantics/test_profile_boundary.py)
leave the resolver, public interface and HTTP/MQTT lowerers unchanged. Test-only
preparation reads the already-resolved standard member and retains its claims
alongside immutable requirements. An explicitly supplied local function
interprets only the HTTP Basic draft's read request/response rule; it is not a
complete profile implementation. The Engine-side wrapper checks opaque coverage
before using the existing selector; no HTTP vocabulary appears in that check.

| Input/action | Observed result and implication |
| --- | --- |
| Standard HTTP Basic URI, or singleton-array spelling | Both resolve to the same preserved advertisement. The original lowerer still refuses `followsProfile`; this negative evidence remains. |
| Erase the standard field without interpretation | Ordinary HTTP emits no expected response status. Advertisement erasure loses a profile obligation; successful compilation is insufficient. |
| Supply the local read-slice interpreter before either candidate | It fixes one versioned read-rule obligation. Ordinary HTTP can construct a native GET but reports no profile coverage; the Engine excludes it. The aware example emits JSON `Accept` and expected status 200, and covers that rule. |
| Reverse candidate order, remove the aware Binding, or strictly request ordinary HTTP | Reversing order still selects the aware candidate. Removing it or strictly requesting the incomplete candidate fails. Requirements retain the same identity; selection cannot dilute them. |
| Missing interpreter, unknown advertised URI, or known-plus-unknown array | Refusal occurs before the sentinel native compiler can run. Knowing one advertised profile does not justify dropping the other. This is the experiment's conservative admission policy, not a TD-mandated treatment of all unknown profiles. |
| Install interpretation code without an advertisement | No profile requirement activates in this fixture. Deployment-required profiles would need a separately declared contract; installation alone does not reinterpret a TD. |
| Profile claim plus contradictory POST, unknown subprotocol/Form step, or Basic security unsupported by the native example | No candidate is selected. A profile claim neither overrides explicit native instructions nor supplies missing security capability. |

**Remaining trade-offs.** Whole-profile conformance, operation-compatible
execution and native Binding support are different claims. The read-slice
function omits shared constraints, other operations, security bootstrapping,
runtime validation and complete profile conformance. The fixture demonstrates
coverage ordering and one native obligation, not admission safety for arbitrary
profiled TDs. Its interpreter deliberately refuses non-HTTP reads; it does not
show that profiles should prohibit extra MQTT Forms.

For a general implementation, compute scoped requirements for all eligible
coordinates before Binding ranking. A known SSE profile's observation rules
need not apply to a simple read, and HTTP-specific rules need not govern an
additional MQTT Form. Shared constraints can still apply. An unavailable
profile interpreter cannot establish such isolation. Conservatively refusing
that unknown document-level claim preserves uncertainty but reduces availability,
including a known-plus-unknown array that might actually be compatible. A broader
base-TD-only path would require a qualified contract establishing that the
ignored profile has no required effects on the selected operation; guessing
from a URI or choosing a weaker Binding is insufficient. No such path is proven.

The preferred next experiment is scoped known-profile composition and real
shared-constraint/provider coverage, not a manifest framework. Exact activation,
scope/composition, offline dependency qualification and truthful native coverage
remain open before production SPI design. Unknown-profile refusal is a proposed
feature/availability limit for Owner review, not a new standards requirement.

#### Exact limit for non-`@type` annotations

The proposed common rule covers parent **class assertions only**. “Unknown”
here means uninterpreted by the selected common rules/libraries; a published,
resolvable IRI is not sufficient. The new case attaches
[RDFS's human-readable label](https://www.w3.org/TR/rdf-schema/#ch_label) to a
Thing or Property. Its resolved predicate/literal is preserved, but both native
lowerers refuse it as an unknown extension. Activating the known read-profile
slice does not exempt it. This demonstrates a concrete annotation availability
failure, not loss of information or proof that the label is an instruction.

**Recommendation for the initial feature envelope:** deliberately leave automatic
acceptance of unfamiliar non-`@type` predicates unsupported. A maintained shared
interpreter or separately reviewed common rule may classify/interpret specific
annotation properties. Do not ask applications or every Binding to maintain
term exceptions, and do not let a known profile bless an entire namespace.
Broad arbitrary semantic-annotation interoperability is therefore not a proven
property or part of this initial proposal. If it is required for v1, this scope
is insufficient and that contract needs further investigation before migration;
the Owner must accept or reject the availability limit. The prototype's refusal
is not a justification for permanently classifying all foreign predicates as
mandatory protocol instructions.

#### Evidence, exclusions and decision boundary

The original 23 tests still establish their narrow information/selection facts;
the exact application allowlist proves only that an explicit classification can
make those fixtures compile. It is **no longer recommended** as the target API,
whether maintained by an application or copied into every Binding. The four
[role-contract cases](probes/td-binding-semantics/test_role_contract.py) distinguish the common-rule alternative,
mandatory-instruction refusal, explicit parent-context activation and the hidden
class-obligation counterexample. Conflicting/incomplete declarations are concrete
reasoning cases above, not tests of an invented manifest framework. The five
standard-profile/annotation cases above additionally distinguish advertisement,
trusted read-slice interpretation, fixed requirement coverage and native
applicability. They do not complete the whole-profile model.

**Proposed decision for independent review and Owner approval:** prefer the
minimal common-rule/explicit-operational-profile combination. Keep the stronger
closed interpretation path available as a profile-specific alternative where
legacy class implications are required. Do not require an ontology/role package
for each domain class, and do not let application role exceptions repair missing
interpreter support. A generic consumed/required-fact helper may be useful for
Binding authors; this investigation does not settle its API or justify a
universal declaration language, resolver registry or vocabulary service.

**Residual product trade-off:** accepting the descriptive-class contract gives
open class-annotation interoperability while excluding undeclared implicit
protocol/security obligations. Supporting such legacy semantics requires a
trusted whole profile and refusal when its needed interpretation is unavailable.
The Owner decides whether that compatibility limit is acceptable; applications
must not be asked to make per-term exceptions. This question does not shift the
technical responsibility for defining/qualifying a supported profile to them.

**Still unverified before a Rust SPI:** the exact profile activation/composition
contract; standard/vendor mapping compatibility with the base rule; operational
annotation predicates beyond class assertions; scope/link/response/schema and
per-call parameter completeness; conflict/coverage diagnostics; helper ergonomics
for an independently authored Binding; and actual preparation/storage/cleanup
costs. Host can prepare with a broader feature envelope, and build-time tooling
can supply immutable native definitions to `no_std` firmware. Neither that
decomposition nor these Python tests proves bounded embedded resolution, memory
fit, SDK termination or a production security guarantee. No additional native
resource accounting or registration framework is selected.

### Applicability and final choice

**Selection ownership (question 5).** The Engine determines that a Form can
describe the requested WoT operation, validates shared requirements and applies
application/security/codec constraints. A Binding determines whether it can
implement the resulting native request, including its protocol defaults,
extensions, URI dialect and supported execution profile. Coarse indexed
capabilities are an over-approximation, not the final support verdict.

Use one pure lowering result per eligible Form/operation/registration:
supported native definition, unsupported with a diagnostic, or a fatal
construction/contract/resource failure. Separate these outcomes so allocation
failure does not masquerade as permission to choose another protocol.
Compilation can supply the support verdict; a mandatory separate `supports`
pass risks duplicate parsing and disagreement.

Before invoking any lowerer, restrict the candidate set by the requested
affordance identity, operation, explicit Form and explicit Binding. A Form
index is local to its affordance, not a global selector. The probe now requires
the property name, with Actions/Events still outside its input profile, and
records that name in candidate diagnostics. This corrects the earlier
operation-only selection and post-compilation strict Form filter.

The Engine ranks supported Consumer candidates deterministically using caller
constraints, policy, Form order and explicit registration priority. Multiple
Bindings can support the same scheme. Strict Form/Binding selection must fail
if unavailable, without choosing an unrequested alternative. One unsupported
Form must not invalidate another supported Consumer Form. Preserve a bounded
reason for skipped alternatives. Runtime fallback after effects is a distinct
operation policy, not a side effect of compilation failure handling.

The negative evidence checks a fatal earlier compiler outside a strict Form
selection, an excluded Binding, two read properties both with Form index zero,
and mismatched operation/missing identity. Excluded lowerers are never invoked;
a fatal failure from an eligible lowerer still propagates. This establishes
pre-lowering filtering in the property probe, not scoped TD-validation or
resolution: `resolve()` still processes the full input first.

For Producers, every advertised Form/operation needs a supported owner and
handler/security/codec coverage before publication. Multiple native owners
require explicit resolution, not incidental registration order. Reject the
draft or explicitly derive a reduced advertised description first; never
publish an unchanged unsupported advertisement. Native pattern/topic overlap
still belongs to Binding session/install logic, with Engine-owned publication
and exact route provenance. Equality keys cannot describe every overlap relation.
The probe checks coverage/ambiguity without installing anything.

### Plan, program, context and resource ownership

**Execution division (questions 6–8).** Keep semantic authority and native
execution data distinct, without requiring a mandatory persisted intermediate
object between them.

| Owner/object | Necessary content; content that belongs elsewhere |
| --- | --- |
| Engine Plan/execution entry | Operation and affordance identity, supported candidate/registration association, security/codec/result obligations, dispatch and diagnostic coordinates, and program reference. Retain only facts actually needed for selection/validation/execution; it is not the Binding's source-information container. |
| Binding Compiled Program | Pure native instructions: parsed endpoint/template, method/packet, topic/filter and response/correlation mapping. Native representation can be an ordinary typed value. No socket, SDK task, handler, credentials or live generation authority. |
| Reusable definition | Optional factoring of immutable instructions/schema/template work with an explicit dependency and operand contract. Same TD model name, compatibility marker or binding configuration does not prove equivalent programs. No mandatory cache, serialization, VM or program registry. |
| Instance context | Checked endpoint/tenant parameters, selected definition, credential-provider references and instance routing/authorization associations where they actually vary. Live authority comes from current Engine ownership, not reusable bytes. Per-call URI variables remain checked per-call operands. |
| Runtime Binding owner | Sessions/connections, subscriptions, pending replies, correlation state, native buffers and SDK tasks. A shared broker session may serve many instances; closing one must not destroy siblings. |
| Engine generic adapters | Controlled table/job/slot/erasure allocation, actual layouts, storage leases and funded rollback/cleanup transfer. Third-party lowerers should not recreate Engine resource-account schemas. |
| Binding/application policy | Binding bounds native retained work/input and reports its capabilities/requirements; application qualifies SDK/runtime behavior and supplies deployment resources. An in-process trait cannot prove termination, honest costs or recoverability from SDK OOM. |

Retain Cargo-linked startup registration and pairing of compiler/executor by
native associated types or a generic erased adapter. **Redesign eligibility**:
remove concrete compiler recognition and private support tokens as conditions
for independent authorship. Consumer-only and Producer-only authors need no
dummy other half. The useful complete-registration invariant does not justify
today's representation-specific boilerplate.

Host authors should implement one lowering body and their native execution
owner, using generic storage/erasure helpers. They should not implement both
Host and static APIs unless they claim both deployments. A strict static
adapter can require caller-owned storage, finite resumable work and qualified
cleanup. Host can use allocating native types and its selected executor under
truthful deployment limits. Shared semantic outcomes do not imply identical
allocation failure or destructor contracts.

Separate preparation from execution dependencies. A fixed `no_std` application
can consume build-produced typed definitions and checked instance operands;
it need not link a JSON-LD processor. On-device dynamic admission needs a named
finite feature profile and actual input/context/depth/output/work bounds, with
resumable resolution or qualified atomic maxima. Host preprocessing may accept
a wider profile. Context dependency identity and compiler/profile versions
must accompany prepared data so another device cannot reinterpret it silently.
No probe here proves those resource bounds, an allocation-free implementation,
or physical MCU suitability.

[0081](0081-reusable-plan-and-thing-identity.md) already distinguishes definition
reuse from instance authority. This investigation adds no reuse implementation
or measured cold-start benefit. Do not force definitions to be parameterized
before a real compiler has reusable work. One per-instance execution-table
owner is sufficient unless plans can be replaced independently; remove a
separate mandatory plan-set lifecycle when it serves no such capability.

### Executable evidence and authoring burden

The [isolated probe](probes/td-binding-semantics/README.md) uses a pinned W3C TD
context and PyLD 3.1.0 to perform actual context resolution. Instrumentation
collects source occurrences; the public interface contains no raw TD or context.
HTTP and MQTT lowerers import only that interface and the Python standard
library. Both were authored in this investigation, so separate readability is
evidence of interface use, not independent implementation review.

The original 23 cases are preserved, with four role-contract and five
standard-profile/annotation cases: 32 tests pass, including deliberate refusal,
information-loss and hidden-obligation counterexamples. The original cases test prefix/alias/full-IRI/supplied-context
equivalence, scoped contexts, IRI/literal/list/JSON distinctions, alias collisions,
effective operations/security, response instructions, unsupported extensions, Consumer
choice, strict choice and Producer coverage. Unknown datatypes remain visible
but unsupported: carrying `"GET"` with a custom datatype cannot justify treating
it as an ordinary HTTP method.

The HTTP example produces GET/PUT from one defaulted read/write Form, accepts
an explicit POST for a single-operation Form, and refuses an explicit method
over a multi-operation Form. The MQTT example produces distinct subscribe/
retained-message and publish programs from broker and topic/filter metadata.
Its retained-read flag is a declared mapping prerequisite, not proof that a
broker currently holds a value or that delivery is fresh. No network runtime
is needed to expose these semantic differences.

| Authoring observation | Interpretation and remaining cost |
| --- | --- |
| HTTP/MQTT modules are 40/49 physical lines, including comments and blanks. | Actual protocol lowering is small in these deliberately narrow examples. Neither duplicates context resolution, WoT defaults, security inheritance or candidate selection. |
| Shared public model is 144 lines; experimental Engine adapter is 281. | Semantic preparation is nontrivial work transferred to one owner, not eliminated. Occurrence tracing depends on a private PyLD hook and must not become a production dependency by accident. |
| Authors enumerate the native instructions they interpret and inspect applicable scopes. | Actual protocol support policy remains necessary. Common semantic rules and shared vocabulary helpers remove unrelated class inventories and duplicate common interpretation. They must not require a parallel hand-maintained declaration of every parser branch. |
| Historical exact scoped `AnnotationProfile` composes with both unchanged lowerers. | Establishes explicit classification/preservation only. Application ownership fails the clarified requirement; copying its lists into Bindings is also no longer recommended. |
| A test-only common parent-class rule composes with both unchanged lowerers; a small dialect wrapper consumes parent context. | No class inventory or domain lookup is needed for the base rule. Explicit dialect discrimination prevents ordinary HTTP fallback in that fixture. This composability claim excludes unfamiliar annotation predicates. |
| A test-only standard-profile interpreter fixes one read obligation before any candidate. | Five cases separate advertisement, trusted interpretation, native support and Engine coverage/selection. The author of this narrow HTTP example reuses native lowering; full profile composition, common runtime/provider coverage and Rust author ergonomics remain unproved. |
| No author implements Host/static transport wrappers or Engine ledgers in this probe. | Demonstrates separation for pure lowering only. Generic Rust registration, allocating programs, pending jobs, indirect backing and cleanup remain unproved authoring costs. Do not extrapolate these line counts to a network binding. |

### Proven limitations and concrete counterexamples

1. **Preservation failure:** `[null,"A"]` and `["A",null]` under one ordinary
   extension property yield equal tested views. A future vocabulary whose
   positional instruction matters cannot be implemented from that input.
   Declared `@json` preserves it; declared list semantics preserve ordered
   semantic values. [JSON-LD's ordering model](https://www.w3.org/TR/json-ld11/#sets-and-lists)
   makes ordinary arrays unordered. This falsifies arbitrary source preservation,
   not sufficiency for semantic vocabularies using the right value constructs.
   Supporting syntax-sensitive mappings needs more than this sidecar; do not
   assume such support is necessary for every Binding.
2. **Meaning failure:** a preserved unfamiliar `requireConfirmation` term could
   require a handshake before success. An Engine cannot safely assume it is
   annotation, and a Binding cannot claim support by merely retaining it.
   Refusal or a corresponding interpreter is necessary.
3. **Address/profile mismatch:** the MQTT lowerer refuses wildcard reads and
   broker URIs with topic paths. These are author limitations, not failures
   to preserve the URI or a need for MQTT branches in Planning. Different
   mapping dialects can be separate registrations with the same scheme.
4. **Runtime mismatch:** a retained MQTT publication is not a fresh read at
   invocation time; exactly-once transport delivery does not prove an action
   executed exactly once. A semantic view cannot supply these guarantees.
   The Binding must qualify its mapping against the WoT operation contract.
5. **Graph/features limit:** reverse edges are explicitly refused by the probe;
   named graphs, imported/propagated contexts and arbitrary core aliases are
   not qualified. A complete JSON-LD processor underneath does not establish
   that its public projection exposes every feature correctly.
6. **Context/resource limit:** an unavailable root context prevents resolution;
   an unbounded remote dependency chain cannot fit a finite embedded budget.
   Input caps and bounded member queries do not bound PyLD's preceding
   allocations/expansion. This code supplies no strict preprocessing guarantee.
7. **Whole-document/operand limit:** additional responses, schema references,
   cross-affordance platform rules, URI templates, per-call parameters and
   credential alternatives are not demonstrated. A Form-only fixed record
   cannot claim their sufficiency. Full document scope/link access and typed
   WoT projections are proposed remedies, not executed proof.
8. **Reuse/ownership limit:** two instances with different broker credentials
   or tenant topics cannot safely share a program specialized on the first
   instance. An immutable payload or matching digest does not fund its final
   owner or isolate a late callback. No semantic representation proves these
   runtime properties.
9. **Role/activation limit:** the same unknown parent class can mean a domain
   category to one producer and an implicit nonce/authentication requirement to
   another. Under the tested descriptive contract, the unmarked input compiles
   to ordinary HTTP. An explicit dialect activates the parent-context-dependent
   requirement and blocks that fallback. This proves a conditional distinction,
   not correctness for unconstrained legacy class implications.
10. **Profile/annotation limit:** a standard `profile` claim is preserved but
    rejected by the original examples; stripping it can omit required response
    behavior. The added read-slice interpreter prevents that omission by fixing
    requirements before selection. It does not qualify whole-profile execution,
    mixed-scope profile composition or ignoring unknown profiles. A preserved
    `rdfs:label` still prevents both mappings at parent scopes; the class-only
    rule does not provide arbitrary annotation-predicate interoperability.

The closed security discriminator above is a separate source-level
counterexample to assuming all future vocabulary can enter through today's TD
types. Generic extension/security description preservation must precede any
new SPI. Unknown authorization requirements still fail unless an interpreter
and provider can satisfy them; preservation cannot authorize access.

### Evidence classification and recommended decision

**Proven feasibility:** Within the probe's explicit property/JSON/NoSec input
profile, one resolved public interface carries enough information for HTTP and
MQTT lowering, including alternate term spellings, scoped resolution, native
defaults and generic applicability/selection diagnostics. Domain types compose
with both mappings under either the historical exact-list experiment or the
new shared descriptive-class rule; the latter needs no application/domain IRI
inventory. The fixture dialect can use preserved parent context while ordinary
HTTP refuses its explicit selector. Requested property/Form/Binding
constraints exclude unrelated lowerers before compilation. The Engine-side
resolver and selector contain no HTTP/MQTT vocabulary branches. This establishes
constructive expressibility for those cases, not general protocol independence.
The standard-profile experiment additionally fixes one opaque read obligation
independently of candidate order, excludes incomplete native coverage, and
selects a corresponding native definition. This is operation-slice evidence,
not full W3C profile conformance or a general profile implementation.

**Proven limitations/counterexamples:** The existing compiler input is lossy by
source inspection; raw source lending does not discharge context interpretation;
the tested shallow-presence projection collides on ordered source data; unknown
meaning and unsupported feature/resource cases cannot be repaired by naming
the interface “semantic.” The hidden class-obligation counterexample excludes
unconstrained operational class implications from the descriptive contract.
The negative suite intentionally records these limits.

**Unverified assumptions:** The selected list/JSON-literal profile covers enough
real mapping vocabularies; a complete nested source overlay, if needed, can
remain small and ergonomic; generic graph/reference access will cover relevant
future bindings; strict resolution can meet actual embedded limits; generic Rust
Host/static adapters can hide accounting without obscuring physical ownership;
definition factoring saves meaningful work. Neither existing ADRs nor passing
fixtures establish these assumptions. Generic classification of unknown
annotations versus executable instructions is impossible without a contract or
additional trusted interpretation. The proposed base contract's compatibility,
admission qualification, activation/composition, conflict diagnostics and shared
interpreter ergonomics remain unverified. No role metadata framework is built.

**Proposed architectural decisions requiring Owner approval after independent
review:**

1. Adopt Engine-owned WoT/context resolution plus Binding-owned protocol
   interpretation as the replacement direction. Remove raw TD/context access
   from its ordinary lowering contract. Explicitly accept a declared semantic
   feature envelope and refusal of unknown executable meaning. Preserve admitted
   extension data before typed parsing rejects/normalizes it; do not promise
   automatic support for future vocabularies or security requirements.
2. Select bounded queries over structured resolved data rather than a fixed
   universal Semantic View, protocol-field union or reasoning engine. Start
   with the explicit list/JSON-literal feature profile and loss diagnostics;
   require a real syntax-sensitive mapping and preservation evidence before
   adopting a complete source overlay.
3. **Remove the mandatory persisted `LogicalInteractionPlan` as a compiler
   bottleneck. Redesign the compiler SPI** around resolved input and native
   pure output. Retain minimal Engine execution facts and their authority.
   **Retain paired startup registration, redesign its openness/adapters** and
   remove concrete compiler eligibility. **Retain Plan/Compiled Program
   responsibility separation, redesign its persistent containers/lifecycles**;
   no universal program or mandatory reuse layer is selected.
4. Keep native applicability with the Binding and final Consumer selection/
   Producer coverage with the Engine. Separate unsupported alternatives from
   fatal failures. Keep Host deployment contracts distinct from strict embedded
   qualification, with one semantic contract and optional preparation tooling.
5. Prefer the minimal common-rule/explicit-operational-profile responsibility
   model described above, subject to qualification of its admission and activation
   contract. Parent extension classes are descriptive by contract; selected
   operational interpreters may consume their preserved context. Unknown
   instructions remain unsupported. Optional shared vocabulary/profile libraries
   belong with their maintainers, not an application allowlist, per-Binding
   domain inventory or central protocol registry. Use `Thing.profile` claims as
   preparation inputs to installed trusted interpretation, separately retaining
   the advertisements; fix scoped active requirements independently of native
   candidate choice and verify coverage before selection. Accepting exclusions
   for undeclared implicit obligations, unknown profile interpretation and
   unfamiliar annotation predicates is the residual product decision. No
   production role/activation API is approved here.

The smallest independently reviewable next step is **one isolated semantic
contract refinement**, before any production SPI migration. First independently
challenge admission/activation, the class-only compatibility limit and the
smallest shared interpreter contract using an existing mapping with parent
semantics. If that direction survives review, specify the supported value/container
profile and test that dropped or malformed nested instructions cannot silently
become defaults; add linked response/schema and one URI-template/per-call-operand
example; and have a fresh reviewer author another lowering against only the
public contract. Measure resolver work/storage and author-side helpers; retain
failures as counterexamples. Challenge whether real mappings require the broader
source overlay. Only then specify a small Rust semantic-access/compiler
vertical slice with generic registration; runtime ownership migration remains
separate. Do not reopen #210 or activate old gates to shortcut this evidence.

This investigation changes only this workspace topic and its disposable probe.
It does not adopt a replacement architecture, modify production SPI contracts,
admit implementation, change milestone/gate states, or authorize a merge.
