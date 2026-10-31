//! Operation-scoped continuing Action selection. Each actual native mapping
//! compiles independently; control compatibility is checked before invocation.
use super::*;
use alloc::boxed::Box;
use clinkz_wot_core::consumer_action::*;
use clinkz_wot_td::semantic::{FormOwner, ResolvedSemanticDocument};

pub struct ConsumerActionPreparationInput<'a> {
    pub document: &'a ResolvedSemanticDocument<'a>,
    pub name: &'a str,
    pub form: u32,
    pub operation: Operation,
    pub endpoint: &'a str,
    pub schemas: ConsumerSchemas,
}
/// Startup-linked pure native compiler. Returned mappings own their original
/// configuration/executor, not source loans. Bounds include rollback and Drop.
pub trait ConsumerActionPreparation {
    fn identity(&self) -> BindingRegistrationIdentity;
    fn bounds(&self) -> ConsumerPreparationBounds;
    fn compile_action(
        &self,
        input: ConsumerActionPreparationInput<'_>,
    ) -> CoreResult<Option<Box<dyn ConsumerActionMapping>>>;
}
pub struct ConsumerActionRegistration {
    pub preparation: Box<dyn ConsumerActionPreparation>,
    pub services: ConsumerServices,
}
#[derive(Clone, Copy, Default)]
pub struct ConsumerActionSelectors {
    pub invoke: ConsumerSelector,
    pub query: ConsumerSelector,
    pub cancel: ConsumerSelector,
}
pub struct ConsumerActionRegistry {
    registrations: Vec<ConsumerActionRegistration>,
}
impl ConsumerActionRegistry {
    pub fn try_new(registrations: Vec<ConsumerActionRegistration>) -> CoreResult<Self> {
        if registrations.is_empty()
            || registrations.len() > MAX_REGISTRATIONS
            || registrations.iter().enumerate().any(|(i, r)| {
                r.preparation.bounds().work == 0
                    || r.services.access.work() == 0
                    || r.services.data.policy().validation_work == 0
                    || registrations[..i].iter().any(|old| {
                        old.preparation.identity().binding_id()
                            == r.preparation.identity().binding_id()
                    })
            })
        {
            return Err(invalid());
        }
        Ok(Self { registrations })
    }

