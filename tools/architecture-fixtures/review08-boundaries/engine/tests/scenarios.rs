use review08_core::*;
use review08_engine::Engine;
use review08_planning::{ReadCompiler, Step};
use review08_td::{ResolvedInstance, ResolvedRead};
use review08_third_party_binding::{
    Counts, MappingPolicy, NativeOwner, Observation, Program, Route, prepare::Compiler,
};
use std::{
    net::UdpSocket,
    rc::Rc,
    task::Poll,
    time::{Duration, Instant},
};

fn facts(prefix: &str) -> ResolvedRead<'_> {
    ResolvedRead {
        operation: Operation::ReadProperty,
        result: ResultShape::Boolean,
        native_prefix: prefix,
    }
}
fn definition(compiler: &Compiler) -> Rc<Definition<Program>> {
    let source = String::from("sensor/");
    let input = facts(&source);
    let cursor = compiler.start(&input);
    let Step::Pending(cursor) = compiler.step(cursor, &mut 0) else {
        panic!("zero credit advanced");
    };
    assert_eq!(compiler.0.compiles.get(), 0);
    let Step::Pending(cursor) = compiler.step(cursor, &mut 1) else {
        panic!("lost continuation");
    };
    let Step::Ready(native) = compiler.step(cursor, &mut 1) else {
        panic!("not complete");
    };
    review08_planning::finish::<Compiler>(&input, native)
    // input/source loans end here. Program contains its own Box<str>.
}
fn bound(
    compiler: &Compiler,
    definition: &Rc<Definition<Program>>,
    suffix: &str,
) -> Bound<Program, review08_third_party_binding::Operands> {
    review08_planning::bind(
        compiler,
        definition.clone(),
        &ResolvedInstance {
            definition: facts("sensor/"),
            suffix,
        },
    )
    .ok()
    .unwrap()
}
fn endpoint(name: &str) -> Endpoint {
    Endpoint(name.into())
}
fn observe(owner: &NativeOwner, client: &UdpSocket, key: &[u8]) -> Observation {
    client.send_to(key, owner.address()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(input) = owner.observe().unwrap() {
            return input;
        }
        assert!(Instant::now() < deadline, "loopback receive stalled");
        std::thread::yield_now();
    }
}
fn client() -> UdpSocket {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    socket
}
fn reply(client: &UdpSocket) -> Vec<u8> {
    let mut bytes = [0; 16];
    let (n, _) = client.recv_from(&mut bytes).unwrap();
    bytes[..n].to_vec()
}

