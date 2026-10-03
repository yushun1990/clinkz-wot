//! Charged, nonrecursive projection of ONE literal DataSchema node. Nested
//! schemas are borrowed handles, not traversed here. Basic and whole admission
//! remain outside this cursor. All field decisions come from schema_fields.
use super::{
    schema_arena::{DecodeError, Extras, List, Primitive},
    schema_fields::{self as policy, Context, Decoded, Field, Metadata},
    schema_kernel::{
        SchemaKind,
        projection_step::{self, ProjectionProgress},
    },
};
use clinkz_wot_foundation::{WorkBudget, WorkClass as W};
use serde::Deserialize;
use validated_thing_value_construction_probe::{Kind, View};

const FIELDS: usize = Field::ALL.len();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Operation {
    Literal,
    Text,
    Values,
    Strings,
    StringsMany,
    Languages,
    Schemas,
    SchemasMany,
    SchemaMap,
    Boolean,
    Unsigned,
    Float,
    Integer,
}
#[derive(Clone, Copy)]
enum Fact<'a> {
    View(View<'a>),
    Text(&'a str),
    List(List<'a>),
    Boolean(bool),
    Unsigned(u32),
    Float(f64),
    Integer(i64),
}
#[derive(Clone, Copy)]
struct Cached<'a> {
    operation: Operation,
    fact: Fact<'a>,
}
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct Request<'a> {
    field: Field,
    operation: Operation,
    value: View<'a>,
}

