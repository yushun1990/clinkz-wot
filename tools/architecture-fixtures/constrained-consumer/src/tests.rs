use super::*;
use clinkz_wot_core::{ThingSlotId, data_schema::ValueSchemas, security::SecurityRequirement};
use clinkz_wot_foundation::{ClockId, Generation, MonotonicInstant, SlotIndex};
use clinkz_wot_servient_finite::borrowed_consumer::{
    ReadDefinition, ReadResultSlot, ReadRuntime, ReadSettlement,
};
use std::{
    cell::RefCell,
    net::{SocketAddr, UdpSocket},
    time::Duration,
};

static VALUES: ValueSchemas = ValueSchemas {
    input: None,
    output: Some(PreparedSchema::Integer {
        minimum: Some(1),
        exclusive_minimum: None,
        maximum: Some(50),
        exclusive_maximum: None,
    }),
};
static PROGRAM: Program<'static> = Program {
    target: "test-datagram:temperature",
    plan: PlanId::new(SlotIndex::new(9), Generation::INITIAL),
};
fn now(t: u64) -> MonotonicInstant {
    MonotonicInstant::new(ClockId::new(1), t)
}
fn authority() -> LocalConsumerInstance {
    LocalConsumerInstance::new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL))
}
fn meaning() -> ValueExecution<'static> {
    ValueExecution::try_new(
        clinkz_wot_core::AffordanceSite::Property("temperature"),
        Operation::ReadProperty,
        &VALUES,
        &SecurityRequirement::NOSEC,
    )
    .unwrap()
}
struct Peer {
    socket: UdpSocket,
    client: RefCell<Option<SocketAddr>>,
}
impl Peer {
    fn new() -> Self {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        Self {
            socket,
            client: RefCell::new(None),
        }
    }
    fn open(&self) -> CoreResult<NativeSocket> {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket.connect(self.socket.local_addr().unwrap()).unwrap();
        socket.set_nonblocking(true).unwrap();
        *self.client.borrow_mut() = Some(socket.local_addr().unwrap());
        Ok(NativeSocket(socket))
    }
    fn respond(&self, flags: u8, body: &[u8]) {
        let mut request = [0; 64];
        let (n, client) = self.socket.recv_from(&mut request).unwrap();
        assert_eq!(&request[..n], b"read-value");
        let mut wire = [0; 65];
        wire[0] = flags;
        wire[1..body.len() + 1].copy_from_slice(body);
        self.socket
            .send_to(&wire[..body.len() + 1], client)
            .unwrap();
    }
    fn assert_open(&self) {
        assert!(UdpSocket::bind(self.client.borrow().unwrap()).is_err());
    }
    fn assert_released(&self) {
        assert!(UdpSocket::bind(self.client.borrow().unwrap()).is_ok());
    }
}
struct NativeSocket(UdpSocket);
impl TestDatagrams for NativeSocket {
    fn send(&mut self, bytes: &[u8]) -> CoreResult<()> {
        self.0.send(bytes).map(|_| ()).map_err(|_| failed())
    }
    fn receive(&mut self, buffer: &mut [u8]) -> CoreResult<Option<usize>> {
        match self.0.recv(buffer) {
            Ok(n) => Ok(Some(n)),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            _ => Err(failed()),
        }
    }
}
fn definition<'a>(
    b: &'a TestBinding<'a, NativeSocket>,
    c: &'a NumberCodec<'a>,
) -> ReadDefinition<'a, TestBinding<'a, NativeSocket>> {
    ReadDefinition::try_new(meaning(), b, &PROGRAM, 7, c).unwrap()
}
fn completed<'a, B: ReadBinding>(
    p: Poll<CoreResult<ReadSettlement<'a, B>>>,
) -> ReadSettlement<'a, B> {
    match p {
        Poll::Ready(Ok(s)) => s,
        Poll::Ready(Err(e)) => panic!("{e:?}"),
        _ => panic!("pending"),
    }
}

