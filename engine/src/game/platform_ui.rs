#[cfg(feature = "no_std")]
use alloc::format;
#[cfg(feature = "no_std")]
use alloc::string::String;
#[cfg(feature = "no_std")]
use alloc::vec::Vec;
#[cfg(not(feature = "no_std"))]
use std::format;

use crate::game::game_setup;
use crate::game_state::GameState;
use crate::turn::TurnEngine;

pub trait PlatformUi {
    fn clear_screen(&mut self);
    fn println(&mut self, text: &str);
    fn swap_buffers(&mut self);
    fn poll_input(&mut self);
    fn just_pressed_a(&self) -> bool;
    fn just_pressed_b(&self) -> bool;
    fn just_pressed_up(&self) -> bool;
    fn just_pressed_down(&self) -> bool;
    fn just_pressed_start(&self) -> bool;
    fn wait_vblank(&mut self);
}

/// AI turn: pick a random action and execute it.
pub fn ai_turn(gs: &mut GameState, acts: &[game_setup::Action]) -> bool {
    let _ = game_setup::execute_action(gs, &acts[crate::rng::rand_range(acts.len())]);
    true
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
pub fn select(ui: &mut dyn PlatformUi, items: &[&str], title: &str) -> usize {
    let mut sel: usize = 0;
    loop {
        ui.clear_screen();
        ui.println(title);
        for (n, item) in items.iter().enumerate() {
            let prefix = if n == sel { " >" } else { "  " };
            ui.println(&format!("{prefix} {item}"));
        }
        ui.swap_buffers();
        ui.poll_input();
        if ui.just_pressed_up() {
            sel = sel.saturating_sub(1);
        } else if ui.just_pressed_down() {
            if sel + 1 < items.len() {
                sel += 1;
            }
        } else if ui.just_pressed_a() || ui.just_pressed_b() {
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
    loop {
        ui.clear_screen();
        ui.println(title);
        for (n, item) in all_items.iter().enumerate() {
            let prefix = if n == sel { " >" } else { "  " };
            ui.println(&format!("{prefix} {item}"));
        }
        ui.swap_buffers();
        ui.poll_input();
        if ui.just_pressed_up() {
            sel = sel.saturating_sub(1);
        } else if ui.just_pressed_down() {
            if sel + 1 < all_items.len() {
                sel += 1;
            }
        } else if ui.just_pressed_a() {
            if Some(sel) == skip_idx {
                return None;
            }
            return Some(sel);
        } else if ui.just_pressed_b() {
            return None;
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
    const VIS: usize = 10;
    loop {
        ui.clear_screen();
        ui.println(&format!("Turn {} | {:?}", gs.turn_number, gs.current_phase));
        let p1 = &gs.player1;
        let p2 = &gs.player2;
        let is_p1 = gs.active_player().id == "p1";
        let tag = |a: bool| if a { ">>" } else { "  " };
        ui.println(&format!(
            "{} P1 h:{} e:{} dk:{}",
            tag(is_p1),
            p1.hand.cards.len(),
            p1.energy_zone.active_count(),
            p1.main_deck.cards.len()
        ));
        ui.println(&format!(
            "{} P2 h:{} e:{} dk:{}",
            tag(!is_p1),
            p2.hand.cards.len(),
            p2.energy_zone.active_count(),
            p2.main_deck.cards.len()
        ));
        if sel < scroll {
            scroll = sel;
        }
        if sel >= scroll + VIS {
            scroll = sel + 1 - VIS;
        }
        let end = (scroll + VIS).min(acts.len());
        for a in scroll..end {
            let prefix = if a == sel { " >" } else { "  " };
            let line = acts[a].description.lines().next().unwrap_or("");
            let tag_str = match &acts[a].parameters {
                Some(p) => p
                    .card_no
                    .as_ref()
                    .and_then(|n| gs.card_database.get_card_by_no(n))
                    .map(|c| format!(" [{}]", c.name))
                    .unwrap_or_default(),
                None => String::new(),
            };
            ui.println(&format!("{prefix}[{a}] {line}{tag_str}"));
        }
        if acts.len() > end {
            ui.println(&format!("  .. {} more", acts.len() - end));
        }
        ui.swap_buffers();
        ui.poll_input();
        if ui.just_pressed_down() {
            sel = (sel + 1).min(acts.len() - 1);
        } else if ui.just_pressed_up() {
            sel = sel.saturating_sub(1);
        } else if ui.just_pressed_a() {
            let _ = game_setup::execute_action(gs, &acts[sel]);
            return true;
        } else if ui.just_pressed_b() {
            return false;
        }
        ui.wait_vblank();
    }
}

/// Handle a pending player choice (SelectCard, SelectTarget, etc).
/// Returns true if the choice was handled.
pub fn handle_choice(ui: &mut dyn PlatformUi, gs: &mut GameState) -> bool {
    use crate::ability::types::Choice;
    use crate::ability::util::zone_cards;
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
                                .map(|c| c.name.to_string())
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
                            .map(|c| c.name.to_string())
                            .unwrap_or_else(|| format!("#{}", cid))
                    })
                    .collect(),
            };

            if count <= 1 {
                let sel = menu_select(ui, &items, &description, allow_skip);
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
                    let sel = menu_select(ui, &display_items, &description, allow_skip);
                    match sel {
                        None => break,
                        Some(idx) => {
                            if !selected.contains(&idx) {
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
