use clinkz_wot_binding_core::{BindingCoreError, resolve_form_target, select_form};
use clinkz_wot_td::{
    affordance::{ActionAffordance, InteractionHelper, PropertyAffordance},
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
