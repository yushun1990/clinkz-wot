#![cfg(feature = "http")]
mod support;
use clinkz_wot_discovery::{DiscoveryError as E, http::*};
use serde_json::json;
use std::time::Duration;
use support::*;

#[tokio::test]
async fn listing_retains_every_td_and_follows_base_relative_next_without_assuming_full_pages() {
    let first = json!([
        td("urn:a", "http://127.0.0.1:9/never"),
        td("urn:b", "http://127.0.0.1:9/never")
    ])
    .to_string();
    let last = json!([td("urn:c", "http://127.0.0.1:9/never")]).to_string();
    let peer = Peer::start(vec![
        Step::new("/registry/things?limit=2", reply(&first,"application/ld+json", "Link: <things?offset=2&limit=2&session=a%2Cb>; rel=next; title=\"a,b;c\", <things>; rel=canonical; etag=\"rev,1\"\r\n")),
        Step::new("/registry/things?offset=2&limit=2&session=a%2Cb", reply(&last,"application/json; charset=\"UTF-8\"", "Link: <things>; rel=\"canonical\"; etag=\"rev,1\"\r\n"))], None).await;
    let mut session = HttpDiscoverySession::directory(
        peer.endpoint("/registry/things", DiscoveryTransport::cleartext_loopback()),
        None,
        limits(),
    )
    .unwrap();
    let cancel = DiscoveryCancellation::default();
    let mut ids = Vec::new();
    while let Some(page) = session.next_page(&cancel).await.unwrap() {
        for thing in page {
            ids.push(thing.thing().id.as_ref().unwrap().as_str().to_string());
        }
    }
    assert_eq!(ids, ["urn:a", "urn:b", "urn:c"]);
    assert_eq!(session.error(), None);
    peer.finish(2).await;
}

#[tokio::test]
async fn single_td_supports_anonymous_descriptions_and_chunked_framing() {
    let mut value = td("urn:a", "http://127.0.0.1:9/no");
    value.as_object_mut().unwrap().remove("id");
    let body = value.to_string();
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/td+json\r\nTransfer-Encoding: chunked\r\n\r\n{:x};test=yes\r\n{}\r\n0\r\n\r\n",
        body.len(),
        body
    );
    let peer = Peer::start(vec![Step::new("/td", response)], None).await;
    let mut session = HttpDiscoverySession::td(
        peer.endpoint("/td", DiscoveryTransport::cleartext_loopback()),
        None,
        limits(),
    )
    .unwrap();
    let page = session
        .next_page(&DiscoveryCancellation::default())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(page.len(), 1);
    assert!(page[0].thing().id.is_none());
    assert_eq!(session.error(), None);
    peer.finish(1).await;
}

