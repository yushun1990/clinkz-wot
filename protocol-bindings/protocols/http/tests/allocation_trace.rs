//! Allocation observations for the real public path. No allocator replacement
//! or null injection is attempted inside unqualified infallible SDK operations.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::{Cell, RefCell},
};
#[derive(Clone, Copy)]
struct Trace {
    armed: bool,
    count: usize,
    live: usize,
    peak: usize,
    largest: usize,
    blocks: [(usize, usize); 512],
}
const EMPTY: Trace = Trace {
    armed: false,
    count: 0,
    live: 0,
    peak: 0,
    largest: 0,
    blocks: [(0, 0); 512],
};
thread_local! { static TRACE: RefCell<Trace> = const { RefCell::new(EMPTY) }; }
thread_local! {
    // On this pinned public path, the first 4096-byte, align-1 allocation
    // after acceptance is the fallible body reservation, not SDK internals.
    static DENY_NEXT_BODY: Cell<bool> = const { Cell::new(false) };
    static BODY_REFUSALS: Cell<usize> = const { Cell::new(0) };
}
struct Allocator;
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() == 4096
            && layout.align() == 1
            && DENY_NEXT_BODY
                .try_with(|deny| deny.replace(false))
                .unwrap_or(false)
        {
            BODY_REFUSALS.with(|count| count.set(count.get() + 1));
            return std::ptr::null_mut();
        }
        let p = unsafe { System.alloc(layout) };
        let _ = TRACE.try_with(|t| {
            let mut t = t.borrow_mut();
            if t.armed && !p.is_null() {
                t.count += 1;
                t.live += layout.size();
                t.peak = t.peak.max(t.live);
                t.largest = t.largest.max(layout.size());
                let slot = t
                    .blocks
                    .iter_mut()
                    .find(|b| b.0 == 0)
                    .expect("trace capacity");
                *slot = (p as usize, layout.size());
            }
        });
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        let _ = TRACE.try_with(|t| {
            let mut t = t.borrow_mut();
            if let Some(i) = t.blocks.iter().position(|b| b.0 == p as usize) {
                assert_eq!(t.blocks[i].1, layout.size());
                t.live -= layout.size();
                t.blocks[i] = (0, 0);
            }
        });
        unsafe { System.dealloc(p, layout) }
    }
}
// Keep the application/peer implementation in one place. The normal tests are
// imported only in the real_peer integration target, not repeated here.
include!("support/application.rs");

#[test]
fn body_allocator_refusal_preserves_deployment_domain_and_allows_retry() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let peer = std::thread::spawn(move || {
        for attempt in 0..3 {
            let (mut socket, _) = listener.accept().unwrap();
            assert!(request(&mut socket).starts_with("GET /a?x=1 HTTP/1.1\r\n"));
            if attempt == 1 {
                // No Content-Length: reach the actual decoded-body policy limit.
                socket
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n")
                    .unwrap();
                socket.write_all(&[b'1'; 4097]).unwrap();
                socket.shutdown(std::net::Shutdown::Write).unwrap();
            } else {
                socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\n\r\n1").unwrap();
            }
            let mut byte = [0];
            assert_eq!(
                socket.read(&mut byte).unwrap(),
                0,
                "socket survived settlement"
            );
        }
    });
    recipe().block_on(async {
        let (runtime, handle, observer, parent) = engine(port);
        let mut failures = Vec::new();
        BODY_REFUSALS.with(|count| count.set(0));
        for domain in [
            ResourceFailureDomain::DeploymentFailure,
            ResourceFailureDomain::BindingPolicy,
        ] {
            let mut call = handle
                .read_property("a", InteractionOptions::new(), &mut work())
                .unwrap();
            DENY_NEXT_BODY
                .with(|deny| deny.set(domain == ResourceFailureDomain::DeploymentFailure));
            let result = take(&mut call).await;
            let still_armed = DENY_NEXT_BODY.with(|deny| deny.replace(false));
            assert!(!still_armed, "body reservation was not reached");
            let error = result.unwrap_err();
            assert!(matches!(error, CoreError::Binding(_)), "{error:?}");
            let (key, actual_domain, local, parent_owner) =
                error.context().resource_failure().unwrap();
            assert_eq!(actual_domain, domain, "{error:?}");
            assert_eq!((local, parent_owner), (None, None));
            assert_eq!(
                error.context().binding(),
                Some((BindingId::new(600), BindingGeneration::INITIAL))
            );
            assert!(error.context().plan().is_some());
            assert_eq!(error.context().phase(), ErrorPhase::Binding);
            assert_eq!(error.retry_class(), RetryClass::Never);
            assert_eq!(
                error.context().cause_code(),
                Some(if domain == ResourceFailureDomain::DeploymentFailure {
                    607
                } else {
                    608
                })
            );
            assert_eq!(
                error.context().redacted_cause(),
                Some(if domain == ResourceFailureDomain::DeploymentFailure {
                    "HTTP body backing unavailable"
                } else {
                    "HTTP body limit exceeded"
                })
            );
            failures.push((key, error.context().plan()));
            drop(call);
        }
        assert_eq!(BODY_REFUSALS.with(Cell::get), 1);
        assert_eq!(
            failures[0], failures[1],
            "policy identity or selected plan changed"
        );
        let mut retry = handle
            .read_property("a", InteractionOptions::new(), &mut work())
            .unwrap();
        let output = take(&mut retry).await.unwrap();
        assert_eq!(&*output.data().unwrap().body, b"1");
        assert_eq!(
            Some(output.metadata().binding_response().unwrap().plan_id()),
            failures[0].1
        );
        drop(output);
        drop(retry);
        drop(handle);
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    });
    peer.join().unwrap();
}

