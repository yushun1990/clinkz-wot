//! Typed, startup-linked notification preparation over the common admitted TD.
//! This facet deliberately does not force subscriptions through finite calls.
use super::*;
use clinkz_wot_core::notification::{ConsumerNotificationBinding, NotificationKind};
use clinkz_wot_td::semantic::{ResolvedSemanticDocument, SemanticScope};

/// Bounded pure compilation on the actual execution registration. Implementors
/// may be external crates. No socket, callback, task or source loan may escape.
/// Bounds cover the whole callback and rollback, including variable backing.
pub trait ConsumerNotificationPreparation: ConsumerNotificationBinding + Clone + 'static {
    fn registration_identity(&self) -> BindingRegistrationIdentity;
    fn preparation_bounds(&self) -> NotificationPreparationBounds;
    fn compile_notification(
        &self,
        input: ConsumerNotificationInput<'_>,
    ) -> CoreResult<Option<Self::Program>>;
}
#[derive(Clone, Copy)]
pub struct NotificationPreparationBounds {
    pub work: u64,
    pub temporary_bytes: u64,
    pub retained_bytes: u64,
}
pub struct ConsumerNotificationInput<'a> {
    pub document: &'a ResolvedSemanticDocument<'a>,
    pub event: &'a str,
    pub form: u32,
    pub endpoint: &'a str,
    pub schemas: ConsumerNotificationSchemas,
}
/// A typed registry can contain many actual registrations of a linked Binding.
/// Each selected output owns its actual executor and services. Neither protocol
/// names nor a Core/compiler whitelist participate in selection.
pub struct ConsumerNotificationRegistration<B> {
    pub binding: B,
    pub services: ConsumerServices,
}
pub struct ConsumerNotificationRegistry<B> {
    registrations: Vec<ConsumerNotificationRegistration<B>>,
}
impl<B: ConsumerNotificationPreparation> ConsumerNotificationRegistry<B>
where
    B::Configuration: Clone,
{
    pub fn try_new(registrations: Vec<ConsumerNotificationRegistration<B>>) -> CoreResult<Self> {
        if registrations.is_empty()
            || registrations.len() > MAX_REGISTRATIONS
            || registrations.iter().enumerate().any(|(i, r)| {
                let id = r.binding.registration_identity();
                r.binding.identity() != (id.binding_id(), id.binding_generation())
                    || r.binding.preparation_bounds().work == 0
                    || r.services.access.work() == 0
                    || r.services.data.policy().validation_work == 0
                    || registrations[..i]
                        .iter()
                        .any(|old| old.binding.identity().0 == id.binding_id())
            })
        {
            return Err(invalid());
        }
        Ok(Self { registrations })
    }

    /// Select one Event subscribe Form, in TD order by default. Ambiguity within
    /// a Form refuses. Unsupported alternatives remain eligible for later Forms.
    /// All control schemas are inspected separately before native compilation.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_event(
        &self,
        thing: &Thing,
        event: &str,
        selector: ConsumerSelector,
        limits: &ResourceLimits,
        bank: Arc<HostResourceAccounts>,
        budget: &mut WorkBudget,
    ) -> CoreResult<ConsumerNotificationDefinition<B>> {
        if event.len() > 256 {
            return Err(unsupported());
        }
        let common = limits
            .get(K::AdmissionTemporaryBytesPerOperationMax)
            .unwrap_or(0);
        let _scratch = reserve(
            &bank,
            &[
                (K::EngineLiveBytesGlobalMax, common),
                (K::AdmissionTemporaryBytesGlobalMax, common),
            ],
        )?;
        let config =
            ValidatedThingAdmissionConfig::try_from_limits(limits).map_err(|_| invalid())?;
        let mut basic = ValidatedThingCursor::from_thing(
            thing,
            &config,
            AdmissionLedger::new(
                SlotIndex::new(0),
                Generation::INITIAL,
                0,
                common,
                0,
                0,
                0,
                0,
            ),
        );
        let validated = loop {
            let before = credits(budget);
            basic = match basic.step(budget, false) {
                ValidatedThingProgress::Complete(v) => break v,
                ValidatedThingProgress::Pending(next) => next,
                ValidatedThingProgress::Failed(e) => {
                    return Err(crate::producer_property_read::td_error(e));
                }
            };
            if before == credits(budget) {
                return Err(busy());
            }
        };
        let mut cursor = validated.into_producer();
        loop {
            if !pay(budget, W::PlanningItems, 1) {
                return Err(busy());
            }
            match cursor
                .step(budget, false)
                .map_err(crate::producer_property_read::td_error)?
            {
                ValidatedProducerStep::Done => return Err(unsupported()),
                ValidatedProducerStep::Pending => {}
                ValidatedProducerStep::Ready(ValidatedProducerEvent::Scope { .. }) => {
                    cursor.acknowledge()
                }
                ValidatedProducerStep::Ready(ValidatedProducerEvent::Form(form)) => {
                    let index = form.index;
                    let eligible = matches!(form.scope, ValidatedProducerScope::Event(n) if n == event)
                        && form
                            .effective_operations
                            .contains(&Operation::SubscribeEvent)
                        && selector.form.is_none_or(|f| f == index);
                    if !eligible {
                        cursor.acknowledge();
                        continue;
                    }
                    let result = cursor.with_form_semantics(
                        ValidatedProducerScope::Event(event),
                        index,
                        budget,
                        |document, endpoint, budget| {
                            self.select_form(
                                document, event, index, endpoint, selector, limits, &bank, budget,
                            )
                        },
                    );
                    cursor.acknowledge();
                    match result {
                        Ok(Some(Ok(Some(d)))) => return Ok(d),
                        Ok(Some(Ok(None) | Err(CoreError::UnsupportedOperation(_)))) => {}
                        Ok(Some(Err(e))) => return Err(e),
                        Ok(None) => return Err(busy()),
                        Err(ValidatedSemanticError::Admission(e)) => {
                            return Err(crate::producer_property_read::td_error(e));
                        }
                        Err(ValidatedSemanticError::Resolution(
                            clinkz_wot_td::semantic::SemanticError::Allocation,
                        )) => return Err(busy()),
                        Err(ValidatedSemanticError::Resolution(_)) => {}
                    }
                }
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn select_form(
        &self,
        document: &ResolvedSemanticDocument<'_>,
        event: &str,
        index: u32,
        endpoint: &str,
        selector: ConsumerSelector,
        limits: &ResourceLimits,
        bank: &Arc<HostResourceAccounts>,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<ConsumerNotificationDefinition<B>>> {
        if endpoint.len() > 256 {
            return Ok(None);
        }
        let owner = clinkz_wot_td::semantic::FormOwner::Event(event);
        let facts = document.form(owner, index).ok_or_else(unsupported)?;
        let schemas = ConsumerNotificationSchemas {
            subscription: schema(document, owner, "subscription")?,
            data: schema(document, owner, "data")?,
            data_response: schema(document, owner, "dataResponse")?,
            cancellation: schema(document, owner, "cancellation")?,
        };
        // Common policy remains deliberately NoSec/type-only.
        let [security] = facts.effective_security else {
            return Ok(None);
        };
        let security = document.security(security).map_err(|_| unsupported())?;
        if !matches!(security.details, SecurityDetails::NoSec)
            || security.proxy.is_some()
            || security.semantics.children().next().is_some()
            || facts.scopes.is_some()
            || facts.content_coding.is_some()
            || facts.additional_responses
            || facts.response_content_type.is_some()
            || document
                .scope(SemanticScope::AffordanceResponse { owner, index })
                .is_some()
            || document.uri_variables(UriVariableScope::Thing).is_some()
            || document.uri_variables(UriVariableScope::Event).is_some()
        {
            return Ok(None);
        }
        let mut chosen = None;
        for r in &self.registrations {
            let id = r.binding.registration_identity();
            if selector.binding.is_some_and(|b| b != id.binding_id()) {
                continue;
            }
            if !pay(budget, W::ProviderProbes, 1) {
                return Err(busy());
            }
            let data = &r.services.data;
            if !data.available()
                || data.content_type() != facts.content_type
                || ![
                    schemas.subscription,
                    schemas.data,
                    schemas.data_response,
                    schemas.cancellation,
                ]
                .into_iter()
                .flatten()
                .all(|s| data.supports_schema(s))
            {
                continue;
            }
            if !pay(budget, W::ProviderProbes, r.services.access.work()) {
                return Err(busy());
            }
            if r.services
                .access
                .authorize(id, endpoint, Operation::SubscribeEvent)
                .is_err()
            {
                continue;
            }
            let bounds = r.binding.preparation_bounds();
            check_limit(limits, K::PlanCompileWorkUnitsPerStepMax, bounds.work)?;
            check_limit(
                limits,
                K::AdmissionTemporaryBytesPerOperationMax,
                bounds.temporary_bytes,
            )?;
            check_limit(
                limits,
                K::BindingArtifactBytesPerThingMax,
                bounds.retained_bytes,
            )?;
            if !pay(budget, W::BindingPolls, bounds.work) {
                return Err(busy());
            }
            let _temporary = reserve(
                &bank,
                &[
                    (K::EngineLiveBytesGlobalMax, bounds.temporary_bytes),
                    (K::AdmissionTemporaryBytesGlobalMax, bounds.temporary_bytes),
                ],
            )?;
            let retained = bounds
                .retained_bytes
                .checked_add(1024 + endpoint.len() as u64 + event.len() as u64)
                .and_then(|n| n.checked_add(data.policy().retained_bytes))
                .and_then(|n| n.checked_add(r.services.access.retained_bytes()))
                .ok_or_else(invalid)?;
            let funding = reserve(
                &bank,
                &[
                    (K::EngineLiveBytesGlobalMax, retained),
                    (K::CleanupItemsMax, 1),
                ],
            )?;
            let program = match r.binding.compile_notification(ConsumerNotificationInput {
                document,
                event,
                form: index,
                endpoint,
                schemas,
            }) {
                Ok(Some(p)) => p,
                Ok(None) | Err(CoreError::UnsupportedOperation(_)) => continue,
                Err(e) => return Err(e),
            };
            r.binding.support(&program, NotificationKind::Event)?;
            if chosen.is_some() {
                return Err(ambiguous());
            }
            chosen = Some(
                ConsumerNotificationDefinition::try_new(ConsumerNotificationDefinitionInput {
                    binding: r.binding.clone(),
                    program,
                    configuration: r.binding.configuration().clone(),
                    identity: id,
                    target: AffordanceTarget::Event(Arc::from(event)),
                    endpoint: endpoint.into(),
                    form: index,
                    schemas,
                    services: r.services.clone(),
                    funding,
                    bank: bank.clone(),
                })
                .map_err(|e| e.error().error().clone())?,
            );
        }
        Ok(chosen)
    }
}
fn schema(
    doc: &ResolvedSemanticDocument<'_>,
    owner: clinkz_wot_td::semantic::FormOwner<'_>,
    role: &str,
) -> CoreResult<Option<clinkz_wot_core::value_type::TypeSchema>> {
    doc.affordance_schema(owner, role)
        .map(crate::type_schema::prepare_type_schema)
        .transpose()
}
