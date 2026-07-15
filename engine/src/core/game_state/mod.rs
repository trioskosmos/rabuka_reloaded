use crate::ability::enums::Zone;
use crate::ability_queue::AbilityQueue;
use crate::card::CardDatabase;
use crate::constants::DEFAULT_HISTORY_SIZE;
use crate::core::game_modifiers::GameModifiers;
use crate::player::Player;
use crate::zones::{MemberArea, ResolutionZone};
use std::sync::Arc;

pub use crate::types::{
    AbilityApplication, AbilityBonus, AbilityTrigger, Adjustment, Allocation, BladeSource,
    Breakdown, Duration, EffectEntry, GameResult, HeartSource, LiveCardResult, LivePerformanceData,
    LogEntry, MemberContribution, MovementEvent, PerformanceSnapshot, Phase, ReplacementEffect,
    ScoreLine, TemporaryEffect, TriggeredAbility, TurnPhase, YellCardResult,
};

#[derive(Debug, Clone)]
pub struct GameState {
    // --- 8-byte aligned (Player, HashMap, HashSet, String, Vec, Arc, usize, Value) ---
    pub player1: Player,
    pub player2: Player,
    pub ability_queue: AbilityQueue,
    pub card_database: Arc<CardDatabase>,
    pub mods: GameModifiers,
    pub resolution_zone: ResolutionZone,
    pub heart_color_decision_phase: String,
    pub game_state_history: Vec<u64>,
    pub max_state_history_size: usize,
    pub rule_log: Vec<String>,
    pub structured_log: Vec<LogEntry>,
    pub turn1_abilities_played: std::collections::HashSet<String>,
    pub turn2_abilities_played: std::collections::HashMap<String, u32>,
    pub live_owned_hearts: std::collections::HashMap<String, Vec<(String, u32)>>,
    pub temporary_effects: Vec<TemporaryEffect>,
    pub prohibition_effects: Vec<String>,
    pub delayed_prohibition_effects: Vec<String>,
    pub non_stackable_effects: std::collections::HashSet<String>,
    pub cannot_activate_members: Vec<String>,
    pub constant_cannot_activate_members: std::collections::HashSet<String>,
    pub cannot_live_players: Vec<String>,
    pub turn_limited_abilities_used: std::collections::HashMap<(i16, usize, u32), u8>,
    pub mulligan_selected_indices: Vec<usize>,
    pub live_card_selected_indices: Vec<usize>,
    pub auto_ability_trigger_counts: std::collections::HashMap<String, u32>,
    pub turn_limit_usage: std::collections::HashMap<String, u32>,
    pub card_instance_mapping: std::collections::HashMap<i16, u32>,
    pub areas_placed_this_turn: std::collections::HashSet<String>,
    pub cards_appeared_this_turn: std::collections::HashSet<i16>,
    pub card_appearance_source: std::collections::HashMap<i16, String>,
    pub cards_moved_this_turn: std::collections::HashSet<i16>,
    pub gained_abilities: std::collections::HashMap<i16, Vec<String>>,
    /// Full Ability structs dynamically added to cards via gain_ability.
    /// These are scanned by the trigger pipeline alongside original card abilities.
    pub gained_card_abilities: std::collections::HashMap<i16, Vec<crate::card::Ability>>,
    /// Gained effects that couldn't be evaluated at constant time (e.g. because
    /// they depend on revealed_cards from the yell).  Stored as `(card_id, effect)`
    /// and evaluated during `execute_live_victory_determination` when the yell
    /// results are available.
    pub delayed_gained_effects: Vec<(i16, crate::card::AbilityEffect)>,
    pub negated_abilities: std::collections::HashSet<i16>,
    pub replacement_effects: Vec<ReplacementEffect>,
    pub constant_ability_statuses: Vec<crate::types::ConstantAbilityStatus>,
    pub revealed_cards: Vec<i16>,
    pub revealed_card_sources: Vec<Option<i16>>,
    pub revealed_card_source_names: Vec<Option<String>>,
    pub revealed_card_is_private: Vec<bool>,
    pub revealed_card_owners: Vec<Option<u8>>,
    pub revealed_cost_cards: Vec<i16>,
    pub revealed_cost_card_sources: Vec<Option<i16>>,
    pub revealed_cost_card_source_names: Vec<Option<String>>,
    pub revealed_cost_card_is_private: Vec<bool>,
    pub revealed_cost_card_owners: Vec<Option<u8>>,
    pub player1_cheer_revealed_cards: Vec<i16>,
    pub player2_cheer_revealed_cards: Vec<i16>,
    /// Cards revealed by the initial yell (saved before re-yell overwrites them).
    pub initial_yell_revealed_cards: Vec<i16>,
    /// Cards revealed by a re-yell (set after perform_yell draws new cards).
    pub re_yell_revealed_cards: Vec<i16>,
    pub looked_at_cards: Vec<i16>,
    pub ability_applications: Vec<crate::types::AbilityApplication>,
    /// Synced from batch_movements by push_movement_event().
    pub recently_moved_cards: Option<Vec<i16>>,
    pub recently_appeared_cards: Vec<i16>,
    pub recently_moved_from_zone: Option<String>,
    /// Explicit per-batch event log of stage-area-to-stage-area position changes.
    /// Each entry records the moved card, old/new position, and cause info.
    /// Replaces the fragile snapshot-based detection with direct event tracking.
    /// Cleared at the end of each ability batch process (after post-loop TAS scan
    /// in process_player_abilities), NOT in clear_effect_tracking.
    pub position_change_events: Vec<crate::types::PositionChangeEvent>,

