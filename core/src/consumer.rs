//! Source-independent finite Consumer definitions and captured services.
//! A definition has no compiler, source TD, semantic arena or parser callback.
use crate::collection::{PropertyCollectionSchema, PropertySelection};
use crate::data_schema::PreparedSchema;
use crate::host_accounts::{HostCapacityReservation, HostResourceAccounts};
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
    fn validate(&self, schema: &PreparedSchema, payload: &Payload) -> CoreResult<()>;
    fn policy(&self) -> ConsumerDataPolicy;
    /// Actual status representation support. Scalar Object support cannot
    /// validate invocation identity, state or each Action's completed output.
    fn supports_action_statuses(&self, _schema: &crate::action::ActionCollectionSchema) -> bool {
        false
    }
    fn validate_action_statuses(
        &self,
        _schema: &crate::action::ActionCollectionSchema,
        _payload: &Payload,
    ) -> CoreResult<()> {
        Err(CoreError::UnsupportedOperation(context()))
    }
    /// Complete support of the prepared assertions. The default qualifies only
    /// type assertions; structured validators must opt in explicitly.
    fn supports_schema(&self, schema: &PreparedSchema) -> bool {
        schema.is_type_only()
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
    /// Passive live dependency revision. Changing it invalidates existing
    /// bindings and unread results; rebinding qualifies the new service state.
    /// Revisions must not be reused while original bindings/grants remain live.
    /// Wrappers must preserve invalidation by each actual service dependency.
    fn security_revision(&self) -> u64 {
        0
    }
    /// Passive complete requirement qualification. Existing services explicitly
    /// support NoSec only. Scheme names alone do not qualify parameters.
    fn supports_security(&self, requirement: &crate::security::SecurityRequirement) -> bool {
        *requirement == crate::security::SecurityRequirement::NOSEC
    }
    /// Called after fresh destination/instance permission, with a separate
    /// complete work() prepayment. Permission credit cannot fund issuance too.
    /// Live material is scoped to one call, never stored in reusable definitions.
    fn outbound_security(
        &self,
        _consumer: &ConsumerInstanceRef,
        _binding: BindingRegistrationIdentity,
        _target: &str,
        _operation: Operation,
        requirement: &crate::security::SecurityRequirement,
    ) -> CoreResult<Option<crate::security::OutboundSecurity>> {
        if *requirement == crate::security::SecurityRequirement::NOSEC {
            Ok(None)
        } else {
            Err(CoreError::Security {
                reason: SecurityFailureReason::UnsupportedScheme,
                context: context(),
            })
        }
    }
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

/// An explicitly retained actual service and its original physical funding.
/// Sharing this owner is distinct from selecting it for a logical instance.
/// Raw startup service construction remains caller-funded; retained references
/// use this owner rather than refunding its backing when one instance retires.
/// Adopt a physical service once at the composition root, then pass this owner
/// to every retained user, including independently constructed registries. A
/// second adoption of a raw alias creates another charge, not another backing.
/// Equal configuration does not establish shared ownership. Native resources
/// and any backing shared internally between distinct services remain the
/// concrete service's responsibility.
pub struct ConsumerServiceOwner<S: ?Sized> {
    service: Arc<S>,
    backing_bytes: u64,
    _funding: Vec<HostCapacityReservation>,
}
impl<S: ?Sized> ConsumerServiceOwner<S> {
    fn retain(
        service: Arc<S>,
        bytes: u64,
        bank: &Arc<HostResourceAccounts>,
    ) -> Result<Arc<Self>, BindingInputRejection<Arc<S>>> {
        let funding = bytes
            .checked_add(core::mem::size_of::<Self>() as u64 + 128)
            .ok_or_else(invalid)
            .and_then(|retained| {
                reserve(bank, &[(ResourceKind::EngineLiveBytesGlobalMax, retained)])
            });
        let funding = match funding {
            Ok(funding) => funding,
            Err(error) => {
                return Err(BindingInputRejection::new(
                    service,
                    BindingOperationalError::new(error),
                ));
            }
        };
        Ok(Arc::new(Self {
            service,
            backing_bytes: bytes,
            _funding: funding,
        }))
    }
    pub fn service(&self) -> &S {
        self.service.as_ref()
    }
    pub fn backing_bytes(&self) -> u64 {
        self.backing_bytes
    }
}
impl ConsumerServiceOwner<dyn ConsumerDataService> {
    /// Adopt this physical service at its original bank. On refusal the entire
    /// raw service is returned, including the caller's last reference.
    pub fn retain_data(
        service: Arc<dyn ConsumerDataService>,
        bank: &Arc<HostResourceAccounts>,
    ) -> Result<Arc<Self>, BindingInputRejection<Arc<dyn ConsumerDataService>>> {
        let bytes = service.policy().retained_bytes;
        Self::retain(service, bytes, bank)
    }
}
impl ConsumerServiceOwner<dyn ConsumerAccessService> {
    /// Adopt this physical service at its original bank. Share the returned
    /// owner for reuse; refusal preserves the complete raw input for retry.
    pub fn retain_access(
        service: Arc<dyn ConsumerAccessService>,
        bank: &Arc<HostResourceAccounts>,
    ) -> Result<Arc<Self>, BindingInputRejection<Arc<dyn ConsumerAccessService>>> {
        let bytes = service.retained_bytes();
        Self::retain(service, bytes, bank)
    }
}
impl<S: ?Sized> core::ops::Deref for ConsumerServiceOwner<S> {
    type Target = S;
    fn deref(&self) -> &S {
        self.service()
    }
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
/// an unconstrained present payload uses Some(PreparedSchema::Any).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsumerSchemas {
    pub input: Option<PreparedSchema>,
    pub output: Option<PreparedSchema>,
}

/// Data roles for the implemented finite operations. Collection meaning is
/// retained explicitly rather than disguised as an unconstrained object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConsumerFiniteSchemas {
    Value(ConsumerSchemas),
    QueryAllActions(crate::action::ActionCollectionSchema),
    ReadAllProperties(PropertyCollectionSchema),
    ReadMultipleProperties(PropertyCollectionSchema),
    /// Complete writable-member input; the value-free result may carry a report.
    WriteAllProperties(PropertyCollectionSchema),
    /// Writable eligibility is immutable; the input map determines each call's set.
    WriteMultipleProperties(PropertyCollectionSchema),
}
impl ConsumerFiniteSchemas {
    pub fn input(&self) -> Option<&PreparedSchema> {
        match self {
            Self::Value(s) => s.input.as_ref(),
            Self::ReadAllProperties(_) | Self::QueryAllActions(_) => None,
            Self::WriteAllProperties(_) | Self::WriteMultipleProperties(_) => {
                Some(&PreparedSchema::Type(crate::data_schema::ValueType::Object))
            }
            Self::ReadMultipleProperties(_) => {
                Some(&PreparedSchema::Type(crate::data_schema::ValueType::Array))
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
            Self::QueryAllActions(s) => data.supports_action_statuses(s),
            Self::Value(s) => s
                .input
                .iter()
                .chain(s.output.iter())
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
            (Self::QueryAllActions(s), Some(p)) => data.validate_action_statuses(s, p),
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
            ) => data.validate(s, p),
            (Self::ReadAllProperties(s), Some(p)) => data.validate_property_map(s, p),
            (Self::ReadMultipleProperties(s), Some(p)) => {
                data.validate_selected_property_map(s, selection.ok_or_else(invalid)?, p)
            }
            _ => Err(invalid()),
        }
    }
    pub fn retained_bytes(&self) -> usize {
        match self {
            Self::QueryAllActions(s) => s.retained_bytes(),
            Self::Value(s) => s
                .input
                .iter()
                .chain(s.output.iter())
                .map(PreparedSchema::retained_bytes)
                .sum(),
            Self::ReadAllProperties(s)
            | Self::ReadMultipleProperties(s)
            | Self::WriteAllProperties(s)
            | Self::WriteMultipleProperties(s) => s.retained_bytes(),
        }
    }
    pub fn drop_work(&self) -> u64 {
        match self {
            Self::QueryAllActions(s) => {
                s.members()
                    .iter()
                    .map(|m| m.output.as_ref().map_or(0, |s| s.drop_work()))
                    .sum::<u64>()
                    + s.members().len() as u64
                    + 1
            }
            Self::Value(s) => s
                .input
                .iter()
                .chain(s.output.iter())
                .map(|s| s.drop_work())
                .sum(),
            Self::ReadAllProperties(s)
            | Self::ReadMultipleProperties(s)
            | Self::WriteAllProperties(s)
            | Self::WriteMultipleProperties(s) => {
                s.members()
                    .iter()
                    .map(|m| m.schema.drop_work())
                    .sum::<u64>()
                    + s.members().len() as u64
                    + 1
            }
        }
    }
}