#[test]
fn compile_once_bind_independent_authority_share_io_and_retire_actual_last_user() {
    let counts = Rc::new(Counts::default());
    let compiler = Compiler(counts.clone(), MappingPolicy::TextBoolean);
    let definition = definition(&compiler);
    let weak = Rc::downgrade(&definition);
    let owner = NativeOwner::new(counts.clone());
    let extra_driver_root = owner.clone(); // Root reference is not a native user.

    assert!(
        review08_planning::bind(
            &compiler,
            definition.clone(),
            &ResolvedInstance {
                definition: facts("different/"),
                suffix: "a"
            }
        )
        .is_err()
    );
    assert!(
        review08_planning::bind(
            &compiler,
            definition.clone(),
            &ResolvedInstance {
                definition: facts("sensor/"),
                suffix: "a/b"
            }
        )
        .is_err()
    );
    assert_eq!(
        (
            counts.starts.get(),
            counts.compiles.get(),
            counts.opens.get()
        ),
        (1, 1, 0)
    );

    let mut engine = Engine::<Route>::default();
    engine.grant(10, endpoint("sensor/a"));
    engine.grant(20, endpoint("sensor/b"));
    let other_compiler = Compiler(counts.clone(), MappingPolicy::DigitBoolean);
    assert!(
        review08_planning::bind(
            &other_compiler,
            definition.clone(),
            &ResolvedInstance {
                definition: facts("sensor/"),
                suffix: "a"
            }
        )
        .is_err()
    );
    let other_runtime = NativeOwner::with_policy(counts.clone(), MappingPolicy::DigitBoolean);
    let refused = engine.install(
        1,
        10,
        other_runtime.route(bound(&compiler, &definition, "a")),
        || true,
    );
    assert!(
        refused.is_err(),
        "same Rust type is insufficient for configuration compatibility"
    );
    drop(refused);
    let rejected = engine.install(
        2,
        10,
        owner.route(bound(&compiler, &definition, "b")),
        || false,
    );
    let Err((_, refused_route)) = rejected else {
        panic!("borrowed another instance's authorization");
    };
    drop(refused_route);
    assert_eq!(
        counts.opens.get(),
        0,
        "authorization must precede native effects"
    );

    let a = engine
        .install(
            1,
            10,
            owner.route(bound(&compiler, &definition, "a")),
            || true,
        )
        .ok()
        .unwrap();
    let b = engine
        .install(
            2,
            20,
            owner.route(bound(&compiler, &definition, "b")),
            || false,
        )
        .ok()
        .unwrap();
    assert_eq!(
        (
            counts.starts.get(),
            counts.compiles.get(),
            counts.opens.get()
        ),
        (1, 1, 1)
    );
    drop(compiler);
    drop(definition);
    let client_a = client();
    let client_b = client();
    assert!(
        engine
            .accept(observe(&owner, &client_b, b"sensor/b"))
            .is_none()
    );
    assert!(
        engine.dispatched.is_empty(),
        "native I/O is not semantic acceptance"
    );
    assert!(engine.publish(&[a, b]));

    engine.revoke(10, endpoint("sensor/a"));
    assert!(
        engine
            .accept(observe(&owner, &client_a, b"sensor/a"))
            .is_none()
    );
    engine.grant(10, endpoint("sensor/a"));
    let mut call_a = engine
        .accept(observe(&owner, &client_a, b"sensor/a"))
        .unwrap();
    assert!(engine.stop(a));
    assert_eq!(owner.users(), (1, 0, 1, 0));
    assert!(weak.upgrade().is_some());
    assert!(
        engine
            .accept(observe(&owner, &client_a, b"sensor/a"))
            .is_none()
    );
    let mut call_b = engine
        .accept(observe(&owner, &client_b, b"sensor/b"))
        .unwrap();
    assert_eq!(call_b.id, b);
    assert_eq!(call_b.poll_reply(), Ok(true));
    assert_eq!(reply(&client_b), b"false");
    assert_eq!(engine.dispatched, [a, b]);
    assert_eq!(
        counts.closes.get(),
        0,
        "stopping a must preserve b's socket"
    );

    let callback = owner.hold_native_job();
    assert!(engine.stop(b));
    assert!(!engine.stop(a));
    assert_eq!(owner.users(), (0, 0, 1, 1));
    assert_eq!(owner.poll_retire(), Poll::Pending);
    owner.fail_next_reply();
    assert!(call_a.poll_reply().is_err());
    assert_eq!(
        owner.users(),
        (0, 0, 1, 1),
        "failure retains program and native reply owner"
    );
    assert_eq!(counts.program_drops.get(), 0);
    assert_eq!(call_a.poll_reply(), Ok(true));
    assert_eq!(reply(&client_a), b"true");
    assert!(weak.upgrade().is_none());
    assert_eq!(counts.program_drops.get(), 1);
    assert_eq!(
        owner.poll_retire(),
        Poll::Pending,
        "callback is still a native user"
    );
    drop(callback);
    assert_eq!(owner.poll_retire(), Poll::Ready(()));
    assert_eq!(extra_driver_root.poll_retire(), Poll::Ready(()));
    drop(extra_driver_root);
    drop(owner);
    assert_eq!(
        (
            counts.opens.get(),
            counts.closes.get(),
            counts.program_drops.get()
        ),
        (1, 1, 1)
    );
}

#[test]
fn foreign_route_and_stale_permits_cannot_accept_on_shared_infrastructure() {
    let counts = Rc::new(Counts::default());
    let compiler = Compiler(counts.clone(), MappingPolicy::TextBoolean);
    let definition = definition(&compiler);
    let owner = NativeOwner::new(counts.clone());
    let root = Authority::default();
    let id_a = RouteId {
        slot: 1,
        generation: 1,
    };
    let gate_a = root.gate(id_a);
    let gate_b = root.gate(RouteId {
        slot: 2,
        generation: 1,
    });
    let mut a = owner.route(bound(&compiler, &definition, "a"));
    let mut b = owner.route(bound(&compiler, &definition, "b"));
    a.install(gate_a.clone()).unwrap();
    b.install(gate_b.clone()).unwrap();
    assert!(root.with_permit(&gate_a, |_| ()).is_none());
    root.set_serving(&gate_a, true);
    root.set_serving(&gate_b, true);
    let socket = client();
    let input = observe(&owner, &socket, b"sensor/b");
    let input = root
        .with_permit(&gate_a, |permit| b.accept(permit, input))
        .unwrap()
        .err()
        .unwrap();

    let impostor = Authority::default();
    let fake = impostor.gate(RouteId {
        slot: 2,
        generation: 1,
    });
    impostor.set_serving(&fake, true);
    let input = impostor
        .with_permit(&fake, |permit| b.accept(permit, input))
        .unwrap()
        .err()
        .unwrap();
    let mut delivery = root
        .with_permit(&gate_b, |permit| b.accept(permit, input))
        .unwrap()
        .ok()
        .unwrap();
    assert_eq!(delivery.poll_reply(false), Ok(true));
    assert_eq!(reply(&socket), b"false");
    drop(delivery);

    a.close();
    let mut successor = owner.route(bound(&compiler, &definition, "a"));
    let new_gate = root.gate(RouteId {
        slot: 1,
        generation: 2,
    });
    successor.install(new_gate.clone()).unwrap();
    root.set_serving(&new_gate, true);
    // Deliberately keep the old gate live to isolate exact generation checking.
    let input = observe(&owner, &socket, b"sensor/a");
    let input = root
        .with_permit(&gate_a, |permit| successor.accept(permit, input))
        .unwrap()
        .err()
        .unwrap();
    let mut delivery = root
        .with_permit(&new_gate, |permit| successor.accept(permit, input))
        .unwrap()
        .ok()
        .unwrap();
    assert_eq!(delivery.poll_reply(true), Ok(true));
    assert_eq!(reply(&socket), b"true");
    drop(delivery);
    root.set_serving(&new_gate, false);
    assert!(root.with_permit(&new_gate, |_| ()).is_none());
    successor.close();
    b.close();
    assert_eq!(owner.poll_retire(), Poll::Ready(()));
}

