# WP-100 Handler Value Primitives Entry Audit

Status: Pending

Design revision: v4.9

Admission scope: `WP-100-HANDLER-VALUE-PRIMITIVES`

Verdict: Independent re-review pending

Review 05 rejected the broad handler entry and identified a smaller additive
boundary. This record remains pending until an independent reviewer confirms
the exact registered revision, scope, exclusions, and pre-implementation
checks.

## Scope

This admission is limited to five currently absent public Core values:

- `CancellationView`;
- `SubscriptionAcceptance`;
- `HandlerFootprint`;
- `HandlerStep`; and
- `StaticHandlerRegistration`.

Permitted implementation paths are exactly `core/src/handler.rs` and the root
re-export in `core/src/lib.rs`. The nested compile-contract fixture, actual-source
validator, entry/completion checkers, and declared evidence records are frozen
admission or completion artifacts, not implementation paths. The change is
additive and removes or renames no current item.

The affected requirements are exactly `API-SURFACE-001` and
`HANDLER-VALUE-001`. The latter completely owns the five schemas' attributes,
ownership, and passive value semantics. Subscription, storage, cancellation,
work-budget, ownership-admission, and resource-limit requirements belong to the
later traits, state records, and admission items that consume these values. The
candidate and the `TIME-DOMAIN-AND-DEADLINE` blocking scope share only the
global `API-SURFACE-001` meta-requirement; their API-item and behavioral
requirements are otherwise disjoint.

## Exact exclusions

The tranche does not implement or change:

- `AcceptHint`, `InteractionInput`, `HandlerContext`, or `AffordanceTarget`;
- `Deadline`, `ClockId`, `MonotonicInstant`, `RuntimeClock`, `SourceTimestamp`,
  or `CleanupRecord` timing validation;
- any synchronous, async, or bounded-step handler trait;
- `HostHandlerRegistration`, its factories or ingress;
- handler slots, stores, replacement, dispatch, execution owners, or reducers;
- Producer subscription state or emission;
- Servient or Protocol Binding APIs;
- any state-machine transition, old-API removal, or performance workload.

Discovering a required change outside the two implementation paths or these
five API items revokes admission pending impact review.

The design/tooling candidate is based on full commit
`8c89e9346f424923ef3247dd1c402d5ab141c203`. Its exact non-implementation
diff path set is frozen as `candidate_paths` in the work-package index. The
independent attestation must review the candidate's single child commit of that
base; neither implementation path may appear in that commit.

## Dependency and module-boundary verdict

`WP-100-FOUNDATION-REFRESH` is complete under
`docs/evidence/WP-100-foundation-refresh.toml`. The tranche owns only
`clinkz-wot-core` and composes existing Core result, output, and slot-id values
without adding a dependency or changing the established downward direction.

Core owns protocol-neutral handler values. Servient owns application-facing
host setters and orchestration; binding packages own protocol execution. The
tranche exposes no registration ingress or mutable runtime owner and therefore
does not move either downstream responsibility into Core.

## API and ownership contract

- `CancellationView` derives exactly `Clone`, `Copy`, `Debug`, `Default`, `Eq`,
  `Hash`, `Ord`, `PartialEq`, and `PartialOrd`; it is `#[repr(u8)]`, defaults to
  `Active`, has exact discriminants `Active = 0` and `Requested = 1`, and its
  sole `is_requested(self)` method is `const`.
- `SubscriptionAcceptance` owns one private `InteractionOutput`, derives only
  `Debug`, `Eq`, and `PartialEq`, carries the exact successful-acceptance
  `must_use` message, and implements none of `Clone`, `Copy`, or `Default`.
  `new` and `response` are `const`; `into_response(self)` is the non-const
  linear extraction path.
- `HandlerFootprint` derives exactly `Clone`, `Copy`, `Debug`, `Eq`, `Hash`,
  `Ord`, `PartialEq`, and `PartialOrd`, and does not implement `Default`. It is
  the exact private three-`u64` record in retained/pending-call/subscription
  order; `new` and all three by-value getters are `const` and preserve every
  input, including zero and `u64::MAX`.
- `HandlerStep<R>` has no bound on `R`, derives only `Debug`, `Eq`, and
  `PartialEq`, carries bare `#[must_use]`, and has exactly the exhaustive
  variants `Pending` and `Ready(CoreResult<R>)`. It is not `Clone`, `Copy`,
  `Default`, or `non_exhaustive`.
- `StaticHandlerRegistration<'h, H>` has no bound on `H`, contains exactly the
  private slot, borrowed handler, and footprint fields, and exposes only the
  `const` constructor and three `const` getters. Manual `Copy`, `Clone`, and
  `Debug` work when `H` implements none of those traits; Debug includes slot and
  footprint, omits the handler entirely, and finishes non-exhaustively.

