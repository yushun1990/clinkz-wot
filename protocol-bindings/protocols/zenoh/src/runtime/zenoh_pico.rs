//! Runtime adapter boundary for constrained `zenoh-pico` integrations.
//!
//! This module deliberately stops at a platform hook trait. C ABI bindings,
//! session ownership, polling, and buffer management stay in target-specific
//! platform code that implements [`ZenohPicoPlatform`].

use alloc::{
    borrow::Cow,
    format,
    string::{String, ToString},
};
use core::{any::Any, fmt, time::Duration};

use clinkz_wot_core::{CoreError, CoreResult, InteractionOutput, Payload};

use super::selector::selector_with_parameters;
use crate::{ZenohFormMetadata, ZenohOperationKind, ZenohTransport, ZenohTransportRequest};

const DEFAULT_REPLY_TIMEOUT: Duration = Duration::from_secs(5);

/// Borrowed request passed to a constrained zenoh-pico platform hook.
#[derive(Debug, Clone, Copy)]
pub struct ZenohPicoRequest<'a> {
    /// Zenoh key expression selected by the shared planner.
    pub key_expr: &'a str,
    /// Transport-level operation shape selected by the shared planner.
    pub kind: ZenohOperationKind,
    /// Zenoh-specific metadata parsed from TD extension terms.
    pub metadata: &'a ZenohFormMetadata,
    /// Optional encoded payload from the WoT interaction input.
    pub payload: Option<&'a Payload>,
    /// Runtime parameters supplied by the caller.
    pub parameters: &'a alloc::collections::BTreeMap<String, String>,
    /// Maximum time the platform hook should wait for a reply or sample.
    pub timeout: Duration,
}

impl<'a> ZenohPicoRequest<'a> {
    /// Builds a zenoh selector by appending validated request parameters to the
    /// selected key expression.
    pub fn selector(&self) -> Result<String, ZenohPicoError> {
        selector_with_parameters(self.key_expr, self.parameters).map_err(parameter_error)
    }

    /// Returns the wire target string for this request.
    ///
    /// Query-style operations require a zenoh selector so validated request
    /// parameters can be appended to the selected key expression. Other
    /// operations use the raw key expression directly.
    pub fn target_expr(&self) -> Result<Cow<'a, str>, ZenohPicoError> {
        match self.kind {
            ZenohOperationKind::Query | ZenohOperationKind::RequestReply => {
                self.selector().map(Cow::Owned)
            }
            ZenohOperationKind::Put
            | ZenohOperationKind::Subscribe
            | ZenohOperationKind::Unsubscribe => Ok(Cow::Borrowed(self.key_expr)),
        }
    }

    /// Returns a human-readable transport operation name for diagnostics.
    pub fn operation_name(&self) -> &'static str {
        match self.kind {
            ZenohOperationKind::Put => "put",
            ZenohOperationKind::Query => "query",
            ZenohOperationKind::RequestReply => "request-reply",
            ZenohOperationKind::Subscribe => "subscribe",
            ZenohOperationKind::Unsubscribe => "unsubscribe",
        }
    }
}

/// Error returned by a constrained zenoh-pico platform hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZenohPicoError {
    kind: ZenohPicoErrorKind,
    code: Option<i32>,
    message: String,
}

/// Category of a constrained zenoh-pico platform hook error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZenohPicoErrorKind {
    /// The target-specific integration rejected the request before a platform call.
    Request,
    /// A platform, C ABI, session, polling, or buffer-management operation failed.
    Platform,
    /// The platform hook timed out while waiting for a reply or sample.
    Timeout,
}

impl ZenohPicoError {
    /// Creates an error from a human-readable platform failure message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            kind: ZenohPicoErrorKind::Platform,
            code: None,
            message: message.into(),
        }
    }

    /// Creates an error for a rejected request before platform execution starts.
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            kind: ZenohPicoErrorKind::Request,
            code: None,
            message: message.into(),
        }
    }

    /// Creates an error from a zenoh-pico status code and message.
    pub fn with_code(code: i32, message: impl Into<String>) -> Self {
        Self {
            kind: ZenohPicoErrorKind::Platform,
            code: Some(code),
            message: message.into(),
        }
    }

    /// Creates an error for a platform hook timeout.
    pub fn timeout(operation: &str, key_expr: &str) -> Self {
        Self {
            kind: ZenohPicoErrorKind::Timeout,
            code: None,
            message: timeout_message(operation, key_expr),
        }
    }

    /// Returns the error category.
    pub fn kind(&self) -> ZenohPicoErrorKind {
        self.kind
    }

    /// Returns whether this error represents a timeout.
    pub fn is_timeout(&self) -> bool {
        self.kind == ZenohPicoErrorKind::Timeout
    }

    /// Returns the platform status code, if one was supplied.
    pub fn code(&self) -> Option<i32> {
        self.code
    }

    /// Returns the human-readable platform error message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ZenohPicoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.code {
            Some(code) => write!(f, "zenoh-pico status {}: {}", code, self.message),
            None => f.write_str(&self.message),
        }
    }
}

