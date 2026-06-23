//! Unit tests for the `discover()` Scripting API surface.
//!
//! Covers `ThingFilter`, `ThingDiscovery` iteration, and fragment-based
//! discovery filtering.

use clinkz_wot_discovery::{InMemoryThingDirectory, ThingDirectory, ThingFilter, discover};
use clinkz_wot_td::thing::Thing;
use serde_json::json;

fn thing(id: &str, title: &str) -> Thing {
    Thing::builder(title).id(id).nosec().build().unwrap()
}

fn id_of(t: &Thing) -> String {
    t.id.as_ref().unwrap().as_str().to_string()
}

fn title_of(t: &Thing) -> String {
    t._metadata.title.as_deref().unwrap().to_string()
}

#[test]
fn discover_local_returns_all_things() {
    let mut dir = InMemoryThingDirectory::new();
    dir.register(thing("urn:1", "Sensor")).unwrap();
    dir.register(thing("urn:2", "Lamp")).unwrap();

    let mut td = discover(&dir, ThingFilter::new()).unwrap();
    assert!(!td.is_done());
    assert_eq!(td.remaining(), 2);

    let first = td.next_now().unwrap();
    let second = td.next_now().unwrap();
    assert!(td.next_now().is_none());
    assert!(td.is_done());

    let mut ids = [id_of(&first), id_of(&second)];
    ids.sort();
    assert_eq!(ids, ["urn:1", "urn:2"]);
}

#[test]
fn discover_local_with_query_filters_by_title() {
    let mut dir = InMemoryThingDirectory::new();
    dir.register(thing("urn:1", "Sensor")).unwrap();
    dir.register(thing("urn:2", "Sensor")).unwrap();
    dir.register(thing("urn:3", "Smart Lamp")).unwrap();

    let filter = ThingFilter::new().fragment_field("title", json!("Sensor"));
    let td = discover(&dir, filter).unwrap();
    let titles: Vec<_> = std::iter::from_fn({
        let mut td = td;
        move || td.next_now()
    })
    .map(|t| title_of(&t))
    .collect();
    assert_eq!(titles.len(), 2);
    assert!(titles.iter().all(|t| t == "Sensor"));
}

#[test]
fn discover_local_with_fragment_filters_by_title_fragment() {
    let mut dir = InMemoryThingDirectory::new();
    dir.register(thing("urn:1", "Lamp")).unwrap();
    dir.register(thing("urn:2", "Sensor")).unwrap();

    let filter = ThingFilter::new().fragment_field("title", json!("Lamp"));
    let td = discover(&dir, filter).unwrap();
    let ids: Vec<_> = std::iter::from_fn({
        let mut td = td;
        move || td.next_now()
    })
    .map(|t| id_of(&t))
    .collect();
    assert_eq!(ids, vec!["urn:1"]);
}

#[test]
fn discovery_stop_clears_buffered_results() {
    let mut dir = InMemoryThingDirectory::new();
    dir.register(thing("urn:1", "A")).unwrap();
    dir.register(thing("urn:2", "B")).unwrap();
    dir.register(thing("urn:3", "C")).unwrap();

    let mut td = discover(&dir, ThingFilter::new()).unwrap();
    assert_eq!(td.remaining(), 3);

    td.stop();
    assert!(td.is_done());
    assert_eq!(td.remaining(), 0);
    assert!(td.next_now().is_none());
}

#[test]
fn discovery_filter_ref_returns_the_filter() {
    let dir = InMemoryThingDirectory::new();
    let filter = ThingFilter::new().fragment_field("title", json!("test"));
    let td = discover(&dir, filter).unwrap();
    assert_eq!(
        td.filter_ref()
            .fragment
            .as_ref()
            .unwrap()
            .get("title")
            .and_then(|value| value.as_str()),
        Some("test")
    );
}

#[test]
fn discover_local_with_fragment_filters_by_property_name() {
    use clinkz_wot_td::affordance::{InteractionHelper, PropertyAffordance};
    use clinkz_wot_td::data_schema::DataSchema;
    use clinkz_wot_td::form::Form;

    let mut dir = InMemoryThingDirectory::new();
    // A lamp that exposes the `status` property.
    let lamp = Thing::builder("Lamp")
        .id("urn:lamp")
        .nosec()
        .property(
            "status",
            PropertyAffordance::builder(DataSchema::string())
                .form(Form::read_property("/status").build().unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    dir.register(lamp).unwrap();
    // A sensor that does not expose `status`.
    dir.register(thing("urn:sensor", "Sensor")).unwrap();

    let filter = ThingFilter::new().fragment_field("properties", json!({ "status": {} }));
    let td = discover(&dir, filter).unwrap();
    let ids: Vec<_> = std::iter::from_fn({
        let mut td = td;
        move || td.next_now()
    })
    .map(|t| id_of(&t))
    .collect();
    assert_eq!(ids, vec!["urn:lamp"]);
}
