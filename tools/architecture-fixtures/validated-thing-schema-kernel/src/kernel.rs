//! Storage-neutral Basic DataSchema rules, shared only by evidence candidates.
//! Traversal is synchronous/recursive. Only the Number continuation below is
//! resumable; this is not the future bounded admission traversal.

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

const NUMERIC_FIELDS: [Field; 5] = [
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
    if let Some(data_type) = access.data_type(node)
        && data_type != access.kind(node).name()
    {
        return Err(reject(Rule::TypeMismatch));
    }
    for index in 0..access.one_of_count(node) {
        visit(access, access.one_of(node, index), sink, next_ordinal)
            .map_err(|error| sink.child(ChildSite::Indexed(index), error))?;
    }
    if access.flags(node) == (true, true) {
        return Err(reject(Rule::ReadWrite));
    }
    for (min, max) in [
        (Field::MinItems, Field::MaxItems),
        (Field::MinLength, Field::MaxLength),
    ] {
        ordered(
            access.unsigned_extension(node, min),
            access.unsigned_extension(node, max),
            (min, max),
        )
        .map_err(reject)?;
    }
    let mut numeric = NumericCursor::default();
    let ProjectionProgress::Complete(result) = numeric
        .step(numeric_inputs(access, node), |number| {
            ProjectionProgress::Complete(access.project_number(number))
        })
    else {
        unreachable!("synchronous adapter cannot suspend")
    };
    result.map_err(reject)?;
    match access.kind(node) {
        SchemaKind::Array | SchemaKind::String => {
            let (min, max) = access.typed_unsigned(node);
            let names = if access.kind(node) == SchemaKind::Array {
                (Field::MinItems, Field::MaxItems)
            } else {
                (Field::MinLength, Field::MaxLength)
            };
            ordered(min, max, names).map_err(reject)?;
        }
        SchemaKind::Number => {
            let values = access.typed_float(node);
            bounds(&values)
                .and_then(|()| positive(values[4]))
                .map_err(reject)?;
        }
        SchemaKind::Integer => {
            let values = access.typed_integer(node);
            bounds(&values)
                .and_then(|()| positive(values[4]))
                .map_err(reject)?;
        }
        SchemaKind::Object | SchemaKind::Boolean | SchemaKind::Null => {}
    }
    for index in 0..access.child_count(node) {
        let (site, child) = access.child(node, index);
        visit(access, child, sink, next_ordinal).map_err(|error| sink.child(site, error))?;
    }
    Ok(())
}
