//! Independent complete publications over one actual native HTTP listener.
use super::*;

#[path = "capacity.rs"]
mod capacity;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    task::Wake,
};

struct ReentrantWake {
    config: HttpFiniteConfig,
    listener: HttpFiniteListener,
    calls: AtomicUsize,
}
impl Wake for ReentrantWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        // This safe callback does not poll the borrowed Binding again. A
        // passive native configuration check may legally reenter its listener.
        assert_eq!(
            self.config.clone().with_listener(&self.listener).unwrap(),
            self.config,
        );
        self.calls.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn idle_receive_waker_can_reenter_the_same_listener_without_deadlock() {
    let address = tcp_addr();
    let native = listener(address);
    let accounts = bank();
    let provider = compiler(address, &accounts);
    let key = key(1, Generation::INITIAL).key();
    let prepared = prepare(&td(address, "alpha", &["/alpha"]), key, &provider).unwrap();
    let config = HttpFiniteConfig::try_new(
        HttpProducerConfig::for_loopback(address).unwrap(),
        prepared.operations()[0]
            .definition::<HttpProducerProgram>()
            .unwrap()
            .clone(),
        key,
        AffordanceTarget::Property(Arc::from("alpha")),
        100,
    )
    .unwrap()
    .with_listener(&native)
    .unwrap();
    let mut binding = HttpFiniteBinding::new(
        config.clone(),
        BindingId::new(461),
        BindingGeneration::INITIAL,
    );
    assert!(matches!(binding.poll_start(&mut cx()), Poll::Ready(Ok(()))));
    let wake = Arc::new(ReentrantWake {
        config,
        listener: native,
        calls: AtomicUsize::new(0),
    });
    let (completed, completion) = mpsc::sync_channel(1);
    let worker_wake = wake.clone();
    let worker = std::thread::spawn(move || {
        let waker = Waker::from(worker_wake);
        assert!(
            binding
                .poll_receive(&mut Context::from_waker(&waker), now())
                .is_pending()
        );
        completed.send(binding).unwrap();
    });
    // A broken callback boundary must fail this regression in bounded time,
    // rather than hanging the test process on the same-thread mutex lock.
    let mut binding = completion
        .recv_timeout(Duration::from_secs(5))
        .expect("receive deadlocked while its Waker reentered the listener");
    worker.join().unwrap();
    assert_eq!(wake.calls.load(Ordering::Relaxed), 1);
    assert!(matches!(
        binding.poll_close(&mut cx(), None, None, None),
        Poll::Ready(Ok(_))
    ));
    drop(binding);
    TcpListener::bind(address).unwrap();
}