/// A Property's value schema is not Event data. Event control/response roles
/// stay distinct; native SUBACK/UNSUBACK packets never become application data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConsumerNotificationSchemas {
    PropertyValue(PreparedSchema),
    /// Complete named readable values; shared backing survives the source TD.
    ObserveAllProperties(Arc<PropertyCollectionSchema>),
    SubscribeAllEvents(Arc<crate::collection::EventCollectionSchema>),
    Event {
        subscription: Option<PreparedSchema>,
        data: Option<PreparedSchema>,
        data_response: Option<PreparedSchema>,
        cancellation: Option<PreparedSchema>,
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
            Self::SubscribeAllEvents(_) => {
                crate::notification::NotificationKind::SubscribeAllEvents
            }
        }
    }
    /// Scalar roles only. Event collections retain roles per named member;
    /// None here does not assert that every collection member lacks that role.
    pub fn subscription(&self) -> Option<&PreparedSchema> {
        match self {
            Self::Event { subscription, .. } => subscription.as_ref(),
            _ => None,
        }
    }
    pub fn data(&self) -> Option<&PreparedSchema> {
        match self {
            Self::PropertyValue(s) => Some(s),
            Self::Event { data, .. } => data.as_ref(),
            Self::ObserveAllProperties(_) => None,
            Self::SubscribeAllEvents(_) => None,
        }
    }
    pub fn data_response(&self) -> Option<&PreparedSchema> {
        match self {
            Self::Event { data_response, .. } => data_response.as_ref(),
            _ => None,
        }
    }
    pub fn cancellation(&self) -> Option<&PreparedSchema> {
        match self {
            Self::Event { cancellation, .. } => cancellation.as_ref(),
            _ => None,
        }
    }
    /// Scalar support cannot qualify a named collection decoder.
    pub fn supports(&self, data: &dyn ConsumerDataService) -> bool {
        match self {
            Self::ObserveAllProperties(s) => data.supports_property_map(s),
            Self::SubscribeAllEvents(s) => s.members().iter().all(|m| {
                [&m.subscription, &m.data, &m.data_response, &m.cancellation]
                    .into_iter()
                    .flatten()
                    .all(|s| data.supports_schema(s))
            }),
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
    /// A Property or collection write's native completion was validated. This
    /// reports completion under the selected mapping, not atomicity, rollback
    /// or independent confirmation of physical state.
    /// Local cancellation/policy failure can still refuse application handoff.
    Completed,
    /// A validated member report is retained; the whole group is incomplete.
    /// Consult the original native output for exact member/atomicity evidence.
    Reported,
}

/// Immutable semantic execution content. It contains no endpoint, executor,
/// service, policy, authority or account. The Host owner below chooses Arc;
/// this adds no shared-pointer field to semantic content. Existing finite Core
/// coordinate/schema types and feature gates remain; portable execution is not
/// established by this Host owner.
pub struct ConsumerExecutionContent {
    affordance: AffordanceTarget,
    operation: Operation,
    schemas: ConsumerFiniteSchemas,
    security: crate::security::SecurityRequirement,
}
impl ConsumerExecutionContent {
    pub fn try_new(
        affordance: AffordanceTarget,
        operation: Operation,
        schemas: ConsumerFiniteSchemas,
    ) -> CoreResult<Self> {
        Self::try_new_with_security(
            affordance,
            operation,
            schemas,
            crate::security::SecurityRequirement::NOSEC,
        )
    }
    pub fn try_new_with_security(
        affordance: AffordanceTarget,
        operation: Operation,
        schemas: ConsumerFiniteSchemas,
        security: crate::security::SecurityRequirement,
    ) -> CoreResult<Self> {
        if !matches!(
            (&affordance, operation),
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
                        | Operation::QueryAllActions
                )
        ) || !matches!(
            (&schemas, operation),
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
                    ConsumerFiniteSchemas::QueryAllActions(_),
                    Operation::QueryAllActions
                )
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
        ) {
            return Err(invalid());
        }
        Ok(Self {
            affordance,
            operation,
            schemas,
            security,
        })
    }
    pub fn security(&self) -> &crate::security::SecurityRequirement {
        &self.security
    }
    pub fn affordance(&self) -> &AffordanceTarget {
        &self.affordance
    }
    pub fn operation(&self) -> Operation {
        self.operation
    }
    pub fn schemas(&self) -> &ConsumerFiniteSchemas {
        &self.schemas
    }
}

