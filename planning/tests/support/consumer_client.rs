//! Actual downstream client-only authoring and complete Core registration.
//! This module has no Producer implementation or component.
#![allow(dead_code)]
extern crate alloc;
#[cfg(feature = "std")]
use alloc::boxed::Box;
use alloc::{sync::Arc, vec::Vec};
use clinkz_wot_core::*;
use clinkz_wot_foundation::{WorkBudget, WorkClass};
use clinkz_wot_planning::{binding_compiler::*, binding_registration::*};
use core::{
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll},
};

#[derive(Default)]
pub struct Stats {
    pub starts: AtomicUsize,
    pub clients_destroyed: AtomicUsize,
    pub requests_destroyed: AtomicUsize,
}
pub struct Client {
    pub compatibility: BindingArtifactCompatibility,
    pub stats: Arc<Stats>,
}
impl Drop for Client {
    fn drop(&mut self) {
        self.stats.clients_destroyed.fetch_add(1, Ordering::SeqCst);
    }
}
pub struct State {
    output: Option<InteractionOutput>,
    stats: Arc<Stats>,
}
impl Drop for State {
    fn drop(&mut self) {
        self.stats.requests_destroyed.fetch_add(1, Ordering::SeqCst);
    }
}
fn output(request: &OutboundRequest, target: &str) -> InteractionOutput {
    InteractionOutput::with_data(Payload::new(
        Vec::from(target.as_bytes()),
        "application/json",
    ))
    .try_with_metadata(
        InteractionOutputMetadata::default().with_untrusted_binding_response(
            BindingResponseMetadata::primary(
                request.binding_id(),
                request.binding_generation(),
                request.plan_id(),
                599,
            ),
        ),
    )
    .unwrap()
}
fn error() -> BindingOperationalError {
    BindingOperationalError::new(CoreError::Backpressure(ErrorContext::new(
        ErrorPhase::Binding,
        RetryClass::Never,
    )))
}
impl clinkz_wot_planning::PrepareClientBinding for Client {
    type Compiler = ResolvedTargetCompiler;
}
impl PollClientBinding for Client {
    type Artifact =
        <ResolvedTargetCompiler as clinkz_wot_planning::BindingCompilerExtension>::Artifact;
    type RequestState = State;
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }
    fn request_state_layout(&self) -> BindingStateLayout {
        BindingStateLayout::of::<State>(BindingLifetimeFootprint::new(2, 128))
            .with_transient(BindingTransientFootprint::new(256))
    }
    fn start_request(
        &mut self,
        request: OutboundRequest,
        artifact: &BindingArtifactEnvelope<ResolvedTargetArtifact>,
        slot: &mut ClientRequestSlot<State>,
        budget: &mut WorkBudget,
    ) -> Result<StartStatus<CoreResult<InteractionOutput>>, BindingInputRejection<OutboundRequest>>
    {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Err(BindingInputRejection::new(request, error()));
        }
        let output = output(&request, artifact.artifact().payload().target());
        self.stats.starts.fetch_add(1, Ordering::SeqCst);
        slot.initialize(
            request,
            State {
                output: Some(output),
                stats: self.stats.clone(),
            },
        );
        Ok(StartStatus::Pending)
    }
    fn poll_request(
        &mut self,
        _: &mut Context<'_>,
        slot: &mut ClientRequestSlot<State>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<InteractionOutput>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        Poll::Ready(Ok(slot.state_mut().output.take().unwrap()))
    }
    fn start_cancel_request(
        &mut self,
        _: &mut Context<'_>,
        _: CleanupPhaseContext,
        slot: &mut ClientRequestSlot<State>,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Ok(StartStatus::Pending);
        }
        Ok(StartStatus::Ready(BindingCallSettlement::Returned(Ok(
            slot.state_mut().output.take().unwrap(),
        ))))
    }
    fn poll_cancel_request(
        &mut self,
        _: &mut Context<'_>,
        slot: &mut ClientRequestSlot<State>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        Poll::Ready(Ok(BindingCallSettlement::Returned(Ok(slot
            .state_mut()
            .output
            .take()
            .unwrap()))))
    }
    fn acknowledge_request(&mut self, _: &mut ClientRequestSlot<State>) -> CoreResult<()> {
        Ok(())
    }
}
#[cfg(feature = "std")]
impl binding::ClientBinding for Client {
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }
    fn invoke(
        &self,
        request: OutboundRequest,
        artifact: &BindingArtifactEnvelope<HostBindingArtifact>,
    ) -> Result<
        HostBindingCallBox<CoreResult<InteractionOutput>>,
        BindingInputRejection<OutboundRequest>,
    > {
        let target = artifact
            .artifact()
            .try_payload::<ResolvedTargetArtifact>(self.compatibility)
            .unwrap()
            .target();
        self.stats.starts.fetch_add(1, Ordering::SeqCst);
        Ok(HostBindingCallBox::new(State {
            output: Some(output(&request, target)),
            stats: self.stats.clone(),
        }))
    }
}
#[cfg(feature = "std")]
impl HostBindingCall<CoreResult<InteractionOutput>> for State {
    fn next_deadline(&self) -> Option<Deadline> {
        None
    }
    fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
        BindingLifetimeFootprint::new(3, core::mem::size_of::<Self>() as u64 + 128)
    }
    fn poll_result(
        mut self: core::pin::Pin<&mut Self>,
        _: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<InteractionOutput>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        Poll::Ready(Ok(self.output.take().unwrap()))
    }
    fn start_cancel(
        mut self: core::pin::Pin<&mut Self>,
        _: &mut Context<'_>,
        _: CleanupPhaseContext,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Ok(StartStatus::Pending);
        }
        Ok(StartStatus::Ready(BindingCallSettlement::Returned(Ok(
            self.output.take().unwrap(),
        ))))
    }
    fn poll_cancel(
        mut self: core::pin::Pin<&mut Self>,
        _: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        Poll::Ready(Ok(BindingCallSettlement::Returned(Ok(self
            .output
            .take()
            .unwrap()))))
    }
}
pub fn resources() -> BindingResourceDeclarations {
    BindingResourceDeclarations::new(
        BindingLifetimeFootprint::new(0, 0),
        BindingLifetimeFootprint::new(8, 4096),
    )
    .with_state_footprints(
        BindingLifetimeFootprint::new(0, 0),
        BindingLifetimeFootprint::new(0, 0),
        BindingLifetimeFootprint::new(0, 0),
    )
    .with_transient(BindingTransientFootprint::new(256))
}
pub fn identity(c: &ResolvedTargetCompiler) -> BindingRegistrationIdentity {
    BindingRegistrationIdentity::new(
        BindingId::new(223),
        BindingGeneration::INITIAL.checked_next().unwrap(),
        c.configuration(),
        c.compatibility(),
        31,
    )
}
pub fn static_registration(
    c: ResolvedTargetCompiler,
    stats: Arc<Stats>,
) -> ConsumerCompilerRegistration<StaticConsumerBindingPreparation<Client>> {
    let compatibility = c.compatibility();
    let input = StaticConsumerBindingPreparationInput::new(
        identity(&c),
        BindingExecutionSupport::application_static(),
        StaticBindingCompilerRegistration::new(c),
        Client {
            compatibility,
            stats,
        },
        resources(),
        BindingStatusPolicy::new(2, 17),
    );
    StaticConsumerBindingPreparation::new(input)
        .unwrap_or_else(|_| panic!("static registration"))
        .try_into_consumer_compiler()
        .unwrap_or_else(|_| panic!("static capture"))
}
#[cfg(feature = "std")]
pub fn host_registration(
    c: ResolvedTargetCompiler,
    stats: Arc<Stats>,
) -> ConsumerCompilerRegistration<HostConsumerBindingPreparation> {
    let compatibility = c.compatibility();
    let input = HostConsumerBindingPreparationInput::new(
        identity(&c),
        BindingExecutionSupport::host_erased(),
        HostBindingCompilerRegistration::try_new_consumer(c).unwrap(),
        Box::new(Client {
            compatibility,
            stats,
        }),
        resources(),
        BindingStatusPolicy::new(2, 17),
    );
    HostConsumerBindingPreparation::new(input)
        .unwrap_or_else(|_| panic!("Host registration"))
        .try_into_consumer_compiler()
        .unwrap_or_else(|_| panic!("Host capture"))
}

