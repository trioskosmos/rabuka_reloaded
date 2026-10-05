
use crate::compat::atomic::AtomicBool;
use core::sync::atomic::Ordering;

static ACTION_DISPLAY: AtomicBool = AtomicBool::new(true);

pub fn set_action_display(enabled: bool) {
    ACTION_DISPLAY.store(enabled, Ordering::Relaxed);
}

#[inline]
pub fn action_display_enabled() -> bool {
    ACTION_DISPLAY.load(Ordering::Relaxed)
}

static LOGGING: AtomicBool = AtomicBool::new(true);

pub fn set_logging_enabled(enabled: bool) {
    LOGGING.store(enabled, Ordering::Relaxed);
}

#[inline]
pub fn logging_enabled() -> bool {
    LOGGING.load(Ordering::Relaxed)
}

pub fn set_training_mode(enabled: bool) {
    set_action_display(!enabled);
    set_logging_enabled(!enabled);
}

#[cfg(not(feature = "no_std"))]
static AB_OVERRIDE: std::sync::Mutex<Option<(&'static str, bool)>> = std::sync::Mutex::new(None);

#[cfg(not(feature = "no_std"))]
pub fn set_ab_flag(name: &'static str, on: bool) {
    if let Ok(mut g) = AB_OVERRIDE.lock() {
        *g = Some((name, on));
    }
}

pub fn ab_flag(name: &str) -> bool {
    #[cfg(not(feature = "no_std"))]
    if let Ok(g) = AB_OVERRIDE.lock() {
        if let Some((n, on)) = *g {
            if n == name {
                return on;
            }
        }
    }
    #[cfg(not(feature = "no_std"))]
    {
        let key = format!("RABUKA_AB_{}", name.to_ascii_uppercase());
        match std::env::var(&key) {
            Ok(v) => !matches!(v.trim(), "off" | "0" | "false" | "no"),
            Err(_) => true,
        }
    }
    #[cfg(feature = "no_std")]
    {
        let _ = name;
        true
    }
}

// ---------------------------------------------------------------------------
// The `recalculate_constants` skip-gate counters that lived here are GONE, and so is the
// gate. Five attempts were made and all are recorded at the top of that function; the
// fifth was sound over all five sources, free, passed the suite, and measured +0.2%.
//
// Two things are worth keeping from that, and both are the general lesson rather than
// this optimisation:
//
//   * the skip RATE is the number to trust for a conditional optimisation — it is
//     deterministic and per-mechanism, whereas gps is not comparable across builds
//     because a faster engine fits more games into the same warmup window. Build the
//     counter, measure the rate, then decide; do not decide from gps.
//   * `Timer::start` costs ~0.44 us, so a function at ~5 us cannot be resolved by
//     per-phase timers at all. Measure the rate, or measure end to end.
//
// Nothing here reported anything after the gate was removed, so the counters were dead.
// ---------------------------------------------------------------------------
