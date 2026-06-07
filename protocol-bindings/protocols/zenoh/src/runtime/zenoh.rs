use alloc::{
    format,
    string::{String, ToString},
};
use core::time::Duration;

use clinkz_wot_core::{CoreError, CoreResult, InteractionOutput};
use zenoh::{
    Wait, bytes::Encoding, handlers::FifoChannelHandler, pubsub::Subscriber, sample::Sample,
};

mod metadata;
mod sample;

use self::{
    metadata::{parse_congestion_control, parse_express_qos, parse_priority},
    sample::payload_from_sample,
};
use super::selector::selector_with_parameters;
use crate::{
    ZenohFormMetadata, ZenohOperationKind, ZenohOperationPlan, ZenohTransport,
    ZenohTransportRequest,
};

const DEFAULT_REPLY_TIMEOUT: Duration = Duration::from_secs(5);

type DefaultZenohSubscriber = Subscriber<FifoChannelHandler<Sample>>;

/// Transport backed by a concrete Rust `zenoh` session.
///
/// This adapter is available only with the `runtime-zenoh` feature. It keeps
/// the default zenoh binding crate usable as `no_std + alloc` planning code
/// while giving runtimes a first concrete Rust `zenoh` execution path.
#[derive(Debug, Clone)]
pub struct ZenohSessionTransport {
    session: zenoh::Session,
    reply_timeout: Duration,
}

/// Active zenoh subscription returned by [`ZenohSessionTransport`].
///
/// The binding-level [`ZenohTransport`] trait still exposes a one-shot
/// interaction result for protocol-neutral dispatch. Runtimes that need
/// explicit event lifecycle control can use this handle directly.
#[derive(Debug)]
pub struct ZenohSubscription {
    subscriber: DefaultZenohSubscriber,
    content_type_hint: Option<String>,
    reply_timeout: Duration,
}

impl ZenohSubscription {
    /// Returns the zenoh key expression this subscription listens on.
    pub fn key_expr(&self) -> &str {
        self.subscriber.key_expr().as_str()
    }

    /// Returns the content type hint applied to subscription samples, if any.
    pub fn content_type_hint(&self) -> Option<&str> {
        self.content_type_hint.as_deref()
    }

    /// Returns the default timeout used by [`ZenohSubscription::next_sample`].
    pub fn reply_timeout(&self) -> Duration {
        self.reply_timeout
    }

    /// Waits for the next subscription sample using the default runtime timeout.
    pub fn next_sample(&mut self) -> CoreResult<InteractionOutput> {
        self.next_timeout(self.reply_timeout)
    }

    /// Waits for the next subscription sample using an explicit timeout.
    pub fn next_timeout(&mut self, timeout: Duration) -> CoreResult<InteractionOutput> {
        let sample = self
            .subscriber
            .recv_timeout(timeout)
            .map_err(transport_error)?
            .ok_or_else(|| {
                CoreError::Transport(format!(
                    "Zenoh subscription for '{}' timed out",
                    self.key_expr()
                ))
            })?;

        Ok(InteractionOutput::with_payload(payload_from_sample(
            &sample,
            self.content_type_hint.as_deref(),
        )))
    }

    /// Explicitly undeclares the underlying zenoh subscriber.
    pub fn undeclare(self) -> CoreResult<()> {
        self.subscriber.undeclare().wait().map_err(transport_error)
    }
}

impl ZenohSessionTransport {
    /// Creates a transport from an existing zenoh session.
    pub fn new(session: zenoh::Session) -> Self {
        Self {
            session,
            reply_timeout: DEFAULT_REPLY_TIMEOUT,
        }
    }

    /// Opens a zenoh session from a zenoh configuration.
    pub fn open(config: zenoh::Config) -> CoreResult<Self> {
        let session = zenoh::open(config).wait().map_err(transport_error)?;
        Ok(Self::new(session))
    }

    /// Sets the maximum time to wait for one query or subscription reply.
    pub fn with_reply_timeout(mut self, reply_timeout: Duration) -> Self {
        self.reply_timeout = reply_timeout;
        self
    }

    /// Returns the underlying zenoh session.
    pub fn session(&self) -> &zenoh::Session {
        &self.session
    }