/// Fixed index and completed conversion facts. No source lookup, scalar parse
/// or list scan occurs inside this policy adapter. An unpaid conversion yields
/// a request, not a TD field error. Only the charged cursor can service it.
pub struct Source<'a> {
    object: View<'a>,
    index: [Option<View<'a>>; FIELDS],
    cache: [Option<Cached<'a>>; FIELDS],
    consumed: u64,
    active: Option<Field>,
}
impl<'a> Source<'a> {
    fn request(&self, operation: Operation, value: View<'a>) -> Result<Fact<'a>, Request<'a>> {
        let field = self.active.unwrap();
        if let Some(cached) = self.cache[field as usize] {
            assert_eq!(
                cached.operation, operation,
                "immutable field policy changed conversion"
            );
            Ok(cached.fact)
        } else {
            Err(Request {
                field,
                operation,
                value,
            })
        }
    }
}
macro_rules! fact_reader {
    ($name:ident, $ty:ty, $op:ident, $fact:ident) => {
        fn $name(&mut self, value: View<'a>) -> Result<$ty, Self::Error> {
            let Fact::$fact(value) = self.request(Operation::$op, value)? else {
                unreachable!()
            };
            Ok(value)
        }
    };
}
impl<'a> policy::Source for Source<'a> {
    type Value = View<'a>;
    type Text = &'a str;
    type Values = View<'a>;
    type Strings = List<'a>;
    type Languages = View<'a>;
    type Schemas = List<'a>;
    type SchemaMap = View<'a>;
    type Extras = Extras<'a>;
    type Error = Request<'a>;
    fn take(&mut self, field: Field) -> Option<View<'a>> {
        self.active = Some(field);
        self.consumed |= 1 << field as u8;
        self.index[field as usize]
    }
    fn is_null(value: &View<'a>) -> bool {
        value.kind() == Kind::Null
    }
    fact_reader!(literal, View<'a>, Literal, View);
    fact_reader!(text, &'a str, Text, Text);
    fact_reader!(values, View<'a>, Values, View);
    fact_reader!(strings, List<'a>, Strings, List);
    fact_reader!(strings_many, List<'a>, StringsMany, List);
    fact_reader!(languages, View<'a>, Languages, View);
    fact_reader!(schemas, List<'a>, Schemas, List);
    fact_reader!(schemas_many, List<'a>, SchemasMany, List);
    fact_reader!(schema_map, View<'a>, SchemaMap, View);
    fact_reader!(boolean, bool, Boolean, Boolean);
    fact_reader!(unsigned, u32, Unsigned, Unsigned);
    fact_reader!(float, f64, Float, Float);
    fact_reader!(integer, i64, Integer, Integer);
    fn metadata(&mut self) -> Result<Metadata<Self>, Self::Error> {
        policy::metadata(self)
    }
    fn remaining_context(self) -> Result<Context<Self>, Self::Error> {
        policy::context(self)
    }
    fn extras(self) -> Extras<'a> {
        Extras::indexed(self.object, self.consumed, &self.index)
    }
}
pub type Fields<'a> = Decoded<Source<'a>>;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Trace {
    /// DocumentNodes, CodecInputBytes, JsonSchemaNodes respectively.
    pub work: [u64; 3],
    pub members: u64,
    pub key_bytes: u64,
    pub selector_bytes: u64,
    pub boolean_bytes: u64,
    pub list_elements: u64,
    pub policy_runs: u64,
    /// Each actual i64, u64, f64 parse, including failed attempts.
    pub parses: [u64; 3],
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cause {
    Field(DecodeError),
    NumberLimit { observed: usize, ceiling: usize },
    Lifetime,
    Cancelled,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Failure {
    pub cause: Cause,
    pub trace: Trace,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Index,
    Dispatch,
    Policy,
    Conversion,
    Number,
}

#[derive(Clone, Copy)]
enum Parse {
    Signed,
    Unsigned,
    Float,
}
#[derive(Clone, Copy)]
enum State<'a> {
    Start,
    Member(usize),
    Key {
        member: usize,
        name: &'a str,
        value: View<'a>,
        candidate: usize,
        position: usize,
    },
    Dispatch,
    Selector {
        text: &'a str,
        position: usize,
        bytes: [u8; policy::DISPATCH_BYTES_MAX],
    },
    Policy,
    Convert(Request<'a>),
    List {
        request: Request<'a>,
        list: List<'a>,
        position: usize,
        languages: bool,
    },
    BooleanText {
        request: Request<'a>,
        text: &'a str,
        position: usize,
        bytes: [u8; 5],
    },
    Number {
        request: Request<'a>,
        text: &'a str,
        parse: Parse,
    },
}

/// Borrowing facade retained for the original one-node witness.
pub struct Cursor<'a, 'w> {
    machine: Machine<'a>,
    lifetime: &'w mut u64,
}
/// Resumable field state without an embedded mutable work borrow. This permits
/// a containing traversal to lend its ONE lifetime remainder on each step.
pub struct Machine<'a> {
    object: View<'a>,
    ceiling: usize,
    index: [Option<View<'a>>; FIELDS],
    cache: [Option<Cached<'a>>; FIELDS],
    kind: SchemaKind,
    state: State<'a>,
    trace: Trace,
}
// Boxing would add an unauthorized allocation. Capacity is measured separately.
#[allow(clippy::large_enum_variant)]
pub enum Outcome<'a, C> {
    Pending(C),
    Complete { fields: Fields<'a>, trace: Trace },
    Failed(Failure),
}
pub type Progress<'a, 'w> = Outcome<'a, Cursor<'a, 'w>>;
pub type MachineProgress<'a> = Outcome<'a, Machine<'a>>;
impl<'a, 'w> Cursor<'a, 'w> {
    pub fn new(object: View<'a>, lifetime: &'w mut u64, number_ceiling: usize) -> Self {
        Self {
            machine: Machine::new(object, number_ceiling),
            lifetime,
        }
    }
    pub fn trace(&self) -> Trace {
        self.machine.trace()
    }
    pub fn lifetime_remaining(&self) -> u64 {
        *self.lifetime
    }
    pub fn phase(&self) -> Phase {
        self.machine.phase()
    }
    pub fn step(
        self,
        budget: &mut WorkBudget,
        cancelled: impl FnMut() -> bool,
    ) -> Progress<'a, 'w> {
        match self.machine.step(budget, self.lifetime, cancelled) {
            Outcome::Pending(machine) => Outcome::Pending(Self {
                machine,
                lifetime: self.lifetime,
            }),
            Outcome::Complete { fields, trace } => Outcome::Complete { fields, trace },
            Outcome::Failed(failure) => Outcome::Failed(failure),
        }
    }
}
impl<'a> Machine<'a> {
    /// Fixed work; these local controls remain fixture scaffolding, not the
    /// opaque full admission configuration or a supported maximum M.
    pub fn new(object: View<'a>, number_ceiling: usize) -> Self {
        Self {
            object,
            ceiling: number_ceiling,
            index: [None; FIELDS],
            cache: [None; FIELDS],
            kind: SchemaKind::Object,
            state: State::Start,
            trace: Trace::default(),
        }
    }
    pub fn trace(&self) -> Trace {
        self.trace
    }
    pub fn phase(&self) -> Phase {
        match self.state {
            State::Start | State::Member(_) | State::Key { .. } => Phase::Index,
            State::Dispatch | State::Selector { .. } => Phase::Dispatch,
            State::Policy => Phase::Policy,
            State::Number { .. } => Phase::Number,
            _ => Phase::Conversion,
        }
    }
    /// The hook instruments cancellation on both sides of an atomic parse.
    /// It is fixture scaffolding, not a proposed replacement for the frozen
    /// production step's `cancel_requested: bool` or a user callback boundary.
    pub fn step(
        mut self,
        budget: &mut WorkBudget,
        lifetime: &mut u64,
        mut cancelled: impl FnMut() -> bool,
    ) -> MachineProgress<'a> {
        loop {
            if cancelled() {
                return self.fail(Cause::Cancelled);
            }
            match self.tick(budget, lifetime, &mut cancelled) {
                Ok(Tick::Advanced) => {}
                Ok(Tick::Blocked) => return Outcome::Pending(self),
                Ok(Tick::Complete(fields)) => {
                    return Outcome::Complete {
                        fields,
                        trace: self.trace,
                    };
                }
                Err(cause) => return self.fail(cause),
            }
        }
    }
    fn fail(self, cause: Cause) -> MachineProgress<'a> {
        // This pass borrows an already owned value. It owns no allocation or
        // cleanup charge. The outer owner, not this local failure, owns rollback.
        Outcome::Failed(Failure {
            cause,
            trace: self.trace,
        })
    }
    fn pay(
        &mut self,
        budget: &mut WorkBudget,
        lifetime: &mut u64,
        classes: &[W],
    ) -> Result<bool, Cause> {
        if classes.iter().any(|&class| budget.remaining(class) == 0) {
            return Ok(false);
        }
        if *lifetime < classes.len() as u64 {
            return Err(Cause::Lifetime);
        }
        for &class in classes {
            budget.consume(class, 1).unwrap();
            self.trace.work[match class {
                W::DocumentNodes => 0,
                W::CodecInputBytes => 1,
                W::JsonSchemaNodes => 2,
                _ => unreachable!(),
            }] += 1;
        }
        *lifetime -= classes.len() as u64;
        Ok(true)
    }
    fn cache(&mut self, request: Request<'a>, fact: Fact<'a>) {
        self.cache[request.field as usize] = Some(Cached {
            operation: request.operation,
            fact,
        });
        self.state = State::Policy;
    }
    fn field_error(request: Request<'a>) -> Cause {
        Cause::Field(DecodeError {
            field: Some(request.field),
        })
    }
    fn scalar(&mut self, request: Request<'a>, primitive: Primitive<'_>) -> Result<(), Cause> {
        // The exact same public serde and TD flexible-bool visitors as #120.
        // This never reparses a Number or allocates an error string.
        let fact = match request.operation {
            Operation::Boolean => {
                crate::components::util::deserialize_bool_flexible(primitive).map(Fact::Boolean)
            }
            Operation::Unsigned => u32::deserialize(primitive).map(Fact::Unsigned),
            Operation::Integer => i64::deserialize(primitive).map(Fact::Integer),
            Operation::Float => f64::deserialize(primitive).map(Fact::Float),
            _ => unreachable!(),
        }
        .map_err(|_| Self::field_error(request))?;
        self.cache(request, fact);
        Ok(())
    }
    fn tick(
        &mut self,
        budget: &mut WorkBudget,
        lifetime: &mut u64,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<Tick<'a>, Cause> {
        match self.state {
            State::Start => {
                if !self.pay(budget, lifetime, &[W::DocumentNodes, W::JsonSchemaNodes])? {
                    return Ok(Tick::Blocked);
                }
                if self.object.kind() != Kind::Object {
                    return Err(Cause::Field(DecodeError { field: None }));
                }
                self.state = State::Member(0);
            }
            State::Member(member) => {
                if !self.pay(budget, lifetime, &[W::DocumentNodes])? {
                    return Ok(Tick::Blocked);
                }
                if member == self.object.len() {
                    self.state = State::Dispatch;
                } else {
                    let (name, value) = self.object.member(member).unwrap();
                    self.trace.members += 1;
                    self.state = State::Key {
                        member,
                        name,
                        value,
                        candidate: 0,
                        position: 0,
                    };
                }
            }
            State::Key {
                member,
                name,
                value,
                candidate,
                position,
            } => {
                // Length rejection reads no key byte. For equal lengths, each
                // compared source byte is a separate charged/resumable unit.
                let expected = Field::ALL.get(candidate).map(|field| field.name());
                let reads_byte =
                    expected.is_some_and(|text| text.len() == name.len() && position < name.len());
                let classes = if reads_byte {
                    &[W::DocumentNodes, W::CodecInputBytes][..]
                } else {
                    &[W::DocumentNodes][..]
                };
                if !self.pay(budget, lifetime, classes)? {
                    return Ok(Tick::Blocked);
                }
                match expected {
                    None => self.state = State::Member(member + 1),
                    Some(text) if text.len() != name.len() => {
                        self.state = State::Key {
                            member,
                            name,
                            value,
                            candidate: candidate + 1,
                            position: 0,
                        }
                    }
                    Some(_) if position == name.len() => {
                        self.index[Field::ALL[candidate] as usize] = Some(value);
                        self.state = State::Member(member + 1);
                    }
                    Some(text) => {
                        self.trace.key_bytes += 1;
                        self.state = if text.as_bytes()[position] == name.as_bytes()[position] {
                            State::Key {
                                member,
                                name,
                                value,
                                candidate,
                                position: position + 1,
                            }
                        } else {
                            State::Key {
                                member,
                                name,
                                value,
                                candidate: candidate + 1,
                                position: 0,
                            }
                        };
                    }
                }
            }
            State::Dispatch => {
                if !self.pay(budget, lifetime, &[W::DocumentNodes])? {
                    return Ok(Tick::Blocked);
                }
                match self.index[Field::Type as usize] {
                    None => {
                        self.kind = policy::dispatch(None);
                        self.state = State::Policy;
                    }
                    Some(value) if value.kind() == Kind::Null => {
                        self.kind = policy::dispatch(None);
                        self.state = State::Policy;
                    }
                    Some(value) if value.kind() == Kind::String => {
                        let text = value.text().unwrap();
                        self.cache[Field::Type as usize] = Some(Cached {
                            operation: Operation::Text,
                            fact: Fact::Text(text),
                        });
                        if text.len() > policy::DISPATCH_BYTES_MAX {
                            // No recognized discriminator has this length.
                            // Original content is still retained as data_type.
                            self.kind = policy::dispatch(None);
                            self.state = State::Policy;
                        } else {
                            self.state = State::Selector {
                                text,
                                position: 0,
                                bytes: [0; policy::DISPATCH_BYTES_MAX],
                            };
                        }
                    }
                    Some(_) => {
                        return Err(Cause::Field(DecodeError {
                            field: Some(Field::Type),
                        }));
                    }
                }
            }
            State::Selector {
                text,
                mut position,
                mut bytes,
            } => {
                if position == text.len() {
                    if !self.pay(budget, lifetime, &[W::DocumentNodes])? {
                        return Ok(Tick::Blocked);
                    }
                    self.kind =
                        policy::dispatch(Some(core::str::from_utf8(&bytes[..position]).unwrap()));
                    self.state = State::Policy;
                } else {
                    if !self.pay(budget, lifetime, &[W::CodecInputBytes])? {
                        return Ok(Tick::Blocked);
                    }
                    bytes[position] = text.as_bytes()[position];
                    position += 1;
                    self.trace.selector_bytes += 1;
                    self.state = State::Selector {
                        text,
                        position,
                        bytes,
                    };
                }
            }
            State::Policy => {
                if !self.pay(budget, lifetime, &[W::DocumentNodes])? {
                    return Ok(Tick::Blocked);
                }
                self.trace.policy_runs += 1;
                // Replay is bounded by the FIXED 29-field policy, not document
                // width/depth. Cached conversions are constant-time facts.
                // Every replay pays a structural unit; no scans run on replay.
                let source = Source {
                    object: self.object,
                    index: self.index,
                    cache: self.cache,
                    consumed: 1 << Field::Type as u8,
                    active: None,
                };
                match policy::variant(source, self.kind) {
                    Ok(fields) => return Ok(Tick::Complete(fields)),
                    Err(request) => self.state = State::Convert(request),
                }
            }
            State::Convert(request) => {
                if !self.pay(budget, lifetime, &[W::DocumentNodes])? {
                    return Ok(Tick::Blocked);
                }
                let value = request.value;
                let kind = value.kind();
                match request.operation {
                    Operation::Literal => self.cache(request, Fact::View(value)),
                    Operation::Text if kind == Kind::String => {
                        self.cache(request, Fact::Text(value.text().unwrap()))
                    }
                    Operation::Values if kind == Kind::Array => {
                        self.cache(request, Fact::View(value))
                    }
                    Operation::SchemaMap if kind == Kind::Object => {
                        self.cache(request, Fact::View(value))
                    }
                    Operation::Languages if kind == Kind::Object => {
                        self.state = State::List {
                            request,
                            list: List {
                                value,
                                single: false,
                            },
                            position: 0,
                            languages: true,
                        }
                    }
                    Operation::Strings
                    | Operation::StringsMany
                    | Operation::Schemas
                    | Operation::SchemasMany => {
                        let many = matches!(
                            request.operation,
                            Operation::StringsMany | Operation::SchemasMany
                        );
                        if kind != Kind::Array && !many {
                            return Err(Self::field_error(request));
                        }
                        let list = List {
                            value,
                            single: kind != Kind::Array,
                        };
                        if matches!(
                            request.operation,
                            Operation::Strings | Operation::StringsMany
                        ) {
                            self.state = State::List {
                                request,
                                list,
                                position: 0,
                                languages: false,
                            };
                        } else {
                            self.cache(request, Fact::List(list));
                        }
                    }
                    Operation::Boolean
                    | Operation::Unsigned
                    | Operation::Integer
                    | Operation::Float => match kind {
                        Kind::Number => {
                            self.state = State::Number {
                                request,
                                text: value.text().unwrap(),
                                parse: if request.operation == Operation::Float {
                                    Parse::Float
                                } else {
                                    Parse::Signed
                                },
                            }
                        }
                        Kind::String if request.operation == Operation::Boolean => {
                            let text = value.text().unwrap();
                            if text.len() > 5 {
                                // The existing visitor compares only its four
                                // fixed spellings (maximum five bytes). Length
                                // mismatch reads no content; the visitor still
                                // decides rejection, not this fast path.
                                self.scalar(request, Primitive::Text(text))?;
                            } else {
                                self.state = State::BooleanText {
                                    request,
                                    text,
                                    position: 0,
                                    bytes: [0; 5],
                                };
                            }
                        }
                        Kind::String => {
                            self.scalar(request, Primitive::Text(value.text().unwrap()))?
                        }
                        Kind::Null => self.scalar(request, Primitive::Null)?,
                        Kind::False | Kind::True => {
                            self.scalar(request, Primitive::Boolean(kind == Kind::True))?
                        }
                        _ => return Err(Self::field_error(request)),
                    },
                    _ => return Err(Self::field_error(request)),
                }
            }
            State::List {
                request,
                list,
                position,
                languages,
            } => {
                if !self.pay(budget, lifetime, &[W::DocumentNodes])? {
                    return Ok(Tick::Blocked);
                }
                if position == list.len() {
                    self.cache(
                        request,
                        if languages {
                            Fact::View(list.value)
                        } else {
                            Fact::List(list)
                        },
                    );
                } else {
                    let child = if languages {
                        list.value.member(position).unwrap().1
                    } else {
                        list.get(position).unwrap()
                    };
                    self.trace.list_elements += 1;
                    if child.kind() != Kind::String {
                        return Err(Self::field_error(request));
                    }
                    self.state = State::List {
                        request,
                        list,
                        position: position + 1,
                        languages,
                    };
                }
            }
            State::BooleanText {
                request,
                text,
                mut position,
                mut bytes,
            } => {
                if position == text.len() {
                    if !self.pay(budget, lifetime, &[W::DocumentNodes])? {
                        return Ok(Tick::Blocked);
                    }
                    self.scalar(
                        request,
                        Primitive::Text(core::str::from_utf8(&bytes[..position]).unwrap()),
                    )?;
                } else {
                    if !self.pay(budget, lifetime, &[W::CodecInputBytes])? {
                        return Ok(Tick::Blocked);
                    }
                    bytes[position] = text.as_bytes()[position];
                    position += 1;
                    self.trace.boolean_bytes += 1;
                    self.state = State::BooleanText {
                        request,
                        text,
                        position,
                        bytes,
                    };
                }
            }
            State::Number {
                request,
                text,
                parse,
            } => {
                if text.len() > self.ceiling {
                    return Err(Cause::NumberLimit {
                        observed: text.len(),
                        ceiling: self.ceiling,
                    });
                }
                let before = *lifetime;
                let parses = &mut self.trace.parses;
                let progress = projection_step::project(
                    text,
                    self.ceiling,
                    budget,
                    lifetime,
                    cancelled,
                    || match parse {
                        Parse::Signed => {
                            parses[0] += 1;
                            text.parse::<i64>().ok().map(Primitive::Signed)
                        }
                        Parse::Unsigned => {
                            parses[1] += 1;
                            text.parse::<u64>().ok().map(Primitive::Unsigned)
                        }
                        Parse::Float => {
                            parses[2] += 1;
                            text.parse::<f64>().ok().map(Primitive::Float)
                        }
                    },
                );
                self.trace.work[1] += before - *lifetime;
                match progress {
                    ProjectionProgress::Pending => return Ok(Tick::Blocked),
                    ProjectionProgress::Limit => return Err(Cause::Lifetime),
                    ProjectionProgress::Cancelled => return Err(Cause::Cancelled),
                    ProjectionProgress::Complete(Some(primitive)) => {
                        self.scalar(request, primitive)?
                    }
                    ProjectionProgress::Complete(None) => match parse {
                        Parse::Signed => {
                            self.state = State::Number {
                                request,
                                text,
                                parse: Parse::Unsigned,
                            }
                        }
                        Parse::Unsigned => {
                            self.state = State::Number {
                                request,
                                text,
                                parse: Parse::Float,
                            }
                        }
                        Parse::Float => return Err(Self::field_error(request)),
                    },
                }
            }
        }
        Ok(Tick::Advanced)
    }
}
#[allow(clippy::large_enum_variant)]
enum Tick<'a> {
    Advanced,
    Blocked,
    Complete(Fields<'a>),
}
const _: () = assert!(!core::mem::needs_drop::<Cursor<'static, 'static>>());
const _: () = assert!(!core::mem::needs_drop::<Fields<'static>>());
