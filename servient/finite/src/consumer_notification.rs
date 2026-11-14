//! TD-prepared notification services over the existing raw conversation owner.
//! No TD, compiler, protocol matching or native distribution lives here.
use crate::notification::*;
use alloc::{rc::Rc, sync::Arc, vec::Vec};
use clinkz_wot_core::host_accounts::{HostCapacityReservation, HostResourceAccounts};
use clinkz_wot_core::{consumer::*, notification::*, *};
use clinkz_wot_foundation::{ResourceKind as K, WorkBudget, WorkClass as W};
use core::task::Context;

struct CapturedServices<B: ConsumerNotificationBinding> {
    consumer: ConsumerInstanceRef,
    definition: Rc<ConsumerNotificationDefinition<B>>,
    access: Arc<ConsumerServiceOwner<dyn ConsumerAccessService>>,
    access_work: u64,
    security_revision: u64,
    bank: Arc<HostResourceAccounts>,
    _funding: Vec<HostCapacityReservation>,
}
struct CapturedAuthorizer<B: ConsumerNotificationBinding> {
    association: Rc<CapturedServices<B>>,
    // Original admitted work and instance quotas survive public view loss.
    _quota: HostCapacityReservation,
    _scratch: HostCapacityReservation,
    _temporary: HostCapacityReservation,
}
impl<B: ConsumerNotificationBinding> NotificationAuthorizer for CapturedAuthorizer<B> {
    fn work(&self) -> u64 {
        self.association.access_work
    }
    fn authorize(
        &self,
        request: NotificationAuthorization<'_>,
    ) -> CoreResult<NotificationDecision> {
        let captured = &self.association;
        captured.check_dependencies()?;
        let input = captured.definition.input();
        if request.recipient.consumer != captured.consumer
            || !captured.consumer.is_current()
            || request.target != &input.target
            || request.kind != input.schemas.kind()
        {
            return Err(CoreError::StaleHandle(context()));
        }
        if matches!(
            request.permission,
            NotificationPermission::Deliver | NotificationPermission::Respond
        ) != request.occurrence.is_some()
        {
            return Err(invalid());
        }
        if let Some(occurrence) = request.occurrence {
            if !request.kind.accepts_occurrence(request.target, occurrence) {
                return Err(invalid());
            }
            if let NotificationSchemas::SubscribeAllEvents(schema) = &input.schemas {
                let AffordanceTarget::Event(name) = occurrence else {
                    return Err(invalid());
                };
                if schema.member(name).is_none() {
                    return Err(invalid());
                }
            }
        }
        let result =
            captured
                .access
                .authorize_notification(input.identity, &input.endpoint, request);
        captured.check_dependencies()?;
        if !captured.consumer.is_current() {
            return Err(CoreError::StaleHandle(context()));
        }
        result
    }
}
impl<B: ConsumerNotificationBinding> CapturedServices<B> {
    fn schemas(&self) -> &NotificationSchemas {
        &self.definition.input().schemas
    }
    fn policy(&self) -> ConsumerDataPolicy {
        self.definition.input().data_policy
    }
    fn check_dependencies(&self) -> CoreResult<()> {
        self.definition.check_dependencies()?;
        if self.access.work() != self.access_work
            || self.access.retained_bytes() != self.access.backing_bytes()
            || self.access.security_revision() != self.security_revision
            || !self
                .access
                .supports_security(&clinkz_wot_core::security::SecurityRequirement::NOSEC)
        {
            return Err(invalid());
        }
        Ok(())
    }
    fn validate(
        &self,
        schema: Option<&clinkz_wot_core::data_schema::PreparedSchema>,
        payload: Option<&Payload>,
    ) -> CoreResult<()> {
        self.check_dependencies()?;
        match (schema, payload) {
            (None, None) => Ok(()),
            (Some(schema), Some(payload))
                if self.definition.input().data.available()
                    && self.definition.input().data.policy() == self.policy()
                    && self.definition.input().data.supports_schema(schema)
                    && payload.body.len() as u64 <= self.policy().body_bytes =>
            {
                let result = self.definition.input().data.validate(schema, payload);
                self.check_dependencies()?;
                if !self.definition.input().data.supports_schema(schema) {
                    return Err(invalid());
                }
                result
            }
            _ => Err(invalid()),
        }
    }
    fn prepay(&self, budget: &mut WorkBudget) -> CoreResult<()> {
        self.check_dependencies()?;
        if !pay(budget, W::CodecInputBytes, self.policy().validation_work) {
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
            if !self.definition.input().data.available()
                || self.definition.input().data.policy() != self.policy()
                || !self.definition.input().data.supports_property_map(schema)
                || payload.body.len() as u64 > self.policy().body_bytes
            {
                return Err(invalid());
            }
            let result = self
                .definition
                .input()
                .data
                .validate_property_map(schema, payload);
            self.check_dependencies()?;
            if !self.definition.input().data.supports_property_map(schema) {
                return Err(invalid());
            }
            return result;
        }
        self.validate(self.schemas().data(), data)
    }
}