#[test]
fn native_read_bounded_turn_unread_result_and_exact_final_release() {
    let peer = Peer::new();
    let open = |_: &str| peer.open();
    let stats = Statistics::default();
    let b = TestBinding {
        open: &open,
        stats: &stats,
        configuration: 7,
    };
    let codec = NumberCodec::ready();
    let d = definition(&b, &codec);
    let instance = authority();
    let access = Permission::default();
    let driving = RefCell::new(());
    let result = ReadResultSlot::new();
    let mut runtime = ReadRuntime::bind(&driving, instance.reference(), &d, &access).unwrap();
    let handle = runtime.read(&result, Deadline::NONE, now(0)).unwrap();
    let mut cx = Context::from_waker(core::task::Waker::noop());
    // A nested drive names the same manual loop and cannot enter a native callback.
    let outer = driving.borrow_mut();
    assert!(matches!(
        runtime.poll(handle, &mut cx, now(9)),
        Poll::Ready(Err(CoreError::Lifecycle(_)))
    ));
    assert_eq!(stats.polls.get(), 0);
    drop(outer);
    assert!(runtime.poll(handle, &mut cx, now(1)).is_pending());
    peer.respond(0, b"42e0");
    let mut s = completed(runtime.poll(handle, &mut cx, now(3)));
    assert_eq!(codec.validations.get(), 0);
    assert_eq!(s.result().unwrap().data().unwrap().body, b"42e0");
    assert_eq!(
        s.result()
            .unwrap()
            .metadata()
            .binding_response()
            .unwrap()
            .status_code(),
        None,
        "a native datagram response needs no numeric status surrogate"
    );
    assert!(s.consumer() == instance.reference());
    assert_eq!(s.mapping().plan, PROGRAM.plan);
    assert!(runtime.is_idle());
    assert!(matches!(
        runtime.cancel(handle),
        Err(CoreError::StaleHandle(_))
    ));
    drop(runtime);
    peer.assert_released();
    assert_eq!(stats.outputs_dropped.get(), 0);
    drop(s);
    peer.assert_released();
    assert_eq!(stats.calls_dropped.get(), 1);
    assert_eq!(stats.outputs_dropped.get(), 1);
}

#[test]
fn validation_lends_the_checked_view_and_refuses_a_sibling_native_drive() {
    let driving = RefCell::new(());
    let authority = authority();
    let access = Permission::default();
    let peer = Peer::new();
    let sibling_peer = Peer::new();
    let open = |_: &str| peer.open();
    let sibling_open = |_: &str| sibling_peer.open();
    let stats = Statistics::default();
    let sibling_stats = Statistics::default();
    let binding = TestBinding {
        open: &open,
        stats: &stats,
        configuration: 7,
    };
    let sibling_binding = TestBinding {
        open: &sibling_open,
        stats: &sibling_stats,
        configuration: 7,
    };
    let sibling_codec = NumberCodec::ready();
    let sibling_definition = definition(&sibling_binding, &sibling_codec);
    let sibling_slot = ReadResultSlot::new();
    let sibling = RefCell::new(
        ReadRuntime::bind(
            &driving,
            authority.reference(),
            &sibling_definition,
            &access,
        )
        .unwrap(),
    );
    let sibling_handle = sibling
        .borrow_mut()
        .read(&sibling_slot, Deadline::NONE, now(0))
        .unwrap();
    let nested = || {
        let mut cx = Context::from_waker(core::task::Waker::noop());
        assert!(matches!(
            sibling.borrow_mut().step(&mut cx, now(9)),
            Poll::Ready(Err(CoreError::Lifecycle(_)))
        ));
        assert_eq!(sibling_stats.polls.get(), 0);
    };
    let codec = NumberCodec {
        callback: Some(&nested),
        ..NumberCodec::ready()
    };
    let definition = definition(&binding, &codec);
    let slot = ReadResultSlot::default();
    let mut outer =
        ReadRuntime::bind(&driving, authority.reference(), &definition, &access).unwrap();
    let handle = outer.read(&slot, Deadline::NONE, now(0)).unwrap();
    let mut cx = Context::from_waker(core::task::Waker::noop());
    assert!(outer.step(&mut cx, now(1)).is_pending());
    peer.respond(7, b"42");
    let mut settlement = completed(outer.poll(handle, &mut cx, now(2)));
    assert_eq!(settlement.result().unwrap().data().unwrap().body, b"42");
    assert_eq!(
        settlement
            .native_output()
            .unwrap()
            .unwrap()
            .data()
            .unwrap()
            .body,
        b"999"
    );
    peer.assert_released();
    assert!(sibling.borrow_mut().step(&mut cx, now(3)).is_pending());
    sibling_peer.respond(0, b"9");
    let mut result = completed(sibling.borrow_mut().poll(sibling_handle, &mut cx, now(4)));
    assert_eq!(result.result().unwrap().data().unwrap().body, b"9");
    sibling_peer.assert_released();
}

