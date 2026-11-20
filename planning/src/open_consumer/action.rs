//! Operation-scoped continuing Action selection. Each actual native mapping
//! compiles independently; control compatibility is checked before invocation.
use super::*;
use ::core::mem::size_of;
use alloc::boxed::Box;
use clinkz_wot_core::consumer_action::*;
use clinkz_wot_td::semantic::{FormOwner, ResolvedSemanticDocument, SemanticError};

pub struct ConsumerActionPreparationInput<'a> {
    pub document: &'a ResolvedSemanticDocument<'a>,
    pub name: &'a str,
    pub form: u32,
    pub operation: Operation,
    pub endpoint: &'a str,
    pub schemas: &'a ValueSchemas,
}
/// Startup-linked pure native compiler. Returned mappings own their original
/// configuration and executor. Bounds include rollback and Drop. The BindingId
/// identifies the registration throughout its membership in this registry.
pub trait ConsumerActionPreparation {
    fn identity(&self) -> BindingRegistrationIdentity;
    fn bounds(&self) -> ConsumerPreparationBounds;
    fn compile_action(
        &self,
        input: ConsumerActionPreparationInput<'_>,
    ) -> CoreResult<Option<Box<dyn ConsumerActionMapping>>>;
}
/// Composition imports the original physical owners. Selection never adopts
/// raw service aliases or charges their backing again for another mapping.
pub struct ConsumerActionRegistration {
    pub preparation: Box<dyn ConsumerActionPreparation>,
    pub services: RetainedConsumerServices,
}
#[derive(Clone, Copy, Default)]
pub struct ConsumerActionSelectors {
    pub invoke: ConsumerSelector,
    pub query: ConsumerSelector,
    pub cancel: ConsumerSelector,
}
pub struct ConsumerActionRegistry {
    registrations: Vec<ConsumerActionRegistration>,
    configuration_revision: Box<dyn Fn() -> Option<u64>>,
}
impl ConsumerActionRegistry {
    /// All eligibility inputs stay fixed for the duration of each search.
    /// Intended for immutable registrations/services or externally serialized use.
    pub fn try_new_fixed(registrations: Vec<ConsumerActionRegistration>) -> CoreResult<Self> {
        Self::try_new_revisioned(registrations, || Some(0))
    }

