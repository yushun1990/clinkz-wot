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
    count: usize,
    name_at: impl Fn(usize) -> &'a str,
    exists: impl Fn(&str) -> bool,
) -> Option<(usize, &'a str)> {
    (0..count).find_map(|index| {
        let name = name_at(index);
        (!exists(name)).then_some((index, name))
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
    match scheme_kind(access.scheme(definition)).map_err(|rule| reject(Field::Scheme, rule))? {
        Scheme::Combo => {
            let one = access.combo_names(definition, Field::OneOf);
            let all = access.combo_names(definition, Field::AllOf);
            combo_present(access.names_count(one), access.names_count(all))
                .map_err(|rule| reject(Field::Scheme, rule))?;
            for (field, names) in [(Field::OneOf, one), (Field::AllOf, all)] {
                let count = access.names_count(names);
                let empty = (0..count).find(|&i| access.name_at(names, i).is_empty());
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
        for index in 0..access.schema_count(map) {
            let mut site = Site::new(owner, field);
            site.index = index;
            schema_kernel::validate(access.schemas(), access.schema_at(map, index), sink).map_err(
                |error| sink.schema_site(access, site, Some(access.schema_name(map, index)), error),
            )?;
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

/// One discovery program for synchronous public Basic and paid canonical
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
                let fields = affordance_fields(owner.kind);
                match self.component {
                    0 => {
                        if owner.kind == OwnerKind::Property {
                            Action::Schema(owner, Field::PropertySchema)
                        } else {
                            Action::SchemaMap(owner, Field::UriVariables)
                        }
                    }
                    1 if owner.kind == OwnerKind::Property => {
                        Action::SchemaMap(owner, Field::UriVariables)
                    }
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
            7 => Action::FormSecurity(ROOT),
            8 => Action::Operations(ROOT),
            _ => Action::Done,
        }
    }
    pub fn advance(&mut self) {
        match self.stage {
            3 => self.index += 1,
            6 => {
                let kind = [OwnerKind::Property, OwnerKind::Action, OwnerKind::Event][self.kind];
                let last =
                    affordance_fields(kind).len() + if kind == OwnerKind::Property { 3 } else { 2 };
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
pub fn combo_group(count: usize, first_empty: Option<usize>) -> Result<(), (usize, Rule<'static>)> {
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

// Standalone public component entry in the source candidate; the Snapshot
// harness exercises the shared whole-Thing Walk instead.
#[allow(dead_code)]
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
    let fields = affordance_fields(owner.kind);
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
                site.index = index;
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
    let mut walk = Walk::default();
    let kinds = [OwnerKind::Property, OwnerKind::Action, OwnerKind::Event];
    loop {
        let action = walk.action(
            access.definition_count(),
            kinds.map(|kind| access.affordance_count(kind)),
        );
        match action {
            Action::Title => {
                required(!access.title().unwrap_or("").is_empty()).map_err(|rule| {
                    sink.reject_basic(access, Site::new(ROOT, Field::Title), rule)
                })?;
            }
            Action::RequiredSecurity => {
                required_security(access.names_count(access.root_security())).map_err(|rule| {
                    sink.reject_basic(access, Site::new(ROOT, Field::Security), rule)
                })?
            }
            Action::RootReferences => references(
                access,
                access.root_security(),
                Site::new(ROOT, Field::Security),
                sink,
            )?,
            Action::Definition(owner) => {
                let definition = access.definition_at(owner.ordinal);
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
                    access.uri_variables(access.affordance_at(owner.kind, owner.ordinal))
                };
                schema_map(access, map, owner, field, sink)?;
            }
            Action::Schema(owner, field) => {
                if let Some(node) =
                    access.affordance_schema(access.affordance_at(owner.kind, owner.ordinal), field)
                {
                    schema_kernel::validate(access.schemas(), node, sink).map_err(|error| {
                        sink.schema_site(access, Site::new(owner, field), None, error)
                    })?;
                }
            }
            Action::Operations(owner) | Action::FormSecurity(owner) => {
                let forms = access.forms(
                    (owner.kind != OwnerKind::Thing)
                        .then(|| access.affordance_at(owner.kind, owner.ordinal)),
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
