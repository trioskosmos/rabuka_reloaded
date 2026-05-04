use crate::card::{BladeColor, CardDatabase};
use crate::constants::DEFAULT_HISTORY_SIZE;
use crate::player::Player;
use crate::zones::ResolutionZone;
use crate::ability_queue::AbilityQueue;
use std::sync::Arc;
use std::collections::HashMap;

pub use crate::config::RuleConfig;
pub use crate::mod_map::ModMap;
pub use crate::types::{AbilityTrigger, Duration, GameResult, Phase, ReplacementEffect, TemporaryEffect, TurnPhase};

#[derive(Debug, Clone)]
pub struct GameState {
    pub player1: Player,
    pub player2: Player,
    pub current_turn_phase: TurnPhase,
    pub current_phase: Phase,
    pub turn_number: u32,
    pub resolution_zone: ResolutionZone,
    pub is_first_turn: bool,
    pub live_cheer_count: u32,
    pub turn1_abilities_played: std::collections::HashSet<String>,
    pub turn2_abilities_played: std::collections::HashMap<String, u32>,
    pub player1_cheer_blade_heart_count: u32,
    pub player2_cheer_blade_heart_count: u32,
    pub live_owned_hearts: std::collections::HashMap<String, Vec<(String, u32)>>,
    pub temporary_effects: Vec<TemporaryEffect>,
    pub game_result: GameResult,
    pub cheer_check_completed: bool,
    pub cheer_checks_required: u32,
    pub cheer_checks_done: u32,
    pub prohibition_effects: Vec<String>,
    pub turn_limited_abilities_used: std::collections::HashSet<String>,
    pub mulligan_selected_indices: Vec<usize>,
    pub rps_winner: Option<u8>,
    pub player1_rps_choice: Option<i32>,
    pub player2_rps_choice: Option<i32>,
    pub history: Vec<GameState>,
    pub future: Vec<GameState>,
    pub max_history_size: usize,
    pub card_database: Arc<CardDatabase>,
    pub blade_modifiers: ModMap<i32>,
    pub blade_type_modifiers: ModMap<BladeColor>,
    pub heart_modifiers: HashMap<i16, HashMap<crate::card::HeartColor, i32>>,
    pub heart_override: HashMap<i16, (crate::card::HeartColor, u32)>,
    pub orientation_modifiers: ModMap<String>,
    pub cost_modifiers: ModMap<i32>,
    pub revealed_cards: std::collections::HashSet<i16>,
    pub config: RuleConfig,
    pub ability_queue: AbilityQueue,
    pub pending_choice: Option<serde_json::Value>,
    pub activating_card: Option<i16>,
    pub pending_sequential_actions: Option<Vec<crate::card::AbilityEffect>>,
    pub score_modifiers: ModMap<i32>,
    pub need_heart_modifiers: HashMap<i16, HashMap<crate::card::HeartColor, i32>>,
    pub areas_placed_this_turn: std::collections::HashSet<String>,
    pub cards_appeared_this_turn: std::collections::HashSet<i16>,
    pub turn_order_changed: bool,
    pub auto_ability_trigger_counts: std::collections::HashMap<String, u32>,
    pub baton_touch_zero_cost: bool,
    pub baton_touch_replaced_member_cost: Option<u32>,
    pub turn_limit_usage: std::collections::HashMap<String, u32>,
    pub card_instance_counter: u32,
    pub card_instance_mapping: std::collections::HashMap<i16, u32>,
    pub baton_touch_count: u32,
    pub cards_moved_this_turn: std::collections::HashSet<i16>,
    pub heart_color_decision_phase: String,
    pub deck_refresh_pending: bool,
    pub position_change_occurred_this_turn: bool,
    pub opponent_live_success_this_turn: bool,
    pub opponent_live_no_excess_heart_this_turn: bool,
    pub formation_change_occurred_this_turn: bool,
    pub opponent_choice_declined: bool,
    pub live_being_performed: bool,
    pub game_ended: bool,
    pub draw_state: bool,
    pub gained_abilities: std::collections::HashMap<i16, Vec<String>>,
    pub negated_abilities: std::collections::HashSet<i16>,
    pub replacement_effects: Vec<ReplacementEffect>,
    pub looked_at_cards: Vec<i16>,
    pub effect_creation_counter: u32,
    pub game_state_history: Vec<String>,
    pub max_state_history_size: usize,
    pub loop_detected: bool,
    /// Blade bonuses from 常時 (constant) abilities, tracked so they can be recalculated
    pub constant_blade_bonuses: HashMap<i16, i32>,
    /// Tracks which stage area (0=left,1=center,2=right) was vacated by the last self_cost move
    /// Used for "same_area" destination in move_cards
    pub last_vacated_stage_area: Option<usize>,
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
            current_turn_phase: TurnPhase::FirstAttackerNormal,
            current_phase: Phase::Active,
            turn_number: 1,
            activating_card: None,
            resolution_zone: ResolutionZone::new(),
            is_first_turn: true,
            live_cheer_count: 0,
            turn1_abilities_played: std::collections::HashSet::new(),
            turn2_abilities_played: std::collections::HashMap::new(),
            player1_cheer_blade_heart_count: 0,
            player2_cheer_blade_heart_count: 0,
            live_owned_hearts: std::collections::HashMap::new(),
            temporary_effects: Vec::new(),
            game_result: GameResult::Ongoing,
            cheer_check_completed: false,
            cheer_checks_required: 0,
            cheer_checks_done: 0,
            prohibition_effects: Vec::new(),
            turn_limited_abilities_used: std::collections::HashSet::new(),
            mulligan_selected_indices: Vec::new(),
            rps_winner: None,
            player1_rps_choice: None,
            player2_rps_choice: None,
            history: Vec::new(),
            future: Vec::new(),
            max_history_size: 50,
            card_database,
            blade_modifiers: ModMap::new(),
            blade_type_modifiers: ModMap::new(),
            heart_modifiers: HashMap::new(),
            heart_override: HashMap::new(),
            orientation_modifiers: ModMap::new(),
            cost_modifiers: ModMap::new(),
            revealed_cards: std::collections::HashSet::new(),
            config: RuleConfig::default(),
            ability_queue: AbilityQueue::new(),
            pending_choice: None,
            pending_sequential_actions: None,
            score_modifiers: ModMap::new(),
            need_heart_modifiers: HashMap::new(),
            areas_placed_this_turn: std::collections::HashSet::new(),
            cards_appeared_this_turn: std::collections::HashSet::new(),
            turn_order_changed: false,
            auto_ability_trigger_counts: std::collections::HashMap::new(),
            baton_touch_zero_cost: false,
            baton_touch_replaced_member_cost: None,
            turn_limit_usage: std::collections::HashMap::new(),
            card_instance_counter: 0,
            card_instance_mapping: std::collections::HashMap::new(),
            baton_touch_count: 0,
            cards_moved_this_turn: std::collections::HashSet::new(),
            heart_color_decision_phase: "none".to_string(),
            deck_refresh_pending: false,
            live_being_performed: false,
            game_ended: false,
            draw_state: false,
            gained_abilities: std::collections::HashMap::new(),
            negated_abilities: std::collections::HashSet::new(),
            replacement_effects: Vec::new(),
            position_change_occurred_this_turn: false,
            formation_change_occurred_this_turn: false,
            opponent_choice_declined: false,
            opponent_live_success_this_turn: false,
            opponent_live_no_excess_heart_this_turn: false,
            looked_at_cards: Vec::new(),
            effect_creation_counter: 0,
            game_state_history: Vec::new(),
            max_state_history_size: DEFAULT_HISTORY_SIZE,
            loop_detected: false,
            constant_blade_bonuses: HashMap::new(),
            last_vacated_stage_area: None,
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