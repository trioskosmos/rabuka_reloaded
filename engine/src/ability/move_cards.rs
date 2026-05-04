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

fn remove_cards_from_hand(player: &mut crate::player::Player, indices: &[usize]) -> Vec<i16> {
    indices.iter().rev().map(|&i| player.hand.cards.remove(i)).collect()
}

#[allow(dead_code)]
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
        let card_db = self.game_state.card_database.clone();
        let activating_card_id = self.game_state.activating_card;
        let vacated_stage_area = self.game_state.last_vacated_stage_area;
        self.game_state.last_vacated_stage_area = None;

        let mut moved_cards: Vec<i16> = Vec::new();

        {
            let player = match tgt.as_str() {
                "self" => &mut self.game_state.player1,
                "opponent" => &mut self.game_state.player2,
                _ => &mut self.game_state.player1,
            };

            // --- STEP 1: Get cards from source ---
            let taken: Vec<i16> = match source.as_str() {
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
                    let idxs = util::matching_indices(&player.hand.cards, &card_db, card_type_filter, group_name, cost_limit);
                    if idxs.len() < count { vec![] }
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
                    let idxs = util::matching_indices(&player.waitroom.cards, &card_db, card_type_filter, group_name, cost_limit);
                    if idxs.len() < count {
                        return Err(format!("Not enough cards in discard: need {}", count));
                    } else if idxs.len() > count {
                        self.pending_choice = Some(Choice::SelectCard {
                            zone: "discard".to_string(), card_type: card_type_filter.map(|s| s.to_string()),
                            count, description: format!("Select {} card(s) from discard", count), allow_skip: false,
                        });
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        return Ok(());
                    }
                    idxs.into_iter().map(|i| player.waitroom.cards.remove(i)).collect()
                }
                "energy_zone" => {
                    let idxs = util::matching_indices(&player.energy_zone.cards, &card_db, card_type_filter, None, None);
                    if idxs.len() < count {
                        return Err(format!("Not enough cards in energy zone: need {}", count));
                    } else if idxs.len() > count {
                        self.pending_choice = Some(Choice::SelectCard {
                            zone: "energy_zone".to_string(), card_type: card_type_filter.map(|s| s.to_string()),
                            count, description: format!("Select {} card(s) from energy zone", count), allow_skip: false,
                        });
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        return Ok(());
                    }
                    idxs.into_iter().map(|i| player.energy_zone.cards.remove(i)).collect()
                }
                "live_card_zone" => {
                    let idxs = util::matching_indices(&player.live_card_zone.cards, &card_db, Some("live_card"), group_name, cost_limit);
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
                    idxs.into_iter().map(|i| player.live_card_zone.cards.remove(i)).collect()
                }
                "success_live_zone" => {
                    let idxs = util::matching_indices(&player.success_live_card_zone.cards, &card_db, None::<&str>, None::<&str>, None);
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
                    idxs.into_iter().map(|i| player.success_live_card_zone.cards.remove(i)).collect()
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

            // --- STEP 3: Place cards in destination ---
            for card_id in taken {
                let dest = destination.as_str();
                match dest {
                    "hand" => player.hand.add_card(card_id),
                    "discard" | "" => player.waitroom.add_card(card_id),
                    "stage" | "empty_area" => {
                        let empty_slots: Vec<usize> = (0..3).filter(|&i| player.stage.stage[i] == -1).collect();
                        if is_max && empty_slots.len() < count { /* skip placement if no room */ }
                        else if let Some(pos) = stage_first_empty(&player.stage.stage) {
                            player.stage.stage[pos] = card_id;
                            player.areas_locked_this_turn.insert(pos_to_area(pos));
                        } else {
                            player.hand.add_card(card_id);
                        }
                    }
                    "deck" => {
                        let pos = effect.position.as_ref().and_then(|p| p.get_position()).and_then(|s| s.parse::<usize>().ok());
                        let insert = pos.map(|p| p.saturating_sub(1)).unwrap_or(0).min(player.main_deck.cards.len());
                        player.main_deck.cards.insert(insert, card_id);
                    }
                    "deck_top" => { player.main_deck.cards.insert(0, card_id); }
                    "deck_bottom" => { player.main_deck.cards.push(card_id); }
                    "energy_zone" => { player.energy_zone.cards.push(card_id); }
                    "live_card_zone" => { player.live_card_zone.cards.push(card_id); }
                    "success_live_zone" => { player.success_live_card_zone.cards.push(card_id); }
                    "same_area" => {
                        if let Some(pos) = vacated_stage_area {
                            if pos < 3 && player.stage.stage[pos] == -1 {
                                player.stage.stage[pos] = card_id;
                                player.areas_locked_this_turn.insert(pos_to_area(pos));
                            } else if let Some(ep) = stage_first_empty(&player.stage.stage) {
                                player.stage.stage[ep] = card_id;
                                player.areas_locked_this_turn.insert(pos_to_area(ep));
                            } else { player.hand.add_card(card_id); }
                        }
                    }
                    _ => { player.hand.add_card(card_id); }
                }
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
