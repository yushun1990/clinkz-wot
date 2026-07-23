# Project Workspace

`workspace/` is the non-authoritative proposal and reasoning space.

It contains unresolved topics that require analysis, discussion, or design exploration before becoming project decisions.

Typical workspace topics include:

- architecture questions;
- design proposals;
- alternative evaluations;
- implementation readiness analysis;
- unresolved engineering trade-offs.

The authoritative contract remains under `docs/`.
Implementation execution is tracked by `PLAN.md`, the work-package DAG, issues, and pull requests.
A workspace topic cannot override those sources.

## Lifecycle

Topics move through:

`OPEN -> DISCUSSING -> DECIDED -> MIGRATED`

Meaning:

- OPEN:
  A question or proposal has been identified.

- DISCUSSING:
  Alternatives and consequences are being evaluated.

- DECIDED:
  A direction has been selected.

- MIGRATED:
  Stable conclusions have been projected into authoritative documents.

After migration:

- decisions belong in `docs/adr/`;
- specifications belong in `docs/design/`;
- execution items belong in `PLAN.md`;
- completed review reports belong in `docs/reviews/`.

The workspace keeps only the reasoning history needed to understand how the decision was reached.

## Important Boundary

Do not create final-form artifacts in `workspace/`.

Examples that do NOT belong here:

- architecture review reports;
- review findings;
- final specifications;
- completed audit documents;
- authoritative decisions.

Prefer discussion-oriented names:

Good:
- `0007-time-domain-question.md`
- `0008-binding-loading-proposal.md`

Avoid:
- `xxxx-findings.md`
- `xxxx-report.md`
- `xxxx-spec.md`
