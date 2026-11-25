//! Whole-Thing Producer selection. A prepared operation is an immutable,
//! source-independent Binding definition, not a logical instance or a native
//! listener. Each Form/operation can select a different registered capability.
#![allow(clippy::result_large_err)]
use crate::{finite_producer::PRODUCER_DESCRIPTION_BYTES, producer_property_read::td_error};
use alloc::{boxed::Box, rc::Rc, string::String, vec::Vec};
use clinkz_wot_core::notification::NotificationKind;
pub use clinkz_wot_core::publication::{PublicationFacts, PublicationObserver, PublicationState};
use clinkz_wot_core::{
    BindingGeneration, BindingId, CoreError, CoreResult, ErrorContext, ErrorPhase, Operation,
    RetryClass, producer::ProducerInstanceKey, thing::AffordanceSite,
};
use clinkz_wot_foundation::{AdmissionLedger, ResourceKind, ResourceLimits, WorkBudget, WorkClass};
use clinkz_wot_td::{
    ValidatedProducerEvent, ValidatedProducerScope, ValidatedProducerStep, ValidatedSemanticError,
    ValidatedThingAdmissionConfig, ValidatedThingCursor, ValidatedThingProgress,
    semantic::{FormOwner, ResolvedSemanticDocument},
    thing::Thing,
};
use core::any::Any;
use core::cell::RefCell;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormSite {
    Thing,
    Property(String),
    Action(String),
    Event(String),
}
impl FormSite {
    fn from_scope(scope: ValidatedProducerScope<'_>) -> Self {
        match scope {
            ValidatedProducerScope::Thing => Self::Thing,
            ValidatedProducerScope::Property(n) => Self::Property(n.into()),
            ValidatedProducerScope::Action(n) => Self::Action(n.into()),
            ValidatedProducerScope::Event(n) => Self::Event(n.into()),
        }
    }
    pub fn loan(&self) -> ValidatedProducerScope<'_> {
        match self {
            Self::Thing => ValidatedProducerScope::Thing,
            Self::Property(n) => ValidatedProducerScope::Property(n),
            Self::Action(n) => ValidatedProducerScope::Action(n),
            Self::Event(n) => ValidatedProducerScope::Event(n),
        }
    }
    pub fn owner(&self) -> FormOwner<'_> {
        match self {
            Self::Thing => FormOwner::Thing,
            Self::Property(n) => FormOwner::Property(n),
            Self::Action(n) => FormOwner::Action(n),
            Self::Event(n) => FormOwner::Event(n),
        }
    }
    pub fn site(&self) -> AffordanceSite<'_> {
        match self {
            Self::Thing => AffordanceSite::Thing,
            Self::Property(n) => AffordanceSite::Property(n),
            Self::Action(n) => AffordanceSite::Action(n),
            Self::Event(n) => AffordanceSite::Event(n),
        }
    }
    fn name_bytes(&self) -> usize {
        match self {
            Self::Thing => 0,
            Self::Property(n) | Self::Action(n) | Self::Event(n) => n.len(),
        }
    }
}

/// Temporary common meaning. A Binding interprets native vocabularies and
/// returns only independently owned preparation. No handler or authority is
/// passed to a compiler, and Action/notification controls are not finite calls.
#[derive(Clone, Copy)]
pub struct ProducerOperationInput<'a> {
    pub document: &'a ResolvedSemanticDocument<'a>,
    pub site: &'a FormSite,
    pub form: u32,
    pub operation: Operation,
    pub target: &'a str,
}

