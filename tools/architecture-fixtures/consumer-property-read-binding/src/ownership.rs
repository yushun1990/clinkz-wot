//! Concrete finite private policy for the existing production-runtime author.
//! This adds no engine, seal, scheduler or admission implementation.
use super::*;
use alloc::{
    alloc::{Layout, alloc},
    boxed::Box,
};

pub struct PrivatePool {
    limit: usize,
    used: AtomicUsize,
    shared: Option<Box<[u8; 8]>>,
    pub refusals: AtomicUsize,
    pub disposal_requests: AtomicUsize,
    pub disposal_errors: AtomicUsize,
    pub retired: Arc<AtomicUsize>,
}
impl PrivatePool {
    pub fn new(limit: u64, retired: Arc<AtomicUsize>) -> Arc<Self> {
        assert!(limit >= 8);
        assert!(limit <= u32::MAX as u64);
        Arc::new(Self {
            limit: limit as usize,
            used: AtomicUsize::new(8),
            shared: Some(Box::new([0; 8])),
            refusals: AtomicUsize::new(0),
            disposal_requests: AtomicUsize::new(0),
            disposal_errors: AtomicUsize::new(0),
            retired,
        })
    }
    pub fn used(&self) -> u64 {
        self.used.load(Ordering::SeqCst) as u64
    }
    pub fn limit(&self) -> u64 {
        self.limit as u64
    }
    #[allow(deprecated)] // Keep the accepted Rust 1.95 surface too.
    fn acquire(self: &Arc<Self>) -> Option<PrivateLease> {
        let funded = self
            .used
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
                n.checked_add(8).filter(|n| *n <= self.limit)
            })
            .is_ok();
        if !funded {
            self.refusals.fetch_add(1, Ordering::SeqCst);
            return None;
        }
        // Supported returned allocation error: return the private charge before
        // retry; neither an engine reservation nor an accepted owner is lost.
        let pointer = unsafe { alloc(Layout::new::<[u8; 8]>()) }.cast::<[u8; 8]>();
        if pointer.is_null() {
            assert!(self.used.fetch_sub(8, Ordering::SeqCst) >= 8);
            self.disposal_errors.fetch_add(1, Ordering::SeqCst);
            return None;
        }
        // SAFETY: unique aligned storage for this complete initialized array.
        unsafe {
            pointer.write([0; 8]);
        }
        Some(PrivateLease {
            storage: Some(unsafe { Box::from_raw(pointer) }),
            pool: Arc::clone(self),
        })
    }
}
impl Drop for PrivatePool {
    fn drop(&mut self) {
        drop(self.shared.take());
        assert!(self.used.fetch_sub(8, Ordering::SeqCst) >= 8);
        assert_eq!(self.used(), 0);
        self.retired.fetch_add(1, Ordering::SeqCst);
    }
}
struct PrivateLease {
    storage: Option<Box<[u8; 8]>>,
    pool: Arc<PrivatePool>,
}
impl Drop for PrivateLease {
    fn drop(&mut self) {
        drop(self.storage.take());
        assert!(self.pool.used.fetch_sub(8, Ordering::SeqCst) >= 8);
    }
}

#[derive(Clone)]
pub struct OwnedPolicy {
    configuration: ConsumerBindingConfiguration,
    pub(super) mode: Mode,
    pool: Arc<PrivatePool>,
    allocating_disposal: bool,
}
impl OwnedPolicy {
    pub fn try_new(
        contract: BindingResourceContract,
        mode: Mode,
        pool: Arc<PrivatePool>,
        allocating_disposal: bool,
        capacity: usize,
        body: u64,
        media: u64,
    ) -> CoreResult<Self> {
        if allocating_disposal && contract != BindingResourceContract::ConsumerOwnedHostV2 {
            return Err(CoreError::UnsupportedOperation(ErrorContext::new(
                ErrorPhase::Admission,
                RetryClass::Never,
            )));
        }
        // Injective fixed encoding of every concrete behavioral setting: finite
        // mode, disposal, shared policy ceiling, input 64, inbox 8, concurrency
        // 1, growth chunk 8, no jobs, synchronous one-step retirement. Addresses,
        // live counters, selected targets and engine quotas are deliberately absent.
        let mut key = [0; 32];
        key[..16].copy_from_slice(b"fixture-owned-v1");
        key[16] = mode as u8;
        let disposal = match contract {
            BindingResourceContract::StrictV1 => 0,
            BindingResourceContract::ConsumerOwnedV2 => 1,
            BindingResourceContract::ConsumerOwnedHostV2 => 2,
        };
        key[17] = (disposal << 1) | allocating_disposal as u8;
        key[18..26].copy_from_slice(&pool.limit().to_le_bytes());
        key[26..].copy_from_slice(&[64, 8, 1, 8, 0, 1]);
        let configuration = ConsumerBindingConfiguration::try_new(
            contract,
            capacity,
            BindingConfigurationDigest::new(key),
            body,
            media,
        )?;
        Ok(Self {
            configuration,
            mode,
            pool,
            allocating_disposal,
        })
    }
    pub fn configuration(&self) -> ConsumerBindingConfiguration {
        self.configuration
    }
    pub fn with_default_json_forms(mut self) -> Self {
        self.configuration = self.configuration.with_default_json_forms().unwrap();
        self
    }
    pub(super) fn call(&self) -> OwnedCall {
        OwnedCall {
            policy: self.clone(),
            lease: None,
        }
    }
}
pub(super) struct OwnedCall {
    policy: OwnedPolicy,
    lease: Option<PrivateLease>,
}
impl OwnedCall {
    pub(super) fn progress(&mut self) -> bool {
        if self.lease.is_none() {
            self.lease = self.policy.pool.acquire();
        }
        // One bounded refusal per paid poll; keep the exact prebacked result,
        // raw request and all engine cleanup funding while backpressured.
        self.lease.is_some()
    }
}
impl Drop for OwnedCall {
    fn drop(&mut self) {
        drop(self.lease.take());
        if self.policy.allocating_disposal {
            self.policy
                .pool
                .disposal_requests
                .fetch_add(1, Ordering::SeqCst);
            drop(self.policy.pool.acquire());
        }
    }
}

