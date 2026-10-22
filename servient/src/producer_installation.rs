//! Funded Host Producer installation, deliberately ending at InstalledClosed.
//! No serving permit, handler dispatch, or request/response bridge is supplied
//! by this checkpoint. The old staged Host expose path is not invoked here.
use alloc::{boxed::Box, sync::Arc};
use clinkz_wot_core::{producer::*, *};
use clinkz_wot_foundation::{
    Generation, MonotonicInstant, ResourceAccount, ResourceKind as K, ResourceLimits, SlotIndex,
    WorkBudget, WorkClass,
};
use clinkz_wot_planning::producer_execution::PreparedProducer;
use core::{
    alloc::Layout,
    mem::size_of,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll},
};

const KINDS: [K; 12] = [
    K::EngineLiveBytesGlobalMax,
    K::CompiledRuntimeBytesGlobalMax,
    K::BindingArtifactBytesGlobalMax,
    K::HandlerStateBytesGlobalMax,
    K::BindingSlotStateBytesGlobalMax,
    K::CleanupBytesMax,
    K::CleanupItemsMax,
    K::BindingRoutesGlobalMax,
    K::HandlerSlotsGlobalMax,
    K::BindingPollTemporaryBytesGlobalMax,
    K::BindingArtifactsGlobalMax,
    K::DurableStatusBytesGlobalMax,
];
/// Application-provisioned parent capacity, shared by all participating owners.
/// Limits are immutable and only funded runtime owners can change charges.
/// Inspection returns detached snapshots, never a lock or mutable bank loan.
///
/// ```compile_fail,E0599
/// use clinkz_wot_servient::producer_installation::ProducerInstallationAccounts;
/// fn hold_parent(bank: &ProducerInstallationAccounts) {
///     let _guard = bank.lock();
/// }
/// ```
pub struct ProducerInstallationAccounts {
    limits: [u64; 12],
    used: [AtomicUsize; 12],
    peak: [AtomicUsize; 12],
}
pub fn producer_installation_accounts(limits: &ResourceLimits) -> ProducerInstallationAccounts {
    ProducerInstallationAccounts {
        limits: core::array::from_fn(|i| limits.get(KINDS[i]).unwrap_or(0)),
        used: core::array::from_fn(|_| AtomicUsize::new(0)),
        peak: core::array::from_fn(|_| AtomicUsize::new(0)),
    }
}
impl ProducerInstallationAccounts {
    /// Allocation-free individual account observations, not a transaction or
    /// reservation authority. Holding or mutating the copies cannot delay Drop
    /// or change the original limits/charges. Concurrent admission/retirement
    /// may occur between samples of different resource fields.
    pub fn snapshot(&self) -> [ResourceAccount; 12] {
        core::array::from_fn(|i| {
            let used = self.used[i].load(Ordering::Acquire) as u64;
            let peak = (self.peak[i].load(Ordering::Relaxed) as u64).max(used);
            let mut account = ResourceAccount::new(
                SlotIndex::new(i as u32),
                Generation::INITIAL,
                KINDS[i],
                self.limits[i],
            );
            account.try_reserve(peak).expect("bounded peak").commit();
            assert!(account.release_committed(peak - used));
            account
        })
    }
    fn try_reserve(&self, i: usize, amount: u64) -> bool {
        // These are controlled resident bytes/items, so aggregate backing must
        // fit the native address space. Do not require 64-bit target atomics.
        let Ok(amount) = usize::try_from(amount) else {
            return false;
        };
        match self.used[i].fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
            used.checked_add(amount)
                .filter(|next| *next as u64 <= self.limits[i])
        }) {
            Ok(previous) => {
                self.peak[i].fetch_max(previous + amount, Ordering::Relaxed);
                true
            }
            Err(_) => false,
        }
    }
    fn release(&self, i: usize, amount: u64) {
        if amount != 0 {
            let amount = usize::try_from(amount).expect("original native-sized reservation");
            // No mutex, callback, retry loop or deferred queue on disposal.
            // Only the move-only charge holding these exact bytes can release
            // them, after their physical owner has been destroyed.
            let previous = self.used[i].fetch_sub(amount, Ordering::Release);
            assert!(previous >= amount, "original Producer reservation");
        }
    }
}

