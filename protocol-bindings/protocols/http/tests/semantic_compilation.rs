#![cfg(feature = "prepare")]
//! A downstream crate exercising public production preparation and raw TCP I/O.
#![allow(dead_code)] // Shared application/peer helpers also serve other suites.
include!("support/application.rs");
use serde_json::{Value, json};

fn source(forms: Value) -> Value {
    json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"semantic read", "id":"urn:semantic-read",
        "security":["none"], "securityDefinitions":{"none":{"scheme":"nosec"}},
        "properties":{"a":{"forms":forms}}
    })
}
fn consume_source(value: Value) -> Engine<impl Fn() -> (Option<CleanupOutcome>, [u64; 4])> {
    let policy = policy();
    let parent = Arc::new(
        clinkz_wot_servient::host_accounts::HostResourceAccounts::try_from_accounts(
            policy.parent_accounts(),
        )
        .unwrap(),
    );
    let registration = try_inline_consumer_registration(
        HttpConsumerConfig::for_cleartext_ip().unwrap(),
        BindingId::new(600),
        BindingGeneration::INITIAL,
        600,
    )
    .unwrap();
    let (runtime, observer) = ConsumerPropertyReadRuntime::try_new(
        registration,
        policy,
        Arc::new(Clock(Instant::now())),
        parent.clone(),
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let thing = serde_json::from_value(value).unwrap();
    let handle = match runtime
        .consume(&thing, &mut work(), |build| {
            for _ in 0..300_000 {
                build.step(&mut work());
                if build.is_finished() {
                    return;
                }
            }
            panic!("build stalled");
        })
        .unwrap()
    {
        StartStatus::Ready(handle) => handle,
        _ => panic!("pending publication"),
    };
    drop(thing); // Neither the compiled native artifact nor runtime keeps a TD.
    (runtime, handle, observer, parent)
}
fn close<O>(
    runtime: ConsumerPropertyReadRuntime,
    handle: ConsumerPropertyReadHandle,
    observer: O,
    parent: Arc<clinkz_wot_servient::host_accounts::HostResourceAccounts>,
) {
    drop(handle);
    shutdown(&runtime);
    drop(runtime);
    drop(observer);
    assert!(
        parent
            .snapshot(ConsumerPropertyReadPolicy::PARENT_KINDS)
            .iter()
            .all(|account| account.used() == 0)
    );
}

#[test]
fn compact_vocabularies_keep_their_context_time_meaning_on_the_http_wire() {
    let http = "http://www.w3.org/2011/http#";
    let vendor = "urn:vendor:";
    let cases = [
        (json!({"@vocab":"htv:"}), "POST", "methodName"),
        (json!([{"ex":http}, {"@vocab":"ex:"}]), "POST", "methodName"),
        // @vocab is expanded before this map's term definitions are installed.
        (json!({"@vocab":"htv:", "htv":vendor}), "POST", "methodName"),
        (json!({"htv":vendor, "@vocab":"htv:"}), "POST", "methodName"),
        (
            json!([{"htv":vendor}, {"@vocab":"htv:"}]),
            "GET",
            "methodName",
        ),
        (json!({"@vocab":"ex:", "ex":http}), "GET", "methodName"),
        (
            json!([{"ex":{"@id":http,"@prefix":true}}, {"@vocab":"ex:"}]),
            "POST",
            "methodName",
        ),
        (
            json!([{"@vocab":"htv:","verb":"methodName"}, {"@vocab":vendor}]),
            "POST",
            "verb",
        ),
    ];
    let methods = cases.each_ref().map(|(_, method, _)| *method);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let target = format!("http://{}/a", listener.local_addr().unwrap());
    let peer = std::thread::spawn(move || {
        for method in methods {
            let (mut socket, _) = listener.accept().unwrap();
            let observed = request(&mut socket);
            assert!(
                observed.starts_with(&format!("{method} /a HTTP/1.1\r\n")),
                "{observed}"
            );
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\n\r\n7").unwrap();
            assert_retired(&mut socket);
        }
    });
    recipe().block_on(async {
        for (context, _, field) in cases {
            let mut form = json!({"href":target, "@context":context});
            form[field] = json!("POST");
            let td = source(json!([form]));
            let (runtime, handle, observer, parent) = consume_source(td);
            let mut call = handle
                .read_property("a", InteractionOptions::new(), &mut work())
                .unwrap();
            assert!(take(&mut call).await.is_ok());
            drop(call);
            close(runtime, handle, observer, parent);
        }
    });
    peer.join().unwrap();
}

#[test]
fn proxy_scoped_security_is_refused_and_other_forms_remain_executable() {
    let endpoint = TcpListener::bind("127.0.0.1:0").unwrap();
    let proxy = TcpListener::bind("127.0.0.1:0").unwrap();
    proxy.set_nonblocking(true).unwrap();
    let target = format!("http://{}/a", endpoint.local_addr().unwrap());
    let proxy_uri = format!("http://{}/", proxy.local_addr().unwrap());
    let peer = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut socket, _) = endpoint.accept().unwrap();
            assert!(request(&mut socket).starts_with("GET /a HTTP/1.1\r\n"));
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\n\r\n7").unwrap();
            assert_retired(&mut socket);
        }
    });
    recipe().block_on(async {
        for with_endpoint in [false, true] {
            let mut forms = vec![json!({"href":target, "security":["proxy"]})];
            if with_endpoint {
                forms.push(json!({"href":target}));
            }
            let mut td = source(json!(forms));
            td["securityDefinitions"]["proxy"] = json!({"scheme":"nosec", "proxy":proxy_uri});
            if !with_endpoint {
                td["security"] = json!(["proxy"]);
                td["properties"]["a"]["forms"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("security");
            }
            let (runtime, handle, observer, parent) = consume_source(td);
            assert!(
                handle
                    .read_property(
                        "a",
                        InteractionOptions::new().with_form_index(0),
                        &mut work()
                    )
                    .is_err()
            );
            if with_endpoint {
                for options in [
                    InteractionOptions::new().with_form_index(1),
                    InteractionOptions::new(),
                ] {
                    let mut call = handle.read_property("a", options, &mut work()).unwrap();
                    assert!(take(&mut call).await.is_ok());
                    drop(call);
                }
            } else {
                assert!(
                    handle
                        .read_property("a", InteractionOptions::new(), &mut work())
                        .is_err()
                );
            }
            assert!(matches!(proxy.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
            close(runtime, handle, observer, parent);
        }
    });
    peer.join().unwrap();
}

#[test]
fn inherited_aliases_keep_definition_time_meaning_on_the_http_wire() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let target = format!("http://{}/a", listener.local_addr().unwrap());
    let peer = std::thread::spawn(move || {
        for method in ["POST", "POST", "GET", "GET", "POST"] {
            let (mut socket, _) = listener.accept().unwrap();
            assert!(request(&mut socket).starts_with(&format!("{method} /a HTTP/1.1\r\n")));
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\n\r\n7").unwrap();
            assert_retired(&mut socket);
        }
    });
    recipe().block_on(async {
        for variant in 0..5 {
            let mut td = source(json!([{"href":target, "verb":"POST"}]));
            let http = "http://www.w3.org/2011/http#";
            let vendor = "urn:vendor:";
            let initial = if variant == 2 { vendor } else { http };
            td["@context"] = json!(["https://www.w3.org/2022/wot/td/v1.1",
                {"ex":initial, "verb":"ex:methodName"}]);
            match variant {
                0 | 2 => {
                    td["properties"]["a"]["forms"][0]["@context"] =
                        json!({"ex":if variant == 2 {http} else {vendor}});
                    if variant == 2 {
                        td["properties"]["a"]["forms"][0]["verb"] = json!("DELETE");
                    }
                }
                1 => td["@context"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"ex":vendor})),
                3 => {
                    td["@context"] = json!(["https://www.w3.org/2022/wot/td/v1.1",
                        {"@vocab":vendor,"verb":"methodName"}]);
                    td["properties"]["a"]["forms"][0]["@context"] = json!({"@vocab":http});
                    td["properties"]["a"]["forms"][0]["verb"] = json!("DELETE");
                }
                _ => {
                    td["properties"]["a"]["forms"][0]["@context"] = json!([
                        {"verb":"ex:methodName"}, {"ex":vendor}
                    ])
                }
            }
            let (runtime, handle, observer, parent) = consume_source(td);
            let mut call = handle
                .read_property("a", InteractionOptions::new(), &mut work())
                .unwrap();
            assert!(take(&mut call).await.is_ok());
            drop(call);
            close(runtime, handle, observer, parent);
        }
    });
    peer.join().unwrap();
}

