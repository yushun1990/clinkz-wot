#![cfg(feature = "runtime-zenoh")]

use std::{
    collections::BTreeMap,
    env, thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use clinkz_wot_core::{CoreError, Payload};
use clinkz_wot_protocol_bindings_zenoh::{
    ZenohFormMetadata, ZenohOperationKind, ZenohOperationPlan, ZenohSessionTransport,
    ZenohTransport, ZenohTransportRequest,
};
use zenoh::{
    Config, Wait,
    qos::{CongestionControl, Priority},
};

const RUN_ZENOH_RUNTIME_TESTS: &str = "CLINKZ_WOT_RUN_ZENOH_RUNTIME_TESTS";
const ZENOH_ENDPOINT: &str = "CLINKZ_WOT_ZENOH_ENDPOINT";
const REPLY_TIMEOUT: Duration = Duration::from_secs(5);
const DECLARATION_PROPAGATION_DELAY: Duration = Duration::from_millis(500);

#[test]
fn runtime_zenoh_transport_executes_put_and_get_smoke_paths() {
    if !should_run_runtime_tests() {
        return;
    }

    zenoh::init_log_from_env_or("error");

    let session = open_runtime_session();
    let key_expr = unique_key_expr("runtime-smoke");
    let put_key_expr = key_expr.clone();
    let query_key_expr = key_expr.clone();
    let queryable_key_expr = key_expr.clone();

    let subscriber = session
        .declare_subscriber(put_key_expr.as_str())
        .wait()
        .expect("declare subscriber");
    let queryable = session
        .declare_queryable(queryable_key_expr.as_str())
        .wait()
        .expect("declare queryable");

    thread::sleep(DECLARATION_PROPAGATION_DELAY);

    let reply_thread = thread::spawn(move || {
        let query = queryable
            .handler()
            .recv_timeout(REPLY_TIMEOUT)
            .expect("receive query")
            .expect("query should arrive");
        query
            .reply(query_key_expr.as_str(), "runtime-query-reply")
            .encoding("text/plain")
            .wait()
            .expect("reply to query");
        queryable.undeclare().wait().expect("undeclare queryable");
    });

    let mut transport = ZenohSessionTransport::new(session).with_reply_timeout(REPLY_TIMEOUT);
    let put_output = transport
        .execute(ZenohTransportRequest {
            plan: ZenohOperationPlan {
                key_expr: key_expr.clone(),
                kind: ZenohOperationKind::Put,
                metadata: ZenohFormMetadata {
                    encoding: Some("text/plain".into()),
                    ..Default::default()
                },
            },
            payload: Some(Payload::new(b"runtime-put".to_vec(), "text/plain")),
            parameters: Default::default(),
        })
        .expect("execute put");

    assert!(put_output.payload.is_none());

    let sample = subscriber
        .recv_timeout(REPLY_TIMEOUT)
        .expect("receive put sample")
        .expect("put sample should arrive");
    assert_eq!(sample.payload().to_bytes().as_ref(), b"runtime-put");

    let get_output = transport
        .execute(ZenohTransportRequest {
            plan: ZenohOperationPlan {
                key_expr,
                kind: ZenohOperationKind::RequestReply,
                metadata: ZenohFormMetadata {
                    encoding: Some("text/plain".into()),
                    ..Default::default()
                },
            },
            payload: None,
            parameters: Default::default(),
        })
        .expect("execute get");
    let payload = get_output.payload.expect("query reply payload");

    assert_eq!(payload.content_type, "text/plain");
    assert_eq!(payload.body, b"runtime-query-reply");

    reply_thread.join().expect("join query reply thread");
    subscriber.undeclare().wait().expect("undeclare subscriber");
}

#[test]
fn runtime_zenoh_transport_executes_subscribe_once_smoke_path() {
    if !should_run_runtime_tests() {
        return;
    }

    zenoh::init_log_from_env_or("error");

    let session = open_runtime_session();
    let key_expr = unique_key_expr("runtime-subscribe-once");
    let publish_session = session.clone();
    let publish_key_expr = key_expr.clone();

    let publish_thread = thread::spawn(move || {
        thread::sleep(DECLARATION_PROPAGATION_DELAY);
        publish_session
            .put(publish_key_expr.as_str(), "runtime-subscribe-once-event")
            .wait()
            .expect("publish one-shot subscription event");
    });

    let mut transport = ZenohSessionTransport::new(session).with_reply_timeout(REPLY_TIMEOUT);
    let output = transport
        .execute(ZenohTransportRequest {
            plan: ZenohOperationPlan {
                key_expr,
                kind: ZenohOperationKind::Subscribe,
                metadata: ZenohFormMetadata {
                    encoding: Some("text/plain".into()),
                    ..Default::default()
                },
            },
            payload: None,
            parameters: Default::default(),
        })
        .expect("execute one-shot subscribe");
    let payload = output.payload.expect("one-shot subscription payload");

    assert_eq!(payload.content_type, "text/plain");
    assert_eq!(payload.body, b"runtime-subscribe-once-event");

    publish_thread.join().expect("join one-shot publish thread");
}

#[test]
fn runtime_zenoh_put_propagates_live_metadata() {
    if !should_run_runtime_tests() {
        return;
    }

    zenoh::init_log_from_env_or("error");

    let session = open_runtime_session();
    let key_expr = unique_key_expr("runtime-put-metadata");
    let subscriber = session
        .declare_subscriber(key_expr.as_str())
        .wait()
        .expect("declare metadata subscriber");

    thread::sleep(DECLARATION_PROPAGATION_DELAY);

    let mut transport = ZenohSessionTransport::new(session).with_reply_timeout(REPLY_TIMEOUT);
    let put_output = transport
        .execute(ZenohTransportRequest {
            plan: ZenohOperationPlan {
                key_expr,
                kind: ZenohOperationKind::Put,
                metadata: ZenohFormMetadata {
                    encoding: Some("application/json".into()),
                    qos: Some("express".into()),
                    priority: Some("background".into()),
                    congestion_control: Some("block".into()),
                },
            },
            payload: Some(Payload::new(
                br#"{"runtime":"metadata"}"#.to_vec(),
                "application/json",
            )),
            parameters: Default::default(),
        })
        .expect("execute metadata put");

    assert!(put_output.payload.is_none());

    let sample = subscriber
        .recv_timeout(REPLY_TIMEOUT)
        .expect("receive metadata sample")
        .expect("metadata sample should arrive");

    assert_eq!(
        sample.payload().to_bytes().as_ref(),
        br#"{"runtime":"metadata"}"#
    );
    assert_eq!(sample.encoding().to_string(), "application/json");
    assert!(sample.express());
    assert_eq!(sample.priority(), Priority::Background);
    assert_eq!(sample.congestion_control(), CongestionControl::Block);

    subscriber
        .undeclare()
        .wait()
        .expect("undeclare metadata subscriber");
}

#[test]
fn runtime_zenoh_subscription_receives_multiple_samples_and_undeclares() {
    if !should_run_runtime_tests() {
        return;
    }

    zenoh::init_log_from_env_or("error");

    let session = open_runtime_session();
    let key_expr = unique_key_expr("runtime-subscription");
    let transport = ZenohSessionTransport::new(session.clone()).with_reply_timeout(REPLY_TIMEOUT);
    let mut subscription = transport
        .subscribe(ZenohOperationPlan {
            key_expr: key_expr.clone(),
            kind: ZenohOperationKind::Subscribe,
            metadata: ZenohFormMetadata {
                encoding: Some("text/plain".into()),
                ..Default::default()
            },
        })
        .expect("declare runtime subscription");

    assert_eq!(subscription.key_expr(), key_expr);
    assert_eq!(subscription.content_type_hint(), Some("text/plain"));
    assert_eq!(subscription.reply_timeout(), REPLY_TIMEOUT);

    thread::sleep(DECLARATION_PROPAGATION_DELAY);

    session
        .put(key_expr.as_str(), "runtime-event-1")
        .wait()
        .expect("publish first runtime event");

    let first_payload = subscription
        .next_sample()
        .expect("receive first subscription sample")
        .payload
        .expect("first subscription sample payload");

    assert_eq!(first_payload.content_type, "text/plain");
    assert_eq!(first_payload.body, b"runtime-event-1");

    session
        .put(key_expr.as_str(), "runtime-event-2")
        .wait()
        .expect("publish second runtime event");

    let second_payload = subscription
        .next_sample()
        .expect("receive second subscription sample")
        .payload
        .expect("second subscription sample payload");

    assert_eq!(second_payload.content_type, "text/plain");
    assert_eq!(second_payload.body, b"runtime-event-2");

    subscription
        .undeclare()
        .expect("undeclare runtime subscription");
}

#[test]
fn runtime_zenoh_subscription_timeout_maps_to_transport_error() {
    if !should_run_runtime_tests() {
        return;
    }

    zenoh::init_log_from_env_or("error");

    let session = open_runtime_session();
    let key_expr = unique_key_expr("runtime-subscription-timeout");
    let transport = ZenohSessionTransport::new(session).with_reply_timeout(REPLY_TIMEOUT);
    let mut subscription = transport
        .subscribe(ZenohOperationPlan {
            key_expr: key_expr.clone(),
            kind: ZenohOperationKind::Subscribe,
            metadata: ZenohFormMetadata {
                encoding: Some("text/plain".into()),
                ..Default::default()
            },
        })
        .expect("declare runtime subscription");

    thread::sleep(DECLARATION_PROPAGATION_DELAY);

    let error = subscription
        .next_timeout(Duration::from_millis(100))
        .expect_err("subscription timeout should map to transport error");

    assert_eq!(
        error,
        CoreError::Transport(format!("Zenoh subscription for '{key_expr}' timed out"))
    );

    subscription
        .undeclare()
        .expect("undeclare timed out runtime subscription");
}

#[test]
fn runtime_zenoh_request_reply_timeout_maps_to_transport_error() {
    if !should_run_runtime_tests() {
        return;
    }

    zenoh::init_log_from_env_or("error");

    let key_expr = unique_key_expr("runtime-query-timeout");
    let mut transport = ZenohSessionTransport::new(open_runtime_session())
        .with_reply_timeout(Duration::from_millis(100));

    let error = transport
        .execute(ZenohTransportRequest {
            plan: ZenohOperationPlan {
                key_expr: key_expr.clone(),
                kind: ZenohOperationKind::RequestReply,
                metadata: ZenohFormMetadata {
                    encoding: Some("text/plain".into()),
                    ..Default::default()
                },
            },
            payload: None,
            parameters: Default::default(),
        })
        .expect_err("query timeout should map to transport error");

    assert_eq!(
        error,
        CoreError::Transport(format!("Zenoh query for '{key_expr}' timed out"))
    );
}

#[test]
fn runtime_zenoh_request_reply_propagates_selector_parameters() {
    if !should_run_runtime_tests() {
        return;
    }

    zenoh::init_log_from_env_or("error");

    let session = open_runtime_session();
    let base_key_expr = unique_key_expr("runtime-query-parameters");
    let planned_key_expr = format!("{base_key_expr}?mode=fast");
    let reply_key_expr = base_key_expr.clone();
    let queryable = session
        .declare_queryable(base_key_expr.as_str())
        .wait()
        .expect("declare parameter queryable");

    thread::sleep(DECLARATION_PROPAGATION_DELAY);

    let reply_thread = thread::spawn(move || {
        let query = queryable
            .handler()
            .recv_timeout(REPLY_TIMEOUT)
            .expect("receive parameterized query")
            .expect("parameterized query should arrive");

        assert_eq!(
            query.selector().to_string(),
            format!("{reply_key_expr}?mode=fast;reply=summary;trace")
        );
        assert_eq!(query.parameters().get("mode"), Some("fast"));
        assert_eq!(query.parameters().get("reply"), Some("summary"));
        assert!(query.parameters().contains_key("trace"));

        query
            .reply(reply_key_expr.as_str(), "runtime-query-parameters-ok")
            .encoding("text/plain")
            .wait()
            .expect("reply to parameterized query");
        queryable
            .undeclare()
            .wait()
            .expect("undeclare parameter queryable");
    });

    let mut parameters = BTreeMap::new();
    parameters.insert("reply".into(), "summary".into());
    parameters.insert("trace".into(), String::new());

    let mut transport = ZenohSessionTransport::new(session).with_reply_timeout(REPLY_TIMEOUT);
    let output = transport
        .execute(ZenohTransportRequest {
            plan: ZenohOperationPlan {
                key_expr: planned_key_expr,
                kind: ZenohOperationKind::RequestReply,
                metadata: ZenohFormMetadata {
                    encoding: Some("text/plain".into()),
                    ..Default::default()
                },
            },
            payload: None,
            parameters,
        })
        .expect("execute parameterized get");
    let payload = output.payload.expect("parameterized query reply payload");

    assert_eq!(payload.content_type, "text/plain");
    assert_eq!(payload.body, b"runtime-query-parameters-ok");

    reply_thread.join().expect("join parameter query thread");
}

#[test]
fn runtime_zenoh_request_reply_propagates_request_payload() {
    if !should_run_runtime_tests() {
        return;
    }

    zenoh::init_log_from_env_or("error");

    let session = open_runtime_session();
    let key_expr = unique_key_expr("runtime-query-payload");
    let reply_key_expr = key_expr.clone();
    let queryable = session
        .declare_queryable(key_expr.as_str())
        .wait()
        .expect("declare payload queryable");

    thread::sleep(DECLARATION_PROPAGATION_DELAY);

    let request_body = b"runtime-request-body".to_vec();
    let expected_request_body = request_body.clone();
    let reply_thread = thread::spawn(move || {
        let query = queryable
            .handler()
            .recv_timeout(REPLY_TIMEOUT)
            .expect("receive payload query")
            .expect("payload query should arrive");

        assert_eq!(
            query
                .payload()
                .expect("payload query should include a request body")
                .to_bytes()
                .as_ref(),
            expected_request_body
        );
        assert_eq!(
            query
                .encoding()
                .expect("payload query should include request encoding")
                .to_string(),
            "text/plain"
        );

        query
            .reply(reply_key_expr.as_str(), "runtime-query-payload-ok")
            .encoding("text/plain")
            .wait()
            .expect("reply to payload query");
        queryable
            .undeclare()
            .wait()
            .expect("undeclare payload queryable");
    });

    let mut transport = ZenohSessionTransport::new(session).with_reply_timeout(REPLY_TIMEOUT);
    let output = transport
        .execute(ZenohTransportRequest {
            plan: ZenohOperationPlan {
                key_expr,
                kind: ZenohOperationKind::RequestReply,
                metadata: ZenohFormMetadata {
                    encoding: Some("text/plain".into()),
                    ..Default::default()
                },
            },
            payload: Some(Payload::new(request_body, "text/plain")),
            parameters: Default::default(),
        })
        .expect("execute payload query");
    let payload = output.payload.expect("payload query reply payload");

    assert_eq!(payload.content_type, "text/plain");
    assert_eq!(payload.body, b"runtime-query-payload-ok");

    reply_thread.join().expect("join payload query thread");
}

#[test]
fn runtime_zenoh_request_reply_uses_live_reply_encoding() {
    if !should_run_runtime_tests() {
        return;
    }

    zenoh::init_log_from_env_or("error");

    let session = open_runtime_session();
    let key_expr = unique_key_expr("runtime-query-reply-encoding");
    let reply_key_expr = key_expr.clone();
    let queryable = session
        .declare_queryable(key_expr.as_str())
        .wait()
        .expect("declare reply encoding queryable");

    thread::sleep(DECLARATION_PROPAGATION_DELAY);

    let reply_thread = thread::spawn(move || {
        let query = queryable
            .handler()
            .recv_timeout(REPLY_TIMEOUT)
            .expect("receive reply encoding query")
            .expect("reply encoding query should arrive");

        assert_eq!(
            query
                .encoding()
                .expect("reply encoding query should include request encoding")
                .to_string(),
            "application/json"
        );

        query
            .reply(reply_key_expr.as_str(), "runtime-query-reply-encoding-ok")
            .encoding("text/plain")
            .wait()
            .expect("reply to reply encoding query");
        queryable
            .undeclare()
            .wait()
            .expect("undeclare reply encoding queryable");
    });

    let mut transport = ZenohSessionTransport::new(session).with_reply_timeout(REPLY_TIMEOUT);
    let output = transport
        .execute(ZenohTransportRequest {
            plan: ZenohOperationPlan {
                key_expr,
                kind: ZenohOperationKind::RequestReply,
                metadata: ZenohFormMetadata {
                    encoding: Some("application/json".into()),
                    ..Default::default()
                },
            },
            payload: Some(Payload::new(
                br#"{"request":"reply-encoding"}"#.to_vec(),
                "application/json",
            )),
            parameters: Default::default(),
        })
        .expect("execute reply encoding query");
    let payload = output.payload.expect("reply encoding query reply payload");

    assert_eq!(payload.content_type, "text/plain");
    assert_eq!(payload.body, b"runtime-query-reply-encoding-ok");

    reply_thread
        .join()
        .expect("join reply encoding query thread");
}

fn should_run_runtime_tests() -> bool {
    if env::var(RUN_ZENOH_RUNTIME_TESTS).ok().as_deref() == Some("1") {
        true
    } else {
        eprintln!("skipping real zenoh runtime smoke test; set {RUN_ZENOH_RUNTIME_TESTS}=1");
        false
    }
}

fn open_runtime_session() -> zenoh::Session {
    ZenohSessionTransport::open(runtime_config())
        .expect("open zenoh runtime session")
        .session()
        .clone()
}

fn runtime_config() -> Config {
    let mut config = Config::default();
    if let Ok(endpoint) = env::var(ZENOH_ENDPOINT) {
        config
            .insert_json5("connect/endpoints", &format!("[\"{endpoint}\"]"))
            .expect("set zenoh endpoint");
    }
    config
}

fn unique_key_expr(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();
    format!("clinkz/wot/tests/{prefix}/{}/{}", std::process::id(), nanos)
}
