//! `change_state` - put a member active or wait.
//!
//! Home of `ActionType::ChangeState`: 59 of the 936 unique abilities in
//! `cards/abilities.json` (`docs/ABILITY_MATRIX.md`, "change active/wait
//! state"). `resolve_blade_limit` and the per-unit scaling it needs travel
//! with it because nothing else derives them.
use crate::core::constants::CountCast;
use super::super::enums::{TargetPlayer, Zone};
use super::super::resolver::AbilityResolver;
use super::super::types::{Choice, ChoiceRoute, ExecutionContext};
use super::super::util;

use crate::card::{AbilityEffect};
use crate::core::game_modifiers::CardOrientation;
use crate::game_state::GameState;


use crate::HashMap;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};


impl AbilityResolver {

    /// How many times a 「〜1枚につき」 state change fires.
    ///
    /// A `previous_moved` source counts the cards the PRECEDING sequential step
    /// moved rather than a zone. Anything else counts the zone's cards through
    /// the effect's own filter, with the card type dropped — a state change
    /// applies to whatever is there — and the cost limit kept.
    fn per_unit_state_count(
        &self,
        gs: &GameState,
        effect: &AbilityEffect,
        target: &str,
        cost_limit: Option<u8>,
        count: u8,
    ) -> u8 {
        let per_unit_cnt = effect.per_unit_count_any().unwrap_or(1);
        if effect
            .per_unit_source_any()
            .is_some_and(|s| s.contains("previous_moved"))
        {
            return (self.selection.moved_cards.len().u8_count() / per_unit_cnt) * count.max(1);
        }
        let player = gs.resolve_target_player(target);
        let location = effect
            .location_any()
            .unwrap_or(Zone::Stage.to_str());
        let cards: Vec<i16> = util::zone_cards(player, location).to_vec();
        let mut per_unit_filter = util::CardFilter::from_effect(effect);
        per_unit_filter.card_type = None;
        per_unit_filter.cost_limit = cost_limit;
        let matching: Vec<i16> = cards
            .iter()
            .filter(|&&cid| per_unit_filter.matches(&gs.card_database, cid, false))
            .copied()
            .collect();
        let matched_count = if matches!(
            effect.distinct_any(),
            Some(crate::card::DistinctType::CardName)
        ) {
            // Joint-aware distinct-name count (Q278/Q279): ordinary cards dedupe by
            // name; a joint (multi-name) card adds one unit if it introduces a name
            // not already present as a single-name card.
            util::count_distinct_member_name_units(&matching, &gs.card_database).u8_count()
        } else {
            util::apply_distinct_filter(&matching, effect.distinct_any(), &gs.card_database)
                .len()
                .u8_count()
        };
        (matched_count / per_unit_cnt) * count.max(1)
    }

    /// The blade limit, and whether it can match nothing.
    ///
    /// Two limits are computed rather than read. Q266 derives one from the
    /// member paid as the wait cost; C5 from the energy under the activating
    /// member. Q266 can come out negative, which no card can satisfy — and
    /// clamping it to zero would let every member qualify, so that case is
    /// reported for the caller to require "< 0", which nothing meets.
    fn resolve_blade_limit(gs: &GameState, effect: &AbilityEffect) -> (Option<u8>, bool) {
        if effect.blade_limit_from_cost_member_any().unwrap_or(false) {
            // Q266: dynamic limit = (original blade of the member paid as the
            // wait cost) − offset. 「元々持つブレードの数が…より2つ以上少ない」
            // → limit = costed_blade − 2.
            let base = effect.blade_limit_offset_any().unwrap_or(0) as i32;
            let cost_member_blade = gs
                .last_cost_wait_member()
                .and_then(|cid| gs.card_database.get_card(cid).map(|c| c.blade))
                .unwrap_or(0) as i32;
            let signed = cost_member_blade - base;
            return if signed < 0 {
                (Some(0), true)
            } else {
                (Some(signed.u8_count()), false)
            };
        }
        if effect.blade_limit_from_energy_under_any().unwrap_or(false) {
            // C5: dynamic limit = (energy cards under the activating member) + offset.
            let base = effect.blade_limit_offset_any().unwrap_or(0) as i32;
            let under_count = gs.activating_card.map_or(0, |aid| {
                let p = gs.resolve_target_player("self");
                p.stage
                    .stage
                    .iter()
                    .position(|&id| id == aid)
                    .map(|idx| p.stage.under_cards[idx].len().i32_count())
                    .unwrap_or(0)
            });
            return (Some(crate::constants::saturate_u8(under_count + base)), false);
        }
        (effect.blade_limit_any(), false)
    }

