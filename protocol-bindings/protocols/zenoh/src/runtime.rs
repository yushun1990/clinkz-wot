use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::time::Duration;

use clinkz_wot_core::{CoreError, CoreResult, InteractionOutput, Payload};
use zenoh::{
    Wait,
    bytes::{Encoding, ZBytes},
    handlers::FifoChannelHandler,
    pubsub::Subscriber,
    qos::{CongestionControl, Priority},
    sample::Sample,
};

use crate::{
    ZenohFormMetadata, ZenohOperationKind, ZenohOperationPlan, ZenohTransport,
    ZenohTransportRequest,
};

const DEFAULT_REPLY_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_CONTENT_TYPE: &str = "application/octet-stream";

type DefaultZenohSubscriber = Subscriber<FifoChannelHandler<Sample>>;

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

/// Active host zenoh subscription returned by [`ZenohSessionTransport`].
///
/// The binding-level [`ZenohTransport`] trait still exposes a one-shot
/// interaction result for protocol-neutral dispatch. Host runtimes that need
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

    /// Waits for the next subscription sample using the default runtime timeout.
    pub fn next(&mut self) -> CoreResult<InteractionOutput> {
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

fn selector_with_parameters(
    key_expr: &str,
    parameters: &BTreeMap<String, String>,
) -> CoreResult<String> {
    if parameters.is_empty() {
        return Ok(key_expr.into());
    }

    let mut selector = String::with_capacity(key_expr.len() + encoded_parameters_len(parameters));
    selector.push_str(key_expr);
    if let Some(separator) = selector_parameter_separator(key_expr)? {
        selector.push(separator);
    }

    let mut first = true;
    for (key, value) in parameters {
        validate_selector_parameter("key", key)?;
        validate_selector_parameter("value", value)?;

        if !first {
            selector.push(';');
        }
        selector.push_str(key);
        if !value.is_empty() {
            selector.push('=');
            selector.push_str(value);
        }
        first = false;
    }

    Ok(selector)
}

fn selector_parameter_separator(key_expr: &str) -> CoreResult<Option<char>> {
    let mut parameter_separator_count = 0;
    for char_ in key_expr.chars() {
        if char_ == '?' {
            parameter_separator_count += 1;
        }
    }

    match parameter_separator_count {
        0 => Ok(Some('?')),
        1 if key_expr.ends_with(['?', ';']) => Ok(None),
        1 => Ok(Some(';')),
        _ => Err(CoreError::Transport(format!(
            "Zenoh selector '{}' contains multiple parameter separators",
            key_expr
        ))),
    }
}

fn encoded_parameters_len(parameters: &BTreeMap<String, String>) -> usize {
    let mut len = 1;
    for (key, value) in parameters {
        len += key.len() + 1;
        if !value.is_empty() {
            len += value.len() + 1;
        }
    }
    len
}

fn validate_selector_parameter(kind: &str, value: &str) -> CoreResult<()> {
    if value.contains(['?', ';', '=', '|']) {
        return Err(CoreError::Transport(format!(
            "Zenoh selector parameter {} '{}' contains a reserved separator",
            kind, value
        )));
    }

    Ok(())
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
    use alloc::collections::BTreeMap;

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

    #[test]
    fn builds_selector_with_request_parameters() {
        let mut parameters = BTreeMap::new();
        parameters.insert("reply".into(), "full".into());
        parameters.insert("trace".into(), String::new());

        let selector =
            selector_with_parameters("clinkz/things/lamp/actions/reboot", &parameters).unwrap();

        assert_eq!(
            selector,
            "clinkz/things/lamp/actions/reboot?reply=full;trace"
        );
    }

    #[test]
    fn appends_request_parameters_to_existing_selector_parameters() {
        let mut parameters = BTreeMap::new();
        parameters.insert("trace".into(), "true".into());

        let selector = selector_with_parameters(
            "clinkz/things/lamp/actions/reboot?reply=summary",
            &parameters,
        )
        .unwrap();

        assert_eq!(
            selector,
            "clinkz/things/lamp/actions/reboot?reply=summary;trace=true"
        );
    }

    #[test]
    fn appends_request_parameters_to_open_selector_parameter_list() {
        let mut parameters = BTreeMap::new();
        parameters.insert("trace".into(), "true".into());

        let selector =
            selector_with_parameters("clinkz/things/lamp/actions/reboot?", &parameters).unwrap();

        assert_eq!(selector, "clinkz/things/lamp/actions/reboot?trace=true");
    }

    #[test]
    fn rejects_selectors_with_multiple_parameter_separators() {
        let mut parameters = BTreeMap::new();
        parameters.insert("trace".into(), "true".into());

        let err = selector_with_parameters(
            "clinkz/things/lamp/actions/reboot?reply=summary?trace=false",
            &parameters,
        )
        .unwrap_err();

        assert_eq!(
            err,
            CoreError::Transport(
                "Zenoh selector 'clinkz/things/lamp/actions/reboot?reply=summary?trace=false' contains multiple parameter separators".into()
            )
        );
    }

    #[test]
    fn returns_plain_selector_without_request_parameters() {
        let selector =
            selector_with_parameters("clinkz/things/lamp/properties/status", &BTreeMap::new())
                .unwrap();

        assert_eq!(selector, "clinkz/things/lamp/properties/status");
    }

    #[test]
    fn rejects_ambiguous_selector_parameter_keys() {
        let mut parameters = BTreeMap::new();
        parameters.insert("reply;mode".into(), "full".into());

        let err =
            selector_with_parameters("clinkz/things/lamp/actions/reboot", &parameters).unwrap_err();

        assert_eq!(
            err,
            CoreError::Transport(
                "Zenoh selector parameter key 'reply;mode' contains a reserved separator".into()
            )
        );
    }

    #[test]
    fn rejects_ambiguous_selector_parameter_values() {
        let mut parameters = BTreeMap::new();
        parameters.insert("reply".into(), "full;trace=true".into());

        let err =
            selector_with_parameters("clinkz/things/lamp/actions/reboot", &parameters).unwrap_err();

        assert_eq!(
            err,
            CoreError::Transport(
                "Zenoh selector parameter value 'full;trace=true' contains a reserved separator"
                    .into()
            )
        );
    }

    #[test]
    fn parses_metadata_case_and_whitespace_variants() {
        assert!(parse_express_qos(" YES ").unwrap());
        assert!(!parse_express_qos(" No ").unwrap());
        assert_eq!(parse_priority("DATA_HIGH").unwrap(), Priority::DataHigh);
        assert_eq!(parse_priority("data-low").unwrap(), Priority::DataLow);
        assert_eq!(
            parse_congestion_control(" BLOCK ").unwrap(),
            CongestionControl::Block
        );
    }

    #[test]
    fn rejects_unknown_priority_metadata() {
        let err = parse_priority("urgent").unwrap_err();

        assert_eq!(
            err,
            CoreError::Transport(
                "Unsupported zenoh metadata cz-zenoh:priority value 'urgent'".into()
            )
        );
    }

    #[test]
    fn rejects_unknown_congestion_control_metadata() {
        let err = parse_congestion_control("queue").unwrap_err();

        assert_eq!(
            err,
            CoreError::Transport(
                "Unsupported zenoh metadata cz-zenoh:congestionControl value 'queue'".into()
            )
        );
    }
}
