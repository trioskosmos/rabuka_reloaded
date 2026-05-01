use crate::card::AbilityEffect;
use crate::card::CardDatabase;
use super::types::{Choice, ExecutionContext, LookAndSelectStep};
use super::resolver::AbilityResolver;
use crate::zones::MemberArea;

fn pos_to_area(pos: usize) -> MemberArea {
    match pos { 0 => MemberArea::LeftSide, 1 => MemberArea::Center, _ => MemberArea::RightSide }
}

fn matching_indices(
    cards: &[i16], count: usize,
    card_db: &CardDatabase,
    card_type_filter: Option<&str>,
    group_name: Option<&str>,
    cost_limit: Option<u32>,
) -> Vec<usize> {
    let mut matches = Vec::new();
    for (i, &card_id) in cards.iter().enumerate() {
        if matches.len() >= count { break; }
        let type_ok = match card_type_filter {
            Some("live_card") => card_db.get_card(card_id).map(|c| c.is_live()).unwrap_or(false),
            Some("member_card") => card_db.get_card(card_id).map(|c| c.is_member()).unwrap_or(false),
            Some("energy_card") => card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false),
            None => true, _ => true,
        };
        if !type_ok { continue; }
        if let Some(grp) = group_name {
            if !card_db.get_card(card_id).map(|c| c.group.as_str() == grp).unwrap_or(false) { continue; }
        }
        if let Some(limit) = cost_limit {
            if !card_db.get_card(card_id).and_then(|c| c.cost).map(|c| c <= limit).unwrap_or(false) { continue; }
        }
        matches.push(i);
    }
    matches
}

fn stage_first_empty(stage: &[i16; 3]) -> Option<usize> {
    if stage[1] == -1 { Some(1) }
    else if stage[0] == -1 { Some(0) }
    else if stage[2] == -1 { Some(2) }
    else { None }
}

