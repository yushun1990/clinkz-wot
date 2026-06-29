use clinkz_wot_discovery::ThingDirectory;
use core::sync::atomic::Ordering;

use crate::ServientResult;

use super::Servient;

impl<D> Servient<D> {
    /// Performs one synchronous driving iteration (baseline §4).
    ///
    /// Polls each registered [`ServerBinding::poll_accept_sync`], dispatches
    /// inbound requests against the exposed Thing registry, and writes
    /// [`InboundResponse`] back through [`ServerBinding::send_response`].
    /// Each call processes at most one inbound request across all registered
    /// bindings, mirroring the stepwise semantics of the async
    /// [`poll_serve`](Self::poll_serve) path.
    ///
    /// On bare `no_std` MCU super-loops, this is the primary driving primitive —
    /// call it once per super-loop iteration alongside other work.
    ///
    /// The outer Servient lock is held only briefly to snapshot the binding list
    /// and to extract per-request dependencies (security/registry/broker). The
    /// user handler and `send_response` run **outside** the outer lock (mirrors
    /// the async take-out / await / return discipline).
    pub fn poll_serve_sync(&self) -> ServientResult<()>
    where
        D: ThingDirectory,
    {
        if self.shutdown.load(Ordering::Relaxed) {
            return Ok(());
        }
        self.poll_serve_sync_step().map(|_| ())
    }

    /// Infinite-loop wrapper around [`poll_serve_sync`](Self::poll_serve_sync)
    /// (baseline §4 / addendum §6.2).
    ///
    /// Intended for std host/cloud single-purpose runtimes that dedicate a
    /// thread to serving. On bare `no_std` MCU super-loops, use
    /// `poll_serve_sync` directly instead.
    #[cfg(feature = "std")]
    pub fn serve_sync(&self)
    where
        D: ThingDirectory,
    {
        let mut idle_streak = 0usize;
        loop {
            if self.shutdown.load(Ordering::Relaxed) {
                break;
            }
            match self.poll_serve_sync_step() {
                Ok(did_work) => {
                    if did_work {
                        idle_streak = 0;
                    } else {
                        idle_streak = idle_streak.saturating_add(1);
                        if idle_streak <= 8 {
                            std::thread::yield_now();
                        } else {
                            std::thread::sleep(std::time::Duration::from_millis(1));
                        }
                    }
                }
                Err(err) => {
                    idle_streak = 0;
                    log::error!("clinkz-wot serve_sync error: {}", err);
                }
            }
        }
    }

    /// Runs one synchronous driving step without holding the outer Servient
    /// lock across handler dispatch or `send_response`.
    fn poll_serve_sync_step(&self) -> ServientResult<bool> {
        // Clone the binding list (N Arc refcount bumps) — the bindings
        // snapshot lives in ServientShared, separate from the directory lock.
        // A *read* lock is sufficient: cloning the Vec<Arc<...>> never
        // mutates the source, and this is the per-iteration hot path of the
        // sync driving loop, so writers (expose/destroy of other Things)
        // must not block readers here.
        let bindings = self
            .shared
            .sync_server_bindings
            .with_read_recover(|snapshot| snapshot.clone());
        let binding_count = bindings.len();
        if binding_count == 0 {
            self.shared.sync_binding_cursor.store(0, Ordering::Relaxed);
            return Ok(false);
        }

        // The cursor is an atomic fairness hint. Relaxed ordering is
        // sufficient: the only reader/writer is this driving loop, and
        // `% binding_count` absorbs binding-list size changes without
        // coordination. No directory lock or driving-state lock is needed.
        let start_cursor = self.shared.sync_binding_cursor.load(Ordering::Relaxed);

        let start = start_cursor % binding_count;
        for offset in 0..binding_count {
            let index = (start + offset) % binding_count;
            if let Some(request) = bindings[index].poll_accept_sync() {
                // Dispatch + send_response run with no outer lock held.
                let response = self.dispatch_inbound(request);
                bindings[index].send_response(response);
                self.shared
                    .sync_binding_cursor
                    .store((index + 1) % binding_count, Ordering::Relaxed);
                return Ok(true);
            }
        }

        Ok(false)
    }
}
