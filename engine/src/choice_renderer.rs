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
const CARD_TILE_W: i32 = 3;
const CARD_TILE_H: i32 = 4;
const CARD_GAP_TILES: i32 = 1;

/// Screen dimensions (GBA: 240x160, 8x8 tiles)
#[allow(dead_code)]
const _SCREEN_TILE_W: i32 = 30;
#[allow(dead_code)]
const _SCREEN_TILE_H: i32 = 20;

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
        let waited = false;
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
        let _page_items = page_end - page_start;

        // Clamp selection
        if sel < page_start {
            sel = page_start;
        } else if sel >= page_end {
            sel = page_end - 1;
        }

        render_choice_page(ui, &cards, page_start, page_end, sel, title, allow_skip, skip_idx);

        ui.swap_buffers();

        // Poll input
        ui.poll_input();

        if ui.just_pressed_l() || ui.just_pressed_r() {
            // Show board overlay (L/R) - engine contract
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
            render_choice_page(ui, &cards, page_start, page_end, sel, title, allow_skip, skip_idx);
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
                let new_page_start = page * CARDS_PER_PAGE;
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
                let new_page_end = (new_page_start + CARDS_PER_PAGE).min(total_items);
                sel = (new_page_start + col).min(new_page_end - 1);
            }
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
    skip_idx: Option<usize>,
) {
    ui.clear_screen();

    // Title bar
    ui.println(title);

    let cards_to_draw = (page_end - page_start).min(cards.len().saturating_sub(page_start));
    let _grid_rows = (cards_to_draw + COLS - 1) / COLS;

    // Draw cards in grid
    for i in 0..cards_to_draw {
        let idx = page_start + i;
        let row = i / COLS;
        let col = i % COLS;

        let card = &cards[idx];
        let is_selected = (page_start + i) == sel;

        // Card position in tiles
        let x = 1 + col as i32 * (CARD_TILE_W + CARD_GAP_TILES);
        let y = 2 + row as i32 * (CARD_TILE_H + CARD_GAP_TILES + 1); // +1 for name row

        // Draw card art - use draw_card_image which handles platform-specific rendering
        ui.draw_card_image(&card.card_no, x * 8, y * 8, CARD_TILE_W, CARD_TILE_H, 0);

        // Draw selection indicator and name on text line below card
        if is_selected {
            ui.println(&format!(" > {}", card.name));
        } else {
            ui.println(&format!("   {}", card.name));
        }
    }

    // Draw skip option if on this page
    if let Some(skip_i) = skip_idx {
        if skip_i >= page_start && skip_i < page_end {
            let is_selected = skip_i == sel;
            if is_selected {
                ui.println(&format!(" > [Skip]"));
            } else {
                ui.println(&format!("   [Skip]"));
            }
        }
    }

    // Hint bar at bottom
    ui.println(&format!("  L/R: Board  A:Select  B/Start:Back"));
}