use super::debug::AbDebug;
#[cfg(not(feature = "no_std"))]
use super::log::{drain_verdicts, push_verdict, AbilityLogItem};

// PSP stubs — no debug logging on console
#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
#[cfg(feature = "no_std")]
#[derive(Clone, Debug, serde::Serialize)]
pub struct AbilityLogItem;
#[cfg(feature = "no_std")]
#[allow(dead_code)]
fn drain_verdicts() -> Vec<AbilityLogItem> {
    Vec::new()
}
#[cfg(feature = "no_std")]
#[allow(dead_code)]
fn push_verdict(_item: AbilityLogItem) {}
#[cfg(feature = "no_std")]
#[allow(dead_code)]
fn drain_verdicts_since(_snapshot: usize) -> Vec<AbilityLogItem> {
    Vec::new()
}

use super::types::{
    AbilityTraceNode, Choice, EffectPipeline, EffectSpawnContext, ExecutionContext, StepState,
    ZoneSnapshot,
};
use super::util;
use crate::card::{Ability, AbilityEffect, CardDatabase, Condition, Keyword};
use crate::game_state::{GameState, Phase};
use crate::types::LogEntry;
use crate::zones::MemberArea;
use crate::Arc;
use crate::HashSet;

#[derive(Clone, Debug)]
pub struct AbilityResolver {
    pub pending_choice: Option<Choice>,
    pub card_database: Arc<CardDatabase>,
    pub duration_effects: Vec<(String, String)>,
    pub current_ability: Option<crate::card::Ability>,
    /// The index of `current_ability` within the card's abilities list.
    /// Stored directly (not read from queue) because the queue's current entry
    /// may change during effect execution (e.g. process_pending_auto_abilities).
    pub current_ability_index: Option<usize>,
    pub activating_card_id: Option<i16>,
    pub execution_context: ExecutionContext,
    pub current_effect: Option<AbilityEffect>,
    pub is_reveal_cost: bool,
    pub selected_cards: Vec<i16>,
    pub selected_area: Option<String>,
    pub moved_cards: Vec<i16>,
    pub spawn_context: EffectSpawnContext,
    pub sub_choice_created: bool,
    /// Snapshot of `selected_cards.len()` taken when a choice is created
    /// by a distinct/target_count action. Used by the saved action to exclude
    /// cards selected BEFORE the choice, without excluding the card selected
    /// BY the choice.
    pub selected_count_at_save: Option<usize>,
    pub pending_stage_cards: Vec<(i16, String)>,
    pub debug_trace: bool,
    pub pipeline: EffectPipeline,
    /// Cross-step data flow machinery — see `StepState` for the per-step
    /// output map, last-draw-count, and looked-at-total-count fields.
    pub step_state: StepState,
    pub pending_energy_payment: Option<u32>,
    /// Binary sub-costs (e.g. change_state self_cost) in a sequential_cost that
    /// were deferred until the choice sub-cost is confirmed by the player.
    /// Paid on confirm, cleared on skip.
    pub pending_deferred_costs: Vec<Box<AbilityEffect>>,
    pub cancel_remaining_commands: bool,
    /// Repeat actions fed one-at-a-time after each iteration completes.
    pub pending_repeat_actions: Vec<Box<AbilityEffect>>,
    /// Re-prompt choice (any_number / re-select) set after pending actions finish.
    pub pending_reprompt_choice: Option<Choice>,
    /// Buffer for structured ability resolution log items.
    pub log_items: Vec<AbilityLogItem>,
    /// Formation change plan: (member_id, chosen_destination) pairs accumulated
    /// across sequential choices.  All swaps execute as a batch at the end.
    pub formation_plan: Vec<(i16, String)>,
}

impl AbilityResolver {
    pub fn new(card_database: Arc<CardDatabase>, activating_card_id: Option<i16>) -> Self {
        AbilityResolver {
            pending_choice: None,
            card_database: card_database.clone(),
            duration_effects: Vec::new(),
            current_ability: None,
            current_ability_index: None,
            activating_card_id,
            execution_context: ExecutionContext::None,
            current_effect: None,
            is_reveal_cost: false,
            selected_cards: Vec::new(),
            selected_area: None,
            moved_cards: Vec::new(),
            spawn_context: EffectSpawnContext::default(),
            sub_choice_created: false,
            selected_count_at_save: None,
            pending_stage_cards: Vec::new(),
            debug_trace: false,
            pipeline: { EffectPipeline::new() },
            step_state: StepState::new(),
            pending_energy_payment: None,
            pending_deferred_costs: Vec::new(),
            cancel_remaining_commands: false,
            pending_repeat_actions: Vec::new(),
            pending_reprompt_choice: None,
            log_items: Vec::new(),
            formation_plan: Vec::new(),
        }
    }

    /// Buffer a log text entry (goes to `rule_log` at flush time).
    #[allow(unused)]
    pub fn buffer_log<E: AsRef<str>>(&mut self, _gs: &GameState, _text: E) {}

