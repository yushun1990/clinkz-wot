use std::collections::BTreeMap;

use clinkz_wot_foundation::{
    GatewayDefaultV1, ResourceKind, ResourceLimits, StaticResourceProfile, WorkBudget, WorkClass,
};
use clinkz_wot_td::{
    ValidatedThing, ValidatedThingCursor, ValidatedThingStep,
    affordance::{InteractionHelper, PropertyAffordance},
    context::{Context, WOT_CONTEXT_1_1},
    data_schema::{ContextHelper, DataSchema},
    data_type::{AdditionalExpectedResponse, Metadata},
    form::Form,
    security_scheme::SecurityScheme,
    thing::Thing,
    validate::{Validate, ValidateError, ValidationLevel},
};
use serde_json::{Value, json};

fn full_budget() -> WorkBudget {
    WorkBudget::new()
        .with_remaining(WorkClass::DocumentNodes, u64::MAX)
        .with_remaining(WorkClass::JsonSchemaNodes, u64::MAX)
        .with_remaining(WorkClass::UriBytes, u64::MAX)
        .with_remaining(WorkClass::SecurityBranches, u64::MAX)
}

fn drive(mut cursor: ValidatedThingCursor) -> ValidatedThingStep {
    loop {
        let mut budget = full_budget();
        match cursor.step(&mut budget, false) {
            ValidatedThingStep::Pending(next) => cursor = next,
            terminal => return terminal,
        }
    }
}

fn complete_with(thing: Thing, limits: &ResourceLimits) -> ValidatedThing {
    match drive(ValidatedThingCursor::new(thing, limits)) {
        ValidatedThingStep::Complete(validated) => validated,
        _ => panic!("expected validated Thing"),
    }
}

fn complete(thing: Thing) -> ValidatedThing {
    complete_with(thing, GatewayDefaultV1::limits())
}

fn minimal(title: impl Into<String>) -> Thing {
    Thing::builder(title).nosec().build().unwrap()
}

fn expect_limit(thing: Thing, limits: &ResourceLimits, expected: ResourceKind) -> Thing {
    match drive(ValidatedThingCursor::new(thing, limits)) {
        ValidatedThingStep::Limit {
            thing,
            kind,
            configured,
            observed,
        } => {
            assert_eq!(kind, expected);
            assert_eq!(configured, limits.get(expected));
            if configured.is_some() {
                assert!(observed.is_none_or(|observed| observed > configured.unwrap()));
            }
            thing
        }
        _ => panic!("expected {expected:?} limit"),
    }
}

#[test]
fn frozen_public_signatures_compile_exactly() {
    let _: fn(Thing, &ResourceLimits) -> ValidatedThingCursor = ValidatedThingCursor::new;
    let _: fn(ValidatedThingCursor, &mut WorkBudget, bool) -> ValidatedThingStep =
        ValidatedThingCursor::step;
    let _: for<'a> fn(&'a ValidatedThing) -> &'a Thing = ValidatedThing::thing;
    let _: fn(&ValidatedThing) -> u64 = ValidatedThing::retained_source_bytes;
    let _: fn(&ValidatedThing) -> u64 = ValidatedThing::property_count;
    let _: fn(&ValidatedThing) -> u64 = ValidatedThing::readable_property_form_count;
}

