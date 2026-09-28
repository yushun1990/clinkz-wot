//! Source-isolated Planning-shaped consumer for the frozen borrowed view.
//!
//! This file is compiled only by the TD test prototype. Its imports make the
//! dependency boundary explicit: no `Thing`, Snapshot, arena record, URI
//! resolver, or semantic adapter is available here.

use super::{Operation, ValidatedFormHref, ValidatedThingView};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PlanningSelection<'a> {
    pub thing_id: &'a str,
    pub property_count: usize,
    pub property_name: &'a str,
    pub property_ordinal: u32,
    pub form_original_index: u32,
    pub raw_href: ValidatedFormHref<'a>,
    pub resolved_href: ValidatedFormHref<'a>,
    pub content_type: &'a str,
    pub content_coding: Option<&'a str>,
    pub subprotocol: Option<&'a str>,
    pub scope_count: usize,
    pub security_name: &'a str,
    pub security_scheme_name: &'a str,
    pub security_scheme: &'a str,
}

pub(super) fn query_non_first_property_form(view: ValidatedThingView<'_>) -> PlanningSelection<'_> {
    let properties = view.properties();
    let property_count = properties.len();
    assert_eq!(properties.clone().count(), property_count);

    let property = view.property("zeta").expect("non-first Property exists");
    assert_eq!(property.forms().len(), 2);
    let form = property.form(1).expect("non-first Form exists");
    assert!(
        form.effective_operations()
            .any(|operation| operation == Operation::ReadProperty)
    );

    let mut security = form.effective_security();
    assert_eq!(security.len(), 1);
    let security_name = security.next().unwrap();
    assert_eq!(security.next(), None);
    let definition = view
        .security_definition(security_name)
        .expect("effective security definition exists");

    PlanningSelection {
        thing_id: view.id().unwrap(),
        property_count,
        property_name: property.name(),
        property_ordinal: property.ordinal(),
        form_original_index: form.original_index(),
        raw_href: form.href(),
        resolved_href: form.resolved_href().unwrap(),
        content_type: form.content_type(),
        content_coding: form.content_coding(),
        subprotocol: form.subprotocol(),
        scope_count: form.scopes().len(),
        security_name,
        security_scheme_name: definition.name(),
        security_scheme: definition.scheme(),
    }
}
