# WP-200 — Optional preparation — v6.0

## Target and dependency

Build deterministic complete selected support and pure source-independent construction. Permit owned incremental native output, Consumer first use, optional checked reuse and fully qualified Producer deferred construction. Relocate physical output/reclaim owners before execution-only Servient claims.

Exact package dependencies belong to [index.toml](index.toml). Runtime and optional
preparation are distinct: dynamic preparation integration needs WP-200, while
execution-only composition does not. Package completion is not a blanket start
barrier; exact available predecessors and affected authority must support the
chosen slice under ADR-0013. No new runtime tranche is admitted by this migration.

## Authority and completion

Applicable requirements: PREP-ISOLATION-001, PREP-SELECT-001, PREP-DEFER-001, PREP-REUSE-001, PREP-COVER-001.

Required evidence follows actual claims: Downstream compiler/executor pairing without whitelists; source destruction before execution; pure construction capacity/cancellation/rollback; selected support and qualification/materialization consistency. Existing Copy/eager tests remain adapter regressions.

Retained Property Read code/tests and historical tranches remain useful within
their recorded contracts. Their old status does not close this package under
v6. Exact successor source scope/API/state/resource changes, removals, checks and
independent review are determined from the current repository for each material
slice, not from a saved next-action list. New interaction families and Bindings
are outside this architecture migration.

## Type-only schema prerequisite

The Host-only `type_schema::prepare_type_schema` prepares the optional type
assertion from TD-resolved `SchemaView` facts into Core-owned fixed-size
`TypeSchema`. It refuses
const/enum/composition, nested properties/items, required/range/multiple/length,
pattern/format/content and unknown extension assertions; this is a supported
subset rather than a promise to validate arbitrary DataSchema. Metadata, default,
unit and read/write annotations impose no value assertion. Missing type is
unconstrained only when no unsupported assertions remain.

The HTTP Producer captures this result with native operands. The actual executor
performs the corresponding codec/validation check before success sealing. The
preparation pair requires an actual validation-service facet for a serving
provider; a numeric response policy is insufficient. The
extension-only `supported_schema` predicate is removed. Complete final-TD
participation/behavior/readiness qualification remains distinct from the existing
single-Property `Covered` shape marker and `PreparedProducer` adapter.

## Joined v6 preparation boundary

