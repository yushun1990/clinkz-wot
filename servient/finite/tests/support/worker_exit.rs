use std::{
    cell::Cell,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    sync::mpsc,
    thread,
};

struct WorkerOwner(Cell<Option<Box<dyn FnOnce()>>>);
impl Drop for WorkerOwner {
    fn drop(&mut self) {
        if let Some(cleanup) = self.0.take() {
            cleanup();
        }
    }
}

/// Create the fixture's services first, then initialize the worker-local
/// owner before its first poll. Report destructor failures on the test thread.
pub(super) fn check<S: 'static>(
    setup: impl FnOnce() -> S + Send + 'static,
    park: impl FnOnce(&S) + Send + 'static,
    cleanup: impl FnOnce(S) + Send + 'static,
) {
    thread_local! { static OWNER: WorkerOwner = const { WorkerOwner(Cell::new(None)) }; }
    let (send, recv) = mpsc::channel();
    thread::spawn(move || {
        let state = setup();
        OWNER.with(|owner| {
            park(&state);
            owner.0.set(Some(Box::new(move || {
                send.send(catch_unwind(AssertUnwindSafe(|| cleanup(state))))
                    .unwrap();
            })));
        });
    })
    .join()
    .unwrap();
    if let Err(panic) = recv.try_recv().expect("worker-local cleanup did not run") {
        resume_unwind(panic);
    }
}
