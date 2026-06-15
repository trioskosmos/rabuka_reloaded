use super::enums::Zone;
use super::resolver::AbilityResolver;
use super::types::{Choice, ChoiceRoute, ExecutionContext, LookAndSelectStep};
use crate::card::AbilityEffect;
use crate::game_state::GameState;

impl AbilityResolver {
    pub fn execute_look_and_select(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        self.current_effect = Some(effect.clone());

        if let Some(ref look_action) = effect.compound.look_action {
            println!("DEBUG: Executing look_action: {:?}", look_action);
            self.execute_effect(gs, look_action)?;
            println!(
                "DEBUG: After look_action - looked_at_cards.len(): {}",
                gs.looked_at_cards.len()
            );
        }

        if let Some(ref select_action) = effect.compound.select_action {
            let placement_order = select_action.placement_order.as_deref();
            let any_number = select_action.any_number.unwrap_or(false);
            let count = select_action.count.unwrap_or(1);
            let optional = select_action.optional.unwrap_or(false);

            let card_db = &gs.card_database;
            let filter = super::util::CardFilter::from_effect(select_action);
            let has_filter = filter.card_type.is_some()
                || !filter.heart_colors.is_empty()
                || filter.cost_limit.is_some();
            if has_filter {
                let (matching, non_matching): (Vec<_>, Vec<_>) = gs
                    .looked_at_cards
                    .iter()
                    .partition(|&&card_id| filter.matches(card_db, card_id, false));
                gs.looked_at_cards = matching;
                let player = gs.resolve_target_player_mut("self");
                for &card_id in &non_matching {
                    player.waitroom.add_card(card_id);
                }
            }

            println!(
                "DEBUG: Synced to game_state.looked_at_cards.len(): {}",
                gs.looked_at_cards.len()
            );

            let available_count = gs.looked_at_cards.len();
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
                format!(
                    "Select {} card(s) from the {} looked-at cards (placement_order: {})",
                    max_select,
                    available_count,
                    placement_order.unwrap_or("default")
                )
            };

            let choice = Choice::select_cards(
                Zone::LookedAt.to_str(),
                max_select,
                description.clone(),
                optional || is_max || any_number || available_count == 0,
            )
            .card_type(select_action.card_type.clone())
            .cost_limit(
                select_action.cost_limit,
                select_action.cost_limit_operator.clone(),
            )
            .group(
                select_action
                    .group_names
                    .as_ref()
                    .and_then(|v| v.first().cloned()),
            )
            .characters(select_action.characters.clone())
            .build();
            println!(
                "DEBUG: Creating choice - available_count: {}, max_select: {}, description: {}",
                available_count, max_select, description
            );
            self.pending_choice = Some(choice);
            self.execution_context = ExecutionContext::LookAndSelect {
                step: LookAndSelectStep::Select {
                    count: max_select,
                    max_per_group: select_action.per_group_count,
                },
            };

            // Save followup_action as pending command — it executes after the selection completes.
            // This enables the "その後" (afterwards) pattern where a separate effect runs
            // after the look_and_select finishes (e.g. wait opponent members based on revealed card).
            if let Some(ref followup) = effect.compound.followup_action {
                let mut existing = gs.ability_queue.take_pending_commands();
                existing.push(crate::ability::types::Command::Effect(
                    followup.as_ref().clone(),
                ));
                gs.ability_queue.set_pending_commands(existing);
            }
            println!(
                "DEBUG: Choice created and stored - pending_choice.is_some(): {}",
                self.pending_choice.is_some()
            );

            // Sequential actions will be stored after user makes the choice
            // Don't store them immediately to prevent premature execution

            return Ok(());
        }

