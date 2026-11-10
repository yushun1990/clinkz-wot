//! Qualified Host change indication, independent of semantic permission and I/O.
//!
//! Sources retain one funded record per actual driver, not per call. A private
//! atomic gate serializes bounded metadata. Invalidation never waits for it:
//! an existing guard owns closing/reaping work until it unlocks and posts the
//! pinned batch. No allocator, Rust Waker or linked service runs under the gate.
//! Delivery can therefore await an already executing guard getting CPU.
//!
//! The native backend currently qualifies Linux eventfd. Other std targets keep
//! the manual Consumer representation; driver construction reports Unsupported.
use crate::host_accounts::{HostCapacityReservation, HostResourceAccounts};
use crate::{CoreError, CoreResult, ErrorContext, ErrorPhase, RetryClass};
use alloc::{sync::Arc, vec::Vec};
use clinkz_wot_foundation::{ResourceKind, WorkBudget, WorkClass};
use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering},
};
use std::io;
#[cfg(target_os = "linux")]
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd};

/// Maximum source-wide live/retiring recipient records; unrelated to call slots.
pub const MAX_CHANGE_DRIVERS: usize = 64;
const BUSY: usize = 1;
const DIRTY: usize = 2;
const CLOSED: usize = 4;

fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Admission, RetryClass::Safe)
}
fn busy() -> CoreError {
    CoreError::Backpressure(
        context().with_redacted_cause(740, "change source capacity or metadata busy"),
    )
}
fn stale() -> CoreError {
    CoreError::StaleHandle(context())
}
fn unavailable() -> CoreError {
    CoreError::Lifecycle(context().with_redacted_cause(741, "change driver unavailable"))
}
fn funding(bank: &Arc<HostResourceAccounts>, bytes: usize) -> CoreResult<HostCapacityReservation> {
    bank.reserve(ResourceKind::EngineLiveBytesGlobalMax, bytes as u64)
        .ok_or_else(busy)
}

struct DriverState {
    #[cfg(target_os = "linux")]
    fd: OwnedFd,
    pending: AtomicBool,
    live: AtomicBool,
    failure: AtomicI32,
    _funding: HostCapacityReservation,
}

/// One application-owned recipient. It must service its enrolled work owners or
/// explicitly hand off/close them before withdrawing. It is not a scheduler.
/// Provisioning funds the carrier and acquires one descriptor; OS descriptor and
/// kernel-memory limits remain deployment limits, not extra Engine byte backing.
pub struct HostChangeDriver {
    state: Arc<DriverState>,
}

/// A stable, callback-free native signal lease. Retention grants no authority.
/// The issuer's Arc identity, rather than an fd/thread number, identifies a driver.
#[derive(Clone)]
pub struct HostChangeSignal {
    state: Arc<DriverState>,
}

