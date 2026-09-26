//! Non-production typed traversal over the shared corpus and nested DataSchema
//! fixture. Compiled as a child of `context` to inspect private Context entries.
//! This is storage evidence, not a Basic/default/URI/security rule kernel.
//! Recursive prototype calls and fixed arena headroom do not prove the future
//! resumable traversal, exact sizing, or supported feature matrix.

use super::ContextEntry;
use crate as td_crate;
use alloc::{collections::BTreeMap, string::String, vec::Vec};
use serde_json::Value;
use td_crate::{
    data_schema::DataSchema,
    data_type::{ExtensionMap, Metadata, MultiLanguage},
    form::Form,
    thing::Thing,
    validate::{Validate, ValidationLevel},
};
#[path = "../../../tools/architecture-fixtures/validated-thing-arena-layout/src/lib.rs"]
#[allow(unused_attributes)]
mod arena_layout;
use arena_layout::{Prototype, RetainedEdge, RetainedNode};

#[path = "typed_corpus_shared.rs"]
mod typed_corpus_shared;

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Root,
    Context,
    Uri,
    Str,
    Absent,
    Array,
    Map,
    Entry,
    Null,
    True,
    False,
    Number,
    Property,
    Form,
    Schema,
    SchemaArray,
    SchemaBoolean,
    SchemaNumber,
    SchemaInteger,
    SchemaObject,
    SchemaString,
    SchemaNull,
    Version,
    U32,
    F64,
    I64,
}

const ROOT_CONTEXT: usize = 0;
const ROOT_ID: usize = 1;
const ROOT_TITLE: usize = 2;
const ROOT_TITLES: usize = 3;
const ROOT_SUPPORT: usize = 4;
const ROOT_BASE: usize = 5;
const ROOT_SECURITY: usize = 6;
const ROOT_SECURITY_DEFINITIONS: usize = 7;
const ROOT_PROPERTIES: usize = 8;
const ROOT_FORMS: usize = 9;
const ROOT_EXTENSIONS: usize = 10;
const ROOT_TAGS: usize = 11;
const ROOT_DESCRIPTION: usize = 12;
const ROOT_DESCRIPTIONS: usize = 13;
const ROOT_VERSION: usize = 14;
const ROOT_PROFILE: usize = 15;
const ROOT_SCHEMA_DEFINITIONS: usize = 16;
const ROOT_URI_VARIABLES: usize = 17;
const ROOT_FIELD_COUNT: usize = 18;

const EMPTY_EDGE: RetainedEdge = RetainedEdge {
    target: 0,
    original_index: 0,
};

struct Build {
    arena: Prototype,
    nodes: u32,
    edges: u32,
    bytes: u32,
}

impl Build {
    fn new() -> Self {
        let mut arena = Prototype::new(1_000_000, 1_000_000, 2_000_000, 1_000_000);
        // Fixed headroom keeps this slice focused on semantic storage. The
        // arena fixture separately proves checked growth/seal accounting.
        arena.grow_nodes(2_048).unwrap();
        arena.grow_edges(4_096).unwrap();
        arena.grow_bytes(32_768).unwrap();
        Self {
            arena,
            nodes: 0,
            edges: 0,
            bytes: 0,
        }
    }

    fn node(&mut self, kind: Kind, children: usize) -> u32 {
        let first = self.edges;
        for _ in 0..children {
            self.arena.push_edge(EMPTY_EDGE).unwrap();
            self.edges += 1;
        }
        let id = self.nodes;
        self.arena
            .push_node(RetainedNode {
                kind: kind as u32,
                first_edge: first,
                edge_count: children.try_into().unwrap(),
                first_byte: self.bytes,
                byte_count: 0,
            })
            .unwrap();
        self.nodes += 1;
        id
    }

    fn text(&mut self, kind: Kind, text: &str) -> u32 {
        let first = self.bytes;
        for &byte in text.as_bytes() {
            self.arena.push_byte(byte).unwrap();
            self.bytes += 1;
        }
        let id = self.nodes;
        self.arena
            .push_node(RetainedNode {
                kind: kind as u32,
                first_edge: self.edges,
                edge_count: 0,
                first_byte: first,
                byte_count: self.bytes - first,
            })
            .unwrap();
        self.nodes += 1;
        id
    }

    fn put(&mut self, first: u32, index: usize, target: u32) {
        self.arena.set_edge(
            first as usize + index,
            RetainedEdge {
                target,
                original_index: index.try_into().unwrap(),
            },
        );
    }

    fn sort_object_sources(
        &mut self,
        map: u32,
        values: &serde_json::Map<alloc::string::String, Value>,
    ) {
        let node = self.node_at(map);
        assert_eq!(node.kind, Kind::Map as u32);
        let first = node.first_edge as usize;
        // Until emission, target is a source-member index. Sort these scalar
        // slots before any child node or byte is emitted, so the whole sealed
        // arena layout is independent of serde's map storage order. Source
        // lookup uses the borrowed map and no owned key/value buffer.
        for index in 0..values.len() {
            self.put(node.first_edge, index, index.try_into().unwrap());
        }
        for index in 1..node.edge_count as usize {
            let pivot = self.arena.build_edge(first + index);
            let pivot_key = values.iter().nth(pivot.target as usize).unwrap().0;
            let mut position = index;
            while position > 0 {
                let prior = self.arena.build_edge(first + position - 1);
                let prior_key = values.iter().nth(prior.target as usize).unwrap().0;
                if prior_key <= pivot_key {
                    break;
                }
                self.arena.set_edge(first + position, prior);
                position -= 1;
            }
            self.arena.set_edge(first + position, pivot);
        }
    }

    fn absent_or_text(&mut self, value: Option<&str>, kind: Kind) -> u32 {
        match value {
            Some(value) => self.text(kind, value),
            None => self.node(Kind::Absent, 0),
        }
    }

    fn scalar_u32(&mut self, value: u32) -> u32 {
        self.scalar_bits(Kind::U32, value as u64)
    }

    fn optional_u32(&mut self, value: Option<u32>) -> u32 {
        match value {
            Some(v) => self.scalar_u32(v),
            None => self.node(Kind::Absent, 0),
        }
    }

    fn scalar_bits(&mut self, kind: Kind, bits: u64) -> u32 {
        let first = self.bytes;
        for byte in bits.to_be_bytes() {
            self.arena.push_byte(byte).unwrap();
            self.bytes += 1;
        }
        let id = self.nodes;
        self.arena
            .push_node(RetainedNode {
                kind: kind as u32,
                first_edge: self.edges,
                edge_count: 0,
                first_byte: first,
                byte_count: 8,
            })
            .unwrap();
        self.nodes += 1;
        id
    }

    fn optional_f64(&mut self, value: Option<f64>) -> u32 {
        match value {
            Some(v) => self.scalar_bits(Kind::F64, v.to_bits()),
            None => self.node(Kind::Absent, 0),
        }
    }

    fn optional_i64(&mut self, value: Option<i64>) -> u32 {
        match value {
            Some(v) => self.scalar_bits(Kind::I64, v as u64),
            None => self.node(Kind::Absent, 0),
        }
    }

    fn optional_value(&mut self, value: Option<&Value>) -> u32 {
        match value {
            Some(v) => self.value(v),
            None => self.node(Kind::Absent, 0),
        }
    }

    fn sequence<T>(
        &mut self,
        values: Option<&[T]>,
        mut item: impl FnMut(&mut Self, &T) -> u32,
    ) -> u32 {
        let Some(values) = values else {
            return self.node(Kind::Absent, 0);
        };
        let id = self.node(Kind::Array, values.len());
        let first = self.node_at(id).first_edge;
        for (index, value) in values.iter().enumerate() {
            let child = item(self, value);
            self.put(first, index, child);
        }
        id
    }

