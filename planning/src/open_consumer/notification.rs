//! Startup-linked notification preparation. One TD Form competes at a time.
//! The selected definition keeps its concrete native types through handoff.
mod provider;
use super::*;
use alloc::boxed::Box;
use clinkz_wot_core::notification::{
    ConsumerNotificationBinding, NotificationKind, NotificationSchemas,
};
use clinkz_wot_td::semantic::{
    FormOwner, ResolvedFormFacts, ResolvedSemanticDocument, SemanticError,
};
pub use provider::ConsumerNotificationProvider;
use provider::{Candidate, FormChoice, Selected};

/// Pure bounded compilation on the actual execution registration. The compiler
/// must qualify the selected native mapping, including content coding, and
/// return only a program whose participant can deliver the selected media.
/// Bounds cover compilation, configuration capture and local rollback.
pub trait ConsumerNotificationPreparation: ConsumerNotificationBinding + Clone + 'static {
    fn registration_identity(&self) -> BindingRegistrationIdentity;
    fn preparation_bounds(&self) -> ConsumerPreparationBounds;
    fn compile_notification_form(
        &self,
        input: ConsumerNotificationInput<'_>,
    ) -> CoreResult<Option<Self::Program>>;
}
pub struct ConsumerNotificationInput<'a> {
    pub document: &'a ResolvedSemanticDocument<'a>,
    pub target: &'a AffordanceTarget,
    pub form: u32,
    pub endpoint: &'a str,
    pub facts: &'a ResolvedFormFacts<'a>,
    pub schemas: &'a NotificationSchemas,
}

pub struct ConsumerNotificationRegistration<B> {
    pub binding: B,
    pub services: ConsumerServices,
}
pub struct RetainedConsumerNotificationRegistration<B> {
    pub binding: B,
    pub services: RetainedConsumerServices,
}

/// D is the deployment's selected output. A homogeneous registry returns its
/// concrete definition directly. Mixed registrations supply constructors of a
/// common output (for example an application enum of native definitions).
pub struct ConsumerNotificationRegistry<
    D,
    P = ConsumerNotificationProvider<D>,
    V = Box<dyn Fn() -> Option<u64>>,
> {
    registrations: Vec<P>,
    candidate: fn(&P) -> &dyn Candidate<D>,
    configuration_revision: V,
}

/// A fixed registry of one native Binding type. Its thread mobility follows
/// the registrations it owns.
pub type TypedConsumerNotificationRegistry<B> = ConsumerNotificationRegistry<
    ConsumerNotificationDefinition<B>,
    provider::TypedProvider<B, ConsumerNotificationDefinition<B>>,
    fn() -> Option<u64>,
>;

impl<B: ConsumerNotificationPreparation> TypedConsumerNotificationRegistry<B>
where
    B::Configuration: Clone,
{
    /// Adopt raw backing once at startup, recognizing aliases within this
    /// registry. Across registries, import the same retained owners explicitly.
    /// Eligibility must remain fixed for the duration of each search.
    pub fn try_new(
        inputs: Vec<ConsumerNotificationRegistration<B>>,
        bank: &Arc<HostResourceAccounts>,
    ) -> CoreResult<Self> {
        if inputs.is_empty() || inputs.len() > MAX_REGISTRATIONS {
            return Err(invalid());
        }
        let mut registrations: Vec<RetainedConsumerNotificationRegistration<B>> = Vec::new();
        registrations
            .try_reserve_exact(inputs.len())
            .map_err(|_| busy())?;
        for (i, input) in inputs.iter().enumerate() {
            let data = match inputs[..i]
                .iter()
                .position(|old| Arc::ptr_eq(&old.services.data, &input.services.data))
            {
                Some(first) => registrations[first].services.data.clone(),
                None => ConsumerServiceOwner::retain_data(input.services.data.clone(), bank)
                    .map_err(|e| e.error().error().clone())?,
            };
            let access = match inputs[..i]
                .iter()
                .position(|old| Arc::ptr_eq(&old.services.access, &input.services.access))
            {
                Some(first) => registrations[first].services.access.clone(),
                None => ConsumerServiceOwner::retain_access(input.services.access.clone(), bank)
                    .map_err(|e| e.error().error().clone())?,
            };
            registrations.push(RetainedConsumerNotificationRegistration {
                binding: input.binding.clone(),
                services: RetainedConsumerServices { data, access },
            });
        }
        Self::try_from_retained(registrations)
    }

    pub fn try_from_retained(
        registrations: Vec<RetainedConsumerNotificationRegistration<B>>,
    ) -> CoreResult<Self> {
        if registrations.is_empty() || registrations.len() > MAX_REGISTRATIONS {
            return Err(invalid());
        }
        Self::build(
            registrations
                .into_iter()
                .map(|r| r.typed(::core::convert::identity))
                .collect(),
            |p| p,
            || Some(0),
        )
    }
}