/// An actual startup registration with its own concrete definition type.
/// `None` is a nonmatching candidate; `Err` is a failed selected preparation.
/// Preparation may perform fresh lowering or checked binding of explicitly
/// supplied immutable work. It always returns the mapping for this input, not a
/// shared whole-Thing qualification certificate or logical authority.
/// The final check must compare all immutable services, mapping operands and
/// native endpoint facts against the SAME owned definition. Native readiness,
/// collisions and leases are checked separately before Servient publication.
pub trait ProducerCapability {
    /// Observe every mutable input to eligibility, lowering, final comparison
    /// and their declared bounds before any registration is probed. Immutable
    /// registrations contribute no facts. Per-request authority stays live.
    fn observe_configuration(&self, facts: &mut PublicationObserver<'_>) -> CoreResult<()>;

    fn identity(&self) -> (BindingId, BindingGeneration);
    /// Additional retained definition footprint, including indirect backing
    /// without its own original reservation. Explicitly funded shared content
    /// keeps its complete physical owner through every definition reference;
    /// each returned mapping still declares its own layout/operand cost.
    /// Native infrastructure/work and the logical Thing are separate owners.
    fn definition_bytes(&self, _definition: &dyn Any) -> Option<usize> {
        None
    }
    fn prepare(
        &self,
        input: ProducerOperationInput<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<Box<dyn Any>>>;
    /// Complete current support, with the same matching/refusal semantics as
    /// prepare, without retaining a replacement definition. Final qualification
    /// uses this for every nonselected registration to detect new ambiguity.
    fn supports(
        &self,
        input: ProducerOperationInput<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<bool>;
    /// Peak additional controlled backing used by one supports/check_final
    /// callback, including any temporary definition materialization and indirect
    /// storage. These callbacks retain no new controlled backing after return;
    /// their complete peak, including during a reload, must fit this bound.
    /// Callbacks are sequential.
    /// Managed Host qualification reserves this bound before calling them;
    /// None makes no managed resource claim.
    fn final_check_temporary_bytes(&self) -> Option<usize> {
        None
    }
    /// Compare current complete meaning/services with the existing immutable
    /// definition. This must include support, not just a type/identity match.
    /// A foreign definition or unsupported input returns false; two missing
    /// mappings do not establish equivalent prepared meaning.
    fn check_final(
        &self,
        input: ProducerOperationInput<'_>,
        definition: &dyn Any,
        budget: &mut WorkBudget,
    ) -> CoreResult<bool>;
}

/// A linked Host finite service constructs its native configuration from only its
/// selected mappings. Assembly is bounded and pure. Servient owns the driver,
/// dispatch, work capacity and publication. Execution-only Bindings do not
/// depend on this preparation facet.
pub trait FiniteProducerService: ProducerCapability {
    type Binding: clinkz_wot_core::finite::FiniteIngressBinding + 'static;

    fn assemble<'a>(
        &self,
        instance: ProducerInstanceKey,
        operations: impl Iterator<Item = &'a PreparedOperation>,
    ) -> CoreResult<(
        Self::Binding,
        <Self::Binding as clinkz_wot_core::finite::FiniteIngressBinding>::Configuration,
    )>;
}

pub struct PreparedOperation {
    pub site: FormSite,
    pub form: u32,
    pub operation: Operation,
    pub binding: (BindingId, BindingGeneration),
    provider: usize,
    definition: Box<dyn Any>,
}
impl PreparedOperation {
    /// Position in the exact eligible registration set used by this selection.
    pub fn registration(&self) -> usize {
        self.provider
    }
    pub fn notification_start(&self) -> Option<NotificationKind> {
        NotificationKind::for_operation(self.operation)
            .filter(|kind| kind.start_operation() == self.operation)
    }
    /// The concrete Binding obtains its own typed, immutable definition. This
    /// loan confers no right to create logical authority or shut down I/O.
    pub fn definition<T: 'static>(&self) -> Option<&T> {
        self.definition.downcast_ref()
    }
    pub fn definition_any(&self) -> &dyn Any {
        self.definition.as_ref()
    }
}
/// Complete selection for one description. Operation coordinates and provider
/// indices belong here; reusable subdefinitions belong to concrete Bindings.
/// A new Thing requalifies its complete description, services and behaviors.
pub struct PreparedThing {
    operations: Vec<PreparedOperation>,
    retained_bytes: Option<usize>,
    notification_mappings: usize,
}
impl PreparedThing {
    pub fn operations(&self) -> &[PreparedOperation] {
        &self.operations
    }
    /// Present for preparation under the legacy aggregate backing contract.
    pub fn retained_bytes(&self) -> Option<usize> {
        self.retained_bytes
    }
    /// One publication route per selected start Form/operation, including
    /// alternatives. Stop operations retain definitions but add no route.
    pub fn notification_mapping_count(&self) -> usize {
        self.notification_mappings
    }
}

/// Replace only the document base. Original Form hrefs, extensions and JSON
/// number lexemes survive. The normal TD walk resolves the resulting document.
pub fn description_at_base(
    description: &str,
    base: &str,
    budget: &mut WorkBudget,
) -> CoreResult<String> {
    use alloc::collections::BTreeMap;
    use serde_json::value::RawValue;
    if description.len() > PRODUCER_DESCRIPTION_BYTES || base.len() > PRODUCER_DESCRIPTION_BYTES {
        return Err(unsupported());
    }
    budget
        .consume(
            WorkClass::CodecInputBytes,
            (description.len() + base.len()) as u64,
        )
        .map_err(|_| busy())?;
    clinkz_wot_td::core::data_type::BaseUri::parse(base).map_err(|_| unsupported())?;
    let mut fields: BTreeMap<String, &RawValue> =
        serde_json::from_str(description).map_err(|_| CoreError::Validation(context()))?;
    let encoded = serde_json::value::to_raw_value(base).map_err(|_| unsupported())?;
    // The source already bounds all original members. Include the new encoded
    // base, key, colon and comma before emitting the final document.
    budget
        .consume(
            WorkClass::CodecOutputBytes,
            (description.len() + encoded.get().len() + 8) as u64,
        )
        .map_err(|_| busy())?;
    fields.insert("base".into(), &encoded);
    let result = serde_json::to_string(&fields).map_err(|_| unsupported())?;
    if result.len() > PRODUCER_DESCRIPTION_BYTES {
        return Err(unsupported());
    }
    Ok(result)
}

fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
}
fn busy() -> CoreError {
    CoreError::Backpressure(context())
}
fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(context())
}
fn semantic_error(error: ValidatedSemanticError) -> CoreError {
    match error {
        ValidatedSemanticError::Admission(cause) => td_error(cause),
        ValidatedSemanticError::Resolution(clinkz_wot_td::semantic::SemanticError::Allocation) => {
            busy()
        }
        _ => unsupported(),
    }
}

