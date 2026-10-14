//! Disposable public-SDK discriminator; this is not an engine binding.
use clinkz_wot_foundation::{Generation, ResourceAccount, ResourceKind, SlotIndex};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    future::IntoFuture,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    task::{Context, Poll, Wake, Waker},
    time::Duration,
};
use zenoh::{Wait, query::ConsolidationMode};

struct Meter;
thread_local! {
    static COUNT: Cell<usize> = const { Cell::new(0) };
    static FAIL_NEXT: Cell<bool> = const { Cell::new(false) };
}
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if FAIL_NEXT
            .try_with(|flag| flag.replace(false))
            .unwrap_or(false)
        {
            return std::ptr::null_mut();
        }
        let _ = COUNT.try_with(|count| count.set(count.get() + 1));
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let _ = COUNT.try_with(|count| count.set(count.get() + 1));
        unsafe { System.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static METER: Meter = Meter;
const WAIT: Duration = Duration::from_secs(3);

fn config() -> zenoh::Config {
    let mut c = zenoh::Config::default();
    for (key, value) in [
        ("mode", "\"peer\""),
        ("scouting/multicast/enabled", "false"),
        ("scouting/gossip/enabled", "false"),
        ("listen/endpoints", "[]"),
    ] {
        c.insert_json5(key, value).unwrap();
    }
    c
}

// A held local Query prevents a prompt final. Its timer's error callback then
// blocks on our explicit release, rather than on timing or a real remote peer.
fn blocked_timer(session: &zenoh::Session) -> (mpsc::Sender<()>, mpsc::Receiver<()>) {
    let held = Arc::new(Mutex::new(None));
    let queryable = session
        .declare_queryable("wp600/owner")
        .callback({
            let held = held.clone();
            move |query| *held.lock().unwrap() = Some(query)
        })
        .wait()
        .unwrap();
    let (entered_tx, entered) = mpsc::sync_channel(1);
    let (release, released) = mpsc::channel();
    let (done_tx, done) = mpsc::sync_channel(1);
    let released = Mutex::new(released);
    session
        .get("wp600/owner")
        .consolidation(ConsolidationMode::None)
        .timeout(Duration::from_millis(30))
        .callback(move |reply| {
            assert!(reply.result().is_err());
            entered_tx.send(()).unwrap();
            released.lock().unwrap().recv_timeout(WAIT).unwrap();
            done_tx.send(()).unwrap();
        })
        .wait()
        .unwrap();
    entered.recv_timeout(WAIT).unwrap();
    // The timer has removed QueryState before calling us. These entities no
    // longer keep the test query open and do not own a strong Session.
    drop(queryable);
    drop(held);
    (release, done)
}

fn last_session_drop() {
    let session = zenoh::open(config()).wait().unwrap();
    let (release, callback_done) = blocked_timer(&session);
    let (started_tx, started) = mpsc::sync_channel(1);
    let (ended_tx, ended) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        started_tx.send(()).unwrap();
        drop(session);
        ended_tx.send(()).unwrap();
    });
    started.recv_timeout(WAIT).unwrap();
    assert!(matches!(
        ended.recv_timeout(Duration::from_millis(100)),
        Err(mpsc::RecvTimeoutError::Timeout)
    ));
    release.send(()).unwrap();
    callback_done.recv_timeout(WAIT).unwrap();
    ended.recv_timeout(WAIT).unwrap();
    worker.join().unwrap();
    println!("last-session-drop: blocked until timer callback released");
}

