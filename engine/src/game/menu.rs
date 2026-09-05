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

use crate::game::game_setup;
use crate::game::platform_ui::{card_ability_text, card_detail_title, card_stat_text, choose_card_grid, one_line, PlatformUi, wrap_text};
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

/// Select from a list of items, starting the cursor at `initial`.
/// Returns the selected index. Start button also confirms (same as A).
/// [`select`] is this with `initial = 0`; menus that restore a previous
/// choice (e.g. the engine language picker) pass it in.
pub fn select_with_initial(
    ui: &mut dyn PlatformUi,
    items: &[&str],
    title: &str,
    initial: usize,
) -> usize {
    let mut sel: usize = initial.min(items.len().saturating_sub(1));
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

/// Select from a list of items. Returns the selected index.
/// Start button also confirms (same as A).
pub fn select(ui: &mut dyn PlatformUi, items: &[&str], title: &str) -> usize {
    select_with_initial(ui, items, title, 0)
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
/// Returns the selected index. `card_nos` (1:1 with `items`, `""` when a
/// row has no card) backs the R shortcut: the focused card opens in the
/// port's art detail viewer ([`PlatformUi::show_card_detail`], text fallback
/// on text ports), like the 3DS list + detail split. `dimmed` (1:1 with
/// `items`, skip row excluded) marks unpickable rows: shown dimmed (and
/// `" --"` suffixed on text ports) and A on them is ignored.
pub fn menu_select_with_cards(
    ui: &mut dyn PlatformUi,
    gs: &GameState,
    items: &[String],
    title: &str,
    allow_skip: bool,
    card_nos: Option<&[String]>,
    dimmed: Option<&[bool]>,
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
    let is_dimmed = |n: usize| -> bool { dimmed.and_then(|d| d.get(n).copied()).unwrap_or(false) };
    // Focused-row art lives in the graphical detail viewer (R), like the
    // 3DS list + detail split — queuing thumbnails here would trample the
    // full-width text rows on single-screen ports.
    let row_card = |n: usize| -> &str {
        if n < card_nos.len() {
            &card_nos[n]
        } else {
            ""
        }
    };
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
            // Dimmed rows read as unpickable on text-only ports too.
            if Some(n) != skip_idx && is_dimmed(n) {
                buf.push_str(" --");
            }
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
        } else if ui.just_pressed_l() {
            show_detail(ui, all_items[sel]);
        } else if ui.just_pressed_r() {
            // Art detail for card rows (graphical ports show art + stats;
            // text ports fall back to the same text viewer as L).
            let no = row_card(sel);
            if !no.is_empty() {
                ui.show_card_detail(gs, no);
            } else {
                show_detail(ui, all_items[sel]);
            }
        } else if ui.just_pressed_a() || ui.just_pressed_start() {
            if Some(sel) == skip_idx {
                return None;
            }
            // Dimmed rows can't be picked.
            if is_dimmed(sel) {
                ui.wait_vblank();
            } else {
                return Some(sel);
            }
        }
        ui.wait_vblank();
    }
}

