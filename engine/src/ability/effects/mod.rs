pub mod ability_effects;
pub mod draw;
pub mod misc;
pub mod score;
pub mod state;

pub(crate) use draw::draw_cards_for_player;

use super::debug::AbDebug;
use super::enums::{ActionType, Zone};
use super::resolver::AbilityResolver;
use super::types::Choice;
use super::util;
use crate::card::AbilityEffect;
use crate::game_state::GameState;

impl AbilityResolver {
    // Q55: Effects resolve as much as possible; partial resolution required when full is impossible.
    pub fn execute_effect(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let mut dbg = AbDebug::new();
        eprintln!(
            "[DEBUG_EXEC] execute_effect: action={} effect_ptr={:p}",
            effect.action, effect
        );
        dbg.effect(effect);
        log::debug!(
            "DEBUG: execute_effect - action: {}, source: {}, destination: {}",
            effect.action,
            effect.source_or("none"),
            effect.destination.as_deref().unwrap_or("none")
        );
        let exec_snapshot = crate::ability::log::buffer_len();
        if !self.can_activate_effect(gs, effect) {
            log::debug!("DEBUG: cannot activate effect");
            // Keep verdicts — condition failure info will be captured by push_ability_result
            return Ok(());
        }
        // Drain condition verdicts from the can_activate_effect pre-check;
        // the effect execution will produce its own items and we don't want duplicates.
        {
            let _pre = crate::ability::log::drain_verdicts_since(exec_snapshot);
        }

        // non_stackable check: skip if this effect is already active
        if effect.non_stackable.unwrap_or(false) {
            let effect_key = format!("{}:{}", effect.action, effect.text);
            if gs.non_stackable_effects.contains(&effect_key) {
                log::debug!(
                    "DEBUG: non-stackable effect already active, skipping: {}",
                    effect_key
                );
                return Ok(());
            }
            gs.non_stackable_effects.insert(effect_key);
        }

        // Log effect execution to rule_log
        // Log effect execution to rule_log (skip compound/dispatch-only types
        // whose sub-actions log individually, to avoid duplicates).
        if !effect.text.is_empty()
            && !matches!(
                effect.action.as_str(),
                "compound_action"
                    | "sequential"
                    | "choice"
                    | "conditional_alternative"
                    | "conditional_on_result"
                    | "conditional_on_optional"
            )
        {
            // Effect details are captured in the structured ability_resolution entry
            // — no separate [effect] text entry needed.
        }

        // Legacy opponent_action wrapper (pre-parser-flatten). Flat effects
        // carry target="opponent" directly and dispatch via ActionType.
        if effect.action == "opponent_action" {
            if let Some(ref opponent_action) = effect.opponent_action() {
                // G3: tag spawn context so choices created for this
                // opponent action are routed to the opponent player.
                self.spawn_context.target = Some("opponent".to_string());
                let mut modified = (*opponent_action).clone();
                if modified.target.is_none() || modified.target.as_deref() == Some("self") {
                    modified.target = Some("opponent".into());
                }
                self.execute_effect(gs, &modified)?;
                return Ok(());
            }
        }

        gs.reset_replacement_effect_flags();
        let action_str = effect.action.as_str();

        // Empty action with opponent_action means it was entirely handled by opponent
        if action_str.is_empty() && effect.action_by().is_some() {
            return Ok(());
        }

        // G3: for non-empty actions with action_by: opponent, tag spawn context
        // so choices created inside are routed to the opponent player.
        if effect.action_by().as_deref() == Some("opponent") {
            self.spawn_context.target = Some("opponent".to_string());
        }

        let replacement_indices: Vec<usize> = gs
            .replacement_effects
            .iter()
            .enumerate()
            .filter(|(_, r)| r.original_event == action_str && !r.applied_this_event)
            .map(|(i, _)| i)
            .collect();

        if !replacement_indices.is_empty() {
            for idx in replacement_indices {
                if gs.replacement_effects[idx].is_choice_based {
                    let description =
                        format!("Apply replacement effect for action '{}'?", action_str);
                    self.pending_choice = Some(Choice::SelectTarget {
                        target: "apply_replacement".to_string(),
                        description: description.clone(),
                        description_en: Some(description.clone()),
                        description_ja: Some(format!(
                            "アクション「{}」の置き換え効果を適用？",
                            action_str
                        )),
                        allow_skip: false,
                        options: None,
                    });
                    return Err("Pending choice required: apply replacement effect".to_string());
                } else {
                    let effects_to_execute =
                        gs.replacement_effects[idx].replacement_effects.clone();
                    let card_id = gs.replacement_effects[idx].card_id;
                    for replacement_effect in &effects_to_execute {
                        self.execute_effect(gs, replacement_effect)?;
                    }
                    gs.mark_replacement_effect_applied(card_id);
                }
            }
            return Ok(());
        }

        if let Some(ref effect_type) = effect.effect_type() {
            if *effect_type == "replacement" {
                let original_event = effect.replaces_event_any().clone();
                let is_choice_based = effect.choice_based_any().unwrap_or(false);
                let card_id = gs.activating_card.unwrap_or(-1);
                let player_id =
                    if gs.current_turn_phase == crate::game_state::TurnPhase::FirstAttackerNormal {
                        gs.player1.id.clone()
                    } else {
                        gs.player2.id.clone()
                    };
                if let Some(event) = original_event {
                    gs.add_replacement_effect(
                        card_id,
                        player_id,
                        event.to_string(),
                        vec![effect.clone()],
                        is_choice_based,
                    );
                }
                return Ok(());
            }
        }

        // Rule 9.8 / Q158: Handle target="both" generically
        //   execute once for self, then opponent.
        //   position_change handles "both" internally (opponent first, then self).
        //
        // Q158: "Does blade+2 from the effect apply to all members on stage?"
        //   → Yes. "自分のステージにいるメンバー" means all members.
        if self.handle_both_targets(gs, effect)? {
            return Ok(());
        }

        // Rule 9.2.1: Effect dispatch
        //
        // Convert string action to typed enum for stronger dispatch.
        // Each ActionType variant maps to a dedicated handler:
        //
        //   Sequential           → execute_sequential_effect (compound.rs)
        //   ConditionalAlternative → execute_conditional_alternative (compound.rs)
        //   LookAndSelect        → execute_look_and_select (look.rs)
        //   SelectCards          → execute_select_cards (look.rs)
        //   Draw/DrawCard        → execute_draw_wrapper
        //   MoveCards/DiscardCard→ execute_move_cards (move_cards.rs)
        //   PayEnergy            → execute_pay_energy (effects/misc.rs)
        //   ...and many more
        //
        // Compound routing: Sequential and LookAndSelect both route through
        // the generic sequential pipeline when they carry `effect_steps`.
        // LookAndSelect is collapsed by the parser into:
        //   [look_step, select_cards_step, move_selected_step]
        // The sequential pipeline executes them in order, creating a pending
        // choice on the select_cards step and resuming naturally when the
        // player responds. Legacy dedicated handlers remain as fallback
        // for the case where effect_steps is absent.
        let action_type = ActionType::from_str(&action_str).unwrap_or(ActionType::Custom);
        if crate::ability::debug::ABILITY_DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
            eprintln!(
                "[EXEC_ACTION] action_type={:?} has_steps={} has_actions={}",
                action_type,
                effect.effect_steps.is_some(),
                effect.compound.actions.is_some()
            );
        }

