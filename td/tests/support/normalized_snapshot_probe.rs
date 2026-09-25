//! Non-production, corpus-scoped typed traversal. Compiled only as a child of
//! `context`, where the existing private Context entries can be inspected.
//! This is storage evidence, not a Basic/default/URI/security rule kernel.

use super::ContextEntry;
use crate as td_crate;
use serde_json::Value;
use td_crate::{
    data_schema::DataSchema,
    data_type::ExtensionMap,
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
}

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
        arena.grow_nodes(256).unwrap();
        arena.grow_edges(384).unwrap();
        arena.grow_bytes(16_384).unwrap();
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

    fn absent_or_text(&mut self, value: Option<&str>, kind: Kind) -> u32 {
        match value {
            Some(value) => self.text(kind, value),
            None => self.node(Kind::Absent, 0),
        }
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

    fn thing(&mut self, thing: &Thing) -> u32 {
        let id = self.node(Kind::Root, 11);
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
        self.put(first, 0, context);
        let value = self.absent_or_text(thing.id.as_ref().map(|v| v.as_str()), Kind::Uri);
        self.put(first, 1, value);
        let value = self.absent_or_text(thing._metadata.title.as_deref(), Kind::Str);
        self.put(first, 2, value);
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
        self.put(first, 3, titles);
        let value = self.absent_or_text(thing.support.as_ref().map(|v| v.as_str()), Kind::Uri);
        self.put(first, 4, value);
        let value = self.absent_or_text(thing.base.as_ref().map(|v| v.as_str()), Kind::Uri);
        self.put(first, 5, value);
        let security = self.node(Kind::Array, thing.security.len());
        let security_first = self.node_at(security).first_edge;
        for (index, name) in thing.security.iter().enumerate() {
            let item = self.text(Kind::Str, name);
            self.put(security_first, index, item);
        }
        self.put(first, 6, security);
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
        self.put(first, 7, definitions);
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
                    let type_name = match &property._schema {
                        DataSchema::String(_) => "string",
                        DataSchema::Boolean(_) => "boolean",
                        _ => panic!("corpus-scoped probe: unhandled schema variant"),
                    };
                    let schema_type = self.text(Kind::Str, type_name);
                    self.put(property_first, 0, schema_type);
                    let forms = self.forms(Some(&property._interaction.forms));
                    self.put(property_first, 1, forms);
                    self.put(pair, 1, value);
                    self.put(start, index, entry);
                }
                id
            }
        };
        self.put(first, 8, properties);
        let forms = self.forms(thing.forms.as_deref());
        self.put(first, 9, forms);
        let extras = self.extensions(&thing._extra_fields);
        self.put(first, 10, extras);
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
    fn same(&self, other: &Self) -> bool {
        self.root == other.root
            && self.arena.nodes() == other.arena.nodes()
            && self.arena.edges() == other.arena.edges()
            && self.arena.bytes() == other.arena.bytes()
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
    let context = snapshot.child(root, 0);
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
        snapshot.text(snapshot.child(root, 1)),
        thing.id.as_ref().unwrap().as_str()
    );
    assert_eq!(
        snapshot.text(snapshot.child(root, 2)),
        thing._metadata.title.as_deref().unwrap()
    );
    let titles = snapshot.child(root, 3);
    for (lang, text) in thing._metadata.titles.as_ref().unwrap().as_map() {
        assert_eq!(snapshot.text(snapshot.map_get(titles, lang).unwrap()), text);
    }
    assert_eq!(
        snapshot.text(snapshot.child(root, 4)),
        thing.support.as_ref().unwrap().as_str()
    );
    assert_eq!(
        snapshot.text(snapshot.child(root, 5)),
        thing.base.as_ref().unwrap().as_str()
    );
    let security = snapshot.child(root, 6);
    assert_eq!(
        snapshot.node(security).edge_count as usize,
        thing.security.len()
    );
    for (index, name) in thing.security.iter().enumerate() {
        assert_eq!(snapshot.text(snapshot.child(security, index)), name);
    }
    let properties = snapshot.child(root, 8);
    for (name, property) in thing.properties.as_ref().unwrap() {
        let stored = snapshot.map_get(properties, name).unwrap();
        let expected_type = match &property._schema {
            DataSchema::String(_) => "string",
            DataSchema::Boolean(_) => "boolean",
            _ => unreachable!(),
        };
        assert_eq!(snapshot.text(snapshot.child(stored, 0)), expected_type);
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
    let forms = snapshot.child(root, 9);
    assert_eq!(snapshot.kind(forms), Kind::Array);
    assert_eq!(snapshot.node(forms).edge_count, 0);
    let extras = snapshot.child(root, 10);
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
        snapshot.text(snapshot.map_get(snapshot.child(root, 7), "none").unwrap()),
        "nosec"
    );
    assert_eq!(snapshot.arena.footprint().retained_allocation_count, 3);
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
        .map_get(snapshot.child(snapshot.root, 8), "zeta")
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
    assert_eq!(absent.kind(absent.child(absent.root, 9)), Kind::Absent);
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
    let context = accepted.child(accepted.root, 0);
    assert_eq!(
        accepted.text(accepted.child(context, 0)),
        "https://example.org/extension-only"
    );
    assert_eq!(
        accepted.text(
            accepted.child(
                accepted
                    .map_get(accepted.child(accepted.root, 8), "zeta")
                    .unwrap(),
                0
            )
        ),
        "string"
    );
}
