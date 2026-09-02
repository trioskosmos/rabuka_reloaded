//! 3DS-style card choice grid renderer for GBA.
//! Renders a grid of cards with art, names, and ability text.
//! Supports L/R to show board overlay, A to select, B to cancel.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use rabuka_engine::card::HeartColor;
use rabuka_engine::game::platform_ui::{one_line, wrap_text};
use rabuka_engine::game_state::GameState;

use crate::board::BoardFrame;
use crate::card_art_gen::{CARD_FRONTS, STAGE_FRONTS, LIVE_FRONTS, WAITED_FRONTS};
use crate::display::Display;
use crate::gba_ui::InputSource;
use crate::input::Button;
use crate::texticons_gen::{TEXTICON_GLYPHS, TEXTICON_TILES};
use crate::{FONT_GLYPHS, FONT_TILES, BOARD_UI, MASTER_PAL};

use agb::display::tiled::{
    RegularBackground, RegularBackgroundSize, TileEffect, TileFormat, TileSet, TileSetting,
};
use agb::display::{Graphics, Priority, Rgb15, Rgb};
use agb::display::tiled::{TileSetting, TileSet};

/// Maximum cards per page (5 cols × 2 rows = 10 cards)
const COLS: usize = 5;
const ROWS: usize = 2;
const CARDS_PER_PAGE: usize = COLS * ROWS;

/// Card display dimensions
const CARD_W: i32 = 24;  // 3 tiles
const CARD_H: i32 = 32;  // 4 tiles
const CARD_GAP: i32 = 2;

/// Screen dimensions
const SCREEN_W: i32 = 240;
const SCREEN_H: i32 = 160;
const TILE_W: i32 = 8;
const TILE_H: i32 = 8;

/// Content area
const CONTENT_X: i32 = 4;
const CONTENT_Y: i32 = 2;
const CONTENT_W: i32 = SCREEN_W - 8;
const CONTENT_H: i32 = SCREEN_H - 32;

/// Hints
const HINT_BAR_Y: i32 = 144;

/// Card image info
struct CardInfo {
    card_no: String,
    name: String,
    ability_text: String,
    front_tiles: &'static [u8],
    waited: bool,
}

