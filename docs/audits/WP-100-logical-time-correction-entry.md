# WP-100 Logical Time Correction Entry Audit

Status: Passed

Design revision: v4.9

Admission scope: `WP-100-LOGICAL-TIME-CORRECTION`

Verdict: Implementation-ready

ADR-0016 and `docs/amendments/WP-100-time-domain-v1.md` resolve D2, but
architecture migration is not source implementation admission. The independent
root continuation review recorded at
`8180e1916606b80518a0d975a8ce2fa2c2e93953` confirmed the exact registered
candidate, scope, exclusions, predecessor, evidence disposition, contract
fixture, and all seven pre-implementation checks.

## Scope

This tranche is limited to `foundation/src/time.rs` and the existing public
items:

- `ClockId`;
- `MonotonicInstant`;
- `RuntimeClock`; and
- `SourceTimestamp`.

The public representations and the `RuntimeClock` method set do not change.
The source correction:

- documents `MonotonicInstant::ticks()` as an extended logical tick;
- documents `RuntimeClock::wrap_period_ticks()` as raw-source diagnostic
  metadata that never selects comparison or lifetime policy;
- adds `SourceTimestamp::checked_cmp(self, other) -> Option<Ordering>`;
- compares monotonic source timestamps only when clock id and scale both match;
- compares two Unix millisecond timestamps directly;
- treats Unknown, mixed kinds, different ids, and conflicting scales as
  incomparable;
- retains checked non-wrapping instant arithmetic; and
- replaces the current raw-wrapping unit fixture with logical-domain,
  incomparability, and exhaustion tests.

The affected requirements are exactly `API-SOURCE-TIME-001`,
`API-SURFACE-001`, and `TIME-001`. The completion key is exactly
`logical-time-domain-correction`.

## Exact exclusions

This tranche does not implement or change:

- Core `Deadline` or `CleanupRecord`;
- any Core error category, cancellation owner, handler, binding, Servient, or
  scheduler behavior;
- a public hardware-counter adapter or one mandatory epoch-storage strategy;
- the `ClockId`, `MonotonicInstant`, `RuntimeClock`, or `SourceTimestamp`
  representation;
- the `RuntimeClock` method set;
- Foundation generation code;
- historical `docs/evidence/WP-000.toml`;
- a state machine, old-API removal, resource schema, or performance workload.

Discovering a required source change outside `foundation/src/time.rs` revokes
admission pending impact review.

## Dependency and evidence disposition

`WP-100-FOUNDATION-REFRESH` is complete and is the only tranche predecessor.
Foundation remains dependency-free in its normal dependency graph and must
compile in `no-default`, `async-no-std`, and `std`.

Historical WP-000 evidence remains immutable. Completion creates a new WP-100
record which:

- replaces only the `API-SOURCE-TIME-001` and `TIME-001` claims in the
  historical `time-and-generation-api` record; and
- reaffirms its independent `API-TYPES-001` and
  `CONSTRAINED-STORAGE-001` generation claims by rerunning
  `tools/check-wp-000.sh` without changing generation sources.

WP-000 remains complete. This tranche does not rewrite history or claim that
the v4.6 raw-wrap declaration proved delayed deadline ordering.

## Contract fixture

The independent nested fixture under
`tools/compile-contracts/wp100-logical-time-correction/` consumes the real
Foundation root surface and proves:

- same-clock instant before/equal/after ordering;
- different-clock incomparability;
- checked addition at `u64::MAX`;
- SourceTimestamp kind, id, and scale comparison;
- a finite raw counter extended from an externally supplied overflow epoch
  across one and multiple wraps;
- delayed observation beyond a full raw period;
- reset/lost-epoch replacement with a new incomparable `ClockId`; and
- raw wrap period reporting as diagnostics only.

The overflow-epoch fixture is evidence that the public contract supports a
conforming constrained adapter. It is not a proposed public adapter and does
not imply that successive raw samples can recover a missed wrap.

## Resource and performance assessment

The implementation adds one constant-space comparison method and changes
documentation/tests around existing constant-space arithmetic. It allocates no
memory, starts no work, and adds no queue, retry loop, I/O, lock, critical
section, callback, or state-machine transition. No performance workload
applies.

## Authoritative artifacts

- `docs/ADRs/0013-work-package-scoped-implementation-admission.org`
- `docs/ADRs/0014-transitional-normative-ownership.org`
- `docs/ADRs/0016-extended-logical-monotonic-time.org`
- `docs/amendments/WP-100-time-domain-v1.md`
- `docs/api-ownership.csv`
- `docs/design.md`
- `docs/requirements.csv`
- `docs/work-packages/WP-000-foundation.md`
- `docs/work-packages/WP-100-core.md`
- `docs/work-packages/index.toml`

## Candidate and independent review

The work-package index owns the exact candidate base, candidate commit, changed
path set, implementation path, contract artifacts, prechecks, audit, entry
check, and completion key. The registered candidate commit must be the single
child of the D2 migration checkpoint and must not change
`foundation/src/time.rs`.

The independent review inspected the registered candidate rather than an
uncommitted worktree and recorded
`reviewer_attestation_kind = "independent-root-session"` with
`reviewer_id = "codex-agent:/root"`. A separately spawned reviewer would use
its real canonical child task id. The attestation commit is limited to its TOML
record and artifact-registry row.

## Pre-implementation checks

The candidate entry check reruns:

- `design-requirement-check`;
- `api-ownership-check`;
- `architecture-adr-check`;
- `resource-profile-check`;
- `work-package-dag-check`;
- `wp100-amendment-check`; and
- `wp100-handler-amendment-check`.

It also validates the candidate commit/path boundary, audit state, governance
mapping, nested lockfile, completed predecessor, and expected pre-code
completion failure. Before implementation, the completion check must fail only
because the real `SourceTimestamp::checked_cmp` method is absent.

## Completion evidence

Before this tranche becomes complete:

- its independent review attestation and exact three-file approval checkpoint
  must exist;
- `tools/check-wp100-logical-time-correction.sh` must pass;
- `docs/evidence/WP-100-logical-time-correction.toml` must record passed
  `logical-time-domain-correction` evidence and the exact implementation
  commit;
- the implementation commit must change only `foundation/src/time.rs`;
- all three Foundation feature cells and the independent semantic fixture must
  pass;
- the historical WP-000 evidence blob and generation source must remain
  unchanged; and
- `tools/check-wp-000.sh` must pass to reaffirm the generation portion.

This candidate does not admit `WP-100-DEADLINE-CLEANUP-TIMING` or broad
`WP-100-HANDLER-ENTRY`.