    fn language_map(&mut self, value: Option<&MultiLanguage>) -> u32 {
        let Some(value) = value else {
            return self.node(Kind::Absent, 0);
        };
        let id = self.node(Kind::Map, value.len());
        let first = self.node_at(id).first_edge;
        for (index, (key, value)) in value.as_map().iter().enumerate() {
            let entry = self.node(Kind::Entry, 2);
            let pair = self.node_at(entry).first_edge;
            let key = self.text(Kind::Str, key);
            self.put(pair, 0, key);
            let value = self.text(Kind::Str, value);
            self.put(pair, 1, value);
            self.put(first, index, entry);
        }
        id
    }

    fn metadata(&mut self, metadata: &Metadata) -> u32 {
        let id = self.node(Kind::Map, 5);
        let first = self.node_at(id).first_edge;
        let mut fields = [
            (
                "@type",
                self.sequence(metadata.tags.as_deref(), |this, v| this.text(Kind::Str, v)),
            ),
            (
                "title",
                self.absent_or_text(metadata.title.as_deref(), Kind::Str),
            ),
            ("titles", self.language_map(metadata.titles.as_ref())),
            (
                "description",
                self.absent_or_text(metadata.description.as_deref(), Kind::Str),
            ),
            (
                "descriptions",
                self.language_map(metadata.descriptions.as_ref()),
            ),
        ];
        fields.sort_unstable_by(|a, b| a.0.cmp(b.0));
        for (index, (key, value)) in fields.into_iter().enumerate() {
            let entry = self.node(Kind::Entry, 2);
            let pair = self.node_at(entry).first_edge;
            let key = self.text(Kind::Str, key);
            self.put(pair, 0, key);
            self.put(pair, 1, value);
            self.put(first, index, entry);
        }
        id
    }

    fn schema_map(&mut self, values: Option<&BTreeMap<String, DataSchema>>) -> u32 {
        let Some(values) = values else {
            return self.node(Kind::Absent, 0);
        };
        let id = self.node(Kind::Map, values.len());
        let first = self.node_at(id).first_edge;
        for (index, (key, value)) in values.iter().enumerate() {
            let entry = self.node(Kind::Entry, 2);
            let pair = self.node_at(entry).first_edge;
            let key = self.text(Kind::Str, key);
            self.put(pair, 0, key);
            let value = self.schema(value);
            self.put(pair, 1, value);
            self.put(first, index, entry);
        }
        id
    }

    fn schema(&mut self, schema: &DataSchema) -> u32 {
        let id = self.node(Kind::Schema, 2);
        let first = self.node_at(id).first_edge;
        let (context, variant) = match schema {
            DataSchema::Array(v) => {
                let variant = self.node(Kind::SchemaArray, 3);
                let at = self.node_at(variant).first_edge;
                let items = self.sequence(v.items.as_deref(), |this, v| this.schema(v));
                self.put(at, 0, items);
                let min = self.optional_u32(v.min_items);
                self.put(at, 1, min);
                let max = self.optional_u32(v.max_items);
                self.put(at, 2, max);
                (&v._context, variant)
            }
            DataSchema::Object(v) => {
                let variant = self.node(Kind::SchemaObject, 2);
                let at = self.node_at(variant).first_edge;
                let properties = self.schema_map(v.properties.as_ref());
                self.put(at, 0, properties);
                let required =
                    self.sequence(v.required.as_deref(), |this, v| this.text(Kind::Str, v));
                self.put(at, 1, required);
                (&v._context, variant)
            }
            DataSchema::String(v) => {
                let variant = self.node(Kind::SchemaString, 5);
                let at = self.node_at(variant).first_edge;
                let fields = [
                    self.optional_u32(v.min_length),
                    self.optional_u32(v.max_length),
                    self.absent_or_text(v.pattern.as_deref(), Kind::Str),
                    self.absent_or_text(v.content_encoding.as_deref(), Kind::Str),
                    self.absent_or_text(v.content_media_type.as_deref(), Kind::Str),
                ];
                for (index, child) in fields.into_iter().enumerate() {
                    self.put(at, index, child);
                }
                (&v._context, variant)
            }
            DataSchema::Number(v) => {
                let variant = self.node(Kind::SchemaNumber, 5);
                let at = self.node_at(variant).first_edge;
                let fields = [
                    self.optional_f64(v.minimum),
                    self.optional_f64(v.exclusive_minimum),
                    self.optional_f64(v.maximum),
                    self.optional_f64(v.exclusive_maximum),
                    self.optional_f64(v.multiple_of),
                ];
                for (index, child) in fields.into_iter().enumerate() {
                    self.put(at, index, child);
                }
                (&v._context, variant)
            }
            DataSchema::Integer(v) => {
                let variant = self.node(Kind::SchemaInteger, 5);
                let at = self.node_at(variant).first_edge;
                let fields = [
                    self.optional_i64(v.minimum),
                    self.optional_i64(v.exclusive_minimum),
                    self.optional_i64(v.maximum),
                    self.optional_i64(v.exclusive_maximum),
                    self.optional_i64(v.multiple_of),
                ];
                for (index, child) in fields.into_iter().enumerate() {
                    self.put(at, index, child);
                }
                (&v._context, variant)
            }
            DataSchema::Boolean(v) => (&v._context, self.node(Kind::SchemaBoolean, 0)),
            DataSchema::Null(v) => (&v._context, self.node(Kind::SchemaNull, 0)),
        };
        self.put(first, 0, variant);
        let context_id = self.node(Kind::Map, 11);
        let at = self.node_at(context_id).first_edge;
        let mut fields = [
            ("metadata", self.metadata(&context._metadata)),
            ("const", self.optional_value(context.constant.as_ref())),
            ("default", self.optional_value(context.default.as_ref())),
            (
                "unit",
                self.absent_or_text(context.unit.as_deref(), Kind::Str),
            ),
            (
                "oneOf",
                self.sequence(context.one_of.as_deref(), |this, v| this.schema(v)),
            ),
            (
                "enum",
                self.sequence(context.enumerate.as_deref(), |this, v| this.value(v)),
            ),
            (
                "readOnly",
                self.node(
                    if context.read_only {
                        Kind::True
                    } else {
                        Kind::False
                    },
                    0,
                ),
            ),
            (
                "writeOnly",
                self.node(
                    if context.write_only {
                        Kind::True
                    } else {
                        Kind::False
                    },
                    0,
                ),
            ),
            (
                "format",
                self.absent_or_text(context.format.as_deref(), Kind::Str),
            ),
            (
                "type",
                self.absent_or_text(context.data_type.as_deref(), Kind::Str),
            ),
            ("extensions", self.extensions(&context._extra_fields)),
        ];
        fields.sort_unstable_by(|a, b| a.0.cmp(b.0));
        for (index, (key, value)) in fields.into_iter().enumerate() {
            let entry = self.node(Kind::Entry, 2);
            let pair = self.node_at(entry).first_edge;
            let key = self.text(Kind::Str, key);
            self.put(pair, 0, key);
            self.put(pair, 1, value);
            self.put(at, index, entry);
        }
        self.put(first, 1, context_id);
        id
    }

    fn value(&mut self, value: &Value) -> u32 {
        match value {
            Value::Null => self.node(Kind::Null, 0),
            Value::Bool(true) => self.node(Kind::True, 0),
            Value::Bool(false) => self.node(Kind::False, 0),
            Value::String(value) => self.text(Kind::Str, value),
            Value::Number(value) => self.text(Kind::Number, value.as_str()),
            Value::Array(values) => {
                let id = self.node(Kind::Array, values.len());
                let first = self.node_at(id).first_edge;
                for (index, value) in values.iter().enumerate() {
                    let child = self.value(value);
                    self.put(first, index, child);
                }
                id
            }
            Value::Object(values) => {
                let id = self.node(Kind::Map, values.len());
                let first = self.node_at(id).first_edge;
                self.sort_object_sources(id, values);
                for index in 0..values.len() {
                    let source = self.arena.build_edge(first as usize + index).target;
                    let (key, value) = values.iter().nth(source as usize).unwrap();
                    let entry = self.node(Kind::Entry, 2);
                    let pair = self.node_at(entry).first_edge;
                    let key = self.text(Kind::Str, key);
                    self.put(pair, 0, key);
                    let value = self.value(value);
                    self.put(pair, 1, value);
                    self.put(first, index, entry);
                }
                id
            }
        }
    }

