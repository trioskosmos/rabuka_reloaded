//! Portable UI abstraction for console front-ends.
//!
//! This module defines the `PlatformUi` trait — the seam every console
//! implements — plus the low-level text/card formatting helpers that are
//! portable across ports. The *menus* (list selection, turn/choice screens,
//! result viewer) live in [`menu`], and the *match runner* (mode/deck select +
//! game loop) lives in [`match_runner`]. Both are re-exported here so the other
//! ports keep compiling against `rabuka_engine::game::platform_ui::*`.

#[cfg(feature = "no_std")]
use alloc::format;
#[cfg(feature = "no_std")]
use alloc::string::{String, ToString};
#[cfg(feature = "no_std")]
use alloc::vec::Vec;
#[cfg(not(feature = "no_std"))]
use std::format;

use crate::card::Card;
use crate::game_state::GameState;

/// A console's UI backend. The engine drives menus through this trait; each
/// port supplies its own display/input implementation.
pub trait PlatformUi {
    fn clear_screen(&mut self);
    fn println(&mut self, text: &str);
    fn swap_buffers(&mut self);
    fn poll_input(&mut self);
    fn just_pressed_a(&self) -> bool;
    fn just_pressed_b(&self) -> bool;
    fn just_pressed_up(&self) -> bool;
    fn just_pressed_down(&self) -> bool;
    fn just_pressed_start(&self) -> bool;
    fn wait_vblank(&mut self);

    /// Render a graphical board for `gs` (consoles that support one). Called
    /// from the human-turn loop in place of `swap_buffers` so a board can be
    /// drawn. Return `true` if the renderer consumed this frame's input (e.g.
    /// it is in a board-navigation mode that handles Up/Down itself) so the
    /// engine skips its own action navigation/execution for this frame. The
    /// default just swaps the text buffer and does not consume input.
    fn render_board(&mut self, _gs: &GameState) -> bool {
        self.swap_buffers();
        false
    }

    /// Shoulder buttons. Default off; consoles without them keep the menu
    /// behaviour unchanged. L/R open the full-text detail viewer on the
    /// currently highlighted option.
    fn just_pressed_l(&self) -> bool {
        false
    }
    fn just_pressed_r(&self) -> bool {
        false
    }

    /// Max characters that fit on one menu line, in half-width columns
    /// (a CJK glyph is 2 columns). Used to keep each option on a single
    /// line and to wrap the detail viewer. 30 fits a GBA screen row
    /// (240px / 8px tiles).
    fn option_cols(&self) -> usize {
        30
    }
}

/// Width of a single character in half-width columns (CJK glyphs = 2).
pub(crate) fn char_cols(c: char) -> usize {
    if (c as u32) < 0x1100 {
        1
    } else {
        2
    }
}

/// Wrap `text` into lines of at most `cols` half-width columns, honouring
/// existing newlines as hard breaks.
pub(crate) fn wrap_text(text: &str, cols: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut w = 0usize;
    for ch in text.chars() {
        if ch == '\n' {
            if !cur.is_empty() {
                out.push(core::mem::take(&mut cur));
            }
            w = 0;
            continue;
        }
        let cw = char_cols(ch);
        if w + cw > cols && !cur.is_empty() {
            out.push(core::mem::take(&mut cur));
            w = 0;
        }
        cur.push(ch);
        w += cw;
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

/// Truncate `text` to a single line of at most `cols` columns, marking the
/// cut so the player knows more is available (see the L/R detail viewer).
pub(crate) fn one_line(text: &str, cols: usize) -> String {
    let mut s = String::new();
    let mut w = 0usize;
    for ch in text.chars() {
        let cw = char_cols(ch);
        if w + cw > cols {
            s.push_str("..");
            break;
        }
        s.push(ch);
        w += cw;
    }
    s
}

/// Format a heart map as "R2 B3"-style codes (like the 3DS stat line).
pub fn heart_str(hm: &crate::card::HeartMap) -> String {
    hm.iter()
        .map(|(c, v)| format!("{}{}", c.short_label(), v))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Single-line stat summary, mirroring the 3DS card-detail stat line.
pub fn card_stat_text(card: &Card) -> String {
    use crate::card::CardType;
    match card.card_type {
        CardType::Member => {
            let mut s = String::new();
            if let Some(cost) = card.cost {
                if cost > 0 {
                    s.push_str(&format!("E{}  ", cost));
                }
            }
            let hs = card
                .base_heart
                .as_ref()
                .map(|bh| heart_str(&bh.hearts))
                .unwrap_or_default();
            if !hs.is_empty() {
                s.push_str(&hs);
                s.push_str("  ");
            }
            if card.blade > 0 {
                s.push_str(&format!("BL{}", card.blade));
            }
            s
        }
        CardType::Live => {
            let mut s = String::new();
            if let Some(score) = card.score {
                if score > 0 {
                    s.push_str(&format!("SC{}  ", score));
                }
            }
            let ns = card
                .need_heart
                .as_ref()
                .map(|nh| heart_str(&nh.hearts))
                .unwrap_or_default();
            if !ns.is_empty() {
                s.push_str(&ns);
            }
            s
        }
        CardType::Energy => String::new(),
    }
}

/// A card's ability text. In compact builds `ability_text()` is empty, so the
/// abilities are decoded from bytecode (which reconstructs `full_text`).
/// `{{...}}` icon markers are kept so the renderer can show them.
pub fn card_ability_text(card: &Card) -> String {
    let mut parts: Vec<String> = Vec::new();
    for ab in card.resolved_abilities() {
        let t = ab.full_text.trim();
        if !t.is_empty() {
            parts.push(t.to_string());
        }
    }
    if parts.is_empty() {
        card.ability_text().trim().to_string()
    } else {
        parts.join("\n")
    }
}

pub use crate::game::menu::*;
pub use crate::game::match_runner::*;