/// Platform hook contract for constrained zenoh-pico execution.
///
/// Implementations own the real zenoh-pico session, C ABI calls, polling,
/// timeout handling, and buffer ownership. The transport adapter only maps
/// shared planner output to these hooks.
pub trait ZenohPicoPlatform: Any {
    /// Executes a put-style operation.
    fn put(&mut self, request: ZenohPicoRequest<'_>) -> Result<(), ZenohPicoError>;

    /// Executes a query or request/reply operation.
    fn query(&mut self, request: ZenohPicoRequest<'_>) -> Result<Option<Payload>, ZenohPicoError>;

    /// Opens a subscription, waits for one sample, and leaves lifecycle policy
    /// to the platform hook.
    fn subscribe(
        &mut self,
        request: ZenohPicoRequest<'_>,
    ) -> Result<Option<Payload>, ZenohPicoError>;

    /// Executes a subscription cancellation.
    fn unsubscribe(&mut self, request: ZenohPicoRequest<'_>) -> Result<(), ZenohPicoError>;
}

impl dyn ZenohPicoPlatform {
    /// Downcasts the platform hook to a concrete implementation for tests and diagnostics.
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: ZenohPicoPlatform + 'static,
    {
        (self as &dyn Any).downcast_ref::<T>()
    }

    /// Downcasts the platform hook to a mutable concrete implementation for tests and diagnostics.
    pub fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: ZenohPicoPlatform + 'static,
    {
        (self as &mut dyn Any).downcast_mut::<T>()
    }
}

/// Transport backed by constrained zenoh-pico platform hooks.
pub struct ZenohPicoTransport {
    platform: alloc::boxed::Box<dyn ZenohPicoPlatform>,
    reply_timeout: Duration,
}

impl fmt::Debug for ZenohPicoTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ZenohPicoTransport")
            .field("platform", &"<dyn ZenohPicoPlatform>")
            .field("reply_timeout", &self.reply_timeout)
            .finish()
    }
}

impl ZenohPicoTransport {
    /// Creates a transport from target-specific platform hooks.
    pub fn new(platform: impl ZenohPicoPlatform + 'static) -> Self {
        Self {
            platform: alloc::boxed::Box::new(platform),
            reply_timeout: DEFAULT_REPLY_TIMEOUT,
        }
    }

    /// Sets the maximum time to wait for one query or subscription sample.
    pub fn with_reply_timeout(mut self, reply_timeout: Duration) -> Self {
        self.reply_timeout = reply_timeout;
        self
    }

    /// Returns the underlying platform hook implementation.
    pub fn platform(&self) -> &dyn ZenohPicoPlatform {
        self.platform.as_ref()
    }

    /// Returns a mutable reference to the underlying platform hook implementation.
    pub fn platform_mut(&mut self) -> &mut dyn ZenohPicoPlatform {
        self.platform.as_mut()
    }

    /// Returns the configured query and subscription reply timeout.
    pub fn reply_timeout(&self) -> Duration {
        self.reply_timeout
    }
}

impl ZenohTransport for ZenohPicoTransport {
    fn execute(&mut self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput> {
        match request.plan.kind {
            ZenohOperationKind::Put => {
                let pico_request = self.pico_request(&request);
                self.platform.put(pico_request).map_err(transport_error)?;
                Ok(InteractionOutput::empty())
            }
            ZenohOperationKind::Query | ZenohOperationKind::RequestReply => {
                let pico_request = self.pico_request(&request);
                let payload = self
                    .platform
                    .query(pico_request)
                    .map_err(transport_error)?
                    .ok_or_else(|| timeout_error("query", &request.plan.key_expr))?;
                Ok(InteractionOutput::with_payload(payload))
            }
            ZenohOperationKind::Subscribe => {
                let pico_request = self.pico_request(&request);
                let payload = self
                    .platform
                    .subscribe(pico_request)
                    .map_err(transport_error)?
                    .ok_or_else(|| timeout_error("subscription", &request.plan.key_expr))?;
                Ok(InteractionOutput::with_payload(payload))
            }
            ZenohOperationKind::Unsubscribe => {
                let pico_request = self.pico_request(&request);
                self.platform
                    .unsubscribe(pico_request)
                    .map_err(transport_error)?;
                Ok(InteractionOutput::empty())
            }
        }
    }
}

impl ZenohPicoTransport {
    fn pico_request<'a>(&self, request: &'a ZenohTransportRequest) -> ZenohPicoRequest<'a> {
        ZenohPicoRequest {
            key_expr: request.plan.key_expr.as_str(),
            kind: request.plan.kind,
            metadata: &request.plan.metadata,
            payload: request.payload.as_ref(),
            parameters: &request.parameters,
            timeout: self.reply_timeout,
        }
    }
}

fn transport_error(error: ZenohPicoError) -> CoreError {
    CoreError::Transport(error.to_string())
}

fn timeout_error(operation: &str, key_expr: &str) -> CoreError {
    CoreError::Transport(timeout_message(operation, key_expr))
}

fn timeout_message(operation: &str, key_expr: &str) -> String {
    format!("Zenoh-pico {} for '{}' timed out", operation, key_expr)
}

fn parameter_error(error: CoreError) -> ZenohPicoError {
    match error {
        CoreError::Transport(message) => ZenohPicoError::invalid_request(message),
        _ => ZenohPicoError::invalid_request(error.to_string()),
    }
}
