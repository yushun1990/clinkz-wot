//! Source-independent finite Consumer definitions and captured services.
//! A definition has no compiler, source TD, semantic arena or parser callback.
use crate::collection::{PropertyCollectionSchema, PropertySelection};
use crate::host_accounts::{HostCapacityReservation, HostResourceAccounts};
use crate::value_type::TypeSchema;
use crate::*;
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use clinkz_wot_foundation::{ResourceKind, WorkBudget, WorkClass};
use core::sync::atomic::{AtomicBool, Ordering};

/// Actual codec and type-validation implementation. Preparation checks the
/// entire schema subset; this service validates input and normal/late result bytes.
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
    /// Complete native representation of a named Property map. Scalar/type
    /// support alone does not establish collection support.
    fn supports_property_map(&self, _schema: &PropertyCollectionSchema) -> bool {
        false
    }
    fn validate_property_map(
        &self,
        _schema: &PropertyCollectionSchema,
        _payload: &Payload,
    ) -> CoreResult<()> {
        Err(CoreError::UnsupportedOperation(context()))
    }
    /// Validate every supplied writable value and return exactly its member set.
    /// Generic Object or all-map support does not imply selected-write support.
    fn supports_property_write_selection(&self, _schema: &PropertyCollectionSchema) -> bool {
        false
    }
    fn validate_property_write_map(
        &self,
        _schema: &PropertyCollectionSchema,
        _payload: &Payload,
    ) -> CoreResult<PropertySelection> {
        Err(CoreError::UnsupportedOperation(context()))
    }
    /// Both the default TD name-array request and exact selected response must
    /// be executable. ReadAllProperties support does not imply this facet.
    /// The generic input-schema validator is qualified separately.
    fn supports_property_selection(&self, _schema: &PropertyCollectionSchema) -> bool {
        false
    }
    /// Encode precisely this set with the selected Form's media type. Pure,
    /// bounded by policy.body_bytes/temporary_bytes/validation_work. The caller
    /// reserves policy.encoded_selection_bytes of complete returned payload
    /// backing (body, Arc and media/coding capacities) before entry.
    fn encode_property_selection(
        &self,
        _schema: &PropertyCollectionSchema,
        _selection: PropertySelection,
    ) -> CoreResult<Payload> {
        Err(CoreError::UnsupportedOperation(context()))
    }
    fn validate_selected_property_map(
        &self,
        _schema: &PropertyCollectionSchema,
        _selection: PropertySelection,
        _payload: &Payload,
    ) -> CoreResult<()> {
        Err(CoreError::UnsupportedOperation(context()))
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
    /// Complete returned selection-payload backing, reserved before encoding.
    /// Zero declines selection encoding. Separate from transient encoder scratch.
    pub encoded_selection_bytes: u64,
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
    /// Fresh permission for the complete original writable set, before native
    /// effects and on result handoff. Default group permission covers every
    /// member; policies with member restrictions override this callback.
    /// `work()` bounds the complete callback, including member inspection.
    fn authorize_property_write(
        &self,
        consumer: &ConsumerInstanceRef,
        binding: BindingRegistrationIdentity,
        target: &str,
        operation: Operation,
        members: (&PropertyCollectionSchema, PropertySelection),
    ) -> CoreResult<()> {
        let _ = members;
        self.authorize_instance(consumer, binding, target, operation)
    }
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

/// Data roles for the implemented finite operations. Collection meaning is
/// retained explicitly rather than disguised as an unconstrained object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConsumerFiniteSchemas {
    Value(ConsumerSchemas),
    ReadAllProperties(PropertyCollectionSchema),
    ReadMultipleProperties(PropertyCollectionSchema),
    /// Complete writable-member input; the value-free result may carry a report.
    WriteAllProperties(PropertyCollectionSchema),
    /// Writable eligibility is immutable; the input map determines each call's set.
    WriteMultipleProperties(PropertyCollectionSchema),
}
impl ConsumerFiniteSchemas {
    pub fn input(&self) -> Option<TypeSchema> {
        match self {
            Self::Value(s) => s.input,
            Self::ReadAllProperties(_) => None,
            Self::WriteAllProperties(_) | Self::WriteMultipleProperties(_) => {
                Some(TypeSchema::Type(crate::value_type::ValueType::Object))
            }
            Self::ReadMultipleProperties(_) => {
                Some(TypeSchema::Type(crate::value_type::ValueType::Array))
            }
        }
    }
    /// Called under the captured data policy's complete validation allowance.
    /// A writable map is validated in full before native admission.
    pub fn validate_input(
        &self,
        data: &dyn ConsumerDataService,
        payload: Option<&Payload>,
    ) -> CoreResult<()> {
        self.validate_input_selection(data, payload).map(|_| ())
    }
    /// The selected-write set must outlive transfer of its native input.
    pub fn validate_input_selection(
        &self,
        data: &dyn ConsumerDataService,
        payload: Option<&Payload>,
    ) -> CoreResult<Option<PropertySelection>> {
        match (self, payload) {
            (Self::WriteMultipleProperties(schema), Some(p)) => {
                let selected = data.validate_property_write_map(schema, p)?;
                schema.selection_from_bits(selected.bits())?;
                Ok(Some(selected))
            }
            (Self::WriteAllProperties(schema), Some(p)) => {
                data.validate_property_map(schema, p)?;
                Ok(Some(schema.all()))
            }
            (_, Some(p)) => data
                .validate(self.input().ok_or_else(invalid)?, p)
                .map(|_| None),
            (_, None) if self.input().is_none() => Ok(None),
            _ => Err(invalid()),
        }
    }
    pub fn supports(&self, data: &dyn ConsumerDataService) -> bool {
        match self {
            Self::Value(s) => s
                .input
                .into_iter()
                .chain(s.output)
                .all(|s| data.supports_schema(s)),
            Self::ReadAllProperties(s) | Self::WriteAllProperties(s) => {
                data.supports_property_map(s)
            }
            Self::WriteMultipleProperties(s) => data.supports_property_write_selection(s),
            Self::ReadMultipleProperties(s) => {
                data.policy().encoded_selection_bytes > 0
                    && self
                        .input()
                        .is_some_and(|schema| data.supports_schema(schema))
                    && data.supports_property_selection(s)
            }
        }
    }
    pub fn validate_output(
        &self,
        data: &dyn ConsumerDataService,
        selection: Option<PropertySelection>,
        payload: Option<&Payload>,
    ) -> CoreResult<()> {
        match (self, payload) {
            (
                Self::Value(ConsumerSchemas { output: None, .. })
                | Self::WriteAllProperties(_)
                | Self::WriteMultipleProperties(_),
                None,
            ) => Ok(()),
            (
                Self::Value(ConsumerSchemas {
                    output: Some(s), ..
                }),
                Some(p),
            ) => data.validate(*s, p),
            (Self::ReadAllProperties(s), Some(p)) => data.validate_property_map(s, p),
            (Self::ReadMultipleProperties(s), Some(p)) => {
                data.validate_selected_property_map(s, selection.ok_or_else(invalid)?, p)
            }
            _ => Err(invalid()),
        }
    }
    pub fn retained_bytes(&self) -> usize {
        match self {
            Self::Value(_) => 0,
            Self::ReadAllProperties(s)
            | Self::ReadMultipleProperties(s)
            | Self::WriteAllProperties(s)
            | Self::WriteMultipleProperties(s) => s.retained_bytes(),
        }
    }
    pub fn drop_work(&self) -> u64 {
        match self {
            Self::Value(_) => 0,
            Self::ReadAllProperties(s)
            | Self::ReadMultipleProperties(s)
            | Self::WriteAllProperties(s)
            | Self::WriteMultipleProperties(s) => s.members().len() as u64 + 1,
        }
    }
}

