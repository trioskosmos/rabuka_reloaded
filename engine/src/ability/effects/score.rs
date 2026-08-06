use super::super::enums::Zone;
use super::super::resolver::AbilityResolver;
use super::super::util;
use crate::card::AbilityEffect;
use crate::game_state::GameState;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use smallvec::SmallVec;

impl AbilityResolver {
    pub(crate) fn execute_modify_score(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let operation = effect
            .operation_any()
            .as_deref()
            .unwrap_or("add")
            .to_string();
        let value: u8 = effect.value_any().unwrap_or(0) as u8;
        let target = effect.target_name().to_string();
        let duration = effect.duration_any().map(|s| s.to_string());
        let card_type_filter = effect
            .card_type_any()
            .map(|ct| ct.as_card_str().to_string());
        let group_filter = effect.group_name().map(|s| s.to_string());
        let per_unit = effect.per_unit_any().unwrap_or(false);
        let per_unit_count_val = effect.per_unit_count_any().unwrap_or(1) as u8;
        let per_unit_type_str = effect.per_unit_type_any().map(|s| s.to_string());
        let location = effect.location_any().map(|s| s.to_string());
        let effect_constraint = effect.effect_constraint_any().map(|s| s.to_string());
        let self_target = effect.self_target_any().unwrap_or(false);
        let heart_colors = effect.heart_colors_any();
        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            eprintln!(
                "[SCORE_DIAG] execute_modify_score called: value={} target={} op={} condition={:?}",
                value,
                target,
                operation,
                effect.condition.is_some()
            );
        }
        let card_db = self.card_db();
        let exclude_self_id = if effect.exclude_self_any().unwrap_or(false) {
            gs.activating_card
        } else {
            None
        };

