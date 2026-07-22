# WP-100 Foundation Refresh Entry Audit

Status: Passed

Design revision: v4.9

Admission scope: `WP-100-FOUNDATION-REFRESH`

Verdict: Implementation-ready

The first scoped review found the implementation boundary technically closed,
but a second adversarial review identified that AR-006 in
`workspace/0005-architecture-review-findings.md` directly intersects the
by-value `ResourceLimits` and `WorkBudget` APIs. ADR-0014/0015, Review 04, the
v4.9 amendment correction, and the narrowed tranche scope now provide the
required projection. A final independent re-review confirmed this exact
revision, including the ownership rules, negative trait assertions, scope, and
all six pre-implementation checks.

## Scope

This admission is limited to:

- `ResourceKind` and `ResourceLimits`;
- `StaticResourceProfile` and the generated named-profile implementations;
- `WorkClass` and `WorkBudget`;
- `PendingWorkClass`;
- generated profile snapshots and boundary tests; and
- completion evidence for `handler-foundation-refresh`.

It changes no state machine, removes no named public symbol or compatibility
alias, adds no runtime dependency, and implements no planning, Protocol Binding,
handler, Servient, or transport behavior. It does intentionally correct public
trait and implementation properties described below.

The affected requirements are exactly `API-RESOURCE-001`, `API-SURFACE-001`,
`CONSTRAINED-PROGRESS-001`, `CONSTRAINED-STORAGE-002`,
`CONSTRAINED-WORK-001`, `RES-LIMIT-001`, `RES-LIMIT-002`, and
`RES-PROFILE-001`. Handler, planning, binding, and cleanup requirement ids on
individual resource rows remain provenance for later consumers, not behavior
implemented by this tranche.

Permitted implementation paths are limited to the corresponding generator,
foundation budget/resource tests and fixtures, core status values/tests, and
the declared completion-evidence records. Discovering a required behavioral
change outside that set revokes admission pending impact review.

## Dependency verdict

WP-000 is the only package predecessor and remains complete under
`docs/evidence/WP-000.toml`. The tranche has no predecessor tranche. Changes
remain within `clinkz-wot-foundation` and `clinkz-wot-core` and preserve the
existing downward dependency graph and all three feature cells.

## API and ABI impact

- `ResourceKind` indices `0..=138` remain byte-for-byte ordered by the frozen
  prefix hash.
- The 56 v4.9 fields append at indices `139..=194`; indices `139..=156` are the
  18 planning/artifact fields and `157..=194` are the 38 binding-resource
  fields.
- Existing `WorkClass` discriminants `0..=8` remain unchanged and
  `HandlerSteps = 9`.
- `ResourceLimits` remains explicitly `Clone` but is no longer `Copy`.
  `StaticResourceProfile::LIMITS` and `limits()` return
  `&'static ResourceLimits`; a bare profile id is not value authority.
- `WorkBudget` implements neither `Clone` nor `Copy`; one unique mutable value
  carries the remaining allowance.
- Existing `PendingWorkClass` bits through `RouteCleanup = 1 << 10` remain
  unchanged. The append is:
  - `HandlerCall = 1 << 11`
  - `ProducerSubscriptionSetup = 1 << 12`
  - `ProducerSubscriptionTeardown = 1 << 13`
- Adding variants is visible to downstream exhaustive Rust matches, and the
  private by-value layouts of `ResourceLimits` and `WorkBudget` grow. All v1
  crates are rebuilt together through Cargo. No stable dynamic-library ABI is promised.

## Resource and performance assessment

`docs/resource-limits.csv` is already the exhaustive source for this tranche:
195 fields, three named profiles, finite nonzero gateway/static values for the
56-field suffix, and `NA` for the Directory client profile.

No performance workload applies because this tranche adds schema entries and
discriminants but no execution algorithm, queue, state transition, or hot-path
operation. Existing accounting workloads remain owned by WP-000.

On the reviewed x86-64 toolchain, the array representation grows
`ResourceLimits` from 1,888 to 3,120 bytes and `WorkBudget` from 72 to 80 bytes.
The layout growth itself is non-blocking after ADR-0015 because runtime entry
points borrow `ResourceLimits`, static profiles return references, and the
complete profile cannot be copied implicitly. Adding a new by-value runtime
copy exceeds the admitted scope and requires impact review. The numeric layouts
are disclosed observations, not stable ABI or test thresholds.

## Isolation from open planning and binding work

The open registration, constrained SPI, planning-policy, and binding-lifecycle
findings affect consumers of the limits, not the frozen identities, order, or
profile values projected here. This tranche does not implement or assume their
trait signatures, slot layouts, registration constructors, scheduling, or
state transitions.

Later design may append additional resource fields. Reordering, removing, or
changing the meaning of indices `0..=194` requires an ADR-0013 impact review
and revocation or reopening of this tranche.

The resource-accounting usability question in
`workspace/0005-architecture-review-findings.md` is accepted and migrated by
ADR-0015. The tranche introduces no common application operation, retains named
host profiles by reference, and does not expose reservation machinery through
new user flows.

## Authoritative artifacts reviewed

- `docs/ADRs/0013-work-package-scoped-implementation-admission.org`
- `docs/ADRs/0014-transitional-normative-ownership.org`
- `docs/ADRs/0015-borrowed-resource-profiles-and-linear-work-budgets.org`
- `docs/amendments/WP-100-handler-api-v1.md`
- `docs/api-ownership.csv`
- `docs/design.md`
- `docs/resource-limits.csv`
- `docs/work-packages/WP-100-core.md`
- `docs/work-packages/index.toml`

Supporting evidence also included `docs/architecture/20-module-boundaries.md`,
ADR-0009, `docs/work-packages/WP-000-foundation.md`, and
`docs/evidence/WP-000.toml`.

## Pre-implementation checks

All declared checks passed:

- `api-ownership-check` — 686 frozen items.
- `architecture-adr-check` — fifteen accepted ADRs registered.
- `resource-profile-check` — 195 fields and three profiles.
- `work-package-dag-check` — work-package DAG and tranche schema valid.
- `wp100-amendment-check` — schemas, dispositions, and staging frozen.
- `wp100-handler-amendment-check` — API, state, resource, workload, and staging
  projections frozen.

`tools/check-design-artifacts.sh` also passed completely: 120 requirements,
66 performance fixtures/cases, valid state/API/DAG/governance projections, and
8/8 design-check tests.

`tools/check-wp100-foundation-refresh.sh` currently fails at the expected first
post-code condition because the generator remains at v4.6/118. It is the
completion check, not a pre-implementation check.

## Completion evidence

Before the tranche becomes complete:

- `tools/check-wp100-foundation-refresh.sh` must pass;
- the governance check must become `executable`;
- `docs/evidence/WP-100-foundation-refresh.toml` must record passed
  `handler-foundation-refresh` evidence for v4.9;
- all three feature cells, prefix/suffix order, profile snapshots,
  borrowed static profile access, noncopy `ResourceLimits`, nonclone/noncopy
  `WorkBudget`, dependency-free negative trait assertions, exact/one-over
  boundaries, `HandlerSteps`, and pending-work discriminants must pass; and
- the tranche may then move from `pending` to `complete`.

This approval does not admit `WP-100-HANDLER-ENTRY` and does not close or waive
any global convergence gate.
