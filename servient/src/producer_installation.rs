//! Funded Host Producer installation, publication, dispatch and retirement.
//! The retained Binding owner borrows current acceptance permits and sealed
//! replies; the Servient owns application dispatch and all Engine charges.
use alloc::{boxed::Box, sync::Arc};
use clinkz_wot_core::{producer::*, *};
use clinkz_wot_foundation::{
    MonotonicInstant, ResourceKind as K, ResourceLimits, WorkBudget, WorkClass,
};
use clinkz_wot_planning::producer_execution::PreparedProducer;
use core::{
    alloc::Layout,
    mem::size_of,
    task::{Context, Poll},
};

pub const INSTALLATION_ACCOUNT_KINDS: [K; 19] = [
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
    K::PendingHandlerCallsGlobalMax,
    K::InFlightResponsesGlobalMax,
    K::BindingResponseBufferBytesGlobalMax,
    K::BindingIngressItemsGlobalMax,
    K::BindingIngressBytesGlobalMax,
    K::ThingsGlobalMax,
    K::BindingsGlobalMax,
];
pub use crate::host_accounts::HostResourceAccounts as ProducerInstallationAccounts;
pub fn producer_installation_accounts(limits: &ResourceLimits) -> ProducerInstallationAccounts {
    ProducerInstallationAccounts::new(limits)
}