        // Rule 9.8.1 / Q85 / Q86: Sequential/LookAndSelect routing
        //
        // Q85: LookAndSelect with insufficient deck → refresh mid-look
        //   (handled inside execute_look_at, which implements the 4-step
        //   procedure: look available → refresh → look remaining → resolve)
        //
        // Q86: LookAndSelect with exactly enough cards → no refresh during
        //   look. After resolution, if deck is 0, refresh on next check timing.
        if action_type == ActionType::Sequential || action_type == ActionType::LookAndSelect {
            let steps = effect.normalized_steps();
            if !steps.is_empty() {
                if action_type == ActionType::Sequential {
                    eprintln!(
                        "[DEBUG_STEPS] sequential: n_steps={} actions=[{}] effect.action={}",
                        steps.len(),
                        steps
                            .iter()
                            .map(|s| s.action.as_str())
                            .collect::<Vec<_>>()
                            .join(","),
                        effect.action
                    );
                }
                let mut normalized = effect.clone();
                normalized.effect_steps = None;
                normalized.compound.actions = Some(steps);
                normalized.action = "sequential".to_string();
                return self.execute_sequential_effect(
                    gs,
                    &normalized,
                    normalized.conditional.unwrap_or(false),
                    normalized.is_further.unwrap_or(false),
                );
            }
        }

