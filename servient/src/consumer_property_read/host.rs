use super::shared::{FinalRelease, Shared};
use super::*;
use alloc::sync::Arc;
use clinkz_wot_planning::{
    ConsumerPropertyReadOwnedOutput as OwnedOutput,
    ConsumerPropertyReadOwnedRollback as OwnedRollback,
};
use std::sync::{Mutex, MutexGuard};

type Registration = ConsumerCompilerRegistration<HostConsumerBindingPreparation>;
#[derive(Clone)]
struct HostParent(Arc<crate::host_accounts::HostResourceAccounts>);
impl Parent for HostParent {
    fn snapshot(&self) -> [ResourceAccount; N] {
        self.0.snapshot(KINDS)
    }
    fn reserve(&self, amount: [u64; N]) -> Option<(usize, SlotIndex, Generation, u64, u64)> {
        for i in 0..N {
            if !self.0.try_reserve(KINDS[i], amount[i]) {
                for j in 0..i {
                    self.0.release(KINDS[j], amount[j]);
                }
                let a = self.0.account(KINDS[i]);
                let (slot, generation) = self.0.coordinates(KINDS[i]);
                return Some((i, slot, generation, a.limit(), a.used()));
            }
        }
        None
    }
    fn release(&self, amount: [u64; N]) {
        for (kind, n) in KINDS.into_iter().zip(amount) {
            self.0.release(kind, n);
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

struct Report {
    value: Mutex<(Option<CleanupOutcome>, [u64; 4])>,
    charge: Option<Charge<HostParent>>,
}
impl Drop for Report {
    fn drop(&mut self) {
        drop(self.charge.take());
    }
}
impl FinalRelease for Report {}
struct GenerationOwner {
    output: Option<OwnedOutput<HostBindingArtifact>>,
    rollback: Option<OwnedRollback<HostBindingArtifact>>,
    charge: Option<Charge<HostParent>>,
    terminal: Terminal,
    census: Preflight,
    nonce: u64,
    slot: Generation,
    plan: PlanSetGeneration,
    identity: BindingRegistrationIdentity,
}
impl GenerationOwner {
    fn discard(&mut self) {
        let (rollback, root) = if let Some(output) = self.output.take() {
            let (draft, root) = output.into_local();
            (Some(draft.into_rollback()), Some(root))
        } else if let Some(rollback) = self.rollback.take() {
            let (rollback, root) = rollback.into_local();
            (Some(rollback), Some(root))
        } else {
            (None, None)
        };
        if let Some(rollback) = rollback {
            self.terminal.discard(rollback);
        }
        drop(root);
        drop(self.charge.take());
    }
}
impl Drop for GenerationOwner {
    fn drop(&mut self) {
        self.discard();
    }
}

struct CallOwner {
    raw: Option<HostBindingCallBox<CoreResult<InteractionOutput>>>,
    result: Option<CoreResult<InteractionOutput>>,
    charge: Option<Charge<HostParent>>,
    slot: Generation,
    plan: PlanSetGeneration,
    generation: Generation,
    transfer: Option<CleanupTransferRequest>,
}
fn transfer_bytes(policy: &ConsumerPropertyReadPolicy) -> CoreResult<u64> {
    // Complete physical destination plus its indirect call/request/result
    // allowance. Inline account/lease/phase and durable status are included.
    add(
        size_of::<(CleanupTransferEnvelope<CallOwner>, CleanupRecord)>() as u64,
        policy.call_bytes()?,
    )
}
impl CallOwner {
    fn transfer_backed(&self, bytes: u64) -> bool {
        self.charge.as_ref().is_some_and(|c| {
            c.held[TRANSFER_SLOTS] == 1 && c.held[TRANSFER_BYTES] >= bytes && c.is_held()
        })
    }
}
impl Drop for CallOwner {
    fn drop(&mut self) {
        drop(self.result.take());
        drop(self.raw.take());
        self.transfer = None;
        drop(self.charge.take());
    }
}
struct Driver {
    registration: Option<Registration>,
    generation: Option<GenerationOwner>,
    call: Option<CallOwner>,
    manual: Option<CleanupTransferEnvelope<CallOwner>>,
    provision: Option<Charge<HostParent>>,
    terminal_work: WorkBudget,
}
impl Drop for Driver {
    fn drop(&mut self) {
        drop(self.call.take());
        drop(self.manual.take());
        drop(self.generation.take());
        drop(self.registration.take());
        drop(self.provision.take());
    }
}
struct CellState {
    kernel: Kernel,
    driver: Option<Driver>,
    busy: bool,
    handle_ready: bool,
}
struct Root {
    policy: Option<ConsumerPropertyReadPolicy>,
    clock: Arc<dyn RuntimeClock + Send + Sync>,
    parent: HostParent,
    report: Shared<Report>,
    cell: Mutex<CellState>,
}
impl Root {
    fn policy(&self) -> &ConsumerPropertyReadPolicy {
        self.policy.as_ref().expect("constructed policy owner")
    }
    fn write_report(&self, outcome: CleanupOutcome) {
        let mut r = lock(&self.report.get().value);
        if !r.0.is_some_and(|v| v.is_residual()) {
            r.0 = Some(outcome);
        }
    }
    fn finish_transfer(&self, call: &CallOwner) {
        let Some(request) = &call.transfer else {
            return;
        };
        let mut report = lock(&self.report.get().value);
        if let Some(CleanupOutcome::PendingCleanup(record)) = report.0
            && record.subject() == request.phase().reservation().subject()
            && record.owner() == request.requested_owner()
        {
            report.0 = Some(CleanupOutcome::Complete);
        }
    }
}
impl FinalRelease for Root {
    fn before_release(&mut self) {
        let cell = self.cell.get_mut().unwrap_or_else(|e| e.into_inner());
        if !cell.kernel.shutdown
            || cell.kernel.phase != CompiledPlanSetState::Reclaimed
            || cell.kernel.call_phase != CallPhase::Vacant
        {
            lock(&self.report.get().value).0 = Some(CleanupOutcome::ResidualExternalState(
                cell.kernel.record(499),
            ));
        }
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let cell = self.cell.get_mut().unwrap_or_else(|e| e.into_inner());
        assert!(!cell.busy, "an exclusive driver retains a strong root");
        let incomplete = !cell.kernel.shutdown
            || cell.kernel.phase != CompiledPlanSetState::Reclaimed
            || cell.kernel.call_phase != CallPhase::Vacant;
        // Commit the durable disposition before *any* unresolved local owner
        // is destroyed. No clock, client progress, or allocation occurs here.
        if incomplete {
            let mut report = lock(&self.report.get().value);
            report.0 = Some(CleanupOutcome::ResidualExternalState(
                cell.kernel.record(499),
            ));
        }
        if let Some(mut driver) = cell.driver.take() {
            driver
                .terminal_work
                .consume(
                    W::CleanupItems,
                    driver.terminal_work.remaining(W::CleanupItems),
                )
                .expect("one prepaid terminal root allowance");
            drop(driver.call.take());
            drop(driver.manual.take());
            if let Some(mut generation) = driver.generation.take() {
                generation.discard();
                let mut report = lock(&self.report.get().value);
                report.1 = [
                    generation.terminal.used_steps,
                    generation.terminal.steps,
                    generation.terminal.work.remaining(W::CodecInputBytes),
                    generation.terminal.work.remaining(W::PlanningItems),
                ];
            }
            drop(driver); // Registration, then original provisioning/parent.
        }
    }
}

/// Non-Clone Host manual root. Its complete backing is fallibly provisioned.
pub struct ConsumerPropertyReadRuntime {
    root: Shared<Root>,
}
impl ConsumerPropertyReadRuntime {
    /// Returns the runtime and a non-Clone fixed report observer. The observer
    /// owns its provisioning allowance independently of every Runtime owner.
    /// Its four counters are terminal steps, admitted step bound, remaining
    /// byte credit and remaining item credit. Drop it to release report backing.
    #[allow(clippy::type_complexity)] // Ownership-preserving existing types; no extra public family.
    pub fn try_new(
        registration: Registration,
        policy: ConsumerPropertyReadPolicy,
        clock: Arc<dyn RuntimeClock + Send + Sync>,
        parent: Arc<crate::host_accounts::HostResourceAccounts>,
    ) -> Result<
        (
            Self,
            impl Fn() -> (Option<CleanupOutcome>, [u64; 4]) + Send + Sync + 'static,
        ),
        BindingInputRejection<(
            Registration,
            ConsumerPropertyReadPolicy,
            Arc<dyn RuntimeClock + Send + Sync>,
            Arc<crate::host_accounts::HostResourceAccounts>,
        )>,
    > {
        let supported = (|| {
            policy.qualify(&registration, registration.registration().resources(), true)?;
            policy.check(K::CleanupTransferSlotsGlobalMax, 1)?;
            policy.check(K::CleanupTransferBytesGlobalMax, transfer_bytes(&policy)?)?;
            Ok(())
        })();
        if let Err(e) = supported {
            let e = policy.resource_error(e, ResourceFailureDomain::EngineCapacity);
            return Err(reject((registration, policy, clock, parent), e));
        }
        let report_bytes = Shared::<Report>::allocation_layout().size() as u64;
        let root_bytes = Shared::<Root>::allocation_layout().size() as u64;
        if let Err(error) = policy.check(K::CleanupItemBytesMax, report_bytes) {
            return Err(reject((registration, policy, clock, parent), error));
        }
        let now = match policy.startup_time(&*clock, true) {
            Ok(now) => now,
            Err(error) => return Err(reject((registration, policy, clock, parent), error)),
        };
        let parent = HostParent(parent);
        let scratch = registration
            .registration()
            .resources()
            .transient()
            .peak_bytes();
        let provision = (|| {
            policy.check(
                K::LargestContiguousAllocationBytesMax,
                report_bytes.max(root_bytes),
            )?;
            let upstream = add(
                add(root_bytes, size_of::<Driver>() as u64)?,
                add(
                    add(policy.registration.retained_bytes(), report_bytes)?,
                    scratch,
                )?,
            )?;
            policy.check(
                K::PeakLiveBytesPerAdmissionMax,
                add(
                    upstream,
                    add(
                        policy.scratch,
                        size_of::<BuildStorage<HostBindingArtifact, HostParent>>() as u64,
                    )?,
                )?,
            )?;
            let mut report = Charge::new(parent.clone(), &policy)?;
            report.grow(amounts(&[
                (ENGINE, report_bytes),
                (CLEANUP_BYTES, report_bytes),
                (CLEANUP_ITEMS, 1),
            ]))?;
            report.local[CLEANUP_BYTES]
                .try_reserve(report_bytes)
                .expect("paired report")
                .commit();
            let mut provision = Charge::new(parent.clone(), &policy)?;
            // The parked root and one complete checked-out Driver can overlap.
            let bytes = add(
                add(root_bytes, size_of::<Driver>() as u64)?,
                add(policy.registration.retained_bytes(), scratch)?,
            )?;
            provision.grow(amounts(&[
                (ENGINE, bytes),
                (BINDINGS, 1),
                (TEMP, scratch),
                (POLL_TEMP, scratch),
            ]))?;
            let report = Shared::try_new(Report {
                value: Mutex::new((None, [0; 4])),
                charge: Some(report),
            })
            .map_err(|r| {
                drop(r);
                policy.resource_error(busy(), ResourceFailureDomain::DeploymentFailure)
            })?;
            Ok((report, provision))
        })();
        let (report, provision) = match provision {
            Ok(v) => v,
            Err(e) => return Err(reject((registration, policy, clock, parent.0), e)),
        };
        let observer = report.retain();
        let fixed = policy.terminal_fixed_work().expect("checked startup");
        let value = Root {
            policy: Some(policy),
            clock,
            parent,
            report,
            cell: Mutex::new(CellState {
                kernel: Kernel::new(now),
                driver: Some(Driver {
                    registration: Some(registration),
                    generation: None,
                    call: None,
                    manual: None,
                    provision: Some(provision),
                    terminal_work: WorkBudget::new().with_remaining(W::CleanupItems, fixed),
                }),
                busy: false,
                handle_ready: false,
            }),
        };
        let root = match Shared::try_new(value) {
            Ok(root) => root,
            Err(mut root) => {
                let cell = root.cell.get_mut().unwrap_or_else(|e| e.into_inner());
                let registration = cell.driver.as_mut().unwrap().registration.take().unwrap();
                // Failed construction had no input or protocol effects.
                cell.kernel.shutdown = true;
                let policy = root.policy.take().expect("original checked policy");
                let clock = Arc::clone(&root.clock);
                let parent = Arc::clone(&root.parent.0);
                drop(root);
                drop(observer);
                let error = policy.resource_error(busy(), ResourceFailureDomain::DeploymentFailure);
                return Err(reject((registration, policy, clock, parent), error));
            }
        };
        Ok((Self { root }, move || *lock(&observer.get().value)))
    }
    /// Lends a local consuming build. Drive `step` until `is_finished`, or leave
    /// the scope to abandon it into the already reserved manual root. Complete
    /// builds wait source-free in FinalChecks if `final_work` is short.
    pub fn consume<'td>(
        &self,
        thing: &'td Thing,
        final_work: &mut WorkBudget,
        drive: impl for<'env> FnOnce(&mut HostConsumerPropertyReadBuild<'td, 'env>),
    ) -> CoreResult<StartStatus<ConsumerPropertyReadHandle>> {
        let mut scope = {
            let mut cell = lock(&self.root.get().cell);
            if cell.busy {
                return Err(busy());
            }
            cell.kernel.begin()?;
            cell.handle_ready = false;
            cell.busy = true;
            BuildScope {
                root: self.root.retain(),
                driver: cell.driver.take(),
                storage: None,
                nonce: cell.kernel.nonce,
                slot: cell.kernel.slot,
                plan: cell.kernel.plan,
            }
        };
        let policy = &self.root.get().policy();
        let upstream = add(
            scope
                .driver
                .as_ref()
                .unwrap()
                .provision
                .as_ref()
                .unwrap()
                .held[ENGINE],
            self.root.get().report.get().charge.as_ref().unwrap().held[ENGINE],
        )?;
        scope.storage = Some(
            match BuildStorage::new(self.root.get().parent.clone(), policy, upstream) {
                Ok(storage) => storage,
                Err(e) => {
                    scope.fail(e.clone());
                    return Err(e);
                }
            },
        );
        {
            let storage = scope.storage.as_mut().unwrap();
            let registration = scope
                .driver
                .as_ref()
                .unwrap()
                .registration
                .as_ref()
                .unwrap();
            let root = storage.root.as_mut().unwrap();
            let mut adapter = HostConsumerPropertyReadBuild {
                build: Build {
                    state: Building::Td(
                        ValidatedThingCursor::from_thing(
                            thing,
                            &policy.td,
                            policy.ledger(scope.slot, 0),
                        ),
                        root,
                    ),
                    registration,
                    policy,
                    generation: scope.plan,
                    upstream,
                    charge: storage.charge.as_mut().unwrap(),
                    terminal: &mut storage.terminal,
                    census: &mut storage.census,
                    draft: &mut storage.draft,
                    rollback: &mut storage.rollback,
                    cause: &mut storage.cause,
                    td_cause: &mut storage.td_cause,
                },
                root: &self.root,
                cancelled: false,
            };
            drive(&mut adapter);
        } // Every source/compiler/configuration loan ends before final checks.
        scope.deposit();
        let error = self.build_failure();
        if let Some(e) = error {
            return Err(e);
        }
        self.install(final_work)?;
        Ok(match self.take_handle() {
            Some(h) => StartStatus::Ready(h),
            None => StartStatus::Pending,
        })
    }
    fn install(&self, work: &mut WorkBudget) -> CoreResult<()> {
        install(&self.root, work)
    }
    /// Retrieves the sole handle after deferred final installation.
    pub fn take_handle(&self) -> Option<ConsumerPropertyReadHandle> {
        let mut cell = lock(&self.root.get().cell);
        if !cell.handle_ready
            || cell.kernel.handle_live
            || !matches!(
                cell.kernel.phase,
                CompiledPlanSetState::Published | CompiledPlanSetState::Draining
            )
        {
            return None;
        }
        cell.handle_ready = false;
        cell.kernel.handle_live = true;
        Some(ConsumerPropertyReadHandle {
            root: self.root.retain(),
            slot: cell.kernel.slot,
        })
    }
    pub fn state(&self) -> CompiledPlanSetState {
        lock(&self.root.get().cell).kernel.phase
    }
    /// Original fixed TD cause, including its phase and semantic coordinates.
    pub fn td_diagnostic(&self) -> Option<ValidatedThingCause> {
        lock(&self.root.get().cell).kernel.td_cause
    }
    pub fn build_failure(&self) -> Option<CoreError> {
        lock(&self.root.get().cell).kernel.build_error.clone()
    }
    pub fn generations(&self) -> (Generation, PlanSetGeneration, Generation) {
        let cell = lock(&self.root.get().cell);
        (cell.kernel.slot, cell.kernel.plan, cell.kernel.call)
    }
    /// Closes this generation before reporting Draining; a reclaimed root can
    /// consume again after its old handle and call ticket have been released.
    pub fn close(&self) {
        let mut c = lock(&self.root.get().cell);
        c.kernel
            .close_generation(Cause::Drain, self.root.get().policy().timeouts[1]);
    }
    pub fn cleanup_record(&self) -> Option<CleanupOutcome> {
        lock(&self.root.get().report.get().value).0
    }
    pub fn next_deadline(&self, work: &mut WorkBudget) -> CoreResult<Option<Deadline>> {
        let mut checkout = Checkout::take(&self.root, work).ok_or_else(busy)?;
        if !pay(work, &[(W::BindingPolls, self.root.get().policy().work[2])]) {
            return Err(busy());
        }
        let cell = lock(&self.root.get().cell);
        let deadline = cell.kernel.next_deadline();
        drop(cell);
        let driver = checkout.driver.as_mut().unwrap();
        if driver.manual.is_some() {
            driver.call = take_call(driver);
        }
        let raw = driver
            .call
            .as_mut()
            .and_then(|call| call.raw.as_mut())
            .and_then(|call| call.as_pin_mut().next_deadline());
        Ok(earlier(deadline, raw))
    }
    pub fn step(&self, cx: &mut Context<'_>, work: &mut WorkBudget) -> StepStatus<()> {
        progress(&self.root, cx, work, &mut |_| true)
    }
    /// An optional transfer admission callback can refuse the offered request.
    /// The identical whole envelope then stays in the reserved manual root.
    pub fn step_with_transfer_offer(
        &self,
        cx: &mut Context<'_>,
        work: &mut WorkBudget,
        offer: &mut impl FnMut(&CleanupTransferRequest) -> bool,
    ) -> StepStatus<()> {
        progress(&self.root, cx, work, offer)
    }
    /// Explicitly drive until Ready, then drop facades/root to release their
    /// provisioning. Pending is always retained work, never a cleanup success.
    pub fn shutdown(
        &self,
        cx: &mut Context<'_>,
        work: &mut WorkBudget,
    ) -> StartStatus<CleanupOutcome> {
        let cleanup = CleanupStep::new(
            work,
            self.root
                .get()
                .policy()
                .limit(K::CleanupWorkItemsPerStepMax),
        );
        let work = &mut *cleanup.work;
        {
            let mut c = lock(&self.root.get().cell);
            c.kernel.closed = true;
            c.kernel.discard_results = true;
            c.kernel
                .close_generation(Cause::Drain, self.root.get().policy().timeouts[1]);
        }
        self.step(cx, work);
        let mut c = lock(&self.root.get().cell);
        if c.busy
            || c.kernel.phase != CompiledPlanSetState::Reclaimed
            || c.kernel.call_phase != CallPhase::Vacant
        {
            return StartStatus::Pending;
        }
        if c.driver.as_ref().is_some_and(|d| d.registration.is_some()) {
            drop(c);
            let Some(mut checkout) = Checkout::take(&self.root, work) else {
                return StartStatus::Pending;
            };
            let policy = self.root.get().policy();
            let cost = policy
                .registration_cleanup_work()
                .expect("qualified cleanup quantum");
            if !pay(work, &[(W::CleanupItems, cost)]) {
                return StartStatus::Pending;
            }
            let driver = checkout.driver.as_mut().unwrap();
            if driver.registration.is_some() {
                let scratch = driver.provision.as_ref().unwrap().held[POLL_TEMP];
                drop(driver.registration.take());
                driver.provision.as_mut().unwrap().release(amounts(&[
                    (ENGINE, policy.registration.retained_bytes() + scratch),
                    (BINDINGS, 1),
                    (TEMP, scratch),
                    (POLL_TEMP, scratch),
                ]));
            }
            drop(checkout);
            c = lock(&self.root.get().cell);
            if c.busy {
                return StartStatus::Pending;
            }
        }
        c.kernel.shutdown = true;
        let outcome = if c.kernel.residual {
            CleanupOutcome::ResidualExternalState(c.kernel.record(480))
        } else {
            CleanupOutcome::Complete
        };
        drop(c);
        self.root.get().write_report(outcome);
        StartStatus::Ready(outcome)
    }
}
impl Drop for ConsumerPropertyReadRuntime {
    fn drop(&mut self) {
        let mut c = lock(&self.root.get().cell);
        c.kernel.closed = true;
        c.kernel
            .close_generation(Cause::Drop, self.root.get().policy().timeouts[1]);
    }
}
struct InstallPermit;

/// Local source-bearing progress. Drop never exports a borrowed selection.
pub struct HostConsumerPropertyReadBuild<'td, 'env> {
    build: Build<'td, 'env, Registration, HostParent>,
    root: &'env Shared<Root>,
    cancelled: bool,
}
impl HostConsumerPropertyReadBuild<'_, '_> {
    pub fn step(&mut self, work: &mut WorkBudget) -> StepStatus<()> {
        let cancel = self.cancelled || lock(&self.root.get().cell).kernel.first.is_some();
        let result = self.build.step(work, cancel);
        if let Some(error) = self.build.cause.clone() {
            lock(&self.root.get().cell)
                .kernel
                .build_error
                .get_or_insert(error);
        }
        result
    }
    pub fn cancel(&mut self) {
        self.cancelled = true;
        lock(&self.root.get().cell)
            .kernel
            .close_generation(Cause::Manual, self.root.get().policy().timeouts[1]);
    }
    pub fn is_finished(&self) -> bool {
        self.build.is_finished()
    }
}
struct BuildScope {
    root: Shared<Root>,
    driver: Option<Driver>,
    storage: Option<BuildStorage<HostBindingArtifact, HostParent>>,
    nonce: u64,
    slot: Generation,
    plan: PlanSetGeneration,
}
impl BuildScope {
    fn fail(&mut self, error: CoreError) {
        lock(&self.root.get().cell)
            .kernel
            .build_error
            .get_or_insert(error.clone());
        if let Some(s) = self.storage.as_mut() {
            s.cause.get_or_insert(error);
        }
        lock(&self.root.get().cell)
            .kernel
            .close_generation(Cause::Drop, self.root.get().policy().timeouts[1]);
    }
    fn deposit(&mut self) {
        let mut driver = self.driver.take().expect("one build driver");
        let mut success = false;
        if let Some(mut storage) = self.storage.take() {
            lock(&self.root.get().cell).kernel.td_cause = storage.td_cause;
            if let Some(error) = storage.cause.clone() {
                lock(&self.root.get().cell)
                    .kernel
                    .build_error
                    .get_or_insert(error);
            }
            storage.end_scratch();
            let root = storage.root.take().unwrap();
            let output = storage.draft.take();
            success = output.is_some() && storage.cause.is_none();
            let mut rollback = storage.rollback.take();
            if output.is_none() && rollback.is_none() {
                let mut root = root;
                rollback = root.take();
                storage.root = Some(root);
            } else {
                storage.root = Some(root);
            }
            let root = storage.root.take().unwrap();
            if let Some(draft) = output {
                let identity = draft.registration_identity();
                let output = OwnedOutput::try_from_local(draft, root, &mut storage.terminal.bridge)
                    .unwrap_or_else(|_| panic!("prepaid matching output/root"));
                driver.generation = Some(GenerationOwner {
                    output: Some(output),
                    rollback: None,
                    charge: storage.charge.take(),
                    terminal: core::mem::replace(&mut storage.terminal, Terminal::empty()),
                    census: storage.census.expect("complete preflight"),
                    nonce: self.nonce,
                    slot: self.slot,
                    plan: self.plan,
                    identity,
                });
            } else if let Some(rollback) = rollback {
                match OwnedRollback::try_from_local(rollback, root, &mut storage.terminal.bridge) {
                    Ok(rollback) => {
                        let census = storage.census.expect("variable root has paid preflight");
                        driver.generation = Some(GenerationOwner {
                            output: None,
                            rollback: Some(rollback),
                            charge: storage.charge.take(),
                            terminal: core::mem::replace(&mut storage.terminal, Terminal::empty()),
                            census,
                            nonce: self.nonce,
                            slot: self.slot,
                            plan: self.plan,
                            identity: census.registration,
                        });
                    }
                    Err((rollback, root, _)) => {
                        // Fixed, allocation-free rollback.
                        storage.terminal.discard(rollback);
                        drop(root);
                    }
                }
            } else {
                drop(root);
            }
        }
        let mut cell = lock(&self.root.get().cell);
        assert!(cell.busy && cell.driver.is_none());
        if success && cell.kernel.first.is_none() {
            cell.kernel.phase = CompiledPlanSetState::Frozen;
        } else {
            cell.kernel
                .close_generation(Cause::Drop, self.root.get().policy().timeouts[1]);
            if driver.generation.is_none() {
                cell.kernel.phase = CompiledPlanSetState::Reclaimed;
            }
        }
        cell.driver = Some(driver);
        cell.busy = false;
    }
}
impl Drop for BuildScope {
    fn drop(&mut self) {
        if self.driver.is_some() {
            self.fail(Cause::Drop.error());
            self.deposit();
        }
    }
}

struct Checkout {
    root: Shared<Root>,
    driver: Option<Driver>,
    return_work: WorkBudget,
}
impl Checkout {
    fn take(root: &Shared<Root>, work: &mut WorkBudget) -> Option<Self> {
        let mut cell = lock(&root.get().cell);
        if cell.busy || cell.driver.is_none() {
            return None;
        }
        if !pay(work, &[(W::PlanningItems, 1)]) {
            return None;
        }
        cell.busy = true;
        Some(Self {
            root: root.retain(),
            driver: cell.driver.take(),
            return_work: WorkBudget::new().with_remaining(W::PlanningItems, 1),
        })
    }
}
impl Drop for Checkout {
    fn drop(&mut self) {
        let mut cell = lock(&self.root.get().cell);
        assert!(cell.busy && cell.driver.is_none());
        let abandoned_start = cell.kernel.call_phase == CallPhase::Starting
            && self
                .driver
                .as_ref()
                .is_some_and(|d| d.call.is_none() && d.manual.is_none());
        let residual = if abandoned_start {
            cell.kernel.residual = true;
            cell.kernel.acknowledge();
            cell.kernel
                .close_generation(Cause::Drop, self.root.get().policy().timeouts[1]);
            Some(cell.kernel.record(481))
        } else {
            None
        };
        if std::thread::panicking() && self.driver.as_ref().is_some_and(|d| d.call.is_some()) {
            self.driver
                .as_mut()
                .unwrap()
                .call
                .as_mut()
                .unwrap()
                .result
                .get_or_insert(Err(CoreError::Application(
                    context().with_redacted_cause(483, "Consumer driver adapter unwound"),
                )));
            cell.kernel
                .cancel(Cause::Drop, self.root.get().policy().timeouts[1]);
            cell.kernel.call_phase = CallPhase::AwaitingResidual;
        }
        cell.driver = self.driver.take();
        cell.busy = false;
        drop(cell);
        if let Some(record) = residual {
            self.root
                .get()
                .write_report(CleanupOutcome::ResidualExternalState(record));
        }
    }
}
// The short local pair always returns to its original owner, including unwind.
struct SelectionCheckout<'a> {
    destination: &'a mut Option<OwnedOutput<HostBindingArtifact>>,
    pair: Option<(Draft<HostBindingArtifact>, PlanRoot<HostBindingArtifact>)>,
    work: &'a mut WorkBudget,
}
impl Drop for SelectionCheckout<'_> {
    fn drop(&mut self) {
        let (draft, root) = self.pair.take().unwrap();
        *self.destination = Some(
            OwnedOutput::try_from_local(draft, root, self.work)
                .unwrap_or_else(|_| panic!("prepaid exclusive selection return")),
        );
    }
}

