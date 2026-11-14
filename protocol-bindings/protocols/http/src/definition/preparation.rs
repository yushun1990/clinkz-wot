//! Explicit HTTP compile-once / checked-bind-many through the existing owned SPI.
use super::*;
use clinkz_wot_core::security::SecurityRequirement;
use clinkz_wot_planning::{open_consumer::ConsumerRegistrationInput, *};
use clinkz_wot_td::semantic::{
    FormOwner, SemanticName, SemanticNode, SemanticScope, SemanticValue,
};

// A deliberately bounded dependency closure, not a serialized program format.
const DEPENDENCY_BYTES: usize = 8192;
const TEMPORARY: u64 = (DEPENDENCY_BYTES + std::mem::size_of::<HttpArtifact>() + 512) as u64;

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
                let mut sink = DependencySink::compare(&definition.0.dependencies);
                dependencies(input, &mut sink)?;
                sink.finish()?;
                definition.clone()
            }
            Mode::Compile => {
                // Acquire original physical backing before dependency allocation
                // or lowering. Failure drops provisional bytes before leases.
                let funding = definition_funding(&self.0.bank)?;
                let mut dependencies_bytes = Vec::new();
                dependencies_bytes
                    .try_reserve_exact(DEPENDENCY_BYTES)
                    .map_err(|_| busy())?;
                let mut sink = DependencySink::write(&mut dependencies_bytes);
                dependencies(input, &mut sink)?;
                sink.finish()?;
                self.0.compilations.fetch_add(1, Ordering::Relaxed);
                let instructions =
                    HttpCompiler::for_finite(self.0.config).compile_definition(input)?;
                HttpRequestDefinition(Arc::new(CompiledRequest {
                    configuration: self.0.config.configuration,
                    instructions,
                    dependencies: dependencies_bytes,
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
) -> CoreResult<[HostCapacityReservation; 5]> {
    let bytes = (std::mem::size_of::<CompiledRequest>() + DEPENDENCY_BYTES + 128) as u64;
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

// This projection sits beside the actual HTTP compiler. It captures everything
// lowering reads except the separately checked destination. Common schema/URI
// meanings and live service qualification still run in ConsumerRegistry.
fn dependencies(input: ResolvedBindingInput<'_>, sink: &mut DependencySink<'_>) -> CoreResult<()> {
    let collection = input.owner == FormOwner::Thing;
    sink.byte(match input.owner {
        FormOwner::Thing => 0,
        FormOwner::Property(_) => 1,
        _ => return Err(crate::compiler::unsupported()),
    })?;
    sink.byte(input.operation as u8)?;
    sink.optional_text(input.content_type)?;
    sink.optional_text(input.subprotocol)?;
    let facts = input
        .document
        .form(input.owner, input.form_index)
        .ok_or_else(crate::compiler::unsupported)?;
    sink.text(facts.content_type)?;
    sink.optional_text(facts.content_coding)?;
    sink.optional_text(facts.response_content_type)?;
    sink.byte(u8::from(facts.declared_operations.is_some()))?;
    for ops in [
        facts.declared_operations.unwrap_or(&[]),
        facts.effective_operations,
    ] {
        sink.length(ops.len())?;
        for op in ops {
            sink.byte(*op as u8)?;
        }
    }
    // ConsumerRegistry already qualifies this conjunction against the current
    // effective expression; raw scheme names are not native placement identity.
    let requirement = input
        .selected_security
        .ok_or_else(crate::compiler::unsupported)?;
    if !requirement.is_conjunction() {
        return Err(crate::compiler::unsupported());
    }
    security(requirement, sink)?;
    if !collection {
        let flags = input.document.property().schema.facts();
        sink.byte(u8::from(flags.read_only))?;
        sink.byte(u8::from(flags.write_only))?;
    }
    let scope = if collection {
        SemanticScope::AffordanceForm {
            owner: input.owner,
            index: input.form_index,
        }
    } else {
        SemanticScope::Form {
            property: input.property,
            index: input.form_index,
        }
    };
    sink.scope(input.document.scope(scope))?;
    // Root extension assertions affect the supported native collection profile.
    if matches!(
        input.operation,
        Operation::ReadMultipleProperties
            | Operation::QueryAllActions
            | Operation::WriteAllProperties
            | Operation::WriteMultipleProperties
    ) {
        sink.scope(input.document.scope(SemanticScope::Thing))?;
    }
    let response = if collection {
        SemanticScope::AffordanceResponse {
            owner: input.owner,
            index: input.form_index,
        }
    } else {
        SemanticScope::Response {
            property: input.property,
            index: input.form_index,
        }
    };
    sink.scope(input.document.scope(response))?;
    let additional = input
        .document
        .additional_responses(input.owner, input.form_index)
        .ok_or_else(crate::compiler::unsupported)?;
    for (i, response) in additional.enumerate() {
        if i == 32 {
            return Err(crate::compiler::unsupported());
        }
        sink.byte(1)?;
        sink.text(response.content_type)?;
        sink.byte(u8::from(response.success))?;
        sink.byte(u8::from(response.schema.is_some()))?;
        let scope = if collection {
            SemanticScope::AffordanceAdditionalResponse {
                owner: input.owner,
                form: input.form_index,
                index: i as u32,
            }
        } else {
            SemanticScope::AdditionalResponse {
                property: input.property,
                form: input.form_index,
                index: i as u32,
            }
        };
        sink.scope(input.document.scope(scope))?;
    }
    sink.byte(0)
}
fn security(r: &SecurityRequirement, sink: &mut DependencySink<'_>) -> CoreResult<()> {
    match r {
        SecurityRequirement::NoSec => sink.byte(0),
        SecurityRequirement::Scheme { scheme, parameters } => {
            sink.byte(1)?;
            sink.text(scheme)?;
            sink.length(parameters.len())?;
            for (k, v) in parameters {
                sink.text(k)?;
                sink.text(v)?;
            }
            Ok(())
        }
        SecurityRequirement::AllOf(children) => {
            sink.byte(2)?;
            sink.length(children.len())?;
            for r in children {
                security(r, sink)?;
            }
            Ok(())
        }
        _ => Err(crate::compiler::unsupported()),
    }
}

enum Output<'a> {
    Write(&'a mut Vec<u8>),
    Compare(&'a [u8]),
}
struct DependencySink<'a> {
    output: Output<'a>,
    position: usize,
    nodes: usize,
}
impl<'a> DependencySink<'a> {
    fn write(bytes: &'a mut Vec<u8>) -> Self {
        Self {
            output: Output::Write(bytes),
            position: 0,
            nodes: 0,
        }
    }
    fn compare(bytes: &'a [u8]) -> Self {
        Self {
            output: Output::Compare(bytes),
            position: 0,
            nodes: 0,
        }
    }
    fn bytes(&mut self, bytes: &[u8]) -> CoreResult<()> {
        let end = self
            .position
            .checked_add(bytes.len())
            .filter(|n| *n <= DEPENDENCY_BYTES)
            .ok_or_else(crate::compiler::unsupported)?;
        match &mut self.output {
            Output::Write(out) => out.extend_from_slice(bytes),
            Output::Compare(expected) if expected.get(self.position..end) == Some(bytes) => {}
            _ => return Err(crate::compiler::unsupported()),
        }
        self.position = end;
        Ok(())
    }
    fn finish(&self) -> CoreResult<()> {
        if matches!(&self.output, Output::Compare(bytes) if bytes.len() != self.position) {
            return Err(crate::compiler::unsupported());
        }
        Ok(())
    }
    fn byte(&mut self, byte: u8) -> CoreResult<()> {
        self.bytes(&[byte])
    }
    fn length(&mut self, n: usize) -> CoreResult<()> {
        self.bytes(
            &u32::try_from(n)
                .map_err(|_| crate::compiler::unsupported())?
                .to_le_bytes(),
        )
    }
    fn text(&mut self, text: &str) -> CoreResult<()> {
        self.length(text.len())?;
        self.bytes(text.as_bytes())
    }
    fn optional_text(&mut self, text: Option<&str>) -> CoreResult<()> {
        self.byte(u8::from(text.is_some()))?;
        if let Some(text) = text {
            self.text(text)?;
        }
        Ok(())
    }
    fn iri(&mut self, iri: clinkz_wot_td::semantic::SemanticIri<'_>) -> CoreResult<()> {
        let (prefix, suffix) = iri.segments();
        self.length(prefix.len() + suffix.len())?;
        self.bytes(prefix.as_bytes())?;
        self.bytes(suffix.as_bytes())
    }
    fn scope(&mut self, scope: Option<SemanticNode<'_>>) -> CoreResult<()> {
        self.byte(u8::from(scope.is_some()))?;
        if let Some(scope) = scope {
            self.children(scope)?;
        }
        Ok(())
    }
    fn children(&mut self, node: SemanticNode<'_>) -> CoreResult<()> {
        for child in node.children() {
            self.byte(1)?;
            self.node(child)?;
        }
        self.byte(0)
    }
    fn node(&mut self, node: SemanticNode<'_>) -> CoreResult<()> {
        self.nodes += 1;
        if self.nodes > 128 {
            return Err(crate::compiler::unsupported());
        }
        match node.name() {
            SemanticName::Iri(iri) => {
                self.byte(0)?;
                self.iri(iri)?;
            }
            SemanticName::Keyword(s) => {
                self.byte(1)?;
                self.text(s)?;
            }
            SemanticName::Unmapped(s) => {
                self.byte(2)?;
                self.text(s)?;
            }
            SemanticName::Item => self.byte(3)?,
        }
        match node.value() {
            SemanticValue::Null => self.byte(0)?,
            SemanticValue::Bool(b) => {
                self.byte(1)?;
                self.byte(u8::from(b))?;
            }
            SemanticValue::Number(n) => {
                self.byte(2)?;
                self.text(&n.to_string())?;
            }
            SemanticValue::Text(s) => {
                self.byte(3)?;
                self.text(s)?;
            }
            SemanticValue::Iri(iri) => {
                self.byte(4)?;
                self.iri(iri)?;
            }
            SemanticValue::Array => self.byte(5)?,
            SemanticValue::Object => self.byte(6)?,
        }
        self.children(node)
    }
}
