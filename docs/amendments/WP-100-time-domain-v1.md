# WP-100 Logical Time, Deadline, and Cleanup Timing Amendment

Status: Frozen

Base design revision: v4.9

Amendment id: WP-100-TIME-DOMAIN-001

Affected requirements: API-SOURCE-TIME-001, API-SURFACE-001,
CLEANUP-RECORD-001, HANDLER-CANCEL-001, HANDLER-CANCEL-002, TIME-001

## Authority and scope

This registered normative amendment owns the D2 refinement boundary for
Foundation monotonic/source time, the Core Deadline value, CleanupRecord timing
validation, and incomparable-clock error disposition. It integrates ADR-0016
without changing crate ownership, public value layouts, the RuntimeClock method
set, or timeout linearization rules.

It supersedes the residual half-period modular-comparison text in
`docs/design.md` and the deferred time semantics in
`docs/amendments/WP-100-handler-api-v1.md`. The handler amendment continues to
own non-time handler values, traits, storage, and cancellation state machines.
This amendment does not admit implementation. The work-package index owns the
two-tranche corrective plan and its completion-evidence identities; each
tranche still requires an ADR-0013 admission record before source changes.

## Logical monotonic domain

`MonotonicInstant { clock_id, ticks }` stores an extended logical `u64` tick,
not an unqualified hardware-counter sample. For one live `ClockId`:

- `RuntimeClock::ticks_per_second()` is nonzero and immutable;
- successive `RuntimeClock::now()` results are monotonic nondecreasing;
- exposed ticks never wrap or decrease;
- `checked_add_ticks` and duration conversions fail before logical overflow;
- ordering, subtraction, and elapsed-duration conversion require the same
  `ClockId`; and
- Core never reads wall-clock time or infers an epoch from raw ticks.

The public RuntimeClock contract remains:

```rust
pub trait RuntimeClock {
    fn now(&self) -> MonotonicInstant;
    fn ticks_per_second(&self) -> core::num::NonZeroU64;

    fn wrap_period_ticks(&self) -> Option<core::num::NonZeroU64> {
        None
    }
}
```

`wrap_period_ticks()` is v1 diagnostic metadata for the underlying raw source.
It does not describe the exposed logical domain, does not select a comparison
algorithm, and does not bound deadlines or leases. A finite raw counter must be
extended inside the clock adapter before `now()` constructs a
`MonotonicInstant`.

An adapter may use a hardware overflow epoch, caller-owned state, a critical
section, or a host-native wide clock. Successive-sample wrap detection is valid
only when the adapter enforces a sampling interval that makes missed wraps
impossible. A manual-poll consumer has no such implicit guarantee.

## Reset and exhaustion

A raw reset, lost extension epoch, scale change, or adapter restart retires the
old `ClockId`. The replacement id is not reused while any retained timestamp,
deadline, cleanup record, or operation from the old domain can remain.
Different ids are incomparable and fail closed.

Before logical `u64` exhaustion, positive-duration admission fails when checked
addition cannot represent the deadline. The old domain may remain saturated at
`u64::MAX` while admitted work expires and drains. A runtime changes to a new
id only after no live time-bearing state from the old domain remains. Ticks
never wrap or reset under one id.

## Source timestamps

`API-SOURCE-TIME-001`: Source retrieval, freshness, and lease timestamps use
the following checked time-domain contract.

The SourceTimestamp representation remains:

```rust
pub enum SourceTimestamp {
    Monotonic {
        clock_id: ClockId,
        ticks: u64,
        ticks_per_second: core::num::NonZeroU64,
    },
    UnixMillis(i64),
    Unknown,
}
```

Monotonic `ticks` use the extended logical domain. The exact checked comparison
is:

- two Monotonic values compare only when both `clock_id` and
  `ticks_per_second` match;
- two UnixMillis values compare by their integer millisecond values;
- Unknown, mixed kinds, different ids, and conflicting scales return `None`.

`SourceTimestamp::checked_cmp(self, other)` owns that behavior.
`monotonic_instant()` remains a projection for already-validated monotonic
metadata; callers must not use it to bypass scale validation between two source
records.

## Deadline