        let orientation_modifiers = gs.mods.orientation_modifiers.clone();
        let last_energy = gs.mods.last_cost_energy_count;
        let (live_card_ids, final_value) = {
            let player = gs.resolve_target_player_mut(&target);

            // Use CardFilter::from_effect (not filter_subset) so that
            // need_heart_total, need_heart_operator, and other condition
            // fields are included in the filter.  filter_subset() drops
            // these fields intentionally — it's a minimal set for simple
            // lookups — but modify_score + per_unit needs the full filter
            // for accurate per-unit counting (e.g. 絶対的LOVER counts only
            // Liella! members with ≥4 hearts per Q149 rules).
            let mut filter = util::CardFilter::from_effect(effect);
            if let Some(ref ct) = card_type_filter {
                filter.card_type = Some(ct.as_str());
            }
            if let Some(ref g) = group_filter {
                filter.group = Some(g.as_str());
            }
            filter.exclude_self = exclude_self_id;

            // Target selection uses a filter WITHOUT negation: card_property +
            // negation on a modify_score describe the per-unit COUNT predicate
            // (e.g. "ブレードハートを持たない...2枚につき"), not which members
            // RECEIVE the modifier. Negated targets would wrongly exclude the
            // recipients (shiki: kanon has blade_heart, still gets the score).
            let mut target_filter = filter.clone();
            target_filter.negation = false;
            target_filter.card_property = None;

            let final_value = if per_unit {
                // Determine the effective zone for per-unit counting.
                // - Special pseudo-zones like "heart_colors" come from per_unit_type
                //   and must take priority (they trigger unique counting logic).
                // - Otherwise, use location as the zone override (e.g. EMOTION
                //   has per_unit_type="枚" but location="success_live_zone").
                let pt = per_unit_type_str.as_deref();
                let effective_per_unit_zone = match pt {
                    Some("heart_colors") => pt,
                    _ => location.as_deref().or(pt),
                };
                let matching_count = if effective_per_unit_zone == Some("つ") {
                    // "つ" = counter for units; for energy costs this is the
                    // number of energy paid in the current cost step.
                    log::debug!("[PER_UNIT_つ] last_cost_energy_count={}", last_energy);
                    last_energy
                } else {
                    util::resolve_per_unit_count(
                        true,
                        effective_per_unit_zone,
                        player,
                        &card_db,
                        &filter,
                        heart_colors,
                        effect.state_any().as_deref(),
                        &orientation_modifiers,
                        None,
                    )
                };
                // per_unit_count: apply value once per N units (e.g. 4 energy = +1)
                let effective_units = matching_count / per_unit_count_val.max(1);
                let effective_units = if let Some(cap) = effect.repeat_limit_any() {
                    effective_units.min(cap)
                } else {
                    effective_units
                };
                value * effective_units
            } else {
                value
            };

            let candidate_ids: SmallVec<[i16; 8]> = match card_type_filter.as_deref() {
                Some("member_card") => util::matching_ids(
                    util::zone_cards(player, Zone::Stage.to_str()),
                    &card_db,
                    &target_filter,
                    true,
                )
                .into(),
                _ => {
                    let mut ids: SmallVec<[i16; 8]> = player
                        .live_card_zone
                        .cards
                        .iter()
                        .chain(player.success_live_card_zone.cards.iter())
                        .copied()
                        .collect();
                    ids.extend(player.stage.stage.iter().filter(|&&id| id != -1).copied());
                    ids
                }
            };
            let target_card_ids: Vec<(i16, i16)> = candidate_ids
                .iter()
                .filter(|&&card_id| {
                    if !target_filter.matches(&card_db, card_id, false) {
                        return false;
                    }
                    if self_target {
                        if let Some(activating_id) = self.activating_card_id {
                            if card_id != activating_id {
                                return false;
                            }
                        }
                    }
                    true
                })
                .map(|&card_id| {
                    let delta = match operation.as_str() {
                        "add" => final_value as i16,
                        "remove" => -(final_value as i16),
                        "set" => final_value as i16,
                        _ => 0i16,
                    };
                    (card_id, delta)
                })
                .collect();

            log::debug!(
                "▶ Score {} {} {} {}: {} applied to [{}]",
                operation,
                final_value,
                if self_target { "(self)" } else { "" },
                duration.as_deref().unwrap_or(""),
                final_value,
                target_card_ids
                    .iter()
                    .map(|(id, _)| self.fmt_card(*id))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            (target_card_ids, final_value)
        };

        let mut count_applied = 0u8;
        for (card_id, delta) in &live_card_ids {
            if let Some(constraint) = &effect_constraint {
                let current_mod = gs.mods.get_score_modifier(*card_id);
                if constraint.as_str() == "min:0" && current_mod + (*delta as i32) < 0 {
                    continue;
                }
            }
            if operation == "set" {
                gs.mods.set_score_modifier(*card_id, *delta);
                gs.record_ability_application(
                    gs.activating_card.unwrap_or(-1),
                    effect.text.to_string(),
                    "score_set",
                    *card_id,
                    None,
                    *delta,
                );
            } else {
                gs.mods.add_score_modifier(*card_id, *delta);
                gs.record_ability_application(
                    gs.activating_card.unwrap_or(-1),
                    effect.text.to_string(),
                    "score_bonus",
                    *card_id,
                    None,
                    *delta,
                );
            }
            count_applied += 1;
        }

        let effect_data = {
            let items: Vec<crate::core::types::CardEffectItem> = live_card_ids
                .iter()
                .map(|(card_id, delta)| crate::core::types::CardEffectItem {
                    card_id: *card_id,
                    amount: *delta,
                    color: None,
                })
                .collect();
            Some(crate::core::types::EffectData::MultiCard { items })
        };
        util::push_temporary_effect(
            gs,
            &format!("modify_score_{}", operation),
            duration.as_deref(),
            &target,
            &format!(
                "Modify score by {} {} (applied to {} cards)",
                operation, final_value, count_applied
            ),
            effect_data,
        );
        {
            let pp = gs.player_prefix();
            let act_name = gs
                .activating_card
                .and_then(|id| gs.card_database.get_card(id))
                .map(|c| c.name.to_string())
                .unwrap_or_default();
            gs.push_rule_log(format!(
                "{} {}: [[log_score_modify:op={},value={},applied={}]]",
                pp, act_name, operation, final_value, count_applied
            ));
        }
        Ok(())
    }