#[test]
fn exact_original_thing_reaches_every_terminal_owner() {
    let title = String::from("exact owner");
    let title_pointer = title.as_ptr();
    let completed = complete(minimal(title));
    assert_eq!(
        completed.thing()._metadata.title.as_ref().unwrap().as_ptr(),
        title_pointer
    );

    let mut invalid = minimal("invalid owner");
    invalid._metadata.title.as_mut().unwrap().clear();
    let invalid_pointer = invalid._metadata.title.as_ref().unwrap().as_ptr();
    match drive(ValidatedThingCursor::new(
        invalid,
        GatewayDefaultV1::limits(),
    )) {
        ValidatedThingStep::Invalid { thing, error } => {
            assert_eq!(
                thing._metadata.title.as_ref().unwrap().as_ptr(),
                invalid_pointer
            );
            assert_eq!(error, ValidateError::MissingRequiredField("title".into()));
        }
        _ => panic!("expected Basic-validation failure"),
    }

    let limited = minimal("limited owner");
    let limited_pointer = limited._metadata.title.as_ref().unwrap().as_ptr();
    let limits = GatewayDefaultV1::limits()
        .clone()
        .with_limit(ResourceKind::StringBytesMax, Some(0));
    let limited = expect_limit(limited, &limits, ResourceKind::StringBytesMax);
    assert_eq!(
        limited._metadata.title.as_ref().unwrap().as_ptr(),
        limited_pointer
    );

    let cancelled = minimal("cancelled owner");
    let cancelled_pointer = cancelled._metadata.title.as_ref().unwrap().as_ptr();
    let mut zero = WorkBudget::new();
    match ValidatedThingCursor::new(cancelled, GatewayDefaultV1::limits()).step(&mut zero, true) {
        ValidatedThingStep::Cancelled(thing) => assert_eq!(
            thing._metadata.title.as_ref().unwrap().as_ptr(),
            cancelled_pointer
        ),
        _ => panic!("cancellation must win at zero budget"),
    }
}

#[test]
fn pending_is_linear_and_fresh_budget_does_not_reset_lifetime_work() {
    let thing = minimal("pending");
    let title_pointer = thing._metadata.title.as_ref().unwrap().as_ptr();
    let mut zero = WorkBudget::new();
    let cursor =
        match ValidatedThingCursor::new(thing, GatewayDefaultV1::limits()).step(&mut zero, false) {
            ValidatedThingStep::Pending(cursor) => cursor,
            _ => panic!("zero work must not progress"),
        };
    match cursor.step(&mut zero, true) {
        ValidatedThingStep::Cancelled(thing) => assert_eq!(
            thing._metadata.title.as_ref().unwrap().as_ptr(),
            title_pointer
        ),
        _ => panic!("the unique pending cursor must return the exact Thing"),
    }

    let limits = GatewayDefaultV1::limits()
        .clone()
        .with_limit(ResourceKind::DocumentValidationWorkUnitsMax, Some(1));
    let mut first_budget = full_budget();
    first_budget.set_remaining(WorkClass::DocumentNodes, 1);
    let cursor = match ValidatedThingCursor::new(minimal("lifetime"), &limits)
        .step(&mut first_budget, false)
    {
        ValidatedThingStep::Pending(cursor) => cursor,
        _ => panic!("the next rejected unit must leave a pending continuation"),
    };
    let mut fresh_budget = full_budget();
    match cursor.step(&mut fresh_budget, false) {
        ValidatedThingStep::Limit {
            kind,
            configured,
            observed,
            ..
        } => {
            assert_eq!(kind, ResourceKind::DocumentValidationWorkUnitsMax);
            assert_eq!(configured, Some(1));
            assert_eq!(observed, Some(2));
        }
        _ => panic!("fresh caller budget must not reset cursor lifetime work"),
    }
}

#[test]
fn basic_validation_has_exact_positive_and_negative_parity() {
    let valid = minimal("valid parity");
    assert_eq!(valid.validate_with_level(ValidationLevel::Basic), Ok(()));
    assert!(matches!(
        drive(ValidatedThingCursor::new(valid, GatewayDefaultV1::limits())),
        ValidatedThingStep::Complete(_)
    ));

    let mut invalid = minimal("invalid parity");
    invalid.security.push("missing".into());
    let expected = invalid
        .validate_with_level(ValidationLevel::Basic)
        .expect_err("fixture must be invalid");
    match drive(ValidatedThingCursor::new(
        invalid,
        GatewayDefaultV1::limits(),
    )) {
        ValidatedThingStep::Invalid { error, .. } => assert_eq!(error, expected),
        _ => panic!("cursor must delegate to the existing Basic authority"),
    }
}

#[test]
fn idless_thing_and_effective_readable_property_counts_complete() {
    let readable = PropertyAffordance::builder(DataSchema::string())
        .form(Form::builder("/readable").build().unwrap())
        .build()
        .unwrap();
    let write_only = PropertyAffordance::builder(DataSchema::String(
        DataSchema::string().write_only(true).build(),
    ))
    .form(Form::builder("/write-only").build().unwrap())
    .build()
    .unwrap();
    let empty = PropertyAffordance::builder(DataSchema::string())
        .build()
        .unwrap();
    let thing = Thing::builder("counts")
        .nosec()
        .property("readable", readable)
        .property("write-only", write_only)
        .property("empty", empty)
        .build()
        .unwrap();
    assert!(thing.id.is_none());

    let validated = complete(thing);
    assert!(validated.thing().id.is_none());
    assert_eq!(validated.property_count(), 3);
    assert_eq!(validated.readable_property_form_count(), 1);
}