/// Physical Host owner of common prepared content. References never transfer
/// or duplicate its original backing reservations. Drop order keeps every name
/// and member funded through its last actual user, including late results.
pub struct PreparedConsumerContent {
    content: ConsumerExecutionContent,
    _funding: Vec<HostCapacityReservation>,
}
impl PreparedConsumerContent {
    /// The caller funds temporary construction and prepays content destruction
    /// before entry. This acquires persistent backing before Host publication.
    pub fn try_new(
        content: ConsumerExecutionContent,
        bank: &Arc<HostResourceAccounts>,
    ) -> Result<Arc<Self>, BindingInputRejection<ConsumerExecutionContent>> {
        let bytes = core::mem::size_of::<Self>() as u64
            + content.schemas.retained_bytes() as u64
            + content.affordance.name().map_or(0, str::len) as u64
            + 128; // Arc and reservation-vector backing
        let funding = match reserve(bank, &[(ResourceKind::EngineLiveBytesGlobalMax, bytes)]) {
            Ok(f) => f,
            Err(e) => {
                return Err(BindingInputRejection::new(
                    content,
                    BindingOperationalError::new(e),
                ));
            }
        };
        Ok(Arc::new(Self {
            content,
            _funding: funding,
        }))
    }
    pub fn content(&self) -> &ConsumerExecutionContent {
        &self.content
    }
}
impl core::ops::Deref for PreparedConsumerContent {
    type Target = ConsumerExecutionContent;
    fn deref(&self) -> &Self::Target {
        &self.content
    }
}

