use super::*;
use clinkz_wot_core::execution::read::LocalConsumerInstance;
use clinkz_wot_core::{
    AffordanceSite, ThingSlotId, data_schema::ValueSchemas, security::SecurityRequirement,
};
use clinkz_wot_foundation::{ClockId, Generation, MonotonicInstant, SlotIndex};
use clinkz_wot_servient_finite::borrowed_action::{
    ActionDefinition, ActionHandle, ActionResultSlot, ActionSettlement, ActionSlot,
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
type Slot<'a, 'native> = ActionSlot<'a, Binding<'native, Socket>, Input<'native>>;
fn drive<B: ActionBinding, I: ActionInput>(slot: &mut ActionSlot<'_, B, I>, now: MonotonicInstant) {
    let mut cx = Context::from_waker(core::task::Waker::noop());
    for _ in 0..3 {
        if slot.step(&mut cx, now).unwrap() != ActionTurn::Advanced {
            return;
        }
    }
}
type Settlement<'a, 'native> = ActionSettlement<'a, Binding<'native, Socket>, Input<'native>>;
struct Case<'a, 'native: 'a> {
    peer: &'a Peer,
    stats: &'native Statistics,
    invoke_codec: &'a Codec<'a>,
    query_codec: &'a Codec<'a>,
    authority: &'a LocalConsumerInstance,
    invoke_access: &'a Permission<'a>,
    query_access: &'a Permission<'a>,
    slot: Slot<'a, 'native>,
    other: Slot<'a, 'native>,
    driving: &'a RefCell<()>,
    definition: &'a ActionDefinition<'a, Program<'native>, u32>,
    binding: &'a Binding<'native, Socket>,
    result: &'a ActionResultSlot<Output<'native>>,
    other_result: &'a ActionResultSlot<Output<'native>>,
}
impl<'a, 'native: 'a> Case<'a, 'native> {
    fn start(&mut self) -> ActionHandle<'a> {
        self.slot
            .invoke(
                Some(Input {
                    body: b"7",
                    stats: self.stats,
                }),
                self.result,
                Deadline::NONE,
                now(1),
            )
            .unwrap_or_else(|_| panic!("admission"))
    }
    fn step(&mut self) {
        drive(&mut self.slot, now(1));
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
    fn finish(&mut self, h: ActionHandle<'a>) -> Settlement<'a, 'native> {
        if self.slot.state(h).unwrap() == ConsumerActionState::Closing {
            self.step();
        }
        self.slot.take_settlement(h, now(1)).unwrap()
    }
    fn complete(&mut self) -> ActionHandle<'a> {
        let h = self.accept();
        self.slot.query(h, now(1)).unwrap();
        self.exchange(&[b'Q', 7], b"O\x0042");
        h
    }
}
fn fixture_with(meaning: ValueExecution<'_>, f: impl FnOnce(Case<'_, '_>)) {
    let peer = Peer::new();
    let open = |_: &str| peer.open();
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
    let definition = ActionDefinition::try_new(meaning, &program, 4).unwrap();
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
    let driving = RefCell::new(());
    let data = [
        Some(&invoke_codec as &dyn ValueValidator),
        Some(&query_codec as &dyn ValueValidator),
    ];
    let slot = Slot::bind(
        &driving,
        &definition,
        &binding,
        authority.reference(),
        data,
        access,
    )
    .unwrap();
    let other = Slot::bind(
        &driving,
        &definition,
        &binding,
        other_authority.reference(),
        data,
        access,
    )
    .unwrap();
    let result = ActionResultSlot::new();
    let other_result = ActionResultSlot::new();
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
        driving: &driving,
        definition: &definition,
        binding: &binding,
        result: &result,
        other_result: &other_result,
    });
    peer.released();
    assert_eq!(stats.starts.get(), stats.exchanges_dropped.get());
}
fn fixture(f: impl FnOnce(Case<'_, '_>)) {
    fixture_with(meaning(), f);
}

