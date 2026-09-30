//! Diagnostics for the GBA port.
//!
//! Why this exists, and why it does NOT use the `log` crate:
//!
//! 1. **Nothing here ever installed a logger.** The `log` crate's default is a
//!    no-op with max level `Off`, so every `log::debug!` in `display.rs` —
//!    including all the sprite-VRAM pressure messages — was silently
//!    discarded. The port has been debug-blind.
//! 2. **The `log` crate's no_std logger API is unavailable here.** `set_logger`
//!    and `set_max_level` are gated behind `#[cfg(target_has_atomic = "ptr")]`
//!    (log-0.4.22 `lib.rs:1313`, `:1395`). ARMv4T has no atomic-pointer
//!    support, so they do not exist in this build at all. That is also why the
//!    engine's own `compat::atomic` uses `Cell` fallbacks.
//! 3. `agb::println!` formats and transmits a line per call. That is fine for
//!    a rare event and ruinous for per-frame chatter, so this module gates on
//!    its own level rather than spraying prints.
//!
//! The important diagnostics are numbers, so the primary channel is
//! [`crate::ui::Display::vram_stats`] drawn on screen by the board view. See
//! `vram_stats` for why OBJ VRAM is the constraint worth watching.

use core::cell::Cell;

/// 0 = off, 1 = info, 2 = debug.
type Level = u8;

// `Cell` is not `Sync`, so a plain `static` of one will not compile. The GBA is
// single-threaded and these are trivial word-sized flags, so raw statics with
// `addr_of` access are the honest fit (no `unsafe impl Sync` hand-waving).
static mut LEVEL: Cell<Level> = Cell::new(1);

fn level() -> Level {
    // SAFETY: single-threaded target; only this module writes LEVEL, from the
    // main loop, never from an interrupt handler.
    unsafe { (*core::ptr::addr_of!(LEVEL)).get() }
}

/// Enable `debug`-level tracing (per-frame sprite-cache chatter).
///
/// Only worth turning on with an emulator console or link cable attached: it is
/// high-volume and will itself visibly slow the game.
pub fn set_verbose(on: bool) {
    // SAFETY: as `level()`.
    unsafe { (*core::ptr::addr_of!(LEVEL)).set(if on { 2 } else { 1 }) }
}

pub fn verbose() -> bool {
    level() >= 2
}

/// Whether `debug!`-class tracing is enabled.
pub fn want_debug() -> bool {
    level() >= 2
}

/// Short level tag for the on-screen readout.
pub fn level_tag() -> &'static str {
    match level() {
        0 => "off",
        2 => "dbg",
        _ => "info",
    }
}

/// Emit one rare diagnostic line.
pub fn note(args: core::fmt::Arguments<'_>) {
    agb::println!("[diag] {args}");
}