struct NoWake;
impl Wake for NoWake {
    fn wake(self: Arc<Self>) {}
}
fn abandoned_close() {
    let session = zenoh::open(config()).wait().unwrap();
    let (release, callback_done) = blocked_timer(&session);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_time()
        .build()
        .unwrap();
    let _entered = runtime.enter();
    let waker = Waker::from(Arc::new(NoWake));
    let mut cx = Context::from_waker(&waker);
    let mut original = session.close().into_future();
    assert!(matches!(original.as_mut().poll(&mut cx), Poll::Pending));
    assert!(session.is_closed());
    // Drop the ORIGINAL close while its task join is pending. The retry no
    // longer owns the primitives and succeeds before our callback finishes.
    drop(original);
    session.close().wait().unwrap();
    assert!(matches!(
        callback_done.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    drop(session);
    assert!(matches!(
        callback_done.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    release.send(()).unwrap();
    callback_done.recv_timeout(WAIT).unwrap();
    println!("abandoned-close: is_closed + retry Ok + Session drop precede callback completion");
}

fn get_callback() {
    let session = zenoh::open(config()).wait().unwrap();
    let queryable = session
        .declare_queryable("wp600/get-owner")
        .callback(|query| {
            query.reply("wp600/get-owner", "ok").wait().unwrap();
        })
        .wait()
        .unwrap();
    let (entered_tx, entered) = mpsc::sync_channel(1);
    let (release, released) = mpsc::channel();
    let (done_tx, done) = mpsc::sync_channel(1);
    let released = Mutex::new(released);
    let worker = std::thread::spawn({
        let session = session.clone();
        move || {
            session
                .get("wp600/get-owner")
                .consolidation(ConsolidationMode::None)
                .callback(move |reply| {
                    assert!(reply.result().is_ok());
                    entered_tx.send(()).unwrap();
                    released.lock().unwrap().recv_timeout(WAIT).unwrap();
                })
                .wait()
                .unwrap();
            drop(session);
            done_tx.send(()).unwrap();
        }
    });
    entered.recv_timeout(WAIT).unwrap();
    session.close().wait().unwrap();
    assert!(session.is_closed());
    assert!(matches!(done.try_recv(), Err(mpsc::TryRecvError::Empty)));
    release.send(()).unwrap();
    done.recv_timeout(WAIT).unwrap();
    worker.join().unwrap();
    drop(queryable);
    drop(session);
    println!("get-callback: first close Ok precedes synchronous get-job/callback completion");
}

// This scoped worker models only a separate lifetime and one real ORIGINAL
// cleanup-slot account. One item is NOT an SDK byte or allocation ceiling.
// No public Runtime, engine input, registration, call or cleanup transfer is
// implemented. The worker is always joined here; final worker abandonment is
// deliberately unproved.
fn separate_owner() {
    let parent = Arc::new(Mutex::new(ResourceAccount::new(
        SlotIndex::new(0),
        Generation::new(1).unwrap(),
        ResourceKind::CleanupItemsMax,
        1,
    )));
    parent.lock().unwrap().try_reserve(1).unwrap().commit();
    let abandoned = Arc::new(AtomicBool::new(false));
    let (ready_tx, ready) = mpsc::sync_channel(1);
    let (retire_tx, retire) = mpsc::sync_channel(1);
    let (ack_tx, ack) = mpsc::sync_channel(1);
    let worker = std::thread::spawn({
        let parent = parent.clone();
        move || {
            let session = zenoh::open(config()).wait().unwrap();
            let (release, done) = blocked_timer(&session);
            ready_tx.send((release, done)).unwrap();
            retire.recv_timeout(WAIT).unwrap();
            // The ORIGINAL close is fully driven once, outside any engine
            // callback, and final SDK destruction happens before release.
            session.close().wait().unwrap();
            drop(session);
            assert!(parent.lock().unwrap().release_committed(1));
            ack_tx.send(()).unwrap();
        }
    });
    let (release, done) = ready.recv_timeout(WAIT).unwrap();
    struct RoleLease(Arc<AtomicBool>);
    impl Drop for RoleLease {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let lease = RoleLease(abandoned.clone());
    let before = COUNT.with(Cell::get);
    drop(lease);
    let after = COUNT.with(Cell::get);
    assert!(abandoned.load(Ordering::SeqCst));
    assert_eq!(before, after, "role lease Drop allocated");
    assert_eq!(parent.lock().unwrap().used(), 1);
    assert!(parent.lock().unwrap().try_reserve(1).is_none());
    retire_tx.send(()).unwrap();
    assert!(matches!(
        ack.recv_timeout(Duration::from_millis(100)),
        Err(mpsc::RecvTimeoutError::Timeout)
    ));
    assert_eq!(parent.lock().unwrap().used(), 1);
    release.send(()).unwrap();
    done.recv_timeout(WAIT).unwrap();
    ack.recv_timeout(WAIT).unwrap();
    worker.join().unwrap();
    assert_eq!(parent.lock().unwrap().used(), 0);
    println!(
        "separate-owner: finite lease Drop; original parent exhausted until SDK close/destruction"
    );
}

fn closed_session_drop(fail: bool) {
    let session = zenoh::open(config()).wait().unwrap();
    session.close().wait().unwrap();
    let before = COUNT.with(Cell::get);
    if fail {
        FAIL_NEXT.with(|flag| flag.set(true));
    }
    drop(session);
    let requests = COUNT.with(Cell::get) - before;
    assert!(requests > 0);
    println!("closed-session-drop: calling-thread allocation requests during Drop={requests}");
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("last-drop") => last_session_drop(),
        Some("abandoned-close") => abandoned_close(),
        Some("get-callback") => get_callback(),
        Some("separate-owner") => separate_owner(),
        Some("closed-drop") => closed_session_drop(false),
        Some("closed-drop-oom") => closed_session_drop(true),
        _ => panic!(
            "expected last-drop, abandoned-close, get-callback, separate-owner, closed-drop or closed-drop-oom"
        ),
    }
}
