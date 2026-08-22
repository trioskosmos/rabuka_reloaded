use crate::ability::enums::Zone;
use crate::ability_queue::AbilityQueue;
use crate::card::CardDatabase;
use crate::core::game_modifiers::{CardOrientation, GameModifiers};
use crate::player::Player;
use crate::zones::{MemberArea, ResolutionZone};
use crate::Arc;
use crate::{HashMap, HashSet};
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// Bound on plain-text rule log lines kept per state. Oldest lines are dropped
/// first so a long match cannot grow the buffer without limit.
///
/// The GBA is text-only and never renders a log-history screen, so it must not
/// accumulate a log buffer in RAM at all — the log pushes would otherwise
/// reserve ~116KB just to hold strings that are never shown. Bounds are 0 there
/// (each push is immediately dropped). Other console targets keep a small
/// window; desktop keeps the full history.
#[cfg(feature = "gba")]
pub const LOG_BOUND_RULE: usize = 0;
#[cfg(all(feature = "compact_state", not(feature = "gba")))]
pub const LOG_BOUND_RULE: usize = 60;
#[cfg(not(feature = "compact_state"))]
pub const LOG_BOUND_RULE: usize = 500;
/// Bound on structured log entries kept per state. Oldest entries are dropped
/// first; the UI renders the newest window of game events.
#[cfg(feature = "gba")]
pub const LOG_BOUND_STRUCTURED: usize = 0;
#[cfg(all(feature = "compact_state", not(feature = "gba")))]
pub const LOG_BOUND_STRUCTURED: usize = 60;
#[cfg(not(feature = "compact_state"))]
pub const LOG_BOUND_STRUCTURED: usize = 500;

/// Tracking metadata for a single revealed card, kept in lockstep with the
/// `revealed_cards` / `revealed_cost_cards` id vectors. Consolidates the four
/// parallel `Vec` columns into one struct for better locality and fewer
/// allocator headers.
#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct RevealedCardMeta {
    pub source: Option<i16>,
    pub source_name: Option<String>,
    pub is_private: bool,
    pub owner: Option<u8>,
    pub reveal_type: String,
}

pub use crate::types::{
    AbilityApplication, AbilityBonus, AbilityTrigger, Adjustment, Allocation, BladeSource,
    Breakdown, Duration, EffectEntry, GameResult, HeartSource, LiveCardResult, LivePerformanceData,
    LogEntry, MemberContribution, MovementEvent, PerformanceSnapshot, Phase, ReplacementEffect,
    ScoreLine, TemporaryEffect, TriggeredAbility, TurnPhase, YellCardResult,
};

