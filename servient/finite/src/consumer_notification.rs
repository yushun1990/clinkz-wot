//! TD-prepared notification services over the existing raw conversation owner.
//! No TD, compiler, protocol matching or native distribution lives here.
use crate::notification::*;
use alloc::{rc::Rc, vec::Vec};
use clinkz_wot_core::{consumer::*, notification::*, *};
use clinkz_wot_foundation::{ResourceKind as K, WorkBudget, WorkClass as W};
use core::task::Context;

struct CapturedServices {
    target: AffordanceTarget,
    endpoint: alloc::boxed::Box<str>,
    binding: BindingRegistrationIdentity,
    consumer: ConsumerInstanceRef,
    services: ConsumerServices,
    schemas: ConsumerNotificationSchemas,
    policy: ConsumerDataPolicy,
    access_work: u64,
}
struct CapturedAuthorizer(Rc<CapturedServices>);
impl NotificationAuthorizer for CapturedAuthorizer {
    fn work(&self) -> u64 {
        self.0.access_work
    }
    fn authorize(
        &self,
        id: &NotificationIdentity,
        target: &AffordanceTarget,
        permission: NotificationPermission,
    ) -> CoreResult<()> {
        let captured = &self.0;
        if captured.services.access.work() != captured.access_work {
            return Err(invalid());
        }
        if id.consumer != captured.consumer
            || !captured.consumer.is_current()
            || target != &captured.target
        {
            return Err(CoreError::StaleHandle(context()));
        }
        let operation = match permission {
            NotificationPermission::Cancel => Operation::UnsubscribeEvent,
            _ => Operation::SubscribeEvent,
        };
        captured.services.access.authorize_instance(
            &id.consumer,
            captured.binding,
            &captured.endpoint,
            operation,
        )
    }
}
impl CapturedServices {
    fn validate(
        &self,
        schema: Option<clinkz_wot_core::value_type::TypeSchema>,
        payload: Option<&Payload>,
    ) -> CoreResult<()> {
        match (schema, payload) {
            (None, None) => Ok(()),
            (Some(schema), Some(payload))
                if self.services.data.available()
                    && self.services.data.policy() == self.policy
                    && self.services.data.supports_schema(schema)
                    && payload.body.len() as u64 <= self.policy.body_bytes =>
            {
                self.services.data.validate(schema, payload)
            }
            _ => Err(invalid()),
        }
    }
    fn prepay(&self, budget: &mut WorkBudget) -> CoreResult<()> {
        if !pay(budget, W::CodecInputBytes, self.policy.validation_work) {
            return Err(busy());
        }
        Ok(())
    }
}
impl NotificationValidation for CapturedServices {
    fn subscription(&self, input: &SubscriptionInput, budget: &mut WorkBudget) -> CoreResult<()> {
        if self.schemas.subscription.is_some() {
            self.prepay(budget)?;
        }
        self.validate(self.schemas.subscription, input.0.as_ref())
    }
    fn cancellation(&self, input: &CancellationInput, budget: &mut WorkBudget) -> CoreResult<()> {
        if self.schemas.cancellation.is_some() {
            self.prepay(budget)?;
        }
        self.validate(self.schemas.cancellation, input.0.as_ref())
    }
    fn response(&self, input: &EventDataResponse, budget: &mut WorkBudget) -> CoreResult<()> {
        if self.schemas.data_response.is_some() {
            self.prepay(budget)?;
        }
        self.validate(self.schemas.data_response, input.0.as_ref())
    }
    fn prepay_data(&self, budget: &mut WorkBudget) -> CoreResult<()> {
        if self.schemas.data.is_some() {
            self.prepay(budget)?;
        }
        Ok(())
    }
    fn data(&self, data: Option<&Payload>) -> CoreResult<()> {
        self.validate(self.schemas.data, data)
    }
}