    /// Detailed event log for per-batch tracking: cards moved in the current
    /// cost/effect batch, what caused the move (cause_card_id), etc.
    /// recently_moved_cards/from_zone are synced from this vec.
    pub batch_movements: Vec<MovementEvent>,
    /// Turn-level record of stage-area-to-stage-area movements.
    /// Used by conditions checking "this member has moved areas this turn".
    pub turn_area_movements: Vec<MovementEvent>,
    /// Turn-level record of ALL zone-to-zone movements.
    /// Accumulated across the entire turn (not cleared between ability batches).
    /// Used by movement conditions with source+destination zones
    /// (e.g. "member card went from live_card_zone to discard this turn").
    pub turn_movements: Vec<MovementEvent>,
    /// Counter for assigning unique timestamps to MovementEvents within a turn.
    pub movement_event_counter: u32,
    /// Snapshot of target cards' orientations taken before a change_state
    /// effect executes. Compared after the effect to detect actual transitions.
    /// None = no snapshot active.
    pub state_snapshot_before_change: Option<std::collections::HashMap<i16, Option<String>>>,
    /// After a change_state effect executes, records what actually changed:
    /// (card_id, from_state, to_state). Cleared after post-resolution TAS scan.
    pub recently_state_changed: Vec<(i16, String, String)>,
    pub debut_ability_triggers: Vec<(String, i16)>,
    pub last_vacated_stage_area: Option<usize>,
    // --- 4-byte aligned (u32, Option<i32>) ---
    pub turn_number: u32,
    pub live_cheer_count: u32,
    pub player1_cheer_blade_heart_count: u32,
    pub player2_cheer_blade_heart_count: u32,
    pub cheer_checks_required: u32,
    pub cheer_checks_done: u32,
    pub card_instance_counter: u32,
    pub baton_touch_count: std::collections::HashMap<String, u32>,
    pub baton_touch_arriving_card_ids: Vec<i16>,
    pub effect_creation_counter: u32,
    pub last_state_change_wait_to_active_count: u32,
    pub player1_rps_choice: Option<i32>,
    pub player2_rps_choice: Option<i32>,
    pub baton_touch_replaced_member_cost: Option<u32>,
    pub baton_touch_replaced_member_id: Option<i16>,
    pub baton_touch_arriving_card_id: Option<i16>,
    /// Set during the performance phase after a yell actually occurs
    /// (total_blade > 0 after modifiers). Checked by on_yell abilities.
    pub yell_occurred: bool,
    /// Set by execute_re_yell when a card's ability triggers a re-yell.
    /// The phase code checks this to re-compute yell data for live success.
    pub re_yell_occurred: bool,