#[test]
fn unsupported_context_dependencies_never_turn_explicit_instructions_into_defaults() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let target = format!("http://{}/a", listener.local_addr().unwrap());
    recipe().block_on(async {
        for context in [
            json!(["https://www.w3.org/2022/wot/td/v1.1", {"base":"http://www.w3.org/2011/http#", "x":"base", "verb":"x:methodName"}]),
            json!(["https://www.w3.org/2022/wot/td/v1.1", {"htv":"urn:vendor:"}, "https://www.w3.org/2022/wot/td/v1.1"]),
        ] {
            let mut td = source(json!([{"href":target,"verb":"POST"}]));
            td["@context"] = context;
            let (runtime, handle, observer, parent) = consume_source(td);
            assert!(
                handle
                    .read_property("a", InteractionOptions::new(), &mut work())
                    .is_err()
            );
            assert!(listener.accept().is_err());
            close(runtime, handle, observer, parent);
        }
        for context in [
            json!({"@vocab":"htv:namespace:"}),
            json!([{"ex":"htv:"}, {"@vocab":"ex:"}]),
        ] {
            let td = source(json!([{"href":target,"@context":context,"methodName":"POST"}]));
            let (runtime, handle, observer, parent) = consume_source(td);
            assert!(handle.read_property("a", InteractionOptions::new(), &mut work()).is_err());
            assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
            close(runtime, handle, observer, parent);
        }
    });
}

