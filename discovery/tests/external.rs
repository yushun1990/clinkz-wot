//! Manual interoperability against an independently deployed, seeded TDD.
//! CLINKZ_TDD_URL=http://127.0.0.1:PORT/things
//! CLINKZ_TDD_SOCKET=127.0.0.1:PORT
//! CLINKZ_TDD_IDS=urn:interop:a,urn:interop:b,urn:interop:c
//! Optional CLINKZ_TDD_ROOT_DER selects HTTPS with explicit roots.
#![cfg(feature = "http")]
use clinkz_wot_discovery::http::*;
#[tokio::test]
#[ignore = "requires an independently deployed and seeded Directory; see remote-discovery evidence"]
async fn independent_directory_listing_and_individual_td_retrieval() {
    let url = std::env::var("CLINKZ_TDD_URL").unwrap();
    let socket = std::env::var("CLINKZ_TDD_SOCKET").unwrap().parse().unwrap();
    let ids = std::env::var("CLINKZ_TDD_IDS")
        .unwrap()
        .split(',')
        .map(str::to_string)
        .collect::<Vec<_>>();
    let transport = if let Ok(path) = std::env::var("CLINKZ_TDD_ROOT_DER") {
        DiscoveryTransport::tls(vec![std::fs::read(path).unwrap().into()]).unwrap()
    } else {
        DiscoveryTransport::cleartext_loopback()
    };
    let endpoint = TrustedDiscoveryEndpoint::new(&url, socket, transport.clone()).unwrap();
    let mut session =
        HttpDiscoverySession::directory(endpoint, None, DiscoveryLimits::default()).unwrap();
    let mut actual = Vec::new();
    let cancel = DiscoveryCancellation::default();
    while let Some(page) = session.next_page(&cancel).await.unwrap() {
        actual.extend(
            page.into_iter()
                .map(|t| t.thing().id.as_ref().unwrap().as_str().to_string()),
        );
    }
    assert_eq!(actual, ids);
    assert_eq!(session.error(), None);
    drop(session);
    for id in ids {
        // These are independently configured fixture identifiers, not links or
        // endpoint authority inferred from the returned TDs.
        let endpoint =
            TrustedDiscoveryEndpoint::new(&format!("{url}/{id}"), socket, transport.clone())
                .unwrap();
        let mut session =
            HttpDiscoverySession::td(endpoint, None, DiscoveryLimits::default()).unwrap();
        let page = session.next_page(&cancel).await.unwrap().unwrap();
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].thing().id.as_ref().unwrap().as_str(), id);
        assert_eq!(session.error(), None);
    }
}
