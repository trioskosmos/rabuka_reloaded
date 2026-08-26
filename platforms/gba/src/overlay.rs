//! In-game Start-menu overlay for the GBA port.
//!
//! The 3DS keeps Game Log / zone inspection / revealed cards on its second
//! screen (see `platforms/3ds/src/game/overlays.rs` `Overlay::StartMenu`).
//! The GBA has one small screen, so the same features live behind a Start
//! menu with submenus instead: MENU -> Game Log / Cards -> zone lists.
//! Everything renders through the shared text path, so `{{icon}}` tokens in
//! log entries and ability text render as baked texticons.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use rabuka_engine::game::platform_ui::{one_line, wrap_text};
use rabuka_engine::game_state::GameState;

use crate::display::{Display, COLS};
use crate::gba_ui::InputSource;
use crate::input::Button;
use crate::menu::show_card_detail;

/// List rows visible under the title line (screen is 20 text rows).
const VISIBLE: usize = 8;

/// Scrollable `items` picker. A confirms, B cancels (None).
fn select<I: InputSource>(
    display: &mut Display,
    input: &mut I,
    items: &[String],
    title: &str,
) -> Option<usize> {
    let mut sel = 0usize;
    let mut scroll = 0usize;
    loop {
        if sel < scroll {
            scroll = sel;
        }
        if sel >= scroll + VISIBLE {
            scroll = sel + 1 - VISIBLE;
        }
        display.clear();
        display.println(title);
        let end = (scroll + VISIBLE).min(items.len());
        for n in scroll..end {
            let prefix = if n == sel { " >" } else { "  " };
            display.println(&one_line(&format!("{prefix} {}", items[n]), COLS as usize));
        }
        if items.len() > end {
            display.println(&format!("  .. {} more", items.len() - end));
        }
        display.swap_buffers();
        input.poll();
        if input.just_pressed(Button::Up) {
            sel = if sel == 0 { items.len() - 1 } else { sel - 1 };
        } else if input.just_pressed(Button::Down) {
            sel = if sel + 1 == items.len() { 0 } else { sel + 1 };
        } else if input.just_pressed(Button::A) {
            return Some(sel);
        } else if input.just_pressed(Button::B) {
            return None;
        }
        display.wait();
    }
}

/// Scrollable card list for one zone; A pops the art+ability detail of the
/// focused card (like the 3DS zone viewer's card screen), B returns.
fn show_card_list<I: InputSource>(
    display: &mut Display,
    input: &mut I,
    gs: &GameState,
    title: &str,
    cards: &[i16],
) {
    let items: Vec<String> = cards
        .iter()
        .map(|&cid| {
            gs.card_database
                .get_card(cid)
                .map(|c| format!("{} {}", c.card_no, c.name))
                .unwrap_or_else(|| format!("#{}", cid))
        })
        .collect();
    if items.is_empty() {
        // Info screen, B/A closes.
        display.clear();
        display.println(title);
        display.println("(empty)");
        display.swap_buffers();
        loop {
            input.poll();
            if input.just_pressed(Button::A) || input.just_pressed(Button::B) {
                return;
            }
            display.wait();
        }
    }
    let mut sel = 0usize;
    let mut scroll = 0usize;
    loop {
        if sel < scroll {
            scroll = sel;
        }
        if sel >= scroll + VISIBLE {
            scroll = sel + 1 - VISIBLE;
        }
        display.clear();
        display.println(title);
        let end = (scroll + VISIBLE).min(items.len());
        for n in scroll..end {
            let prefix = if n == sel { " >" } else { "  " };
            display.println(&one_line(&format!("{prefix} {}", items[n]), COLS as usize));
        }
        if items.len() > end {
            display.println(&format!("  .. {} more", items.len() - end));
        }
        display.swap_buffers();
        input.poll();
        if input.just_pressed(Button::Up) {
            sel = if sel == 0 { items.len() - 1 } else { sel - 1 };
        } else if input.just_pressed(Button::Down) {
            sel = if sel + 1 == items.len() { 0 } else { sel + 1 };
        } else if input.just_pressed(Button::A) {
            let cid = cards[sel];
            if let Some(c) = gs.card_database.get_card(cid) {
                show_card_detail(display, input, gs, c.card_no.to_string());
            }
        } else if input.just_pressed(Button::B) {
            return;
        }
        display.wait();
    }
}

