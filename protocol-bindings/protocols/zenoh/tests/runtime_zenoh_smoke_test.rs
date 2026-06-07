#![cfg(feature = "runtime-zenoh")]

use std::{
    env, thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use clinkz_wot_core::Payload;
use clinkz_wot_protocol_bindings_zenoh::{
    ZenohFormMetadata, ZenohOperationKind, ZenohOperationPlan, ZenohSessionTransport,
    ZenohTransport, ZenohTransportRequest,
};
use zenoh::{Config, Wait};

const RUN_ZENOH_RUNTIME_TESTS: &str = "CLINKZ_WOT_RUN_ZENOH_RUNTIME_TESTS";
const ZENOH_ENDPOINT: &str = "CLINKZ_WOT_ZENOH_ENDPOINT";
const REPLY_TIMEOUT: Duration = Duration::from_secs(5);
const DECLARATION_PROPAGATION_DELAY: Duration = Duration::from_millis(500);

#[test]
fn runtime_zenoh_transport_executes_put_and_get_smoke_paths() {
    if env::var(RUN_ZENOH_RUNTIME_TESTS).ok().as_deref() != Some("1") {
        eprintln!("skipping real zenoh runtime smoke test; set {RUN_ZENOH_RUNTIME_TESTS}=1");
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