    // --- 2-byte aligned (i16, Option<i16>) ---
    pub activating_card: Option<i16>,
    pub activating_ability_index: Option<usize>,
    /// Key of the most recently completed auto ability, used by the re-scan
    /// to prevent re-enqueueing the exact same ability while still allowing
    /// other abilities on the same card (e.g. each_time) to fire.
    pub just_completed_ability_key: Option<String>,
    /// Batch-scoped set of ability IDs already enqueued during the current movement batch.
    /// Prevents each_time/movement abilities from being re-enqueued across multiple
    /// post-resolution TAS scans within the same batch. Cleared at post-loop batch scan.
    pub this_batch_triggered_ability_ids: std::collections::HashSet<String>,
    /// Cutoff index for depth-first each_time drain. Entries enqueued at >= this index
    /// are newly-triggered (each_time watchers) and must be force-resolved before
    /// stale entries are offered to the player. Set by process_player_abilities and
    /// resume_queue_with_choice before calling process_current_ability.
    pub depth_first_cutoff: Option<usize>,
    // --- 1-byte aligned (bool, enum) ---
    pub rps_winner: Option<u8>,
    pub current_turn_phase: TurnPhase,
    pub current_phase: Phase,
    pub game_result: GameResult,
    pub is_first_turn: bool,
    pub cheer_check_completed: bool,
    pub turn_order_changed: bool,
    pub baton_touch_zero_cost: bool,
    pub deck_refresh_pending: bool,
    pub position_change_occurred_this_turn: bool,
    pub opponent_live_success_this_turn: bool,
    pub opponent_live_no_excess_heart_this_turn: bool,
    pub self_no_excess_heart_this_turn: bool,
    pub opponent_live_surplus_count: u32,
    pub self_live_surplus_count: u32,
    pub formation_change_occurred_this_turn: bool,
    pub opponent_choice_declined: bool,
    pub live_being_performed: bool,
    pub game_ended: bool,
    pub draw_state: bool,
    pub loop_detected: bool,
    pub live_success_triggered_this_turn: bool,
    pub live_success_p2_fired: bool,
    pub live_success_p1_extra: u32,
    pub live_success_p2_extra: u32,
    pub live_surplus_ready_this_turn: bool,
    pub performance_snapshots: Vec<PerformanceSnapshot>,
    /// Trace from the last ability resolution (for debugging).
    pub last_ability_trace: Option<crate::ability::types::AbilityTraceNode>,
    /// Card ID being replaced by a success zone replacement effect (e.g. 錯覚CROSSROADS).
    pub pending_success_replacement_card_id: Option<i16>,
    /// Player ID of the player making the success zone replacement choice.
    pub pending_success_replacement_player_id: Option<String>,
    /// Transient: set by web_server for PVP RPS routing so the action handler
    /// knows which player sent the request (0=P1, 1=P2).
    pub pending_rps_player_id: Option<i32>,
}

impl GameState {
    pub fn phase_invariant(&self) -> bool {
        if matches!(
            self.current_phase,
            Phase::RockPaperScissors
                | Phase::ChooseFirstAttacker
                | Phase::MulliganFirstAttacker
                | Phase::MulliganSecondAttacker
        ) {
            return true;
        }
        match self.current_turn_phase {
            TurnPhase::FirstAttackerNormal | TurnPhase::SecondAttackerNormal => {
                matches!(
                    self.current_phase,
                    Phase::Active | Phase::Energy | Phase::Draw | Phase::Main
                )
            }
            TurnPhase::Live => {
                matches!(
                    self.current_phase,
                    Phase::LiveCardSetFirstAttacker
                        | Phase::LiveCardSetSecondAttacker
                        | Phase::FirstAttackerPerformance
                        | Phase::SecondAttackerPerformance
                        | Phase::LiveVictoryDetermination
                )
            }
        }
    }