    fn node_at(&self, id: u32) -> RetainedNode {
        self.arena.build_node(id as usize)
    }

    fn extensions(&mut self, values: &ExtensionMap) -> u32 {
        let id = self.node(Kind::Map, values.len());
        let first = self.node_at(id).first_edge;
        for (index, (key, value)) in values.iter().enumerate() {
            let entry = self.node(Kind::Entry, 2);
            let pair = self.node_at(entry).first_edge;
            let key = self.text(Kind::Str, key);
            self.put(pair, 0, key);
            let value = self.value(value);
            self.put(pair, 1, value);
            self.put(first, index, entry);
        }
        id
    }

    fn form(&mut self, form: &Form) -> u32 {
        let id = self.node(Kind::Form, 5);
        let first = self.node_at(id).first_edge;
        let href = self.text(Kind::Uri, form.href.as_str());
        self.put(first, 0, href);
        let content_type = self.text(Kind::Str, &form.content_type);
        self.put(first, 1, content_type);
        let coding = self.absent_or_text(form.content_coding.as_deref(), Kind::Str);
        self.put(first, 2, coding);
        let ops = match &form.op {
            None => self.node(Kind::Absent, 0),
            Some(ops) => {
                let seq = self.node(Kind::Array, ops.len());
                let seq_first = self.node_at(seq).first_edge;
                for (index, op) in ops.iter().enumerate() {
                    let item = self.text(Kind::Str, op.as_str());
                    self.put(seq_first, index, item);
                }
                seq
            }
        };
        self.put(first, 3, ops);
        let extras = self.extensions(&form._extra_fields);
        self.put(first, 4, extras);
        id
    }

    fn forms(&mut self, forms: Option<&[Form]>) -> u32 {
        match forms {
            None => self.node(Kind::Absent, 0),
            Some(forms) => {
                let id = self.node(Kind::Array, forms.len());
                let first = self.node_at(id).first_edge;
                for (index, form) in forms.iter().enumerate() {
                    let item = self.form(form);
                    self.put(first, index, item);
                }
                id
            }
        }
    }

    fn version(&mut self, version: Option<&td_crate::data_type::VersionInfo>) -> u32 {
        let Some(version) = version else {
            return self.node(Kind::Absent, 0);
        };
        let id = self.node(Kind::Version, 3);
        let first = self.node_at(id).first_edge;
        let instance = self.text(Kind::Str, &version.instance);
        self.put(first, 0, instance);
        let model = self.absent_or_text(version.model.as_deref(), Kind::Str);
        self.put(first, 1, model);
        let extensions = self.extensions(&version._extra_fields);
        self.put(first, 2, extensions);
        id
    }

    fn thing(&mut self, thing: &Thing) -> u32 {
        let id = self.node(Kind::Root, ROOT_FIELD_COUNT);
        let first = self.node_at(id).first_edge;
        let context = self.node(Kind::Context, thing.context.entries.len());
        let context_first = self.node_at(context).first_edge;
        for (index, entry) in thing.context.entries.iter().enumerate() {
            let item = match entry {
                ContextEntry::Uri(uri) => self.text(Kind::Uri, uri.as_str()),
                ContextEntry::Object(map) => self.extensions(map),
            };
            self.put(context_first, index, item);
        }
        self.put(first, ROOT_CONTEXT, context);
        let value = self.absent_or_text(thing.id.as_ref().map(|v| v.as_str()), Kind::Uri);
        self.put(first, ROOT_ID, value);
        let value = self.absent_or_text(thing._metadata.title.as_deref(), Kind::Str);
        self.put(first, ROOT_TITLE, value);
        let titles = match thing._metadata.titles.as_ref() {
            None => self.node(Kind::Absent, 0),
            Some(titles) => {
                let map = titles.as_map();
                let id = self.node(Kind::Map, map.len());
                let start = self.node_at(id).first_edge;
                for (index, (lang, title)) in map.iter().enumerate() {
                    let entry = self.node(Kind::Entry, 2);
                    let pair = self.node_at(entry).first_edge;
                    let key = self.text(Kind::Str, lang);
                    self.put(pair, 0, key);
                    let value = self.text(Kind::Str, title);
                    self.put(pair, 1, value);
                    self.put(start, index, entry);
                }
                id
            }
        };
        self.put(first, ROOT_TITLES, titles);
        let value = self.absent_or_text(thing.support.as_ref().map(|v| v.as_str()), Kind::Uri);
        self.put(first, ROOT_SUPPORT, value);
        let value = self.absent_or_text(thing.base.as_ref().map(|v| v.as_str()), Kind::Uri);
        self.put(first, ROOT_BASE, value);
        let security = self.node(Kind::Array, thing.security.len());
        let security_first = self.node_at(security).first_edge;
        for (index, name) in thing.security.iter().enumerate() {
            let item = self.text(Kind::Str, name);
            self.put(security_first, index, item);
        }
        self.put(first, ROOT_SECURITY, security);
        let definitions = self.node(Kind::Map, thing.security_definitions.len());
        let definitions_first = self.node_at(definitions).first_edge;
        for (index, (name, scheme)) in thing.security_definitions.iter().enumerate() {
            let entry = self.node(Kind::Entry, 2);
            let pair = self.node_at(entry).first_edge;
            let key = self.text(Kind::Str, name);
            self.put(pair, 0, key);
            let scheme = match scheme {
                td_crate::security_scheme::SecurityScheme::NoSec(s) => {
                    self.text(Kind::Str, &s._context.scheme)
                }
                _ => panic!("corpus-scoped probe: unhandled security variant"),
            };
            self.put(pair, 1, scheme);
            self.put(definitions_first, index, entry);
        }
        self.put(first, ROOT_SECURITY_DEFINITIONS, definitions);
        let properties = match thing.properties.as_ref() {
            None => self.node(Kind::Absent, 0),
            Some(properties) => {
                let id = self.node(Kind::Map, properties.len());
                let start = self.node_at(id).first_edge;
                for (index, (name, property)) in properties.iter().enumerate() {
                    let entry = self.node(Kind::Entry, 2);
                    let pair = self.node_at(entry).first_edge;
                    let key = self.text(Kind::Str, name);
                    self.put(pair, 0, key);
                    let value = self.node(Kind::Property, 2);
                    let property_first = self.node_at(value).first_edge;
                    let schema = self.schema(&property._schema);
                    self.put(property_first, 0, schema);
                    let forms = self.forms(Some(&property._interaction.forms));
                    self.put(property_first, 1, forms);
                    self.put(pair, 1, value);
                    self.put(start, index, entry);
                }
                id
            }
        };
        self.put(first, ROOT_PROPERTIES, properties);
        let forms = self.forms(thing.forms.as_deref());
        self.put(first, ROOT_FORMS, forms);
        let extras = self.extensions(&thing._extra_fields);
        self.put(first, ROOT_EXTENSIONS, extras);
        let tags = self.sequence(thing._metadata.tags.as_deref(), |this, value| {
            this.text(Kind::Str, value)
        });
        self.put(first, ROOT_TAGS, tags);
        let description = self.absent_or_text(thing._metadata.description.as_deref(), Kind::Str);
        self.put(first, ROOT_DESCRIPTION, description);
        let descriptions = self.language_map(thing._metadata.descriptions.as_ref());
        self.put(first, ROOT_DESCRIPTIONS, descriptions);
        let version = self.version(thing.version.as_ref());
        self.put(first, ROOT_VERSION, version);
        let profile = self.sequence(thing.profile.as_deref(), |this, value| {
            this.text(Kind::Uri, value.as_str())
        });
        self.put(first, ROOT_PROFILE, profile);
        let schema_definitions = self.schema_map(thing.schema_definitions.as_ref());
        self.put(first, ROOT_SCHEMA_DEFINITIONS, schema_definitions);
        let uri_variables = self.schema_map(thing.uri_variables.as_ref());
        self.put(first, ROOT_URI_VARIABLES, uri_variables);
        id
    }
}

