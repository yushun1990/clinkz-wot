use super::*;
use clinkz_wot_core::{ThingSlotId, data_schema::ValueSchemas, security::SecurityRequirement};
use clinkz_wot_foundation::{
    ClockId, Generation, MonotonicInstant, ResourceAccount, ResourceKind, SlotIndex,
};
use clinkz_wot_servient_finite::borrowed_consumer::{ReadDefinition, ReadRuntime, ReadSettlement};
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
fn budget() -> WorkBudget {
    let mut b = WorkBudget::new();
    for class in WorkClass::ALL {
        b.set_remaining(class, 1000);
    }
    b
}
fn authority() -> LocalConsumerInstance {
    LocalConsumerInstance::new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL))
}
fn account() -> ResourceAccount {
    ResourceAccount::new(
        SlotIndex::new(1),
        Generation::INITIAL,
        ResourceKind::EngineLiveBytesGlobalMax,
        100_000,
    )
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
fn native_read_short_work_unread_result_and_exact_final_release() {
    let peer = Peer::new();
    let open = || peer.open();
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
    let mut bank = account();
    let mut runtime = ReadRuntime::bind(instance.reference(), &d, &access, &mut budget()).unwrap();
    let lease = bank.try_reserve(runtime.required_bytes().unwrap()).unwrap();
    let handle = runtime
        .read(lease, Deadline::NONE, now(0), &mut budget())
        .unwrap();
    let mut cx = Context::from_waker(core::task::Waker::noop());
    assert!(
        runtime
            .poll(handle, &mut cx, now(1), &mut WorkBudget::new())
            .is_pending()
    );
    assert_eq!(stats.polls.get(), 0);
    assert!(
        runtime
            .poll(handle, &mut cx, now(1), &mut budget())
            .is_pending()
    );
    peer.respond(0, b"42e0");
    let mut short = budget();
    short.set_remaining(WorkClass::CodecInputBytes, 31);
    assert!(
        runtime
            .poll(handle, &mut cx, now(2), &mut short)
            .is_pending()
    );
    assert_eq!(codec.validations.get(), 0);
    assert_eq!(stats.calls_dropped.get(), 0);
    // Validation succeeds, but final authorization must wait for fresh credit.
    let mut handoff = budget();
    handoff.set_remaining(WorkClass::ProviderProbes, 1);
    assert!(
        runtime
            .poll(handle, &mut cx, now(2), &mut handoff)
            .is_pending()
    );
    assert_eq!(codec.validations.get(), 1);
    peer.assert_open();
    let mut resume = budget();
    resume.set_remaining(WorkClass::CodecInputBytes, 0);
    let s = completed(runtime.poll(handle, &mut cx, now(3), &mut resume));
    assert_eq!(s.result().unwrap().data().unwrap().body, b"42e0");
    assert!(s.consumer() == instance.reference());
    assert_eq!(s.mapping().plan, PROGRAM.plan);
    assert!(runtime.is_idle());
    assert!(matches!(
        runtime.cancel(handle),
        Err(CoreError::StaleHandle(_))
    ));
    drop(runtime);
    peer.assert_open();
    assert_eq!(stats.outputs_dropped.get(), 0);
    drop(s);
    peer.assert_released();
    assert_eq!(stats.calls_dropped.get(), 1);
    assert_eq!(stats.outputs_dropped.get(), 1);
    assert_eq!(bank.used(), 0);
}

#[test]
fn cancellation_keeps_failed_cleanup_and_validated_late_output_without_replay() {
    let peer = Peer::new();
    let open = || peer.open();
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
    let mut bank = account();
    let mut runtime = ReadRuntime::bind(instance.reference(), &d, &access, &mut budget()).unwrap();
    let lease = bank.try_reserve(runtime.required_bytes().unwrap()).unwrap();
    let h = runtime
        .read(lease, Deadline::NONE, now(0), &mut budget())
        .unwrap();
    let mut cx = Context::from_waker(core::task::Waker::noop());
    assert!(runtime.poll(h, &mut cx, now(1), &mut budget()).is_pending());
    runtime.cancel(h).unwrap();
    instance.revoke();
    let mut short = budget();
    short.set_remaining(WorkClass::CleanupItems, 0);
    assert!(runtime.poll(h, &mut cx, now(1), &mut short).is_pending());
    assert_eq!(stats.cancel_starts.get(), 0);
    assert!(runtime.poll(h, &mut cx, now(1), &mut budget()).is_pending());
    assert!(runtime.poll(h, &mut cx, now(1), &mut budget()).is_pending());
    peer.assert_open();
    peer.respond(0, b"42");
    let s = completed(runtime.poll(h, &mut cx, now(2), &mut budget()));
    assert!(matches!(s.result(), Err(CoreError::Cancelled(_))));
    assert!(matches!(s.cleanup_error(), Some(CoreError::Binding(_))));
    assert_eq!(s.native_output().unwrap().data().unwrap().body, b"42");
    assert_eq!(codec.validations.get(), 1);
    assert_eq!(stats.starts.get(), 1);
    assert_eq!(stats.cancel_starts.get(), 1);
    assert_eq!(stats.cancel_polls.get(), 2);
    drop(runtime);
    drop(s);
    peer.assert_released();
    assert_eq!(bank.used(), 0);
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
        let open = || peer.open();
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
        let mut bank = account();
        let mut runtime =
            ReadRuntime::bind(instance.reference(), &d, &access, &mut budget()).unwrap();
        let lease = bank.try_reserve(runtime.required_bytes().unwrap()).unwrap();
        let h = runtime
            .read(lease, Deadline::NONE, now(0), &mut budget())
            .unwrap();
        let mut cx = Context::from_waker(core::task::Waker::noop());
        assert!(runtime.poll(h, &mut cx, now(1), &mut budget()).is_pending());
        peer.respond(flags, body);
        let s = completed(runtime.poll(h, &mut cx, now(2), &mut budget()));
        assert!(s.result().is_err());
        let original = s.native_output().unwrap();
        assert_eq!(
            original.data().map(|p| p.body),
            (flags != 4).then_some(body)
        );
        assert_eq!(codec.validations.get(), calls);
        peer.assert_open();
        drop(runtime);
        drop(s);
        peer.assert_released();
        assert_eq!(stats.outputs_dropped.get(), 1);
        assert_eq!(bank.used(), 0);
    }
}