#[test]
fn independent_forms_do_not_share_semantic_preparation_failures() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let target = format!("http://{}/a", listener.local_addr().unwrap());
    let peer = std::thread::spawn(move || {
        for _ in 0..3 {
            let (mut socket, _) = listener.accept().unwrap();
            assert!(request(&mut socket).starts_with("GET /a HTTP/1.1\r\n"));
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\n\r\n7").unwrap();
            assert_retired(&mut socket);
        }
    });
    recipe().block_on(async {
        for variant in 0..3 {
            let mut td = source(json!([
                {"href":target},
                {"href":"mqtt://example.org/topic"}
            ]));
            let other = &mut td["properties"]["a"]["forms"][1];
            match variant {
                // Valid JSON-LD syntax outside this resolver's supported profile.
                0 => {
                    other["@context"] =
                        json!({"labels":{"@id":"urn:labels:", "@container":"@set"}});
                    other["labels"] = json!(["one", "two"]);
                }
                1 => other["urn:many:values"] = json!(vec![0; 256]),
                // The same restriction on an HTTP candidate must not hide its sibling.
                _ => {
                    other["href"] = json!("http://127.0.0.1:9/unsupported");
                    other["@context"] = json!({"@base":"https://example.org/"});
                }
            }
            let (runtime, handle, observer, parent) = consume_source(td);
            let mut call = handle
                .read_property("a", InteractionOptions::new(), &mut work())
                .unwrap();
            assert!(take(&mut call).await.is_ok());
            drop(call);
            close(runtime, handle, observer, parent);
        }
    });
    peer.join().unwrap();
}

