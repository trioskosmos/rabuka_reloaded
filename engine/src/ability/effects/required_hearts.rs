//! Required-heart changes: `modify_required_hearts` (16),
//! `modify_required_hearts_global` (3), `modify_required_hearts_success`,
//! `specify_heart_color` and `choose_required_hearts`.
//!
//! `docs/ABILITY_MATRIX.md`, "modify required hearts". The per-unit
//! arithmetic they share with `modify_score` stays in `score.rs`.
use crate::core::constants::CountCast;

use super::super::resolver::AbilityResolver;
use super::super::types::{Choice};
use super::super::util;

use crate::card::{AbilityEffect};

use crate::game_state::GameState;



#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use super::score::{PerUnitFilter, OriginalScoreFilter, capped_units, per_unit_base, per_unit_source_cards};

impl AbilityResolver {

    pub(crate) fn execute_modify_required_hearts(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let operation_binding = effect.operation_any();
        let operation = operation_binding.unwrap_or("decrease");
        let mut value: u8 = effect.value_or_count(0);
        let heart_colors = effect.heart_colors_any();
        let target = effect.target_name();
        let per_unit = effect.per_unit_any().unwrap_or(false);
        let per_unit_count: u8 = effect.per_unit_count_any().unwrap_or(1);
        let group_name = effect.group_name();
        let timing_condition_binding = effect.timing_condition_any();
        let timing_condition = timing_condition_binding;
        let location_binding = effect.location_any();
        let location = location_binding;
        let original_value = effect.original_value_any();
        let original_count = effect.original_count_any();
        let original_operator_binding = effect.original_operator_any();
        let original_operator = original_operator_binding.as_deref();
        let exclude_self = effect.exclude_self_any().unwrap_or(false);
        let self_target = effect.is_self_target();
        let exclude_heart_colors = effect.exclude_heart_colors_any();
        let max = effect.max.unwrap_or(false);
        let repeat_limit = effect.repeat_limit_any();
        let per_unit_heart_colors = effect.per_unit_heart_colors_any();
        let distinct = effect.distinct_any();
        let is_distinct_names = matches!(
            distinct,
            Some(crate::card::DistinctType::CardName)
                | Some(crate::card::DistinctType::True)
                | Some(crate::card::DistinctType::Distinct)
        );
        if per_unit {
            let card_db = &gs.card_database;
            let player = gs.resolve_target_player(target);
            // The unit criteria, resolved once. Both per-unit branches below
            // accept the same set of cards; only how much each is worth differs.
            let unit_filter = PerUnitFilter {
                exclude_self,
                group: group_name,
                distinct_names: is_distinct_names,
                timing_condition,
                exclude_heart_colors,
            };

            if !per_unit_heart_colors.is_empty() {
                // Count total heart icon count of specified colors across matching members.
                // Used for patterns like "そのメンバーが持つheart03 2つにつき" (count heart03 on card).
                let mut total_hearts = 0u8;
                let mut seen_names = crate::HashSet::default();
                let stage_ids: Vec<i16> = player
                    .stage
                    .stage
                    .iter()
                    .filter(|&&id| id != -1)
                    .copied()
                    .collect();
                for &card_id in &stage_ids {
                    if !unit_filter.accepts(gs, card_db, card_id, &mut seen_names) {
                        continue;
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
                value = per_unit_base(max, value) * capped_units(total_hearts, per_unit_count, repeat_limit);
            } else {
                // Default per-unit: count cards in the specified location.
                let cards = per_unit_source_cards(player, location);
                let mut seen_names = crate::HashSet::default();
                let count = cards
                    .iter()
                    .filter(|&&card_id| {
                        unit_filter.accepts(gs, card_db, card_id, &mut seen_names)
                    })
                    .count()
                    .u8_count();
                value = per_unit_base(max, value) * capped_units(count, per_unit_count, repeat_limit);
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
        let original_score_filter = OriginalScoreFilter {
            enabled: original_value.unwrap_or(false),
            count: original_count,
            operator: original_operator,
        };
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
                original_score_filter.accepts(db, card_id)
            })
            .collect();
        log::debug!(
            "[MOD_REQ_HEARTS] op={} value={} colors={:?} self_target={:?} activating={:?} candidates={:?}",
            operation,
            value,
            heart_colors,
            effect.is_self_target(),
            gs.activating_card,
            card_ids
        );
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
            gs.push_rule_log_fmt(format_args!(
                "{} {}: [[log_required_hearts:op={},value={},color={}]]",
                pp, act_name, operation, per_color_value, hc
            ));
            for card_id in &card_ids {
                let (op_kind, delta) = match operation {
                    "decrease" => ("need_heart_mod", -(value as i16)),
                    "increase" => ("need_heart_mod", value as i16),
                    "set" => ("need_heart_set", per_color_value as i16),
                    _ => return Err(format!("Unknown operation: {}", operation)),
                };
                match operation {
                    "set" => {
                        gs.mods
                            .set_need_heart_modifier(*card_id, color, per_color_value as i16);
                    }
                    _ => {
                        gs.mods.add_need_heart_modifier(*card_id, color, delta);
                    }
                }
                gs.record_ability_application(
                    gs.activating_card.unwrap_or(-1),
                    effect.text.to_string(),
                    op_kind,
                    *card_id,
                    Some(color.index().u8_count()),
                    delta,
                );
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
        effect_text: &str,
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
                gs.record_ability_application(
                    gs.activating_card.unwrap_or(-1),
                    effect_text.to_string(),
                    "need_heart_mod",
                    *card_id,
                    Some(color.index().u8_count()),
                    modifier_value,
                );
            }
        }
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log_fmt(format_args!(
            "{} {}: [[log_required_hearts_std:op={},value={}]]",
            pp, act_name, operation, value
        ));
        Ok(())
    }

    pub(crate) fn execute_modify_required_hearts_success(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) {
        let operation_binding = effect.operation_any();
        let operation = operation_binding.unwrap_or("increase");
        let value: u8 = effect.value_any().unwrap_or(0);
        let target = effect.target_name();
        let card_type = effect.card_type_any().map(|ct| ct.as_card_str());
        let heart_colors = effect.heart_colors_any();
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log_fmt(format_args!(
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
                gs.record_ability_application(
                    gs.activating_card.unwrap_or(-1),
                    effect.text.to_string(),
                    "need_heart_mod",
                    card_id,
                    Some(color.index().u8_count()),
                    delta as i16,
                );
            }
        }
    }

    pub(crate) fn execute_specify_heart_color(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) {
        let choice = effect.choice_any().unwrap_or(false);
        log::debug!(
            "[SPECIFY_HEART] entry: choice_any={} action={:?}",
            choice,
            effect.action
        );
        if choice {
            // Q190 (2025.11.17): ALL heart (heart00) cannot be selected.
            // Present the 6 individual heart colors for the player to choose.
            self.awaiting.choice = Some(Choice::SelectHeartColor {
                count: 1,
                options: vec![
                    "heart01".into(),
                    "heart02".into(),
                    "heart03".into(),
                    "heart04".into(),
                    "heart05".into(),
                    "heart06".into(),
                ],
                description: "Choose a heart color".to_string(),
                description_en: Some("Choose a heart color".to_string()),
                description_ja: Some("ハートの色を選択".to_string()),
            });
        }
        let pp = self.player_prefix(gs);
        if crate::core::runtime_flags::logging_enabled() {
            let act_name = gs
                .activating_card
                .map(|c| self.card_name(c))
                .unwrap_or_default();
            gs.push_rule_log_fmt(format_args!("{} {}: ハート色指定", pp, act_name));
        }
    }

    /// Resolve heart color for gain_resource. Returns Ok(Some(color)) if fixed, Ok(None) if choice was set or not a heart resource.
    pub(crate) fn resolve_gain_heart_color(
        &mut self,
        _gs: &mut GameState,
        effect: &AbilityEffect,
        resource: &str,
        count: u8,
        heart_colors: &[String],
        heart_selection: bool,
    ) -> Result<Option<String>, String> {
        if resource != "heart" {
            return Ok(None);
        }
        if heart_colors.is_empty() && !heart_selection && effect.heart_type_any().is_none() {
            return Ok(None);
        }
        let colors: Vec<String> = if let Some(ref ht) = effect.heart_type_any() {
            vec![ht.to_string()]
        } else if !heart_colors.is_empty() {
            heart_colors.to_vec()
        } else {
            vec![
                "heart01".into(),
                "heart02".into(),
                "heart03".into(),
                "heart04".into(),
                "heart05".into(),
                "heart06".into(),
            ]
        };
        let mut unique_colors: Vec<String> = Vec::new();
        for c in colors {
            if !unique_colors.contains(&c) {
                unique_colors.push(c);
            }
        }
        if unique_colors.len() == 1 && !heart_selection {
            Ok(Some(unique_colors[0].clone()))
        } else if !heart_selection
            && unique_colors.len() > 1
            && count as usize >= unique_colors.len()
        {
            // Multiple fixed heart colors with matching or exceeding count.
            // Don't create a choice — caller will distribute the count across all colors.
            Ok(None)
        } else {
            self.awaiting.choice = Some(Choice::SelectHeartColor {
                count: count as usize,
                options: unique_colors,
                description: "Choose a heart color".to_string(),
                description_en: Some("Choose a heart color".to_string()),
                description_ja: Some("ハートの色を選択".to_string()),
            });
            Ok(None)
        }
    }

    pub(crate) fn execute_choose_required_hearts(&mut self, gs: &mut GameState) {
        self.awaiting.choice = Some(Choice::SelectTarget {
            target: "choose_required_hearts".to_string(),
            description: "Choose required hearts".to_string(),
            description_en: Some("Choose required hearts".to_string()),
            description_ja: Some("必要なハートを選択".to_string()),
            allow_skip: false,
            options: None,
        });
        let pp = self.player_prefix(gs);
        gs.push_rule_log_fmt(format_args!("{}: [[log_heart_select]]", pp));
    }
}