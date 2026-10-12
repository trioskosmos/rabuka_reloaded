use super::*;

impl super::resolver::AbilityResolver {
    /// LiveCardZone effect path: move validated cards to the destination and
    /// accumulate them into `selected_cards` / recently-moved.
    pub(in crate::ability::choice) fn move_live_card_zone_cards(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
        target_player_id: &Option<String>,
        card_db: &crate::card::CardDatabase,
        validate_card: &mut impl FnMut(i16) -> bool,
    ) {
        // Neither of these needs to be an owned `String`. `target_player_id`
        // is a borrowed parameter and `ctx.destination` borrows the context,
        // so neither touches the game state; building `tgt` as a String here
        // was two heap allocations to satisfy a `&str` parameter. Only the
        // fallback has to read through `gs`, and that one copy is what
        // releases the borrow for the mutable player lookup below.
        let tgt = target_player_id.as_deref().unwrap_or("self");
        let dst_str: Cow<'_, str> = match ctx.destination.as_deref() {
            Some(d) => Cow::Borrowed(d),
            None => Cow::Owned(
                gs.entry_destination()
                    .unwrap_or_else(|| Zone::Discard.to_str())
                    .to_string(),
            ),
        };
        let player = gs.resolve_target_player_mut(tgt);
        let card_ids: Vec<i16> = ctx.mfi(&ctx.indices)
            .iter()
            .filter_map(|&i| player.live_card_zone.cards.get(i).copied())
            .filter(|&cid| validate_card(cid))
            .collect();
        let (moved, _, _) = util::move_cards(
            player,
            &card_ids,
            Zone::LiveCardZone.to_str(),
            &dst_str,
            None,
            card_db,
        );
        if moved > 0 {
            for &cid in &card_ids {
                if !self.selection.cards.contains(&cid) {
                    self.selection.cards.push(cid);
                }
            }
            gs.set_recently_moved_batch(card_ids.into(), Some(Zone::LiveCardZone.to_str()));
        }
    }

    /// LiveCardZone select-action path: take chosen cards out of the zone into
    /// `selected_cards` without moving them elsewhere.
    pub(in crate::ability::choice) fn take_live_card_zone_selection(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
        target_player_id: &Option<String>,
    ) {
        let target = target_player_id
            .clone()
            .unwrap_or_else(|| "self".to_string());
        let player = gs.resolve_target_player_mut(&target);
        let mapped_indices = ctx.mfi(&ctx.indices);
        let mut cards: Vec<i16> = Vec::new();
        for &i in mapped_indices.iter() {
            if i < player.live_card_zone.cards.len() {
                let cid = player.live_card_zone.cards[i];
                if !self.selection.cards.contains(&cid) {
                    self.selection.cards.push(cid);
                    cards.push(cid);
                }
            }
        }
        player.live_card_zone.cards.retain(|c| !cards.contains(c));
        log::debug!(
            "[LIVE_CARD_SELECTION] selected_cards={:?} removed from live_card_zone",
            self.selection.cards
        );
    }

    /// UnderMember effect path: move validated cards, accumulate movement, and
    /// re-prompt for any_number selections while cards remain. Returns true
    /// when a re-prompt was issued (caller must return early).
    pub(in crate::ability::choice) fn handle_under_member_zone(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
        target_player_id: &Option<String>,
        validate_card: &mut impl FnMut(i16) -> bool,
    ) -> Result<bool, String> {
        let edst = gs.entry_destination().map(|s| s.to_string());
        let dst_str = ctx
            .destination
            .clone()
            .or(edst)
            .unwrap_or_else(|| Zone::EnergyDeck.to_str().to_string());
        let tgt = target_player_id
            .clone()
            .unwrap_or_else(|| "self".to_string());
        let moved = self.move_from_under_member(
            gs,
            &ctx.indices,
            validate_card,
            &dst_str,
            &tgt,
        )?;
        if !moved.is_empty() {
            self.selection.moved_cards.extend(moved.iter().copied());
            // Accumulate across any_number re-prompts unconditionally
            // (same card ID can appear multiple times under a member).
            gs.accumulate_recently_moved(&moved);
        }
        // any_number re-prompt: after each selection, show remaining cards
        // Empty indices = player chose to stop selecting (skip).
        if ctx.indices.is_empty() {
            return Ok(false);
        }
        if ctx.count == 0 && ctx.allow_skip {
            let player = gs.resolve_target_player_mut(&tgt);
            let mut remaining_idxs: Vec<usize> = Vec::new();
            let mut global_idx = 0;
            for si in 0..3 {
                for &cid in &player.stage.under_cards[si] {
                    if validate_card(cid) {
                        remaining_idxs.push(global_idx);
                    }
                    global_idx += 1;
                }
            }
            if !remaining_idxs.is_empty() {
                self.awaiting.choice = Some(
                    Choice::select_cards(
                        Zone::UnderMember.to_str(),
                        0,
                        "Select more energy cards (or skip to finish)",
                        true,
                    )
                    .description_ja(Some(
                        "エネルギーカードをさらに選択（スキップで終了）".to_string(),
                    ))
                    .card_type(Some("energy_card".to_string()))
                    .target_player_id(Some(tgt.clone()))
                    .filtered_indices(Some(remaining_idxs))
                    .build(),
                );
                self.store_pending_choice(gs);
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(in crate::ability::choice) fn handle_stage_selection(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
        validate_card: &mut impl FnMut(i16) -> bool,
    ) -> Result<(), String> {
        if ctx.is_select_action {
            if ctx.indices.is_empty() {
                gs.ability_queue.take_pending_actions();
                self.selection.cards = SmallVec::new();
                // For under_member optional skip, mark no move
                if gs.ability_queue.current_entry().is_some_and(|e| {
                    e.ability.effect.as_ref().is_some_and(|eff| {
                        eff.source
                            .as_ref()
                            .is_some_and(|z| z.to_str() == "under_member")
                            || eff
                                .compound_branch()
                                .and_then(|c| c.primary_effect.as_ref())
                                .is_some_and(|pe| {
                                    pe.source
                                        .as_ref()
                                        .is_some_and(|z| z.to_str() == "under_member")
                                })
                    })
                }) {
                    self.carried.last_move_moved_any = Some(false);
                    self.selection.moved_cards.clear();
                    gs.clear_recently_moved_batch();
                }
            log::trace!("[CHOICE] stage selected: no selection, cleared pending commands");
            }
            let stage_indices = ctx.mfi(&ctx.indices);
            log::trace!(
                "[CHOICE] stage index mapping: indices={:?} mapped={:?} filtered={:?} stage={:?}",
                ctx.indices,
                stage_indices,
                ctx.filtered_indices,
                gs.resolve_target_player(ctx.target_player_id.as_deref().unwrap_or("self"))
                    .stage
                    .stage
            );
            let player =
                gs.resolve_target_player_mut(ctx.target_player_id.as_deref().unwrap_or("self"));
            let mut cards: Vec<i16> = Vec::new();
            for &idx in stage_indices.iter() {
                if idx < 3 && player.stage.stage[idx] != -1 {
                    let cid = player.stage.stage[idx];
                    if validate_card(cid) {
                        cards.push(cid);
                    } else {
                        log::debug!("[SELECT_STAGE] validate failed cid={} stage idx {}", cid, idx);
                    }
                } else {
                    log::debug!("[SELECT_STAGE] empty slot idx {}", idx);
                }
            }
            for &cid in &cards {
                if !self.selection.cards.contains(&cid) {
                    self.selection.cards.push(cid);
                }
            }
            // Dispatch on the DECLARED intent of this prompt. Every producer
            // of an is_select_action Stage choice declares one; there is no
            // guess-based fallback.
            log::debug!(
                "[CHOICE] source={:?} stage selection: intent={:?} mapped_indices={:?} cards={:?} effect_started={}",
                self.session.activating_card_id,
                self.awaiting.stage_select_intent,
                stage_indices,
                cards,
                gs.ability_queue
                    .current_entry()
                    .is_some_and(|e| e.effect_started)
            );
            match self.awaiting.stage_select_intent.take() {
                Some(crate::ability::types::StageSelectIntent::ChangeStateWait) => {
                    if !cards.is_empty()
                        && gs
                            .ability_queue
                            .current_entry()
                            .is_some_and(|e| !e.effect_started)
                    {
                        for &cid in &cards {
                            gs.set_orientation(cid, "wait");
                            gs.last_cost_waited_members.push(cid);
                        }
                    }
                    // Effect phase: the stored ChangeState pending-action
                    // re-applies with wait-immunity filtering.
                }
                Some(crate::ability::types::StageSelectIntent::UnderMemberMove) => {
                    if !cards.is_empty() {
                        let entry_target = gs
                            .ability_queue
                            .current_entry()
                            .and_then(|e| e.ability.effect.as_ref())
                            .and_then(|eff| eff.target.as_deref())
                            .unwrap_or("self")
                            .to_string();
                        let mut moved: Vec<i16> = Vec::new();
                        for &mid in &cards {
                            if let Some(idx) = gs
                                .resolve_target_player(&entry_target)
                                .stage
                                .stage
                                .iter()
                                .position(|&id| id == mid)
                            {
                                let mut m = crate::ability::effects::move_cards::drain_under_cards_to_energy_zone(
                                    gs, &entry_target, idx,
                                );
                                moved.append(&mut m);
                            }
                        }
                        if !moved.is_empty() {
                            self.selection.moved_cards.extend(moved.iter().copied());
gs.set_recently_moved_batch(moved.clone().into(), Some("under_member"));
                            self.carried.last_move_moved_any = Some(true);
                        } else {
                            self.carried.last_move_moved_any = Some(false);
                        }
                        log::debug!(
                            "[UNDER_MEMBER_VIA_STAGE] moved {} energies for members {:?}",
                            moved.len(),
                            cards
                        );
                    }
                    return Ok(());
                }
                Some(crate::ability::types::StageSelectIntent::CollectTargets) => {
                    return Ok(());
                }
                Some(crate::ability::types::StageSelectIntent::PlaceEnergyDeckUnder) => {
                    if let Some(effect) = gs.entry_effect().cloned() {
                        let target = effect
                            .target
                            .as_deref()
                            .unwrap_or("self")
                            .to_string();
                        let count = effect.count.unwrap_or(1);
                        let mut placed = Vec::new();
                        {
                            let player = gs.resolve_target_player_mut(&target);
                            if let Some(&member) = cards.first() {
                                if let Some(idx) =
                                    player.stage.stage.iter().position(|&id| id == member)
                                {
                                    let area = crate::ability::util::pos_to_area(idx);
                                    for _ in 0..count {
                                        if let Some(energy) = player.energy_deck.draw() {
                                            player.stage.place_under_card(area, energy);
                                            placed.push(energy);
                                        } else {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        if !placed.is_empty() {
                            let player_id = gs.resolve_target_player(&target).id.clone();
                            for &energy in &placed {
                                gs.push_movement_event(
                                    energy,
                                    "energy_deck",
                                    "under_member",
                                    cards.first().copied(),
                                    &player_id,
                                    false,
                                );
                            }
                            self.selection.moved_cards = placed.clone().into();
                            gs.set_recently_moved_batch(placed.into(), Some("under_member"));
                            gs.recalculate_constants();
                        }
                        self.clear_choice_state_and_resume(gs)?;
                    }
                    return Ok(());
                }
                None => {}
            }
        } else {
            let edst = gs.entry_destination().map(|s| s.to_string());
            let dst_str = ctx
                .destination
                .clone()
                .or(edst)
                .unwrap_or_else(|| Zone::Discard.to_string());
            let card_db = gs.card_database.clone();
            let player =
                gs.resolve_target_player_mut(ctx.target_player_id.as_deref().unwrap_or("self"));
            let card_ids = util::resolve_indices_to_ids(player, Zone::Stage.to_str(), &ctx.indices);
            let valid_ids: Vec<i16> = card_ids
                .into_iter()
                .filter(|&cid| validate_card(cid))
                .collect();
            let mut last_vacated = None;
            for &cid in &valid_ids {
                if let Some(pos) = player.stage.stage.iter().position(|&x| x == cid) {
                    last_vacated = Some(pos);
                }
            }
            let owner_id = player.id.clone();
            let (moved_count, member_under, energy_under) = util::move_cards(
                player,
                &valid_ids,
                Zone::Stage.to_str(),
                &dst_str,
                None,
                &card_db,
            );
            if moved_count > 0 {
                if let Some(pos) = last_vacated {
                    gs.last_vacated_stage_area = Some(pos.u8_count());
                }
                self.selection.cards = valid_ids.clone().into();
                self.selection.moved_cards = valid_ids.clone().into();
                // Record the member's own movement
                // (stage -> destination). A discard
                // from the stage is a placement to
                // the waitroom (Discard and Waitroom
                // are the same zone here), so the
                // 「メンバーが控え室に置かれたとき」
                // watchers must fire. `effect_only`
                // follows the phase: a cost-phase
                // discard is not an effect, an
                // effect-phase discard is.
                let effect_only = gs
                    .ability_queue
                    .current_entry()
                    .is_some_and(|e| e.effect_started);
                for &cid in &valid_ids {
                    gs.push_movement_event(
                        cid,
                        Zone::Stage.to_str(),
                        &dst_str,
                        gs.activating_card,
                        owner_id.as_str(),
                        effect_only,
                    );
                }
                gs.set_recently_moved_batch(valid_ids.into(), Some("stage"));
            }
            // Rule 10.5.3-10.5.4: the under-cards of
            // every member discarded off the stage are
            // placed to the waitroom / energy deck.
            // Record those placements so the
            // 「…控え室に置かれたとき」 watchers fire.
            gs.record_under_card_recycling(
                owner_id.as_str(),
                &member_under,
                &energy_under,
                gs.activating_card,
            );
        }
        Ok(())
    }

    pub(in crate::ability::choice) fn filter_discard_by_budget(
        &self,
        gs: &GameState,
        ctx: &SelectionContext,
        waitroom_cards: &[i16],
    ) -> (Option<u8>, Vec<usize>) {
        let spent: u8 = self
            .selection.cards
            .iter()
            .filter_map(|&cid| gs.card_database.get_card(cid).and_then(|c| c.cost))
            .sum();
        let remaining_budget = ctx.cost_total.map(|tb| tb.saturating_sub(spent));
        let use_budget_filter = match ctx.cost_total_operator.as_deref() {
            Some(op) => op == "<=",
            None => ctx.cost_total.is_some(),
        };
        let all_idxs: Vec<usize> = if use_budget_filter {
            let rb = remaining_budget.unwrap_or(0);
            waitroom_cards
                .iter()
                .enumerate()
                .filter(|(_, &cid)| {
                    gs.card_database
                        .get_card(cid)
                        .and_then(|c| c.cost)
                        .unwrap_or(99)
                        <= rb
                })
                .map(|(idx, _)| idx)
                .collect()
        } else {
            (0..waitroom_cards.len()).collect()
        };
        (remaining_budget, all_idxs)
    }

    pub(in crate::ability::choice) fn handle_discard_selection(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
        context: &ExecutionContext,
        _validate_card: &mut impl FnMut(i16) -> bool,
    ) -> Result<(), String> {
        let mapped_indices = ctx.mfi(&ctx.indices);
        // 「3枚まで選び」 is a CEILING, not a quota: a player who picks fewer is
        // finished, not chased for the rest of `ctx.count`. The printed 「まで」 is
        // the `max` flag, and it lives on the queued select action — not on the
        // context, which only carries the numeric count.
        let printed_max = gs
            .ability_queue
            .current_entry()
            .and_then(|e| e.ability.effect.as_ref())
            .map(|ef| {
                // Single-effect shape: `compound.select_action`, or the effect itself.
                let c = ef.compound_branch();
                ef.max.unwrap_or(false)
                    || c.and_then(|c| c.select_action.as_ref())
                        .is_some_and(|sa| sa.max.unwrap_or(false))
                    // Sequential shape (「…選び、デッキの上に置く」 parses as
                    // select → move): the 「まで」 lives on the select STEP.
                    || c.and_then(|c| c.actions.as_ref())
                        .is_some_and(|steps| steps.iter().any(|a| a.max.unwrap_or(false)))
            })
            .unwrap_or(false);
        let picked_now = mapped_indices.len();
        log::debug!(
            "[SELECT_ANSWER] zone=discard select_action={} picked={} selected_so_far={} count={} max={} allow_skip={}",
            ctx.is_select_action,
            picked_now,
            self.selection.cards.len(),
            ctx.count,
            printed_max,
            ctx.allow_skip
        );
        if ctx.is_select_action {
            let target = ctx
                .target_player_id
                .as_deref()
                .unwrap_or("self")
                .to_string();
            let player = gs.resolve_target_player_mut(&target);
            let mut cards: Vec<i16> = Vec::new();
            for &i in mapped_indices.iter() {
                if i < player.waitroom.cards.len() {
                    let cid = player.waitroom.cards[i];
                    if !self.selection.cards.contains(&cid) {
                        self.selection.cards.push(cid);
                        cards.push(cid);
                    }
                }
            }
            let selected_so_far = self.selection.cards.len();
            // A 「3枚まで」 re-prompt is an OFFER, not a quota: the player may stop
            // after a partial pick. The re-prompt still appears (the remaining cards
            // are still legally selectable) — it just becomes skippable, which is
            // the difference between 「3枚まで」 (up to 3) and 「3枚必ず」 (exactly 3).
            let may_reprompt =
                ctx.count > 0 && !mapped_indices.is_empty() && selected_so_far < ctx.count;
            let skippable = ctx.allow_skip || printed_max;
            log::debug!(
                "[SELECT_REPROMPT] selected_so_far={} count={} max={} skip={} -> {}",
                selected_so_far,
                ctx.count,
                printed_max,
                skippable,
                if may_reprompt { "offer the rest" } else { "finalize" }
            );
            if may_reprompt {
                let remaining = ctx.count - selected_so_far;
                let waitroom_cards: Vec<i16> = {
                    let p = gs.resolve_target_player_mut(&target);
                    p.waitroom.cards.to_vec()
                };
                let filtered_idxs: Vec<usize> = (0..waitroom_cards.len())
                    .filter(|&i| !self.selection.cards.contains(&waitroom_cards[i]))
                    .collect();
                let remaining_count = filtered_idxs.len();
                let fi = if filtered_idxs.is_empty() {
                    None
                } else {
                    Some(filtered_idxs)
                };
                let desc = format!(
                    "Select {} more card(s) from discard from {} remaining",
                    remaining, remaining_count
                );
                let desc_ja = format!(
                    "控え室から残り{}枚中さらに{}枚選択",
                    remaining_count, remaining
                );
                self.awaiting.choice = Some(
                    self.build_reprompt(
                        ctx,
                        Zone::Discard.to_str(),
                        remaining,
                        desc,
                        desc_ja,
                        skippable,
                        fi,
                        Some(target.clone()),
                        ctx.cost_total,
                        ctx.cost_total_operator.clone(),
                    )
                    .is_select_action(true)
                    .build(),
                );
                self.store_pending_choice(gs);
                return Ok(());
            }
            return self.finalize_choice(gs, context);
        } else {
            // An empty pick (index outside filtered_indices, or nothing mapped)
            // must never satisfy a MANDATORY (allow_skip=false) selection:
            // silently completing would skip the required move entirely
            // (Q123: 必ず手札に加える必要がある). Re-offer the SAME choice so
            // the player can pick legal targets.
            if mapped_indices.is_empty() && !ctx.allow_skip {
                let target = ctx
                    .target_player_id
                    .clone()
                    .unwrap_or_else(|| "self".to_string());
                self.awaiting.choice = Some(
                    self.build_reprompt(
                        ctx,
                        Zone::Discard.to_str(),
                        ctx.count,
                        format!(
                            "Select {} card(s) from the waiting room",
                            ctx.count
                        ),
                        format!("控え室から{}枚選択", ctx.count),
                        false,
                        None,
                        Some(target),
                        ctx.cost_total,
                        ctx.cost_total_operator.clone(),
                    )
                    .build(),
                );
                self.store_pending_choice(gs);
                return Ok(());
            }
            if !mapped_indices.is_empty() && ctx.count > 0 && mapped_indices.len() < ctx.count {
                let target = ctx
                    .target_player_id
                    .as_deref()
                    .unwrap_or("self")
                    .to_string();
                let waitroom_cards: Vec<i16> = {
                    let p = gs.resolve_target_player_mut(&target);
                    p.waitroom.cards.to_vec()
                };
                let mut filtered_idxs: Vec<usize> = Vec::new();
                if !self.selection.cards.is_empty() {
                    for (idx, cid) in waitroom_cards.iter().enumerate() {
                        if self.selection.cards.contains(cid) && !filtered_idxs.contains(&idx) {
                            filtered_idxs.push(idx);
                        }
                    }
                }
                let current_card_ids: Vec<i16> = mapped_indices
                    .iter()
                    .filter_map(|&idx| {
                        if idx < waitroom_cards.len() {
                            Some(waitroom_cards[idx])
                        } else {
                            None
                        }
                    })
                    .collect();
                for &cid in &current_card_ids {
                    if !self.selection.cards.contains(&cid) {
                        self.selection.cards.push(cid);
                    }
                }
                self.execute_selected_cards_from_zone(
                    gs,
                    Zone::Discard.to_str(),
                    &mapped_indices,
                    ctx.card_type.as_deref(),
                    ctx.cost_limit,
                    ctx.cost_limit_operator.as_deref(),
                    ctx.cost_total,
                    ctx.cost_total_operator.as_deref(),
                    ctx.group.as_deref(),
                    ctx.characters.as_ref(),
                    ctx.target_player_id.as_deref(),
                )?;
                if self.in_flight.sub_choice_created {
                    self.in_flight.sub_choice_created = false;
                    let remaining = ctx.count - mapped_indices.len();
                    if remaining > 0 {
                        let waitroom_cards: Vec<i16> = {
                            let p = gs.resolve_target_player_mut(&target);
                            p.waitroom.cards.to_vec()
                        };
                        let (remaining_budget, all_idxs) =
                            self.filter_discard_by_budget(gs, ctx, &waitroom_cards);
                        if !all_idxs.is_empty() || ctx.allow_skip {
                            let desc = format!(
                                "Select {} more card(s) from discard{}",
                                remaining,
                                if ctx.allow_skip {
                                    " (or skip to finish)"
                                } else {
                                    ""
                                }
                            );
                            let desc_ja = format!(
                                "控え室からさらに{}枚選択{}",
                                remaining,
                                if ctx.allow_skip {
                                    "（スキップで終了）"
                                } else {
                                    ""
                                }
                            );
                            let fi = if all_idxs.is_empty() {
                                Some(vec![])
                            } else {
                                Some(all_idxs)
                            };
                            let reprompt = self
                                .build_reprompt(
                                    ctx,
                                    Zone::Discard.to_str(),
                                    remaining,
                                    desc,
                                    desc_ja,
                                    ctx.allow_skip,
                                    fi,
                                    Some(target),
                                    remaining_budget,
                                    Some("<=".to_string()),
                                )
                                .build();
        let pending = gs.ability_queue.take_pending_actions();
                            self.awaiting.reprompt = Some(reprompt);
                            gs.ability_queue.set_pending_actions(pending);
                        }
                    }
                    return Ok(());
                }
                let waitroom_cards: Vec<i16> = {
                    let p = gs.resolve_target_player_mut(&target);
                    p.waitroom.cards.to_vec()
                };
                let (remaining_budget, all_idxs) =
                    self.filter_discard_by_budget(gs, ctx, &waitroom_cards);
                let remaining = ctx.count - mapped_indices.len();
                let desc = format!(
                    "Select {} more card(s) from discard{}",
                    remaining,
                    if ctx.allow_skip {
                        " (or skip to finish)"
                    } else {
                        ""
                    }
                );
                let desc_ja = format!(
                    "控え室からさらに{}枚選択{}",
                    remaining,
                    if ctx.allow_skip {
                        "（スキップで終了）"
                    } else {
                        ""
                    }
                );
                let fi = if all_idxs.is_empty() {
                    Some(vec![])
                } else {
                    Some(all_idxs)
                };
                self.awaiting.choice = Some(
                    self.build_reprompt(
                        ctx,
                        Zone::Discard.to_str(),
                        remaining,
                        desc,
                        desc_ja,
                        ctx.allow_skip || printed_max,
                        fi,
                        Some(target),
                        remaining_budget,
                        Some("<=".to_string()),
                    )
                    .build(),
                );
                self.store_pending_choice(gs);
                return Ok(());
            }
            let target = ctx
                .target_player_id
                .as_deref()
                .unwrap_or("self")
                .to_string();
            let mut all_idxs: Vec<usize> = mapped_indices.to_vec();
            let prev_ids = self.selection.cards.clone();
            if !prev_ids.is_empty() {
                let waitroom_cards: Vec<i16> = {
                    let p = gs.resolve_target_player_mut(&target);
                    p.waitroom.cards.to_vec()
                };
                for (idx, cid) in waitroom_cards.iter().enumerate() {
                    if prev_ids.contains(cid) && !all_idxs.contains(&idx) {
                        all_idxs.push(idx);
                    }
                }
            }
            self.execute_selected_cards_from_zone(
                gs,
                Zone::Discard.to_str(),
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
            self.selection.cards.clear();
            if self.in_flight.sub_choice_created {
                self.store_pending_choice(gs);
            }
        }
        self.handle_selection_epilogue(gs, context)
    }

    pub(in crate::ability::choice) fn handle_selection_epilogue(
        &mut self,
        gs: &mut GameState,
        context: &ExecutionContext,
    ) -> Result<(), String> {
        if gs.ability_queue.has_pending_actions() && self.awaiting.choice.is_none() {
            self.clear_choice_state(gs);
            return self.resume_pending_actions(gs);
        }
        self.finalize_choice(gs, context)
    }

}
