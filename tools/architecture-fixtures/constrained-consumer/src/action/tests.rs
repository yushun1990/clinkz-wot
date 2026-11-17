use super::*;
use clinkz_wot_core::execution::read::LocalConsumerInstance;
use clinkz_wot_core::{
    AffordanceSite, ThingSlotId, data_schema::ValueSchemas, security::SecurityRequirement,
};
use clinkz_wot_foundation::{
    ClockId, Generation, MonotonicInstant, ResourceAccount, ResourceKind, ResourceReservation,
    SlotIndex, WorkBudget, WorkClass,
};
use clinkz_wot_servient_finite::borrowed_action::{
    ActionDefinition, ActionHandle, ActionSettlement, ActionSlot,
};
use std::{
    cell::RefCell,
    net::{SocketAddr, UdpSocket},
    time::Duration,
    vec::Vec,
};

static SCHEMAS: ValueSchemas = ValueSchemas {
    input: Some(PreparedSchema::Integer {
        minimum: Some(1),
        maximum: Some(50),
        exclusive_minimum: None,
        exclusive_maximum: None,
    }),
    output: Some(PreparedSchema::Integer {
        minimum: Some(1),
        maximum: Some(50),
        exclusive_minimum: None,
        exclusive_maximum: None,
    }),
};
fn now(t: u64) -> MonotonicInstant {
    MonotonicInstant::new(ClockId::new(1), t)
}
fn budget() -> WorkBudget {
    let mut b = WorkBudget::new();
    for w in WorkClass::ALL {
        b.set_remaining(w, 1000);
    }
    b
}
fn meaning() -> ValueExecution<'static> {
    ValueExecution::try_new(
        AffordanceSite::Action("measure"),
        Operation::InvokeAction,
        &SCHEMAS,
        &SecurityRequirement::NOSEC,
    )
    .unwrap()
}
fn instance() -> LocalConsumerInstance {
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
struct Peer {
    socket: UdpSocket,
    ports: RefCell<Vec<SocketAddr>>,
}
impl Peer {
    fn new() -> Self {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        Self {
            socket,
            ports: RefCell::new(Vec::new()),
        }
    }
    fn open(&self) -> CoreResult<Socket> {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket.connect(self.socket.local_addr().unwrap()).unwrap();
        socket.set_nonblocking(true).unwrap();
        self.ports.borrow_mut().push(socket.local_addr().unwrap());
        Ok(Socket(socket))
    }
    fn respond(&self, expected: &[u8], response: &[u8]) {
        let mut bytes = [0; 64];
        let (n, address) = self.socket.recv_from(&mut bytes).unwrap();
        assert_eq!(&bytes[..n], expected);
        self.socket.send_to(response, address).unwrap();
    }
    fn last_open(&self) {
        assert!(UdpSocket::bind(*self.ports.borrow().last().unwrap()).is_err());
    }
    fn released(&self) {
        for port in self.ports.borrow().iter() {
            assert!(UdpSocket::bind(port).is_ok());
        }
    }
}
struct Socket(UdpSocket);
impl TestDatagrams for Socket {
    fn send(&mut self, b: &[u8]) -> CoreResult<()> {
        self.0.send(b).map(|_| ()).map_err(|_| failed())
    }
    fn receive(&mut self, b: &mut [u8]) -> CoreResult<Option<usize>> {
        match self.0.recv(b) {
            Ok(n) => Ok(Some(n)),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            _ => Err(failed()),
        }
    }
}
type Slot<'a> = ActionSlot<'a, Binding<'a, Socket>, Input<'a>>;
type Settlement<'a> = ActionSettlement<'a, Binding<'a, Socket>, Input<'a>>;
struct Case<'a> {
    peer: &'a Peer,
    stats: &'a Statistics,
    invoke_codec: &'a Codec<'a>,
    query_codec: &'a Codec<'a>,
    authority: &'a LocalConsumerInstance,
    invoke_access: &'a Permission<'a>,
    query_access: &'a Permission<'a>,
    slot: Slot<'a>,
    other: Slot<'a>,
    funding: Option<ResourceReservation<'a>>,
    other_funding: Option<ResourceReservation<'a>>,
}
impl<'a> Case<'a> {
    fn start(&mut self) -> ActionHandle<'a> {
        self.slot
            .invoke(
                Some(Input {
                    body: b"7",
                    stats: self.stats,
                }),
                self.funding.take().unwrap(),
                Deadline::NONE,
                now(1),
                &mut budget(),
            )
            .unwrap_or_else(|_| panic!("admission"))
    }
    fn step(&mut self) {
        self.slot.step(
            &mut Context::from_waker(core::task::Waker::noop()),
            now(1),
            &mut budget(),
        );
    }
    fn exchange(&mut self, expected: &[u8], reply: &[u8]) {
        self.step();
        self.peer.respond(expected, reply);
        self.step();
    }
    fn accept(&mut self) -> ActionHandle<'a> {
        let h = self.start();
        self.exchange(b"I7", &[b'A', 7]);
        assert_eq!(self.slot.state(h).unwrap(), ConsumerActionState::Pending);
        h
    }
    fn finish(&mut self, h: ActionHandle<'a>) -> Settlement<'a> {
        self.slot.take_settlement(h, now(1), &mut budget()).unwrap()
    }
    fn complete(&mut self) -> ActionHandle<'a> {
        let h = self.accept();
        self.slot.query(h, now(1), &mut budget()).unwrap();
        self.exchange(&[b'Q', 7], b"O\x0042");
        h
    }
}
fn fixture_with(meaning: ValueExecution<'_>, f: impl FnOnce(Case<'_>)) {
    let peer = Peer::new();
    let open = || peer.open();
    let stats = Statistics::default();
    let binding = Binding {
        open: &open,
        stats: &stats,
        configuration: 4,
        query_media: "text/x-test-number",
    };
    let program = Program {
        target: "test-datagram:measure",
    };
    let authority = instance();
    let other_authority = instance();
    let invoke_codec = Codec {
        number: NumberCodec::ready(),
        media: "application/x-test-number",
        revoke_on_validate: Cell::new(false),
        authority: &authority,
    };
    let query_codec = Codec {
        number: NumberCodec::ready(),
        media: "text/x-test-number",
        revoke_on_validate: Cell::new(false),
        authority: &authority,
    };
    let definition = ActionDefinition::try_new(
        meaning,
        &binding,
        &program,
        4,
        Some(&invoke_codec),
        Some(&query_codec),
    )
    .unwrap();
    let select_after_validation = || {
        if (invoke_codec.number.validations.get() >= 2 || query_codec.number.validations.get() != 0)
            && let Some(variant) = stats.select_after_validation.get()
        {
            stats.output_variant.set(variant);
        }
    };
    let invoke_access = Permission {
        callback: Some(&select_after_validation),
        ..Permission::default()
    };
    let query_access = Permission {
        callback: Some(&select_after_validation),
        ..Permission::default()
    };
    let cancel_access = Permission::default();
    let access = [
        Some(&invoke_access as &dyn ActionAccess),
        Some(&query_access as &dyn ActionAccess),
        Some(&cancel_access as &dyn ActionAccess),
    ];
    let slot = Slot::try_new(&definition, authority.reference(), access).unwrap();
    let other = Slot::try_new(&definition, other_authority.reference(), access).unwrap();
    let mut bank = account();
    let mut other_bank = account();
    let bytes = slot.required_bytes(None).unwrap();
    f(Case {
        peer: &peer,
        stats: &stats,
        invoke_codec: &invoke_codec,
        query_codec: &query_codec,
        authority: &authority,
        invoke_access: &invoke_access,
        query_access: &query_access,
        slot,
        other,
        funding: Some(bank.try_reserve(bytes).unwrap()),
        other_funding: Some(other_bank.try_reserve(bytes).unwrap()),
    });
    assert_eq!(bank.used(), 0);
    assert_eq!(other_bank.used(), 0);
    peer.released();
    assert_eq!(stats.starts.get(), stats.exchanges_dropped.get());
}
fn fixture(f: impl FnOnce(Case<'_>)) {
    fixture_with(meaning(), f);
}

#[test]
fn authorization_cannot_expose_output_selected_after_validation() {
    for variant in [
        OutputVariant::Bytes,
        OutputVariant::Media,
        OutputVariant::Coding,
        OutputVariant::Status,
        OutputVariant::Metadata,
    ] {
        for at_handoff in [false, true] {
            fixture(|mut c| {
                if !at_handoff {
                    c.stats.select_after_validation.set(Some(variant));
                }
                let h = c.accept();
                c.slot.query(h, now(1), &mut budget()).unwrap();
                c.exchange(&[b'Q', 7], b"O\x007");
                assert_eq!(c.query_codec.number.validations.get(), 1);
                if at_handoff {
                    assert_eq!(c.stats.output_variant.get(), OutputVariant::Original);
                    c.stats.select_after_validation.set(Some(variant));
                }
                assert_eq!(c.slot.state(h).unwrap(), ConsumerActionState::Completed);
                let mut s = c.finish(h);
                assert_eq!(c.stats.output_variant.get(), variant);
                let result = s.result(&mut budget());
                assert!(
                    result.is_err(),
                    "unvalidated {variant:?} escaped application handoff: {:?}",
                    result
                        .ok()
                        .and_then(|view| view.data())
                        .map(|payload| payload.body)
                );
                assert!(s.validation_error().is_some());
                assert!(matches!(s.first_cause(), Some(CoreError::Validation(_))));
                assert_eq!(s.state(), ConsumerActionState::Closed);
                assert!(s.native_result().unwrap().is_ok());
                assert_eq!(c.stats.outputs_dropped.get(), 0);
                assert_eq!(c.stats.tokens_dropped.get(), 0);
                drop(s);
                assert_eq!(c.stats.outputs_dropped.get(), 1);
                assert_eq!(c.stats.tokens_dropped.get(), 1);
            });
        }
    }
}

#[test]
fn result_loan_stays_valid_when_later_views_select_different_bytes() {
    fixture(|mut c| {
        let h = c.accept();
        c.slot.query(h, now(1), &mut budget()).unwrap();
        c.exchange(&[b'Q', 7], b"O\x007");
        let mut s = c.finish(h);
        let checks = c.query_access.checks.get();
        let polls = c.stats.polls.get();
        let closes = c.stats.closes.get();
        {
            let view = s.result(&mut budget()).unwrap();
            c.stats.output_variant.set(OutputVariant::Bytes);
            // A changed selector cannot alter the exact loan already validated.
            assert_eq!(view.data().unwrap().body, b"7");
            assert_eq!(view.data().unwrap().content_type, "text/x-test-number");
            assert_eq!(view.data().unwrap().content_coding, None);
            assert_eq!(view.status(), InteractionStatus::Ok);
            assert_eq!(view.metadata(), &InteractionOutputMetadata::default());
        }
        assert_eq!(c.query_codec.number.validations.get(), 2);
        assert!(s.result(&mut budget()).is_err());
        assert_eq!(c.query_codec.number.validations.get(), 3);
        assert_eq!(c.query_access.checks.get(), checks);
        assert_eq!(c.stats.polls.get(), polls);
        assert_eq!(c.stats.closes.get(), closes);
        assert!(s.validation_error().is_some());
        let views = c.stats.output_views.get();
        c.stats.output_variant.set(OutputVariant::Original);
        assert!(matches!(
            s.result(&mut WorkBudget::new()),
            Err(CoreError::Validation(_))
        ));
        assert_eq!(c.stats.output_views.get(), views);
        assert_eq!(c.query_codec.number.validations.get(), 3);
        let bytes = c.slot.required_bytes(None).unwrap();
        let lease = s.into_funding();
        assert_eq!(lease.bytes(), bytes);
        assert_eq!(c.stats.outputs_dropped.get(), 1);
        assert_eq!(c.stats.tokens_dropped.get(), 1);
        assert_eq!(c.stats.inputs_dropped.get(), 1);
        c.funding = Some(lease);
    });
}

#[test]
fn result_loan_credit_is_separate_from_original_owner_and_permission_handoff() {
    fixture(|mut c| {
        let h = c.accept();
        c.slot.query(h, now(1), &mut budget()).unwrap();
        c.exchange(&[b'Q', 7], b"O\x007");
        let mut s = c.finish(h);
        let views = c.stats.output_views.get();
        let checks = c.query_access.checks.get();
        let work = c.query_codec.number.work();
        for short in [WorkClass::HandlerSteps, WorkClass::CodecInputBytes] {
            let mut allowance = WorkBudget::new()
                .with_remaining(WorkClass::HandlerSteps, 1)
                .with_remaining(WorkClass::CodecInputBytes, work);
            allowance.set_remaining(short, 0);
            assert!(matches!(
                s.result(&mut allowance),
                Err(CoreError::Backpressure(_))
            ));
            assert_eq!(c.stats.output_views.get(), views);
            assert_eq!(c.query_codec.number.validations.get(), 1);
            assert_eq!(s.state(), ConsumerActionState::Completed);
            assert!(s.first_cause().is_none());
            assert!(s.validation_error().is_none());
            assert_eq!(c.stats.outputs_dropped.get(), 0);
        }
        // Ownership permission already transferred. Revocation cannot retarget
        // or retract it; this loan still needs its own data validation work.
        c.authority.revoke();
        let mut allowance = WorkBudget::new()
            .with_remaining(WorkClass::HandlerSteps, 1)
            .with_remaining(WorkClass::CodecInputBytes, work);
        assert_eq!(s.result(&mut allowance).unwrap().data().unwrap().body, b"7");
        assert_eq!(allowance.remaining(WorkClass::HandlerSteps), 0);
        assert_eq!(allowance.remaining(WorkClass::CodecInputBytes), 0);
        assert_eq!(c.query_codec.number.validations.get(), 2);
        assert_eq!(c.query_access.checks.get(), checks);
        assert!(c.slot.is_idle());
        assert_eq!(c.stats.outputs_dropped.get(), 0);
    });
}

#[test]
fn accepted_owner_retains_native_output_when_loan_validation_service_is_unavailable() {
    fixture(|mut c| {
        let h = c.accept();
        c.slot.query(h, now(1), &mut budget()).unwrap();
        c.exchange(&[b'Q', 7], b"O\x007");
        let mut s = c.finish(h);
        c.query_codec.number.available.set(false);
        assert!(s.result(&mut budget()).is_err());
        assert_eq!(c.query_codec.number.validations.get(), 1);
        assert!(s.validation_error().is_some());
        assert_eq!(
            s.native_result().unwrap().unwrap().data().unwrap().body,
            b"7"
        );
        assert_eq!(c.stats.outputs_dropped.get(), 0);
        assert_eq!(c.stats.tokens_dropped.get(), 0);
    });
}

#[test]
fn continuing_invoke_pending_query_cancel_then_completion_keeps_original_owners() {
    fixture(|mut c| {
        let h = c.accept();
        c.slot.query(h, now(1), &mut budget()).unwrap();
        c.exchange(&[b'Q', 7], b"P");
        c.slot.request_cancel(h, now(1), &mut budget()).unwrap();
        c.exchange(&[b'C', 7], b"C");
        // Retired Invoke data is no longer a dependency of Query's representation.
        c.invoke_codec.number.available.set(false);
        c.slot.query(h, now(1), &mut budget()).unwrap();
        c.exchange(&[b'Q', 7], b"O\x0042");
        assert_eq!(c.slot.state(h).unwrap(), ConsumerActionState::Completed);
        assert_eq!(c.stats.tokens_dropped.get(), 0);
        assert_eq!(c.stats.inputs_dropped.get(), 0);
        let mut s = c.finish(h);
        assert!(s.accepted());
        assert_eq!(
            s.effect(),
            clinkz_wot_core::execution::ConsumerEffect::Unknown
        );
        assert_eq!(
            s.remote_cancellation(),
            ConsumerActionCancellation::Requested
        );
        assert_eq!(s.result(&mut budget()).unwrap().data().unwrap().body, b"42");
        assert_eq!(
            s.result(&mut budget())
                .unwrap()
                .data()
                .unwrap()
                .content_type,
            "text/x-test-number"
        );
        assert_eq!(s.input().unwrap().body, b"7");
        assert_eq!(c.invoke_codec.number.validations.get(), 1);
        // Initial completion plus two separately validated application loans.
        assert_eq!(c.query_codec.number.validations.get(), 3);
        let lease = s.into_funding();
        c.funding = Some(lease);
        assert_eq!(c.stats.tokens_dropped.get(), 1);
        assert_eq!(c.stats.outputs_dropped.get(), 1);
        assert_eq!(c.stats.inputs_dropped.get(), 1);
        c.peer.released();
        // Reusing the same slot and exact original reservation never revives h.
        c.invoke_codec.number.available.set(true);
        let next = c.start();
        assert!(matches!(
            c.slot.query(h, now(1), &mut budget()),
            Err(CoreError::StaleHandle(_))
        ));
        c.exchange(b"I7", b"O\x0041");
        assert_eq!(
            c.finish(next)
                .result(&mut budget())
                .unwrap()
                .data()
                .unwrap()
                .body,
            b"41"
        );
    });
}

#[test]
fn cancellation_retains_pending_failed_close_and_validated_late_output_after_revocation() {
    fixture(|mut c| {
        let h = c.accept();
        c.slot.query(h, now(1), &mut budget()).unwrap();
        c.step();
        c.peer.last_open();
        c.slot.cancel_local(h).unwrap();
        c.authority.revoke();
        c.stats.close_blocked.set(true);
        let polls = c.stats.polls.get();
        c.step();
        assert_eq!(c.stats.polls.get(), polls);
        c.peer.last_open();
        assert!(c.slot.take_settlement(h, now(1), &mut budget()).is_err());
        c.stats.close_error.set(true);
        c.step();
        c.peer.last_open();
        c.peer.respond(&[b'Q', 7], b"O\x0042");
        c.stats.close_blocked.set(false);
        c.step();
        let mut s = c.finish(h);
        assert!(matches!(s.first_cause(), Some(CoreError::Cancelled(_))));
        assert!(s.cleanup_error().is_some());
        assert!(s.native_error().is_none());
        assert!(s.result(&mut budget()).is_err());
        assert_eq!(
            s.native_result().unwrap().unwrap().data().unwrap().body,
            b"42"
        );
        assert_eq!(c.query_codec.number.validations.get(), 1);
        assert_eq!(c.stats.outputs_dropped.get(), 0);
        assert_eq!(c.stats.tokens_dropped.get(), 0);
        drop(s);
        assert_eq!(c.stats.outputs_dropped.get(), 1);
        assert_eq!(c.stats.tokens_dropped.get(), 1);
    });
}

#[test]
fn poll_error_and_late_output_survive_repeated_physical_retirement_failure() {
    fixture(|mut c| {
        let h = c.accept();
        c.slot.query(h, now(1), &mut budget()).unwrap();
        c.stats.poll_error_with_late.set(true);
        c.stats.close_blocked.set(true);
        c.exchange(&[b'Q', 7], b"O\x0042");
        c.peer.last_open();
        c.stats.close_error.set(true);
        c.step();
        c.stats.close_error.set(true);
        c.step();
        let polls = c.stats.polls.get();
        c.step();
        assert_eq!(c.stats.polls.get(), polls);
        c.stats.close_blocked.set(false);
        c.step();
        let s = c.finish(h);
        assert!(matches!(s.first_cause(), Some(CoreError::Binding(_))));
        assert!(s.native_error().is_some());
        assert!(s.cleanup_error().is_some());
        assert!(s.validation_error().is_none());
        assert_eq!(
            s.native_result().unwrap().unwrap().data().unwrap().body,
            b"42"
        );
        assert_eq!(c.query_codec.number.validations.get(), 1);
    });
}

#[test]
fn unread_completion_requires_fresh_permission_and_short_handoff_retains_slot() {
    fixture(|mut c| {
        let h = c.complete();
        let validations = c.query_codec.number.validations.get();
        assert!(
            c.slot
                .take_settlement(h, now(1), &mut WorkBudget::new())
                .is_err()
        );
        assert!(!c.slot.is_idle());
        c.query_access.denied.set(true);
        let mut s = c.finish(h);
        assert!(matches!(s.first_cause(), Some(CoreError::Security { .. })));
        assert!(s.result(&mut budget()).is_err());
        assert!(s.native_result().unwrap().is_ok());
        assert_eq!(c.query_codec.number.validations.get(), validations);
    });
}

#[test]
fn equal_slots_share_definition_without_sharing_authority_or_work() {
    fixture(|mut c| {
        let h = c.accept();
        let other = c
            .other
            .invoke(
                Some(Input {
                    body: b"7",
                    stats: c.stats,
                }),
                c.other_funding.take().unwrap(),
                Deadline::NONE,
                now(1),
                &mut budget(),
            )
            .unwrap_or_else(|_| panic!());
        assert!(matches!(
            c.other.cancel_local(h),
            Err(CoreError::StaleHandle(_))
        ));
        c.authority.revoke();
        c.step();
        let mut old = c.finish(h);
        assert!(matches!(old.first_cause(), Some(CoreError::StaleHandle(_))));
        let mut cx = Context::from_waker(core::task::Waker::noop());
        c.other.step(&mut cx, now(1), &mut budget());
        c.peer.respond(b"I7", b"O\x0042");
        c.other.step(&mut cx, now(1), &mut budget());
        let mut current = c
            .other
            .take_settlement(other, now(1), &mut budget())
            .unwrap();
        assert_eq!(
            current.result(&mut budget()).unwrap().data().unwrap().body,
            b"42"
        );
        assert!(old.result(&mut budget()).is_err());
        assert!(c.slot.is_idle());
        // Retained old settlement still owns its original continuation/input.
        assert_eq!(c.stats.tokens_dropped.get(), 0);
        drop(current);
        drop(old);
        assert_eq!(c.stats.tokens_dropped.get(), 1);
    });
}

#[test]
fn unused_control_drift_and_prefight_refusal_do_not_revoke_original_pending_invocation() {
    fixture(|mut c| {
        let h = c.accept();
        let starts = c.stats.starts.get();
        c.stats.query_work.set(1);
        assert!(c.slot.query(h, now(1), &mut budget()).is_err());
        assert_eq!(c.stats.starts.get(), starts);
        assert_eq!(c.slot.state(h).unwrap(), ConsumerActionState::Pending);
        c.slot.request_cancel(h, now(1), &mut budget()).unwrap();
        c.exchange(&[b'C', 7], b"T");
        c.slot.cancel_local(h).unwrap();
        let s = c.finish(h);
        assert_eq!(
            s.remote_cancellation(),
            ConsumerActionCancellation::AlreadyTerminal
        );
        assert!(matches!(s.first_cause(), Some(CoreError::Cancelled(_))));
    });
}

#[test]
fn unavailable_query_codec_refuses_late_value_without_reentering_service() {
    fixture(|mut c| {
        let h = c.accept();
        c.slot.query(h, now(1), &mut budget()).unwrap();
        c.step();
        c.query_codec.number.available.set(false);
        c.peer.respond(&[b'Q', 7], b"O\x0042");
        c.step();
        let mut s = c.finish(h);
        assert!(s.validation_error().is_some());
        assert!(s.result(&mut budget()).is_err());
        assert!(s.native_result().unwrap().is_ok());
        assert_eq!(c.query_codec.number.validations.get(), 0);
    });
}

#[test]
fn wrong_shape_media_coding_and_schema_keep_rejected_original_output() {
    for reply in [b"O\x0142".as_slice(), b"O\x0242", b"O\x0342", b"O\x0099"] {
        fixture(|mut c| {
            let h = c.accept();
            c.slot.query(h, now(1), &mut budget()).unwrap();
            c.exchange(&[b'Q', 7], reply);
            let mut s = c.finish(h);
            assert!(s.validation_error().is_some());
            assert!(s.result(&mut budget()).is_err());
            assert!(s.native_result().unwrap().is_ok());
            assert_eq!(c.stats.outputs_dropped.get(), 0);
            drop(s);
            assert_eq!(c.stats.outputs_dropped.get(), 1);
        });
    }
}

#[test]
fn short_credits_enter_no_native_or_validator_callback_and_pending_never_auto_queries() {
    fixture(|mut c| {
        let h = c.start();
        for class in [
            WorkClass::ProviderProbes,
            WorkClass::BindingPolls,
            WorkClass::CleanupItems,
            WorkClass::CodecInputBytes,
        ] {
            let mut b = budget();
            b.set_remaining(class, 0);
            c.slot.step(
                &mut Context::from_waker(core::task::Waker::noop()),
                now(1),
                &mut b,
            );
            assert_eq!(c.stats.polls.get(), 0);
            assert_eq!(c.stats.closes.get(), 0);
            assert_eq!(c.invoke_codec.number.validations.get(), 1);
        }
        c.exchange(b"I7", &[b'A', 7]);
        for _ in 0..4 {
            c.step();
        }
        assert_eq!(c.stats.starts.get(), 1);
        assert_eq!(c.slot.state(h).unwrap(), ConsumerActionState::Pending);
        c.slot.cancel_local(h).unwrap();
        drop(c.finish(h));
    });
}

#[test]
fn changed_policy_during_callback_refuses_before_native_effects_and_returns_owners() {
    fixture(|mut c| {
        c.invoke_access.rotate_on_check.set(true);
        let input = Input {
            body: b"7",
            stats: c.stats,
        };
        let lease = c.funding.take().unwrap();
        let bytes = lease.bytes();
        let (input, lease, error) = c
            .slot
            .invoke(Some(input), lease, Deadline::NONE, now(1), &mut budget())
            .err()
            .unwrap();
        assert!(matches!(error, CoreError::Validation(_)));
        assert_eq!(lease.bytes(), bytes);
        assert_eq!(c.stats.starts.get(), 0);
        assert_eq!(c.stats.inputs_dropped.get(), 0);
        drop(input);
        drop(lease);
        assert_eq!(c.stats.inputs_dropped.get(), 1);
    });
}

#[test]
fn deadline_and_clock_regression_preserve_original_late_completion() {
    for expired in [true, false] {
        fixture(|mut c| {
            let deadline = if expired {
                Deadline::at(now(3))
            } else {
                Deadline::NONE
            };
            let h = c
                .slot
                .invoke(
                    Some(Input {
                        body: b"7",
                        stats: c.stats,
                    }),
                    c.funding.take().unwrap(),
                    deadline,
                    now(1),
                    &mut budget(),
                )
                .unwrap_or_else(|_| panic!());
            c.step();
            c.peer.respond(b"I7", b"O\x0042");
            let time = if expired { now(3) } else { now(0) };
            c.slot.step(
                &mut Context::from_waker(core::task::Waker::noop()),
                time,
                &mut budget(),
            );
            let mut s = c.slot.take_settlement(h, time, &mut budget()).unwrap();
            if expired {
                assert!(matches!(s.first_cause(), Some(CoreError::TimedOut(_))));
            } else {
                assert!(matches!(s.first_cause(), Some(CoreError::Validation(_))));
            }
            assert!(s.native_result().unwrap().is_ok());
            assert!(s.result(&mut budget()).is_err());
        });
    }
}

#[test]
fn foreign_acceptance_cannot_replace_original_control_token() {
    fixture(|mut c| {
        let h = c.accept();
        c.slot.query(h, now(1), &mut budget()).unwrap();
        c.exchange(&[b'Q', 7], &[b'A', 9]);
        let mut s = c.finish(h);
        assert!(s.result(&mut budget()).is_err());
        assert!(s.first_cause().is_some());
        assert_eq!(c.stats.tokens_dropped.get(), 1);
        drop(s);
        assert_eq!(c.stats.tokens_dropped.get(), 2);
    });
}

#[test]
fn handoff_callback_policy_rotation_cannot_accept_unread_success() {
    fixture(|mut c| {
        let h = c.complete();
        c.query_access.rotate_on_check.set(true);
        let mut s = c.finish(h);
        assert!(matches!(s.first_cause(), Some(CoreError::Validation(_))));
        assert!(s.native_result().unwrap().is_ok());
        assert!(s.result(&mut budget()).is_err());
    });
}

#[test]
fn input_and_short_admission_refusals_return_original_funding_before_native_start() {
    fixture(|mut c| {
        let lease = c.funding.take().unwrap();
        let bytes = lease.bytes();
        let input = Some(Input {
            body: b"99",
            stats: c.stats,
        });
        let (input, lease, error) = c
            .slot
            .invoke(input, lease, Deadline::NONE, now(1), &mut budget())
            .err()
            .unwrap();
        assert!(matches!(error, CoreError::Validation(_)));
        assert_eq!(lease.bytes(), bytes);
        assert_eq!(c.stats.starts.get(), 0);
        assert_eq!(c.stats.inputs_dropped.get(), 0);
        let (input, lease, error) = c
            .slot
            .invoke(input, lease, Deadline::NONE, now(1), &mut WorkBudget::new())
            .err()
            .unwrap();
        assert!(matches!(error, CoreError::Backpressure(_)));
        assert_eq!(lease.bytes(), bytes);
        assert_eq!(c.stats.starts.get(), 0);
        drop(input);
        drop(lease);
        assert_eq!(c.stats.inputs_dropped.get(), 1);
    });
}

#[test]
fn failed_remote_cancel_keeps_uncertainty_without_claiming_rollback() {
    fixture(|mut c| {
        let h = c.accept();
        c.slot.request_cancel(h, now(1), &mut budget()).unwrap();
        c.exchange(&[b'C', 7], b"F");
        let mut s = c.finish(h);
        assert_eq!(
            s.remote_cancellation(),
            ConsumerActionCancellation::Uncertain
        );
        assert!(s.accepted());
        assert!(s.native_result().unwrap().is_err());
        assert!(s.result(&mut budget()).is_err());
    });
}

#[test]
fn drop_of_live_slot_disposes_locally_without_claiming_close_acknowledgement() {
    fixture(|mut c| {
        c.start();
        c.step();
        c.peer.last_open();
        assert_eq!(c.stats.closes.get(), 0);
        drop(c.slot);
        assert_eq!(c.stats.closes.get(), 0);
        assert_eq!(c.stats.inputs_dropped.get(), 1);
        c.peer.released();
    });
}

#[test]
fn output_validation_callback_revocation_preserves_output_before_failed_acceptance() {
    fixture(|mut c| {
        let h = c.accept();
        c.query_codec.revoke_on_validate.set(true);
        c.slot.query(h, now(1), &mut budget()).unwrap();
        c.exchange(&[b'Q', 7], b"O\x0042");
        let mut s = c.finish(h);
        assert!(matches!(s.first_cause(), Some(CoreError::StaleHandle(_))));
        assert!(s.result(&mut budget()).is_err());
        assert!(s.validation_error().is_none());
        assert_eq!(
            s.native_result().unwrap().unwrap().data().unwrap().body,
            b"42"
        );
        assert_eq!(c.query_codec.number.validations.get(), 1);
    });
}

#[test]
fn cancellation_before_first_native_poll_closes_original_unsent_exchange() {
    fixture(|mut c| {
        let h = c.start();
        c.slot.cancel_local(h).unwrap();
        c.step();
        let s = c.finish(h);
        assert!(matches!(s.first_cause(), Some(CoreError::Cancelled(_))));
        assert_eq!(c.stats.polls.get(), 0);
        assert_eq!(c.stats.closes.get(), 1);
        assert!(!s.accepted());
        assert!(s.native_result().is_none());
        c.peer.released();
    });
}

#[cfg(feature = "host-meaning")]
#[test]
fn host_action_content_lends_same_schema_to_distinct_native_representation() {
    use clinkz_wot_core::{
        AffordanceTarget,
        consumer::{ConsumerExecutionContent, ConsumerFiniteSchemas},
    };
    let content = ConsumerExecutionContent::try_new(
        AffordanceTarget::Action("measure".into()),
        Operation::InvokeAction,
        ConsumerFiniteSchemas::Value(SCHEMAS.clone()),
    )
    .unwrap();
    let shared = content.value_execution().unwrap();
    let ConsumerFiniteSchemas::Value(schemas) = content.schemas() else {
        panic!()
    };
    assert!(core::ptr::eq(shared.schemas(), schemas));
    fixture_with(shared, |mut c| {
        let h = c.complete();
        assert_eq!(
            c.finish(h)
                .result(&mut budget())
                .unwrap()
                .data()
                .unwrap()
                .body,
            b"42"
        );
    });
}

#[test]
fn input_view_revocation_is_fenced_before_native_start() {
    struct RevokingInput<'a> {
        authority: &'a LocalConsumerInstance,
        views: &'a Cell<usize>,
        revoke_on: usize,
    }
    impl ActionInput for RevokingInput<'_> {
        fn view(&self) -> PayloadView<'_> {
            let views = self.views.get() + 1;
            self.views.set(views);
            if views == self.revoke_on {
                self.authority.revoke();
            }
            PayloadView {
                body: b"7",
                content_type: "application/x-test-number",
                content_coding: None,
            }
        }
        fn backing_bytes(&self) -> u64 {
            0
        }
    }
    // First-view revocation must refuse before effects; a second view must
    // never be requested after the first has passed fresh permission checks.
    for revoke_on in [1, 2] {
        let peer = Peer::new();
        let authority = instance();
        let started_after_revocation = Cell::new(false);
        let open = || {
            started_after_revocation.set(!authority.is_current());
            let socket = peer.open()?;
            socket.0.send(b"START").unwrap();
            Ok(socket)
        };
        let stats = Statistics::default();
        let binding = Binding {
            open: &open,
            stats: &stats,
            configuration: 4,
            query_media: "text/x-test-number",
        };
        let program = Program {
            target: "test-datagram:measure",
        };
        let invoke_codec = Codec {
            number: NumberCodec::ready(),
            media: "application/x-test-number",
            revoke_on_validate: Cell::new(false),
            authority: &authority,
        };
        let query_codec = Codec {
            number: NumberCodec::ready(),
            media: "text/x-test-number",
            revoke_on_validate: Cell::new(false),
            authority: &authority,
        };
        let definition = ActionDefinition::try_new(
            meaning(),
            &binding,
            &program,
            4,
            Some(&invoke_codec),
            Some(&query_codec),
        )
        .unwrap();
        let access = Permission::default();
        let mut bank = account();
        let mut slot = ActionSlot::<_, RevokingInput<'_>>::try_new(
            &definition,
            authority.reference(),
            [Some(&access), Some(&access), Some(&access)],
        )
        .unwrap();
        let views = Cell::new(0);
        let input = RevokingInput {
            authority: &authority,
            views: &views,
            revoke_on,
        };
        let funding = bank
            .try_reserve(slot.required_bytes(Some(&input)).unwrap())
            .unwrap();
        let bytes = funding.bytes();
        let result = slot.invoke(Some(input), funding, Deadline::NONE, now(1), &mut budget());
        assert!(
            !started_after_revocation.get(),
            "native start sent a real datagram after the input view revoked its issuer"
        );
        assert_eq!(views.get(), 1);
        if revoke_on == 1 {
            let (input, funding, error) = result.err().unwrap();
            assert!(matches!(error, CoreError::StaleHandle(_)));
            assert!(core::ptr::eq(input.as_ref().unwrap().authority, &authority));
            assert_eq!(funding.bytes(), bytes);
            assert!(slot.is_idle());
            assert_eq!(stats.starts.get(), 0);
            assert_eq!(invoke_codec.number.validations.get(), 0);
            peer.socket.set_nonblocking(true).unwrap();
            assert!(matches!(peer.socket.recv_from(&mut [0; 64]), Err(e)
            if e.kind() == std::io::ErrorKind::WouldBlock));
            drop(funding);
        } else {
            let handle = result.unwrap_or_else(|_| panic!("admission"));
            // This Binding opens/sends during start, before any progress turn.
            let mut wire = [0; 64];
            let (len, _) = peer.socket.recv_from(&mut wire).unwrap();
            assert_eq!(&wire[..len], b"START");
            let mut cx = Context::from_waker(core::task::Waker::noop());
            slot.step(&mut cx, now(1), &mut budget());
            peer.respond(b"I7", b"O\x0042");
            slot.step(&mut cx, now(1), &mut budget());
            let mut settlement = slot.take_settlement(handle, now(1), &mut budget()).unwrap();
            assert_eq!(
                settlement
                    .result(&mut budget())
                    .unwrap()
                    .data()
                    .unwrap()
                    .body,
                b"42"
            );
            let funding = settlement.into_funding();
            assert_eq!(funding.bytes(), bytes);
            drop(funding);
        }
        drop(slot);
        assert_eq!(bank.used(), 0);
        assert_eq!(stats.starts.get(), stats.exchanges_dropped.get());
        peer.released();
    }
}