/// State saved while a play action is paused for a play-time cost-reduction
/// choice (常時「このカードをプレイする際…コストは減る」).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct PlayTimeCostPlay {
    pub card_id: i16,
    pub area: crate::zones::MemberArea,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct GameState {
    // --- 8-byte aligned (Player, HashMap, HashSet, String, Vec, Arc, usize, Value) ---
    pub player1: Player,
    pub player2: Player,
    pub ability_queue: AbilityQueue,
    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub card_database: Arc<CardDatabase>,
    pub mods: GameModifiers,
    pub resolution_zone: ResolutionZone,
    pub heart_color_decision_phase: String,
    /// Engine-internal loop-detection history. Skipped on the wire: the 3DS
    /// client never runs the engine and it only inflates every state transfer.
    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub game_state_history: Vec<u64>,
    pub rule_log: Vec<String>,
    /// Engine-internal structured log. Skipped on the wire: the client's game
    /// log overlay reads `rule_log`, not this.
    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub structured_log: Vec<LogEntry>,
    /// Always-on compact event trace for debugging (tests, support dumps).
    /// Records phase transitions, trigger firings and ability-queue outcomes
    /// as short strings. Capped ring: keeps the most recent
    /// [`DEBUG_TRACE_CAP`] entries. Skipped on the wire.
    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub debug_trace: Vec<String>,
    pub turn1_abilities_played: SmallVec<[String; 8]>,
    pub turn2_abilities_played: SmallVec<[(String, u8); 8]>,
    pub live_owned_hearts: SmallVec<[(String, Vec<(String, u8)>); 4]>,
    pub temporary_effects: SmallVec<[TemporaryEffect; 4]>,
    pub prohibition_effects: SmallVec<[String; 4]>,
    pub delayed_prohibition_effects: SmallVec<[String; 4]>,
    pub non_stackable_effects: SmallVec<[String; 16]>,
    pub cannot_activate_members: SmallVec<[String; 2]>,
    pub constant_cannot_activate_members: SmallVec<[String; 4]>,
    pub cannot_live_players: SmallVec<[String; 2]>,
    /// Member cards that are immune to being put to WAIT by an OPPONENT's effect
    /// ("相手の効果によってはウェイトしない"), recorded as (member_id, owner_id) by
    /// the `cannot_wait_by_effect` restriction. Cleared at the end of the live.
    pub wait_immune_members: SmallVec<[(i16, String); 8]>,
    pub turn_limited_abilities_used: HashMap<(i16, usize, u8), u8>,
    pub mulligan_selected_indices: SmallVec<[u8; 6]>,
    pub live_card_selected_indices: SmallVec<[u8; 3]>,
    pub auto_ability_trigger_counts: SmallVec<[(String, u8); 8]>,
    pub turn_limit_usage: SmallVec<[(String, u8); 8]>,
    pub card_instance_mapping: HashMap<i16, u8>,
    pub areas_placed_this_turn: SmallVec<[String; 8]>,
    pub cards_appeared_this_turn: SmallVec<[i16; 8]>,
    pub card_appearance_source: SmallVec<[(i16, String); 4]>,
    pub cards_moved_this_turn: SmallVec<[i16; 16]>,
    pub gained_abilities: HashMap<i16, Vec<String>>,
    /// Full Ability structs dynamically added to cards via gain_ability.
    /// These are scanned by the trigger pipeline alongside original card abilities.
    pub gained_card_abilities: HashMap<i16, Vec<crate::card::Ability>>,
    /// Gained effects that couldn't be evaluated at constant time (e.g. because
    /// they depend on revealed_cards from the yell).  Stored as `(card_id, effect)`
    /// and evaluated during `execute_live_victory_determination` when the yell
    /// results are available.
    pub delayed_gained_effects: SmallVec<[(i16, crate::card::AbilityEffect); 2]>,
    /// Scratch buffers for recalculate_constants — reused across calls to avoid
    /// allocating 7+ HashMaps/Vecs on every state change. Swapped out via
    /// `core::mem::take` at the start of each call and swapped back at the end.
    /// All skipped on the wire: transient engine-internal buffers.
    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub scratch_exp_blade: HashMap<i16, i16>,
    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub scratch_exp_score: HashMap<i16, i16>,
    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub scratch_exp_heart: HashMap<i16, HashMap<String, i16>>,
    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub scratch_entry_positions: HashMap<i16, Option<u8>>,
    pub negated_abilities: SmallVec<[i16; 8]>,
    pub replacement_effects: SmallVec<[ReplacementEffect; 2]>,
    pub constant_ability_statuses: SmallVec<[crate::types::ConstantAbilityStatus; 6]>,
    pub revealed_cards: SmallVec<[i16; 8]>,
    pub revealed_card_meta: SmallVec<[RevealedCardMeta; 8]>,
    pub revealed_cost_cards: SmallVec<[i16; 8]>,
    pub revealed_cost_card_meta: SmallVec<[RevealedCardMeta; 8]>,
    pub player1_cheer_revealed_cards: SmallVec<[i16; 8]>,
    pub player2_cheer_revealed_cards: SmallVec<[i16; 8]>,
    /// Cards revealed by the initial yell (saved before re-yell overwrites them).
    pub initial_yell_revealed_cards: SmallVec<[i16; 8]>,
    /// Cards revealed by a re-yell (set after perform_yell draws new cards).
    pub re_yell_revealed_cards: SmallVec<[i16; 8]>,
    /// Pending play-time cost-reduction resume state (常時「このカードをプレイする際
    /// …コストは減る」). Set when a play action offers the optional reduction choice;
    /// consumed on the re-entered play action after the choice is answered.
    pub play_time_cost_play: Option<PlayTimeCostPlay>,
    /// Whether the player accepted the play-time cost reduction choice.
    pub play_time_cost_reduction_accepted: Option<bool>,
    pub looked_at_cards: SmallVec<[i16; 8]>,
    pub ability_applications: SmallVec<[crate::types::AbilityApplication; 4]>,
    /// Synced from batch_movements by push_movement_event().
    pub recently_moved_cards: Option<SmallVec<[i16; 4]>>,
    pub recently_appeared_cards: SmallVec<[i16; 4]>,
    pub recently_moved_from_zone: Option<String>,
    /// Explicit per-batch event log of stage-area-to-stage-area position changes.
    /// Each entry records the moved card, old/new position, and cause info.
    /// Replaces the fragile snapshot-based detection with direct event tracking.
    /// Cleared at the end of each ability batch process (after post-loop TAS scan
    /// in process_player_abilities), NOT in clear_effect_tracking.
    pub position_change_events: SmallVec<[crate::types::PositionChangeEvent; 2]>,

    /// Detailed event log for per-batch tracking: cards moved in the current
    /// cost/effect batch, what caused the move (cause_card_id), etc.
    /// recently_moved_cards/from_zone are synced from this vec.
    pub batch_movements: SmallVec<[MovementEvent; 4]>,
    /// Turn-level record of stage-area-to-stage-area movements.
    /// Used by conditions checking "this member has moved areas this turn".
    pub turn_area_movements: SmallVec<[MovementEvent; 4]>,
    /// Turn-level record of ALL zone-to-zone movements.
    /// Accumulated across the entire turn (not cleared between ability batches).
    /// Used by movement conditions with source+destination zones
    /// (e.g. "member card went from live_card_zone to discard this turn").
    pub turn_movements: SmallVec<[MovementEvent; 8]>,
    /// Counter for assigning unique timestamps to MovementEvents within a turn.
    pub movement_event_counter: u16,
    /// Snapshot of target cards' orientations taken before a change_state
    /// effect executes. Compared after the effect to detect actual transitions.
    /// None = no snapshot active.
    pub state_snapshot_before_change: Option<HashMap<i16, Option<CardOrientation>>>,
    /// After a change_state effect executes, records what actually changed:
    /// (card_id, from_state, to_state). Cleared after post-resolution TAS scan.
    pub recently_state_changed: SmallVec<[(i16, String, String); 2]>,
    pub debut_ability_triggers: SmallVec<[(String, i16); 4]>,
    pub last_vacated_stage_area: Option<u8>,
    // --- 4-byte aligned (u8, Option<i32>) ---
    pub turn_number: u8,
    pub live_cheer_count: u8,
    pub player1_cheer_blade_heart_count: u8,
    pub player2_cheer_blade_heart_count: u8,
    pub cheer_checks_required: u8,
    /// Blade count of the current live, set when the first cheer check runs.
    /// `None` between lives. The required count is derived from this base
    /// plus `yell_count_modifiers` — never mutated directly by effects.
    pub cheer_check_base: Option<u8>,
    /// All modify_yell_count effects as (player_slot 1|2, delta). LiveEnd-
    /// scoped: cleared together with the base when a live-scoped effect
    /// expires. Derived total = max(0, base + Σ deltas for the player).
    pub yell_count_modifiers: SmallVec<[(u8, i32); 4]>,
    pub cheer_checks_done: u8,
    pub card_instance_counter: u8,
    pub baton_touch_count_p1: u8,
    pub baton_touch_count_p2: u8,
    pub baton_touch_arriving_card_ids: SmallVec<[i16; 2]>,
    pub effect_creation_counter: u8,
    pub last_state_change_wait_to_active_count: u8,
    pub last_cost_wait_member: Option<i16>,
    pub player1_rps_choice: Option<u8>,
    pub player2_rps_choice: Option<u8>,
    pub baton_touch_replaced_member_cost: Option<u8>,
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
    /// Encoded as `(card_id as u8) << 16 | ability_index as u8`.
    pub just_completed_ability_key: Option<u32>,
    /// Batch-scoped set of ability IDs already enqueued during the current movement batch.
    /// Prevents each_time/movement abilities from being re-enqueued across multiple
    /// post-resolution TAS scans within the same batch. Cleared at post-loop batch scan.
    /// Each entry is `(card_id as u32) << 16 | ability_index as u32`.
    pub this_batch_triggered_ability_ids: SmallVec<[u32; 16]>,
    /// Cutoff index for depth-first each_time drain. Entries enqueued at >= this index
    /// are newly-triggered (each_time watchers) and must be force-resolved before
    /// stale entries are offered to the player. Set by process_player_abilities and
    /// resume_queue_with_choice before calling process_current_ability.
    pub depth_first_cutoff: Option<u16>,
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
    pub opponent_live_surplus_count: u8,
    pub self_live_surplus_count: u8,
    pub formation_change_occurred_this_turn: bool,
    pub opponent_choice_declined: bool,
    pub live_being_performed: bool,
    pub game_ended: bool,
    pub draw_state: bool,
    pub loop_detected: bool,
    pub live_success_triggered_this_turn: bool,
    pub live_success_p2_fired: bool,
    pub live_success_p1_extra: u8,
    pub live_success_p2_extra: u8,
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
    pub pending_rps_player_id: Option<u8>,
}

