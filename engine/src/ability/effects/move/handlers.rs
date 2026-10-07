use super::super::super::enums::Zone;
use super::super::super::resolver::AbilityResolver;
use super::super::super::types::{Choice, ExecutionContext, LookAndSelectStep};
use super::super::super::util;
use crate::card::{AbilityEffect, CardDatabase, Operator, PlacementOrder};
use crate::core::constants::CountCast;
use crate::game_state::GameState;
use crate::HashMap;
use super::placement::log_move_result;
use super::DeckOrderHint;
#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use smallvec::SmallVec;

/// The 「デッキの上からN番目に置く」 position a selection must land at, if any.
///
/// A position only means anything when the cards are going onto the deck top,
/// so a stated position with any other destination is ignored. `stashed` is the
/// fallback for when the effect context is already gone by answer time.
fn numeric_deck_position(
    select_action: Option<&AbilityEffect>,
    current: Option<&AbilityEffect>,
    stashed: Option<usize>,
    destination: &str,
) -> Option<usize> {
    if !matches!(Zone::from_str(destination), Some(Zone::DeckTop | Zone::Deck)) {
        return None;
    }
    select_action
        .and_then(|sa| sa.position_any())
        .or_else(|| current.and_then(|c| c.position_any()))
        .and_then(|pi| pi.get_position())
        .and_then(|s| s.parse::<usize>().ok())
        .or(stashed)
        .filter(|&n| n > 0)
}

/// The filter a card-selection step applies to a chosen card.
///
/// Assembled from the Choice the player was offered, plus `card_property` and
/// `negation`, which that Choice does not carry and which therefore come from
/// the activating effect. Same shape as the other selection filters in the
/// engine: build once, then ask `accepts` about each candidate.
struct SelectionFilter<'a> {
    db: crate::compat::Arc<CardDatabase>,
    card_type: Option<&'a str>,
    cost_limit: Option<u8>,
    cost_operator: Option<&'a str>,
    group: Option<&'a str>,
    characters: Option<&'a Vec<String>>,
    card_property: Option<String>,
    property_negation: bool,
}

impl SelectionFilter<'_> {
    /// Does `cid` satisfy every stated restriction?
    ///
    /// A card property with a name this engine does not know is treated as
    /// absent rather than as a filter nothing can pass, so an unrecognised
    /// property cannot silently empty the selection.
    fn accepts(&self, cid: i16) -> bool {
        util::card_matches_type(&self.db, cid, self.card_type)
            && util::card_matches_cost_limit_op(
                &self.db,
                cid,
                self.cost_limit,
                self.cost_operator,
            )
            && util::card_matches_group_str(&self.db, cid, self.group)
            && match self.characters {
                Some(chars) if !chars.is_empty() => {
                    util::card_matches_characters(&self.db, cid, Some(chars))
                }
                _ => true,
            }
            && match self.card_property.as_deref() {
                Some(prop) => {
                    let has = match prop {
                        "has_blade_heart" => {
                            self.db.get_card(cid).is_some_and(|c| c.has_blade_heart())
                        }
                        "has_score_icon" => {
                            self.db.get_card(cid).is_some_and(|c| c.has_score_icon())
                        }
                        "has_all_blade" => {
                            self.db.get_card(cid).is_some_and(|c| c.has_all_blade())
                        }
                        "has_draw_icon" => {
                            self.db.get_card(cid).is_some_and(|c| c.has_draw_icon())
                        }
                        _ => false,
                    };
                    if self.property_negation {
                        !has
                    } else {
                        has
                    }
                }
                None => true,
            }
    }
}

impl<'a> SelectionFilter<'a> {
    /// Keep only the indices whose card passes the filter, logging each
    /// rejection. Non-matching cards are silently skipped from the effect's
    /// point of view (consistent with the cost-phase handler).
    fn retain_valid(
        &self,
        player: &crate::player::Player,
        zone: &str,
        indices: &[usize],
    ) -> Vec<usize> {
        let cards = util::zone_cards(player, zone);
        let result: Vec<usize> = indices
            .iter()
            .filter(|&&idx| {
                let ok = idx < cards.len() && self.accepts(cards[idx]);
                if !ok {
                    log::debug!(
                        "[SELECTION_REJECTED] zone={} index={} reason={}",
                        zone,
                        idx,
                        if idx >= cards.len() {
                            "out_of_bounds"
                        } else {
                            "filter_mismatch"
                        }
                    );
                    log::trace!("[SELECTION_CARD] index={} id={:?}", idx, cards.get(idx));
                }
                ok
            })
            .copied()
            .collect();
        log::debug!(
            "[SELECTION_MAPPED] zone={} requested={:?} accepted={:?}",
            zone,
            indices,
            result
        );
        result
    }
}


