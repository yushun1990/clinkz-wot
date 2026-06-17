use alloc::{
    format,
    string::{String, ToString},
};
use core::time::Duration;

use crate::ZenohTransportRequest;
use crate::{ZenohFormMetadata, ZenohOperationKind, ZenohOperationPlan, ZenohTransport};
use clinkz_wot_core::{CoreError, CoreResult, InteractionOutput};
use zenoh::{
    bytes::Encoding, handlers::FifoChannelHandler, pubsub::Subscriber, sample::Sample, Wait,
};

mod metadata;
mod sample;

use self::{
    metadata::{parse_congestion_control, parse_express_qos, parse_priority},
    sample::payload_from_sample,
};
use super::selector::selector_with_parameters;

const DEFAULT_REPLY_TIMEOUT: Duration = Duration::from_secs(5);

type DefaultZenohSubscriber = Subscriber<FifoChannelHandler<Sample>>;

/// Transport backed by a concrete Rust `zenoh` session.
///
/// This adapter is available only with the `zenoh` feature. It keeps the
/// default zenoh binding crate usable as `no_std + alloc` planning code while
/// giving runtimes a first concrete Rust `zenoh` execution path.
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
        if let Some(content_type) = request.plan.metadata.content_type.as_deref() {
            builder = builder.encoding(Encoding::from(content_type));
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
        if let Some(content_type) = request.plan.metadata.content_type.as_deref() {
            builder = builder.encoding(Encoding::from(content_type));
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
            &sample, None,
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
            content_type_hint: metadata.content_type,
            reply_timeout: self.reply_timeout,
        })
    }
}

fn transport_error(error: impl core::fmt::Display) -> CoreError {
    CoreError::Transport(error.to_string())
}

/// Shareable zenoh transport handle for std runtime integrations.
///
/// This wrapper lets binding factories clone a handle to the same underlying
/// session, connection pool, or runtime adapter while each `ZenohBinding`
/// still owns its protocol binding value.
#[derive(Debug)]
pub struct SharedZenohTransport<T> {
    inner: std::sync::Arc<std::sync::Mutex<T>>,
}

impl<T> SharedZenohTransport<T> {
    /// Creates a shared transport handle from a concrete transport adapter.
    pub fn new(transport: T) -> Self {
        Self {
            inner: std::sync::Arc::new(std::sync::Mutex::new(transport)),
        }
    }

    /// Creates a shared transport handle from an existing `Arc<Mutex<T>>`.
    pub fn from_arc(inner: std::sync::Arc<std::sync::Mutex<T>>) -> Self {
        Self { inner }
    }

    /// Returns the underlying shared transport container.
    pub fn inner(&self) -> &std::sync::Arc<std::sync::Mutex<T>> {
        &self.inner
    }
}

impl<T> Clone for SharedZenohTransport<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> ZenohTransport for SharedZenohTransport<T>
where
    T: ZenohTransport,
{
    fn execute(&mut self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput> {
        self.inner
            .lock()
            .map_err(|_| CoreError::Transport("Zenoh shared transport lock is poisoned".into()))?
            .execute(request)
    }
}
