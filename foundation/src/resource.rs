//! Exhaustive resource policies and rollback-safe admission accounting.

use core::fmt;

use crate::{Generation, SlotIndex};

include!(concat!(env!("OUT_DIR"), "/resource_limits.rs"));

impl fmt::Display for ResourceKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.field_name())
    }
}

/// Stable identity of a named or application-defined resource profile.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResourceProfileId(u16);

impl ResourceProfileId {
    /// An explicitly supplied application profile.
    pub const APPLICATION_DEFINED: Self = Self(0);
    /// The versioned default gateway profile.
    pub const GATEWAY_DEFAULT_V1: Self = Self(1);
    /// The versioned engine-side Directory client profile.
    pub const DIRECTORY_CLIENT_DEFAULT_V1: Self = Self(2);
    /// The versioned constrained benchmark reference profile.
    pub const BENCHMARK_STATIC_REFERENCE_V1: Self = Self(3);

    /// Creates an application-assigned profile identity.
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Returns the stable numeric identity.
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl fmt::Display for ResourceProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::GATEWAY_DEFAULT_V1 => formatter.write_str("GatewayDefaultV1"),
            Self::DIRECTORY_CLIENT_DEFAULT_V1 => formatter.write_str("DirectoryClientDefaultV1"),
            Self::BENCHMARK_STATIC_REFERENCE_V1 => {
                formatter.write_str("BenchmarkStaticReferenceV1")
            }
            Self::APPLICATION_DEFINED => formatter.write_str("ApplicationDefined"),
            Self(value) => write!(formatter, "ApplicationDefined({value})"),
        }
    }
}

/// A complete set of resource ceilings in the CSV schema's stable order.
///
/// `Some(0)` is an explicit disabled or rendezvous limit according to the
/// corresponding [`ResourceKind::zero_semantics`]. `None` means that the field
/// is not applicable to this profile; it never means unbounded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceLimits {
    values: [Option<u64>; RESOURCE_LIMIT_COUNT],
}

impl ResourceLimits {
    /// Creates a complete application-defined limit set.
    pub const fn new(values: [Option<u64>; RESOURCE_LIMIT_COUNT]) -> Self {
        Self { values }
    }

    /// Returns the configured ceiling, or `None` when not applicable.
    pub const fn get(&self, kind: ResourceKind) -> Option<u64> {
        self.values[kind.index()]
    }

    /// Replaces one ceiling explicitly.
    pub fn set(&mut self, kind: ResourceKind, limit: Option<u64>) {
        self.values[kind.index()] = limit;
    }

    /// Returns a copy with one ceiling replaced explicitly.
    #[must_use]
    pub const fn with_limit(mut self, kind: ResourceKind, limit: Option<u64>) -> Self {
        self.values[kind.index()] = limit;
        self
    }

    /// Returns all limits in the authoritative CSV order.
    pub const fn as_values(&self) -> &[Option<u64>; RESOURCE_LIMIT_COUNT] {
        &self.values
    }
}

/// Supplies an immutable, versioned static resource profile.
pub trait StaticResourceProfile {
    /// Stable identity of this profile.
    const ID: ResourceProfileId;
    /// Complete profile values.
    const LIMITS: ResourceLimits;

    /// Returns the complete profile value.
    fn limits() -> ResourceLimits {
        Self::LIMITS
    }
}

/// Versioned default limits for a host gateway.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GatewayDefaultV1;

impl StaticResourceProfile for GatewayDefaultV1 {
    const ID: ResourceProfileId = ResourceProfileId::GATEWAY_DEFAULT_V1;
    const LIMITS: ResourceLimits = ResourceLimits::new(GATEWAY_DEFAULT_VALUES);
}

/// Versioned default limits for the engine-side Directory client.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DirectoryClientDefaultV1;

impl StaticResourceProfile for DirectoryClientDefaultV1 {
    const ID: ResourceProfileId = ResourceProfileId::DIRECTORY_CLIENT_DEFAULT_V1;
    const LIMITS: ResourceLimits = ResourceLimits::new(DIRECTORY_CLIENT_DEFAULT_VALUES);
}