fn listener(address: SocketAddr) -> HttpFiniteListener {
    HttpFiniteListener::new(HttpProducerConfig::for_loopback(address).unwrap(), 100).unwrap()
}
fn compiler(address: SocketAddr, accounts: &Arc<HostResourceAccounts>) -> Rc<HttpFiniteProducer> {
    Rc::new(
        HttpFiniteProducer::compiling(
            HttpProducerConfig::for_loopback(address).unwrap(),
            BindingId::new(401),
            BindingGeneration::INITIAL,
            100,
            accounts.clone(),
        )
        .unwrap(),
    )
}
fn drive(owner: &mut RawProducerServient, exposures: &mut [&mut ComposedProducerExposure]) {
    drive_at(owner, exposures, now());
}
fn drive_at(
    owner: &mut RawProducerServient,
    exposures: &mut [&mut ComposedProducerExposure],
    instant: MonotonicInstant,
) {
    for exposure in exposures {
        let _ = owner
            .step(
                exposure.participant_mut::<HttpRuntime>(0).unwrap(),
                &mut cx(),
                &mut budget(),
                instant,
            )
            .unwrap();
    }
}
fn response(
    peer: TcpStream,
    owner: &mut RawProducerServient,
    exposures: &mut [&mut ComposedProducerExposure],
) -> Vec<u8> {
    response_at(peer, owner, exposures, now())
}
fn response_at(
    mut peer: TcpStream,
    owner: &mut RawProducerServient,
    exposures: &mut [&mut ComposedProducerExposure],
    instant: MonotonicInstant,
) -> Vec<u8> {
    let end = Instant::now() + Duration::from_secs(5);
    let mut output = Vec::new();
    loop {
        drive_at(owner, exposures, instant);
        let mut bytes = [0; 1024];
        match peer.read(&mut bytes) {
            Ok(0) => return output,
            Ok(n) => output.extend_from_slice(&bytes[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
                ) =>
            {
                return output;
            }
            Err(e) => panic!("{e}"),
        }
        assert!(Instant::now() < end, "shared HTTP request stalled");
    }
}
fn assert_value(response: &[u8], value: u8) {
    assert!(response.starts_with(b"HTTP/1.1 200"), "{response:?}");
    assert!(response.ends_with(format!("{{\"value\":{value}}}").as_bytes()));
}

#[test]
fn independent_qualification_publication_policy_replacement_and_release_on_one_listener() {
    let mut owner = owner();
    let address = tcp_addr();
    let native = listener(address);
    let accounts = [bank(), bank(), bank()];
    let apps = [
        Rc::new(App::default()),
        Rc::new(App::default()),
        Rc::new(App::default()),
    ];
    let provider = compiler(address, &accounts[0]);
    let first_input = input_with_listener(
        &owner,
        key(1, Generation::INITIAL),
        &td(address, "alpha", &["/alpha"]),
        provider.clone(),
        "alpha",
        11,
        apps[0].clone(),
        Some(&native),
    );
    let shared = definition(&first_input.prepared);
    let sibling_provider = reuse(address, 402, &shared);
    let second_input = input_with_listener(
        &owner,
        key(2, Generation::INITIAL),
        &td(address, "beta", &["/beta", "/alias"]),
        sibling_provider.clone(),
        "beta",
        22,
        apps[1].clone(),
        Some(&native),
    );
    let mut first = admit(&owner, first_input, &accounts[0]);
    let mut second = admit(&owner, second_input, &accounts[1]);
    start(&mut first);
    start(&mut second); // Same address succeeds without SO_REUSEPORT or a second socket.
    for exposure in [&mut first, &mut second] {
        assert_eq!(
            exposure
                .participant_mut::<HttpRuntime>(0)
                .unwrap()
                .closed_binding()
                .unwrap()
                .bound_address(),
            Some(address)
        );
    }
    assert!(TcpListener::bind(address).is_err());
    first.publish(&mut owner, &mut budget()).unwrap();
    let early = connect(address, "/beta");
    // Only the already published sibling progresses native matching. The
    // staged sibling's input stays native and cannot enter its behavior.
    for _ in 0..20 {
        drive(&mut owner, &mut [&mut first, &mut second]);
    }
    assert_eq!(apps[1].calls.get(), 0);
    assert_eq!(owner.description(second.key()), None);
    assert_value(&request(address, "/alpha", &mut owner, &mut first), 11);
    // Leave alpha's original settlement unread while beta publishes/executes.
    second.publish(&mut owner, &mut budget()).unwrap();
    assert_value(
        &response(early, &mut owner, &mut [&mut first, &mut second]),
        22,
    );
    acknowledge(&mut second);
    apps[1].denied.set(true);
    response(
        connect(address, "/alias"),
        &mut owner,
        &mut [&mut first, &mut second],
    );
    let denied = acknowledge(&mut second);
    assert_eq!(denied.effect, FiniteEffect::NotInvoked);
    assert!(matches!(
        denied.result,
        Err(CoreError::Security {
            reason: SecurityFailureReason::AuthorizationDenied,
            ..
        })
    ));
    assert_eq!(apps[1].calls.get(), 1);
    apps[1].denied.set(false);

    // Route reuse is native collision ownership: retire the old native lease,
    // retaining its accepted result, before starting the same route again.
    let old = first.key();
    let successor_provider = reuse(address, 403, &shared);
    let mut replacement_input = input_with_listener(
        &owner,
        key(1, Generation::INITIAL.checked_next().unwrap()),
        &td(address, "successor", &["/alpha"]),
        successor_provider.clone(),
        "successor",
        33,
        apps[2].clone(),
        Some(&native),
    );
    replacement_input.expected = Some(old);
    let mut replacement = admit(&owner, replacement_input, &accounts[2]);
    close(&mut first);
    assert!(first.retire(&mut owner, &mut budget()).is_err());
    start(&mut replacement);
    replacement.publish(&mut owner, &mut budget()).unwrap();
    assert_eq!(owner.description(old), None);
    assert!(
        owner
            .description(replacement.key())
            .unwrap()
            .contains("successor")
    );
    assert_value(
        &response(
            connect(address, "/alpha"),
            &mut owner,
            &mut [&mut second, &mut replacement],
        ),
        33,
    );
    acknowledge(&mut replacement);
    let retained = acknowledge(&mut first);
    assert_eq!(retained.recipient, Some(old));
    assert_eq!(retained.delivery, FiniteDelivery::Delivered);
    assert!(
        retained
            .result
            .as_ref()
            .unwrap()
            .data()
            .unwrap()
            .body
            .ends_with(b"11}")
    );
    first.retire(&mut owner, &mut budget()).unwrap();
    drop(first);
    drop(provider);
    assert_eq!(live(&accounts[0]), shared.retained_bytes() as u64);
    // An old exposure's retirement cannot close the physical listener or beta.
    apps[1].problem.set(true);
    let problem = response(
        connect(address, "/alias"),
        &mut owner,
        &mut [&mut second, &mut replacement],
    );
    assert!(problem.starts_with(b"HTTP/1.1 422"));
    assert_eq!(
        acknowledge(&mut second)
            .result
            .unwrap()
            .metadata()
            .response_selection(),
        ResponseSelection::Additional(0)
    );
    close(&mut replacement);
    replacement.retire(&mut owner, &mut budget()).unwrap();
    drop(replacement);
    drop(successor_provider);
    assert_empty(&accounts[2]);
    assert!(TcpListener::bind(address).is_err());
    assert_eq!(sibling_provider.native_compilations(), 0);
    close(&mut second);
    second.retire(&mut owner, &mut budget()).unwrap();
    drop(second);
    drop(sibling_provider);
    assert_empty(&accounts[1]);
    // The passive infrastructure handle does not keep the socket or any
    // publication bank alive after the final native participation releases.
    TcpListener::bind(address).unwrap();
    drop(native);
    drop(shared);
    assert_empty(&accounts[0]);
}

#[test]
fn native_collision_is_atomic_and_failed_publication_does_not_affect_a_sibling() {
    let mut owner = owner();
    let address = tcp_addr();
    let native = listener(address);
    let accounts = [bank(), bank(), bank()];
    let provider = compiler(address, &accounts[0]);
    let app = Rc::new(App::default());
    let first_input = input_with_listener(
        &owner,
        key(1, Generation::INITIAL),
        &td(address, "alpha", &["/alpha"]),
        provider.clone(),
        "alpha",
        11,
        app.clone(),
        Some(&native),
    );
    let shared = definition(&first_input.prepared);
    let mut first = admit(&owner, first_input, &accounts[0]);
    start(&mut first);
    // Stage the losing logical contender while the slot is still vacant.
    let failed_provider = reuse(address, 412, &shared);
    let failed_app = Rc::new(App::default());
    let failed_input = input_with_listener(
        &owner,
        key(1, Generation::INITIAL),
        &td(address, "failed", &["/free"]),
        failed_provider.clone(),
        "failed",
        33,
        failed_app.clone(),
        Some(&native),
    );
    let mut failed = admit(&owner, failed_input, &accounts[2]);
    first.publish(&mut owner, &mut budget()).unwrap();

    // A shared native listener supplies no shortcut around whole final-TD
    // qualification. A retained unsupported Form invalidates this candidate.
    let invalid_provider = reuse(address, 410, &shared);
    let mut invalid = input_with_listener(
        &owner,
        key(2, Generation::INITIAL),
        &td(address, "invalid", &["/invalid"]),
        invalid_provider,
        "invalid",
        44,
        Rc::new(App::default()),
        Some(&native),
    );
    let mut changed = td(address, "invalid", &["/invalid"]);
    changed["properties"]["invalid"]["forms"]
        .as_array_mut()
        .unwrap()
        .push(
            serde_json::json!({"href":format!("http://{address}/observe"),"op":"observeproperty"}),
        );
    invalid.instance = invalid
        .instance
        .with_description(Arc::from(changed.to_string()));
    assert!(
        ComposedProducerExposure::try_new(
            &owner,
            invalid,
            limits(),
            accounts[1].clone(),
            &mut budget()
        )
        .is_err()
    );
    assert_empty(&accounts[1]);

    let colliding_provider = reuse(address, 411, &shared);
    let colliding_input = input_with_listener(
        &owner,
        key(2, Generation::INITIAL),
        &td(address, "collision", &["/free", "/alpha"]),
        colliding_provider.clone(),
        "collision",
        22,
        Rc::new(App::default()),
        Some(&native),
    );
    let mut colliding = admit(&owner, colliding_input, &accounts[1]);
    assert_eq!(
        colliding
            .participant_mut::<HttpRuntime>(0)
            .unwrap()
            .step(&mut cx(), &mut budget(), now()),
        Poll::Ready(RawFiniteState::Closing)
    );
    assert!(
        colliding
            .participant_mut::<HttpRuntime>(0)
            .unwrap()
            .first_cause()
            .is_some()
    );
    close(&mut colliding);
    colliding.retire(&mut owner, &mut budget()).unwrap();
    drop(colliding);
    drop(colliding_provider);
    assert_empty(&accounts[1]);

    // The failed multi-route acquisition did not reserve its nonconflicting
    // prefix. This candidate starts /free and then loses logical publication.
    start(&mut failed);
    let prepublication = connect(address, "/free");
    for _ in 0..20 {
        drive(&mut owner, &mut [&mut first]);
    }
    assert!(failed.publish(&mut owner, &mut budget()).is_err());
    assert!(failed.publication_failure().is_some());
    close(&mut failed);
    failed.retire(&mut owner, &mut budget()).unwrap();
    drop(failed);
    drop(failed_provider);
    assert_empty(&accounts[2]);
    assert!(response(prepublication, &mut owner, &mut [&mut first]).is_empty());
    assert_eq!(failed_app.calls.get(), 0);
    assert_value(&request(address, "/alpha", &mut owner, &mut first), 11);
    acknowledge(&mut first);
    close(&mut first);
    first.retire(&mut owner, &mut budget()).unwrap();
    drop(first);
    drop(provider);
    drop(shared);
    assert_empty(&accounts[0]);
    TcpListener::bind(address).unwrap();
}

#[test]
fn incomplete_original_input_cannot_retarget_a_successor_or_block_sibling_progress() {
    let mut owner = owner();
    let address = tcp_addr();
    let native = listener(address);
    let accounts = [bank(), bank(), bank()];
    let provider = compiler(address, &accounts[0]);
    let original_app = Rc::new(App::default());
    let original_input = input_with_listener(
        &owner,
        key(1, Generation::INITIAL),
        &td(address, "alpha", &["/alpha"]),
        provider.clone(),
        "alpha",
        11,
        original_app.clone(),
        Some(&native),
    );
    let shared = definition(&original_input.prepared);
    let sibling_provider = reuse(address, 422, &shared);
    let sibling_input = input_with_listener(
        &owner,
        key(2, Generation::INITIAL),
        &td(address, "beta", &["/beta"]),
        sibling_provider.clone(),
        "beta",
        22,
        Rc::new(App::default()),
        Some(&native),
    );
    let mut first = admit(&owner, original_input, &accounts[0]);
    let mut second = admit(&owner, sibling_input, &accounts[1]);
    start(&mut first);
    start(&mut second);
    first.publish(&mut owner, &mut budget()).unwrap();
    second.publish(&mut owner, &mut budget()).unwrap();
    let mut old_peer = TcpStream::connect(address).unwrap();
    old_peer.write_all(b"GET /alpha HTTP/1.1\r\n").unwrap();
    old_peer.set_nonblocking(true).unwrap();
    for _ in 0..20 {
        drive(&mut owner, &mut [&mut second]);
    }
    assert_value(&request(address, "/beta", &mut owner, &mut second), 22);
    acknowledge(&mut second);
    let old = first.key();
    let successor_app = Rc::new(App::default());
    let successor_provider = reuse(address, 423, &shared);
    let mut successor_input = input_with_listener(
        &owner,
        key(1, Generation::INITIAL.checked_next().unwrap()),
        &td(address, "successor", &["/alpha"]),
        successor_provider.clone(),
        "successor",
        33,
        successor_app.clone(),
        Some(&native),
    );
    successor_input.expected = Some(old);
    let mut successor = admit(&owner, successor_input, &accounts[2]);
    close(&mut first);
    start(&mut successor);
    successor.publish(&mut owner, &mut budget()).unwrap();
    old_peer
        .write_all(format!("Host: {address}\r\nConnection: close\r\n\r\n").as_bytes())
        .unwrap();
    assert!(response(old_peer, &mut owner, &mut [&mut second, &mut successor]).is_empty());
    assert_eq!(original_app.calls.get(), 0);
    assert_eq!(successor_app.calls.get(), 0);
    assert_value(
        &response(
            connect(address, "/alpha"),
            &mut owner,
            &mut [&mut second, &mut successor],
        ),
        33,
    );
    acknowledge(&mut successor);
    first.retire(&mut owner, &mut budget()).unwrap();
    for exposure in [&mut second, &mut successor] {
        close(exposure);
        exposure.retire(&mut owner, &mut budget()).unwrap();
    }
    drop(first);
    drop(second);
    drop(successor);
    drop(provider);
    drop(sibling_provider);
    drop(successor_provider);
    drop(shared);
    for bank in &accounts {
        assert_empty(bank);
    }
    TcpListener::bind(address).unwrap();
}

#[test]
fn replacing_one_publication_retains_its_accepted_reply_and_original_bank() {
    let mut owner = owner();
    let address = tcp_addr();
    let native = listener(address);
    let original_bank = bank();
    let replacement_bank = bank();
    let provider = compiler(address, &original_bank);
    let original_app = Rc::new(App::default());
    let original_input = input_with_listener(
        &owner,
        key(1, Generation::INITIAL),
        &td(address, "alpha", &["/old"]),
        provider.clone(),
        "alpha",
        11,
        original_app.clone(),
        Some(&native),
    );
    let shared = definition(&original_input.prepared);
    let mut first = admit(&owner, original_input, &original_bank);
    start(&mut first);
    first.publish(&mut owner, &mut budget()).unwrap();
    let peer = connect(address, "/old");
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        drive(&mut owner, &mut [&mut first]);
        if first.participant_mut::<HttpRuntime>(0).unwrap().state() == RawFiniteState::Delivering {
            break;
        }
        assert!(Instant::now() < end);
    }
    assert_eq!(original_app.calls.get(), 1);
    let old = first.key();
    let replacement_app = Rc::new(App::default());
    let replacement_provider = reuse(address, 431, &shared);
    let mut replacement_input = input_with_listener(
        &owner,
        key(1, Generation::INITIAL.checked_next().unwrap()),
        &td(address, "successor", &["/new"]),
        replacement_provider.clone(),
        "successor",
        33,
        replacement_app.clone(),
        Some(&native),
    );
    replacement_input.expected = Some(old);
    let mut replacement = admit(&owner, replacement_input, &replacement_bank);
    start(&mut replacement);
    replacement.publish(&mut owner, &mut budget()).unwrap();
    // Old accepted work remains owned; its connection is aborted by explicit
    // local close, rather than transferred to the replacement or replayed.
    close(&mut first);
    assert!(first.retire(&mut owner, &mut budget()).is_err());
    assert_value(&request(address, "/new", &mut owner, &mut replacement), 33);
    acknowledge(&mut replacement);
    let retained = acknowledge(&mut first);
    assert_eq!(retained.recipient, Some(old));
    assert_eq!(retained.effect, FiniteEffect::Unknown);
    assert_eq!(retained.delivery, FiniteDelivery::NotDelivered);
    assert!(
        retained
            .result
            .unwrap()
            .data()
            .unwrap()
            .body
            .ends_with(b"11}")
    );
    assert!(
        first
            .participant_mut::<HttpRuntime>(0)
            .unwrap()
            .first_cause()
            .is_some()
    );
    first.retire(&mut owner, &mut budget()).unwrap();
    drop(first);
    drop(provider);
    drop(peer);
    assert_eq!(live(&original_bank), shared.retained_bytes() as u64);
    assert_eq!(original_app.calls.get(), 1);
    assert_eq!(replacement_app.calls.get(), 1);
    close(&mut replacement);
    replacement.retire(&mut owner, &mut budget()).unwrap();
    drop(replacement);
    drop(replacement_provider);
    drop(shared);
    assert_empty(&original_bank);
    assert_empty(&replacement_bank);
    TcpListener::bind(address).unwrap();
}