/// Select from options with full multi-line body text (3DS auto-ability
/// queue style). `items` are (header, body, card_no) with card_no `""`
/// when the row has no card. The list anchors the selected option at the
/// top with ^+N / v+N scroll indicators, so long ability text stays
/// readable — one_line rows would truncate it away. L shows the row's full
/// text, R the card's art detail (text fallback without a card). Returns
/// the selected index, or None if skipped.
pub fn menu_select_detailed(
    ui: &mut dyn PlatformUi,
    gs: &GameState,
    items: &[(String, String, String)],
    title: &str,
    allow_skip: bool,
) -> Option<usize> {
    let mut all_items: Vec<(String, String, String)> = items.to_vec();
    let skip_idx = if allow_skip {
        all_items.push((
            String::from("[Skip]"),
            String::new(),
            String::new(),
        ));
        Some(all_items.len() - 1)
    } else {
        None
    };
    if all_items.is_empty() {
        return None;
    }
    let mut sel: usize = 0;
    let vis = ui.option_rows();
    let cols = ui.option_cols();
    // Bodies are invariant while the menu is open; wrap once.
    let bodies: Vec<Vec<String>> = all_items
        .iter()
        .map(|(_, body, _)| wrap_text(body, cols))
        .collect();
    // Line count of one item (header + body), for the v+N remainder.
    let item_lines = |n: usize| -> usize { 1 + bodies[n].len() };
    loop {
        ui.clear_screen();
        ui.println(title);
        if sel > 0 {
            ui.println(&format!("^ +{}", sel));
        }
        // Window anchored at the selected item, like the 3DS queue: the
        // selected option (and its full ability text) is always visible.
        let mut shown = 0usize;
        let mut n = sel;
        let mut leftover = 0usize;
        while n < all_items.len() {
            let need = item_lines(n);
            if shown > 0 && shown + need > vis {
                leftover += need;
                for m in n + 1..all_items.len() {
                    leftover += item_lines(m);
                }
                break;
            }
            if shown + need > vis && shown == 0 {
                // Single option taller than the window: show its head.
                let (header, _, _) = &all_items[n];
                ui.println(&format!(" > {}", header));
                for line in bodies[n].iter().take(vis - 1) {
                    ui.println(&format!("  {}", line));
                }
                leftover = bodies[n].len().saturating_sub(vis - 1);
                for m in n + 1..all_items.len() {
                    leftover += item_lines(m);
                }
                break;
            }
            let (header, _, _) = &all_items[n];
            ui.println(&format!(
                "{} {}",
                if n == sel { " >" } else { "  " },
                header
            ));
            for line in &bodies[n] {
                ui.println(&format!("  {}", line));
            }
            shown += need;
            n += 1;
        }
        if leftover > 0 {
            ui.println(&format!("v +{}", leftover));
        }
        ui.swap_buffers();
        ui.poll_input();
        if ui.just_pressed_up() {
            sel = if sel == 0 { all_items.len() - 1 } else { sel - 1 };
        } else if ui.just_pressed_down() {
            sel = if sel + 1 == all_items.len() { 0 } else { sel + 1 };
        } else if ui.just_pressed_l() {
            let (header, body, _) = &all_items[sel];
            show_detail(ui, &format!("{}\n{}", header, body));
        } else if ui.just_pressed_r() {
            let (header, body, card_no) = &all_items[sel];
            if !card_no.is_empty() {
                ui.show_card_detail(gs, card_no);
            } else {
                show_detail(ui, &format!("{}\n{}", header, body));
            }
        } else if ui.just_pressed_a() || ui.just_pressed_start() {
            if Some(sel) == skip_idx {
                return None;
            }
            return Some(sel);
        }
        ui.wait_vblank();
    }
}

