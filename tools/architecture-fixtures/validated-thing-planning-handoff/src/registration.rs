//! A complete existing static registration, with execution deliberately inert.
use clinkz_wot_core::*;
use clinkz_wot_foundation::WorkBudget;
use clinkz_wot_property_read_binding_fixture::{ManualMockBinding, MockArtifact, MockCompiler};
use core::task::{Context, Poll};

pub type Registration =
    StaticBindingRegistration<StaticBindingComponents<ManualMockBinding, InertClient>>;

// No TD-derived value or borrow can enter this separately retained owner.
pub struct InertClient {
    compatibility: BindingArtifactCompatibility,
}

impl PollClientBinding for InertClient {
    type Compiler = MockCompiler;
    type RequestState = ();
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }
    fn request_state_layout(&self) -> BindingStateLayout {
        BindingStateLayout::of::<()>(BindingLifetimeFootprint::new(0, 0))
    }
    fn start_request(
        &mut self,
        _: OutboundRequest,
        _: &BindingArtifactEnvelope<MockArtifact>,
        _: &mut ClientRequestSlot<()>,
        _: &mut WorkBudget,
    ) -> Result<StartStatus<CoreResult<InteractionOutput>>, BindingInputRejection<OutboundRequest>>
    {
        panic!("handoff proof must not execute protocol work")
    }
    fn poll_request(
        &mut self,
        _: &mut Context<'_>,
        _: &mut ClientRequestSlot<()>,
        _: &mut WorkBudget,
    ) -> Poll<CoreResult<InteractionOutput>> {
        panic!("handoff proof must not execute protocol work")
    }
    fn start_cancel_request(
        &mut self,
        _: &mut Context<'_>,
        _: CleanupPhaseContext,
        _: &mut ClientRequestSlot<()>,
        _: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        panic!("handoff proof must not execute protocol work")
    }
    fn poll_cancel_request(
        &mut self,
        _: &mut Context<'_>,
        _: &mut ClientRequestSlot<()>,
        _: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        panic!("handoff proof must not execute protocol work")
    }
    fn acknowledge_request(&mut self, _: &mut ClientRequestSlot<()>) -> CoreResult<()> {
        panic!("handoff proof must not execute protocol work")
    }
}

pub fn registration() -> Registration {
    let compatibility = BindingArtifactCompatibility::new([0x41; 16]);
    let identity = BindingRegistrationIdentity::new(
        BindingId::new(7),
        BindingGeneration::INITIAL,
        BindingConfigurationDigest::new([0x52; 32]),
        compatibility,
        0,
    );
    let input = StaticBindingRegistrationInput::producer_and_consumer_property_read(
        identity,
        BindingExecutionSupport::application_static(),
        StaticBindingCompilerRegistration::new(MockCompiler::new(compatibility)),
        StaticBindingComponents::new(
            ManualMockBinding::new(compatibility, 0),
            InertClient { compatibility },
        ),
        BindingResourceDeclarations::new(
            BindingLifetimeFootprint::new(4, 4096),
            BindingLifetimeFootprint::new(4, 4096),
        ),
        BindingIngressPolicy::hidden(),
        BindingStatusPolicy::new(2, 128),
    );
    match StaticBindingRegistration::producer_and_consumer_property_read(input) {
        Ok(registration) => registration,
        Err(_) => panic!("complete Consumer-capable fixture registration must validate"),
    }
}
