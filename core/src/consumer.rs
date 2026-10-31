//! Source-independent finite Consumer definitions and captured services.
//! A definition has no compiler, source TD, semantic arena or parser callback.
use crate::host_accounts::{HostCapacityReservation, HostResourceAccounts};
use crate::value_type::TypeSchema;
use crate::*;
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use clinkz_wot_foundation::{ResourceKind, WorkBudget, WorkClass};
use core::sync::atomic::{AtomicBool, Ordering};

/// Actual codec and type-validation implementation. Preparation checks the
/// entire schema subset; this service validates bytes on every normal/late result.
/// Callbacks are trusted, pure and bounded by their captured declared policy.
/// Metadata/support/availability queries are bounded allocation-free passive
/// inspection; they acquire no credentials, source documents or native I/O.
pub trait ConsumerDataService: Send + Sync {
    fn content_type(&self) -> &str;
    fn validate(&self, schema: TypeSchema, payload: &Payload) -> CoreResult<()>;
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
    /// Fresh application permission for this original logical incarnation.
    /// Preparation's destination check alone grants no execution authority.
    fn authorize_instance(
        &self,
        consumer: &ConsumerInstanceRef,
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

/// One local logical Consumer authority, independent of any capability, TD or
/// physical connection. Startup construction is caller-funded Host work.
/// Drop/revoke is immediate and allocation/callback free; there is no reactivation.
///
/// ```compile_fail
/// fn duplicate(instance: clinkz_wot_core::consumer::ConsumerInstance) {
///     let _second_owner = instance.clone();
/// }
/// ```
pub struct ConsumerInstance(ConsumerInstanceRef);
impl ConsumerInstance {
    pub fn new(slot: ThingSlotId) -> Self {
        Self(ConsumerInstanceRef {
            slot,
            current: Arc::new(AtomicBool::new(true)),
        })
    }
    pub fn reference(&self) -> ConsumerInstanceRef {
        self.0.clone()
    }
    pub fn revoke(&self) {
        self.0.current.store(false, Ordering::Release);
    }
}
impl Drop for ConsumerInstance {
    fn drop(&mut self) {
        self.revoke();
    }
}
/// Retention/identity and passive currentness only. Cloning cannot revoke the
/// instance, keep it current after its owner disappears, or shut down native I/O.
#[derive(Clone)]
pub struct ConsumerInstanceRef {
    slot: ThingSlotId,
    current: Arc<AtomicBool>,
}
impl ConsumerInstanceRef {
    pub fn slot(&self) -> ThingSlotId {
        self.slot
    }
    pub fn is_current(&self) -> bool {
        self.current.load(Ordering::Acquire)
    }
}
impl PartialEq for ConsumerInstanceRef {
    fn eq(&self, other: &Self) -> bool {
        self.slot == other.slot && Arc::ptr_eq(&self.current, &other.current)
    }
}
impl Eq for ConsumerInstanceRef {}
impl core::fmt::Debug for ConsumerInstanceRef {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ConsumerInstanceRef")
            .field("slot", &self.slot)
            .finish_non_exhaustive()
    }
}

/// Distinct finite application-data roles. None means no application payload;
/// an unconstrained present payload uses Some(TypeSchema::Any).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConsumerSchemas {
    pub input: Option<TypeSchema>,
    pub output: Option<TypeSchema>,
}

/// A Property's value schema is not Event data. Event control/response roles
/// stay distinct; native SUBACK/UNSUBACK packets never become application data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsumerNotificationSchemas {
    PropertyValue(TypeSchema),
    Event {
        subscription: Option<TypeSchema>,
        data: Option<TypeSchema>,
        data_response: Option<TypeSchema>,
        cancellation: Option<TypeSchema>,
    },
}
impl ConsumerNotificationSchemas {
    pub fn kind(self) -> crate::notification::NotificationKind {
        match self {
            Self::PropertyValue(_) => crate::notification::NotificationKind::PropertyObservation,
            Self::Event { .. } => crate::notification::NotificationKind::Event,
        }
    }
    pub fn subscription(self) -> Option<TypeSchema> {
        match self {
            Self::Event { subscription, .. } => subscription,
            _ => None,
        }
    }
    pub fn data(self) -> Option<TypeSchema> {
        match self {
            Self::PropertyValue(s) => Some(s),
            Self::Event { data, .. } => data,
        }
    }
    pub fn data_response(self) -> Option<TypeSchema> {
        match self {
            Self::Event { data_response, .. } => data_response,
            _ => None,
        }
    }
    pub fn cancellation(self) -> Option<TypeSchema> {
        match self {
            Self::Event { cancellation, .. } => cancellation,
            _ => None,
        }
    }
}

