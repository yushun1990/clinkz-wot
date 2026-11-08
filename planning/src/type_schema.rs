//! Pure preparation over the existing Host-only resolved semantic loan.
//! Constraint meaning is independent of an operation's execution lifetime.
//! Concrete participants can require a narrower facet of that meaning.
use clinkz_wot_core::{
    CoreError, CoreResult, ErrorContext, ErrorPhase, RetryClass,
    data_schema::{PreparedSchema, ValueType},
};
use clinkz_wot_td::semantic::{
    SchemaConstraints, SchemaView, SemanticName, SemanticNode, SemanticValue,
};

pub(crate) fn event_member_count(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
) -> CoreResult<usize> {
    let count = doc.collection_events().ok_or_else(refused)?.count();
    if count > clinkz_wot_core::collection::EventCollectionSchema::MAX_MEMBERS {
        return Err(refused());
    }
    Ok(count)
}
/// All exact Event names and four independent roles; no individual Forms.
pub fn prepare_event_collection_schema(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
) -> CoreResult<clinkz_wot_core::collection::EventCollectionSchema> {
    use clinkz_wot_core::collection::{EventCollectionSchema as S, EventMemberSchema};
    let mut preparation = SchemaPreparation::default();
    for event in doc.collection_events().ok_or_else(refused)? {
        for role in ["subscription", "data", "dataResponse", "cancellation"] {
            if let Some(s) =
                doc.affordance_schema(clinkz_wot_td::semantic::FormOwner::Event(event.name), role)
            {
                preparation.include(s)?;
            }
        }
    }
    if preparation.nodes > S::MAX_SCHEMA_NODES
        || preparation.backing_bytes > S::SCHEMA_BACKING_BOUND
    {
        return Err(refused());
    }
    let mut members = alloc::vec::Vec::new();
    members
        .try_reserve_exact(event_member_count(doc)?)
        .map_err(|_| clinkz_wot_core::consumer::busy())?;
    let mut bytes = 0;
    for event in doc.collection_events().ok_or_else(refused)? {
        bytes += event.name.len();
        if event.name.len() > S::MAX_NAME_BYTES || bytes > S::MAX_TOTAL_NAME_BYTES {
            return Err(refused());
        }
        let role = |name| {
            doc.affordance_schema(clinkz_wot_td::semantic::FormOwner::Event(event.name), name)
                .map(prepare_schema)
                .transpose()
        };
        members.push(EventMemberSchema {
            name: event.name.into(),
            subscription: role("subscription")?,
            data: role("data")?,
            data_response: role("dataResponse")?,
            cancellation: role("cancellation")?,
        });
    }
    S::try_new(members)
}

/// Select WoT data roles, independently of protocol and execution lifetime.
/// Callers fund the borrowed census, construction and final owner before entry.
pub fn prepare_notification_schemas(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
    target: &clinkz_wot_core::AffordanceTarget,
    kind: clinkz_wot_core::notification::NotificationKind,
) -> CoreResult<clinkz_wot_core::notification::NotificationSchemas> {
    use alloc::sync::Arc;
    use clinkz_wot_core::{
        notification::NotificationKind as N, notification::NotificationSchemas as S,
    };
    let owner = notification_owner(target, kind)?;
    Ok(match kind {
        N::PropertyObservation => {
            let p = doc.property();
            if !p.observable || p.schema.facts().write_only {
                return Err(refused());
            }
            S::PropertyValue(prepare_schema(p.schema)?)
        }
        N::Event => {
            let role = |r| {
                doc.affordance_schema(owner, r)
                    .map(prepare_schema)
                    .transpose()
            };
            S::Event {
                subscription: role("subscription")?,
                data: role("data")?,
                data_response: role("dataResponse")?,
                cancellation: role("cancellation")?,
            }
        }
        N::ObserveAllProperties => {
            S::ObserveAllProperties(Arc::new(prepare_readable_property_schema(doc)?))
        }
        N::SubscribeAllEvents => {
            S::SubscribeAllEvents(Arc::new(prepare_event_collection_schema(doc)?))
        }
    })
}

