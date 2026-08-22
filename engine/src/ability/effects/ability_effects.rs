use super::super::resolver::AbilityResolver;
use super::super::util;
use crate::card::{Ability, AbilityEffect};
use crate::game_state::GameState;
#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

impl AbilityResolver {
    pub(crate) fn execute_gain_ability_effect(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let text_binding = effect.ability_gain_any();
        let text = text_binding
            .filter(|s| !s.is_empty())
            .or({
                if effect.text.is_empty() {
                    None
                } else {
                    Some(effect.text.as_ref())
                }
            })
            .unwrap_or("");

        // 「自分のステージにいる『Aqours』のメンバー2人までは…を得る」 — the
        // targets chosen STAGE members, not the activating card. Mirror the
        // change_state flow: prompt once (deferred via pending_actions), then
        // register the gained ability on each selection.
        let stage_member_targeting = effect.source
            == Some(crate::ability::enums::Zone::Stage)
            && effect.card_type_any() == Some(&crate::card::CardType::Member)
            && effect.group_names_any().is_some_and(|g| !g.is_empty());

        if stage_member_targeting && self.selected_cards.is_empty() {
            let card_db = gs.card_database.clone();
            let groups: Vec<String> = effect
                .group_names_any()
                .map(|g| g.to_vec())
                .unwrap_or_default();
            let candidates: Vec<usize> = gs
                .resolve_target_player(effect.target_name())
                .stage
                .stage
                .iter()
                .enumerate()
                .filter(|(_, &cid)| {
                    cid != -1
                        && groups.iter().any(|g| {
                            crate::ability::util::card_matches_group_str(&card_db, cid, Some(g))
                        })
                })
                .map(|(i, _)| i)
                .collect();
            if !candidates.is_empty() {
                let pick = (effect.count_or(1) as usize).min(candidates.len());
                self.pending_choice = Some(
                    crate::ability::types::Choice::select_cards(
                        crate::ability::enums::Zone::Stage.to_str(),
                        pick,
                        format!("Select up to {} member(s) to gain the ability", pick),
                        true,
                    )
                    .description_ja(Some(format!("能力を得るメンバーを{}体選択", pick)))
                    .card_type(effect.card_type_any().map(|ct| format!("{:?}", ct)))
                    .group(groups.first().cloned())
                    .filtered_indices(Some(candidates))
                    .target_player_id(Some(effect.target_name().to_string()))
                    .is_select_action(true)
                    .build(),
                );
                self.stage_select_intent =
                    Some(crate::ability::types::StageSelectIntent::CollectTargets);
                self.execution_context =
                    crate::ability::types::ExecutionContext::SingleEffect { effect_index: 0 };
                // Re-apply THIS effect after the choice so the non-empty
                // selected_cards branch below registers per member.
                gs.ability_queue.set_pending_actions(vec![effect.clone()]);
                return Ok(());
            }
        }

        let targets: Vec<i16> = if self.selected_cards.is_empty() {
            gs.activating_card.into_iter().collect()
        } else {
            core::mem::take(&mut self.selected_cards).to_vec()
        };

        for target_card in targets {
            let _ = self.execute_gain_ability(
                gs,
                text,
                effect.target_name(),
                effect.duration_any().as_deref(),
                effect.gained_effect_any().cloned(),
                effect.ability_gain_trigger_any().as_deref(),
                Some(target_card),
            );
        }
        Ok(())
    }

    pub(crate) fn execute_set_card_identity_effect(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        if effect.all_regions_any().unwrap_or(false) {
            self.execute_set_card_identity_all_regions(gs, effect);
        } else {
            self.execute_set_card_identity(
                gs,
                &effect.identities_any().cloned().unwrap_or_default(),
            );
        }
        Ok(())
    }

    // ===== LEAF EFFECTS (all data directly from AbilityEffect params) =====

