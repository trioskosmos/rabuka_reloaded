use crate::HashMap;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use std::cell::RefCell;
use std::sync::Mutex;
#[cfg(feature = "profiling")]
use std::time::Instant;

type TimerKey = Vec<&'static str>;
type TimerValue = (u64, u128);
type TimerMap = HashMap<TimerKey, TimerValue>;

static TIMERS: Mutex<Option<TimerMap>> = Mutex::new(None);

/// Allocations attributed to each call path. Same key space as `TIMERS`, so a
/// row can be read as "this region cost N µs AND made M heap allocations" —
/// which is how an allocation-bound region is told apart from a compute-bound
/// one. Only meaningful at `--jobs 1` for the same reason timings are.
#[cfg(feature = "alloc_tracker")]
static ALLOCS: Mutex<Option<HashMap<TimerKey, u64>>> = Mutex::new(None);

#[cfg(feature = "alloc_tracker")]
fn get_alloc_timers() -> std::sync::MutexGuard<'static, Option<HashMap<TimerKey, u64>>> {
    let mut guard = ALLOCS.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::default());
    }
    guard
}

/// Per-call-path size-class breakdown, indexed by the same bucket order as
/// `alloc_counter::alloc_buckets` (0 = 1-7B, 1 = 8-15B, 2 = 16-31B, ...).
#[cfg(feature = "alloc_tracker")]
type AllocBucketMap = HashMap<TimerKey, [u64; 14]>;

#[cfg(feature = "alloc_tracker")]
static ALLOC_BUCKETS: Mutex<Option<AllocBucketMap>> = Mutex::new(None);

#[cfg(feature = "alloc_tracker")]
fn get_alloc_buckets() -> std::sync::MutexGuard<'static, Option<AllocBucketMap>> {
    let mut guard = ALLOC_BUCKETS.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::default());
    }
    guard
}

// The label stack MUST be per-thread. With one shared Mutex<Vec<&str>>, two
// threads interleave their push/pop so each Drop reconstructs a call path
// neither thread actually took: the key set becomes ~combinatorial, and the
// `TIMERS` map grows without bound (measured: 2 workers -> 3.9 GB and an OOM
// abort; 12 workers -> instant death). The map itself stays global and
// aggregated, which is bounded because per-thread paths are the real ones.
// Same discipline as ability/log.rs VERDICT_BUFFER.
thread_local! {
    static CALL_STACK: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

fn get_timers() -> std::sync::MutexGuard<'static, Option<TimerMap>> {
    let mut guard = TIMERS.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::default());
    }
    guard
}

// ---------------------------------------------------------------------------
// Self time.
//
// The inclusive table cannot be converted into a self-time table after the
// fact by subtracting child rows. The engine recurses (`effect::gates ->
// effect::gates`), and a nested subtree whose intermediate parent never
// recorded a row of its own has no row to subtract from, so the post-hoc
// arithmetic double-counts — measured at 238% of the top-level total. Guessing
// at where time goes from a table that does not add up is how you end up
// optimizing the wrong function, so the exclusive number is recorded here
// instead, where it is exact.
//
// Each timer accumulates, on a stack parallel to the call stack, the elapsed
// time of the regions that finished while it was on top. A region's self time
// is its own elapsed minus that sum. Children always drop before their parent,
// so by the time a parent is popped its children have already reported into it.
// ---------------------------------------------------------------------------
type SelfTimerMap = HashMap<Vec<&'static str>, (u64, u128)>;

static SELF_TIMERS: Mutex<Option<SelfTimerMap>> = Mutex::new(None);

fn get_self_timers() -> std::sync::MutexGuard<'static, Option<SelfTimerMap>> {
    let mut guard = SELF_TIMERS.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::default());
    }
    guard
}

#[cfg(feature = "profiling")]
thread_local! {
    /// Nanoseconds spent in regions that completed while this entry was on
    /// top of the stack. Indexed in lockstep with `CALL_STACK`.
    static CHILD_NS: std::cell::RefCell<Vec<u128>> = const { std::cell::RefCell::new(Vec::new()) };
}

pub struct Timer {
    label: &'static str,
    #[cfg(feature = "profiling")]
    start: Instant,
    /// Allocation count when this region was entered. Under
    /// `--features alloc_tracker` this attributes heap allocations to the
    /// same call path as the timings, which is how you tell a region that is
    /// genuinely compute-bound from one that is merely allocating.
    #[cfg(all(feature = "profiling", feature = "alloc_tracker"))]
    allocs_at_start: u64,
    /// Size-bucket counts when the region was entered, so the report can show
    /// *what size* of allocation a region makes, not just how many.
    #[cfg(all(feature = "profiling", feature = "alloc_tracker"))]
    buckets_at_start: [u64; 14],
}