fn notification_owner(
    target: &clinkz_wot_core::AffordanceTarget,
    kind: clinkz_wot_core::notification::NotificationKind,
) -> CoreResult<clinkz_wot_td::semantic::FormOwner<'_>> {
    use clinkz_wot_core::AffordanceTarget as T;
    use clinkz_wot_td::semantic::FormOwner as O;
    if !kind.accepts_target(target) {
        return Err(refused());
    }
    Ok(match target {
        T::Thing => O::Thing,
        T::Property(n) => O::Property(n),
        T::Event(n) => O::Event(n),
        _ => return Err(refused()),
    })
}

pub(crate) fn notification_preparation(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
    target: &clinkz_wot_core::AffordanceTarget,
    kind: clinkz_wot_core::notification::NotificationKind,
) -> CoreResult<SchemaPreparation> {
    use clinkz_wot_core::notification::NotificationKind as N;
    use clinkz_wot_td::semantic::FormOwner;
    let owner = notification_owner(target, kind)?;
    let mut p = SchemaPreparation::default();
    match kind {
        N::PropertyObservation => p.include(doc.property().schema)?,
        N::Event => {
            for role in ["subscription", "data", "dataResponse", "cancellation"] {
                if let Some(s) = doc.affordance_schema(owner, role) {
                    p.include(s)?;
                }
            }
        }
        N::ObserveAllProperties => {
            for member in doc.collection_properties().ok_or_else(refused)? {
                if !member.schema.facts().write_only {
                    p.include(member.schema)?;
                }
            }
            if p.nodes > PreparedSchema::MAX_NODES {
                return Err(refused());
            }
        }
        N::SubscribeAllEvents => {
            for member in doc.collection_events().ok_or_else(refused)? {
                for role in ["subscription", "data", "dataResponse", "cancellation"] {
                    if let Some(s) = doc.affordance_schema(FormOwner::Event(member.name), role) {
                        p.include(s)?;
                    }
                }
            }
            use clinkz_wot_core::collection::EventCollectionSchema as S;
            if p.nodes > S::MAX_SCHEMA_NODES || p.backing_bytes > S::SCHEMA_BACKING_BOUND {
                return Err(refused());
            }
        }
    }
    Ok(p)
}

/// Full bounded pure prepare/compare/disposal credit for a native callback.
/// It does not qualify that callback's media, security or native semantics.
pub fn notification_preparation_work(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
    target: &clinkz_wot_core::AffordanceTarget,
    kind: clinkz_wot_core::notification::NotificationKind,
) -> CoreResult<u64> {
    let p = notification_preparation(doc, target, kind)?;
    Ok(p.work + p.cleanup + 4096)
}

/// Final qualification borrows the original role constraints without rebuilding
/// trees. Collection identity and missing versus unconstrained roles stay exact.
pub fn matches_notification_schemas(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
    target: &clinkz_wot_core::AffordanceTarget,
    expected: &clinkz_wot_core::notification::NotificationSchemas,
) -> CoreResult<bool> {
    use clinkz_wot_core::notification::NotificationSchemas as S;
    use clinkz_wot_td::semantic::FormOwner;
    let owner = notification_owner(target, expected.kind())?;
    let roles = |owner, expected: [Option<&PreparedSchema>; 4]| -> CoreResult<bool> {
        for (role, expected) in ["subscription", "data", "dataResponse", "cancellation"]
            .into_iter()
            .zip(expected)
        {
            match (doc.affordance_schema(owner, role), expected) {
                (Some(a), Some(e)) if matches_schema(a, e)? => {}
                (None, None) => {}
                _ => return Ok(false),
            }
        }
        Ok(true)
    };
    match expected {
        S::PropertyValue(s) => Ok(doc.property().observable
            && !doc.property().schema.facts().write_only
            && matches_schema(doc.property().schema, s)?),
        S::Event { .. } => roles(
            owner,
            [
                expected.subscription(),
                expected.data(),
                expected.data_response(),
                expected.cancellation(),
            ],
        ),
        S::ObserveAllProperties(s) => matches_property_schema(doc, s, false),
        S::SubscribeAllEvents(s) => {
            let mut actual = doc.collection_events().ok_or_else(refused)?;
            for member in s.members() {
                let Some(a) = actual.next() else {
                    return Ok(false);
                };
                if a.name != member.name.as_ref()
                    || !roles(
                        FormOwner::Event(a.name),
                        [
                            member.subscription.as_ref(),
                            member.data.as_ref(),
                            member.data_response.as_ref(),
                            member.cancellation.as_ref(),
                        ],
                    )?
                {
                    return Ok(false);
                }
            }
            Ok(actual.next().is_none())
        }
    }
}

