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

The joined Producer path supplies the first composition foothold. The Consumer
sequence below now has priority; neither path generalizes the Copy/atomic
Property Read adapter into the v6 compiler contract. A Producer's
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

The smallest useful sequence is **two implementation PRs**: open registration /
preparation with working finite I/O, then TD-driven notification I/O
([WP-600](WP-600-protocol-bindings.md#td-driven-consumer-observation)). Combine
opening the path and its real capability proof: an abstraction-only precursor
would leave the important compiler-to-executor seam untested.

### Source constraints and disposition

This sequence implements ADR-0025 and PREP-ISOLATION/SELECT/DEFER; it does not
replace their architecture. The relevant implementation evidence is:

| Source | Constraint and implementation decision |
| --- | --- |
| [binding_registration.rs](../../planning/src/binding_registration.rs), `static_consumer_support` / `host_consumer_support` | Static capture downcasts to `ResolvedTargetCompiler`; Host capture requires private `consumer_support`. Generic `HostBindingCompilerRegistration::try_new` never issues that support. Replace these as **general** eligibility checks with captured actual compiler/executor/configuration/services and qualified work/storage/cleanup bounds. |
| [binding_compiler.rs](../../planning/src/binding_compiler.rs), `HostSemanticCompiler`, `ConsumerCompilerSupport` | Atomic lowering, `Artifact: Copy` and fixed destruction/layout costs are legitimate restrictions of the inline adapter. They cannot qualify arbitrary owned output merely by removing the downcast or setting a support flag. Keep their narrow regressions; replace Consumer-wide dependence on those assumptions. |
| [semantic_compilation.rs](../../protocol-bindings/protocols/http/tests/semantic_compilation.rs), `reply_bus` tests | Third-party inline compiler/executor composition already works, with an in-memory result. `owned_incremental_compiler_fits_generic_erasure_but_not_consumer_admission` demonstrates the exact missing admission while successfully running 512 paid pure steps outside it. General storage is available; a wholesale compiler redesign is unwarranted. |
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
