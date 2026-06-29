#![cfg(feature = "zenoh-pico")]

use std::time::Duration;

use clinkz_wot_core::{CoreError, Payload};
use clinkz_wot_protocol_bindings_zenoh::{
    ZenohFormMetadata, ZenohOperationKind, ZenohOperationPlan, ZenohPicoError, ZenohPicoErrorKind,
    ZenohPicoPlatform, ZenohPicoRequest, ZenohPicoTransport, ZenohTransport, ZenohTransportRequest,
};

#[derive(Debug, Default)]
struct FakePicoPlatform {
    calls: Vec<String>,
    fail_next: Option<ZenohPicoError>,
    query_reply: Option<Payload>,
    subscription_reply: Option<Payload>,
}

impl ZenohPicoPlatform for FakePicoPlatform {
    fn put(&mut self, request: ZenohPicoRequest<'_>) -> Result<(), ZenohPicoError> {
        self.calls.push(format!(
            "put:{}:{}:{}:{}",
            request.target_expr()?.as_ref(),
            request.metadata.content_type.as_deref().unwrap_or(""),
            request
                .payload
                .map(|payload| payload.body.len())
                .unwrap_or(0),
            request.timeout.as_millis()
        ));
        self.take_error()
    }

    fn query(&mut self, request: ZenohPicoRequest<'_>) -> Result<Option<Payload>, ZenohPicoError> {
        self.calls.push(format!(
            "{}:{}",
            request.operation_name(),
            request.target_expr()?.as_ref()
        ));
        self.take_error()?;
        Ok(self.query_reply.take())
    }

    fn subscribe(
        &mut self,
        request: ZenohPicoRequest<'_>,
    ) -> Result<Option<Payload>, ZenohPicoError> {
        self.calls.push(format!(
            "{}:{}",
            request.operation_name(),
            request.target_expr()?.as_ref()
        ));
        self.take_error()?;
        Ok(self.subscription_reply.take())
    }

    fn unsubscribe(&mut self, request: ZenohPicoRequest<'_>) -> Result<(), ZenohPicoError> {
        self.calls.push(format!(
            "{}:{}",
            request.operation_name(),
            request.target_expr()?.as_ref()
        ));
        self.take_error()
    }
}

