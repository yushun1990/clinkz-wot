//! P1 verification: filter→batch→continuation, get/open_search, publisher
//! CRUD, projection modes, count modes, Live monotonicity (no re-emit),
//! ThingDiscoveryProcess laziness.

#![cfg(feature = "async")]

use std::string::ToString;

use clinkz_wot_core::ThingId;
use clinkz_wot_td::{affordance::PropertyAffordance, data_schema::DataSchema, thing::Thing};

use clinkz_wot_discovery::{
    CapabilityFilter, CountMode, DirectoryFilter, DirectoryItem, DirectoryPatch,
    DirectoryPublisher, DirectoryQuery, DirectoryReader, DirectoryRegistration, Discoverer,
    DiscoveryFilter, InMemoryDirectory, LocalDiscoverer, ProjectionMode, ThingDiscoveryProcess,
    ThingFragment,
};

use clinkz_wot_core::MediaType;
use std::sync::Arc;

fn thing(id: &str, title: &str) -> Thing {
    Thing::builder(title)
        .id(id)
        .nosec()
        .build()
        .expect("valid TD")
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
        .expect("valid TD")
}

#[tokio::test]
async fn reader_get_and_open_search_full_projection() {
    let dir = InMemoryDirectory::new();
    dir.register(DirectoryRegistration {
        td: thing_with_property("urn:t:a", "Alpha", "status"),
        ttl: None,
    })
    .await
    .unwrap();
    dir.register(DirectoryRegistration {
        td: thing_with_property("urn:t:b", "Beta", "level"),
        ttl: None,
    })
    .await
    .unwrap();

    // get
    let got = dir.get(&ThingId::from("urn:t:a")).await.unwrap();
    assert!(got.is_some());

    // open_search yields both full Things in sorted-id order
    let mut s = dir.open_search(DirectoryQuery::all()).await.unwrap();
    let batch = s.next().await.unwrap().unwrap();
    assert_eq!(batch.items.len(), 2);
    assert!(matches!(batch.items[0], DirectoryItem::Full(_)));
    assert!(!batch.stats.has_more);
}

#[tokio::test]
async fn filter_by_example_property() {
    let dir = InMemoryDirectory::new();
    dir.register(DirectoryRegistration {
        td: thing_with_property("urn:t:a", "Alpha", "status"),
        ttl: None,
    })
    .await
    .unwrap();
    dir.register(DirectoryRegistration {
        td: thing_with_property("urn:t:b", "Beta", "level"),
        ttl: None,
    })
    .await
    .unwrap();

    let q = DirectoryQuery {
        filter: DirectoryFilter::ByExample(ThingFragment {
            properties: vec!["status".to_string()],
            ..Default::default()
        }),
        projection: ProjectionMode::IdOnly,
        ..DirectoryQuery::all()
    };
    let mut s = dir.open_search(q).await.unwrap();
    let batch = s.next().await.unwrap().unwrap();
    assert_eq!(batch.items.len(), 1);
    assert!(matches!(batch.items[0], DirectoryItem::Id(_)));
}

#[tokio::test]
async fn live_session_does_not_reemit_on_update() {
    // Live Semantics rule 4: an updated item whose id was already emitted is
    // NOT re-emitted in the same session.
    let dir = InMemoryDirectory::new();
    dir.register(DirectoryRegistration {
        td: thing_with_property("urn:t:a", "Alpha", "status"),
        ttl: None,
    })
    .await
    .unwrap();

    let mut s = dir.open_search(DirectoryQuery::all()).await.unwrap();
    let batch1 = s.next().await.unwrap().unwrap();
    assert_eq!(batch1.items.len(), 1);

    // Update the already-emitted Thing (new revision).
    dir.update(
        &ThingId::from("urn:t:a"),
        DirectoryPatch {
            body: br#"{"title":"Alpha-v2"}"#.to_vec(),
            content_type: MediaType::from("application/json"),
        },
    )
    .await
    .unwrap();

    // Next batch: no re-emit of urn:t:a (cursor past it).
    let batch2 = s.next().await.unwrap();
    assert!(batch2.is_none() || batch2.unwrap().items.is_empty());
}