    /// Set `pending_choice` and return `Ok(())` in one call.
    /// Replaces the repeated 2-liner:
    ///   `self.pending_choice = Some(c); return Ok(());`
    /// that appears in ~25 handler sites across effects/*.rs.
    #[inline]
    pub fn emit_choice(&mut self, c: Choice) -> Result<(), String> {
        self.pending_choice = Some(c);
        Ok(())
    }

    /// Name of the activating card, or `"<unknown>"` when unavailable.
    /// Used by log and debug helpers to avoid repeated DB look-ups.
    pub fn activating_card_name(&self) -> &str {
        self.activating_card_id
            .and_then(|cid| self.card_database.get_card(cid))
            .map(|c| c.name.as_ref())
            .unwrap_or("<unknown>")
    }

    /// Find matching card indices in a zone, prompt if too many.
    /// Takes &[i16] (read-only — works with Vec, SmallVec, any container).
    /// Returns Ok(Some(indices)) if exact match or fewer.
    /// Returns Ok(None) if too many — sets pending_choice, caller should `return Ok(())`.
    pub fn match_cards_in_zone(
        &mut self,
        cards: &[i16],
        count: usize,
        card_db: &crate::card::CardDatabase,
        card_type: Option<&str>,
        group_name: Option<&str>,
        cost_limit: Option<u32>,
        zone_name: &str,
        _prompt_desc: &str,
    ) -> Result<Option<Vec<usize>>, String> {
        let filter =
            util::filter_from_parts(card_type, group_name, cost_limit, None, None, None, None);
        let idxs = util::matching_indices(cards, card_db, &filter, false);
        if idxs.is_empty() || idxs.len() < count {
            return Err(format!("Not enough cards in {}: need {}", zone_name, count));
        }
        if idxs.len() > count {
            let desc_en = format!(
                "Select {} card(s) to {} for cost",
                count,
                crate::ability::describe::zone_label(Some(zone_name))
            );
            let desc_ja = format!(
                "コストとして{}に置くカードを{}枚選択",
                crate::ability::describe::zone_label(Some(zone_name)),
                count
            );
            self.pending_choice = Some(
                Choice::select_cards(zone_name.to_string(), 0, desc_en, true)
                    .description_ja(Some(desc_ja))
                    .build(),
            );
            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            return Ok(None);
        }
        Ok(Some(idxs.into_iter().rev().take(count).collect()))
    }

    pub fn get_pending_choice(&self) -> Option<&Choice> {
        self.pending_choice.as_ref()
    }

