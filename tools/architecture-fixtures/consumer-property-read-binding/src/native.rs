//! Independently authored inline lowering. This crate is downstream of all
//! engine crates. Its finite loopback transport records native request bytes;
//! it makes no real-protocol/SDK or general JSON-LD conformance claim.
use super::*;
use clinkz_wot_td::context::ContextEntry;

pub const COMPATIBILITY: BindingArtifactCompatibility =
    BindingArtifactCompatibility::new(*b"external-wire-v1");
pub const CONFIGURATION: BindingConfigurationDigest = BindingConfigurationDigest::new([191; 32]);
const STEP: u64 = 64;
pub struct Compiler(pub Arc<Stats>);
#[derive(Debug)]
pub struct Artifact {
    bytes: [u8; 256],
    len: usize,
}
impl Artifact {
    pub fn request(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap()
    }
    fn append(&mut self, value: &str) -> CoreResult<()> {
        let end = self
            .len
            .checked_add(value.len())
            .filter(|n| *n <= self.bytes.len())
            .ok_or_else(unsupported)?;
        self.bytes[self.len..end].copy_from_slice(value.as_bytes());
        self.len = end;
        Ok(())
    }
}
fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
}
impl BindingCompilerExtension for Compiler {
    type Cursor = bool;
    type Artifact = Artifact;
    fn compatibility(&self) -> BindingArtifactCompatibility {
        COMPATIBILITY
    }
    fn consumer_contract(&self) -> Option<ConsumerCompilerContract> {
        Some(
            ConsumerCompilerContract::try_new(
                CONFIGURATION,
                COMPATIBILITY,
                [1, 1, 1, STEP, 1, 1],
                2,
            )
            .unwrap(),
        )
    }
    fn bounds(&self, _: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        Ok(BindingCompilerBounds::new(
            BindingArtifactFootprint::new(1, core::mem::size_of::<Artifact>() as u64),
            core::mem::size_of::<bool>() as u64,
            0,
            WorkBudget::new().with_remaining(WorkClass::BindingPolls, 2 * STEP),
        ))
    }
    fn start(&self, _: &BindingCompilerInput<'_>) -> CoreResult<bool> {
        Ok(false)
    }
    fn abort(&self, _: bool) {
        self.0.compiler_aborts.fetch_add(1, Ordering::SeqCst);
    }
    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: bool,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<bool, Artifact> {
        if budget.consume(WorkClass::BindingPolls, STEP).is_err() {
            return BindingCompilerStep::Pending(cursor);
        }
        self.0.compiler_polls.fetch_add(1, Ordering::SeqCst);
        if !cursor {
            return BindingCompilerStep::Pending(true);
        }
        match lower(input) {
            Ok(artifact) => {
                self.0.compilations.fetch_add(1, Ordering::SeqCst);
                BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
                    COMPATIBILITY,
                    BindingArtifactFootprint::new(1, core::mem::size_of::<Artifact>() as u64),
                    artifact,
                )))
            }
            Err(error) => BindingCompilerStep::Failed(BindingCompilerFailure::new(error, cursor)),
        }
    }
}
fn lower(input: &BindingCompilerInput<'_>) -> CoreResult<Artifact> {
    let source = input.source().ok_or_else(unsupported)?;
    let form = source.form();
    let entries = source.thing().context.entries();
    if entries.len() > 4
        || form._extra_fields.len() != 1
        || input.logical_plan().resolved_target().len() > 192
        || source.thing().uri_variables.is_some()
        || source.property()._interaction.uri_variables.is_some()
        || input.logical_plan().resolved_target().contains(['{', '}'])
        || input.logical_plan().subprotocol().is_some()
        || form.response.is_some()
        || form.content_coding.is_some()
        || form.additional_responses.is_some()
        || form.scopes.as_ref().is_some_and(|s| !s.is_empty())
        || source.effective_security().len() != 1
        || input.logical_plan().content_type() != Some("application/json")
    {
        return Err(unsupported());
    }
    // A deliberately finite binding vocabulary: one local prefix, <=4 context
    // entries, <=4 object terms, <=8 prefix bytes, two literal instructions.
    // No remote lookup, recursive expansion, source copy, or protocol I/O.
    let (key, value) = form._extra_fields.iter().next().unwrap();
    if key.len() > 16 {
        return Err(unsupported());
    }
    let (prefix, term) = key.split_once(':').ok_or_else(unsupported)?;
    if prefix.len() > 8 || term != "verb" {
        return Err(unsupported());
    }
    let mut namespace = None;
    for entry in entries {
        if let ContextEntry::Object(terms) = entry {
            if terms.len() > 4 {
                return Err(unsupported());
            }
            if let Some(value) = terms.get(prefix) {
                namespace = value.as_str();
            }
        }
    }
    if namespace != Some("urn:external:wire:") {
        return Err(unsupported());
    }
    let verb = match value.as_str() {
        Some("FETCH") => "FETCH",
        Some("READ") => "READ",
        _ => return Err(unsupported()),
    };
    let mut out = Artifact {
        bytes: [0; 256],
        len: 0,
    };
    out.append(verb)?;
    out.append(" ")?;
    out.append(input.logical_plan().resolved_target())?;
    Ok(out)
}
fn identity() -> BindingRegistrationIdentity {
    BindingRegistrationIdentity::new(
        BindingId::new(224),
        BindingGeneration::INITIAL,
        CONFIGURATION,
        COMPATIBILITY,
        31,
    )
}
pub struct StaticClient(pub super::StaticClient);
impl PollClientBinding for StaticClient {
    type Compiler = Compiler;
    type RequestState = State;
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        COMPATIBILITY
    }
    fn request_state_layout(&self) -> BindingStateLayout {
        self.0.request_state_layout()
    }
    fn start_request(
        &mut self,
        request: OutboundRequest,
        artifact: &BindingArtifactEnvelope<Artifact>,
        slot: &mut ClientRequestSlot<State>,
        budget: &mut WorkBudget,
    ) -> Result<StartStatus<CoreResult<InteractionOutput>>, BindingInputRejection<OutboundRequest>>
    {
        self.0.start_target(
            request,
            artifact.artifact().payload().request(),
            slot,
            budget,
        )
    }
    fn poll_request(
        &mut self,
        cx: &mut Context<'_>,
        slot: &mut ClientRequestSlot<State>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<InteractionOutput>> {
        self.0.poll_request(cx, slot, budget)
    }
    fn start_cancel_request(
        &mut self,
        cx: &mut Context<'_>,
        cleanup: CleanupPhaseContext,
        slot: &mut ClientRequestSlot<State>,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        self.0.start_cancel_request(cx, cleanup, slot, budget)
    }
    fn poll_cancel_request(
        &mut self,
        cx: &mut Context<'_>,
        slot: &mut ClientRequestSlot<State>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        self.0.poll_cancel_request(cx, slot, budget)
    }
    fn acknowledge_request(&mut self, slot: &mut ClientRequestSlot<State>) -> CoreResult<()> {
        self.0.acknowledge_request(slot)
    }
}
pub fn static_registration(
    client: super::StaticClient,
) -> ConsumerCompilerRegistration<StaticConsumerBindingRegistration<StaticClient>> {
    let compiler = Compiler(client.stats.clone());
    StaticConsumerBindingRegistration::new(StaticConsumerBindingRegistrationInput::new(
        identity(),
        BindingExecutionSupport::application_static(),
        StaticBindingCompilerRegistration::new(compiler),
        StaticClient(client),
        resources(),
        BindingStatusPolicy::new(2, 17),
    ))
    .unwrap_or_else(|_| panic!("native registration"))
    .try_into_consumer_compiler()
    .unwrap_or_else(|_| panic!("open native admission"))
}
#[cfg(feature = "std")]
pub struct HostClient(pub super::HostClient);
#[cfg(feature = "std")]
impl binding::ClientBinding for HostClient {
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        COMPATIBILITY
    }
    fn invoke(
        &self,
        request: OutboundRequest,
        artifact: &BindingArtifactEnvelope<HostBindingArtifact>,
    ) -> Result<
        HostBindingCallBox<CoreResult<InteractionOutput>>,
        BindingInputRejection<OutboundRequest>,
    > {
        self.0.invoke_target(
            request,
            artifact
                .artifact()
                .try_payload::<Artifact>(COMPATIBILITY)
                .unwrap()
                .request(),
        )
    }
}
#[cfg(feature = "std")]
pub fn host_registration(
    client: super::HostClient,
) -> ConsumerCompilerRegistration<HostConsumerBindingRegistration> {
    let compiler = Compiler(client.stats.clone());
    HostConsumerBindingRegistration::new(HostConsumerBindingRegistrationInput::new(
        identity(),
        BindingExecutionSupport::host_erased(),
        HostBindingCompilerRegistration::try_new_consumer(compiler)
            .unwrap_or_else(|_| panic!("native compiler")),
        alloc::boxed::Box::new(HostClient(client)),
        resources(),
        BindingStatusPolicy::new(2, 17),
    ))
    .unwrap_or_else(|_| panic!("native registration"))
    .try_into_consumer_compiler()
    .unwrap_or_else(|_| panic!("open native admission"))
}
