//! Target resolution, shared by every executor.
//!
//! Not an `ActionType`: this is the machinery that decides which members an
//! effect may touch, and it is read by the resource, position, cost and
//! score executors alike. It mirrors `TargetPlayer` in `enums.rs` and
//! `core/game_state/targeting.rs` on the rule side.
use crate::core::constants::CountCast;
use super::super::enums::{TargetPlayer, Zone};
use super::super::resolver::AbilityResolver;
use super::super::types::{Choice};
use super::super::util;

use crate::card::{AbilityEffect};

use crate::game_state::GameState;
use smallvec::SmallVec;



#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use super::resource::ResourceKind;

/// Keep only the targets standing in the one stage slot the effect names.
///
/// The blade and heart target lists each applied this inline, so a position
/// constraint could be enforced on one and silently skipped on the other. Both
/// now come through here. An unparseable or unstated position is not a
/// constraint, so it leaves the list alone.
pub(crate) fn position_limited_targets(
    player: &crate::player::Player,
    position: Option<&str>,
    targets: &[i16],
) -> Vec<i16> {
    let Some(idx) = position.and_then(util::stage_position_index) else {
        return targets.to_vec();
    };
    let expected = player.stage.stage[idx];
    targets
        .iter()
        .copied()
        .filter(|&cid| cid == expected)
        .collect()
}

/// Keep only the targets whose name matches a card moved as cost (`same_name`).
///
/// No moved card means nothing can match, so the list empties rather than
/// passing everything through. The blade and heart paths each had their own
/// copy of this; they share it now so the two cannot disagree.
pub(crate) fn same_name_targets(
    db: &crate::card::CardDatabase,
    moved: &[i16],
    targets: &[i16],
) -> Vec<i16> {
    let ref_names: Vec<String> = moved
        .iter()
        .filter_map(|&cid| db.get_card(cid).map(|c| c.name.to_string()))
        .collect();
    if ref_names.is_empty() {
        return Vec::new();
    }
    targets
        .iter()
        .copied()
        .filter(|&cid| {
            db.get_card(cid)
                .is_some_and(|c| ref_names.contains(&c.name.to_string()))
        })
        .collect()
}


impl AbilityResolver {

    /// Handles target="both" by executing the effect for self, then opponent.
    /// Returns true if the effect was fully handled (has "both" target), false otherwise.
    pub(crate) fn handle_both_targets(
            &mut self,
            gs: &mut GameState,
            effect: &AbilityEffect,
        ) -> Result<bool, String> {
        // Skip if not "both" or if this is position_change (handles "both" internally)
        if effect.target.as_deref() != Some("both")
            || effect.action == crate::ability::enums::ActionType::PositionChange
        {
            return Ok(false);
        }

        // Skip if this effect (or its primary_effect) is a MoveCards with multiple_targets to deck
        // — execute_move_cards_both handles "both" internally with opponent-first order.
        let is_move_cards_both = effect.action == crate::ability::enums::ActionType::MoveCards
            && effect.multiple_targets_any().unwrap_or(false)
            && effect.target.as_deref() == Some("deck");
        let primary_is_move_cards_both = effect
            .primary_effect()
            .as_ref()
            .is_some_and(|pe| {
                pe.action == crate::ability::enums::ActionType::MoveCards
                    && pe.multiple_targets_any().unwrap_or(false)
                    && pe.target.as_deref() == Some("deck")
            });
        log::debug!(
            "[HANDLE_BOTH] action={:?} target={:?} is_mcb={} primary_mcb={} skip={}",
            effect.action,
            effect.target,
            is_move_cards_both,
            primary_is_move_cards_both,
            is_move_cards_both || primary_is_move_cards_both
        );
        if is_move_cards_both || primary_is_move_cards_both {
            return Ok(false);
        }

        // Execute for self first
        let mut for_self = effect.clone();
        for_self.target = Some("self".into());
        for_self.set_action_by(Some("self".into())); // self makes the choice
        self.in_flight.spawn_context.target = Some("self".to_string());

        let had_choice_before = self.awaiting.choice.is_some();
        // Propagated, not discarded. This half used to be `let _ = …`, so a failure
        // on the SELF side vanished and the dispatch carried on to the opponent and
        // reported the whole effect as handled. A 「両者の…」 effect whose own side
        // could not resolve therefore looked like it had resolved both sides.
        // `execute_effect` signals "needs a choice" by setting `awaiting.choice` and
        // returning Ok, so an Err here is a genuine failure and not control flow.
        self.execute_effect(gs, &for_self)?;

        // If self created a NEW pending choice, save opponent for later
        if self.awaiting.choice.is_some() && !had_choice_before {
            let mut for_opponent = effect.clone();
            for_opponent.target = Some("opponent".into());
            for_opponent.set_action_by(Some("opponent".into())); // opponent makes the choice
            // Preserve any existing pending commands (e.g. remaining sequential actions)
            let mut existing = gs.ability_queue.take_pending_actions();
            existing.push(for_opponent);
            gs.ability_queue.set_pending_actions(existing);
            return Ok(true);
        }

        // Execute for opponent
        let mut for_opponent = effect.clone();
        for_opponent.target = Some("opponent".into());
        for_opponent.set_action_by(Some("opponent".into())); // opponent makes the choice
        self.in_flight.spawn_context.target = Some("opponent".to_string());
        self.execute_effect(gs, &for_opponent)?;

        Ok(true)
    }