#[test]
fn same_slot_foreign_authority_and_reused_call_tokens_cannot_redirect_work() {
    let peer = Peer::new();
    let open = || peer.open();
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
    let mut original_bank = account();
    let mut replacement_bank = account();
    let mut next_bank = account();
    let mut a = ReadRuntime::bind(old.reference(), &d, &access, &mut budget()).unwrap();
    let mut z = ReadRuntime::bind(replacement.reference(), &d, &access, &mut budget()).unwrap();
    let lease = original_bank
        .try_reserve(a.required_bytes().unwrap())
        .unwrap();
    let h = a
        .read(lease, Deadline::NONE, now(0), &mut budget())
        .unwrap();
    let lease = replacement_bank
        .try_reserve(z.required_bytes().unwrap())
        .unwrap();
    let k = z
        .read(lease, Deadline::NONE, now(0), &mut budget())
        .unwrap();
    assert!(matches!(z.cancel(h), Err(CoreError::StaleHandle(_))));
    let mut cx = Context::from_waker(core::task::Waker::noop());
    old.revoke();
    assert!(a.poll(h, &mut cx, now(1), &mut budget()).is_pending());
    let cancelled = completed(a.poll(h, &mut cx, now(1), &mut budget()));
    assert!(matches!(cancelled.result(), Err(CoreError::StaleHandle(_))));
    assert!(z.poll(k, &mut cx, now(1), &mut budget()).is_pending());
    peer.respond(0, b"42");
    let s = completed(z.poll(k, &mut cx, now(1), &mut budget()));
    assert!(s.result().is_ok());
    let lease = next_bank.try_reserve(z.required_bytes().unwrap()).unwrap();
    let next = z
        .read(lease, Deadline::NONE, now(2), &mut budget())
        .unwrap();
    assert!(matches!(z.cancel(k), Err(CoreError::StaleHandle(_))));
    assert!(matches!(
        z.poll(k, &mut cx, now(2), &mut budget()),
        Poll::Ready(Err(CoreError::StaleHandle(_)))
    ));
    z.cancel(next).unwrap();
    drop(a);
    drop(z);
    drop(cancelled);
    drop(s);
    assert_eq!(original_bank.used(), 0);
    assert_eq!(replacement_bank.used(), 0);
    assert_eq!(next_bank.used(), 0);
    assert_eq!(stats.starts.get(), 3);
    assert_eq!(stats.calls_dropped.get(), 3);
}