    pub(crate) fn execute_change_state(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let state_change = effect.state_change_any().unwrap_or("").to_string();
        let target = effect.target_name().to_string();
        let card_type = effect.card_type_any().map(|ct| ct.as_card_str());
        let card_type_filter = card_type.map(|s| s.to_string());
        let cost_limit = if effect.cost_from_revealed_any().unwrap_or(false) {
            gs.revealed_cards
                .first()
                .and_then(|&cid| gs.card_database.get_card(cid))
                .and_then(|c| c.cost)
        } else {
            effect.cost_limit_any()
        };
        // Per-unit count derivation (1x per N matching cards).
        let mut count: u8 = effect.count_or(0);
        let mut group_name = effect.group_name();
        if effect.per_unit_any().unwrap_or(false) {
            count = self.per_unit_state_count(gs, effect, &target, cost_limit, count);
            group_name = None;
        }
        if effect.per_unit_any().unwrap_or(false) && count == 0 {
            return Ok(());
        }
        let max = effect.max.unwrap_or(false);
        let optional = effect.optional.unwrap_or(false);
        let self_cost = effect.self_cost_any().unwrap_or(false);
        let source = effect.source_any();
        let destination = effect.destination.map(|z| z.as_str());
        let cost_limit_operator = effect.cost_limit_operator_any().map(|s| s.to_string());
        let characters = effect.characters_any();
        let (blade_limit, blade_limit_unreachable) = Self::resolve_blade_limit(gs, effect);
        let blade_limit_operator_binding = effect.blade_limit_operator_any();
        let mut blade_limit_operator = blade_limit_operator_binding.as_deref();
        if blade_limit_unreachable {
            blade_limit_operator = Some("<");
        }
        // When targeting opponent, group_names is trigger-level metadata
        // (from the wrapper's condition), not an effect filter.
        let group_filter = if effect.target_name_player() == Some(TargetPlayer::Opponent) {
            None
        } else {
            group_name.map(|s| s.to_string())
        };

        if optional {
            let decided = gs
                .ability_queue
                .current_entry()
                .and_then(|e| e.optional_cost_result);
            if decided.is_none() {
                // Only offer the optional choice if there's at least one valid target.
                // For state_change="active", the member must be in "wait" state.
                // For state_change="wait", any member works.
                // If no valid targets exist, return early without creating the choice.
                let can_target = if state_change == "active" {
                    let p = gs.resolve_target_player(&target);
                    let ct = card_type_filter.as_deref();
                    let gf = group_filter.as_deref();
                    p.stage.stage.iter().any(|&cid| {
                        if cid == -1 {
                            return false;
                        }
                        let is_wait = gs
                            .mods
                            .get_orientation_modifier(cid)
                            .is_some_and(|o| o == "wait");
                        if !is_wait {
                            return false;
                        }
                        if let Some(t) = ct {
                            if !util::card_matches_type(&gs.card_database, cid, Some(t)) {
                                return false;
                            }
                        }
                        if let Some(g) = gf {
                            if !util::card_matches_group_str(&gs.card_database, cid, Some(g)) {
                                return false;
                            }
                        }
                        true
                    })
                } else if state_change == "wait" && effect.state_any() == Some("active")
                {
                    // wait effect targeting only active members:
                    // check if there is at least one active (non-wait) member
                    let p = gs.resolve_target_player(&target);
                    let ct = card_type_filter.as_deref();
                    let gf = group_filter.as_deref();
                    p.stage.stage.iter().any(|&cid| {
                        if cid == -1 {
                            return false;
                        }
                        // member is active when there is no "wait" orientation modifier
                        let is_active = gs
                            .mods
                            .get_orientation_modifier(cid)
                            .is_none_or(|o| o != "wait");
                        if !is_active {
                            return false;
                        }
                        if let Some(t) = ct {
                            if !util::card_matches_type(&gs.card_database, cid, Some(t)) {
                                return false;
                            }
                        }
                        if let Some(g) = gf {
                            if !util::card_matches_group_str(&gs.card_database, cid, Some(g)) {
                                return false;
                            }
                        }
                        true
                    })
                } else if state_change == "wait" {
                    // Q137: 「ウェイトにする」とは、アクティブ状態のメンバーをウェイト状態に
                    // することを意味します。既にウェイト状態のメンバーは対象外です。
                    let p = gs.resolve_target_player(&target);
                    let ct = card_type_filter.as_deref();
                    let gf = group_filter.as_deref();
                    p.stage.stage.iter().any(|&cid| {
                        if cid == -1 {
                            return false;
                        }
                        let is_active = gs
                            .mods
                            .get_orientation_modifier(cid)
                            .is_none_or(|o| o != "wait");
                        if !is_active {
                            return false;
                        }
                        if let Some(t) = ct {
                            if !util::card_matches_type(&gs.card_database, cid, Some(t)) {
                                return false;
                            }
                        }
                        if let Some(g) = gf {
                            if !util::card_matches_group_str(&gs.card_database, cid, Some(g)) {
                                return false;
                            }
                        }
                        true
                    })
                } else {
                    true // no state filter for non-wait: any member is a valid target
                };
                if !can_target {
                    log::debug!(
                        "[EXEC_CHANGE_STATE] optional {} but no valid targets — skipping",
                        state_change
                    );
                    return Ok(());
                }
                self.emit_pay_skip_gate(
                    gs,
                    Some(ChoiceRoute::ChangeState),
                    format!("Change state to {} (pay optional cost)?", state_change),
                    format!("状態を{}に変更（オプションコスト）？", state_change),
                    optional,
                    None,
                );
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.choice_card_no = Some(ChoiceRoute::ChangeState);
                }
                return Ok(());
            } // end if decided.is_none
        }

