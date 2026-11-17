//! Tight Action-turn allowances over qualified HTTP ingress and original owners.
use super::*;

const SLOTS: u64 = 4;
// The fixture's original output envelope is 4096 + 2*64 + 1. Include the
// existing manual lookup allowance so the pre-correction direct path can run.
const NORMAL_WORK: u64 = 4096 + 2 * 64 + 1 + SLOTS;
fn normal_credit() -> WorkBudget {
    WorkBudget::new()
        .with_remaining(WorkClass::CleanupItems, SLOTS)
        .with_remaining(WorkClass::HandlerSteps, NORMAL_WORK)
}
struct Deployed {
    owner: RawProducerServient,
    exposure: ComposedProducerExposure,
    app: Rc<Counters>,
    bank: Arc<HostResourceAccounts>,
    addresses: [SocketAddr; 2],
}
impl Deployed {
    fn new(mixed: bool) -> Self {
        let probes = [
            TcpListener::bind("127.0.0.1:0").unwrap(),
            TcpListener::bind("127.0.0.1:0").unwrap(),
        ];
        let addresses = probes.each_ref().map(|p| p.local_addr().unwrap());
        drop(probes);
        let mut owner = owner();
        let app = Rc::new(Counters::default());
        let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        let mut exposure = create(
            &owner,
            key(Generation::INITIAL),
            addresses[0],
            mixed.then_some(addresses[1]),
            app.clone(),
            None,
            bank.clone(),
        );
        exposure
            .participant_mut::<HttpActionRuntime>(0)
            .unwrap()
            .install_closed()
            .unwrap();
        exposure
            .step(&mut owner, &mut cx(), now(), |_, _| budget())
            .unwrap();
        exposure.publish(&mut owner, &mut budget()).unwrap();
        Self {
            owner,
            exposure,
            app,
            bank,
            addresses,
        }
    }
    fn invoke(&mut self) -> u32 {
        self.invoke_info().0
    }
    fn invoke_info(&mut self) -> (u32, String) {
        let reply = send(
            &mut self.owner,
            &mut self.exposure,
            self.addresses[0],
            "POST",
            "/run",
            "12",
        );
        assert!(reply.starts_with("HTTP/1.1 202"));
        let value: serde_json::Value =
            serde_json::from_str(reply.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        let wire = value["id"].as_str().unwrap().to_owned();
        (wire.rsplit('-').next().unwrap().parse().unwrap(), wire)
    }
    fn take(&mut self, id: u32) -> Option<RawActionSettlement> {
        self.owner
            .acknowledge_action(
                self.exposure
                    .participant_mut::<HttpActionRuntime>(0)
                    .unwrap(),
                id,
            )
            .unwrap()
    }
    fn retire(&mut self) {
        self.exposure.begin_close();
        self.exposure
            .step(&mut self.owner, &mut cx(), now(), |_, _| budget())
            .unwrap();
        self.exposure
            .retire(&mut self.owner, &mut budget())
            .unwrap();
    }
}
fn assert_empty(bank: &HostResourceAccounts) {
    for kind in ResourceKind::ALL {
        assert_eq!(bank.account(kind).used(), 0, "{kind:?}");
    }
}

#[test]
fn aggregate_normal_progress_uses_the_direct_manual_cleanup_allowance() {
    let mut deployed = Deployed::new(false);
    let manual = deployed.invoke();
    deployed.app.complete.set(true);
    assert!(
        deployed
            .owner
            .progress_actions(
                deployed
                    .exposure
                    .participant_mut::<HttpActionRuntime>(0)
                    .unwrap(),
                &mut normal_credit()
            )
            .unwrap()
            .is_ready()
    );
    assert_eq!(deployed.app.polls.get(), 1);
    let original = deployed.take(manual).unwrap();
    let aggregate = deployed.invoke();
    for _ in 0..100 {
        deployed
            .exposure
            .step(&mut deployed.owner, &mut cx(), now(), |_, lane| {
                if lane == ProducerWork::Actions {
                    normal_credit()
                } else {
                    budget()
                }
            })
            .unwrap();
    }
    assert_eq!(
        deployed.app.polls.get(),
        2,
        "100 aggregate turns must progress under the same four-unit scan allowance as direct progress_actions"
    );
    let composed = deployed.take(aggregate).unwrap();
    assert_eq!(original.result().unwrap().data().unwrap().body, b"24");
    assert_eq!(composed.result().unwrap().data().unwrap().body, b"24");
    assert_eq!(deployed.app.cancelled.get(), 0);
    deployed.retire();
    let accounts = deployed.bank.clone();
    let address = deployed.addresses[0];
    drop(deployed);
    TcpListener::bind(address).unwrap();
    assert!(
        accounts
            .account(ResourceKind::EngineLiveBytesGlobalMax)
            .used()
            > 0
    );
    drop((original, composed));
    assert_empty(&accounts);
}

#[test]
fn a_single_scan_and_callback_are_funded_before_any_application_poll() {
    let mut deployed = Deployed::new(false);
    let id = deployed.invoke();
    deployed.app.complete.set(true);
    let native = deployed
        .exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap();
    let mut below_scan = normal_credit().with_remaining(WorkClass::CleanupItems, SLOTS - 1);
    assert!(
        deployed
            .owner
            .progress_actions_closing_revoked(native, &mut below_scan)
            .unwrap()
            .is_pending()
    );
    assert_eq!(below_scan.remaining(WorkClass::CleanupItems), SLOTS - 1);
    assert_eq!(below_scan.remaining(WorkClass::HandlerSteps), NORMAL_WORK);
    assert_eq!(deployed.app.polls.get(), 0);
    let callback = NORMAL_WORK - SLOTS;
    let mut below_callback = normal_credit().with_remaining(WorkClass::HandlerSteps, callback - 1);
    assert!(
        deployed
            .owner
            .progress_actions_closing_revoked(native, &mut below_callback)
            .unwrap()
            .is_pending()
    );
    assert_eq!(below_callback.remaining(WorkClass::CleanupItems), 0);
    assert_eq!(
        below_callback.remaining(WorkClass::HandlerSteps),
        callback - 1
    );
    assert_eq!(deployed.app.polls.get(), 0);
    let mut exact = normal_credit().with_remaining(WorkClass::HandlerSteps, callback);
    assert_eq!(
        deployed
            .owner
            .progress_actions_closing_revoked(native, &mut exact)
            .unwrap(),
        Poll::Ready(Some(id))
    );
    assert_eq!(exact.remaining(WorkClass::CleanupItems), 0);
    assert_eq!(exact.remaining(WorkClass::HandlerSteps), 0);
    // Unread completion needs the scan but no second callback or validation.
    let mut unread = WorkBudget::new().with_remaining(WorkClass::CleanupItems, SLOTS);
    assert_eq!(
        deployed
            .owner
            .progress_actions_closing_revoked(native, &mut unread)
            .unwrap(),
        Poll::Ready(Some(id))
    );
    assert_eq!(unread.remaining(WorkClass::CleanupItems), 0);
    assert_eq!(deployed.app.polls.get(), 1);
    let result = deployed.take(id).unwrap();
    assert_eq!(deployed.app.polls.get(), 1);
    // Empty selection also terminates without an application allowance.
    assert_eq!(
        deployed
            .owner
            .progress_actions_closing_revoked(
                deployed
                    .exposure
                    .participant_mut::<HttpActionRuntime>(0)
                    .unwrap(),
                &mut WorkBudget::new().with_remaining(WorkClass::CleanupItems, SLOTS)
            )
            .unwrap(),
        Poll::Ready(None)
    );
    deployed.retire();
    let accounts = deployed.bank.clone();
    drop((deployed, result));
    assert_empty(&accounts);
}

#[test]
fn manual_revocation_and_described_cancel_still_allow_normal_completion() {
    let mut deployed = Deployed::new(false);
    let (cancelled, wire) = deployed.invoke_info();
    let response = send(
        &mut deployed.owner,
        &mut deployed.exposure,
        deployed.addresses[0],
        "DELETE",
        &format!("/stop?id={wire}"),
        "",
    );
    assert!(response.starts_with("HTTP/1.1 200"));
    deployed
        .exposure
        .step(&mut deployed.owner, &mut cx(), now(), |_, lane| {
            if lane == ProducerWork::Actions {
                normal_credit()
            } else {
                budget()
            }
        })
        .unwrap();
    assert_eq!(deployed.app.cancelled.get(), 1);
    assert_eq!(deployed.app.closes.get(), 0);
    assert!(deployed.take(cancelled).is_none());
    deployed.app.complete.set(true);
    deployed
        .exposure
        .step(&mut deployed.owner, &mut cx(), now(), |_, lane| {
            if lane == ProducerWork::Actions {
                normal_credit()
            } else {
                budget()
            }
        })
        .unwrap();
    let semantic = deployed.take(cancelled).unwrap();
    assert!(semantic.cancel_requested);
    assert!(!semantic.abandoned);
    assert_eq!(semantic.result().unwrap().data().unwrap().body, b"24");
    let id = deployed.invoke();
    // Local permission revocation preserves the existing manual late-result policy.
    deployed.owner.revoke(deployed.exposure.key());
    deployed.app.complete.set(true);
    let mut exact =
        normal_credit().with_remaining(WorkClass::HandlerSteps, NORMAL_WORK - SLOTS + 1);
    assert_eq!(
        deployed
            .owner
            .progress_actions(
                deployed
                    .exposure
                    .participant_mut::<HttpActionRuntime>(0)
                    .unwrap(),
                &mut exact
            )
            .unwrap(),
        Poll::Ready(Some(id))
    );
    let late = deployed.take(id).unwrap();
    assert!(late.cancel_requested);
    assert!(!late.abandoned);
    assert_eq!(deployed.app.cancelled.get(), 2);
    assert_eq!(deployed.app.closes.get(), 0);
    assert_eq!(late.result().unwrap().data().unwrap().body, b"24");
    deployed.retire();
    let accounts = deployed.bank.clone();
    drop((deployed, late, semantic));
    assert_empty(&accounts);
}

#[test]
fn a_pending_current_invocation_does_not_starve_its_sibling_under_tight_turns() {
    let mut deployed = Deployed::new(false);
    let stalled = deployed.invoke();
    let ready = deployed.invoke();
    deployed.app.complete.set(true);
    deployed.app.first_pending.set(true);
    for _ in 0..2 {
        assert!(matches!(
            deployed
                .exposure
                .step(&mut deployed.owner, &mut cx(), now(), |_, lane| {
                    if lane == ProducerWork::Actions {
                        normal_credit().with_remaining(WorkClass::HandlerSteps, NORMAL_WORK - SLOTS)
                    } else {
                        WorkBudget::new()
                    }
                }),
            Err(CoreError::Backpressure(_))
        ));
    }
    assert_eq!(deployed.app.polls.get(), 2);
    assert!(deployed.take(stalled).is_none());
    let completed = deployed.take(ready).unwrap();
    assert_eq!(completed.result().unwrap().data().unwrap().body, b"24");
    assert_eq!(deployed.app.closes.get(), 0);
    assert_eq!(deployed.app.cancelled.get(), 0);
    deployed.app.first_pending.set(false);
    deployed
        .exposure
        .step(&mut deployed.owner, &mut cx(), now(), |_, lane| {
            if lane == ProducerWork::Actions {
                normal_credit()
            } else {
                budget()
            }
        })
        .unwrap();
    let released = deployed.take(stalled).unwrap();
    assert_eq!(deployed.app.polls.get(), 3);
    deployed.retire();
    let accounts = deployed.bank.clone();
    drop((deployed, completed, released));
    assert_empty(&accounts);
}

#[test]
fn revoked_pending_and_error_close_share_tight_credit_with_an_independent_sibling() {
    let mut old = Deployed::new(false);
    let blocked = old.invoke();
    let closable = old.invoke();
    old.app.first_close_pending.set(true);
    let close_error = CoreError::Cleanup(ErrorContext::new(ErrorPhase::Cleanup, RetryClass::Never));
    let cancel_error =
        CoreError::Cancelled(ErrorContext::new(ErrorPhase::Handler, RetryClass::Never));
    *old.app.close_error.borrow_mut() = Some(close_error.clone());
    *old.app.cancel_error.borrow_mut() = Some(cancel_error.clone());

    let probes = [
        TcpListener::bind("127.0.0.1:0").unwrap(),
        TcpListener::bind("127.0.0.1:0").unwrap(),
    ];
    let addresses = probes.each_ref().map(|p| p.local_addr().unwrap());
    drop(probes);
    let app = Rc::new(Counters::default());
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let authority = clinkz_wot_core::producer::ProducerInstanceAuthority::try_new(
        ThingSlotId::new(SlotIndex::new(2), Generation::INITIAL),
    )
    .unwrap();
    let mut sibling = create(
        &old.owner,
        authority,
        addresses[0],
        Some(addresses[1]),
        app.clone(),
        None,
        bank.clone(),
    );
    sibling
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .install_closed()
        .unwrap();
    sibling
        .step(&mut old.owner, &mut cx(), now(), |_, _| budget())
        .unwrap();
    sibling.publish(&mut old.owner, &mut budget()).unwrap();
    let reply = send(
        &mut old.owner,
        &mut sibling,
        addresses[0],
        "POST",
        "/run",
        "12",
    );
    assert!(reply.starts_with("HTTP/1.1 202"));
    let live = sibling
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .action_ids()
        .next()
        .unwrap();
    app.complete.set(true);
    let old_key = old.exposure.key();
    old.exposure.begin_close();

    let mut too_short = normal_credit().with_remaining(WorkClass::CleanupItems, SLOTS - 1);
    assert!(
        old.owner
            .progress_actions_closing_revoked(
                old.exposure
                    .participant_mut::<HttpActionRuntime>(0)
                    .unwrap(),
                &mut too_short
            )
            .unwrap()
            .is_pending()
    );
    assert_eq!(old.app.cancelled.get(), 0);
    assert_eq!(old.app.closes.get(), 0);
    assert_eq!(too_short.remaining(WorkClass::CleanupItems), SLOTS - 1);
    // One shared allowance pays both scans, one cancel+close, and the live
    // sibling's exact normal output/validation envelope. No new credit in between.
    let mut shared = WorkBudget::new()
        .with_remaining(WorkClass::CleanupItems, 2 * SLOTS + 2)
        .with_remaining(WorkClass::HandlerSteps, NORMAL_WORK - SLOTS);
    assert_eq!(
        old.owner.progress_actions_closing_revoked(
            old.exposure
                .participant_mut::<HttpActionRuntime>(0)
                .unwrap(),
            &mut shared
        ),
        Err(close_error.clone())
    );
    assert_eq!(
        old.owner
            .progress_actions(
                sibling.participant_mut::<HttpActionRuntime>(0).unwrap(),
                &mut shared
            )
            .unwrap(),
        Poll::Ready(Some(live))
    );
    assert_eq!(shared.remaining(WorkClass::CleanupItems), 0);
    assert_eq!(shared.remaining(WorkClass::HandlerSteps), 0);
    assert_eq!(old.app.closes.get(), 1);
    assert_eq!(old.app.cancelled.get(), 1);
    assert_eq!(app.polls.get(), 1);
    assert_eq!(app.closes.get(), 0);
    let live_result = old
        .owner
        .acknowledge_action(
            sibling.participant_mut::<HttpActionRuntime>(0).unwrap(),
            live,
        )
        .unwrap()
        .unwrap();
    assert!(!live_result.cancel_requested);
    assert_eq!(live_result.result().unwrap().data().unwrap().body, b"24");
    assert!(old.take(blocked).is_none());
    assert!(old.exposure.retire(&mut old.owner, &mut budget()).is_err());

    // The errored first invocation cannot monopolize the same participation.
    let mut next = WorkBudget::new().with_remaining(WorkClass::CleanupItems, SLOTS + 2);
    assert_eq!(
        old.owner
            .progress_actions_closing_revoked(
                old.exposure
                    .participant_mut::<HttpActionRuntime>(0)
                    .unwrap(),
                &mut next
            )
            .unwrap(),
        Poll::Ready(Some(closable))
    );
    assert_eq!(next.remaining(WorkClass::CleanupItems), 0);
    let closed_sibling = old.take(closable).unwrap();
    assert!(matches!(
        closed_sibling.result(),
        Err(CoreError::Cancelled(_))
    ));
    assert!(closed_sibling.close_failure().is_none());
    for _ in 0..3 {
        let mut turn = WorkBudget::new().with_remaining(WorkClass::CleanupItems, SLOTS + 1);
        assert!(
            old.owner
                .progress_actions_closing_revoked(
                    old.exposure
                        .participant_mut::<HttpActionRuntime>(0)
                        .unwrap(),
                    &mut turn
                )
                .unwrap()
                .is_pending()
        );
        assert_eq!(turn.remaining(WorkClass::CleanupItems), 0);
        assert!(old.take(blocked).is_none());
    }
    assert_eq!(old.app.cancelled.get(), 2);
    let closes = old.app.closes.get();
    let mut scan_only = WorkBudget::new().with_remaining(WorkClass::CleanupItems, SLOTS);
    assert!(
        old.owner
            .progress_actions_closing_revoked(
                old.exposure
                    .participant_mut::<HttpActionRuntime>(0)
                    .unwrap(),
                &mut scan_only
            )
            .unwrap()
            .is_pending()
    );
    assert_eq!(old.app.closes.get(), closes);

    // Aggregate driving retains Pending old work while a distinct publication
    // completes actual finite HTTP I/O with its independently supplied credit.
    let mut peer = TcpStream::connect(addresses[1]).unwrap();
    peer.set_nonblocking(true).unwrap();
    peer.write_all(
        format!(
            "GET /value HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            addresses[1]
        )
        .as_bytes(),
    )
    .unwrap();
    let mut bytes = Vec::new();
    let end = Instant::now() + Duration::from_secs(3);
    loop {
        old.exposure
            .step(&mut old.owner, &mut cx(), now(), |_, lane| {
                if lane == ProducerWork::Actions {
                    WorkBudget::new().with_remaining(WorkClass::CleanupItems, SLOTS + 1)
                } else {
                    WorkBudget::new()
                }
            })
            .unwrap();
        sibling
            .step(&mut old.owner, &mut cx(), now(), |_, lane| {
                if lane == ProducerWork::Actions {
                    normal_credit()
                } else {
                    budget()
                }
            })
            .unwrap();
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
    assert!(bytes.ends_with(b"42"));
    assert!(old.take(blocked).is_none());
    assert!(old.exposure.retire(&mut old.owner, &mut budget()).is_err());
    let finite = sibling
        .participant_mut::<RawFiniteRuntime<HttpFiniteBinding>>(1)
        .unwrap()
        .take_settlement()
        .unwrap();
    drop(finite);
    old.app.first_close_pending.set(false);
    old.exposure
        .step(&mut old.owner, &mut cx(), now(), |_, lane| {
            if lane == ProducerWork::Actions {
                WorkBudget::new().with_remaining(WorkClass::CleanupItems, SLOTS + 1)
            } else {
                WorkBudget::new()
            }
        })
        .unwrap();
    let original = old.take(blocked).unwrap();
    assert_eq!(original.instance, old_key);
    assert_eq!(original.close_failure(), Some(&close_error));
    assert_eq!(original.cancel_failure(), Some(&cancel_error));
    assert!(matches!(original.result(), Err(CoreError::Cancelled(_))));
    assert_eq!(
        original.effect,
        clinkz_wot_core::action::ActionEffect::PossiblyApplied
    );
    assert_eq!(old.app.cancelled.get(), 2);
    old.retire();
    sibling.begin_close();
    for _ in 0..8 {
        sibling
            .step(&mut old.owner, &mut cx(), now(), |_, _| budget())
            .unwrap();
    }
    sibling.retire(&mut old.owner, &mut budget()).unwrap();
    let original_bank = old.bank.clone();
    let original_address = old.addresses[0];
    drop((sibling, old));
    for address in [original_address, addresses[0], addresses[1]] {
        TcpListener::bind(address).unwrap();
    }
    assert!(
        original_bank
            .account(ResourceKind::EngineLiveBytesGlobalMax)
            .used()
            > 0
    );
    assert!(bank.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0);
    drop((original, closed_sibling, live_result));
    assert_empty(&original_bank);
    assert_empty(&bank);
}