    /// Is this an explicit self target with nothing in it to widen the meaning?
    ///
    /// An explicit `self_target` with no source, card type, group, selection or
    /// all-selections qualifier: there is nothing in the effect to widen it
    /// beyond the card that triggered it, so "self" can only mean the
    /// activating card.
    pub(crate) fn is_broadly_self(
        effect: &AbilityEffect,
        is_self_target: bool,
        activating_card_id: Option<i16>,
    ) -> bool {
        is_self_target
            && effect.target_name_player() == Some(TargetPlayer::Self_)
            && activating_card_id.is_some()
            && effect.source_any().is_none()
            && effect.card_type_any().is_none()
            && !effect.target_from_selection_any().unwrap_or(false)
            && !effect.multiple_targets_any().unwrap_or(false)
            && effect.group_names_any().is_none_or(|g| g.is_empty())
            && !effect.all_any().unwrap_or(false)
    }

    /// The single card a heart gain applies to when the effect resolved to no
    /// explicit target list.
    ///
    /// A stated position names the stage slot and always wins — if that slot is
    /// empty, the effect applies to nothing rather than quietly falling back to
    /// the activating card. Otherwise an unqualified self target means the
    /// activating card: either a no-count effect that does not exclude self, or
    /// an explicit `self_target` with nothing to widen it.
    pub(crate) fn implicit_heart_target(
        gs: &mut GameState,
        effect: &AbilityEffect,
        is_self_target: bool,
        activating_card_id: Option<i16>,
    ) -> Option<i16> {
        if effect.position_any().is_some() {
            let pos = effect
                .position_any()
                .as_ref()
                .and_then(|p| p.get_position())?;
            let stage_idx = util::stage_position_index(pos)?;
            let card_id = gs.resolve_target_player_mut(effect.target_name()).stage.stage
                [stage_idx];
            return (card_id != -1).then_some(card_id);
        }
        let unqualified_self = effect.target_count_any().is_none()
            && (effect.exclude_self_any().is_none()
                || effect.target_player() == Some(TargetPlayer::Self_));
        (unqualified_self
            || Self::is_broadly_self(effect, is_self_target, activating_card_id))
        .then_some(activating_card_id)
        .flatten()
    }

