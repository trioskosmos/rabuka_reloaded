// Stereoscopic 3D configuration for the top screen.
//
// How it works: every queued draw op carries a depth in [0,1]. The C renderer
// (`ctru_shim.c`) offsets each op per eye, symmetric around the screen plane:
//   off_eye = +/- slider * max_shift * strength * depth / 2
// so flat UI (depth 0) stays pixel-sharp while cards float. The right-eye pass
// is skipped entirely when the slider is ~off, halving GPU cost in 2D mode.
//
// Depth budget (total disparity at slider max, 32px * depth):
//   0.0  screen plane - header text, hint bar, camera preview. Always sharp.
//   0.05 backgrounds, 0.10 body text, 0.15 panels - subtle separation.
//   0.55 resting cards - clearly above the panel behind them.
//   0.80 showcase portraits (card detail) - hero lift.
//   1.00 selected/focused card - pops + drop shadow (also visible in 2D).
//
// Comfort: text never exceeds 0.15 (~5px total disparity), so long ability
// descriptions stay readable with the slider fully up. Only card images -
// large, high-contrast targets - use the upper half of the range.

/// Resting card-image depth (grid cells, choice options).
pub const CARD_DEPTH: f32 = 0.55;
/// Large showcase portrait depth (card-detail left column).
pub const PORTRAIT_DEPTH: f32 = 0.80;
/// Focused/selected card depth: full pop + drop shadow.
pub const SELECTED_DEPTH: f32 = 1.0;

/// 3D strength presets (multiplier on the slider-driven disparity).
/// The hardware slider is the master: at slider 0 the output is flat no
/// matter the strength. At slider max, total disparity = 32px * strength.
/// The C default is POP (2.0) — the physical slider is right there if it's
/// too strong. 1.0 = full but comfortable; anything higher than 2.0 can't
/// be fused by human eyes on a 400px screen, so the C side clamps at 2.0:
/// x10 would just be double vision plus LCD crosstalk, not more depth.
pub const STRENGTH_OFF: f32 = 0.0;
pub const STRENGTH_COMFORT: f32 = 0.5;
pub const STRENGTH_FULL: f32 = 1.0;
pub const STRENGTH_POP: f32 = 2.0;

/// Apply a strength preset to the C renderer. Call when the player changes
/// the setting (takes effect on the next frame).
pub fn apply_strength(strength: f32) {
    unsafe {
        crate::ffi::_3ds_set_3d_strength(strength);
    }
}

/// Current 3D-slider position (0.0..1.0) as reported by the OS.
pub fn slider_position() -> f32 {
    unsafe { crate::ffi::_3ds_get_3d_slider() }
}

/// Whether stereoscopic output is currently active (enabled, slider up).
pub fn is_stereo_active() -> bool {
    slider_position() > 0.02
}
