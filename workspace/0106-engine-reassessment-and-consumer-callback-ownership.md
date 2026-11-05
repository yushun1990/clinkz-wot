# Engine reassessment: callback ownership before further capability growth

Status: MIGRATED for the selected finite Consumer correction. Independent
architecture and implementation acceptance remain separate.

## Basis and product criteria

Basis: master `ecefd35ad27aaf4093762a21ff64f5816cf29a27` (PR #257), read
from GitHub. Its tree `d21377949035dde2dde93242fb271dc4abd0ad4b` matches the
inspected source. Operation breadth in PLAN is bounded execution evidence, not
architecture closure. Review 08 and topics 0056/0081/0082 provide hypotheses;
their older source diagnoses are not assumed to remain true.

The [enduring goals](../docs/architecture/00-system-goals-and-context.md) require
producing, consuming and discovering Things through replaceable Bindings,
secure/extensible TD interpretation, dependable ownership and portable semantics.
The v6 [authority map](../docs/spec/authority.toml), ADR-0025 and the TD,
Planning, Binding, Core and Runtime Safety specifications remain the active
contracts. Accepted authority does not establish implementation availability.

The dated W3C [TD 1.1 Recommendation](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/)
separates interaction operations, DataSchema constraints, security and extension
meaning; its §§5.3.2/5.3.3/7/8 are relevant beyond operation coverage.
[Architecture 1.1](https://www.w3.org/TR/2023/REC-wot-architecture11-20231205/)
§§6/8 describes interoperable Things/Servients and modular protocol/security
building blocks without prescribing a Rust scheduler, cache or session model.
Neither standard makes an HTTP method, MQTT topic, local test count or a particular
compiled representation the Engine's general semantic boundary.

## Material gaps and evidence classification

Classifications are **M** confirmed missing functionality, **D** demonstrated
architectural defect, **R** unverified design/scaling risk, and **F** deliberately
deferred capability. A row can contain distinct findings in different classes.

| Area | Evidence at the basis | Classification and consequence |
| --- | --- | --- |
| TD and complete DataSchema execution | `td/src/semantic/facts.rs` lends nested schemas, required/range/string facts, const/enum and oneOf. `planning/src/type_schema.rs::prepare_type_schema` refuses these before `ConsumerDataService::supports_schema`; `core/src/value_type.rs` represents only Any/type. HTTP/MQTT use that same restricted preparation. | **M:** general value validation and extensible schema-service preparation. The current paths refuse unsupported assertions rather than claim to validate them: absence is not itself an unsound abstraction. A broader validator cannot be added just by overriding the current type-only service. An owned, bounded schema/service boundary must precede that capability. |
| Extensions and security meaning | TD preserves unknown security definitions and resolves bounded local contexts. `planning/src/open_consumer.rs::common_services`, the Action/notification preparation facets and concrete Producer compilers accept only endpoint NoSec; unknown selected meaning refuses. | **M:** general security execution, credential acquisition/verification, scopes, trust and HTTPS delivery. Preservation is not execution support. **R:** interoperability across additional context/extension/security compositions has not been qualified. Review 08's old closed-security-discriminator defect has been corrected; it is not a current defect. |
| Remote ingestion | Ordinary typed Serde and bounded admission operate on supplied input. ADR-0020 separates literal strict wire decoding, duplicates/null/numeric rules and transport ingestion. | **F:** strict remote ingestion remains explicitly deferred. It must not be presented as implemented merely because typed semantic admission passes. |
| Preparation, reuse and operands | `PreparedConsumer::into_execution` moves definitions/funding into one runtime. `ConsumerRuntime::try_new` requires the same parent bank. Definitions carry exact target, actual services/configuration and artifact. HTTP additionally embeds target and fixed header storage per operation. Action/notification preparation are separate transactions. | **M:** public checked bind-many reuse across independently funded instances and first-use preparation coordination. **R:** repeated Basic/semantic work, invariant bytes, per-operation native operands and reference costs at fleet scale. Specializing a definition per endpoint is allowed by PREP-REUSE-001, so neither specialization nor these missing facilities alone proves a defect. Adding Arc clones or a cache without proving operand/configuration/schema/security dependencies would not establish safe parameterized reuse. |
| Finite Consumer callback ownership | In the basis `servient/finite/src/consumer.rs`, start/step/poll retain `RefMut<State>` across actual authorization, codec and Binding calls, and abandoned call destruction. The public regression in `consumer_callbacks.rs` fails on the basis with `RefCell already mutably borrowed` when authorization reenters the same runtime. | **D, selected:** mutable registry ownership is incorrectly also the linked-code execution boundary. Read-only reentry, close, sibling cancel/progress and destruction can panic instead of preserving independent logical work. This contradicts SAFE-CONCUR-001 independently of protocol mapping or operation coverage. |
| Continuing Consumer Action callback ownership | `servient/finite/src/consumer_action.rs::invoke_action` borrows State mutably before `Selected::check` invokes access policy; `begin_close` takes the same mutable borrow. Step/control/handoff similarly call linked services while borrowing State. | **D:** the same source-proven reentry conflict remains in a distinct continuing owner. This PR's executed reproduction and correction cover the finite owner only. It does not claim to fix all Servient callbacks. Extending the finite carrier to Action invocations would erase their distinct continuation/query/cancel lifetime. |
| Composition and authority | One `ConsumerInstance` can back finite, continuing and notification facets. Producer authority can participate in multiple native types; `servient/finite/tests/native_ingress/shared_ownership.rs` exercises original ownership and replacement. Notification stop controls are separate passive Cells. | Existing separation is useful but not full composition acceptance. **R:** broader cross-facet reentry, fairness, cancellation interaction and long-running aggregate capacity need scoped evidence. A common scheduler or universal session is not demonstrated necessary. |
| Native sharing versus Thing count | MQTT tests share native subscriptions/connection ownership while preserving independent conversations. Core's external TCP/datagram Producer fixture shares a native listener across Things. In contrast, `HttpFiniteConfig` captures one instance, and the HTTP Consumer client has one occupancy flag per registration. | The one-instance HTTP listener is a concrete **M** Binding capability, not an Engine requirement; the existing contrast already rejects that inference. **R:** multi-tenant compatibility, wildcard overlap, partitioning and throughput at many-Thing scale. Bounded single-flight MQTT/HTTP policies are real limits, not measured fleet failures. Lowering a described root interaction into N leaf calls would be a semantic defect, not a scaling remedy. |
| Host/static/embedded boundaries | `servient-finite` has Core/Foundation-only normal dependencies; execution need not retain TD/Planning. Its new family modules and open preparation are Host/std implementations. `core/src/lib.rs` gates execution modules on pointer atomics; no-atomic targets retain vocabulary/tooling. Retained static Property Read is narrower than all joined operations. | **M:** equivalent broader constrained execution and no-atomic runtime representations. Successful no_std tooling compilation is not runtime parity. **F:** actual embedded protocol/hardware and Zenoh/pico product qualification are explicitly future scope, not reasons to abandon portable semantic goals. |
| Discovery and product qualification | Discovery has explicit traits and an in-memory backend; the normal Servient has no implicit Directory. PLAN still requires client Discovery/Directory and Host HTTPS/security qualification. | **M:** joined production Discovery/Directory client and remaining interoperability/security qualification. **F:** a Directory service, durable Core broker, arbitrary dynamic library loading and hot unloading are v1 exclusions, not architectural defects. |

Existing tests also cannot establish aggregate CPU/memory bounds, useful sustained
throughput, arbitrary third-party mapping support, complete DataSchema/security
or all-profile equivalence. Those remain scoped engineering questions. No new
protocol is justified for the selected callback defect: it exists above native
mapping, and the repository already has an independently linked owned/incremental
datagram Binding as a relevant contrast to HTTP's inline compiler.

## Candidate comparison and selection

| Candidate | Architectural impact | Prerequisites / cost | Uncertainty resolved |
| --- | --- | --- | --- |
| Correct callback/registry ownership for finite Consumers | Restores linked-service extensibility and independent cancellation/progress for every finite operation. Removes an actual contract violation. | Existing Core definitions, currentness and native owners suffice. Bounded change to one execution kernel, its controls and evidence; comparatively small. | Directly falsifiable nested capacity, stale handoff, late ownership, wake and destructor behavior. |
| General prepared DataSchema/value services | High interoperability impact across all roles and encodings; type-only carriers must evolve. | Medium/large: owned trees or service artifacts, resource/destruction bounds, exact value semantics and actual validators. Number, pattern/format/content and extension policies need deliberate support. | General data expressiveness and service extensibility; does not correct existing callback ownership. |
| TD security plus transport trust | Essential product capability, wider than another HTTP mapping. | Medium/large: prepared requirements, runtime credential/audience/principal services and actual trusted native I/O. | Secure execution and fail-closed composition under actual credentials; independent of operation breadth. |
| Definition/operand separation and checked instance reuse | Potentially high for many Things and devices. | Medium/large: actual invariant dependencies, permitted instance operands, fresh authority/policy and original shared-account retirement. No automatic cache is prerequisite. | Whether concrete mappings can avoid measured repeated work/backing; no current fleet workload establishes the dominant cost. |
| Portable execution / production Discovery | Advances enduring capabilities independently. | Separate bounded implementations/backends and environment-specific evidence. | Profile equivalence or retrieval behavior; neither repairs the demonstrated linked-callback defect. |

Choose the finite callback ownership correction because the observed behavior
violates a current guarantee at an extensibility/authority boundary, with no
uncertain product premise or new protocol prerequisite. Correct this boundary
before adding more linked schema/security code to it. This is not a claim that
all missing capabilities must wait for every lifecycle defect or that finite
operations are the Engine's universal runtime.

Rejected remedies: document callbacks as nonreentrant (withdraws the ownership
contract); replace RefCell with a mutex (deadlock/serialization preserves the
wrong boundary); ignore failed try_borrow (can lose control intent/work);
temporarily vacate the slot (nested admission can reuse accepted capacity);
move callbacks to hidden tasks (new executor/queue/lifetime machinery); or merge
finite, Action and notification states into a universal session (different controls
and retained lifetimes). No compatibility facade, protocol mapping or new gate
is required.

## Implemented boundary and review limits

The selected implementation uses immutable instance composition, short registry
borrows, reserved original slots, scoped checkout/restoration and independently
funded passive call control. Close/cancel/abandonment remain reachable while
the complete native/result owner is outside the registry. Native construction
is local; callback-time closure retains returned work for cancellation before
polling effects. Controls are checked again after service/native callbacks and
before handoff. Final removal, rather than native readiness, linearizes handoff.
Destruction and wake callbacks occur outside registry borrows. Unwind restores
ownership without promising arbitrary linked-code recovery.

The exact contract is migrated into Runtime Safety, the Servient backbone,
WP-400 and API/state projections. Tests use public TD preparation, deterministic
native scheduling and the existing real HTTP executor; no new mapping is added.
PR evidence owns the executed commands, revisions and environment limitations.
Independent acceptance must assess the correction and its omitted defects; this
investigation supplies no self-acceptance or package/release completion claim.
