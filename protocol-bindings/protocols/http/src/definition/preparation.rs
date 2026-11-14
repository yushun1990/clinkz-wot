//! Explicit HTTP compile-once / checked-bind-many through the existing owned SPI.
use super::*;
use clinkz_wot_planning::{open_consumer::ConsumerRegistrationInput, *};
// Include the common security extractor's published envelope as well as bounded
// header/response vectors, URI qualification and inline lowering. All of this
// is temporary; retained input descendants are charged at their actual sizes.
const TEMPORARY: u64 = clinkz_wot_planning::security::TEMPORARY_BYTES
    + 8192
    + std::mem::size_of::<HttpArtifact>() as u64
    + 512;

enum Mode {
    Compile,
    Bind(HttpRequestDefinition),
}
struct Compiler {
    config: HttpConsumerConfig,
    mode: Mode,
    bank: Arc<HostResourceAccounts>,
    compilations: AtomicUsize,
    checks: AtomicUsize,
    bindings: AtomicUsize,
    _funding: HostCapacityReservation,
}

/// Explicit preparation capability, independent of any Thing or executor.
/// `new` lowers each selected mapping; `with_definition` only checks dependencies
/// and binds destinations, refusing changed dependencies without recompiling.
/// Keep a clone to observe actual work; no lookup table or automatic cache exists.
#[derive(Clone)]
pub struct HttpDefinitionCompiler(Arc<Compiler>);
impl HttpDefinitionCompiler {
    pub fn new(config: HttpConsumerConfig, bank: Arc<HostResourceAccounts>) -> CoreResult<Self> {
        Self::create(config, Mode::Compile, bank)
    }
    pub fn with_definition(
        config: HttpConsumerConfig,
        definition: &HttpRequestDefinition,
        bank: Arc<HostResourceAccounts>,
    ) -> CoreResult<Self> {
        if definition.0.configuration != config.configuration {
            return Err(crate::compiler::unsupported());
        }
        Self::create(config, Mode::Bind(definition.clone()), bank)
    }
    fn create(
        config: HttpConsumerConfig,
        mode: Mode,
        bank: Arc<HostResourceAccounts>,
    ) -> CoreResult<Self> {
        let funding = reserve(
            &bank,
            ResourceKind::EngineLiveBytesGlobalMax,
            (std::mem::size_of::<Compiler>() + 128) as u64,
        )?;
        Ok(Self(Arc::new(Compiler {
            config,
            mode,
            bank,
            compilations: AtomicUsize::new(0),
            checks: AtomicUsize::new(0),
            bindings: AtomicUsize::new(0),
            _funding: funding,
        })))
    }
    /// Number of actual native lowerings, including unsuccessful attempts.
    pub fn native_compilations(&self) -> usize {
        self.0.compilations.load(Ordering::Relaxed)
    }
    pub fn dependency_checks(&self) -> usize {
        self.0.checks.load(Ordering::Relaxed)
    }
    pub fn instance_bindings(&self) -> usize {
        self.0.bindings.load(Ordering::Relaxed)
    }
    /// Pair fresh cleartext native execution and services. Neither this method
    /// nor subsequent preparation performs I/O or grants instance permission.
    pub fn registration(
        &self,
        id: BindingId,
        generation: BindingGeneration,
        ordinal: u32,
    ) -> CoreResult<ConsumerRegistrationInput> {
        self.pair(
            try_consumer_execution(self.0.config, id, generation, ordinal)?,
            self.0.config.consumer_services(),
        )
    }
    /// Trust and credentials remain fresh live dependencies of this executor
    /// and access service, never fields of the retained request definition.
    pub fn https_registration(
        &self,
        tls: HttpClientTls,
        access: Arc<HttpClientAccess>,
        id: BindingId,
        generation: BindingGeneration,
        ordinal: u32,
    ) -> CoreResult<ConsumerRegistrationInput> {
        if !self.0.config.secure {
            return Err(crate::compiler::unsupported());
        }
        self.pair(
            consumer_execution(self.0.config, id, generation, ordinal, Some(tls))?,
            consumer::ConsumerServices {
                data: Arc::new(HttpConsumerData),
                access,
            },
        )
    }
    fn pair(
        &self,
        execution: HostConsumerBindingRegistration,
        services: consumer::ConsumerServices,
    ) -> CoreResult<ConsumerRegistrationInput> {
        let compiler = HostBindingCompilerRegistration::try_new_consumer_extension(self.clone())
            .map_err(|e| e.error().error().clone())?;
        let preparation = HostConsumerBindingPreparation::try_pair(execution, compiler)
            .map_err(|e| e.error().error().clone())?;
        Ok(ConsumerRegistrationInput {
            preparation,
            services,
        })
    }
    fn check(&self, input: &BindingCompilerInput<'_>) -> CoreResult<()> {
        if input.role() != BindingArtifactRole::ConsumerCall
            || input.candidate().configuration() != self.0.config.configuration.configuration()
            || input.candidate().compatibility() != COMPATIBILITY
            || input.resolved().is_none()
        {
            return Err(crate::compiler::unsupported());
        }
        Ok(())
    }
    fn construct(&self, input: ResolvedBindingInput<'_>) -> CoreResult<HttpBoundRequest> {
        // Concrete destination/template syntax is always instance work. In
        // particular, a template is never silently retargeted into a collection.
        HttpCompiler::for_finite(self.0.config).qualify_target(input)?;
        if input.target.len() > self.0.config.configuration.target_capacity() {
            return Err(crate::compiler::unsupported());
        }
        let definition = match &self.0.mode {
            Mode::Bind(definition) => {
                self.0.checks.fetch_add(1, Ordering::Relaxed);
                let current = HttpCompiler::for_finite(self.0.config).extract(input)?;
                if current != definition.0.input {
                    return Err(crate::compiler::unsupported());
                }
                definition.clone()
            }
            Mode::Compile => {
                // Borrowed extraction uses admitted temporary backing. Acquire
                // original physical backing before owning strings or lowering.
                // Failure drops provisional data before its original leases.
                let normalized = HttpCompiler::for_finite(self.0.config).extract(input)?;
                let funding = definition_funding(&self.0.bank, normalized.owned_bytes())?;
                self.0.compilations.fetch_add(1, Ordering::Relaxed);
                let instructions = normalized.lower()?;
                let normalized = normalized.into_owned();
                HttpRequestDefinition(Arc::new(CompiledRequest {
                    configuration: self.0.config.configuration,
                    instructions,
                    input: normalized,
                    _funding: funding,
                }))
            }
        };
        let mut target = [0; 256];
        target[..input.target.len()].copy_from_slice(input.target.as_bytes());
        self.0.bindings.fetch_add(1, Ordering::Relaxed);
        Ok(HttpBoundRequest {
            definition,
            target,
            target_len: input.target.len() as u16,
        })
    }
}

