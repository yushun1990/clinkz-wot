# WP-100 Deadline and Cleanup Timing Entry Audit

Status: Passed

Design revision: v4.9

Admission scope: `WP-100-DEADLINE-CLEANUP-TIMING`

Verdict: Implementation-ready

ADR-0016 and `docs/amendments/WP-100-time-domain-v1.md` freeze the logical
clock domain and Core timing behavior, but that migration is not source
implementation admission. The independent root review recorded at
`e37224fc9978ad16c0d76639bac58f0e221d824d` confirmed the exact registered
candidate, scope, exclusions, predecessor, fixture, and all eight
pre-implementation checks.

## Scope

This tranche changes exactly three Core source paths:

- `core/src/deadline.rs`, which defines the frozen public `Deadline` value;
- `core/src/status.rs`, where `CleanupRecord::try_with_timing` uses checked
  logical instant ordering; and
- `core/src/lib.rs`, which exposes the Deadline module and root re-export.

The affected requirements are exactly:

- `API-SURFACE-001`;
- `CLEANUP-RECORD-001`;
- `HANDLER-CANCEL-001`;
- `HANDLER-CANCEL-002`; and
- `TIME-001`.

The public API items changed by this tranche are exactly `Deadline` and the
existing `CleanupRecord` timing constructor. Existing `CoreError`,
`ErrorContext`, `ErrorPhase`, and `RetryClass` representations are consumed by
the contract fixture but are not changed.

The completion key is exactly `deadline-cleanup-timing`.

## Frozen behavior

`Deadline` has one private `Option<MonotonicInstant>` field, derives
`Clone`, `Copy`, `Debug`, `Default`, `Eq`, `Hash`, and `PartialEq`, and exposes
only:

- `Deadline::NONE`;
- `Deadline::at`;
- `Deadline::instant`; and
- `Deadline::checked_is_elapsed_at`.

No deadline returns `Some(false)` for every observation. A finite deadline
returns `Some(false)` before its instant and `Some(true)` at or after it.
Different clock ids return `None`; raw wrap metadata never participates.

`CleanupRecord::try_with_timing` requires every supplied instant to use the
runtime clock id. When both retry and terminal instants exist, their
`MonotonicInstant::checked_cmp` result must be `Less` or `Equal`. `Greater` or
`None` is rejected.

The independent fixture freezes the incomparable-clock disposition:

- caller-supplied admission mismatch becomes
  `Validation/Admission/Never`;
- post-admission handler mismatch becomes
  `InternalInvariant/Handler/Never`;
- post-admission binding mismatch becomes
  `InternalInvariant/Binding/Never`; and
- post-admission cleanup mismatch becomes
  `InternalInvariant/Cleanup/Never`.

The fixture also freezes the timeout race oracle: success already published at
the timeout cancellation linearization point wins; otherwise an elapsed
deadline produces `TimedOut` and later success is discarded. This tranche does
not implement a dispatcher, binding driver, scheduler, or cancellation state
machine; those future owners must consume the frozen value and disposition
contract.

## Exact exclusions

This tranche does not:

- modify Foundation time code or reconstruct raw wrap epochs in Core;
- change any public time, error, identity, cleanup-record, or runtime-clock
  representation other than adding the already frozen `Deadline` value;
- add handler traits, request/context migration, storage, scheduling, binding,
  Servient, Producer, or protocol behavior;
- change a state machine, resource schema, queue, retry policy, or performance
  workload;
- remove an old API;
- implement the future interaction, binding, or cleanup cancellation owners;
  or
- rewrite historical WP-000 evidence or the completed logical-time evidence.

Discovering a required source change outside the three registered
implementation paths revokes admission pending impact review.

## Dependency and evidence disposition

The only direct predecessor is `WP-100-LOGICAL-TIME-CORRECTION`. It is
approved, complete, and evidenced by
`docs/evidence/WP-100-logical-time-correction.toml`. Its completion checker
proves the extended logical domain, source timestamp comparison, raw overflow
epoch fixture, reset, exhaustion, and the immutable historical-evidence
disposition.

