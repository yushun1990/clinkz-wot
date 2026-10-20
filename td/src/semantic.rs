//! Bounded, borrowed semantic extensions for Host Consumer compilation.
//!
//! This is deliberately a local-context subset, not a JSON-LD processor. Remote
//! extension contexts, containers, reverse properties and unsupported coercions
//! are refused rather than guessed. No context or compound raw JSON is lent to
//! a protocol compiler. All names and IRI-valued strings are resolved here once.

use crate::{
    context::{ContextEntry, WOT_CONTEXT_1_0, WOT_CONTEXT_1_1},
    data_type::ExtensionMap,
    thing::Thing,
};
use alloc::{alloc::alloc, collections::BTreeMap, string::String, vec::Vec};
use core::{alloc::Layout, mem::size_of};
use serde_json::{Map, Number, Value};

mod facts;
pub use facts::*;

const NODES: usize = 128;
const SCOPES: usize = 32;
const DEPTH: usize = 16;
#[cfg(test)]
#[path = "semantic_tests.rs"]
mod tests;
/// Maximum atomic resolution work, including context lookup and string bytes.
pub const RESOLUTION_WORK: u64 = 65_536;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticError {
    UnsupportedContext,
    Limit,
    Allocation,
    UnsupportedDependency,
}

/// An expanded IRI in two borrowed segments; no protocol meaning is assigned.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticIri<'a> {
    prefix: &'a str,
    suffix: &'a str,
}
impl SemanticIri<'_> {
    pub fn is(&self, iri: &str) -> bool {
        iri.strip_prefix(self.prefix) == Some(self.suffix)
    }
    pub fn segments(&self) -> (&str, &str) {
        (self.prefix, self.suffix)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SemanticValue<'a> {
    Null,
    Bool(bool),
    Number(&'a Number),
    Text(&'a str),
    Iri(SemanticIri<'a>),
    Array,
    Object,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticName<'a> {
    Iri(SemanticIri<'a>),
    Keyword(&'a str),
    Unmapped(&'a str),
    Item,
}
impl SemanticName<'_> {
    pub fn is(&self, iri: &str) -> bool {
        matches!(self, Self::Iri(i) if i.is(iri))
    }
}

#[derive(Clone, Copy, Debug)]
struct Node<'a> {
    name: SemanticName<'a>,
    value: SemanticValue<'a>,
    end: usize,
    schema: Option<&'a crate::data_schema::DataSchema>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticScope<'a> {
    Thing,
    Property(&'a str),
    Form {
        property: &'a str,
        index: u32,
    },
    Response {
        property: &'a str,
        index: u32,
    },
    AdditionalResponse {
        property: &'a str,
        form: u32,
        index: u32,
    },
    Security(&'a str),
    SchemaDefinition(&'a str),
    UriVariable {
        property: Option<&'a str>,
        name: &'a str,
    },
}
struct Scope<'a> {
    key: SemanticScope<'a>,
    root: usize,
    form: Option<&'a crate::form::Form>,
}

/// Standard Form facts kept distinct from its extension tree.
pub struct ResolvedFormFacts<'a> {
    pub href: &'a str,
    pub content_type: &'a str,
    pub subprotocol: Option<&'a str>,
    pub declared_operations: Option<&'a [crate::data_type::Operation]>,
    pub effective_operations: &'a [crate::data_type::Operation],
    pub declared_security: Option<&'a [String]>,
    pub effective_security: &'a [String],
    pub content_coding: Option<&'a str>,
    pub scopes: Option<&'a [String]>,
    pub response_content_type: Option<&'a str>,
    pub additional_responses: bool,
}

/// A resolved node. Child order, nulls and explicit empty values survive.
#[derive(Clone, Copy)]
pub struct SemanticNode<'a> {
    nodes: &'a [Node<'a>],
    index: usize,
}
impl<'a> SemanticNode<'a> {
    pub fn name(self) -> SemanticName<'a> {
        self.nodes[self.index].name
    }
    pub fn value(self) -> SemanticValue<'a> {
        self.nodes[self.index].value
    }
    pub fn children(self) -> impl Iterator<Item = Self> + 'a {
        let mut next = self.index + 1;
        let end = self.nodes[self.index].end;
        core::iter::from_fn(move || {
            if next >= end {
                return None;
            }
            let index = next;
            next = self.nodes[index].end;
            Some(Self {
                nodes: self.nodes,
                index,
            })
        })
    }
    pub fn field(self, iri: &str) -> Option<Self> {
        self.children().find(|n| n.name().is(iri))
    }
}

/// Temporary semantic closure of one Form, its property, Thing and effective
/// security definitions. Other Forms/properties are not present. Every scalar
/// borrows the admitted immutable TD;
/// artifacts must copy the native instructions they need before this is freed.
pub struct ResolvedSemanticDocument<'a> {
    nodes: Vec<Node<'a>>,
    scopes: Vec<Scope<'a>>,
    selected: Option<Selected<'a>>,
}
struct Selected<'a> {
    thing: &'a Thing,
    property: &'a crate::affordance::PropertyAffordance,
    name: &'a str,
}
impl<'a> ResolvedSemanticDocument<'a> {
    pub const fn capacity_bytes() -> u64 {
        (NODES * size_of::<Node<'a>>() + SCOPES * size_of::<Scope<'a>>()) as u64
    }
    pub fn scope(&self, scope: SemanticScope<'_>) -> Option<SemanticNode<'_>> {
        self.scopes
            .iter()
            .find(|s| s.key == scope)
            .map(|s| SemanticNode {
                nodes: &self.nodes,
                index: s.root,
            })
    }
    pub fn form_facts(&self, property: &str, index: u32) -> Option<ResolvedFormFacts<'_>> {
        let form = self
            .scopes
            .iter()
            .find(|s| s.key == SemanticScope::Form { property, index })?
            .form?;
        Some(ResolvedFormFacts {
            href: form.href.as_str(),
            content_type: &form.content_type,
            subprotocol: form.subprotocol.as_deref(),
            declared_operations: form.op.as_deref(),
            effective_operations: form.op.as_deref().unwrap_or_else(|| {
                crate::td_defaults::default_operations_for_context(
                    crate::td_defaults::FormContext::Property(
                        self.selected.as_ref().unwrap().property,
                    ),
                )
            }),
            declared_security: form.security.as_deref(),
            effective_security: crate::td_defaults::effective_form_security(
                self.selected.as_ref().unwrap().thing,
                form,
            ),
            content_coding: form.content_coding.as_deref(),
            scopes: form.scopes.as_deref(),
            response_content_type: form.response.as_ref().map(|r| r.content_type.as_str()),
            additional_responses: form.additional_responses.is_some(),
        })
    }
    #[cfg(test)]
    pub(crate) fn resolve(
        thing: &'a Thing,
        property: &str,
        index: u32,
    ) -> Result<Self, SemanticError> {
        Self::resolve_metered(thing, property, index).0
    }
    pub(crate) fn resolve_metered(
        thing: &'a Thing,
        property: &str,
        index: u32,
    ) -> (Result<Self, SemanticError>, u64) {
        let mut resolver = match (buffer(NODES), buffer(SCOPES)) {
            (Ok(nodes), Ok(scopes)) => Resolver {
                document: Self {
                    nodes,
                    scopes,
                    selected: None,
                },
                fuel: RESOLUTION_WORK,
            },
            _ => return (Err(SemanticError::Allocation), 0),
        };
        let result = resolver.prepare(thing, property, index);
        let work = RESOLUTION_WORK - resolver.fuel;
        (result.map(|()| resolver.document), work)
    }
}
impl<'a> Resolver<'a> {
    fn prepare(
        &mut self,
        thing: &'a Thing,
        selected_property: &str,
        selected_form: u32,
    ) -> Result<(), SemanticError> {
        let resolver = self;
        resolver.metadata_cost(&thing._metadata)?;
        if let Some(id) = &thing.id {
            resolver.pay(id.as_str().len() as u64 + 1)?;
        }
        let mut context = ActiveContext::new("https://www.w3.org/2019/wot/td#");
        let mut seeded = false;
        for entry in thing.context.admission_entries() {
            resolver.pay(1)?;
            match entry {
                ContextEntry::Uri(uri)
                    if !seeded && matches!(uri.as_str(), WOT_CONTEXT_1_0 | WOT_CONTEXT_1_1) =>
                {
                    seeded = true;
                }
                ContextEntry::Uri(_) => return Err(SemanticError::UnsupportedContext),
                ContextEntry::Object(map) => {
                    if !seeded {
                        return Err(SemanticError::UnsupportedContext);
                    }
                    context.push(ContextMap::Typed(map))?;
                    for (key, value) in map {
                        resolver.context_definition(&mut context, key, value)?;
                    }
                    context.maps[context.len - 1].as_mut().unwrap().vocab = context.vocab;
                }
            }
        }
        let root_context = resolver.local(context, &thing._extra_fields)?;
        resolver.scope(
            SemanticScope::Thing,
            &thing._extra_fields,
            thing._metadata.tags.as_deref(),
            root_context,
        )?;
        // Profile advertisement is retained without asserting profile conformance.
        if let Some(profiles) = &thing.profile {
            let root = resolver.document.scopes[0].root;
            let list = resolver.push(
                SemanticName::Iri(SemanticIri {
                    prefix: "https://www.w3.org/2019/wot/td#",
                    suffix: "followsProfile",
                }),
                SemanticValue::Array,
            )?;
            for profile in profiles {
                resolver.push(
                    SemanticName::Item,
                    SemanticValue::Iri(SemanticIri {
                        prefix: profile.as_str(),
                        suffix: "",
                    }),
                )?;
            }
            resolver.finish(list);
            resolver.finish(root);
        }
        let (name, property) = thing
            .properties
            .as_ref()
            .and_then(|properties| properties.get_key_value(selected_property))
            .ok_or(SemanticError::UnsupportedContext)?;
        resolver.pay(name.len() as u64 + 1)?;
        let schema = property._schema.context();
        let mut context = root_context;
        context.vocab = "https://www.w3.org/2019/wot/json-schema#";
        let context = resolver.local(context, &schema._extra_fields)?;
        resolver.scope(
            SemanticScope::Property(name),
            &schema._extra_fields,
            schema._metadata.tags.as_deref(),
            context,
        )?;
        let root = resolver.document.scopes.last().unwrap().root;
        resolver.document.nodes[root].schema = Some(&property._schema);
        resolver.schema_facts(&property._schema, 0, context)?;
        resolver.schema_children(&property._schema, context, 0)?;
        resolver.finish(root);
        let index = selected_form;
        let form = property
            ._interaction
            .forms
            .get(index as usize)
            .ok_or(SemanticError::UnsupportedContext)?;
        resolver.form_facts_cost(form)?;
        let mut form_context = context;
        form_context.vocab = "https://www.w3.org/2019/wot/hypermedia#";
        let form_context = resolver.local(form_context, &form._extra_fields)?;
        resolver.scope(
            SemanticScope::Form {
                property: name,
                index,
            },
            &form._extra_fields,
            None,
            form_context,
        )?;
        resolver.document.scopes.last_mut().unwrap().form = Some(form);
        if let Some(response) = &form.response {
            let context = resolver.local(form_context, &response._extra_fields)?;
            resolver.scope(
                SemanticScope::Response {
                    property: name,
                    index,
                },
                &response._extra_fields,
                None,
                context,
            )?;
        }
        if let Some(responses) = &form.additional_responses {
            for (response_index, response) in responses.iter().enumerate() {
                let context = resolver.local(form_context, &response._extra_fields)?;
                resolver.scope(
                    SemanticScope::AdditionalResponse {
                        property: name,
                        form: index,
                        index: response_index as u32,
                    },
                    &response._extra_fields,
                    None,
                    context,
                )?;
            }
        }
        let security_names = form.security.as_ref().unwrap_or(&thing.security);
        for name in security_names {
            resolver.pay(name.len() as u64 + 1)?;
            let security = thing
                .security_definitions
                .get(name)
                .ok_or(SemanticError::UnsupportedContext)?;
            let security = crate::validate::basic_typed::security_context(security);
            resolver.security_facts_cost(security)?;
            let mut context = root_context;
            context.vocab = "https://www.w3.org/2019/wot/security#";
            let context = resolver.local(context, &security._extra_fields)?;
            resolver.scope(
                SemanticScope::Security(name),
                &security._extra_fields,
                security.tags.as_deref(),
                context,
            )?;
        }
        resolver.uri_variables(thing.uri_variables.as_ref(), None, root_context)?;
        resolver.uri_variables(
            property._interaction.uri_variables.as_ref(),
            Some(name),
            context,
        )?;
        if let Some(responses) = &form.additional_responses {
            for response in responses {
                if let Some(reference) = &response.schema {
                    resolver.pay(reference.len() as u64 + 1)?;
                    if resolver
                        .document
                        .scope(SemanticScope::SchemaDefinition(reference))
                        .is_some()
                    {
                        continue;
                    }
                    let schema = thing
                        .schema_definitions
                        .as_ref()
                        .and_then(|definitions| definitions.get(reference))
                        .ok_or(SemanticError::UnsupportedDependency)?;
                    let mut context = root_context;
                    context.vocab = "https://www.w3.org/2019/wot/json-schema#";
                    resolver.schema_scope(
                        SemanticScope::SchemaDefinition(reference),
                        schema,
                        context,
                    )?;
                }
            }
        }
        resolver.document.selected = Some(Selected {
            thing,
            property,
            name,
        });
        Ok(())
    }
}

