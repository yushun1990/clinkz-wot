#![allow(dead_code)]
use super::*;
use std::{any::Any, cell::Cell};

/// Linked participant with selectable actual services, used to challenge the
/// neutral qualification boundary, rather than bypassing it with raw requests.
pub struct ServicesParticipant {
    pub native: MqttNotificationRuntime,
    pub services: Option<Rc<dyn ProducerNotificationServices>>,
}
impl ClosedProducerParticipation for ServicesParticipant {
    fn participation_id(&self) -> &ProducerParticipationId {
        self.native.participation_id()
    }
    fn retain_owner(&mut self, owner: Rc<dyn Any>) {
        self.native.retain_owner(owner);
    }
    fn admission_backing_bytes(
        &self,
        owner: &RawProducerServient,
        key: ProducerInstanceKey,
    ) -> Option<usize> {
        self.native
            .admission_backing_bytes(owner, key)?
            .checked_add(4096)
    }
    fn begin_retire(&mut self) {
        self.native.begin_retire();
    }
    fn ready_for(&self, owner: &RawProducerServient, key: ProducerInstanceKey) -> bool {
        self.native.ready_for(owner, key)
    }
    fn covers(
        &self,
        key: ProducerInstanceKey,
        binding: (BindingId, BindingGeneration),
        site: clinkz_wot_core::thing::AffordanceSite<'_>,
        form: u32,
        op: Operation,
        definition: &dyn Any,
    ) -> bool {
        self.native.covers(key, binding, site, form, op, definition)
    }
    fn notification_publication(
        &self,
        key: ProducerInstanceKey,
        binding: (BindingId, BindingGeneration),
        site: clinkz_wot_core::thing::AffordanceSite<'_>,
        form: u32,
        definition: &dyn Any,
    ) -> Option<(
        NotificationRoute,
        Rc<dyn ProducerNotificationServices>,
        usize,
    )> {
        let (route, _, bytes) = self
            .native
            .notification_publication(key, binding, site, form, definition)?;
        Some((route, Rc::clone(self.services.as_ref()?), bytes))
    }
    fn ingress_gate(&self) -> &Rc<Cell<bool>> {
        self.native.ingress_gate()
    }
    fn retired(&self) -> bool {
        self.native.retired()
    }
}
pub struct ProbeServices {
    pub original: Rc<dyn ProducerNotificationServices>,
    pub denied: Cell<bool>,
    pub authorized: Cell<usize>,
    pub validated: Cell<usize>,
}
impl ProducerNotificationServices for ProbeServices {
    fn authorize(&self, data: &PublicationData, budget: &mut WorkBudget) -> CoreResult<()> {
        self.authorized.set(self.authorized.get() + 1);
        if self.denied.get() {
            return Err(CoreError::Security {
                reason: SecurityFailureReason::AuthorizationDenied,
                context: ErrorContext::new(ErrorPhase::Handler, RetryClass::Never),
            });
        }
        self.original.authorize(data, budget)
    }
    fn validate(&self, data: &PublicationData, budget: &mut WorkBudget) -> CoreResult<()> {
        self.validated.set(self.validated.get() + 1);
        self.original.validate(data, budget)
    }
}
pub fn intercept(
    owner: &RawProducerServient,
    authority: ProducerInstanceAuthority,
    provider: &MqttNotificationProducer,
    service: &MqttNotificationService,
    text: &str,
    missing: bool,
) -> (ComposedProducerInput, Rc<ProbeServices>) {
    let instance = instance(authority, text);
    intercept_with_instance(owner, instance, provider, service, missing)
}
pub fn intercept_with_instance(
    owner: &RawProducerServient,
    instance: RawProducerInstance,
    provider: &MqttNotificationProducer,
    service: &MqttNotificationService,
    missing: bool,
) -> (ComposedProducerInput, Rc<ProbeServices>) {
    let prepared = prepare(&instance, &[provider]).unwrap();
    let definitions = prepared
        .operations()
        .iter()
        .map(|o| {
            o.definition::<MqttNotificationDefinition>()
                .unwrap()
                .clone()
        })
        .collect();
    let runtime = service
        .participation(owner, instance.key(), definitions)
        .unwrap();
    let op = prepared
        .operations()
        .iter()
        .find(|o| {
            matches!(
                o.operation,
                Operation::SubscribeEvent | Operation::ObserveProperty
            )
        })
        .unwrap();
    let (_, original, _) = runtime
        .notification_publication(
            instance.key(),
            op.binding,
            op.site.site(),
            op.form,
            op.definition_any(),
        )
        .unwrap();
    let probe = Rc::new(ProbeServices {
        original,
        denied: Cell::new(false),
        authorized: Cell::new(0),
        validated: Cell::new(0),
    });
    let services = if missing {
        None
    } else {
        Some(probe.clone() as Rc<dyn ProducerNotificationServices>)
    };
    (
        ComposedProducerInput {
            instance,
            prepared,
            providers: vec![Rc::new(provider.clone())],
            participants: vec![Box::new(ServicesParticipant {
                native: runtime,
                services,
            })],
            expected: None,
            handler_backing_bytes: 0,
        },
        probe,
    )
}