#[test]
fn cancellation_keeps_failed_cleanup_and_raw_late_output_without_replay() {
    let peer = Peer::new();
    let open = |_: &str| peer.open();
    let stats = Statistics::default();
    stats.late.set(true);
    stats.cleanup_error.set(true);
    let b = TestBinding {
        open: &open,
        stats: &stats,
        configuration: 7,
    };
    let codec = NumberCodec::ready();
    let d = definition(&b, &codec);
    let instance = authority();
    let access = Permission::default();
    let driving = RefCell::new(());
    let result = ReadResultSlot::new();
    let mut runtime = ReadRuntime::bind(&driving, instance.reference(), &d, &access).unwrap();
    let h = runtime.read(&result, Deadline::NONE, now(0)).unwrap();
    let mut cx = Context::from_waker(core::task::Waker::noop());
    assert!(runtime.poll(h, &mut cx, now(1)).is_pending());
    runtime.cancel(h).unwrap();
    instance.revoke();
    codec.available.set(false);
    access.denied.set(true);
    let checks = access.checks.get();
    assert_eq!(stats.cancel_starts.get(), 0);
    assert!(runtime.poll(h, &mut cx, now(1)).is_pending());
    assert!(runtime.poll(h, &mut cx, now(1)).is_pending());
    peer.assert_open();
    peer.respond(0, b"42");
    let mut s = completed(runtime.poll(h, &mut cx, now(2)));
    assert!(matches!(s.result(), Err(CoreError::Cancelled(_))));
    assert!(matches!(s.cleanup_error(), Some(CoreError::Binding(_))));
    assert_eq!(
        s.native_output().unwrap().unwrap().data().unwrap().body,
        b"42"
    );
    assert_eq!(codec.validations.get(), 0);
    assert_eq!(access.checks.get(), checks);
    assert_eq!(stats.starts.get(), 1);
    assert_eq!(stats.cancel_starts.get(), 1);
    assert_eq!(stats.cancel_polls.get(), 2);
    drop(runtime);
    drop(s);
    peer.assert_released();
    assert_eq!(stats.outputs_dropped.get(), 1);
}

#[test]
fn every_untrusted_shape_media_bound_and_schema_failure_retains_original_output() {
    for (flags, body, calls) in [
        (1, &b"42"[..], 0),
        (2, &b"42"[..], 0),
        (3, &b"42"[..], 0),
        (4, &b"42"[..], 0),
        (5, &b"42"[..], 0),
        (6, &b"42"[..], 0),
        (0, &b"12345678901234567890123456789012345"[..], 0),
        (0, &b"51"[..], 1),
        (0, &b"1.5"[..], 1),
        (0, &b"garbage"[..], 1),
    ] {
        let peer = Peer::new();
        let open = |_: &str| peer.open();
        let stats = Statistics::default();
        let b = TestBinding {
            open: &open,
            stats: &stats,
            configuration: 7,
        };
        let codec = NumberCodec::ready();
        let d = definition(&b, &codec);
        let instance = authority();
        let access = Permission::default();
        let driving = RefCell::new(());
        let result = ReadResultSlot::new();
        let mut runtime = ReadRuntime::bind(&driving, instance.reference(), &d, &access).unwrap();
        let h = runtime.read(&result, Deadline::NONE, now(0)).unwrap();
        let mut cx = Context::from_waker(core::task::Waker::noop());
        assert!(runtime.poll(h, &mut cx, now(1)).is_pending());
        peer.respond(flags, body);
        let mut s = completed(runtime.poll(h, &mut cx, now(2)));
        assert!(s.result().is_err());
        let original = s.native_output().unwrap().unwrap();
        assert_eq!(
            original.data().map(|p| p.body),
            (flags != 4).then_some(body)
        );
        assert_eq!(codec.validations.get(), calls);
        peer.assert_released();
        drop(runtime);
        drop(s);
        peer.assert_released();
        assert_eq!(stats.outputs_dropped.get(), 1);
    }
}

#[test]
fn same_slot_foreign_authority_and_reused_call_tokens_cannot_redirect_work() {
    let peer = Peer::new();
    let open = |_: &str| peer.open();
    let stats = Statistics::default();
    let b = TestBinding {
        open: &open,
        stats: &stats,
        configuration: 7,
    };
    let codec = NumberCodec::ready();
    let d = definition(&b, &codec);
    let old = authority();
    let replacement = authority();
    let access = Permission::default();
    let driving = RefCell::new(());
    let original = ReadResultSlot::new();
    let replacement_result = ReadResultSlot::new();
    let next_result = ReadResultSlot::new();
    let mut a = ReadRuntime::bind(&driving, old.reference(), &d, &access).unwrap();
    let mut z = ReadRuntime::bind(&driving, replacement.reference(), &d, &access).unwrap();
    let h = a.read(&original, Deadline::NONE, now(0)).unwrap();
    let k = z.read(&replacement_result, Deadline::NONE, now(0)).unwrap();
    assert!(matches!(z.cancel(h), Err(CoreError::StaleHandle(_))));
    let mut cx = Context::from_waker(core::task::Waker::noop());
    old.revoke();
    assert!(a.poll(h, &mut cx, now(1)).is_pending());
    let mut cancelled = completed(a.poll(h, &mut cx, now(1)));
    assert!(matches!(cancelled.result(), Err(CoreError::StaleHandle(_))));
    assert!(z.poll(k, &mut cx, now(1)).is_pending());
    peer.respond(0, b"42");
    let mut s = completed(z.poll(k, &mut cx, now(1)));
    assert!(s.result().is_ok());
    let next = z.read(&next_result, Deadline::NONE, now(2)).unwrap();
    let polls = stats.polls.get();
    {
        let _turn = driving.borrow_mut();
        for handle in [next, k] {
            assert!(
                matches!(z.poll(handle, &mut cx, now(9)), Poll::Ready(Err(CoreError::Lifecycle(e))) if e.phase() == ErrorPhase::Readiness)
            );
        }
    }
    assert_eq!(stats.polls.get(), polls);
    assert!(matches!(z.cancel(k), Err(CoreError::StaleHandle(_))));
    assert!(matches!(
        z.poll(k, &mut cx, now(2)),
        Poll::Ready(Err(CoreError::StaleHandle(_)))
    ));
    assert!(z.poll(next, &mut cx, now(2)).is_pending());
    peer.respond(0, b"9");
    let mut result = completed(z.poll(next, &mut cx, now(3)));
    assert_eq!(result.result().unwrap().data().unwrap().body, b"9");
    drop(a);
    drop(z);
    drop(cancelled);
    drop(s);
    assert_eq!(stats.starts.get(), 3);
    assert_eq!(stats.calls_dropped.get(), 3);
}

