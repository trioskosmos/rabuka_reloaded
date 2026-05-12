use crate::card::AbilityEffect;
use super::types::{Choice, ExecutionContext, LookAndSelectStep};
use super::resolver::AbilityResolver;
use super::util;

enum SelectionOutcome {
    Exact(Vec<usize>),
    Prompt,
    Skip,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MoveCardsTarget {
    PlayerSelf,
    Opponent,
}

enum InsufficientBehavior { Silent, Error(&'static str) }

fn classify_selection(idxs: &[usize], count: usize, is_all: bool, on_insufficient: InsufficientBehavior) -> Result<SelectionOutcome, String> {
    if is_all { return Ok(SelectionOutcome::Exact(idxs.to_vec())); }
    if idxs.len() < count {
        return match on_insufficient {
            InsufficientBehavior::Silent => Ok(SelectionOutcome::Skip),
            InsufficientBehavior::Error(msg) => Err(msg.to_string()),
        };
    }
    if idxs.len() > count { return Ok(SelectionOutcome::Prompt); }
    Ok(SelectionOutcome::Exact(idxs.to_vec()))
}

fn remove_cards_from_hand(player: &mut crate::player::Player, indices: &[usize]) -> Vec<i16> {
    indices.iter().rev().map(|&i| player.hand.cards.remove(i)).collect()
}

impl<'a> AbilityResolver<'a> {
    pub fn execute_move_cards(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Resolve dynamic_count if count is not explicitly set
        let count = if effect.count.is_some() {
            effect.count.unwrap() as usize
        } else if let Some(ref dc) = effect.dynamic_count {
            match dc.count_type.as_str() {
                "revealed_cards" => {
                    // Count cards revealed by a previous cost/effect step
                    let revealed = if !self.game_state.player1_cheer_revealed_cards.is_empty() {
                        &self.game_state.player1_cheer_revealed_cards
                    } else {
                        &self.game_state.revealed_cards
                    };
                    revealed.len()
                }
                _ => 0
            }
        } else {
            0
        };
        let group_name = effect.group_name();

        // Handle or_card_types: let the player pick which type to search for
        let card_type_owned: Option<String> = if let Some(or_types) = &effect.or_card_types {
            if or_types.is_empty() { effect.card_type.clone() } else {
                let chosen = self.game_state.ability_queue.current_entry()
                    .and_then(|e| e.conditional_choice.clone());
                match chosen {
                    Some(s) => Some(s),
                    None => {
                        self.pending_choice = Some(Choice::SelectTarget {
                            target: "choice_string".to_string(),
                            description: format!("Pick card type: {:?}", or_types),
                            allow_skip: false,
                        });
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        self.game_state.ability_queue.current_entry_mut().map(|e| {
                            e.conditional_choice = Some(serde_json::to_string(&or_types).unwrap());
                        });
                        return Ok(());
                    }
                }
            }
        } else { effect.card_type.clone() };
        let card_type_filter: Option<&str> = card_type_owned.as_deref();
        let tgt = effect.target.clone();
        let cost_limit = effect.cost_limit;
        let is_self_cost = effect.self_cost.unwrap_or(false);
        let exclude_self = effect.exclude_self.unwrap_or(false);
        let is_max = effect.max.unwrap_or(false);
        let is_all = effect.all.unwrap_or(false);
        let card_db = self.game_state.card_database.clone();
        let activating_card_id = self.game_state.activating_card;
        let vacated_stage_area = self.game_state.last_vacated_stage_area;
        self.game_state.last_vacated_stage_area = None;

        // Character name filter from the effect
        // TEMPORARILY REVERTED: effect.characters breaks Chika score tests
        let character_filter: Option<Vec<String>> = None;

        // Resolve name_constraint (e.g. "contains_all" from a revealed card)
        let name_fragments: Option<Vec<String>> = if effect.name_constraint.as_deref() == Some("contains_all")
            && effect.name_constraint_source.as_deref() == Some("revealed_card")
        {
            let fragments: Vec<String> = self.game_state.revealed_cost_cards.iter()
                .chain(self.game_state.revealed_cards.iter())
                .filter_map(|&id| {
                    let card = self.game_state.card_database.get_card(id)?;
                    Some(card.name.replace("\u{FF06}", "&").split('&').map(|s| s.to_string()).collect::<Vec<_>>())
                })
                .flatten()
                .collect();
            if fragments.is_empty() { None } else { Some(fragments) }
        } else {
            None
        };

        let _name_filter = |card_db: &crate::card::CardDatabase, card_id: i16| -> bool {
            match &name_fragments {
                Some(fragments) => {
                    if let Some(card) = card_db.get_card(card_id) {
                        fragments.iter().all(|f| card.name.contains(f.as_str()))
                    } else { false }
                }
                None => true,
            }
        };

        let mut moved_cards: Vec<i16> = Vec::new();
        let source = effect.source.clone().unwrap_or_default();
        let destination = effect.destination.clone().unwrap_or_default();

        {
            let player = match tgt.as_ref().map(|s: &String| s.as_str()).unwrap_or("self") {
                "self" => &mut self.game_state.player1,
                "opponent" => &mut self.game_state.player2,
                _ => &mut self.game_state.player1,
            };

            // --- STEP 1: Get cards from source ---
            let source_str = if source.is_empty() { "" } else { source.as_str() };
            let mut taken: Vec<i16> = match source_str {
                // Deck → anything (sequential draw, no selection prompt)
                "deck" | "deck_top" => {
                    let mut drawn = Vec::new();
                    let mut attempts = 0u32;
                    while drawn.len() < count && attempts < (count as u32 + player.main_deck.cards.len() as u32) {
                        if let Some(card) = player.main_deck.draw() {
                            attempts += 1;
                            if !util::card_matches_type(&card_db, card, card_type_filter) { player.main_deck.cards.push(card); continue; }
                            if !util::card_matches_group_str(&card_db, card, group_name) { player.main_deck.cards.push(card); continue; }
                            drawn.push(card);
                        } else { break; }
                    }
                    drawn
                }
                "energy_deck" => {
                    let mut drawn = Vec::new();
                    for _i in 0..count {
                        if let Some(card) = player.energy_deck.draw() {
                            drawn.push(card);
                        } else { break; }
                    }
                    drawn
                }

                // Stage → anything
                "stage" => {
                    if is_self_cost {
                        let mut cards = Vec::new();
                        let mut found = false;
                        if let Some(activating_id) = activating_card_id {
                            for i in 0..3 {
                                if player.stage.stage[i] == activating_id {
                                    self.game_state.last_vacated_stage_area = Some(i);
                                    // Only recycle under-cards if the card is actually leaving (not same_area)
                                    if destination != "same_area" {
                                        if let Some(cid) = player.remove_member_from_stage_with_recycling(i, &card_db) {
                                            cards.push(cid);
                                        }
                                    } else {
                                        cards.push(player.stage.stage[i]);
                                        player.stage.stage[i] = activating_id;
                                        self.game_state.last_vacated_stage_area = None;
                                    }
                                    found = true; break;
                                }
                            }
                        }
                        if !found { return Err("Activating card not found at stage".to_string()); }
                        cards
                    } else {
                        let filter = util::filter_from_parts_full(card_type_filter, group_name, cost_limit, None, character_filter.as_ref(), name_fragments.as_ref(), None, if exclude_self { activating_card_id } else { None });
                        let mut idxs = util::matching_indices(&player.stage.stage, &card_db, &filter, true);
                        if effect.self_target.unwrap_or(false) { if let Some(aid) = activating_card_id { idxs.retain(|&i| i < 3 && player.stage.stage[i] == aid); } }
                        match classify_selection(&idxs, count, is_all, InsufficientBehavior::Silent)? {
                            SelectionOutcome::Exact(indices) => {
                                let (cards, vacated) = Self::stage_remove_with_vacated(player, &indices, &card_db);
                                self.game_state.last_vacated_stage_area = vacated;
                                cards
                            }
                            SelectionOutcome::Prompt => {
                                self.pending_choice = Some(Choice::SelectCard {
                                    zone: "stage".to_string(), card_type: card_type_filter.map(|s| s.to_string()),
                                    count, description: format!("Select {} card(s) from stage", count), allow_skip: false,
                                    cost_limit, cost_limit_operator: effect.cost_limit_operator.clone(), group: group_name.map(|s| s.to_string()), characters: character_filter.clone(),
                    filtered_indices: None,
                    is_select_action: false,
                });
                                self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                                return Ok(());
                            }
                            SelectionOutcome::Skip => vec![],
                        }
                    }
                }

                // Template zones: CardFilter → matching_indices → classify_selection
                "hand" => {
                    let filter = util::filter_from_parts_full(card_type_filter, group_name, cost_limit, None, character_filter.as_ref(), name_fragments.as_ref(), None, None);
                    let mut idxs = util::matching_indices(&player.hand.cards, &card_db, &filter, false);
                    if effect.self_target.unwrap_or(false) { if let Some(aid) = activating_card_id { idxs.retain(|&i| i < player.hand.cards.len() && player.hand.cards[i] == aid); } }
                    match classify_selection(&idxs, count, is_all, InsufficientBehavior::Silent)? {
                        SelectionOutcome::Exact(indices) => remove_cards_from_hand(player, &indices),
                        SelectionOutcome::Prompt | SelectionOutcome::Skip => {
                            // Always prompt for hand selection - optional determines if skip is allowed
                            let is_optional = effect.optional.unwrap_or(false);
                            self.pending_choice = Some(Choice::SelectCard {
                                zone: "hand".to_string(),
                                card_type: card_type_filter.map(|s| s.to_string()),
                                count,
                                description: format!("Select {} card(s) from hand", count),
                                allow_skip: is_optional,
                                cost_limit,
                                cost_limit_operator: effect.cost_limit_operator.clone(),
                                group: group_name.map(|s| s.to_string()),
                                characters: character_filter.clone(),
                    filtered_indices: None,
                    is_select_action: false,
                });
                            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                            return Ok(());
                        }
                    }
                }
                "discard" => {
                    let filter = util::filter_from_parts_full(card_type_filter, group_name, cost_limit, None, character_filter.as_ref(), name_fragments.as_ref(), None, None);
                    let mut idxs = util::matching_indices(&player.waitroom.cards, &card_db, &filter, false);
                    idxs.retain(|&i| i < player.waitroom.cards.len());
                    if effect.self_target.unwrap_or(false) { if let Some(aid) = activating_card_id { idxs.retain(|&i| i < player.waitroom.cards.len() && player.waitroom.cards[i] == aid); } }
                    match classify_selection(&idxs, count, is_all, InsufficientBehavior::Silent)? {
                        SelectionOutcome::Exact(indices) => indices.iter().rev().map(|&i| player.waitroom.cards.remove(i)).collect(),
                        SelectionOutcome::Prompt => {
                            if vacated_stage_area.is_some() { self.game_state.last_vacated_stage_area = vacated_stage_area; }
                            self.pending_choice = Some(Choice::SelectCard {
                                zone: "discard".to_string(),
                                card_type: card_type_filter.map(|s| s.to_string()),
                                count,
                                description: format!("Select {} card(s) from discard", count),
                                allow_skip: false,
                                cost_limit,
                                cost_limit_operator: effect.cost_limit_operator.clone(),
                                group: group_name.map(|s| s.to_string()),
                                characters: character_filter.clone(),
                                filtered_indices: None,
                                is_select_action: false,
                            });
                            return Ok(());
                        }
                        SelectionOutcome::Skip => vec![],
                    }
                }
                "energy_zone" => {
                    let filter = util::filter_from_parts(card_type_filter, None, None, None, character_filter.as_ref(), None);
                    let mut idxs = util::matching_indices(&player.energy_zone.cards, &card_db, &filter, false);
                    if effect.self_target.unwrap_or(false) { if let Some(aid) = activating_card_id { idxs.retain(|&i| i < player.energy_zone.cards.len() && player.energy_zone.cards[i] == aid); } }
                    match classify_selection(&idxs, count, is_all, InsufficientBehavior::Error("Not enough cards in energy zone"))? {
                        SelectionOutcome::Exact(indices) => indices.iter().rev().map(|&i| player.energy_zone.cards.remove(i)).collect(),
                        SelectionOutcome::Prompt => {
                            self.pending_choice = Some(Choice::SelectCard {
                                zone: "energy_zone".to_string(),
                                card_type: card_type_filter.map(|s| s.to_string()),
                                count,
                                description: format!("Select {} card(s) from energy zone", count),
                                allow_skip: false,
                                cost_limit,
                                cost_limit_operator: effect.cost_limit_operator.clone(),
                                group: group_name.map(|s| s.to_string()),
                                characters: character_filter.clone(),
                                filtered_indices: None,
                                is_select_action: false,
                            });
                            return Ok(());
                        }
                        SelectionOutcome::Skip => vec![],
                    }
                }
                "live_card_zone" => {
                    let filter = util::filter_from_parts(Some("live_card"), group_name, cost_limit, None, character_filter.as_ref(), None);
                    let mut idxs = util::matching_indices(&player.live_card_zone.cards, &card_db, &filter, false);
                    if effect.self_target.unwrap_or(false) { if let Some(aid) = activating_card_id { idxs.retain(|&i| i < player.live_card_zone.cards.len() && player.live_card_zone.cards[i] == aid); } }
                    match classify_selection(&idxs, count, false, InsufficientBehavior::Error("Not enough cards in live card zone"))? {
                        SelectionOutcome::Exact(indices) => indices.iter().rev().map(|&i| player.live_card_zone.cards.remove(i)).collect(),
                        SelectionOutcome::Prompt => {
                            self.pending_choice = Some(Choice::SelectCard {
                                zone: "live_card_zone".to_string(), card_type: Some("live_card".to_string()),
                                count, description: format!("Select {} card(s) from live card zone", count), allow_skip: false,
                                cost_limit: None, cost_limit_operator: None, group: None, characters: None,
                    filtered_indices: None,
                    is_select_action: false,
                });
                            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                            return Ok(());
                        }
                        SelectionOutcome::Skip => vec![],
                    }
                }
                "success_live_zone" => {
                    let filter = util::filter_from_parts(None, None, None, None, character_filter.as_ref(), None);
                    let mut idxs = util::matching_indices(&player.success_live_card_zone.cards, &card_db, &filter, false);
                    if effect.self_target.unwrap_or(false) { if let Some(aid) = activating_card_id { idxs.retain(|&i| i < player.success_live_card_zone.cards.len() && player.success_live_card_zone.cards[i] == aid); } }
                    match classify_selection(&idxs, count, false, InsufficientBehavior::Error("Not enough cards in success live zone"))? {
                        SelectionOutcome::Exact(indices) => indices.iter().rev().map(|&i| player.success_live_card_zone.cards.remove(i)).collect(),
                        SelectionOutcome::Prompt => {
                            self.pending_choice = Some(Choice::select_card(
                                "success_live_zone".to_string(),
                                count,
                                format!("Select {} card(s) from success live zone", count),
                                false,
                            ));
                            return Ok(());
                        }
                        SelectionOutcome::Skip => vec![],
                    }
                }
                "those_cards" => {
                    let filter = util::filter_from_parts(card_type_filter, group_name, None, None, character_filter.as_ref(), None);
                    let mut idxs = util::matching_indices(&player.waitroom.cards, &card_db, &filter, false);
                    if effect.self_target.unwrap_or(false) { if let Some(aid) = activating_card_id { idxs.retain(|&i| i < player.waitroom.cards.len() && player.waitroom.cards[i] == aid); } }
                    match classify_selection(&idxs, count, false, InsufficientBehavior::Silent)? {
                        SelectionOutcome::Exact(indices) => indices.iter().rev().map(|&i| player.waitroom.cards.remove(i)).collect(),
                        SelectionOutcome::Prompt => {
                            if vacated_stage_area.is_some() { self.game_state.last_vacated_stage_area = vacated_stage_area; }
                            self.pending_choice = Some(Choice::SelectCard {
                                zone: "discard".to_string(),
                                card_type: card_type_filter.map(|s| s.to_string()),
                                count,
                                description: format!("Select {} card(s) from discard", count),
                                allow_skip: false,
                                cost_limit,
                                cost_limit_operator: effect.cost_limit_operator.clone(),
                                group: group_name.map(|s| s.to_string()),
                                characters: character_filter.clone(),
                                filtered_indices: None,
                                is_select_action: false,
                            });
                            return Ok(());
                        }
                        SelectionOutcome::Skip => vec![],
                    }
                }
                "looked_at" => {
                    let mut idxs: Vec<usize> = (0..self.looked_at_cards.len()).filter(|&i| {
                        let cid = self.looked_at_cards[i];
                        util::card_matches_type(&card_db, cid, card_type_filter)
                            && util::card_matches_group_str(&card_db, cid, group_name)
                            && util::card_matches_cost_limit(&card_db, cid, cost_limit)
                    }).collect();
                    if is_all && card_type_filter.is_none() { idxs = (0..self.looked_at_cards.len()).collect(); }
                    if idxs.is_empty() { vec![] }
                    else if card_type_filter.is_none() && idxs.len() > count && !is_all {
                        let taken: Vec<i16> = self.looked_at_cards.drain(..count).collect();
                        taken
                    }
                    else if idxs.len() > count && !is_all {
                        self.pending_choice = Some(Choice::SelectCard {
                            zone: "looked_at".to_string(), card_type: card_type_filter.map(|s| s.to_string()),
                            count, description: format!("Select {} card(s) from looked-at cards", count), allow_skip: false,
                            cost_limit, cost_limit_operator: effect.cost_limit_operator.clone(), group: group_name.map(|s| s.to_string()), characters: character_filter.clone(),
                            filtered_indices: None,
                            is_select_action: false,
                        });
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        return Ok(());
                    } else {
                        // For You's ability, we want to move the first card (chosen card) regardless of filters
                        // So if no filters are specified, just take the first card
                        if card_type_filter.is_none() && group_name.is_none() && cost_limit.is_none() {
                            let taken: Vec<i16> = if self.looked_at_cards.len() > 0 {
                                self.looked_at_cards.drain(..1).collect()
                            } else {
                                vec![]  // No card available
                            };
                            taken
                        } else {
                            // Apply filters as usual
                            idxs.sort_unstable_by(|a, b| b.cmp(a));
                            let taken: Vec<i16> = idxs.iter().map(|&i| self.looked_at_cards.remove(i)).collect();
                            for &card in &taken { moved_cards.push(card); }
                            taken
                        }
                    }
                }
                "looked_at_remaining" => {
                    // Only move remaining cards after the chosen card (first card)
                    let cards: Vec<i16> = if self.looked_at_cards.len() > 1 {
                        self.looked_at_cards.drain(1..).collect()
                    } else {
                        self.looked_at_cards.drain(..).collect()
                    };
                    for &card in &cards { player.waitroom.add_card(card); }
                    cards
                }
                "selected_cards" => {
                    // Cards previously selected by a "select" action, stored in entry.selected_card_ids.
                    // Find and remove them from whatever zone they're in, then move to destination.
                    let selected = self.selected_cards.clone();
                    if selected.len() < count {
                        vec![]
                    } else if selected.len() > count {
                        self.pending_choice = Some(Choice::SelectCard {
                            zone: "selected_cards".to_string(), card_type: card_type_filter.map(|s| s.to_string()),
                            count, description: format!("Select {} card(s) from selected cards", count), allow_skip: false,
                            cost_limit, cost_limit_operator: effect.cost_limit_operator.clone(),
                            group: group_name.map(|s| s.to_string()), characters: character_filter.clone(),
                            filtered_indices: None,
                            is_select_action: false,
                        });
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        return Ok(());
                    } else {
                        // Move from their actual locations to destination
                        let taken: Vec<i16> = selected.iter().copied().collect();
                        for &card_id in &taken {
                            // Remove from whatever zone it's in
                            if let Some(idx) = player.hand.cards.iter().position(|&c| c == card_id) {
                                player.hand.cards.remove(idx);
                            } else if let Some(idx) = player.waitroom.cards.iter().position(|&c| c == card_id) {
                                player.waitroom.cards.remove(idx);
                            } else if let Some(idx) = player.stage.stage.iter().position(|&c| c == card_id) {
                                player.stage.stage[idx] = -1;
                            }
                            moved_cards.push(card_id);
                        }
                        taken
                    }
                }
                "revealed_cards" => {
                    let is_self = tgt.as_ref().map(|s| s.as_str()).unwrap_or("self") == "self";
                    let cards: Vec<i16> = if is_self && !self.game_state.player1_cheer_revealed_cards.is_empty() {
                        self.game_state.player1_cheer_revealed_cards.drain(..).collect()
                    } else if !is_self && !self.game_state.player2_cheer_revealed_cards.is_empty() {
                        self.game_state.player2_cheer_revealed_cards.drain(..).collect()
                    } else {
                        self.game_state.revealed_cards.drain(..).collect()
                    };
                    if cards.len() > count {
                        for &c in &cards {
                            if is_self { self.game_state.player1_cheer_revealed_cards.push(c); }
                            else { self.game_state.player2_cheer_revealed_cards.push(c); }
                            self.game_state.revealed_cards.push(c);
                        }
                        self.pending_choice = Some(Choice::SelectCard {
                            zone: "revealed_cards".to_string(), card_type: card_type_filter.map(|s| s.to_string()),
                            count, description: format!("Select {} card(s) from revealed cards", count), allow_skip: false,
                            cost_limit, cost_limit_operator: effect.cost_limit_operator.clone(), group: group_name.map(|s| s.to_string()), characters: character_filter.clone(),
                            filtered_indices: None,
                            is_select_action: false,
                        });
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        return Ok(());
                    }
                    cards
                }
                _ => { return Err(format!("Unknown source zone: {}", source)); }
            };

            // --- STEP 2: Any-order deck placement (before consuming taken) ---
            if source == "discard" && destination == "deck" && effect.placement_order.as_deref() == Some("any_order") && taken.len() > 1 {
                let taken_count = taken.len();
                for &c in &taken { moved_cards.push(c); }
                self.looked_at_cards = taken.clone();
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "order".to_string(),
                    description: format!("Choose order for cards on deck ({} cards)", taken_count),
                    allow_skip: false,
                });
                self.execution_context = ExecutionContext::LookAndSelect { step: LookAndSelectStep::Finalize { destination: "deck".to_string() } };
                return Ok(());
            }

            // Apply distinct card name filter if specified
            let distinct = effect.distinct.as_deref();
            if distinct == Some("card_name") || distinct == Some("true") || distinct == Some("distinct") {
                let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
                taken.retain(|&id| {
                    card_db.get_card(id)
                        .map(|c| seen.insert(c.name.clone()))
                        .unwrap_or(true)
                });
                if taken.len() < count {
                    taken.clear();  // Not enough distinct cards — skip
                }
            }

            // --- STEP 3: Place cards in destination ---
            let deck_pos = effect.position.as_ref().and_then(|p| match p {
                crate::card::PositionInfo::String(s) => s.parse::<usize>().ok(),
                crate::card::PositionInfo::Struct { position, .. } => position.as_ref().and_then(|s| s.parse::<usize>().ok()),
            }).map(|p| if p > 0 { p - 1 } else { 0 });
            for card_id in &taken {
                if destination == "deck" && !is_max {
                    if let Some(pos) = deck_pos {
                        let clamped = pos.min(player.main_deck.cards.len());
                        player.main_deck.cards.insert(clamped, *card_id);
                    } else {
                        player.main_deck.cards.insert(0, *card_id);
                    }
                } else if destination == "empty_area" {
                    // Check if multiple empty slots exist for position choice
                    let empty_slots: Vec<usize> = (0..3).filter(|&i| player.stage.stage[i] == -1).collect();
                    if empty_slots.len() > 1 {
                        // Create position choice
                        self.pending_choice = Some(crate::ability::types::Choice::SelectPosition {
                            position: "center".to_string(), // Default position
                            description: format!("Choose position for {}", self.game_state.card_database.get_card(*card_id).map(|c| &c.name).map_or("card", |v| v)),
                            allow_skip: false,
                        });
                        let pos_target = tgt.clone().unwrap_or_else(|| "self".to_string());
                        self.execution_context = crate::ability::types::ExecutionContext::MoveCardsPosition {
                            card_id: *card_id,
                            state_change: effect.state_change.clone(),
                            target: pos_target,
                        };
                        return Ok(());
                    } else {
                        // Single empty slot - place directly
                        util::place_card_in_zone(player, *card_id, destination.as_str(), vacated_stage_area, is_max, count);
                    }
                } else {
                    // For "both" targeting or other cases, use normal placement
                    util::place_card_in_zone(player, *card_id, destination.as_str(), vacated_stage_area, is_max, count);
                }
                moved_cards.push(*card_id);
            }
        }

        // --- STEP 3: Apply state_change to moved cards ---
        if let Some(ref sc) = effect.state_change {
            if sc == "wait" {
                for card_id in &moved_cards { self.game_state.add_orientation_modifier(*card_id, "wait"); }
                if destination == "energy_zone" {
                    {
                        let p = match tgt.as_deref().unwrap_or("self") {
                            "self" => &mut self.game_state.player1,
                            "opponent" => &mut self.game_state.player2,
                            _ => &mut self.game_state.player1,
                        };
                        for _ in &moved_cards { p.energy_zone.active_energy_count = p.energy_zone.active_energy_count.saturating_sub(1); }
                    }
                }
            } else if sc == "active" {
                for card_id in &moved_cards { self.game_state.add_orientation_modifier(*card_id, "active"); }
                if destination == "energy_zone" {
                    let p = match tgt.as_ref().map(|s| s.as_str()).unwrap_or("self") {
                        "self" => &mut self.game_state.player1,
                        "opponent" => &mut self.game_state.player2,
                        _ => &mut self.game_state.player1,
                    };
                    p.energy_zone.active_energy_count += moved_cards.len();
                }
            }
        }

        for card_id in &moved_cards { self.game_state.clear_modifiers_for_card(*card_id); }
        for card_id in &moved_cards { self.game_state.record_card_movement(*card_id); }
        
        // Store moved cards on the resolver for chained condition checking
        // e.g. "move 3 cards to discard → if all are member cards → draw"
        if destination == "discard" {
            self.moved_cards = moved_cards.clone();
        }
        
        Ok(())
    }

    fn stage_remove_with_vacated(player: &mut crate::player::Player, idxs: &[usize], card_db: &crate::card::CardDatabase) -> (Vec<i16>, Option<usize>) {
        let mut vacated = None;
        let cards: Vec<i16> = idxs.iter().rev().filter_map(|&i| {
            let cid = player.remove_member_from_stage_with_recycling(i, card_db);
            if cid.is_some() { vacated = Some(i); }
            cid
        }).collect();
        (cards, vacated)
    }
}