/// Shared action-list picker behind [`human_turn`] (and link-mode local
/// picks): drives `ui` over `acts` and returns the picked index. Loops
/// until A is pressed — callers guarantee a non-empty list.
pub fn select_action(
    ui: &mut dyn PlatformUi,
    gs: &GameState,
    acts: &[game_setup::Action],
) -> usize {
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
            // Card tag disambiguates rows that don't name their card
            // ("Draw 2", "Pass"...). When the row already contains the
            // name, appending " [Name]" is pure redundancy, so skip it.
            let tag_str = a
                .parameters
                .as_ref()
                .and_then(|p| p.card_no.as_ref())
                .and_then(|n| gs.card_database.get_card_by_no(n))
                .filter(|c| !line.contains(c.name.as_ref()))
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
                // Action detail: the full action text plus the acting
                // card's screen (art/stats/ability), composed by the
                // port's detail renderer from these parts.
                let act = &acts[sel];
                let card = act
                    .parameters
                    .as_ref()
                    .and_then(|p| p.card_no.as_ref())
                    .and_then(|n| gs.card_database.get_card_by_no(n));
                match card {
                    Some(c) => {
                        let mut header: Vec<String> = Vec::new();
                        header.push(card_detail_title(c));
                        header.push(card_stat_text(c));
                        let ab = card_ability_text(c);
                        let body = if ab.trim().is_empty() {
                            act.description.clone()
                        } else {
                            format!("{}\n\n{}", act.description, ab)
                        };
                        ui.show_detail_screen(gs, Some(c.card_no.as_ref()), &header, &body);
                    }
                    None => {
                        ui.show_detail_screen(gs, None, &[], &act.description);
                    }
                }
            } else if ui.just_pressed_r() {
                let card = acts[sel]
                    .parameters
                    .as_ref()
                    .and_then(|p| p.card_no.as_ref())
                    .and_then(|n| gs.card_database.get_card_by_no(n));
                if let Some(c) = card {
                    let mut header: Vec<String> = Vec::new();
                    header.push(card_detail_title(c));
                    header.push(card_stat_text(c));
                    ui.show_detail_screen(
                        gs,
                        Some(c.card_no.as_ref()),
                        &header,
                        &card_ability_text(c),
                    );
                }
            } else if ui.just_pressed_a() {
                return sel;
            }
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
    let sel = select_action(ui, gs, acts);
    let _ = game_setup::execute_action(gs, &acts[sel]);
    true
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
            // Full ability text per option (3DS auto-ability queue style):
            // (header, body, card_no) rows show everything, unlike one_line
            // truncation which hid what each ability does.
            let items: Vec<(String, String, String)> = options
                .iter()
                .map(|o| {
                    let (header, card_no) = match o
                        .card_id
                        .and_then(|cid| gs.card_database.get_card(cid))
                    {
                        // Option names come from the same DB read, so the
                        // shared title helper covers both.
                        Some(c) => (card_detail_title(c), c.card_no.to_string()),
                        None => (o.card_name.clone(), String::new()),
                    };
                    (header, o.ability_text.clone(), card_no)
                })
                .collect();
            if items.is_empty() {
                TurnEngine::resume_with_choice(gs, Some(0), None).ok();
                return true;
            }
            let sel =
                menu_select_detailed(ui, gs, &items, &description, false).unwrap_or(0);
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
            // Option pool. Player zones resolve via `zone_cards`, but engine-
            // global pools don't live on a player: LookedAt (look abilities)
            // mirrors game_setup's ChoiceSelect resolution. This is also why
            // a look choice showed imageless "#i" rows before — `card_ids`
            // was empty so nothing resolved.
            let is_look_zone = zone == crate::ability::enums::Zone::LookedAt.to_str();
            let pool_ids: Vec<i16> = if is_look_zone {
                gs.looked_at_cards.to_vec()
            } else {
                zone_cards(player, &zone).to_vec()
            };
            let card_ids: &[i16] = &pool_ids;
            // Selectable set. Look zones show ALL looked-at cards with the
            // non-matching ones dimmed (mirroring the 3DS ChoiceSelect
            // `disabled` flag); other zones list matching cards only.
            let selectable = |idx: usize| -> bool {
                match filtered_indices {
                    Some(ref fi) if !fi.is_empty() => fi.contains(&idx),
                    _ => true,
                }
            };
            let show_all = is_look_zone;
            let shown: Vec<usize> = if show_all {
                (0..card_ids.len()).collect()
            } else {
                match filtered_indices {
                    Some(ref fi) => fi.iter().copied().filter(|&i| i < card_ids.len()).collect(),
                    None => (0..card_ids.len()).collect(),
                }
            };
            // Bare display names here: the grid's selected-identity line
            // already prints "> [card_no] name", so embedding the number
            // in the item would show it twice. (The multi-pick list below
            // re-attaches the number itself, since it has no identity line.)
            let name_of = |idx: usize| -> String {
                gs.card_database
                    .get_card(card_ids[idx])
                    .map(|c| c.name.to_string())
                    .unwrap_or_else(|| format!("#{}", card_ids[idx]))
            };
            let items: Vec<String> = shown.iter().map(|&i| name_of(i)).collect();

            // Card numbers for image display (3DS-style grid). These must be
            // the printable card_no strings (e.g. "BP1-005-R") that the
            // port's art lookup understands — NOT the numeric DB ids.
            // 1:1 with `items` so grid images line up with names.
            let card_nos: Vec<String> = shown
                .iter()
                .map(|&i| {
                    gs.card_database
                        .get_card(card_ids[i])
                        .map(|c| c.card_no.to_string())
                        .unwrap_or_default()
                })
                .collect();
            // Dim flags 1:1 with `items` (look zones only; elsewhere None).
            let dimmed: Vec<bool> = shown.iter().map(|&i| !selectable(i)).collect();

