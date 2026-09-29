//! `position_change` (19), `rotation` and `play_baton_touch` (1).
//!
//! 20 of the 936 unique abilities. Formation changes, not zone moves: a
//! position change swaps members on stage rather than relocating a card.
use crate::core::constants::CountCast;
use super::super::enums::{Zone};
use super::super::resolver::AbilityResolver;
use super::super::types::{Choice, ChoiceRoute};
use super::super::util;

use crate::card::{AbilityEffect, PositionInfo};

use crate::game_state::GameState;



// `Cow` is the borrow-vs-own split for the "front" destination string below —
// a plain `&str` optimisation with no std dependency, so it comes from `alloc`
// on `no_std` and from `std` (the same type) on host builds.
#[cfg(not(feature = "no_std"))]
use std::borrow::Cow;
#[cfg(feature = "no_std")]
use alloc::{
    borrow::Cow,
    string::{String, ToString},
    vec::Vec,
};

impl AbilityResolver {

    pub(crate) fn execute_play_baton_touch(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let count: u8 = effect.count_or(1);
        let target = effect.target_name();
        log::debug!("play_baton_touch: count={}, target={}", count, target);
        let player_id = gs.resolve_target_player(target).id.clone();
        if gs.get_baton_touch_count(&player_id) > 0 {
            // Already performed baton touch during play action — no-op now.
            return Ok(());
        }
        // Double baton: generate member pair choices so the player can pick
        // which 2 occupied positions to replace. This path is used when the
        // constant ability is triggered directly (e.g. via web UI buttons).
        if count > 1 {
            let player = gs.resolve_target_player(target);
            let stage_ids = [
                player.stage.stage[0],
                player.stage.stage[1],
                player.stage.stage[2],
            ];
            let occupied: Vec<(usize, &str)> = [0, 1, 2]
                .iter()
                .filter(|&&idx| stage_ids[idx] != -1)
                // Rule 9.6.2.1.2.1: Check card identity, not area identity.
                .filter(|&&idx| !player.deployed_this_turn.contains(&stage_ids[idx]))
                .map(|&idx| {
                    let area_names = ["left", "center", "right"];
                    (idx, area_names[idx])
                })
                .collect();
            if occupied.len() < 2 {
                return Err(
                    "Not enough unlocked occupied positions for double baton touch".to_string(),
                );
            }
            let mut options = Vec::new();
            for i in 0..occupied.len() {
                for j in (i + 1)..occupied.len() {
                    let (_idx1, name1) = occupied[i];
                    let (_idx2, name2) = occupied[j];
                    options.push(format!("{},{}", name1, name2));
                }
            }
            self.awaiting.choice = Some(Choice::SelectTarget {
                target: "double_baton_touch".to_string(),
                description: "Choose 2 occupied areas for double baton touch".to_string(),
                description_en: Some("Choose 2 occupied areas for double baton touch".to_string()),
                description_ja: Some("ダブルバトンのエリア2つを選択".to_string()),
                allow_skip: true,
                options: Some(options.to_vec()),
            });
            return Ok(());
        }
        gs.prohibition_effects
            .push(format!("baton_touch_allowed:{}", count));
        let pp = self.player_prefix(gs);
        gs.rule_log
            .push(format!("{}: バトンタッチ {}回", pp, count));
        Ok(())
    }

    pub fn execute_position_change(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        position: Option<PositionInfo>,
        target: &str,
        target_member: &str,
    ) -> Result<(), String> {
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: [[log_position_change]]", pp, act_name));

        // Each-time resolution watchers (「…能力が解決したとき、そのメンバーを
        // ポジションチェンジする」): the member to move is the RESOLVER carried
        // by the queue entry (triggering_member_id), at its CURRENT area.
        // The parsed `position` field belongs to the TRIGGER clause
        // (「センターエリアにいる…」) and must not be read as a source or
        // destination here (Q255: the resolver may already have left center).
        if effect.trigger_type_any() == Some("each_time") {
            if let Some(triggering_member) = gs
                .ability_queue
                .current_entry()
                .and_then(|e| e.triggering_member_id)
            {
                let cur_idx = {
                    let player = gs.resolve_target_player_mut(target);
                    player
                        .stage
                        .stage
                        .iter()
                        .position(|&id| id == triggering_member)
                };
                let Some(cur_idx) = cur_idx else {
                    log::debug!(
                        "[RESOLVE_WATCHER] triggering member {} not on {} stage — nothing to move",
                        triggering_member,
                        target
                    );
                    return Ok(());
                };
                let cur_area = util::pos_to_area(cur_idx).to_string();
                let valid_destinations: Vec<String> = ["left", "center", "right"]
                    .iter()
                    .filter(|p| **p != cur_area.as_str())
                    .map(|p| p.to_string())
                    .collect();
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.choice_card_no = Some(ChoiceRoute::Raw(format!(
                        "position_change:{}:{}",
                        target, cur_area
                    )));
                }
                let member_name = self.card_name(triggering_member);
                self.awaiting.choice = Some(Choice::SelectTarget {
                    target: "position|destination".to_string(),
                    description: format!(
                        "Choose destination for {} (currently at {})",
                        member_name,
                        util::pos_to_area(cur_idx)
                    ),
                    description_en: Some(format!(
                        "Choose destination for {} (currently at {})",
                        member_name,
                        util::pos_to_area(cur_idx)
                    )),
                    description_ja: Some(format!(
                        "{}の移動先を選択（現在: {}）",
                        member_name,
                        util::pos_to_area(cur_idx)
                    )),
                    allow_skip: effect.optional.unwrap_or(false),
                    options: Some(valid_destinations),
                });
                return Ok(());
            }
        }