#[test]
fn raw_evidence_adapter_uses_one_exclusive_turn_without_policy_or_validation() {
    fixture(|mut c| {
        let h = c.complete();
        let s = c.finish(h);
        let before = c.stats.output_views.get();
        let validations = c.query_codec.number.validations.get();
        let checks = c.query_access.checks.get();
        let polls = c.stats.polls.get();
        let closes = c.stats.closes.get();
        c.query_codec.number.available.set(false);
        {
            let _turn = c.driving.borrow_mut();
            assert!(matches!(s.native_result(), Err(CoreError::Lifecycle(_))));
            assert_eq!(c.stats.output_views.get(), before);
        }
        assert_eq!(
            s.native_result()
                .unwrap()
                .unwrap()
                .unwrap()
                .data()
                .unwrap()
                .body,
            b"42"
        );
        assert_eq!(c.stats.output_views.get(), before + 1);
        assert_eq!(c.query_codec.number.validations.get(), validations);
        assert_eq!(c.query_access.checks.get(), checks);
        assert_eq!(c.stats.polls.get(), polls);
        assert_eq!(c.stats.closes.get(), closes);
        assert!(s.first_cause().is_none());
        assert_eq!(c.stats.outputs_dropped.get(), 0);
    });
}

#[test]
fn input_owner_inspection_obeys_the_same_turn_boundary() {
    fixture(|mut c| {
        let h = c.complete();
        let s = c.finish(h);
        let before = c.stats.input_views.get();
        let validations = c.invoke_codec.number.validations.get();
        {
            let _turn = c.driving.borrow_mut();
            assert!(matches!(s.input(), Err(CoreError::Lifecycle(_))));
            assert_eq!(c.stats.input_views.get(), before);
        }
        assert_eq!(s.input().unwrap().unwrap().body, b"7");
        assert_eq!(c.stats.input_views.get(), before + 1);
        assert_eq!(c.invoke_codec.number.validations.get(), validations);
        assert_eq!(c.stats.inputs_dropped.get(), 0);
    });
}

#[test]
fn absent_and_failed_native_dispositions_call_no_view_adapter() {
    for failed in [false, true] {
        fixture(|mut c| {
            let h = if failed {
                let h = c.accept();
                c.slot.query(h, now(1)).unwrap();
                c.exchange(&[b'Q', 7], b"F");
                h
            } else {
                let h = c.start();
                c.slot.cancel_local(h).unwrap();
                c.step();
                h
            };
            let s = c.finish(h);
            let views = c.stats.output_views.get();
            let observation = s.native_result().unwrap();
            if failed {
                let error = observation.unwrap().unwrap_err();
                assert!(matches!(error, CoreError::Binding(_)));
                let again = s.native_result().unwrap().unwrap().unwrap_err();
                assert!(core::ptr::eq(error, again));
            } else {
                assert!(observation.is_none());
                assert!(matches!(s.first_cause(), Some(CoreError::Cancelled(_))));
            }
            assert_eq!(c.stats.output_views.get(), views);
        });
    }
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
                c.slot.query(h, now(1)).unwrap();
                c.exchange(&[b'Q', 7], b"O\x007");
                assert_eq!(c.query_codec.number.validations.get(), 1);
                if at_handoff {
                    assert_eq!(c.stats.output_variant.get(), OutputVariant::Original);
                    c.stats.select_after_validation.set(Some(variant));
                }
                assert_eq!(c.slot.state(h).unwrap(), ConsumerActionState::Completed);
                let mut s = c.finish(h);
                assert_eq!(c.stats.output_variant.get(), variant);
                let result = s.result();
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
                assert!(s.native_result().unwrap().unwrap().is_ok());
                assert_eq!(c.stats.outputs_dropped.get(), 0);
                assert_eq!(c.stats.tokens_dropped.get(), 1);
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
        c.slot.query(h, now(1)).unwrap();
        c.exchange(&[b'Q', 7], b"O\x007");
        let mut s = c.finish(h);
        let checks = c.query_access.checks.get();
        let polls = c.stats.polls.get();
        let closes = c.stats.closes.get();
        {
            let view = s.result().unwrap();
            c.stats.output_variant.set(OutputVariant::Bytes);
            // A changed selector cannot alter the exact loan already validated.
            assert_eq!(view.data().unwrap().body, b"7");
            assert_eq!(view.data().unwrap().content_type, "text/x-test-number");
            assert_eq!(view.data().unwrap().content_coding, None);
            assert_eq!(view.status(), InteractionStatus::Ok);
            assert_eq!(view.metadata(), &InteractionOutputMetadata::default());
        }
        assert_eq!(c.query_codec.number.validations.get(), 2);
        assert!(s.result().is_err());
        assert_eq!(c.query_codec.number.validations.get(), 3);
        assert_eq!(c.query_access.checks.get(), checks + 4);
        assert_eq!(c.stats.polls.get(), polls);
        assert_eq!(c.stats.closes.get(), closes);
        assert!(s.validation_error().is_some());
        let views = c.stats.output_views.get();
        c.stats.output_variant.set(OutputVariant::Original);
        assert!(matches!(s.result(), Err(CoreError::Validation(_))));
        assert_eq!(c.stats.output_views.get(), views);
        assert_eq!(c.query_codec.number.validations.get(), 3);
        drop(s);
        assert_eq!(c.stats.outputs_dropped.get(), 1);
        assert_eq!(c.stats.tokens_dropped.get(), 1);
        assert_eq!(c.stats.inputs_dropped.get(), 1);
    });
}