#[test]
fn unread_success_rechecks_live_permission_without_redecoding() {
    let peer = Peer::new();
    let open = |_: &str| peer.open();
    let stats = Statistics::default();
    let b = TestBinding {
        open: &open,
        stats: &stats,
        configuration: 7,
    };
    let codec = NumberCodec::ready();
    let d = definition(&b, &codec);
    let instance = authority();
    let access = Permission::default();
    let driving = RefCell::new(());
    let result = ReadResultSlot::new();
    let mut runtime = ReadRuntime::bind(&driving, instance.reference(), &d, &access).unwrap();
    let h = runtime.read(&result, Deadline::NONE, now(0)).unwrap();
    let mut cx = Context::from_waker(core::task::Waker::noop());
    assert!(runtime.step(&mut cx, now(1)).is_pending());
    peer.respond(0, b"42");
    assert!(matches!(runtime.step(&mut cx, now(2)), Poll::Ready(Ok(()))));
    access.denied.set(true);
    let mut s = completed(runtime.poll(h, &mut cx, now(2)));
    assert!(matches!(s.result(), Err(CoreError::Security { .. })));
    assert_eq!(
        s.native_output().unwrap().unwrap().data().unwrap().body,
        b"42"
    );
    assert_eq!(codec.validations.get(), 0);
    drop(runtime);
    drop(s);
}

#[test]
fn callback_revocation_and_policy_rotation_fence_before_io_and_after_validation() {
    let peer = Peer::new();
    let open = |_: &str| peer.open();
    let stats = Statistics::default();
    let b = TestBinding {
        open: &open,
        stats: &stats,
        configuration: 7,
    };
    let instance = authority();
    let driving = RefCell::new(());
    let invalidate = || instance.revoke();
    let codec = NumberCodec {
        callback: Some(&invalidate),
        ..NumberCodec::ready()
    };
    let d = definition(&b, &codec);
    let access = Permission::default();
    let result = ReadResultSlot::new();
    // Changing an actual service during permission cannot qualify itself.
    access.rotate_on_check.set(true);
    assert!(ReadRuntime::bind(&driving, instance.reference(), &d, &access).is_err());
    assert_eq!(stats.starts.get(), 0);
    let mut runtime = ReadRuntime::bind(&driving, instance.reference(), &d, &access).unwrap();
    access.rotate_on_check.set(true);
    let error = runtime.read(&result, Deadline::NONE, now(0)).err().unwrap();
    assert!(matches!(error, CoreError::Validation(_)));
    drop(runtime);
    assert_eq!(stats.starts.get(), 0);
    let mut runtime = ReadRuntime::bind(&driving, instance.reference(), &d, &access).unwrap();
    let h = runtime.read(&result, Deadline::NONE, now(0)).unwrap();
    let mut cx = Context::from_waker(core::task::Waker::noop());
    assert!(runtime.poll(h, &mut cx, now(1)).is_pending());
    peer.respond(0, b"42");
    let mut s = completed(runtime.poll(h, &mut cx, now(2)));
    assert!(matches!(s.result(), Err(CoreError::StaleHandle(_))));
    assert_eq!(
        s.native_output().unwrap().unwrap().data().unwrap().body,
        b"42"
    );
    drop(runtime);
    drop(s);
}