    pub fn new(player1: Player, player2: Player, card_database: Arc<CardDatabase>) -> Self {
        let state = GameState {
            player1,
            player2,
            ability_queue: AbilityQueue::new(),
            card_database,
            mods: GameModifiers::new(),
            resolution_zone: ResolutionZone::new(),
            heart_color_decision_phase: "none".to_string(),
            game_state_history: Vec::new(),
            max_state_history_size: DEFAULT_HISTORY_SIZE,
            rule_log: Vec::new(),
            structured_log: Vec::new(),
            turn1_abilities_played: std::collections::HashSet::new(),
            turn2_abilities_played: std::collections::HashMap::new(),
            live_owned_hearts: std::collections::HashMap::new(),
            temporary_effects: Vec::new(),
            prohibition_effects: Vec::new(),
            delayed_prohibition_effects: Vec::new(),
            non_stackable_effects: std::collections::HashSet::new(),
            cannot_activate_members: Vec::new(),
            constant_cannot_activate_members: std::collections::HashSet::new(),
            cannot_live_players: Vec::new(),
            turn_limited_abilities_used: std::collections::HashMap::new(),
            mulligan_selected_indices: Vec::new(),
            live_card_selected_indices: Vec::new(),
            auto_ability_trigger_counts: std::collections::HashMap::new(),
            turn_limit_usage: std::collections::HashMap::new(),
            card_instance_mapping: std::collections::HashMap::new(),
            areas_placed_this_turn: std::collections::HashSet::new(),
            cards_appeared_this_turn: std::collections::HashSet::new(),
            card_appearance_source: std::collections::HashMap::new(),
            cards_moved_this_turn: std::collections::HashSet::new(),
            gained_abilities: std::collections::HashMap::new(),
            gained_card_abilities: std::collections::HashMap::new(),
            delayed_gained_effects: Vec::new(),
            negated_abilities: std::collections::HashSet::new(),
            replacement_effects: Vec::new(),
            constant_ability_statuses: Vec::new(),
            revealed_cards: Vec::new(),
            revealed_card_sources: Vec::new(),
            revealed_card_source_names: Vec::new(),
            revealed_card_is_private: Vec::new(),
            revealed_card_owners: Vec::new(),
            revealed_cost_cards: Vec::new(),
            revealed_cost_card_sources: Vec::new(),
            revealed_cost_card_source_names: Vec::new(),
            revealed_cost_card_is_private: Vec::new(),
            revealed_cost_card_owners: Vec::new(),
            player1_cheer_revealed_cards: Vec::new(),
            player2_cheer_revealed_cards: Vec::new(),
            initial_yell_revealed_cards: Vec::new(),
            re_yell_revealed_cards: Vec::new(),
            looked_at_cards: Vec::new(),
            ability_applications: Vec::new(),
            recently_moved_cards: None,
            recently_appeared_cards: Vec::new(),
            recently_moved_from_zone: None,
            position_change_events: Vec::new(),
            batch_movements: Vec::new(),
            turn_area_movements: Vec::new(),
            turn_movements: Vec::new(),
            movement_event_counter: 0,
            state_snapshot_before_change: None,
            recently_state_changed: Vec::new(),
            debut_ability_triggers: Vec::new(),
            last_vacated_stage_area: None,
            // 4-byte aligned
            turn_number: 1,
            live_cheer_count: 0,
            player1_cheer_blade_heart_count: 0,
            player2_cheer_blade_heart_count: 0,
            cheer_checks_required: 0,
            cheer_checks_done: 0,
            card_instance_counter: 0,
            baton_touch_count: std::collections::HashMap::new(),
            baton_touch_arriving_card_ids: Vec::new(),
            effect_creation_counter: 0,
            last_state_change_wait_to_active_count: 0,
            player1_rps_choice: None,
            player2_rps_choice: None,
            baton_touch_replaced_member_cost: None,
            baton_touch_replaced_member_id: None,
            baton_touch_arriving_card_id: None,
            yell_occurred: false,
            re_yell_occurred: false,
            // 2-byte aligned
            activating_card: None,
            activating_ability_index: None,
            just_completed_ability_key: None,
            this_batch_triggered_ability_ids: std::collections::HashSet::new(),
            depth_first_cutoff: None,
            // 1-byte aligned
            rps_winner: None,
            current_turn_phase: TurnPhase::FirstAttackerNormal,
            current_phase: Phase::Active,
            game_result: GameResult::Ongoing,
            is_first_turn: true,
            cheer_check_completed: false,
            turn_order_changed: false,
            baton_touch_zero_cost: false,
            deck_refresh_pending: false,
            position_change_occurred_this_turn: false,
            opponent_live_success_this_turn: false,
            opponent_live_no_excess_heart_this_turn: false,
            self_no_excess_heart_this_turn: false,
            opponent_live_surplus_count: 0,
            self_live_surplus_count: 0,
            formation_change_occurred_this_turn: false,
            opponent_choice_declined: false,
            live_being_performed: false,
            game_ended: false,
            draw_state: false,
            loop_detected: false,
            live_success_triggered_this_turn: false,
            live_success_p2_fired: false,
            live_success_p1_extra: 0,
            live_success_p2_extra: 0,
            live_surplus_ready_this_turn: false,
            performance_snapshots: Vec::new(),
            last_ability_trace: None,
            pending_success_replacement_card_id: None,
            pending_success_replacement_player_id: None,
            pending_rps_player_id: None,
        };
        debug_assert!(
            state.phase_invariant(),
            "GameState phase invariant violated after creation"
        );
        state
    }