#[test]
fn new_result_loans_need_current_authority_and_existing_loans_stay_valid() {
    fixture(|mut c| {
        let h = c.complete();
        let mut s = c.finish(h);
        let views = c.stats.output_views.get();
        let checks = c.query_access.checks.get();
        {
            let _turn = c.driving.borrow_mut();
            assert!(matches!(s.result(), Err(CoreError::Lifecycle(_))));
            assert_eq!(c.stats.output_views.get(), views);
            assert_eq!(c.query_access.checks.get(), checks);
            assert_eq!(c.query_codec.number.validations.get(), 1);
            assert!(s.first_cause().is_none());
        }
        let loan = s.result().unwrap();
        c.authority.revoke();
        assert_eq!(loan.data().unwrap().body, b"42");
        assert!(matches!(s.result(), Err(CoreError::StaleHandle(_))));
        assert!(c.slot.is_idle());
        assert_eq!(c.stats.outputs_dropped.get(), 0);
    });
}

#[test]
fn accepted_owner_retains_native_output_when_loan_validation_service_is_unavailable() {
    fixture(|mut c| {
        let h = c.accept();
        c.slot.query(h, now(1)).unwrap();
        c.exchange(&[b'Q', 7], b"O\x007");
        let mut s = c.finish(h);
        c.query_codec.number.available.set(false);
        assert!(s.result().is_err());
        assert_eq!(c.query_codec.number.validations.get(), 1);
        assert!(s.validation_error().is_none());
        assert_eq!(
            s.native_result()
                .unwrap()
                .unwrap()
                .unwrap()
                .data()
                .unwrap()
                .body,
            b"7"
        );
        assert_eq!(c.stats.outputs_dropped.get(), 0);
        assert_eq!(c.stats.tokens_dropped.get(), 1);
    });
}

#[test]
fn continuing_invoke_pending_query_cancel_then_completion_keeps_original_owners() {
    fixture(|mut c| {
        let h = c.accept();
        c.slot.query(h, now(1)).unwrap();
        c.exchange(&[b'Q', 7], b"P");
        c.slot.request_cancel(h, now(1)).unwrap();
        c.exchange(&[b'C', 7], b"C");
        // Retired Invoke data is no longer a dependency of Query's representation.
        c.invoke_codec.number.available.set(false);
        c.slot.query(h, now(1)).unwrap();
        c.exchange(&[b'Q', 7], b"O\x0042");
        assert_eq!(c.slot.state(h).unwrap(), ConsumerActionState::Completed);
        assert_eq!(c.stats.tokens_dropped.get(), 1);
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
        assert_eq!(s.result().unwrap().data().unwrap().body, b"42");
        assert_eq!(
            s.result().unwrap().data().unwrap().content_type,
            "text/x-test-number"
        );
        assert_eq!(s.input().unwrap().unwrap().body, b"7");
        assert_eq!(c.invoke_codec.number.validations.get(), 1);
        // Initial completion plus two separately validated application loans.
        assert_eq!(c.query_codec.number.validations.get(), 3);
        drop(s);
        assert_eq!(c.stats.tokens_dropped.get(), 1);
        assert_eq!(c.stats.outputs_dropped.get(), 1);
        assert_eq!(c.stats.inputs_dropped.get(), 1);
        c.peer.released();
        // Reusing the same slot never revives h.
        c.invoke_codec.number.available.set(true);
        let next = c.start();
        assert!(matches!(
            c.slot.query(h, now(1)),
            Err(CoreError::StaleHandle(_))
        ));
        c.exchange(b"I7", b"O\x0041");
        assert_eq!(c.finish(next).result().unwrap().data().unwrap().body, b"41");
    });
}

