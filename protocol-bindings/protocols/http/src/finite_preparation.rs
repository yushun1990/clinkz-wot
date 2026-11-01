//! Optional preparation of the direct finite participant, without the legacy
//! Producer executor/installation interface.
use crate::*;
use clinkz_wot_core::{finite::FiniteIngressBinding, producer::ProducerInstanceKey};
use clinkz_wot_planning::{
    ResolvedProducerInput,
    finite_producer::FiniteProducerCompiler,
    producer_composition::{FormSite, ProducerCapability, ProducerOperationInput},
};
use std::any::Any;

/// One immutable startup registration. Explicit loopback/nonzero endpoints
/// remain fixed in this first slice; port-0 rewriting is deliberately unsupported.
pub struct HttpFiniteProducer {
    listen: HttpProducerConfig,
    id: BindingId,
    generation: BindingGeneration,
    timeout_ticks: u64,
}

impl HttpFiniteProducer {
    pub fn try_new(
        listen: HttpProducerConfig,
        id: BindingId,
        generation: BindingGeneration,
        timeout_ticks: u64,
    ) -> CoreResult<Self> {
        if timeout_ticks == 0 {
            return Err(crate::producer::producer_unsupported());
        }
        Ok(Self {
            listen,
            id,
            generation,
            timeout_ticks,
        })
    }

    fn compile_definition(
        &self,
        input: ProducerOperationInput<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<HttpProducerProgram>> {
        let property = match (input.site, input.operation) {
            (FormSite::Property(p), Operation::ReadProperty | Operation::WriteProperty) => {
                p.as_str()
            }
            (FormSite::Thing, Operation::ReadAllProperties) => "",
            _ => return Ok(None),
        };
        if Target::parse(input.target).map_or(true, |target| target.address != self.listen.address)
        {
            return Ok(None);
        }
        self.listen
            .compile_mapping(
                clinkz_wot_planning::ResolvedBindingInput {
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
            )?
            .map(Some)
            .ok_or_else(|| {
                CoreError::Backpressure(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
            })
    }
}

impl FiniteProducerCompiler for HttpFiniteProducer {
    type Binding = HttpFiniteBinding;

    fn prepare(
        &self,
        input: ResolvedProducerInput<'_>,
        instance: ProducerInstanceKey,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<(HttpFiniteBinding, HttpFiniteConfig)>> {
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
// Thing-root ReadAllProperties mapping. The composition selector asks it about
// each actual Form/operation; it interprets no Action/Event lifecycle.
impl ProducerCapability for HttpFiniteProducer {
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
        Ok(self
            .compile_definition(input, budget)?
            .map(|program| Box::new(program) as Box<dyn Any>))
    }
    fn supports(
        &self,
        input: ProducerOperationInput<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<bool> {
        Ok(self.compile_definition(input, budget)?.is_some())
    }
    fn final_check_temporary_bytes(&self) -> Option<usize> {
        // Cover the bounded URI/program and collection member/name backing used
        // by a sequential pure support/final-check callback, including its Arc.
        // No replacement definition survives and no native setup occurs.
        Some(16_384)
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
        Ok(self.compile_definition(input, budget)?.as_ref() == Some(expected))
    }
}