impl FakePicoPlatform {
    fn take_error(&mut self) -> Result<(), ZenohPicoError> {
        match self.fail_next.take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

#[test]
fn pico_transport_routes_put_and_query_requests_to_platform_hooks() {
    let mut transport = ZenohPicoTransport::new(FakePicoPlatform {
        query_reply: Some(Payload::new(b"query-reply".to_vec(), "text/plain")),
        ..Default::default()
    })
    .with_reply_timeout(Duration::from_millis(250));

    let put = transport
        .execute(ZenohTransportRequest {
            plan: std::sync::Arc::new(ZenohOperationPlan {
                key_expr: "clinkz/things/lamp/status".into(),
                kind: ZenohOperationKind::Put,
                metadata: ZenohFormMetadata {
                    content_type: Some("text/plain".into()),
                    ..Default::default()
                },
            }),
            payload: Some(Payload::new(b"on".to_vec(), "text/plain")),
            parameters: Default::default(),
        })
        .unwrap();

    assert!(put.payload.is_none());

    let mut query_parameters = std::collections::BTreeMap::new();
    query_parameters.insert("trace".into(), "full".into());
    let query = transport
        .execute(ZenohTransportRequest {
            plan: std::sync::Arc::new(ZenohOperationPlan {
                key_expr: "clinkz/things/lamp/status".into(),
                kind: ZenohOperationKind::RequestReply,
                metadata: ZenohFormMetadata::default(),
            }),
            payload: None,
            parameters: query_parameters,
        })
        .unwrap();

    assert_eq!(query.payload.unwrap().body.as_ref(), b"query-reply");
    assert_eq!(
        transport
            .platform()
            .downcast_ref::<FakePicoPlatform>()
            .expect("fake platform")
            .calls,
        [
            "put:clinkz/things/lamp/status:text/plain:2:250",
            "request-reply:clinkz/things/lamp/status?trace=full"
        ]
    );
}

#[test]
fn pico_transport_routes_subscription_lifecycle_hooks() {
    let mut transport = ZenohPicoTransport::new(FakePicoPlatform {
        subscription_reply: Some(Payload::new(b"event".to_vec(), "text/plain")),
        ..Default::default()
    });

    let subscribed = transport
        .execute(ZenohTransportRequest {
            plan: std::sync::Arc::new(ZenohOperationPlan {
                key_expr: "clinkz/things/lamp/events/status".into(),
                kind: ZenohOperationKind::Subscribe,
                metadata: ZenohFormMetadata::default(),
            }),
            payload: None,
            parameters: Default::default(),
        })
        .unwrap();
    let unsubscribed = transport
        .execute(ZenohTransportRequest {
            plan: std::sync::Arc::new(ZenohOperationPlan {
                key_expr: "clinkz/things/lamp/events/status".into(),
                kind: ZenohOperationKind::Unsubscribe,
                metadata: ZenohFormMetadata::default(),
            }),
            payload: None,
            parameters: Default::default(),
        })
        .unwrap();

    assert_eq!(subscribed.payload.unwrap().body.as_ref(), b"event");
    assert!(unsubscribed.payload.is_none());
    assert_eq!(
        transport
            .platform()
            .downcast_ref::<FakePicoPlatform>()
            .expect("fake platform")
            .calls,
        [
            "subscribe:clinkz/things/lamp/events/status",
            "unsubscribe:clinkz/things/lamp/events/status"
        ]
    );
}

#[test]
fn pico_transport_maps_platform_errors_and_timeouts() {
    let mut failing = ZenohPicoTransport::new(FakePicoPlatform {
        fail_next: Some(ZenohPicoError::with_code(-7, "platform rejected put")),
        ..Default::default()
    });

    let err = failing
        .execute(ZenohTransportRequest {
            plan: std::sync::Arc::new(ZenohOperationPlan {
                key_expr: "clinkz/things/lamp/status".into(),
                kind: ZenohOperationKind::Put,
                metadata: ZenohFormMetadata::default(),
            }),
            payload: None,
            parameters: Default::default(),
        })
        .unwrap_err();

    assert_eq!(
        err,
        CoreError::Transport("zenoh-pico status -7: platform rejected put".into())
    );

    let timeout = ZenohPicoError::timeout("query", "clinkz/things/lamp/status");
    assert_eq!(timeout.kind(), ZenohPicoErrorKind::Timeout);
    assert!(timeout.is_timeout());
    assert_eq!(timeout.code(), None);
    assert_eq!(
        timeout.message(),
        "Zenoh-pico query for 'clinkz/things/lamp/status' timed out"
    );

    let mut timing_out = ZenohPicoTransport::new(FakePicoPlatform::default());
    let err = timing_out
        .execute(ZenohTransportRequest {
            plan: std::sync::Arc::new(ZenohOperationPlan {
                key_expr: "clinkz/things/lamp/status".into(),
                kind: ZenohOperationKind::Query,
                metadata: ZenohFormMetadata::default(),
            }),
            payload: None,
            parameters: Default::default(),
        })
        .unwrap_err();

    assert_eq!(
        err,
        CoreError::Transport("Zenoh-pico query for 'clinkz/things/lamp/status' timed out".into())
    );
}

#[test]
fn pico_error_can_classify_rejected_requests() {
    let err = ZenohPicoError::invalid_request("target rejects request parameters");

    assert_eq!(err.kind(), ZenohPicoErrorKind::Request);
    assert_eq!(err.code(), None);
    assert!(!err.is_timeout());
    assert_eq!(err.message(), "target rejects request parameters");
}

#[test]
fn pico_request_builds_selector_from_request_parameters() {
    let mut parameters = std::collections::BTreeMap::new();
    parameters.insert("reply".into(), "summary".into());
    parameters.insert("trace".into(), String::new());

    let request = ZenohPicoRequest {
        key_expr: "clinkz/things/lamp/actions/reboot",
        kind: ZenohOperationKind::RequestReply,
        metadata: &ZenohFormMetadata::default(),
        payload: None,
        parameters: &parameters,
        timeout: Duration::from_secs(1),
    };

    assert_eq!(
        request.selector().unwrap(),
        "clinkz/things/lamp/actions/reboot?reply=summary;trace"
    );
    assert_eq!(
        request.target_expr().unwrap().as_ref(),
        "clinkz/things/lamp/actions/reboot?reply=summary;trace"
    );
    assert_eq!(request.operation_name(), "request-reply");
}

#[test]
fn pico_request_keeps_key_expression_for_non_query_operations() {
    let metadata = ZenohFormMetadata::default();
    let parameters = std::collections::BTreeMap::new();
    let request = ZenohPicoRequest {
        key_expr: "clinkz/things/lamp/events/status",
        kind: ZenohOperationKind::Subscribe,
        metadata: &metadata,
        payload: None,
        parameters: &parameters,
        timeout: Duration::from_secs(1),
    };

    assert_eq!(
        request.target_expr().unwrap().as_ref(),
        "clinkz/things/lamp/events/status"
    );
    assert_eq!(request.operation_name(), "subscribe");
}

#[test]
fn pico_request_rejects_invalid_selector_parameters() {
    let mut parameters = std::collections::BTreeMap::new();
    parameters.insert("reply;mode".into(), "summary".into());

    let request = ZenohPicoRequest {
        key_expr: "clinkz/things/lamp/actions/reboot",
        kind: ZenohOperationKind::RequestReply,
        metadata: &ZenohFormMetadata::default(),
        payload: None,
        parameters: &parameters,
        timeout: Duration::from_secs(1),
    };

    let err = request.selector().unwrap_err();
    assert_eq!(err.kind(), ZenohPicoErrorKind::Request);
    assert_eq!(err.code(), None);
    assert_eq!(
        err.message(),
        "Zenoh selector parameter key 'reply;mode' contains a reserved separator"
    );
}