type ProviderSet = Rc<[Rc<dyn ProducerCapability>]>;

/// The deployment's actual eligible registrations. Replacing even an
/// unselected registration invalidates coverage of the previous set. Existing
/// publications retain their selected native services independently.
#[derive(Clone)]
pub struct ProducerRegistrations(Rc<RefCell<PublicationState<ProviderSet>>>);

impl ProducerRegistrations {
    pub fn new(providers: Vec<Rc<dyn ProducerCapability>>) -> Self {
        Self(Rc::new(RefCell::new(PublicationState::new(
            providers.into(),
        ))))
    }

    pub fn replace(&self, providers: Vec<Rc<dyn ProducerCapability>>) {
        let old = self.0.borrow_mut().replace(providers.into());
        drop(old);
    }

    pub fn snapshot(&self, facts: &mut PublicationFacts) -> CoreResult<ProviderSet> {
        let set = self.0.borrow();
        set.observe(&mut facts.observer())?;
        Ok(set.get().clone())
    }
}

fn observe_providers(
    providers: &[&dyn ProducerCapability],
    facts: &mut PublicationFacts,
    budget: &mut WorkBudget,
) -> CoreResult<()> {
    budget
        .consume(WorkClass::ProviderProbes, providers.len() as u64)
        .map_err(|_| busy())?;
    for provider in providers {
        provider.observe_configuration(&mut facts.observer())?;
    }
    Ok(())
}

