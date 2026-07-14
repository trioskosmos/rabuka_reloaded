use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

static ALLOC_COUNT: AtomicU64 = AtomicU64::new(0);
static DEALLOC_COUNT: AtomicU64 = AtomicU64::new(0);
static BYTES_ALLOCATED: AtomicI64 = AtomicI64::new(0);
static PEAK_BYTES: AtomicI64 = AtomicI64::new(0);
static TOTAL_BYTES_ALLOCATED: AtomicU64 = AtomicU64::new(0);

pub struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        let size = layout.size() as i64;
        let prev = BYTES_ALLOCATED.fetch_add(size, Ordering::Relaxed);
        TOTAL_BYTES_ALLOCATED.fetch_add(size as u64, Ordering::Relaxed);
        let current = prev + size;
        let mut peak = PEAK_BYTES.load(Ordering::Relaxed);
        while current > peak {
            match PEAK_BYTES.compare_exchange(peak, current, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(p) => peak = p,
            }
        }
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        BYTES_ALLOCATED.fetch_sub(layout.size() as i64, Ordering::Relaxed);
        System.dealloc(ptr, layout)
    }
}

#[derive(Clone, Copy)]
struct Snapshot {
    alloc_calls: u64,
    dealloc_calls: u64,
    live_bytes: i64,
    peak_bytes: i64,
    total_allocated: u64,
}

fn snapshot() -> Snapshot {
    Snapshot {
        alloc_calls: ALLOC_COUNT.load(Ordering::Relaxed),
        dealloc_calls: DEALLOC_COUNT.load(Ordering::Relaxed),
        live_bytes: BYTES_ALLOCATED.load(Ordering::Relaxed),
        peak_bytes: PEAK_BYTES.load(Ordering::Relaxed),
        total_allocated: TOTAL_BYTES_ALLOCATED.load(Ordering::Relaxed),
    }
}

/// Start tracking allocations from this point.
/// Returns a Guard that prints the delta on drop.
pub fn start() -> Option<AllocGuard> {
    if std::env::var("RABUKA_ALLOC_TRACK").is_err() && std::env::var("RABUKA_CPU_TRACK").is_err() {
        return None;
    }
    let guard = AllocGuard {
        start: std::time::Instant::now(),
        baseline: snapshot(),
    };
    Some(guard)
}

pub struct AllocGuard {
    start: std::time::Instant,
    baseline: Snapshot,
}

impl Drop for AllocGuard {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        let now = snapshot();
        let d = now.alloc_calls.saturating_sub(self.baseline.alloc_calls);
        let dd = now
            .dealloc_calls
            .saturating_sub(self.baseline.dealloc_calls);
        let live = now.live_bytes - self.baseline.live_bytes;
        let peak = now.peak_bytes - self.baseline.peak_bytes;
        let total = now
            .total_allocated
            .saturating_sub(self.baseline.total_allocated);

        if std::env::var("RABUKA_ALLOC_TRACK").is_ok() {
            eprintln!();
            eprintln!("=== Allocator report ({} ms) ===", elapsed.as_millis());
            eprintln!("  alloc calls:       {}", d);
            eprintln!("  dealloc calls:     {}", dd);
            eprintln!("  net live allocs:   {}", d.saturating_sub(dd));
            if live >= 0 {
                eprintln!("  live bytes:        {} B  ({} KB)", live, live / 1024);
            } else {
                eprintln!("  live bytes:        -{} B  (freed during test)", -live);
            }
            eprintln!(
                "  peak bytes:        {} B  ({} KB)",
                peak.max(0),
                peak.max(0) / 1024
            );
            eprintln!(
                "  lifetime peak:     {} B  ({} KB)",
                now.peak_bytes,
                now.peak_bytes / 1024
            );
            eprintln!("  total allocated:   {} B  ({} KB)", total, total / 1024);
        }
        if std::env::var("RABUKA_CPU_TRACK").is_ok() {
            eprintln!();
            eprintln!("=== CPU report: {} ms ===", elapsed.as_millis());
        }
    }
}

/// Print lifetime stats (call at program exit).
pub fn print_lifetime() {
    let now = snapshot();
    eprintln!();
    eprintln!("=== Allocator lifetime totals ===");
    eprintln!("  alloc calls:       {}", now.alloc_calls);
    eprintln!("  dealloc calls:     {}", now.dealloc_calls);
    eprintln!("  live bytes:        {} B", now.live_bytes.max(0));
    eprintln!(
        "  peak bytes (lifetime):  {} B  ({} KB)",
        now.peak_bytes,
        now.peak_bytes / 1024
    );
    eprintln!(
        "  total allocated:   {} B  ({} KB)",
        now.total_allocated,
        now.total_allocated / 1024
    );
}
