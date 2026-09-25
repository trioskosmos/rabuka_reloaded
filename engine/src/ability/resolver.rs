use super::debug::AbDebug;
#[cfg(not(feature = "no_std"))]
use super::log::{drain_verdicts, push_verdict, AbilityLogItem};
use smallvec::SmallVec;

// PSP stubs — no debug logging on console
#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
#[cfg(feature = "no_std")]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_support", derive(serde::Serialize))]
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

use super::gates::{
    pre_cost_gates, post_cost_gates, effect_gates, record_use_limit,
    handle_pending_choice, use_limit_gate, activation_keywords_gate,
    UseLimitPhase,
};
use super::types::{
    Choice, EffectPipeline, EffectSpawnContext, ExecutionContext, StepState, ZoneSnapshot,
};
use super::util;
use crate::card::{Ability, AbilityEffect, CardDatabase, Condition, Keyword};
use crate::game_state::{GameState, Phase};
use crate::types::LogEntry;
use crate::zones::MemberArea;
use crate::Arc;

/// Collapse a set of offered option labels + skip flag into a single signature
/// string, used to detect "the same offer re-presented" and dedup `choice_offered`
/// structured entries.
fn choice_offer_sig(offered: &[String], skip_allowed: bool) -> String {
    #[cfg(feature = "no_std")]
    use core::fmt::Write;
    #[cfg(not(feature = "no_std"))]
    use std::fmt::Write;
    let mut sig = String::new();
    let _ = write!(sig, "skip={};", skip_allowed);
    for (i, o) in offered.iter().enumerate() {
        let _ = write!(sig, "[{}]{}", i, o);
    }
    sig
}

/// The ability execution engine. ONE resolver instance drives a whole game;
/// per-ability scratch state below is reset by `resolve_ability` phases
/// (resolver.rs) so concurrent/queued abilities never see stale data.
///
/// Data flow (see `resolve_ability` → `execute_effect` in effects/mod.rs):
///   queue entry → gates (use-limit, keywords, cost) → `execute_effect`
///     → ActionType match → per-action handler (move_cards.rs, cost.rs,
///       look.rs, effects/*, compound.rs for Sequential/Conditional*)
///     → pending `Choice` (choice.rs builds, `answer_choice` resumes)
///     → movement/results recorded on `GameState` (mods, queues, logs).
///
/// Cross-step communication rides on `GameState` (moved/selected cards,
/// revealed pools, queue entries), never on globals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LastKnownMemberInfo {
    pub blades: u8,
    pub area: MemberArea,
}