    pub(crate) fn execute_modify_required_hearts(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let operation_binding = effect.operation_any();
        let operation = operation_binding.as_deref().unwrap_or("decrease");
        let mut value: u8 = effect.value_or_count(0) as u8;
        let heart_colors = effect.heart_colors_any();
        let target = effect.target_name();
        let per_unit = effect.per_unit_any().unwrap_or(false);
        let per_unit_count: u8 = effect.per_unit_count_any().unwrap_or(1) as u8;
        let group_name = effect.group_name();
        let timing_condition_binding = effect.timing_condition_any();
        let timing_condition = timing_condition_binding.as_deref();
        let location_binding = effect.location_any();
        let location = location_binding.as_deref();
        let original_value = effect.original_value_any();
        let original_count = effect.original_count_any().map(|v| v as u8);
        let original_operator_binding = effect.original_operator_any();
        let original_operator = original_operator_binding.as_deref();
        let exclude_self = effect.exclude_self_any().unwrap_or(false);
        let self_target = effect.self_target_any().unwrap_or(false);
        let exclude_heart_colors = effect.exclude_heart_colors_any();
        let max = effect.max.unwrap_or(false);
        let repeat_limit = effect.repeat_limit_any().map(|v| v as u8);
        let per_unit_heart_colors = effect.per_unit_heart_colors_any();
        if per_unit {
            let card_db = &gs.card_database;
            let player = gs.resolve_target_player(target);

            if !per_unit_heart_colors.is_empty() {
                // Count total heart icon count of specified colors across matching members.
                // Used for patterns like "そのメンバーが持つheart03 2つにつき" (count heart03 on card).
                let mut total_hearts = 0u8;
                let stage_ids: Vec<i16> = player
                    .stage
                    .stage
                    .iter()
                    .filter(|&&id| id != -1)
                    .copied()
                    .collect();
                for &card_id in &stage_ids {
                    if exclude_self {
                        if gs.activating_card == Some(card_id) {
                            continue;
                        }
                    }
                    if let Some(g) = group_name {
                        if !util::card_matches_group_str(card_db, card_id, Some(g)) {
                            continue;
                        }
                    }
                    if let Some(card) = card_db.get_card(card_id) {
                        if let Some(ref base) = card.base_heart {
                            for hc_str in per_unit_heart_colors {
                                let hc: crate::card::HeartColor =
                                    hc_str.parse().unwrap_or(crate::card::HeartColor::Heart00);
                                total_hearts += base.hearts.get(&hc).copied().unwrap_or(0);
                            }
                        }
                    }
                }
                let per_unit_base = if max { 1 } else { value };
                let mut units = total_hearts / per_unit_count.max(1);
                if let Some(cap) = repeat_limit {
                    units = units.min(cap);
                }
                value = per_unit_base * units;
            } else {
                // Default per-unit: count cards in the specified location.
                let cards: Vec<i16> = match location {
                    Some("success_live_zone") | Some("success_live_card_zone") => {
                        player.success_live_card_zone.cards.to_vec()
                    }
                    Some("live_card_zone") | Some("live_zone") => {
                        player.live_card_zone.cards.to_vec()
                    }
                    _ => player
                        .stage
                        .stage
                        .iter()
                        .filter(|&&id| id != -1)
                        .copied()
                        .collect(),
                };
                let activating_id = gs.activating_card;
                let mut count = 0u8;
                for &card_id in &cards {
                    if exclude_self {
                        if activating_id == Some(card_id) {
                            continue;
                        }
                    }
                    if let Some(g) = group_name {
                        if !util::card_matches_group_str(card_db, card_id, Some(g)) {
                            continue;
                        }
                    }
                    if let Some(tc) = timing_condition {
                        if tc == "appeared_or_moved_this_turn" {
                            let moved = gs.has_card_moved_this_turn(card_id);
                            let appeared = gs.has_card_appeared_this_turn(card_id);
                            if !moved && !appeared {
                                continue;
                            }
                        }
                    }
                    if !exclude_heart_colors.is_empty() {
                        let card = card_db.get_card(card_id);
                        if let Some(card) = card {
                            let all_excluded = card.base_heart.as_ref().map_or(false, |base| {
                                base.hearts.keys().all(|hc| {
                                    exclude_heart_colors
                                        .iter()
                                        .any(|exc| &hc.to_string() == exc)
                                })
                            });
                            if all_excluded {
                                continue;
                            }
                        }
                    }
                    count += 1;
                }
                let per_unit_base = if max { 1 } else { value };
                let mut units = count / per_unit_count.max(1);
                if let Some(cap) = repeat_limit {
                    units = units.min(cap);
                }
                value = per_unit_base * units;
            }
        }
        let card_ids: Vec<i16> = {
            // Include success_live_card_zone only when the activating card
            // is in that zone AND has self_target.  This handles LiveStart
            // triggers from a previously successful card (e.g. EMOTION),
            // while constant-ability evaluation
            // (evaluate_success_zone_heart_reductions) only targets
            // live_card_zone.
            let activating_in_success = self_target
                && gs.activating_card.is_some_and(|cid| {
                    let player_ref = gs.resolve_target_player(target);
                    player_ref.success_live_card_zone.cards.contains(&cid)
                });
            let player = gs.resolve_target_player_mut(target);
            if activating_in_success {
                player
                    .live_card_zone
                    .cards
                    .iter()
                    .chain(player.success_live_card_zone.cards.iter())
                    .copied()
                    .collect()
            } else {
                player.live_card_zone.cards.to_vec()
            }
        };
        let db = &gs.card_database;
        let card_ids: Vec<i16> = card_ids
            .into_iter()
            .filter(|&card_id| {
                // When self_target is set, only the activating card receives the
                // modifier (e.g. ハナムスビ reduces only its own heart requirement).
                if self_target {
                    if let Some(act) = gs.activating_card {
                        if card_id != act {
                            return false;
                        }
                    }
                }
                // Filter by group_name (e.g. "μ's")
                if let Some(gn) = group_name {
                    if !util::card_matches_group_str(db, card_id, Some(gn)) {
                        return false;
                    }
                }
                // Filter by original score when original_value is set
                if original_value.unwrap_or(false) {
                    let card = db.get_card(card_id);
                    match (
                        card.and_then(|c| c.score),
                        original_count,
                        original_operator,
                    ) {
                        (Some(score), Some(threshold), Some(op)) => {
                            let met = match op {
                                ">=" => score >= threshold,
                                "<=" => score <= threshold,
                                ">" => score > threshold,
                                "<" => score < threshold,
                                "==" => score == threshold,
                                "!=" => score != threshold,
                                _ => true,
                            };
                            if !met {
                                return false;
                            }
                        }
                        _ => {}
                    }
                }
                true
            })
            .collect();
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        let colors = if heart_colors.is_empty() {
            vec!["heart00".to_string()]
        } else {
            heart_colors.to_vec()
        };
        // value is now always per-color count (parser fix: per-color, not total).
        // Each listed color gets the same per-color value.
        let per_color_value = value;
        for hc in &colors {
            let color = crate::card::parse_heart_color(hc);
            gs.push_rule_log(format!(
                "{} {}: [[log_required_hearts:op={},value={},color={}]]",
                pp, act_name, operation, per_color_value, hc
            ));
            for card_id in &card_ids {
                match operation {
                    "decrease" => {
                        gs.mods
                            .add_need_heart_modifier(*card_id, color, -(value as i16));
                    }
                    "increase" => {
                        gs.mods
                            .add_need_heart_modifier(*card_id, color, value as i16);
                    }
                    "set" => {
                        gs.mods
                            .set_need_heart_modifier(*card_id, color, per_color_value as i16);
                    }
                    _ => return Err(format!("Unknown operation: {}", operation)),
                }
            }
        }
        Ok(())
    }