impl AbilityResolver {
    pub fn handle_select_position(
        &mut self,
        gs: &mut GameState,
        position: &str,
        context: ExecutionContext,
    ) -> Result<(), String> {
        match &context {
            ExecutionContext::LookAndSelect {
                step:
                    LookAndSelectStep::Finalize {
                        destination,
                        source_zone,
                    },
            } => {
                if Zone::from_str(destination) == Some(Zone::Stage) {
                        if let Some(&card_id) = gs.looked_at_cards.last() {
                            let player = &mut gs.player1;
                            let pos_idx = super::util::stage_position_index(position);
                            let should_lock = source_zone != Zone::Stage.to_str();
                            fn do_place(
                                player: &mut crate::player::Player,
                                idx: usize,
                                card_id: i16,
                                should_lock: bool,
                            ) {
                                if player.stage.stage[idx] != -1 {
                                    player.waitroom.add_card(player.stage.stage[idx]);
                                }
                                player.stage.stage[idx] = card_id;
                                if should_lock {
                                    // Rule 9.6.2.1.2.1: Track card deployed from non-stage.
                                    player.track_deployment(card_id);
                                }
                            }
                            match pos_idx {
                                Some(0) => do_place(player, 0, card_id, should_lock),
                                Some(1) => do_place(player, 1, card_id, should_lock),
                                Some(2) => do_place(player, 2, card_id, should_lock),
                                _ => {
                                    player.hand.add_card(card_id);
                                }
                            }
                            gs.looked_at_cards.clear();
                        }
                    }
            }
            ExecutionContext::MoveCardsPosition {
                card_id,
                state_change,
                target,
                source_zone,
            } => {
                let card_id = *card_id;
                let state_change = state_change.clone();
                let target = target.clone();
                let should_lock = source_zone != Zone::Stage.to_str();

                let pos_idx = super::util::stage_position_index(position);

                let player = gs.resolve_target_player_mut(&target);
                let mut placed = match pos_idx {
                    Some(idx) if idx < 3 && player.stage.stage[idx] == -1 => {
                        player.stage.stage[idx] = card_id;
                        if should_lock {
                            // Rule 9.6.2.1.2.1: Track card deployed from non-stage.
                            player.track_deployment(card_id);
                        }
                        true
                    }
                    _ => false,
                };
                // If chosen position is occupied, replace (move existing to waitroom)
                if !placed {
                    if let Some(idx) = pos_idx {
                        if idx < 3 && player.stage.stage[idx] != -1 {
                            player.waitroom.add_card(player.stage.stage[idx]);
                            player.stage.stage[idx] = card_id;
                            if should_lock {
                                // Rule 9.6.2.1.2.1: Track card deployed from non-stage.
                                player.track_deployment(card_id);
                            }
                            placed = true;
                        }
                    }
                }
                if !placed {
                    player.hand.add_card(card_id);
                }

                gs.mods.clear_all_for_card(card_id);
                gs.record_card_movement(card_id);
                if !self.selection.moved_cards.contains(&card_id) {
                    self.selection.moved_cards.push(card_id);
                }
                if state_change.as_deref() == Some("wait") {
                    gs.set_orientation(card_id, "wait");
                }
                log::debug!("[DEPLOY_POSITION] target={} position={} placed={} next=debut_side_effects", target, position, placed);
                log_move_result(gs.resolve_target_player(&target), &gs.card_database, card_id, source_zone, "stage");
                self.fire_debut_side_effects(gs, card_id, &target);
            }
            _ => {}
        }
        // Clear only the position choice itself. fire_debut_side_effects may
        // have queued the placed card's OWN debut (Q200/Q201/Q202), whose
        // cost/effect prompts must survive — a blanket clear here silently
        // swallowed them.
        if matches!(
            self.awaiting.choice,
            Some(Choice::SelectPosition { .. })
        ) {
            self.awaiting.choice = None;
        }
        log::debug!(
            "[DEPLOY] after position handling: pending={:?}",
            format!("{:?}", self.awaiting.choice)
        );
        self.in_flight.execution_context = ExecutionContext::None;
        // Place remaining cards deferred by multi-card stage selection
        // BEFORE resuming pending commands, so deferred cards get their
        // position choices before the re-prompt (if any).
        if !self.awaiting.stage_cards.is_empty() {
            let remaining = core::mem::take(&mut self.awaiting.stage_cards);
            for (i, (cid, tgt)) in remaining.iter().enumerate() {
                let source = self.in_flight.spawn_context.source.clone().unwrap_or_default();
                match self.place_card_with_stage_choice(
                    gs,
                    tgt,
                    *cid,
                    Zone::Stage.to_str(),
                    None,
                    false,
                    1,
                    None,
                    None,
                    &source,
                    false,
                    false,
                ) {
                    Ok(true) => {
                        if i + 1 < remaining.len() {
                            self.awaiting.stage_cards = remaining[i + 1..].into();
                        }
                        return Ok(());
                    }
                    Ok(false) => {
                        self.fire_debut_side_effects(gs, *cid, tgt);
                        gs.mods.clear_all_for_card(*cid);
                        gs.record_card_movement(*cid);
                    }
                    Err(_) => {}
                }
            }
        }
        self.resume_pending_actions(gs)?;
        log::debug!(
            "[DEPLOY] after resume_pending_actions: pending={:?}",
            format!("{:?}", self.awaiting.choice)
        );
        Ok(())
    }


pub fn execute_selected_cards_from_zone(
        &mut self,
        gs: &mut GameState,
        zone: &str,
        indices: &[usize],
        card_type_filter: Option<&str>,
        cost_limit: Option<u8>,
        cost_limit_operator: Option<&str>,
        cost_total: Option<u8>,
        cost_total_operator: Option<&str>,
        group: Option<&str>,
        characters: Option<&Vec<String>>,
        target_player_id: Option<&str>,
    ) -> Result<(), String> {
        let destination = gs
            .entry_destination()
            .map(|s| s.to_string())
            .or_else(|| self.in_flight.spawn_context.destination.clone())
            // Sub-action select_cards steps carry their own destination
            // (e.g. 希 bp3-007 「1枚をデッキの上に置き」) — without this
            // fallback they all defaulted to discard.
            .or_else(|| {
                gs.entry_effect()
                    .and_then(|e| e.destination.map(|z| z.to_str().to_string()))
            });
        // Target for zone operations (whose zone to read/write) comes from spawn_context.target
        // (set by the effect execution), NOT from target_player_id (who makes the choice).
        let target = self
            .in_flight.spawn_context
            .target
            .clone()
            .or_else(|| {
                gs.entry_effect()
                    .and_then(|e| e.target.clone().map(|s| s.to_string()))
            })
            .unwrap_or_else(|| "self".to_string());
        // target_player_id is only used for choice routing, not for zone operations
        let _choice_player = target_player_id;
        let card_db = gs.card_database.clone();
        let vacated_area = gs.last_vacated_stage_area;

        // Card-property restriction (e.g. emma bp7-008 「ブレードハートを
        // 持たないメンバーカード」→ card_property="has_blade_heart",
        // negation=true). The Choice advertised to the player does not carry
        // these fields, so read them from the activating effect.
        let filter = SelectionFilter {
            db: card_db.clone(),
            card_type: card_type_filter,
            cost_limit,
            cost_operator: cost_limit_operator,
            group,
            characters,
            card_property: gs
                .entry_effect()
                .and_then(|e| e.card_property_any().map(|s| s.to_string())),
            property_negation: gs
                .entry_effect()
                .and_then(|e| e.negation_any())
                .unwrap_or(false),
        };

        // Filter indices to only include cards that match required filters.
        let filtered_indices: Vec<usize> =
            filter.retain_valid(gs.resolve_target_player(&target), zone, indices);

        let zone_enum = Zone::from_str(zone);
        let dest = destination
            .as_deref()
            .unwrap_or(if zone_enum == Some(Zone::Discard) {
                Zone::Hand.to_str()
            } else {
                Zone::Discard.to_str()
            });
        log::debug!("[SELECTION_ROUTE] source={} destination={} fallback={} target={} choice_player={:?}", zone, dest, destination.is_none(), target, target_player_id);
        if zone_enum == Some(Zone::Hand) {
            if let Some(effect) = self.owning_effect(gs) {
                let mut order_indices = filtered_indices.clone();
                order_indices.sort_unstable_by(|a, b| b.cmp(a));
                let order_card_ids = {
                    let player = gs.resolve_target_player(&target);
                    util::resolve_indices_to_ids(player, zone, &order_indices)
                };
                let mut order_moved = Vec::new();
                if self.maybe_prompt_deck_order(
                    gs,
                    DeckOrderHint::of(effect),
                    &order_card_ids,
                    zone,
                    dest,
                    &mut order_moved,
                ) {
                    let removed = {
                        let player = gs.resolve_target_player_mut(&target);
                        util::zone_remove_at_indices(player, zone, &filtered_indices)
                    };
                    gs.on_cards_left_zones(&removed);
                    self.in_flight.sub_choice_created = true;
                    return Ok(());
                }
            }
        }
        let mut moved = Vec::new();
        match zone_enum {
            Some(Zone::Hand)
            | Some(Zone::Discard)
            | Some(Zone::Deck)
            | Some(Zone::LiveCardZone) => {
                if Zone::from_str(dest) == Some(Zone::Stage) {
                    let player = gs.resolve_target_player(&target);
                    if player.stage.stage.iter().all(|&id| id != -1) {
                        log::debug!("[STAGE_PLACEMENT] stage is full, cannot place cards");
                        return Ok(());
                    }
                }

                // A SUM constraint applies only when the ability text states a
                // total (「コストの合計」→ parsed cost_total). Falling back to
                // the per-card cost_limit here was wrong: 「コスト2以下の
                // メンバーを2枚まで」 limits EACH card, and eligibility is
                // already enforced by the filtered_indices pass above.
                let sum_limit = cost_total;
                let sum_operator = cost_total_operator;
                if zone_enum == Some(Zone::Discard) {
                    if let Some(limit) = sum_limit {
                        let player = gs.resolve_target_player(&target);
                        let op = sum_operator.unwrap_or("<=");
                        let card_ids =
                            util::resolve_indices_to_ids(player, zone, &filtered_indices);
                        let total_cost: u8 = card_ids
                            .iter()
                            .filter_map(|&cid| card_db.get_card(cid).and_then(|c| c.cost))
                            .sum();
                        // A selection with no stated operator means "up to the
                        // limit", so an operator we do not know keeps that meaning
                        // rather than rejecting the selection.
                        let ok = util::compare_with_operator(op, total_cost, limit)
                            .unwrap_or(total_cost <= limit);
                        if !ok {
                            log::debug!(
                                "[SELECTION_REJECTED] reason=total_cost actual={} operator={} limit={}",
                                total_cost,
                                op,
                                limit
                            );
                            return Ok(());
                        }
                    }
                }

                let card_ids = {
                    let player = gs.resolve_target_player(&target);
                    util::resolve_indices_to_ids(player, zone, &filtered_indices)
                };

                match Zone::from_str(dest) {
                    Some(Zone::Stage) | Some(Zone::EmptyArea) | Some(Zone::SameArea)
                        if zone_enum != Some(Zone::Deck) =>
                    {
                        moved = self.execute_stage_placement_choices(
                            gs,
                            &card_ids,
                            zone,
                            dest,
                            vacated_area,
                            &target,
                        )?;
                    }
                    Some(Zone::Deck) => {
                        // Resolve deck position from the entry effect,
                        // falling back to spawn_context.position for sub-action effects.
                        let deck_pos = gs
                            .entry_effect()
                            .and_then(|ef| {
                                ef.position_any().as_ref().and_then(|p| match p {
                                    crate::card::PositionInfo::String(s) => s.parse::<u8>().ok(),
                                    crate::card::PositionInfo::Struct { position, .. } => {
                                        position.as_ref().and_then(|s| s.parse::<u8>().ok())
                                    }
                                })
                            })
                            .or(self.in_flight.spawn_context.position)
                            .map(|p| if p > 0 { p - 1 } else { 0 });
                        let player = gs.resolve_target_player_mut(&target);
                        for &cid in &card_ids {
                            if let Some(pos) = deck_pos {
                                let clamped = (pos as usize).min(player.main_deck.cards.len());
                                player.main_deck.cards.insert(clamped, cid);
                            } else {
                                util::place_card_in_zone(player, cid, dest, None, false, 1);
                            }
                        }
                        util::zone_remove_at_indices(player, zone, &filtered_indices);
                        for &cid in &card_ids {
                            log_move_result(player, &card_db, cid, zone, dest);
                        }
                        moved = card_ids;
                    }
                    _ if dest == "deck_top_or_bottom" => {
                        if let Some(&cid) = card_ids.first() {
                            self.prompt_deck_top_or_bottom(
                                cid,
                                None,
                                target.clone(),
                                zone.to_string(),
                                false,
                            );
                            self.in_flight.sub_choice_created = true;
                            return Ok(());
                        }
                    }
                    _ => {
                        log::debug!(
                            "[MOVE_INTENT] source={} destination={} selected_count={} target={}",
                            zone,
                            dest,
                            card_ids.len(),
                            target
                        );
                        // Check for success zone replacement (e.g. 錯覚CROSSROADS)
                        if let Some(&replaced_card_id) = card_ids.first() {
                            if self.maybe_prompt_success_replacement(
                                gs,
                                replaced_card_id,
                                dest,
                                &target,
                            ) {
                                return Ok(());
                            }
                        }
                        let player = gs.resolve_target_player_mut(&target);
                        util::move_cards(player, &card_ids, zone, dest, None, &card_db);
                        for &cid in &card_ids {
                            log_move_result(player, &card_db, cid, zone, dest);
                        }
                        moved = card_ids;
                    }
                }

                if zone_enum == Some(Zone::Hand)
                    && (Zone::from_str(dest) == Some(Zone::Discard)
                        || Zone::from_str(dest) == Some(Zone::Waitroom))
                {
                    gs.mods.last_cost_discard_count = moved.len().u8_count();
                    gs.mods.last_cost_moved_card_ids = moved.clone().into();
                }
            }
            Some(Zone::Stage) => {
                let player = gs.resolve_target_player_mut(&target);
                for &idx in &filtered_indices {
                    if idx < 3 && player.stage.stage[idx] != -1 {
                        self.selection.cards.push(player.stage.stage[idx]);
                    }
                }
            }
            Some(Zone::RevealedCards) => {
                for &idx in filtered_indices.iter().rev() {
                    if idx < gs.revealed_cards.len() {
                        let card_id = gs
                            .remove_revealed_at(idx)
                            .expect("index bounds checked above");
                        self.selection.cards.push(card_id);
                    }
                }
            }
            _ => return Err(format!("Unknown zone: {}", zone)),
        }

        for cid in &moved {
            gs.mods.clear_all_for_card(*cid);
            if !self.selection.cards.contains(cid) {
                self.selection.cards.push(*cid);
            }
            if !self.selection.moved_cards.contains(cid) {
                self.selection.moved_cards.push(*cid);
            }
        }

        let state_change = gs
            .ability_queue
            .current_entry()
            .and_then(|e| e.ability.effect.as_ref())
            .and_then(|ef| ef.state_change_any().map(|s| s.to_string()));
        if let Some(sc) = state_change {
            if sc == "wait" {
                for &cid in &moved {
                    gs.set_orientation(cid, "wait");
                }
            }
        }

        if !moved.is_empty() {
            let cause_cid = gs.activating_card;
            let cause_pid = gs
                .ability_queue
                .current_entry()
                .map(|e| e.player_id.clone())
                .unwrap_or_default();
            for &cid in &moved {
                if cid != -1 {
                    gs.push_movement_event(cid, zone, dest, cause_cid, &cause_pid, true);
                }
            }
        }
        Ok(())
    }