#[test]
fn bounded_unclassified_pressure_and_staged_input_expire_without_revoking_siblings() {
    let mut owner = owner();
    let address = tcp_addr();
    let native = listener(address);
    let accounts = [bank(), bank()];
    let provider = compiler(address, &accounts[0]);
    let first_input = input_with_listener(
        &owner,
        key(1, Generation::INITIAL),
        &td(address, "alpha", &["/alpha"]),
        provider.clone(),
        "alpha",
        11,
        Rc::new(App::default()),
        Some(&native),
    );
    let shared = definition(&first_input.prepared);
    let second_provider = reuse(address, 442, &shared);
    let second_app = Rc::new(App::default());
    let second_input = input_with_listener(
        &owner,
        key(2, Generation::INITIAL),
        &td(address, "beta", &["/beta"]),
        second_provider.clone(),
        "beta",
        22,
        second_app.clone(),
        Some(&native),
    );
    let mut first = admit(&owner, first_input, &accounts[0]);
    let mut second = admit(&owner, second_input, &accounts[1]);
    start(&mut first);
    start(&mut second);
    first.publish(&mut owner, &mut budget()).unwrap();
    let staged = connect(address, "/beta");
    for _ in 0..20 {
        drive(&mut owner, &mut [&mut first]);
    }
    let mut slow = Vec::new();
    for _ in 0..32 {
        let mut peer = TcpStream::connect(address).unwrap();
        peer.write_all(b"GET /alpha HTTP/1.1\r\n").unwrap();
        slow.push(peer);
        drive(&mut owner, &mut [&mut first]);
    }
    // One more connection waits in the bounded OS backlog while all native
    // head slots are occupied. Expiry frees slots without an application call.
    let waiting = connect(address, "/alpha");
    let later = MonotonicInstant::new(ClockId::new(1), 100);
    assert_value(
        &response_at(waiting, &mut owner, &mut [&mut first], later),
        11,
    );
    acknowledge(&mut first);
    assert_eq!(second_app.calls.get(), 0);
    second.publish(&mut owner, &mut budget()).unwrap();
    assert!(response_at(staged, &mut owner, &mut [&mut first, &mut second], later).is_empty());
    assert_eq!(second_app.calls.get(), 0);
    assert_value(
        &response_at(
            connect(address, "/beta"),
            &mut owner,
            &mut [&mut first, &mut second],
            later,
        ),
        22,
    );
    acknowledge(&mut second);
    for exposure in [&mut first, &mut second] {
        exposure.begin_close();
        assert_eq!(
            exposure.participant_mut::<HttpRuntime>(0).unwrap().step(
                &mut cx(),
                &mut budget(),
                later
            ),
            Poll::Ready(RawFiniteState::Closed)
        );
        exposure.retire(&mut owner, &mut budget()).unwrap();
    }
    drop(first);
    drop(second);
    drop(provider);
    drop(second_provider);
    drop(shared);
    drop(slow);
    for bank in &accounts {
        assert_empty(bank);
    }
    TcpListener::bind(address).unwrap();
}

