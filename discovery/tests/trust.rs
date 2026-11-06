#![cfg(feature = "http")]
mod support;
use clinkz_wot_discovery::{DiscoveryError as E, http::*};
use serde_json::json;
use support::*;

#[tokio::test]
async fn verified_tls_and_preprovisioned_basic_have_separate_retrieval_audience() {
    let keys = Keys::new("localhost");
    let body = json!([td("urn:a", "http://127.0.0.1:9/untrusted-thing")]).to_string();
    let peer = Peer::start(
        vec![Step::new(
            "/things?limit=2",
            reply(&body, "application/ld+json", ""),
        )],
        Some(&keys),
    )
    .await;
    // DNS identity is verified, but the explicitly pinned socket is used. No
    // DNS resolution (and therefore no rebinding) occurs during exploration.
    let endpoint = TrustedDiscoveryEndpoint::new(
        &format!("https://localhost:{}/things", peer.addr.port()),
        peer.addr,
        keys.transport(),
    )
    .unwrap();
    let credentials =
        DiscoveryCredentials::basic(&endpoint, b"directory-user", b"directory-secret").unwrap();
    assert!(!format!("{credentials:?} {endpoint:?}").contains("secret"));
    let mut session =
        HttpDiscoverySession::directory(endpoint, Some(credentials), limits()).unwrap();
    assert_eq!(
        session
            .next_page(&DiscoveryCancellation::default())
            .await
            .unwrap()
            .unwrap()
            .len(),
        1
    );
    {
        let requests = peer.requests.lock().unwrap();
        assert!(
            requests[0]
                .contains("authorization: Basic ZGlyZWN0b3J5LXVzZXI6ZGlyZWN0b3J5LXNlY3JldA==\r\n")
        );
    }
    peer.finish(1).await;
}
#[tokio::test]
async fn missing_wrong_and_forbidden_credentials_are_explicit_without_challenge_replay() {
    for (credentials, status, expected) in [
        (false, 401, E::Authentication),
        (true, 401, E::Authentication),
        (true, 403, E::Forbidden),
    ] {
        let keys = Keys::new("127.0.0.1");
        let peer=Peer::start(vec![Step::new("/td",format!("HTTP/1.1 {status} Refused\r\nWWW-Authenticate: Basic realm=\"directory\"\r\nContent-Length: 0\r\n\r\n"))],Some(&keys)).await;
        let endpoint = peer.endpoint("/td", keys.transport());
        let credentials =
            credentials.then(|| DiscoveryCredentials::basic(&endpoint, b"user", b"wrong").unwrap());
        let mut session = HttpDiscoverySession::td(endpoint, credentials, limits()).unwrap();
        assert_eq!(
            session
                .next_page(&DiscoveryCancellation::default())
                .await
                .unwrap_err(),
            expected
        );
        peer.finish(1).await;
    }
}
#[tokio::test]
async fn bad_roots_hostname_or_expiry_never_release_http_credentials() {
    for mode in 0..3 {
        let keys = if mode == 2 {
            Keys::expired()
        } else {
            Keys::new(if mode == 1 {
                "other.example"
            } else {
                "127.0.0.1"
            })
        };
        let other = Keys::new("127.0.0.1");
        let peer = Peer::start(
            vec![Step::new("/td", reply("{}", "application/td+json", ""))],
            Some(&keys),
        )
        .await;
        let endpoint = peer.endpoint(
            "/td",
            if mode == 0 {
                other.transport()
            } else {
                keys.transport()
            },
        );
        let credentials = DiscoveryCredentials::basic(&endpoint, b"user", b"secret").unwrap();
        let mut session = HttpDiscoverySession::td(endpoint, Some(credentials), limits()).unwrap();
        assert_eq!(
            session
                .next_page(&DiscoveryCancellation::default())
                .await
                .unwrap_err(),
            E::Trust
        );
        assert!(peer.requests.lock().unwrap().is_empty());
        peer.finish(1).await;
    }
}
#[tokio::test]
async fn tls_continuation_cannot_downgrade_or_redirect_credentials_and_audience_cannot_be_reused() {
    let keys = Keys::new("127.0.0.1");
    let body = json!([td("urn:a", "http://127.0.0.1:9/untrusted-thing")]).to_string();
    let peer=Peer::start(vec![Step::new("/things?limit=2",reply(&body,"application/ld+json","Link: <http://127.0.0.1:9/things?offset=1&limit=2>; rel=next\r\nLink: </things>; rel=canonical; etag=one\r\n"))],Some(&keys)).await;
    let endpoint = peer.endpoint("/things", keys.transport());
    let credentials = DiscoveryCredentials::basic(&endpoint, b"user", b"secret").unwrap();
    let other = peer.endpoint("/private", keys.transport());
    assert!(matches!(
        HttpDiscoverySession::td(other, Some(credentials), limits()),
        Err(E::DestinationDenied)
    ));
    let credentials = DiscoveryCredentials::basic(&endpoint, b"user", b"secret").unwrap();
    let mut session =
        HttpDiscoverySession::directory(endpoint, Some(credentials), limits()).unwrap();
    assert_eq!(
        session
            .next_page(&DiscoveryCancellation::default())
            .await
            .unwrap_err(),
        E::DestinationDenied
    );
    peer.finish(1).await;
}
#[tokio::test]
async fn configured_destinations_refuse_userinfo_fragments_cleartext_remote_and_mismatched_pins() {
    let addr = "127.0.0.1:1234".parse().unwrap();
    for url in [
        "http://user:secret@127.0.0.1:1234/td",
        "http://127.0.0.1:1234/td#f",
        "http://127.0.0.2:1234/td",
        "http://127.0.0.1:1235/td",
        "file:///etc/passwd",
        "http://example.org:1234/td",
    ] {
        assert!(matches!(
            TrustedDiscoveryEndpoint::new(url, addr, DiscoveryTransport::cleartext_loopback()),
            Err(E::DestinationDenied)
        ));
    }
    let endpoint = TrustedDiscoveryEndpoint::new(
        "http://127.0.0.1:1234/td",
        addr,
        DiscoveryTransport::cleartext_loopback(),
    )
    .unwrap();
    assert!(matches!(
        DiscoveryCredentials::basic(&endpoint, b"u", b"p"),
        Err(E::Configuration)
    ));
}
