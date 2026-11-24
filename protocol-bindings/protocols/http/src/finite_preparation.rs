//! Optional preparation of the direct finite participant, without the legacy
//! Producer executor/installation interface.
use crate::finite_producer::{AssignedSocket, MAX_FINITE_MAPPINGS};
use crate::*;
use clinkz_wot_core::host_accounts::HostResourceAccounts;
use clinkz_wot_core::{finite::FiniteIngressBinding, producer::ProducerInstanceKey};
use clinkz_wot_planning::{
    ResolvedProducerInput,
    finite_producer::FiniteProducerCompiler,
    producer_composition::{
        FiniteProducerService, FormSite, PreparedOperation, ProducerCapability,
        ProducerOperationInput, PublicationObserver,
    },
};
use std::{any::Any, cell::Cell};

enum Preparation {
    Specialized,
    Compile(Arc<HostResourceAccounts>),
    Bind(HttpProducerDefinition),
}

/// One immutable finite HTTP preparation and native assembly registration.
pub struct HttpFiniteProducer {
    listen: HttpProducerConfig,
    id: BindingId,
    generation: BindingGeneration,
    timeout_ticks: u64,
    security: Option<HttpServerSecurity>,
    assigned: Option<AssignedSocket>,
    listener: Option<HttpFiniteListener>,
    preparation: Preparation,
    compilations: Cell<usize>,
    checks: Cell<usize>,
    bindings: Cell<usize>,
}

impl HttpFiniteProducer {
    pub fn try_new(
        listen: HttpProducerConfig,
        id: BindingId,
        generation: BindingGeneration,
        timeout_ticks: u64,
    ) -> CoreResult<Self> {
        if timeout_ticks == 0 || listen.secure {
            return Err(crate::producer::producer_unsupported());
        }
        Ok(Self {
            listen,
            id,
            generation,
            timeout_ticks,
            security: None,
            assigned: None,
            listener: None,
            preparation: Preparation::Specialized,
            compilations: Cell::new(0),
            checks: Cell::new(0),
            bindings: Cell::new(0),
        })
    }

    /// Use the deployment's fixed shared listener for this registration's routes.
    /// Each exposed Thing acquires and releases its own route set at native start and close.
    pub fn with_listener(mut self, listener: &HttpFiniteListener) -> CoreResult<Self> {
        if self.assigned.is_some()
            || self.listen.address.port() == 0
            || !listener.compatible(self.listen, self.timeout_ticks)
        {
            return Err(crate::producer::producer_unsupported());
        }
        self.listener = Some(listener.clone());
        Ok(self)
    }

    /// A draft registration for a TD using a loopback base with port zero.
    /// Its mappings cannot start a native route until bound to the acquired endpoint.
    pub fn for_assigned_port(
        ip: IpAddr,
        id: BindingId,
        generation: BindingGeneration,
        timeout_ticks: u64,
    ) -> CoreResult<Self> {
        if !ip.is_loopback() {
            return Err(crate::producer::producer_unsupported());
        }
        Self::try_new(
            HttpProducerConfig {
                address: SocketAddr::new(ip, 0),
                secure: false,
            },
            id,
            generation,
            timeout_ticks,
        )
    }

    /// Use the already acquired listener for the final endpoint and every
    /// selected route. Each registration keeps its original identity and policy.
    pub fn at_endpoint(&self, endpoint: &HttpProducerEndpoint) -> CoreResult<Self> {
        if self.listen.address.ip() != endpoint.listen.address.ip()
            || self.listen.secure
            || self.listener.is_some()
            || !matches!(self.preparation, Preparation::Specialized)
        {
            return Err(crate::producer::producer_unsupported());
        }
        let mut actual = Self::try_new(
            endpoint.listen,
            self.id,
            self.generation,
            self.timeout_ticks,
        )?;
        actual.assigned = Some(endpoint.socket.clone());
        Ok(actual)
    }

