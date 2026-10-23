//! Pure typed Producer pairing and completed preparation. No native effects.
// Refusal preserves complete inline inputs; boxing them could itself fail.
#![allow(clippy::result_large_err)]
use crate::{ProducerPropertyReadAdmission, ResolvedProducerInput};
use clinkz_wot_core::{producer::*, *};
use clinkz_wot_foundation::{WorkBudget, WorkClass};

/// Open native lowering for the matching executor type. Configuration means
/// the complete concrete policy/operands, not merely a format or syntax tag.
pub trait ProducerCompiler<E: ProducerExecutor> {
    /// Passive, allocation-free view of the complete concrete configuration.
    /// Pair construction checks a separately provisioned owned snapshot;
    /// funded preparation rechecks borrow this view without reconstructing it.
    fn configuration(&self) -> &E::Configuration;
    fn compile(
        &self,
        input: ResolvedProducerInput<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<E::Program>>;
}

/// Captures the actual executor. There is no mutable/extractable executor loan
/// while the pair is live. Compilers are destroyed before runtime admission.
pub struct ProducerBindingPreparation<C, E: ProducerExecutor> {
    compiler: C,
    executor: E,
    configuration: E::Configuration,
    identity: BindingRegistrationIdentity,
    policy: ProducerInstallPolicy,
    service: Option<ProducerServicePolicy>,
}

impl<C: ProducerCompiler<E>, E: ProducerExecutor> ProducerBindingPreparation<C, E> {
    /// Checks and moves the three caller-provisioned inputs, with no new
    /// configuration allocation/copy. Refusal returns all of them unchanged.
    /// Input provisioning is the caller's responsibility; retained configuration
    /// backing still participates in the later funded runtime admission.
    pub fn try_new(
        compiler: C,
        executor: E,
        configuration: E::Configuration,
    ) -> Result<Self, BindingInputRejection<(C, E, E::Configuration)>> {
        if compiler.configuration() != &configuration
            || !executor.configuration_matches(&configuration)
        {
            return Err(refused((compiler, executor, configuration)));
        }
        let identity = executor.identity();
        let policy = executor.install_policy();
        let service = executor.service_policy();
        Ok(Self {
            compiler,
            executor,
            configuration,
            identity,
            policy,
            service,
        })
    }

    /// Requires full draft coverage and the exact *registered* capability.
    /// On refusal the caller retains both actual inputs. Insufficient step
    /// credit likewise returns the pair and handler, with no installation.
    /// One BindingPolls credit funds the passive pair rechecks before any
    /// compiler/executor query or semantic loan; lowering uses its own budget.
    #[allow(clippy::type_complexity)]
    pub fn prepare_property_read<H: ReadPropertyHandler + Send + 'static>(
        self,
        admission: &mut ProducerPropertyReadAdmission<'_, '_>,
        authority: &ProducerInstanceAuthority,
        handler: ProducerHandlerRegistration<H>,
        budget: &mut WorkBudget,
    ) -> Result<PreparedProducer<E, H>, BindingInputRejection<(Self, ProducerHandlerRegistration<H>)>>
    {
        if handler.instance_key() != authority.key() {
            return Err(refused((self, handler)));
        }
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Err(BindingInputRejection::new(
                (self, handler),
                BindingOperationalError::new(CoreError::Backpressure(context())),
            ));
        }
        if !self.executor.configuration_matches(&self.configuration)
            || self.executor.identity() != self.identity
            || self.executor.install_policy() != self.policy
            || self.executor.service_policy() != self.service
            || self.compiler.configuration() != &self.configuration
        {
            return Err(refused((self, handler)));
        }
        let program = admission.with_input(budget, |input, budget| {
            if input.property() != handler.property() {
                return Err(pairing_error());
            }
            self.compiler.compile(input, budget)
        });
        let program = match program {
            Ok(Some(program)) => program,
            Ok(None) => {
                return Err(BindingInputRejection::new(
                    (self, handler),
                    BindingOperationalError::new(CoreError::Backpressure(context())),
                ));
            }
            Err(error) => {
                return Err(BindingInputRejection::new(
                    (self, handler),
                    BindingOperationalError::new(error),
                ));
            }
        };
        if let Err(error) = self.executor.check_program(&program) {
            return Err(BindingInputRejection::new(
                (self, handler),
                BindingOperationalError::new(error),
            ));
        }
        let (instance, property, handler, footprint) = handler.into_parts();
        let entry = ProducerExecutionEntry::new(instance, self.identity, property, program);
        // No compiler or source loan is retained in the completed value.
        Ok(PreparedProducer(ProducerExecutionRegistration::new(
            self.executor,
            self.configuration,
            self.policy,
            self.service,
            entry,
            handler,
            footprint,
        )))
    }
}

/// Privately completed semantic/native/capability pairing. Public runtime data
/// cannot be reassembled into this certificate after extraction or tampering.
///
/// ```compile_fail,E0423
/// use clinkz_wot_core::{producer::*, ReadPropertyHandler};
/// use clinkz_wot_planning::producer_execution::PreparedProducer;
/// fn forge<E: ProducerExecutor, H: ReadPropertyHandler>(value: ProducerExecutionRegistration<E,H>) -> PreparedProducer<E,H> {
///     PreparedProducer(value)
/// }
/// ```
pub struct PreparedProducer<E: ProducerExecutor, H: ReadPropertyHandler>(
    ProducerExecutionRegistration<E, H>,
);
impl<E: ProducerExecutor, H: ReadPropertyHandler> PreparedProducer<E, H> {
    pub fn into_execution(self) -> ProducerExecutionRegistration<E, H> {
        self.0
    }
}
impl<E: ProducerExecutor, H: ReadPropertyHandler> core::ops::Deref for PreparedProducer<E, H> {
    type Target = ProducerExecutionRegistration<E, H>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
}
fn pairing_error() -> CoreError {
    CoreError::Validation(
        context().with_redacted_cause(622, "Producer executor/capability pairing refused"),
    )
}
fn refused<T>(input: T) -> BindingInputRejection<T> {
    BindingInputRejection::new(input, BindingOperationalError::new(pairing_error()))
}