// Borrowed census lets callers prepay construction and destruction of each
// role-specific schema, including partial-construction paths.
#[derive(Clone, Copy)]
enum Members {
    Readable,
    Writable,
}
impl Members {
    fn includes(self, schema: SchemaView<'_>) -> bool {
        let facts = schema.facts();
        match self {
            Self::Readable => !facts.write_only,
            Self::Writable => !facts.read_only,
        }
    }
}
fn member_count(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
    role: Members,
) -> CoreResult<usize> {
    let count = doc
        .collection_properties()
        .ok_or_else(refused)?
        .filter(|p| role.includes(p.schema))
        .count();
    if count > clinkz_wot_core::collection::PropertyCollectionSchema::MAX_MEMBERS {
        return Err(refused());
    }
    Ok(count)
}
pub(crate) fn readable_member_count(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
) -> CoreResult<usize> {
    member_count(doc, Members::Readable)
}
pub(crate) fn writable_member_count(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
) -> CoreResult<usize> {
    member_count(doc, Members::Writable)
}
/// TD 1.1: every non-writeOnly Property and its complete supported schema.
/// No individual Form, behavior or native read is consulted.
pub fn prepare_readable_property_schema(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
) -> CoreResult<clinkz_wot_core::collection::PropertyCollectionSchema> {
    prepare_members(doc, Members::Readable)
}
/// TD 1.1: every non-readOnly Property, including writeOnly members. The
/// invocation's all/selected scope is separate from this immutable eligibility.
pub fn prepare_writable_property_schema(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
) -> CoreResult<clinkz_wot_core::collection::PropertyCollectionSchema> {
    prepare_members(doc, Members::Writable)
}

/// Compare borrowed assertions against retained meaning. This deliberately
/// performs qualification again, but allocates no member/name table. Unsupported
/// assertions refuse even when their declared scalar type would compare equal.
pub fn matches_property_schema(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
    expected: &clinkz_wot_core::collection::PropertyCollectionSchema,
    writing: bool,
) -> CoreResult<bool> {
    let role = if writing {
        Members::Writable
    } else {
        Members::Readable
    };
    let mut actual = doc
        .collection_properties()
        .ok_or_else(refused)?
        .filter(|p| role.includes(p.schema));
    for member in expected.members() {
        let Some(property) = actual.next() else {
            return Ok(false);
        };
        if property.name != member.name.as_ref()
            || !matches_schema(property.schema, &member.schema)?
        {
            return Ok(false);
        }
    }
    Ok(actual.next().is_none())
}
fn prepare_members(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
    role: Members,
) -> CoreResult<clinkz_wot_core::collection::PropertyCollectionSchema> {
    use clinkz_wot_core::collection::{PropertyCollectionSchema as S, PropertyMemberSchema};
    let mut total_nodes = 0;
    let mut total_names = 0;
    for property in doc.collection_properties().ok_or_else(refused)? {
        if !role.includes(property.schema) {
            continue;
        }
        let census = schema_census(property.schema)?;
        total_nodes += census.nodes;
        total_names += census.names;
        if total_nodes > PreparedSchema::MAX_NODES
            || total_names > PreparedSchema::MAX_TOTAL_NAME_BYTES
        {
            return Err(refused());
        }
    }
    let mut members = alloc::vec::Vec::new();
    members
        .try_reserve_exact(member_count(doc, role)?)
        .map_err(|_| clinkz_wot_core::consumer::busy())?;
    let mut bytes = 0;
    for p in doc.collection_properties().ok_or_else(refused)? {
        if !role.includes(p.schema) {
            continue;
        }
        bytes += p.name.len();
        if members.len() == S::MAX_MEMBERS
            || p.name.len() > S::MAX_NAME_BYTES
            || bytes > S::MAX_TOTAL_NAME_BYTES
        {
            return Err(refused());
        }
        let schema = prepare_schema(p.schema)?;
        members.push(PropertyMemberSchema {
            name: p.name.into(),
            schema,
        });
    }
    S::try_new(members)
}

