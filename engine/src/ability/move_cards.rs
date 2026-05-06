use crate::card::AbilityEffect;
use super::types::{Choice, ExecutionContext, LookAndSelectStep};
use super::resolver::AbilityResolver;
use super::util;

fn remove_cards_from_hand(player: &mut crate::player::Player, indices: &[usize]) -> Vec<i16> {
    indices.iter().rev().map(|&i| player.hand.cards.remove(i)).collect()
}

impl<'a> AbilityResolver<'a> {
pub fn execute_move_cards(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let m = effect.extract_modifiers();
        let count = m.count as usize;
        let source = m.source.as_deref().unwrap_or("").to_string();
        let destination = m.destination.as_deref().unwrap_or("").to_string();
        let card_type_filter = m.card_type.as_deref();
        let group_name = m.group_name.as_deref();
        let tgt = m.target.clone();
        let cost_limit = m.cost_limit;
        let is_self_cost = effect.self_cost.unwrap_or(false);
        let exclude_self = m.exclude_self;
        let is_max = m.max;
        let is_all = m.all;
        let card_db = self.game_state.card_database.clone();
        let activating_card_id = self.game_state.activating_card;
        let vacated_stage_area = self.game_state.last_vacated_stage_area;
        self.game_state.last_vacated_stage_area = None;

        // Character name filter from universal ActionModifiers
        let character_filter: Option<Vec<String>> = m.characters.clone();

        // Resolve name_constraint (e.g. "contains_all" from a revealed card)
        let name_fragments: Option<Vec<String>> = if effect.name_constraint.as_deref() == Some("contains_all")
            && effect.name_constraint_source.as_deref() == Some("revealed_card")
        {
            let fragments: Vec<String> = self.game_state.revealed_cost_cards.iter()
                .chain(self.game_state.revealed_cards.iter())
                .filter_map(|&id| {
                    let card = self.game_state.card_database.get_card(id)?;
                    Some(card.name.replace('＆', "&").split('&').map(|s| s.to_string()).collect::<Vec<_>>())
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

        {
            let player = match tgt.as_str() {
                "self" => &mut self.game_state.player1,
                "opponent" => &mut self.game_state.player2,
                _ => &mut self.game_state.player1,
            };

            // --- STEP 1: Get cards from source ---
            let mut taken: Vec<i16> = match source.as_str() {
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
                    for _ in 0..count {
                        if let Some(card) = player.energy_deck.draw() {
                            drawn.push(card);
                        } else { break; }
                    }
                    drawn
                }

                // Stage → anything
                "stage" => {
                    if is_self_cost {
                        if let Some(activating_id) = activating_card_id {
                            let mut found = false;
                            let mut cards = Vec::new();
                            for i in 0..3 {
                                if player.stage.stage[i] == activating_id {
                                    self.game_state.last_vacated_stage_area = Some(i);
                                    cards.push(player.stage.stage[i]);
                                    player.stage.stage[i] = -1;
                                    if destination == "same_area" {
                                        player.stage.stage[i] = activating_id;
                                        self.game_state.last_vacated_stage_area = None;
                                    }
                                    found = true; break;
                                }
                            }
                            if !found { return Err(format!("Activating card {} not found at stage", activating_id)); }
                            cards
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
                        let cards: Vec<i16> = valid.into_iter().take(count).map(|(i, _)| {
                            let cid = player.stage.stage[i];
                            player.stage.stage[i] = -1;
                            cid
                        }).collect();
                        cards
                    }
                }

                // Zones with player selection — select cards via matching indices
                "hand" => {
                    let filter = util::CardFilter {
                        card_type: card_type_filter,
                        group: group_name,
                        cost_limit,
                        characters: character_filter.as_ref(),
                        name_fragments: name_fragments.as_ref(),
                        ..util::CardFilter::default()
                    };
                    let idxs = util::matching_indices(&player.hand.cards, &card_db, &filter, false);
                    if is_all { idxs.iter().rev().map(|&i| player.hand.cards.remove(i)).collect() }
                    else if idxs.len() < count { vec![] }
                    else if idxs.len() > count {
                        self.pending_choice = Some(Choice::SelectCard {
                            zone: "hand".to_string(), card_type: card_type_filter.map(|s| s.to_string()),
                            count, description: format!("Select {} card(s) from hand", count), allow_skip: false,
                        });
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        return Ok(());
                    } else {
                        remove_cards_from_hand(player, &idxs)
                    }
                }
                "discard" => {
                    let filter = util::CardFilter {
                        card_type: card_type_filter,
                        group: group_name,
                        cost_limit,
                        characters: character_filter.as_ref(),
                        name_fragments: name_fragments.as_ref(),
                        ..util::CardFilter::default()
                    };
                    let mut idxs = util::matching_indices(&player.waitroom.cards, &card_db, &filter, false);
                    idxs.retain(|&i| i < player.waitroom.cards.len());
                    let taken: Vec<i16> = if is_all {
                        idxs.iter().rev().map(|&i| player.waitroom.cards.remove(i)).collect()
                    } else if idxs.len() < count {
                        vec![]  // No matching cards — effect does nothing gracefully
                    } else if idxs.len() > count {
                        self.pending_choice = Some(Choice::SelectCard {
                            zone: "discard".to_string(), card_type: card_type_filter.map(|s| s.to_string()),
                            count, description: format!("Select {} card(s) from discard", count), allow_skip: false,
                        });
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        return Ok(());
                    } else {
                        idxs.iter().rev().map(|&i| player.waitroom.cards.remove(i)).collect()
                    };
                    taken
                }
                "energy_zone" => {
                    let filter = util::CardFilter { card_type: card_type_filter, characters: character_filter.as_ref(), ..util::CardFilter::default() };
                    let idxs = util::matching_indices(&player.energy_zone.cards, &card_db, &filter, false);
                    let taken: Vec<i16> = if is_all {
                        idxs.iter().rev().map(|&i| player.energy_zone.cards.remove(i)).collect()
                    } else if idxs.len() < count {
                        return Err(format!("Not enough cards in energy zone: need {}", count));
                    } else if idxs.len() > count {
                        self.pending_choice = Some(Choice::SelectCard {
                            zone: "energy_zone".to_string(), card_type: card_type_filter.map(|s| s.to_string()),
                            count, description: format!("Select {} card(s) from energy zone", count), allow_skip: false,
                        });
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        return Ok(());
                    } else {
                        idxs.iter().rev().map(|&i| player.energy_zone.cards.remove(i)).collect()
                    };
                    taken
                }
                "live_card_zone" => {
                    let filter = util::CardFilter {
                        card_type: Some("live_card"),
                        group: group_name,
                        cost_limit,
                        characters: character_filter.as_ref(),
                        ..util::CardFilter::default()
                    };
                    let idxs = util::matching_indices(&player.live_card_zone.cards, &card_db, &filter, false);
                    if idxs.len() < count {
                        return Err(format!("Not enough cards in live card zone: need {}", count));
                    } else if idxs.len() > count {
                        self.pending_choice = Some(Choice::SelectCard {
                            zone: "live_card_zone".to_string(), card_type: Some("live_card".to_string()),
                            count, description: format!("Select {} card(s) from live card zone", count), allow_skip: false,
                        });
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        return Ok(());
                    }
                    idxs.iter().rev().map(|&i| player.live_card_zone.cards.remove(i)).collect()
                }
                "success_live_zone" => {
                    let filter = util::CardFilter { characters: character_filter.as_ref(), ..util::CardFilter::default() };
                    let idxs = util::matching_indices(&player.success_live_card_zone.cards, &card_db, &filter, false);
                    if idxs.len() < count {
                        return Err(format!("Not enough cards in success live zone: need {}", count));
                    } else if idxs.len() > count {
                        self.pending_choice = Some(Choice::SelectCard {
                            zone: "success_live_zone".to_string(), card_type: None,
                            count, description: format!("Select {} card(s) from success live zone", count), allow_skip: false,
                        });
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        return Ok(());
                    }
                    idxs.iter().rev().map(|&i| player.success_live_card_zone.cards.remove(i)).collect()
                }
                "those_cards" => {
                    let filter = util::CardFilter {
                        card_type: card_type_filter,
                        group: group_name,
                        characters: character_filter.as_ref(),
                        ..util::CardFilter::default()
                    };
                    let idxs = util::matching_indices(&player.waitroom.cards, &card_db, &filter, false);
                    if idxs.len() < count { vec![] }
                    else if idxs.len() > count {
                        self.pending_choice = Some(Choice::SelectCard {
                            zone: "discard".to_string(), card_type: card_type_filter.map(|s| s.to_string()),
                            count, description: format!("Select {} card(s) from those cards", count), allow_skip: false,
                        });
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        return Ok(());
                    } else {
                        idxs.iter().rev().map(|&i| player.waitroom.cards.remove(i)).collect()
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
                        });
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        return Ok(());
                    } else {
                        idxs.sort_unstable_by(|a, b| b.cmp(a));
                        let taken: Vec<i16> = idxs.iter().map(|&i| self.looked_at_cards.remove(i)).collect();
                        taken
                    }
                }
                "looked_at_remaining" => {
                    let cards: Vec<i16> = self.looked_at_cards.drain(..).collect();
                    cards
                }
                _ => { return Err(format!("Unknown source zone: {}", source)); }
            };

            // --- STEP 2: Any-order deck placement (before consuming taken) ---
            if source == "discard" && destination == "deck" && effect.placement_order.as_deref() == Some("any_order") && taken.len() > 1 {
                for &c in &taken { moved_cards.push(c); }
                self.looked_at_cards = taken;
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "order".to_string(), description: "Choose order for cards on deck".to_string(),
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
            for card_id in taken {
                if destination == "deck" {
                    if let Some(ref pos_info) = effect.position {
                        if let Some(pos_str) = pos_info.get_position() {
                            if let Ok(n) = pos_str.parse::<usize>() {
                                let idx = n.saturating_sub(1).min(player.main_deck.cards.len());
                                player.main_deck.cards.insert(idx, card_id);
                                moved_cards.push(card_id);
                                continue;
                            }
                        }
                    }
                    player.main_deck.cards.insert(0, card_id);
                    moved_cards.push(card_id);
                    continue;
                }
                util::place_card_in_zone(player, card_id, destination.as_str(), vacated_stage_area, is_max, count);
                moved_cards.push(card_id);
            }
        } // player borrow ends here

        // --- STEP 3: Apply state_change to moved cards ---
        if let Some(ref sc) = effect.state_change {
            if sc == "wait" {
                for card_id in &moved_cards { self.game_state.add_orientation_modifier(*card_id, "wait"); }
                if destination == "energy_zone" {
                    {
                        let p = match tgt.as_str() {
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
                    let p = match tgt.as_str() {
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
        Ok(())
    }
}
