//! 3DS-style card choice grid renderer using PlatformUi trait.
//!
//! One screen: title, image grid with cursor border, selected card identity,
//! its ability preview (the 3DS ability banner, first wrapped line), skip
//! state, and a two-line hint bar. Select shows the board overlay, L shows
//! the choice hint (+ source card), R shows the cursor card detail, Start
//! opens the port's start menu (stats + zones). D-pad navigation wraps
//! around. Start is never confirm (A) nor cancel (B).

extern crate alloc;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

use crate::game::platform_ui::{PlatformUi, card_ability_text, card_stat_text, one_line, wrap_text};
use crate::game_state::GameState;

/// Stage-size cards (5x6 tiles) are the largest art that fits a shared grid
/// (the bigger detail portraits each need their own palette). Five across
/// with no gaps, one row per page, keeps the text block + art on one screen.
const COLS: usize = 5;
const ROWS: usize = 1;
const CARDS_PER_PAGE: usize = COLS * ROWS;

/// Card display dimensions (5 tiles wide, 6 tiles tall = stage size).
const CARD_TILE_W: i32 = 5;
const CARD_TILE_H: i32 = 6;
const CARD_GAP_TILES: i32 = 0;
/// Left margin: (30 - 5*5) / 2 centers the gapless row.
const GRID_X0: i32 = 2;
/// First tile row of the art grid. The text block above is capped at 6 lines
/// (12 tile rows, 0-11), so punched image holes and corner ticks never eat
/// status lines.
const GRID_Y: i32 = 13;

/// Card image info
struct CardInfo {
    card_no: String,
    name: String,
    ability_text: String,
}