#[test]
fn multi_property_multi_form_td_is_not_limited_by_one_shared_semantic_arena() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let target = format!("http://{}/value", listener.local_addr().unwrap());
    let peer = std::thread::spawn(move || {
        for index in 0..24 {
            let (mut socket, _) = listener.accept().unwrap();
            let method = if index % 2 == 0 { "GET" } else { "POST" };
            assert!(request(&mut socket).starts_with(&format!("{method} /value HTTP/1.1\r\n")));
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\n\r\n7").unwrap();
            assert_retired(&mut socket);
        }
    });
    recipe().block_on(async {
        let mut td = source(json!([]));
        td["properties"] = json!({});
        for index in 0..24 {
            td["properties"][format!("p{index:02}")] = json!({
                "type":"number", "urn:unit:":"Celsius",
                "forms":[
                    {"href":target, "htv:methodName":"GET", "urn:label:":"primary"},
                    {"href":target, "htv:methodName":"POST", "urn:label:":"secondary"}
                ]
            });
        }
        let (runtime, handle, observer, parent) = consume_source(td);
        for index in 0..24 {
            let mut call = handle
                .read_property(
                    &format!("p{index:02}"),
                    InteractionOptions::new().with_form_index(index % 2),
                    &mut work(),
                )
                .unwrap();
            assert!(take(&mut call).await.is_ok());
            drop(call);
        }
        close(runtime, handle, observer, parent);
    });
    peer.join().unwrap();
}

#[test]
fn defaults_explicit_methods_headers_and_unrelated_annotations_reach_the_wire() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let target = format!("http://{}/a", listener.local_addr().unwrap());
    let peer = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for _ in 0..8 {
            let (mut socket, _) = listener.accept().unwrap();
            requests.push(request(&mut socket));
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\n\r\n7").unwrap();
            assert_retired(&mut socket);
        }
        requests
    });
    recipe().block_on(async {
        for variant in 0..8 {
            let mut td = source(json!([{"href": target}]));
            match variant {
                1 => {
                    td["@context"] = json!(["https://www.w3.org/2022/wot/td/v1.1", {"saref":"https://saref.etsi.org/core/", "vendor":"urn:vendor:"}]);
                    td["@type"] = json!("saref:TemperatureSensor");
                    td["rdfs:label"] = json!("room sensor");
                    td["properties"]["a"]["@type"] = json!("saref:Temperature");
                    td["properties"]["a"]["rdfs:label"] = json!("Celsius");
                    td["properties"]["a"]["forms"][0]["vendor:metadata"] = json!({"vendor:values":[null, {}, [], "unchanged"]});
                },
                2 => td["properties"]["a"]["forms"][0]["htv:methodName"] = json!("GET"),
                3 => {
                    let form = &mut td["properties"]["a"]["forms"][0];
                    form["@context"] = json!({"verb":{"@id":"htv:methodName"}});
                    form["verb"] = json!("POST");
                },
                4..=6 => td["properties"]["a"]["forms"][0]["htv:methodName"] = json!(["PUT", "PATCH", "DELETE"][variant - 4]),
                7 => {
                    td["properties"]["a"]["forms"][0]["htv:headers"] = json!([
                        {"htv:fieldName":"X-Device-Mode", "htv:fieldValue":"exact"},
                        {"htv:fieldName":"Accept", "htv:fieldValue":"application/json"}
                    ]);
                    td["properties"]["a"]["forms"][0]["response"] = json!({"contentType":"application/json", "htv:statusCodeValue":200});
                },
                _ => {},
            }
            let (runtime, handle, observer, parent) = consume_source(td);
            let mut call = handle.read_property("a", InteractionOptions::new(), &mut work()).unwrap();
            assert_eq!(&*take(&mut call).await.unwrap().data().unwrap().body, b"7");
            drop(call); close(runtime, handle, observer, parent);
        }
    });
    let requests = peer.join().unwrap();
    assert_eq!(
        requests[0], requests[1],
        "unrelated annotations changed native behavior"
    );
    assert_eq!(
        requests[0], requests[2],
        "explicit GET differs from default GET"
    );
    for (index, method) in ["GET", "GET", "GET", "POST", "PUT", "PATCH", "DELETE", "GET"]
        .iter()
        .enumerate()
    {
        assert!(requests[index].starts_with(&format!("{method} /a HTTP/1.1\r\n")));
    }
    assert!(
        requests[7]
            .to_ascii_lowercase()
            .contains("x-device-mode: exact\r\n")
    );
    assert_eq!(
        requests[7]
            .to_ascii_lowercase()
            .matches("accept: application/json")
            .count(),
        1
    );
}