async fn rejection(response: String, expected: E) {
    let diagnostic = response.clone();
    let peer = Peer::start(vec![Step::new("/things?limit=2", response)], None).await;
    let mut session = HttpDiscoverySession::directory(
        peer.endpoint("/things", DiscoveryTransport::cleartext_loopback()),
        None,
        limits(),
    )
    .unwrap();
    assert_eq!(
        session
            .next_page(&DiscoveryCancellation::default())
            .await
            .expect_err(&diagnostic),
        expected
    );
    assert_eq!(session.error(), Some(expected));
    assert!(
        session
            .next_page(&DiscoveryCancellation::default())
            .await
            .unwrap()
            .is_none()
    );
    session.stop();
    assert_eq!(session.error(), Some(expected));
    peer.finish(1).await;
}
#[tokio::test]
async fn malformed_or_ambiguous_description_refuses_whole_page() {
    let good = td("urn:a", "http://127.0.0.1:9/no");
    for invalid in [json!({}), json!({"title":"missing security"}), json!(17)] {
        rejection(
            reply(&json!([good, invalid]).to_string(), "application/json", ""),
            E::InvalidThingDescription,
        )
        .await;
    }
    let bad_op = td("urn:b","http://127.0.0.1:9/no").to_string().replace("\"readOnly\":true", "\"readOnly\":true,\"forms\":[{\"href\":\"http://127.0.0.1/no\",\"op\":\"subscribeevent\"}]");
    rejection(
        reply(&format!("[{bad_op}]"), "application/json", ""),
        E::InvalidJson,
    )
    .await;
    rejection(
        reply(
            &format!("[{good},{{\"extension\":1,\"extensio\\u006e\":2}}]"),
            "application/json",
            "",
        ),
        E::InvalidJson,
    )
    .await;
    rejection(reply("[{}", "application/json", ""), E::InvalidJson).await;
    rejection(
        reply("{\"members\":[]}", "application/ld+json", ""),
        E::InvalidJson,
    )
    .await;
}
#[tokio::test]
async fn unsupported_media_status_and_framing_fail_closed() {
    for media in [
        "text/html",
        "application/td+json",
        "application/json; charset=latin1",
        "application/json; profile=x",
    ] {
        rejection(reply("[]", media, ""), E::UnsupportedMedia).await;
    }
    rejection(
        reply(
            "[]",
            "application/json",
            "Content-Type: application/json\r\n",
        ),
        E::UnsupportedMedia,
    )
    .await;
    rejection(
        reply("[]", "application/json", "Content-Encoding: gzip\r\n"),
        E::UnsupportedMedia,
    )
    .await;
    for headers in [
        "Content-Length: 2\r\n",
        "Transfer-Encoding: chunked\r\n",
        "Trailer: Link\r\n",
    ] {
        rejection(reply("[]", "application/json", headers), E::Protocol).await;
    }
    for (status, expected) in [
        (401, E::Authentication),
        (403, E::Forbidden),
        (302, E::HttpStatus(302)),
        (503, E::HttpStatus(503)),
    ] {
        rejection(format!("HTTP/1.1 {status} Refused\r\nLocation: http://127.0.0.1:9/leak\r\nWWW-Authenticate: Basic realm=\"peer\"\r\nContent-Length: 0\r\n\r\n"),expected).await;
    }
}
#[tokio::test]
async fn hostile_continuations_never_acquire_a_second_destination() {
    let body = json!([td("urn:a", "http://127.0.0.1:9/no")]).to_string();
    let canonical = "Link: </things>; rel=canonical; etag=one\r\n";
    for (next, expected) in [
        (
            "http://127.0.0.1:9/things?offset=1&limit=2",
            E::DestinationDenied,
        ),
        (
            "//127.0.0.1:9/things?offset=1&limit=2",
            E::DestinationDenied,
        ),
        ("/private?offset=1&limit=2", E::DestinationDenied),
        (
            "https://user:password@example.com/things?offset=1&limit=2",
            E::InvalidContinuation,
        ),
        ("/things?offset=1&limit=2#fragment", E::InvalidContinuation),
        ("/things?offset=0&limit=2", E::InvalidContinuation),
        ("/things?offset=1&limit=3", E::InvalidContinuation),
        ("/things?offset=1&limit=2&limit=2", E::InvalidContinuation),
        ("/things?offset=1&limit=2&l%69mit=2", E::InvalidContinuation),
        (
            "/things?offset=1&limit=2&format=collection",
            E::InvalidContinuation,
        ),
    ] {
        rejection(
            reply(
                &body,
                "application/json",
                &format!("{canonical}Link: <{next}>; rel=next\r\n"),
            ),
            expected,
        )
        .await;
    }
    for extra in [
        "Link: </things?offset=1&limit=2>; rel=next; anchor=\"/other\"\r\n",
        "Link: </things?offset=1&limit=2>; rel=next; rel=next\r\n",
        "Link: </things?offset=1&limit=2>; rel=next, </things?offset=1&limit=2>; rel=next\r\n",
    ] {
        rejection(
            reply(&body, "application/json", &format!("{canonical}{extra}")),
            E::InvalidContinuation,
        )
        .await;
    }
    rejection(
        reply(
            &body,
            "application/json",
            "Link: </things?offset=1&limit=2>; rel=next\r\n",
        ),
        E::InvalidContinuation,
    )
    .await;
}
#[tokio::test]
async fn changed_collection_and_cross_page_duplicate_ids_are_terminal() {
    for (last, etag, expected) in [
        ("urn:b", "two", E::CollectionChanged),
        ("urn:a", "one", E::InvalidContinuation),
    ] {
        let peer = Peer::start(vec![
            Step::new("/things?limit=2",reply(&json!([td("urn:a","http://127.0.0.1:9/no")]).to_string(),"application/json","Link: </things?offset=1&limit=2>; rel=next\r\nLink: </things>; rel=canonical; etag=one\r\n")),
            Step::new("/things?offset=1&limit=2",reply(&json!([td(last,"http://127.0.0.1:9/no")]).to_string(),"application/json",&format!("Link: </things>; rel=canonical; etag={etag}\r\n")))],None).await;
        let mut session = HttpDiscoverySession::directory(
            peer.endpoint("/things", DiscoveryTransport::cleartext_loopback()),
            None,
            limits(),
        )
        .unwrap();
        assert_eq!(
            session
                .next_page(&DiscoveryCancellation::default())
                .await
                .unwrap()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            session
                .next_page(&DiscoveryCancellation::default())
                .await
                .unwrap_err(),
            expected
        );
        peer.finish(2).await;
    }
}
#[tokio::test]
async fn limits_refuse_excess_results_bytes_work_and_continuations() {
    for (response, limits) in [
        (
            reply(
                &json!([
                    td("urn:a", "http://127.0.0.1:9/no"),
                    td("urn:b", "http://127.0.0.1:9/no")
                ])
                .to_string(),
                "application/json",
                "",
            ),
            DiscoveryLimits {
                max_results: 1,
                ..limits()
            },
        ),
        (
            reply(
                &json!([td("urn:a", "http://127.0.0.1:9/no")]).to_string(),
                "application/json",
                "",
            ),
            DiscoveryLimits {
                td_bytes: 10,
                ..limits()
            },
        ),
        (
            reply("[]", "application/json", ""),
            DiscoveryLimits {
                page_bytes: 1,
                td_bytes: 1,
                ..limits()
            },
        ),
        (
            reply(
                &json!([td("urn:a", "http://127.0.0.1:9/no")]).to_string(),
                "application/json",
                "Link: </things?offset=1&limit=2>; rel=next\r\nLink: </things>; rel=canonical; etag=one\r\n",
            ),
            DiscoveryLimits {
                max_pages: 1,
                ..limits()
            },
        ),
    ] {
        let peer = Peer::start(vec![Step::new("/things?limit=2", response)], None).await;
        let mut session = HttpDiscoverySession::directory(
            peer.endpoint("/things", DiscoveryTransport::cleartext_loopback()),
            None,
            limits,
        )
        .unwrap();
        assert_eq!(
            session
                .next_page(&DiscoveryCancellation::default())
                .await
                .unwrap_err(),
            E::Limit
        );
        peer.finish(1).await;
    }
    rejection(
        reply(
            &format!("{}0{}", "[".repeat(34), "]".repeat(34)),
            "application/json",
            "",
        ),
        E::InvalidJson,
    )
    .await;
}
#[tokio::test]
async fn cancellation_timeout_and_future_abandonment_release_partial_socket_without_retry() {
    for mode in 0..3 {
        let partial =
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 999\r\n\r\n[{}";
        let peer = Peer::start(vec![Step::new("/things?limit=2", partial.into())], None).await;
        let mut settings = limits();
        if mode == 1 {
            settings.request_timeout = Duration::from_millis(30);
        }
        let mut session = HttpDiscoverySession::directory(
            peer.endpoint("/things", DiscoveryTransport::cleartext_loopback()),
            None,
            settings,
        )
        .unwrap();
        let cancel = DiscoveryCancellation::default();
        {
            let future = session.next_page(&cancel);
            tokio::pin!(future);
            tokio::select! { _ = &mut future => panic!("partial page completed"), _ = peer.wait_request() => {} }
            if mode == 0 {
                cancel.cancel();
                assert_eq!(future.await.unwrap_err(), E::Cancelled);
            } else if mode == 1 {
                assert_eq!(future.await.unwrap_err(), E::Timeout);
            }
            // mode 2 drops the pending future with no explicit cancellation.
        }
        assert_eq!(
            session.error(),
            Some([E::Cancelled, E::Timeout, E::Interrupted][mode])
        );
        assert!(
            session
                .next_page(&DiscoveryCancellation::default())
                .await
                .unwrap()
                .is_none()
        );
        session.stop();
        peer.finish(1).await;
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let endpoint = TrustedDiscoveryEndpoint::new(
        &format!("http://{addr}/td"),
        addr,
        DiscoveryTransport::cleartext_loopback(),
    )
    .unwrap();
    let mut session = HttpDiscoverySession::td(endpoint, None, limits()).unwrap();
    let cancel = DiscoveryCancellation::default();
    cancel.cancel();
    assert_eq!(session.next_page(&cancel).await.unwrap_err(), E::Cancelled);
    assert!(
        tokio::time::timeout(Duration::from_millis(20), listener.accept())
            .await
            .is_err()
    );
}
#[tokio::test]
async fn truncated_connection_refuses_all_even_after_a_complete_first_td() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut s, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let mut b = [0];
            s.read_exact(&mut b).await.unwrap();
            request.push(b[0]);
            assert!(request.len() < 8192);
        }
        s.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 900\r\n\r\n[{}",
        )
        .await
        .unwrap();
    });
    let endpoint = TrustedDiscoveryEndpoint::new(
        &format!("http://{addr}/things"),
        addr,
        DiscoveryTransport::cleartext_loopback(),
    )
    .unwrap();
    let mut session = HttpDiscoverySession::directory(endpoint, None, limits()).unwrap();
    assert_eq!(
        session
            .next_page(&DiscoveryCancellation::default())
            .await
            .unwrap_err(),
        E::Protocol
    );
    server.await.unwrap();
}

