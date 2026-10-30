use super::*;
use clinkz_wot_core::{CoreResult, notification::ConsumerNotificationBinding};
use clinkz_wot_foundation::{WorkBudget, WorkClass};
use clinkz_wot_planning::{
    open_consumer::*,
    producer_composition::{FormSite, ProducerOperationInput},
};
impl ConsumerNotificationPreparation for MqttNotificationConsumer {
    fn registration_identity(&self) -> clinkz_wot_core::BindingRegistrationIdentity {
        consumer_identity(self)
    }
    fn preparation_bounds(&self) -> NotificationPreparationBounds {
        NotificationPreparationBounds {
            work: 4096,
            temporary_bytes: 8192,
            retained_bytes: 1024,
        }
    }
    fn compile_notification(
        &self,
        input: ConsumerNotificationInput<'_>,
    ) -> CoreResult<Option<MqttConsumerProgram>> {
        // Same supported description as the Producer; Consumer application-data
        // roles are still captured independently by Planning and execution.
        if input.schemas.subscription().is_some()
            || input.schemas.cancellation().is_some()
            || input.schemas.data_response().is_some()
            || input.schemas.data().is_none()
        {
            return Err(unsupported());
        }
        let site = match input.target {
            AffordanceTarget::Property(n) => FormSite::Property(n.to_string()),
            AffordanceTarget::Event(n) => FormSite::Event(n.to_string()),
            _ => return Err(unsupported()),
        };
        let kind = NotificationKind::for_target(input.target).unwrap();
        if input.schemas.kind() != kind {
            return Err(unsupported());
        }
        let mut budget = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 4096);
        let Some((topic, schema)) = preparation::lower_notification(
            self.configuration().address,
            ProducerOperationInput {
                document: input.document,
                site: &site,
                form: input.form,
                operation: kind.start_operation(),
                target: input.endpoint,
            },
            &mut budget,
        )?
        else {
            return Ok(None);
        };
        if Some(schema) != input.schemas.data() {
            return Err(unsupported());
        }
        let cancellation = input
            .document
            .form(
                match input.target {
                    AffordanceTarget::Property(n) => {
                        clinkz_wot_td::semantic::FormOwner::Property(n)
                    }
                    AffordanceTarget::Event(n) => clinkz_wot_td::semantic::FormOwner::Event(n),
                    _ => unreachable!(),
                },
                input.form,
            )
            .is_some_and(|f| f.effective_operations.contains(&kind.stop_operation()));
        Ok(Some(consumer_program(self, topic, cancellation, kind)))
    }
}
