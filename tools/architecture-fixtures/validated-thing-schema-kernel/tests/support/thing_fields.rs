//! Independent public-field readback over the fixed typed corpus. These
//! assertions do not consume the emitter's Task, Children, or field mapping.
use super::{
    canonical_fields::{self as schema, languages, literal, optional, strings},
    td_crate,
};
use td_crate::{
    data_type::{AdditionalExpectedResponse, Metadata},
    form::Form,
    schema_build::View,
    security_scheme::{Qop, SecurityScheme, SecuritySchemeContext},
    thing::Thing,
};
use validated_thing_value_construction_probe::Kind;

fn text(view: View<'_>, expected: &str) {
    assert_eq!(view.text(), Some(expected));
}
fn boolean(view: View<'_>, expected: bool) {
    assert_eq!(
        view.literal_kind(),
        Some(if expected { Kind::True } else { Kind::False })
    );
}
fn extras(view: View<'_>, source: &td_crate::data_type::ExtensionMap) {
    assert_eq!(view.len(), source.len());
    for (i, (key, value)) in source.iter().enumerate() {
        let (name, v) = view.member(i).unwrap();
        assert_eq!(name, key);
        literal(v, value);
    }
}
fn metadata(view: View<'_>, v: &Metadata) {
    optional(view.child(0), v.tags.as_deref(), strings);
    optional(view.child(1), v.title.as_deref(), text);
    optional(view.child(2), v.titles.as_ref(), languages);
    optional(view.child(3), v.description.as_deref(), text);
    optional(view.child(4), v.descriptions.as_ref(), languages);
}
fn schema_map(
    view: View<'_>,
    v: &std::collections::BTreeMap<String, td_crate::data_schema::DataSchema>,
) {
    assert_eq!(view.len(), v.len());
    for (i, (key, value)) in v.iter().enumerate() {
        let (name, child) = view.member(i).unwrap();
        assert_eq!(name, key);
        schema::fields(child, value);
    }
}
fn additional(view: View<'_>, v: &AdditionalExpectedResponse) {
    optional(view.child(0), v.content_type.as_deref(), text);
    optional(view.child(1), v.schema.as_deref(), text);
    boolean(view.child(2).unwrap(), v.success);
    extras(view.child(3).unwrap(), &v._extra_fields);
}
fn form(view: View<'_>, v: &Form) {
    text(view.child(0).unwrap(), v.href.as_str());
    assert_eq!(
        view.child(0).unwrap().kind_for_fixture(),
        if matches!(v.href, td_crate::data_type::FormHref::Template(_)) {
            61
        } else {
            60
        }
    );
    text(view.child(1).unwrap(), &v.content_type);
    optional(view.child(2), v.content_coding.as_deref(), text);
    optional(view.child(3), v.security.as_deref(), strings);
    optional(view.child(4), v.scopes.as_deref(), strings);
    optional(view.child(5), v.response.as_ref(), |view, v| {
        text(view.child(0).unwrap(), &v.content_type);
        extras(view.child(1).unwrap(), &v._extra_fields);
    });
    optional(
        view.child(6),
        v.additional_responses.as_deref(),
        |view, values| {
            assert_eq!(view.len(), values.len());
            for (i, v) in values.iter().enumerate() {
                additional(view.child(i).unwrap(), v);
            }
        },
    );
    optional(view.child(7), v.subprotocol.as_deref(), text);
    optional(view.child(8), v.op.as_deref(), |view, values| {
        assert_eq!(view.len(), values.len());
        for (i, v) in values.iter().enumerate() {
            assert_eq!(view.child(i).unwrap().unsigned(), Some(*v as u32));
        }
    });
    extras(view.child(9).unwrap(), &v._extra_fields);
}
fn forms(view: View<'_>, v: &[Form]) {
    assert_eq!(view.len(), v.len());
    for (i, v) in v.iter().enumerate() {
        assert_eq!(view.original_index(i), Some(i as u32));
        form(view.child(i).unwrap(), v);
    }
}
fn date(view: View<'_>, v: &time::OffsetDateTime) {
    assert_eq!(view.child(0).unwrap().integer(), Some(v.year() as i64));
    for (i, value) in [
        v.month() as u32,
        v.day() as u32,
        v.hour() as u32,
        v.minute() as u32,
        v.second() as u32,
        v.nanosecond(),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(view.child(i + 1).unwrap().unsigned(), Some(value));
    }
    assert_eq!(
        view.child(7).unwrap().integer(),
        Some(v.offset().whole_seconds() as i64)
    );
}
fn security_context(view: View<'_>, v: &SecuritySchemeContext) {
    optional(view.child(0), v.tags.as_deref(), strings);
    optional(view.child(1), v.description.as_deref(), text);
    optional(view.child(2), v.descriptions.as_ref(), languages);
    optional(view.child(3), v.proxy.as_ref().map(|v| v.as_str()), text);
    text(view.child(4).unwrap(), &v.scheme);
    extras(view.child(5).unwrap(), &v._extra_fields);
}
fn security(view: View<'_>, v: &SecurityScheme) {
    let variant = view.child(1).unwrap();
    let (context, index) = match v {
        SecurityScheme::NoSec(v) => (&v._context, 0),
        SecurityScheme::Auto(v) => (&v._context, 1),
        SecurityScheme::Combo(v) => {
            strings(variant.child(0).unwrap(), &v.one_of);
            strings(variant.child(1).unwrap(), &v.all_of);
            (&v._context, 2)
        }
        SecurityScheme::Basic(v) => {
            optional(variant.child(0), v.name.as_deref(), text);
            text(
                variant.child(1).unwrap(),
                &format!("{:?}", v.location).to_lowercase(),
            );
            (&v._context, 3)
        }
        SecurityScheme::Digest(v) => {
            optional(variant.child(0), v.name.as_deref(), text);
            text(
                variant.child(1).unwrap(),
                &format!("{:?}", v.location).to_lowercase(),
            );
            text(
                variant.child(2).unwrap(),
                if v.qop == Qop::Auth {
                    "auth"
                } else {
                    "auth-int"
                },
            );
            (&v._context, 4)
        }
        SecurityScheme::APIKey(v) => {
            optional(variant.child(0), v.name.as_deref(), text);
            text(
                variant.child(1).unwrap(),
                &format!("{:?}", v.location).to_lowercase(),
            );
            (&v._context, 5)
        }
        SecurityScheme::Bearer(v) => {
            optional(
                variant.child(0),
                v.authorization.as_ref().map(|v| v.as_str()),
                text,
            );
            optional(variant.child(1), v.name.as_deref(), text);
            text(variant.child(2).unwrap(), &v.alg);
            text(variant.child(3).unwrap(), &v.format);
            text(
                variant.child(4).unwrap(),
                &format!("{:?}", v.location).to_lowercase(),
            );
            (&v._context, 6)
        }
        SecurityScheme::PSK(v) => {
            optional(variant.child(0), v.identity.as_deref(), text);
            (&v._context, 7)
        }
        SecurityScheme::OAuth2(v) => {
            optional(
                variant.child(0),
                v.authorization.as_ref().map(|v| v.as_str()),
                text,
            );
            optional(variant.child(1), v.token.as_ref().map(|v| v.as_str()), text);
            optional(
                variant.child(2),
                v.refresh.as_ref().map(|v| v.as_str()),
                text,
            );
            optional(variant.child(3), v.scopes.as_deref(), strings);
            text(variant.child(4).unwrap(), &v.flow);
            (&v._context, 8)
        }
    };
    assert_eq!(variant.kind_for_fixture(), 48 + index);
    security_context(view.child(0).unwrap(), context);
}
pub fn assert_thing(view: View<'_>, v: &Thing) {
    assert_eq!(view.kind_for_fixture(), 32);
    assert_eq!(view.len(), 19);
    // Independent fixed Context oracle, including the nonserializable input.
    let context = view.child(0).unwrap();
    if v.context.has_wot_context() {
        assert_eq!(context.len(), 3);
        text(
            context.child(0).unwrap(),
            "https://www.w3.org/2022/wot/td/v1.1",
        );
        let (name, value) = context.child(1).unwrap().member(0).unwrap();
        assert_eq!(name, "ex");
        text(value, "https://example.org/ns#");
        text(
            context.child(2).unwrap(),
            "https://example.org/extra-context",
        );
    } else {
        assert_eq!(context.len(), 1);
        text(
            context.child(0).unwrap(),
            "https://example.org/extension-only",
        );
    }
    optional(view.child(1), v.id.as_ref().map(|v| v.as_str()), text);
    metadata(view.child(2).unwrap(), &v._metadata);
    optional(view.child(3), v.version.as_ref(), |view, v| {
        text(view.child(0).unwrap(), &v.instance);
        optional(view.child(1), v.model.as_deref(), text);
        extras(view.child(2).unwrap(), &v._extra_fields);
    });
    optional(view.child(4), v.created.as_ref(), date);
    optional(view.child(5), v.modified.as_ref(), date);
    optional(view.child(6), v.support.as_ref().map(|v| v.as_str()), text);
    optional(view.child(7), v.base.as_ref(), |view, v| {
        text(view, v.as_str());
        assert_eq!(
            view.kind_for_fixture(),
            if matches!(v, td_crate::data_type::BaseUri::Template(_)) {
                63
            } else {
                62
            }
        );
    });
    optional(view.child(8), v.properties.as_ref(), |view, values| {
        assert_eq!(view.len(), values.len());
        for (i, (key, v)) in values.iter().enumerate() {
            let (name, p) = view.member(i).unwrap();
            assert_eq!(name, key);
            schema::fields(p.child(0).unwrap(), &v._schema);
            forms(p.child(1).unwrap(), &v._interaction.forms);
            optional(
                p.child(2),
                v._interaction.uri_variables.as_ref(),
                schema_map,
            );
            boolean(p.child(3).unwrap(), v.observable);
        }
    });
    optional(view.child(9), v.actions.as_ref(), |view, values| {
        assert_eq!(view.len(), values.len());
        for (i, (key, v)) in values.iter().enumerate() {
            let (name, a) = view.member(i).unwrap();
            assert_eq!(name, key);
            metadata(a.child(0).unwrap(), &v._metadata);
            forms(a.child(1).unwrap(), &v._interaction.forms);
            optional(
                a.child(2),
                v._interaction.uri_variables.as_ref(),
                schema_map,
            );
            optional(a.child(3), v.input.as_ref(), schema::fields);
            optional(a.child(4), v.output.as_ref(), schema::fields);
            boolean(a.child(5).unwrap(), v.safe);
            boolean(a.child(6).unwrap(), v.idempotent);
            #[cfg(feature = "td2-preview")]
            optional(a.child(7), v.synchronous, boolean);
            #[cfg(not(feature = "td2-preview"))]
            assert!(a.child(7).is_none());
            extras(a.child(8).unwrap(), &v._extra_fields);
        }
    });
    optional(view.child(10), v.events.as_ref(), |view, values| {
        assert_eq!(view.len(), values.len());
        for (i, (key, v)) in values.iter().enumerate() {
            let (name, e) = view.member(i).unwrap();
            assert_eq!(name, key);
            metadata(e.child(0).unwrap(), &v._metadata);
            forms(e.child(1).unwrap(), &v._interaction.forms);
            optional(
                e.child(2),
                v._interaction.uri_variables.as_ref(),
                schema_map,
            );
            optional(e.child(3), v.subscription.as_ref(), schema::fields);
            optional(e.child(4), v.data.as_ref(), schema::fields);
            optional(e.child(5), v.data_response.as_ref(), schema::fields);
            optional(e.child(6), v.cancellation.as_ref(), schema::fields);
            extras(e.child(7).unwrap(), &v._extra_fields);
        }
    });
    optional(view.child(11), v.links.as_deref(), |view, values| {
        assert_eq!(view.len(), values.len());
        for (i, v) in values.iter().enumerate() {
            let link = view.child(i).unwrap();
            text(link.child(0).unwrap(), v.href.as_str());
            optional(link.child(1), v.content_type.as_deref(), text);
            optional(link.child(2), v.rel.as_deref(), text);
            optional(link.child(3), v.anchor.as_ref().map(|v| v.as_str()), text);
            optional(link.child(4), v.sizes.as_deref(), text);
            optional(link.child(5), v.hreflang.as_deref(), strings);
            extras(link.child(6).unwrap(), &v._extra_fields);
        }
    });
    optional(view.child(12), v.forms.as_deref(), forms);
    strings(view.child(13).unwrap(), &v.security);
    let definitions = view.child(14).unwrap();
    assert_eq!(definitions.len(), v.security_definitions.len());
    for (i, (key, v)) in v.security_definitions.iter().enumerate() {
        let (name, value) = definitions.member(i).unwrap();
        assert_eq!(name, key);
        security(value, v);
    }
    optional(view.child(15), v.profile.as_deref(), |view, values| {
        assert_eq!(view.len(), values.len());
        for (i, v) in values.iter().enumerate() {
            text(view.child(i).unwrap(), v.as_str());
        }
    });
    optional(view.child(16), v.schema_definitions.as_ref(), schema_map);
    optional(view.child(17), v.uri_variables.as_ref(), schema_map);
    extras(view.child(18).unwrap(), &v._extra_fields);
}