/// One purely qualified native mapping and its actual data service. Independent
/// logical instances bind policy and accounts separately. The complete native
/// artifact remains specialized unless the actual Binding establishes reuse.
/// This carrier does not itself perform TD support assessment.
pub struct ConsumerDefinition {
    input: ConsumerDefinitionInput,
    data_policy: ConsumerDataPolicy,
}
pub struct ConsumerDefinitionInput {
    pub content: Arc<PreparedConsumerContent>,
    pub form: u32,
    pub target: Box<str>,
    pub artifact: BindingArtifactEnvelope<HostBindingArtifact>,
    pub execution: Arc<HostConsumerBindingRegistration>,
    pub configuration: ConsumerBindingConfiguration,
    pub data: Arc<ConsumerServiceOwner<dyn ConsumerDataService>>,
    pub content_type: Box<str>,
    /// Original shared registration backing, retained by every mapping of that
    /// actual executor. It never migrates to a later mapping/instance bank.
    pub execution_funding: Arc<Vec<HostCapacityReservation>>,
    // Last: native/data/content destruction precedes original funding release.
    pub funding: Vec<HostCapacityReservation>,
}
impl ConsumerDefinition {
    pub fn try_new(
        input: ConsumerDefinitionInput,
    ) -> Result<Self, BindingInputRejection<ConsumerDefinitionInput>> {
        let id = input.artifact.identity();
        let actual = input.execution.identity();
        let policy = input.data.policy();
        if id.binding_id() != actual.binding_id()
            || id.binding_generation() != actual.binding_generation()
            || id.configuration() != actual.configuration()
            || id.compatibility() != actual.artifact_compatibility()
            || id.role() != BindingArtifactRole::ConsumerCall
            || input.execution.consumer_configuration() != Some(input.configuration)
            || !input
                .execution
                .supports_operation(input.content.operation())
            || !input.data.available()
            || policy.retained_bytes != input.data.backing_bytes()
            || input.data.content_type() != input.content_type.as_ref()
            || !input.content.schemas().supports(input.data.service())
            || policy.validation_work == 0
        {
            return Err(BindingInputRejection::new(
                input,
                BindingOperationalError::new(invalid()),
            ));
        }
        Ok(Self {
            input,
            data_policy: policy,
        })
    }
    pub fn content(&self) -> &Arc<PreparedConsumerContent> {
        &self.input.content
    }
    pub fn name(&self) -> Option<&str> {
        self.affordance().name()
    }
    pub fn affordance(&self) -> &AffordanceTarget {
        self.input.content.affordance()
    }
    pub fn operation(&self) -> Operation {
        self.input.content.operation()
    }
    pub fn form(&self) -> u32 {
        self.input.form
    }
    pub fn target(&self) -> &str {
        &self.input.target
    }
    pub fn binding(&self) -> BindingRegistrationIdentity {
        self.input.execution.identity()
    }
    pub fn artifact(&self) -> &BindingArtifactEnvelope<HostBindingArtifact> {
        &self.input.artifact
    }
    pub fn execution(&self) -> &HostConsumerBindingRegistration {
        &self.input.execution
    }
    pub fn resources(&self) -> BindingResourceDeclarations {
        self.input.execution.resources()
    }
    pub fn configuration(&self) -> ConsumerBindingConfiguration {
        self.input.configuration
    }
    pub fn data(&self) -> &dyn ConsumerDataService {
        self.input.data.service()
    }
    pub fn data_policy(&self) -> ConsumerDataPolicy {
        self.data_policy
    }
    /// Passive dependency checks also run after permission callbacks and at
    /// result handoff. A new instance cannot refresh changed captured bounds.
    pub fn check_dependencies(&self) -> CoreResult<()> {
        if self.execution().consumer_configuration() != Some(self.configuration())
            || self.binding().configuration() != self.configuration().configuration()
            || self.data().policy() != self.data_policy
            || self.data().content_type() != self.input.content_type.as_ref()
            || !self.data().available()
            || !self.content().schemas().supports(self.data())
            || !self.execution().supports_operation(self.operation())
        {
            return Err(invalid());
        }
        Ok(())
    }
    /// Source-free instance dependency binding. No TD/compiler lookup or native
    /// retargeting is possible here. Logical authority is attached by the runtime.
    /// Refusal returns both original owners, including a caller's last reference.
    #[allow(clippy::type_complexity)] // Refusal preserves both move-owned inputs.
    pub fn bind(
        self: Arc<Self>,
        access: Arc<ConsumerServiceOwner<dyn ConsumerAccessService>>,
        bank: Arc<HostResourceAccounts>,
        budget: &mut WorkBudget,
    ) -> Result<
        ConsumerBinding,
        BindingInputRejection<(
            Arc<Self>,
            Arc<ConsumerServiceOwner<dyn ConsumerAccessService>>,
        )>,
    > {
        let access_work = access.work();
        let security_revision = access.security_revision();
        let checked = (|| {
            if access_work == 0 {
                return Err(invalid());
            }
            if !pay(budget, WorkClass::ProviderProbes, access_work) {
                return Err(busy());
            }
            self.check_dependencies()?;
            if !access.supports_security(self.content().security()) {
                return Err(invalid());
            }
            access.authorize(self.binding(), self.target(), self.operation())?;
            self.check_dependencies()?;
            if access.security_revision() != security_revision
                || access.work() != access_work
                || access.retained_bytes() != access.backing_bytes()
            {
                return Err(invalid());
            }
            let bytes = core::mem::size_of::<ConsumerBinding>() as u64 + 128;
            reserve(&bank, &[(ResourceKind::EngineLiveBytesGlobalMax, bytes)])
        })();
        let funding = match checked {
            Ok(f) => f,
            Err(e) => {
                return Err(BindingInputRejection::new(
                    (self, access),
                    BindingOperationalError::new(e),
                ));
            }
        };
        Ok(ConsumerBinding {
            input: ConsumerBindingInput {
                definition: self,
                security_revision,
                access,
                funding,
                bank,
            },
            access_work,
        })
    }
    pub fn into_input(self) -> ConsumerDefinitionInput {
        self.input
    }
}