    /// When target_count is set and there are more eligible stage members
    /// than the count allows, park a saved copy of this effect and prompt the
    /// player to pick which members receive it. Returns Ok(true) when a
    /// choice was created (caller must stop processing this effect now).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn try_create_target_selection_choice(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        kind: ResourceKind,
        resource: &str,
        count: u8,
        target: &str,
        is_self_target: bool,
        per_unit: bool,
        exclude_self_id: Option<i16>,
        preceding_moved: &Option<SmallVec<[i16; 4]>>,
        all_selected: &[i16],
        selected_for_current: &[i16],
    ) -> Result<bool, String> {
        if effect.target_count_any().is_none()
            || is_self_target
            || per_unit
            || kind == ResourceKind::Other
            || !(selected_for_current.is_empty() || effect.distinct_any().is_some())
        {
            return Ok(false);
        }
        let card_db = self.card_db();
        let stage_ids: Vec<i16> = {
            let p = gs.resolve_target_player(target);
            p.stage
                .stage
                .iter()
                .copied()
                .filter(|&id| id != -1)
                .collect()
        };
        let mut prelim_filter = effect.filter_subset();
        prelim_filter.exclude_self = exclude_self_id;
        let exclude_names: Vec<String> = effect
            .exclude_by_name_source_any()
            .filter(|&s| s == "preceding_moved")
            .and(preceding_moved.as_ref())
            .map(|moved| {
                moved
                    .iter()
                    .filter_map(|&cid| card_db.get_card(cid).map(|c| c.name.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        if !exclude_names.is_empty() {
            prelim_filter.exclude_names = Some(&exclude_names);
        }
        let choice_exclude = if all_selected.is_empty() {
            None
        } else {
            Some(all_selected)
        };
        let mut candidates = util::matching_ids_filtered(
            &stage_ids,
            &card_db,
            &prelim_filter,
            true,
            None,
            effect.distinct_any(),
            choice_exclude,
        );
        // "ブレードをNつ以上持つ" (no 元々) — filter by CURRENT blade total
        // (base or set + modifiers); matches() only handles printed values.
        if prelim_filter.has_current_blade_filter() {
            candidates = util::filter_current_blade(
                candidates,
                gs,
                prelim_filter.current_blade_limit,
                prelim_filter.current_blade_operator,
            );
        }
        // Filter target_count candidates by position if specified.
        if let Some(pos) = effect.position_any() {
            if let Some(p) = pos.get_position() {
                if let Some(stage_idx) = util::stage_position_index(p) {
                    let p = gs.resolve_target_player(target);
                    let expected = p.stage.stage.get(stage_idx).copied().unwrap_or(-1);
                    candidates.retain(|cid| *cid == expected);
                }
            }
        }
        // Filter candidates by group_reference: "same_group_name" — use the
        // cost-discarded card's group (c.group, card position ②) so the target
        // prompt shows only members matching that group name.
        if effect.group_reference_any() == Some("same_group_name") {
            let ref_group: Option<String> = self
                .selection.moved_cards
                .first()
                .and_then(|cid| gs.card_database.get_card(*cid))
                .map(|c| c.group.to_string());
            if let Some(ref group) = ref_group {
                candidates.retain(|cid| {
                    util::card_matches_group_str(&gs.card_database, *cid, Some(group.as_str()))
                });
            }
        }
        if effect.same_name_any().unwrap_or(false) {
            let ref_names: Vec<String> = self
                .selection.moved_cards
                .iter()
                .filter_map(|&cid| card_db.get_card(cid).map(|c| c.name.to_string()))
                .collect();
            log::debug!(
                "[SAME_NAME_CHOICE] ref_names={:?} before={}",
                ref_names,
                candidates.len()
            );
            if ref_names.is_empty() {
                candidates.clear();
            } else {
                candidates.retain(|cid| {
                    card_db
                        .get_card(*cid)
                        .map(|card| ref_names.contains(&card.name.to_string()))
                        .unwrap_or(false)
                });
            }
            log::debug!(
                "[SAME_NAME_CHOICE] after={} candidates={:?}",
                candidates.len(),
                candidates
            );
        }
        let tc = effect.target_count_any().unwrap_or(1) as usize;
        if candidates.len() <= tc {
            return Ok(false);
        }
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
        self.selection.count_at_save = Some(self.selection.cards.len().u8_count());
        let mut pending = gs.ability_queue.take_pending_actions();
        pending.insert(0, saved);
        gs.ability_queue.set_pending_actions(pending);
        let desc_en = format!("Select {} {} to receive {} {}", tc, util::card_plural(tc), count, resource);
        let resource_label =
            crate::ability::describe::resource_label_ja(Some(resource));
        let desc_ja = format!(
            "リソースを受け取る{}枚のカードを選択（{} {}）",
            tc, count, resource_label
        );
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
        // Don't call store_pending_choice — keep self.pending_choice set
        // so the caller (e.g. resume_pending_commands) can detect the
        // sub-choice and properly save remaining commands before returning.
        self.in_flight.sub_choice_created = true;
        Ok(true)
    }
}