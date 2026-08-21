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
        lines.push(rabuka_engine::game::platform_ui::card_stat_text(card));
    } else {
        lines.push(card_no);
    }
    display.render_card_detail(art, &lines);
    loop {
        input.poll();
        if input.just_pressed(Button::A)
            || input.just_pressed(Button::B)
            || input.just_pressed(Button::L)
            || input.just_pressed(Button::R)
        {
            return;
        }
        display.wait();
    }
}