    pub fn active_player(&self) -> &Player {
        match self.current_phase {
            Phase::MulliganFirstAttacker | Phase::LiveCardSetFirstAttacker => self.first_attacker(),
            Phase::MulliganSecondAttacker | Phase::LiveCardSetSecondAttacker => {
                self.second_attacker()
            }
            Phase::FirstAttackerPerformance | Phase::LiveVictoryDetermination => {
                self.first_attacker()
            }
            Phase::SecondAttackerPerformance => self.second_attacker(),
            _ => match self.current_turn_phase {
                TurnPhase::FirstAttackerNormal => self.first_attacker(),
                TurnPhase::SecondAttackerNormal => self.second_attacker(),
                TurnPhase::Live => self.first_attacker(),
            },
        }
    }

    pub fn active_player_mut(&mut self) -> &mut Player {
        match self.current_phase {
            Phase::MulliganFirstAttacker | Phase::LiveCardSetFirstAttacker => {
                self.first_attacker_mut()
            }
            Phase::MulliganSecondAttacker | Phase::LiveCardSetSecondAttacker => {
                self.second_attacker_mut()
            }
            Phase::FirstAttackerPerformance | Phase::LiveVictoryDetermination => {
                if self.player1.is_first_attacker {
                    &mut self.player1
                } else {
                    &mut self.player2
                }
            }
            Phase::SecondAttackerPerformance => {
                if self.player1.is_first_attacker {
                    &mut self.player2
                } else {
                    &mut self.player1
                }
            }
            _ => match self.current_turn_phase {
                TurnPhase::FirstAttackerNormal => {
                    if self.player1.is_first_attacker {
                        &mut self.player1
                    } else {
                        &mut self.player2
                    }
                }
                TurnPhase::SecondAttackerNormal => {
                    if self.player1.is_first_attacker {
                        &mut self.player2
                    } else {
                        &mut self.player1
                    }
                }
                TurnPhase::Live => {
                    if self.player1.is_first_attacker {
                        &mut self.player1
                    } else {
                        &mut self.player2
                    }
                }
            },
        }
    }

    pub fn first_attacker(&self) -> &Player {
        if self.player1.is_first_attacker {
            &self.player1
        } else {
            &self.player2
        }
    }

    pub fn first_attacker_mut(&mut self) -> &mut Player {
        if self.player1.is_first_attacker {
            &mut self.player1
        } else {
            &mut self.player2
        }
    }

    pub fn second_attacker(&self) -> &Player {
        if self.player1.is_first_attacker {
            &self.player2
        } else {
            &self.player1
        }
    }

    pub fn second_attacker_mut(&mut self) -> &mut Player {
        if self.player1.is_first_attacker {
            &mut self.player2
        } else {
            &mut self.player1
        }
    }