impl<D: 'static> ConsumerNotificationRegistry<D> {
    /// Eligibility is immutable or the deployment serializes all changes with search.
    pub fn try_new_fixed(registrations: Vec<ConsumerNotificationProvider<D>>) -> CoreResult<Self> {
        Self::try_new_revisioned(registrations, || Some(0))
    }

    /// The passive, bounded observer covers every writer of eligibility, including
    /// rejected Forms, native support, data and access. Return None during change,
    /// and a non-reused revision when stable. Binding IDs are fixed at registration.
    pub fn try_new_revisioned(
        registrations: Vec<ConsumerNotificationProvider<D>>,
        revision: impl Fn() -> Option<u64> + 'static,
    ) -> CoreResult<Self> {
        Self::build(registrations, |p| &*p.native, Box::new(revision))
    }
}

impl<D: 'static, P, V: Fn() -> Option<u64>> ConsumerNotificationRegistry<D, P, V> {
    fn build(
        registrations: Vec<P>,
        candidate: fn(&P) -> &dyn Candidate<D>,
        configuration_revision: V,
    ) -> CoreResult<Self> {
        if registrations.is_empty() || registrations.len() > MAX_REGISTRATIONS {
            return Err(invalid());
        }
        for (i, r) in registrations.iter().map(candidate).enumerate() {
            r.check()?;
            if registrations[..i]
                .iter()
                .map(candidate)
                .any(|old| old.id() == r.id())
            {
                return Err(invalid());
            }
        }
        Ok(Self {
            registrations,
            candidate,
            configuration_revision,
        })
    }

    /// Select the first viable Form in TD order. All allowed registrations for
    /// that Form compete under the same decision, including earlier rejections.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_notification(
        &self,
        thing: &Thing,
        target: &AffordanceTarget,
        kind: NotificationKind,
        selector: ConsumerSelector,
        limits: &ResourceLimits,
        bank: Arc<HostResourceAccounts>,
        budget: &mut WorkBudget,
    ) -> CoreResult<D> {
        let admission = ConsumerAdmission::new(&bank, limits);
        at_configuration(&self.configuration_revision, budget, |budget| {
            self.search(thing, target, kind, selector, &admission, budget)
        })
        .map(Selected::handoff)
    }

    fn search<'a>(
        &self,
        thing: &Thing,
        target: &AffordanceTarget,
        kind: NotificationKind,
        selector: ConsumerSelector,
        admission: &'a ConsumerAdmission<'a>,
        budget: &mut WorkBudget,
    ) -> CoreResult<Selected<'a, D>> {
        if !kind.accepts_target(target) || target.name().is_some_and(|n| n.len() > 256) {
            return Err(unsupported());
        }
        let scope = match target {
            AffordanceTarget::Thing => ValidatedProducerScope::Thing,
            AffordanceTarget::Property(n) => ValidatedProducerScope::Property(n),
            AffordanceTarget::Event(n) => ValidatedProducerScope::Event(n),
            _ => return Err(unsupported()),
        };
        let td_error = crate::producer_property_read::td_error;
        let mut walk = admission.temporary(0)?;
        let mut cursor = admission.admit(thing, &mut walk, budget)?;
        let candidates = match selector.binding {
            Some(pin) => {
                debit(budget, W::PlanningItems, self.registrations.len() as u64)?;
                let registration = self
                    .registrations
                    .iter()
                    .find(|r| (self.candidate)(r).id() == pin)
                    .ok_or_else(unsupported)?;
                ::core::slice::from_ref(registration)
            }
            None => self.registrations.as_slice(),
        };
        let mut denied = None;
        loop {
            debit(budget, W::PlanningItems, 1)?;
            walk.resize(cursor.next_step_temporary_bytes().map_err(td_error)?)?;
            let before = credits(budget);
            match cursor.step(budget, false).map_err(td_error)? {
                ValidatedProducerStep::Done => return Err(denied.unwrap_or_else(unsupported)),
                ValidatedProducerStep::Pending if before == credits(budget) => return Err(busy()),
                ValidatedProducerStep::Ready(ValidatedProducerEvent::Form(form))
                    if form.scope == scope && selector.form.is_none_or(|f| f == form.index) =>
                {
                    if !form.effective_operations.contains(&kind.start_operation())
                        || form.resolved_href.len() > 256
                    {
                        if selector.form.is_some() {
                            return Err(unsupported());
                        }
                        cursor.acknowledge();
                        continue;
                    }
                    let index = form.index;
                    let _arena = admission.temporary(ResolvedSemanticDocument::capacity_bytes())?;
                    let mut selected = None;
                    let result = cursor.with_form_semantics(
                        scope,
                        index,
                        budget,
                        |doc, endpoint, budget| -> CoreResult<()> {
                            selected = Form::from_loan(doc, target, index, endpoint)?.select(
                                candidates.iter().map(self.candidate),
                                kind,
                                admission,
                                budget,
                                &mut denied,
                            )?;
                            Ok(())
                        },
                    );
                    match result {
                        Ok(Some(Ok(()))) => {
                            if let Some(winner) = selected {
                                return Ok(winner);
                            }
                        }
                        Ok(Some(Err(CoreError::UnsupportedOperation(_)))) => {}
                        Ok(Some(Err(e))) => return Err(e),
                        Err(ValidatedSemanticError::Admission(e)) => return Err(td_error(e)),
                        Ok(None)
                        | Err(ValidatedSemanticError::Resolution(
                            SemanticError::Allocation | SemanticError::Limit,
                        )) => return Err(busy()),
                        Err(ValidatedSemanticError::Resolution(
                            SemanticError::UnsupportedContext
                            | SemanticError::UnsupportedDependency,
                        )) => {}
                    }
                    if selector.form.is_some() {
                        return Err(denied.unwrap_or_else(unsupported));
                    }
                }
                _ => {}
            }
            cursor.acknowledge();
        }
    }
}