#[test]
fn generic_form_dispatch_skips_other_schemes_and_preserves_strict_selection() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let target = format!("http://{}/a", listener.local_addr().unwrap());
    let td = source(json!([
        {"href":"replybus:temperature", "urn:vendor:replyTopic":"responses"},
        {"href":target, "htv:methodName":"BREW"},
        {"href":target, "htv:methodName":"POST"}
    ]));
    let (runtime, handle, observer, parent) = consume_source(td);
    listener.set_nonblocking(true).unwrap();
    assert!(listener.accept().is_err(), "compilation performed I/O");
    for index in [0, 1] {
        let failure = handle
            .read_property(
                "a",
                InteractionOptions::new().with_form_index(index),
                &mut work(),
            )
            .err()
            .expect("strict unsupported Form accepted");
        assert!(failure.into_parts().0.1.is_none());
        assert!(listener.accept().is_err());
    }
    listener.set_nonblocking(false).unwrap();
    let peer = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        assert!(request(&mut socket).starts_with("POST /a HTTP/1.1\r\n"));
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\n\r\n1",
            )
            .unwrap();
        assert_retired(&mut socket);
    });
    recipe().block_on(async {
        let mut call = handle
            .read_property("a", InteractionOptions::new(), &mut work())
            .unwrap();
        assert!(take(&mut call).await.is_ok());
        drop(call);
    });
    close(runtime, handle, observer, parent);
    peer.join().unwrap();
}

#[test]
fn unsupported_explicit_instructions_never_become_default_requests() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let target = format!("http://{}/a", listener.local_addr().unwrap());
    for extra in [
        json!({"op":["readproperty","writeproperty"],"htv:methodName":"POST"}),
        json!({"op":["readproperty","writeproperty"],"http://www.w3.org/2011/http#methodName":"GET"}),
        json!({"htv:methodName":null}),
        json!({"htv:methodName":"get"}),
        json!({"htv:methodName":["POST"]}),
        json!({"htv:methodName":"CONNECT"}),
        json!({"htv:methodName":"POST", "http://www.w3.org/2011/http#methodName":"GET"}),
        json!({"htv:headers":[{"htv:fieldName":"Authorization", "htv:fieldValue":"secret"}]}),
        json!({"htv:headers":[{"htv:fieldName":"Host", "htv:fieldValue":"elsewhere"}]}),
        json!({"htv:headers":[{"htv:fieldName":"X-Header", "htv:fieldValue":"bad\r\nvalue"}]}),
        json!({"response":{"contentType":"application/json", "htv:statusCodeValue":201}}),
        json!({"htv:headers":[{"@type":"htv:ResponseHeader", "htv:fieldName":"X-Response", "htv:fieldValue":"wrong role"}]}),
        json!({"htv:headers":vec![json!({"htv:fieldName":"X", "htv:fieldValue":""}); 9]}),
        json!({"htv:headers":[{"htv:fieldName":"X", "htv:fieldValue":"a".repeat(1024)}]}),
    ] {
        let mut form = extra;
        form["href"] = json!(target);
        let (runtime, handle, observer, parent) = consume_source(source(json!([form])));
        assert!(
            handle
                .read_property("a", InteractionOptions::new(), &mut work())
                .is_err()
        );
        assert!(
            listener.accept().is_err(),
            "unsupported semantics produced a request"
        );
        close(runtime, handle, observer, parent);
    }
}

