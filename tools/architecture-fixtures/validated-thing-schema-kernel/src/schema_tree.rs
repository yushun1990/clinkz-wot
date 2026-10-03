//! Non-production composition: full nested DataSchema decoding, then Basic.
//! No recursive calls, owning TD graph, synchronous arena Access, or raw lookup.
use super::{
    schema_arena::DecodeError,
    schema_fields::{self as policy, Shape},
    schema_kernel::{
        self as rules, Action, NumericCursor, Rule, SchemaKind, Walk,
        projection_step::{self, ProjectionProgress},
    },
    schema_step::{self as fields, Fields, Machine, Outcome},
};
use clinkz_wot_foundation::{WorkBudget, WorkClass as W};
use core::mem;
use validated_thing_value_construction_probe::{FrameResources, Frames, Kind, OwnedValue, View};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pass {
    Decode,
    Basic,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Enter(Pass),
    Projection(Pass),
    Frames(Pass),
    Walk(Pass),
    Type,
    Unsigned,
    Numeric,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Trace {
    /// DocumentNodes, CodecInputBytes, JsonSchemaNodes, CleanupItems.
    pub work: [u64; 4],
    pub field_work: [u64; 3],
    pub nodes: [u64; 2],
    pub field_parses: [u64; 3],
    pub unsigned_parses: u64,
    pub numeric_parses: u64,
    pub frame_requests: u64,
    pub frame_copies: u64,
    pub maximum_depth: usize,
    pub resources: Option<FrameResources>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cause {
    Field { ordinal: u64, error: DecodeError },
    Basic(rules::InlineInvalid),
    Depth { observed: usize, ceiling: usize },
    Number { observed: usize, ceiling: usize },
    Lifetime,
    Memory,
    Arithmetic,
    Allocation,
    Cancelled,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Failure {
    pub cause: Cause,
    pub phase: Phase,
    pub trace: Trace,
}

struct Frame<'a> {
    fields: Fields<'a>,
    ordinal: u64,
    nested: usize,
    walk: Walk,
}
impl<'a> Frame<'a> {
    fn one_of_len(&self) -> usize {
        self.fields.context.one_of.map_or(0, |list| list.len())
    }
    fn child_len(&self) -> usize {
        match &self.fields.shape {
            Shape::Array { items, .. } => items.map_or(0, |list| list.len()),
            Shape::Object { properties, .. } => properties.map_or(0, |map| map.len()),
            _ => 0,
        }
    }
    fn one_of(&self, index: usize) -> View<'a> {
        self.fields.context.one_of.unwrap().get(index).unwrap()
    }
    fn child(&self, index: usize) -> View<'a> {
        match &self.fields.shape {
            Shape::Array { items, .. } => items.unwrap().get(index).unwrap(),
            Shape::Object { properties, .. } => properties.unwrap().member(index).unwrap().1,
            _ => unreachable!(),
        }
    }
    fn kind(&self) -> SchemaKind {
        match self.fields.shape {
            Shape::Array { .. } => SchemaKind::Array,
            Shape::Boolean => SchemaKind::Boolean,
            Shape::Number(_) => SchemaKind::Number,
            Shape::Integer(_) => SchemaKind::Integer,
            Shape::Object { .. } => SchemaKind::Object,
            Shape::String { .. } => SchemaKind::String,
            Shape::Null => SchemaKind::Null,
        }
    }
    fn number(&self, field: rules::Field) -> Option<&'a str> {
        let value = self.fields.context.extras.basic_field(field)?;
        (value.kind() == Kind::Number).then(|| value.text().unwrap())
    }
}
// Keep the one-node machine inline; boxing it would add an allocation site.
#[allow(clippy::large_enum_variant)]
enum State<'a> {
    Enter(View<'a>),
    Project {
        machine: Machine<'a>,
        ordinal: u64,
    },
    Push(Frame<'a>),
    Walk,
    Type {
        text: &'a str,
        position: usize,
        bytes: [u8; policy::DISPATCH_BYTES_MAX],
    },
    Unsigned {
        position: usize,
        values: [Option<u64>; 4],
    },
    UnsignedNumber {
        position: usize,
        values: [Option<u64>; 4],
        text: &'a str,
    },
    Numeric {
        cursor: NumericCursor,
        numbers: [Option<&'a str>; 5],
    },
    Moving,
}
pub struct Cursor<'a> {
    root: View<'a>,
    lifetime: &'a mut u64,
    frames: Frames<'a, Frame<'a>>,
    number_ceiling: usize,
    depth_ceiling: usize,
    pass: Pass,
    next_ordinal: u64,
    state: State<'a>,
    trace: Trace,
}
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
    /// Local fixture controls, not full admission configuration. The source
    /// owner lends its actual immutable arenas, lifetime and Accounting together.
    pub fn new(owner: &'a mut OwnedValue, number_ceiling: usize, depth_ceiling: usize) -> Self {
        let (root, lifetime, frames) = owner.inspection_parts();
        Self {
            root,
            lifetime,
            frames,
            number_ceiling,
            depth_ceiling,
            pass: Pass::Decode,
            next_ordinal: 0,
            state: State::Enter(root),
            trace: Trace::default(),
        }
    }
    pub fn trace(&self) -> Trace {
        Trace {
            resources: Some(self.frames.resources()),
            ..self.trace
        }
    }
    pub fn lifetime_remaining(&self) -> u64 {
        *self.lifetime
    }
    pub fn phase(&self) -> Phase {
        match self.state {
            State::Enter(_) => Phase::Enter(self.pass),
            State::Project { .. } => Phase::Projection(self.pass),
            State::Push(_) => Phase::Frames(self.pass),
            State::Walk => Phase::Walk(self.pass),
            State::Type { .. } => Phase::Type,
            State::Unsigned { .. } | State::UnsignedNumber { .. } => Phase::Unsigned,
            State::Numeric { .. } => Phase::Numeric,
            State::Moving => unreachable!(),
        }
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
        // No cancellation check or fallible work after fixing the first cause.
        self.frames.clear();
        Progress::Failed(Failure {
            cause,
            phase,
            trace: self.trace(),
        })
    }
    fn pay(&mut self, budget: &mut WorkBudget, classes: &[W]) -> Result<bool, Cause> {
        if classes.iter().any(|&class| budget.remaining(class) == 0) {
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
                W::CleanupItems => 3,
                _ => unreachable!(),
            }] += 1;
        }
        *self.lifetime -= classes.len() as u64;
        Ok(true)
    }
    fn rule(&self, result: Result<(), Rule>) -> Result<(), Cause> {
        result.map_err(|rule| {
            Cause::Basic(rules::InlineInvalid {
                ordinal: self.frames.last().ordinal,
                rule,
            })
        })
    }
    fn advance(&mut self) {
        self.frames.last_mut().walk.advance();
        self.state = State::Walk;
    }
    fn checked_number(&self, text: &str) -> Result<(), Cause> {
        if text.len() > self.number_ceiling {
            Err(Cause::Number {
                observed: text.len(),
                ceiling: self.number_ceiling,
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
        match state {
            State::Enter(node) => {
                if !self.pay(budget, &[W::DocumentNodes])? {
                    self.state = State::Enter(node);
                    return Ok(Tick::Blocked);
                }
                let depth = self.frames.len().checked_add(1).ok_or(Cause::Arithmetic)?;
                if depth > self.depth_ceiling {
                    return Err(Cause::Depth {
                        observed: depth,
                        ceiling: self.depth_ceiling,
                    });
                }
                let ordinal = self.next_ordinal;
                self.next_ordinal = ordinal.checked_add(1).ok_or(Cause::Arithmetic)?;
                self.trace.nodes[usize::from(self.pass == Pass::Basic)] += 1;
                self.trace.maximum_depth = self.trace.maximum_depth.max(depth);
                self.state = State::Project {
                    machine: Machine::new(node, self.number_ceiling),
                    ordinal,
                };
            }
            State::Project { machine, ordinal } => {
                let before = machine.trace();
                let progress = machine.step(budget, self.lifetime, cancelled);
                let after = match &progress {
                    Outcome::Pending(machine) => machine.trace(),
                    Outcome::Complete { trace, .. } => *trace,
                    Outcome::Failed(failure) => failure.trace,
                };
                for i in 0..3 {
                    self.trace.work[i] += after.work[i] - before.work[i];
                    self.trace.field_work[i] += after.work[i] - before.work[i];
                    self.trace.field_parses[i] += after.parses[i] - before.parses[i];
                }
                match progress {
                    Outcome::Pending(machine) => {
                        self.state = State::Project { machine, ordinal };
                        return Ok(Tick::Blocked);
                    }
                    Outcome::Complete { fields, .. } => {
                        self.state = State::Push(Frame {
                            fields,
                            ordinal,
                            nested: 0,
                            walk: Walk::default(),
                        });
                    }
                    Outcome::Failed(failure) => {
                        return Err(match failure.cause {
                            fields::Cause::Field(error) => Cause::Field { ordinal, error },
                            fields::Cause::NumberLimit { observed, ceiling } => {
                                Cause::Number { observed, ceiling }
                            }
                            fields::Cause::Lifetime => Cause::Lifetime,
                            fields::Cause::Cancelled => Cause::Cancelled,
                        });
                    }
                }
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
                    if !self.pay(budget, &[W::CleanupItems])? {
                        self.state = State::Push(frame);
                        return Ok(Tick::Blocked);
                    }
                    let capacity = if self.frames.capacity() == 0 {
                        1
                    } else {
                        self.frames
                            .capacity()
                            .checked_mul(2)
                            .ok_or(Cause::Arithmetic)?
                            .min(self.depth_ceiling)
                    };
                    self.frames
                        .begin_grow(capacity)
                        .map_err(|error| match error {
                            validated_thing_value_construction_probe::FrameError::Arithmetic => {
                                Cause::Arithmetic
                            }
                            validated_thing_value_construction_probe::FrameError::Limit => {
                                Cause::Memory
                            }
                            validated_thing_value_construction_probe::FrameError::Allocation => {
                                Cause::Allocation
                            }
                        })?;
                    self.trace.frame_requests += 1;
                    self.state = State::Push(frame);
                } else {
                    if !self.pay(budget, &[W::DocumentNodes])? {
                        self.state = State::Push(frame);
                        return Ok(Tick::Blocked);
                    }
                    self.frames.push(frame);
                    self.state = State::Walk;
                }
            }
            State::Walk => {
                if !self.pay(budget, &[W::DocumentNodes])? {
                    self.state = State::Walk;
                    return Ok(Tick::Blocked);
                }
                if self.pass == Pass::Decode {
                    let frame = self.frames.last_mut();
                    let one_of = frame.one_of_len();
                    let next = frame.nested;
                    let child = if next < one_of {
                        Some(frame.one_of(next))
                    } else {
                        let next = next - one_of;
                        (next < frame.child_len()).then(|| frame.child(next))
                    };
                    if let Some(node) = child {
                        frame.nested += 1;
                        self.state = State::Enter(node);
                    } else {
                        self.frames.pop();
                        if self.frames.is_empty() {
                            self.pass = Pass::Basic;
                            self.next_ordinal = 0;
                            self.state = State::Enter(self.root);
                        } else {
                            self.state = State::Walk;
                        }
                    }
                } else {
                    let frame = self.frames.last_mut();
                    let (one_of, children) = (frame.one_of_len(), frame.child_len());
                    match frame.walk.action(one_of, children) {
                        Action::Type => {
                            let text = frame.fields.context.data_type;
                            let kind = frame.kind();
                            if let Some(text) = text.filter(|text| text.len() == kind.name().len())
                            {
                                self.state = State::Type {
                                    text,
                                    position: 0,
                                    bytes: [0; policy::DISPATCH_BYTES_MAX],
                                };
                            } else {
                                self.rule(rules::check_type(text, kind))?;
                                self.advance();
                            }
                        }
                        Action::OneOf(index) | Action::Child(index) => {
                            let action = frame.walk.action(one_of, children);
                            let node = if matches!(action, Action::OneOf(_)) {
                                frame.one_of(index)
                            } else {
                                frame.child(index)
                            };
                            frame.walk.advance();
                            self.state = State::Enter(node);
                        }
                        Action::Flags => {
                            let context = &frame.fields.context;
                            let result =
                                rules::check_flags((context.read_only, context.write_only));
                            self.rule(result)?;
                            self.advance();
                        }
                        Action::Unsigned => {
                            self.state = State::Unsigned {
                                position: 0,
                                values: [None; 4],
                            }
                        }
                        Action::Numeric => {
                            let numbers = rules::NUMERIC_FIELDS.map(|field| frame.number(field));
                            self.state = State::Numeric {
                                cursor: NumericCursor::default(),
                                numbers,
                            };
                        }
                        Action::Typed => {
                            let result = match &frame.fields.shape {
                                Shape::Array { min, max, .. } | Shape::String { min, max, .. } => {
                                    rules::check_typed_unsigned(frame.kind(), *min, *max)
                                }
                                Shape::Number(values) => rules::check_typed_numeric(values),
                                Shape::Integer(values) => rules::check_typed_numeric(values),
                                _ => Ok(()),
                            };
                            self.rule(result)?;
                            self.advance();
                        }
                        Action::Done => {
                            self.frames.pop();
                            if self.frames.is_empty() {
                                return Ok(Tick::Complete);
                            }
                            self.state = State::Walk;
                        }
                    }
                }
            }
            State::Type {
                text,
                mut position,
                mut bytes,
            } => {
                if position == text.len() {
                    if !self.pay(budget, &[W::DocumentNodes])? {
                        self.state = State::Type {
                            text,
                            position,
                            bytes,
                        };
                        return Ok(Tick::Blocked);
                    }
                    self.rule(rules::check_type(
                        Some(core::str::from_utf8(&bytes[..position]).unwrap()),
                        self.frames.last().kind(),
                    ))?;
                    self.advance();
                } else {
                    if !self.pay(budget, &[W::CodecInputBytes])? {
                        self.state = State::Type {
                            text,
                            position,
                            bytes,
                        };
                        return Ok(Tick::Blocked);
                    }
                    bytes[position] = text.as_bytes()[position];
                    position += 1;
                    self.state = State::Type {
                        text,
                        position,
                        bytes,
                    };
                }
            }
            State::Unsigned {
                mut position,
                values,
            } => {
                if !self.pay(budget, &[W::DocumentNodes])? {
                    self.state = State::Unsigned { position, values };
                    return Ok(Tick::Blocked);
                }
                if position == 2 {
                    // The first pair's rejection precedes any later scalar
                    // projection, through the same shared pair predicate.
                    self.rule(rules::check_unsigned_pair(0, [values[0], values[1]]))?;
                }
                if position == 4 {
                    self.rule(rules::check_unsigned_pair(1, [values[2], values[3]]))?;
                    self.advance();
                } else if let Some(text) = self.frames.last().number(rules::FIELDS[position]) {
                    self.checked_number(text)?;
                    self.state = State::UnsignedNumber {
                        position,
                        values,
                        text,
                    };
                } else {
                    position += 1;
                    self.state = State::Unsigned { position, values };
                }
            }
            State::UnsignedNumber {
                mut position,
                mut values,
                text,
            } => {
                let before = *self.lifetime;
                let parses = &mut self.trace.unsigned_parses;
                let progress = projection_step::project(
                    text,
                    self.number_ceiling,
                    budget,
                    self.lifetime,
                    cancelled,
                    || {
                        *parses += 1;
                        text.parse::<u64>().ok()
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
                        position += 1;
                        self.state = State::Unsigned { position, values };
                    }
                }
            }
            State::Numeric {
                mut cursor,
                numbers,
            } => {
                let before = *self.lifetime;
                let ceiling = self.number_ceiling;
                let parses = &mut self.trace.numeric_parses;
                let mut lexical = None;
                let progress = cursor.step(numbers, |text| {
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
                        self.rule(result)?;
                        self.advance();
                    }
                }
            }
            State::Moving => unreachable!(),
        }
        Ok(Tick::Advanced)
    }
}
const _: () = assert!(!mem::needs_drop::<Frame<'static>>());
