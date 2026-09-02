//! 3DS-style card choice grid renderer using PlatformUi trait.
//! Renders a grid of cards with art, names, and ability text.
//! Supports L/R to show board overlay, A to select, B to cancel.

extern crate alloc;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::game::platform_ui::{PlatformUi, card_ability_text};
use crate::game_state::GameState;

/// Maximum cards per page (5 cols × 2 rows = 10 cards)
const COLS: usize = 5;
const ROWS: usize = 2;
const CARDS_PER_PAGE: usize = COLS * ROWS;

/// Card display dimensions (3 tiles wide, 4 tiles tall)
const _CARD_W: i32 = 24;  // 3 tiles
const _CARD_H: i32 = 32;  // 4 tiles
const _CARD_GAP: i32 = 2;

/// Screen dimensions (GBA: 240x160, 8x8 tiles)
const _SCREEN_W: i32 = 240;
const _SCREEN_H: i32 = 160;
const _TILE_W: i32 = 8;
const _TILE_H: i32 = 8;

/// Content area
const _CONTENT_X: i32 = 4;
const _CONTENT_Y: i32 = 2;
const _CONTENT_W: i32 = _SCREEN_W - 8;
const _CONTENT_H: i32 = _SCREEN_H - 32;

/// Hints
const _HINT_BAR_Y: i32 = 144;

/// Card image info
struct CardInfo {
    card_no: String,
    name: String,
    _ability_text: String,
    _waited: bool,
}

/// Renders a card choice grid (3DS style) with board overlay support.
/// Returns the selected index, or None if cancelled.
pub fn render_card_choice_grid(
    ui: &mut dyn PlatformUi,
    gs: &GameState,
    title: &str,
    items: &[String],
    card_nos: &[String],
    allow_skip: bool,
) -> Option<usize> {
    let db = &gs.card_database;
    // Build card info list
    let mut cards: Vec<CardInfo> = Vec::new();
    for (item, card_no) in items.iter().zip(card_nos.iter()) {
        let ability_text = db
            .get_card_by_no(card_no)
            .map(|c| card_ability_text(c))
            .unwrap_or_default();
        let waited = false; // TODO: check waited state from game state
        cards.push(CardInfo {
            card_no: card_no.clone(),
            name: item.clone(),
            _ability_text: ability_text,
            _waited: waited,
        });
    }

    // Add skip option if allowed
    let skip_idx = if allow_skip {
        Some(items.len())
    } else {
        None
    };

    let total_items = items.len() + if allow_skip { 1 } else { 0 };
    let total_pages = (total_items + CARDS_PER_PAGE - 1) / CARDS_PER_PAGE;

    let mut page = 0;
    let mut sel = 0;

    loop {
        let page_start = page * CARDS_PER_PAGE;
        let page_end = (page_start + CARDS_PER_PAGE).min(total_items);
        let page_items = page_end - page_start;
        let _page_rows = (page_items + COLS - 1) / COLS;

        // Clamp selection
        if sel < page_start {
            sel = page_start;
        } else if sel >= page_end {
            sel = page_end - 1;
        }

        render_choice_page(ui, &cards, page_start, page_end, sel, title, allow_skip);

        ui.swap_buffers();

        // Poll input
        ui.poll_input();

        if ui.just_pressed_l() || ui.just_pressed_r() {
            // Show board overlay (L/R)
            ui.show_board_overlay(gs);
            // Wait for any button to dismiss
            loop {
                ui.poll_input();
                if ui.just_pressed_a() || ui.just_pressed_b() 
                    || ui.just_pressed_l() || ui.just_pressed_r()
                    || ui.just_pressed_start() {
                    break;
                }
                ui.wait_vblank();
            }
            // Redraw the choice page after overlay
            render_choice_page(ui, &cards, page_start, page_end, sel, title, allow_skip);
            ui.swap_buffers();
        } else if ui.just_pressed_a() {
            if skip_idx == Some(sel) {
                return None;
            }
            return Some(sel);
        } else if ui.just_pressed_b() || ui.just_pressed_start() {
            return None;
        } else if ui.just_pressed_up() {
            if sel >= COLS {
                sel -= COLS;
            } else if page > 0 {
                page -= 1;
                let col = (sel - page_start) % COLS;
                let new_page_start = (page - 1) * CARDS_PER_PAGE;
                let new_page_end = (new_page_start + CARDS_PER_PAGE).min(total_items);
                sel = (new_page_start + col).min(new_page_end - 1);
            }
        } else if ui.just_pressed_down() {
            if sel + COLS < page_end {
                sel += COLS;
            } else if page + 1 < total_pages {
                page += 1;
                let col = (sel - page_start) % COLS;
                let new_page_start = page * CARDS_PER_PAGE;
                let new_page_end = (page * CARDS_PER_PAGE + CARDS_PER_PAGE).min(total_items);
                sel = (new_page_start + col).min(new_page_end - 1);
            } else if ui.just_pressed_left() {
                if sel > page_start {
                    sel -= 1;
                } else if page > 0 {
                    page -= 1;
                    sel = page_end - 1;
                }
            } else if ui.just_pressed_right() {
                if sel + 1 < page_end {
                    sel += 1;
                } else if page + 1 < total_pages {
                    page += 1;
                    sel = page * CARDS_PER_PAGE;
                }
            }
        } else {
            ui.wait_vblank();
        }
    }
}

/// Render a single page of the card choice grid
fn render_choice_page(
    ui: &mut dyn PlatformUi,
    cards: &[CardInfo],
    page_start: usize,
    page_end: usize,
    sel: usize,
    title: &str,
    _allow_skip: bool,
) {
    ui.clear_screen();

    // Title bar
    ui.println(title);

    let _page_items = page_end - page_start;

    // Grid origin
    let _grid_x = 4;
    let _grid_y = 2;

    // Draw cards in grid
    // Only iterate over actual cards (not the skip option which is at total_items - 1 if allowed)
    let cards_to_draw = (page_end - page_start).min(cards.len().saturating_sub(page_start));
    for i in 0..cards_to_draw {
        let idx = page_start + i;
        let row = i / COLS;
        let col = i % COLS;

        let card = &cards[idx];
        let is_selected = (page_start + i) == sel;

        // Card position
        let x = 4 + col as i32 * (24 + 2) * 8;
        let y = 2 + row as i32 * (32 + 2) * 8; // +2 for gap

        // Draw card art
        draw_card_at(ui, x, y, card, is_selected);

        // Draw name below card
        let _name = &cards[idx].name;
        // TODO: blit text for name
    }

    // Skip option at bottom
    // TODO: draw skip option

    // Hint bar
    ui.println(&format!("  L/R: Board  A:Select  B/Start:Back"));
}

/// Draw a card at position with art and selection highlight
fn draw_card_at(ui: &mut dyn PlatformUi, x: i32, y: i32, card: &CardInfo, _selected: bool) {
    // Use PlatformUi::draw_card_image to draw the card art
    // 3 tiles wide (24px), 4 tiles tall (32px), palette 0
    ui.draw_card_image(&card.card_no, x, y, 3, 4, 0);
}