/// One non-Clone generation facade. It owns interest, never the sole plan root.
pub struct ConsumerPropertyReadHandle {
    root: Shared<Root>,
    slot: Generation,
}
impl ConsumerPropertyReadHandle {
    pub fn generation(&self) -> Generation {
        self.slot
    }
    /// Failure carries (selection controls, original rejected request if it was
    /// constructed). URI backing occurs in exactly one of those two owners.
    pub fn read_property(
        &self,
        name: &str,
        mut options: InteractionOptions,
        work: &mut WorkBudget,
    ) -> Result<
        ConsumerPropertyReadCall,
        BindingInputRejection<(InteractionOptions, Option<OutboundRequest>)>,
    > {
        let mut checkout = match Checkout::take(&self.root, work) {
            Some(c) => c,
            None => return Err(reject((options, None), busy())),
        };
        let policy = &self.root.get().policy();
        let driver = checkout.driver.as_mut().unwrap();
        let generation = match driver.generation.as_mut() {
            Some(g) if g.output.is_some() => g,
            _ => return Err(reject((options, None), busy())),
        };
        if let Err(e) = policy.validate_input(name, &options, work, generation.census) {
            return Err(reject((options, None), e));
        }
        let now = self.root.get().clock.now();
        let deadline = match policy.deadline(now, &*self.root.get().clock, &options) {
            Ok(d) => d,
            Err(e) => return Err(reject((options, None), e)),
        };
        let call_generation = {
            let mut cell = lock(&self.root.get().cell);
            if now.checked_duration_since(cell.kernel.last_now).is_none() {
                return Err(reject((options, None), invalid()));
            }
            cell.kernel.expire(now, policy.timeouts[1]);
            match cell.kernel.start_call(self.slot, deadline) {
                Ok(g) => g,
                Err(e) => return Err(reject((options, None), e)),
            }
        };
        let mut charge = match Charge::new(self.root.get().parent.clone(), policy) {
            Ok(c) => c,
            Err(e) => {
                lock(&self.root.get().cell).kernel.acknowledge();
                return Err(reject((options, None), e));
            }
        };
        let call_bytes = policy.call_bytes().expect("checked policy");
        let temporary = policy
            .call_temporary(
                driver
                    .registration
                    .as_ref()
                    .unwrap()
                    .registration()
                    .resources(),
            )
            .expect("qualified callback storage");
        let peak = match [
            driver.provision.as_ref().unwrap().held[ENGINE],
            generation.charge.as_ref().unwrap().held[ENGINE],
            self.root.get().report.get().charge.as_ref().unwrap().held[ENGINE],
            call_bytes,
            temporary,
        ]
        .into_iter()
        .try_fold(0, add)
        {
            Ok(peak) => peak,
            Err(error) => {
                lock(&self.root.get().cell).kernel.acknowledge();
                return Err(reject((options, None), error));
            }
        };
        if let Err(error) = policy.check(K::PeakLiveBytesPerAdmissionMax, peak) {
            lock(&self.root.get().cell).kernel.acknowledge();
            return Err(reject((options, None), error));
        }
        if let Err(e) = charge.grow(amounts(&[
            (ENGINE, call_bytes + temporary),
            (TEMP, temporary),
            (PEAK, temporary),
            (PINS, 1),
            (CALLS, 1),
            (HOST_BYTES, call_bytes),
            (CANCEL_BYTES, call_bytes),
            (TRANSFER_SLOTS, 1),
            (
                TRANSFER_BYTES,
                transfer_bytes(policy).expect("qualified destination"),
            ),
        ])) {
            lock(&self.root.get().cell).kernel.acknowledge();
            return Err(reject((options, None), e));
        }
        let output = generation.output.take().unwrap();
        let selection = SelectionCheckout {
            destination: &mut generation.output,
            pair: Some(output.into_local()),
            work: &mut checkout.return_work,
        };
        let selected = match select_consumer_property_read_aggregate(
            &selection.pair.as_ref().unwrap().0,
            name,
            &options,
        ) {
            Ok(s) => s,
            Err(e) => {
                lock(&self.root.get().cell).kernel.acknowledge();
                return Err(reject((options, None), e));
            }
        };
        let request = OutboundRequest::property_read(
            selected.artifact_ref(),
            core::mem::take(&mut options.uri_variables),
            Some(deadline),
        )
        .expect("existing selector yields checked Consumer identity");
        let started = driver
            .registration
            .as_ref()
            .unwrap()
            .registration()
            .start_consumer_property_read(request, selected.artifact());
        drop(selection); // Repark the unchanged graph before root reference release.
        match started {
            Err(rejection) => {
                let (request, error) = rejection.into_parts();
                let error = if policy.contract != BindingResourceContract::StrictV1
                    && matches!(error.error(), CoreError::Backpressure(_))
                    && error.error().context().cause_code() == Some(314)
                {
                    // Core's preacceptance envelope allocation failed. Binding
                    // rejections keep their own policy/domain and route context.
                    let (route, error) = error.into_parts();
                    let error =
                        policy.resource_error(error, ResourceFailureDomain::DeploymentFailure);
                    match route {
                        Some(route) => BindingOperationalError::for_route(route, error),
                        None => BindingOperationalError::new(error),
                    }
                } else {
                    error
                };
                lock(&self.root.get().cell).kernel.acknowledge();
                Err(BindingInputRejection::new((options, Some(request)), error))
            }
            Ok(raw) => {
                charge.release(amounts(&[
                    (ENGINE, temporary),
                    (TEMP, temporary),
                    (PEAK, temporary),
                ]));
                driver.call = Some(CallOwner {
                    raw: Some(raw),
                    result: None,
                    charge: Some(charge),
                    slot: self.slot,
                    plan: generation.plan,
                    generation: call_generation,
                    transfer: None,
                });
                let mut cell = lock(&self.root.get().cell);
                cell.kernel.call_phase = CallPhase::Running;
                cell.kernel.ticket_live = true;
                Ok(ConsumerPropertyReadCall {
                    root: Some(self.root.retain()),
                    slot: self.slot,
                    generation: call_generation,
                    live: true,
                })
            }
        }
    }
    pub fn close(&self) {
        let mut c = lock(&self.root.get().cell);
        if c.kernel.slot == self.slot {
            c.kernel
                .close_generation(Cause::Drain, self.root.get().policy().timeouts[1]);
        }
    }
    pub fn step(&self, cx: &mut Context<'_>, work: &mut WorkBudget) -> StepStatus<()> {
        progress(&self.root, cx, work, &mut |_| true)
    }
}
impl Drop for ConsumerPropertyReadHandle {
    fn drop(&mut self) {
        let mut c = lock(&self.root.get().cell);
        if c.kernel.slot == self.slot {
            c.kernel.handle_live = false;
            c.kernel
                .close_generation(Cause::Drop, self.root.get().policy().timeouts[1]);
        }
    }
}
/// One non-Clone call interest. Its loss commits cancellation, retaining the
/// complete decorated call, generation lease and capacity in the manual root.
pub struct ConsumerPropertyReadCall {
    root: Option<Shared<Root>>,
    slot: Generation,
    generation: Generation,
    live: bool,
}
impl ConsumerPropertyReadCall {
    pub fn generation(&self) -> Generation {
        self.generation
    }
    pub fn cancel(&self) -> CoreResult<()> {
        let root = self.root.as_ref().ok_or_else(stale)?;
        let mut c = lock(&root.get().cell);
        if c.kernel.slot != self.slot
            || c.kernel.call != self.generation
            || c.kernel.call_phase == CallPhase::Vacant
        {
            return Err(stale());
        }
        c.kernel
            .cancel(Cause::Manual, root.get().policy().timeouts[1]);
        Ok(())
    }
    pub fn step(&self, cx: &mut Context<'_>, work: &mut WorkBudget) -> StepStatus<()> {
        if let Some(root) = &self.root {
            progress(root, cx, work, &mut |_| true)
        } else {
            StepStatus::progress(Some(()), None)
        }
    }
    /// Consumes the result opportunity. Raw state is destroyed and the exact
    /// lease released before this succeeds; short work preserves the ticket.
    pub fn try_take(
        &mut self,
        work: &mut WorkBudget,
    ) -> CoreResult<Option<CoreResult<InteractionOutput>>> {
        let root = self.root.as_ref().ok_or_else(stale)?;
        let cleanup = CleanupStep::new(
            work,
            root.get().policy().limit(K::CleanupWorkItemsPerStepMax),
        );
        let work = &mut *cleanup.work;
        let mut checkout = match Checkout::take(root, work) {
            Some(c) => c,
            None => return Ok(None),
        };
        let mut c = lock(&root.get().cell);
        if c.kernel.slot != self.slot
            || c.kernel.call != self.generation
            || !self.live
            || c.kernel.call_phase == CallPhase::Vacant
        {
            return Err(stale());
        }
        if c.kernel.call_phase != CallPhase::Result {
            return Ok(None);
        }
        let cost = root.get().policy().work[5];
        if !pay(work, &[(W::CleanupItems, cost)]) {
            return Ok(None);
        }
        c.kernel.call_phase = CallPhase::Acknowledging;
        drop(c);
        let driver = checkout.driver.as_mut().unwrap();
        let mut call = take_call(driver).expect("terminal whole call");
        root.get().finish_transfer(&call);
        let result = call
            .result
            .take()
            .map(|result| result.map_err(|error| root.get().policy().result_error(error)));
        drop(call); // No structural lock around raw destruction or release.
        let mut c = lock(&root.get().cell);
        c.kernel.acknowledge();
        c.kernel.ticket_live = false;
        self.live = false;
        drop(c);
        drop(checkout); // Repark before releasing this interest reference.
        drop(self.root.take());
        Ok(result)
    }
}
impl Drop for ConsumerPropertyReadCall {
    fn drop(&mut self) {
        if !self.live {
            return;
        }
        let Some(root) = &self.root else {
            return;
        };
        let mut c = lock(&root.get().cell);
        if c.kernel.slot == self.slot && c.kernel.call == self.generation {
            c.kernel.ticket_live = false;
            c.kernel.interested = false;
            c.kernel
                .cancel(Cause::Drop, root.get().policy().timeouts[1]);
        }
    }
}

