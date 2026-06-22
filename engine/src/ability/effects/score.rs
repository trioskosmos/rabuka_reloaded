use super::super::enums::Zone;
use super::super::resolver::AbilityResolver;
use super::super::util;
use crate::card::AbilityEffect;
use crate::game_state::GameState;

impl AbilityResolver {
    pub(crate) fn execute_modify_score(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        operation: &str,
        value: u32,
        target: &str,
        duration: Option<&str>,
        card_type: Option<&str>,
        group_name: Option<&str>,
        per_unit: bool,
        per_unit_count: u32,
        per_unit_type: Option<&str>,
        location: Option<&str>,
        effect_constraint: Option<&str>,
        self_target: bool,
        heart_colors: &[String],
    ) -> Result<(), String> {
        if crate::ability::debug::ABILITY_DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
            eprintln!(
                "[SCORE_DIAG] execute_modify_score called: value={} target={} op={} condition={:?}",
                value,
                target,
                operation,
                effect.condition.is_some()
            );
        }
        let operation = operation.to_string();
        let target = target.to_string();
        let duration = duration.map(|s| s.to_string());
        let card_type_filter = card_type.map(|s| s.to_string());
        let group_filter = group_name.map(|s| s.to_string());
        let per_unit_count_val = per_unit_count;
        let per_unit_type_str = per_unit_type.map(|s| s.to_string());
        let effect_constraint = effect_constraint.map(|s| s.to_string());
        let card_db = self.card_db();
        let exclude_self_id = if effect.exclude_self.unwrap_or(false) {
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

            let final_value = if per_unit {
                // Determine the effective zone for per-unit counting.
                // - Special pseudo-zones like "heart_colors" come from per_unit_type
                //   and must take priority (they trigger unique counting logic).
                // - Otherwise, use location as the zone override (e.g. EMOTION
                //   has per_unit_type="枚" but location="success_live_zone").
                let pt = per_unit_type_str.as_deref();
                let effective_per_unit_zone = match pt {
                    Some("heart_colors") => pt,
                    _ => location.or(pt),
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
                        effect.state.as_deref(),
                        &orientation_modifiers,
                    )
                };
                // per_unit_count: apply value once per N units (e.g. 4 energy = +1)
                let effective_units = matching_count / per_unit_count_val.max(1);
                value * effective_units
            } else {
                value
            };