    /// Handle looked_at card selection: validate, move to destination, handle multi-select and remaining cards.
    // Q86 / Q122 / Rule 4.8.3: Finalize look-and-select selection
    //
    // After the user selects cards from looked_at_cards:
    //   1. Selected cards → destination (usually hand, via place_card_in_zone)
    //   2. Unselected cards → discard (if discard_remaining = true)
    //      or → deck bottom (if discard_remaining = false)
    //   3. Per-group constraint (max_per_group) enforced before moving
    //
    // Rule 4.8.3: When moving multiple cards from main deck, they are moved
    //   one at a time. Since these cards are already in looked_at_cards
    //   (not in the deck zone anymore), this rule doesn't apply here.
    //
    // Q86: After selected cards go to hand and remainder to discard, if
    //   the source deck is now empty, Rule 10.2.2.1 triggers at next
    //   check timing (refresh).
    //
    // Q122: If an effect looks at cards and puts them back on deck
    //   (rearrangement), the cards were never removed from the deck zone
    //   for refresh purposes, so no refresh occurs during the effect.
    pub fn handle_select_cards_looked_at(
        &mut self,
        gs: &mut GameState,
        indices: &[usize],
        ctx_destination: Option<String>,
        ctx_discard_remaining: Option<bool>,
    ) -> Result<(), String> {
        let target = self
            .in_flight.spawn_context
            .target
            .clone()
            .or_else(|| {
                gs.entry_effect()
                    .and_then(|e| e.target.clone().map(|s| s.to_string()))
            })
            .unwrap_or_else(|| "self".to_string());
        let select_action = self
            .owner.executing
            .as_ref()
            .and_then(|ef| ef.compound_branch().and_then(|c| c.select_action.clone()))
            .or_else(|| {
                gs.ability_queue
                    .current_entry()
                    .and_then(|e| e.ability.effect.as_ref())
                    .and_then(|ef| ef.compound_branch().and_then(|c| c.select_action.clone()))
            })
            .or_else(|| {
                self.owner.executing.as_ref().and_then(|ef| {
                    if ef.action == crate::ability::enums::ActionType::SelectCards {
                        Some(Box::new(ef.clone()))
                    } else {
                        None
                    }
                })
            });
        let current = self.owner.executing.as_ref();
        let remainder_dest = select_action
            .as_ref()
            .and_then(|sa| sa.remainder_destination_any())
            .or_else(|| current.and_then(|c| c.remainder_destination_any()))
            .map(str::to_string);
        log::debug!(
            "[LA_REMAINDER] selected_indices={:?} destination={:?}",
            indices,
            remainder_dest
        );
        // Whether the EFFECT TEXT explicitly says what happens to the
        // unselected remainder (e.g. 「残りを控え室に置く」). When it does NOT,
        // a fully-declined optional selection must return the cards to where
        // they were looked at from — not to the discard fallback.
        let explicit_discard = select_action
            .as_ref()
            .and_then(|sa| sa.discard_remaining_any())
            .or_else(|| current.and_then(|c| c.discard_remaining_any()))
            .or(ctx_discard_remaining);
        let (destination, discard_remaining, placement_order) = (
            select_action
                .as_ref()
                .and_then(|sa| sa.destination.map(|s| s.to_string()))
                .or_else(|| current.and_then(|c| c.destination.map(|s| s.to_string())))
                .or(ctx_destination)
                .unwrap_or_else(|| Zone::Hand.to_str().to_string()),
            explicit_discard.unwrap_or(true),
            select_action
                .as_ref()
                .and_then(|sa| sa.placement_order_any())
                .or_else(|| current.and_then(|c| c.placement_order_any())),
        );

        if gs.looked_at_cards.is_empty() && !self.selection.cards.is_empty() {
            gs.looked_at_cards = self.selection.cards.iter().copied().collect();
        }

        log::debug!(
            "[LA_DEBUG] len={} indices={:?}",
            gs.looked_at_cards.len(),
            indices
        );
        log::debug!(
            "[LA_DEST_DBG] resolved_dest={:?} discard_remaining={:?} placement_order={:?}",
            destination, discard_remaining, placement_order
        );

        // Extract per-group constraint from execution context
        let max_per_group = match &self.in_flight.execution_context {
            ExecutionContext::LookAndSelect {
                step: LookAndSelectStep::Select { max_per_group, .. },
            } => *max_per_group,
            _ => None,
        };

        let looked_at = &mut gs.looked_at_cards;

        // Validate per-group constraint before removing cards
        if let Some(mpg) = max_per_group {
            let card_db = &gs.card_database;
            let mut group_counts: HashMap<String, u8> = HashMap::default();
            for &idx in indices {
                if idx < looked_at.len() {
                    let cid = looked_at[idx];
                    if let Some(card) = card_db.get_card(cid) {
                        if !card.series.is_empty() {
                            let count = group_counts.entry(card.series.to_string()).or_insert(0);
                            *count += 1;
                            if *count > mpg {
                                return Err(format!(
                                    "Cannot select more than {} {} from the same series ({})",
                                    mpg, util::card_plural(mpg as usize), card.series
                                ));
                            }
                        }
                    }
                }
            }
        }

        let mut indices_sorted: Vec<usize> = indices.to_vec();
        indices_sorted.sort_by(|a, b| b.cmp(a));

        let mut selected_cards: Vec<i16> = Vec::new();
        for i in indices_sorted {
            if i < looked_at.len() {
                selected_cards.insert(0, looked_at.remove(i));
            }
        }

        {
            let card_db = &gs.card_database;
            let cost_limit = select_action.as_ref().and_then(|sa| sa.cost_limit_any());
            let cost_limit_operator = select_action
                .as_ref()
                .and_then(|sa| sa.cost_limit_operator_any().map(Operator::as_str));
            selected_cards.retain(|&cid| {
                util::card_matches_cost_limit_op(card_db, cid, cost_limit, cost_limit_operator)
            });
        }
        // AFTER the cost-limit retain: a card the filter rejects was never taken,
        // so it must not count against the printed cap.
        let selected_count = selected_cards.len();
        // 「2枚まで」 bounds the TOTAL this look hands over across every repeated
        // answer. Measuring each answer on its own let a player who picked one card
        // at a time walk past the cap. Counted here (before any early return) so
        // every later path — including choice.rs's own re-prompt — sees one number.
        self.carried.looked_at_taken += selected_count;
        let cumulative_taken = self.carried.looked_at_taken;

        let remaining_cards: SmallVec<[i16; 8]> = core::mem::take(looked_at);

        let is_deck_dest = Zone::from_str(&destination) == Some(Zone::DeckTop)
            || Zone::from_str(&destination) == Some(Zone::Deck)
            || Zone::from_str(&destination) == Some(Zone::DeckBottom);
        let needs_order = is_deck_dest
            && placement_order == Some(PlacementOrder::AnyOrder)
            && selected_cards.len() > 1;

        if needs_order {
            gs.looked_at_cards = selected_cards.into();
            let player = gs.resolve_target_player_mut(&target);
            let dest_zone = if discard_remaining {
                Zone::Discard.to_str()
            } else {
                Zone::DeckBottom.to_str()
            };
            for card_id in remaining_cards {
                util::place_card_in_zone(player, card_id, dest_zone, None, false, 1);
            }
            let card_count = gs.looked_at_cards.len();
            self.awaiting.choice = Some(Choice::SelectTarget {
                target: "order".to_string(),
                description: format!("Choose order for cards on deck ({} cards)", card_count),
                description_en: Some(format!(
                    "Choose order for cards on deck ({} cards)",
                    card_count
                )),
                description_ja: Some(format!("山札のカード順を選択（{}枚）", card_count)),
                allow_skip: false,
                options: None,
            });
            self.in_flight.execution_context = ExecutionContext::LookAndSelect {
                step: LookAndSelectStep::Finalize {
                    destination: Zone::Deck.to_str().to_string(),
                    source_zone: String::new(),
                },
            };
            return Ok(());
        }

        let player = gs.resolve_target_player_mut(&target);
        // 「デッキの上からN番目に置く」— numeric deck position insert
        // (PositionInfo position "4" = 4th from the TOP = index N-1).
        // The effect context is gone at answer time, so also honor the
        // position captured when the optional looked_at choice was spawned.
        // The stash is consumed unconditionally so a stale value can never
        // leak into an unrelated later selection.
        let stashed_deck_pos = self.carried.looked_at_deck_position.take();
        let numeric_deck_pos = numeric_deck_position(
            select_action.as_deref(),
            current,
            stashed_deck_pos,
            &destination,
        );
        if let Some(n) = numeric_deck_pos {
            for &card_id in &selected_cards {
                let idx = (n - 1).min(player.main_deck.cards.len());
                player.main_deck.cards.insert(idx, card_id);
                log::debug!(
                    "[LA_DECK_POS] inserted card {} at index {} ({}th from top)",
                    card_id, idx, n
                );
            }
        } else {
            for &card_id in &selected_cards {
                util::place_card_in_zone(player, card_id, &destination, None, false, 1);
            }
        }
        self.selection.moved_cards.extend(selected_cards.iter().copied());

        let any_number = select_action
            .as_ref()
            .and_then(|sa| sa.any_number_any())
            .unwrap_or(false);
        let json_count = select_action.as_ref().and_then(|sa| sa.count).unwrap_or(1) as usize;
        // 「好きな枚数」 caps at the WHOLE look window: cards taken on earlier answers
        // plus what is still offered. Recomputing from this answer alone cut the cap
        // short as soon as a second answer arrived.
        let max_select = if any_number {
            cumulative_taken + remaining_cards.len()
        } else {
            json_count
        };
        let can_select_more = select_action
            .as_ref()
            .map(|sa| {
                sa.max.unwrap_or(false)
                    || sa.optional.unwrap_or(false)
                    || any_number
                    || json_count > cumulative_taken
            })
            .unwrap_or(false);

        // 「…3枚まで公開して手札に加えてもよい。残りを控え室に置く。」 — a
        // 「残り」 card is by definition one the filter REJECTS, so it is not
        // offerable again. Gating the re-prompt on `!remaining_cards.is_empty()`
        // therefore built a zero-option prompt the moment the eligible set ran
        // out, and the remainder was stranded rather than milled: see
        // `stranded_remainder_risk` in `cards/test_inventory.py`. Test
        // reachability on the ELIGIBLE set — which is also what bounds the
        // honest pick count.
        let remaining_indices: Vec<usize> = {
            let card_db = &gs.card_database;
            let filter = select_action
                .as_ref()
                .map(|sa| util::CardFilter::from_effect(sa));
            match filter {
                Some(ref f) if f.has_filter() => remaining_cards
                    .iter()
                    .enumerate()
                    .filter(|&(_, &cid)| f.matches(card_db, cid, false))
                    .map(|(i, _)| i)
                    .collect(),
                _ => (0..remaining_cards.len()).collect(),
            }
        };

        if selected_count > 0
            && can_select_more
            && max_select > cumulative_taken
            && !remaining_indices.is_empty()
        {
            gs.looked_at_cards = remaining_cards.clone();
            let remaining_available = gs.looked_at_cards.len();
            // Never promise more picks than there are cards left that CAN be
            // picked. With no filter this is `remaining_available` as before.
            let remaining_selections =
                (max_select - cumulative_taken).min(remaining_indices.len());
            let description = format!(
                "Select up to {} more {} from the {} remaining looked-at {}",
                remaining_selections,
                util::card_plural(remaining_selections as usize),
                remaining_available,
                util::card_plural(remaining_available)
            );
            let description_ja = format!(
                "残りの見たカード{}枚から最大{}枚まで選択（スキップ可）",
                remaining_available, remaining_selections
            );
            log::debug!(
                "[LOOKED_AT_PROMPT] remaining dest=looked_at en={:?} ja={:?}",
                description, description_ja
            );
            self.awaiting.choice = Some(
                Choice::select_cards(
                    Zone::LookedAt.to_str(),
                    remaining_selections,
                    description,
                    true,
                )
                .description_ja(Some(description_ja))
                .card_type(
                    select_action
                        .as_ref()
                        .and_then(|sa| sa.card_type_any().map(|s| s.to_string())),
                )
                .cost_limit(
                    select_action.as_ref().and_then(|sa| sa.cost_limit_any()),
                    select_action
                        .as_ref()
                        .and_then(|sa| sa.cost_limit_operator_any().map(|s| s.to_string())),
                )
                .group(
                    select_action
                        .as_ref()
                        .and_then(|sa| sa.group_names_any())
                        .and_then(|v| v.first().cloned()),
                )
                .characters(
                    select_action
                        .as_ref()
                        .and_then(|sa| sa.characters_any().cloned()),
                )
                .filtered_indices(Some(remaining_indices))
                .build(),
            );
            return Ok(());
        }

        let player = gs.resolve_target_player_mut(&target);

        // Optional move DECLINED outright (nothing selected, no explicit
        // remainder directive): the looked-at cards go back where they came
        // from. Rule 5.7 — 見る only informs; skipping 「置いてもよい」 must not
        // discard or reposition anything.
        if selected_cards.is_empty() && explicit_discard.is_none() && remainder_dest.is_none() {
            let origin = self
                .carried.looked_at_origin
                .clone()
                .unwrap_or_else(|| Zone::DeckTop.to_str().to_string());
            for &card_id in &remaining_cards {
                util::place_card_in_zone(player, card_id, &origin, None, false, 1);
            }
            log::debug!(
                "[LA_SKIP_RETURN] {} {} returned to origin {}",
                remaining_cards.len(),
                util::card_plural(remaining_cards.len()),
                origin
            );
            gs.looked_at_cards.clear();
            self.awaiting.choice = None;
            return Ok(());
        }

        // If the effect specifies where the REMAINING (unselected) looked-at cards
        // go (e.g. "残りを好きな順番でデッキの下に置く" → deck_bottom), honor that.
        // Otherwise fall back to discard_remaining (discard) or deck top.
        if remainder_dest.as_deref() == Some("looked_at") {
            // Intermediate leg of a multi-destination look split (希 bp3-007):
            // leftovers stay in the pool for the NEXT select step.
            gs.looked_at_cards = remaining_cards;
            self.awaiting.choice = None;
            return Ok(());
        }
        let dest_zone = if let Some(rd) = remainder_dest {
            rd
        } else if discard_remaining {
            Zone::Discard.to_str().to_string()
        } else {
            Zone::DeckTop.to_str().to_string()
        };
        for &card_id in &remaining_cards {
            util::place_card_in_zone(player, card_id, &dest_zone, None, false, 1);
        }
        if discard_remaining {
            // Track the discarded cards so each_time watchers (e.g. Hazuki Ren ab#1)
            // can react to them as a single batch discard event.
            self.finalize_card_movement(gs, &remaining_cards, &dest_zone, "deck_top", &None, None);
        }

        // Clear the stale looked_at choice now that all cards have been
        // processed (selected cards → hand, remaining → waitroom).
        // Without this, finalize_choice's should_preserve logic keeps the
        // original looked_at SelectCard alive through the followup_action
        // pending command, and resume_queue_with_choice re-stages it as a
        // spurious sub-choice prompt.
        self.awaiting.choice = None;

        Ok(())
    }

