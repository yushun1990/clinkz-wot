//! Non-production composition of the complete existing Thing Basic boundary.
//! Accessors expose facts; only this module chooses checks and discovery order.
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Owner {
    pub kind: OwnerKind,
    pub ordinal: u32,
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
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Site {
    pub owner: Owner,
    pub field: Field,
    /// Schema-map or original Form index, depending on the field.
    pub index: u32,
    /// Reference/operation index within its ordered sequence.
    pub member: u32,
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
pub enum RuleKind {
    Missing,
    UnsupportedScheme,
    ComboMissing,
    ComboCardinality,
    ComboEmpty,
    UnsupportedFlow,
    Undefined,
    Operation,
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

impl Rule<'_> {
    pub const fn kind(self) -> RuleKind {
        match self {
            Self::Missing => RuleKind::Missing,
            Self::UnsupportedScheme(_) => RuleKind::UnsupportedScheme,
            Self::ComboMissing => RuleKind::ComboMissing,
            Self::ComboCardinality => RuleKind::ComboCardinality,
            Self::ComboEmpty => RuleKind::ComboEmpty,
            Self::UnsupportedFlow(_) => RuleKind::UnsupportedFlow,
            Self::Undefined(_) => RuleKind::Undefined,
            Self::Operation(_) => RuleKind::Operation,
        }
    }
}

/// Shared with the existing default/security query witness. Explicit empty
/// sequences are preserved by callers; these helpers introduce no new policy.
pub fn default_property_operations(flags: (bool, bool)) -> &'static [Operation] {
    match flags {
        (true, false) => &[Operation::ReadProperty],
        (false, true) => &[Operation::WriteProperty],
        _ => &[Operation::ReadProperty, Operation::WriteProperty],
    }
}

pub fn inherited_security<N: Copy>(root: N, explicit: Option<N>) -> (N, bool) {
    explicit.map_or((root, true), |names| (names, false))
}

pub fn required_security(count: usize) -> Result<(), Rule<'static>> {
    if count == 0 {
        Err(Rule::Missing)
    } else {
        Ok(())
    }
}

pub fn combo_groups(scheme: &str) -> &'static [Field] {
    if scheme == "combo" {
        &[Field::OneOf, Field::AllOf]
    } else {
        &[]
    }
}

pub fn first_undefined<'a>(
    count: usize,
    name_at: impl Fn(usize) -> &'a str,
    exists: impl Fn(&str) -> bool,
) -> Option<(u32, &'a str)> {
    (0..count).find_map(|index| {
        let name = name_at(index);
        (!exists(name)).then(|| (index.try_into().unwrap(), name))
    })
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

    fn schemas(&self) -> &Self::Schemas;
    fn title(&self) -> Option<&'a str>;
    fn root_security(&self) -> Self::Names;
    fn names_count(&self, names: Self::Names) -> usize;
    fn name_at(&self, names: Self::Names, index: usize) -> &'a str;
    fn definition_count(&self) -> usize;
    fn definition_at(&self, index: usize) -> Self::Definition;
    fn definition_name(&self, index: usize) -> &'a str;
    fn definition_exists(&self, name: &str) -> bool;
    fn scheme(&self, definition: Self::Definition) -> &'a str;
    fn combo_names(&self, definition: Self::Definition, field: Field) -> Self::Names;
    fn security_string(&self, definition: Self::Definition, field: Field) -> Option<&'a str>;
    fn endpoint_present(&self, definition: Self::Definition, field: Field) -> bool;

    fn root_schema_map(&self, field: Field) -> Option<Self::SchemaMap>;
    fn schema_count(&self, map: Self::SchemaMap) -> usize;
    fn schema_at(
        &self,
        map: Self::SchemaMap,
        index: usize,
    ) -> <Self::Schemas as SchemaAccess<'a>>::Node;
    fn schema_name(&self, map: Self::SchemaMap, index: usize) -> &'a str;
    fn affordance_count(&self, kind: OwnerKind) -> usize;
    fn affordance_at(&self, kind: OwnerKind, index: usize) -> Self::Affordance;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InlineRule {
    Basic(RuleKind),
    Schema(schema_kernel::Rule),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InlineInvalid {
    pub site: Site,
    pub rule: InlineRule,
    /// Schema-local preorder ordinal, deliberately not a document-node ordinal.
    pub schema_ordinal: Option<u64>,
}

pub struct InlineSink;

impl<'a, A: SchemaAccess<'a>> schema_kernel::DiagnosticSink<'a, A> for InlineSink {
    type Error = InlineInvalid;
    fn reject(&self, _: &A, _: A::Node, ordinal: u64, rule: schema_kernel::Rule) -> Self::Error {
        InlineInvalid {
            site: Site::new(ROOT, Field::PropertySchema),
            rule: InlineRule::Schema(rule),
            schema_ordinal: Some(ordinal),
        }
    }
    fn child(&self, _: schema_kernel::ChildSite<'a>, error: Self::Error) -> Self::Error {
        error
    }
}

impl<'a, A: BasicAccess<'a>> DiagnosticSink<'a, A> for InlineSink {
    fn reject_basic(&self, _: &A, site: Site, rule: Rule<'a>) -> Self::Error {
        InlineInvalid {
            site,
            rule: InlineRule::Basic(rule.kind()),
            schema_ordinal: None,
        }
    }
    fn schema_site(
        &self,
        _: &A,
        site: Site,
        _: Option<&'a str>,
        mut error: Self::Error,
    ) -> Self::Error {
        error.site = site;
        error
    }
}