        // Draw from energy deck and place in energy zone with state (e.g. wait)
        if Zone::from_str(source.unwrap_or("")) == Some(Zone::Deck)
            && Zone::from_str(destination.unwrap_or("")) == Some(Zone::Energy)
        {
            self.execute_energy_placement(gs, &state_change, &target, count);
            return Ok(());
        }

        // Member card state change — operate on stage
        let is_member_op = card_type_filter.as_deref() == Some("member_card") || self_cost;

        if is_member_op {
            log::debug!(
                "[EXEC_CHANGE_STATE] member_op: target={} count={} max={} state_change={}",
                target,
                count,
                max,
                state_change
            );

            // Check cannot_activate_by_effect restriction before mutable borrow.
            let is_cannot_activate_by_effect = if state_change == "active" {
                let target_player = gs.resolve_target_player(&target);
                gs.cannot_activate_members
                    .iter()
                    .any(|x| x.as_str() == target_player.id.as_str())
            } else {
                false
            };

            let exclude_self_id = if effect.exclude_self_any().unwrap_or(false) {
                gs.activating_card
            } else {
                None
            };

            let card_db = self.card_db();
            let player = gs.resolve_target_player_mut(&target);

            let filter = crate::ability::util::CardFilter {
                card_type: card_type_filter.as_deref(),
                group: group_filter.as_deref(),
                cost_limit,
                cost_operator: cost_limit_operator.as_deref(),
                characters,
                exclude_self: exclude_self_id,
                ..Default::default()
            };
            let filter = filter
                .original_blade_limit(blade_limit, blade_limit_operator)
                // 「元々持つハートの数がNつ以下」 — e.g. PL!-pb2-006 西木野真姫
                // 「相手のステージにいる元々持つハートの数が1つ以下のメンバー1人を
                // ウェイトにする」. Without this the clause degrades to "any member"
                // and the selection offers over-budget members too.
                .original_heart_total(
                    effect.heart_total_limit_any(),
                    effect.heart_total_operator_any().or(Some("<=")),
                );
            let mut candidates: Vec<(usize, i16)> = Vec::new();

            // If we have selected cards from a previous choice, use them
            if !self.selection.cards.is_empty() {
                for &card_id in &self.selection.cards {
                    if let Some(pos) = player.stage.stage.iter().position(|&id| id == card_id) {
                        candidates.push((pos, card_id));
                    }
                }
                // A prior step's selection that left NO card on the target
                // stage cannot be the object of a stage rest (waiting requires
                // the stage). Fall through to the stage scan so stage-scoped
                // follow-up clauses like 「（そうした場合、）相手のステージにいる
                // 元々…ブレード3つ以下のメンバー1人をウェイトにする」
                // (百生吟子 PL!HS-PR-035-PR, whose select/move steps populate
                // selected_cards from the DISCARD) still resolve correctly.
            if candidates.is_empty() {
                    log::debug!(
                        "[EXEC_CHANGE_STATE] prior selection {:?} has no card on the target stage; scanning the stage instead",
                        self.selection.cards
                    );
                }
            }
            if candidates.is_empty() {
                // Collect all potential candidates (filter by card_type, group, etc.)
                // in a first pass, then filter by orientation in a second pass
                // to avoid borrow conflicts with gs.mods.
                let stage_snapshot: Vec<(usize, i16)> = player
                    .stage
                    .stage
                    .iter()
                    .copied()
                    .enumerate()
                    .filter(|(_, id)| *id != -1 && filter.matches(&card_db, *id, false))
                    .collect();
                let _ = card_db;
                let _ = player;
                for (i, card_id) in &stage_snapshot {
                    // State-based candidate filtering:
                    //   wait→active (state_change=="active"): only "wait" members.
                    //   active→wait with state=="active" filter: only active members.
                    //   Everything else: accept any member.
                    let matches_state = if state_change == "active" {
                        let ori = gs.mods.get_orientation_modifier(*card_id);
                        ori.is_some_and(|o| o == "wait")
                    } else if effect.state_any() == Some("active") {
                        // e.g. "アクティブ状態のメンバーをウェイトにする"
                        // Only members currently in active state (no wait modifier).
                        let ori = gs.mods.get_orientation_modifier(*card_id);
                        ori.is_none_or(|o| o != "wait")
                    } else if state_change == "wait" {
                        // For "wait" state change, exclude cards already in wait state
                        // so previously waited targets don't remain selectable.
                        let ori = gs.mods.get_orientation_modifier(*card_id);
                        ori.is_none_or(|o| o != "wait")
                    } else {
                        true
                    };
                    if matches_state {
                        candidates.push((*i, *card_id));
                    }
                }
            }

            // When self_cost or self_target explicitly restricts to "this member"
            // (e.g. "このメンバーをウェイトにする"), filter candidates to only the
            // activating card. If the card is already in the target state, skip.
            if (self_cost || effect.is_self_target())
                && self.selection.cards.is_empty()
            {
                if let Some(act_id) = gs.activating_card {
                    if candidates.iter().any(|(_, cid)| *cid == act_id) {
                        candidates.retain(|(_, cid)| *cid == act_id);
                    } else {
                        let player = gs.resolve_target_player(&target);
                        let on_stage = player.stage.stage.contains(&act_id);
                        if !on_stage {
                            log::debug!(
                                "[EXEC_CHANGE_STATE] self_cost: activating card not on stage"
                            );
                            return Ok(());
                        }
                        let ori = gs.mods.get_orientation_modifier(act_id);
                        let already_target = match state_change.as_str() {
                            "wait" => ori.is_some_and(|o| o == "wait"),
                            "active" => ori.is_none_or(|o| o != "wait"),
                            _ => false,
                        };
                        if already_target {
                            log::debug!(
                                "[EXEC_CHANGE_STATE] self_cost: already {}, skipping",
                                state_change
                            );
                            return Ok(());
                        }
                        // Should be in candidates but wasn't — add it back
                        if let Some(pos) = player.stage.stage.iter().position(|&id| id == act_id) {
                            candidates.push((pos, act_id));
                        }
                    }
                }
            }

            // Q275: when an effect makes the TARGET player select members of their OWN
            // stage to be waited (e.g. セラス "action_by: opponent" — "相手は、自分の
            // ステージのアクティブなメンバーをウェイトにする"), a member that is
            // wait-immune against the effect's controller is NOT a legal choice. Exclude
            // it from the offered candidates so the sacrificing player must pick a
            // waitable member. This is the inverse of Q274 (opponent freely picking a
            // victim): there `action_by` is self and the member stays selectable, with
            // the wait suppressed only at application time below.
            if state_change == "wait"
                && matches!(effect.action_by_any(), Some("opponent"))
            {
                let controller = gs.ability_master_id();
                candidates.retain(|(_, cid)| {
                    !gs.wait_immune_members.iter().any(|(m, owner)| {
                        *m == *cid && controller.is_some_and(|c| c != owner)
                    })
                });
            }

            if candidates.is_empty() {
                // Energy state change (parser emits card_type="energy_card"
                // when エネルギー is the object; mixed either/or steps are
                // split into choice options upstream).
                let declares_energy = effect.card_type_any() == Some(&crate::card::CardType::Energy);
                if declares_energy {
                    if let Err(e) = self.execute_energy_state_change(
                        gs,
                        effect,
                        &state_change,
                        &target,
                        count,
                        max,
                        Some("energy_card"),
                        None,
                    ) {
                        log::debug!("Failed to change energy state: {}", e);
                    }
                }
                return Ok(());
            }

            // count=0 means "change all matching" (no limit)
            let is_change_all = count == 0;

            // Prompt when: there are candidates to choose from AND we haven't already selected,
            // AND either max allows subset selection or more candidates than count need narrowing.
            // Exception: if the effect targets "this member" (activating card is among candidates)
            // and it's a single-target self-member effect, auto-select instead of prompting.
            let is_self_target = count == 1
                && target.as_str() != "opponent"
                && card_type_filter.as_deref() == Some("member_card")
                && gs
                    .activating_card
                    .is_some_and(|act_id| candidates.iter().any(|(_, cid)| *cid == act_id));
            let needs_prompt = !is_self_target
                && self.selection.cards.is_empty()
                && ((max && !candidates.is_empty())
                    || (!is_change_all && candidates.len() > count as usize));

            if needs_prompt {
                let allow_skip = max;
                let pick_count = usize::from(count);
                let desc = if max {
                    format!("Select up to {} member(s) to change state", count)
                } else {
                    format!("Select {} member(s) to change state", count)
                };
                // Map candidate positions to stage indices for filtered_indices
                let candidate_positions: Vec<usize> =
                    candidates.iter().map(|(pos, _)| *pos).collect();
                let state_label =
                    crate::ability::describe::state_verb_ja(Some(state_change.as_str()));
                let desc_ja = format!("{}に変更するメンバーを{}体選択", state_label, pick_count);
                self.awaiting.choice = Some(
                    Choice::select_cards(
                        Zone::Stage.to_str(),
                        pick_count,
                        desc.clone(),
                        allow_skip,
                    )
                    .description_ja(Some(desc_ja))
                    .card_type(card_type_filter.clone())
                    .cost_limit(cost_limit, cost_limit_operator.clone())
                    .group(group_filter.clone())
                    .characters(characters.cloned())
                    .filtered_indices(Some(candidate_positions))
                    .is_select_action(true)
                    .target_player_id(Some(target.clone()))
                    .build(),
                );
                self.awaiting.stage_select_intent =
                    Some(crate::ability::types::StageSelectIntent::ChangeStateWait);
                self.in_flight.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                // Store a re-apply effect so finalize_choice applies the state
                // change to the selected target after the choice is resolved.
                gs.ability_queue
                    .set_pending_actions(vec![crate::card::AbilityEffect {
                        action: crate::ability::enums::ActionType::ChangeState,
                        target: Some(target.clone().into()),
                        count: Some(count),
                        kind: effect.kind.clone(),
                        ..Default::default()
                    }]);
                return Ok(());
            }

            let change_count = if is_change_all {                candidates.len()
            } else {
                count.min(candidates.len().u8_count()) as usize
            };

            let actual_targets: Vec<_> = if is_self_target {
                if let Some(act_id) = gs.activating_card {
                    candidates
                        .iter()
                        .filter(|(_, cid)| *cid == act_id)
                        .take(change_count)
                        .collect()
                } else {
                    candidates.iter().take(change_count).collect()
                }
            } else {
                candidates.iter().take(change_count).collect()
            };
            // Wait-immunity ("相手の効果によってはウェイトしない"): drop members that
            // are recorded as wait-immune against the current effect's controller.
            let actual_targets: Vec<_> = if state_change == "wait" {
                let controller = gs.ability_master_id();
                actual_targets
                    .into_iter()
                    .filter(|(_, cid)| {
                        !gs.wait_immune_members.iter().any(|(m, owner)| {
                            *m == *cid && controller.is_some_and(|c| c != owner)
                        })
                    })
                    .collect()
            } else {
                actual_targets
            };

            log::debug!(
                "[EXEC_CHANGE_STATE] targets={:?} state_change={}",
                actual_targets
                    .iter()
                    .map(|(_, cid)| cid)
                    .collect::<Vec<_>>(),
                state_change
            );

            // Snapshot orientations BEFORE applying any changes (for active→wait
            // and wait→active transition detection).
            let snapshots: HashMap<i16, Option<CardOrientation>> = actual_targets
                .iter()
                .map(|(_, card_id)| {
                    (
                        *card_id,
                        gs.mods.orientation_modifiers.get(card_id).copied(),
                    )
                })
                .collect();
            gs.state_snapshot_before_change = Some(snapshots);

            // Count how many are in wait state before changing (for wait→active tracking)
            let wait_before_count = actual_targets
                .iter()
                .filter(|(_, card_id)| {
                    let o = gs.mods.get_orientation_modifier(*card_id);
                    // None = active (no modifier), Some("wait") = wait
                    o.is_some_and(|o| o == "wait")
                })
                .count();

            for (_, card_id) in &actual_targets {
                if is_cannot_activate_by_effect {
                    log::debug!(
                        "[EXEC_CHANGE_STATE] blocked by cannot_activate_by_effect: card_id={}",
                        card_id
                    );
                    continue;
                }
                log::debug!(
                    "[EXEC_CHANGE_STATE] applying: card_id={} state={} before_ori={:?}",
                    card_id,
                    state_change,
                    gs.mods.get_orientation_modifier(*card_id)
                );
                gs.mods.add_orientation_modifier(*card_id, &state_change);
                log::debug!(
                    "[EXEC_CHANGE_STATE] after: card_id={} ori={:?}",
                    card_id,
                    gs.mods.get_orientation_modifier(*card_id)
                );
                // Record wait -> active so the per-turn history clause can see it:
                // 「このターン中に…ウェイト状態からアクティブにしていたメンバー」. A
                // card already in the list is left alone, so the history is a SET and
                // not a tally — activating a card twice in one turn still counts once.
                if state_change == "active" && !gs.cards_activated_this_turn.contains(card_id) {
                    gs.cards_activated_this_turn.push(*card_id);
                }
            }

            // Conditional constants read wait/active orientations, so they must be
            // re-evaluated as soon as the orientation change is applied — otherwise
            // a card whose own effect satisfies its 「~が2人以上いる」 常時 stays stale.
            log::debug!(
                "[EXEC_CHANGE_STATE] recalculating constants after state={}",
                state_change
            );
            gs.recalculate_constants();

            // Push changed cards to selected_cards so subsequent sequential
            // actions (e.g. gain_resource with target_from_selection: true)
            // can target the affected member(s).
            for (_, card_id) in &actual_targets {
                log::debug!(
                    "[EXEC_CHANGE_STATE] pushing card_id={} to selected_cards (len={})",
                    card_id,
                    self.selection.cards.len()
                );
                if !self.selection.cards.contains(card_id) {
                    self.selection.cards.push(*card_id);
                }
            }
            // Record the members this step changed so a following delayed
            // restriction step ("そのメンバーは次のターンのアクティブフェイズに
            // アクティブしない") can key its flags on exactly these victims.
            self.selection.changed_state_members = actual_targets.iter().map(|(_, cid)| *cid).collect();

            // Track how many members were actually changed from wait→active
            // (activations blocked by cannot_activate_by_effect don't count)
            if state_change == "active" {
                let actual_count = if is_cannot_activate_by_effect {
                    0
                } else {
                    wait_before_count.u8_count()
                };
                gs.last_state_change_wait_to_active_count = actual_count;
            }

            // Compare snapshot with current state to detect actual transitions.
            if let Some(before) = gs.state_snapshot_before_change.take() {
                // Attribute the transitions to the effect's owner for
                // 自分のカードの効果 scoping: activating card's owner, else
                // the queue entry player, else unknown ("").
                let cause_pid: String = gs
                    .activating_card
                    .and_then(|cid| {
                        if gs.player1.contains_card(cid) {
                            Some(gs.player1.id.to_string())
                        } else if gs.player2.contains_card(cid) {
                            Some(gs.player2.id.to_string())
                        } else {
                            None
                        }
                    })
                    .or_else(|| {
                        gs.ability_queue
                            .current_entry()
                            .map(|e| e.player_id.to_string())
                    })
                    .unwrap_or_default();
                for (card_id, before_ori) in &before {
                    let after_ori = gs.mods.orientation_modifiers.get(card_id).copied();
                    if *before_ori != after_ori {
                        let from_str = before_ori
                            .unwrap_or(CardOrientation::Active)
                            .as_str()
                            .to_string();
                        let to_str = after_ori
                            .unwrap_or(CardOrientation::Active)
                            .as_str()
                            .to_string();
                        gs.recently_state_changed.push((
                            *card_id,
                            from_str.clone(),
                            to_str.clone(),
                            cause_pid.clone(),
                        ));
                        // Turn-scoped attributed log (cleared only at the turn
                        // boundary) — consumed by temporal state-change
                        // conditions like Q203.
                        gs.turn_state_changes.push((
                            gs.activating_card.unwrap_or(-1),
                            *card_id,
                            from_str.clone(),
                            to_str.clone(),
                        ));
                        log::debug!(
                            "[STATE_CHANGE] detected: card={} {}→{}",
                            card_id,
                            from_str,
                            to_str
                        );
                    }
                }
            }

            // Re-trigger auto abilities for both players — a member's state
            // change may satisfy state_change_condition on an auto ability
            // (e.g. "when opponent cost ≤4 member is waited → draw").
            log::debug!(
                "[STATE_CHANGE] modifier applied, re-triggering auto abilities (state={})",
                state_change
            );
            let p1 = gs.player1.id.clone();
            let p2 = gs.player2.id.clone();
            gs.trigger_auto_abilities_for_player(&p1);
            gs.trigger_auto_abilities_for_player(&p2);

            if crate::core::runtime_flags::logging_enabled() {
                let pp = self.player_prefix(gs);
                let act_name = gs
                    .activating_card
                    .map(|c| self.card_name(c))
                    .unwrap_or_default();
                gs.push_rule_log_fmt(format_args!(
                    "{} {}: 状態変更→{}",
                    pp, act_name, state_change
                ));
            }
            return Ok(());
        }

        // Energy card state change (original behavior) — delegated
        self.execute_energy_state_change(
            gs,
            effect,
            &state_change,
            &target,
            count,
            max,
            card_type_filter.as_deref(),
            group_filter.as_deref(),
        )
    }
}