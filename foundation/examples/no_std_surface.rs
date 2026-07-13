//! Compile fixture for the portable foundation public surface.

#![no_std]

use core::num::NonZeroU64;

use clinkz_wot_foundation::{
    AdmissionLedger, BenchmarkStaticReferenceV1, BudgetExceeded, ClockId, DirectoryClientDefaultV1,
    GatewayDefaultV1, Generation, MonotonicInstant, ResourceAccount, ResourceKind, ResourceLimits,
    ResourceProfileId, ResourceReservation, RuntimeClock, SlotIndex, SourceTimestamp,
    StaticResourceProfile, WorkBudget, WorkClass,
};

/// A deterministic clock used only to compile the portable trait contract.
pub struct FixtureClock;

impl RuntimeClock for FixtureClock {
    fn now(&self) -> MonotonicInstant {
        MonotonicInstant::new(ClockId::new(1), 0)
    }

    fn ticks_per_second(&self) -> NonZeroU64 {
        NonZeroU64::MIN
    }
}

/// Exercises the frozen budget paths without requiring `std`.
pub fn consume_one(budget: &mut WorkBudget) -> Result<(), BudgetExceeded> {
    budget.consume(WorkClass::BindingPolls, 1)
}

/// Creates a generation-bearing bounded account.
pub fn account(limit: u64) -> ResourceAccount {
    ResourceAccount::new(
        SlotIndex::new(0),
        Generation::INITIAL,
        ResourceKind::PayloadBytesMax,
        limit,
    )
}

/// Reserves one byte through the move-only public reservation type.
pub fn reserve_one(account: &mut ResourceAccount) -> Option<ResourceReservation<'_>> {
    account.try_reserve(1)
}

/// Creates the six-account admission ledger.
pub fn ledger() -> AdmissionLedger {
    AdmissionLedger::new(SlotIndex::new(0), Generation::INITIAL, 1, 1, 1, 1, 1, 1)
}

/// Returns all three named profile snapshots and their identities.
pub fn profiles() -> [(ResourceProfileId, ResourceLimits); 3] {
    [
        (GatewayDefaultV1::ID, GatewayDefaultV1::LIMITS),
        (
            DirectoryClientDefaultV1::ID,
            DirectoryClientDefaultV1::LIMITS,
        ),
        (
            BenchmarkStaticReferenceV1::ID,
            BenchmarkStaticReferenceV1::LIMITS,
        ),
    ]
}

/// Creates source-time metadata without host wall-clock types.
pub fn source_timestamp() -> SourceTimestamp {
    SourceTimestamp::Monotonic {
        clock_id: ClockId::new(1),
        ticks: 0,
        ticks_per_second: NonZeroU64::MIN,
    }
}