#[test]
fn time_fencing_and_cleanup_acknowledgement_preserve_first_cause() {
    for observation in [now(10), now(0), MonotonicInstant::new(ClockId::new(2), 2)] {
        let peer = Peer::new();
        let open = |_: &str| peer.open();
        let stats = Statistics::default();
        let b = TestBinding {
            open: &open,
            stats: &stats,
            configuration: 7,
        };
        let codec = NumberCodec::ready();
        let d = definition(&b, &codec);
        let instance = authority();
        let access = Permission::default();
        let driving = RefCell::new(());
        let result = ReadResultSlot::new();
        let mut runtime = ReadRuntime::bind(&driving, instance.reference(), &d, &access).unwrap();
        let h = runtime
            .read(&result, Deadline::at(now(10)), now(1))
            .unwrap();
        let mut cx = Context::from_waker(core::task::Waker::noop());
        assert!(runtime.poll(h, &mut cx, now(2)).is_pending());
        assert!(runtime.poll(h, &mut cx, observation).is_pending());
        runtime.cancel(h).unwrap(); // Cannot overwrite time fencing.
        let mut s = completed(runtime.poll(h, &mut cx, now(11)));
        if observation == now(10) {
            assert!(matches!(s.result(), Err(CoreError::TimedOut(_))));
        } else {
            assert!(matches!(s.result(), Err(CoreError::InternalInvariant(_))));
        }
        assert_eq!(stats.cleanup_acks.get(), 1);
        peer.assert_released();
        drop(runtime);
        drop(s);
    }
}

#[test]
fn capacity_pairing_required_meaning_and_permission_refuse_without_native_effects() {
    let peer = Peer::new();
    let open = |_: &str| peer.open();
    let stats = Statistics::default();
    let b = TestBinding {
        open: &open,
        stats: &stats,
        configuration: 7,
    };
    let codec = NumberCodec::ready();
    assert!(ReadDefinition::try_new(meaning(), &b, &PROGRAM, 8, &codec).is_err());
    let variables = clinkz_wot_core::uri_variables::PreparedUriVariables::try_new(std::vec![
        clinkz_wot_core::uri_variables::UriVariable {
            name: "zone".into(),
            schema: PreparedSchema::Type(ValueType::String),
            referenced: true,
        },
    ])
    .unwrap();
    let dynamic = meaning().with_uri_variables(&variables);
    assert!(ReadDefinition::try_new(dynamic, &b, &PROGRAM, 7, &codec).is_err());
    assert_eq!(stats.polls.get(), 0);
    let values = ValueSchemas {
        input: None,
        output: Some(PreparedSchema::Type(ValueType::Boolean)),
    };
    let unsupported = ValueExecution::try_new(
        clinkz_wot_core::AffordanceSite::Property("p"),
        Operation::ReadProperty,
        &values,
        &SecurityRequirement::NOSEC,
    )
    .unwrap();
    assert!(ReadDefinition::try_new(unsupported, &b, &PROGRAM, 7, &codec).is_err());
    let secured = SecurityRequirement::scheme("basic", &[]);
    let unsupported = ValueExecution::try_new(
        clinkz_wot_core::AffordanceSite::Property("p"),
        Operation::ReadProperty,
        &VALUES,
        &secured,
    )
    .unwrap();
    assert!(ReadDefinition::try_new(unsupported, &b, &PROGRAM, 7, &codec).is_err());
    let d = definition(&b, &codec);
    let instance = authority();
    let access = Permission::default();
    let driving = RefCell::new(());
    let result = ReadResultSlot::new();
    let mut runtime = ReadRuntime::bind(&driving, instance.reference(), &d, &access).unwrap();
    let occupied = driving.borrow_mut();
    assert!(matches!(
        runtime.read(&result, Deadline::NONE, now(0)),
        Err(CoreError::Lifecycle(_))
    ));
    drop(occupied);
    access.denied.set(true);
    let error = runtime.read(&result, Deadline::NONE, now(0)).err().unwrap();
    assert!(matches!(error, CoreError::Security { .. }));
    drop(runtime);
    assert_eq!(stats.starts.get(), 0);
}

#[test]
fn last_owner_drop_disposes_native_io_locally_without_claiming_cleanup_ack() {
    let peer = Peer::new();
    let open = |_: &str| peer.open();
    let stats = Statistics::default();
    let b = TestBinding {
        open: &open,
        stats: &stats,
        configuration: 7,
    };
    let codec = NumberCodec::ready();
    let d = definition(&b, &codec);
    let instance = authority();
    let access = Permission::default();
    let driving = RefCell::new(());
    let result = ReadResultSlot::new();
    let mut runtime = ReadRuntime::bind(&driving, instance.reference(), &d, &access).unwrap();
    let _lost = runtime.read(&result, Deadline::NONE, now(0)).unwrap();
    peer.assert_open();
    drop(runtime);
    peer.assert_released();
    assert_eq!(stats.calls_dropped.get(), 1);
    assert_eq!(stats.cleanup_acks.get(), 0);
}

