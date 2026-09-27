use crate::ability::enums::{ActionType, ConditionType, Zone};
use crate::ability::types::ChoiceRoute;
use crate::card::CardDatabase;
use crate::game_state::GameState;
use crate::game_state::Phase;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
#[cfg(feature = "3ds")]
extern "C" {
    fn _3ds_tdbg(msg: *const u8);
}

#[cfg(feature = "3ds")]
macro_rules! tdbg {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        let s = format!("{}\0", msg);
        unsafe { _3ds_tdbg(s.as_ptr()); }
    }};
}
#[cfg(not(feature = "3ds"))]
macro_rules! tdbg {
    ($($arg:tt)*) => {};
}

/// Snapshot of everything the completion branches of
/// `resume_queue_with_choice` read from the queue entry (captured BEFORE
/// `complete_current()` removes it) plus the caller-side resume flags.
struct ResumeSnapshot {
    just_completed_key: Option<u32>,
    entry_player_id: Option<String>,
    cost_entry_trigger: Option<crate::game_state::AbilityTrigger>,
    cost_entry_card_id: Option<i16>,
    cost_entry_opt_result: Option<bool>,
    saved_activating_card: Option<i16>,
    saved_activating_ability_index: Option<usize>,
    optional_skipped: bool,
    pending_cleared: bool,
}

/// Which completion path a finished answer takes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ResumeOutcome {
    CompleteSkipped,
    Reprocess,
    FinishPaid,
    /// Neither skipped, reprocess, nor paid: just complete and scan.
    /// (The original if/else-if chain's final `else`.)
    CompleteIdle,
}

impl ResumeSnapshot {
    /// Capture the snapshot and decide the outcome. Centralizes the flag
    /// computation previously smeared across the three branches, with the
    /// single CHOICE_RESUME_BRANCH debug line.
    fn capture(game_state: &mut GameState, had_pending_sequential: bool) -> (Self, ResumeOutcome) {
        let entry = game_state.ability_queue.current_entry();
        let cost_was_paid = entry.is_some_and(|e| e.cost_paid);
        let effect_started = game_state
            .ability_queue
            .current_entry()
            .is_some_and(|e| e.effect_started);
        // Capture key and player_id BEFORE complete_current() removes the entry.
        // Numeric key: (card_id as u8) << 16 | ability_index as u8
        let just_completed_key: Option<u32> =
            game_state.ability_queue.current_entry().and_then(|e| {
                let cid = e.card_id?;
                let idx = e.ability_index;
                Some((u32::try_from(cid).ok()? << 16) | u32::try_from(idx).ok()?)
            });
        let entry_player_id = game_state
            .ability_queue
            .current_entry()
            .map(|e| e.player_id.clone());
        // Capture each_time trigger info before entry is lost
        let cost_entry_trigger = game_state
            .ability_queue
            .current_entry()
            .map(|e| e.trigger_type.clone());
        let cost_entry_card_id = game_state
            .ability_queue
            .current_entry()
            .and_then(|e| e.card_id);
        let cost_entry_opt_result = game_state
            .ability_queue
            .current_entry()
            .and_then(|e| e.optional_cost_result);
        // Save activating_card before clearing — it must be restored when
        // the ability continues processing (needs_reprocess), otherwise
        // gain_resource etc. in nested sequentials lose their target.
        let saved_activating_card = game_state.activating_card;
        let saved_activating_ability_index = game_state.activating_ability_index;
        game_state.activating_card = None;
        game_state.activating_ability_index = None;

        let optional_skipped = game_state.ability_queue.current_entry().is_some_and(|e| {
            e.cost_paid
                && e.optional_cost_result == Some(false)
                && e.choice_card_no == Some(crate::ability::types::ChoiceRoute::OptionalCost)
        });
        // Re-check pending commands — they may have been cleared by the choice
        // handler (e.g. optional draw skip), leaving the sequential stranded.
        // Only fire when effect hasn't started yet (the skip is between optional
        // draw choice and the draw action itself). Normal sequential mid-execution
        // has effect_started=true and must NOT be cancelled.
        let pending_cleared = cost_was_paid
            && !effect_started
            && had_pending_sequential
            && !game_state.ability_queue.has_pending_actions();
        let effect_ready = cost_was_paid && !had_pending_sequential && !effect_started;
        // When an optional choice inside a sequential is resolved with "pay",
        // re-process so remaining pending actions can execute.
        let chose_to_pay = game_state
            .ability_queue
            .current_entry()
            .is_some_and(|e| e.optional_cost_result == Some(true));
        let needs_reprocess = effect_ready
            || (cost_was_paid && !effect_started && had_pending_sequential && chose_to_pay);
        log::debug!("[CHOICE_RESUME_BRANCH] cost_paid={} effect_started={} had_pending={} optional_skipped={} pending_cleared={} effect_ready={} chose_to_pay={} needs_reprocess={}",
            cost_was_paid, effect_started, had_pending_sequential, optional_skipped,
            pending_cleared, effect_ready, chose_to_pay, needs_reprocess);
        let snap = Self {
            just_completed_key,
            entry_player_id,
            cost_entry_trigger,
            cost_entry_card_id,
            cost_entry_opt_result,
            saved_activating_card,
            saved_activating_ability_index,
            optional_skipped,
            pending_cleared,
        };
        let outcome = if optional_skipped || pending_cleared {
            ResumeOutcome::CompleteSkipped
        } else if needs_reprocess {
            ResumeOutcome::Reprocess
        } else if cost_was_paid {
            ResumeOutcome::FinishPaid
        } else {
            ResumeOutcome::CompleteIdle
        };
        (snap, outcome)
    }
}

/// A resolved activatable ability: index, parsed ability, and the zone
/// it activates from. Shared by the printed-ability and gained-ability
/// paths of `handle_use_ability`.
struct AbilityActivation {
    idx: usize,
    ability: crate::Arc<crate::card::Ability>,
    loc: Zone,
}

impl super::TurnEngine {
    pub fn execute_main_phase_action(
        game_state: &mut GameState,
        action: &crate::game_setup::ActionType,
        card_id: Option<i16>,
        card_indices: Option<Vec<usize>>,
        stage_area: Option<crate::zones::MemberArea>,
        use_baton_touch: Option<bool>,
    ) -> Result<(), String> {
        Self::execute_main_phase_action_with_ability_index(
            game_state,
            action,
            card_id,
            card_indices,
            stage_area,
            use_baton_touch,
            None,
        )
    }

    /// Refill hand from live zone on pass during live-card-set phases.
    /// Shared by the FirstAttacker / SecondAttacker pass arms, which differ
    /// only in tag and next phase.
    fn refill_live_zone_hand(game_state: &mut GameState, tag: &str) {
        let player = game_state.active_player_mut();
        let cards_placed = player.live_card_zone.cards.len();
        for _ in 0..cards_placed {
            let _ = player.draw_card();
        }
        game_state.push_debug_note_fmt(format_args!(
            "pass live_card_set({}): refill +{} from live zone",
            tag, cards_placed
        ));
    }

    /// Numeric RPS value for a choice action.
    fn rps_choice_value(action: &crate::game_setup::ActionType) -> u8 {
        match action {
            crate::game_setup::ActionType::RockChoice => 0,
            crate::game_setup::ActionType::PaperChoice => 1,
            crate::game_setup::ActionType::ScissorsChoice => 2,
            _ => unreachable!(),
        }
    }

    /// Route an RPS pick: PVP sessions use the stored player id, sandbox
    /// plays P1 then P2 sequentially.
    fn handle_rps_choice(game_state: &mut GameState, choice_value: u8) -> Result<(), String> {
        // PVP: route by player_id from session
        if let Some(pid) = game_state.pending_rps_player_id {
            let r = if pid == 0 {
                Self::handle_rps_choice_p1(game_state, choice_value)
            } else {
                Self::handle_rps_choice_p2(game_state, choice_value)
            };
            game_state.pending_rps_player_id = None;
            r
        } else {
            // Sandbox / no player context: sequential (P1 then P2)
            if game_state.player1_rps_choice.is_none() {
                let result = Self::handle_rps_choice_p1(game_state, choice_value);
                game_state.pending_rps_player_id = Some(1);
                result
            } else {
                Self::handle_rps_choice_p2(game_state, choice_value)
            }
        }
    }

    /// First/second attacker choice: set flags, draw 6 each, enter mulligan.
    /// The two arms differ only in the RPS-winner polarity.
    fn handle_choose_attacker(game_state: &mut GameState, p1_first: bool) -> Result<(), String> {
        game_state.player1.is_first_attacker = p1_first;
        game_state.player2.is_first_attacker = !p1_first;
        for _ in 0..6 {
            game_state.player1.draw_card();
            game_state.player2.draw_card();
        }
        game_state.current_phase = Phase::MulliganFirstAttacker;
        game_state.mulligan_selected_indices.clear();
        Ok(())
    }

