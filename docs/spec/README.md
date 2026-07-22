# v4.9 Domain Specifications

This directory contains the single-owner detailed behavioral and public API
specifications selected by `docs/design.md` and constrained by the architecture
backbone.

## Rules

- One observable behavior has one domain-specification owner.
- Architecture files define cross-domain invariants; domain specifications
  define exact behavior and API roles without changing those invariants.
- Machine-readable API, state, resource, requirement, and performance artifacts
  are exact projections of a domain specification, not parallel prose owners.
- An accepted ADR must be integrated into the owning specification in the same
  design revision.
- Work packages describe migration and evidence only.
- A registered normative amendment is active only for its explicit affected
  requirements and refinement boundary. It is merged into the relevant
  specification and then retained as historical evidence under ADR-0014.
- A conflict blocks implementation; file order does not resolve it.

## Active v4.9 owners

- `planning.md`: effective-form planning, compiled plan sets, compiler
  extensions, plan publication, lazy artifacts, and reclamation.
- `binding-spi.md`: complete binding registration, client/server execution,
  routes, calls, subscriptions, responses, emissions, cancellation, and cleanup
  transfer.

The remaining valid material is still being migrated. Until a domain file is
present and registered, the applicable v4.9 clauses in `docs/design.md` remain
the residual detailed owner identified by the requirement registry, subject to
the architecture and accepted ADRs. Registered normative amendments may refine
only their declared residual scope. Historical v4.8 text is migration input,
not active authority. No unmigrated domain is implementation-ready merely
because residual prose remains available; a bounded tranche still requires the
ADR-0013 admission record and review.

## Planned ownership map

| Domain specification | Detailed owner |
| --- | --- |
| `foundation.md` | Resource, work, time, generation, reservation, and accounting APIs |
| `documents.md` | TD/TM compatibility, representation, validation, and extension fidelity |
| `interaction-core.md` | Handler and interaction semantics, payload values, errors, progress, and local dispatch |
| `security-and-codecs.md` | Security planning/application and bounded codec contracts |
| `planning.md` | Logical/binding plan compilation and lifecycle |
| `binding-spi.md` | Protocol Binding integration and execution contracts |
| `subscriptions-and-emissions.md` | Servient-facing subscription and emission transactions |
| `servient.md` | Application facade, produced/consumed handles, registries, scheduling, and cleanup runtime |
| `discovery-client.md` | Discovery and Directory client behavior only |
| `profiles-and-verification.md` | Feature matrix, named profiles, reliability, performance, and conformance rules |

This table defines decomposition ownership, not permission to create empty
placeholder specifications. A file is registered only when it contains the
complete reconciled contract for its current scope.