    /// The composition owner supplies a passive observation costing one provider probe. It covers
    /// *all* inputs to eligibility, including negative compiler, data, access
    /// policy and compatibility answers. Return a stable non-reused revision,
    /// or None during a change. Every contributing writer must participate.
    /// A constant is valid for immutable inputs or externally serialized search.
    pub fn try_new_revisioned(
        registrations: Vec<ConsumerActionRegistration>,
        configuration_revision: impl Fn() -> Option<u64> + 'static,
    ) -> CoreResult<Self> {
        if registrations.is_empty()
            || registrations.len() > MAX_REGISTRATIONS
            || registrations.iter().enumerate().any(|(i, r)| {
                r.preparation.bounds().work == 0
                    || r.services.check_backing().is_err()
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
        Ok(Self {
            registrations,
            configuration_revision: Box::new(configuration_revision),
        })
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
        let observe = |budget: &mut WorkBudget| {
            debit(budget, W::ProviderProbes, 1)?;
            (self.configuration_revision)().ok_or_else(|| CoreError::StaleHandle(context()))
        };
        let revision = observe(budget)?;
        let result = self.search(thing, name, selectors, limits, bank, budget);
        // Also fence unsupported and ambiguous answers, including observations
        // of losing providers. No competitor or revision enters the definition.
        if observe(budget)? != revision {
            return Err(CoreError::StaleHandle(context()));
        }
        result
    }

    fn search(
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
        let config =
            ValidatedThingAdmissionConfig::try_from_limits(limits).map_err(|_| invalid())?;
        let common = config.temporary_allowance();
        let _scratch = reserve(
            &bank,
            &[
                (K::EngineLiveBytesGlobalMax, common),
                (K::AdmissionTemporaryBytesGlobalMax, common),
                (K::AdmissionPeakLiveBytesGlobalMax, common),
            ],
        )?;
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
        let selectors = [selectors.invoke, selectors.query, selectors.cancel];
        let mut forms = Vec::new();
        let mut described = [false; 3];
        let mut indexed = [false; 3];
        loop {
            debit(budget, W::PlanningItems, 1)?;
            let before = credits(budget);
            match cursor
                .step(budget, false)
                .map_err(crate::producer_property_read::td_error)?
            {
                ValidatedProducerStep::Done => break,
                ValidatedProducerStep::Pending if before == credits(budget) => return Err(busy()),
                ValidatedProducerStep::Pending => continue,
                ValidatedProducerStep::Ready(ValidatedProducerEvent::Scope {
                    scope: ValidatedProducerScope::Action(n),
                    forms: Some(count),
                }) if n == name => {
                    // Each retained resolved href fits the existing 256-byte profile.
                    let bytes = count
                        .checked_mul(size_of::<ActionForm>() + 256)
                        .ok_or_else(busy)?;
                    cursor
                        .split_temporary_allowance(bytes as u64)
                        .map_err(crate::producer_property_read::td_error)?;
                    check_limit(
                        limits,
                        K::LargestContiguousAllocationBytesMax,
                        (count * size_of::<ActionForm>()) as u64,
                    )?;
                    forms.try_reserve_exact(count).map_err(|_| busy())?;
                }
                ValidatedProducerStep::Ready(ValidatedProducerEvent::Form(form))
                    if form.scope == ValidatedProducerScope::Action(name) =>
                {
                    debit(
                        budget,
                        W::PlanningItems,
                        (form.effective_operations.len() as u64)
                            .checked_mul(OPERATIONS.len() as u64)
                            .ok_or_else(busy)?,
                    )?;
                    let bounded_href = form.resolved_href.len() <= 256;
                    let roles = ::core::array::from_fn(|i| {
                        let applicable = form.effective_operations.contains(&OPERATIONS[i]);
                        described[i] |= applicable;
                        let admitted = applicable
                            && bounded_href
                            && selectors[i].form.is_none_or(|f| f == form.index);
                        indexed[i] |= admitted;
                        admitted
                    });
                    if roles.contains(&true) {
                        check_limit(
                            limits,
                            K::LargestContiguousAllocationBytesMax,
                            form.resolved_href.len() as u64,
                        )?;
                        debit(budget, W::UriBytes, form.resolved_href.len() as u64)?;
                        debit(budget, W::CleanupItems, 1)?;
                        forms.push(ActionForm {
                            index: form.index,
                            endpoint: form.resolved_href.into(),
                            roles,
                            candidates: Default::default(),
                            schemas_checked: false,
                        });
                    }
                }
                ValidatedProducerStep::Ready(_) => {}
            }
            cursor.acknowledge();
        }
        if !indexed[0]
            || (1..3).any(|i| {
                !indexed[i]
                    && (described[i]
                        || selectors[i].binding.is_some()
                        || selectors[i].form.is_some())
            })
        {
            return Err(unsupported());
        }
        let mut search = ActionSearch {
            registry: self,
            name,
            selectors,
            cursor,
            forms,
            described,
            schemas: None,
            funding: Vec::new(),
            bank: &bank,
            limits,
        };
        let selected = search.choose(budget)?;
        // Construction checks each mapping and each control's compatibility with Invoke.
        let controls = selected[1..].iter().flatten().count() as u64;
        debit(budget, W::ProviderProbes, 1 + 2 * controls)?;
        let mut take = |role: usize| {
            let (form, provider) = selected[role]?;
            let candidates = search.forms[form].candidates[role]
                .as_mut()
                .expect("selected operation was probed");
            let candidate = candidates.swap_remove(provider);
            search.funding.extend(candidate.funding);
            Some(candidate.operation)
        };
        let input = ConsumerActionDefinitionInput {
            target: AffordanceTarget::Action(Arc::from(name)),
            invoke: take(0).unwrap(),
            query: take(1),
            cancel: take(2),
            schemas: search.schemas.take().ok_or_else(unsupported)?,
            bank: bank.clone(),
            funding: ::core::mem::take(&mut search.funding),
        };
        drop(search);
        ConsumerActionDefinition::try_new(input).map_err(|e| e.error().error().clone())
    }

    #[allow(clippy::too_many_arguments)]
    fn qualify(
        &self,
        doc: &ResolvedSemanticDocument<'_>,
        name: &str,
        form: u32,
        operation: Operation,
        endpoint: &str,
        schemas: &ValueSchemas,
        selector: ConsumerSelector,
        limits: &ResourceLimits,
        bank: &Arc<HostResourceAccounts>,
        budget: &mut WorkBudget,
        temporary_headroom: ValidatedFormScratch,
        mut accept: impl FnMut(Candidate, &mut WorkBudget) -> CoreResult<()>,
    ) -> CoreResult<()> {
        if endpoint.len() > 256 || doc.action().is_none_or(|a| a.synchronous == Some(true)) {
            return Ok(());
        }
        let owner = FormOwner::Action(name);
        let facts = doc.form(owner, form).ok_or_else(unsupported)?;
        let [security] = facts.effective_security else {
            return Ok(());
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
            return Ok(());
        }

        for r in &self.registrations {
            debit(budget, W::ProviderProbes, 1)?;
            let id = r.preparation.identity();
            if selector.binding.is_some_and(|b| b != id.binding_id()) {
                continue;
            }
            r.services.check_backing()?;
            let policy = r.services.data.policy();
            let access_work = r.services.access.work();
            let revision = r.services.access.qualification_revision();
            let content_type = r.services.data.content_type();
            let b = r.preparation.bounds();
            if b.work == 0 || policy.validation_work == 0 || access_work == 0 {
                return Err(invalid());
            }
            let current = || {
                r.services.check_backing()?;
                if r.preparation.identity() != id
                    || r.preparation.bounds() != b
                    || r.services.data.policy() != policy
                    || !r.services.data.available()
                    || r.services.data.content_type() != content_type
                    || !action_data_supported(operation, schemas, r.services.data.service())
                    || r.services.access.work() != access_work
                    || r.services.access.qualification_revision() != revision
                    || !r
                        .services
                        .access
                        .supports_security(&clinkz_wot_core::security::SecurityRequirement::NOSEC)
                {
                    return Err(invalid());
                }
                Ok(())
            };
            if !pay(budget, W::ProviderProbes, access_work) {
                return Err(busy());
            }
            if !r
                .services
                .access
                .supports_security(&clinkz_wot_core::security::SecurityRequirement::NOSEC)
                || !r.services.data.available()
                || content_type != facts.content_type
                || !action_data_supported(operation, schemas, r.services.data.service())
            {
                continue;
            }
            let authorized = r.services.access.authorize(id, endpoint, operation);
            // A policy callback cannot change the compiler allowance before entry.
            current()?;
            match authorized {
                Ok(()) => {}
                Err(
                    CoreError::Security {
                        reason:
                            SecurityFailureReason::AuthorizationDenied
                            | SecurityFailureReason::MissingCredentials
                            | SecurityFailureReason::InvalidCredentials
                            | SecurityFailureReason::UnsupportedScheme,
                        ..
                    }
                    // Access implementations can reject a destination in their
                    // native validation or Binding policy vocabulary.
                    | CoreError::Validation(_)
                    | CoreError::Binding(_)
                    | CoreError::UnsupportedOperation(_),
                ) => continue,
                Err(e) => return Err(e),
            }
            check_limit(limits, K::PlanCompileWorkUnitsPerStepMax, b.work)?;
            if b.temporary_bytes > temporary_headroom.available_bytes {
                return Err(CoreError::LimitExceeded {
                    resource: temporary_headroom.limiting_resource,
                    limit: temporary_headroom.available_bytes,
                    requested: Some(b.temporary_bytes),
                    observed: None,
                    context: context(),
                });
            }
            check_limit(limits, K::BindingArtifactBytesPerThingMax, b.retained_bytes)?;
            if !pay(budget, W::BindingPolls, b.work) || !pay(budget, W::CleanupItems, 1) {
                return Err(busy());
            }
            let bytes = b
                .retained_bytes
                .checked_add(1024 + endpoint.len() as u64 + name.len() as u64)
                // The service owner funds its original media string. This
                // independent snapshot is acquired at the definition bank.
                .and_then(|bytes| bytes.checked_add(content_type.len() as u64))
                .ok_or_else(invalid)?;
            let funding = reserve(
                bank,
                &[
                    (K::EngineLiveBytesGlobalMax, bytes),
                    (K::CleanupItemsMax, 1),
                ],
            )?;
            let compiled = r
                .preparation
                .compile_action(ConsumerActionPreparationInput {
                    document: doc,
                    name,
                    form,
                    operation,
                    endpoint,
                    schemas,
                });
            current()?;
            let mapping = match compiled {
                Ok(Some(m)) => m,
                Ok(None) | Err(CoreError::UnsupportedOperation(_)) => continue,
                Err(e) => return Err(e),
            };
            let selected = ConsumerActionOperation {
                bounds: mapping.bounds(),
                mapping,
                endpoint: endpoint.into(),
                form,
                data: r.services.data.clone(),
                data_policy: policy,
                content_type: content_type.into(),
                identity: id,
                operation,
            };
            selected.check_native_dependencies()?;
            accept(
                Candidate {
                    operation: selected,
                    funding,
                },
                budget,
            )?;
        }
        Ok(())
    }
}

const OPERATIONS: [Operation; 3] = [
    Operation::InvokeAction,
    Operation::QueryAction,
    Operation::CancelAction,
];

struct Candidate {
    operation: ConsumerActionOperation,
    funding: Vec<HostCapacityReservation>,
}

struct ActionForm {
    index: u32,
    endpoint: Box<str>,
    roles: [bool; 3],
    // Some(empty) records that all eligible registrations declined this operation.
    candidates: [Option<Vec<Candidate>>; 3],
    schemas_checked: bool,
}

fn ambiguous() -> CoreError {
    CoreError::Selection {
        reason: SelectionFailureReason::AmbiguousBindingOwner,
        context: context(),
    }
}

type Pick = (usize, usize); // Form position, provider position within that Form and role.
type Family = [Option<Pick>; 3];

// One representative assignment per Invoke provider. All assignments in the
// current prefix share its Form coordinates.
struct ProviderAssignment {
    providers: [usize; 3],
    ambiguous: bool,
}

struct ActionSearch<'a, 'td> {
    registry: &'a ConsumerActionRegistry,
    name: &'a str,
    selectors: [ConsumerSelector; 3],
    cursor: ValidatedProducerCursor<'td>,
    forms: Vec<ActionForm>,
    described: [bool; 3],
    schemas: Option<ValueSchemas>,
    funding: Vec<HostCapacityReservation>,
    bank: &'a Arc<HostResourceAccounts>,
    limits: &'a ResourceLimits,
}

impl ActionSearch<'_, '_> {
    fn choose(&mut self, budget: &mut WorkBudget) -> CoreResult<Family> {
        let mut assignments = self.provider_slots()?;
        let mut invokes = self.provider_slots()?;
        let invoke_only = !self.described[1..].contains(&true);
        for form in 0..self.forms.len() {
            debit(budget, W::PlanningItems, 1)?;
            if !self.forms[form].roles[0] {
                continue;
            }
            self.compile_operation(form, 0, budget, |candidate, _| {
                if invoke_only && !invokes.is_empty() {
                    return Err(ambiguous());
                }
                invokes.push(candidate);
                Ok(())
            })?;
            let count = invokes.len();
            if count == 0 {
                continue;
            }
            debit(budget, W::PlanningItems, count as u64)?;
            assignments.clear();
            assignments.extend((0..count).map(|invoke| ProviderAssignment {
                providers: [invoke, 0, 0],
                ambiguous: false,
            }));
            if let Some(family) = self.choose_controls(
                1,
                [Some(form), None, None],
                &invokes,
                &mut assignments,
                budget,
            )? {
                self.forms[form].candidates[0] = Some(invokes);
                return Ok(family);
            }
            // This Invoke Form can never win. Drop its mappings with their
            // original funding and reuse the same paid slots for the next Form.
            invokes.clear();
        }
        Err(unsupported())
    }

    // Visit Form prefixes in preference order, carrying all surviving Invoke
    // providers together. Described controls are the only two levels of descent.
    fn choose_controls(
        &mut self,
        next_role: usize,
        mut forms: [Option<usize>; 3],
        invokes: &[Candidate],
        mut remaining: &mut [ProviderAssignment],
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<Family>> {
        let Some(role) = (next_role..OPERATIONS.len()).find(|&role| self.described[role]) else {
            let [only] = remaining else {
                return Err(ambiguous());
            };
            return Ok(Some(::core::array::from_fn(|i| {
                forms[i].map(|form| (form, only.providers[i]))
            })));
        };
        let final_control = !self.described[role + 1..].contains(&true);
        for form in 0..self.forms.len() {
            debit(budget, W::PlanningItems, 1)?;
            if !self.forms[form].roles[role] {
                continue;
            }
            let mut matched = 0;
            self.visit_control(form, role, budget, |provider, control, budget| {
                for i in 0..remaining.len() {
                    let path = &mut remaining[i];
                    let origin = &invokes[path.providers[0]].operation.mapping;
                    debit(budget, W::ProviderProbes, 1)?;
                    match control
                        .operation
                        .mapping
                        .supports_continuation(origin.as_ref())
                    {
                        Ok(()) => {}
                        Err(CoreError::UnsupportedOperation(_)) => continue,
                        Err(e) => return Err(e),
                    }
                    if final_control && (matched != 0 || path.ambiguous) {
                        return Err(ambiguous());
                    }
                    if i < matched {
                        path.ambiguous = true;
                    } else {
                        // Keep one representative per matching Invoke in the
                        // prefix. The entry swapped out has already been tested
                        // against this control provider.
                        debit(budget, W::PlanningItems, 1)?;
                        path.providers[role] = provider;
                        remaining.swap(matched, i);
                        matched += 1;
                    }
                }
                Ok(())
            })?;
            if matched == 0 {
                continue;
            }
            forms[role] = Some(form);
            let (eligible, others) = remaining.split_at_mut(matched);
            if let Some(family) =
                self.choose_controls(role + 1, forms, invokes, eligible, budget)?
            {
                return Ok(Some(family));
            }
            // Every later control depends only on the original Invoke. An
            // origin that exhausted those controls cannot gain a completion
            // from another provider or a later Form for this role.
            if others.is_empty() {
                break;
            }
            remaining = others;
        }
        Ok(None)
    }

    // The native candidate cache and the reusable assignment slots draw from
    // the same temporary allowance as the TD loan and compiler scratch.
    fn provider_slots<T>(&mut self) -> CoreResult<Vec<T>> {
        let count = self.registry.registrations.len();
        let bytes = (count * size_of::<T>()) as u64;
        check_limit(self.limits, K::LargestContiguousAllocationBytesMax, bytes)?;
        self.cursor
            .split_temporary_allowance(bytes)
            .map_err(crate::producer_property_read::td_error)?;
        let mut slots = Vec::new();
        slots.try_reserve_exact(count).map_err(|_| busy())?;
        Ok(slots)
    }

    fn visit_control(
        &mut self,
        index: usize,
        role: usize,
        budget: &mut WorkBudget,
        mut visit: impl FnMut(usize, &Candidate, &mut WorkBudget) -> CoreResult<()>,
    ) -> CoreResult<()> {
        if let Some(candidates) = &self.forms[index].candidates[role] {
            for (provider, candidate) in candidates.iter().enumerate() {
                visit(provider, candidate, budget)?;
            }
            return Ok(());
        }
        let mut candidates = self.provider_slots()?;
        self.compile_operation(index, role, budget, |candidate, budget| {
            visit(candidates.len(), &candidate, budget)?;
            candidates.push(candidate);
            Ok(())
        })?;
        self.forms[index].candidates[role] = Some(candidates);
        Ok(())
    }

    fn compile_operation(
        &mut self,
        index: usize,
        role: usize,
        budget: &mut WorkBudget,
        accept: impl FnMut(Candidate, &mut WorkBudget) -> CoreResult<()>,
    ) -> CoreResult<()> {
        let form = &mut self.forms[index];
        let result = self.cursor.with_indexed_form_semantics(
            ValidatedProducerScope::Action(self.name),
            form.index,
            budget,
            |doc, temporary_headroom, budget| -> CoreResult<()> {
                if !form.schemas_checked {
                    qualify_schemas(
                        doc,
                        self.name,
                        &mut self.schemas,
                        self.limits,
                        self.bank,
                        &mut self.funding,
                        budget,
                    )?;
                    form.schemas_checked = true;
                }
                self.registry.qualify(
                    doc,
                    self.name,
                    form.index,
                    OPERATIONS[role],
                    &form.endpoint,
                    self.schemas.as_ref().unwrap(),
                    self.selectors[role],
                    self.limits,
                    self.bank,
                    budget,
                    temporary_headroom,
                    accept,
                )
            },
        );
        match result {
            Ok(Some(Ok(()) | Err(CoreError::UnsupportedOperation(_)))) => {}
            Ok(Some(Err(e))) => return Err(e),
            Err(ValidatedSemanticError::Admission(e)) => {
                return Err(crate::producer_property_read::td_error(e));
            }
            Ok(None)
            | Err(ValidatedSemanticError::Resolution(
                SemanticError::Allocation | SemanticError::Limit,
            )) => return Err(busy()),
            Err(ValidatedSemanticError::Resolution(
                SemanticError::UnsupportedContext | SemanticError::UnsupportedDependency,
            )) => {}
        }
        Ok(())
    }
}

fn debit(budget: &mut WorkBudget, class: W, count: u64) -> CoreResult<()> {
    budget.consume(class, count).map_err(|_| busy())
}

fn qualify_schemas(
    doc: &ResolvedSemanticDocument<'_>,
    name: &str,
    schemas: &mut Option<ValueSchemas>,
    limits: &ResourceLimits,
    bank: &Arc<HostResourceAccounts>,
    funding: &mut Vec<HostCapacityReservation>,
    budget: &mut WorkBudget,
) -> CoreResult<()> {
    let owner = FormOwner::Action(name);
    let input = doc.affordance_schema(owner, "input");
    let output = doc.affordance_schema(owner, "output");
    let mut preparation = crate::type_schema::SchemaPreparation::default();
    for schema in input.into_iter().chain(output) {
        preparation.include(schema)?;
    }
    check_limit(
        limits,
        K::CleanupWorkItemsPerStepMax,
        preparation.cleanup * 2 + 4,
    )?;
    debit(budget, W::PlanningItems, preparation.work)?;
    if let Some(original) = schemas {
        let agrees = |actual, expected: &Option<_>| match (actual, expected) {
            (None, None) => Ok(true),
            (Some(a), Some(e)) => crate::type_schema::matches_schema(a, e),
            _ => Ok(false),
        };
        if !agrees(input, &original.input)? || !agrees(output, &original.output)? {
            return Err(unsupported());
        }
    } else {
        debit(budget, W::CleanupItems, preparation.cleanup)?;
        let lease = reserve(
            bank,
            &[(
                K::EngineLiveBytesGlobalMax,
                preparation.backing_bytes as u64,
            )],
        )?;
        *schemas = Some(ValueSchemas {
            input: input.map(crate::type_schema::prepare_schema).transpose()?,
            output: output.map(crate::type_schema::prepare_schema).transpose()?,
        });
        funding.extend(lease);
    }
    Ok(())
}
