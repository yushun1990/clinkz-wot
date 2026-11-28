//! TD-prepared notification services over the existing raw conversation owner.
//! No TD, compiler, protocol matching or native distribution lives here.
use super::{ConsumerOwned, ConsumerTurn};
use crate::notification::*;
use alloc::{rc::Rc, sync::Arc};
use clinkz_wot_core::{consumer::*, notification::*, *};
use core::task::Context;

fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Validate, RetryClass::Never)
}
fn invalid() -> CoreError {
    CoreError::Validation(context())
}

struct CapturedServices<B: ConsumerNotificationBinding> {
    consumer: ConsumerInstanceRef,
    definition: Rc<ConsumerNotificationDefinition<B>>,
    access: Arc<ConsumerServiceOwner<dyn ConsumerAccessService>>,
    security_revision: u64,
}
struct CapturedAuthorizer<B: ConsumerNotificationBinding> {
    association: Rc<ConsumerOwned<CapturedServices<B>>>,
}
impl<B: ConsumerNotificationBinding> NotificationAuthorizer for CapturedAuthorizer<B> {
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
    fn check_dependencies(&self) -> CoreResult<()> {
        self.definition.check_dependencies()?;
        if self.access.retained_bytes() != self.access.backing_bytes()
            || self.access.qualification_revision() != self.security_revision
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
    ) -> Result<(), NotificationValidationError> {
        use NotificationValidationError::{Dependency, Rejected};
        self.check_dependencies().map_err(Dependency)?;
        match (schema, payload) {
            (None, None) => Ok(()),
            (Some(schema), Some(payload)) => {
                self.check_payload(payload)?;
                let result = self.definition.input().data.validate(schema, payload);
                // This check also runs when the codec refuses the caller's
                // payload. Observed dependency loss cannot become input error.
                self.check_dependencies().map_err(Dependency)?;
                result.map_err(Rejected)
            }
            _ => Err(Rejected(invalid())),
        }
    }
    fn check_payload(&self, payload: &Payload) -> Result<(), NotificationValidationError> {
        let input = self.definition.input();
        if payload.body.len() as u64 > input.data_policy.body_bytes
            || payload.content_type != input.content_type.as_ref()
        {
            return Err(NotificationValidationError::Rejected(invalid()));
        }
        Ok(())
    }
}
impl<B: ConsumerNotificationBinding> NotificationValidation for ConsumerOwned<CapturedServices<B>> {
    fn subscription(&self, input: &SubscriptionInput) -> Result<(), NotificationValidationError> {
        if matches!(self.schemas(), NotificationSchemas::SubscribeAllEvents(_)) && input.0.is_some()
        {
            return Err(NotificationValidationError::Rejected(
                CoreError::UnsupportedOperation(context()),
            ));
        }
        self.validate(self.schemas().subscription(), input.0.as_ref())
    }
    fn cancellation(&self, input: &CancellationInput) -> Result<(), NotificationValidationError> {
        if matches!(self.schemas(), NotificationSchemas::SubscribeAllEvents(_)) && input.0.is_some()
        {
            return Err(NotificationValidationError::Rejected(
                CoreError::UnsupportedOperation(context()),
            ));
        }
        self.validate(self.schemas().cancellation(), input.0.as_ref())
    }
    fn response(
        &self,
        target: &AffordanceTarget,
        input: &EventDataResponse,
    ) -> Result<(), NotificationValidationError> {
        let schema = if let NotificationSchemas::SubscribeAllEvents(s) = self.schemas() {
            let AffordanceTarget::Event(name) = target else {
                return Err(NotificationValidationError::Rejected(invalid()));
            };
            s.member(name)
                .ok_or_else(|| NotificationValidationError::Rejected(invalid()))?
                .data_response
                .as_ref()
        } else {
            self.schemas().data_response()
        };
        self.validate(schema, input.0.as_ref())
    }
    fn data(
        &self,
        target: &AffordanceTarget,
        data: Option<&Payload>,
    ) -> Result<(), NotificationValidationError> {
        use NotificationValidationError::{Dependency, Rejected};
        if let NotificationSchemas::SubscribeAllEvents(s) = self.schemas() {
            let AffordanceTarget::Event(name) = target else {
                return Err(Rejected(invalid()));
            };
            return self.validate(
                s.member(name)
                    .ok_or_else(|| Rejected(invalid()))?
                    .data
                    .as_ref(),
                data,
            );
        }
        if let NotificationSchemas::ObserveAllProperties(schema) = self.schemas() {
            self.check_dependencies().map_err(Dependency)?;
            let payload = data.ok_or_else(|| Rejected(invalid()))?;
            self.check_payload(payload)?;
            let result = self
                .definition
                .input()
                .data
                .validate_property_map(schema, payload);
            self.check_dependencies().map_err(Dependency)?;
            return result.map_err(Rejected);
        }
        self.validate(self.schemas().data(), data)
    }
}

