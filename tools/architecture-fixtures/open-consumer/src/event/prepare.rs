//! Optional TD authoring for the two source-free Event transports.
use super::*;
use clinkz_wot_planning::open_consumer::*;

fn identity(id: u32, format: [u8; 16]) -> BindingRegistrationIdentity {
    BindingRegistrationIdentity::new(
        BindingId::new(id),
        BindingGeneration::INITIAL,
        BindingConfigurationDigest::new([id as u8; 32]),
        BindingArtifactCompatibility::new(format),
        0,
    )
}
const BOUNDS: ConsumerPreparationBounds = ConsumerPreparationBounds {
    work: 64,
    temporary_bytes: 256,
    retained_bytes: 128,
};

// The test peer offers a resource at the same loopback address over both
// transports. An explicit subprotocol requires its own wire mapping. An
// unqualified dual Form permits either, so both registrations really compete.
fn integer_event(input: &ConsumerNotificationInput<'_>) -> bool {
    input.target == &alarm()
        && input.facts.content_type == "application/json"
        && matches!(
            input.schemas.data(),
            Some(data_schema::PreparedSchema::Type(
                data_schema::ValueType::Integer
            ))
        )
        && input.schemas.subscription().is_none()
        && input.schemas.cancellation().is_none()
        && input.schemas.data_response().is_none()
        && input
            .facts
            .effective_operations
            .contains(&Operation::UnsubscribeEvent)
}

impl ConsumerNotificationPreparation for Stream {
    fn registration_identity(&self) -> BindingRegistrationIdentity {
        identity(71, *b"stream-events-01")
    }
    fn preparation_bounds(&self) -> ConsumerPreparationBounds {
        BOUNDS
    }
    fn compile_notification_form(
        &self,
        input: ConsumerNotificationInput<'_>,
    ) -> CoreResult<Option<SocketAddr>> {
        self.compilations
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if !integer_event(&input)
            || input.facts.content_coding.is_some()
            || !matches!(input.facts.subprotocol, None | Some("stream"))
            || input.endpoint != format!("testevent://{}/alarm", self.peer)
        {
            return Ok(None);
        }
        Ok(Some(self.peer))
    }
}
impl ConsumerNotificationPreparation for Datagram {
    fn registration_identity(&self) -> BindingRegistrationIdentity {
        identity(72, *b"dgram-events-v01")
    }
    fn preparation_bounds(&self) -> ConsumerPreparationBounds {
        BOUNDS
    }
    fn compile_notification_form(
        &self,
        input: ConsumerNotificationInput<'_>,
    ) -> CoreResult<Option<DatagramProgram>> {
        self.compilations.set(self.compilations.get() + 1);
        if !integer_event(&input)
            || !matches!(input.facts.subprotocol, None | Some("datagram"))
            || input.endpoint != format!("testevent://{}/alarm", self.config.peer)
        {
            return Ok(None);
        }
        let hex = match input.facts.content_coding {
            None => false,
            Some("x-test-hex") => true,
            _ => return Ok(None),
        };
        Ok(Some(DatagramProgram {
            peer: self.config.peer,
            hex,
        }))
    }
}
