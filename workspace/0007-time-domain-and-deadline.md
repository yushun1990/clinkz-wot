# 0007 Time Domain and Deadline

Status: MIGRATED

## Scope and authority

This topic records the cross-cutting time-domain problem discovered while
reviewing the proposed `WP-100-HANDLER-VALUE-PRIMITIVES` tranche and the
reasoning that resolved it. It is decision history, not an implementation
authorization.

ADR-0016 and `docs/amendments/WP-100-time-domain-v1.md` own the accepted
technical contract. `docs/work-packages/index.toml` projects its exact
two-tranche correction as the structured `TIME-DOMAIN-AND-DEADLINE` blocking
scope. That record keeps broad handler entry blocked; it does not itself admit
either corrective tranche.

The affected surface is wider than the proposed Core `Deadline` value. It
includes the foundation clock domain, retained source timestamps, existing
cleanup timing records, future dispatcher timeout checks, and the deadline
values returned by binding calls and subscription drivers.

## Problem

The active material currently combines two incompatible clock models:

- `docs/design.md` says that a finite-width clock declares a wrap period and
  that admitted deadlines shorter than half that period can use modular
  comparison;
- `foundation::MonotonicInstant::checked_cmp` explicitly compares only
  non-wrapping tick values, while `checked_duration_since` and
  `checked_add_ticks` also use ordinary checked `u64` arithmetic;
- `RuntimeClock::wrap_period_ticks` exposes a raw wrap period, but that policy
  is not carried by `MonotonicInstant`;
- the proposed `Deadline` stores only one `MonotonicInstant`, and
  `checked_is_elapsed_at` receives only another instant; and
- `CleanupRecord::try_with_timing` validates retry order by comparing raw tick
  values after checking only the clock id.

For a raw period of 256, a deadline at tick 250 and a later observation at tick
3 cannot be ordered by the proposed `Deadline` API. The same problem affects a
cleanup retry or terminal deadline crossing that wrap. The half-period rule is
also insufficient when a constrained manual-poll runtime is not driven until
more than half a period after expiry: a stateless modular comparison cannot
distinguish a late observation from an observation before the deadline.

## Evidence and admission impact

The completed WP-000 evidence record `time-and-generation-api` claims
`TIME-001` and includes finite-clock wrap declaration coverage. That evidence
proves that a wrap period can be reported; it does not prove that deadlines,
duration ordering, source freshness, or cleanup timing remain correct across a
raw wrap. The accepted disposition preserves the historical record, replaces
its time portion with new WP-100 logical-time evidence, and reaffirms its
disjoint generation portion. This does not invalidate the WP-000 completion
checkpoint or the disjoint WP-100 resource-schema and linear-budget evidence.

`Deadline` cannot enter implementation until the predecessor logical-time
correction is complete and its own Core tranche is admitted. Because it is a
prerequisite of request admission and future binding/Servient scheduling,
`WP-100-HANDLER-ENTRY` remains blocked until both time tranches complete.

The other five proposed passive values are disjoint from the clock model:

- `CancellationView`;
- `SubscriptionAcceptance`;
- `HandlerFootprint`;
- `HandlerStep<R>`; and
- `StaticHandlerRegistration<'h, H>`.

They contain no instant, clock id, duration, wrap policy, timeout transition,
or source timestamp. The completed containment removed `Deadline` and
`TIME-001` from their value-primitives candidate and moved all time impact into
the separate blocking scope. The five-value tranche depends only on the
completed foundation refresh; the time scope separately blocks broad handler
entry.

## Alternatives considered

### A. Isolate Deadline now

Split `Deadline` from the first value-primitives tranche and open an independent
time-domain design. This keeps unrelated additive values reviewable and makes
the cross-package impact explicit. It is a containment and sequencing action,
not a solution to the clock semantics.

### B. Carry wrap policy with Deadline comparisons

Add a linear/modular policy or wrap period to `Deadline`, or pass that policy to
every `checked_is_elapsed_at` call. This supports a bounded modular comparison
without state only while every observation remains within the unambiguous
half-period window.

It also duplicates immutable clock-domain metadata in consumers, permits the
same `ClockId` to be paired with conflicting policies, expands deadline and
cleanup records, and requires corresponding changes to `SourceTimestamp` and
its `monotonic_instant` projection. It still fails after an unobserved half
period or multiple raw wraps, so making it correct eventually requires the
epoch state described by alternative C.