    /// Execute energy zone selection: optionally move cards to a destination.
    pub fn handle_energy_zone_selection(
        &mut self,
        gs: &mut GameState,
        indices: &[usize],
        count: usize,
        destination: Option<String>,
        validate_card: &mut impl FnMut(i16) -> bool,
    ) -> Result<(), String> {
        log::debug!(
            "[ENERGY_SEL] indices={:?} count={} dst={:?} zone_now={:?}",
            indices,
            count,
            destination,
            gs.resolve_target_player("self").energy_zone.cards
        );
        if let Some(ref dst) = destination {
            let cids: Vec<i16> = {
                let player = gs.resolve_target_player_mut("self");
                let mut removed = Vec::new();
                // High→low so earlier removals don't shift later indices.
                // remove_at adjusts the counter only for actually-active
                // cards (positional convention).
                for &i in indices.iter().rev() {
                    if i < player.energy_zone.cards.len()
                        && validate_card(player.energy_zone.cards[i])
                    {
                        if let Some(cid) = player.energy_zone.remove_at(i) {
                            removed.push(cid);
                        }
                    }
                }
                removed.reverse();
                removed
            };
            if dst == Zone::UnderMember.to_str() {
                if cids.is_empty() {
                    // Optional placement was skipped (allow_skip). Clear the
                    // pending conditional actions (そうした場合) so subsequent
                    // effects (e.g. draw / gain_resource) do not still fire.
                    gs.ability_queue.take_pending_actions();
                    if let Some(entry) = gs.ability_queue.current_entry_mut() {
                        entry.optional_cost_result = Some(false);
                    }
                } else {
                    self.place_energy_under_member_selected(gs, &cids);
                }
            } else {
                let player = gs.resolve_target_player_mut("self");
                for &cid in &cids {
                    crate::ability::util::place_card_in_zone(player, cid, dst, None, false, 1);
                }
            }
            for &cid in &cids {
                gs.mods.clear_all_for_card(cid);
                gs.record_card_movement(cid);
            }
            self.selection.moved_cards = cids.into();
        } else {
            self.execute_selected_energy_zone_cards(gs, indices, count)?;
        }
        Ok(())
    }