/// Persistent checked Consumer association. Clones are passive, non-revoking
/// views. It owns no vacant slot, queue or native subscription. Accepted work
/// retains this original association even after all application views disappear.
pub struct ConsumerNotificationAssociation<B: ConsumerNotificationBinding> {
    captured: Rc<CapturedServices<B>>,
}
impl<B: ConsumerNotificationBinding> Clone for ConsumerNotificationAssociation<B> {
    fn clone(&self) -> Self {
        Self {
            captured: self.captured.clone(),
        }
    }
}
impl<B: ConsumerNotificationBinding> ConsumerNotificationAssociation<B> {
    #[allow(clippy::type_complexity)]
    pub fn try_new(
        consumer: ConsumerInstanceRef,
        definition: Rc<ConsumerNotificationDefinition<B>>,
        access: Arc<ConsumerServiceOwner<dyn ConsumerAccessService>>,
        bank: Arc<HostResourceAccounts>,
        budget: &mut WorkBudget,
    ) -> Result<
        Self,
        BindingInputRejection<(
            Rc<ConsumerNotificationDefinition<B>>,
            Arc<ConsumerServiceOwner<dyn ConsumerAccessService>>,
        )>,
    > {
        let access_work = access.work();
        let security_revision = access.security_revision();
        let build = (|| {
            definition.check_dependencies()?;
            if !consumer.is_current()
                || access_work == 0
                || access.retained_bytes() != access.backing_bytes()
                || !access.supports_security(&clinkz_wot_core::security::SecurityRequirement::NOSEC)
            {
                return Err(invalid());
            }
            if !pay(budget, W::ProviderProbes, access_work) {
                return Err(busy());
            }
            let input = definition.input();
            access.authorize(
                input.identity,
                &input.endpoint,
                input.schemas.kind().start_operation(),
            )?;
            definition.check_dependencies()?;
            if !consumer.is_current()
                || access.work() != access_work
                || access.security_revision() != security_revision
                || access.retained_bytes() != access.backing_bytes()
            {
                return Err(invalid());
            }
            reserve(
                &bank,
                &[(
                    K::EngineLiveBytesGlobalMax,
                    core::mem::size_of::<CapturedServices<B>>() as u64 + 128,
                )],
            )
        })();
        let funding = match build {
            Ok(f) => f,
            Err(error) => {
                return Err(BindingInputRejection::new(
                    (definition, access),
                    BindingOperationalError::new(error),
                ));
            }
        };
        Ok(Self {
            captured: Rc::new(CapturedServices {
                consumer,
                definition,
                access,
                access_work,
                security_revision,
                bank,
                _funding: funding,
            }),
        })
    }
    pub fn definition(&self) -> &Rc<ConsumerNotificationDefinition<B>> {
        &self.captured.definition
    }
    pub fn consumer(&self) -> &ConsumerInstanceRef {
        &self.captured.consumer
    }
}

