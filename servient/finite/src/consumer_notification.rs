//! TD-prepared notification services over the existing raw conversation owner.
//! No TD, compiler, protocol matching or native distribution lives here.
use crate::notification::*;
use alloc::{rc::Rc, vec::Vec};
use clinkz_wot_core::{consumer::*, notification::*, *};
use clinkz_wot_foundation::{ResourceKind as K, WorkBudget, WorkClass as W};
use core::task::Context;

struct CapturedServices<B: ConsumerNotificationBinding> {
    consumer: ConsumerInstanceRef,
    definition: Rc<ConsumerNotificationDefinition<B>>,
    policy: ConsumerDataPolicy,
    access_work: u64,
}
struct CapturedAuthorizer<B: ConsumerNotificationBinding>(Rc<CapturedServices<B>>);
impl<B: ConsumerNotificationBinding> NotificationAuthorizer for CapturedAuthorizer<B> {
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
        let input = captured.definition.input();
        if input.services.access.work() != captured.access_work {
            return Err(invalid());
        }
        if id.consumer != captured.consumer
            || !captured.consumer.is_current()
            || target != &input.target
        {
            return Err(CoreError::StaleHandle(context()));
        }
        let kind = captured.schemas().kind();
        let operation = match permission {
            NotificationPermission::Cancel => kind.stop_operation(),
            _ => kind.start_operation(),
        };
        input.services.access.authorize_instance(
            &id.consumer,
            input.identity,
            &input.endpoint,
            operation,
        )
    }
}
impl<B: ConsumerNotificationBinding> CapturedServices<B> {
    fn schemas(&self) -> &NotificationSchemas {
        &self.definition.input().schemas
    }
    fn services(&self) -> &ConsumerServices {
        &self.definition.input().services
    }
    fn check_data(&self) -> CoreResult<()> {
        if !self.services().data.available() || self.services().data.policy() != self.policy {
            return Err(invalid());
        }
        Ok(())
    }
    fn validate(
        &self,
        schema: Option<&clinkz_wot_core::data_schema::PreparedSchema>,
        payload: Option<&Payload>,
    ) -> CoreResult<()> {
        match (schema, payload) {
            (None, None) => Ok(()),
            (Some(schema), Some(payload))
                if self.services().data.available()
                    && self.services().data.policy() == self.policy
                    && self.services().data.supports_schema(schema)
                    && payload.body.len() as u64 <= self.policy.body_bytes =>
            {
                let result = self.services().data.validate(schema, payload);
                self.check_data()?;
                if !self.services().data.supports_schema(schema) {
                    return Err(invalid());
                }
                result
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
impl<B: ConsumerNotificationBinding> NotificationValidation for CapturedServices<B> {
    fn subscription(&self, input: &SubscriptionInput, budget: &mut WorkBudget) -> CoreResult<()> {
        if let NotificationSchemas::SubscribeAllEvents(s) = self.schemas() {
            // No universal aggregate control encoding is implied by TD 1.1.
            if input.0.is_some() || s.members().iter().any(|m| m.subscription.is_some()) {
                return Err(CoreError::UnsupportedOperation(context()));
            }
        }
        if self.schemas().subscription().is_some() {
            self.prepay(budget)?;
        }
        self.validate(self.schemas().subscription(), input.0.as_ref())
    }
    fn cancellation(&self, input: &CancellationInput, budget: &mut WorkBudget) -> CoreResult<()> {
        if let NotificationSchemas::SubscribeAllEvents(s) = self.schemas() {
            if input.0.is_some() || s.members().iter().any(|m| m.cancellation.is_some()) {
                return Err(CoreError::UnsupportedOperation(context()));
            }
        }
        if self.schemas().cancellation().is_some() {
            self.prepay(budget)?;
        }
        self.validate(self.schemas().cancellation(), input.0.as_ref())
    }
    fn response(
        &self,
        target: &AffordanceTarget,
        input: &EventDataResponse,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        let schema = if let NotificationSchemas::SubscribeAllEvents(s) = self.schemas() {
            let AffordanceTarget::Event(name) = target else {
                return Err(invalid());
            };
            s.member(name).ok_or_else(invalid)?.data_response.as_ref()
        } else {
            self.schemas().data_response()
        };
        if schema.is_some() {
            self.prepay(budget)?;
        }
        self.validate(schema, input.0.as_ref())
    }
    fn prepay_data(&self, budget: &mut WorkBudget) -> CoreResult<()> {
        if self.schemas().data().is_some()
            || matches!(
                self.schemas(),
                NotificationSchemas::ObserveAllProperties(_)
                    | NotificationSchemas::SubscribeAllEvents(_)
            )
        {
            self.prepay(budget)?;
        }
        Ok(())
    }
    fn data(&self, target: &AffordanceTarget, data: Option<&Payload>) -> CoreResult<()> {
        if let NotificationSchemas::SubscribeAllEvents(s) = self.schemas() {
            let AffordanceTarget::Event(name) = target else {
                return Err(invalid());
            };
            return self.validate(s.member(name).ok_or_else(invalid)?.data.as_ref(), data);
        }
        if let NotificationSchemas::ObserveAllProperties(schema) = self.schemas() {
            let payload = data.ok_or_else(invalid)?;
            if !self.services().data.available()
                || self.services().data.policy() != self.policy
                || !self.services().data.supports_property_map(schema)
                || payload.body.len() as u64 > self.policy.body_bytes
            {
                return Err(invalid());
            }
            let result = self.services().data.validate_property_map(schema, payload);
            self.check_data()?;
            if !self.services().data.supports_property_map(schema) {
                return Err(invalid());
            }
            return result;
        }
        self.validate(self.schemas().data(), data)
    }
}

/// One reusable selected leaf or root notification definition with independent cursors.
/// Several such facets may share one ConsumerInstance and native Binding service.
pub struct ConsumerNotificationRuntime<B: ConsumerNotificationBinding> {
    root: RawNotificationServient<B, CapturedAuthorizer<B>>,
    definition: Rc<ConsumerNotificationDefinition<B>>,
    // Root, native participants, programs and actual services drop first.
    _funding: Vec<clinkz_wot_core::host_accounts::HostCapacityReservation>,
}
impl<B: ConsumerNotificationBinding + Clone + 'static> ConsumerNotificationRuntime<B>
where
    B::Program: Clone + 'static,
    B::Configuration: Clone + 'static,
{
    pub fn try_new(
        consumer: ConsumerInstanceRef,
        definition: ConsumerNotificationDefinition<B>,
        limits: RawNotificationLimits,
    ) -> Result<Self, BindingInputRejection<ConsumerNotificationDefinition<B>>> {
        // Capture the original definition and funding. Structured role trees
        // are never copied into a second validator owner.
        let definition = Rc::new(definition);
        let build = (|| {
            let input = definition.input();
            let id = input.identity;
            let data = &input.services.data;
            let policy = data.policy();
            if !consumer.is_current()
                || input.binding.identity() != (id.binding_id(), id.binding_generation())
                || input.binding.configuration() != &input.configuration
                || !input.schemas.kind().accepts_target(&input.target)
                || !data.available()
                || policy.validation_work == 0
                || input.services.access.work() == 0
                || !input.schemas.supports(data.as_ref())
            {
                return Err(invalid());
            }
            input
                .binding
                .support(&input.program, input.schemas.kind())?;
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
                consumer: consumer.clone(),
                definition: definition.clone(),
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
                Rc::try_unwrap(definition)
                    .unwrap_or_else(|_| unreachable!("failed root retained definition")),
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
                    kind: self.definition.input().schemas.kind(),
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