#[test]
fn cancellation_retains_pending_failed_close_and_raw_late_output_after_revocation() {
    fixture(|mut c| {
        let h = c.accept();
        c.slot.query(h, now(1)).unwrap();
        c.step();
        c.peer.last_open();
        c.slot.cancel_local(h).unwrap();
        c.authority.revoke();
        c.stats.close_blocked.set(true);
        let polls = c.stats.polls.get();
        c.step();
        assert_eq!(c.stats.polls.get(), polls);
        c.peer.last_open();
        assert!(c.slot.take_settlement(h, now(1)).is_err());
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
        assert!(s.result().is_err());
        let views = c.stats.output_views.get();
        assert!(matches!(s.first_cause(), Some(CoreError::Cancelled(_))));
        assert!(s.cleanup_error().is_some());
        assert_eq!(s.state(), ConsumerActionState::Closed);
        c.query_codec.number.available.set(false);
        assert_eq!(
            s.native_result()
                .unwrap()
                .unwrap()
                .unwrap()
                .data()
                .unwrap()
                .body,
            b"42"
        );
        assert_eq!(c.stats.output_views.get(), views + 1);
        assert_eq!(c.query_codec.number.validations.get(), 0);
        assert_eq!(c.stats.outputs_dropped.get(), 0);
        assert_eq!(c.stats.tokens_dropped.get(), 1);
        drop(s);
        assert_eq!(c.stats.outputs_dropped.get(), 1);
        assert_eq!(c.stats.tokens_dropped.get(), 1);
    });
}

#[test]
fn poll_error_and_late_output_survive_repeated_physical_retirement_failure() {
    fixture(|mut c| {
        let h = c.accept();
        c.slot.query(h, now(1)).unwrap();
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
            s.native_result()
                .unwrap()
                .unwrap()
                .unwrap()
                .data()
                .unwrap()
                .body,
            b"42"
        );
        assert_eq!(c.query_codec.number.validations.get(), 0);
    });
}

#[test]
fn unread_and_held_results_exclude_admission_and_keep_fresh_permission() {
    fixture(|mut c| {
        let h = c.complete();
        let validations = c.query_codec.number.validations.get();
        // The result cell remains exclusive before and after owner handoff.
        let denied = || {
            Some(Input {
                body: b"7",
                stats: c.stats,
            })
        };
        let (input, error) = c
            .other
            .invoke(denied(), c.result, Deadline::NONE, now(1))
            .err()
            .unwrap();
        assert!(matches!(error, CoreError::Backpressure(_)));
        assert!(!c.slot.is_idle());
        c.query_access.denied.set(true);
        let mut s = c.finish(h);
        let (input, error) = c
            .other
            .invoke(input, c.result, Deadline::NONE, now(1))
            .err()
            .unwrap();
        assert!(matches!(error, CoreError::Backpressure(_)));
        assert!(matches!(s.first_cause(), Some(CoreError::Security { .. })));
        assert!(s.result().is_err());
        assert!(s.native_result().unwrap().unwrap().is_ok());
        assert_eq!(c.query_codec.number.validations.get(), validations);
        drop(s);
        let _ = c
            .other
            .invoke(input, c.result, Deadline::NONE, now(1))
            .unwrap_or_else(|_| panic!("result slot released"));
    });
}

