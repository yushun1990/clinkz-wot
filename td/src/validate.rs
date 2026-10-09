use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;

use crate::{
    context::Context,
    data_schema::DataSchema,
    data_type::{AdditionalExpectedResponse, Operation},
    form::Form,
    security_scheme::SecurityScheme,
};

/// Validation strictness for Thing Description documents and components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationLevel {
    /// Accepts any value that passed serde shape and typed field parsing.
    Minimal,
    /// Checks TD required fields, operation context, and local references.
    Basic,
    /// Checks WoT Profile compatibility rules.
    Profile,
    /// Checks all practical semantic rules.
    Full,
}

/// Errors that can occur during the validation of a Thing Description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidateError {
    /// A required field according to the W3C WoT specification is missing.
    MissingRequiredField(String),
    /// An operation type is not allowed in the current context (e.g., 'invokeaction' in a Property).
    InvalidOperation { context: String, found: String },
    /// The data schema constraints are violated.
    InvalidSchema(String),
    /// The security scheme constraints are violated.
    InvalidSecurity(String),
    /// The provided URI does not conform to the expected format.
    InvalidUri(String),
    /// A named reference points to an item that is not defined in this document.
    InvalidReference { context: String, reference: String },
    /// A semantic or profile-level constraint is violated (e.g., missing
    /// standard `@context`, missing interaction affordances).
    InvalidContext(String),
    /// Two or more validation failures discovered in one pass.
    ///
    /// Builders accumulate every error they encounter (instead of returning
    /// only the first) so a caller can fix several issues per rebuild. The
    /// order mirrors discovery order.
    Multiple(Vec<ValidateError>),
}

