use clinkz_wot_td::{
    context::Context,
    data_type::{AbsoluteUri, BaseUri, ResolveFormHrefError, UriReference, resolve_form_href},
    form::Form,
    link::Link,
    security_scheme::{
        BearerSecurityScheme, ContextHelper as SecurityContextHelper, NoSecurityScheme,
        OAuth2SecurityScheme,
    },
    thing::Thing,
};

#[test]
fn absolute_uri_parser_rejects_relative_references_and_templates() {
    assert!(AbsoluteUri::parse("https://example.com/things/lamp").is_ok());
    assert!(AbsoluteUri::parse("urn:dev:ops:32473-HueSwitch-1234").is_ok());
    assert!(AbsoluteUri::parse("/properties/temperature").is_err());
    assert!(AbsoluteUri::parse("properties/temperature").is_err());
    assert!(AbsoluteUri::parse("https://example.com/{thingId}/").is_err());
}

#[test]
fn base_uri_accepts_absolute_uri_templates_but_rejects_relative_references() {
    let absolute_template = BaseUri::parse("https://example.com/{tenant}/")
        .expect("absolute URI template base should be accepted");
    assert!(absolute_template.is_template());

    assert!(BaseUri::parse("https://example.com/things/").is_ok());
    assert!(BaseUri::parse("/things/{tenant}/").is_err());
    assert!(BaseUri::parse("things/{tenant}/").is_err());
    assert!(BaseUri::parse("https://exa mple.com/{tenant}/").is_err());
}

#[test]
fn form_href_accepts_relative_references_and_templates() {
    let relative = Form::builder("/properties/temperature")
        .build()
        .expect("relative form href should be accepted");
    assert_eq!(relative.href, *"/properties/temperature");

    let template = Form::builder("/properties/{propertyName}")
        .build()
        .expect("URI template form href should be accepted");
    assert!(template.href.is_template());
}

#[test]
fn form_href_resolution_uses_thing_base_for_relative_references() {
    let base =
        BaseUri::parse("zenoh://clinkz/gateways/gw001/").expect("absolute base should parse");
    let form = Form::builder("properties/temperature")
        .build()
        .expect("relative form href should parse");

    let resolved = resolve_form_href(Some(&base), &form.href)
        .expect("relative href should resolve against absolute base");

    assert_eq!(
        resolved,
        *"zenoh://clinkz/gateways/gw001/properties/temperature"
    );
}

#[test]
fn form_href_resolution_preserves_absolute_references_without_using_base() {
    let base =
        BaseUri::parse("https://example.com/things/lamp/").expect("absolute base should parse");
    let form = Form::builder("zenoh://clinkz/things/lamp/properties/status")
        .build()
        .expect("absolute form href should parse");

    let resolved = resolve_form_href(Some(&base), &form.href)
        .expect("absolute href should not need base resolution");

    assert_eq!(resolved, *"zenoh://clinkz/things/lamp/properties/status");
}

#[test]
fn form_href_resolution_preserves_templates_for_runtime_expansion() {
    let base =
        BaseUri::parse("https://example.com/things/lamp/").expect("absolute base should parse");
    let form = Form::builder("properties/{propertyName}")
        .build()
        .expect("template form href should parse");

    let resolved =
        resolve_form_href(Some(&base), &form.href).expect("template href should be preserved");

    assert_eq!(resolved, *"properties/{propertyName}");
    assert!(resolved.is_template());
}

#[test]
fn form_href_resolution_rejects_concrete_resolution_against_template_base() {
    let base = BaseUri::parse("https://example.com/{tenant}/")
        .expect("absolute template base should parse");
    let form = Form::builder("properties/temperature")
        .build()
        .expect("relative form href should parse");

    let err = resolve_form_href(Some(&base), &form.href)
        .expect_err("relative href cannot be resolved against a template base");

    assert!(
        matches!(err, ResolveFormHrefError::TemplateBase(template) if template == "https://example.com/{tenant}/")
    );
}

#[test]
fn link_href_accepts_uri_references_but_rejects_templates() {
    assert!(UriReference::parse("/things/virtual-things-4").is_ok());
    assert!(UriReference::parse("https://example.com/things/lamp").is_ok());
    assert!(UriReference::parse("/things/{thingId}").is_err());
}

#[test]
fn deserialization_accepts_absolute_uri_template_base() {
    let raw = r#"{
        "@context": "https://www.w3.org/2022/wot/td/v1.1",
        "title": "Templated Base",
        "security": "nosec_sc",
        "securityDefinitions": {
            "nosec_sc": { "scheme": "nosec" }
        },
        "base": "https://example.com/{tenant}/",
        "forms": [
            { "href": "/things" }
        ]
    }"#;

    let thing: Thing = serde_json::from_str(raw).expect("TD should deserialize losslessly");
    let base = thing.base.expect("base should be preserved");
    assert_eq!(base.as_str(), "https://example.com/{tenant}/");
    assert!(base.is_template());
}

#[test]
fn thing_builder_reports_invalid_uri_inputs() {
    let err = match Thing::builder("Invalid URI Thing")
        .id("/relative-id")
        .build()
    {
        Ok(_) => panic!("invalid id should fail the builder"),
        Err(err) => err,
    };

    assert!(err.to_string().contains("id: /relative-id"));
}

#[test]
fn thing_builder_reports_invalid_profile_in_iterators() {
    let err = match Thing::builder("Invalid Profile Thing")
        .profiles(["https://example.com/profile", "/relative-profile"])
        .build()
    {
        Ok(_) => panic!("invalid profile should fail the builder"),
        Err(err) => err,
    };

    assert!(err.to_string().contains("profile: /relative-profile"));
}

#[test]
fn context_builder_reports_invalid_uri_inputs() {
    let err = Context::builder()
        .uri("/relative-context")
        .build()
        .expect_err("invalid context URI should fail the builder");

    assert!(err.to_string().contains("@context: /relative-context"));
}

#[test]
fn link_builder_reports_invalid_anchor_inputs() {
    Link::builder("/things/lamp")
        .anchor("/things/{thingId}")
        .build()
        .expect_err("invalid link anchor should fail the builder");
}

#[test]
fn security_builders_report_invalid_uri_inputs() {
    let proxy_err = NoSecurityScheme::builder()
        .proxy("/relative-proxy")
        .build()
        .expect_err("invalid proxy should fail the builder");
    assert!(proxy_err.to_string().contains("proxy: /relative-proxy"));

    let bearer_err = BearerSecurityScheme::builder()
        .authorization("/relative-auth")
        .build()
        .expect_err("invalid authorization should fail the builder");
    assert!(
        bearer_err
            .to_string()
            .contains("authorization: /relative-auth")
    );

    let oauth_err = OAuth2SecurityScheme::builder("code")
        .token("/relative-token")
        .build()
        .expect_err("invalid token should fail the builder");
    assert!(oauth_err.to_string().contains("token: /relative-token"));
}
