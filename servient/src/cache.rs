use std::{cell::RefCell, collections::BTreeMap};

use clinkz_wot_protocol_bindings::{AffordanceRef, FormSelectionCriteria};
use clinkz_wot_td::{data_type::Operation, form::Form, thing::Thing};

/// Cache boundary for consumed Thing TDs used by Servient-level invocation APIs.
pub trait ConsumedThingCache {
    /// Retrieves a cached Thing Description by Thing id.
    fn get(&self, id: &str) -> Option<Thing>;

    /// Inserts or replaces a cached Thing Description by Thing id.
    fn insert(&mut self, id: String, thing: Thing) -> Option<Thing>;

    /// Removes a cached Thing Description by Thing id.
    fn remove(&mut self, id: &str) -> Option<Thing>;
}

/// Deterministic in-memory cache for consumed Thing TDs.
pub struct InMemoryConsumedThingCache {
    things: BTreeMap<String, Thing>,
}

impl InMemoryConsumedThingCache {
    /// Creates an empty consumed Thing cache.
    pub fn new() -> Self {
        Self {
            things: BTreeMap::new(),
        }
    }

    /// Returns the number of cached Thing Descriptions.
    pub fn len(&self) -> usize {
        self.things.len()
    }

    /// Returns true when the cache contains no Thing Descriptions.
    pub fn is_empty(&self) -> bool {
        self.things.is_empty()
    }
}

impl Default for InMemoryConsumedThingCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsumedThingCache for InMemoryConsumedThingCache {
    fn get(&self, id: &str) -> Option<Thing> {
        self.things.get(id).cloned()
    }

    fn insert(&mut self, id: String, thing: Thing) -> Option<Thing> {
        self.things.insert(id, thing)
    }

    fn remove(&mut self, id: &str) -> Option<Thing> {
        self.things.remove(id)
    }
}

/// Owned affordance location used by selected form cache keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectedFormCacheAffordance {
    /// A form declared at Thing level.
    Thing,
    /// A property affordance by name.
    Property(String),
    /// An action affordance by name.
    Action(String),
    /// An event affordance by name.
    Event(String),
}

impl SelectedFormCacheAffordance {
    pub(crate) fn from_affordance_ref(affordance: AffordanceRef<'_>) -> Self {
        match affordance {
            AffordanceRef::Thing => Self::Thing,
            AffordanceRef::Property(name) => Self::Property(name.to_owned()),
            AffordanceRef::Action(name) => Self::Action(name.to_owned()),
            AffordanceRef::Event(name) => Self::Event(name.to_owned()),
        }
    }
}

/// Cache key for a Servient-selected TD form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedFormCacheKey {
    /// Thing id used for the consumed interaction.
    pub thing_id: String,
    /// Affordance location used for form selection.
    pub affordance: SelectedFormCacheAffordance,
    /// Required effective operation.
    pub operation: Operation,
    /// Optional required form content type.
    pub content_type: Option<String>,
    /// Optional required form subprotocol.
    pub subprotocol: Option<String>,
}

impl SelectedFormCacheKey {
    /// Creates a cache key from a Thing id, affordance location, and selection criteria.
    pub fn new(
        thing_id: impl Into<String>,
        affordance: SelectedFormCacheAffordance,
        criteria: FormSelectionCriteria<'_>,
    ) -> Self {
        Self {
            thing_id: thing_id.into(),
            affordance,
            operation: criteria.operation,
            content_type: criteria.content_type.map(str::to_owned),
            subprotocol: criteria.subprotocol.map(str::to_owned),
        }
    }
}

/// Cache boundary for selected TD forms used by Servient-level invocation APIs.
pub trait SelectedFormCache {
    /// Retrieves a cached form selection.
    fn get(&self, key: &SelectedFormCacheKey) -> Option<Form>;

    /// Inserts or replaces a cached form selection.
    fn insert(&self, key: SelectedFormCacheKey, form: Form) -> Option<Form>;

    /// Removes a cached form selection.
    fn remove(&self, key: &SelectedFormCacheKey) -> Option<Form>;

    /// Removes all cached form selections for a Thing id.
    fn remove_thing(&self, id: &str);
}

/// Deterministic in-memory cache for Servient-selected TD forms.
pub struct InMemorySelectedFormCache {
    forms: RefCell<Vec<(SelectedFormCacheKey, Form)>>,
}

