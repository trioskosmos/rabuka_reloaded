use crate::card::CardDatabase;
use crate::constants::DEFAULT_HISTORY_SIZE;
use crate::core::game_modifiers::GameModifiers;
use crate::player::Player;
use crate::zones::ResolutionZone;
use crate::ability_queue::AbilityQueue;
use std::sync::Arc;

pub use crate::types::{AbilityTrigger, Duration, GameResult, Phase, ReplacementEffect, TemporaryEffect, TurnPhase,
    PerformanceSnapshot, LiveCardResult, MemberContribution, YellCardResult, Breakdown,
    HeartSource, BladeSource, Allocation, EffectEntry, ScoreLine, TriggeredAbility, Adjustment, AbilityBonus,
    LivePerformanceData};

#[derive(Debug, Clone)]
pub struct GameState {
    // --- 8-byte aligned (Player, HashMap, HashSet, String, Vec, Arc, usize, Value) ---
    pub player1: Player,
    pub player2: Player,
    pub ability_queue: AbilityQueue,
    pub card_database: Arc<CardDatabase>,
    pub mods: Arc<GameModifiers>,
    pub resolution_zone: ResolutionZone,
    pub pending_choice: Option<serde_json::Value>,
    pub pending_sequential_actions: Option<Vec<crate::card::AbilityEffect>>,
    pub heart_color_decision_phase: String,
    pub game_state_history: Vec<String>,
    pub max_state_history_size: usize,
    pub rule_log: Vec<String>,
    pub turn1_abilities_played: std::collections::HashSet<String>,
    pub turn2_abilities_played: std::collections::HashMap<String, u32>,
    pub live_owned_hearts: std::collections::HashMap<String, Vec<(String, u32)>>,
    pub temporary_effects: Vec<TemporaryEffect>,
    pub prohibition_effects: Vec<String>,
    pub cannot_activate_members: Vec<String>,
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
    pub recently_moved_cards: Option<Vec<i16>>,
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
    pub formation_change_occurred_this_turn: bool,
    pub opponent_choice_declined: bool,
    pub live_being_performed: bool,
    pub game_ended: bool,
    pub draw_state: bool,
    pub loop_detected: bool,
    pub live_success_triggered_this_turn: bool,
    pub self_no_excess_heart_this_turn: bool,
    pub performance_snapshots: Vec<PerformanceSnapshot>,
}

impl GameState {
    pub fn phase_invariant(&self) -> bool {
        if matches!(self.current_phase, Phase::RockPaperScissors | Phase::ChooseFirstAttacker | Phase::MulliganP1Turn | Phase::MulliganP2Turn) {
            return true;
        }
        match self.current_turn_phase {
            TurnPhase::FirstAttackerNormal | TurnPhase::SecondAttackerNormal => {
                matches!(self.current_phase, Phase::Active | Phase::Energy | Phase::Draw | Phase::Main)
            }
            TurnPhase::Live => {
                matches!(self.current_phase, Phase::LiveCardSetP1Turn | Phase::LiveCardSetP2Turn | Phase::FirstAttackerPerformance | Phase::SecondAttackerPerformance | Phase::LiveVictoryDetermination)
            }
        }
    }

    pub fn new(player1: Player, player2: Player, card_database: Arc<CardDatabase>) -> Self {
        let state = GameState {
            player1,
            player2,
            ability_queue: AbilityQueue::new(),
            card_database,
            mods: Arc::new(GameModifiers::new()),
            resolution_zone: ResolutionZone::new(),
            pending_choice: None,
            pending_sequential_actions: None,
            heart_color_decision_phase: "none".to_string(),
            game_state_history: Vec::new(),
            max_state_history_size: DEFAULT_HISTORY_SIZE,
            rule_log: Vec::new(),
            turn1_abilities_played: std::collections::HashSet::new(),
            turn2_abilities_played: std::collections::HashMap::new(),
            live_owned_hearts: std::collections::HashMap::new(),
            temporary_effects: Vec::new(),
            prohibition_effects: Vec::new(),
            cannot_activate_members: Vec::new(),
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
            recently_moved_cards: None,
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
            formation_change_occurred_this_turn: false,
            opponent_choice_declined: false,
            live_being_performed: false,
            game_ended: false,
            draw_state: false,
            loop_detected: false,
            live_success_triggered_this_turn: false,
            self_no_excess_heart_this_turn: false,
            performance_snapshots: Vec::new(),
        };
        debug_assert!(state.phase_invariant(), "GameState phase invariant violated after creation");
        state
    }

    pub fn active_player(&self) -> &Player {
        match self.current_phase {
            Phase::MulliganP1Turn => &self.player1,
            Phase::MulliganP2Turn => &self.player2,
            Phase::LiveCardSetP1Turn => &self.player1,
            Phase::LiveCardSetP2Turn => &self.player2,
            _ => match self.current_turn_phase {
                TurnPhase::FirstAttackerNormal => self.first_attacker(),
                TurnPhase::SecondAttackerNormal => self.second_attacker(),
                TurnPhase::Live => self.first_attacker(),
            }
        }
    }

    pub fn active_player_mut(&mut self) -> &mut Player {
        match self.current_phase {
            Phase::MulliganP1Turn => &mut self.player1,
            Phase::MulliganP2Turn => &mut self.player2,
            Phase::LiveCardSetP1Turn => &mut self.player1,
            Phase::LiveCardSetP2Turn => &mut self.player2,
            _ => match self.current_turn_phase {
                TurnPhase::FirstAttackerNormal => {
                    if self.player1.is_first_attacker { &mut self.player1 } else { &mut self.player2 }
                }
                TurnPhase::SecondAttackerNormal => {
                    if self.player1.is_first_attacker { &mut self.player2 } else { &mut self.player1 }
                }
                TurnPhase::Live => {
                    if self.player1.is_first_attacker { &mut self.player1 } else { &mut self.player2 }
                }
            }
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
}

include!("tracking.rs");
include!("modifiers.rs");
include!("abilities.rs");