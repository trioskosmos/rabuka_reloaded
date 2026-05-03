use crate::card::{BladeColor, CardDatabase, Ability};
use crate::constants::DEFAULT_HISTORY_SIZE;
use crate::player::Player;
use crate::zones::ResolutionZone;
use crate::ability_queue::{AbilityQueue, AbilityQueueEntry, AbilityId};
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

    pub fn can_play_turn1_ability(&self, ability_id: &str) -> bool {
        !self.turn1_abilities_played.contains(ability_id)
    }

    pub fn can_play_turn2_ability(&self, ability_id: &str) -> bool {
        let count = self.turn2_abilities_played.get(ability_id).unwrap_or(&0);
        *count < 2
    }

    pub fn record_turn1_ability(&mut self, ability_id: String) {
        self.turn1_abilities_played.insert(ability_id);
    }

    pub fn record_turn2_ability(&mut self, ability_id: String) {
        *self.turn2_abilities_played.entry(ability_id).or_insert(0) += 1;
    }

    pub fn can_activate_area_ability(&self, player_id: &str, card_no: &str, area: crate::zones::MemberArea) -> bool {
        let player = if player_id == self.player1.id { &self.player1 } else { &self.player2 };
        if let Some(card_in_zone) = player.stage.get_area(area) {
            if let Some(card) = self.card_database.get_card(card_in_zone) {
                card.card_no == card_no
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn can_activate_center_ability(&self, player_id: &str, card_no: &str) -> bool {
        self.can_activate_area_ability(player_id, card_no, crate::zones::MemberArea::Center)
    }

    pub fn can_activate_left_side_ability(&self, player_id: &str, card_no: &str) -> bool {
        self.can_activate_area_ability(player_id, card_no, crate::zones::MemberArea::LeftSide)
    }

    pub fn can_activate_right_side_ability(&self, player_id: &str, card_no: &str) -> bool {
        self.can_activate_area_ability(player_id, card_no, crate::zones::MemberArea::RightSide)
    }

    pub fn reset_keyword_tracking(&mut self) {
        self.turn1_abilities_played.clear();
        self.turn2_abilities_played.clear();
        self.player1_cheer_blade_heart_count = 0;
        self.player2_cheer_blade_heart_count = 0;
        self.reset_change_flags();
        self.cheer_check_completed = false;
        self.reset_loop_detection();
    }

    pub fn perform_cheer_check(&mut self, player_id: &str, blade_count: u32) -> Result<(), String> {
        let player = if player_id == self.player1.id {
            &mut self.player1
        } else {
            &mut self.player2
        };

        if self.cheer_checks_required == 0 {
            self.cheer_checks_required = blade_count;
        }

        for _ in 0..blade_count {
            if let Some(card_id) = player.main_deck.draw() {
                self.resolution_zone.cards.push(card_id);
                self.cheer_checks_done += 1;
            }
        }

        if self.cheer_checks_done >= self.cheer_checks_required {
            self.cheer_check_completed = true;
        }
        Ok(())
    }

    pub fn check_required_hearts(&self) -> Result<bool, String> {
        if self.cheer_checks_done < self.cheer_checks_required {
            return Err(format!("Cannot check required hearts: {} of {} cheer checks completed",
                self.cheer_checks_done, self.cheer_checks_required));
        }
        Ok(true)
    }

    pub fn add_prohibition_effect(&mut self, effect: String) {
        self.prohibition_effects.push(effect);
    }

    pub fn is_action_prohibited(&self, action: &str) -> bool {
        self.prohibition_effects.iter().any(|e| e.contains(action))
    }

    pub fn record_turn_limited_ability_use(&mut self, card_id: String) {
        self.turn_limited_abilities_used.insert(card_id);
    }

    pub fn has_turn_limited_ability_been_used(&self, card_id: &str) -> bool {
        self.turn_limited_abilities_used.contains(card_id)
    }

    pub fn add_blade_modifier(&mut self, card_id: i16, delta: i32) {
        *self.blade_modifiers.entry(card_id).or_insert(0) += delta;
    }

    pub fn remove_blade_modifier(&mut self, card_id: i16, delta: i32) {
        let val = self.blade_modifiers.entry(card_id).or_insert(0);
        *val -= delta;
        if *val == 0 {
            self.blade_modifiers.remove(card_id);
        }
    }

    pub fn get_blade_modifier(&self, card_id: i16) -> i32 {
        self.blade_modifiers.get(card_id).copied().unwrap_or(0)
    }

    pub fn set_blade_type_modifier(&mut self, card_id: i16, blade_color: BladeColor) {
        self.blade_type_modifiers.set(card_id, blade_color);
    }

    pub fn get_blade_type_modifier(&self, card_id: i16) -> Option<BladeColor> {
        self.blade_type_modifiers.get(card_id).copied()
    }

    pub fn clear_blade_type_modifier(&mut self, card_id: i16) {
        self.blade_type_modifiers.remove(card_id);
    }

    /// Recalculate all 常時 (constant) ability blade bonuses.
    /// Evaluates each constant ability's condition against the current game state
    /// and applies/removes blade modifiers accordingly.
    pub fn recalculate_constant_blade_modifiers(&mut self) {
        let saved_queue = self.ability_queue.clone();
        let saved_activating = self.activating_card;

        let player1_id = self.player1.id.clone();
        let player2_id = self.player2.id.clone();
        let p1_cards: Vec<i16> = self.player1.stage.stage.iter().filter(|&&id| id != -1).copied().collect();
        let p2_cards: Vec<i16> = self.player2.stage.stage.iter().filter(|&&id| id != -1).copied().collect();

        // Pre-collect all constant abilities with their card IDs
        let mut all_const_abilities: Vec<(i16, usize, crate::card::Ability)> = Vec::new();
        for &cid in &p1_cards {
            if let Some(card) = self.card_database.get_card(cid) {
                for (idx, ability) in card.abilities.iter().enumerate() {
                    if ability.triggers.as_ref().map_or(false, |t| t.contains(crate::triggers::CONSTANT)) {
                        all_const_abilities.push((cid, idx, ability.clone()));
                    }
                }
            }
        }
        for &cid in &p2_cards {
            if let Some(card) = self.card_database.get_card(cid) {
                for (idx, ability) in card.abilities.iter().enumerate() {
                    if ability.triggers.as_ref().map_or(false, |t| t.contains(crate::triggers::CONSTANT)) {
                        all_const_abilities.push((cid, idx, ability.clone()));
                    }
                }
            }
        }

        let mut expected: HashMap<i16, i32> = HashMap::new();

        // Process each player's cards with proper queue context
        let players_info = [(player1_id.clone(), &p1_cards[..]), (player2_id.clone(), &p2_cards[..])];
        for (player_id, stage_cards) in &players_info {
            if stage_cards.is_empty() { continue; }

            self.ability_queue = AbilityQueue::new();
            self.ability_queue.enqueue(AbilityQueueEntry {
                id: AbilityId::new("_const", 0, "Constant"),
                card_no: String::new(),
                player_id: player_id.clone(),
                ability: Ability::default(),
                ability_index: 0,
                card_id: None,
                trigger_type: AbilityTrigger::Constant,
                completed: false,
                pending_choice_result: None,
                choice_card_no: None,
                conditional_choice: None,
            });
            self.ability_queue.start_next();
            self.activating_card = None;

            let mut eval_state = self.clone();
            let resolver = crate::ability::resolver::AbilityResolver::new(&mut eval_state);

            for &cid in *stage_cards {
                for ability in &all_const_abilities {
                    if ability.0 != cid { continue; }
                    if let Some(ref effect) = ability.2.effect {
                        if effect.action == "gain_resource" {
                            let is_blade = matches!(effect.resource.as_deref(), Some("blade") | Some("ブレード"));
                            if is_blade {
                                let cond_met = effect.condition.as_ref()
                                    .map_or(true, |c| resolver.evaluate_condition(c));
                                if cond_met {
                                    let count = effect.resource_icon_count.unwrap_or(effect.count.unwrap_or(1));
                                    *expected.entry(cid).or_insert(0) += count as i32;
                                }
                            }
                        }
                    }
                }
            }
        }

        self.ability_queue = saved_queue;
        self.activating_card = saved_activating;

        // Collect old keys before modifying
        let old_keys: Vec<i16> = self.constant_blade_bonuses.keys().copied().collect();
        for &cid in &old_keys {
            if let Some(&old) = self.constant_blade_bonuses.get(&cid) {
                self.remove_blade_modifier(cid, old);
            }
        }

        for (&cid, &new_val) in &expected {
            self.add_blade_modifier(cid, new_val);
        }

        self.constant_blade_bonuses = expected;
    }

    pub fn add_heart_modifier(&mut self, card_id: i16, color: crate::card::HeartColor, delta: i32) {
        let colors = self.heart_modifiers.entry(card_id).or_insert_with(std::collections::HashMap::new);
        *colors.entry(color).or_insert(0) += delta;
    }

    pub fn remove_heart_modifier(&mut self, card_id: i16, color: crate::card::HeartColor, delta: i32) {
        if let Some(colors) = self.heart_modifiers.get_mut(&card_id) {
            if let Some(modifier) = colors.get_mut(&color) {
                *modifier -= delta;
                if *modifier == 0 {
                    colors.remove(&color);
                }
            }
            if colors.is_empty() {
                self.heart_modifiers.remove(&card_id);
            }
        }
    }

    pub fn get_heart_modifier(&self, card_id: i16, color: crate::card::HeartColor) -> i32 {
        self.heart_modifiers.get(&card_id)
            .and_then(|colors| colors.get(&color))
            .copied()
            .unwrap_or(0)
    }

    pub fn set_heart_override(&mut self, card_id: i16, color: crate::card::HeartColor, count: u32, duration: &str) {
        self.heart_override.insert(card_id, (color, count));
        let mut data = serde_json::Map::new();
        data.insert("card_id".to_string(), serde_json::Value::Number(card_id.into()));
        data.insert("color".to_string(), serde_json::Value::String(format!("{:?}", color)));
        data.insert("count".to_string(), serde_json::Value::Number(count.into()));
        self.temporary_effects.push(TemporaryEffect {
            effect_type: "heart_override".to_string(),
            duration: match duration { "live_end" => Duration::LiveEnd, "this_turn" => Duration::ThisTurn, _ => Duration::ThisLive },
            created_turn: self.turn_number,
            created_phase: self.current_phase.clone(),
            target_player_id: String::new(),
            description: format!("Heart override: card {} = {:?} x{}", card_id, color, count),
            creation_order: 0,
            effect_data: Some(serde_json::Value::Object(data)),
        });
    }

    pub fn add_score_modifier(&mut self, card_id: i16, delta: i32) {
        *self.score_modifiers.entry(card_id).or_insert(0) += delta;
    }

    pub fn get_score_modifier(&self, card_id: i16) -> i32 {
        self.score_modifiers.get(card_id).copied().unwrap_or(0)
    }

    pub fn set_score_modifier(&mut self, card_id: i16, value: i32) {
        self.score_modifiers.set(card_id, value);
    }

    pub fn add_need_heart_modifier(&mut self, card_id: i16, color: crate::card::HeartColor, delta: i32) {
        let colors = self.need_heart_modifiers.entry(card_id).or_insert_with(std::collections::HashMap::new);
        *colors.entry(color).or_insert(0) += delta;
    }

    pub fn get_need_heart_modifier(&self, card_id: i16, color: crate::card::HeartColor) -> i32 {
        self.need_heart_modifiers.get(&card_id)
            .and_then(|colors| colors.get(&color))
            .copied()
            .unwrap_or(0)
    }

    pub fn record_area_placement(&mut self, player_id: &str, area: &str) {
        let key = format!("{}:{}", player_id, area);
        self.areas_placed_this_turn.insert(key);
    }

    pub fn has_area_been_placed_this_turn(&self, player_id: &str, area: &str) -> bool {
        let key = format!("{}:{}", player_id, area);
        self.areas_placed_this_turn.contains(&key)
    }

    pub fn clear_area_placement_tracking(&mut self) {
        self.areas_placed_this_turn.clear();
    }

    pub fn record_card_appearance(&mut self, card_id: i16) {
        self.cards_appeared_this_turn.insert(card_id);
    }

    pub fn has_card_appeared_this_turn(&self, card_id: i16) -> bool {
        self.cards_appeared_this_turn.contains(&card_id)
    }

    pub fn clear_card_appearance_tracking(&mut self) {
        self.cards_appeared_this_turn.clear();
    }

    pub fn set_player_has_live_score(&mut self, player_id: &str, has_score: bool) {
        if player_id == "player1" {
            self.player1.has_live_score = has_score;
        } else {
            self.player2.has_live_score = has_score;
        }
    }

    pub fn player_has_live_score(&self, player_id: &str) -> bool {
        if player_id == "player1" {
            self.player1.has_live_score
        } else {
            self.player2.has_live_score
        }
    }

    pub fn set_turn_order_changed(&mut self, changed: bool) {
        self.turn_order_changed = changed;
    }

    pub fn has_turn_order_changed(&self) -> bool {
        self.turn_order_changed
    }

    pub fn record_auto_ability_trigger(&mut self, card_id: &str) {
        *self.auto_ability_trigger_counts.entry(card_id.to_string()).or_insert(0) += 1;
    }

    pub fn get_auto_ability_trigger_count(&self, card_id: &str) -> u32 {
        *self.auto_ability_trigger_counts.get(card_id).unwrap_or(&0)
    }

    pub fn clear_auto_ability_trigger_tracking(&mut self) {
        self.auto_ability_trigger_counts.clear();
    }

    pub fn record_turn_limit_usage(&mut self, player_id: &str, card_instance_id: u32) {
        let key = format!("{}:{}", player_id, card_instance_id);
        *self.turn_limit_usage.entry(key).or_insert(0) += 1;
    }

    pub fn get_turn_limit_usage(&self, player_id: &str, card_instance_id: u32) -> u32 {
        let key = format!("{}:{}", player_id, card_instance_id);
        *self.turn_limit_usage.get(&key).unwrap_or(&0)
    }

    pub fn clear_turn_limit_tracking(&mut self) {
        self.turn_limit_usage.clear();
    }

    pub fn assign_card_instance_id(&mut self, card_id: i16) -> u32 {
        self.card_instance_counter += 1;
        let instance_id = self.card_instance_counter;
        self.card_instance_mapping.insert(card_id, instance_id);
        instance_id
    }

    pub fn get_card_instance_id(&self, card_id: i16) -> Option<u32> {
        self.card_instance_mapping.get(&card_id).copied()
    }

    pub fn remove_card_instance(&mut self, card_id: i16) {
        self.card_instance_mapping.remove(&card_id);
    }

    pub fn clear_card_instance_tracking(&mut self) {
        self.card_instance_mapping.clear();
        self.card_instance_counter = 0;
    }

    pub fn record_baton_touch(&mut self) {
        self.baton_touch_count += 1;
    }

    pub fn get_baton_touch_count(&self) -> u32 {
        self.baton_touch_count
    }

    pub fn clear_baton_touch_tracking(&mut self) {
        self.baton_touch_count = 0;
        self.baton_touch_zero_cost = false;
        self.baton_touch_replaced_member_cost = None;
    }

    pub fn record_card_movement(&mut self, card_id: i16) {
        self.cards_moved_this_turn.insert(card_id);
    }

    pub fn has_card_moved_this_turn(&self, card_id: i16) -> bool {
        self.cards_moved_this_turn.contains(&card_id)
    }

    pub fn clear_card_movement_tracking(&mut self) {
        self.cards_moved_this_turn.clear();
    }

    pub fn set_heart_color_decision_phase(&mut self, phase: &str) {
        self.heart_color_decision_phase = phase.to_string();
    }

    pub fn get_heart_color_decision_phase(&self) -> &str {
        &self.heart_color_decision_phase
    }

    pub fn is_in_required_hearts_check_phase(&self) -> bool {
        self.heart_color_decision_phase == "required_hearts_check"
    }

    pub fn is_in_live_start_phase(&self) -> bool {
        self.heart_color_decision_phase == "live_start"
    }

    pub fn set_deck_refresh_pending(&mut self, pending: bool) {
        self.deck_refresh_pending = pending;
    }

    pub fn is_deck_refresh_pending(&self) -> bool {
        self.deck_refresh_pending
    }

    pub fn perform_deck_refresh(&mut self, player_id: &str) {
        let player = if player_id == "player1" {
            &mut self.player1
        } else {
            &mut self.player2
        };

        let waitroom_cards: Vec<i16> = player.waitroom.cards.iter().copied().collect();
        player.waitroom.cards.clear();
        for card_id in waitroom_cards {
            player.main_deck.cards.push(card_id);
        }

        player.main_deck.shuffle();
        self.deck_refresh_pending = false;
    }



    pub fn set_live_being_performed(&mut self, performed: bool) {
        self.live_being_performed = performed;
    }

    pub fn is_live_being_performed(&self) -> bool {
        self.live_being_performed
    }

    pub fn set_game_ended(&mut self, ended: bool) {
        self.game_ended = ended;
    }

    pub fn is_game_ended(&self) -> bool {
        self.game_ended
    }

    pub fn set_draw_state(&mut self, draw: bool) {
        self.draw_state = draw;
    }

    pub fn is_draw_state(&self) -> bool {
        self.draw_state
    }

    pub fn check_success_zone_draw_condition(&self, player_id: &str) -> bool {
        let player = if player_id == self.player1.id {
            &self.player1
        } else if player_id == self.player2.id {
            &self.player2
        } else {
            return false;
        };

        let success_count = player.success_live_card_zone.cards.len();
        success_count >= 3
    }

    pub fn add_revealed_card(&mut self, card_id: i16) {
        self.revealed_cards.insert(card_id);
    }

    pub fn remove_revealed_card(&mut self, card_id: i16) {
        self.revealed_cards.remove(&card_id);
    }

    pub fn is_card_revealed(&self, card_id: i16) -> bool {
        self.revealed_cards.contains(&card_id)
    }

    pub fn clear_revealed_cards(&mut self) {
        self.revealed_cards.clear();
    }

    pub fn add_gained_ability(&mut self, card_id: i16, ability_type: String) {
        self.gained_abilities.entry(card_id).or_insert_with(Vec::new).push(ability_type);
    }

    pub fn remove_gained_abilities(&mut self, card_id: i16) {
        self.gained_abilities.remove(&card_id);
    }

    pub fn has_gained_ability(&self, card_id: i16, ability_type: &str) -> bool {
        if let Some(abilities) = self.gained_abilities.get(&card_id) {
            abilities.iter().any(|a| a == ability_type)
        } else {
            false
        }
    }

    pub fn clear_gained_abilities_for_card(&mut self, card_id: i16) {
        self.gained_abilities.remove(&card_id);
    }

    pub fn set_need_heart_modifier(&mut self, card_id: i16, color: crate::card::HeartColor, value: i32) {
        self.need_heart_modifiers.entry(card_id).or_default().insert(color, value);
    }

    pub fn add_orientation_modifier(&mut self, card_id: i16, orientation: &str) {
        self.orientation_modifiers.set(card_id, orientation.to_string());
    }

    pub fn add_cost_modifier(&mut self, card_id: i16, delta: i32) {
        *self.cost_modifiers.entry(card_id).or_insert(0) += delta;
    }

    pub fn set_cost_modifier(&mut self, card_id: i16, value: i32) {
        self.cost_modifiers.set(card_id, value);
    }

    pub fn get_cost_modifier(&self, card_id: i16) -> i32 {
        self.cost_modifiers.get(card_id).copied().unwrap_or(0)
    }

    pub fn get_orientation_modifier(&self, card_id: i16) -> Option<&String> {
        self.orientation_modifiers.get(card_id)
    }

    pub fn clear_modifiers_for_card(&mut self, card_id: i16) {
        self.blade_modifiers.remove(card_id);
        self.heart_modifiers.remove(&card_id);
        self.heart_override.remove(&card_id);
        self.score_modifiers.remove(card_id);
        self.need_heart_modifiers.remove(&card_id);
        self.orientation_modifiers.remove(card_id);
        self.cost_modifiers.remove(card_id);
    }

    pub fn move_resolution_zone_to_waitroom(&mut self, player_id: &str) {
        let player = if player_id == self.player1.id {
            &mut self.player1
        } else {
            &mut self.player2
        };

        for card_id in self.resolution_zone.cards.drain(..) {
            player.waitroom.cards.push(card_id);
        }
    }

    pub fn trigger_auto_ability(&mut self, ability_id: String, trigger_type: AbilityTrigger, player_id: String, source_card_id: Option<String>, explicit_card_id: Option<i16>) {
        use crate::ability_queue::{AbilityQueueEntry, AbilityId};

        if let Some(ref card_no) = source_card_id {
            let (card, card_id) = if let Some(cid) = explicit_card_id {
                (self.card_database.get_card(cid).cloned(), Some(cid))
            } else {
                self.find_card_by_number(card_no)
            };
            if let Some(card) = card {
                for (ability_index, ability) in card.abilities.iter().enumerate() {
                    if ability_id.contains(&ability.full_text) {
                        let entry = AbilityQueueEntry {
                            id: AbilityId::new(card_no, ability_index, &format!("{:?}", trigger_type)),
                            card_no: card_no.clone(),
                            player_id,
                            ability: ability.clone(),
                            ability_index,
                            card_id,
                            trigger_type,
                            completed: false,
                            pending_choice_result: None,
                            choice_card_no: None,
                            conditional_choice: None,
                        };

                        self.ability_queue.enqueue(entry);
                        break;
                    }
                }
            }
        }
    }

    fn find_card_by_number(&self, card_no: &str) -> (Option<crate::card::Card>, Option<i16>) {
        for player in [&self.player1, &self.player2] {
            for id in &player.hand.cards {
                if let Some(card) = self.card_database.get_card(*id) {
                    if card.card_no == card_no {
                        return (Some(card.clone()), Some(*id));
                    }
                }
            }

            for stage_card_id in &player.stage.stage {
                if *stage_card_id != -1 {
                    if let Some(card) = self.card_database.get_card(*stage_card_id) {
                        if card.card_no == card_no {
                            return (Some(card.clone()), Some(*stage_card_id));
                        }
                    }
                }
            }

            for waitroom_card_id in &player.waitroom.cards {
                if let Some(card) = self.card_database.get_card(*waitroom_card_id) {
                    if card.card_no == card_no {
                        return (Some(card.clone()), Some(*waitroom_card_id));
                    }
                }
            }

            for live_card_id in &player.live_card_zone.cards {
                if let Some(card) = self.card_database.get_card(*live_card_id) {
                    if card.card_no == card_no {
                        return (Some(card.clone()), Some(*live_card_id));
                    }
                }
            }

            for success_card_id in &player.success_live_card_zone.cards {
                if let Some(card) = self.card_database.get_card(*success_card_id) {
                    if card.card_no == card_no {
                        return (Some(card.clone()), Some(*success_card_id));
                    }
                }
            }
        }

        (None, None)
    }

    pub fn process_pending_auto_abilities(&mut self, _active_player_id: &str) {
        loop {
            if !self.ability_queue.is_idle() {
                break;
            }
            if !self.ability_queue.start_next() {
                break;
            }
            self.process_current_ability();
            // If a choice is pending, stop processing and wait for player input
            if self.pending_choice.is_some() {
                break;
            }
        }
    }

    fn process_current_ability(&mut self) {
        if let Some(entry) = self.ability_queue.current_entry().cloned() {
            self.activating_card = entry.card_id;

            let (choice, looked_at, result) = {
                let mut resolver = crate::ability_resolver::AbilityResolver::new(self);
                let result = resolver.resolve_ability(&entry.ability, entry.card_id, entry.ability_index);
                let choice = resolver.get_pending_choice().cloned();
                let looked_at = resolver.take_looked_at();
                (choice, looked_at, result)
            };
            self.looked_at_cards = looked_at;

            if let Err(e) = result {
                eprintln!("Failed to resolve ability: {}", e);
                self.ability_queue.complete_current();
                return;
            }

            if let Some(c) = choice {
                self.ability_queue.pause_for_choice(c);
            } else {
                self.ability_queue.complete_current();
                self.activating_card = None;
            }
        }
    }

    pub fn get_pending_choice(&self) -> Option<&crate::ability_resolver::Choice> {
        self.ability_queue.is_waiting_for_choice()
    }

    pub fn entry_effect(&self) -> Option<&crate::card::AbilityEffect> {
        self.ability_queue.current_entry().and_then(|e| e.ability.effect.as_ref())
    }

    pub fn entry_cost(&self) -> Option<&crate::card::AbilityCost> {
        self.ability_queue.current_entry().and_then(|e| e.ability.cost.as_ref())
    }

    pub fn entry_characters(&self) -> Option<&Vec<String>> {
        self.entry_cost().and_then(|c| c.characters.as_ref())
    }

    pub fn entry_destination(&self) -> Option<&str> {
        self.entry_effect().and_then(|e| e.destination.as_deref())
    }

    pub fn entry_choice_card_no(&self) -> Option<String> {
        self.ability_queue.current_entry().and_then(|e| e.choice_card_no.clone())
    }

    pub fn entry_conditional_choice(&self) -> Option<String> {
        self.ability_queue.current_entry().and_then(|e| e.conditional_choice.clone())
    }

    /// Resolve which player "self" refers to based on the ability master's player_id.
    /// The ability queue entry stores which player activated this ability.
    fn ability_master_id(&self) -> Option<String> {
        self.ability_queue.current_entry().map(|e| e.player_id.clone())
    }

    pub fn resolve_target_player_mut(&mut self, target: &str) -> &mut Player {
        let master = self.ability_master_id();
        match (target, master.as_deref()) {
            ("self", Some("player2")) => &mut self.player2,
            ("self", _) => &mut self.player1,
            ("opponent", Some("player2")) => &mut self.player1,
            ("opponent", _) => &mut self.player2,
            ("both", _) => {
                eprintln!("WARN: resolve_target_player_mut called with 'both' — returning player1, use execute_for_targets instead");
                &mut self.player1
            }
            _ => &mut self.player1,
        }
    }

    pub fn resolve_target_player(&self, target: &str) -> &Player {
        let master = self.ability_master_id();
        match (target, master.as_deref()) {
            ("self", Some("player2")) => &self.player2,
            ("self", _) => &self.player1,
            ("opponent", Some("player2")) => &self.player1,
            ("opponent", _) => &self.player2,
            _ => &self.player1,
        }
    }

    pub fn check_victory(&self) -> GameResult {
        let p1_success = self.player1.success_live_card_zone.len();
        let p2_success = self.player2.success_live_card_zone.len();

        let p1_wins = p1_success >= 3 && p2_success <= 2;
        let p2_wins = p2_success >= 3 && p1_success <= 2;

        if p1_success >= 3 && p2_success >= 3 {
            GameResult::Draw
        } else if p1_wins && !p2_wins {
            GameResult::FirstAttackerWins
        } else if p2_wins && !p1_wins {
            GameResult::SecondAttackerWins
        } else {
            GameResult::Ongoing
        }
    }

    pub fn resolve_target<'a>(&'a self, target: &str, perspective_player: &'a Player) -> Vec<&'a Player> {
        match target {
            "self" | "自分" => {
                vec![perspective_player]
            }
            "opponent" | "相手" => {
                if std::ptr::eq(perspective_player, &self.player1) {
                    vec![&self.player2]
                } else {
                    vec![&self.player1]
                }
            }
            "both" | "両方" => {
                vec![&self.player1, &self.player2]
            }
            "either" | "どちらか" => {
                vec![&self.player1, &self.player2]
            }
            _ => vec![],
        }
    }

    pub fn resolve_target_mut(&mut self, target: &str, perspective_player_id: &str) -> Vec<&mut Player> {
        match target {
            "self" | "自分" => {
                if perspective_player_id == self.player1.id {
                    vec![&mut self.player1]
                } else {
                    vec![&mut self.player2]
                }
            }
            "opponent" | "相手" => {
                if perspective_player_id == self.player1.id {
                    vec![&mut self.player2]
                } else {
                    vec![&mut self.player1]
                }
            }
            "both" | "両方" => {
                vec![&mut self.player1, &mut self.player2]
            }
            "either" | "どちらか" => {
                vec![&mut self.player1, &mut self.player2]
            }
            _ => vec![],
        }
    }

    pub fn get_player(&self, player_id: &str) -> Option<&Player> {
        if self.player1.id == player_id {
            Some(&self.player1)
        } else if self.player2.id == player_id {
            Some(&self.player2)
        } else {
            None
        }
    }

    pub fn get_player_mut(&mut self, player_id: &str) -> Option<&mut Player> {
        if self.player1.id == player_id {
            Some(&mut self.player1)
        } else if self.player2.id == player_id {
            Some(&mut self.player2)
        } else {
            None
        }
    }

    pub fn should_trigger_debut(&self, _player: &Player, card: &crate::card::Card) -> bool {
        card.is_member()
    }

    pub fn should_trigger_live_start(&self, _player: &Player) -> bool {
        self.current_phase == Phase::FirstAttackerPerformance
            || self.current_phase == Phase::SecondAttackerPerformance
    }

    pub fn should_trigger_live_success(&self, _player: &Player) -> bool {
        self.current_phase == Phase::LiveVictoryDetermination
    }

    pub fn can_place_card_in_zone(&self, card_id: i16, zone: &str, _player_id: &str) -> bool {
        if let Some(card) = self.card_database.get_card(card_id) {
            for ability in &card.abilities {
                if ability.triggers.as_ref().map_or(false, |t| t.contains(crate::triggers::CONSTANT)) {
                    if let Some(ref effect) = ability.effect {
                        if effect.action == "restriction"
                            && effect.restriction_type.as_deref() == Some("cannot_place")
                            && (effect.restricted_destination.as_deref() == Some(zone)
                                || effect.restricted_destination.as_deref() == Some("live_card_zone") && zone == "success_live_zone"
                                || effect.restricted_destination.as_deref() == Some("success_live_zone") && zone == "live_card_zone")
                        {
                            eprintln!("Card {} cannot be placed in {} due to constant ability restriction", card.card_no, zone);
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    pub fn enforce_constant_ability_restrictions(&mut self) {
        let p1_id = self.player1.id.clone();
        let p2_id = self.player2.id.clone();
        let p1_cards: Vec<(usize, i16)> = self.player1.live_card_zone.cards.iter().enumerate().map(|(i, &id)| (i, id)).collect();
        let p2_cards: Vec<(usize, i16)> = self.player2.live_card_zone.cards.iter().enumerate().map(|(i, &id)| (i, id)).collect();

        let mut cards_to_remove: Vec<(&str, usize)> = Vec::new();
        for (index, card_id) in p1_cards {
            if !self.can_place_card_in_zone(card_id, "live_card_zone", &p1_id) {
                cards_to_remove.push((&p1_id, index));
            }
        }
        for (index, card_id) in p2_cards {
            if !self.can_place_card_in_zone(card_id, "live_card_zone", &p2_id) {
                cards_to_remove.push((&p2_id, index));
            }
        }

        for (player_id, index) in cards_to_remove {
            let player = if *player_id == self.player1.id { &mut self.player1 } else { &mut self.player2 };
            let card = player.live_card_zone.cards.remove(index);
            player.waitroom.cards.push(card);
            if let Some(card_data) = self.card_database.get_card(card) {
                eprintln!("Removed card {} from live_card_zone due to constant ability restriction", card_data.card_no);
            }
        }
    }

    pub fn get_triggerable_abilities<'a>(
        &self,
        card: &'a crate::card::Card,
        trigger: AbilityTrigger,
        player: &Player,
    ) -> Vec<&'a crate::card::Ability> {
        card.abilities.iter().filter(|ability| {
            match trigger {
                AbilityTrigger::Activation => {
                    let trigger_match = ability.triggers.as_ref().map_or(false, |t| t.contains(crate::triggers::ACTIVATION));
                    trigger_match
                }
                AbilityTrigger::Debut => {
                    let trigger_match = ability.triggers.as_ref().map_or(false, |t| t.contains(crate::triggers::DEBUT) || t.contains(crate::triggers::DEBUT_EN));
                    let should_trigger = trigger_match && self.should_trigger_debut(player, card);
                    should_trigger
                }
                AbilityTrigger::LiveStart => {
                    let trigger_match = ability.triggers.as_ref().map_or(false, |t| t.contains(crate::triggers::LIVE_START));
                    let should_trigger = trigger_match && self.should_trigger_live_start(player);
                    should_trigger
                }
                AbilityTrigger::LiveSuccess => {
                    let trigger_match = ability.triggers.as_ref().map_or(false, |t| t.contains(crate::triggers::LIVE_SUCCESS));
                    let should_trigger = trigger_match && self.should_trigger_live_success(player);
                    should_trigger
                }
                AbilityTrigger::Constant => {
                    let trigger_match = ability.triggers.as_ref().map_or(false, |t| t.contains(crate::triggers::CONSTANT));
                    trigger_match
                }
                AbilityTrigger::Auto => {
                    let trigger_match = ability.triggers.as_ref().map_or(false, |t| t.contains(crate::triggers::AUTO));
                    trigger_match
                }
            }
        }).collect()
    }

    pub fn add_temporary_effect(
        &mut self,
        effect_type: String,
        duration: Duration,
        target_player_id: String,
        description: String,
    ) {
        let order = self.effect_creation_counter;
        self.effect_creation_counter += 1;
        self.temporary_effects.push(TemporaryEffect {
            effect_type,
            duration,
            created_turn: self.turn_number,
            created_phase: self.current_phase.clone(),
            target_player_id,
            description,
            creation_order: order,
            effect_data: None,
        });
    }

    pub fn get_temporary_effects_in_order(&self) -> Vec<&TemporaryEffect> {
        let mut effects = self.temporary_effects.iter().collect::<Vec<_>>();
        effects.sort_by_key(|e| e.creation_order);
        effects
    }

    pub fn check_expired_effects(&mut self) {
        let mut expired_indices = Vec::new();

        for (i, effect) in self.temporary_effects.iter().enumerate() {
            let is_expired = match effect.duration {
                Duration::LiveEnd => {
                    self.current_turn_phase != TurnPhase::Live
                }
                Duration::ThisTurn => {
                    self.turn_number > effect.created_turn
                }
                Duration::ThisLive => {
                    self.current_turn_phase != TurnPhase::Live
                }
                Duration::Permanent => false,
                Duration::AsLongAs => {
                    self.current_turn_phase != TurnPhase::Live
                }
            };

            if is_expired {
                expired_indices.push(i);
            }
        }

        for i in expired_indices.into_iter().rev() {
            let effect = self.temporary_effects.remove(i);
            match effect.effect_type.as_str() {
                "activation_cost_increase" => {
                    self.prohibition_effects.retain(|p| !p.contains(&effect.effect_type));
                }
                "activation_cost_decrease" => {
                    self.prohibition_effects.retain(|p| !p.contains(&effect.effect_type));
                }
                "gain_resource_blade" => {
                    if let Some(ref data) = effect.effect_data {
                        if let Some(cards) = data.as_array() {
                            for card_data in cards {
                                if let Some(card_id) = card_data.get("card_id").and_then(|v| v.as_i64()) {
                                    if let Some(amount) = card_data.get("amount").and_then(|v| v.as_i64()) {
                                        self.remove_blade_modifier(card_id as i16, amount as i32);
                                        eprintln!("Reverted {} blades from card {}", amount, card_id);
                                    }
                                }
                            }
                        } else if let Some(card_data) = data.as_object() {
                            if let Some(card_id) = card_data.get("card_id").and_then(|v| v.as_i64()) {
                                if let Some(amount) = card_data.get("amount").and_then(|v| v.as_i64()) {
                                    self.remove_blade_modifier(card_id as i16, amount as i32);
                                    eprintln!("Reverted {} blades from card {}", amount, card_id);
                                }
                            }
                        }
                    }
                }
                "gain_resource_heart" => {
                    if let Some(ref data) = effect.effect_data {
                        if let Some(cards) = data.as_array() {
                            for card_data in cards {
                                if let Some(card_id) = card_data.get("card_id").and_then(|v| v.as_i64()) {
                                    if let Some(amount) = card_data.get("amount").and_then(|v| v.as_i64()) {
                                        self.remove_heart_modifier(card_id as i16, crate::card::HeartColor::Heart01, amount as i32);
                                        eprintln!("Reverted {} hearts from card {}", amount, card_id);
                                    }
                                }
                            }
                        }
                    }
                }
                "heart_override" => {
                    if let Some(ref data) = effect.effect_data {
                        if let Some(card_id) = data.get("card_id").and_then(|v| v.as_i64()) {
                            self.heart_override.remove(&(card_id as i16));
                            eprintln!("Removed heart override for card {}", card_id);
                        }
                    }
                }
                _ => {
                    eprintln!("Expired effect: {}", effect.description);
                }
            }
        }
    }

    pub fn get_active_effects_for_player(&self, player_id: &str) -> Vec<&TemporaryEffect> {
        self.temporary_effects
            .iter()
            .filter(|e| e.target_player_id == player_id)
            .collect()
    }

    pub fn add_replacement_effect(
        &mut self,
        card_id: i16,
        player_id: String,
        original_event: String,
        replacement_effects: Vec<crate::card::AbilityEffect>,
        is_choice_based: bool,
    ) {
        self.replacement_effects.push(ReplacementEffect {
            card_id,
            player_id,
            original_event,
            replacement_effects,
            is_choice_based,
            applied_this_event: false,
        });
    }

    pub fn remove_replacement_effects_for_card(&mut self, card_id: i16) {
        self.replacement_effects.retain(|e| e.card_id != card_id);
    }

    pub fn get_replacement_effects_for_event(&self, event: &str) -> Vec<&ReplacementEffect> {
        self.replacement_effects
            .iter()
            .filter(|e| e.original_event == event && !e.applied_this_event)
            .collect()
    }

    pub fn reset_replacement_effect_flags(&mut self) {
        for effect in &mut self.replacement_effects {
            effect.applied_this_event = false;
        }
    }

    pub fn mark_replacement_effect_applied(&mut self, card_id: i16) {
        if let Some(effect) = self.replacement_effects.iter_mut().find(|e| e.card_id == card_id) {
            effect.applied_this_event = true;
        }
    }

    pub fn set_opponent_live_success(&mut self, no_excess_heart: bool) {
        self.opponent_live_success_this_turn = true;
        self.opponent_live_no_excess_heart_this_turn = no_excess_heart;
    }

    pub fn set_formation_change_occurred(&mut self) {
        self.formation_change_occurred_this_turn = true;
    }

    pub fn reset_change_flags(&mut self) {
        self.position_change_occurred_this_turn = false;
        self.formation_change_occurred_this_turn = false;
        self.opponent_live_success_this_turn = false;
        self.opponent_live_no_excess_heart_this_turn = false;
    }

    pub fn check_permanent_loop(&mut self) -> bool {
        let state_hash = self.generate_state_hash();

        if self.game_state_history.contains(&state_hash) {
            self.loop_detected = true;
            return true;
        }

        self.game_state_history.push(state_hash);

        if self.game_state_history.len() > self.max_state_history_size {
            self.game_state_history.remove(0);
        }

        false
    }

    fn generate_state_hash(&self) -> String {
        format!(
            "t{}_p{}_tp{}_p1h{}_p1e{}_p1w{}_p1l{}_p1su{}_p1st{:?}_p2h{}_p2e{}_p2w{}_p2l{}_p2su{}_p2st{:?}_oe{}_pro{}_tmp{}_rps{:?}",
            self.turn_number,
            self.current_phase.to_string(),
            self.current_turn_phase.to_string(),
            self.player1.hand.cards.len(),
            self.player1.energy_zone.cards.len(),
            self.player1.waitroom.cards.len(),
            self.player1.live_card_zone.cards.len(),
            self.player1.success_live_card_zone.cards.len(),
            self.player1.stage.stage,
            self.player2.hand.cards.len(),
            self.player2.energy_zone.cards.len(),
            self.player2.waitroom.cards.len(),
            self.player2.live_card_zone.cards.len(),
            self.player2.success_live_card_zone.cards.len(),
            self.player2.stage.stage,
            self.orientation_modifiers.len(),
            self.prohibition_effects.len(),
            self.temporary_effects.len(),
            self.rps_winner
        )
    }

    pub fn reset_loop_detection(&mut self) {
        self.game_state_history.clear();
        self.loop_detected = false;
    }

    pub fn is_loop_detected(&self) -> bool {
        self.loop_detected
    }

    pub fn save_state(&mut self) {
        self.future.clear();
        self.history.push(self.clone());

        if self.history.len() > self.max_history_size {
            self.history.drain(..1);
        }
    }

    pub fn undo(&mut self) -> Result<(), String> {
        if self.history.is_empty() {
            return Err("No history to undo".to_string());
        }

        self.future.push(self.clone());

        let previous = self.history.pop().unwrap();
        *self = previous;

        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), String> {
        if self.future.is_empty() {
            return Err("No future to redo".to_string());
        }

        self.history.push(self.clone());

        let next = self.future.pop().unwrap();
        *self = next;

        Ok(())
    }

    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }
}