`TIME-001`: Timeout races use the operation's existing linearization point. If
terminal success is published before timeout cancellation, success wins;
otherwise the caller receives `TimedOut` and a later success is discarded.
Every monotonic deadline and timeout comparison uses the logical domain below.
Clock-domain incomparability follows the separate fail-closed disposition and
is never reported as `TimedOut`.

The Core-owned public value is frozen as:

```rust
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Deadline {
    instant: Option<clinkz_wot_foundation::MonotonicInstant>,
}

impl Deadline {
    pub const NONE: Self = Self { instant: None };

    pub const fn at(
        instant: clinkz_wot_foundation::MonotonicInstant,
    ) -> Self;

    pub const fn instant(
        self,
    ) -> Option<clinkz_wot_foundation::MonotonicInstant>;

    pub fn checked_is_elapsed_at(
        self,
        now: clinkz_wot_foundation::MonotonicInstant,
    ) -> Option<bool>;
}
```

`Deadline::NONE.checked_is_elapsed_at(now)` returns `Some(false)`. A finite
same-clock deadline returns `Some(now >= instant)`. Different clock ids return
`None`. Raw wrap diagnostics never participate.

## Incomparable-clock disposition

The value API reports incomparability as `None`. Boundaries map it as follows:

| Boundary | CoreError category | ErrorPhase | RetryClass | Required behavior |
| --- | --- | --- | --- | --- |
| Caller-supplied deadline or timing admission | `Validation` | `Admission` | `Never` | Reject before publication or side effects |
| Interaction deadline after admission | `InternalInvariant` | `Handler` | `Never` | Begin cancellation; do not report `TimedOut` |
| Binding call/driver deadline after admission | `InternalInvariant` | `Binding` | `Never` | Begin binding cancellation/cleanup |
| Cleanup timing after admission | `InternalInvariant` | `Cleanup` | `Never` | Retain or transfer cleanup ownership |

A later value is discarded under the existing late-result rules. No boundary
reinterprets a different clock id as elapsed or not elapsed.

## Cleanup timing

`CleanupRecord::try_with_timing` accepts only instants using the supplied
runtime `ClockId`. When both values exist,
`retry_not_before.checked_cmp(deadline)` must be `Less` or `Equal`; `Greater`
or `None` is rejected. This comparison is ordinary checked logical ordering,
not raw tick or half-period modular ordering.

## Corrective tranches and evidence

The correction is split because ownership, evidence truth, and validation
boundaries differ:

1. `WP-100-LOGICAL-TIME-CORRECTION` depends on
   `WP-100-FOUNDATION-REFRESH`, owns `foundation/src/time.rs`, and verifies
   logical ordering, raw-wrap extension, delayed observation, source timestamp
   comparability, reset, and exhaustion in all three feature cells. Its
   completion key is `logical-time-domain-correction`.
2. `WP-100-DEADLINE-CLEANUP-TIMING` depends on
   `WP-100-LOGICAL-TIME-CORRECTION`, owns the Core Deadline module/root export
   and CleanupRecord timing validation/tests, and verifies error disposition
   plus delayed timeout behavior. Its completion key is
   `deadline-cleanup-timing`.

Foundation unit tests own logical ordering, raw-wrap extension, delayed
observation, source timestamp comparability, reset, scale, and exhaustion.
Generated resource-schema snapshots and boundary tests continue to own the
disjoint `API-TYPES-001` and `CONSTRAINED-STORAGE-001` generation contract.

The `TIME-DOMAIN-AND-DEADLINE` scope continues to block broad
`WP-100-HANDLER-ENTRY` until both corrective tranches have passed completion
evidence. The already completed five-value handler tranche remains disjoint.

## Required fixtures

The two completion contracts together must cover:

- same-clock before/equal/after ordering;
- different-clock incomparability;
- a finite raw counter extended across one and multiple wraps;
- delayed polling beyond half and whole raw periods;
- reset and lost-epoch ClockId replacement;
- checked addition at `u64::MAX`;
- SourceTimestamp id and scale mismatch;
- CleanupRecord retry/deadline ordering; and
- Deadline NONE, finite, mismatch, and error-disposition behavior.