The joined Producer and open Consumer paths supply composition footholds for
[the roadmap's capability growth](../../PLAN.md#near-term-capability-growth-and-priority).
Neither path generalizes the Copy/atomic Property Read adapter into the v6
compiler contract. A Producer's
selected mapping must capture the *actual* Binding/configuration and executable
codec/schema/security services, carry independent owned facts after source TD
destruction, and qualify every retained final Form/effective operation before
Serving. Final endpoint facts discovered during closed native setup require
revalidation; optional deferred pure materialization cannot defer detection of
deterministic Unsupported. Explicitly reduced TDs need complete requalification.

Relocate runtime output/reclaim ownership out of Planning-dependent Servient
adapters when constructing the execution-only composition. Keep pure
preparation and live instance/physical infrastructure ownership independent;
consider checked definition reuse only with proved compatibility. The
type-only HTTP schema path is valid evidence for exactly its supported subset,
not general DataSchema acceptance. Test the first composed path rather than
introduce a universal artifact/cache/program type or a preparatory gate.

## Open Consumer integration

The open finite Property Read foothold and
[protocol-neutral operation composition](#consumer-operation-composition) are
implemented at the scopes below. Further ordering follows
[PLAN.md](../../PLAN.md#near-term-capability-growth-and-priority) across both roles
and all Affordance families. The earlier source diagnosis and initial slice
boundary below remain scoped implementation history. TD-driven Property/Event notification preparation is implemented at the bounded
MQTT scope; HTTP SSE remains an optional mapping under WP-600.

### Source constraints and disposition

This sequence implements ADR-0025 and PREP-ISOLATION/SELECT/DEFER; it does not
replace their architecture. The approved diagnosis used source basis `9c38d2a`; the following table records
those constraints and dispositions. The implemented successor is described below.

| Source | Constraint and implementation decision |
| --- | --- |
| [binding_registration.rs](../../planning/src/binding_registration.rs), `static_consumer_support` / `host_consumer_support` | Static capture downcasts to `ResolvedTargetCompiler`; Host capture requires private `consumer_support`. Generic `HostBindingCompilerRegistration::try_new` never issues that support. Replace these as **general** eligibility checks with captured actual compiler/executor/configuration/services and qualified work/storage/cleanup bounds. |
| [binding_compiler.rs](../../planning/src/binding_compiler.rs), `HostSemanticCompiler`, `ConsumerCompilerSupport` | Atomic lowering, `Artifact: Copy` and fixed destruction/layout costs are legitimate restrictions of the inline adapter. They cannot qualify arbitrary owned output merely by removing the downcast or setting a support flag. Keep their narrow regressions; replace Consumer-wide dependence on those assumptions. |
| [semantic_compilation.rs](../../protocol-bindings/protocols/http/tests/semantic_compilation.rs), `reply_bus` tests | Third-party inline compiler/executor composition already works, with an in-memory result. The former `owned_incremental_compiler_fits_generic_erasure_but_not_consumer_admission` demonstrated the exact missing admission while successfully running 512 paid pure steps outside it. General storage is available; a wholesale compiler redesign is unwarranted. |
| [consumer_property_read.rs](../../planning/src/consumer_property_read.rs), `ConsumerPropertyReadRegistration`; [binding.rs](../../core/src/binding.rs), `BindingRegistrationCapabilities` | Sealed aggregate eligibility and Property Read role bits are restricted adapters, not the general registry. Consumer admission needs per-operation support and deterministic actual-provider selection, allowing unsupported alternatives and requiring at least one supported interaction by default. |
| [binding_compiler.rs](../../planning/src/binding_compiler.rs), `BindingCompilerInput::resolved`; [TD semantics](../../td/src/semantic.rs) | The compiler carrier assumes a Property Read logical record and a matching TD id. Reuse TD-resolved Form/operation facts and extension associations (`FormOwner`); replace this carrier's general use, including mandatory TD identity. Do not copy Producer whole-Thing coverage rules into Consumer admission. |

Reuse complete-registration/recoverable-pairing mechanics,
`BindingCompilerExtension`'s owned cursor/output and failure-return model,
generic Host erasure, and Core's `HostBindingArtifact` / `HostArtifactStorage`.
Generalize their input/pairing where needed, rather than add another facade.
Registration identity or artifact type/format alone proves neither compatibility
nor support. Keep pure preparation above execution and hand source-independent
facts to the selected native facet. Eager selected preparation is sufficient for
the first slice; no new first-use coordinator, cache or universal program is
needed. Normalize/index common meaning once, distinguish coarse eligibility from
prepared support, and do not require an eager all-operations plan-set.

### First implementation PR: open path with real finite execution

Deliver public startup registration, TD admission/selection, bounded pure
incremental construction and execution for actual downstream Consumer Bindings.
Use finite Property Read to establish this seam and migrate the existing HTTP
Consumer onto it. Capture actual codec/schema/security services for the supported
subset; result shape sealing alone is not schema validation. Reuse finite result,
first-cause, cancellation and accounting mechanisms; retire replaced closed
public build/support entry points instead of adding compatibility aliases.
Unchanged legacy adapters may retain
their measured internal restrictions, with no eligibility authority over the new
path and no copies for observation or Discovery.

Acceptance requires executable evidence through the public composed path:

- A Cargo-linked crate **outside workspace membership** supplies an independently
  authored finite Binding, a non-`Copy` artifact with owned variable backing and
  destructor, and a resumable compiler. Its real loopback peer exchanges the
  prepared native request/result; no synthetic return or built-in protocol case
  can substitute. The same application also executes existing HTTP Property Read.
- A TD with multiple Forms/providers reports per-operation support; unsupported
  alternatives remain unsupported, ambiguous matches refuse, and an explicit
  selector cannot weaken schema/security/policy. Actual configuration/executor
  substitutions refuse before I/O. Include an anonymous TD; a TD id is metadata.
- Destroy TD/parser/semantic and completed compiler owners before execution;
  repeated warm calls use retained preparation without source callbacks or
  lowering. A source-free runtime composition has no normal TD/Planning/compiler
  dependency; optional consume/build owners stay above that boundary.
- Prepay callback work and actual cursor/artifact/temporary/indirect backing
  before acquisition. Exercise zero/short credit, capacity/allocation refusal,
  cancelled/failed construction and bounded rollback/destruction. No failed pure
  preparation causes native I/O, partial installation or leaked funding.
- Real calls cover validation failure, saturation, cancellation/late or unread
  results, replacement and final socket/backing/account release. Preserve HTTP
  method/header/context/framing behavior with `real_peer`, `semantic_compilation`
  and affected ownership/race regressions. Replace the owned-compiler rejection
  demonstration with successful composed execution; retain mismatch negatives.

The first PR need not implement every operation or constrained runtime. Preserve
portable compiler authoring and remove compiler-name eligibility from general
typed/static registration too; use external compile/runtime fixtures and affected
no-default/async checks without claiming hardware qualification. Owned callback
costs and variable destruction are the main integration risks, not an excuse to
retain `Copy` as a universal rule. Broader DataSchema/security, deferred first use,
checked sharing/reuse and other capability families remain separate working
slices, with unsupported meaning reported truthfully.

## Implemented open finite Consumer slice

The public entry is `open_consumer::ConsumerRegistry`; HTTP's
`try_consumer_registration` supplies a complete registration and actual services.
The existing generic Host compiler erasure now captures `ConsumerCompiler` work,
configuration and actual Layout metadata for owned incremental authoring. Typed
static capture uses `StaticConsumerBindingPreparation::try_pair` and the same
portable contract. `BindingCompilerInput::operation_input` carries resolved
Form/operation facts rather than a Property Read logical record or source TD id.

Core retains completed definitions, service contracts and the shared opaque Host
bank. `servient-finite::consumer::ConsumerRuntime` owns only finite logical calls,
current authority, captured definitions, native calls, unread/late outcomes and
original leases. Its normal dependency graph contains Core and Foundation only.
The HTTP example uses this path; method/header/context/framing and prior finite
ownership regressions continue on the explicitly restricted inline helper.
`InlineConsumerCompilerSupport`, `InlineConsumerCompilerRegistration`,
`InlineConsumerPropertyReadRegistration`, `try_into_inline_consumer_compiler` and
`try_inline_consumer_registration` replace their misleading general public names;
there are no compatibility aliases for those removed names.

The standalone [downstream crate](../../tools/architecture-fixtures/open-consumer/README.md)
is outside workspace membership and supplies its own real datagram finite
Binding, variable owned request program/destructor, incremental compiler and
integer codec/access service. One application selects and executes both it and
HTTP after dropping the source/compiler owners. It covers actual pairing,
unsupported operations/alternatives, ambiguity, selectors, anonymous TDs, short
credit, allocation/capacity refusal, compilation failure/cancellation, original
results/accounts, replacement, validation/service loss, cleanup and final release.
It replaces the old owned-compiler admission-rejection demonstration with successful
composed real execution. Commands and exact author results belong to the PR;
this does not grant independent acceptance or package/milestone closure.

This Host/manual slice bounds registration count to 32, reported Form/operations
to 128 and concurrent logical slots to 32. It eagerly prepares each selected
candidate while the common semantic Form loan lives; early driver return aborts.
The service subset is type-only schemas plus actual codec and NoSec destination
policy, with unsupported assertions/security refused. Portable authoring and
typed/static pairing are preserved, without claiming a constrained TD runtime.
Deferred first use and additional schemas/security remain incremental
capabilities. Notification mapping choices follow the roadmap's capability
priorities; Producer complete publication qualification is unchanged.

## Consumer operation composition

Confirmed defects and alternatives are recorded in
[0095](../../workspace/0095-consumer-operation-and-instance-composition.md).
The shared registry now selects exact affordance kind/name/operation coordinates;
it prepares Property Read and explicitly synchronous Action invocation using the
same owned compiler/actual service boundary. Same-named Properties and Actions
remain independent. Required input and output schemas are captured separately;
missing roles prohibit data, and each present schema must fit the supported
shared type-only subset. Binding operation metadata only narrows eligibility.

The independent downstream Binding executes both families over real datagrams
after TD/compiler destruction, including warm calls, rejected/unpaid inputs,
empty Action roles, policy/replacement/cancellation and retained late results.
Continuing/unspecified Action synchronicity, control operations, collections,
additional/different response formats and unsupported schema/security assertions
remain explicit unsupported alternatives. This is not a universal finite model
for every WoT operation or complete Action support. Notification preparation uses
the separate typed facet described in the
[Planning contract](../spec/planning.md#open-consumer-event-preparation).

The existing registry/erasure is retained because the owned compiler is usable;
its Property-shaped keys, requests and validator contract are replaced rather
than repeated for each family. A general schema IR, security verifier framework,
first-use cache or protocol-specific stream adds no necessary functionality to
this composition proof and is not introduced.

## Implemented finite join

`finite_producer::qualify_property_read` decodes the exact bounded retained TD,
performs TD Basic admission, complete `ValidatedProducerCursor` coverage through
`ProducerPropertyReadAdmission`, then lends resolved meaning to the actual
`FiniteProducerCompiler`. The open compiler constructs the concrete native
Binding and independent passive configuration snapshot; its final check sees
that same Binding after closed setup. Runtime types still belong to the Binding.
There is no `ProducerExecutor`, legacy installer or compatibility adapter in
this join. Source/semantic loans die before native progress.

The first scope is one Property, one Form, exactly effective `readproperty`, one
actual behavior, at most 16 KiB retained JSON and 256-byte property names.
Unsupported root Forms, siblings, operations, mapping, schema, security and
lifecycle declarations refuse the entire input. An explicit final reduced
description would have to run the same complete process. Work is charged before
JSON decoding, TD steps and lowering; an unpaid step returns Backpressure without
busy retry. Finalization may retry bounded pure qualification on work exhaustion,
but warm Serving never reinterprets the TD. No general schema, incremental JSON
decoder, reuse/cache, delayed materialization or constrained-profile claim is
added. The Servient owner in WP-400 reserves finalization capacity before setup.

The whole-Thing selector's owned definitions can also be handed to the managed
composition owner in [WP-400](WP-400-servient.md#owned-composed-host-producer).
Pure caller construction/compilation stays separate from funded runtime
ownership handoff. That owner captures the actual registrations, validates the
complete staged behavior/mapping set before native effects, reserves final
qualification scratch and repeats complete qualification after closed setup.
It neither changes immutable definition identity into live permission nor adds
a general definition cache or native runtime ABI.

`check_final_thing` shares the bounded common TD walk with construction but
compares the existing owned definitions in place. It never reconstructs a full
`PreparedThing` beside the original. Each selected provider performs its actual
complete `check_final`; every other candidate runs `supports` to detect new
ambiguity under the same matching/error contract as `prepare`. Original operation
coordinates, selected registration identity, complete final operation count and
behavior coverage remain checked. The HTTP and test-only UDP participants reuse
their concrete lowering for these comparisons without boxing another definition.
Work exhaustion remains Backpressure and does not silently certify support.

Actual capabilities declare `final_check_temporary_bytes` for the complete peak
of one support/comparison callback, including indirect or temporary materialized
data. Managed WP-400 admission reserves the largest sequential callback peak
alongside common TD scratch before executing any such callback. Unknown bounds
refuse that managed handoff; the pure caller-provisioned construction boundary
does not implicitly fund requalification allocations.

The typed notification registry also selects an observable Property's Form and
captures its own value schema, actual Binding/configuration and data/access
services. Its PropertyValue schema variant cannot become an Event data/control
role. [MQTT Property qualification](../../protocol-bindings/protocols/mqtt/tests/property_qualification.rs)
exercises refusal/selection, while the
[downstream fixture](../../tools/architecture-fixtures/open-consumer/tests/support/composition.rs)
implements this public facet in an external Cargo crate. No package acceptance
or whole-Thing Consumer support follows from these bounded capabilities.