impl HostChangeDriver {
    /// Caller-funded startup, before admitting work. No executor is constructed.
    pub fn try_new(bank: &Arc<HostResourceAccounts>) -> io::Result<Self> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = bank;
            Err(io::Error::from(io::ErrorKind::Unsupported))
        }
        #[cfg(target_os = "linux")]
        {
            let lease = funding(
                bank,
                core::mem::size_of::<DriverState>() + 2 * core::mem::size_of::<usize>(),
            )
            .map_err(|_| io::Error::from(io::ErrorKind::OutOfMemory))?;
            // SAFETY: no pointers; failure returns no descriptor. The sole reader
            // owns registration, and all writers retain this exact owned fd.
            let fd = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK | libc::EFD_CLOEXEC) };
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: successful eventfd transferred one newly owned descriptor.
            let fd = unsafe { OwnedFd::from_raw_fd(fd) };
            Ok(Self {
                state: Arc::new(DriverState {
                    fd,
                    pending: AtomicBool::new(false),
                    live: AtomicBool::new(true),
                    failure: AtomicI32::new(0),
                    _funding: lease,
                }),
            })
        }
    }
    pub fn signal(&self) -> HostChangeSignal {
        HostChangeSignal {
            state: self.state.clone(),
        }
    }

    /// Serialize the recipient's acknowledge/survey/parking handshake at the
    /// application driver. Acknowledge before surveying all its enrolled work,
    /// then recheck/register before
    /// parking. False/spurious readiness is allowed. Receipt supplies no credit:
    /// retain a runnable obligation after a zero-credit or partial turn.
    pub fn acknowledge(&self) -> io::Result<bool> {
        self.signal().check()?;
        #[cfg(target_os = "linux")]
        {
            let mut count = 0u64;
            // SAFETY: exact writable u64, live owned fd, one nonblocking read.
            let n = unsafe {
                libc::read(
                    self.state.fd.as_raw_fd(),
                    (&mut count as *mut u64).cast(),
                    8,
                )
            };
            if n < 0 {
                let error = io::Error::last_os_error();
                if error.kind() != io::ErrorKind::WouldBlock {
                    self.state
                        .failure
                        .store(error.raw_os_error().unwrap_or(libc::EIO), Ordering::Release);
                    return Err(error);
                }
            } else if n != 8 {
                self.state.failure.store(libc::EIO, Ordering::Release);
                return Err(io::Error::from_raw_os_error(libc::EIO));
            }
            // RMW acquires every prior publisher's release sequence, including
            // a publisher whose kernel write has not yet occurred. A later write
            // can produce a harmless additional readiness event.
            Ok(self.state.pending.swap(false, Ordering::AcqRel) || n == 8)
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(io::Error::from(io::ErrorKind::Unsupported))
        }
    }
}
impl Drop for HostChangeDriver {
    fn drop(&mut self) {
        self.state.live.store(false, Ordering::Release);
    }
}
#[cfg(target_os = "linux")]
impl AsFd for HostChangeDriver {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.state.fd.as_fd()
    }
}
#[cfg(target_os = "linux")]
impl AsRawFd for HostChangeDriver {
    fn as_raw_fd(&self) -> i32 {
        self.state.fd.as_raw_fd()
    }
}
impl HostChangeSignal {
    fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state)
    }
    pub fn same_recipient(&self, other: &Self) -> bool {
        self.same(other)
    }
    pub fn is_available(&self) -> bool {
        self.state.live.load(Ordering::Acquire) && self.state.failure.load(Ordering::Acquire) == 0
    }
    pub fn check(&self) -> io::Result<()> {
        let failure = self.state.failure.load(Ordering::Acquire);
        if failure != 0 {
            return Err(io::Error::from_raw_os_error(failure));
        }
        if !self.state.live.load(Ordering::Acquire) {
            return Err(io::Error::from(io::ErrorKind::BrokenPipe));
        }
        Ok(())
    }
    /// Request a turn, including an application credit/deadline turn. Linux
    /// counter saturation is already readable/coalesced; other failures are not
    /// delivery and latch the endpoint unhealthy. No retries or Rust callbacks.
    pub fn request_turn(&self) -> io::Result<()> {
        self.check()?;
        self.state.pending.swap(true, Ordering::Release);
        #[cfg(target_os = "linux")]
        {
            let count = 1u64;
            // SAFETY: exact readable u64, retained owned fd, nonblocking write.
            let n =
                unsafe { libc::write(self.state.fd.as_raw_fd(), (&count as *const u64).cast(), 8) };
            if n == 8 {
                return Ok(());
            }
            let error = if n < 0 {
                io::Error::last_os_error()
            } else {
                io::Error::from_raw_os_error(libc::EIO)
            };
            if error.kind() == io::ErrorKind::WouldBlock {
                return Ok(());
            }
            self.state
                .failure
                .store(error.raw_os_error().unwrap_or(libc::EIO), Ordering::Release);
            Err(error)
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(io::Error::from(io::ErrorKind::Unsupported))
        }
    }
}