fn take_call(driver: &mut Driver) -> Option<CallOwner> {
    driver.call.take().or_else(|| {
        driver.manual.take().map(|e| {
            let (request, mut call) = e.into_parts();
            call.transfer = Some(request);
            call
        })
    })
}
fn call_mut(driver: &mut Driver) -> Option<&mut CallOwner> {
    if driver.call.is_some() {
        driver.call.as_mut()
    } else {
        None
    } // Manual envelopes are checked out wholly before progress.
}
struct ManualDestination<'a> {
    destination: &'a mut Option<CleanupTransferEnvelope<CallOwner>>,
    allow: bool,
    record: CleanupRecord,
    bytes: u64,
}
impl CleanupTransferTarget<CallOwner> for ManualDestination<'_> {
    fn try_accept(
        &mut self,
        e: CleanupTransferEnvelope<CallOwner>,
    ) -> CleanupTransferAcceptance<CallOwner> {
        let (request, call) = e.into_parts();
        let backed = call.transfer_backed(self.bytes);
        let e = CleanupTransferEnvelope::new(request, call);
        if !self.allow || self.destination.is_some() || !backed {
            return CleanupTransferAcceptance::Rejected(e);
        }
        *self.destination = Some(e);
        CleanupTransferAcceptance::Accepted(self.record)
    }
}
fn settle(
    root: &Shared<Root>,
    driver: &mut Driver,
    settlement: CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>,
    offer: &mut impl FnMut(&CleanupTransferRequest) -> bool,
) {
    match settlement {
        Ok(BindingCallSettlement::Returned(result)) => {
            call_mut(driver).unwrap().result = Some(result);
            lock(&root.get().cell).kernel.settle();
        }
        Ok(BindingCallSettlement::Cancelled {
            disposition: BindingCancellationDisposition::TransferRequired(request),
            ..
        }) => {
            let c = lock(&root.get().cell);
            let bytes = transfer_bytes(root.get().policy()).expect("qualified destination");
            let backed = driver
                .call
                .as_ref()
                .is_some_and(|call| call.transfer_backed(bytes));
            let expected = c.kernel.cleanup_phase(root.get().policy(), backed);
            let valid = backed
                && request.phase() == &expected
                && Some(request.requested_owner()) == expected.transfer_owner();
            let record = c.kernel.record(470);
            drop(c);
            if !valid {
                driver.call.as_mut().unwrap().result =
                    Some(Err(CoreError::Validation(context().with_redacted_cause(
                        482,
                        "cleanup phase does not match its reserved owner",
                    ))));
                residual(root, driver);
                return;
            }
            let allow = offer(&request);
            let call = driver.call.take().unwrap();
            let envelope = CleanupTransferEnvelope::new(request, call);
            let mut destination = ManualDestination {
                destination: &mut driver.manual,
                allow,
                record,
                bytes,
            };
            let accepted = destination.try_accept(envelope);
            match accepted {
                CleanupTransferAcceptance::Accepted(record) => root
                    .get()
                    .write_report(CleanupOutcome::PendingCleanup(record)),
                CleanupTransferAcceptance::Rejected(envelope) => {
                    // Exact rejected owner becomes unconditional root-owned
                    // manual work, never a fabricated external acknowledgement.
                    driver.manual = Some(envelope);
                }
            }
            lock(&root.get().cell).kernel.call_phase = CallPhase::Manual;
        }
        Ok(BindingCallSettlement::Cancelled { disposition, .. }) => {
            let mut c = lock(&root.get().cell);
            if let BindingCancellationDisposition::ResidualExternalState { record, .. } =
                disposition
            {
                c.kernel.residual = true;
                drop(c);
                root.get()
                    .write_report(CleanupOutcome::ResidualExternalState(record));
                c = lock(&root.get().cell);
            }
            let error = c.kernel.call_first.unwrap_or(Cause::Manual).error();
            c.kernel.settle();
            drop(c);
            driver.call.as_mut().unwrap().result = Some(Err(error));
        }
        Err(error) => {
            driver.call.as_mut().unwrap().result = Some(Err(error));
            lock(&root.get().cell).kernel.call_phase = CallPhase::AwaitingResidual;
        }
    }
}
fn residual(root: &Shared<Root>, driver: &mut Driver) {
    let mut c = lock(&root.get().cell);
    c.kernel.residual = true;
    let record = c.kernel.record(480);
    let error = c.kernel.call_first.unwrap_or(Cause::Deadline).error();
    c.kernel.settle();
    drop(c);
    root.get()
        .write_report(CleanupOutcome::ResidualExternalState(record));
    driver
        .call
        .as_mut()
        .unwrap()
        .result
        .get_or_insert(Err(error));
}
fn progress(
    root: &Shared<Root>,
    cx: &mut Context<'_>,
    work: &mut WorkBudget,
    offer: &mut impl FnMut(&CleanupTransferRequest) -> bool,
) -> StepStatus<()> {
    let cleanup = CleanupStep::new(
        work,
        root.get().policy().limit(K::CleanupWorkItemsPerStepMax),
    );
    let work = &mut *cleanup.work;
    if lock(&root.get().cell).kernel.phase == CompiledPlanSetState::Frozen {
        let _ = install(root, work);
        return pending();
    }
    let policy = root.get().policy();
    let now = root.get().clock.now();
    lock(&root.get().cell)
        .kernel
        .expire(now, policy.timeouts[1]);
    let mut checkout = match Checkout::take(root, work) {
        Some(c) => c,
        None => return pending(),
    };
    let driver = checkout.driver.as_mut().unwrap();
    let mut c = lock(&root.get().cell);
    if driver.manual.is_some() {
        driver.call = take_call(driver);
    }
    let phase = c.kernel.call_phase;
    if let Some(call) = driver.call.as_mut() {
        assert_eq!(
            (call.slot, call.plan, call.generation),
            (c.kernel.slot, c.kernel.plan, c.kernel.call)
        );
        if matches!(
            phase,
            CallPhase::Running
                | CallPhase::Cancelling
                | CallPhase::Manual
                | CallPhase::AwaitingResidual
        ) {
            if c.kernel.cancel_expired() {
                drop(c);
                residual(root, driver);
                c = lock(&root.get().cell);
            } else if c.kernel.call_first.is_some() && phase != CallPhase::AwaitingResidual {
                let cost = if phase == CallPhase::Running {
                    policy.work[4]
                } else {
                    policy.work[3]
                };
                if pay(work, &[(W::CleanupItems, 1), (W::BindingPolls, cost)]) {
                    let mut callback = WorkBudget::new().with_remaining(W::BindingPolls, cost);
                    if phase == CallPhase::Running {
                        let backed = call.transfer_backed(
                            transfer_bytes(policy).expect("qualified destination"),
                        );
                        let cleanup = c.kernel.cleanup_phase(policy, backed);
                        c.kernel.call_phase = CallPhase::Cancelling;
                        drop(c);
                        let result = call.raw.as_mut().unwrap().as_pin_mut().start_cancel(
                            cx,
                            cleanup,
                            &mut callback,
                        );
                        match result {
                            Ok(StartStatus::Pending) => {}
                            Ok(StartStatus::Ready(s)) => settle(root, driver, Ok(s), offer),
                            Err(e) => settle(root, driver, Err(e), offer),
                        }
                    } else {
                        drop(c);
                        if let Poll::Ready(s) = call
                            .raw
                            .as_mut()
                            .unwrap()
                            .as_pin_mut()
                            .poll_cancel(cx, &mut callback)
                        {
                            settle(root, driver, s, offer);
                        }
                    }
                    c = lock(&root.get().cell);
                }
            } else if c.kernel.call_first.is_none()
                && phase == CallPhase::Running
                && pay(work, &[(W::BindingPolls, policy.work[3])])
            {
                let mut callback =
                    WorkBudget::new().with_remaining(W::BindingPolls, policy.work[3]);
                drop(c);
                if let Poll::Ready(result) = call
                    .raw
                    .as_mut()
                    .unwrap()
                    .as_pin_mut()
                    .poll_result(cx, &mut callback)
                {
                    call.result = Some(result);
                    lock(&root.get().cell).kernel.settle();
                }
                c = lock(&root.get().cell);
            }
        }
    }
    if c.kernel.call_phase == CallPhase::Result
        && (!c.kernel.interested || c.kernel.discard_results)
        && pay(work, &[(W::CleanupItems, policy.work[5])])
    {
        c.kernel.call_phase = CallPhase::Acknowledging;
        drop(c);
        let call = take_call(driver).expect("terminal whole call");
        root.get().finish_transfer(&call);
        drop(call);
        c = lock(&root.get().cell);
        c.kernel.acknowledge();
    }
    if c.kernel.phase == CompiledPlanSetState::Draining && c.kernel.call_phase == CallPhase::Vacant
    {
        drop(c);
        if let Some(g) = driver.generation.as_mut() {
            if g.rollback.is_none() {
                let (draft, root) = g.output.take().unwrap().into_local();
                g.rollback = Some(
                    OwnedRollback::try_from_local(
                        draft.into_rollback(),
                        root,
                        &mut checkout.return_work,
                    )
                    .unwrap_or_else(|_| panic!("prepaid reclaim checkout")),
                );
                // Conversion consumed the checkout credit; no second
                // conversion is needed until a later driver step.
            } else {
                let (rollback, local_root) = g.rollback.take().unwrap().into_local();
                match rollback.step(work) {
                    ReclaimStep::Pending(rollback) => {
                        g.rollback = Some(
                            OwnedRollback::try_from_local(
                                rollback,
                                local_root,
                                &mut checkout.return_work,
                            )
                            .unwrap_or_else(|_| panic!("prepaid rollback return")),
                        );
                    }
                    ReclaimStep::Complete(_) => {
                        drop(local_root);
                        drop(driver.generation.take());
                        lock(&root.get().cell).kernel.phase = CompiledPlanSetState::Reclaimed;
                    }
                }
            }
        } else {
            lock(&root.get().cell).kernel.phase = CompiledPlanSetState::Reclaimed;
        }
        c = lock(&root.get().cell);
    }
    let idle = c.kernel.phase == CompiledPlanSetState::Reclaimed
        && c.kernel.call_phase == CallPhase::Vacant;
    if idle {
        StepStatus::progress(Some(()), None)
    } else {
        pending()
    }
}
fn pending() -> StepStatus<()> {
    StepStatus::progress(None, Some(PendingWork::new(PendingWorkClass::Cleanup)))
}

