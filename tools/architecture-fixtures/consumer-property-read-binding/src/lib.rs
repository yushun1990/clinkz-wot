//! Finite client-only author used by the Runtime completion evidence.
//!
//! All variable results are backed by the application before installation.
//! Invocation only takes one fixed result, inspects the actual compiled target,
//! and (Host) attempts one recoverable concrete call allocation. No payload,
//! String, map or shared-stat allocation occurs after raw acceptance. Each
//! callback consumes one poll unit, inspects fixed state, and settles at most
//! once. Drop releases one bounded request, one prebacked result and one stats
//! reference. Larger/overcapacity inputs are rejected by Runtime before invoke.
#![no_std]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;
use alloc::sync::Arc;
use clinkz_wot_core::*;
use clinkz_wot_foundation::{ClockId, MonotonicInstant, RuntimeClock, WorkBudget, WorkClass};
#[cfg(feature = "std")]
use core::pin::Pin;
use core::{
    num::NonZeroU64,
    sync::atomic::{AtomicU8, AtomicU32, AtomicUsize, Ordering},
    task::{Context, Poll},
};

pub mod native;
pub mod ownership;
use ownership::{OwnedCall, OwnedPolicy};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Mode {
    Normal,
    Error,
    InvalidIdentity,
    InvalidShape,
    Pending,
    LateStart,
    LatePoll,
    Complete,
    Transfer,
    Residual,
    Reject,
    Overrun,
    AckFailure,
    CancelError,
}
impl Mode {
    fn from(n: u8) -> Self {
        match n {
            1 => Self::Error,
            2 => Self::InvalidIdentity,
            3 => Self::InvalidShape,
            4 => Self::Pending,
            5 => Self::LateStart,
            6 => Self::LatePoll,
            7 => Self::Complete,
            8 => Self::Transfer,
            9 => Self::Residual,
            10 => Self::Reject,
            11 => Self::Overrun,
            12 => Self::AckFailure,
            13 => Self::CancelError,
            _ => Self::Normal,
        }
    }
}
pub struct Stats {
    pub compiler_polls: AtomicUsize,
    pub compilations: AtomicUsize,
    pub compiler_aborts: AtomicUsize,
    pub starts: AtomicUsize,
    pub polls: AtomicUsize,
    pub cancels: AtomicUsize,
    pub acknowledgements: AtomicUsize,
    pub states_destroyed: AtomicUsize,
    pub clients_destroyed: AtomicUsize,
    pub mode: AtomicU8,
    pub validation: AtomicU8,
    pub target: [AtomicU8; 256],
    pub target_len: AtomicUsize,
    #[cfg(feature = "std")]
    pub hook: std::sync::Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
}
impl Default for Stats {
    fn default() -> Self {
        Self {
            compiler_polls: AtomicUsize::new(0),
            compilations: AtomicUsize::new(0),
            compiler_aborts: AtomicUsize::new(0),
            starts: AtomicUsize::new(0),
            polls: AtomicUsize::new(0),
            cancels: AtomicUsize::new(0),
            acknowledgements: AtomicUsize::new(0),
            states_destroyed: AtomicUsize::new(0),
            clients_destroyed: AtomicUsize::new(0),
            mode: AtomicU8::new(0),
            validation: AtomicU8::new(0),
            target: core::array::from_fn(|_| AtomicU8::new(0)),
            target_len: AtomicUsize::new(0),
            #[cfg(feature = "std")]
            hook: std::sync::Mutex::new(None),
        }
    }
}
impl Stats {
    pub fn set_mode(&self, mode: Mode) {
        self.mode.store(mode as u8, Ordering::SeqCst);
    }
    fn mode(&self) -> Mode {
        Mode::from(self.mode.load(Ordering::SeqCst))
    }
    fn validation(&self, mode: Mode) -> Mode {
        match self.validation.load(Ordering::SeqCst) {
            1 => Mode::InvalidIdentity,
            2 => Mode::InvalidShape,
            3 => Mode::Error,
            _ => mode,
        }
    }
    fn hook(&self) {
        #[cfg(feature = "std")]
        {
            let hook = self.hook.lock().unwrap().as_ref().map(Arc::clone);
            if let Some(hook) = hook {
                hook();
            }
        }
    }
    fn selected(&self, target: &str) {
        assert!(target.len() <= self.target.len());
        for (to, from) in self.target.iter().zip(target.bytes()) {
            to.store(from, Ordering::SeqCst);
        }
        self.target_len.store(target.len(), Ordering::SeqCst);
        self.hook();
    }
}
pub struct Clock(pub AtomicU32);
impl Default for Clock {
    fn default() -> Self {
        Self(AtomicU32::new(10))
    }
}
impl RuntimeClock for Clock {
    fn now(&self) -> MonotonicInstant {
        MonotonicInstant::new(ClockId::new(23), u64::from(self.0.load(Ordering::SeqCst)))
    }
    fn ticks_per_second(&self) -> NonZeroU64 {
        NonZeroU64::new(1000).unwrap()
    }
}
fn error() -> CoreError {
    CoreError::Binding(
        ErrorContext::new(ErrorPhase::Binding, RetryClass::Never)
            .with_redacted_cause(451, "finite client error"),
    )
}
fn rejection(
    request: OutboundRequest,
    policy: Option<&OwnedPolicy>,
    domain: ResourceFailureDomain,
) -> BindingInputRejection<OutboundRequest> {
    let mut context = ErrorContext::new(ErrorPhase::Binding, RetryClass::Never);
    if let Some(policy) = policy {
        context = context
            .with_plan(request.plan_id())
            .with_binding(request.binding_id(), request.binding_generation())
            .with_resource_failure(policy.configuration().private_policy(), domain, None, None);
    }
    BindingInputRejection::new(
        request,
        BindingOperationalError::new(CoreError::Backpressure(context)),
    )
}
fn prepare_output(
    request: &OutboundRequest,
    output: InteractionOutput,
    mode: Mode,
) -> CoreResult<InteractionOutput> {
    if mode == Mode::Error {
        return Err(error());
    }
    let binding = if mode == Mode::InvalidIdentity {
        BindingId::new(999)
    } else {
        request.binding_id()
    };
    let output = if mode == Mode::InvalidShape {
        InteractionOutput::empty()
    } else {
        output
    };
    Ok(output
        .try_with_metadata(
            InteractionOutputMetadata::default().with_untrusted_binding_response(
                BindingResponseMetadata::primary(
                    binding,
                    request.binding_generation(),
                    request.plan_id(),
                    599,
                ),
            ),
        )
        .unwrap())
}
pub struct State {
    request: Option<OutboundRequest>,
    output: Option<CoreResult<InteractionOutput>>,
    stats: Arc<Stats>,
    mode: Mode,
    static_fabricated: bool,
    prepared: bool,
    validation: Mode,
    owned: Option<OwnedCall>,
}
impl State {
    fn prepare(&mut self) {
        if !self.prepared {
            let output = self.output.take().unwrap().unwrap();
            self.output = Some(prepare_output(
                self.request.as_ref().unwrap(),
                output,
                self.validation,
            ));
            self.prepared = true;
        }
    }
    fn poll(&mut self, budget: &mut WorkBudget) -> Poll<CoreResult<InteractionOutput>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        self.stats.hook();
        self.stats.polls.fetch_add(1, Ordering::SeqCst);
        if self.owned.as_mut().is_some_and(|p| !p.progress()) {
            return Poll::Pending;
        }
        self.prepare();
        if self.mode == Mode::Pending {
            return Poll::Pending;
        }
        Poll::Ready(self.output.take().expect("one output"))
    }
    fn cancel(
        &mut self,
        cleanup: CleanupPhaseContext,
        budget: &mut WorkBudget,
        host: bool,
    ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Ok(StartStatus::Pending);
        }
        self.stats.hook();
        self.stats.cancels.fetch_add(1, Ordering::SeqCst);
        self.prepare();
        match self.mode {
            Mode::CancelError => Err(error()),
            Mode::LateStart | Mode::InvalidIdentity | Mode::InvalidShape | Mode::Error => Ok(
                StartStatus::Ready(BindingCallSettlement::Returned(self.output.take().unwrap())),
            ),
            Mode::Complete => Ok(StartStatus::Ready(BindingCallSettlement::Cancelled {
                retry_class: RetryClass::Never,
                disposition: BindingCancellationDisposition::Complete {
                    successor: NoCleanupSuccessor,
                },
            })),
            Mode::Transfer => {
                let transfer = if host {
                    cleanup.try_into_transfer_request().unwrap()
                } else {
                    // Deliberately dishonest static request: Core must reject it.
                    let subject = cleanup.reservation().subject();
                    self.static_fabricated = true;
                    CleanupPhaseContext::bind_with_transfer_owner(
                        CleanupReservation::new(
                            subject,
                            BindingLifetimeFootprint::new(1, 1),
                            1,
                            WorkBudget::new(),
                        ),
                        subject,
                        CleanupOperation::CancelRequest,
                        error(),
                        Deadline::NONE,
                    )
                    .try_into_transfer_request()
                    .unwrap()
                };
                Ok(StartStatus::Ready(BindingCallSettlement::Cancelled {
                    retry_class: RetryClass::Never,
                    disposition: BindingCancellationDisposition::TransferRequired(transfer),
                }))
            }
            Mode::Residual => {
                let subject = cleanup.reservation().subject();
                let record = CleanupRecord::try_new(
                    CleanupHandle::new(subject),
                    subject,
                    subject,
                    CleanupOperation::CancelRequest,
                    0,
                    RetryClass::Never,
                    454,
                    0,
                )
                .unwrap();
                Ok(StartStatus::Ready(BindingCallSettlement::Cancelled {
                    retry_class: RetryClass::Never,
                    disposition: BindingCancellationDisposition::ResidualExternalState {
                        successor: NoCleanupSuccessor,
                        record,
                    },
                }))
            }
            _ => Ok(StartStatus::Pending),
        }
    }
    fn poll_cancel(
        &mut self,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        self.stats.hook();
        self.stats.cancels.fetch_add(1, Ordering::SeqCst);
        self.prepare();
        if self.mode == Mode::Pending || self.mode == Mode::Transfer && self.static_fabricated {
            return Poll::Pending;
        }
        Poll::Ready(Ok(BindingCallSettlement::Returned(
            self.output.take().unwrap(),
        )))
    }
}
impl Drop for State {
    fn drop(&mut self) {
        self.stats.states_destroyed.fetch_add(1, Ordering::SeqCst);
    }
}
pub struct StaticClient {
    pub stats: Arc<Stats>,
    outputs: [Option<InteractionOutput>; 8],
    next: usize,
    owned: Option<OwnedPolicy>,
}
impl StaticClient {
    pub fn new_owned(
        stats: Arc<Stats>,
        outputs: [Option<InteractionOutput>; 8],
        policy: OwnedPolicy,
    ) -> Self {
        let mut client = Self::new(stats, outputs);
        client.owned = Some(policy);
        client
    }