    pub fn non_active_player(&self) -> &Player {
        if std::ptr::eq(self.active_player(), &self.player1) {
            &self.player2
        } else {
            &self.player1
        }
    }

    pub fn non_active_player_mut(&mut self) -> &mut Player {
        if std::ptr::eq(self.active_player(), &self.player1) {
            &mut self.player2
        } else {
            &mut self.player1
        }
    }

    /// Clear tracking fields that transiently live across effect resolution.
    /// Must be called whenever an ability completes (success or failure) and
    /// when post-choice state machine exits.
    pub fn find_card_stage_position(&self, card_id: i16) -> Option<MemberArea> {
        for (idx, &cid) in self.player1.stage.stage.iter().enumerate() {
            if cid == card_id {
                return Some(match idx {
                    0 => MemberArea::LeftSide,
                    1 => MemberArea::Center,
                    2 => MemberArea::RightSide,
                    _ => unreachable!(),
                });
            }
        }
        for (idx, &cid) in self.player2.stage.stage.iter().enumerate() {
            if cid == card_id {
                return Some(match idx {
                    0 => MemberArea::LeftSide,
                    1 => MemberArea::Center,
                    2 => MemberArea::RightSide,
                    _ => unreachable!(),
                });
            }
        }
        None
    }

    pub fn clear_effect_tracking(&mut self) {
        self.batch_movements.clear();
        self.mods.last_cost_discard_count = 0;
        self.mods.last_cost_moved_card_ids.clear();
        self.mods.last_cost_energy_count = 0;
    }

    /// Backward-compat: the card that last moved areas (from turn_area_movements).
    pub fn last_area_move_card_id(&self) -> Option<i16> {
        self.turn_area_movements.last().map(|m| m.moved_card_id)
    }
    /// Backward-compat: which player's effect caused the last area move.
    pub fn last_area_move_by_player(&self) -> Option<&str> {
        self.turn_area_movements
            .last()
            .map(|m| m.cause_player_id.as_str())
    }
    /// Backward-compat: whether energy was placed by a card effect this batch.
    pub fn last_energy_placed_by_effect(&self) -> bool {
        self.batch_movements
            .iter()
            .any(|m| (m.dest_zone == "energy" || m.dest_zone == "energy_zone") && m.effect_only)
    }
    /// Backward-compat: which player's effect caused the last energy placement.
    pub fn last_energy_placed_by_player(&self) -> Option<&str> {
        self.batch_movements
            .iter()
            .find(|m| m.dest_zone == "energy" || m.dest_zone == "energy_zone")
            .map(|m| m.cause_player_id.as_str())
    }

    /// Resolve which player's cheer_revealed_cards to use based on ability master.
    pub fn cheer_revealed_cards_mut(&mut self) -> &mut Vec<i16> {
        match self.ability_master_id().as_deref() {
            Some("player2") | Some("p2") => &mut self.player2_cheer_revealed_cards,
            _ => &mut self.player1_cheer_revealed_cards,
        }
    }

    pub fn cheer_revealed_cards(&self) -> &Vec<i16> {
        match self.ability_master_id().as_deref() {
            Some("player2") | Some("p2") => &self.player2_cheer_revealed_cards,
            _ => &self.player1_cheer_revealed_cards,
        }
    }

    /// Cheer blade heart count, keyed by first/second attacker.
    pub fn cheer_blade_heart_count_mut(&mut self, is_first: bool) -> &mut u32 {
        if is_first {
            &mut self.player1_cheer_blade_heart_count
        } else {
            &mut self.player2_cheer_blade_heart_count
        }
    }

    /// Cheer revealed cards, keyed by first/second attacker.
    pub fn cheer_revealed_cards_first(&mut self, is_first: bool) -> &mut Vec<i16> {
        if is_first {
            &mut self.player1_cheer_revealed_cards
        } else {
            &mut self.player2_cheer_revealed_cards
        }
    }

