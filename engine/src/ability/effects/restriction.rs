//! `restriction`, `activation_restriction`, `modify_limit` and
//! `reduce_live_card_set_limit`.
//!
//! 8 of the 936 unique abilities emit `restriction`; the other three are the
//! limit-changing actions that share the mechanism.


use super::super::resolver::AbilityResolver;



use crate::card::{AbilityEffect};

use crate::game_state::GameState;



#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

impl AbilityResolver {

    pub(crate) fn execute_restriction(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let restriction_type_binding = effect.restriction_type_any();
        let restriction_type = restriction_type_binding;
        let restricted_dest_binding = effect.restricted_destination_any();
        let restricted_destination = restricted_dest_binding
            .or(effect.destination.map(|z| z.as_str()));
        let target = effect.target_name();
        let delayed = effect.delayed_any().unwrap_or(false);
        let pp = self.player_prefix(gs);
        if crate::core::runtime_flags::logging_enabled() {
            let act_name = gs
                .activating_card
                .map(|c| self.card_name(c))
                .unwrap_or_default();
            gs.push_rule_log_fmt(format_args!("{} {}: [[log_restriction]]", pp, act_name));
        }
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
        //
        // ONE normalized type reaches here: the parser collapses every
        // 「アクティブにならない」 spelling to `cannot_activate`, with `delayed`
        // as the orthogonal 「次のターン」 marker (HS-PR-038 ab#1 and the bp7
        // energy/member wait cards). A standing (non-delayed) effect-emitted
        // `cannot_activate` stays owned by the constants path
        // (game_state/modifiers.rs); routing it into the non-delayed arm here
        // would block EVERY member of the target player instead of the one
        // card. `cannot_activate_by_effect` is the separate effect-only
        // spelling (「効果によってはアクティブにならない」).
        if restriction_type == Some("cannot_activate_by_effect")
            || (delayed && restriction_type == Some("cannot_activate"))
        {
            if delayed {
                // Per-card "next turn only" cannot_active flag.
                // Q280: the restriction's "それらのエネルギーカード" refers to the cards
                // this ability just moved (e.g. energy placed in wait), NOT the card
                // that owns the ability. Key the flag on the recently-moved cards so
                // it survives being later activated/waited by other effects.
                // A preceding change_state member-op step (e.g. "メンバーを2人まで
                // ウェイトにする。そのメンバーは次のターンの…アクティブしない") records
                // its victims in changed_state_members — those take precedence since
                // they were populated most recently by THIS ability's steps and no
                // move-based flow touches that field.
                // Fall back to the activating card for member "次のターンにアクティブしない"
                // restrictions where the target is the ability's own member.
                if !self.selection.changed_state_members.is_empty() {
                    for &cid in self.selection.changed_state_members.iter() {
                        if cid != -1 {
                            gs.mods.add_delayed_cannot_active(cid, 1);
                            log::debug!(
                                "[RESTRICT_DELAYED] keyed on changed_state_members: cid={}",
                                cid
                            );
                        }
                    }
                    self.selection.changed_state_members.clear();
                } else if let Some(moved) = gs.recently_moved_cards.as_ref() {
                    for &cid in moved.iter() {
                        if cid != -1 {
                            gs.mods.add_delayed_cannot_active(cid, 1);
                        }
                    }
                } else if let Some(card_id) = gs.activating_card {
                    gs.mods.add_delayed_cannot_active(card_id, 1);
                }
            } else {
                let resolved = gs.resolve_target_player(target).id.to_string();
                if !gs.cannot_activate_members.contains(&resolved) {
                    gs.cannot_activate_members.push(resolved);
                }
            }
        }
        // Handle cannot_live restrictions — store per-player
        if restriction_type == Some("cannot_live") {
            let resolved = gs.resolve_target_player(target).id.clone();
            if !gs.cannot_live_players.contains(&resolved) {
                gs.cannot_live_players.push(resolved);
            }
        }
        // Handle cannot_wait_by_effect — record the protected member cards.
        // "…は相手の効果によってはウェイトしない" — the members matching the
        // restriction's filter (group + blade-limit) on the CONTROLLER'S OWN stage
        // are immune to wait effects from the opponent. The restriction protects
        // the owner's own members ("自分のステージにいる…"), so resolve the owner
        // from the ability controller, not the parsed (often misleading) target.
        if restriction_type == Some("cannot_wait_by_effect") {
            let master = gs.ability_master_id();
            let owner_is_p2 = matches!(
                master,
                Some("player2") | Some("p2")
            );
            let (owner_stage, owner_id): (Vec<i16>, String) = if owner_is_p2 {
                (
                    gs.player2.stage.stage.to_vec(),
                    gs.player2.id.to_string(),
                )
            } else {
                (
                    gs.player1.stage.stage.to_vec(),
                    gs.player1.id.to_string(),
                )
            };
            let card_db = &gs.card_database;
            let groups: Vec<String> = effect
                .group_names_any()
                .map(|g| g.to_vec())
                .unwrap_or_default();
            let ct = effect.card_type_any().map(|c| c.as_card_str());
            let blade_limit = effect.blade_limit_any().unwrap_or(u8::MAX);
            let blade_op = effect.blade_limit_operator_any().map(|o| o.as_str());
            let original_blade = effect.original_value_any().unwrap_or(false);
            let mut to_protect: Vec<i16> = Vec::new();
            for &cid in &owner_stage {
                if cid == -1 {
                    continue;
                }
                let base_blade = card_db.get_card(cid).map(|c| c.blade).unwrap_or(0);
                if let Some(t) = ct {
                    if !crate::ability::util::card_matches_type(card_db, cid, Some(t)) {
                        continue;
                    }
                }
                if !groups.is_empty()
                    && !groups.iter().any(|g| {
                        crate::ability::util::card_matches_group_str(card_db, cid, Some(g))
                    })
                {
                    continue;
                }
                // `i32` for the same reason as the limit being compared against
                // it: a negative modifier is a legal blade value here and must
                // survive the addition rather than floor at zero.
                let current_blade =
                    i32::from(base_blade) + i32::from(gs.mods.get_blade_modifier(cid));
                let blade = if original_blade {
                    i32::from(base_blade)
                } else {
                    current_blade
                };
                log::debug!(
                    "[WAIT_IMMUNITY_FILTER] member={} printed_blade={} current_blade={} limit={} original_value={}",
                    cid,
                    base_blade,
                    current_blade,
                    blade_limit,
                    original_blade
                );
                if crate::ability::util::compare_counts(
                    blade_op,
                    crate::constants::saturate_u8(blade),
                    blade_limit,
                ) {
                    to_protect.push(cid);
                }
            }
            // Store as (member_id, owner_id) so the wait check knows whose member
            // is protected; opponents' wait effects on that member are blocked.
            for cid in to_protect {
                if !gs
                    .wait_immune_members
                    .iter()
                    .any(|(m, _)| *m == cid)
                {
                    gs.wait_immune_members.push((cid, owner_id.clone()));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn execute_activation_restriction(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) {
        let target = effect.target_name();
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log_fmt(format_args!(
            "{} {}: [[log_activation_restriction:target={}]]",
            pp, act_name, target
        ));
        gs.prohibition_effects
            .push(format!("activation_restriction:{}", target));
    }

    pub(crate) fn execute_reduce_live_card_set_limit(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) {
        let count: u8 = effect.count_or(1);
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log_fmt(format_args!(
            "{} {}: [[log_reduce_live_set_limit:n={}]]",
            pp, act_name, count
        ));
        let player = gs.resolve_target_player_mut("self");
        player.live_card_set_limit_reduction += count;
    }

    pub(crate) fn execute_modify_limit(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let operation_binding = effect.operation_any();
        let operation = operation_binding.unwrap_or("decrease");
        let count: u8 = effect.count_or(0);
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log_fmt(format_args!(
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
}