fn references<'a, A: BasicAccess<'a>, S: DiagnosticSink<'a, A>>(
    access: &A,
    names: A::Names,
    mut site: Site,
    sink: &S,
) -> Result<(), S::Error> {
    if let Some((index, reference)) = first_undefined(
        access.names_count(names),
        |i| access.name_at(names, i),
        |name| access.definition_exists(name),
    ) {
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
    match access.scheme(definition) {
        "combo" => {
            let one = access.combo_names(definition, Field::OneOf);
            let all = access.combo_names(definition, Field::AllOf);
            if access.names_count(one) == 0 && access.names_count(all) == 0 {
                return Err(reject(Field::Scheme, Rule::ComboMissing));
            }
            for (field, names) in [(Field::OneOf, one), (Field::AllOf, all)] {
                let count = access.names_count(names);
                if count == 1 {
                    return Err(reject(field, Rule::ComboCardinality));
                }
                for index in 0..count {
                    if access.name_at(names, index).is_empty() {
                        let mut site = Site::new(owner, field);
                        site.member = index.try_into().unwrap();
                        return Err(sink.reject_basic(access, site, Rule::ComboEmpty));
                    }
                }
            }
        }
        "apikey" => {
            if access
                .security_string(definition, Field::Name)
                .unwrap_or("")
                .is_empty()
            {
                return Err(reject(Field::Name, Rule::Missing));
            }
        }
        "oauth2" => match access
            .security_string(definition, Field::Flow)
            .unwrap_or("")
        {
            "code" => {
                for field in [Field::Authorization, Field::Token] {
                    if !access.endpoint_present(definition, field) {
                        return Err(reject(field, Rule::Missing));
                    }
                }
            }
            "client" | "device" => {}
            flow => return Err(reject(Field::Flow, Rule::UnsupportedFlow(flow))),
        },
        "nosec" | "auto" | "basic" | "digest" | "bearer" | "psk" => {}
        scheme => return Err(reject(Field::Scheme, Rule::UnsupportedScheme(scheme))),
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
        for index in 0..access.schema_count(map) {
            let mut site = Site::new(owner, field);
            site.index = index.try_into().unwrap();
            schema_kernel::validate(access.schemas(), access.schema_at(map, index), sink).map_err(
                |error| sink.schema_site(access, site, Some(access.schema_name(map, index)), error),
            )?;
        }
    }
    Ok(())
}

fn allowed(kind: OwnerKind, operation: Operation) -> bool {
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
                                index: index.try_into().unwrap(),
                                member: member.try_into().unwrap(),
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

pub fn validate_affordance<'a, A: BasicAccess<'a>, S: DiagnosticSink<'a, A>>(
    access: &A,
    affordance: A::Affordance,
    owner: Owner,
    sink: &S,
) -> Result<(), S::Error> {
    let schema = |field| {
        if let Some(node) = access.affordance_schema(affordance, field) {
            schema_kernel::validate(access.schemas(), node, sink)
                .map_err(|error| sink.schema_site(access, Site::new(owner, field), None, error))?;
        }
        Ok(())
    };
    if owner.kind == OwnerKind::Property {
        schema(Field::PropertySchema)?;
    }
    schema_map(
        access,
        access.uri_variables(affordance),
        owner,
        Field::UriVariables,
        sink,
    )?;
    let fields: &[Field] = match owner.kind {
        OwnerKind::Property => &[],
        OwnerKind::Action => &[Field::Input, Field::Output],
        OwnerKind::Event => &[
            Field::Subscription,
            Field::Data,
            Field::DataResponse,
            Field::Cancellation,
        ],
        _ => unreachable!(),
    };
    for &field in fields {
        schema(field)?;
    }
    operations(access, access.forms(Some(affordance)), owner, sink)
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
                site.index = index.try_into().unwrap();
                references(access, names, site, sink)?;
            }
        }
    }
    Ok(())
}

pub fn validate<'a, A: BasicAccess<'a>, S: DiagnosticSink<'a, A>>(
    access: &A,
    sink: &S,
) -> Result<(), S::Error> {
    if access.title().unwrap_or("").is_empty() {
        return Err(sink.reject_basic(access, Site::new(ROOT, Field::Title), Rule::Missing));
    }
    let security = access.root_security();
    required_security(access.names_count(security))
        .map_err(|rule| sink.reject_basic(access, Site::new(ROOT, Field::Security), rule))?;
    references(access, security, Site::new(ROOT, Field::Security), sink)?;
    for index in 0..access.definition_count() {
        let owner = Owner {
            kind: OwnerKind::SecurityDefinition,
            ordinal: index.try_into().unwrap(),
        };
        let definition = access.definition_at(index);
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
    for field in [Field::SchemaDefinitions, Field::UriVariables] {
        schema_map(access, access.root_schema_map(field), ROOT, field, sink)?;
    }
    for kind in [OwnerKind::Property, OwnerKind::Action, OwnerKind::Event] {
        for index in 0..access.affordance_count(kind) {
            let owner = Owner {
                kind,
                ordinal: index.try_into().unwrap(),
            };
            let affordance = access.affordance_at(kind, index);
            validate_affordance(access, affordance, owner, sink)?;
            form_security(access, access.forms(Some(affordance)), owner, sink)?;
        }
    }
    // Existing Thing Basic checks all root Form references before any root op.
    form_security(access, access.forms(None), ROOT, sink)?;
    operations(access, access.forms(None), ROOT, sink)
}