    pub fn new(stats: Arc<Stats>, outputs: [Option<InteractionOutput>; 8]) -> Self {
        Self {
            stats,
            outputs,
            next: 0,
            owned: None,
        }
    }
}
impl Drop for StaticClient {
    fn drop(&mut self) {
        self.stats.clients_destroyed.fetch_add(1, Ordering::SeqCst);
    }
}
impl StaticClient {
    fn start_target(
        &mut self,
        request: OutboundRequest,
        target: &str,
        slot: &mut ClientRequestSlot<State>,
        budget: &mut WorkBudget,
    ) -> Result<StartStatus<CoreResult<InteractionOutput>>, BindingInputRejection<OutboundRequest>>
    {
        let mode = self
            .owned
            .as_ref()
            .map_or_else(|| self.stats.mode(), |p| p.mode);
        if mode == Mode::Reject
            || self.next == self.outputs.len()
            || budget.consume(WorkClass::BindingPolls, 1).is_err()
        {
            return Err(rejection(
                request,
                self.owned.as_ref(),
                ResourceFailureDomain::BindingPolicy,
            ));
        }
        self.stats.selected(target);
        let output = prepare_output(
            &request,
            self.outputs[self.next].take().unwrap(),
            if self.owned.is_some() {
                mode
            } else {
                self.stats.validation(mode)
            },
        );
        self.next += 1;
        self.stats.starts.fetch_add(1, Ordering::SeqCst);
        slot.initialize(
            request,
            State {
                request: None,
                output: Some(output),
                stats: Arc::clone(&self.stats),
                mode,
                static_fabricated: false,
                prepared: true,
                validation: if self.owned.is_some() {
                    mode
                } else {
                    self.stats.validation(mode)
                },
                owned: self.owned.as_ref().map(|p| p.call()),
            },
        );
        Ok(StartStatus::Pending)
    }
}
impl PollClientBinding for StaticClient {
    type Compiler = ResolvedTargetCompiler;
    type RequestState = State;
    fn consumer_configuration(&self) -> Option<ConsumerBindingConfiguration> {
        self.owned.as_ref().map(|p| p.configuration())
    }
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        ResolvedTargetCompiler::try_new(64).unwrap().compatibility()
    }
    fn request_state_layout(&self) -> BindingStateLayout {
        BindingStateLayout::of::<State>(BindingLifetimeFootprint::new(4, 1024))
            .with_transient(BindingTransientFootprint::new(128))
    }
    fn start_request(
        &mut self,
        request: OutboundRequest,
        artifact: &BindingArtifactEnvelope<ResolvedTargetArtifact>,
        slot: &mut ClientRequestSlot<State>,
        budget: &mut WorkBudget,
    ) -> Result<StartStatus<CoreResult<InteractionOutput>>, BindingInputRejection<OutboundRequest>>
    {
        self.start_target(
            request,
            artifact.artifact().payload().target(),
            slot,
            budget,
        )
    }
    fn poll_request(
        &mut self,
        _: &mut Context<'_>,
        slot: &mut ClientRequestSlot<State>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<InteractionOutput>> {
        slot.state_mut().poll(budget)
    }
    fn start_cancel_request(
        &mut self,
        _: &mut Context<'_>,
        cleanup: CleanupPhaseContext,
        slot: &mut ClientRequestSlot<State>,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        slot.state_mut().cancel(cleanup, budget, false)
    }
    fn poll_cancel_request(
        &mut self,
        _: &mut Context<'_>,
        slot: &mut ClientRequestSlot<State>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        slot.state_mut().poll_cancel(budget)
    }
    fn acknowledge_request(&mut self, slot: &mut ClientRequestSlot<State>) -> CoreResult<()> {
        if slot.state_mut().mode == Mode::AckFailure {
            return Err(error());
        }
        self.stats.acknowledgements.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
pub fn resources() -> BindingResourceDeclarations {
    BindingResourceDeclarations::new(
        BindingLifetimeFootprint::new(0, 0),
        BindingLifetimeFootprint::new(16, 8192),
    )
    .with_state_footprints(
        BindingLifetimeFootprint::new(0, 0),
        BindingLifetimeFootprint::new(0, 0),
        BindingLifetimeFootprint::new(0, 0),
    )
    .with_transient(BindingTransientFootprint::new(128))
}
fn identity(c: &ResolvedTargetCompiler) -> BindingRegistrationIdentity {
    BindingRegistrationIdentity::new(
        BindingId::new(223),
        BindingGeneration::INITIAL.checked_next().unwrap(),
        c.configuration(),
        c.compatibility(),
        31,
    )
}
pub fn static_registration(
    client: StaticClient,
) -> ConsumerCompilerRegistration<StaticConsumerBindingRegistration<StaticClient>> {
    let owned = client.owned.as_ref().map(|p| p.configuration());
    let compiler = owned.map_or_else(
        || ResolvedTargetCompiler::try_new(64).unwrap(),
        ResolvedTargetCompiler::new_owned,
    );
    let resources = owned.map_or_else(resources, |c| {
        resources()
            .with_resource_contract(c.resource_contract())
            .with_consumer_result_limits(c.result_limits().0, c.result_limits().1)
    });
    StaticConsumerBindingRegistration::new(StaticConsumerBindingRegistrationInput::new(
        identity(&compiler),
        BindingExecutionSupport::application_static(),
        StaticBindingCompilerRegistration::new(compiler),
        client,
        resources,
        BindingStatusPolicy::new(2, 17),
    ))
    .unwrap_or_else(|_| panic!("static complete registration"))
    .try_into_consumer_compiler()
    .unwrap_or_else(|_| panic!("closed static support"))
}

#[cfg(feature = "std")]
pub struct HostClient {
    pub stats: Arc<Stats>,
    inbox: std::sync::Mutex<([Option<InteractionOutput>; 8], usize)>,
    owned: Option<OwnedPolicy>,
}
#[cfg(feature = "std")]
impl HostClient {
    pub fn new_owned(
        stats: Arc<Stats>,
        outputs: [Option<InteractionOutput>; 8],
        policy: OwnedPolicy,
    ) -> Self {
        let mut client = Self::new(stats, outputs);
        client.owned = Some(policy);
        client
    }

    pub fn new(stats: Arc<Stats>, outputs: [Option<InteractionOutput>; 8]) -> Self {
        Self {
            stats,
            inbox: std::sync::Mutex::new((outputs, 0)),
            owned: None,
        }
    }
}
#[cfg(feature = "std")]
impl Drop for HostClient {
    fn drop(&mut self) {
        self.stats.clients_destroyed.fetch_add(1, Ordering::SeqCst);
    }
}
#[cfg(feature = "std")]
impl HostClient {
    fn invoke_target(
        &self,
        request: OutboundRequest,
        target: &str,
    ) -> Result<
        HostBindingCallBox<CoreResult<InteractionOutput>>,
        BindingInputRejection<OutboundRequest>,
    > {
        let mode = self
            .owned
            .as_ref()
            .map_or_else(|| self.stats.mode(), |p| p.mode);
        if mode == Mode::Reject {
            return Err(rejection(
                request,
                self.owned.as_ref(),
                ResourceFailureDomain::BindingPolicy,
            ));
        }
        let mut inbox = match self.inbox.try_lock() {
            Ok(i) => i,
            Err(_) => {
                return Err(rejection(
                    request,
                    self.owned.as_ref(),
                    ResourceFailureDomain::BindingPolicy,
                ));
            }
        };
        let index = inbox.1;
        if index == inbox.0.len() {
            return Err(rejection(
                request,
                self.owned.as_ref(),
                ResourceFailureDomain::BindingPolicy,
            ));
        }
        self.stats.selected(target);
        let state = State {
            request: Some(request),
            output: Some(Ok(inbox.0[index].take().unwrap())),
            stats: Arc::clone(&self.stats),
            mode,
            static_fabricated: false,
            prepared: false,
            validation: if self.owned.is_some() {
                mode
            } else {
                self.stats.validation(mode)
            },
            owned: self.owned.as_ref().map(|p| p.call()),
        };
        let call = match HostBindingCallBox::try_new(state) {
            Ok(call) => call,
            Err(mut state) => {
                inbox.0[index] = Some(state.output.take().unwrap().unwrap());
                return Err(rejection(
                    state.request.take().unwrap(),
                    self.owned.as_ref(),
                    ResourceFailureDomain::DeploymentFailure,
                ));
            }
        };
        inbox.1 += 1;
        self.stats.starts.fetch_add(1, Ordering::SeqCst);
        Ok(call)
    }
}
#[cfg(feature = "std")]
impl binding::ClientBinding for HostClient {
    fn consumer_configuration(&self) -> Option<ConsumerBindingConfiguration> {
        self.owned.as_ref().map(|p| p.configuration())
    }
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        ResolvedTargetCompiler::try_new(64).unwrap().compatibility()
    }
    fn invoke(
        &self,
        request: OutboundRequest,
        artifact: &BindingArtifactEnvelope<HostBindingArtifact>,
    ) -> Result<
        HostBindingCallBox<CoreResult<InteractionOutput>>,
        BindingInputRejection<OutboundRequest>,
    > {
        self.invoke_target(
            request,
            artifact
                .artifact()
                .try_payload::<ResolvedTargetArtifact>(self.artifact_compatibility())
                .unwrap()
                .target(),
        )
    }
}
#[cfg(feature = "std")]
impl HostBindingCall<CoreResult<InteractionOutput>> for State {
    fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
        if self.mode == Mode::Overrun {
            BindingLifetimeFootprint::new(17, 8193)
        } else {
            BindingLifetimeFootprint::new(8, 2048)
        }
    }
    fn next_deadline(&self) -> Option<Deadline> {
        self.request.as_ref().unwrap().deadline()
    }
    fn poll_result(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<InteractionOutput>> {
        self.poll(budget)
    }
    fn start_cancel(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        cleanup: CleanupPhaseContext,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        self.cancel(cleanup, budget, true)
    }
    fn poll_cancel(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        State::poll_cancel(&mut self, budget)
    }
}
#[cfg(feature = "std")]
pub fn host_registration(
    client: HostClient,
) -> ConsumerCompilerRegistration<HostConsumerBindingRegistration> {
    let owned = client.owned.as_ref().map(|p| p.configuration());
    let compiler = owned.map_or_else(
        || ResolvedTargetCompiler::try_new(64).unwrap(),
        ResolvedTargetCompiler::new_owned,
    );
    let resources = owned.map_or_else(resources, |c| {
        resources()
            .with_resource_contract(c.resource_contract())
            .with_consumer_result_limits(c.result_limits().0, c.result_limits().1)
    });
    HostConsumerBindingRegistration::new(HostConsumerBindingRegistrationInput::new(
        identity(&compiler),
        BindingExecutionSupport::host_erased(),
        HostBindingCompilerRegistration::try_new_consumer(compiler).unwrap(),
        alloc::boxed::Box::new(client),
        resources,
        BindingStatusPolicy::new(2, 17),
    ))
    .unwrap_or_else(|_| panic!("Host complete registration"))
    .try_into_consumer_compiler()
    .unwrap_or_else(|_| panic!("closed Host support"))
}
