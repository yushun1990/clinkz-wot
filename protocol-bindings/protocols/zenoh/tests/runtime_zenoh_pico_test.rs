#![cfg(feature = "runtime-zenoh-pico")]

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
            request.key_expr,
            request.metadata.encoding.as_deref().unwrap_or(""),
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
            "query:{}:{}",
            request.key_expr,
            request
                .parameters
                .get("trace")
                .map(String::as_str)
                .unwrap_or("")
        ));
        self.take_error()?;
        Ok(self.query_reply.take())
    }

    fn subscribe(
        &mut self,
        request: ZenohPicoRequest<'_>,
    ) -> Result<Option<Payload>, ZenohPicoError> {
        self.calls.push(format!("subscribe:{}", request.key_expr));
        self.take_error()?;
        Ok(self.subscription_reply.take())
    }

    fn unsubscribe(&mut self, request: ZenohPicoRequest<'_>) -> Result<(), ZenohPicoError> {
        self.calls.push(format!("unsubscribe:{}", request.key_expr));
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
            plan: ZenohOperationPlan {
                key_expr: "clinkz/things/lamp/status".into(),
                kind: ZenohOperationKind::Put,
                metadata: ZenohFormMetadata {
                    encoding: Some("text/plain".into()),
                    ..Default::default()
                },
            },
            payload: Some(Payload::new(b"on".to_vec(), "text/plain")),
            parameters: Default::default(),
        })
        .unwrap();

    assert!(put.payload.is_none());

    let mut query_parameters = std::collections::BTreeMap::new();
    query_parameters.insert("trace".into(), "full".into());
    let query = transport
        .execute(ZenohTransportRequest {
            plan: ZenohOperationPlan {
                key_expr: "clinkz/things/lamp/status".into(),
                kind: ZenohOperationKind::RequestReply,
                metadata: ZenohFormMetadata::default(),
            },
            payload: None,
            parameters: query_parameters,
        })
        .unwrap();

    assert_eq!(query.payload.unwrap().body, b"query-reply");
    assert_eq!(
        transport.platform().calls,
        [
            "put:clinkz/things/lamp/status:text/plain:2:250",
            "query:clinkz/things/lamp/status:full"
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
            plan: ZenohOperationPlan {
                key_expr: "clinkz/things/lamp/events/status".into(),
                kind: ZenohOperationKind::Subscribe,
                metadata: ZenohFormMetadata::default(),
            },
            payload: None,
            parameters: Default::default(),
        })
        .unwrap();
    let unsubscribed = transport
        .execute(ZenohTransportRequest {
            plan: ZenohOperationPlan {
                key_expr: "clinkz/things/lamp/events/status".into(),
                kind: ZenohOperationKind::Unsubscribe,
                metadata: ZenohFormMetadata::default(),
            },
            payload: None,
            parameters: Default::default(),
        })
        .unwrap();

    assert_eq!(subscribed.payload.unwrap().body, b"event");
    assert!(unsubscribed.payload.is_none());
    assert_eq!(
        transport.platform().calls,
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
            plan: ZenohOperationPlan {
                key_expr: "clinkz/things/lamp/status".into(),
                kind: ZenohOperationKind::Put,
                metadata: ZenohFormMetadata::default(),
            },
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
            plan: ZenohOperationPlan {
                key_expr: "clinkz/things/lamp/status".into(),
                kind: ZenohOperationKind::Query,
                metadata: ZenohFormMetadata::default(),
            },
            payload: None,
            parameters: Default::default(),
        })
        .unwrap_err();

    assert_eq!(
        err,
        CoreError::Transport("Zenoh-pico query for 'clinkz/things/lamp/status' timed out".into())
    );
}
