//! Shared Host capacity authority. Protocol-private allocation stays in Bindings.
use alloc::sync::Arc;
use clinkz_wot_foundation::{
    Generation, ResourceAccount, ResourceKind as K, ResourceLimits, SlotIndex,
};
use core::sync::atomic::{AtomicUsize, Ordering};

const N: usize = K::ALL.len();

/// Application-provisioned immutable limits and opaque live charges. The same
/// bank can fund Consumer roots and Producer installations. Snapshots are
/// detached observations, never locks, reservations or mutable account loans.
pub struct HostResourceAccounts {
    limits: [u64; N],
    used: [AtomicUsize; N],
    peak: [AtomicUsize; N],
    coordinates: [(SlotIndex, Generation); N],
}
impl HostResourceAccounts {
    pub fn new(limits: &ResourceLimits) -> Self {
        Self {
            limits: core::array::from_fn(|i| limits.get(K::ALL[i]).unwrap_or(0)),
            used: core::array::from_fn(|_| AtomicUsize::new(0)),
            peak: core::array::from_fn(|_| AtomicUsize::new(0)),
            coordinates: core::array::from_fn(|i| (SlotIndex::new(i as u32), Generation::INITIAL)),
        }
    }
    /// Provision selected empty account limits. Unspecified fields have zero
    /// capacity. Existing live accounts cannot be copied into another bank.
    pub fn try_from_accounts<const M: usize>(
        accounts: [ResourceAccount; M],
    ) -> Result<Self, [ResourceAccount; M]> {
        let mut seen = [false; N];
        for account in &accounts {
            let i = account.kind().index();
            if account.used() != 0 || seen[i] {
                return Err(accounts);
            }
            seen[i] = true;
        }
        let mut bank = Self {
            limits: [0; N],
            used: core::array::from_fn(|_| AtomicUsize::new(0)),
            peak: core::array::from_fn(|_| AtomicUsize::new(0)),
            coordinates: core::array::from_fn(|i| (SlotIndex::new(i as u32), Generation::INITIAL)),
        };
        for mut account in accounts {
            let i = account.kind().index();
            bank.limits[i] = account.limit();
            bank.coordinates[i] = account.try_reserve(0).unwrap().owner();
        }
        Ok(bank)
    }
    pub fn account(&self, kind: K) -> ResourceAccount {
        let i = kind.index();
        let used = self.used[i].load(Ordering::Acquire) as u64;
        let peak = (self.peak[i].load(Ordering::Relaxed) as u64).max(used);
        let mut account = ResourceAccount::new(
            self.coordinates[i].0,
            self.coordinates[i].1,
            kind,
            self.limits[i],
        );
        account.try_reserve(peak).expect("bounded peak").commit();
        assert!(account.release_committed(peak - used));
        account
    }
    pub fn snapshot<const M: usize>(&self, kinds: [K; M]) -> [ResourceAccount; M] {
        kinds.map(|k| self.account(k))
    }
    /// Reserve capacity for another application-owned Engine participant. The
    /// move-only lease holds the original bank until physical owner disposal.
    pub fn reserve(self: &Arc<Self>, kind: K, amount: u64) -> Option<HostCapacityReservation> {
        self.try_reserve(kind, amount)
            .then(|| HostCapacityReservation {
                bank: self.clone(),
                kind,
                amount,
            })
    }
    pub(crate) fn try_reserve(&self, kind: K, amount: u64) -> bool {
        self.try_reserve_up_to(kind, amount, u64::MAX)
    }
    fn try_reserve_up_to(&self, kind: K, amount: u64, ceiling: u64) -> bool {
        let i = kind.index();
        let limit = self.limits[i].min(ceiling);
        let Ok(amount) = usize::try_from(amount) else {
            return false;
        };
        match self.used[i].fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
            used.checked_add(amount)
                .filter(|next| *next as u64 <= limit)
        }) {
            Ok(previous) => {
                self.peak[i].fetch_max(previous + amount, Ordering::Relaxed);
                true
            }
            Err(_) => false,
        }
    }
    pub(crate) fn release(&self, kind: K, amount: u64) {
        if amount != 0 {
            let amount = usize::try_from(amount).expect("original native-sized reservation");
            let previous = self.used[kind.index()].fetch_sub(amount, Ordering::Release);
            assert!(previous >= amount, "original Host reservation");
        }
    }
}

pub struct HostCapacityReservation {
    bank: Arc<HostResourceAccounts>,
    kind: K,
    amount: u64,
}
impl Drop for HostCapacityReservation {
    fn drop(&mut self) {
        self.bank.release(self.kind, self.amount);
    }
}

impl HostCapacityReservation {
    pub(crate) const fn kind(&self) -> K {
        self.kind
    }
    pub fn amount(&self) -> u64 {
        self.amount
    }
    pub fn try_grow(&mut self, amount: u64) -> bool {
        self.try_grow_up_to(amount, u64::MAX)
    }
    /// Grow under both the bank's immutable limit and the caller's ceiling
    /// on total shared use. The same atomic acquisition enforces both.
    pub fn try_grow_up_to(&mut self, amount: u64, ceiling: u64) -> bool {
        if self.amount.checked_add(amount).is_none()
            || !self.bank.try_reserve_up_to(self.kind, amount, ceiling)
        {
            return false;
        }
        self.amount += amount;
        true
    }
    /// Transfer already reserved units to an independent owner on the same bank.
    /// Total use and peak stay constant throughout the handoff.
    pub fn split(&mut self, amount: u64) -> Option<Self> {
        self.amount = self.amount.checked_sub(amount)?;
        Some(Self {
            bank: self.bank.clone(),
            kind: self.kind,
            amount,
        })
    }

    /// Release only capacity held by this original move-only lease.
    pub fn release_part(&mut self, amount: u64) {
        self.amount = self
            .amount
            .checked_sub(amount)
            .expect("original reservation");
        self.bank.release(self.kind, amount);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clinkz_wot_foundation::{GatewayDefaultV1, StaticResourceProfile};

    #[test]
    fn concurrent_lease_growth_observes_a_tighter_shared_ceiling() {
        let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        let kind = K::AdmissionPeakLiveBytesGlobalMax;
        let other = bank.reserve(kind, 1024).unwrap();
        let barrier = std::sync::Barrier::new(2);
        let contender = || {
            let mut lease = bank.reserve(kind, 0).unwrap();
            let acquired = lease.try_grow_up_to(6144, 8192);
            barrier.wait();
            acquired
        };
        std::thread::scope(|scope| {
            let peer = scope.spawn(contender);
            assert_ne!(contender(), peer.join().unwrap());
        });
        assert_eq!(bank.account(kind).peak(), 7168);
        assert_eq!(bank.account(kind).used(), 1024);
        drop(other);
        assert_eq!(bank.account(kind).used(), 0);
    }
}