impl InMemorySelectedFormCache {
    /// Creates an empty selected form cache.
    pub fn new() -> Self {
        Self {
            forms: RefCell::new(Vec::new()),
        }
    }

    /// Returns the number of cached form selections.
    pub fn len(&self) -> usize {
        self.forms.borrow().len()
    }

    /// Returns true when the cache contains no selected forms.
    pub fn is_empty(&self) -> bool {
        self.forms.borrow().is_empty()
    }
}

impl Default for InMemorySelectedFormCache {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectedFormCache for InMemorySelectedFormCache {
    fn get(&self, key: &SelectedFormCacheKey) -> Option<Form> {
        self.forms
            .borrow()
            .iter()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, form)| form.clone())
    }

    fn insert(&self, key: SelectedFormCacheKey, form: Form) -> Option<Form> {
        let mut forms = self.forms.borrow_mut();
        if let Some((_, cached_form)) = forms.iter_mut().find(|(candidate, _)| *candidate == key) {
            let previous = cached_form.clone();
            *cached_form = form;
            Some(previous)
        } else {
            forms.push((key, form));
            None
        }
    }

    fn remove(&self, key: &SelectedFormCacheKey) -> Option<Form> {
        let mut forms = self.forms.borrow_mut();
        forms
            .iter()
            .position(|(candidate, _)| candidate == key)
            .map(|index| forms.remove(index).1)
    }

    fn remove_thing(&self, id: &str) {
        self.forms
            .borrow_mut()
            .retain(|(key, _)| key.thing_id != id);
    }
}

/// Protocol-neutral cached binding plan for a criteria-selected remote request.
#[derive(Debug, Clone, PartialEq)]
pub struct BindingPlan {
    /// Selected TD form for the remote interaction.
    pub form: Form,
    /// Index of the protocol binding factory selected for this form.
    pub binding_factory_index: usize,
}

/// Cache boundary for criteria-selected forms and protocol binding factories.
pub trait BindingPlanCache {
    /// Retrieves a cached binding plan by the same key used for selected forms.
    fn get(&self, key: &SelectedFormCacheKey) -> Option<BindingPlan>;

    /// Inserts or replaces a cached binding plan.
    fn insert(&self, key: SelectedFormCacheKey, plan: BindingPlan) -> Option<BindingPlan>;

    /// Removes a cached binding plan.
    fn remove(&self, key: &SelectedFormCacheKey) -> Option<BindingPlan>;

    /// Removes all cached binding plans for a Thing id.
    fn remove_thing(&self, id: &str);
}

/// Deterministic in-memory cache for Servient binding plans.
pub struct InMemoryBindingPlanCache {
    plans: RefCell<Vec<(SelectedFormCacheKey, BindingPlan)>>,
}

impl InMemoryBindingPlanCache {
    /// Creates an empty binding plan cache.
    pub fn new() -> Self {
        Self {
            plans: RefCell::new(Vec::new()),
        }
    }

    /// Returns the number of cached binding plans.
    pub fn len(&self) -> usize {
        self.plans.borrow().len()
    }

    /// Returns true when the cache contains no binding plans.
    pub fn is_empty(&self) -> bool {
        self.plans.borrow().is_empty()
    }
}

impl Default for InMemoryBindingPlanCache {
    fn default() -> Self {
        Self::new()
    }
}

impl BindingPlanCache for InMemoryBindingPlanCache {
    fn get(&self, key: &SelectedFormCacheKey) -> Option<BindingPlan> {
        self.plans
            .borrow()
            .iter()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, plan)| plan.clone())
    }

    fn insert(&self, key: SelectedFormCacheKey, plan: BindingPlan) -> Option<BindingPlan> {
        let mut plans = self.plans.borrow_mut();
        if let Some((_, cached_plan)) = plans.iter_mut().find(|(candidate, _)| *candidate == key) {
            let previous = cached_plan.clone();
            *cached_plan = plan;
            Some(previous)
        } else {
            plans.push((key, plan));
            None
        }
    }

    fn remove(&self, key: &SelectedFormCacheKey) -> Option<BindingPlan> {
        let mut plans = self.plans.borrow_mut();
        plans
            .iter()
            .position(|(candidate, _)| candidate == key)
            .map(|index| plans.remove(index).1)
    }

    fn remove_thing(&self, id: &str) {
        self.plans
            .borrow_mut()
            .retain(|(key, _)| key.thing_id != id);
    }
}