#[test]
fn native_start_uses_validated_input_view_when_authorization_changes_selection() {
    struct SelectedInput<'a> {
        body: &'a Cell<&'static [u8]>,
        views: &'a Cell<usize>,
        dropped: &'a Cell<usize>,
    }
    impl ActionInput for SelectedInput<'_> {
        fn view(&self) -> PayloadView<'_> {
            bump(self.views);
            PayloadView {
                body: self.body.get(),
                content_type: "application/x-test-number",
                content_coding: None,
            }
        }
        fn backing_bytes(&self) -> u64 {
            0
        }
    }
    impl Drop for SelectedInput<'_> {
        fn drop(&mut self) {
            bump(self.dropped);
        }
    }
    let peer = Peer::new();
    let authority = instance();
    let open = || peer.open();
    let stats = Statistics::default();
    let binding = Binding {
        open: &open,
        stats: &stats,
        configuration: 4,
        query_media: "text/x-test-number",
    };
    let program = Program {
        target: "test-datagram:measure",
    };
    let invoke_codec = Codec {
        number: NumberCodec::ready(),
        media: "application/x-test-number",
        revoke_on_validate: Cell::new(false),
        authority: &authority,
    };
    let query_codec = Codec {
        number: NumberCodec::ready(),
        media: "text/x-test-number",
        revoke_on_validate: Cell::new(false),
        authority: &authority,
    };
    let definition = ActionDefinition::try_new(
        meaning(),
        &binding,
        &program,
        4,
        Some(&invoke_codec),
        Some(&query_codec),
    )
    .unwrap();
    let body = Cell::new(b"7".as_slice());
    let checks = Cell::new(0);
    let callback = || {
        checks.set(checks.get() + 1);
        if checks.get() == 2 {
            body.set(b"99");
        }
    };
    let access = Permission {
        callback: Some(&callback),
        ..Permission::default()
    };
    let mut bank = account();
    let mut slot = ActionSlot::<_, SelectedInput<'_>>::try_new(
        &definition,
        authority.reference(),
        [Some(&access), Some(&access), Some(&access)],
    )
    .unwrap();
    let views = Cell::new(0);
    let dropped = Cell::new(0);
    let input = SelectedInput {
        body: &body,
        views: &views,
        dropped: &dropped,
    };
    let funding = bank
        .try_reserve(slot.required_bytes(Some(&input)).unwrap())
        .unwrap();
    let bytes = funding.bytes();
    let (input, funding, error) = slot
        .invoke(
            Some(input),
            funding,
            Deadline::NONE,
            now(1),
            &mut WorkBudget::new(),
        )
        .err()
        .unwrap();
    assert!(matches!(error, CoreError::Backpressure(_)));
    assert_eq!(funding.bytes(), bytes);
    assert_eq!(views.get(), 0);
    assert_eq!(checks.get(), 0);
    assert_eq!(stats.starts.get(), 0);
    assert_eq!(dropped.get(), 0);
    let handle = slot
        .invoke(input, funding, Deadline::NONE, now(1), &mut budget())
        .unwrap_or_else(|_| panic!("admission"));
    assert_eq!(body.get(), b"99");
    assert_eq!(views.get(), 1);
    assert_eq!(invoke_codec.number.validations.get(), 1);
    let mut cx = Context::from_waker(core::task::Waker::noop());
    slot.step(&mut cx, now(1), &mut budget());
    peer.respond(b"I7", b"O\x0042");
    slot.step(&mut cx, now(1), &mut budget());
    let mut settlement = slot.take_settlement(handle, now(1), &mut budget()).unwrap();
    assert_eq!(
        settlement
            .result(&mut budget())
            .unwrap()
            .data()
            .unwrap()
            .body,
        b"42"
    );
    assert_eq!(views.get(), 1);
    assert_eq!(dropped.get(), 0);
    // The retained owner now supplies 99, even though the actual request was I7.
    assert_eq!(settlement.input().unwrap().body, b"99");
    assert_eq!(views.get(), 2);
    let funding = settlement.into_funding();
    assert_eq!(dropped.get(), 1);
    assert_eq!(funding.bytes(), bytes);
    drop(funding);
    drop(slot);
    assert_eq!(bank.used(), 0);
    assert_eq!(stats.starts.get(), stats.exchanges_dropped.get());
    peer.released();
}
