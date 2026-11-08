//! Bounded Host MQTT 3.1.1 Producer and Consumer Property/Event mappings.
//! Native sharing/distribution stays here; each Engine conversation is separate.
//! No reconnect/replay or reciprocal data responses are supplied.
mod native;
pub use native::*;
mod consumer;
pub use consumer::*;
#[cfg(feature = "prepare")]
mod consumer_preparation;
#[cfg(feature = "prepare")]
mod preparation;

use clinkz_wot_core::{
    AffordanceTarget, BindingGeneration, BindingId, CoreError, ErrorContext, ErrorPhase, Operation,
    RetryClass, data_schema::PreparedSchema, notification::NotificationKind,
};
use std::{net::SocketAddr, rc::Rc, sync::Arc};

pub const MAX_NOTIFICATION_BYTES: usize = 4096;
pub const MAX_TOPIC_BYTES: usize = 128;

/// Explicit native policy. Local NoSec endpoints only; no DNS, credentials or
/// TLS are implied. Each progress call writes at most write_bytes_per_step.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MqttProducerConfig {
    address: SocketAddr,
    write_bytes_per_step: usize,
}
impl MqttProducerConfig {
    pub fn for_loopback(
        address: SocketAddr,
        write_bytes_per_step: usize,
    ) -> Result<Self, CoreError> {
        if !address.ip().is_loopback()
            || address.port() == 0
            || !(1..=8192).contains(&write_bytes_per_step)
        {
            return Err(unsupported());
        }
        Ok(Self {
            address,
            write_bytes_per_step,
        })
    }
}

struct Registration {
    config: MqttProducerConfig,
    binding: (BindingId, BindingGeneration),
}

/// Actual immutable startup registration; shared native services retain this
/// exact owner. Preparation and runtime cannot pair only by protocol/id tag.
#[derive(Clone)]
pub struct MqttNotificationProducer {
    registration: Rc<Registration>,
}
impl MqttNotificationProducer {
    pub fn new(config: MqttProducerConfig, id: BindingId, generation: BindingGeneration) -> Self {
        Self {
            registration: Rc::new(Registration {
                config,
                binding: (id, generation),
            }),
        }
    }
}

#[derive(Clone)]
pub struct MqttNotificationDefinition {
    registration: Rc<Registration>,
    target: AffordanceTarget,
    topic: Arc<str>,
    form: u32,
    operation: Operation,
    schemas: Arc<clinkz_wot_core::notification::NotificationSchemas>,
}

fn schema_backing(s: &clinkz_wot_core::notification::NotificationSchemas) -> usize {
    s.retained_bytes() + std::mem::size_of_val(s) + 2 * std::mem::size_of::<usize>()
}

/// Validate with the original role-specific meaning. Native attribution and
/// the collection representation remain separate from the common evaluator.
fn validate_data(
    schemas: &clinkz_wot_core::notification::NotificationSchemas,
    data: &clinkz_wot_core::publication::PublicationData,
) -> clinkz_wot_core::CoreResult<()> {
    use clinkz_wot_core::{
        notification::NotificationSchemas as S, publication::PublicationData as D,
    };
    let payload = data.payload();
    match (schemas, data) {
        (S::PropertyValue(s), D::PropertyValue(_)) => {
            clinkz_wot_codec_json::validate_payload(s, payload)
        }
        (S::Event { data: Some(s), .. }, D::EventData(_)) => {
            clinkz_wot_codec_json::validate_payload(s, payload)
        }
        (S::ObserveAllProperties(s), D::PropertyValues(_)) => {
            clinkz_wot_codec_json::validate_property_map(s, payload)
        }
        (S::SubscribeAllEvents(s), D::EventOccurrence { event, .. }) => {
            let schema = s
                .member(event)
                .and_then(|m| m.data.as_ref())
                .ok_or_else(unsupported)?;
            clinkz_wot_codec_json::validate_payload(schema, payload)
        }
        _ => Err(unsupported()),
    }
}

/// Only exact filters and a terminal single-level wildcard are supported.
/// Reject overlapping distinct interests rather than invent duplicate delivery
/// suppression or competing-consumer semantics.
fn topic_matches(filter: &str, topic: &str) -> bool {
    if let Some(prefix) = filter.strip_suffix("/+") {
        topic
            .strip_prefix(prefix)
            .and_then(|s| s.strip_prefix('/'))
            .is_some_and(|name| !name.contains('/'))
    } else {
        filter == topic
    }
}
fn filters_overlap(a: &str, b: &str) -> bool {
    topic_matches(a, b) || topic_matches(b, a)
}
impl PartialEq for MqttNotificationDefinition {
    fn eq(&self, b: &Self) -> bool {
        Rc::ptr_eq(&self.registration, &b.registration)
            && self.target == b.target
            && self.topic == b.topic
            && self.form == b.form
            && self.operation == b.operation
            && self.schemas == b.schemas
    }
}
impl Eq for MqttNotificationDefinition {}

fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(
        ErrorContext::new(ErrorPhase::Admission, RetryClass::Never).with_redacted_cause(
            650,
            "unsupported MQTT notification description or configuration",
        ),
    )
}
fn io_error() -> CoreError {
    CoreError::Binding(
        ErrorContext::new(ErrorPhase::Binding, RetryClass::Never)
            .with_redacted_cause(651, "MQTT native connection failed; no automatic replay"),
    )
}

fn target_site(target: &AffordanceTarget) -> clinkz_wot_core::thing::AffordanceSite<'_> {
    use clinkz_wot_core::thing::AffordanceSite;
    match target {
        AffordanceTarget::Thing => AffordanceSite::Thing,
        AffordanceTarget::Property(n) => AffordanceSite::Property(n),
        AffordanceTarget::Event(n) => AffordanceSite::Event(n),
        _ => unreachable!("only qualified notification targets"),
    }
}
fn target_name_bytes(target: &AffordanceTarget) -> usize {
    match target {
        AffordanceTarget::Thing => 0,
        AffordanceTarget::Property(n) | AffordanceTarget::Event(n) => n.len(),
        _ => unreachable!(),
    }
}
