//! Bounded Host MQTT 3.1.1 Producer Event mapping. No Consumer execution,
//! subscriber registry, reconnect/replay, retained messages or reciprocal data
//! responses are supplied. Mosquitto real-peer tests define the evidence scope.
mod native;
pub use native::*;
#[cfg(feature = "prepare")]
mod preparation;

use clinkz_wot_core::{
    BindingGeneration, BindingId, CoreError, ErrorContext, ErrorPhase, Operation, RetryClass,
    value_type::TypeSchema,
};
use std::{net::SocketAddr, rc::Rc, sync::Arc};

pub const MAX_EVENT_BYTES: usize = 4096;
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
pub struct MqttEventProducer {
    registration: Rc<Registration>,
}
impl MqttEventProducer {
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
pub struct MqttEventDefinition {
    registration: Rc<Registration>,
    event: Arc<str>,
    topic: Arc<str>,
    form: u32,
    operation: Operation,
    schema: TypeSchema,
}
impl PartialEq for MqttEventDefinition {
    fn eq(&self, b: &Self) -> bool {
        Rc::ptr_eq(&self.registration, &b.registration)
            && self.event == b.event
            && self.topic == b.topic
            && self.form == b.form
            && self.operation == b.operation
            && self.schema == b.schema
    }
}
impl Eq for MqttEventDefinition {}

fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(
        ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
            .with_redacted_cause(650, "unsupported MQTT Event description or configuration"),
    )
}
fn io_error() -> CoreError {
    CoreError::Binding(
        ErrorContext::new(ErrorPhase::Binding, RetryClass::Never)
            .with_redacted_cause(651, "MQTT native connection failed; no automatic replay"),
    )
}
