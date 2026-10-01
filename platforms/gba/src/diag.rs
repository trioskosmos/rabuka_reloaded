//! Diagnostics for the GBA port.
//!
//! Why this exists, and why it does NOT use the `log` crate:
//!
//! 1. **Nothing here ever installed a logger.** The `log` crate's default is
//!    a no-op with max level `Off`, so every `log::debug!` in `display.rs`
//!    was silently discarded. The port has been debug-blind. `display.rs`
//!    now routes those through [`note`] instead.
//! 2. **The `log` crate's no_std logger API is unavailable here.** `set_logger`
//!    and `set_max_level` are gated behind `#[cfg(target_has_atomic = "ptr")]`
//!    (log-0.4.22 `lib.rs:1313`, `:1395`). ARMv4T has no atomic-pointer
//!    support, so they do not exist in this build at all. That is also why the
//!    engine's own `compat::atomic` uses `Cell` fallbacks.
//! 3. `agb::println!` formats and transmits a line per call. That is fine for
//!    a rare event and ruinous for per-frame chatter, so tracing is gated.
//!
//! # Mode is a build-time decision
//!
//! There are deliberately **no diagnostic hotkeys**. Every diagnostic ships
//! as its own ROM, selected by a cargo feature (see `Cargo.toml`), so the
//! ROM you boot is the ROM whose behaviour you get — there is no held-button
//! modifier to mis-time and no way to capture the wrong mode by accident.
//!
//! The numeric diagnostics are the on-screen `vram_stats` readout drawn by
//! the board view; see `Display::vram_stats` for why OBJ VRAM is the
//! constraint worth watching there.

use core::cell::Cell;

/// 0 = off, 1 = info, 2 = debug.
type Level = u8;

// `Cell` is not `Sync`, so a plain `static` of one will not compile. The GBA
// is single-threaded and these are trivial word-sized flags, so raw statics
// with `addr_of` access are the honest fit (no `unsafe impl Sync` hand-waving).

/// Trace level this ROM was built with: 2 (verbose) under `diag-trace`,
/// otherwise 1 (info, rare lines only).
static mut LEVEL: Cell<Level> = Cell::new(if cfg!(feature = "diag-trace") { 2 } else { 1 });

/// Card art enabled at boot: off under `diag-noart`.
static mut ART_ENABLED: Cell<bool> = Cell::new(!cfg!(feature = "diag-noart"));

/// Build identity, printed once at startup so a ROM can never be confused
/// with the baseline when results are compared side by side.
const BUILD_LABEL: &str = if cfg!(feature = "diag-noart") {
    "noart"
} else if cfg!(feature = "diag-trace") {
    "trace"
} else if cfg!(feature = "diag-cardid") {
    "cardid"
} else if cfg!(feature = "diag-spriteraw") {
    "spriteraw"
} else if cfg!(feature = "diag-bgspeck") {
    "bgspeck"
} else {
    "baseline"
};

fn level() -> Level {
    // SAFETY: single-threaded target; only boot initialisation writes LEVEL.
    unsafe { (*core::ptr::addr_of!(LEVEL)).get() }
}

/// Whether `debug!`-class tracing is enabled.
pub fn want_debug() -> bool {
    level() >= 2
}

/// Which diagnostic this ROM is. Printed at startup.
pub fn build_label() -> &'static str {
    BUILD_LABEL
}

/// Emit one rare diagnostic line.
pub fn note(args: core::fmt::Arguments<'_>) {
    agb::println!("[diag] {args}");
}

/// Announce the build once the display is up, so the log and the ROM on
/// screen always agree about which mode produced a given capture.
pub fn announce() {
    note(format_args!(
        "build={} trace={} art={}",
        BUILD_LABEL,
        if want_debug() { "debug" } else { "info" },
        if art_enabled() { "on" } else { "OFF" }
    ));
}

/// Whether card sprites should be drawn.
pub fn art_enabled() -> bool {
    // SAFETY: single-threaded; written only at boot initialisation.
    unsafe { (*core::ptr::addr_of!(ART_ENABLED)).get() }
}