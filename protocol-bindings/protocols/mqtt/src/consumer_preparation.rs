use super::*;
use clinkz_wot_core::{
    CoreResult, notification::ConsumerNotificationBinding, notification::NotificationSchemas,
};
use clinkz_wot_foundation::{WorkBudget, WorkClass};
use clinkz_wot_planning::{
    open_consumer::*,
    producer_composition::{FormSite, ProducerOperationInput},
};
impl ConsumerNotificationPreparation for MqttNotificationConsumer {
    fn registration_identity(&self) -> clinkz_wot_core::BindingRegistrationIdentity {
        consumer_identity(self)
    }
    fn preparation_bounds(&self) -> ConsumerPreparationBounds {
        ConsumerPreparationBounds {
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
                && !matches!(
                    input.schemas,
                    NotificationSchemas::ObserveAllProperties(_)
                        | NotificationSchemas::SubscribeAllEvents(_)
                )
        {
            return Err(unsupported());
        }
        let site = match input.target {
            AffordanceTarget::Thing => FormSite::Thing,
            AffordanceTarget::Property(n) => FormSite::Property(n.to_string()),
            AffordanceTarget::Event(n) => FormSite::Event(n.to_string()),
            _ => return Err(unsupported()),
        };
        let kind = input.schemas.kind();
        if !kind.accepts_target(input.target) {
            return Err(unsupported());
        }
        let mut budget = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 4096);
        let Some(topic) = preparation::lower_notification(
            self.configuration().address,
            ProducerOperationInput {
                document: input.document,
                site: &site,
                form: input.form,
                operation: kind.start_operation(),
                target: input.endpoint,
            },
            input.schemas,
            &mut budget,
        )?
        else {
            return Ok(None);
        };
        let cancellation = input
            .document
            .form(
                match input.target {
                    AffordanceTarget::Thing => clinkz_wot_td::semantic::FormOwner::Thing,
                    AffordanceTarget::Property(n) => {
                        clinkz_wot_td::semantic::FormOwner::Property(n)
                    }
                    AffordanceTarget::Event(n) => clinkz_wot_td::semantic::FormOwner::Event(n),
                    _ => unreachable!(),
                },
                input.form,
            )
            .is_some_and(|f| f.effective_operations.contains(&kind.stop_operation()));
        Ok(Some(consumer_program(
            self,
            topic,
            cancellation,
            kind,
            input.target.clone(),
        )))
    }
}
