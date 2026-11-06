# Executable structured DataSchema

Status: MIGRATED

Assessment at master e482b6a: TD owns resolved SchemaViews, including recursive
properties and exact i64 IntegerSchema bounds; NumberSchema bounds are f64.
Core owns only Copy TypeSchema and operation-specific role carriers. Planning
refuses structured assertions. JSON classifies raw values without f64 conversion.
Consumer captures actual data/access services and independently funded content;
the finite Producer validates through its captured program before behavior and
before delivery. Bindings select native mappings, not common value semantics.

Selected vertical increment: nested object properties/required names, integer
inclusive/exclusive bounds, Unicode string lengths, and existing unconstrained
primitive types. Number bounds, multipleOf, enum/const, arrays with assertions,
oneOf, pattern/format/content transformations and unknown assertions refuse.
Integer schema bounds retain TD's exact i64 domain; payload integers are compared
mathematically, including decimal/exponent encodings and values outside i64.

Replace Copy TypeSchema with an owned, alloc-only PreparedSchema, borrowed by
actual data services. Core evaluates constraints against a codec's value-access
contract. JSON lends raw value slices and decodes only requested names/strings;
no JSON schema tree enters Core or the runtime. Owned nested schema storage is
bounded, source independent and charged to the original prepared owner. A
non-JSON codec can implement the same value access; no std/Arc requirement is
introduced in the semantic module.

Alternatives: codec-local JSON checks duplicate semantic authority; a JSON
schema engine/VM creates a second representation and precision/lifetime policy;
fixed inline recursive buffers make every scalar/runtime pay maximum schema
storage; an Arc inside every schema makes the semantic model depend on pointer
atomics. A bounded ordinary constraint tree with borrowed evaluation is smaller
and preserves the existing content/instance/native separation.

Consumer preparation compiles constraints once, then compares borrowed TD facts
without constructing another constraint tree for reused Property Read/Write or
ReadAllProperties content. Native endpoint compilation and actual service
qualification remain separate. Producer final qualification still checks the
complete final TD; no cached Consumer support certificate is reused.

Affected authority: WOT-SEM-001 (TD), EXEC-DATA-001 (Core), PREP-REUSE-001 and
PREP-COVER-001 (Planning), captured actual validation (SPI/runtime safety), and
RES-OWNER-001/RES-HANDOFF-001/RES-WORK-001 (Foundation). Migrate the resulting
contracts and API/work-package projections before handoff. No milestone or gate
acceptance follows from implementation.

Evidence boundary: real HTTP read/write Consumers and completely qualified
Producers; independent instances sharing constraint/name backing after TD and
compiler retirement; invalid input before behavior, invalid declared output,
validator/schema mismatch, revocation/cancellation/late results, bounded refusal
and final account release. Other media/profile/continuing-family execution is
not established by this increment's HTTP evidence.

Migration: Core, TD, Planning, SPI, runtime-safety and Foundation specifications
now describe the implemented boundary; API and WP-100/200/400 projections and
structured-data evidence are updated. Full collection-map qualification remains
independent of bare-scalar support. Scalar-only validators explicitly refuse
richer constraints; an outer-kind predicate is named accepts_kind and grants no
schema support. HTTP selected final checking compares borrowed constraints;
configuration/reply views retain the original immutable native schema owner.
The independent owned compiler updates its actual group-artifact storage bound
for the larger member layout. These are implementation corrections within the
existing responsibilities, not a new architecture model or acceptance gate.