#[test]
fn unread_success_rechecks_live_permission_without_redecoding() {
    let peer = Peer::new();
    let open = || peer.open();
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
    let mut bank = account();
    let mut runtime = ReadRuntime::bind(instance.reference(), &d, &access, &mut budget()).unwrap();
    let lease = bank.try_reserve(runtime.required_bytes().unwrap()).unwrap();
    let h = runtime
        .read(lease, Deadline::NONE, now(0), &mut budget())
        .unwrap();
    let mut cx = Context::from_waker(core::task::Waker::noop());
    assert!(runtime.step(&mut cx, now(1), &mut budget()).is_pending());
    peer.respond(0, b"42");
    assert!(runtime.step(&mut cx, now(2), &mut budget()).is_ready());
    access.denied.set(true);
    assert!(
        runtime
            .poll(h, &mut cx, now(2), &mut WorkBudget::new())
            .is_pending()
    );
    let s = completed(runtime.poll(h, &mut cx, now(2), &mut budget()));
    assert!(matches!(s.result(), Err(CoreError::Security { .. })));
    assert_eq!(s.native_output().unwrap().data().unwrap().body, b"42");
    assert_eq!(codec.validations.get(), 1);
    drop(runtime);
    drop(s);
    assert_eq!(bank.used(), 0);
}

#[test]
fn callback_revocation_and_policy_rotation_fence_before_io_and_after_validation() {
    let peer = Peer::new();
    let open = || peer.open();
    let stats = Statistics::default();
    let b = TestBinding {
        open: &open,
        stats: &stats,
        configuration: 7,
    };
    let instance = authority();
    let mut bank = account();
    let invalidate = || instance.revoke();
    let codec = NumberCodec {
        callback: Some(&invalidate),
        ..NumberCodec::ready()
    };
    let d = definition(&b, &codec);
    let access = Permission::default();
    // Changing an actual service during permission cannot qualify itself.
    access.rotate_on_check.set(true);
    assert!(ReadRuntime::bind(instance.reference(), &d, &access, &mut budget()).is_err());
    assert_eq!(stats.starts.get(), 0);
    let mut runtime = ReadRuntime::bind(instance.reference(), &d, &access, &mut budget()).unwrap();
    access.rotate_on_check.set(true);
    let lease = bank.try_reserve(runtime.required_bytes().unwrap()).unwrap();
    let (lease, error) = runtime
        .read(lease, Deadline::NONE, now(0), &mut budget())
        .err()
        .unwrap();
    assert!(matches!(error, CoreError::Validation(_)));
    drop(lease);
    drop(runtime);
    assert_eq!(stats.starts.get(), 0);
    assert_eq!(bank.used(), 0);
    let mut runtime = ReadRuntime::bind(instance.reference(), &d, &access, &mut budget()).unwrap();
    let lease = bank.try_reserve(runtime.required_bytes().unwrap()).unwrap();
    let h = runtime
        .read(lease, Deadline::NONE, now(0), &mut budget())
        .unwrap();
    let mut cx = Context::from_waker(core::task::Waker::noop());
    assert!(runtime.poll(h, &mut cx, now(1), &mut budget()).is_pending());
    peer.respond(0, b"42");
    let s = completed(runtime.poll(h, &mut cx, now(2), &mut budget()));
    assert!(matches!(s.result(), Err(CoreError::StaleHandle(_))));
    assert_eq!(s.native_output().unwrap().data().unwrap().body, b"42");
    drop(runtime);
    drop(s);
    assert_eq!(bank.used(), 0);
}

