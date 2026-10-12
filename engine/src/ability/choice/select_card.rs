use super::*;

impl super::resolver::AbilityResolver {
    pub(in crate::ability::choice) fn handle_select_card(
        &mut self,
        gs: &mut GameState,
        choice: &Choice,
        zone: &str,
        context: ExecutionContext,
        ctx: SelectionContext,
    ) -> Result<(), String> {
        log::debug!(
            "[CHOICE] source={:?} select cards: zone={} indices={:?} filtered_indices={:?} count={} allow_skip={} reveal={} context={:?}",
            self.session.activating_card_id, zone, ctx.indices, ctx.filtered_indices, ctx.count, ctx.allow_skip, ctx.is_reveal, context
        );

        // §4.13: the zone string was parsed five times in
        // this function; parse it once and match the Copy
        // enum.
        let zone_enum = Zone::from_str(zone);

        // Distinguish cost vs effect: cost handler only fires when effect NOT yet started.
        // effect_started is false during cost payment, true during effect execution.
        let effect_started = gs
            .ability_queue
            .current_entry()
            .is_some_and(|e| e.effect_started);

        // Handle reveal action: push selected cards to revealed_cards, don't discard.
        if ctx.is_reveal && zone_enum == Some(Zone::Hand) {
            return self.handle_reveal_selection(gs, &ctx, &context);
        }

        if !effect_started
            && gs
                .entry_cost()
                .is_some_and(|cost| cost.action == ActionType::Reveal)
        {
            return self.handle_entry_cost_reveal(gs, &ctx, &context);
        }

        let card_type: &Option<String> = &ctx.card_type;
        let count: usize = ctx.count;
        let allow_skip: bool = ctx.allow_skip;
        let indices: &[usize] = &ctx.indices;
        let cost_limit: Option<u8> = ctx.cost_limit;
        let cost_limit_operator: &Option<String> = &ctx.cost_limit_operator;
        let cost_total: Option<u8> = ctx.cost_total;
        let cost_total_operator: &Option<String> = &ctx.cost_total_operator;
        let group: &Option<String> = &ctx.group;
        let characters: &Option<Vec<String>> = &ctx.characters;
        let is_select_action: bool = ctx.is_select_action;
        let target_player_id: &Option<String> = &ctx.target_player_id;

        let card_db = gs.card_database.clone();
        let validate_filter = choice.as_filter();
        let mut validate_card =
            |cid: i16| -> bool { validate_filter.matches(&card_db, cid, false) };

        self.consume_deferred_gate(gs, indices);

        log::trace!(
            "[COST] hand-payment routing: zone={} is_hand={} has_cost={} effect_started={}",
            zone,
            zone_enum == Some(Zone::Hand),
            gs.entry_cost().is_some(),
            effect_started
        );
        if zone_enum == Some(Zone::Hand) && gs.entry_cost().is_some() && !effect_started
        {
            return self.handle_hand_cost_payment(
                gs,
                &ctx,
                indices,
                count,
                allow_skip,
                cost_total,
                cost_total_operator.clone(),
                target_player_id,
                &card_db,
                &mut validate_card,
            );
        }

        if zone_enum == Some(Zone::Energy)
            && !effect_started
            && ctx.destination.as_deref() != Some("under_member")
        {
            return self.handle_energy_cost_payment(
                gs,
                &ctx,
                indices,
                cost_total,
                cost_total_operator.clone(),
                target_player_id,
            );
        }

        log::trace!(
            "[CHOICE] selection routing: source={:?} zone={} effect_started={} queued_actions={}",
            self.session.activating_card_id, zone, effect_started, gs.ability_queue.has_pending_actions()
        );
        if self.resume_under_member_placement(gs, zone, &ctx, &context) {
            return self.resume_pending_actions(gs);
        }
        match zone_enum {
            Some(Zone::Hand) => {
                return self.handle_hand_selection(gs, &ctx, &context, &mut validate_card);
            }
            Some(Zone::Deck) => self.execute_selected_cards_from_zone(
                gs,
                Zone::Deck.to_str(),
                indices,
                card_type.as_deref(),
                cost_limit,
                cost_limit_operator.as_deref(),
                cost_total,
                cost_total_operator.as_deref(),
                group.as_deref(),
                characters.as_ref(),
                target_player_id.as_deref(),
            )?,
            Some(Zone::Discard) => {
                return self.handle_discard_selection(gs, &ctx, &context, &mut validate_card);
            }
            Some(Zone::LookedAt) => {
                return self.handle_looked_at_selection(gs, &ctx, &context);
            }
            Some(Zone::RevealedCards) => {
                let edst = gs.entry_destination().map(|s| s.to_string());
                let dst_str = ctx
                    .destination
                    .clone()
                    .or(edst)
                    .unwrap_or_else(|| Zone::Discard.to_string());
                self.handle_revealed_cards_selection(gs, &ctx, &mut validate_card, &dst_str)?;
            }
            Some(Zone::Energy) => {
                let dst = if let Choice::SelectCard { destination, .. } = choice {
                    destination.clone()
                } else {
                    None
                };
                self.handle_energy_zone_selection(gs, indices, count, dst, &mut validate_card)?;
            }
            Some(Zone::SelectedCards) => {
                self.narrow_selected_pool(indices);
            }
            Some(Zone::LiveCardZone) => {
                if is_select_action {
                    self.take_live_card_zone_selection(gs, &ctx, target_player_id);
                } else {
                    self.move_live_card_zone_cards(gs, &ctx, target_player_id, &card_db, &mut validate_card);
                }
            }
            Some(Zone::Stage) => {
                self.handle_stage_selection(gs, &ctx, &mut validate_card)?;
            }
            Some(Zone::UnderMember) => {
                if self.handle_under_member_zone(gs, &ctx, target_player_id, &mut validate_card)? {
                    return Ok(());
                }
            }
            Some(Zone::SuccessLiveZone) => {
                return self.handle_success_live_zone_selection(gs, &ctx, &mut validate_card);
            }
            _ => {
                return Err(format!(
                    "Card selection from zone '{}' not yet implemented or unsupported",
                    zone
                ))
            }
        }
        log::debug!(
            "[CHOICE] source={:?} selection complete: zone={} selected_count={} cards={:?}",
            self.session.activating_card_id,
            zone,
            self.selection.cards.len(),
            self.selection.cards
        );
        self.handle_selection_epilogue(gs, &context)
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::ability::choice) fn build_reprompt(
        &self,
        ctx: &SelectionContext,
        zone: &str,
        count: usize,
        desc: String,
        desc_ja: String,
        skip: bool,
        fi: Option<Vec<usize>>,
        tpid: Option<String>,
        ct: Option<u8>,
        cto: Option<String>,
    ) -> ChoiceBuilder {
        Choice::select_cards(zone, count, desc, skip)
            .description_ja(Some(desc_ja))
            .card_type(ctx.card_type.clone())
            .cost_limit(ctx.cost_limit, ctx.cost_limit_operator.clone())
            .cost_total(ct, cto)
            .group(ctx.group.clone())
            .characters(ctx.characters.clone())
            .filtered_indices(fi)
            .target_player_id(tpid)
    }

