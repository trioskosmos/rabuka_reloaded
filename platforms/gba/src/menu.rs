//! GBA menu screens — card detail viewer.
//!
//! The per-match menus (mode/deck select, action list, etc.) live in the
//! engine's `platform_ui::menu` module and are shared with the other embedded
//! ports. What lives here is GBA-specific menu UI that used to be inlined in
//! the binary: the card-detail popup (art + stats, dismissed with A/B/L/R).

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use rabuka_engine::game_state::GameState;

use crate::display::Display;
use crate::gba_ui::InputSource;
use crate::input::Button;
use crate::ui::CARD_ART;

/// Show the focused card's art + stats until A/B/L/R is pressed.
///
/// `art` is looked up by `card_no`; the stat line reuses the engine's
/// `card_stat_text` (mirroring the 3DS detail line) so the bin doesn't carry a
/// duplicate formatter.
pub fn show_card_detail<I: InputSource>(
    display: &mut Display,
    input: &mut I,
    gs: &GameState,
    card_no: String,
) {
    let art = CARD_ART.iter().find(|a| a.card_no == card_no);
    let mut lines: Vec<String> = Vec::new();
    if let Some(card) = gs.card_database.get_card_by_no(&card_no) {
        lines.push(format!("[{}] {}", card.card_no, card.name));
        let stat = rabuka_engine::game::platform_ui::card_stat_text(card);
        if !stat.is_empty() {
            lines.push(stat);
        }
        let abil = rabuka_engine::game::platform_ui::card_ability_text(card);
        if !abil.is_empty() {
            // wrap ability text to the detail pane width (30 - 13 = 17 cols)
            for para in abil.split('\n') {
                let mut cur = String::new();
                let mut w = 0usize;
                for ch in para.chars() {
                    let cw = if (ch as u32) < 0x1100 { 1 } else { 2 };
                    if w + cw > 17 && !cur.is_empty() {
                        lines.push(core::mem::take(&mut cur));
                        w = 0;
                    }
                    cur.push(ch);
                    w += cw;
                }
                if !cur.is_empty() {
                    lines.push(cur);
                }
            }
        }
    } else {
        lines.push(card_no);
    }
    // Paginated like 3DS detail (render.rs:437 lpp 10) — Up/Down scrolls, A/B/L/R closes
    let mut scroll = 0usize;
    const VISIBLE: usize = 8;
    display.render_card_detail(art, &lines, scroll);
    loop {
        input.poll();
        if input.just_pressed(Button::Up) && scroll > 0 {
            scroll -= 1;
            display.render_card_detail(art, &lines, scroll);
        } else if input.just_pressed(Button::Down) && scroll + VISIBLE < lines.len() {
            scroll += 1;
            display.render_card_detail(art, &lines, scroll);
        } else if input.just_pressed(Button::A)
            || input.just_pressed(Button::B)
            || input.just_pressed(Button::L)
            || input.just_pressed(Button::R)
        {
            return;
        }
        display.wait();
    }
}
