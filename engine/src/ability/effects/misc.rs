use super::super::resolver::AbilityResolver;
use super::super::types::{Choice, ExecutionContext};
use super::super::util;
use crate::card::AbilityEffect;
use crate::card::PositionInfo;
use crate::game_state::GameState;

impl AbilityResolver {
    pub(crate) fn execute_reveal_effect(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        if effect.multiple_targets.unwrap_or(false) && effect.source.as_deref() == Some("deck_top")
        {
            let chosen = gs
                .ability_queue
                .current_entry()
                .and_then(|e| e.conditional_choice.clone())
                .or_else(|| effect.card_type.clone());
            return self.execute_reveal_until_target(gs, effect.target_name(), chosen.as_deref());
        }
        self.execute_reveal(
            gs,
            effect.source_or("hand"),
            effect.count_or(1),
            effect.target_name(),
            effect.card_type.as_deref(),
            &effect.heart_colors,
            effect.blind.unwrap_or(false),
        )
    }

    pub(crate) fn execute_custom(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        action_str: &str,
    ) -> Result<(), String> {
        // Handle "custom" actions that could not be parsed into a standard action type.
        // Some custom actions have enough info to re-route to a known handler.

        // 1) Deck reordering: placement_order=any_order → route as move_cards looked_at→deck_top
        if effect.placement_order.as_deref() == Some("any_order") {
            let mut routed = effect.clone();
            routed.action = "move_cards".into();
            if routed.source.is_none() {
                routed.source = Some("looked_at".into());
            }
            if routed.destination.is_none() {
                routed.destination = Some("deck_top".into());
            }
            return self.execute_move_cards(gs, &routed);
        }

        // 2) Complex conditional scoring / gain_ability: has duration
        if effect.duration.is_some() {
            let text = if effect.text.is_empty() {
                action_str
            } else {
                &effect.text
            };
            return self.execute_gain_ability(
                gs,
                text,
                effect.target.as_deref().unwrap_or("self"),
                effect.duration.as_deref(),
            );
        }

        eprintln!("Unhandled custom action: {}", action_str);
        Ok(())
    }

    pub(crate) fn execute_reveal_until_chosen_card(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        // Get the chosen card type from the effect or from the current ability queue entry
        let chosen_card_type = gs
            .ability_queue
            .current_entry()
            .and_then(|e| e.conditional_choice.clone())
            .or_else(|| effect.card_type.clone());

        if let Some(card_type) = chosen_card_type {
            // Use the existing reveal_until_target functionality
            self.execute_reveal_until_target(gs, effect.target_name(), Some(&card_type))?;

            // After reveal, we need to move the chosen card to hand and others to discard
            // The looked_at_cards should contain: [chosen_card, other_revealed_cards...]
            if !self.looked_at_cards.is_empty() {
                let chosen_card = self.looked_at_cards[0];
                let other_cards = self.looked_at_cards[1..].to_vec();

                // Move chosen card to hand
                let player = gs.resolve_target_player_mut(effect.target_name());
                player.hand.cards.push(chosen_card);

                // Move other cards to discard
                player.waitroom.cards.extend(other_cards);

                // Clear looked_at_cards
                self.looked_at_cards.clear();
            }
        } else if let Some(ref or_types) = effect.or_card_types {
            // No card type chosen yet — create the type choice prompt.
            let desc = format!("Choose: {}", or_types.join(", or "));
            self.pending_choice = Some(Choice::SelectTarget {
                target: "choice_string".to_string(),
                description: desc,
                allow_skip: false,
                options: Some(
                    or_types
                        .iter()
                        .map(|t| {
                            let label = match t.as_str() {
                                "live_card" => {
                                    if effect.cost_limit.is_some() {
                                        format!(
                                            "Live card (cost {}{})",
                                            effect.cost_limit_operator.as_deref().unwrap_or(">="),
                                            effect.cost_limit.unwrap()
                                        )
                                    } else {
                                        "Live card".to_string()
                                    }
                                }
                                "member_card" => {
                                    format!(
                                        "Member card (cost {} {})",
                                        effect.cost_limit_operator.as_deref().unwrap_or(">="),
                                        effect.cost_limit.unwrap_or(0)
                                    )
                                }
                                _ => t.clone(),
                            };
                            label
                        })
                        .collect(),
                ),
            });
            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            // Store the or_card_types so the choice handler can look them up
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.conditional_choice =
                    Some(serde_json::to_string(or_types).unwrap_or_default());
            }
        } else {
            // If no card type was chosen and no available types, just clear any looked_at_cards
            self.looked_at_cards.clear();
        }