#[test]
fn public_path_allocation_observation_and_complete_release() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let peer = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        request(&mut socket);
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 4096\r\n\r\n").unwrap();
        socket.write_all(&[b'1'; 4096]).unwrap();
        let mut byte = [0];
        assert_eq!(socket.read(&mut byte).unwrap(), 0);
    });
    let io_runtime = recipe();
    let _ = std::thread::current();
    io_runtime.block_on(async {});
    // Runtime creation is outside the measurement. First-use reactor growth on
    // this thread is still included and reported, not assigned to engine rows.
    let (startup, accepted, terminal, parent_peak, accepted_layouts) = io_runtime.block_on(async {
        TRACE.with(|t| t.borrow_mut().armed = true);
        let (runtime, handle, observer, parent) = engine(port);
        let startup_snapshot = TRACE.with(|t| *t.borrow());
        let startup = startup_snapshot.live;
        let mut call = handle
            .read_property("a", InteractionOptions::new(), &mut work())
            .unwrap();
        let accepted_snapshot = TRACE.with(|t| *t.borrow());
        let accepted = accepted_snapshot.live;
        let accepted_layouts: [usize; 512] = std::array::from_fn(|i| {
            let block = accepted_snapshot.blocks[i];
            if block.0 != 0 && !startup_snapshot.blocks.iter().any(|old| old.0 == block.0) {
                block.1
            } else {
                0
            }
        });
        let output = take(&mut call).await.unwrap();
        assert_eq!(output.data().unwrap().content_type.capacity(), 16);
        let terminal = TRACE.with(|t| t.borrow().live);
        let parent_peak: u64 = parent.lock().unwrap().iter().map(|a| a.peak()).sum();
        drop(output);
        drop(call);
        drop(handle);
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
        (startup, accepted, terminal, parent_peak, accepted_layouts)
    });
    // Shared-runtime shutdown may initialize its thread-local parker; that
    // deployment allocation is outside this binding-path observation.
    TRACE.with(|t| t.borrow_mut().armed = false);
    drop(io_runtime);
    let measured = TRACE.with(|t| {
        let mut t = t.borrow_mut();
        t.armed = false;
        *t
    });
    assert_eq!(
        measured.live, 0,
        "tracked backing survived root and I/O runtime destruction"
    );
    println!(
        "startup={startup} accepted={accepted} terminal={terminal} requests={} peak={} largest={} parent_peak_sum={parent_peak} remaining={}",
        measured.count, measured.peak, measured.largest, measured.live
    );
    println!(
        "accepted live allocation sizes: {:?}",
        accepted_layouts
            .into_iter()
            .filter(|n| *n != 0)
            .collect::<Vec<_>>()
    );
    peer.join().unwrap();
}

#[test]
fn cancellation_observes_sdk_disposal_and_checks_allocation_free_engine_handoff() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (sent, received) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        request(&mut socket);
        sent.send(()).unwrap();
        let mut byte = [0];
        assert_eq!(socket.read(&mut byte).unwrap(), 0);
    });
    recipe().block_on(async {
        let (runtime, handle, observer, parent) = engine(port);
        let mut call = handle
            .read_property("a", InteractionOptions::new(), &mut work())
            .unwrap();
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            call.step(&mut Context::from_waker(Waker::noop()), &mut work());
            if received.try_recv().is_ok() {
                break;
            }
            assert!(Instant::now() < end);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        TRACE.with(|t| {
            let mut t = t.borrow_mut();
            *t = EMPTY;
            t.armed = true;
        });
        call.cancel().unwrap();
        assert_eq!(TRACE.with(|t| t.borrow().count), 0);
        call.step(&mut Context::from_waker(Waker::noop()), &mut work());
        let disposal = TRACE.with(|t| *t.borrow());
        let result = call.try_take(&mut work()).unwrap().unwrap();
        let requests = TRACE.with(|t| {
            let mut t = t.borrow_mut();
            t.armed = false;
            t.count
        });
        assert_eq!(requests, disposal.count, "engine result-taking allocated");
        println!("SDK cancellation: requests={} peak={} largest={} retained_before_reactor_turn={}; engine cancel/status and result-taking=0", disposal.count,disposal.peak,disposal.largest,disposal.live);
        assert!(matches!(result, Err(CoreError::Cancelled(_))));
        drop(call);
        drop(handle);
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    });
    peer.join().unwrap();
}
