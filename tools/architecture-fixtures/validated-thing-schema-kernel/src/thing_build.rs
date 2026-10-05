//! Direct typed Thing construction facts for the existing canonical emitter.
//! No serializer, JSON reconstruction, semantic predicates, or owning task tree.
use super::{
    schema_build::{self, View},
    schema_fields::Field,
    schema_kernel::SchemaKind,
};
use crate::{
    affordance::{ActionAffordance, EventAffordance, PropertyAffordance},
    context::{Context, ContextEntry},
    data_schema::{DataSchema, DataSchemaContext},
    data_type::{
        AbsoluteUri, AdditionalExpectedResponse, BaseUri, ExpectedResponse, Metadata, Operation,
        VersionInfo,
    },
    form::Form,
    link::Link,
    security_scheme::{Qop, SecurityLocation, SecurityScheme, SecuritySchemeContext},
    thing::Thing,
};
use alloc::{
    collections::{BTreeMap, btree_map},
    string::String,
};
use serde_json::Value;

pub const ROOT: u32 = 32;
pub const METADATA: u32 = 33;
pub const PROPERTY: u32 = 34;
pub const ACTION: u32 = 35;
pub const EVENT: u32 = 36;
pub const FORM: u32 = 37;
pub const LINK: u32 = 38;
pub const DATE: u32 = 39;
pub const VERSION: u32 = 40;
pub const RESPONSE: u32 = 41;
pub const ADDITIONAL: u32 = 42;
pub const SECURITY: u32 = 43;
pub const SECURITY_CONTEXT: u32 = 44;
pub const SECURITY_VARIANT: u32 = 48;
pub const URI: u32 = 60;
pub const TEMPLATE: u32 = 61;
pub const BASE: u32 = 62;
pub const BASE_TEMPLATE: u32 = 63;
pub const OPERATION: u32 = 8;

/// Slot names describe typed fields, not wire decoding/default policy.
pub const ROOT_FIELDS: [&str; 19] = [
    "context",
    "id",
    "metadata",
    "version",
    "created",
    "modified",
    "support",
    "base",
    "properties",
    "actions",
    "events",
    "links",
    "forms",
    "security",
    "securityDefinitions",
    "profile",
    "schemaDefinitions",
    "uriVariables",
    "extras",
];
pub const FORM_FIELDS: [&str; 10] = [
    "href",
    "contentType",
    "contentCoding",
    "security",
    "scopes",
    "response",
    "additionalResponses",
    "subprotocol",
    "op",
    "extras",
];