    pub(crate) fn execute_modify_required_hearts_standard(
        &mut self,
        gs: &mut GameState,
        operation: &str,
        value: u8,
        heart_colors: &[String],
        target: &str,
    ) -> Result<(), String> {
        let colors = if heart_colors.is_empty() {
            vec!["heart00".to_string()]
        } else {
            heart_colors.to_vec()
        };
        let card_ids: Vec<i16> = {
            let player = gs.resolve_target_player_mut(target);
            player.live_card_zone.cards.to_vec()
        };
        for hc in &colors {
            let color = crate::card::parse_heart_color(hc);
            for card_id in &card_ids {
                let modifier_value = match operation {
                    "increase" => value as i16,
                    "decrease" => -(value as i16),
                    _ => return Err(format!("Unknown operation: {}", operation)),
                };
                gs.mods
                    .add_need_heart_modifier(*card_id, color, modifier_value);
            }
        }
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log(format!(
            "{} {}: [[log_required_hearts_std:op={},value={}]]",
            pp, act_name, operation, value
        ));
        Ok(())
    }

    pub(crate) fn execute_modify_yell_count(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let operation_binding = effect.operation_any();
        let operation = operation_binding.as_deref().unwrap_or("subtract");
        let count: u8 = effect.count_or(0) as u8;
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log(format!(
            "{} {}: [[log_yell_count:op={},n={}]]",
            pp, act_name, operation, count
        ));
        match operation {
            "add" => {
                gs.cheer_checks_required += count;
            }
            "subtract" => {
                gs.cheer_checks_required = gs.cheer_checks_required.saturating_sub(count);
            }
            "set" => {
                gs.cheer_checks_required = count;
            }
            _ => log::debug!("Unknown operation: {}", operation),
        }
    }

