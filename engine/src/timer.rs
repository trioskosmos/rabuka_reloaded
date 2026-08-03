use crate::HashMap;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use std::sync::Mutex;
use std::time::Instant;

static TIMERS: Mutex<Option<HashMap<Vec<&'static str>, (u64, u128)>>> = Mutex::new(None);
static CALL_STACK: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

fn get_timers() -> std::sync::MutexGuard<'static, Option<HashMap<Vec<&'static str>, (u64, u128)>>> {
    let mut guard = TIMERS.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::default());
    }
    guard
}

pub struct Timer {
    label: &'static str,
    start: Instant,
}

impl Timer {
    pub fn start(label: &'static str) -> Self {
        if !cfg!(feature = "profiling") {
            return Timer {
                label,
                start: Instant::now(),
            };
        }
        if let Ok(mut stack) = CALL_STACK.lock() {
            stack.push(label);
        }
        Timer {
            label,
            start: Instant::now(),
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        if !cfg!(feature = "profiling") {
            return;
        }
        let elapsed = self.start.elapsed().as_nanos();

        // Reconstruct the full call path (entire stack at this moment)
        let call_path: Vec<&'static str> = if let Ok(stack) = CALL_STACK.lock() {
            // Check that we're at the top of the stack
            if stack.last() == Some(&self.label) {
                stack.clone()
            } else {
                vec![self.label]
            }
        } else {
            vec![self.label]
        };

        // Remove ourselves from the call stack
        if let Ok(mut stack) = CALL_STACK.lock() {
            if stack.last() == Some(&self.label) {
                stack.pop();
            }
        }

        // Record the time against the full call path
        if !call_path.is_empty() {
            let mut guard = get_timers();
            if let Some(ref mut map) = *guard {
                let entry = map.entry(call_path).or_insert((0, 0));
                entry.0 += 1;
                entry.1 += elapsed;
            }
        }
    }
}

pub fn get_data() -> Vec<(Vec<&'static str>, u64, u128)> {
    let guard = get_timers();
    if let Some(ref map) = *guard {
        let mut results: Vec<_> = map.iter().map(|(k, &v)| (k.clone(), v.0, v.1)).collect();
        results.sort_by(|a, b| b.2.cmp(&a.2));
        results
    } else {
        Vec::new()
    }
}

pub fn print_results() {
    let guard = get_timers();
    if let Some(ref map) = *guard {
        let mut results: Vec<_> = map.iter().collect();
        results.sort_by(|a, b| b.1 .1.cmp(&a.1 .1));
        eprintln!("\n=== Timing Results (sorted by total time) ===");
        eprintln!(
            "{:<90} {:>10} {:>15} {:>15} {:>15}",
            "Call Path", "Calls", "Total (ms)", "Avg (µs)", "% of total"
        );
        eprintln!("{}", "-".repeat(150));
        let grand_total: u128 = results.iter().map(|(_, (_, ns))| ns).sum();
        for (path, (count, total_ns)) in &results {
            let path_str = path.join(" → ");
            let total_ms = *total_ns as f64 / 1_000_000.0;
            let avg_us = if *count > 0 {
                *total_ns as f64 / *count as f64 / 1_000.0
            } else {
                0.0
            };
            let pct = if grand_total > 0 {
                *total_ns as f64 / grand_total as f64 * 100.0
            } else {
                0.0
            };
            eprintln!(
                "{:<90} {:>10} {:>15.2} {:>15.2} {:>14.1}%",
                path_str, count, total_ms, avg_us, pct
            );
        }
        eprintln!("\n");
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
    if let Ok(mut stack) = CALL_STACK.lock() {
        stack.clear();
    }
}

/// Macro to time a block of code. Usage: `timeit!("label", { ... })`
#[macro_export]
macro_rules! timeit {
    ($label:expr, $body:block) => {{
        let _timer = $crate::timer::Timer::start($label);
        $body
    }};
}