// Normal-library compilation checks the actual adapters in both ARM cells.
fn aggregate_eligible<R: clinkz_wot_planning::ConsumerPropertyReadRegistration>() {}
pub fn check_normal_graph() {
    aggregate_eligible::<ConsumerCompilerRegistration<StaticConsumerBindingPreparation<Client>>>();
    #[cfg(feature = "std")]
    aggregate_eligible::<ConsumerCompilerRegistration<HostConsumerBindingPreparation>>();
}

const fn layout<T>() -> [usize; 2] {
    [core::mem::size_of::<T>(), core::mem::align_of::<T>()]
}
// Kept in normal-library IR so each ARM compile cell reports its own layouts.
// Rows: complete registration, checked registration, complete request slot,
// raw state, client, Planning cleanup root, Planning draft, native artifact,
// compiler, support, owned configuration and fixed diagnostic context.
#[used]
pub static CONSUMER_ONLY_STATIC_LAYOUTS: [[usize; 2]; 12] = [
    layout::<StaticConsumerBindingPreparation<Client>>(),
    layout::<ConsumerCompilerRegistration<StaticConsumerBindingPreparation<Client>>>(),
    layout::<StaticConsumerPropertyReadSlot<State>>(),
    layout::<State>(),
    layout::<Client>(),
    layout::<clinkz_wot_planning::ConsumerPropertyReadCleanupSlot<ResolvedTargetArtifact>>(),
    layout::<clinkz_wot_planning::ConsumerPropertyReadDraft<ResolvedTargetArtifact>>(),
    layout::<ResolvedTargetArtifact>(),
    layout::<ResolvedTargetCompiler>(),
    layout::<ConsumerCompilerSupport>(),
    layout::<ConsumerBindingConfiguration>(),
    layout::<ErrorContext>(),
];