impl GameState {
    /// Whether the opponent (the player at index `1 - my_player_idx`) has
    /// performed their live this turn. Used to decide when the opponent's live
    /// zone / need-hearts may be revealed to the local player.
    pub fn opponent_has_performed(&self, my_player_idx: usize) -> bool {
        let opp_idx = 1 - my_player_idx;
        let opp_first = if opp_idx == 0 {
            self.player1.is_first_attacker
        } else {
            self.player2.is_first_attacker
        };
        matches!(
            self.current_phase,
            Phase::SecondAttackerPerformance | Phase::LiveVictoryDetermination
        ) || (matches!(self.current_phase, Phase::FirstAttackerPerformance) && opp_first)
    }

    /// Check if the given player (0=P1, 1=P2) can act right now.
    /// Accounts for pending choices (including SelectAutoAbility/SelectLiveSuccess),
    /// phase-specific rules, and active player checks.
    pub fn can_player_act(&self, player_id: i32) -> bool {
        use crate::ability::types::Choice;
        let pid_str = || if player_id == 0 { "p1" } else { "p2" };
        if self.has_pending_choice() {
            if let Some(cpid) = self.get_pending_choice_player_id() {
                return cpid == pid_str();
            }
            if let Some(choice) = self.get_pending_choice() {
                match choice {
                    Choice::SelectAutoAbility {
                        player_id: cpid, ..
                    }
                    | Choice::SelectLiveSuccess {
                        player_id: cpid, ..
                    } => {
                        return *cpid == pid_str();
                    }
                    _ => {}
                }
            }
        }
        match self.current_phase {
            Phase::RockPaperScissors => {
                if player_id == 0 {
                    self.player1_rps_choice.is_none()
                } else {
                    self.player2_rps_choice.is_none()
                }
            }
            Phase::ChooseFirstAttacker => {
                let winner_idx = self.rps_winner;
                winner_idx == Some(if player_id == 0 { 1 } else { 2 })
            }
            Phase::MulliganFirstAttacker
            | Phase::LiveCardSetFirstAttacker
            | Phase::FirstAttackerPerformance => {
                (self.player1.is_first_attacker && player_id == 0)
                    || (!self.player1.is_first_attacker && player_id == 1)
            }
            Phase::MulliganSecondAttacker
            | Phase::LiveCardSetSecondAttacker
            | Phase::SecondAttackerPerformance => {
                (self.player1.is_first_attacker && player_id == 1)
                    || (!self.player1.is_first_attacker && player_id == 0)
            }
            _ => {
                let active = self.active_player();
                (active.id == self.player1.id) == (player_id == 0)
            }
        }
    }

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
            rule_log: Vec::new(),
            structured_log: Vec::new(),
            debug_trace: Vec::new(),
            turn1_abilities_played: SmallVec::new(),
            turn2_abilities_played: SmallVec::new(),
            live_owned_hearts: SmallVec::new(),
            temporary_effects: SmallVec::new(),
            prohibition_effects: SmallVec::new(),
            delayed_prohibition_effects: SmallVec::new(),
            non_stackable_effects: SmallVec::new(),
            cannot_activate_members: SmallVec::new(),
            constant_cannot_activate_members: SmallVec::new(),
            cannot_live_players: SmallVec::new(),
    wait_immune_members: SmallVec::new(),
            turn_limited_abilities_used: HashMap::default(),
            mulligan_selected_indices: SmallVec::new(),
            live_card_selected_indices: SmallVec::new(),
            auto_ability_trigger_counts: SmallVec::new(),
            turn_limit_usage: SmallVec::new(),
            card_instance_mapping: HashMap::default(),
            areas_placed_this_turn: SmallVec::new(),
            cards_appeared_this_turn: SmallVec::new(),
            card_appearance_source: SmallVec::new(),
            cards_moved_this_turn: SmallVec::new(),
            gained_abilities: HashMap::default(),
            gained_card_abilities: HashMap::default(),
            delayed_gained_effects: SmallVec::new(),
            scratch_exp_blade: HashMap::default(),
            scratch_exp_score: HashMap::default(),
            scratch_exp_heart: HashMap::default(),
            scratch_entry_positions: HashMap::default(),
            negated_abilities: SmallVec::new(),
            replacement_effects: SmallVec::new(),
            constant_ability_statuses: SmallVec::new(),
            revealed_cards: SmallVec::new(),
            revealed_card_meta: SmallVec::new(),
            revealed_cost_cards: SmallVec::new(),
            revealed_cost_card_meta: SmallVec::new(),
            player1_cheer_revealed_cards: SmallVec::new(),
            player2_cheer_revealed_cards: SmallVec::new(),
            initial_yell_revealed_cards: SmallVec::new(),
            re_yell_revealed_cards: SmallVec::new(),
            play_time_cost_play: None,
            play_time_cost_reduction_accepted: None,
            looked_at_cards: SmallVec::new(),
            ability_applications: SmallVec::new(),
            recently_moved_cards: None,
            recently_appeared_cards: SmallVec::new(),
            recently_moved_from_zone: None,
            position_change_events: SmallVec::new(),
            batch_movements: SmallVec::new(),
            turn_area_movements: SmallVec::new(),
            turn_movements: SmallVec::new(),
            movement_event_counter: 0,
            state_snapshot_before_change: None,
            recently_state_changed: SmallVec::new(),
            debut_ability_triggers: SmallVec::new(),
            last_vacated_stage_area: None,
            // 4-byte aligned
            turn_number: 1,
            live_cheer_count: 0,
            player1_cheer_blade_heart_count: 0,
            player2_cheer_blade_heart_count: 0,
            cheer_checks_required: 0,
            cheer_check_base: None,
            yell_count_modifiers: SmallVec::new(),
            cheer_checks_done: 0,
            card_instance_counter: 0,
            baton_touch_count_p1: 0,
            baton_touch_count_p2: 0,
            baton_touch_arriving_card_ids: SmallVec::new(),
            effect_creation_counter: 0,
            last_state_change_wait_to_active_count: 0,
        last_cost_wait_member: None,
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
            this_batch_triggered_ability_ids: SmallVec::new(),
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
    /// The last member card that was put into wait state as an ability cost
    /// (Q266 dynamic blade-limit reference: limit = its original blade + offset).
    pub fn last_cost_wait_member(&self) -> Option<i16> {
        self.last_cost_wait_member
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
            .any(|m| (m.dest_zone == "energy" || m.dest_zone == "energy_zone" || m.dest_zone == "under_member") && m.effect_only)
    }
    /// Backward-compat: which player's effect caused the last energy placement.
    pub fn last_energy_placed_by_player(&self) -> Option<&str> {
        self.batch_movements
            .iter()
            .find(|m| m.dest_zone == "energy" || m.dest_zone == "energy_zone" || m.dest_zone == "under_member")
            .map(|m| m.cause_player_id.as_str())
    }