impl fmt::Display for ValidateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => write!(f, "Missing required field: {}", field),
            Self::InvalidOperation { context, found } => {
                write!(f, "Invalid operation '{}' in context '{}'", found, context)
            }
            Self::InvalidSchema(msg) => write!(f, "Invalid schema: {}", msg),
            Self::InvalidSecurity(msg) => write!(f, "Invalid security scheme: {}", msg),
            Self::InvalidUri(uri) => write!(f, "Invalid URI: {}", uri),
            Self::InvalidReference { context, reference } => {
                write!(
                    f,
                    "Invalid reference '{}' in context '{}'",
                    reference, context
                )
            }
            Self::InvalidContext(msg) => write!(f, "Invalid context: {}", msg),
            Self::Multiple(errors) => {
                write!(f, "Multiple validation errors ({}):", errors.len())?;
                for err in errors {
                    write!(f, "\n  - {err}")?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ValidateError {}

/// A trait for validating components against W3C WoT Thing Description constraints.
pub trait Validate {
    /// Validates the component with the default `Basic` validation level.
    fn validate(&self) -> Result<(), ValidateError> {
        self.validate_with_level(ValidationLevel::Basic)
    }

    /// Validates the component at the requested strictness level.
    fn validate_with_level(&self, level: ValidationLevel) -> Result<(), ValidateError>;
}

/// Parses a URI-valued builder field using `parse`, returning the parsed value
/// as `Some` on success or `None` after recording an
/// [`ValidateError::InvalidUri`] on `errors`.
///
/// Builders share the same "parse-or-record-error" pattern for URI fields.
/// Pass the concrete parser (e.g., `AbsoluteUri::parse` or `BaseUri::parse`)
/// so the helper can serve both absolute and base URI fields.
pub(crate) fn parse_uri_field<T, E>(
    label: &str,
    value: &str,
    parse: impl FnOnce(&str) -> Result<T, E>,
    errors: &mut Vec<ValidateError>,
) -> Option<T> {
    match parse(value) {
        Ok(parsed) => Some(parsed),
        Err(_) => {
            errors.push(ValidateError::InvalidUri(format!("{}: {}", label, value)));
            None
        }
    }
}

/// Flattens a [`ValidateError`] into a concise message string.
///
/// `InvalidSchema` is unwrapped to its inner message so that callers can wrap it
/// again without producing a redundant `Invalid schema:` prefix; any other
/// variant is rendered through its [`fmt::Display`] implementation.
pub(crate) fn schema_error_message(err: ValidateError) -> String {
    match err {
        ValidateError::InvalidSchema(message) => message,
        other => other.to_string(),
    }
}

/// Collapses a collected `Vec<ValidateError>` into a single `Result`.
///
/// Empty → `Ok(())`. One error → that error verbatim. Two or more → a single
/// [`ValidateError::Multiple`] aggregating them, so callers learn about every
/// problem in one pass instead of one-at-a-time across rebuilds.
pub(crate) fn collected_errors(errors: Vec<ValidateError>) -> Result<(), ValidateError> {
    match errors.len() {
        0 => Ok(()),
        1 => Err(errors.into_iter().next().expect("len == 1")),
        _ => Err(ValidateError::Multiple(errors)),
    }
}

/// Prepends an affordance/security `context` to every message-carrying variant
/// of a [`ValidateError`] **without changing its variant**, so the original
/// error taxonomy is preserved for programmatic matching.
pub(crate) fn prepend_context(context: String, err: ValidateError) -> ValidateError {
    match err {
        ValidateError::InvalidSchema(msg) => {
            ValidateError::InvalidSchema(format!("{}: {}", context, msg))
        }
        ValidateError::InvalidSecurity(msg) => {
            ValidateError::InvalidSecurity(format!("{}: {}", context, msg))
        }
        ValidateError::InvalidUri(msg) => {
            ValidateError::InvalidUri(format!("{}: {}", context, msg))
        }
        ValidateError::InvalidContext(msg) => {
            ValidateError::InvalidContext(format!("{}: {}", context, msg))
        }
        ValidateError::MissingRequiredField(field) => {
            ValidateError::MissingRequiredField(format!("{}: {}", context, field))
        }
        ValidateError::InvalidOperation {
            context: inner,
            found,
        } => ValidateError::InvalidOperation {
            context: format!("{}: {}", context, inner),
            found,
        },
        ValidateError::InvalidReference {
            context: inner,
            reference,
        } => ValidateError::InvalidReference {
            context: format!("{}: {}", context, inner),
            reference,
        },
        ValidateError::Multiple(errors) => ValidateError::Multiple(
            errors
                .into_iter()
                .map(|e| prepend_context(context.clone(), e))
                .collect(),
        ),
    }
}

/// Validates that every name in `security` is defined in `security_definitions`.
pub(crate) fn validate_security_references(
    context: &str,
    security: &[String],
    security_definitions: &BTreeMap<String, SecurityScheme>,
) -> Result<(), ValidateError> {
    for reference in security {
        if !security_definitions.contains_key(reference) {
            return Err(ValidateError::InvalidReference {
                context: context.to_string(),
                reference: reference.clone(),
            });
        }
    }

    Ok(())
}

/// Validates each [`DataSchema`] in an optional schema map, contextualizing
/// failures as `"{context}.{name}: {message}"`.
///
/// Accepts an `Option` so callers with optional maps (e.g., `schemaDefinitions`
/// and `uriVariables`) can pass them through directly.
pub(crate) fn validate_schema_map(
    context: &str,
    schemas: Option<&BTreeMap<String, DataSchema>>,
    level: ValidationLevel,
) -> Result<(), ValidateError> {
    let Some(schemas) = schemas else {
        return Ok(());
    };

    for (name, schema) in schemas {
        schema.validate_with_level(level).map_err(|err| {
            ValidateError::InvalidSchema(format!(
                "{}.{}: {}",
                context,
                name,
                schema_error_message(err)
            ))
        })?;
    }

    Ok(())
}

/// Abstraction over form-like types that expose `additionalResponses`.
///
/// Enables [`validate_form_response_references`] to serve both concrete TD forms
/// and Thing Model form templates.
pub(crate) trait HasAdditionalResponses {
    /// Returns the additional expected responses, if any.
    fn additional_responses(&self) -> Option<&[AdditionalExpectedResponse]>;
}

impl HasAdditionalResponses for Form {
    fn additional_responses(&self) -> Option<&[AdditionalExpectedResponse]> {
        self.additional_responses.as_deref()
    }
}

/// Validates that every `additionalResponses[*].schema` reference resolves to a
/// named entry in `schema_definitions`.
///
/// Only runs at [`ValidationLevel::Profile`] or stricter. At lower levels,
/// dangling references are tolerated so that Basic validation stays lenient.
pub(crate) fn validate_form_response_references<T>(
    context: &str,
    forms: &[T],
    schema_definitions: Option<&BTreeMap<String, DataSchema>>,
    level: ValidationLevel,
) -> Result<(), ValidateError>
where
    T: HasAdditionalResponses,
{
    if !matches!(level, ValidationLevel::Profile | ValidationLevel::Full) {
        return Ok(());
    }

    for (form_index, form) in forms.iter().enumerate() {
        let Some(additional_responses) = form.additional_responses() else {
            continue;
        };

        for (response_index, response) in additional_responses.iter().enumerate() {
            let Some(schema) = &response.schema else {
                continue;
            };

            // Build the reference context lazily; only needed on the error path.
            let reference_context = || {
                format!(
                    "{}[{}].additionalResponses[{}].schema",
                    context, form_index, response_index
                )
            };

            let Some(schema_definitions) = schema_definitions else {
                return Err(ValidateError::InvalidReference {
                    context: reference_context(),
                    reference: schema.clone(),
                });
            };

            if !schema_definitions.contains_key(schema) {
                return Err(ValidateError::InvalidReference {
                    context: reference_context(),
                    reference: schema.clone(),
                });
            }
        }
    }

    Ok(())
}

/// Validates that the `@context` contains at least one standard WoT context URI.
///
/// Only runs at [`ValidationLevel::Profile`] or stricter. At lower levels,
/// the context shape is accepted as-is because serde parsing already rejected
/// structurally invalid contexts (e.g., empty arrays).
pub(crate) fn validate_context_at_profile_level(
    context: &Context,
    level: ValidationLevel,
) -> Result<(), ValidateError> {
    if !matches!(level, ValidationLevel::Profile | ValidationLevel::Full) {
        return Ok(());
    }

    if !context.has_wot_context() {
        return Err(ValidateError::InvalidContext(
            "@context must contain at least one standard WoT context URI \
             (https://www.w3.org/2019/wot/td/v1 or https://www.w3.org/2022/wot/td/v1.1)"
                .to_string(),
        ));
    }

    if !context.is_wot_context_first() {
        return Err(ValidateError::InvalidContext(
            "@context must start with a standard WoT context URI \
             (https://www.w3.org/2019/wot/td/v1 or https://www.w3.org/2022/wot/td/v1.1); \
             extension namespaces must follow the standard context"
                .to_string(),
        ));
    }

    Ok(())
}

/// Validates the `op` values declared on Thing-level forms (TD 1.1 §5.3.4).
///
/// Forms declared at the Thing level may only carry meta-interaction
/// operations that target the Thing as a whole (`readallproperties`,
/// `writeallproperties`, `readmultipleproperties`, `writemultipleproperties`,
/// `observeallproperties`, `unobserveallproperties`, `queryallactions`,
/// `subscribeallevents`, `unsubscribeallevents`). Operations that belong to a
/// specific affordance (e.g. `readproperty`, `invokeaction`) are rejected.
/// Forms without an explicit `op` are accepted: Thing-level forms have no
/// default operation, so an omitted `op` simply makes the form unusable until a
/// consumer selects it by an operation it advertises elsewhere.
pub(crate) fn validate_thing_level_form_operations(forms: &[Form]) -> Result<(), ValidateError> {
    for form in forms {
        let Some(operations) = &form.op else {
            continue;
        };
        for operation in operations {
            if !is_thing_level_operation(operation) {
                return Err(ValidateError::InvalidOperation {
                    context: "Thing.forms".to_string(),
                    found: operation.as_str().to_string(),
                });
            }
        }
    }

    Ok(())
}

/// Returns `true` when `operation` is a valid meta-operation for a Thing-level
/// form (TD 1.1 §5.3.4).
fn is_thing_level_operation(operation: &Operation) -> bool {
    basic_kernel::allowed(basic_kernel::OwnerKind::Thing, *operation)
}
/// Validates that the Thing declares at least one interaction affordance or
/// top-level form at Profile/Full level.
///
/// A WoT Profile-conformant Thing MUST provide at least one interaction
/// affordance (property, action, event) or a top-level form so that consumers
/// can discover a usable operation.
pub(crate) fn validate_profile_interaction_presence(
    has_properties: bool,
    has_actions: bool,
    has_events: bool,
    has_top_level_forms: bool,
    level: ValidationLevel,
) -> Result<(), ValidateError> {
    if !matches!(level, ValidationLevel::Profile | ValidationLevel::Full) {
        return Ok(());
    }

    if !has_properties && !has_actions && !has_events && !has_top_level_forms {
        return Err(ValidateError::InvalidContext(
            "Profile-conformant Thing must declare at least one interaction \
             affordance (properties, actions, events) or a top-level form"
                .to_string(),
        ));
    }

    Ok(())
}

// Private rule/discovery program shared by the synchronous TD adapters.
// Work/resource admission is a separate driver; these adapters do not bound it.
pub(crate) mod schema_kernel {

    use core::ops::ControlFlow;

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
    pub enum ChildSite<'a> {
        Indexed(usize),
        Property(&'a str),
    }

    /// Representation facts only: no adapter decides validity or error precedence.
    pub trait SchemaAccess<'a> {
        type Node: Copy;
        type Number: Copy;
        type Children: Iterator<Item = (ChildSite<'a>, Self::Node)>;

        fn kind(&self, node: Self::Node) -> SchemaKind;
        fn data_type(&self, node: Self::Node) -> Option<&'a str>;
        fn flags(&self, node: Self::Node) -> (bool, bool);
        fn one_of_count(&self, node: Self::Node) -> usize;
        fn one_of(&self, node: Self::Node, index: usize) -> Self::Node;
        fn child_count(&self, node: Self::Node) -> usize;
        fn children(&self, node: Self::Node) -> Self::Children;
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

    /// Resolve field lookups once for the synchronous projection loop. A bounded
    /// driver must pay discovery separately and supply already inspected handles;
    /// this helper is not a bounded lookup adapter.
    pub fn numeric_inputs<'a, A: SchemaAccess<'a>>(
        access: &A,
        node: A::Node,
    ) -> [Option<A::Number>; 5] {
        NUMERIC_FIELDS.map(|field| access.number_extension(node, field))
    }

    impl NumericCursor {
        pub fn step<N: Copy, B>(
            &mut self,
            numbers: [Option<N>; 5],
            mut project: impl FnMut(N) -> ControlFlow<B, Option<f64>>,
        ) -> ControlFlow<B, Result<(), Rule>> {
            if let Some(terminal) = self.terminal {
                return ControlFlow::Continue(terminal);
            }
            while self.next < NUMERIC_FIELDS.len() {
                if self.next == 4
                    && let Err(rule) = bounds(&self.values)
                {
                    self.terminal = Some(Err(rule));
                    return ControlFlow::Continue(Err(rule));
                }
                if let Some(number) = numbers[self.next] {
                    let value = match project(number) {
                        ControlFlow::Break(reason) => return ControlFlow::Break(reason),
                        ControlFlow::Continue(value) => value.filter(|value| value.is_finite()),
                    };
                    let Some(value) = value else {
                        let rule = Rule::FailedProjection(NUMERIC_FIELDS[self.next]);
                        self.terminal = Some(Err(rule));
                        return ControlFlow::Continue(Err(rule));
                    };
                    self.values[self.next] = Some(value);
                }
                self.next += 1;
            }
            let result = positive(self.values[4]);
            self.terminal = Some(result);
            ControlFlow::Continue(result)
        }
    }

    /// Synchronous traversal, including recursive children and public formatting.
    /// Admission must resume Walk/NumericCursor and pay source access separately.
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
        let mut children = None;
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
                    let ControlFlow::Continue(result) =
                        numeric.step(numeric_inputs(access, node), |number| {
                            ControlFlow::<core::convert::Infallible, _>::Continue(
                                access.project_number(number),
                            )
                        });
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
                Action::Child(_) => {
                    let (site, child) = children
                        .get_or_insert_with(|| access.children(node))
                        .next()
                        .expect("child count and iterator agree");
                    visit(access, child, sink, next_ordinal)
                        .map_err(|error| sink.child(site, error))?;
                }
                Action::Done => return Ok(()),
            }
            walk.advance();
        }
    }
}

