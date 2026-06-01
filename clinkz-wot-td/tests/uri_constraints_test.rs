use clinkz_wot_td::{
    data_type::{AbsoluteUri, BaseUri, UriReference},
    form::Form,
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
