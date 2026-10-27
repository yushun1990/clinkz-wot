//! Run: cargo run -p clinkz-wot-binding-http --example serve_property -- 8080 30
//! Read: curl --http1.1 -H 'Connection: close' http://127.0.0.1:8080/value
//! Publishes one completely qualified v6 TD through the joined finite path.
use clinkz_wot_binding_http::*;
use clinkz_wot_core::{finite::RawFiniteHandler, producer::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_servient::{host_accounts::HostResourceAccounts, producer::ProducerServient};
use clinkz_wot_servient_finite::RawFiniteState;
use std::{
    net::SocketAddr,
    sync::Arc,
    task::{Context, Waker},
    time::{Duration, Instant},
};

struct Temperature;
impl RawFiniteHandler for Temperature {
    fn authorize(&self, _: &InteractionInput) -> CoreResult<()> {
        Ok(())
    }
    fn handle(
        &self,
        context: HandlerContext<'_>,
        _: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        assert_eq!(context.target().name(), Some("value"));
        Ok(InteractionOutput::with_data(Payload::new(
            b"21.5".as_slice(),
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
    let port = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "8080".into())
        .parse::<u16>()
        .expect("port");
    let seconds = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "30".into())
        .parse::<u64>()
        .expect("duration seconds");
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let compiler = HttpFiniteProducer::try_new(
        HttpProducerConfig::for_loopback(address).unwrap(),
        BindingId::new(601),
        BindingGeneration::INITIAL,
        5_000,
    )
    .unwrap();
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let mut servient =
        ProducerServient::try_new(compiler, GatewayDefaultV1::LIMITS.clone(), bank).unwrap();
    let authority = ProducerInstanceAuthority::try_new(ThingSlotId::new(
        SlotIndex::new(0),
        Generation::INITIAL,
    ))
    .unwrap();
    let description = serde_json::json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"joined Producer",
        "securityDefinitions":{"n":{"scheme":"nosec"}}, "security":["n"],
        "properties":{"value":{"type":"number","readOnly":true,"forms":[{"href":format!("http://{address}/value")}]}}
    }).to_string();
    let mut exposure = servient
        .prepare(
            Arc::from(description),
            Arc::from("value"),
            authority,
            Temperature,
            0,
            None,
            Deadline::NONE,
            &mut work(),
        )
        .unwrap();
    let start = Instant::now();
    let mut announced = false;
    loop {
        if start.elapsed() >= Duration::from_secs(seconds) {
            exposure.begin_close();
        }
        let now = MonotonicInstant::new(
            ClockId::new(1),
            start.elapsed().as_millis().try_into().unwrap(),
        );
        let _ = servient
            .step(
                &mut exposure,
                &mut Context::from_waker(Waker::noop()),
                &mut work(),
                now,
                false,
            )
            .unwrap();
        if exposure.state() == RawFiniteState::Serving && !announced {
            println!("Serving http://{address}/value for {seconds} seconds");
            println!(
                "Final TD: {}",
                servient.description(exposure.key()).unwrap()
            );
            announced = true;
        }
        if let Some(settlement) = exposure.take_settlement() {
            println!("{settlement:?}");
        }
        if exposure.state() == RawFiniteState::Closed {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    servient.retire(&mut exposure, &mut work()).unwrap();
    if let Some(error) = exposure.first_cause() {
        panic!("service closed: {error:?}");
    }
}