/// Versioned static limits for the constrained benchmark reference target.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BenchmarkStaticReferenceV1;

impl StaticResourceProfile for BenchmarkStaticReferenceV1 {
    const ID: ResourceProfileId = ResourceProfileId::BENCHMARK_STATIC_REFERENCE_V1;
    const LIMITS: ResourceLimits = ResourceLimits::new(BENCHMARK_STATIC_REFERENCE_VALUES);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AccountState {
    limit: u64,
    used: u64,
    peak: u64,
}

impl AccountState {
    const fn new(limit: u64) -> Self {
        Self {
            limit,
            used: 0,
            peak: 0,
        }
    }

    fn try_reserve(&mut self, amount: u64) -> bool {
        let Some(next) = self.used.checked_add(amount) else {
            return false;
        };
        if next > self.limit {
            return false;
        }
        self.used = next;
        self.peak = self.peak.max(next);
        true
    }

    fn release(&mut self, amount: u64) -> bool {
        let Some(next) = self.used.checked_sub(amount) else {
            return false;
        };
        self.used = next;
        true
    }
}

/// A bounded account for one owner and one resource field.
#[derive(Debug, Eq, PartialEq)]
pub struct ResourceAccount {
    owner_slot: SlotIndex,
    owner_generation: Generation,
    kind: ResourceKind,
    state: AccountState,
}

impl ResourceAccount {
    /// Creates an empty bounded account.
    pub const fn new(
        owner_slot: SlotIndex,
        owner_generation: Generation,
        kind: ResourceKind,
        limit: u64,
    ) -> Self {
        Self {
            owner_slot,
            owner_generation,
            kind,
            state: AccountState::new(limit),
        }
    }

    /// Attempts to reserve an amount without partial mutation on failure.
    pub fn try_reserve(&mut self, amount: u64) -> Option<ResourceReservation<'_>> {
        if !self.state.try_reserve(amount) {
            return None;
        }
        let (count, bytes) = count_and_bytes(self.kind, amount);
        let owner_slot = self.owner_slot;
        let owner_generation = self.owner_generation;
        let kind = self.kind;
        Some(ResourceReservation {
            target: ReservationTarget::Account(self),
            owner_slot,
            owner_generation,
            kind,
            amount,
            count,
            bytes,
            committed: false,
            released: false,
        })
    }

    /// Releases a previously committed amount.
    pub fn release_committed(&mut self, amount: u64) -> bool {
        self.state.release(amount)
    }

    /// Returns the resource field charged by this account.
    pub const fn kind(&self) -> ResourceKind {
        self.kind
    }

    /// Returns the account ceiling.
    pub const fn limit(&self) -> u64 {
        self.state.limit
    }

    /// Returns the currently charged amount.
    pub const fn used(&self) -> u64 {
        self.state.used
    }

    /// Returns the largest simultaneously charged amount.
    pub const fn peak(&self) -> u64 {
        self.state.peak
    }
}

const SOURCE: usize = 0;
const TEMPORARY: usize = 1;
const PERSISTENT_DOCUMENT: usize = 2;
const PERSISTENT_RUNTIME: usize = 3;
const DIAGNOSTIC: usize = 4;
const CLEANUP: usize = 5;
const ADMISSION_ACCOUNT_COUNT: usize = 6;

/// Admission-phase accounts with aggregate peak-live observations.
#[derive(Debug, Eq, PartialEq)]
pub struct AdmissionLedger {
    owner_slot: SlotIndex,
    owner_generation: Generation,
    accounts: [AccountState; ADMISSION_ACCOUNT_COUNT],
    live_bytes: u64,
    peak_live_bytes: u64,
    largest_contiguous_allocation: u64,
}