impl Timer {
    pub fn start(label: &'static str) -> Self {
        #[cfg(not(feature = "profiling"))]
        {
            // Profiling off: no clock read, no stack push, no mutex. The
            // struct is zero-sized-ish and Drop is a no-op — hot paths pay
            // nothing for Timer construction.
            Timer { label }
        }
        #[cfg(feature = "profiling")]
        {
            CALL_STACK.with(|stack| stack.borrow_mut().push(label));
            CHILD_NS.with(|c| c.borrow_mut().push(0));
            #[cfg(feature = "alloc_tracker")]
            let allocs_at_start = crate::alloc_counter::alloc_count();
            #[cfg(feature = "alloc_tracker")]
            let buckets_at_start = crate::alloc_counter::alloc_buckets();
            Timer {
                label,
                start: Instant::now(),
                #[cfg(feature = "alloc_tracker")]
                allocs_at_start,
                #[cfg(feature = "alloc_tracker")]
                buckets_at_start,
            }
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        #[cfg(not(feature = "profiling"))]
        {
            let _ = self.label;
        }
        #[cfg(feature = "profiling")]
        {
            let elapsed = self.start.elapsed().as_nanos();

            // Reconstruct the full call path (entire stack at this moment)
            let call_path: Vec<&'static str> = CALL_STACK.with(|stack| {
                let stack = stack.borrow();
                // Check that we're at the top of the stack
                if stack.last() == Some(&self.label) {
                    stack.clone()
                } else {
                    vec![self.label]
                }
            });

            // Remove ourselves from the call stack
            CALL_STACK.with(|stack| {
                let mut stack = stack.borrow_mut();
                if stack.last() == Some(&self.label) {
                    stack.pop();
                }
            });

            // Self time: our elapsed minus whatever our children spent while
            // we were on top. Children always drop before their parent, so this
            // sum is complete by the time we get here. Our own full elapsed
            // then rolls up into our parent, which is what makes the parent's
            // own subtraction correct too.
            let child_ns = CHILD_NS.with(|c| {
                let mut c = c.borrow_mut();
                let child_ns = c.pop().unwrap_or(0);
                if let Some(parent) = c.last_mut() {
                    *parent += elapsed;
                }
                child_ns
            });

            // Record the time against the full call path
            if !call_path.is_empty() {
                #[cfg(feature = "alloc_tracker")]
                let allocs =
                    crate::alloc_counter::alloc_count().saturating_sub(self.allocs_at_start);
                let mut guard = get_timers();
                if let Some(ref mut map) = *guard {
                    let entry = map.entry(call_path.clone()).or_insert((0, 0));
                    entry.0 += 1;
                    entry.1 += elapsed;
                }
                {
                    let self_ns = elapsed.saturating_sub(child_ns);
                    let mut guard = get_self_timers();
                    if let Some(ref mut map) = *guard {
                        let entry = map.entry(call_path.clone()).or_insert((0, 0));
                        entry.0 += 1;
                        entry.1 += self_ns;
                    }
                }
                #[cfg(feature = "alloc_tracker")]
                {
                    let mut guard = get_alloc_timers();
                    if let Some(ref mut map) = *guard {
                        *map.entry(call_path.clone()).or_insert(0) += allocs;
                    }
                }
                // Size-class breakdown per call path. Buckets 0/1/2 are
                // 1-7B / 8-15B / 16-31B, i.e. the tiny churn that dominates
                // this engine. Reporting them per region is what turns
                // "this region allocates a lot" into "this region calls
                // to_string() on small values".
                #[cfg(feature = "alloc_tracker")]
                {
                    let now = crate::alloc_counter::alloc_buckets();
                    let mut deltas = [0u64; 14];
                    for i in 0..14 {
                        deltas[i] = now[i].saturating_sub(self.buckets_at_start[i]);
                    }
                    let mut guard = get_alloc_buckets();
                    if let Some(ref mut map) = *guard {
                        let entry = map.entry(call_path).or_insert([0u64; 14]);
                        for i in 0..14 {
                            entry[i] += deltas[i];
                        }
                    }
                }
            }
        }
    }
}