pub(crate) mod schema_access {
    use super::schema_kernel::{ChildSite, Field, SchemaAccess, SchemaKind};
    use crate::data_schema::DataSchema;

    pub struct TypedAccess;

    pub enum Children<'a> {
        Indexed(core::iter::Enumerate<core::slice::Iter<'a, DataSchema>>),
        Properties(alloc::collections::btree_map::Iter<'a, alloc::string::String, DataSchema>),
        Empty,
    }
    impl<'a> Iterator for Children<'a> {
        type Item = (ChildSite<'a>, &'a DataSchema);
        fn next(&mut self) -> Option<Self::Item> {
            match self {
                Self::Indexed(values) => values.next().map(|(i, v)| (ChildSite::Indexed(i), v)),
                Self::Properties(values) => values.next().map(|(k, v)| (ChildSite::Property(k), v)),
                Self::Empty => None,
            }
        }
    }

    impl<'a> SchemaAccess<'a> for TypedAccess {
        type Node = &'a DataSchema;
        type Number = &'a serde_json::Number;
        type Children = Children<'a>;

        fn kind(&self, node: Self::Node) -> SchemaKind {
            match node {
                DataSchema::Array(_) => SchemaKind::Array,
                DataSchema::Boolean(_) => SchemaKind::Boolean,
                DataSchema::Number(_) => SchemaKind::Number,
                DataSchema::Integer(_) => SchemaKind::Integer,
                DataSchema::Object(_) => SchemaKind::Object,
                DataSchema::String(_) => SchemaKind::String,
                DataSchema::Null(_) => SchemaKind::Null,
            }
        }
        fn data_type(&self, node: Self::Node) -> Option<&'a str> {
            node.context().data_type.as_deref()
        }
        fn flags(&self, node: Self::Node) -> (bool, bool) {
            (node.context().read_only, node.context().write_only)
        }
        fn one_of_count(&self, node: Self::Node) -> usize {
            node.context()
                .one_of
                .as_ref()
                .map_or(0, |values| values.len())
        }
        fn one_of(&self, node: Self::Node, index: usize) -> Self::Node {
            &node.context().one_of.as_ref().unwrap()[index]
        }
        fn child_count(&self, node: Self::Node) -> usize {
            match node {
                DataSchema::Array(value) => value.items.as_ref().map_or(0, |values| values.len()),
                DataSchema::Object(value) => {
                    value.properties.as_ref().map_or(0, |values| values.len())
                }
                _ => 0,
            }
        }
        fn children(&self, node: Self::Node) -> Children<'a> {
            match node {
                DataSchema::Array(value) => value
                    .items
                    .as_ref()
                    .map_or(Children::Empty, |v| Children::Indexed(v.iter().enumerate())),
                DataSchema::Object(value) => value
                    .properties
                    .as_ref()
                    .map_or(Children::Empty, |v| Children::Properties(v.iter())),
                _ => Children::Empty,
            }
        }
        fn unsigned_extension(&self, node: Self::Node, field: Field) -> Option<u64> {
            node.context()
                ._extra_fields
                .get(field.name())
                .and_then(serde_json::Value::as_u64)
        }
        fn number_extension(&self, node: Self::Node, field: Field) -> Option<Self::Number> {
            node.context()
                ._extra_fields
                .get(field.name())
                .and_then(serde_json::Value::as_number)
        }
        fn project_number(&self, number: Self::Number) -> Option<f64> {
            number.as_f64()
        }
        fn typed_unsigned(&self, node: Self::Node) -> (Option<u32>, Option<u32>) {
            match node {
                DataSchema::Array(value) => (value.min_items, value.max_items),
                DataSchema::String(value) => (value.min_length, value.max_length),
                _ => (None, None),
            }
        }
        fn typed_float(&self, node: Self::Node) -> [Option<f64>; 5] {
            let DataSchema::Number(value) = node else {
                unreachable!()
            };
            [
                value.minimum,
                value.exclusive_minimum,
                value.maximum,
                value.exclusive_maximum,
                value.multiple_of,
            ]
        }
        fn typed_integer(&self, node: Self::Node) -> [Option<i64>; 5] {
            let DataSchema::Integer(value) = node else {
                unreachable!()
            };
            [
                value.minimum,
                value.exclusive_minimum,
                value.maximum,
                value.exclusive_maximum,
                value.multiple_of,
            ]
        }
    }
}

pub(crate) mod schema_diagnostics {
    use super::schema_kernel::{self, SchemaAccess};
    use crate::validate::ValidateError;
    use alloc::{format, string::String};

    pub struct PublicSchemaSink;

    impl<'a, A: SchemaAccess<'a>> schema_kernel::DiagnosticSink<'a, A> for PublicSchemaSink {
        type Error = ValidateError;
        fn reject(
            &self,
            access: &A,
            schema: A::Node,
            _: u64,
            rule: schema_kernel::Rule,
        ) -> ValidateError {
            use schema_kernel::Rule;
            let message = match rule {
                Rule::TypeMismatch => format!(
                    "type '{}' does not match {} schema",
                    access.data_type(schema).unwrap(),
                    access.kind(schema).name(),
                ),
                Rule::ReadWrite => String::from("readOnly and writeOnly must not both be true"),
                Rule::Ordered(min, max) => format!(
                    "{} must be less than or equal to {}",
                    min.name(),
                    max.name(),
                ),
                Rule::Positive(field) => format!("{} must be greater than 0", field.name()),
                Rule::FailedProjection(field) => {
                    format!("{} Number has no finite binary64 projection", field.name())
                }
            };
            ValidateError::InvalidSchema(message)
        }
        fn child(&self, site: schema_kernel::ChildSite<'a>, error: ValidateError) -> ValidateError {
            use schema_kernel::ChildSite;
            let ValidateError::InvalidSchema(message) = error else {
                unreachable!()
            };
            ValidateError::InvalidSchema(match site {
                ChildSite::Indexed(index) => format!("[{}]: {}", index, message),
                ChildSite::Property(name) => format!("properties.{}: {}", name, message),
            })
        }
    }
}

pub(crate) mod basic_kernel {
    use super::schema_kernel::{self, SchemaAccess};
    use crate::data_type::Operation;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum OwnerKind {
        Thing,
        SecurityDefinition,
        Property,
        Action,
        Event,
    }