/// Persistent checked Consumer association. Clones are passive, non-revoking
/// views. It owns no vacant slot, queue or native subscription. Accepted work
/// retains this original association even after all application views disappear.
pub struct ConsumerNotificationAssociation<B: ConsumerNotificationBinding> {
    captured: Rc<ConsumerOwned<CapturedServices<B>>>,
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
    ) -> Result<
        Self,
        BindingInputRejection<(
            Rc<ConsumerNotificationDefinition<B>>,
            Arc<ConsumerServiceOwner<dyn ConsumerAccessService>>,
        )>,
    > {
        let _turn = match ConsumerTurn::enter() {
            Ok(turn) => turn,
            Err(error) => {
                return Err(BindingInputRejection::new(
                    (definition, access),
                    BindingOperationalError::new(error),
                ));
            }
        };
        let built = (|| {
            definition.check_dependencies()?;
            let revision = access.qualification_revision();
            if !consumer.is_current()
                || access.retained_bytes() != access.backing_bytes()
                || !access.supports_security(&clinkz_wot_core::security::SecurityRequirement::NOSEC)
                || revision != access.qualification_revision()
                || !consumer.is_current()
            {
                return Err(invalid());
            }
            Ok(revision)
        })();
        match built {
            Ok(security_revision) => Ok(Self {
                captured: Rc::new(ConsumerOwned::new(CapturedServices {
                    consumer,
                    definition,
                    access,
                    security_revision,
                })),
            }),
            Err(error) => Err(BindingInputRejection::new(
                (definition, access),
                BindingOperationalError::new(error),
            )),
        }
    }
    pub fn definition(&self) -> &Rc<ConsumerNotificationDefinition<B>> {
        &self.captured.definition
    }
    pub fn consumer(&self) -> &ConsumerInstanceRef {
        &self.captured.consumer
    }
}

/// Shared conversation slots for independent associations on one Host driver.
pub struct ConsumerNotifications<B: ConsumerNotificationBinding> {
    root: RawNotificationServient<B, CapturedAuthorizer<B>>,
}
impl<B: ConsumerNotificationBinding + Clone + 'static> ConsumerNotifications<B>
where
    B::Program: Clone + 'static,
    B::Configuration: Clone + 'static,
{
    pub fn try_new(limits: RawNotificationLimits) -> CoreResult<Self> {
        Ok(Self {
            root: RawNotificationServient::try_new(limits)?,
        })
    }
    pub fn retained_subscriptions(&self) -> usize {
        self.root.retained_subscriptions()
    }
    /// Visit one rotating slot, including original cleanup and abandoned reports.
    pub fn step(&mut self, cx: &mut Context<'_>) -> CoreResult<NotificationTurn> {
        self.root.step(cx)
    }
    pub fn subscribe(
        &mut self,
        association: &ConsumerNotificationAssociation<B>,
        input: SubscriptionInput,
        responses: ResponseRequirement,
        overflow: NotificationOverflow,
    ) -> Result<NotificationCursor, BindingInputRejection<SubscriptionInput>> {
        let mut turn = match ConsumerTurn::enter() {
            Ok(turn) => turn,
            Err(error) => return Err(reject(input, error)),
        };
        let captured = &association.captured;
        let definition = captured.definition.input();
        let context = match RawNotificationContext::new_in_turn(
            &mut turn,
            definition.binding.clone(),
            definition.configuration.clone(),
            CapturedAuthorizer {
                association: captured.clone(),
            },
            captured.consumer.clone(),
            Some(captured.clone()),
        ) {
            Ok(context) => context,
            Err(error) => return Err(reject(input, error.error().error().clone())),
        };
        self.root
            .subscribe_in_turn(
                &mut turn,
                RawNotificationRequest {
                    target: definition.target.clone(),
                    kind: definition.schemas.kind(),
                    subscription: input,
                    responses,
                    overflow,
                },
                definition.program.clone(),
                context,
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
    ) -> CoreResult<NotificationTurn> {
        self.root.progress(id, cx)
    }
    pub fn front(
        &mut self,
        cursor: &mut NotificationCursor,
    ) -> CoreResult<Option<RawNotificationView<'_>>> {
        self.root.front(cursor)
    }
    pub fn advance(&mut self, cursor: &mut NotificationCursor) -> CoreResult<()> {
        self.root.advance(cursor)
    }
    pub fn decline_response(&mut self, cursor: &mut NotificationCursor) -> CoreResult<()> {
        self.root.decline_response(cursor)
    }
    pub fn stop(
        &mut self,
        cursor: &NotificationCursor,
        input: Option<CancellationInput>,
    ) -> Result<(), BindingInputRejection<Option<CancellationInput>>> {
        self.root.stop(cursor, input)
    }
    pub fn respond(
        &mut self,
        cursor: &mut NotificationCursor,
        input: EventDataResponse,
    ) -> Result<(), BindingInputRejection<EventDataResponse>> {
        self.root.respond(cursor, input)
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
    ) -> CoreResult<Option<NotificationSettlement>> {
        self.root.step_cleanup(cx)
    }
    pub fn begin_close(&mut self) {
        self.root.begin_close();
    }
}