#[test]
fn query_result_keeps_its_authorized_destination_after_native_retirement() {
    fixture(|c| {
        const QUERY_TARGET: &str = "test-datagram:query";
        let query_peer = Peer::new();
        let open = |target: &str| match target {
            QUERY_TARGET => query_peer.open(),
            "test-datagram:measure" => c.peer.open(),
            _ => Err(failed()),
        };
        let binding = Binding {
            open: &open,
            ..*c.binding
        };
        let query_access = Permission {
            allowed_target: Some(QUERY_TARGET),
            ..Permission::default()
        };
        let result = ActionResultSlot::new();
        let successor_result = ActionResultSlot::new();
        let mut slot = ActionSlot::bind(
            c.driving,
            c.definition,
            &binding,
            c.authority.reference(),
            [Some(c.invoke_codec), Some(c.query_codec)],
            [
                Some(c.invoke_access),
                Some(&query_access),
                Some(c.invoke_access),
            ],
        )
        .unwrap();
        let input = |body| {
            Some(Input {
                body,
                stats: c.stats,
            })
        };
        let original = slot
            .invoke(input(b"7"), &result, Deadline::NONE, now(1))
            .unwrap_or_else(|_| panic!("original admission"));
        drive(&mut slot, now(1));
        c.peer.respond(b"I7", &[b'A', 7]);
        drive(&mut slot, now(1));

        c.stats.query_target.set(Some(QUERY_TARGET));
        slot.query(original, now(1)).unwrap();
        drive(&mut slot, now(1));
        query_peer.respond(&[b'Q', 7], b"O\x0042");
        drive(&mut slot, now(1));
        assert_eq!(
            slot.state(original).unwrap(),
            ConsumerActionState::Completed
        );
        assert_eq!(c.stats.tokens_dropped.get(), 1);
        query_peer.released();

        // The next resolution selects the Invoke peer while policy stays current.
        c.stats.query_target.set(None);
        let mut settlement = slot.take_settlement(original, now(1)).unwrap();
        assert_eq!(settlement.first_cause(), None);
        assert_eq!(settlement.result().unwrap().data().unwrap().body, b"42");

        let successor = slot
            .invoke(input(b"8"), &successor_result, Deadline::NONE, now(1))
            .unwrap_or_else(|_| panic!("successor admission"));
        drive(&mut slot, now(1));
        c.peer.respond(b"I8", b"O\x0035");
        assert_eq!(settlement.result().unwrap().data().unwrap().body, b"42");
        drive(&mut slot, now(1));
        let mut successor = slot.take_settlement(successor, now(1)).unwrap();
        assert_eq!(successor.result().unwrap().data().unwrap().body, b"35");
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
                c.other_result,
                Deadline::NONE,
                now(1),
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
        drive(&mut c.other, now(1));
        c.peer.respond(b"I7", b"O\x0042");
        drive(&mut c.other, now(1));
        let mut current = c.other.take_settlement(other, now(1)).unwrap();
        assert_eq!(current.result().unwrap().data().unwrap().body, b"42");
        assert!(old.result().is_err());
        assert!(c.slot.is_idle());
        // The old result still owns its input after the continuation retires.
        assert_eq!(c.stats.tokens_dropped.get(), 1);
        drop(current);
        drop(old);
        assert_eq!(c.stats.tokens_dropped.get(), 1);
    });
}

#[test]
fn unused_control_drift_and_preflight_refusal_do_not_revoke_original_pending_invocation() {
    fixture(|mut c| {
        let h = c.accept();
        let starts = c.stats.starts.get();
        c.stats.query_mapping_changed.set(true);
        assert!(c.slot.query(h, now(1)).is_err());
        assert_eq!(c.stats.starts.get(), starts);
        assert_eq!(c.slot.state(h).unwrap(), ConsumerActionState::Pending);
        c.slot.request_cancel(h, now(1)).unwrap();
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
        c.slot.query(h, now(1)).unwrap();
        c.step();
        c.query_codec.number.available.set(false);
        c.peer.respond(&[b'Q', 7], b"O\x0042");
        c.step();
        let mut s = c.finish(h);
        assert!(s.validation_error().is_none());
        assert!(s.result().is_err());
        assert!(s.native_result().unwrap().unwrap().is_ok());
        assert_eq!(c.query_codec.number.validations.get(), 0);
    });
}