    /// Live verification/key service is paired with this startup registration;
    /// definitions retain only immutable scheme meaning and native operands.
    pub fn try_https(
        listen: HttpProducerConfig,
        id: BindingId,
        generation: BindingGeneration,
        timeout_ticks: u64,
        security: HttpServerSecurity,
    ) -> CoreResult<Self> {
        if !listen.secure || timeout_ticks == 0 {
            return Err(crate::producer::producer_unsupported());
        }
        Ok(Self {
            listen,
            id,
            generation,
            timeout_ticks,
            security: Some(security),
            assigned: None,
            listener: None,
            preparation: Preparation::Specialized,
            compilations: Cell::new(0),
            checks: Cell::new(0),
            bindings: Cell::new(0),
        })
    }

    /// Explicitly lower reusable NoSec HTTP Property Read instructions.
    /// The original bank pays for physical backing until the final reference.
    pub fn compiling(
        listen: HttpProducerConfig,
        id: BindingId,
        generation: BindingGeneration,
        timeout_ticks: u64,
        accounts: Arc<HostResourceAccounts>,
    ) -> CoreResult<Self> {
        let mut provider = Self::try_new(listen, id, generation, timeout_ticks)?;
        provider.preparation = Preparation::Compile(accounts);
        Ok(provider)
    }

    /// Check complete supported meaning and bind fresh native route operands.
    /// A changed dependency refuses; this capability never falls back to lowering.
    pub fn reusing(
        listen: HttpProducerConfig,
        id: BindingId,
        generation: BindingGeneration,
        timeout_ticks: u64,
        definition: &HttpProducerDefinition,
    ) -> CoreResult<Self> {
        let mut provider = Self::try_new(listen, id, generation, timeout_ticks)?;
        provider.preparation = Preparation::Bind(definition.clone());
        Ok(provider)
    }
    /// Actual instruction lowerings, including failed attempts and support probes.
    pub fn native_compilations(&self) -> usize {
        self.compilations.get()
    }
    pub fn dependency_checks(&self) -> usize {
        self.checks.get()
    }
    /// Successful preparation bindings; final/support checks are not new instances.
    pub fn mapping_bindings(&self) -> usize {
        self.bindings.get()
    }
    fn build_binding(
        &self,
        mut config: HttpFiniteConfig,
    ) -> CoreResult<(HttpFiniteBinding, HttpFiniteConfig)> {
        if let Some(security) = &self.security {
            config = config.with_security(security.clone())?;
        }
        if let Some(listener) = &self.listener {
            config = config.with_listener(listener)?;
        }
        let mut native = HttpFiniteBinding::new(config.clone(), self.id, self.generation);
        if let Some(socket) = &self.assigned {
            native.use_assigned(socket.clone());
        }
        Ok((native, config))
    }

    fn compile_definition(
        &self,
        input: ProducerOperationInput<'_>,
        budget: &mut WorkBudget,
        expected: Option<&HttpProducerProgram>,
    ) -> CoreResult<Option<HttpProducerProgram>> {
        let property = match (input.site, input.operation) {
            (FormSite::Property(p), Operation::ReadProperty | Operation::WriteProperty) => {
                p.as_str()
            }
            (
                FormSite::Thing,
                Operation::ReadAllProperties
                | Operation::ReadMultipleProperties
                | Operation::WriteAllProperties
                | Operation::WriteMultipleProperties,
            ) => "",
            _ => return Ok(None),
        };
        if Target::parse_port(
            input.target,
            self.listen.secure,
            self.listen.address.port() == 0,
        )
        .map_or(true, |target| target.address != self.listen.address)
        {
            return Ok(None);
        }
        if !matches!(self.preparation, Preparation::Specialized)
            && (!matches!(input.site, FormSite::Property(_))
                || input.operation != Operation::ReadProperty)
        {
            return Err(crate::producer::producer_unsupported());
        }
        if self.listen.secure && self.security.as_ref().is_none_or(|s| !s.is_ready()) {
            return Err(crate::producer::producer_unsupported());
        }
        // A supplied definition is an actual capability dependency, including
        // at final qualification. An unrelated expected program cannot make a
        // currently incompatible registration claim support.
        let original = match &self.preparation {
            Preparation::Bind(definition) => Some(&definition.0),
            _ => expected.map(|p| &p.instructions),
        };
        if original.is_some() {
            self.checks.set(self.checks.get() + 1);
        }
        let accounts = match &self.preparation {
            Preparation::Compile(accounts) => Some(accounts),
            _ => None,
        };
        let mut program = self
            .listen
            .compile_mapping_shared(
                clinkz_wot_planning::ResolvedBindingInput {
                    selected_security: None,
                    document: input.document,
                    target: input.target,
                    property,
                    form_index: input.form,
                    operation: input.operation,
                    owner: input.site.owner(),
                    content_type: None,
                    subprotocol: None,
                },
                budget,
                original,
                accounts,
                Some(&self.compilations),
            )?
            .ok_or_else(|| {
                CoreError::Backpressure(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
            })?;
        if self
            .security
            .as_ref()
            .is_some_and(|service| !service.supports(program.instructions.authentication))
        {
            return Err(crate::producer::producer_unsupported());
        }
        program.security_registration = self.security.as_ref().map(|s| s.registration.clone());
        Ok(Some(program))
    }
}

impl FiniteProducerCompiler for HttpFiniteProducer {
    fn observe_configuration(&self, facts: &mut PublicationObserver<'_>) -> CoreResult<()> {
        ProducerCapability::observe_configuration(self, facts)
    }

