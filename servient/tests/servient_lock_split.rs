//! Concurrency-property tests for the post-lock-split Servient.
//!
//! These tests verify the central property of the refactor: a slow
//! `register_thing` call inside `expose()` runs **without** holding any lock
//! that would block directory operations or the driving loop.
//!
//! `Servient` is not yet `Send` (handler trait-object bounds are tracked as a
//! follow-up in PLAN.md), so we cannot spawn threads. Instead, each test
//! performs a nested call from inside a `ServerBinding` callback: the nested
//! call would deadlock (or panic on `RefCell` re-borrow) if the surrounding
//! operation still held the relevant lock. This is a stronger test than
//! thread-based timing, because it deterministically proves lock
//! non-ownership rather than racing on scheduler jitter.

#![allow(clippy::arc_with_non_send_sync, clippy::type_complexity)]

use std::sync::{Arc, Mutex};

use clinkz_wot_core::{
    ServerBinding,
    inbound::{InboundRequest, InboundResponse},
};
use clinkz_wot_discovery::DirectoryQuery;
use clinkz_wot_servient::Servient;
use clinkz_wot_td::{security_scheme::SecurityScheme, thing::Thing};

type RegisterCallback = Box<dyn Fn(&str) -> Result<(), String>>;

/// `ServerBinding` whose `register_thing` runs a caller-supplied closure
/// before returning. The closure lets the test perform a re-entrant call
/// (e.g. a directory query) to prove no lock is held around `register_thing`.
struct ReentrantServerBinding {
    /// Closure executed inside `register_thing`, *before* the route is
    /// recorded as registered. Returning `Err` aborts the registration.
    ///
    /// The closure is `!Send` because it captures a `Servient` clone, and
    /// `Servient` is intentionally `!Send` until the tracked trait-object
    /// bounds follow-up lands (PLAN.md). The binding is used single-threaded
    /// in these tests.
    on_register: Mutex<Option<RegisterCallback>>,
    registered: Mutex<Vec<String>>,
}

impl ReentrantServerBinding {
    fn new() -> Self {
        Self {
            on_register: Mutex::new(None),
            registered: Mutex::new(Vec::new()),
        }
    }

    fn set_on_register<F>(&self, f: F)
    where
        F: Fn(&str) -> Result<(), String> + 'static,
    {
        *self.on_register.lock().unwrap() = Some(Box::new(f));
    }
}

impl ServerBinding for ReentrantServerBinding {
    fn poll_accept_sync(&self) -> Option<InboundRequest> {
        None
    }
    fn send_response(&self, _response: InboundResponse) {}
    fn register_thing(&self, thing_id: &str, _td: &Thing) -> Result<(), String> {
        if let Some(callback) = self.on_register.lock().unwrap().as_ref() {
            callback(thing_id)?;
        }
        self.registered.lock().unwrap().push(thing_id.to_owned());
        Ok(())
    }
    fn unregister_thing(&self, _thing_id: &str) {}
}

fn thing(id: &str, title: &str) -> Thing {
    Thing::builder(title)
        .id(id)
        .security_definition("nosec", SecurityScheme::nosec())
        .security_name("nosec")
        .build()
        .expect("valid Thing Description")
}