        // Check source_position from effect (new parser field), fall back to position param
        let source_pos_binding = effect.source_position_any();
        let source_pos = source_pos_binding
            .or_else(|| position.as_ref().and_then(|p| p.get_position()));
        let position_str = source_pos.unwrap_or("");

        // If destination is already specified (from conditional position_change or area_select),
        // route directly to execute_position_change_with_destination.
        // EXCEPTION: "front" destination for opponent needs source selection first.
        if let Some(dest) = effect.destination_any() {
            if dest == "front" && target == "opponent" {
                // "front" destination for opponent: the destination is fixed (front area of
                // activating card). Create a choice to select which OPPONENT member to move.
                let valid_sources: Vec<String> = {
                    let player = gs.resolve_target_player(target);
                    (0..3)
                        .filter(|&i| player.stage.stage[i] != -1)
                        .map(|i| util::pos_to_area(i).to_string())
                        .collect()
                };
                if valid_sources.is_empty() {
                    return Ok(());
                }
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.choice_card_no = Some(ChoiceRoute::Raw(
                        "position_change:opponent:front".to_string(),
                    ));
                }
                self.awaiting.choice = Some(Choice::SelectTarget {
                    target: "position|destination".to_string(),
                    description: "Choose which opponent member to move".to_string(),
                    description_en: Some("Choose which opponent member to move".to_string()),
                    description_ja: Some("移動する相手のメンバーを選択".to_string()),
                    allow_skip: effect.optional.unwrap_or(false),
                    options: Some(valid_sources),
                });
                return Ok(());
            } else {
                return self.execute_position_change_with_destination(gs, effect, dest);
            }
        }

        // "target_member: select" from parser — player must pick which member to move
        if target_member == "select" {
            let valid_sources: Vec<String> = {
                let card_db = self.card_db();
                let gns_binding = effect.group_names_any();
                let group_names = gns_binding.as_ref();
                let exclude_self = effect.exclude_self_any().unwrap_or(false);
                let activating_card_id = gs.activating_card;
                let has_explicit_target = effect.target_any().is_some();

                // When target is null (no "自分の" qualifier in card text),
                // the ability can target ANY member on either player's stage.
                // When target is explicitly set, only that player's stage.
                let players_to_check: Vec<&str> = if has_explicit_target {
                    vec![target]
                } else {
                    vec!["self", "opponent"]
                };

                let mut sources = Vec::new();
                for player_key in &players_to_check {
                    let player = gs.resolve_target_player(player_key);
                    for i in 0..3 {
                        let card_id = player.stage.stage[i];
                        if card_id == -1 {
                            continue;
                        }
                        if exclude_self && Some(card_id) == activating_card_id {
                            continue;
                        }
                        if let Some(gn) = group_names {
                            if !gn.iter().any(|g| {
                                util::card_matches_group_str(&card_db, card_id, Some(g.as_str()))
                            }) {
                                continue;
                            }
                        }
                        let pos = util::pos_to_area(i);
                        if has_explicit_target {
                            sources.push(pos.to_string());
                        } else {
                            sources.push(format!("{}:{}", player_key, pos));
                        }
                    }
                }
                sources
            };
            if valid_sources.is_empty() {
                return Ok(());
            }
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.choice_card_no = Some(ChoiceRoute::Raw(format!(
                    "position_change:{}:select",
                    target
                )));
            }
            self.awaiting.choice = Some(Choice::SelectTarget {
                target: "position|destination".to_string(),
                description: "Choose which member to move".to_string(),
                description_en: Some("Choose which member to move".to_string()),
                description_ja: Some("移動するメンバーを選択".to_string()),
                allow_skip: effect.optional.unwrap_or(false),
                options: Some(valid_sources),
            });
            return Ok(());
        }

        // Handle "both" target: opponent first (choice), then self (choice via pending).
        if target == "both" {
            let mut opp_effect = effect.clone();
            opp_effect.target = Some("opponent".into());
            self.execute_position_change(
                gs,
                &opp_effect,
                position.clone(),
                "opponent",
                target_member,
            )?;
            if self.awaiting.choice.is_some() {
                let mut self_effect = effect.clone();
                self_effect.target = Some("self".into());
                gs.ability_queue.set_pending_actions(vec![self_effect]);
            } else {
                let mut self_effect = effect.clone();
                self_effect.target = Some("self".into());
                self.execute_position_change(
                    gs,
                    &self_effect,
                    position.clone(),
                    "self",
                    target_member,
                )?;
            }
            return Ok(());
        }

        if target_member == "this_member" {
            // Handle "multiple_targets=true" (それぞれ) pattern: iterate all stage members.
            if effect.multiple_targets_any().unwrap_or(false) {
                // Check for predetermined rotation: multiple_targets + position specified.
                // e.g. 003-R: center→left, left→right, right→center
                if effect.position_any().is_some() || effect.source_position_any().is_some() {
                    return self.execute_rotation(gs, effect, target);
                }

                let target_m = effect.target_any().unwrap_or("self");
                let card_db = self.card_db();
                let mut card_ids: Vec<i16> = Vec::new();
                {
                    let player = gs.resolve_target_player_mut(target_m);
                    for i in 0..3 {
                        if player.stage.stage[i] != -1 {
                            card_ids.push(player.stage.stage[i]);
                        }
                    }
                }
                if card_ids.is_empty() {
                    return Ok(());
                }

                // Initialize formation_plan with all members (no destination yet).
                // This drives zone exclusion in compute_valid_position_destinations
                // and tracks which members still need assignment.
                self.carried.formation_plan = card_ids.iter().map(|&cid| (cid, String::new())).collect();

                // First member: create destination choice.
                let first_card_id = card_ids[0];
                let current_idx = {
                    let player = gs.resolve_target_player_mut(target_m);
                    player
                        .stage
                        .stage
                        .iter()
                        .position(|&id| id == first_card_id)
                };
                let pos_name = match current_idx {
                    Some(0) => "Left",
                    Some(1) => "Center",
                    Some(2) => "Right",
                    _ => "?",
                };
                let _first_card_name = card_db
                    .get_card(first_card_id)
                    .map(|c| c.name.to_string())
                    .unwrap_or_else(|| "member".to_string());

                let valid_destinations =
                    self.compute_valid_position_destinations(gs, effect, target_m);
                if valid_destinations.is_empty() {
                    self.carried.formation_plan.clear();
                    return Ok(());
                }
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.choice_card_no = Some(ChoiceRoute::Raw(format!(
                        "position_change:self:{}",
                        first_card_id
                    )));
                }
                self.awaiting.choice = Some(Choice::SelectTarget {
                    target: "position|destination".to_string(),
                    description: format!(
                        "Choose destination for {} (currently at {})",
                        card_db
                            .get_card(first_card_id)
                            .map(|c| c.name.as_ref())
                            .unwrap_or("member"),
                        pos_name
                    ),
                    description_en: Some(format!(
                        "Choose destination for {} (currently at {})",
                        card_db
                            .get_card(first_card_id)
                            .map(|c| c.name.as_ref())
                            .unwrap_or("member"),
                        pos_name
                    )),
                    description_ja: Some(format!(
                        "{}の移動先を選択（現在: {}）",
                        card_db
                            .get_card(first_card_id)
                            .map(|c| c.name.as_ref())
                            .unwrap_or("member"),
                        pos_name
                    )),
                    allow_skip: effect.optional.unwrap_or(false),
                    options: Some(valid_destinations),
                });
                return Ok(());
            }

            if !position_str.is_empty() {
                // Position is SOURCE ("member AT center"). Find that member on
                // the target's stage and create choice to pick destination.
                let player = gs.resolve_target_player_mut(target);
                let pos_idx = util::stage_position_index(position_str)
                    .ok_or_else(|| format!("Unknown position: {}", position_str))?;
                if player.stage.stage[pos_idx] == -1 {
                    return Ok(()); // no member at source → skip this side
                }
                let valid_destinations =
                    self.compute_valid_position_destinations(gs, effect, target);
                if valid_destinations.is_empty() {
                    return Ok(());
                }
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.choice_card_no = Some(ChoiceRoute::Raw(format!(
                        "position_change:{}:{}",
                        target, position_str
                    )));
                }
                let from_label = match position_str.to_lowercase().as_str() {
                    "center" => "Center",
                    "left" | "left_side" => "Left",
                    "right" | "right_side" => "Right",
                    _ => position_str,
                };
                self.awaiting.choice = Some(Choice::SelectTarget {
                    target: "position|destination".to_string(),
                    description: format!(
                        "Choose destination for position change (currently at {})",
                        from_label
                    ),
                    description_en: Some(format!(
                        "Choose destination for position change (currently at {})",
                        from_label
                    )),
                    description_ja: Some(format!("移動先を選択（現在: {}）", from_label)),
                    allow_skip: effect.optional.unwrap_or(false),
                    options: Some(valid_destinations),
                });
                return Ok(());
            }

            // No position specified: check if a previous area_select stored the destination.
            let stored_area = self.selection.area.clone();
            if let Some(ref area) = stored_area {
                self.selection.area = None;
                let mut copy = effect.clone();
                copy.destination = Some(Zone::from_source_str(area));
                return self.execute_position_change_with_destination(gs, &copy, area);
            }

            // No position specified: create choice for destination (move activating card).
            // Delegates to compute_valid_position_destinations which handles empty slots,
            // source exclusion, and group filtering consistently across all code paths.
            let valid_destinations = self.compute_valid_position_destinations(gs, effect, target);
            if valid_destinations.is_empty() {
                return Ok(());
            }
            // Find the activating card's current position on stage.
            let activating_card_id = gs.activating_card;
            let from_label = {
                let player = gs.resolve_target_player_mut(target);
                let pos = player
                    .stage
                    .stage
                    .iter()
                    .position(|&id| Some(id) == activating_card_id);
                match pos {
                    Some(0) => "Left".to_string(),
                    Some(1) => "Center".to_string(),
                    Some(2) => "Right".to_string(),
                    _ => "?".to_string(),
                }
            };
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.choice_card_no = Some(ChoiceRoute::Raw("position_change:self".to_string()));
            }
            self.awaiting.choice = Some(Choice::SelectTarget {
                target: "position|destination".to_string(),
                description: format!(
                    "Choose destination for position change (currently at {})",
                    from_label
                ),
                description_en: Some(format!(
                    "Choose destination for position change (currently at {})",
                    from_label
                )),
                description_ja: Some(format!("移動先を選択（現在: {}）", from_label)),
                allow_skip: effect.optional.unwrap_or(false),
                options: Some(valid_destinations),
            });
            return Ok(());
        }

        // Handle specific card_no target with no destination specified:
        // find the card's current position and create a destination choice.
        if effect.target_member_any().is_some() && position_str.is_empty() {
            let target_m = target.to_string();
            let card_no = effect
                .target_member_any()
                .unwrap_or("")
                .to_string();
            let optional = effect.optional.unwrap_or(false);
            let card_db = gs.card_database.clone();
            // Collect stage card info before mutable borrow
            let stage_snapshot: Vec<(i16, String)> = {
                let player = gs.resolve_target_player_mut(&target_m);
                (0..3)
                    .filter_map(|i| {
                        let cid = player.stage.stage[i];
                        if cid == -1 {
                            None
                        } else {
                            let cn = card_db
                                .get_card(cid)
                                .map(|c| c.card_no.to_string())
                                .unwrap_or_default();
                            Some((cid, cn))
                        }
                    })
                    .collect()
            };
            let target_pos = stage_snapshot.iter().position(|(_, cn)| cn == &card_no);
            if let Some(current_idx) = target_pos {
                let card_id = stage_snapshot[current_idx].0;
                let pos_name = match current_idx {
                    0 => "Left",
                    1 => "Center",
                    _ => "Right",
                };
                let card_name = card_db
                    .get_card(card_id)
                    .map(|c| c.name.to_string())
                    .unwrap_or_else(|| "member".to_string());
                let valid_destinations =
                    self.compute_valid_position_destinations(gs, effect, &target_m);
                if valid_destinations.is_empty() {
                    return Ok(());
                }
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.choice_card_no = Some(ChoiceRoute::Raw(format!(
                        "position_change:self:{}",
                        card_no
                    )));
                }
                self.awaiting.choice = Some(Choice::SelectTarget {
                    target: "position|destination".to_string(),
                    description: format!(
                        "Choose destination for {} (currently at {})",
                        card_name, pos_name
                    ),
                    description_en: Some(format!(
                        "Choose destination for {} (currently at {})",
                        card_name, pos_name
                    )),
                    description_ja: Some(format!(
                        "{}の移動先を選択（現在: {}）",
                        card_name, pos_name
                    )),
                    allow_skip: optional,
                    options: Some(valid_destinations),
                });
            }
            return Ok(());
        }

        let card_db = self.card_db();
        let (cause_cid, mover_pid) = (
            gs.activating_card,
            gs.ability_queue
                .current_entry()
                .map(|e| e.player_id.clone())
                .unwrap_or_default(),
        );
        let player = gs.resolve_target_player_mut(target);
        let target_index = util::stage_position_index(position_str)
            .ok_or_else(|| format!("Unknown position: {}", position_str))?;

        let current_index = player.stage.stage.iter().position(|&card_id| {
            if card_id == -1 {
                false
            } else {
                card_db
                    .get_card(card_id)
                    .map(|c| c.card_no == target_member)
                    .unwrap_or(false)
            }
        });

        if let Some(current_idx) = current_index {
            let from_area = util::pos_to_area(current_idx);
            let to_area = util::pos_to_area(target_index);
            let (target_id, source_id) = (
                player.stage.stage[target_index],
                player.stage.stage[current_idx],
            );
            if player.stage.position_change(from_area, to_area).is_ok() {
                // Push events BEFORE push_movement_event so the
                // PositionChangeEvent list captures the change before the
                // self-trigger fires from within push_movement_event.
    let source_old = current_idx.u8_count();
    let source_new = target_index.u8_count();
                if source_id != -1 {
                    gs.position_change_events
                        .push(crate::types::PositionChangeEvent {
                            moved_card_id: source_id,
                            old_position: source_old,
                            new_position: source_new,
                            cause_card_id: cause_cid,
                            cause_player_id: mover_pid.clone(),
                            effect_only: true,
                        });
                }
                if target_id != -1 {
                    gs.position_change_events
                        .push(crate::types::PositionChangeEvent {
                            moved_card_id: target_id,
                            old_position: source_new,
                            new_position: source_old,
                            cause_card_id: cause_cid,
                            cause_player_id: mover_pid.clone(),
                            effect_only: true,
                        });
                }
                let moved_ids = [target_id, source_id];
                for &cid in &moved_ids {
                    if cid != -1 {
                        gs.push_movement_event_typed(
                            cid,
                            crate::core::types::ZoneId::Stage,
                            crate::core::types::ZoneId::Stage,
                            cause_cid,
                            &mover_pid,
                            true,
                        );
                    }
                }
            } else {
                return Err(format!(
                    "Failed to move member from {:?} to {:?}",
                    from_area, to_area
                ));
            }
        } else {
            return Err(format!("Member not found: {}", target_member));
        }
        gs.position_change_occurred_this_turn = true;
        gs.trigger_auto_abilities_for_movement_current();
        gs.recalculate_constants();
        Ok(())
    }

    pub(crate) fn compute_valid_position_destinations(
        &self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        target: &str,
    ) -> Vec<String> {
        let card_db = self.card_db();
        let player = gs.resolve_target_player(target);
        let activating_card_id = gs.activating_card;
        let gns_binding = effect.group_names_any();
        let group_names = gns_binding.as_ref();
        let exclude_self = effect.exclude_self_any().unwrap_or(false);

        let position_names = ["left", "center", "right"];
        let mut valid = Vec::new();

        // Formation change: exclude zones already assigned to another member.
        let planned_zones: Vec<String> = self
            .carried.formation_plan
            .iter()
            .map(|(_, d)| d.clone())
            .filter(|d| !d.is_empty())
            .collect();

        for (i, pos_name) in position_names.iter().enumerate() {
            let card_id = player.stage.stage[i];

            // Formation change: skip zones already claimed.
            if planned_zones.contains(&pos_name.to_string()) {
                continue;
            }

            // Exclude explicitly excluded positions (e.g. "センターエリア以外" → exclude center).
            if let Some(ref exclude_pos) = effect.exclude_position_any() {
                if pos_name == exclude_pos {
                    continue;
                }
            }

            if !effect.multiple_targets_any().unwrap_or(false) {
                if let Some(source_index) = effect
                    .source_position_any()
                    .and_then(util::stage_position_index)
                {
                    if i == source_index {
                        continue;
                    }
                }
            }

            // Exclude the activating card's own position when exclude_self is set.
            if exclude_self && Some(card_id) == activating_card_id {
                continue;
            }

            // Apply group filter if specified: only positions occupied by a
            // matching group member are valid destinations. For formation
            // changes (multiple_targets=true), empty slots are always valid
            // destinations — you can move a member to any area, including
            // empty ones. For single position changes (e.g. "move to a X
            // member's area"), empty slots are invalid.
            if let Some(gn) = group_names {
                if card_id == -1 {
                    if !effect.multiple_targets_any().unwrap_or(false) {
                        continue;
                    }
                } else {
                    let matches = gn
                        .iter()
                        .any(|g| util::card_matches_group_str(&card_db, card_id, Some(g.as_str())));
                    if !matches {
                        continue;
                    }
                }
            }

            valid.push(pos_name.to_string());
        }

        log::debug!(
            "[POSITION_DESTINATIONS] target={} source={:?} exclude_self={} valid={:?}",
            target,
            effect.source_position_any(),
            exclude_self,
            valid
        );
        valid
    }

    /// Execute all formation change swaps as a batch after all members have
    /// been assigned destinations via `formation_plan`.  Each swap is executed
    /// via `stage.position_change` and each member's movement is individually
    /// tracked via `push_movement_event`.
    pub(crate) fn finalize_formation_change(&mut self, gs: &mut GameState) -> Result<(), String> {
        if self.carried.formation_plan.is_empty() {
            return Ok(());
        }
        let (cause_cid, mover_pid) = (
            gs.activating_card,
            gs.ability_queue
                .current_entry()
                .map(|e| e.player_id.clone())
                .unwrap_or_default(),
        );
        let target = "self";

        // Build desired final stage as a direct permutation.
        // Strategy: for each position, determine which card belongs there.
        // Three cases: (a) planned move into this position, (b) stay-in-place,
        // (c) evicted card from a position that was taken by another planned move.
        let player = gs.resolve_target_player_mut(target);
        let old_stage = player.stage.stage;
        let old_under = core::mem::take(&mut player.stage.under_cards);
        let mut new_stage = [-1i16; 3];
        let mut new_under = [
            smallvec::SmallVec::new(),
            smallvec::SmallVec::new(),
            smallvec::SmallVec::new(),
        ];
        let mut events: Vec<(i16, u8, u8)> = Vec::new();

        // Phase 1: place every planned card at its destination.
        // Track which card got evicted from each destination.
        let mut occupant: [i16; 3] = old_stage; // current occupant of each pos
        for &(member_id, ref dest) in &self.carried.formation_plan {
            if member_id == -1 || dest.is_empty() {
                continue;
            }
            let dest_idx = match dest.as_str() {
                "left" => 0,
                "center" => 1,
                "right" => 2,
                _ => continue,
            };
            let from_idx = match old_stage.iter().position(|&id| id == member_id) {
                Some(idx) => idx,
                None => continue,
            };
            if from_idx == dest_idx {
                continue; // handled in phase 2 (stay-in-place)
            }
            // Place member, record evicted card
            let _evicted = if new_stage[dest_idx] != -1 {
                new_stage[dest_idx]
            } else {
                occupant[dest_idx]
            };
            new_stage[dest_idx] = member_id;
            new_under[dest_idx] = old_under[from_idx].clone();
            occupant[dest_idx] = member_id;
            events.push((
        member_id,
        from_idx.u8_count(),
        dest_idx.u8_count(),
    ));
        }

        // Phase 2: place evicted cards / stay-in-place.
        // Each evicted card goes to its mover's original position.
        for &(member_id, ref dest) in &self.carried.formation_plan {
            if member_id == -1 || dest.is_empty() {
                continue;
            }
            let dest_idx = match dest.as_str() {
                "left" => 0,
                "center" => 1,
                "right" => 2,
                _ => continue,
            };
            let from_idx = match old_stage.iter().position(|&id| id == member_id) {
                Some(idx) => idx,
                None => continue,
            };
            if new_stage[from_idx] != -1 {
                continue; // slot already filled
            }
            if from_idx == dest_idx {
                // Stay in place (only if no one else moved into this slot)
                new_stage[from_idx] = member_id;
                new_under[from_idx] = old_under[from_idx].clone();
            } else {
                // Evicted occupant goes to the mover's vacated position.
                // Skip occupants that are themselves planned movers — they
                // were already relocated in Phase 1 (e.g. a clean two-way
                // swap vacates both origins; nobody is evicted). Re-inserting
                // them here would duplicate one member across two areas.
                let evicted_id = old_stage[dest_idx];
                let evicted_is_planned = self
                    .carried.formation_plan
                    .iter()
                    .any(|(id, _)| *id == evicted_id);
                if evicted_id != -1
                    && evicted_id != member_id
                    && !evicted_is_planned
                    && new_stage[from_idx] == -1
                {
                    new_stage[from_idx] = evicted_id;
                    new_under[from_idx] = old_under[dest_idx].clone();
                    events.push((
        evicted_id,
        dest_idx.u8_count(),
        from_idx.u8_count(),
    ));
                }
            }
        }

        // Phase 3: unplanned cards keep their original slot if free
        for (i, &cid) in old_stage.iter().enumerate() {
            if new_stage[i] == -1 && cid != -1 {
                let is_planned = self.carried.formation_plan.iter().any(|(id, _)| *id == cid);
                if !is_planned {
                    new_stage[i] = cid;
                    new_under[i] = old_under[i].clone();
                }
            }
        }

        // Apply atomic update
        player.stage.stage = new_stage;
        player.stage.under_cards = new_under;

        // Record position_change_events and push_movement_event for each moved card
        for &(moved_card_id, old_pos, new_pos) in &events {
            gs.position_change_events
                .push(crate::types::PositionChangeEvent {
                    moved_card_id,
                    old_position: old_pos,
                    new_position: new_pos,
                    cause_card_id: cause_cid,
                    cause_player_id: mover_pid.clone(),
                    effect_only: true,
                });
            gs.push_movement_event(moved_card_id, "stage", "stage", cause_cid, &mover_pid, true);
        }

        gs.position_change_occurred_this_turn = true;
        gs.recalculate_constants();
        self.carried.formation_plan.clear();
        Ok(())
    }

    pub fn execute_position_change_with_destination(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        destination: &str,
    ) -> Result<(), String> {
        let raw_target = effect.target_any().unwrap_or("self");
        // "both" at resolution time means "self" (the ability controller resolves choices)
        let target = if raw_target == "both" {
            "self"
        } else {
            raw_target
        };
        let target_member_binding = effect.target_member_any();
        let target_member = target_member_binding.unwrap_or("this_member");
        // Check source_position first (new parser field), fall back to position
        let sp_binding = effect.source_position_any();
        let source_position = sp_binding.or_else(|| {
            effect
                .position_any()
                .as_ref()
                .and_then(|p| p.get_position())
        });
        log::debug!(
            "[EPCWD] entry: target={} target_member={} source_pos={:?} dest={} activating={:?}",
            target,
            target_member,
            source_position,
            destination,
            self.session.activating_card_id
        );

        // destination "same_area" means the area that was just vacated by the
        // previous position_change (the source area of the first move). Since
        // position_change already swaps members, this second move is redundant.
        if destination == "same_area" {
            log::debug!("[EPCWD] same_area → early return");
            return Ok(());
        }

        // destination "front" means the area in front of the activating member
        // (mirrored position on opponent's stage per Rule 4.5.7).
        let dest_owned: Cow<'_, str> = if destination == "front" {
            let front_pos = gs.activating_card.and_then(|cid| {
                let player = gs.resolve_target_player("self");
                let idx = player.stage.stage.iter().position(|&id| id == cid)?;
                let area = util::pos_to_area(idx);
                let front = area.front_area();
                match front {
                    crate::zones::MemberArea::LeftSide => Some("left"),
                    crate::zones::MemberArea::Center => Some("center"),
                    crate::zones::MemberArea::RightSide => Some("right"),
                }
            });
            Cow::Owned(front_pos.unwrap_or("left").to_string())
        } else {
            Cow::Borrowed(destination)
        };
        let destination = dest_owned.as_ref();

        // Reject destination if it matches exclude_position
        if let Some(ref exclude) = effect.exclude_position_any() {
            let exclude_idx = util::stage_position_index(exclude).unwrap_or(999);
            let dest_idx = util::stage_position_index(destination).unwrap_or(999);
            if exclude_idx == dest_idx {
                return Err(format!(
                    "Destination {} is excluded by exclude_position={}",
                    destination, exclude
                ));
            }
        }

        let target_index = util::stage_position_index(destination)
            .ok_or_else(|| format!("Unknown destination: {}", destination))?;

        if let Some(source) = source_position {
            // Source position specified: move member AT source TO destination.
            let player = gs.resolve_target_player_mut(target);
            let source_idx = util::stage_position_index(source)
                .ok_or_else(|| format!("Unknown source position: {}", source))?;
            if player.stage.stage[source_idx] == -1 {
                return Ok(()); // no member at source, skip
            }
            if source_idx == target_index {
                log::debug!("[EPCWD] source == target → NOOP");
                return Ok(()); // same position, no move needed
            }
            let from_area2 = util::pos_to_area(source_idx);
            let to_area2 = util::pos_to_area(target_index);
            let (target_id2, source_id2) = (
                player.stage.stage[target_index],
                player.stage.stage[source_idx],
            );
            player.stage.position_change(from_area2, to_area2)?;
            let _ = player;
            gs.record_stage_position_swap(
                source_id2,
                source_idx.u8_count(),
                target_id2,
                target_index.u8_count(),
            );
            gs.trigger_auto_abilities_for_movement_current();
            gs.recalculate_constants();
            return Ok(());
        }

        // Handle specific card_no (for "multiple_targets" each-member pattern)
        if let Some(ref card_no) = effect.target_member_any() {
            if *card_no != "this_member" {
                log::debug!("[EPCWD] card_no branch: card_no={}", card_no);
                let card_db = self.card_db();
                let player = gs.resolve_target_player_mut(target);
                let current_index = player.stage.stage.iter().position(|&cid| {
                    if cid == -1 {
                        false
                    } else {
                        card_db
                            .get_card(cid)
                            .map(|c| c.card_no == *card_no)
                            .unwrap_or(false)
                    }
                });
                if let Some(current_idx) = current_index {
                    if current_idx == target_index {
                        return Ok(());
                    }
                    let from_area = util::pos_to_area(current_idx);
                    let to_area = util::pos_to_area(target_index);
                    let (target_id, source_id) = (
                        player.stage.stage[target_index],
                        player.stage.stage[current_idx],
                    );
                    player.stage.position_change(from_area, to_area)?;
                    let _ = player;
                    gs.record_stage_position_swap(
                        source_id,
                        current_idx.u8_count(),
                        target_id,
                        target_index.u8_count(),
                    );
                    gs.trigger_auto_abilities_for_movement_current();
                    gs.recalculate_constants();
                    return Ok(());
                }
            }
        }

        if target_member == "this_member" {
            log::debug!(
                "[EPCWD] this_member branch: target={} dest_idx={}",
                target,
                target_index
            );
            if let Some(activating_card_id) = self.session.activating_card_id {
                let player = gs.resolve_target_player_mut(target);
                log::debug!("[EPCWD] stage: {:?}", player.stage.stage);

                let current_index = player
                    .stage
                    .stage
                    .iter()
                    .position(|&card_id| card_id == activating_card_id);

                if let Some(current_idx) = current_index {
                    if current_idx == target_index {
                        return Ok(());
                    }
                    let from_area3 = util::pos_to_area(current_idx);
                    let to_area3 = util::pos_to_area(target_index);
                    let target_id3 = player.stage.stage[target_index];
                    player.stage.position_change(from_area3, to_area3)?;
                    let _ = player;
                    // Source is the activating card, which we just located on
                    // stage, so it is never -1; the helper's unguarded source
                    // push matches what this branch did.
                    gs.record_stage_position_swap(
                        activating_card_id,
                        current_idx.u8_count(),
                        target_id3,
                        target_index.u8_count(),
                    );
                } else {
                    return Err(format!(
                        "Activating card {} not found on stage",
                        activating_card_id
                    ));
                }
            } else {
                return Err("No activating card for position change".to_string());
            }
        }
        gs.position_change_occurred_this_turn = true;
        gs.trigger_auto_abilities_for_movement_current();
        gs.recalculate_constants();
        Ok(())
    }

    pub fn execute_rotation(
        &mut self,
        gs: &mut GameState,
        _effect: &AbilityEffect,
        target: &str,
    ) -> Result<(), String> {
        let targets: Vec<&str> = if target == "both" {
            vec!["self", "opponent"]
        } else {
            vec![target]
        };

        for tgt in targets {
            let (moved_card_ids, original_positions): (Vec<i16>, Vec<(u8, u8)>) = {
                let player = gs.resolve_target_player_mut(tgt);

                // Snapshot current stage
                let snapshot_cards = player.stage.stage;
                let snapshot_under = player.stage.under_cards.clone();

                // Rotation mapping: left(0)→right(2), center(1)→left(0), right(2)→center(1)
                let rotation_map = [2usize, 0, 1];

                // Clear the stage
                for i in 0..3 {
                    player.stage.stage[i] = -1;
                    player.stage.under_cards[i].clear();
                }

                let mut moved = Vec::new();
                let mut positions = Vec::new();
                // Place rotated members
                for src_idx in 0..3 {
                    let card_id = snapshot_cards[src_idx];
                    if card_id == -1 {
                        continue;
                    }
                    let dest_idx = rotation_map[src_idx];
                    player.stage.stage[dest_idx] = card_id;
                    player.stage.under_cards[dest_idx] = snapshot_under[src_idx].clone();
                    moved.push(card_id);
                    positions.push((
        src_idx.u8_count(),
        dest_idx.u8_count(),
    ));
                }
                (moved, positions)
            };

            for (i, &cid) in moved_card_ids.iter().enumerate() {
                let (old_pos, new_pos) = original_positions[i];
                // push_movement_event covers cards_moved_this_turn, turn_area_movements,
                // last_area_move_card_id/by_player, batch_movements, and
                // position_change_occurred_this_turn — all needed for TAS "moves" conditions.
                gs.push_movement_event(cid, "stage", "stage", gs.activating_card, "", true);
                gs.position_change_events
                    .push(crate::types::PositionChangeEvent {
                        moved_card_id: cid,
                        old_position: old_pos,
                        new_position: new_pos,
                        cause_card_id: gs.activating_card,
                        cause_player_id: String::new(),
                        effect_only: true,
                    });
            }
        }

        gs.position_change_occurred_this_turn = true;
        gs.recalculate_constants();
        gs.trigger_auto_abilities_for_movement_current();
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: [[log_rotation]]", pp, act_name));
        Ok(())
    }
}