    /// Returns the configured query and subscription reply timeout.
    pub fn reply_timeout(&self) -> Duration {
        self.reply_timeout
    }

    /// Declares a long-lived zenoh subscription from a planned subscribe operation.
    pub fn subscribe(&self, plan: ZenohOperationPlan) -> CoreResult<ZenohSubscription> {
        if plan.kind != ZenohOperationKind::Subscribe {
            return Err(CoreError::UnsupportedOperation(format!(
                "Zenoh {:?} operation cannot be opened as a subscription",
                plan.kind
            )));
        }

        self.declare_subscription(plan.key_expr, plan.metadata)
    }
}

impl ZenohTransport for ZenohSessionTransport {
    fn execute(&mut self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput> {
        match request.plan.kind {
            ZenohOperationKind::Put => self.put(request),
            ZenohOperationKind::Query | ZenohOperationKind::RequestReply => self.get(request),
            ZenohOperationKind::Subscribe => self.subscribe_once(request),
            ZenohOperationKind::Unsubscribe => Ok(InteractionOutput::empty()),
        }
    }
}

impl ZenohSessionTransport {
    fn put(&self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput> {
        let body = request
            .payload
            .map(|payload| payload.body)
            .unwrap_or_default();
        let mut builder = self.session.put(request.plan.key_expr.as_str(), body);
        if let Some(encoding) = request.plan.metadata.encoding.as_deref() {
            builder = builder.encoding(Encoding::from(encoding));
        }
        if let Some(qos) = request.plan.metadata.qos.as_deref() {
            builder = builder.express(parse_express_qos(qos)?);
        }
        if let Some(priority) = request.plan.metadata.priority.as_deref() {
            builder = builder.priority(parse_priority(priority)?);
        }
        if let Some(congestion_control) = request.plan.metadata.congestion_control.as_deref() {
            builder = builder.congestion_control(parse_congestion_control(congestion_control)?);
        }
        builder.wait().map_err(transport_error)?;
        Ok(InteractionOutput::empty())
    }

    fn get(&self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput> {
        let selector = selector_with_parameters(&request.plan.key_expr, &request.parameters)?;
        let mut builder = self.session.get(selector.as_str());
        if let Some(payload) = request.payload {
            builder = builder.payload(payload.body);
        }
        if let Some(encoding) = request.plan.metadata.encoding.as_deref() {
            builder = builder.encoding(Encoding::from(encoding));
        }
        if let Some(qos) = request.plan.metadata.qos.as_deref() {
            builder = builder.express(parse_express_qos(qos)?);
        }
        if let Some(priority) = request.plan.metadata.priority.as_deref() {
            builder = builder.priority(parse_priority(priority)?);
        }
        if let Some(congestion_control) = request.plan.metadata.congestion_control.as_deref() {
            builder = builder.congestion_control(parse_congestion_control(congestion_control)?);
        }

        let replies = builder.wait().map_err(transport_error)?;
        let reply = replies
            .recv_timeout(self.reply_timeout)
            .map_err(transport_error)?
            .ok_or_else(|| {
                CoreError::Transport(format!(
                    "Zenoh query for '{}' timed out",
                    request.plan.key_expr
                ))
            })?;
        let sample = reply.into_result().map_err(transport_error)?;

        Ok(InteractionOutput::with_payload(payload_from_sample(
            &sample,
            request.plan.metadata.encoding.as_deref(),
        )))
    }

    fn subscribe_once(&self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput> {
        let mut subscription =
            self.declare_subscription(request.plan.key_expr, request.plan.metadata)?;
        let output = subscription.next_timeout(self.reply_timeout)?;
        subscription.undeclare()?;

        Ok(output)
    }

    fn declare_subscription(
        &self,
        key_expr: String,
        metadata: ZenohFormMetadata,
    ) -> CoreResult<ZenohSubscription> {
        let subscriber = self
            .session
            .declare_subscriber(key_expr.as_str())
            .wait()
            .map_err(transport_error)?;

        Ok(ZenohSubscription {
            subscriber,
            content_type_hint: metadata.encoding,
            reply_timeout: self.reply_timeout,
        })
    }
}

fn transport_error(error: impl core::fmt::Display) -> CoreError {
    CoreError::Transport(error.to_string())
}
