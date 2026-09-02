//! Console menu / navigation front-end.
//!
//! Every scrollable list, the per-turn action menu, the choice prompts, and
//! the result viewer live here. They are port-agnostic: each takes a
//! `&mut dyn PlatformUi` (or a generic `U: PlatformUi`) and drives it. The
//! low-level text/card formatting they rely on lives in `platform_ui`, and the
//! shared game loop that calls these lives in `match_runner`.

#[cfg(feature = "no_std")]
use alloc::format;
#[cfg(feature = "no_std")]
use alloc::string::{String, ToString};
#[cfg(feature = "no_std")]
use alloc::vec::Vec;
#[cfg(not(feature = "no_std"))]
use std::format;

use crate::ability::types::Choice;
use crate::ability::util::zone_cards;
use crate::card::Card;

use crate::game::game_setup;
use crate::game::platform_ui::{card_ability_text, card_stat_text, one_line, PlatformUi, wrap_text};
use crate::game_state::GameState;
use crate::turn::TurnEngine;

/// Scrollable full-text viewer. Shows `lines` in a window; Up/Down scroll,
/// and A/B/L/R close it back to the menu without disturbing the option list's
/// own scroll position.
fn show_lines(ui: &mut dyn PlatformUi, lines: &[String]) {
    let mut off = 0usize;
    const H: usize = 8; // 9 screen rows, one held for a hint bar
    loop {
        ui.clear_screen();
        ui.println("A/B/Start close, Up/Down scroll");
        let end = (off + H).min(lines.len());
        for l in off..end {
            ui.println(&lines[l]);
        }
        if lines.len() > end {
            ui.println(&format!("  .. {} more", lines.len() - end));
        }
        ui.swap_buffers();
        ui.poll_input();
        if ui.just_pressed_up() {
            off = off.saturating_sub(1);
        } else if ui.just_pressed_down() && off + H < lines.len() {
            off += 1;
        } else if ui.just_pressed_a()
            || ui.just_pressed_b()
            || ui.just_pressed_l()
            || ui.just_pressed_r()
            || ui.just_pressed_start()
        {
            return;
        }
        ui.wait_vblank();
    }
}

/// Wrap `text` and show it in a scrollable viewer (L: full action/ability text).
fn show_detail(ui: &mut dyn PlatformUi, text: &str) {
    let cols = ui.option_cols();
    let lines = wrap_text(text, cols);
    show_lines(ui, &lines);
}

/// Scrollable card detail (R): header, stat line, then the ability text.
fn show_card_stats(ui: &mut dyn PlatformUi, card: &Card) {
    let cols = ui.option_cols();
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("[{}] {}", card.card_no, card.name));
    lines.push(card_stat_text(card));
    let ab = card_ability_text(card);
    if !ab.is_empty() {
        lines.extend(wrap_text(&ab, cols));
    }
    show_lines(ui, &lines);
}

/// Show the game result screen and wait for a button press.
pub fn show_result(ui: &mut dyn PlatformUi, gs: &GameState) {
    loop {
        ui.clear_screen();
        ui.println("=== GAME OVER ===");
        ui.println(&format!("{:?}", gs.game_result));
        ui.println(&format!(
            "P1 success:{} wait:{}",
            gs.player1.success_live_card_zone.cards.len(),
            gs.player1.waitroom.cards.len()
        ));
        ui.println(&format!(
            "P2 success:{} wait:{}",
            gs.player2.success_live_card_zone.cards.len(),
            gs.player2.waitroom.cards.len()
        ));
        ui.println("Press A to continue");
        ui.poll_input();
        if ui.just_pressed_a() || ui.just_pressed_start() {
            break;
        }
        ui.wait_vblank();
    }
}

