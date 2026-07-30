# 0037 Resource Configuration Surface Growth

Status: MIGRATED

Kind: owner-raised resource-model and authoring-ergonomics investigation

Priority: MEDIUM

Target: the growing exhaustive resource schema, profile authoring, Binding declarations, and application-facing configuration

## Scope and authority

This topic records a Project Owner concern that explicit bounded-resource design may become impractical as the schema and per-role declarations grow. It does not request removal of resource bounds or implicit unbounded defaults. Codex owns the resource-model decision.

## Repository observations

- ADR-0015 records growth from 118 to 195 resource fields.
- `ResourceLimits` is borrowed and non-`Copy`, avoiding hot-path copies but not reducing configuration breadth.
- Application-defined exhaustive construction intentionally makes schema additions source-visible.
- Binding registrations also declare lifetime, transient, ingress, route, response, and cleanup footprints.
- Ordinary application interaction APIs should not expose reservation bookkeeping.

## Questions for investigation

1. Which fields are universal, role-specific, operation-specific, or profile-specific?
2. Can schema generation provide role-scoped builders, validated templates, and compile-time omission checks without losing exhaustive authority?
3. How much configuration must a simple server-only Binding supply directly?
4. Are `NotApplicable`, inherited limits, and derived limits represented explicitly enough to prevent accidental defaults?
5. What measurements would justify changing the current flat representation?
6. How are additions kept source-visible without forcing every downstream author to understand unrelated domains?
7. Can documentation and diagnostics explain which limit rejected a registration and why?

## Constraints

- Preserve deterministic finite bounds and explicit admission.
- Do not hide missing required limits behind permissive defaults.
- Keep normal application interaction calls free of low-level accounting parameters.
- Separate ergonomic generated views from the authoritative resource schema.

## Expected decision output

Codex should classify the schema by ownership and applicability, define safe generated/profile authoring surfaces and diagnostics, identify measurements for representation changes, and migrate any stable resource-ergonomics requirements to the schema, generator, Binding SPI, or examples.

## Decision

`docs/resource-limits.csv` remains the flat exhaustive authority. Its existing
resource kind, scope, capability-role expression, zero semantics, and named
profile cells are the ownership/applicability classification. `None` remains
typed non-applicability and never inheritance or unbounded capacity.

Ergonomic surfaces are generated views over that schema. A builder may start
from an explicit versioned named profile or require an explicit value for every
applicable field. Role-scoped views may hide unrelated fields only after the
schema proves them non-applicable; they cannot silently default a required
field. A Binding supplies its own lifetime/transient/ingress/cleanup
declarations, while the application selects the complete Servient profile.
Ordinary interaction calls continue to inherit that profile.

Admission diagnostics name `ResourceKind`, scope, configured ceiling, safely
known requested/observed amount, owning registration or generation, and phase.
Generated documentation groups the same authoritative rows without creating a
second schema.

Representation changes require measurements of retained profile bytes,
construction/clone stack and allocation cost, generated code/binary size,
compile time, and real application/binding author edits. Schema width by itself
does not justify a sparse or implicit representation.

## Migration

The generated-authoring and diagnostic contract is projected into
`docs/spec/foundation.md`; Binding and Servient work packages retain their
existing declaration/admission ownership. This topic is `MIGRATED`.