#[allow(dead_code)]
impl<'a> AbilityResolver<'a> {
    fn prompt_card_choice(&mut self, zone: &str, count: usize, desc: &str) -> Result<(), String> {
        self.pending_choice = Some(Choice::SelectCard {
            zone: zone.to_string(), card_type: None,
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
                            let type_ok = match card_type_filter {
                                Some("live_card") => card_db.get_card(card).map(|c| c.is_live()).unwrap_or(false),
                                Some("member_card") => card_db.get_card(card).map(|c| c.is_member()).unwrap_or(false),
                                Some("energy_card") => card_db.get_card(card).map(|c| c.is_energy()).unwrap_or(false),
                                None => true, _ => true,
                            };
                            if !type_ok { player.main_deck.cards.push(card); continue; }
                            if let Some(grp) = group_name {
                                if !card_db.get_card(card).map(|c| c.group.as_str() == grp).unwrap_or(false) {
                                    player.main_deck.cards.push(card); continue;
                                }
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
                                "deck_top" => { player.main_deck.cards.insert(0, card); }
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
                                    let card_id = player.stage.stage[i];
                                    player.stage.stage[i] = -1;
                                    match dest {
                                        "discard" | "" => player.waitroom.add_card(card_id),
                                        "hand" => player.hand.add_card(card_id),
                                        "deck_bottom" => player.main_deck.cards.push(card_id),
                                        "deck_top" => player.main_deck.cards.insert(0, card_id),
                                        "same_area" => {
                                            if let Some(act_id) = self.activating_card_id {
                                                for pos in 0..3 { if player.stage.stage[pos] == act_id { player.stage.stage[pos] = card_id; break; } }
                                            } else { player.hand.add_card(card_id); }
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
                    return self.prompt_card_choice("hand", count, &format!("Select {} card(s) from hand to discard", count));
                }
                ("hand", "deck_bottom") | ("hand", "deck_top") => {
                    let idxs = matching_indices(&player.hand.cards, count, &card_db, card_type_filter, group_name, cost_limit);
                    if idxs.len() < count { return Err("Not enough cards in hand".into()); }
                    if idxs.len() > count { return self.prompt_card_choice("hand", count, &format!("Select {} cards to move to {}", count, destination)); }
                    for &i in idxs.iter().rev() { let card = player.hand.cards.remove(i); player.main_deck.cards.push(card); moved_cards.push(card); }
                }
                ("hand", "stage") => {
                    let idxs = matching_indices(&player.hand.cards, count, &card_db, card_type_filter, group_name, cost_limit);
                    if idxs.len() < count { return Err("Not enough cards in hand".into()); }
                    for &i in idxs.iter().rev() {
                        let card = player.hand.cards.remove(i);
                        match stage_first_empty(&player.stage.stage) {
                            Some(pos) => { player.stage.stage[pos] = card; player.areas_locked_this_turn.insert(pos_to_area(pos)); }
                            None => player.hand.add_card(card),
                        }
                        moved_cards.push(card);
                    }
                }
                ("hand", "live_card_zone") => {
                    let idxs = matching_indices(&player.hand.cards, count, &card_db, Some("live_card"), group_name, cost_limit);
                    if idxs.len() < count { return Err("Not enough live cards in hand".into()); }
                    for &i in idxs.iter().rev() { let card = player.hand.cards.remove(i); player.live_card_zone.cards.push(card); moved_cards.push(card); }
                }

                // --- DISCARD → hand/deck/stage ---
                ("discard", "hand") => {
                    let idxs = matching_indices(&player.waitroom.cards, count, &card_db, card_type_filter, group_name, cost_limit);
                    if idxs.len() < count { return Err(format!("Not enough cards in discard: need {}, have {}", count, idxs.len())); }
                    if idxs.len() > count { return self.prompt_card_choice("discard", count, &format!("Select {} cards from discard to hand", count)); }
                    for &i in idxs.iter().rev() { let card = player.waitroom.cards.remove(i); player.hand.add_card(card); moved_cards.push(card); }
                }
                ("discard", "deck_bottom") | ("discard", "deck_top") => {
                    let idxs = matching_indices(&player.waitroom.cards, count, &card_db, card_type_filter, group_name, cost_limit);
                    if idxs.len() < count { return Err("Not enough cards in discard".into()); }
                    if idxs.len() > count { return self.prompt_card_choice("discard", count, &format!("Select {} cards from discard to {}", count, destination)); }
                    for &i in idxs.iter().rev() { let card = player.waitroom.cards.remove(i); player.main_deck.cards.push(card); moved_cards.push(card); }
                }
                ("discard", "deck") => {
                    if effect.placement_order.as_deref() == Some("any_order") && count > 1 {
                        let idxs = matching_indices(&player.waitroom.cards, count, &card_db, card_type_filter, group_name, cost_limit);
                        let cards: Vec<i16> = idxs.iter().rev().map(|&i| player.waitroom.cards.remove(i)).collect();
                        for &c in &cards { moved_cards.push(c); }
                        self.looked_at_cards = cards;
                        self.pending_choice = Some(Choice::SelectTarget {
                            target: "order".to_string(), description: "Choose order for cards on deck".to_string(),
                        });
                        self.execution_context = ExecutionContext::LookAndSelect { step: LookAndSelectStep::Finalize { destination: "deck".to_string() } };
                        return Ok(());
                    }
                    let idxs = matching_indices(&player.waitroom.cards, count, &card_db, card_type_filter, group_name, cost_limit);
                    let pos = effect.position.as_ref().and_then(|p| p.get_position()).and_then(|s| s.parse::<usize>().ok());
                    for &i in idxs.iter().rev() {
                        let card = player.waitroom.cards.remove(i);
                        let insert = pos.map(|p| p.saturating_sub(1)).unwrap_or(0).min(player.main_deck.cards.len());
                        player.main_deck.cards.insert(insert, card);
                        moved_cards.push(card);
                    }
                }
                ("discard", "stage") => {
                    let _total_cost_limit = effect.total_cost_limit;
                    let is_max = effect.max.unwrap_or(false);

                    let candidate_indices: Vec<usize> = player.waitroom.cards.iter().enumerate()
                        .filter(|(_, &card_id)| {
                            let type_ok = card_db.get_card(card_id).map(|c| c.is_member()).unwrap_or(false);
                            if !type_ok { return false; }
                            if let Some(grp) = group_name {
                                if !card_db.get_card(card_id).map(|c| c.group.as_str() == grp).unwrap_or(false) { return false; }
                            }
                            true
                        })
                        .map(|(i, _)| i)
                        .collect();

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
                    let idxs = matching_indices(&player.energy_zone.cards, count, &card_db, card_type_filter, None, None);
                    if idxs.len() < count { return Err("Not enough cards in energy zone".into()); }
                    if idxs.len() > count { return self.prompt_card_choice("energy_zone", count, &format!("Select {} cards from energy", count)); }
                    for &i in idxs.iter().rev() {
                        let card = player.energy_zone.cards.remove(i);
                        if destination == "hand" { player.hand.add_card(card); } else { player.waitroom.add_card(card); }
                    }
                }

                // --- LIVE/Success zone → hand/deck ---
                ("live_card_zone", "hand") | ("live_card_zone", "success_live_zone") | ("live_card_zone", "discard") => {
                    let idxs = matching_indices(&player.live_card_zone.cards, count, &card_db, None, None, None);
                    if idxs.len() < count { return Err("Not enough cards in live zone".into()); }
                    if idxs.len() > count { return self.prompt_card_choice("live_card_zone", count, &format!("Select {} cards", count)); }
                    for &i in idxs.iter().rev() {
                        let card = player.live_card_zone.cards.remove(i);
                        match destination.as_str() { "hand" => player.hand.add_card(card), "success_live_zone" => player.success_live_card_zone.cards.push(card), _ => player.waitroom.add_card(card), }
                    }
                }
                ("success_live_zone", "hand") | ("success_live_zone", "deck_top") | ("success_live_zone", "deck_bottom") => {
                    let idxs = matching_indices(&player.success_live_card_zone.cards, count, &card_db, None, None, None);
                    if idxs.len() < count { return Err("Not enough cards in success zone".into()); }
                    if idxs.len() > count { return self.prompt_card_choice("success_live_zone", count, &format!("Select {} cards", count)); }
                    for &i in idxs.iter().rev() {
                        let card = player.success_live_card_zone.cards.remove(i);
                        match destination.as_str() { "hand" => player.hand.add_card(card), "deck_top" => player.main_deck.cards.insert(0, card), _ => player.main_deck.cards.push(card), }
                        moved_cards.push(card);
                    }
                }

                _ => { eprintln!("Unsupported move: {} -> {}", source, destination); }
            }
        } // player borrow scope ends here

        // Clear modifiers after player borrow ends
        for card_id in &moved_cards { self.game_state.clear_modifiers_for_card(*card_id); }
        for card_id in &moved_cards { self.game_state.record_card_movement(*card_id); }

        Ok(())
    }
}