#[test]
fn incompatible_native_configuration_and_known_listener_failure_cannot_qualify_publication() {
    let mut owner = owner();
    let address = tcp_addr();
    let native = listener(address);
    let accounts = bank();
    let provider = compiler(address, &accounts);
    let app = Rc::new(App::default());
    let candidate_input = input_with_listener(
        &owner,
        key(1, Generation::INITIAL),
        &td(address, "alpha", &["/alpha"]),
        provider.clone(),
        "alpha",
        11,
        app.clone(),
        Some(&native),
    );
    let shared = definition(&candidate_input.prepared);
    let other_provider = reuse(address, 452, &shared);
    let other_key = key(2, Generation::INITIAL).key();
    let other = prepare(
        &td(address, "other", &["/other"]),
        other_key,
        &other_provider,
    )
    .unwrap();
    let config = HttpFiniteConfig::try_new(
        HttpProducerConfig::for_loopback(address).unwrap(),
        other.operations()[0]
            .definition::<HttpProducerProgram>()
            .unwrap()
            .clone(),
        other_key,
        AffordanceTarget::Property(Arc::from("other")),
        100,
    )
    .unwrap();
    assert!(config.clone().with_listener(&listener(tcp_addr())).is_err());
    assert!(
        config
            .clone()
            .with_listener(
                &HttpFiniteListener::new(HttpProducerConfig::for_loopback(address).unwrap(), 99)
                    .unwrap()
            )
            .is_err()
    );
    assert!(
        HttpFiniteListener::new(
            HttpProducerConfig::for_https_loopback(address).unwrap(),
            100
        )
        .is_err()
    );
    // Compatibility checks and construction have not bound anything.
    TcpListener::bind(address).unwrap();
    let config = config.with_listener(&native).unwrap();
    let mut participant =
        HttpFiniteBinding::new(config, BindingId::new(452), BindingGeneration::INITIAL);
    let mut candidate = admit(&owner, candidate_input, &accounts);
    start(&mut candidate);
    assert!(matches!(
        participant.poll_start(&mut cx()),
        Poll::Ready(Ok(()))
    ));
    let early = connect(address, "/alpha");
    assert!(participant.poll_receive(&mut cx(), now()).is_pending());
    // A shared physical failure is distinct from logical close. Deliberately
    // incompatible clock progress terminates this native owner's availability.
    assert!(matches!(
        participant.poll_receive(&mut cx(), MonotonicInstant::new(ClockId::new(2), 0)),
        Poll::Ready(Err(_))
    ));
    assert!(candidate.publish(&mut owner, &mut budget()).is_err());
    assert!(candidate.publication_failure().is_some());
    assert_eq!(owner.description(candidate.key()), None);
    assert_eq!(app.calls.get(), 0);
    close(&mut candidate);
    candidate.retire(&mut owner, &mut budget()).unwrap();
    assert!(matches!(
        participant.poll_close(&mut cx(), None, None, None),
        Poll::Ready(Ok(_))
    ));
    drop(candidate);
    drop(participant);
    drop(early);
    drop(other);
    drop(other_provider);
    drop(provider);
    drop(shared);
    assert_empty(&accounts);
    TcpListener::bind(address).unwrap();
}