/// Renders a card choice grid (3DS style) with board overlay support.
/// `dimmed` (1:1 with `items`) marks unpickable options: drawn greyed and
/// A on them is ignored. Returns the selected index, or None if cancelled.
pub fn render_card_choice_grid(
    ui: &mut dyn PlatformUi,
    gs: &GameState,
    title: &str,
    items: &[String],
    card_nos: &[String],
    allow_skip: bool,
    dimmed: Option<&[bool]>,
) -> Option<usize> {
    let db = &gs.card_database;
    // Build card info list
    let mut cards: Vec<CardInfo> = Vec::new();
    for (item, card_no) in items.iter().zip(card_nos.iter()) {
        let ability_text = db
            .get_card_by_no(card_no)
            .map(|c| card_ability_text(c))
            .unwrap_or_default();
        cards.push(CardInfo {
            card_no: card_no.clone(),
            name: item.clone(),
            ability_text,
        });
    }

    // Add skip option if allowed
    let skip_idx = if allow_skip {
        Some(items.len())
    } else {
        None
    };

    let total_items = items.len() + if allow_skip { 1 } else { 0 };
    if total_items == 0 {
        return None;
    }
    let total_pages = (total_items + CARDS_PER_PAGE - 1) / CARDS_PER_PAGE;

    // Card number of the ability currently being resolved (L shows its
    // detail screen). Cloned up front so the input loop stays simple.
    let source_card_no: Option<String> = gs
        .ability_queue
        .current_entry()
        .map(|e| e.card_no.clone())
        .filter(|n| !n.is_empty());

    let mut sel = 0;

    let is_dimmed = |idx: usize| -> bool { dimmed.and_then(|d| d.get(idx).copied()).unwrap_or(false) };

    // Fresh pool for the grid art + text: the previous screen's dead tiles
    // would otherwise pile onto this screen's demand (see reset_vram).
    ui.reset_vram();

    loop {
        let page = sel / CARDS_PER_PAGE;
        let page_start = page * CARDS_PER_PAGE;
        let page_end = (page_start + CARDS_PER_PAGE).min(total_items);

        render_choice_page(
            ui,
            &cards,
            page_start,
            page_end,
            sel,
            title,
            allow_skip,
            skip_idx,
            total_pages,
            dimmed,
        );

        ui.swap_buffers();

        // Poll input
        ui.poll_input();

        if ui.just_pressed_select() {
            // Board overlay on Select.
            ui.show_board_overlay(gs);
            // Wait for any button to dismiss
            loop {
                ui.poll_input();
                if ui.just_pressed_a()
                    || ui.just_pressed_b()
                    || ui.just_pressed_l()
                    || ui.just_pressed_r()
                    || ui.just_pressed_select()
                    || ui.just_pressed_start()
                {
                    break;
                }
                ui.wait_vblank();
            }
            // Redraw the choice page after overlay
            render_choice_page(
                ui,
                &cards,
                page_start,
                page_end,
                sel,
                title,
                allow_skip,
                skip_idx,
                total_pages,
                dimmed,
            );
            ui.swap_buffers();
        } else if ui.just_pressed_l() {
            // Choice hint detail: the full prompt text plus the source
            // card's ability (like the base card detail), composed by the
            // port's detail renderer. Works even with no source card.
            let src = source_card_no
                .as_ref()
                .and_then(|no| db.get_card_by_no(no));
            let header: Vec<String> = src
                .map(|c| alloc::vec![format!("[{}] {}", c.card_no, c.name)])
                .unwrap_or_default();
            let body = match src {
                Some(c) => {
                    let ab = card_ability_text(c);
                    if ab.trim().is_empty() {
                        title.to_string()
                    } else {
                        format!("{}\n\n{}", title, ab)
                    }
                }
                None => title.to_string(),
            };
            ui.show_detail_screen(gs, source_card_no.as_deref(), &header, &body);
        } else if ui.just_pressed_r() {
            // Detail of the cursor card (stats + ability).
            if sel < cards.len() && !cards[sel].card_no.is_empty() {
                if let Some(c) = db.get_card_by_no(&cards[sel].card_no) {
                    let header: Vec<String> = alloc::vec![
                        format!("[{}] {}", c.card_no, c.name),
                        card_stat_text(c),
                    ];
                    ui.show_detail_screen(
                        gs,
                        Some(cards[sel].card_no.as_str()),
                        &header,
                        &card_ability_text(c),
                    );
                }
            }
        } else if ui.just_pressed_a() {
            // Dimmed cards can't be picked — ignore the press.
            if skip_idx != Some(sel) && is_dimmed(sel) {
                ui.wait_vblank();
            } else {
                // Release the grid before the caller rebuilds its screen.
                ui.reset_vram();
                if skip_idx == Some(sel) {
                    return None;
                }
                return Some(sel);
            }
        } else if ui.just_pressed_b() {
            // Release the grid before the caller rebuilds its screen.
            ui.reset_vram();
            return None;
        } else if ui.just_pressed_start() {
            // Start menu over the choice (stats + zones); the choice
            // resumes when it closes.
            ui.open_start_menu(gs);
            // Redraw the choice page after the menu
            render_choice_page(
                ui,
                &cards,
                page_start,
                page_end,
                sel,
                title,
                allow_skip,
                skip_idx,
                total_pages,
                dimmed,
            );
            ui.swap_buffers();
        } else if ui.just_pressed_left() {
            sel = (sel + total_items - 1) % total_items;
        } else if ui.just_pressed_right() {
            sel = (sel + 1) % total_items;
        } else if ui.just_pressed_up() || ui.just_pressed_down() {
            // Column-preserving vertical wrap: Up from the top row lands on
            // the bottom row in the same column and vice versa. A short
            // last row clamps onto its final cell.
            let up = ui.just_pressed_up();
            let col = sel % COLS;
            let rows = (total_items + COLS - 1) / COLS;
            let row = sel / COLS;
            let new_row = if up {
                (row + rows - 1) % rows
            } else {
                (row + 1) % rows
            };
            sel = (new_row * COLS + col).min(total_items - 1);
        } else {
            ui.wait_vblank();
        }
        // NOTE: Start opens the menu above (handled in its own branch) —
        // it never confirms (A) nor cancels (B) a choice.
    }
}