#[test]
fn retained_footprint_counts_string_and_context_storage_capacity() {
    let ordinary = complete(minimal("x"));

    let mut over_reserved_title = String::with_capacity(4096);
    over_reserved_title.push('x');
    let title_capacity = over_reserved_title.capacity() as u64;
    let title = complete(minimal(over_reserved_title));
    assert!(title.retained_source_bytes() >= ordinary.retained_source_bytes() + title_capacity - 1);

    let mut context_value = String::with_capacity(8192);
    context_value.push('v');
    let mut object = BTreeMap::new();
    object.insert("extension".into(), Value::String(context_value));
    let context = Context::builder().object(object).build().unwrap();
    let mut thing = minimal("context value");
    thing.context = context;
    let serialized_len = serde_json::to_vec(&thing).unwrap().len() as u64;
    let retained = complete(thing).retained_source_bytes();
    assert!(retained > serialized_len + 7000);

    let compact_context = Context::new().with_1_0_compatibility();
    let mut builder = Context::builder();
    for _ in 0..128 {
        builder = builder.uri(WOT_CONTEXT_1_1);
    }
    let over_capacity_context = builder.with_1_0_compatibility().build().unwrap();
    let mut compact = minimal("context vec");
    compact.context = compact_context;
    let mut over_capacity = minimal("context vec");
    over_capacity.context = over_capacity_context;
    assert_eq!(
        serde_json::to_value(&compact).unwrap(),
        serde_json::to_value(&over_capacity).unwrap()
    );
    assert!(
        complete(over_capacity).retained_source_bytes()
            > complete(compact).retained_source_bytes() + 1000
    );
}

#[test]
fn retained_footprint_preserves_and_counts_nested_reserved_buffers() {
    let compact = Thing::builder("nested reserved buffers")
        .nosec()
        .extra_field("nested", json!({"items": ["v"]}))
        .build()
        .unwrap();

    let mut nested_string = String::with_capacity(8192);
    nested_string.push('v');
    let nested_string_pointer = nested_string.as_ptr();
    let mut items = Vec::with_capacity(512);
    items.push(Value::String(nested_string));
    let items_pointer = items.as_ptr();
    let mut object = serde_json::Map::new();
    object.insert("items".into(), Value::Array(items));
    let reserved = Thing::builder("nested reserved buffers")
        .nosec()
        .extra_field("nested", Value::Object(object))
        .build()
        .unwrap();

    assert_eq!(
        serde_json::to_value(&compact).unwrap(),
        serde_json::to_value(&reserved).unwrap(),
        "serialized content cannot reveal nested spare capacity"
    );
    let compact_retained = complete(compact).retained_source_bytes();
    let reserved = complete(reserved);
    assert!(
        reserved.retained_source_bytes() > compact_retained + 20_000,
        "nested String and Value Vec capacities must both enter the census"
    );

    let Value::Object(object) = &reserved.thing()._extra_fields["nested"] else {
        panic!("nested object was not preserved");
    };
    let Value::Array(items) = &object["items"] else {
        panic!("nested array was not preserved");
    };
    let Value::String(value) = &items[0] else {
        panic!("nested string was not preserved");
    };
    assert_eq!(items.as_ptr(), items_pointer);
    assert_eq!(value.as_ptr(), nested_string_pointer);
}

#[test]
fn retained_document_limits_accept_exact_and_reject_one_below() {
    let observed = complete(minimal("footprint boundary")).retained_source_bytes();
    let exact = GatewayDefaultV1::limits()
        .clone()
        .with_limit(ResourceKind::DocumentBytesMax, Some(observed))
        .with_limit(ResourceKind::RetainedSourceBytesPerOwnerMax, Some(observed));
    assert_eq!(
        complete_with(minimal("footprint boundary"), &exact).retained_source_bytes(),
        observed
    );

    let one_below = exact
        .clone()
        .with_limit(ResourceKind::DocumentBytesMax, Some(observed - 1));
    expect_limit(
        minimal("footprint boundary"),
        &one_below,
        ResourceKind::DocumentBytesMax,
    );

    let one_below = exact.clone().with_limit(
        ResourceKind::RetainedSourceBytesPerOwnerMax,
        Some(observed - 1),
    );
    expect_limit(
        minimal("footprint boundary"),
        &one_below,
        ResourceKind::RetainedSourceBytesPerOwnerMax,
    );
}

