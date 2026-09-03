//! GBA menu screens — modular detail viewer.
//!
//! The per-match menus (mode/deck select, action list, etc.) live in the
//! engine's `platform_ui::menu` module and are shared with the other embedded
//! ports. What lives here is the GBA-specific detail screen backend: one
//! paginated renderer (`show_detail_screen`) over three caller-composed
//! parts — card art, header lines, body text. Every detail type is just a
//! different composition:
//!
//! - card detail (R / zone lists): art + name/stat header + ability body
//! - action detail (L): acting card art + name/stat header + full action
//!   text + ability body (composed by the engine)
//! - choice hint (L): source card art + name header + full prompt text
//!   (composed by the engine)
//!
//! The engine drives these through `PlatformUi::show_detail_screen`, so new
//! detail types never re-render — they recompose.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use rabuka_engine::game::platform_ui::{card_ability_text, card_stat_text, wrap_text};
use rabuka_engine::game_state::GameState;

use crate::display::Display;
use crate::gba_ui::InputSource;
use crate::input::Button;
use crate::ui::CARD_ART;

/// Detail pane width: the portrait takes the left 12 tile columns, text
/// starts at column 13, so 30 - 13 = 17 columns. Engine wrap keeps
/// `{{icon}}` tokens whole so they render inline.
const PANE_COLS: usize = 17;

/// Paginated detail screen over caller-composed parts: `art_card_no` art on
/// the left, `header` lines, then wrapped `body`. Up/Down scrolls (like the
/// 3DS detail view); A/B/L/R/Start closes.
pub fn show_detail_screen<I: InputSource>(
    display: &mut Display,
    input: &mut I,
    gs: &GameState,
    art_card_no: Option<&str>,
    header: &[String],
    body: &str,
) {
    let _ = gs;
    let art = art_card_no.and_then(|n| CARD_ART.iter().find(|a| a.card_no == n));
    let mut lines: Vec<String> = Vec::new();
    for h in header {
        if h.trim().is_empty() {
            continue;
        }
        lines.extend(wrap_text(h, PANE_COLS));
    }
    if !lines.is_empty() && !body.trim().is_empty() {
        lines.push(String::new());
    }
    lines.extend(wrap_text(body, PANE_COLS));
    // Fresh pool for the portrait + text: the previous screen's dead tiles
    // would otherwise pile onto this screen's demand (see reset_vram).
    display.reset_vram();
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
            || input.just_pressed(Button::Start)
        {
            // Release the portrait + text before the caller rebuilds its
            // own screen, so demands never stack across the transition.
            display.reset_vram();
            return;
        }
        display.wait();
    }
}

/// Card detail: art + `[no] name` / stat header + ability body.
pub fn show_card_detail<I: InputSource>(
    display: &mut Display,
    input: &mut I,
    gs: &GameState,
    card_no: String,
) {
    if let Some(card) = gs.card_database.get_card_by_no(&card_no) {
        let header: Vec<String> = alloc::vec![
            format!("[{}] {}", card.card_no, card.name),
            card_stat_text(card),
        ];
        show_detail_screen(
            display,
            input,
            gs,
            Some(card_no.as_str()),
            &header,
            &card_ability_text(card),
        );
    } else {
        show_detail_screen(display, input, gs, None, &[card_no], "");
    }
}