fn check_shared_listener_loss(phase: RawFiniteState, left_budget: fn() -> WorkBudget) {
    let mut owner = owner();
    let accounts = bank();
    let addresses = [tcp_addr(), tcp_addr()];
    let shared = listener(addresses[0]);
    let counts = Rc::new(Counts {
        long_string: phase == RawFiniteState::Delivering,
        ..Counts::default()
    });
    let expected = counts.property_body();
    let mut input = http_input(
        &owner,
        key(1, Generation::INITIAL),
        addresses,
        counts.clone(),
    );
    let left = &input.prepared.operations()[0];
    let config = HttpFiniteConfig::try_new(
        HttpProducerConfig::for_loopback(addresses[0]).unwrap(),
        left.definition::<HttpProducerProgram>().unwrap().clone(),
        input.instance.key(),
        AffordanceTarget::Property(Arc::from("left")),
        100,
    )
    .unwrap()
    .with_listener(&shared)
    .unwrap();
    input.participants[0] = OwnedProducerParticipation::finite(http_runtime(
        &owner,
        config,
        left.binding.0,
        FiniteValidation::Required,
    ));
    let mut thing = admit(&owner, input, &accounts);
    start_http(&mut thing);
    thing.publish(&mut owner, &mut budget()).unwrap();
    let mut first = connect(addresses[0], "/value");
    let member = thing.participant_mut::<HttpRuntime>(0).unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    let expiring = phase == RawFiniteState::RetiringReply;
    let before_expiry = if expiring {
        RawFiniteState::Delivering
    } else {
        phase
    };
    while owner.step(member, &mut cx(), &mut budget(), now()).unwrap() != Poll::Ready(before_expiry)
    {
        assert!(
            Instant::now() < end,
            "first request stalled at {:?}",
            member.state()
        );
    }
    let instant = if expiring {
        let deadline = MonotonicInstant::new(ClockId::new(1), 100);
        assert!(
            member
                .step(&mut cx(), &mut WorkBudget::new(), deadline)
                .is_pending()
        );
        assert_eq!(member.state(), RawFiniteState::RetiringReply);
        deadline
    } else {
        now()
    };
    let assert_delivered = |peer: &mut TcpStream| {
        let mut wire = Vec::new();
        peer.read_to_end(&mut wire).unwrap();
        assert_http_body(&wire, &expected);
    };
    if phase == RawFiniteState::Serving {
        assert_delivered(&mut first);
    }
    let accepted = phase != RawFiniteState::Dispatching;

    // A second route on the same physical listener brings an incompatible clock.
    let other_input = input_with_listener(
        &owner,
        key(2, Generation::INITIAL),
        &td(addresses[0], "other", &["/other"]),
        compiler(addresses[0], &accounts),
        "other",
        7,
        Rc::new(App::default()),
        Some(&shared),
    );
    let mut other = admit(&owner, other_input, &accounts);
    let other_clock = MonotonicInstant::new(ClockId::new(2), 0);
    other
        .step(&mut owner, &mut cx(), other_clock, |_, _| budget())
        .unwrap();
    other.publish(&mut owner, &mut budget()).unwrap();
    other
        .step(&mut owner, &mut cx(), other_clock, |_, _| budget())
        .unwrap();
    assert!(matches!(
        other.progress_failure(0),
        Some(CoreError::Binding(_))
    ));

    thing
        .step(&mut owner, &mut cx(), instant, |_, _| WorkBudget::new())
        .unwrap();
    assert!(owner.description(thing.key()).is_some());
    let mut sibling = connect(addresses[1], "/value");
    for _ in 0..8 {
        thing
            .step(&mut owner, &mut cx(), instant, |index, _| {
                if index == 0 { left_budget() } else { budget() }
            })
            .unwrap();
    }
    let mut wire = Vec::new();
    if let Err(error) = sibling.read_to_end(&mut wire) {
        assert_eq!(error.kind(), std::io::ErrorKind::ConnectionReset);
    }
    assert_eq!(
        (
            owner.description(thing.key()).is_some(),
            counts.property.get(),
            wire.starts_with(b"HTTP/1.1 200")
        ),
        (false, usize::from(accepted), false),
    );
    assert!(matches!(
        thing.progress_failure(0),
        Some(CoreError::Binding(_))
    ));
    assert!(thing.retire(&mut owner, &mut budget()).is_err());
    let original = acknowledge(&mut thing);
    if accepted {
        if phase == RawFiniteState::Delivering {
            assert_delivered(&mut first);
        }
        if expiring {
            assert!(matches!(original.first_cause, Some(CoreError::TimedOut(_))));
            assert_eq!(original.delivery, FiniteDelivery::NotDelivered);
        } else {
            assert!(original.first_cause.is_none());
            assert_eq!(original.delivery, FiniteDelivery::Delivered);
        }
        assert_eq!(
            original.result.unwrap().data().unwrap().body.as_ref(),
            expected.as_slice()
        );
    } else {
        assert_eq!(original.effect, FiniteEffect::NotInvoked);
        assert_eq!(original.delivery, FiniteDelivery::NotDelivered);
        assert!(matches!(original.result, Err(CoreError::Binding(_))));
    }
    thing.retire(&mut owner, &mut budget()).unwrap();
    other
        .step(&mut owner, &mut cx(), other_clock, |_, _| budget())
        .unwrap();
    other.retire(&mut owner, &mut budget()).unwrap();
    drop((thing, other));
    for address in addresses {
        TcpListener::bind(address).unwrap();
    }
    assert_empty(&accounts);
}

#[test]
fn shared_listener_loss_fences_a_sibling_with_an_unacknowledged_result() {
    check_shared_listener_loss(RawFiniteState::Serving, budget);
}

#[test]
fn shared_listener_loss_refuses_a_candidate_and_preserves_an_accepted_reply() {
    check_shared_listener_loss(RawFiniteState::Dispatching, budget);
    check_shared_listener_loss(RawFiniteState::Delivering, budget);
}

#[test]
fn shared_listener_loss_during_reply_retirement_fences_the_publication() {
    check_shared_listener_loss(RawFiniteState::RetiringReply, budget);
}

#[test]
fn shared_listener_loss_fences_dispatch_without_handler_credit() {
    check_shared_listener_loss(RawFiniteState::Dispatching, || {
        budget().with_remaining(WorkClass::HandlerSteps, 0)
    });
}
