use super::super::enums::Zone;
use super::super::resolver::AbilityResolver;
use super::super::types::{Choice, ChoiceRoute, ExecutionContext};
use super::super::util;
use crate::ability_queue::ConditionalChoice;
use crate::card::{AbilityEffect, PlacementOrder, PositionInfo};
use crate::core::types::ArcStr;
use crate::game_state::GameState;
use crate::{HashMap, HashSet};
#[cfg(feature = "no_std")]
use alloc::{
    borrow::Cow,
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use smallvec::SmallVec;
#[cfg(not(feature = "no_std"))]
use std::borrow::Cow;

impl AbilityResolver {
    pub(crate) fn execute_reveal_effect(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        if effect.multiple_targets_any().unwrap_or(false)
            && Zone::from_str(effect.source_any().unwrap_or("")) == Some(Zone::DeckTop)
        {
            if effect.optional.unwrap_or(false) {
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "pay_optional_cost:skip_optional_cost".to_string(),
                    description: "Reveal cards from deck (optional cost)?".to_string(),
                    description_en: Some("Reveal cards from deck (optional cost)?".to_string()),
                    description_ja: Some("山札からカードを公開（オプションコスト）？".to_string()),
                    allow_skip: true,
                    options: None,
                });
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.choice_card_no = Some(ChoiceRoute::OptionalCost);
                }
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
            return self.execute_reveal_until_target(
                gs,
                effect.target_name(),
                chosen.as_deref(),
                cl,
                co,
            );
        }
        self.execute_reveal(
            gs,
            effect.source_or(Zone::Hand.to_str()),
            effect.count_or(1),
            effect.target_name(),
            effect.card_type_any().map(|ct| ct.as_card_str()),
            effect.heart_colors_any(),
            effect.blind_any().unwrap_or(false),
        )?;

        if effect.self_target_any().unwrap_or(false) {
            if let Some(ct) = effect.card_type_any() {
                let card_db = &gs.card_database;
                let has_matching = gs.revealed_cards.iter().any(|&cid| {
                    crate::ability::util::card_matches_type(card_db, cid, Some(ct.as_card_str()))
                });
                if has_matching {
                    if let Some(cid) = gs.activating_card {
                        gs.mods.add_score_modifier(cid, 1);
                    }
                }
            }
        }

        Ok(())
    }

    pub(crate) fn execute_custom(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        action_str: &str,
    ) -> Result<(), String> {
        // Handle "custom" actions that could not be parsed into a standard action type.
        // Some custom actions have enough info to re-route to a known handler.

        // 1) Deck reordering: placement_order=any_order → route as move_cards looked_at→deck_top
        if effect.placement_order_any() == Some(PlacementOrder::AnyOrder) {
            let mut routed = effect.clone();
            routed.action = crate::ability::enums::ActionType::MoveCards;
            if routed.source.is_none() {
                routed.source = Some(Zone::LookedAt.to_str().into());
            }
            if routed.destination.is_none() {
                routed.destination = Some(Zone::DeckTop.to_str().into());
            }
            return self.execute_move_cards(gs, &routed);
        }

        // 2) Complex conditional scoring / gain_ability: has duration
        if effect.duration_any().is_some() {
            let text = if effect.text.is_empty() {
                action_str
            } else {
                &effect.text
            };
            return self.execute_gain_ability(
                gs,
                text,
                effect.target_any().unwrap_or("self"),
                effect.duration_any().as_deref(),
                effect.gained_effect_any().cloned(),
                effect.ability_gain_trigger_any().as_deref(),
            );
        }

        log::debug!("Unhandled custom action: {}", action_str);
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: [[log_custom_effect]]", pp, act_name));
        Ok(())
    }

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

    /// Handles target="both" by executing the effect for self, then opponent.
    /// Returns true if the effect was fully handled (has "both" target), false otherwise.
    pub(crate) fn handle_both_targets(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<bool, String> {
        // Skip if not "both" or if this is position_change (handles "both" internally)
        if effect.target.as_deref() != Some("both")
            || effect.action == crate::ability::enums::ActionType::PositionChange
        {
            return Ok(false);
        }

        // Execute for self first
        let mut for_self = effect.clone();
        for_self.target = Some("self".into());
        self.spawn_context.target = Some("self".to_string());

        let had_choice_before = self.pending_choice.is_some();
        let _ = self.execute_effect(gs, &for_self);

        // If self created a NEW pending choice, save opponent for later
        if self.pending_choice.is_some() && !had_choice_before {
            let mut for_opponent = effect.clone();
            for_opponent.target = Some("opponent".into());
            // Preserve any existing pending commands (e.g. remaining sequential actions)
            let mut existing = gs.ability_queue.take_pending_actions();
            existing.push(for_opponent);
            gs.ability_queue.set_pending_actions(existing);
            return Ok(true);
        }

        // Execute for opponent
        let mut for_opponent = effect.clone();
        for_opponent.target = Some("opponent".into());
        self.spawn_context.target = Some("opponent".to_string());
        self.execute_effect(gs, &for_opponent)?;

        Ok(true)
    }

    fn apply_heart_to_card(
        &mut self,
        gs: &mut GameState,
        card_id: i16,
        heart_distribution: &[(crate::card::HeartColor, u8)],
        is_negative: bool,
        is_temporary: bool,
        effect_data: &mut Option<crate::core::types::EffectData>,
        heart_color_str: &Option<String>,
        heart_to_add: i16,
        effect_text: &str,
    ) {
        for &(color, dist_count) in heart_distribution {
            let dist_amount = if is_negative {
                -(dist_count as i16)
            } else {
                dist_count as i16
            };
            gs.mods.add_heart_modifier_with_trace(
                card_id,
                color,
                dist_amount,
                &mut gs.ability_applications,
                gs.activating_card.unwrap_or(-1),
                effect_text,
            );
        }
        if is_temporary && effect_data.is_none() {
            if heart_distribution.len() > 1 {
                let items: Vec<crate::core::types::CardEffectItem> = heart_distribution
                    .iter()
                    .map(|&(c, dc)| {
                        let amount = if is_negative { -(dc as i16) } else { dc as i16 };
                        crate::core::types::CardEffectItem {
                            card_id,
                            amount,
                            color: Some(format!("{:?}", c)),
                        }
                    })
                    .collect();
                *effect_data = Some(crate::core::types::EffectData::MultiCard { items });
            } else {
                let color_name = heart_color_str.as_deref().unwrap_or("heart01");
                *effect_data = Some(Self::make_card_effect_data(
                    card_id,
                    heart_to_add,
                    Some(color_name),
                ));
            }
        }
    }

    pub(crate) fn handle_bp6_pattern(
        &self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<bool, String> {
        // bp6 pattern: "gain 1 heart per distinct color among discarded cards"
        // Detected by: resource=heart, per_unit=true, per_unit_type="discard", multiple_targets=true
        // For each distinct heart color present among recently_moved_cards, grant 1 heart of that color.
        if effect.resource_any().as_deref() == Some("heart")
            && effect.per_unit_any().unwrap_or(false)
            && Zone::from_str(effect.per_unit_type_any().as_deref().unwrap_or(""))
                == Some(Zone::Discard)
            && effect.multiple_targets_any().unwrap_or(false)
        {
            let card_db = self.card_db();
            let duration = effect.duration_any().clone();
            let is_temporary = duration.is_some() && duration.as_deref() != Some("permanent");
            let target = effect.target_name().to_string();
            let activating_card_id = gs.activating_card;

            // Collect distinct heart colors from all recently discarded cards.
            // Member cards carry their heart colors in base_heart, not need_heart.
            let recently_moved = gs.recently_moved_cards.clone();
            let mut distinct_colors: SmallVec<[crate::card::HeartColor; 8]> = SmallVec::new();
            if let Some(ref moved) = recently_moved {
                for &cid in moved {
                    if let Some(card) = card_db.get_card(cid) {
                        // Use base_heart for member cards (need_heart is only on live cards)
                        if let Some(ref bh) = card.base_heart {
                            for &(color, amt) in &bh.hearts {
                                if amt > 0 && !distinct_colors.contains(&color) {
                                    distinct_colors.push(color);
                                }
                            }
                        }
                    }
                }
            }

            log::debug!(
                "[BP6_HEART] distinct colors from {} discarded cards: {:?}",
                recently_moved.as_ref().map(|v| v.len()).unwrap_or(0),
                distinct_colors
            );

            if let Some(card_id) = activating_card_id {
                for color in &distinct_colors {
                    gs.mods.add_heart_modifier_with_trace(
                        card_id,
                        *color,
                        1,
                        &mut gs.ability_applications,
                        gs.activating_card.unwrap_or(-1),
                        &effect.text,
                    );
                    gs.record_ability_application(
                        card_id,
                        effect.text.to_string(),
                        "heart_bonus",
                        card_id,
                        Some(color.index() as u8),
                        1,
                    );
                }
                if is_temporary && !distinct_colors.is_empty() {
                    let color_names: Vec<String> = distinct_colors
                        .iter()
                        .map(|c| format!("{:?}", c).to_lowercase())
                        .collect();
                    let items: Vec<crate::core::types::CardEffectItem> = distinct_colors
                        .iter()
                        .map(|color| crate::core::types::CardEffectItem {
                            card_id,
                            amount: 1,
                            color: Some(color.to_string()),
                        })
                        .collect();
                    let effect_data = Some(crate::core::types::EffectData::MultiCard { items });
                    util::push_temporary_effect(
                        gs,
                        "gain_heart",
                        duration.as_deref(),
                        &target,
                        &format!("Gain 1 heart of each color: {}", color_names.join(", ")),
                        effect_data,
                    );
                }
            }
            return Ok(true);
        }

        Ok(false)
    }

    pub(crate) fn execute_gain_surplus_heart(
        &self,
        gs: &mut GameState,
        _effect: &AbilityEffect,
        target: &str,
        is_temporary: bool,
        duration: Option<&str>,
        sign: Option<&str>,
        is_all: bool,
    ) -> Result<(), String> {
        let player = gs.resolve_target_player(target);
        let is_p1 = player.id == gs.player1.id;
        let old = if sign == Some("negative") && is_all {
            // Compute surplus from snapshot total_hearts minus requirements,
            // so test modifications to the snapshot (e.g. total_hearts)
            // are reflected in last_surplus_loss_count.
            let pid = if is_p1 {
                &gs.player1.id
            } else {
                &gs.player2.id
            };
            let v = gs
                .performance_snapshots
                .iter()
                .find(|s| &s.player_id == pid)
                .map(|s| {
                    s.total_hearts.iter().sum::<u8>()
                        - s.lives.iter().flat_map(|l| l.required.iter()).sum::<u8>()
                })
                .unwrap_or(if is_p1 {
                    gs.self_live_surplus_count
                } else {
                    gs.opponent_live_surplus_count
                });
            gs.mods.last_surplus_loss_count = v;
            if is_p1 {
                gs.self_live_surplus_count = 0;
            } else {
                gs.opponent_live_surplus_count = 0;
            }
            Some(v)
        } else {
            None
        };
        if is_temporary {
            let desc = format!(
                "{} all surplus hearts",
                if sign == Some("negative") {
                    "Lose"
                } else {
                    "Gain"
                }
            );
            let effect_data = old.map(|v| crate::core::types::EffectData::SurplusHeart {
                is_p1,
                old_value: v,
            });
            util::push_temporary_effect(
                gs,
                "gain_surplus_heart",
                duration.as_deref(),
                &target,
                &desc,
                effect_data,
            );
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn calculate_gain_multiplier(
        &self,
        gs: &GameState,
        effect: &AbilityEffect,
        per_unit: bool,
        base_count: u8,
        per_unit_type_str: Option<&str>,
        target: &str,
        recently_moved: &Option<SmallVec<[i16; 4]>>,
        entry_snapshot: &Option<SmallVec<[i16; 4]>>,
        last_energy: u8,
        last_discard_count: u8,
        orientation_modifiers: &HashMap<i16, crate::core::game_modifiers::CardOrientation>,
        filter: &crate::ability::util::CardFilter,
    ) -> u8 {
        if !per_unit {
            return base_count;
        }
        let card_db = self.card_db();
        let player = gs.resolve_target_player(target);

        // If the effect has an explicit location, use that as the count zone
        // instead of the generic per_unit_type → zone mapping.
        let loc_binding = effect.location_any();
        let effective_per_unit_type = loc_binding.as_deref().or(per_unit_type_str);
        let mut matching_count = if effective_per_unit_type == Some("つ") {
            last_energy
        } else {
            util::resolve_per_unit_count(
                true,
                effective_per_unit_type,
                player,
                &card_db,
                filter,
                &[],
                effect.state_any().as_deref(),
                orientation_modifiers,
            )
        };
        // For per_unit_type="discard": always use tracked move/cost counts,
        // never the full waitroom. Checks recently_moved first, then the
        // enqueue-time snapshot (trigger_moved_cards) as fallback.
        let tracked_moved = recently_moved.as_ref().or(entry_snapshot.as_ref());
        if Zone::from_str(per_unit_type_str.unwrap_or("")) == Some(Zone::Discard) {
            let tm_len = tracked_moved.map(|v| v.len()).unwrap_or(0);
            log::debug!(
                "[DBG_GR] tracked_moved.len={} last_discard_count={}",
                tm_len,
                last_discard_count,
            );
            matching_count = util::resolve_discard_per_unit_count(
                tracked_moved,
                last_discard_count,
                &card_db,
                filter,
            );
            log::debug!("[DBG_GR] resolve result: matching_count={}", matching_count);
        } else if (Zone::from_str(per_unit_type_str.unwrap_or("")) == Some(Zone::Waitroom)
            || per_unit_type_str == Some("waitroom_card"))
            && (last_discard_count > 0 || recently_moved.is_some())
        {
            matching_count = util::resolve_discard_per_unit_count(
                recently_moved.as_ref(),
                last_discard_count,
                &card_db,
                filter,
            );
        } else if per_unit_type_str == Some("energy_deck") {
            // Cards placed in energy deck are always energy cards,
            // so don't filter by card_type (which may differ from
            // the gain_resource's target card_type for member targeting).
            // Only count recently_moved (from the current effect),
            // NOT entry_snapshot (which may contain trigger-setup cards).
            if let Some(moved) = recently_moved.as_ref() {
                matching_count = moved.len() as u8;
            } else {
                matching_count = 0;
            }
        }
        let per_unit_count_val = effect.per_unit_count_any().unwrap_or(1);
        let mut units = matching_count / per_unit_count_val;
        if effect.max.unwrap_or(false) {
            if let Some(cap) = effect.count_any() {
                units = units.min(cap);
            }
        }
        // Also cap by max_repeats (aliased as repeat_limit), used
        // when the parser sets it as the sole cap on per_unit effects
        // (e.g. "N枚までしか数えない" text constraints).
        if let Some(cap) = effect.repeat_limit_any() {
            units = units.min(cap);
        }
        let per_unit_base = if effect.max.unwrap_or(false) {
            1
        } else {
            effect
                .resource_icon_count_any()
                .unwrap_or(effect.count_or(1))
        };
        units * per_unit_base
    }
    pub fn execute_gain_resource(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            log::debug!("[GR_ENTER] resource={:?} count={:?} target_count={:?} source={:?} card_type={:?} target={:?} exclude_self={:?} target_from_sel={:?}",
                effect.resource_any(), effect.count_any(), effect.target_count_any(), effect.source_any(), effect.card_type_any(), effect.target_any(), effect.exclude_self_any(), effect.target_from_selection_any());
        }
        // heart_colors_from_selected_card: gain 1 heart of each color
        // that the previously-selected card in the sequential has (base_heart).
        if effect.resource_any().as_deref() == Some("heart")
            && effect
                .heart_colors_from_selected_card_any()
                .unwrap_or(false)
        {
            let target_str = effect.target_name().to_string();
            let player = gs.resolve_target_player_mut(&target_str);
            let card_db = self.card_db();
            let target_ids: Vec<i16> = player
                .stage
                .stage
                .iter()
                .cloned()
                .filter(|&tid| {
                    tid != -1
                        && crate::ability::util::card_matches_characters(
                            &card_db,
                            tid,
                            effect.characters_any().map(|v| &**v),
                        )
                })
                .collect();
            let _ = player;
            if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
                eprintln!("[GR_SELECTED_CARD] entering branch. selected_cards={:?} target_ids={:?} gs.activating={:?}",
                    self.selected_cards, target_ids, gs.activating_card);
            }
            if let Some(&selected_id) = self.selected_cards.first() {
                if let Some(selected_card) = card_db.get_card(selected_id) {
                    if crate::ability::debug::ABILITY_DEBUG
                        .load(core::sync::atomic::Ordering::Relaxed)
                    {
                        eprintln!(
                            "[GR_SELECTED_BH] selected_id={} has_base_heart={} hearts_count={}",
                            selected_id,
                            selected_card.base_heart.is_some(),
                            selected_card
                                .base_heart
                                .as_ref()
                                .map(|bh| bh.hearts.len())
                                .unwrap_or(0)
                        );
                    }
                    if let Some(ref base_heart) = selected_card.base_heart {
                        for &(color, _) in &base_heart.hearts {
                            for &target_id in &target_ids {
                                if crate::ability::debug::ABILITY_DEBUG
                                    .load(core::sync::atomic::Ordering::Relaxed)
                                {
                                    eprintln!(
                                        "[GR_APPLY] target={} color={:?} before={}",
                                        target_id,
                                        color,
                                        gs.mods.get_heart_modifier(target_id, color)
                                    );
                                }
                                gs.mods.add_heart_modifier_with_trace(
                                    target_id,
                                    color,
                                    1,
                                    &mut gs.ability_applications,
                                    target_id,
                                    &effect.text,
                                );
                                if crate::ability::debug::ABILITY_DEBUG
                                    .load(core::sync::atomic::Ordering::Relaxed)
                                {
                                    eprintln!(
                                        "[GR_APPLY] target={} color={:?} after={}",
                                        target_id,
                                        color,
                                        gs.mods.get_heart_modifier(target_id, color)
                                    );
                                }
                                if effect.duration_any().as_deref() == Some("live_end") {
                                    let effect_data = crate::core::types::EffectData::SingleCard {
                                        card_id: target_id,
                                        amount: 1,
                                        color: Some(format!("{:?}", color)),
                                    };
                                    crate::ability::util::push_temporary_effect(
                                        gs,
                                        "gain_heart",
                                        Some("live_end"),
                                        "self",
                                        &format!("Gain +1 {:?} from selected card", color),
                                        Some(effect_data),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            return Ok(());
        }

        if effect.resource_any().as_deref() == Some("heart")
            && effect.heart_type_any().as_deref() == Some("all")
        {
            let target_str = effect.target_name().to_string();
            // Capture values before mutable borrow of gs
            let triggering_member = gs
                .ability_queue
                .current_entry()
                .and_then(|e| e.triggering_member_id);
            let activating = gs.activating_card;
            let has_explicit_target = effect.target_any().is_some();
            let player = gs.resolve_target_player_mut(&target_str);
            let card_id = triggering_member.or_else(|| {
                if let Some(ref pos) = effect.position_any() {
                    let pos_str = pos.get_position()?;
                    let idx = crate::ability::util::stage_position_index(pos_str)?;
                    if idx < player.stage.stage.len() && player.stage.stage[idx] != -1 {
                        Some(player.stage.stage[idx])
                    } else {
                        None
                    }
                } else if has_explicit_target {
                    player.stage.stage.iter().find(|&&id| id != -1).copied()
                } else {
                    activating
                }
            });
            if let Some(card_id) = card_id {
                let amount = effect.count_or(1) as i16;
                gs.mods.add_heart_modifier_with_trace(
                    card_id,
                    crate::card::HeartColor::All,
                    amount,
                    &mut gs.ability_applications,
                    card_id,
                    &effect.text,
                );
                if effect.duration_any().as_deref() == Some("live_end") {
                    let effect_data = crate::core::types::EffectData::SingleCard {
                        card_id,
                        amount,
                        color: Some("all".to_string()),
                    };
                    crate::ability::util::push_temporary_effect(
                        gs,
                        "gain_heart",
                        Some("live_end"),
                        "self",
                        &format!("Gain {} all-heart", amount),
                        Some(effect_data),
                    );
                }
            }
            return Ok(());
        }

        if self.handle_bp6_pattern(gs, effect)? {
            return Ok(());
        }
        let resource = effect.resource_any().as_deref().unwrap_or("").to_string();
        let count = effect
            .resource_icon_count_any()
            .unwrap_or(effect.count_or(1));
        let target = effect.target_name().to_string();
        let duration = effect.duration_any().clone();
        let is_temporary = duration.is_some() && duration.as_deref() != Some("permanent");
        let card_type_filter = effect.card_type_any().map(|s| s.to_string());
        let group_filter = effect.group_name().map(|s| s.to_string());
        let per_unit_type_str = effect.per_unit_type_any().clone();
        let heart_selection = effect.heart_selection_any().unwrap_or(false);
        let per_unit = effect.per_unit_any().unwrap_or(false);
        let sign_binding = effect.sign_any();
        let sign = sign_binding.as_deref();
        let activating_card_id = gs.activating_card;
        let card_db = self.card_db();
        let is_self_target = effect.self_target_any().unwrap_or(false);
        let last_discard_count = gs.mods.last_cost_discard_count;
        let is_all = effect.all_any().unwrap_or(false)
            || (effect.source_any().is_none()
                && effect.card_type_any() == Some(&crate::card::CardType::Member)
                && (target == "self" || target == "opponent")
                && !is_self_target
                && effect.exclude_self_any().is_none()
                && effect.target_count_any().is_none())
            // Also detect "all members" when the effect has no target_count limit
            // and targets "self"/"opponent" members (e.g. "自分のステージにいるメンバーは")
            || (effect.card_type_any() == Some(&crate::card::CardType::Member)
                && (target == "self" || target == "opponent")
                && effect.target_count_any().is_none()
                && effect.distinct_any().is_none());

        if resource == "surplus_heart" {
            return self.execute_gain_surplus_heart(
                gs,
                effect,
                &target,
                is_temporary,
                duration.as_deref(),
                sign,
                is_all,
            );
        }

        // If a preceding select action already set the heart color in
        // conditional_choice, use it directly — skip any heart_colors prompt.
        // This handles both per_unit and non-per_unit gain_resource that
        // follow a select in a sequential effect.
        let existing_choice =
            gs.ability_queue
                .current_entry()
                .and_then(|e| match &e.conditional_choice {
                    Some(ConditionalChoice::Str(s)) => Some(s.clone()),
                    _ => None,
                });
        let single_fixed_heart = if let Some(chosen) = existing_choice {
            Some(chosen)
        } else if per_unit && resource == "heart" {
            gs.ability_queue
                .current_entry()
                .and_then(|e| match &e.conditional_choice {
                    Some(ConditionalChoice::Str(s)) => Some(s.clone()),
                    _ => None,
                })
        } else {
            let effective_heart_colors: Vec<String> = effect
                .heart_color_any()
                .map(|hc| vec![hc.to_string()])
                .unwrap_or_else(|| effect.heart_colors_any().to_vec());
            let result = self.resolve_gain_heart_color(
                gs,
                effect,
                resource.as_str(),
                count,
                &effective_heart_colors,
                heart_selection,
            )?;
            if self.pending_choice.is_some() {
                return Ok(());
            }
            result.or_else(|| {
                if resource == "heart" || resource == "ハート" {
                    gs.ability_queue
                        .current_entry()
                        .and_then(|e| match &e.conditional_choice {
                            Some(ConditionalChoice::Str(s)) => Some(s.clone()),
                            _ => None,
                        })
                        .or_else(|| {
                            // Fallback: handle_heart_color_selection stores the color in
                            // prohibition_effects as "selected_heart_color:{color}" before
                            // clear_choice_meta resets conditional_choice.
                            gs.prohibition_effects
                                .iter()
                                .find_map(|e| e.strip_prefix("selected_heart_color:"))
                                .map(|s| s.to_string())
                        })
                } else {
                    None
                }
            })
        };

        // Extract accumulated selected card IDs from resolver
        let all_selected: SmallVec<[i16; 8]> = self.selected_cards.iter().copied().collect();

        // Pre-filter selected_cards by current character/card_type to prevent
        // cross-character leakage in sequential (e.g. blade for char A leaks
        // into blade for char B).
        let selected_for_current: Vec<i16> = if !all_selected.is_empty() {
            if let Some(ref chars) = effect.characters_any() {
                all_selected
                    .iter()
                    .filter(|&&cid| {
                        crate::ability::util::card_matches_characters(&card_db, cid, Some(chars))
                    })
                    .copied()
                    .collect()
            } else {
                all_selected.iter().copied().collect()
            }
        } else {
            Vec::new()
        };

        let recently_moved = gs.recently_moved_cards.clone();
        let entry_snapshot = gs.entry_trigger_moved_cards();
        let last_cost_moved = gs.mods.last_cost_moved_card_ids.clone();
        let preceding_moved: Option<SmallVec<[i16; 4]>> = entry_snapshot
            .clone()
            .or_else(|| {
                let rm = recently_moved.clone()?;
                if rm.is_empty() {
                    None
                } else {
                    Some(rm)
                }
            })
            .or_else(|| {
                if last_cost_moved.is_empty() {
                    None
                } else {
                    Some(last_cost_moved.clone())
                }
            });

        let exclude_self_id = if effect.exclude_self_any().unwrap_or(false) {
            gs.activating_card
        } else {
            None
        };

        if effect.target_count_any().is_some()
            && !is_self_target
            && (selected_for_current.is_empty() || effect.distinct_any().is_some())
            && !per_unit
            && (resource == "blade"
                || resource == "ブレード"
                || resource == "heart"
                || resource == "ハート")
        {
            let stage_ids: Vec<i16> = {
                let p = gs.resolve_target_player(&target);
                p.stage
                    .stage
                    .iter()
                    .copied()
                    .filter(|&id| id != -1)
                    .collect()
            };
            let mut prelim_filter = effect.filter_subset();
            prelim_filter.exclude_self = exclude_self_id;
            let exclude_names: Vec<String> = effect
                .exclude_by_name_source_any()
                .as_deref()
                .filter(|&s| s == "preceding_moved")
                .and_then(|_| preceding_moved.as_ref())
                .map(|moved| {
                    moved
                        .iter()
                        .filter_map(|&cid| card_db.get_card(cid).map(|c| c.name.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            if !exclude_names.is_empty() {
                prelim_filter.exclude_names = Some(&exclude_names);
            }
            let choice_exclude = if (effect.target_count_any().is_some()
                || effect.distinct_any().is_some())
                && !all_selected.is_empty()
            {
                Some(all_selected.as_slice())
            } else {
                None
            };
            let mut candidates = util::matching_ids_filtered(
                &stage_ids,
                &card_db,
                &prelim_filter,
                true,
                None,
                if resource == "blade"
                    || resource == "ブレード"
                    || resource == "heart"
                    || resource == "ハート"
                {
                    effect.distinct_any()
                } else {
                    None
                },
                choice_exclude,
            );
            // "ブレードをNつ以上持つ" (no 元々) — filter by CURRENT blade total
            // (base or set + modifiers); matches() only handles printed values.
            if prelim_filter.has_current_blade_filter() {
                candidates = util::filter_current_blade(
                    candidates,
                    gs,
                    prelim_filter.current_blade_limit,
                    prelim_filter.current_blade_operator,
                );
            }
            // Filter target_count candidates by position if specified.
            if let Some(ref pos) = effect.position_any() {
                if let Some(p) = pos.get_position() {
                    if let Some(stage_idx) = util::stage_position_index(p) {
                        let p = gs.resolve_target_player(&target);
                        let expected = p.stage.stage.get(stage_idx).copied().unwrap_or(-1);
                        candidates.retain(|cid| *cid == expected);
                    }
                }
            }
            // Filter candidates by group_reference: "same_group_name" — use the
            // cost-discarded card's group (c.group, card position ②) so the target
            // prompt shows only members matching that group name.
            if effect.group_reference_any().as_deref() == Some("same_group_name") {
                let ref_group: Option<String> = self
                    .moved_cards
                    .first()
                    .and_then(|cid| gs.card_database.get_card(*cid))
                    .map(|c| c.group.to_string());
                if let Some(ref group) = ref_group {
                    candidates.retain(|cid| {
                        util::card_matches_group_str(&gs.card_database, *cid, Some(group.as_str()))
                    });
                }
            }
            let tc = effect.target_count_any().unwrap_or(1) as usize;
            if candidates.len() > tc {
                let stage_snapshot: Vec<i16> = {
                    let p = gs.resolve_target_player(&target);
                    p.stage.stage.to_vec()
                };
                let filtered_indices: Vec<usize> = candidates
                    .iter()
                    .filter_map(|&cid| stage_snapshot.iter().position(|&s| s == cid))
                    .collect();
                let mut saved = effect.clone();
                saved.set_target_count(None);
                self.selected_count_at_save = Some(self.selected_cards.len() as u8);
                let mut pending = gs.ability_queue.take_pending_actions();
                pending.insert(0, saved);
                gs.ability_queue.set_pending_actions(pending);
                let desc_en = format!("Select {} card(s) to receive {} {}", tc, count, resource);
                let resource_label =
                    crate::ability::describe::resource_label_ja(Some(resource.as_str()));
                let desc_ja = format!(
                    "リソースを受け取る{}枚のカードを選択（{} {}）",
                    tc, count, resource_label
                );
                self.pending_choice = Some(
                    Choice::select_cards(Zone::Stage.to_str().to_string(), tc, desc_en, false)
                        .description_ja(Some(desc_ja))
                        .card_type(effect.card_type_any().map(|s| s.to_string()))
                        .group(effect.group_name().map(|s| s.to_string()))
                        .characters(effect.characters_any().cloned())
                        .filtered_indices(Some(filtered_indices))
                        .target_player_id(Some(target.clone()))
                        .is_select_action(true)
                        .build(),
                );
                // Don't call store_pending_choice — keep self.pending_choice set
                // so the caller (e.g. resume_pending_commands) can detect the
                // sub-choice and properly save remaining commands before returning.
                self.sub_choice_created = true;
                return Ok(());
            }
        }

        let orientation_modifiers = gs.mods.orientation_modifiers.clone();
        let last_energy = gs.mods.last_cost_energy_count;
        // Issue 6: Pre-compute appeared/moved-this-turn sets before mutable borrow
        let appeared_ids: HashSet<i16> =
            if effect.timing_condition_any().as_deref() == Some("appeared_this_turn") {
                let p = gs.resolve_target_player(&target);
                p.stage
                    .stage
                    .iter()
                    .filter(|&&cid| cid != -1 && gs.has_card_appeared_this_turn(cid))
                    .copied()
                    .collect()
            } else if effect.timing_condition_any().as_deref() == Some("moved_this_turn") {
                let area_moved_ids: HashSet<i16> = gs
                    .turn_area_movements
                    .iter()
                    .map(|m| m.moved_card_id)
                    .collect();
                let p = gs.resolve_target_player(&target);
                p.stage
                    .stage
                    .iter()
                    .filter(|&&cid| {
                        if cid == -1 {
                            return false;
                        }
                        // Prefer turn_area_movements (precise area-move tracking),
                        // fall back to cards_moved_this_turn for backward compat.
                        if !area_moved_ids.is_empty() {
                            area_moved_ids.contains(&cid)
                        } else {
                            gs.cards_moved_this_turn.iter().any(|x| x == &cid)
                        }
                    })
                    .copied()
                    .collect()
            } else {
                HashSet::default()
            };
        let (blade_targets, mut heart_targets, heart_color_str, final_count) = {
            let mut filter = effect.filter_subset();
            filter.exclude_self = exclude_self_id;
            let exclude_names: Vec<String> = effect
                .exclude_by_name_source_any()
                .as_deref()
                .filter(|&s| s == "preceding_moved")
                .and_then(|_| preceding_moved.as_ref())
                .map(|moved| {
                    moved
                        .iter()
                        .filter_map(|&cid| card_db.get_card(cid).map(|c| c.name.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            if !exclude_names.is_empty() {
                filter.exclude_names = Some(&exclude_names);
            }

            let final_count = self.calculate_gain_multiplier(
                gs,
                effect,
                per_unit,
                count,
                per_unit_type_str.as_deref(),
                &target,
                &recently_moved,
                &entry_snapshot,
                last_energy,
                last_discard_count,
                &orientation_modifiers,
                &filter,
            );

            // "ブレードをNつ以上持つ" (no 元々) — CURRENT blade total filter
            // (base or set + modifiers). Snapshot the eligible stage members
            // BEFORE the mutable `player` borrow (matches() only handles
            // printed/original blade values).
            let current_blade_eligible: SmallVec<[i16; 8]> = if filter.has_current_blade_filter() {
                let stage_ids: Vec<i16> = {
                    let p = gs.resolve_target_player(&target);
                    util::zone_cards(p, Zone::Stage.to_str()).to_vec()
                };
                util::filter_current_blade(
                    stage_ids,
                    gs,
                    filter.current_blade_limit,
                    filter.current_blade_operator,
                )
                .into()
            } else {
                SmallVec::new()
            };

            let player = gs.resolve_target_player_mut(&target);

            let has_selection_filter =
                effect.target_count_any().is_some() || effect.distinct_any().is_some();
            // When a distinct choice was saved (target_count cleared), exclude
            // only cards selected BEFORE the choice, not the card selected BY it.
            let saved_exclude: Option<SmallVec<[i16; 8]>> = if effect.target_count_any().is_none()
                && effect.distinct_any().is_some()
                && !all_selected.is_empty()
            {
                if let Some(save_len) = self.selected_count_at_save {
                    if (save_len as usize) < all_selected.len() {
                        let prev: SmallVec<[i16; 8]> =
                            all_selected[..save_len as usize].iter().copied().collect();
                        if !prev.is_empty() {
                            Some(prev)
                        } else {
                            None
                        }
                    } else {
                        Some(all_selected.clone())
                    }
                } else {
                    Some(all_selected.clone())
                }
            } else {
                None
            };
            let exclude: Option<&[i16]> = if let Some(ref saved) = saved_exclude {
                Some(saved.as_slice())
            } else if has_selection_filter && !all_selected.is_empty() {
                Some(all_selected.as_slice())
            } else {
                None
            };
            let tc = effect.target_count_any();
            let dn = effect.distinct_any();

            let has_characters = effect.characters_any().is_some_and(|c| !c.is_empty());
            let has_blade_filter =
                card_type_filter.is_some() || group_filter.is_some() || has_characters;
            // When has_selection_filter is set (target_count/distinct), don't blindly
            // use all_selected — apply the filter with exclusion to find the right targets.
            // Only use all_selected directly for pure sequential select→gain_resource.
            // When distinct is set, the saved action must filter by exclude to
            // prevent cards selected in previous steps from also getting the resource.
            let use_raw = !all_selected.is_empty()
                && !has_selection_filter
                && effect.distinct_any().is_none();
            let mut all_candidates: SmallVec<[i16; 8]> = if use_raw {
                all_selected.clone()
            } else if has_blade_filter || is_all {
                util::matching_ids_filtered(
                    util::zone_cards(player, Zone::Stage.to_str()),
                    &card_db,
                    &filter,
                    true,
                    None, // don't truncate yet — we may need a player choice
                    if resource == "blade" || resource == "ブレード" {
                        dn
                    } else {
                        None
                    },
                    exclude,
                )
                .into()
            } else {
                SmallVec::new()
            };

            // "ブレードをNつ以上持つ" (no 元々) — restrict to the snapshot of
            // current-blade-eligible stage members computed before the borrow.
            if filter.has_current_blade_filter() {
                all_candidates.retain(|cid| current_blade_eligible.contains(cid));
            }

            // Filter by position if the effect specifies one (e.g. "center").
            if let Some(ref pos) = effect.position_any() {
                if let Some(p) = pos.get_position() {
                    if let Some(stage_idx) = util::stage_position_index(p) {
                        let expected = player.stage.stage[stage_idx];
                        all_candidates.retain(|cid| *cid == expected);
                    }
                }
            }
            // Issue 6: Filter by timing_condition (e.g. "appeared_this_turn")
            log::debug!(
                "[APP_IDS] appeared_ids={:?} all_candidates before={:?}",
                appeared_ids,
                all_candidates
            );
            if effect.timing_condition_any().is_some() {
                all_candidates.retain(|cid| appeared_ids.contains(cid));
                log::debug!("[APP_IDS] all_candidates after={:?}", all_candidates);
            }
            // same_name: filter candidates to only members whose name matches
            // the card(s) moved as cost (self.moved_cards).
            if effect.same_name_any().unwrap_or(false) {
                let ref_names: Vec<String> = self
                    .moved_cards
                    .iter()
                    .filter_map(|&cid| card_db.get_card(cid).map(|c| c.name.to_string()))
                    .collect();
                log::debug!(
                    "[SAME_NAME] ref_names={:?} before={}",
                    ref_names,
                    all_candidates.len()
                );
                if !ref_names.is_empty() {
                    all_candidates.retain(|cid| {
                        card_db
                            .get_card(*cid)
                            .map(|c| ref_names.contains(&c.name.to_string()))
                            .unwrap_or(false)
                    });
                } else {
                    all_candidates.clear();
                }
                log::debug!("[SAME_NAME] after={}", all_candidates.len());
            }

            // If target_count is set and more candidates than needed,
            // create a choice for the player (unless already selected via previous choice).
            log::debug!("[GAIN_RESOURCE] res={} is_all={} has_filter={} tc={:?} dn={:?} all_cand={} selected={}",
                resource, is_all, has_blade_filter, tc, dn, all_candidates.len(), self.selected_cards.len());
            log::debug!(
                "[GAIN_RESOURCE] blade_targets computation starts ({} candidates)",
                all_candidates.len()
            );
            let blade_targets: SmallVec<[i16; 8]> =
                if effect.target_from_selection_any().unwrap_or(false) {
                    selected_for_current.iter().copied().collect()
                } else if let Some(tgt_count) = tc {
                    if !selected_for_current.is_empty() {
                        selected_for_current
                            .iter()
                            .take(tgt_count as usize)
                            .copied()
                            .collect()
                    } else if (tgt_count as usize) < all_candidates.len() {
                        // Multiple candidates — truncate to target_count for now
                        // (future: create a SelectTarget choice for the player)
                        all_candidates.truncate(tgt_count as usize);
                        all_candidates
                    } else {
                        all_candidates.truncate(tgt_count as usize);
                        all_candidates
                    }
                } else if !selected_for_current.is_empty() && effect.distinct_any().is_none() {
                    selected_for_current.iter().copied().collect()
                } else {
                    all_candidates
                };

            let heart_color_inner = single_fixed_heart
                .clone()
                .or_else(|| effect.heart_color_any().map(|s| s.to_string()))
                .or_else(|| effect.heart_colors_any().first().map(|s| s.to_string()));
            let mut heart_targets: Vec<i16> = if effect.target_from_selection_any().unwrap_or(false)
            {
                // Explicitly target only the cards selected by the preceding
                // sequential action (e.g. a change_state that activated a member).
                log::debug!(
                    "[TARGET_FROM_SEL] heart: selected_cards={:?} selected_for_current={:?} all_selected={:?}",
                    self.selected_cards, selected_for_current, all_selected
                );
                selected_for_current
            } else if use_raw && !selected_for_current.is_empty() && effect.distinct_any().is_none()
            {
                if effect.multiple_targets_any().unwrap_or(false) {
                    let mut targets = selected_for_current;
                    if let Some(aid) = activating_card_id {
                        if !targets.contains(&aid) {
                            targets.push(aid);
                        }
                    }
                    targets
                } else {
                    selected_for_current
                }
            } else if use_raw {
                all_selected.iter().copied().collect()
            } else if resource == "heart" || resource == "ハート" {
                let mut h = if !selected_for_current.is_empty() && effect.distinct_any().is_none() {
                    selected_for_current
                } else if !selected_for_current.is_empty()
                    && effect.target_count_any().is_none()
                    && effect.distinct_any().is_some()
                {
                    // Saved action from distinct choice: target only the
                    // NEWLY selected cards (after the pre-choice save point).
                    if let Some(save_len) = self.selected_count_at_save {
                        if (save_len as usize) < selected_for_current.len() {
                            selected_for_current[save_len as usize..].to_vec()
                        } else {
                            selected_for_current
                        }
                    } else {
                        selected_for_current
                    }
                } else if effect.card_type_any().is_none()
                    && effect.group_names_any().is_none()
                    && effect.characters_any().is_none()
                    && effect.target_count_any().is_none()
                    && effect.distinct_any().is_none()
                    && !is_all
                {
                    // No targeting info: default to activating card only.
                    // Prevents heart from leaking to all stage members when
                    // no card_type/group/characters/target_count filter is set.
                    activating_card_id.map_or(vec![], |id| vec![id])
                } else {
                    util::matching_ids_filtered(
                        util::zone_cards(player, Zone::Stage.to_str()),
                        &card_db,
                        &filter,
                        true,
                        if is_self_target { None } else { tc },
                        dn,
                        exclude,
                    )
                    .to_vec()
                };
                if effect.timing_condition_any().is_some() {
                    h.retain(|cid| appeared_ids.contains(cid));
                }
                // same_name: filter heart_targets to same-name members
                if effect.same_name_any().unwrap_or(false) {
                    let ref_names: Vec<String> = self
                        .moved_cards
                        .iter()
                        .filter_map(|&cid| card_db.get_card(cid).map(|c| c.name.to_string()))
                        .collect();
                    if !ref_names.is_empty() {
                        h.retain(|cid| {
                            card_db
                                .get_card(*cid)
                                .map(|c| ref_names.contains(&c.name.to_string()))
                                .unwrap_or(false)
                        });
                    } else {
                        h.clear();
                    }
                }
                h
            } else {
                vec![]
            };
            if let Some(ref pos) = effect.position_any() {
                if let Some(p) = pos.get_position() {
                    if let Some(stage_idx) = util::stage_position_index(p) {
                        let expected = player.stage.stage[stage_idx];
                        heart_targets.retain(|&cid| cid == expected);
                    }
                }
            }

            // "ブレードをNつ以上持つ" (no 元々) — heart targets are resolved via
            // a SEPARATE matching_ids_filtered call, so apply the current-blade
            // post-filter here as well (the blade_targets path already filters).
            if filter.has_current_blade_filter() {
                heart_targets.retain(|cid| current_blade_eligible.contains(cid));
            }

            // Apply heart_colors as a target filter when the effect
            // specifies that targets must already possess the heart color.
            if effect.filter_targets_by_heart_colors_any().unwrap_or(false)
                && !effect.heart_colors_any().is_empty()
            {
                heart_targets.retain(|&id| {
                    if effect.require_all_heart_colors_any().unwrap_or(false) {
                        util::card_matches_all_heart_colors(&card_db, id, effect.heart_colors_any())
                    } else {
                        util::card_matches_heart_colors(&card_db, id, effect.heart_colors_any())
                    }
                });
            }

            (blade_targets, heart_targets, heart_color_inner, final_count)
        };

        // Store selected card IDs when target_count/distinct is set
        // so the next sequential action can exclude these cards.
        // Only store when we have explicit selection limits to avoid
        // polluting all_selected for blanket effects like "both players gain blade".
        if effect.target_count_any().is_some() || effect.distinct_any().is_some() {
            let selected_targets: Vec<i16> = if resource == "blade" || resource == "ブレード" {
                blade_targets.to_vec()
            } else {
                heart_targets.clone()
            };
            for &cid in &selected_targets {
                if !self.selected_cards.contains(&cid) {
                    self.selected_cards.push(cid);
                }
            }
        }

        let mut effect_data: Option<crate::core::types::EffectData> = None;
        let is_negative = sign == Some("negative");
        let blades_to_add = if is_negative {
            -(final_count as i16)
        } else {
            final_count as i16
        };
        let heart_to_add = if is_negative {
            -(final_count as i16)
        } else {
            final_count as i16
        };
        let heart_color_val =
            crate::card::parse_heart_color(heart_color_str.as_deref().unwrap_or("heart00"));

        // Build heart distribution: for fixed multi-color grants, distribute count
        // across all specified colors instead of using a single color.
        let heart_distribution: Vec<(crate::card::HeartColor, u8)> = if resource == "heart"
            && !heart_selection
            && effect.heart_colors_any().len() > 1
            && final_count >= effect.heart_colors_any().len() as u8
        {
            let per_color = final_count / effect.heart_colors_any().len() as u8;
            effect
                .heart_colors_any()
                .iter()
                .map(|c| (crate::card::parse_heart_color(c), per_color))
                .collect()
        } else {
            vec![(heart_color_val, final_count)]
        };

        if is_self_target {
            if let Some(card_id) = activating_card_id {
                if !gs
                    .resolve_target_player_mut(&target)
                    .stage
                    .stage
                    .contains(&card_id)
                {
                    return Err(
                        "Cannot use self_target on gain_resource: activating card not on stage"
                            .to_string(),
                    );
                }
                if resource == "blade" || resource == "ブレード" {
                    gs.mods.add_blade_modifier_with_trace(
                        card_id,
                        blades_to_add,
                        &mut gs.ability_applications,
                        gs.activating_card.unwrap_or(-1),
                        &effect.text,
                    );
                    if is_temporary {
                        effect_data =
                            Some(Self::make_card_effect_data(card_id, blades_to_add, None));
                    }
                }
                if resource == "heart" || resource == "ハート" {
                    for (color, color_amount) in &heart_distribution {
                        let amount = if is_negative {
                            -(*color_amount as i16)
                        } else {
                            *color_amount as i16
                        };
                        gs.mods.add_heart_modifier_with_trace(
                            card_id,
                            *color,
                            amount,
                            &mut gs.ability_applications,
                            gs.activating_card.unwrap_or(-1),
                            &effect.text,
                        );
                    }
                    if is_temporary && effect_data.is_none() {
                        let mut items: Vec<crate::core::types::CardEffectItem> = Vec::new();
                        for (color, color_amount) in &heart_distribution {
                            let color_name = format!("{:?}", color).to_lowercase();
                            items.push(crate::core::types::CardEffectItem {
                                card_id,
                                amount: *color_amount as i16,
                                color: Some(color_name),
                            });
                        }
                        effect_data = Some(crate::core::types::EffectData::MultiCard { items });
                    }
                }
                if is_temporary {
                    util::push_temporary_effect(
                        gs,
                        &format!("gain_{}", resource),
                        duration.as_deref(),
                        &target,
                        &format!("Gain {} {}", final_count, resource),
                        effect_data,
                    );
                }
                return Ok(());
            }
        }

        let blade_targets_save = blade_targets.clone();
        if resource == "blade" || resource == "ブレード" {
            if blade_targets.is_empty() {
                if is_all
                    && effect.group_names_any().is_none()
                    && effect.card_type_any().is_none()
                    && effect.characters_any().is_none()
                    && effect.timing_condition_any().is_none()
                    && effect.position_any().is_none()
                {
                    let stage_ids: Vec<i16> = {
                        let player = gs.resolve_target_player(&target);
                        player
                            .stage
                            .stage
                            .iter()
                            .copied()
                            .filter(|&id| id != -1)
                            .collect()
                    };
                    for card_id in stage_ids {
                        gs.mods.add_blade_modifier_with_trace(
                            card_id,
                            blades_to_add,
                            &mut gs.ability_applications,
                            gs.activating_card.unwrap_or(-1),
                            &effect.text,
                        );
                    }
                    if is_temporary {
                        effect_data = Some(crate::core::types::EffectData::AllCards {
                            amount: blades_to_add,
                        });
                    }
                } else if effect.position_any().is_some() {
                    // Position-based target: apply to the stage member at that position
                    if resource == "blade" || resource == "ブレード" {
                        if let Some(pos_info) = effect.position_any().as_ref() {
                            if let Some(p) = pos_info.get_position() {
                                if let Some(stage_idx) = util::stage_position_index(p) {
                                    let player = gs.resolve_target_player_mut(&target);
                                    let card_id = player.stage.stage[stage_idx];
                                    if card_id != -1 {
                                        gs.mods.add_blade_modifier_with_trace(
                                            card_id,
                                            blades_to_add,
                                            &mut gs.ability_applications,
                                            gs.activating_card.unwrap_or(-1),
                                            &effect.text,
                                        );
                                        if is_temporary {
                                            effect_data = Some(Self::make_card_effect_data(
                                                card_id,
                                                blades_to_add,
                                                None,
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if effect.target_count_any().is_none()
                    && (effect.exclude_self_any().is_none() || effect.target_any() == Some("self"))
                {
                    if let Some(card_id) = activating_card_id {
                        gs.mods.add_blade_modifier_with_trace(
                            card_id,
                            blades_to_add,
                            &mut gs.ability_applications,
                            gs.activating_card.unwrap_or(-1),
                            &effect.text,
                        );
                        if is_temporary {
                            effect_data =
                                Some(Self::make_card_effect_data(card_id, blades_to_add, None));
                        }
                    }
                }
            } else if !all_selected.is_empty() && effect.source_any().is_none() {
                // Pure sequential select→gain_resource: apply to ALL selected cards with full count
                for &card_id in &blade_targets {
                    gs.mods.add_blade_modifier_with_trace(
                        card_id,
                        blades_to_add,
                        &mut gs.ability_applications,
                        gs.activating_card.unwrap_or(-1),
                        &effect.text,
                    );
                }
            } else {
                let targets: Vec<i16> = if is_all {
                    blade_targets.iter().copied().collect()
                } else {
                    blade_targets
                        .into_iter()
                        .take(final_count as usize)
                        .collect()
                };
                if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed)
                {
                    log::debug!(
                        "[BLADE_APPLY] targets={:?} is_all={} final_count={} blades_to_add={}",
                        targets,
                        is_all,
                        final_count,
                        blades_to_add
                    );
                }
                for &card_id in &targets {
                    gs.mods.add_blade_modifier_with_trace(
                        card_id,
                        blades_to_add,
                        &mut gs.ability_applications,
                        gs.activating_card.unwrap_or(-1),
                        &effect.text,
                    );
                }
            }
        }

        // group_reference: "same_group_name" — filter heart targets to only
        // include cards whose group name (c.group, card position ②) matches the
        // group of the card that was discarded as cost (tracked in self.moved_cards).
        if effect.group_reference_any().as_deref() == Some("same_group_name") {
            log::debug!("[SAME_GROUP] moved_cards={:?}", self.moved_cards);
            let ref_group: Option<String> = self
                .moved_cards
                .first()
                .and_then(|cid| {
                    let c = gs.card_database.get_card(*cid);
                    log::debug!(
                        "[SAME_GROUP] cid={} card={:?}",
                        cid,
                        c.as_ref().map(|c| (&c.name, &c.group))
                    );
                    c
                })
                .map(|c| c.group.to_string());
            log::debug!("[SAME_GROUP] ref_group={:?}", ref_group);
            if let Some(ref group) = ref_group {
                let before = heart_targets.len();
                heart_targets.retain(|cid: &i16| {
                    let matches =
                        util::card_matches_group_str(&gs.card_database, *cid, Some(group.as_str()));
                    log::debug!(
                        "[SAME_GROUP] cid={} matches={} group={}",
                        cid,
                        matches,
                        group
                    );
                    matches
                });
                log::debug!(
                    "[SAME_GROUP] heart_targets: {} -> {}",
                    before,
                    heart_targets.len()
                );
            }
        }

        if resource == "heart" || resource == "ハート" {
            if heart_targets.is_empty() {
                if effect.position_any().is_some() {
                    if let Some(pos_info) = effect.position_any().as_ref() {
                        if let Some(p) = pos_info.get_position() {
                            if let Some(stage_idx) = util::stage_position_index(p) {
                                let player = gs.resolve_target_player_mut(&target);
                                let card_id = player.stage.stage[stage_idx];
                                if card_id != -1 {
                                    self.apply_heart_to_card(
                                        gs,
                                        card_id,
                                        &heart_distribution,
                                        is_negative,
                                        is_temporary,
                                        &mut effect_data,
                                        &heart_color_str,
                                        heart_to_add,
                                        &effect.text,
                                    );
                                }
                            }
                        }
                    }
                } else if effect.target_count_any().is_none()
                    && (effect.exclude_self_any().is_none() || effect.target_any() == Some("self"))
                {
                    if let Some(card_id) = activating_card_id {
                        self.apply_heart_to_card(
                            gs,
                            card_id,
                            &heart_distribution,
                            is_negative,
                            is_temporary,
                            &mut effect_data,
                            &heart_color_str,
                            heart_to_add,
                            &effect.text,
                        );
                    }
                }
            } else if is_self_target
                || (target == "self"
                    && activating_card_id.is_some()
                    && effect.source_any().is_none()
                    && effect.card_type_any().is_none()
                    && !effect.target_from_selection_any().unwrap_or(false)
                    && !effect.multiple_targets_any().unwrap_or(false))
            {
                if let Some(card_id) = activating_card_id {
                    self.apply_heart_to_card(
                        gs,
                        card_id,
                        &heart_distribution,
                        is_negative,
                        is_temporary,
                        &mut effect_data,
                        &heart_color_str,
                        heart_to_add,
                        &effect.text,
                    );
                }
            } else {
                let targets: Vec<i16> = if is_all || effect.multiple_targets_any().unwrap_or(false)
                {
                    heart_targets.clone()
                } else {
                    heart_targets
                        .into_iter()
                        .take(final_count as usize)
                        .collect()
                };
                log::debug!(
                    "[HEART_APPLY] targets={:?} is_all={} final_count={}",
                    targets,
                    is_all,
                    final_count
                );
                for &card_id in &targets {
                    log::debug!(
                        "[HEART_APPLY] adding heart04 to card_id={}, activating={:?}",
                        card_id,
                        gs.activating_card
                    );
                    for &(color, _) in &heart_distribution {
                        log::debug!(
                            "[HEART_APPLY]   color={:?} current_mod={}",
                            color,
                            gs.mods.get_heart_modifier(card_id, color)
                        );
                    }
                    for &(color, dist_count) in &heart_distribution {
                        let dist_amount = if is_negative {
                            -(dist_count as i16)
                        } else {
                            dist_count as i16
                        };
                        gs.mods.add_heart_modifier_with_trace(
                            card_id,
                            color,
                            dist_amount,
                            &mut gs.ability_applications,
                            gs.activating_card.unwrap_or(-1),
                            &effect.text,
                        );
                    }
                }
                // Build effect_data for heart cleanup on expiry
                if is_temporary && effect_data.is_none() && !targets.is_empty() {
                    if heart_distribution.len() > 1 {
                        let items: Vec<crate::core::types::CardEffectItem> = targets
                            .iter()
                            .flat_map(|&cid| {
                                heart_distribution.iter().map(move |&(c, dc)| {
                                    let amount = if is_negative { -(dc as i16) } else { dc as i16 };
                                    crate::core::types::CardEffectItem {
                                        card_id: cid,
                                        amount,
                                        color: Some(format!("{:?}", c)),
                                    }
                                })
                            })
                            .collect();
                        effect_data = Some(crate::core::types::EffectData::MultiCard { items });
                    } else {
                        let color_name = heart_color_str.as_deref().unwrap_or("heart01");
                        let items: Vec<crate::core::types::CardEffectItem> = targets
                            .iter()
                            .map(|&cid| crate::core::types::CardEffectItem {
                                card_id: cid,
                                amount: heart_to_add,
                                color: Some(color_name.to_string()),
                            })
                            .collect();
                        effect_data = Some(crate::core::types::EffectData::MultiCard { items });
                    }
                }
            }
        }

        // Store effect_data for blade cleanup.
        if is_temporary && effect_data.is_none() && (resource == "blade" || resource == "ブレード")
        {
            let items: Vec<crate::core::types::CardEffectItem> = blade_targets_save
                .iter()
                .map(|&cid| crate::core::types::CardEffectItem {
                    card_id: cid,
                    amount: final_count as i16,
                    color: None,
                })
                .collect();
            effect_data = Some(crate::core::types::EffectData::MultiCard { items });
        }

        // Resource gain details captured in structured ability_resolution entry
        if is_temporary {
            util::push_temporary_effect(
                gs,
                &format!("gain_{}", resource),
                duration.as_deref(),
                &target,
                &format!("Gain {} {}", final_count, resource),
                effect_data,
            );
        }
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log(format!(
            "{} {}: [[log_gain_resource:n={},type={}]]",
            pp,
            act_name,
            effect.count_any().unwrap_or(1),
            effect.resource_any().as_deref().unwrap_or("?")
        ));
        Ok(())
    }

    pub(crate) fn execute_play_baton_touch(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let count: u8 = effect.count_or(1) as u8;
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
            self.pending_choice = Some(Choice::SelectTarget {
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

    pub fn execute_place_energy_under_member(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) {
        self.execute_place_energy_under_member_impl(gs, effect, false)
    }

    /// Cost-path variant: forces optional=false to avoid re-prompting the
    /// optional-cost choice (infinite-loop guard used by cost.rs re-entry).
    pub fn execute_place_energy_under_member_non_optional(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) {
        self.execute_place_energy_under_member_impl(gs, effect, true)
    }

    fn execute_place_energy_under_member_impl(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        force_non_optional: bool,
    ) {
        // Resolve the count (dynamic_count overrides energy_count).
        let count: u8 = if let Some(ref dc) = effect.dynamic_count_any() {
            self.resolve_dynamic_count(gs, dc)
        } else {
            effect.energy_count_any().unwrap_or(1) as u8
        };
        let target = effect.target_name().to_string();
        let optional = !force_non_optional && effect.optional.unwrap_or(false);
        let source = effect.source_any().map(|s| s.to_string());
        let any_number = effect.any_number_any().unwrap_or(false);

        // Special case: source="under_member" + destination="energy_zone" means
        // count from under member, but move from energy_deck → energy_zone (wait).
        // e.g. PL!N-bp5-012-R+ LiveSuccess: place (under_count + 1) from deck.
        if source.as_deref() == Some("under_member")
            && effect.destination.as_deref() == Some("energy_zone")
        {
            let player = gs.resolve_target_player_mut(&target);
            for _ in 0..count {
                if let Some(energy) = player.energy_deck.draw() {
                    player.energy_zone.cards.push(energy);
                    // Don't increment active_energy_count — wait state
                } else {
                    break;
                }
            }
            return;
        }

        // Special case: deploy from under_member to empty_area
        // (e.g. PL!-bp6-003-R+ LiveSuccess)
        if source.as_deref() == Some("under_member")
            && effect.destination.as_deref() == Some("empty_area")
        {
            let player = gs.resolve_target_player(&target);
            let has_empty_slot = (0..3).any(|i| player.stage.stage[i] == -1);
            if !has_empty_slot {
                return;
            }
            let pos = gs
                .activating_card
                .and_then(|c| player.stage.stage.iter().position(|&id| id == c))
                .unwrap_or(1);
            let area = match pos {
                0 => crate::zones::MemberArea::LeftSide,
                1 => crate::zones::MemberArea::Center,
                _ => crate::zones::MemberArea::RightSide,
            };
            let under_cards = player.stage.get_under_cards(area);
            if under_cards.is_empty() {
                return;
            }
            let target_str = target.clone();
            let desc_ja = "このメンバーの下から出すメンバーカードを選択".to_string();
            let mut b = Choice::select_cards(
                Zone::UnderMember.to_str(),
                count as usize,
                "Select a member card to deploy from under this member",
                optional,
            )
            .description_ja(Some(desc_ja))
            .card_type(effect.card_type_any().map(|s| s.to_string()))
            .target_player_id(Some(target_str));
            if let Some(ref groups) = effect.group_names_any() {
                if let Some(first) = groups.first() {
                    b = b.group(Some(first.clone()));
                }
            }
            b = b.cost_limit(
                effect.cost_limit_any().map(|v| v as u8),
                effect.cost_limit_operator_any().map(|s| s.to_string()),
            );
            self.pending_choice = Some(b.build());
            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            return;
        }

        if Zone::from_str(source.as_deref().unwrap_or("")) == Some(Zone::UnderMember) {
            let player = gs.resolve_target_player_mut(&target);
            let mut all_under: Vec<i16> = Vec::new();
            for si in 0..3 {
                for &cid in &player.stage.under_cards[si] {
                    all_under.push(cid);
                }
            }
            if all_under.is_empty() {
                return;
            }

            // any_number → count=0 so the player can select any subset
            // (the choice handler re-prompts after each selection).
            // Fixed count → require exactly that many (up to max available).
            let choice_count = if any_number {
                0
            } else {
                all_under.len().min(count as usize)
            };
            self.pending_choice = Some(
                Choice::select_cards(
                    Zone::UnderMember.to_str(),
                    choice_count,
                    "Select energy cards to move from under member to energy deck",
                    optional,
                )
                .description_ja(Some(
                    "メンバーの下からエネルギーデッキに戻すエネルギーカードを選択".to_string(),
                ))
                .card_type(Some("energy_card".to_string()))
                .target_player_id(Some(target.to_string()))
                .build(),
            );
            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            return;
        }

        // Original logic: move from energy_zone to under_member
        // This should only execute if source is NOT "under_member"
        // Ask the player to tap the specific energy card(s) to place under the
        // member. Both active and waited energy cards are selectable. Skippable
        // only when the placement is optional (single prompt, no separate
        // pay/skip gate).
        if gs.resolve_target_player(&target).energy_zone.cards.is_empty() {
            return;
        }
        let desc_ja = if count == 1 {
            "このメンバーの下に置くエネルギーカードを選択".to_string()
        } else {
            format!("このメンバーの下に置くエネルギーカードを{}枚選択", count)
        };
        self.pending_choice = Some(
            Choice::select_cards(
                Zone::Energy.to_str(),
                count as usize,
                "Choose energy card(s) to place under member",
                optional,
            )
            .destination(Some("under_member".to_string()))
            .card_type(Some("energy_card".to_string()))
            .target_player_id(Some(target.clone()))
            .description_ja(Some(desc_ja))
            .build(),
        );
        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
        return;
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
        // Check source_position from effect (new parser field), fall back to position param
        let source_pos_binding = effect.source_position_any();
        let source_pos = source_pos_binding
            .as_deref()
            .or_else(|| position.as_ref().and_then(|p| p.get_position()));
        let position_str = source_pos.unwrap_or("");

        // If destination is already specified (from conditional position_change or area_select),
        // route directly to execute_position_change_with_destination.
        // EXCEPTION: "front" destination for opponent needs source selection first.
        if let Some(ref dest) = effect.destination_any() {
            if &**dest == "front" && target == "opponent" {
                // "front" destination for opponent: the destination is fixed (front area of
                // activating card). Create a choice to select which OPPONENT member to move.
                let valid_sources: Vec<String> = {
                    let player = gs.resolve_target_player(target);
                    (0..3)
                        .filter(|&i| player.stage.stage[i] != -1)
                        .map(|i| {
                            match i {
                                0 => "left",
                                1 => "center",
                                _ => "right",
                            }
                            .to_string()
                        })
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
                self.pending_choice = Some(Choice::SelectTarget {
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
                        let pos = match i {
                            0 => "left",
                            1 => "center",
                            _ => "right",
                        };
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
            self.pending_choice = Some(Choice::SelectTarget {
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
            if self.pending_choice.is_some() {
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
                self.formation_plan = card_ids.iter().map(|&cid| (cid, String::new())).collect();

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
                    .unwrap_or_else(|| "member".to_string().into());

                let valid_destinations =
                    self.compute_valid_position_destinations(gs, effect, target_m);
                if valid_destinations.is_empty() {
                    self.formation_plan.clear();
                    return Ok(());
                }
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.choice_card_no = Some(ChoiceRoute::Raw(format!(
                        "position_change:self:{}",
                        first_card_id
                    )));
                }
                self.pending_choice = Some(Choice::SelectTarget {
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
                    _ => &position_str,
                };
                self.pending_choice = Some(Choice::SelectTarget {
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
            let stored_area = self.selected_area.clone();
            if let Some(ref area) = stored_area {
                self.selected_area = None;
                let mut copy = effect.clone();
                copy.destination = Some(ArcStr::from(area.as_str()));
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
            self.pending_choice = Some(Choice::SelectTarget {
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
                .as_deref()
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
                    .unwrap_or_else(|| "member".to_string().into());
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
                self.pending_choice = Some(Choice::SelectTarget {
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
            let from_area = match current_idx {
                0 => crate::zones::MemberArea::LeftSide,
                1 => crate::zones::MemberArea::Center,
                _ => crate::zones::MemberArea::RightSide,
            };
            let to_area = match target_index {
                0 => crate::zones::MemberArea::LeftSide,
                1 => crate::zones::MemberArea::Center,
                _ => crate::zones::MemberArea::RightSide,
            };
            let (target_id, source_id) = (
                player.stage.stage[target_index],
                player.stage.stage[current_idx],
            );
            if player.stage.position_change(from_area, to_area).is_ok() {
                // Push events BEFORE push_movement_event so the
                // PositionChangeEvent list captures the change before the
                // self-trigger fires from within push_movement_event.
                let source_old = current_idx as u8;
                let source_new = target_index as u8;
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
                        gs.push_movement_event(cid, "stage", "stage", cause_cid, &mover_pid, true);
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
        let pid = gs
            .ability_queue
            .current_entry()
            .map(|e| e.player_id.clone())
            .unwrap_or_default();
        gs.trigger_auto_abilities_for_player_with_event(
            &pid,
            &crate::ability::types::TriggerEvent {
                moved_cards: gs.recently_moved_cards.clone().unwrap_or_default().into(),
                position_change_occurred: gs.position_change_occurred_this_turn,
                ..Default::default()
            },
        );
        gs.mark_constants_dirty();
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
            .formation_plan
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

        valid
    }

    /// Execute all formation change swaps as a batch after all members have
    /// been assigned destinations via `formation_plan`.  Each swap is executed
    /// via `stage.position_change` and each member's movement is individually
    /// tracked via `push_movement_event`.
    pub(crate) fn finalize_formation_change(&mut self, gs: &mut GameState) -> Result<(), String> {
        if self.formation_plan.is_empty() {
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
        let mut occupant: [i16; 3] = old_stage.clone(); // current occupant of each pos
        for &(member_id, ref dest) in &self.formation_plan {
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
            events.push((member_id, from_idx as u8, dest_idx as u8));
        }

        // Phase 2: place evicted cards / stay-in-place.
        // Each evicted card goes to its mover's original position.
        for &(member_id, ref dest) in &self.formation_plan {
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
                // Evicted occupant goes to the mover's vacated position
                let evicted_id = old_stage[dest_idx];
                if evicted_id != -1 && evicted_id != member_id && new_stage[from_idx] == -1 {
                    new_stage[from_idx] = evicted_id;
                    new_under[from_idx] = old_under[dest_idx].clone();
                    events.push((evicted_id, dest_idx as u8, from_idx as u8));
                }
            }
        }

        // Phase 3: unplanned cards keep their original slot if free
        for (i, &cid) in old_stage.iter().enumerate() {
            if new_stage[i] == -1 && cid != -1 {
                let is_planned = self.formation_plan.iter().any(|(id, _)| *id == cid);
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
        gs.mark_constants_dirty();
        gs.recalculate_constants();
        self.formation_plan.clear();
        Ok(())
    }

    pub fn execute_position_change_with_destination(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        destination: &str,
    ) -> Result<(), String> {
        let raw_target = effect.target_any().unwrap_or("self");
        let target = if raw_target == "both" {
            "self"
        } else {
            raw_target
        };
        let target_member_binding = effect.target_member_any();
        let target_member = target_member_binding.unwrap_or("this_member");
        let sp_binding = effect.source_position_any();
        let source_position = sp_binding.as_deref().or_else(|| {
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
            self.activating_card_id
        );
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
        let source_position = sp_binding.as_deref().or_else(|| {
            effect
                .position_any()
                .as_ref()
                .and_then(|p| p.get_position())
        });

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
                let area = match idx {
                    0 => crate::zones::MemberArea::LeftSide,
                    1 => crate::zones::MemberArea::Center,
                    _ => crate::zones::MemberArea::RightSide,
                };
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
            let from_area2 = match source_idx {
                0 => crate::zones::MemberArea::LeftSide,
                1 => crate::zones::MemberArea::Center,
                _ => crate::zones::MemberArea::RightSide,
            };
            let to_area2 = match target_index {
                0 => crate::zones::MemberArea::LeftSide,
                1 => crate::zones::MemberArea::Center,
                _ => crate::zones::MemberArea::RightSide,
            };
            let (target_id2, source_id2) = (
                player.stage.stage[target_index],
                player.stage.stage[source_idx],
            );
            player.stage.position_change(from_area2, to_area2)?;
            let _ = player;
            gs.record_card_movement(target_id2);
            if source_id2 != -1 {
                gs.record_card_movement(source_id2);
            }
            let mover_pid = gs
                .ability_queue
                .current_entry()
                .map(|e| e.player_id.clone())
                .unwrap_or_default();
            // Push events BEFORE push_movement_event
            if source_id2 != -1 {
                gs.position_change_events
                    .push(crate::types::PositionChangeEvent {
                        moved_card_id: source_id2,
                        old_position: source_idx as u8,
                        new_position: target_index as u8,
                        cause_card_id: gs.activating_card,
                        cause_player_id: mover_pid.clone(),
                        effect_only: true,
                    });
            }
            if target_id2 != -1 {
                gs.position_change_events
                    .push(crate::types::PositionChangeEvent {
                        moved_card_id: target_id2,
                        old_position: target_index as u8,
                        new_position: source_idx as u8,
                        cause_card_id: gs.activating_card,
                        cause_player_id: mover_pid.clone(),
                        effect_only: true,
                    });
            }
            gs.push_movement_event(
                source_id2,
                "stage",
                "stage",
                gs.activating_card,
                &mover_pid,
                true,
            );
            if target_id2 != -1 {
                gs.push_movement_event(
                    target_id2,
                    "stage",
                    "stage",
                    gs.activating_card,
                    &mover_pid,
                    true,
                );
            }
            gs.trigger_auto_abilities_for_player_with_event(
                &mover_pid,
                &crate::ability::types::TriggerEvent {
                    moved_cards: gs.recently_moved_cards.clone().unwrap_or_default().into(),
                    position_change_occurred: gs.position_change_occurred_this_turn,
                    ..Default::default()
                },
            );
            gs.mark_constants_dirty();
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
                    let from_area = match current_idx {
                        0 => crate::zones::MemberArea::LeftSide,
                        1 => crate::zones::MemberArea::Center,
                        _ => crate::zones::MemberArea::RightSide,
                    };
                    let to_area = match target_index {
                        0 => crate::zones::MemberArea::LeftSide,
                        1 => crate::zones::MemberArea::Center,
                        _ => crate::zones::MemberArea::RightSide,
                    };
                    let (target_id, source_id) = (
                        player.stage.stage[target_index],
                        player.stage.stage[current_idx],
                    );
                    player.stage.position_change(from_area, to_area)?;
                    let _ = player;
                    gs.record_card_movement(target_id);
                    if source_id != -1 {
                        gs.record_card_movement(source_id);
                    }
                    let mover_pid = gs
                        .ability_queue
                        .current_entry()
                        .map(|e| e.player_id.clone())
                        .unwrap_or_default();
                    // Push events BEFORE push_movement_event
                    if source_id != -1 {
                        gs.position_change_events
                            .push(crate::types::PositionChangeEvent {
                                moved_card_id: source_id,
                                old_position: current_idx as u8,
                                new_position: target_index as u8,
                                cause_card_id: gs.activating_card,
                                cause_player_id: mover_pid.clone(),
                                effect_only: true,
                            });
                    }
                    if target_id != -1 {
                        gs.position_change_events
                            .push(crate::types::PositionChangeEvent {
                                moved_card_id: target_id,
                                old_position: target_index as u8,
                                new_position: current_idx as u8,
                                cause_card_id: gs.activating_card,
                                cause_player_id: mover_pid.clone(),
                                effect_only: true,
                            });
                    }
                    gs.push_movement_event(
                        source_id,
                        "stage",
                        "stage",
                        gs.activating_card,
                        &mover_pid,
                        true,
                    );
                    if target_id != -1 {
                        gs.push_movement_event(
                            target_id,
                            "stage",
                            "stage",
                            gs.activating_card,
                            &mover_pid,
                            true,
                        );
                    }
                    gs.trigger_auto_abilities_for_player_with_event(
                        &mover_pid,
                        &crate::ability::types::TriggerEvent {
                            moved_cards: gs.recently_moved_cards.clone().unwrap_or_default().into(),
                            position_change_occurred: gs.position_change_occurred_this_turn,
                            ..Default::default()
                        },
                    );
                    gs.mark_constants_dirty();
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
            if let Some(activating_card_id) = self.activating_card_id {
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
                    let from_area3 = match current_idx {
                        0 => crate::zones::MemberArea::LeftSide,
                        1 => crate::zones::MemberArea::Center,
                        _ => crate::zones::MemberArea::RightSide,
                    };
                    let to_area3 = match target_index {
                        0 => crate::zones::MemberArea::LeftSide,
                        1 => crate::zones::MemberArea::Center,
                        _ => crate::zones::MemberArea::RightSide,
                    };
                    let (target_id3, source_id3) = (
                        player.stage.stage[target_index],
                        player.stage.stage[current_idx],
                    );
                    player.stage.position_change(from_area3, to_area3)?;
                    let _ = player;
                    gs.record_card_movement(target_id3);
                    if source_id3 != -1 {
                        gs.record_card_movement(source_id3);
                    }
                    let mover_pid = gs
                        .ability_queue
                        .current_entry()
                        .map(|e| e.player_id.clone())
                        .unwrap_or_default();
                    // Push events BEFORE push_movement_event
                    gs.position_change_events
                        .push(crate::types::PositionChangeEvent {
                            moved_card_id: activating_card_id,
                            old_position: current_idx as u8,
                            new_position: target_index as u8,
                            cause_card_id: gs.activating_card,
                            cause_player_id: mover_pid.clone(),
                            effect_only: true,
                        });
                    if target_id3 != -1 {
                        gs.position_change_events
                            .push(crate::types::PositionChangeEvent {
                                moved_card_id: target_id3,
                                old_position: target_index as u8,
                                new_position: current_idx as u8,
                                cause_card_id: gs.activating_card,
                                cause_player_id: mover_pid.clone(),
                                effect_only: true,
                            });
                    }
                    gs.push_movement_event(
                        activating_card_id,
                        "stage",
                        "stage",
                        gs.activating_card,
                        &mover_pid,
                        true,
                    );
                    if target_id3 != -1 {
                        gs.push_movement_event(
                            target_id3,
                            "stage",
                            "stage",
                            gs.activating_card,
                            &mover_pid,
                            true,
                        );
                    }
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
        let pid = gs
            .ability_queue
            .current_entry()
            .map(|e| e.player_id.clone())
            .unwrap_or_default();
        gs.trigger_auto_abilities_for_player_with_event(
            &pid,
            &crate::ability::types::TriggerEvent {
                moved_cards: gs.recently_moved_cards.clone().unwrap_or_default().into(),
                position_change_occurred: gs.position_change_occurred_this_turn,
                ..Default::default()
            },
        );
        gs.mark_constants_dirty();
        gs.recalculate_constants();
        Ok(())
    }

    pub fn execute_rotation(
        &mut self,
        gs: &mut GameState,
        _effect: &AbilityEffect,
        target: &str,
    ) -> Result<(), String> {
        let tgt = if target == "both" { "self" } else { target };
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
                positions.push((src_idx as u8, dest_idx as u8));
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

        gs.position_change_occurred_this_turn = true;
        gs.mark_constants_dirty();
        gs.recalculate_constants();
        let pid = gs
            .ability_queue
            .current_entry()
            .map(|e| e.player_id.clone())
            .unwrap_or_default();
        gs.trigger_auto_abilities_for_player_with_event(
            &pid,
            &crate::ability::types::TriggerEvent {
                moved_cards: gs.recently_moved_cards.clone().unwrap_or_default().into(),
                position_change_occurred: gs.position_change_occurred_this_turn,
                ..Default::default()
            },
        );
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: [[log_rotation]]", pp, act_name));
        Ok(())
    }

    pub(crate) fn execute_choice(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let co_binding = effect.choice_options_any();
        let choice_options = co_binding.as_ref();
        let ct_binding = effect.choice_type_any();
        let choice_type = ct_binding.as_deref();
        let opt_binding = effect.options_any();
        let options = opt_binding.as_ref();
        let cm_binding = effect.choice_maker_any();
        let choice_maker = cm_binding.as_deref();
        // If a selection was already made (from a prior choice resolution),
        // execute the selected option's effect instead of creating another choice.
        if let Some(effect_options) = options {
            if let Some(entry) = gs.ability_queue.current_entry() {
                if let Some(ConditionalChoice::Str(ref cc)) = entry.conditional_choice {
                    if let Ok(idx) = cc.parse::<usize>() {
                        if idx < effect_options.len() {
                            let selected = &effect_options[idx];
                            return self.execute_effect(gs, selected);
                        }
                    }
                }
            }
        }
        if let Some(alt_options) = choice_options {
            if let Some(entry) = gs.ability_queue.current_entry() {
                if let Some(ConditionalChoice::Str(ref cc)) = entry.conditional_choice {
                    if alt_options.contains(cc) {
                        // String-based choice matched — nothing to execute, choice was just informational
                        return Ok(());
                    }
                }
            }
        }
        // Propagate parent choice's group_names to each child option when the
        // group_names is a selection filter (not a condition threshold).  We detect
        // this by checking for alternative_count_type — when set, the group_names is
        // used as a condition (e.g. "if X group is present, pick any number"), not
        // as a filter on the options themselves.  Fixes Kanon pb2-001-R's followup
        // choice where group_names=["Liella!"] was lost after option selection.
        let should_propagate_group = effect.group_names_any().is_some()
            && effect.alternative_count_type_any().is_none()
            && effect.compound.alternative_condition.is_none();
        let propagated_options: Option<Vec<Box<AbilityEffect>>> = if should_propagate_group {
            options.map(|opts| {
                opts.iter()
                    .map(|opt| {
                        let mut p = opt.clone();
                        if p.group_names_any().is_none() {
                            p.set_group_names(effect.group_names_any().cloned().map(Box::new));
                        }
                        p
                    })
                    .collect()
            })
        } else {
            None
        };
        let conditional_choice_val = if let Some(ref opts) = propagated_options {
            Some(ConditionalChoice::Effects(opts.clone()))
        } else if let Some(opts) = options {
            Some(ConditionalChoice::Effects(opts.iter().cloned().collect()))
        } else if let Some(opts) = choice_options {
            Some(ConditionalChoice::Strings(opts.to_vec()))
        } else {
            None
        };
        if let Some(entry) = gs.ability_queue.current_entry_mut() {
            entry.choice_card_no = if options.is_some() {
                Some(ChoiceRoute::Choice)
            } else if choice_options.is_some() {
                Some(ChoiceRoute::ChoiceString)
            } else {
                Some(ChoiceRoute::Choice)
            };
            entry.conditional_choice = conditional_choice_val;

            // Set choice_player_id based on choice_maker
            if choice_maker == Some("opponent") {
                let current_player_id = entry.player_id.clone();
                let opponent_id = if current_player_id == "p1" {
                    "p2".to_string()
                } else {
                    "p1".to_string()
                };
                entry.choice_player_id = Some(opponent_id);
            } else {
                entry.choice_player_id = Some(entry.player_id.clone());
            }
        }
        if let Some(effect_options) = options {
            let description = effect_options
                .iter()
                .map(|o| {
                    o.answers_any()
                        .as_ref()
                        .map(|a| a.join(", "))
                        .unwrap_or_else(|| o.text.to_string())
                })
                .collect::<Vec<_>>()
                .join(" / ");
            self.pending_choice = Some(Choice::SelectTarget {
                target: "choice".to_string(),
                description: description.clone(),
                description_en: Some(description.clone()),
                description_ja: Some(format!("選択: {}", description)),
                allow_skip: effect.optional.unwrap_or(false),
                options: None,
            });
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.choice_effect_text = Some(effect.text.to_string());
            }
        } else if let Some(string_options) = choice_options {
            self.pending_choice = Some(Choice::SelectTarget {
                target: "choice_string".to_string(),
                description: format!("Choose one: {}", string_options.join(", ")),
                description_en: Some(format!("Choose one: {}", string_options.join(", "))),
                description_ja: Some(format!("選択: {}", string_options.join(", "))),
                allow_skip: effect.optional.unwrap_or(false),
                options: None,
            });
        } else if let Some(ct) = choice_type {
            self.pending_choice = Some(Choice::SelectTarget {
                target: "choice".to_string(),
                description: format!("Choose: {}", ct),
                description_en: Some(format!("Choose: {}", ct)),
                description_ja: Some(format!("選択: {}", ct)),
                allow_skip: effect.optional.unwrap_or(false),
                options: None,
            });
        }
        Ok(())
    }

    pub(crate) fn execute_pay_energy(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let count: u8 = if let Some(ref dc) = effect.dynamic_count_any() {
            self.resolve_dynamic_count(gs, dc)
        } else {
            effect
                .energy_count_any()
                .unwrap_or_else(|| effect.count_or(0)) as u8
        };
        if effect.optional.unwrap_or(false) {
            let player = gs.resolve_target_player(effect.target_name());
            if player.energy_zone.active_count() < count {
                // Insufficient energy: skip payment and clear remaining actions
                self.cancel_remaining_commands = true;
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.pending_actions.clear();
                }
                return Ok(());
            }
            self.pending_energy_payment = Some(count);
            self.pending_choice = Some(Choice::SelectTarget {
                target: "pay_optional_cost:skip_optional_cost".to_string(),
                description: format!("Pay {} energy?", count),
                description_en: Some(format!("Pay {} energy?", count)),
                description_ja: Some(format!("{}エネルギー支払う？", count)),
                allow_skip: false,
                options: None,
            });
            return Ok(());
        }
        if count > 0 {
            let player = gs.resolve_target_player_mut(effect.target_name());
            player.energy_zone.pay_energy(count)?;
        }
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: {}エネルギー支払", pp, act_name, count));
        Ok(())
    }

    pub(crate) fn execute_discard_until_count(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let target_count: u8 = effect.target_count_any().unwrap_or(0) as u8;
        let target = effect.target_name();
        let player = gs.resolve_target_player_mut(target);
        let current_count = player.hand.cards.len();
        if current_count <= target_count as usize {
            return Ok(());
        }
        let cards_to_discard = current_count - target_count as usize;
        let desc_en = format!(
            "Discard {} cards from hand (target: {} cards in hand)",
            cards_to_discard, target_count
        );
        let desc_ja = format!(
            "手札から{}枚捨てる（目標: 手札{}枚）",
            cards_to_discard, target_count
        );
        self.pending_choice = Some(
            Choice::select_cards(Zone::Hand.to_str(), cards_to_discard, desc_en, false)
                .description_ja(Some(desc_ja))
                .target_player_id(Some(target.to_string()))
                .build(),
        );
        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log(format!(
            "{} {}: [[log_discard_until:n={}]]",
            pp, act_name, target_count
        ));
        Ok(())
    }

    pub(crate) fn execute_restriction(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let restriction_type_binding = effect.restriction_type_any();
        let restriction_type = restriction_type_binding.as_deref();
        let restricted_dest_binding = effect.restricted_destination_any();
        let restricted_destination = restricted_dest_binding
            .as_deref()
            .or(effect.destination.as_deref());
        let target = effect.target_name();
        let delayed = effect.delayed_any().unwrap_or(false);
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: [[log_restriction]]", pp, act_name));
        let restriction_str = format!(
            "restriction:{}:{}",
            restriction_type.unwrap_or("unknown"),
            restricted_destination.unwrap_or("")
        );
        if delayed {
            gs.delayed_prohibition_effects.push(restriction_str);
        } else {
            gs.prohibition_effects.push(restriction_str);
        }
        // Handle cannot_activate restrictions — store for checking during Active phase
        if restriction_type == Some("cannot_activate_by_effect")
            || restriction_type == Some("cannot_active")
        {
            if delayed {
                // Per-card "next turn only" cannot_active flag.
                // Only blocks the activating card, not the whole player.
                if let Some(card_id) = gs.activating_card {
                    gs.mods.add_delayed_cannot_active(card_id, 1);
                }
            } else {
                let resolved = gs.resolve_target_player(target).id.clone();
                if !gs.cannot_activate_members.contains(&resolved) {
                    gs.cannot_activate_members.push(resolved);
                }
            }
        }
        // Handle cannot_live restrictions — store per-player
        if restriction_type == Some("cannot_live") {
            let resolved = gs.resolve_target_player(target).id.clone();
            if !gs.cannot_live_players.contains(&resolved) {
                gs.cannot_live_players.push(resolved);
            }
        }
        Ok(())
    }

    pub(crate) fn execute_re_yell(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let lose_blade_hearts = effect.lose_blade_hearts_any().unwrap_or(false);
        let target = effect.target_name();
        log::debug!("re_yell: lose_blade_hearts={}", lose_blade_hearts);
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

    pub(crate) fn execute_activation_restriction(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) {
        let target = effect.target_name();
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log(format!(
            "{} {}: [[log_activation_restriction:target={}]]",
            pp, act_name, target
        ));
        gs.prohibition_effects
            .push(format!("activation_restriction:{}", target));
    }

    pub(crate) fn execute_choose_required_hearts(&mut self, gs: &mut GameState) {
        self.pending_choice = Some(Choice::SelectTarget {
            target: "choose_required_hearts".to_string(),
            description: "Choose required hearts".to_string(),
            description_en: Some("Choose required hearts".to_string()),
            description_ja: Some("必要なハートを選択".to_string()),
            allow_skip: false,
            options: None,
        });
        let pp = self.player_prefix(gs);
        gs.push_rule_log(format!("{}: [[log_heart_select]]", pp));
    }

    pub(crate) fn execute_choose_target_player(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        self.current_effect = Some(effect.clone());
        let options = effect
            .choice_options_any()
            .cloned()
            .unwrap_or_else(|| vec!["自分".to_string(), "相手".to_string()]);
        self.pending_choice = Some(Choice::SelectTarget {
            target: "self_or_opponent".to_string(),
            description: "Choose self or opponent".to_string(),
            description_en: Some("Choose self or opponent".to_string()),
            description_ja: Some("自分または相手を選択".to_string()),
            allow_skip: false,
            options: Some(options.to_vec()),
        });
        let pp = self.player_prefix(gs);
        gs.rule_log
            .push(format!("{}: ターゲットプレイヤー選択", pp));
        Ok(())
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
                log::debug!("Unknown shuffle zone: {}", source);
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

    pub(crate) fn player_prefix(&self, gs: &GameState) -> String {
        if let Some(card_id) = gs.activating_card {
            if gs.player1.stage.stage.contains(&card_id)
                || gs.player1.live_card_zone.cards.contains(&card_id)
                || gs.player1.hand.cards.contains(&card_id)
            {
                return "P1".to_string();
            }
            if gs.player2.stage.stage.contains(&card_id)
                || gs.player2.live_card_zone.cards.contains(&card_id)
                || gs.player2.hand.cards.contains(&card_id)
            {
                return "P2".to_string();
            }
        }
        if gs.player1.id == gs.active_player().id {
            "P1"
        } else {
            "P2"
        }
        .to_string()
    }

    pub(crate) fn card_name<'a>(&self, card_id: i16) -> String {
        self.card_db()
            .get_card(card_id)
            .map(|c| c.name.to_string())
            .unwrap_or_else(|| format!("Card#{}", card_id))
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
            for cid in total_blade {
                gs.push_revealed_card(cid, reyell_source, false, reyell_owner, "re_yell");
            }
        }
        gs.re_yell_revealed_cards = gs.revealed_cards.clone();
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
}