/// Independently authored single-inbox static registration. It reuses the
/// fixture's finite raw call mechanics and private pool, not an engine or seal.
pub struct SecondClient {
    policy: OwnedPolicy,
    output: Option<InteractionOutput>,
    stats: Arc<Stats>,
}
impl clinkz_wot_planning::PrepareClientBinding for SecondClient {
    type Compiler = ResolvedTargetCompiler;
}
impl PollClientBinding for SecondClient {
    type Artifact =
        <ResolvedTargetCompiler as clinkz_wot_planning::BindingCompilerExtension>::Artifact;
    type RequestState = State;
    fn consumer_configuration(&self) -> Option<ConsumerBindingConfiguration> {
        Some(self.policy.configuration())
    }
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        ResolvedTargetCompiler::new_owned(self.policy.configuration()).compatibility()
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
        if self.output.is_none() || budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Err(rejection(
                request,
                Some(&self.policy),
                ResourceFailureDomain::BindingPolicy,
            ));
        }
        self.stats.selected(artifact.artifact().payload().target());
        self.stats.starts.fetch_add(1, Ordering::SeqCst);
        let output = prepare_output(&request, self.output.take().unwrap(), Mode::LateStart);
        slot.initialize(
            request,
            State {
                request: None,
                output: Some(output),
                stats: Arc::clone(&self.stats),
                mode: Mode::LateStart,
                static_fabricated: false,
                prepared: true,
                validation: Mode::LateStart,
                owned: Some(self.policy.call()),
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
    fn acknowledge_request(&mut self, _: &mut ClientRequestSlot<State>) -> CoreResult<()> {
        self.stats.acknowledgements.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
pub fn second_registration(
    policy: OwnedPolicy,
    output: InteractionOutput,
) -> ConsumerCompilerRegistration<StaticConsumerBindingPreparation<SecondClient>> {
    let mut policy = policy;
    let c = policy.configuration();
    let mut private = *c.private_policy().as_bytes();
    // A distinct authored behavior: one inbox item and fixed LateStart mode.
    private[16] = Mode::LateStart as u8;
    private[27] = 1;
    policy.mode = Mode::LateStart;
    policy.configuration = ConsumerBindingConfiguration::try_new(
        c.resource_contract(),
        c.target_capacity(),
        BindingConfigurationDigest::new(private),
        c.result_limits().0,
        c.result_limits().1,
    )
    .unwrap();
    let compiler = ResolvedTargetCompiler::new_owned(policy.configuration());
    let id = BindingRegistrationIdentity::new(
        BindingId::new(224),
        BindingGeneration::INITIAL,
        compiler.configuration(),
        compiler.compatibility(),
        32,
    );
    let resources = resources()
        .with_resource_contract(c.resource_contract())
        .with_consumer_result_limits(c.result_limits().0, c.result_limits().1);
    StaticConsumerBindingPreparation::new(StaticConsumerBindingPreparationInput::new(
        id,
        BindingExecutionSupport::application_static(),
        StaticBindingCompilerRegistration::new(compiler),
        SecondClient {
            policy,
            output: Some(output),
            stats: Arc::new(Stats::default()),
        },
        resources,
        BindingStatusPolicy::new(2, 17),
    ))
    .unwrap_or_else(|_| panic!("second author registration"))
    .try_into_consumer_compiler()
    .unwrap_or_else(|_| panic!("second author support"))
}