/// One reusable selected Event definition, with independent logical cursors.
/// Several such facets may share one ConsumerInstance and native Binding service.
pub struct ConsumerNotificationRuntime<B: ConsumerNotificationBinding> {
    root: RawNotificationServient<B, CapturedAuthorizer>,
    definition: ConsumerNotificationDefinition<B>,
    // Root, native participants, programs and actual services drop first.
    _funding: Vec<clinkz_wot_core::host_accounts::HostCapacityReservation>,
}
impl<B: ConsumerNotificationBinding + Clone> ConsumerNotificationRuntime<B>
where
    B::Program: Clone,
    B::Configuration: Clone,
{
    pub fn try_new(
        consumer: ConsumerInstanceRef,
        definition: ConsumerNotificationDefinition<B>,
        limits: RawNotificationLimits,
    ) -> Result<Self, BindingInputRejection<ConsumerNotificationDefinition<B>>> {
        let build = (|| {
            let input = definition.input();
            let id = input.identity;
            let data = &input.services.data;
            let policy = data.policy();
            if !consumer.is_current()
                || input.binding.identity() != (id.binding_id(), id.binding_generation())
                || input.binding.configuration() != &input.configuration
                || !matches!(input.target, AffordanceTarget::Event(_))
                || !data.available()
                || policy.validation_work == 0
                || input.services.access.work() == 0
                || ![
                    input.schemas.subscription,
                    input.schemas.data,
                    input.schemas.data_response,
                    input.schemas.cancellation,
                ]
                .into_iter()
                .flatten()
                .all(|s| data.supports_schema(s))
            {
                return Err(invalid());
            }
            input
                .binding
                .support(&input.program, NotificationKind::Event)?;
            let funding = reserve(
                &input.bank,
                &[
                    (
                        K::EngineLiveBytesGlobalMax,
                        (limits.engine_bytes as u64)
                            .checked_add(policy.temporary_bytes)
                            .ok_or_else(invalid)?,
                    ),
                    (K::AdmissionTemporaryBytesGlobalMax, policy.temporary_bytes),
                    (K::CleanupItemsMax, limits.subscriptions as u64),
                ],
            )?;
            let captured = Rc::new(CapturedServices {
                target: input.target.clone(),
                endpoint: input.endpoint.clone(),
                binding: id,
                consumer: consumer.clone(),
                services: input.services.clone(),
                schemas: input.schemas,
                policy,
                access_work: input.services.access.work(),
            });
            let root = RawNotificationServient::try_new_validated(
                input.binding.clone(),
                input.configuration.clone(),
                CapturedAuthorizer(captured.clone()),
                consumer,
                limits,
                captured,
            )
            .map_err(|e| e.error().error().clone())?;
            Ok((root, funding))
        })();
        match build {
            Ok((root, funding)) => Ok(Self {
                root,
                definition,
                _funding: funding,
            }),
            Err(error) => Err(BindingInputRejection::new(
                definition,
                BindingOperationalError::new(error),
            )),
        }
    }
    pub fn subscribe(
        &mut self,
        input: SubscriptionInput,
        responses: ResponseRequirement,
        overflow: NotificationOverflow,
        budget: &mut WorkBudget,
    ) -> Result<NotificationCursor, BindingInputRejection<SubscriptionInput>> {
        self.root
            .subscribe(
                RawNotificationRequest {
                    target: self.definition.input().target.clone(),
                    subscription: input,
                    responses,
                    overflow,
                },
                self.definition.input().program.clone(),
                budget,
            )
            .map_err(|e| {
                let ((request, _), error) = e.into_parts();
                BindingInputRejection::new(request.subscription, error)
            })
    }
    pub fn progress(
        &mut self,
        id: &NotificationIdentity,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<NotificationState> {
        self.root.progress(id, cx, budget)
    }
    pub fn front(
        &mut self,
        cursor: &mut NotificationCursor,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<RawNotificationView<'_>>> {
        self.root.front(cursor, budget)
    }
    pub fn advance(
        &mut self,
        cursor: &mut NotificationCursor,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        self.root.advance(cursor, budget)
    }
    /// Retire the exposed item's unused optional response without sending or
    /// stopping. Progress must finish native retirement before cursor advance;
    /// required or already submitted responses cannot be declined.
    pub fn decline_response(
        &mut self,
        cursor: &mut NotificationCursor,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        self.root.decline_response(cursor, budget)
    }
    pub fn stop(
        &mut self,
        cursor: &NotificationCursor,
        input: Option<CancellationInput>,
        budget: &mut WorkBudget,
    ) -> Result<(), BindingInputRejection<Option<CancellationInput>>> {
        self.root.stop(cursor, input, budget)
    }
    pub fn respond(
        &mut self,
        cursor: &mut NotificationCursor,
        input: EventDataResponse,
        budget: &mut WorkBudget,
    ) -> Result<(), BindingInputRejection<EventDataResponse>> {
        self.root.respond(cursor, input, budget)
    }
    pub fn status(&self, id: &NotificationIdentity) -> CoreResult<NotificationStatus<'_>> {
        self.root.status(id)
    }
    pub fn take_settlement(
        &mut self,
        id: &NotificationIdentity,
    ) -> CoreResult<NotificationSettlement> {
        self.root.take_settlement(id)
    }
    pub fn step_cleanup(
        &mut self,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<NotificationSettlement>> {
        self.root.step_cleanup(cx, budget)
    }
    pub fn begin_close(&mut self) {
        self.root.begin_close();
    }
}