        self.current_effect = None;
        Ok(())
    }
    pub fn execute_reveal(
        &mut self,
        gs: &mut GameState,
        source: &str,
        count: u32,
        target: &str,
        card_type: Option<&str>,
        heart_colors: &[String],
        blind: bool,
    ) -> Result<(), String> {
        let card_db = gs.card_database.clone();
        let any_number = self
            .current_effect
            .as_ref()
            .is_some_and(|e| e.any_number.unwrap_or(false));
        let looked_at_len = gs.looked_at_cards.len();
        let player = gs.resolve_target_player_mut(target);

        // Support player selection for reveal: when count allows choice, prompt instead of auto-revealing all
        let available = match Zone::from_str(source) {
            Some(Zone::Hand) => player.hand.cards.len(),
            Some(Zone::LookedAt) => looked_at_len,
            _ => 0,
        };

        println!(
            "DEBUG: execute_reveal - source: {}, available: {}, count: {}, any_number: {}",
            source, available, count, any_number
        );

        if (Zone::from_str(source) == Some(Zone::Hand)
            || Zone::from_str(source) == Some(Zone::LookedAt))
            && available > 0
        {
            let current_effect = self.current_effect.as_ref();
            let is_max = current_effect.is_some_and(|e| e.max.unwrap_or(false));
            let is_optional = current_effect.is_some_and(|e| e.optional.unwrap_or(false));

            println!("DEBUG: is_max: {}, is_optional: {}", is_max, is_optional);

            // Create choice if max=true (up to X cards) or optional, or if count < available
            if is_max || is_optional || count == 0 || count < available as u32 {
                let choices_count = if any_number {
                    available
                } else {
                    count as usize
                };
                let allow_skip = any_number || is_optional || is_max;

                println!(
                    "DEBUG: Creating choice - choices_count: {}, allow_skip: {}",
                    choices_count, allow_skip
                );

                self.pending_choice = Some(
                    Choice::select_cards(
                        source.to_string(),
                        choices_count,
                        format!("Select card(s) to reveal from {}", source),
                        allow_skip,
                    )
                    .card_type(card_type.map(|s| s.to_string()))
                    .cost_limit(
                        self.current_effect.as_ref().and_then(|e| e.cost_limit),
                        self.current_effect
                            .as_ref()
                            .and_then(|e| e.cost_limit_operator.clone()),
                    )
                    .group(
                        self.current_effect
                            .as_ref()
                            .and_then(|e| e.group_names.as_ref())
                            .and_then(|v| v.first().cloned()),
                    )
                    .characters(
                        self.current_effect
                            .as_ref()
                            .and_then(|e| e.characters.clone()),
                    )
                    .target_player_id(Some(target.to_string()))
                    .blind(blind)
                    .is_reveal(true)
                    .build(),
                );
                self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                return Ok(());
            } else {
                println!("DEBUG: Not creating choice - conditions not met");
            }
        } else {
            println!("DEBUG: Not creating choice - source not supported or no available cards");
        }

        let card_ids: Vec<i16> = {
            match Zone::from_str(source) {
                Some(Zone::Hand) => player.hand.cards.iter().copied().collect(),
                Some(Zone::Deck) => player
                    .main_deck
                    .cards
                    .iter()
                    .take(count as usize)
                    .copied()
                    .collect(),
                Some(Zone::LookedAt) => gs
                    .looked_at_cards
                    .iter()
                    .filter(|&&card_id| {
                        super::util::card_matches_type(&card_db, card_id, card_type)
                            && super::util::card_matches_heart_colors(
                                &card_db,
                                card_id,
                                heart_colors,
                            )
                    })
                    .copied()
                    .collect(),
                _ => vec![],
            }
        };

        for card_id in &card_ids {
            gs.revealed_cards.push(*card_id);
        }

        if !card_ids.is_empty() {
            let names: Vec<String> = card_ids
                .iter()
                .filter_map(|id| card_db.get_card(*id))
                .map(|c| c.name.clone())
                .collect();
            let turn = gs.turn_number;
            let master = gs.ability_master_id();
            let player_label = super::util::target_player_label(target, master.as_deref());
            gs.rule_log.push(format!(
                "[Turn {}] {} reveals {} from {}",
                turn,
                player_label,
                names.join(", "),
                source
            ));
        }

        Ok(())
    }
    pub fn execute_select(
        &mut self,
        gs: &mut GameState,
        source: &str,
        count: u32,
        target: &str,
        card_type: Option<&str>,
        distinct: Option<&str>,
        heart_colors: &[String],
        or_card_types: Option<Vec<String>>,
        exclude_selected: bool,
        characters: Option<Vec<String>>,
        group_names: Option<Vec<String>>,
        exclude_self: Option<bool>,
    ) -> Result<(), String> {
        // Handle or_card_types (type choice, e.g. Honoka: pick live_card or member_card)
        if let Some(ref or_types) = or_card_types {
            if !or_types.is_empty() {
                // If already chosen (re-processing after choice was resolved), skip
                let already_chosen = gs
                    .ability_queue
                    .current_entry()
                    .and_then(|e| e.conditional_choice.as_ref())
                    .is_some_and(|cc| or_types.contains(cc));
                if already_chosen {
                    return Ok(());
                }
                let desc_parts: Vec<String> = or_types
                    .iter()
                    .map(|t| {
                        let base = match t.as_str() {
                            "live_card" => "Live card".to_string(),
                            "member_card" => "Member card".to_string(),
                            "energy_card" => "Energy card".to_string(),
                            _ => t.clone(),
                        };
                        let cl = self.current_effect.as_ref().and_then(|e| e.cost_limit);
                        let co = self
                            .current_effect
                            .as_ref()
                            .and_then(|e| e.cost_limit_operator.as_deref());
                        match (cl, co) {
                            (Some(l), Some("<=")) => format!("{} with cost {} or less", base, l),
                            (Some(l), Some(">=")) => format!("{} with cost {} or more", base, l),
                            (Some(l), _) => format!("{} with cost {}", base, l),
                            _ => base,
                        }
                    })
                    .collect();
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "choice_string".to_string(),
                    description: format!("Choose: {}", desc_parts.join(", or ")),
                    allow_skip: false,
                    options: Some(desc_parts.clone()),
                });
                self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                // Store the options as JSON array in conditional_choice so the reveal can read the player's pick
                if let Some(e) = gs.ability_queue.current_entry_mut() { e.conditional_choice = Some(serde_json::to_string(or_types).unwrap()); }
                return Ok(());
            }
        }

        let target = target.to_string();
        let card_db = gs.card_database.clone();
        let player = gs.resolve_target_player_mut(&target);

        let card_ids: Vec<i16> = match Zone::from_str(source) {
            Some(Zone::Hand) => player.hand.cards.iter().copied().collect(),
            Some(Zone::Deck) => player
                .main_deck
                .cards
                .iter()
                .take(count as usize)
                .copied()
                .collect(),
            Some(Zone::Discard) | Some(Zone::Waitroom) => {
                player.waitroom.cards.iter().copied().collect()
            }
            Some(Zone::Stage) => player
                .stage
                .stage
                .iter()
                .filter(|&&id| id != -1)
                .copied()
                .collect(),
            Some(Zone::LookedAt) => gs.looked_at_cards.clone(),
            Some(Zone::SelectedCards) => self.selected_cards.clone(),
            _ => vec![],
        };

        let filtered: Vec<i16> = card_ids
            .iter()
            .filter(|&&card_id| {
                super::util::card_matches_type(&card_db, card_id, card_type)
                    && super::util::card_matches_heart_colors(&card_db, card_id, heart_colors)
            })
            .copied()
            .collect();

        // Apply distinct filter if needed, then check count
        let distinct_filter = super::util::filter_from_parts_full(
            None, None, None, None, None, None, distinct, None, None, None,
        );
        let distinct_indices =
            super::util::filter_distinct(&filtered, &card_db, &distinct_filter, false);
        gs.looked_at_cards = distinct_indices.iter().map(|&i| filtered[i]).collect();

        // Apply additional filters (characters, group, cost_limit) that the choice will validate.
        // These come from parameters or fall back to current_effect.
        let ce_chars = self
            .current_effect
            .as_ref()
            .and_then(|e| e.characters.as_ref());
        let ce_group = self
            .current_effect
            .as_ref()
            .and_then(|e| e.group_names.as_ref())
            .and_then(|v| v.first().map(|s| s.as_str()));
        let ce_cost_limit = self.current_effect.as_ref().and_then(|e| e.cost_limit);
        let ce_cost_op = self
            .current_effect
            .as_ref()
            .and_then(|e| e.cost_limit_operator.as_deref());
        let filter = super::util::CardFilter {
            card_type: None,
            group: group_names
                .as_ref()
                .and_then(|v| v.first().map(|s| s.as_str()))
                .or(ce_group),
            groups: None,
            cost_limit: ce_cost_limit,
            cost_operator: ce_cost_op,
            characters: characters.as_ref().or(ce_chars),
            ..Default::default()
        };
        gs.looked_at_cards
            .retain(|&id| filter.matches(&card_db, id, false));

        // Exclude previously selected cards if exclude_selected is true
        log::debug!(
            "[EXCLUDE_SEL] selected={:?} looked_at_before={:?}",
            self.selected_cards, gs.looked_at_cards
        );
        if exclude_selected && !self.selected_cards.is_empty() {
            gs.looked_at_cards
                .retain(|id| !self.selected_cards.contains(id));
        }
        if exclude_self.unwrap_or(false) {
            if let Some(activating_id) = gs.activating_card {
                gs.looked_at_cards.retain(|&id| id != activating_id);
            }
        }
        log::debug!("[EXCLUDE_SEL] looked_at_after={:?}", gs.looked_at_cards);

        let optional = self
            .current_effect
            .as_ref()
            .is_some_and(|e| e.optional.unwrap_or(false));
        if gs.looked_at_cards.len() < count as usize {
            return Ok(()); // Not enough distinct cards — skip silently
        }
        // Compute filtered stage indices: map looked_at_cards back to stage positions.
        // This ensures handle_select_card looks up the right card when exclude_selected
        // or other filters shift which cards are available.
        let filtered_indices: Option<Vec<usize>> = if Zone::from_str(source) == Some(Zone::Stage) {
            let looked = gs.looked_at_cards.clone();
            Some(
                looked
                    .iter()
                    .filter_map(|&id| {
                        gs.resolve_target_player_mut(&target)
                            .stage
                            .stage
                            .iter()
                            .position(|&sid| sid == id)
                    })
                    .collect(),
            )
        } else {
            None
        };
        self.pending_choice = Some(
            Choice::select_cards(
                source.to_string(),
                count as usize,
                format!("Select {} card(s) from {}", count, source),
                optional,
            )
            .card_type(card_type.map(|s| s.to_string()))
            .cost_limit(
                self.current_effect.as_ref().and_then(|e| e.cost_limit),
                self.current_effect
                    .as_ref()
                    .and_then(|e| e.cost_limit_operator.clone()),
            )
            .group(
                group_names
                    .as_ref()
                    .and_then(|v| v.first().cloned())
                    .or_else(|| {
                        self.current_effect
                            .as_ref()
                            .and_then(|e| e.group_names.as_ref())
                            .and_then(|v| v.first().cloned())
                    }),
            )
            .characters(characters.clone().or_else(|| {
                self.current_effect
                    .as_ref()
                    .and_then(|e| e.characters.clone())
            }))
            .filtered_indices(filtered_indices.clone())
            .target_player_id(Some(target.clone()))
            .is_select_action(true)
            .build(),
        );
        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
        Ok(())
    }
    pub fn execute_select_cards(
        &mut self,
        _gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        self.current_effect = Some(effect.clone());
        Ok(())
    }
    pub fn execute_look_at(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        count: u32,
        target: &str,
        source: &str,
    ) -> Result<(), String> {
        if effect.optional.unwrap_or(false) {
            self.pending_choice = Some(Choice::SelectTarget {
                target: "pay_optional_cost:skip_optional_cost".to_string(),
                description: format!("Look at {} card(s) (optional cost)?", count),
                allow_skip: true,
                options: None,
            });
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.choice_card_no = Some(ChoiceRoute::OptionalCost);
            }
            return Ok(());
        }
        let player = gs.resolve_target_player_mut(target);

        // If deck has fewer cards than requested, the effect cannot execute.
        if (Zone::from_str(source) == Some(Zone::Deck)
            || Zone::from_str(source) == Some(Zone::DeckTop))
            && player.main_deck.cards.len() < count as usize {
                return Err(format!(
                    "Not enough cards in deck: need {}, have {}",
                    count,
                    player.main_deck.cards.len()
                ));
            }

        let cards = match Zone::from_str(source) {
            Some(Zone::Deck) | Some(Zone::DeckTop) => {
                player.main_deck.draw_multiple(count as usize)
            }
            Some(Zone::Hand) => player
                .hand
                .cards
                .iter()
                .take(count as usize)
                .copied()
                .collect(),
            Some(Zone::Discard) | Some(Zone::Waitroom) => player
                .waitroom
                .cards
                .iter()
                .take(count as usize)
                .copied()
                .collect(),
            Some(Zone::Stage) => player
                .stage
                .stage
                .iter()
                .filter(|&&id| id != -1)
                .take(count as usize)
                .copied()
                .collect(),
            Some(Zone::Energy) | Some(Zone::EnergyZone) => player
                .energy_zone
                .cards
                .iter()
                .take(count as usize)
                .copied()
                .collect(),
            _ => vec![],
        };

        gs.looked_at_cards = cards;
        Ok(())
    }
    pub fn execute_reveal_per_group(
        &mut self,
        gs: &mut GameState,
        source: &str,
        count: u32,
        target: &str,
    ) -> Result<(), String> {
        let card_db = gs.card_database.clone();
        let card_ids: Vec<i16> = {
            let player = gs.resolve_target_player_mut(target);
            match Zone::from_str(source) {
                Some(Zone::Hand) => player.hand.cards.iter().copied().collect(),
                Some(Zone::Deck) => player
                    .main_deck
                    .cards
                    .iter()
                    .take(count as usize)
                    .copied()
                    .collect(),
                Some(Zone::Discard) | Some(Zone::Waitroom) => {
                    player.waitroom.cards.iter().copied().collect()
                }
                Some(Zone::LookedAt) => gs.looked_at_cards.clone(),
                _ => vec![],
            }
        };

        let mut by_group: std::collections::HashMap<String, Vec<i16>> =
            std::collections::HashMap::new();
        for &card_id in &card_ids {
            let group_name = card_db
                .get_card(card_id)
                .map(|c| c.group.clone())
                .unwrap_or_default();
            by_group.entry(group_name).or_default().push(card_id);
        }

        for members in by_group.values() {
            for &card_id in members {
                gs.revealed_cards.push(card_id);
            }
        }

        if !card_ids.is_empty() {
            let turn = gs.turn_number;
            let master = gs.ability_master_id();
            let player_label = super::util::target_player_label(target, master.as_deref());
            gs.rule_log.push(format!(
                "[Turn {}] {} reveals {} cards from {} per group",
                turn,
                player_label,
                card_ids.len(),
                source
            ));
        }

        Ok(())
    }
    /// Draw from deck until `termination_check` passes, refreshing from waitroom if deck empties.
    fn reveal_until<F>(
        &mut self,
        gs: &mut GameState,
        target: &str,
        termination_check: F,
    ) -> (Vec<i16>, Option<usize>)
    where
        F: Fn(&crate::card::CardDatabase, i16) -> bool,
    {
        let card_db = gs.card_database.clone();
        let mut all_revealed = Vec::new();
        let mut matched_idx = None;

        loop {
            let card_id = {
                let player = gs.resolve_target_player_mut(target);
                player.main_deck.draw()
            };
            match card_id {
                Some(cid) => {
                    all_revealed.push(cid);
                    gs.revealed_cards.push(cid);
                    if termination_check(&card_db, cid) {
                        matched_idx = Some(all_revealed.len() - 1);
                        break;
                    }
                }
                None => {
                    let player = gs.resolve_target_player_mut(target);
                    let refresh_count = player.waitroom.cards.len();
                    if refresh_count == 0 {
                        break;
                    }
                    for _ in 0..refresh_count {
                        if let Some(card) = player.waitroom.cards.pop() {
                            player.main_deck.cards.push(card);
                        }
                    }
                    player.main_deck.shuffle();
                    if let Some(cid) = player.main_deck.draw() {
                        all_revealed.push(cid);
                        gs.revealed_cards.push(cid);
                        if termination_check(&card_db, cid) {
                            matched_idx = Some(all_revealed.len() - 1);
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        if !all_revealed.is_empty() {
            let turn = gs.turn_number;
            let master = gs.ability_master_id();
            let player_label = super::util::target_player_label(target, master.as_deref());
            let found_str = matched_idx.map(|_| " (found target)").unwrap_or("");
            let names: Vec<String> = all_revealed
                .iter()
                .filter_map(|id| card_db.get_card(*id))
                .map(|c| c.name.clone())
                .collect();
            gs.rule_log.push(format!(
                "[Turn {}] {} reveals {} from deck{}",
                turn,
                player_label,
                names.join(", "),
                found_str
            ));
        }

        (all_revealed, matched_idx)
    }

    pub fn execute_reveal_until_live_card(
        &mut self,
        gs: &mut GameState,
        target: &str,
    ) -> Result<(), String> {
        let (all_revealed, _) = self.reveal_until(gs, target, |card_db, cid| {
            card_db.get_card(cid).map(|c| c.is_live()).unwrap_or(false)
        });
        gs.looked_at_cards = all_revealed;
        Ok(())
    }

    pub fn execute_reveal_until_target(
        &mut self,
        gs: &mut GameState,
        target: &str,
        card_type: Option<&str>,
        cost_limit: Option<u32>,
        cost_limit_operator: Option<&str>,
    ) -> Result<(), String> {
        let card_type_owned = card_type.map(|s| s.to_string());
        let cost_limit_owned = cost_limit;
        let cost_op_owned = cost_limit_operator.map(|s| s.to_string());
        // cost_limit only applies when the selected card_type is member_card
        // (ability text: "live_card or member_card with cost >= 10")
        let apply_cost =
            cost_limit_owned.is_some() && card_type_owned.as_deref() == Some("member_card");
        let (mut all_revealed, matched_idx) = self.reveal_until(gs, target, move |card_db, cid| {
            if !super::util::card_matches_type(card_db, cid, card_type_owned.as_deref()) {
                return false;
            }
            if apply_cost {
                if let Some(lim) = cost_limit_owned {
                    if !super::util::card_matches_cost_limit_op(
                        card_db,
                        cid,
                        Some(lim),
                        cost_op_owned.as_deref(),
                    ) {
                        return false;
                    }
                }
            }
            true
        });

        if let Some(idx) = matched_idx {
            let matched = all_revealed.remove(idx);
            gs.looked_at_cards = std::iter::once(matched).chain(all_revealed).collect();
        } else {
            gs.looked_at_cards.clear();
        }
        Ok(())
    }
}