    pub(crate) fn execute_modify_limit(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let operation_binding = effect.operation_any();
        let operation = operation_binding.as_deref().unwrap_or("decrease");
        let count: u8 = effect.count_or(0) as u8;
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log(format!(
            "{} {}: [[log_stage_limit:op={},n={}]]",
            pp, act_name, operation, count
        ));
        match operation {
            "decrease" => {
                gs.prohibition_effects
                    .push(format!("limit_decrease:{}", count));
            }
            "increase" => {
                gs.prohibition_effects
                    .push(format!("limit_increase:{}", count));
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn execute_modify_required_hearts_success(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) {
        let operation_binding = effect.operation_any();
        let operation = operation_binding.as_deref().unwrap_or("increase");
        let value: u8 = effect.value_any().unwrap_or(0) as u8;
        let target = effect.target_name();
        let card_type = effect.card_type_any().map(|ct| ct.as_card_str());
        let heart_colors = effect.heart_colors_any();
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log(format!(
            "{} {}: [[log_success_hearts:op={},value={}]]",
            pp, act_name, operation, value
        ));
        let player = gs.resolve_target_player_mut(target);
        let card_ids: Vec<i16> = if let Some("live_card") = card_type {
            player
                .success_live_card_zone
                .cards
                .iter()
                .copied()
                .collect()
        } else {
            vec![]
        };
        let delta = match operation {
            "increase" => value as i16,
            "decrease" => -(value as i16),
            _ => {
                log::debug!("Unknown operation: {}", operation);
                return;
            }
        };
        let color_strs: Vec<&str> = if !heart_colors.is_empty() {
            heart_colors.iter().map(|s| s.as_str()).collect()
        } else {
            vec![
                "heart00", "heart01", "heart02", "heart03", "heart04", "heart05", "heart06",
            ]
        };
        for card_id in card_ids {
            for color_str in &color_strs {
                let color = crate::card::parse_heart_color(color_str);
                gs.mods
                    .add_need_heart_modifier(card_id, color, delta as i16);
            }
        }
    }
}