/// The common supported assertions, resolved by TD and retained without source
/// loans. Pure callbacks run inside their caller's prepaid preparation envelope.
pub fn prepare_schema(schema: SchemaView<'_>) -> CoreResult<PreparedSchema> {
    schema_census(schema)?;
    build_schema(schema)
}
/// A concrete participant's explicitly narrower admission subset. This is the
/// same interpretation, not a restriction inherent to continuing interactions.
pub fn prepare_type_schema(schema: SchemaView<'_>) -> CoreResult<PreparedSchema> {
    match schema_head(schema)? {
        Some(s) if s.is_type_only() => Ok(s),
        _ => Err(refused()),
    }
}

fn schema_head(schema: SchemaView<'_>) -> CoreResult<Option<PreparedSchema>> {
    let f = schema.facts();
    if f.declared_type.is_some_and(|name| {
        !matches!(
            name,
            "null" | "boolean" | "number" | "integer" | "string" | "array" | "object"
        )
    }) {
        return Err(refused());
    }
    if f.constant.is_some()
        || f.enumeration.is_some()
        || f.format.is_some()
        || schema.one_of().is_some()
        || schema.items().is_some()
        || !schema.extension_semantics().all(annotation)
    {
        return Err(refused());
    }
    let leaf = match f.constraints {
        SchemaConstraints::Object { required }
            if schema.properties().is_some() || required.is_some() =>
        {
            if f.declared_type != Some("object")
                || required.is_some_and(|names| names.len() > PreparedSchema::MAX_PROPERTIES)
            {
                return Err(refused());
            }
            return Ok(None);
        }
        SchemaConstraints::Integer {
            minimum,
            exclusive_minimum,
            maximum,
            exclusive_maximum,
            multiple_of,
        } => {
            if multiple_of.is_some() {
                return Err(refused());
            }
            if [minimum, exclusive_minimum, maximum, exclusive_maximum]
                .iter()
                .any(Option::is_some)
            {
                if f.declared_type != Some("integer") {
                    return Err(refused());
                }
                return Ok(Some(PreparedSchema::Integer {
                    minimum,
                    exclusive_minimum,
                    maximum,
                    exclusive_maximum,
                }));
            }
            ValueType::Integer
        }
        SchemaConstraints::Number {
            minimum,
            exclusive_minimum,
            maximum,
            exclusive_maximum,
            multiple_of,
        } => {
            // NumberSchema's upstream f64 bounds cannot prove exact decimal
            // constraints. Refuse instead of reconstructing a rounded bound.
            if [
                minimum,
                exclusive_minimum,
                maximum,
                exclusive_maximum,
                multiple_of,
            ]
            .iter()
            .any(Option::is_some)
            {
                return Err(refused());
            }
            ValueType::Number
        }
        SchemaConstraints::String {
            min_length,
            max_length,
            pattern,
            content_encoding,
            content_media_type,
        } => {
            if pattern.is_some() || content_encoding.is_some() || content_media_type.is_some() {
                return Err(refused());
            }
            if min_length.is_some() || max_length.is_some() {
                if f.declared_type != Some("string") {
                    return Err(refused());
                }
                return Ok(Some(PreparedSchema::String {
                    min_length,
                    max_length,
                }));
            }
            ValueType::String
        }
        SchemaConstraints::Array {
            min_items,
            max_items,
        } => {
            if min_items.is_some() || max_items.is_some() {
                return Err(refused());
            }
            ValueType::Array
        }
        SchemaConstraints::Object { .. } => ValueType::Object,
        SchemaConstraints::Boolean => ValueType::Boolean,
        SchemaConstraints::Null => ValueType::Null,
    };
    Ok(Some(if f.declared_type.is_none() {
        PreparedSchema::Any
    } else {
        PreparedSchema::Type(leaf)
    }))
}

