//! cargo run -p clinkz-wot-binding-http --example serve_properties -- 8082 30 > TD.json
//! cargo run -p clinkz-wot-binding-http --example read_all_properties -- TD.json
//! One completely qualified Thing, one native listener, and an application-owned
//! combined value. No transactional or atomic snapshot guarantee is implied.
use clinkz_wot_binding_http::*;
use clinkz_wot_core::{finite::*, host_accounts::HostResourceAccounts, *};
use clinkz_wot_foundation::*;
use clinkz_wot_servient::host_producer::*;
use std::{
    cell::RefCell,
    net::SocketAddr,
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};

struct Values(Rc<RefCell<serde_json::Value>>);
impl RawFiniteHandler for Values {
    fn authorize(&self, _: &InteractionInput) -> CoreResult<()> {
        Ok(())
    }
    fn handle(
        &self,
        ctx: HandlerContext<'_>,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        match ctx.operation() {
            Operation::WriteAllProperties | Operation::WriteProperty => {
                let value =
                    serde_json::from_slice(&input.data.as_ref().unwrap().body).map_err(|_| {
                        CoreError::Validation(ErrorContext::new(
                            ErrorPhase::Handler,
                            RetryClass::Never,
                        ))
                    })?;
                if ctx.operation() == Operation::WriteAllProperties {
                    *self.0.borrow_mut() = value;
                } else {
                    self.0.borrow_mut()[ctx.target().name().unwrap()] = value;
                }
                return Ok(InteractionOutput::empty());
            }
            Operation::ReadMultipleProperties => {
                let requested: Vec<String> =
                    serde_json::from_slice(&input.data.as_ref().unwrap().body).unwrap();
                let values = self.0.borrow();
                let selected: serde_json::Map<String, serde_json::Value> = requested
                    .into_iter()
                    .map(|name| {
                        let value = values[&name].clone();
                        (name, value)
                    })
                    .collect();
                return Ok(InteractionOutput::with_data(Payload::new(
                    serde_json::to_vec(&selected).unwrap(),
                    "application/json",
                )));
            }
            _ => {}
        }
        let values = self.0.borrow();
        let value = match ctx.target() {
            AffordanceTarget::Thing => &*values,
            AffordanceTarget::Property(name) => &values[name.as_ref()],
            _ => unreachable!(),
        };
        Ok(InteractionOutput::with_data(Payload::new(
            serde_json::to_vec(value).unwrap(),
            "application/json",
        )))
    }
}
fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, k| b.with_remaining(k, 1_000_000))
}
fn main() {
    let port: u16 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "8082".into())
        .parse()
        .unwrap();
    let seconds: u64 = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "30".into())
        .parse()
        .unwrap();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let description: Arc<str> = Arc::from(serde_json::json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"native finite collections",
        "base":format!("http://{addr}/"), "security":["n"], "securityDefinitions":{"n":{"scheme":"nosec"}},
        "forms":[{"op":"readallproperties", "href":"properties"},
            {"op":"readmultipleproperties", "href":"properties", "htv:methodName":"POST"},
            {"op":"writeallproperties", "href":"properties"}],
        "properties":{
            "level":{"type":"integer","forms":[{"href":"level"}]},
            "on":{"type":"boolean","forms":[{"href":"on"}]}
        }
    }).to_string());
    let accounts = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let config = HostProducerLimits {
        registry: RawProducerRegistryLimits {
            instances: 1,
            behaviors_per_instance: 7,
            target_name_bytes: 256,
        },
        exchange: RawFiniteLimits {
            candidates: 1,
            input_body_bytes: 4096,
            output_body_bytes: 4096,
            output_media_capacity: 64,
            output_retained_bytes: 8192,
        },
        active_exchanges: 1,
        retained_results: 2,
        candidate_bytes: 256 * 1024,
    };
    let limits = Arc::new(GatewayDefaultV1::LIMITS.clone());
    let id = BindingId::new(601);
    let generation = BindingGeneration::INITIAL;
    let mut producer = if port == 0 {
        let draft =
            HttpFiniteProducer::for_assigned_port(addr.ip(), id, generation, 5_000).unwrap();
        HostProducer::with_assigned_base(
            vec![FiniteProducerRegistration::at_base(
                draft,
                HttpFiniteProducer::at_endpoint,
            )],
            move || HttpProducerEndpoint::acquire(addr),
            4096,
            config,
            limits,
            accounts.clone(),
        )
    } else {
        let service = HttpFiniteProducer::try_new(
            HttpProducerConfig::for_loopback(addr).unwrap(),
            id,
            generation,
            5_000,
        )
        .unwrap();
        HostProducer::new(
            vec![FiniteProducerRegistration::fixed(service)],
            config,
            limits,
            accounts.clone(),
        )
    }
    .unwrap();
    let values = Rc::new(RefCell::new(serde_json::json!({"level":7,"on":true})));
    let mut behaviors = Vec::new();
    for operation in [
        Operation::ReadAllProperties,
        Operation::ReadMultipleProperties,
        Operation::WriteAllProperties,
    ] {
        behaviors.push(RawFiniteBehavior::new(
            AffordanceTarget::Thing,
            operation,
            Values(values.clone()),
        ));
    }
    for name in ["level", "on"] {
        for operation in [Operation::ReadProperty, Operation::WriteProperty] {
            behaviors.push(RawFiniteBehavior::new(
                AffordanceTarget::Property(Arc::from(name)),
                operation,
                Values(values.clone()),
            ));
        }
    }
    let mut exposure = producer
        .expose(
            description,
            behaviors,
            ExposurePolicy {
                handler_backing_bytes: 8192,
                ..Default::default()
            },
            &mut work(),
        )
        .unwrap();
    let key = exposure.key();
    let mut cx = Context::from_waker(Waker::noop());
    let start = Instant::now();
    let now = || MonotonicInstant::new(ClockId::new(601), start.elapsed().as_millis() as u64);
    loop {
        if let Poll::Ready(result) =
            producer.poll_publish(&mut exposure, &mut cx, now(), &mut work(), |_, _| work())
        {
            result.unwrap();
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    println!("{}", producer.description(key).unwrap());
    loop {
        if start.elapsed() >= Duration::from_secs(seconds) {
            exposure.begin_close();
        }
        producer
            .step(&mut exposure, &mut cx, now(), |_, _| work())
            .unwrap();
        if let Some(result) = exposure.take_finite_settlement() {
            eprintln!("Collection/Property settlement: {result:?}");
        }
        if exposure.members_retired() {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    producer.retire(&mut exposure, &mut work()).unwrap();
    drop(exposure);
    drop(producer);
    assert!(
        ResourceKind::ALL
            .iter()
            .all(|&k| accounts.account(k).used() == 0)
    );
}
