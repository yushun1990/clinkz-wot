# v4.9 Domain Specifications

This directory contains the single-owner detailed behavioral and public API
specifications selected by `docs/design.md` and constrained by the architecture
backbone. `decomposition.csv` is the authoritative completion map from every
stable requirement to its final detailed owner.

## Rules

- One observable behavior has one domain-specification owner.
- Architecture files define cross-domain invariants; domain specifications
  define exact behavior and API roles without changing those invariants.
- Machine-readable API, state, resource, requirement, and performance artifacts
  are exact projections of a domain specification, not parallel prose owners.
- `docs/requirements.csv` records current requirement authority;
  `decomposition.csv` records final target authority and migration dependencies.
  A target does not become current authority until the requirement source is
  moved atomically.
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

## Final responsibility and authority state

After decomposition, `docs/design.md` retains only the active revision,
normative-language and requirement-identity rules, authority and change
control, the normative-source manifest, the standards baseline, and a concise
revision record. It will not own detailed domain behavior.

The two requirement indexes have deliberately different jobs:

- `docs/requirements.csv::source_path` names the current normative owner;
- `decomposition.csv::target_path` names the final owner and `depends_on`
  records the target-domain migration DAG.

`tools/check-design-requirements.sh` expands both indexes, requires every one of
the 121 stable requirements exactly once in each, rejects unknown or duplicate
ownership, validates dependency ordering, and classifies requirements as
already at their final target, residual in `docs/design.md`, or temporarily
owned by a registered amendment. The target index never grants implementation
authority by itself.

## Reviewed target graph

The sequence is a dependency phase, not a promise to create placeholders.
Domains in the same phase may be reviewed independently. Existing
`planning.md` and `binding-spi.md` clauses remain active; their later
reconciliation or remaining residual requirements still follows this graph.

| Phase | Final target | Detailed responsibility | Direct prerequisites |
| --- | --- | --- | --- |
| 0 | `docs/design.md` | Revision, language, authority, change control, standards, and source manifest | None |
| 100 | `docs/architecture/20-module-boundaries.md` | Crate and dependency-direction invariants | Design manifest |
| 100 | `docs/architecture/10-primary-data-flows.md` | Lock, callback, critical-section, and linearization invariants | Design manifest |
| 200 | `foundation.md` | Resource, work, time, generation, reservation, and memory-admission primitives | Architecture invariants |
| 300 | `documents.md` | TD/TM runtime representation, retention, JSON-LD prefixes, and memory ownership | Foundation |
| 400 | `interaction-core.md` | Handler values and semantics, interaction types, errors, cancellation, cleanup, and in-flight state | Documents, foundation |
| 500 | `planning.md` | Logical and binding-plan compilation, form finalization, publication, and lifecycle | Documents, foundation, interaction core |
| 500 | `codecs.md` | Compiled validation and bounded codec API contracts | Documents, interaction core |
| 600 | `security.md` | Security planning, application, and probe/commit behavior | Documents, interaction core, planning |
| 600 | `binding-spi.md` | Binding registration, route/call/driver execution, delivery, cancellation, host adaptation, and binding state | Interaction core, planning |
| 600 | `discovery-client.md` | Discovery and Directory client values, authority, paging, watch, streaming, and progress | Codecs, documents, foundation |
| 700 | `subscriptions-and-emissions.md` | Subscription storage/data and Producer emission transactions | Binding SPI, interaction core |
| 800 | `servient.md` | Expose/drop lifecycle, scheduling, sharding, admission transactions, and orchestration | Binding, discovery, interaction, planning, security, subscription domains |
| 900 | `profiles-and-verification.md` | Feature/profile matrices, resource defaults, reliability, observability, performance, complexity, capacity, and conformance | All behavioral domains |

Security and codecs are separate targets. Security owns trust, credential, and
probe/commit lifecycle behavior; codecs own document/value transformation and
compiled validation. Their crates, failure effects, resource boundaries, and
verification evidence differ, so a combined `security-and-codecs.md` owner
would hide a real review boundary. Profiles and verification remain combined
because their requirements are cross-domain closure rules rather than another
runtime execution owner.

## Atomic migration protocol

One exact target-domain migration candidate must atomically:

1. reconcile the complete admitted requirement set against architecture,
   accepted ADRs, active amendments, machine-readable projections, code, and
   tests;
2. move stable requirement definitions into the target without renaming ids
   or leaving normative duplicates;
3. update the matching `docs/requirements.csv::source_path` rows, register
   the complete target, and remove the migrated detailed clauses from the
   previous owner;
4. merge any affected amendment into the target and retain that amendment
   only as historical evidence;
5. update affected work packages, checkers, fixtures, audits, reviews, and
   evidence truth; and
6. pass the requirement checker, aggregate design checks, and relevant
   domain-specific verification.

Current authority changes only when that exact candidate passes independent
review and is integrated without changing its reviewed migration boundary.

The project will therefore finish decomposition through several independently
reviewed target-domain migrations, not one monolithic document move. A domain
is split further only when blockers, ownership, lifecycle, contracts,
validation independence, rollback boundaries, or evidence truth actually
differ. A file is never registered merely to represent future intent.