This Core tranche does not replace or reaffirm WP-000 claims again. It closes
the remaining Core half of the registered `TIME-DOMAIN-AND-DEADLINE` corrective
scope. Broad handler entry remains independently blocked by its own admission
state and remaining handler evidence even after this time-specific completion
key passes.

## Contract fixture

The nested fixture under
`tools/compile-contracts/wp100-deadline-cleanup-timing/` consumes the real
Core and Foundation root surfaces and proves:

- the exact public Deadline construction and copy/default behavior;
- private Deadline storage through a negative UI compilation target;
- no-deadline, before, equal, after, delayed, and different-clock results;
- CleanupRecord retry/deadline ordering and runtime-clock validation;
- the four exact incomparable-clock error dispositions;
- success-before-timeout and timeout-before-late-success linearization; and
- all three Core feature cells.

The fixture's small boundary oracles are executable projections of the
normative table. They do not create new library API or claim that downstream
runtime owners are already implemented.

## Risk, resources, and performance

This is Category C implementation because it projects a public time value and
a cleanup-ordering invariant. The required Category C design controls are
already satisfied by workspace topic 0007, ADR-0016, the frozen time-domain
amendment, Review 06, the two-tranche dependency split, and explicit prior
evidence disposition. The candidate still requires its own ADR-0013
independent review and approval.

The implementation is constant-space and constant-time. It allocates no
memory, starts no work, retains no additional state, performs no I/O, and adds
no lock, critical section, callback, queue, or retry loop. No performance
workload applies.

## Authoritative artifacts

- `docs/ADRs/0013-work-package-scoped-implementation-admission.org`
- `docs/ADRs/0014-transitional-normative-ownership.org`
- `docs/ADRs/0016-extended-logical-monotonic-time.org`
- `docs/amendments/WP-100-error-cleanup-v1.md`
- `docs/amendments/WP-100-handler-api-v1.md`
- `docs/amendments/WP-100-time-domain-v1.md`
- `docs/api-ownership.csv`
- `docs/design.md`
- `docs/requirements.csv`
- `docs/work-packages/WP-100-core.md`
- `docs/work-packages/index.toml`

## Candidate and independent review

The work-package index owns the exact candidate base, candidate commit, changed
path set, implementation paths, contract artifacts, prechecks, audit, entry
check, and completion key. The candidate commit must be the single child of the
logical-time completion checkpoint and must not change any registered Core
implementation path.

The independent review must inspect the registered commit rather than an
uncommitted worktree. A root continuation review records
`reviewer_attestation_kind = "independent-root-session"` and
`reviewer_id = "codex-agent:/root"`; a separately spawned reviewer records its
real canonical child task id. The attestation commit is limited to its TOML
record and artifact-registry row.

## Pre-implementation checks

The candidate entry check reruns:

- `api-ownership-check`;
- `architecture-adr-check`;
- `design-requirement-check`;
- `resource-profile-check`;
- `work-package-dag-check`;
- `wp100-amendment-check`;
- `wp100-handler-amendment-check`; and
- `wp100-logical-time-correction-check`.

It validates the exact candidate commit and path boundary, audit state,
governance mapping, fixture lockfile, completed predecessor, empty
state/resource/performance/removal scope, and expected pre-code completion
failure. Before implementation, the completion checker must fail only because
the real Core Deadline implementation is absent.

## Completion evidence

Before this tranche becomes complete:

- its independent review attestation and exact three-file approval checkpoint
  must exist;
- the exact two-file progress checkpoint must precede source changes;
- `tools/check-wp100-deadline-cleanup-timing.sh` must pass;
- `docs/evidence/WP-100-deadline-cleanup-timing.toml` must record passed
  `deadline-cleanup-timing` evidence and the exact implementation commit;
- the implementation commit must change only the three registered Core source
  paths;
- all three Core feature cells, semantic fixture, negative privacy fixture,
  Core unit tests, and logical-time predecessor check must pass; and
- the `TIME-DOMAIN-AND-DEADLINE` impact must change from blocking to resolved.

This candidate does not admit broad `WP-100-HANDLER-ENTRY`.