#[test]
fn dropping_active_or_taken_work_refuses_a_sibling_datagram() {
    for (taken, in_turn) in [(false, false), (true, false), (false, true), (true, true)] {
        let driving = RefCell::new(());
        let issuer = authority();
        let access = Permission::default();
        let codec = NumberCodec::ready();
        let sibling_peer = Peer::new();
        let open_sibling = |_: &str| sibling_peer.open();
        let sibling_stats = Statistics::default();
        let sibling_binding = TestBinding {
            open: &open_sibling,
            stats: &sibling_stats,
            configuration: 7,
        };
        let sibling_definition = definition(&sibling_binding, &codec);
        let sibling_slot = ReadResultSlot::new();
        let sibling = RefCell::new(
            ReadRuntime::bind(&driving, issuer.reference(), &sibling_definition, &access).unwrap(),
        );
        let sibling_handle = sibling
            .borrow_mut()
            .read(&sibling_slot, Deadline::NONE, now(0))
            .unwrap();
        let callback = || {
            let mut cx = Context::from_waker(core::task::Waker::noop());
            assert!(matches!(
                sibling.borrow_mut().step(&mut cx, now(9)),
                Poll::Ready(Err(CoreError::Lifecycle(_)))
            ));
        };
        let peer = Peer::new();
        let open = |_: &str| peer.open();
        let stats = Statistics::default();
        let binding = TestBinding {
            open: &open,
            stats: &stats,
            configuration: 7,
        };
        let definition = definition(&binding, &codec);
        let slot = ReadResultSlot::new();
        let mut runtime =
            ReadRuntime::bind(&driving, issuer.reference(), &definition, &access).unwrap();
        let handle = runtime.read(&slot, Deadline::NONE, now(0)).unwrap();
        let mut cx = Context::from_waker(core::task::Waker::noop());
        assert!(runtime.step(&mut cx, now(1)).is_pending());
        let settlement = taken.then(|| {
            peer.respond(0, b"42");
            completed(runtime.poll(handle, &mut cx, now(1)))
        });
        let outer_turn = in_turn.then(|| driving.borrow_mut());
        stats.on_drop.set(Some(&callback));
        drop((runtime, settlement));
        drop(outer_turn);
        assert!(stats.on_drop.get().is_none());
        assert_eq!(sibling_stats.polls.get(), 0, "native I/O inside disposal");
        peer.assert_released();
        assert!(sibling.borrow_mut().step(&mut cx, now(2)).is_pending());
        sibling_peer.respond(0, b"9");
        let mut result = completed(sibling.borrow_mut().poll(sibling_handle, &mut cx, now(2)));
        assert_eq!(result.result().unwrap().data().unwrap().body, b"9");
    }
}

#[test]
fn held_result_excludes_the_same_backing_until_disposal() {
    let peer = Peer::new();
    let open = |_: &str| peer.open();
    let stats = Statistics::default();
    let b = TestBinding {
        open: &open,
        stats: &stats,
        configuration: 7,
    };
    let codec = NumberCodec::ready();
    let d = definition(&b, &codec);
    let instance = authority();
    let access = Permission::default();
    let driving = RefCell::new(());
    let result = ReadResultSlot::new();
    let mut runtime = ReadRuntime::bind(&driving, instance.reference(), &d, &access).unwrap();
    let h = runtime.read(&result, Deadline::NONE, now(0)).unwrap();
    let mut cx = Context::from_waker(core::task::Waker::noop());
    assert!(runtime.poll(h, &mut cx, now(1)).is_pending());
    peer.respond(0, b"42");
    let mut s = completed(runtime.poll(h, &mut cx, now(1)));
    assert!(matches!(
        runtime.read(&result, Deadline::NONE, now(2)),
        Err(CoreError::Backpressure(_))
    ));
    assert_eq!(s.result().unwrap().data().unwrap().body, b"42");
    peer.assert_released();
    assert_eq!(stats.starts.get(), 1);
    drop(s);
    assert_eq!(stats.outputs_dropped.get(), 1);
    let k = runtime.read(&result, Deadline::NONE, now(2)).unwrap();
    assert!(matches!(runtime.cancel(h), Err(CoreError::StaleHandle(_))));
    runtime.begin_close();
    assert!(runtime.poll(k, &mut cx, now(3)).is_pending());
    let mut s = completed(runtime.poll(k, &mut cx, now(3)));
    assert!(matches!(s.result(), Err(CoreError::StaleHandle(_))));
    drop(runtime);
    drop(s);
    assert_eq!(stats.starts.get(), 2);
    assert_eq!(stats.calls_dropped.get(), 2);
}

#[test]
fn unavailable_original_validator_retains_late_data_without_entering_callback() {
    let peer = Peer::new();
    let open = |_: &str| peer.open();
    let stats = Statistics::default();
    stats.late.set(true);
    let b = TestBinding {
        open: &open,
        stats: &stats,
        configuration: 7,
    };
    let codec = NumberCodec::ready();
    let d = definition(&b, &codec);
    let instance = authority();
    let access = Permission::default();
    let driving = RefCell::new(());
    let result = ReadResultSlot::new();
    let mut runtime = ReadRuntime::bind(&driving, instance.reference(), &d, &access).unwrap();
    let h = runtime.read(&result, Deadline::NONE, now(0)).unwrap();
    let mut cx = Context::from_waker(core::task::Waker::noop());
    assert!(runtime.poll(h, &mut cx, now(1)).is_pending());
    codec.available.set(false);
    assert!(runtime.poll(h, &mut cx, now(1)).is_pending());
    peer.respond(0, b"42");
    let mut s = completed(runtime.poll(h, &mut cx, now(2)));
    assert!(matches!(s.result(), Err(CoreError::Validation(_))));
    assert_eq!(
        s.native_output().unwrap().unwrap().data().unwrap().body,
        b"42"
    );
    assert_eq!(codec.validations.get(), 0);
    drop(runtime);
    drop(s);
}

