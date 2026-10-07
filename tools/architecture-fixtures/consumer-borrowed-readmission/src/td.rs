//! Compiled inside the existing TD source candidate, never production TD.
//! Storage facts reuse thing_build; discovery/predicates reuse shared kernels.
use crate::{
    basic_kernel::{self as b, Field, Owner, OwnerKind, Site as ErrorSite},
    basic_typed::security_context,
    data_schema::DataSchema,
    data_type::Operation,
    form::Form,
    schema_access::TypedAccess,
    schema_kernel::{self as s, SchemaAccess},
    thing::Thing,
};
use alloc::{
    collections::{BTreeMap, btree_map},
    string::String,
};
use clinkz_wot_foundation::{WorkBudget, WorkClass as W};
use core::cell::Cell;
use serde_json::{Number, Value};

#[path = "typed.rs"]
mod typed;
use typed::{self as t, Atom, Children, Map, Task};
#[path = "policy.rs"]
mod policy;
use policy::Policy;
pub use policy::{
    CATALOG, OPERATION, RESOURCE_INTERPRETATION_REVISION, ValidatedThingAdmissionConfig,
    ValidatedThingConfigError, ValidatedThingConfigErrorKind,
};
#[path = "storage.rs"]
mod storage;
use storage::{Site, Storage};
#[path = "uri.rs"]
mod uri;
use clinkz_wot_foundation::{AdmissionLedger, ResourceKind as R};
pub use uri::Observations as UriObservations;
#[path = "api.rs"]
mod api;
pub use api::*;
#[path = "inspect.rs"]
mod inspect;
use inspect::Scope;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cause {
    Configuration,
    Arithmetic,
    Resource {
        kind: R,
        configured: u64,
        observed: u64,
    },
    Allocation {
        requested_bytes: u64,
    },
    Nodes,
    Members,
    Depth,
    Text,
    Content,
    Number,
    Lifetime,
    Memory,
    Cancelled,
    MissingId,
    Security,
    Uri,
    Invalid(b::InlineInvalid),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Trace {
    pub work: [u64; 12],
    pub nodes: usize,
    pub content: usize,
    pub next_calls: u64,
    pub compared_bytes: u64,
    pub schema_nodes: u64,
    pub projections: u64,
    pub frame_moves: u64,
    pub max_depth: usize,
    pub allocations: u64,
    pub releases: u64,
    pub live: u64,
    pub peak: u64,
    pub largest: u64,
    pub attempts: u64,
    pub source_text_reads: u64,
    pub content_checksum: u64,
    pub kinds: [u64; 128],
    pub text_bytes: u64,
    pub extension_bytes: u64,
    pub container_depth: u64,
    pub array_items: u64,
    pub object_members: u64,
    pub affordances: u64,
    pub forms_context: u64,
    pub forms: u64,
    pub responses: u64,
    pub variables: u64,
    pub supplied_schemas: u64,
    pub schema_depth: u64,
    pub schema_edges: u64,
    pub security_branches: u64,
    pub uri_source: u64,
    pub number_bytes: u64,
}
impl Default for Trace {
    fn default() -> Self {
        Self {
            work: [0; 12],
            nodes: 0,
            content: 0,
            next_calls: 0,
            compared_bytes: 0,
            schema_nodes: 0,
            projections: 0,
            frame_moves: 0,
            max_depth: 0,
            allocations: 0,
            releases: 0,
            live: 0,
            peak: 0,
            largest: 0,
            attempts: 0,
            source_text_reads: 0,
            content_checksum: 0,
            kinds: [0; 128],
            text_bytes: 0,
            extension_bytes: 0,
            container_depth: 0,
            array_items: 0,
            object_members: 0,
            affordances: 0,
            forms_context: 0,
            forms: 0,
            responses: 0,
            variables: 0,
            supplied_schemas: 0,
            schema_depth: 0,
            schema_edges: 0,
            security_branches: 0,
            uri_source: 0,
            number_bytes: 0,
        }
    }
}
fn invalid(site: ErrorSite, rule: b::Rule<'_>) -> Cause {
    Cause::Invalid(b::InlineInvalid {
        site,
        rule: b::InlineRule::Basic(rule.kind()),
        schema_ordinal: None,
    })
}
fn schema_invalid(site: ErrorSite, ordinal: u64, rule: s::Rule) -> Cause {
    Cause::Invalid(b::InlineInvalid {
        site,
        rule: b::InlineRule::Schema(rule),
        schema_ordinal: Some(ordinal),
    })
}

fn pay(
    budget: &mut WorkBudget,
    lifetime: &mut u64,
    trace: &mut Trace,
    costs: &[(W, u64)],
) -> Result<bool, Cause> {
    if costs.iter().any(|&(c, n)| budget.remaining(c) < n) {
        return Ok(false);
    }
    let n = costs
        .iter()
        .try_fold(0u64, |sum, (_, n)| sum.checked_add(*n))
        .ok_or(Cause::Arithmetic)?;
    if *lifetime < n {
        let spent = trace
            .work
            .iter()
            .try_fold(0u64, |sum, n| sum.checked_add(*n))
            .ok_or(Cause::Arithmetic)?;
        return Err(Cause::Resource {
            kind: R::DocumentValidationWorkUnitsMax,
            configured: spent.checked_add(*lifetime).ok_or(Cause::Arithmetic)?,
            observed: spent.checked_add(n).ok_or(Cause::Arithmetic)?,
        });
    }
    for &(c, n) in costs {
        budget.consume(c, n).unwrap();
        trace.work[c as usize] += n;
    }
    *lifetime -= n;
    Ok(true)
}
// Deliberately conservative debit for a standard-container iterator primitive:
// it includes seek/ascend work; it is not a claim of one CPU instruction.
fn iter_cost(length: usize) -> u64 {
    length as u64 + 1
}

enum EnumChildren<'a> {
    Typed(Children<'a>),
    Json(serde_json::map::Iter<'a>),
}
struct Structure<'a> {
    parent: Task<'a>,
    scope: Scope,
    members: usize,
    children: EnumChildren<'a>,
    count: usize,
    next: usize,
}
#[derive(Clone, Copy)]
enum Aff<'a> {
    Property(&'a crate::affordance::PropertyAffordance),
    Action(&'a crate::affordance::ActionAffordance),
    Event(&'a crate::affordance::EventAffordance),
}
impl<'a> Aff<'a> {
    fn forms(self) -> &'a [Form] {
        match self {
            Self::Property(v) => &v._interaction.forms,
            Self::Action(v) => &v._interaction.forms,
            Self::Event(v) => &v._interaction.forms,
        }
    }
    fn variables(self) -> Option<&'a BTreeMap<String, DataSchema>> {
        match self {
            Self::Property(v) => v._interaction.uri_variables.as_ref(),
            Self::Action(v) => v._interaction.uri_variables.as_ref(),
            Self::Event(v) => v._interaction.uri_variables.as_ref(),
        }
    }
    fn schema(self, field: Field) -> Option<&'a DataSchema> {
        match (self, field) {
            (Self::Property(v), Field::PropertySchema) => Some(&v._schema),
            (Self::Action(v), Field::Input) => v.input.as_ref(),
            (Self::Action(v), Field::Output) => v.output.as_ref(),
            (Self::Event(v), Field::Subscription) => v.subscription.as_ref(),
            (Self::Event(v), Field::Data) => v.data.as_ref(),
            (Self::Event(v), Field::DataResponse) => v.data_response.as_ref(),
            (Self::Event(v), Field::Cancellation) => v.cancellation.as_ref(),
            _ => unreachable!(),
        }
    }
}
#[derive(Clone, Copy)]
enum Names<'a> {
    Strings(&'a [String]),
    Json(&'a [Value]),
    One(&'a str),
    Empty,
}
impl<'a> Names<'a> {
    fn from_value(v: Option<&'a Value>) -> Self {
        match v {
            Some(Value::Array(v)) => Self::Json(v),
            Some(Value::String(v)) => Self::One(v),
            _ => Self::Empty,
        }
    }
    fn raw_len(self) -> usize {
        match self {
            Self::Strings(v) => v.len(),
            Self::Json(v) => v.len(),
            Self::One(_) => 1,
            Self::Empty => 0,
        }
    }
    fn at(self, n: usize) -> Option<&'a str> {
        match self {
            Self::Strings(v) => Some(&v[n]),
            Self::Json(v) => v[n].as_str(),
            Self::One(v) => Some(v),
            Self::Empty => None,
        }
    }
}
struct Sec<'a> {
    definition: &'a crate::security_scheme::SecurityScheme,
    extras: btree_map::Iter<'a, String, Value>,
    one: Names<'a>,
    all: Names<'a>,
    name: Option<&'a str>,
    flow: &'a str,
    authorization: bool,
    token: bool,
    group: usize,
    position: usize,
    counts: [usize; 2],
    empty: [Option<usize>; 2],
}
impl<'a> Sec<'a> {
    fn new(definition: &'a crate::security_scheme::SecurityScheme) -> Self {
        use crate::security_scheme::SecurityScheme as S;
        let mut out = Self {
            definition,
            extras: security_context(definition)._extra_fields.iter(),
            one: Names::Empty,
            all: Names::Empty,
            name: None,
            flow: "",
            authorization: false,
            token: false,
            group: 0,
            position: 0,
            counts: [0; 2],
            empty: [None; 2],
        };
        match definition {
            S::Combo(v) => {
                out.one = Names::Strings(&v.one_of);
                out.all = Names::Strings(&v.all_of);
            }
            S::APIKey(v) => out.name = v.name.as_deref(),
            S::OAuth2(v) => {
                out.flow = &v.flow;
                out.authorization = v.authorization.is_some();
                out.token = v.token.is_some();
            }
            _ => {}
        }
        out
    }
}
struct Schema<'a> {
    node: &'a DataSchema,
    walk: s::Walk,
    ordinal: u64,
    children: Option<btree_map::Iter<'a, String, DataSchema>>,
    extras: Option<btree_map::Iter<'a, String, Value>>,
    extras_started: bool,
    unsigned: [Option<u64>; 4],
    numbers: [Option<&'a Number>; 5],
    numeric: s::NumericCursor,
}
impl<'a> Schema<'a> {
    fn new(node: &'a DataSchema, ordinal: u64) -> Self {
        Self {
            node,
            walk: s::Walk::default(),
            ordinal,
            children: None,
            extras: None,
            extras_started: false,
            unsigned: [None; 4],
            numbers: [None; 5],
            numeric: s::NumericCursor::default(),
        }
    }
}
enum Job<'a> {
    Enter(Task<'a>, Scope),
    Structure(Structure<'a>),
    Text(&'a str, usize),
    References(Names<'a>, usize, ErrorSite),
    Lookup(Search<'a>, ErrorSite),
    Definition(Sec<'a>, ErrorSite),
    Groups(Sec<'a>, ErrorSite),
    SchemaMap(btree_map::Iter<'a, String, DataSchema>, ErrorSite, usize),
    Schema(Schema<'a>, ErrorSite),
    Operations(&'a [Form], usize, usize, Owner),
    FormSecurity(&'a [Form], usize, Owner),
}
const _: () = assert!(!core::mem::needs_drop::<Job<'static>>());

/// A scan never restarts on Pending; no native lookup or implicit string
/// comparison. Every actual compared byte is a separate paid operation.
struct Search<'a> {
    iter: btree_map::Iter<'a, String, crate::security_scheme::SecurityScheme>,
    target: &'a str,
    current: Option<(&'a str, &'a crate::security_scheme::SecurityScheme)>,
    byte: usize,
    done: Option<Option<&'a crate::security_scheme::SecurityScheme>>,
}
impl<'a> Search<'a> {
    fn new(thing: &'a Thing, target: &'a str) -> Self {
        Self {
            iter: thing.security_definitions.iter(),
            target,
            current: None,
            byte: 0,
            done: None,
        }
    }
    fn costs(&self, thing: &Thing) -> [(W, u64); 2] {
        [
            (
                W::DocumentNodes,
                if self.current.is_none() {
                    iter_cost(thing.security_definitions.len())
                } else {
                    0
                },
            ),
            (
                W::CodecInputBytes,
                if self.current.is_some() { 2 } else { 0 },
            ),
        ]
    }
    fn tick(&mut self, trace: &mut Trace) {
        if let Some((key, definition)) = self.current {
            if self.byte == self.target.len() {
                self.done = Some(Some(definition));
                return;
            }
            trace.compared_bytes += 2;
            if key.as_bytes()[self.byte] != self.target.as_bytes()[self.byte] {
                self.current = None;
                self.byte = 0;
            } else {
                self.byte += 1;
            }
        } else {
            trace.next_calls += 1;
            match self.iter.next() {
                None => self.done = Some(None),
                Some((key, value)) if key.len() == self.target.len() => {
                    self.current = Some((key, value));
                    self.byte = 0;
                }
                Some(_) => {}
            }
        }
    }
}

/// All iterator references point into the external immutable input. The frame
/// arena may move; no frame points to another frame or to cursor-owned bytes.
struct Validation<'a> {
    thing: &'a Thing,
    policy: Policy,
    lifetime: u64,
    trace: Trace,
    storage: Storage<Job<'a>>,
    initial: bool,
    basic: bool,
    walk: b::Walk,
    definitions: Option<btree_map::Iter<'a, String, crate::security_scheme::SecurityScheme>>,
    properties: Option<btree_map::Iter<'a, String, crate::affordance::PropertyAffordance>>,
    actions: Option<btree_map::Iter<'a, String, crate::affordance::ActionAffordance>>,
    events: Option<btree_map::Iter<'a, String, crate::affordance::EventAffordance>>,
    current: Option<(Owner, Aff<'a>)>,
    next_schema: u64,
}
enum Progress<'a> {
    Pending(Validation<'a>),
    Complete(Validated<'a>),
    Failed(Cause, Trace, bool),
}
/// Private fields and no raw-source getter: this proof can only cross the paid
/// structural + complete Basic barrier. It carries the unreplenishable TD debit.
struct Validated<'a> {
    storage: Storage<Job<'a>>,
    thing: &'a Thing,
    policy: Policy,
    lifetime: u64,
    trace: Trace,
}
impl<'a> Validation<'a> {
    fn new(thing: &'a Thing, policy: Policy, ledger: AdmissionLedger) -> Self {
        Self {
            thing,
            policy,
            lifetime: policy.get(R::DocumentValidationWorkUnitsMax),
            trace: Trace::default(),
            storage: Storage::new(
                ledger,
                [
                    R::AdmissionTemporaryBytesPerOperationMax,
                    R::AdmissionTemporaryBytesGlobalMax,
                    R::PeakLiveBytesPerAdmissionMax,
                    R::AdmissionPeakLiveBytesGlobalMax,
                    R::EngineLiveBytesGlobalMax,
                    R::LargestContiguousAllocationBytesMax,
                ]
                .map(|r| (r, policy.get(r))),
            ),
            initial: true,
            basic: false,
            walk: b::Walk::default(),
            definitions: None,
            properties: None,
            actions: None,
            events: None,
            current: None,
            next_schema: 0,
        }
    }
    pub fn trace(&self) -> Trace {
        let mut t = self.trace;
        let f = self.storage.footprint();
        t.allocations = self.storage.allocations();
        t.releases = self.storage.releases();
        t.live = self.storage.live_bytes();
        t.peak = f.reservation_peak_bytes;
        t.largest = f.largest_request_bytes;
        t.attempts = self.storage.attempts();
        t
    }
    pub fn lifetime_remaining(&self) -> u64 {
        self.lifetime
    }
    pub fn transferring(&self) -> bool {
        self.storage.transferring().is_some()
    }
    pub fn frame_request_pending(&self) -> bool {
        !self.transferring()
            && self.storage.len(Site::Frames) + 2 > self.storage.capacity(Site::Frames)
    }
    pub fn numeric_charge(&mut self) -> Option<u64> {
        if self.storage.transferring().is_some() || self.storage.len(Site::Frames) == 0 {
            return None;
        }
        if let Job::Schema(f, _) = self.storage.frame_mut() {
            let mut walk = f.walk;
            if walk.action(
                TypedAccess.one_of_count(f.node),
                TypedAccess.child_count(f.node),
            ) == s::Action::Numeric
            {
                let mut numeric = f.numeric;
                let mut cost = None;
                let _ = numeric.step(f.numbers, |n| {
                    cost = Some(n.as_str().len() as u64);
                    s::projection_step::ProjectionProgress::Pending
                });
                return cost;
            }
        }
        None
    }
    pub fn step(mut self, budget: &mut WorkBudget, cancel: bool) -> Progress<'a> {
        if cancel {
            return self.fail(Cause::Cancelled);
        }
        if budget.is_exhausted() {
            return Progress::Pending(self);
        }
        match self.tick(budget) {
            Ok(Some(_)) => Progress::Pending(self),
            Ok(None) => {
                self.storage.clear();
                let trace = self.trace();
                Progress::Complete(Validated {
                    storage: self.storage,
                    thing: self.thing,
                    policy: self.policy,
                    lifetime: self.lifetime,
                    trace,
                })
            }
            Err(c) => self.fail(c),
        }
    }
    fn fail(mut self, c: Cause) -> Progress<'a> {
        self.storage.clear();
        Progress::Failed(c, self.trace(), self.basic)
    }
    fn debit(&mut self, budget: &mut WorkBudget, costs: &[(W, u64)]) -> Result<bool, Cause> {
        pay(budget, &mut self.lifetime, &mut self.trace, costs)
    }
    fn push(&mut self, job: Job<'a>) -> Result<(), Cause> {
        if self.storage.len(Site::Frames) == self.policy.frames {
            return Err(Cause::Depth);
        }
        self.storage.push_frame(job);
        self.trace.max_depth = self.trace.max_depth.max(self.storage.len(Site::Frames));
        Ok(())
    }
    fn room(&mut self, budget: &mut WorkBudget) -> Result<bool, Cause> {
        if self.storage.transferring().is_some() {
            if !self.debit(budget, &[(W::DocumentNodes, 1)])? {
                return Ok(false);
            }
            self.storage.copy_one();
            self.trace.frame_moves += 1;
            return Ok(false);
        }
        let wanted = self.storage.len(Site::Frames) + 2;
        if wanted > self.storage.capacity(Site::Frames) {
            let cap = (self.storage.capacity(Site::Frames).max(2) * 2).min(self.policy.frames);
            self.storage.check_grow(Site::Frames, cap)?;
            if !self.debit(budget, &[(W::CleanupItems, 1)])? {
                return Ok(false);
            }
            self.storage.begin_grow(Site::Frames, cap)?;
            return Ok(false);
        }
        Ok(true)
    }
    fn tick(&mut self, budget: &mut WorkBudget) -> Result<Option<bool>, Cause> {
        // Progress in transfer/growth is visible even when semantic work waits.
        let before = self.trace.work;
        if !self.room(budget)? {
            return Ok(Some(self.trace.work != before));
        }
        if self.initial {
            if !self.debit(budget, &[(W::DocumentNodes, 1)])? {
                return Ok(Some(false));
            }
            self.push(Job::Enter(Task::Thing(self.thing), Scope::default()))?;
            self.initial = false;
            return Ok(Some(true));
        }
        if self.storage.len(Site::Frames) == 0 {
            if !self.basic {
                let n = iter_cost(self.thing.security_definitions.len())
                    + self
                        .thing
                        .properties
                        .as_ref()
                        .map_or(0, |v| iter_cost(v.len()))
                    + self
                        .thing
                        .actions
                        .as_ref()
                        .map_or(0, |v| iter_cost(v.len()))
                    + self.thing.events.as_ref().map_or(0, |v| iter_cost(v.len()));
                if !self.debit(budget, &[(W::DocumentNodes, n)])? {
                    return Ok(Some(false));
                }
                self.definitions = Some(self.thing.security_definitions.iter());
                self.properties = self.thing.properties.as_ref().map(BTreeMap::iter);
                self.actions = self.thing.actions.as_ref().map(BTreeMap::iter);
                self.events = self.thing.events.as_ref().map(BTreeMap::iter);
                self.basic = true;
            }
            return self.global(budget);
        }
        let job = self.storage.frame_mut();
        let mut costs = [
            (W::DocumentNodes, 1),
            (W::CodecInputBytes, 0),
            (W::SecurityBranches, 0),
        ];
        match job {
            Job::Enter(Task::Map(map), _) => {
                let n = map_len(*map);
                if n as u64 > self.policy.get(R::JsonMembersPerObjectMax) {
                    return Err(Cause::Members);
                }
                costs[0].1 = iter_cost(n);
            }
            Job::Enter(Task::Value(Value::Object(map)), _) => {
                inspect::limit(self.policy, R::JsonMembersPerObjectMax, map.len() as u64)?;
                costs[0].1 = iter_cost(map.len());
            }
            Job::Structure(f) => {
                costs[0].1 = if inspect::map(f.parent) {
                    iter_cost(f.count)
                } else {
                    1
                }
            }
            Job::Text(_, _) => {
                costs[0].1 = 0;
                costs[1].1 = 1;
            }
            Job::Enter(Task::Schema(_), _) => costs[0] = (W::JsonSchemaNodes, 1),
            Job::References(_, _, _) => {
                costs[0] = (
                    W::DocumentNodes,
                    iter_cost(self.thing.security_definitions.len()),
                );
                costs[2] = (W::SecurityBranches, 1);
            }
            Job::Groups(_, _) => costs[0] = (W::SecurityBranches, 1),
            Job::Lookup(search, _) => {
                let c = search.costs(self.thing);
                costs[0] = c[0];
                costs[1] = c[1];
            }
            Job::Definition(sec, _) => {
                costs[0] = (
                    W::DocumentNodes,
                    iter_cost(security_context(sec.definition)._extra_fields.len()),
                );
                costs[1].1 = 512;
                costs[2] = (W::SecurityBranches, 1);
            }
            Job::Schema(frame, _) => {
                costs[0] = (
                    W::JsonSchemaNodes,
                    iter_cost(
                        frame
                            .node
                            .context()
                            ._extra_fields
                            .len()
                            .max(TypedAccess.child_count(frame.node))
                            .max(TypedAccess.one_of_count(frame.node)),
                    ),
                );
                // Discovery can skip empty OneOf/child phases. Preview it on
                // a copy so insufficient credit cannot change the owner.
                let mut preview = frame.walk;
                let action = preview.action(
                    TypedAccess.one_of_count(frame.node),
                    TypedAccess.child_count(frame.node),
                );
                if matches!(action, s::Action::Child(_))
                    && matches!(frame.node, DataSchema::Object(_))
                    && frame.children.is_none()
                {
                    costs[0].1 *= 2;
                }
                if action == s::Action::Numeric {
                    let mut preview = frame.numeric;
                    let mut charge = 0;
                    let _ = preview.step(frame.numbers, |n| {
                        charge = n.as_str().len() as u64;
                        s::projection_step::ProjectionProgress::Pending
                    });
                    costs[1].1 = charge;
                } else if frame.extras.is_some() || !frame.extras_started {
                    // Recognizing nine fixed field names is a bounded atomic
                    // island; as_u64 has an additional complete Number debit.
                    costs[1].1 = 512 + self.policy.get(R::NumberLexemeBytesMax);
                }
            }
            Job::SchemaMap(iter, _, n) => costs[0].1 = iter_cost(iter.len() + *n),
            _ => {}
        }
        if !self.debit(budget, &costs)? {
            return Ok(Some(false));
        }
        let job = self.storage.pop_frame();
        match job {
            Job::Enter(task, scope) => {
                let d = t::describe(task);
                let scope = inspect::visit(task, scope, &d, &mut self.trace, self.policy)?;
                if let Some(text) = d.text {
                    if !text.is_empty() {
                        self.push(Job::Text(text, 0))?;
                    }
                }
                if d.count != 0 {
                    let children = match d.children {
                        Children::Json { map, .. } => EnumChildren::Json(map.iter()),
                        other => EnumChildren::Typed(other),
                    };
                    self.push(Job::Structure(Structure {
                        parent: task,
                        scope,
                        members: 0,
                        children,
                        count: d.count,
                        next: 0,
                    }))?;
                }
            }
            Job::Text(text, n) => {
                let byte = text.as_bytes()[n];
                self.trace.source_text_reads += 1;
                self.trace.content_checksum = self
                    .trace
                    .content_checksum
                    .wrapping_add(u64::from(byte) + 1);
                if n + 1 < text.len() {
                    self.push(Job::Text(text, n + 1))?;
                }
            }
            Job::Structure(mut f) => {
                if f.next < f.count {
                    self.trace.next_calls += 1;
                    let task = match &mut f.children {
                        EnumChildren::Typed(c) => c.next(f.next),
                        EnumChildren::Json(i) => {
                            i.next().map(|(k, v)| Task::Entry(k, Atom::Value(v)))
                        }
                    };
                    let scope = inspect::child_scope(f.parent, f.next, f.scope);
                    let task = if let Task::Operations(ops) = f.parent {
                        Some(Task::Text(ops[f.next].as_str(), t::OPERATION))
                    } else {
                        task
                    };
                    f.next += 1;
                    if task.is_some()
                        && !inspect::array(f.parent)
                        && !matches!(f.parent, Task::Entry(..))
                    {
                        f.members += 1;
                        self.trace.object_members = self.trace.object_members.max(f.members as u64);
                        inspect::limit(self.policy, R::JsonMembersPerObjectMax, f.members as u64)?;
                    }
                    self.push(Job::Structure(f))?;
                    if let Some(task) = task {
                        self.push(Job::Enter(task, scope))?;
                    }
                }
            }
            Job::References(names, n, site) => {
                if n < names.raw_len() {
                    let name = names.at(n);
                    let mut next_site = site;
                    if name.is_some() {
                        next_site.member += 1;
                    }
                    self.push(Job::References(names, n + 1, next_site))?;
                    if let Some(name) = name {
                        self.push(Job::Lookup(Search::new(self.thing, name), site))?;
                    }
                }
            }
            Job::Lookup(mut search, site) => {
                search.tick(&mut self.trace);
                match search.done {
                    Some(None) => return Err(invalid(site, b::Rule::Undefined(search.target))),
                    Some(Some(_)) => {}
                    None => self.push(Job::Lookup(search, site))?,
                }
            }
            Job::Definition(mut sec, site) => {
                self.trace.next_calls += 1;
                if let Some((key, value)) = sec.extras.next() {
                    use crate::security_scheme::SecurityScheme as S;
                    match key.as_str() {
                        "oneOf" if !matches!(sec.definition, S::Combo(_)) => {
                            sec.one = Names::from_value(Some(value))
                        }
                        "allOf" if !matches!(sec.definition, S::Combo(_)) => {
                            sec.all = Names::from_value(Some(value))
                        }
                        "name" if !matches!(sec.definition, S::APIKey(_)) => {
                            sec.name = value.as_str()
                        }
                        "flow" if !matches!(sec.definition, S::OAuth2(_)) => {
                            sec.flow = value.as_str().unwrap_or("")
                        }
                        "authorization" if !matches!(sec.definition, S::OAuth2(_)) => {
                            sec.authorization = value.as_str().is_some_and(|v| !v.is_empty())
                        }
                        "token" if !matches!(sec.definition, S::OAuth2(_)) => {
                            sec.token = value.as_str().is_some_and(|v| !v.is_empty())
                        }
                        _ => {}
                    }
                    self.push(Job::Definition(sec, site))?;
                } else {
                    match b::scheme_kind(sec.definition.scheme()).map_err(|r| invalid(site, r))? {
                        b::Scheme::Combo => self.push(Job::Groups(sec, site))?,
                        b::Scheme::ApiKey => {
                            b::required(sec.name.is_some_and(|v| !v.is_empty()))
                                .map_err(|r| invalid(ErrorSite::new(site.owner, Field::Name), r))?
                        }
                        b::Scheme::OAuth => {
                            if b::flow_requires_endpoints(sec.flow)
                                .map_err(|r| invalid(ErrorSite::new(site.owner, Field::Flow), r))?
                            {
                                for (field, present) in [
                                    (Field::Authorization, sec.authorization),
                                    (Field::Token, sec.token),
                                ] {
                                    b::required(present).map_err(|r| {
                                        invalid(ErrorSite::new(site.owner, field), r)
                                    })?;
                                }
                            }
                        }
                        b::Scheme::Other => {}
                    }
                }
            }
            Job::Groups(mut sec, site) => {
                let names = [sec.one, sec.all][sec.group];
                if sec.position < names.raw_len() {
                    if let Some(v) = names.at(sec.position) {
                        if v.is_empty() && sec.empty[sec.group].is_none() {
                            sec.empty[sec.group] = Some(sec.counts[sec.group]);
                        }
                        sec.counts[sec.group] += 1;
                    }
                    sec.position += 1;
                    self.push(Job::Groups(sec, site))?;
                } else if sec.group == 0 {
                    sec.group = 1;
                    sec.position = 0;
                    self.push(Job::Groups(sec, site))?;
                } else {
                    b::combo_present(sec.counts[0], sec.counts[1]).map_err(|r| invalid(site, r))?;
                    for (n, field) in [Field::OneOf, Field::AllOf].into_iter().enumerate() {
                        b::combo_group(sec.counts[n], sec.empty[n]).map_err(|(member, r)| {
                            let mut site = ErrorSite::new(site.owner, field);
                            site.member = member;
                            invalid(site, r)
                        })?;
                    }
                    self.push(Job::References(
                        sec.all,
                        0,
                        ErrorSite::new(site.owner, Field::AllOf),
                    ))?;
                    self.push(Job::References(
                        sec.one,
                        0,
                        ErrorSite::new(site.owner, Field::OneOf),
                    ))?;
                }
            }
            Job::SchemaMap(mut iter, mut site, n) => {
                self.trace.next_calls += 1;
                if let Some((_, node)) = iter.next() {
                    site.index = n;
                    self.push(Job::SchemaMap(iter, site, n + 1))?;
                    self.next_schema = 0;
                    self.schema(node, site)?;
                }
            }
            Job::Schema(mut f, site) => {
                let node = f.node;
                let a = f.walk.action(
                    TypedAccess.one_of_count(node),
                    TypedAccess.child_count(node),
                );
                match a {
                    s::Action::Type => {
                        s::check_type(TypedAccess.data_type(node), TypedAccess.kind(node))
                            .map_err(|r| schema_invalid(site, f.ordinal, r))?
                    }
                    s::Action::Flags => s::check_flags(TypedAccess.flags(node))
                        .map_err(|r| schema_invalid(site, f.ordinal, r))?,
                    s::Action::OneOf(n) => {
                        f.walk.advance();
                        self.push(Job::Schema(f, site))?;
                        self.schema(TypedAccess.one_of(node, n), site)?;
                        return Ok(Some(true));
                    }
                    s::Action::Unsigned => {
                        if !f.extras_started {
                            f.extras = Some(node.context()._extra_fields.iter());
                            f.extras_started = true;
                            self.push(Job::Schema(f, site))?;
                            return Ok(Some(true));
                        }
                        if let Some(mut iter) = f.extras.take() {
                            self.trace.next_calls += 1;
                            if let Some((key, value)) = iter.next() {
                                for (n, field) in [
                                    s::Field::MinItems,
                                    s::Field::MaxItems,
                                    s::Field::MinLength,
                                    s::Field::MaxLength,
                                ]
                                .into_iter()
                                .enumerate()
                                {
                                    if key == field.name() {
                                        f.unsigned[n] = value.as_u64();
                                    }
                                }
                                for (n, field) in s::NUMERIC_FIELDS.into_iter().enumerate() {
                                    if key == field.name() {
                                        f.numbers[n] = value.as_number();
                                    }
                                }
                                f.extras = Some(iter);
                                self.push(Job::Schema(f, site))?;
                                return Ok(Some(true));
                            }
                        }
                        for n in 0..2 {
                            s::check_unsigned_pair(n, [f.unsigned[n * 2], f.unsigned[n * 2 + 1]])
                                .map_err(|r| schema_invalid(site, f.ordinal, r))?;
                        }
                    }
                    s::Action::Numeric => {
                        let mut projected = false;
                        let result = f.numeric.step(f.numbers, |n| {
                            if projected {
                                return s::projection_step::ProjectionProgress::Pending;
                            }
                            projected = true;
                            self.trace.projections += 1;
                            s::projection_step::ProjectionProgress::Complete(n.as_f64())
                        });
                        match result {
                            s::projection_step::ProjectionProgress::Complete(result) => {
                                result.map_err(|r| schema_invalid(site, f.ordinal, r))?
                            }
                            s::projection_step::ProjectionProgress::Pending => {
                                self.push(Job::Schema(f, site))?;
                                return Ok(Some(true));
                            }
                            _ => unreachable!(),
                        }
                    }
                    s::Action::Typed => {
                        let result = match TypedAccess.kind(node) {
                            s::SchemaKind::Array | s::SchemaKind::String => {
                                let (min, max) = TypedAccess.typed_unsigned(node);
                                s::check_typed_unsigned(TypedAccess.kind(node), min, max)
                            }
                            s::SchemaKind::Number => {
                                s::check_typed_numeric(&TypedAccess.typed_float(node))
                            }
                            s::SchemaKind::Integer => {
                                s::check_typed_numeric(&TypedAccess.typed_integer(node))
                            }
                            _ => Ok(()),
                        };
                        result.map_err(|r| schema_invalid(site, f.ordinal, r))?;
                    }
                    s::Action::Child(n) => {
                        let child = match node {
                            DataSchema::Array(v) => &v.items.as_ref().unwrap()[n],
                            DataSchema::Object(v) => {
                                let iter = f
                                    .children
                                    .get_or_insert_with(|| v.properties.as_ref().unwrap().iter());
                                self.trace.next_calls += 1;
                                iter.next().unwrap().1
                            }
                            _ => unreachable!(),
                        };
                        f.walk.advance();
                        self.push(Job::Schema(f, site))?;
                        self.schema(child, site)?;
                        return Ok(Some(true));
                    }
                    s::Action::Done => return Ok(Some(true)),
                }
                f.walk.advance();
                self.push(Job::Schema(f, site))?;
            }
            Job::Operations(forms, n, m, owner) => {
                if n < forms.len() {
                    let ops = forms[n].op.as_deref().unwrap_or(&[]);
                    if m < ops.len() {
                        if !b::allowed(owner.kind, ops[m]) {
                            return Err(invalid(
                                ErrorSite {
                                    owner,
                                    field: Field::FormOperation,
                                    index: n,
                                    member: m,
                                },
                                b::Rule::Operation(ops[m]),
                            ));
                        }
                        self.push(Job::Operations(forms, n, m + 1, owner))?;
                    } else {
                        self.push(Job::Operations(forms, n + 1, 0, owner))?;
                    }
                }
            }
            Job::FormSecurity(forms, n, owner) => {
                if n < forms.len() {
                    self.push(Job::FormSecurity(forms, n + 1, owner))?;
                    if let Some(names) = forms[n].security.as_deref() {
                        let mut site = ErrorSite::new(owner, Field::FormSecurity);
                        site.index = n;
                        self.push(Job::References(Names::Strings(names), 0, site))?;
                    }
                }
            }
        }
        Ok(Some(true))
    }
    fn schema(&mut self, node: &'a DataSchema, site: ErrorSite) -> Result<(), Cause> {
        let ordinal = self.next_schema;
        self.next_schema += 1;
        self.trace.schema_nodes += 1;
        self.push(Job::Schema(Schema::new(node, ordinal), site))
    }
    fn global(&mut self, budget: &mut WorkBudget) -> Result<Option<bool>, Cause> {
        if !self.debit(
            budget,
            // One owner advance can coincide with one nested iterator creation
            // (definition extras or affordance URI variables). Whole-field
            // inspection established this maximum across *all* maps.
            &[(
                W::DocumentNodes,
                2 * iter_cost(self.trace.object_members as usize),
            )],
        )? {
            return Ok(Some(false));
        }
        let counts = [
            self.thing.properties.as_ref().map_or(0, BTreeMap::len),
            self.thing.actions.as_ref().map_or(0, BTreeMap::len),
            self.thing.events.as_ref().map_or(0, BTreeMap::len),
        ];
        let action = self
            .walk
            .action(self.thing.security_definitions.len(), counts);
        let owner = match action {
            b::Action::SchemaMap(o, _)
            | b::Action::Schema(o, _)
            | b::Action::Operations(o)
            | b::Action::FormSecurity(o) => Some(o),
            _ => None,
        };
        if let Some(owner) = owner.filter(|o| o.kind != OwnerKind::Thing) {
            if self.current.as_ref().is_none_or(|(old, _)| *old != owner) {
                self.trace.next_calls += 1;
                let aff = match owner.kind {
                    OwnerKind::Property => {
                        Aff::Property(self.properties.as_mut().unwrap().next().unwrap().1)
                    }
                    OwnerKind::Action => {
                        Aff::Action(self.actions.as_mut().unwrap().next().unwrap().1)
                    }
                    OwnerKind::Event => Aff::Event(self.events.as_mut().unwrap().next().unwrap().1),
                    _ => unreachable!(),
                };
                self.current = Some((owner, aff));
            }
        }
        match action {
            b::Action::Title => b::required(
                self.thing
                    ._metadata
                    .title
                    .as_deref()
                    .is_some_and(|v| !v.is_empty()),
            )
            .map_err(|r| invalid(ErrorSite::new(b::ROOT, Field::Title), r))?,
            b::Action::RequiredSecurity => b::required_security(self.thing.security.len())
                .map_err(|r| invalid(ErrorSite::new(b::ROOT, Field::Security), r))?,
            b::Action::RootReferences => self.push(Job::References(
                Names::Strings(&self.thing.security),
                0,
                ErrorSite::new(b::ROOT, Field::Security),
            ))?,
            b::Action::Definition(owner) => {
                self.trace.next_calls += 1;
                let definition = self.definitions.as_mut().unwrap().next().unwrap().1;
                self.push(Job::Definition(
                    Sec::new(definition),
                    ErrorSite::new(owner, Field::Scheme),
                ))?;
            }
            b::Action::SchemaMap(owner, field) => {
                let map = if owner.kind == OwnerKind::Thing {
                    match field {
                        Field::SchemaDefinitions => self.thing.schema_definitions.as_ref(),
                        Field::UriVariables => self.thing.uri_variables.as_ref(),
                        _ => unreachable!(),
                    }
                } else {
                    self.current.unwrap().1.variables()
                };
                if let Some(map) = map {
                    self.push(Job::SchemaMap(map.iter(), ErrorSite::new(owner, field), 0))?;
                }
            }
            b::Action::Schema(owner, field) => {
                if let Some(node) = self.current.unwrap().1.schema(field) {
                    self.next_schema = 0;
                    self.schema(node, ErrorSite::new(owner, field))?;
                }
            }
            b::Action::Operations(owner) => {
                let forms = if owner.kind == OwnerKind::Thing {
                    self.thing.forms.as_deref().unwrap_or(&[])
                } else {
                    self.current.unwrap().1.forms()
                };
                self.push(Job::Operations(forms, 0, 0, owner))?;
            }
            b::Action::FormSecurity(owner) => {
                let forms = if owner.kind == OwnerKind::Thing {
                    self.thing.forms.as_deref().unwrap_or(&[])
                } else {
                    self.current.unwrap().1.forms()
                };
                self.push(Job::FormSecurity(forms, 0, owner))?;
            }
            b::Action::Done => return Ok(None),
        }
        self.walk.advance();
        Ok(Some(true))
    }
}
fn map_len(map: Map<'_>) -> usize {
    match map {
        Map::Json(v) => v.len(),
        Map::Values(v) => v.len(),
        Map::Strings(v) => v.len(),
        Map::Schemas(v) => v.len(),
        Map::Properties(v) => v.len(),
        Map::Actions(v) => v.len(),
        Map::Events(v) => v.len(),
        Map::Security(v) => v.len(),
    }
}
impl Validated<'_> {
    pub fn trace(&self) -> Trace {
        let mut t = self.trace;
        let f = self.storage.footprint();
        t.allocations = self.storage.allocations();
        t.releases = self.storage.releases();
        t.live = self.storage.live_bytes();
        t.peak = f.reservation_peak_bytes;
        t.largest = f.largest_request_bytes;
        t.attempts = self.storage.attempts();
        t
    }
}

/// Charged semantic projection, consumed twice for preflight/materialization.
/// Only the current derived URI lives here. Planning retains coordinates or
/// owned copies, and takes a fresh short loan after every suspension/move.
struct Read<'a> {
    proof: Validated<'a>,
    properties: Option<btree_map::Iter<'a, String, crate::affordance::PropertyAffordance>>,
    current: Option<(&'a str, &'a crate::affordance::PropertyAffordance)>,
    property: usize,
    form: usize,
    op: usize,
    readable: bool,
    stage: u8,
    search: Option<Search<'a>>,
    scheme: Option<&'a str>,
    uri: Option<uri::Resolver<'a>>,
    uri_observed: UriObservations,
    failure: Option<Cause>,
    scope_position: usize,
    copy_bytes: u64,
    effective_bytes: u64,
    scope_bytes: u64,
    scope_visits: Cell<u64>,
}
enum SecurityFact<'a> {
    Empty,
    Multiple(usize),
    Single { name: &'a str, scheme: &'a str },
}
#[derive(Clone, Copy)]
struct TextSequence<'a>(&'a [String], u64, &'a Cell<u64>);
impl<'a> TextSequence<'a> {
    pub fn len(self) -> usize {
        self.0.len()
    }
    pub fn is_empty(self) -> bool {
        self.0.is_empty()
    }
    /// Total source bytes, established by the paid semantic projection.
    pub fn byte_len(self) -> u64 {
        self.1
    }
    pub fn iter(self) -> impl ExactSizeIterator<Item = &'a str> {
        self.0.iter().map(move |value| {
            self.2.set(self.2.get() + 1);
            value.as_str()
        })
    }
}
struct Fact<'a> {
    pub property: usize,
    pub name: &'a str,
    pub original: usize,
    pub raw: &'a str,
    pub resolved: &'a str,
    pub content_type: &'a str,
    pub content_coding: Option<&'a str>,
    pub subprotocol: Option<&'a str>,
    pub copy_bytes: u64,
    pub scopes: TextSequence<'a>,
    pub security: SecurityFact<'a>,
}
enum Event<'a> {
    Pending,
    Property { ordinal: usize, name: &'a str },
    Form(Fact<'a>),
    Done,
}
impl<'a> Read<'a> {
    pub fn new(proof: Validated<'a>) -> Self {
        Self {
            proof,
            properties: None,
            current: None,
            property: 0,
            form: 0,
            op: 0,
            readable: false,
            stage: 0,
            search: None,
            scheme: None,
            uri: None,
            uri_observed: UriObservations::default(),
            failure: None,
            scope_position: 0,
            copy_bytes: 0,
            effective_bytes: 0,
            scope_bytes: 0,
            scope_visits: Cell::new(0),
        }
    }
    pub fn trace(&self) -> Trace {
        self.proof.trace()
    }
    pub fn lifetime_remaining(&self) -> u64 {
        self.proof.lifetime
    }
    pub fn uri_validation_bytes(&self) -> u64 {
        self.uri_observed.utf8_bytes + self.uri.as_ref().map_or(0, |u| u.observations().utf8_bytes)
    }
    /// Instrument actual source iteration in both sizing and external copying.
    pub fn scope_visits(&self) -> u64 {
        self.scope_visits.get()
    }
    pub fn id(&self) -> Option<&'a str> {
        self.proof.thing.id.as_ref().map(|v| v.as_str())
    }
    pub fn ready(&self) -> bool {
        self.failure.is_none() && matches!(self.stage, 6 | 8)
    }
    fn is_done(&self) -> bool {
        self.failure.is_none() && self.stage == 7
    }
    pub fn acknowledge(&mut self) {
        if !self.ready() {
            return;
        }
        match self.stage {
            8 => self.stage = 2,
            6 => {
                self.form += 1;
                self.stage = 2;
            }
            _ => {}
        }
    }
    pub fn finish(self) -> Validated<'a> {
        assert!(self.is_done());
        self.proof
    }
    fn debit(&mut self, budget: &mut WorkBudget, costs: &[(W, u64)]) -> Result<bool, Cause> {
        let result = pay(
            budget,
            &mut self.proof.lifetime,
            &mut self.proof.trace,
            costs,
        );
        if let Err(c) = result {
            self.failure = Some(c);
        }
        result
    }
    pub fn step(&mut self, budget: &mut WorkBudget, cancel: bool) -> Result<Event<'_>, Cause> {
        if let Some(first) = self.failure {
            return Err(first);
        }
        // Completion is terminal until a successful rewind. A later cancel
        // request cannot replace Done or create a failure that rewind erases.
        if self.is_done() {
            return Ok(Event::Done);
        }
        if cancel {
            self.failure = Some(Cause::Cancelled);
            return Err(Cause::Cancelled);
        }
        if budget.is_exhausted() && !matches!(self.stage, 6 | 8) {
            return Ok(Event::Pending);
        }
        let mut actions = 0;
        loop {
            if self.is_done() {
                return Ok(Event::Done);
            }
            if self.stage == 8 {
                let (name, _) = self.current.unwrap();
                return Ok(Event::Property {
                    ordinal: self.property,
                    name,
                });
            }
            let current = self.current;
            let form = current.and_then(|(_, p)| p._interaction.forms.get(self.form));
            if self.stage == 6 {
                let (name, _) = current.unwrap();
                let form = form.unwrap();
                let (names, _) = b::inherited_security(
                    self.proof.thing.security.as_slice(),
                    form.security.as_deref(),
                );
                let security = match names.len() {
                    0 => SecurityFact::Empty,
                    1 => SecurityFact::Single {
                        name: &names[0],
                        scheme: self.scheme.unwrap(),
                    },
                    n => SecurityFact::Multiple(n),
                };
                let raw = form.href.as_str();
                let resolved = self.uri.as_ref().unwrap().result(&self.proof.storage);
                return Ok(Event::Form(Fact {
                    property: self.property,
                    name,
                    original: self.form,
                    raw,
                    resolved,
                    content_type: &form.content_type,
                    content_coding: form.content_coding.as_deref(),
                    subprotocol: form.subprotocol.as_deref(),
                    copy_bytes: self.copy_bytes,
                    scopes: TextSequence(
                        form.scopes.as_deref().unwrap_or(&[]),
                        self.scope_bytes,
                        &self.scope_visits,
                    ),
                    security,
                }));
            }
            if actions == 1 {
                return Ok(Event::Pending);
            }
            actions += 1;
            let costs = match self.stage {
                0 | 1 => [
                    (
                        W::DocumentNodes,
                        iter_cost(
                            self.proof
                                .thing
                                .properties
                                .as_ref()
                                .map_or(0, BTreeMap::len),
                        ),
                    ),
                    (W::CodecInputBytes, 0),
                ],
                3 => [
                    (
                        W::DocumentNodes,
                        iter_cost(self.proof.thing.security_definitions.len()),
                    ),
                    (W::CodecInputBytes, 0),
                ],
                4 => self.search.as_ref().unwrap().costs(self.proof.thing),
                5 => [
                    (W::DocumentNodes, u64::from(self.uri.is_none())),
                    (W::CodecInputBytes, 0),
                ],
                9 => [(W::DocumentNodes, 1), (W::CodecInputBytes, 0)],
                _ => [(W::DocumentNodes, 1), (W::CodecInputBytes, 0)],
            };
            let costs = [
                costs[0],
                costs[1],
                (W::SecurityBranches, u64::from(self.stage == 3)),
            ];
            if !self.debit(budget, &costs)? {
                return Ok(Event::Pending);
            }
            match self.stage {
                0 => {
                    self.properties = self.proof.thing.properties.as_ref().map(BTreeMap::iter);
                    self.stage = 1;
                }
                1 => {
                    self.proof.trace.next_calls += 1;
                    match self.properties.as_mut().and_then(Iterator::next) {
                        Some((name, p)) => {
                            self.current = Some((name, p));
                            self.form = 0;
                            self.stage = 8;
                            return Ok(Event::Property {
                                ordinal: self.property,
                                name,
                            });
                        }
                        None => {
                            self.stage = 7;
                            return Ok(Event::Done);
                        }
                    }
                }
                2 => {
                    if form.is_none() {
                        self.property += 1;
                        self.current = None;
                        self.stage = 1;
                        continue;
                    }
                    self.op = 0;
                    self.readable = false;
                    self.scheme = None;
                    self.scope_bytes = 0;
                    if let Some(old) = self.uri.take() {
                        let o = old.observations();
                        self.uri_observed.component_bytes += o.component_bytes;
                        self.uri_observed.merge_bytes += o.merge_bytes;
                        self.uri_observed.segment_bytes += o.segment_bytes;
                        self.uri_observed.pop_bytes += o.pop_bytes;
                        self.uri_observed.emitted_bytes += o.emitted_bytes;
                        self.uri_observed.reversed_bytes += o.reversed_bytes;
                        self.uri_observed.shifted_bytes += o.shifted_bytes;
                        self.uri_observed.utf8_bytes += o.utf8_bytes;
                    }
                    self.stage = 3;
                }
                3 => {
                    let (_, property) = current.unwrap();
                    let form = form.unwrap();
                    let ops = form.op.as_deref().unwrap_or_else(|| {
                        b::default_property_operations((
                            property._schema.context().read_only,
                            property._schema.context().write_only,
                        ))
                    });
                    if self.op < ops.len() {
                        self.readable |= ops[self.op] == Operation::ReadProperty;
                        self.op += 1;
                        continue;
                    }
                    if !self.readable {
                        self.form += 1;
                        self.stage = 2;
                        continue;
                    }
                    let (names, _) = b::inherited_security(
                        self.proof.thing.security.as_slice(),
                        form.security.as_deref(),
                    );
                    if let Err(c) = inspect::limit(
                        self.proof.policy,
                        R::SecurityBranchesPerPlanMax,
                        names.len() as u64,
                    ) {
                        self.failure = Some(c);
                        return Err(c);
                    }
                    if !names.is_empty() {
                        if let Err(c) =
                            inspect::limit(self.proof.policy, R::SecurityExpressionDepthMax, 1)
                        {
                            self.failure = Some(c);
                            return Err(c);
                        }
                    }
                    if names.len() == 1 {
                        self.search = Some(Search::new(self.proof.thing, &names[0]));
                        self.stage = 4;
                    } else {
                        self.stage = 5;
                    }
                }
                4 => {
                    let search = self.search.as_mut().unwrap();
                    search.tick(&mut self.proof.trace);
                    match search.done {
                        Some(Some(definition)) => {
                            self.scheme = Some(definition.scheme());
                            self.search = None;
                            self.stage = 5;
                        }
                        Some(None) => {
                            self.failure = Some(Cause::Security);
                            return Err(Cause::Security);
                        }
                        None => {}
                    }
                }
                5 => {
                    if self.uri.is_none() {
                        self.uri = Some(uri::Resolver::new(
                            self.proof.thing.base.as_ref(),
                            &form.unwrap().href,
                            self.proof.policy.get(R::UriTemplateSourceBytesMax) as usize,
                        ));
                    }
                    match self.uri.as_mut().unwrap().step(
                        &mut self.proof.storage,
                        budget,
                        &mut self.proof.lifetime,
                        &mut self.proof.trace,
                    ) {
                        Ok(false) => return Ok(Event::Pending),
                        Ok(true) => {}
                        Err(c) => {
                            self.failure = Some(c);
                            return Err(c);
                        }
                    }
                    self.scope_position = 0;
                    self.stage = 9;
                }
                9 => {
                    let scopes = form.unwrap().scopes.as_deref().unwrap_or(&[]);
                    if self.scope_position < scopes.len() {
                        self.scope_visits.set(self.scope_visits.get() + 1);
                        self.scope_bytes = self
                            .scope_bytes
                            .checked_add(scopes[self.scope_position].len() as u64)
                            .ok_or(Cause::Arithmetic)?;
                        self.scope_position += 1;
                    } else {
                        let (name, _) = self.current.unwrap();
                        let f = form.unwrap();
                        let resolved = self.uri.as_ref().unwrap().result(&self.proof.storage);
                        self.copy_bytes = [
                            self.id().map_or(0, str::len),
                            name.len(),
                            f.href.as_str().len(),
                            resolved.len(),
                            f.content_type.len(),
                            f.content_coding.as_deref().map_or(0, str::len),
                            f.subprotocol.as_deref().map_or(0, str::len),
                        ]
                        .into_iter()
                        .try_fold(self.scope_bytes, |sum, n| sum.checked_add(n as u64))
                        .ok_or(Cause::Arithmetic)?;
                        if !self.uri.as_ref().unwrap().is_alias() {
                            self.effective_bytes = self
                                .effective_bytes
                                .checked_add(resolved.len() as u64)
                                .ok_or(Cause::Arithmetic)?;
                        }
                        let total = (self.proof.trace.content as u64)
                            .checked_add(self.effective_bytes)
                            .ok_or(Cause::Arithmetic)?;
                        if let Err(c) = inspect::limit(
                            self.proof.policy,
                            R::GeneratedEffectiveDocumentBytesMax,
                            total,
                        ) {
                            self.failure = Some(c);
                            return Err(c);
                        }
                        self.stage = 6;
                    }
                }
                _ => unreachable!(),
            }
        }
    }
}
