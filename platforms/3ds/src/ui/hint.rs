// Consistent hint bar at the bottom of the top screen.

use crate::ffi::{_3ds_bot_queue_text, _3ds_top_queue_text};
use crate::ui::colors::COL_MED;

/// The y position is always 225 (above the 240px bottom edge).
pub const HINT_BAR_Y: f32 = 218.0;
pub const HINT_BAR_SCALE: f32 = 0.58;

/// Render a hint bar at the bottom of the top screen.
/// Place this in the overlay's rendering code to show button hints.
pub fn render_hint_bar(text: &str) {
    unsafe {
        _3ds_top_queue_text(
            4.0,
            HINT_BAR_Y,
            COL_MED,
            HINT_BAR_SCALE,
            format!("{}\0", text).as_ptr(),
        );
    }
}

/// Render a hint bar at the bottom of the bottom (touch) screen.
/// Used by setup-phase menus that render on the bottom screen.
pub fn render_hint_bar_bot(text: &str) {
    unsafe {
        _3ds_bot_queue_text(
            4.0,
            HINT_BAR_Y,
            COL_MED,
            HINT_BAR_SCALE,
            format!("{}\0", text).as_ptr(),
        );
    }
}
