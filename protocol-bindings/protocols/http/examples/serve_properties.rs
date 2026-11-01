//! cargo run -p clinkz-wot-binding-http --example serve_properties -- 8082 30 > TD.json
//! cargo run -p clinkz-wot-binding-http --example read_all_properties -- TD.json
//! One completely qualified Thing, one native listener, and an application-owned
//! combined value. No simultaneous sampling/atomic snapshot guarantee is implied.
use clinkz_wot_binding_http::*;
use clinkz_wot_core::{finite::*, host_accounts::HostResourceAccounts, producer::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::producer_composition::*;
use clinkz_wot_servient::producer_composition::*;
use clinkz_wot_servient_finite::*;
use clinkz_wot_td::ValidatedProducerScope;
use std::{
    net::SocketAddr,
    rc::Rc,
    sync::Arc,
    task::{Context, Waker},
    time::{Duration, Instant},
};

struct Values;
impl RawFiniteHandler for Values {
    fn authorize(&self, _: &InteractionInput) -> CoreResult<()> {
        Ok(())
    }
    fn handle(
        &self,
        ctx: HandlerContext<'_>,
        _: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        let body: &[u8] = match ctx.target() {
            AffordanceTarget::Thing => br#"{"level":7,"on":true}"#,
            AffordanceTarget::Property(name) if name.as_ref() == "level" => b"7",
            AffordanceTarget::Property(name) if name.as_ref() == "on" => b"true",
            _ => unreachable!(),
        };
        Ok(InteractionOutput::with_data(Payload::new(
            body,
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
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"native collection read",
        "security":["n"], "securityDefinitions":{"n":{"scheme":"nosec"}},
        "forms":[{"op":"readallproperties", "href":format!("http://{addr}/properties")}],
        "properties":{
            "level":{"type":"integer","readOnly":true,"forms":[{"href":format!("http://{addr}/level")}]},
            "on":{"type":"boolean","readOnly":true,"forms":[{"href":format!("http://{addr}/on")}]}
        }
    }).to_string());
    let accounts = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let mut owner = RawProducerServient::try_empty(RawProducerRegistryLimits {
        instances: 1,
        behaviors_per_instance: 3,
        target_name_bytes: 256,
    })
    .unwrap();
    let authority = ProducerInstanceAuthority::try_new(ThingSlotId::new(
        SlotIndex::new(0),
        Generation::INITIAL,
    ))
    .unwrap();
    let instance = RawProducerInstance::new(
        authority,
        vec![
            RawFiniteBehavior::new(
                AffordanceTarget::Thing,
                Operation::ReadAllProperties,
                Values,
            ),
            RawFiniteBehavior::new(
                AffordanceTarget::Property(Arc::from("level")),
                Operation::ReadProperty,
                Values,
            ),
            RawFiniteBehavior::new(
                AffordanceTarget::Property(Arc::from("on")),
                Operation::ReadProperty,
                Values,
            ),
        ],
    )
    .with_description(description.clone());
    let key = instance.key();
    let listen = HttpProducerConfig::for_loopback(addr).unwrap();
    let provider: Rc<dyn ProducerCapability> = Rc::new(
        HttpFiniteProducer::try_new(
            listen,
            BindingId::new(601),
            BindingGeneration::INITIAL,
            5_000,
        )
        .unwrap(),
    );
    let prepared = prepare_thing(
        &description,
        key,
        GatewayDefaultV1::LIMITS,
        &mut work(),
        &[provider.as_ref()],
        |scope, op| match scope {
            ValidatedProducerScope::Thing => instance.supports(&AffordanceTarget::Thing, op),
            ValidatedProducerScope::Property(n) => instance.supports_property(n, op),
            _ => false,
        },
    )
    .unwrap();
    let mappings = prepared
        .operations()
        .iter()
        .map(|o| {
            (
                match &o.site {
                    FormSite::Thing => AffordanceTarget::Thing,
                    FormSite::Property(n) => AffordanceTarget::Property(Arc::from(n.as_str())),
                    _ => unreachable!(),
                },
                o.definition::<HttpProducerProgram>().unwrap().clone(),
            )
        })
        .collect();
    let config = HttpFiniteConfig::try_from_mappings(listen, mappings, key, 5_000)
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let native = RawFiniteRuntime::try_new(
        HttpFiniteBinding::new(
            config.clone(),
            BindingId::new(601),
            BindingGeneration::INITIAL,
        ),
        config,
        &owner,
        RawFiniteLimits {
            candidates: 1,
            input_body_bytes: 4096,
            output_body_bytes: 4096,
            output_media_capacity: 64,
            output_retained_bytes: 8192,
        },
        FiniteValidation::Required,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
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
        accounts.clone(),
        &mut work(),
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    type Native = RawFiniteRuntime<HttpFiniteBinding>;
    let mut cx = Context::from_waker(Waker::noop());
    let start = Instant::now();
    let now = || MonotonicInstant::new(ClockId::new(601), start.elapsed().as_millis() as u64);
    let _ = exposure
        .participant_mut::<Native>(0)
        .unwrap()
        .step(&mut cx, &mut work(), now());
    exposure.publish(&mut owner, &mut work()).unwrap();
    println!("{}", owner.description(key).unwrap());
    loop {
        if start.elapsed() >= Duration::from_secs(seconds) {
            exposure.begin_close();
        }
        let native = exposure.participant_mut::<Native>(0).unwrap();
        let _ = owner.step(native, &mut cx, &mut work(), now()).unwrap();
        if let Some(result) = native.take_settlement() {
            eprintln!("Collection/Property settlement: {result:?}");
        }
        if native.state() == RawFiniteState::Closed {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    exposure.retire(&mut owner, &mut work()).unwrap();
    drop(exposure);
    drop(owner);
    assert!(
        ResourceKind::ALL
            .iter()
            .all(|&k| accounts.account(k).used() == 0)
    );
}
