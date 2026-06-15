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
    LogEntry, MemberContribution, PerformanceSnapshot, Phase, ReplacementEffect, ScoreLine,
    TemporaryEffect, TriggeredAbility, TurnPhase, YellCardResult,
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
    pub game_state_history: Vec<String>,
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
    pub constant_cannot_activate_members: Vec<String>,
    pub turn_limited_abilities_used: std::collections::HashSet<String>,
    pub mulligan_selected_indices: Vec<usize>,
    pub auto_ability_trigger_counts: std::collections::HashMap<String, u32>,
    pub turn_limit_usage: std::collections::HashMap<String, u32>,
    pub card_instance_mapping: std::collections::HashMap<i16, u32>,
    pub areas_placed_this_turn: std::collections::HashSet<String>,
    pub cards_appeared_this_turn: std::collections::HashSet<i16>,
    pub cards_moved_this_turn: std::collections::HashSet<i16>,
    pub gained_abilities: std::collections::HashMap<i16, Vec<String>>,
    pub negated_abilities: std::collections::HashSet<i16>,
    pub replacement_effects: Vec<ReplacementEffect>,
    pub revealed_cards: Vec<i16>,
    pub revealed_cost_cards: Vec<i16>,
    pub player1_cheer_revealed_cards: Vec<i16>,
    pub player2_cheer_revealed_cards: Vec<i16>,
    pub looked_at_cards: Vec<i16>,
    pub ability_applications: Vec<crate::types::AbilityApplication>,
    pub recently_moved_cards: Option<Vec<i16>>,
    /// The zone the cards in `recently_moved_cards` were moved FROM.
    /// Used to distinguish e.g. hand-to-waitroom from stage-to-waitroom (baton touch).
    pub recently_moved_from_zone: Option<String>,
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
    pub baton_touch_count: u32,
    pub effect_creation_counter: u32,
    pub last_state_change_wait_to_active_count: u32,
    pub player1_rps_choice: Option<i32>,
    pub player2_rps_choice: Option<i32>,
    pub baton_touch_replaced_member_cost: Option<u32>,
    pub baton_touch_replaced_member_id: Option<i16>,
    pub baton_touch_arriving_card_id: Option<i16>,
    /// Tracking for movement_condition "moves": the card that last moved areas.
    pub last_area_move_card_id: Option<i16>,
    /// Which player's effect caused the last area move (player_id string).
    pub last_area_move_by_player: Option<String>,
    /// Whether energy was placed by a card effect (vs energy phase draw).
    pub last_energy_placed_by_effect: bool,
    // --- 2-byte aligned (i16, Option<i16>) ---
    pub activating_card: Option<i16>,
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
    pub live_surplus_ready_this_turn: bool,
    pub performance_snapshots: Vec<PerformanceSnapshot>,
    /// Trace from the last ability resolution (for debugging).
    pub last_ability_trace: Option<crate::ability::types::AbilityTraceNode>,
    /// Card ID being replaced by a success zone replacement effect (e.g. 錯覚CROSSROADS).
    pub pending_success_replacement_card_id: Option<i16>,
    /// Player ID of the player making the success zone replacement choice.
    pub pending_success_replacement_player_id: Option<String>,
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
            constant_cannot_activate_members: Vec::new(),
            turn_limited_abilities_used: std::collections::HashSet::new(),
            mulligan_selected_indices: Vec::new(),
            auto_ability_trigger_counts: std::collections::HashMap::new(),
            turn_limit_usage: std::collections::HashMap::new(),
            card_instance_mapping: std::collections::HashMap::new(),
            areas_placed_this_turn: std::collections::HashSet::new(),
            cards_appeared_this_turn: std::collections::HashSet::new(),
            cards_moved_this_turn: std::collections::HashSet::new(),
            gained_abilities: std::collections::HashMap::new(),
            negated_abilities: std::collections::HashSet::new(),
            replacement_effects: Vec::new(),
            revealed_cards: Vec::new(),
            revealed_cost_cards: Vec::new(),
            player1_cheer_revealed_cards: Vec::new(),
            player2_cheer_revealed_cards: Vec::new(),
            looked_at_cards: Vec::new(),
            ability_applications: Vec::new(),
            recently_moved_cards: None,
            recently_moved_from_zone: None,
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
            baton_touch_count: 0,
            effect_creation_counter: 0,
            last_state_change_wait_to_active_count: 0,
            player1_rps_choice: None,
            player2_rps_choice: None,
            baton_touch_replaced_member_cost: None,
            baton_touch_replaced_member_id: None,
            baton_touch_arriving_card_id: None,
            last_area_move_card_id: None,
            last_area_move_by_player: None,
            last_energy_placed_by_effect: false,
            // 2-byte aligned
            activating_card: None,
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
            live_surplus_ready_this_turn: false,
            performance_snapshots: Vec::new(),
            last_ability_trace: None,
            pending_success_replacement_card_id: None,
            pending_success_replacement_player_id: None,
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
        self.last_area_move_card_id = None;
        self.last_area_move_by_player = None;
        self.last_energy_placed_by_effect = false;
        self.recently_moved_cards = None;
        self.recently_moved_from_zone = None;
        self.mods.last_cost_discard_count = 0;
        self.mods.last_cost_energy_count = 0;
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
        });
    }

    /// Push a log entry using the currently activating card's info.
    pub fn log_ability(&mut self, text: String, category: &str) {
        let pp = self.player_prefix();
        let act_name = self
            .activating_card
            .and_then(|id| self.card_database.get_card(id))
            .map(|c| c.name.clone());
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
                ability_text,
                effect_type: effect_type.to_string(),
                target_card_id,
                heart_color,
                amount,
            });
    }
}

include!("tracking.rs");
include!("modifiers.rs");
include!("abilities.rs");
