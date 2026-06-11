use clinkz_wot_protocol_bindings::{
    resolve_form_target, resolve_selected_affordance_form_security, select_affordance_form,
    select_affordance_form_with_criteria, select_affordance_form_with_filter, select_form,
    select_form_with_criteria, select_form_with_filter, validate_affordance_form,
    validate_affordance_form_with_criteria, AffordanceRef, BindingError, FormSelectionCriteria,
};
use clinkz_wot_td::{
    affordance::{ActionAffordance, EventAffordance, InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    data_type::{Operation, ResolveFormHrefError, ResolvedFormHref},
    form::Form,
    td_defaults::FormContext,
    thing::Thing,
};

#[test]
fn selects_first_form_matching_effective_operation() {
    let read_form = Form::read_property("properties/status")
        .content_type("application/json")
        .build()
        .unwrap();
    let write_form = Form::write_property("properties/status")
        .content_type("application/cbor")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .forms([read_form, write_form.clone()])
        .build()
        .unwrap();

    let selected = select_form(
        FormContext::Property(&property),
        property._interaction.forms.as_slice(),
        Operation::WriteProperty,
    )
    .unwrap();

    assert_eq!(selected.index, 1);
    assert_eq!(selected.form, &write_form);
    assert_eq!(selected.operations.as_ref(), &[Operation::WriteProperty]);
}

#[test]
fn applies_affordance_default_operations() {
    let form = Form::builder("actions/ping").build().unwrap();
    let action = ActionAffordance::builder()
        .form(form.clone())
        .build()
        .unwrap();

    let selected = select_form(
        FormContext::Action(&action),
        action._interaction.forms.as_slice(),
        Operation::InvokeAction,
    )
    .unwrap();

    assert_eq!(selected.index, 0);
    assert_eq!(selected.form, &form);
    assert_eq!(selected.operations.as_ref(), &[Operation::InvokeAction]);
}

#[test]
fn reports_unsupported_operation_when_no_form_matches() {
    let form = Form::read_property("properties/status").build().unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(form)
        .build()
        .unwrap();

    let err = select_form(
        FormContext::Property(&property),
        property._interaction.forms.as_slice(),
        Operation::WriteProperty,
    )
    .unwrap_err();

    assert_eq!(
        err,
        BindingError::UnsupportedOperation("No form supports WriteProperty".into())
    );
}

#[test]
fn selects_form_matching_content_type_criteria() {
    let json_form = Form::read_property("properties/status")
        .content_type("application/json")
        .build()
        .unwrap();
    let cbor_form = Form::read_property("properties/status")
        .content_type("application/cbor")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .forms([json_form, cbor_form.clone()])
        .build()
        .unwrap();

    let selected = select_form_with_criteria(
        FormContext::Property(&property),
        property._interaction.forms.as_slice(),
        FormSelectionCriteria::new(Operation::ReadProperty).content_type("application/cbor"),
    )
    .unwrap();

    assert_eq!(selected.index, 1);
    assert_eq!(selected.form, &cbor_form);
}

#[test]
fn selects_form_matching_subprotocol_criteria() {
    let longpoll_form = Form::subscribe_event("events/ready")
        .content_type("application/json")
        .subprotocol("longpoll")
        .build()
        .unwrap();
    let sse_form = Form::subscribe_event("events/ready")
        .content_type("text/event-stream")
        .subprotocol("sse")
        .build()
        .unwrap();
    let event = EventAffordance::builder()
        .forms([longpoll_form, sse_form.clone()])
        .build()
        .unwrap();

    let selected = select_form_with_criteria(
        FormContext::Event(&event),
        event._interaction.forms.as_slice(),
        FormSelectionCriteria::new(Operation::SubscribeEvent).subprotocol("sse"),
    )
    .unwrap();

    assert_eq!(selected.index, 1);
    assert_eq!(selected.form, &sse_form);
}

#[test]
fn selects_form_matching_caller_filter() {
    let http_form = Form::read_property("https://example.com/things/lamp/properties/status")
        .build()
        .unwrap();
    let bus_form = Form::read_property("bus:things/lamp/properties/status")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .forms([http_form, bus_form.clone()])
        .build()
        .unwrap();

    let selected = select_form_with_filter(
        FormContext::Property(&property),
        property._interaction.forms.as_slice(),
        FormSelectionCriteria::new(Operation::ReadProperty),
        |form| form.href.as_str().starts_with("bus:"),
    )
    .unwrap();

    assert_eq!(selected.index, 1);
    assert_eq!(selected.form, &bus_form);
}

#[test]
fn reports_filter_mismatch_when_operation_exists() {
    let http_form = Form::read_property("https://example.com/things/lamp/properties/status")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(http_form)
        .build()
        .unwrap();

    let err = select_form_with_filter(
        FormContext::Property(&property),
        property._interaction.forms.as_slice(),
        FormSelectionCriteria::new(Operation::ReadProperty),
        |form| form.href.as_str().starts_with("bus:"),
    )
    .unwrap_err();

    assert_eq!(
        err,
        BindingError::CallerFilterMismatch(
            "No form matches FormSelectionCriteria { operation: ReadProperty, content_type: None, subprotocol: None } after applying caller filter".into()
        )
    );
}

#[test]
fn reports_metadata_mismatch_when_operation_exists() {
    let json_form = Form::read_property("properties/status")
        .content_type("application/json")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(json_form)
        .build()
        .unwrap();

    let err = select_form_with_criteria(
        FormContext::Property(&property),
        property._interaction.forms.as_slice(),
        FormSelectionCriteria::new(Operation::ReadProperty).content_type("application/cbor"),
    )
    .unwrap_err();

    assert_eq!(
        err,
        BindingError::MetadataMismatch(
            "No form matches FormSelectionCriteria { operation: ReadProperty, content_type: Some(\"application/cbor\"), subprotocol: None }".into()
        )
    );
}

#[test]
fn resolves_form_target_against_thing_base() {
    let form = Form::read_property("properties/status").build().unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(form.clone())
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .base("https://example.com/things/lamp/")
        .nosec()
        .property("status", property)
        .build()
        .unwrap();

    let target = resolve_form_target(&thing, &form).unwrap();

    match target.href {
        ResolvedFormHref::Reference(reference) => {
            assert_eq!(
                reference.as_str(),
                "https://example.com/things/lamp/properties/status"
            );
        }
        ResolvedFormHref::Template(_) => panic!("expected resolved URI reference"),
    }
}

#[test]
fn reports_target_resolution_failure_from_affordance_selection() {
    let form = Form::read_property("properties/status").build().unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(form)
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .base("https://example.com/{tenant}/")
        .nosec()
        .property("status", property)
        .build()
        .unwrap();

    let err = select_affordance_form(
        &thing,
        AffordanceRef::Property("status"),
        Operation::ReadProperty,
    )
    .unwrap_err();

    assert_eq!(
        err,
        BindingError::TargetResolution(ResolveFormHrefError::TemplateBase(
            "https://example.com/{tenant}/".into()
        ))
    );
}

#[test]
fn selects_and_resolves_property_form_from_thing_affordance() {
    let read_form = Form::read_property("properties/status")
        .content_type("application/json")
        .build()
        .unwrap();
    let write_form = Form::write_property("properties/status")
        .content_type("application/cbor")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .forms([read_form, write_form.clone()])
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .base("https://example.com/things/lamp/")
        .nosec()
        .property("status", property)
        .build()
        .unwrap();

    let selected = select_affordance_form(
        &thing,
        AffordanceRef::Property("status"),
        Operation::WriteProperty,
    )
    .unwrap();

    assert_eq!(selected.affordance, AffordanceRef::Property("status"));
    assert_eq!(selected.selection.index, 1);
    assert_eq!(selected.selection.form, &write_form);
    assert_eq!(
        selected.selection.operations.as_ref(),
        &[Operation::WriteProperty]
    );
    match selected.target.href {
        ResolvedFormHref::Reference(reference) => {
            assert_eq!(
                reference.as_str(),
                "https://example.com/things/lamp/properties/status"
            );
        }
        ResolvedFormHref::Template(_) => panic!("expected resolved URI reference"),
    }
}

#[test]
fn selects_and_resolves_affordance_form_with_metadata_criteria() {
    let json_form = Form::read_property("properties/status")
        .content_type("application/json")
        .build()
        .unwrap();
    let cbor_form = Form::read_property("properties/status")
        .content_type("application/cbor")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .forms([json_form, cbor_form.clone()])
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .base("https://example.com/things/lamp/")
        .nosec()
        .property("status", property)
        .build()
        .unwrap();

    let selected = select_affordance_form_with_criteria(
        &thing,
        AffordanceRef::Property("status"),
        FormSelectionCriteria::new(Operation::ReadProperty).content_type("application/cbor"),
    )
    .unwrap();

    assert_eq!(selected.selection.index, 1);
    assert_eq!(selected.selection.form, &cbor_form);
    match selected.target.href {
        ResolvedFormHref::Reference(reference) => {
            assert_eq!(
                reference.as_str(),
                "https://example.com/things/lamp/properties/status"
            );
        }
        ResolvedFormHref::Template(_) => panic!("expected resolved URI reference"),
    }
}

#[test]
fn selects_and_resolves_affordance_form_with_caller_filter() {
    let http_form = Form::read_property("https://example.com/things/lamp/properties/status")
        .build()
        .unwrap();
    let bus_form = Form::read_property("bus:things/lamp/properties/status")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .forms([http_form, bus_form.clone()])
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .nosec()
        .property("status", property)
        .build()
        .unwrap();

    let selected = select_affordance_form_with_filter(
        &thing,
        AffordanceRef::Property("status"),
        FormSelectionCriteria::new(Operation::ReadProperty),
        |form| form.href.as_str().starts_with("bus:"),
    )
    .unwrap();

    assert_eq!(selected.selection.index, 1);
    assert_eq!(selected.selection.form, &bus_form);
}

#[test]
fn selects_action_and_event_forms_with_default_operations() {
    let action_form = Form::builder("actions/ping").build().unwrap();
    let event_form = Form::builder("events/ready").build().unwrap();
    let action = ActionAffordance::builder()
        .form(action_form.clone())
        .build()
        .unwrap();
    let event = EventAffordance::builder()
        .form(event_form.clone())
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .base("https://example.com/things/lamp/")
        .nosec()
        .action("ping", action)
        .event("ready", event)
        .build()
        .unwrap();

    let selected_action = select_affordance_form(
        &thing,
        AffordanceRef::Action("ping"),
        Operation::InvokeAction,
    )
    .unwrap();
    let selected_event = select_affordance_form(
        &thing,
        AffordanceRef::Event("ready"),
        Operation::SubscribeEvent,
    )
    .unwrap();

    assert_eq!(selected_action.selection.form, &action_form);
    assert_eq!(
        selected_action.selection.operations.as_ref(),
        &[Operation::InvokeAction]
    );
    assert_eq!(selected_event.selection.form, &event_form);
    assert_eq!(
        selected_event.selection.operations.as_ref(),
        &[Operation::SubscribeEvent, Operation::UnsubscribeEvent]
    );
}

#[test]
fn reports_unknown_affordance_from_thing_lookup() {
    let thing = Thing::builder("Lamp").nosec().build().unwrap();

    let err = select_affordance_form(
        &thing,
        AffordanceRef::Property("status"),
        Operation::ReadProperty,
    )
    .unwrap_err();

    assert_eq!(
        err,
        BindingError::UnknownAffordance {
            kind: "property",
            name: "status".into()
        }
    );
}

#[test]
fn validates_selected_affordance_form_against_effective_operation() {
    let form = Form::builder("actions/ping").build().unwrap();
    let action = ActionAffordance::builder()
        .form(form.clone())
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .nosec()
        .action("ping", action)
        .build()
        .unwrap();

    let selected = validate_affordance_form(
        &thing,
        AffordanceRef::Action("ping"),
        &form,
        Operation::InvokeAction,
    )
    .unwrap();

    assert_eq!(selected.index, 0);
    assert_eq!(selected.form, &form);
    assert_eq!(selected.operations.as_ref(), &[Operation::InvokeAction]);
}

#[test]
fn validates_selected_thing_level_form() {
    let form = Form::builder("properties")
        .op([Operation::ReadAllProperties])
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .nosec()
        .form(form.clone())
        .build()
        .unwrap();

    let selected = validate_affordance_form(
        &thing,
        AffordanceRef::Thing,
        &form,
        Operation::ReadAllProperties,
    )
    .unwrap();

    assert_eq!(selected.index, 0);
    assert_eq!(selected.form, &form);
    assert_eq!(
        selected.operations.as_ref(),
        &[Operation::ReadAllProperties]
    );
}

#[test]
fn validates_selected_event_form_with_default_operations() {
    let form = Form::builder("events/ready").build().unwrap();
    let event = EventAffordance::builder()
        .form(form.clone())
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .nosec()
        .event("ready", event)
        .build()
        .unwrap();

    let selected = validate_affordance_form(
        &thing,
        AffordanceRef::Event("ready"),
        &form,
        Operation::UnsubscribeEvent,
    )
    .unwrap();

    assert_eq!(selected.index, 0);
    assert_eq!(
        selected.operations.as_ref(),
        &[Operation::SubscribeEvent, Operation::UnsubscribeEvent]
    );
}

#[test]
fn validates_copied_selected_form_value() {
    let form = Form::read_property("properties/status").build().unwrap();
    let copied_form = form.clone();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(form)
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .nosec()
        .property("status", property)
        .build()
        .unwrap();

    let selected = validate_affordance_form(
        &thing,
        AffordanceRef::Property("status"),
        &copied_form,
        Operation::ReadProperty,
    )
    .unwrap();

    assert_eq!(selected.index, 0);
    assert_eq!(selected.form, &copied_form);
}

#[test]
fn rejects_selected_affordance_form_when_operation_does_not_match() {
    let form = Form::read_property("properties/status").build().unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(form.clone())
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .nosec()
        .property("status", property)
        .build()
        .unwrap();

    let err = validate_affordance_form(
        &thing,
        AffordanceRef::Property("status"),
        &form,
        Operation::WriteProperty,
    )
    .unwrap_err();

    assert_eq!(
        err,
        BindingError::UnsupportedOperation(
            "Selected form does not support WriteProperty".into()
        )
    );
}

#[test]
fn rejects_selected_form_that_does_not_belong_to_affordance() {
    let property_form = Form::read_property("properties/status").build().unwrap();
    let other_form = Form::read_property("properties/other").build().unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(property_form)
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .nosec()
        .property("status", property)
        .build()
        .unwrap();

    let err = validate_affordance_form(
        &thing,
        AffordanceRef::Property("status"),
        &other_form,
        Operation::ReadProperty,
    )
    .unwrap_err();

    assert_eq!(err, BindingError::FormNotInAffordance);
}

#[test]
fn rejects_selected_affordance_form_when_metadata_does_not_match() {
    let form = Form::read_property("properties/status")
        .content_type("application/json")
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(form.clone())
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .nosec()
        .property("status", property)
        .build()
        .unwrap();

    let err = validate_affordance_form_with_criteria(
        &thing,
        AffordanceRef::Property("status"),
        &form,
        FormSelectionCriteria::new(Operation::ReadProperty).content_type("application/cbor"),
    )
    .unwrap_err();

    assert_eq!(
        err,
        BindingError::MetadataMismatch(
            "Selected form does not match FormSelectionCriteria { operation: ReadProperty, content_type: Some(\"application/cbor\"), subprotocol: None }".into()
        )
    );
}

#[test]
fn resolves_inherited_thing_level_security_for_selected_form() {
    let form = Form::read_property("properties/status").build().unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(form)
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .basic_security("basic_auth", "Authorization")
        .property("status", property)
        .build()
        .unwrap();
    let selected = select_affordance_form(
        &thing,
        AffordanceRef::Property("status"),
        Operation::ReadProperty,
    )
    .unwrap();

    let security = resolve_selected_affordance_form_security(&thing, &selected);

    assert_eq!(security.security, &["basic_auth".to_string()]);
    assert!(security.scopes.is_empty());
}

#[test]
fn resolves_form_level_security_and_scopes_for_selected_form() {
    let form = Form::read_property("properties/status")
        .security(["oauth"])
        .scopes(["status:read", "status:audit"])
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
        .form(form)
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .basic_security("basic_auth", "Authorization")
        .oauth2_client_security("oauth")
        .property("status", property)
        .build()
        .unwrap();
    let selected = select_affordance_form(
        &thing,
        AffordanceRef::Property("status"),
        Operation::ReadProperty,
    )
    .unwrap();

    let security = resolve_selected_affordance_form_security(&thing, &selected);

    assert_eq!(security.security, &["oauth".to_string()]);
    assert_eq!(
        security.scopes,
        &["status:read".to_string(), "status:audit".to_string()]
    );
}

#[test]
fn resolves_nosec_metadata_for_selected_form() {
    let form = Form::invoke_action("actions/ping").build().unwrap();
    let action = ActionAffordance::builder().form(form).build().unwrap();
    let thing = Thing::builder("Lamp")
        .nosec()
        .action("ping", action)
        .build()
        .unwrap();
    let selected = select_affordance_form(
        &thing,
        AffordanceRef::Action("ping"),
        Operation::InvokeAction,
    )
    .unwrap();

    let security = resolve_selected_affordance_form_security(&thing, &selected);

    assert_eq!(security.security, &["nosec".to_string()]);
    assert!(security.scopes.is_empty());
}