if count <= 1 {
                let sel = choose_card_grid(
                    ui,
                    gs,
                    &description,
                    &items,
                    &card_nos,
                    allow_skip,
                    Some(&dimmed),
                );
                match sel {
                    None => {
                        TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).ok();
                    }
                    Some(idx) => {
                        // `shown` maps the grid position back to the zone
                        // index the engine resolves (look zones show the
                        // whole pool, so this is the identity there).
                        let actual_idx = shown.get(idx).copied().unwrap_or(idx);
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
                            let mark = if selected.contains(&i) {
                                "[X]"
                            } else if dimmed.get(i).copied().unwrap_or(false) {
                                "[--]"
                            } else {
                                "[ ]"
                            };
                            // The text list has no art/identity line, so the
                            // card number goes back on (it was stripped from
                            // `items` for the grid path above).
                            match card_nos.get(i).map(|s| s.as_str()).unwrap_or("") {
                                "" => format!("{} {}", mark, name),
                                no => format!("{} {} {}", mark, no, name),
                            }
                        })
                        .collect();
                    let sel = menu_select_with_cards(
                        ui,
                        gs,
                        &display_items,
                        &description,
                        allow_skip,
                        Some(&card_nos),
                        Some(&dimmed),
                    );
                    match sel {
                        None => break,
                        Some(idx) => {
                            // Dimmed cards can't be picked.
                            if dimmed.get(idx).copied().unwrap_or(false) {
                                continue;
                            }
                            if let Some(pos) = selected.iter().position(|&x| x == idx) {
                                selected.remove(pos);
                            } else {
                                selected.push(idx);
                            }
                        }
                    }
                }
                let actual_indices: Vec<usize> =
                    selected.iter().map(|&i| shown.get(i).copied().unwrap_or(i)).collect();
                TurnEngine::resume_with_choice(gs, None, Some(actual_indices)).ok();
            }
            true
        }
        Choice::SelectTarget {
            target,
            description,
            allow_skip,
            ..
        } => {
            // Same options the web server and 3DS offer: the shared
            // pending-choice action list (Yes/No, positions, draw counts,
            // card choices...). The raw-`options` menu used to degrade to
            // "Option 1 / Option 2" whenever the ability embedded no
            // strings, hiding what each pick does.
            let acts = game_setup::generate_possible_actions(gs);
            if acts.is_empty() {
                log::debug!("[CHOICE] SelectTarget '{}' produced no actions", target);
                TurnEngine::resume_with_choice(gs, Some(-1), None).ok();
                return true;
            }
            let items: Vec<String> =
                acts.iter().map(|a| a.description.clone()).collect();
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
                    if game_setup::execute_action(gs, &acts[idx]).is_err() {
                        log::debug!(
                            "[CHOICE] SelectTarget '{}' execute failed for act {}",
                            target,
                            idx
                        );
                    }
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
            player_id,
            ..
        } => {
            let items: Vec<String> = options.iter().map(|o| o.card_name.clone()).collect();
            if items.is_empty() {
                TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).ok();
                return true;
            }
            // Card numbers for the focused-row art preview. Options were
            // built in live-zone order (see try_take_success_zone_choice),
            // so index back into the deciding player's live zone.
            let zone: &[i16] = if player_id == gs.player1.id {
                &gs.player1.live_card_zone.cards
            } else if player_id == gs.player2.id {
                &gs.player2.live_card_zone.cards
            } else {
                &[]
            };
            let card_nos: Vec<String> = options
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    zone.get(i)
                        .and_then(|cid| gs.card_database.get_card(*cid))
                        .map(|c| c.card_no.to_string())
                        .unwrap_or_default()
                })
                .collect();
            let sel = menu_select_with_cards(
                ui,
                gs,
                &items,
                &description,
                false,
                Some(&card_nos),
                None,
            )
            .unwrap_or(0);
            TurnEngine::resume_with_choice(gs, None, Some(vec![sel])).ok();
            true
        }
    }
}