#[test]
fn wrong_shape_media_coding_and_schema_keep_rejected_original_output() {
    for reply in [b"O\x0142".as_slice(), b"O\x0242", b"O\x0342", b"O\x0099"] {
        fixture(|mut c| {
            let h = c.accept();
            c.slot.query(h, now(1)).unwrap();
            c.exchange(&[b'Q', 7], reply);
            let mut s = c.finish(h);
            assert!(s.validation_error().is_some());
            assert!(s.result().is_err());
            assert!(s.native_result().unwrap().unwrap().is_ok());
            assert_eq!(c.stats.outputs_dropped.get(), 0);
            drop(s);
            assert_eq!(c.stats.outputs_dropped.get(), 1);
        });
    }
}

#[test]
fn one_native_poll_per_turn_and_an_idle_invocation_never_auto_queries() {
    fixture(|mut c| {
        let h = c.start();
        let mut cx = Context::from_waker(core::task::Waker::noop());
        assert_eq!(c.slot.step(&mut cx, now(1)).unwrap(), ActionTurn::Waiting);
        c.peer.respond(b"I7", &[b'A', 7]);
        let polls = c.stats.polls.get();
        assert_eq!(c.slot.step(&mut cx, now(1)).unwrap(), ActionTurn::Advanced);
        assert_eq!(c.stats.polls.get(), polls + 1);
        assert_eq!(c.stats.closes.get(), 0);
        assert_eq!(c.slot.step(&mut cx, now(1)).unwrap(), ActionTurn::Advanced);
        assert_eq!(c.stats.polls.get(), polls + 1);
        assert_eq!(c.stats.closes.get(), 1);
        assert_eq!(c.slot.step(&mut cx, now(1)).unwrap(), ActionTurn::Advanced);
        for _ in 0..4 {
            assert_eq!(c.slot.step(&mut cx, now(1)).unwrap(), ActionTurn::Idle);
        }
        assert_eq!(c.stats.starts.get(), 1);
        assert_eq!(c.slot.state(h).unwrap(), ConsumerActionState::Pending);
        c.slot.cancel_local(h).unwrap();
        drop(c.finish(h));
    });
}

#[test]
fn policy_callback_cannot_drive_a_sibling_but_can_cancel_it() {
    fixture(|mut c| {
        let input = || {
            Some(Input {
                body: b"7",
                stats: c.stats,
            })
        };
        let sibling = c
            .other
            .invoke(input(), c.other_result, Deadline::NONE, now(1))
            .unwrap_or_else(|_| panic!("sibling admission"));
        let other = RefCell::new(c.other);
        let armed = Cell::new(false);
        let callback = || {
            if armed.replace(false) {
                let mut other = other.borrow_mut();
                let mut cx = Context::from_waker(core::task::Waker::noop());
                assert!(matches!(
                    other.step(&mut cx, now(9)),
                    Err(CoreError::Lifecycle(_))
                ));
                assert_eq!(other.state(sibling).unwrap(), ConsumerActionState::Invoking);
                other.cancel_local(sibling).unwrap();
            }
        };
        let access = Permission {
            callback: Some(&callback),
            ..Permission::default()
        };
        let mut original = ActionSlot::bind(
            c.driving,
            c.definition,
            c.binding,
            c.authority.reference(),
            [Some(c.invoke_codec), Some(c.query_codec)],
            [Some(&access), Some(c.query_access), Some(&access)],
        )
        .unwrap();
        original
            .invoke(input(), c.result, Deadline::NONE, now(1))
            .unwrap_or_else(|_| panic!("original admission"));
        armed.set(true);
        let mut cx = Context::from_waker(core::task::Waker::noop());
        assert_eq!(original.step(&mut cx, now(1)).unwrap(), ActionTurn::Waiting);
        assert_eq!(c.stats.polls.get(), 1);
        assert_eq!(c.stats.closes.get(), 0);
        assert!(!armed.get());
        assert_eq!(
            other.borrow_mut().step(&mut cx, now(1)).unwrap(),
            ActionTurn::Settled
        );
        let settlement = other.borrow_mut().take_settlement(sibling, now(1)).unwrap();
        assert!(matches!(
            settlement.first_cause(),
            Some(CoreError::Cancelled(_))
        ));
        assert_eq!(c.stats.polls.get(), 1);
        c.peer.respond(b"I7", &[b'A', 7]);
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
        let (input, error) = c
            .slot
            .invoke(Some(input), c.result, Deadline::NONE, now(1))
            .err()
            .unwrap();
        assert!(matches!(error, CoreError::Validation(_)));
        assert_eq!(c.stats.starts.get(), 0);
        assert_eq!(c.stats.inputs_dropped.get(), 0);
        drop(input);
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
                    c.result,
                    deadline,
                    now(1),
                )
                .unwrap_or_else(|_| panic!());
            c.step();
            c.peer.respond(b"I7", b"O\x0042");
            let time = if expired { now(3) } else { now(0) };
            drive(&mut c.slot, time);
            let mut s = c.slot.take_settlement(h, time).unwrap();
            if expired {
                assert!(matches!(s.first_cause(), Some(CoreError::TimedOut(_))));
            } else {
                assert!(matches!(s.first_cause(), Some(CoreError::Validation(_))));
            }
            assert!(s.native_result().unwrap().unwrap().is_ok());
            assert!(s.result().is_err());
        });
    }
}

