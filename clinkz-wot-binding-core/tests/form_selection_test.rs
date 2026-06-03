use clinkz_wot_binding_core::{
    AffordanceRef, BindingCoreError, FormSelectionCriteria, resolve_form_target,
    select_affordance_form, select_affordance_form_with_criteria, select_form,
    select_form_with_criteria,
};
use clinkz_wot_td::{
    affordance::{ActionAffordance, EventAffordance, InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    data_type::{Operation, ResolvedFormHref},
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
        BindingCoreError::UnsupportedOperation("No form supports WriteProperty".into())
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
        FormSelectionCriteria::operation(Operation::ReadProperty).content_type("application/cbor"),
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
        FormSelectionCriteria::operation(Operation::SubscribeEvent).subprotocol("sse"),
    )
    .unwrap();

    assert_eq!(selected.index, 1);
    assert_eq!(selected.form, &sse_form);
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
        FormSelectionCriteria::operation(Operation::ReadProperty).content_type("application/cbor"),
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
        BindingCoreError::UnknownAffordance {
            kind: "property",
            name: "status".into()
        }
    );
}