fn install(root: &Shared<Root>, work: &mut WorkBudget) -> CoreResult<()> {
    let mut cell = lock(&root.get().cell);
    if cell.busy || cell.kernel.phase != CompiledPlanSetState::Frozen {
        return Ok(());
    }
    if !pay(work, &[(W::PlanningItems, FINAL_ITEMS)]) {
        return Ok(());
    }
    let generation = cell
        .driver
        .as_ref()
        .and_then(|d| d.generation.as_ref())
        .ok_or_else(invariant)?;
    let registration = cell.driver.as_ref().unwrap().registration.as_ref().unwrap();
    let check = cell.kernel.final_checks(
        generation.nonce,
        generation.slot,
        generation.plan,
        generation.identity,
        registration.registration().identity(),
        root.get()
            .report
            .get()
            .charge
            .as_ref()
            .is_some_and(Charge::is_held)
            && generation.output.is_some()
            && generation.charge.as_ref().is_some_and(|c| {
                c.is_held() && c.held[OUTPUT] >= generation.census.output_capacity_bytes
            })
            && cell
                .driver
                .as_ref()
                .unwrap()
                .provision
                .as_ref()
                .is_some_and(Charge::is_held),
    );
    if let Err(e) = check {
        cell.kernel.build_error.get_or_insert(e.clone());
        cell.kernel
            .close_generation(Cause::Drain, root.get().policy().timeouts[1]);
        return Err(e);
    }
    // Private permit. From here through install there is only infallible
    // movement/assignment of already backed complete owners under this lock.
    let _permit = InstallPermit;
    cell.kernel.phase = CompiledPlanSetState::Published;
    cell.handle_ready = true;
    Ok(())
}

