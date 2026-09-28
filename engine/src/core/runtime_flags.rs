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

#[cfg(feature = "no_std")]
use core::sync::atomic::{AtomicBool, Ordering};
#[cfg(not(feature = "no_std"))]
use std::sync::atomic::{AtomicBool, Ordering};

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