/// Instance-local policy and funding over qualified reusable content/mapping.
/// Reusing the content never shares permission or moves physical backing to this
/// bank. Runtime records and every call/result continue to charge this bank.
pub struct ConsumerBinding {
    input: ConsumerBindingInput,
    access_work: u64,
}
pub struct ConsumerBindingInput {
    pub security_revision: u64,
    pub definition: Arc<ConsumerDefinition>,
    pub access: Arc<ConsumerServiceOwner<dyn ConsumerAccessService>>,
    pub funding: Vec<HostCapacityReservation>,
    pub bank: Arc<HostResourceAccounts>,
}
impl ConsumerBinding {
    pub fn definition(&self) -> &Arc<ConsumerDefinition> {
        &self.input.definition
    }
    pub fn name(&self) -> Option<&str> {
        self.definition().name()
    }
    pub fn affordance(&self) -> &AffordanceTarget {
        self.definition().affordance()
    }
    pub fn operation(&self) -> Operation {
        self.definition().operation()
    }
    pub fn form(&self) -> u32 {
        self.definition().form()
    }
    pub fn binding(&self) -> BindingRegistrationIdentity {
        self.definition().binding()
    }
    pub fn bank(&self) -> &Arc<HostResourceAccounts> {
        &self.input.bank
    }
    pub fn into_parts(self) -> (ConsumerBindingInput, ConsumerDataPolicy, u64) {
        let policy = self.definition().data_policy();
        (self.input, policy, self.access_work)
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
