//! Storage-neutral Basic DataSchema rules, shared only by evidence candidates.
//! Public traversal is synchronous/recursive. Walk supplies the same discovery
//! and rule order to the bounded subtree witness; it does not charge work.

#[path = "../../validated-thing-feature-boundary/td-boundary/src/projection_step.rs"]
#[allow(dead_code)]
pub mod projection_step;
use projection_step::ProjectionProgress;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchemaKind {
    Array,
    Boolean,
    Number,
    Integer,
    Object,
    String,
    Null,
}

impl SchemaKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Array => "array",
            Self::Boolean => "boolean",
            Self::Number => "number",
            Self::Integer => "integer",
            Self::Object => "object",
            Self::String => "string",
            Self::Null => "null",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Field {
    MinItems,
    MaxItems,
    MinLength,
    MaxLength,
    Minimum,
    ExclusiveMinimum,
    Maximum,
    ExclusiveMaximum,
    MultipleOf,
}
pub const FIELDS: [Field; 9] = [
    Field::MinItems,
    Field::MaxItems,
    Field::MinLength,
    Field::MaxLength,
    Field::Minimum,
    Field::ExclusiveMinimum,
    Field::Maximum,
    Field::ExclusiveMaximum,
    Field::MultipleOf,
];
const UNSIGNED_PAIRS: [(Field, Field); 2] = [
    (Field::MinItems, Field::MaxItems),
    (Field::MinLength, Field::MaxLength),
];

impl Field {
    pub const fn name(self) -> &'static str {
        match self {
            Self::MinItems => "minItems",
            Self::MaxItems => "maxItems",
            Self::MinLength => "minLength",
            Self::MaxLength => "maxLength",
            Self::Minimum => "minimum",
            Self::ExclusiveMinimum => "exclusiveMinimum",
            Self::Maximum => "maximum",
            Self::ExclusiveMaximum => "exclusiveMaximum",
            Self::MultipleOf => "multipleOf",
        }
    }
}