    pub fn execute_main_phase_action_with_ability_index(
        game_state: &mut GameState,
        action: &crate::game_setup::ActionType,
        card_id: Option<i16>,
        card_indices: Option<Vec<usize>>,
        stage_area: Option<crate::zones::MemberArea>,
        use_baton_touch: Option<bool>,
        ability_index: Option<usize>,
    ) -> Result<(), String> {
        #[cfg(not(feature = "no_std"))]
        let _t = crate::timer::Timer::start("execute_main_phase_action");
        // UseAbility must check activation legality independently  Enever
        // route through resume_with_choice, even when another ability's choice
        // is pending (e.g. a debut look-and-select from play_to_stage).
        if matches!(action, crate::game_setup::ActionType::UseAbility) {
            if game_state.has_pending_choice() {
                return Err("Cannot activate ability while another choice is pending".to_string());
            }
            return {
                #[cfg(not(feature = "no_std"))]
                let _t = crate::timer::Timer::start("exec::use_ability");
                Self::handle_use_ability(game_state, card_id, ability_index)
            };
        }

        if game_state.has_pending_choice() {
            #[cfg(not(feature = "no_std"))]
            let _t = crate::timer::Timer::start("exec::resume_with_choice");
            return Self::resume_with_choice(game_state, card_id, card_indices);
        }

        match action {
            crate::game_setup::ActionType::Pass => match game_state.current_phase {
                Phase::LiveCardSetFirstAttacker => {
                    Self::refill_live_zone_hand(game_state, "FA");
                    game_state.current_phase = Phase::LiveCardSetSecondAttacker;
                    Ok(())
                }
                Phase::LiveCardSetSecondAttacker => {
                    Self::refill_live_zone_hand(game_state, "SA");
                    Self::advance_phase(game_state);
                    Ok(())
                }
                _ => {
                    Self::advance_phase(game_state);
                    Ok(())
                }
            },
            crate::game_setup::ActionType::MulliganHeader => Ok(()),
            crate::game_setup::ActionType::RockChoice
            | crate::game_setup::ActionType::PaperChoice
            | crate::game_setup::ActionType::ScissorsChoice => {
                Self::handle_rps_choice(game_state, Self::rps_choice_value(action))
            }
            crate::game_setup::ActionType::ChooseFirstAttacker => {
                Self::handle_choose_attacker(game_state, game_state.rps_winner != Some(2))
            }
            crate::game_setup::ActionType::ChooseSecondAttacker => {
                Self::handle_choose_attacker(game_state, game_state.rps_winner == Some(2))
            }
            crate::game_setup::ActionType::SelectMulligan => {
                Self::handle_mulligan_selection(game_state, card_id, card_indices)
            }
            crate::game_setup::ActionType::ConfirmMulligan => {
                Self::handle_mulligan_confirmation(game_state, card_indices.clone())
            }
            crate::game_setup::ActionType::SkipMulligan => Self::handle_mulligan_skip(game_state),
            crate::game_setup::ActionType::PlayMemberToStage => {
                #[cfg(not(feature = "no_std"))]
                let _t = crate::timer::Timer::start("exec::play_member_to_stage");
                Self::handle_play_member_to_stage(
                    game_state,
                    card_id,
                    card_indices.clone(),
                    stage_area,
                    use_baton_touch,
                )
            }
            crate::game_setup::ActionType::SetLiveCard => {
                Self::handle_set_live_card(game_state, card_id)
            }
            crate::game_setup::ActionType::LiveCardHeader => Ok(()),
            crate::game_setup::ActionType::SelectLiveCard => {
                Self::handle_live_card_selection(game_state, card_id, card_indices)
            }
            crate::game_setup::ActionType::ConfirmLiveCardSet => {
                Self::handle_live_card_confirmation(game_state, card_indices.clone())
            }
            crate::game_setup::ActionType::SkipLiveCardSet => {
                Self::handle_live_card_skip(game_state)
            }
            crate::game_setup::ActionType::FinishLiveCardSet => {
                Err("FinishLiveCardSet action is obsolete - use Pass instead".into())
            }
            crate::game_setup::ActionType::UseAbility => {
                Self::handle_use_ability(game_state, card_id, ability_index)
            }
            _ => Ok(()),
        }
    }

    /// Zone an activation ability plays from: explicit location condition
    /// (hand/discard) or stage by default.
    fn activation_location(ability: &crate::card::Ability) -> Zone {
        ability
            .effect
            .as_ref()
            .and_then(|e| e.activation_condition_parsed_any())
            .and_then(|c| {
                if c.condition_type() == Some(ConditionType::LocationCondition)
                    || matches!(c.get_location(), Some("hand") | Some("discard"))
                {
                    Zone::from_str(c.get_location().unwrap_or(""))
                } else {
                    None
                }
            })
            .unwrap_or(Zone::Stage)
    }

    fn cost_has_hand_self_move(cost: &crate::card::AbilityEffect) -> bool {
        match cost.action {
            ActionType::MoveCards => {
                cost.source_zone() == Some(Zone::Hand) && cost.self_cost_any().unwrap_or(false)
            }
            ActionType::SequentialCost => cost.compound.actions.as_ref().is_some_and(|actions| {
                actions
                    .iter()
                    .any(|action| Self::cost_has_hand_self_move(action.as_ref()))
            }),
            _ => false,
        }
    }