    /// Push a card to revealed_cards with source/owner/private tracking.
    pub fn push_revealed_card(
        &mut self,
        card_id: i16,
        source_card_id: Option<i16>,
        is_private: bool,
        owner: Option<u8>,
    ) {
        let source_name = source_card_id
            .and_then(|sid| self.card_database.get_card(sid))
            .map(|c| c.name.to_string());
        self.revealed_cards.push(card_id);
        self.revealed_card_sources.push(source_card_id);
        self.revealed_card_source_names.push(source_name);
        self.revealed_card_is_private.push(is_private);
        self.revealed_card_owners.push(owner);
    }

    /// Push a card to revealed_cost_cards with source/owner/private tracking.
    pub fn push_revealed_cost_card(
        &mut self,
        card_id: i16,
        source_card_id: Option<i16>,
        is_private: bool,
        owner: Option<u8>,
    ) {
        let source_name = source_card_id
            .and_then(|sid| self.card_database.get_card(sid))
            .map(|c| c.name.to_string());
        self.revealed_cost_cards.push(card_id);
        self.revealed_cost_card_sources.push(source_card_id);
        self.revealed_cost_card_source_names.push(source_name);
        self.revealed_cost_card_is_private.push(is_private);
        self.revealed_cost_card_owners.push(owner);
    }

    /// Get the source card ID from the current ability queue entry, if any.
    pub fn current_ability_source_card_id(&self) -> Option<i16> {
        self.ability_queue.current_entry().and_then(|e| e.card_id)
    }

    /// Push a log entry to both rule_log and structured_log.
    pub fn log_entry(
        &mut self,
        text: String,
        player_label: &str,
        source_card_id: Option<i16>,
        source_card_name: Option<String>,
        category: &str,
    ) {
        self.rule_log.push(text.clone());
        self.structured_log.push(LogEntry {
            text,
            turn: self.turn_number,
            player_label: player_label.to_string(),
            source_card_id,
            source_card_name,
            category: category.to_string(),
            metadata: None,
        });
    }

    /// Push a log entry using the currently activating card's info.
    pub fn log_ability(&mut self, text: String, category: &str) {
        let pp = self.player_prefix();
        let act_name = self
            .activating_card
            .and_then(|id| self.card_database.get_card(id))
            .map(|c| c.name.to_string());
        self.log_entry(text, &pp, self.activating_card, act_name, category);
    }

    /// Determine the player label (P1/P2) for the activating card.
    pub fn player_prefix(&self) -> String {
        if let Some(card_id) = self.activating_card {
            if self.player1.stage.stage.contains(&card_id)
                || self
                    .player1
                    .stage
                    .under_cards
                    .iter()
                    .any(|uc| uc.contains(&card_id))
            {
                return self.player1.id.clone();
            }
            if self.player2.stage.stage.contains(&card_id)
                || self
                    .player2
                    .stage
                    .under_cards
                    .iter()
                    .any(|uc| uc.contains(&card_id))
            {
                return self.player2.id.clone();
            }
        }
        self.active_player().id.clone()
    }

    /// Snapshot current zone sizes for delta tracking.
    pub fn zone_snapshot(&self) -> std::collections::HashMap<String, usize> {
        let mut m = std::collections::HashMap::new();
        for (prefix, p) in [("P1", &self.player1), ("P2", &self.player2)] {
            m.insert(format!("{}.hand", prefix), p.hand.len());
            m.insert(format!("{}.deck", prefix), p.main_deck.len());
            m.insert(format!("{}.energy_deck", prefix), p.energy_deck.cards.len());
            m.insert(format!("{}.energy_zone", prefix), p.energy_zone.cards.len());
            m.insert(format!("{}.waitroom", prefix), p.waitroom.len());
            m.insert(format!("{}.live_zone", prefix), p.live_card_zone.len());
            m.insert(
                format!("{}.success_live", prefix),
                p.success_live_card_zone.len(),
            );
        }
        m
    }

