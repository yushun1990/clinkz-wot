# URI-variable execution and actual destination authority

Status: MIGRATED

Basis: master `5fea8d7228fca6666a244a229c5020ad3a7425b2`, ADR-0025,
WOT-SEM-001, PREP-SELECT/COVER/REUSE-001, EXEC-DATA/SECURITY-001,
SPI-NATIVE/DELIVERY-001 and SAFE-AUTH/CANCEL/CLEANUP/PROFILE-001.

TD 1.1 §5.3.1 and §6.3.9.1 define scalar URI-variable DataSchemas,
Thing-level scope and affordance-level precedence, and RFC 6570 expressions.
These are shared WoT meaning. URI variables are not Action payloads, logical
Thing identifiers, physical device identities or credentials by default.
RFC 6570 expansion does not define a universal inverse matcher.

Current finite Planning rejects declarations. Its execution path supplies no
variables and authorizes the prepared target before native invocation. Merely
removing the refusal would authorize a template rather than the destination
actually contacted. Reinterpreting the TD or compiling a definition for every
value would also conflate definition lifetime with interaction operands.

Selected boundary: prepare complete effective declarations and their schemas,
including declared but unused variables; validate supplied values before native
resolution. Capture only the selected template's references as required operands.
The bounded subset requires each reference, rejects unknown/unused/duplicate
operands, and does not insert DataSchema annotation defaults. Shared RFC syntax
and encoding are pure utilities, not a native router. Concrete Bindings qualify
their supported expression placement, resolve addresses and check actual native
compatibility. Resolution is pure, bounded and has no I/O or credential access.
The resolved destination is a protocol-opaque policy operand in Core; fresh
instance authorization and credential issuance use it, and native start must
verify it agrees with the artifact and operands. Results retain that original
destination for later permission checks. No fallback/replay follows refusal.

Definitions own invariant schemas, reference sets and native mapping meaning.
Instances own identity, permission, service dependencies and accounts; an
application can choose constant values when making its calls without creating
a second mandatory variable-binding abstraction. Calls own actual operands,
resolved destinations, grants, effects and retirement. Native connections remain
Binding-owned and independent of logical Thing cardinality. Compatibility can
require a new native artifact for a particular Binding, but the Engine must not
require this for every dynamic target.

The increment uses finite HTTP Property Read/Write with string/integer/boolean
variables, existing supported scalar constraints, and bounded simple/query
RFC expressions. HTTP qualifies fixed schemes and IP hosts with optional dynamic
ports, paths and queries; concrete destinations are reparsed before effects.
Credential audiences remain exact and independent of instance permission.
Other native mappings retain their own qualified profiles; Producer publication
must decline any mapping whose entire advertisement, extraction and schemas
cannot be qualified. Existing native Action continuation-ID mappings do not gain
general caller-supplied variable support from this increment.
The caller-owned portable Read facet explicitly refuses dynamic meaning while
borrowing it without a Host owner; no portable dynamic I/O claim is made.

Rejected alternatives: HTTP parsing in Core; generic string interpolation;
universal inverse routing; treating a Thing as a physical connection; checking
only the template's origin; silently dropping unused declarations or arguments;
per-target mandatory recompilation; a target cache; schema interpretation in
each Binding. These neither follow W3C meaning nor solve actual-target authority.

Affected owners: TD shared meaning, Core prepared/portable data and destination
contracts, Planning selected/full qualification and reuse, Binding SPI native
resolution, runtime safety, relevant API and WP-100/200/300/400/600 projections.
Static interaction evidence remains scoped regression evidence, not dynamic
execution acceptance. Execution/refusal/cleanup tests, not benchmarks, define
the implementation boundary.

## Migration

Shared scope, scalar/schema/reference meaning, pure native resolution and actual
destination authorization are projected into the registered TD/Core/Planning/
Binding/runtime owners, API projections and affected work packages. The
[execution record](../docs/evidence/uri-variables.md) owns capability limits and
scoped evidence. Relative templates against `base` remain preserved by TD but
unresolved, so this native increment refuses them rather than adding HTTP-only
base interpretation. This concretizes ADR-0025's existing responsibilities; no
new architecture activation, universal target format or acceptance gate is added.
