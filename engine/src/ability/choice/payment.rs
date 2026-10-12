use super::*;

impl super::resolver::AbilityResolver {
    /// Indices into `hand` whose cards validate, optionally restricted to one unit.
    /// Unifies the three copy-pasted hand-scan loops in the cost-payment path.
    fn hand_matching_indices(
        hand: &[i16],
        card_db: &crate::card::CardDatabase,
        validate: &mut impl FnMut(i16) -> bool,
        unit: Option<&crate::core::types::ArcStr>,
        require_unit: bool,
    ) -> Vec<usize> {
        (0..hand.len())
            .filter(|i| {
                let cid = hand[*i];
                validate(cid)
                    && unit.as_ref().map_or(!require_unit, |un| {
                        card_db.get_card(cid).and_then(|c| c.unit.as_ref()) == Some(un)
                    })
            })
            .collect()
    }

    /// Move validated hand-cost cards to discard + feed movement tracking.
    /// Returns the newly moved card IDs.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::ability::choice) fn discard_hand_cost_selection(
        &mut self,
        gs: &mut GameState,
        target: &str,
        hand_cards: &[i16],
        cost_hand_indices: &[usize],
        card_db: &crate::card::CardDatabase,
        validate_card: &mut impl FnMut(i16) -> bool,
        count: usize,
    ) -> Vec<i16> {
        let new_card_ids: Vec<i16> = cost_hand_indices
            .iter()
            .filter_map(|&i| {
                if i < hand_cards.len() {
                    Some(hand_cards[i])
                } else {
                    None
                }
            })
            .filter(|&cid| validate_card(cid))
            .collect();
        if new_card_ids.is_empty() {
            return new_card_ids;
        }
        log::trace!(
            "[COST] source={:?} discard intent: required={} cards={:?}",
            self.session.activating_card_id,
            count,
            new_card_ids
        );
        let player = gs.resolve_target_player_mut(target);
        let (_, _, _) = util::move_cards(
            player,
            &new_card_ids,
            Zone::Hand.to_str(),
            Zone::Discard.to_str(),
            None,
            card_db,
        );
        // R1 unification: cost discards are real zone changes and must
        // feed the movement-tracking views (cards_moved_this_turn,
        // turn/batch logs) like every other move. Cost payment ⇒
        // effect_only=false.
        for &cid in &new_card_ids {
            let cause = gs.activating_card;
            gs.push_movement_event(
                cid,
                Zone::Hand.to_str(),
                Zone::Discard.to_str(),
                cause,
                target,
                false,
            );
        }
        gs.mods.last_cost_discard_count += new_card_ids.len().u8_count();
        gs.mods
            .last_cost_moved_card_ids
            .extend(new_card_ids.iter().copied());
        for &cid in &new_card_ids {
            self.selection.moved_cards.push(cid);
            if !self.selection.cards.contains(&cid) {
                self.selection.cards.push(cid);
            }
        }
        log::debug!(
            "[COST] source={:?} discarded cards={:?} total_paid={}",
            self.session.activating_card_id,
            new_card_ids,
            self.selection.moved_cards.len()
        );
        new_card_ids
    }

    /// Issue a hand-cost re-prompt for `remaining` more cards.
    /// Unifies the three build_reprompt call sites (partial, same-unit, any_number).
    #[allow(clippy::too_many_arguments)]
    pub(in crate::ability::choice) fn reprompt_hand_cost(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
        remaining: usize,
        desc: String,
        desc_ja: String,
        allow_skip: bool,
        filtered: Option<Vec<usize>>,
        target: String,
        cost_total: Option<u8>,
        cost_total_operator: Option<String>,
    ) {
        self.awaiting.choice = Some(
            self.build_reprompt(
                ctx,
                Zone::Hand.to_str(),
                remaining,
                desc,
                desc_ja,
                allow_skip,
                filtered,
                Some(target),
                cost_total,
                cost_total_operator,
            )
            .build(),
        );
        self.store_pending_choice(gs);
    }

    /// Full hand cost-payment flow (fixed-count, same-unit, any_number).
    /// Always handles the choice when called — the caller must return early.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::ability::choice) fn handle_hand_cost_payment(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
        indices: &[usize],
        count: usize,
        allow_skip: bool,
        cost_total: Option<u8>,
        cost_total_operator: Option<String>,
        target_player_id: &Option<String>,
        card_db: &crate::card::CardDatabase,
        validate_card: &mut impl FnMut(i16) -> bool,
    ) -> Result<(), String> {
        if !indices.is_empty() {
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.optional_cost_result = Some(true);
            }
            // Outside the `current_entry_mut` borrow: the payer is the entry's own card, and
            // PL!-pb2-021-N's 「…コストを支払うため」 reads this history.
            gs.record_cost_paid();
        }
        let target = target_player_id
            .clone()
            .unwrap_or_else(|| "self".to_string());
        let hand_cards: Vec<i16> = {
            let p = gs.resolve_target_player_mut(&target);
            p.hand.cards.to_vec()
        };
        let cost_hand_indices = ctx.mfi(indices);
        let new_card_ids = self.discard_hand_cost_selection(
            gs,
            &target,
            &hand_cards,
            &cost_hand_indices,
            card_db,
            validate_card,
            count,
        );
        if new_card_ids.is_empty() {
            if !self.selection.moved_cards.is_empty() {
                log::debug!("[COST] source={:?} hand payment complete: cards={:?} optional_cost_result=true", self.session.activating_card_id, self.selection.moved_cards);
                gs.mods.last_cost_discard_count = self.selection.moved_cards.len().u8_count();
                gs.mods.last_cost_moved_card_ids = self.selection.moved_cards.clone();
                gs.set_recently_moved_batch(self.selection.moved_cards.clone(), Some("hand"));
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.mark_cost_paid();
                    entry.optional_cost_result = Some(true);
                }
                gs.record_cost_paid();
            } else if allow_skip {
                log::debug!("[COST] source={:?} hand payment skipped: no cards moved, optional_cost_result=false", self.session.activating_card_id);
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.mark_cost_paid();
                    entry.optional_cost_result = Some(false);
                }
            }
            self.awaiting.choice = None;
            return Ok(());
        }
        // Rule 9.4.2.3: cost must be paid in full. Once a player selects the
        // first card for a fixed-count cost, they are committed — no bail-out.
        if count > 0 && new_card_ids.len() < count {
            let remaining = count - new_card_ids.len();
            let hand_now: Vec<i16> = {
                let p = gs.resolve_target_player_mut(&target);
                p.hand.cards.to_vec()
            };
            let same_unit_name = gs
                .entry_cost()
                .and_then(|c| c.same_unit_name_any())
                .unwrap_or(false);
            let same_unit_filter = if same_unit_name {
                self.selection.moved_cards
                    .first()
                    .and_then(|&cid| card_db.get_card(cid))
                    .and_then(|c| c.unit.clone())
            } else {
                None
            };
            let available_idxs =
                Self::hand_matching_indices(&hand_now, card_db, validate_card, same_unit_filter.as_ref(), false);
            let fi = if available_idxs.is_empty() {
                None
            } else {
                Some(available_idxs)
            };
            self.reprompt_hand_cost(
                gs,
                ctx,
                remaining,
                format!("Select {} more card(s) from hand for cost", remaining),
                format!("コストとして手札からさらに{}枚選択", remaining),
                false, // Rule 9.4.2.3: no bail-out once committed
                fi,
                target.clone(),
                cost_total,
                cost_total_operator.clone(),
            );
            return Ok(());
        }
        // Same-unit cost: after selecting the first card, re-prompt
        // for remaining cards filtered to only the chosen unit.
        if count > 0 && new_card_ids.len() == count {
            if let Some(cost) = gs.entry_cost() {
                if cost.same_unit_name_any().unwrap_or(false) {
                    let total_needed = cost.count.unwrap_or(1).usize_count();
                    let total_moved = self.selection.moved_cards.len();
                    if total_moved < total_needed {
                        let remaining = total_needed - total_moved;
                        let hand_now: Vec<i16> = {
                            let p = gs.resolve_target_player_mut(&target);
                            p.hand.cards.to_vec()
                        };
                        let unit_name = self
                            .selection.moved_cards
                            .first()
                            .and_then(|&cid| card_db.get_card(cid))
                            .and_then(|c| c.unit.clone());
                        let same_unit_idxs = Self::hand_matching_indices(
                            &hand_now,
                            card_db,
                            validate_card,
                            unit_name.as_ref(),
                            true,
                        );
                        if same_unit_idxs.is_empty() {
                            return Err(format!(
                                "Cannot pay cost: not enough cards with unit '{}' in hand",
                                unit_name.unwrap_or_default()
                            ));
                        }
                        self.reprompt_hand_cost(
                            gs,
                            ctx,
                            remaining,
                            format!(
                                "Select {} more card(s) with the same unit name",
                                remaining
                            ),
                            format!("同じユニット名のカードをさらに{}枚選択", remaining),
                            false,
                            Some(same_unit_idxs),
                            target.clone(),
                            cost_total,
                            cost_total_operator.clone(),
                        );
                        return Ok(());
                    }
                }
            }
        }
        // Re-prompt for any_number costs: after each selection, ask
        // if the player wants to select more cards or skip to finish.
        let cost_max_cap = gs
            .entry_cost()
            .and_then(|c| c.count)
            .map(usize::from)
            .unwrap_or(usize::MAX);
        if count == 0 && allow_skip && self.selection.moved_cards.len() < cost_max_cap {
            log::debug!("[COST] source={:?} choose more cards or finish: added={} paid_cards={:?} cap={}", self.session.activating_card_id, new_card_ids.len(), self.selection.moved_cards, cost_max_cap);
            let hand_now: Vec<i16> = {
                let p = gs.resolve_target_player_mut(&target);
                p.hand.cards.to_vec()
            };
            let available_idxs =
                Self::hand_matching_indices(&hand_now, card_db, validate_card, None, false);
            let fi = if available_idxs.is_empty() {
                None
            } else {
                Some(available_idxs)
            };
            let desc = if fi.as_ref().map_or(0, |v| v.len()) > 0 {
                "Select more card(s) from hand for cost (or skip to finish)".to_string()
            } else {
                "No more matching cards in hand (skip to finish)".to_string()
            };
            let desc_ja = if fi.as_ref().map_or(0, |v| v.len()) > 0 {
                "コストとして手札からさらに選択（スキップで終了）".to_string()
            } else {
                "手札に一致するカードがありません（スキップで終了）".to_string()
            };
            self.reprompt_hand_cost(
                gs,
                ctx,
                0,
                desc,
                desc_ja,
                true,
                fi,
                target.clone(),
                cost_total,
                cost_total_operator.clone(),
            );
            return Ok(());
        }
        log::debug!(
            "[COST] source={:?} finishing hand payment: cards={} deferred_costs={}",
            self.session.activating_card_id,
            self.selection.moved_cards.len(),
            self.awaiting.deferred_costs.len()
        );
        self.pay_deferred_costs(gs)?;
        let final_count = self.selection.moved_cards.len().u8_count();
        gs.mods.last_cost_discard_count = final_count;
        gs.mods.last_cost_moved_card_ids = self.selection.moved_cards.clone();
        gs.set_recently_moved_batch(self.selection.moved_cards.clone(), Some("hand"));
        if let Some(entry) = gs.ability_queue.current_entry_mut() {
            entry.mark_cost_paid();
        }
        self.awaiting.choice = None;
        Ok(())
    }

    /// Energy cost-payment flow. Always handles the choice when called.
    pub(in crate::ability::choice) fn handle_energy_cost_payment(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
        indices: &[usize],
        cost_total: Option<u8>,
        cost_total_operator: Option<String>,
        target_player_id: &Option<String>,
    ) -> Result<(), String> {
        let count_paid = indices.len();
        if count_paid > 0 {
            let player =
                gs.resolve_target_player_mut(target_player_id.as_deref().unwrap_or("self"));
            player.energy_zone.pay_energy(count_paid.u8_count())?;
            gs.mods.last_cost_energy_count += count_paid.u8_count();
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.mark_cost_paid();
                entry.optional_cost_result = Some(true);
            }
            gs.record_cost_paid();
            // Energy-count constants (「エネルギーがちょうどN枚あるかぎり」)
            // are live state — re-evaluate right after the payment.
            gs.recalculate_constants();
        }
        let energy_left = {
            let player =
                gs.resolve_target_player(target_player_id.as_deref().unwrap_or("self"));
            player.energy_zone.active_energy_count
        };
        if count_paid > 0 && energy_left > 0 {
            // Active prefix only — waited energy can never pay (shared helper
            // with the cost-side menu so the offerable set can't diverge).
            let efi = {
                let player =
                    gs.resolve_target_player(target_player_id.as_deref().unwrap_or("self"));
                crate::ability::util::active_energy_indices(player)
            };
            let target = target_player_id
                .clone()
                .unwrap_or_else(|| "self".to_string());
            self.awaiting.choice = Some(
                self.build_reprompt(
                    ctx,
                    Zone::Energy.to_str(),
                    0,
                    format!(
                        "Select energy card to pay (active: {}). Skip when done",
                        energy_left
                    ),
                    format!(
                        "エネルギーカードを選択（アクティブ: {}）完了でスキップ",
                        energy_left
                    ),
                    true,
                    Some(efi),
                    Some(target),
                    cost_total,
                    cost_total_operator.clone(),
                )
                .build(),
            );
            self.store_pending_choice(gs);
            return Ok(());
        }
        self.clear_choice_state(gs);
        self.resume_pending_actions(gs)
    }

}
