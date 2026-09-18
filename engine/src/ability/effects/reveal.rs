use super::super::enums::Zone;
use super::super::resolver::AbilityResolver;
use super::super::types::{Choice, ChoiceRoute, ExecutionContext};
use crate::ability_queue::ConditionalChoice;
use crate::card::AbilityEffect;
use crate::game_state::GameState;
#[cfg(feature = "no_std")]
use alloc::{string::{String, ToString}, vec::Vec};

impl AbilityResolver {
    pub(crate) fn execute_reveal_until_chosen_card(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        // Get the chosen card type from the effect or from the current ability queue entry
        let chosen_card_type = gs
            .ability_queue
            .current_entry()
            .and_then(|e| match &e.conditional_choice {
                Some(ConditionalChoice::Str(s)) => Some(s.clone()),
                _ => None,
            })
            .or_else(|| effect.card_type_any().map(|s| s.to_string()));

        if let Some(card_type) = chosen_card_type {
            // Use the existing reveal_until_target functionality
            self.current_effect = Some(effect.clone());
            self.execute_reveal_until_target(
                gs,
                effect.target_name(),
                Some(&card_type),
                None,
                None,
            )?;

            // After reveal, we need to move the chosen card to hand and others to discard
            // The looked_at_cards should contain: [chosen_card, other_revealed_cards...]
            if !gs.looked_at_cards.is_empty() {
                let chosen_card = gs.looked_at_cards[0];
                let other_cards = gs.looked_at_cards[1..].to_vec();

                // Move chosen card to hand
                let player = gs.resolve_target_player_mut(effect.target_name());
                player.hand.cards.push(chosen_card);

                // Move other cards to discard
                player.waitroom.cards.extend(other_cards);

                // Clear looked_at_cards
                gs.looked_at_cards.clear();
            }
        } else if let Some(ref or_types) = effect.or_card_types_any() {
            // No card type chosen yet — create the type choice prompt.
            let desc = format!("Choose: {}", or_types.join(", or "));
            self.pending_choice = Some(Choice::SelectTarget {
                target: "choice_string".to_string(),
                description: desc,
                description_en: Some(format!("Choose: {}", or_types.join(", or "))),
                description_ja: Some(format!("選択: {}", or_types.join(", or "))),
                allow_skip: false,
                options: Some(
                    or_types
                        .iter()
                        .map(|t| {
                            let label = match t.as_str() {
                                "live_card" => {
                                    if effect.cost_limit_any().is_some() {
                                        format!(
                                            "Live card (cost {}{})",
                                            effect
                                                .cost_limit_operator_any()
                                                .as_deref()
                                                .unwrap_or(">="),
                                            effect.cost_limit_any().unwrap()
                                        )
                                    } else {
                                        "Live card".to_string()
                                    }
                                }
                                "member_card" => {
                                    format!(
                                        "Member card (cost {} {})",
                                        effect.cost_limit_operator_any().as_deref().unwrap_or(">="),
                                        effect.cost_limit_any().unwrap_or(0)
                                    )
                                }
                                _ => t.clone(),
                            };
                            label
                        })
                        .collect(),
                ),
            });
            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            // Store the or_card_types so the choice handler can look them up
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.conditional_choice = Some(ConditionalChoice::Strings(or_types.to_vec()));
            }
        } else {
            // If no card type was chosen and no available types, just clear any looked_at_cards
            gs.looked_at_cards.clear();
        }

