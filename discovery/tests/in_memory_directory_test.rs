use clinkz_wot_discovery::{
    DirectoryQuery, DiscoveryError, InMemoryThingDirectory, QueryFilter, ThingDirectory,
};
use clinkz_wot_td::{
    affordance::PropertyAffordance, data_schema::DataSchema, security_scheme::SecurityScheme,
    thing::Thing, validate::ValidationLevel,
};

fn thing(id: &str, title: &str) -> Thing {
    Thing::builder(title)
        .id(id)
        .nosec()
        .build()
        .expect("valid Thing Description")
}

fn thing_with_property(id: &str, title: &str, property: &str) -> Thing {
    Thing::builder(title)
        .id(id)
        .nosec()
        .property(
            property,
            PropertyAffordance::builder(DataSchema::string())
                .build()
                .unwrap(),
        )
        .build()
        .expect("valid Thing Description")
}

#[test]
fn registers_and_retrieves_thing_description() {
    let mut directory = InMemoryThingDirectory::new();

    directory
        .register(thing("urn:thing:lamp", "Lamp"))
        .expect("registration succeeds");

    let retrieved = directory.get("urn:thing:lamp").expect("TD is present");

    assert_eq!(retrieved.id.as_ref().unwrap().as_str(), "urn:thing:lamp");
    assert_eq!(retrieved._metadata.title.as_deref(), Some("Lamp"));
}

#[test]
fn rejects_duplicate_registration() {
    let mut directory = InMemoryThingDirectory::new();
    directory
        .register(thing("urn:thing:lamp", "Lamp"))
        .expect("registration succeeds");

    let err = directory
        .register(thing("urn:thing:lamp", "Other Lamp"))
        .expect_err("duplicate id is rejected");

    assert_eq!(
        err,
        DiscoveryError::DuplicateThingId("urn:thing:lamp".to_string())
    );
}

#[test]
fn updates_existing_thing_description() {
    let mut directory = InMemoryThingDirectory::new();
    directory
        .register(thing("urn:thing:lamp", "Lamp"))
        .expect("registration succeeds");

    directory
        .update(thing("urn:thing:lamp", "Desk Lamp"))
        .expect("update succeeds");

    let retrieved = directory.get("urn:thing:lamp").expect("TD is present");
    assert_eq!(retrieved._metadata.title.as_deref(), Some("Desk Lamp"));
}

#[test]
fn rejects_update_for_unknown_thing() {
    let mut directory = InMemoryThingDirectory::new();

    let err = directory
        .update(thing("urn:thing:missing", "Missing"))
        .expect_err("unknown id is rejected");

    assert_eq!(
        err,
        DiscoveryError::ThingNotFound("urn:thing:missing".to_string())
    );
}

#[test]
fn deletes_thing_description() {
    let mut directory = InMemoryThingDirectory::new();
    directory
        .register(thing("urn:thing:lamp", "Lamp"))
        .expect("registration succeeds");

    let deleted = directory
        .delete("urn:thing:lamp")
        .expect("deletion succeeds");

    assert_eq!(deleted._metadata.title.as_deref(), Some("Lamp"));
    assert_eq!(
        directory.get("urn:thing:lamp").unwrap_err(),
        DiscoveryError::ThingNotFound("urn:thing:lamp".to_string())
    );
}

#[test]
fn lists_entries_in_deterministic_id_order() {
    let mut directory = InMemoryThingDirectory::new();
    directory
        .register(thing("urn:thing:z", "Z"))
        .expect("registration succeeds");
    directory
        .register(thing("urn:thing:a", "A"))
        .expect("registration succeeds");

    let page = directory.list();
    let ids: Vec<_> = page.entries.into_iter().map(|entry| entry.id).collect();

    assert_eq!(ids, vec!["urn:thing:a", "urn:thing:z"]);
    assert_eq!(page.total, 2);
}