    pub fn can_activate_effect(&self, gs: &mut GameState, effect: &AbilityEffect) -> bool {
        let ctx = super::condition::ConditionContext::with_moved_and_selected(
            gs,
            &self.moved_cards,
            &self.selected_cards,
        );
        let mut dbg = AbDebug::new();
        dbg.effect(effect);

        let cost_already_paid = gs
            .ability_queue
            .current_entry()
            .is_some_and(|e| e.cost_paid);

        if !cost_already_paid {
            if let Some(ref activation_condition) = effect.activation_condition_parsed_any() {
                let mut merged_cond = Box::clone(activation_condition);
                // Merge the effect's position info into the condition so it's checked.
                if merged_cond.get_position().is_none()
                    && merged_cond.get_positions_characters().is_none()
                {
                    if let Some(ref pos) = effect.position_any() {
                        merged_cond.set_position((*pos).clone());
                    } else if let Some(ref act_pos) = effect.activation_position_any() {
                        merged_cond.set_activation_position((*act_pos).to_string());
                    }
                }
                #[cfg(not(feature = "no_std"))]
                let snapshot = crate::ability::log::buffer_len();
                let result = ctx.evaluate_condition(&merged_cond);
                // On success: drain pre-check verdicts (condition will be re-evaluated
                // during effect execution, avoiding duplicates).
                // On failure: keep verdicts (they're the only info for the failure path).
                #[cfg(not(feature = "no_std"))]
                {
                    if result {
                        let _pre_check_verdicts =
                            crate::ability::log::drain_verdicts_since(snapshot);
                    }
                }
                return result;
            }
        }
        if let Some(ref condition) = effect.condition {
            if effect.action == crate::ability::enums::ActionType::ConditionalAlternative {
                // skip — condition is a branch selector, not a gate
            } else {
                // Check cache first — avoids re-evaluation against stale state
                // (e.g. revealed_cards modified by a prior select_cards filter).
                if condition.get_cache().unwrap_or(false) {
                    if let Some(entry) = gs.ability_queue.current_entry() {
                        if let Some(text) = condition.get_text() {
                            if let Some(cached) = entry.condition_cache.get(text) {
                                if *cached {
                                    return true;
                                }
                                return false;
                            }
                        }
                    }
                }
                let mut cond = condition.clone();
                if cond.get_position().is_none() && cond.get_positions_characters().is_none() {
                    if let Some(ref pos) = effect.position_any() {
                        cond.set_position((*pos).clone());
                    } else if let Some(ref act_pos) = effect.activation_position_any() {
                        cond.set_activation_position((*act_pos).to_string());
                    }
                }
                // Merge effect-level group_names into conditions that need
                // group filtering: AppearanceCondition (group appeared check)
                // and conditions with distinct (name distinctness within group).
                // Recurse into compound sub-conditions with the same logic.
                fn merge_group_names(cond: &mut Condition, group_names: Option<&Vec<String>>) {
                    let needs_group = cond.condition_type()
                        == Some(crate::ability::enums::ConditionType::AppearanceCondition)
                        || cond.get_distinct().is_some_and(|d| d.is_distinct());
                    if needs_group
                        && (cond.get_group_names().is_none()
                            || cond.get_group_names().is_some_and(|v| v.is_empty()))
                    {
                        if let Some(gns) = group_names {
                            if !gns.is_empty() {
                                cond.set_group_names(gns.clone());
                            }
                        }
                    }
                    if let Some(ref mut sub_conds) = cond.get_conditions_mut() {
                        for sub in sub_conds.iter_mut() {
                            merge_group_names(sub, group_names);
                        }
                    }
                }
                let gns_binding = effect.group_names_any();
                let gns = gns_binding.as_ref();
                merge_group_names(&mut cond, gns.map(|v| &**v));
                #[cfg(not(feature = "no_std"))]
                let cond_snapshot = crate::ability::log::buffer_len();
                let passed = ctx.evaluate_condition(&cond);
                // On success: drain (will be re-evaluated during execution).
                // On failure: keep verdicts.
                #[cfg(not(feature = "no_std"))]
                {
                    if passed {
                        let _pre_check_verdicts2 =
                            crate::ability::log::drain_verdicts_since(cond_snapshot);
                    }
                }
                // Cache the result if the condition asks for it
                if condition.get_cache().unwrap_or(false) {
                    if let Some(entry) = gs.ability_queue.current_entry_mut() {
                        if let Some(text) = condition.get_text() {
                            entry.condition_cache.insert(text.to_string(), passed);
                        }
                    }
                }
                if !passed {
                    log::debug!("[CAN_ACTIVATE] condition FAILED for {}: type={:?} location={:?} group={:?} exclude={:?}",
                        effect.action, condition.condition_type(), condition.get_location(), condition.get_group_names(), condition.get_exclude_characters());
                    return false;
                }
            }
        }
        // Q240: Standalone activation_position check for effects with no condition.
        // When an effect has activation_position but no condition field, the merge
        // paths above never fire. Check the position directly here.
        if effect.condition.is_none() {
            if let Some(ref act_pos) = effect.activation_position_any() {
                let card_id = gs.activating_card;
                let player = gs.resolve_target_player("self");
                let passes = act_pos.split(',').any(|p| {
                    let idx = match p.trim() {
                        "left" | "left_side" => 0,
                        "center" => 1,
                        "right" | "right_side" => 2,
                        _ => return false,
                    };
                    idx < player.stage.stage.len()
                        && card_id.is_some()
                        && player.stage.stage[idx] == card_id.unwrap()
                });
                if !passes {
                    log::debug!(
                        "[CAN_ACTIVATE] activation_position {:?} failed for {:?}",
                        act_pos,
                        card_id
                    );
                    return false;
                }
            }
        }
        true
    }