#[derive(Clone, Debug)]
pub struct AbilityResolver {
    pub pending_choice: Option<Choice>,
    pub card_database: Arc<CardDatabase>,
    pub duration_effects: SmallVec<[(String, String); 2]>,
    pub current_ability: Option<crate::card::Ability>,
    /// The index of `current_ability` within the card's abilities list.
    /// Stored directly (not read from queue) because the queue's current entry
    /// may change during effect execution (e.g. process_pending_auto_abilities).
    pub current_ability_index: Option<usize>,
    pub activating_card_id: Option<i16>,
    pub execution_context: ExecutionContext,
    pub current_effect: Option<AbilityEffect>,
    /// Parent effect that created a nested choice (e.g. look_and_select with or_card_types).
    /// Preserved so choice handlers can access or_card_types after sub-effects overwrite current_effect.
    pub parent_effect: Option<AbilityEffect>,
    pub is_reveal_cost: bool,
    pub selected_cards: SmallVec<[i16; 4]>,
    /// Member card IDs actually changed state by the most recent change_state
    /// member-op step of THIS ability (e.g. members waited). Used by a
    /// following delayed restriction step ("そのメンバーは次のターンの…アクティブ
    /// しない") to key the flag on the waited victims instead of falling back
    /// to recently_moved_cards/activating_card.
    pub changed_state_members: SmallVec<[i16; 4]>,
    pub selected_area: Option<String>,
    pub moved_cards: SmallVec<[i16; 4]>,
    /// C6 keep-N-shuffle-rest: phase 0=idle, 1=awaiting self's hand selection,
    /// 2=awaiting opponent's hand selection. Snapshots hold each player's hand
    /// at selection time so the non-selected cards can be moved under the deck.
    pub keep_shuffle_under_phase: u8,
    pub keep_shuffle_under_count: u8,
    pub keep_shuffle_under_snapshots: SmallVec<[Vec<i16>; 2]>,
    /// The hand POSITIONS each player chose to KEEP (per keep-shuffle phase).
    /// Stored as positions, not card ids, because a hand can hold multiple
    /// copies of the same card id (e.g. test fillers).
    pub keep_shuffle_selected: SmallVec<[u8; 8]>,
    pub spawn_context: EffectSpawnContext,
    pub sub_choice_created: bool,
    /// Set when a parent-conditional (そうした場合) sequential defers on a
    /// choice whose outcome decides the gate. The choice answer handler
    /// consumes it: empty/skip answer drops the remaining actions; a real
    /// selection lets them run.
    pub deferred_conditional_gate: bool,
    /// Snapshot of `selected_cards.len()` taken when a choice is created
    /// by a distinct/target_count action. Used by the saved action to exclude
    /// cards selected BEFORE the choice, without excluding the card selected
    /// BY the choice.
    pub selected_count_at_save: Option<u8>,
    pub pending_stage_cards: SmallVec<[(i16, String); 2]>,
    pub debug_trace: bool,
    pub pipeline: EffectPipeline,
    /// Cross-step data flow machinery — see `StepState` for the per-step
    /// output map, last-draw-count, and looked-at-total-count fields.
    pub step_state: StepState,
    pub pending_energy_payment: Option<u8>,
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
    /// Tracks whether the most recent "those_cards" move actually moved any
    /// card. Used by the "…したとき" (when you do so) pattern: a
    /// `modify_score` step directly following a `those_cards`→hand move must
    /// only apply when the move actually added a card.
    pub last_move_moved_any: Option<bool>,
    pub last_action_result: Option<(crate::ability::enums::ActionType, bool)>,
    /// Formation change plan: (member_id, chosen_destination) pairs accumulated
    /// across sequential choices.  All swaps execute as a batch at the end.
    pub formation_plan: SmallVec<[(i16, String); 2]>,
    /// Signature of the last `choice_offered` structured entry emitted, so the
    /// same pending choice re-stored (re-prompt, auto-ability interleave) is not
    /// re-logged as a fresh offer. `None` = never offered yet.
    pub last_offered_sig: Option<String>,
    last_debug_choice_sig: Option<String>,
    /// Zone the current `looked_at` pool was taken from (rule 5.7: looking at
    /// cards only informs — a DECLINED optional move must return them there,
    /// not to the default remainder destination).
    pub looked_at_origin: Option<String>,
    /// Numeric deck position (「デッキの上からN番目」) captured from the
    /// optional `looked_at`-source move_cards step when its SelectCard choice
    /// is spawned. The answer-time handler has no effect context, so the
    /// position rides here (consumed on first looked_at selection).
    pub looked_at_deck_position: Option<usize>,
    /// Declared intent of the pending Stage SelectCard choice, if its producer
    /// declared one. Consumed by handle_stage_selection.
    pub stage_select_intent: Option<crate::ability::types::StageSelectIntent>,
    /// Structured EffectData produced by the most recent gain_ability
    /// registration, forwarded to push_temporary_effect so expiry can revert
    /// by card id instead of text-searching the gained maps.
    pub last_gain_effect_data: Option<crate::core::types::EffectData>,
    pub pending_replacement: Option<(usize, AbilityEffect)>,
    pub resolving_replacement: bool,
    pub replacement_original_suppressed: bool,
    last_known_members: SmallVec<[(i16, LastKnownMemberInfo); 2]>,
}

impl AbilityResolver {
    /// Whether the ability currently being resolved is a 起動 (activation).
    /// Single source of truth for the trigger check previously copy-pasted at
    /// every optional-cost site.
    pub(crate) fn current_ability_is_activation(&self) -> bool {
        self.current_ability
            .as_ref()
            .is_some_and(|a| a.has_trigger(crate::triggers::TriggerKind::Activation))
    }