    // Synchronous public Basic has no admission resource ceiling. Internal indices
    // therefore use usize; a future bounded adapter checks its own u32 envelope.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Owner {
        pub kind: OwnerKind,
        pub ordinal: usize,
    }

    pub const ROOT: Owner = Owner {
        kind: OwnerKind::Thing,
        ordinal: 0,
    };

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum Field {
        Title,
        Security,
        Scheme,
        OneOf,
        AllOf,
        Name,
        Flow,
        Authorization,
        Token,
        SchemaDefinitions,
        UriVariables,
        PropertySchema,
        Input,
        Output,
        Subscription,
        Data,
        DataResponse,
        Cancellation,
        FormSecurity,
        FormOperation,
        FormHref,
    }

    impl Field {
        pub const fn name(self) -> &'static str {
            match self {
                Self::Title => "title",
                Self::Security => "security",
                Self::Scheme => "scheme",
                Self::OneOf => "oneOf",
                Self::AllOf => "allOf",
                Self::Name => "name",
                Self::Flow => "flow",
                Self::Authorization => "authorization",
                Self::Token => "token",
                Self::SchemaDefinitions => "schemaDefinitions",
                Self::UriVariables => "uriVariables",
                Self::PropertySchema => "",
                Self::Input => "input",
                Self::Output => "output",
                Self::Subscription => "subscription",
                Self::Data => "data",
                Self::DataResponse => "dataResponse",
                Self::Cancellation => "cancellation",
                Self::FormSecurity => "security",
                Self::FormOperation => "op",
                Self::FormHref => "href",
            }
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Site {
        pub owner: Owner,
        pub field: Field,
        /// Schema-map or original Form index, depending on the field.
        pub index: usize,
        /// Reference/operation index within its ordered sequence.
        pub member: usize,
    }

    impl Site {
        pub const fn new(owner: Owner, field: Field) -> Self {
            Self {
                owner,
                field,
                index: 0,
                member: 0,
            }
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum Rule<'a> {
        Missing,
        UnsupportedScheme(&'a str),
        ComboMissing,
        ComboCardinality,
        ComboEmpty,
        UnsupportedFlow(&'a str),
        Undefined(&'a str),
        Operation(Operation),
    }

    /// Shared with the public TD default/security adapters. Explicit empty
    /// sequences are preserved by callers; these helpers introduce no new policy.
    pub fn default_property_operations(flags: (bool, bool)) -> &'static [Operation] {
        match flags {
            (true, false) => &[Operation::ReadProperty],
            (false, true) => &[Operation::WriteProperty],
            // Both true is Basic-invalid. Minimal still uses the neutral default.
            _ => &[Operation::ReadProperty, Operation::WriteProperty],
        }
    }

    pub fn inherited_security<N: Copy>(root: N, explicit: Option<N>) -> (N, bool) {
        explicit.map_or((root, true), |names| (names, false))
    }

    pub fn required_security(count: usize) -> Result<(), Rule<'static>> {
        required(count != 0)
    }

    pub fn required(present: bool) -> Result<(), Rule<'static>> {
        if !present { Err(Rule::Missing) } else { Ok(()) }
    }

    pub fn combo_groups(scheme: &str) -> &'static [Field] {
        if scheme == "combo" {
            &[Field::OneOf, Field::AllOf]
        } else {
            &[]
        }
    }

    pub fn first_undefined<'a>(
        names: impl Iterator<Item = &'a str>,
        exists: impl Fn(&str) -> bool,
    ) -> Option<(usize, &'a str)> {
        names.enumerate().find(|(_, name)| !exists(name))
    }

    /// No validity, defaulting, reference policy, or error precedence in adapters.
    pub trait BasicAccess<'a> {
        type Schemas: SchemaAccess<'a>;
        type SchemaMap: Copy;
        type Affordance: Copy;
        type Forms: Copy;
        type Form: Copy;
        type Operations: Copy;
        type Names: Copy;
        type Definition: Copy;
        type NameIter: Iterator<Item = &'a str>;
        type DefinitionIter: Iterator<Item = Self::Definition>;
        type SchemaIter: Iterator<Item = (&'a str, <Self::Schemas as SchemaAccess<'a>>::Node)>;
        type AffordanceIter: Iterator<Item = Self::Affordance>;

        fn schemas(&self) -> &Self::Schemas;
        fn title(&self) -> Option<&'a str>;
        fn root_security(&self) -> Self::Names;
        fn names_count(&self, names: Self::Names) -> usize;
        fn names(&self, names: Self::Names) -> Self::NameIter;
        fn definition_count(&self) -> usize;
        fn definitions(&self) -> Self::DefinitionIter;
        /// Used only by the synchronous public diagnostic sink after rejection.
        fn definition_name(&self, index: usize) -> &'a str;
        fn definition_exists(&self, name: &str) -> bool;
        fn scheme(&self, definition: Self::Definition) -> &'a str;
        fn combo_names(&self, definition: Self::Definition, field: Field) -> Self::Names;
        fn security_string(&self, definition: Self::Definition, field: Field) -> Option<&'a str>;
        fn endpoint_present(&self, definition: Self::Definition, field: Field) -> bool;

        fn root_schema_map(&self, field: Field) -> Option<Self::SchemaMap>;
        fn schema_entries(&self, map: Self::SchemaMap) -> Self::SchemaIter;
        fn affordance_count(&self, kind: OwnerKind) -> usize;
        fn affordances(&self, kind: OwnerKind) -> Self::AffordanceIter;
        /// Used only by the synchronous public diagnostic sink after rejection.
        fn affordance_name(&self, owner: Owner) -> &'a str;
        fn uri_variables(&self, affordance: Self::Affordance) -> Option<Self::SchemaMap>;
        fn affordance_schema(
            &self,
            affordance: Self::Affordance,
            field: Field,
        ) -> Option<<Self::Schemas as SchemaAccess<'a>>::Node>;
        fn forms(&self, affordance: Option<Self::Affordance>) -> Option<Self::Forms>;
        fn form_count(&self, forms: Self::Forms) -> usize;
        fn form_at(&self, forms: Self::Forms, index: usize) -> Self::Form;
        fn operations(&self, form: Self::Form) -> Option<Self::Operations>;
        fn operation_count(&self, ops: Self::Operations) -> usize;
        fn operation_at(&self, ops: Self::Operations, index: usize) -> Operation;
        fn form_security(&self, form: Self::Form) -> Option<Self::Names>;
    }

    pub trait DiagnosticSink<'a, A: BasicAccess<'a>>:
        schema_kernel::DiagnosticSink<'a, A::Schemas>
    {
        fn reject_basic(&self, access: &A, site: Site, rule: Rule<'a>) -> Self::Error;
        fn schema_site(
            &self,
            access: &A,
            site: Site,
            key: Option<&'a str>,
            error: Self::Error,
        ) -> Self::Error;
    }

    fn references<'a, A: BasicAccess<'a>, S: DiagnosticSink<'a, A>>(
        access: &A,
        names: A::Names,
        mut site: Site,
        sink: &S,
    ) -> Result<(), S::Error> {
        if let Some((index, reference)) =
            first_undefined(access.names(names), |name| access.definition_exists(name))
        {
            site.member = index;
            return Err(sink.reject_basic(access, site, Rule::Undefined(reference)));
        }
        Ok(())
    }

    pub fn validate_security_scheme<'a, A: BasicAccess<'a>, S: DiagnosticSink<'a, A>>(
        access: &A,
        definition: A::Definition,
        owner: Owner,
        sink: &S,
    ) -> Result<(), S::Error> {
        let reject = |field, rule| sink.reject_basic(access, Site::new(owner, field), rule);
        match scheme_kind(access.scheme(definition)).map_err(|rule| reject(Field::Scheme, rule))? {
            Scheme::Combo => {
                let one = access.combo_names(definition, Field::OneOf);
                let all = access.combo_names(definition, Field::AllOf);
                combo_present(access.names_count(one), access.names_count(all))
                    .map_err(|rule| reject(Field::Scheme, rule))?;
                for (field, names) in [(Field::OneOf, one), (Field::AllOf, all)] {
                    let count = access.names_count(names);
                    // Cardinality rejects before this group's member checks.
                    if count == 1 {
                        combo_group(count, None).map_err(|(member, rule)| {
                            let mut site = Site::new(owner, field);
                            site.member = member;
                            sink.reject_basic(access, site, rule)
                        })?;
                    }
                    let empty = access.names(names).position(str::is_empty);
                    combo_group(count, empty).map_err(|(member, rule)| {
                        let mut site = Site::new(owner, field);
                        site.member = member;
                        sink.reject_basic(access, site, rule)
                    })?;
                }
            }
            Scheme::ApiKey => {
                required(
                    !access
                        .security_string(definition, Field::Name)
                        .unwrap_or("")
                        .is_empty(),
                )
                .map_err(|rule| reject(Field::Name, rule))?;
            }
            Scheme::OAuth => {
                let flow = access
                    .security_string(definition, Field::Flow)
                    .unwrap_or("");
                if flow_requires_endpoints(flow).map_err(|rule| reject(Field::Flow, rule))? {
                    for field in [Field::Authorization, Field::Token] {
                        required(access.endpoint_present(definition, field))
                            .map_err(|rule| reject(field, rule))?;
                    }
                }
            }
            Scheme::Other => {}
        }
        Ok(())
    }

    fn schema_map<'a, A: BasicAccess<'a>, S: DiagnosticSink<'a, A>>(
        access: &A,
        map: Option<A::SchemaMap>,
        owner: Owner,
        field: Field,
        sink: &S,
    ) -> Result<(), S::Error> {
        if let Some(map) = map {
            for (index, (name, node)) in access.schema_entries(map).enumerate() {
                let mut site = Site::new(owner, field);
                site.index = index;
                schema_kernel::validate(access.schemas(), node, sink)
                    .map_err(|error| sink.schema_site(access, site, Some(name), error))?;
            }
        }
        Ok(())
    }

    pub(crate) fn allowed(kind: OwnerKind, operation: Operation) -> bool {
        use Operation::*;
        match kind {
            OwnerKind::Property => matches!(
                operation,
                ReadProperty | WriteProperty | ObserveProperty | UnobserveProperty
            ),
            OwnerKind::Action => matches!(operation, InvokeAction | QueryAction | CancelAction),
            OwnerKind::Event => matches!(operation, SubscribeEvent | UnsubscribeEvent),
            OwnerKind::Thing => matches!(
                operation,
                ReadAllProperties
                    | WriteAllProperties
                    | ReadMultipleProperties
                    | WriteMultipleProperties
                    | ObserveAllProperties
                    | UnobserveAllProperties
                    | QueryAllActions
                    | SubscribeAllEvents
                    | UnsubscribeAllEvents
            ),
            OwnerKind::SecurityDefinition => unreachable!(),
        }
    }

    /// One discovery program for synchronous public Basic and paid bounded
    /// Basic. Actions contain coordinates only; they do not inspect storage.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum Action {
        Title,
        RequiredSecurity,
        RootReferences,
        Definition(Owner),
        SchemaMap(Owner, Field),
        Schema(Owner, Field),
        Operations(Owner),
        FormSecurity(Owner),
        Done,
    }
    #[derive(Clone, Copy, Debug, Default)]
    pub struct Walk {
        stage: u8,
        index: usize,
        kind: usize,
        component: usize,
    }
    pub fn affordance_fields(kind: OwnerKind) -> &'static [Field] {
        match kind {
            OwnerKind::Property => &[],
            OwnerKind::Action => &[Field::Input, Field::Output],
            OwnerKind::Event => &[
                Field::Subscription,
                Field::Data,
                Field::DataResponse,
                Field::Cancellation,
            ],
            _ => unreachable!(),
        }
    }
    impl Walk {
        pub fn action(&mut self, definitions: usize, affordances: [usize; 3]) -> Action {
            if self.stage == 3 && self.index == definitions {
                self.stage = 4;
                self.index = 0;
            }
            while self.stage == 6 && self.kind < 3 && self.index == affordances[self.kind] {
                self.kind += 1;
                self.index = 0;
            }
            if self.stage == 6 && self.kind == 3 {
                self.stage = 7;
            }
            match self.stage {
                0 => Action::Title,
                1 => Action::RequiredSecurity,
                2 => Action::RootReferences,
                3 => Action::Definition(Owner {
                    kind: OwnerKind::SecurityDefinition,
                    ordinal: self.index,
                }),
                4 => Action::SchemaMap(ROOT, Field::SchemaDefinitions),
                5 => Action::SchemaMap(ROOT, Field::UriVariables),
                6 => {
                    let owner = Owner {
                        kind: [OwnerKind::Property, OwnerKind::Action, OwnerKind::Event][self.kind],
                        ordinal: self.index,
                    };
                    affordance_action(owner, self.component)
                }
                7 => Action::FormSecurity(ROOT),
                8 => Action::Operations(ROOT),
                _ => Action::Done,
            }
        }
        pub fn advance(&mut self) {
            match self.stage {
                3 => self.index += 1,
                6 => {
                    let kind =
                        [OwnerKind::Property, OwnerKind::Action, OwnerKind::Event][self.kind];
                    let last = affordance_fields(kind).len()
                        + if kind == OwnerKind::Property { 3 } else { 2 };
                    if self.component == last {
                        self.component = 0;
                        self.index += 1;
                    } else {
                        self.component += 1;
                    }
                }
                _ => self.stage += 1,
            }
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum Scheme {
        Combo,
        ApiKey,
        OAuth,
        Other,
    }
    pub fn scheme_kind(scheme: &str) -> Result<Scheme, Rule<'_>> {
        Ok(match scheme {
            "combo" => Scheme::Combo,
            "apikey" => Scheme::ApiKey,
            "oauth2" => Scheme::OAuth,
            "nosec" | "auto" | "basic" | "digest" | "bearer" | "psk" => Scheme::Other,
            _ => return Err(Rule::UnsupportedScheme(scheme)),
        })
    }
    pub fn flow_requires_endpoints(flow: &str) -> Result<bool, Rule<'_>> {
        match flow {
            "code" => Ok(true),
            "client" | "device" => Ok(false),
            _ => Err(Rule::UnsupportedFlow(flow)),
        }
    }
    pub fn combo_present(one: usize, all: usize) -> Result<(), Rule<'static>> {
        if one == 0 && all == 0 {
            Err(Rule::ComboMissing)
        } else {
            Ok(())
        }
    }
    pub fn combo_group(
        count: usize,
        first_empty: Option<usize>,
    ) -> Result<(), (usize, Rule<'static>)> {
        if count == 1 {
            Err((0, Rule::ComboCardinality))
        } else if let Some(index) = first_empty {
            Err((index, Rule::ComboEmpty))
        } else {
            Ok(())
        }
    }

    fn operations<'a, A: BasicAccess<'a>, S: DiagnosticSink<'a, A>>(
        access: &A,
        forms: Option<A::Forms>,
        owner: Owner,
        sink: &S,
    ) -> Result<(), S::Error> {
        if let Some(forms) = forms {
            for index in 0..access.form_count(forms) {
                if let Some(ops) = access.operations(access.form_at(forms, index)) {
                    for member in 0..access.operation_count(ops) {
                        let op = access.operation_at(ops, member);
                        if !allowed(owner.kind, op) {
                            return Err(sink.reject_basic(
                                access,
                                Site {
                                    owner,
                                    field: Field::FormOperation,
                                    index,
                                    member,
                                },
                                Rule::Operation(op),
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Component discovery is shared by Thing's Walk and standalone adapters.
    fn affordance_action(owner: Owner, component: usize) -> Action {
        let fields = affordance_fields(owner.kind);
        match component {
            0 if owner.kind == OwnerKind::Property => Action::Schema(owner, Field::PropertySchema),
            0 => Action::SchemaMap(owner, Field::UriVariables),
            1 if owner.kind == OwnerKind::Property => Action::SchemaMap(owner, Field::UriVariables),
            n if owner.kind != OwnerKind::Property && n <= fields.len() => {
                Action::Schema(owner, fields[n - 1])
            }
            n if n
                == fields.len()
                    + if owner.kind == OwnerKind::Property {
                        2
                    } else {
                        1
                    } =>
            {
                Action::Operations(owner)
            }
            _ => Action::FormSecurity(owner),
        }
    }

    pub fn validate_affordance<'a, A: BasicAccess<'a>, S: DiagnosticSink<'a, A>>(
        access: &A,
        affordance: A::Affordance,
        owner: Owner,
        sink: &S,
    ) -> Result<(), S::Error> {
        let mut component = 0;
        loop {
            match affordance_action(owner, component) {
                Action::Schema(_, field) => {
                    if let Some(node) = access.affordance_schema(affordance, field) {
                        schema_kernel::validate(access.schemas(), node, sink).map_err(|error| {
                            sink.schema_site(access, Site::new(owner, field), None, error)
                        })?;
                    }
                }
                Action::SchemaMap(_, field) => {
                    schema_map(access, access.uri_variables(affordance), owner, field, sink)?
                }
                Action::Operations(_) => {
                    operations(access, access.forms(Some(affordance)), owner, sink)?
                }
                // Standalone components have no containing security definitions.
                Action::FormSecurity(_) => return Ok(()),
                _ => unreachable!(),
            }
            component += 1;
        }
    }

    fn form_security<'a, A: BasicAccess<'a>, S: DiagnosticSink<'a, A>>(
        access: &A,
        forms: Option<A::Forms>,
        owner: Owner,
        sink: &S,
    ) -> Result<(), S::Error> {
        if let Some(forms) = forms {
            for index in 0..access.form_count(forms) {
                if let Some(names) = access.form_security(access.form_at(forms, index)) {
                    let mut site = Site::new(owner, Field::FormSecurity);
                    site.index = index;
                    references(access, names, site, sink)?;
                }
            }
        }
        Ok(())
    }

    /// Synchronous complete Basic adapter. Bounded admission instead drives Walk
    /// and these predicates with its paid native iterators and frame workspace.
    pub fn validate<'a, A: BasicAccess<'a>, S: DiagnosticSink<'a, A>>(
        access: &A,
        sink: &S,
    ) -> Result<(), S::Error> {
        let mut walk = Walk::default();
        let kinds = [OwnerKind::Property, OwnerKind::Action, OwnerKind::Event];
        let mut definitions = access.definitions();
        let mut affordances = kinds.map(|kind| access.affordances(kind));
        let mut current_affordance = None;
        loop {
            let action = walk.action(
                access.definition_count(),
                kinds.map(|kind| access.affordance_count(kind)),
            );
            // All component actions for an owner use the same native iterator
            // entry. No prefix traversal is rebuilt from its ordinal.
            let owner = match action {
                Action::SchemaMap(owner, _)
                | Action::Schema(owner, _)
                | Action::Operations(owner)
                | Action::FormSecurity(owner)
                    if owner.kind != OwnerKind::Thing =>
                {
                    Some(owner)
                }
                _ => None,
            };
            if let Some(owner) = owner {
                if current_affordance.is_none_or(|(selected, _)| selected != owner) {
                    let kind = kinds.iter().position(|&k| k == owner.kind).unwrap();
                    let value = affordances[kind]
                        .next()
                        .expect("affordance count and iterator agree");
                    current_affordance = Some((owner, value));
                }
            }
            match action {
                Action::Title => {
                    required(!access.title().unwrap_or("").is_empty()).map_err(|rule| {
                        sink.reject_basic(access, Site::new(ROOT, Field::Title), rule)
                    })?;
                }
                Action::RequiredSecurity => {
                    required_security(access.names_count(access.root_security())).map_err(
                        |rule| sink.reject_basic(access, Site::new(ROOT, Field::Security), rule),
                    )?
                }
                Action::RootReferences => references(
                    access,
                    access.root_security(),
                    Site::new(ROOT, Field::Security),
                    sink,
                )?,
                Action::Definition(owner) => {
                    let definition = definitions
                        .next()
                        .expect("definition count and iterator agree");
                    validate_security_scheme(access, definition, owner, sink)?;
                    for &field in combo_groups(access.scheme(definition)) {
                        references(
                            access,
                            access.combo_names(definition, field),
                            Site::new(owner, field),
                            sink,
                        )?;
                    }
                }
                Action::SchemaMap(owner, field) => {
                    let map = if owner.kind == OwnerKind::Thing {
                        access.root_schema_map(field)
                    } else {
                        access.uri_variables(current_affordance.unwrap().1)
                    };
                    schema_map(access, map, owner, field, sink)?;
                }
                Action::Schema(owner, field) => {
                    if let Some(node) =
                        access.affordance_schema(current_affordance.unwrap().1, field)
                    {
                        schema_kernel::validate(access.schemas(), node, sink).map_err(|error| {
                            sink.schema_site(access, Site::new(owner, field), None, error)
                        })?;
                    }
                }
                Action::Operations(owner) | Action::FormSecurity(owner) => {
                    let forms = access.forms(
                        (owner.kind != OwnerKind::Thing).then(|| current_affordance.unwrap().1),
                    );
                    if matches!(action, Action::Operations(_)) {
                        operations(access, forms, owner, sink)?;
                    } else {
                        form_security(access, forms, owner, sink)?;
                    }
                }
                Action::Done => return Ok(()),
            }
            walk.advance();
        }
    }
}

pub(crate) mod basic_typed {
    use super::{
        basic_kernel::{BasicAccess, Field, Owner, OwnerKind},
        schema_access::TypedAccess,
    };
    use crate::{
        affordance::{
            ActionAffordance, EventAffordance, InteractionAffordance, PropertyAffordance,
        },
        data_schema::DataSchema,
        data_type::Operation,
        form::Form,
        security_scheme::{SecurityScheme, SecuritySchemeContext},
        thing::Thing,
    };
    use alloc::{collections::BTreeMap, string::String};
    use serde_json::Value;

    #[derive(Clone, Copy)]
    pub enum Affordance<'a> {
        Property(&'a PropertyAffordance),
        Action(&'a ActionAffordance),
        Event(&'a EventAffordance),
    }

    impl<'a> Affordance<'a> {
        fn interaction(self) -> &'a InteractionAffordance {
            match self {
                Self::Property(value) => &value._interaction,
                Self::Action(value) => &value._interaction,
                Self::Event(value) => &value._interaction,
            }
        }
    }

    #[derive(Clone, Copy)]
    pub enum Names<'a> {
        Strings(&'a [String]),
        JsonArray(&'a [Value]),
        JsonString(&'a str),
        Empty,
    }

    impl<'a> Names<'a> {
        pub fn count(self) -> usize {
            match self {
                Self::Strings(values) => values.len(),
                Self::JsonArray(values) => values.iter().filter(|value| value.is_string()).count(),
                Self::JsonString(_) => 1,
                Self::Empty => 0,
            }
        }
        pub fn iter(self) -> NameIter<'a> {
            match self {
                Self::Strings(values) => NameIter::Strings(values.iter()),
                Self::JsonArray(values) => NameIter::Json(values.iter()),
                Self::JsonString(value) => NameIter::One(Some(value)),
                Self::Empty => NameIter::One(None),
            }
        }
    }
    pub enum NameIter<'a> {
        Strings(core::slice::Iter<'a, String>),
        Json(core::slice::Iter<'a, Value>),
        One(Option<&'a str>),
    }
    impl<'a> Iterator for NameIter<'a> {
        type Item = &'a str;
        fn next(&mut self) -> Option<Self::Item> {
            match self {
                Self::Strings(values) => values.next().map(String::as_str),
                Self::Json(values) => values.find_map(Value::as_str),
                Self::One(value) => value.take(),
            }
        }
    }
    pub enum AffordanceIter<'a> {
        Properties(Option<alloc::collections::btree_map::Values<'a, String, PropertyAffordance>>),
        Actions(Option<alloc::collections::btree_map::Values<'a, String, ActionAffordance>>),
        Events(Option<alloc::collections::btree_map::Values<'a, String, EventAffordance>>),
    }
    impl<'a> Iterator for AffordanceIter<'a> {
        type Item = Affordance<'a>;
        fn next(&mut self) -> Option<Self::Item> {
            match self {
                Self::Properties(values) => values.as_mut()?.next().map(Affordance::Property),
                Self::Actions(values) => values.as_mut()?.next().map(Affordance::Action),
                Self::Events(values) => values.as_mut()?.next().map(Affordance::Event),
            }
        }
    }
    pub type SchemaIter<'a> = core::iter::Map<
        alloc::collections::btree_map::Iter<'a, String, DataSchema>,
        fn((&'a String, &'a DataSchema)) -> (&'a str, &'a DataSchema),
    >;
    fn schema_entry<'a>((name, node): (&'a String, &'a DataSchema)) -> (&'a str, &'a DataSchema) {
        (name, node)
    }

    pub fn security_context(value: &SecurityScheme) -> &SecuritySchemeContext {
        match value {
            SecurityScheme::NoSec(v) => &v._context,
            SecurityScheme::Auto(v) => &v._context,
            SecurityScheme::Combo(v) => &v._context,
            SecurityScheme::Basic(v) => &v._context,
            SecurityScheme::Digest(v) => &v._context,
            SecurityScheme::APIKey(v) => &v._context,
            SecurityScheme::Bearer(v) => &v._context,
            SecurityScheme::PSK(v) => &v._context,
            SecurityScheme::OAuth2(v) => &v._context,
        }
    }

    /// Whole-document methods require a root; component methods borrow only the
    /// passed component. No dummy Thing is constructed for standalone validation.
    pub struct TypedBasicAccess<'a>(pub Option<&'a Thing>);
    static SCHEMAS: TypedAccess = TypedAccess;

    impl<'a> BasicAccess<'a> for TypedBasicAccess<'a> {
        type Schemas = TypedAccess;
        type SchemaMap = &'a BTreeMap<String, DataSchema>;
        type Affordance = Affordance<'a>;
        type Forms = &'a [Form];
        type Form = &'a Form;
        type Operations = &'a [Operation];
        type Names = Names<'a>;
        type Definition = &'a SecurityScheme;
        type NameIter = NameIter<'a>;
        type DefinitionIter = alloc::collections::btree_map::Values<'a, String, SecurityScheme>;
        type SchemaIter = SchemaIter<'a>;
        type AffordanceIter = AffordanceIter<'a>;
        fn schemas(&self) -> &TypedAccess {
            &SCHEMAS
        }
        fn title(&self) -> Option<&'a str> {
            self.0.unwrap()._metadata.title.as_deref()
        }
        fn root_security(&self) -> Names<'a> {
            Names::Strings(&self.0.unwrap().security)
        }
        fn names_count(&self, names: Names<'a>) -> usize {
            names.count()
        }
        fn names(&self, names: Names<'a>) -> NameIter<'a> {
            names.iter()
        }
        fn definition_count(&self) -> usize {
            self.0.unwrap().security_definitions.len()
        }
        fn definitions(&self) -> Self::DefinitionIter {
            self.0.unwrap().security_definitions.values()
        }
        /// Used only by the synchronous public diagnostic sink after rejection.
        fn definition_name(&self, index: usize) -> &'a str {
            self.0
                .unwrap()
                .security_definitions
                .keys()
                .nth(index)
                .unwrap()
        }
        fn definition_exists(&self, name: &str) -> bool {
            self.0.unwrap().security_definitions.contains_key(name)
        }
        fn scheme(&self, definition: Self::Definition) -> &'a str {
            definition.scheme()
        }
        fn combo_names(&self, definition: Self::Definition, field: Field) -> Names<'a> {
            if let SecurityScheme::Combo(value) = definition {
                return Names::Strings(match field {
                    Field::OneOf => &value.one_of,
                    Field::AllOf => &value.all_of,
                    _ => unreachable!(),
                });
            }
            match security_context(definition)._extra_fields.get(field.name()) {
                Some(Value::Array(values)) => Names::JsonArray(values),
                Some(Value::String(value)) => Names::JsonString(value),
                _ => Names::Empty,
            }
        }
        fn security_string(&self, definition: Self::Definition, field: Field) -> Option<&'a str> {
            match (definition, field) {
                (SecurityScheme::APIKey(value), Field::Name) => value.name.as_deref(),
                (SecurityScheme::OAuth2(value), Field::Flow) => Some(&value.flow),
                _ => security_context(definition)
                    ._extra_fields
                    .get(field.name())
                    .and_then(Value::as_str),
            }
        }
        fn endpoint_present(&self, definition: Self::Definition, field: Field) -> bool {
            match (definition, field) {
                (SecurityScheme::OAuth2(value), Field::Authorization) => {
                    value.authorization.is_some()
                }
                (SecurityScheme::OAuth2(value), Field::Token) => value.token.is_some(),
                _ => self
                    .security_string(definition, field)
                    .is_some_and(|value| !value.is_empty()),
            }
        }
        fn root_schema_map(&self, field: Field) -> Option<Self::SchemaMap> {
            let thing = self.0.unwrap();
            match field {
                Field::SchemaDefinitions => thing.schema_definitions.as_ref(),
                Field::UriVariables => thing.uri_variables.as_ref(),
                _ => unreachable!(),
            }
        }
        fn schema_entries(&self, map: Self::SchemaMap) -> SchemaIter<'a> {
            map.iter().map(schema_entry)
        }
        fn affordance_count(&self, kind: OwnerKind) -> usize {
            let thing = self.0.unwrap();
            match kind {
                OwnerKind::Property => thing.properties.as_ref().map_or(0, |map| map.len()),
                OwnerKind::Action => thing.actions.as_ref().map_or(0, |map| map.len()),
                OwnerKind::Event => thing.events.as_ref().map_or(0, |map| map.len()),
                _ => unreachable!(),
            }
        }
        fn affordances(&self, kind: OwnerKind) -> AffordanceIter<'a> {
            let thing = self.0.unwrap();
            match kind {
                OwnerKind::Property => {
                    AffordanceIter::Properties(thing.properties.as_ref().map(BTreeMap::values))
                }
                OwnerKind::Action => {
                    AffordanceIter::Actions(thing.actions.as_ref().map(BTreeMap::values))
                }
                OwnerKind::Event => {
                    AffordanceIter::Events(thing.events.as_ref().map(BTreeMap::values))
                }
                _ => unreachable!(),
            }
        }
        /// Used only by the synchronous public diagnostic sink after rejection.
        fn affordance_name(&self, owner: Owner) -> &'a str {
            let thing = self.0.unwrap();
            let index = owner.ordinal;
            match owner.kind {
                OwnerKind::Property => thing
                    .properties
                    .as_ref()
                    .unwrap()
                    .keys()
                    .nth(index)
                    .unwrap(),
                OwnerKind::Action => thing.actions.as_ref().unwrap().keys().nth(index).unwrap(),
                OwnerKind::Event => thing.events.as_ref().unwrap().keys().nth(index).unwrap(),
                _ => unreachable!(),
            }
        }
        fn uri_variables(&self, affordance: Affordance<'a>) -> Option<Self::SchemaMap> {
            affordance.interaction().uri_variables.as_ref()
        }
        fn affordance_schema(
            &self,
            affordance: Affordance<'a>,
            field: Field,
        ) -> Option<&'a DataSchema> {
            match (affordance, field) {
                (Affordance::Property(value), Field::PropertySchema) => Some(&value._schema),
                (Affordance::Action(value), Field::Input) => value.input.as_ref(),
                (Affordance::Action(value), Field::Output) => value.output.as_ref(),
                (Affordance::Event(value), Field::Subscription) => value.subscription.as_ref(),
                (Affordance::Event(value), Field::Data) => value.data.as_ref(),
                (Affordance::Event(value), Field::DataResponse) => value.data_response.as_ref(),
                (Affordance::Event(value), Field::Cancellation) => value.cancellation.as_ref(),
                _ => unreachable!(),
            }
        }
        fn forms(&self, affordance: Option<Affordance<'a>>) -> Option<Self::Forms> {
            match affordance {
                Some(value) => Some(&value.interaction().forms),
                None => self.0.unwrap().forms.as_deref(),
            }
        }
        fn form_count(&self, forms: Self::Forms) -> usize {
            forms.len()
        }
        fn form_at(&self, forms: Self::Forms, index: usize) -> Self::Form {
            &forms[index]
        }
        fn operations(&self, form: Self::Form) -> Option<Self::Operations> {
            form.op.as_deref()
        }
        fn operation_count(&self, ops: Self::Operations) -> usize {
            ops.len()
        }
        fn operation_at(&self, ops: Self::Operations, index: usize) -> Operation {
            ops[index]
        }
        fn form_security(&self, form: Self::Form) -> Option<Names<'a>> {
            form.security.as_deref().map(Names::Strings)
        }
    }
}

