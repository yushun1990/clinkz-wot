//! Recursive *test-only* public-field oracle. No candidate Task/slot/serializer.
extern crate alloc;
use alloc::{format, string::String};
use serde_json::Value;
use td_candidate::{
    data_schema::DataSchema, data_type::Metadata, form::Form, security_scheme::SecurityScheme,
    thing::Thing,
};
#[derive(Default, Debug)]
pub struct Totals {
    pub document: u64,
    pub text: u64,
    pub extension: u64,
    pub reads: u64,
    pub checksum: u64,
}
impl Totals {
    fn leaf(&mut self, v: &str, number: bool, extra: bool) {
        let n = v.len() as u64;
        self.document += n;
        if !number {
            self.text += n;
        }
        if extra {
            self.extension += n;
        }
        self.reads += n;
        self.checksum += v.as_bytes().iter().map(|b| u64::from(*b) + 1).sum::<u64>();
    }
    fn scalar(&mut self, extra: bool) {
        self.document += 8;
        if extra {
            self.extension += 8;
        }
    }
    fn text(&mut self, v: &str) {
        self.leaf(v, false, false);
    }
    fn optional(&mut self, v: Option<&str>) {
        if let Some(v) = v {
            self.text(v);
        }
    }
    fn strings(&mut self, v: &[String]) {
        for v in v {
            self.text(v);
        }
    }
    fn value(&mut self, v: &Value, extra: bool) {
        match v {
            Value::Null | Value::Bool(_) => self.scalar(extra),
            Value::String(s) => self.leaf(s, false, extra),
            Value::Number(n) => self.leaf(n.as_str(), true, extra),
            Value::Array(a) => {
                for v in a {
                    self.value(v, extra)
                }
            }
            Value::Object(m) => {
                for (k, v) in m {
                    self.leaf(k, false, extra);
                    self.value(v, extra)
                }
            }
        }
    }
    fn extras(&mut self, m: &td_candidate::data_type::ExtensionMap) {
        for (k, v) in m {
            self.leaf(k, false, true);
            self.value(v, true);
        }
    }
    fn metadata(&mut self, m: &Metadata) {
        if let Some(v) = &m.tags {
            self.strings(v);
        }
        self.optional(m.title.as_deref());
        self.optional(m.description.as_deref());
        for v in [&m.titles, &m.descriptions].into_iter().flatten() {
            for (k, v) in v.as_map() {
                self.text(k);
                self.text(v);
            }
        }
    }
    fn schema(&mut self, s: &DataSchema) {
        let c = match s {
            DataSchema::Null(v) => &v._context,
            DataSchema::Boolean(v) => &v._context,
            DataSchema::Integer(v) => &v._context,
            DataSchema::Number(v) => &v._context,
            DataSchema::String(v) => &v._context,
            DataSchema::Object(v) => &v._context,
            DataSchema::Array(v) => &v._context,
        };
        self.metadata(&c._metadata);
        for v in [&c.constant, &c.default].into_iter().flatten() {
            self.value(v, true);
        }
        self.optional(c.unit.as_deref());
        if let Some(v) = &c.one_of {
            for s in v {
                self.schema(s);
            }
        }
        if let Some(v) = &c.enumerate {
            for v in v {
                self.value(v, true);
            }
        }
        self.scalar(false);
        self.scalar(false);
        self.optional(c.format.as_deref());
        self.optional(c.data_type.as_deref());
        self.extras(&c._extra_fields);
        match s {
            DataSchema::Array(v) => {
                if let Some(v) = &v.items {
                    for s in v {
                        self.schema(s);
                    }
                }
                for _ in [v.min_items, v.max_items].into_iter().flatten() {
                    self.scalar(false);
                }
            }
            DataSchema::Object(v) => {
                if let Some(v) = &v.properties {
                    for (k, s) in v {
                        self.text(k);
                        self.schema(s);
                    }
                }
                if let Some(v) = &v.required {
                    self.strings(v);
                }
            }
            DataSchema::String(v) => {
                for _ in [v.min_length, v.max_length].into_iter().flatten() {
                    self.scalar(false);
                }
                for v in [
                    v.pattern.as_deref(),
                    v.content_encoding.as_deref(),
                    v.content_media_type.as_deref(),
                ] {
                    self.optional(v);
                }
            }
            DataSchema::Number(v) => {
                for _ in [
                    v.minimum,
                    v.exclusive_minimum,
                    v.maximum,
                    v.exclusive_maximum,
                    v.multiple_of,
                ]
                .into_iter()
                .flatten()
                {
                    self.scalar(false);
                }
            }
            DataSchema::Integer(v) => {
                for _ in [
                    v.minimum,
                    v.exclusive_minimum,
                    v.maximum,
                    v.exclusive_maximum,
                    v.multiple_of,
                ]
                .into_iter()
                .flatten()
                {
                    self.scalar(false);
                }
            }
            _ => {}
        }
    }
    fn schema_map(&mut self, v: &alloc::collections::BTreeMap<String, DataSchema>) {
        for (k, s) in v {
            self.text(k);
            self.schema(s);
        }
    }
    fn form(&mut self, f: &Form) {
        self.text(f.href.as_str());
        self.text(&f.content_type);
        self.optional(f.content_coding.as_deref());
        for v in [&f.security, &f.scopes].into_iter().flatten() {
            self.strings(v);
        }
        if let Some(v) = &f.response {
            self.text(&v.content_type);
            self.extras(&v._extra_fields);
        }
        if let Some(v) = &f.additional_responses {
            for v in v {
                self.optional(v.content_type.as_deref());
                self.optional(v.schema.as_deref());
                self.scalar(false);
                self.extras(&v._extra_fields);
            }
        }
        self.optional(f.subprotocol.as_deref());
        if let Some(v) = &f.op {
            for v in v {
                self.text(v.as_str());
            }
        }
        self.extras(&f._extra_fields);
    }
    fn interaction(&mut self, v: &td_candidate::affordance::InteractionAffordance) {
        for f in &v.forms {
            self.form(f);
        }
        if let Some(v) = &v.uri_variables {
            self.schema_map(v);
        }
    }
    fn security(&mut self, s: &SecurityScheme) {
        let c = match s {
            SecurityScheme::NoSec(v) => &v._context,
            SecurityScheme::Auto(v) => &v._context,
            SecurityScheme::Combo(v) => {
                self.strings(&v.one_of);
                self.strings(&v.all_of);
                &v._context
            }
            SecurityScheme::Basic(v) => {
                self.optional(v.name.as_deref());
                self.text(&format!("{:?}", v.location).to_lowercase());
                &v._context
            }
            SecurityScheme::Digest(v) => {
                self.optional(v.name.as_deref());
                self.text(&format!("{:?}", v.location).to_lowercase());
                self.text(match v.qop {
                    td_candidate::security_scheme::Qop::Auth => "auth",
                    _ => "auth-int",
                });
                &v._context
            }
            SecurityScheme::APIKey(v) => {
                self.optional(v.name.as_deref());
                self.text(&format!("{:?}", v.location).to_lowercase());
                &v._context
            }
            SecurityScheme::Bearer(v) => {
                self.optional(v.authorization.as_ref().map(|v| v.as_str()));
                self.optional(v.name.as_deref());
                self.text(&v.alg);
                self.text(&v.format);
                self.text(&format!("{:?}", v.location).to_lowercase());
                &v._context
            }
            SecurityScheme::PSK(v) => {
                self.optional(v.identity.as_deref());
                &v._context
            }
            SecurityScheme::OAuth2(v) => {
                for v in [&v.authorization, &v.token, &v.refresh]
                    .into_iter()
                    .flatten()
                {
                    self.text(v.as_str());
                }
                if let Some(v) = &v.scopes {
                    self.strings(v);
                }
                self.text(&v.flow);
                &v._context
            }
        };
        if let Some(v) = &c.tags {
            self.strings(v);
        }
        self.optional(c.description.as_deref());
        if let Some(v) = &c.descriptions {
            for (k, v) in v.as_map() {
                self.text(k);
                self.text(v);
            }
        }
        self.optional(c.proxy.as_ref().map(|v| v.as_str()));
        self.text(&c.scheme);
        self.extras(&c._extra_fields);
    }
}
/// Context is deliberately private. The caller supplies its independently
/// constructed entries rather than serializing it or reading a candidate seam.
pub fn thing(t: &Thing, context: &[Value]) -> Totals {
    let mut o = Totals::default();
    for v in context {
        o.value(v, false);
    }
    o.optional(t.id.as_ref().map(|v| v.as_str()));
    o.metadata(&t._metadata);
    if let Some(v) = &t.version {
        o.text(&v.instance);
        o.optional(v.model.as_deref());
        o.extras(&v._extra_fields);
    }
    for _ in [&t.created, &t.modified].into_iter().flatten() {
        for _ in 0..8 {
            o.scalar(false);
        }
    }
    o.optional(t.support.as_ref().map(|v| v.as_str()));
    o.optional(t.base.as_ref().map(|v| v.as_str()));
    if let Some(v) = &t.properties {
        for (k, v) in v {
            o.text(k);
            o.schema(&v._schema);
            o.interaction(&v._interaction);
            o.scalar(false);
        }
    }
    if let Some(v) = &t.actions {
        for (k, v) in v {
            o.text(k);
            o.metadata(&v._metadata);
            o.interaction(&v._interaction);
            for v in [&v.input, &v.output].into_iter().flatten() {
                o.schema(v);
            }
            o.scalar(false);
            o.scalar(false);
            o.extras(&v._extra_fields);
        }
    }
    if let Some(v) = &t.events {
        for (k, v) in v {
            o.text(k);
            o.metadata(&v._metadata);
            o.interaction(&v._interaction);
            for v in [&v.subscription, &v.data, &v.data_response, &v.cancellation]
                .into_iter()
                .flatten()
            {
                o.schema(v);
            }
            o.extras(&v._extra_fields);
        }
    }
    if let Some(v) = &t.links {
        for v in v {
            o.text(v.href.as_str());
            o.optional(v.content_type.as_deref());
            o.optional(v.rel.as_deref());
            o.optional(v.anchor.as_ref().map(|v| v.as_str()));
            o.optional(v.sizes.as_deref());
            if let Some(v) = &v.hreflang {
                o.strings(v);
            }
            o.extras(&v._extra_fields);
        }
    }
    if let Some(v) = &t.forms {
        for v in v {
            o.form(v);
        }
    }
    o.strings(&t.security);
    for (k, s) in &t.security_definitions {
        o.text(k);
        o.security(s);
    }
    if let Some(v) = &t.profile {
        for v in v {
            o.text(v.as_str());
        }
    }
    for v in [&t.schema_definitions, &t.uri_variables]
        .into_iter()
        .flatten()
    {
        o.schema_map(v);
    }
    o.extras(&t._extra_fields);
    o
}
