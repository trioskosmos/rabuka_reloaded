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

    /// Cards referenced by the currently available actions (deduped), for
    /// consoles whose board highlights actionable cards. Called from the
    /// human-turn loop each frame before [`PlatformUi::render_board`].
    fn set_actionable_cards(&mut self, _card_nos: &[String]) {}

    /// The currently selected action in the human-turn list (first line of its
    /// description plus its index and the total count), for consoles whose
    /// board shows an action bar. Called each frame before
    /// [`PlatformUi::render_board`].
    fn set_selected_action(&mut self, _desc: &str, _index: usize, _total: usize) {}

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

    /// Max menu items visible at once (the scroll window height).
    /// Platforms with tall screens override this so menus use the full
    /// display instead of scrolling after 7 items. The engine adds a
    /// title line and possibly a ".. N more" line around the list.
    fn option_rows(&self) -> usize {
        7
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

/// Column width estimate for a `{{name|label}}` texticon token's inner part.
/// Renderers substitute the token with a baked icon whose width varies by
/// name; the label's column count (min 2, like a CJK glyph) is a close
/// portable estimate good enough to wrap around.
fn token_cols(inner: &str) -> usize {
    let label = inner.split('|').next_back().unwrap_or("");
    2.max(label.chars().map(char_cols).sum())
}

/// Split `text` into per-char pieces, keeping `{{...}}` texticon tokens as
/// single atomic `(width, token)` units so wrapping never cuts a token in
/// half (renderers draw the whole token as one inline icon).
fn text_units(text: &str) -> Vec<(usize, &str)> {
    let mut units: Vec<(usize, &str)> = Vec::new();
    let mut i = 0usize;
    while i < text.len() {
        if text[i..].starts_with("{{") {
            if let Some(close) = text[i + 2..].find("}}") {
                let end = i + 2 + close + 2;
                let inner = &text[i + 2..i + 2 + close];
                units.push((token_cols(inner), &text[i..end]));
                i = end;
                continue;
            }
        }
        let ch = text[i..].chars().next().unwrap();
        units.push((char_cols(ch), &text[i..i + ch.len_utf8()]));
        i += ch.len_utf8();
    }
    units
}

/// Wrap `text` into lines of at most `cols` half-width columns, honouring
/// existing newlines as hard breaks and keeping `{{...}}` texticon tokens
/// intact. Public so console ports reuse it instead of re-rolling
/// token-unaware wrappers.
pub fn wrap_text(text: &str, cols: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut w = 0usize;
    for piece in text.split('\n') {
        for (cw, unit) in text_units(piece) {
            if w + cw > cols && !cur.is_empty() {
                out.push(core::mem::take(&mut cur));
                w = 0;
            }
            cur.push_str(unit);
            w += cw;
        }
        if !cur.is_empty() {
            out.push(core::mem::take(&mut cur));
        }
        w = 0;
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

/// Truncate `text` to a single line of at most `cols` columns, marking the
/// cut so the player knows more is available (see the L/R detail viewer).
/// `{{...}}` texticon tokens are kept whole: a token that no longer fits
/// ends the line instead of being cut in half. Public for console ports.
pub fn one_line(text: &str, cols: usize) -> String {
    let mut s = String::new();
    let mut w = 0usize;
    for (cw, unit) in text_units(text) {
        if w + cw > cols {
            s.push_str("..");
            break;
        }
        s.push_str(unit);
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