#[test]
fn time_fencing_and_cleanup_acknowledgement_preserve_first_cause() {
    for observation in [now(10), now(0), MonotonicInstant::new(ClockId::new(2), 2)] {
        let peer = Peer::new();
        let open = || peer.open();
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
        let mut bank = account();
        let mut runtime =
            ReadRuntime::bind(instance.reference(), &d, &access, &mut budget()).unwrap();
        let lease = bank.try_reserve(runtime.required_bytes().unwrap()).unwrap();
        let h = runtime
            .read(lease, Deadline::at(now(10)), now(1), &mut budget())
            .unwrap();
        let mut cx = Context::from_waker(core::task::Waker::noop());
        assert!(runtime.poll(h, &mut cx, now(2), &mut budget()).is_pending());
        assert!(
            runtime
                .poll(h, &mut cx, observation, &mut budget())
                .is_pending()
        );
        runtime.cancel(h).unwrap(); // Cannot overwrite time fencing.
        let s = completed(runtime.poll(h, &mut cx, now(11), &mut budget()));
        if observation == now(10) {
            assert!(matches!(s.result(), Err(CoreError::TimedOut(_))));
        } else {
            assert!(matches!(s.result(), Err(CoreError::InternalInvariant(_))));
        }
        assert_eq!(stats.cleanup_acks.get(), 1);
        peer.assert_released();
        drop(runtime);
        drop(s);
        assert_eq!(bank.used(), 0);
    }
}

#[test]
fn capacity_pairing_required_meaning_and_permission_refuse_without_native_effects() {
    let peer = Peer::new();
    let open = || peer.open();
    let stats = Statistics::default();
    let b = TestBinding {
        open: &open,
        stats: &stats,
        configuration: 7,
    };
    let codec = NumberCodec::ready();
    assert!(ReadDefinition::try_new(meaning(), &b, &PROGRAM, 8, &codec).is_err());
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
    let secured = SecurityRequirement {
        scheme: "basic",
        parameters: &[],
    };
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
    let mut bank = account();
    let mut denied_bank = account();
    let mut runtime = ReadRuntime::bind(instance.reference(), &d, &access, &mut budget()).unwrap();
    let lease = bank.try_reserve(1).unwrap();
    let (lease, _) = runtime
        .read(lease, Deadline::NONE, now(0), &mut budget())
        .err()
        .unwrap();
    drop(lease);
    access.denied.set(true);
    let lease = denied_bank
        .try_reserve(runtime.required_bytes().unwrap())
        .unwrap();
    let (lease, error) = runtime
        .read(lease, Deadline::NONE, now(0), &mut budget())
        .err()
        .unwrap();
    drop(lease);
    assert!(matches!(error, CoreError::Security { .. }));
    drop(runtime);
    assert_eq!(bank.used(), 0);
    assert_eq!(denied_bank.used(), 0);
    assert_eq!(stats.starts.get(), 0);
}

#[test]
fn last_owner_drop_disposes_native_io_locally_without_claiming_cleanup_ack() {
    let peer = Peer::new();
    let open = || peer.open();
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
    let mut bank = account();
    let mut runtime = ReadRuntime::bind(instance.reference(), &d, &access, &mut budget()).unwrap();
    let lease = bank.try_reserve(runtime.required_bytes().unwrap()).unwrap();
    let _lost = runtime
        .read(lease, Deadline::NONE, now(0), &mut budget())
        .unwrap();
    peer.assert_open();
    drop(runtime);
    peer.assert_released();
    assert_eq!(stats.calls_dropped.get(), 1);
    assert_eq!(stats.cleanup_acks.get(), 0);
    assert_eq!(bank.used(), 0);
}