    pub fn check_keywords(
        &self,
        gs: &mut GameState,
        keywords: &[Keyword],
        card_position: Option<MemberArea>,
    ) -> bool {
        for keyword in keywords {
            match keyword {
                Keyword::Center => {
                    if card_position != Some(MemberArea::Center) {
                        return false;
                    }
                }
                Keyword::LeftSide => {
                    if card_position != Some(MemberArea::LeftSide) {
                        return false;
                    }
                }
                Keyword::RightSide => {
                    if card_position != Some(MemberArea::RightSide) {
                        return false;
                    }
                }
                Keyword::Turn1 => {
                    if gs.turn_number != 1 {
                        return false;
                    }
                }
                Keyword::Turn2 => {
                    if gs.turn_number != 2 {
                        return false;
                    }
                }
                Keyword::Debut => {
                    if let Some(pos) = card_position {
                        let master = gs.ability_master_id();
                        let stage = if master.as_deref() == Some("player2")
                            || master.as_deref() == Some("p2")
                        {
                            &gs.player2.stage.stage
                        } else {
                            &gs.player1.stage.stage
                        };
                        let card_id = match pos {
                            MemberArea::Center => stage[1],
                            MemberArea::LeftSide => stage[0],
                            MemberArea::RightSide => stage[2],
                        };
                        if card_id == -1 {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                Keyword::LiveStart => {
                    if !matches!(
                        gs.current_phase,
                        Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker
                    ) {
                        return false;
                    }
                }
                Keyword::LiveSuccess => {
                    if !matches!(gs.current_phase, Phase::LiveVictoryDetermination) {
                        return false;
                    }
                }
                Keyword::PositionChange => {
                    return gs.position_change_occurred_this_turn;
                }
                Keyword::FormationChange => {
                    return gs.formation_change_occurred_this_turn;
                }
            }
        }
        true
    }

    pub(crate) fn store_pending_choice(&mut self, gs: &mut GameState) {
        gs.ability_queue.snapshot_requested = true;
        if let Some(ref choice) = self.pending_choice {
            // Always-on debug: log every pending choice (ABILITY_DEBUG is set true in tests)
            if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
                match choice {
                    crate::ability::types::Choice::SelectCard {
                        zone,
                        card_type,
                        count,
                        description,
                        allow_skip,
                        group,
                        is_select_action,
                        heart_colors,
                        target_player_id,
                        ..
                    } => {
                        log::debug!("[PENDING_CHOICE] SelectCard zone={} count={} allow_skip={} group={:?} card_type={:?} is_select_action={} heart_colors={:?} target_player_id={:?} description={}", zone, count, allow_skip, group, card_type, is_select_action, heart_colors, target_player_id, description);
                    }
                    crate::ability::types::Choice::SelectHeartColor {
                        count,
                        options,
                        description,
                        ..
                    } => {
                        log::debug!(
                        "[PENDING_CHOICE] SelectHeartColor count={} options={:?} description={}",
                        count, options, description
                    );
                    }
                    crate::ability::types::Choice::SelectTarget {
                        target,
                        description,
                        options,
                        allow_skip,
                        ..
                    } => {
                        log::debug!("[PENDING_CHOICE] SelectTarget target={} options={:?} allow_skip={} description={}", target, options, allow_skip, description);
                    }
                    crate::ability::types::Choice::SelectPosition { description, .. } => {
                        log::debug!(
                            "[PENDING_CHOICE] SelectPosition description={}",
                            description
                        );
                    }
                    crate::ability::types::Choice::SelectHeartType {
                        count,
                        options,
                        description,
                        ..
                    } => {
                        log::debug!(
                            "[PENDING_CHOICE] SelectHeartType count={} options={:?} description={}",
                            count,
                            options,
                            description
                        );
                    }
                    crate::ability::types::Choice::SelectAutoAbility { options, .. } => {
                        log::debug!("[PENDING_CHOICE] SelectAutoAbility options={:?}", options);
                    }
                    crate::ability::types::Choice::SelectLiveSuccess { description, .. } => {
                        log::debug!(
                            "[PENDING_CHOICE] SelectLiveSuccess description={}",
                            description
                        );
                    }
                }
            }
            let mut json = choice.to_frontend_json();
            if let Some(ref mut j) = json {
                if let Some(entry) = gs.ability_queue.current_entry() {
                    if let Some(ref effect) = entry.ability.effect {
                        if let Some(ref maker) = effect.choice_maker_any() {
                            if let Some(obj) = j.as_object_mut() {
                                obj.insert(
                                    "choice_maker".to_string(),
                                    serde_json::Value::String(maker.to_string()),
                                );
                            }
                        }
                    }
                }
                gs.inject_choice_ability_context(j);
            }
        }
    }

    fn zone_for_card(gs: &GameState, card_id: i16) -> String {
        for player in [&gs.player1, &gs.player2] {
            if player.stage.stage.contains(&card_id) {
                return "stage".to_string();
            }
            if player.live_card_zone.cards.contains(&card_id) {
                return "live_card_zone".to_string();
            }
            if player.success_live_card_zone.cards.contains(&card_id) {
                return "success_live_card_zone".to_string();
            }
            if player.hand.cards.contains(&card_id) {
                return "hand".to_string();
            }
            if player.waitroom.cards.contains(&card_id) {
                return "waitroom".to_string();
            }
            if player.energy_zone.cards.contains(&card_id) {
                return "energy_zone".to_string();
            }
        }
        "?".to_string()
    }

    /// Push a structured ability_resolution log entry with the given result and items.
    fn push_ability_result(
        &self,
        gs: &mut GameState,
        result: &str,
        items: Vec<AbilityLogItem>,
        error: Option<&str>,
    ) {
        let pp = gs.player_prefix();
        let card_id = gs.activating_card;
        let card_name = card_id
            .and_then(|id| gs.card_database.get_card(id))
            .map(|c| c.name.to_string())
            .unwrap_or_default();
        let raw_trigger = self
            .current_ability
            .as_ref()
            .and_then(|a| a.triggers.as_deref())
            .unwrap_or("?");
        // Normalize trigger string to match trigger_evaluation metadata format.
        // ability.triggers is the raw text from cards.json (e.g. "登場", "live_success"),
        // but the trigger_evaluation entry stores a canonical English key (e.g. "debut", "live_success").
        let trigger_str = match raw_trigger {
            s if s.contains(crate::triggers::DEBUT) || s.contains(crate::triggers::DEBUT_EN) => {
                "debut"
            }
            s if s.contains(crate::triggers::LIVE_START) => "live_start",
            s if s.contains(crate::triggers::LIVE_SUCCESS)
                || s.contains(crate::triggers::LIVE_SUCCESS_EN) =>
            {
                "live_success"
            }
            s if s.contains(crate::triggers::ACTIVATION) => "activation",
            s if s.contains(crate::triggers::CONSTANT) => "constant",
            s if s.contains(crate::triggers::AUTO) => "auto",
            _ => raw_trigger,
        };
        let ability_text = self
            .current_ability
            .as_ref()
            .map(|a| a.full_text.clone())
            .unwrap_or_default();
        let zone = card_id
            .map(|cid| Self::zone_for_card(gs, cid))
            .unwrap_or_default();
        let items_json: Vec<serde_json::Value> = items
            .iter()
            .map(|i| serde_json::to_value(i).unwrap_or_default())
            .collect();
        let meta = crate::core::types::LogMetadata::AbilityResolution {
            result: result.to_string(),
            items: items_json.clone(),
            ability_text: ability_text.clone(),
            zone: zone.clone(),
            error: error.map(|e| e.to_string()),
            resolved: None,
        };
        let log_text = format!(
            "{pp} {card_name} [{zone}]: [[log_ability_result:trigger=trigger_{trigger_str},result=result_{}]]",
            result
        );
        gs.push_rule_log(log_text.clone());
        if !crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            return;
        }
        gs.push_structured_log(LogEntry {
            text: log_text,
            turn: gs.turn_number,
            player_label: pp.clone(),
            source_card_id: card_id,
            source_card_name: Some(card_name),
            category: "ability_resolution".to_string(),
            metadata: Some(meta),
        });

        // Update the matching trigger_evaluation entry with the resolution result.
        // Match on (source_card_id, turn, ability_index, trigger_str) to distinguish
        // multiple abilities on the same card with the same trigger type.
        // Use the resolver's stored index (not the queue's current entry) because
        // the queue entry may have changed during effect execution.
        let ability_index = self.current_ability_index;
        if let Some(cid) = card_id {
            for entry in gs.structured_log.iter_mut().rev() {
                if entry.category != "trigger_evaluation" {
                    continue;
                }
                if entry.source_card_id != Some(cid) {
                    continue;
                }
                if entry.turn != gs.turn_number {
                    continue;
                }
                let trigger_match = match entry.metadata.as_ref() {
                    Some(crate::core::types::LogMetadata::TriggerEvaluation {
                        trigger, ..
                    }) => trigger == trigger_str,
                    _ => false,
                };
                if !trigger_match {
                    continue;
                }
                let eval_idx = match entry.metadata.as_ref() {
                    Some(crate::core::types::LogMetadata::TriggerEvaluation {
                        ability_index,
                        ..
                    }) => Some(*ability_index),
                    _ => None,
                };
                if let Some(ai) = ability_index {
                    if let Some(ei) = eval_idx {
                        if ai != ei {
                            continue;
                        }
                    }
                }
                // Found the matching entry — update its metadata
                if let Some(ref mut meta) = entry.metadata {
                    *meta = crate::core::types::LogMetadata::AbilityResolution {
                        result: result.to_string(),
                        items: items_json.clone(),
                        ability_text: ability_text.clone(),
                        zone: String::new(),
                        error: None,
                        resolved: Some(true),
                    };
                }
                break;
            }
        }
    }

    pub fn resolve_ability(
        &mut self,
        gs: &mut GameState,
        ability: &Ability,
        activating_card: Option<i16>,
        ability_index: usize,
    ) -> Result<(), String> {
        let mut dbg = AbDebug::new();
        // Clear structured verdict buffer from any previous ability
        #[cfg(not(feature = "no_std"))]
        crate::ability::log::clear_verdicts();

        // Card info for debug (owned Strings to avoid borrowing gs across mutation)
        let card_data = activating_card.and_then(|id| gs.card_database.get_card(id));
        let card_name = card_data.map(|c| c.name.to_string()).unwrap_or_default();
        let card_no = card_data.map(|c| c.card_no.to_string()).unwrap_or_default();
        let card_id_str = activating_card.map(|id| id.to_string()).unwrap_or_default();

        // Initialize root trace node with ability information
        if self.debug_trace {
            self.pipeline.trace.label = format!(
                "ability[{}]: {}",
                ability_index,
                ability.full_text.chars().take(60).collect::<String>()
            );
            self.pipeline.trace.card = Some(card_name.to_string());
            self.pipeline.trace.before = Some(ZoneSnapshot::from_game_state(gs));
        }

        dbg.ability(&card_name, &card_no, &card_id_str, ability);

        // Check use_limit before cost, but don't insert until after effect runs
        let ability_key = activating_card.map(|card_id| (card_id, ability_index, gs.turn_number));

        // Set these early so push_ability_result can access them on early exits
        self.current_ability = Some(ability.clone());
        self.current_ability_index = Some(ability_index);
        gs.activating_card = activating_card;

        if let Some(ref key) = ability_key {
            if let Some(use_limit) = ability.use_limit {
                let used = gs
                    .turn_limited_abilities_used
                    .get(key)
                    .copied()
                    .unwrap_or(0);
                if u32::from(used) >= use_limit {
                    let msg = format!(
                        "Ability already used {} of {} times this turn",
                        used, use_limit
                    );
                    dbg.p("RESULT", &msg);
                    let items = drain_verdicts();
                    self.push_ability_result(gs, "skipped", items, Some(&msg));
                    return Err(msg);
                }
            }
        }

        // Check activation keywords (center/left/right/turn position restrictions)
        if let Some(card_id) = activating_card {
            let position = gs.find_card_stage_position(card_id);
            if !self.check_keywords(gs, ability.keywords.as_ref().unwrap_or(&vec![]), position) {
                let items = drain_verdicts();
                self.push_ability_result(
                    gs,
                    "position_fail",
                    items,
                    Some("Activation keywords not satisfied"),
                );
                return Err(
                    "Activation keywords not satisfied (e.g. card not at required position)"
                        .to_string(),
                );
            }
        }

        let cost_already_paid = gs
            .ability_queue
            .current_entry()
            .is_some_and(|e| e.cost_paid);

        if !cost_already_paid {
            if let Some(ref cost) = ability.cost {
                let cost = self.apply_modify_cost_to_ability_cost(gs, cost, ability);
                if let Err(e) = self.pay_cost(gs, &cost) {
                    dbg.p("RESULT", format_args!("COST FAILED: {}", e));
                    let items = drain_verdicts();
                    self.push_ability_result(gs, "cost_fail", items, Some(&e));
                    return Err(e);
                }
                dbg.p("RESULT", "cost paid ✓");
                #[cfg(not(feature = "no_std"))]
                let cost_desc = format!(
                    "{}: {}→{} {}",
                    cost.action,
                    cost.source.as_deref().unwrap_or("?"),
                    cost.destination.as_deref().unwrap_or("?"),
                    cost.count.unwrap_or(cost.energy_count_any().unwrap_or(1))
                );
                #[cfg(not(feature = "no_std"))]
                push_verdict(AbilityLogItem::Cost {
                    text: cost.text.clone(),
                    expectation: cost_desc,
                    actual: "支払済".into(),
                    passed: true,
                    optional: cost.optional.unwrap_or(false),
                });
            }
        }

        // Record use_limit early only if the effect's conditions can be met
        // AND it's not a conditional_on_optional (the optional cost hasn't been
        // decided yet — record after the choice if the player pays).
        // This prevents consuming use_limit on premature triggers (e.g. auto
        // abilities queued eagerly before their trigger condition is satisfied).
        let is_conditional_optional = ability
            .effect
            .as_ref()
            .is_some_and(|e| e.action == crate::ability::enums::ActionType::ConditionalOnOptional);
        let is_optional_effect = ability
            .effect
            .as_ref()
            .is_some_and(|e| e.optional.unwrap_or(false));
        // For optional effects (may place / may use), the use_limit is consumed
        // by the EFFECT execution, not the trigger. If the player skips the
        // optional effect later, the key is never inserted.
        if !cost_already_paid
            && self.pending_choice.is_none()
            && !is_conditional_optional
            && !is_optional_effect
        {
            if let Some(ref key) = ability_key {
                if ability.use_limit.is_some() {
                    let can_activate = ability
                        .effect
                        .as_ref()
                        .is_none_or(|e| self.can_activate_effect(gs, e));
                    if can_activate {
                        *gs.turn_limited_abilities_used.entry(*key).or_insert(0) += 1;
                    }
                }
            }
        }

        if self.pending_choice.is_some() {
            if !cost_already_paid {
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.cost_paid = true;
                }
            }
            self.store_pending_choice(gs);
            return Ok(());
        }

        // Check position keywords (Center/LeftSide/RightSide) AFTER cost payment.
        // Position checks gate the effect, not the cost — the test expects cost to still be paid.
        if let Some(card_id) = activating_card {
            let position = gs
                .player1
                .stage
                .stage
                .iter()
                .position(|&id| id == card_id)
                .or_else(|| gs.player2.stage.stage.iter().position(|&id| id == card_id))
                .map(|idx| match idx {
                    0 => crate::zones::MemberArea::LeftSide,
                    1 => crate::zones::MemberArea::Center,
                    _ => crate::zones::MemberArea::RightSide,
                });
            if let Some(ref kws) = ability.keywords {
                for kw in kws {
                    let pos_ok = match kw {
                        Keyword::Center => position == Some(crate::zones::MemberArea::Center),
                        Keyword::LeftSide => position == Some(crate::zones::MemberArea::LeftSide),
                        Keyword::RightSide => position == Some(crate::zones::MemberArea::RightSide),
                        _ => true,
                    };
                    if !pos_ok {
                        // Suppress position condition failures for auto abilities
                        // to avoid noise (they fire on every phase transition).
                        if ability.triggers.as_deref() != Some(crate::triggers::ACTIVATION)
                            && ability.triggers.as_deref() != Some(crate::triggers::DEBUT)
                        {
                            let pp2 = gs.player_prefix();
                            gs.push_rule_log(format!(
                                "{pp2} {card_name}: [[log_position_fail:keyword={kw:?}]]"
                            ));
                        }
                        dbg.p("RESULT", "position requirement not met — effect skipped");
                        let items = drain_verdicts();
                        self.push_ability_result(gs, "position_fail", items, None);
                        return Ok(());
                    }
                }
            }
        }

        // Mark cost as paid when it auto-resolved without creating a pending choice.
        if !cost_already_paid && ability.cost.is_some() && self.pending_choice.is_none() {
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.cost_paid = true;
            }
        }

