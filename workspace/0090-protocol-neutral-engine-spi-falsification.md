# 0090 Protocol-neutral Engine/SPI extensibility falsification

Status: OPEN

## Question and decision boundary

Can ClinkZ-WoT implement a stable **protocol-independent semantic Core, Planning
and Servient** such that a new protocol is integrated primarily by authoring a
new Protocol Binding crate against the existing public SPI, **without adding
protocol-specific interpretation, lifecycle branches, or dispatch paths to
those shared layers**?

This is a feasibility question about the product's central architectural
promise, not a request to make Core immutable forever. Adding a new WoT
interaction *capability* may require a separately reviewed shared semantic
extension. Adding a new *protocol implementation* of an already supported
capability should not, by itself, require such an extension.

The project Owner asks for an adversarial, repository-grounded answer **before
assuming continued broad Runtime expansion is justified**. Neither the
protocol-neutral architecture nor its failure is predetermined. This topic
authorizes no implementation, SPI rewrite, gate transition or roadmap change.

## Why this is a new question, not a reset of prior decisions

- [Topic 0042](0042-protocol-binding-spi-validation-risk.md) already decided
  what levels of SPI evidence and independent implementability feedback matter.
  [Topic 0056](0056-target-spi-external-validation-and-cross-protocol-neutrality.md)
  selected an early real Zenoh feedback probe and warned that Zenoh/pico are
  one protocol family, not proof of shape neutrality. Their settled findings
  must be reused, not restaged.
- The accepted [SPI specification](../docs/spec/binding-spi.md),
  [deployment architecture](../docs/architecture/40-protocol-binding-spi-and-deployment.md),
  [planning authority](../docs/spec/planning.md), and
  [module boundaries](../docs/architecture/20-module-boundaries.md) already
  assign transport parsing, I/O, native correlation/multiplexing and
  protocol-local state to bindings; Servient retains publication, handler
  dispatch, admission and cleanup authority. The target SPI remains
  startup-registered, Cargo-linked and capability-scoped.
- [ADR-0022](../docs/ADRs/0022-first-production-binding-and-runtime-environments.org)
  selects Host HTTP as the first production protocol, deferring Zenoh/pico;
  [ADR-0023](../docs/ADRs/0023-resource-ownership-and-policy-boundaries.org)
  separates engine-controlled from binding-controlled resources.
  Host/embedded physical implementations need not be identical.
