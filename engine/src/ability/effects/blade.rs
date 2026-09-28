//! `set_blade_type`, `set_blade_count` and `all_blade_timing`.
//!
//! 3 of the 936 unique abilities write blade state. They share one file on
//! purpose: one file per action would be three files for three functions.


use super::super::resolver::AbilityResolver;

use super::super::util;

use crate::card::{AbilityEffect};

use crate::game_state::GameState;
use smallvec::SmallVec;


#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

impl AbilityResolver {

    pub(crate) fn execute_set_blade_type(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let bt_binding = effect.blade_type_any();
        let blade_type = bt_binding;
        let target = effect.target_name();
        let dur_binding = effect.duration_any();
        let duration = dur_binding;
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        let bt_str = blade_type.unwrap_or("none");
        gs.push_rule_log_fmt(format_args!(
            "{} {}: [[log_set_blade_type:type={}]]",
            pp, act_name, bt_str
        ));
        let card_db = self.card_db();
        let blade_color = blade_type.and_then(|bt| match bt {
            "red" | "赤ブレード" => Some(crate::card::BladeColor::Red),
            "blue" | "青ブレード" => Some(crate::card::BladeColor::Blue),
            "green" | "緑ブレード" => Some(crate::card::BladeColor::Green),
            "yellow" | "黄ブレード" => Some(crate::card::BladeColor::Yellow),
            "purple" | "紫ブレード" => Some(crate::card::BladeColor::Purple),
            _ => {
                log::debug!("[set_blade_type] Unknown blade type: {:?}", bt);
                None
            }
        });
        let mut stage_card_ids: Vec<(i16, String)> = {
            let player = gs.resolve_target_player(target);
            (0..3)
                .filter_map(|i| {
                    let id = player.stage.stage[i];
                    if id == -1 {
                        None
                    } else {
                        Some((id, player.id.to_string()))
                    }
                })
                .collect()
        };
        if effect.group_names_any().is_some()
            || effect.exclude_group_names_any().is_some()
            || effect.characters_any().is_some()
            || effect.exclude_characters_any().is_some()
        {
            let filter = effect.filter_subset();
            let ids: Vec<i16> = stage_card_ids.iter().map(|(id, _)| *id).collect();
            let filtered = util::matching_ids_filtered(
                &ids,
                &gs.card_database,
                &filter,
                true,
                None,
                None,
                None,
            );
            stage_card_ids.retain(|(id, _)| filtered.contains(id));
        }
        for (card_id, pid) in stage_card_ids {
            if let Some(color) = blade_color {
                gs.mods.set_blade_type_modifier(card_id, color);
                gs.record_ability_application(
                    gs.activating_card.unwrap_or(-1),
                    effect.text.to_string(),
                    "blade_type_set",
                    card_id,
                    None,
                    0,
                );
            }
            let ed = crate::core::types::EffectData::SetBladeCount { card_id };
            util::push_temporary_effect(
                gs,
                &format!("set_blade_type:{}", blade_type.unwrap_or("")),
                duration,
                &pid,
                &format!(
                    "Set blade type to {} for {}",
                    blade_type.unwrap_or(""),
                    card_db
                        .get_card(card_id)
                        .map(|c| c.name.as_ref())
                        .unwrap_or("unknown")
                ),
                Some(ed),
            );
        }
    }

    pub(crate) fn execute_set_blade_count(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let value: u8 = effect.value_any().unwrap_or(effect.count_or(0));
        let target = effect.target_name();
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log_fmt(format_args!(
            "{} {}: [[log_set_blade_count:n={}]]",
            pp, act_name, value
        ));
        let mut stage_cards: SmallVec<[i16; 8]> = {
            let player = gs.resolve_target_player_mut(target);
            player.stage.stage.iter().copied().collect()
        };
        stage_cards.retain(|id| *id != -1);
        if effect.group_names_any().is_some()
            || effect.exclude_group_names_any().is_some()
            || effect.characters_any().is_some()
            || effect.exclude_characters_any().is_some()
        {
            let filter = effect.filter_subset();
            stage_cards = util::matching_ids_filtered(
                &stage_cards,
                &gs.card_database,
                &filter,
                true,
                None,
                None,
                None,
            )
            .into();
        }
        if let Some(pos) = effect.position_any() {
            if let Some(p) = pos.get_position() {
                if let Some(stage_idx) = util::stage_position_index(p) {
                    let player = gs.resolve_target_player(target);
                    let expected = player.stage.stage[stage_idx];
                    if expected == -1 {
                        stage_cards.clear();
                    } else {
                        stage_cards.retain(|cid| *cid == expected);
                    }
                }
            }
        }
        for &card_id in &stage_cards {
            gs.mods.set_blade_modifier(card_id, i16::from(value));
            gs.record_ability_application(
                gs.activating_card.unwrap_or(-1),
                effect.text.to_string(),
                "blade_set",
                card_id,
                None,
                value as i16,
            );
            // Register for cleanup at live end / duration expiry
            if effect.duration_any().is_some() {
                util::push_temporary_effect(
                    gs,
                    "set_blade_count",
                    effect.duration_any(),
                    target,
                    &format!("set blade count to {} for card {}", value, card_id),
                    Some(crate::core::types::EffectData::SetBladeCount { card_id }),
                );
            }
        }
    }

    pub(crate) fn execute_all_blade_timing(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let timing_binding = effect.timing_any();
        let timing = timing_binding.unwrap_or("check_required_hearts");
        let treat_as_binding = effect.treat_as_any();
        let treat_as = treat_as_binding.unwrap_or("any_heart_color");
        let card_id = self.session.activating_card_id.or(gs.activating_card);
        if let Some(card_id) = card_id {
            gs.prohibition_effects.push(format!(
                "all_blade_timing:{}:{}:{}",
                card_id, timing, treat_as
            ));
        }
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: 全ブレードタイミング", pp, act_name));
    }
}