    type Binding = HttpFiniteBinding;

    fn prepare(
        &self,
        input: ResolvedProducerInput<'_>,
        instance: ProducerInstanceKey,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<(HttpFiniteBinding, HttpFiniteConfig)>> {
        if !matches!(self.preparation, Preparation::Specialized) {
            return Err(crate::producer::producer_unsupported());
        }
        let property = input.property();
        let Some(program) = self.listen.compile_property_read(input, budget)? else {
            return Ok(None);
        };
        let config = HttpFiniteConfig::try_new(
            self.listen,
            program,
            instance,
            clinkz_wot_core::AffordanceTarget::Property(Arc::from(property)),
            self.timeout_ticks,
        )
        .map_err(|_| crate::producer::producer_unsupported())?;
        self.build_binding(config).map(Some)
    }

    fn check_final(
        &self,
        input: ResolvedProducerInput<'_>,
        instance: ProducerInstanceKey,
        binding: &HttpFiniteBinding,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<()>> {
        // Reconstruct exact supported meaning using the actual compiler, not a
        // tag/hash match. This is pure; the temporary participant opens no I/O.
        let Some((_, config)) = FiniteProducerCompiler::prepare(self, input, instance, budget)?
        else {
            return Ok(None);
        };
        if binding.identity() != (self.id, self.generation)
            || binding.configuration() != &config
            || binding.bound_address() != Some(config.listen_address())
        {
            return Err(CoreError::Validation(ErrorContext::new(
                ErrorPhase::Admission,
                RetryClass::Never,
            )));
        }
        Ok(Some(()))
    }

    fn retained_bytes(&self) -> usize {
        // The snapshot shares the native property backing; charge the complete
        // bounded Arc allocation here until all participants release it.
        256 + core::mem::size_of::<(AffordanceTarget, HttpProducerProgram)>()
            + 6 * core::mem::size_of::<usize>()
            + 64
    }
}

// This concrete participant supports HTTP Property Read/Write and the native
// Thing-root finite collection mappings. The composition selector asks it about
// each actual Form/operation; it interprets no Action/Event lifecycle.
impl ProducerCapability for HttpFiniteProducer {
    fn observe_configuration(&self, facts: &mut PublicationObserver<'_>) -> CoreResult<()> {
        self.security
            .as_ref()
            .map_or(Ok(()), |security| security.observe_configuration(facts))
    }

    fn identity(&self) -> (BindingId, BindingGeneration) {
        (self.id, self.generation)
    }
    fn definition_bytes(&self, definition: &dyn Any) -> Option<usize> {
        definition
            .downcast_ref::<HttpProducerProgram>()
            .map(|p| core::mem::size_of::<HttpProducerProgram>() + p.schema_backing_bytes())
    }
    fn prepare(
        &self,
        input: ProducerOperationInput<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<Box<dyn Any>>> {
        let program = self.compile_definition(input, budget, None)?;
        if program.is_some() {
            self.bindings.set(self.bindings.get() + 1);
        }
        Ok(program.map(|program| Box::new(program) as Box<dyn Any>))
    }
    fn supports(
        &self,
        input: ProducerOperationInput<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<bool> {
        Ok(self.compile_definition(input, budget, None)?.is_some())
    }
    fn final_check_temporary_bytes(&self) -> Option<usize> {
        // Cover the bounded URI/program and collection member/name backing used
        // by a sequential pure support/final-check callback, including its Arc.
        // No replacement definition survives and no native setup occurs.
        Some(
            clinkz_wot_planning::security::TEMPORARY_BYTES as usize
                + crate::producer_compiler::definition_bound()
                + 8192,
        )
    }
    fn check_final(
        &self,
        input: ProducerOperationInput<'_>,
        definition: &dyn Any,
        budget: &mut WorkBudget,
    ) -> CoreResult<bool> {
        let Some(expected) = definition.downcast_ref::<HttpProducerProgram>() else {
            return Ok(false);
        };
        Ok(self
            .compile_definition(input, budget, Some(expected))?
            .as_ref()
            == Some(expected))
    }
}

/// A candidate's closed socket. A selected participant takes the original
/// listener at native start; rollback drops an unclaimed socket locally.
pub struct HttpProducerEndpoint {
    listen: HttpProducerConfig,
    socket: AssignedSocket,
    base: String,
}
impl HttpProducerEndpoint {
    pub fn acquire(address: SocketAddr) -> CoreResult<Self> {
        if !address.ip().is_loopback()
            || matches!(address, SocketAddr::V6(v6) if v6.flowinfo() != 0 || v6.scope_id() != 0)
        {
            return Err(crate::producer::producer_unsupported());
        }
        let failed =
            |_| CoreError::Binding(ErrorContext::new(ErrorPhase::Prepare, RetryClass::Never));
        let socket = std::net::TcpListener::bind(address).map_err(failed)?;
        socket.set_nonblocking(true).map_err(failed)?;
        let actual = socket.local_addr().map_err(failed)?;
        Ok(Self {
            listen: HttpProducerConfig::for_loopback(actual)?,
            socket: Arc::new(std::sync::Mutex::new(Some(socket))),
            base: format!("http://{actual}/"),
        })
    }
}
impl AsRef<str> for HttpProducerEndpoint {
    fn as_ref(&self) -> &str {
        &self.base
    }
}
impl Drop for HttpProducerEndpoint {
    fn drop(&mut self) {
        self.socket.lock().unwrap().take();
    }
}

impl FiniteProducerService for HttpFiniteProducer {
    type Binding = HttpFiniteBinding;
    fn setup_peak_bytes(&self) -> usize {
        // At most 32 selected native routes, with both Vec and Arc layouts
        // present at conversion. Instruction/schema backing is shared.
        self.final_check_temporary_bytes()
            .expect("bounded HTTP preparation")
            + MAX_FINITE_MAPPINGS
                * (2 * size_of::<(AffordanceTarget, HttpProducerProgram)>()
                    + 256
                    + 2 * size_of::<usize>())
    }

    fn assemble<'a>(
        &self,
        instance: ProducerInstanceKey,
        operations: impl Iterator<Item = &'a PreparedOperation>,
    ) -> CoreResult<(HttpFiniteBinding, HttpFiniteConfig)> {
        let mut mappings = Vec::new();
        for op in operations {
            if mappings.len() == MAX_FINITE_MAPPINGS {
                return Err(crate::producer::producer_unsupported());
            }
            let target = match &op.site {
                FormSite::Thing => AffordanceTarget::Thing,
                FormSite::Property(name) if name.len() <= 256 => {
                    AffordanceTarget::Property(Arc::from(name.as_str()))
                }
                _ => return Err(crate::producer::producer_unsupported()),
            };
            let program = op
                .definition::<HttpProducerProgram>()
                .ok_or_else(crate::producer::producer_unsupported)?;
            mappings.push((target, program.clone()));
        }
        let config = HttpFiniteConfig::try_from_mappings(
            self.listen,
            mappings,
            instance,
            self.timeout_ticks,
        )
        .map_err(|r| r.into_parts().1.into_parts().1)?;
        self.build_binding(config)
    }
}