struct Snapshot {
    arena: Prototype,
    root: u32,
}

impl Snapshot {
    fn normalize(thing: &Thing) -> Self {
        // The shared corpus builder calls today's TD Basic before entry. A
        // storage-neutral Basic kernel is not present, so this probe neither
        // validates a snapshot nor reimplements any Basic/default/URI rule.
        let mut build = Build::new();
        let root = build.thing(thing);
        let arena = build.arena.seal().unwrap();
        Self { arena, root }
    }
    fn node(&self, id: u32) -> &RetainedNode {
        &self.arena.nodes()[id as usize]
    }
    fn kind(&self, id: u32) -> Kind {
        match self.node(id).kind {
            x if x == Kind::Root as u32 => Kind::Root,
            x if x == Kind::Context as u32 => Kind::Context,
            x if x == Kind::Uri as u32 => Kind::Uri,
            x if x == Kind::Str as u32 => Kind::Str,
            x if x == Kind::Absent as u32 => Kind::Absent,
            x if x == Kind::Array as u32 => Kind::Array,
            x if x == Kind::Map as u32 => Kind::Map,
            x if x == Kind::Entry as u32 => Kind::Entry,
            x if x == Kind::Null as u32 => Kind::Null,
            x if x == Kind::True as u32 => Kind::True,
            x if x == Kind::False as u32 => Kind::False,
            x if x == Kind::Number as u32 => Kind::Number,
            x if x == Kind::Property as u32 => Kind::Property,
            x if x == Kind::Form as u32 => Kind::Form,
            x if x == Kind::Schema as u32 => Kind::Schema,
            x if x == Kind::SchemaArray as u32 => Kind::SchemaArray,
            x if x == Kind::SchemaBoolean as u32 => Kind::SchemaBoolean,
            x if x == Kind::SchemaNumber as u32 => Kind::SchemaNumber,
            x if x == Kind::SchemaInteger as u32 => Kind::SchemaInteger,
            x if x == Kind::SchemaObject as u32 => Kind::SchemaObject,
            x if x == Kind::SchemaString as u32 => Kind::SchemaString,
            x if x == Kind::SchemaNull as u32 => Kind::SchemaNull,
            x if x == Kind::Version as u32 => Kind::Version,
            x if x == Kind::U32 as u32 => Kind::U32,
            x if x == Kind::F64 as u32 => Kind::F64,
            x if x == Kind::I64 as u32 => Kind::I64,
            _ => panic!("invalid prototype node kind"),
        }
    }
    fn edge(&self, id: u32, index: usize) -> RetainedEdge {
        let node = self.node(id);
        self.arena.edges()[node.first_edge as usize + index]
    }
    fn child(&self, id: u32, index: usize) -> u32 {
        self.edge(id, index).target
    }
    fn text(&self, id: u32) -> &str {
        let node = self.node(id);
        core::str::from_utf8(
            &self.arena.bytes()
                [node.first_byte as usize..(node.first_byte + node.byte_count) as usize],
        )
        .unwrap()
    }
    fn map_get(&self, id: u32, key: &str) -> Option<u32> {
        let node = self.node(id);
        assert_eq!(self.kind(id), Kind::Map);
        (0..node.edge_count as usize).find_map(|i| {
            let entry = self.child(id, i);
            (self.text(self.child(entry, 0)) == key).then(|| self.child(entry, 1))
        })
    }
    fn assert_sorted_map(&self, id: u32) {
        assert_eq!(self.kind(id), Kind::Map);
        let mut previous = None;
        for index in 0..self.node(id).edge_count as usize {
            let edge = self.edge(id, index);
            assert_eq!(edge.original_index as usize, index);
            let entry = edge.target;
            let key = self.text(self.child(entry, 0));
            if let Some(previous) = previous {
                assert!(previous < key, "map keys must be sorted and unique");
            }
            previous = Some(key);
        }
    }
    fn same(&self, other: &Self) -> bool {
        self.root == other.root
            && self.arena.nodes() == other.arena.nodes()
            && self.arena.edges() == other.arena.edges()
            && self.arena.bytes() == other.arena.bytes()
    }
    fn assert_optional_text(&self, id: u32, value: Option<&str>) {
        match value {
            Some(value) => {
                assert_eq!(self.kind(id), Kind::Str);
                assert_eq!(self.text(id), value);
            }
            None => assert_eq!(self.kind(id), Kind::Absent),
        }
    }
    fn assert_optional_value(&self, id: u32, value: Option<&Value>) {
        match value {
            Some(value) => self.assert_value(id, value),
            None => assert_eq!(self.kind(id), Kind::Absent),
        }
    }
    fn assert_sequence<T>(
        &self,
        id: u32,
        values: Option<&[T]>,
        mut check: impl FnMut(&Self, u32, &T),
    ) {
        let Some(values) = values else {
            assert_eq!(self.kind(id), Kind::Absent);
            return;
        };
        assert_eq!(self.kind(id), Kind::Array);
        assert_eq!(self.node(id).edge_count as usize, values.len());
        for (index, value) in values.iter().enumerate() {
            let edge = self.edge(id, index);
            assert_eq!(edge.original_index as usize, index);
            check(self, edge.target, value);
        }
    }
    fn assert_bits(&self, id: u32, kind: Kind, value: Option<u64>) {
        let Some(value) = value else {
            assert_eq!(self.kind(id), Kind::Absent);
            return;
        };
        assert_eq!(self.kind(id), kind);
        let node = self.node(id);
        assert_eq!(node.byte_count, 8);
        let bytes: [u8; 8] = self.arena.bytes()[node.first_byte as usize..][..8]
            .try_into()
            .unwrap();
        assert_eq!(u64::from_be_bytes(bytes), value);
    }
    fn assert_language(&self, id: u32, source: Option<&MultiLanguage>) {
        let Some(source) = source else {
            assert_eq!(self.kind(id), Kind::Absent);
            return;
        };
        assert_eq!(self.kind(id), Kind::Map);
        self.assert_sorted_map(id);
        assert_eq!(self.node(id).edge_count as usize, source.len());
        for (key, value) in source.as_map() {
            assert_eq!(self.text(self.map_get(id, key).unwrap()), value);
        }
    }
    fn assert_metadata(&self, id: u32, source: &Metadata) {
        assert_eq!(self.kind(id), Kind::Map);
        self.assert_sorted_map(id);
        assert_eq!(self.node(id).edge_count, 5);
        self.assert_sequence(
            self.map_get(id, "@type").unwrap(),
            source.tags.as_deref(),
            |snapshot, node, tag| {
                assert_eq!(snapshot.text(node), tag);
            },
        );
        self.assert_optional_text(self.map_get(id, "title").unwrap(), source.title.as_deref());
        self.assert_language(self.map_get(id, "titles").unwrap(), source.titles.as_ref());
        self.assert_optional_text(
            self.map_get(id, "description").unwrap(),
            source.description.as_deref(),
        );
        self.assert_language(
            self.map_get(id, "descriptions").unwrap(),
            source.descriptions.as_ref(),
        );
    }
    fn assert_schema(&self, id: u32, source: &DataSchema) {
        assert_eq!(self.kind(id), Kind::Schema);
        let variant = self.child(id, 0);
        let context = self.child(id, 1);
        let source_context = match source {
            DataSchema::Array(v) => {
                assert_eq!(self.kind(variant), Kind::SchemaArray);
                self.assert_sequence(
                    self.child(variant, 0),
                    v.items.as_deref(),
                    |snapshot, node, item| snapshot.assert_schema(node, item),
                );
                self.assert_bits(
                    self.child(variant, 1),
                    Kind::U32,
                    v.min_items.map(u64::from),
                );
                self.assert_bits(
                    self.child(variant, 2),
                    Kind::U32,
                    v.max_items.map(u64::from),
                );
                &v._context
            }
            DataSchema::Object(v) => {
                assert_eq!(self.kind(variant), Kind::SchemaObject);
                let properties = self.child(variant, 0);
                match &v.properties {
                    Some(map) => {
                        assert_eq!(self.kind(properties), Kind::Map);
                        self.assert_sorted_map(properties);
                        assert_eq!(self.node(properties).edge_count as usize, map.len());
                        for (key, schema) in map {
                            self.assert_schema(self.map_get(properties, key).unwrap(), schema);
                        }
                    }
                    None => assert_eq!(self.kind(properties), Kind::Absent),
                }
                self.assert_sequence(
                    self.child(variant, 1),
                    v.required.as_deref(),
                    |snapshot, node, name| {
                        assert_eq!(snapshot.text(node), name);
                    },
                );
                &v._context
            }
            DataSchema::String(v) => {
                assert_eq!(self.kind(variant), Kind::SchemaString);
                self.assert_bits(
                    self.child(variant, 0),
                    Kind::U32,
                    v.min_length.map(u64::from),
                );
                self.assert_bits(
                    self.child(variant, 1),
                    Kind::U32,
                    v.max_length.map(u64::from),
                );
                self.assert_optional_text(self.child(variant, 2), v.pattern.as_deref());
                self.assert_optional_text(self.child(variant, 3), v.content_encoding.as_deref());
                self.assert_optional_text(self.child(variant, 4), v.content_media_type.as_deref());
                &v._context
            }
            DataSchema::Number(v) => {
                assert_eq!(self.kind(variant), Kind::SchemaNumber);
                for (index, value) in [
                    v.minimum,
                    v.exclusive_minimum,
                    v.maximum,
                    v.exclusive_maximum,
                    v.multiple_of,
                ]
                .into_iter()
                .enumerate()
                {
                    self.assert_bits(
                        self.child(variant, index),
                        Kind::F64,
                        value.map(f64::to_bits),
                    );
                }
                &v._context
            }
            DataSchema::Integer(v) => {
                assert_eq!(self.kind(variant), Kind::SchemaInteger);
                for (index, value) in [
                    v.minimum,
                    v.exclusive_minimum,
                    v.maximum,
                    v.exclusive_maximum,
                    v.multiple_of,
                ]
                .into_iter()
                .enumerate()
                {
                    self.assert_bits(
                        self.child(variant, index),
                        Kind::I64,
                        value.map(|v| v as u64),
                    );
                }
                &v._context
            }
            DataSchema::Boolean(v) => {
                assert_eq!(self.kind(variant), Kind::SchemaBoolean);
                &v._context
            }
            DataSchema::Null(v) => {
                assert_eq!(self.kind(variant), Kind::SchemaNull);
                &v._context
            }
        };
        assert_eq!(self.kind(context), Kind::Map);
        self.assert_sorted_map(context);
        assert_eq!(self.node(context).edge_count, 11);
        self.assert_metadata(
            self.map_get(context, "metadata").unwrap(),
            &source_context._metadata,
        );
        self.assert_optional_value(
            self.map_get(context, "const").unwrap(),
            source_context.constant.as_ref(),
        );
        self.assert_optional_value(
            self.map_get(context, "default").unwrap(),
            source_context.default.as_ref(),
        );
        self.assert_optional_text(
            self.map_get(context, "unit").unwrap(),
            source_context.unit.as_deref(),
        );
        self.assert_sequence(
            self.map_get(context, "oneOf").unwrap(),
            source_context.one_of.as_deref(),
            |snapshot, node, schema| snapshot.assert_schema(node, schema),
        );
        self.assert_sequence(
            self.map_get(context, "enum").unwrap(),
            source_context.enumerate.as_deref(),
            |snapshot, node, value| snapshot.assert_value(node, value),
        );
        assert_eq!(
            self.kind(self.map_get(context, "readOnly").unwrap()),
            if source_context.read_only {
                Kind::True
            } else {
                Kind::False
            }
        );
        assert_eq!(
            self.kind(self.map_get(context, "writeOnly").unwrap()),
            if source_context.write_only {
                Kind::True
            } else {
                Kind::False
            }
        );
        self.assert_optional_text(
            self.map_get(context, "format").unwrap(),
            source_context.format.as_deref(),
        );
        self.assert_optional_text(
            self.map_get(context, "type").unwrap(),
            source_context.data_type.as_deref(),
        );
        let extras = self.map_get(context, "extensions").unwrap();
        assert_eq!(self.kind(extras), Kind::Map);
        self.assert_sorted_map(extras);
        assert_eq!(
            self.node(extras).edge_count as usize,
            source_context._extra_fields.len()
        );
        for (key, value) in &source_context._extra_fields {
            self.assert_value(self.map_get(extras, key).unwrap(), value);
        }
    }
    fn assert_value(&self, id: u32, source: &Value) {
        match source {
            Value::Null => assert_eq!(self.kind(id), Kind::Null),
            Value::Bool(true) => assert_eq!(self.kind(id), Kind::True),
            Value::Bool(false) => assert_eq!(self.kind(id), Kind::False),
            Value::String(s) => {
                assert_eq!(self.kind(id), Kind::Str);
                assert_eq!(self.text(id), s);
            }
            Value::Number(n) => {
                assert_eq!(self.kind(id), Kind::Number);
                assert_eq!(self.text(id), n.as_str());
            }
            Value::Array(values) => {
                assert_eq!(self.kind(id), Kind::Array);
                assert_eq!(self.node(id).edge_count as usize, values.len());
                for (index, value) in values.iter().enumerate() {
                    assert_eq!(self.edge(id, index).original_index as usize, index);
                    self.assert_value(self.child(id, index), value);
                }
            }
            Value::Object(values) => {
                assert_eq!(self.kind(id), Kind::Map);
                assert_eq!(self.node(id).edge_count as usize, values.len());
                self.assert_sorted_map(id);
                for (key, value) in values {
                    self.assert_value(self.map_get(id, key).unwrap(), value);
                }
            }
        }
    }
}