struct Charge {
    parent: Arc<ProducerInstallationAccounts>,
    held: [u64; 12],
}
impl Charge {
    fn retain_status(&mut self, bytes: u64, status: u64) {
        let keep = [bytes, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, status];
        for (i, remaining) in keep.into_iter().enumerate() {
            self.parent.release(i, self.held[i] - remaining);
            self.held[i] = remaining;
        }
    }
}
impl Drop for Charge {
    fn drop(&mut self) {
        for (i, amount) in self.held.iter().copied().enumerate() {
            self.parent.release(i, amount);
        }
    }
}
fn check(limits: &ResourceLimits, kind: K, amount: u64, context: ErrorContext) -> CoreResult<()> {
    let limit = limits.get(kind).unwrap_or(0);
    if amount > limit {
        return Err(CoreError::LimitExceeded {
            context,
            resource: kind,
            limit,
            requested: Some(amount),
            observed: None,
        });
    }
    Ok(())
}
fn add(a: u64, b: u64) -> CoreResult<u64> {
    a.checked_add(b).ok_or_else(invalid)
}
fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
}
fn invalid() -> CoreError {
    CoreError::Validation(context())
}

struct Owner<E: ProducerExecutor, H: ReadPropertyHandler> {
    execution: Option<ProducerExecutionRegistration<E, H>>,
    installation: Option<E::Installation>,
}
impl<E: ProducerExecutor, H: ReadPropertyHandler> Drop for Owner<E, H> {
    fn drop(&mut self) {
        // Native participation dies before its program, executor and capability.
        drop(self.installation.take());
    }
}

/// InstalledClosed is native setup completion, not publication/serviceability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProducerInstallationState {
    Reserved,
    Installing,
    InstalledClosed,
    Closing,
    Closed,
}