/// Render a single page of the card choice grid.
///
/// Text budget is 6 lines (12 tile rows); art sits at tile row 12+ so the
/// punched image holes never eat the status lines. Every line is capped at
/// 30 columns so wrapped engine descriptions cannot overflow the budget.
fn render_choice_page(
    ui: &mut dyn PlatformUi,
    cards: &[CardInfo],
    page_start: usize,
    page_end: usize,
    sel: usize,
    title: &str,
    allow_skip: bool,
    skip_idx: Option<usize>,
    total_pages: usize,
    dimmed: Option<&[bool]>,
) {
    ui.clear_screen();

    let page_no = page_start / CARDS_PER_PAGE + 1;
    if total_pages > 1 {
        ui.println(&one_line(&format!("{} [{}/{}]", title, page_no, total_pages.max(1)), 30));
    } else {
        ui.println(&one_line(title, 30));
    }

    let cards_to_draw = (page_end - page_start).min(cards.len().saturating_sub(page_start));

    // Draw card art in a grid (coordinates are in tiles, matching the
    // PlatformUi::draw_card_image contract). `palette_index` is bit flags:
    // bit 0 = cursor, bit 1 = dimmed (greyed, unpickable) — the 3DS
    // `disabled` overlay equivalent.
    for i in 0..cards_to_draw {
        let idx = page_start + i;
        let row = i / COLS;
        let col = i % COLS;

        let card = &cards[idx];
        let is_selected = (page_start + i) == sel;
        let is_dimmed = dimmed.and_then(|d| d.get(idx).copied()).unwrap_or(false);

        // Card position in tiles (gapless row, centered via GRID_X0)
        let x = GRID_X0 + col as i32 * (CARD_TILE_W + CARD_GAP_TILES);
        let y = GRID_Y + row as i32 * (CARD_TILE_H + CARD_GAP_TILES);

        let mut flag = 0usize;
        if is_selected {
            flag |= 1;
        }
        if is_dimmed {
            flag |= 2;
        }
        ui.draw_card_image(&card.card_no, x, y, CARD_TILE_W, CARD_TILE_H, flag);
    }

    // Selected card identity (card_no + name, like the 3DS label row).
    // Dimmed cursors are marked in text too, so text-only ports (whose
    // draw_card_image is a no-op) still show the unpickable state.
    let on_skip = skip_idx == Some(sel);
    if on_skip {
        ui.println("> [Skip]");
    } else if sel < cards.len() {
        let dim_mark = if dimmed.and_then(|d| d.get(sel).copied()).unwrap_or(false) {
            " --"
        } else {
            ""
        };
        ui.println(&one_line(
            &format!("> [{}] {}{}", cards[sel].card_no, cards[sel].name, dim_mark),
            30,
        ));
    }

    // Ability preview of the selected card (3DS ability banner equivalent,
    // first wrapped line so the text budget holds).
    if !on_skip && sel < cards.len() {
        let abil = cards[sel].ability_text.replace('\n', " ");
        if !abil.trim().is_empty() {
            if let Some(line) = wrap_text(&abil, 30).into_iter().next() {
                ui.println(&line);
            }
        }
    }

    // Skip row when it sits on this page but the cursor is elsewhere.
    if allow_skip {
        if let Some(skip_i) = skip_idx {
            if !on_skip && skip_i >= page_start && skip_i < page_end {
                ui.println("  [Skip]");
            }
        }
    }

    // Hint bar: A picks, B goes back (or skips), Select shows the board,
    // L/R pop card detail screens.
    if allow_skip {
        ui.println("A:Pick B:Skip SL:Board");
    } else {
        ui.println("A:Pick B:Back SL:Board");
    }
    ui.println("L:Hint R:Card");
}
