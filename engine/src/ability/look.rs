use crate::card::AbilityEffect;
use super::types::{Choice, ExecutionContext, LookAndSelectStep};
use super::resolver::AbilityResolver;

impl<'a> AbilityResolver<'a> {
    pub fn execute_look_and_select(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        self.current_effect = Some(effect.clone());

        if let Some(ref look_action) = effect.compound.look_action {
            println!("DEBUG: Executing look_action: {:?}", look_action);
            self.execute_effect(look_action)?;
            println!("DEBUG: After look_action - looked_at_cards.len(): {}", self.looked_at_cards.len());
        }

        if let Some(ref select_action) = effect.compound.select_action {
            let placement_order = select_action.placement_order.as_deref();
            let count = select_action.count.unwrap_or(1);
            let optional = select_action.optional.unwrap_or(false);
            let any_number = select_action.any_number.unwrap_or(false);

            let card_db = &self.game_state.card_database;
            let card_type_filter = select_action.card_type.as_deref();
            let heart_colors_filter = &select_action.heart_colors;
            let has_filter = card_type_filter.is_some() || !heart_colors_filter.is_empty();
            if has_filter {
                println!("DEBUG: Filtering looked_at_cards - before: {}", self.looked_at_cards.len());
                let (matching, non_matching): (Vec<_>, Vec<_>) = self.looked_at_cards.iter()
                    .partition(|&&card_id| {
                        super::util::card_matches_type(card_db, card_id, card_type_filter)
                            && super::util::card_matches_heart_colors(card_db, card_id, heart_colors_filter)
                    });
                println!("DEBUG: Filtering results - matching: {}, non_matching: {}", matching.len(), non_matching.len());
                self.looked_at_cards = matching;
                let player = self.game_state.resolve_target_player_mut("self");
                for &card_id in &non_matching {
                    player.waitroom.add_card(card_id);
                }
                println!("DEBUG: After filtering - looked_at_cards.len(): {}", self.looked_at_cards.len());
            }

            // Always sync to game_state so handle_select_cards_looked_at can access them
            self.game_state.looked_at_cards = self.looked_at_cards.clone();
            println!("DEBUG: Synced to game_state.looked_at_cards.len(): {}", self.game_state.looked_at_cards.len());

            let available_count = self.looked_at_cards.len();
            let is_max = select_action.max.unwrap_or(false);
            let max_select = if any_number { 
                available_count 
            } else if is_max || optional {
                // When max: true or optional: true, allow up to count cards
                std::cmp::min(count as usize, available_count)
            } else {
                // When neither max nor optional, require exactly count cards
                std::cmp::min(count as usize, available_count)
            };
            
            
            let description = if available_count == 0 {
                "No eligible cards found among looked-at cards".to_string()
            } else if any_number {
                format!("Select any number of cards from the {} looked-at cards (or skip) (placement_order: {})",
                    available_count, placement_order.unwrap_or("default"))
            } else if is_max || optional {
                format!("Select up to {} card(s) from the {} looked-at cards (or skip) (placement_order: {})",
                    max_select, available_count, placement_order.unwrap_or("default"))
            } else {
                format!("Select {} card(s) from the {} looked-at cards (placement_order: {})",
                    max_select, available_count, placement_order.unwrap_or("default"))
            };

            let choice = Choice::SelectCard {
                zone: "looked_at".to_string(), card_type: select_action.card_type.clone(), count: max_select,
                description: description.clone(), allow_skip: optional || is_max || any_number || available_count == 0,
                cost_limit: None, cost_limit_operator: None, group: None, characters: None,
                filtered_indices: None,
                is_select_action: false,
            };
            println!("DEBUG: Creating choice - available_count: {}, max_select: {}, description: {}", available_count, max_select, description);
            self.pending_choice = Some(choice);
            self.execution_context = ExecutionContext::LookAndSelect {
                step: LookAndSelectStep::Select { count: max_select },
            };
            println!("DEBUG: Choice created and stored - pending_choice.is_some(): {}", self.pending_choice.is_some());
            
            // Sequential actions will be stored after user makes the choice
            // Don't store them immediately to prevent premature execution
            
            return Ok(());
        }

        self.current_effect = None;
        Ok(())
    }
    pub fn execute_reveal(&mut self, source: &str, count: u32, target: &str, card_type: Option<&str>, heart_colors: &[String]) -> Result<(), String> {
        let card_db = self.game_state.card_database.clone();
        let any_number = self.current_effect.as_ref().map_or(false, |e| e.any_number.unwrap_or(false));
        let player = self.game_state.resolve_target_player_mut(target);

        // Support player selection for reveal: when count allows choice, prompt instead of auto-revealing all
        let available = match source {
            "hand" => player.hand.cards.len(),
            "looked_at" => self.looked_at_cards.len(),
            _ => 0,
        };
        
        println!("DEBUG: execute_reveal - source: {}, available: {}, count: {}, any_number: {}", source, available, count, any_number);
        
        if (source == "hand" || source == "looked_at") && available > 0 {
            let current_effect = self.current_effect.as_ref();
            let is_max = current_effect.map_or(false, |e| e.max.unwrap_or(false));
            let is_optional = current_effect.map_or(false, |e| e.optional.unwrap_or(false));
            
            println!("DEBUG: is_max: {}, is_optional: {}", is_max, is_optional);
            
            // Create choice if max=true (up to X cards) or optional, or if count < available
            if is_max || is_optional || count == 0 || count < available as u32 {
                let choices_count = if any_number { available } else { count as usize };
                let allow_skip = any_number || is_optional || is_max;
                
                println!("DEBUG: Creating choice - choices_count: {}, allow_skip: {}", choices_count, allow_skip);
                
                self.pending_choice = Some(Choice::SelectCard {
                    zone: source.to_string(), 
                    card_type: card_type.map(|s| s.to_string()),
                    count: choices_count, 
                    description: format!("Select card(s) to reveal from {}", source),
                    allow_skip,
                    cost_limit: None, 
                    cost_limit_operator: None, 
                    group: None, 
                    characters: None,
                    filtered_indices: None,
                    is_select_action: false,
                });
                self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                return Ok(());
            } else {
                println!("DEBUG: Not creating choice - conditions not met");
            }
        } else {
            println!("DEBUG: Not creating choice - source not supported or no available cards");
        }

        let card_ids: Vec<i16> = {
            match source {
                "hand" => player.hand.cards.iter().copied().collect(),
                "deck" => player.main_deck.cards.iter().take(count as usize).copied().collect(),
                "looked_at" => self.looked_at_cards.iter().filter(|&&card_id| {
                    super::util::card_matches_type(&card_db, card_id, card_type)
                        && super::util::card_matches_heart_colors(&card_db, card_id, heart_colors)
                }).copied().collect(),
                _ => vec![],
            }
        };

        for card_id in &card_ids { self.game_state.revealed_cards.push(*card_id); }
        Ok(())
    }
    pub fn execute_select(&mut self, source: &str, count: u32, target: &str, card_type: Option<&str>, distinct: Option<&str>, heart_colors: &[String], or_card_types: Option<Vec<String>>, exclude_selected: bool) -> Result<(), String> {

        // Handle or_card_types (type choice, e.g. Honoka: pick live_card or member_card)
        if let Some(ref or_types) = or_card_types {
            if !or_types.is_empty() {
                // If already chosen (re-processing after choice was resolved), skip
                let already_chosen = self.game_state.ability_queue.current_entry()
                    .and_then(|e| e.conditional_choice.as_ref())
                    .map_or(false, |cc| or_types.contains(cc));
                if already_chosen {
                    return Ok(());
                }
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "choice_string".to_string(),
                    description: format!("Pick card type: {:?}", or_types),
                    allow_skip: false,
                });
                self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                // Store the options as JSON array in conditional_choice so the reveal can read the player's pick
                self.game_state.ability_queue.current_entry_mut().map(|e| {
                    e.conditional_choice = Some(serde_json::to_string(or_types).unwrap());
                });
                return Ok(());
            }
        }

        let target = target.to_string();
        let card_db = self.game_state.card_database.clone();
        let player = self.game_state.resolve_target_player_mut(&target);

        let card_ids: Vec<i16> = match source {
            "hand" => player.hand.cards.iter().copied().collect(),
            "deck" => player.main_deck.cards.iter().take(count as usize).copied().collect(),
            "discard" => player.waitroom.cards.iter().copied().collect(),
            "stage" => player.stage.stage.iter().filter(|&&id| id != -1).copied().collect(),
            "looked_at" => self.looked_at_cards.clone(),
            "selected_cards" => self.selected_cards.clone(),
            _ => vec![],
        };

        let filtered: Vec<i16> = card_ids.iter().filter(|&&card_id| {
            super::util::card_matches_type(&card_db, card_id, card_type)
                && super::util::card_matches_heart_colors(&card_db, card_id, heart_colors)
        }).copied().collect();

        // Apply distinct filter if needed, then check count
        let distinct_filter = super::util::filter_from_parts_full(None, None, None, None, None, None, distinct, None);
        let distinct_indices = super::util::filter_distinct(&filtered, &card_db, &distinct_filter, false);
        self.looked_at_cards = distinct_indices.iter().map(|&i| filtered[i]).collect();

        // Exclude previously selected cards if exclude_selected is true
        if exclude_selected && !self.selected_cards.is_empty() {
            self.looked_at_cards.retain(|id| !self.selected_cards.contains(id));
        }

        if self.looked_at_cards.len() < count as usize {
            return Ok(());  // Not enough distinct cards — skip silently
        }
        self.pending_choice = Some(Choice::SelectCard {
            zone: source.to_string(), card_type: card_type.map(|s| s.to_string()),
            count: count as usize,
            description: format!("Select {} card(s) from {}", count, source),
            allow_skip: false,
            cost_limit: None, cost_limit_operator: None, group: None, characters: None,
            filtered_indices: None,
            is_select_action: true,
        });
        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
        Ok(())
    }
    pub fn execute_select_cards(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        self.current_effect = Some(effect.clone());
        Ok(())
    }
    pub fn execute_look_at(&mut self, count: u32, target: &str, source: &str) -> Result<(), String> {
        let player = self.game_state.resolve_target_player_mut(target);

        // If deck has fewer cards than requested, the effect cannot execute.
        if source == "deck" || source == "deck_top" {
            if player.main_deck.cards.len() < count as usize {
                return Err(format!("Not enough cards in deck: need {}, have {}", count, player.main_deck.cards.len()));
            }
        }

        let cards = match source {
            "deck" | "deck_top" => player.main_deck.draw_multiple(count as usize),
            "hand" => player.hand.cards.iter().take(count as usize).copied().collect(),
            "discard" => player.waitroom.cards.iter().take(count as usize).copied().collect(),
            "stage" => player.stage.stage.iter().filter(|&&id| id != -1).take(count as usize).copied().collect(),
            "energy_zone" => player.energy_zone.cards.iter().take(count as usize).copied().collect(),
            _ => vec![],
        };

        self.looked_at_cards = cards;
        Ok(())
    }
    pub fn execute_reveal_per_group(&mut self, source: &str, count: u32, target: &str) -> Result<(), String> {
        let card_db = self.game_state.card_database.clone();
        let card_ids: Vec<i16> = {
            let player = self.game_state.resolve_target_player_mut(target);
            match source {
                "hand" => player.hand.cards.iter().copied().collect(),
                "deck" => player.main_deck.cards.iter().take(count as usize).copied().collect(),
                "discard" => player.waitroom.cards.iter().copied().collect(),
                "looked_at" => self.looked_at_cards.clone(),
                _ => vec![],
            }
        };

        let mut by_group: std::collections::HashMap<String, Vec<i16>> = std::collections::HashMap::new();
        for &card_id in &card_ids {
            let group_name = card_db.get_card(card_id).map(|c| c.group.clone()).unwrap_or_default();
            by_group.entry(group_name).or_default().push(card_id);
        }

        for (_group, members) in &by_group {
            for &card_id in members {
                self.game_state.revealed_cards.push(card_id);
            }
        }
        Ok(())
    }
    /// Draw from deck until `termination_check` passes, refreshing from waitroom if deck empties.
    fn reveal_until<F>(&mut self, target: &str, termination_check: F) -> (Vec<i16>, Option<usize>)
    where
        F: Fn(&crate::card::CardDatabase, i16) -> bool,
    {
        let card_db = self.game_state.card_database.clone();
        let mut all_revealed = Vec::new();
        let mut matched_idx = None;

        loop {
            let card_id = {
                let player = self.game_state.resolve_target_player_mut(target);
                player.main_deck.draw()
            };
            match card_id {
                Some(cid) => {
                    all_revealed.push(cid);
                    self.game_state.revealed_cards.push(cid);
                    if termination_check(&card_db, cid) {
                        matched_idx = Some(all_revealed.len() - 1);
                        break;
                    }
                }
                None => {
                    let player = self.game_state.resolve_target_player_mut(target);
                    let refresh_count = player.waitroom.cards.len();
                    if refresh_count == 0 { break; }
                    for _ in 0..refresh_count {
                        if let Some(card) = player.waitroom.cards.pop() {
                            player.main_deck.cards.push(card);
                        }
                    }
                    player.main_deck.shuffle();
                    if let Some(cid) = player.main_deck.draw() {
                        all_revealed.push(cid);
                        self.game_state.revealed_cards.push(cid);
                        if termination_check(&card_db, cid) {
                            matched_idx = Some(all_revealed.len() - 1);
                            break;
                        }
                    } else { break; }
                }
            }
        }

        (all_revealed, matched_idx)
    }

    pub fn execute_reveal_until_live_card(&mut self, target: &str) -> Result<(), String> {
        let (all_revealed, _) = self.reveal_until(target, |card_db, cid| {
            card_db.get_card(cid).map(|c| c.is_live()).unwrap_or(false)
        });
        self.looked_at_cards = all_revealed;
        Ok(())
    }

    pub fn execute_reveal_until_target(&mut self, target: &str, card_type: Option<&str>) -> Result<(), String> {
        let card_type_owned = card_type.map(|s| s.to_string());
        let (mut all_revealed, matched_idx) = self.reveal_until(target, move |card_db, cid| {
            super::util::card_matches_type(card_db, cid, card_type_owned.as_deref())
        });

        if let Some(idx) = matched_idx {
            let matched = all_revealed.remove(idx);
            self.looked_at_cards = std::iter::once(matched).chain(all_revealed).collect();
        } else {
            self.looked_at_cards.clear();
        }
        Ok(())
    }
}