struct Quota {
    used: AtomicUsize,
    limit: usize,
    _funding: HostCapacityReservation,
}
struct QuotaLease(Arc<Quota>);
impl QuotaLease {
    fn acquire(quota: &Arc<Quota>) -> CoreResult<Self> {
        // A contention refusal is safe before effects; never spin indefinitely.
        let mut used = quota.used.load(Ordering::Acquire);
        for _ in 0..4 {
            if used >= quota.limit {
                return Err(busy());
            }
            match quota
                .used
                .compare_exchange(used, used + 1, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => return Ok(Self(quota.clone())),
                Err(next) => used = next,
            }
        }
        Err(busy())
    }
}
impl Drop for QuotaLease {
    fn drop(&mut self) {
        self.0.used.fetch_sub(1, Ordering::AcqRel);
    }
}
struct Relationship {
    signal: HostChangeSignal,
    dependants: AtomicUsize,
    _funding: HostCapacityReservation,
    // Counts candidates, dispatch pins and retiring records through final use.
    _quota: QuotaLease,
}
#[derive(Default)]
struct Storage {
    entries: Vec<Arc<Relationship>>,
    _funding: Option<HostCapacityReservation>,
}
struct Source {
    gate: AtomicUsize,
    quota: Arc<Quota>,
    failures: AtomicUsize,
    data: UnsafeCell<Storage>,
    _funding: HostCapacityReservation,
}
// SAFETY: data is accessible only by the unique BUSY owner, acquired/released
// through the atomic gate. No guard escapes a source operation. All other state
// is atomic/immutable; external signal and destructor code runs after unlocking.
unsafe impl Sync for Source {}

/// Move-only source owner. Normal Drop and explicit invalidate share one fence.
pub struct HostChangeAuthority {
    source: HostChangeSource,
}
/// Passive source identity/subscription capability; it cannot invalidate.
#[derive(Clone)]
pub struct HostChangeSource {
    source: Arc<Source>,
}
/// One dependency. Repeated calls coalesce records but own independent leases.
/// This is not Clone: acquisition, bounded work and capacity precede each lease.
pub struct HostChangeLease {
    source: HostChangeSource,
    relationship: Arc<Relationship>,
}

impl HostChangeAuthority {
    /// Caller-funded startup. Zero disables subscriptions; at most 64 distinct
    /// live/retiring drivers per source. No array or port is prepaid per source.
    pub fn try_new(bank: &Arc<HostResourceAccounts>, max_drivers: usize) -> CoreResult<Self> {
        if max_drivers > MAX_CHANGE_DRIVERS {
            return Err(CoreError::Validation(context()));
        }
        let source_funding = funding(
            bank,
            core::mem::size_of::<Source>() + 2 * core::mem::size_of::<usize>(),
        )?;
        let quota_funding = funding(
            bank,
            core::mem::size_of::<Quota>() + 2 * core::mem::size_of::<usize>(),
        )?;
        Ok(Self {
            source: HostChangeSource {
                source: Arc::new(Source {
                    gate: AtomicUsize::new(0),
                    quota: Arc::new(Quota {
                        used: AtomicUsize::new(0),
                        limit: max_drivers,
                        _funding: quota_funding,
                    }),
                    failures: AtomicUsize::new(0),
                    data: UnsafeCell::new(Storage::default()),
                    _funding: source_funding,
                }),
            },
        })
    }
    pub fn reference(&self) -> HostChangeSource {
        self.source.clone()
    }
    pub fn invalidate(&self) {
        let source = &self.source.source;
        if source.gate.fetch_or(CLOSED, Ordering::AcqRel) & CLOSED == 0 {
            source.request_maintenance();
        }
    }
}
impl Drop for HostChangeAuthority {
    fn drop(&mut self) {
        self.invalidate();
    }
}

impl HostChangeSource {
    pub fn is_current(&self) -> bool {
        self.source.gate.load(Ordering::Acquire) & CLOSED == 0
    }
    pub fn driver_limit(&self) -> usize {
        self.source.quota.limit
    }
    /// Includes candidates, final-release races and in-flight notification pins.
    pub fn retained_relationships(&self) -> usize {
        self.source.quota.used.load(Ordering::Acquire)
    }
    /// Failed native posts, not an assertion that the affected driver woke.
    pub fn failed_posts(&self) -> usize {
        self.source.failures.load(Ordering::Acquire)
    }