// Exact, fallible, engine-prepaid backing. Node/Scope have no owned descendants.
fn buffer<T>(count: usize) -> Result<Vec<T>, SemanticError> {
    let layout = Layout::array::<T>(count).map_err(|_| SemanticError::Limit)?;
    let pointer = unsafe { alloc(layout) }.cast::<T>();
    if pointer.is_null() {
        return Err(SemanticError::Allocation);
    }
    Ok(unsafe { Vec::from_raw_parts(pointer, 0, count) })
}

#[derive(Clone, Copy)]
enum ContextMap<'a> {
    Typed(&'a BTreeMap<String, Value>),
    Json(&'a Map<String, Value>),
}
#[derive(Clone, Copy)]
struct ContextFrame<'a> {
    map: ContextMap<'a>,
    vocab: &'a str,
}
impl<'a> ContextMap<'a> {
    fn get(self, name: &str, fuel: &mut u64) -> Result<Option<&'a Value>, SemanticError> {
        let mut find = |key: &str, value: &'a Value| {
            *fuel = fuel
                .checked_sub((name.len() + key.len() + 1) as u64)
                .ok_or(SemanticError::Limit)?;
            Ok(if key == name { Some(value) } else { None })
        };
        match self {
            Self::Typed(map) => {
                for (key, value) in map {
                    if let Some(v) = find(key, value)? {
                        return Ok(Some(v));
                    }
                }
            }
            Self::Json(map) => {
                for (key, value) in map {
                    if let Some(v) = find(key, value)? {
                        return Ok(Some(v));
                    }
                }
            }
        }
        Ok(None)
    }
}
#[derive(Clone, Copy)]
struct ActiveContext<'a> {
    maps: [Option<ContextFrame<'a>>; DEPTH],
    len: usize,
    vocab: &'a str,
}
impl<'a> ActiveContext<'a> {
    fn new(vocab: &'a str) -> Self {
        Self {
            maps: [None; DEPTH],
            len: 0,
            vocab,
        }
    }
    fn push(&mut self, map: ContextMap<'a>) -> Result<(), SemanticError> {
        if self.len == DEPTH {
            return Err(SemanticError::Limit);
        }
        self.maps[self.len] = Some(ContextFrame {
            map,
            vocab: self.vocab,
        });
        self.len += 1;
        Ok(())
    }
    fn definition(self, name: &str, fuel: &mut u64) -> Result<Option<&'a Value>, SemanticError> {
        Ok(self.defined_in(name, fuel)?.map(|(value, _)| value))
    }
    // Source maps are immutable. Truncating at the defining map reconstructs
    // its environment, including same-map forward references, without allowing
    // later contexts to reinterpret an inherited mapping. Vocabulary is captured
    // with that frame, not taken from the current Property/Form environment.
    fn defined_in(
        self,
        name: &str,
        fuel: &mut u64,
    ) -> Result<Option<(&'a Value, Self)>, SemanticError> {
        for index in (0..self.len).rev() {
            let frame = self.maps[index].unwrap();
            if let Some(value) = frame.map.get(name, fuel)? {
                let mut defining = self;
                defining.len = index + 1;
                defining.vocab = frame.vocab;
                return Ok(Some((value, defining)));
            }
        }
        Ok(None)
    }
}
struct Resolver<'a> {
    document: ResolvedSemanticDocument<'a>,
    fuel: u64,
}
impl<'a> Resolver<'a> {
    fn pay(&mut self, n: u64) -> Result<(), SemanticError> {
        self.fuel = self.fuel.checked_sub(n).ok_or(SemanticError::Limit)?;
        Ok(())
    }
    fn local(
        &mut self,
        mut context: ActiveContext<'a>,
        extras: &'a ExtensionMap,
    ) -> Result<ActiveContext<'a>, SemanticError> {
        if let Some(value) = extras.get("@context") {
            self.context_value(&mut context, value)?;
        }
        Ok(context)
    }
    fn context_value(
        &mut self,
        context: &mut ActiveContext<'a>,
        value: &'a Value,
    ) -> Result<(), SemanticError> {
        self.pay(1)?;
        match value {
            Value::Object(map) => {
                context.push(ContextMap::Json(map))?;
                for (key, value) in map {
                    self.context_definition(context, key, value)?;
                }
                context.maps[context.len - 1].as_mut().unwrap().vocab = context.vocab;
                Ok(())
            }
            Value::Array(array) => {
                for value in array {
                    if !value.is_object() {
                        return Err(SemanticError::UnsupportedContext);
                    }
                    self.context_value(context, value)?;
                }
                Ok(())
            }
            _ => Err(SemanticError::UnsupportedContext),
        }
    }
    fn context_definition(
        &mut self,
        context: &mut ActiveContext<'a>,
        key: &str,
        value: &'a Value,
    ) -> Result<(), SemanticError> {
        self.pay(key.len() as u64 + 1)?;
        if key == "@vocab" {
            let vocab = value.as_str().ok_or(SemanticError::UnsupportedContext)?;
            self.pay(vocab.len() as u64)?;
            if !vocab.contains(':') {
                return Err(SemanticError::UnsupportedContext);
            }
            // JSON-LD context processing expands @vocab before installing this
            // map's term definitions. Use the preceding environment, including
            // the standard context's prefixes, through the ordinary IRI rules.
            let mut preceding = *context;
            preceding.len -= 1;
            context.vocab = match self.name(vocab, preceding, 0)? {
                SemanticName::Iri(SemanticIri { prefix, suffix: "" }) => prefix,
                // Appending a name to a two-segment namespace would require a
                // third segment. Refuse it instead of retaining compact text.
                _ => return Err(SemanticError::UnsupportedContext),
            };
        } else if key.starts_with('@') || key.contains([':', '/']) {
            return Err(SemanticError::UnsupportedContext);
        }
        self.check_definition(value)
    }
    fn check_definition(&mut self, value: &Value) -> Result<(), SemanticError> {
        self.pay(1)?;
        match value {
            Value::Null => Ok(()),
            Value::String(id) if !id.starts_with('@') => Ok(()),
            Value::Object(map)
                if map
                    .keys()
                    .all(|k| matches!(k.as_str(), "@id" | "@prefix" | "@type" | "@context")) =>
            {
                if map
                    .get("@type")
                    .is_some_and(|v| !matches!(v.as_str(), Some("@id" | "@vocab")))
                {
                    return Err(SemanticError::UnsupportedContext);
                }
                if map.get("@prefix").is_some_and(|v| !v.is_boolean())
                    || !map.get("@id").is_some_and(Value::is_string)
                    || map
                        .get("@id")
                        .and_then(Value::as_str)
                        .is_some_and(|id| id.starts_with('@'))
                {
                    return Err(SemanticError::UnsupportedContext);
                }
                Ok(())
            }
            _ => Err(SemanticError::UnsupportedContext),
        }
    }
    fn name(
        &mut self,
        text: &'a str,
        context: ActiveContext<'a>,
        depth: usize,
    ) -> Result<SemanticName<'a>, SemanticError> {
        self.pay(text.len() as u64 + 1)?;
        if depth == DEPTH {
            return Err(SemanticError::Limit);
        }
        if text.starts_with('@') {
            return Ok(SemanticName::Keyword(text));
        }
        if let Some((definition, defining)) = context.defined_in(text, &mut self.fuel)? {
            self.check_definition(definition)?;
            return match definition {
                Value::Null => Ok(SemanticName::Unmapped(text)),
                Value::String(id) if id == text => Ok(SemanticName::Iri(SemanticIri {
                    prefix: if text.contains(':') {
                        text
                    } else {
                        defining.vocab
                    },
                    suffix: if text.contains(':') { "" } else { text },
                })),
                Value::String(id) => self.name(id, defining, depth + 1),
                Value::Object(map) => self.name(map["@id"].as_str().unwrap(), defining, depth + 1),
                _ => Err(SemanticError::UnsupportedContext),
            };
        }
        if let Some((prefix, suffix)) = text.split_once(':') {
            if prefix == "_" || suffix.starts_with("//") {
                return Ok(SemanticName::Iri(SemanticIri {
                    prefix: text,
                    suffix: "",
                }));
            }
            let found = context.defined_in(prefix, &mut self.fuel)?;
            let definition = found.map(|(value, _)| value);
            let defining = found.map_or(context, |(_, defining)| defining);
            if matches!(definition, Some(Value::String(base)) if !base.contains(':')) {
                // Prefix flags depend on the expanded mapping. A chained plain
                // alias is outside this two-segment prefix profile; treating it
                // as a non-prefix would silently erase an executable predicate.
                return Err(SemanticError::UnsupportedContext);
            }
            let base = match definition {
                Some(Value::String(base))
                    if base.ends_with([':', '/', '?', '#', '[', ']', '@']) =>
                {
                    Some(base.as_str())
                }
                Some(Value::Object(map))
                    if map.get("@prefix").and_then(Value::as_bool) == Some(true) =>
                {
                    map.get("@id").and_then(Value::as_str)
                }
                Some(_) => None,
                None => match prefix {
                    // Prefix expansion from the standard TD context, not a protocol interpreter.
                    "htv" => Some("http://www.w3.org/2011/http#"),
                    "rdfs" => Some("http://www.w3.org/2000/01/rdf-schema#"),
                    "xsd" => Some("http://www.w3.org/2001/XMLSchema#"),
                    "dct" => Some("http://purl.org/dc/terms/"),
                    "tm" => Some("https://www.w3.org/2019/wot/tm#"),
                    "td" => Some("https://www.w3.org/2019/wot/td#"),
                    "hctl" => Some("https://www.w3.org/2019/wot/hypermedia#"),
                    "jsonschema" => Some("https://www.w3.org/2019/wot/json-schema#"),
                    "wotsec" => Some("https://www.w3.org/2019/wot/security#"),
                    _ => None,
                },
            };
            if let Some(base) = base {
                let Some((base_prefix, _)) = base.split_once(':') else {
                    return Err(SemanticError::UnsupportedContext);
                };
                if defining.definition(base_prefix, &mut self.fuel)?.is_some()
                    || matches!(
                        base_prefix,
                        "htv"
                            | "rdfs"
                            | "td"
                            | "hctl"
                            | "jsonschema"
                            | "wotsec"
                            | "xsd"
                            | "dct"
                            | "tm"
                    )
                {
                    // Chained compact prefix definitions need a larger IRI
                    // representation; do not emit a partially expanded name.
                    return Err(SemanticError::UnsupportedContext);
                }
                return Ok(SemanticName::Iri(SemanticIri {
                    prefix: base,
                    suffix,
                }));
            }
            return Ok(SemanticName::Iri(SemanticIri {
                prefix: text,
                suffix: "",
            }));
        }
        Ok(SemanticName::Iri(SemanticIri {
            prefix: context.vocab,
            suffix: text,
        }))
    }
    fn push(
        &mut self,
        name: SemanticName<'a>,
        value: SemanticValue<'a>,
    ) -> Result<usize, SemanticError> {
        self.pay(1)?;
        if self.document.nodes.len() == NODES {
            return Err(SemanticError::Limit);
        }
        let index = self.document.nodes.len();
        self.document.nodes.push(Node {
            name,
            value,
            end: index + 1,
            schema: None,
        });
        Ok(index)
    }
    fn finish(&mut self, index: usize) {
        self.document.nodes[index].end = self.document.nodes.len();
    }
    fn scope(
        &mut self,
        key: SemanticScope<'a>,
        extras: &'a ExtensionMap,
        tags: Option<&'a [String]>,
        context: ActiveContext<'a>,
    ) -> Result<(), SemanticError> {
        if self.document.scopes.len() == SCOPES {
            return Err(SemanticError::Limit);
        }
        let root = self.push(SemanticName::Item, SemanticValue::Object)?;
        self.document.scopes.push(Scope {
            key,
            root,
            form: None,
        });
        for (key, value) in extras {
            if key == "@context" {
                continue;
            }
            self.member(key, value, context, 0)?;
        }
        if let Some(tags) = tags {
            let list = self.push(SemanticName::Keyword("@type"), SemanticValue::Array)?;
            for tag in tags {
                if context
                    .definition(tag, &mut self.fuel)?
                    .and_then(Value::as_object)
                    .is_some_and(|v| v.contains_key("@context"))
                {
                    return Err(SemanticError::UnsupportedContext);
                }
                let SemanticName::Iri(iri) = self.name(tag, context, 0)? else {
                    return Err(SemanticError::UnsupportedContext);
                };
                self.push(SemanticName::Item, SemanticValue::Iri(iri))?;
            }
            self.finish(list);
        }
        self.finish(root);
        Ok(())
    }
    fn member(
        &mut self,
        key: &'a str,
        value: &'a Value,
        context: ActiveContext<'a>,
        depth: usize,
    ) -> Result<(), SemanticError> {
        let name = self.name(key, context, 0)?;
        let mut child_context = context;
        let definition = context.definition(key, &mut self.fuel)?;
        let coercion = if key == "@type" {
            Some("@vocab")
        } else if key == "@id" {
            Some("@id")
        } else {
            definition
                .and_then(Value::as_object)
                .and_then(|v| v.get("@type"))
                .and_then(Value::as_str)
        };
        if let Some(scoped) = definition
            .and_then(Value::as_object)
            .and_then(|v| v.get("@context"))
        {
            self.context_value(&mut child_context, scoped)?;
        }
        self.value(name, value, child_context, depth, coercion)
    }
    fn schema_children(
        &mut self,
        schema: &'a crate::data_schema::DataSchema,
        context: ActiveContext<'a>,
        depth: usize,
    ) -> Result<(), SemanticError> {
        use crate::data_schema::DataSchema;
        if depth == DEPTH {
            return Err(SemanticError::Limit);
        }
        let iri = |suffix| {
            SemanticName::Iri(SemanticIri {
                prefix: "https://www.w3.org/2019/wot/json-schema#",
                suffix,
            })
        };
        if let Some(one_of) = &schema.context().one_of {
            let array = self.push(iri("oneOf"), SemanticValue::Array)?;
            for child in one_of {
                self.schema_node(SemanticName::Item, child, context, depth + 1)?;
            }
            self.finish(array);
        }
        match schema {
            DataSchema::Object(schema) => {
                if let Some(properties) = &schema.properties {
                    let object = self.push(iri("properties"), SemanticValue::Object)?;
                    for (key, child) in properties {
                        self.pay(key.len() as u64 + 1)?;
                        self.schema_node(SemanticName::Unmapped(key), child, context, depth + 1)?;
                    }
                    self.finish(object);
                }
            }
            DataSchema::Array(schema) => {
                if let Some(items) = &schema.items {
                    let array = self.push(iri("items"), SemanticValue::Array)?;
                    for child in items.iter() {
                        self.schema_node(SemanticName::Item, child, context, depth + 1)?;
                    }
                    self.finish(array);
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn schema_node(
        &mut self,
        name: SemanticName<'a>,
        schema: &'a crate::data_schema::DataSchema,
        context: ActiveContext<'a>,
        depth: usize,
    ) -> Result<(), SemanticError> {
        let context = self.local(context, &schema.context()._extra_fields)?;
        let root = self.push(name, SemanticValue::Object)?;
        self.document.nodes[root].schema = Some(schema);
        self.schema_facts(schema, depth, context)?;
        for (key, value) in &schema.context()._extra_fields {
            if key != "@context" {
                self.member(key, value, context, depth)?;
            }
        }
        if let Some(tags) = &schema.context()._metadata.tags {
            let array = self.push(SemanticName::Keyword("@type"), SemanticValue::Array)?;
            for tag in tags {
                let SemanticName::Iri(iri) = self.name(tag, context, 0)? else {
                    return Err(SemanticError::UnsupportedContext);
                };
                self.push(SemanticName::Item, SemanticValue::Iri(iri))?;
            }
            self.finish(array);
        }
        self.schema_children(schema, context, depth)?;
        self.finish(root);
        Ok(())
    }
    fn value(
        &mut self,
        name: SemanticName<'a>,
        value: &'a Value,
        mut context: ActiveContext<'a>,
        depth: usize,
        coercion: Option<&str>,
    ) -> Result<(), SemanticError> {
        if depth == DEPTH {
            return Err(SemanticError::Limit);
        }
        let scalar = match value {
            Value::Null => SemanticValue::Null,
            Value::Bool(v) => SemanticValue::Bool(*v),
            Value::Number(v) => SemanticValue::Number(v),
            Value::String(text) => {
                self.pay(text.len() as u64)?;
                if coercion.is_some() {
                    if coercion == Some("@id") && !text.contains(':') {
                        return Err(SemanticError::UnsupportedContext);
                    }
                    if context
                        .definition(text, &mut self.fuel)?
                        .and_then(Value::as_object)
                        .is_some_and(|v| v.contains_key("@context"))
                    {
                        return Err(SemanticError::UnsupportedContext);
                    }
                    match self.name(text, context, 0)? {
                        SemanticName::Iri(iri) => SemanticValue::Iri(iri),
                        _ => return Err(SemanticError::UnsupportedContext),
                    }
                } else {
                    SemanticValue::Text(text)
                }
            }
            Value::Array(_) => SemanticValue::Array,
            Value::Object(map) => {
                if let Some(local) = map.get("@context") {
                    self.context_value(&mut context, local)?;
                }
                SemanticValue::Object
            }
        };
        let index = self.push(name, scalar)?;
        match value {
            Value::Array(array) => {
                for item in array {
                    self.value(SemanticName::Item, item, context, depth + 1, coercion)?;
                }
            }
            Value::Object(map) => {
                for (key, value) in map {
                    if key != "@context" {
                        self.member(key, value, context, depth + 1)?;
                    }
                }
            }
            _ => {}
        }
        self.finish(index);
        Ok(())
    }
}
