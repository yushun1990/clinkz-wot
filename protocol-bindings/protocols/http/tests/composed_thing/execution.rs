//! Real native startup failure and typed-loan substitution at aggregate turns.
use super::*;

#[test]
fn producer_turn_retains_native_start_failure_and_still_visits_the_next_member() {
    let mut owner = owner();
    let accounts = bank();
    let addresses = [tcp_addr(), tcp_addr()];
    let input = http_input(
        &owner,
        key(1, Generation::INITIAL),
        addresses,
        Rc::new(Counts::default()),
    );
    let conflict = TcpListener::bind(addresses[0]).unwrap();
    let mut exposure =
        ComposedProducerExposure::try_new(&owner, input, limits(), accounts.clone(), &mut budget())
            .ok()
            .unwrap();
    exposure
        .step(&mut owner, &mut cx(), now(), |_, _| budget())
        .unwrap();
    let first = exposure
        .progress_failure(0)
        .expect("native failure remains observable without downcasting")
        .clone();
    assert_eq!(
        exposure.participant_mut::<HttpRuntime>(1).unwrap().state(),
        RawFiniteState::InstalledClosed
    );
    assert!(owner.description(exposure.key()).is_none());
    assert!(exposure.publish(&mut owner, &mut budget()).is_err());
    exposure.begin_close();
    for _ in 0..8 {
        exposure
            .step(&mut owner, &mut cx(), now(), |_, _| budget())
            .unwrap();
    }
    assert_eq!(exposure.progress_failure(0), Some(&first));
    exposure.retire(&mut owner, &mut budget()).unwrap();
    drop((exposure, owner, conflict));
    for addr in addresses {
        TcpListener::bind(addr).unwrap();
    }
    assert_empty(&accounts);
}

#[test]
fn substituted_native_member_cannot_progress_but_its_original_sibling_completes() {
    let mut owner = owner();
    let accounts = bank();
    let addresses = [tcp_addr(), tcp_addr()];
    let mut input = http_input(
        &owner,
        key(1, Generation::INITIAL),
        addresses,
        Rc::new(Counts::default()),
    );
    let operation = &input.prepared.operations()[0];
    let program = operation
        .definition::<HttpProducerProgram>()
        .unwrap()
        .clone();
    let binding = operation.binding.0;
    let key = input.instance.key();
    let substitute = http_read(
        &owner,
        addresses[0],
        program.clone(),
        key,
        "left",
        binding,
        FiniteValidation::Required,
    );
    let original = core::mem::replace(
        input.participants[0]
            .as_any_mut()
            .downcast_mut::<HttpRuntime>()
            .unwrap(),
        substitute,
    );
    let rejection =
        ComposedProducerExposure::try_new(&owner, input, limits(), accounts.clone(), &mut budget())
            .err()
            .expect("a substituted stored native must be returned before admission");
    assert_empty(&accounts);
    let mut input = rejection.into_input();
    let substitute = core::mem::replace(
        input.participants[0]
            .as_any_mut()
            .downcast_mut::<HttpRuntime>()
            .unwrap(),
        original,
    );
    drop(substitute);
    let mut exposure =
        ComposedProducerExposure::try_new(&owner, input, limits(), accounts.clone(), &mut budget())
            .ok()
            .unwrap();
    exposure
        .step(&mut owner, &mut cx(), now(), |_, _| budget())
        .unwrap();
    exposure.publish(&mut owner, &mut budget()).unwrap();
    let substitute = http_read(
        &owner,
        addresses[0],
        program,
        key,
        "left",
        binding,
        FiniteValidation::Required,
    );
    let original = core::mem::replace(
        exposure.participant_mut::<HttpRuntime>(0).unwrap(),
        substitute,
    );
    let mut peer = TcpStream::connect(addresses[1]).unwrap();
    peer.write_all(
        format!(
            "GET /value HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            addresses[1]
        )
        .as_bytes(),
    )
    .unwrap();
    peer.set_nonblocking(true).unwrap();
    let end = Instant::now() + Duration::from_secs(3);
    let mut bytes = Vec::new();
    loop {
        assert!(matches!(
            exposure.step(&mut owner, &mut cx(), now(), |_, _| budget()),
            Err(CoreError::StaleHandle(_))
        ));
        let mut buf = [0; 4096];
        match peer.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => bytes.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("{e}"),
        }
        assert!(Instant::now() < end);
    }
    assert!(bytes.starts_with(b"HTTP/1.1 200"));
    assert_eq!(
        exposure.participant_mut::<HttpRuntime>(0).unwrap().state(),
        RawFiniteState::Starting
    );
    assert!(matches!(
        exposure.progress_failure(0),
        Some(CoreError::StaleHandle(_))
    ));
    let settlement = exposure
        .participant_mut::<HttpRuntime>(1)
        .unwrap()
        .take_settlement()
        .unwrap();
    assert_eq!(settlement.recipient, Some(key));
    assert_eq!(settlement.delivery, FiniteDelivery::Delivered);
    // Moving a live native owner preserved the sibling's request. Closing
    // that same owner away from its facade withdraws the whole publication.
    let mut original = original;
    original.begin_close();
    assert!(matches!(
        exposure.step(&mut owner, &mut cx(), now(), |_, _| budget()),
        Err(CoreError::StaleHandle(_))
    ));
    assert!(owner.description(key).is_none());
    let substitute = core::mem::replace(
        exposure.participant_mut::<HttpRuntime>(0).unwrap(),
        original,
    );
    drop(substitute);
    exposure.begin_close();
    for _ in 0..8 {
        exposure
            .step(&mut owner, &mut cx(), now(), |_, _| budget())
            .unwrap();
    }
    exposure.retire(&mut owner, &mut budget()).unwrap();
    drop((settlement, exposure, owner));
    for addr in addresses {
        TcpListener::bind(addr).unwrap();
    }
    assert_empty(&accounts);
}