#[derive(Default)]
struct SchemaCensus {
    nodes: usize,
    names: usize,
}
fn required(schema: SchemaView<'_>) -> &[alloc::string::String] {
    match schema.facts().constraints {
        SchemaConstraints::Object { required } => required.unwrap_or(&[]),
        _ => &[],
    }
}
/// Complete bounded recursive/name-walk credit for a pure schema callback.
pub fn preparation_work(schema: SchemaView<'_>) -> CoreResult<u64> {
    let mut preparation = SchemaPreparation::default();
    preparation.include(schema)?;
    Ok(preparation.work)
}
/// Borrowed census shared by finite and continuing preparation. It owns no
/// schemas, services or authority. Callers fund construction, rollback and the
/// final original owner before building any variable constraint storage.
#[derive(Default)]
pub(crate) struct SchemaPreparation {
    pub cleanup: u64,
    pub work: u64,
    pub nodes: usize,
    pub backing_bytes: usize,
}
impl SchemaPreparation {
    pub fn include(&mut self, schema: SchemaView<'_>) -> CoreResult<()> {
        let c = schema_census(schema)?;
        if schema_head(schema)?.is_none() {
            self.cleanup += c.nodes as u64 * 2;
            // One entry per child plus decoded names. Counting the root too is
            // a conservative bound, including partially constructed trees.
            self.backing_bytes += c.nodes
                * core::mem::size_of::<clinkz_wot_core::data_schema::SchemaProperty>()
                + c.names;
        }
        self.work += (c.nodes * 8 + c.names * PreparedSchema::MAX_PROPERTIES) as u64;
        self.nodes += c.nodes;
        Ok(())
    }
}
fn schema_census(schema: SchemaView<'_>) -> CoreResult<SchemaCensus> {
    fn walk(s: SchemaView<'_>, depth: usize, c: &mut SchemaCensus) -> CoreResult<()> {
        if depth > PreparedSchema::MAX_DEPTH {
            return Err(refused());
        }
        c.nodes += 1;
        if c.nodes > PreparedSchema::MAX_NODES {
            return Err(refused());
        }
        if schema_head(s)?.is_some() {
            return Ok(());
        }
        let mut fields = 0;
        for (name, child) in s.properties().into_iter().flatten() {
            c.names += name.len();
            fields += 1;
            if name.len() > PreparedSchema::MAX_NAME_BYTES
                || fields > PreparedSchema::MAX_PROPERTIES
                || c.names > PreparedSchema::MAX_TOTAL_NAME_BYTES
            {
                return Err(refused());
            }
            walk(child, depth + 1, c)?;
        }
        for name in required(s) {
            if s.properties().into_iter().flatten().any(|(n, _)| n == name) {
                continue;
            }
            fields += 1;
            c.names += name.len();
            c.nodes += 1;
            if fields > PreparedSchema::MAX_PROPERTIES
                || name.len() > PreparedSchema::MAX_NAME_BYTES
                || c.names > PreparedSchema::MAX_TOTAL_NAME_BYTES
                || c.nodes > PreparedSchema::MAX_NODES
            {
                return Err(refused());
            }
        }
        Ok(())
    }
    let mut census = SchemaCensus::default();
    walk(schema, 1, &mut census)?;
    Ok(census)
}
fn build_schema(schema: SchemaView<'_>) -> CoreResult<PreparedSchema> {
    if let Some(leaf) = schema_head(schema)? {
        return Ok(leaf);
    }
    let required = required(schema);
    let count = schema.properties().into_iter().flatten().count()
        + required
            .iter()
            .filter(|name| {
                !schema
                    .properties()
                    .into_iter()
                    .flatten()
                    .any(|(n, _)| n == name.as_str())
            })
            .count();
    let mut fields = alloc::vec::Vec::new();
    fields
        .try_reserve_exact(count)
        .map_err(|_| clinkz_wot_core::consumer::busy())?;
    for (name, child) in schema.properties().into_iter().flatten() {
        fields.push(clinkz_wot_core::data_schema::SchemaProperty {
            name: name.into(),
            required: required.iter().any(|n| n == name),
            schema: build_schema(child)?,
        });
    }
    for name in required {
        if fields.iter().any(|p| p.name.as_ref() == name) {
            continue;
        }
        fields.push(clinkz_wot_core::data_schema::SchemaProperty {
            name: name.as_str().into(),
            required: true,
            schema: PreparedSchema::Any,
        });
    }
    PreparedSchema::object(fields).map_err(|_| refused())
}