/// One move-only Engine installation/cleanup owner. All partial native state
/// remains in its prebacked owner through Pending, errors and cancellation.
/// It can be moved as a complete object into an existing cleanup envelope;
/// refusal of that transfer therefore leaves the original owner intact.
pub struct ProducerInstallationRuntime<E: ProducerExecutor, H: ReadPropertyHandler> {
    authority: ProducerInstanceAuthority,
    owner: Option<Box<Owner<E, H>>>,
    charge: Option<Charge>,
    state: ProducerInstallationState,
    first_cause: Option<CoreError>,
    deadline: Deadline,
    terminal_work: WorkBudget,
}
impl<E: ProducerExecutor, H: ReadPropertyHandler + Send + 'static>
    ProducerInstallationRuntime<E, H>
{
    /// Admission is pure. Failure returns the completed preparation and the
    /// exact supplied limits/bank. The bank's existing charges participate in
    /// admission; creating a second bank creates a different capacity authority.
    #[allow(clippy::type_complexity)]
    pub fn try_new(
        prepared: PreparedProducer<E, H>,
        authority: ProducerInstanceAuthority,
        limits: ResourceLimits,
        parent: Arc<ProducerInstallationAccounts>,
        deadline: Deadline,
    ) -> Result<
        Self,
        BindingInputRejection<(
            PreparedProducer<E, H>,
            ProducerInstanceAuthority,
            ResourceLimits,
            Arc<ProducerInstallationAccounts>,
        )>,
    > {
        let identity = prepared.entry().binding();
        let quota_context = context()
            .with_thing(authority.key().slot())
            .with_binding(identity.binding_id(), identity.binding_generation())
            .with_resource_failure(
                identity.configuration(),
                ResourceFailureDomain::EngineCapacity,
                None,
                None,
            );
        let admit = || -> CoreResult<[u64; 12]> {
            prepared.check_current()?;
            if prepared.entry().instance_key() != authority.key() {
                return Err(invalid());
            }
            let p = prepared.install_policy();
            let f = prepared.footprint();
            if f.retained_bytes() < size_of::<H>() as u64 || p.local_drop_work == 0 {
                return Err(invalid());
            }
            let property = prepared.entry().property().len() as u64;
            let extra = add(
                add(
                    add(p.executor_bytes, p.program_bytes)?,
                    p.installation_bytes,
                )?,
                add(property, f.retained_bytes() - size_of::<H>() as u64)?,
            )?;
            let owned = add(size_of::<Owner<E, H>>() as u64, extra)?;
            let total = add(owned, size_of::<Self>() as u64)?;
            let live_peak = add(total, p.temporary_bytes)?;
            let program = add(
                add(
                    size_of::<ProducerExecutionEntry<E::Program>>() as u64,
                    property,
                )?,
                p.program_bytes,
            )?;
            for (kind, amount) in [
                (
                    K::LargestContiguousAllocationBytesMax,
                    Layout::new::<Owner<E, H>>().size() as u64,
                ),
                (K::LargestContiguousAllocationBytesMax, property),
                (
                    K::LargestContiguousAllocationBytesMax,
                    p.largest_indirect_allocation,
                ),
                (K::CompiledRuntimeBytesPerThingMax, total),
                (K::BindingSlotStateBytesPerItemMax, owned),
                (K::BindingSlotStateBytesPerThingMax, owned),
                (K::BindingArtifactBytesPerItemMax, program),
                (K::BindingArtifactBytesPerThingMax, program),
                (K::HandlerStateBytesPerThingMax, f.retained_bytes()),
                (K::HandlerSlotsPerThingMax, 1),
                (K::BindingArtifactsPerThingMax, 1),
                (K::BindingRoutesPerThingMax, 1),
                (K::BindingPollTemporaryBytesPerCallMax, p.temporary_bytes),
                (K::CleanupItemBytesMax, total),
                (K::CleanupWorkItemsPerStepMax, p.local_drop_work),
                (K::DurableStatusEntriesPerBindingMax, 1),
                (
                    K::DurableStatusBytesPerBindingMax,
                    size_of::<Option<CoreError>>() as u64,
                ),
            ] {
                check(&limits, kind, amount, quota_context)?;
            }
            Ok([
                // Temporary backing overlaps the retained owner even during
                // cleanup/Drop. Keep this peak funded until physical disposal;
                // compiled-runtime accounting covers only retained storage.
                live_peak,
                total,
                program,
                f.retained_bytes(),
                owned,
                total,
                1,
                1,
                1,
                p.temporary_bytes,
                1,
                size_of::<Option<CoreError>>() as u64,
            ])
        };
        let amounts = match admit() {
            Ok(amounts) => amounts,
            Err(error) => {
                return Err(BindingInputRejection::new(
                    (prepared, authority, limits, parent),
                    BindingOperationalError::new(error),
                ));
            }
        };
        let mut charge = Charge {
            parent: parent.clone(),
            held: [0; 12],
        };
        let reserve = (|| -> CoreResult<()> {
            for i in 0..12 {
                check(&limits, KINDS[i], amounts[i], quota_context)?;
                let limit = parent.limits[i];
                if !parent.try_reserve(i, amounts[i]) {
                    return Err(CoreError::LimitExceeded {
                        context: quota_context,
                        resource: KINDS[i],
                        limit,
                        requested: Some(amounts[i]),
                        observed: None,
                    });
                }
                charge.held[i] = amounts[i];
            }
            Ok(())
        })();
        if let Err(error) = reserve {
            drop(charge);
            return Err(BindingInputRejection::new(
                (prepared, authority, limits, parent),
                BindingOperationalError::new(error),
            ));
        }
        let drop_work = prepared.install_policy().local_drop_work;
        let mut owner = match clinkz_wot_core::artifact::try_box(Owner::<E, H> {
            execution: None,
            installation: None,
        }) {
            Ok(owner) => owner,
            Err(_) => {
                drop(charge);
                return Err(BindingInputRejection::new(
                    (prepared, authority, limits, parent),
                    BindingOperationalError::new(CoreError::Backpressure(
                        quota_context.with_resource_failure(
                            identity.configuration(),
                            ResourceFailureDomain::DeploymentFailure,
                            None,
                            None,
                        ),
                    )),
                ));
            }
        };
        owner.execution = Some(prepared.into_execution());
        Ok(Self {
            authority,
            owner: Some(owner),
            charge: Some(charge),
            state: ProducerInstallationState::Reserved,
            first_cause: None,
            deadline,
            terminal_work: WorkBudget::new().with_remaining(WorkClass::CleanupItems, drop_work),
        })
    }
    pub fn state(&self) -> ProducerInstallationState {
        self.state
    }
    pub fn instance_key(&self) -> ProducerInstanceKey {
        self.authority.key()
    }
    pub fn first_cause(&self) -> Option<&CoreError> {
        self.first_cause.as_ref()
    }
    pub fn entry(&self) -> Option<&ProducerExecutionEntry<E::Program>> {
        self.owner
            .as_ref()
            .map(|o| o.execution.as_ref().unwrap().entry())
    }
    pub fn begin_close(&mut self) {
        if self.state != ProducerInstallationState::Closed {
            self.state = ProducerInstallationState::Closing;
        }
    }
    pub fn cancel(&mut self) {
        if self.state == ProducerInstallationState::Closed {
            return;
        }
        self.fail(CoreError::Cancelled(ErrorContext::new(
            ErrorPhase::Cleanup,
            RetryClass::Never,
        )));
    }
    fn fail(&mut self, error: CoreError) {
        if self.first_cause.is_none() {
            self.first_cause = Some(error);
        }
        self.state = ProducerInstallationState::Closing;
    }
    /// One funded construction, installation or close callback. No callbacks
    /// run with zero credit. Deadline/clock failure closes authority locally;
    /// first cause survives subsequent cleanup failures and cancellation.
    pub fn step(
        &mut self,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
        now: MonotonicInstant,
    ) -> Poll<ProducerInstallationState> {
        use ProducerInstallationState::*;
        if matches!(self.state, Reserved | Installing) {
            match self.deadline.checked_is_elapsed_at(now) {
                Some(false) => {}
                Some(true) => self.fail(CoreError::TimedOut(ErrorContext::new(
                    ErrorPhase::Binding,
                    RetryClass::Never,
                ))),
                None => self.fail(invalid()),
            }
        }
        if matches!(self.state, Closed | InstalledClosed) {
            return Poll::Ready(self.state);
        }
        let class = if self.state == Closing {
            WorkClass::CleanupItems
        } else {
            WorkClass::BindingPolls
        };
        if budget.consume(class, 1).is_err() {
            return Poll::Pending;
        }
        match self.state {
            Reserved => {
                match self
                    .owner
                    .as_ref()
                    .unwrap()
                    .execution
                    .as_ref()
                    .unwrap()
                    .create_installation()
                {
                    Ok(installation) => {
                        self.owner.as_mut().unwrap().installation = Some(installation);
                        self.state = Installing;
                    }
                    Err(error) => self.fail(error),
                }
                Poll::Ready(self.state)
            }
            Installing => {
                if let Err(error) = self
                    .owner
                    .as_ref()
                    .unwrap()
                    .execution
                    .as_ref()
                    .unwrap()
                    .check_current()
                {
                    self.fail(error);
                    return Poll::Ready(self.state);
                }
                match self
                    .owner
                    .as_mut()
                    .unwrap()
                    .installation
                    .as_mut()
                    .unwrap()
                    .poll_install(cx)
                {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(Ok(())) => {
                        self.state = InstalledClosed;
                        Poll::Ready(self.state)
                    }
                    Poll::Ready(Err(error)) => {
                        self.fail(error);
                        Poll::Ready(self.state)
                    }
                }
            }
            Closing => {
                if let Some(installation) = self.owner.as_mut().unwrap().installation.as_mut() {
                    match installation.poll_close(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(error)) => {
                            self.fail(error);
                            return Poll::Ready(self.state);
                        }
                        Poll::Ready(Ok(())) => {}
                    }
                }
                self.release();
                self.state = Closed;
                Poll::Ready(Closed)
            }
            _ => unreachable!(),
        }
    }
}
impl<E: ProducerExecutor, H: ReadPropertyHandler> ProducerInstallationRuntime<E, H> {
    fn release(&mut self) {
        if self.owner.is_some() {
            self.terminal_work
                .consume(
                    WorkClass::CleanupItems,
                    self.terminal_work.remaining(WorkClass::CleanupItems),
                )
                .expect("prepaid local disposal");
            drop(self.owner.take());
            // After physical Box release only inline terminal/status backing
            // remains charged. The root's Drop releases that final allowance.
            self.charge.as_mut().unwrap().retain_status(
                size_of::<Self>() as u64,
                size_of::<Option<CoreError>>() as u64,
            );
        }
    }
}
impl<E: ProducerExecutor, H: ReadPropertyHandler> Drop for ProducerInstallationRuntime<E, H> {
    fn drop(&mut self) {
        self.release();
        drop(self.charge.take());
    }
}
