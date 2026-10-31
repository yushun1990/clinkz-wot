//! Shared qualified HTTP Producer fixture.
#![allow(dead_code, unused_imports)]
use clinkz_wot_binding_http::*;
use clinkz_wot_core::{action::*, finite::*, producer::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::producer_composition::*;
use clinkz_wot_servient::{host_accounts::HostResourceAccounts, producer_composition::*};
use clinkz_wot_servient_finite::*;
use clinkz_wot_td::ValidatedProducerScope;
use std::{
    cell::Cell,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};

pub(crate) fn budget() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, c| b.with_remaining(c, 1_000_000))
}
pub(crate) fn addr() -> SocketAddr {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
}
pub(crate) fn key(generation: Generation) -> ProducerInstanceAuthority {
    ProducerInstanceAuthority::try_new(ThingSlotId::new(SlotIndex::new(1), generation)).unwrap()
}
pub(crate) fn now() -> MonotonicInstant {
    MonotonicInstant::new(ClockId::new(1), 0)
}
pub(crate) fn cx() -> Context<'static> {
    Context::from_waker(Waker::noop())
}
pub(crate) fn td(action: SocketAddr, property: Option<SocketAddr>) -> Arc<str> {
    let mut json = serde_json::json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"HTTP Actions",
        "security":["n"], "securityDefinitions":{"n":{"scheme":"nosec"}},
        "actions":{"run":{"synchronous":false, "input":{"type":"integer"}, "output":{"type":"integer"},
            "uriVariables":{"id":{"type":"string"}},
            "forms":[
                {"href":format!("http://{action}/run"), "op":"invokeaction", "htv:methodName":"POST"},
                {"href":format!("http://{action}/status{{?id}}"), "op":"queryaction", "htv:methodName":"GET"},
                {"href":format!("http://{action}/stop{{?id}}"), "op":"cancelaction", "htv:methodName":"DELETE"}
            ]}}
    });
    if let Some(property) = property {
        json["properties"] = serde_json::json!({"level":{"type":"integer","readOnly":true,
            "forms":[{"href":format!("http://{property}/value")}]}});
    }
    Arc::from(json.to_string())
}
#[derive(Default)]
pub(crate) struct Counters {
    pub(crate) invoked: Cell<usize>,
    pub(crate) cancelled: Cell<usize>,
    pub(crate) complete: Cell<bool>,
    pub(crate) invalid: Cell<bool>,
    pub(crate) immediate: Cell<bool>,
    pub(crate) deny_invoke: Cell<bool>,
    pub(crate) deny_query: Cell<bool>,
    pub(crate) deny_cancel: Cell<bool>,
}
struct Action(Rc<Counters>);
impl RawActionHandler for Action {
    fn authorize(&self, op: Operation, _: &InteractionInput) -> CoreResult<()> {
        if match op {
            Operation::InvokeAction => self.0.deny_invoke.get(),
            Operation::QueryAction => self.0.deny_query.get(),
            Operation::CancelAction => self.0.deny_cancel.get(),
            _ => false,
        } {
            Err(CoreError::Security {
                reason: SecurityFailureReason::AuthorizationDenied,
                context: ErrorContext::new(ErrorPhase::Handler, RetryClass::Never),
            })
        } else {
            Ok(())
        }
    }
    fn invoke(
        &self,
        _: HandlerContext<'_>,
        input: &InteractionInput,
    ) -> CoreResult<RawActionStart> {
        assert_eq!(input.data.as_ref().unwrap().body.as_ref(), b"12");
        self.0.invoked.set(self.0.invoked.get() + 1);
        if self.0.immediate.get() {
            return Ok(RawActionStart::Complete(InteractionOutput::with_data(
                Payload::new(b"\"invalid\"".to_vec(), "application/json"),
            )));
        }
        Ok(RawActionStart::Continue(Box::new(Work(Rc::clone(&self.0)))))
    }
}
struct Work(Rc<Counters>);
impl RawActionWork for Work {
    fn poll(&mut self) -> Poll<CoreResult<InteractionOutput>> {
        if !self.0.complete.get() {
            return Poll::Pending;
        }
        Poll::Ready(Ok(InteractionOutput::with_data(Payload::new(
            if self.0.invalid.get() {
                b"\"invalid\"".to_vec()
            } else {
                b"24".to_vec()
            },
            "application/json",
        ))))
    }
    fn request_cancel(&mut self) -> CoreResult<()> {
        self.0.cancelled.set(self.0.cancelled.get() + 1);
        Ok(())
    }
    fn poll_close(&mut self) -> Poll<CoreResult<()>> {
        Poll::Ready(Ok(()))
    }
    fn abort_local(self: Box<Self>) {}
}
struct Property;
impl RawFiniteHandler for Property {
    fn authorize(&self, _: &InteractionInput) -> CoreResult<()> {
        Ok(())
    }
    fn handle(&self, _: HandlerContext<'_>, _: &InteractionInput) -> CoreResult<InteractionOutput> {
        Ok(InteractionOutput::with_data(Payload::new(
            b"42".to_vec(),
            "application/json",
        )))
    }
}
pub(crate) fn owner() -> RawProducerServient {
    let mut owner = RawProducerServient::try_empty(RawProducerRegistryLimits {
        instances: 2,
        behaviors_per_instance: 4,
        target_name_bytes: 64,
    })
    .unwrap();
    owner
        .enable_raw_actions(RawActionLimits {
            invocations: 4,
            application_work_bytes: 64,
            input_fields: 8,
            input_bytes: 4096 + 64,
            principal_id_bytes: 32,
            output_body_bytes: 4096,
            output_media_capacity: 64,
            output_retained_bytes: 8192,
        })
        .unwrap();
    owner
}
pub(crate) fn create(
    owner: &RawProducerServient,
    authority: ProducerInstanceAuthority,
    action: SocketAddr,
    property: Option<SocketAddr>,
    counts: Rc<Counters>,
    expected: Option<ProducerInstanceKey>,
    bank: Arc<HostResourceAccounts>,
) -> ComposedProducerExposure {
    let text = td(action, property);
    let key = authority.key();
    let instance = RawProducerInstance::new(
        authority,
        property.map_or_else(Vec::new, |_| {
            vec![RawFiniteBehavior::new(
                AffordanceTarget::Property(Arc::from("level")),
                Operation::ReadProperty,
                Property,
            )]
        }),
    )
    .with_actions(vec![RawActionBehavior::new(
        Arc::from("run"),
        Action(counts),
        0,
        true,
        true,
    )])
    .with_description(Arc::clone(&text));
    let action_provider = Rc::new(HttpActionProducer::new(
        HttpProducerConfig::for_loopback(action).unwrap(),
        BindingId::new(61),
        BindingGeneration::INITIAL,
    ));
    let property_provider = property.map(|addr| {
        Rc::new(
            HttpFiniteProducer::try_new(
                HttpProducerConfig::for_loopback(addr).unwrap(),
                BindingId::new(62),
                BindingGeneration::INITIAL,
                100,
            )
            .unwrap(),
        )
    });
    let mut providers: Vec<Rc<dyn ProducerCapability>> = vec![action_provider];
    if let Some(p) = property_provider {
        providers.push(p);
    }
    let borrowed: Vec<_> = providers.iter().map(|p| p.as_ref()).collect();
    let prepared = prepare_thing(
        &text,
        key,
        GatewayDefaultV1::LIMITS,
        &mut budget(),
        &borrowed,
        |scope, op| match scope {
            ValidatedProducerScope::Action(n) => instance.supports_action(n, op),
            ValidatedProducerScope::Property(n) => instance.supports_property(n, op),
            _ => false,
        },
    )
    .unwrap();
    let definitions: Vec<_> = prepared
        .operations()
        .iter()
        .filter_map(|o| o.definition::<HttpActionDefinition>().cloned())
        .collect();
    assert_eq!(definitions.len(), 3);
    let mut participants: Vec<Box<dyn OwnedProducerParticipation>> = vec![Box::new(
        HttpActionRuntime::try_new(owner, key, definitions, 4).unwrap(),
    )];
    if let Some(addr) = property {
        let program = prepared
            .operations()
            .iter()
            .find_map(|o| o.definition::<HttpProducerProgram>())
            .unwrap()
            .clone();
        let config = HttpFiniteConfig::try_new(
            HttpProducerConfig::for_loopback(addr).unwrap(),
            program,
            key,
            Arc::from("level"),
            100,
        )
        .unwrap();
        participants.push(Box::new(
            RawFiniteRuntime::try_new(
                HttpFiniteBinding::new(
                    config.clone(),
                    BindingId::new(62),
                    BindingGeneration::INITIAL,
                ),
                config,
                owner,
                RawFiniteLimits {
                    candidates: 1,
                    input_body_bytes: 0,
                    output_body_bytes: 4096,
                    output_media_capacity: 64,
                    output_retained_bytes: 8192,
                },
                FiniteValidation::Required,
                Deadline::NONE,
            )
            .ok()
            .unwrap(),
        ));
    }
    ComposedProducerExposure::try_new(
        owner,
        ComposedProducerInput {
            instance,
            prepared,
            providers,
            participants,
            expected,
            handler_backing_bytes: 0,
        },
        Arc::new(GatewayDefaultV1::LIMITS.clone()),
        bank,
        &mut budget(),
    )
    .ok()
    .unwrap()
}