#[tokio::test]
async fn live_session_mid_drain_insert_appears() {
    // Live Semantics rule 3 (optional): an insert during a multi-batch drain,
    // with id > cursor, appears in a later batch of the same session.
    let dir = InMemoryDirectory::new();
    dir.register(DirectoryRegistration {
        td: thing("urn:t:a", "Alpha"),
        ttl: None,
    })
    .await
    .unwrap();
    dir.register(DirectoryRegistration {
        td: thing("urn:t:b", "Beta"),
        ttl: None,
    })
    .await
    .unwrap();

    let q = DirectoryQuery {
        page_size: 1,
        ..DirectoryQuery::all()
    };
    let mut s = dir.open_search(q).await.unwrap();
    let batch1 = s.next().await.unwrap().unwrap();
    assert_eq!(batch1.items.len(), 1); // emits "urn:t:a", cursor=a, session still open (b > a)

    // Insert "urn:t:c" mid-drain (c > a, c > b).
    dir.register(DirectoryRegistration {
        td: thing("urn:t:c", "Gamma"),
        ttl: None,
    })
    .await
    .unwrap();

    let batch2 = s.next().await.unwrap().unwrap();
    assert_eq!(batch2.items.len(), 1); // emits "urn:t:b"
    let batch3 = s.next().await.unwrap().unwrap();
    assert_eq!(batch3.items.len(), 1); // emits "urn:t:c" (mid-drain insert, id > cursor)
    assert!(s.next().await.unwrap().is_none()); // session drained
}

#[tokio::test]
async fn count_modes() {
    let dir = InMemoryDirectory::new();
    for n in 0..5 {
        dir.register(DirectoryRegistration {
            td: thing(&format!("urn:t:{n}"), "T"),
            ttl: None,
        })
        .await
        .unwrap();
    }
    let q = DirectoryQuery {
        count_mode: CountMode::Exact,
        ..DirectoryQuery::all()
    };
    let mut s = dir.open_search(q).await.unwrap();
    let batch = s.next().await.unwrap().unwrap();
    assert_eq!(
        batch.stats.count,
        Some(clinkz_wot_discovery::CountValue::Exact(5))
    );
}

#[tokio::test]
async fn publisher_register_unregister() {
    let dir = InMemoryDirectory::new();
    let ack = dir
        .register(DirectoryRegistration {
            td: thing("urn:t:1", "One"),
            ttl: Some(core::time::Duration::from_secs(60)),
        })
        .await
        .unwrap();
    assert_eq!(ack.id, ThingId::from("urn:t:1"));
    assert!(ack.lease.is_some());

    dir.unregister(&ThingId::from("urn:t:1")).await.unwrap();
    assert!(dir.get(&ThingId::from("urn:t:1")).await.unwrap().is_none());
}

#[tokio::test]
async fn thing_discovery_process_is_lazy_and_drains() {
    let dir = Arc::new(InMemoryDirectory::new());
    dir.register(DirectoryRegistration {
        td: thing_with_property("urn:t:a", "Alpha", "status"),
        ttl: None,
    })
    .await
    .unwrap();

    let discoverer = LocalDiscoverer::new(dir.clone() as Arc<dyn DirectoryReader>);
    // discover() is sync; the process is in Pending state (no session opened).
    let mut process: ThingDiscoveryProcess = discoverer.discover(DiscoveryFilter::all()).unwrap();

    // First next() opens the session and yields the Thing.
    let t = process.next().await.unwrap().unwrap();
    assert_eq!(
        t.id.as_ref().map(|u| u.as_str().to_string()),
        Some("urn:t:a".to_string())
    );
    // Drain complete.
    assert_eq!(process.next().await.unwrap(), None);
}

#[tokio::test]
async fn capability_filter_smoke() {
    let dir = InMemoryDirectory::new();
    dir.register(DirectoryRegistration {
        td: thing_with_property("urn:t:a", "Alpha", "status"),
        ttl: None,
    })
    .await
    .unwrap();

    let q = DirectoryQuery {
        filter: DirectoryFilter::Capability(CapabilityFilter {
            affordance: Some("status".to_string()),
            ..Default::default()
        }),
        projection: ProjectionMode::Summary,
        ..DirectoryQuery::all()
    };
    let mut s = dir.open_search(q).await.unwrap();
    let batch = s.next().await.unwrap().unwrap();
    assert!(matches!(batch.items[0], DirectoryItem::Summary { .. }));
}

// Re-exports needed in tests.
mod _exports {}
