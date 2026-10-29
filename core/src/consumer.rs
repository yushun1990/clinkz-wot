//! Source-independent finite Consumer definitions and captured services.
//! A definition has no compiler, source TD, semantic arena or parser callback.
use crate::host_accounts::{HostCapacityReservation, HostResourceAccounts};
use crate::value_type::TypeSchema;
use crate::*;
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use clinkz_wot_foundation::{ResourceKind, WorkBudget, WorkClass};

/// Actual codec and type-validation implementation. Preparation checks the
/// entire schema subset; this service validates bytes on every normal/late result.
/// Callbacks are trusted, pure and bounded by their captured declared policy.
/// Metadata/support/availability queries are bounded allocation-free passive
/// inspection; they acquire no credentials, source documents or native I/O.
pub trait ConsumerDataService: Send + Sync {
    fn content_type(&self) -> &str;
    fn validate(&self, schema: TypeSchema, output: &InteractionOutput) -> CoreResult<()>;
    fn policy(&self) -> ConsumerDataPolicy;
    /// Complete support of the already prepared type-only assertion.
    fn supports_schema(&self, _schema: TypeSchema) -> bool {
        true
    }
    fn available(&self) -> bool {
        true
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConsumerDataPolicy {
    pub retained_bytes: u64,
    pub temporary_bytes: u64,
    pub body_bytes: u64,
    /// Complete maximum callback/traversal work, prepaid before validation.
    pub validation_work: u64,
}

/// Actual destination/authorization policy, selected before compilation. NoSec
/// means no protocol credential, not permission to contact every destination.
pub trait ConsumerAccessService: Send + Sync {
    fn authorize(
        &self,
        binding: BindingRegistrationIdentity,
        target: &str,
        operation: Operation,
    ) -> CoreResult<()>;
    fn retained_bytes(&self) -> u64;
    fn work(&self) -> u64;
}

#[derive(Clone)]
pub struct ConsumerServices {
    pub data: Arc<dyn ConsumerDataService>,
    pub access: Arc<dyn ConsumerAccessService>,
}

/// One immutable selected executor, native definition, actual services and
/// original resource leases. Constructor refusal returns all original owners.
/// This carrier does not itself perform TD support assessment.
pub struct ConsumerDefinition {
    name: Arc<str>,
    form: u32,
    target: Box<str>,
    artifact: BindingArtifactEnvelope<HostBindingArtifact>,
    execution: Arc<HostConsumerBindingRegistration>,
    configuration: ConsumerBindingConfiguration,
    services: ConsumerServices,
    schema: TypeSchema,
    data_policy: ConsumerDataPolicy,
    access_work: u64,
    // Last to drop: artifact destructors, executor/services and backing are
    // disposed before their original funding returns to the parent bank.
    _funding: Vec<HostCapacityReservation>,
    bank: Arc<HostResourceAccounts>,
}
pub struct ConsumerDefinitionInput {
    pub name: Arc<str>,
    pub form: u32,
    pub target: Box<str>,
    pub artifact: BindingArtifactEnvelope<HostBindingArtifact>,
    pub execution: Arc<HostConsumerBindingRegistration>,
    pub configuration: ConsumerBindingConfiguration,
    pub services: ConsumerServices,
    pub schema: TypeSchema,
    pub funding: Vec<HostCapacityReservation>,
    pub bank: Arc<HostResourceAccounts>,
}
impl ConsumerDefinition {
    pub fn try_new(
        input: ConsumerDefinitionInput,
    ) -> Result<Self, BindingInputRejection<ConsumerDefinitionInput>> {
        let id = input.artifact.identity();
        let actual = input.execution.identity();
        let policy = input.services.data.policy();
        if id.binding_id() != actual.binding_id()
            || id.binding_generation() != actual.binding_generation()
            || id.configuration() != actual.configuration()
            || id.compatibility() != actual.artifact_compatibility()
            || id.role() != BindingArtifactRole::ConsumerCall
            || input.execution.consumer_configuration() != Some(input.configuration)
            || !input.services.data.available()
            || !input.services.data.supports_schema(input.schema)
            || policy.validation_work == 0
            || input.services.access.work() == 0
        {
            return Err(BindingInputRejection::new(
                input,
                BindingOperationalError::new(invalid()),
            ));
        }
        let access_work = input.services.access.work();
        Ok(Self {
            name: input.name,
            form: input.form,
            target: input.target,
            artifact: input.artifact,
            execution: input.execution,
            configuration: input.configuration,
            services: input.services,
            schema: input.schema,
            data_policy: policy,
            access_work,
            _funding: input.funding,
            bank: input.bank,
        })
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn form(&self) -> u32 {
        self.form
    }
    pub fn binding(&self) -> BindingRegistrationIdentity {
        self.execution.identity()
    }
    pub fn artifact(&self) -> &BindingArtifactEnvelope<HostBindingArtifact> {
        &self.artifact
    }
    pub fn resources(&self) -> BindingResourceDeclarations {
        self.execution.resources()
    }
    pub fn configuration(&self) -> ConsumerBindingConfiguration {
        self.configuration
    }
    pub fn bank(&self) -> &Arc<HostResourceAccounts> {
        &self.bank
    }
    pub fn data_policy(&self) -> ConsumerDataPolicy {
        self.data_policy
    }
    pub fn access_work(&self) -> u64 {
        self.access_work
    }
    /// Consuming handoff for an execution owner. A data carrier is not an
    /// execution API or support certificate; mutable parts are never lent.
    pub fn into_parts(self) -> (ConsumerDefinitionInput, ConsumerDataPolicy, u64) {
        (
            ConsumerDefinitionInput {
                name: self.name,
                form: self.form,
                target: self.target,
                artifact: self.artifact,
                execution: self.execution,
                configuration: self.configuration,
                services: self.services,
                schema: self.schema,
                funding: self._funding,
                bank: self.bank,
            },
            self.data_policy,
            self.access_work,
        )
    }
}

/// Atomic prepayment of one cooperative callback. Short credit never invokes it.
pub fn pay(budget: &mut WorkBudget, class: WorkClass, amount: u64) -> bool {
    budget.consume(class, amount).is_ok()
}
/// Reuse the original bank; failure rolls back already acquired leases.
pub fn reserve(
    bank: &Arc<HostResourceAccounts>,
    amounts: &[(ResourceKind, u64)],
) -> CoreResult<Vec<HostCapacityReservation>> {
    let mut leases = Vec::new();
    leases
        .try_reserve_exact(amounts.len())
        .map_err(|_| busy())?;
    for &(kind, amount) in amounts {
        leases.push(
            bank.reserve(kind, amount)
                .ok_or_else(|| CoreError::LimitExceeded {
                    resource: kind,
                    limit: bank.account(kind).limit(),
                    requested: Some(amount),
                    observed: Some(bank.account(kind).used()),
                    context: context(),
                })?,
        );
    }
    Ok(leases)
}
pub fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
}
pub fn busy() -> CoreError {
    CoreError::Backpressure(context())
}
pub fn invalid() -> CoreError {
    CoreError::Validation(context())
}
