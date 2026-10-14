use super::*;

type ReportStorage = (
    Cell<(Option<CleanupOutcome>, [u64; 4])>,
    Cell<u8>,
    RefCell<ResourceAccount>,
);
struct Report<'a> {
    storage: &'a ReportStorage,
    parent: StaticParent<'a>,
    bytes: u64,
}
impl Report<'_> {
    fn write(&self, outcome: CleanupOutcome) {
        let old = self.storage.0.get();
        if !old.0.is_some_and(|v| v.is_residual()) {
            self.storage.0.set((Some(outcome), old.1));
        }
    }
}
impl Drop for Report<'_> {
    fn drop(&mut self) {
        let refs = self
            .storage
            .1
            .get()
            .checked_sub(1)
            .expect("fixed report owners");
        self.storage.1.set(refs);
        if refs == 0 {
            assert!(self.storage.2.borrow_mut().release_committed(self.bytes));
            self.parent.edit(|a| {
                assert!(a[ENGINE].release_committed(self.bytes));
                assert!(a[CLEANUP_BYTES].release_committed(self.bytes));
                assert!(a[CLEANUP_ITEMS].release_committed(1));
            });
        }
    }
}

/// Caller-owned typed root. All runtime work is driven by short mutable
/// borrows. `C::RequestState` stays inside the actual Core slot until terminal
/// acknowledgement or recorded residual followed by destruction of that slot.
pub struct StaticConsumerPropertyReadRuntime<'a, C: PollClientBinding>
where
    C::Compiler: 'static,
    Artifact<C>: 'static,
{
    policy: ConsumerPropertyReadPolicy,
    registration: Option<StaticRegistration<C>>,
    clock: &'a dyn RuntimeClock,
    parent: StaticParent<'a>,
    report: Report<'a>,
    kernel: Kernel,
    storage: Option<BuildStorage<Artifact<C>, StaticParent<'a>>>,
    slot: Option<StaticConsumerPropertyReadSlot<C::RequestState>>,
    result: Option<CoreResult<InteractionOutput>>,
    call_charge: Option<Charge<StaticParent<'a>>>,
    provision: Option<Charge<StaticParent<'a>>>,
    nonce: u64,
    terminal_work: WorkBudget,
}
impl<'a, C: PollClientBinding> StaticConsumerPropertyReadRuntime<'a, C>
where
    C::Compiler: 'static,
    Artifact<C>: 'static,
{
    /// `report_storage` is exclusively lent caller storage; the returned
    /// observer can outlive the Runtime. The report's original account remains
    /// charged until the last of those two owners ends its loan. Its account
    /// must be empty, of kind CleanupBytesMax, and large enough for the tuple.
    #[allow(clippy::type_complexity)] // Original owned inputs plus the independently retained report.
    pub fn try_new(
        registration: StaticRegistration<C>,
        policy: ConsumerPropertyReadPolicy,
        clock: &'a dyn RuntimeClock,
        parent: &'a RefCell<[ResourceAccount; N]>,
        report_storage: &'a mut ReportStorage,
    ) -> Result<
        (Self, impl Fn() -> (Option<CleanupOutcome>, [u64; 4]) + 'a),
        BindingInputRejection<(StaticRegistration<C>, ConsumerPropertyReadPolicy)>,
    > {
        let supported = policy.qualify(
            &registration,
            registration.registration().resources(),
            false,
        );
        if let Err(e) = supported {
            return Err(reject((registration, policy), e));
        }
        // The complete Core slot is resident even while vacant. Add only the
        // author's indirect/raw backing beyond its inline associated state.
        let slot_bytes = (|| {
            let state = size_of::<C::RequestState>() as u64;
            if state > policy.call.retained_bytes() {
                return Err(unsupported());
            }
            let bytes = add(
                size_of::<StaticConsumerPropertyReadSlot<C::RequestState>>() as u64,
                policy.call.retained_bytes() - state,
            )?;
            policy.check_binding_storage(false, bytes)?;
            check(
                &policy.limits,
                K::CleanupItemBytesMax,
                size_of::<ReportStorage>() as u64,
            )?;
            Ok(bytes)
        })();
        let slot_bytes = match slot_bytes {
            Ok(bytes) => bytes,
            Err(error) => return Err(reject((registration, policy), error)),
        };
        let now = match policy.startup_time(clock, false) {
            Ok(now) => now,
            Err(error) => return Err(reject((registration, policy), error)),
        };
        let bytes = size_of::<ReportStorage>() as u64;
        if now
            .checked_add_ticks(policy.timeouts[0].max(policy.timeouts[1]))
            .is_none()
            || report_storage.0.get().0.is_some()
            || report_storage.1.get() != 0
            || report_storage.2.borrow().used() != 0
            || report_storage.2.borrow().kind() != K::CleanupBytesMax
            || report_storage.2.borrow().limit() < bytes
        {
            return Err(reject((registration, policy), unsupported()));
        }
        let parent = StaticParent(parent);
        let provisioning = (|| {
            let mut provision = Charge::new(parent, &policy)?;
            let root_bytes = add(
                size_of::<Self>() as u64,
                policy.registration.retained_bytes(),
            )?;
            check(
                &policy.limits,
                K::PeakLiveBytesPerAdmissionMax,
                add(
                    add(root_bytes, bytes)?,
                    add(
                        policy.scratch,
                        size_of::<BuildStorage<Artifact<C>, StaticParent<'a>>>() as u64,
                    )?,
                )?,
            )?;
            provision.grow(amounts(&[
                (ENGINE, root_bytes),
                (BINDINGS, 1),
                (SLOT_BYTES, slot_bytes),
            ]))?;
            let held = parent.edit(|a| {
                let wanted =
                    amounts(&[(ENGINE, bytes), (CLEANUP_BYTES, bytes), (CLEANUP_ITEMS, 1)]);
                for i in 0..N {
                    if let Some(r) = a[i].try_reserve(wanted[i]) {
                        r.commit();
                    } else {
                        for j in 0..i {
                            assert!(a[j].release_committed(wanted[j]));
                        }
                        return false;
                    }
                }
                true
            });
            if !held {
                return Err(busy());
            }
            report_storage
                .2
                .borrow_mut()
                .try_reserve(bytes)
                .expect("report loan checked")
                .commit();
            report_storage.1.set(2);
            report_storage.0.set((None, [0; 4]));
            Ok(provision)
        })();
        let provision = match provisioning {
            Ok(p) => p,
            Err(e) => return Err(reject((registration, policy), e)),
        };
        let report = Report {
            storage: report_storage,
            parent,
            bytes,
        };
        let observer = Report {
            storage: report_storage,
            parent,
            bytes,
        };
        let fixed = policy.terminal_fixed_work().expect("checked startup");
        Ok((
            Self {
                policy,
                registration: Some(registration),
                clock,
                parent,
                report,
                kernel: Kernel::new(now),
                storage: None,
                slot: Some(StaticConsumerPropertyReadSlot::new()),
                result: None,
                call_charge: None,
                provision: Some(provision),
                nonce: 0,
                terminal_work: WorkBudget::new().with_remaining(W::CleanupItems, fixed),
            },
            move || {
                let _keep = &observer;
                observer.storage.0.get()
            },
        ))
    }
    pub fn consume<'td>(
        &mut self,
        thing: &'td Thing,
        final_work: &mut WorkBudget,
        drive: impl for<'env> FnOnce(&mut StaticConsumerPropertyReadBuild<'td, 'env, 'a, C>),
    ) -> CoreResult<StartStatus<()>> {
        self.kernel.begin()?;
        self.nonce = self.kernel.nonce;
        let upstream = add(
            self.provision.as_ref().unwrap().held[ENGINE],
            self.report.bytes,
        )?;
        let storage = match BuildStorage::new(self.parent, &self.policy, upstream) {
            Ok(s) => s,
            Err(e) => {
                self.kernel.phase = CompiledPlanSetState::Reclaimed;
                self.kernel.build_error = Some(e.clone());
                return Err(e);
            }
        };
        self.storage = Some(storage);
        // Scope Drop handles unwinding after the input-bearing continuation has
        // returned its original root. It cannot create a self-reference.
        let mut scope = Scope {
            runtime: self,
            completed: false,
        };
        {
            let runtime = &mut *scope.runtime;
            let storage = runtime.storage.as_mut().unwrap();
            let mut adapter = StaticConsumerPropertyReadBuild {
                build: Build {
                    state: Building::Td(
                        ValidatedThingCursor::from_thing(
                            thing,
                            &runtime.policy.td,
                            runtime.policy.ledger(runtime.kernel.slot, 0),
                        ),
                        storage.root.as_mut().unwrap(),
                    ),
                    registration: runtime.registration.as_ref().unwrap(),
                    policy: &runtime.policy,
                    generation: runtime.kernel.plan,
                    upstream,
                    charge: storage.charge.as_mut().unwrap(),
                    terminal: &mut storage.terminal,
                    census: &mut storage.census,
                    draft: &mut storage.draft,
                    rollback: &mut storage.rollback,
                    cause: &mut storage.cause,
                    td_cause: &mut storage.td_cause,
                },
                cancelled: false,
                error: &mut runtime.kernel.build_error,
            };
            drive(&mut adapter);
            let cancelled = adapter.cancelled;
            drop(adapter);
            if cancelled {
                scope.runtime.kernel.first.get_or_insert(Cause::Manual);
            }
        }
        scope.finish();
        let runtime = &mut *scope.runtime;
        let error = runtime.kernel.build_error.clone();
        if let Some(e) = error {
            return Err(e);
        }
        runtime.install(final_work)?;
        Ok(if runtime.kernel.phase == CompiledPlanSetState::Published {
            StartStatus::Ready(())
        } else {
            StartStatus::Pending
        })
    }
    fn install(&mut self, work: &mut WorkBudget) -> CoreResult<()> {
        if self.kernel.phase != CompiledPlanSetState::Frozen {
            return Ok(());
        }
        if !pay(work, &[(W::PlanningItems, FINAL_ITEMS)]) {
            return Ok(());
        }
        let storage = self.storage.as_ref().ok_or_else(invariant)?;
        let draft = storage.draft.as_ref().ok_or_else(invariant)?;
        let identity = draft.registration_identity();
        let checked = self.kernel.final_checks(
            self.nonce,
            self.kernel.slot,
            self.kernel.plan,
            identity,
            self.registration
                .as_ref()
                .unwrap()
                .registration()
                .identity(),
            storage.charge.as_ref().is_some_and(|c| {
                c.is_held() && c.held[OUTPUT] >= storage.census.unwrap().output_capacity_bytes
            }) && self.provision.as_ref().is_some_and(Charge::is_held),
        );
        if let Err(error) = checked {
            self.kernel.build_error.get_or_insert(error.clone());
            self.kernel
                .close_generation(Cause::Drain, self.policy.timeouts[1]);
            return Err(error);
        }
        let _permit = InstallPermit;
        self.kernel.phase = CompiledPlanSetState::Published;
        Ok(())
    }
    pub fn state(&self) -> CompiledPlanSetState {
        self.kernel.phase
    }
    pub fn td_diagnostic(&self) -> Option<ValidatedThingCause> {
        self.kernel.td_cause
    }
    pub fn build_failure(&self) -> Option<CoreError> {
        self.kernel.build_error.clone()
    }
    pub fn generations(&self) -> (Generation, PlanSetGeneration, Generation) {
        (self.kernel.slot, self.kernel.plan, self.kernel.call)
    }
    pub fn next_deadline(&self) -> Option<Deadline> {
        self.kernel.next_deadline()
    }
    pub fn cleanup_record(&self) -> Option<CleanupOutcome> {
        self.report.storage.0.get().0
    }
    pub fn read_property(
        &mut self,
        name: &str,
        mut options: InteractionOptions,
        work: &mut WorkBudget,
    ) -> Result<Generation, BindingInputRejection<(InteractionOptions, Option<OutboundRequest>)>>
    {
        let storage = match self.storage.as_ref() {
            Some(s) if s.draft.is_some() => s,
            _ => return Err(reject((options, None), busy())),
        };
        if let Err(e) = self
            .policy
            .validate_input(name, &options, work, storage.census.unwrap())
        {
            return Err(reject((options, None), e));
        }
        let now = self.clock.now();
        if now.checked_duration_since(self.kernel.last_now).is_none() {
            return Err(reject((options, None), invalid()));
        }
        let deadline = match self.policy.deadline(now, self.clock, &options) {
            Ok(d) => d,
            Err(e) => return Err(reject((options, None), e)),
        };
        self.kernel.expire(now, self.policy.timeouts[1]);
        let generation = match self.kernel.start_call(self.kernel.slot, deadline) {
            Ok(g) => g,
            Err(e) => return Err(reject((options, None), e)),
        };
        let mut charge = match Charge::new(self.parent, &self.policy) {
            Ok(c) => c,
            Err(e) => {
                self.kernel.acknowledge();
                return Err(reject((options, None), e));
            }
        };
        let calls = u64::from(
            self.policy
                .limits
                .get(K::PendingClientCallsGlobalMax)
                .is_some(),
        );
        let temporary = self
            .policy
            .call_temporary(
                self.registration
                    .as_ref()
                    .unwrap()
                    .registration()
                    .resources(),
            )
            .expect("qualified callback storage");
        let peak = match [
            self.provision.as_ref().unwrap().held[ENGINE],
            self.report.bytes,
            storage.charge.as_ref().unwrap().held[ENGINE],
            self.policy.call_bytes().expect("checked policy"),
            temporary,
        ]
        .into_iter()
        .try_fold(0, add)
        {
            Ok(peak) => peak,
            Err(error) => {
                self.kernel.acknowledge();
                return Err(reject((options, None), error));
            }
        };
        if let Err(error) = check(&self.policy.limits, K::PeakLiveBytesPerAdmissionMax, peak) {
            self.kernel.acknowledge();
            return Err(reject((options, None), error));
        }
        if let Err(e) = charge.grow(amounts(&[
            (
                ENGINE,
                self.policy.call_bytes().expect("checked policy") + temporary,
            ),
            (TEMP, temporary),
            (PEAK, temporary),
            (PINS, 1),
            (CALLS, calls),
        ])) {
            self.kernel.acknowledge();
            return Err(reject((options, None), e));
        }
        let selected = match select_consumer_property_read_aggregate(
            storage.draft.as_ref().unwrap(),
            name,
            &options,
        ) {
            Ok(s) => s,
            Err(e) => {
                self.kernel.acknowledge();
                return Err(reject((options, None), e));
            }
        };
        let request = OutboundRequest::property_read(
            selected.artifact_ref(),
            core::mem::take(&mut options.uri_variables),
            Some(deadline),
        )
        .expect("selected Consumer identity");
        let mut callback = WorkBudget::new().with_remaining(W::BindingPolls, self.policy.work[0]);
        let started = with_registration(&mut self.registration, |r| {
            r.start_consumer_property_read(
                request,
                selected.artifact(),
                self.slot.as_mut().unwrap(),
                &mut callback,
            )
        });
        match started {
            Err(rejection) => {
                self.kernel.acknowledge();
                let (request, error) = rejection.into_parts();
                Err(BindingInputRejection::new((options, Some(request)), error))
            }
            Ok(started) => {
                charge.release(amounts(&[
                    (ENGINE, temporary),
                    (TEMP, temporary),
                    (PEAK, temporary),
                ]));
                self.call_charge = Some(charge);
                match started {
                    StartStatus::Pending => self.kernel.call_phase = CallPhase::Running,
                    StartStatus::Ready(result) => {
                        self.result = Some(result);
                        self.kernel.settle();
                    }
                }
                Ok(generation)
            }
        }
    }
    pub fn cancel(&mut self, generation: Generation) -> CoreResult<()> {
        if generation != self.kernel.call || self.kernel.call_phase == CallPhase::Vacant {
            return Err(stale());
        }
        self.kernel.cancel(Cause::Manual, self.policy.timeouts[1]);
        Ok(())
    }
    pub fn abandon_call(&mut self, generation: Generation) -> CoreResult<()> {
        if generation != self.kernel.call || self.kernel.call_phase == CallPhase::Vacant {
            return Err(stale());
        }
        self.kernel.interested = false;
        self.kernel.cancel(Cause::Drop, self.policy.timeouts[1]);
        Ok(())
    }
    pub fn close(&mut self) {
        self.kernel
            .close_generation(Cause::Drain, self.policy.timeouts[1]);
    }
    pub fn try_take(
        &mut self,
        generation: Generation,
        work: &mut WorkBudget,
    ) -> CoreResult<Option<CoreResult<InteractionOutput>>> {
        let cleanup = CleanupStep::new(work, self.policy.limit(K::CleanupWorkItemsPerStepMax));
        let work = &mut *cleanup.work;
        if generation != self.kernel.call || self.kernel.call_phase == CallPhase::Vacant {
            return Err(stale());
        }
        if self.kernel.call_phase != CallPhase::Result {
            return Ok(None);
        }
        if !pay(
            work,
            &[(
                W::CleanupItems,
                add(self.policy.work[5], self.policy.work[6])?,
            )],
        ) {
            return Ok(None);
        }
        self.acknowledge()?;
        let result = self.result.take();
        drop(self.call_charge.take());
        self.kernel.acknowledge();
        Ok(result)
    }
    fn acknowledge(&mut self) -> CoreResult<()> {
        if self.slot.as_ref().unwrap().is_vacant() && self.kernel.residual {
            return Ok(());
        }
        with_registration(&mut self.registration, |r| {
            r.acknowledge_consumer_property_read(self.slot.as_mut().unwrap())
        })
    }
    fn residual(&mut self) {
        self.kernel.residual = true;
        self.report.write(CleanupOutcome::ResidualExternalState(
            self.kernel.record(480),
        ));
        // Core's Cancelling slot is never cleared by a fabricated transfer or
        // callback error. Only the recorded residual permits complete local
        // slot destruction. A fresh vacant slot is a new physical occupant.
        drop(self.slot.take());
        self.slot = Some(StaticConsumerPropertyReadSlot::new());
        self.result.get_or_insert(Err(self
            .kernel
            .call_first
            .unwrap_or(Cause::Deadline)
            .error()));
        self.kernel.settle();
    }
    fn settlement(
        &mut self,
        settlement: CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>,
    ) {
        match settlement {
            Ok(BindingCallSettlement::Returned(result)) => {
                self.result = Some(result);
                self.kernel.settle();
            }
            Ok(BindingCallSettlement::Cancelled {
                disposition: BindingCancellationDisposition::Complete { .. },
                ..
            }) => {
                self.result = Some(Err(self.kernel.call_first.unwrap_or(Cause::Manual).error()));
                self.kernel.settle();
            }
            Ok(BindingCallSettlement::Cancelled {
                disposition: BindingCancellationDisposition::ResidualExternalState { record, .. },
                ..
            }) => {
                self.kernel.residual = true;
                self.report
                    .write(CleanupOutcome::ResidualExternalState(record));
                self.result = Some(Err(self.kernel.call_first.unwrap_or(Cause::Manual).error()));
                self.kernel.settle();
            }
            // Core rejects static client-fabricated TransferRequired. Retain
            // the Cancelling slot until the independent drain deadline.
            Err(error) => {
                self.result = Some(Err(error));
                self.kernel.call_phase = CallPhase::AwaitingResidual;
            }
            Ok(BindingCallSettlement::Cancelled { .. }) => {}
        }
    }
    pub fn step(&mut self, cx: &mut Context<'_>, work: &mut WorkBudget) -> StepStatus<()> {
        let cleanup = CleanupStep::new(work, self.policy.limit(K::CleanupWorkItemsPerStepMax));
        let work = &mut *cleanup.work;
        if self.kernel.phase == CompiledPlanSetState::Frozen {
            let _ = self.install(work);
            return StepStatus::progress(None, Some(PendingWork::new(PendingWorkClass::Cleanup)));
        }
        self.kernel
            .expire(self.clock.now(), self.policy.timeouts[1]);
        let phase = self.kernel.call_phase;
        if matches!(
            phase,
            CallPhase::Running | CallPhase::Cancelling | CallPhase::AwaitingResidual
        ) {
            if self.kernel.cancel_expired() {
                if pay(work, &[(W::CleanupItems, self.policy.work[5])]) {
                    self.residual();
                }
            } else if self.kernel.call_first.is_some() && phase != CallPhase::AwaitingResidual {
                let cost = if phase == CallPhase::Running {
                    self.policy.work[4]
                } else {
                    self.policy.work[3]
                };
                if pay(work, &[(W::CleanupItems, 1), (W::BindingPolls, cost)]) {
                    let mut callback = WorkBudget::new().with_remaining(W::BindingPolls, cost);
                    if phase == CallPhase::Running {
                        let cleanup = self.kernel.cleanup_phase(&self.policy, false);
                        self.kernel.call_phase = CallPhase::Cancelling;
                        let result = with_registration(&mut self.registration, |r| {
                            r.start_cancel_consumer_property_read(
                                cx,
                                cleanup,
                                self.slot.as_mut().unwrap(),
                                &mut callback,
                            )
                        });
                        match result {
                            Ok(StartStatus::Ready(s)) => self.settlement(Ok(s)),
                            Ok(StartStatus::Pending) => {}
                            Err(e) => self.settlement(Err(e)),
                        }
                    } else {
                        let result = with_registration(&mut self.registration, |r| {
                            r.poll_cancel_consumer_property_read(
                                cx,
                                self.slot.as_mut().unwrap(),
                                &mut callback,
                            )
                        });
                        if let Poll::Ready(s) = result {
                            self.settlement(s);
                        }
                    }
                }
            } else if self.kernel.call_first.is_none()
                && phase == CallPhase::Running
                && pay(work, &[(W::BindingPolls, self.policy.work[3])])
            {
                let mut callback =
                    WorkBudget::new().with_remaining(W::BindingPolls, self.policy.work[3]);
                let result = with_registration(&mut self.registration, |r| {
                    r.poll_consumer_property_read(cx, self.slot.as_mut().unwrap(), &mut callback)
                });
                if let Poll::Ready(result) = result {
                    self.result = Some(result);
                    self.kernel.settle();
                }
            }
        }
        if self.kernel.call_phase == CallPhase::Result
            && (!self.kernel.interested || self.kernel.discard_results)
            && pay(
                work,
                &[(W::CleanupItems, self.policy.work[5] + self.policy.work[6])],
            )
        {
            let acknowledged =
                self.slot.as_ref().unwrap().is_vacant() || self.acknowledge().is_ok();
            if !acknowledged && self.kernel.cancel_expired() {
                self.residual();
            }
            if acknowledged || self.slot.as_ref().unwrap().is_vacant() {
                drop(self.result.take());
                drop(self.call_charge.take());
                self.kernel.acknowledge();
            }
        }
        if self.kernel.phase == CompiledPlanSetState::Draining
            && self.kernel.call_phase == CallPhase::Vacant
        {
            if let Some(storage) = self.storage.as_mut() {
                if let Some(draft) = storage.draft.take() {
                    storage.rollback = Some(draft.into_rollback());
                }
                if storage.rollback.is_none() {
                    storage.rollback = storage.root.as_mut().and_then(PlanRoot::take);
                }
                if let Some(rollback) = storage.rollback.take() {
                    match rollback.step(work) {
                        ReclaimStep::Pending(rollback) => storage.rollback = Some(rollback),
                        ReclaimStep::Complete(_) => {
                            drop(storage.root.take());
                            drop(self.storage.take());
                            self.kernel.phase = CompiledPlanSetState::Reclaimed;
                        }
                    }
                } else {
                    drop(self.storage.take());
                    self.kernel.phase = CompiledPlanSetState::Reclaimed;
                }
            } else {
                self.kernel.phase = CompiledPlanSetState::Reclaimed;
            }
        }
        if self.kernel.phase == CompiledPlanSetState::Reclaimed {
            StepStatus::progress(Some(()), None)
        } else {
            StepStatus::progress(None, Some(PendingWork::new(PendingWorkClass::Cleanup)))
        }
    }
    pub fn shutdown(
        &mut self,
        cx: &mut Context<'_>,
        work: &mut WorkBudget,
    ) -> StartStatus<CleanupOutcome> {
        let cleanup = CleanupStep::new(work, self.policy.limit(K::CleanupWorkItemsPerStepMax));
        let work = &mut *cleanup.work;
        self.kernel.closed = true;
        self.kernel.discard_results = true;
        self.close();
        self.step(cx, work);
        if self.kernel.phase != CompiledPlanSetState::Reclaimed
            || self.kernel.call_phase != CallPhase::Vacant
        {
            return StartStatus::Pending;
        }
        if self.registration.is_some() {
            let cost = self
                .policy
                .registration_cleanup_work()
                .expect("qualified cleanup quantum");
            if !pay(work, &[(W::CleanupItems, cost)]) {
                return StartStatus::Pending;
            }
            drop(self.registration.take());
            self.provision.as_mut().unwrap().release(amounts(&[
                (ENGINE, self.policy.registration.retained_bytes()),
                (BINDINGS, 1),
            ]));
        }
        self.kernel.shutdown = true;
        let outcome = if self.kernel.residual {
            CleanupOutcome::ResidualExternalState(self.kernel.record(480))
        } else {
            CleanupOutcome::Complete
        };
        self.report.write(outcome);
        StartStatus::Ready(outcome)
    }
}
impl<C: PollClientBinding> Drop for StaticConsumerPropertyReadRuntime<'_, C>
where
    C::Compiler: 'static,
    Artifact<C>: 'static,
{
    fn drop(&mut self) {
        if !self.kernel.shutdown
            || self.kernel.phase != CompiledPlanSetState::Reclaimed
            || self.kernel.call_phase != CallPhase::Vacant
        {
            self.report.write(CleanupOutcome::ResidualExternalState(
                self.kernel.record(499),
            ));
        }
        self.terminal_work
            .consume(
                W::CleanupItems,
                self.policy.terminal_fixed_work().expect("checked startup"),
            )
            .expect("one prepaid terminal root allowance");
        drop(self.result.take());
        drop(self.slot.take());
        drop(self.call_charge.take());
        if let Some(mut storage) = self.storage.take() {
            storage.local_discard();
            let old = self.report.storage.0.get();
            self.report.storage.0.set((
                old.0,
                [
                    storage.terminal.used_steps,
                    storage.terminal.steps,
                    storage.terminal.work.remaining(W::CodecInputBytes),
                    storage.terminal.work.remaining(W::PlanningItems),
                ],
            ));
        }
        drop(self.registration.take());
        drop(self.provision.take());
    }
}
struct Scope<'r, 'a, C: PollClientBinding>
where
    C::Compiler: 'static,
    Artifact<C>: 'static,
{
    runtime: &'r mut StaticConsumerPropertyReadRuntime<'a, C>,
    completed: bool,
}
impl<C: PollClientBinding> Scope<'_, '_, C>
where
    C::Compiler: 'static,
    Artifact<C>: 'static,
{
    fn finish(&mut self) {
        let storage = self.runtime.storage.as_mut().unwrap();
        storage.end_scratch();
        self.runtime.kernel.td_cause = storage.td_cause;
        if let Some(error) = storage.cause.clone() {
            self.runtime.kernel.build_error.get_or_insert(error);
        }
        if storage.draft.is_some() && storage.cause.is_none() && self.runtime.kernel.first.is_none()
        {
            self.runtime.kernel.phase = CompiledPlanSetState::Frozen;
        } else {
            self.runtime
                .kernel
                .close_generation(Cause::Drop, self.runtime.policy.timeouts[1]);
        }
        self.completed = true;
    }
}
impl<C: PollClientBinding> Drop for Scope<'_, '_, C>
where
    C::Compiler: 'static,
    Artifact<C>: 'static,
{
    fn drop(&mut self) {
        if !self.completed {
            self.finish();
        }
    }
}
struct InstallPermit;

// ConsumerCompilerRegistration intentionally exposes no installed-client
// mutation. Move the unchanged complete registration into execution, then
// recapture its closed compiler support. Capture allocates and calls no client.
// RAII preserves the owner even if a caller's progress adapter unwinds.
fn with_registration<C: PollClientBinding, T>(
    destination: &mut Option<StaticRegistration<C>>,
    f: impl FnOnce(&mut StaticConsumerBindingRegistration<C>) -> T,
) -> T
where
    C::Compiler: 'static,
{
    let raw = destination.take().unwrap().into_registration();
    let mut execution = Execution {
        destination,
        raw: Some(raw),
    };
    f(execution.raw.as_mut().unwrap())
}
struct Execution<'a, C: PollClientBinding>
where
    C::Compiler: 'static,
{
    destination: &'a mut Option<StaticRegistration<C>>,
    raw: Option<StaticConsumerBindingRegistration<C>>,
}
impl<C: PollClientBinding> Drop for Execution<'_, C>
where
    C::Compiler: 'static,
{
    fn drop(&mut self) {
        *self.destination = Some(
            self.raw
                .take()
                .unwrap()
                .try_into_consumer_compiler()
                .unwrap_or_else(|_| panic!("unchanged closed compiler registration")),
        );
    }
}
