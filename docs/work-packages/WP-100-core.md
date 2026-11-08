# WP-100 — TD and semantic execution contracts — v6.0

## Target and dependency

Maintain one TD interpreter; complete media/schema/security contracts below preparation, local identity and capability-specific interaction outcomes. Preserve Action invocation and Event control/response meaning without implementing a universal request/session.

Exact package dependencies belong to [index.toml](index.toml). Runtime and optional
preparation are distinct: dynamic preparation integration needs WP-200, while
execution-only composition does not. Package completion is not a blanket start
barrier; exact available predecessors and affected authority must support the
chosen slice under ADR-0013. No new runtime tranche is admitted by this migration.

## Authority and completion

Applicable requirements: MOD-DEPS-001, WOT-SEM-001, WOT-EXT-001, EXEC-ID-001, EXEC-HANDLER-001, EXEC-DATA-001, EXEC-SECURITY-001, EXEC-ACTION-001, EXEC-NOTIFY-001, EXEC-COLLECTION-001, EXEC-OUTCOME-001.

Required evidence follows actual claims: Shared semantic defaults and extension cases, source-free validation, missing/unknown versus unsupported, first-cause/late result and supported profile checks. No claimed no-atomic runtime from vocabulary compilation.

Retained Property Read code/tests and historical tranches remain useful within
their recorded contracts. Their old status does not close this package under
v6. Exact successor source scope/API/state/resource changes, removals, checks and
independent review are determined from the current repository for each material
slice, not from a saved next-action list. New interaction families and Bindings
are outside this architecture migration.

## Consumer composition scope

Core now carries operation-scoped finite definitions, distinct input/output
payload schema roles, and a separate move-only Consumer authority/passive
reference. The implementation and independent Binding evidence are scoped in
WP-200/WP-400. The bounded finite structured subset and narrower type-only/NoSec facets remain
supported subsets; general DataSchema, security composition/credentials,
continuing Consumer Actions and TD-driven Event/observation preparation remain
open. Existing raw Event control/response/retirement evidence is retained.


## Finite collection write evidence

The joined all/selected root writes share bounded writable-member schemas while
retaining their distinct completeness rules. `PropertyWriteReport` replaces an
empty-success-only interpretation of the shared collection result: original-set
member effects, optional opaque native statuses and explicit atomicity evidence
survive independently of local cause and delivery. It is inline in existing
size-accounted outputs; it adds no universal transaction or collection runtime.
The [Core collection contract](../spec/interaction-core.md) owns exact semantics.
[Core report tests](../../core/tests/collection_write_report.rs), the HTTP joined
slice and an independent Cargo-linked datagram fixture exercise its scoped
validation, original selection and late-release obligations. No package, gate,
milestone or production qualification is inferred from this increment.

## Action collection data and permission

Core now retains Action definitions/output roles separately from scalar values
and requires an actual aggregate-status validation service. A root query policy
is distinct from the original per-invocation handler and principal checks.
[Interaction Core](../spec/interaction-core.md) owns those semantics; the
[HTTP integration](../../protocol-bindings/protocols/http/tests/query_all_actions.rs)
exercises native identity, different member schemas and source-independent
validation. This introduces no universal status envelope or Action/session store.

The Consumer ownership counterexample established a second concrete use of the
slice-only output carrier. Core `InteractionOutputView`/`PayloadView` now supply
the shared read-only data representation for finite Consumer and raw Producer
Action result boundaries, directly replacing the Action-local views. Owned
`InteractionOutput`/`Payload` remain internal handoff carriers. The
[ownership contract](../spec/interaction-core.md) requires retaining the original
funded settlement to share its backing; this adds no generic interaction owner.
Notification delivery also reuses Core `PayloadView` together with the existing
`AffordanceSite` name loan. Its queue owner remains exclusively borrowed until
the slices are no longer used; it creates no detached delivery/result owner.

## Executable reusable structured data

The bounded [prepared DataSchema contract](../spec/interaction-core.md) now
covers nested object properties/required names, integer boundaries and Unicode
string lengths for the real HTTP finite slice. Core owns the alloc-only
constraint model/evaluator; TD supplies common facts, Planning owns pure
construction/borrowed compatibility, and codecs lend actual encoded values.
TypeSchema is replaced directly; data-service schema inputs borrow PreparedSchema.
Property Read/Write and ReadAllProperties demonstrate constraint reuse across
independent Consumers, while fully qualified Producers validate input before
behavior and declared output before delivery. Dynamic storage/destruction and
actual validator qualification remain funded. Whole-TD normalization reuse,
native retargeting, richer schema assertions, non-JSON interoperability,
continuing-family reuse and broader portable Engine capabilities remain unproven. Detailed
executable scope is in [structured schema evidence](../evidence/structured-dataschema.md).
No package, architecture gate or milestone acceptance is claimed.

## Representation-independent finite boundary

The [finite execution contract](../spec/interaction-core.md#finite-execution-and-ownership-representations)
extracts shared value-role checks, result identity/shape validation and actual
finite transitions from Host ownership. Portable errors, identity, deadline,
metadata and borrowed views no longer inherit unrelated Arc module gates. The
caller-owned native Read facet remains in Core, below Servient. Executable scope
and limits are in [the evidence](../evidence/constrained-finite-execution.md);
historical PlanSet/static acceptance is not reused for this modern claim.

## Finite security contracts

Core now carries immutable security requirement operands and redacted opaque
per-call capabilities without concrete scheme/transport cases. Access-service
qualification/revisions and paid finite verification extend EXEC-SECURITY-001;
TD remains the shared owner of defaulting and Form override. Scope and executable
negative evidence are in [the security assessment](../evidence/td-security-https.md).


## Security composition correction

The existing EXEC-SECURITY-001 / PREP-SELECT/COVER/REUSE-001 scope now replaces
singleton requirements with bounded effective composition, exact Consumer branch
selection and complete Producer security qualification. HTTP finite Read/Write
executes independently paired Basic/API-key services, conjunctions and distinguishable
alternatives. Domain specifications own meaning and the Binding owns placement,
identity composition and operation limits. [Scoped evidence and superseded
assumptions](../evidence/security-composition.md) identify the supported subset,
revocation/ownership obligations and remaining work. No package/milestone,
production-security or additional Embedded acceptance is claimed.