pub const NUMERIC_FIELDS: [Field; 5] = [
    Field::Minimum,
    Field::ExclusiveMinimum,
    Field::Maximum,
    Field::ExclusiveMaximum,
    Field::MultipleOf,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rule {
    TypeMismatch,
    ReadWrite,
    Ordered(Field, Field),
    Positive(Field),
    FailedProjection(Field),
}

#[derive(Clone, Copy)]
// The public formatting sink is compiled only in the source-projection crate;
// TD's Snapshot tests intentionally keep only the inline node ordinal.
#[allow(dead_code)]
pub enum ChildSite<'a> {
    Indexed(usize),
    Property(&'a str),
}

/// Representation facts only: no adapter decides validity or error precedence.
pub trait SchemaAccess<'a> {
    type Node: Copy;
    type Number: Copy;

    fn kind(&self, node: Self::Node) -> SchemaKind;
    fn data_type(&self, node: Self::Node) -> Option<&'a str>;
    fn flags(&self, node: Self::Node) -> (bool, bool);
    fn one_of_count(&self, node: Self::Node) -> usize;
    fn one_of(&self, node: Self::Node, index: usize) -> Self::Node;
    fn child_count(&self, node: Self::Node) -> usize;
    fn child(&self, node: Self::Node, index: usize) -> (ChildSite<'a>, Self::Node);
    fn unsigned_extension(&self, node: Self::Node, field: Field) -> Option<u64>;
    /// Missing and non-Number values both remain absent for numeric predicates.
    fn number_extension(&self, node: Self::Node, field: Field) -> Option<Self::Number>;
    fn project_number(&self, number: Self::Number) -> Option<f64>;
    fn typed_unsigned(&self, node: Self::Node) -> (Option<u32>, Option<u32>);
    fn typed_float(&self, node: Self::Node) -> [Option<f64>; 5];
    fn typed_integer(&self, node: Self::Node) -> [Option<i64>; 5];
}

/// A sink can format the existing public error or retain an inline ordinal.
/// It cannot change whether/when a rule rejects, or visit another node.
pub trait DiagnosticSink<'a, A: SchemaAccess<'a>> {
    type Error;
    fn reject(&self, access: &A, node: A::Node, ordinal: u64, rule: Rule) -> Self::Error;
    fn child(&self, site: ChildSite<'a>, error: Self::Error) -> Self::Error;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InlineInvalid {
    pub ordinal: u64,
    pub rule: Rule,
}

pub struct InlineSink;

impl<'a, A: SchemaAccess<'a>> DiagnosticSink<'a, A> for InlineSink {
    type Error = InlineInvalid;
    fn reject(&self, _: &A, _: A::Node, ordinal: u64, rule: Rule) -> Self::Error {
        InlineInvalid { ordinal, rule }
    }
    fn child(&self, _: ChildSite<'a>, error: Self::Error) -> Self::Error {
        error
    }
}

fn ordered<T: PartialOrd>(
    min: Option<T>,
    max: Option<T>,
    names: (Field, Field),
) -> Result<(), Rule> {
    match (min, max) {
        (Some(min), Some(max)) if min > max => Err(Rule::Ordered(names.0, names.1)),
        _ => Ok(()),
    }
}

fn bounds<T: Copy + PartialOrd>(values: &[Option<T>; 5]) -> Result<(), Rule> {
    for (lower, upper) in [(0, 2), (0, 3), (1, 2), (1, 3)] {
        ordered(
            values[lower],
            values[upper],
            (NUMERIC_FIELDS[lower], NUMERIC_FIELDS[upper]),
        )?;
    }
    Ok(())
}

fn positive<T: PartialOrd + Default>(value: Option<T>) -> Result<(), Rule> {
    match value {
        Some(value) if value <= T::default() => Err(Rule::Positive(Field::MultipleOf)),
        _ => Ok(()),
    }
}

/// Shared rule/discovery order. Adapters may suspend while servicing an action;
/// advancing an action is explicit, so a Pending child cannot be skipped.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Walk {
    stage: u8,
    index: usize,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    Type,
    OneOf(usize),
    Flags,
    Unsigned,
    Numeric,
    Typed,
    Child(usize),
    Done,
}
impl Walk {
    pub fn action(&mut self, one_of: usize, children: usize) -> Action {
        if self.stage == 1 && self.index == one_of {
            self.stage = 2;
            self.index = 0;
        }
        if self.stage == 6 && self.index == children {
            self.stage = 7;
        }
        match self.stage {
            0 => Action::Type,
            1 => Action::OneOf(self.index),
            2 => Action::Flags,
            3 => Action::Unsigned,
            4 => Action::Numeric,
            5 => Action::Typed,
            6 => Action::Child(self.index),
            7 => Action::Done,
            _ => unreachable!(),
        }
    }
    pub fn advance(&mut self) {
        if matches!(self.stage, 1 | 6) {
            self.index += 1;
        } else {
            self.stage += 1;
        }
    }
}
pub fn check_type(data_type: Option<&str>, kind: SchemaKind) -> Result<(), Rule> {
    if data_type.is_some_and(|text| text != kind.name()) {
        Err(Rule::TypeMismatch)
    } else {
        Ok(())
    }
}
pub fn check_flags(flags: (bool, bool)) -> Result<(), Rule> {
    if flags == (true, true) {
        Err(Rule::ReadWrite)
    } else {
        Ok(())
    }
}
pub fn check_unsigned_pair(pair: usize, values: [Option<u64>; 2]) -> Result<(), Rule> {
    ordered(values[0], values[1], UNSIGNED_PAIRS[pair])
}
pub fn check_typed_unsigned(
    kind: SchemaKind,
    min: Option<u32>,
    max: Option<u32>,
) -> Result<(), Rule> {
    let names = if kind == SchemaKind::Array {
        (Field::MinItems, Field::MaxItems)
    } else {
        (Field::MinLength, Field::MaxLength)
    };
    ordered(min, max, names)
}
pub fn check_typed_numeric<T: Copy + PartialOrd + Default>(
    values: &[Option<T>; 5],
) -> Result<(), Rule> {
    bounds(values).and_then(|()| positive(values[4]))
}

/// One shared five-predicate continuation. Four bounds are projected before
/// comparisons, and multipleOf is visited only if those comparisons succeed.
/// Re-entering Pending never accumulates credit or projects an earlier field.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NumericCursor {
    next: usize,
    values: [Option<f64>; 5],
    terminal: Option<Result<(), Rule>>,
}

/// Resolve storage lookups before the isolated projection continuation. These
/// lookups belong to the containing schema traversal, which this synchronous
/// prototype does not budget. A zero projection budget must not hide a map scan.
pub fn numeric_inputs<'a, A: SchemaAccess<'a>>(
    access: &A,
    node: A::Node,
) -> [Option<A::Number>; 5] {
    NUMERIC_FIELDS.map(|field| access.number_extension(node, field))
}