#[test]
fn failed_resolution_or_unsupported_security_reclaims_the_original_accounts() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let target = format!("http://{}/a", listener.local_addr().unwrap());
    for variant in 0..3 {
        let mut td = source(json!([{"href":target}]));
        match variant {
            0 => {
                td["@context"] = json!([
                    "https://www.w3.org/2022/wot/td/v1.1",
                    "https://example.org/unprepared-context"
                ])
            }
            1 => td["securityDefinitions"]["none"] = json!({"scheme":"basic", "in":"header"}),
            _ => td["urn:many:values"] = json!(vec![0; 128]),
        }
        let policy = policy();
        let parent = Arc::new(
            clinkz_wot_servient::host_accounts::HostResourceAccounts::try_from_accounts(
                policy.parent_accounts(),
            )
            .unwrap(),
        );
        let registration = try_inline_consumer_registration(
            HttpConsumerConfig::for_cleartext_ip().unwrap(),
            BindingId::new(600),
            BindingGeneration::INITIAL,
            600,
        )
        .unwrap();
        let (runtime, observer) = ConsumerPropertyReadRuntime::try_new(
            registration,
            policy,
            Arc::new(Clock(Instant::now())),
            parent.clone(),
        )
        .unwrap_or_else(|_| panic!("runtime"));
        let thing = serde_json::from_value(td).unwrap();
        let result = runtime.consume(&thing, &mut work(), |build| {
            for _ in 0..10000 {
                build.step(&mut work());
                if build.is_finished() {
                    return;
                }
            }
            panic!("failed build stalled");
        });
        if variant == 1 {
            assert!(result.is_err(), "security refusal must abort admission");
        } else {
            let StartStatus::Ready(handle) = result.unwrap() else {
                panic!("publication");
            };
            assert!(
                handle
                    .read_property("a", InteractionOptions::new(), &mut work())
                    .is_err()
            );
            drop(handle);
        }
        drop(thing);
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert!(
            parent
                .snapshot(ConsumerPropertyReadPolicy::PARENT_KINDS)
                .iter()
                .all(|account| account.used() == 0)
        );
        assert!(listener.accept().is_err());
    }
}

#[test]
fn exact_native_header_capacity_reaches_an_independent_peer() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let target = format!("http://{}/a", listener.local_addr().unwrap());
    let peer = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let head = request(&mut socket);
        for index in 0..8 {
            assert!(head.contains(&format!("x{index}: {}\r\n", "v".repeat(126))));
        }
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\n\r\n7",
            )
            .unwrap();
        assert_retired(&mut socket);
    });
    recipe().block_on(async {
        let headers: Vec<_> = (0..8).map(|index| json!({"htv:fieldName":format!("x{index}"), "htv:fieldValue":"v".repeat(126)})).collect();
        let (runtime, handle, observer, parent) = consume_source(source(json!([{"href":target, "htv:headers":headers}])));
        let mut call = handle.read_property("a", InteractionOptions::new(), &mut work()).unwrap();
        assert!(take(&mut call).await.is_ok()); drop(call); close(runtime, handle, observer, parent);
    });
    peer.join().unwrap();
}