#[test]
fn queries_by_backend_portable_filters() {
    let mut directory = InMemoryThingDirectory::new();
    directory
        .register(thing_with_property("urn:thing:lamp", "Lamp", "status"))
        .expect("registration succeeds");
    directory
        .register(thing("urn:thing:button", "Button"))
        .expect("registration succeeds");

    let by_title = directory.query(DirectoryQuery::title("Lamp"));
    let by_property = directory.query(DirectoryQuery::property("status"));
    let by_closure = directory
        .query_predicate(|thing: &Thing| thing.id.as_ref().unwrap().as_str().ends_with("button"));

    assert_eq!(by_title.total, 1);
    assert_eq!(by_title.entries[0].id, "urn:thing:lamp");
    assert_eq!(by_property.total, 1);
    assert_eq!(by_property.entries[0].id, "urn:thing:lamp");
    assert_eq!(by_closure.len(), 1);
    assert_eq!(by_closure[0].id, "urn:thing:button");
}

#[test]
fn queries_with_conjunctive_filters_and_pagination() {
    let mut directory = InMemoryThingDirectory::new();
    directory
        .register(thing_with_property("urn:thing:a", "Lamp", "status"))
        .expect("registration succeeds");
    directory
        .register(thing_with_property("urn:thing:b", "Lamp", "status"))
        .expect("registration succeeds");
    directory
        .register(thing_with_property("urn:thing:c", "Lamp", "level"))
        .expect("registration succeeds");

    let page = directory.query(
        DirectoryQuery::title("Lamp")
            .and(QueryFilter::property("status"))
            .offset(1)
            .limit(1),
    );

    assert_eq!(page.total, 2);
    assert_eq!(page.offset, 1);
    assert_eq!(page.limit, Some(1));
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].id, "urn:thing:b");
}

#[test]
fn thing_description_can_be_cloned_for_owned_directory_results() {
    let original = thing_with_property("urn:thing:lamp", "Lamp", "status");
    let cloned = original.clone();

    assert_eq!(cloned.id.as_ref().unwrap().as_str(), "urn:thing:lamp");
    assert_eq!(cloned._metadata.title.as_deref(), Some("Lamp"));
    assert!(cloned.properties.as_ref().unwrap().contains_key("status"));
}

#[test]
fn validates_registered_thing_descriptions() {
    let mut directory = InMemoryThingDirectory::new();
    let invalid = Thing::builder("Lamp")
        .id("urn:thing:lamp")
        .security_definition("apikey", SecurityScheme::apikey("header"))
        .security_name("missing")
        .build()
        .unwrap_err();

    assert!(invalid.to_string().contains("Invalid reference"));

    let invalid = Thing {
        id: Some(clinkz_wot_td::data_type::AbsoluteUri::parse("urn:thing:lamp").unwrap()),
        ..Default::default()
    };

    let err = directory
        .register(invalid)
        .expect_err("invalid TD is rejected");

    assert!(matches!(err, DiscoveryError::InvalidThingDescription(_)));
}

#[test]
fn supports_configurable_validation_level() {
    let mut directory = InMemoryThingDirectory::with_validation_level(ValidationLevel::Minimal);
    let minimal = Thing {
        id: Some(clinkz_wot_td::data_type::AbsoluteUri::parse("urn:thing:minimal").unwrap()),
        ..Default::default()
    };

    directory
        .register(minimal)
        .expect("minimal validation accepts serde-shaped TD");

    assert_eq!(directory.validation_level(), ValidationLevel::Minimal);
}

#[test]
fn rejects_registration_without_id() {
    let mut directory = InMemoryThingDirectory::with_validation_level(ValidationLevel::Minimal);

    let err = directory
        .register(Thing::default())
        .expect_err("id is required for directory keys");

    assert_eq!(err, DiscoveryError::MissingThingId);
}

#[cfg(feature = "std")]
#[test]
fn std_storage_handle_shares_directory_backend() {
    let shared =
        clinkz_wot_discovery::storage::SharedThingDirectory::new(InMemoryThingDirectory::new());
    let cloned = shared.clone();

    shared
        .lock()
        .expect("shared directory lock succeeds")
        .register(thing("urn:thing:lamp", "Lamp"))
        .expect("registration succeeds");

    let page = cloned
        .lock()
        .expect("shared directory lock succeeds")
        .list();

    assert_eq!(page.total, 1);
    assert_eq!(page.entries[0].id, "urn:thing:lamp");
}
