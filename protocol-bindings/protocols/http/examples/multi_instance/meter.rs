//! Process-wide requested heap traffic, plus Linux RSS and glibc observations.
//! The peer runs in another process; the measured runtime has one thread.
use serde_json::{Value, json};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicU64, Ordering::Relaxed},
    time::Instant,
};

pub struct Allocator;
static LIVE: AtomicU64 = AtomicU64::new(0);
static PEAK: AtomicU64 = AtomicU64::new(0);
static ALLOCS: AtomicU64 = AtomicU64::new(0);
static REQUESTED: AtomicU64 = AtomicU64::new(0);
fn acquire(bytes: usize) {
    let live = LIVE.fetch_add(bytes as u64, Relaxed) + bytes as u64;
    PEAK.fetch_max(live, Relaxed);
    ALLOCS.fetch_add(1, Relaxed);
    REQUESTED.fetch_add(bytes as u64, Relaxed);
}
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            acquire(layout.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(layout) };
        if !p.is_null() {
            acquire(layout.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe {
            System.dealloc(p, layout);
        }
        LIVE.fetch_sub(layout.size() as u64, Relaxed);
    }
    unsafe fn realloc(&self, p: *mut u8, old: Layout, size: usize) -> *mut u8 {
        let p = unsafe { System.realloc(p, old, size) };
        if !p.is_null() {
            LIVE.fetch_sub(old.size() as u64, Relaxed);
            acquire(size);
        }
        p
    }
}
#[derive(Clone, Copy)]
pub struct Reading {
    live: u64,
    allocations: u64,
    requested: u64,
}
pub fn read() -> Reading {
    Reading {
        live: LIVE.load(Relaxed),
        allocations: ALLOCS.load(Relaxed),
        requested: REQUESTED.load(Relaxed),
    }
}
#[derive(Default)]
pub struct Cost {
    pub ns: u128,
    pub allocations: u64,
    pub requested: u64,
    pub live: i64,
}
impl Cost {
    pub fn add(&mut self, start: Reading, elapsed: u128) {
        let end = read();
        self.ns += elapsed;
        self.allocations += end.allocations - start.allocations;
        self.requested += end.requested - start.requested;
        self.live += end.live as i64 - start.live as i64;
    }
    pub fn json(&self) -> Value {
        json!({"ns": self.ns, "allocation_calls": self.allocations, "requested_bytes": self.requested, "net_live_bytes": self.live})
    }
}
pub struct Phase {
    start: Reading,
    time: Instant,
}
impl Phase {
    pub fn begin() -> Self {
        let start = read();
        PEAK.store(start.live, Relaxed);
        Self {
            start,
            time: Instant::now(),
        }
    }
    pub fn finish(self, name: &str) -> Value {
        let ns = self.time.elapsed().as_nanos();
        let end = read();
        let peak = PEAK.load(Relaxed);
        let mut cost = Cost::default();
        // No allocations between the end snapshot and this calculation.
        cost.ns = ns;
        cost.allocations = end.allocations - self.start.allocations;
        cost.requested = end.requested - self.start.requested;
        cost.live = end.live as i64 - self.start.live as i64;
        let mut out = cost.json();
        out["phase"] = json!(name);
        out["heap_live_bytes"] = json!(end.live);
        out["heap_peak_bytes"] = json!(peak);
        out["heap_peak_above_start_bytes"] = json!(peak.saturating_sub(self.start.live));
        out["os"] = os_memory();
        out
    }
}
fn os_memory() -> Value {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let field = |key: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .and_then(|s| s.split_whitespace().next()?.parse::<u64>().ok())
            .map(|kb| kb * 1024)
    };
    // mallinfo2 measures allocator arena bookkeeping, not just requested Layout
    // bytes. RSS additionally includes stacks, executable pages and libraries.
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    let allocator = {
        #[repr(C)]
        struct Mallinfo {
            arena: usize,
            ordblks: usize,
            smblks: usize,
            hblks: usize,
            hblkhd: usize,
            usmblks: usize,
            fsmblks: usize,
            uordblks: usize,
            fordblks: usize,
            keepcost: usize,
        }
        unsafe extern "C" {
            fn mallinfo2() -> Mallinfo;
        }
        let m = unsafe { mallinfo2() };
        json!({"in_use_bytes": m.uordblks + m.hblkhd, "mapped_bytes": m.arena + m.hblkhd})
    };
    #[cfg(not(all(target_os = "linux", target_env = "gnu")))]
    let allocator = Value::Null;
    let open_fds = std::fs::read_dir("/proc/self/fd")
        .ok()
        .map(|entries| entries.count());
    json!({"rss_bytes": field("VmRSS:"), "rss_high_water_bytes": field("VmHWM:"), "glibc": allocator, "open_fds": open_fds})
}
