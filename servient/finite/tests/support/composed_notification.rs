//! Test assembly only: keep existing lifecycle regressions readable while the
//! production API supplies persistent association and work capacity separately.
#![allow(dead_code)]
use clinkz_wot_core::{consumer::*, host_accounts::HostResourceAccounts, notification::*, *};
use clinkz_wot_servient_finite::{consumer_notification::*, notification::*};
use std::{
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::Arc,
};

pub struct NotificationFixture<B: ConsumerNotificationBinding> {
    pub(crate) association: ConsumerNotificationAssociation<B>,
    pub(crate) work: ConsumerNotifications<B>,
}
impl<B: ConsumerNotificationBinding + Clone + 'static> NotificationFixture<B>
where
    B::Program: Clone + 'static,
    B::Configuration: Clone + 'static,
{
    pub fn try_new(
        consumer: ConsumerInstanceRef,
        definition: ConsumerNotificationDefinition<B>,
        access: Arc<dyn ConsumerAccessService>,
        limits: RawNotificationLimits,
        bank: &Arc<HostResourceAccounts>,
    ) -> CoreResult<Self> {
        let access = ConsumerServiceOwner::retain_access(access, bank)
            .map_err(|e| e.error().error().clone())?;
        let association =
            ConsumerNotificationAssociation::try_new(consumer, Rc::new(definition), access)
                .map_err(|e| e.error().error().clone())?;
        let work = ConsumerNotifications::try_new(limits)?;
        Ok(Self { association, work })
    }
    pub fn subscribe(
        &mut self,
        input: SubscriptionInput,
        responses: ResponseRequirement,
        overflow: NotificationOverflow,
    ) -> Result<NotificationCursor, BindingInputRejection<SubscriptionInput>> {
        self.work
            .subscribe(&self.association, input, responses, overflow)
    }
}
impl<B: ConsumerNotificationBinding> Deref for NotificationFixture<B> {
    type Target = ConsumerNotifications<B>;
    fn deref(&self) -> &Self::Target {
        &self.work
    }
}
impl<B: ConsumerNotificationBinding> DerefMut for NotificationFixture<B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.work
    }
}
