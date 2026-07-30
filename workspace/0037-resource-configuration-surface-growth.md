# 0037 Resource Configuration Surface Growth

Status: OPEN

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