            let candidate_ids: Vec<i16> = match card_type_filter.as_deref() {
                Some("member_card") => util::matching_ids(
                    util::zone_cards(player, Zone::Stage.to_str()),
                    &card_db,
                    &filter,
                    true,
                ),
                _ => player
                    .live_card_zone
                    .cards
                    .iter()
                    .chain(player.success_live_card_zone.cards.iter())
                    .copied()
                    .collect(),
            };
            let target_card_ids: Vec<(i16, i32)> = candidate_ids
                .iter()
                .filter(|&&card_id| {
                    if !filter.matches(&card_db, card_id, false) {
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
                        "add" => final_value as i32,
                        "remove" => -(final_value as i32),
                        "set" => final_value as i32,
                        _ => 0i32,
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

        let mut count_applied = 0u32;
        for (card_id, delta) in &live_card_ids {
            if let Some(constraint) = &effect_constraint {
                let current_mod = gs.mods.get_score_modifier(*card_id);
                if constraint.as_str() == "min:0" && current_mod + delta < 0 {
                    continue;
                }
            }
            if operation == "set" {
                gs.mods.set_score_modifier(*card_id, *delta);
                gs.record_ability_application(
                    gs.activating_card.unwrap_or(-1),
                    effect.text.clone(),
                    "score_set",
                    *card_id,
                    None,
                    *delta,
                );
            } else {
                gs.mods.add_score_modifier(*card_id, *delta);
                gs.record_ability_application(
                    gs.activating_card.unwrap_or(-1),
                    effect.text.clone(),
                    "score_bonus",
                    *card_id,
                    None,
                    *delta,
                );
            }
            count_applied += 1;
        }

        util::push_temporary_effect(
            gs,
            &format!("modify_score_{}", operation),
            duration.as_deref(),
            &target,
            &format!(
                "Modify score by {} {} (applied to {} cards)",
                operation, final_value, count_applied
            ),
            None,
        );
        Ok(())
    }

    pub(crate) fn execute_modify_required_hearts(
        &mut self,
        gs: &mut GameState,
        operation: &str,
        mut value: u32,
        heart_colors: &[String],
        target: &str,
        per_unit: bool,
        per_unit_count: u32,
        group_name: Option<&str>,
        timing_condition: Option<&str>,
        location: Option<&str>,
        original_value: Option<bool>,
        original_count: Option<u32>,
        original_operator: Option<&str>,
        exclude_self: bool,
        self_target: bool,
    ) -> Result<(), String> {
        if per_unit {
            let card_db = &gs.card_database;
            let player = gs.resolve_target_player(target);

            let cards: Vec<i16> = match location {
                Some("success_live_zone") | Some("success_live_card_zone") => {
                    player.success_live_card_zone.cards.to_vec()
                }
                Some("live_card_zone") | Some("live_zone") => player.live_card_zone.cards.to_vec(),
                _ => {
                    // Default: count stage members
                    player
                        .stage
                        .stage
                        .iter()
                        .filter(|&&id| id != -1)
                        .copied()
                        .collect()
                }
            };
            let activating_id = gs.activating_card;
            let mut count = 0u32;
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
                count += 1;
            }
            // Mirror execute_modify_cost: divide the matched count by per_unit_count
            // (every N cards) then multiply by the per-unit heart amount (value).
            value = value * (count / per_unit_count.max(1));
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
        let op_jp = match operation {
            "decrease" => "減少",
            "increase" => "増加",
            "set" => "設定",
            _ => operation,
        };
        let colors = if heart_colors.is_empty() {
            vec!["heart00".to_string()]
        } else {
            heart_colors.to_vec()
        };
        // value is now always per-color count (parser fix: per-color, not total).
        // Each listed color gets the same per-color value.
        let per_color_value = value;
        for hc in &colors {
            let color = crate::zones::parse_heart_color(hc);
            gs.rule_log.push(format!(
                "{} {}: 要求ハート{} {} {}",
                pp, act_name, op_jp, per_color_value, hc
            ));
            for card_id in &card_ids {
                match operation {
                    "decrease" => {
                        gs.mods
                            .add_need_heart_modifier(*card_id, color, -(value as i32));
                    }
                    "increase" => {
                        gs.mods
                            .add_need_heart_modifier(*card_id, color, value as i32);
                    }
                    "set" => {
                        gs.mods
                            .set_need_heart_modifier(*card_id, color, per_color_value as i32);
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
        value: u32,
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
            let color = crate::zones::parse_heart_color(hc);
            for card_id in &card_ids {
                let modifier_value = match operation {
                    "increase" => value as i32,
                    "decrease" => -(value as i32),
                    _ => return Err(format!("Unknown operation: {}", operation)),
                };
                gs.mods
                    .add_need_heart_modifier(*card_id, color, modifier_value);
            }
        }
        Ok(())
    }

    pub(crate) fn execute_modify_yell_count(
        &mut self,
        gs: &mut GameState,
        operation: &str,
        count: u32,
    ) {
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        let op_jp = match operation {
            "add" => "増加",
            "subtract" => "減少",
            "set" => "設定",
            _ => operation,
        };
        gs.rule_log.push(format!(
            "{} {}: エール回数{} {}",
            pp, act_name, op_jp, count
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
        operation: &str,
        count: u32,
    ) -> Result<(), String> {
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log.push(format!(
            "{} {}: ステージ上限{} {}",
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
        operation: &str,
        value: u32,
        target: &str,
        card_type: Option<&str>,
        heart_colors: &[String],
    ) {
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log.push(format!(
            "{} {}: 成功ライブ要求ハート{} {}",
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
            "increase" => value as i32,
            "decrease" => -(value as i32),
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
                let color = crate::zones::parse_heart_color(color_str);
                gs.mods.add_need_heart_modifier(card_id, color, delta);
            }
        }
    }
}
