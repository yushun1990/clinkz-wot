# 0056 Target SPI External Validation and Cross-Protocol Neutrality

Status: DISCUSSING

Kind: owner-raised architecture investigation

Priority: HIGH

Target: conditional protocol independence for already supported WoT semantic capabilities and the evidence boundary of the public Binding/Servient SPI

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
retain a `LogicalInteractionPlan` for every candidate. Compile directly from the
resolved view into an immutable entry containing only engine-used semantic
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

### Consumer inline-lowering implementation cut

The implementation chooses the existing Consumer transaction as the first open
lowering cut. Remove the exact compiler downcast/private-adapter eligibility;
attach qualified inline callback costs and actual generic layouts to the paired
complete owner. Forward the selected original source/context while TD's paid
cursor retains the immutable loan. Compile sequentially under existing capacity
and rollback roots, then destroy source-bearing state before publication.
The Binding/Planning specifications and original work-package owners carry the
exact replacement contracts; their original acceptance does not accept this cut.

Removing only downcasts leaves source distinctions erased. A Producer preflight
repairs unsafe coverage but leaves Consumer lowering closed. Another storage
repair has less direct extension value after #209. A complete Producer driver
rewrite combines independent ingress/session/publication risks with compilation.
This cut exercises an independent compiler/executor through both installed
Consumer runtimes now, without selecting those later mechanisms. It deliberately
retains the engine-used logical plan until a smaller execution entry replaces
its actual consumers; no duplication-per-binding premise justifies that work.

The finite external binding probe distinguishes locally aliased READ/FETCH
instructions, owns targets beyond 64 bytes, executes after TD destruction, and
rejects an unknown later instruction with earlier outputs rolled back. This is
an executable extension boundary, not real-wire conformance or universal
callback qualification. Allocating native lowering, supported-alternative
selection, optional identity/empty-name correction, parameterized definitions,
Producer source/coverage/driver replacement, registration/session retirement,
native route overlap and legacy/public-facade removal remain fundamental work.
