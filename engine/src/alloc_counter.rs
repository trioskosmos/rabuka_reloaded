use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};

#[cfg(feature = "arena_allocator")]
use crate::arena;

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static BYTES_ALLOCATED: AtomicIsize = AtomicIsize::new(0);
static PEAK_BYTES: AtomicIsize = AtomicIsize::new(0);
static TOTAL_BYTES_ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static ARENA_BUMPS: AtomicUsize = AtomicUsize::new(0);

// Size-class histogram: count of allocs per power-of-2 bucket
// 0=1-7B, 1=8-15B, 2=16-31B, ..., 12=4KB+, 13=8KB+
static SIZE_BUCKETS: [AtomicUsize; 14] = [
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
];

fn size_bucket(size: usize) -> usize {
    let bits = (usize::BITS - size.leading_zeros()) as usize; // floor(log2(size)) + 1
    if bits <= 3 {
        return 0;
    } // 1-7 bytes
    let idx = bits - 3; // 8B → bucket 1, 16B → 2, 32B → 3, ...
    idx.min(SIZE_BUCKETS.len() - 1)
}

pub struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        let bucket = size_bucket(layout.size());
        SIZE_BUCKETS[bucket].fetch_add(1, Ordering::Relaxed);
        let size = layout.size() as isize;
        let prev = BYTES_ALLOCATED.fetch_add(size, Ordering::Relaxed);
        TOTAL_BYTES_ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        let current = prev + size;
        let mut peak = PEAK_BYTES.load(Ordering::Relaxed);
        while current > peak {
            match PEAK_BYTES.compare_exchange(peak, current, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(p) => peak = p,
            }
        }
        #[cfg(feature = "arena_allocator")]
        {
            if let Some(ptr) = arena::arena_alloc(layout) {
                ARENA_BUMPS.fetch_add(1, Ordering::Relaxed);
                return ptr;
            }
        }
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        #[cfg(feature = "arena_allocator")]
        {
            if arena::arena_contains_ptr(ptr) {
                return;
            }
        }
        DEALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        BYTES_ALLOCATED.fetch_sub(layout.size() as isize, Ordering::Relaxed);
        System.dealloc(ptr, layout)
    }
}

fn read_buckets() -> [u64; 14] {
    let mut b = [0u64; 14];
    for (i, bucket) in SIZE_BUCKETS.iter().enumerate() {
        b[i] = bucket.load(Ordering::Relaxed) as u64;
    }
    b
}

fn bucket_label(i: usize) -> &'static str {
    match i {
        0 => "1-7B   ",
        1 => "8-15B  ",
        2 => "16-31B ",
        3 => "32-63B ",
        4 => "64-127B",
        5 => "128-255",
        6 => "256-511",
        7 => "512-1K ",
        8 => "1K-2K  ",
        9 => "2K-4K  ",
        10 => "4K-8K  ",
        11 => "8K-16K ",
        12 => "16K-32K",
        13 => "32K+   ",
        _ => "?",
    }
}

#[derive(Clone, Copy)]
struct Snapshot {
    alloc_calls: u64,
    dealloc_calls: u64,
    live_bytes: i64,
    peak_bytes: i64,
    total_allocated: u64,
    arena_bumps: u64,
    buckets: [u64; 14],
}

fn snapshot() -> Snapshot {
    Snapshot {
        alloc_calls: ALLOC_COUNT.load(Ordering::Relaxed) as u64,
        dealloc_calls: DEALLOC_COUNT.load(Ordering::Relaxed) as u64,
        live_bytes: BYTES_ALLOCATED.load(Ordering::Relaxed) as i64,
        peak_bytes: PEAK_BYTES.load(Ordering::Relaxed) as i64,
        total_allocated: TOTAL_BYTES_ALLOCATED.load(Ordering::Relaxed) as u64,
        arena_bumps: ARENA_BUMPS.load(Ordering::Relaxed) as u64,
        buckets: read_buckets(),
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
            let ab = now.arena_bumps.saturating_sub(self.baseline.arena_bumps);
            eprintln!("  arena bumps:       {} (skipped system alloc)", ab);
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
            // Size-class histogram (delta from baseline)
            eprintln!("  --- allocs by size ---");
            for i in 0..SIZE_BUCKETS.len() {
                let cnt = now.buckets[i].saturating_sub(self.baseline.buckets[i]);
                if cnt > 0 {
                    eprintln!("    {}: {} allocs", bucket_label(i), cnt);
                }
            }
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