#[test]
fn native_construction_cannot_grant_progress_after_callback_revocation() {
    let peer = Peer::new();
    let instance = authority();
    let open = |_: &str| {
        instance.revoke();
        peer.open()
    };
    let stats = Statistics::default();
    let b = TestBinding {
        open: &open,
        stats: &stats,
        configuration: 7,
    };
    let codec = NumberCodec::ready();
    let d = definition(&b, &codec);
    let access = Permission::default();
    let driving = RefCell::new(());
    let result = ReadResultSlot::new();
    let mut runtime = ReadRuntime::bind(&driving, instance.reference(), &d, &access).unwrap();
    let h = runtime.read(&result, Deadline::NONE, now(0)).unwrap();
    let mut cx = Context::from_waker(core::task::Waker::noop());
    assert!(runtime.poll(h, &mut cx, now(1)).is_pending());
    assert_eq!(stats.polls.get(), 0);
    let mut s = completed(runtime.poll(h, &mut cx, now(1)));
    assert!(matches!(s.result(), Err(CoreError::StaleHandle(_))));
    assert_eq!(stats.cleanup_acks.get(), 1);
    peer.assert_released();
    drop(runtime);
    drop(s);
}

#[cfg(feature = "host-meaning")]
#[test]
fn modern_host_prepared_content_lends_identical_meaning_to_caller_owned_execution() {
    use clinkz_wot_core::{
        AffordanceTarget,
        consumer::{ConsumerExecutionContent, ConsumerFiniteSchemas},
    };
    let content = ConsumerExecutionContent::try_new(
        AffordanceTarget::Property("temperature".into()),
        Operation::ReadProperty,
        ConsumerFiniteSchemas::Value(VALUES.clone()),
    )
    .unwrap();
    let shared = content.value_execution().unwrap();
    let ConsumerFiniteSchemas::Value(values) = content.schemas() else {
        panic!()
    };
    assert!(core::ptr::eq(shared.schemas(), values));
    let peer = Peer::new();
    let open = |_: &str| peer.open();
    let stats = Statistics::default();
    let b = TestBinding {
        open: &open,
        stats: &stats,
        configuration: 7,
    };
    let codec = NumberCodec::ready();
    let d = ReadDefinition::try_new(shared, &b, &PROGRAM, 7, &codec).unwrap();
    let instance = authority();
    let access = Permission::default();
    let driving = RefCell::new(());
    let result = ReadResultSlot::new();
    let mut runtime = ReadRuntime::bind(&driving, instance.reference(), &d, &access).unwrap();
    let h = runtime.read(&result, Deadline::NONE, now(0)).unwrap();
    let mut cx = Context::from_waker(core::task::Waker::noop());
    assert!(runtime.poll(h, &mut cx, now(1)).is_pending());
    peer.respond(0, b"42");
    let mut s = completed(runtime.poll(h, &mut cx, now(1)));
    assert_eq!(s.result().unwrap().data().unwrap().body, b"42");
    drop(runtime);
    drop(s);
}

#[test]
fn caller_owned_primary_read_refuses_lent_additional_response_meaning_before_native_start() {
    use clinkz_wot_core::execution::responses::{
        FiniteResponses, ResponseAlternative, ResponseData,
    };
    let responses = FiniteResponses::try_new(
        ResponseAlternative {
            content_type: "application/json".into(),
            success: true,
            data: ResponseData::Operation,
        },
        std::vec![ResponseAlternative {
            content_type: "application/json".into(),
            success: false,
            data: ResponseData::Value(PreparedSchema::Any),
        }],
    )
    .unwrap();
    let peer = Peer::new();
    let open = |_: &str| peer.open();
    let stats = Statistics::default();
    let binding = TestBinding {
        open: &open,
        stats: &stats,
        configuration: 7,
    };
    let codec = NumberCodec::ready();
    let loan = meaning().with_responses(Some(&responses));
    assert!(core::ptr::eq(loan.responses().unwrap(), &responses));
    assert!(ReadDefinition::try_new(loan, &binding, &PROGRAM, 7, &codec).is_err());
}