struct Charge {
    leases: [Option<crate::host_accounts::HostCapacityReservation>; 19],
}
impl Charge {
    fn retain_status(&mut self, bytes: u64, status: u64) {
        let keep = [
            bytes, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, status, 0, 0, 0, 0, 0, 0, 0,
        ];
        for (lease, keep) in self.leases.iter_mut().zip(keep) {
            if let Some(lease) = lease {
                lease.release_part(lease.amount() - keep);
            }
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
    request: Option<ProducerRequest>,
    response: Option<ProducerResponse>,
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
    Serving,
    Dispatching,
    Delivering,
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
    timeout_ticks: u64,
    io_deadline: Deadline,
    last_now: Option<MonotonicInstant>,
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
        let admit = || -> CoreResult<[u64; 19]> {
            prepared.check_current()?;
            if prepared.entry().instance_key() != authority.key() {
                return Err(invalid());
            }
            let p = prepared.install_policy();
            let f = prepared.footprint();
            if f.retained_bytes() < size_of::<H>() as u64 || p.local_drop_work == 0 {
                return Err(invalid());
            }
            let property = Layout::new::<[usize; 2]>()
                .extend(
                    Layout::array::<u8>(prepared.entry().property().len())
                        .map_err(|_| invalid())?,
                )
                .map_err(|_| invalid())?
                .0
                .pad_to_align()
                .size() as u64;
            let service = prepared.service_policy();
            let response = service
                .map(|s| {
                    clinkz_wot_core::consumer_result_backing(
                        s.response_body_bytes,
                        s.response_media_capacity,
                    )
                    .ok_or_else(invalid)
                })
                .transpose()?
                .unwrap_or(0);
            let request = if service.is_some() {
                size_of::<ProducerRequest>() as u64
            } else {
                0
            };
            if let Some(service) = service {
                for (kind, amount) in [
                    (K::PendingHandlerCallsPerThingMax, 1),
                    (K::InFlightResponsesPerThingMax, 1),
                    (K::BindingIngressItemsPerRouteMax, 1),
                    (K::BindingIngressItemsPerBindingMax, 1),
                    (K::BindingIngressBytesPerRouteMax, request),
                    (K::BindingIngressBytesPerBindingMax, request),
                    (K::BindingResponseBufferBytesPerRouteMax, response),
                    (
                        K::LargestContiguousAllocationBytesMax,
                        Layout::new::<[usize; 2]>()
                            .extend(
                                Layout::array::<u8>(
                                    usize::try_from(service.response_body_bytes)
                                        .map_err(|_| invalid())?,
                                )
                                .map_err(|_| invalid())?,
                            )
                            .map_err(|_| invalid())?
                            .0
                            .pad_to_align()
                            .size() as u64,
                    ),
                    (
                        K::LargestContiguousAllocationBytesMax,
                        service.response_media_capacity,
                    ),
                ] {
                    check(&limits, kind, amount, quota_context)?;
                }
            }
            let extra = add(
                add(
                    add(p.executor_bytes, p.program_bytes)?,
                    p.installation_bytes,
                )?,
                add(property, f.retained_bytes() - size_of::<H>() as u64)?,
            )?;
            let extra = add(extra, add(response, f.pending_call_bytes())?)?;
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
                u64::from(service.is_some()),
                u64::from(service.is_some()),
                response,
                u64::from(service.is_some()),
                request,
                1,
                1,
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
            leases: core::array::from_fn(|_| None),
        };
        let reserve = (|| -> CoreResult<()> {
            for i in 0..19 {
                check(
                    &limits,
                    INSTALLATION_ACCOUNT_KINDS[i],
                    amounts[i],
                    quota_context,
                )?;
                let limit = parent.account(INSTALLATION_ACCOUNT_KINDS[i]).limit();
                let Some(lease) = parent.reserve(INSTALLATION_ACCOUNT_KINDS[i], amounts[i]) else {
                    return Err(CoreError::LimitExceeded {
                        context: quota_context,
                        resource: INSTALLATION_ACCOUNT_KINDS[i],
                        limit,
                        requested: Some(amounts[i]),
                        observed: None,
                    });
                };
                charge.leases[i] = Some(lease);
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
            request: None,
            response: None,
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
            timeout_ticks: 0,
            io_deadline: Deadline::NONE,
            last_now: None,
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
    /// Requests publication after native readiness. All source preparation has
    /// finished; stepping installs the driver and then publishes locally.
    pub fn begin_expose(&mut self, timeout_ticks: u64) -> CoreResult<()> {
        if !matches!(
            self.state,
            ProducerInstallationState::Reserved
                | ProducerInstallationState::Installing
                | ProducerInstallationState::InstalledClosed
        ) || timeout_ticks == 0
            || self
                .owner
                .as_ref()
                .unwrap()
                .execution
                .as_ref()
                .unwrap()
                .service_policy()
                .is_none()
        {
            return Err(invalid());
        }
        self.timeout_ticks = timeout_ticks;
        Ok(())
    }
    /// Atomically publishes local acceptance. No fallible Binding callback is
    /// performed here. The caller supplies a nonzero duration in its clock's
    /// ticks; the same clock must drive every subsequent step.
    pub fn begin_serving(&mut self, timeout_ticks: u64) -> CoreResult<()> {
        if self.state != ProducerInstallationState::InstalledClosed
            || timeout_ticks == 0
            || self
                .owner
                .as_ref()
                .unwrap()
                .execution
                .as_ref()
                .unwrap()
                .service_policy()
                .is_none()
        {
            return Err(invalid());
        }
        self.timeout_ticks = timeout_ticks;
        self.state = ProducerInstallationState::Serving;
        Ok(())
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
    /// One funded construction, native poll, handler call or close callback. No callbacks
    /// run with zero credit. Deadline/clock failure closes authority locally;
    /// first cause survives subsequent cleanup failures and cancellation.
    pub fn step(
        &mut self,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
        now: MonotonicInstant,
    ) -> Poll<ProducerInstallationState> {
        use ProducerInstallationState::*;
        if self.state != Closed {
            if self
                .last_now
                .is_some_and(|last| now.checked_duration_since(last).is_none())
            {
                self.fail(invalid());
            }
            self.last_now = Some(now);
        }
        if matches!(self.state, Serving | Dispatching | Delivering) {
            match self.io_deadline.checked_is_elapsed_at(now) {
                Some(false) => {}
                Some(true) => self.fail(CoreError::TimedOut(ErrorContext::new(
                    ErrorPhase::Binding,
                    RetryClass::Never,
                ))),
                None => self.fail(invalid()),
            }
        }
        if matches!(self.state, Reserved | Installing | InstalledClosed) {
            match self.deadline.checked_is_elapsed_at(now) {
                Some(false) => {}
                Some(true) => self.fail(CoreError::TimedOut(ErrorContext::new(
                    ErrorPhase::Binding,
                    RetryClass::Never,
                ))),
                None => self.fail(invalid()),
            }
        }
        if self.state == InstalledClosed && self.timeout_ticks != 0 {
            self.begin_serving(self.timeout_ticks)
                .expect("admitted publication");
        }
        if matches!(self.state, Closed | InstalledClosed) {
            return Poll::Ready(self.state);
        }
        let class = if self.state == Closing {
            WorkClass::CleanupItems
        } else if self.state == Dispatching {
            WorkClass::HandlerSteps
        } else {
            WorkClass::BindingPolls
        };
        if budget.consume(class, 1).is_err() {
            return Poll::Pending;
        }
        // Required services and their captured configuration must remain
        // available before lending an accept permit, entering behavior, or
        // delivering its result. Cleanup remains drivable without that facet.
        if matches!(self.state, Serving | Dispatching | Delivering)
            && let Err(error) = self
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
            Serving => {
                let owner = self.owner.as_mut().unwrap();
                let native = owner.installation.as_mut().unwrap();
                let result = native.poll_request(
                    cx,
                    self.authority.accept_permit(),
                    now,
                    self.timeout_ticks,
                );
                self.io_deadline = native.request_deadline();
                match result {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(Err(error)) => {
                        self.fail(error);
                        Poll::Ready(self.state)
                    }
                    Poll::Ready(Ok(request)) => {
                        if request.instance() != self.authority.key()
                            || request.correlation().is_empty()
                        {
                            owner.request = Some(request);
                            self.fail(CoreError::StaleHandle(context()));
                        } else {
                            owner.request = Some(request);
                            self.state = Dispatching;
                        }
                        Poll::Ready(self.state)
                    }
                }
            }
            Dispatching => {
                let owner = self.owner.as_mut().unwrap();
                let execution = owner.execution.as_ref().unwrap();
                let entry = execution.entry();
                let binding = entry.binding();
                let context = HandlerContext::for_producer(
                    self.authority.key(),
                    entry.target(),
                    Operation::ReadProperty,
                    (binding.binding_id(), binding.binding_generation()),
                )
                .expect("prepared Property Read target");
                let result = execution.dispatch(context);
                let service = execution.service_policy().unwrap();
                owner.response = Some(ProducerResponse::seal(
                    owner.request.take().unwrap(),
                    result,
                    service.response_body_bytes,
                    service.response_media_capacity,
                ));
                self.state = Delivering;
                Poll::Ready(self.state)
            }
            Delivering => {
                let owner = self.owner.as_mut().unwrap();
                match owner.installation.as_mut().unwrap().poll_response(
                    cx,
                    owner.response.as_ref().unwrap(),
                    now,
                ) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(Err(error)) => {
                        self.fail(error);
                        Poll::Ready(self.state)
                    }
                    Poll::Ready(Ok(())) => {
                        owner.response = None;
                        self.io_deadline = Deadline::NONE;
                        self.state = Serving;
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
