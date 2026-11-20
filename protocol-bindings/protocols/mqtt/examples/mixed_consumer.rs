//! cargo run -p clinkz-wot-binding-mqtt --example mixed_consumer --
//!     TD.json 127.0.0.1:1883 PROPERTY ACTION EVENT JSON_INPUT
//! The TD must name an ID and supported HTTP finite/continuing Action and MQTT
//! Event Forms. Uses the current bounded NoSec/JSON subsets for ten seconds.
use clinkz_wot_binding_http::{HttpActionConsumer, HttpConsumerConfig, try_consumer_registration};
use clinkz_wot_binding_mqtt::{MqttConsumerConfig, MqttConsumerService, MqttJsonData};
use clinkz_wot_core::{consumer::*, host_accounts::HostResourceAccounts, notification::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::open_consumer::*;
use clinkz_wot_servient_finite::{
    consumer::*, consumer_action::*, consumer_execution::step_consumers, consumer_notification::*,
    notification::RawNotificationLimits,
};
use std::{
    future::{Future, poll_fn},
    net::SocketAddr,
    rc::Rc,
    sync::Arc,
    task::Poll,
    time::{Duration, Instant},
};

fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, c| b.with_remaining(c, 1_000_000))
}
// Example application policy, independently selected from HTTP's access service.
struct EventAccess {
    consumer: ConsumerInstanceRef,
    endpoint: String,
}
impl ConsumerAccessService for EventAccess {
    fn qualification_revision(&self) -> u64 {
        0
    }