#[test]
fn foreign_acceptance_cannot_replace_original_control_token() {
    fixture(|mut c| {
        let h = c.accept();
        c.slot.query(h, now(1)).unwrap();
        c.exchange(&[b'Q', 7], &[b'A', 9]);
        let mut s = c.finish(h);
        assert!(s.result().is_err());
        assert!(s.first_cause().is_some());
        assert_eq!(c.stats.tokens_dropped.get(), 2); // Both tokens have retired.
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
        assert!(s.native_result().unwrap().unwrap().is_ok());
        assert!(s.result().is_err());
    });
}

#[test]
fn invalid_input_leaves_the_supplied_result_slot_reusable() {
    fixture(|mut c| {
        let input = Some(Input {
            body: b"99",
            stats: c.stats,
        });
        let (input, error) = c
            .slot
            .invoke(input, c.result, Deadline::NONE, now(1))
            .err()
            .unwrap();
        assert!(matches!(error, CoreError::Validation(_)));
        assert_eq!(c.stats.starts.get(), 0);
        assert_eq!(c.stats.inputs_dropped.get(), 0);
        assert_eq!(input.as_ref().unwrap().body, b"99");
        let h = c.start();
        c.slot.cancel_local(h).unwrap();
        drop(c.finish(h));
        drop(input);
        assert_eq!(c.stats.inputs_dropped.get(), 2);
    });
}