#[test]
fn complete_caller_reservation_can_be_reused_after_disposing_original_result() {
    let peer = Peer::new();
    let open = || peer.open();
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
    let mut bank = account();
    let mut runtime = ReadRuntime::bind(instance.reference(), &d, &access, &mut budget()).unwrap();
    let bytes = runtime.required_bytes().unwrap();
    let lease = bank.try_reserve(bytes).unwrap();
    let h = runtime
        .read(lease, Deadline::NONE, now(0), &mut budget())
        .unwrap();
    let mut cx = Context::from_waker(core::task::Waker::noop());
    assert!(runtime.poll(h, &mut cx, now(1), &mut budget()).is_pending());
    peer.respond(0, b"42");
    let s = completed(runtime.poll(h, &mut cx, now(1), &mut budget()));
    let lease = s.into_funding();
    assert_eq!(lease.bytes(), bytes);
    assert_eq!(stats.outputs_dropped.get(), 1);
    peer.assert_released();
    // Refusal returns the unchanged lease too: unpaid start creates no socket.
    let (lease, _) = runtime
        .read(lease, Deadline::NONE, now(2), &mut WorkBudget::new())
        .err()
        .unwrap();
    assert_eq!(lease.bytes(), bytes);
    assert_eq!(stats.starts.get(), 1);
    let k = runtime
        .read(lease, Deadline::NONE, now(2), &mut budget())
        .unwrap();
    assert!(matches!(runtime.cancel(h), Err(CoreError::StaleHandle(_))));
    runtime.begin_close();
    assert!(runtime.poll(k, &mut cx, now(3), &mut budget()).is_pending());
    let s = completed(runtime.poll(k, &mut cx, now(3), &mut budget()));
    assert!(matches!(s.result(), Err(CoreError::StaleHandle(_))));
    drop(runtime);
    drop(s);
    assert_eq!(bank.used(), 0);
    assert_eq!(stats.starts.get(), 2);
    assert_eq!(stats.calls_dropped.get(), 2);
}

#[test]
fn unavailable_original_validator_retains_late_data_without_entering_callback() {
    let peer = Peer::new();
    let open = || peer.open();
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
    let mut bank = account();
    let mut runtime = ReadRuntime::bind(instance.reference(), &d, &access, &mut budget()).unwrap();
    let lease = bank.try_reserve(runtime.required_bytes().unwrap()).unwrap();
    let h = runtime
        .read(lease, Deadline::NONE, now(0), &mut budget())
        .unwrap();
    let mut cx = Context::from_waker(core::task::Waker::noop());
    assert!(runtime.poll(h, &mut cx, now(1), &mut budget()).is_pending());
    codec.available.set(false);
    assert!(runtime.poll(h, &mut cx, now(1), &mut budget()).is_pending());
    peer.respond(0, b"42");
    let s = completed(runtime.poll(h, &mut cx, now(2), &mut budget()));
    assert!(matches!(s.result(), Err(CoreError::Validation(_))));
    assert_eq!(s.native_output().unwrap().data().unwrap().body, b"42");
    assert_eq!(codec.validations.get(), 0);
    drop(runtime);
    drop(s);
    assert_eq!(bank.used(), 0);
}

#[test]
fn native_construction_cannot_grant_progress_after_callback_revocation() {
    let peer = Peer::new();
    let instance = authority();
    let open = || {
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
    let mut bank = account();
    let mut runtime = ReadRuntime::bind(instance.reference(), &d, &access, &mut budget()).unwrap();
    let lease = bank.try_reserve(runtime.required_bytes().unwrap()).unwrap();
    let h = runtime
        .read(lease, Deadline::NONE, now(0), &mut budget())
        .unwrap();
    let mut cx = Context::from_waker(core::task::Waker::noop());
    assert!(runtime.poll(h, &mut cx, now(1), &mut budget()).is_pending());
    assert_eq!(stats.polls.get(), 0);
    let s = completed(runtime.poll(h, &mut cx, now(1), &mut budget()));
    assert!(matches!(s.result(), Err(CoreError::StaleHandle(_))));
    assert_eq!(stats.cleanup_acks.get(), 1);
    peer.assert_released();
    drop(runtime);
    drop(s);
    assert_eq!(bank.used(), 0);
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
    let open = || peer.open();
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
    let mut bank = account();
    let mut runtime = ReadRuntime::bind(instance.reference(), &d, &access, &mut budget()).unwrap();
    let lease = bank.try_reserve(runtime.required_bytes().unwrap()).unwrap();
    let h = runtime
        .read(lease, Deadline::NONE, now(0), &mut budget())
        .unwrap();
    let mut cx = Context::from_waker(core::task::Waker::noop());
    assert!(runtime.poll(h, &mut cx, now(1), &mut budget()).is_pending());
    peer.respond(0, b"42");
    let s = completed(runtime.poll(h, &mut cx, now(1), &mut budget()));
    assert_eq!(s.result().unwrap().data().unwrap().body, b"42");
    drop(runtime);
    drop(s);
    assert_eq!(bank.used(), 0);
}