    pub fn cheer_revealed_cards(&self) -> &SmallVec<[i16; 8]> {
        match self.ability_master_id().as_deref() {
            Some("player2") | Some("p2") => &self.player2_cheer_revealed_cards,
            _ => &self.player1_cheer_revealed_cards,
        }
    }

    /// Push a card to revealed_cards with source/owner/private tracking.
    pub fn push_revealed_card(
        &mut self,
        card_id: i16,
        source_card_id: Option<i16>,
        is_private: bool,
        owner: Option<u8>,
        reveal_type: &'static str,
    ) {
        let source_name = source_card_id
            .and_then(|sid| self.card_database.get_card(sid))
            .map(|c| c.name.to_string());
        self.revealed_cards.push(card_id);
        self.revealed_card_meta.push(RevealedCardMeta {
            source: source_card_id,
            source_name,
            is_private,
            owner,
            reveal_type: reveal_type.to_string(),
        });
    }

    /// Push a card to revealed_cost_cards with source/owner/private tracking.
    pub fn push_revealed_cost_card(
        &mut self,
        card_id: i16,
        source_card_id: Option<i16>,
        is_private: bool,
        owner: Option<u8>,
        reveal_type: &'static str,
    ) {
        let source_name = source_card_id
            .and_then(|sid| self.card_database.get_card(sid))
            .map(|c| c.name.to_string());
        self.revealed_cost_cards.push(card_id);
        self.revealed_cost_card_meta.push(RevealedCardMeta {
            source: source_card_id,
            source_name,
            is_private,
            owner,
            reveal_type: reveal_type.to_string(),
        });
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
        self.push_rule_log(text.clone());
        if !crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            return;
        }
        self.push_structured_log(LogEntry {
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
    pub fn zone_snapshot(&self) -> HashMap<String, usize> {
        let mut m = HashMap::default();
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

    /// Commit an ability resolution to the structured log deterministically.
    ///
    /// Finds the NEWEST matching `trigger_evaluation` entry for this
    /// `(card_id, trigger_str, ability_index)` and overwrites its metadata in
    /// place (single entry — no orphaned "pending" copy). Unlike the old logic
    /// this does NOT require `entry.turn == current turn`, so abilities that
    /// resolve across a turn boundary commit correctly. If no matching trigger
    /// entry exists (e.g. constant/activation abilities with no scan entry),
    /// falls back to pushing the supplied `fallback_entry`.
    pub fn commit_or_push_structured(
        &mut self,
        card_id: Option<i16>,
        trigger_str: &str,
        ability_index: Option<usize>,
        meta: crate::core::types::LogMetadata,
        fallback_entry: crate::types::LogEntry,
    ) {
        if let Some(cid) = card_id {
            for entry in self.structured_log.iter_mut().rev() {
                if entry.category != "trigger_evaluation" {
                    continue;
                }
                if entry.source_card_id != Some(cid) {
                    continue;
                }
                let (t, ei) = match &entry.metadata {
                    Some(crate::core::types::LogMetadata::TriggerEvaluation {
                        trigger,
                        ability_index,
                        ..
                    }) => (trigger.as_str(), Some(*ability_index)),
                    _ => continue,
                };
                if t != trigger_str {
                    continue;
                }
                if let (Some(ai), Some(ei)) = (ability_index, ei) {
                    if ai != ei {
                        continue;
                    }
                }
                // Found the newest matching trigger — commit in place. Replace
                // the text too so downstream consumers (e.g. tests probing for
                // "trigger_debut") see the resolution line, not the scan line.
                entry.metadata = Some(meta);
                entry.text = fallback_entry.text;
                return;
            }
        }
        // No matching pending trigger (or no card id): push a standalone entry.
        self.push_structured_log(fallback_entry);
    }

    /// Whether a zone is 非公開領域 (private / hidden): its card *identities*
    /// must never appear in a shared log. Per rules 4.8 (main deck), 4.9
    /// (energy deck) and 4.11 (hand). 公開領域 (stage, live zone, energy zone,
    /// success zone, waitroom/discard, revealed, etc.) are public.
    pub fn zone_is_private(&self, zone: &str) -> bool {
        matches!(
            zone,
            "hand" | "deck" | "deck_top" | "deck_bottom" | "main_deck" | "energy_deck"
        )
    }

    /// Resolve a card index to a readable label, checking either player.
    /// Returns the card NAME only for public (公開領域) zones. For private zones
    /// a neutral placeholder is returned so shared logs never leak identities.
    pub fn resolve_index_any_player(&self, zone: &str, idx: usize) -> String {
        if self.zone_is_private(zone) {
            return format!("#{idx}");
        }
        let p1 = self.resolve_card(&self.player1, zone, idx);
        if p1 != format!("#{idx}") {
            return p1;
        }
        self.resolve_card(&self.player2, zone, idx)
    }

    fn resolve_card(&self, player: &crate::player::Player, zone: &str, idx: usize) -> String {
        use crate::ability::util::resolve_indices_to_ids;
        let ids = resolve_indices_to_ids(player, zone, &[idx]);
        ids.first()
            .and_then(|&cid| self.card_database.get_card(cid))
            .map(|c| c.name.as_ref().to_string())
            .unwrap_or_else(|| format!("#{idx}"))
    }

    /// Human-readable labels for the options offered by a choice, resolving any
    /// card indices to real card names where possible.
    pub fn choice_offered_labels(&self, choice: &crate::ability::types::Choice) -> Vec<String> {
        use crate::ability::types::Choice as C;
        match choice {
            C::SelectCard {
                zone,
                count,
                description,
                card_type,
                filtered_indices,
                ..
            } => {
                let mut out = vec![format!("[{zone}] select {count} card(s): {description}")];
                if let Some(ct) = card_type {
                    out.push(format!("  type: {ct}"));
                }
                if let Some(fi) = filtered_indices {
                    for &idx in fi {
                        out.push(format!("  - {}", self.resolve_index_any_player(zone, idx)));
                    }
                }
                if filtered_indices.is_none() {
                    out.push(format!("  - candidates from {zone}"));
                }
                out
            }
            C::SelectTarget {
                description,
                options,
                allow_skip,
                ..
            } => {
                let mut out = vec![description.clone()];
                if let Some(opts) = options {
                    out.extend(opts.iter().map(|o| format!("  - {o}")));
                }
                out.push(format!("  (skip_allowed={allow_skip})"));
                out
            }
            C::SelectPosition {
                description,
                allow_skip,
                ..
            } => vec![
                format!("{description} (left/center/right)"),
                format!("  (skip allowed={allow_skip})"),
            ],
            C::SelectHeartColor { count, options, .. } => {
                let mut out = vec![format!("select {count} heart color(s)")];
                out.extend(options.iter().map(|o| format!("  - {o}")));
                out
            }
            C::SelectHeartType { count, options, .. } => {
                let mut out = vec![format!("select {count} heart type(s)")];
                out.extend(options.iter().map(|o| format!("  - {o}")));
                out
            }
            C::SelectAutoAbility {
                options, description, ..
            } => {
                let mut out = vec![description.clone()];
                for o in options {
                    out.push(format!("  - {} ({})", o.card_name, o.ability_text));
                }
                out
            }
            C::SelectLiveSuccess {
                options, description, ..
            } => {
                let mut out = vec![description.clone()];
                for o in options {
                    out.push(format!("  - {}", o.card_name));
                }
                out
            }
        }
    }

    /// Push a `choice_offered` structured entry capturing the options presented
    /// to the player at the moment the choice is stored/committed. Provides the
    /// "offered" half of the offer→resolve pairing in the log.
    pub fn push_choice_offered(&mut self, choice: &crate::ability::types::Choice) {
        let offered = self.choice_offered_labels(choice);
        let skip_allowed = choice.allow_skip();
        let entry = crate::types::LogEntry {
            text: format!(
                "[choice] offered: {} option(s){}",
                offered.len(),
                if skip_allowed { " (skip allowed)" } else { "" }
            ),
            turn: self.turn_number,
            player_label: self.player_prefix(),
            source_card_id: self.activating_card,
            source_card_name: self
                .activating_card
                .and_then(|id| self.card_database.get_card(id))
                .map(|c| c.name.as_ref().to_string()),
            category: "choice_offered".to_string(),
            metadata: Some(crate::core::types::LogMetadata::ChoiceOffered {
                offered,
                skip_allowed,
            }),
        };
        self.push_structured_log(entry);
    }

    /// Push a `choice_resolved` structured entry capturing what was offered and
    /// what the player actually picked.
    pub fn push_choice_resolved(
        &mut self,
        choice: &crate::ability::types::Choice,
        chosen: Vec<String>,
        skipped: bool,
    ) {
        // Headless bot playouts never render choice history — skip the
        // label/format work entirely.
        if cfg!(feature = "headless") {
            return;
        }
        let offered_count = self.choice_offered_labels(choice).len();
        let chosen_final = if skipped {
            vec!["skip".to_string()]
        } else {
            chosen
        };
        let entry = crate::types::LogEntry {
            text: format!(
                "[choice] resolved: offered {} option(s), picked {}",
                offered_count,
                chosen_final.join(", ")
            ),
            turn: self.turn_number,
            player_label: self.player_prefix(),
            source_card_id: self.activating_card,
            source_card_name: self
                .activating_card
                .and_then(|id| self.card_database.get_card(id))
                .map(|c| c.name.as_ref().to_string()),
            category: "choice_resolved".to_string(),
            metadata: Some(crate::core::types::LogMetadata::ChoiceResolved {
                offered_count,
                chosen: chosen_final.clone(),
                skipped,
            }),
        };
        self.push_structured_log(entry);
        // Also emit a plain-text line so text-only log consumers (e.g. the 3DS
        // game log history) surface the choice result without structured data.
        self.push_rule_log(format!(
            "[choice] resolved: offered {} option(s), picked {}",
            offered_count,
            chosen_final.join(", ")
        ));
    }

    /// Push a line to the rule log. Bounded to a fixed window so memory can't
    /// grow unbounded during a long match.
    pub fn push_rule_log(&mut self, text: String) {
        Self::push_rule_log_to(&mut self.rule_log, text);
    }

    /// Push an entry to the structured log. Bounded to a fixed window (see
    /// `LOG_BOUND_STRUCTURED`); the newest entries are kept.
    pub fn push_structured_log(&mut self, entry: crate::types::LogEntry) {
        Self::push_structured_log_to(&mut self.structured_log, entry);
    }

    /// Append a compact event to the always-on [`Self::debug_trace`] ring.
    /// Prefixes the turn number so dumps read chronologically across turns.
    /// Use for facts a test/support dump needs: phase transitions, trigger
    /// firings, queue outcomes — not per-card hot-loop noise.
    pub fn push_debug_note(&mut self, text: String) {
        const DEBUG_TRACE_CAP: usize = 600;
        self.debug_trace.push(format!("[T{}]", self.turn_number));
        let last = self.debug_trace.len() - 1;
        self.debug_trace[last].push_str(&text);
        if self.debug_trace.len() > DEBUG_TRACE_CAP {
            self.debug_trace
                .drain(0..self.debug_trace.len() - DEBUG_TRACE_CAP);
        }
    }

    /// True when any debug-trace event contains `needle`.
    pub fn debug_trace_contains(&self, needle: &str) -> bool {
        self.debug_trace.iter().any(|e| e.contains(needle))
    }

    /// Field-level helper: push to a rule log Vec, keeping only the newest
    /// `LOG_BOUND_RULE` lines.
    pub fn push_rule_log_to(log: &mut Vec<String>, text: String) {
        log.push(text);
        Self::truncate_rule_log(log);
    }

    /// Field-level helper: push to a structured log Vec, keeping only the newest
    /// `LOG_BOUND_STRUCTURED` entries.
    pub fn push_structured_log_to(
        log: &mut Vec<crate::types::LogEntry>,
        entry: crate::types::LogEntry,
    ) {
        log.push(entry);
        Self::truncate_structured_log(log);
    }

    fn truncate_rule_log(log: &mut Vec<String>) {
        if log.len() > LOG_BOUND_RULE {
            log.drain(0..log.len() - LOG_BOUND_RULE);
        }
    }

    fn truncate_structured_log(log: &mut Vec<crate::types::LogEntry>) {
        if log.len() > LOG_BOUND_STRUCTURED {
            log.drain(0..log.len() - LOG_BOUND_STRUCTURED);
        }
    }

    /// Push a performance snapshot.
    pub fn push_performance_snapshot(&mut self, snap: crate::types::PerformanceSnapshot) {
        self.performance_snapshots.push(snap);
    }

    /// Record an ability application for source-tracking in the performance snapshot.
    /// Called from effect handlers after applying a modifier.
    pub fn record_ability_application(
        &mut self,
        source_card_id: i16,
        ability_text: String,
        effect_type: &str,
        target_card_id: i16,
        heart_color: Option<u8>,
        amount: i16,
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
    #[cfg(not(feature = "no_std"))]
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

    #[cfg(not(feature = "no_std"))]
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

// Real child modules now (were textual include!() splices). Each contains
// inherent `impl GameState` blocks split out of this file.
mod abilities;
mod modifiers;
mod tracking;

#[cfg(test)]
mod serde_roundtrip_tests {
    use super::*;
    use crate::card::{Card, CardDatabase};
    use crate::player::Player;
    use crate::zones::Stage;
    use crate::Arc;

    fn test_gs() -> GameState {
        let db = CardDatabase::new();
        let mut p1 = Player::new("p1".into(), "P1".into(), true);
        let mut p2 = Player::new("p2".into(), "P2".into(), false);
        p1.stage = Stage::new();
        p2.stage = Stage::new();
        let mut gs = GameState::new(p1, p2, Arc::new(db));
        gs.current_phase = Phase::Main;
        gs.turn_number = 3;
        gs
    }

    #[test]
    fn game_state_roundtrips_through_rmp() {
        let mut gs = test_gs();
        gs.player1.hand.cards.push(1);
        gs.player2.stage.stage[1] = 5;
        let bytes = rmp_serde::to_vec(&gs).unwrap();
        let mut back: GameState = rmp_serde::from_slice(&bytes).unwrap();
        // card_database is #[serde(skip)] — caller reattaches its own
        back.card_database = gs.card_database.clone();
        assert_eq!(gs.player1.hand.cards, back.player1.hand.cards);
        assert_eq!(gs.player2.stage.stage, back.player2.stage.stage);
        assert_eq!(gs.turn_number, back.turn_number);
        assert_eq!(gs.current_phase, back.current_phase);
    }
}
