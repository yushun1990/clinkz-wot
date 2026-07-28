# v5.0 Authority-Reset Specifications

Status: active v5.0 authority, independently reviewed and integrated.

ADR-0018 replaces the v4.9 residual-decomposition program with one bounded
authority set. `v5-authority-reset.toml` classifies all 121 inherited identities
and registers the only sources allowed to define the 62 active requirements.
The deleted `decomposition.csv` remains recoverable from Git history as D3
migration input; it is not a v5 authority or roadmap.

## Active candidate owners

| Owner | Active responsibility | Count |
| --- | --- | ---: |
| `docs/design.md` | revision, authority, admission, API ownership, profiles, standards | 6 |
| `docs/architecture/10-primary-data-flows.md` | concurrency, callbacks, linearization, Directory scope | 5 |
| `docs/architecture/20-module-boundaries.md` | dependency direction | 1 |
| `foundation.md` | resource policy, admission memory, constrained storage/work | 8 |
| `docs/amendments/WP-100-time-domain-v1.md` | logical source time and deadline semantics | 2 |
| `interaction-core.md` | handler, value, error, cleanup, in-flight contracts | 10 |
| `docs/amendments/WP-100-handler-api-v1.md` | exact completed handler values | 1 |
| `runtime-safety.md` | representation, security, progress, lifecycle safety | 10 |
| `planning.md` | Property Read planning and immutable-plan core | 8 |
| `binding-spi.md` | registration, routes, delivery, I/O, cancellation, exposure | 11 |

Total: 62. The machine manifest is authoritative for the exact identities;
this table is navigation.

## Rules

- One active requirement has exactly one registered definition.
- Architecture sources own cross-domain invariants; specifications own their
  exact detailed contracts without overriding architecture.
- Machine artifacts project named fields and evidence; they are not parallel
  prose owners.
- `docs/requirements.csv` retains profile/package/evidence metadata for all 121
  stable identities. Under v5 it is not an authority selector; classification
  and active ownership come only from `v5-authority-reset.toml`.
- Historical text retained inside `planning.md` or `binding-spi.md` is visibly
  labelled inactive and cannot authorize work.
- A narrow completed amendment remains an active owner only for an identity
  registered to its path. Other affected-id mentions are history or refinement
  context.
- Work packages describe migration and evidence. An inactive requirement may
  trigger domain-entry review but cannot satisfy admission or close a gate.
- A conflict blocks the affected implementation; file order does not resolve
  it.

## Inactive dispositions

The 34 domain-entry identities must be reconsidered when subscriptions,
Directory client execution, codecs/validation, discovery, emissions, advanced
planning, observability, scheduling, or profile defaults enter implementation.
That review may re-adopt, replace, split, or retire them in a new revision.

The 15 historical design inputs retain useful constraints or measurements but
do not freeze an implementation. The four premature/superseded identities are
retired. The six redundant identities defer to stronger governance, authority,
or executable evidence owners. All 59 remain searchable in the transition
manifest and at the immutable v4.9 reset base.

## Activation protocol

The exact candidate must:

1. contain only documentation and checker changes;
2. define the 62 active identities exactly once in the registered sources;
3. prove all 59 other identities inactive and absent from active ownership;
4. disposition every completed evidence claim without treating an inactive id
   as v5 gate evidence;
5. preserve the WP-200 representation/fixture blocker from issue 0014;
6. pass the aggregate candidate checks and implementation regressions; and
7. receive an independent immutable-commit attestation.

Only a separate mainline checkpoint may activate the reviewed commit. That
checkpoint updates the revision selector and continuation state; it does not
rewrite the reviewed owner boundary. Rollback restores the registered v4.9
reset basis and invalidates v5-only admissions.