    /// Admit one continuing Action, including every described query/cancel
    /// operation. Absence of synchronous is eligible here, never a finite-result
    /// guarantee. Unsupported described controls refuse the association.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_action(
        &self,
        thing: &Thing,
        name: &str,
        selectors: ConsumerActionSelectors,
        limits: &ResourceLimits,
        bank: Arc<HostResourceAccounts>,
        budget: &mut WorkBudget,
    ) -> CoreResult<ConsumerActionDefinition> {
        if name.len() > 256 {
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
        // Locals unwind in reverse order: executable owners precede refunds.
        let mut funding = Vec::new();
        let mut selected: [Option<ConsumerActionOperation>; 3] = [None, None, None];
        let mut described = [false; 3];
        let mut schemas = None;
        loop {
            if !pay(budget, W::PlanningItems, 1) {
                return Err(busy());
            }
            match cursor
                .step(budget, false)
                .map_err(crate::producer_property_read::td_error)?
            {
                ValidatedProducerStep::Done => break,
                ValidatedProducerStep::Pending => {}
                ValidatedProducerStep::Ready(ValidatedProducerEvent::Scope { .. }) => {
                    cursor.acknowledge()
                }
                ValidatedProducerStep::Ready(ValidatedProducerEvent::Form(form)) => {
                    if form.scope != ValidatedProducerScope::Action(name) {
                        cursor.acknowledge();
                        continue;
                    }
                    let index = form.index;
                    let ops = form.effective_operations.to_vec();
                    for (i, operation, selector) in [
                        (0, Operation::InvokeAction, selectors.invoke),
                        (1, Operation::QueryAction, selectors.query),
                        (2, Operation::CancelAction, selectors.cancel),
                    ] {
                        if !ops.contains(&operation) {
                            continue;
                        }
                        described[i] = true;
                        if selected[i].is_some() || selector.form.is_some_and(|f| f != index) {
                            continue;
                        }
                        let result = cursor.with_form_semantics(
                            ValidatedProducerScope::Action(name),
                            index,
                            budget,
                            |doc, endpoint, budget| {
                                self.select(
                                    doc, name, index, operation, endpoint, selector, limits, &bank,
                                    budget,
                                )
                            },
                        );
                        match result {
                            Ok(Some(Ok(Some((s, op, mut leases))))) => {
                                if schemas.is_some_and(|old| old != s) {
                                    return Err(invalid());
                                }
                                schemas = Some(s);
                                funding.append(&mut leases);
                                selected[i] = Some(op);
                            }
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
                    cursor.acknowledge();
                }
            }
        }
        if selected[0].is_none() || (0..3).any(|i| described[i] && selected[i].is_none()) {
            return Err(unsupported());
        }
        ConsumerActionDefinition::try_new(ConsumerActionDefinitionInput {
            target: AffordanceTarget::Action(Arc::from(name)),
            schemas: schemas.ok_or_else(unsupported)?,
            invoke: selected[0].take().unwrap(),
            query: selected[1].take(),
            cancel: selected[2].take(),
            bank,
            funding,
        })
        .map_err(|e| e.error().error().clone())
    }

    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    fn select(
        &self,
        doc: &ResolvedSemanticDocument<'_>,
        name: &str,
        form: u32,
        operation: Operation,
        endpoint: &str,
        selector: ConsumerSelector,
        limits: &ResourceLimits,
        bank: &Arc<HostResourceAccounts>,
        budget: &mut WorkBudget,
    ) -> CoreResult<
        Option<(
            ConsumerSchemas,
            ConsumerActionOperation,
            Vec<HostCapacityReservation>,
        )>,
    > {
        if endpoint.len() > 256 || doc.action().is_none_or(|a| a.synchronous == Some(true)) {
            return Ok(None);
        }
        let owner = FormOwner::Action(name);
        let facts = doc.form(owner, form).ok_or_else(unsupported)?;
        let [security] = facts.effective_security else {
            return Ok(None);
        };
        let security = doc.security(security).map_err(|_| unsupported())?;
        if !matches!(security.details, SecurityDetails::NoSec)
            || security.proxy.is_some()
            || security.semantics.children().next().is_some()
            || facts.scopes.is_some()
            || facts.content_coding.is_some()
            || facts.additional_responses
            || facts
                .response_content_type
                .is_some_and(|m| m != facts.content_type)
        {
            return Ok(None);
        }
        let schemas = ConsumerSchemas {
            input: doc
                .affordance_schema(owner, "input")
                .map(crate::type_schema::prepare_type_schema)
                .transpose()?,
            output: doc
                .affordance_schema(owner, "output")
                .map(crate::type_schema::prepare_type_schema)
                .transpose()?,
        };
        let mut chosen = None;
        for r in &self.registrations {
            let id = r.preparation.identity();
            if selector.binding.is_some_and(|b| b != id.binding_id()) {
                continue;
            }
            if !pay(budget, W::ProviderProbes, r.services.access.work()) {
                return Err(busy());
            }
            if !r.services.data.available()
                || r.services.data.content_type() != facts.content_type
                || !schemas
                    .input
                    .into_iter()
                    .chain(schemas.output)
                    .all(|s| r.services.data.supports_schema(s))
                || r.services
                    .access
                    .authorize(id, endpoint, operation)
                    .is_err()
            {
                continue;
            }
            let b = r.preparation.bounds();
            check_limit(limits, K::PlanCompileWorkUnitsPerStepMax, b.work)?;
            check_limit(
                limits,
                K::AdmissionTemporaryBytesPerOperationMax,
                b.temporary_bytes,
            )?;
            check_limit(limits, K::BindingArtifactBytesPerThingMax, b.retained_bytes)?;
            if !pay(budget, W::BindingPolls, b.work) || !pay(budget, W::CleanupItems, 1) {
                return Err(busy());
            }
            let _temporary = reserve(
                bank,
                &[
                    (K::EngineLiveBytesGlobalMax, b.temporary_bytes),
                    (K::AdmissionTemporaryBytesGlobalMax, b.temporary_bytes),
                ],
            )?;
            let bytes = b
                .retained_bytes
                .checked_add(1024 + endpoint.len() as u64 + name.len() as u64)
                .and_then(|n| n.checked_add(r.services.data.policy().retained_bytes))
                .and_then(|n| n.checked_add(r.services.access.retained_bytes()))
                .ok_or_else(invalid)?;
            let funding = reserve(
                bank,
                &[
                    (K::EngineLiveBytesGlobalMax, bytes),
                    (K::CleanupItemsMax, 1),
                ],
            )?;
            let mapping = match r
                .preparation
                .compile_action(ConsumerActionPreparationInput {
                    document: doc,
                    name,
                    form,
                    operation,
                    endpoint,
                    schemas,
                }) {
                Ok(Some(m)) => m,
                Ok(None) | Err(CoreError::UnsupportedOperation(_)) => continue,
                Err(e) => return Err(e),
            };
            if mapping.identity() != id || mapping.operation() != operation {
                return Err(invalid());
            }
            if chosen.is_some() {
                return Err(ambiguous());
            }
            chosen = Some((
                schemas,
                ConsumerActionOperation {
                    mapping,
                    endpoint: endpoint.into(),
                    form,
                    services: r.services.clone(),
                },
                funding,
            ));
        }
        Ok(chosen)
    }
}