    fn authorize(
        &self,
        _: BindingRegistrationIdentity,
        target: &str,
        operation: Operation,
    ) -> CoreResult<()> {
        if target == self.endpoint
            && matches!(
                operation,
                Operation::SubscribeEvent | Operation::UnsubscribeEvent
            )
        {
            Ok(())
        } else {
            Err(CoreError::Security {
                reason: SecurityFailureReason::AuthorizationDenied,
                context: ErrorContext::new(ErrorPhase::Admission, RetryClass::Never),
            })
        }
    }
    fn authorize_instance(
        &self,
        consumer: &ConsumerInstanceRef,
        binding: BindingRegistrationIdentity,
        target: &str,
        operation: Operation,
    ) -> CoreResult<()> {
        if consumer != &self.consumer || !consumer.is_current() {
            return Err(CoreError::StaleHandle(ErrorContext::new(
                ErrorPhase::Admission,
                RetryClass::Never,
            )));
        }
        self.authorize(binding, target, operation)
    }
    fn retained_bytes(&self) -> u64 {
        (std::mem::size_of::<Self>() + self.endpoint.capacity()) as u64
    }
    fn work(&self) -> u64 {
        1
    }
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(
        args.len(),
        7,
        "provide TD.json, broker address, Property, Action, Event and JSON input (- for absent input)"
    );
    let thing = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let broker: SocketAddr = args[2].parse().unwrap();
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(ResourceKind::PlanCompileWorkUnitsPerStepMax, Some(4096)));
    let bank = Arc::new(HostResourceAccounts::new(&limits));
    let owner = ConsumerOwner::try_new(bank.clone(), 1, 1).unwrap();
    let instance =
        clinkz_wot_servient::consumer::register_thing(&owner, &thing, &mut work()).unwrap();
    let config = HttpConsumerConfig::for_cleartext_ip().unwrap();
    let registry = ConsumerRegistry::try_new(
        vec![
            try_consumer_registration(config, BindingId::new(701), BindingGeneration::INITIAL, 0)
                .unwrap(),
        ],
        &bank,
    )
    .ok()
    .unwrap();
    let prepared = registry
        .prepare(
            &thing,
            &limits,
            bank.clone(),
            ConsumerSelector::default(),
            &mut work(),
            |job| {
                while !job.is_finished() {
                    job.step(&mut work(), false);
                }
            },
        )
        .unwrap();
    let (bindings, funding) = prepared.into_execution();
    let association = StandaloneConsumerAssociation::try_new(
        instance.consumer(),
        bindings,
        funding,
        bank.clone(),
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    owner
        .set_configuration(instance.thing_id(), "http", association, &mut work())
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    drop(registry);

    let services = config.consumer_services();
    let services = RetainedConsumerServices {
        data: ConsumerServiceOwner::retain_data(services.data, &bank)
            .ok()
            .unwrap(),
        access: ConsumerServiceOwner::retain_access(services.access, &bank)
            .ok()
            .unwrap(),
    };
    let registry = ConsumerActionRegistry::try_new_fixed(vec![ConsumerActionRegistration {
        preparation: Box::new(HttpActionConsumer::new(
            config,
            BindingId::new(702),
            BindingGeneration::INITIAL,
            1,
        )),
        services: services.clone(),
    }])
    .unwrap();
    let definition = registry
        .prepare_action(
            &thing,
            &args[4],
            ConsumerActionSelectors::default(),
            &limits,
            bank.clone(),
            &mut work(),
        )
        .unwrap();
    let action_association = ConsumerActionAssociation::try_new(
        instance.consumer(),
        Rc::new(definition),
        ConsumerActionAccess {
            invoke: services.access.clone(),
            query: Some(services.access.clone()),
            cancel: Some(services.access.clone()),
        },
        bank.clone(),
        &mut work(),
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    drop((registry, services));

    let mqtt = MqttConsumerService::new(
        MqttConsumerConfig::for_loopback(broker, 8192).unwrap(),
        BindingId::new(703),
        BindingGeneration::INITIAL,
    )
    .unwrap();
    let services = RetainedConsumerServices {
        data: ConsumerServiceOwner::retain_data(Arc::new(MqttJsonData), &bank)
            .ok()
            .unwrap(),
        access: ConsumerServiceOwner::retain_access(
            Arc::new(EventAccess {
                consumer: instance.consumer(),
                endpoint: format!("mqtt://{broker}"),
            }),
            &bank,
        )
        .ok()
        .unwrap(),
    };
    let registry = ConsumerNotificationRegistry::try_from_retained(vec![
        RetainedConsumerNotificationRegistration {
            binding: mqtt.binding(),
            services: services.clone(),
        },
    ])
    .unwrap();
    let definition = registry
        .prepare_notification(
            &thing,
            &AffordanceTarget::Event(Arc::from(args[5].as_str())),
            NotificationKind::Event,
            ConsumerSelector::default(),
            &limits,
            bank.clone(),
            &mut work(),
        )
        .unwrap();
    let notification_association = ConsumerNotificationAssociation::try_new(
        instance.consumer(),
        Rc::new(definition),
        services.access.clone(),
        bank.clone(),
        &mut work(),
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    drop((thing, registry, services));

    let mut calls = ConsumerCalls::try_new(bank.clone(), 1).unwrap();
    let mut actions = ConsumerActions::try_new(1, bank.clone()).unwrap();
    let mut notifications = ConsumerNotifications::try_new(
        RawNotificationLimits {
            subscriptions: 1,
            queue_items: 4,
            queue_payload_bytes: 24_000,
            payload_backing_bytes: 4608,
            control_backing_bytes: 0,
            target_name_bytes: 64,
            engine_bytes: 128_000,
        },
        bank.clone(),
    )
    .unwrap();
    let io = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let clock = Instant::now();
    let now = || MonotonicInstant::new(ClockId::new(701), clock.elapsed().as_millis() as u64);
    io.block_on(async {
        let deadline = Deadline::at(now().checked_add_ticks(10_000).unwrap());
        let mut read = Some(
            owner
                .read_property(
                    instance.thing_id(),
                    "http",
                    &calls,
                    &args[3],
                    deadline,
                    now(),
                    &mut work(),
                )
                .unwrap(),
        );
        let mut action = Some(
            actions
                .invoke_action(
                    &action_association,
                    (args[6] != "-").then(|| Payload::new(args[6].as_bytes(), "application/json")),
                    deadline,
                    now(),
                    &mut work(),
                )
                .unwrap_or_else(|e| panic!("{:?}", e.error())),
        );
        let mut cursor = Some(
            notifications
                .subscribe(
                    &notification_association,
                    SubscriptionInput(None),
                    ResponseRequirement::None,
                    NotificationOverflow::Terminate,
                    &mut work(),
                )
                .unwrap_or_else(|e| panic!("{:?}", e.error())),
        );
        let mut pulse = Box::pin(tokio::time::sleep(Duration::from_millis(10)));
        let mut next_query = Instant::now();
        let mut closing = false;
        poll_fn(|cx| {
            // The deployment supplies this timer and its clock/credit policy.
            // Native readiness can run an earlier turn; the pulse also covers
            // passive authority loss, deadlines and Ready-to-next-phase work.
            if pulse.as_mut().poll(cx).is_ready() {
                pulse
                    .as_mut()
                    .reset(tokio::time::Instant::now() + Duration::from_millis(10));
                let _ = pulse.as_mut().poll(cx);
            }
            let idle = step_consumers(
                &mut [&mut calls, &mut actions, &mut notifications],
                cx,
                now(),
                |_| work(),
            );
            if closing {
                return if idle { Poll::Ready(()) } else { Poll::Pending };
            }
            // Application results/decisions share this Host task with progress.
            if let Some(call) = read.as_mut()
                && let Poll::Ready(result) = call.poll(
                    cx,
                    now(),
                    &mut work().with_remaining(WorkClass::BindingPolls, 0),
                )
            {
                eprintln!("Property: {:?}", result.result());
                read = None;
            }
            if let Some(handle) = &action {
                match handle.state().unwrap() {
                    ConsumerActionState::Pending if Instant::now() >= next_query => {
                        handle.query(now(), &mut work()).unwrap(); // Explicit application decision.
                        next_query = Instant::now() + Duration::from_millis(100);
                    }
                    ConsumerActionState::Completed
                    | ConsumerActionState::Failed
                    | ConsumerActionState::Closed => {
                        let result = handle.take_settlement(now(), &mut work()).unwrap();
                        eprintln!("Action: {:?}", result.result());
                        action = None;
                    }
                    _ => {}
                }
            }
            if let Some(receiver) = &mut cursor {
                if notifications.status(receiver.identity()).unwrap().state
                    == clinkz_wot_servient_finite::notification::NotificationState::Closed
                {
                    let report = notifications.take_settlement(receiver.identity()).unwrap();
                    eprintln!("Event closed: {:?}", report.first_cause);
                    cursor = None;
                } else if notifications.status(receiver.identity()).unwrap().state
                    == clinkz_wot_servient_finite::notification::NotificationState::Active
                    && let Some(event) = notifications.front(receiver, &mut work()).unwrap()
                {
                    eprintln!("Event: {:?}", event.data);
                    notifications.advance(receiver, &mut work()).unwrap();
                }
            }
            if clock.elapsed() >= Duration::from_secs(10) {
                owner.begin_close();
                drop((read.take(), action.take(), cursor.take()));
                closing = true;
            }
            Poll::Pending
        })
        .await;
    });
    drop((
        calls,
        actions,
        notifications,
        action_association,
        notification_association,
        owner,
        instance,
        mqtt,
    ));
    assert!(
        ResourceKind::ALL
            .into_iter()
            .all(|k| bank.account(k).used() == 0)
    );
}
