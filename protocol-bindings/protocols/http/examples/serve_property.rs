//! Run: cargo run -p clinkz-wot-binding-http --example serve_property -- 8080 30
//! Read: curl --http1.1 -H 'Connection: close' http://127.0.0.1:8080/value
use clinkz_wot_binding_http::*;
use clinkz_wot_core::{producer::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::{ProducerPropertyReadAdmission, ProducerPropertyReadStep};
use clinkz_wot_servient::{
    Servient, ServientBuilder, host_accounts::HostResourceAccounts,
    producer_installation::ProducerInstallationState as State,
};
use clinkz_wot_td::{
    ValidatedThingAdmissionConfig, ValidatedThingCursor, ValidatedThingProgress, thing::Thing,
};
use std::{
    net::SocketAddr,
    sync::Arc,
    task::{Context, Waker},
    time::{Duration, Instant},
};

struct Temperature;
impl ReadPropertyHandler for Temperature {
    fn handle(
        &self,
        cx: HandlerContext<'_>,
        _: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        assert_eq!(cx.target().name(), Some("value"));
        Ok(InteractionOutput::with_data(Payload::new(
            b"21.5".as_slice(),
            "application/json",
        )))
    }
}
type Server = Servient<HttpProducerExecutor, Temperature>;
fn work(n: u64) -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, k| b.with_remaining(k, n))
}
fn accepted<T, I>(result: Result<T, BindingInputRejection<I>>) -> T {
    result.unwrap_or_else(|e| panic!("{:?}", e.error()))
}
fn prepare(address: SocketAddr, bank: Arc<HostResourceAccounts>) -> Server {
    let authority = ProducerInstanceAuthority::try_new(ThingSlotId::new(
        SlotIndex::new(0),
        Generation::INITIAL,
    ))
    .unwrap();
    // All source and compiler owners die before the first native socket poll.
    let td: Thing = serde_json::from_value(serde_json::json!({"@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"real Producer", "securityDefinitions":{"n":{"scheme":"nosec"}},"security":["n"], "properties":{"value":{"type":"number","readOnly":true,"forms":[{"href":format!("http://{address}/value")} ]}}})).unwrap();
    let config = ValidatedThingAdmissionConfig::try_from_limits(GatewayDefaultV1::LIMITS).unwrap();
    let mut cursor = ValidatedThingCursor::from_thing(
        &td,
        &config,
        AdmissionLedger::new(
            SlotIndex::new(0),
            Generation::INITIAL,
            0,
            8 << 20,
            0,
            0,
            0,
            0,
        ),
    );
    let proof = loop {
        cursor = match cursor.step(&mut work(1_000_000), false) {
            ValidatedThingProgress::Complete(p) => break p,
            ValidatedThingProgress::Pending(c) => c,
            ValidatedThingProgress::Failed(e) => panic!("{e:?}"),
        };
    };
    let mut admission = ProducerPropertyReadAdmission::new(proof, "value");
    while admission.step(&mut work(1_000_000), false).unwrap() != ProducerPropertyReadStep::Covered
    {
    }
    let pair = accepted(try_producer_registration(
        HttpProducerConfig::for_loopback(address).unwrap(),
        BindingId::new(601),
        BindingGeneration::INITIAL,
        0,
    ));
    let handler = Temperature;
    let prepared = accepted(pair.prepare_property_read(
        &mut admission,
        &authority,
        ProducerHandlerRegistration::new(
            &authority,
            "value".into(),
            handler,
            HandlerFootprint::new(std::mem::size_of::<Temperature>() as u64, 0, 0),
        ),
        &mut work(1_000_000),
    ));
    accepted(
        ServientBuilder::new(GatewayDefaultV1::LIMITS.clone(), bank).produce(
            prepared,
            authority,
            Deadline::NONE,
        ),
    )
}
fn main() {
    let port: u16 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "8080".into())
        .parse()
        .expect("port");
    let seconds: u64 = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "30".into())
        .parse()
        .expect("duration seconds");
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let mut server = prepare(address, bank);
    server.begin_expose(5_000).unwrap(); // caller clock uses milliseconds
    let start = Instant::now();
    let mut announced = false;
    loop {
        if start.elapsed() >= Duration::from_secs(seconds) {
            server.begin_close();
        }
        let now = MonotonicInstant::new(
            ClockId::new(1),
            start.elapsed().as_millis().try_into().unwrap(),
        );
        let _ = server.step(&mut Context::from_waker(Waker::noop()), &mut work(1), now);
        if server.state() == State::Serving && !announced {
            println!("Serving http://{address}/value for {seconds} seconds");
            announced = true;
        }
        if server.state() == State::Closed {
            break;
        }
        // This adapter is manually progressed, not driven by an OS reactor.
        std::thread::sleep(Duration::from_millis(1));
    }
    if let Some(error) = server.first_cause() {
        panic!("service closed: {error:?}");
    }
}