#[test]
fn failed_remote_cancel_keeps_uncertainty_without_claiming_rollback() {
    fixture(|mut c| {
        let h = c.accept();
        c.slot.request_cancel(h, now(1)).unwrap();
        c.exchange(&[b'C', 7], b"F");
        let mut s = c.finish(h);
        assert_eq!(
            s.remote_cancellation(),
            ConsumerActionCancellation::Uncertain
        );
        assert!(s.accepted());
        assert!(s.native_result().unwrap().unwrap().is_err());
        assert!(s.result().is_err());
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
        c.slot.query(h, now(1)).unwrap();
        c.exchange(&[b'Q', 7], b"O\x0042");
        let mut s = c.finish(h);
        assert!(matches!(s.first_cause(), Some(CoreError::StaleHandle(_))));
        assert!(s.result().is_err());
        assert!(s.validation_error().is_none());
        assert_eq!(
            s.native_result()
                .unwrap()
                .unwrap()
                .unwrap()
                .data()
                .unwrap()
                .body,
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
        assert!(s.native_result().unwrap().is_none());
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
        assert_eq!(c.finish(h).result().unwrap().data().unwrap().body, b"42");
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
    }
    // First-view revocation must refuse before effects; a second view must
    // never be requested after the first has passed fresh permission checks.
    for revoke_on in [1, 2] {
        let peer = Peer::new();
        let authority = instance();
        let started_after_revocation = Cell::new(false);
        let open = |_: &str| {
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
        let definition = ActionDefinition::try_new(meaning(), &program, 4).unwrap();
        let access = Permission::default();
        let driving = RefCell::new(());
        let result_slot = ActionResultSlot::new();
        let mut slot = ActionSlot::<_, RevokingInput<'_>>::bind(
            &driving,
            &definition,
            &binding,
            authority.reference(),
            [Some(&invoke_codec), Some(&query_codec)],
            [Some(&access), Some(&access), Some(&access)],
        )
        .unwrap();
        let views = Cell::new(0);
        let input = RevokingInput {
            authority: &authority,
            views: &views,
            revoke_on,
        };
        let result = slot.invoke(Some(input), &result_slot, Deadline::NONE, now(1));
        assert!(
            !started_after_revocation.get(),
            "native start sent a real datagram after the input view revoked its issuer"
        );
        assert_eq!(views.get(), 1);
        if revoke_on == 1 {
            let (input, error) = result.err().unwrap();
            assert!(matches!(error, CoreError::StaleHandle(_)));
            assert!(core::ptr::eq(input.as_ref().unwrap().authority, &authority));
            assert!(slot.is_idle());
            assert_eq!(stats.starts.get(), 0);
            assert_eq!(invoke_codec.number.validations.get(), 0);
            peer.socket.set_nonblocking(true).unwrap();
            assert!(matches!(peer.socket.recv_from(&mut [0; 64]), Err(e)
            if e.kind() == std::io::ErrorKind::WouldBlock));
        } else {
            let handle = result.unwrap_or_else(|_| panic!("admission"));
            // This Binding opens/sends during start, before any progress turn.
            let mut wire = [0; 64];
            let (len, _) = peer.socket.recv_from(&mut wire).unwrap();
            assert_eq!(&wire[..len], b"START");
            drive(&mut slot, now(1));
            peer.respond(b"I7", b"O\x0042");
            drive(&mut slot, now(1));
            let mut settlement = slot.take_settlement(handle, now(1)).unwrap();
            assert_eq!(settlement.result().unwrap().data().unwrap().body, b"42");
            drop(settlement);
        }
        drop(slot);
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
    }
    impl Drop for SelectedInput<'_> {
        fn drop(&mut self) {
            bump(self.dropped);
        }
    }
    let peer = Peer::new();
    let authority = instance();
    let open = |_: &str| peer.open();
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
    let definition = ActionDefinition::try_new(meaning(), &program, 4).unwrap();
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
    let driving = RefCell::new(());
    let result_slot = ActionResultSlot::new();
    let mut slot = ActionSlot::<_, SelectedInput<'_>>::bind(
        &driving,
        &definition,
        &binding,
        authority.reference(),
        [Some(&invoke_codec), Some(&query_codec)],
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
    let (input, error) = {
        let _turn = driving.borrow_mut();
        slot.invoke(Some(input), &result_slot, Deadline::NONE, now(1))
            .err()
            .unwrap()
    };
    assert!(matches!(error, CoreError::Lifecycle(_)));
    assert_eq!(views.get(), 0);
    assert_eq!(checks.get(), 0);
    assert_eq!(stats.starts.get(), 0);
    assert_eq!(dropped.get(), 0);
    let handle = slot
        .invoke(input, &result_slot, Deadline::NONE, now(1))
        .unwrap_or_else(|_| panic!("admission"));
    assert_eq!(body.get(), b"99");
    assert_eq!(views.get(), 1);
    assert_eq!(invoke_codec.number.validations.get(), 1);
    drive(&mut slot, now(1));
    peer.respond(b"I7", b"O\x0042");
    drive(&mut slot, now(1));
    let mut settlement = slot.take_settlement(handle, now(1)).unwrap();
    assert_eq!(settlement.result().unwrap().data().unwrap().body, b"42");
    assert_eq!(views.get(), 1);
    assert_eq!(dropped.get(), 0);
    // The retained owner now supplies 99, even though the actual request was I7.
    assert_eq!(settlement.input().unwrap().unwrap().body, b"99");
    assert_eq!(views.get(), 2);
    drop(settlement);
    assert_eq!(dropped.get(), 1);
    drop(slot);
    assert_eq!(stats.starts.get(), stats.exchanges_dropped.get());
    peer.released();
}