- After [PR #203](https://github.com/yushun1990/clinkz-wot/pull/203),
  the admitted narrow HTTP Consumer Property Read tranche is complete.
  It establishes a real HTTP operation, not extensibility across independent
  protocol shapes, broad operation families or embedded suitability.
- The **unmerged** [PR #204](https://github.com/yushun1990/clinkz-wot/pull/204)
  records a concrete Producer-admission counterexample: Basic-valid HTTP Forms
  with different eligibility/security/coverage meaning can produce the same
  narrow logical projection. That finding is a reason to test information
  preservation, not proof that a generic compiler boundary cannot work.
  Its independent acceptance, correction and source status remain owned by
  that PR/topic, not by this investigation.
- [Topic 0076](0076-composable-capabilities-and-runtime-boundaries.md)
  addresses capability packaging, [topic 0081](0081-reusable-plan-and-thing-identity.md)
  reusable definitions versus per-Thing authority, and
  [topic 0082](0082-logical-physical-subscription-decoupling.md)
  logical versus physical broker subscriptions. Their unresolved questions
  intersect this study, but this topic does not select their solutions.

The missing post-HTTP question is **extension locality**: does a
meaningfully different, independently authored binding fit the existing
semantic and execution boundaries without shared code learning that
protocol's vocabulary or special execution topology?

## Evidence to reconstruct first (no presumed defect)

1. Trace the actual current **public production path**, from TD validation
   and effective Form semantics through Planning, closed/open compiler
   support, selected artifacts, registered binding owner, Servient
   execution, output validation and shutdown. Distinguish installed target
   paths, mock fixtures, legacy behavior and merely deferred specifications.
2. Inspect `core/src/plan.rs::LogicalInteractionPlan` (currently a narrow
   Property Read projection), `core/src/binding_compiler.rs::BindingCompilerInput`
   and `BindingCompilerExtension`, the actual Consumer compiler-support
   restriction, Producer planning, `core/src/binding.rs` client/server SPI,
   and the corresponding Servient runtime paths. An open Rust trait is
   **not** proof that an arbitrary third-party compiler can be admitted by
   the installed production registration.
3. Determine whether the compiler receives enough **validated, effective,
   protocol-significant** information to accept/reject the selected Form
   and produce a private artifact. Test information loss separately from
   protocol support: a correct pre-I/O refusal is valid; silently ignoring
   unsupported Form/security/operation/route meaning is not.
4. Distinguish an invariant semantic obligation (e.g. generation validity,
   no pre-publication handler dispatch, result validation, explicit cleanup)
   from a lifecycle *mechanism* peculiar to an HTTP listener, broker session,
   request/reply responder, retained subscription, or register-oriented
   industrial device. Identify costs of imposing a common phase structure.
5. Record what real evidence actually establishes about public authoring,
   native protocol progress, resource/disposal ownership, multi-route
   operation and independent third-party crate composition. Mock gate
   acceptance alone cannot establish a general protocol-neutral engine.

## Proposed falsification property

For a **previously supported WoT semantic capability**, a new independently
authored protocol binding should be able to:

- express its honest capability limits and reject unsupported interaction
  Forms before externally visible effects, without changing shared defaults,
  losing security/extension facts or inventing implicit operations;
- compile a bounded private protocol artifact using public inputs and
  registration, without Core/Planning/Servient understanding protocol-specific
  addresses, headers, QoS, registers or correlation rules;
- execute and retire the operation using protocol-local SDK state, transport
  flow control, sessions and native multiplexing while satisfying the existing
  observable semantic/ownership invariants;
- preserve correct source attribution, handler authority, result/error
  classification, cancellation and generation-scoped cleanup, without a
  protocol-name conditional or hidden direct handler path in shared layers;
- participate in a normal external Cargo dependency/registration composition,
  without private engine APIs, a patched shared crate or a test-only substitute
  for a production compiler/route/Runtime entry.

Core/Planning/Servient may evolve for a **genuinely missing shared WoT semantic
capability** or to repair an existing general contract. Such a change must be
identified explicitly and tested across previous bindings; it must not be
camouflaged as a routine new-protocol adapter. An extension can also legitimately
report `UnsupportedOperation` when the protocol has no semantically valid
mapping. Full fidelity to every native protocol feature is **not** the goal.

## Contrasting cases to challenge the claim

Choose minimal contrasting slices by *mechanism*, not a protocol-count
checklist. Prefer external/public authoring with no source modifications;
where unavailable, use a read-only contract mapping and identify the exact
missing public entry. Do not silently treat proposed code as shipped evidence.

| Shape | Narrow falsification case | Boundary under pressure |
| --- | --- | --- |
| HTTP request/response | The already completed Consumer Property Read, plus PR #204's scoped Producer Form/coverage counterexample | Form metadata preservation, selected route, response ownership and admission |
| Query/request-reply (e.g. Zenoh) | Same **Property Read** semantic capability using the actual target SPI, where it is supported | SDK correlation, request ownership, responder route lifecycle, cancellation and native progress |
| Register-oriented (e.g. Modbus) | Same Property Read semantics with register address, decoding/endian rules, serial/shared bus and read failure | Protocol-local addressing versus logical plan, resource sharing and non-HTTP error semantics |
| Broker push (e.g. MQTT or Zenoh pub/sub) | A separately admitted **ObserveProperty/Event** long-lived capability, not forced to masquerade as Property Read | Logical versus physical subscriptions, native session/push, independent stop, source mapping and backpressure |

The first three cases test **protocol substitution at fixed semantic scope**.
The fourth tests **semantic-family extensibility** and must be marked
unimplemented/unproven until its separate long-lived domain entry and
minimum ObserveProperty proof are admitted. It is not a prerequisite requiring
a full MQTT binding now. MQTT request/reply conventions are not assumed to
equal native HTTP GET; a valid broker-only refusal is evidence of an honest SPI.

Where a real protocol implementation is too expensive for this question,
record a precise code-level public API impossibility, a minimal external
compiler/authoring discriminator, or a credible executable prototype with
its provenance. Neither a paper compatibility table nor a mock-only positive
is a production neutrality proof. Avoid universal architecture claims from
one tested application, one operation or one runtime profile.

## Questions the independent technical assessment must answer

1. Can the existing `LogicalInteractionPlan` and compiler input retain
   validated protocol-significant information **without leaking every
   binding's vocabulary into shared Core**? Does the current
   `additional_form_metadata` projection indicate a narrow admissible
   restriction, a systematic expressiveness problem, or both?
2. Is compiler openness real for an external Binding's full installed
   registration, or only at the trait/fixture level? Identify each closed
   support, identity, representation or configuration prerequisite.
3. Which Producer prepare/readiness/activate/commit/permit/accept transitions
   are protocol-independent safety requirements, and which become unnatural
   or incorrect when applied to broker subscriptions, register buses and
   protocol-native responders? Preserve publication and handler authority.
4. Which semantic results and error/cancellation guarantees apply across
   these models? Which protocol effects are inherently unobservable or
   nonreversible, so shared wording would overpromise?
5. Can subscriptions or shared protocol infrastructure have independent
   physical lifetimes without multiplying transport owners by Thing or
   logical subscription count? Keep topics 0081/0082 distinct from this
   decision.
6. Would a **narrow, validated effective-Form projection with binding-private
   compilation**, a distinct execution-family SPI, the current unified route
   path, or another boundary better satisfy the invariant? Do not preselect
   any mechanism; compare costs, lost guarantees and impact on completed
   evidence before proposing changes.
7. Does the evidence support (a) retaining the current architecture,
   (b) one bounded shared-boundary correction, (c) separating semantic Core
   from protocol execution-family orchestration, or (d) narrowing the
   project/product claim? What would falsify each alternative?

## Decision criteria and output

The investigation should produce a **reasoned disposition, not a new
architecture specification**:

- **Reaffirm** only if public inputs, support admission, lifecycle and
  execution can accommodate a materially contrasting fixed-capability
  binding without protocol-specific shared changes, with honest limits on
  what real execution has proved.
- **Scoped correction** if a concrete, generally applicable shared
  information/authority boundary is defective but can be repaired without
  turning Core into a protocol registry of special cases.
- **Architectural reassessment** if every credible extension demands
  protocol-specific shared state, mandatory lifecycle emulation, hidden
  cross-layer callbacks or unacceptable duplication/ownership inversion.
- **Insufficient evidence** if mock or paper reasoning cannot distinguish
  alternatives. Specify the smallest independent external experiment and
  the explicit success/failure observations instead of claiming neutrality.

Record for each counterexample: supported WoT capability, target protocol
behavior, first public boundary that fails, exact source/API evidence,
what can be implemented inside the binding alone, whether a generic
shared-semantic correction is justified, and whether the finding is
already tracked by an existing topic. A complete answer must also state
the **strongest counterargument** to its preferred conclusion.

Any selected change must identify affected authority owners, active
requirements, previously accepted gates/tranches, source/feature/profile
cells, and evidence requiring independent reaffirmation. Follow
`ARCHITECTURE_GOVERNANCE.md` and ADR-0013 for a separate decision/migration
and implementation admission. Do **not** automatically reopen the completed
HTTP Consumer qualification or PR #204's independent review.

## Non-goals and scope protection

- No production Rust edits, binding migration, new general framework,
  code generator, Cargo dependency, persistent state database or new gate
  in this Proposal PR.
- No requirement to ship HTTP/Zenoh/MQTT/Modbus simultaneously; no claim
  that all protocol-specific features must become portable WoT operations.
- No weakening of security applicability, semantic validation, ownership,
  finite explicit cleanup, generation fencing or bounded engine capacity
  merely to make a mismatched transport compile.
- No mandate that Host and constrained physical implementations share
  the same scheduling, storage or SDK environment.
- Do not conflate feasibility with performance. Once an honest cross-protocol
  shape exists, benchmark the marginal hot-path cost against the same native
  SDK and workload separately; architecture fixtures and quota numbers
  are not throughput measurements.
- Do not replace existing 0042/0056 conclusions or conflate this issue with
  the separate Plan-reuse and subscription-sharing topics.

This topic remains `OPEN` until an independent technical assessment
supplies evidence and a bounded direction. A later decision belongs in
its authoritative owner and a separate reviewed PR; passing this
Proposal review alone grants neither implementation admission nor a
protocol-neutrality product claim.