    /// Place the tapped energy cards under the activating member (Rule 10.5.3).
    /// Used for the energy_zone → under_member placement choice. Falls back to
    /// the moved_cards / center / left / right slots when the activating card is
    /// not on stage. If the chosen slot has no member, energy goes to the energy
    /// deck (Rule 10.5.4).
    fn place_energy_under_member_selected(
        &mut self,
        gs: &mut GameState,
        cids: &[i16],
    ) {
        if cids.is_empty() {
            return;
        }
        let activating_pos =
            gs.activating_card.and_then(|c| gs.resolve_target_player("self").stage.stage.iter().position(|&id| id == c));
        let player = gs.resolve_target_player_mut("self");
        let target_index = if let Some(idx) = activating_pos {
            if player.stage.stage[idx] != -1 {
                idx
            } else {
                for &card in cids {
                    player.energy_deck.cards.push(card);
                }
                return;
            }
        } else if let Some(idx) = self
            .selection.moved_cards
            .iter()
            .rev()
            .find_map(|&cid| player.stage.stage.iter().position(|&id| id == cid))
        {
            idx
        } else if player.stage.stage[1] != -1 {
            1
        } else if player.stage.stage[0] != -1 {
            0
        } else if player.stage.stage[2] != -1 {
            2
        } else {
            for &card in cids {
                player.energy_deck.cards.push(card);
            }
            return;
        };
        if player.stage.stage[target_index] == -1 {
            for &card in cids {
                player.energy_deck.cards.push(card);
            }
            return;
        }
        let area = util::pos_to_area(target_index);
        for &card in cids {
            player.stage.place_under_card(area, card);
        }
        // Record the placement as a movement event so energy-placement watcher
        // triggers (e.g. "エネルギーがメンバーの下に置かれたとき") fire.
        let cause_pid = gs
            .ability_queue
            .current_entry()
            .map(|e| e.player_id.clone())
            .unwrap_or_default();
        let cause_cid = gs.activating_card;
        for &card in cids {
            gs.push_movement_event(
                card,
                "energy_zone",
                "under_member",
                cause_cid,
                &cause_pid,
                true,
            );
        }
        gs.recalculate_constants();
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log_fmt(format_args!(
            "{} {}: [[log_energy_under:n={}]]",
            pp, act_name, cids.len()
        ));
    }

    /// Execute energy zone cards: move to wait state. Only actually-active
    /// cards move the counter (mark_waited is a no-op for already-waited);
    /// the wait mod write below stays idempotent.
    pub fn execute_selected_energy_zone_cards(
        &mut self,
        gs: &mut GameState,
        indices: &[usize],
        _count: usize,
    ) -> Result<(), String> {
        let player = gs.resolve_target_player_mut("self");
        // Snapshot the selected ids BEFORE rebuilding (positions shift).
        let to_mark: Vec<i16> = indices
            .iter()
            .filter_map(|&i| player.energy_zone.cards.get(i).copied())
            .collect();
        // Positional rebuild (duplicate-id safe): no id re-resolution.
        player.energy_zone.set_indices_waited(indices);
        for cid in to_mark {
            gs.mods.clear_all_for_card(cid);
            gs.set_orientation(cid, "wait");
        }
        Ok(())
    }


}
