use super::super::resolver::AbilityResolver;
use super::super::util;
use crate::card::AbilityEffect;
use crate::game_state::GameState;

impl AbilityResolver {
    pub(crate) fn execute_gain_ability_effect(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let text = effect
            .ability_gain
            .as_deref()
            .filter(|s| !s.is_empty())
            .or({
                if effect.text.is_empty() {
                    None
                } else {
                    Some(effect.text.as_str())
                }
            })
            .unwrap_or("");
        self.execute_gain_ability(gs, text, effect.target_name(), effect.duration.as_deref())
    }

    pub(crate) fn execute_set_card_identity_effect(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        if effect.all_regions.unwrap_or(false) {
            self.execute_set_card_identity_all_regions(
                gs,
                effect.identities.as_ref(),
                effect.target_name(),
            );
        } else {
            self.execute_set_card_identity(gs, &effect.identities.clone().unwrap_or_default());
        }
        Ok(())
    }

    // ===== LEAF EFFECTS (all data directly from AbilityEffect params) =====

    pub(crate) fn execute_activate_ability(
        &mut self,
        gs: &mut GameState,
        ability_text: &str,
        target_trigger: Option<&str>,
        count: Option<u32>,
        source_card: Option<&str>,
    ) {
        let card_id = source_card.and_then(|sc| match sc {
            "cost_card" => gs
                .recently_moved_cards
                .as_ref()
                .and_then(|cards| cards.last().copied()),
            "previous_selected" => self.selected_cards.last().copied(),
            _ => None,
        });
        if let Some(cid) = card_id {
            let trigger = target_trigger.map(|t| t.to_string());
            let card_data = gs
                .card_database
                .get_card(cid)
                .map(|c| (c.abilities.clone(), c.name.clone()));
            if let Some((abilities, cn)) = card_data {
                if let Some(ref trig) = trigger {
                    let matching: Vec<&crate::card::Ability> = abilities
                        .iter()
                        .filter(|a| {
                            let at = a
                                .triggers
                                .as_ref()
                                .and_then(|t| t.split('/').next())
                                .unwrap_or("");
                            at == trig
                        })
                        .collect();
                    let selected = if matching.len() > 1 && count.unwrap_or(1) == 1 {
                        // Multiple abilities match the trigger; use the first one.
                        // (A full implementation would prompt the player, but for
                        // engine purposes, picking the first matching ability is
                        // sufficient since abilities of the same trigger on a card
                        // are typically ordered by priority.)
                        matching.first()
                    } else {
                        matching.first()
                    };
                    if let Some(ability) = selected {
                        if let Some(ref effect) = ability.effect {
                            let _ = self.execute_effect(gs, effect);
                        }
                        let pp = self.player_prefix(gs);
                        gs.rule_log.push(format!(
                            "{} {}: activated {} ability from {}",
                            pp,
                            gs.activating_card
                                .map(|c| self.card_name(c))
                                .unwrap_or_default(),
                            trig,
                            cn
                        ));
                        return;
                    }
                }
            }
            return;
        }

        // Fallback: store gained ability string
        if let Some(card_id) = gs.activating_card {
            let mut text = ability_text.to_string();
            if let Some(trigger) = target_trigger {
                text = format!("{}_trigger:{}", text, trigger);
            }
            gs.gained_abilities.entry(card_id).or_default().push(text);
        }
    }

    pub(crate) fn execute_invalidate_ability(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        // Check if there are valid targets on stage matching the effect's filters,
        // excluding the activating card itself (you can't invalidate your own abilities
        // as part of playing yourself).
        let db = &gs.card_database;
        let player = gs.resolve_target_player(effect.target.as_deref().unwrap_or("self"));
        let stage_ids: Vec<i16> = player
            .stage
            .stage
            .iter()
            .copied()
            .filter(|&id| id != -1)
            .collect();

        let mut filter = super::util::CardFilter::from_effect(effect);
        if let Some(activating) = gs.activating_card {
            filter.exclude_self = Some(activating);
        }
        let valid = super::util::matching_ids(&stage_ids, db, &filter, true);

        if valid.is_empty() {
            return Err("no valid targets for invalidation".to_string());
        }

        if let Some(card_id) = gs.activating_card {
            let pp = self.player_prefix(gs);
            let cn = self.card_name(card_id);
            gs.rule_log.push(format!("{} {}: 能力無効化", pp, cn));
            gs.negated_abilities.insert(card_id);
        }
        Ok(())
    }