    pub(crate) fn execute_activate_ability(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let ability_text_binding = effect.ability_text_any();
        let ability_text = ability_text_binding.as_deref().unwrap_or("");
        let target_trigger_binding = effect.target_trigger_any();
        let target_trigger = target_trigger_binding.as_deref();
        let source_card_binding = effect.source_card_any();
        let source_card = source_card_binding.as_deref();

        // Which cards' abilities are fired. "previous_selected" (emitted by the
        // parser for 「そのカーチEそれらが持つ…能力を発動させる、E fires EVERY selected
        // card  Ee.g. 渡辺曁Eab#2: select THIS + 1 other Aqours member, then fire
        // each of their 登場 abilities.
        let use_selected = source_card == Some("previous_selected");
        let card_ids: Vec<i16> = if use_selected {
            self.selected_cards.iter().copied().collect()
        } else {
            let single = source_card.and_then(|sc| match sc {
                "cost_card" => gs
                    .recently_moved_cards
                    .as_ref()
                    .and_then(|cards| cards.last().copied()),
                _ => None,
            });
            single.into_iter().collect()
        };

        // The ability trigger to fire. When the parser leaves target_trigger null
        // (only the human target text "…登場能力…" remains), infer 登場.
        let trigger = target_trigger.or_else(|| {
            let t = effect.target_name();
            if t.contains("登場") {
                Some("登場")
            } else {
                None
            }
        });

        let player_id = gs
            .ability_queue
            .current_entry()
            .map(|e| e.player_id.clone())
            .unwrap_or_default();

        if let Some(trig) = trigger {
            for cid in card_ids {
                // Clone the card data so the immutable borrow is dropped before
                // the mutable gs.trigger_auto_ability call below.
                let (card_no, name, abilities) = match gs.card_database.get_card(cid) {
                    Some(c) => (c.card_no.to_string(), c.name.to_string(), c.abilities.clone()),
                    None => continue,
                };
                let matching: Vec<_> = abilities
                    .iter()
                    .map(|a| a.resolve())
                    .filter(|a| {
                        a.triggers
                            .as_ref()
                            .and_then(|t| t.split('/').next())
                            .unwrap_or("")
                            == trig
                    })
                    .collect();
                if let Some(ability) = matching.first() {
                    // Q273: fire through the normal ability queue so the 登場
                    // ability's own cost is paid before its effect resolves.
                    let ability_id = format!("{}_{}", card_no, ability.full_text);
                    gs.trigger_auto_ability(
                        ability_id,
                        crate::game_state::AbilityTrigger::Debut,
                        player_id.clone(),
                        Some(card_no),
                        Some(cid),
                        None,
                        None,
                    );
                    let pp = self.player_prefix(gs);
                    gs.push_rule_log(format!(
                        "{} {}: [[log_activated_ability:trigger={}]]",
                        pp, trig, name
                    ));
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
        let db = &gs.card_database;
        let is_self = effect.is_self_target();

        if is_self {
            // Self-targeting: invalidate the activating card itself (works for any zone)
            if let Some(card_id) = gs.activating_card {
                let pp = self.player_prefix(gs);
                let cn = self.card_name(card_id);
                gs.rule_log
                    .push(format!("{} {}: [[log_negate_ability_self]]", pp, cn));
                gs.negated_abilities.push(card_id);
                return Ok(());
            }
            return Err("no activating card for self-targeted invalidation".to_string());
        }

        // Other-targeting: find valid target on stage
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

        if let Some(activating) = gs.activating_card {
            let pp = self.player_prefix(gs);
            let cn = self.card_name(activating);
            gs.rule_log
                .push(format!("{} {}: [[log_negate_ability]]", pp, cn));
            if let Some(&target_id) = valid.first() {
                gs.negated_abilities.push(target_id);
            }
        }
        Ok(())
    }

    pub(crate) fn execute_suppress_ability_trigger(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let trigger_binding = effect.suppressed_trigger_any();
        let trigger = trigger_binding.unwrap_or("unknown");
        if let Some(card_id) = gs.activating_card {
            let pp = self.player_prefix(gs);
            let cn = self.card_name(card_id);
            gs.push_rule_log(format!(
                "{} {}: [[log_suppress_ability:trigger={}]]",
                pp, cn, trigger
            ));
        }
        log::info!(
            "[SUPPRESS] suppressing trigger={} for card={:?}",
            trigger,
            gs.activating_card
        );
        Ok(())
    }

    pub(crate) fn execute_gain_ability(
        &mut self,
        gs: &mut GameState,
        ability_text: &str,
        target: &str,
        duration: Option<&str>,
        gained_effect: Option<Box<AbilityEffect>>,
        trigger: Option<&str>,
        target_card: Option<i16>,
    ) -> Result<(), String> {
        // Store the gained ability for tracking purposes
        if let Some(card_id) = gs.activating_card {
            gs.gained_abilities
                .entry(card_id)
                .or_default()
                .push(ability_text.to_string());
        }

        // Register the gained ability as a REAL synthetic ability so the
        // constant scanner (recalculate_constants) and trigger scans see it:
        //
        // - LIVE_SUCCESS gains 竊・live_success.png texticon + LiveSuccess
        //   re-trigger support (existing behavior).
        // - 蟶ｸ譎・gains 竊・scanned by collect_constant_ids_for and routed
        //   through the normal constant arms (e.g. ModifyScore target=
        //   "live_total" feeds p1/p2_constant_total_score_bonus).
        //
        // The old "+N digit parse" fallback only remains for legacy cards
        // whose gain_ability carries no structured gained_effect.
        match (gained_effect, target_card) {
            (Some(gained), Some(card_id)) => {
                let triggers = trigger.and_then(|t| {
                    if t.is_empty() {
                        None
                    } else {
                        Some(t.to_string())
                    }
                });
                let is_live_total = gained.target_any() == Some("live_total");
                let immediate_val = if gained.action == crate::ability::enums::ActionType::ModifyScore
                    && !is_live_total
                {
                    // Structured value first; fall back to the "+N"/"＋N" text
                    // the old hack parsed (many cards carry no structured value).
                    gained.value_any().or_else(|| {
                        ability_text
                            .split(['+', '\u{FF0B}'])
                            .nth(1)
                            .and_then(|s| {
                                s.chars()
                                    .take_while(|c| c.is_ascii_digit())
                                    .collect::<String>()
                                    .parse::<u8>()
                                    .ok()
                            })
                    })
                } else {
                    None
                }
                .unwrap_or(0);
                let gained_ability = Ability {
                    full_text: ability_text.to_string(),
                    triggerless_text: Some(ability_text.to_string()),
                    triggers: triggers.map(Into::into),
                    use_limit: None,
                    is_null: false,
                    cost: None,
                    effect: Some(gained),
                    keywords: None,
                };
                log::debug!(
                    "[GAINED_ABILITY] registered trigger={:?} on card {}",
                    trigger,
                    card_id
                );
                gs.gained_card_abilities
                    .entry(card_id)
                    .or_default()
                    .push(gained_ability);
                // Per-card score gains must ALSO apply immediately: many flows
                // and assertions read mods.score_modifiers right after
                // resolution, and live.rs computes live card scores from it.
                // (target="live_total" gains route through the constant scanner
                // into p*_constant_total_score_bonus instead.)
                if immediate_val != 0 {
                    gs.mods.add_score_modifier(card_id, immediate_val as i16);
                    log::debug!(
                        "[GAINED_ABILITY] immediate +{} score to card {}",
                        immediate_val,
                        card_id
                    );
                }
                // A gained 常時 changes the constant landscape — make sure the
                // next recalculation picks it up.
                self.last_gain_effect_data = Some(crate::core::types::EffectData::GainAbility {
                    card_id,
                    amount: immediate_val as i16,
                    is_live_total,
                });
            }
            _ => {}
        }

        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log(format!(
            "{} {}: [[log_gain_ability]]: {}",
            pp, act_name, ability_text
        ));

        util::push_temporary_effect(
            gs,
            &format!("gain_ability:{}", ability_text),
            duration,
            target,
            &format!("Gained ability: {}", ability_text),
            self.last_gain_effect_data.take(),
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

                if let Some(ct) = effect.card_type_any() {
                    let type_ok = match *ct {
                        crate::card::CardType::Member => {
                            c.card_type == crate::card::CardType::Member
                        }
                        crate::card::CardType::Live => c.card_type == crate::card::CardType::Live,
                        crate::card::CardType::Energy => {
                            c.card_type == crate::card::CardType::Energy
                        }
                    };
                    if !type_ok {
                        return false;
                    }
                }

                if let Some(cl) = effect.cost_limit_any() {
                    let card_cost = c.cost.unwrap_or(0);
                    let passes = match effect.cost_limit_operator_any().as_deref() {
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

                if let Some(ref groups) = effect.group_names_any() {
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
                for ar in &src_card.abilities {
                    let ability = ar.resolve();
                    let should_copy = match effect.trigger_filter_any().as_ref() {
                        Some(filters) => filters.iter().any(|f| {
                            ability
                                .triggers
                                .as_ref()
                                .is_some_and(|t| t.contains(f) || f.contains(&**t))
                        }),
                        None => true,
                    };
                    if should_copy {
                        gs.gained_abilities
                            .entry(activating_card)
                            .or_default()
                            .push(format!(
                                "ability_from_source:{}:{}",
                                src_id,
                                ability.triggerless_text()
                            ));
                        let gained = crate::card::Ability {
                            full_text: ability.full_text.clone(),
                            triggerless_text: ability.triggerless_text.clone(),
                            triggers: ability.triggers.clone(),
                            use_limit: ability.use_limit,
                            is_null: ability.is_null,
                            cost: ability.cost.clone(),
                            effect: ability.effect.clone(),
                            keywords: ability.keywords.clone(),
                        };
                        gs.gained_card_abilities
                            .entry(activating_card)
                            .or_default()
                            .push(gained);
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
        gs.push_rule_log(format!(
            "{} {}: [[log_gain_ability_from_source]]: {}",
            pp,
            act_name,
            source_names.join(", ")
        ));
        Ok(())
    }
}

