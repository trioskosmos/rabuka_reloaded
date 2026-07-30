use super::enums::Zone;
use super::resolver::AbilityResolver;
use super::types::{Choice, ChoiceRoute, ExecutionContext, LookAndSelectStep};
use super::util;
use crate::card::AbilityEffect;
use crate::game_state::GameState;
use crate::HashMap;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

impl AbilityResolver {
    pub fn execute_look_and_select(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        self.current_effect = Some(effect.clone());

        if let Some(ref look_action) = effect.compound.look_action {
            self.execute_effect(gs, look_action)?;
        }

        if let Some(ref select_action) = effect.compound.select_action {
            let _placement_order = select_action.placement_order_any();
            let any_number = select_action.any_number_any().unwrap_or(false);
            let count = select_action.count.unwrap_or(1);
            let optional = select_action.optional.unwrap_or(false);

            let card_db = &gs.card_database;
            let total_count = gs.looked_at_cards.len();

            // Compute which looked-at card indices match the filter.
            // Keep ALL cards in looked_at_cards — non-matching ones appear
            // Compute which looked-at card indices match the filter.
            // Keep ALL cards in looked_at_cards — non-matching ones appear
            // greyed out in the choice. filtered_indices restricts selection.
            let matching_indices: Vec<usize> = if let Some(opts) = &select_action.options_any() {
                if opts.is_empty() {
                    // Empty options → skip OR filter, use regular filter below
                    let filter = super::util::CardFilter::from_effect(select_action);
                    if filter.has_filter() {
                        gs.looked_at_cards
                            .iter()
                            .enumerate()
                            .filter(|&(_, &card_id)| filter.matches(card_db, card_id, false))
                            .map(|(i, _)| i)
                            .collect()
                    } else {
                        (0..total_count).collect()
                    }
                } else {
                    gs.looked_at_cards
                        .iter()
                        .enumerate()
                        .filter(|&(_, &card_id)| {
                            opts.iter().any(|opt| {
                                let f = super::util::CardFilter::from_effect(opt);
                                f.matches(card_db, card_id, false)
                            })
                        })
                        .map(|(i, _)| i)
                        .collect()
                }
            } else {
                let filter = super::util::CardFilter::from_effect(select_action);
                if filter.has_filter() {
                    gs.looked_at_cards
                        .iter()
                        .enumerate()
                        .filter(|&(_, &card_id)| filter.matches(card_db, card_id, false))
                        .map(|(i, _)| i)
                        .collect()
                } else {
                    (0..total_count).collect()
                }
            };

            let matching_count = matching_indices.len();
            if matching_count == 0 {
                let cards = core::mem::take(&mut gs.looked_at_cards);
                let player_target = effect.target_name();
                let player = gs.resolve_target_player_mut(player_target);
                for &card_id in &cards {
                    player.waitroom.add_card(card_id);
                }
                if let Some(ref followup) = effect.compound.followup_action {
                    let mut existing = gs.ability_queue.take_pending_actions();
                    existing.push(followup.as_ref().clone());
                    gs.ability_queue.set_pending_actions(existing);
                }
                return Ok(());
            }
            let is_max = select_action.max.unwrap_or(false);
            let max_select = if any_number {
                matching_count
            } else if is_max || optional {
                core::cmp::min(count as usize, matching_count)
            } else {
                core::cmp::min(count as usize, matching_count)
            };

            let description = if any_number {
                format!(
                    "Select any number of cards from the {} looked-at cards (or skip)",
                    total_count
                )
            } else if is_max || optional {
                format!(
                    "Select up to {} card(s) from the {} looked-at cards (or skip)",
                    max_select, total_count
                )
            } else {
                format!(
                    "Select {} card(s) from the {} looked-at cards",
                    max_select, total_count,
                )
            };

            let desc_ja = if any_number {
                format!(
                    "確認した{}枚のカードから好きな枚数を選択（スキップ可）",
                    total_count
                )
            } else if is_max || optional {
                format!(
                    "確認した{}枚のカードから最大{}枚を選択（スキップ可）",
                    total_count, max_select
                )
            } else {
                format!(
                    "確認した{}枚のカードから{}枚を選択",
                    total_count, max_select
                )
            };
            let choice = Choice::select_cards(
                Zone::LookedAt.to_str(),
                max_select,
                description.clone(),
                optional || is_max || any_number,
            )
            .description_ja(Some(desc_ja))
            .card_type(select_action.card_type_any().map(|s| s.to_string()))
            .cost_limit(
                select_action.cost_limit_any(),
                select_action
                    .cost_limit_operator_any()
                    .map(|s| s.to_string()),
            )
            .group(
                select_action
                    .group_names_any()
                    .as_ref()
                    .and_then(|v| v.first().cloned()),
            )
            .characters(select_action.characters_any().cloned())
            .filtered_indices(Some(matching_indices))
            .build();
            self.pending_choice = Some(choice);
            self.execution_context = ExecutionContext::LookAndSelect {
                step: LookAndSelectStep::Select {
                    count: max_select,
                    max_per_group: select_action.per_group_count_any(),
                },
            };

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
        let require_all_heart_colors = self
            .current_effect
            .as_ref()
            .and_then(|e| e.require_all_heart_colors_any())
            .unwrap_or(false);
        let card_db = gs.card_database.clone();
        let any_number = self
            .current_effect
            .as_ref()
            .is_some_and(|e| e.any_number_any().unwrap_or(false));
        let looked_at_len = gs.looked_at_cards.len();
        let player = gs.resolve_target_player_mut(target);

        // Support player selection for reveal: when count allows choice, prompt instead of auto-revealing all
        let available = match Zone::from_str(source) {
            Some(Zone::Hand) => player.hand.cards.len(),
            Some(Zone::LookedAt) => looked_at_len,
            Some(Zone::Deck) | Some(Zone::DeckTop) => player.main_deck.cards.len(),
            _ => 0,
        };

        log::debug!(
            "DEBUG: execute_reveal - source: {}, available: {}, count: {}, any_number: {}",
            source,
            available,
            count,
            any_number
        );

        if (Zone::from_str(source) == Some(Zone::Hand)
            || Zone::from_str(source) == Some(Zone::LookedAt))
            && available > 0
        {
            let current_effect = self.current_effect.as_ref();
            let is_max = current_effect.is_some_and(|e| e.max.unwrap_or(false));
            let is_optional = current_effect.is_some_and(|e| e.optional.unwrap_or(false));

            log::debug!("DEBUG: is_max: {}, is_optional: {}", is_max, is_optional);

            // Create choice if max=true (up to X cards) or optional, or if count < available
            if is_max || is_optional || count == 0 || count < available as u32 {
                let choices_count = if any_number {
                    available
                } else {
                    count as usize
                };
                let allow_skip = any_number || is_optional || is_max;

                log::debug!(
                    "DEBUG: Creating choice - choices_count: {}, allow_skip: {}",
                    choices_count,
                    allow_skip
                );

                let desc_en = format!(
                    "Select card(s) to reveal from {}",
                    crate::ability::describe::zone_label(Some(&source))
                );
                let desc_ja = format!(
                    "{}から公開するカードを選択",
                    crate::ability::describe::zone_label_ja(Some(&source))
                );
                self.pending_choice = Some(
                    Choice::select_cards(source.to_string(), choices_count, desc_en, allow_skip)
                        .description_ja(Some(desc_ja))
                        .card_type(card_type.map(|s| s.to_string()))
                        .cost_limit(
                            self.current_effect
                                .as_ref()
                                .and_then(|e| e.cost_limit_any()),
                            self.current_effect
                                .as_ref()
                                .and_then(|e| e.cost_limit_operator_any())
                                .map(|s| s.to_string()),
                        )
                        .group(
                            self.current_effect
                                .as_ref()
                                .and_then(|e| e.group_names_any())
                                .and_then(|v| v.first().cloned()),
                        )
                        .characters(
                            self.current_effect
                                .as_ref()
                                .and_then(|e| e.characters_any().cloned()),
                        )
                        .target_player_id(Some(target.to_string()))
                        .blind(blind)
                        .is_reveal(true)
                        .build(),
                );
                self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                return Ok(());
            } else {
                log::debug!("DEBUG: Not creating choice - conditions not met");
            }
        } else {
            log::debug!("DEBUG: Not creating choice - source not supported or no available cards");
        }

        let card_ids: Vec<i16> = {
            match Zone::from_str(source) {
                Some(Zone::Hand) => player.hand.cards.iter().copied().collect(),
                Some(Zone::Deck) | Some(Zone::DeckTop) => {
                    let take_count = count.min(player.main_deck.cards.len() as u32) as usize;
                    // Peek only — don't drain from deck here. The card stays
                    // in the deck until the conditional move_cards consumes it.
                    // If the condition fails, the card remains on top of the
                    // deck (the parenthetical "nothing happens" behavior).
                    player.main_deck.cards[..take_count].to_vec()
                }
                Some(Zone::LookedAt) => gs
                    .looked_at_cards
                    .iter()
                    .filter(|&&card_id| {
                        super::util::card_matches_type(&card_db, card_id, card_type) && {
                            if heart_colors.is_empty() {
                                true
                            } else if require_all_heart_colors {
                                super::util::card_matches_all_heart_colors(
                                    &card_db,
                                    card_id,
                                    heart_colors,
                                )
                            } else {
                                super::util::card_matches_heart_colors(
                                    &card_db,
                                    card_id,
                                    heart_colors,
                                )
                            }
                        }
                    })
                    .copied()
                    .collect(),
                _ => vec![],
            }
        };

        let reveal_src = gs.current_ability_source_card_id();
        let owner = util::target_player_index(target, gs.ability_master_id().as_deref());
        for card_id in &card_ids {
            gs.push_revealed_card(*card_id, reveal_src, false, owner, "ability");
        }

        if !card_ids.is_empty() {
            let names: Vec<String> = card_ids
                .iter()
                .filter_map(|id| card_db.get_card(*id))
                .map(|c| c.name.to_string())
                .collect();
            let turn = gs.turn_number;
            let master = gs.ability_master_id();
            let player_label = super::util::target_player_label(target, master.as_deref());
            gs.push_rule_log(format!(
                "[Turn {}] {} [[log_reveal_zone:source={}]] » {}",
                turn,
                player_label,
                source,
                names.join(", ")
            ));
        }

        Ok(())
    }
    pub fn execute_select(
        &mut self,
        gs: &mut GameState,
        source: &str,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        // Handle or_card_types (type choice, e.g. Honoka: pick live_card or member_card)
        let chosen_card_type: Option<String> =
            if let Some(ref or_types) = effect.or_card_types_any() {
                if !or_types.is_empty() {
                    let maybe_cc = gs
                        .ability_queue
                        .current_entry()
                        .and_then(|e| e.conditional_choice.clone());
                    if let Some(ref cc) = maybe_cc {
                        if or_types.contains(cc) {
                            Some(cc.clone())
                        } else {
                            None
                        }
                    } else {
                        let desc_parts: Vec<String> = or_types
                            .iter()
                            .map(|t| {
                                let base = match t.as_str() {
                                    "live_card" => "Live card".to_string(),
                                    "member_card" => "Member card".to_string(),
                                    "energy_card" => "Energy card".to_string(),
                                    _ => t.clone(),
                                };
                                match (
                                    effect.cost_limit_any(),
                                    effect.cost_limit_operator_any().as_deref(),
                                ) {
                                    (Some(l), Some("<=")) => {
                                        format!("{} with cost {} or less", base, l)
                                    }
                                    (Some(l), Some(">=")) => {
                                        format!("{} with cost {} or more", base, l)
                                    }
                                    (Some(l), _) => format!("{} with cost {}", base, l),
                                    _ => base,
                                }
                            })
                            .collect();
                        self.pending_choice = Some(Choice::SelectTarget {
                            target: "choice_string".to_string(),
                            description: format!("Choose: {}", desc_parts.join(", or ")),
                            description_en: Some(format!("Choose: {}", desc_parts.join(", or "))),
                            description_ja: Some(format!("選択: {}", desc_parts.join(", または "))),
                            allow_skip: false,
                            options: Some(desc_parts.clone()),
                        });
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        if let Some(e) = gs.ability_queue.current_entry_mut() {
                            e.conditional_choice = Some(serde_json::to_string(or_types).unwrap());
                        }
                        return Ok(());
                    }
                } else {
                    None
                }
            } else {
                None
            };

        let ct_binding = effect.card_type_any();
        let card_type = chosen_card_type
            .as_deref()
            .or(ct_binding.map(|ct| ct.as_card_str()));
        let target = effect.target_name().to_string();
        let count = effect
            .count
            .or_else(|| {
                effect
                    .dynamic_count_any()
                    .as_ref()
                    .and_then(|dc| self.resolve_dynamic_count(gs, dc).into())
            })
            .unwrap_or(1);
        let optional = effect.optional.unwrap_or(false);
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
            Some(Zone::LookedAt) => gs.looked_at_cards.to_vec(),
            Some(Zone::LiveCardZone) => player.live_card_zone.cards.iter().copied().collect(),
            Some(Zone::SelectedCards) => self.selected_cards.to_vec(),
            Some(Zone::RevealedCards) => gs.revealed_cards.to_vec(),
            _ => vec![],
        };

        let filtered: Vec<i16> = card_ids
            .iter()
            .filter(|&&card_id| {
                super::util::card_matches_type(&card_db, card_id, card_type) && {
                    let hc = effect.heart_colors_any();
                    if hc.is_empty() {
                        true
                    } else if effect.require_all_heart_colors_any().unwrap_or(false) {
                        super::util::card_matches_all_heart_colors(&card_db, card_id, hc)
                    } else {
                        super::util::card_matches_heart_colors(&card_db, card_id, hc)
                    }
                }
            })
            .copied()
            .collect();

        if let Some(distinct) = effect.distinct_any() {
            let distinct_filter = super::util::filter_from_parts_full(
                None,
                None,
                None,
                None,
                None,
                None,
                Some(distinct),
                None,
                None,
                None,
            );
            let distinct_indices =
                super::util::filter_distinct(&filtered, &card_db, &distinct_filter, false);
            gs.looked_at_cards = distinct_indices.iter().map(|&i| filtered[i]).collect();
        } else {
            gs.looked_at_cards = filtered.into();
        }

        // Apply CardFilter::from_effect for all remaining filters
        let filter = super::util::CardFilter::from_effect(effect);
        gs.looked_at_cards
            .retain(|id| filter.matches(&card_db, *id, false));

        if effect.exclude_selected_any().unwrap_or(false) && !self.selected_cards.is_empty() {
            gs.looked_at_cards
                .retain(|id| !self.selected_cards.contains(id));
        }
        if effect.exclude_self_any().unwrap_or(false) {
            if let Some(activating_id) = gs.activating_card {
                gs.looked_at_cards.retain(|id| *id != activating_id);
            }
        }

        if count == 0 {
            return Ok(());
        }
        if gs.looked_at_cards.is_empty() {
            return Ok(());
        }
        // Q118: If distinct filter reduced available cards below the required count,
        // bail out so conditional sequential effects can detect the failure.
        // Only applies when distinct constraint is active — regular selects
        // (e.g. "pick up to N" where N > available) should still proceed.
        if effect.distinct_any().is_some() && gs.looked_at_cards.len() < count as usize {
            return Ok(());
        }
        let count = count.min(gs.looked_at_cards.len() as u32);

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
        } else if effect.distinct_any().is_some()
            || (gs.looked_at_cards.len() < card_ids.len()
                && Zone::from_str(source) == Some(Zone::Discard))
        {
            let looked = gs.looked_at_cards.clone();
            Some(
                looked
                    .iter()
                    .map(|&id| card_ids.iter().position(|&cid| cid == id).unwrap_or(0))
                    .collect(),
            )
        } else {
            None
        };
        let desc_en = format!(
            "Select {} card(s) from {}",
            count,
            crate::ability::describe::zone_label(Some(&source))
        );
        let desc_ja = format!(
            "{}から{}枚のカードを選択",
            crate::ability::describe::zone_label_ja(Some(&source)),
            count
        );
        self.pending_choice = Some(
            Choice::select_cards(source.to_string(), count as usize, desc_en, optional)
                .description_ja(Some(desc_ja))
                .card_type(effect.card_type_any().map(|s| s.to_string()))
                .cost_limit(
                    effect.cost_limit_any(),
                    effect.cost_limit_operator_any().map(|s| s.to_string()),
                )
                .group(
                    effect
                        .group_names_any()
                        .as_ref()
                        .and_then(|v| v.first().cloned()),
                )
                .characters(effect.characters_any().cloned())
                .filtered_indices(filtered_indices.clone())
                .target_player_id(Some(target.clone()))
                .is_select_action(true)
                .build(),
        );
        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
        Ok(())
    }
    /// Select cards from `gs.looked_at_cards` matching this effect's filter
    /// (card_type, characters, heart_colors, cost_limit, group_names, etc.).
    /// Sets `pending_choice` so the caller yields and waits for a choice result.
    /// This is the real implementation replacing the earlier stub — extracted
    /// from the select-phase logic inside `execute_look_and_select` so it can
    /// be used as an independent sequential step.
    pub fn execute_select_cards(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        self.current_effect = Some(effect.clone());

        let _placement_order = effect.placement_order_any();
        let any_number = effect.any_number_any().unwrap_or(false);
        let count = effect.count.unwrap_or(1) as usize;
        let optional = effect.optional.unwrap_or(false);
        let src = effect.source_any().unwrap_or("");

        // If source is revealed_cards, handle differently — no look_and_select pipeline
        if src == "revealed_cards" {
            let card_db = &gs.card_database;
            let filter = util::CardFilter::from_effect(effect);
            if filter.has_filter() {
                // Filter revealed_cards in-place
                gs.revealed_cards
                    .retain(|cid| filter.matches(card_db, *cid, false));
            }
            let available = gs.revealed_cards.len();
            let max_select = if optional || any_number {
                available
            } else {
                count.min(available)
            };
            let choice = Choice::select_cards(
                Zone::RevealedCards.to_str(),
                max_select,
                format!("Select card(s) from revealed cards"),
                optional || any_number || available == 0,
            )
            .description_ja(Some("公開されたカードからカードを選択".to_string()))
            .card_type(effect.card_type_any().map(|s| s.to_string()))
            .cost_limit(
                effect.cost_limit_any(),
                effect.cost_limit_operator_any().map(|s| s.to_string()),
            )
            .group(
                effect
                    .group_names_any()
                    .as_ref()
                    .and_then(|v| v.first().cloned()),
            )
            .characters(effect.characters_any().cloned())
            .destination(effect.destination.clone().map(|s| s.to_string()))
            .build();
            self.pending_choice = Some(choice);
            return Ok(());
        }

        // Handle or_card_types (type choice) — same as execute_select.
        let chosen_card_type: Option<String> =
            if let Some(ref or_types) = effect.or_card_types_any() {
                if !or_types.is_empty() {
                    let maybe_cc = gs
                        .ability_queue
                        .current_entry()
                        .and_then(|e| e.conditional_choice.clone());
                    if let Some(ref cc) = maybe_cc {
                        if or_types.contains(cc) {
                            Some(cc.clone())
                        } else {
                            None
                        }
                    } else {
                        let desc_parts: Vec<String> = or_types
                            .iter()
                            .map(|t| {
                                let base = match t.as_str() {
                                    "live_card" => "Live card".to_string(),
                                    "member_card" => "Member card".to_string(),
                                    "energy_card" => "Energy card".to_string(),
                                    _ => t.clone(),
                                };
                                match (
                                    effect.cost_limit_any(),
                                    effect.cost_limit_operator_any().as_deref(),
                                ) {
                                    (Some(l), Some("<=")) => {
                                        format!("{} with cost {} or less", base, l)
                                    }
                                    (Some(l), Some(">=")) => {
                                        format!("{} with cost {} or more", base, l)
                                    }
                                    (Some(l), _) => format!("{} with cost {}", base, l),
                                    _ => base,
                                }
                            })
                            .collect();
                        self.pending_choice = Some(Choice::SelectTarget {
                            target: "choice_string".to_string(),
                            description: format!("Choose: {}", desc_parts.join(", or ")),
                            description_en: Some(format!("Choose: {}", desc_parts.join(", or "))),
                            description_ja: Some(format!("選択: {}", desc_parts.join(", または "))),
                            allow_skip: false,
                            options: Some(desc_parts.clone()),
                        });
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        if let Some(e) = gs.ability_queue.current_entry_mut() {
                            e.conditional_choice = Some(serde_json::to_string(or_types).unwrap());
                        }
                        // Save a copy of this effect so it re-executes after the choice.
                        {
                            let mut remaining = gs.ability_queue.take_pending_actions();
                            remaining.push(effect.clone());
                            gs.ability_queue.set_pending_actions(remaining);
                        }
                        return Ok(());
                    }
                } else {
                    None
                }
            } else {
                None
            };

        // Override card_type with player's choice from or_card_types.
        let override_card_type: Option<String> = chosen_card_type.map(|s| s.to_string());

        let card_db = &gs.card_database;
        let total_count = gs.looked_at_cards.len();

        // Compute matching indices without removing cards from looked_at_cards.
        let oct = override_card_type.as_deref();
        let matching_indices: Vec<usize> = if let Some(opts) = &effect.options_any() {
            if opts.is_empty() {
                let filter = util::CardFilter::from_effect(effect);
                if filter.has_filter() || oct.is_some() {
                    gs.looked_at_cards
                        .iter()
                        .enumerate()
                        .filter(|&(_, &card_id)| {
                            oct.map_or(true, |ct| {
                                util::card_matches_type(card_db, card_id, Some(ct))
                            }) && filter.matches(card_db, card_id, false)
                        })
                        .map(|(i, _)| i)
                        .collect()
                } else {
                    (0..total_count).collect()
                }
            } else {
                gs.looked_at_cards
                    .iter()
                    .enumerate()
                    .filter(|&(_, &card_id)| {
                        opts.iter().any(|opt| {
                            let f = util::CardFilter::from_effect(opt);
                            f.matches(card_db, card_id, false)
                        })
                    })
                    .map(|(i, _)| i)
                    .collect()
            }
        } else {
            let filter = util::CardFilter::from_effect(effect);
            if filter.has_filter() || oct.is_some() {
                gs.looked_at_cards
                    .iter()
                    .enumerate()
                    .filter(|&(_, &card_id)| {
                        oct.map_or(true, |ct| {
                            util::card_matches_type(card_db, card_id, Some(ct))
                        }) && filter.matches(card_db, card_id, false)
                    })
                    .map(|(i, _)| i)
                    .collect()
            } else {
                (0..total_count).collect()
            }
        };

        let matching_count = matching_indices.len();
        if matching_count == 0 {
            let cards = core::mem::take(&mut gs.looked_at_cards);
            let player_target = effect.target_name();
            let player = gs.resolve_target_player_mut(player_target);
            for &card_id in &cards {
                player.waitroom.add_card(card_id);
            }
            return Ok(());
        }
        let is_max = effect.max.unwrap_or(false);
        let max_select = if any_number {
            matching_count
        } else if is_max || optional {
            core::cmp::min(count, matching_count)
        } else {
            core::cmp::min(count, matching_count)
        };

        let description = if any_number {
            format!(
                "Select any number of cards from the {} looked-at cards (or skip)",
                total_count
            )
        } else if is_max || optional {
            format!(
                "Select up to {} card(s) from the {} looked-at cards (or skip)",
                max_select, total_count
            )
        } else {
            format!(
                "Select {} card(s) from the {} looked-at cards",
                max_select, total_count,
            )
        };

        let desc_ja = if any_number {
            format!(
                "確認した{}枚のカードから好きな枚数を選択（スキップ可）",
                total_count
            )
        } else if is_max || optional {
            format!(
                "確認した{}枚のカードから最大{}枚を選択（スキップ可）",
                total_count, max_select
            )
        } else {
            format!(
                "確認した{}枚のカードから{}枚を選択",
                total_count, max_select
            )
        };
        let choice = Choice::select_cards(
            Zone::LookedAt.to_str(),
            max_select,
            description.clone(),
            optional || is_max || any_number,
        )
        .description_ja(Some(desc_ja))
        .card_type(
            override_card_type
                .clone()
                .or_else(|| effect.card_type_any().map(|s| s.to_string())),
        )
        .cost_limit(
            effect.cost_limit_any(),
            effect.cost_limit_operator_any().map(|s| s.to_string()),
        )
        .group(
            effect
                .group_names_any()
                .as_ref()
                .and_then(|v| v.first().cloned()),
        )
        .characters(effect.characters_any().cloned())
        .filtered_indices(Some(matching_indices))
        .build();
        self.pending_choice = Some(choice);
        self.execution_context = ExecutionContext::LookAndSelect {
            step: LookAndSelectStep::Select {
                count: max_select,
                max_per_group: effect.per_group_count_any(),
            },
        };
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: カード選択 {}枚", pp, act_name, count));
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
                description_en: Some(format!("Look at {} card(s) (optional cost)?", count)),
                description_ja: Some(format!("{}枚確認（オプションコスト）？", count)),
                allow_skip: true,
                options: None,
            });
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.choice_card_no = Some(ChoiceRoute::OptionalCost);
            }
            return Ok(());
        }
        // Rule 10.2.2.2 / Q85: If deck has fewer cards than needed, take
        // what's available (Q85 multi-step: draw → refresh → draw remaining).
        let look_from_deck = Zone::from_str(source) == Some(Zone::Deck)
            || Zone::from_str(source) == Some(Zone::DeckTop);
        if look_from_deck {
            let deck_count = gs.resolve_target_player(target).main_deck.cards.len();
            if (deck_count as u32) < count {
                return self.look_at_with_refresh(gs, effect, count, target, source);
            }
        }
        let player = gs.resolve_target_player_mut(target);

        let fetch_all = effect.all_any().unwrap_or(false);
        let take_cards = |cards: &[i16]| -> Vec<i16> {
            if fetch_all {
                cards.to_vec()
            } else {
                cards.iter().take(count as usize).copied().collect()
            }
        };
        let cards: Vec<i16> = match Zone::from_str(source) {
            Some(Zone::Deck) | Some(Zone::DeckTop) => {
                player.main_deck.draw_multiple(count as usize)
            }
            Some(Zone::Stage) => {
                let sc: Vec<i16> = player
                    .stage
                    .stage
                    .iter()
                    .filter(|&&id| id != -1)
                    .copied()
                    .collect();
                take_cards(&sc)
            }
            _ => take_cards(util::zone_cards(player, source)),
        };

        gs.looked_at_cards = cards.into();
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: 確認 {}枚", pp, act_name, count));
        Ok(())
    }

    // Q85 / Rule 10.2.2.2: Look at N from deck with < N cards available.
    fn look_at_with_refresh(
        &mut self,
        gs: &mut GameState,
        _effect: &AbilityEffect,
        count: u32,
        target: &str,
        _source: &str,
    ) -> Result<(), String> {
        let count = count as usize;
        let first = {
            let player = gs.resolve_target_player_mut(target);
            let take = player.main_deck.cards.len();
            if take > 0 {
                player.main_deck.draw_multiple(take)
            } else {
                Vec::new()
            }
        };
        let mut looked = first;
        if looked.len() < count {
            {
                let player = gs.resolve_target_player_mut(target);
                if player.main_deck.cards.is_empty() && !player.waitroom.cards.is_empty() {
                    player.refresh();
                }
            }
            let remaining = count - looked.len();
            let player = gs.resolve_target_player_mut(target);
            if !player.main_deck.cards.is_empty() {
                let more = core::cmp::min(remaining, player.main_deck.cards.len());
                looked.extend(player.main_deck.draw_multiple(more));
            }
        }
        gs.looked_at_cards = looked.into();
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
                Some(Zone::LookedAt) => gs.looked_at_cards.to_vec(),
                _ => vec![],
            }
        };

        let mut by_group: HashMap<String, Vec<i16>> = HashMap::default();
        for &card_id in &card_ids {
            let group_name = card_db
                .get_card(card_id)
                .map(|c| c.group.to_string())
                .unwrap_or_default();
            by_group.entry(group_name).or_default().push(card_id);
        }

        let pg_source = gs.current_ability_source_card_id();
        let pg_owner = util::target_player_index(target, gs.ability_master_id().as_deref());
        for members in by_group.values() {
            for &card_id in members {
                gs.push_revealed_card(card_id, pg_source, false, pg_owner, "ability");
            }
        }

        if !card_ids.is_empty() {
            let turn = gs.turn_number;
            let master = gs.ability_master_id();
            let player_label = super::util::target_player_label(target, master.as_deref());
            gs.push_rule_log(format!(
                "[Turn {}] {} [[log_reveal_group:n={},source=zone_{}]]",
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
        let ru_source = gs.current_ability_source_card_id();
        let ru_owner = util::target_player_index(target, gs.ability_master_id().as_deref());

        loop {
            let card_id = {
                let player = gs.resolve_target_player_mut(target);
                player.main_deck.draw()
            };
            match card_id {
                Some(cid) => {
                    all_revealed.push(cid);
                    gs.push_revealed_card(cid, ru_source, false, ru_owner, "ability");
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
                        gs.push_revealed_card(cid, ru_source, false, ru_owner, "ability");
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
            let found = matched_idx.is_some();
            let names: Vec<String> = all_revealed
                .iter()
                .filter_map(|id| card_db.get_card(*id))
                .map(|c| c.name.to_string())
                .collect();
            gs.push_rule_log(format!(
                "[Turn {}] {} [[log_reveal_deck_until:found={}]]: {}",
                turn,
                player_label,
                found,
                names.join(", ")
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
        gs.looked_at_cards = all_revealed.into();
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
            gs.looked_at_cards = core::iter::once(matched).chain(all_revealed).collect();
        } else {
            gs.looked_at_cards.clear();
        }
        Ok(())
    }
}