impl AdmissionLedger {
    /// Creates an empty ledger with explicit byte ceilings for every account.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        owner_slot: SlotIndex,
        owner_generation: Generation,
        source_limit: u64,
        temporary_limit: u64,
        persistent_document_limit: u64,
        persistent_runtime_limit: u64,
        diagnostic_limit: u64,
        cleanup_limit: u64,
    ) -> Self {
        Self {
            owner_slot,
            owner_generation,
            accounts: [
                AccountState::new(source_limit),
                AccountState::new(temporary_limit),
                AccountState::new(persistent_document_limit),
                AccountState::new(persistent_runtime_limit),
                AccountState::new(diagnostic_limit),
                AccountState::new(cleanup_limit),
            ],
            live_bytes: 0,
            peak_live_bytes: 0,
            largest_contiguous_allocation: 0,
        }
    }

    /// Reserves source bytes.
    pub fn try_reserve_source(
        &mut self,
        kind: ResourceKind,
        bytes: u64,
    ) -> Option<ResourceReservation<'_>> {
        self.try_reserve(SOURCE, kind, bytes)
    }

    /// Reserves phase-local temporary bytes.
    pub fn try_reserve_temporary(
        &mut self,
        kind: ResourceKind,
        bytes: u64,
    ) -> Option<ResourceReservation<'_>> {
        self.try_reserve(TEMPORARY, kind, bytes)
    }

    /// Reserves persistent document-retention bytes.
    pub fn try_reserve_persistent_document(
        &mut self,
        kind: ResourceKind,
        bytes: u64,
    ) -> Option<ResourceReservation<'_>> {
        self.try_reserve(PERSISTENT_DOCUMENT, kind, bytes)
    }

    /// Reserves persistent compiled-runtime bytes.
    pub fn try_reserve_persistent_runtime(
        &mut self,
        kind: ResourceKind,
        bytes: u64,
    ) -> Option<ResourceReservation<'_>> {
        self.try_reserve(PERSISTENT_RUNTIME, kind, bytes)
    }

    /// Reserves bounded diagnostic bytes.
    pub fn try_reserve_diagnostic(
        &mut self,
        kind: ResourceKind,
        bytes: u64,
    ) -> Option<ResourceReservation<'_>> {
        self.try_reserve(DIAGNOSTIC, kind, bytes)
    }

    /// Reserves bounded cleanup bytes.
    pub fn try_reserve_cleanup(
        &mut self,
        kind: ResourceKind,
        bytes: u64,
    ) -> Option<ResourceReservation<'_>> {
        self.try_reserve(CLEANUP, kind, bytes)
    }

    /// Releases committed source bytes.
    pub fn release_source(&mut self, bytes: u64) -> bool {
        self.release(SOURCE, bytes)
    }

    /// Releases committed temporary bytes.
    pub fn release_temporary(&mut self, bytes: u64) -> bool {
        self.release(TEMPORARY, bytes)
    }

    /// Releases committed persistent document bytes.
    pub fn release_persistent_document(&mut self, bytes: u64) -> bool {
        self.release(PERSISTENT_DOCUMENT, bytes)
    }

    /// Releases committed persistent runtime bytes.
    pub fn release_persistent_runtime(&mut self, bytes: u64) -> bool {
        self.release(PERSISTENT_RUNTIME, bytes)
    }

    /// Releases committed diagnostic bytes.
    pub fn release_diagnostic(&mut self, bytes: u64) -> bool {
        self.release(DIAGNOSTIC, bytes)
    }

    /// Releases committed cleanup bytes.
    pub fn release_cleanup(&mut self, bytes: u64) -> bool {
        self.release(CLEANUP, bytes)
    }

    /// Returns the current sum of all account bytes.
    pub const fn live_bytes(&self) -> u64 {
        self.live_bytes
    }

    /// Returns the largest simultaneously live byte count.
    pub const fn peak_live_bytes(&self) -> u64 {
        self.peak_live_bytes
    }

    /// Returns the largest single successful reservation.
    pub const fn largest_contiguous_allocation(&self) -> u64 {
        self.largest_contiguous_allocation
    }

    fn try_reserve(
        &mut self,
        account: usize,
        kind: ResourceKind,
        bytes: u64,
    ) -> Option<ResourceReservation<'_>> {
        let next_live = self.live_bytes.checked_add(bytes)?;
        if !self.accounts[account].try_reserve(bytes) {
            return None;
        }
        self.live_bytes = next_live;
        self.peak_live_bytes = self.peak_live_bytes.max(next_live);
        self.largest_contiguous_allocation = self.largest_contiguous_allocation.max(bytes);
        let owner_slot = self.owner_slot;
        let owner_generation = self.owner_generation;
        Some(ResourceReservation {
            target: ReservationTarget::Ledger {
                ledger: self,
                account,
            },
            owner_slot,
            owner_generation,
            kind,
            amount: bytes,
            count: 0,
            bytes,
            committed: false,
            released: false,
        })
    }

    fn release(&mut self, account: usize, bytes: u64) -> bool {
        if self.accounts[account].used < bytes || self.live_bytes < bytes {
            return false;
        }
        let released = self.accounts[account].release(bytes);
        debug_assert!(released);
        self.live_bytes -= bytes;
        true
    }
}