### C. Require extended non-wrapping logical ticks

Keep `MonotonicInstant { clock_id, ticks: u64 }` and the current checked
comparison APIs, but define `RuntimeClock::now()` as returning an extended
logical tick value. For one live `ClockId`, observations must be monotonic
nondecreasing and must not wrap. A finite hardware counter is widened by its
clock adapter before it enters foundation values.

The clock source owns the necessary epoch or overflow state. A constrained
single-owner adapter may use caller-owned interior state or an application
critical section; a host adapter may use its native wide monotonic clock. Core
does not acquire a mandatory lock, allocate, or infer wrap from raw ticks.

An underlying reset or loss of epoch state must not reuse the old `ClockId`.
An adapter that cannot reliably extend its counter cannot expose raw wrapping
ticks as a comparable runtime clock. `SourceTimestamp::Monotonic` uses the same
extended-tick rule; an unqualified raw timestamp is `Unknown` or belongs to a
new incomparable clock domain.

`RuntimeClock::wrap_period_ticks` would be deprecated or explicitly redefined
as diagnostic metadata for the underlying raw source. It would not participate
in `MonotonicInstant`, deadline, cleanup, or freshness ordering. The current
half-period modular-comparison statement would be removed.

## Decision

AI selected A followed by C:

1. isolate `Deadline` so the five clock-independent values remain disjoint;
2. expose non-wrapping extended logical `u64` ticks for each live `ClockId`;
3. retain `wrap_period_ticks()` in v1 as raw-source diagnostics only;
4. retire a clock id on reset, lost epoch state, scale change, or adapter
   restart, without reusing it while old time-bearing state can remain;
5. fail positive-duration admission before logical `u64` overflow and drain
   already admitted work before moving to a new clock id;
6. compare monotonic source timestamps only when id and scale both match;
7. map caller-provided incomparability to `Validation/Admission/Never`, and
   post-admission clock-domain loss to `InternalInvariant/Never` in the owning
   Handler, Binding, or Cleanup phase; and
8. implement Foundation logical time before Core Deadline and cleanup timing.

Alternative B is not recommended because its correctness depends on a polling
interval guarantee that the constrained profile does not provide, while its
metadata and layout cost propagates to every time consumer.

## Candidate API semantics

The preferred direction preserves the current public signatures:

```rust
pub trait RuntimeClock {
    fn now(&self) -> MonotonicInstant;
    fn ticks_per_second(&self) -> core::num::NonZeroU64;

    // Transitional only: raw-source diagnostics, never instant ordering.
    fn wrap_period_ticks(&self) -> Option<core::num::NonZeroU64> {
        None
    }
}
```

The contract to freeze is:

- one live runtime clock uses one stable, non-recycled `ClockId`;
- its scale is immutable;
- its logical ticks are monotonic nondecreasing and never wrap;
- checked addition fails before logical `u64` overflow;
- raw counter wrap is extended inside the adapter; and
- different clock ids remain incomparable and fail closed at deadline
  admission or dispatch.

Under that contract, `Deadline::NONE` returns `Some(false)` for every `now`;
finite before/equal/after observations return `Some(false)`, `Some(true)`, and
`Some(true)` respectively; and a different clock id returns `None`.

## Migration

The decision is migrated into:

- `docs/ADRs/0016-extended-logical-monotonic-time.org`;
- `docs/amendments/WP-100-time-domain-v1.md`;
- the `API-SOURCE-TIME-001` and `TIME-001` projections in
  `docs/design.md` and `docs/requirements.csv`;
- `docs/amendments/WP-100-handler-api-v1.md`;
- `docs/work-packages/WP-000-foundation.md`;
- `docs/work-packages/WP-100-core.md`; and
- the `TIME-DOMAIN-AND-DEADLINE` corrective plan in
  `docs/work-packages/index.toml`.

The corrective identities are `WP-100-LOGICAL-TIME-CORRECTION` followed by
`WP-100-DEADLINE-CLEANUP-TIMING`, with completion keys
`logical-time-domain-correction` and `deadline-cleanup-timing`. Neither is
admitted merely by this migration.