#[test]
fn missing_applicable_limit_is_terminal_without_work() {
    let limits = GatewayDefaultV1::limits()
        .clone()
        .with_limit(ResourceKind::SchemaNodesPerDocumentMax, None);
    let mut budget = WorkBudget::new();
    match ValidatedThingCursor::new(minimal("missing limit"), &limits).step(&mut budget, false) {
        ValidatedThingStep::Limit {
            kind,
            configured,
            observed,
            ..
        } => {
            assert_eq!(kind, ResourceKind::SchemaNodesPerDocumentMax);
            assert_eq!(configured, None);
            assert_eq!(observed, None);
            assert!(budget.is_exhausted());
        }
        _ => panic!("missing applicable limits are not unbounded"),
    }
}

#[test]
fn typed_structure_limits_report_exact_kinds_and_observations() {
    let property =
        PropertyAffordance::builder(DataSchema::object().property("nested", DataSchema::string()))
            .form(Form::read_property("/{value}").build().unwrap())
            .build()
            .unwrap();
    let thing = Thing::builder("structure")
        .nosec()
        .property("p", property)
        .uri_variable("value", DataSchema::string())
        .build()
        .unwrap();

    for kind in [
        ResourceKind::AffordancesPerThingMax,
        ResourceKind::FormsPerContextMax,
        ResourceKind::FormsPerThingMax,
        ResourceKind::UriVariablesPerFormMax,
        ResourceKind::SchemaNodesPerDocumentMax,
        ResourceKind::SchemaReferenceEdgesPerDocumentMax,
        ResourceKind::UriTemplateSourceBytesMax,
        ResourceKind::UriTemplateVariablesMax,
    ] {
        let limits = GatewayDefaultV1::limits().clone().with_limit(kind, Some(0));
        expect_limit(thing.clone(), &limits, kind);
    }

    let form = Form::read_property("/p")
        .additional_response(AdditionalExpectedResponse::new("application/json".into()))
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::string())
        .form(form)
        .build()
        .unwrap();
    let thing = Thing::builder("responses")
        .nosec()
        .property("p", property)
        .build()
        .unwrap();
    let limits = GatewayDefaultV1::limits()
        .clone()
        .with_limit(ResourceKind::AdditionalResponsesPerFormMax, Some(0));
    expect_limit(thing, &limits, ResourceKind::AdditionalResponsesPerFormMax);
}

#[test]
fn typed_structure_exact_boundaries_are_accepted() {
    let property =
        PropertyAffordance::builder(DataSchema::object().property("nested", DataSchema::string()))
            .form(Form::read_property("/{value}").build().unwrap())
            .build()
            .unwrap();
    let thing = Thing::builder("structure boundaries")
        .nosec()
        .property("p", property)
        .uri_variable("value", DataSchema::string())
        .build()
        .unwrap();
    let limits = GatewayDefaultV1::limits()
        .clone()
        .with_limit(ResourceKind::AffordancesPerThingMax, Some(1))
        .with_limit(ResourceKind::FormsPerContextMax, Some(1))
        .with_limit(ResourceKind::FormsPerThingMax, Some(1))
        .with_limit(ResourceKind::UriVariablesPerFormMax, Some(1))
        .with_limit(ResourceKind::SchemaNodesPerDocumentMax, Some(3))
        .with_limit(ResourceKind::SchemaCompositionDepthMax, Some(2))
        .with_limit(ResourceKind::SchemaReferenceEdgesPerDocumentMax, Some(1))
        .with_limit(ResourceKind::UriTemplateSourceBytesMax, Some(8))
        .with_limit(ResourceKind::UriTemplateVariablesMax, Some(1));
    assert!(matches!(
        drive(ValidatedThingCursor::new(thing, &limits)),
        ValidatedThingStep::Complete(_)
    ));

    let form = Form::read_property("/p")
        .additional_response(AdditionalExpectedResponse::new("application/json".into()))
        .build()
        .unwrap();
    let property = PropertyAffordance::builder(DataSchema::string())
        .form(form)
        .build()
        .unwrap();
    let thing = Thing::builder("response boundary")
        .nosec()
        .property("p", property)
        .build()
        .unwrap();
    let limits = GatewayDefaultV1::limits()
        .clone()
        .with_limit(ResourceKind::AdditionalResponsesPerFormMax, Some(1));
    assert!(matches!(
        drive(ValidatedThingCursor::new(thing, &limits)),
        ValidatedThingStep::Complete(_)
    ));
}

