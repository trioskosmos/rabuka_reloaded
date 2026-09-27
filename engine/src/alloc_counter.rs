use std::alloc::{GlobalAlloc, Layout};

// The counter is a wrapper, not a replacement: it measures whatever allocator
// the engine actually ships with, so that a profiling run and a production run
// agree about where the time goes. See the `SystemAlloc` resolution in lib.rs.
#[cfg(feature = "fast_alloc")]
use crate::pool_alloc::PoolAllocator as SystemAlloc;
#[cfg(not(feature = "fast_alloc"))]
use std::alloc::System as SystemAlloc;
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static BYTES_ALLOCATED: AtomicIsize = AtomicIsize::new(0);
static PEAK_BYTES: AtomicIsize = AtomicIsize::new(0);
static TOTAL_BYTES_ALLOCATED: AtomicUsize = AtomicUsize::new(0);

// Size-class histogram: count of allocs per power-of-2 bucket
// 0=1-7B, 1=8-15B, 2=16-31B, ..., 12=4KB+, 13=8KB+
static SIZE_BUCKETS: [AtomicUsize; 14] = [    AtomicUsize::new(0),
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

// ---------------------------------------------------------------------------
// Sampled self-timing of the allocator.
//
// Counting allocations tells you the engine makes a lot of them; it does NOT
// tell you whether the allocator is what is actually costing you time. This
// closes that gap: every 1-in-N allocation is timed, so the allocator's share
// of wall time can be reported directly. Sampling keeps the perturbation to
// ~1/N; the residual clock cost is measured separately and subtracted, since a
// single `Instant::now()` is itself tens of nanoseconds and would otherwise be
// indistinguishable from a small malloc.
// ---------------------------------------------------------------------------
static ALLOC_SELF_NS: AtomicIsize = AtomicIsize::new(0);
static ALLOC_SELF_DEALLOC_NS: AtomicIsize = AtomicIsize::new(0);
static ALLOC_SELF_SAMPLES: AtomicUsize = AtomicUsize::new(0);
static ALLOC_SELF_DEALLOC_SAMPLES: AtomicUsize = AtomicUsize::new(0);

const SELF_TIME_SAMPLE_MASK: usize = 63;

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
        let sampled = ALLOC_COUNT.load(Ordering::Relaxed) & SELF_TIME_SAMPLE_MASK == 0;
        let t0 = if sampled {
            Some(std::time::Instant::now())
        } else {
            None
        };
        let ptr = SystemAlloc.alloc(layout);
        if let Some(t0) = t0 {
            ALLOC_SELF_NS.fetch_add(t0.elapsed().as_nanos() as isize, Ordering::Relaxed);
            ALLOC_SELF_SAMPLES.fetch_add(1, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        BYTES_ALLOCATED.fetch_sub(layout.size() as isize, Ordering::Relaxed);
        let sampled = DEALLOC_COUNT.load(Ordering::Relaxed) & SELF_TIME_SAMPLE_MASK == 0;
        let t0 = if sampled {
            Some(std::time::Instant::now())
        } else {
            None
        };
        SystemAlloc.dealloc(ptr, layout);
        if let Some(t0) = t0 {
            ALLOC_SELF_DEALLOC_NS.fetch_add(t0.elapsed().as_nanos() as isize, Ordering::Relaxed);
            ALLOC_SELF_DEALLOC_SAMPLES.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Cost of one `Instant::now()` pair, i.e. the floor the sampled allocator
/// timings sit on. Subtract `samples * clock_ns_per_call` from the raw totals
/// to get the allocator's real cost.
pub fn clock_overhead_ns() -> f64 {
    const N: u32 = 20_000;
    let t0 = std::time::Instant::now();
    let mut acc = 0u64;
    for _ in 0..N {
        let a = std::time::Instant::now();
        let b = std::time::Instant::now();
        acc = acc.wrapping_add(b.duration_since(a).as_nanos() as u64);
    }
    let total = t0.elapsed().as_nanos() as f64;
    (acc as f64 / N as f64).min(total / N as f64)
}

/// (alloc_ns, dealloc_ns, alloc_samples, dealloc_samples) — raw sampled totals.
pub fn allocator_self_ns() -> (i64, i64, u64, u64) {
    (
        ALLOC_SELF_NS.load(Ordering::Relaxed) as i64,
        ALLOC_SELF_DEALLOC_NS.load(Ordering::Relaxed) as i64,
        ALLOC_SELF_SAMPLES.load(Ordering::Relaxed) as u64,
        ALLOC_SELF_DEALLOC_SAMPLES.load(Ordering::Relaxed) as u64,
    )
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
    buckets: [u64; 14],
}

fn snapshot() -> Snapshot {
    Snapshot {
        alloc_calls: ALLOC_COUNT.load(Ordering::Relaxed) as u64,
        dealloc_calls: DEALLOC_COUNT.load(Ordering::Relaxed) as u64,
        live_bytes: BYTES_ALLOCATED.load(Ordering::Relaxed) as i64,
        peak_bytes: PEAK_BYTES.load(Ordering::Relaxed) as i64,
        total_allocated: TOTAL_BYTES_ALLOCATED.load(Ordering::Relaxed) as u64,
        buckets: read_buckets(),
    }
}

/// Total allocation calls since process start.
///
/// Exposed so per-region instrumentation (see `timer`) can attribute
/// allocations to a call path. The engine is allocation-bound rather than
/// compute-bound, so a region that costs little wall time can still be the
/// one producing hundreds of allocations per action.
#[cfg(feature = "alloc_tracker")]
pub fn alloc_count() -> u64 {
    ALLOC_COUNT.load(Ordering::Relaxed) as u64
}

/// Allocation counts per power-of-2 size bucket since process start.
///
/// Exposed alongside [`alloc_count`] so per-region instrumentation can show not
/// just *how many* allocations a region makes but *what size*. Over half of
/// this engine's allocations are 15 bytes or smaller, and that statistic is
/// what identifies `to_string()`-on-a-tiny-value churn as distinct from real
/// data-structure work.
#[cfg(feature = "alloc_tracker")]
pub fn alloc_buckets() -> [u64; 14] {
    let mut b = [0u64; 14];
    for (i, bucket) in SIZE_BUCKETS.iter().enumerate() {
        b[i] = bucket.load(Ordering::Relaxed) as u64;
    }
    b
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
            // Size-class histogram (delta from baseline)
            eprintln!("  --- allocs by size ---");
            for i in 0..SIZE_BUCKETS.len() {
                let cnt = now.buckets[i].saturating_sub(self.baseline.buckets[i]);
                if cnt > 0 {
                    eprintln!("    {}: {} allocs", bucket_label(i), cnt);
                }
            }
            // The question "is the allocator actually what is slow?" needs the
            // allocator's own time, not just its call count.
            let clock_ns = clock_overhead_ns();
            let (a_ns, d_ns, a_s, d_s) = allocator_self_ns();
            let total_ns = elapsed.as_nanos() as f64;
            if a_s + d_s > 0 {
                let raw = (a_ns + d_ns) as f64;
                let overhead = clock_ns * (a_s + d_s) as f64;
                let corrected = (raw - overhead).max(0.0);
                // `corrected` covers only the 1-in-N samples actually timed, so
                // scale it back up to the whole run before comparing to elapsed.
                let sample_rate = (SELF_TIME_SAMPLE_MASK + 1) as f64;
                let extrapolated = corrected * sample_rate;
                eprintln!("  --- allocator self time (1-in-{} sampled) ---", SELF_TIME_SAMPLE_MASK + 1);
                eprintln!(
                    "    samples:          {} alloc + {} dealloc",
                    a_s, d_s
                );
                eprintln!("    clock overhead:   {:.1} ns/call", clock_ns);
                eprintln!(
                    "    raw sampled:      {:.2} ms  (of {:.2} ms elapsed)",
                    raw / 1e6,
                    total_ns / 1e6
                );
                eprintln!(
                    "    less clock:       {:.2} ms (sampled) ",
                    corrected / 1e6
                );
                eprintln!(
                    "    extrapolated:     {:.2} ms  = {:.1}% of elapsed  <-- allocator share",
                    extrapolated / 1e6,
                    if total_ns > 0.0 {
                        extrapolated / total_ns * 100.0
                    } else {
                        0.0
                    }
                );
                eprintln!(
                    "    ns per alloc+dealloc (corrected): {:.1}",
                    corrected / (a_s + d_s) as f64
                );
            }
        }
        if std::env::var("RABUKA_CPU_TRACK").is_ok() {
            eprintln!();
            eprintln!("=== CPU report: {} ms ===", elapsed.as_millis());
        }
    }
}