#[cfg(feature = "prepare")]
#[test]
fn source_read_refuses_unhandled_semantics_in_the_selected_mapping() {
    use clinkz_wot_td::context::WOT_CONTEXT_1_1;
    use serde_json::json;

    let source = json!({
        "@context": WOT_CONTEXT_1_1,
        "title": "datagrams", "id": "urn:source-read",
        "security": ["default"],
        "securityDefinitions": {"default": {"scheme": "nosec"}, "selected": {"scheme": "nosec"}},
        "properties": {"value": {"type": "integer", "forms": [
            {"href": "test-datagram:value", "contentType": "application/x-test-number", "security": ["selected"]},
            {"href": "test-datagram:other", "contentType": "application/x-test-number"}
        ]}}
    });
    let plan = PlanId::new(SlotIndex::new(0), Generation::INITIAL);
    let prepare = |source, form| {
        let thing = serde_json::from_value(source).unwrap();
        preparation::prepare(&thing, "value", form, plan)
    };
    assert!(prepare(source.clone(), 0).is_ok());

    let remote = "https://example.invalid/unknown-context";
    for (scope, context) in [
        ("", json!([WOT_CONTEXT_1_1, remote])),
        ("", json!([WOT_CONTEXT_1_1, {"@vocab": "urn:other:"}])),
        ("/properties/value", json!(remote)),
        ("/properties/value/forms/0", json!(remote)),
        ("/securityDefinitions/selected", json!(remote)),
    ] {
        let mut td = source.clone();
        td.pointer_mut(scope).unwrap()["@context"] = context;
        assert!(
            matches!(prepare(td, 0), Err(CoreError::UnsupportedOperation(_))),
            "accepted an unsupported context at {scope:?}"
        );
    }

    for field in [
        "proxy",
        "https://www.w3.org/2019/wot/security#proxy",
        "urn:test:security-extension",
    ] {
        let mut td = source.clone();
        td["securityDefinitions"]["selected"][field] = json!("https://example.invalid/proxy");
        assert!(
            matches!(
                prepare(td.clone(), 0),
                Err(CoreError::UnsupportedOperation(_))
            ),
            "accepted {field:?} on the selected security definition"
        );
        assert!(
            prepare(td, 1).is_ok(),
            "unused {field:?} blocked another Form"
        );
    }

    let mut td = source;
    td["@context"] = json!([WOT_CONTEXT_1_1]);
    td["properties"]["value"]["forms"][1]["@context"] = json!(remote);
    td["securityDefinitions"]["default"]["@context"] = json!(remote);
    assert!(prepare(td, 0).is_ok());
}

#[cfg(feature = "prepare")]
#[test]
fn selected_source_forms_execute_in_typed_slots_after_the_td_is_dropped() {
    let thing: clinkz_wot_td::thing::Thing = serde_json::from_str(
        r#"{
        "@context":"https://www.w3.org/2022/wot/td/v1.1",
        "title":"datagrams","id":"urn:source-read",
        "security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}},
        "properties":{
            "a":{"type":"integer","minimum":10,"forms":[
                {"href":"test-datagram:unused","op":"writeproperty"},
                {"href":"test-datagram:a","contentType":"application/x-test-number"}]},
            "温度":{"type":"integer","maximum":30,"forms":[
                {"href":"test-datagram:unused","op":[]},
                {"href":"test-datagram:b","contentType":"application/x-test-number"}]}
        }
    }"#,
    )
    .unwrap();
    let plan = |n| PlanId::new(SlotIndex::new(n), Generation::INITIAL);
    assert!(preparation::prepare(&thing, "a", 0, plan(0)).is_err());
    let a = preparation::prepare(&thing, "a", 1, plan(1)).unwrap();
    let b = preparation::prepare(&thing, "温度", 1, plan(2)).unwrap();
    drop(thing);

    let peers = [Peer::new(), Peer::new()];
    let open = |target: &str| match target {
        "test-datagram:a" => peers[0].open(),
        "test-datagram:b" => peers[1].open(),
        _ => panic!("did not compile the selected Form"),
    };
    let stats = Statistics::default();
    let binding = TestBinding {
        open: &open,
        stats: &stats,
        configuration: 7,
    };
    let codec = NumberCodec::ready();
    let instance = authority();
    let access = Permission::default();
    let driving = RefCell::new(());
    let slot = ReadResultSlot::new();
    let mut cx = Context::from_waker(core::task::Waker::noop());
    for (index, prepared) in [&a, &b].into_iter().enumerate() {
        let program = prepared.program();
        let definition =
            ReadDefinition::try_new(prepared.meaning(), &binding, &program, 7, &codec).unwrap();
        let mut runtime =
            ReadRuntime::bind(&driving, instance.reference(), &definition, &access).unwrap();
        let handle = runtime.read(&slot, Deadline::NONE, now(0)).unwrap();
        assert!(runtime.poll(handle, &mut cx, now(0)).is_pending());
        let expected = [b"17", b"29"][index];
        peers[index].respond(0, expected);
        let mut result = completed(runtime.poll(handle, &mut cx, now(0)));
        assert_eq!(result.mapping().plan, plan(index as u32 + 1));
        assert_eq!(result.result().unwrap().data().unwrap().body, expected);
        peers[index].assert_released();
    }
    assert_eq!(stats.starts.get(), 2);
}
