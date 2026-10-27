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
            Arc::from(property),
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
        256 + 2 * core::mem::size_of::<usize>()
    }
}

// This concrete participant supports only HTTP Property Read. The general
// composition selector asks it about *each* Form/operation; it owns no
// interpretation of Action, Event or collection semantics.
impl ProducerCapability for HttpFiniteProducer {
    fn identity(&self) -> (BindingId, BindingGeneration) {
        (self.id, self.generation)
    }
    fn definition_bytes(&self, definition: &dyn Any) -> Option<usize> {
        definition
            .downcast_ref::<HttpProducerProgram>()
            .map(|_| core::mem::size_of::<HttpProducerProgram>())
    }
    fn prepare(
        &self,
        input: ProducerOperationInput<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<Box<dyn Any>>> {
        let FormSite::Property(property) = input.site else {
            return Ok(None);
        };
        if input.operation != Operation::ReadProperty {
            return Ok(None);
        }
        // A registration at another listen address is not a failed selected
        // mapping. Let the selector try the other linked HTTP registration;
        // only a matching registration may reject unsupported TD meaning.
        if Target::parse(input.target).map_or(true, |target| target.address != self.listen.address)
        {
            return Ok(None);
        }
        let input = ResolvedProducerInput::new(
            input.document,
            input.target,
            property,
            input.form,
            input.operation,
        );
        Ok(self
            .listen
            .compile_property_read(input, budget)?
            .map(|program| Box::new(program) as Box<dyn Any>))
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
        let Some(program) = <Self as ProducerCapability>::prepare(self, input, budget)? else {
            return Ok(false);
        };
        Ok(program.downcast_ref::<HttpProducerProgram>() == Some(expected))
    }
}