    pub(crate) fn execute_gain_ability(
        &mut self,
        gs: &mut GameState,
        ability_text: &str,
        target: &str,
        duration: Option<&str>,
    ) -> Result<(), String> {
        // Store the gained ability for tracking purposes
        if let Some(card_id) = gs.activating_card {
            gs.gained_abilities
                .entry(card_id)
                .or_default()
                .push(ability_text.to_string());
        }

        // If the parser provided a pre-parsed gained_effect, execute it directly.
        // This handles the common "score +X" pattern generically (any value).
        if let Some(gained) = self
            .current_effect
            .as_ref()
            .and_then(|e| e.gained_effect.clone())
        {
            log::debug!(
                "[GAINED_ABILITY] Executing gained effect: {:?}",
                gained.action
            );
            self.execute_effect(gs, &gained)?;
        } else {
            // Fallback: parse value from the gained ability text
            if let Some(val) = ability_text.split('+').nth(1).and_then(|s| {
                s.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<i32>()
                    .ok()
            }) {
                if let Some(card_id) = gs.activating_card {
                    gs.mods.add_score_modifier(card_id, val);
                    gs.record_ability_application(
                        card_id,
                        ability_text.to_string(),
                        "score_bonus",
                        card_id,
                        None,
                        val,
                    );
                    log::debug!(
                        "[GAINED_ABILITY] Applied +{} score modifier to card {}",
                        val,
                        card_id
                    );
                }
            }
        }

        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log.push(format!(
            "{} {}: 能力獲得 「{}」",
            pp, act_name, ability_text
        ));

        util::push_temporary_effect(
            gs,
            &format!("gain_ability:{}", ability_text),
            duration,
            target,
            &format!("Gained ability: {}", ability_text),
            None,
        );
        Ok(())
    }

    pub(crate) fn execute_gain_ability_from_source(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let activating_card = match gs.activating_card {
            Some(c) => c,
            None => return Ok(()),
        };

        gs.gained_abilities.remove(&activating_card);

        let player = if gs.player1.stage.stage.contains(&activating_card) {
            &gs.player1
        } else if gs.player2.stage.stage.contains(&activating_card) {
            &gs.player2
        } else {
            return Ok(());
        };

        let pos = player
            .stage
            .stage
            .iter()
            .position(|&id| id == activating_card);
        let area = match pos {
            Some(0) => crate::zones::MemberArea::LeftSide,
            Some(1) => crate::zones::MemberArea::Center,
            Some(2) => crate::zones::MemberArea::RightSide,
            _ => return Ok(()),
        };

        let under_cards: Vec<i16> = player.stage.get_under_cards(area).to_vec();
        let card_db = self.card_db();

        let source_cards: Vec<i16> = under_cards
            .iter()
            .filter(|&&id| id != -1)
            .filter(|&&id| {
                let c = match card_db.get_card(id) {
                    Some(c) => c,
                    None => return false,
                };

                if let Some(ref ct) = effect.card_type {
                    let type_ok = match ct.as_str() {
                        "member_card" => c.card_type == crate::card::CardType::Member,
                        "live_card" => c.card_type == crate::card::CardType::Live,
                        "energy_card" => c.card_type == crate::card::CardType::Energy,
                        _ => true,
                    };
                    if !type_ok {
                        return false;
                    }
                }

                if let Some(cl) = effect.cost_limit {
                    let card_cost = c.cost.unwrap_or(0);
                    let passes = match effect.cost_limit_operator.as_deref() {
                        Some("<=") | None => card_cost <= cl,
                        Some(">=") => card_cost >= cl,
                        Some("<") => card_cost < cl,
                        Some(">") => card_cost > cl,
                        Some("==") => card_cost == cl,
                        _ => card_cost <= cl,
                    };
                    if !passes {
                        return false;
                    }
                }

                if let Some(ref groups) = effect.group_names {
                    if !groups.iter().any(|g| {
                        crate::ability::util::card_matches_group_str(&card_db, id, Some(g))
                    }) {
                        return false;
                    }
                }

                true
            })
            .copied()
            .collect();

        for &src_id in &source_cards {
            if let Some(src_card) = card_db.get_card(src_id) {
                for ability in &src_card.abilities {
                    let should_copy = match effect.trigger_filter.as_ref() {
                        Some(filters) => filters.iter().any(|f| {
                            ability
                                .triggers
                                .as_ref()
                                .is_some_and(|t| t.contains(f) || f.contains(t.as_str()))
                        }),
                        None => true,
                    };
                    if should_copy {
                        gs.gained_abilities
                            .entry(activating_card)
                            .or_default()
                            .push(format!(
                                "ability_from_source:{}:{}",
                                src_id, ability.triggerless_text
                            ));
                    }
                }
            }
        }

        let pp = self.player_prefix(gs);
        let act_name = self.card_name(activating_card);
        let source_names: Vec<String> = source_cards
            .iter()
            .map(|&cid| self.card_name(cid))
            .collect();
        gs.rule_log.push(format!(
            "{} {}: {}から能力獲得",
            pp,
            act_name,
            source_names.join(",")
        ));
        Ok(())
    }
}