enum ReservationTarget<'a> {
    Account(&'a mut ResourceAccount),
    Ledger {
        ledger: &'a mut AdmissionLedger,
        account: usize,
    },
}

/// A move-only resource charge that rolls back unless explicitly committed.
pub struct ResourceReservation<'a> {
    target: ReservationTarget<'a>,
    owner_slot: SlotIndex,
    owner_generation: Generation,
    kind: ResourceKind,
    amount: u64,
    count: u64,
    bytes: u64,
    committed: bool,
    released: bool,
}

impl ResourceReservation<'_> {
    /// Returns the generation-bearing owner identity.
    pub const fn owner(&self) -> (SlotIndex, Generation) {
        (self.owner_slot, self.owner_generation)
    }

    /// Returns the charged resource field.
    pub const fn kind(&self) -> ResourceKind {
        self.kind
    }

    /// Returns the reserved item/count amount when applicable.
    pub const fn count(&self) -> u64 {
        self.count
    }

    /// Returns the reserved byte amount when applicable.
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }

    /// Commits the charge into the published owner.
    ///
    /// The owning account or ledger must later use its explicit
    /// `release_committed` or account-specific release method.
    pub fn commit(mut self) {
        self.committed = true;
    }

    /// Releases the charge explicitly. Releasing by drop afterward is a no-op.
    pub fn release(mut self) {
        self.release_inner();
    }

    fn release_inner(&mut self) {
        if self.committed || self.released {
            return;
        }
        let released = match &mut self.target {
            ReservationTarget::Account(account) => account.state.release(self.amount),
            ReservationTarget::Ledger { ledger, account } => ledger.release(*account, self.amount),
        };
        debug_assert!(released);
        self.released = true;
    }
}

impl fmt::Debug for ResourceReservation<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResourceReservation")
            .field("owner_slot", &self.owner_slot)
            .field("owner_generation", &self.owner_generation)
            .field("kind", &self.kind)
            .field("count", &self.count)
            .field("bytes", &self.bytes)
            .field("committed", &self.committed)
            .field("released", &self.released)
            .finish_non_exhaustive()
    }
}

impl Drop for ResourceReservation<'_> {
    fn drop(&mut self) {
        self.release_inner();
    }
}