struct Form<'a> {
    document: &'a ResolvedSemanticDocument<'a>,
    target: &'a AffordanceTarget,
    index: u32,
    endpoint: &'a str,
    facts: ResolvedFormFacts<'a>,
}

impl<'a> Form<'a> {
    fn from_loan(
        document: &'a ResolvedSemanticDocument<'a>,
        target: &'a AffordanceTarget,
        index: u32,
        endpoint: &'a str,
    ) -> CoreResult<Self> {
        let (owner, variables) = match target {
            AffordanceTarget::Thing => (FormOwner::Thing, UriVariableScope::Thing),
            AffordanceTarget::Property(n) => (FormOwner::Property(n), UriVariableScope::Property),
            AffordanceTarget::Event(n) => (FormOwner::Event(n), UriVariableScope::Event),
            _ => return Err(unsupported()),
        };
        let facts = document.form(owner, index).ok_or_else(unsupported)?;
        let [security] = facts.effective_security else {
            return Err(unsupported());
        };
        let security = document.security(security).map_err(|_| unsupported())?;
        if !matches!(security.details, SecurityDetails::NoSec)
            || security.proxy.is_some()
            || security.semantics.children().next().is_some()
            || facts.scopes.is_some()
            || facts.additional_responses
            || facts.response_content_type.is_some()
            || document.uri_variables(UriVariableScope::Thing).is_some()
            || document.uri_variables(variables).is_some()
        {
            return Err(unsupported());
        }
        Ok(Self {
            document,
            target,
            index,
            endpoint,
            facts,
        })
    }

    fn select<'bank, 'r, D: 'r>(
        &self,
        candidates: impl Iterator<Item = &'r dyn Candidate<D>>,
        kind: NotificationKind,
        admission: &'bank ConsumerAdmission<'bank>,
        budget: &mut WorkBudget,
        denied: &mut Option<CoreError>,
    ) -> CoreResult<Option<Selected<'bank, D>>> {
        let mut choice = prepare_schemas(self.document, self.target, kind, admission, budget)?;
        for r in candidates {
            debit(budget, W::ProviderProbes, 1)?;
            choice = r.probe(self, choice, admission, budget, denied)?;
        }
        Ok(match choice {
            FormChoice::Open { .. } => None,
            FormChoice::Selected(winner) => Some(winner),
        })
    }
}

fn prepare_schemas<'a, D>(
    doc: &ResolvedSemanticDocument<'_>,
    target: &AffordanceTarget,
    kind: NotificationKind,
    admission: &'a ConsumerAdmission<'a>,
    budget: &mut WorkBudget,
) -> CoreResult<FormChoice<'a, D>> {
    use ::core::mem::size_of;
    use clinkz_wot_core::collection::{
        EventCollectionSchema as Events, PropertyCollectionSchema as Properties,
    };
    let preparation = crate::type_schema::notification_preparation(doc, target, kind)?;
    check_limit(
        admission.limits,
        K::CleanupWorkItemsPerStepMax,
        preparation.cleanup * 2 + 4,
    )?;
    debit(budget, W::PlanningItems, preparation.work)?;
    debit(budget, W::CleanupItems, preparation.cleanup)?;
    let bytes = match kind {
        NotificationKind::ObserveAllProperties => {
            debit(
                budget,
                W::PlanningItems,
                (Properties::MAX_MEMBERS + 1) as u64,
            )?;
            let count = crate::type_schema::readable_member_count(doc)?;
            debit(budget, W::PlanningItems, 2 * (count as u64 + 1))?;
            Properties::backing_bound() + size_of::<Properties>() + 2 * size_of::<usize>()
        }
        NotificationKind::SubscribeAllEvents => {
            debit(budget, W::PlanningItems, (Events::MAX_MEMBERS + 1) as u64)?;
            let count = crate::type_schema::event_member_count(doc)?;
            debit(budget, W::PlanningItems, 5 * (count as u64 + 1))?;
            Events::backing_bound() + size_of::<Events>() + 2 * size_of::<usize>()
        }
        _ => preparation.backing_bytes,
    };
    let (mut peak, mut funding) = admission.retained(bytes as u64)?;
    let schemas = crate::type_schema::prepare_notification_schemas(doc, target, kind)?;
    let excess = bytes
        .checked_sub(schemas.retained_bytes())
        .ok_or_else(invalid)? as u64;
    peak.release(excess);
    funding.release_part(excess);
    Ok(FormChoice::Open {
        schemas,
        funding,
        peak,
    })
}