#[test]
fn engine_does_not_retarget_buffered_input_to_a_reused_slot() {
    let counts = Rc::new(Counts::default());
    let compiler = Compiler(counts.clone(), MappingPolicy::TextBoolean);
    let definition = definition(&compiler);
    let owner = NativeOwner::new(counts);
    let mut engine = Engine::<Route>::default();
    engine.grant(10, endpoint("sensor/a"));
    engine.grant(20, endpoint("sensor/b"));
    let a = engine
        .install(
            1,
            10,
            owner.route(bound(&compiler, &definition, "a")),
            || true,
        )
        .ok()
        .unwrap();
    let b = engine
        .install(
            2,
            20,
            owner.route(bound(&compiler, &definition, "b")),
            || false,
        )
        .ok()
        .unwrap();
    assert!(engine.publish(&[a, b]));
    let socket = client();
    let buffered = engine.route(observe(&owner, &socket, b"sensor/a")).unwrap();
    assert!(engine.stop(a));
    let successor = engine
        .install(
            1,
            10,
            owner.route(bound(&compiler, &definition, "a")),
            || false,
        )
        .ok()
        .unwrap();
    assert_eq!(successor.slot, a.slot);
    assert!(successor.generation > a.generation);
    assert!(engine.publish(&[successor]));
    assert!(engine.dispatch(buffered).is_none());
    assert!(engine.dispatched.is_empty());
    let mut current = engine
        .accept(observe(&owner, &socket, b"sensor/a"))
        .unwrap();
    assert_eq!(current.id, successor);
    assert_eq!(current.poll_reply(), Ok(true));
    assert_eq!(reply(&socket), b"false");
    let unaccepted = engine.route(observe(&owner, &socket, b"sensor/b")).unwrap();
    assert!(engine.stop(successor));
    assert!(engine.stop(b));
    assert_eq!(owner.users(), (0, 1, 0, 0));
    assert_eq!(owner.poll_retire(), Poll::Pending);
    drop(unaccepted);
    assert_eq!(owner.poll_retire(), Poll::Ready(()));
}

#[test]
fn early_retirement_poll_preserves_sibling_ingress_and_attachment() {
    let counts = Rc::new(Counts::default());
    let compiler = Compiler(counts.clone(), MappingPolicy::TextBoolean);
    let definition = definition(&compiler);
    let owner = NativeOwner::new(counts.clone());
    let clone = owner.clone();
    let mut engine = Engine::<Route>::default();
    engine.grant(10, endpoint("sensor/a"));
    engine.grant(20, endpoint("sensor/b"));
    let a = engine
        .install(
            1,
            10,
            owner.route(bound(&compiler, &definition, "a")),
            || true,
        )
        .ok()
        .unwrap();
    let b = engine
        .install(
            2,
            20,
            owner.route(bound(&compiler, &definition, "b")),
            || false,
        )
        .ok()
        .unwrap();
    assert!(engine.publish(&[a, b]));
    assert!(engine.stop(a));
    assert_eq!(owner.users(), (1, 0, 0, 0));
    assert_eq!(clone.poll_retire(), Poll::Pending);
    let socket = client();
    let mut first = engine
        .accept(observe(&owner, &socket, b"sensor/b"))
        .unwrap();
    assert_eq!(first.poll_reply(), Ok(true));
    assert_eq!(reply(&socket), b"false");
    let replacement = engine
        .install(
            1,
            10,
            owner.route(bound(&compiler, &definition, "a")),
            || true,
        )
        .ok()
        .unwrap();
    assert!(engine.publish(&[replacement]));
    assert!(engine.stop(replacement));
    assert_eq!(clone.poll_retire(), Poll::Pending);
    let mut last = engine
        .accept(observe(&owner, &socket, b"sensor/b"))
        .unwrap();
    let unaccepted = observe(&owner, &socket, b"sensor/b");
    let callback = owner.hold_native_job();
    assert!(engine.stop(b));
    assert_eq!(owner.users(), (0, 1, 1, 1));
    assert_eq!(clone.poll_retire(), Poll::Pending);
    drop(unaccepted);
    assert_eq!(clone.poll_retire(), Poll::Pending);
    assert_eq!(last.poll_reply(), Ok(true));
    assert_eq!(reply(&socket), b"false");
    assert_eq!(clone.poll_retire(), Poll::Pending);
    drop(callback);
    assert_eq!(clone.poll_retire(), Poll::Ready(()));
    assert_eq!(owner.poll_retire(), Poll::Ready(()));
    assert_eq!((counts.opens.get(), counts.closes.get()), (1, 1));
}