#[tokio::test]
async fn semantic_validation_refuses_wrong_operation_and_undefined_security() {
    let mut wrong = td("urn:a", "http://127.0.0.1:9/no");
    wrong["properties"]["level"]["forms"][0]["op"] = json!("subscribeevent");
    rejection(
        reply(&json!([wrong]).to_string(), "application/json", ""),
        E::InvalidThingDescription,
    )
    .await;
    let mut undefined = td("urn:a", "http://127.0.0.1:9/no");
    undefined["security"] = json!("undefined");
    rejection(
        reply(&json!([undefined]).to_string(), "application/json", ""),
        E::InvalidThingDescription,
    )
    .await;
    rejection(reply("[]", "application/json", "Link: </things?offset=0&limit=2>; rel=next\r\nLink: </things>; rel=canonical; etag=one\r\n"), E::InvalidContinuation).await;
}
#[tokio::test]
async fn cancellation_between_pages_and_total_input_budget_do_not_publish_partial_success() {
    for cancelled in [true, false] {
        let body = json!([td("urn:a", "http://127.0.0.1:9/no")]).to_string();
        let mut steps = vec![Step::new(
            "/things?limit=2",
            reply(
                &body,
                "application/json",
                "Link: </things?offset=1&limit=2>; rel=next\r\nLink: </things>; rel=canonical; etag=one\r\n",
            ),
        )];
        if !cancelled {
            steps.push(Step::new(
                "/things?offset=1&limit=2",
                reply(
                    &body.replace("urn:a", "urn:b"),
                    "application/json",
                    "Link: </things>; rel=canonical; etag=one\r\n",
                ),
            ));
        }
        let peer = Peer::start(steps, None).await;
        let mut session = HttpDiscoverySession::directory(
            peer.endpoint("/things", DiscoveryTransport::cleartext_loopback()),
            None,
            DiscoveryLimits {
                page_bytes: 300,
                td_bytes: 300,
                total_bytes: 400,
                ..limits()
            },
        )
        .unwrap();
        let cancel = DiscoveryCancellation::default();
        let accepted = session.next_page(&cancel).await.unwrap().unwrap();
        if cancelled {
            cancel.cancel();
        }
        assert_eq!(
            session.next_page(&cancel).await.unwrap_err(),
            if cancelled { E::Cancelled } else { E::Limit }
        );
        assert_eq!(accepted[0].thing().id.as_ref().unwrap().as_str(), "urn:a");
        peer.finish(if cancelled { 1 } else { 2 }).await;
    }
}
#[tokio::test]
async fn session_deadline_and_tls_handshake_timeout_bound_idle_native_work() {
    use tokio::io::AsyncReadExt;
    let keys = Keys::new("127.0.0.1");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut scratch = [0; 4096];
        assert!(socket.read(&mut scratch).await.unwrap() > 0); // TLS ClientHello
        loop {
            match socket.read(&mut scratch).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
    });
    let endpoint =
        TrustedDiscoveryEndpoint::new(&format!("https://{addr}/td"), addr, keys.transport())
            .unwrap();
    let mut session = HttpDiscoverySession::td(
        endpoint,
        None,
        DiscoveryLimits {
            request_timeout: Duration::from_millis(30),
            ..limits()
        },
    )
    .unwrap();
    assert_eq!(
        session
            .next_page(&DiscoveryCancellation::default())
            .await
            .unwrap_err(),
        E::Timeout
    );
    tokio::time::timeout(Duration::from_secs(1), server)
        .await
        .unwrap()
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let endpoint = TrustedDiscoveryEndpoint::new(
        &format!("http://{addr}/td"),
        addr,
        DiscoveryTransport::cleartext_loopback(),
    )
    .unwrap();
    let mut session = HttpDiscoverySession::td(
        endpoint,
        None,
        DiscoveryLimits {
            session_timeout: Duration::from_millis(5),
            ..limits()
        },
    )
    .unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert_eq!(
        session
            .next_page(&DiscoveryCancellation::default())
            .await
            .unwrap_err(),
        E::Timeout
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(10), listener.accept())
            .await
            .is_err()
    );
}