    pub(in crate::ability::choice) fn handle_hand_selection(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
        context: &ExecutionContext,
        validate_card: &mut impl FnMut(i16) -> bool,
    ) -> Result<(), String> {
        let mapped_indices = ctx.mfi(&ctx.indices);
        let hand_idx = if mapped_indices.is_empty() && !ctx.allow_skip && ctx.count > 0 {
            return Err("No cards selected from hand for required selection".to_string());
        } else {
            &mapped_indices
        };
        // C6 keep-N-shuffle-rest: the selected hand cards are KEPT in hand (not
        // moved); the handler later shuffles the non-selected under the deck.
        // Record the chosen card IDs into selected_cards and return — do NOT
        // execute the selection (move) or clear selected_cards.
        if self.selection.keep_shuffle_phase > 0 {
            let target = ctx
                .target_player_id
                .as_deref()
                .unwrap_or("self")
                .to_string();
            let hand_cards: Vec<i16> = {
                let p = gs.resolve_target_player_mut(&target);
                p.hand.cards.to_vec()
            };
            // Record the chosen hand POSITIONS to keep (hand is unchanged during
            // selection, so these absolute positions map onto the snapshot).
            for &idx in hand_idx.iter() {
                let idx = idx.u8_count();
                if !self.selection.keep_shuffle_selected.contains(&idx) {
                    self.selection.keep_shuffle_selected.push(idx);
                }
            }
            let count = usize::from(self.selection.keep_shuffle_count);
            let available_idxs: Vec<usize> = (0..hand_cards.len())
                .filter(|i| !self.selection.keep_shuffle_selected.contains(&i.u8_count()))
                .collect();
            if hand_idx.len() < count && !hand_idx.is_empty() && !available_idxs.is_empty() {
                let remaining = count.saturating_sub(self.selection.keep_shuffle_selected.len().min(count));
                let fi = Some(available_idxs);
                let desc = format!(
                    "Select up to {} more card(s) from hand to keep",
                    remaining
                );
                self.awaiting.choice = Some(
                    self.build_reprompt(
                        ctx,
                        Zone::Hand.to_str(),
                        remaining,
                        desc,
                        "手札からさらに選ぶ（スキップで終了）".to_string(),
                        true,
                        fi,
                        Some(target.clone()),
                        ctx.cost_total,
                        ctx.cost_total_operator.clone(),
                    )
                    .build(),
                );
                self.store_pending_choice(gs);
                return Ok(());
            }
            if self.selection.keep_shuffle_phase == 1 {
                // Self's selection completed (including empty "keep 0" which
                // correctly shuffles the entire hand under). Move self's
                // non-selected cards, then prompt opponent.
                let snapshot = self.selection.keep_shuffle_snapshots[0].clone();
                self.move_non_selected_hand_to_deck_bottom(gs, "self", &snapshot);
                self.selection.keep_shuffle_selected.clear();
                // Snapshot opponent hand before prompting
                let opp_hand = gs.resolve_target_player_mut("opponent").hand.cards.to_vec();
                self.selection.keep_shuffle_snapshots.push(opp_hand);
                let opp_hand_len = self.selection.keep_shuffle_snapshots[1].len();
                let pick = (count).min(opp_hand_len);
                let c = crate::ability::types::Choice::select_cards(
                    Zone::Hand.to_str(),
                    pick,
                    format!("Select up to {} card(s) to keep", count),
                    true,
                )
                .description_ja(Some(format!(
                    "最大{}枚まで手札に残すカードを選択",
                    count
                )))
                .target_player_id(Some("opponent".to_string()))
                .build();
                self.selection.keep_shuffle_phase = 2;
                self.in_flight.spawn_context.target = Some("opponent".to_string());
                self.awaiting.choice = Some(c);
                self.store_pending_choice(gs);
                return Ok(());
            }
            // Opponent's selection is recorded (phase 2). The effect does not
            // re-enter at phase 2 (sub-choice resolution), so move opponent's
            // non-selected hand cards under opponent's deck right here.
            if self.selection.keep_shuffle_phase == 2 {
                let snapshot = self.selection.keep_shuffle_snapshots[1].clone();
                self.move_non_selected_hand_to_deck_bottom(gs, "opponent", &snapshot);
                self.selection.keep_shuffle_phase = 0;
                self.selection.keep_shuffle_snapshots.clear();
                self.selection.keep_shuffle_selected.clear();
                self.in_flight.spawn_context.target = None;
                // The sequential's second action is "draw 3 for both". The
                // pending queue may be corrupted (observed n=2 [select,draw]),
                // so perform the draw directly here and clear any pending draw
                // to avoid double-draw.
                let card_db = gs.card_database.clone();
                for player in [&mut gs.player1, &mut gs.player2] {
                    let _ = crate::ability::effects::deck::draw_cards_for_player(
                        player,
                        3,
                        "deck",
                        "hand",
                        None,
                        false,
                        None,
                        &card_db,
                        None,
                    );
                }
                // Clear any pending sequential actions (the draw is now
                // performed directly). This prevents the corrupted
                // [select,draw] pending from restarting a second keep_shuffle.
                gs.ability_queue.take_pending_actions();
            }
            return self.handle_selection_epilogue(gs, context);
        }
        if !hand_idx.is_empty() || ctx.allow_skip {
            if !hand_idx.is_empty() && ctx.count > 0 && hand_idx.len() < ctx.count {
                let target = ctx
                    .target_player_id
                    .as_deref()
                    .unwrap_or("self")
                    .to_string();
                let hand_cards: Vec<i16> = {
                    let p = gs.resolve_target_player_mut(&target);
                    p.hand.cards.to_vec()
                };
                let mut all_hand_idxs = hand_idx.to_vec();
                if !self.selection.cards.is_empty() {
                    for (hidx, cid) in hand_cards.iter().enumerate() {
                        if self.selection.cards.contains(cid) && !all_hand_idxs.contains(&hidx) {
                            all_hand_idxs.push(hidx);
                        }
                    }
                }
                let new_card_ids: Vec<i16> = all_hand_idxs
                    .iter()
                    .filter_map(|&idx| {
                        if idx < hand_cards.len() {
                            Some(hand_cards[idx])
                        } else {
                            None
                        }
                    })
                    .collect();
                for cid in new_card_ids {
                    if !self.selection.cards.contains(&cid) {
                        self.selection.cards.push(cid);
                    }
                }
                let remaining = ctx.count - hand_idx.len();
                let available_idxs: Vec<usize> = (0..hand_cards.len())
                    .filter(|i| !all_hand_idxs.contains(i))
                    .collect();
                let fi = if available_idxs.is_empty() {
                    None
                } else {
                    Some(available_idxs)
                };
                let desc = format!(
                    "Select {} more card(s) from hand{}",
                    remaining,
                    if ctx.blind { " (blind)" } else { "" }
                    // Rule 4.1.2.3 audit: `blind` is prompt text only — the
                    // player is never allowed to treat a matching hidden card
                    // as absent, as the rulebook permits.
                );
                let desc_ja = format!(
                    "手札からさらに{}枚選択{}",
                    remaining,
                    if ctx.blind {
                        "（控えに選択）"
                    } else {
                        ""
                    }
                );
                self.awaiting.choice = Some(
                    self.build_reprompt(
                        ctx,
                        Zone::Hand.to_str(),
                        remaining,
                        desc,
                        desc_ja,
                        false,
                        fi,
                        Some(target.clone()),
                        ctx.cost_total,
                        ctx.cost_total_operator.clone(),
                    )
                    .blind(ctx.blind)
                    .is_reveal(ctx.is_reveal)
                    .build(),
                );
                self.store_pending_choice(gs);
                return Ok(());
            }
            if !hand_idx.is_empty() && ctx.count == 0 && ctx.allow_skip {
                let target = ctx
                    .target_player_id
                    .as_deref()
                    .unwrap_or("self")
                    .to_string();
                let old_hand_cards: Vec<i16> = {
                    let p = gs.resolve_target_player(&target);
                    p.hand.cards.to_vec()
                };
                let moved_ids: Vec<i16> = hand_idx
                    .iter()
                    .filter_map(|&idx| old_hand_cards.get(idx).copied())
                    .collect();
                self.execute_selected_cards_from_zone(
                    gs,
                    Zone::Hand.to_str(),
                    hand_idx,
                    ctx.card_type.as_deref(),
                    ctx.cost_limit,
                    ctx.cost_limit_operator.as_deref(),
                    ctx.cost_total,
                    ctx.cost_total_operator.as_deref(),
                    ctx.group.as_deref(),
                    ctx.characters.as_ref(),
                    ctx.target_player_id.as_deref(),
                )?;
                for &cid in &moved_ids {
                    if !self.selection.cards.contains(&cid) {
                        self.selection.cards.push(cid);
                    }
                }
                let hand_cards: Vec<i16> = {
                    let p = gs.resolve_target_player_mut(&target);
                    p.hand.cards.to_vec()
                };
                let include_idxs: Vec<usize> = (0..hand_cards.len())
                    .filter(|i| validate_card(hand_cards[*i]))
                    .collect();
                let fi = if include_idxs.is_empty() {
                    None
                } else {
                    Some(include_idxs)
                };
                self.awaiting.choice = Some(
                    self.build_reprompt(
                        ctx,
                        Zone::Hand.to_str(),
                        0,
                        "Select more card(s) from hand (or skip to finish)".to_string(),
                        "手札からさらに選択（スキップで終了）".to_string(),
                        true,
                        fi,
                        Some(target),
                        ctx.cost_total,
                        ctx.cost_total_operator.clone(),
                    )
                    .build(),
                );
                self.store_pending_choice(gs);
                return Ok(());
            }
            let mut all_idxs = hand_idx.to_vec();
            if !self.selection.cards.is_empty() {
                let target = ctx
                    .target_player_id
                    .as_deref()
                    .unwrap_or("self")
                    .to_string();
                let hand_cards: Vec<i16> = {
                    let p = gs.resolve_target_player_mut(&target);
                    p.hand.cards.to_vec()
                };
                for (hidx, cid) in hand_cards.iter().enumerate() {
                    if self.selection.cards.contains(cid) && !all_idxs.contains(&hidx) {
                        all_idxs.push(hidx);
                    }
                }
            }
            let selected_hand_ids: Vec<i16> = {
                let hand_target = ctx
                    .target_player_id
                    .clone()
                    .unwrap_or_else(|| "self".to_string());
                let hand_cards = gs.resolve_target_player(&hand_target).hand.cards.clone();
                all_idxs
                    .iter()
                    .filter_map(|index| hand_cards.get(*index).copied())
                    .collect()
            };
            self.execute_selected_cards_from_zone(
                gs,
                Zone::Hand.to_str(),
                &all_idxs,
                ctx.card_type.as_deref(),
                ctx.cost_limit,
                ctx.cost_limit_operator.as_deref(),
                ctx.cost_total,
                ctx.cost_total_operator.as_deref(),
                ctx.group.as_deref(),
                ctx.characters.as_ref(),
                ctx.target_player_id.as_deref(),
            )?;
            // The widest of the owner chains: the snapshot, then the queue
            // entry's root effect, then the queued ability's own effect. This
            // site accepts a card selection made on behalf of an ability whose
            // effect lives on the ability rather than the entry, so the third
            // link is load-bearing here and nowhere else.
            let keep_selected = self
                .answering_effect(gs)
                .or_else(|| {
                    gs.ability_queue
                        .current_entry()
                        .and_then(|entry| entry.ability.effect.as_deref())
                })
                .is_some_and(util::effect_uses_selected_cards);
            if keep_selected {
                self.selection.cards = selected_hand_ids.into();
            } else {
                self.selection.cards.clear();
            }
        }
        if ctx.allow_skip {
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.optional_cost_result =
                    Some(!ctx.indices.is_empty() || !self.selection.moved_cards.is_empty());
            }
            let is_opponent_action = ctx.target_player_id.as_deref() == Some("opponent");
            if ctx.indices.is_empty()
                && ctx.count > 0
                && !is_opponent_action
                && self.selection.moved_cards.is_empty()
            {
                gs.ability_queue.take_pending_actions();
            }
        }
        self.handle_selection_epilogue(gs, context)
    }

    pub(in crate::ability::choice) fn handle_reveal_selection(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
        context: &ExecutionContext,
    ) -> Result<(), String> {
        let effect_started = gs
            .ability_queue
            .current_entry()
            .is_some_and(|e| e.effect_started);
        let target = ctx
            .target_player_id
            .clone()
            .unwrap_or_else(|| "self".to_string());

        let hand_positions: Vec<usize> = if let Some(ref fi) = ctx.filtered_indices {
            if ctx.count == 0 {
                ctx.indices
                    .iter()
                    .filter_map(|&i| fi.get(i).copied())
                    .collect()
            } else {
                ctx.indices.to_vec()
            }
        } else {
            ctx.indices.to_vec()
        };

        if !hand_positions.is_empty() && ctx.count > 0 && hand_positions.len() < ctx.count {
            for &hp in &hand_positions {
                if !self
                    .selection.cards
                    .contains(&hp.i16_count())
                {
                    self.selection.cards.push(hp.i16_count());
                }
            }
            let remaining = ctx.count - hand_positions.len();
            self.awaiting.choice = Some(
                Choice::select_cards(
                    Zone::Hand.to_str(),
                    remaining,
                    format!(
                        "Select {} more card(s) from hand{}",
                        remaining,
                        if ctx.blind { " (blind)" } else { "" }
                    ),
                    false,
                )
                .description_ja(Some(format!(
                    "手札からさらに{}枚選択{}",
                    remaining,
                    if ctx.blind {
                        "（控えに選択）"
                    } else {
                        ""
                    }
                )))
                .target_player_id(Some(target.clone()))
                .blind(ctx.blind)
                .is_reveal(true)
                .filtered_indices(Some(
                    self.selection.cards
            .iter()
            .map(|&i| i.usize_count())
            .collect(),
                ))
                .build(),
            );
            self.store_pending_choice(gs);
            return Ok(());
        }

        let mut all_indices: Vec<usize> = self
            .selection.cards
            .iter()
            .map(|&i| i.usize_count())
            .collect();
        self.selection.cards.clear();
        for &hp in &hand_positions {
            if !all_indices.contains(&hp) {
                all_indices.push(hp);
            }
        }

        let ids_to_reveal = if ctx.count == 0 {
            &hand_positions
        } else {
            &all_indices
        };
        let revealed_card_ids = {
            let player = gs.resolve_target_player_mut(&target);
            util::resolve_indices_to_ids(player, Zone::Hand.to_str(), ids_to_reveal)
        };

        let source = gs.current_ability_source_card_id();
        let owner = util::target_player_index(&target, gs.ability_master_id());
        for &cid in &revealed_card_ids {
            gs.push_revealed_card(cid, source, false, owner, "ability");
        }
        if revealed_card_ids.is_empty() && ctx.allow_skip && !effect_started {
            self.selection.cards.clear();
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.mark_cost_paid();
                entry.optional_cost_result = Some(false);
            }
            return self.finalize_choice(gs, context);
        }
        if !revealed_card_ids.is_empty() {
            let player_label = super::util::target_player_label(&target, gs.ability_master_id());
            let turn = gs.turn_number;
            gs.push_rule_log_fmt(format_args!(
                "[Turn {}] {} [[log_reveal_hand:n={}]]",
                turn,
                player_label,
                revealed_card_ids.len()
            ));
        }
        // Reduced to a bool here: the borrow of the effect tree would
        // otherwise stay live across the `&mut gs` work below. Where the
        // effect itself is needed (re-queueing it) it is re-read then.
        let selected_effect_uses_chosen = self
            .answering_effect(gs)
            .is_some_and(util::effect_uses_selected_cards);
        if !effect_started {
            let cost_source = gs.current_ability_source_card_id();
            let cost_owner = util::target_player_index(&target, gs.ability_master_id());
            for &cid in &revealed_card_ids {
                gs.push_revealed_cost_card(cid, cost_source, false, cost_owner, "cost");
            }
        }
        if selected_effect_uses_chosen {
            self.selection.cards = revealed_card_ids.clone().into();
        }

        if ctx.count == 0 && ctx.allow_skip && !effect_started && !all_indices.is_empty() {
            self.selection.cards = all_indices
            .iter()
            .map(|&i| i.i16_count())
            .collect();
            let hand_len = {
                let p = gs.resolve_target_player_mut(&target);
                p.hand.cards.len()
            };
            let remaining_indices: Vec<usize> = (0..hand_len)
                .filter(|&i| !all_indices.contains(&i))
                .collect();
            if !remaining_indices.is_empty() {
                self.awaiting.choice = Some(
                    Choice::select_cards(
                        Zone::Hand.to_str(),
                        0,
                        "Select more cards to reveal from hand (or skip to finish)",
                        true,
                    )
                    .description_ja(Some(
                        "手札からさらにカードを選択して公開（スキップで終了）".to_string(),
                    ))
                    .filtered_indices(Some(remaining_indices))
                    .card_type(ctx.card_type.clone())
                    .is_reveal(true)
                    .target_player_id(Some(target.clone()))
                    .build(),
                );
                self.store_pending_choice(gs);
                return Ok(());
            }
        }

        self.clear_choice_state(gs);
        if !effect_started && selected_effect_uses_chosen {
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.effect_started = true;
            }
            // Storing the effect needs ownership; borrowing it for the test
            // above does not.
            if let Some(effect) = self.answering_effect(gs) {
                gs.ability_queue.set_pending_actions(vec![effect.clone()]);
            }
        }
        self.resume_pending_actions(gs)
    }

    pub(in crate::ability::choice) fn handle_revealed_cards_selection(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
        validate_card: &mut impl FnMut(i16) -> bool,
        dst_str: &str,
    ) -> Result<(), String> {
        let mapped = ctx.mfi(&ctx.indices);
        let moved = self.move_from_revealed(gs, &mapped, validate_card, dst_str);
        // If this is a select action, store selected cards for downstream
        // sequential actions (e.g. gain_resource with heart_colors_from_selected_card).
        if ctx.is_select_action {
            for &cid in &moved {
                if !self.selection.cards.contains(&cid) {
                    self.selection.cards.push(cid);
                }
            }
        }
        // Track moved cards so preceding_moved conditions on the same
        // ability (e.g. conditional_on_result) can see them.
        self.selection.moved_cards.extend(moved.iter().copied());
        // Apply resource_on_select if present — grants resource (e.g. blade)
        // automatically when a card is selected from revealed_cards.
        let res = self
            .owner.executing
            .as_ref()
            .and_then(|e| e.resource_on_select_any().cloned());
        if let Some(ref res) = res {
            self.execute_effect(gs, res)?;
        }
        if self
            .owner.executing
            .as_ref()
            .is_some_and(|e| e.discard_remaining_any().unwrap_or(false))
        {
            let remaining: Vec<i16> = gs
                .revealed_cards
                .iter()
                .filter(|&&cid| !moved.contains(&cid))
                .copied()
                .collect();
            let player = gs.resolve_target_player_mut("self");
            for &cid in &remaining {
                player.waitroom.add_card(cid);
            }
            // Both halves, kept index-aligned: `revealed_card_meta` exists to
            // be read (owner scoping in condition/card.rs), and a pool clear
            // that leaves the meta behind shifts every later owner tag.
            gs.clear_revealed_cards();
        }
        // When user skips an optional revealed_cards selection, clear any
        // pending sequential actions (e.g. re_yell should not fire after skip).
        let is_opponent_action = ctx.target_player_id.as_deref() == Some("opponent");
        if ctx.indices.is_empty()
            && ctx.allow_skip
            && ctx.count > 0
            && !is_opponent_action
            && moved.is_empty()
        {
            gs.ability_queue.take_pending_actions();
        }
        Ok(())
    }

    pub(in crate::ability::choice) fn handle_success_live_zone_selection(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
        validate_card: &mut impl FnMut(i16) -> bool,
    ) -> Result<(), String> {
        let mapped = ctx.mfi(&ctx.indices);
        let edst = gs.entry_destination().map(|s| s.to_string());
        let dst_str = ctx
            .destination
            .clone()
            .or(edst)
            .unwrap_or_else(|| Zone::Discard.to_string());
        let target = ctx.target_player_id.as_deref().unwrap_or("self");
        let card_db = gs.card_database.clone();
        let player = gs.resolve_target_player_mut(target);
        let card_ids =
            util::resolve_indices_to_ids(player, Zone::SuccessLiveZone.to_str(), &mapped);
        if !card_ids.is_empty() {
            let valid_ids: Vec<i16> = card_ids
                .into_iter()
                .filter(|&cid| validate_card(cid))
                .collect();
            if !valid_ids.is_empty() {
                let (mc, _, _) = util::move_cards(
                    player,
                    &valid_ids,
                    Zone::SuccessLiveZone.to_str(),
                    &dst_str,
                    None,
                    &card_db,
                );
                if mc > 0 {
                    self.selection.moved_cards = valid_ids.clone().into();
gs.set_recently_moved_batch(valid_ids.into(), Some(Zone::SuccessLiveZone.to_str()));
                }
            }
        }
        self.clear_choice_state(gs);
        let pending = gs.ability_queue.take_pending_actions();
        let filtered: Vec<AbilityEffect> = pending
            .into_iter()
            .filter(|cmd| cmd.source != Some(Zone::SuccessLiveZone))
            .collect();
        gs.ability_queue.set_pending_actions(filtered);
        self.resume_pending_actions(gs)
    }

    pub(in crate::ability::choice) fn handle_entry_cost_reveal(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
        context: &ExecutionContext,
    ) -> Result<(), String> {
        let Some(cost) = gs.entry_cost().cloned() else {
            log::error!("handle_entry_cost_reveal: no entry cost on queue entry; aborting reveal");
            self.clear_choice_state(gs);
            return Err("entry cost reveal without entry cost".to_string());
        };
        let card_db = gs.card_database.clone();
        let player = gs.resolve_target_player("self");
        let card_ids: Vec<i16> = ctx
            .indices
            .iter()
            .filter_map(|&idx| {
                if idx < player.hand.cards.len() {
                    let cid = player.hand.cards[idx];
                    let passes =
                        util::card_matches_type(
                            &card_db,
                            cid,
                            cost.card_type_any().map(|ct| ct.as_card_str()),
                        ) && util::card_matches_characters(
                            &card_db,
                            cid,
                            cost.characters_any().map(|v| &**v),
                        ) && match cost.group_names_any().as_ref() {
                            Some(groups) => groups.iter().any(|g| {
                                util::card_matches_group_str(&card_db, cid, Some(g.as_str()))
                            }),
                            None => true,
                        } && util::card_matches_cost_limit(&card_db, cid, cost.cost_limit_any());
                    if passes {
                        Some(cid)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        let count = cost.count.unwrap_or(1).usize_count();
        if card_ids.is_empty() && cost.optional.unwrap_or(false) {
            self.selection.cards.clear();
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.mark_cost_paid();
                entry.optional_cost_result = Some(false);
            }
            return self.finalize_choice(gs, context);
        }
        if card_ids.len() < count {
            return Err("Not enough valid cards to reveal for cost".to_string());
        }
        if let Some(entry) = gs.ability_queue.current_entry_mut() {
            entry.mark_cost_paid();
            entry.optional_cost_result = Some(!card_ids.is_empty());
        }
        if !card_ids.is_empty() {
            let card_db = &gs.card_database;
            let names: Vec<String> = card_ids
                .iter()
                .filter_map(|id| card_db.get_card(*id))
                .map(|c| c.name.to_string())
                .collect();
            let turn = gs.turn_number;
            let player_num = if core::ptr::eq(gs.resolve_target_player("self"), &gs.player1) {
                1
            } else {
                2
            };
            if !names.is_empty() {
                gs.push_rule_log_fmt(format_args!(
                    "[Turn {}] P{} [[log_reveal_cost]]: {}",
                    turn,
                    player_num,
                    names.join(", ")
                ));
            }
        }
        let cost_source = gs.current_ability_source_card_id();
        let cost_owner: Option<u8> = if core::ptr::eq(gs.resolve_target_player("self"), &gs.player1)
        {
            Some(0)
        } else {
            Some(1)
        };
        for card_id in &card_ids {
            gs.push_revealed_card(*card_id, cost_source, false, cost_owner, "cost");
            gs.push_revealed_cost_card(*card_id, cost_source, false, cost_owner, "cost");
        }
        self.selection.cards = card_ids.into();
        if self
            .answering_effect(gs)
            .is_some_and(util::effect_uses_selected_cards)
        {
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.effect_started = true;
            }
            // Storing the effect needs ownership; borrowing it for the test
            // above does not.
            if let Some(effect) = self.answering_effect(gs) {
                gs.ability_queue.set_pending_actions(vec![effect.clone()]);
            }
        }
        self.resume_pending_actions(gs)
    }

    // Q86 / Q122: Handle user's selection from looked_at_cards
    //
    // After the user selects which cards to take, this method:
    //   1. Reveals selected cards if the effect requires it (reveal flag)
    //   2. Delegates to handle_select_cards_looked_at (move_cards.rs)
    //      which moves selected to destination, remainder to discard
    //   3. If max/optional/any_number and more cards can be selected,
    //      re-prompts the user with remaining looked_at_cards
    //
    // Q86: If the selection empties the looked_at set AND the source deck
    //   becomes empty, refresh (Rule 10.2.2.1) fires at next check timing.
    //
    // Q122: If the effect is a rearrangement (look + rearrange on deck),
    //   no refresh happens until cards actually leave the deck zone.
    pub(in crate::ability::choice) fn handle_looked_at_selection(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
        context: &ExecutionContext,
    ) -> Result<(), String> {
        let valid = ctx.mfi(&ctx.indices);

        let select_action_entry = gs
            .ability_queue
            .current_entry()
            .and_then(|e| e.ability.effect.as_ref())
            .and_then(|ef| ef.compound_branch().and_then(|c| c.select_action.clone()))
            .or_else(|| {
                self.owner.executing.as_ref().and_then(|ef| {
                    if ef.action == ActionType::SelectCards {
                        Some(Box::new(ef.clone()))
                    } else {
                        None
                    }
                })
            });
        let is_select_cards = select_action_entry
            .as_ref()
            .map(|sa| sa.action == ActionType::SelectCards)
            .unwrap_or(false);

        if select_action_entry
            .as_ref()
            .and_then(|sa| sa.reveal_any())
            .unwrap_or(false)
        {
            self.reveal_selected_looked_at(gs, &valid);
        }

        if is_select_cards {
            self.handle_select_cards_looked_at(gs, &valid, None, None)?;

            if matches!(self.awaiting.choice, Some(Choice::SelectTarget { ref target, .. }) if super::enums::SelectTargetKind::from_str(target) == Some(super::enums::SelectTargetKind::Order))
            {
                return Ok(());
            }

            if !valid.is_empty() {
                let sa = select_action_entry.as_ref();
                let any_number = sa.and_then(|s| s.any_number_any()).unwrap_or(false);
                let is_max = sa.and_then(|s| s.max).unwrap_or(false);
                let is_optional = sa.and_then(|s| s.optional).unwrap_or(false);
                let json_count = sa.and_then(|s| s.count).unwrap_or(1).usize_count();
                let selected_count = valid.len();
                let remaining = gs.looked_at_cards.len();

                // Cumulative across repeated answers: the mover has already counted
                // this answer into `carried.looked_at_taken`, so `taken` is the TOTAL
                // handed over so far, not this answer alone.
                let taken = self.carried.looked_at_taken;
                // 「好きな枚数」 caps at the whole look window (taken + what is left),
                // not `ctx.count` measured against a single answer.
                let max_count = if any_number {
                    taken + remaining
                } else {
                    ctx.count
                };
                let can_reprompt =
                    is_max || is_optional || any_number || json_count > selected_count;
                if can_reprompt && max_count > taken && remaining > 0 {
                    let remaining_max = max_count - taken;
                    let ct = ctx.card_type.clone();
                    // Compute filtered_indices for remaining looked_at cards
                    let remaining_indices: Vec<usize> = {
                        let card_db = &gs.card_database;
                        let filter = sa.map(|s| super::util::CardFilter::from_effect(s));
                        match filter {
                            Some(ref f) if f.has_filter() => gs
                                .looked_at_cards
                                .iter()
                                .enumerate()
                                .filter(|&(_, &cid)| f.matches(card_db, cid, false))
                                .map(|(i, _)| i)
                                .collect(),
                            _ => (0..gs.looked_at_cards.len()).collect(),
                        }
                    };
                    self.capture_choice_owner();
                    self.awaiting.choice = Some(
                        Choice::select_cards(
                            Zone::LookedAt.to_str(),
                            remaining_max,
                            format!("Select up to {} more card(s) from the {} remaining looked-at cards", remaining_max, remaining),
                            // 「2枚まで」 is an upper bound, so a player who has taken
                            // fewer must be allowed to stop — otherwise a partial take
                            // is trapped until the cap is filled.
                            is_optional || any_number || is_max,
                        )
                        .description_ja(Some(format!(
                            "残り{}枚の確認済みカードから最大{}枚選択",
                            remaining, remaining_max
                        )))
                        .card_type(ct)
                        .cost_limit(
                            sa.and_then(|s| s.cost_limit_any()),
                            sa.and_then(|s| s.cost_limit_operator_any()).map(|s| s.to_string()),
                        )
                        .group(sa.as_ref().and_then(|s| s.group_names_any()).and_then(|v| v.first().cloned()))
                        .characters(sa.and_then(|s| s.characters_any().cloned()))
                        .filtered_indices(Some(remaining_indices))
                        .build(),
                    );
                    self.in_flight.execution_context = context.clone();
                    return Ok(());
                }
            }

            // Nothing re-prompted above, but the MOVER may still have staged a
            // choice of its own — the 「select up to N more」 prompt, reached when
            // `can_reprompt` above was narrower than the mover's own test. That
            // is a LIVE prompt, and falling through to `finalize_choice` clears
            // it: `should_preserve` only holds for a looked_at choice with queued
            // sequential actions. The remainder would then be left sitting in
            // `gs.looked_at_cards` — in no zone, yet still readable as shared
            // state by the next look-based card and by the opponent.
            //
            // The test is safe in both directions because the mover clears
            // `awaiting.choice` itself on every path that does NOT stage one
            // (return-to-origin, the looked_at intermediate leg, and the final
            // `self.awaiting.choice = None`), so `is_some()` here means exactly
            // "the mover left a prompt standing".
            if self.awaiting.choice.is_some() {
                log::debug!(
                    "[CHOICE] looked_at: keeping the prompt staged by handle_select_cards_looked_at instead of finalizing"
                );
                return Ok(());
            }
        } else {
            self.handle_select_cards_looked_at(
                gs,
                &valid,
                ctx.destination.clone(),
                ctx.discard_remaining,
            )?;
            if self.awaiting.choice.is_some() {
                log::debug!(
                    "[CHOICE] looked_at: keeping the prompt staged by handle_select_cards_looked_at instead of finalizing"
                );
                return Ok(());
            }
        }
        self.finalize_choice(gs, context)
    }

}
