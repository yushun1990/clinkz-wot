# General SPI architecture: WoT semantics, execution authority and native resources

Status: MIGRATED — Owner accepted the target direction on 2026-10-24; successor
contracts are owned by [ADR-0025](../docs/ADRs/0025-semantic-engine-and-binding-runtimes.org)
and [the v6.0 authority map](../docs/spec/authority.toml). The proposal below is
preserved reasoning, not active authority or implementation acceptance.

This proposal recommends the architecture for a protocol-independent Rust WoT
Engine. It replaces this topic's earlier broad capability/package proposals;
[their original text remains in Git](https://github.com/yushun1990/clinkz-wot/blob/d67dba8fc9894546ac177f217ff41917dbb96aaa/workspace/0076-composable-capabilities-and-runtime-boundaries.md).
[Review 08](../docs/reviews/review-08-integrated-architecture-reassessment.md)
remains a historical, non-normative reassessment. This proposal changes no active
specification, ADR, implementation, admission, gate or milestone.

## Recommended decision

**Build a semantic Engine with separate execution capabilities, over
Binding-owned protocol runtimes.** The common boundary expresses WoT intent,
selected mapping, current permission, owned work and observable outcomes.
It does not prescribe a common network request, session, server or task model.

Adopt these responsibility decisions together:

- TD owns description meaning; Planning prepares supported interactions and
  checks coverage. Prepared execution data is distinct from permission to use it.
- Core defines protocol-independent interaction, validation, security and
  ownership contracts. Servient owns live logical instances, admission,
  authorization, application dispatch and retirement of accepted work.
- A Binding owns interpretation of its protocol vocabulary, native matching,
  conversations, I/O and physical infrastructure. Native progress can occur
  independently of a particular Thing's semantic acceptance.
- Finite Consumer interactions, notification conversations, Producer exposure
  and Discovery have different control contracts. No universal `execute`
  method or mandatory symmetric client/server pair is selected.
- Description/definition reuse, logical instances, operations and physical
  resources have independent identities and lifetimes. Sharing is permitted
  only through explicit compatible owners and leases; it grants no authority.
- Normalize shared semantics once and execute from owned prepared data. Permit
  bounded first-use preparation for Consumer partial support. Producer publication
  requires qualified support for every retained advertised Form/operation;
  only construction whose support is already established may be deferred.
  Neither role requires a universal compiled-program representation.
- Keep the current acyclic Core/TD/Planning direction and Cargo composition.
  Replace restrictions of the narrow Property Read adapters where necessary;
  do not wrap them into the general architecture through compatibility facades.

This preserves the goals in
[System Goals and Context](../docs/architecture/00-system-goals-and-context.md),
including all Property/Action/Event families, both WoT roles, discovery,
security and constrained execution. It makes no claim of support for every
future protocol or of currently implemented replacement capabilities.

## Basis: semantics first, mechanisms second

The first conceptual pass used the official specifications and the enduring
product goals before inspecting Rust implementation, active SPI contracts,
prior implementation proposals or Review 08. Its working notes remained
outside the repository. The model was then challenged against master
`d67dba8fc9894546ac177f217ff41917dbb96aaa`, accepted contracts, source,
existing tests and Review 08. Current architectural means described in the
goals document were treated as hypotheses, rather than product requirements.

The following distinction matters throughout this proposal:

| Basis | What it establishes | What it does not establish |
| --- | --- | --- |
| [WoT Architecture 1.1 §§6.5–6.7](https://www.w3.org/TR/2023/REC-wot-architecture11-20231205/#sec-interaction-model) | Affordances express state, processes and notifications; hypermedia controls describe their concrete activation. | A Rust SPI, compiler, scheduler, storage model or universal protocol shape. Its [abstract Servient implementation, §8](https://www.w3.org/TR/2023/REC-wot-architecture11-20231205/#sec-servient-implementation), is explicitly non-normative. |
| [TD 1.1 §§5–8](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#form) | Form operations, schemas, defaults, effective security, protocol mapping assertions and context extensions. | Guaranteed reachability, permission to act, support for all forms, or physical resource ownership. |
| [TD 1.1 §7.2/§8.3](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#protocol-bindings) | The target's URI scheme identifies the protocol; vocabulary extensions describe mapping details. HTTP-specific assertions apply to HTTP mappings. | A new ClinkZ protocol-matching vocabulary, mandatory-extension flag or Core protocol switch. |
| [TD 1.1 §7.3](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#adding-security-schemes) | Security schemes can be extended. Preserving a scheme and satisfying it are different capabilities. | Authorization from a TD or permission to downgrade an unsupported scheme. |
| [WoT Discovery Recommendation](https://www.w3.org/TR/2023/REC-wot-discovery-20231205/) | Introduction and exploration distribute descriptions, with security considerations and specified exploration services. | A Directory server inside the Engine or discovery as a Property operation. |
| ClinkZ product goals and this engineering decision | Bounded owners, deterministic selection, independent local authority, cancellation/cleanup truth and equivalent supported semantics across profiles. | W3C mandates for generations, permits, Rust traits, allocation budgets or package names. |

The [Binding Templates retirement notice](https://www.w3.org/TR/2025/NOTE-wot-binding-templates-20251104/)
redirects mechanism discussion to TD Bindings. The
[TD 2.0 Bindings chapter](https://www.w3.org/TR/2025/WD-wot-thing-description-2.0-20251104/#bindings)
and [Binding Registry](https://www.w3.org/TR/2025/DRY-wot-binding-registry-20251104/)
are useful direction and discovery aids, respectively a Working Draft and Draft
Registry. They do not supersede the TD 1.1 Recommendation baseline. A concrete
Binding documents which protocol/mapping specification and version it implements;
not every document listed by a registry is a W3C Recommendation.

### Fundamental constraints

1. **Description is not authority.** A TD id, endpoint, schema, compiled object,
   Rust type or numeric generation does not authorize execution or identify a
   live local owner. Anonymous Things and valid empty affordance names remain
   usable; the Engine supplies its own local identity.
2. **WoT semantics have one shared interpreter.** Common defaults, operation
   applicability, security-reference composition and schema meaning are not
   independently reinvented by each Binding. Protocol methods, topics, options,
   acknowledgements and native security exchanges stay outside semantic Core.
3. **Every accepted obligation has an owner.** Pending, cancellation, failure,
   partial installation and late success preserve the work and its release path.
   Replacing a slot never redirects old work to the successor.
4. **Logical roles do not determine network topology.** A WoT Producer may
   connect to a broker; a Consumer subscription may require a callback endpoint.
   One Thing is neither one connection nor one physical device.
5. **Support and failure are truthful.** Invalid description, unsupported
   combination, denied permission, exhausted capacity, failed communication
   and uncertain remote effect are different outcomes.
6. **Limits apply where enforced.** Engine admission bounds Engine retention
   and promised handoffs. Bindings bound native ingress/growth. Deployment
   owns OS/SDK failure containment. Numeric declarations alone prove neither
   transitive memory bounds nor preemption of linked code.

The exact generation/permit representation and the choice to keep bindings
startup-composed are ClinkZ decisions. The semantic/authority and logical/physical
distinctions are the enduring design principles behind them.

## Components and dependencies

| Component | Responsibility and boundary |
| --- | --- |
| TD | Preserve descriptions/extensions; inspect known TD structure; resolve shared defaults, form context, base/template and security/schema meaning. Produce bounded semantic views, retaining original coordinates for diagnostics. Interpretation of an HTTP method is not TD's job. |
| Planning / preparation | Build operation indexes and selected mapping data; coordinate Binding, codec, validator and security support; check actual compiler/executor/configuration pairing and Producer advertisement coverage. It performs no native I/O or credential acquisition. |
| Core | Define operation/target identity, data roles, common result checks, prepared validation/security execution contracts, authority/lease boundaries and observable lifecycle outcomes. It has no TD parser, protocol case or dependency on concrete Bindings. |
| Servient execution | Own consumed/exposed instances, current permission, publication, operation/invocation/subscription records, behavior dispatch, fair logical progress, diagnostics and cleanup ownership. Compose optional services without implementing their protocols. |
| Concrete Binding | Interpret its protocol vocabulary and mapping defaults; create native execution data; implement conversations, matching/captures, framing, correlation, native authentication integration, delivery and infrastructure lifecycle. It cannot choose an application handler or grant semantic acceptance. |
| Payload/validation services | Implement media decoding/encoding and evaluation of supported DataSchema constraints. Shared executable validation belongs below TD preparation and Servient; protocol/ecosystem payload adaptation may compose those services inside a Binding. |
| Security services | Resolve credential capabilities, verify evidence, establish principals and evaluate application policy. Core defines their neutral contracts; applications provision implementations. Bindings implement the wire/security-stack parts. |
| Discovery clients | Own introduction, exploration, retrieval and continuation workflows, with bounded results, cancellation and trust checks. Directory servers/storage remain outside this product boundary. |
| Application/platform | Link registrations, provide behavior and services, select storage/progress/limits, and own deployment, cutover and environment recovery. |

Dependencies, with `A -> B` meaning **A depends on B**:

```text
Core                        -> Foundation
TD                          -> Core, Foundation
Planning                    -> TD, Core, Foundation
Binding runtime             -> Core, selected native/platform/payload services
Binding preparation         -> Binding runtime, Planning, TD
Servient execution          -> Core, Foundation
Servient preparation facade -> Servient execution, Planning, TD
Discovery                   -> Core, TD, selected discovery adapters
Application                 -> selected capabilities and concrete implementations
```

There is no Core -> TD/Planning edge, including through optional features or
reexports. Planning and Servient do not name HTTP/MQTT/etc. Concrete Bindings do
not depend on Servient. Discovery may optionally reuse a Consumer adapter for a
Directory's described interactions; the shared Engine does not require that
path for every discovery mechanism. Codecs/verifiers can be composed services
without a new mandatory crate or a universal runtime backend.

The current package names are suitable. Complete the execution/preparation
feature boundary by moving physical output/reclaim owners to their execution
owner, rather than hiding an actual dependency behind a feature alias. Additional
crates are justified only by a concrete dependency or distribution need.

A third-party crate exports runtime registration constructors for its supported
facets and, optionally, preparation producing those same native program types.
The application supplies the complete registrations; generic Host erasure or
application-closed static types accept conforming downstream implementations
without a built-in protocol/compiler whitelist or workspace membership. Pairing
checks the actual executor's configuration, format and policy, not just a tag.
Registrations declare supported combinations, native progress/ingress limits,
visibility, delivery and retirement policy, and required codec/security services.
Runtime-only registrations retain no compiler. Missing roles require no dummy
components, and no Binding obtains an application-dispatch handle.

## Description, preparation, authorization and execution

These phases have different effects and validity conditions:

| Boundary | Data and result | Permitted effects |
| --- | --- | --- |
| Interpret description | Immutable input plus origin/base and supported context profile -> common semantic facts and structural diagnostics. | Bounded local work. Remote retrieval is a separate authorized workflow. |
| Prepare interaction | Operation/form facts plus actual registered capabilities -> selected native mapping, data/validation contract, security requirements and compatibility information. | Pure, bounded construction. No sockets, tasks, credentials or handler calls. |
| Admit current use | Instance, prepared mapping, call arguments, policy and capacity -> owned call/exposure/subscription and current execution grant. | Reserve retention/cleanup; acquire credential or verification capabilities as needed. Permission to attempt authentication is distinct from final authorization. |
| Perform | Owned grant/work plus dynamic operands -> Binding conversation -> validated semantic outcomes. | Authorized native I/O; permitted behavior dispatch only after inbound verification/authorization. |
| Retire | Revoke new work, cancel/drain existing owners -> complete, acknowledged ownership transfer, or explicitly retained residual obligation. | Bounded cleanup; final local release only when actual owners permit it. |

`consume` normalizes and indexes description meaning once. It need not lower all
native candidates eagerly: it can prepare an operation on first use through a
bounded, cancellable local job. The proposed default requires at least one
statically supported interaction and returns a per-operation support report;
a caller can require a stricter capability set. TD tooling remains usable with
no executable interaction. Missing credentials, current capacity and peer
reachability are runtime availability, not structural support certificates.
The report distinguishes coarse eligibility from support established by complete
selected preparation; a deferred Consumer candidate can still prove unsupported.
That partial-support rule does not apply to a published Producer description.

An operation cannot proceed until its complete selected mapping, validation,
security and necessary capacity checks succeed. Required preparation is not
optional; ahead-of-time compilation, caching and two-level plan storage are.
First-use latency and preparation failure are observable. A warmed execution
path uses indexed prepared data, never rescans a TD or reruns shared defaulting.
If optional caching is used, duplicate work/waiters and negative results need
bounded ownership; cached support never becomes a permanent permission.

First-use work belongs to the optional preparation coordinator above execution,
not to Core or a protocol callback. It consumes a bounded owned candidate
snapshot containing normalized common facts and the mapping's required extension
metadata. Its storage and lifetime are admitted separately from live calls;
irrelevant document content need not survive `consume`. Completion installs an
immutable mapping into a generation-checked preparation slot, without changing
instance authority. Source input can then remain destroyed. An execution-only
composition accepts prepared entries and reports preparation-required for an
unprepared Consumer operation; it never acquires an upward TD/Planning dependency.
A Producer can use deferred construction only under the pre-publication support
guarantee specified below, with its preparation owner and input already retained.

TD preparation resolves base and validates URI templates. Per-call variables
are checked and expanded by the shared prepared URI mechanism under execution
budgets; the Binding then validates its native addressing constraints. No
protocol case is needed in that common mechanism. Policy and credential audience
checks cover the actual expanded destination, proxy and any permitted redirects
or continuation targets. A supplied TD or discovered link is not permission to
contact arbitrary referenced addresses.

A compiler can produce owned non-`Copy` data through incremental pure jobs.
Capacity precedes each acquisition; failure retains provisional output and a
funded rollback path. An atomic inline compiler remains an efficient optional
adapter. There is no universal all-native-bounds-before-any-pure-start barrier.
Each native acquisition still needs prior authority, capacity and cleanup
ownership; the final Producer coverage/endpoint checks precede Serving, not
necessarily closed native setup that determines an endpoint. This deliberately
replaces the exercised Consumer ordering and the old Producer freeze-before-
all-effects rule where they conflict with the selected design.

## Definitions, instances and physical owners

| Concept | Identity and lifetime |
| --- | --- |
| Thing Model / source TD revision | Descriptive content. A reusable model is not automatically an executable definition or runtime Thing. |
| Prepared definition | Immutable shared semantic/native data with explicit dependencies. No credentials, live handlers, sessions, cancellation or local serving authority. |
| Binding implementation registration | Linked implementation plus immutable configuration, supported facets and resource/progress policy. Its identity is not a network endpoint or permission. |
| Logical Thing instance | Local consumed/exposed identity, description association, behavior/policy, generation and publication state. Several instances may refer to the same TD id or definition. |
| Admitted interaction | One call, action invocation, logical subscription or exposure participation, retaining its own authority, reservations and required leases. |
| Native infrastructure | Binding-owned sessions, listeners, connections, protocol registrations, queues and workers. It can serve many instances/roles and outlive any one interaction. |
| Deployment runtime | Executor/reactor/firmware/process owner. Its loss is an environment event, not successful completion of dependent cleanup. |

References to immutable backing retain its real physical owner and original
account until final destruction. Each instance and call pays for its own
operands, records and references. Do not copy data per instance merely because
it needs independent permission; do not share permission merely because bytes
are identical. A separately versioned Plan Set is needed only if independently
retained/replaced; ordinary instance publication does not intrinsically need a
second epoch.

Allow explicit prepare-once / checked-bind-many where a Binding can declare
invariant dependencies and valid operands. Both compilation and binding can
fail and must be funded. The compiler checks schemas, security meaning, native
policy and non-operand target facts; a name/hash/type match alone is insufficient.
A different endpoint can be an operand for one mapping and a true definition
dependency for another. Specialized per-instance preparation is also valid: no
Binding must manufacture a parameter language, common VM or `bind` abstraction
when it has no reusable work. Automatic interning, persistence and generated
images are optional later features, with their own trust/compatibility checks.

A live local reference identifies its issuer/registry, slot, generation and
particular admitted obligation, or has an equivalent lifetime/ownership proof.
Numeric equality is insufficient. Deferred input, wakes and completions remain
pinned to the original recipient. Accepted work keeps the original handler,
program, principal association and reply capability through settlement; it is
never looked up again by a recycled name or endpoint. Revocation stops new
acceptance and applies the configured cancellation policy to existing work.
Bindings additionally fence replaced native sessions and correlation namespaces;
old native work cannot inherit a new session's trust or reply capability.

## Execution capabilities and essential flows

The following are responsibility families, not a prescribed trait inventory.
One Binding can implement several facets using the same private driver. An
absent facet requires no dummy server, compiler or subscription implementation.

```mermaid
flowchart LR
  TD[TD semantic interpretation] --> P[Preparation and capability selection]
  P <--> BP[Binding preparation]
  P --> R[Owned execution mapping]
  A[Application intent and behavior] <--> S[Servient admission and dispatch]
  R --> S
  SEC[Security and data validation services] <--> S
  S <--> B[Binding conversations and native infrastructure]
  B <--> N[Peers and protocol services]
  D[Discovery client] --> TD
```

### Consumer finite interactions and Actions

A Consumer request identifies semantic target/operation, dynamic values and
control intent. Servient selects an allowed prepared candidate, checks current
policy and data, reserves result/cleanup capacity, and supplies a scoped grant
and operands to the Binding. The Binding returns an owned conversation before
effects; it performs however many messages the mapping requires. Responses
carry selected identity and the applicable data role/response classification.
The Engine validates these and the promised payload contract before delivery.

Property reads and writes, finite Action interactions and native collections
can use this family. It is not a universal transport RPC: the Binding may use
request/reply, publish plus correlated response, multiple messages or another
faithful described mapping. Return of a response, local delivery and actual
remote effect are separate facts. Retry after uncertain write/Action commitment
is a new explicit policy decision, never automatic candidate fallback. TD
`safe`/`idempotent` declarations inform policy but do not prove that an unknown
previous attempt had no effect.

**Action lifetime is independent of request lifetime.** Invocation can complete
with output, fail, or establish a continuing invocation where the selected
mapping provides one. A neutral local invocation reference retains that exact
mapping, opaque native continuation, instance and security association. Query
and cancel are further described operations targeting that invocation; they
can themselves be unsupported. The Engine invents neither a universal task URI
nor a remote task-state schema. Mapping/profile-defined status data remains
available, with only justified common execution certainty recorded by Core.

[TD 1.1 ActionAffordance, Table 6](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#actionaffordance)
already defines optional `synchronous`; absence makes no claim about
synchronicity. It does not mean synchronous-by-default. Likewise
[HTTP 202](https://www.rfc-editor.org/rfc/rfc9110.html#name-202-accepted)
does not prove process completion. Do not manufacture query/cancel support from
an HTTP status or the existence of an `invokeaction` form.

For a Producer, application behavior supplies the actual action process and
its query/cancel behavior. Servient owns the admitted invocation records and
binding association; a job may outlive the initiating response but cannot lose
its local task/cleanup owner. Handing a job to an application supervisor is an
explicit ownership transfer. Local timeout, dropping a future, or connection
closure never proves that a physical action stopped. Results distinguish local
revocation, requested remote cancellation and known remote terminal state.

### Notification conversations and Event emission

A logical observation/subscription owns establishment, control payloads,
delivery, optional reciprocal responses, stop and cleanup. Property observation
uses Property value semantics. Event delivery also accounts for the TD's
[subscription, data, dataResponse and cancellation schemas](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#eventaffordance).
Reducing both to an endless iterator of payloads would lose required control
and response opportunities. Where the selected mapping supports a reciprocal
Event response, its conversation retains the actual bounded response capability
and deadline/disposal obligation. A `dataResponse` schema does not invent a
protocol response channel. Required response semantics cannot be silently
discarded by selecting a mapping that does not support them; such a combination
is refused. Protocol ACKs and semantic Event responses are separate.

Use an application-facing pull cursor, with asynchronous convenience adapters,
over owned conversation state. Wire push, callbacks or polling are Binding
choices. A bounded callback can enqueue native observations and wake progress;
it cannot invoke an application handler. Receiving application credit is not a
claim that the wire or remote publisher supports backpressure.

Each Engine-owned logical subscription has an independent cursor, permission,
limits and stop state. Native publish/subscribe, multicast, group distribution,
matching and any necessary local routing belong to the protocol/Binding. A
broker's remote subscribers need not become individual Engine records. For
Engine-local recipients, matching preserves original logical identities rather
than inventing Things from topic text. Native distribution is not a mandatory
Engine broadcast layer, and one protocol's group semantics do not define another's.

Bindings may share native subscriptions and immutable packet backing when their
authentication, QoS and delivery contracts permit it. Multiple logical deliveries
can borrow the same owned bytes; the Engine mandates no body copy or fresh decode
per recipient. A particular data contract can still require separate processing.
Account backing once at its owner and actual recipient references/work separately.
Native acknowledgements, correlation and reciprocal reply cardinality remain
Binding responsibilities. If a protocol supplies only one reply opportunity, the
Binding declares and enforces the supported response-bearing topology before
acceptance. The Engine checks that capability against the promised logical
interaction; it does not impose a universal unique-replier, aggregation or
arbitration mechanism. It neither silently drops logical response obligations
nor grants more native reply rights than the selected mapping provides.

Property updates can coalesce only under an explicit policy; Event loss cannot
be hidden as a newer state. Overflow must produce visible loss/termination or
supported backpressure without depending on the full queue to report it. An
initial observed value, ordering, replay, duplicates and loss guarantees are
capability properties, not invented Engine promises. For example,
[CoAP Observe](https://www.rfc-editor.org/rfc/rfc7641.html#section-3.6)
has protocol-specific cancellation, and
[MQTT 3.1.1 §3.3.5](https://docs.oasis-open.org/mqtt/mqtt/v3.1.1/os/mqtt-v3.1.1-os.html#_Toc398718042)
permits overlapping subscriptions whose logical recipients cannot be inferred
from packet count. These differences constrain ownership; they do not prove
that this architecture implements either mapping.

Producer Event emission and observable Property changes enter through semantic
sources owned by application behavior. Servient checks the source instance and
its Engine-owned logical delivery policy; Binding conversations perform native
publication/distribution and any required response/control exchanges. A broker
may enforce remote subscriber policy without the Engine enumerating that broker's
subscribers. A source can have zero subscribers. A one-off `publish` success
alone does not implement an exposed
Event's subscription lifecycle or establish consumer receipt.

Stopping a logical subscription revokes local delivery first, then progresses
its mapped unsubscribe/implicit release. It releases only its participation in
shared native state. If remote removal is not acknowledged, report the uncertain
external residue while safely retaining any surviving local obligations. Local
stop must remain possible even when remote cancellation is unsupported.

### Producer exposure, native ingress and dispatch

**Serving means every retained advertised Form and its effective operations have
an established supported mapping, actual behavior coverage and ready required
native participation. It does not mean every future request will succeed.** This
all-retained-Forms publication guarantee is a ClinkZ engineering policy serving
truthful advertisement. [TD 1.1 §8.3](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#protocol-bindings)
requires accurate protocol mappings; it does not mandate our publication phases,
coverage-result representation or a network-availability guarantee.

Before Serving, TD/Planning verify all retained Thing-root and affordance Forms,
effective operation sets, schemas/security inheritance and supported extension
meaning. The actual paired Binding and services must establish support for each
complete combination: native addressing/matching, subprotocol, payload/response
formats, required schema rules, security mechanisms, and any subscription,
cancellation, Action continuation or Event-response behavior that is promised.
Actual registered behavior or an explicitly supported aggregation path covers
each operation. One preferred Form, default operation or security alternative
cannot certify other retained advertised alternatives. A handler-key match,
scheme bitset, `Covered` draft marker or program format tag alone is insufficient.
Unregistered late handlers are private
draft state, never covered live advertisement. There is no Serving exception
that advertises an unimplemented operation and later returns Unsupported.

Producer preparation has two legitimate strategies:

| Strategy | Pre-publication obligation | What may remain deferred |
| --- | --- | --- |
| Construct the required native mappings | Complete native lowering and all common/Binding/service checks. This is the default if lowering is the only reliable support check. | Per-interaction operands, credentials, admission and runtime protocol work. |
| Validate completely, construct some pure data later | The actual Binding supplies an equivalent complete support result for the exact final mapping/configuration, including all dependencies and representability restrictions. Checks that can first discover Unsupported cannot be postponed. | Only bounded pure materialization/cache building whose supported meaning is already fixed; it may still encounter ordinary runtime capacity or cancellation failure. |

The second strategy is optional, not a universal compiler or certificate API.
An unmaterialized definition contains owned normalized mapping facts and the
Binding-qualified validation result with its immutable dependency requirements.
The exposure record separately pins the actual executor/configuration and
codec/security/behavior registrations, plus the funded deferred-construction
owner/input and its coordinator. Shared definition content contains no live
handler, infrastructure, authority or cleanup obligation. The checked association
covers the final description and Form/operation coordinates, bound endpoints/
operands and all policies relevant to mapping support, not a permanent permission.
Semantically equivalent content may share backing; an arbitrary hash, public
boolean or copied generation is not evidence. The result is a trusted linked
implementation's checked obligation, not a mathematical proof or serving grant.
Where validation cannot establish all required support without constructing the
program, the Binding must construct it before publication or refuse exposure.
Sharing validator/lowerer rules and testing their consistency is necessary
implementation evidence; renaming a later Unsupported as an operational error
does not satisfy the obligation.

The timing is deliberately different from freezing all metadata before native
effects. These are semantic boundaries, not mandatory public trait stages:

1. Validate the private draft and known mapping/behavior dependencies. Capture
   the actual providers/configuration and reserve setup, ingress, finalization
   and cleanup capacity before acquiring native resources. Any unresolved endpoint
   or negotiated fact remains an explicit dependency, not a completed coverage claim.
2. Install required native participation while application ingress is closed.
   The Binding can listen, register with a broker or perform another supported
   topology. Partial setup remains owned; external visibility and compensation
   are declared. Native control/ACK traffic is not semantic acceptance.
3. Bind the actual endpoints and other setup-derived facts; finalize the effective
   TD; revalidate all affected common and Binding support/ownership/collision
   facts against that exact final revision. Supplied forms cannot silently change
   meaning. A deliberate reduced TD must itself complete this entire validation
   and readiness process. Failure keeps the instance unpublished and rolls back.
4. With complete qualified coverage and every required participation ready,
   publish the final TD revision, logical registry and current serving authority
   in one Servient-local transition. Final dependency/cancellation checks and
   install make no new fallible native preparation call. Required dispatch/ingress
   material and ownership are already present; no handler receives work earlier.

Readiness concerns the resources needed to expose this topology, such as its
listener or broker participation. It does not pre-open every future connection,
create subscriptions for absent consumers or reserve every future request's
maximum storage. Those resources are admitted when their interactions occur.
Conversely, a known supported mapping without the necessary serving
infrastructure is not ready merely because its pure support check passed.

Deferred materialization is driven by the retained preparation coordinator,
never by a source-TD callback from Core or a hidden native handler. It remains
bounded and cancellation-aware, preserves any accepted work and cleanup owner,
and completes before the native operation, handler dispatch or result delivery
that requires that data. A composition without preparation materializes required
data before installation. A coarse preparation-required or unknown-support
record cannot satisfy Producer coverage.

| Later observation | Truthful disposition |
| --- | --- |
| Invalid operands, failed authorization, ordinary request/queue/allocation limits, timeout/cancellation, handler business failure, or lost connectivity | Runtime rejection/failure under the established mapping. Publication never promised all callers access, permanent capacity, peer cooperation or successful application effects. |
| First use reveals an unimplemented op/subprotocol/codec/schema/security branch, a missing handler, or a deterministic mapping/configuration mismatch already present at publication | Qualification/implementation defect. Fail closed for affected acceptance, retain the exact failure and cleanup owners, and repair/requalify before claiming that mapping ready. Do not hide it as normal Unsupported, remove a Form silently, or substitute another mapping. |
| Advertised endpoint meaning, bound implementation, capability profile or mapping-support policy changes after qualification | The old qualification cannot authorize the new configuration. Revoke affected acceptance and revalidate/re-publish as required; reconnecting equivalent infrastructure under the same checked mapping is ordinary availability recovery. Current authorization and credential rotation remain per-use policy, not a new definition by themselves. |

Externally cached TDs, remote registration and rollback across protocols are not
globally atomic. Local publication guarantees consistent admitted meaning and
ownership, not that remote observers see no intermediate effects or that a
published endpoint can never fail. Four public setup-phase guard families are
unnecessary: one retained installation owner is sufficient if it enforces these
boundaries and preserves partial-failure/drain obligations.

The shared infrastructure path is:

```text
bounded native receive/authentication/matching
  -> zero/many candidates with original registration tokens and native captures
  -> independent Engine identity/publication/policy/capacity checks per candidate
  -> scoped semantic acceptance
  -> Engine selects the registered handler and invokes it outside locks
  -> validated result + original reply/publication opportunity
  -> Binding delivery and settlement
```

Native receive/matching has its own progress owner and budget. It cannot require
an arbitrary Thing's serving permit merely to find a packet's destination.
The Engine does not parse native patterns. A match is not authorization: captured
variables and verified source facts feed the policy/data checks for that exact
recipient. Exclusive request endpoints refuse ambiguous ownership; a genuinely
multicast notification may have several independent recipients. Different native
messages can have different multiplicity contracts; a universal broadcast queue
or unique-equality-key dispatcher is unsuitable.

Pre-publication or draining input can be boundedly rejected, buffered or handled
by a declared protocol policy, but cannot dispatch a handler. A protocol that
cannot enforce this local semantic boundary is unsupported for that exposure
capability. Binding-owned transport ACKs/control messages may still be required;
they are not reported as application acceptance or effect.

### Collections and Discovery

Thing-root forms retain their declared collection semantics. A native collection
can use one conversation and deliver attributed logical results. Do not silently
replace one collection operation with N independent calls, change partial-failure
behavior, or promise atomic writes by aggregation. Explicit application-level
fan-out can be a separate, bounded operation with its own advertised guarantees.
Collection subscriptions retain logical attribution and independent delivery
control without manufacturing protocol topics as semantic identities.

Discovery owns introduction -> exploration -> bounded retrieval/continuation ->
TD admission. Its adapters own DNS/mDNS, HTTP, CoRE or other discovery mechanics;
Core has no discovery-protocol switch. Retrieved metadata is untrusted input;
URI/credential policy, parser limits, result limits and cancellation apply before
and during retrieval. Introduction does not authenticate a Thing or authorize
its interactions. Directory clients can reuse Consumer operations where the
Directory describes them, while direct retrieval need not become `readproperty`.
The current in-memory backend is useful separate functionality, not a reason to
compose a Directory service into every Servient.

## Security, payloads and extension support

Support is the intersection of the selected operation, form context, protocol,
subprotocol, payload/response formats and parameters, schema constraints, security
requirements, runtime profile and configured limits. A scheme list or operation
bitset is only a coarse index; the actual bounded support check validates the
whole combination. Positive preparation means it is constructible, not that
credentials, current capacity or a reachable peer are guaranteed.

Shared TD interpretation resolves security references, Form override and
`allOf`/`oneOf` composition. Security implementations interpret the selected
scheme; concrete Bindings map its protocol handshake/placement/transport trust.
Applications provision credential and policy providers. Runtime credential
capabilities are tied to destination/audience, operation, instance and current
policy, remain outside prepared data/logs/application payloads, and can be
refreshed under a retained bounded owner. Transport-native raw evidence is not
a principal. Only the configured verifier's successful result establishes the
principal/claims used for authorization. NoSec still permits application policy.
Providers accept their own typed or opaque evidence/credential capabilities;
adding a scheme must not require a protocol-specific Core enum variant.
Scheme-defined body/URI credential insertion and removal use bounded projections
around encoding/decoding, without leaking credentials into behavior values or
performing an unaccounted second decode.

Inbound authentication can require bounded parsing, handshake and challenge I/O
before final authorization; it cannot trigger behavior dispatch. Outbound policy
first permits the attempted destination/security operation, then credential use
and authentication proceed under the admitted owner. Recheck mutable policy at
semantic acceptance, including long-lived deliveries where applicable. Session
sharing respects credential/trust/tenant domains and reauthentication requirements.
Changing credentials never silently broadens a prepared security branch.

Unknown extension associations survive document round trips. A bounded context
resolver identifies vocabulary IRIs independently of their protocol meaning.
Local aliases, expanded names and declared contexts must not acquire different
meaning by prefix spelling. Contexts can be explicitly provisioned/pinned;
unsupported remote context processing is reported rather than implicitly fetched
from an execution callback. Opaque annotations need no protocol implementation.
Where an unresolved context can change the selected mapping's meaning, refuse
that candidate rather than guess a default. The mapping specification, not an
Engine-wide prefix rule or new mandatory-extension marker, determines which
extension terms are operational. An unknown selected security scheme refuses;
an unused extension scheme does not invalidate an otherwise supported candidate.

Core defines media-typed payload and executable codec/validator contracts.
TD/Planning prepare the corresponding common data constraints; shared runtime
validation evaluates them without requiring source `td::DataSchema` trees or a
TD parser. This is a common DataSchema implementation, not a protocol IR/VM.
Codec implementations supply generic media serialization; a Binding may add
platform payload projection/envelopes or an inseparable native codec. It must
still preserve the declared semantic data roles and required validation.

Structured values and opaque media-typed bytes/streams are both legitimate.
The Engine does not force every payload through JSON. Bounded decoding and schema
checks occur at the semantic boundary before handler input or claimed validated
output delivery. Unstructured media without a schema does not require an invented
JSON schema. If a required schema/encoding/streaming combination is unsupported,
report it before claiming that guarantee. A separately explicit raw-content API
may expose unvalidated data with that status; identity sealing alone never means
payload conformance. Response selection includes `response`/`additionalResponses`
and their declared schemas, not a single hardcoded successful representation.

## Resource policy, cancellation and retirement

Adopt the ownership-scoped target of
[ADR-0023](../docs/ADRs/0023-resource-ownership-and-policy-boundaries.org):
Engine-controlled backing and promised handoffs are admitted by the Engine;
Bindings enforce native input, concurrent operations, queued bytes, pool/cache
sizes, retry and growth limits; deployment composes process/firmware limits.
Existing StrictV1 and scoped successor contracts retain their original meaning
until their exact migration. This proposal does not relabel SDK bytes as free.

Every boundary accounts for complete retained backing, rather than just the
size of a borrowed view. Shared allocations have one funded physical owner;
independent recipient/call leases have their own cost. New growth can fail after
acceptance if that policy was declared and preserves existing owners; admission
must not promise simultaneous worst-case success without its backing. Cleanup,
terminal status and ownership transfer retain reserved capacity under saturation.
Bounds and fairness are cooperative contracts on trusted linked implementations,
not a sandbox or proof that a native callback always returns.

Cancellation fixes the first local cause, stops new local acceptance/delivery,
and requests the appropriate protocol/application cancellation. Already accepted
work retains its identity and cleanup owner, including a late successful result.
Terminal local settlement does not imply remote rollback, exactly-once remote
execution or completion of physical infrastructure shutdown.

Drop can revoke/request cancellation and transfer into prepaid cleanup storage;
it cannot await a network. An explicit close/destroy operation is the API that
reports drain/cleanup completion. A caller-driven profile must retain and drive
that work. A Host root must have a qualified finite local abort or transfer the
complete remaining owner to an acknowledged longer-lived supervisor. An executor
that may disappear is not that guarantee. Capacity remains charged while work is
quarantined or cleanup is pending. A residual report never licenses freeing a
live local resource; exhaustion fails closed instead of orphaning a task.

**The infrastructure owner controls physical retirement.** Closing one logical
participant releases only that lease and cannot stop a sibling. Outstanding
packets, replies, callbacks and SDK jobs retain the actual infrastructure owner.
Final physical destruction waits for their release/terminal disposal.

Do not generalize Review 08's exact last-route trigger into the only permitted
retirement policy. A configured owner may retain a bounded idle connection pool;
a failed connection may be replaced while logical registrations remain; an
explicit infrastructure shutdown may revoke all dependent gates and drain them.
These are different authorized transitions. Merely polling retirement grants no
shutdown authority, and an arbitrary participation clone cannot initiate it.
Idle resources remain charged to an explicit owner with a bounded retention
policy. Cross-Servient sharing, if chosen, needs an application-owned physical
service with distinct per-Servient leases and compatible trust/account policy.
No generic Engine session manager is required.

Use separate statuses for logical operation settlement, local resource release,
remote residue and infrastructure/executor state. A physical failure can affect
many logical instances; report that scope honestly. Logical isolation prevents
accidental sibling cancellation, not all shared-network or process failures.

## Host and constrained representations

Share operation meaning and lifecycle checks; allow different storage, dispatch
and progress mechanisms. `std`, `alloc`, async syntax, thread mobility, pointer
atomics, dynamic TD preparation and native backend support are independent axes.
Host adapters may use erased owned calls, Arc, an executor and a reactor.
Constrained adapters use associated native state, caller-owned slots, lifetimes
or local generation tables and explicit progress without mandatory atomics.
Neither a universal executor nor a common socket/HAL layer belongs in Core.

Dynamic preparation can be omitted for a fixed executable description, but its
installation still checks compatibility, capacity and fresh local authority.
No serialized validation result is a permission token. The final layout of such
data and any generation tool can wait; TD-independent execution is the boundary
selected now. Shared kernels/invariant tests should protect common semantics;
profile-specific representations need their own storage, wake, cancellation and
release evidence. Identical RAM costs, trait spellings or scheduling traces are
not a product requirement. A portable compile does not qualify a real MCU, SDK,
allocator, hard-real-time workload or no-atomic runtime.

## Reconciliation with the repository and Review 08

Source inspection and scoped execution at `d67dba8` support the following
assessment. Earlier decisions' rationale matters even when their mechanism is
replaced: [ADR-0008](../docs/ADRs/0008-compiled-plan-lifecycle.org) addresses
hot-path rescanning/ownership; [ADR-0009](../docs/ADRs/0009-protocol-binding-integration-and-deployment.org)
addresses practical integration and unowned direct dispatch;
[ADR-0010](../docs/ADRs/0010-server-route-lifecycle.org) addresses lost guards,
late results and ambiguous progress. Those problems remain real.

| Actual owner/evidence | Assessment and decision |
| --- | --- |
| [Cargo dependencies](../Cargo.toml), [module boundary](../docs/architecture/20-module-boundaries.md), [ADR-0024](../docs/ADRs/0024-preparation-and-execution-boundary.org) | Sound dependency correction already made: Core -> Foundation; TD -> Core; compiler authoring lives in Planning; HTTP preparation is optional. Servient still unconditionally depends on TD/Planning. Finish that capability boundary without a Contracts crate or parallel semantic runtimes. Review 08's old Core cycle/implicit Directory observations are not descriptions of current master. |
| [Planning requirements](../docs/spec/planning.md#normative-requirements) | Already permit minimal entries, independent shared backing and incremental pure construction. Keep them. The exercised Consumer aggregate remains eager, all-bounds-first, with its own plan-set generation. This is a valid subset, not the general mechanism. First-use preparation and reusable definitions require actual owner/cost changes, not new names over current envelopes. |
| [HostSemanticCompiler](../planning/src/binding_compiler.rs), [semantic compilation tests](../protocol-bindings/protocols/http/tests/semantic_compilation.rs) | The installed Host Consumer adapter still requires `Copy` output and atomic compilation. A test constructs a generic owned incremental compiler but demonstrates exclusion from that admission. Replace this eligibility restriction; retain the simple inline adapter as optional. Generic erasure alone is not a practical open integration path. |
| [Producer contracts](../core/src/producer.rs), [preparation](../planning/src/producer_execution.rs), [Servient](../servient/src/producer_installation.rs) | Sound current instance authority, whole-description/capability pairing, source-free installation, permits and retained reply. Actual composition is one synchronous Property Read instance; requests lack a general operation/recipient discriminator. Replace that public composition boundary for broader capabilities. Do not multiply this root per operation as the architecture. |
| [Core route API](../core/src/binding.rs), [current SPI](../docs/spec/binding-spi.md#server-route-spi) | Linear stage guards solved concrete ownership bugs and remain useful restricted adapters. Their route-scoped native progress and phase inventory are unnecessary universal constraints. Select separate native progress, recipient acceptance and retained installation/retirement owners. |
| [TD security variants](../td/src/components/security_scheme.rs), [diagnostics](../protocol-bindings/protocols/http/tests/architecture_reassessment.rs) | Current deserialization rejects even an unused extension security scheme. Consumer also refuses absent source id/empty Property key. These are real restrictions to replace, not product semantics or cleanup failures. |
| [HTTP compiler](../protocol-bindings/protocols/http/src/compiler.rs), same diagnostics | Explicit HTTP method on a multi-operation Form is still admitted contrary to TD 1.1 HTTP assertions. Repair in HTTP mapping after acceptance of the relevant change; no HTTP case belongs in semantic Core. |
| [Action data model](../td/src/components/affordance.rs), [TD features](../td/Cargo.toml) | Code labels `synchronous` TD 2.0-only and comments claim TD 1.1 defaults to synchronous. TD 1.1 Table 6 contradicts both. Default typed parsing can retain the name as an extra field; that is not correct interpretation. Revise the TD contract/feature attribution before claiming Action semantics. This issue is additional to Review 08. |
| [Core result validation](../core/src/response.rs), [HTTP serving tests](../protocol-bindings/protocols/http/tests/producer_serving.rs) | Identity/shape sealing is valuable and is not full schema validation. HTTP Producer now checks bounded JSON syntax, including UTF-8, before wire success. Preserve both claims at their actual scopes; add common declared validation rather than treating a seal as DataSchema evidence. |
| [Core feature gates](../core/src/lib.rs), [runtime-safety contract](../docs/spec/runtime-safety.md) | Core now exposes operation vocabulary without pointer atomics but gates runtime APIs away on those targets. This corrects the old compile failure without implementing no-atomic execution. Retain the goal and replace the representation restriction deliberately. |
| [Resource target](../docs/ADRs/0023-resource-ownership-and-policy-boundaries.org), [Consumer runtime](../servient/src/consumer_property_read.rs) | Original-account retention, complete owners and bounded cleanup are sound. Universal SDK maxima and duplicated operation-specific policy frameworks are not minimum Engine responsibilities. Apply the accepted ownership-scoped target consistently as new capabilities are implemented. |
| [Discovery](../discovery/src/lib.rs), [topics 0081](0081-reusable-plan-and-thing-identity.md)/[0082](0082-logical-physical-subscription-decoupling.md) | Preserve explicit client composition and the distinction between content reuse and native subscription sharing. Existing query/reference types and motivating fleet counts do not prove complete Discovery, automatic pooling or fleet performance. |

Review 08 independently identified many of the same separations. ADR-0024 has
accepted a subset of them; that does not accept the complete historical proposal
or extend its executable evidence. Its dependency cut and distinction between
program/instance/native ownership are sound. Its compiler and matching findings
remain useful; the obsolete Producer first-leaf defect is addressed by current
whole-description admission and should not be presented as the current HTTP path.

This recommendation goes further in four consequential ways:

- It specifies Action invocation/query/cancel ownership and Event reciprocal
  control, payload validation, security collaboration and Discovery, rather
  than extrapolating the Property Read sketches to those capabilities.
- It makes complete shared semantic interpretation mandatory, while allowing
  operation-scoped native preparation. Review 08 challenges some barriers but
  does not settle the general Consumer preparation boundary sufficiently.
- It allows specialization and checked reuse as supported strategies; its
  prepare-once example is not a mandatory compiler/operand mechanism for every
  Binding. Correct separation must not create a universal program abstraction.
- It replaces the exact last-route retirement trigger with explicit physical
  owner policy, safe dependent revocation and a final-use destruction barrier.
  Readiness, idle retention, reconnect, failure and shutdown have distinct rights.

The historical UDP/decoded-matching sketches establish their named limited
constructions. They are not MQTT integration, fully funded shared infrastructure,
concurrent publication, async SDK teardown, Action/Event execution or a generality
proof. No new protocol experiment is a prerequisite for deciding this proposal.

### Material alternatives

| Alternative | Why choose or reject it |
| --- | --- |
| Preserve the narrow SPI and add methods per WoT operation | Lowest immediate disruption, but carries single-read carrier/progress assumptions into every capability and still excludes owned compilers. Rejected as the general design; retain only useful adapters and invariant tests. |
| A universal request/response or message bus SPI | Small apparent interface, but shifts subscription control, exposure, action continuations and lifecycle into ad hoc side channels or forces protocol concepts into Core. Rejected. Family contracts may share private ownership helpers without sharing an execution mechanism. |
| Binding-owned complete semantic runtimes behind one facade | Gives protocol authors freedom but duplicates defaults, policy, validation and instance truth, and makes consistent dispatch/security hard to enforce. Rejected. Bindings own mapping and physical execution, not the WoT semantic runtime. |
| Mandatory whole-TD native compilation and two-level immutable plans | Predictable upfront cost and useful for strict deployments. Excessive as a universal Consumer admission requirement. For Producers it is a valid support-validation strategy and required when lowering is the only complete check, without mandating two-level storage. |
| Publish a Producer from coarse capabilities and discover unsupported mappings on first use | Reject: makes a retained Form a speculative advertisement. Missing support is not an ordinary runtime availability failure. |
| Binding-qualified Producer validation with deferred pure materialization | Permit where a complete validation path establishes the same supported meaning without constructing all data. Retained dependencies, construction ownership and final endpoint checks are mandatory; a Binding without that validation path uses pre-publication construction. |
| Core-owned sessions/reactor/shared subscription manager | Central pooling might simplify one transport, but centralizes protocol authentication, matching, reconnect and QoS policy. Rejected. Explicit Binding infrastructure owners already provide the needed control boundary. |
| Separate Host/embedded semantic engines or a new Contracts crate now | Can improve distribution in some projects, but no remaining cycle requires it here; duplicated meaning and migration add cost. Keep one semantic design with optional representation adapters and an execution-only dependency boundary. |

## Assessment of the superseding review

The [superseding review and Owner corrections](https://github.com/yushun1990/clinkz-wot/pull/227#pullrequestreview-5638007609)
are challenge input, not a substitute for the decisions above. The publication
objection is valid: the earlier text left first-use support discovery and complete
Producer advertisement compatible without specifying how. Complete Binding-qualified
validation, or construction when it is the only check, closes that gap. Calling
an unchecked definition a certificate would not close it.

The withdrawn universal Event reply-arbitration objection was too strong. Native
distribution and reply cardinality belong to each Binding; logical semantics and
permission do not require an Engine-wide arbitration or byte-copy layer. What
remains necessary is truthful support for the selected logical response contract.
The architecture-first correction is also justified: mutually incompatible active
definitions cannot safely govern different halves of the same new lifecycle.
Small useful implementation slices come after coherent contract correction;
the historical Property Read interface does not determine the target.

Concrete counterexamples explain the chosen limits:

- A Binding advertising Event support but discovering an unsupported required
  response mapping only on first use cannot justify Producer publication from
  its coarse Event bit. Validate that combination first or reject the Form.
- A listener assigned a port or broker-assigned address during closed setup
  cannot have its final endpoint truthfully frozen earlier. Bind and recheck it
  before Serving, retaining rollback ownership for the externally visible setup.
- A validated codec mapping can defer a pure acceleration table and later fail
  to allocate it. That is a declared runtime resource failure. Discovering an
  unknown encoding branch instead means qualification was incomplete.
- A broker can distribute one publication to many peers without an Engine
  record or payload allocation for each peer. A different protocol's limited
  reciprocal reply remains that Binding's compatibility problem.

These are falsifiers of responsibility claims, not implemented examples or a
finite proof of generality.

## Consequences after acceptance

Retain TD's single semantic ownership, Foundation's real accounts/work values,
protocol-neutral operation vocabulary, complete checked registrations, source-free
execution, original-owner leases, private dispatch, first-cause/late-result
handling, result sealing, explicit local publication and truthful cleanup.
Retain useful HTTP and external-fixture negative cases as behavior evidence.
Preserve historical revisions and their scoped acceptance; do not transfer it to
new representations by editing an evidence label.

Replace the general admission/composition layer, rather than adding a facade
that still depends on `Copy` Property Read or single-route roots. Remove obsolete
API and ownership machinery as replacements become usable. Old stage carriers
can remain implementation adapters when they fit; their signatures must not
constrain the new contracts. Withdraw any unmatched capability claim explicitly.
There is no requirement to keep incorrect public compatibility before v1.

### Comprehensive contract reconciliation before dependent implementation

The architecture is derived first. All incompatible active contracts must then
be amended, superseded or removed coherently **before dependent implementation
is treated as governed by this target**. Neither accepted ADRs (including
ADR-0024), frozen API rows, successful narrow implementations nor old admission
machinery constrain the redesign. Preserve their historical rationale and valid
correctness obligations, not their authority over incompatible new mechanisms.
A broad reconciliation PR, or a small set of dependent PRs with one unambiguous
activation boundary, is appropriate. Do not scatter contradictory rules across
old and new capability slices or add compatibility facades to avoid a large diff.
This proposal PR performs none of that migration and creates no additional gate.

The following inventory follows the registered owners in
[artifacts.csv](../docs/artifacts.csv) and the
[current authority map](../docs/spec/v5-authority-reset.toml). It distinguishes
actual incompatible obligations from sound requirements needing new projections;
an accepted ADR title alone does not make every historical clause active.

| Current owner / exact contract | Reconciliation required by the recommended target |
| --- | --- |
| [Architecture backbone](../docs/architecture/README.md), documents 00/10/20/30/40/50; [Architecture Governance](../ARCHITECTURE_GOVERNANCE.md), Frozen Direction; [ADR set](../docs/ADRs), ADR-0001/0008/0009/0024 | Keep the independently justified dependency direction and explicit Cargo composition. Replace universal plan-set/native-stage/route-progress assumptions, compiler eligibility restrictions, last-route-only retirement and mandatory TD/Planning coupling in execution. Update the current-means portions of the goals document without shrinking its enduring goals. |
| [Planning](../docs/spec/planning.md), `PLAN-ARTIFACT-001`, `FORM-FINALIZE-001`, Producer finalization order; ADR-0008 | Current text requires complete semantics/coverage before effects, eager inbound/publication compilation, final locally determined metadata, and forbids readiness from discovering endpoints. Split pre-setup checks from final pre-publication qualification; retain pure contribution, allow separately owned closed setup and endpoint binding, and permit qualified deferred pure construction. No final TD is served before all checks complete. |
| Planning `FORM-COVERAGE-001` and Advertised behavior coverage | Remove the `AllowLateHandlers` live-publication exception and its normal first-use Unsupported outcome. Draft-only late registration is fine; every live advertised operation needs actual behavior plus Binding/service support. Preserve complete coverage of every retained Form/effective operation; a reduced TD is separately validated before publication. |
| Planning `FORM-OWNER-001`, Owner and collision rules, Host Producer entry; [Binding SPI](../docs/spec/binding-spi.md), Host Producer installation | Reaffirm one accountable paired mapping owner and real collision checks. Reconcile reservation identities with late-bound endpoints and Binding-native sharing, and replace single-read entry/handler/response cardinalities as general contracts. A draft marker or constructor is not complete support evidence. |
| Planning `PLAN-COST-001`, `PLAN-SET-001`, Consumer aggregate, eager/lazy sections; [Runtime safety](../docs/spec/runtime-safety.md), `ADMIT-TXN-001`/`DOC-RUNTIME-001`; ADR-0021 | Replace mandatory eager all-bounds-first Consumer admission and per-instance physical compilation with bounded partial support/preparation snapshots and independently owned definitions. Preserve complete common validity checks, paid work, source independence, exact leases and actual backing retention. Distinguish Consumer unknown support from qualified Producer deferred construction in the lifecycle itself. |
| Binding SPI `BIND-REG-001` and compiler support sections; Planning compiler contracts | Remove installed Consumer `Copy`/atomic-only eligibility and mandatory role pairs while retaining actual compiler/executor/configuration pairing. An execution-only registration retains no compiler; an optional preparation coordinator owns any deferred jobs. Do not rename the restricted adapter into an allegedly open SPI. |
| Binding SPI `LIFE-EXPOSE-001/002/003`, `BIND-ROUTE-001`; Runtime safety `STATE-EXPOSE-001`/`STATE-BIND-001`; ADR-0010/0012 | Preserve local publication, no premature semantic dispatch, complete failure/rollback owners and revocation. Supersede compulsory native prepare/readiness/activate/commit carrier families and route-only native progress. Publish final TD/support facts with current authority; native ingress/matching has independent bounded progress. |
| Binding SPI `BIND-ROUTE-001` and Shared protocol infrastructure lifetime; architecture 50; ADR-0024 and relevant ADR-0023 retirement clauses | Replace the exact last-route initiation rule with explicit physical-owner retirement policy, bounded idle ownership, equivalent reconnect, authorized whole-owner shutdown and a final-use destruction barrier. Logical close must not revoke siblings; polling alone never grants shutdown rights. |
| [Interaction Core](../docs/spec/interaction-core.md), `HANDLER-API-001`/`API-PAYLOAD-001`/`API-OPTIONS-001`; Binding `BIND-OUT-001`/`BIND-IO-001`/`BIND-DELIVERY-001`; ADR-0005/0019 | Replace a single selected Property Read request/result shape as the broad API. Specify separate finite, invocation, notification/control/reply and exposure contracts, including response variants and shared backing. Identity sealing remains distinct from codec/schema validation and remote effect. Equivalent supported Host/static semantics do not mean every capability uses the old read trace or carrier. |
| ADR-0002/0003/0004; architecture 10/40/50 notification/emission/collection sections; corresponding retained Binding/Planning clauses | Reaffirm native remote distribution, no per-recipient byte copies, independent logical subscriptions and no hidden N-call collection fallback. Replace frozen staged publication-target machinery and data-only subscription driver shapes where they lose Event response/control. Broker recipients do not require mirrored Engine subscribers or a universal reply arbitrator. One logical collection operation need not mean one physical message/resource. |
| [Runtime safety](../docs/spec/runtime-safety.md), `API-SECURITY-001`; Interaction Core payload/handler contracts; [handler amendment](../docs/amendments/WP-100-handler-api-v1.md), [output amendment](../docs/amendments/WP-100-interaction-output-api-v1.md); TD preparation under ADR-0020/0021 | Project open scheme preservation, selected security verification/authorization, bounded credential projection, actual required data validation and operation-specific roles. Correct TD 1.1 `synchronous` attribution and Consumer source-id/key restrictions. Preserve strict-versus-ordinary ingestion truth; do not reinstate retired ingestion prototypes or claim their evidence. |
| [Foundation](../docs/spec/foundation.md), Binding `BIND-MEM-001`/`BIND-STORAGE-001`, [Consumer resource contract](../docs/spec/consumer-resource-ownership.md), [ownership schema](../docs/resource-contracts/consumer-ownership-v2.toml), [resource limits](../docs/resource-limits.csv); ADR-0022/0023 and scoped ADR-0006/0011 disposal clauses | Reconcile universal SDK maxima/final-drop assumptions and one-eager-generation policy projections with owner-scoped limits, shared backing and funded deferred construction/cleanup. Strict profiles can remain explicit supported policies. Generalize the useful principle, not a permanent set of one-off operation resource variants; renew physical accounting evidence. |
| Interaction/Runtime safety cancellation, cleanup and profile clauses; [error/cleanup amendments](../docs/amendments/WP-100-error-cleanup-v1.md), [error disposition](../docs/amendments/WP-100-error-disposition-v1.md); ADR-0006/0011/0015/0016/0017 | Reaffirm owned late results, first cause, acknowledged transfer, linear work, clock truth and no implicit retry after uncertain effects. Reproject exact cleanup carriers/error phases for qualification defects, deferred construction, native retirement and no-atomic execution; no old shape is a universal ABI. No reason was found to reverse these underlying safety principles. |
| [State machines](../docs/state-machines.toml) | Reconcile serving activation, compiled-plan-set/lazy-artifact, expose, binding-route/lifecycle/readiness/active-driver, Host Producer endpoint and Consumer admission machines with final qualification and optional materialization. Reproject subscription, producer-subscription, emission and cleanup/call machines for native distribution and separate logical/physical retirement. A changed label cannot preserve obsolete transition/ownership truth. |
| [API ownership](../docs/api-ownership.csv), [performance contracts](../docs/performance), registered validation fixtures/checkers | Amend/delete incompatible frozen paths, stages, request/driver signatures and cardinalities. Reconcile eager/all-bounds, per-route, allocation/fan-out and trace assumptions with the selected capability/profile. Preserve valid invariant tests; replace incompatible assertions and renew affected measurements. Checkers are projections, never a veto on the target design. |
| [Design manifest](../docs/design.md), [spec index](../docs/spec/README.md), [authority map](../docs/spec/v5-authority-reset.toml), [requirements](../docs/requirements.csv), [artifact registry](../docs/artifacts.csv); ADR-0007/0014/0018/0019 | Keep single-owner, explicit revision and history principles. Reconcile affected ownership/active/inactive dispositions and cross-references; the current 65-requirement set and deferred v4.9 clauses do not bound the new architecture. In particular `PLAN-LAZY-001`/`FORM-FINALIZE-002` and broad subscription/codec clauses are inactive input, not independent current authority or automatically reactivated target contracts. |
| [Work-package DAG](../docs/work-packages/index.toml), WP-000/100/200/300/400/500/600/700, all registered intersecting admission and Property Read gate records; [PLAN](../PLAN.md) | Re-derive dependencies, implementation boundaries and evidence obligations from the accepted target, including TD/preparation, open compiler support, public runtime composition and Producer coverage. Preserve named historical results without transferring acceptance. Update durable roadmap/current-target projections coherently; old package boundaries, tranches or proof ordering cannot force a wrong interface. |

ADR-0013's proportional review/admission principle and the explicit client-only
Discovery boundary remain useful. The inventory is not a request to discard
correct accounting, cancellation, strict input or time semantics merely because
they predate the redesign. It is a requirement to revise every conflicting
normative projection together, including broad clauses embedded beside narrow
exceptions, and to state what is retained as a compatible invariant or adapter.
Historical evidence stays tied to the code/contracts actually tested; no green
current-authority checker establishes acceptance of these replacements.

After Owner acceptance and independent assessment, reconcile that complete
contract set before dependent implementation, then implement useful supported
capability slices against it. Test real publication and cancellation, conditional
Producer materialization, notifications, action processes and resource release;
select focused evidence for the claims rather than a quota of contrasting
protocols. Existing useful HTTP/observation checks can be retained or revised
for that purpose, without freezing the old SPI or adding a generality gate.
HTTPS qualification and client-only Discovery remain product directions; their
exact parallel work must be compatible with the accepted target. This task
performs proposal rework only and authorizes no production implementation.

## Assumptions, objections and open decisions

The strongest assumptions are that linked Bindings are trusted cooperative
code; WoT operations plus their mapping specifications supply enough common
meaning; and immutable semantic preparation can keep the runtime independent of
TD parsing at acceptable cost. Different security/payload ecosystems may need
additional facets. The design permits them but does not assume arbitrary native
operations automatically have a faithful WoT mapping.

The strongest objection is authoring complexity: separate preparation, data,
security, logical and physical owners can become a token framework heavier than
a native stack. The response is a small set of responsibility contracts with
simple specialized adapters, not a mandatory type for each noun in this document.
A second objection is that first-use preparation moves failure/latency into calls.
Eager profiles remain available. For Producers, the important residual risk is
divergence between qualification and materialization or a stale dependency check;
an unsupported branch is then a defect, not an allowed late surprise. If a complete
separate support check is as expensive or complex as construction, construct
before publication instead of inventing certificate machinery. Closed setup and
final endpoint binding also need cancellation/publication-race and rollback
evidence, including when native effects are already externally visible.
A third objection is that common validation/security services risk a new universal
IR. Limit them to actual WoT data/policy semantics and supported formats; keep
protocol methods, tasks and wire control entirely in Binding implementations.

I would change this recommendation if realistic integrations show a necessary
protocol/payload coupling cannot cross these boundaries without semantic loss,
if immutable preparation cannot represent required dynamic interaction state
without repeated TD interpretation, or if the family interfaces require repeated
unsafe escapes to native dispatch. The remedy would be a demonstrated boundary
correction, including deletion of an abstraction, not a growing protocol switch
or an assertion that the inconvenient case is outside the goals. A concrete
firmware deadline, hard-real-time requirement or externally committed API could
also change profile/ordering decisions; no such additional constraint is assumed.

Remaining implementation choices include exact Rust trait/enum layout, compiled
schema storage, erasure, index representation, scheduling policy, pool algorithms
and static synchronization. Their owners and semantic obligations are decided
above. Automatic caching, persistence, code generation and cross-Thing pooling
can wait for a real benefit; Action/security/notification responsibility boundaries
cannot.

**No additional Owner facts are required to recommend this architecture.** The
Owner decision is whether to accept these responsibility boundaries and the
pre-v1 replacement cost, or redirect them based on a product constraint. No
runtime-loading, Directory-server, protocol-coverage or release-scope expansion
is requested. Acceptance selects the design direction; it does not itself
migrate active authority or certify a replacement implementation.

## Historical construction records

Existing inbound links retain their historical evidence destination through
these pointers. The old experiments are not reclassified as evidence for this
recommendation.

<a id="sharing-and-generated-image-boundary"></a>
The [earlier sharing/generated-image comparison](https://github.com/yushun1990/clinkz-wot/blob/d67dba8fc9894546ac177f217ff41917dbb96aaa/workspace/0076-composable-capabilities-and-runtime-boundaries.md#sharing-and-generated-image-boundary)
remains available at its original revision. This proposal permits fixed prepared
execution without requiring a generator or accepting serialized authority.

<a id="reevaluation-timing-and-possible-migration"></a>
The [earlier sequencing analysis](https://github.com/yushun1990/clinkz-wot/blob/d67dba8fc9894546ac177f217ff41917dbb96aaa/workspace/0076-composable-capabilities-and-runtime-boundaries.md#reevaluation-timing-and-possible-migration)
records the conditions behind the bounded compiler-support admission. The new
recommendation does not retroactively change that admission or its evidence.

<a id="bounded-host-construction-result"></a>
The [Host construction/ownership investigation](https://github.com/yushun1990/clinkz-wot/blob/d67dba8fc9894546ac177f217ff41917dbb96aaa/workspace/0076-composable-capabilities-and-runtime-boundaries.md#bounded-host-construction-result)
records the output/rollback transfer, call-backing and account experiments used
by their separate scoped work packages. Their technical results remain in their
migrated owners and historical evidence; this proposal replaces neither.

## Evidence and limits of this submission

On master `d67dba8fc9894546ac177f217ff41917dbb96aaa`, Cargo metadata and the
referenced source/specifications were inspected. The following existing test
selection was executed with the lockfile and offline dependency resolution:

```sh
cargo test --locked --offline -p clinkz-wot-binding-http --test architecture_reassessment --test semantic_compilation --test producer_serving --test real_peer
```

All 57 baseline tests passed: 4 diagnostics, 14 semantic compilation, 17 Producer serving
and 22 real-peer tests. The diagnostics intentionally reproduce restrictions or
defects, not desirable target behavior. The serving/peer tests exercise actual
HTTP I/O, publication, cancellation, response validation and resource release
within their narrow cleartext Property Read profiles. The compiler counterexample
proves an admission restriction. None establishes implemented general SPI,
HTTPS/security qualification, Action/Event support, no-atomic execution, embedded
hardware fitness, shared-infrastructure production or architectural acceptance.
They were not rerun for the subsequent text-only review rework. In particular,
qualified deferred construction and endpoint-finalization guarantees need new
focused implementation evidence after contract reconciliation; the existing
tests cannot certify them.

This PR changes this proposal and its workspace index label only. Production,
active contracts and historical review records are unchanged. The required
GitHub `validation` status checks Git diff hygiene; proposal/architecture review
is a separate assessment. No protocol implementation or new gate is included.