    /// Can this card activate this ability from `loc` right now?
    fn can_activate_at_location(
        player: &crate::player::Player,
        ability: &crate::card::Ability,
        card_id: i16,
        loc: Zone,
    ) -> bool {
        match loc {
            Zone::Hand => player.hand.cards.contains(&card_id),
            Zone::Discard => player.waitroom.cards.contains(&card_id),
            Zone::Stage => {
                let stage_position = player.stage.stage.iter().position(|&id| id == card_id);
                if let Some(pos) = stage_position {
                    let stage_area = crate::ability::util::pos_to_area(pos);
                    crate::zones::check_trigger_position(ability.triggers.as_deref(), stage_area)
                        && crate::zones::check_effect_position(
                            ability
                                .effect
                                .as_ref()
                                .and_then(|e| e.activation_position_any()),
                            stage_area,
                        )
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Mandatory-cost affordability pre-check: same evaluator as generation
    /// (rules 9.6.2.3 — an unpayable cost is not a legal activation).
    /// Optional payments may be skipped, so they never block here.
    fn check_mandatory_activation_cost(
        game_state: &GameState,
        player_id: &str,
        ability: &crate::card::Ability,
        active_energy: u8,
    ) -> Result<(), String> {
        if let Some(ref cost) = ability.cost {
            let groups = game_state.distinct_stage_groups(player_id);
            let effective = game_state.effective_activation_cost_for(cost, groups) as u32;
            if !cost.has_optional_payment() && effective > u32::from(active_energy) {
                return Err(format!(
                    "Cannot activate: cost {} energy exceeds active {}",
                    effective, active_energy
                ));
            }
        }
        Ok(())
    }

    /// Gained-ability fallback: an Activation trigger granted to this card
    /// by another effect (indexed past GAINED_ABILITY_INDEX_BASE).
    fn find_gained_activation(
        game_state: &GameState,
        player: &crate::player::Player,
        card_id: i16,
    ) -> Option<AbilityActivation> {
        let (gained_idx, gained) =
            game_state
                .gained_card_abilities
                .get(&card_id)
                .and_then(|list| {
                    list.iter()
                        .enumerate()
                        .find(|(_, a)| a.has_trigger(crate::triggers::TriggerKind::Activation))
                        .map(|(i, a)| (i, a.clone()))
                })?;
        if !player.stage.stage.contains(&card_id) {
            return None;
        }
        Some(AbilityActivation {
            idx: crate::ability::types::GAINED_ABILITY_INDEX_BASE + gained_idx,
            ability: crate::Arc::new(gained),
            loc: Zone::Stage,
        })
    }

    /// Queue id for the activation: gained abilities use the
    /// "card_no_gained_{idx}" format so trigger_auto_ability's
    /// gained-ability code path can find and enqueue them.
    fn activation_ability_id(
        game_state: &GameState,
        card_no: &str,
        card_id: i16,
        idx: usize,
        ability: &crate::card::Ability,
    ) -> String {
        if let Some(gidx) = crate::ability::types::gained_ability_index(idx) {
            debug_assert!(
                game_state.gained_card_abilities.contains_key(&card_id),
                "gained ability index but no gained_card_abilities entry"
            );
            format!("{}_gained_{}", card_no, gidx)
        } else {
            format!("{}_{}", card_no, ability.full_text)
        }
    }

    fn handle_use_ability(
        game_state: &mut GameState,
        card_id: Option<i16>,
        requested_ability_index: Option<usize>,
    ) -> Result<(), String> {
        let card_id = card_id.ok_or("No card specified for ability activation")?;
        if game_state.is_action_prohibited("cannot_activate")
            || game_state.is_action_prohibited("cannot_activate_by_effect")
        {
            return Err("Ability activation is prohibited by a restriction effect".to_string());
        }
        let card_db = game_state.card_database.clone();
        let card = card_db
            .get_card(card_id)
            .ok_or("Card not found in database")?;
        if !card.is_member() {
            return Err("Only member cards can activate abilities".to_string());
        }
        let player = game_state.active_player();
        let player_id = player.id.clone();
        let log_prefix = if player_id == "p1" || player_id == "player1" {
            "P1"
        } else {
            "P2"
        };

        // Find the requested ability, or the first one for legacy callers, that can
        // be activated from the current location.
        let mut ability_to_activate: Option<AbilityActivation> = None;
        for (idx, ar) in card.abilities.iter().enumerate() {
            if requested_ability_index.is_some_and(|requested| requested != idx) {
                continue;
            }
            let ability = ar.resolve();
            if ability.has_trigger(crate::triggers::TriggerKind::Activation) {
                let loc = Self::activation_location(&ability);

                log::debug!(
                    "[ACTIVATE_CHECK] ability idx={} triggers={:?} loc={:?} card_pos={:?}",
                    idx,
                    ability.triggers,
                    loc,
                    player.stage.stage.iter().position(|&id| id == card_id)
                );
                if Self::can_activate_at_location(player, &ability, card_id, loc) {
                    // Check use limit
                    if let Some(use_limit) = ability.use_limit {
                        let key = (card_id, idx, game_state.turn_number);
                        if !crate::ability::util::ability_under_use_limit(
                            game_state, &key, use_limit,
                        ) {
                            continue;
                        }
                    }
                    Self::check_mandatory_activation_cost(
                        game_state,
                        &player_id,
                        &ability,
                        player.energy_zone.active_count(),
                    )?;
                    ability_to_activate = Some(AbilityActivation { idx, ability, loc });
                    break;
                }
            }
        }

        if ability_to_activate.is_none() {
            ability_to_activate = Self::find_gained_activation(game_state, player, card_id);
        }

        let AbilityActivation { ability, loc, idx } = ability_to_activate
            .ok_or("No activatable ability found for this card at its current location")?;

        // The activating card is known from here on: record it before any
        // cost logic so self-referential filters resolve to this card.
        game_state.activating_card = Some(card_id);

        // Mandatory costs must be fully payable BEFORE anything happens
        // (Rule 9.4.2.3 / Q56): a mandatory-unpayable activation is refused
        // here, never fizzled at resolution. Optional components skip at pay
        // time (wakana bp2-008, umi Q228) and are excluded from this check;
        // energy stays owned by check_mandatory_activation_cost above.
        if let Some(ref cost) = ability.cost {
            crate::ability::resolver::AbilityResolver::validate_mandatory_cost(
                game_state,
                &cost.0,
                Some(card_id),
            )?;
        }

        let hand_self_cost = ability
            .cost
            .as_ref()
            .is_some_and(|cost| Self::cost_has_hand_self_move(&cost.0));
        if loc == Zone::Hand && !hand_self_cost {
            let player = game_state.active_player_mut();
            player.hand.cards.retain(|id| *id != card_id);
            player.waitroom.add_card(card_id);
        } else if loc == Zone::Hand {
            log::debug!("[HAND_ACTIVATION] card={} defer_self_cost=true", card_id);
        }

        // Gained abilities use the "card_no_gained_{idx}" format so
        // trigger_auto_ability's gained-ability code path (line ~683)
        // can find and enqueue them.
        let ability_id =
            Self::activation_ability_id(game_state, &card.card_no, card_id, idx, &ability);
        game_state.trigger_auto_ability(
            ability_id,
            crate::game_state::AbilityTrigger::Activation,
            player_id.as_str(),
            Some(card.card_no.to_string()),
            Some(card_id),
            None,
            None,
        );
        game_state.process_pending_auto_abilities(&player_id);
        game_state.push_rule_log_fmt(format_args!(
            "{} [[log_activation]] {}: {}",
            log_prefix, card.name, ability.full_text
        ));
        Ok(())
    }

    /// Best-effort labels for what the player picked, resolving card indices to
    /// card names where possible. Mirrors the decision logic in
    /// `build_choice_result` without mutating game state.
    fn profile_chosen_labels(
        game_state: &GameState,
        choice: &crate::ability::types::Choice,
        card_id: Option<i16>,
        card_indices: Option<&[usize]>,
    ) -> Vec<String> {
        use crate::ability::types::Choice as C;
        match choice {
            C::SelectCard { zone, .. } => {
                let idxs = card_indices
                    .map(|v| v.to_vec())
                    .or_else(|| {
                        card_id
                            .and_then(|id| usize::try_from(id).ok())
                            .map(|id| vec![id])
                    })
                    .unwrap_or_default();
                if idxs.is_empty() {
                    vec!["(none)".to_string()]
                } else {
                    let name = |i: usize| game_state.resolve_index_any_player(zone, i);
                    idxs.iter().map(|&i| name(i)).collect()
                }
            }
            C::SelectTarget {
                options, target, ..
            } => {
                let chosen = match card_id {
                    Some(-1) => Some("skip".to_string()),
                    Some(id)
                        if target != "choice"
                            && target != "choice_string"
                            && target != "conditional_optional" =>
                    {
                        // Use the option text when it's a labelled option.
                        if let Some(ref o) = options {
                            if id >= 0 {
                                if let Ok(idx) = usize::try_from(id) {
                                    if idx < o.len() {
                                        Some(o[idx].clone())
                                    } else {
                                        Some(id.to_string())
                                    }
                                } else {
                                    Some(id.to_string())
                                }
                            } else {
                                Some(id.to_string())
                            }
                        } else {
                            Some(id.to_string())
                        }
                    }
                    Some(id) => Some(id.to_string()),
                    None if card_indices.is_none_or(|v| v.is_empty()) => {
                        None // may be skipped; handled by skip flag
                    }
                    None => card_indices
                        .and_then(|v| v.first())
                        .and_then(|&i| options.as_ref().and_then(|o| o.get(i).cloned())),
                };
                chosen.map(|s| vec![s]).unwrap_or_default()
            }
            C::SelectPosition { .. } => card_id
                .map(|id| {
                    vec![match id {
                        0 => "left".to_string(),
                        1 => "center".to_string(),
                        2 => "right".to_string(),
                        _ => "center".to_string(),
                    }]
                })
                .unwrap_or_default(),
            C::SelectHeartColor { options, .. } | C::SelectHeartType { options, .. } => card_id
                .map(|id| {
                    if id >= 0 {
                        if let Ok(idx) = usize::try_from(id) {
                            if idx < options.len() {
                                vec![options[idx].clone()]
                            } else {
                                vec!["heart00".to_string()]
                            }
                        } else {
                            vec!["heart00".to_string()]
                        }
                    } else {
                        vec!["heart00".to_string()]
                    }
                })
                .or_else(|| {
                    card_indices.and_then(|v| v.first().copied()).map(|idx| {
                        if idx < options.len() {
                            vec![options[idx].clone()]
                        } else {
                            vec!["heart00".to_string()]
                        }
                    })
                })
                .unwrap_or_default(),
            C::SelectAutoAbility { options, .. } => card_id
                .map(|id| {
                    if id >= 0 {
                        if let Ok(idx) = usize::try_from(id) {
                            if idx < options.len() {
                                vec![options[idx].card_name.clone()]
                            } else {
                                vec![format!("#{id}")]
                            }
                        } else {
                            vec![format!("#{id}")]
                        }
                    } else {
                        vec![format!("#{id}")]
                    }
                })
                .unwrap_or_default(),
            C::SelectLiveSuccess { options, .. } => {
                let idx = card_indices
                    .and_then(|v| v.first().copied())
                    .or_else(|| card_id.and_then(|id| usize::try_from(id).ok()))
                    .unwrap_or(0);
                if idx < options.len() {
                    vec![options[idx].card_name.clone()]
                } else {
                    vec![format!("#{idx}")]
                }
            }
        }
    }

    /// Whether the raw resume inputs map to a "skip" outcome.
    fn profile_choice_is_skip(
        choice: &crate::ability::types::Choice,
        card_id: Option<i16>,
        card_indices: Option<&[usize]>,
    ) -> bool {
        use crate::ability::types::Choice as C;
        match choice {
            C::SelectCard { .. } => {
                card_indices.is_some_and(|v| v.is_empty())
                    || (card_indices.is_none() && card_id.is_none())
            }
            C::SelectTarget {
                target, allow_skip, ..
            } => {
                if !allow_skip {
                    return false;
                }
                match target.as_str() {
                    "primary|alternative" => card_id == Some(2),
                    crate::ability::types::PAY_SKIP_TARGET => card_id == Some(2),
                    "pay_cost_all:discard_all" => card_id == Some(2),
                    "choice" | "choice_string" | "conditional_optional" => {
                        card_id.is_none() && card_indices.is_none_or(|v| v.is_empty())
                    }
                    _ => card_id == Some(-1),
                }
            }
            C::SelectPosition { allow_skip, .. } => *allow_skip && card_id.is_none(),
            C::SelectHeartColor { .. } | C::SelectHeartType { .. } => false,
            C::SelectAutoAbility { .. } | C::SelectLiveSuccess { .. } => false,
        }
    }

    pub fn resume_with_choice(
        game_state: &mut GameState,
        card_id: Option<i16>,
        card_indices: Option<Vec<usize>>,
    ) -> Result<(), String> {
        if game_state.pending_loop_protocol.is_some() {
            let choice = game_state
                .pending_loop_protocol
                .as_ref()
                .map(|protocol| protocol.choice.clone())
                .ok_or("No pending choice to resume")?;
            if let crate::ability::types::Choice::SelectTarget { options, .. } = &choice {
                let selected = card_id
                    .and_then(|id| usize::try_from(id).ok())
                    .and_then(|idx| options.as_ref().and_then(|values| values.get(idx)));
                match selected {
                    Some(value) if value == "Continue" => {
                        game_state.resolve_loop_protocol(true);
                        return Ok(());
                    }
                    Some(value) if value == "Stop" => {
                        game_state.resolve_loop_protocol(false);
                        return Ok(());
                    }
                    _ => return Err("Invalid Rule 12.1 protocol choice".to_string()),
                }
            }
        }
        let choice = game_state
            .ability_queue
            .take_waiting_choice()
            .ok_or("No pending choice to resume")?;

        // Robustness (P3): a NON-skippable prompt must never receive an
        // empty answer. Previously such answers fell through to handlers
        // that silently no-opped (e.g. position|destination resolving to
        // "Unknown source position"), dropping cards without any error.
        {
            let empty_answer =
                card_id.is_none() && card_indices.as_deref().is_none_or(|v| v.is_empty());
            let skippable = match &choice {
                crate::ability::types::Choice::SelectCard { allow_skip, .. }
                | crate::ability::types::Choice::SelectTarget { allow_skip, .. }
                | crate::ability::types::Choice::SelectPosition { allow_skip, .. } => *allow_skip,
                _ => true,
            };
            if empty_answer && !skippable {
                let kind = match &choice {
                    crate::ability::types::Choice::SelectCard { .. } => "SelectCard",
                    crate::ability::types::Choice::SelectTarget { .. } => "SelectTarget",
                    crate::ability::types::Choice::SelectPosition { .. } => "SelectPosition",
                    crate::ability::types::Choice::SelectHeartColor { .. } => "SelectHeartColor",
                    crate::ability::types::Choice::SelectHeartType { .. } => "SelectHeartType",
                    crate::ability::types::Choice::SelectAutoAbility { .. } => "SelectAutoAbility",
                    crate::ability::types::Choice::SelectLiveSuccess { .. } => "SelectLiveSuccess",
                };
                if matches!(choice, crate::ability::types::Choice::SelectCard { .. }) {
                    game_state.ability_queue.restore_waiting_choice(choice);
                    let offered = crate::game_setup::generate_possible_actions(game_state);
                    let has_selectable = offered.iter().any(|a| {
                        a.action_type == crate::game_setup::ActionType::ChoiceSelect
                            && a.parameters.as_ref().and_then(|p| p.disabled) != Some(true)
                            && a.parameters
                                .as_ref()
                                .and_then(|p| p.card_indices.as_deref())
                                .is_some_and(|v| !v.is_empty())
                    });
                    let choice = game_state
                        .ability_queue
                        .take_waiting_choice()
                        .ok_or("No pending choice to resume")?;
                    if !has_selectable {
                        log::debug!(
                            "[CHOICE] empty non-skippable SelectCard with no eligible cards — auto-skip (was: {kind})"
                        );
                        return Self::resume_queue_with_choice(
                            game_state,
                            choice,
                            crate::ability::types::ChoiceResult::Skip,
                        );
                    }
                    game_state.ability_queue.restore_waiting_choice(choice);
                } else {
                    game_state.ability_queue.restore_waiting_choice(choice);
                }
                return Err(format!(
                    "non-skippable {kind} prompt requires a selection - empty answer rejected",
                ));
            }
        }

        // Record a structured `choice_resolved` entry: what was offered vs chosen.
        // Skipped under `headless`  Ethe label computation itself allocates.
        #[cfg(not(feature = "headless"))]
        if crate::game_setup::logging_enabled() {
            game_state.push_choice_resolved(
                &choice,
                Self::profile_chosen_labels(game_state, &choice, card_id, card_indices.as_deref()),
                Self::profile_choice_is_skip(&choice, card_id, card_indices.as_deref()),
            );
        }
        #[cfg(feature = "headless")]
        {
            let _ = (&choice, &card_id, &card_indices);
        }

        // Handle non-ability choices early (live success, etc.)
        if matches!(
            choice,
            crate::ability::types::Choice::SelectLiveSuccess { .. }
        ) {
            let result = Self::build_choice_result(&choice, card_id, card_indices.clone(), None)?;
            if let crate::ability::types::ChoiceResult::LiveSuccessSelected { card_index } = &result
            {
                let player_id = match &choice {
                    crate::ability::types::Choice::SelectLiveSuccess { player_id, .. } => {
                        player_id.clone()
                    }
                    _ => return Err("Wrong choice type".to_string()),
                };
                super::TurnEngine::handle_live_success_choice(game_state, *card_index, &player_id)?;
                game_state.ability_queue.complete_current();
                return Ok(());
            }
        }

        // Handle success zone replacement choices (e.g. 錯覚CROSSROADS)
        if let Some(replaced_card_id) = game_state.pending_success_replacement_card_id.take() {
            let player_id = game_state
                .pending_success_replacement_player_id
                .take()
                .unwrap_or_else(|| "player1".to_string());
            let result = Self::build_choice_result(&choice, card_id, card_indices, None)?;
            let filtered_indices = match &choice {
                crate::ability::types::Choice::SelectCard {
                    filtered_indices, ..
                } => filtered_indices.as_ref(),
                _ => None,
            };
            let physical_idx = match &result {
                crate::ability::types::ChoiceResult::CardSelected { indices } => {
                    indices.first().map(|index| {
                        filtered_indices
                            .and_then(|filtered| filtered.get(*index))
                            .copied()
                            .unwrap_or(*index)
                    })
                }
                _ => None,
            };
            let player = if player_id == game_state.player1.id {
                &mut game_state.player1
            } else {
                &mut game_state.player2
            };
            match result {
                crate::ability::types::ChoiceResult::CardSelected { indices }
                    if !indices.is_empty() =>
                {
                    // Player chose a card from discard  Emove it to success zone,
                    // and put the original card in waitroom.
                    if let Some(selected_idx) = physical_idx {
                        if selected_idx < player.waitroom.cards.len() {
                            let selected_card_id = player.waitroom.cards.remove(selected_idx);
                            // Remove the original card from live_card_zone if present
                            player
                                .live_card_zone
                                .cards
                                .retain(|cid| *cid != replaced_card_id);
                            player.waitroom.add_card(replaced_card_id);
                            // Move selected card to success zone
                            player.success_live_card_zone.cards.push(selected_card_id);
                        }
                    }
                    // Move any remaining live cards to waitroom
                    while !player.live_card_zone.cards.is_empty() {
                        player
                            .waitroom
                            .add_card(player.live_card_zone.cards.remove(0));
                    }
                }
                _ => {
                    // Player declined replacement (Skip or empty indices)  E                    // place original card in success zone normally
                    player
                        .live_card_zone
                        .cards
                        .retain(|cid| *cid != replaced_card_id);
                    player.success_live_card_zone.cards.push(replaced_card_id);
                    while !player.live_card_zone.cards.is_empty() {
                        player
                            .waitroom
                            .add_card(player.live_card_zone.cards.remove(0));
                    }
                }
            }
            game_state.ability_queue.complete_current();
            return Ok(());
        }

        // Play-time cost reduction choice (play-time cost reduction):
        if let crate::ability::types::Choice::SelectTarget { target, .. } = &choice {
            if target == "play_time_cost_reduction" {
                let accepted = card_id == Some(1); // option "Yes"
                game_state.play_time_cost_reduction_accepted = Some(accepted);
                let play = game_state
                    .play_time_cost_play
                    .clone()
                    .ok_or("No pending play-time cost play to resume")?;
                game_state.ability_queue.complete_current();
                return Self::handle_play_member_to_stage(
                    game_state,
                    Some(play.card_id),
                    None,
                    Some(play.area),
                    Some(false),
                );
            }
        }

        let choice_card_no = game_state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_card_no.clone());
        let result =
            Self::build_choice_result(&choice, card_id, card_indices, choice_card_no.as_ref())?;
        Self::resume_queue_with_choice(game_state, choice, result)
    }

    /// Binary yes/no-style targets (`pay` vs `skip`). ONE table for the
    /// PAY_SKIP / pay_cost_all / primary|alternative arms, which only differ
    /// in labels. Returns `None` for non-binary targets.
    fn binary_target_result(target: &str, card_id: Option<i16>) -> Option<String> {
        const ROWS: &[(&str, &str, &str)] = &[
            (
                crate::ability::types::PAY_SKIP_TARGET,
                "pay_optional_cost",
                "skip_optional_cost",
            ),
            (
                "pay_cost_all:discard_all",
                "pay_cost_all",
                "skip_optional_cost",
            ),
            ("primary|alternative", "alternative", "primary"),
        ];
        ROWS.iter()
            .find(|(t, _, _)| *t == target)
            .map(|(_, one, other)| {
                if card_id == Some(1) {
                    one.to_string()
                } else {
                    other.to_string()
                }
            })
    }

    /// Map an answer (indices channel first, then card id) through an option
    /// list. Shared by the position|destination / area_select /
    /// double_baton_touch / self_or_opponent arms: before this helper each
    /// spelled the same nested lookup, and an indices-channel answer that
    /// fell through became the raw number as destination ("0" matches no
    /// zone — the card was silently dropped).
    fn lookup_option_target(
        options: Option<&Vec<String>>,
        card_id: Option<i16>,
        card_indices: &Option<Vec<usize>>,
    ) -> Option<String> {
        let opts = options?;
        if let Some(&idx) = card_indices.as_deref().and_then(|v| v.first()) {
            if idx < opts.len() {
                return Some(opts[idx].clone());
            }
        }
        if let Some(id) = card_id {
            if id >= 0 {
                if let Ok(idx) = usize::try_from(id) {
                    if idx < opts.len() {
                        return Some(opts[idx].clone());
                    }
                }
            }
        }
        None
    }

    /// Bounds-checked option pick with fallback. Shared by the
    /// SelectHeartColor / SelectHeartType arms (both default heart00).
    fn pick_heart_option(options: &[String], card_id: Option<i16>) -> String {
        let idx = card_id.and_then(|id| usize::try_from(id).ok()).unwrap_or(0);
        options
            .get(idx)
            .cloned()
            .unwrap_or_else(|| "heart00".to_string())
    }

    /// Answer index, indices channel first, then card id, else 0. Shared by
    /// the SelectLiveSuccess / SelectAutoAbility arms.
    fn answer_index(card_id: Option<i16>, card_indices: &Option<Vec<usize>>) -> usize {
        card_indices
            .as_ref()
            .and_then(|v| v.first().copied())
            .or_else(|| card_id.and_then(|id| usize::try_from(id).ok()))
            .unwrap_or(0)
    }

    fn build_choice_result(
        choice: &crate::ability::types::Choice,
        card_id: Option<i16>,
        card_indices: Option<Vec<usize>>,
        choice_card_no: Option<&ChoiceRoute>,
    ) -> Result<crate::ability::types::ChoiceResult, String> {
        match choice {
            crate::ability::types::Choice::SelectCard { .. } => {
                let indices = card_indices.unwrap_or_else(|| {
                    card_id
                        .and_then(|id| usize::try_from(id).ok())
                        .map(|id| vec![id])
                        .unwrap_or_default()
                });
                Ok(crate::ability::types::ChoiceResult::CardSelected { indices })
            }
            crate::ability::types::Choice::SelectTarget {
                target, options, ..
            } => {
                log::debug!(
                    "[BCR] SelectTarget target={} card_id={:?} card_indices={:?} options={:?}",
                    target,
                    card_id,
                    card_indices,
                    options
                );
                let selected = match target.as_str() {
                    t @ (crate::ability::types::PAY_SKIP_TARGET
                    | "pay_cost_all:discard_all"
                    | "primary|alternative") => {
                        Self::binary_target_result(t, card_id).unwrap_or_default()
                    }
                    "choice" | "choice_string" | "conditional_optional" => {
                        // card_id=None + card_indices absent/empty means skip
                        if card_id.is_none() && card_indices.as_deref().is_none_or(|v| v.is_empty())
                        {
                            return Ok(crate::ability::types::ChoiceResult::Skip);
                        }
                        card_id
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| "0".into())
                    }
                    _ => {
                        // For position_change:opponent choices, use card_id as option index
                        // to look up the actual option string instead of the raw index.
                        if choice_card_no
                            == Some(&ChoiceRoute::Raw(
                                "position_change:opponent:front".to_string(),
                            ))
                        {
                            if let Some(found) =
                                Self::lookup_option_target(options.as_ref(), card_id, &None)
                            {
                                return Ok(crate::ability::types::ChoiceResult::TargetSelected {
                                    target: found,
                                });
                            }
                        }
                        // For position|destination choices, look up the option text in the
                        // options array by index.  The options array contains the actual
                        // position names (e.g. "left", "center", "right") that the handler
                        // expects, not raw numeric indices.
                        // double_baton_touch likewise: its handler parses the option
                        // text as an "area1,area2" pair and cannot read numeric ids.
                        // self_or_opponent likewise: its options are the two player
                        // names, not a zone.
                        //
                        // Indices channel (select_indices / web UI): map the
                        // first index through the option list. Before this arm
                        // existed, an indices-channel answer fell through and
                        // the raw number became the destination string ("0"),
                        // which matches no zone — the card was silently dropped
                        // (found by zone_change_gate_test riko_responds_only_to_own_side).
                        let target_needs_option_text = matches!(
                            target.as_str(),
                            "position|destination" | "area_select" | "double_baton_touch" | "self_or_opponent"
                        );
                        if target_needs_option_text {
                            if let Some(found) =
                                Self::lookup_option_target(options.as_ref(), card_id, &card_indices)
                            {
                                return Ok(crate::ability::types::ChoiceResult::TargetSelected {
                                    target: found,
                                });
                            }
                        }
                        match card_id {
                            Some(-1) => "skip".to_string(),
                            Some(id) => id.to_string(),
                            None => "0".into(),
                        }
                    }
                };
                Ok(crate::ability::types::ChoiceResult::TargetSelected { target: selected })
            }
            crate::ability::types::Choice::SelectPosition { .. } => {
                let pos = card_id
                    .and_then(|id| usize::try_from(id).ok())
                    .and_then(|idx| ["left", "center", "right"].get(idx).copied())
                    .unwrap_or("center")
                    .to_string();
                Ok(crate::ability::types::ChoiceResult::PositionSelected { position: pos })
            }
            crate::ability::types::Choice::SelectHeartColor {
                count: _, options, ..
            } => Ok(crate::ability::types::ChoiceResult::HeartColorSelected {
                colors: vec![Self::pick_heart_option(options, card_id)],
            }),
            crate::ability::types::Choice::SelectHeartType {
                count: _, options, ..
            } => Ok(crate::ability::types::ChoiceResult::HeartTypeSelected {
                types: vec![Self::pick_heart_option(options, card_id)],
            }),
            crate::ability::types::Choice::SelectLiveSuccess { options, .. } => {
                let idx = Self::answer_index(card_id, &card_indices);
                let card_index = options.get(idx).map(|o| o.card_index).unwrap_or(0);
                Ok(crate::ability::types::ChoiceResult::LiveSuccessSelected { card_index })
            }
            crate::ability::types::Choice::SelectAutoAbility { options, .. } => {
                // NOTE: card_id only (no indices channel) — preserved as-is.
                let idx = card_id.and_then(|id| usize::try_from(id).ok()).unwrap_or(0);
                let queue_idx = options.get(idx).map(|o| o.queue_index).unwrap_or(0);
                Ok(crate::ability::types::ChoiceResult::AutoAbilitySelected {
                    queue_index: queue_idx,
                })
            }
        }
    }

    /// `entry_player_id.clone().unwrap_or_else(|| "p1")` appears at every
    /// completion site below. ONE fallback for "no entry, assume p1".
    fn entry_player_or_p1(entry_player_id: &Option<String>) -> String {
        entry_player_id.clone().unwrap_or_else(|| "p1".to_string())
    }

    /// AutoAbility-choice fast path of `resume_queue_with_choice`: promote
    /// the chosen entry and run it, then drain stale same-player entries.
    /// Returns `Ok(true)` when this path handled the answer.
    fn resume_auto_ability_choice(
        game_state: &mut GameState,
        choice: &crate::ability::types::Choice,
        result: &crate::ability::types::ChoiceResult,
    ) -> Result<bool, String> {
        if let crate::ability_queue::QueueState::WaitingForAutoAbilityChoice { .. } =
            game_state.ability_queue.get_state()
        {
            if let crate::ability::types::ChoiceResult::AutoAbilitySelected { queue_index } = result
            {
                let queue_index = *queue_index;
                let player_id = if let crate::ability::types::Choice::SelectAutoAbility {
                    ref player_id,
                    ..
                } = choice
                {
                    player_id.clone()
                } else {
                    String::new()
                };
                game_state.ability_queue.resume_with_choice();
                // Set depth-first cutoff BEFORE resolution so entries queued by
                // process_current_ability (each_time watchers) are excluded from
                // the stale-entries pool when process_player_abilities re-enters.
                let cutoff = game_state.ability_queue.len();
                game_state.depth_first_cutoff = Some(u16::try_from(cutoff).unwrap());
                game_state.ability_queue.promote_entry_by_abs(queue_index);
                if game_state.ability_queue.start_next() {
                    let saved_moved = game_state.recently_moved_cards.take();
                    let saved_from_zone = game_state.recently_moved_from_zone.take();
                    game_state.recently_state_changed.clear();
                    game_state.process_current_ability();
                    if game_state.recently_moved_cards.is_none() {
                        game_state.recently_moved_cards = saved_moved;
                        game_state.recently_moved_from_zone = saved_from_zone;
                    }
                }
                // If the ability completed without pausing, drain newly-queued
                // entries (each_time watchers) immediately. If it paused (e.g.,
                // sequential effect sub-choice), the cutoff carries forward to
                // the next process_player_abilities entry.
                if !game_state.has_pending_choice() && game_state.ability_queue.is_idle() {
                    Self::drain_stale_auto_entries(game_state, cutoff, &player_id);
                }
                if !game_state.has_pending_choice() && !player_id.is_empty() {
                    game_state.process_pending_auto_abilities(&player_id);
                }
                return Ok(true);
            }
            return Err("Expected AutoAbilitySelected result".to_string());
        }
        Ok(false)
    }

    /// Drain stale same-player entries queued past `cutoff` (each_time
    /// watchers), stopping at the first pending choice or after 50 rounds.
    fn drain_stale_auto_entries(game_state: &mut GameState, cutoff: usize, pid: &str) {
        let mut drain_iters = 0;
        while !game_state.has_pending_choice() && game_state.ability_queue.is_idle() {
            drain_iters += 1;
            if drain_iters > 50 {
                break;
            }
            let new_idx = (cutoff..game_state.ability_queue.len()).find(|&i| {
                game_state.ability_queue.is_entry_available(i)
                    && game_state
                        .ability_queue
                        .entry_player_id(i)
                        .is_some_and(|id| id == pid)
            });
            match new_idx {
                Some(idx) => {
                    game_state.ability_queue.set_current_entry(idx);
                    if !game_state.ability_queue.start_next() {
                        break;
                    }
                    game_state.process_current_ability();
                    if game_state.has_pending_choice() {
                        break;
                    }
                }
                None => break,
            }
        }
    }

    /// Take the persistent resolver off the queue entry. Fails when the
    /// entry carries none (cannot resume an answer without resolver state).
    fn take_queue_resolver(
        game_state: &mut GameState,
    ) -> Result<Box<crate::ability::resolver::AbilityResolver>, String> {
        match game_state.ability_queue.take_resolver() {
            Some(mut r) => {
                // Only log a take that actually carried a take. Resuming with
                // nothing moved and nothing selected is the common path and
                // printed an all-empty line ~1.9k times per suite, burying the
                // ~600 resumes that had state worth inspecting.
                if !r.moved_cards.is_empty() || !r.selected_cards.is_empty() {
                    log::debug!(
                        "[RWC] took resolver: moved_cards={:?} selected={:?}",
                        r.moved_cards,
                        r.selected_cards
                    );
                }
                r.sub_choice_created = false;
                Ok(r)
            }
            None => Err("No resolver found on queue entry".to_string()),
        }
    }

    /// G1/G3: does a pending sub-choice target the opponent? ONE predicate
    /// for the SelectCard (target_player_id + spawn context) and
    /// SelectPosition (MoveCardsPosition context) shapes.
    fn sub_choice_targets_opponent(
        resolver: &crate::ability::resolver::AbilityResolver,
        sub_choice: &crate::ability::types::Choice,
    ) -> bool {
        let targets = match sub_choice {
            crate::ability::types::Choice::SelectCard {
                target_player_id: Some(tpid),
                ..
            } if tpid == "opponent"
                && resolver.spawn_context.target.as_deref() == Some("opponent") =>
            {
                true
            }
            crate::ability::types::Choice::SelectPosition { .. }
                if matches!(
                    resolver.execution_context,
                    crate::ability::types::ExecutionContext::MoveCardsPosition { ref target, .. }
                    if target == "opponent"
                ) =>
            {
                true
            }
            crate::ability::types::Choice::SelectTarget { target, .. }
                if target == "position|destination"
                    && resolver.spawn_context.target.as_deref() == Some("opponent") =>
            {
                true
            }
            _ => false,
        };
        log::debug!(
            "[RWC_G1] tpid_opp={} spawn={:?} choice={:?}",
            targets,
            resolver.spawn_context.target,
            sub_choice
        );
        targets
    }

    /// Route the paused choice to the right player: opponent choices go to
    /// the opponent (via the canonical `opponent_id`, not a p1/p2 literal),
    /// self-targeted SelectCards reset to the activator.
    fn route_sub_choice_player(
        game_state: &mut GameState,
        sub_choice: &crate::ability::types::Choice,
        targets_opponent: bool,
    ) {
        let current_pid = game_state
            .ability_queue
            .current_entry()
            .map(|e| e.player_id.clone());
        let Some(current) = current_pid else {
            return;
        };
        let route = game_state
            .ability_queue
            .current_entry()
            .and_then(|entry| entry.choice_card_no.as_ref())
            .map(ToString::to_string);
        let self_position_route = route
            .as_deref()
            .is_some_and(|raw| raw.starts_with("position_change:self"));
        let self_targeted = matches!(sub_choice, crate::ability::types::Choice::SelectCard { target_player_id: Some(tpid), .. } if tpid == "self")
            || self_position_route;
        log::debug!(
            "[RWC_G1_ROUTE] route={:?} self_targeted={} targets_opponent={}",
            route,
            self_targeted,
            targets_opponent
        );
        if targets_opponent {
            let opponent_id = game_state.opponent_id(&current);
            log::debug!("[RWC_G1] SET choice_player_id={}", opponent_id);
            if let Some(entry) = game_state.ability_queue.current_entry_mut() {
                entry.choice_player_id = Some(opponent_id.to_string());
            }
        } else if self_targeted {
            log::debug!("[RWC_G1] RESET choice_player_id to activator={}", current);
            if let Some(entry) = game_state.ability_queue.current_entry_mut() {
                entry.choice_player_id = Some(current);
            }
        }
    }

    /// Skipped-optional / stranded-sequential completion: finish the entry,
    /// clear tracking, and resume pending autos for the completing player.
    fn complete_skipped_ability(game_state: &mut GameState, snap: &ResumeSnapshot) {
        log::debug!(
            "[RWC] optional_skipped={} pending_cleared={} completing ability",
            snap.optional_skipped,
            snap.pending_cleared
        );
        game_state.ability_queue.complete_current();
        game_state.clear_effect_tracking();
        let player_id = Self::entry_player_or_p1(&snap.entry_player_id);
        game_state.set_just_completed(snap.just_completed_key);
        game_state.process_pending_auto_abilities(&player_id);
        game_state.just_completed_ability_key = None;
        game_state.clear_movement_tracking();
    }

    /// Reprocess path: the ability is still executing, so restore the saved
    /// activating card, store the resolver back, and re-enter processing.
    fn reprocess_ability(
        game_state: &mut GameState,
        resolver: Box<crate::ability::resolver::AbilityResolver>,
        snap: &ResumeSnapshot,
    ) {
        // Restore activating_card — the ability is still executing.
        game_state.activating_card = snap.saved_activating_card;
        game_state.activating_ability_index = snap.saved_activating_ability_index;
        log::debug!("[RWC] needs_reprocess=true: storing resolver and calling PCA");
        game_state.ability_queue.set_resolver(resolver);
        game_state.process_current_ability();
        if game_state.has_pending_choice() {
            let player_id = game_state
                .ability_queue
                .current_entry()
                .map(|e| e.player_id.clone())
                .unwrap_or_else(|| "p1".to_string());
            game_state.process_with_completed_key(snap.just_completed_key, &player_id);
        } else {
            // Effect completed without sub-choice — process any newly
            // enqueued watcher abilities (e.g. each_time triggers).
            let player_id = Self::entry_player_or_p1(&snap.entry_player_id);
            game_state.process_with_completed_key(snap.just_completed_key, &player_id);
        }
    }

    /// Shared completion tail: finish the entry, fire movement-based
    /// triggers (TAS scan), and resume. Used by the paid-finish path AND
    /// the idle path — previously copy-pasted in both.
    fn complete_and_scan(game_state: &mut GameState, snap: &ResumeSnapshot) {
        game_state.ability_queue.complete_current();
        game_state.clear_effect_tracking();
        let player_id = Self::entry_player_or_p1(&snap.entry_player_id);
        // Post-resolution TAS scan for movement-based triggers.
        // Mirrors process_current_ability's post-resolution scan, which is
        // NOT called when a resolver completes via this path.
        // Must run AFTER complete_current() to match process_current_ability ordering.
        if game_state.recently_moved_cards.is_some()
            || game_state.last_energy_placed_by_effect()
            || !game_state.recently_appeared_cards.is_empty()
        {
            let event = crate::ability::types::TriggerEvent {
                moved_cards: game_state.recently_moved_cards.clone().unwrap_or_default(),
                moved_from_zone: game_state.recently_moved_from_zone.clone(),
                position_change_occurred: game_state.position_change_occurred_this_turn,
                energy_placed_by_effect: game_state.last_energy_placed_by_effect(),
                energy_placed_by_player: game_state
                    .last_energy_placed_by_player()
                    .map(|s| s.to_string()),
                ..Default::default()
            };
            game_state.set_just_completed(snap.just_completed_key);
            game_state.trigger_auto_abilities_for_player_with_event(&player_id, &event);
            game_state.just_completed_ability_key = None;
        }
        game_state.process_with_completed_key(snap.just_completed_key, &player_id);
        game_state.clear_movement_tracking();
    }

    /// Paid-finish path: record use-limit, fire post-resolution each_time
    /// triggers, then complete (unless a sub-choice is pending).
    /// Returns true when the caller must return early.
    fn finish_paid_ability(game_state: &mut GameState, snap: &ResumeSnapshot) -> bool {
        // Record use_limit when ability completes (cost+effect both resolved).
        // Insert for any ability with use_limit, unless the player declined
        // an optional action (signaled by optional_cost_result == Some(false)).
        if snap.cost_entry_opt_result != Some(false) {
            if let Some(entry) = game_state.ability_queue.current_entry() {
                if let Some(cid) = entry.card_id {
                    let turn = game_state.turn_number;
                    let key = (cid, entry.ability_index, turn);
                    game_state.record_ability_use(key);
                }
            }
        }
        // Post-resolution each_time for LiveStart/LiveSuccess
        if snap.cost_entry_opt_result != Some(false) {
            let pid = Self::entry_player_or_p1(&snap.entry_player_id);
            if let Some(crate::game_state::AbilityTrigger::LiveStart) = snap.cost_entry_trigger {
                if let Some(cid) = snap.cost_entry_card_id {
                    game_state.trigger_each_time_for_member(&pid, crate::triggers::LIVE_START, cid);
                }
            } else if let Some(crate::game_state::AbilityTrigger::LiveSuccess) =
                snap.cost_entry_trigger
            {
                if let Some(cid) = snap.cost_entry_card_id {
                    game_state.trigger_each_time_for_member(
                        &pid,
                        crate::triggers::LIVE_SUCCESS,
                        cid,
                    );
                }
            }
        }
        // Don't complete if a pending choice (e.g. SelectPosition) was
        // created by the current effect — it would be orphaned.
        if game_state.has_pending_choice() {
            log::debug!("[RWC] skipping complete_current — pending choice exists");
            return true;
        }
        Self::complete_and_scan(game_state, snap);
        false
    }

    fn resume_queue_with_choice(
        game_state: &mut GameState,
        choice: crate::ability::types::Choice,
        result: crate::ability::types::ChoiceResult,
    ) -> Result<(), String> {
        if Self::resume_auto_ability_choice(game_state, &choice, &result)? {
            return Ok(());
        }

        game_state.ability_queue.resume_with_choice();
        let had_pending_sequential = game_state.ability_queue.has_pending_actions();

        // Take the persistent resolver from the queue entry
        let mut resolver = Self::take_queue_resolver(game_state)?;
        resolver.pending_choice = Some(choice);
        let res = resolver.provide_choice_result(game_state, result);

        if let Err(e) = res {
            log::debug!("[RWC_ERROR] provide_choice_result failed: {}", e);
            game_state.ability_queue.complete_current();
            return Err(e);
        }

        // If inner processing (e.g. depth-first trigger scan) created a
        // pending choice on the game-state level (a different queue entry),
        // don't touch it  Ereturn immediately.
        if game_state.has_pending_choice() {
            log::debug!("[RWC] inner processing created pending choice  Ereturning early");
            return Ok(());
        }

        log::debug!(
            "[RWC] after provide: pending_choice={:?} moved_cards={:?} selected={:?}",
            resolver.pending_choice.is_some(),
            resolver.moved_cards,
            resolver.selected_cards
        );

        if resolver.pending_choice.is_some() {
            // Sub-choice created — store resolver back on entry and pause queue
            let sub_choice = resolver.pending_choice.clone().unwrap();
            // G1/G3: route pending choice to opponent if it targets opponent
            let targets_opponent = Self::sub_choice_targets_opponent(&resolver, &sub_choice);
            Self::route_sub_choice_player(game_state, &sub_choice, targets_opponent);
            resolver.store_pending_choice(game_state);
            game_state.ability_queue.set_resolver(resolver);
            game_state.ability_queue.pause_for_choice(sub_choice);
        } else {
            // No more choices — ability execution finished
            let (snap, outcome) = ResumeSnapshot::capture(game_state, had_pending_sequential);
            match outcome {
                ResumeOutcome::CompleteSkipped => Self::complete_skipped_ability(game_state, &snap),
                ResumeOutcome::Reprocess => Self::reprocess_ability(game_state, resolver, &snap),
                ResumeOutcome::FinishPaid => {
                    if Self::finish_paid_ability(game_state, &snap) {
                        return Ok(());
                    }
                }
                ResumeOutcome::CompleteIdle => Self::complete_and_scan(game_state, &snap),
            }
        }
        Ok(())
    }

    pub fn check_timing(game_state: &mut GameState) {
        #[cfg(not(feature = "no_std"))]
        let _t = crate::timer::Timer::start("check_timing");
        Self::check_duplicate_members(&mut game_state.player1, &game_state.card_database);
        Self::check_duplicate_members(&mut game_state.player2, &game_state.card_database);
        tdbg!("CHECK_TIMING:0");
        if game_state.player1.is_first_attacker {
            game_state.player1.refresh();
            game_state.player2.refresh();
        } else {
            game_state.player2.refresh();
            game_state.player1.refresh();
        }
        tdbg!("CHECK_TIMING:1 refresh OK");
        tdbg!("CHECK_TIMING:3 refresh done");
        {
            #[cfg(not(feature = "no_std"))]
            let _t = crate::timer::Timer::start("check_timing::check_victory_condition");
            Self::check_victory_condition(game_state);
        }
        if game_state.game_ended {
            return;
        }
        tdbg!("CHECK_TIMING:4 victory OK");
        {
            #[cfg(not(feature = "no_std"))]
            let _t = crate::timer::Timer::start("check_timing::check_invalid_live_cards");
            Self::check_invalid_live_cards(game_state, true);
            Self::check_invalid_live_cards(game_state, false);
        }
        tdbg!("CHECK_TIMING:5 invalid live p1 OK");
        Self::check_invalid_energy_cards(&mut game_state.player1, &game_state.card_database);
        Self::check_invalid_energy_cards(&mut game_state.player2, &game_state.card_database);
        tdbg!("CHECK_TIMING:7 invalid energy OK");
        Self::check_orphaned_under_cards(&mut game_state.player1, &game_state.card_database);
        Self::check_orphaned_under_cards(&mut game_state.player2, &game_state.card_database);
        tdbg!("CHECK_TIMING:8 orphaned under OK");
        {
            #[cfg(not(feature = "no_std"))]
            let _t = crate::timer::Timer::start("check_timing::recalculate_constants");
            game_state.recalculate_constants();
        }
        tdbg!("CHECK_TIMING:9 recalc_constants OK");
        Self::check_invalid_resolution_zone(game_state);
        tdbg!("CHECK_TIMING:10 invalid resolution OK");
        tdbg!("CHECK_TIMING:11 perm_loop OK");
        // NOTE: check_victory_condition already ran above; the steps between
        // (invalid-card cleanup, recalculate_constants, resolution-zone check,
        // permanent-loop detection) cannot add success-zone cards, so a second
        // pass would be a no-op.
        let active_player_id = game_state.active_player().id.clone();
        {
            #[cfg(not(feature = "no_std"))]
            let _t = crate::timer::Timer::start("check_timing::process_pending_auto_abilities");
            game_state.process_pending_auto_abilities(&active_player_id);
        }
        tdbg!("CHECK_TIMING:13 auto_abilities OK");
    }

    pub fn check_victory_condition(game_state: &mut GameState) {
        let p1_success_count = game_state.player1.success_live_card_zone.cards.len();
        let p2_success_count = game_state.player2.success_live_card_zone.cards.len();

        // Q54: If 3+ cards end up in success zone simultaneously, game is a draw.
        // Q49: Turn order stays same if no player won. Q50: Same if both placed.
        // Q51: Turn order swaps to the player who placed (if only one did).
        // Q52: Turn order stays same if both had 2+ already and neither could place.
        // Rule 1.2.1.1: Player wins with 3+ cards when opponent has 2- cards
        // Rule 1.2.1.2: Draw if both players have 3+ cards simultaneously
        if p1_success_count >= crate::constants::VICTORY_CARD_COUNT
            && p2_success_count >= crate::constants::VICTORY_CARD_COUNT
        {
            // Both players have 3+ cards - draw
            game_state.game_result = crate::game_state::GameResult::Draw;
            game_state.game_ended = true;
        } else if p1_success_count >= crate::constants::VICTORY_CARD_COUNT && p2_success_count <= 2
        {
            // Player 1 has 3+ cards, player 2 has 2- cards - player 1 wins
            game_state.game_result = if game_state.player1.is_first_attacker {
                crate::game_state::GameResult::FirstAttackerWins
            } else {
                crate::game_state::GameResult::SecondAttackerWins
            };
            game_state.game_ended = true;
        } else if p2_success_count >= crate::constants::VICTORY_CARD_COUNT && p1_success_count <= 2
        {
            // Player 2 has 3+ cards, player 1 has 2- cards - player 2 wins
            game_state.game_result = if game_state.player2.is_first_attacker {
                crate::game_state::GameResult::FirstAttackerWins
            } else {
                crate::game_state::GameResult::SecondAttackerWins
            };
            game_state.game_ended = true;
        }
    }

    // Q88: Players cannot voluntarily discard, retire members, move members,
    // or weigh active cards without an effect or cost.

    /// Rule 10.5.1: Non-live cards in live card zone ↁEmoved to discard.
    /// Also records movement events so turn-level tracking (turn_movements)
    /// captures which cards moved where, enabling "from live_card_zone to
    /// discard" conditions.
    ///
    /// `is_p1` selects the player; the id String is only cloned on the rare
    /// path where a card actually moves (needed for the movement event).
    fn check_invalid_live_cards(game_state: &mut GameState, is_p1: bool) {
        let seat = if is_p1 { 0 } else { 1 };
        if !game_state.seat_player(seat).live_card_zone.face_up {
            return;
        }
        let invalids: Vec<(usize, i16, bool)> = {
            let player = game_state.seat_player(seat);
            player
                .live_card_zone
                .cards
                .iter()
                .enumerate()
                .filter_map(|(i, &card_id)| {
                    let card = game_state.card_database.get_card(card_id)?;
                    if !card.is_live() {
                        Some((i, card_id, card.is_energy()))
                    } else {
                        None
                    }
                })
                .collect()
        };
        if invalids.is_empty() {
            return;
        }
        let player_id = game_state.seat_player(seat).id.clone();
        // Live-zone membership changed ↁEconstant outputs may differ.
        let mut moved = Vec::new();
        for &(i, card_id, is_energy) in invalids.iter().rev() {
            let player = game_state.seat_player_mut(seat);
            if i < player.live_card_zone.cards.len() {
                player.live_card_zone.cards.remove(i);
                if is_energy {
                    player.energy_deck.cards.push(card_id);
                    moved.push((card_id, "energy_deck"));
                } else {
                    player.waitroom.add_card(card_id);
                    moved.push((card_id, "waitroom"));
                }
            }
        }
        for (card_id, dest_zone) in moved {
            game_state.push_movement_event(
                card_id,
                "live_card_zone",
                dest_zone,
                None,
                player_id.as_str(),
                false,
            );
        }
    }

    fn check_duplicate_members(
        player: &mut crate::player::Player,
        card_db: &CardDatabase,
    ) -> usize {
        let mut moved = 0;
        for area in crate::zones::MemberArea::ALL {
            let pending = player.stage.take_pending_duplicate_members(area);
            for (member_id, under_cards) in pending {
                log::debug!(
                    "[DUPLICATE_MEMBER] player={} area={} member={}",
                    player.id,
                    area,
                    member_id
                );
                player.waitroom.add_card(member_id);
                moved += 1;
                for card_id in under_cards {
                    if card_db
                        .get_card(card_id)
                        .is_some_and(|card| card.is_energy())
                    {
                        player.energy_deck.cards.push(card_id);
                    } else {
                        player.waitroom.add_card(card_id);
                    }
                    moved += 1;
                }
            }
        }
        moved
    }

    /// Rule 10.5.2: Non-energy cards in energy zone ↁEmoved to discard.
    fn check_invalid_energy_cards(
        player: &mut crate::player::Player,
        card_db: &CardDatabase,
    ) -> usize {
        let mut invalid_indices = Vec::new();
        for (i, card_id) in player.energy_zone.cards.iter().enumerate() {
            if !card_db.get_card(*card_id).is_some_and(|c| c.is_energy()) {
                invalid_indices.push(i);
            }
        }
        let mut moved = 0;
        for &i in invalid_indices.iter().rev() {
            if i < player.energy_zone.cards.len() {
                let card_id = player.energy_zone.cards.remove(i);
                player.waitroom.add_card(card_id);
                moved += 1;
            }
        }
        moved
    }

    /// Rule 10.5.3-4: Orphaned cards under members.
    /// When a member leaves its area, any member cards under it go to discard (10.5.3)
    /// and any energy cards under it go to energy deck (10.5.4).
    fn check_orphaned_under_cards(
        player: &mut crate::player::Player,
        card_db: &CardDatabase,
    ) -> usize {
        let mut moved = 0;
        for area_idx in 0..3 {
            let top = player.stage.stage[area_idx];
            if top == -1 {
                let under = core::mem::take(&mut player.stage.under_cards[area_idx]);
                for cid in under {
                    if card_db.get_card(cid).is_some_and(|c| c.is_energy()) {
                        player.energy_deck.cards.push(cid);
                    } else {
                        player.waitroom.cards.push(cid);
                    }
                    moved += 1;
                }
            }
        }
        moved
    }

    fn check_invalid_resolution_zone(game_state: &mut GameState) {
        let cards = core::mem::take(&mut game_state.resolution_zone.cards);
        let owners = core::mem::take(&mut game_state.resolution_zone.owners);
        if cards.is_empty() {
            return;
        }
        let active_seat = game_state.seat_index_by_id(&game_state.active_player().id);
        for (index, card_id) in cards.into_iter().enumerate() {
            let owner = owners.get(index).copied().unwrap_or(active_seat);
            let player = if owner == 0 {
                &mut game_state.player1
            } else {
                &mut game_state.player2
            };
            let is_energy = game_state
                .card_database
                .get_card(card_id)
                .is_some_and(|card| card.is_energy());
            log::debug!(
                "[RESOLUTION_CLEANUP] card={} owner={} energy={}",
                card_id,
                owner,
                is_energy
            );
            if is_energy {
                player.energy_deck.cards.push(card_id);
            } else {
                player.waitroom.add_card(card_id);
            }
        }
    }
}