/// Select from a list of items. Returns the selected index.
/// Start button also confirms (same as A).
pub fn select(ui: &mut dyn PlatformUi, items: &[&str], title: &str) -> usize {
    let mut sel: usize = 0;
    let mut scroll: usize = 0;
    let vis = ui.option_rows();
    let cols = ui.option_cols();
    // Same hoist as menu_select: wrap once, swap the cursor per frame.
    let rows: Vec<String> = items
        .iter()
        .map(|item| one_line(&format!("   {item}"), cols))
        .collect();
    loop {
        if sel < scroll {
            scroll = sel;
        }
        if sel >= scroll + vis {
            scroll = sel + 1 - vis;
        }
        ui.clear_screen();
        ui.println(title);
        let end = (scroll + vis).min(items.len());
        let mut buf = String::new();
        for n in scroll..end {
            buf.clear();
            buf.push_str(if n == sel { " >" } else { "  " });
            buf.push_str(&rows[n][2..]);
            ui.println(&buf);
        }
        if items.len() > end {
            ui.println(&format!("  .. {} more", items.len() - end));
        }
        ui.swap_buffers();
        ui.poll_input();
        if ui.just_pressed_up() {
            sel = if sel == 0 { items.len() - 1 } else { sel - 1 };
        } else if ui.just_pressed_down() {
            sel = if sel + 1 == items.len() { 0 } else { sel + 1 };
        } else if ui.just_pressed_l() || ui.just_pressed_r() {
            show_detail(ui, items[sel]);
        } else if ui.just_pressed_a() || ui.just_pressed_start() {
            return sel;
        }
        ui.wait_vblank();
    }
}

/// Select from a list of string items with optional skip. Returns None if skipped.
pub fn menu_select(
    ui: &mut dyn PlatformUi,
    items: &[String],
    title: &str,
    allow_skip: bool,
) -> Option<usize> {
    let mut all_items: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
    let skip_idx = if allow_skip {
        all_items.push("[Skip]");
        Some(all_items.len() - 1)
    } else {
        None
    };
    let mut sel: usize = 0;
    let mut scroll: usize = 0;
    let vis = ui.option_rows();
    let cols = ui.option_cols();
    // Rows are invariant while the menu is open; wrap them once (the 3-space
    // base keeps the original "prefix + space + item" layout) and per frame
    // only swap the cursor onto a reused buffer.
    let rows: Vec<String> = all_items
        .iter()
        .map(|item| one_line(&format!("   {item}"), cols))
        .collect();
    loop {
        if sel < scroll {
            scroll = sel;
        }
        if sel >= scroll + vis {
            scroll = sel + 1 - vis;
        }
        ui.clear_screen();
        ui.println(title);
        let end = (scroll + vis).min(all_items.len());
        let mut buf = String::new();
        for n in scroll..end {
            buf.clear();
            buf.push_str(if n == sel { " >" } else { "  " });
            buf.push_str(&rows[n][2..]);
            ui.println(&buf);
        }
        if all_items.len() > end {
            ui.println(&format!("  .. {} more", all_items.len() - end));
        }
        ui.swap_buffers();
        ui.poll_input();
        if ui.just_pressed_up() {
            sel = if sel == 0 { all_items.len() - 1 } else { sel - 1 };
        } else if ui.just_pressed_down() {
            sel = if sel + 1 == all_items.len() { 0 } else { sel + 1 };
        } else if ui.just_pressed_l() || ui.just_pressed_r() {
            show_detail(ui, &all_items[sel]);
        } else if ui.just_pressed_a() || ui.just_pressed_start() {
            if Some(sel) == skip_idx {
                return None;
            }
            return Some(sel);
        }
        ui.wait_vblank();
    }
}