fn earlier(left: Option<Deadline>, right: Option<Deadline>) -> Option<Deadline> {
    match (left, right) {
        (Some(l), Some(r)) => match (l.instant(), r.instant()) {
            (Some(li), Some(ri)) if ri.checked_cmp(li) == Some(core::cmp::Ordering::Less) => {
                Some(r)
            }
            _ => Some(l),
        },
        (Some(l), None) => Some(l),
        (None, r) => r,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clinkz_wot_consumer_property_read_binding_fixture as binding;
    use clinkz_wot_foundation::{GatewayDefaultV1, StaticResourceProfile};

    #[test]
    fn resource_ownership_native_layouts_cover_complete_aggregate_and_transfer_owners() {
        use binding::{State, StaticClient};
        use core::alloc::Layout;
        use std::println;
        println!(
            "ownership Host Layouts: root={:?}, report={:?}, driver={:?}, call={:?}, charge={:?}, build={:?}, draft={:?}, rollback={:?}, cleanup-root={}, terminal={:?}, transfer={:?}",
            Shared::<Root>::allocation_layout(),
            Shared::<Report>::allocation_layout(),
            Layout::new::<Driver>(),
            Layout::new::<CallOwner>(),
            Layout::new::<Charge<HostParent>>(),
            Layout::new::<BuildStorage<HostBindingArtifact, HostParent>>(),
            Layout::new::<Draft<HostBindingArtifact>>(),
            Layout::new::<Rollback<HostBindingArtifact>>(),
            PlanRoot::<HostBindingArtifact>::required_capacity(),
            Layout::new::<Terminal>(),
            Layout::new::<(CleanupTransferEnvelope<CallOwner>, CleanupRecord)>()
        );
        println!(
            "ownership static Layouts: runtime={:?}, build={:?}, slot={:?}, raw={:?}, registration={:?}, draft={:?}, rollback={:?}, cleanup-root={}, policy={:?}, context={:?}, error={:?}, payload={:?}",
            Layout::new::<StaticConsumerPropertyReadRuntime<'static, StaticClient>>(),
            Layout::new::<BuildStorage<ResolvedTargetArtifact, StaticParent<'static>>>(),
            Layout::new::<StaticConsumerPropertyReadSlot<State>>(),
            Layout::new::<State>(),
            Layout::new::<
                ConsumerCompilerRegistration<StaticConsumerBindingPreparation<StaticClient>>,
            >(),
            Layout::new::<Draft<ResolvedTargetArtifact>>(),
            Layout::new::<Rollback<ResolvedTargetArtifact>>(),
            PlanRoot::<ResolvedTargetArtifact>::required_capacity(),
            Layout::new::<ConsumerPropertyReadPolicy>(),
            Layout::new::<ErrorContext>(),
            Layout::new::<CoreError>(),
            Layout::new::<Payload>()
        );
    }

    fn work(n: u64) -> WorkBudget {
        W::ALL
            .into_iter()
            .fold(WorkBudget::new(), |b, w| b.with_remaining(w, n))
    }
    fn address(root: &Shared<Root>) -> (usize, BindingArtifactRef) {
        let mut checkout = Checkout::take(root, &mut work(1)).unwrap();
        let g = checkout
            .driver
            .as_mut()
            .unwrap()
            .generation
            .as_mut()
            .unwrap();
        let pair = g.output.take().unwrap().into_local();
        let selected =
            select_consumer_property_read_aggregate(&pair.0, "a", &InteractionOptions::new())
                .unwrap();
        let result = (
            selected.artifact() as *const _ as usize,
            selected.artifact_ref(),
        );
        g.output = Some(
            OwnedOutput::try_from_local(pair.0, pair.1, &mut checkout.return_work)
                .unwrap_or_else(|_| panic!()),
        );
        result
    }
    #[test]
    fn every_mutable_final_check_rejects_without_replacing_the_original_graph_or_accounts() {
        for fault in 0..7 {
            let stats = Arc::new(binding::Stats::default());
            let outputs = core::array::from_fn(|_| {
                Some(InteractionOutput::with_data(Payload::new(
                    &b"17"[..],
                    "application/json",
                )))
            });
            let registration = binding::host_registration(binding::HostClient::new(stats, outputs));
            let policy = ConsumerPropertyReadPolicy::try_new(
                GatewayDefaultV1::LIMITS.clone(),
                BindingLifetimeFootprint::new(4, 4096),
                BindingLifetimeFootprint::new(8, 1280),
                [1, 64, 64, 64],
                [64, 32],
                [2, 2, 1, 1, 1, 1, 1],
                [50, 5],
                65536,
            )
            .unwrap();
            let parent = Arc::new(
                crate::host_accounts::HostResourceAccounts::try_from_accounts(
                    policy.parent_accounts(),
                )
                .unwrap(),
            );
            let (runtime, observer) = ConsumerPropertyReadRuntime::try_new(
                registration,
                policy,
                Arc::new(binding::Clock::default()),
                parent.clone(),
            )
            .unwrap_or_else(|_| panic!());
            let thing = serde_json::from_str(r#"{"@context":"https://www.w3.org/2022/wot/td/v1.1","id":"urn:a","title":"a","security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}},"properties":{"a":{"forms":[{"href":"https://peer/a"}]}}}"#).unwrap();
            runtime
                .consume(&thing, &mut work(0), |b| {
                    for _ in 0..100000 {
                        b.step(&mut work(4096));
                        if b.is_finished() {
                            return;
                        }
                    }
                    panic!();
                })
                .unwrap();
            let original_address = address(&runtime.root);
            let original = {
                let c = lock(&runtime.root.get().cell);
                let g = c.driver.as_ref().unwrap().generation.as_ref().unwrap();
                (
                    g.nonce,
                    g.slot,
                    g.plan,
                    g.identity,
                    g.charge.as_ref().unwrap().held[OUTPUT],
                )
            };
            let parent_used: [u64; N] = core::array::from_fn(|i| parent.snapshot(KINDS)[i].used());
            let mut stashed = None;
            {
                let mut c = lock(&runtime.root.get().cell);
                if fault == 5 {
                    c.kernel.close_generation(Cause::Manual, 5);
                } else {
                    let g = c.driver.as_mut().unwrap().generation.as_mut().unwrap();
                    match fault {
                        0 => g.nonce += 1,
                        1 => g.slot = g.slot.checked_next().unwrap(),
                        2 => g.plan = g.plan.checked_next().unwrap(),
                        3 => {
                            g.identity = BindingRegistrationIdentity::new(
                                BindingId::new(999),
                                g.identity.binding_generation(),
                                g.identity.configuration(),
                                g.identity.artifact_compatibility(),
                                g.identity.diagnostic_ordinal(),
                            )
                        }
                        4 => g.charge.as_mut().unwrap().held[OUTPUT] = 0,
                        6 => stashed = g.output.take(),
                        _ => unreachable!(),
                    }
                }
            }
            let _ = runtime.install(&mut work(FINAL_ITEMS));
            assert!(runtime.take_handle().is_none());
            assert!(runtime.build_failure().is_some());
            assert_ne!(runtime.state(), CompiledPlanSetState::Published);
            {
                let mut c = lock(&runtime.root.get().cell);
                let g = c.driver.as_mut().unwrap().generation.as_mut().unwrap();
                g.nonce = original.0;
                g.slot = original.1;
                g.plan = original.2;
                g.identity = original.3;
                g.charge.as_mut().unwrap().held[OUTPUT] = original.4;
                if let Some(output) = stashed {
                    g.output = Some(output);
                }
            }
            assert_eq!(address(&runtime.root), original_address);
            assert_eq!(
                core::array::from_fn::<_, N, _>(|i| parent.snapshot(KINDS)[i].used()),
                parent_used
            );
            let mut cx = Context::from_waker(std::task::Waker::noop());
            for _ in 0..10000 {
                if matches!(
                    runtime.shutdown(&mut cx, &mut work(4096)),
                    StartStatus::Ready(_)
                ) {
                    break;
                }
            }
            drop(runtime);
            drop(observer);
            assert!(parent.snapshot(KINDS).iter().all(|a| a.used() == 0));
        }
    }
}