/// Pure preparation cursor; it retains the exact original candidate across steps.
pub struct HttpDefinitionCursor {
    plan: PlanId,
    candidate: BindingCandidate,
}
impl BindingCompilerExtension for HttpDefinitionCompiler {
    type Cursor = HttpDefinitionCursor;
    type Artifact = HttpBoundRequest;
    fn compatibility(&self) -> BindingArtifactCompatibility {
        COMPATIBILITY
    }
    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        self.check(input)?;
        Ok(BindingCompilerBounds::new(
            BindingArtifactFootprint::new(1, std::mem::size_of::<HttpBoundRequest>() as u64),
            std::mem::size_of::<HttpDefinitionCursor>() as u64,
            TEMPORARY,
            WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1),
        ))
    }
    fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
        self.check(input)?;
        Ok(HttpDefinitionCursor {
            plan: input.plan_id(),
            candidate: input.candidate(),
        })
    }
    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return BindingCompilerStep::Pending(cursor);
        }
        let result = self.check(input).and_then(|()| {
            if cursor.plan != input.plan_id() || cursor.candidate != input.candidate() {
                return Err(crate::compiler::unsupported());
            }
            self.construct(input.resolved().unwrap())
        });
        match result {
            Ok(output) => {
                BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
                    COMPATIBILITY,
                    BindingArtifactFootprint::new(
                        1,
                        std::mem::size_of::<HttpBoundRequest>() as u64,
                    ),
                    output,
                )))
            }
            Err(error) => BindingCompilerStep::Failed(BindingCompilerFailure::new(error, cursor)),
        }
    }
    fn abort(&self, _: Self::Cursor) {}
}
impl ConsumerCompiler for HttpDefinitionCompiler {
    const CONTRACT: ConsumerCompilerContract = ConsumerCompilerContract {
        inspection_work: 1,
        start_work: 1,
        step_work: 1,
        max_steps: 1,
        abort_work: 1,
        artifact_drop_work: 1,
        inspection_temporary_bytes: 0,
    };
    fn configuration(&self) -> ConsumerBindingConfiguration {
        self.0.config.configuration
    }
}

fn busy() -> CoreError {
    CoreError::Backpressure(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
}
fn reserve(
    bank: &Arc<HostResourceAccounts>,
    kind: ResourceKind,
    bytes: u64,
) -> CoreResult<HostCapacityReservation> {
    bank.reserve(kind, bytes).ok_or_else(busy)
}
fn definition_funding(
    bank: &Arc<HostResourceAccounts>,
    input_bytes: usize,
) -> CoreResult<[HostCapacityReservation; 5]> {
    let bytes = (std::mem::size_of::<CompiledRequest>() + input_bytes + 128) as u64;
    for kind in [
        ResourceKind::LargestContiguousAllocationBytesMax,
        ResourceKind::BindingArtifactBytesPerItemMax,
    ] {
        if bytes > bank.account(kind).limit() {
            return Err(busy());
        }
    }
    Ok([
        reserve(bank, ResourceKind::EngineLiveBytesGlobalMax, bytes)?,
        reserve(bank, ResourceKind::BindingArtifactsGlobalMax, 1)?,
        reserve(bank, ResourceKind::BindingArtifactBytesGlobalMax, bytes)?,
        reserve(bank, ResourceKind::CleanupBytesMax, bytes)?,
        reserve(bank, ResourceKind::CleanupItemsMax, 1)?,
    ])
}
