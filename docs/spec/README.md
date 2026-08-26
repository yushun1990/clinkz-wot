# v5 Authority Specifications

Status: v5.1 Consumer one-shot activation candidate. v5.0 remains the active
mainline authority until the separately reviewed activation checkpoint selects
v5.1.

ADR-0018 established the bounded v5.0 authority reset. ADR-0019 is the proposed
v5.1 domain-entry decision that re-adopts exactly three previously deferred
Consumer one-shot identities. `v5-authority-reset.toml` now projects the v5.1
candidate while retaining `current_design_revision = "5.0"` and
`target_design_revision = "5.1"` so review cannot be mistaken for activation.
The deleted `decomposition.csv` remains recoverable from Git history as D3
migration input; it is not v5 authority or roadmap.

## v5.1 candidate owners

| Owner | Candidate responsibility | Count |
| --- | --- | ---: |
| `docs/design.md` | revision, authority, admission, API ownership, profiles, standards | 6 |
| `docs/architecture/10-primary-data-flows.md` | concurrency, callbacks, linearization, Directory scope | 5 |
| `docs/architecture/20-module-boundaries.md` | dependency direction | 1 |
| `foundation.md` | resource policy, admission memory, constrained storage/work | 8 |
| `docs/amendments/WP-100-time-domain-v1.md` | logical source time and deadline semantics | 2 |
| `interaction-core.md` | handler/value/error/cleanup plus Consumer selection-control values | 11 |
| `docs/amendments/WP-100-handler-api-v1.md` | exact completed handler values | 1 |
| `runtime-safety.md` | representation, security, progress, lifecycle safety | 10 |
| `planning.md` | immutable-plan core plus selected Consumer request static-data boundary | 9 |
| `binding-spi.md` | registration/routes/delivery/I/O/cancellation plus `OutboundRequest` boundary | 12 |

Candidate total: 65. The machine manifest is authoritative for the exact
candidate identities; this table is navigation. Until activation, the last
integrated v5.0 checkpoint remains the current authority for source admission.

## Rules

- One candidate-active requirement has exactly one registered candidate
  definition.
- Architecture sources own cross-domain invariants; specifications own their
  exact detailed contracts without overriding architecture.
- Machine artifacts project named fields and evidence; they are not parallel
  prose owners.
- `docs/requirements.csv` retains profile/package/evidence metadata for all 121
  stable identities. It is not the authority selector; classification and
  candidate ownership come from `v5-authority-reset.toml`.
- Historical text retained inside `planning.md` or `binding-spi.md` is
  authoritative only when its stable id is registered active by the manifest.
- A narrow completed amendment remains an owner only for an identity registered
  to its path. Other affected-id mentions are history or refinement context.
- Work packages describe migration and evidence. A deferred requirement may
  trigger domain-entry review but cannot satisfy admission or close a gate.
- A candidate does not admit source implementation merely because its proposed
  normative text and checker pass.
- A conflict blocks the affected implementation; file order does not resolve
  it.

## v5.1 domain-entry change

The candidate moves exactly these identities from
`inactive-domain-entry-review-required` to candidate-active authority:

- `PLAN-REQUEST-001` in `planning.md`;
- `BIND-OUT-001` in `binding-spi.md`; and
- `API-OPTIONS-001` in `interaction-core.md`.

`API-OPTIONS-001` is intentionally narrower than its historical v4.9 wording.
The v5.1 baseline owns only the Consumer selection/control kernel needed by the
first one-shot proof: URI-template variables, optional explicit form selection,
and call timeout/deadline intent, with omission distinct from explicit
selection. Operation payload is not an option. Historical binding-id, media,
subprotocol, security-branch, validation-profile, broad defaults merging, and
other advanced selectors remain outside this authority entry.

Consumer Property Read response validation is a narrow refinement of already
active `API-PAYLOAD-001`, `BIND-IO-001`, and `BIND-DELIVERY-001`; it does not
activate codec/schema or broad validation requirements.

## Inactive dispositions

After v5.1 activation, 31 domain-entry identities remain deferred. They cover
long-lived subscription/emission behavior, Directory client execution,
codec/validation breadth, advanced planning/lazy/cache/index behavior,
observability/status, scheduling, and profile defaults. Each affected domain
must re-adopt, replace, split, or retire its identities in a later reviewed
revision before implementation admission.

The 15 historical design inputs remain useful constraints or measurements but
do not freeze an implementation. The four premature/superseded identities stay
retired. The six redundant identities remain discharged by stronger governance,
authority, or executable owners. The classified total remains exactly 121.

## Candidate and activation protocol

The v5.1 candidate must:

1. contain only documentation and checker changes;
2. keep `current_design_revision = "5.0"`, set
   `target_design_revision = "5.1"`, and remain `status = "candidate"`;
3. define the 65 candidate-active identities exactly once and classify all 121
   identities exactly once;
4. prove the remaining 56 identities are inactive in their recorded classes,
   including exactly 31 domain-entry-deferred identities;
5. keep ADR-0019 `Proposed` and outside the accepted ADR index during candidate
   review;
6. leave the current v5.0 work-package/gate selectors active and create no
   Consumer gate manifest or source admission;
7. pass candidate authority checks plus unchanged implementation regressions;
   and
8. receive independent architecture-level acceptance at one immutable commit.

Only a separate activation checkpoint may then:

- mark ADR-0019 accepted and index it;
- switch current/target selectors and registered revision projections to v5.1;
- replace candidate-only checker routing with the normal active-v5.1 checker;
- record the independently accepted candidate commit; and
- mark workspace/0061 `MIGRATED`.

That activation checkpoint must not rewrite the reviewed requirement semantics.
Only after activation may the repository create and register the
`CONSUMER-PROPERTY-READ-ARCHITECTURE` gate and admit its exact ADR-0013 source
tranches.
