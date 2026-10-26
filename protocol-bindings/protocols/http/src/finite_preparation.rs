//! Optional preparation of the direct finite participant, without the legacy
//! Producer executor/installation interface.
use crate::*;
use clinkz_wot_core::{finite::FiniteIngressBinding, producer::ProducerInstanceKey};
use clinkz_wot_planning::{ResolvedProducerInput, finite_producer::FiniteProducerCompiler};

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
        let Some((_, config)) = self.prepare(input, instance, budget)? else {
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