fn count_and_bytes(kind: ResourceKind, amount: u64) -> (u64, u64) {
    if kind.unit() == "bytes" {
        (0, amount)
    } else {
        (amount, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AdmissionLedger, BenchmarkStaticReferenceV1, DirectoryClientDefaultV1, GatewayDefaultV1,
        RESOURCE_LIMIT_COUNT, ResourceAccount, ResourceKind, ResourceProfileId,
        StaticResourceProfile,
    };
    use crate::{Generation, SlotIndex};

    #[test]
    fn generated_profiles_cover_every_schema_field() {
        assert_eq!(ResourceKind::ALL.len(), RESOURCE_LIMIT_COUNT);
        assert_eq!(GatewayDefaultV1::ID, ResourceProfileId::GATEWAY_DEFAULT_V1);
        assert_eq!(
            DirectoryClientDefaultV1::ID,
            ResourceProfileId::DIRECTORY_CLIENT_DEFAULT_V1
        );
        assert_eq!(
            BenchmarkStaticReferenceV1::ID,
            ResourceProfileId::BENCHMARK_STATIC_REFERENCE_V1
        );
        assert_eq!(
            GatewayDefaultV1::LIMITS.document_bytes_max(),
            Some(1_048_576)
        );
        assert_eq!(
            DirectoryClientDefaultV1::LIMITS.generated_effective_document_bytes_max(),
            None
        );
        assert_eq!(
            BenchmarkStaticReferenceV1::LIMITS.cleanup_work_items_per_step_max(),
            Some(16)
        );
        assert_eq!(
            GatewayDefaultV1::LIMITS.cleanup_retry_attempts_max(),
            Some(16)
        );
        assert_eq!(
            DirectoryClientDefaultV1::LIMITS.cleanup_retry_attempts_max(),
            Some(16)
        );
        assert_eq!(
            BenchmarkStaticReferenceV1::LIMITS.cleanup_retry_attempts_max(),
            Some(4)
        );
    }

    #[test]
    fn every_profile_limit_accepts_exactly_its_boundary() {
        for limits in [
            GatewayDefaultV1::LIMITS,
            DirectoryClientDefaultV1::LIMITS,
            BenchmarkStaticReferenceV1::LIMITS,
        ] {
            for kind in ResourceKind::ALL {
                let Some(limit) = limits.get(kind) else {
                    continue;
                };
                let mut account = ResourceAccount::new(
                    SlotIndex::new(kind.index() as u32),
                    Generation::INITIAL,
                    kind,
                    limit,
                );
                account
                    .try_reserve(limit)
                    .expect("the exact configured limit must fit")
                    .commit();
                assert!(account.release_committed(limit));
                if let Some(over_limit) = limit.checked_add(1) {
                    assert!(account.try_reserve(over_limit).is_none());
                }
                if limit > 0 {
                    assert!(!account.release_committed(1));
                }
            }
        }
    }

    #[test]
    fn account_reservations_roll_back_unless_committed() {
        let mut account = ResourceAccount::new(
            SlotIndex::new(3),
            Generation::INITIAL,
            ResourceKind::PayloadBytesMax,
            8,
        );
        {
            let reservation = account.try_reserve(6).expect("six bytes fit");
            assert_eq!(reservation.bytes(), 6);
        }
        assert_eq!(account.used(), 0);
        account
            .try_reserve(8)
            .expect("the full limit fits")
            .commit();
        assert_eq!(account.used(), 8);
        assert!(account.try_reserve(1).is_none());
        assert!(account.release_committed(8));
        assert_eq!(account.used(), 0);
    }

    #[test]
    fn admission_ledger_tracks_aggregate_peak_and_rollback() {
        let mut ledger = AdmissionLedger::new(
            SlotIndex::new(1),
            Generation::INITIAL,
            100,
            100,
            100,
            100,
            100,
            100,
        );
        ledger
            .try_reserve_source(ResourceKind::DocumentBytesMax, 40)
            .expect("source reservation fits")
            .commit();
        {
            let temporary = ledger
                .try_reserve_temporary(ResourceKind::AdmissionTemporaryBytesPerOperationMax, 60)
                .expect("temporary reservation fits");
            assert_eq!(temporary.bytes(), 60);
        }
        assert_eq!(ledger.live_bytes(), 40);
        assert_eq!(ledger.peak_live_bytes(), 100);
        assert_eq!(ledger.largest_contiguous_allocation(), 60);
        assert!(ledger.release_source(40));
        assert_eq!(ledger.live_bytes(), 0);
    }
}
