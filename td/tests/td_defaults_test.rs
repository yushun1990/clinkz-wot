use clinkz_wot_td::{
    affordance::{ActionAffordance, EventAffordance, InteractionHelper, PropertyAffordance},
    data_schema::{ContextHelper, DataSchema},
    data_type::{AdditionalExpectedResponse, Operation},
    form::Form,
    td_defaults::{
        FormContext, effective_additional_response_content_type, effective_form_operations,
        effective_form_security,
    },
    thing::Thing,
};

fn form(href: &str) -> Form {
    Form::builder(href).build().expect("form should build")
}

fn string_schema() -> DataSchema {
    DataSchema::String(DataSchema::string().build())
}

#[test]
fn explicit_form_operations_are_returned_unchanged() {
    let explicit = Form::builder("/properties/status")
        .op([Operation::ObserveProperty, Operation::UnobserveProperty])
        .build()
        .expect("form should build");
    let property = PropertyAffordance::builder(string_schema())
        .form(explicit.clone())
        .build()
        .expect("property should build");

    let operations = effective_form_operations(FormContext::Property(&property), &explicit);

    assert_eq!(
        operations.as_ref(),
        &[Operation::ObserveProperty, Operation::UnobserveProperty]
    );
}

#[test]
fn property_default_operations_follow_read_and_write_flags() {
    let writable_property = PropertyAffordance::builder(string_schema())
        .form(form("/properties/status"))
        .build()
        .expect("property should build");
    assert_eq!(
        effective_form_operations(
            FormContext::Property(&writable_property),
            &writable_property._interaction.forms[0]
        )
        .as_ref(),
        &[Operation::ReadProperty, Operation::WriteProperty]
    );

    let read_only_property = PropertyAffordance::builder(DataSchema::String(
        DataSchema::string().read_only(true).build(),
    ))
    .form(form("/properties/status"))
    .build()
    .expect("property should build");
    assert_eq!(
        effective_form_operations(
            FormContext::Property(&read_only_property),
            &read_only_property._interaction.forms[0]
        )
        .as_ref(),
        &[Operation::ReadProperty]
    );

    let write_only_property = PropertyAffordance::builder(DataSchema::String(
        DataSchema::string().write_only(true).build(),
    ))
    .form(form("/properties/status"))
    .build()
    .expect("property should build");
    assert_eq!(
        effective_form_operations(
            FormContext::Property(&write_only_property),
            &write_only_property._interaction.forms[0]
        )
        .as_ref(),
        &[Operation::WriteProperty]
    );
}

#[test]
fn action_event_and_thing_default_operations_are_context_specific() {
    let action_form = form("/actions/reboot");
    let action = ActionAffordance::builder()
        .form(action_form.clone())
        .build()
        .expect("action should build");
    assert_eq!(
        effective_form_operations(FormContext::Action(&action), &action_form).as_ref(),
        &[Operation::InvokeAction]
    );

    let event_form = form("/events/alerts");
    let event = EventAffordance::builder()
        .form(event_form.clone())
        .build()
        .expect("event should build");
    assert_eq!(
        effective_form_operations(FormContext::Event(&event), &event_form).as_ref(),
        &[Operation::SubscribeEvent, Operation::UnsubscribeEvent]
    );

    let thing_form = form("/properties");
    assert!(effective_form_operations(FormContext::Thing, &thing_form).is_empty());
}

#[test]
fn form_security_overrides_or_inherits_thing_security() {
    let thing: Thing = serde_json::from_str(
        r#"{
            "@context": "https://www.w3.org/2022/wot/td/v1.1",
            "title": "Security Defaults",
            "security": "nosec_sc",
            "securityDefinitions": {
                "nosec_sc": { "scheme": "nosec" },
                "oauth2_sc": {
                    "scheme": "oauth2",
                    "flow": "client",
                    "token": "https://auth.example.com/token"
                }
            },
            "forms": [
                { "href": "/properties/status" },
                {
                    "href": "/properties/status",
                    "security": "oauth2_sc"
                }
            ]
        }"#,
    )
    .expect("thing should deserialize");

    let forms = thing.forms.as_ref().expect("forms should be present");

    assert_eq!(effective_form_security(&thing, &forms[0]), &["nosec_sc"]);
    assert_eq!(effective_form_security(&thing, &forms[1]), &["oauth2_sc"]);
}

#[test]
fn additional_response_content_type_inherits_from_parent_form() {
    let form = Form::builder("/actions/reboot")
        .content_type("application/cbor")
        .additional_response(AdditionalExpectedResponse::default().schema("error"))
        .additional_response(AdditionalExpectedResponse::new(
            "application/problem+json".to_string(),
        ))
        .build()
        .expect("form should build");

    let additional_responses = form
        .additional_responses
        .as_ref()
        .expect("additional responses should be present");

    assert_eq!(
        effective_additional_response_content_type(&form, &additional_responses[0]),
        "application/cbor"
    );
    assert_eq!(
        effective_additional_response_content_type(&form, &additional_responses[1]),
        "application/problem+json"
    );
}
