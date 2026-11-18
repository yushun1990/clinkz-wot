# Engine lifecycle and implementation proposal

Status: DISCUSSING

Separate reusable compilation, qualification of actual services, instance binding,
and admitted work. Keep native matching, continuation formats, transport and
physical infrastructure in each Binding. Use the existing finite and Action
kernels where their transitions fit. Extend the ownership rules across families
without requiring a common program format or execution owner.

The source baseline is `d6a4c86b2629a930b5337913844d14a8fd3cfcf7`. PR #288
qualifies finite Consumer support once per revision and adds a bound Property
Read with shared destination backing. That path is the starting point for the
finite Consumer, alongside independent Producer participation and caller-owned
Read and Action execution already on master. Implementation starts after this
design and plan are accepted. [PR #288](https://github.com/yushun1990/clinkz-wot/pull/288) ·
[Current ownership direction](../docs/ADRs/0025-semantic-engine-and-binding-runtimes.org)

## Source audit

| Path | Confirmed Engine costs and constraints | Work execution still owes |
| --- | --- | --- |
| TD preparation and selection | The finite Consumer, continuing Action and notification entry points each start basic TD admission and walk Forms. Preparing all three from one immutable TD repeats that input work. Finite and Action already share a semantic arena within a Form. Producer `visit_thing` resolves a Form inside its effective-operation loop. Composed Producer preparation, staged admission and publication each parse and walk the description. Some `supports` probes produce temporary lowered definitions. [Finite](../planning/src/open_consumer.rs#L298) · [Action](../planning/src/open_consumer/action.rs#L65) · [Notification](../planning/src/open_consumer/notification.rs#L108) · [Producer walk](../planning/src/producer_composition.rs#L258) · [Staged admission](../servient/src/producer_composition.rs#L545) · [Publish](../servient/src/producer_composition.rs#L702) · [HTTP Action provider](../protocol-bindings/protocols/http/src/action_preparation.rs#L115) | Each distinct input needs admission and the selected Form's complete semantics. If the final Producer TD, endpoint or provider set changes, every affected assumption must be qualified. Complete coverage and detection of new ambiguity are required even when the draft compiled successfully. |
| Finite Consumer | Planning calls access policy during candidate selection and returns `ConsumerBinding` entries already paired to access and a bank. Servient attaches authority separately. Name-based calls scan the association and resolve fixed targets on each start. A retained `ConsumerPropertyRead` avoids those steps. HTTP still reconstructs the destination to verify agreement with the authorized destination. This is a safety check required by the separate inputs of the current SPI. [Planning selection](../planning/src/open_consumer.rs#L566) · [Association selection and bound Read](../servient/finite/src/consumer.rs#L1010) · [HTTP execution](../protocol-bindings/protocols/http/src/lib.rs#L277) | Fresh actual destination and member-set permissions, credential issuance, native correlation, output classification/validation and unread-result checks. A root WriteMultiple with a different member set is different work under the same selected mapping. |
| Continuing Consumer Action | The endpoint-specialized definition already shares Action schemas. New Invoke requalifies the complete Invoke/Query/Cancel set. Progress and handoff ask again about participating schema/security support and control compatibility. Comparison with the original declaration is necessary, yet these paths also repeat support predicates and resolve destinations for access checks. HTTP start reconstructs the destination again. [Core definition](../core/src/consumer_action.rs#L122) · [Host checks](../servient/finite/src/consumer_action.rs#L46) · [Invocation and controls](../servient/finite/src/consumer_action.rs#L578) · [HTTP mapping](../protocol-bindings/protocols/http/src/action_consumer.rs#L104) | Query/Cancel must use the original invocation and compatible independently selected mapping, including across Bindings. Actual continuation operands are late. Completed output uses the returning operation's representation. The Invoke native dependency can outlive its input codec. |
| Consumer Notifications | The TD-prepared adapter walks schema roles and invokes native `support` from checks around policy and validation. The raw subscription entry point also probes support. A steady SubscribeAllEvents delivery can revisit complete schema support even though only one original member's data is delivered. Both this and Action recheck support without capturing the data service's qualification revision as finite does. Equal resource declarations alone cannot identify changed qualification. [Definition](../core/src/consumer.rs#L612) · [Captured services](../servient/finite/src/consumer_notification.rs#L82) · [Raw subscription](../servient/finite/src/notification.rs#L492) · [Finite revision capture](../core/src/consumer.rs#L903) | Actual occurrence identity, member schema, optional exclusive response right, queue/loss policy, and fresh recipient permission. For mappings that require authenticated sources, policy needs verified evidence for the actual occurrence. A topic or named Event coordinate only identifies a mapping. |
| Producer finite ingress, Actions and emission | Finite dispatch scans the Servient's instance and behavior vectors. Action invoke/control, native-failure reconciliation and notification emission similarly repeat semantic lookups. Action progress for one participation prepays the global invocation ceiling and scans the store. QueryAllActions searches that same store for each native inventory member. Servient's `pair_is_current` asks for `additional_validation_work`, which can traverse the HTTP mapping/schema inventory. Input, response-contract and output callbacks each retrieve the same accepted mapping by index and recheck the candidate. [Finite dispatch](../servient/finite/src/host.rs#L504) · [HTTP validation](../protocol-bindings/protocols/http/src/finite_producer.rs#L570) · [Action progress and dispatch](../servient/finite/src/action.rs#L504) · [Emission admission](../servient/finite/src/notification_publication.rs#L248) · [Failure reconciliation](../servient/finite/src/publication.rs#L415) | A native candidate is evidence for lookup. Servient still decides acceptance for the current issuer, incarnation, operation, behavior and policy. A root status query must visit the actual qualified inventory, preserve correlation and check per-member visibility. Emission must validate the actual payload and hold the correct native opportunity. |

Two capacity constraints survive logical independence. The finite Producer's
shared capacity separates active credit from retained outcomes, but each
participant still owns a one-exchange state machine. A small dedicated opportunity
is a valid deployment choice. A native Binding with several concurrently usable
reply opportunities must be able to hand them off without one Engine participant
per exchange. Managed MQTT Producer participation also budgets emission state,
payload and validation scratch in each publication's backing before any
emission. This ties idle exposure funding to work that could draw from an
independent emission capacity. Host Producer instance/behavior ceilings are
currently 32/16, and raising them increases the prepaid cost of an unrelated
finite dispatch even if it hits the first entry. Counting actual publication
members for setup/retirement is legitimate. Charging by unrelated instances or
invocations for a known mapping constrains scale. The managed finite Consumer
already uses funded indexed registration. [Finite runtime](../servient/finite/src/host.rs#L800) ·
[Shared work bank](../servient/finite/src/producer_capacity.rs#L35) ·
[MQTT declared backing](../protocol-bindings/protocols/mqtt/src/native.rs#L481) ·
[Producer Action progress](../servient/finite/src/action.rs#L645) ·
[Consumer registry](../servient/finite/src/consumer_managed.rs#L138)

Cleanup also inherits avoidable execution dependencies. Finite progress requests
permission credit even after a passive fence has recorded a local cause.
Continuing Action progress prepays permission, native, cleanup and applicable
codec work together. A revoked call can wait for permission credit despite having
the credit and storage needed to close its native owner. Late output validation
still needs its original codec and work. Native release can proceed while that
validation waits, with the original evidence retained. [Finite transition](../core/src/execution.rs#L235) ·
[Action work prepayment](../core/src/execution/action.rs#L196)

Caller-owned Read and continuing Action borrow source-free meanings and concrete
native facets and reuse the Core transitions. Borrowed Read still repeats
schema and native support checks around access callbacks. TD-driven preparation
uses Host owners, and Producer and notification execution still enter through
the Host module. Producer Action retention uses an internal `Instant` even
under the optional Host turn. These are current representation boundaries.
[Borrowed Read](../servient/finite/src/borrowed_consumer.rs#L68) ·
[Borrowed Action](../servient/finite/src/borrowed_action.rs#L80) ·
[Execution exports](../servient/finite/src/lib.rs) ·
[Producer Action time](../servient/finite/src/action.rs#L695)

Repeated qualification during execution, idle publication charges for active
work and dispatch that scales with unrelated records are interface defects.
Finite association installation's prefix comparisons for duplicate detection
are a smaller implementation cost. The retained Producer finite
FIFO traverses unread entries to append. These are bounded representation
choices, addressable when those owners change. HTTP serialization, MQTT filter
matching and distribution, genuine per-instance collision checks and native
seriality under a Binding's own policy belong at the native boundary. Fixed
Host capacity sweeps remain reasonable for small configured stores. The scaling
claim to remove is work proportional to unrelated idle Things on a selected
execution path. [Association construction](../servient/finite/src/consumer.rs#L1477) ·
[Finite outcome queue](../servient/finite/src/producer_capacity.rs#L232) ·
[Native MQTT participation](../protocol-bindings/protocols/mqtt/src/consumer.rs#L624)

## Target design

The separation follows whose facts can change. TD 1.1 assigns meanings to Forms,
operations, schemas, security and URI variables. The native mapping decides how
those meanings execute over a protocol. Local policy decides who may act on a
particular Thing, invocation or occurrence at a particular edge. The Engine
needs explicit ownership at those boundaries. The W3C Architecture and TD 1.1
are Recommendations. Binding Templates is a Group Note. [WoT Architecture 1.1](https://www.w3.org/TR/wot-architecture11/) ·
[TD 1.1 Forms and URI variables](https://www.w3.org/TR/wot-thing-description11/#form) ·
[Binding Templates](https://www.w3.org/TR/wot-binding-templates/)

| Phase | Product of the phase and its owner |
| --- | --- |
| Prepare and compile | TD/Planning admits one immutable input revision under a defined interpretation/profile and lends normalized, indexed operation facts to requested selectors. Share interpretation within that revision across family and role entry points where they read the same facts. Keep distinct roles, complete response alternatives, security branches, and mapping-relevant extensions. The concrete Binding may lower a separately owned immutable definition over the full set of invariant inputs it uses. Source loans are temporary. Deferred preparation needs owned normalized inputs, bounded work and rollback, and completes before dependent acceptance. |
| Qualify and bind | Prove support for the actual registration, codec/validator and security service. Record the exact owner, compatibility/configuration, immutable work/storage bounds and revision of each relied-on facet. Construct a bound operation from a selected definition, checked local operands and original Servient authority, behavior or access service and accounts. Apply any instance installation policy here. Resolve a fixed destination once per installed mapping, eagerly or at explicit bind. Public operation references are non-revoking views of that installation. |
| Execute | Use the selected bound operation, actual request or occurrence, and separately acquired capacity. A named convenience call selects from an instance's bounded operation index and enters the same path. For dynamic targets, resolve a bounded native operand from the actual variables or continuation. Policy inspects its destination and native start uses that same operand. Ingress carries candidate evidence and its original reply opportunity. Servient validates it and captures behavior before acceptance. |
| Retire | The logical owner fences new acceptance and delivery. Admitted work and provisional native setup retain their original owners, operands, first cause, late evidence and cleanup funding. Drive local close independently of new permission or unrelated admission credit. Keep remote effect, cancellation, native delivery, local release and result acknowledgement observable separately. Release backing in its original account when the final retaining owner disposes it. |

### Qualification, authority and reuse

A prepared definition may specialize an endpoint or program layout. A reusable
portion owns the complete immutable dependencies of its lowering and declares
the operands its Binding can check later. Extract one normalized input for both
lowering and compatibility comparison, or use an equally complete typed witness.
Consider operation and role, defaults, media and response roles, schema
constraints, security mode and mapping-relevant vocabulary, compiler/format
compatibility and profile/configuration assumptions insofar as lowering depends
on them. Qualify actual trust and service owners at local installation. TD ID,
logical generation, provider slot and an account's identity only enter a reuse
comparison when the lowerer genuinely uses them. A mismatch refuses or requests
an explicit new compilation. Sharing the same specialized artifact with a
second association at the same endpoint remains valid. [PREP-REUSE-001](../docs/spec/planning.md) ·
[HTTP normalized compiler input](../protocol-bindings/protocols/http/src/definition/preparation.rs#L155)

Keep one small, passive dependency check for each facet used at an edge.
Capture qualification once, require explicit adoption of fixed or non-reused
changing revisions, and propagate invalidation through wrappers. Sample around
qualification and every relevant linked callback, then fence acceptance or
handoff using the original revision and actual owner. Availability and original
declared bounds remain independently observable. Mutable policy, credentials,
trust evidence and payloads still receive their paid checks at dependent edges.
Requalify changed services and reuse immutable instructions when their inputs
still agree. A replacement issues a new incarnation or configuration.
New Action invocations honor the control set required at binding. Admitted
Actions check only the original continuation, the requested control and
the actual data roles still in use. [Qualification contract](../docs/spec/planning.md#open-finite-consumer-implementation) ·
[Authorization and cleanup](../docs/spec/runtime-safety.md)

For native ingress, an opaque installed-operation reference can identify a
match the Binding already made. Scope it to the issuer, incarnation and
original participation, and validate it against operation and reply
correlation. A copied reference conveys candidate identity. Servient resolves
it and grants permission through the same checks as other candidates.
Ambiguous and absent matches follow the capability's declared refusal rules.
The Binding receives the accepted semantic output and its native opportunity
through the SPI. Native crates keep their dependency on the SPI, and Servient
retains behavior, the registry and its publication authority. [Ingress and delivery](../docs/spec/binding-spi.md#producer-action-candidates-and-native-participation)

Producer publication commits a complete final TD, current authority and exact
closed native member set together. Use the staged normalized proof again when
it still covers the exact final input and the selected and alternative
providers. For changed input or a new provider revision, requalify the affected
facts, including potential ambiguity. Use full final qualification when
dependency coverage is uncertain.
Check current membership, behavior coverage, actual readiness, service
qualification and collision results at the proper owner before the local commit.
Retain the resulting dispatch and emission references. Failure of an original
committed participation gives Servient evidence to revoke that publication.
Bindings continue to own partial setup, native refusal and physical retirement.
[Final coverage](../docs/spec/planning.md#reusable-producer-preparation) ·
[Publication and authority](../docs/spec/runtime-safety.md#host-producer-execution-turns)

### Capacity and execution representations

Size definitions by distinct compatible inputs, bound records by installed
instances and operations, and work by admitted obligations. A deployment can
use one or several work owners per family. Fund active native work, invocation
continuations, queued items and response rights, and retained results according
to their distinct lifetimes. Admission acquires all backing and cleanup needed
for the next effect before entering a callback. An invocation retains enough
to close locally even when a new control or result-delivery opportunity cannot
be funded. Return active credit at its settlement edge and keep original
continuation or unread/detached result credit until disposal. Keep reserves and
quotas distinct when a shared capacity and an instance both constrain admission.
Native participation may yield multiple independently owned finite opportunities
when its protocol and configuration support that concurrency. Preserve original
correlation through out-of-order completion. A serial Binding can offer one.

Order acceptance, revocation and handoff at the original Servient owner using
its exclusive loan or serialized state. Reserve credit and capture the
original work before the effect edge. A revocation that wins the edge fences
it. A Write already accepted may finish after revocation, leaving an effect
report and uncertain delivery under the old incarnation. A native start that
successfully creates work during a callback must retain that work even if the
callback also closes its authority. Passive fences are checked again before a
new effect or successful result handoff. Cleanup errors keep the complete owner
and original first cause. Drop can perform bounded local abort. If the
deployment transfers cleanup, the successor acknowledges the complete funded
owner before the transfer commits. [Acceptance and cleanup](../docs/spec/runtime-safety.md)

Retain the family-specific state machines. An Action invocation can survive many
exchanges, an observation has a cursor and may have per-occurrence response
rights, and a Producer exposure owns a set of native participations. Share
portable transitions and validation/authorization edges where they have the same
meaning. Host maps, erasure, queues and reentrant checkout are implementation
choices. Caller-owned storage uses typed native state, borrowed prepared facts,
lifetime-based authority and caller-supplied time, credit and memory. Use the
exact validated output loan at handoff. Permit explicit bounded step/poll
drivers and optional Host composition. Native readiness and maintenance
(deadlines, revocation, credits and cleanup) are both deployment progress
obligations. Preserve exclusive progress of each original obligation and keep
callbacks outside shared registry borrows. [Finite and Action kernels](../core/src/execution.rs#L173) ·
[Caller-owned execution](../docs/spec/runtime-safety.md#caller-owned-finite-read)

For 0081, the ownership decision is already migrated. The implementation test
is compile once for each compatible invariant definition and bind independently
for each instance. Hold the definition count fixed while varying endpoints,
policies and instances. Count actual lowerer entries separately from TD
validation, dependency comparisons, operand checks and native setup. Prove
reuse of the same funded storage through independent refusal, execution and
final disposal. The existing HTTP Consumer and Producer Read paths supply
controls for that experiment. Extend it to HTTP continuing Actions and a
supported MQTT notification mapping. Keep unsupported dependency/operand
combinations specialized with a concrete reason. Compilation reuse, logical
instance independence, independence from a particular executor, and
runtime/storage independence each need separate evidence. [0081](0081-reusable-plan-and-thing-identity.md) ·
[Consumer lowering](../docs/evidence/consumer-execution-reuse.md#explicit-http-definition-compilation) ·
[Producer lowering](../docs/evidence/producer-preparation-reuse.md)

## Ordered implementation plan

Each stage replaces the affected path at its normal entry point and includes
bounded refusal before effects, callback-time invalidation, short credit,
replacement, late/unread outcomes and final original-owner release where the
path exposes those edges. Use executable Rust with actual HTTP, MQTT or
downstream test Binding I/O and deterministic counts of actual work.

### 0. Accept and migrate the design

Review the proposal and the exact source evidence in a fresh architecture
review. On acceptance, project the decisions and supersessions into the
compiled-plan and runtime architecture, Planning, Core, Binding SPI and Runtime
Safety. Update Foundation only if an owner or funding contract changes. Give
the cross-domain decision an ADR, reconcile affected requirement/API projections
and WP-100/200/300/400, and use WP-600/700 for concrete Binding/integration
evidence. PLAN records durable outcomes and dependencies. Acceptance requires
one consistent authority for preparation, binding, execution and retirement
and a review disposition for changed evidence. This is the dependency for all
implementation stages. [Architecture change control](../ARCHITECTURE_GOVERNANCE.md#architecture-change-control)

### 1. Prepare once and qualify actual services

Depends on 0. Put shared TD admission and normalized operation facts under the
existing finite Consumer and composed HTTP Producer. Lend those facts to the
Action and notification selectors and keep their existing execution edges
during this stage. Move support and stable bounds into qualified definitions.
Use complete provider evidence from a bounded probe or from retained lowering
when support and lowering are inseparable. Replace unconditional replay of
identical staged facts with verified provenance and passive dependency checks.
Changed final input and closed native readiness still receive qualification.

Accept on a mixed TD with Property, Action and Event Forms, including a
multi-operation Form. The three preparation selectors share one admission of
that input. An unchanged final Producer TD uses staged evidence. A changed
endpoint, new provider ambiguity, unsupported response/security or foreign
coverage causes requalification or refusal before publication. Demonstrate
one immutable HTTP Read lowering across two compatible endpoints in each
role, with separate operand checks, accounts and final release.

### 2. Bind operations and decouple finite exchanges

Depends on 1. Install issuer/generation-scoped finite operation records that
carry fixed operands, selected services and bounded work. Migrate the existing
bound Read and name-based finite Consumer APIs to the same selected execution
path. Bind native Producer route matches to the installed dispatch reference.
Capture pending ingress, reply and outcome in funded exchange records. Configure
instance and operation storage independently of the dispatch bound. Make the
dedicated finite opportunity a capacity choice. Reuse one captured mapping
for validation and response metadata and use phase-specific work allowance
through native close and validation. Migrate raw entry points to checked
candidate construction where they exercise the same semantics.

Accept on HTTP Read and secure Write or collection execution with two
independent publications and shared native infrastructure. A retained Consumer
operation requires zero name scans or fixed-target reconstruction. A selected
Producer dispatch and passive dependency read do bounded work independent of
unrelated installed instances/mappings. Exercise configured Host counts beyond
the current 32-instance and 16-behavior ceilings. A downstream native
participant can offer two concurrent exclusive exchanges under shared capacity. Native
settlement returns active credit while a held result counts toward the
retained limit. Check foreign and stale references, exact selected members,
denial between validation and effects, late results and cleanup with no
remaining permission credit.

### 3. Bind continuing Action controls and index original work

Depends on 1–2. Migrate Consumer and Producer Invoke, Query, Cancel and
QueryAllActions to qualified per-operation records and an explicit Invoke/control
compatibility join. Bind fixed parts of the mapping at installation and compute
the actual continuation-dependent operand at the control edge. Retain original
invocation ownership, with indexed invocation identity and paid progress scoped
to the original participant's work. Use caller-qualified monotonic time for age
and retirement decisions. Migrate existing Host and caller-owned Consumer
drivers through the common Action transitions. Keep Producer behavior and native
reply ownership distinct.

Accept on two independent HTTP Things executing Invoke and later Query/Cancel
with shared invariant Action instructions. Verify endpoint and continuation
compatibility at bind, including refusal for a changed control template.
Exercise distinct Invoke/control Bindings over real downstream datagrams,
returning operation media, unused sibling failure, root inventory visibility,
short control credit and an unread late result after replacement. Initial
exchange retirement leaves the original invocation reachable. Progress for one
participant and lookup of one invocation visit only its own work. Verify
source-free Host and caller-owned outcomes at their supported scope.

### 4. Bind notification reception and emission

Depends on 1–2. Follow 3 for the first mixed-role rollout, with the notification
contract independent of Action controls. Prepare named member/control/response
roles and capture native support once per qualified mapping. A prepared
subscription enters the conversation owner with that qualified mapping. Bind a
Producer emitter to the registered source, services and native route. Move
logical emission/queue/result capacity and native session or frame capacity to
their enforcing owners. Migrate raw and TD-driven handoffs together where they
share conversation semantics. Pass verified original occurrence evidence to
policy when the mapping supplies it, with explicit refusal where it is required
and unavailable.

Accept on real MQTT observation/subscription and emission. For a compatible
SubscribeAllEvents inventory, lower normalized native member/filter meaning
once per role and bind it to two distinct valid topic prefixes. Check actual
prefix and member length limits at binding and select the actual member's
schema at delivery with zero support scans. Shared physical interest survives
one recipient's stop. Confirm live per-occurrence denial, overflow/loss,
explicit cancellation, native failure and final release. Exercise an exclusive
response opportunity and its late retirement through a Binding that provides it.

### 5. Exercise caller-owned lifetimes and consolidate

Depends on 2–4. Use the replacement contracts through source-free typed/
caller-owned execution for representative finite exchanges, continuing Actions,
notification conversations/emissions and closed Producer publication. Keep
profile support explicit. Retire obsolete Property Read entry points after
migrating normal callers, fixtures and examples. Preserve scoped historical
evidence at its original revision. Ship the supported Host paths together
through optional deployment-driven turns.

Accept on no-std/no-pointer-atomic compilation of supported portable owners and
real I/O under a caller-driven test loop. Exercise time and revocation,
accepted-work and unread-result pressure, late evidence, and close through
release for each distinct portable lifetime claimed. Demonstrate mixed
HTTP/MQTT Producer and Consumer progress while one native owner stalls. Verify
execution-only dependencies contain no TD or compiler and the downstream
Binding retains control of its native types and transport. Require fresh
independent acceptance for API/ownership changes and scope current evidence to
the actual paths exercised.

## Replacement and trade-offs

- Redesign `PreparedConsumer::into_execution` and the separate access-bound
  `ConsumerBinding` packaging so preparation need not materialize an instance
  binding. Fold the single-purpose `ConsumerPropertyRead` behavior into selected
  finite execution. Explicitly retaining a bound operation saves repeat lookup
  and fixed-operand work at the cost of a funded reference. A selector remains
  useful when the caller only has a name. [Current packaging](../planning/src/open_consumer.rs#L850) ·
  [Bound Read](../servient/finite/src/consumer.rs#L320)
- Replace repeated `check_dependencies` support probes and Producer
  `supports`-via-lowering with checked facts and scoped invalidation. Replace
  repeated publication `covers` scans, runtime route reconstruction and
  the global Action scan with captured, indexed ownership. Keep
  `PreparedThing`'s complete advertisement role and narrow
  `ClosedProducerParticipation` to the concrete
  membership/readiness/retirement and installation evidence it needs. This
  adds provider responsibility for accurate invalidation and bounded capture.
  A provider unable to establish complete reusable support qualifies by lowering
  during preparation. [Producer SPI](../core/src/publication.rs#L198) ·
  [Final checks](../planning/src/producer_composition.rs#L366)
- Retire the restricted whole-Plan-Set Property Read builders, their
  `InlineConsumerCompilerSupport` protocol, and route-stage serving APIs once the
  successors cover their justified invariants. This includes the single-Read
  `ProducerExecutor` path exposed as `Servient` and `StaticServient`'s
  route-stage façade. Retain useful typed or inline storage and explicit raw
  fixtures where they prove a live contract. Migrate pre-v1 callers directly. Shared
  reference counts and bounded indices add their own storage/work. A borrowed
  deployment can choose equivalent loans and small caller-owned tables.
  [Restricted adapters](../docs/spec/binding-spi.md#removed-general-contracts-and-retained-adapters) ·
  [Public exports](../servient/src/lib.rs)
- Prefer explicit ownership and checked bind across instances to implicit
  caching, interning after lowering, or universal programs/runtimes. Explicit
  reuse leaves selection and lifetime decisions with the caller. Specialization
  remains simpler for genuinely unique inputs. A cache would add eviction,
  invalidation, negative results, waiters and reclaim ownership. A universal
  scheduler would force a progress policy on unrelated native facilities.
  Evaluate these only against a concrete product case with fewer moving parts
  than the explicit path.

## Owner decisions

Accept the architecture and sequence in this PR before implementation. The
technical recommendation uses the current Host-first release scope and curated
source-free execution for the first portable proof. If a committed constrained
deployment requires TD admission on-device for v1, identify its memory,
transport and provisioning constraints. That changes the scope and ordering of
stage 5 and the supported preparation profile. Likewise, if authenticated
notification sources are a delivery commitment, identify the actual source,
trust model and protocol mapping. That determines the real verification evidence
required in stage 4. The ordinary MQTT fixture proves occurrence identity and
recipient policy under its declared NoSec subset. No further product choice is
needed to review the ownership model under the current roadmap. [Product scope](../PLAN.md) ·
[Notification scope](../docs/evidence/consumer-notification-composition.md)
