//! `set_heart_type` and its two variants.
//!
//! 3 of the 936 unique abilities (`docs/ABILITY_MATRIX.md`).
use crate::core::constants::CountCast;
use super::super::enums::{Zone};
use super::super::resolver::AbilityResolver;
use super::super::types::{Choice};
use super::super::util;
use crate::ability::queue::ConditionalChoice;
use crate::card::{AbilityEffect};

use crate::game_state::GameState;



#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

impl AbilityResolver {

    pub(crate) fn execute_set_heart_type(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        // C4: heart becomes the same as the card just placed under this member
        // (ref_value="placed_under") — a copy, not a fixed color.
        if effect.ref_value_any() == Some("placed_under") {
            self.execute_set_heart_copy_from_under(gs, effect.duration_any());
            return;
        }
        let is_self_target = effect.is_self_target();
        let needs_target = !is_self_target
            && (effect.heart_selection_any().unwrap_or(false)
                || effect.group_names_any().is_some()
                || effect.card_type_any() == Some(&crate::card::CardType::Member));
        let ht_binding = effect.heart_type_any();
        let heart_type = ht_binding.or(effect.heart_colors_any().first().map(|s| s.as_str()));

        if is_self_target || !needs_target {
            // Self-target (e.g. Kanan PL!S-pb1-003-R): apply to activating_card.
            // Also fallback for member-card abilities without group/selection signals.
            self.execute_set_heart_type_applied(
                gs,
                heart_type,
                effect.target_name(),
                effect.count_or(1) as i32,
                effect.duration_any(),
            );
        } else if self.selection.cards.is_empty() {
            // Need target selection: find eligible stage members
            let target = effect.target_name();
            let stage_ids: Vec<i16> = {
                let p = gs.resolve_target_player(target);
                p.stage
                    .stage
                    .iter()
                    .copied()
                    .filter(|&id| id != -1)
                    .collect()
            };
            let card_db = self.card_db();
            let filter = effect.filter_subset();
            let candidates =
                util::matching_ids_filtered(&stage_ids, &card_db, &filter, true, None, None, None);
            if candidates.is_empty() {
                // No eligible targets — no-op
                return;
            }
            let tc = usize::from(effect.target_count_any().unwrap_or(1));
            if candidates.len() <= tc {
                // Auto-select: push to selected_cards and apply
                for &cid in &candidates {
                    if !self.selection.cards.contains(&cid) {
                        self.selection.cards.push(cid);
                    }
                }
                self.execute_set_heart_type_applied(
                    gs,
                    heart_type,
                    effect.target_name(),
                    effect.count_or(1) as i32,
                    effect.duration_any(),
                );
            } else {
                // Multiple eligible: create SelectCard choice
                let stage_snapshot: Vec<i16> = {
                    let p = gs.resolve_target_player(target);
                    p.stage.stage.to_vec()
                };
                let filtered_indices: Vec<usize> = candidates
                    .iter()
                    .filter_map(|&cid| stage_snapshot.iter().position(|&s| s == cid))
                    .collect();
                let mut saved = effect.clone();
                saved.set_target_count(None);
                let mut pending = gs.ability_queue.take_pending_actions();
                pending.insert(0, saved);
                gs.ability_queue.set_pending_actions(pending);
                let desc_en = format!("Select {} member(s) for heart type conversion", tc);
                let desc_ja = format!("ハート種類変換のメンバーを{}体選択", tc);
                self.awaiting.choice = Some(
                    Choice::select_cards(Zone::Stage.to_str().to_string(), tc, desc_en, false)
                        .description_ja(Some(desc_ja))
                        .card_type(effect.card_type_any().map(|s| s.to_string()))
                        .group(effect.group_name().map(|s| s.to_string()))
                        .characters(effect.characters_any().cloned())
                        .filtered_indices(Some(filtered_indices))
                        .target_player_id(Some(target.to_string()))
                        .is_select_action(true)
                        .build(),
                );
                self.awaiting.stage_select_intent =
                    Some(crate::ability::types::StageSelectIntent::CollectTargets);
                self.in_flight.sub_choice_created = true;
            }
        } else {
            // Already have selected target from previous choice resolution
            self.execute_set_heart_type_applied(
                gs,
                heart_type,
                effect.target_name(),
                effect.count_or(1) as i32,
                effect.duration_any(),
            );
        }
    }