#[test]
fn typed_corpus_survives_sealed_snapshot() {
    let thing = typed_corpus_shared::typed_corpus();
    let snapshot = Snapshot::normalize(&thing);
    let root = snapshot.root;
    let context = snapshot.child(root, ROOT_CONTEXT);
    assert_eq!(
        snapshot.node(context).edge_count as usize,
        thing.context.entries.len()
    );
    for (index, entry) in thing.context.entries.iter().enumerate() {
        let edge = snapshot.edge(context, index);
        assert_eq!(edge.original_index as usize, index);
        match entry {
            ContextEntry::Uri(uri) => {
                assert_eq!(snapshot.kind(edge.target), Kind::Uri);
                assert_eq!(snapshot.text(edge.target), uri.as_str());
            }
            ContextEntry::Object(map) => {
                for (key, value) in map {
                    snapshot.assert_value(snapshot.map_get(edge.target, key).unwrap(), value);
                }
            }
        }
    }
    assert_eq!(
        snapshot.text(snapshot.child(root, ROOT_ID)),
        thing.id.as_ref().unwrap().as_str()
    );
    assert_eq!(
        snapshot.text(snapshot.child(root, ROOT_TITLE)),
        thing._metadata.title.as_deref().unwrap()
    );
    let titles = snapshot.child(root, ROOT_TITLES);
    for (lang, text) in thing._metadata.titles.as_ref().unwrap().as_map() {
        assert_eq!(snapshot.text(snapshot.map_get(titles, lang).unwrap()), text);
    }
    assert_eq!(
        snapshot.text(snapshot.child(root, ROOT_SUPPORT)),
        thing.support.as_ref().unwrap().as_str()
    );
    assert_eq!(
        snapshot.text(snapshot.child(root, ROOT_BASE)),
        thing.base.as_ref().unwrap().as_str()
    );
    let security = snapshot.child(root, ROOT_SECURITY);
    assert_eq!(
        snapshot.node(security).edge_count as usize,
        thing.security.len()
    );
    for (index, name) in thing.security.iter().enumerate() {
        assert_eq!(snapshot.text(snapshot.child(security, index)), name);
    }
    let properties = snapshot.child(root, ROOT_PROPERTIES);
    for (name, property) in thing.properties.as_ref().unwrap() {
        let stored = snapshot.map_get(properties, name).unwrap();
        snapshot.assert_schema(snapshot.child(stored, 0), &property._schema);
        let forms = snapshot.child(stored, 1);
        assert_eq!(
            snapshot.node(forms).edge_count as usize,
            property._interaction.forms.len()
        );
        for (index, form) in property._interaction.forms.iter().enumerate() {
            let edge = snapshot.edge(forms, index);
            assert_eq!(edge.original_index as usize, index);
            assert_eq!(
                snapshot.text(snapshot.child(edge.target, 0)),
                form.href.as_str()
            );
            assert_eq!(
                snapshot.text(snapshot.child(edge.target, 1)),
                form.content_type
            );
            assert_eq!(snapshot.kind(snapshot.child(edge.target, 2)), Kind::Absent);
            let ops = snapshot.child(edge.target, 3);
            for (op_index, op) in form.op.as_ref().unwrap().iter().enumerate() {
                let op_edge = snapshot.edge(ops, op_index);
                assert_eq!(op_edge.original_index as usize, op_index);
                assert_eq!(snapshot.text(op_edge.target), op.as_str());
            }
        }
    }
    let forms = snapshot.child(root, ROOT_FORMS);
    assert_eq!(snapshot.kind(forms), Kind::Array);
    assert_eq!(snapshot.node(forms).edge_count, 0);
    let extras = snapshot.child(root, ROOT_EXTENSIONS);
    for (key, value) in &thing._extra_fields {
        snapshot.assert_value(snapshot.map_get(extras, key).unwrap(), value);
    }
    assert_eq!(
        snapshot.text(
            snapshot
                .map_get(extras, "ex:payload")
                .and_then(|id| snapshot.map_get(id, "count"))
                .unwrap()
        ),
        thing._extra_fields["ex:payload"]["count"]
            .as_number()
            .unwrap()
            .as_str()
    );
    assert_eq!(
        snapshot.text(
            snapshot
                .map_get(snapshot.child(root, ROOT_SECURITY_DEFINITIONS), "none")
                .unwrap()
        ),
        "nosec"
    );
    assert_eq!(snapshot.arena.footprint().retained_allocation_count, 3);
}

