use crate::core::constants::U8Count;
use super::super::enums::{TargetPlayer, Zone};
use super::super::resolver::AbilityResolver;
use super::super::types::{Choice, ChoiceRoute, ExecutionContext};
use super::super::util;
use crate::ability_queue::ConditionalChoice;
use crate::card::AbilityEffect;
use crate::core::game_modifiers::CardOrientation;
use crate::game_state::GameState;
use smallvec::SmallVec;

use crate::HashMap;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

impl AbilityResolver {
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
        let mut count: u8 = effect.count_or(0) as u8;
        let mut group_name = effect.group_name();
        if effect.per_unit_any().unwrap_or(false) {
            // "これによりデッキに置いたカード1枚につき" — count against the
            // cards the PRECEDING sequential step moved, not a zone.
            if effect
                .per_unit_source_any()
                .is_some_and(|s| s.contains("previous_moved"))
            {
                let per_unit_cnt = effect.per_unit_count_any().unwrap_or(1) as u8;
                count = (self.moved_cards.len().u8_count() / per_unit_cnt) * count.max(1);
            } else {
                let player = gs.resolve_target_player(&target);
                let loc_binding = effect.location_any();
                let location = loc_binding.unwrap_or(Zone::Stage.to_str());
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
                    util::count_distinct_member_name_units(&matching, &gs.card_database) as u8
                } else {
                    util::apply_distinct_filter(&matching, effect.distinct_any(), &gs.card_database)
                        .len().u8_count()
                };
                let per_unit_cnt = effect.per_unit_count_any().unwrap_or(1) as u8;
                count = (matched_count / per_unit_cnt) * count.max(1);
            }
            group_name = None;
        }
        let max = effect.max.unwrap_or(false);
        let optional = effect.optional.unwrap_or(false);
        let self_cost = effect.self_cost_any().unwrap_or(false);
        let source = effect.source_any();
        let destination = effect.destination.map(|z| z.as_str());
        let cost_limit_operator = effect.cost_limit_operator_any().map(|s| s.to_string());
        let characters = effect.characters_any();
        let mut q266_no_target = false;
        let blade_limit: Option<u8> = if effect.blade_limit_from_cost_member_any().unwrap_or(false) {
            // Q266: dynamic limit = (original blade of the member paid as the wait cost) − offset.
            // "元々持つブレードの数が…より2つ以上少ない" → limit = costed_blade − 2.
            let base = effect.blade_limit_offset_any().unwrap_or(0) as i32;
            let cost_member_blade = gs
                .last_cost_wait_member()
                .and_then(|cid| gs.card_database.get_card(cid).map(|c| c.blade))
                .unwrap_or(0) as i32;
            let signed = cost_member_blade - base;
            if signed < 0 {
                // No member can have negative blades → no legal target. Encode as
                // "< 0" (matches nothing), since blades are >= 0.
                q266_no_target = true;
                Some(0)
            } else {
                Some(signed as u8)
            }
        } else if effect.blade_limit_from_energy_under_any().unwrap_or(false) {
            // C5: dynamic limit = (energy cards under the activating member) + offset.
            let base = effect.blade_limit_offset_any().unwrap_or(0) as i32;
            let under_count = gs.activating_card.map_or(0, |aid| {
                let p = gs.resolve_target_player("self");
                p.stage
                    .stage
                    .iter()
                    .position(|&id| id == aid)
                    .map(|idx| p.stage.under_cards[idx].len() as i32)
                    .unwrap_or(0)
            });
            Some(crate::constants::saturate_u8(under_count + base))
        } else {
            effect.blade_limit_any().map(|v| v as u8)
        };
        let blade_limit_operator_binding = effect.blade_limit_operator_any();
        let mut blade_limit_operator = blade_limit_operator_binding.as_deref();
        if q266_no_target {
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
                } else if state_change == "wait" && effect.state_any().as_deref() == Some("active")
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
                gs.cannot_activate_members.contains(&target_player.id)
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

            let mut filter = crate::ability::util::CardFilter::default();
            filter.card_type = card_type_filter.as_deref();
            filter.group = group_filter.as_deref();
            filter.cost_limit = cost_limit;
            filter.cost_operator = cost_limit_operator.as_deref();
            filter.characters = characters;
            filter.exclude_self = exclude_self_id;
            let filter = filter.original_blade_limit(blade_limit, blade_limit_operator);
            let mut candidates: Vec<(usize, i16)> = Vec::new();

            // If we have selected cards from a previous choice, use them
            if !self.selected_cards.is_empty() {
                for &card_id in &self.selected_cards {
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
                        self.selected_cards
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
                    } else if effect.state_any().as_deref() == Some("active") {
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
                && self.selected_cards.is_empty()
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
                        *m == *cid && controller.as_deref().is_some_and(|c| c != owner)
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
                && self.selected_cards.is_empty()
                && ((max && !candidates.is_empty())
                    || (!is_change_all && candidates.len() > count as usize));

            if needs_prompt {
                let allow_skip = max;
                let pick_count = if max { count as usize } else { count as usize };
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
                self.pending_choice = Some(
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
                self.stage_select_intent =
                    Some(crate::ability::types::StageSelectIntent::ChangeStateWait);
                self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
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

            let change_count = if is_change_all {
                candidates.len()
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
                            *m == *cid && controller.as_deref().is_some_and(|c| c != owner)
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
            }

            // Push changed cards to selected_cards so subsequent sequential
            // actions (e.g. gain_resource with target_from_selection: true)
            // can target the affected member(s).
            for (_, card_id) in &actual_targets {
                log::debug!(
                    "[EXEC_CHANGE_STATE] pushing card_id={} to selected_cards (len={})",
                    card_id,
                    self.selected_cards.len()
                );
                if !self.selected_cards.contains(card_id) {
                    self.selected_cards.push(*card_id);
                }
            }
            // Record the members this step changed so a following delayed
            // restriction step ("そのメンバーは次のターンのアクティブフェイズに
            // アクティブしない") can key its flags on exactly these victims.
            self.changed_state_members = actual_targets.iter().map(|(_, cid)| *cid).collect();

            // Track how many members were actually changed from wait→active
            // (activations blocked by cannot_activate_by_effect don't count)
            if state_change == "active" {
                let actual_count = if is_cannot_activate_by_effect {
                    0
                } else {
                    wait_before_count as u8
                };
                gs.last_state_change_wait_to_active_count = actual_count;
            }

            // Compare snapshot with current state to detect actual transitions.
            if let Some(before) = gs.state_snapshot_before_change.take() {
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

            let pp = self.player_prefix(gs);
            let act_name = gs
                .activating_card
                .map(|c| self.card_name(c))
                .unwrap_or_default();
            gs.rule_log
                .push(format!("{} {}: 状態変更→{}", pp, act_name, state_change));
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

    /// Place energy from deck to energy zone with specific state (wait/active).
    pub(crate) fn execute_energy_placement(
        &mut self,
        gs: &mut GameState,
        state_change: &str,
        target: &str,
        count: u8,
    ) {
        let cause_cid = gs.activating_card;
        let mut placed_energy: Vec<i16> = Vec::new();
        let player_id = {
            let player = gs.resolve_target_player_mut(target);
            for _ in 0..count {
                if let Some(energy_id) = player.energy_deck.draw() {
                    player.energy_zone.cards.push(energy_id);
                    if state_change == "active" {
                        player.energy_zone.add_active(1);
                    }
                    placed_energy.push(energy_id);
                }
            }
            player.id.clone()
        };
        for &eid in &placed_energy {
            gs.push_movement_event_typed(
                eid,
                crate::core::types::ZoneId::EnergyDeck,
                crate::core::types::ZoneId::EnergyZone,
                cause_cid,
                &player_id,
                true,
            );
        }
    }

    /// Change the state of energy zone cards (wait/active).
    pub(crate) fn execute_energy_state_change(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        state_change: &str,
        target: &str,
        count: u8,
        max: bool,
        card_type_filter: Option<&str>,
        group_filter: Option<&str>,
    ) -> Result<(), String> {
        let card_db = self.card_db();
        let exclude_self_id = if effect.exclude_self_any().unwrap_or(false) {
            gs.activating_card
        } else {
            None
        };
        let (wait_cards, deactivate_count) = {
            let player = gs.resolve_target_player_mut(target);

            let mut filter = effect.filter_subset();
            if card_type_filter == Some("energy_card") {
                filter.group = None;
                filter.characters = None;
                filter.cost_limit = None;
                filter.cost_operator = None;
            } else {
                filter.group = group_filter;
            }
            filter.card_type = card_type_filter;
            filter.exclude_self = exclude_self_id;
            let valid_indices =
                util::matching_indices(&player.energy_zone.cards, &card_db, &filter, false);

            let effective_count = if max {
                let available = match state_change {
                    "active" | "アクティブ" => player
                        .energy_zone
                        .cards
                        .len()
                        .saturating_sub(player.energy_zone.active_count() as usize),
                    _ => player.energy_zone.active_count() as usize,
                };
                let capped = (count as usize).min(available) as u8;
                log::debug!(
                    "[ENERGY] max=true: count={} available={} effective={}",
                    count,
                    available,
                    capped
                );
                capped
            } else if count == 0 {
                let val = match state_change {
                    "active" | "アクティブ" => player
                        .energy_zone
                        .cards
                        .len()
                        .saturating_sub(player.energy_zone.active_count() as usize),
                    _ => player.energy_zone.active_count() as usize,
                };
                log::debug!("[ENERGY] count=0 (all): effective={}", val);
                val as u8
            } else {
                log::debug!("[ENERGY] max=false: count={} effectve={}", count, count);
                count
            };

            // Partial resolution (Q167: 「実行可能な限り解決する」): when the
            // zone holds fewer matching candidates than requested, resolve as
            // many as possible instead of aborting the whole effect. Legacy
            // behavior (candidates ≥ requested) is untouched.
            let effective_count = if valid_indices.len() < effective_count as usize {
                let capped = valid_indices.len().u8_count();
                log::debug!(
                    "[ENERGY] partial: requested={} candidates={} effective={}",
                    effective_count,
                    valid_indices.len(),
                    capped
                );
                capped
            } else {
                effective_count
            };

            if !max
                && valid_indices.len() > effective_count as usize
                && state_change != "active"
                && state_change != "アクティブ"
            {
                let desc_en = format!(
                    "Select {} energy card(s) to deactivate (set to wait)",
                    effective_count
                );
                let desc_ja = format!("待機状態にするエネルギーカードを{}枚選択", effective_count);
                self.pending_choice = Some(
                    Choice::select_cards(
                        Zone::Energy.to_str(),
                        effective_count as usize,
                        desc_en,
                        false,
                    )
                    .description_ja(Some(desc_ja))
                    .card_type(card_type_filter.map(|s| s.to_string()))
                    .group(group_filter.map(|s| s.to_string()))
                    .target_player_id(Some(target.to_string()))
                    .build(),
                );
                self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                return Ok(());
            }

            let wait_cards: Vec<i16> = valid_indices
                .iter()
                .take(effective_count as usize)
                .filter_map(|i| {
                    if *i < player.energy_zone.cards.len() {
                        Some(player.energy_zone.cards[*i])
                    } else {
                        None
                    }
                })
                .collect();

            (wait_cards, effective_count)
        };

        let active_cards: Vec<i16> = if state_change == "active" || state_change == "アクティブ"
        {
            // The eligible WAITING energies selected above — identical set to
            // wait_cards. Re-scanning from zone index 0 would re-include
            // already-active cards and double-count them.
            wait_cards.clone()
        } else {
            vec![]
        };

        match state_change {
            "wait" | "ウェイト" => {
                // Wait-immunity: members protected by a `cannot_wait_by_effect`
                // restriction are not put to WAIT by the OPPONENT's effects
                // ("相手の効果によってはウェイトしない").
                let controller = gs.ability_master_id();
                let skip: Vec<i16> = wait_cards
                    .iter()
                    .filter(|&&cid| {
                        let protected = gs
                            .wait_immune_members
                            .iter()
                            .any(|(m, _)| *m == cid);
                        if !protected {
                            return false;
                        }
                        let owner = gs
                            .wait_immune_members
                            .iter()
                            .find(|(m, _)| *m == cid)
                            .map(|(_, o)| o.as_str())
                            .unwrap_or_default();
                        controller.as_deref().is_some_and(|c| c != owner)
                    })
                    .copied()
                    .collect();
                for card_id in &wait_cards {
                    if skip.contains(card_id) {
                        continue;
                    }
                    gs.mods.add_orientation_modifier(*card_id, "wait");
                }
                for _ in 0..deactivate_count {
                    let player = gs.resolve_target_player_mut(target);
                    player.energy_zone.sub_active(1);
                }
            }
            "active" | "アクティブ" => {
                for card_id in &active_cards {
                    gs.mods.add_orientation_modifier(*card_id, "active");
                    // Turn-scoped attributed log (see GameState::turn_state_changes).
                    gs.turn_state_changes.push((
                        gs.activating_card.unwrap_or(-1),
                        *card_id,
                        "wait".to_string(),
                        "active".to_string(),
                    ));
                }
                let player = gs.resolve_target_player_mut(target);
                player.energy_zone.add_active(active_cards.len().u8_count());
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn execute_set_cost(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let value: u8 = effect.value_any().unwrap_or(0) as u8;
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
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: [[log_set_cost]]", pp, act_name));
        for card_id in card_ids {
            gs.mods.set_cost_modifier(card_id, value as i16);
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

    pub(crate) fn execute_set_blade_type(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let bt_binding = effect.blade_type_any();
        let blade_type = bt_binding.as_deref();
        let target = effect.target_name();
        let dur_binding = effect.duration_any();
        let duration = dur_binding.as_deref();
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        let bt_str = blade_type.unwrap_or("none");
        gs.push_rule_log(format!(
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
                        Some((id, player.id.clone()))
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

    pub(crate) fn execute_set_heart_type(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        // C4: heart becomes the same as the card just placed under this member
        // (ref_value="placed_under") — a copy, not a fixed color.
        if effect.ref_value_any() == Some("placed_under") {
            self.execute_set_heart_copy_from_under(gs, effect.duration_any().as_deref());
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
                effect.duration_any().as_deref(),
            );
        } else if self.selected_cards.is_empty() {
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
            let tc = effect.target_count_any().unwrap_or(1) as usize;
            if candidates.len() <= tc {
                // Auto-select: push to selected_cards and apply
                for &cid in &candidates {
                    if !self.selected_cards.contains(&cid) {
                        self.selected_cards.push(cid);
                    }
                }
                self.execute_set_heart_type_applied(
                    gs,
                    heart_type,
                    effect.target_name(),
                    effect.count_or(1) as i32,
                    effect.duration_any().as_deref(),
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
                self.pending_choice = Some(
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
                self.stage_select_intent =
                    Some(crate::ability::types::StageSelectIntent::CollectTargets);
                self.sub_choice_created = true;
            }
        } else {
            // Already have selected target from previous choice resolution
            self.execute_set_heart_type_applied(
                gs,
                heart_type,
                effect.target_name(),
                effect.count_or(1) as i32,
                effect.duration_any().as_deref(),
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
        gs.push_rule_log(format!(
            "{} {}: [[log_set_heart_type:type={}]]",
            pp, act_name, ht
        ));
        // Use selected_target from self.selected_cards if available (member-targeting
        // abilities like PL!HS-bp5-021-L), otherwise fall back to activating_card
        // (self-targeting abilities like Kanan PL!S-pb1-003-R).
        let card_id = self
            .selected_cards
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
            Some(color.index() as u8),
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
            .selected_cards
            .first()
            .copied()
            .or(gs.activating_card)
            .or(self.activating_card_id)
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
                    .find(|&&cid| self.moved_cards.contains(&cid))
                    .copied()
                    .or_else(|| uc.last().copied()),
                _ => self.moved_cards.last().copied(),
            }
        };
        let Some(source) = source_card else {
            return;
        };
        gs.push_rule_log(format!(
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

    pub(crate) fn execute_activation_cost(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let operation_binding = effect.operation_any();
        let operation = operation_binding.as_deref().unwrap_or("increase");
        let value: u8 = effect.value_any().unwrap_or(0) as u8;
        let target = effect.target_name();
        let duration_binding = effect.duration_any();
        let duration = duration_binding.as_deref();
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log(format!(
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

    pub(crate) fn execute_set_card_identity(&mut self, gs: &mut GameState, identities: &[String]) {
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: カード同一性変更", pp, act_name));
        if !identities.is_empty() {
            gs.prohibition_effects
                .push(format!("card_identity:{}", identities.join(",")));
        }
    }

    pub(crate) fn execute_reduce_live_card_set_limit(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) {
        let count: u8 = effect.count_or(1) as u8;
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log(format!(
            "{} {}: [[log_reduce_live_set_limit:n={}]]",
            pp, act_name, count
        ));
        let player = gs.resolve_target_player_mut("self");
        player.live_card_set_limit_reduction += count;
    }

    pub(crate) fn execute_set_blade_count(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let value: u8 = effect.value_any().unwrap_or(effect.count_or(0)) as u8;
        let target = effect.target_name();
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log(format!(
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
        if let Some(ref pos) = effect.position_any() {
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
            gs.mods.set_blade_modifier(card_id, value as i16);
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
                    effect.duration_any().as_deref(),
                    target,
                    &format!("set blade count to {} for card {}", value, card_id),
                    Some(crate::core::types::EffectData::SetBladeCount { card_id }),
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
            self.pending_choice = Some(Choice::SelectHeartColor {
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
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: ハート色指定", pp, act_name));
    }

    pub(crate) fn execute_set_card_identity_all_regions(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) {
        let identities = effect.identities_any();
        let target = effect.target_name();
        let _target = target;
        let card_id = self.activating_card_id.or(gs.activating_card);
        if let Some(card_id) = card_id {
            if let Some(identities) = identities {
                for identity in identities {
                    gs.prohibition_effects
                        .push(format!("card_identity:{}:{}", card_id, identity));
                }
            }
        }
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: 全領域カード同一性変更", pp, act_name));
    }

    pub(crate) fn execute_set_cost_to_use(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let value: u8 = effect.value_any().unwrap_or(0) as u8;
        let card_id = self.activating_card_id.or(gs.activating_card);
        if let Some(card_id) = card_id {
            gs.mods.set_cost_modifier(card_id, value as i16);
        }
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: 使用コスト設定", pp, act_name));
        Ok(())
    }

    pub(crate) fn execute_all_blade_timing(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let timing_binding = effect.timing_any();
        let timing = timing_binding.as_deref().unwrap_or("check_required_hearts");
        let treat_as_binding = effect.treat_as_any();
        let treat_as = treat_as_binding.as_deref().unwrap_or("any_heart_color");
        let card_id = self.activating_card_id.or(gs.activating_card);
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

    pub(crate) fn execute_modify_cost(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let op_binding = effect.operation_any();
        let operation = op_binding.unwrap_or("add");
        let target = effect.target_name();
        let ct_binding = effect.card_type_any();
        let card_type = ct_binding.as_deref();
        let dur_binding = effect.duration_any();
        let duration = dur_binding.as_deref();
        // Compute the final value: base value scaled by per-unit count.
        let mut value: u8 = effect.value_any().unwrap_or(0) as u8;
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
                effect.state_any().as_deref(),
                &gs.mods.orientation_modifiers,
                gs.activating_card,
            );
            let per_unit_count = effect.per_unit_count_any().unwrap_or(1) as u8;
            let mut units = matching_count / per_unit_count;
            // Apply max_repeats cap (aliased as repeat_limit).
            // The text side-constraint "N枚までしか数えない" is parsed as
            // max_repeats on the effect.
            if let Some(cap) = effect.repeat_limit_any() {
                units = units.min(cap as u8);
            }
            value *= units;
        }
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log(format!(
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
            let selected = self.selected_cards.last().copied();
            let moved = self.moved_cards.last().copied();
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
                    (resolved - printed).clamp(i16::MIN as i32, i16::MAX as i32) as i16
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
}
