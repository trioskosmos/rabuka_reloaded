use crate::card::AbilityEffect;
use super::types::{Choice, ExecutionContext, LookAndSelectStep};
use super::resolver::AbilityResolver;
use super::util;
use crate::zones::MemberArea;

fn pos_to_area(pos: usize) -> MemberArea {
    match pos { 0 => MemberArea::LeftSide, 1 => MemberArea::Center, _ => MemberArea::RightSide }
}

fn stage_first_empty(stage: &[i16; 3]) -> Option<usize> {
    if stage[1] == -1 { Some(1) }
    else if stage[0] == -1 { Some(0) }
    else if stage[2] == -1 { Some(2) }
    else { None }
}

#[allow(dead_code)]
impl<'a> AbilityResolver<'a> {
    fn prompt_card_choice(&mut self, zone: &str, count: usize, desc: &str, card_type_filter: Option<&str>) -> Result<(), String> {
        self.pending_choice = Some(Choice::SelectCard {
            zone: zone.to_string(), card_type: card_type_filter.map(|s| s.to_string()),
            count, description: desc.to_string(), allow_skip: false,
        });
        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
        Ok(())
    }

    pub fn execute_move_cards(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let count = effect.count.unwrap_or(1) as usize;
        let source = effect.source.as_deref().unwrap_or("").to_string();
        let destination = effect.destination.as_deref().unwrap_or("").to_string();
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type_filter = effect.card_type.as_deref();
        let group_name = effect.group.as_ref().map(|g| g.name.as_str());
        let cost_limit = effect.cost_limit;
        let is_self_cost = effect.self_cost.unwrap_or(false);
        let exclude_self = effect.exclude_self.unwrap_or(false);
        let card_db = self.game_state.card_database.clone();
        let activating_card_id = self.game_state.activating_card;

        // Collect moved cards for modifier clearing after player borrow ends
        let mut moved_cards: Vec<i16> = Vec::new();

        // Read vacated stage area (set by the cost phase) for same_area destination
        let vacated_stage_area = self.game_state.last_vacated_stage_area;
        self.game_state.last_vacated_stage_area = None;

        {
            let player = match target {
                "self" => &mut self.game_state.player1,
                "opponent" => &mut self.game_state.player2,
                _ => &mut self.game_state.player1,
            };

            match (source.as_str(), destination.as_str()) {
                // --- DECK → anything ---
                ("deck" | "deck_top", dest) => {
                    let mut moved = 0u32;
                    let mut drawn = 0u32;
                    let max_draws = player.main_deck.cards.len() + count;
                    while moved < count as u32 && drawn < max_draws as u32 {
                        if let Some(card) = player.main_deck.draw() {
                            drawn += 1;
                            if !util::card_matches_type(&card_db, card, card_type_filter) {
                                player.main_deck.cards.push(card); continue;
                            }
                            if !util::card_matches_group_str(&card_db, card, group_name) {
                                player.main_deck.cards.push(card); continue;
                            }
                            match dest {
                                "hand" => player.hand.add_card(card),
                                "discard" => player.waitroom.add_card(card),
                                "stage" => match stage_first_empty(&player.stage.stage) {
                                    Some(pos) => { player.stage.stage[pos] = card; player.areas_locked_this_turn.insert(pos_to_area(pos)); }
                                    None => player.hand.add_card(card),
                                },
                                "live_card_zone" => { player.live_card_zone.cards.push(card); }
                                "success_live_zone" => { player.success_live_card_zone.cards.push(card); }
                                "energy_zone" => { player.energy_zone.cards.push(card); }
                                "energy_deck" => { player.energy_deck.cards.push(card); }
                                "deck_top" => { player.main_deck.cards.insert(0, card); }
                                "deck_bottom" => { player.main_deck.cards.push(card); }
                                _ => {}
                            }
                            moved += 1;
                        } else { break; }
                    }
                }

                // --- STAGE → anything ---
                ("stage", dest) => {
                    if is_self_cost {
                        if let Some(activating_id) = activating_card_id {
                            let mut found = false;
                            for i in 0..3 {
                                if player.stage.stage[i] == activating_id {
                                    self.game_state.last_vacated_stage_area = Some(i);
                                    let card_id = player.stage.stage[i];
                                    player.stage.stage[i] = -1;
                                    match dest {
                                        "discard" | "" => player.waitroom.add_card(card_id),
                                        "hand" => player.hand.add_card(card_id),
                                        "deck_bottom" => player.main_deck.cards.push(card_id),
                                        "deck_top" => player.main_deck.cards.insert(0, card_id),
                                        "same_area" => {
                                            player.stage.stage[i] = card_id;
                                            self.game_state.last_vacated_stage_area = None;
                                        }
                                        "live_card_zone" => player.live_card_zone.cards.push(card_id),
                                        "success_live_zone" => player.success_live_card_zone.cards.push(card_id),
                                        _ => player.hand.add_card(card_id),
                                    }
                                    found = true; break;
                                }
                            }
                            if !found { return Err(format!("Activating card {} not found", activating_id)); }
                        } else { return Err("Self-cost required but no activating card".into()); }
                    } else {
                        let valid: Vec<(usize, i16)> = (0..3).filter_map(|i| {
                            let c = player.stage.stage[i];
                            if c == -1 || (exclude_self && activating_card_id == Some(c)) { None } else { Some((i, c)) }
                        }).collect();
                        if valid.len() < count { return Err(format!("Not enough cards on stage: need {}, have {}", count, valid.len())); }
                        if valid.len() > count {
                            self.pending_choice = Some(Choice::SelectCard {
                                zone: "stage".to_string(), card_type: card_type_filter.map(|s| s.to_string()),
                                count, description: format!("Select {} card(s) from stage", count), allow_skip: false,
                            });
                            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                            return Ok(());
                        }
                        for (i, card_id) in valid.iter().take(count) {
                            player.stage.stage[*i] = -1;
                            let cid = *card_id;
                            match dest {
                                "discard" => player.waitroom.add_card(cid),
                                "hand" => player.hand.add_card(cid),
                                "deck_bottom" => player.main_deck.cards.push(cid),
                                "deck_top" => player.main_deck.cards.insert(0, cid),
                                "live_card_zone" => player.live_card_zone.cards.push(cid),
                                "success_live_zone" => player.success_live_card_zone.cards.push(cid),
                                _ => player.hand.add_card(cid),
                            }
                            moved_cards.push(cid);
                        }
                    }
                }

                // --- HAND → discard/deck/stage/live ---
                ("hand", "discard") => {
                    let idxs = util::matching_indices(&player.hand.cards, &card_db, card_type_filter, group_name, cost_limit);
                    if idxs.len() < count { return Ok(()); }
                    if idxs.len() > count { return self.prompt_card_choice("hand", count, &format!("Select {} card(s) from hand to discard", count), card_type_filter); }
                    for &i in idxs.iter().rev().take(count) { let card = player.hand.cards.remove(i); player.waitroom.add_card(card); moved_cards.push(card); }
                }
                ("hand", "deck_bottom") | ("hand", "deck_top") => {
                    let idxs = util::matching_indices(&player.hand.cards, &card_db, card_type_filter, group_name, cost_limit);
                    if idxs.len() < count { return Err("Not enough cards in hand".into()); }
                    if idxs.len() > count { return self.prompt_card_choice("hand", count, &format!("Select {} cards to move to {}", count, destination), card_type_filter); }
                    for &i in idxs.iter().rev().take(count) { let card = player.hand.cards.remove(i); player.main_deck.cards.push(card); moved_cards.push(card); }
                }
                ("hand", "stage") => {
                    let idxs = util::matching_indices(&player.hand.cards, &card_db, card_type_filter, group_name, cost_limit);
                    if idxs.len() < count { return Err("Not enough cards in hand".into()); }
                    if idxs.len() > count { return self.prompt_card_choice("hand", count, &format!("Select {} cards from hand to stage", count), card_type_filter); }
                    for &i in idxs.iter().rev().take(count) {
                        let card = player.hand.cards.remove(i);
                        match stage_first_empty(&player.stage.stage) {
                            Some(pos) => { player.stage.stage[pos] = card; player.areas_locked_this_turn.insert(pos_to_area(pos)); }
                            None => player.hand.add_card(card),
                        }
                        moved_cards.push(card);
                    }
                }
                ("hand", "live_card_zone") => {
                    let idxs = util::matching_indices(&player.hand.cards, &card_db, Some("live_card"), group_name, cost_limit);
                    if idxs.len() < count { return Err("Not enough live cards in hand".into()); }
                    for &i in idxs.iter().rev() { let card = player.hand.cards.remove(i); player.live_card_zone.cards.push(card); moved_cards.push(card); }
                }

                // --- DISCARD → hand/deck/stage ---
                ("discard", "hand") => {
                    let idxs = util::matching_indices(&player.waitroom.cards, &card_db, card_type_filter, group_name, cost_limit);
                    if idxs.len() < count { return Err(format!("Not enough cards in discard: need {}, have {}", count, idxs.len())); }
                    if idxs.len() > count { return self.prompt_card_choice("discard", count, &format!("Select {} cards from discard to hand", count), card_type_filter); }
                    for &i in idxs.iter().rev().take(count) { let card = player.waitroom.cards.remove(i); player.hand.add_card(card); moved_cards.push(card); }
                }
                ("discard", "deck_bottom") | ("discard", "deck_top") => {
                    let idxs = util::matching_indices(&player.waitroom.cards, &card_db, card_type_filter, group_name, cost_limit);
                    if idxs.len() < count { return Err("Not enough cards in discard".into()); }
                    if idxs.len() > count { return self.prompt_card_choice("discard", count, &format!("Select {} cards from discard to {}", count, destination), card_type_filter); }
                    for &i in idxs.iter().rev().take(count) { let card = player.waitroom.cards.remove(i); player.main_deck.cards.push(card); moved_cards.push(card); }
                }
                ("discard", "deck") => {
                    if effect.placement_order.as_deref() == Some("any_order") && count > 1 {
                        let idxs = util::matching_indices(&player.waitroom.cards, &card_db, card_type_filter, group_name, cost_limit);
                        let cards: Vec<i16> = idxs.iter().rev().map(|&i| player.waitroom.cards.remove(i)).collect();
                        for &c in &cards { moved_cards.push(c); }
                        self.looked_at_cards = cards;
                        self.pending_choice = Some(Choice::SelectTarget {
                            target: "order".to_string(), description: "Choose order for cards on deck".to_string(),
                        });
                        self.execution_context = ExecutionContext::LookAndSelect { step: LookAndSelectStep::Finalize { destination: "deck".to_string() } };
                        return Ok(());
                    }
                    let idxs = util::matching_indices(&player.waitroom.cards, &card_db, card_type_filter, group_name, cost_limit);
                    let pos = effect.position.as_ref().and_then(|p| p.get_position()).and_then(|s| s.parse::<usize>().ok());
                    for &i in idxs.iter().rev() {
                        let card = player.waitroom.cards.remove(i);
                        let insert = pos.map(|p| p.saturating_sub(1)).unwrap_or(0).min(player.main_deck.cards.len());
                        player.main_deck.cards.insert(insert, card);
                        moved_cards.push(card);
                    }
                }
                ("discard", "live_card_zone") => {
                    let idxs = util::matching_indices(&player.waitroom.cards, &card_db, Some("live_card"), group_name, cost_limit);
                    if idxs.is_empty() {
                        return Err("No live cards in discard to move to live card zone".into());
                    }
                    let select_count = if idxs.len() < count { idxs.len() } else { count };
                    if idxs.len() > count {
                        return self.prompt_card_choice("discard", count, &format!("Select {} live cards for live card zone", count), Some("live_card"));
                    }
                    for &i in idxs.iter().rev().take(select_count) {
                        let card = player.waitroom.cards.remove(i);
                        player.live_card_zone.cards.push(card);
                        moved_cards.push(card);
                    }
                }
                ("discard", "same_area") => {
                    let target_pos = vacated_stage_area.unwrap_or(1);
                    let idxs = util::matching_indices(&player.waitroom.cards, &card_db, Some("member_card"), group_name, cost_limit);
                    if idxs.is_empty() {
                        return Err("No matching member cards in discard to place in same area".into());
                    }
                    let pick_idx = if idxs.len() > 1 { idxs[0] } else { idxs[0] };
                    let card = player.waitroom.cards.remove(pick_idx);
                    if target_pos < 3 && player.stage.stage[target_pos] == -1 {
                        player.stage.stage[target_pos] = card;
                        player.areas_locked_this_turn.insert(pos_to_area(target_pos));
                    } else if let Some(pos) = stage_first_empty(&player.stage.stage) {
                        player.stage.stage[pos] = card;
                        player.areas_locked_this_turn.insert(pos_to_area(pos));
                    } else {
                        player.hand.add_card(card);
                    }
                    moved_cards.push(card);
                }
                ("discard", "stage") | ("discard", "empty_area") => {
                    let is_max = effect.max.unwrap_or(false);

                    let candidate_indices = util::matching_indices(&player.waitroom.cards, &card_db, Some("member_card"), group_name, cost_limit);

                    let available_count = candidate_indices.len();
                    let empty_stage_slots = player.stage.stage.iter().filter(|&&s| s == -1).count();

                    if available_count == 0 {
                        // No valid targets
                    } else if available_count <= count && !is_max {
                        // Auto-place all matching cards
                        for &i in candidate_indices.iter().rev() {
                            let card = player.waitroom.cards.remove(i);
                            if let Some(pos) = stage_first_empty(&player.stage.stage) {
                                player.stage.stage[pos] = card;
                                player.areas_locked_this_turn.insert(pos_to_area(pos));
                            } else {
                                player.hand.add_card(card);
                            }
                            moved_cards.push(card);
                        }
                    } else {
                        // Need player to choose which cards
                        let select_count = if empty_stage_slots < count { empty_stage_slots } else { count };
                        let description = if is_max {
                            format!("Select up to {} member cards from waiting room to put on stage", select_count)
                        } else {
                            format!("Select {} member cards from waiting room to put on stage", select_count)
                        };
                        self.pending_choice = Some(Choice::SelectCard {
                            zone: "discard".to_string(),
                            card_type: effect.card_type.clone(),
                            count: select_count,
                            description,
                            allow_skip: is_max,
                        });
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        return Ok(());
                    }
                }

                // --- ENERGY_ZONE → hand/discard ---
                ("energy_zone", "hand") | ("energy_zone", "discard") => {
                    let idxs = util::matching_indices(&player.energy_zone.cards, &card_db, card_type_filter, None, None);
                    if idxs.len() < count { return Err("Not enough cards in energy zone".into()); }
                    if idxs.len() > count { return self.prompt_card_choice("energy_zone", count, &format!("Select {} cards from energy", count), card_type_filter); }
                    for &i in idxs.iter().rev().take(count) {
                        let card = player.energy_zone.cards.remove(i);
                        if destination == "hand" { player.hand.add_card(card); } else { player.waitroom.add_card(card); }
                    }
                }

                // --- LIVE/Success zone → hand/deck ---
                ("live_card_zone", "hand") | ("live_card_zone", "success_live_zone") | ("live_card_zone", "discard") => {
                    let idxs = util::matching_indices(&player.live_card_zone.cards, &card_db, None, None, None);
                    if idxs.len() < count { return Err("Not enough cards in live zone".into()); }
                    if idxs.len() > count { return self.prompt_card_choice("live_card_zone", count, &format!("Select {} cards", count), None); }
                    for &i in idxs.iter().rev().take(count) {
                        let card = player.live_card_zone.cards.remove(i);
                        match destination.as_str() { "hand" => player.hand.add_card(card), "success_live_zone" => player.success_live_card_zone.cards.push(card), _ => player.waitroom.add_card(card), }
                    }
                }
                ("success_live_zone", "hand") | ("success_live_zone", "deck_top") | ("success_live_zone", "deck_bottom") => {
                    let idxs = util::matching_indices(&player.success_live_card_zone.cards, &card_db, None, None, None);
                    if idxs.len() < count { return Err("Not enough cards in success zone".into()); }
                    if idxs.len() > count { return self.prompt_card_choice("success_live_zone", count, &format!("Select {} cards", count), None); }
                    for &i in idxs.iter().rev().take(count) {
                        let card = player.success_live_card_zone.cards.remove(i);
                        match destination.as_str() { "hand" => player.hand.add_card(card), "deck_top" => player.main_deck.cards.insert(0, card), _ => player.main_deck.cards.push(card), }
                        moved_cards.push(card);
                    }
                }

                _ => { eprintln!("Unsupported move: {} -> {}", source, destination); }
            }
        } // player borrow scope ends here

        // Apply state_change to moved cards (e.g. "ウェイト状態で置く")
        if let Some(ref sc) = effect.state_change {
            if sc == "wait" {
                for card_id in &moved_cards {
                    self.game_state.add_orientation_modifier(*card_id, "wait");
                }
                // For energy zone placements, deactivate energy
                if destination == "energy_zone" {
                    for _ in &moved_cards {
                        let p = match target {
                            "self" => &mut self.game_state.player1,
                            "opponent" => &mut self.game_state.player2,
                            _ => &mut self.game_state.player1,
                        };
                        p.energy_zone.active_energy_count = p.energy_zone.active_energy_count.saturating_sub(1);
                    }
                }
            } else if sc == "active" {
                for card_id in &moved_cards {
                    self.game_state.add_orientation_modifier(*card_id, "active");
                }
                if destination == "energy_zone" {
                    let p = match target {
                        "self" => &mut self.game_state.player1,
                        "opponent" => &mut self.game_state.player2,
                        _ => &mut self.game_state.player1,
                    };
                    p.energy_zone.active_energy_count += moved_cards.len();
                }
            }
        }

        // Clear modifiers after player borrow ends
        for card_id in &moved_cards { self.game_state.clear_modifiers_for_card(*card_id); }
        for card_id in &moved_cards { self.game_state.record_card_movement(*card_id); }

        Ok(())
    }
}