        Ok(())
    }

    pub(crate) fn execute_re_yell(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let lose_blade_hearts = effect.lose_blade_hearts_any().unwrap_or(false);
        let target = effect.target_name();
        log::debug!(target: "rabuka_engine::ability::effects::misc","re_yell: lose_blade_hearts={}", lose_blade_hearts);
        if lose_blade_hearts {
            // Collect card IDs to clear first (avoid borrow conflict with gs.mods)
            let cids: Vec<i16> = {
                let player = gs.resolve_target_player(target);
                player
                    .stage
                    .stage
                    .iter()
                    .copied()
                    .filter(|&id| id != -1)
                    .collect()
            };
            for cid in cids {
                gs.mods.clear_all_for_card(cid);
            }
        }
        // Clear the old yell cards so perform_yell's new cards replace them.
        // initial_yell_revealed_cards was already saved in the phase code before
        // auto abilities fired, so it still contains the full initial yell list.
        gs.clear_revealed_cards();
        gs.re_yell_occurred = true;
        gs.prohibition_effects.push("re_yell".to_string());
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: [[log_re_yell]]", pp, act_name));
    }

    pub(crate) fn execute_shuffle(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let target = effect.target_name();
        let source = effect.source_or(Zone::Deck.to_str());
        let player = gs.resolve_target_player_mut(target);
        match Zone::from_str(source) {
            Some(Zone::Deck) => {
                crate::rng::shuffle_slice(&mut player.main_deck.cards);
            }
            Some(Zone::EnergyDeck) => {
                crate::rng::shuffle_slice(&mut player.energy_deck.cards);
            }
            _ => {
                log::debug!(target: "rabuka_engine::ability::effects::misc","Unknown shuffle zone: {}", source);
            }
        }
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: [[log_shuffle]]", pp, act_name));
    }

    /// Perform N additional yells.
    /// A yell reveals cards from deck top until a live card is found.
    /// Perform an actual yell: draw total_blade cards from the player's deck
    /// and add them to revealed_cards. The yell count is the number of times
    /// to repeat this draw-and-reveal process (calculated from per_unit for
    /// MIRAI TICKET's "for every 5 cost, perform 1 additional yell").
    pub(crate) fn execute_perform_yell(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let count: u8 = if effect.per_unit_any().unwrap_or(false) {
            // per_unit with per_unit_source = "previous_moved_cards":
            // sum costs of cards moved by the preceding action,
            // divide by per_unit_count, cap at repeat_limit.
            let total_cost: u8 = self
                .moved_cards
                .iter()
                .filter_map(|&cid| gs.card_database.get_card(cid).and_then(|c| c.cost))
                .map(|v| v as u8)
                .sum();
            let divisor = effect.per_unit_count_any().unwrap_or(1) as u8;
            let mut c = total_cost / divisor;
            if let Some(cap) = effect.repeat_limit_any() {
                c = c.min(cap as u8);
            }
            c
        } else if let Some(ref dc) = effect.dynamic_count_any() {
            self.resolve_dynamic_count(gs, dc)
        } else {
            effect.count_or(1) as u8
        };
        let target = effect.target_name();
        let card_db = gs.card_database.clone();
        let bm = gs.mods.blade_modifiers.clone();
        let om = gs.mods.orientation_modifiers.clone();
        // set_blade_type recoloring for the performing player, identical to
        // the primary yell path in player_perform_live.
        let override_color = {
            let btm = &gs.mods.blade_type_modifiers;
            let performer = gs.resolve_target_player(target);
            (0..3)
                .filter_map(|i| {
                    let cid = performer.stage.stage[i];
                    if cid == -1 {
                        None
                    } else {
                        btm.get(&cid).copied().map(crate::turn::live::blade_color_to_heart)
                    }
                })
                .next()
        };
        let mut all_drawn: Vec<i16> = Vec::new();
        for _ in 0..count {
            let total_blade = {
                let player = gs.resolve_target_player_mut(target);
                let tb = player.stage.total_blades(&card_db, &bm, &om, false);
                let mut drawn: Vec<i16> = Vec::new();
                // Q104 / Rule 10.2.1: refresh from waitroom mid-draw
                for _ in 0..tb {
                    if player.main_deck.cards.is_empty() && !player.waitroom.cards.is_empty() {
                        player.refresh();
                    }
                    if let Some(cid) = player.main_deck.draw() {
                        drawn.push(cid);
                    }
                }
                drawn
            };
            let reyell_source = gs.current_ability_source_card_id();
            let reyell_owner = crate::ability::util::target_player_index(
                target,
                gs.ability_master_id().as_deref(),
            );
            for cid in &total_blade {
                gs.push_revealed_card(*cid, reyell_source, false, reyell_owner, "re_yell");
            }
            all_drawn.extend(total_blade);
        }
        gs.re_yell_revealed_cards = gs.revealed_cards.clone();

        // Rule 8.3.12 / 8.3.15.1.1: process the re-yelled cards' blade-heart
        // and special-heart icons through the SAME shared helper as the
        // primary yell, then execute their draws immediately ("エールをすべ
        // て行った後", rule 8.3.12.1). The tallies are stashed for the phase
        // code, which applies them before its success check — it may already
        // have computed its original yell data because this ability can pause
        // on the optional discard choice.
        {
            let mut yell_cards: Vec<crate::core::types::YellCardResult> =
                Vec::with_capacity(all_drawn.len());
            let mut total_hearts = [0u8; 8];
            let mut note_icons = 0u8;
            let mut draw_total = 0u8;
            let mut card_nos: Vec<&str> = Vec::with_capacity(all_drawn.len());
            let mut scratch =
                crate::card::BaseHeart { hearts: crate::card::HeartMap::new() };
            for cid in &all_drawn {
                if let Some(card) = gs.card_database.get_card(*cid) {
                    let outcome = crate::turn::live::process_yell_revealed_card_icons(
                        card,
                        override_color,
                        &mut scratch,
                        &mut total_hearts,
                        &mut note_icons,
                    );
                    draw_total += outcome.draw_icons;
                    card_nos.push(card.card_no.as_ref());
                    yell_cards.push(crate::core::types::YellCardResult {
                        card_id: *cid,
                        blade_hearts: outcome.blade_hearts,
                        note_icons: outcome.note_icons,
                        draw_icons: outcome.draw_icons,
                        card_no: card.card_no.to_string().into(),
                    });
                }
            }
            log::debug!(target: "rabuka_engine::ability::effects::misc",
                "[REYELL_REBUILD] n={} cards={:?} draws={} scores={}",
                all_drawn.len(),
                card_nos,
                draw_total,
                note_icons
            );
            // Rule 8.3.12.1: each re-yelled draw icon draws 1 card.
            {
                let player = gs.resolve_target_player_mut(target);
                // Q104 / Rule 10.2.1: refresh from waitroom when deck runs out.
                for _ in 0..draw_total {
                    if player.main_deck.cards.is_empty() && !player.waitroom.cards.is_empty() {
                        player.refresh();
                    }
                    if let Some(new_card) = player.main_deck.draw() {
                        player.hand.add_card(new_card);
                    }
                }
            }
            gs.pending_reyell_rebuild = Some(crate::types::PendingReyellRebuild {
                owner: gs.resolve_target_player(target).id.clone(),
                prev_note_icons: {
                    let player = gs.resolve_target_player(target);
                    if player.id == gs.player1.id {
                        gs.player1_cheer_blade_heart_count
                    } else {
                        gs.player2_cheer_blade_heart_count
                    }
                },
                yell_cards,
                total_hearts,
                note_icons,
            });
        }
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log(format!(
            "{} {}: [[log_yell_execute:n={}]]",
            pp, act_name, count
        ));
    }

    pub(crate) fn execute_reveal_effect(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        if effect.multiple_targets_any().unwrap_or(false)
            && Zone::from_str(effect.source_any().unwrap_or("")) == Some(Zone::DeckTop)
        {
            if effect.optional.unwrap_or(false) {
                self.emit_pay_skip_gate(
                    gs,
                    Some(ChoiceRoute::OptionalCost),
                    "Reveal cards from deck (optional cost)?".to_string(),
                    "山札からカードを公開（オプションコスト）？".to_string(),
                    true,
                    None,
                );
                return Ok(());
            }
            let chosen = gs
                .ability_queue
                .current_entry()
                .and_then(|e| match &e.conditional_choice {
                    Some(ConditionalChoice::Str(s)) => Some(s.clone()),
                    _ => None,
                })
                .or_else(|| effect.card_type_any().map(|s| s.to_string()));
            // cost_limit and operator come from the reveal effect itself if set,
            // or from the conditional_choice JSON if stored there by select.
            let cl = effect.cost_limit_any();
            let co_binding = effect.cost_limit_operator_any();
            let co = co_binding.as_deref();
            self.current_effect = Some(effect.clone());
            return self.execute_reveal_until_target(
                gs,
                effect.target_name(),
                chosen.as_deref(),
                cl,
                co,
            );
        }
        self.current_effect = Some(effect.clone());
        self.execute_reveal(
            gs,
            effect.source_or(Zone::Hand.to_str()),
            effect.count_or(1),
            effect.target_name(),
            effect.card_type_any().map(|ct| ct.as_card_str()),
            effect.heart_colors_any(),
            effect.blind_any().unwrap_or(false),
        )?;

        // NOTE: "これによりライブカードを公開した場合スコア+1" patterns are parsed
        // as conditional_on_result (result_condition on revealed_cards +
        // followup modify_score) and resolved by execute_conditional_on_result.
        // The old hardcoded reveal→score heuristic here was removed: it fired
        // even when the card text had no such follow-up.

        Ok(())
    }
}