/// Renders a card choice grid (3DS style) with board overlay support.
pub fn render_card_choice_grid<I: crate::gba_ui::InputSource>(
    display: &mut Display,
    input: &mut I,
    gs: &GameState,
    title: &str,
    items: &[String],
    card_nos: &[String],
    allow_skip: bool,
) -> Option<usize> {
    // Build card info list
    let mut cards: Vec<CardInfo> = Vec::new();
    for (item, card_no) in items.iter().zip(card_nos.iter()) {
        let ability_text = if let Some(card) = gs.card_database.get_card(card_no) {
            card.ability.as_deref().unwrap_or("").to_string()
        } else {
            String::new()
        };
        let waited = false; // TODO: check waited state from game state
        cards.push(CardInfo {
            card_no: card_no.clone(),
            name: item.clone(),
            ability_text,
            front_tiles: get_front_tiles(card_no, waited),
            waited,
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
        let page_rows = (page_items + COLS - 1) / COLS;

        // Clamp selection
        if sel < page_start {
            sel = page_start;
        } else if sel >= page_end {
            sel = page_end - 1;
        }

        render_page(display, &cards, page_start, page_end, sel, title, allow_skip);

        display.swap_buffers();

        // Poll input
        let mut input_copy = input; // Need to handle input properly
        // Actually, we need to poll the input passed in
        // For now, use a simpler approach

        // Wait for vblank and poll
        display.wait();
        input.poll();

        if input.just_pressed(Button::Left) {
            if sel > page_start {
                sel -= 1;
            } else if page > 0 {
                page -= 1;
                sel = page_end - 1;
            }
        } else if input.just_pressed(Button::Right) {
            if sel + 1 < page_end {
                sel += 1;
            } else if page + 1 < total_pages {
                page += 1;
                sel = page * CARDS_PER_PAGE;
            }
        } else if input.just_pressed(Button::Up) {
            if sel >= COLS {
                sel -= COLS;
            } else if page > 0 {
                page -= 1;
                // Try to keep same column
                let col = (sel - page_start) % COLS;
                let new_page_start = (page - 1) * CARDS_PER_PAGE;
                let new_page_end = (new_page_start + CARDS_PER_PAGE).min(cards.len() + if allow_skip { 1 } else { 0 });
                sel = (new_page_start + col).min(new_page_end - 1);
            }
        } else if input.just_pressed(Button::Down) {
            if sel + COLS < page_end {
                sel += COLS;
            } else if page + 1 < total_pages {
                page += 1;
                let col = (sel - page_start) % COLS;
                let new_page_start = page * CARDS_PER_PAGE;
                let new_page_end = (page * CARDS_PER_PAGE + CARDS_PER_PAGE).min(cards.len() + if allow_skip { 1 } else { 0 });
                sel = (new_page_start + col).min(new_page_end - 1);
            }
        } else if input.just_pressed(Button::L) || input.just_pressed(Button::R) {
            // Show board overlay
            show_board_overlay(display, &GameState { /* need gs */ });
        } else if input.just_pressed(Button::A) {
            if skip_idx == Some(sel) {
                return None;
            }
            return Some(sel);
        } else if input.just_pressed(Button::B) || input.just_pressed(Button::Start) {
            return None;
        }
    }
}

/// Render a single page of cards
fn render_page(
    display: &mut Display,
    cards: &[CardInfo],
    page_start: usize,
    page_end: usize,
    sel: usize,
    title: &str,
    allow_skip: bool,
) {
    display.clear();

    // Title
    display.println(title);

    let page_start = page_start;
    let page_end = page_end;
    let page_items = page_end - page_start;
    let page_rows = (page_items + COLS - 1) / COLS;

    // Grid origin
    let grid_x = CONTENT_X;
    let grid_y = CONTENT_Y;

    // Draw cards in grid
    for i in 0..(page_end - page_start) {
        let idx = page_start + i;
        let row = i / COLS;
        let col = i % COLS;

        let card = &cards[idx];
        let is_selected = (page_start + i) == sel;

        // Card position
        let x = CONTENT_X + col as i32 * (CARD_W + CARD_GAP) * TILE_W;
        let y = CONTENT_Y + row as i32 * (CARD_H + 2) * TILE_H; // +2 for gap

        // Draw card art
        draw_card_at(display, x, y, card, true);

        // Draw name below card
        let name_y = y + CARD_H * 8 + 2; // below card
        let name = if idx == page_start + cards.len() { // skip
            "[Skip]"
        } else {
            &cards[idx].name
        };
        // Draw name text (simplified)
        // TODO: blit text

        // Selection highlight
        if (page_start + i) == sel {
            // Draw selection border
            // TODO
        }
    }

    // Skip option at bottom
    if allow_skip {
        let skip_y = CONTENT_Y + ((cards.len() + COLS - 1) / COLS) as i32 * (CARD_H + 2) * TILE_H + 8;
        display.println(&one_line("  [Skip]", 30));
    }

    // Hint bar
    display.println(&one_line("  L/R: Board  A:Select  B/Start:Back", 30));
}

/// Draw a card at position
fn draw_card_at(display: &mut Display, x: i32, y: i32, card: &CardInfo, _selected: bool) {
    // This needs access to display's graphics and backgrounds
    // For now, placeholder
}

fn get_front_tiles(card_no: &str, waited: bool) -> &'static [u8] {
    let fronts = if waited { WAITED_FRONTS } else { CARD_FRONTS };
    fronts.iter()
        .find(|f| f.card_no == card_no)
        .map(|f| f.tiles)
        .unwrap_or(&[])
}