/// Explicit family-specific conversation capacity, independently funded and
/// usable by many persistent associations. No scheduling, native matching or
/// physical subscription pool is introduced. Drive progress/cleanup manually.
pub struct ConsumerNotifications<B: ConsumerNotificationBinding> {
    root: RawNotificationServient<B, CapturedAuthorizer<B>>,
    bank: Arc<HostResourceAccounts>,
    _funding: Vec<HostCapacityReservation>,
}
impl<B: ConsumerNotificationBinding + Clone + 'static> ConsumerNotifications<B>
where
    B::Program: Clone + 'static,
    B::Configuration: Clone + 'static,
{
    pub fn try_new(
        limits: RawNotificationLimits,
        bank: Arc<HostResourceAccounts>,
    ) -> CoreResult<Self> {
        let overhead = (core::mem::size_of::<Self>()
            - core::mem::size_of::<RawNotificationServient<B, CapturedAuthorizer<B>>>()
            + 3 * core::mem::size_of::<HostCapacityReservation>()) as u64;
        let funding = reserve(
            &bank,
            &[
                (
                    K::EngineLiveBytesGlobalMax,
                    (limits.engine_bytes as u64)
                        .checked_add(overhead)
                        .ok_or_else(invalid)?,
                ),
                (K::CleanupItemsMax, limits.subscriptions as u64),
                (K::SubscriptionsGlobalMax, limits.subscriptions as u64),
            ],
        )?;
        let root = RawNotificationServient::try_new(limits)?;
        Ok(Self {
            root,
            bank,
            _funding: funding,
        })
    }
    pub fn retained_subscriptions(&self) -> usize {
        self.root.retained_subscriptions()
    }
    pub fn subscribe(
        &mut self,
        association: &ConsumerNotificationAssociation<B>,
        input: SubscriptionInput,
        responses: ResponseRequirement,
        overflow: NotificationOverflow,
        budget: &mut WorkBudget,
    ) -> Result<NotificationCursor, BindingInputRejection<SubscriptionInput>> {
        let captured = &association.captured;
        let build = (|| {
            captured.check_dependencies()?;
            if !captured.consumer.is_current() {
                return Err(CoreError::StaleHandle(context()));
            }
            let quota = reserve_one(&captured.bank, K::SubscriptionsPerThingMax, 1)?;
            let scratch = reserve_one(
                &self.bank,
                K::EngineLiveBytesGlobalMax,
                captured.policy().temporary_bytes,
            )?;
            let temporary = reserve_one(
                &self.bank,
                K::AdmissionTemporaryBytesGlobalMax,
                captured.policy().temporary_bytes,
            )?;
            let definition = captured.definition.input();
            RawNotificationContext::try_new(
                definition.binding.clone(),
                definition.configuration.clone(),
                CapturedAuthorizer {
                    association: captured.clone(),
                    _quota: quota,
                    _scratch: scratch,
                    _temporary: temporary,
                },
                captured.consumer.clone(),
                Some(captured.clone()),
            )
            .map_err(|e| e.error().error().clone())
        })();
        let context = match build {
            Ok(context) => context,
            Err(error) => {
                return Err(BindingInputRejection::new(
                    input,
                    BindingOperationalError::new(error),
                ));
            }
        };
        let definition = captured.definition.input();
        self.root
            .subscribe(
                RawNotificationRequest {
                    target: definition.target.clone(),
                    kind: definition.schemas.kind(),
                    subscription: input,
                    responses,
                    overflow,
                },
                definition.program.clone(),
                context,
                budget,
            )
            .map_err(|e| {
                let ((request, _, _), error) = e.into_parts();
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

fn reserve_one(
    bank: &Arc<HostResourceAccounts>,
    kind: K,
    amount: u64,
) -> CoreResult<HostCapacityReservation> {
    bank.reserve(kind, amount)
        .ok_or_else(|| CoreError::LimitExceeded {
            resource: kind,
            limit: bank.account(kind).limit(),
            requested: Some(amount),
            observed: Some(bank.account(kind).used()),
            context: context(),
        })
}