    pub fn subscribe(
        &self,
        signal: &HostChangeSignal,
        bank: &Arc<HostResourceAccounts>,
        budget: &mut WorkBudget,
    ) -> CoreResult<HostChangeLease> {
        let limit = self.source.quota.limit;
        // Prepay lookup, up to two bounded guard exits, final unlink and source
        // fan-out. Future destruction never acquires ordinary work credit.
        let envelope = 4 * (limit as u64 + 3).pow(2) + 4 * MAX_CHANGE_DRIVERS as u64;
        budget
            .consume(WorkClass::QueueOperations, envelope)
            .map_err(|_| busy())?;
        if !signal.is_available() {
            return Err(unavailable());
        }
        let wanted;
        {
            let mut guard = self.source.enter().ok_or_else(busy)?;
            if !self.is_current() {
                return Err(stale());
            }
            if let Some(found) = guard.data().entries.iter().find(|r| r.signal.same(signal)) {
                let relationship = found.clone();
                let source = self.clone();
                // Arc ownership bounds this count before usize overflow; an
                // in-gate revival from zero is safe while the source owns it.
                relationship.dependants.fetch_add(1, Ordering::AcqRel);
                drop(guard);
                let lease = HostChangeLease {
                    source,
                    relationship,
                };
                if !self.is_current() {
                    return Err(stale());
                }
                return Ok(lease);
            }
            let storage = guard.data();
            wanted = if storage.entries.len() == storage.entries.capacity() {
                Some((storage.entries.capacity().max(2) * 2).min(limit))
            } else {
                None
            };
        }
        let quota = QuotaLease::acquire(&self.source.quota)?;
        let record_funding = funding(
            bank,
            core::mem::size_of::<Relationship>() + 2 * core::mem::size_of::<usize>(),
        )?;
        let candidate = Arc::new(Relationship {
            signal: signal.clone(),
            dependants: AtomicUsize::new(0),
            _funding: record_funding,
            _quota: quota,
        });
        let mut replacement = if let Some(capacity) = wanted {
            let lease = funding(bank, capacity * core::mem::size_of::<Arc<Relationship>>())?;
            let mut entries = Vec::new();
            entries.try_reserve_exact(capacity).map_err(|_| busy())?;
            Some(Storage {
                entries,
                _funding: Some(lease),
            })
        } else {
            None
        };
        let mut old = None;
        let relationship;
        {
            let mut guard = self.source.enter().ok_or_else(busy)?;
            if !self.is_current() {
                return Err(stale());
            }
            if !signal.is_available() {
                return Err(unavailable());
            }
            let storage = guard.data();
            if let Some(found) = storage.entries.iter().find(|r| r.signal.same(signal)) {
                relationship = found.clone();
            } else {
                if storage.entries.len() == storage.entries.capacity() {
                    let next = replacement.as_mut().ok_or_else(busy)?;
                    if next.entries.capacity() < storage.entries.len() + 1 {
                        return Err(busy());
                    }
                    // Capacity was acquired outside the gate. Append only moves
                    // references; old allocation/funding retires after unlocking.
                    next.entries.append(&mut storage.entries);
                    old = Some(core::mem::replace(storage, replacement.take().unwrap()));
                }
                storage.entries.push(candidate.clone());
                relationship = candidate.clone();
            }
            relationship.dependants.fetch_add(1, Ordering::AcqRel);
        }
        drop(old);
        drop(replacement);
        let lease = HostChangeLease {
            source: self.clone(),
            relationship,
        };
        if !self.is_current() {
            return Err(stale());
        }
        Ok(lease)
    }
}
impl Drop for HostChangeLease {
    fn drop(&mut self) {
        if self.relationship.dependants.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.source.source.request_maintenance();
        }
    }
}

