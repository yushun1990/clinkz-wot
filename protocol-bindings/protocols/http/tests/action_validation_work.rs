#![cfg(feature = "prepare")]
//! Real managed HTTP ingress must fund the captured validator before effects
//! and before polling a continuation that can return a structured result.
use clinkz_wot_binding_http::*;
use clinkz_wot_core::*;
use clinkz_wot_foundation::*;
use clinkz_wot_servient::{host_accounts::HostResourceAccounts, producer_composition::*};
use clinkz_wot_servient_finite::*;
use serde_json::{Map, Value, json};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    rc::Rc,
    sync::Arc,
    task::Poll,
    time::{Duration, Instant},
};
#[path = "support/action.rs"]
mod fixture;
use fixture::*;

// Existing fixture's complete scalar input/output/lookup envelopes. These are
// deliberate regression credits: formerly they also admitted 32 object scans.
const SCALAR_INVOKE: u64 = 8654;
const SCALAR_PROGRESS: u64 = 4229;
const STRUCTURED_CALLBACK: u64 = 4096 * 80;

fn description(address: SocketAddr, input: bool, output: bool) -> (Arc<str>, Vec<u8>) {
    let mut text: Value = serde_json::from_str(&td(address, None)).unwrap();
    let mut fields = Map::new();
    let mut values = Map::new();
    for i in 0..32 {
        let name = format!("f{i:02}");
        fields.insert(
            name.clone(),
            json!({"type":"string", "minLength":100,"maxLength":100}),
        );
        values.insert(name, Value::String("x".repeat(100)));
    }
    let schema = json!({"type":"object", "required": fields.keys().collect::<Vec<_>>(),
        "properties":fields});
    for (role, structured) in [("input", input), ("output", output)] {
        text["actions"]["run"][role] = if structured {
            schema.clone()
        } else {
            json!({"type":"object"})
        };
    }
    let body = serde_json::to_vec(&values).unwrap();
    assert_eq!(body.len(), 3489);
    (Arc::from(text.to_string()), body)
}

fn send(
    owner: &mut RawProducerServient,
    exposure: &mut ComposedProducerExposure,
    address: SocketAddr,
    body: &[u8],
    credits: &mut WorkBudget,
) -> (String, HttpActionDelivery) {
    let mut peer = TcpStream::connect(address).unwrap();
    peer.set_nonblocking(true).unwrap();
    peer.write_all(format!("POST /run HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n", body.len()).as_bytes()).unwrap();
    peer.write_all(body).unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    let mut bytes = Vec::new();
    let mut delivery = None;
    loop {
        let native = exposure.participant_mut::<HttpActionRuntime>(0).unwrap();
        let _ = owner.step_actions(native, credits).unwrap();
        if delivery.is_none() {
            delivery = native.take_delivery();
        }
        let mut buffer = [0; 512];
        match peer.read(&mut buffer) {
            Ok(0) if delivery.is_some() => {
                return (String::from_utf8(bytes).unwrap(), delivery.unwrap());
            }
            Ok(n) => bytes.extend_from_slice(&buffer[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("{e}"),
        }
        assert!(Instant::now() < end, "HTTP work-credit regression stalled");
    }
}
fn credits(handler: u64) -> WorkBudget {
    budget()
        .with_remaining(WorkClass::HandlerSteps, handler)
        .with_remaining(WorkClass::CodecInputBytes, 0)
}

#[test]
fn managed_http_funds_structured_input_immediate_and_continuing_output_before_callbacks() {
    for (input, output) in [(false, false), (true, false), (false, true), (true, true)] {
        for immediate in [false, true] {
            let address = addr();
            let (text, body) = description(address, input, output);
            let mut owner = owner();
            let counts = Rc::new(Counters::default());
            *counts.input.borrow_mut() = Some(body.clone());
            *counts.output.borrow_mut() = Some(body.clone());
            counts.immediate.set(immediate);
            let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
            let mut exposure = create_from_text(
                &owner,
                key(Generation::INITIAL),
                text,
                address,
                None,
                counts.clone(),
                None,
                bank.clone(),
            );
            exposure
                .participant_mut::<HttpActionRuntime>(0)
                .unwrap()
                .install_closed()
                .unwrap();
            exposure.publish(&mut owner, &mut budget()).unwrap();
            let additional = if input || output {
                STRUCTURED_CALLBACK
            } else {
                0
            };
            let invoke_work = SCALAR_INVOKE + additional * 2;
            for short in [SCALAR_INVOKE - 1, invoke_work - 1] {
                let (wire, report) = send(
                    &mut owner,
                    &mut exposure,
                    address,
                    &body,
                    &mut credits(short),
                );
                assert!(wire.starts_with("HTTP/1.1 503"));
                assert!(matches!(
                    report.request_error,
                    Some(CoreError::Backpressure(_))
                ));
                assert_eq!(
                    counts.authorized.get(),
                    0,
                    "short work must precede permission/effects"
                );
                assert_eq!(counts.invoked.get(), 0);
                assert_eq!(
                    exposure
                        .participant_mut::<HttpActionRuntime>(0)
                        .unwrap()
                        .records(),
                    0
                );
            }
            let mut enough = credits(invoke_work);
            let (wire, report) = send(&mut owner, &mut exposure, address, &body, &mut enough);
            assert!(wire.starts_with("HTTP/1.1 202"), "{wire}");
            assert_eq!(enough.remaining(WorkClass::HandlerSteps), 0);
            assert_eq!(counts.invoked.get(), 1);
            let id = report.invocation.unwrap();
            let native = exposure.participant_mut::<HttpActionRuntime>(0).unwrap();
            let handle = owner.action_handle(native, id).unwrap();
            if !immediate {
                counts.complete.set(true);
                let work = SCALAR_PROGRESS + additional;
                for short in [SCALAR_PROGRESS - 1, work - 1] {
                    assert_eq!(
                        owner.step_raw_action(&handle, &mut credits(short)).unwrap(),
                        Poll::Pending
                    );
                    assert_eq!(
                        counts.polls.get(),
                        0,
                        "no output is materialized before full funding"
                    );
                    assert!(owner.acknowledge_action(native, id).unwrap().is_none());
                }
                let mut enough = credits(work);
                assert_eq!(
                    owner.step_raw_action(&handle, &mut enough).unwrap(),
                    Poll::Ready(())
                );
                assert_eq!(enough.remaining(WorkClass::HandlerSteps), 0);
                assert_eq!(counts.polls.get(), 1);
            }
            let result = owner.acknowledge_action(native, id).unwrap().unwrap();
            assert_eq!(result.result().unwrap().data().unwrap().body.as_ref(), body);
            exposure.begin_close();
            exposure.retire(&mut owner, &mut budget()).unwrap();
            drop((exposure, owner, handle));
            assert!(bank.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0);
            drop(result);
            for kind in ResourceKind::ALL {
                assert_eq!(bank.account(kind).used(), 0, "{kind:?}");
            }
            TcpListener::bind(address).unwrap();
        }
    }
}
