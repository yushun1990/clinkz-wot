//! Paid whole-Thing Basic over the actually constructed canonical owner.
//! The shared rule/discovery kernels own policy; this cursor owns suspension,
//! byte inspection, projection, the existing frame site and terminal cleanup.
use super::{
    basic_kernel::{self as basic, Action, Field, Owner, OwnerKind, Site},
    schema_build::{Access, View},
    schema_kernel::{
        self as schema, NumericCursor, SchemaAccess,
        projection_step::{self, ProjectionProgress},
    },
    thing_build::{self as typed, NormalizedThing},
};
use clinkz_wot_foundation::{WorkBudget, WorkClass as W};
use core::mem;
use validated_thing_value_construction_probe::{
    FrameError, FrameResources, Frames, Kind, Limits, OwnedInspection, OwnedValue, Sealed,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Walk,
    Lookup,
    Text,
    Schema,
    Frames,
    Numeric,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Trace {
    /// DocumentNodes, CodecInputBytes, JsonSchemaNodes, SecurityBranches, cleanup.
    pub work: [u64; 5],
    pub key_bytes: u64,
    pub schemas: u64,
    pub projections: [u64; 2],
    pub frame_requests: u64,
    pub frame_copies: u64,
    pub maximum_depth: usize,
    pub resources: Option<FrameResources>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cause {
    Invalid(basic::InlineInvalid),
    Lifetime,
    Memory,
    Arithmetic,
    Allocation,
    Cancelled,
    Depth { observed: usize, ceiling: usize },
    Number { observed: usize, ceiling: usize },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Failure {
    pub cause: Cause,
    pub phase: Phase,
    pub trace: Trace,
}

/// Private relocation handles. They never retain an arena pointer or borrow.
#[derive(Clone, Copy)]
struct NodeId(usize);
impl NodeId {
    fn bind(self, arenas: Sealed<'_>) -> View<'_> {
        View::at(arenas, self.0)
    }
    fn child(self, arenas: Sealed<'_>, index: usize) -> Option<Self> {
        self.bind(arenas).child(index).map(|v| Self(v.node_id()))
    }
    fn member(self, arenas: Sealed<'_>, index: usize) -> Option<(Text, Self)> {
        if self.literal_kind(arenas) != Some(Kind::Object) {
            return None;
        }
        let entry = self.child(arenas, index)?;
        Some((
            entry.child(arenas, 0)?.text(arenas)?,
            entry.child(arenas, 1)?,
        ))
    }
    fn kind_for_fixture(self, arenas: Sealed<'_>) -> u32 {
        self.bind(arenas).kind_for_fixture()
    }
    fn literal_kind(self, arenas: Sealed<'_>) -> Option<Kind> {
        self.bind(arenas).literal_kind()
    }
    fn schema_kind(self, arenas: Sealed<'_>) -> Option<schema::SchemaKind> {
        self.bind(arenas).schema_kind()
    }
    fn extras(self, arenas: Sealed<'_>) -> Self {
        Self(self.bind(arenas).extras().node_id())
    }
    fn len(self, arenas: Sealed<'_>) -> usize {
        self.bind(arenas).len()
    }
    fn text(self, arenas: Sealed<'_>) -> Option<Text> {
        self.bind(arenas)
            .text_range()
            .map(|(start, len)| Text::Arena { start, len })
    }
}
#[derive(Clone, Copy)]
enum Text {
    Static(&'static str),
    Arena { start: usize, len: usize },
}
impl Text {
    fn len(self) -> usize {
        match self {
            Self::Static(s) => s.len(),
            Self::Arena { len, .. } => len,
        }
    }
    fn is_empty(self) -> bool {
        self.len() == 0
    }
    fn borrow(self, arenas: Sealed<'_>) -> &str {
        match self {
            Self::Static(s) => s,
            Self::Arena { start, len } => {
                // SAFETY: private ranges come only from checked literal/typed text
                // in an immutable canonical owner. Lending does not grow or mutate
                // source arrays; constructing a slice must not rescan its content.
                unsafe { core::str::from_utf8_unchecked(&arenas.bytes()[start..start + len]) }
            }
        }
    }
}

struct Frame {
    node: NodeId,
    ordinal: u64,
    walk: schema::Walk,
}
#[derive(Clone, Copy)]
struct Forms {
    forms: NodeId,
    index: usize,
    member: usize,
    owner: Owner,
    security: bool,
}
#[derive(Clone, Copy)]
struct References {
    names: Option<NodeId>,
    position: usize,
    member: usize,
    site: Site,
    next: Resume,
}
#[derive(Clone, Copy)]
struct Combo {
    definition: NodeId,
    owner: Owner,
    names: [Option<NodeId>; 2],
    counts: [usize; 2],
    empty: [Option<usize>; 2],
    group: usize,
    position: usize,
}
#[derive(Clone, Copy)]
enum Resume {
    Done,
    Global,
    Forms(Forms),
    Schema,
    SchemaMap {
        map: NodeId,
        index: usize,
        site: Site,
    },
    ComboAll {
        names: Option<NodeId>,
        owner: Owner,
    },
}
#[derive(Clone, Copy)]
enum Probe {
    Scheme { definition: usize, owner: Owner },
    Flow { definition: usize, owner: Owner },
    SchemaType,
}
#[derive(Clone, Copy)]
enum Purpose {
    Reference {
        name: Text,
        references: References,
    },
    Combo(Combo),
    Name(Owner),
    Flow {
        owner: Owner,
    },
    Endpoint {
        definition: NodeId,
        owner: Owner,
        field: Field,
    },
    Unsigned {
        position: usize,
        values: [Option<u64>; 4],
    },
    Numeric {
        position: usize,
        numbers: [Option<Text>; 5],
    },
}
struct Lookup {
    map: NodeId,
    target: Text,
    input_target: bool,
    index: usize,
    key: Option<Text>,
    byte: usize,
    first: Option<u8>,
    purpose: Purpose,
}
enum State {
    Resume(Resume),
    References(References),
    Lookup(Lookup),
    Probe {
        text: Text,
        position: usize,
        bytes: [u8; 8],
        probe: Probe,
    },
    Combo(Combo),
    Enter(NodeId),
    Push(Frame),
    Unsigned {
        position: usize,
        values: [Option<u64>; 4],
    },
    UnsignedNumber {
        position: usize,
        values: [Option<u64>; 4],
        text: Text,
    },
    NumericGather {
        position: usize,
        numbers: [Option<Text>; 5],
    },
    Numeric {
        cursor: NumericCursor,
        numbers: [Option<Text>; 5],
    },
    Moving,
}
pub struct Cursor<'a> {
    arenas: Sealed<'a>,
    root: NodeId,
    lifetime: &'a mut u64,
    frames: Frames<'a, Frame>,
    limits: Limits,
    walk: basic::Walk,
    state: State,
    schema_site: Site,
    after_schema: Resume,
    ordinal: u64,
    trace: Trace,
}

struct Continuation {
    walk: basic::Walk,
    state: State,
    schema_site: Site,
    after_schema: Resume,
    ordinal: u64,
    trace: Trace,
}
impl Continuation {
    fn new(schema: bool) -> Self {
        Self {
            walk: basic::Walk::default(),
            state: if schema {
                State::Enter(NodeId(0))
            } else {
                State::Resume(Resume::Global)
            },
            schema_site: Site::new(basic::ROOT, Field::Title),
            after_schema: if schema { Resume::Done } else { Resume::Global },
            ordinal: 0,
            trace: Trace::default(),
        }
    }
}

/// Non-production owning post-construction pass shared by both input paths.
/// No stored state or frame contains a borrow of the source/accounting owner.
pub struct OwningCursor {
    owner: OwnedInspection<Frame>,
    continuation: Continuation,
    limits: Limits,
    schema: bool,
}
pub enum Output {
    Thing(NormalizedThing),
    Schema(super::schema_build::Schema),
}
#[allow(clippy::large_enum_variant)]
pub enum OwningProgress {
    Pending(OwningCursor),
    Complete { output: Output, trace: Trace },
    Failed(OwningFailure),
}
#[derive(Clone, Copy, Debug)]
pub struct OwningFailure {
    pub failure: Failure,
    pub resources: validated_thing_value_construction_probe::Footprint,
    pub lifetime_remaining: u64,
    pub live_after_rollback: u64,
    pub allocations: u64,
    pub releases: u64,
}
impl OwningCursor {
    pub fn new(owner: NormalizedThing, limits: Limits) -> Self {
        Self::from_owner(owner.owner, limits, false)
    }
    pub fn from_schema(owner: super::schema_build::Schema, limits: Limits) -> Self {
        Self::from_owner(owner.into_owner(), limits, true)
    }
    pub(crate) fn from_owner(owner: OwnedValue, limits: Limits, schema: bool) -> Self {
        Self {
            owner: owner.into_inspection(),
            continuation: Continuation::new(schema),
            limits,
            schema,
        }
    }
    pub fn trace(&self) -> Trace {
        Trace {
            resources: Some(self.owner.resources()),
            ..self.continuation.trace
        }
    }
    pub fn phase(&self) -> Phase {
        state_phase(&self.continuation.state)
    }
    pub fn lifetime_remaining(&self) -> u64 {
        self.owner.lifetime_remaining()
    }
    pub fn frame_transfer(&mut self) -> bool {
        self.owner.parts().2.transferring()
    }
    pub fn step(
        mut self,
        budget: &mut WorkBudget,
        cancelled: impl FnMut() -> bool,
    ) -> OwningProgress {
        // Boxing suspension state would introduce another allocation site.
        #[allow(clippy::large_enum_variant)]
        enum Step {
            Pending(Continuation),
            Complete(Trace),
            Failed(Failure),
        }
        let result = {
            let (arenas, lifetime, frames) = self.owner.parts();
            let runner = Cursor::resume(arenas, lifetime, frames, self.limits, self.continuation);
            match runner.step(budget, cancelled) {
                Progress::Pending(cursor) => Step::Pending(cursor.suspend()),
                Progress::Complete(trace) => Step::Complete(trace),
                Progress::Failed(failure) => Step::Failed(failure),
            }
        };
        match result {
            Step::Pending(continuation) => {
                self.continuation = continuation;
                OwningProgress::Pending(self)
            }
            Step::Complete(trace) => {
                let owner = self.owner.into_value();
                let output = if self.schema {
                    Output::Schema(super::schema_build::Schema::from_owner(owner))
                } else {
                    Output::Thing(NormalizedThing::new(owner))
                };
                OwningProgress::Complete { output, trace }
            }
            Step::Failed(failure) => {
                let mut owner = self.owner.into_value();
                owner.rollback_for_fixture();
                OwningProgress::Failed(OwningFailure {
                    failure,
                    resources: owner.footprint(),
                    lifetime_remaining: owner.lifetime_remaining(),
                    live_after_rollback: owner.footprint().retained_requested_bytes,
                    allocations: owner.allocations(),
                    releases: owner.releases(),
                })
            }
        }
    }
}
// A boxed Pending cursor would invent another allocation site.
#[allow(clippy::large_enum_variant)]
pub enum Progress<'a> {
    Pending(Cursor<'a>),
    Complete(Trace),
    Failed(Failure),
}
enum Tick {
    Advanced,
    Blocked,
    Complete,
}
impl<'a> Cursor<'a> {
    pub fn new(owner: &'a mut NormalizedThing, limits: Limits) -> Self {
        Self::from_owner(&mut owner.owner, limits)
    }
    pub(crate) fn from_owner(owner: &'a mut OwnedValue, limits: Limits) -> Self {
        let (arenas, lifetime, frames) = owner.canonical_inspection_parts();
        Self::resume(arenas, lifetime, frames, limits, Continuation::new(false))
    }
    fn resume(
        arenas: Sealed<'a>,
        lifetime: &'a mut u64,
        frames: Frames<'a, Frame>,
        limits: Limits,
        continuation: Continuation,
    ) -> Self {
        Self {
            arenas,
            root: NodeId(0),
            lifetime,
            frames,
            limits,
            walk: continuation.walk,
            state: continuation.state,
            schema_site: continuation.schema_site,
            after_schema: continuation.after_schema,
            ordinal: continuation.ordinal,
            trace: continuation.trace,
        }
    }
    fn suspend(self) -> Continuation {
        Continuation {
            walk: self.walk,
            state: self.state,
            schema_site: self.schema_site,
            after_schema: self.after_schema,
            ordinal: self.ordinal,
            trace: self.trace,
        }
    }
    pub fn lifetime_remaining(&self) -> u64 {
        *self.lifetime
    }
    pub fn trace(&self) -> Trace {
        Trace {
            resources: Some(self.frames.resources()),
            ..self.trace
        }
    }
    pub fn phase(&self) -> Phase {
        state_phase(&self.state)
    }
    pub fn step(
        mut self,
        budget: &mut WorkBudget,
        mut cancelled: impl FnMut() -> bool,
    ) -> Progress<'a> {
        loop {
            let phase = self.phase();
            if cancelled() {
                return self.fail(Cause::Cancelled, phase);
            }
            match self.tick(budget, &mut cancelled) {
                Ok(Tick::Advanced) => {}
                Ok(Tick::Blocked) => return Progress::Pending(self),
                Ok(Tick::Complete) => {
                    self.frames.clear();
                    return Progress::Complete(self.trace());
                }
                Err(cause) => return self.fail(cause, phase),
            }
        }
    }
    fn fail(mut self, cause: Cause, phase: Phase) -> Progress<'a> {
        self.frames.clear();
        Progress::Failed(Failure {
            cause,
            phase,
            trace: self.trace(),
        })
    }
    fn pay(&mut self, budget: &mut WorkBudget, classes: &[W]) -> Result<bool, Cause> {
        if classes.iter().any(|&class| {
            budget.remaining(class) < classes.iter().filter(|&&c| c == class).count() as u64
        }) {
            return Ok(false);
        }
        if *self.lifetime < classes.len() as u64 {
            return Err(Cause::Lifetime);
        }
        for &class in classes {
            budget.consume(class, 1).unwrap();
            self.trace.work[match class {
                W::DocumentNodes => 0,
                W::CodecInputBytes => 1,
                W::JsonSchemaNodes => 2,
                W::SecurityBranches => 3,
                W::CleanupItems => 4,
                _ => unreachable!(),
            }] += 1;
        }
        *self.lifetime -= classes.len() as u64;
        Ok(true)
    }
    fn invalid(site: Site, rule: basic::Rule<'_>) -> Cause {
        Cause::Invalid(basic::InlineInvalid {
            site,
            rule: basic::InlineRule::Basic(rule.kind()),
            schema_ordinal: None,
        })
    }
    fn schema_rule(&self, result: Result<(), schema::Rule>) -> Result<(), Cause> {
        result.map_err(|rule| {
            Cause::Invalid(basic::InlineInvalid {
                site: self.schema_site,
                rule: basic::InlineRule::Schema(rule),
                schema_ordinal: Some(self.frames.last().ordinal),
            })
        })
    }
    fn definitions(&self) -> NodeId {
        self.root.child(self.arenas, 14).unwrap()
    }
    fn affordance(&self, owner: Owner) -> NodeId {
        self.root
            .child(
                self.arenas,
                match owner.kind {
                    OwnerKind::Property => 8,
                    OwnerKind::Action => 9,
                    OwnerKind::Event => 10,
                    _ => unreachable!(),
                },
            )
            .unwrap()
            .member(self.arenas, owner.ordinal)
            .unwrap()
            .1
    }
    fn forms(&self, owner: Owner) -> Option<NodeId> {
        if owner.kind == OwnerKind::Thing {
            self.root.child(self.arenas, 12)
        } else {
            self.affordance(owner).child(self.arenas, 1)
        }
    }
    fn lookup(&mut self, map: NodeId, target: Text, input_target: bool, purpose: Purpose) {
        self.state = State::Lookup(Lookup {
            map,
            target,
            input_target,
            purpose,
            index: 0,
            key: None,
            byte: 0,
            first: None,
        });
    }
    fn security_field(
        &mut self,
        definition: NodeId,
        field: Field,
        purpose: Purpose,
    ) -> Result<(), Cause> {
        let variant = definition.child(self.arenas, 1).unwrap();
        let slot = match (variant.kind_for_fixture(self.arenas), field) {
            (k, Field::Name) if k == typed::SECURITY_VARIANT + 5 => Some(0),
            (k, Field::Flow) if k == typed::SECURITY_VARIANT + 8 => Some(4),
            (k, Field::Authorization) if k == typed::SECURITY_VARIANT + 8 => Some(0),
            (k, Field::Token) if k == typed::SECURITY_VARIANT + 8 => Some(1),
            (k, Field::OneOf) if k == typed::SECURITY_VARIANT + 2 => Some(0),
            (k, Field::AllOf) if k == typed::SECURITY_VARIANT + 2 => Some(1),
            _ => None,
        };
        if let Some(slot) = slot {
            self.found(variant.child(self.arenas, slot), purpose)
        } else {
            self.lookup(
                definition
                    .child(self.arenas, 0)
                    .unwrap()
                    .child(self.arenas, 5)
                    .unwrap(),
                Text::Static(field.name()),
                false,
                purpose,
            );
            Ok(())
        }
    }
    fn probe(&mut self, text: Text, probe: Probe) {
        self.state = State::Probe {
            text,
            probe,
            position: 0,
            bytes: [0; 8],
        };
    }
    fn probe_result(&mut self, text: &str, probe: Probe) -> Result<(), Cause> {
        match probe {
            Probe::Scheme { definition, owner } => {
                let definition = self
                    .definitions()
                    .member(self.arenas, definition)
                    .unwrap()
                    .1;
                match basic::scheme_kind(text)
                    .map_err(|r| Self::invalid(Site::new(owner, Field::Scheme), r))?
                {
                    basic::Scheme::Other => self.state = State::Resume(Resume::Global),
                    basic::Scheme::ApiKey => {
                        self.security_field(definition, Field::Name, Purpose::Name(owner))?
                    }
                    basic::Scheme::OAuth => {
                        self.security_field(definition, Field::Flow, Purpose::Flow { owner })?
                    }
                    basic::Scheme::Combo => self.security_field(
                        definition,
                        Field::OneOf,
                        Purpose::Combo(Combo {
                            definition,
                            owner,
                            names: [None; 2],
                            counts: [0; 2],
                            empty: [None; 2],
                            group: 0,
                            position: 0,
                        }),
                    )?,
                }
            }
            Probe::Flow { definition, owner } => {
                let definition = self
                    .definitions()
                    .member(self.arenas, definition)
                    .unwrap()
                    .1;
                if basic::flow_requires_endpoints(text)
                    .map_err(|r| Self::invalid(Site::new(owner, Field::Flow), r))?
                {
                    self.security_field(
                        definition,
                        Field::Authorization,
                        Purpose::Endpoint {
                            definition,
                            owner,
                            field: Field::Authorization,
                        },
                    )?;
                } else {
                    self.state = State::Resume(Resume::Global);
                }
            }
            Probe::SchemaType => {
                self.schema_rule(schema::check_type(
                    Some(text),
                    self.frames.last().node.schema_kind(self.arenas).unwrap(),
                ))?;
                self.advance_schema();
            }
        }
        Ok(())
    }
    fn found(&mut self, value: Option<NodeId>, purpose: Purpose) -> Result<(), Cause> {
        let string = value
            .filter(|v| v.literal_kind(self.arenas) == Some(Kind::String))
            .and_then(|v| v.text(self.arenas));
        let number = value
            .filter(|v| v.literal_kind(self.arenas) == Some(Kind::Number))
            .and_then(|v| v.text(self.arenas));
        match purpose {
            Purpose::Reference { name, references } => {
                if value.is_none() {
                    let mut site = references.site;
                    site.member = references.member - 1;
                    return Err(Self::invalid(
                        site,
                        basic::Rule::Undefined(name.borrow(self.arenas)),
                    ));
                }
                self.state = State::References(references);
            }
            Purpose::Combo(mut combo) => {
                combo.names[combo.group] = value;
                self.state = State::Combo(combo);
            }
            Purpose::Name(owner) => {
                basic::required(!string.unwrap_or(Text::Static("")).is_empty())
                    .map_err(|r| Self::invalid(Site::new(owner, Field::Name), r))?;
                self.state = State::Resume(Resume::Global);
            }
            Purpose::Flow { owner } => {
                self.probe(
                    string.unwrap_or(Text::Static("")),
                    Probe::Flow {
                        definition: owner.ordinal,
                        owner,
                    },
                );
            }
            Purpose::Endpoint {
                definition,
                owner,
                field,
            } => {
                let typed = definition
                    .child(self.arenas, 1)
                    .unwrap()
                    .kind_for_fixture(self.arenas)
                    == typed::SECURITY_VARIANT + 8;
                let present = if typed {
                    value.is_some()
                } else {
                    string.is_some_and(|v| !v.is_empty())
                };
                basic::required(present).map_err(|r| Self::invalid(Site::new(owner, field), r))?;
                if field == Field::Authorization {
                    self.security_field(
                        definition,
                        Field::Token,
                        Purpose::Endpoint {
                            definition,
                            owner,
                            field: Field::Token,
                        },
                    )?;
                } else {
                    self.state = State::Resume(Resume::Global);
                }
            }
            Purpose::Unsigned { position, values } => {
                self.state = if let Some(text) = number {
                    State::UnsignedNumber {
                        position,
                        values,
                        text,
                    }
                } else {
                    State::Unsigned {
                        position: position + 1,
                        values,
                    }
                };
            }
            Purpose::Numeric {
                position,
                mut numbers,
            } => {
                numbers[position] = number;
                self.state = State::NumericGather {
                    position: position + 1,
                    numbers,
                };
            }
        }
        Ok(())
    }
    fn start_schema(&mut self, node: NodeId, site: Site, next: Resume) {
        assert!(self.frames.is_empty());
        self.schema_site = site;
        self.after_schema = next;
        self.ordinal = 0;
        self.state = State::Enter(node);
    }
    fn advance_schema(&mut self) {
        self.frames.last_mut().walk.advance();
        self.state = State::Resume(Resume::Schema);
    }
    fn number_limit(&self, text: &str) -> Result<(), Cause> {
        if text.len() > self.limits.number {
            Err(Cause::Number {
                observed: text.len(),
                ceiling: self.limits.number,
            })
        } else {
            Ok(())
        }
    }
    fn tick(
        &mut self,
        budget: &mut WorkBudget,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<Tick, Cause> {
        let state = mem::replace(&mut self.state, State::Moving);
        // Byte/projection/allocation states check their complete own debit.
        let classes: &[W] = match state {
            State::Lookup(_)
            | State::Probe { .. }
            | State::Push(_)
            | State::UnsignedNumber { .. }
            | State::Numeric { .. } => &[],
            State::Enter(_) => &[W::DocumentNodes, W::JsonSchemaNodes],
            State::References(_) | State::Combo(_) => &[W::DocumentNodes, W::SecurityBranches],
            _ => &[W::DocumentNodes],
        };
        if !self.pay(budget, classes)? {
            self.state = state;
            return Ok(Tick::Blocked);
        }
        match state {
            State::Resume(Resume::Done) => return Ok(Tick::Complete),
            State::Resume(Resume::Global) => {
                let action = self.walk.action(
                    self.definitions().len(self.arenas),
                    [8, 9, 10].map(|slot| {
                        self.root
                            .child(self.arenas, slot)
                            .map_or(0, |v| v.len(self.arenas))
                    }),
                );
                self.walk.advance();
                self.state = State::Resume(Resume::Global);
                match action {
                    Action::Title => {
                        basic::required(
                            !self
                                .root
                                .child(self.arenas, 2)
                                .unwrap()
                                .child(self.arenas, 1)
                                .and_then(|v| v.text(self.arenas))
                                .unwrap_or(Text::Static(""))
                                .is_empty(),
                        )
                        .map_err(|r| Self::invalid(Site::new(basic::ROOT, Field::Title), r))?;
                    }
                    Action::RequiredSecurity => basic::required_security(
                        self.root.child(self.arenas, 13).unwrap().len(self.arenas),
                    )
                    .map_err(|r| Self::invalid(Site::new(basic::ROOT, Field::Security), r))?,
                    Action::RootReferences => {
                        self.state = State::References(References {
                            names: self.root.child(self.arenas, 13),
                            position: 0,
                            member: 0,
                            site: Site::new(basic::ROOT, Field::Security),
                            next: Resume::Global,
                        })
                    }
                    Action::Definition(owner) => {
                        let definition = self
                            .definitions()
                            .member(self.arenas, owner.ordinal)
                            .unwrap()
                            .1;
                        self.probe(
                            definition
                                .child(self.arenas, 0)
                                .unwrap()
                                .child(self.arenas, 4)
                                .unwrap()
                                .text(self.arenas)
                                .unwrap(),
                            Probe::Scheme {
                                definition: owner.ordinal,
                                owner,
                            },
                        );
                    }
                    Action::SchemaMap(owner, field) => {
                        let map = if owner.kind == OwnerKind::Thing {
                            self.root.child(
                                self.arenas,
                                if field == Field::SchemaDefinitions {
                                    16
                                } else {
                                    17
                                },
                            )
                        } else {
                            self.affordance(owner).child(self.arenas, 2)
                        };
                        if let Some(map) = map {
                            self.state = State::Resume(Resume::SchemaMap {
                                map,
                                index: 0,
                                site: Site::new(owner, field),
                            });
                        }
                    }
                    Action::Schema(owner, field) => {
                        let node = self.affordance(owner).child(
                            self.arenas,
                            match field {
                                Field::PropertySchema => 0,
                                Field::Input | Field::Subscription => 3,
                                Field::Output | Field::Data => 4,
                                Field::DataResponse => 5,
                                Field::Cancellation => 6,
                                _ => unreachable!(),
                            },
                        );
                        if let Some(node) = node {
                            self.start_schema(node, Site::new(owner, field), Resume::Global);
                        }
                    }
                    Action::Operations(owner) | Action::FormSecurity(owner) => {
                        if let Some(forms) = self.forms(owner) {
                            self.state = State::Resume(Resume::Forms(Forms {
                                forms,
                                index: 0,
                                member: 0,
                                owner,
                                security: matches!(action, Action::FormSecurity(_)),
                            }));
                        }
                    }
                    Action::Done => return Ok(Tick::Complete),
                }
            }
            State::Resume(Resume::Forms(mut forms)) => {
                if forms.index == forms.forms.len(self.arenas) {
                    self.state = State::Resume(Resume::Global);
                } else {
                    let form = forms.forms.child(self.arenas, forms.index).unwrap();
                    if forms.security {
                        let mut site = Site::new(forms.owner, Field::FormSecurity);
                        site.index = forms.index;
                        forms.index += 1;
                        self.state = State::References(References {
                            names: form.child(self.arenas, 3),
                            position: 0,
                            member: 0,
                            site,
                            next: Resume::Forms(forms),
                        });
                    } else if let Some(ops) = form
                        .child(self.arenas, 8)
                        .filter(|v| forms.member < v.len(self.arenas))
                    {
                        let op = typed::OPERATIONS[ops
                            .child(self.arenas, forms.member)
                            .unwrap()
                            .bind(self.arenas)
                            .unsigned()
                            .unwrap() as usize];
                        if !basic::allowed(forms.owner.kind, op) {
                            return Err(Self::invalid(
                                Site {
                                    owner: forms.owner,
                                    field: Field::FormOperation,
                                    index: forms.index,
                                    member: forms.member,
                                },
                                basic::Rule::Operation(op),
                            ));
                        }
                        forms.member += 1;
                        self.state = State::Resume(Resume::Forms(forms));
                    } else {
                        forms.index += 1;
                        forms.member = 0;
                        self.state = State::Resume(Resume::Forms(forms));
                    }
                }
            }
            State::Resume(Resume::SchemaMap {
                map,
                index,
                mut site,
            }) => {
                if index == map.len(self.arenas) {
                    self.state = State::Resume(Resume::Global);
                } else {
                    site.index = index;
                    self.start_schema(
                        map.member(self.arenas, index).unwrap().1,
                        site,
                        Resume::SchemaMap {
                            map,
                            index: index + 1,
                            site,
                        },
                    );
                }
            }
            State::Resume(Resume::ComboAll { names, owner }) => {
                self.state = State::References(References {
                    names,
                    position: 0,
                    member: 0,
                    site: Site::new(owner, Field::AllOf),
                    next: Resume::Global,
                })
            }
            State::References(mut refs) => {
                if refs.position == names_len(self.arenas, refs.names) {
                    self.state = State::Resume(refs.next);
                } else {
                    let name = name_at(self.arenas, refs.names.unwrap(), refs.position);
                    refs.position += 1;
                    if let Some(name) = name {
                        refs.member += 1;
                        self.lookup(
                            self.definitions(),
                            name,
                            true,
                            Purpose::Reference {
                                name,
                                references: refs,
                            },
                        );
                    } else {
                        self.state = State::References(refs);
                    }
                }
            }
            State::Lookup(mut lookup) => {
                let mut found = None;
                let mut complete = false;
                if let Some(key) = lookup.key {
                    if lookup.byte == key.len() {
                        if !self.pay(budget, &[W::DocumentNodes])? {
                            self.state = State::Lookup(lookup);
                            return Ok(Tick::Blocked);
                        }
                        complete = true;
                        found = Some(lookup.map.member(self.arenas, lookup.index).unwrap().1);
                    } else {
                        if !self.pay(budget, &[W::DocumentNodes, W::CodecInputBytes])? {
                            self.state = State::Lookup(lookup);
                            return Ok(Tick::Blocked);
                        }
                        self.trace.key_bytes += 1;
                        if let Some(left) = lookup.first.take() {
                            let right = lookup.target.borrow(self.arenas).as_bytes()[lookup.byte];
                            if left == right {
                                lookup.byte += 1;
                            } else {
                                lookup.key = None;
                                lookup.index += 1;
                            }
                        } else {
                            let left = key.borrow(self.arenas).as_bytes()[lookup.byte];
                            if lookup.input_target {
                                lookup.first = Some(left);
                            } else if left
                                == lookup.target.borrow(self.arenas).as_bytes()[lookup.byte]
                            {
                                lookup.byte += 1;
                            } else {
                                lookup.key = None;
                                lookup.index += 1;
                            }
                        }
                    }
                } else {
                    if !self.pay(budget, &[W::DocumentNodes])? {
                        self.state = State::Lookup(lookup);
                        return Ok(Tick::Blocked);
                    }
                    if lookup.index == lookup.map.len(self.arenas) {
                        complete = true;
                    } else {
                        let key = lookup.map.member(self.arenas, lookup.index).unwrap().0;
                        if key.len() == lookup.target.len() {
                            lookup.key = Some(key);
                            lookup.byte = 0;
                        } else {
                            lookup.index += 1;
                        }
                    }
                }
                if complete {
                    self.found(found, lookup.purpose)?;
                } else {
                    self.state = State::Lookup(lookup);
                }
            }
            State::Probe {
                text,
                mut position,
                mut bytes,
                probe,
            } => {
                if text.len()
                    > match probe {
                        Probe::SchemaType => self
                            .frames
                            .last()
                            .node
                            .schema_kind(self.arenas)
                            .unwrap()
                            .name()
                            .len(),
                        _ => 6,
                    }
                {
                    if !self.pay(budget, &[W::DocumentNodes])? {
                        self.state = State::Probe {
                            text,
                            position,
                            bytes,
                            probe,
                        };
                        return Ok(Tick::Blocked);
                    }
                    // Length alone excludes every supported discriminator.
                    // Never pass the unread input to a string predicate.
                    self.probe_result("unsupported", probe)?;
                } else if position == text.len() {
                    if !self.pay(budget, &[W::DocumentNodes])? {
                        self.state = State::Probe {
                            text,
                            position,
                            bytes,
                            probe,
                        };
                        return Ok(Tick::Blocked);
                    }
                    self.probe_result(core::str::from_utf8(&bytes[..position]).unwrap(), probe)?;
                } else {
                    if !self.pay(budget, &[W::CodecInputBytes])? {
                        self.state = State::Probe {
                            text,
                            position,
                            bytes,
                            probe,
                        };
                        return Ok(Tick::Blocked);
                    }
                    bytes[position] = text.borrow(self.arenas).as_bytes()[position];
                    position += 1;
                    self.state = State::Probe {
                        text,
                        position,
                        bytes,
                        probe,
                    };
                }
            }
            State::Combo(mut combo) => {
                if combo.position < names_len(self.arenas, combo.names[combo.group]) {
                    if let Some(name) = name_at(
                        self.arenas,
                        combo.names[combo.group].unwrap(),
                        combo.position,
                    ) {
                        if name.is_empty() {
                            combo.empty[combo.group].get_or_insert(combo.counts[combo.group]);
                        }
                        combo.counts[combo.group] += 1;
                    }
                    combo.position += 1;
                    self.state = State::Combo(combo);
                } else if combo.group == 0 {
                    // A known oneOf failure must end this definition before
                    // allOf can consume credit or observe cancellation.
                    basic::combo_group(combo.counts[0], combo.empty[0]).map_err(
                        |(member, r)| {
                            let mut site = Site::new(combo.owner, Field::OneOf);
                            site.member = member;
                            Self::invalid(site, r)
                        },
                    )?;
                    combo.group = 1;
                    combo.position = 0;
                    self.security_field(combo.definition, Field::AllOf, Purpose::Combo(combo))?;
                } else {
                    basic::combo_present(combo.counts[0], combo.counts[1])
                        .map_err(|r| Self::invalid(Site::new(combo.owner, Field::Scheme), r))?;
                    basic::combo_group(combo.counts[1], combo.empty[1]).map_err(
                        |(member, r)| {
                            let mut site = Site::new(combo.owner, Field::AllOf);
                            site.member = member;
                            Self::invalid(site, r)
                        },
                    )?;
                    self.state = State::References(References {
                        names: combo.names[0],
                        position: 0,
                        member: 0,
                        site: Site::new(combo.owner, Field::OneOf),
                        next: Resume::ComboAll {
                            names: combo.names[1],
                            owner: combo.owner,
                        },
                    });
                }
            }
            State::Enter(node) => {
                let depth = self.frames.len().checked_add(1).ok_or(Cause::Arithmetic)?;
                if depth > self.limits.frames {
                    return Err(Cause::Depth {
                        observed: depth,
                        ceiling: self.limits.frames,
                    });
                }
                let ordinal = self.ordinal;
                self.ordinal = ordinal.checked_add(1).ok_or(Cause::Arithmetic)?;
                self.trace.schemas += 1;
                self.trace.maximum_depth = self.trace.maximum_depth.max(depth);
                self.state = State::Push(Frame {
                    node,
                    ordinal,
                    walk: schema::Walk::default(),
                });
            }
            State::Push(frame) => {
                if self.frames.transferring() {
                    if !self.pay(budget, &[W::DocumentNodes])? {
                        self.state = State::Push(frame);
                        return Ok(Tick::Blocked);
                    }
                    self.trace.frame_copies += u64::from(self.frames.copy_pending());
                    self.frames.copy_one();
                    self.state = State::Push(frame);
                } else if self.frames.len() == self.frames.capacity() {
                    let capacity = if self.frames.capacity() == 0 {
                        1
                    } else {
                        self.frames
                            .capacity()
                            .checked_mul(2)
                            .ok_or(Cause::Arithmetic)?
                            .min(self.limits.frames)
                    };
                    self.frames.check_grow(capacity).map_err(frame_cause)?;
                    if !self.pay(budget, &[W::CleanupItems])? {
                        self.state = State::Push(frame);
                        return Ok(Tick::Blocked);
                    }
                    self.frames.begin_grow(capacity).map_err(frame_cause)?;
                    self.trace.frame_requests += 1;
                    self.state = State::Push(frame);
                } else {
                    if !self.pay(budget, &[W::DocumentNodes])? {
                        self.state = State::Push(frame);
                        return Ok(Tick::Blocked);
                    }
                    self.frames.push(frame);
                    self.state = State::Resume(Resume::Schema);
                }
            }
            State::Resume(Resume::Schema) => {
                let frame = self.frames.last_mut();
                let id = frame.node;
                let node = id.bind(self.arenas);
                match frame
                    .walk
                    .action(Access.one_of_count(node), Access.child_count(node))
                {
                    schema::Action::Type => {
                        if let Some(text) = id
                            .child(self.arenas, super::schema_fields::Field::Type as usize)
                            .and_then(|v| v.text(self.arenas))
                        {
                            self.probe(text, Probe::SchemaType);
                        } else {
                            self.advance_schema();
                        }
                    }
                    schema::Action::OneOf(index) | schema::Action::Child(index) => {
                        let child = if matches!(
                            frame
                                .walk
                                .action(Access.one_of_count(node), Access.child_count(node)),
                            schema::Action::OneOf(_)
                        ) {
                            Access.one_of(node, index)
                        } else {
                            Access.child(node, index).1
                        };
                        frame.walk.advance();
                        self.state = State::Enter(NodeId(child.node_id()));
                    }
                    schema::Action::Flags => {
                        self.schema_rule(schema::check_flags(Access.flags(node)))?;
                        self.advance_schema();
                    }
                    schema::Action::Unsigned => {
                        self.state = State::Unsigned {
                            position: 0,
                            values: [None; 4],
                        }
                    }
                    schema::Action::Numeric => {
                        self.state = State::NumericGather {
                            position: 0,
                            numbers: [None; 5],
                        }
                    }
                    schema::Action::Typed => {
                        let result = match node.schema_kind().unwrap() {
                            schema::SchemaKind::Array | schema::SchemaKind::String => {
                                let (min, max) = Access.typed_unsigned(node);
                                schema::check_typed_unsigned(node.schema_kind().unwrap(), min, max)
                            }
                            schema::SchemaKind::Number => {
                                schema::check_typed_numeric(&Access.typed_float(node))
                            }
                            schema::SchemaKind::Integer => {
                                schema::check_typed_numeric(&Access.typed_integer(node))
                            }
                            _ => Ok(()),
                        };
                        self.schema_rule(result)?;
                        self.advance_schema();
                    }
                    schema::Action::Done => {
                        self.frames.pop();
                        self.state = State::Resume(if self.frames.is_empty() {
                            self.after_schema
                        } else {
                            Resume::Schema
                        });
                    }
                }
            }
            State::Unsigned { position, values } => {
                if position == 2 {
                    self.schema_rule(schema::check_unsigned_pair(0, [values[0], values[1]]))?;
                }
                if position == 4 {
                    self.schema_rule(schema::check_unsigned_pair(1, [values[2], values[3]]))?;
                    self.advance_schema();
                } else {
                    self.lookup(
                        self.frames.last().node.extras(self.arenas),
                        Text::Static(schema::FIELDS[position].name()),
                        false,
                        Purpose::Unsigned { position, values },
                    );
                }
            }
            State::UnsignedNumber {
                position,
                mut values,
                text,
            } => {
                self.number_limit(text.borrow(self.arenas))?;
                let value = text.borrow(self.arenas);
                let before = *self.lifetime;
                let parses = &mut self.trace.projections[0];
                let progress = projection_step::project(
                    value,
                    self.limits.number,
                    budget,
                    self.lifetime,
                    cancelled,
                    || {
                        *parses += 1;
                        value.parse::<u64>().ok()
                    },
                );
                self.trace.work[1] += before - *self.lifetime;
                match progress {
                    ProjectionProgress::Pending => {
                        self.state = State::UnsignedNumber {
                            position,
                            values,
                            text,
                        };
                        return Ok(Tick::Blocked);
                    }
                    ProjectionProgress::Limit => return Err(Cause::Lifetime),
                    ProjectionProgress::Cancelled => return Err(Cause::Cancelled),
                    ProjectionProgress::Complete(value) => {
                        values[position] = value;
                        self.state = State::Unsigned {
                            position: position + 1,
                            values,
                        };
                    }
                }
            }
            State::NumericGather { position, numbers } => {
                if position == 5 {
                    self.state = State::Numeric {
                        cursor: NumericCursor::default(),
                        numbers,
                    };
                } else {
                    self.lookup(
                        self.frames.last().node.extras(self.arenas),
                        Text::Static(schema::NUMERIC_FIELDS[position].name()),
                        false,
                        Purpose::Numeric { position, numbers },
                    );
                }
            }
            State::Numeric {
                mut cursor,
                numbers,
            } => {
                let before = *self.lifetime;
                let ceiling = self.limits.number;
                let mut lexical = None;
                let parses = &mut self.trace.projections[1];
                let progress =
                    cursor.step(numbers.map(|t| t.map(|t| t.borrow(self.arenas))), |text| {
                        if text.len() > ceiling {
                            lexical = Some(Cause::Number {
                                observed: text.len(),
                                ceiling,
                            });
                            return ProjectionProgress::Limit;
                        }
                        projection_step::project(
                            text,
                            ceiling,
                            budget,
                            self.lifetime,
                            &mut *cancelled,
                            || {
                                *parses += 1;
                                text.parse::<f64>().ok()
                            },
                        )
                    });
                self.trace.work[1] += before - *self.lifetime;
                match progress {
                    ProjectionProgress::Pending => {
                        self.state = State::Numeric { cursor, numbers };
                        return Ok(Tick::Blocked);
                    }
                    ProjectionProgress::Limit => return Err(lexical.unwrap_or(Cause::Lifetime)),
                    ProjectionProgress::Cancelled => return Err(Cause::Cancelled),
                    ProjectionProgress::Complete(result) => {
                        self.schema_rule(result)?;
                        self.advance_schema();
                    }
                }
            }
            State::Moving => unreachable!(),
        }
        Ok(Tick::Advanced)
    }
}
fn names_len(arenas: Sealed<'_>, names: Option<NodeId>) -> usize {
    names.map_or(0, |v| match v.literal_kind(arenas) {
        Some(Kind::String) => 1,
        Some(Kind::Array) => v.len(arenas),
        _ => 0,
    })
}
fn name_at(arenas: Sealed<'_>, names: NodeId, position: usize) -> Option<Text> {
    let v = if names.literal_kind(arenas) == Some(Kind::String) {
        names
    } else {
        names.child(arenas, position).unwrap()
    };
    (v.literal_kind(arenas) == Some(Kind::String)).then(|| v.text(arenas).unwrap())
}
fn frame_cause(error: FrameError) -> Cause {
    match error {
        FrameError::Limit => Cause::Memory,
        FrameError::Arithmetic => Cause::Arithmetic,
        FrameError::Allocation => Cause::Allocation,
    }
}
const _: () = assert!(!mem::needs_drop::<Frame>());

fn state_phase(state: &State) -> Phase {
    match state {
        State::Lookup(_) => Phase::Lookup,
        State::Probe { .. } => Phase::Text,
        State::Enter(_) | State::Resume(Resume::Schema) => Phase::Schema,
        State::Push(_) => Phase::Frames,
        State::Unsigned { .. }
        | State::UnsignedNumber { .. }
        | State::NumericGather { .. }
        | State::Numeric { .. } => Phase::Numeric,
        _ => Phase::Walk,
    }
}
const _: () = assert!(!mem::needs_drop::<Continuation>());
