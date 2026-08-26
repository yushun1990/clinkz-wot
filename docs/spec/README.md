# v5 Authority Specifications

Status: active v5.1 Consumer one-shot authority.

ADR-0018 established the bounded v5.0 reset. ADR-0019 activates the independently
reviewed v5.1 Consumer one-shot entry at candidate commit `3b133ebfe3c870102931982d6c056595f9d44255`. The
active `v5-authority-reset.toml` now selects `current_design_revision = "5.1"`
with 65 active requirements and records that immutable reviewed candidate.
The deleted `decomposition.csv` remains recoverable from Git history as D3
migration input; it is not v5 authority or roadmap.

## v5.1 active owners

| Owner | Active responsibility | Count |
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

Active total: 65. The machine manifest is authoritative for the exact active
identities; this table is navigation. Source admission still requires an exact
ADR-0013 tranche; revision activation alone authorizes no Rust edit.

## Rules

- One active requirement has exactly one registered active definition.
- Architecture sources own cross-domain invariants; specifications own their
  exact detailed contracts without overriding architecture.
- Machine artifacts project named fields and evidence; they are not parallel
  prose owners.
- `docs/requirements.csv` retains profile/package/evidence metadata for all 121
  stable identities. It is not the authority selector; classification and
  active ownership comes from `v5-authority-reset.toml`.
- Historical text retained inside `planning.md` or `binding-spi.md` is
  authoritative only when its stable id is registered active by the manifest.
- A narrow completed amendment remains an owner only for an identity registered
  to its path. Other affected-id mentions are history or refinement context.
- Work packages describe migration and evidence. A deferred requirement may
  trigger domain-entry review but cannot satisfy admission or close a gate.
- An active revision does not admit source implementation merely because its
  normative text and checker pass.
- A conflict blocks the affected implementation; file order does not resolve
  it.

## v5.1 domain-entry change

v5.1 moves exactly these identities from
`inactive-domain-entry-review-required` to active authority:

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

In active v5.1, 31 domain-entry identities remain deferred. They cover
long-lived subscription/emission behavior, Directory client execution,
codec/validation breadth, advanced planning/lazy/cache/index behavior,
observability/status, scheduling, and profile defaults. Each affected domain
must re-adopt, replace, split, or retire its identities in a later reviewed
revision before implementation admission.

The 15 historical design inputs remain useful constraints or measurements but
do not freeze an implementation. The four premature/superseded identities stay
retired. The six redundant identities remain discharged by stronger governance,
authority, or executable owners. The classified total remains exactly 121.

## v5.1 activation record

The docs-only candidate at `3b133ebfe3c870102931982d6c056595f9d44255` passed independent architecture-level
acceptance after the Consumer response validator ownership projection was moved
from Planning to Core and enforced by the candidate checker.

The activation checkpoint:

- accepts and indexes ADR-0019;
- selects v5.1 as the active authority while preserving 65/31/121;
- promotes the reviewed Consumer one-shot ownership and metadata checks into the
  normal active-v5.1 checker path;
- migrates the reviewed WP-100/WP-200/WP-300/WP-400 Consumer Property Read
  slices into the active work-package documents;
- marks workspace/0061 `MIGRATED`; and
- removes candidate-only checker/package-projection files.

It does not register `CONSUMER-PROPERTY-READ-ARCHITECTURE`, admit a source
tranche, or activate any additional deferred identity. Those steps begin only
after this activation checkpoint is integrated.