#[test]
fn remaining_root_fields_survive_and_distinguish_semantic_mutations() {
    let thing = typed_corpus_shared::typed_corpus();
    let snapshot = Snapshot::normalize(&thing);
    let root = snapshot.root;

    let tags = snapshot.child(root, ROOT_TAGS);
    let source_tags = thing._metadata.tags.as_deref().unwrap();
    assert_eq!(snapshot.node(tags).edge_count as usize, source_tags.len());
    for (index, tag) in source_tags.iter().enumerate() {
        let edge = snapshot.edge(tags, index);
        assert_eq!(edge.original_index as usize, index);
        assert_eq!(snapshot.text(edge.target), tag);
    }
    assert_eq!(
        snapshot.text(snapshot.child(root, ROOT_DESCRIPTION)),
        thing._metadata.description.as_deref().unwrap()
    );
    let descriptions = snapshot.child(root, ROOT_DESCRIPTIONS);
    snapshot.assert_sorted_map(descriptions);
    for (language, text) in thing._metadata.descriptions.as_ref().unwrap().as_map() {
        assert_eq!(
            snapshot.text(snapshot.map_get(descriptions, language).unwrap()),
            text
        );
    }

    let source_version = thing.version.as_ref().unwrap();
    let version = snapshot.child(root, ROOT_VERSION);
    assert_eq!(snapshot.kind(version), Kind::Version);
    assert_eq!(
        snapshot.text(snapshot.child(version, 0)),
        source_version.instance
    );
    assert_eq!(
        snapshot.text(snapshot.child(version, 1)),
        source_version.model.as_deref().unwrap()
    );
    let version_extensions = snapshot.child(version, 2);
    for (key, value) in &source_version._extra_fields {
        snapshot.assert_value(snapshot.map_get(version_extensions, key).unwrap(), value);
    }

    let profile = snapshot.child(root, ROOT_PROFILE);
    let source_profile = thing.profile.as_deref().unwrap();
    assert_eq!(
        snapshot.node(profile).edge_count as usize,
        source_profile.len()
    );
    for (index, uri) in source_profile.iter().enumerate() {
        let edge = snapshot.edge(profile, index);
        assert_eq!(edge.original_index as usize, index);
        assert_eq!(snapshot.kind(edge.target), Kind::Uri);
        assert_eq!(snapshot.text(edge.target), uri.as_str());
    }

    for (root_field, source) in [
        (
            ROOT_SCHEMA_DEFINITIONS,
            thing.schema_definitions.as_ref().unwrap(),
        ),
        (ROOT_URI_VARIABLES, thing.uri_variables.as_ref().unwrap()),
    ] {
        let stored = snapshot.child(root, root_field);
        snapshot.assert_sorted_map(stored);
        for (name, schema) in source {
            snapshot.assert_schema(snapshot.map_get(stored, name).unwrap(), schema);
        }
    }
    assert_eq!(snapshot.arena.footprint().retained_allocation_count, 3);

    let mut changed = thing.clone();
    changed._metadata.tags.as_mut().unwrap().swap(0, 1);
    assert!(
        !snapshot.same(&Snapshot::normalize(&changed)),
        "metadata tag order"
    );

    let mut changed = thing.clone();
    changed.profile.as_mut().unwrap().swap(0, 1);
    changed.validate_with_level(ValidationLevel::Basic).unwrap();
    assert!(
        !snapshot.same(&Snapshot::normalize(&changed)),
        "profile order"
    );

    let mut changed = thing.clone();
    changed.version.as_mut().unwrap().model = None;
    changed.validate_with_level(ValidationLevel::Basic).unwrap();
    assert!(
        !snapshot.same(&Snapshot::normalize(&changed)),
        "version optional distinction"
    );

    let mut changed = thing.clone();
    let definitions = changed.schema_definitions.as_mut().unwrap();
    let mode = definitions.remove("mode").unwrap();
    let threshold = definitions.remove("threshold").unwrap();
    definitions.insert("mode".into(), threshold);
    definitions.insert("threshold".into(), mode);
    changed.validate_with_level(ValidationLevel::Basic).unwrap();
    assert!(
        !snapshot.same(&Snapshot::normalize(&changed)),
        "schema name/value association"
    );

    let mut reordered = thing.clone();
    let definitions = reordered.schema_definitions.take().unwrap();
    reordered.schema_definitions = Some(definitions.into_iter().rev().collect());
    assert!(
        snapshot.same(&Snapshot::normalize(&reordered)),
        "schema map insertion history is not semantic"
    );

    let mut changed = thing.clone();
    changed.schema_definitions = Some(BTreeMap::new());
    let present_empty = Snapshot::normalize(&changed);
    changed.schema_definitions = None;
    assert!(
        !present_empty.same(&Snapshot::normalize(&changed)),
        "absent vs present-empty schemaDefinitions"
    );
}

#[test]
fn property_form_operation_order_survives_snapshot() {
    let original = typed_corpus_shared::typed_corpus();
    original
        .validate_with_level(ValidationLevel::Basic)
        .unwrap();
    let source_ops = original.properties.as_ref().unwrap()["zeta"]
        ._interaction
        .forms[0]
        .op
        .as_ref()
        .unwrap();
    assert_eq!(source_ops.len(), 2);
    assert_ne!(source_ops[0], source_ops[1]);

    let snapshot = Snapshot::normalize(&original);
    let zeta = snapshot
        .map_get(snapshot.child(snapshot.root, ROOT_PROPERTIES), "zeta")
        .unwrap();
    let first_form = snapshot.child(snapshot.child(zeta, 1), 0);
    let stored_ops = snapshot.child(first_form, 3);
    assert_eq!(snapshot.node(stored_ops).edge_count, 2);
    for (index, op) in source_ops.iter().enumerate() {
        let edge = snapshot.edge(stored_ops, index);
        assert_eq!(edge.original_index as usize, index);
        assert_eq!(snapshot.text(edge.target), op.as_str());
    }

    let mut swapped = original.clone();
    swapped
        .properties
        .as_mut()
        .unwrap()
        .get_mut("zeta")
        .unwrap()
        ._interaction
        .forms[0]
        .op
        .as_mut()
        .unwrap()
        .swap(0, 1);
    swapped.validate_with_level(ValidationLevel::Basic).unwrap();
    assert!(!snapshot.same(&Snapshot::normalize(&swapped)));
}