        let result = match action_type {
            ActionType::Sequential => self.execute_sequential_effect(
                gs,
                effect,
                effect.conditional.unwrap_or(false),
                effect.is_further.unwrap_or(false),
            ),
            ActionType::ConditionalAlternative => self.execute_conditional_alternative(gs, effect),
            ActionType::LookAndSelect => self.execute_look_and_select(gs, effect),
            ActionType::SelectCards => self.execute_select_cards(gs, effect),
            ActionType::Draw | ActionType::DrawCard => self.execute_draw_wrapper(gs, effect),
            ActionType::DrawUntilCount => {
                self.execute_draw_until_count(
                    gs,
                    effect.target_count_any().unwrap_or(0),
                    effect.target_name(),
                    effect.destination.as_deref().unwrap_or(Zone::Hand.to_str()),
                );
                Ok(())
            }
            ActionType::DiscardCard => {
                let pp = self.player_prefix(gs);
                let cn = gs
                    .activating_card
                    .and_then(|id| gs.card_database.get_card(id))
                    .map(|c| c.name.to_string())
                    .unwrap_or_default();
                gs.rule_log.push(format!("{} {}: [[log_discard]]", pp, cn));
                self.execute_move_cards(gs, effect)
            }
            ActionType::MoveCards => {
                let pp = self.player_prefix(gs);
                let cn = gs
                    .activating_card
                    .and_then(|id| gs.card_database.get_card(id))
                    .map(|c| c.name.to_string())
                    .unwrap_or_default();
                gs.rule_log.push(format!("{} {}: [[log_move]]", pp, cn));
                if effect.multiple_targets_any().unwrap_or(false)
                    && effect.target.as_deref() == Some("deck")
                {
                    self.execute_move_cards_both(gs, effect)
                } else {
                    self.execute_move_cards(gs, effect)
                }
            }
            ActionType::GainResource => self.execute_gain_resource(gs, effect),
            ActionType::ChangeState => {
                let change_cost_limit = if effect.cost_from_revealed_any().unwrap_or(false) {
                    gs.revealed_cards
                        .first()
                        .and_then(|&cid| gs.card_database.get_card(cid))
                        .and_then(|c| c.cost)
                } else {
                    effect.cost_limit_any()
                };
                let mut change_count = effect.count_or(0);
                let mut change_group = effect.group_name();
                if effect.per_unit_any().unwrap_or(false) {
                    let player = gs.resolve_target_player(effect.target_name());
                    let loc_binding = effect.location_any();
                    let location = loc_binding.unwrap_or(Zone::Stage.to_str());
                    let cards: Vec<i16> = util::zone_cards(player, location).to_vec();
                    let mut per_unit_filter = util::CardFilter::from_effect(effect);
                    per_unit_filter.card_type = None;
                    per_unit_filter.cost_limit = change_cost_limit;
                    let matching: Vec<i16> = cards
                        .iter()
                        .filter(|&&cid| per_unit_filter.matches(&gs.card_database, cid, false))
                        .copied()
                        .collect();
                    let count = util::apply_distinct_filter(
                        &matching,
                        effect.distinct_any(),
                        &gs.card_database,
                    )
                    .len() as u32;
                    let per_unit_cnt = effect.per_unit_count_any().unwrap_or(1);
                    change_count = (count / per_unit_cnt) * change_count.max(1);
                    change_group = None;
                }
                self.execute_change_state(
                    gs,
                    effect,
                    effect.state_change_any().as_deref().unwrap_or(""),
                    effect.target_name(),
                    change_count,
                    effect.max.unwrap_or(false),
                    effect.card_type_any().as_deref(),
                    change_cost_limit,
                    effect.optional.unwrap_or(false),
                    change_group,
                    effect.self_cost_any().unwrap_or(false),
                    effect.source_any(),
                    effect.destination.as_deref(),
                    effect.cost_limit_operator_any().map(|s| s.to_string()),
                    effect.characters_any(),
                    effect.blade_limit_any(),
                    effect.blade_limit_operator_any().as_deref(),
                )
            }
            ActionType::ModifyScore => self.execute_modify_score(
                gs,
                effect,
                effect.operation_any().as_deref().unwrap_or("add"),
                effect.value_any().unwrap_or(0),
                effect.target_name(),
                effect.duration_any().as_deref(),
                effect.card_type_any().as_deref(),
                effect.group_name(),
                effect.per_unit_any().unwrap_or(false),
                effect.per_unit_count_any().unwrap_or(1),
                effect.per_unit_type_any().as_deref(),
                effect.location_any().as_deref(),
                effect.effect_constraint_any().as_deref(),
                effect.self_target_any().unwrap_or(false),
                effect.heart_colors_any(),
            ),
            ActionType::ModifyRequiredHearts => self.execute_modify_required_hearts(
                gs,
                effect.operation_any().as_deref().unwrap_or("decrease"),
                effect.value_or_count(0),
                effect.heart_colors_any(),
                effect.target_name(),
                effect.per_unit_any().unwrap_or(false),
                effect.per_unit_count_any().unwrap_or(1),
                effect.group_name(),
                effect.timing_condition_any().as_deref(),
                effect.location_any().as_deref(),
                effect.original_value_any(),
                effect.original_count_any(),
                effect.original_operator_any().as_deref(),
                effect.exclude_self_any().unwrap_or(false),
                effect.self_target_any().unwrap_or(false),
                effect.exclude_heart_colors_any(),
                effect.max.unwrap_or(false),
                effect.repeat_limit_any(),
                &effect.per_unit_heart_colors_any(),
            ),
            ActionType::SetCost => {
                self.execute_set_cost(gs, effect, effect.value_any().unwrap_or(0));
                Ok(())
            }
            ActionType::SetBladeType => {
                self.execute_set_blade_type(gs, effect);
                Ok(())
            }
            ActionType::SetHeartType => {
                let is_self_target = effect.self_target_any().unwrap_or(false);
                let needs_target = !is_self_target
                    && (effect.heart_selection_any().unwrap_or(false)
                        || effect.group_names_any().is_some()
                        || effect.card_type_any().as_deref() == Some("member_card"));
                let ht_binding = effect.heart_type_any();
                let heart_type =
                    ht_binding.or(effect.heart_colors_any().first().map(|s| s.as_str()));

                if is_self_target || !needs_target {
                    // Self-target (e.g. Kanan PL!S-pb1-003-R): apply to activating_card.
                    // Also fallback for member-card abilities without group/selection signals.
                    self.execute_set_heart_type(
                        gs,
                        heart_type,
                        effect.target_name(),
                        effect.count_or(1) as i32,
                        effect.duration_any().as_deref(),
                    );
                } else if self.selected_cards.is_empty() {
                    // Need target selection: find eligible stage members
                    let target = effect.target_name();
                    let stage_ids: Vec<i16> = {
                        let p = gs.resolve_target_player(target);
                        p.stage
                            .stage
                            .iter()
                            .copied()
                            .filter(|&id| id != -1)
                            .collect()
                    };
                    let card_db = self.card_db();
                    let filter = effect.filter_subset();
                    let candidates = util::matching_ids_filtered(
                        &stage_ids, &card_db, &filter, true, None, None, None,
                    );
                    if candidates.is_empty() {
                        // No eligible targets — no-op
                        return Ok(());
                    }
                    let tc = effect.target_count_any().unwrap_or(1) as usize;
                    if candidates.len() <= tc {
                        // Auto-select: push to selected_cards and apply
                        for &cid in &candidates {
                            if !self.selected_cards.contains(&cid) {
                                self.selected_cards.push(cid);
                            }
                        }
                        self.execute_set_heart_type(
                            gs,
                            heart_type,
                            effect.target_name(),
                            effect.count_or(1) as i32,
                            effect.duration_any().as_deref(),
                        );
                    } else {
                        // Multiple eligible: create SelectCard choice
                        let stage_snapshot: Vec<i16> = {
                            let p = gs.resolve_target_player(target);
                            p.stage.stage.to_vec()
                        };
                        let filtered_indices: Vec<usize> = candidates
                            .iter()
                            .filter_map(|&cid| stage_snapshot.iter().position(|&s| s == cid))
                            .collect();
                        let mut saved = effect.clone();
                        saved.set_target_count(None);
                        let mut pending = gs.ability_queue.take_pending_actions();
                        pending.insert(0, saved);
                        gs.ability_queue.set_pending_actions(pending);
                        let desc_en = format!("Select {} member(s) for heart type conversion", tc);
                        let desc_ja = format!("ハート種類変換のメンバーを{}体選択", tc);
                        self.pending_choice = Some(
                            Choice::select_cards(
                                Zone::Stage.to_str().to_string(),
                                tc,
                                desc_en,
                                false,
                            )
                            .description_ja(Some(desc_ja))
                            .card_type(effect.card_type_any().map(|s| s.to_string()))
                            .group(effect.group_name().map(|s| s.to_string()))
                            .characters(effect.characters_any().cloned())
                            .filtered_indices(Some(filtered_indices))
                            .target_player_id(Some(target.to_string()))
                            .is_select_action(true)
                            .build(),
                        );
                        self.sub_choice_created = true;
                        return Ok(());
                    }
                } else {
                    // Already have selected target from previous choice resolution
                    self.execute_set_heart_type(
                        gs,
                        heart_type,
                        effect.target_name(),
                        effect.count_or(1) as i32,
                        effect.duration_any().as_deref(),
                    );
                }
                Ok(())
            }
            ActionType::ActivateAbility => {
                self.execute_activate_ability(
                    gs,
                    effect.ability_text_any().as_deref().unwrap_or(""),
                    effect.target_trigger_any().as_deref(),
                    effect.count,
                    effect.source_card_any().as_deref(),
                );
                Ok(())
            }
            ActionType::InvalidateAbility => self.execute_invalidate_ability(gs, effect),
            ActionType::SuppressAbilityTrigger => self.execute_suppress_ability_trigger(gs, effect),
            ActionType::GainAbility => self.execute_gain_ability_effect(gs, effect),
            ActionType::GainAbilityFromSource => self.execute_gain_ability_from_source(gs, effect),
            ActionType::PlayBatonTouch => {
                self.execute_play_baton_touch(gs, effect.count_or(1), effect.target_name())
            }
            ActionType::Reveal | ActionType::RevealEffect => self.execute_reveal_effect(gs, effect),
            ActionType::Select => {
                let pp = self.player_prefix(gs);
                let cn = gs
                    .activating_card
                    .and_then(|id| gs.card_database.get_card(id))
                    .map(|c| c.name.to_string())
                    .unwrap_or_default();
                gs.rule_log.push(format!("{} {}: [[log_select]]", pp, cn));
                self.execute_select_effect(gs, effect)
            }
            ActionType::SelectNumber => self.execute_select_number(gs, effect),
            ActionType::Look | ActionType::LookAt => {
                let base_count = if let Some(ref dc) = effect.dynamic_count_any() {
                    self.resolve_dynamic_count(gs, dc)
                } else {
                    effect.count_or(1)
                };
                let final_count = if effect.per_unit_any().unwrap_or(false) {
                    use crate::ability::util;
                    use crate::HashMap;
                    let player = gs.resolve_target_player(effect.target_name());
                    let filter = util::CardFilter::from_effect(effect);
                    let per_mult = util::resolve_per_unit_count(
                        true,
                        effect.per_unit_type_any().as_deref(),
                        player,
                        &gs.card_database,
                        &filter,
                        effect.heart_colors_any(),
                        None,
                        &HashMap::new(),
                    );
                    base_count * per_mult
                } else {
                    base_count
                };
                self.execute_look_at(
                    gs,
                    effect,
                    final_count,
                    effect.target_name(),
                    effect.source_or(Zone::Deck.to_str()),
                )
            }
            ActionType::ModifyRequiredHeartsGlobal => self.execute_modify_required_hearts_standard(
                gs,
                effect.operation_any().as_deref().unwrap_or("increase"),
                effect.value_or_count(1),
                effect.heart_colors_any(),
                effect.target_name(),
            ),
            ActionType::ModifyYellCount => {
                self.execute_modify_yell_count(
                    gs,
                    effect.operation_any().as_deref().unwrap_or("subtract"),
                    effect.count_or(0),
                );
                Ok(())
            }
            ActionType::PlaceEnergyUnderMember => {
                // Resolve dynamic_count if present
                let actual_count = if let Some(ref dc) = effect.dynamic_count_any() {
                    self.resolve_dynamic_count(gs, dc)
                } else {
                    effect.energy_count_any().unwrap_or(1)
                };
                // Special case: source="under_member" + destination="energy_zone" means
                // count from under member, but move from energy_deck → energy_zone (wait).
                // e.g. PL!N-bp5-012-R+ LiveSuccess: place (under_count + 1) from deck.
                if effect.source_any() == Some("under_member")
                    && effect.destination.as_deref() == Some("energy_zone")
                {
                    let player = gs.resolve_target_player_mut(effect.target_name());
                    for _ in 0..actual_count {
                        if let Some(energy) = player.energy_deck.draw() {
                            player.energy_zone.cards.push(energy);
                            // Don't increment active_energy_count — wait state
                        } else {
                            break;
                        }
                    }
                } else if effect.source_any() == Some("under_member")
                    && effect.destination.as_deref() == Some("empty_area")
                {
                    // Deploy from under_member to empty_area (e.g. PL!-bp6-003-R+ LiveSuccess)
                    // Only offer choice if there's an empty stage slot.
                    let player = gs.resolve_target_player(effect.target_name());
                    let has_empty_slot = (0..3).any(|i| player.stage.stage[i] == -1);
                    if !has_empty_slot {
                        return Ok(());
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
                        return Ok(());
                    }
                    let target_str = effect.target_name().to_string();
                    let desc_ja = "このメンバーの下から出すメンバーカードを選択".to_string();
                    let mut b = Choice::select_cards(
                        Zone::UnderMember.to_str(),
                        actual_count as usize,
                        "Select a member card to deploy from under this member",
                        effect.optional.unwrap_or(false),
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
                        effect.cost_limit_any(),
                        effect.cost_limit_operator_any().map(|s| s.to_string()),
                    );
                    self.pending_choice = Some(b.build());
                    self.execution_context =
                        super::types::ExecutionContext::SingleEffect { effect_index: 0 };
                } else if effect.source_any() == Some("under_member")
                    && effect.destination.as_deref() == Some("energy_deck")
                {
                    // Awakening Promise case: move from under_member → energy_deck
                    self.execute_place_energy_under_member(
                        gs,
                        actual_count,
                        effect.target_name(),
                        effect.position_any(),
                        effect.optional.unwrap_or(false),
                        effect.source_any(),
                        effect.any_number_any().unwrap_or(false),
                    );
                } else {
                    self.execute_place_energy_under_member(
                        gs,
                        actual_count,
                        effect.target_name(),
                        effect.position_any(),
                        effect.optional.unwrap_or(false),
                        effect.source_any(),
                        effect.any_number_any().unwrap_or(false),
                    );
                }
                Ok(())
            }
            ActionType::ActivationCost => {
                self.execute_activation_cost(
                    gs,
                    effect.operation_any().as_deref().unwrap_or("increase"),
                    effect.value_any().unwrap_or(0),
                    effect.target_name(),
                    effect.duration_any().as_deref(),
                );
                Ok(())
            }
            ActionType::PositionChange => self.execute_position_change(
                gs,
                effect,
                effect.position_any().cloned(),
                effect.target_name(),
                effect
                    .target_member_any()
                    .as_deref()
                    .unwrap_or("this_member"),
            ),
            ActionType::Rotation => self.execute_rotation(gs, effect, effect.target_name()),

            ActionType::Choice => self.execute_choice(gs, effect),
            ActionType::PayEnergy => {
                let count = if let Some(ref dc) = effect.dynamic_count_any() {
                    self.resolve_dynamic_count(gs, dc)
                } else {
                    effect
                        .energy_count_any()
                        .unwrap_or_else(|| effect.count_or(0))
                };
                if effect.optional.unwrap_or(false) {
                    let player = gs.resolve_target_player(effect.target_name());
                    if player.energy_zone.active_count() < count as usize {
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
                self.execute_pay_energy(gs, count, effect.target_name())
            }
            ActionType::SetCardIdentity => self.execute_set_card_identity_effect(gs, effect),
            ActionType::RepeatProcedure => {
                self.execute_repeat_procedure(gs, effect, effect.repeat_limit_any().unwrap_or(1))
            }
            ActionType::DiscardUntilCount => self.execute_discard_until_count(
                gs,
                effect.target_count_any().unwrap_or(0),
                effect.target_name(),
            ),
            // Q57: A "cannot do X" effect takes priority over an effect that would do X.
            ActionType::Restriction => self.execute_restriction(
                gs,
                effect.restriction_type_any().as_deref(),
                effect
                    .restricted_destination_any()
                    .as_deref()
                    .or(effect.destination.as_deref()),
                effect.target_name(),
                effect.delayed_any().unwrap_or(false),
            ),
            ActionType::ReYell => {
                self.execute_re_yell(
                    gs,
                    effect.lose_blade_hearts_any().unwrap_or(false),
                    effect.target_name(),
                );
                Ok(())
            }
            ActionType::ActivationRestriction => {
                self.execute_activation_restriction(gs, effect.target_name());
                Ok(())
            }
            ActionType::ChooseRequiredHearts => {
                self.execute_choose_required_hearts(gs);
                Ok(())
            }
            ActionType::ModifyLimit => self.execute_modify_limit(
                gs,
                effect.operation_any().as_deref().unwrap_or("decrease"),
                effect.count_or(0),
            ),
            ActionType::ReduceLiveCardSetLimit => {
                self.execute_reduce_live_card_set_limit(gs, effect.count_or(1));
                Ok(())
            }
            ActionType::SetBladeCount => {
                self.execute_set_blade_count(
                    gs,
                    effect,
                    effect.value_any().unwrap_or(effect.count_or(0)),
                );
                Ok(())
            }
            ActionType::Custom => self.execute_custom(gs, effect, &action_str),
            ActionType::DoNothing => Ok(()),

            ActionType::SpecifyHeartColor => {
                self.execute_specify_heart_color(
                    gs,
                    effect.choice_any().unwrap_or(false),
                    effect.target_name(),
                );
                Ok(())
            }
            ActionType::ModifyRequiredHeartsSuccess => {
                self.execute_modify_required_hearts_success(
                    gs,
                    effect.operation_any().as_deref().unwrap_or("increase"),
                    effect.value_any().unwrap_or(0),
                    effect.target_name(),
                    effect.card_type_any().as_deref(),
                    effect.heart_colors_any(),
                );
                Ok(())
            }
            ActionType::SetCostToUse => {
                self.execute_set_cost_to_use(gs, effect.value_any().unwrap_or(0))
            }
            ActionType::AllBladeTiming => {
                self.execute_all_blade_timing(
                    gs,
                    effect
                        .timing_any()
                        .as_deref()
                        .unwrap_or("check_required_hearts"),
                    effect
                        .treat_as_any()
                        .as_deref()
                        .unwrap_or("any_heart_color"),
                );
                Ok(())
            }
            ActionType::SetCardIdentityAllRegions => {
                self.execute_set_card_identity_all_regions(
                    gs,
                    effect.identities_any(),
                    effect.target_name(),
                );
                Ok(())
            }
            ActionType::Shuffle => {
                self.execute_shuffle(
                    gs,
                    effect.target_name(),
                    effect.source_or(Zone::Deck.to_str()),
                );
                Ok(())
            }
            ActionType::RevealPerGroup => self.execute_reveal_per_group(
                gs,
                effect.source_or(Zone::Hand.to_str()),
                effect.count_or(1),
                effect.target_name(),
            ),
            ActionType::ConditionalOnResult => self.execute_conditional_on_result(gs, effect),
            ActionType::ConditionalOnOptional => self.execute_conditional_on_optional(gs, effect),
            ActionType::ModifyCost => {
                let card_db = &gs.card_database;
                let mut value = effect.value_any().unwrap_or(0);
                if effect.per_unit_any().unwrap_or(false) {
                    let put_binding = effect.per_unit_type_any();
                    let loc_binding2 = effect.location_any();
                    let per_unit_type_str = put_binding.or(loc_binding2).unwrap_or("枚");
                    let player = gs.resolve_target_player(effect.target_name());
                    // Use resolve_per_unit_count which handles under_member,
                    // discard, waitroom_card and other special zones that
                    // zone_cards() cannot represent as a flat slice.
                    let per_unit_filter = util::CardFilter::from_effect(effect);
                    let matching_count = util::resolve_per_unit_count(
                        true,
                        Some(per_unit_type_str),
                        player,
                        card_db,
                        &per_unit_filter,
                        &[],
                        effect.state_any().as_deref(),
                        &gs.mods.orientation_modifiers,
                    );
                    let per_unit_count = effect.per_unit_count_any().unwrap_or(1);
                    let mut units = matching_count / per_unit_count;
                    // Apply max_repeats cap (aliased as repeat_limit).
                    // The text side-constraint "N枚までしか数えない" is parsed as
                    // max_repeats on the effect.
                    if let Some(cap) = effect.repeat_limit_any() {
                        units = units.min(cap);
                    }
                    value *= units;
                }
                self.execute_modify_cost(gs, effect, value);
                Ok(())
            }
            ActionType::RevealUntilLiveCard => {
                self.execute_reveal_until_live_card(gs, effect.target_name())
            }
            ActionType::RevealUntilChosenCard => self.execute_reveal_until_chosen_card(gs, effect),
            ActionType::ChooseTargetPlayer => self.execute_choose_target_player(gs, effect),
            ActionType::PerformYell => {
                let count = if effect.per_unit_any().unwrap_or(false) {
                    // per_unit with per_unit_source = "previous_moved_cards":
                    // sum costs of cards moved by the preceding action,
                    // divide by per_unit_count, cap at repeat_limit.
                    let total_cost: u32 = self
                        .moved_cards
                        .iter()
                        .filter_map(|&cid| gs.card_database.get_card(cid).and_then(|c| c.cost))
                        .sum();
                    let divisor = effect.per_unit_count_any().unwrap_or(1) as u32;
                    let mut c = total_cost / divisor;
                    if let Some(cap) = effect.repeat_limit_any() {
                        c = c.min(cap);
                    }
                    c
                } else if let Some(ref dc) = effect.dynamic_count_any() {
                    self.resolve_dynamic_count(gs, dc)
                } else {
                    effect.count_or(1)
                };
                self.execute_perform_yell(gs, count, effect.target_name());
                Ok(())
            }
        };
        // Push effect verdict for non-structural action types
        const SKIP: &[&str] = &[
            "compound_action",
            "sequential",
            "choice",
            "conditional_alternative",
            "conditional_on_result",
            "conditional_on_optional",
        ];
        if !SKIP.contains(&effect.action.as_str()) {
            let val = effect
                .count
                .or(effect.value_any())
                .map(|v| v.to_string())
                .unwrap_or_default();
            let details = if !val.is_empty() {
                format!("{} {}", effect.action, val)
            } else {
                effect.action.clone()
            };
            crate::ability::log::push_verdict(crate::ability::log::AbilityLogItem::Effect {
                text: effect.text.clone(),
                action: effect.action.clone(),
                details,
            });
        }
        result
    }
}