#[test]
fn schema_json_and_security_depths_use_their_existing_limits() {
    let nested_schema =
        DataSchema::array().items([DataSchema::array().items([DataSchema::string()])]);
    let thing = Thing::builder("schema depth")
        .nosec()
        .schema_definition("nested", nested_schema)
        .build()
        .unwrap();
    let limits = GatewayDefaultV1::limits()
        .clone()
        .with_limit(ResourceKind::SchemaCompositionDepthMax, Some(2));
    expect_limit(thing, &limits, ResourceKind::SchemaCompositionDepthMax);

    let thing = Thing::builder("json limits")
        .nosec()
        .extra_field("nested", json!([{"member": [1, 2]}]))
        .build()
        .unwrap();
    for kind in [
        ResourceKind::JsonNestingDepthMax,
        ResourceKind::JsonMembersPerObjectMax,
        ResourceKind::JsonArrayItemsMax,
        ResourceKind::JsonValueNodesPerDocumentMax,
        ResourceKind::ExtensionBytesMax,
    ] {
        let limits = GatewayDefaultV1::limits().clone().with_limit(kind, Some(0));
        expect_limit(thing.clone(), &limits, kind);
    }

    let thing = Thing::builder("security depth")
        .security_definition("leaf-a", SecurityScheme::nosec())
        .security_definition("leaf-b", SecurityScheme::nosec())
        .security_definition("inner", SecurityScheme::combo_one_of(["leaf-a", "leaf-b"]))
        .security_named("outer", SecurityScheme::combo_one_of(["inner", "leaf-a"]))
        .build()
        .unwrap();
    let limits = GatewayDefaultV1::limits()
        .clone()
        .with_limit(ResourceKind::SecurityExpressionDepthMax, Some(1));
    expect_limit(
        thing.clone(),
        &limits,
        ResourceKind::SecurityExpressionDepthMax,
    );
    let limits = GatewayDefaultV1::limits()
        .clone()
        .with_limit(ResourceKind::SecurityBranchesPerPlanMax, Some(2));
    expect_limit(thing, &limits, ResourceKind::SecurityBranchesPerPlanMax);
}

#[test]
fn json_and_security_exact_boundaries_are_accepted() {
    let thing = Thing::builder("json boundaries")
        .nosec()
        .extra_field("nested", json!([{"member": [1, 2]}]))
        .build()
        .unwrap();
    let limits = GatewayDefaultV1::limits()
        .clone()
        .with_limit(ResourceKind::JsonNestingDepthMax, Some(3))
        .with_limit(ResourceKind::JsonMembersPerObjectMax, Some(1))
        .with_limit(ResourceKind::JsonArrayItemsMax, Some(2))
        .with_limit(ResourceKind::JsonValueNodesPerDocumentMax, Some(5));
    assert!(matches!(
        drive(ValidatedThingCursor::new(thing, &limits)),
        ValidatedThingStep::Complete(_)
    ));

    let thing = Thing::builder("security boundaries")
        .security_definition("leaf-a", SecurityScheme::nosec())
        .security_definition("leaf-b", SecurityScheme::nosec())
        .security_definition("inner", SecurityScheme::combo_one_of(["leaf-a", "leaf-b"]))
        .security_named("outer", SecurityScheme::combo_one_of(["inner", "leaf-a"]))
        .build()
        .unwrap();
    let limits = GatewayDefaultV1::limits()
        .clone()
        .with_limit(ResourceKind::SecurityExpressionDepthMax, Some(3))
        .with_limit(ResourceKind::SecurityBranchesPerPlanMax, Some(5));
    assert!(matches!(
        drive(ValidatedThingCursor::new(thing, &limits)),
        ValidatedThingStep::Complete(_)
    ));

    let two_roots = Thing::builder("aggregate security branches")
        .security_definition("leaf-a", SecurityScheme::nosec())
        .security_definition("leaf-b", SecurityScheme::nosec())
        .security_named(
            "outer-a",
            SecurityScheme::combo_one_of(["leaf-a", "leaf-b"]),
        )
        .security_named(
            "outer-b",
            SecurityScheme::combo_one_of(["leaf-a", "leaf-b"]),
        )
        .build()
        .unwrap();
    let exact = GatewayDefaultV1::limits()
        .clone()
        .with_limit(ResourceKind::SecurityExpressionDepthMax, Some(2))
        .with_limit(ResourceKind::SecurityBranchesPerPlanMax, Some(6));
    assert!(matches!(
        drive(ValidatedThingCursor::new(two_roots.clone(), &exact)),
        ValidatedThingStep::Complete(_)
    ));
    let one_below = exact.with_limit(ResourceKind::SecurityBranchesPerPlanMax, Some(5));
    expect_limit(
        two_roots,
        &one_below,
        ResourceKind::SecurityBranchesPerPlanMax,
    );
}