/// Select from a list of string items with optional card images.
/// Returns the selected index. If `card_nos` is provided, draws card images
/// next to each option using `ui.draw_card_image`.
pub fn menu_select_with_cards(
    ui: &mut dyn PlatformUi,
    items: &[String],
    title: &str,
    allow_skip: bool,
    card_nos: Option<&[String]>,
) -> Option<usize> {
    let mut all_items: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
    let skip_idx = if allow_skip {
        all_items.push("[Skip]");
        Some(all_items.len() - 1)
    } else {
        None
    };
    let mut sel: usize = 0;
    let mut scroll: usize = 0;
    let vis = ui.option_rows();
    let cols = ui.option_cols();
    // Rows are invariant while the menu is open; wrap them once.
    let rows: Vec<String> = all_items
        .iter()
        .map(|item| one_line(&format!("   {item}"), cols))
        .collect();
    let card_nos = card_nos.unwrap_or(&[]);
    let has_images = !card_nos.is_empty();
    let _img_cols = 3; // 3 tiles wide = 24px
    let _img_rows = 4; // 4 tiles tall = 32px
    let _img_x = 26; // Right side of screen (30 - 3 - 1)
    loop {
        if sel < scroll {
            scroll = sel;
        }
        if sel >= scroll + vis {
            scroll = sel + 1 - vis;
        }
        ui.clear_screen();
        ui.println(title);
        let end = (scroll + vis).min(all_items.len());
        let mut buf = String::new();
        for n in scroll..end {
            buf.clear();
            buf.push_str(if n == sel { " >" } else { "  " });
            buf.push_str(&rows[n][2..]);
            ui.println(&buf);
        }
        // Draw card images for visible items
        if has_images {
            for (i, n) in (scroll..end).enumerate() {
                if n < card_nos.len() {
                    let img_y = 2 + i as i32 * 2; // 2 tile rows per line, plus header
                    ui.draw_card_image(&card_nos[n], 26, img_y, 3, 4, 0);
                }
            }
        }
        if all_items.len() > end {
            ui.println(&format!("  .. {} more", all_items.len() - end));
        }
        ui.swap_buffers();
        ui.poll_input();
        if ui.just_pressed_up() {
            sel = if sel == 0 { all_items.len() - 1 } else { sel - 1 };
        } else if ui.just_pressed_down() {
            sel = if sel + 1 == all_items.len() { 0 } else { sel + 1 };
        } else if ui.just_pressed_l() || ui.just_pressed_r() {
            show_detail(ui, all_items[sel]);
        } else if ui.just_pressed_a() || ui.just_pressed_start() {
            if Some(sel) == skip_idx {
                return None;
            }
            return Some(sel);
        }
        ui.wait_vblank();
    }
}

/// Scrollable action list for a human player's turn.
/// Returns true if an action was executed, false if the turn was passed.
pub fn human_turn(
    ui: &mut dyn PlatformUi,
    gs: &mut GameState,
    acts: &[game_setup::Action],
) -> bool {
    let mut sel = 0;
    let mut scroll = 0;
    let vis = ui.option_rows().min(9);
    let cols = ui.option_cols();

    // Nothing on this screen changes between frames except the cursor: gs is
    // read-only until an action executes (which returns). Building every line
    // via format! + DB lookups *per poll* was the dominant cost on interpreted
    // console targets (Dreamcast/WAMR), so construct it all once up front.
    let header_turn = format!("Turn {} | {:?}", gs.turn_number, gs.current_phase);
    let p1 = &gs.player1;
    let p2 = &gs.player2;
    let is_p1 = gs.active_player().id == "p1";
    let tag = |a: bool| if a { ">>" } else { "  " };
    let header_p1 = format!(
        "{} P1 h:{} e:{} dk:{}",
        tag(is_p1),
        p1.hand.cards.len(),
        p1.energy_zone.active_count(),
        p1.main_deck.cards.len()
    );
    let header_p2 = format!(
        "{} P2 h:{} e:{} dk:{}",
        tag(!is_p1),
        p2.hand.cards.len(),
        p2.energy_zone.active_count(),
        p2.main_deck.cards.len()
    );
    // Rows carry the neutral 2-char prefix through one_line so the wrapped
    // width matches the original exactly; per frame only bytes [0..2] differ,
    // swapped onto a reused buffer instead of re-running fmt machinery.
    let rows: Vec<String> = acts
        .iter()
        .map(|a| {
            let line = a.description.lines().next().unwrap_or("");
            let tag_str = a
                .parameters
                .as_ref()
                .and_then(|p| p.card_no.as_ref())
                .and_then(|n| gs.card_database.get_card_by_no(n))
                .map(|c| format!(" [{}]", c.name))
                .unwrap_or_default();
            one_line(&format!("  {line}{tag_str}"), cols)
        })
        .collect();
    let action_cards: Vec<String> = {
        let mut v: Vec<String> = Vec::new();
        for a in acts {
            if let Some(n) = a.parameters.as_ref().and_then(|p| p.card_no.as_ref()) {
                if !v.contains(n) {
                    v.push(n.clone());
                }
            }
        }
        v
    };
    loop {
        ui.clear_screen();
        ui.println(&header_turn);
        ui.println(&header_p1);
        ui.println(&header_p2);
        if sel < scroll {
            scroll = sel;
        }
        if sel >= scroll + vis {
            scroll = sel + 1 - vis;
        }
        let end = (scroll + vis).min(acts.len());
        let mut buf = String::new();
        for a in scroll..end {
            buf.clear();
            buf.push_str(if a == sel { " >" } else { "  " });
            buf.push_str(&rows[a]);
            ui.println(&buf);
        }
        if acts.len() > end {
            ui.println(&format!("  .. {} more", acts.len() - end));
        }
        ui.set_actionable_cards(&action_cards);
        ui.set_selected_action(
            acts[sel].description.lines().next().unwrap_or(""),
            sel,
            acts.len(),
        );
        let consumed = ui.render_board(gs);
        ui.poll_input();
        if !consumed {
            if ui.just_pressed_down() {
                sel = if sel + 1 == acts.len() { 0 } else { sel + 1 };
            } else if ui.just_pressed_up() {
                sel = if sel == 0 { acts.len() - 1 } else { sel - 1 };
            } else if ui.just_pressed_l() {
                show_detail(ui, &acts[sel].description);
            } else if ui.just_pressed_r() {
                let card = acts[sel]
                    .parameters
                    .as_ref()
                    .and_then(|p| p.card_no.as_ref())
                    .and_then(|n| gs.card_database.get_card_by_no(n));
                if let Some(c) = card {
                    show_card_stats(ui, c);
                }
            } else if ui.just_pressed_a() {
                let _ = game_setup::execute_action(gs, &acts[sel]);
                return true;
            }
        }
        ui.wait_vblank();
    }
}

