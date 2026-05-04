use crate::card::{AbilityCost, AbilityEffect};
use super::types::Choice;
use super::resolver::AbilityResolver;
use super::util;

impl<'a> AbilityResolver<'a> {
    pub fn validate_cost(&self, cost: &AbilityCost) -> Result<(), String> {
        match cost.cost_type.as_deref() {
            Some("sequential_cost") => {
                if let Some(ref costs) = cost.costs {
                    for sub_cost in costs { self.validate_cost(sub_cost)?; }
                }
                Ok(())
            }
            Some("choice_condition") => Ok(()),
            Some("move_cards") => {
                let count = cost.count.unwrap_or(1) as usize;
                let source = cost.source.as_deref().unwrap_or("");
                let player = self.game_state.active_player();
                let available = match source {
                    "hand" => player.hand.cards.len(),
                    "stage" => player.stage.stage.iter().filter(|&&id| id != -1).count(),
                    "waitroom" => player.waitroom.cards.len(),
                    "energy_zone" => player.energy_zone.cards.len(),
                    _ => return Ok(()),
                };
                if available < count {
                    return Err(format!("Not enough cards in {}: need {}, have {}", source, count, available));
                }
                Ok(())
            }
            Some("energy_condition") => {
                let count = cost.count.unwrap_or(1) as usize;
                let player = self.game_state.active_player();
                if player.energy_zone.cards.len() < count {
                    return Err(format!("Not enough energy cards: need {}, have {}", count, player.energy_zone.cards.len()));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn pay_cost_inner(&mut self, cost: &AbilityCost) -> Result<(), String> {
        match cost.cost_type.as_deref() {
            Some("sequential_cost") => {
                if let Some(ref costs) = cost.costs {
                    for sub_cost in costs {
                        if let Err(e) = self.validate_cost(sub_cost) {
                            return Err(format!("Cannot pay sequential cost: {}", e));
                        }
                    }
                    for sub_cost in costs { self.pay_cost(sub_cost)?; }
                }
                Ok(())
            }
            Some("choice_condition") => {
                let texts: Vec<String> = cost.options.as_ref()
                    .map(|o| o.iter().map(|opt| opt.text.clone()).collect())
                    .unwrap_or_default();
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "choice_condition".to_string(),
                    description: format!("Choose cost option: {}", texts.join(" OR ")),
                });
                if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                    entry.choice_card_no = Some("choice_cost".to_string());
                }
                Ok(())
            }
            Some("move_cards") => {
                let source = cost.source.as_deref().unwrap_or("");
                let count = cost.count.unwrap_or(1) as usize;
                let card_type = cost.card_type.clone();
                let optional = cost.optional.unwrap_or(false);
                let text = &cost.text;
                let is_activation = self.current_ability.as_ref()
                    .and_then(|a| a.triggers.as_ref())
                    .map_or(false, |t| t == crate::triggers::ACTIVATION);

                if optional && !is_activation {
                    self.pending_choice = Some(Choice::SelectCard {
                        zone: source.to_string(),
                        card_type: card_type.clone(),
                        count,
                        description: format!("Select card(s) to pay optional cost (or skip): {}", text),
                        allow_skip: true,
                    });
                    return Ok(());
                }

                if !source.is_empty() {
                    let target = cost.target.as_deref().unwrap_or("self");
                    let cost_limit = cost.cost_limit;
                    let card_type_filter = card_type.as_deref();

                    let player = &*self.game_state.resolve_target_player(target);
                    let card_db = &self.game_state.card_database;
                    let character_filter = cost.characters.as_ref();
                    let count_matching_in = |cards: &[i16]| -> usize {
                        cards.iter().filter(|&&card_id| {
                            util::card_matches_type(card_db, card_id, card_type_filter)
                                && util::card_matches_cost_limit(card_db, card_id, cost_limit)
                                && util::card_matches_characters(card_db, card_id, character_filter)
                        }).count()
                    };

                    let matching_count = match source {
                        "deck" | "deck_top" => count_matching_in(&player.main_deck.cards),
                        "hand" => count_matching_in(&player.hand.cards),
                        "discard" => count_matching_in(&player.waitroom.cards),
                        "energy_zone" => count_matching_in(&player.energy_zone.cards),
                        _ => usize::MAX,
                    };

                    if matching_count < count {
                        return Err(format!("Cannot pay cost: {} has only {} cards matching cost limit {}, need {}", source, matching_count, cost_limit.map(|l| l.to_string()).unwrap_or("none".to_string()), count));
                    }
                }

                let effect = AbilityEffect {
                    text: cost.text.clone(), action: cost.cost_type.clone().unwrap_or_default(),
                    source: cost.source.clone(), destination: cost.destination.clone(),
                    count: cost.count, card_type: cost.card_type.clone(), target: cost.target.clone(),
                    self_cost: cost.self_cost, exclude_self: cost.exclude_self,
                    cost_limit: cost.cost_limit, state_change: cost.state_change.clone(),
                    position: cost.position.clone(),
                    effect_type: None, ..Default::default()
                };
                self.execute_move_cards(&effect)
            }
            Some("change_state") => {
                let state_change = cost.state_change.as_deref().unwrap_or("");
                let target = cost.target.as_deref().unwrap_or("self");
                let optional = cost.optional.unwrap_or(false);
                let is_activation = self.current_ability.as_ref()
                    .and_then(|a| a.triggers.as_ref())
                    .map_or(false, |t| t == crate::triggers::ACTIVATION);

                if optional && !is_activation {
                    let cost_description = if state_change == "wait" { "Put this member to wait state" } else { "Pay cost" };
                    self.pending_choice = Some(Choice::SelectTarget {
                        target: "pay_optional_cost:skip_optional_cost".to_string(),
                        description: format!("Pay optional cost: {}? (pay or skip)", cost_description),
                    });
                    if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                        entry.choice_card_no = Some("optional_cost".to_string());
                    }
                    return Ok(());
                }

                if state_change == "wait" {
                    let card_ids: Vec<i16> = self.game_state.resolve_target_player(target).stage.stage.iter().filter(|&&id| id != -1).copied().collect();
                    for card_id in card_ids {
                        self.game_state.add_orientation_modifier(card_id, "wait");
                    }
                }
                Ok(())
            }
            Some("pay_energy") => {
                let energy = cost.energy.unwrap_or(0);
                let target = cost.target.as_deref().unwrap_or("self");
                let optional = cost.optional.unwrap_or(false);
                let is_activation = self.current_ability.as_ref()
                    .and_then(|a| a.triggers.as_ref())
                    .map_or(false, |t| t == crate::triggers::ACTIVATION);

                if optional && !is_activation {
                    self.pending_choice = Some(Choice::SelectTarget {
                        target: "pay_optional_cost:skip_optional_cost".to_string(),
                        description: format!("Pay {} energy (or skip)?", energy),
                    });
                    if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                        entry.choice_card_no = Some("optional_cost".to_string());
                    }
                    return Ok(());
                }

                if self.game_state.baton_touch_zero_cost && energy > 0 {
                    eprintln!("Skipping pay_energy cost of {} due to baton touch zero cost", energy);
                    return Ok(());
                }

                let player = self.game_state.resolve_target_player_mut(target);

                if energy > 0 {
                    if let Err(e) = player.energy_zone.pay_energy(energy as usize) {
                        return Err(e);
                    }
                }
                Ok(())
            }
            Some("energy_condition") => {
                let count = cost.count.unwrap_or(1) as usize;
                let target = cost.target.as_deref().unwrap_or("self");
                let player = self.game_state.resolve_target_player_mut(target);
                if player.energy_zone.cards.len() < count {
                    return Err(format!("Not enough energy cards: need {}, have {}", count, player.energy_zone.cards.len()));
                }
                for _ in 0..count {
                    if let Some(card) = player.energy_zone.cards.pop() {
                        player.energy_deck.cards.push(card);
                    }
                }
                player.energy_zone.active_energy_count = player.energy_zone.active_energy_count.saturating_sub(count);
                Ok(())
            }
            Some("reveal") => {
                let source = cost.source.as_deref().unwrap_or("hand");
                let target = cost.target.as_deref().unwrap_or("self");
                let card_type = cost.card_type.clone();

                let card_ids: Vec<i16> = {
                    let player = &*self.game_state.resolve_target_player(target);
                    let card_db = &self.game_state.card_database;
                    match source {
                        "hand" => player.hand.cards.iter()
                            .filter(|&&id| super::util::card_matches_type(card_db, id, card_type.as_deref()))
                            .copied().collect(),
                        _ => vec![],
                    }
                };

                if card_ids.is_empty() {
                    return Err("No cards to reveal".to_string());
                }

                let has_explicit_count = cost.count.is_some();
                let explicit_count = cost.count.unwrap_or(1) as usize;

                if has_explicit_count && card_ids.len() <= explicit_count {
                    for card_id in card_ids {
                        self.game_state.revealed_cards.insert(card_id);
                        self.revealed_cost_cards.push(card_id);
                    }
                    Ok(())
                } else {
                    self.pending_choice = Some(Choice::SelectCard {
                        zone: source.to_string(),
                        card_type: card_type.clone(),
                        count: if has_explicit_count { explicit_count } else { 0 },
                        description: "Select cards to reveal from hand".to_string(),
                        allow_skip: true,
                    });
                    Ok(())
                }
            }
            Some("place_energy_under_member") => {
                self.execute_place_energy_under_member(
                    cost.count.unwrap_or(1),
                    cost.target.as_deref().unwrap_or("self"),
                    cost.position.as_ref(),
                    cost.optional.unwrap_or(false),
                )
            }
            _ => Ok(()),
        }
    }

    pub fn pay_cost(&mut self, cost: &AbilityCost) -> Result<(), String> {
        eprintln!("PAY_COST: cost_type={:?}, source={:?}, destination={:?}, card_type={:?}", cost.cost_type, cost.source, cost.destination, cost.card_type);
        self.pay_cost_inner(cost)
    }
}