#[derive(Clone, Copy)]
pub(crate) enum Task<'a> {
    Thing(&'a Thing),
    Metadata(&'a Metadata),
    Schema(&'a DataSchema),
    Property(&'a PropertyAffordance),
    Action(&'a ActionAffordance),
    Event(&'a EventAffordance),
    Form(&'a Form),
    Link(&'a Link),
    Date(&'a time::OffsetDateTime),
    Version(&'a VersionInfo),
    Response(&'a ExpectedResponse),
    Additional(&'a AdditionalExpectedResponse),
    Security(&'a SecurityScheme),
    SecurityContext(&'a SecuritySchemeContext),
    SecurityVariant(&'a SecurityScheme),
    Context(&'a Context),
    Strings(&'a [String]),
    Schemas(&'a [DataSchema]),
    Values(&'a [Value]),
    Forms(&'a [Form]),
    Links(&'a [Link]),
    Additionals(&'a [AdditionalExpectedResponse]),
    Uris(&'a [AbsoluteUri]),
    Operations(&'a [Operation]),
    Map(Map<'a>),
    Entry(&'a str, Atom<'a>),
    Value(&'a Value),
    Text(&'a str, u32),
    Scalar(u32, u64),
}
#[derive(Clone, Copy)]
pub(crate) enum Map<'a> {
    Json(&'a serde_json::Map<String, Value>),
    Values(&'a BTreeMap<String, Value>),
    Strings(&'a BTreeMap<String, String>),
    Schemas(&'a BTreeMap<String, DataSchema>),
    Properties(&'a BTreeMap<String, PropertyAffordance>),
    Actions(&'a BTreeMap<String, ActionAffordance>),
    Events(&'a BTreeMap<String, EventAffordance>),
    Security(&'a BTreeMap<String, SecurityScheme>),
}
#[derive(Clone, Copy)]
pub(crate) enum Atom<'a> {
    Value(&'a Value),
    Text(&'a str),
    Schema(&'a DataSchema),
    Property(&'a PropertyAffordance),
    Action(&'a ActionAffordance),
    Event(&'a EventAffordance),
    Security(&'a SecurityScheme),
}
impl<'a> Atom<'a> {
    fn task(self) -> Task<'a> {
        match self {
            Self::Value(v) => Task::Value(v),
            Self::Text(v) => text(v),
            Self::Schema(v) => Task::Schema(v),
            Self::Property(v) => Task::Property(v),
            Self::Action(v) => Task::Action(v),
            Self::Event(v) => Task::Event(v),
            Self::Security(v) => Task::Security(v),
        }
    }
}
pub(crate) enum Iter<'a> {
    Values(btree_map::Iter<'a, String, Value>),
    Strings(btree_map::Iter<'a, String, String>),
    Schemas(btree_map::Iter<'a, String, DataSchema>),
    Properties(btree_map::Iter<'a, String, PropertyAffordance>),
    Actions(btree_map::Iter<'a, String, ActionAffordance>),
    Events(btree_map::Iter<'a, String, EventAffordance>),
    Security(btree_map::Iter<'a, String, SecurityScheme>),
}
pub(crate) enum Children<'a> {
    Slots(Task<'a>),
    Iter(Iter<'a>),
    Json {
        map: &'a serde_json::Map<String, Value>,
        previous: Option<&'a str>,
    },
}
pub(crate) struct Description<'a> {
    pub kind: u32,
    pub count: usize,
    pub bits: Option<u64>,
    pub text: Option<&'a str>,
    pub children: Children<'a>,
}
use validated_thing_value_construction_probe::Kind as K;
fn text(value: &str) -> Task<'_> {
    Task::Text(value, K::String as u32)
}
fn optional_text(value: Option<&str>) -> Option<Task<'_>> {
    value.map(text)
}
fn boolean(value: bool) -> Task<'static> {
    Task::Scalar(if value { K::True } else { K::False } as u32, 0)
}
fn unsigned(value: u32) -> Task<'static> {
    Task::Scalar(8, value as u64)
}
fn integer(value: i64) -> Task<'static> {
    Task::Scalar(10, value as u64)
}
fn uri(value: &str) -> Task<'_> {
    Task::Text(value, URI)
}

pub(crate) fn describe(task: Task<'_>) -> Description<'_> {
    let mut description = Description {
        kind: 0,
        count: 0,
        bits: None,
        text: None,
        children: Children::Slots(task),
    };
    description.kind = match task {
        Task::Thing(_) => {
            description.count = ROOT_FIELDS.len();
            ROOT
        }
        Task::Metadata(_) => {
            description.count = 5;
            METADATA
        }
        Task::Schema(v) => {
            description.count = Field::ALL.len() + 1;
            16 + schema_kind(v) as u32
        }
        Task::Property(_) => {
            description.count = 4;
            PROPERTY
        }
        Task::Action(_) => {
            description.count = 9;
            ACTION
        }
        Task::Event(_) => {
            description.count = 8;
            EVENT
        }
        Task::Form(_) => {
            description.count = FORM_FIELDS.len();
            FORM
        }
        Task::Link(_) => {
            description.count = 7;
            LINK
        }
        Task::Date(_) => {
            description.count = 8;
            DATE
        }
        Task::Version(_) => {
            description.count = 3;
            VERSION
        }
        Task::Response(_) => {
            description.count = 2;
            RESPONSE
        }
        Task::Additional(_) => {
            description.count = 4;
            ADDITIONAL
        }
        Task::Security(_) => {
            description.count = 2;
            SECURITY
        }
        Task::SecurityContext(_) => {
            description.count = 6;
            SECURITY_CONTEXT
        }
        Task::SecurityVariant(v) => {
            let (index, count) = match v {
                SecurityScheme::NoSec(_) => (0, 0),
                SecurityScheme::Auto(_) => (1, 0),
                SecurityScheme::Combo(_) => (2, 2),
                SecurityScheme::Basic(_) => (3, 2),
                SecurityScheme::Digest(_) => (4, 3),
                SecurityScheme::APIKey(_) => (5, 2),
                SecurityScheme::Bearer(_) => (6, 5),
                SecurityScheme::PSK(_) => (7, 1),
                SecurityScheme::OAuth2(_) => (8, 5),
            };
            description.count = count;
            SECURITY_VARIANT + index
        }
        Task::Context(v) => {
            description.count = v.entries_for_construction().len();
            K::Array as u32
        }
        Task::Strings(v) => {
            description.count = v.len();
            K::Array as u32
        }
        Task::Schemas(v) => {
            description.count = v.len();
            K::Array as u32
        }
        Task::Values(v) => {
            description.count = v.len();
            K::Array as u32
        }
        Task::Forms(v) => {
            description.count = v.len();
            K::Array as u32
        }
        Task::Links(v) => {
            description.count = v.len();
            K::Array as u32
        }
        Task::Additionals(v) => {
            description.count = v.len();
            K::Array as u32
        }
        Task::Uris(v) => {
            description.count = v.len();
            K::Array as u32
        }
        Task::Operations(v) => {
            description.count = v.len();
            K::Array as u32
        }
        Task::Entry(_, _) => {
            description.count = 2;
            K::Entry as u32
        }
        Task::Map(map) => {
            let (count, children) = match map {
                Map::Json(v) => (
                    v.len(),
                    Children::Json {
                        map: v,
                        previous: None,
                    },
                ),
                Map::Values(v) => (v.len(), Children::Iter(Iter::Values(v.iter()))),
                Map::Strings(v) => (v.len(), Children::Iter(Iter::Strings(v.iter()))),
                Map::Schemas(v) => (v.len(), Children::Iter(Iter::Schemas(v.iter()))),
                Map::Properties(v) => (v.len(), Children::Iter(Iter::Properties(v.iter()))),
                Map::Actions(v) => (v.len(), Children::Iter(Iter::Actions(v.iter()))),
                Map::Events(v) => (v.len(), Children::Iter(Iter::Events(v.iter()))),
                Map::Security(v) => (v.len(), Children::Iter(Iter::Security(v.iter()))),
            };
            description.count = count;
            description.children = children;
            K::Object as u32
        }
        Task::Value(v) => {
            return match v {
                Value::Null => describe(Task::Scalar(K::Null as u32, 0)),
                Value::Bool(v) => describe(boolean(*v)),
                Value::String(v) => describe(text(v)),
                Value::Number(v) => describe(Task::Text(v.as_str(), K::Number as u32)),
                Value::Array(v) => describe(Task::Values(v)),
                Value::Object(v) => describe(Task::Map(Map::Json(v))),
            };
        }
        Task::Text(text, kind) => {
            description.text = Some(text);
            kind
        }
        Task::Scalar(kind, bits) => {
            description.bits = Some(bits);
            kind
        }
    };
    description
}
impl<'a> Children<'a> {
    pub(crate) fn next(&mut self, index: usize) -> Option<Task<'a>> {
        match self {
            Self::Slots(task) => child(*task, index),
            Self::Iter(iter) => Some(match iter {
                Iter::Values(v) => {
                    let (k, v) = v.next().unwrap();
                    Task::Entry(k, Atom::Value(v))
                }
                Iter::Strings(v) => {
                    let (k, v) = v.next().unwrap();
                    Task::Entry(k, Atom::Text(v))
                }
                Iter::Schemas(v) => {
                    let (k, v) = v.next().unwrap();
                    Task::Entry(k, Atom::Schema(v))
                }
                Iter::Properties(v) => {
                    let (k, v) = v.next().unwrap();
                    Task::Entry(k, Atom::Property(v))
                }
                Iter::Actions(v) => {
                    let (k, v) = v.next().unwrap();
                    Task::Entry(k, Atom::Action(v))
                }
                Iter::Events(v) => {
                    let (k, v) = v.next().unwrap();
                    Task::Entry(k, Atom::Event(v))
                }
                Iter::Security(v) => {
                    let (k, v) = v.next().unwrap();
                    Task::Entry(k, Atom::Security(v))
                }
            }),
            Self::Json { .. } => unreachable!("canonical emitter services charged selection"),
        }
    }
}
fn child(task: Task<'_>, index: usize) -> Option<Task<'_>> {
    Some(match task {
        Task::Thing(v) => match index {
            0 => Task::Context(&v.context),
            1 => return v.id.as_ref().map(|v| uri(v.as_str())),
            2 => Task::Metadata(&v._metadata),
            3 => return v.version.as_ref().map(Task::Version),
            4 => return v.created.as_ref().map(Task::Date),
            5 => return v.modified.as_ref().map(Task::Date),
            6 => return v.support.as_ref().map(|v| uri(v.as_str())),
            7 => {
                return v.base.as_ref().map(|v| match v {
                    BaseUri::Absolute(v) => Task::Text(v.as_str(), BASE),
                    BaseUri::Template(v) => Task::Text(v, BASE_TEMPLATE),
                });
            }
            8 => return v.properties.as_ref().map(|v| Task::Map(Map::Properties(v))),
            9 => return v.actions.as_ref().map(|v| Task::Map(Map::Actions(v))),
            10 => return v.events.as_ref().map(|v| Task::Map(Map::Events(v))),
            11 => return v.links.as_deref().map(Task::Links),
            12 => return v.forms.as_deref().map(Task::Forms),
            13 => Task::Strings(&v.security),
            14 => Task::Map(Map::Security(&v.security_definitions)),
            15 => return v.profile.as_deref().map(Task::Uris),
            16 => {
                return v
                    .schema_definitions
                    .as_ref()
                    .map(|v| Task::Map(Map::Schemas(v)));
            }
            17 => return v.uri_variables.as_ref().map(|v| Task::Map(Map::Schemas(v))),
            18 => Task::Map(Map::Values(&v._extra_fields)),
            _ => unreachable!(),
        },
        Task::Metadata(v) => match index {
            0 => return v.tags.as_deref().map(Task::Strings),
            1 => return optional_text(v.title.as_deref()),
            2 => {
                return v
                    .titles
                    .as_ref()
                    .map(|v| Task::Map(Map::Strings(v.as_map())));
            }
            3 => return optional_text(v.description.as_deref()),
            4 => {
                return v
                    .descriptions
                    .as_ref()
                    .map(|v| Task::Map(Map::Strings(v.as_map())));
            }
            _ => unreachable!(),
        },
        Task::Schema(v) => return schema_child(v, index),
        Task::Property(v) => match index {
            0 => Task::Schema(&v._schema),
            1 => Task::Forms(&v._interaction.forms),
            2 => {
                return v
                    ._interaction
                    .uri_variables
                    .as_ref()
                    .map(|v| Task::Map(Map::Schemas(v)));
            }
            3 => boolean(v.observable),
            _ => unreachable!(),
        },
        Task::Action(v) => match index {
            0 => Task::Metadata(&v._metadata),
            1 => Task::Forms(&v._interaction.forms),
            2 => {
                return v
                    ._interaction
                    .uri_variables
                    .as_ref()
                    .map(|v| Task::Map(Map::Schemas(v)));
            }
            3 => return v.input.as_ref().map(Task::Schema),
            4 => return v.output.as_ref().map(Task::Schema),
            5 => boolean(v.safe),
            6 => boolean(v.idempotent),
            7 => {
                #[cfg(feature = "td2-preview")]
                {
                    return v.synchronous.map(boolean);
                }
                #[cfg(not(feature = "td2-preview"))]
                {
                    return None;
                }
            }
            8 => Task::Map(Map::Values(&v._extra_fields)),
            _ => unreachable!(),
        },
        Task::Event(v) => match index {
            0 => Task::Metadata(&v._metadata),
            1 => Task::Forms(&v._interaction.forms),
            2 => {
                return v
                    ._interaction
                    .uri_variables
                    .as_ref()
                    .map(|v| Task::Map(Map::Schemas(v)));
            }
            3 => return v.subscription.as_ref().map(Task::Schema),
            4 => return v.data.as_ref().map(Task::Schema),
            5 => return v.data_response.as_ref().map(Task::Schema),
            6 => return v.cancellation.as_ref().map(Task::Schema),
            7 => Task::Map(Map::Values(&v._extra_fields)),
            _ => unreachable!(),
        },
        Task::Form(v) => match index {
            0 => match &v.href {
                crate::data_type::FormHref::Reference(v) => uri(v.as_str()),
                crate::data_type::FormHref::Template(v) => Task::Text(v, TEMPLATE),
            },
            1 => text(&v.content_type),
            2 => return optional_text(v.content_coding.as_deref()),
            3 => return v.security.as_deref().map(Task::Strings),
            4 => return v.scopes.as_deref().map(Task::Strings),
            5 => return v.response.as_ref().map(Task::Response),
            6 => return v.additional_responses.as_deref().map(Task::Additionals),
            7 => return optional_text(v.subprotocol.as_deref()),
            8 => return v.op.as_deref().map(Task::Operations),
            9 => Task::Map(Map::Values(&v._extra_fields)),
            _ => unreachable!(),
        },
        Task::Link(v) => match index {
            0 => uri(v.href.as_str()),
            1 => return optional_text(v.content_type.as_deref()),
            2 => return optional_text(v.rel.as_deref()),
            3 => return v.anchor.as_ref().map(|v| uri(v.as_str())),
            4 => return optional_text(v.sizes.as_deref()),
            5 => return v.hreflang.as_deref().map(Task::Strings),
            6 => Task::Map(Map::Values(&v._extra_fields)),
            _ => unreachable!(),
        },
        Task::Date(v) => match index {
            0 => integer(v.year() as i64),
            1 => unsigned(v.month() as u32),
            2 => unsigned(v.day() as u32),
            3 => unsigned(v.hour() as u32),
            4 => unsigned(v.minute() as u32),
            5 => unsigned(v.second() as u32),
            6 => unsigned(v.nanosecond()),
            7 => integer(v.offset().whole_seconds() as i64),
            _ => unreachable!(),
        },
        Task::Version(v) => match index {
            0 => text(&v.instance),
            1 => return optional_text(v.model.as_deref()),
            2 => Task::Map(Map::Values(&v._extra_fields)),
            _ => unreachable!(),
        },
        Task::Response(v) => match index {
            0 => text(&v.content_type),
            1 => Task::Map(Map::Values(&v._extra_fields)),
            _ => unreachable!(),
        },
        Task::Additional(v) => match index {
            0 => return optional_text(v.content_type.as_deref()),
            1 => return optional_text(v.schema.as_deref()),
            2 => boolean(v.success),
            3 => Task::Map(Map::Values(&v._extra_fields)),
            _ => unreachable!(),
        },
        Task::Security(v) => match index {
            0 => Task::SecurityContext(security_context(v)),
            1 => Task::SecurityVariant(v),
            _ => unreachable!(),
        },
        Task::SecurityContext(v) => match index {
            0 => return v.tags.as_deref().map(Task::Strings),
            1 => return optional_text(v.description.as_deref()),
            2 => {
                return v
                    .descriptions
                    .as_ref()
                    .map(|v| Task::Map(Map::Strings(v.as_map())));
            }
            3 => return v.proxy.as_ref().map(|v| uri(v.as_str())),
            4 => text(&v.scheme),
            5 => Task::Map(Map::Values(&v._extra_fields)),
            _ => unreachable!(),
        },
        Task::SecurityVariant(v) => return security_child(v, index),
        Task::Context(v) => match &v.entries_for_construction()[index] {
            ContextEntry::Uri(v) => uri(v.as_str()),
            ContextEntry::Object(v) => Task::Map(Map::Values(v)),
        },
        Task::Strings(v) => text(&v[index]),
        Task::Schemas(v) => Task::Schema(&v[index]),
        Task::Values(v) => Task::Value(&v[index]),
        Task::Forms(v) => Task::Form(&v[index]),
        Task::Links(v) => Task::Link(&v[index]),
        Task::Additionals(v) => Task::Additional(&v[index]),
        Task::Uris(v) => uri(v[index].as_str()),
        Task::Operations(v) => Task::Scalar(OPERATION, operation_index(v[index]) as u64),
        Task::Entry(k, v) => {
            if index == 0 {
                text(k)
            } else {
                v.task()
            }
        }
        _ => unreachable!(),
    })
}
pub(crate) fn schema_kind(v: &DataSchema) -> SchemaKind {
    match v {
        DataSchema::Array(_) => SchemaKind::Array,
        DataSchema::Boolean(_) => SchemaKind::Boolean,
        DataSchema::Number(_) => SchemaKind::Number,
        DataSchema::Integer(_) => SchemaKind::Integer,
        DataSchema::Object(_) => SchemaKind::Object,
        DataSchema::String(_) => SchemaKind::String,
        DataSchema::Null(_) => SchemaKind::Null,
    }
}
fn schema_context(v: &DataSchema) -> &DataSchemaContext {
    match v {
        DataSchema::Array(v) => &v._context,
        DataSchema::Boolean(v) => &v._context,
        DataSchema::Number(v) => &v._context,
        DataSchema::Integer(v) => &v._context,
        DataSchema::Object(v) => &v._context,
        DataSchema::String(v) => &v._context,
        DataSchema::Null(v) => &v._context,
    }
}
fn schema_child(v: &DataSchema, index: usize) -> Option<Task<'_>> {
    let c = schema_context(v);
    let m = &c._metadata;
    if index == Field::ALL.len() {
        return Some(Task::Map(Map::Values(&c._extra_fields)));
    }
    match Field::ALL[index] {
        Field::Tags => m.tags.as_deref().map(Task::Strings),
        Field::Title => optional_text(m.title.as_deref()),
        Field::Titles => m
            .titles
            .as_ref()
            .map(|v| Task::Map(Map::Strings(v.as_map()))),
        Field::Description => optional_text(m.description.as_deref()),
        Field::Descriptions => m
            .descriptions
            .as_ref()
            .map(|v| Task::Map(Map::Strings(v.as_map()))),
        Field::Const => c.constant.as_ref().map(Task::Value),
        Field::Default => c.default.as_ref().map(Task::Value),
        Field::Unit => optional_text(c.unit.as_deref()),
        Field::OneOf => c.one_of.as_deref().map(Task::Schemas),
        Field::Enum => c.enumerate.as_deref().map(Task::Values),
        Field::ReadOnly => Some(boolean(c.read_only)),
        Field::WriteOnly => Some(boolean(c.write_only)),
        Field::Format => optional_text(c.format.as_deref()),
        Field::Type => optional_text(c.data_type.as_deref()),
        Field::Items => {
            if let DataSchema::Array(v) = v {
                v.items.as_deref().map(Task::Schemas)
            } else {
                None
            }
        }
        Field::MinItems | Field::MaxItems => {
            if let DataSchema::Array(v) = v {
                if index == Field::MinItems as usize {
                    v.min_items
                } else {
                    v.max_items
                }
                .map(unsigned)
            } else {
                None
            }
        }
        Field::Properties => {
            if let DataSchema::Object(v) = v {
                v.properties.as_ref().map(|v| Task::Map(Map::Schemas(v)))
            } else {
                None
            }
        }
        Field::Required => {
            if let DataSchema::Object(v) = v {
                v.required.as_deref().map(Task::Strings)
            } else {
                None
            }
        }
        Field::MinLength | Field::MaxLength => {
            if let DataSchema::String(v) = v {
                if index == Field::MinLength as usize {
                    v.min_length
                } else {
                    v.max_length
                }
                .map(unsigned)
            } else {
                None
            }
        }
        Field::Pattern | Field::ContentEncoding | Field::ContentMediaType => {
            if let DataSchema::String(v) = v {
                optional_text(match Field::ALL[index] {
                    Field::Pattern => v.pattern.as_deref(),
                    Field::ContentEncoding => v.content_encoding.as_deref(),
                    _ => v.content_media_type.as_deref(),
                })
            } else {
                None
            }
        }
        Field::Minimum
        | Field::ExclusiveMinimum
        | Field::Maximum
        | Field::ExclusiveMaximum
        | Field::MultipleOf => {
            let i = index - Field::Minimum as usize;
            match v {
                DataSchema::Number(v) => [
                    v.minimum,
                    v.exclusive_minimum,
                    v.maximum,
                    v.exclusive_maximum,
                    v.multiple_of,
                ][i]
                    .map(|v| Task::Scalar(9, v.to_bits())),
                DataSchema::Integer(v) => [
                    v.minimum,
                    v.exclusive_minimum,
                    v.maximum,
                    v.exclusive_maximum,
                    v.multiple_of,
                ][i]
                    .map(integer),
                _ => None,
            }
        }
    }
}
fn security_context(v: &SecurityScheme) -> &SecuritySchemeContext {
    match v {
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
fn location(v: &SecurityLocation) -> Task<'_> {
    text(match v {
        SecurityLocation::Header => "header",
        SecurityLocation::Query => "query",
        SecurityLocation::Body => "body",
        SecurityLocation::Cookie => "cookie",
        SecurityLocation::Auto => "auto",
        SecurityLocation::Uri => "uri",
    })
}
fn security_child(v: &SecurityScheme, index: usize) -> Option<Task<'_>> {
    Some(match v {
        SecurityScheme::Combo(v) => Task::Strings(if index == 0 { &v.one_of } else { &v.all_of }),
        SecurityScheme::Basic(v) => match index {
            0 => return optional_text(v.name.as_deref()),
            1 => location(&v.location),
            _ => unreachable!(),
        },
        SecurityScheme::Digest(v) => match index {
            0 => return optional_text(v.name.as_deref()),
            1 => location(&v.location),
            2 => text(match v.qop {
                Qop::Auth => "auth",
                Qop::AuthInt => "auth-int",
            }),
            _ => unreachable!(),
        },
        SecurityScheme::APIKey(v) => match index {
            0 => return optional_text(v.name.as_deref()),
            1 => location(&v.location),
            _ => unreachable!(),
        },
        SecurityScheme::Bearer(v) => match index {
            0 => return v.authorization.as_ref().map(|v| uri(v.as_str())),
            1 => return optional_text(v.name.as_deref()),
            2 => text(&v.alg),
            3 => text(&v.format),
            4 => location(&v.location),
            _ => unreachable!(),
        },
        SecurityScheme::PSK(v) => return optional_text(v.identity.as_deref()),
        SecurityScheme::OAuth2(v) => match index {
            0 => return v.authorization.as_ref().map(|v| uri(v.as_str())),
            1 => return v.token.as_ref().map(|v| uri(v.as_str())),
            2 => return v.refresh.as_ref().map(|v| uri(v.as_str())),
            3 => return v.scopes.as_deref().map(Task::Strings),
            4 => text(&v.flow),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    })
}
pub const OPERATIONS: &[Operation] = &[
    Operation::ReadProperty,
    Operation::WriteProperty,
    Operation::ObserveProperty,
    Operation::UnobserveProperty,
    Operation::InvokeAction,
    Operation::QueryAction,
    Operation::CancelAction,
    Operation::SubscribeEvent,
    Operation::UnsubscribeEvent,
    Operation::ReadAllProperties,
    Operation::WriteAllProperties,
    Operation::ReadMultipleProperties,
    Operation::WriteMultipleProperties,
    Operation::ObserveAllProperties,
    Operation::UnobserveAllProperties,
    Operation::QueryAllActions,
    Operation::SubscribeAllEvents,
    Operation::UnsubscribeAllEvents,
];
fn operation_index(v: Operation) -> usize {
    OPERATIONS.iter().position(|&op| op == v).unwrap()
}

/// Owned normalized typed Thing, deliberately not a ValidatedThing admission.
/// from_thing_basic includes paid Basic; URI caching remains external evidence.
pub struct NormalizedThing {
    pub(crate) owner: validated_thing_value_construction_probe::OwnedValue,
}
impl NormalizedThing {
    pub(crate) fn new(owner: validated_thing_value_construction_probe::OwnedValue) -> Self {
        Self { owner }
    }
    pub fn view(&self) -> View<'_> {
        View::root(self.owner.sealed_arenas())
    }
    pub fn footprint(&self) -> validated_thing_value_construction_probe::Footprint {
        self.owner.footprint()
    }
    pub fn lifetime_remaining(&self) -> u64 {
        self.owner.lifetime_remaining()
    }
    pub fn arenas_for_fixture(&self) -> validated_thing_value_construction_probe::Sealed<'_> {
        self.owner.sealed_arenas()
    }
}

/// Uses the same build/equivalence/grow/reseal transaction as strict DataSchema.
#[allow(clippy::result_large_err)]
pub fn from_thing(
    input: &Thing,
    limits: validated_thing_value_construction_probe::Limits,
    control: impl FnMut(schema_build::Stage) -> (clinkz_wot_foundation::WorkBudget, bool),
) -> Result<(NormalizedThing, schema_build::ConstructionTrace), schema_build::ConstructionFailure> {
    schema_build::from_typed_thing(input, limits, control)
}

/// The same complete construction transaction, including paid whole-Thing
/// Basic on its sealed result and first-cause rollback of the entire owner.
#[allow(clippy::result_large_err)]
pub fn from_thing_basic(
    input: &Thing,
    limits: validated_thing_value_construction_probe::Limits,
    control: impl FnMut(schema_build::Stage) -> (clinkz_wot_foundation::WorkBudget, bool),
) -> Result<(NormalizedThing, schema_build::ConstructionTrace), schema_build::ConstructionFailure> {
    schema_build::from_typed_thing_basic(input, limits, control)
}

/// Borrowed canonical facts for the already shared whole-Thing Basic kernel.
/// This synchronous adapter is a query/parity oracle, not charged admission.
pub struct Basic<'a> {
    root: View<'a>,
    schemas: schema_build::Access,
}
impl<'a> Basic<'a> {
    pub fn new(thing: &'a NormalizedThing) -> Self {
        Self {
            root: thing.view(),
            schemas: schema_build::Access,
        }
    }
    fn map(&self, kind: super::basic_kernel::OwnerKind) -> Option<View<'a>> {
        use super::basic_kernel::OwnerKind::*;
        self.root.child(match kind {
            Property => 8,
            Action => 9,
            Event => 10,
            _ => unreachable!(),
        })
    }
}
pub fn map_get<'a>(map: View<'a>, key: &str) -> Option<View<'a>> {
    (0..map.len()).find_map(|i| {
        let (name, value) = map.member(i)?;
        (name == key).then_some(value)
    })
}
#[derive(Clone, Copy)]
pub struct Names<'a>(Option<View<'a>>);
impl<'a> Names<'a> {
    fn count(self) -> usize {
        let Some(v) = self.0 else {
            return 0;
        };
        match v.literal_kind() {
            Some(K::String) => 1,
            Some(K::Array) => (0..v.len())
                .filter(|&i| v.child(i).unwrap().literal_kind() == Some(K::String))
                .count(),
            _ => 0,
        }
    }
    fn at(self, index: usize) -> &'a str {
        let v = self.0.unwrap();
        if v.literal_kind() == Some(K::String) {
            assert_eq!(index, 0);
            v.text().unwrap()
        } else {
            (0..v.len())
                .filter_map(|i| {
                    let v = v.child(i).unwrap();
                    (v.literal_kind() == Some(K::String)).then(|| v.text().unwrap())
                })
                .nth(index)
                .unwrap()
        }
    }
}
impl<'a> super::basic_kernel::BasicAccess<'a> for Basic<'a> {
    type Schemas = schema_build::Access;
    type SchemaMap = View<'a>;
    type Affordance = View<'a>;
    type Forms = View<'a>;
    type Form = View<'a>;
    type Operations = View<'a>;
    type Names = Names<'a>;
    type Definition = View<'a>;
    fn schemas(&self) -> &Self::Schemas {
        &self.schemas
    }
    fn title(&self) -> Option<&'a str> {
        self.root.child(2)?.child(1)?.text()
    }
    fn root_security(&self) -> Names<'a> {
        Names(self.root.child(13))
    }
    fn names_count(&self, names: Names<'a>) -> usize {
        names.count()
    }
    fn name_at(&self, names: Names<'a>, index: usize) -> &'a str {
        names.at(index)
    }
    fn definition_count(&self) -> usize {
        self.root.child(14).unwrap().len()
    }
    fn definition_at(&self, index: usize) -> View<'a> {
        self.root.child(14).unwrap().member(index).unwrap().1
    }
    fn definition_name(&self, index: usize) -> &'a str {
        self.root.child(14).unwrap().member(index).unwrap().0
    }
    fn definition_exists(&self, name: &str) -> bool {
        map_get(self.root.child(14).unwrap(), name).is_some()
    }
    fn scheme(&self, definition: View<'a>) -> &'a str {
        definition
            .child(0)
            .unwrap()
            .child(4)
            .unwrap()
            .text()
            .unwrap()
    }
    fn combo_names(&self, definition: View<'a>, field: super::basic_kernel::Field) -> Names<'a> {
        let variant = definition.child(1).unwrap();
        if variant.kind_for_fixture() == SECURITY_VARIANT + 2 {
            Names(variant.child(usize::from(field == super::basic_kernel::Field::AllOf)))
        } else {
            Names(map_get(
                definition.child(0).unwrap().child(5).unwrap(),
                field.name(),
            ))
        }
    }
    fn security_string(
        &self,
        definition: View<'a>,
        field: super::basic_kernel::Field,
    ) -> Option<&'a str> {
        use super::basic_kernel::Field::*;
        let variant = definition.child(1).unwrap();
        match (variant.kind_for_fixture(), field) {
            (kind, Name) if kind == SECURITY_VARIANT + 5 => variant.child(0)?.text(),
            (kind, Flow) if kind == SECURITY_VARIANT + 8 => variant.child(4)?.text(),
            _ => {
                let v = map_get(definition.child(0)?.child(5)?, field.name())?;
                (v.literal_kind() == Some(K::String)).then(|| v.text().unwrap())
            }
        }
    }
    fn endpoint_present(&self, definition: View<'a>, field: super::basic_kernel::Field) -> bool {
        let variant = definition.child(1).unwrap();
        if variant.kind_for_fixture() == SECURITY_VARIANT + 8 {
            variant
                .child(usize::from(field == super::basic_kernel::Field::Token))
                .is_some()
        } else {
            self.security_string(definition, field)
                .is_some_and(|v| !v.is_empty())
        }
    }
    fn root_schema_map(&self, field: super::basic_kernel::Field) -> Option<View<'a>> {
        self.root
            .child(if field == super::basic_kernel::Field::SchemaDefinitions {
                16
            } else {
                17
            })
    }
    fn schema_count(&self, map: View<'a>) -> usize {
        map.len()
    }
    fn schema_at(&self, map: View<'a>, index: usize) -> View<'a> {
        map.member(index).unwrap().1
    }
    fn schema_name(&self, map: View<'a>, index: usize) -> &'a str {
        map.member(index).unwrap().0
    }
    fn affordance_count(&self, kind: super::basic_kernel::OwnerKind) -> usize {
        self.map(kind).map_or(0, View::len)
    }
    fn affordance_at(&self, kind: super::basic_kernel::OwnerKind, index: usize) -> View<'a> {
        self.map(kind).unwrap().member(index).unwrap().1
    }
    fn affordance_name(&self, owner: super::basic_kernel::Owner) -> &'a str {
        self.map(owner.kind)
            .unwrap()
            .member(owner.ordinal)
            .unwrap()
            .0
    }
    fn uri_variables(&self, affordance: View<'a>) -> Option<View<'a>> {
        affordance.child(2)
    }
    fn affordance_schema(
        &self,
        affordance: View<'a>,
        field: super::basic_kernel::Field,
    ) -> Option<View<'a>> {
        use super::basic_kernel::Field::*;
        affordance.child(match field {
            PropertySchema => 0,
            Input | Subscription => 3,
            Output | Data => 4,
            DataResponse => 5,
            Cancellation => 6,
            _ => unreachable!(),
        })
    }
    fn forms(&self, affordance: Option<View<'a>>) -> Option<View<'a>> {
        affordance.map_or_else(|| self.root.child(12), |v| v.child(1))
    }
    fn form_count(&self, forms: View<'a>) -> usize {
        forms.len()
    }
    fn form_at(&self, forms: View<'a>, index: usize) -> View<'a> {
        forms.child(index).unwrap()
    }
    fn operations(&self, form: View<'a>) -> Option<View<'a>> {
        form.child(8)
    }
    fn operation_count(&self, ops: View<'a>) -> usize {
        ops.len()
    }
    fn operation_at(&self, ops: View<'a>, index: usize) -> Operation {
        OPERATIONS[ops.child(index).unwrap().unsigned().unwrap() as usize]
    }
    fn form_security(&self, form: View<'a>) -> Option<Names<'a>> {
        form.child(3).map(|v| Names(Some(v)))
    }
}

impl<'a> View<'a> {
    pub fn root_field(self, name: &str) -> Option<Self> {
        assert_eq!(self.kind_for_fixture(), ROOT);
        self.child(ROOT_FIELDS.iter().position(|&v| v == name)?)
    }
    pub fn form_field(self, name: &str) -> Option<Self> {
        assert_eq!(self.kind_for_fixture(), FORM);
        self.child(FORM_FIELDS.iter().position(|&v| v == name)?)
    }
    pub fn effective_property_operations(self, property: Self) -> &'static [Operation] {
        let schema = property.child(0).unwrap();
        super::basic_kernel::default_property_operations((
            schema.field(Field::ReadOnly).unwrap().literal_kind() == Some(K::True),
            schema.field(Field::WriteOnly).unwrap().literal_kind() == Some(K::True),
        ))
    }
    pub fn effective_security(self, form: Self) -> Names<'a> {
        Names(Some(
            super::basic_kernel::inherited_security(
                self.root_field("security").unwrap(),
                form.form_field("security"),
            )
            .0,
        ))
    }
}