/// Game Log viewer: `gs.rule_log` newest-last, Up/Down scroll (mirrors the
/// 3DS GameLog overlay's offset-from-end window).
fn show_game_log<I: InputSource>(
    display: &mut Display,
    input: &mut I,
    gs: &GameState,
) {
    // Wrap each entry to screen width so the window counts real rows.
    let mut lines: Vec<String> = Vec::new();
    for entry in &gs.rule_log {
        lines.extend(wrap_text(entry, COLS as usize));
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    let mut off = lines.len().saturating_sub(VISIBLE); // start at the newest
    loop {
        display.clear();
        display.println("GAME LOG  A/B close");
        let end = (off + VISIBLE).min(lines.len());
        for l in off..end {
            display.println(&lines[l]);
        }
        display.swap_buffers();
        input.poll();
        if input.just_pressed(Button::Up) {
            off = off.saturating_sub(1);
        } else if input.just_pressed(Button::Down) && off + VISIBLE < lines.len() {
            off += 1;
        } else if input.just_pressed(Button::A) || input.just_pressed(Button::B) {
            return;
        }
        display.wait();
    }
}

/// The Cards submenu: one entry per zone, active player first. Mirrors the
/// 3DS zone-viewer entry points (hand/deck/waitroom/success/revealed).
fn run_cards_menu<I: InputSource>(
    display: &mut Display,
    input: &mut I,
    gs: &GameState,
) {
    loop {
        let me = gs.active_player();
        let you = if me.id == gs.player1.id {
            &gs.player2
        } else {
            &gs.player1
        };
        let zones: Vec<(String, Vec<i16>)> = alloc::vec![
            (
                format!("Hand ({})", me.hand.cards.len()),
                me.hand.cards.to_vec()
            ),
            (
                format!("Main Deck ({})", me.main_deck.cards.len()),
                me.main_deck.cards.to_vec()
            ),
            (
                format!("Energy ({})", me.energy_zone.cards.len()),
                me.energy_zone.cards.to_vec()
            ),
            (
                format!("Energy Deck ({})", me.energy_deck.cards.len()),
                me.energy_deck.cards.to_vec()
            ),
            (
                format!("Waitroom ({})", me.waitroom.cards.len()),
                me.waitroom.cards.to_vec()
            ),
            (
                format!("Success Live ({})", me.success_live_card_zone.cards.len()),
                me.success_live_card_zone.cards.to_vec()
            ),
            (
                format!("Exclusion ({})", me.exclusion_zone.cards.len()),
                me.exclusion_zone.cards.to_vec()
            ),
            (
                format!("Opp Waitroom ({})", you.waitroom.cards.len()),
                you.waitroom.cards.to_vec()
            ),
            (
                format!("Opp Success ({})", you.success_live_card_zone.cards.len()),
                you.success_live_card_zone.cards.to_vec()
            ),
            (
                format!(
                    "Revealed ({})",
                    gs.revealed_cards.len() + gs.revealed_cost_cards.len()
                ),
                {
                    let mut v: Vec<i16> = gs.revealed_cards.to_vec();
                    v.extend_from_slice(&gs.revealed_cost_cards);
                    v
                }
            ),
        ];
        let items: Vec<String> = zones.iter().map(|(label, _)| label.clone()).collect();
        match select(display, input, &items, "CARDS  B back") {
            Some(i) => {
                let (label, cards) = &zones[i];
                show_card_list(display, input, gs, label, cards);
            }
            None => return,
        }
    }
}

/// Blocking Start-menu overlay. Called from the board/actions frame when
/// Start is pressed; returns once the player closes the menu (the caller
/// treats the frame as consumed).
pub fn run_start_menu<I: InputSource>(
    display: &mut Display,
    input: &mut I,
    gs: &GameState,
) {
    loop {
        let items = alloc::vec![
            String::from("Game Log"),
            String::from("Cards"),
            String::from("Back"),
        ];
        match select(display, input, &items, "MENU") {
            Some(0) => show_game_log(display, input, gs),
            Some(1) => run_cards_menu(display, input, gs),
            _ => return,
        }
    }
}