        // When an optional cost was skipped (no eligible cards or player declined),
        // the primary effect should not run. Handles colon-gated patterns like
        // "may discard X: gain Y" where the colon gates the effect.
        let cost_was_skipped = gs
            .ability_queue
            .current_entry()
            .is_some_and(|e| e.optional_cost_result == Some(false));
        log::debug!(
            "[KANAN_DEBUG] cost_was_skipped={} optional_cost_result={:?}",
            cost_was_skipped,
            gs.ability_queue
                .current_entry()
                .and_then(|e| e.optional_cost_result)
        );
        if cost_was_skipped {
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.effect_started = true;
            }
            dbg.p("RESULT", "optional cost skipped — effect not executed");
            let items = drain_verdicts();
            self.push_ability_result(gs, "skipped", items, Some("optional cost not paid"));
            return Ok(());
        }

        if let Some(ref effect) = ability.effect {
            // Check the effect's condition BEFORE executing. The condition must
            // be met in the current game state (after cost payment). This prevents
            // effects like "choice" from being shown when the condition fails.
            if effect.condition.is_some() || effect.activation_condition_parsed_any().is_some() {
                let passed = self.can_activate_effect(gs, effect);
                if !passed {
                    // For 起動 (activation) abilities, the player deliberately paid the
                    // cost, so the attempt counts toward the turn limit even when the
                    // effect's condition isn't met.  AUTO-triggered abilities preserve
                    // their use_limit for when the board state actually satisfies the
                    // condition.
                    if ability.use_limit.is_some()
                        && ability.triggers.as_deref() == Some(crate::triggers::ACTIVATION)
                    {
                        if let Some(ref key) = ability_key {
                            *gs.turn_limited_abilities_used.entry(*key).or_insert(0) += 1;
                        }
                    }
                    dbg.p("RESULT", "effect condition not met — skipped");
                    let items = drain_verdicts();
                    self.push_ability_result(gs, "failure", items, None);
                    return Ok(());
                }
            }
            if let Err(e) = self.execute_effect(gs, effect) {
                dbg.p("RESULT", format_args!("EFFECT FAILED: {}", e));
                let items = drain_verdicts();
                self.push_ability_result(gs, "failure", items, Some(&e));
                return Err(e);
            }
            log::debug!(
                "[AFTER_EXEC] pending={:?} action={:?}",
                self.pending_choice.is_some(),
                effect.action
            );
            if self.pending_choice.is_some() {
                if !cost_already_paid {
                    if let Some(entry) = gs.ability_queue.current_entry_mut() {
                        entry.cost_paid = true;
                    }
                    // Don't record use_limit yet when the pending choice is:
                    // - "conditional_optional" (may pay) — player decides at choice
                    // - "position|destination" for optional effects (may place)
                    //   Record after the choice resolves if they actually did it.
                    let is_optional_pos = matches!(
                        self.pending_choice,
                        Some(Choice::SelectTarget { ref target, .. })
                            if target == "position|destination"
                    ) && ability
                        .effect
                        .as_ref()
                        .is_some_and(|e| e.optional.unwrap_or(false));
                    let skip_use_limit = matches!(
                        self.pending_choice,
                        Some(Choice::SelectTarget { ref target, .. })
                            if target == "conditional_optional"
                    ) || is_optional_pos;
                    if let Some(ref key) = ability_key {
                        if ability.use_limit.is_some() && !skip_use_limit {
                            *gs.turn_limited_abilities_used.entry(*key).or_insert(0) += 1;
                        }
                    }
                }
                // Mark effect as started when a pending choice comes from effect
                // execution (not cost). This prevents RWC from re-entering the
                // ability after the effect's choice resolves.
                let is_paid = cost_already_paid
                    || gs
                        .ability_queue
                        .current_entry()
                        .is_some_and(|e| e.cost_paid);
                if is_paid {
                    if let Some(entry) = gs.ability_queue.current_entry_mut() {
                        entry.effect_started = true;
                    }
                }
                self.store_pending_choice(gs);
                return Ok(());
            }
            dbg.p("RESULT", "effect applied ✓");
            let items = drain_verdicts();
            self.push_ability_result(gs, "success", items, None);
        }

        if !cost_already_paid {
            if let Some(ref key) = ability_key {
                if ability.use_limit.is_some() {
                    let can_activate = ability
                        .effect
                        .as_ref()
                        .is_none_or(|e| self.can_activate_effect(gs, e));
                    if can_activate {
                        *gs.turn_limited_abilities_used.entry(*key).or_insert(0) += 1;
                    }
                }
            }
        }
        if let Some(key) = ability_key {
            if ability.use_limit.is_some() {
                // When cost is already paid (e.g. conditional_on_optional's second entry
                // after the player accepted), the effect already ran and may have moved
                // cards around.  Re-checking can_activate_effect would see stale state
                // (the resolver's moved_cards includes cards moved BY this effect), so
                // the condition can spuriously fail and the key never gets inserted.
                // Skip the can_activate_effect guard when cost is already paid AND the
                // ability is a conditional_on_optional (the player already accepted).
                let is_cond_opt = ability.effect.as_ref().is_some_and(|e| {
                    e.action == crate::ability::enums::ActionType::ConditionalOnOptional
                });
                if (cost_already_paid && is_cond_opt)
                    || ability
                        .effect
                        .as_ref()
                        .is_none_or(|e| self.can_activate_effect(gs, e))
                {
                    *gs.turn_limited_abilities_used.entry(key).or_insert(0) += 1;
                }
            }
        }

        gs.activating_card = None;
        self.current_ability = None;
        self.current_ability_index = None;

        if self.debug_trace {
            self.pipeline.trace.after = Some(ZoneSnapshot::from_game_state(gs));
        }

        Ok(())
    }

    /// Record the start of an effect execution to the trace.
    pub fn trace_effect_start(
        &mut self,
        gs: &GameState,
        effect_name: &str,
        card_name: Option<String>,
    ) {
        if !self.debug_trace {
            return;
        }
        let before = ZoneSnapshot::from_game_state(gs);
        let node = AbilityTraceNode::new(effect_name)
            .with_card(card_name)
            .with_before(before);
        self.pipeline.trace.add_child(node);
    }

    /// Record the end of an effect execution (update after state in the last trace node).
    pub fn trace_effect_end(&mut self, gs: &GameState) {
        if !self.debug_trace {
            return;
        }
        let after = ZoneSnapshot::from_game_state(gs);
        if let Some(last_child) = self.pipeline.trace.children.last_mut() {
            last_child.after = Some(after);
        }
    }

    /// Get a reference to the current trace node for adding details.
    pub fn get_trace_node(&mut self) -> &mut AbilityTraceNode {
        &mut self.pipeline.trace
    }

    pub fn card_matches_type(
        &self,
        gs: &mut GameState,
        card_id: i16,
        card_type_filter: Option<&str>,
    ) -> bool {
        util::card_matches_type(&gs.card_database, card_id, card_type_filter)
    }

    pub fn card_matches_group(
        &self,
        gs: &mut GameState,
        card_id: i16,
        group_filter: Option<&String>,
    ) -> bool {
        util::card_matches_group(&gs.card_database, card_id, group_filter)
    }

    pub fn card_matches_cost_limit(
        &self,
        gs: &mut GameState,
        card_id: i16,
        cost_limit: Option<u32>,
    ) -> bool {
        util::card_matches_cost_limit(&gs.card_database, card_id, cost_limit)
    }

    /// Walk the ability's effect tree to find modify_cost sub-actions and adjust the cost.
    /// Handles patterns like "コストはグループ名1種類につきE減る" (cost reduced per group name).
    fn apply_modify_cost_to_ability_cost(
        &self,
        gs: &mut GameState,
        cost: &AbilityEffect,
        ability: &Ability,
    ) -> AbilityEffect {
        let mut cost = cost.clone();
        if let Some(ref effect) = ability.effect {
            if let Some(mod_cost) = util::find_modify_cost(effect, None, None) {
                if mod_cost.operation_any().as_deref() == Some("subtract")
                    && mod_cost.per_unit_any().unwrap_or(false)
                    && mod_cost.per_unit_type_any().as_deref() == Some("group_name")
                {
                    // Count distinct group names on self's stage
                    let player = gs.resolve_target_player("self");
                    let card_db = &gs.card_database;
                    let mut groups = HashSet::<String>::default();
                    for &cid in &player.stage.stage {
                        if cid == -1 {
                            continue;
                        }
                        if let Some(card) = card_db.get_card(cid) {
                            if !card.group.is_empty() {
                                groups.insert(card.group.to_string());
                            }
                        }
                    }
                    let per_unit_count = mod_cost.per_unit_count_any().unwrap_or(1);
                    let reduction =
                        (groups.len() as u32 / per_unit_count) * mod_cost.count.unwrap_or(1);
                    if cost.action == crate::ability::enums::ActionType::PayEnergy {
                        let new_energy = cost
                            .energy_count_any()
                            .unwrap_or(0)
                            .saturating_sub(reduction);
                        cost.set_energy_count(Some(new_energy));
                    }
                }
            }
        }
        cost
    }

    pub fn card_db(&self) -> Arc<CardDatabase> {
        self.card_database.clone()
    }

    pub fn fmt_card(&self, cid: i16) -> String {
        self.card_database
            .get_card(cid)
            .map(|c| c.name.as_ref())
            .unwrap_or("?")
            .to_string()
    }

    pub fn fmt_ids(&self, ids: &[i16]) -> String {
        if ids.is_empty() {
            "[]".into()
        } else {
            ids.iter()
                .map(|&id| self.fmt_card(id))
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
}
