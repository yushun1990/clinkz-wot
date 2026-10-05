//! Charged canonical DataSchema construction from the actual literal owner.
//! The input stays live through a second, paid equivalence pass. No Thing,
//! serde graph, scalar formatting, recursive emission or second ledger occurs.
use super::{
    schema_arena::{DecodeError, Extras, List},
    schema_fields::{Field, Shape},
    schema_kernel::SchemaKind,
    schema_step::{self, Fields, Machine, Outcome},
    schema_tree, thing_build as typed,
};
use clinkz_wot_foundation::{WorkBudget, WorkClass as W};
use core::mem;
use validated_thing_value_construction_probe::{
    Cause as ResourceCause, Edge, FrameError, Kind, Limits, Node, OwnedValue, Rebuild, Sealed,
    Site, View as Literal,
};

const ABSENT: u32 = u32::MAX;
const UNSIGNED: u32 = 8;
const FLOAT: u32 = 9;
const INTEGER: u32 = 10;
const SCHEMA: u32 = 16;
const SLOTS: usize = Field::ALL.len() + 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    Literal(validated_thing_value_construction_probe::Phase),
    Basic(schema_tree::Phase),
    Canonical(Pass),
    Seal,
    ThingBasic(super::thing_step::Phase),
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ConstructionTrace {
    pub literal: validated_thing_value_construction_probe::Trace,
    pub basic: schema_tree::Trace,
    pub canonical: Trace,
    pub seal: [u64; 4],
    pub thing_basic: super::thing_step::Trace,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstructionCause {
    Literal(ResourceCause),
    Basic(schema_tree::Cause),
    Canonical(Cause),
    Seal(ResourceCause),
    ThingBasic(super::thing_step::Cause),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConstructionFailure {
    pub cause: ConstructionCause,
    pub stage: Stage,
    pub trace: ConstructionTrace,
    pub resources: validated_thing_value_construction_probe::Footprint,
    pub lifetime_remaining: u64,
    pub live_after_rollback: u64,
    pub allocations: u64,
    pub releases: u64,
}
/// Fixture driver, not the frozen owning admission cursor. It services actual
/// borrowing continuations in lexical scopes, so no self-reference, lifetime
/// erasure or source pointer can escape in the typed result. The control hook
/// supplies a fresh budget/cancel value at every Pending boundary.
#[allow(clippy::result_large_err)]
pub fn from_json(
    input: &[u8],
    limits: Limits,
    mut control: impl FnMut(Stage) -> (WorkBudget, bool),
) -> Result<(Schema, ConstructionTrace), ConstructionFailure> {
    use validated_thing_value_construction_probe::{
        Cursor as LiteralCursor, Progress as LiteralProgress, SealProgress,
    };
    let mut trace = ConstructionTrace::default();
    let mut cursor = LiteralCursor::from_json(input, limits);
    let mut owner = loop {
        let stage = Stage::Literal(cursor.phase());
        let (mut budget, cancel) = control(stage);
        match cursor.step(&mut budget, cancel) {
            LiteralProgress::Pending(next) => cursor = next,
            LiteralProgress::Complete(owner) => {
                trace.literal = owner.trace();
                break owner;
            }
            LiteralProgress::Failed(failure) => {
                trace.literal = failure.trace;
                return Err(ConstructionFailure {
                    cause: ConstructionCause::Literal(failure.cause),
                    stage: Stage::Literal(failure.phase),
                    trace,
                    resources: failure.resources,
                    lifetime_remaining: limits.lifetime
                        - failure.trace.work.into_iter().sum::<u64>(),
                    live_after_rollback: failure.live_after_rollback,
                    allocations: failure.allocations,
                    releases: failure.releases,
                });
            }
        }
    };
    let result = {
        let mut cursor = schema_tree::Cursor::new(&mut owner, limits.number, limits.frames);
        loop {
            let stage = Stage::Basic(cursor.phase());
            let (mut budget, cancel) = control(stage);
            match cursor.step(&mut budget, || cancel) {
                schema_tree::Progress::Pending(next) => cursor = next,
                schema_tree::Progress::Complete(done) => {
                    trace.basic = done;
                    break Ok(());
                }
                schema_tree::Progress::Failed(failure) => {
                    trace.basic = failure.trace;
                    break Err((
                        ConstructionCause::Basic(failure.cause),
                        Stage::Basic(failure.phase),
                    ));
                }
            }
        }
    };
    if let Err((cause, stage)) = result {
        return Err(rollback(owner, cause, stage, trace));
    }
    let result = {
        let mut cursor = Cursor::new(&mut owner, limits);
        loop {
            let stage = Stage::Canonical(cursor.pass());
            let (mut budget, cancel) = control(stage);
            match cursor.step(&mut budget, || cancel) {
                Progress::Pending(next) => cursor = next,
                Progress::Complete(done) => {
                    trace.canonical = done;
                    break Ok(());
                }
                Progress::Failed(failure) => {
                    trace.canonical = failure.trace;
                    break Err((
                        ConstructionCause::Canonical(failure.cause),
                        Stage::Canonical(failure.pass),
                    ));
                }
            }
        }
    };
    if let Err((cause, stage)) = result {
        return Err(rollback(owner, cause, stage, trace));
    }
    let result = {
        let mut cursor = owner.reseal();
        loop {
            let (mut budget, cancel) = control(Stage::Seal);
            match cursor.step(&mut budget, cancel) {
                SealProgress::Pending(next) => cursor = next,
                SealProgress::Complete(work) => {
                    trace.seal = work;
                    break Ok(());
                }
                SealProgress::Failed { cause, work } => {
                    trace.seal = work;
                    break Err(cause);
                }
            }
        }
    };
    if let Err(cause) = result {
        return Err(rollback(
            owner,
            ConstructionCause::Seal(cause),
            Stage::Seal,
            trace,
        ));
    }
    Ok((Schema::from_resealed(owner), trace))
}
fn rollback(
    mut owner: OwnedValue,
    cause: ConstructionCause,
    stage: Stage,
    trace: ConstructionTrace,
) -> ConstructionFailure {
    owner.rollback_for_fixture();
    ConstructionFailure {
        cause,
        stage,
        trace,
        resources: owner.footprint(),
        lifetime_remaining: owner.lifetime_remaining(),
        live_after_rollback: owner.footprint().retained_requested_bytes,
        allocations: owner.allocations(),
        releases: owner.releases(),
    }
}

/// Direct typed entry into the same emitter and owning transaction. The
/// synchronous Basic oracle is intentionally outside this construction proof.
#[allow(clippy::result_large_err)]
pub(crate) fn from_typed_thing(
    input: &crate::thing::Thing,
    limits: Limits,
    control: impl FnMut(Stage) -> (WorkBudget, bool),
) -> Result<(typed::NormalizedThing, ConstructionTrace), ConstructionFailure> {
    typed_thing(input, limits, control, false)
}
#[allow(clippy::result_large_err)]
pub(crate) fn from_typed_thing_basic(
    input: &crate::thing::Thing,
    limits: Limits,
    control: impl FnMut(Stage) -> (WorkBudget, bool),
) -> Result<(typed::NormalizedThing, ConstructionTrace), ConstructionFailure> {
    typed_thing(input, limits, control, true)
}
#[allow(clippy::result_large_err)]
fn typed_thing(
    input: &crate::thing::Thing,
    limits: Limits,
    mut control: impl FnMut(Stage) -> (WorkBudget, bool),
    with_basic: bool,
) -> Result<(typed::NormalizedThing, ConstructionTrace), ConstructionFailure> {
    use validated_thing_value_construction_probe::SealProgress;
    let mut owner = OwnedValue::empty_for_fixture(limits);
    let mut trace = ConstructionTrace::default();
    let result = {
        let mut cursor = Cursor::from_thing(&mut owner, input, limits);
        loop {
            let (mut budget, cancel) = control(Stage::Canonical(cursor.pass()));
            match cursor.step(&mut budget, || cancel) {
                Progress::Pending(next) => cursor = next,
                Progress::Complete(done) => {
                    trace.canonical = done;
                    break Ok(());
                }
                Progress::Failed(failure) => {
                    trace.canonical = failure.trace;
                    break Err((
                        ConstructionCause::Canonical(failure.cause),
                        Stage::Canonical(failure.pass),
                    ));
                }
            }
        }
    };
    if let Err((cause, stage)) = result {
        return Err(rollback(owner, cause, stage, trace));
    }
    let result = {
        let mut cursor = owner.reseal();
        loop {
            let (mut budget, cancel) = control(Stage::Seal);
            match cursor.step(&mut budget, cancel) {
                SealProgress::Pending(next) => cursor = next,
                SealProgress::Complete(work) => {
                    trace.seal = work;
                    break Ok(());
                }
                SealProgress::Failed { cause, work } => {
                    trace.seal = work;
                    break Err(cause);
                }
            }
        }
    };
    if let Err(cause) = result {
        return Err(rollback(
            owner,
            ConstructionCause::Seal(cause),
            Stage::Seal,
            trace,
        ));
    }
    if with_basic {
        let result = {
            let mut cursor = super::thing_step::Cursor::from_owner(&mut owner, limits);
            loop {
                let (mut budget, cancel) = control(Stage::ThingBasic(cursor.phase()));
                match cursor.step(&mut budget, || cancel) {
                    super::thing_step::Progress::Pending(next) => cursor = next,
                    super::thing_step::Progress::Complete(work) => {
                        trace.thing_basic = work;
                        break Ok(());
                    }
                    super::thing_step::Progress::Failed(failure) => {
                        trace.thing_basic = failure.trace;
                        break Err((failure.cause, failure.phase));
                    }
                }
            }
        };
        if let Err((cause, phase)) = result {
            return Err(rollback(
                owner,
                ConstructionCause::ThingBasic(cause),
                Stage::ThingBasic(phase),
                trace,
            ));
        }
    }
    Ok((typed::NormalizedThing::new(owner), trace))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pass {
    Construction,
    Equivalence,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cause {
    Resource(ResourceCause),
    Field { ordinal: u64, error: DecodeError },
    SemanticMismatch,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Trace {
    /// DocumentNodes, input bytes, output bytes, schema nodes, cleanup, URI bytes.
    pub work: [u64; 6],
    pub schemas: [u64; 2],
    pub field_parses: [u64; 3],
    pub grow_copies: u64,
    pub compared_nodes: u64,
    pub compared_edges: u64,
    pub compared_bytes: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Failure {
    pub cause: Cause,
    pub pass: Pass,
    pub trace: Trace,
}

// Boxing the paid field index would introduce an unauthorized fifth site.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Copy)]
enum Task<'a> {
    Schema(Literal<'a>),
    Literal(Literal<'a>),
    Text(&'a str, Kind),
    Scalar(u32, u64),
    List(List<'a>, bool),
    Map(Literal<'a>, bool),
    Entry(&'a str, Literal<'a>, bool),
    Extras(Extras<'a>),
    Typed(typed::Task<'a>),
}
#[derive(Clone, Copy)]
enum Root<'a> {
    Schema(Literal<'a>),
    Thing(&'a crate::thing::Thing),
}
impl<'a> Root<'a> {
    fn task(self) -> Task<'a> {
        match self {
            Self::Schema(value) => Task::Schema(value),
            Self::Thing(value) => Task::Typed(typed::Task::Thing(value)),
        }
    }
}
#[allow(clippy::large_enum_variant)]
enum Mode<'a> {
    Schema(Fields<'a>),
    List(List<'a>, bool),
    Map(Literal<'a>, bool),
    Entry(&'a str, Literal<'a>, bool),
    Extras {
        extras: Extras<'a>,
        source_index: usize,
    },
    Text(&'a str),
    Leaf,
    Typed(typed::Children<'a>),
}
struct Frame<'a> {
    mode: Mode<'a>,
    node: Node,
    id: u32,
    position: usize,
}
#[derive(Clone, Copy)]
struct Link {
    at: usize,
    index: u32,
}
/// Selection over a caller JSON map pays each iterator transition and each
/// compared byte. It keeps only borrowed handles in the existing frame owner;
/// downstream preserve_order cannot require a sort/key allocation category.
struct Selection<'a> {
    iter: serde_json::map::Iter<'a>,
    previous: Option<&'a str>,
    best: Option<(&'a str, &'a serde_json::Value)>,
    candidate: Option<(&'a str, &'a serde_json::Value)>,
    phase: u8,
    byte: usize,
}
impl<'a> Selection<'a> {
    fn new(
        map: &'a serde_json::Map<alloc::string::String, serde_json::Value>,
        previous: Option<&'a str>,
    ) -> Self {
        Self {
            iter: map.iter(),
            previous,
            best: None,
            candidate: None,
            phase: 0,
            byte: 0,
        }
    }
    fn reads_byte(&self) -> bool {
        if self.phase == 0 {
            return false;
        }
        let left = self.candidate.unwrap().0;
        let right = if self.phase == 1 {
            self.previous.unwrap()
        } else {
            self.best.unwrap().0
        };
        self.byte < left.len().min(right.len())
    }
    fn tick(&mut self) -> Option<(&'a str, &'a serde_json::Value)> {
        use core::cmp::Ordering;
        if self.phase == 0 {
            let Some((key, value)) = self.iter.next() else {
                return Some(self.best.unwrap());
            };
            self.candidate = Some((key.as_str(), value));
            self.byte = 0;
            if self.previous.is_some() {
                self.phase = 1;
            } else if self.best.is_some() {
                self.phase = 2;
            } else {
                self.best = self.candidate;
            }
            return None;
        }
        let left = self.candidate.unwrap().0;
        let right = if self.phase == 1 {
            self.previous.unwrap()
        } else {
            self.best.unwrap().0
        };
        let ordering = match (
            left.as_bytes().get(self.byte),
            right.as_bytes().get(self.byte),
        ) {
            (Some(a), Some(b)) if a == b => {
                self.byte += 1;
                return None;
            }
            (Some(a), Some(b)) => a.cmp(b),
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            (None, None) => Ordering::Equal,
        };
        if self.phase == 1 && ordering == Ordering::Greater {
            if self.best.is_some() {
                self.phase = 2;
                self.byte = 0;
            } else {
                self.best = self.candidate;
                self.phase = 0;
            }
        } else {
            if self.phase == 2 && ordering == Ordering::Less {
                self.best = self.candidate;
            }
            self.phase = 0;
        }
        None
    }
}
#[allow(clippy::large_enum_variant)]
enum State<'a> {
    Enter(Task<'a>, Option<Link>),
    Project(Machine<'a>, Option<Link>, u64),
    CountExtras {
        extras: Extras<'a>,
        link: Option<Link>,
        position: usize,
        count: usize,
    },
    Header {
        frame: Frame<'a>,
        link: Option<Link>,
        reserved: Option<usize>,
    },
    Text {
        node: u32,
        text: &'a str,
        position: usize,
    },
    Next,
    Select(Selection<'a>, Link),
    Moving,
}
pub struct Cursor<'a> {
    root: Root<'a>,
    lifetime: &'a mut u64,
    build: Rebuild<'a, Frame<'a>>,
    limits: Limits,
    state: State<'a>,
    pass: Pass,
    nodes: usize,
    edges: usize,
    bytes: usize,
    trace: Trace,
}
#[allow(clippy::large_enum_variant)]
pub enum Progress<'a> {
    Pending(Cursor<'a>),
    Complete(Trace),
    Failed(Failure),
}
impl<'a> Cursor<'a> {
    /// Run the charged subtree field/Basic visitor first. This borrower only
    /// constructs its typed result; it does not grant whole-Thing admission.
    pub fn new(owner: &'a mut OwnedValue, limits: Limits) -> Self {
        let (root, lifetime, build) = owner.rebuild_parts();
        Self {
            root: Root::Schema(root),
            lifetime,
            build,
            limits,
            state: State::Enter(Task::Schema(root), None),
            pass: Pass::Construction,
            nodes: 0,
            edges: 0,
            bytes: 0,
            trace: Trace::default(),
        }
    }
    pub fn from_thing(
        owner: &'a mut OwnedValue,
        input: &'a crate::thing::Thing,
        limits: Limits,
    ) -> Self {
        let mut cursor = Self::new(owner, limits);
        cursor.root = Root::Thing(input);
        cursor.state = State::Enter(cursor.root.task(), None);
        cursor
    }
    pub fn pass(&self) -> Pass {
        self.pass
    }
    pub fn trace(&self) -> Trace {
        self.trace
    }
    pub fn lifetime_remaining(&self) -> u64 {
        *self.lifetime
    }
    pub fn step(
        mut self,
        budget: &mut WorkBudget,
        mut cancelled: impl FnMut() -> bool,
    ) -> Progress<'a> {
        loop {
            if cancelled() {
                return self.fail(Cause::Resource(ResourceCause::Cancelled));
            }
            match self.tick(budget, &mut cancelled) {
                Ok(Some(true)) => return Progress::Complete(self.trace),
                Ok(Some(false)) => {}
                Ok(None) => return Progress::Pending(self),
                Err(cause) => return self.fail(cause),
            }
        }
    }
    fn fail(self, cause: Cause) -> Progress<'a> {
        // Rebuild's prepaid drop ends all borrows and releases only frames and
        // an unfinished replacement. The containing owner owns both graphs.
        Progress::Failed(Failure {
            cause,
            pass: self.pass,
            trace: self.trace,
        })
    }
    fn pay(&mut self, budget: &mut WorkBudget, classes: &[W]) -> Result<bool, Cause> {
        if classes.iter().any(|&class| {
            budget.remaining(class)
                < classes
                    .iter()
                    .filter(|&&required| required == class)
                    .count() as u64
        }) {
            return Ok(false);
        }
        if *self.lifetime < classes.len() as u64 {
            return Err(Cause::Resource(ResourceCause::Lifetime));
        }
        for &class in classes {
            budget.consume(class, 1).unwrap();
            self.trace.work[match class {
                W::DocumentNodes => 0,
                W::CodecInputBytes => 1,
                W::CodecOutputBytes => 2,
                W::JsonSchemaNodes => 3,
                W::CleanupItems => 4,
                W::UriBytes => 5,
                _ => unreachable!(),
            }] += 1;
        }
        *self.lifetime -= classes.len() as u64;
        Ok(true)
    }
    fn ensure(&mut self, site: Site, budget: &mut WorkBudget) -> Result<bool, Cause> {
        let length = self
            .build
            .len(site)
            .checked_add(1)
            .ok_or(Cause::Resource(ResourceCause::Arithmetic))?;
        let ceiling = match site {
            Site::Nodes => self.limits.nodes.min(ABSENT as usize),
            Site::Edges => self.limits.edges.min(ABSENT as usize),
            Site::Bytes => self.limits.bytes.min(ABSENT as usize),
            Site::Frames => self.limits.frames,
        };
        if length > ceiling {
            return Err(Cause::Resource(match site {
                Site::Nodes => ResourceCause::Nodes,
                Site::Edges => ResourceCause::Edges,
                Site::Bytes => ResourceCause::Bytes,
                Site::Frames => ResourceCause::Frames,
            }));
        }
        if length <= self.build.capacity(site) {
            return Ok(true);
        }
        let capacity = self
            .build
            .capacity(site)
            .max(1)
            .checked_mul(2)
            .ok_or(Cause::Resource(ResourceCause::Arithmetic))?
            .max(length)
            .min(ceiling);
        self.build.check_grow(site, capacity).map_err(arena_cause)?;
        if !self.pay(budget, &[W::DocumentNodes, W::CleanupItems])? {
            return Ok(false);
        }
        self.build.begin_grow(site, capacity).map_err(arena_cause)?;
        Ok(false)
    }
    fn header(&self, mode: Mode<'a>, kind: u32, count: usize, bits: Option<u64>) -> Frame<'a> {
        let (first_byte, byte_count) = match (&mode, bits) {
            (_, Some(bits)) => (bits as u32, (bits >> 32) as u32),
            (Mode::Text(text), _) => (self.bytes as u32, text.len() as u32),
            _ => (0, 0),
        };
        Frame {
            mode,
            node: Node {
                kind,
                first_edge: self.edges as u32,
                edge_count: count as u32,
                first_byte,
                byte_count,
            },
            id: self.nodes as u32,
            position: 0,
        }
    }
    fn tick(
        &mut self,
        budget: &mut WorkBudget,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<Option<bool>, Cause> {
        if let Some(site) = self.build.transferring() {
            let copying = self.build.copy_pending();
            let classes: &[W] = if site == Site::Bytes && copying {
                &[W::DocumentNodes, W::CodecInputBytes, W::CodecOutputBytes]
            } else {
                &[W::DocumentNodes]
            };
            if !self.pay(budget, classes)? {
                return Ok(None);
            }
            self.build.copy_one();
            self.trace.grow_copies += u64::from(copying);
            return Ok(Some(false));
        }
        // Growth is its own transition. Preflight it before debiting either
        // structural or cleanup credit; a successful request starts transfer.
        let site = match &self.state {
            State::Header { reserved: None, .. } if self.pass == Pass::Construction => {
                Some(Site::Nodes)
            }
            State::Header {
                frame,
                reserved: Some(reserved),
                ..
            } if *reserved < frame.node.edge_count as usize && self.pass == Pass::Construction => {
                Some(Site::Edges)
            }
            State::Header {
                frame,
                reserved: Some(reserved),
                ..
            } if *reserved == frame.node.edge_count as usize
                && frame.node.edge_count != 0
                && !matches!(&frame.mode, Mode::Text(_)) =>
            {
                Some(Site::Frames)
            }
            State::Text { text, position, .. }
                if *position < text.len() && self.pass == Pass::Construction =>
            {
                Some(Site::Bytes)
            }
            _ => None,
        };
        if let Some(site) = site
            && !self.ensure(site, budget)?
        {
            return Ok(None);
        }
        // The field machine pays its own work. Every other transition checks
        // all its classes together, so a blocked poll spends no partial debit.
        let classes: &[W] = match &self.state {
            State::Project(..) => &[],
            State::Select(..) => &[],
            State::Enter(Task::Typed(typed::Task::Schema(_)), _) => {
                &[W::DocumentNodes, W::JsonSchemaNodes]
            }
            State::Text {
                node,
                text,
                position,
            } if *position < text.len()
                && (typed::URI..=typed::BASE_TEMPLATE)
                    .contains(&self.build.node(*node as usize).kind) =>
            {
                &[
                    W::DocumentNodes,
                    W::CodecInputBytes,
                    W::CodecOutputBytes,
                    W::UriBytes,
                    W::UriBytes,
                ]
            }
            State::Text { text, position, .. } if *position < text.len() => {
                &[W::DocumentNodes, W::CodecInputBytes, W::CodecOutputBytes]
            }
            _ => &[W::DocumentNodes],
        };
        if !self.pay(budget, classes)? {
            return Ok(None);
        }
        let state = mem::replace(&mut self.state, State::Moving);
        match state {
            State::Enter(Task::Schema(source), link) => {
                let ordinal = self.trace.schemas[usize::from(self.pass == Pass::Equivalence)];
                self.trace.schemas[usize::from(self.pass == Pass::Equivalence)] += 1;
                self.state =
                    State::Project(Machine::new(source, self.limits.number), link, ordinal);
            }
            State::Project(machine, link, ordinal) => {
                let before = machine.trace();
                let result = machine.step(budget, self.lifetime, cancelled);
                let after = match &result {
                    Outcome::Pending(machine) => machine.trace(),
                    Outcome::Complete { trace, .. } => *trace,
                    Outcome::Failed(failure) => failure.trace,
                };
                for (i, output) in [0, 1, 3].into_iter().enumerate() {
                    self.trace.work[output] += after.work[i] - before.work[i];
                }
                for i in 0..3 {
                    self.trace.field_parses[i] += after.parses[i] - before.parses[i];
                }
                match result {
                    Outcome::Pending(machine) => {
                        self.state = State::Project(machine, link, ordinal);
                        return Ok(None);
                    }
                    Outcome::Failed(failure) => {
                        return Err(match failure.cause {
                            schema_step::Cause::Field(error) => Cause::Field { ordinal, error },
                            schema_step::Cause::NumberLimit { .. } => {
                                Cause::Resource(ResourceCause::DecodedNumber)
                            }
                            schema_step::Cause::Lifetime => {
                                Cause::Resource(ResourceCause::Lifetime)
                            }
                            schema_step::Cause::Cancelled => {
                                Cause::Resource(ResourceCause::Cancelled)
                            }
                        });
                    }
                    Outcome::Complete { fields, .. } => {
                        let kind = shape_kind(&fields.shape);
                        let frame =
                            self.header(Mode::Schema(fields), SCHEMA + kind as u32, SLOTS, None);
                        self.state = State::Header {
                            frame,
                            link,
                            reserved: None,
                        };
                    }
                }
            }
            State::Enter(Task::Extras(extras), link) => {
                self.state = State::CountExtras {
                    extras,
                    link,
                    position: 0,
                    count: 0,
                };
            }
            State::CountExtras {
                extras,
                link,
                mut position,
                mut count,
            } => {
                if position == extras.object.len() {
                    let frame = self.header(
                        Mode::Extras {
                            extras,
                            source_index: 0,
                        },
                        Kind::Object as u32,
                        count,
                        None,
                    );
                    self.state = State::Header {
                        frame,
                        link,
                        reserved: None,
                    };
                } else {
                    count += usize::from(
                        !extras.consumed_value(extras.object.member(position).unwrap().1),
                    );
                    position += 1;
                    self.state = State::CountExtras {
                        extras,
                        link,
                        position,
                        count,
                    };
                }
            }
            State::Enter(task, link) => {
                let frame = match task {
                    Task::Typed(task) => {
                        let description = typed::describe(task);
                        if description.kind == Kind::Number as u32
                            && description.text.unwrap().len() > self.limits.number
                        {
                            return Err(Cause::Resource(ResourceCause::DecodedNumber));
                        }
                        // The strict source is already u32-bounded; direct
                        // typed input needs this check before header casts.
                        u32::try_from(description.count)
                            .map_err(|_| Cause::Resource(ResourceCause::Arithmetic))?;
                        if let Some(text) = description.text {
                            u32::try_from(text.len())
                                .map_err(|_| Cause::Resource(ResourceCause::Arithmetic))?;
                        }
                        if matches!(task, typed::Task::Schema(_)) {
                            self.trace.schemas[usize::from(self.pass == Pass::Equivalence)] += 1;
                        }
                        let mode = description
                            .text
                            .map_or(Mode::Typed(description.children), Mode::Text);
                        self.header(mode, description.kind, description.count, description.bits)
                    }
                    Task::Text(text, kind) => self.header(Mode::Text(text), kind as u32, 0, None),
                    Task::Scalar(kind, bits) => self.header(Mode::Leaf, kind, 0, Some(bits)),
                    Task::List(list, schemas) => self.header(
                        Mode::List(list, schemas),
                        Kind::Array as u32,
                        list.len(),
                        None,
                    ),
                    Task::Map(map, schemas) => self.header(
                        Mode::Map(map, schemas),
                        Kind::Object as u32,
                        map.len(),
                        None,
                    ),
                    Task::Entry(key, value, schema) => {
                        self.header(Mode::Entry(key, value, schema), Kind::Entry as u32, 2, None)
                    }
                    Task::Literal(value) => match value.kind() {
                        Kind::String | Kind::Number => self.header(
                            Mode::Text(value.text().unwrap()),
                            value.kind() as u32,
                            0,
                            None,
                        ),
                        Kind::Array => self.header(
                            Mode::List(
                                List {
                                    value,
                                    single: false,
                                },
                                false,
                            ),
                            Kind::Array as u32,
                            value.len(),
                            None,
                        ),
                        Kind::Object => self.header(
                            Mode::Map(value, false),
                            Kind::Object as u32,
                            value.len(),
                            None,
                        ),
                        kind => self.header(Mode::Leaf, kind as u32, 0, None),
                    },
                    _ => unreachable!(),
                };
                self.state = State::Header {
                    frame,
                    link,
                    reserved: None,
                };
            }
            State::Header {
                frame,
                link,
                reserved: None,
            } => {
                if self.pass == Pass::Construction {
                    self.build.push_node(frame.node);
                } else {
                    if self.nodes >= self.build.len(Site::Nodes)
                        || self.build.node(self.nodes) != frame.node
                    {
                        return Err(Cause::SemanticMismatch);
                    }
                    self.trace.compared_nodes += 1;
                }
                if let Some(link) = link {
                    let edge = Edge {
                        target: frame.id,
                        original_index: link.index,
                    };
                    if self.pass == Pass::Construction {
                        self.build.set_edge(link.at, edge);
                    } else {
                        self.compare_edge(link.at, edge)?;
                    }
                }
                self.nodes += 1;
                self.state = State::Header {
                    frame,
                    link,
                    reserved: Some(0),
                };
            }
            State::Header {
                frame,
                link,
                reserved: Some(mut reserved),
            } => {
                if reserved < frame.node.edge_count as usize {
                    if self.pass == Pass::Construction {
                        self.build.push_edge(Edge {
                            target: ABSENT,
                            original_index: reserved as u32,
                        });
                    } else if self.edges >= self.build.len(Site::Edges)
                        || self.build.edge(self.edges).original_index != reserved as u32
                    {
                        return Err(Cause::SemanticMismatch);
                    }
                    self.edges += 1;
                    reserved += 1;
                    self.state = State::Header {
                        frame,
                        link,
                        reserved: Some(reserved),
                    };
                } else if let Mode::Text(text) = frame.mode {
                    self.state = State::Text {
                        node: frame.id,
                        text,
                        position: 0,
                    };
                } else if frame.node.edge_count == 0 {
                    self.state = State::Next;
                } else {
                    self.build.push_frame(frame);
                    self.state = State::Next;
                }
            }
            State::Text {
                node,
                text,
                mut position,
            } => {
                if position == text.len() {
                    self.state = State::Next;
                } else {
                    let byte = text.as_bytes()[position];
                    if self.pass == Pass::Construction {
                        self.build.push_byte(byte);
                    } else {
                        if self.bytes >= self.build.len(Site::Bytes)
                            || self.build.byte(self.bytes) != byte
                        {
                            return Err(Cause::SemanticMismatch);
                        }
                        self.trace.compared_bytes += 1;
                    }
                    self.bytes += 1;
                    position += 1;
                    self.state = State::Text {
                        node,
                        text,
                        position,
                    };
                }
            }
            State::Next => {
                if self.build.len(Site::Frames) == 0 {
                    if self.pass == Pass::Equivalence {
                        if [self.nodes, self.edges, self.bytes]
                            != [
                                self.build.len(Site::Nodes),
                                self.build.len(Site::Edges),
                                self.build.len(Site::Bytes),
                            ]
                        {
                            return Err(Cause::SemanticMismatch);
                        }
                        return Ok(Some(true));
                    }
                    self.pass = Pass::Equivalence;
                    self.nodes = 0;
                    self.edges = 0;
                    self.bytes = 0;
                    self.state = State::Enter(self.root.task(), None);
                } else {
                    let frame = self.build.frame_mut();
                    if frame.position == frame.node.edge_count as usize {
                        self.build.pop_frame();
                        self.state = State::Next;
                    } else {
                        let index = frame.position;
                        if let Mode::Typed(typed::Children::Json { map, previous }) = &frame.mode {
                            let selection = Selection::new(map, *previous);
                            let link = Link {
                                at: frame.node.first_edge as usize + index,
                                index: index as u32,
                            };
                            frame.position += 1;
                            self.state = State::Select(selection, link);
                            return Ok(Some(false));
                        }
                        let task = match &mut frame.mode {
                            Mode::Typed(children) => children.next(index).map(Task::Typed),
                            Mode::Schema(fields) => {
                                if index == Field::ALL.len() {
                                    Some(Task::Extras(fields.context.extras))
                                } else {
                                    field_task(Field::ALL[index], fields)
                                }
                            }
                            Mode::List(list, schemas) => Some(if *schemas {
                                Task::Schema(list.get(index).unwrap())
                            } else {
                                Task::Literal(list.get(index).unwrap())
                            }),
                            Mode::Map(map, schemas) => {
                                let (key, value) = map.member(index).unwrap();
                                Some(Task::Entry(key, value, *schemas))
                            }
                            Mode::Entry(key, value, schema) => Some(if index == 0 {
                                Task::Text(key, Kind::String)
                            } else if *schema {
                                Task::Schema(*value)
                            } else {
                                Task::Literal(*value)
                            }),
                            Mode::Extras {
                                extras,
                                source_index,
                            } => {
                                let (key, value) = extras.object.member(*source_index).unwrap();
                                *source_index += 1;
                                if extras.consumed_value(value) {
                                    self.state = State::Next;
                                    return Ok(Some(false));
                                }
                                Some(Task::Entry(key, value, false))
                            }
                            _ => unreachable!(),
                        };
                        let link = Link {
                            at: frame.node.first_edge as usize + index,
                            index: index as u32,
                        };
                        frame.position += 1;
                        if let Some(task) = task {
                            self.state = State::Enter(task, Some(link));
                        } else {
                            if self.pass == Pass::Equivalence {
                                self.compare_edge(
                                    link.at,
                                    Edge {
                                        target: ABSENT,
                                        original_index: link.index,
                                    },
                                )?;
                            }
                            self.state = State::Next;
                        }
                    }
                }
            }
            State::Select(mut selection, link) => {
                let classes: &[W] = if selection.reads_byte() {
                    // Both operands are caller-owned key bytes. Precharge
                    // the pair before either read, just as URI pairs do.
                    &[W::DocumentNodes, W::CodecInputBytes, W::CodecInputBytes]
                } else {
                    &[W::DocumentNodes]
                };
                if !self.pay(budget, classes)? {
                    self.state = State::Select(selection, link);
                    return Ok(None);
                }
                if let Some((key, value)) = selection.tick() {
                    let Mode::Typed(typed::Children::Json { previous, .. }) =
                        &mut self.build.frame_mut().mode
                    else {
                        unreachable!()
                    };
                    *previous = Some(key);
                    self.state = State::Enter(
                        Task::Typed(typed::Task::Entry(key, typed::Atom::Value(value))),
                        Some(link),
                    );
                } else {
                    self.state = State::Select(selection, link);
                }
            }
            State::Moving => unreachable!(),
        }
        Ok(Some(false))
    }
    fn compare_edge(&mut self, at: usize, edge: Edge) -> Result<(), Cause> {
        if self.build.edge(at) != edge {
            return Err(Cause::SemanticMismatch);
        }
        self.trace.compared_edges += 1;
        Ok(())
    }
    /// Deliberate fixture fault to prove the equivalence pass can falsify a
    /// malformed output. This is not part of any proposed public TD surface.
    pub fn corrupt_root_for_test(&mut self) {
        let mut node = self.build.node(0);
        node.kind ^= 1;
        self.build.set_node(0, node);
    }
}
fn arena_cause(error: FrameError) -> Cause {
    Cause::Resource(match error {
        FrameError::Limit => ResourceCause::Memory,
        FrameError::Arithmetic => ResourceCause::Arithmetic,
        FrameError::Allocation => ResourceCause::Allocation,
    })
}
fn shape_kind<B: super::schema_fields::Source>(shape: &Shape<B>) -> SchemaKind {
    match shape {
        Shape::Array { .. } => SchemaKind::Array,
        Shape::Boolean => SchemaKind::Boolean,
        Shape::Number(_) => SchemaKind::Number,
        Shape::Integer(_) => SchemaKind::Integer,
        Shape::Object { .. } => SchemaKind::Object,
        Shape::String { .. } => SchemaKind::String,
        Shape::Null => SchemaKind::Null,
    }
}
fn field_task<'a>(field: Field, fields: &Fields<'a>) -> Option<Task<'a>> {
    let c = &fields.context;
    let text = |value: Option<&'a str>| value.map(|text| Task::Text(text, Kind::String));
    let unsigned = |value: Option<u32>| value.map(|value| Task::Scalar(UNSIGNED, value as u64));
    match field {
        Field::Tags => c.metadata.tags.map(|list| Task::List(list, false)),
        Field::Title => text(c.metadata.title),
        Field::Titles => c.metadata.titles.map(Task::Literal),
        Field::Description => text(c.metadata.description),
        Field::Descriptions => c.metadata.descriptions.map(Task::Literal),
        Field::Const => c.constant.map(Task::Literal),
        Field::Default => c.default.map(Task::Literal),
        Field::Unit => text(c.unit),
        Field::OneOf => c.one_of.map(|list| Task::List(list, true)),
        Field::Enum => c.enumerate.map(Task::Literal),
        Field::ReadOnly => Some(Task::Scalar(
            if c.read_only { Kind::True } else { Kind::False } as u32,
            0,
        )),
        Field::WriteOnly => Some(Task::Scalar(
            if c.write_only {
                Kind::True
            } else {
                Kind::False
            } as u32,
            0,
        )),
        Field::Format => text(c.format),
        Field::Type => text(c.data_type),
        Field::Items => {
            if let Shape::Array { items, .. } = &fields.shape {
                items.map(|list| Task::List(list, true))
            } else {
                None
            }
        }
        Field::MinItems | Field::MaxItems => {
            if let Shape::Array { min, max, .. } = fields.shape {
                unsigned(if field == Field::MinItems { min } else { max })
            } else {
                None
            }
        }
        Field::MinLength | Field::MaxLength => {
            if let Shape::String { min, max, .. } = fields.shape {
                unsigned(if field == Field::MinLength { min } else { max })
            } else {
                None
            }
        }
        Field::Minimum
        | Field::ExclusiveMinimum
        | Field::Maximum
        | Field::ExclusiveMaximum
        | Field::MultipleOf => {
            let index = field as usize - Field::Minimum as usize;
            match &fields.shape {
                Shape::Number(values) => {
                    values[index].map(|value| Task::Scalar(FLOAT, value.to_bits()))
                }
                Shape::Integer(values) => {
                    values[index].map(|value| Task::Scalar(INTEGER, value as u64))
                }
                _ => None,
            }
        }
        Field::Properties => {
            if let Shape::Object { properties, .. } = fields.shape {
                properties.map(|map| Task::Map(map, true))
            } else {
                None
            }
        }
        Field::Required => {
            if let Shape::Object { required, .. } = fields.shape {
                required.map(|list| Task::List(list, false))
            } else {
                None
            }
        }
        Field::Pattern | Field::ContentEncoding | Field::ContentMediaType => {
            if let Shape::String {
                pattern,
                encoding,
                media_type,
                ..
            } = fields.shape
            {
                text(match field {
                    Field::Pattern => pattern,
                    Field::ContentEncoding => encoding,
                    _ => media_type,
                })
            } else {
                None
            }
        }
    }
}

/// Typed result owner, constructed only after build/equivalence and reseal.
pub struct Schema {
    owner: OwnedValue,
}
impl Schema {
    fn from_resealed(owner: OwnedValue) -> Self {
        let root = View {
            owner: owner.sealed_arenas(),
            node: 0,
        };
        assert!(root.schema_kind().is_some());
        Self { owner }
    }
    pub fn view(&self) -> View<'_> {
        View {
            owner: self.owner.sealed_arenas(),
            node: 0,
        }
    }
    pub fn footprint(&self) -> validated_thing_value_construction_probe::Footprint {
        self.owner.footprint()
    }
    pub fn lifetime_remaining(&self) -> u64 {
        self.owner.lifetime_remaining()
    }
    pub fn arenas_for_fixture(&self) -> Sealed<'_> {
        self.owner.sealed_arenas()
    }
}
#[derive(Clone, Copy)]
pub struct View<'a> {
    owner: Sealed<'a>,
    node: usize,
}
impl<'a> View<'a> {
    pub(crate) fn root(owner: Sealed<'a>) -> Self {
        Self { owner, node: 0 }
    }
    pub fn kind_for_fixture(self) -> u32 {
        self.record().kind
    }
    pub fn original_index(self, index: usize) -> Option<u32> {
        (index < self.len())
            .then(|| self.owner.edges()[self.record().first_edge as usize + index].original_index)
    }
    fn record(self) -> Node {
        self.owner.nodes()[self.node]
    }
    pub fn schema_kind(self) -> Option<SchemaKind> {
        [
            SchemaKind::Array,
            SchemaKind::Boolean,
            SchemaKind::Number,
            SchemaKind::Integer,
            SchemaKind::Object,
            SchemaKind::String,
            SchemaKind::Null,
        ]
        .into_iter()
        .find(|kind| self.record().kind == SCHEMA + *kind as u32)
    }
    pub fn literal_kind(self) -> Option<Kind> {
        [
            Kind::Null,
            Kind::False,
            Kind::True,
            Kind::String,
            Kind::Number,
            Kind::Array,
            Kind::Object,
            Kind::Entry,
        ]
        .into_iter()
        .find(|kind| self.record().kind == *kind as u32)
    }
    pub fn field(self, field: Field) -> Option<Self> {
        assert!(self.schema_kind().is_some());
        self.child(field as usize)
    }
    pub fn extras(self) -> Self {
        assert!(self.schema_kind().is_some());
        self.child(Field::ALL.len()).unwrap()
    }
    pub fn child(self, index: usize) -> Option<Self> {
        let node = self.record();
        if index >= node.edge_count as usize {
            return None;
        }
        let edge = self.owner.edges()[node.first_edge as usize + index];
        (edge.target != ABSENT).then_some(Self {
            owner: self.owner,
            node: edge.target as usize,
        })
    }
    pub fn len(self) -> usize {
        self.record().edge_count as usize
    }
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
    pub fn member(self, index: usize) -> Option<(&'a str, Self)> {
        if self.literal_kind() != Some(Kind::Object) {
            return None;
        }
        let entry = self.child(index)?;
        Some((entry.child(0)?.text()?, entry.child(1)?))
    }
    pub fn text(self) -> Option<&'a str> {
        if !matches!(self.literal_kind(), Some(Kind::String | Kind::Number))
            && !(typed::URI..=typed::BASE_TEMPLATE).contains(&self.record().kind)
        {
            return None;
        }
        let n = self.record();
        // SAFETY: borrowed typed text or checked literal bytes were copied one
        // byte at a time, then compared, and seal preserves initialized ranges.
        Some(unsafe {
            core::str::from_utf8_unchecked(
                &self.owner.bytes()[n.first_byte as usize..(n.first_byte + n.byte_count) as usize],
            )
        })
    }
    pub fn unsigned(self) -> Option<u32> {
        (self.record().kind == UNSIGNED).then_some(self.record().first_byte)
    }
    pub fn float(self) -> Option<f64> {
        (self.record().kind == FLOAT).then(|| f64::from_bits(self.bits()))
    }
    pub fn integer(self) -> Option<i64> {
        (self.record().kind == INTEGER).then(|| self.bits() as i64)
    }
    fn bits(self) -> u64 {
        u64::from(self.record().first_byte) | (u64::from(self.record().byte_count) << 32)
    }
}
const _: () = assert!(!mem::needs_drop::<Frame<'static>>());

/// Synchronous query adapter for the same Basic rule source. It is a post-seal
/// semantic oracle, not an additional charged constructor phase.
pub struct Access;
impl<'a> super::schema_kernel::SchemaAccess<'a> for Access {
    type Node = View<'a>;
    type Number = &'a str;
    fn kind(&self, node: View<'a>) -> SchemaKind {
        node.schema_kind().unwrap()
    }
    fn data_type(&self, node: View<'a>) -> Option<&'a str> {
        node.field(Field::Type)?.text()
    }
    fn flags(&self, node: View<'a>) -> (bool, bool) {
        (
            node.field(Field::ReadOnly).unwrap().literal_kind() == Some(Kind::True),
            node.field(Field::WriteOnly).unwrap().literal_kind() == Some(Kind::True),
        )
    }
    fn one_of_count(&self, node: View<'a>) -> usize {
        node.field(Field::OneOf).map_or(0, View::len)
    }
    fn one_of(&self, node: View<'a>, index: usize) -> View<'a> {
        node.field(Field::OneOf).unwrap().child(index).unwrap()
    }
    fn child_count(&self, node: View<'a>) -> usize {
        match node.schema_kind().unwrap() {
            SchemaKind::Array => node.field(Field::Items).map_or(0, View::len),
            SchemaKind::Object => node.field(Field::Properties).map_or(0, View::len),
            _ => 0,
        }
    }
    fn child(
        &self,
        node: View<'a>,
        index: usize,
    ) -> (super::schema_kernel::ChildSite<'a>, View<'a>) {
        match node.schema_kind().unwrap() {
            SchemaKind::Array => (
                super::schema_kernel::ChildSite::Indexed(index),
                node.field(Field::Items).unwrap().child(index).unwrap(),
            ),
            SchemaKind::Object => {
                let (key, child) = node
                    .field(Field::Properties)
                    .unwrap()
                    .member(index)
                    .unwrap();
                (super::schema_kernel::ChildSite::Property(key), child)
            }
            _ => unreachable!(),
        }
    }
    fn unsigned_extension(
        &self,
        node: View<'a>,
        field: super::schema_kernel::Field,
    ) -> Option<u64> {
        extension(node, field)?.parse().ok()
    }
    fn number_extension(
        &self,
        node: View<'a>,
        field: super::schema_kernel::Field,
    ) -> Option<&'a str> {
        extension(node, field)
    }
    fn project_number(&self, text: &'a str) -> Option<f64> {
        text.parse().ok()
    }
    fn typed_unsigned(&self, node: View<'a>) -> (Option<u32>, Option<u32>) {
        let (min, max) = match node.schema_kind().unwrap() {
            SchemaKind::Array => (Field::MinItems, Field::MaxItems),
            SchemaKind::String => (Field::MinLength, Field::MaxLength),
            _ => return (None, None),
        };
        (
            node.field(min).and_then(View::unsigned),
            node.field(max).and_then(View::unsigned),
        )
    }
    fn typed_float(&self, node: View<'a>) -> [Option<f64>; 5] {
        [
            Field::Minimum,
            Field::ExclusiveMinimum,
            Field::Maximum,
            Field::ExclusiveMaximum,
            Field::MultipleOf,
        ]
        .map(|field| node.field(field).and_then(View::float))
    }
    fn typed_integer(&self, node: View<'a>) -> [Option<i64>; 5] {
        [
            Field::Minimum,
            Field::ExclusiveMinimum,
            Field::Maximum,
            Field::ExclusiveMaximum,
            Field::MultipleOf,
        ]
        .map(|field| node.field(field).and_then(View::integer))
    }
}
fn extension(node: View<'_>, field: super::schema_kernel::Field) -> Option<&str> {
    let extras = node.extras();
    (0..extras.len()).find_map(|index| {
        let (key, value) = extras.member(index)?;
        (key == field.name() && value.literal_kind() == Some(Kind::Number))
            .then(|| value.text().unwrap())
    })
}