pub fn print_results() {
    let guard = get_timers();
    if let Some(ref map) = *guard {
        let mut results: Vec<_> = map.iter().collect();
        results.sort_by_key(|a| std::cmp::Reverse(a.1 .1));
        // The console truncates long call paths, which hides exactly the deep
        // rows you need. Always write the complete, untruncated report to a
        // file as well.
        write_full_report(&results);
        eprintln!("\n=== Timing Results (sorted by total time) === [full report: timer_report.txt]");
        eprintln!(
            "{:<90} {:>10} {:>15} {:>15} {:>15}",
            "Call Path", "Calls", "Total (ms)", "Avg (µs)", "% of rows"
        );
        eprintln!("{}", "-".repeat(150));
        eprintln!(
            "NOTE: every row is INCLUSIVE time, and a nested path is also counted \
             inside its parent's row,\n      so this column sums to ~300% and is not a \
             share of wall time. For shares, use the rows with no\n      parent \
             (depth-1 paths) and divide by the sum of THOSE. 'Total' and 'Avg' are \
             unaffected.");
        let grand_total: u128 = results.iter().map(|(_, (_, ns))| ns).sum();
        let top_level_total: u128 = results
            .iter()
            .filter(|(path, _)| path.len() == 1)
            .map(|(_, (_, ns))| *ns)
            .sum();
        for (path, (count, total_ns)) in &results {
            let path_str = path.join(" → ");
            let total_ms = *total_ns as f64 / 1_000_000.0;
            let avg_us = if *count > 0 {
                *total_ns as f64 / *count as f64 / 1_000.0
            } else {
                0.0
            };
            let pct = if top_level_total > 0 && path.len() == 1 {
                *total_ns as f64 / top_level_total as f64 * 100.0
            } else {
                0.0
            };
            eprintln!(
                "{:<90} {:>10} {:>15.2} {:>15.2} {:>14.1}%",
                path_str, count, total_ms, avg_us, pct
            );
        }
        // Allocation attribution, when built with --features alloc_tracker.
        // This is the column that says whether a region is compute-bound or
        // just churning the heap: `allocs/call` is the number to watch.
        #[cfg(feature = "alloc_tracker")]
        {
            let guard = get_alloc_timers();
            if let Some(ref map) = *guard {
                let mut allocs: Vec<(&Vec<&'static str>, &u64)> = map.iter().collect();
                allocs.sort_by_key(|a| std::cmp::Reverse(*a.1));
                eprintln!("\n=== Allocations by call path (inclusive, sorted) ===");
                eprintln!(
                    "{:<52} {:>12} {:>14} {:>14}",
                    "Call Path (leaf)", "Allocs", "Allocs/call", "Share"
                );
                eprintln!("{}", "-".repeat(96));
                let total: u64 = allocs.iter().map(|(_, v)| **v).sum();
                for (path, count) in allocs.iter().take(80) {
                    let count = **count;
                    let calls = results
                        .iter()
                        .find(|(p, _)| *p == path.as_slice())
                        .map(|(_, (c, _))| *c)
                        .unwrap_or(0);
                    let per_call = if calls > 0 {
                        count as f64 / calls as f64
                    } else {
                        0.0
                    };
                    let share = if total > 0 {
                        count as f64 / total as f64 * 100.0
                    } else {
                        0.0
                    };
                    eprintln!(
                        "{:<52} {:>12} {:>14.1} {:>13.1}%",
                        path.last().copied().unwrap_or("?"),
                        count,
                        per_call,
                        share
                    );
                }
                eprintln!("total attributed allocations: {}", total);
            }
        }
        eprintln!(
            "top-level (depth-1) instrumented total: {:.2} ms across {} path(s)\n",
            top_level_total as f64 / 1_000_000.0,
            results.iter().filter(|(path, _)| path.len() == 1).count()
        );
        let _ = grand_total;
        eprintln!("\n");
    }
}

/// Write the complete, untruncated timing + allocation report to
/// `timer_report.txt`. The console table truncates deep call paths at the
/// terminal width, which hides precisely the rows you need when chasing a
/// specific region.
fn write_full_report(results: &[(&Vec<&'static str>, &(u64, u128))]) {
    use std::fmt::Write as _;
    let mut out = String::new();
    writeln!(
        out,
        "=== TIMING (inclusive per call path; a nested path is ALSO counted\n    inside its parent's row, so these do NOT sum to wall time) ==="
    )
    .unwrap();
    writeln!(
        out,
        "{:>7}  {:>11}  {:>10}  {:>5}  call path",
        "calls", "total_ms", "avg_us", "depth"
    )
    .unwrap();
    let top_level: u128 = results
        .iter()
        .filter(|(p, _)| p.len() == 1)
        .map(|(_, (_, ns))| *ns)
        .sum();
    for (path, (count, total_ns)) in results {
        writeln!(
            out,
            "{:>7}  {:>11.2}  {:>10.2}  {:>5}  {}",
            count,
            *total_ns as f64 / 1_000_000.0,
            if *count > 0 {
                *total_ns as f64 / *count as f64 / 1_000.0
            } else {
                0.0
            },
            path.len(),
            path.join(" -> ")
        )
        .unwrap();
    }
    writeln!(
        out,
        "\ndepth-1 (top level) instrumented total: {:.2} ms",
        top_level as f64 / 1_000_000.0
    )
    .unwrap();

    // Exclusive time, recorded at drop rather than derived from the inclusive
    // table above. This is the table to optimize from: it answers "what does
    // this function cost on its own", which inclusive rows cannot.
    {
        let guard = get_self_timers();
        if let Some(ref map) = *guard {
            let mut rows: Vec<(&Vec<&'static str>, &(u64, u128))> = map.iter().collect();
            rows.sort_by_key(|r| std::cmp::Reverse(r.1 .1));
            let self_total: u128 = rows.iter().map(|r| r.1 .1).sum();
            let incl_total: u128 = results.iter().map(|r| r.1 .1).sum();
            writeln!(
                out,
                "\n\n=== SELF TIME (exclusive; sums exactly, unlike the inclusive table) ===\ntotal self: {:.2} ms  of {:.2} ms inclusive ({:.1}%)\n{:>7}  {:>10}  {:>9}  {:>5}  call path",
                self_total as f64 / 1_000_000.0,
                incl_total as f64 / 1_000_000.0,
                if incl_total > 0 {
                    self_total as f64 / incl_total as f64 * 100.0
                } else {
                    0.0
                },
                "calls",
                "self_ms",
                "self_us",
                "depth"
            )
            .unwrap();
            for (path, (calls, ns)) in rows.iter().take(45) {
                writeln!(
                    out,
                    "{:>7}  {:>10.2}  {:>9.2}  {:>5}  {}",
                    calls,
                    *ns as f64 / 1_000_000.0,
                    if *calls > 0 {
                        *ns as f64 / *calls as f64 / 1000.0
                    } else {
                        0.0
                    },
                    path.len(),
                    path.join(" -> ")
                )
                .unwrap();
            }
        }
    }

    #[cfg(feature = "alloc_tracker")]
    {
        let guard = get_alloc_timers();
        if let Some(ref map) = *guard {
            let mut allocs: Vec<(&Vec<&'static str>, &u64)> = map.iter().collect();
            allocs.sort_by_key(|a| std::cmp::Reverse(*a.1));
            let total: u64 = allocs.iter().map(|(_, v)| **v).sum();
            writeln!(
                out,
                "\n\n=== ALLOCATIONS (inclusive per call path) ===\ntotal attributed: {}\n{:>7}  {:>12}  {:>11}  {:>5}  {}",
                total, "calls", "allocs", "allocs/call", "depth", "call path"
            )
            .unwrap();
            for (path, count) in allocs {
                let calls = results
                    .iter()
                    .find(|(p, _)| *p == path.as_slice())
                    .map(|(_, (c, _))| *c)
                    .unwrap_or(0);
                writeln!(
                    out,
                    "{:>7}  {:>12}  {:>11.1}  {:>5}  {}",
                    calls,
                    count,
                    if calls > 0 {
                        *count as f64 / calls as f64
                    } else {
                        0.0
                    },
                    path.len(),
                    path.join(" -> ")
                )
                .unwrap();
            }
        }
    }
    // Size-class breakdown. The count table above says a region allocates a
    // lot; this says *what it allocates*. A path whose allocs are all in the
    // 8-15 byte classes is churning `String`/`to_string` on small values, which
    // is a different fix from one building real containers, so the two must be
    // separable before either is worth optimizing.
    #[cfg(feature = "alloc_tracker")]
    {
        let mut rows: Vec<(Vec<&'static str>, [u64; 14], u64)> = Vec::new();
        {
            let guard = get_alloc_buckets();
            if let Some(ref map) = *guard {
                for (path, buckets) in map.iter() {
                    // Call count, not the alloc total: the timing table is the
                    // only place that knows how many times a path ran, and
                    // dividing allocs by allocs would just print 1.0.
                    let calls = results
                        .iter()
                        .find(|(p, _)| *p == path.as_slice())
                        .map(|(_, (c, _))| *c)
                        .unwrap_or(0);
                    let sum: u64 = buckets.iter().sum();
                    if sum > 0 {
                        rows.push((path.clone(), *buckets, calls));
                    }
                }
            }
        }
        rows.sort_by_key(|r| {
            let sum: u64 = r.1.iter().sum();
            std::cmp::Reverse(sum)
        });
        let global: u64 = rows.iter().map(|r| r.2).sum();
        let mut gsum = [0u64; 14];
        for r in rows.iter() {
            for i in 0..14 {
                gsum[i] += r.1[i];
            }
        }
        writeln!(
            out,
            "\n\n=== ALLOC SIZE CLASSES (global) ===\n{:>7}  {:>9}  {:>7}  {}",
            "allocs", "bytes/avg", "share", "size class"
        )
        .unwrap();
        for i in 0..14 {
            if gsum[i] == 0 {
                continue;
            }
            writeln!(
                out,
                "{:>7}  {:>9}  {:>6.1}%  {}",
                gsum[i],
                bucket_bytes(i),
                gsum[i] as f64 / global.max(1) as f64 * 100.0,
                bucket_label(i)
            )
            .unwrap();
        }
        writeln!(
            out,
            "\n=== ALLOC SIZE CLASSES by call path (top 60 by allocs) ===\n{:>7}  {:>10}  {}",
            "allocs", "allocs/call", "size classes, as  pct of that path's allocs"
        )
        .unwrap();
        writeln!(out, "{}", "-".repeat(110)).unwrap();
        for (path, buckets, count) in rows.iter().take(60) {
            let sum: u64 = buckets.iter().sum();
            if sum == 0 {
                continue;
            }
            let per_call = if *count > 0 {
                sum as f64 / *count as f64
            } else {
                0.0
            };
            let profile: Vec<String> = (0..14)
                .filter(|i| buckets[*i] > 0)
                .map(|i| {
                    format!(
                        "{}={:.0}%",
                        bucket_label(i).trim(),
                        buckets[i] as f64 / sum as f64 * 100.0
                    )
                })
                .collect();
            writeln!(
                out,
                "{:>7}  {:>10.1}  {}  {}",
                sum,
                per_call,
                path.join(" -> "),
                profile.join(" ")
            )
            .unwrap();
        }
    }
    if let Err(e) = std::fs::write("timer_report.txt", out) {
        eprintln!("(could not write timer_report.txt: {})", e);
    }
}

#[cfg(feature = "alloc_tracker")]
fn bucket_label(i: usize) -> &'static str {
    match i {
        0 => "1-7B",
        1 => "8-15B",
        2 => "16-31B",
        3 => "32-63B",
        4 => "64-127B",
        5 => "128-255",
        6 => "256-511",
        7 => "512-1K",
        8 => "1K-2K",
        9 => "2K-4K",
        10 => "4K-8K",
        11 => "8K-16K",
        12 => "16K-32K",
        13 => "32K+",
        _ => "?",
    }
}

/// Midpoint of a size class, for the "bytes/avg" column. An average over a
/// whole class is only a rough figure, but it is enough to tell a 24-byte
/// `Vec` header from a 4 KB clone of a card's ability text.
#[cfg(feature = "alloc_tracker")]
fn bucket_bytes(i: usize) -> usize {
    match i {
        0 => 4,
        1 => 12,
        2 => 24,
        3 => 48,
        4 => 96,
        5 => 192,
        6 => 384,
        7 => 768,
        8 => 1536,
        9 => 3072,
        10 => 6144,
        11 => 12288,
        12 => 24576,
        13 => 49152,
        _ => 0,
    }
}

/// Emit timer data in inferno "folded stack" format for flamegraph generation.
///
/// Each call path is written as `frame1;frame2;...;<leaf> <nanoseconds>`, where
/// the count is the total nanoseconds spent in that exact call path. This can be
/// piped into `inferno`'s `FlameGraph` to produce an SVG.
pub fn print_folded() {
    let guard = get_timers();
    if let Some(ref map) = *guard {
        let mut results: Vec<_> = map.iter().collect();
        results.sort_by(|a, b| a.0.cmp(b.0));
        for (path, (_count, total_ns)) in &results {
            let folded = path.join(";");
            println!("{} {}", folded, total_ns);
        }
    }
}

pub fn reset() {
    let mut guard = get_timers();
    if let Some(ref mut map) = *guard {
        map.clear();
    }
    CALL_STACK.with(|stack| stack.borrow_mut().clear());
}

/// Macro to time a block of code. Usage: `timeit!("label", { ... })`
#[macro_export]
macro_rules! timeit {
    ($label:expr, $body:block) => {{
        let _timer = $crate::timer::Timer::start($label);
        $body
    }};
}