/// While inside `expose()` → `register_thing()`, a directory query must not
/// deadlock. Before the refactor, both shared `MapLock<ServientState>`, so
/// the nested `query` call would either:
/// - deadlock on `std::sync::Mutex` (the same thread re-acquiring a
///   non-reentrant `Mutex`), or
/// - panic on the `no_std` `RefCell` backend ("already mutably borrowed").
///
/// Post-refactor, `register_thing` holds no directory lock, so the nested
/// query succeeds.
#[test]
fn directory_query_is_reachable_from_inside_register_thing() {
    let servient = Servient::new();
    let binding = Arc::new(ReentrantServerBinding::new());

    // Pre-populate the directory with a sentinel so the nested query has
    // something to find.
    servient
        .register(thing("urn:thing:sentinel", "Sentinel"))
        .expect("register sentinel");

    // The callback runs *inside* register_thing. If expose still held the
    // directory lock, this query would deadlock / panic.
    let servient_for_callback = servient.clone();
    binding.set_on_register(move |_thing_id| {
        let page = servient_for_callback.query(DirectoryQuery::title("Sentinel"));
        assert_eq!(
            page.total, 1,
            "nested directory query must see the sentinel entry"
        );
        assert_eq!(page.entries[0].id, "urn:thing:sentinel");
        Ok(())
    });

    servient
        .register_server_binding(binding)
        .expect("register reentrant binding");

    // This call enters register_thing, which runs the nested query. If the
    // refactor regresses, this single call deadlocks (std) or panics (no_std).
    servient
        .expose(thing("urn:thing:probe", "Probe"))
        .expect("expose with nested directory query succeeds");
}

/// While inside `expose()` → `register_thing()`, the sync driving loop must
/// be invokable. Before the refactor, the cursor lived in
/// `MapLock<ServientState>`; calling `poll_serve_sync` from inside
/// `register_thing` would have re-acquired the same `Mutex` and deadlocked.
#[test]
fn driving_loop_is_runnable_from_inside_register_thing() {
    let servient = Servient::new();
    let binding = Arc::new(ReentrantServerBinding::new());

    let servient_for_callback = servient.clone();
    binding.set_on_register(move |_thing_id| {
        // poll_serve_sync reads/writes the cursor. If the cursor still lived
        // behind the directory lock, this call would deadlock or panic.
        servient_for_callback
            .poll_serve_sync()
            .expect("nested poll_serve_sync must not deadlock");
        Ok(())
    });

    servient
        .register_server_binding(binding)
        .expect("register reentrant binding");

    servient
        .expose(thing("urn:thing:probe", "Probe"))
        .expect("expose with nested poll_serve_sync succeeds");
}

/// While inside `expose()` → `register_thing()`, a second directory mutation
/// (`register`) must succeed. This proves directory write paths are also
/// decoupled from route registration.
#[test]
fn directory_write_is_reachable_from_inside_register_thing() {
    let servient = Servient::new();
    let binding = Arc::new(ReentrantServerBinding::new());

    let servient_for_callback = servient.clone();
    binding.set_on_register(move |_thing_id| {
        servient_for_callback
            .register(thing("urn:thing:from-callback", "From Callback"))
            .expect("nested directory register must succeed");
        Ok(())
    });

    servient
        .register_server_binding(binding)
        .expect("register reentrant binding");

    servient
        .expose(thing("urn:thing:probe", "Probe"))
        .expect("expose with nested directory write succeeds");

    // The nested register call must have actually written to the directory.
    let page = servient.query(DirectoryQuery::title("From Callback"));
    assert_eq!(page.total, 1);
    assert_eq!(page.entries[0].id, "urn:thing:from-callback");
}

/// While inside `expose()` → `register_thing()`, a directory `list` must
/// succeed — proving the directory lock is held per-operation, not across
/// the whole expose sequence.
#[test]
fn directory_list_is_reachable_from_inside_register_thing() {
    let servient = Servient::new();
    let binding = Arc::new(ReentrantServerBinding::new());

    for i in 0..3 {
        servient
            .register(thing(&format!("urn:thing:{i}"), &format!("Thing {i}")))
            .expect("pre-populate directory");
    }

    let servient_for_callback = servient.clone();
    binding.set_on_register(move |_thing_id| {
        let page = servient_for_callback.list();
        // The expose's directory write has NOT happened yet (we're inside
        // register_thing, before the directory publish step), so we still
        // see the three pre-populated entries.
        assert_eq!(page.total, 3);
        Ok(())
    });

    servient
        .register_server_binding(binding)
        .expect("register reentrant binding");

    servient
        .expose(thing("urn:thing:probe", "Probe"))
        .expect("expose with nested list succeeds");

    // After expose completes, the directory has the probe entry too.
    let page = servient.list();
    assert_eq!(page.total, 4);
}
