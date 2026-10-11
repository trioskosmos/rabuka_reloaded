use super::*;

impl super::resolver::AbilityResolver {
    pub(in crate::ability::choice) fn handle_select_target(
        &mut self,
        gs: &mut GameState,
        target: &str,
        selected: &str,
    ) -> Result<(), String> {
        let choice_card_no = gs.entry_choice_card_no();
        let conditional_choice = gs.entry_conditional_choice();
        log::debug!(
            "[HST] target={} selected={} choice_card_no={:?} activating={:?}",
            target,
            selected,
            choice_card_no,
            gs.activating_card
        );

        // choice_card_no-based routing
        match choice_card_no.as_ref() {
            Some(ChoiceRoute::Choice) => {
                if let Some(ConditionalChoice::Effects(all_options)) = conditional_choice {
                    let Ok(idx) = selected.parse::<usize>() else {
                        log::warn!("[HST] non-numeric option index {:?}; rejecting selection", selected);
                        return Err(format!("invalid selection: {:?}", selected));
                    };
                    if idx < all_options.len() {
                        let selected_effect = (*all_options[idx]).clone();
                        let mut remaining = all_options.clone();
                        remaining.remove(idx);
                        // Schedule the selected effect and optional re-prompt
                        // as pending commands. resume_pending_commands runs them
                        // sequentially — after the selected effect completes (and
                        // any sub-choices), the re-prompt command re-enters
                        // execute_choice which sees the updated conditional_choice
                        // enum and creates a new SelectTarget for the
                        // remaining options.
                        let commands = vec![selected_effect];

                        // 「『X』のカード1枚につき、以下から1つを選ぶ」 — one further offering
                        // per remaining qualifying card. The FULL option list is restored,
                        // unlike the any_number branch below: 「同じ選択肢を2回以上選んでも
                        // よい」 explicitly permits picking the same option again, so the
                        // option must still be there for the repeat.
                        let per_card_repeat = gs
                            .ability_queue
                            .current_entry()
                            .is_some_and(|e| e.choice_repeats_remaining > 0);
                        if per_card_repeat {
                            let full = all_options.clone();
                            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                                entry.choice_repeats_remaining -= 1;
                                entry.conditional_choice = Some(ConditionalChoice::Effects(full.clone()));
                            }
                            let desc: Vec<String> = full
                                .iter()
                                .map(|o| {
                                    o.answers_any()
                                        .as_ref()
                                        .map(|a| a.join(", "))
                                        .unwrap_or_else(|| o.text.to_string())
                                })
                                .collect();
                            let desc_en: Vec<String> = full
                                .iter()
                                .map(|o| {
                                    let en = crate::ability::describe::describe_effect_en(o);
                                    if en.trim().is_empty() {
                                        o.answers_any()
                                            .as_ref()
                                            .map(|a| a.join(", "))
                                            .unwrap_or_else(|| o.text.to_string())
                                    } else {
                                        en
                                    }
                                })
                                .collect();
                            let n_full = full.len();
                            log::debug!(
                                "[CHOICE_PER_CARD] re-prompting over {n_full} options; the same \
                                 option may be picked again (同じ選択肢を2回以上選んでもよい)"
                            );
                            self.awaiting.reprompt = Some(Choice::SelectTarget {
                                target: "choice".to_string(),
                                description: desc.join(" / "),
                                description_en: Some(desc_en.join(" / ")),
                                description_ja: Some(desc.join(" / ")),
                                allow_skip: true,
                                options: None,
                            });
                            gs.ability_queue.set_pending_actions(commands);
                            self.awaiting.choice = None;
                            return self.resume_pending_actions(gs);
                        }

                        let wants_re_prompt = !remaining.is_empty()
                            && gs.entry_effect().is_some_and(|eff| {
                                if eff.any_number_any().unwrap_or(false) {
                                    return true;
                                }
                                if let Some(ref alt_cond) = eff.compound_branch().and_then(|c| c.alternative_condition.clone()) {
                                    let ctx =
                                        ConditionContext::with_moved_cards(gs, &self.selection.moved_cards);
                                    ctx.evaluate_condition(alt_cond)
                                        && eff.alternative_count_type_any()
                                            == Some("any_number")
                                } else {
                                    false
                                }
                            });
                        if wants_re_prompt {
                            let desc: Vec<String> = remaining
                                .iter()
                                .map(|o| {
                                    o.answers_any()
                                        .as_ref()
                                        .map(|a| a.join(", "))
                                        .unwrap_or_else(|| o.text.to_string())
                                })
                                .collect();
                            // Keep EN/JA in sync with creation (misc.rs):
                            // EN generated per remaining effect, JA from text.
                            let desc_en: Vec<String> = remaining
                                .iter()
                                .map(|o| {
                                    let en =
                                        crate::ability::describe::describe_effect_en(o);
                                    if en.trim().is_empty() {
                                        o.answers_any()
                                            .as_ref()
                                            .map(|a| a.join(", "))
                                            .unwrap_or_else(|| o.text.to_string())
                                    } else {
                                        en
                                    }
                                })
                                .collect();
                            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                                entry.conditional_choice =
                                    Some(ConditionalChoice::Effects(remaining));
                            }
                            self.awaiting.reprompt = Some(Choice::SelectTarget {
                                target: "choice".to_string(),
                                description: desc.join(" / "),
                                description_en: Some(desc_en.join(" / ")),
                                description_ja: Some(desc.join(" / ")),
                                allow_skip: true,
                                options: None,
                            });
                        }
                        gs.ability_queue.set_pending_actions(commands);
                        self.awaiting.choice = None; // clear stale
                        return self.resume_pending_actions(gs);
                    }
                } else if self.awaiting.choice.is_some() {
                    self.awaiting.choice = None;
                }
                return self.resume_pending_actions(gs);
            }
            Some(ChoiceRoute::ChoiceString) => {
                return self.handle_choice_string_selection(gs, selected, conditional_choice);
            }
            Some(ChoiceRoute::Raw(s)) if s.starts_with("position_change") => {
                log::debug!(
                    "[HPCC_MATCH] routing to handle_position_change_choice: s={} selected={}",
                    s,
                    selected
                );
                return self.handle_position_change_choice(gs, choice_card_no, selected);
            }
            _ => {}
        }

        // target-based routing via typed enum
        // Note: Choice and ChoiceString are handled by choice_card_no above
        match super::enums::SelectTargetKind::from_str(target) {
            Some(super::enums::SelectTargetKind::Choice) => {
                self.clear_choice_state(gs);
                return Ok(());
            }
            Some(super::enums::SelectTargetKind::ChoiceString) => {
                return self.handle_choice_string_store(gs, selected, conditional_choice);
            }
            Some(super::enums::SelectTargetKind::PayOptionalCostSkipOptionalCost) => {
                return self.handle_optional_cost_payment(gs, selected);
            }
            Some(super::enums::SelectTargetKind::PayCostAllDiscard) => {
                return self.handle_pay_cost_all_discard(gs, selected);
            }
            Some(super::enums::SelectTargetKind::DoubleBatonTouch) => {
                return self.handle_double_baton_touch(gs, selected);
            }
            Some(super::enums::SelectTargetKind::PrimaryAlternative) => {
                return self.handle_primary_alternative(gs, selected);
            }
            Some(super::enums::SelectTargetKind::ApplyReplacement) => {
                return self.handle_apply_replacement(gs, selected);
            }
            Some(super::enums::SelectTargetKind::ChooseRequiredHearts) => {
                gs.prohibition_effects
                    .push(format!("chosen_required_hearts:{}", selected));
                self.clear_choice_state(gs);
                return Ok(());
            }
            Some(super::enums::SelectTargetKind::PositionDestination) => {
                return self.handle_position_destination(gs, selected);
            }
            Some(super::enums::SelectTargetKind::HeartColor) => {
                return self.handle_heart_color_selection(gs, selected);
            }
            Some(super::enums::SelectTargetKind::ChoiceType) => {
                self.clear_choice_state(gs);
                return Ok(());
            }
            Some(super::enums::SelectTargetKind::ChoiceCondition) => {
                return self.handle_choice_condition(gs, selected);
            }
            Some(super::enums::SelectTargetKind::ConditionalOptional) => {
                return self.handle_conditional_optional(gs, selected);
            }
            Some(super::enums::SelectTargetKind::DrawAnyNumber) => {
                return self.handle_draw_any_number(gs, selected);
            }
            Some(super::enums::SelectTargetKind::Order) => {
                return self.handle_order_selection(gs, selected);
            }
            Some(super::enums::SelectTargetKind::SelfOrOpponent) => {
                let chosen = match selected {
                    "自分" => "self",
                    "相手" => "opponent",
                    _ => return Err("Invalid choice for SelfOrOpponent".to_string()),
                };
                log::debug!("[SELFOR] chosen={}", chosen);
                self.in_flight.spawn_context.target = Some(chosen.to_string());
                if let Some(ref current) = self.owner.executing {
                    log::debug!(
                        "[SELFOR] current.action={} steps={:?}",
                        current.action,
                        current.effect_steps.as_ref().map(|s| s.len())
                    );
                    if let Some(ref steps) = current.effect_steps {
                        if let Some(inner) = steps.first() {
                            let mut modified = inner.clone();
                            log::debug!(
                                "[SELFOR] inner.action={} steps_before={}",
                                modified.action,
                                modified.effect_steps.as_ref().map(|s| s.len()).unwrap_or(0)
                            );
                            Self::set_chosen_target(&mut modified, chosen);
                            log::debug!(
                                "[SELFOR] after_set: target={:?} has_la={} has_sa={}",
                                modified.target,
                                modified.compound_branch().and_then(|c| c.look_action.clone()).is_some(),
                                modified.compound_branch().and_then(|c| c.select_action.clone()).is_some()
                            );
                            self.awaiting.choice = None;
                            gs.ability_queue.set_pending_actions(vec![*modified]);
                            let res = self.resume_pending_actions(gs);
                            log::debug!(
                                "[SELFOR] after resume: pending={:?} res={:?}",
                                self.awaiting.choice.is_some(),
                                res
                            );
                            match res {
                                Ok(()) => return Ok(()),
                                Err(e) => {
                                    log::debug!("[SELFOR] inner effect failed: {}", e);
                                    self.clear_choice_state(gs);
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
                log::debug!("[SELFOR] no inner effect found");
                self.clear_choice_state(gs);
                return Ok(());
            }
            None => {}
        }

        self.clear_choice_state(gs);
        Ok(())
    }

    pub(in crate::ability::choice) fn handle_apply_replacement(
        &mut self,
        gs: &mut GameState,
        selected: &str,
    ) -> Result<(), String> {
        let Some((index, original)) = self.awaiting.replacement.take() else {
            self.clear_choice_state(gs);
            return Err("Pending replacement is missing".to_string());
        };
        self.clear_choice_state(gs);
        let accepted = selected == "0";
        let effects = gs
            .replacement_effects
            .get(index)
            .map(|replacement| replacement.replacement_effects.clone())
            .ok_or_else(|| "Pending replacement registration is missing".to_string())?;
        gs.mark_replacement_effect_applied(index);
        log::debug!(
            "[REPLACEMENT] registration={} accepted={} deferred_original={}",
            index,
            accepted,
            !accepted
        );
        if accepted {
            for effect in &effects {
                self.execute_effect(gs, effect)?;
            }
        }
        self.awaiting.replacement_original_suppressed = accepted;
        self.awaiting.resolving_replacement = true;
        let result = self.execute_effect(gs, &original);
        self.awaiting.resolving_replacement = false;
        result
    }

    pub(in crate::ability::choice) fn handle_draw_any_number(&mut self, gs: &mut GameState, selected: &str) -> Result<(), String> {
        let Ok(count) = selected.parse::<usize>() else {
            log::warn!("[DRAW_ANY] non-numeric count {:?}; rejecting selection", selected);
            return Err(format!("invalid selection: {:?}", selected));
        };
        if let Some(effect) = gs.entry_effect().cloned() {
            let source = effect.source_any().unwrap_or(Zone::Deck.to_str());
            let destination = effect
                .destination
                .map(|d| d.to_str())
                .unwrap_or(Zone::Hand.to_str());
            let ct_binding = effect.card_type_any();
            let card_type = ct_binding.map(|ct| ct.as_card_str());
            let card_db = gs.card_database.clone();
            let target = effect.target.as_deref().unwrap_or("self");
            let player = gs.resolve_target_player_mut(target);
            if count > 0 {
                crate::ability::effects::draw_cards_for_player(
                    player,
                    count.u8_count(),
                    source,
                    destination,
                    card_type,
                    false,
                    None,
                    &card_db,
                    None,
                )?;
            }
        }
        self.clear_choice_state(gs);
        Ok(())
    }

    pub(in crate::ability::choice) fn handle_order_selection(&mut self, gs: &mut GameState, selected: &str) -> Result<(), String> {
        let mut ordered = match self.in_flight.execution_context.clone() {
            ExecutionContext::LookAndSelect {
                step: LookAndSelectStep::Finalize { destination, .. },
            } if Zone::from_str(&destination) == Some(Zone::Deck) => Vec::new(),
            ExecutionContext::LookAndSelect {
                step: LookAndSelectStep::Order { ordered },
            } => ordered,
            _ => return Err("No pending deck order".to_string()),
        };
        let idx = selected
            .parse::<usize>()
            .ok()
            .filter(|&idx| idx < gs.looked_at_cards.len())
            .ok_or_else(|| format!("Invalid deck order selection: {}", selected))?;
        ordered.push(gs.looked_at_cards.remove(idx));
        log::debug!(
            "[ORDER_PICK] index={} ordered={:?} remaining={:?}",
            idx, ordered, gs.looked_at_cards
        );
        if gs.looked_at_cards.len() > 1 {
            let count = gs.looked_at_cards.len();
            self.clear_choice_meta(gs);
            self.capture_choice_owner();
            self.awaiting.choice = Some(Choice::SelectTarget {
                target: "order".to_string(),
                description: format!("Choose order for cards on deck ({} cards)", count),
                description_en: Some(format!("Choose order for cards on deck ({} cards)", count)),
                description_ja: Some(format!("山札のカード順を選択（{}枚）", count)),
                allow_skip: false,
                options: None,
            });
            self.in_flight.execution_context = ExecutionContext::LookAndSelect {
                step: LookAndSelectStep::Order { ordered },
            };
            return Ok(());
        }
        ordered.extend(gs.looked_at_cards.drain(..));
        let target = self
            .in_flight.spawn_context
            .target
            .clone()
            .or_else(|| {
                gs.entry_effect()
                    .and_then(|e| e.target.clone().map(|s| s.to_string()))
            })
            .unwrap_or_else(|| "self".to_string());
        let card_nos: Vec<String> = ordered
            .iter()
            .map(|&card_id| {
                gs.card_database
                    .get_card(card_id)
                    .map(|card| card.card_no.to_string())
                    .unwrap_or_default()
            })
            .collect();
        log::debug!("[ORDER_DONE] target={} cards={:?}", target, ordered);
        let player = gs.resolve_target_player_mut(&target);
        for &card_id in ordered.iter().rev() {
            player.main_deck.cards.insert(0, card_id);
        }
        log::debug!(
            "[ORDER_DONE] target={} cards={:?} card_nos={:?} deck_after={:?}",
            target,
            ordered,
            card_nos,
            player.main_deck.cards
        );
        self.selection.moved_cards.extend(ordered);
        self.in_flight.execution_context = ExecutionContext::None;
        self.clear_choice_state_and_resume(gs)
    }

    pub(in crate::ability::choice) fn handle_heart_color_selection(
        &mut self,
        gs: &mut GameState,
        selected: &str,
    ) -> Result<(), String> {
        const HEART_VALS: [&str; 7] = [
            "heart00", "heart01", "heart02", "heart03", "heart04", "heart05", "heart06",
        ];
        let Ok(idx) = selected.parse::<usize>() else {
            log::warn!("[HEART_COLOR_SEL] non-numeric index {:?}; rejecting selection", selected);
            return Err(format!("invalid selection: {:?}", selected));
        };
        if idx < HEART_VALS.len() {
            let color = HEART_VALS[idx];
            gs.prohibition_effects
                .push(format!("selected_heart_color:{}", color));
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.conditional_choice = Some(ConditionalChoice::Str(color.to_string()));
            }
        }
        self.clear_choice_state(gs);
        self.resume_pending_actions(gs)
    }

    /// The `choice_condition` step whose options the pending prompt belongs to.
    ///
    /// A cost that is ITSELF a choice puts its options on the root
    /// (`{"type": "choice_condition", "options": [...]}`), but a cost that is a
    /// `sequential_cost` with a choice among its steps nests them one level down
    /// — PL!-pb2-012-P＋ 南ことり's 起動 is
    /// 「このメンバーをウェイトにする：この能力を起動するための追加コストとして、手札を2枚
    /// 控え室に置くか、『Printemps』のメンバーを2人ウェイトにする」, whose cost is
    /// `[self-wait, choice(2 options)]`.
    ///
    /// Reading only the root resolved the player's selection against the WRONG step
    /// and paid the self-wait instead, which failed with "No matching members on
    /// stage to change state" — the choice looked live but paid nothing of itself.
    fn pending_choice_cost_options(
        gs: &GameState,
    ) -> Option<Vec<Box<crate::card::AbilityEffect>>> {
        let cost = gs.entry_cost()?;
        if cost.action == super::enums::ActionType::ChoiceCondition {
            return cost.compound_branch().and_then(|c| c.actions.clone());
        }
cost.compound_branch()
            .and_then(|c| c.actions.as_deref())
            .into_iter()
            .flatten()
            .find(|step| step.action == super::enums::ActionType::ChoiceCondition)
            .and_then(|step| step.compound_branch().and_then(|c| c.actions.as_deref()).map(|a| a.to_vec()))
    }

    pub(in crate::ability::choice) fn handle_choice_condition(
        &mut self,
        gs: &mut GameState,
        selected: &str,
    ) -> Result<(), String> {
        let Ok(idx) = selected.parse::<usize>() else {
            log::warn!("[CHOICE_COND] non-numeric index {:?}; rejecting selection", selected);
            return Err(format!("invalid selection: {:?}", selected));
        };
        if let Some(options) = Self::pending_choice_cost_options(gs) {
            if idx < options.len() {
                if crate::core::runtime_flags::logging_enabled() {
                    let label = options[idx]
                        .text
                        .split("}}")
                        .last()
                        .unwrap_or(&options[idx].text)
                        .trim()
                        .to_string();
                    let pp = gs.player_prefix();
                    let act_name = gs
                        .activating_card
                        .and_then(|id| gs.card_database.get_card(id))
                        .map(|c| c.name.to_string());
                    gs.log_entry(
                        format!(
                            "{pp} {}: [choice] {} ✓",
                            act_name.as_deref().unwrap_or(""),
                            label
                        ),
                        &pp,
                        gs.activating_card,
                        act_name,
                        "choice",
                    );
                }
                // Take the pending SelectTarget so we can detect if pay_cost
                // creates a new sub-choice (e.g., SelectCard for discard from hand).
                let old_choice = self.awaiting.choice.take();
                self.pay_cost(gs, &options[idx])?;
                if self.awaiting.choice.is_some() {
                    // pay_cost created a sub-choice — signal to preserve it
                    self.in_flight.sub_choice_created = true;
                } else {
                    // pay_cost resolved immediately — restore original choice
                    // so clear_choice_state can clean it up properly.
                    self.awaiting.choice = old_choice;
                }
            }
        }
        self.clear_choice_state(gs);
        Ok(())
    }

    pub(in crate::ability::choice) fn handle_heart_selection(
        &mut self,
        gs: &mut GameState,
        count: u8,
        colors: &[String],
    ) -> Result<(), String> {
        if let Some(chosen) = colors.first() {
            let color = crate::card::parse_heart_color(chosen);
            if let Some(card_id) = gs.activating_card {
                gs.set_heart_override(card_id, color, count.max(1), "live_end");
                gs.record_ability_application(
                    card_id,
                    format!("Treat hearts as {} ×{}", chosen, count.max(1)),
                    "heart_override",
                    card_id,
        Some(color.index().u8_count()),
        usize::from(count.max(1)).i16_count(),
                );
            }
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.conditional_choice = Some(ConditionalChoice::Str(chosen.clone()));
            }
        }
        self.awaiting.choice = None;
        self.finalize_choice(gs, &self.in_flight.execution_context.clone())
    }

}
