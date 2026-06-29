//! Unit tests for the `discover()` Scripting API surface.
//!
//! Covers `ThingFilter`, `ThingDiscovery` iteration, and fragment-based
//! discovery filtering.

use clinkz_wot_discovery::{
    DirectoryEntry, DirectoryPage, DirectoryQuery, DiscoveryMethod, InMemoryThingDirectory,
    ThingDirectory, ThingFilter, discover,
};
use clinkz_wot_td::thing::Thing;
use core::sync::atomic::{AtomicUsize, Ordering};
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
fn discover_pushes_indexable_fragment_filters_into_directory_query() {
    struct SpyDirectory {
        thing: Thing,
        query_calls: AtomicUsize,
        for_each_calls: AtomicUsize,
    }

    impl ThingDirectory for SpyDirectory {
        fn register(&mut self, _thing: Thing) -> clinkz_wot_discovery::DiscoveryResult<DirectoryEntry> {
            unreachable!("register is not used in this test")
        }

        fn update(&mut self, _thing: Thing) -> clinkz_wot_discovery::DiscoveryResult<DirectoryEntry> {
            unreachable!("update is not used in this test")
        }

        fn delete(&mut self, _id: &str) -> clinkz_wot_discovery::DiscoveryResult<Thing> {
            unreachable!("delete is not used in this test")
        }

        fn get(&self, _id: &str) -> clinkz_wot_discovery::DiscoveryResult<Thing> {
            unreachable!("get is not used in this test")
        }

        fn list(&self) -> DirectoryPage {
            unreachable!("list is not used in this test")
        }

        fn query(&self, query: DirectoryQuery) -> DirectoryPage {
            self.query_calls.fetch_add(1, Ordering::Relaxed);
            assert_eq!(query, DirectoryQuery::title("Lamp"));
            DirectoryPage {
                entries: vec![DirectoryEntry {
                    id: "urn:lamp".to_string(),
                    thing: self.thing.clone(),
                }],
                total: 1,
                offset: 0,
                limit: None,
            }
        }

        fn for_each_thing(&self, _f: impl FnMut(&Thing)) {
            self.for_each_calls.fetch_add(1, Ordering::Relaxed);
        }
    }

    let dir = SpyDirectory {
        thing: thing("urn:lamp", "Lamp"),
        query_calls: AtomicUsize::new(0),
        for_each_calls: AtomicUsize::new(0),
    };

    let td = discover(&dir, ThingFilter::new().fragment_field("title", json!("Lamp"))).unwrap();

    assert_eq!(td.remaining(), 1);
    assert_eq!(dir.query_calls.load(Ordering::Relaxed), 1);
    assert_eq!(dir.for_each_calls.load(Ordering::Relaxed), 0);
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

// ---------------------------------------------------------------------------
// DiscoveryMethod / query / url (WoT Scripting API alignment)
// ---------------------------------------------------------------------------

#[test]
fn discover_directory_method_returns_error_without_transport() {
    let dir = InMemoryThingDirectory::new();
    let filter = ThingFilter::new()
        .method(DiscoveryMethod::Directory)
        .url("https://tdd.example.com");

    let td = discover(&dir, filter).unwrap();
    assert!(td.is_done());
    assert!(td.error().is_some());
    assert_eq!(td.remaining(), 0);
}

#[test]
fn discover_multicast_method_returns_error_without_transport() {
    let dir = InMemoryThingDirectory::new();
    let filter = ThingFilter::new().method(DiscoveryMethod::Multicast);

    let td = discover(&dir, filter).unwrap();
    assert!(td.is_done());
    assert!(td.error().is_some());
}

#[test]
fn discover_everything_method_searches_local() {
    let mut dir = InMemoryThingDirectory::new();
    dir.register(thing("urn:1", "A")).unwrap();
    dir.register(thing("urn:2", "B")).unwrap();

    let filter = ThingFilter::new().method(DiscoveryMethod::Everything);
    let td = discover(&dir, filter).unwrap();
    assert_eq!(td.remaining(), 2);
    assert!(td.error().is_none());
}

#[test]
fn discover_query_filter_matches_serialized_td() {
    let mut dir = InMemoryThingDirectory::new();
    dir.register(thing("urn:1", "Temperature Sensor")).unwrap();
    dir.register(thing("urn:2", "Pressure Gauge")).unwrap();

    let filter = ThingFilter::new().query("Temperature");
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
fn discover_query_and_fragment_combine_with_and() {
    let mut dir = InMemoryThingDirectory::new();
    dir.register(thing("urn:1", "Temperature Sensor")).unwrap();
    dir.register(thing("urn:2", "Temperature Probe")).unwrap();

    // Both fragment (title) and query must match.
    let filter = ThingFilter::new()
        .fragment_field("title", json!("Temperature Sensor"))
        .query("Sensor");
    let td = discover(&dir, filter).unwrap();
    let ids: Vec<_> = std::iter::from_fn({
        let mut td = td;
        move || td.next_now()
    })
    .map(|t| id_of(&t))
    .collect();
    assert_eq!(ids, vec!["urn:1"]);
}