struct Gate<'a> {
    source: &'a Source,
}
impl Source {
    fn enter(&self) -> Option<Gate<'_>> {
        if self.gate.fetch_or(BUSY, Ordering::AcqRel) & BUSY == 0 {
            Some(Gate { source: self })
        } else {
            None
        }
    }
    fn request_maintenance(&self) {
        self.gate.fetch_or(DIRTY, Ordering::AcqRel);
        // Failure to enter assigns the request to the existing guard. Its CAS
        // cannot unlock across a newly published DIRTY/CLOSED transition.
        drop(self.enter());
    }
}
impl Gate<'_> {
    fn data(&mut self) -> &mut Storage {
        // SAFETY: this private guard is the sole BUSY owner.
        unsafe { &mut *self.source.data.get() }
    }
}
impl Drop for Gate<'_> {
    fn drop(&mut self) {
        let mut retired: [Option<Arc<Relationship>>; MAX_CHANGE_DRIVERS] =
            core::array::from_fn(|_| None);
        let mut count = 0;
        let mut batch = None;
        loop {
            let state = self.source.gate.fetch_and(!DIRTY, Ordering::AcqRel);
            let storage = self.data();
            if state & CLOSED != 0 {
                if batch.is_none() {
                    batch = Some(core::mem::take(storage));
                }
            } else {
                let mut index = 0;
                while index < storage.entries.len() {
                    if storage.entries[index].dependants.load(Ordering::Acquire) == 0 {
                        retired[count] = Some(storage.entries.swap_remove(index));
                        count += 1;
                    } else {
                        index += 1;
                    }
                }
                if storage.entries.is_empty() && batch.is_none() {
                    batch = Some(core::mem::take(storage));
                }
            }
            let observed = self.source.gate.load(Ordering::Acquire);
            if observed & DIRTY != 0 || (observed ^ state) & CLOSED != 0 {
                continue;
            }
            if self
                .source
                .gate
                .compare_exchange(
                    observed,
                    observed & !BUSY,
                    Ordering::Release,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                break;
            }
            // No new record/count increment can occur while BUSY. Only at most
            // limit last-release requests and one close can extend this exit.
        }
        if let Some(batch) = batch {
            if self.source.gate.load(Ordering::Acquire) & CLOSED != 0 {
                for relationship in &batch.entries {
                    if relationship.dependants.load(Ordering::Acquire) != 0
                        && relationship.signal.request_turn().is_err()
                    {
                        self.source.failures.fetch_add(1, Ordering::AcqRel);
                    }
                }
            }
            drop(batch);
        }
        drop(retired);
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use clinkz_wot_foundation::{GatewayDefaultV1, StaticResourceProfile};
    fn bank() -> Arc<HostResourceAccounts> {
        Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS))
    }
    fn work() -> WorkBudget {
        WorkBudget::new().with_remaining(WorkClass::QueueOperations, 1_000_000)
    }
    fn clean(bank: &HostResourceAccounts) {
        for kind in ResourceKind::ALL {
            assert_eq!(bank.account(kind).used(), 0, "{kind:?}");
        }
    }

    #[test]
    fn repeated_calls_coalesce_and_retiring_pins_keep_the_original_account() {
        let (backing, original, later) = (bank(), bank(), bank());
        let owner = HostChangeAuthority::try_new(&backing, 1).unwrap();
        let source = owner.reference();
        let driver = HostChangeDriver::try_new(&backing).unwrap();
        let signal = driver.signal();
        let first = source.subscribe(&signal, &original, &mut work()).unwrap();
        let leases: Vec<_> = (0..20)
            .map(|_| source.subscribe(&signal, &later, &mut work()).unwrap())
            .collect();
        assert_eq!(source.retained_relationships(), 1);
        assert_eq!(
            later.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
            0
        );
        drop(first);
        assert_ne!(
            original
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .used(),
            0
        );
        owner.invalidate();
        assert!(!source.is_current());
        assert!(driver.acknowledge().unwrap());
        owner.invalidate();
        assert!(!driver.acknowledge().unwrap());
        assert_eq!(source.retained_relationships(), 1);
        drop(leases);
        assert_eq!(source.retained_relationships(), 0);
        clean(&original);
        drop((owner, source, signal, driver));
        clean(&backing);
        clean(&later);
    }

    #[test]
    fn a_later_independent_driver_enrolls_without_changing_the_source() {
        let backing = bank();
        let owner = HostChangeAuthority::try_new(&backing, 2).unwrap();
        let source = owner.reference();
        let (d1, d2, d3) = (
            HostChangeDriver::try_new(&backing).unwrap(),
            HostChangeDriver::try_new(&backing).unwrap(),
            HostChangeDriver::try_new(&backing).unwrap(),
        );
        let a = source
            .subscribe(&d1.signal(), &backing, &mut work())
            .unwrap();
        let b = source
            .subscribe(&d2.signal(), &backing, &mut work())
            .unwrap();
        assert!(matches!(
            source.subscribe(&d3.signal(), &backing, &mut work()),
            Err(CoreError::Backpressure(_))
        ));
        drop(a);
        let c = source
            .subscribe(&d3.signal(), &backing, &mut work())
            .unwrap();
        drop(owner); // normal RAII release, without any explicit close
        assert!(!d1.acknowledge().unwrap());
        assert!(d2.acknowledge().unwrap());
        assert!(d3.acknowledge().unwrap());
        drop((b, c, source, d1, d2, d3));
        clean(&backing);
    }

    #[test]
    fn closing_and_final_release_are_owned_by_an_inflight_guard() {
        let backing = bank();
        let owner = HostChangeAuthority::try_new(&backing, 2).unwrap();
        let source = owner.reference();
        let (d1, d2) = (
            HostChangeDriver::try_new(&backing).unwrap(),
            HostChangeDriver::try_new(&backing).unwrap(),
        );
        let a = source
            .subscribe(&d1.signal(), &backing, &mut work())
            .unwrap();
        let b = source
            .subscribe(&d2.signal(), &backing, &mut work())
            .unwrap();
        let entered = std::sync::Barrier::new(2);
        let resume = std::sync::Barrier::new(2);
        std::thread::scope(|scope| {
            scope.spawn(|| {
                let guard = source.source.enter().unwrap();
                // Test-only pause models OS preemption. Production guards never
                // call linked code, allocate, await or expose this guard.
                entered.wait();
                resume.wait();
                drop(guard);
            });
            entered.wait();
            drop(a); // last release cannot lose its unlink request while busy
            owner.invalidate();
            assert!(!source.is_current());
            assert!(!d2.acknowledge().unwrap(), "an owed batch is not delivered");
            assert_eq!(source.retained_relationships(), 2);
            resume.wait();
        });
        assert!(!d1.acknowledge().unwrap());
        assert!(d2.acknowledge().unwrap());
        assert_eq!(source.retained_relationships(), 1);
        drop((b, owner, source, d1, d2));
        clean(&backing);
    }

    #[test]
    fn concurrent_registration_and_last_release_cannot_hide_a_close() {
        for _ in 0..64 {
            let backing = bank();
            let owner = HostChangeAuthority::try_new(&backing, 2).unwrap();
            let source = owner.reference();
            let driver = HostChangeDriver::try_new(&backing).unwrap();
            let signal = driver.signal();
            let baseline = source.subscribe(&signal, &backing, &mut work()).unwrap();
            let start = std::sync::Barrier::new(3);
            std::thread::scope(|scope| {
                for _ in 0..2 {
                    let (source, signal, backing, start) = (&source, &signal, &backing, &start);
                    scope.spawn(move || {
                        start.wait();
                        for _ in 0..32 {
                            drop(source.subscribe(signal, backing, &mut work()));
                        }
                    });
                }
                start.wait();
                owner.invalidate();
            });
            assert!(driver.acknowledge().unwrap());
            assert!(!source.is_current());
            drop((baseline, source, owner, driver, signal));
            clean(&backing);
        }
    }

    #[test]
    fn first_registration_racing_close_either_refuses_or_pins_a_notice() {
        for _ in 0..128 {
            let backing = bank();
            let owner = HostChangeAuthority::try_new(&backing, 1).unwrap();
            let source = owner.reference();
            let driver = HostChangeDriver::try_new(&backing).unwrap();
            let start = std::sync::Barrier::new(2);
            let lease = std::thread::scope(|scope| {
                let enrolling = scope.spawn(|| {
                    start.wait();
                    source.subscribe(&driver.signal(), &backing, &mut work())
                });
                start.wait();
                owner.invalidate();
                enrolling.join().unwrap()
            });
            assert!(!source.is_current());
            if lease.is_ok() {
                assert!(driver.acknowledge().unwrap());
            }
            drop((lease, source, owner, driver));
            clean(&backing);
        }
    }

    #[test]
    fn backend_failure_is_not_delivered_notification_or_fresh_admission() {
        let backing = bank();
        let owner = HostChangeAuthority::try_new(&backing, 1).unwrap();
        let source = owner.reference();
        let driver = HostChangeDriver::try_new(&backing).unwrap();
        let signal = driver.signal();
        let lease = source.subscribe(&signal, &backing, &mut work()).unwrap();
        // The syscall's unexpected-failure branch is latched, not treated as
        // EAGAIN/coalescing. No forced native event is manufactured afterward.
        driver.state.failure.store(libc::EIO, Ordering::Release);
        owner.invalidate();
        assert_eq!(source.failed_posts(), 1);
        assert!(driver.acknowledge().is_err());
        assert!(matches!(
            source.subscribe(&signal, &backing, &mut work()),
            Err(CoreError::Lifecycle(_))
        ));
        assert_eq!(source.retained_relationships(), 1);
        drop((lease, source, owner, signal, driver));
        clean(&backing);
    }

    #[test]
    fn a_late_notice_cannot_target_a_replacement_recipient() {
        let backing = bank();
        let owner = HostChangeAuthority::try_new(&backing, 1).unwrap();
        let source = owner.reference();
        let old = HostChangeDriver::try_new(&backing).unwrap();
        let signal = old.signal();
        let lease = source.subscribe(&signal, &backing, &mut work()).unwrap();
        drop(old);
        let replacement = HostChangeDriver::try_new(&backing).unwrap();
        owner.invalidate();
        assert_eq!(source.failed_posts(), 1);
        assert!(!replacement.acknowledge().unwrap());
        assert!(signal.check().is_err());
        drop((lease, signal, replacement, source, owner));
        clean(&backing);
    }

    #[test]
    fn native_counter_saturation_is_already_readable_and_acknowledged() {
        let backing = bank();
        let driver = HostChangeDriver::try_new(&backing).unwrap();
        let saturated = u64::MAX - 1;
        // SAFETY: exact readable u64 and live owned nonblocking descriptor.
        assert_eq!(
            unsafe { libc::write(driver.as_raw_fd(), (&saturated as *const u64).cast(), 8) },
            8
        );
        driver.signal().request_turn().unwrap(); // actual EAGAIN, no retry
        assert!(driver.acknowledge().unwrap());
        assert!(!driver.acknowledge().unwrap());
        drop(driver);
        clean(&backing);
    }

    #[test]
    fn concurrent_publication_and_acknowledgement_do_not_require_periodic_work_polls() {
        let backing = bank();
        let driver = HostChangeDriver::try_new(&backing).unwrap();
        let signal = driver.signal();
        let published = AtomicUsize::new(0);
        std::thread::scope(|scope| {
            scope.spawn(|| {
                for next in 1..=1000 {
                    published.store(next, Ordering::Release);
                    signal.request_turn().unwrap();
                }
            });
            let mut observed = 0;
            while observed < 1000 {
                let mut fd = libc::pollfd {
                    fd: driver.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                };
                // SAFETY: one writable pollfd. This blocking test receiver is
                // outside Engine source/Drop; timeout only detects test failure.
                assert_eq!(unsafe { libc::poll(&mut fd, 1, 5000) }, 1);
                assert!(driver.acknowledge().unwrap());
                // Survey is after acknowledgement. More publication must be
                // included in that survey or leave readiness for a later turn.
                observed = published.load(Ordering::Acquire);
            }
        });
        drop((signal, driver));
        clean(&backing);
    }
}
