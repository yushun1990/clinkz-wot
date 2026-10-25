# Producer qualification before publication

Status: MIGRATED

## Reconstructed boundary

Basis: merged `742afac8298012631f34683fa010beeaa6d0e860`. PR #230
separates one logical Producer authority/behavior owner from typed native finite
runtimes. `activate_raw` explicitly grants raw acceptance, not TD publication.
TCP/UDP downstream test providers establish ownership and real I/O, not the
production HTTP mapping or complete description support.

Inspection of the existing HTTP Producer, Planning, TD and Servient shows:

- TD's Producer cursor traverses every declaration and Form and supplies shared
  effective operations/security, URI resolution and scoped semantic lending.
  It does not establish execution support.
- Planning's `ProducerPropertyReadAdmission` accepts exactly one Property, one
  Form and one Read operation. `Covered` means shape/key coverage. Its captured
  compiler/executor/configuration pair is useful, but `PreparedProducer` bundles
  one handler and provider. It cannot qualify a shared multi-Binding exposure.
- HTTP's runtime-only executor is a real bounded nonblocking listener, Hyper
  parser/correlation/reply owner and explicit retirement implementation. It uses
  the older single-route Producer contract, not `FiniteIngressBinding`. Moving
  its handler/authority into each new Binding would recreate the defect #230
  removed. A facade around its permit/request ABI would conceal that boundary.
- The old Servient root retains TD/Planning dependencies through preparation
  admission. The finite root does not. The former's `Serving` marker is not v6
  publication of a final TD under complete actual participation.
- HTTP's removed `supported_schema` walked extensions/children but accepted all
  standard assertions. Core sealed shape/size; native delivery checked JSON
  syntax/UTF-8. None executed the declared DataSchema. The real-peer regression
  `advertised_number_schema_refuses_a_json_string_before_wire_success` returned
  `200 OK` and `"not a number"` before the fix, despite a number-valued TD.
  `[type=object, properties=...]` likewise passed preparation without a validator.

This falsifies complete support under PREP-COVER-001/EXEC-DATA-001. TD 1.1
[section 8.2](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#behavior-data-schema)
requires description/payload accuracy. Handler registration and JSON syntax
cannot establish it. Publication on top of the raw slice would be premature.

## Composition conclusion

Keep the responsibilities in ADR-0025. Qualification above execution must join
the *final* TD's complete Form/effective-operation inventory with actual behavior,
actual native participants and their exact configuration/codec/validator/security
services. A scheme/type/format tag or compiler object alone is insufficient.
Each retained mapping needs one unambiguous supported provider, including every
promised response and lifetime/control branch. The shape checker is not that join.

Bindings decide native matching/collisions and readiness of their required
participation. A registered capability is not ready infrastructure; a ready
listener is not complete semantic support. Closed setup can supply endpoints;
the final exact description/dependencies must then be requalified. A pending,
failed or absent required participant cannot be omitted to publish the remainder.
Native preparation and semantic qualification may interleave when endpoint
facts require it, retaining funded rollback throughout.

Servient owns the final currentness/cancellation checks and the one local
publication commit over the final TD and logical authority. All fallible
preparation/setup/finalization must already be complete. Execution retains only
owned prepared operands/services and the original participation/work owners;
neither TD lending nor Planning is an execution dependency. Optional deferred
construction is justified only if the actual provider can establish equivalent
complete support first; eager construction is simpler here. Do not create a
generic certificate framework to excuse missing behavior.

## Smallest meaningful prerequisite and alternatives

Select actual prepared type validation on the existing HTTP path. Core supplies
the source-independent `TypeSchema` subset/type relation. Planning refuses every
assertion outside it using TD's resolved facts. HTTP supplies bounded JSON value
classification, including exact decimal integrality, and captures the immutable
schema in its native program. The actual executor must validate handler output
before Core success sealing/native delivery. There is no default validation
bypass, new runtime queue, registration facade or TD/Planning runtime callback.
The TD resolved semantic loan and this preparation helper are Host-only; Core's
type vocabulary/kernel has no std/atomic dependency. Portable compilation of the
kernel is not constrained codec/preparation/runtime qualification.

Supported assertions are an optional single type: null, boolean, number,
integer, string, object or array. Missing type imposes no type restriction.
Nested/composed/value/range/length/format/content assertions refuse; metadata,
unit, default and read/write annotations do not constrain returned values.
This is explicitly a subset, not general JSON Schema support.

Rejected/deferred alternatives:

- Treat existing handler promises as validation: the independent wire
  counterexample disproves that shortcut.
- Add a mock-only qualification/readiness state machine: it would still certify
  an HTTP capability that cannot enforce its advertised data meaning.
- First wrap HTTP in the raw runtime and call activation publication: raw mode
  deliberately has no schema guarantee, and replacing the old funded HTTP root
  would also require preserving deadlines, result/cleanup capacity and retirement.
- Implement every schema/codec/security family now: too broad. Refusing missing
  assertions before effects establishes a truthful useful subset immediately.
- Preserve extension-only `supported_schema` behind a new name: remove it.

## Migration and evidence boundaries

The implementation is the existing HTTP service, not a new protocol or a v6
exposure API. Core's restricted `ProducerExecutor` lends an actual optional output
validation facet. Serving preparation requires it; numeric service limits alone
cannot qualify support. Nonserving providers omit the facet rather than add dummy
methods. Runtime rechecks fail closed if the service disappears. Its existing configuration,
closed readiness, identity, funding, dispatch and retirement owners remain.
Keeping this working adapter while fixing its concrete data defect does not make
it the general SPI. The full participation/behavior/final-revision join and
production HTTP integration with the shared logical runtime remain unimplemented.

Affected owners are EXEC-DATA-001 in Interaction Core, PREP-COVER-001 in Planning,
the public API matrix and WP-200/WP-400/WP-600 implementation scope. No semantic
requirement, package DAG edge or milestone is superseded. Prior schema-acceptance
tests and historical HTTP evidence retain their exact revision; they establish
neither current full-schema support nor v6 publication acceptance.

Falsification uses the actual HTTP producer/raw peer and public Consumer loop:
wrong types become 500/null, correct values retain their original bytes, exact
integer meaning survives decimal/exponent/large-number encodings, unsupported
typed and typeless assertions refuse preparation, sources die before execution,
and existing credits/deadlines/cancellation/native disposal/original accounts
remain correct. Runtime-only HTTP/Core and finite dependency checks must retain
their TD/Planning cut. Independent review remains required.
The downstream installation regression also refuses declared service limits
without an actual validator and exercises service loss after preparation,
readiness, receipt and handler completion, with no subsequent dispatch/delivery
and retained cleanup/first cause.
