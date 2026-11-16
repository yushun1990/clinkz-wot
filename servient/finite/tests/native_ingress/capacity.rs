//! Deterministic accounting probes; production HTTP I/O is covered separately.
use super::*;
use clinkz_wot_core::host_accounts::HostResourceAccounts;
use clinkz_wot_foundation::{ResourceKind as K, ResourceLimits};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy)]
enum Completion {
    Pending,
    Error,
    Ready,
}
struct Controls {
    send: Cell<Completion>,
    close: Cell<Completion>,
    receive_polls: Cell<usize>,
    send_polls: Cell<usize>,
    close_polls: Cell<usize>,
    disposed: Cell<usize>,
}
struct PaidReply {
    capacity: ProducerFiniteCapacity,
    bank: Arc<HostResourceAccounts>,
    controls: Rc<Controls>,
}
impl Drop for PaidReply {
    fn drop(&mut self) {
        // Original input/reply disposal MUST precede the active/ingress refund.
        assert_eq!(self.capacity.active_work(), 1);
        for kind in [
            K::CleanupItemsMax,
            K::InFlightResponsesGlobalMax,
            K::PendingHandlerCallsGlobalMax,
            K::BindingIngressItemsGlobalMax,
        ] {
            assert!(self.bank.account(kind).used() > 0, "{kind:?}");
        }
        self.controls.disposed.set(self.controls.disposed.get() + 1);
    }
}
struct Binding {
    key: ProducerInstanceKey,
    capacity: ProducerFiniteCapacity,
    bank: Arc<HostResourceAccounts>,
    controls: Rc<Controls>,
}
impl FiniteIngressBinding for Binding {
    type Configuration = ();
    type Reply = PaidReply;
    fn identity(&self) -> (BindingId, BindingGeneration) {
        (BindingId::new(81), BindingGeneration::INITIAL)
    }
    fn configuration(&self) -> &() {
        &()
    }
    fn poll_start(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_receive(
        &mut self,
        _: &mut Context<'_>,
        _: MonotonicInstant,
    ) -> Poll<CoreResult<FiniteIngress<PaidReply>>> {
        self.controls
            .receive_polls
            .set(self.controls.receive_polls.get() + 1);
        Poll::Ready(Ok(FiniteIngress {
            candidates: vec![FiniteCandidate {
                instance: self.key,
                target: AffordanceTarget::Property("value".into()),
                operation: Operation::ReadProperty,
            }],
            input: InteractionInput::empty(),
            reply: PaidReply {
                capacity: self.capacity.clone(),
                bank: self.bank.clone(),
                controls: self.controls.clone(),
            },
        }))
    }
    fn poll_send(
        &mut self,
        _: &mut Context<'_>,
        _: &RawFiniteReply<PaidReply>,
    ) -> Poll<CoreResult<()>> {
        self.controls
            .send_polls
            .set(self.controls.send_polls.get() + 1);
        match self.controls.send.get() {
            Completion::Pending => Poll::Pending,
            Completion::Error => Poll::Ready(Err(error())),
            Completion::Ready => Poll::Ready(Ok(())),
        }
    }
    fn poll_close(
        &mut self,
        _: &mut Context<'_>,
        _: Option<&FiniteIngress<PaidReply>>,
        _: Option<&RawFiniteReply<PaidReply>>,
        _: Option<&CoreError>,
    ) -> Poll<CoreResult<FiniteDelivery>> {
        self.controls
            .close_polls
            .set(self.controls.close_polls.get() + 1);
        match self.controls.close.get() {
            Completion::Pending => Poll::Pending,
            Completion::Error => Poll::Ready(Err(error())),
            Completion::Ready => Poll::Ready(Ok(FiniteDelivery::NotDelivered)),
        }
    }
}
struct Fixture {
    owner: RawProducerServient,
    runtime: RawFiniteRuntime<Binding>,
    capacity: ProducerFiniteCapacity,
    bank: Arc<HostResourceAccounts>,
    controls: Rc<Controls>,
}
fn fixture(retained: usize) -> Fixture {
    let a = authority(80);
    let key = a.key();
    let mut owner = servient(
        vec![instance(
            a,
            Operation::ReadProperty,
            Arc::new(AtomicUsize::new(0)),
            b"1",
        )],
        registry_limits(),
    );
    let limits = [
        K::EngineLiveBytesGlobalMax,
        K::CleanupBytesMax,
        K::CleanupItemsMax,
        K::InFlightResponsesGlobalMax,
        K::PendingHandlerCallsGlobalMax,
        K::BindingIngressItemsGlobalMax,
    ]
    .into_iter()
    .fold(ResourceLimits::new([None; K::ALL.len()]), |l, k| {
        l.with_limit(k, Some(1_000_000))
    });
    let bank = Arc::new(HostResourceAccounts::new(&limits));
    let capacity =
        ProducerFiniteCapacity::try_new(&owner, super::limits(), 1, retained, bank.clone())
            .unwrap();
    let controls = Rc::new(Controls {
        send: Cell::new(Completion::Pending),
        close: Cell::new(Completion::Pending),
        receive_polls: Cell::new(0),
        send_polls: Cell::new(0),
        close_polls: Cell::new(0),
        disposed: Cell::new(0),
    });
    let binding = Binding {
        key,
        capacity: capacity.clone(),
        bank: bank.clone(),
        controls: controls.clone(),
    };
    let mut runtime = RawFiniteRuntime::try_new(
        binding,
        (),
        &owner,
        super::limits(),
        FiniteValidation::Raw,
        Deadline::NONE,
    )
    .ok()
    .unwrap()
    .with_capacity(capacity.clone())
    .ok()
    .unwrap();
    assert_eq!(
        step(&mut owner, &mut runtime),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
    runtime.activate_raw().unwrap();
    Fixture {
        owner,
        runtime,
        capacity,
        bank,
        controls,
    }
}
fn work(native: u64, cleanup: u64, queue: u64) -> WorkBudget {
    budget(5000)
        .with_remaining(WorkClass::BindingPolls, native)
        .with_remaining(WorkClass::CleanupItems, cleanup)
        .with_remaining(WorkClass::QueueOperations, queue)
}
fn poll(f: &mut Fixture, budget: &mut WorkBudget) -> Poll<RawFiniteState> {
    f.owner
        .step(&mut f.runtime, &mut context(), budget, now())
        .unwrap()
}
fn assert_empty(bank: &HostResourceAccounts) {
    for kind in K::ALL {
        assert_eq!(bank.account(kind).used(), 0, "{kind:?}");
    }
}

#[test]
fn shared_capacity_delivery_charges_current_fifo_only_at_terminal_insertion() {
    for retained in [1, 3, 32] {
        let mut f = fixture(retained);
        let baseline = f.bank.snapshot(K::ALL).map(|a| a.used());
        for _ in 0..3 {
            assert_eq!(poll(&mut f, &mut work(0, 0, 0)), Poll::Pending);
            assert_eq!(f.bank.snapshot(K::ALL).map(|a| a.used()), baseline);
            assert_eq!(f.controls.receive_polls.get(), 0);
        }
        for index in 0..retained {
            assert_eq!(
                poll(&mut f, &mut work(1, 0, 0)),
                Poll::Ready(RawFiniteState::Dispatching)
            );
            assert_eq!(
                poll(&mut f, &mut work(0, 0, 0)),
                Poll::Ready(RawFiniteState::Delivering)
            );
            let mut short = work(1, 0, index as u64);
            let sends = f.controls.send_polls.get();
            assert_eq!(poll(&mut f, &mut short), Poll::Pending);
            assert_eq!(f.controls.send_polls.get(), sends);
            assert_eq!(short.remaining(WorkClass::BindingPolls), 1);
            assert_eq!(short.remaining(WorkClass::QueueOperations), index as u64);

            // Reuse the SAME queue allowance across three native Pending polls
            // and the terminal poll; it is spent only on actual insertion.
            f.controls.send.set(Completion::Pending);
            let mut allowance = work(4, 0, index as u64 + 1);
            for _ in 0..3 {
                assert_eq!(poll(&mut f, &mut allowance), Poll::Pending);
                assert_eq!(
                    allowance.remaining(WorkClass::QueueOperations),
                    index as u64 + 1
                );
                assert_eq!(f.capacity.active_work(), 1);
                assert_eq!(f.controls.disposed.get(), index);
            }
            assert_eq!(f.controls.send_polls.get(), sends + 3);
            f.controls.send.set(Completion::Ready);
            assert_eq!(
                poll(&mut f, &mut allowance),
                Poll::Ready(RawFiniteState::Serving)
            );
            assert_eq!(allowance.remaining(WorkClass::BindingPolls), 0);
            assert_eq!(allowance.remaining(WorkClass::QueueOperations), 0);
            assert_eq!(f.capacity.active_work(), 0);
            assert_eq!(f.capacity.retained_work(), index + 1);
            assert_eq!(f.controls.disposed.get(), index + 1);
        }
        let receives = f.controls.receive_polls.get();
        let mut blocked = work(1, 0, 0);
        assert_eq!(poll(&mut f, &mut blocked), Poll::Pending);
        assert_eq!(blocked.remaining(WorkClass::BindingPolls), 1);
        assert_eq!(f.controls.receive_polls.get(), receives);
        // Popping an outcome updates the next insertion cost, but holding the
        // acknowledged owner still occupies the original retained allowance.
        let first = f.runtime.take_retained_settlement().unwrap();
        assert_eq!(first.sequence(), 1);
        assert_eq!(poll(&mut f, &mut blocked), Poll::Pending);
        drop(first);
        assert_eq!(
            poll(&mut f, &mut blocked),
            Poll::Ready(RawFiniteState::Dispatching)
        );
        assert_eq!(
            poll(&mut f, &mut work(0, 0, 0)),
            Poll::Ready(RawFiniteState::Delivering)
        );
        let mut exact = work(1, 0, retained as u64);
        assert_eq!(
            poll(&mut f, &mut exact),
            Poll::Ready(RawFiniteState::Serving)
        );
        assert_eq!(exact.remaining(WorkClass::QueueOperations), 0);
        let results: Vec<_> = (0..retained)
            .map(|_| f.runtime.take_retained_settlement().unwrap())
            .collect();
        assert_eq!(
            results.iter().map(|r| r.sequence()).collect::<Vec<_>>(),
            (2..=retained as u64 + 1).collect::<Vec<_>>()
        );
        for result in &results {
            assert_eq!(result.delivery(), FiniteDelivery::Delivered);
            assert_eq!(result.result().unwrap().data().unwrap().body, b"1");
        }
        let bank = f.bank.clone();
        drop(f);
        assert!(bank.account(K::CleanupItemsMax).used() > 0);
        drop(results);
        assert_empty(&bank);
    }
}

#[test]
fn shared_capacity_native_pending_does_not_spend_terminal_queue_allowance() {
    for closing in [false, true] {
        let mut f = fixture(1);
        assert_eq!(
            poll(&mut f, &mut work(1, 0, 0)),
            Poll::Ready(RawFiniteState::Dispatching)
        );
        assert_eq!(
            poll(&mut f, &mut work(0, 0, 0)),
            Poll::Ready(RawFiniteState::Delivering)
        );
        if closing {
            f.runtime.begin_close();
        }
        let mut allowance = work(3, 3, 96);
        for _ in 0..3 {
            assert_eq!(poll(&mut f, &mut allowance), Poll::Pending);
        }
        assert_eq!(
            if closing {
                f.controls.close_polls.get()
            } else {
                f.controls.send_polls.get()
            },
            3
        );
        assert_eq!(
            allowance.remaining(WorkClass::QueueOperations),
            96,
            "Pending native polls performed no FIFO insertion"
        );
        assert_eq!(
            (f.capacity.active_work(), f.capacity.retained_work()),
            (1, 1)
        );
        assert_eq!(f.controls.disposed.get(), 0);
    }
}

#[test]
fn shared_capacity_pending_and_error_retirement_keep_work_and_queue_credit() {
    for dispatch in [false, true] {
        let mut f = fixture(1);
        assert_eq!(
            poll(&mut f, &mut work(1, 0, 0)),
            Poll::Ready(RawFiniteState::Dispatching)
        );
        if dispatch {
            assert_eq!(
                poll(&mut f, &mut work(0, 0, 0)),
                Poll::Ready(RawFiniteState::Delivering)
            );
            f.controls.send.set(Completion::Error);
            let mut send = work(1, 0, 1);
            assert_eq!(
                poll(&mut f, &mut send),
                Poll::Ready(RawFiniteState::Closing)
            );
            assert_eq!(send.remaining(WorkClass::QueueOperations), 1);
        } else {
            f.runtime.begin_close();
        }
        let original_cause = f.runtime.first_cause().unwrap().clone();
        let mut pending = work(1, 3, 1);
        for _ in 0..3 {
            assert_eq!(poll(&mut f, &mut pending), Poll::Pending);
            assert_eq!(pending.remaining(WorkClass::QueueOperations), 1);
            assert_eq!(f.capacity.active_work(), 1);
            assert_eq!(f.controls.disposed.get(), 0);
        }
        assert_eq!(f.controls.close_polls.get(), 3);
        assert_eq!(pending.remaining(WorkClass::CleanupItems), 0);
        assert_eq!(pending.remaining(WorkClass::BindingPolls), 1);
        f.controls.close.set(Completion::Error);
        let mut failed = work(0, 1, 1);
        assert_eq!(
            poll(&mut f, &mut failed),
            Poll::Ready(RawFiniteState::Closing)
        );
        assert_eq!(failed.remaining(WorkClass::QueueOperations), 1);
        assert_eq!(f.runtime.first_cause(), Some(&original_cause));
        f.controls.close.set(Completion::Ready);
        for mut short in [work(0, 0, 1), work(0, 1, 0)] {
            assert_eq!(poll(&mut f, &mut short), Poll::Pending);
            assert_eq!(
                short.remaining(WorkClass::CleanupItems)
                    + short.remaining(WorkClass::QueueOperations),
                1
            );
        }
        assert_eq!(f.controls.close_polls.get(), 4);
        assert_eq!(
            (f.capacity.active_work(), f.capacity.retained_work()),
            (1, 1)
        );
        assert_eq!(f.controls.disposed.get(), 0);
        let mut terminal = work(0, 1, 1);
        assert_eq!(
            poll(&mut f, &mut terminal),
            Poll::Ready(RawFiniteState::Closed)
        );
        assert_eq!(terminal.remaining(WorkClass::QueueOperations), 0);
        assert_eq!(terminal.remaining(WorkClass::CleanupItems), 0);
        assert_eq!(f.controls.disposed.get(), 1);
        assert_eq!(
            (f.capacity.active_work(), f.capacity.retained_work()),
            (0, 1)
        );
        assert_eq!(f.runtime.first_cause(), Some(&original_cause));
        let result = f.runtime.take_retained_settlement().unwrap();
        assert_eq!(result.delivery(), FiniteDelivery::NotDelivered);
        assert_eq!(
            result.effect(),
            if dispatch {
                FiniteEffect::Unknown
            } else {
                FiniteEffect::NotInvoked
            }
        );
        assert_eq!(result.result().is_ok(), dispatch);
        let bank = f.bank.clone();
        drop(f);
        assert!(bank.account(K::CleanupItemsMax).used() > 0);
        drop(result);
        assert_empty(&bank);
    }
}