impl NumericCursor {
    pub fn step<N: Copy>(
        &mut self,
        numbers: [Option<N>; 5],
        mut project: impl FnMut(N) -> ProjectionProgress<Option<f64>>,
    ) -> ProjectionProgress<Result<(), Rule>> {
        if let Some(terminal) = self.terminal {
            return ProjectionProgress::Complete(terminal);
        }
        while self.next < NUMERIC_FIELDS.len() {
            if self.next == 4
                && let Err(rule) = bounds(&self.values)
            {
                self.terminal = Some(Err(rule));
                return ProjectionProgress::Complete(Err(rule));
            }
            if let Some(number) = numbers[self.next] {
                let value = match project(number) {
                    ProjectionProgress::Pending => return ProjectionProgress::Pending,
                    ProjectionProgress::Limit => return ProjectionProgress::Limit,
                    ProjectionProgress::Cancelled => return ProjectionProgress::Cancelled,
                    ProjectionProgress::Complete(value) => value.filter(|value| value.is_finite()),
                };
                let Some(value) = value else {
                    let rule = Rule::FailedProjection(NUMERIC_FIELDS[self.next]);
                    self.terminal = Some(Err(rule));
                    return ProjectionProgress::Complete(Err(rule));
                };
                self.values[self.next] = Some(value);
            }
            self.next += 1;
        }
        let result = positive(self.values[4]);
        self.terminal = Some(result);
        ProjectionProgress::Complete(result)
    }
}

pub fn validate<'a, A: SchemaAccess<'a>, S: DiagnosticSink<'a, A>>(
    access: &A,
    root: A::Node,
    sink: &S,
) -> Result<(), S::Error> {
    visit(access, root, sink, &mut 0)
}

fn visit<'a, A: SchemaAccess<'a>, S: DiagnosticSink<'a, A>>(
    access: &A,
    node: A::Node,
    sink: &S,
    next_ordinal: &mut u64,
) -> Result<(), S::Error> {
    let ordinal = *next_ordinal;
    *next_ordinal += 1;
    let reject = |rule| sink.reject(access, node, ordinal, rule);
    let mut walk = Walk::default();
    loop {
        match walk.action(access.one_of_count(node), access.child_count(node)) {
            Action::Type => {
                check_type(access.data_type(node), access.kind(node)).map_err(reject)?
            }
            Action::OneOf(index) => {
                visit(access, access.one_of(node, index), sink, next_ordinal)
                    .map_err(|error| sink.child(ChildSite::Indexed(index), error))?;
            }
            Action::Flags => check_flags(access.flags(node)).map_err(reject)?,
            Action::Unsigned => {
                for (pair, (min, max)) in UNSIGNED_PAIRS.into_iter().enumerate() {
                    check_unsigned_pair(
                        pair,
                        [
                            access.unsigned_extension(node, min),
                            access.unsigned_extension(node, max),
                        ],
                    )
                    .map_err(reject)?;
                }
            }
            Action::Numeric => {
                let mut numeric = NumericCursor::default();
                let ProjectionProgress::Complete(result) = numeric
                    .step(numeric_inputs(access, node), |number| {
                        ProjectionProgress::Complete(access.project_number(number))
                    })
                else {
                    unreachable!("synchronous adapter cannot suspend")
                };
                result.map_err(reject)?;
            }
            Action::Typed => match access.kind(node) {
                SchemaKind::Array | SchemaKind::String => {
                    let (min, max) = access.typed_unsigned(node);
                    check_typed_unsigned(access.kind(node), min, max).map_err(reject)?;
                }
                SchemaKind::Number => {
                    check_typed_numeric(&access.typed_float(node)).map_err(reject)?
                }
                SchemaKind::Integer => {
                    check_typed_numeric(&access.typed_integer(node)).map_err(reject)?
                }
                SchemaKind::Object | SchemaKind::Boolean | SchemaKind::Null => {}
            },
            Action::Child(index) => {
                let (site, child) = access.child(node, index);
                visit(access, child, sink, next_ordinal)
                    .map_err(|error| sink.child(site, error))?;
            }
            Action::Done => return Ok(()),
        }
        walk.advance();
    }
}