pub(crate) mod basic_diagnostics {
    use super::{
        basic_kernel::{BasicAccess, DiagnosticSink, Owner, OwnerKind, Rule, Site},
        schema_kernel::{self, SchemaAccess},
    };
    use crate::validate::ValidateError;
    use alloc::{
        format,
        string::{String, ToString},
    };

    /// Component entry points omit the enclosing Thing's name prefix.
    pub struct PublicSink {
        pub document: bool,
    }

    fn owner_name<'a, A: BasicAccess<'a>>(access: &A, owner: Owner) -> String {
        match owner.kind {
            OwnerKind::Thing => "Thing.forms".into(),
            OwnerKind::SecurityDefinition => format!(
                "securityDefinitions.{}",
                access.definition_name(owner.ordinal)
            ),
            kind => format!("{} '{}'", kind_name(kind), access.affordance_name(owner)),
        }
    }

    fn kind_name(kind: OwnerKind) -> &'static str {
        match kind {
            OwnerKind::Property => "Property",
            OwnerKind::Action => "Action",
            OwnerKind::Event => "Event",
            _ => unreachable!(),
        }
    }

    impl<'a, A: SchemaAccess<'a>> schema_kernel::DiagnosticSink<'a, A> for PublicSink {
        type Error = ValidateError;
        fn reject(
            &self,
            access: &A,
            node: A::Node,
            ordinal: u64,
            rule: schema_kernel::Rule,
        ) -> Self::Error {
            schema_kernel::DiagnosticSink::reject(
                &super::schema_diagnostics::PublicSchemaSink,
                access,
                node,
                ordinal,
                rule,
            )
        }
        fn child(&self, site: schema_kernel::ChildSite<'a>, error: Self::Error) -> Self::Error {
            <super::schema_diagnostics::PublicSchemaSink as schema_kernel::DiagnosticSink<'a, A>>::child(
            &super::schema_diagnostics::PublicSchemaSink,
            site,
            error,
        )
        }
    }

    impl<'a, A: BasicAccess<'a>> DiagnosticSink<'a, A> for PublicSink {
        fn reject_basic(&self, access: &A, site: Site, rule: Rule<'a>) -> Self::Error {
            use super::basic_kernel::Field;
            let field = site.field.name();
            let error = match rule {
                Rule::Missing => ValidateError::MissingRequiredField(field.into()),
                Rule::UnsupportedScheme(scheme) => {
                    ValidateError::InvalidSecurity(format!("unsupported scheme '{}'", scheme))
                }
                Rule::ComboMissing => ValidateError::InvalidSecurity(
                    "combo schemes must define at least one of oneOf or allOf".into(),
                ),
                Rule::ComboCardinality => ValidateError::InvalidSecurity(format!(
                    "{} must contain at least two references",
                    field
                )),
                Rule::ComboEmpty => ValidateError::InvalidSecurity(format!(
                    "{} must not contain empty references",
                    field
                )),
                Rule::UnsupportedFlow(flow) => {
                    ValidateError::InvalidSecurity(format!("unsupported OAuth2 flow '{}'", flow))
                }
                Rule::Undefined(reference) => {
                    let context = match (site.owner.kind, site.field) {
                        (OwnerKind::Thing, Field::Security) => "Thing.security".into(),
                        (OwnerKind::SecurityDefinition, _) => {
                            format!("{}.{}", owner_name(access, site.owner), field)
                        }
                        (OwnerKind::Thing, Field::FormSecurity) => {
                            format!("Thing.forms.forms[{}].security", site.index)
                        }
                        (_, Field::FormSecurity) => format!(
                            "{}.forms[{}].security",
                            owner_name(access, site.owner),
                            site.index
                        ),
                        _ => unreachable!(),
                    };
                    return ValidateError::InvalidReference {
                        context,
                        reference: reference.into(),
                    };
                }
                Rule::Operation(op) => {
                    let (context, found) = if site.owner.kind == OwnerKind::Thing {
                        ("Thing.forms".into(), op.as_str().to_string())
                    } else {
                        (
                            format!("{}Affordance", kind_name(site.owner.kind)),
                            format!("{:?}", op),
                        )
                    };
                    ValidateError::InvalidOperation { context, found }
                }
            };
            if self.document && site.owner.kind != OwnerKind::Thing {
                crate::validate::prepend_context(owner_name(access, site.owner), error)
            } else {
                error
            }
        }
        fn schema_site(
            &self,
            access: &A,
            site: Site,
            key: Option<&'a str>,
            error: Self::Error,
        ) -> Self::Error {
            use super::basic_kernel::Field;
            let ValidateError::InvalidSchema(message) = error else {
                unreachable!()
            };
            let message = match (site.field, key) {
                (Field::PropertySchema, None) => message,
                (_, Some(key)) => format!("{}.{}: {}", site.field.name(), key, message),
                (_, None) => format!("{}: {}", site.field.name(), message),
            };
            let error = ValidateError::InvalidSchema(message);
            if self.document && site.owner.kind != OwnerKind::Thing {
                crate::validate::prepend_context(owner_name(access, site.owner), error)
            } else {
                error
            }
        }
    }
}

#[cfg(test)]
#[path = "../tests/support/basic_program_private.rs"]
mod basic_program_tests;
