//! Process-global speed/verbosity gates.
//!
//! These live in `core`, not in the platform shell, because the rules engine
//! reads them on its hot paths: `core::game_state` (log materialization),
//! `turn::phases`, `turn::live` and `ability::resolver` all guard
//! `format!`/`LogEntry` construction behind `logging_enabled()`. They used to
//! live in `game::game_setup`, which made `core` import the shell and left the
//! two modules in a dependency cycle that only compiled because they share a
//! crate.
//!
//! Both flags are process-global and default ON, so web/UI paths and the test
//! suite are unaffected. Training and headless benches turn them OFF via
//! `set_training_mode`.

use crate::compat::atomic::AtomicBool;
use core::sync::atomic::Ordering;

/// Runtime display gate for action-generation UI strings (EN/JA descriptions,
/// card_name/card_no/source_ability copies, existing_member_name labels).
/// Default ON so web/UI paths are unchanged. Headless benches (`sim_bench`)
/// turn it off to skip `format!`/`to_string` on every decision step.
/// Decision fields (`card_id`, `stage_area`, `available_areas`, costs,
/// `ability_index`, `card_indices`) are always built.
static ACTION_DISPLAY: AtomicBool = AtomicBool::new(true);

/// Enable/disable display-only fields on generated `Action`s.
/// Thread-safe; process-global. Default is enabled.
pub fn set_action_display(enabled: bool) {
    ACTION_DISPLAY.store(enabled, Ordering::Relaxed);
}

/// Whether display-only action fields should be materialized.
#[inline]
pub fn action_display_enabled() -> bool {
    ACTION_DISPLAY.load(Ordering::Relaxed)
}

/// Runtime gate for rule/structured/debug log materialization (format!,
/// LogEntry construction, choice offer/resolve payloads). Default ON so
/// web/tests keep full history. Training/sim turn it off.
static LOGGING: AtomicBool = AtomicBool::new(true);

/// Enable/disable log string materialization. Thread-safe; process-global.
pub fn set_logging_enabled(enabled: bool) {
    LOGGING.store(enabled, Ordering::Relaxed);
}

/// Whether hot-path log strings should be built.
#[inline]
pub fn logging_enabled() -> bool {
    LOGGING.load(Ordering::Relaxed)
}

/// Training/headless speed: strip UI Action strings and log materialization.
/// Web/UI paths must leave this at the default (full display + logs).
pub fn set_training_mode(enabled: bool) {
    set_action_display(!enabled);
    set_logging_enabled(!enabled);
}

// ---------------------------------------------------------------------------
// Counters for the `recalculate_constants` fast path.
//
// The end-to-end bench is NOT a sound measure of that optimisation:
// `CardDatabase.cards` is a `HashMap` with a randomised hasher, so game outcomes — and
// therefore action counts — differ between runs of the SAME binary, and any A/B across
// builds carries that noise. What IS deterministic is how often the gate fires, which is
// exactly what the optimisation claims. Read the skip rate, not the wall clock.
// ---------------------------------------------------------------------------

#[cfg(not(feature = "no_std"))]
static CONST_RECALC_SKIPS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
#[cfg(not(feature = "no_std"))]
static CONST_RECALC_FULLS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

#[cfg(not(feature = "no_std"))]
#[inline]
pub fn note_constant_recalc_skip() {
    CONST_RECALC_SKIPS.fetch_add(1, Ordering::Relaxed);
}

#[cfg(not(feature = "no_std"))]
#[inline]
pub fn note_constant_recalc_full() {
    CONST_RECALC_FULLS.fetch_add(1, Ordering::Relaxed);
}

/// A/B switch: `RABUKA_AB_<NAME>=off` disables one named optimisation at runtime.
///
/// This exists because comparing two BUILDS with `sim_bench` is not sound. Game length
/// and action count move whenever engine behaviour moves, so `mean_aps` mixes the
/// optimisation with a different set of games — measured here as `total_actions` swinging
/// 197k → 202k between builds, which is ±2.5% on top of whatever the change actually did.
///
/// With the flag, both arms run the SAME binary and therefore the SAME games, so the
/// difference is the optimisation and nothing else. Read `total_actions` in the summary to
/// confirm the arms really did play the same games; if it differs, the comparison is void.
///
/// Names are upper-cased and `RABUKA_AB_` is prefixed, so `ab_flag("check_timing")` reads
/// `RABUKA_AB_CHECK_TIMING`. Anything other than an explicit `off`/`0`/`false` leaves the
/// optimisation ON, so a typo cannot silently disable a code path.
pub fn ab_flag(name: &str) -> bool {
    let key = format!("RABUKA_AB_{}", name.to_ascii_uppercase());
    match std::env::var(&key) {
        Ok(v) => !matches!(v.trim(), "off" | "0" | "false" | "no"),
        Err(_) => true,
    }
}

/// `(skipped, full)`.
#[cfg(not(feature = "no_std"))]
pub fn constant_recalc_counts() -> (u64, u64) {
    (
        CONST_RECALC_SKIPS.load(Ordering::Relaxed),
        CONST_RECALC_FULLS.load(Ordering::Relaxed),
    )
}

/// Set `RABUKA_NO_CONSTANT_GATE=1` to disable the `recalculate_constants` fast path.
///
/// The fast path itself was measured and REMOVED (see the record in `modifiers.rs`), so
/// this switch is gone with it. [`ab_flag`] is the general replacement: one helper for any
/// optimisation, rather than one env var per abandoned experiment.
#[cfg(not(feature = "no_std"))]
pub fn constant_recalc_gated_off() -> bool {
    !ab_flag("constant_recalc_gate")
}

#[cfg(not(feature = "no_std"))]
pub fn reset_constant_recalc_counts() {
    CONST_RECALC_SKIPS.store(0, Ordering::Relaxed);
    CONST_RECALC_FULLS.store(0, Ordering::Relaxed);
}