/// Source-free typed notification handoff. The actual linked Binding checks
/// its owned program/configuration; numeric tags alone grant no capability.
/// Field order retains original funding until every executable owner is gone.
pub struct ConsumerNotificationDefinition<B: crate::notification::ConsumerNotificationBinding> {
    input: ConsumerNotificationDefinitionInput<B>,
}
pub struct ConsumerNotificationDefinitionInput<B: crate::notification::ConsumerNotificationBinding>
{
    pub binding: B,
    pub program: B::Program,
    pub configuration: B::Configuration,
    pub identity: BindingRegistrationIdentity,
    pub target: AffordanceTarget,
    pub endpoint: Box<str>,
    pub form: u32,
    pub schemas: ConsumerNotificationSchemas,
    pub services: ConsumerServices,
    pub funding: Vec<HostCapacityReservation>,
    pub bank: Arc<HostResourceAccounts>,
}
impl<B: crate::notification::ConsumerNotificationBinding> ConsumerNotificationDefinition<B> {
    /// Trusted source-free construction, not a TD support certificate. Refusal
    /// returns every original owner; successful definitions expose no mutation.
    pub fn try_new(
        input: ConsumerNotificationDefinitionInput<B>,
    ) -> Result<Self, BindingInputRejection<ConsumerNotificationDefinitionInput<B>>> {
        let check = (|| {
            if input.binding.identity()
                != (
                    input.identity.binding_id(),
                    input.identity.binding_generation(),
                )
                || input.binding.configuration() != &input.configuration
                || crate::notification::NotificationKind::for_target(&input.target)
                    != Some(input.schemas.kind())
                || !input.services.data.available()
                || input.services.data.policy().validation_work == 0
                || input.services.access.work() == 0
                || ![
                    input.schemas.subscription(),
                    input.schemas.data(),
                    input.schemas.data_response(),
                    input.schemas.cancellation(),
                ]
                .into_iter()
                .flatten()
                .all(|s| input.services.data.supports_schema(s))
            {
                return Err(invalid());
            }
            input
                .binding
                .support(&input.program, input.schemas.kind())?;
            Ok(())
        })();
        match check {
            Ok(()) => Ok(Self { input }),
            Err(e) => Err(BindingInputRejection::new(
                input,
                BindingOperationalError::new(e),
            )),
        }
    }
    pub fn input(&self) -> &ConsumerNotificationDefinitionInput<B> {
        &self.input
    }
    pub fn into_input(self) -> ConsumerNotificationDefinitionInput<B> {
        self.input
    }
}

/// Conservative remote-operation certainty, independent of local cancellation
/// and validated result delivery. Any native progress may cross the effect edge;
/// failure or cancellation afterwards cannot authorize implicit replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsumerEffect {
    NotAttempted,
    Unknown,
}

/// One immutable selected executor, native definition, actual services and
/// original resource leases. Constructor refusal returns all original owners.
/// This carrier does not itself perform TD support assessment.
pub struct ConsumerDefinition {
    affordance: AffordanceTarget,
    operation: Operation,
    form: u32,
    target: Box<str>,
    artifact: BindingArtifactEnvelope<HostBindingArtifact>,
    execution: Arc<HostConsumerBindingRegistration>,
    configuration: ConsumerBindingConfiguration,
    services: ConsumerServices,
    schemas: ConsumerSchemas,
    data_policy: ConsumerDataPolicy,
    access_work: u64,
    // Last to drop: artifact destructors, executor/services and backing are
    // disposed before their original funding returns to the parent bank.
    _funding: Vec<HostCapacityReservation>,
    bank: Arc<HostResourceAccounts>,
}
pub struct ConsumerDefinitionInput {
    pub affordance: AffordanceTarget,
    pub operation: Operation,
    pub form: u32,
    pub target: Box<str>,
    pub artifact: BindingArtifactEnvelope<HostBindingArtifact>,
    pub execution: Arc<HostConsumerBindingRegistration>,
    pub configuration: ConsumerBindingConfiguration,
    pub services: ConsumerServices,
    pub schemas: ConsumerSchemas,
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
            || !input
                .schemas
                .input
                .into_iter()
                .chain(input.schemas.output)
                .all(|s| input.services.data.supports_schema(s))
            || !matches!(
                (&input.affordance, input.operation),
                (
                    AffordanceTarget::Property(_),
                    Operation::ReadProperty | Operation::WriteProperty
                ) | (AffordanceTarget::Action(_), Operation::InvokeAction)
            )
            || (input.operation == Operation::ReadProperty
                && (input.schemas.input.is_some() || input.schemas.output.is_none()))
            || (input.operation == Operation::WriteProperty
                && (input.schemas.input.is_none() || input.schemas.output.is_some()))
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
            affordance: input.affordance,
            operation: input.operation,
            form: input.form,
            target: input.target,
            artifact: input.artifact,
            execution: input.execution,
            configuration: input.configuration,
            services: input.services,
            schemas: input.schemas,
            data_policy: policy,
            access_work,
            _funding: input.funding,
            bank: input.bank,
        })
    }
    pub fn name(&self) -> &str {
        self.affordance.name().unwrap()
    }
    pub fn affordance(&self) -> &AffordanceTarget {
        &self.affordance
    }
    pub fn operation(&self) -> Operation {
        self.operation
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
                affordance: self.affordance,
                operation: self.operation,
                form: self.form,
                target: self.target,
                artifact: self.artifact,
                execution: self.execution,
                configuration: self.configuration,
                services: self.services,
                schemas: self.schemas,
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