#[test]
fn census_work_classes_are_independent() {
    let property = PropertyAffordance::builder(DataSchema::string())
        .form(Form::read_property("/{x}").build().unwrap())
        .build()
        .unwrap();
    let thing = Thing::builder("classes")
        .nosec()
        .property("p", property)
        .build()
        .unwrap();
    let mut cursor = ValidatedThingCursor::new(thing, GatewayDefaultV1::limits());

    let mut document_only = WorkBudget::new().with_remaining(WorkClass::DocumentNodes, u64::MAX);
    cursor = match cursor.step(&mut document_only, false) {
        ValidatedThingStep::Pending(cursor) => cursor,
        _ => panic!("schema/security/URI work must not be relabelled as DocumentNodes"),
    };
    assert_eq!(document_only.remaining(WorkClass::JsonSchemaNodes), 0);
    assert_eq!(document_only.remaining(WorkClass::UriBytes), 0);
    assert_eq!(document_only.remaining(WorkClass::SecurityBranches), 0);

    let mut all = full_budget().with_remaining(WorkClass::PlanningItems, 7);
    assert!(matches!(
        cursor.step(&mut all, false),
        ValidatedThingStep::Complete(_)
    ));
    assert_eq!(
        all.remaining(WorkClass::PlanningItems),
        7,
        "TD validation must never charge PlanningItems"
    );
}

#[test]
fn string_limit_boundary_uses_content_while_footprint_uses_capacity() {
    let thing = minimal("abc");
    let limits = GatewayDefaultV1::limits()
        .clone()
        .with_limit(ResourceKind::StringBytesMax, Some(0));
    let returned = expect_limit(thing, &limits, ResourceKind::StringBytesMax);
    assert_eq!(returned._metadata.title.as_deref(), Some("abc"));

    let mut metadata = Metadata {
        title: Some("abc".into()),
        ..Metadata::default()
    };
    metadata.title.as_mut().unwrap().reserve(4096);
    let mut thing = minimal("placeholder");
    thing._metadata = metadata;
    assert!(complete(thing).retained_source_bytes() > 4096);
}

#[test]
fn largest_contiguous_boundary_uses_actual_buffer_capacity() {
    let mut title = String::with_capacity(65_536);
    title.push('x');
    let capacity = title.capacity() as u64;
    let limits = GatewayDefaultV1::limits().clone().with_limit(
        ResourceKind::LargestContiguousAllocationBytesMax,
        Some(capacity),
    );
    assert!(matches!(
        drive(ValidatedThingCursor::new(minimal(title), &limits)),
        ValidatedThingStep::Complete(_)
    ));

    let mut title = String::with_capacity(capacity as usize);
    title.push('x');
    let limits = limits.with_limit(
        ResourceKind::LargestContiguousAllocationBytesMax,
        Some(title.capacity() as u64 - 1),
    );
    expect_limit(
        minimal(title),
        &limits,
        ResourceKind::LargestContiguousAllocationBytesMax,
    );
}
