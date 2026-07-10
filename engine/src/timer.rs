use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

static TIMERS: Mutex<Option<HashMap<&'static str, (u64, u128)>>> = Mutex::new(None);

fn get_timers() -> std::sync::MutexGuard<'static, Option<HashMap<&'static str, (u64, u128)>>> {
    let mut guard = TIMERS.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    guard
}

pub struct Timer {
    label: &'static str,
    start: Instant,
}

impl Timer {
    pub fn start(label: &'static str) -> Self {
        Timer {
            label,
            start: Instant::now(),
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed().as_nanos();
        let mut guard = get_timers();
        if let Some(ref mut map) = *guard {
            let entry = map.entry(self.label).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += elapsed;
        }
    }
}

pub fn print_results() {
    let guard = get_timers();
    if let Some(ref map) = *guard {
        let mut results: Vec<_> = map.iter().collect();
        results.sort_by(|a, b| b.1 .1.cmp(&a.1 .1));
        eprintln!("\n=== Timing Results (sorted by total time) ===");
        eprintln!(
            "{:<70} {:>10} {:>15} {:>15} {:>15}",
            "Function", "Calls", "Total (ms)", "Avg (µs)", "% of total"
        );
        eprintln!("{}", "-".repeat(130));
        let grand_total: u128 = results.iter().map(|(_, (_, ns))| ns).sum();
        for (label, (count, total_ns)) in &results {
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
                "{:<70} {:>10} {:>15.2} {:>15.2} {:>14.1}%",
                label, count, total_ms, avg_us, pct
            );
        }
        eprintln!("\n");
    }
}

pub fn reset() {
    let mut guard = get_timers();
    if let Some(ref mut map) = *guard {
        map.clear();
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
