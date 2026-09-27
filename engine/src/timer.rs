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
            return Timer { label };
        }
        #[cfg(feature = "profiling")]
        {
            CALL_STACK.with(|stack| stack.borrow_mut().push(label));
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
                #[cfg(feature = "alloc_tracker")]
                {
                    let mut guard = get_alloc_timers();
                    if let Some(ref mut map) = *guard {
                        *map.entry(call_path).or_insert(0) += allocs;
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
        "{:>7}  {:>11}  {:>10}  {:>5}  {}",
        "calls", "total_ms", "avg_us", "depth", "call path"
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
    if let Err(e) = std::fs::write("timer_report.txt", out) {
        eprintln!("(could not write timer_report.txt: {})", e);
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
