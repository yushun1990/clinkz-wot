#![cfg(feature = "prepare")]
use clinkz_wot_core::host_accounts::*;
use clinkz_wot_foundation::*;
use clinkz_wot_planning::open_consumer::*;
use independent_open_consumer::{prepare::*, *};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    sync::{Arc, atomic::Ordering},
};
struct Allocator;
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
thread_local! { static FAIL_AFTER:Cell<Option<usize>>=const{Cell::new(None)}; }
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let fail = FAIL_AFTER
            .try_with(|n| match n.get() {
                Some(0) => {
                    n.set(None);
                    true
                }
                Some(left) => {
                    n.set(Some(left - 1));
                    false
                }
                None => false,
            })
            .unwrap_or(false);
        if fail {
            std::ptr::null_mut()
        } else {
            unsafe { System.alloc(layout) }
        }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, 1_000_000))
}
#[test]
fn allocation_refusal_before_start_and_inside_owned_cursor_releases_reserved_backing() {
    let thing=serde_json::from_value(serde_json::json!({"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"allocation","security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}},"properties":{"a":{"forms":[{"href":"linkedudp://127.0.0.1:39991/temperature"}]}}})).unwrap();
    for after in [0, 1] {
        let probe = Arc::new(Probe::default());
        let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
        let registry = ConsumerRegistry::try_new(vec![registration(1, probe.clone())])
            .unwrap_or_else(|_| panic!());
        let result = registry.prepare(
            &thing,
            &GatewayDefaultV1::LIMITS,
            bank.clone(),
            ConsumerSelector::default(),
            &mut work(),
            |job| {
                FAIL_AFTER.with(|n| n.set(Some(after)));
                job.step(&mut work(), false);
                FAIL_AFTER.with(|n| assert!(n.get().is_none(), "targeted allocation was reached"));
                assert!(job.is_finished());
            },
        );
        assert!(result.is_err());
        assert_eq!(probe.starts.load(Ordering::Relaxed), after);
        assert_eq!(probe.native.load(Ordering::Relaxed), 0);
        assert!(
            ResourceKind::ALL
                .iter()
                .all(|&k| bank.account(k).used() == 0)
        );
    }
}