        Ok(())
    }

    /// Handles target="both" by executing the effect for self, then opponent.
    /// Returns true if the effect was fully handled (has "both" target), false otherwise.
    pub(crate) fn handle_both_targets(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<bool, String> {
        // Skip if not "both" or if this is position_change (handles "both" internally)
        if effect.target.as_deref() != Some("both") || effect.action.as_str() == "position_change" {
            return Ok(false);
        }

        // Execute for self first
        let mut for_self = effect.clone();
        for_self.target = Some("self".to_string());
        self.last_effect_target = Some("self".to_string());

        let had_choice_before = self.pending_choice.is_some();
        let _ = self.execute_effect(gs, &for_self);

        // If self created a NEW pending choice, save opponent for later
        if self.pending_choice.is_some() && !had_choice_before {
            let mut for_opponent = effect.clone();
            for_opponent.target = Some("opponent".to_string());
            gs.ability_queue
                .set_pending_commands(vec![crate::ability::types::Command::Effect(for_opponent)]);
            return Ok(true);
        }

        // Execute for opponent
        let mut for_opponent = effect.clone();
        for_opponent.target = Some("opponent".to_string());
        self.execute_effect(gs, &for_opponent)?;

        Ok(true)
    }

    pub fn execute_gain_resource(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        if effect.resource.as_deref() == Some("heart")
            && effect.heart_type.as_deref() == Some("all")
        {
            if let Some(card_id) = gs.activating_card {
                gs.mods.add_heart_modifier(
                    card_id,
                    crate::card::HeartColor::Heart00,
                    effect.count_or(1) as i32,
                );
            }
            return Ok(());
        }

        // bp6 pattern: "gain 1 heart per distinct color among discarded cards"
        // Detected by: resource=heart, per_unit=true, per_unit_type="discard", multiple_targets=true
        // For each distinct heart color present among recently_moved_cards, grant 1 heart of that color.
        if effect.resource.as_deref() == Some("heart")
            && effect.per_unit.unwrap_or(false)
            && effect.per_unit_type.as_deref() == Some("discard")
            && effect.multiple_targets.unwrap_or(false)
        {
            let card_db = self.card_db();
            let duration = effect.duration.clone();
            let is_temporary = duration.is_some() && duration.as_deref() != Some("permanent");
            let target = effect.target_name().to_string();
            let activating_card_id = gs.activating_card;

            // Collect distinct heart colors from all recently discarded cards.
            // Member cards carry their heart colors in base_heart, not need_heart.
            let recently_moved = gs.recently_moved_cards.clone();
            let mut distinct_colors: Vec<crate::card::HeartColor> = Vec::new();
            if let Some(ref moved) = recently_moved {
                for &cid in moved {
                    if let Some(card) = card_db.get_card(cid) {
                        // Use base_heart for member cards (need_heart is only on live cards)
                        if let Some(ref bh) = card.base_heart {
                            for (&color, &amt) in &bh.hearts {
                                if amt > 0 && !distinct_colors.contains(&color) {
                                    distinct_colors.push(color);
                                }
                            }
                        }
                    }
                }
            }

            eprintln!(
                "[BP6_HEART] distinct colors from {} discarded cards: {:?}",
                recently_moved.as_ref().map(|v| v.len()).unwrap_or(0),
                distinct_colors
            );

            if let Some(card_id) = activating_card_id {
                for color in &distinct_colors {
                    gs.mods.add_heart_modifier(card_id, *color, 1);
                }
                if is_temporary && !distinct_colors.is_empty() {
                    let color_names: Vec<String> = distinct_colors
                        .iter()
                        .map(|c| format!("{:?}", c).to_lowercase())
                        .collect();
                    util::push_temporary_effect(
                        gs,
                        "gain_heart",
                        duration.as_deref(),
                        &target,
                        &format!("Gain 1 heart of each color: {}", color_names.join(", ")),
                        None,
                    );
                }
            }
            return Ok(());
        }
        let resource = effect.resource.as_deref().unwrap_or("").to_string();
        let count = effect.resource_icon_count.unwrap_or(effect.count_or(1));
        let target = effect.target_name().to_string();
        let duration = effect.duration.clone();
        let is_temporary = duration.is_some() && duration.as_deref() != Some("permanent");
        let card_type_filter = effect.card_type.clone();
        let group_filter = effect.group_name().map(|s| s.to_string());
        let per_unit_count_val = effect.per_unit_count.unwrap_or(1);
        let per_unit_type_str = effect.per_unit_type.clone();
        let heart_selection = effect.heart_selection.unwrap_or(false);
        let per_unit = effect.per_unit.unwrap_or(false);
        let sign = effect.sign.as_deref();
        let activating_card_id = gs.activating_card;
        let card_db = self.card_db();
        let is_self_target = effect.self_target.unwrap_or(false);
        let last_discard_count = gs.mods.last_cost_discard_count;
        let is_all = effect.all.unwrap_or(false)
            || (effect.source.is_none()
                && effect.card_type.as_deref() == Some("member_card")
                && target == "self"
                && !is_self_target);

        // For per_unit gain_resource, skip heart_colors selection and use
        // the previously selected heart from conditional_choice (set by a
        // preceding select action). Multiple heart_colors would create a
        // spurious choice in the middle of a per-unit counting operation.
        let single_fixed_heart = if per_unit && resource == "heart" {
            gs.ability_queue
                .current_entry()
                .and_then(|e| e.conditional_choice.clone())
        } else {
            let result = self.resolve_gain_heart_color(
                gs,
                effect,
                resource.as_str(),
                count,
                &effect.heart_colors,
                heart_selection,
            )?;
            if self.pending_choice.is_some() {
                return Ok(());
            }
            result.or_else(|| {
                if resource == "heart" || resource == "ハート" {
                    gs.ability_queue
                        .current_entry()
                        .and_then(|e| e.conditional_choice.clone())
                } else {
                    None
                }
            })
        };

        // Extract accumulated selected card IDs from resolver
        let all_selected: Vec<i16> = self.selected_cards.clone();

        let recently_moved = gs.recently_moved_cards.clone();

        let exclude_self_id = effect.exclude_self.and_then(|_| gs.activating_card);

        if effect.target_count.is_some()
            && !is_all
            && !is_self_target
            && all_selected.is_empty()
            && !per_unit
            && (resource == "blade"
                || resource == "ブレード"
                || resource == "heart"
                || resource == "ハート")
        {
            let stage_ids: Vec<i16> = {
                let p = gs.resolve_target_player(&target);
                p.stage
                    .stage
                    .iter()
                    .copied()
                    .filter(|&id| id != -1)
                    .collect()
            };
            let prelim_filter = util::filter_from_parts(
                effect.card_type.as_deref(),
                effect.group_name(),
                None,
                None,
                effect.characters.as_ref(),
                None,
                exclude_self_id,
            );
            let candidates = util::matching_ids_filtered(
                &stage_ids,
                &card_db,
                &prelim_filter,
                true,
                None,
                if resource == "blade"
                    || resource == "ブレード"
                    || resource == "heart"
                    || resource == "ハート"
                {
                    effect.distinct.as_deref()
                } else {
                    None
                },
                None,
            );
            let tc = effect.target_count.unwrap_or(1) as usize;
            if candidates.len() > tc {
                let stage_snapshot: Vec<i16> = {
                    let p = gs.resolve_target_player(&target);
                    p.stage.stage.to_vec()
                };
                let filtered_indices: Vec<usize> = candidates
                    .iter()
                    .filter_map(|&cid| stage_snapshot.iter().position(|&s| s == cid))
                    .collect();
                let mut saved = effect.clone();
                saved.target_count = None;
                let mut pending = gs.ability_queue.take_pending_commands();
                pending.insert(0, crate::ability::types::Command::Effect(saved));
                gs.ability_queue.set_pending_commands(pending);
                self.pending_choice = Some(
                    crate::ability::types::Choice::select_cards(
                        "stage".to_string(),
                        tc,
                        format!("Select {} card(s) to receive {} {}", tc, count, resource),
                        false,
                    )
                    .card_type(effect.card_type.clone())
                    .group(effect.group_name().map(|s| s.to_string()))
                    .characters(effect.characters.clone())
                    .filtered_indices(Some(filtered_indices))
                    .target_player_id(Some(target.clone()))
                    .is_select_action(true)
                    .build(),
                );
                self.store_pending_choice(gs);
                return Ok(());
            }
        }

        let (blade_targets, heart_targets, heart_color_str, final_count) = {
            let player = gs.resolve_target_player_mut(&target);

            let filter = util::filter_from_parts(
                effect.card_type.as_deref(),
                effect.group_name(),
                None,
                None,
                effect.characters.as_ref(),
                None,
                exclude_self_id,
            );

            let final_count = if per_unit {
                // If the effect has an explicit location, use that as the count zone
                // instead of the generic per_unit_type → zone mapping.
                let effective_per_unit_type =
                    effect.location.as_deref().or(per_unit_type_str.as_deref());
                let mut matching_count = util::resolve_per_unit_count(
                    true,
                    effective_per_unit_type,
                    player,
                    &card_db,
                    &filter,
                    &[],
                );
                // For per_unit_type="discard": always use the cost-tracked discard count.
                // This gives exact discard count for abilities like LL-bp2-001
                // (cost: discard named characters → gain 1 blade per discarded card).
                // When the optional cost is skipped, last_cost_discard_count=0 and
                // recently_moved=None, so we correctly get 0 (no gain).
                // DO NOT fall back to counting waitroom — that would incorrectly give
                // 1 blade even when the cost was skipped.
                if per_unit_type_str.as_deref() == Some("discard") {
                    if let Some(ref moved) = recently_moved {
                        let filtered = moved
                            .iter()
                            .filter(|&&cid| filter.matches(&card_db, cid, false))
                            .count() as u32;
                        matching_count = filtered;
                    } else {
                        // No recent move tracked → cost was skipped or 0 discarded.
                        matching_count = last_discard_count;
                    }
                } else if (per_unit_type_str.as_deref() == Some("waitroom")
                    || per_unit_type_str.as_deref() == Some("waitroom_card"))
                    && (last_discard_count > 0 || recently_moved.is_some())
                {
                    if let Some(ref moved) = recently_moved {
                        let filtered = moved
                            .iter()
                            .filter(|&&cid| filter.matches(&card_db, cid, false))
                            .count() as u32;
                        matching_count = filtered;
                    } else {
                        matching_count = last_discard_count;
                    }
                }
                let mut units = matching_count / per_unit_count_val;
                if effect.max.unwrap_or(false) {
                    if let Some(cap) = effect.count {
                        units = units.min(cap);
                    }
                }
                units * effect.resource_icon_count.unwrap_or(1)
            } else {
                count
            };

            // Only exclude previously-selected cards when this action has target limits.
            // Cards like "give blade to both players" should not exclude anything.
            let has_selection_filter = effect.target_count.is_some() || effect.distinct.is_some();
            let exclude = if has_selection_filter && !all_selected.is_empty() {
                Some(all_selected.as_slice())
            } else {
                None
            };
            let tc = effect.target_count;
            let dn = effect.distinct.as_deref();

            let has_blade_filter = card_type_filter.is_some() || group_filter.is_some();
            // When has_selection_filter is set (target_count/distinct), don't blindly
            // use all_selected — apply the filter with exclusion to find the right targets.
            // Only use all_selected directly for pure sequential select→gain_resource.
            let use_raw = !all_selected.is_empty() && !has_selection_filter;
            let blade_targets: Vec<i16> = if use_raw {
                all_selected.clone()
            } else if has_blade_filter || is_all {
                util::matching_ids_filtered(
                    util::zone_cards(player, "stage"),
                    &card_db,
                    &filter,
                    true,
                    tc,
                    if resource == "blade" || resource == "ブレード" {
                        dn
                    } else {
                        None
                    },
                    exclude,
                )
            } else {
                vec![]
            };

            let heart_color_inner = single_fixed_heart
                .clone()
                .or_else(|| effect.heart_colors.first().map(|s| s.to_string()));
            let heart_targets: Vec<i16> = if use_raw {
                all_selected.clone()
            } else if resource == "heart" || resource == "ハート" {
                util::matching_ids_filtered(
                    util::zone_cards(player, "stage"),
                    &card_db,
                    &filter,
                    true,
                    tc,
                    if resource == "heart" || resource == "ハート" {
                        dn
                    } else {
                        None
                    },
                    exclude,
                )
            } else {
                vec![]
            };

            (blade_targets, heart_targets, heart_color_inner, final_count)
        };

        // Store selected card IDs when target_count/distinct is set
        // so the next sequential action can exclude these cards.
        // Only store when we have explicit selection limits to avoid
        // polluting all_selected for blanket effects like "both players gain blade".
        if effect.target_count.is_some() || effect.distinct.is_some() {
            let selected_targets: Vec<i16> = if resource == "blade" || resource == "ブレード" {
                blade_targets.clone()
            } else {
                heart_targets.clone()
            };
            if !selected_targets.is_empty() {
                self.selected_cards.extend(&selected_targets);
            }
        }

        let mut effect_data: Option<serde_json::Value> = None;
        let is_negative = sign == Some("negative");
        let blades_to_add = if is_negative {
            -(final_count as i32)
        } else {
            final_count as i32
        };
        let heart_to_add = if is_negative {
            -(final_count as i32)
        } else {
            final_count as i32
        };
        let heart_color_val =
            crate::zones::parse_heart_color(heart_color_str.as_deref().unwrap_or("heart00"));

        if is_self_target {
            if let Some(card_id) = activating_card_id {
                if !gs
                    .resolve_target_player_mut(&target)
                    .stage
                    .stage
                    .contains(&card_id)
                {
                    return Err(
                        "Cannot use self_target on gain_resource: activating card not on stage"
                            .to_string(),
                    );
                }
                if resource == "blade" || resource == "ブレード" {
                    gs.mods.add_blade_modifier(card_id, blades_to_add);
                    if is_temporary {
                        effect_data =
                            Some(Self::make_card_effect_data(card_id, blades_to_add, None));
                    }
                }
                if resource == "heart" || resource == "ハート" {
                    gs.mods
                        .add_heart_modifier(card_id, heart_color_val, heart_to_add);
                    if is_temporary && effect_data.is_none() {
                        let color_name = heart_color_str.as_deref().unwrap_or("heart01");
                        effect_data = Some(Self::make_card_effect_data(
                            card_id,
                            heart_to_add,
                            Some(color_name),
                        ));
                    }
                }
                if is_temporary {
                    util::push_temporary_effect(
                        gs,
                        &format!("gain_{}", resource),
                        duration.as_deref(),
                        &target,
                        &format!("Gain {} {}", final_count, resource),
                        effect_data,
                    );
                }
                return Ok(());
            }
        }

        let blade_targets_save = blade_targets.clone();
        if resource == "blade" || resource == "ブレード" {
            if blade_targets.is_empty() {
                if is_all {
                    let stage_ids: Vec<i16> = {
                        let player = gs.resolve_target_player(&target);
                        player
                            .stage
                            .stage
                            .iter()
                            .copied()
                            .filter(|&id| id != -1)
                            .collect()
                    };
                    for card_id in stage_ids {
                        gs.mods.add_blade_modifier(card_id, blades_to_add);
                    }
                    if is_temporary {
                        let mut data = serde_json::Map::new();
                        data.insert("all_cards".to_string(), serde_json::Value::Bool(true));
                        data.insert(
                            "amount".to_string(),
                            serde_json::Value::Number(blades_to_add.into()),
                        );
                        effect_data = Some(serde_json::Value::Object(data));
                    }
                } else if let Some(card_id) = activating_card_id {
                    gs.mods.add_blade_modifier(card_id, blades_to_add);
                    if is_temporary {
                        effect_data =
                            Some(Self::make_card_effect_data(card_id, blades_to_add, None));
                    }
                }
            } else if !all_selected.is_empty() && effect.source.is_none() {
                // Pure sequential select→gain_resource: apply to ALL selected cards with full count
                for &card_id in &blade_targets {
                    gs.mods.add_blade_modifier(card_id, blades_to_add);
                }
            } else {
                let targets = if is_all {
                    blade_targets.clone()
                } else {
                    blade_targets
                        .into_iter()
                        .take(final_count as usize)
                        .collect()
                };
                for &card_id in &targets {
                    gs.mods.add_blade_modifier(card_id, blades_to_add);
                }
            }
        }

        if resource == "heart" || resource == "ハート" {
            if heart_targets.is_empty() {
                if let Some(card_id) = activating_card_id {
                    gs.mods
                        .add_heart_modifier(card_id, heart_color_val, heart_to_add);
                    if is_temporary && effect_data.is_none() {
                        let color_name = heart_color_str.as_deref().unwrap_or("heart01");
                        effect_data = Some(Self::make_card_effect_data(
                            card_id,
                            heart_to_add,
                            Some(color_name),
                        ));
                    }
                }
            } else if is_self_target
                || (target == "self"
                    && activating_card_id.is_some()
                    && effect.source.is_none()
                    && effect.card_type.is_none())
            {
                if let Some(card_id) = activating_card_id {
                    gs.mods
                        .add_heart_modifier(card_id, heart_color_val, heart_to_add);
                    if is_temporary && effect_data.is_none() {
                        let color_name = heart_color_str.as_deref().unwrap_or("heart01");
                        effect_data = Some(Self::make_card_effect_data(
                            card_id,
                            heart_to_add,
                            Some(color_name),
                        ));
                    }
                }
            } else {
                let targets: Vec<i16> = if is_all {
                    heart_targets.clone()
                } else {
                    heart_targets
                        .into_iter()
                        .take(final_count as usize)
                        .collect()
                };
                for &card_id in &targets {
                    gs.mods
                        .add_heart_modifier(card_id, heart_color_val, heart_to_add);
                }
            }
        }

        // Store effect_data for blade cleanup.
        if is_temporary && effect_data.is_none() {
            if resource == "blade" || resource == "ブレード" {
                let cards_json: Vec<serde_json::Value> = blade_targets_save
                    .iter()
                    .map(|&cid| serde_json::json!({"card_id": cid, "amount": final_count}))
                    .collect();
                effect_data = Some(serde_json::Value::Array(cards_json.clone()));
            }
        }

        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        let resource_jp = if resource == "blade" || resource == "ブレード" {
            "ブレード"
        } else {
            "ハート"
        };
        let color_tag = heart_color_str.as_deref().unwrap_or("heart00");
        if resource == "heart" || resource == "ハート" {
            gs.rule_log.push(format!(
                "{} {}: {} {}獲得 ({})",
                pp, act_name, final_count, resource_jp, color_tag
            ));
        } else {
            gs.rule_log.push(format!(
                "{} {}: {} {}獲得",
                pp, act_name, final_count, resource_jp
            ));
        }

        if is_temporary {
            util::push_temporary_effect(
                gs,
                &format!("gain_{}", resource),
                duration.as_deref(),
                &target,
                &format!("Gain {} {}", final_count, resource),
                effect_data,
            );
        }
        Ok(())
    }

    pub(crate) fn execute_play_baton_touch(
        &mut self,
        gs: &mut GameState,
        count: u32,
        target: &str,
    ) -> Result<(), String> {
        eprintln!("play_baton_touch: count={}, target={}", count, target);
        if gs.baton_touch_count == 0 {
            return Err(
                "Baton touch condition not met: card was not played via baton touch".to_string(),
            );
        }
        gs.prohibition_effects
            .push(format!("baton_touch_allowed:{}", count));
        Ok(())
    }

    pub fn execute_place_energy_under_member(
        &mut self,
        gs: &mut GameState,
        count: u32,
        target: &str,
        position: Option<&PositionInfo>,
        optional: bool,
        source: Option<&str>,
    ) {
        // Check if we're moving from under_member to energy_deck (Awakening case)
        println!(
            "DEBUG: execute_place_energy_under_member - source: {:?}",
            source
        );
        let activating_pos = gs.activating_card.and_then(|c| {
            gs.resolve_target_player(target)
                .stage
                .stage
                .iter()
                .position(|&id| id == c)
        });
        if source == Some("under_member") {
            // Handle moving from under_member to energy_deck
            let player = gs.resolve_target_player_mut(target);
            let target_index = match position.and_then(|p| p.get_position()) {
                Some("center") | Some("中央") => 1,
                Some("left") | Some("左側") => 0,
                Some("right") | Some("右側") => 2,
                None => {
                    // Default to activating card's position
                    if let Some(idx) = activating_pos {
                        if player.stage.stage[idx] != -1 {
                            idx
                        } else {
                            return;
                        }
                    } else if player.stage.stage[1] != -1 {
                        1
                    } else if player.stage.stage[0] != -1 {
                        0
                    } else if player.stage.stage[2] != -1 {
                        2
                    } else {
                        return;
                    }
                }
                _ => 1,
            };

            if player.stage.stage[target_index] == -1 {
                return;
            }

            let area = match target_index {
                0 => crate::zones::MemberArea::LeftSide,
                1 => crate::zones::MemberArea::Center,
                _ => crate::zones::MemberArea::RightSide,
            };

            let under_cards = player.stage.get_under_cards(area);
            if under_cards.is_empty() {
                return;
            }

            // Create choice to select cards to move
            self.pending_choice = Some(
                Choice::select_cards(
                    "under_member",
                    under_cards.len().min(count as usize),
                    "Select energy cards to move from under member to energy deck",
                    optional,
                )
                .card_type(Some("energy_card".to_string()))
                .target_player_id(Some(target.to_string()))
                .build(),
            );
            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            return;
        }

        // Original logic: move from energy_zone to under_member
        // This should only execute if source is NOT "under_member"
        if optional {
            let is_activation = self
                .current_ability
                .as_ref()
                .and_then(|a| a.triggers.as_ref())
                .map_or(false, |t| t == crate::triggers::ACTIVATION);
            if !is_activation {
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "pay_optional_cost:skip_optional_cost".to_string(),
                    description: "Place energy under member? (pay or skip)".to_string(),
                    allow_skip: false,
                    options: None,
                });
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.choice_card_no = Some("optional_cost".to_string());
                }
                return;
            }
        }
        let player = gs.resolve_target_player_mut(target);
        let mut energy_cards = Vec::new();
        for _ in 0..count {
            if let Some(energy_card) = player.energy_zone.cards.pop() {
                energy_cards.push(energy_card);
            } else {
                break;
            }
        }
        if energy_cards.is_empty() {
            return;
        }
        player.energy_zone.active_energy_count = player
            .energy_zone
            .active_energy_count
            .saturating_sub(energy_cards.len());
        let target_index = match position.and_then(|p| p.get_position()) {
            Some("center") | Some("中央") => 1,
            Some("left") | Some("左側") => 0,
            Some("right") | Some("右側") => 2,
            None => {
                // Default to activating card's position
                if let Some(idx) = activating_pos {
                    if player.stage.stage[idx] != -1 {
                        idx
                    } else {
                        for card in energy_cards {
                            player.energy_deck.cards.push(card);
                        }
                        return;
                    }
                } else if player.stage.stage[1] != -1 {
                    1
                } else if player.stage.stage[0] != -1 {
                    0
                } else if player.stage.stage[2] != -1 {
                    2
                } else {
                    for card in energy_cards {
                        player.energy_deck.cards.push(card);
                    }
                    return;
                }
            }
            _ => 1,
        };
        if player.stage.stage[target_index] == -1 {
            // Rule 10.5.4: Energy without a member goes to energy deck
            for card in energy_cards {
                player.energy_deck.cards.push(card);
            }
            return;
        }
        // Rule 10.5.3: Energy placed under member — track it for recycling
        let area = match target_index {
            0 => crate::zones::MemberArea::LeftSide,
            1 => crate::zones::MemberArea::Center,
            _ => crate::zones::MemberArea::RightSide,
        };
        for card in energy_cards {
            player.stage.place_under_card(area, card);
        }
    }

    pub fn execute_position_change(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        position: Option<PositionInfo>,
        target: &str,
        target_member: &str,
    ) -> Result<(), String> {
        // Check source_position from effect (new parser field), fall back to position param
        let source_pos = effect
            .source_position
            .as_deref()
            .or_else(|| position.as_ref().and_then(|p| p.get_position()));
        let position_str = source_pos.unwrap_or("");

        // If destination is already specified (from conditional position_change or area_select),
        // route directly to execute_position_change_with_destination.
        // EXCEPTION: "front" destination for opponent needs source selection first.
        if let Some(ref dest) = effect.destination {
            if dest == "front" && target == "opponent" {
                // "front" destination for opponent: the destination is fixed (front area of
                // activating card). Create a choice to select which OPPONENT member to move.
                let valid_sources: Vec<String> = {
                    let player = gs.resolve_target_player(target);
                    (0..3)
                        .filter(|&i| player.stage.stage[i] != -1)
                        .map(|i| {
                            match i {
                                0 => "left",
                                1 => "center",
                                _ => "right",
                            }
                            .to_string()
                        })
                        .collect()
                };
                if valid_sources.is_empty() {
                    return Ok(());
                }
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.choice_card_no = Some("position_change:opponent:front".to_string());
                }
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "position|destination".to_string(),
                    description: "Choose which opponent member to move".to_string(),
                    allow_skip: effect.optional.unwrap_or(false),
                    options: Some(valid_sources),
                });
                return Ok(());
            } else {
                return self.execute_position_change_with_destination(gs, effect, dest);
            }
        }

        // Handle "both" target: opponent first (choice), then self (choice via pending).
        if target == "both" {
            let mut opp_effect = effect.clone();
            opp_effect.target = Some("opponent".to_string());
            self.execute_position_change(
                gs,
                &opp_effect,
                position.clone(),
                "opponent",
                target_member,
            )?;
            if self.pending_choice.is_some() {
                let mut self_effect = effect.clone();
                self_effect.target = Some("self".to_string());
                gs.ability_queue.set_pending_commands(vec![
                    crate::ability::types::Command::Effect(self_effect),
                ]);
            } else {
                let mut self_effect = effect.clone();
                self_effect.target = Some("self".to_string());
                self.execute_position_change(
                    gs,
                    &self_effect,
                    position.clone(),
                    "self",
                    target_member,
                )?;
            }
            return Ok(());
        }

        if target_member == "this_member" {
            // Handle "multiple_targets=true" (それぞれ) pattern: iterate all stage members.
            if effect.multiple_targets.unwrap_or(false) {
                // Check for predetermined rotation: multiple_targets + position specified.
                // e.g. 003-R: center→left, left→right, right→center
                if effect.position.is_some() || effect.source_position.is_some() {
                    return self.execute_rotation(gs, effect, target);
                }

                let target_m = effect.target.as_deref().unwrap_or("self");
                let card_db = self.card_db();
                let mut card_ids: Vec<i16> = Vec::new();
                {
                    let player = gs.resolve_target_player_mut(target_m);
                    for i in 0..3 {
                        if player.stage.stage[i] != -1 {
                            card_ids.push(player.stage.stage[i]);
                        }
                    }
                }
                if card_ids.is_empty() {
                    return Ok(());
                }
                let card_nos: Vec<String> = card_ids
                    .iter()
                    .filter_map(|&cid| card_db.get_card(cid).map(|c| c.card_no.clone()))
                    .collect();
                if card_nos.is_empty() {
                    return Ok(());
                }

                // First card: create choice for destination
                let first_card_id = card_ids[0];
                let current_idx = {
                    let player = gs.resolve_target_player_mut(target_m);
                    player
                        .stage
                        .stage
                        .iter()
                        .position(|&id| id == first_card_id)
                };
                let pos_name = match current_idx {
                    Some(0) => "Left",
                    Some(1) => "Center",
                    Some(2) => "Right",
                    _ => "?",
                };

                let valid_destinations =
                    self.compute_valid_position_destinations(gs, effect, target_m);
                if valid_destinations.is_empty() {
                    return Ok(());
                }
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.choice_card_no = Some(format!("position_change:self:{}", card_nos[0]));
                }
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "position|destination".to_string(),
                    description: format!(
                        "Choose destination for {} (currently at {})",
                        card_db
                            .get_card(first_card_id)
                            .map(|c| c.name.as_str())
                            .unwrap_or("member"),
                        pos_name
                    ),
                    allow_skip: effect.optional.unwrap_or(false),
                    options: Some(valid_destinations),
                });

                // Save remaining cards as pending_sequential_actions
                if card_nos.len() > 1 {
                    let mut remaining: Vec<AbilityEffect> = Vec::new();
                    for cn in &card_nos[1..] {
                        let mut sub = AbilityEffect::default();
                        sub.action = "position_change".to_string();
                        sub.target = Some(target_m.to_string());
                        sub.target_member = Some(cn.clone());
                        sub.optional = effect.optional;
                        remaining.push(sub);
                    }
                    gs.ability_queue.set_pending_commands(
                        remaining
                            .into_iter()
                            .map(crate::ability::types::Command::Effect)
                            .collect(),
                    );
                }
                return Ok(());
            }

            if !position_str.is_empty() {
                // Position is SOURCE ("member AT center"). Find that member on
                // the target's stage and create choice to pick destination.
                let player = gs.resolve_target_player_mut(target);
                let pos_idx = util::stage_position_index(&position_str)
                    .ok_or_else(|| format!("Unknown position: {}", position_str))?;
                if player.stage.stage[pos_idx] == -1 {
                    return Ok(()); // no member at source → skip this side
                }
                let valid_destinations =
                    self.compute_valid_position_destinations(gs, effect, target);
                if valid_destinations.is_empty() {
                    return Ok(());
                }
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.choice_card_no =
                        Some(format!("position_change:{}:{}", target, position_str));
                }
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "position|destination".to_string(),
                    description: "Choose destination for position change".to_string(),
                    allow_skip: effect.optional.unwrap_or(false),
                    options: Some(valid_destinations),
                });
                return Ok(());
            }

            // No position specified: check if a previous area_select stored the destination.
            let stored_area = self.selected_area.clone();
            if let Some(ref area) = stored_area {
                self.selected_area = None;
                let mut copy = effect.clone();
                copy.destination = Some(area.clone());
                return self.execute_position_change_with_destination(gs, &copy, area);
            }

            // No position specified: create choice for destination (move activating card).
            let valid_destinations = self.compute_valid_position_destinations(gs, effect, target);
            if valid_destinations.is_empty() {
                return Ok(());
            }
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.choice_card_no = Some("position_change:self".to_string());
            }
            self.pending_choice = Some(Choice::SelectTarget {
                target: "position|destination".to_string(),
                description: "Choose destination for position change".to_string(),
                allow_skip: effect.optional.unwrap_or(false),
                options: Some(valid_destinations),
            });
            return Ok(());
        }

        // Handle specific card_no target with no destination specified:
        // find the card's current position and create a destination choice.
        if effect.target_member.is_some() && position_str.is_empty() {
            let target_m = target.to_string();
            let card_no = effect.target_member.as_deref().unwrap_or("").to_string();
            let optional = effect.optional.unwrap_or(false);
            let card_db = gs.card_database.clone();
            // Collect stage card info before mutable borrow
            let stage_snapshot: Vec<(i16, String)> = {
                let player = gs.resolve_target_player_mut(&target_m);
                (0..3)
                    .filter_map(|i| {
                        let cid = player.stage.stage[i];
                        if cid == -1 {
                            None
                        } else {
                            let cn = card_db
                                .get_card(cid)
                                .map(|c| c.card_no.clone())
                                .unwrap_or_default();
                            Some((cid, cn))
                        }
                    })
                    .collect()
            };
            let target_pos = stage_snapshot.iter().position(|(_, cn)| cn == &card_no);
            if let Some(current_idx) = target_pos {
                let card_id = stage_snapshot[current_idx].0;
                let pos_name = match current_idx {
                    0 => "Left",
                    1 => "Center",
                    _ => "Right",
                };
                let card_name = card_db
                    .get_card(card_id)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "member".to_string());
                let valid_destinations =
                    self.compute_valid_position_destinations(gs, effect, &target_m);
                if valid_destinations.is_empty() {
                    return Ok(());
                }
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.choice_card_no = Some(format!("position_change:self:{}", card_no));
                }
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "position|destination".to_string(),
                    description: format!(
                        "Choose destination for {} (currently at {})",
                        card_name, pos_name
                    ),
                    allow_skip: optional,
                    options: Some(valid_destinations),
                });
            }
            return Ok(());
        }

        let card_db = self.card_db();
        let player = gs.resolve_target_player_mut(target);
        let target_index = util::stage_position_index(&position_str)
            .ok_or_else(|| format!("Unknown position: {}", position_str))?;

        let current_index = player.stage.stage.iter().position(|&card_id| {
            if card_id == -1 {
                false
            } else {
                card_db
                    .get_card(card_id)
                    .map(|c| c.card_no == target_member)
                    .unwrap_or(false)
            }
        });

        if let Some(current_idx) = current_index {
            let from_area = match current_idx {
                0 => crate::zones::MemberArea::LeftSide,
                1 => crate::zones::MemberArea::Center,
                _ => crate::zones::MemberArea::RightSide,
            };
            let to_area = match target_index {
                0 => crate::zones::MemberArea::LeftSide,
                1 => crate::zones::MemberArea::Center,
                _ => crate::zones::MemberArea::RightSide,
            };
            let (target_id, source_id) = (
                player.stage.stage[target_index],
                player.stage.stage[current_idx],
            );
            if let Ok(_) = player.stage.position_change(from_area, to_area) {
                let _ = player;
                gs.record_card_movement(target_id);
                if source_id != -1 {
                    gs.record_card_movement(source_id);
                }
            } else {
                return Err(format!(
                    "Failed to move member from {:?} to {:?}",
                    from_area, to_area
                ));
            }
        } else {
            return Err(format!("Member not found: {}", target_member));
        }
        gs.position_change_occurred_this_turn = true;
        Ok(())
    }

    pub(crate) fn compute_valid_position_destinations(
        &self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        target: &str,
    ) -> Vec<String> {
        let card_db = self.card_db();
        let player = gs.resolve_target_player(target);
        let activating_card_id = gs.activating_card;
        let group_names = effect.group_names.as_ref();
        let exclude_self = effect.exclude_self.unwrap_or(false);

        let position_names = ["left", "center", "right"];
        let mut valid = Vec::new();

        for (i, pos_name) in position_names.iter().enumerate() {
            let card_id = player.stage.stage[i];
            if card_id == -1 {
                continue;
            }

            if exclude_self && Some(card_id) == activating_card_id {
                continue;
            }

            if let Some(gn) = group_names {
                let matches = gn
                    .iter()
                    .any(|g| util::card_matches_group_str(&card_db, card_id, Some(g.as_str())));
                if !matches {
                    continue;
                }
            }

            valid.push(pos_name.to_string());
        }

        valid
    }

    pub fn execute_position_change_with_destination(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        destination: &str,
    ) -> Result<(), String> {
        let raw_target = effect.target.as_deref().unwrap_or("self");
        // "both" at resolution time means "self" (the ability controller resolves choices)
        let target = if raw_target == "both" {
            "self"
        } else {
            raw_target
        };
        let target_member = effect.target_member.as_deref().unwrap_or("this_member");
        // Check source_position first (new parser field), fall back to position
        let source_position = effect
            .source_position
            .as_deref()
            .or_else(|| effect.position.as_ref().and_then(|p| p.get_position()));

        // destination "same_area" means the area that was just vacated by the
        // previous position_change (the source area of the first move). Since
        // position_change already swaps members, this second move is redundant.
        if destination == "same_area" {
            return Ok(());
        }

        // destination "front" means the area in front of the activating member
        // (mirrored position on opponent's stage per Rule 4.5.7).
        let dest_owned: std::borrow::Cow<'_, str> = if destination == "front" {
            let front_pos = gs.activating_card.and_then(|cid| {
                let player = gs.resolve_target_player("self");
                let idx = player.stage.stage.iter().position(|&id| id == cid)?;
                let area = match idx {
                    0 => crate::zones::MemberArea::LeftSide,
                    1 => crate::zones::MemberArea::Center,
                    _ => crate::zones::MemberArea::RightSide,
                };
                let front = area.front_area();
                match front {
                    crate::zones::MemberArea::LeftSide => Some("left"),
                    crate::zones::MemberArea::Center => Some("center"),
                    crate::zones::MemberArea::RightSide => Some("right"),
                }
            });
            std::borrow::Cow::Owned(front_pos.unwrap_or("left").to_string())
        } else {
            std::borrow::Cow::Borrowed(destination)
        };
        let destination = dest_owned.as_ref();

        // Reject destination if it matches exclude_position
        if let Some(ref exclude) = effect.exclude_position {
            let exclude_idx = util::stage_position_index(exclude).unwrap_or(999);
            let dest_idx = util::stage_position_index(destination).unwrap_or(999);
            if exclude_idx == dest_idx {
                return Err(format!(
                    "Destination {} is excluded by exclude_position={}",
                    destination, exclude
                ));
            }
        }

        let target_index = util::stage_position_index(destination)
            .ok_or_else(|| format!("Unknown destination: {}", destination))?;

        if let Some(source) = source_position {
            // Source position specified: move member AT source TO destination.
            let player = gs.resolve_target_player_mut(target);
            let source_idx = util::stage_position_index(&source)
                .ok_or_else(|| format!("Unknown source position: {}", source))?;
            if player.stage.stage[source_idx] == -1 {
                return Ok(()); // no member at source, skip
            }
            if source_idx == target_index {
                return Ok(()); // same position, no move needed
            }
            let from_area2 = match source_idx {
                0 => crate::zones::MemberArea::LeftSide,
                1 => crate::zones::MemberArea::Center,
                _ => crate::zones::MemberArea::RightSide,
            };
            let to_area2 = match target_index {
                0 => crate::zones::MemberArea::LeftSide,
                1 => crate::zones::MemberArea::Center,
                _ => crate::zones::MemberArea::RightSide,
            };
            let (target_id2, source_id2) = (
                player.stage.stage[target_index],
                player.stage.stage[source_idx],
            );
            player.stage.position_change(from_area2, to_area2)?;
            let _ = player;
            gs.record_card_movement(target_id2);
            if source_id2 != -1 {
                gs.record_card_movement(source_id2);
            }
            gs.position_change_occurred_this_turn = true;
            return Ok(());
        }

        // Handle specific card_no (for "multiple_targets" each-member pattern)
        if let Some(ref card_no) = effect.target_member {
            if card_no != "this_member" {
                let card_db = self.card_db();
                let player = gs.resolve_target_player_mut(target);
                let current_index = player.stage.stage.iter().position(|&cid| {
                    if cid == -1 {
                        false
                    } else {
                        card_db
                            .get_card(cid)
                            .map(|c| c.card_no == *card_no)
                            .unwrap_or(false)
                    }
                });
                if let Some(current_idx) = current_index {
                    if current_idx == target_index {
                        return Ok(());
                    }
                    let from_area = match current_idx {
                        0 => crate::zones::MemberArea::LeftSide,
                        1 => crate::zones::MemberArea::Center,
                        _ => crate::zones::MemberArea::RightSide,
                    };
                    let to_area = match target_index {
                        0 => crate::zones::MemberArea::LeftSide,
                        1 => crate::zones::MemberArea::Center,
                        _ => crate::zones::MemberArea::RightSide,
                    };
                    let (target_id, source_id) = (
                        player.stage.stage[target_index],
                        player.stage.stage[current_idx],
                    );
                    player.stage.position_change(from_area, to_area)?;
                    let _ = player;
                    gs.record_card_movement(target_id);
                    if source_id != -1 {
                        gs.record_card_movement(source_id);
                    }
                }
                gs.position_change_occurred_this_turn = true;
                return Ok(());
            }
        }

        if target_member == "this_member" {
            if let Some(activating_card_id) = self.activating_card_id {
                let player = gs.resolve_target_player_mut(target);

                let current_index = player
                    .stage
                    .stage
                    .iter()
                    .position(|&card_id| card_id == activating_card_id);

                if let Some(current_idx) = current_index {
                    if current_idx == target_index {
                        return Ok(());
                    }
                    let from_area3 = match current_idx {
                        0 => crate::zones::MemberArea::LeftSide,
                        1 => crate::zones::MemberArea::Center,
                        _ => crate::zones::MemberArea::RightSide,
                    };
                    let to_area3 = match target_index {
                        0 => crate::zones::MemberArea::LeftSide,
                        1 => crate::zones::MemberArea::Center,
                        _ => crate::zones::MemberArea::RightSide,
                    };
                    let (target_id3, source_id3) = (
                        player.stage.stage[target_index],
                        player.stage.stage[current_idx],
                    );
                    player.stage.position_change(from_area3, to_area3)?;
                    let _ = player;
                    gs.record_card_movement(target_id3);
                    if source_id3 != -1 {
                        gs.record_card_movement(source_id3);
                    }
                } else {
                    return Err(format!(
                        "Activating card {} not found on stage",
                        activating_card_id
                    ));
                }
            } else {
                return Err("No activating card for position change".to_string());
            }
        }
        gs.position_change_occurred_this_turn = true;
        Ok(())
    }

    pub fn execute_rotation(
        &mut self,
        gs: &mut GameState,
        _effect: &AbilityEffect,
        target: &str,
    ) -> Result<(), String> {
        let tgt = if target == "both" { "self" } else { target };
        let moved_card_ids: Vec<i16> = {
            let player = gs.resolve_target_player_mut(tgt);

            // Snapshot current stage
            let snapshot_cards = player.stage.stage;
            let snapshot_under = player.stage.under_cards.clone();

            // Rotation mapping: left(0)→right(2), center(1)→left(0), right(2)→center(1)
            let rotation_map = [2usize, 0, 1];

            // Clear the stage
            for i in 0..3 {
                player.stage.stage[i] = -1;
                player.stage.under_cards[i].clear();
            }

            let mut moved = Vec::new();
            // Place rotated members
            for src_idx in 0..3 {
                let card_id = snapshot_cards[src_idx];
                if card_id == -1 {
                    continue;
                }
                let dest_idx = rotation_map[src_idx];
                player.stage.stage[dest_idx] = card_id;
                player.stage.under_cards[dest_idx] = snapshot_under[src_idx].clone();
                moved.push(card_id);
            }
            moved
        };

        for &cid in &moved_card_ids {
            gs.record_card_movement(cid);
        }

        gs.position_change_occurred_this_turn = true;
        Ok(())
    }

    pub(crate) fn execute_appear(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let source = effect.source_or("");
        let destination = effect.destination.as_deref().unwrap_or("stage");
        let count = effect.count_or(1);
        let target = effect.target_name();
        let card_type = effect.card_type.as_deref();
        let card_db = self.card_db();
        let player = gs.resolve_target_player_mut(target);

        match source {
            "deck" => {
                let mut appeared = 0;
                let mut cards_to_record: Vec<i16> = Vec::new();
                while appeared < count {
                    if let Some(card) = player.main_deck.draw() {
                        let matches_type = util::card_matches_type(&card_db, card, card_type);
                        if matches_type {
                            if destination == "stage" {
                                if let Some(pos) = util::stage_first_empty(&player.stage.stage) {
                                    player.stage.stage[pos] = card;
                                    player.areas_locked_this_turn.insert(util::pos_to_area(pos));
                                } else {
                                    player.hand.add_card(card);
                                }
                                cards_to_record.push(card);
                            } else {
                                util::place_card_in_zone(player, card, destination, None, false, 1);
                            }
                            appeared += 1;
                        } else {
                            player.main_deck.cards.push(card);
                        }
                    } else {
                        break;
                    }
                }
                for card_id in cards_to_record {
                    gs.record_card_movement(card_id);
                }
            }
            "discard" => {
                let mut appeared = 0;
                let mut indices_to_remove = Vec::new();
                for (i, card) in player.waitroom.cards.iter().enumerate() {
                    if appeared >= count {
                        break;
                    }
                    let matches_type = util::card_matches_type(&card_db, *card, card_type);
                    if matches_type {
                        indices_to_remove.push(i);
                        appeared += 1;
                    }
                }
                for i in indices_to_remove.into_iter().rev() {
                    let card = player.waitroom.cards.remove(i);
                    player.hand.add_card(card);
                }
            }
            "hand" => {
                let eligible: Vec<i16> = player
                    .hand
                    .cards
                    .iter()
                    .copied()
                    .filter(|&cid| {
                        if let Some(ct) = card_type {
                            if !util::card_matches_type(&card_db, cid, Some(ct)) {
                                return false;
                            }
                        }
                        if !util::card_matches_cost_limit_op(
                            &card_db,
                            cid,
                            effect.cost_limit,
                            effect.cost_limit_operator.as_deref(),
                        ) {
                            return false;
                        }
                        true
                    })
                    .collect();
                if let Some(&card_id) = eligible.first() {
                    if let Some(idx) = player.hand.cards.iter().position(|&c| c == card_id) {
                        player.hand.cards.remove(idx);
                        util::place_card_in_zone(player, card_id, destination, None, false, 1);
                        gs.record_card_movement(card_id);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn execute_choice(
        &mut self,
        gs: &mut GameState,
        choice_options: Option<&Vec<String>>,
        choice_type: Option<&str>,
        options: Option<&Vec<AbilityEffect>>,
        choice_maker: Option<&str>,
    ) -> Result<(), String> {
        // If a selection was already made (from a prior choice resolution),
        // execute the selected option's effect instead of creating another choice.
        if let Some(effect_options) = options {
            if let Some(entry) = gs.ability_queue.current_entry() {
                if let Some(ref cc) = entry.conditional_choice {
                    if let Ok(idx) = cc.parse::<usize>() {
                        if idx < effect_options.len() {
                            let selected = &effect_options[idx];
                            return self.execute_effect(gs, selected);
                        }
                    }
                }
            }
        }
        if let Some(alt_options) = choice_options {
            if let Some(entry) = gs.ability_queue.current_entry() {
                if let Some(ref cc) = entry.conditional_choice {
                    if alt_options.contains(cc) {
                        // String-based choice matched — nothing to execute, choice was just informational
                        return Ok(());
                    }
                }
            }
        }
        let options_json = options
            .and_then(|opts| serde_json::to_string(opts).ok())
            .or_else(|| choice_options.and_then(|opts| serde_json::to_string(opts).ok()));
        if let Some(entry) = gs.ability_queue.current_entry_mut() {
            entry.choice_card_no = if options.is_some() {
                Some("choice".to_string())
            } else if choice_options.is_some() {
                Some("choice_string".to_string())
            } else {
                Some("choice".to_string())
            };
            entry.conditional_choice = options_json;

            // Set choice_player_id based on choice_maker
            if choice_maker == Some("opponent") {
                let current_player_id = entry.player_id.clone();
                let opponent_id = if current_player_id == "p1" {
                    "p2".to_string()
                } else {
                    "p1".to_string()
                };
                entry.choice_player_id = Some(opponent_id);
            } else {
                entry.choice_player_id = Some(entry.player_id.clone());
            }
        }
        if let Some(effect_options) = options {
            let description = effect_options
                .iter()
                .map(|o| {
                    o.answers
                        .as_ref()
                        .map(|a| a.join(", "))
                        .unwrap_or_else(|| o.text.clone())
                })
                .collect::<Vec<_>>()
                .join(" / ");
            self.pending_choice = Some(Choice::SelectTarget {
                target: "choice".to_string(),
                description,
                allow_skip: false,
                options: None,
            });
        } else if let Some(string_options) = choice_options {
            self.pending_choice = Some(Choice::SelectTarget {
                target: "choice_string".to_string(),
                description: format!("Choose one: {}", string_options.join(", ")),
                allow_skip: false,
                options: None,
            });
        } else if let Some(ct) = choice_type {
            self.pending_choice = Some(Choice::SelectTarget {
                target: "choice".to_string(),
                description: format!("Choose: {}", ct),
                allow_skip: false,
                options: None,
            });
        }
        Ok(())
    }

    pub(crate) fn execute_pay_energy(&mut self, gs: &mut GameState, count: u32, target: &str) {
        let player = gs.resolve_target_player_mut(target);
        if count > 0 {
            if let Err(e) = player.energy_zone.pay_energy(count as usize) {
                eprintln!("{}", e);
            }
        }
    }

    pub(crate) fn execute_discard_until_count(
        &mut self,
        gs: &mut GameState,
        target_count: u32,
        target: &str,
    ) -> Result<(), String> {
        let player = gs.resolve_target_player_mut(target);
        let current_count = player.hand.cards.len();
        if current_count <= target_count as usize {
            return Ok(());
        }
        let cards_to_discard = current_count - target_count as usize;
        self.pending_choice = Some(
            Choice::select_cards(
                "hand",
                cards_to_discard,
                format!(
                    "Discard {} cards from hand (target: {} cards in hand)",
                    cards_to_discard, target_count
                ),
                false,
            )
            .target_player_id(Some(target.to_string()))
            .build(),
        );
        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
        Ok(())
    }

    pub(crate) fn execute_restriction(
        &mut self,
        gs: &mut GameState,
        restriction_type: Option<&str>,
        restricted_destination: Option<&str>,
        target: &str,
        delayed: bool,
    ) -> Result<(), String> {
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log.push(format!(
            "{} {}: 制限追加 type={:?} dest={:?} delayed={}",
            pp, act_name, restriction_type, restricted_destination, delayed
        ));
        let restriction_str = format!(
            "restriction:{}:{}",
            restriction_type.unwrap_or("unknown"),
            restricted_destination.unwrap_or("")
        );
        if delayed {
            gs.delayed_prohibition_effects.push(restriction_str);
        } else {
            gs.prohibition_effects.push(restriction_str);
        }
        // Handle cannot_activate restrictions — store for checking during Active phase
        if restriction_type == Some("cannot_activate")
            || restriction_type == Some("cannot_activate_by_effect")
            || restriction_type == Some("cannot_active")
        {
            let resolved = gs.resolve_target_player(target).id.clone();
            if !gs.cannot_activate_members.contains(&resolved) {
                gs.cannot_activate_members.push(resolved);
            }
        }
        Ok(())
    }

    pub(crate) fn execute_re_yell(
        &mut self,
        gs: &mut GameState,
        lose_blade_hearts: bool,
        target: &str,
    ) {
        eprintln!("re_yell: lose_blade_hearts={}", lose_blade_hearts);
        let card_db = self.card_db();
        let mut cards_to_clear_modifiers: Vec<i16> = Vec::new();
        {
            let player = gs.resolve_target_player_mut(target);
            for i in 0..3 {
                if player.stage.stage[i] != -1 {
                    if let Some(card_id) =
                        player.remove_member_from_stage_with_recycling(i, &card_db)
                    {
                        if lose_blade_hearts {
                            cards_to_clear_modifiers.push(card_id);
                        }
                    }
                }
            }
        }
        if lose_blade_hearts {
            for card_id in cards_to_clear_modifiers {
                gs.mods.clear_all_for_card(card_id);
            }
        }
        gs.prohibition_effects.push("re_yell".to_string());
    }

    pub(crate) fn execute_activation_restriction(&mut self, gs: &mut GameState, target: &str) {
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: 起動制限 target={}", pp, act_name, target));
        gs.prohibition_effects
            .push(format!("activation_restriction:{}", target));
    }

    pub(crate) fn execute_choose_required_hearts(&mut self, _gs: &mut GameState) {
        self.pending_choice = Some(Choice::SelectTarget {
            target: "choose_required_hearts".to_string(),
            description: "Choose required hearts".to_string(),
            allow_skip: false,
            options: None,
        });
    }

    pub(crate) fn execute_shuffle(&mut self, gs: &mut GameState, target: &str, source: &str) {
        let player = gs.resolve_target_player_mut(target);
        match source {
            "deck" => {
                use rand::seq::SliceRandom;
                player.main_deck.cards.shuffle(&mut rand::thread_rng());
            }
            "energy_deck" => {
                use rand::seq::SliceRandom;
                player.energy_deck.cards.shuffle(&mut rand::thread_rng());
            }
            _ => {
                eprintln!("Unknown shuffle zone: {}", source);
            }
        }
    }

    pub(crate) fn player_prefix(&self, gs: &GameState) -> String {
        if let Some(card_id) = gs.activating_card {
            if gs.player1.stage.stage.contains(&card_id)
                || gs.player1.live_card_zone.cards.contains(&card_id)
                || gs.player1.hand.cards.contains(&card_id)
            {
                return "P1".to_string();
            }
            if gs.player2.stage.stage.contains(&card_id)
                || gs.player2.live_card_zone.cards.contains(&card_id)
                || gs.player2.hand.cards.contains(&card_id)
            {
                return "P2".to_string();
            }
        }
        if gs.player1.id == gs.active_player().id {
            "P1"
        } else {
            "P2"
        }
        .to_string()
    }

    pub(crate) fn card_name<'a>(&self, card_id: i16) -> String {
        self.card_db()
            .get_card(card_id)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| format!("Card#{}", card_id))
    }
}