/// Pure whole-description selection. `behavior` queries the actual staged
/// Servient registration (including query/cancel), not a nominal operation
/// whitelist. Missing or ambiguous provider, empty op set, missing behavior,
/// exhausted work and unsupported selected meaning all fail before native setup.
/// No single-compiler, single-Form or single-family invariant is imposed.
pub fn prepare_thing(
    description: &str,
    instance: ProducerInstanceKey,
    limits: &ResourceLimits,
    budget: &mut WorkBudget,
    providers: &[&dyn ProducerCapability],
    behavior: impl Fn(ValidatedProducerScope<'_>, Operation) -> bool,
) -> CoreResult<PreparedThing> {
    let mut facts = PublicationFacts::default();
    select_thing(
        description,
        instance,
        limits,
        budget,
        providers,
        behavior,
        &mut facts,
        Some(core::mem::size_of::<PreparedThing>()),
    )
}

/// Select every advertised operation and retain the observed deployment versions
/// until the publication decision. Each selected provider owns its definition.
pub fn select_finite_thing(
    description: &str,
    instance: ProducerInstanceKey,
    limits: &ResourceLimits,
    budget: &mut WorkBudget,
    providers: &[&dyn ProducerCapability],
    behavior: impl Fn(ValidatedProducerScope<'_>, Operation) -> bool,
    facts: &mut PublicationFacts,
) -> CoreResult<PreparedThing> {
    select_thing(
        description,
        instance,
        limits,
        budget,
        providers,
        behavior,
        facts,
        None,
    )
}
#[allow(clippy::too_many_arguments)]
fn select_thing(
    description: &str,
    instance: ProducerInstanceKey,
    limits: &ResourceLimits,
    budget: &mut WorkBudget,
    providers: &[&dyn ProducerCapability],
    behavior: impl Fn(ValidatedProducerScope<'_>, Operation) -> bool,
    facts: &mut PublicationFacts,
    mut retained_bytes: Option<usize>,
) -> CoreResult<PreparedThing> {
    observe_providers(providers, facts, budget)?;
    let mut operations = Vec::new();
    let mut notification_mappings = 0;
    visit_thing(
        description,
        instance,
        limits,
        budget,
        &behavior,
        |_, input, budget| {
            let mut chosen = None;
            for (i, provider) in providers.iter().enumerate() {
                budget
                    .consume(WorkClass::PlanningItems, 1)
                    .map_err(|_| busy())?;
                if let Some(definition) = provider.prepare(input, budget)? {
                    if chosen.is_some() {
                        return Err(unsupported());
                    }
                    chosen = Some((i, provider.identity(), definition));
                }
            }
            let selected = chosen.ok_or_else(unsupported)?;
            let old_capacity = operations.capacity();
            operations.try_reserve(1).map_err(|_| busy())?;
            if let Some(total) = &mut retained_bytes {
                let binding_bytes = providers[selected.0]
                    .definition_bytes(selected.2.as_ref())
                    .ok_or_else(unsupported)?;
                let next = total
                    .checked_add(
                        (operations.capacity() - old_capacity)
                            * core::mem::size_of::<PreparedOperation>(),
                    )
                    .and_then(|n| n.checked_add(input.site.name_bytes()))
                    .and_then(|n| n.checked_add(binding_bytes))
                    .ok_or_else(busy)?;
                if u64::try_from(next).map_or(true, |n| {
                    n > limits
                        .get(ResourceKind::EngineLiveBytesGlobalMax)
                        .unwrap_or(0)
                }) {
                    return Err(busy());
                }
                *total = next;
            }
            let operation = PreparedOperation {
                site: input.site.clone(),
                form: input.form,
                operation: input.operation,
                binding: selected.1,
                provider: selected.0,
                definition: selected.2,
            };
            notification_mappings += usize::from(operation.notification_start().is_some());
            operations.push(operation);
            Ok(())
        },
    )?;
    let _current = PublicationFacts::lock_current(&[facts])?;
    Ok(PreparedThing {
        operations,
        retained_bytes,
        notification_mappings,
    })
}

// The common TD walk lends one operation at a time. Checking an existing Thing
// never constructs an owned replacement operation/definition set.
fn visit_thing(
    description: &str,
    instance: ProducerInstanceKey,
    limits: &ResourceLimits,
    budget: &mut WorkBudget,
    behavior: &impl Fn(ValidatedProducerScope<'_>, Operation) -> bool,
    mut visit: impl FnMut(usize, ProducerOperationInput<'_>, &mut WorkBudget) -> CoreResult<()>,
) -> CoreResult<usize> {
    if description.len() > PRODUCER_DESCRIPTION_BYTES {
        return Err(unsupported());
    }
    budget
        .consume(WorkClass::CodecInputBytes, description.len() as u64)
        .map_err(|_| busy())?;
    let td: Thing =
        serde_json::from_str(description).map_err(|_| CoreError::Validation(context()))?;
    let config = ValidatedThingAdmissionConfig::try_from_limits(limits)
        .map_err(|_| CoreError::Validation(context()))?;
    let temporary = limits
        .get(ResourceKind::AdmissionTemporaryBytesPerOperationMax)
        .unwrap_or(0);
    let mut cursor = ValidatedThingCursor::from_thing(
        &td,
        &config,
        AdmissionLedger::new(
            instance.slot().slot(),
            instance.slot().generation(),
            0,
            temporary,
            0,
            0,
            0,
            0,
        ),
    );
    let validated = loop {
        budget
            .consume(WorkClass::PlanningItems, 1)
            .map_err(|_| busy())?;
        let before = WorkClass::ALL.map(|c| budget.remaining(c));
        cursor = match cursor.step(budget, false) {
            ValidatedThingProgress::Complete(value) => break value,
            ValidatedThingProgress::Failed(cause) => return Err(td_error(cause)),
            ValidatedThingProgress::Pending(next) => next,
        };
        if before == WorkClass::ALL.map(|c| budget.remaining(c)) {
            return Err(busy());
        }
    };
    let mut cursor = validated.into_producer();
    let mut count = 0;
    loop {
        budget
            .consume(WorkClass::PlanningItems, 1)
            .map_err(|_| busy())?;
        let before = WorkClass::ALL.map(|c| budget.remaining(c));
        match cursor.step(budget, false).map_err(td_error)? {
            ValidatedProducerStep::Done => break,
            ValidatedProducerStep::Pending => {}
            ValidatedProducerStep::Ready(ValidatedProducerEvent::Scope { scope, forms }) => {
                if scope != ValidatedProducerScope::Thing && forms.unwrap_or(0) == 0 {
                    return Err(CoreError::Validation(context()));
                }
                cursor.acknowledge()
            }
            ValidatedProducerStep::Ready(ValidatedProducerEvent::Form(form)) => {
                let site = FormSite::from_scope(form.scope);
                let index = form.index;
                let ops = form.effective_operations.to_vec();
                if ops.is_empty() {
                    return Err(unsupported());
                }
                for operation in ops {
                    if !behavior(site.loan(), operation) {
                        return Err(unsupported());
                    }
                    cursor
                        .with_form_semantics(site.loan(), index, budget, |doc, target, budget| {
                            visit(
                                count,
                                ProducerOperationInput {
                                    document: doc,
                                    site: &site,
                                    form: index,
                                    operation,
                                    target,
                                },
                                budget,
                            )
                        })
                        .map_err(semantic_error)?
                        .ok_or_else(busy)??;
                    count += 1;
                }
                cursor.acknowledge();
            }
        }
        if before == WorkClass::ALL.map(|c| budget.remaining(c)) {
            return Err(busy());
        }
    }
    if count == 0 {
        return Err(unsupported());
    }
    Ok(count)
}

/// Check the entire final TD against this fixed registration set. Callers
/// making a publication decision use `ProducerQualification` to carry its
/// observations through closed installation and commit.
pub fn check_final_thing(
    description: &str,
    instance: ProducerInstanceKey,
    limits: &ResourceLimits,
    budget: &mut WorkBudget,
    providers: &[&dyn ProducerCapability],
    prepared: &PreparedThing,
    behavior: impl Fn(ValidatedProducerScope<'_>, Operation) -> bool,
) -> CoreResult<()> {
    ProducerQualification::begin(providers, PublicationFacts::default(), budget)?.check_final(
        description,
        instance,
        limits,
        budget,
        prepared,
        behavior,
    )?;
    Ok(())
}

/// Observations of the actual eligible set, including every competitor. Start
/// before asking about managed scratch, then compare the exact final TD.
pub struct ProducerQualification<'a> {
    providers: &'a [&'a dyn ProducerCapability],
    facts: PublicationFacts,
}
impl<'a> ProducerQualification<'a> {
    pub fn begin(
        providers: &'a [&'a dyn ProducerCapability],
        mut facts: PublicationFacts,
        budget: &mut WorkBudget,
    ) -> CoreResult<Self> {
        observe_providers(providers, &mut facts, budget)?;
        Ok(Self { providers, facts })
    }
    pub fn backing_bytes(&self) -> Option<usize> {
        self.facts.backing_bytes()
    }
    pub fn check_final(
        self,
        description: &str,
        instance: ProducerInstanceKey,
        limits: &ResourceLimits,
        budget: &mut WorkBudget,
        prepared: &PreparedThing,
        behavior: impl Fn(ValidatedProducerScope<'_>, Operation) -> bool,
    ) -> CoreResult<PublicationFacts> {
        let providers = self.providers;
        // Requalify *all* common semantics and candidates, not only the first leaf.
        // Compare exact coordinates and selected providers, then verify their owned
        // definition with the same final semantic input.
        let count = visit_thing(
            description,
            instance,
            limits,
            budget,
            &behavior,
            |ordinal, input, budget| {
                let old = prepared.operations.get(ordinal).ok_or_else(unsupported)?;
                if old.site != *input.site
                    || old.form != input.form
                    || old.operation != input.operation
                {
                    return Err(unsupported());
                }
                let mut chosen = None;
                for (i, provider) in providers.iter().enumerate() {
                    budget
                        .consume(WorkClass::PlanningItems, 1)
                        .map_err(|_| busy())?;
                    let supported = if i == old.provider {
                        if old.binding != provider.identity() {
                            return Err(unsupported());
                        }
                        provider.check_final(input, old.definition.as_ref(), budget)?
                    } else {
                        provider.supports(input, budget)?
                    };
                    if supported {
                        if chosen.is_some() {
                            return Err(unsupported());
                        }
                        chosen = Some(i);
                    }
                }
                if chosen != Some(old.provider) {
                    return Err(unsupported());
                }
                Ok(())
            },
        )?;
        if count != prepared.operations.len() {
            return Err(unsupported());
        }
        drop(PublicationFacts::lock_current(&[&self.facts])?);
        Ok(self.facts)
    }
}
