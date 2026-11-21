//! Optional preparation of the direct finite participant, without the legacy
//! Producer executor/installation interface.
use crate::*;
use clinkz_wot_core::host_accounts::HostResourceAccounts;
use clinkz_wot_core::{finite::FiniteIngressBinding, producer::ProducerInstanceKey};
use clinkz_wot_planning::{
    ResolvedProducerInput,
    finite_producer::FiniteProducerCompiler,
    producer_composition::{
        FormSite, ProducerCapability, ProducerOperationInput, PublicationObserver,
    },
};
use std::{any::Any, cell::Cell};

enum Preparation {
    Specialized,
    Compile(Arc<HostResourceAccounts>),
    Bind(HttpProducerDefinition),
}

/// One immutable startup registration. Explicit loopback/nonzero endpoints
/// remain fixed in this first slice; port-0 rewriting is deliberately unsupported.
pub struct HttpFiniteProducer {
    listen: HttpProducerConfig,
    id: BindingId,
    generation: BindingGeneration,
    timeout_ticks: u64,
    security: Option<HttpServerSecurity>,
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
            preparation: Preparation::Specialized,
            compilations: Cell::new(0),
            checks: Cell::new(0),
            bindings: Cell::new(0),
        })
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
        if Target::parse_for(input.target, self.listen.secure)
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
        Ok(Some((
            HttpFiniteBinding::new(config.clone(), self.id, self.generation),
            config,
        )))
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