The handler module may not introduce `Arc`, `Box`, `std`, an executor, a
runtime dependency, a queue, or a callback. This tranche does not claim the
future incapable-target evidence for `AffordanceTarget` or the complete Core
surface.

## Resource and performance assessment

`HandlerFootprint` and `StaticHandlerRegistration` declare values used by later
admission logic; they do not reserve or mutate a resource account. The other
three values are passive ownership/status records. Construction, getters, and
moves add no independent variable-size algorithm. Derived comparison or
formatting may delegate to a contained result or output and is not claimed to
be constant-cost.

No performance workload applies because the tranche adds no loop, allocation,
queue, state transition, handler invocation, or scheduling algorithm. The
future storage/replacement and cancellation tranches remain responsible for
GW020/GW021/CS015/CS016 and their complete-matrix oracle.

## Isolation from open findings

AR-002/AR-003 concern constructible binding registration/compiler/constrained
SPI and authoring fixtures. AR-004 concerns candidate fallback. H-7 concerns
storage/cancellation workload coverage. H-8 belongs to the binding SPI. H-9
concerns `AffordanceTarget` and a real no-atomic build. Review 06 and
`workspace/0007-time-domain-and-deadline.md` isolate the finite-clock finding
and `Deadline` in the `TIME-DOMAIN-AND-DEADLINE` blocking scope. That scope is
an impact placeholder rather than a defined corrective tranche. None of those
contracts is read or implemented by these five values.

## Authoritative artifacts reviewed

- `docs/ADRs/0013-work-package-scoped-implementation-admission.org`
- `docs/ADRs/0014-transitional-normative-ownership.org`
- `docs/amendments/WP-100-handler-api-v1.md`
- `docs/api-ownership.csv`
- `docs/design.md`
- `docs/requirements.csv`
- `docs/work-packages/WP-100-core.md`
- `docs/work-packages/index.toml`

Supporting evidence includes `docs/reviews/review-05.org` and
`docs/reviews/review-06.org`, `workspace/0007-time-domain-and-deadline.md`, and
`docs/evidence/WP-100-foundation-refresh.toml`.

## Pre-implementation contract artifacts

The executable acceptance contract is frozen before implementation in:

- `tools/design-check/Cargo.toml` and
  `tools/design-check/src/main.rs`, which own the structured actual-source and
  tranche-state validators;
- `tools/check-wp100-handler-value-primitives-entry.sh` and
  `tools/check-wp100-handler-value-primitives.sh`;
- `tools/compile-contracts/wp100-handler-value-primitives/Cargo.toml` and its
  committed `Cargo.lock`;
- `tools/compile-contracts/wp100-handler-value-primitives/src/lib.rs`;
- `tools/compile-contracts/wp100-handler-value-primitives/tests/semantics.rs`;
- the three independent private-field targets under
  `tools/compile-contracts/wp100-handler-value-primitives/ui/`; and
- the two independent `must_use` targets in the same UI directory.

The fixture is an independent nested workspace. It consumes the real Core root
surface but is not discovered by ordinary root-workspace commands while the API
is absent. The completion checker also invokes the actual-handler-source AST
validator, so a self-contained mirror, comment marker, or unrelated compile
failure cannot satisfy the contract.

## Pre-implementation checks

The exact prechecks are:

- `api-ownership-check`;
- `architecture-adr-check`;
- `resource-profile-check`;
- `work-package-dag-check`;
- `wp100-amendment-check`; and
- `wp100-handler-amendment-check`.

`tools/check-wp100-handler-value-primitives-entry.sh --candidate` executes the
six leaf checks, validates this review-pending state, and proves that the
completion checker stops only at the absent implementation boundary. After an
independent exact-commit attestation is committed, the `--admission-ready` mode
requires the completed predecessor, validates the candidate/review checkpoint
chain and bounded approval diff, reruns every precheck, and rejects an
implementation that started before approval.

`wp100-handler-value-primitives-check` is an executable, pre-frozen post-code
completion check. Before implementation it is expected to fail only because
`core/src/handler.rs` and the five root exports do not yet exist. Its nested
compile-contract is outside the root workspace, so ordinary root checks remain
green while the real API is absent.

## Completion evidence

Before the tranche becomes complete:

- `tools/check-wp100-handler-value-primitives.sh` must pass;
- the governance registry must retain the executable completion command;
- `docs/evidence/WP-100-handler-value-primitives.toml` must record passed
  `handler-value-primitives` evidence for v4.9 and reference the implementation
  commit;
- real Core builds must pass for `no-default`, `async-no-std`, and `std`;
- the nested contract fixture and actual-source validator must prove the exact
  attributes, fields, variants, const API, value/ownership, generic-bound,
  private-field, borrowing, must-use, and redacted-Debug rules; and
- the tranche may then move from `pending` to `complete`.

This candidate does not approve the broad `WP-100-HANDLER-ENTRY`.