/// A Property's value schema is not Event data. Event control/response roles
/// stay distinct; native SUBACK/UNSUBACK packets never become application data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConsumerNotificationSchemas {
    PropertyValue(TypeSchema),
    /// Complete named readable values; shared backing survives the source TD.
    ObserveAllProperties(Arc<PropertyCollectionSchema>),
    Event {
        subscription: Option<TypeSchema>,
        data: Option<TypeSchema>,
        data_response: Option<TypeSchema>,
        cancellation: Option<TypeSchema>,
    },
}
impl ConsumerNotificationSchemas {
    pub fn kind(&self) -> crate::notification::NotificationKind {
        match self {
            Self::PropertyValue(_) => crate::notification::NotificationKind::PropertyObservation,
            Self::ObserveAllProperties(_) => {
                crate::notification::NotificationKind::ObserveAllProperties
            }
            Self::Event { .. } => crate::notification::NotificationKind::Event,
        }
    }
    pub fn subscription(&self) -> Option<TypeSchema> {
        match self {
            Self::Event { subscription, .. } => *subscription,
            _ => None,
        }
    }
    pub fn data(&self) -> Option<TypeSchema> {
        match self {
            Self::PropertyValue(s) => Some(*s),
            Self::Event { data, .. } => *data,
            Self::ObserveAllProperties(_) => None,
        }
    }
    pub fn data_response(&self) -> Option<TypeSchema> {
        match self {
            Self::Event { data_response, .. } => *data_response,
            _ => None,
        }
    }
    pub fn cancellation(&self) -> Option<TypeSchema> {
        match self {
            Self::Event { cancellation, .. } => *cancellation,
            _ => None,
        }
    }
    /// Scalar support cannot qualify a named collection decoder.
    pub fn supports(&self, data: &dyn ConsumerDataService) -> bool {
        match self {
            Self::ObserveAllProperties(s) => data.supports_property_map(s),
            _ => [
                self.subscription(),
                self.data(),
                self.data_response(),
                self.cancellation(),
            ]
            .into_iter()
            .flatten()
            .all(|s| data.supports_schema(s)),
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
                || !input.schemas.kind().accepts_target(&input.target)
                || !input.services.data.available()
                || input.services.data.policy().validation_work == 0
                || input.services.access.work() == 0
                || !input.schemas.supports(input.services.data.as_ref())
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
    /// Remote effects may include a partially applied collection. No replay.
    Unknown,
    /// A collection's native successful completion was validated. This is
    /// evidence of the whole group completion, not atomicity or rollback.
    /// Local cancellation/policy failure can still refuse application handoff.
    Completed,
    /// A validated member report is retained; the whole group is incomplete.
    /// Consult the original native output for exact member/atomicity evidence.
    Reported,
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
    schemas: ConsumerFiniteSchemas,
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
    pub schemas: ConsumerFiniteSchemas,
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
            || !input.schemas.supports(input.services.data.as_ref())
            || !matches!(
                (&input.affordance, input.operation),
                (
                    AffordanceTarget::Property(_),
                    Operation::ReadProperty | Operation::WriteProperty
                ) | (AffordanceTarget::Action(_), Operation::InvokeAction)
                    | (
                        AffordanceTarget::Thing,
                        Operation::ReadAllProperties
                            | Operation::ReadMultipleProperties
                            | Operation::WriteAllProperties
                            | Operation::WriteMultipleProperties
                    )
            )
            || !matches!(
                (&input.schemas, input.operation),
                (
                    ConsumerFiniteSchemas::Value(ConsumerSchemas {
                        input: None,
                        output: Some(_)
                    }),
                    Operation::ReadProperty
                ) | (
                    ConsumerFiniteSchemas::Value(ConsumerSchemas {
                        input: Some(_),
                        output: None
                    }),
                    Operation::WriteProperty
                ) | (ConsumerFiniteSchemas::Value(_), Operation::InvokeAction)
                    | (
                        ConsumerFiniteSchemas::ReadAllProperties(_),
                        Operation::ReadAllProperties
                    )
                    | (
                        ConsumerFiniteSchemas::ReadMultipleProperties(_),
                        Operation::ReadMultipleProperties
                    )
                    | (
                        ConsumerFiniteSchemas::WriteAllProperties(_),
                        Operation::WriteAllProperties
                    )
                    | (
                        ConsumerFiniteSchemas::WriteMultipleProperties(_),
                        Operation::WriteMultipleProperties
                    )
            )
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
    /// Root operations have no affordance name; the Thing coordinate remains
    /// distinct from a Property whose name happens to be empty.
    pub fn name(&self) -> Option<&str> {
        self.affordance.name()
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
