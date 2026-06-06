use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::time::Duration;

use clinkz_wot_core::{CoreError, CoreResult, InteractionOutput, Payload};
use zenoh::{
    Wait,
    bytes::{Encoding, ZBytes},
    qos::{CongestionControl, Priority},
    sample::Sample,
};

use crate::{ZenohOperationKind, ZenohTransport, ZenohTransportRequest};

const DEFAULT_REPLY_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_CONTENT_TYPE: &str = "application/octet-stream";

/// Host transport backed by a concrete Rust `zenoh` session.
///
/// This adapter is available only with the `zenoh-runtime` feature. It keeps
/// the default zenoh binding crate usable as `no_std + alloc` planning code
/// while giving host runtimes a first concrete execution path.
#[derive(Debug, Clone)]
pub struct ZenohSessionTransport {
    session: zenoh::Session,
    reply_timeout: Duration,
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
        let mut builder = self.session.get(request.plan.key_expr.as_str());
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
        let subscriber = self
            .session
            .declare_subscriber(request.plan.key_expr.as_str())
            .wait()
            .map_err(transport_error)?;
        let sample = subscriber
            .recv_timeout(self.reply_timeout)
            .map_err(transport_error)?
            .ok_or_else(|| {
                CoreError::Transport(format!(
                    "Zenoh subscription for '{}' timed out",
                    request.plan.key_expr
                ))
            })?;
        self.session
            .undeclare(subscriber)
            .wait()
            .map_err(transport_error)?;

        Ok(InteractionOutput::with_payload(payload_from_sample(
            &sample,
            request.plan.metadata.encoding.as_deref(),
        )))
    }
}

fn payload_from_sample(sample: &Sample, content_type_hint: Option<&str>) -> Payload {
    let content_type = content_type_hint
        .map(ToString::to_string)
        .unwrap_or_else(|| sample.encoding().to_string());
    let content_type = if content_type.is_empty() {
        DEFAULT_CONTENT_TYPE.into()
    } else {
        content_type
    };

    Payload::new(bytes_from_zbytes(sample.payload()), content_type)
}

fn bytes_from_zbytes(bytes: &ZBytes) -> Vec<u8> {
    bytes.to_bytes().into_owned()
}

fn transport_error(error: impl core::fmt::Display) -> CoreError {
    CoreError::Transport(error.to_string())
}

fn parse_express_qos(value: &str) -> CoreResult<bool> {
    match normalized_metadata_value(value).as_str() {
        "express" | "true" | "yes" | "1" => Ok(true),
        "normal" | "default" | "false" | "no" | "0" => Ok(false),
        _ => Err(unsupported_metadata("cz-zenoh:qos", value)),
    }
}

fn parse_priority(value: &str) -> CoreResult<Priority> {
    match normalized_metadata_value(value).as_str() {
        "real-time" | "realtime" | "real_time" => Ok(Priority::RealTime),
        "interactive-high" | "interactivehigh" | "interactive_high" => {
            Ok(Priority::InteractiveHigh)
        }
        "interactive-low" | "interactivelow" | "interactive_low" => Ok(Priority::InteractiveLow),
        "data-high" | "datahigh" | "data_high" => Ok(Priority::DataHigh),
        "data" | "default" => Ok(Priority::Data),
        "data-low" | "datalow" | "data_low" => Ok(Priority::DataLow),
        "background" => Ok(Priority::Background),
        _ => Err(unsupported_metadata("cz-zenoh:priority", value)),
    }
}

fn parse_congestion_control(value: &str) -> CoreResult<CongestionControl> {
    match normalized_metadata_value(value).as_str() {
        "drop" => Ok(CongestionControl::Drop),
        "block" => Ok(CongestionControl::Block),
        _ => Err(unsupported_metadata("cz-zenoh:congestionControl", value)),
    }
}

fn normalized_metadata_value(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn unsupported_metadata(term: &str, value: &str) -> CoreError {
    CoreError::Transport(format!(
        "Unsupported zenoh metadata {} value '{}'",
        term, value
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_express_qos_metadata() {
        assert!(parse_express_qos("express").unwrap());
        assert!(parse_express_qos("true").unwrap());
        assert!(!parse_express_qos("normal").unwrap());
        assert!(!parse_express_qos("default").unwrap());
    }

    #[test]
    fn rejects_unknown_qos_metadata() {
        let err = parse_express_qos("guaranteed").unwrap_err();

        assert_eq!(
            err,
            CoreError::Transport(
                "Unsupported zenoh metadata cz-zenoh:qos value 'guaranteed'".into()
            )
        );
    }

    #[test]
    fn parses_priority_metadata() {
        assert_eq!(parse_priority("real-time").unwrap(), Priority::RealTime);
        assert_eq!(
            parse_priority("interactive-high").unwrap(),
            Priority::InteractiveHigh
        );
        assert_eq!(parse_priority("data").unwrap(), Priority::Data);
        assert_eq!(parse_priority("background").unwrap(), Priority::Background);
    }

    #[test]
    fn parses_congestion_control_metadata() {
        assert_eq!(
            parse_congestion_control("drop").unwrap(),
            CongestionControl::Drop
        );
        assert_eq!(
            parse_congestion_control("block").unwrap(),
            CongestionControl::Block
        );
    }
}