    /// Emit the shared 「〜してもよい／支払う？」 SelectTarget gate
    /// (target = pay_optional_cost:skip_optional_cost). ONE construction
    /// point for every optional-action question across costs, effect moves,
    /// look steps and energy placements; answers route through
    /// handle_optional_cost_payment. `route` selects the resume handler;
    /// None leaves any existing route untouched. The "Repeat effect?"
    /// prompts intentionally do NOT use this (different resume semantics).
    pub(crate) fn emit_pay_skip_gate(
        &mut self,
        gs: &mut GameState,
        route: Option<crate::ability::types::ChoiceRoute>,
        description_en: String,
        description_ja: String,
        allow_skip: bool,
        options: Option<Vec<String>>,
    ) {
        log::debug!(
            "[CHOICE] source={:?} optional payment: {} route={:?} retain_route={} allow_skip={}",
            self.activating_card_id,
            description_en,
            route.as_ref().or_else(|| gs.ability_queue.current_entry().and_then(|e| e.choice_card_no.as_ref())),
            route.is_none(),
            allow_skip
        );
        self.pending_choice = Some(crate::ability::types::Choice::SelectTarget {
            target: crate::ability::types::PAY_SKIP_TARGET.to_string(),
            description: description_en.clone(),
            description_en: Some(description_en),
            description_ja: Some(description_ja),
            allow_skip,
            options,
        });
        if let Some(route) = route {
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.choice_card_no = Some(route);
            }
        }
    }

    pub fn new(card_database: Arc<CardDatabase>, activating_card_id: Option<i16>) -> Self {
        AbilityResolver {
            pending_choice: None,
            card_database: card_database.clone(),
            duration_effects: SmallVec::new(),
            current_ability: None,
            current_ability_index: None,
            activating_card_id,
            execution_context: ExecutionContext::None,
            current_effect: None,
            parent_effect: None,
            is_reveal_cost: false,
            selected_cards: SmallVec::new(),
            changed_state_members: SmallVec::new(),
            selected_area: None,
            moved_cards: SmallVec::new(),
            keep_shuffle_under_phase: 0,
            keep_shuffle_under_count: 0,
            keep_shuffle_under_snapshots: SmallVec::new(),
            keep_shuffle_selected: SmallVec::new(),
            spawn_context: EffectSpawnContext::default(),
            sub_choice_created: false,
            deferred_conditional_gate: false,
            selected_count_at_save: None,
            pending_stage_cards: SmallVec::new(),
            debug_trace: false,
            pipeline: { EffectPipeline::new() },
            step_state: StepState::new(),
            pending_energy_payment: None,
            pending_deferred_costs: Vec::new(),
            cancel_remaining_commands: false,
            pending_repeat_actions: Vec::new(),
            pending_reprompt_choice: None,
            log_items: Vec::new(),
            formation_plan: SmallVec::new(),
             last_move_moved_any: None,
             last_action_result: None,
             last_offered_sig: None,

            last_debug_choice_sig: None,
            looked_at_origin: None,
            looked_at_deck_position: None,
            stage_select_intent: None,
            last_gain_effect_data: None,
            pending_replacement: None,
            resolving_replacement: false,
            replacement_original_suppressed: false,
            last_known_members: SmallVec::new(),
        }
    }

    pub(crate) fn record_member_last_known(
        &mut self,
        card_id: i16,
        card_db: &CardDatabase,
        blade_modifiers: &crate::HashMap<i16, crate::core::game_modifiers::ModifierEntry>,
        area: MemberArea,
    ) {
        let entry = blade_modifiers.get(&card_id).copied().unwrap_or_default();
        let blades = crate::core::stats_pipeline::effective_blade(card_db, card_id, entry);
        self.last_known_members.retain(|entry| entry.0 != card_id);
        self.last_known_members
            .push((card_id, LastKnownMemberInfo { blades, area }));
        log::debug!(
            "[LKI] card={} blades={} area={:?} stored=true",
            card_id,
            blades,
            area
        );
    }

    pub fn last_known_member_info(
        &self,
        gs: &GameState,
        card_id: i16,
    ) -> Option<LastKnownMemberInfo> {
        let current = [&gs.player1, &gs.player2]
            .iter()
            .any(|player| player.stage.stage.contains(&card_id));
        let info = self
            .last_known_members
            .iter()
            .find_map(|&(id, info)| (id == card_id).then_some(info));
        log::debug!(
            "[LKI] card={} current={} resolved={:?}",
            card_id,
            current,
            info
        );
        (!current).then_some(info).flatten()
    }

    pub fn get_pending_choice(&self) -> Option<&Choice> {
        self.pending_choice.as_ref()
    }

    /// Cached verdict for a condition marked `cache: true` on the current
    /// queue entry, if one was stored earlier in this ability's resolution.
    /// ONE definition of the cache lookup — used by the main condition gate
    /// (can_activate_effect) and the sequential step loop (compound.rs).
    pub fn cached_condition_verdict(
        &self,
        gs: &GameState,
        condition: &Condition,
    ) -> Option<bool> {
        if !condition.get_cache().unwrap_or(false) {
            return None;
        }
        let entry = gs.ability_queue.current_entry()?;
        let key = format!("{:?}", condition);
        entry
            .condition_cache
            .iter()
            .find(|(k, _)| k == &key)
            .map(|(_, v)| *v)
    }

    /// Store a verdict for a condition marked `cache: true` on the current
    /// queue entry. Replaces any previously stored verdict for the same key.
    pub fn store_condition_verdict(&self, gs: &mut GameState, condition: &Condition, passed: bool) {
        if !condition.get_cache().unwrap_or(false) {
            return;
        }
        if let Some(entry) = gs.ability_queue.current_entry_mut() {
            let key = format!("{:?}", condition);
            entry.condition_cache.retain(|(k, _)| k != &key);
            entry.condition_cache.push((key, passed));
        }
    }

    pub fn can_activate_effect(&self, gs: &mut GameState, effect: &AbilityEffect) -> bool {
        log::trace!(
            "[EFFECT] activation check: source={:?} action={} has_condition={}",
            self.activating_card_id,
            effect.action,
            effect.condition.is_some()
        );
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

        let mut activation_condition_passed = true;
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
                        crate::ability::log::drain_verdicts_since(snapshot);
                    }
                }
                // Fall through to the main condition gate as well — abilities with
                // BOTH a parenthetical activation-position restriction and a real
                // condition (e.g. 「このターン、このメンバーがエリアを移動している場合」)
                // must satisfy both. Early-returning here skipped the has_moved
                // gate entirely and granted resources unconditionally.
                if !result {
                    return false;
                }
                activation_condition_passed = true;
            }
        }
        let _ = activation_condition_passed;
        if let Some(ref condition) = effect.condition {
            if effect.action == crate::ability::enums::ActionType::ConditionalAlternative {
                // skip — condition is a branch selector, not a gate
            } else {
                // Check cache first — avoids re-evaluation against stale state
                // (e.g. revealed_cards modified by a prior select_cards filter).
                if let Some(cached) = self.cached_condition_verdict(gs, condition) {
                    log::debug!("[CONDITION] source={:?} action={} passed={} verdict=cached type={:?}", self.activating_card_id, effect.action, cached, condition.condition_type());
                    return cached;
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
                        crate::ability::log::drain_verdicts_since(cond_snapshot);
                    }
                }
                // Cache the result if the condition asks for it
                self.store_condition_verdict(gs, condition, passed);
                if !passed {
                    let act = gs.activating_card;
                    let act_pos = effect.action.to_str();
                    let ct = condition.condition_type();
                    let loc = condition.get_location();
                    gs.push_debug_note_fmt(format_args!(
                        "gate BLOCK card={:?} action={} cond_type={:?} location={:?}",
                        act, act_pos, ct, loc
                    ));
                    log::debug!("[CONDITION] source={:?} action={} passed=false type={:?} location={:?} group={:?} exclude={:?}",
                        self.activating_card_id, effect.action, condition.condition_type(), condition.get_location(), condition.get_group_names(), condition.get_exclude_characters());
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
                    let Some(idx) = crate::ability::util::activation_position_index(p) else {
                        return false;
                    };
                    idx < player.stage.stage.len()
                        && card_id.is_some()
                        && player.stage.stage[idx] == card_id.unwrap()
                });
                if !passes {
                    log::debug!(
                        "[CONDITION] source={:?} action={} passed=false activation_position={:?}",
                        card_id,
                        effect.action,
                        act_pos
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
            // Record a `choice_offered` structured entry at presentation time so
            // the log captures what options were actually shown (before the
            // game state may shift ahead of the player's eventual resolution).
            // Dedup: re-storing the SAME pending choice (re-prompt, or an
            // auto-ability processed in between) must not emit a second, identical
            // offer block. Only a genuinely different presentation is a new offer.
            let offered = gs.choice_offered_labels(choice);
            let sig = choice_offer_sig(&offered, choice.allow_skip());
            let is_new_offer = self.last_offered_sig.as_deref() != Some(sig.as_str());
            if is_new_offer {
                gs.push_choice_offered(choice);
                self.last_offered_sig = Some(sig);
            }
            let debug_choice_changed = if log::log_enabled!(log::Level::Debug) {
                let sig = format!("{:?}|{:?}|{:?}|{:?}|{:?}|{:?}", choice,
                    self.activating_card_id, self.current_ability_index,
                    gs.ability_queue.current_entry().and_then(|e| e.choice_card_no.as_ref()),
                    self.spawn_context, self.execution_context);
                let changed = is_new_offer || self.last_debug_choice_sig.as_ref() != Some(&sig);
                self.last_debug_choice_sig = Some(sig);
                changed
            } else {
                false
            };
            if debug_choice_changed {
                log::debug!("[CHOICE] source={:?} route={:?} context={:?}", self.activating_card_id,
                    gs.ability_queue.current_entry().and_then(|e| e.choice_card_no.as_ref()), self.execution_context);
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
                        filtered_indices,
                        picker,
                        destination,
                        cost_limit,
                        cost_limit_operator,
                        cost_total,
                        cost_total_operator,
                        cost_values,
                        characters,
                        require_all_heart_colors,
                        name_fragments,
                        blind,
                        is_reveal,
                        discard_remaining,
                        ..
                    } => {
                        log::debug!("[CHOICE] select cards: source={:?} zone={} count={} allow_skip={} group={:?} card_type={:?} select_only={} heart_colors={:?} target_player={:?} picker={:?} destination={:?} indices={:?} prompt={}", self.activating_card_id, zone, count, allow_skip, group, card_type, is_select_action, heart_colors, target_player_id, picker, destination, filtered_indices, description);
                        log::debug!("[CHOICE] filters: cost_limit={:?} operator={:?} cost_total={:?} total_operator={:?} cost_values={:?} characters={:?} all_hearts={:?} names={:?} blind={} reveal={} discard_remaining={:?}", cost_limit, cost_limit_operator, cost_total, cost_total_operator, cost_values, characters, require_all_heart_colors, name_fragments, blind, is_reveal, discard_remaining);
                    }
                    crate::ability::types::Choice::SelectHeartColor {
                        count,
                        options,
                        description,
                        ..
                    } => {
                    log::debug!("[CHOICE] source={:?} heart colors: count={} options={:?} prompt={}", self.activating_card_id, count, options, description);
                    }
                    crate::ability::types::Choice::SelectTarget {
                        target,
                        description,
                        options,
                        allow_skip,
                        ..
                    } => {
                        log::debug!("[CHOICE] source={:?} target: target={} options={:?} allow_skip={} prompt={}", self.activating_card_id, target, options, allow_skip, description);
                    }
                    crate::ability::types::Choice::SelectPosition { description, .. } => {
                        log::debug!("[CHOICE] source={:?} position: prompt={}", self.activating_card_id, description);
                    }
                    crate::ability::types::Choice::SelectHeartType {
                        count,
                        options,
                        description,
                        ..
                    } => {
                        log::debug!(
                            "[CHOICE] SelectHeartType count={} options={:?} description={}",
                            count,
                            options,
                            description
                        );
                    }
                    crate::ability::types::Choice::SelectAutoAbility { options, .. } => {
                        log::debug!("[CHOICE] SelectAutoAbility options={:?}", options);
                    }
                    crate::ability::types::Choice::SelectLiveSuccess { description, .. } => {
                        log::debug!(
                            "[CHOICE] SelectLiveSuccess description={}",
                            description
                        );
                    }
                }
            }
            #[cfg(feature = "serde_support")]
            {
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
        #[cfg(not(feature = "serde_support"))]
        let _ = &items;
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
        // Canonical trigger key (shared with triggers.rs scan + negated-skip).
        let trigger_str = crate::triggers::canonical_trigger(raw_trigger);
        let ability_text = self
            .current_ability
            .as_ref()
            .map(|a| a.full_text.clone())
            .unwrap_or_default();
        let zone = card_id
            .map(|cid| Self::zone_for_card(gs, cid))
            .unwrap_or_default();
        #[cfg(feature = "serde_support")]
        let items_json: Vec<serde_json::Value> = items
            .iter()
            .map(|i| serde_json::to_value(i).unwrap_or_default())
            .collect();
        let meta = crate::core::types::LogMetadata::AbilityResolution {
            result: result.to_string(),
            trigger: trigger_str.clone(),
            #[cfg(feature = "serde_support")]
            items: items_json.clone(),
            ability_text: ability_text.clone(),
            zone: zone.clone(),
            error: error.map(|e| e.to_string()),
            resolved: None,
        };
        if crate::game_setup::logging_enabled()
            || crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed)
        {
            let log_text = format!(
                "{pp} {card_name} [{zone}]: [[log_ability_result:trigger=trigger_{trigger_str},result=result_{}]]",
                result
            );
            gs.push_rule_log_fmt(format_args!("{}", log_text));
            if !crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
                return;
            }
            let fallback_entry = LogEntry {
                text: log_text,
                turn: gs.turn_number,
                player_label: pp.clone(),
                source_card_id: card_id,
                source_card_name: Some(card_name),
                category: "ability_resolution".to_string(),
                metadata: Some(meta),
            };
            // Commit to the matching trigger_evaluation entry (or push standalone).
            // Use the resolver's stored index (or the queue's current entry) because
            // the queue entry may have changed during effect execution.
            gs.commit_or_push_structured(
                card_id,
                &trigger_str,
                self.current_ability_index,
                crate::core::types::LogMetadata::AbilityResolution {
                    result: result.to_string(),
                    trigger: trigger_str.clone(),
                    #[cfg(feature = "serde_support")]
                    items: items_json.clone(),
                    ability_text: ability_text.clone(),
                    zone: zone.clone(),
                    error: error.map(|e| e.to_string()),
                    resolved: Some(true),
                },
                fallback_entry,
            );
        }
    }

    /// USE-LIMIT RECORDING MAP (resolver + choice.rs) — when the key gets
    /// inserted depends on WHERE in the resolution lifecycle the attempt ends:
    ///
    /// | lifecycle moment                              | record? | why |
    /// |-----------------------------------------------|---------|-----|
    /// | trigger accepted, no cost/choice pending      | yes (guarded by can_activate) | premature auto triggers must not consume |
    /// | effect condition fails, 起動 ability          | yes | player deliberately paid; attempt counts |
    /// | effect condition fails, auto ability          | no  | preserve budget until state satisfies |
    /// | pending choice = conditional_optional         | deferred to choice answer | player decides whether to pay |
    /// | pending choice = position|destination (optional) | deferred | may-place: count only if placed |
    /// | pending choice = other                        | yes | choice is part of the paid effect |
    /// | normal completion                             | yes (guarded) | fallback for paths above |
    ///
    /// choice.rs adds two more variants for OPTIONAL effects whose recording
    /// happens inside the choice resume handler (accepted → record, declined
    /// → never). Any consolidation must keep these distinctions.
    ///
    /// The check/record primitives live in `gates.rs` (`use_limit_gate`,
    /// `record_use_limit`); the composite gates in `resolve_ability` call them
    /// directly. The helpers below are the pre-composite call sites kept for
    /// result-filing (position_fail logging) — not duplicated gate logic.
    /// Post-cost activation-keyword check. Returns Ok(()) when the ability's
    /// position keywords are satisfied (or it has none / isn't an on-stage card).
    /// Shared by `gate_post_cost_position` (result filing) and
    /// `gates::post_cost_position_gate` (composite) — one filter, two callers.
    #[allow(dead_code)]
    fn check_post_cost_position_keywords(
        &mut self,
        gs: &mut GameState,
        ability: &Ability,
        activating_card: Option<i16>,
        card_name: &str,
    ) -> Result<(), String> {
        let Some(card_id) = activating_card else {
            return Ok(());
        };
        let position = gs
            .player1
            .stage
            .stage
            .iter()
            .position(|&id| id == card_id)
            .or_else(|| gs.player2.stage.stage.iter().position(|&id| id == card_id))
            .map(util::pos_to_area);
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
                    if !ability.has_trigger(crate::triggers::TriggerKind::Activation)
                        && !ability.has_trigger(crate::triggers::TriggerKind::Debut)
                    {
                        let pp2 = gs.player_prefix();
                        gs.push_rule_log_fmt(format_args!(
                            "{pp2} {card_name}: [[log_position_fail:keyword={kw:?}]]"
                        ));
                    }
                    return Err("position requirement not met — effect skipped".to_string());
                }
            }
        }
        Ok(())
    }

    /// Gate: reject already-exhausted turn-limited abilities.
    /// Pre-composite filing site: the live path runs `pre_cost_gates()`
    /// (which owns the reach check via `gates::use_limit_gate`); this stays
    /// for direct callers that need the `skipped` result filed with debug.
    #[allow(dead_code)]
    fn gate_ability_use_limit(
        &mut self,
        gs: &mut GameState,
        ability: &Ability,
        activating_card: Option<i16>,
        ability_index: usize,
        ability_key: Option<(i16, usize, u8)>,
        dbg: &mut AbDebug,
    ) -> Result<(), String> {
        let Some(card_id) = activating_card else {
            return Ok(());
        };
        let Some(use_limit) = ability.use_limit else {
            return Ok(());
        };
        let _ = ability_key;
        if use_limit_gate(self, gs, ability).is_continue() {
            return Ok(());
        }
        let used = gs.ability_uses_used(card_id, ability_index);
        let msg = format!(
            "Ability already used {} of {} times this turn",
            used, use_limit
        );
        dbg.p("RESULT", &msg);
        let items = drain_verdicts();
        self.push_ability_result(gs, "skipped", items, Some(&msg));
        Err(msg)
    }

    /// Gate: activation keywords (center/left/right/turn position restrictions).
    /// Pre-composite filing site: the live path runs `pre_cost_gates()`
    /// (which owns the check via `gates::activation_keywords_gate`); this
    /// stays for direct callers that need the `position_fail` result filed.
    #[allow(dead_code)]
    fn gate_activation_keywords(
        &mut self,
        gs: &mut GameState,
        ability: &Ability,
        activating_card: Option<i16>,
    ) -> Result<(), String> {
        let Some(card_id) = activating_card else {
            return Ok(());
        };
        let _ = card_id;
        if activation_keywords_gate(self, gs, ability).is_continue() {
            return Ok(());
        }
        let items = drain_verdicts();
        self.push_ability_result(
            gs,
            "position_fail",
            items,
            Some("Activation keywords not satisfied"),
        );
        Err(
            "Activation keywords not satisfied (e.g. card not at required position)"
                .to_string(),
        )
    }

    /// Phase: pay the ability cost (with modify-cost applied). No-op when the
    /// queue entry already records a paid cost.
    fn pay_ability_cost(
        &mut self,
        gs: &mut GameState,
        ability: &Ability,
        cost_already_paid: bool,
        dbg: &mut AbDebug,
    ) -> Result<(), String> {
        if cost_already_paid {
            return Ok(());
        }
        let Some(ref cost) = ability.cost else {
            return Ok(());
        };
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
            cost.source.map(|z| z.as_str()).unwrap_or("?"),
            cost.destination.map(|z| z.as_str()).unwrap_or("?"),
            cost.count.unwrap_or(cost.energy_count_any().unwrap_or(1))
        );
        #[cfg(not(feature = "no_std"))]
        push_verdict(AbilityLogItem::Cost {
            text: cost.text.to_string(),
            expectation: cost_desc,
            actual: "支払済".into(),
            passed: true,
            optional: cost.optional.unwrap_or(false),
        });
        Ok(())
    }

    /// Record the turn-limit key early (before the effect runs) unless the
    /// effect is still undecided (conditional_on_optional / optional effect).
    /// Pre-composite phase helper: the live path inlines
    /// `gates::record_use_limit(Early)` directly; this stays for direct
    /// callers that resolve an ability without the full pipeline.
    #[allow(dead_code)]
    fn record_early_ability_use(
        &mut self,
        gs: &mut GameState,
        ability: &Ability,
        ability_key: Option<(i16, usize, u8)>,
        cost_already_paid: bool,
    ) {
        record_use_limit(self, gs, ability, ability_key, cost_already_paid, UseLimitPhase::Early, None);
    }

    /// Drain: a pending choice created by cost payment ends resolution here.
    /// Returns true when the caller must `return Ok(())`.
    /// Pre-composite phase helper: the live path inlines
    /// `gates::handle_pending_choice(cost)` directly; this stays for direct
    /// callers.
    #[allow(dead_code)]
    fn drain_cost_pending_choice(&mut self, gs: &mut GameState, cost_already_paid: bool) -> bool {
        let ability = self.current_ability.clone().unwrap_or_default();
        let key = self.activating_card_id.map(|id| (id, self.current_ability_index.unwrap_or(0), gs.turn_number));
        handle_pending_choice(self, gs, &ability, key, cost_already_paid, true)
    }

    /// Gate: post-cost stage-position keywords gate the effect, not the cost.
    /// Returns true to continue, false when the effect was skipped (result
    /// already recorded — the caller must `return Ok(())`).
    /// Owns the position_fail result filing (including the auto-ability log
    /// suppression); the keyword filter itself is shared with
    /// `gates::post_cost_position_gate` via `check_post_cost_position_keywords`.
    /// Pre-composite filing site: the live path runs `post_cost_gates()`.
    #[allow(dead_code)]
    fn gate_post_cost_position(
        &mut self,
        gs: &mut GameState,
        ability: &Ability,
        activating_card: Option<i16>,
        card_name: &str,
        dbg: &mut AbDebug,
    ) -> bool {
        // Check position keywords (Center/LeftSide/RightSide) AFTER cost payment.
        // Position checks gate the effect, not the cost — the test expects cost to still be paid.
        if let Some(card_id) = activating_card {
            match self.check_post_cost_position_keywords(gs, ability, Some(card_id), card_name) {
                Err(_) => {
                    dbg.p("RESULT", "position requirement not met — effect skipped");
                    let items = drain_verdicts();
                    self.push_ability_result(gs, "position_fail", items, None);
                    return false;
                }
                Ok(()) => {}
            }
        }
        true
    }

    /// Gate: a skipped optional cost suppresses the primary effect.
    /// Returns true when the effect was skipped (caller must `return Ok(())`).
    /// Thin delegate to `gates::optional_cost_skip_gate` — the gate owns the
    /// `optional_cost_result` read; this only files the `skipped` result.
    /// Pre-composite filing site: the live path runs `post_cost_gates()`.
    #[allow(dead_code)]
    fn gate_optional_cost_skip(
        &mut self,
        gs: &mut GameState,
        activating_card: Option<i16>,
        ability_index: usize,
        dbg: &mut AbDebug,
    ) -> bool {
        use super::gates::optional_cost_skip_gate;
        let dummy = Ability::default();
        let gate_result = optional_cost_skip_gate(self, gs, &dummy);
        let cost_was_skipped = gate_result.is_stop();
        log::debug!(
            "[COST] source={:?} ability={} effect_gate={} optional_cost_result={:?}",
            activating_card,
            ability_index,
            if cost_was_skipped { "skip: optional cost unpaid" } else { "continue" },
            gs.ability_queue
                .current_entry()
                .and_then(|e| e.optional_cost_result)
        );
        if !cost_was_skipped {
            return false;
        }
        dbg.p("RESULT", "optional cost skipped — effect not executed");
        let items = drain_verdicts();
        self.push_ability_result(gs, "skipped", items, Some("optional cost not paid"));
        true
    }

    /// Phase: check the effect condition, execute it, and file the
    /// pending-choice bookkeeping when the effect asks the player something.
    /// Returns `Ok(true)` when the caller must `return Ok(())`.
    fn run_ability_effect(
        &mut self,
        gs: &mut GameState,
        ability: &Ability,
        ability_key: Option<(i16, usize, u8)>,
        activating_card: Option<i16>,
        cost_already_paid: bool,
        dbg: &mut AbDebug,
    ) -> Result<bool, String> {
        let Some(ref effect) = ability.effect else {
            return Ok(false);
        };
        
        // Effect-level gates
        let gate_result = effect_gates().check_effect(self, gs, effect);
        if gate_result.is_stop() {
            let (reason, result_type) = gate_result.unwrap_stop();
            let items = drain_verdicts();
            self.push_ability_result(gs, &result_type, items, Some(&reason));
            return Ok(true);
        }
        
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
                    && ability.has_trigger(crate::triggers::TriggerKind::Activation)
                {
                    if let Some(ref key) = ability_key {
                        gs.record_ability_use(*key);
                    }
                }
                dbg.p("RESULT", "effect condition not met — skipped");
                let items = drain_verdicts();
                self.push_ability_result(gs, "failure", items, None);
                return Ok(true);
            }
        }
        if let Err(e) = self.execute_effect(gs, effect) {
            dbg.p("RESULT", format_args!("EFFECT FAILED: {}", e));
            let items = drain_verdicts();
            self.push_ability_result(gs, "failure", items, Some(&e));
            return Err(e);
        }
        log::trace!(
            "[EFFECT] source={:?} action={} returned: pending={}",
            activating_card,
            effect.action,
            self.pending_choice.is_some()
        );
        if self.pending_choice.is_none() {
            dbg.p("RESULT", "effect applied ✓");
            let items = drain_verdicts();
            self.push_ability_result(gs, "success", items, None);
            return Ok(false);
        }
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
                    gs.record_ability_use(*key);
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
        Ok(true)
    }

    /// Phase: final turn-limit accounting after the effect ran.
    /// Pre-composite phase helper: the live path inlines
    /// `gates::record_use_limit(Final)` directly; this stays for direct
    /// callers.
    #[allow(dead_code)]
    fn record_final_ability_use(
        &mut self,
        gs: &mut GameState,
        ability: &Ability,
        ability_key: Option<(i16, usize, u8)>,
        cost_already_paid: bool,
    ) {
        record_use_limit(self, gs, ability, ability_key, cost_already_paid, UseLimitPhase::Final, None);
    }

    /// Phase: clear per-ability resolver state + close the debug trace.
    fn finish_ability_resolution(&mut self, gs: &mut GameState) {
        gs.activating_card = None;
        self.current_ability = None;
        self.current_ability_index = None;
        self.last_known_members.clear();

        if self.debug_trace {
            self.pipeline.trace.after = Some(ZoneSnapshot::from_game_state(gs));
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
        self.last_action_result = None;
        self.last_known_members.clear();
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

        // Pre-cost gates
        let pre_gate_result = pre_cost_gates().check_ability(self, gs, ability);
        if pre_gate_result.is_stop() {
            let (reason, result_type) = pre_gate_result.unwrap_stop();
            let items = drain_verdicts();
            self.push_ability_result(gs, &result_type, items, Some(&reason));
            return Err(reason);
        }

        let cost_already_paid = gs
            .ability_queue
            .current_entry()
            .is_some_and(|e| e.cost_paid);

        self.pay_ability_cost(gs, ability, cost_already_paid, &mut dbg)?;

        // Early use limit recording
        record_use_limit(self, gs, ability, ability_key, cost_already_paid, UseLimitPhase::Early, None);

        if handle_pending_choice(self, gs, ability, ability_key, cost_already_paid, true) {
            return Ok(());
        }

        // Post-cost gates
        let post_gate_result = post_cost_gates().check_ability(self, gs, ability);
        if post_gate_result.is_stop() {
            let (reason, result_type) = post_gate_result.unwrap_stop();
            let items = drain_verdicts();
            self.push_ability_result(gs, &result_type, items, Some(&reason));
            return Err(reason);
        }

        // Mark cost as paid when it auto-resolved without creating a pending choice.
        if !cost_already_paid && ability.cost.is_some() && self.pending_choice.is_none() {
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.cost_paid = true;
            }
        }

        if handle_pending_choice(self, gs, ability, ability_key, cost_already_paid, true) {
            return Ok(());
        }

        if self.run_ability_effect(gs, ability, ability_key, activating_card, cost_already_paid, &mut dbg)? {
            return Ok(());
        }

        // Final use limit recording
        record_use_limit(self, gs, ability, ability_key, cost_already_paid, UseLimitPhase::Final, None);
        self.finish_ability_resolution(gs);

        Ok(())
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
                {
                    self.apply_per_unit_cost_reduction(gs, &mut cost, &mod_cost);
                }
            }
        }
        cost
    }

    /// Per-unit cost reduction: `(units / per_unit_count) * amount`.
    /// ONE definition of the reduction math shared by the group-count and
    /// success-zone branches below (and generation's effective-cost
    /// evaluator, which inlines the same formula).
    fn per_unit_reduction(total_units: u8, per_unit_count: u8, unit_amount: u8) -> u8 {
        (total_units / per_unit_count.max(1)) * unit_amount
    }

    /// Apply a parsed per-unit modify_cost to a cloned ability cost.
    fn apply_per_unit_cost_reduction(
        &self,
        gs: &mut GameState,
        cost: &mut AbilityEffect,
        mod_cost: &AbilityEffect,
    ) {
        let per_unit_count = mod_cost.per_unit_count_any().unwrap_or(1);
        let unit = mod_cost.count.unwrap_or(1);
        let per_unit_type = mod_cost.per_unit_type_any();
        if per_unit_type.as_deref() == Some("group_name") {
            // Count distinct group names on self's stage
            // (shared with generation's effective-cost evaluator).
            let groups = gs.distinct_stage_groups("self");
            let reduction = Self::per_unit_reduction(groups, per_unit_count, unit);
            if cost.action == crate::ability::enums::ActionType::PayEnergy {
                let new_energy = cost
                    .energy_count_any()
                    .unwrap_or(0)
                    .saturating_sub(reduction);
                cost.set_energy_count(Some(new_energy));
            }
            return;
        }
        if matches!(
            per_unit_type.as_deref(),
            Some("success_live_card_zone")
                | Some("success_live_zone")
                | Some("live_card_zone")
                | Some("live_zone")
        ) && cost.action == crate::ability::enums::ActionType::MoveCards
        {
            // PB1-007: hand discard 3 reduced by 1 per success live (e.g. PL!-pb1-007-R)
            // per_unit_type may be live_card_zone due to parser gap; handle both.
            let player_id = gs
                .ability_queue
                .current_entry()
                .map(|e| e.player_id.clone())
                .unwrap_or_else(|| gs.player1.id.clone());
            let success_len = gs
                .try_player_by_id(&player_id)
                .unwrap_or(&gs.player2)
                .success_live_card_zone
                .cards
                .len() as u8;
            let reduction = Self::per_unit_reduction(success_len, per_unit_count, unit);
            let new_count = cost.count.unwrap_or(0).saturating_sub(reduction);
            cost.count = Some(new_count);
            log::debug!(
                "[COST_REDUCE_HAND] success_len={} per_unit_cnt={} unit={} reduction={} new_count={}",
                success_len,
                per_unit_count,
                unit,
                reduction,
                new_count
            );
        }
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
