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

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

use rabuka_engine::card::Card;
use rabuka_engine::game::platform_ui::{card_ability_text, card_detail_title, card_stat_text, PlatformUi};
use rabuka_engine::game_state::GameState;

use crate::display::Display;
use crate::gba_ui::InputSource;
use crate::input::Button;
use crate::ui::CARD_ART;

/// Card detail with custom card lookup (for deck builder without GameState).
pub fn show_card_detail_with_lookup<I: InputSource, F: Fn(&str) -> Option<&Card>>(
    display: &mut Display,
    input: &mut I,
    card_no: &str,
    lookup: F,
) {
    if let Some(card) = lookup(card_no) {
        let header: Vec<String> = alloc::vec![
            card_detail_title(card),
            card_stat_text(card),
        ];
        show_detail_screen_simple(display, input, Some(card_no), &header, &card_ability_text(card));
    } else {
        show_detail_screen_simple(display, input, None, &[card_no.to_string()], "");
    }
}

/// Paginated detail screen without GameState (for deck builder).
pub fn show_detail_screen_simple<I: InputSource>(
    display: &mut Display,
    input: &mut I,
    _art_card_no: Option<&str>,
    header: &[String],
    body: &str,
) {
    // No art lookup without GameState
    let art: Option<&crate::card_art_gen::CardArt> = None;
    let mut lines: Vec<String> = Vec::new();
    for h in header {
        if h.trim().is_empty() {
            continue;
        }
        lines.extend(Display::wrap_pane(h, PANE_COLS));
    }
    lines.extend(Display::wrap_pane(body, PANE_COLS));
    display.reset_vram();
    agb::println!("DETAIL-SIMPLE lines={} bodylen={}", lines.len(), body.len()); // TEMP
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
        } else {
            let closer = if input.just_pressed(Button::A) {
                "A"
            } else if input.just_pressed(Button::B) {
                "B"
            } else if input.just_pressed(Button::L) {
                "L"
            } else if input.just_pressed(Button::R) {
                "R"
            } else if input.just_pressed(Button::Start) {
                "Start"
            } else {
                ""
            };
            if !closer.is_empty() {
                agb::println!("DETAIL-SIMPLE close {}", closer); // TEMP
                display.reset_vram();
                return;
            }
        }
        display.wait();
    }
}

/// Detail pane width: the portrait takes the left 12 tile columns, text
/// starts at column 13, so 30 - 13 = 17 tiles. Wrapped with
/// [`Display::wrap_pane`] (exact icon/glyph tile widths), NOT the engine
/// `wrap_text` — the engine estimates `{{icon}}` tokens by bracket-label
/// width (e.g. `heart_00` = 8) while the GBA draws the 2-tile baked icon,
/// which used to shatter every stat line onto its own newline.
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
        lines.extend(Display::wrap_pane(h, PANE_COLS));
    }
    lines.extend(Display::wrap_pane(body, PANE_COLS));
    // Fresh pool for the portrait + text: the previous screen's dead tiles
    // would otherwise pile onto this screen's demand (see reset_vram).
    display.reset_vram();
    agb::println!(
        "DETAIL open art={} lines={} bodylen={}",
        art_card_no.unwrap_or("-"),
        lines.len(),
        body.len()
    ); // TEMP L/R diagnosis
    let mut scroll = 0usize;
    const VISIBLE: usize = 8;
    display.render_card_detail(art, &lines, scroll);
    agb::println!("DETAIL rendered"); // TEMP L/R diagnosis
    loop {
        input.poll();
        if input.just_pressed(Button::Up) && scroll > 0 {
            scroll -= 1;
            display.render_card_detail(art, &lines, scroll);
        } else if input.just_pressed(Button::Down) && scroll + VISIBLE < lines.len() {
            scroll += 1;
            display.render_card_detail(art, &lines, scroll);
        } else {
            let closer = if input.just_pressed(Button::A) {
                "A"
            } else if input.just_pressed(Button::B) {
                "B"
            } else if input.just_pressed(Button::L) {
                "L"
            } else if input.just_pressed(Button::R) {
                "R"
            } else if input.just_pressed(Button::Start) {
                "Start"
            } else {
                ""
            };
            if !closer.is_empty() {
                agb::println!("DETAIL close {}", closer); // TEMP L/R diagnosis
                // Release the portrait + text before the caller rebuilds its
                // own screen, so demands never stack across the transition.
                display.reset_vram();
                return;
            }
        }
        display.wait_vblank();
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
        agb::println!("CARDDETAIL {}", card_no); // TEMP
        let header: Vec<String> = alloc::vec![
            card_detail_title(card),
            card_stat_text(card),
        ];
        // The input-driven paginated viewer (art lookup included) — NOT
        // `display.show_detail_screen`, whose PlatformUi impl renders one
        // frame and returns (no input on Display): routing here flashed
        // a frame and "closed" instantly, without any portrait.
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