#[test]
fn typed_distinctions_and_serializer_failure() {
    let thing = typed_corpus_shared::typed_corpus();
    let baseline = Snapshot::normalize(&thing);
    let mut changed = thing.clone();
    changed.context.entries.swap(0, 1);
    assert!(!baseline.same(&Snapshot::normalize(&changed)));
    let mut changed = thing.clone();
    changed
        .properties
        .as_mut()
        .unwrap()
        .get_mut("zeta")
        .unwrap()
        ._interaction
        .forms
        .swap(0, 1);
    assert!(!baseline.same(&Snapshot::normalize(&changed)));
    let mut changed = thing.clone();
    changed.forms = None;
    let absent = Snapshot::normalize(&changed);
    assert_eq!(
        absent.kind(absent.child(absent.root, ROOT_FORMS)),
        Kind::Absent
    );
    assert!(!baseline.same(&absent));
    let mut changed = thing.clone();
    changed._extra_fields.get_mut("ex:payload").unwrap()["items"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    assert!(!baseline.same(&Snapshot::normalize(&changed)));
    let mut changed = thing.clone();
    changed._extra_fields.get_mut("ex:payload").unwrap()["flag"] = Value::Bool(false);
    assert!(!baseline.same(&Snapshot::normalize(&changed)));
    let mut changed = thing.clone();
    changed
        ._extra_fields
        .insert("ex:long".into(), Value::String("λ".repeat(2_047)));
    assert!(!baseline.same(&Snapshot::normalize(&changed)));
    let mut changed = thing.clone();
    let old = changed.properties.take().unwrap();
    changed.properties = Some(old.into_iter().rev().collect());
    assert!(baseline.same(&Snapshot::normalize(&changed)));

    let failure = typed_corpus_shared::serializer_failure_thing();
    assert!(serde_json::to_vec(&failure).is_err());
    let accepted = Snapshot::normalize(&failure);
    let context = accepted.child(accepted.root, ROOT_CONTEXT);
    assert_eq!(
        accepted.text(accepted.child(context, 0)),
        "https://example.org/extension-only"
    );
    let zeta = accepted
        .map_get(accepted.child(accepted.root, ROOT_PROPERTIES), "zeta")
        .unwrap();
    accepted.assert_schema(
        accepted.child(zeta, 0),
        &failure.properties.as_ref().unwrap()["zeta"]._schema,
    );
}

#[test]
fn nested_data_schema_survives_and_distinguishes_semantic_mutations() {
    let thing = typed_corpus_shared::nested_schema_corpus();
    let baseline = Snapshot::normalize(&thing);
    let properties = baseline.child(baseline.root, ROOT_PROPERTIES);
    let alpha = baseline.map_get(properties, "alpha").unwrap();
    let stored = baseline.child(alpha, 0);
    let schema = &thing.properties.as_ref().unwrap()["alpha"]._schema;
    baseline.assert_schema(stored, schema);
    assert_eq!(baseline.arena.footprint().retained_allocation_count, 3);

    // Every mutation changes exactly one typed distinction that the stored
    // schema must preserve. Validation here is TD's current Basic oracle.
    let mut changed = thing.clone();
    let DataSchema::Object(root) = &mut changed
        .properties
        .as_mut()
        .unwrap()
        .get_mut("alpha")
        .unwrap()
        ._schema
    else {
        unreachable!()
    };
    root.required.as_mut().unwrap().clear();
    changed.validate_with_level(ValidationLevel::Basic).unwrap();
    assert!(
        !baseline.same(&Snapshot::normalize(&changed)),
        "required sequence"
    );

    let mut changed = thing.clone();
    let DataSchema::Object(root) = &mut changed
        .properties
        .as_mut()
        .unwrap()
        .get_mut("alpha")
        .unwrap()
        ._schema
    else {
        unreachable!()
    };
    root._context.one_of.as_mut().unwrap().swap(0, 1);
    changed.validate_with_level(ValidationLevel::Basic).unwrap();
    assert!(
        !baseline.same(&Snapshot::normalize(&changed)),
        "oneOf order"
    );

    let mut changed = thing.clone();
    let DataSchema::Object(root) = &mut changed
        .properties
        .as_mut()
        .unwrap()
        .get_mut("alpha")
        .unwrap()
        ._schema
    else {
        unreachable!()
    };
    root._context.enumerate.as_mut().unwrap().swap(0, 1);
    assert!(!baseline.same(&Snapshot::normalize(&changed)), "enum order");

    let mut changed = thing.clone();
    let DataSchema::Object(root) = &mut changed
        .properties
        .as_mut()
        .unwrap()
        .get_mut("alpha")
        .unwrap()
        ._schema
    else {
        unreachable!()
    };
    root._context._metadata.tags.as_mut().unwrap().swap(0, 1);
    assert!(
        !baseline.same(&Snapshot::normalize(&changed)),
        "metadata tag order"
    );

    let mut changed = thing.clone();
    let DataSchema::Object(root) = &mut changed
        .properties
        .as_mut()
        .unwrap()
        .get_mut("alpha")
        .unwrap()
        ._schema
    else {
        unreachable!()
    };
    let DataSchema::Array(samples) = root
        .properties
        .as_mut()
        .unwrap()
        .get_mut("samples")
        .unwrap()
    else {
        unreachable!()
    };
    samples.items.as_mut().unwrap().swap(0, 1);
    changed.validate_with_level(ValidationLevel::Basic).unwrap();
    assert!(
        !baseline.same(&Snapshot::normalize(&changed)),
        "items order"
    );

    let mut changed = thing.clone();
    let DataSchema::Object(root) = &mut changed
        .properties
        .as_mut()
        .unwrap()
        .get_mut("alpha")
        .unwrap()
        ._schema
    else {
        unreachable!()
    };
    root.properties
        .as_mut()
        .unwrap()
        .insert("samples".into(), DataSchema::Boolean(Default::default()));
    changed.validate_with_level(ValidationLevel::Basic).unwrap();
    assert!(
        !baseline.same(&Snapshot::normalize(&changed)),
        "key to schema association"
    );

    let mut changed = thing.clone();
    let DataSchema::Object(root) = &mut changed
        .properties
        .as_mut()
        .unwrap()
        .get_mut("alpha")
        .unwrap()
        ._schema
    else {
        unreachable!()
    };
    root._context.default = None;
    assert!(
        !baseline.same(&Snapshot::normalize(&changed)),
        "absent vs present default"
    );

    let mut changed = thing.clone();
    if let DataSchema::Object(root) = &mut changed
        .properties
        .as_mut()
        .unwrap()
        .get_mut("alpha")
        .unwrap()
        ._schema
    {
        root._context.one_of = Some(Vec::new());
    }
    let present_empty = Snapshot::normalize(&changed);
    assert!(
        !baseline.same(&present_empty),
        "present empty vs populated oneOf"
    );
    if let DataSchema::Object(root) = &mut changed
        .properties
        .as_mut()
        .unwrap()
        .get_mut("alpha")
        .unwrap()
        ._schema
    {
        root._context.one_of = None;
    }
    assert!(
        !present_empty.same(&Snapshot::normalize(&changed)),
        "absent vs present empty oneOf"
    );

    let mut changed = thing.clone();
    if let DataSchema::Object(root) = &mut changed
        .properties
        .as_mut()
        .unwrap()
        .get_mut("alpha")
        .unwrap()
        ._schema
    {
        root.properties = Some(BTreeMap::new());
    }
    let present_empty = Snapshot::normalize(&changed);
    if let DataSchema::Object(root) = &mut changed
        .properties
        .as_mut()
        .unwrap()
        .get_mut("alpha")
        .unwrap()
        ._schema
    {
        root.properties = None;
    }
    assert!(
        !present_empty.same(&Snapshot::normalize(&changed)),
        "absent vs present empty properties"
    );

    let mut changed = thing.clone();
    if let DataSchema::Object(root) = &mut changed
        .properties
        .as_mut()
        .unwrap()
        .get_mut("alpha")
        .unwrap()
        ._schema
    {
        if let DataSchema::Array(samples) = root
            .properties
            .as_mut()
            .unwrap()
            .get_mut("samples")
            .unwrap()
        {
            samples.items = Some(Vec::new());
        }
    }
    let present_empty = Snapshot::normalize(&changed);
    if let DataSchema::Object(root) = &mut changed
        .properties
        .as_mut()
        .unwrap()
        .get_mut("alpha")
        .unwrap()
        ._schema
    {
        if let DataSchema::Array(samples) = root
            .properties
            .as_mut()
            .unwrap()
            .get_mut("samples")
            .unwrap()
        {
            samples.items = None;
        }
    }
    assert!(
        !present_empty.same(&Snapshot::normalize(&changed)),
        "absent vs present empty items"
    );

    let mut changed = thing.clone();
    let DataSchema::Object(root) = &mut changed
        .properties
        .as_mut()
        .unwrap()
        .get_mut("alpha")
        .unwrap()
        ._schema
    else {
        unreachable!()
    };
    let DataSchema::Array(samples) = root
        .properties
        .as_mut()
        .unwrap()
        .get_mut("samples")
        .unwrap()
    else {
        unreachable!()
    };
    let DataSchema::Object(reading) = &mut samples.items.as_mut().unwrap()[0] else {
        unreachable!()
    };
    let properties = reading.properties.as_mut().unwrap();
    let label = properties.remove("label").unwrap();
    let ratio = properties.remove("ratio").unwrap();
    properties.insert("label".into(), ratio);
    properties.insert("ratio".into(), label);
    changed.validate_with_level(ValidationLevel::Basic).unwrap();
    assert!(
        !baseline.same(&Snapshot::normalize(&changed)),
        "nested key/value association"
    );

    let mut changed = thing.clone();
    let DataSchema::Object(root) = &mut changed
        .properties
        .as_mut()
        .unwrap()
        .get_mut("alpha")
        .unwrap()
        ._schema
    else {
        unreachable!()
    };
    let DataSchema::Array(samples) = root
        .properties
        .as_mut()
        .unwrap()
        .get_mut("samples")
        .unwrap()
    else {
        unreachable!()
    };
    let DataSchema::Object(reading) = &mut samples.items.as_mut().unwrap()[0] else {
        unreachable!()
    };
    let DataSchema::Number(ratio) = reading
        .properties
        .as_mut()
        .unwrap()
        .get_mut("ratio")
        .unwrap()
    else {
        unreachable!()
    };
    ratio.minimum = Some(0.5);
    changed.validate_with_level(ValidationLevel::Basic).unwrap();
    assert!(
        !baseline.same(&Snapshot::normalize(&changed)),
        "typed binary64 bound"
    );

    let mut changed = thing.clone();
    let DataSchema::Object(root) = &mut changed
        .properties
        .as_mut()
        .unwrap()
        .get_mut("alpha")
        .unwrap()
        ._schema
    else {
        unreachable!()
    };
    let old = root.properties.take().unwrap();
    root.properties = Some(old.into_iter().rev().collect());
    assert!(
        baseline.same(&Snapshot::normalize(&changed)),
        "map insertion history is not semantic"
    );

    let mut changed = thing.clone();
    let DataSchema::Object(root) = &mut changed
        .properties
        .as_mut()
        .unwrap()
        .get_mut("alpha")
        .unwrap()
        ._schema
    else {
        unreachable!()
    };
    root._context
        ._extra_fields
        .insert("ex:opaque".into(), serde_json::from_str("1e308").unwrap());
    changed.validate_with_level(ValidationLevel::Basic).unwrap();
    assert!(
        !baseline.same(&Snapshot::normalize(&changed)),
        "opaque number lexeme"
    );
}

// Input construction is outside the snapshot allocation boundary. Reversing
// each object's insertion history distinguishes IndexMap-backed serde JSON
// from its BTreeMap-backed graph without changing any key/value association.
fn json_object<const N: usize>(reverse: bool, entries: [(&str, Value); N]) -> Value {
    let mut map = serde_json::Map::new();
    if reverse {
        for (key, value) in entries.into_iter().rev() {
            map.insert(key.into(), value);
        }
    } else {
        for (key, value) in entries {
            map.insert(key.into(), value);
        }
    }
    Value::Object(map)
}

fn nested_json(reverse: bool) -> Value {
    let inner = json_object(
        reverse,
        [
            ("z", Value::String("last".into())),
            ("a", Value::Bool(true)),
            ("é", Value::Null),
        ],
    );
    let array_object = json_object(
        reverse,
        [
            ("right", Value::String("R".into())),
            ("left", Value::String("L".into())),
        ],
    );
    json_object(
        reverse,
        [
            ("nested", inner),
            (
                "ordered",
                Value::Array(alloc::vec![
                    Value::String("first".into()),
                    array_object,
                    Value::String("last".into()),
                ]),
            ),
            (
                "count",
                serde_json::from_str("123456789012345678901234567890").unwrap(),
            ),
        ],
    )
}

#[test]
fn nested_json_objects_normalize_across_insertion_orders() {
    let mut forward = typed_corpus_shared::typed_corpus();
    forward
        ._extra_fields
        .insert("ex:canonical".into(), nested_json(false));
    let mut reversed = typed_corpus_shared::typed_corpus();
    reversed
        ._extra_fields
        .insert("ex:canonical".into(), nested_json(true));
    forward.validate_with_level(ValidationLevel::Basic).unwrap();
    reversed
        .validate_with_level(ValidationLevel::Basic)
        .unwrap();

    let snapshot = Snapshot::normalize(&forward);
    let reordered = Snapshot::normalize(&reversed);
    assert!(snapshot.same(&reordered));
    assert_eq!(snapshot.arena.footprint(), reordered.arena.footprint());
    assert_eq!(snapshot.arena.footprint().retained_allocation_count, 3);
    let object = snapshot
        .map_get(
            snapshot.child(snapshot.root, ROOT_EXTENSIONS),
            "ex:canonical",
        )
        .unwrap();
    snapshot.assert_value(object, &forward._extra_fields["ex:canonical"]);
    snapshot.assert_value(object, &reversed._extra_fields["ex:canonical"]);
    let nested = snapshot.map_get(object, "nested").unwrap();
    assert_eq!(
        snapshot.kind(snapshot.map_get(nested, "a").unwrap()),
        Kind::True
    );
    assert_eq!(
        snapshot.text(snapshot.map_get(nested, "z").unwrap()),
        "last"
    );
    assert_eq!(
        snapshot.kind(snapshot.map_get(nested, "é").unwrap()),
        Kind::Null
    );
    let ordered = snapshot.map_get(object, "ordered").unwrap();
    assert_eq!(snapshot.text(snapshot.child(ordered, 0)), "first");
    assert_eq!(snapshot.text(snapshot.child(ordered, 2)), "last");
    let pair = snapshot.child(ordered, 1);
    assert_eq!(snapshot.text(snapshot.map_get(pair, "left").unwrap()), "L");
    assert_eq!(snapshot.text(snapshot.map_get(pair, "right").unwrap()), "R");
    assert_eq!(
        snapshot.text(snapshot.map_get(object, "count").unwrap()),
        "123456789012345678901234567890"
    );

    let mut changed = reversed.clone();
    changed._extra_fields.get_mut("ex:canonical").unwrap()["ordered"]
        .as_array_mut()
        .unwrap()
        .swap(0, 2);
    assert!(!snapshot.same(&Snapshot::normalize(&changed)));
    let mut changed = reversed;
    changed._extra_fields.get_mut("ex:canonical").unwrap()["nested"]["z"] =
        Value::String("different".into());
    assert!(!snapshot.same(&Snapshot::normalize(&changed)));
}