/// Handle a pending player choice (SelectCard, SelectTarget, etc).
/// Returns true if the choice was handled.
pub fn handle_choice(ui: &mut dyn PlatformUi, gs: &mut GameState) -> bool {
    let choice = match gs.get_pending_choice() {
        Some(c) => c.clone(),
        None => return true,
    };

    match choice {
        Choice::SelectAutoAbility {
            options,
            description,
            ..
        } => {
            let items: Vec<String> = options
                .iter()
                .map(|o| format!("{}: {}", o.card_name, o.ability_text))
                .collect();
            if items.is_empty() {
                TurnEngine::resume_with_choice(gs, Some(0), None).ok();
                return true;
            }
            let sel = menu_select(ui, &items, &description, false).unwrap_or(0);
            TurnEngine::resume_with_choice(gs, Some(sel as i16), None).ok();
            true
        }
        Choice::SelectCard {
            zone,
            count,
            allow_skip,
            target_player_id,
            description,
            filtered_indices,
            ..
        } => {
            let player = target_player_id
                .as_ref()
                .and_then(|pid| {
                    if pid == &gs.player1.id {
                        Some(&gs.player1)
                    } else if pid == &gs.player2.id {
                        Some(&gs.player2)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| gs.active_player());
            let card_ids = zone_cards(player, &zone);
            let items: Vec<String> = match filtered_indices {
                Some(ref indices) => indices
                    .iter()
                    .map(|&i| {
                        if i < card_ids.len() {
                            gs.card_database
                                .get_card(card_ids[i])
                                .map(|c| format!("{} {}", c.card_no, c.name))
                                .unwrap_or_else(|| format!("#{}", card_ids[i]))
                        } else {
                            format!("#{}", i)
                        }
                    })
                    .collect(),
                None => card_ids
                    .iter()
                    .map(|cid| {
                        gs.card_database
                            .get_card(*cid)
                            .map(|c| format!("{} {}", c.card_no, c.name))
                            .unwrap_or_else(|| format!("#{}", cid))
                    })
                    .collect(),
            };

            // Extract card_nos for image display
            let card_nos: Vec<String> = match filtered_indices {
                Some(ref indices) => {
                    if card_ids.is_empty() {
                        Vec::new()
                    } else {
                        indices
                            .iter()
                            .filter(|&&i| i < card_ids.len())
                            .map(|&i| card_ids[i].to_string())
                            .collect()
                    }
                }
                None => card_ids.iter().map(|cid| cid.to_string()).collect(),
            };

if count <= 1 {
                let sel = crate::choice_renderer::render_card_choice_grid(
                    ui,
                    gs,
                    &description,
                    &items,
                    &card_nos,
                    allow_skip,
                );
                match sel {
                    None => {
                        TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).ok();
                    }
Some(idx) => {
                        let actual_idx = filtered_indices.as_ref().map(|fi| fi[idx]).unwrap_or(idx);
                        TurnEngine::resume_with_choice(gs, None, Some(vec![actual_idx])).ok();
                    }
                }
            } else {
                // Multi-select: A toggles the highlighted card in/out of the
                // selection ([X] marker follows). The loop ends when exactly
                // `count` distinct cards are chosen, or immediately via
                // allow_skip (B). Previously, pressing A on an
                // already-selected card pushed nothing and left the loop
                // condition unchanged -> unavoidable infinite menu (the
                // "stuck in mulligan" freeze).
                let mut selected: Vec<usize> = Vec::new();
                while selected.len() < count.min(items.len()) {
                    let display_items: Vec<String> = items
                        .iter()
                        .enumerate()
                        .map(|(i, name)| {
                            if selected.contains(&i) {
                                format!("[X] {}", name)
                            } else {
                                format!("[ ] {}", name)
                            }
                        })
                        .collect();
                    let sel = menu_select_with_cards(ui, &display_items, &description, allow_skip, Some(&card_nos));
                    match sel {
                        None => break,
                        Some(idx) => {
                            if let Some(pos) = selected.iter().position(|&x| x == idx) {
                                selected.remove(pos);
                            } else {
                                selected.push(idx);
                            }
                        }
                    }
                }
                let actual_indices: Vec<usize> = match filtered_indices {
                    Some(ref fi) => selected.iter().map(|&i| fi[i]).collect(),
                    None => selected,
                };
                TurnEngine::resume_with_choice(gs, None, Some(actual_indices)).ok();
            }
            true
        }
        Choice::SelectTarget {
            target,
            options,
            description,
            allow_skip,
            ..
        } => {
            let items: Vec<String> = match options {
                Some(ref opts) if !opts.is_empty() => opts.clone(),
                _ => (0..2).map(|i| format!("Option {}", i + 1)).collect(),
            };
            let sel = menu_select(ui, &items, &description, allow_skip);
            match sel {
                None => match target.as_str() {
                    "choice" | "choice_string" | "conditional_optional" => {
                        TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).ok();
                    }
                    _ => {
                        TurnEngine::resume_with_choice(gs, Some(-1), None).ok();
                    }
                },
                Some(idx) => {
                    TurnEngine::resume_with_choice(gs, Some(idx as i16), None).ok();
                }
            };
            true
        }
        Choice::SelectPosition {
            description,
            allow_skip,
            ..
        } => {
            let items: Vec<String> = ["Left", "Center", "Right"]
                .iter()
                .map(|s| s.to_string())
                .collect();
            let sel = menu_select(ui, &items, &description, allow_skip);
            match sel {
                None => TurnEngine::resume_with_choice(gs, Some(-1), None).ok(),
                Some(idx) => TurnEngine::resume_with_choice(gs, Some(idx as i16), None).ok(),
            };
            true
        }
        Choice::SelectHeartColor {
            options,
            description,
            ..
        } => {
            let sel = menu_select(ui, &options, &description, false).unwrap_or(0);
            TurnEngine::resume_with_choice(gs, Some(sel as i16), None).ok();
            true
        }
        Choice::SelectHeartType {
            options,
            description,
            ..
        } => {
            let sel = menu_select(ui, &options, &description, false).unwrap_or(0);
            TurnEngine::resume_with_choice(gs, Some(sel as i16), None).ok();
            true
        }
        Choice::SelectLiveSuccess {
            options,
            description,
            ..
        } => {
            let items: Vec<String> = options.iter().map(|o| o.card_name.clone()).collect();
            if items.is_empty() {
                TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).ok();
                return true;
            }
            let sel = menu_select(ui, &items, &description, false).unwrap_or(0);
            TurnEngine::resume_with_choice(gs, None, Some(vec![sel])).ok();
            true
        }
    }
}