    /// Apply a resolved heart type to the target card(s). Split out from
    /// `execute_set_heart_type` so the dispatch stays a pure one-liner.
    pub(crate) fn execute_set_heart_type_applied(
        &mut self,
        gs: &mut GameState,
        heart_type: Option<&str>,
        _target: &str,
        _count: i32,
        duration: Option<&str>,
    ) {
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        // "selected" means the heart type was chosen by a preceding select action
        // in a Sequential effect; look up the choice from the queue entry.
        let resolved_heart_type = match heart_type {
            Some("selected") => {
                gs.ability_queue
                    .current_entry()
                    .and_then(|e| match &e.conditional_choice {
                        Some(ConditionalChoice::Str(s)) => Some(s.as_str()),
                        _ => None,
                    })
            }
            other => other,
        };
        let ht = resolved_heart_type.unwrap_or("heart00").to_string();
        gs.push_rule_log_fmt(format_args!(
            "{} {}: [[log_set_heart_type:type={}]]",
            pp, act_name, ht
        ));
        // Use selected_target from self.selected_cards if available (member-targeting
        // abilities like PL!HS-bp5-021-L), otherwise fall back to activating_card
        // (self-targeting abilities like Kanan PL!S-pb1-003-R).
        let card_id = self
            .selection.cards
            .first()
            .copied()
            .or(gs.activating_card)
            .unwrap_or(-1);
        if card_id == -1 {
            return;
        }
        let color = crate::card::parse_heart_color(&ht);
        gs.mods.heart_color_multiplier.insert(card_id, color);
        gs.record_ability_application(
            card_id,
            format!("Transform hearts to {}", ht),
            "transform",
            card_id,
            Some(color.index().u8_count()),
            0,
        );
        let ed = crate::core::types::EffectData::SetBladeCount { card_id };
        util::push_temporary_effect(
            gs,
            "set_heart_type",
            duration,
            "self",
            &format!("Set heart type to {} for card {}", ht, card_id),
            Some(ed),
        );
    }

    pub(crate) fn execute_set_heart_copy_from_under(
        &mut self,
        gs: &mut GameState,
        duration: Option<&str>,
    ) {
        // "このメンバーが元々持つハートは、これにより下に置いたメンバーカードが持つ
        // ハートと同じになる" — copy the hearts of the card just placed under this
        // member (from the preceding move_cards sub-action) onto the member.
        let member_card = self
            .selection.cards
            .first()
            .copied()
            .or(gs.activating_card)
            .or(self.session.activating_card_id)
            .unwrap_or(-1);
        if member_card == -1 {
            return;
        }
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        // The source card is the one placed under this member by the move that
        // ran just before this step. Prefer moved_cards (the sequential's own
        // moved cards) that now sit under the member; fall back to scanning the
        // member's under_cards for the most recent card.
        let source_card: Option<i16> = {
            let player = gs.resolve_target_player("self");
            let pos = player.stage.stage.iter().position(|&id| id == member_card);
            let under = pos.and_then(|p| player.stage.under_cards.get(p));
            match under {
                Some(uc) if !uc.is_empty() => uc
                    .iter()
                    .rev()
                    .find(|&&cid| self.selection.moved_cards.contains(&cid))
                    .copied()
                    .or_else(|| uc.last().copied()),
                _ => self.selection.moved_cards.last().copied(),
            }
        };
        let Some(source) = source_card else {
            return;
        };
        gs.push_rule_log_fmt(format_args!(
            "{} {}: [[log_set_heart_copy:target={},source={}]]",
            pp, act_name, member_card, source
        ));
        gs.mods.set_heart_copy(member_card, source);
        gs.record_ability_application(
            member_card,
            format!("Copy hearts from card {}", source),
            "heart_copy",
            source,
            None,
            0,
        );
        let ed = crate::core::types::EffectData::SetBladeCount {
            card_id: member_card,
        };
        util::push_temporary_effect(
            gs,
            "set_heart_type",
            duration,
            "self",
            &format!("Copy hearts from card {} onto card {}", source, member_card),
            Some(ed),
        );
    }
}