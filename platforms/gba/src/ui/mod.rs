//! UI primitives for the GBA port — mirrors the 3DS `ui/` crate.
//!
//! The GBA crate keeps its baked rendering assets at the crate root
//! (`board`, `display`, `card_art_gen`, `font_tiles_gen` are private root
//! mods so regeneration scripts keep writing to their original paths). This
//! module re-exports them under `rabuka_gba::ui::*` so port code has a single
//! `ui` namespace like the 3DS port, while the files stay where the toolchain
//! expects.

pub use crate::board::{Board, BoardFrame, Slot, HAND_VISIBLE};
pub use crate::card_art_gen::{CardArt, CardFront, BACK_FRONT, BOARD_UI, CARD_ART, CARD_FRONTS};
pub use crate::display::Display;
pub use crate::font_tiles_gen::{FONT_GLYPHS, FONT_TILES};
pub use crate::texticons_gen::{TEXTICON_GLYPHS, TEXTICON_TILES};
