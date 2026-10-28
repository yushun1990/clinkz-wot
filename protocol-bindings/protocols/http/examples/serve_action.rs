//! cargo run -p clinkz-wot-binding-http --example serve_action -- 8081 30
//! curl -i -X POST -H 'Content-Type: application/json' --data 12 http://127.0.0.1:8081/run
//! Follow the returned Location with GET; DELETE /stop?id=<returned id> requests cancellation.
//! Caller-driven Host service: no background executor, no second logical Thing.
use clinkz_wot_binding_http::*;
use clinkz_wot_core::{action::*, producer::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::producer_composition::*;
use clinkz_wot_servient::{host_accounts::HostResourceAccounts, producer_composition::*};
use clinkz_wot_servient_finite::*;
use clinkz_wot_td::ValidatedProducerScope;
use std::{
    net::SocketAddr,
    rc::Rc,
    sync::Arc,
    task::Poll,
    time::{Duration, Instant},
};

struct Handler;
struct Job(Instant);
impl RawActionHandler for Handler {
    fn authorize(&self, _: Operation, _: &InteractionInput) -> CoreResult<()> {
        Ok(())
    }
    fn invoke(&self, _: HandlerContext<'_>, _: &InteractionInput) -> CoreResult<RawActionStart> {
        Ok(RawActionStart::Continue(Box::new(Job(Instant::now()))))
    }
}
impl RawActionWork for Job {
    fn poll(&mut self) -> Poll<CoreResult<InteractionOutput>> {
        if self.0.elapsed() < Duration::from_millis(200) {
            return Poll::Pending;
        }
        Poll::Ready(Ok(InteractionOutput::with_data(Payload::new(
            b"24".to_vec(),
            "application/json",
        ))))
    }
    fn request_cancel(&mut self) -> CoreResult<()> {
        Ok(())
    }
    fn poll_close(&mut self) -> Poll<CoreResult<()>> {
        Poll::Ready(Ok(()))
    }
    fn abort_local(self: Box<Self>) {}
}
fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, c| b.with_remaining(c, 1_000_000))
}
fn main() {
    let port = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "8081".into())
        .parse::<u16>()
        .unwrap();
    let seconds = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "30".into())
        .parse::<u64>()
        .unwrap();
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let description: Arc<str> = Arc::from(serde_json::json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"HTTP Action",
        "securityDefinitions":{"n":{"scheme":"nosec"}}, "security":["n"],
        "actions":{"run":{"synchronous":false, "input":{"type":"integer"}, "output":{"type":"integer"},
            "uriVariables":{"id":{"type":"string"}}, "forms":[
                {"href":format!("http://{address}/run"),"op":"invokeaction","htv:methodName":"POST"},
                {"href":format!("http://{address}/status{{?id}}"),"op":"queryaction","htv:methodName":"GET"},
                {"href":format!("http://{address}/stop{{?id}}"),"op":"cancelaction","htv:methodName":"DELETE"}
            ]}}
    }).to_string());
    let mut owner = RawProducerServient::try_empty(RawProducerRegistryLimits {
        instances: 1,
        behaviors_per_instance: 1,
        target_name_bytes: 64,
    })
    .unwrap();
    owner
        .enable_raw_actions(RawActionLimits {
            invocations: 8,
            application_work_bytes: 256,
            input_fields: 8,
            input_bytes: 4096 + 64,
            principal_id_bytes: 32,
            output_body_bytes: 4096,
            output_media_capacity: 64,
            output_retained_bytes: 8192,
        })
        .unwrap();
    let authority = ProducerInstanceAuthority::try_new(ThingSlotId::new(
        SlotIndex::new(0),
        Generation::INITIAL,
    ))
    .unwrap();
    let instance = RawProducerInstance::new(authority, vec![])
        .with_actions(vec![RawActionBehavior::new(
            Arc::from("run"),
            Handler,
            0,
            true,
            true,
        )])
        .with_description(Arc::clone(&description));
    let provider: Rc<dyn ProducerCapability> = Rc::new(HttpActionProducer::new(
        HttpProducerConfig::for_loopback(address).unwrap(),
        BindingId::new(601),
        BindingGeneration::INITIAL,
    ));
    let prepared = prepare_thing(
        &description,
        instance.key(),
        GatewayDefaultV1::LIMITS,
        &mut work(),
        &[provider.as_ref()],
        |scope, op| match scope {
            ValidatedProducerScope::Action(n) => instance.supports_action(n, op),
            _ => false,
        },
    )
    .unwrap();
    let definitions = prepared
        .operations()
        .iter()
        .map(|o| o.definition::<HttpActionDefinition>().unwrap().clone())
        .collect();
    let native = HttpActionRuntime::try_new(&owner, instance.key(), definitions, 8).unwrap();
    let accounts = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let mut exposure = ComposedProducerExposure::try_new(
        &owner,
        ComposedProducerInput {
            instance,
            prepared,
            providers: vec![provider],
            participants: vec![Box::new(native)],
            expected: None,
            handler_backing_bytes: 0,
        },
        Arc::new(GatewayDefaultV1::LIMITS.clone()),
        accounts,
        &mut work(),
    )
    .ok()
    .unwrap();
    exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .install_closed()
        .unwrap();
    exposure.publish(&mut owner, &mut work()).unwrap();
    println!(
        "Serving qualified TD: {}",
        owner.description(exposure.key()).unwrap()
    );
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(seconds) {
        let native = exposure.participant_mut::<HttpActionRuntime>(0).unwrap();
        let _ = native.step(&mut owner, &mut work());
        let _ = native.progress_actions(&mut owner, &mut work());
        if let Some(settlement) = native
            .reap_completed(&mut owner, Duration::from_secs(5))
            .unwrap()
        {
            println!("Expired Action settlement: {settlement:?}");
        }
        if let Some(delivery) = native.take_delivery() {
            println!("HTTP Action write: {delivery:?}");
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    exposure.begin_close();
    let native = exposure.participant_mut::<HttpActionRuntime>(0).unwrap();
    let ids: Vec<_> = native.action_ids().collect();
    for id in ids {
        let handle = native.handle(id).unwrap();
        owner.close_raw_action(&handle).unwrap();
        loop {
            match owner.step_raw_action(&handle, &mut work()).unwrap() {
                Poll::Ready(()) => break,
                Poll::Pending => std::thread::sleep(Duration::from_millis(1)),
            }
        }
        println!(
            "Action settlement: {:?}",
            native.acknowledge(&mut owner, id).unwrap()
        );
    }
    exposure.retire(&mut owner, &mut work()).unwrap();
}