// Independently authored request/reply mapping, not another HTTP implementation.
// This tests public compiler/executor authoring and dispatch, not a real bus SDK.
mod reply_bus {
    use super::*;
    use clinkz_wot_td::semantic::{SemanticScope, SemanticValue};
    use std::{
        pin::Pin,
        sync::atomic::{AtomicUsize, Ordering},
        task::Poll,
    };
    const COMPAT: BindingArtifactCompatibility =
        BindingArtifactCompatibility::new(*b"replybus-read-v1");
    include!("support/semantic_schema.rs");
    #[derive(Clone, Copy)]
    struct Program {
        request_topic: [u8; 32],
        reply_topic: [u8; 32],
    }
    struct Compiler {
        config: ConsumerBindingConfiguration,
        compiled: Arc<AtomicUsize>,
    }
    impl HostSemanticCompiler for Compiler {
        type Artifact = Program;
        const WORK: u64 = 1;
        fn compatibility(&self) -> BindingArtifactCompatibility {
            COMPAT
        }
        fn configuration(&self) -> ConsumerBindingConfiguration {
            self.config
        }
        fn schemes(&self) -> &'static [&'static str] {
            &["replybus"]
        }
        fn compile(&self, input: ResolvedBindingInput<'_>) -> CoreResult<Program> {
            self.compiled.fetch_add(1, Ordering::Relaxed);
            let form = input
                .document
                .scope(SemanticScope::Form {
                    property: input.property,
                    index: input.form_index,
                })
                .unwrap();
            // Vendor protocol vocabulary is interpreted entirely in this crate.
            let SemanticValue::Text(reply) = form.field("urn:replybus:replyTopic").unwrap().value()
            else {
                panic!("reply topic missing");
            };
            let topic = input.target.strip_prefix("replybus:").unwrap();
            assert_eq!(
                input
                    .document
                    .scope(SemanticScope::Thing)
                    .unwrap()
                    .field("urn:opaque:annotation")
                    .unwrap()
                    .value(),
                SemanticValue::Text("retained")
            );
            assert_eq!(
                input
                    .document
                    .scope(SemanticScope::Property("a"))
                    .unwrap()
                    .field("urn:opaque:unit")
                    .unwrap()
                    .value(),
                SemanticValue::Text("Kelvin")
            );
            if topic.len() > 32 || reply.len() > 32 {
                return Err(CoreError::UnsupportedOperation(ErrorContext::new(
                    ErrorPhase::Admission,
                    RetryClass::Never,
                )));
            }
            let mut program = Program {
                request_topic: [0; 32],
                reply_topic: [0; 32],
            };
            program.request_topic[..topic.len()].copy_from_slice(topic.as_bytes());
            program.reply_topic[..reply.len()].copy_from_slice(reply.as_bytes());
            Ok(program)
        }
    }
    struct Client(ConsumerBindingConfiguration);
    impl binding::ClientBinding for Client {
        fn supports_operation(&self, operation: clinkz_wot_core::Operation) -> bool {
            operation == clinkz_wot_core::Operation::ReadProperty
        }
        fn consumer_configuration(&self) -> Option<ConsumerBindingConfiguration> {
            Some(self.0)
        }
        fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
            COMPAT
        }
        fn invoke(
            &self,
            request: OutboundRequest,
            artifact: &BindingArtifactEnvelope<HostBindingArtifact>,
        ) -> Result<
            HostBindingCallBox<CoreResult<InteractionOutput>>,
            BindingInputRejection<OutboundRequest>,
        > {
            let program = artifact.artifact().try_payload::<Program>(COMPAT).unwrap();
            assert_eq!(&program.request_topic[..11], b"temperature");
            assert_eq!(&program.reply_topic[..7], b"replies");
            HostBindingCallBox::try_new(Call(request)).map_err(|call| {
                BindingInputRejection::new(
                    call.0,
                    BindingOperationalError::new(CoreError::Backpressure(ErrorContext::new(
                        ErrorPhase::Binding,
                        RetryClass::Never,
                    ))),
                )
            })
        }
    }
    struct Call(OutboundRequest);
    impl HostBindingCall<CoreResult<InteractionOutput>> for Call {
        fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
            BindingLifetimeFootprint::new(1, size_of::<Self>() as u64)
        }
        fn next_deadline(&self) -> Option<Deadline> {
            self.0.deadline()
        }
        fn poll_result(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
            budget: &mut WorkBudget,
        ) -> Poll<CoreResult<InteractionOutput>> {
            if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                return Poll::Pending;
            }
            Poll::Ready(Ok(InteractionOutput::with_data(Payload::new(
                vec![b'7'],
                "application/json",
            ))
            .try_with_metadata(
                InteractionOutputMetadata::default().with_untrusted_binding_response(
                    BindingResponseMetadata::primary(
                        self.0.binding_id(),
                        self.0.binding_generation(),
                        self.0.plan_id(),
                        17,
                    ),
                ),
            )
            .unwrap()))
        }
        fn start_cancel(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
            _: CleanupPhaseContext,
            budget: &mut WorkBudget,
        ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
            if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                return Ok(StartStatus::Pending);
            }
            Ok(StartStatus::Ready(BindingCallSettlement::Cancelled {
                retry_class: RetryClass::Never,
                disposition: BindingCancellationDisposition::Complete {
                    successor: NoCleanupSuccessor,
                },
            }))
        }
        fn poll_cancel(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            budget: &mut WorkBudget,
        ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
            // This source-free fixture owns no network resources to retire.
            let _ = cx;
            if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                return Poll::Pending;
            }
            Poll::Ready(Ok(BindingCallSettlement::Cancelled {
                retry_class: RetryClass::Never,
                disposition: BindingCancellationDisposition::Complete {
                    successor: NoCleanupSuccessor,
                },
            }))
        }
    }

    #[test]
    fn third_party_request_reply_compiler_and_executor_use_the_same_production_path() {
        let config = ConsumerBindingConfiguration::for_semantic_host(
            64,
            BindingConfigurationDigest::new([91; 32]),
            COMPAT,
            4096,
            16,
        )
        .unwrap();
        let wider = ConsumerBindingConfiguration::for_semantic_host(
            1024,
            BindingConfigurationDigest::new([91; 32]),
            COMPAT,
            4096,
            16,
        )
        .unwrap();
        assert_eq!(wider.target_capacity(), 1024);
        assert_ne!(wider.configuration(), config.configuration());
        assert!(wider.with_default_json_forms().is_err());
        let compiled = Arc::new(AtomicUsize::new(0));
        let compiler = HostBindingCompilerRegistration::try_new_inline_semantic(Compiler {
            config,
            compiled: compiled.clone(),
        })
        .unwrap_or_else(|_| panic!("compiler capture"));
        let owner = HostConsumerBindingPreparation::new(HostConsumerBindingPreparationInput::new(
            BindingRegistrationIdentity::new(
                BindingId::new(901),
                BindingGeneration::INITIAL,
                config.configuration(),
                COMPAT,
                901,
            ),
            BindingExecutionSupport::host_erased(),
            compiler,
            Box::new(Client(config)),
            BindingResourceDeclarations::new(
                BindingLifetimeFootprint::new(3, 1024),
                BindingLifetimeFootprint::new(16, 8192),
            )
            .with_resource_contract(BindingResourceContract::ConsumerOwnedHostV2)
            .with_consumer_result_limits(4096, 16),
            BindingStatusPolicy::new(2, 17),
        ))
        .unwrap_or_else(|_| panic!("complete owner capture"))
        .try_into_inline_consumer_compiler()
        .unwrap_or_else(|_| panic!("open compiler capture"));
        let policy = policy();
        let parent = Arc::new(
            clinkz_wot_servient::host_accounts::HostResourceAccounts::try_from_accounts(
                policy.parent_accounts(),
            )
            .unwrap(),
        );
        let (runtime, observer) = ConsumerPropertyReadRuntime::try_new(
            owner,
            policy,
            Arc::new(Clock(Instant::now())),
            parent.clone(),
        )
        .unwrap_or_else(|_| panic!("runtime"));
        let mut td = source(
            json!([{"href":"http://127.0.0.1/other"}, {"href":"replybus:temperature", "urn:replybus:replyTopic":"replies"}]),
        );
        td["urn:opaque:annotation"] = json!("retained");
        td["properties"]["a"]["urn:opaque:unit"] = json!("Kelvin");
        let thing = serde_json::from_value(td).unwrap();
        let handle = match runtime
            .consume(&thing, &mut work(), |build| {
                build.step(&mut WorkBudget::new());
                assert_eq!(compiled.load(Ordering::Relaxed), 0);
                for _ in 0..10000 {
                    build.step(&mut work());
                    if build.is_finished() {
                        return;
                    }
                }
                panic!("build stalled");
            })
            .unwrap()
        {
            StartStatus::Ready(handle) => handle,
            _ => panic!("publication"),
        };
        assert_eq!(compiled.load(Ordering::Relaxed), 1);
        drop(thing);
        let mut call = handle
            .read_property("a", InteractionOptions::new(), &mut work())
            .unwrap();
        call.step(&mut Context::from_waker(Waker::noop()), &mut work());
        assert_eq!(
            &*call
                .try_take(&mut work())
                .unwrap()
                .unwrap()
                .unwrap()
                .data()
                .unwrap()
                .body,
            b"7"
        );
        drop(call);
        close(runtime, handle, observer, parent);
    }
}
