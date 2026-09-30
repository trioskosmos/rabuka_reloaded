//! `modify_cost`, `set_cost`, `set_cost_to_use` and `activation_cost`.
//!
//! `modify_cost` is 16 of the 936 unique abilities; the other three are the
//! cost action types that write the same value.
use crate::core::constants::CountCast;

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

    pub(crate) fn execute_set_cost(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let value: u8 = effect.value_any().unwrap_or(0);
        let target = effect.target_name();
        let ct_binding = effect.card_type_any();
        let card_type = ct_binding;
        let player = gs.resolve_target_player_mut(target);
        let mut card_ids: SmallVec<[i16; 8]> = if Some(&crate::card::CardType::Live) == card_type {
            player.live_card_zone.cards.iter().copied().collect()
        } else if Some(&crate::card::CardType::Member) == card_type {
            player
                .stage
                .stage
                .iter()
                .filter(|&&id| id != -1)
                .copied()
                .collect()
        } else {
            player.hand.cards.iter().copied().collect()
        };
        if effect.group_names_any().is_some()
            || effect.exclude_group_names_any().is_some()
            || effect.characters_any().is_some()
            || effect.exclude_characters_any().is_some()
        {
            let filter = effect.filter_subset();
            card_ids = util::matching_ids_filtered(
                &card_ids,
                &gs.card_database,
                &filter,
                true,
                None,
                None,
                None,
            )
            .into();
        }
        let pp = self.player_prefix(gs);
        if crate::core::runtime_flags::logging_enabled() {
            let act_name = gs
                .activating_card
                .map(|c| self.card_name(c))
                .unwrap_or_default();
            gs.push_rule_log_fmt(format_args!("{} {}: [[log_set_cost]]", pp, act_name));
        }
        for card_id in card_ids {
            gs.mods.set_cost_modifier(card_id, i16::from(value));
            gs.record_ability_application(
                gs.activating_card.unwrap_or(-1),
                effect.text.to_string(),
                "cost_set",
                card_id,
                None,
                value as i16,
            );
        }
    }

    pub(crate) fn execute_modify_cost(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let op_binding = effect.operation_any();
        let operation = op_binding.unwrap_or("add");
        let target = effect.target_name();
        let ct_binding = effect.card_type_any();
        let card_type = ct_binding;
        let dur_binding = effect.duration_any();
        let duration = dur_binding;
        // Compute the final value: base value scaled by per-unit count.
        let mut value: u8 = effect.value_any().unwrap_or(0);
        if effect.per_unit_any().unwrap_or(false) {
            let put_binding = effect.per_unit_type_any();
            let loc_binding2 = effect.location_any();
            let per_unit_type_str = put_binding.or(loc_binding2).unwrap_or("枚");
            let player = gs.resolve_target_player(target);
            // Use resolve_per_unit_count which handles under_member,
            // discard, waitroom_card and other special zones that
            // zone_cards() cannot represent as a flat slice.
            let per_unit_filter = crate::ability::util::CardFilter::from_effect(effect);
            let matching_count = crate::ability::util::resolve_per_unit_count(
                true,
                Some(per_unit_type_str),
                player,
                &gs.card_database,
                &per_unit_filter,
                &[],
                effect.state_any(),
                &gs.mods.orientation_modifiers,
                gs.activating_card,
            );
            let per_unit_count = effect.per_unit_count_any().unwrap_or(1);
            let mut units = matching_count / per_unit_count;
            // Apply max_repeats cap (aliased as repeat_limit).
            // The text side-constraint "N枚までしか数えない" is parsed as
            // max_repeats on the effect.
            if let Some(cap) = effect.repeat_limit_any() {
                units = units.min(cap);
            }
            value *= units;
        }
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log_fmt(format_args!(
            "{} {}: [[log_modify_cost:op={},value={}]]",
            pp, act_name, operation, value
        ));
        let player = gs.resolve_target_player_mut(target);
        let is_hand_cost = effect.source_any() == Some("hand")
            || effect.location_any() == Some("hand")
            || effect.location_any() == Some("deck") && effect.source_any() == Some("hand");
        let mut card_ids: SmallVec<[i16; 8]> = if is_hand_cost {
            // "手札にあるこのカードのコストは..." — a cost modifier on cards in
            // HAND (play cost), not on stage members. Collect from hand.
            player.hand.cards.iter().copied().collect()
        } else if Some(&crate::card::CardType::Live) == card_type {
            player.live_card_zone.cards.iter().copied().collect()
        } else if Some(&crate::card::CardType::Member) == card_type {
            player
                .stage
                .stage
                .iter()
                .filter(|&&id| id != -1)
                .copied()
                .collect()
        } else if Some(&crate::card::CardType::Energy) == card_type {
            player.energy_zone.cards.iter().copied().collect()
        } else {
            player.hand.cards.iter().copied().collect()
        };
        // Filter by group_names etc. using the effect's CardFilter
        if effect.group_names_any().is_some()
            || effect.exclude_group_names_any().is_some()
            || effect.characters_any().is_some()
            || effect.exclude_characters_any().is_some()
        {
            let filter = effect.filter_subset();
            card_ids = util::matching_ids_filtered(
                &card_ids,
                &gs.card_database,
                &filter,
                true,
                None,
                None,
                None,
            )
            .into();
        }
        // When self_target is set, only the activating card receives the modifier
        // (e.g. "このメンバーのコストを+Nする" — only this member, not all matching).
        if effect.is_self_target() {
            if let Some(cid) = gs.activating_card {
                card_ids.retain(|id| *id == cid);
            }
        }
        log::debug!(
            "[MOD_COST_ENTRY] op={:?} offset={:?} reference={:?} value={:?} ids={:?}",
            op_binding,
            effect.cost_offset_any(),
            effect.cost_reference_any(),
            effect.value_any(),
            card_ids
        );
        // 「このメンバーのコストは、選んだメンバーが元々持つコストよりN低い/高い
        // 値に等しくなる」 (bp5-005-R family): resolve the previously selected/
        // moved card, take its ORIGINAL printed cost ± offset, and apply that as
        // an additive modifier relative to each target's own printed cost.
        //
        // NOTE: must run BEFORE the plain add/subtract/set delta match — that
        // match's wildcard arm treats any other operation string as unknown.
        if operation == "set_from_reference" {
            // 「選んだメンバー」 resolves from selection results FIRST — a
            // preceding select action stores its target in selected_cards.
            // moved_cards only holds cards physically MOVED (e.g. a discarded
            // cost payment), which would pick the wrong card here.
            let selected = self.selection.cards.last().copied();
            let moved = self.selection.moved_cards.last().copied();
            let recently = gs
                .recently_moved_cards
                .as_ref()
                .and_then(|cards| cards.last().copied());
            let Some(ref_id) = selected.or(moved).or(recently) else {
                log::debug!("[MOD_COST_REF] no selected/moved card to reference");
                return;
            };
            let ref_cost = gs
                .card_database
                .get_card(ref_id)
                .and_then(|c| c.cost)
                .unwrap_or_else(|| {
                    log::debug!("[MOD_COST_REF] ref card {} has no printed cost", ref_id);
                    0
                }) as i32;
            let offset = effect.cost_offset_any().unwrap_or(0) as i32;
            let resolved = crate::constants::saturate_u8(ref_cost.saturating_add(offset)) as i32;
            log::debug!(
                "[MOD_COST_REF] ref={} ref_cost={} offset={} resolved={}",
                ref_id,
                ref_cost,
                offset,
                resolved
            );
            let deltas: SmallVec<[i16; 8]> = card_ids
                .iter()
                .map(|&cid| {
                    let printed =
                        gs.card_database.get_card(cid).and_then(|c| c.cost).unwrap_or(0)
                            as i32;
                    (resolved - printed).i16_count()
                })
                .collect();
            for (card_id, d) in card_ids.iter().zip(deltas.iter()) {
                // Additive (not the `set` field) so live_end revert via
                // remove_cost_modifier restores exactly what was applied.
                gs.mods.add_cost_modifier(*card_id, *d);
                gs.record_ability_application(
                    gs.activating_card.unwrap_or(-1),
                    effect.text.to_string(),
                    "cost_bonus",
                    *card_id,
                    None,
                    *d,
                );
                log::debug!("[MOD_COST_REF] card={} applied delta={}", card_id, d);
            }
            if let Some(dur) = duration {
                if dur != "permanent" {
                    let target_str = target.to_string();
                    let items: Vec<crate::core::types::CardEffectItem> = card_ids
                        .iter()
                        .zip(deltas.iter())
                        .map(|(&cid, &d)| crate::core::types::CardEffectItem {
                            card_id: cid,
                            amount: d.abs(),
                            color: None,
                        })
                        .collect();
                    util::push_temporary_effect(
                        gs,
                        "modify_cost",
                        Some(dur),
                        &target_str,
                        &format!("Cost set_from_reference ({})", dur),
                        Some(crate::core::types::EffectData::MultiCard { items }),
                    );
                }
            }
            return;
        }
        let delta = match operation {
            "add" => value as i16,
            "subtract" => -(value as i16),
            "set" => value as i16,
            _ => {
                log::debug!("Unknown operation: {}", operation);
                return;
            }
        };
        for card_id in &card_ids {
            if operation == "set" {
                gs.mods.set_cost_modifier(*card_id, delta);
                gs.record_ability_application(
                    gs.activating_card.unwrap_or(-1),
                    effect.text.to_string(),
                    "cost_set",
                    *card_id,
                    None,
                    delta,
                );
            } else {
                gs.mods.add_cost_modifier(*card_id, delta);
                gs.record_ability_application(
                    gs.activating_card.unwrap_or(-1),
                    effect.text.to_string(),
                    "cost_bonus",
                    *card_id,
                    None,
                    delta,
                );
            }
        }
        if let Some(dur) = duration {
            if dur != "permanent" {
                let target_str = target.to_string();
                let items: Vec<crate::core::types::CardEffectItem> = card_ids
                    .iter()
                    .map(|&cid| crate::core::types::CardEffectItem {
                        card_id: cid,
                        amount: delta.abs(),
                        color: None,
                    })
                    .collect();
                util::push_temporary_effect(
                    gs,
                    "modify_cost",
                    Some(dur),
                    &target_str,
                    &format!("Cost {} {} ({})", operation, value, dur),
                    Some(crate::core::types::EffectData::MultiCard { items }),
                );
            }
        }
    }

    pub(crate) fn execute_activation_cost(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let operation_binding = effect.operation_any();
        let operation = operation_binding.unwrap_or("increase");
        let value: u8 = effect.value_any().unwrap_or(0);
        let target = effect.target_name();
        let duration_binding = effect.duration_any();
        let duration = duration_binding;
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log_fmt(format_args!(
            "{} {}: [[log_activation_cost:op={},value={}]]",
            pp, act_name, operation, value
        ));
        let prohibition_text = format!("activation_cost_{}_{}", operation, value);
        match target {
            "self" | "opponent" => {
                gs.prohibition_effects.push(prohibition_text);
            }
            _ => {}
        }
        util::push_temporary_effect(
            gs,
            &format!("activation_cost_{}_{}", operation, value),
            duration,
            target,
            &format!("Modify activation cost by {} {}", operation, value),
            None,
        );
    }

    pub(crate) fn execute_set_cost_to_use(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let value: u8 = effect.value_any().unwrap_or(0);
        let card_id = self.session.activating_card_id.or(gs.activating_card);
        if let Some(card_id) = card_id {
            gs.mods.set_cost_modifier(card_id, i16::from(value));
        }
        let pp = self.player_prefix(gs);
        if crate::core::runtime_flags::logging_enabled() {
            let act_name = gs
                .activating_card
                .map(|c| self.card_name(c))
                .unwrap_or_default();
            gs.push_rule_log_fmt(format_args!("{} {}: 使用コスト設定", pp, act_name));
        }
        Ok(())
    }
}