/// Qualification only: compare borrowed facts without compiling or allocating a
/// second constraint tree. Required names are a set, not a source-order rule.
pub fn matches_schema(actual: SchemaView<'_>, expected: &PreparedSchema) -> CoreResult<bool> {
    schema_census(actual)?;
    fn compare(a: SchemaView<'_>, e: &PreparedSchema) -> CoreResult<bool> {
        if let Some(leaf) = schema_head(a)? {
            return Ok(&leaf == e);
        }
        let PreparedSchema::Object(object) = e else {
            return Ok(false);
        };
        let required = required(a);
        let mut seen = 0;
        for (name, child) in a.properties().into_iter().flatten() {
            let Some(field) = object.properties().iter().find(|f| f.name.as_ref() == name) else {
                return Ok(false);
            };
            if field.required != required.iter().any(|n| n == name)
                || !compare(child, &field.schema)?
            {
                return Ok(false);
            }
            seen += 1;
        }
        for name in required {
            if a.properties().into_iter().flatten().any(|(n, _)| n == name) {
                continue;
            }
            if !object
                .properties()
                .iter()
                .any(|f| f.name.as_ref() == name && f.required && f.schema == PreparedSchema::Any)
            {
                return Ok(false);
            }
            seen += 1;
        }
        Ok(seen == object.properties().len())
    }
    compare(actual, expected)
}

fn annotation(node: SemanticNode<'_>) -> bool {
    node.name() == SemanticName::Keyword("@type")
        && (matches!(node.value(), SemanticValue::Iri(_))
            || node.value() == SemanticValue::Array
                && node
                    .children()
                    .all(|n| matches!(n.value(), SemanticValue::Iri(_))))
}

fn refused() -> CoreError {
    CoreError::UnsupportedOperation(
        ErrorContext::new(ErrorPhase::Admission, RetryClass::Never).with_redacted_cause(
            624,
            "unsupported DataSchema assertion or preparation profile",
        ),
    )
}

/// Action definitions are not status records. Retain each output role and
/// reject duplicate expanded role assertions before selecting a status codec.
pub fn prepare_action_collection_schema(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
) -> CoreResult<clinkz_wot_core::action::ActionCollectionSchema> {
    use clinkz_wot_core::action::{ActionCollectionSchema as S, ActionMemberSchema};
    use clinkz_wot_td::semantic::{FormOwner, SemanticScope};
    let mut members = alloc::vec::Vec::new();
    members
        .try_reserve_exact(action_member_count(doc)?)
        .map_err(|_| clinkz_wot_core::consumer::busy())?;
    let mut bytes = 0;
    for name in doc.collection_actions().ok_or_else(refused)? {
        bytes += name.len();
        if name.len() > 256 || bytes > S::MAX_TOTAL_NAME_BYTES {
            return Err(refused());
        }
        let output = doc.affordance_schema(FormOwner::Action(name), "output");
        let scope = doc.scope(SemanticScope::Action(name)).ok_or_else(refused)?;
        // Input has no data role in a query. Its actual schema need not be
        // executable by this Consumer, but cannot masquerade as another output.
        if scope
            .children()
            .filter(|n| n.name().is("https://www.w3.org/2019/wot/td#output"))
            .count()
            != usize::from(output.is_some())
            || scope.children().any(|n| {
                !n.name().is("https://www.w3.org/2019/wot/td#input")
                    && !n.name().is("https://www.w3.org/2019/wot/td#output")
                    && !annotation(n)
            })
        {
            return Err(refused());
        }
        members.push(ActionMemberSchema {
            name: name.into(),
            output: output.map(prepare_type_schema).transpose()?,
        });
    }
    S::try_new(members)
}
pub(crate) fn action_member_count(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
) -> CoreResult<usize> {
    let count = doc.collection_actions().ok_or_else(refused)?.count();
    if count > clinkz_wot_core::action::ActionCollectionSchema::MAX_MEMBERS {
        return Err(refused());
    }
    Ok(count)
}