    /// Log zone deltas from before/after snapshots.
    pub fn log_zone_delta(
        &mut self,
        before: &std::collections::HashMap<String, usize>,
        category: &str,
    ) {
        let after = self.zone_snapshot();
        let mut parts: Vec<String> = Vec::new();
        let all_keys: std::collections::HashSet<String> =
            before.keys().chain(after.keys()).cloned().collect();
        let mut sorted: Vec<&String> = all_keys.iter().collect();
        sorted.sort();
        for key in sorted {
            let b = before.get(key).copied().unwrap_or(0);
            let a = after.get(key).copied().unwrap_or(0);
            if a != b {
                let delta = a as i64 - b as i64;
                let zone_short = key.split('.').nth(1).unwrap_or(key);
                parts.push(format!(
                    "{} {}{}",
                    zone_short,
                    if delta >= 0 { "+" } else { "" },
                    delta
                ));
            }
        }
        if !parts.is_empty() {
            self.log_ability(parts.join(", "), category);
        }
    }

    /// Record an ability application for source-tracking in the performance snapshot.
    /// Called from effect handlers after applying a modifier.
    pub fn record_ability_application(
        &mut self,
        source_card_id: i16,
        ability_text: String,
        effect_type: &str,
        target_card_id: i16,
        heart_color: Option<usize>,
        amount: i32,
    ) {
        self.ability_applications
            .push(crate::types::AbilityApplication {
                source_card_id,
                ability_text: ability_text.into(),
                effect_type: match effect_type {
                    "heart_bonus" => crate::types::EffectType::HeartBonus,
                    "blade_bonus" => crate::types::EffectType::BladeBonus,
                    "score_bonus" => crate::types::EffectType::ScoreBonus,
                    "score_set" => crate::types::EffectType::ScoreSet,
                    "transform" => crate::types::EffectType::Transform,
                    "need_heart_mod" => crate::types::EffectType::NeedHeartMod,
                    "heart_override" => crate::types::EffectType::HeartOverride,
                    _ => crate::types::EffectType::HeartBonus,
                },
                target_card_id,
                heart_color,
                amount,
            });
    }

    /// Print a human-readable execution trace of the last resolved ability.
    pub fn dump_last_trace(&self) {
        if let Some(ref trace) = self.last_ability_trace {
            println!("\n=== LAST RESOLVED ABILITY TRACE ===");
            if let Some(ref card) = trace.card {
                println!("Card: {}", card);
            }
            println!("Ability: {}", trace.label);
            Self::print_trace_node(trace, 0);
            println!("====================================\n");
        } else {
            println!("\n[No ability trace recorded]\n");
        }
    }

    fn print_trace_node(node: &crate::ability::types::AbilityTraceNode, indent: usize) {
        let pad = "  ".repeat(indent);
        let card_str = node
            .card
            .as_deref()
            .map(|c| format!(" [{}]", c))
            .unwrap_or_default();
        println!("{}- {}{}", pad, node.label, card_str);
        if let (Some(ref b), Some(ref a)) = (&node.before, &node.after) {
            let mut changes = Vec::new();
            if b.hand_count != a.hand_count {
                changes.push(format!("Hand: {} -> {}", b.hand_count, a.hand_count));
            }
            if b.stage_count != a.stage_count {
                changes.push(format!("Stage: {} -> {}", b.stage_count, a.stage_count));
            }
            if b.waitroom_count != a.waitroom_count {
                changes.push(format!(
                    "Discard: {} -> {}",
                    b.waitroom_count, a.waitroom_count
                ));
            }
            if b.energy_count != a.energy_count {
                changes.push(format!("Energy: {} -> {}", b.energy_count, a.energy_count));
            }
            if b.active_energy_count != a.active_energy_count {
                changes.push(format!(
                    "Active Energy: {} -> {}",
                    b.active_energy_count, a.active_energy_count
                ));
            }
            if b.deck_count != a.deck_count {
                changes.push(format!("Deck: {} -> {}", b.deck_count, a.deck_count));
            }
            if !changes.is_empty() {
                println!("{}  Deltas: {}", pad, changes.join(", "));
            }
        }
        for child in &node.children {
            Self::print_trace_node(child, indent + 1);
        }
    }
}

include!("tracking.rs");
include!("modifiers.rs");
include!("abilities.rs");
