use crate::card::CardDatabase;
use crate::player::Player;
use crate::zones::ResolutionZone;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbilityTrigger {
    Activation,           // 起動
    Debut,               // 登場
    LiveStart,           // ライブ開始時
    LiveSuccess,         // ライブ成功時
    PerformancePhaseStart, // パフォーマンスフェイズの始めに (8.3.3)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnPhase {
    FirstAttackerNormal,   // Rule 7.3.2.1
    SecondAttackerNormal,  // Rule 7.3.2.1
    Live,                  // Rule 8.1
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    // Pre-game phases (Rule 6.2)
    RockPaperScissors,
    Mulligan,
    // Normal phase sub-phases (Rule 7.3.3)
    Active,
    Energy,
    Draw,
    Main,
    // Live phase sub-phases (Rule 8.1.2)
    LiveCardSet,
    FirstAttackerPerformance,
    SecondAttackerPerformance,
    LiveVictoryDetermination,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameResult {
    FirstAttackerWins,
    SecondAttackerWins,
    Draw,
    Ongoing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Duration {
    LiveEnd,
    ThisTurn,
    ThisLive,
    Permanent,
}

#[derive(Debug, Clone)]
pub struct TemporaryEffect {
    pub effect_type: String,
    pub duration: Duration,
    pub created_turn: u32,
    pub created_phase: Phase,
    pub target_player_id: String,
    pub description: String,
}

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
    // Keyword tracking
    pub turn1_abilities_played: std::collections::HashSet<String>, // Track Turn1 abilities played this turn
    pub turn2_abilities_played: std::collections::HashMap<String, u32>, // Track Turn2 abilities played this turn
    // Rule 8.4.2.1: Track cheer blade heart counts for victory determination
    pub player1_cheer_blade_heart_count: u32,
    pub player2_cheer_blade_heart_count: u32,
    // Duration tracking for temporary effects
    pub temporary_effects: Vec<TemporaryEffect>,
    // Game result tracking
    pub game_result: GameResult,
    // Automatic ability triggering - Rule 9.7
    pub pending_auto_abilities: Vec<PendingAutoAbility>,
    // Cheer check state - must complete before checking required hearts
    pub cheer_check_completed: bool,
    pub cheer_checks_required: u32,
    pub cheer_checks_done: u32,
    // Prohibition effects tracking - e.g., "cannot play member cards"
    pub prohibition_effects: Vec<String>,
    // Turn-limited ability usage tracking per card instance (card_id + zone)
    pub turn_limited_abilities_used: std::collections::HashSet<String>,
    // Mulligan tracking
    pub mulligan_player1_done: bool,
    pub mulligan_player2_done: bool,
    pub current_mulligan_player: String, // "player1" or "player2"
    pub mulligan_selected_indices: Vec<usize>, // Track selected card indices for mulligan
    // Live card set tracking
    pub live_card_set_player1_done: bool,
    pub live_card_set_player2_done: bool,
    // Undo/redo history
    pub history: Vec<GameState>,
    pub future: Vec<GameState>,
    pub max_history_size: usize,
    // Card database - shared across all game states
    pub card_database: Arc<CardDatabase>,
    // Card modifier tracking - tracks blade/heart/score changes instead of mutating Card objects
    pub blade_modifiers: std::collections::HashMap<i16, i32>, // card_id -> blade delta
    pub heart_modifiers: std::collections::HashMap<i16, std::collections::HashMap<crate::card::HeartColor, i32>>, // card_id -> color -> heart delta
    pub score_modifiers: std::collections::HashMap<i16, i32>, // card_id -> score delta
    pub need_heart_modifiers: std::collections::HashMap<i16, std::collections::HashMap<crate::card::HeartColor, i32>>, // card_id -> color -> need_heart delta
    pub orientation_modifiers: std::collections::HashMap<i16, String>, // card_id -> "active" or "wait"
    // Optional cost behavior: "always_pay", "never_pay", "auto" (pay if beneficial)
    pub optional_cost_behavior: String,
}

#[derive(Debug, Clone)]
pub struct PendingAutoAbility {
    pub ability_id: String,
    pub trigger_type: AbilityTrigger,
    pub player_id: String,
    pub source_card_id: Option<String>,
}

impl GameState {
    pub fn new(player1: Player, player2: Player, card_database: Arc<CardDatabase>) -> Self {
        let is_first_turn = true;
        GameState {
            player1,
            player2,
            current_turn_phase: TurnPhase::FirstAttackerNormal,
            current_phase: Phase::Active,
            turn_number: 1,
            resolution_zone: ResolutionZone::new(),
            is_first_turn,
            live_cheer_count: 0,
            turn1_abilities_played: std::collections::HashSet::new(),
            turn2_abilities_played: std::collections::HashMap::new(),
            player1_cheer_blade_heart_count: 0,
            player2_cheer_blade_heart_count: 0,
            temporary_effects: Vec::new(),
            game_result: GameResult::Ongoing,
            pending_auto_abilities: Vec::new(),
            cheer_check_completed: false,
            cheer_checks_required: 0,
            cheer_checks_done: 0,
            prohibition_effects: Vec::new(),
            turn_limited_abilities_used: std::collections::HashSet::new(),
            mulligan_player1_done: false,
            mulligan_player2_done: false,
            current_mulligan_player: String::new(),
            mulligan_selected_indices: Vec::new(),
            live_card_set_player1_done: false,
            live_card_set_player2_done: false,
            history: Vec::new(),
            future: Vec::new(),
            max_history_size: 50,
            card_database,
            blade_modifiers: std::collections::HashMap::new(),
            heart_modifiers: std::collections::HashMap::new(),
            score_modifiers: std::collections::HashMap::new(),
            need_heart_modifiers: std::collections::HashMap::new(),
            orientation_modifiers: std::collections::HashMap::new(),
            optional_cost_behavior: "always_pay".to_string(), // Default to always pay for bot/test mode
        }
    }

    pub fn active_player(&self) -> &Player {
        // Rule 7.2: Determine active player based on turn phase
        match self.current_turn_phase {
            TurnPhase::FirstAttackerNormal => self.first_attacker(),
            TurnPhase::SecondAttackerNormal => self.second_attacker(),
            TurnPhase::Live => {
                // Rule 7.2.1.2: In phases without specified turn player, first attacker is active
                self.first_attacker()
            }
        }
    }

    pub fn active_player_mut(&mut self) -> &mut Player {
        match self.current_turn_phase {
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
        // Rule 11.2: Turn1 - ability can only be played once per turn
        !self.turn1_abilities_played.contains(ability_id)
    }

    pub fn can_play_turn2_ability(&self, ability_id: &str) -> bool {
        // Rule 11.3: Turn2 - ability can only be played twice per turn
        let count = self.turn2_abilities_played.get(ability_id).unwrap_or(&0);
        *count < 2
    }

    pub fn record_turn1_ability(&mut self, ability_id: String) {
        self.turn1_abilities_played.insert(ability_id);
    }

    pub fn record_turn2_ability(&mut self, ability_id: String) {
        *self.turn2_abilities_played.entry(ability_id).or_insert(0) += 1;
    }

    pub fn can_activate_center_ability(&self, player_id: &str, card_no: &str) -> bool {
        // Rule 11.7.2: Center ability can only activate if member is in center area
        let player = if player_id == self.player1.id { &self.player1 } else { &self.player2 };
        if let Some(card_in_zone) = player.stage.get_area(crate::zones::MemberArea::Center) {
            if let Some(card) = self.card_database.get_card(card_in_zone) {
                card.card_no == card_no
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn can_activate_left_side_ability(&self, player_id: &str, card_no: &str) -> bool {
        // Rule 11.8.2: LeftSide ability can only activate if member is in left side area
        let player = if player_id == self.player1.id { &self.player1 } else { &self.player2 };
        if let Some(card_in_zone) = player.stage.get_area(crate::zones::MemberArea::LeftSide) {
            if let Some(card) = self.card_database.get_card(card_in_zone) {
                card.card_no == card_no
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn can_activate_right_side_ability(&self, player_id: &str, card_no: &str) -> bool {
        // Rule 11.9.2: RightSide ability can only activate if member is in right side area
        let player = if player_id == self.player1.id { &self.player1 } else { &self.player2 };
        if let Some(card_in_zone) = player.stage.get_area(crate::zones::MemberArea::RightSide) {
            if let Some(card) = self.card_database.get_card(card_in_zone) {
                card.card_no == card_no
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn reset_keyword_tracking(&mut self) {
        // Reset keyword tracking at start of new turn
        self.turn1_abilities_played.clear();
        self.turn2_abilities_played.clear();
        // Reset cheer blade heart counts at start of new turn
        self.player1_cheer_blade_heart_count = 0;
        self.player2_cheer_blade_heart_count = 0;
        // Reset cheer check state
        self.cheer_check_completed = false;
    }

    pub fn perform_cheer_check(&mut self, player_id: &str, blade_count: u32) -> Result<(), String> {
        // Rule: Execute cheer - move top cards from main deck to resolution zone
        let player = if player_id == self.player1.id {
            &mut self.player1
        } else {
            &mut self.player2
        };

        // Set required count on first call
        if self.cheer_checks_required == 0 {
            self.cheer_checks_required = blade_count;
        }

        for _ in 0..blade_count {
            if let Some(card_id) = player.main_deck.draw() {
                self.resolution_zone.cards.push(card_id);
                self.cheer_checks_done += 1;
            }
        }

        // Mark as completed only when all required checks are done
        if self.cheer_checks_done >= self.cheer_checks_required {
            self.cheer_check_completed = true;
        }
        Ok(())
    }

    pub fn check_required_hearts(&self) -> Result<bool, String> {
        // Rule: Can only check required hearts after all cheer checks are completed
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

    // Modifier management methods for blade/heart/score tracking
    pub fn add_blade_modifier(&mut self, card_id: i16, delta: i32) {
        *self.blade_modifiers.entry(card_id).or_insert(0) += delta;
    }

    pub fn get_blade_modifier(&self, card_id: i16) -> i32 {
        *self.blade_modifiers.get(&card_id).unwrap_or(&0)
    }

    pub fn add_heart_modifier(&mut self, card_id: i16, color: crate::card::HeartColor, delta: i32) {
        let colors = self.heart_modifiers.entry(card_id).or_insert_with(std::collections::HashMap::new);
        *colors.entry(color).or_insert(0) += delta;
    }

    pub fn get_heart_modifier(&self, card_id: i16, color: crate::card::HeartColor) -> i32 {
        self.heart_modifiers.get(&card_id)
            .and_then(|colors| colors.get(&color))
            .copied()
            .unwrap_or(0)
    }

    pub fn add_score_modifier(&mut self, card_id: i16, delta: i32) {
        let current = self.score_modifiers.entry(card_id).or_insert(0);
        *current += delta;
    }

    pub fn get_score_modifier(&self, card_id: i16) -> i32 {
        *self.score_modifiers.get(&card_id).unwrap_or(&0)
    }

    pub fn set_score_modifier(&mut self, card_id: i16, value: i32) {
        self.score_modifiers.insert(card_id, value);
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

    pub fn set_need_heart_modifier(&mut self, card_id: i16, color: crate::card::HeartColor, value: i32) {
        let colors = self.need_heart_modifiers.entry(card_id).or_insert_with(std::collections::HashMap::new);
        colors.insert(color, value);
    }

    pub fn add_orientation_modifier(&mut self, card_id: i16, orientation: &str) {
        self.orientation_modifiers.insert(card_id, orientation.to_string());
    }

    pub fn get_orientation_modifier(&self, card_id: i16) -> Option<&String> {
        self.orientation_modifiers.get(&card_id)
    }

    pub fn clear_modifiers_for_card(&mut self, card_id: i16) {
        self.blade_modifiers.remove(&card_id);
        self.heart_modifiers.remove(&card_id);
        self.score_modifiers.remove(&card_id);
        self.need_heart_modifiers.remove(&card_id);
        self.orientation_modifiers.remove(&card_id);
    }

    pub fn move_resolution_zone_to_waitroom(&mut self, player_id: &str) {
        // Rule: In live victory determination phase, after winner places cards in success zone
        // Remaining cards in resolution zone go to waitroom
        let player = if player_id == self.player1.id {
            &mut self.player1
        } else {
            &mut self.player2
        };

        for card_id in self.resolution_zone.cards.drain(..) {
            player.waitroom.cards.push(card_id);
        }
    }

    pub fn trigger_auto_ability(&mut self, ability_id: String, trigger_type: AbilityTrigger, player_id: String, source_card_id: Option<String>) {
        // Rule 9.7.2: When automatic ability trigger condition is met, it enters waiting state
        self.pending_auto_abilities.push(PendingAutoAbility {
            ability_id,
            trigger_type,
            player_id,
            source_card_id,
        });
    }

    pub fn process_pending_auto_abilities(&mut self, active_player_id: &str) {
        // Rule 9.5.1: After rule processing, play and resolve automatic abilities
        // Rule 9.5.3.2: Active player chooses which of their waiting abilities to play first
        // Rule 9.5.3.3: Non-active player then plays their waiting abilities
        
        use std::collections::HashSet;
        
        let non_active_id = if active_player_id == self.player1.id { self.player2.id.as_str() } else { self.player1.id.as_str() };
        
        let mut processed = HashSet::new();
        let mut abilities_to_execute = Vec::with_capacity(self.pending_auto_abilities.len());
        
        // Single pass: collect abilities for both active and non-active players
        for (i, pending) in self.pending_auto_abilities.iter().enumerate() {
            if pending.player_id == active_player_id || pending.player_id == non_active_id {
                processed.insert(i);
                if let Some(ref card_no) = pending.source_card_id {
                    abilities_to_execute.push((card_no.clone(), pending.player_id.clone()));
                }
            }
        }
        
        // Remove processed abilities (in reverse order to maintain indices)
        let mut sorted_indices: Vec<_> = processed.iter().copied().collect();
        sorted_indices.sort_by(|a, b| b.cmp(a));
        for i in sorted_indices {
            self.pending_auto_abilities.remove(i);
        }
        
        // Execute collected abilities
        for (card_no, player_id) in abilities_to_execute {
            self.execute_card_ability(&card_no, &player_id);
        }
    }
    
    fn execute_card_ability(&mut self, card_no: &str, player_id: &str) {
        // Find the card and its abilities, then execute them using AbilityResolver
        
        let player_id_clone = player_id.to_string();
        let player = if player_id_clone == self.player1.id {
            &self.player1
        } else {
            &self.player2
        };
        
        // Search for the card on stage or in other zones
        let card = {
            let mut found_card = None;
            
            // Check stage
            let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
            for area in areas {
                if let Some(card_in_zone) = player.stage.get_area(area) {
                    if let Some(card) = self.card_database.get_card(card_in_zone) {
                        if card.card_no == card_no {
                            found_card = Some(card);
                            break;
                        }
                    }
                }
            }
            
            // Check live card zone
            if found_card.is_none() {
                for card_id in &player.live_card_zone.cards {
                    if let Some(card) = self.card_database.get_card(*card_id) {
                        if card.card_no == card_no {
                            found_card = Some(card);
                            break;
                        }
                    }
                }
            }
            
            found_card
        };
        
        if let Some(card) = card {
            let abilities = card.abilities.clone();
            for ability in abilities.iter() {
                // Use AbilityResolver to evaluate conditions and execute effects
                let mut resolver = crate::ability_resolver::AbilityResolver::new(self);
                if let Err(e) = resolver.resolve_ability(ability) {
                    eprintln!("Failed to resolve ability: {}", e);
                }
            }
        } else {
            eprintln!("Card not found: {}", card_no);
        }
    }
    
    #[allow(dead_code)]
    fn execute_ability_effect(&mut self, effect: &crate::card::AbilityEffect, player_id: &str) {
        // Execute ability effects directly on game state
        
        match effect.action.as_str() {
            "draw" => {
                let count = effect.count.unwrap_or(1);
                let player = if player_id == self.player1.id {
                    &mut self.player1
                } else {
                    &mut self.player2
                };
                for _ in 0..count {
                    let _ = player.draw_card();
                }
            }
            "move_cards" => {
                let count = effect.count.unwrap_or(1);
                let source = effect.source.as_deref().unwrap_or("");
                let destination = effect.destination.as_deref().unwrap_or("");
                let card_type = effect.card_type.as_deref();
                let target = effect.target.as_deref().unwrap_or("self");
                
                
                let player = if target == "self" {
                    if player_id == self.player1.id { &mut self.player1 } else { &mut self.player2 }
                } else {
                    if player_id == self.player1.id { &mut self.player2 } else { &mut self.player1 }
                };
                
                // Simplified implementation - move cards from source to destination
                match source {
                    "deck" | "デッキ" => {
                        for _ in 0..count {
                            if let Some(card) = player.main_deck.draw() {
                                match destination {
                                    "hand" | "手札" => player.hand.add_card(card),
                                    "discard" | "控え室" => player.waitroom.add_card(card),
                                    _ => {}
                                }
                            }
                        }
                    }
                    "hand" | "手札" => {
                        match destination {
                            "discard" | "控え室" => {
                                for _ in 0..count.min(player.hand.cards.len() as u32) {
                                    if let Some(card) = player.hand.remove_card(0) {
                                        player.waitroom.add_card(card);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    "discard" | "控え室" => {
                        match destination {
                            "hand" | "手札" => {
                                // Move from waitroom to hand, filtering by card type if specified
                                let card_ids_to_move: Vec<_> = player.waitroom.cards.iter()
                                    .filter(|c| {
                                        if let Some(ct) = card_type {
                                            if let Some(card) = self.card_database.get_card(**c) {
                                                match ct {
                                                    "live_card" | "ライブカード" => card.is_live(),
                                                    "member_card" | "メンバーカード" => card.is_member(),
                                                    _ => true
                                                }
                                            } else {
                                                false
                                            }
                                        } else {
                                            true
                                        }
                                    })
                                    .take(count as usize)
                                    .copied()
                                    .collect();

                                for card_id in card_ids_to_move {
                                    if let Some(pos) = player.waitroom.cards.iter().position(|&c| c == card_id) {
                                        player.waitroom.cards.remove(pos);
                                    }
                                    player.hand.cards.push(card_id);
                                    player.rebuild_hand_index_map();
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {
                    }
                }
            }
            "gain_resource" => {
                let resource = effect.resource.as_deref().unwrap_or("");
                let count = effect.count.unwrap_or(1);
                let target = effect.target.as_deref().unwrap_or("self");
                
                
                let player = if target == "self" {
                    if player_id == self.player1.id { &mut self.player1 } else { &mut self.player2 }
                } else {
                    if player_id == self.player1.id { &mut self.player2 } else { &mut self.player1 }
                };
                
                // Add resource to members on stage using modifier tracking
                let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
                let mut card_ids_to_modify: Vec<i16> = Vec::new();
                for area in areas {
                    if let Some(card_in_zone) = player.stage.get_area(area) {
                        card_ids_to_modify.push(card_in_zone);
                    }
                }

                for card_id in card_ids_to_modify {
                    match resource {
                        "blade" | "ブレード" => {
                            self.add_blade_modifier(card_id, count as i32);
                        }
                        _ => {}
                    }
                }
            }
            "sequential" => {
                if let Some(ref actions) = effect.actions {
                    for action in actions {
                        self.execute_ability_effect(action, player_id);
                    }
                }
            }
            "choice" => {
                // Choice effects require player input - for automated testing, skip or default
            }
            "look_and_select" => {
                // Look and select effects require player input - for automated testing, skip or default
            }
            "look_at" => {
                let count = effect.count.unwrap_or(1);
                let source = effect.source.as_deref().unwrap_or("");
                let player = if player_id == self.player1.id {
                    &mut self.player1
                } else {
                    &mut self.player2
                };
                match source {
                    "deck_top" => {
                        let _cards_to_look: Vec<_> = player.main_deck.cards.iter()
                            .take(count as usize)
                            .filter_map(|c| self.card_database.get_card(*c))
                            .map(|c| format!("{} ({})", c.name, c.card_no))
                            .collect();
                    }
                    "hand" => {
                        let _cards_to_look: Vec<_> = player.hand.cards.iter()
                            .take(count as usize)
                            .filter_map(|c| self.card_database.get_card(*c))
                            .map(|c| format!("{} ({})", c.name, c.card_no))
                            .collect();
                    }
                    "discard" | "控え室" => {
                        let _cards_to_look: Vec<_> = player.waitroom.cards.iter()
                            .take(count as usize)
                            .filter_map(|c| self.card_database.get_card(*c))
                            .map(|c| format!("{} ({})", c.name, c.card_no))
                            .collect();
                    }
                    _ => {
                    }
                }
            }
            "reveal" => {
                let count = effect.count.unwrap_or(1);
                let source = effect.source.as_deref().unwrap_or("");
                let player = if player_id == self.player1.id {
                    &mut self.player1
                } else {
                    &mut self.player2
                };
                match source {
                    "deck" | "デッキ" => {
                        let _cards_to_reveal: Vec<_> = player.main_deck.cards.iter()
                            .take(count as usize)
                            .filter_map(|c| self.card_database.get_card(*c))
                            .map(|c| format!("{} ({})", c.name, c.card_no))
                            .collect();
                    }
                    "hand" | "手札" => {
                        let _cards_to_reveal: Vec<_> = player.hand.cards.iter()
                            .take(count as usize)
                            .filter_map(|c| self.card_database.get_card(*c))
                            .map(|c| format!("{} ({})", c.name, c.card_no))
                            .collect();
                    }
                    _ => {
                    }
                }
            }
            "modify_score" => {
                let operation = effect.operation.as_deref().unwrap_or("add");
                let value = effect.value.unwrap_or(effect.count.unwrap_or(0));
                let target = effect.target.as_deref().unwrap_or("self");
                
                let player = if target == "self" {
                    if player_id == self.player1.id { &mut self.player1 } else { &mut self.player2 }
                } else {
                    if player_id == self.player1.id { &mut self.player2 } else { &mut self.player1 }
                };

                // Use modifier tracking instead of direct card mutation
                let card_ids: Vec<i16> = player.live_card_zone.cards.iter().copied().collect();
                for card_id in card_ids {
                    match operation {
                        "add" => self.add_score_modifier(card_id, value as i32),
                        "remove" => self.add_score_modifier(card_id, -(value as i32)),
                        "set" => self.set_score_modifier(card_id, value as i32),
                        _ => {}
                    }
                }
            }
            "change_state" => {
                let _state_change = effect.state_change.as_deref().unwrap_or("");
                // Change card state to active/wait
            }
            "modify_required_hearts" => {
                let operation = effect.operation.as_deref().unwrap_or("decrease");
                let value = effect.value.unwrap_or(0);
                let heart_color = effect.heart_color.as_deref().unwrap_or("heart00");
                let target = effect.target.as_deref().unwrap_or("self");

                let player = if target == "self" {
                    if player_id == self.player1.id { &mut self.player1 } else { &mut self.player2 }
                } else {
                    if player_id == self.player1.id { &mut self.player2 } else { &mut self.player1 }
                };

                // Collect card IDs first to avoid borrow issues
                let card_ids: Vec<i16> = player.live_card_zone.cards.iter().copied().collect();

                let modifier: i8 = match operation {
                    "decrease" => -(value as i8),
                    "increase" => value as i8,
                    "set" => {
                        // For set, we need to clear and set the heart
                        // This is more complex, for now skip
                        return;
                    }
                    _ => return,
                };

                for card_id in card_ids {
                    let color = crate::zones::parse_heart_color(heart_color);
                    self.add_heart_modifier(card_id, color, modifier as i32);
                }
            }
            "set_required_hearts" => {
                let count = effect.count.unwrap_or(0);
                let heart_color = effect.heart_color.as_deref().unwrap_or("heart00");
                let target = effect.target.as_deref().unwrap_or("self");

                let player = if target == "self" {
                    if player_id == self.player1.id { &mut self.player1 } else { &mut self.player2 }
                } else {
                    if player_id == self.player1.id { &mut self.player2 } else { &mut self.player1 }
                };

                // Collect card IDs first to avoid borrow issues
                let card_ids: Vec<i16> = player.live_card_zone.cards.iter().copied().collect();

                // For set, we need to clear existing modifiers and set new ones
                // This is complex, for now just add the modifier
                let modifier = count as i8;

                for card_id in card_ids {
                    let color = crate::zones::parse_heart_color(heart_color);
                    self.add_heart_modifier(card_id, color, modifier as i32);
                }
            }
            "modify_required_hearts_global" => {
                let operation = effect.operation.as_deref().unwrap_or("increase");
                let value = effect.value.unwrap_or(1);
                let heart_color = effect.heart_color.as_deref().unwrap_or("heart00");
                let target = effect.target.as_deref().unwrap_or("opponent");

                let player = if target == "opponent" {
                    if player_id == self.player1.id { &mut self.player2 } else { &mut self.player1 }
                } else {
                    if player_id == self.player1.id { &mut self.player1 } else { &mut self.player2 }
                };

                // Collect card IDs first to avoid borrow issues
                let card_ids: Vec<i16> = player.live_card_zone.cards.iter().copied().collect();

                let color = crate::zones::parse_heart_color(heart_color);
                let modifier: i32 = match operation {
                    "increase" => value as i32,
                    "decrease" => -(value as i32),
                    _ => return,
                };

                for card_id in card_ids {
                    self.add_need_heart_modifier(card_id, color, modifier);
                }
            }
            "set_blade_type" => {
                let blade_type = effect.blade_type.as_deref().unwrap_or("");
                let target = effect.target.as_deref().unwrap_or("self");
                // Track as temporary effect
                let temp_effect = TemporaryEffect {
                    effect_type: format!("set_blade_type:{}", blade_type),
                    duration: effect.duration.clone().map(|d| match d.as_str() {
                        "live_end" => Duration::LiveEnd,
                        "this_turn" => Duration::ThisTurn,
                        "this_live" => Duration::ThisLive,
                        "permanent" => Duration::Permanent,
                        _ => Duration::ThisLive,
                    }).unwrap_or(Duration::ThisLive),
                    created_turn: self.turn_number,
                    created_phase: self.current_phase.clone(),
                    target_player_id: if target == "self" { player_id.to_string() } else { 
                        if player_id == self.player1.id { self.player2.id.clone() } else { self.player1.id.clone() }
                    },
                    description: format!("Set blade type to {}", blade_type),
                };
                self.temporary_effects.push(temp_effect);
            }
            "set_heart_type" => {
                let heart_type = effect.heart_color.as_deref().unwrap_or("heart00");
                let count = effect.count.unwrap_or(1);
                let target = effect.target.as_deref().unwrap_or("self");
                let color = crate::zones::parse_heart_color(heart_type);
                
                // Collect card IDs first to avoid borrow conflicts
                let card_ids_to_modify: Vec<i16> = {
                    let player = if target == "self" {
                        if player_id == self.player1.id { &self.player1 } else { &self.player2 }
                    } else {
                        if player_id == self.player1.id { &self.player2 } else { &self.player1 }
                    };
                    
                    let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
                    areas.iter().filter_map(|&area| {
                        player.stage.get_area(area)
                    }).collect()
                };
                
                // Now apply modifiers after the borrow is released
                for card_id in card_ids_to_modify {
                    self.add_heart_modifier(card_id, color.clone(), count as i32);
                }
            }
            "position_change" => {
                let _position = effect.position.as_ref().and_then(|p| p.position.as_deref()).unwrap_or("");
                // Position change requires user choice - simplified for now
            }
            "place_energy_under_member" => {
                let energy_count = effect.energy_count.unwrap_or(1);
                let _target_member = effect.target_member.as_deref().unwrap_or("this_member");
                
                let player = if player_id == self.player1.id { &mut self.player1 } else { &mut self.player2 };
                
                for _ in 0..energy_count {
                    if let Some(energy_card) = player.energy_deck.draw() {
                        player.energy_zone.cards.push(energy_card);
                    }
                }
            }
            "modify_yell_count" => {
                let operation = effect.operation.as_deref().unwrap_or("subtract");
                let count = effect.count.unwrap_or(0);
                
                match operation {
                    "add" => {
                        self.cheer_checks_required += count;
                    }
                    "subtract" => {
                        self.cheer_checks_required = self.cheer_checks_required.saturating_sub(count);
                    }
                    "set" => {
                        self.cheer_checks_required = count;
                    }
                    _ => {}
                }
            }
            "conditional_alternative" => {
                // Conditional alternative - requires condition evaluation
            }
            "modify_cost" => {
                let _count = effect.count.unwrap_or(1);
                // Modify card cost - would need to track cost modifiers
            }
            "draw_until_count" => {
                let count = effect.count.unwrap_or(5);
                let player = if player_id == self.player1.id {
                    &mut self.player1
                } else {
                    &mut self.player2
                };
                while player.hand.cards.len() < count as usize {
                    let _ = player.draw_card();
                }
            }
            "play_baton_touch" => {
                // Play baton touch - replace a member on stage with another from hand
                // This is a complex effect that requires player choice
            }
            "activation_cost" => {
                let _count = effect.count.unwrap_or(0);
                // Activation cost is handled separately in cost payment
            }
            "custom" => {
                // Custom effect - game-specific handling
            }
            _ => {
            }
        }
    }

    pub fn check_victory(&self) -> GameResult {
        // Rule 1.2.1.1: Victory condition
        // Player wins if they have 3+ success live cards AND opponent has 2 or fewer
        let p1_success = self.player1.success_live_card_zone.len();
        let p2_success = self.player2.success_live_card_zone.len();

        let p1_wins = p1_success >= 3 && p2_success <= 2;
        let p2_wins = p2_success >= 3 && p1_success <= 2;

        // Rule 1.2.1.2: If both have 3+ simultaneously, it's a draw
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

    // ============== TARGET RESOLUTION ==============

    /// Resolve target string to player reference(s)
    /// Returns a vector of player references (0-2 players)
    /// For "self" or "opponent", returns single player
    /// For "both", returns both players
    /// For "either", returns both players (caller must choose)
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

    /// Resolve target string to mutable player reference(s)
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

    /// Get player by ID
    pub fn get_player(&self, player_id: &str) -> Option<&Player> {
        if self.player1.id == player_id {
            Some(&self.player1)
        } else if self.player2.id == player_id {
            Some(&self.player2)
        } else {
            None
        }
    }

    /// Get mutable player by ID
    pub fn get_player_mut(&mut self, player_id: &str) -> Option<&mut Player> {
        if self.player1.id == player_id {
            Some(&mut self.player1)
        } else if self.player2.id == player_id {
            Some(&mut self.player2)
        } else {
            None
        }
    }

    // ============== TRIGGER DETECTION ==============

    /// Check if debut trigger should occur (member placed on stage from non-stage zone)
    pub fn should_trigger_debut(&self, _player: &Player, card: &crate::card::Card) -> bool {
        // Rule 11.4: Debut - member placed on stage from non-stage zone
        card.is_member()
    }

    /// Check if live start trigger should occur
    pub fn should_trigger_live_start(&self, _player: &Player) -> bool {
        // Rule 11.5: Live Start - at start of performance phase when active player
        self.current_phase == Phase::FirstAttackerPerformance
            || self.current_phase == Phase::SecondAttackerPerformance
    }

    /// Check if live success trigger should occur
    pub fn should_trigger_live_success(&self, _player: &Player) -> bool {
        // Rule 11.6: Live Success - when player's live is successful
        // This is determined by comparing live scores during LiveVictoryDetermination phase
        self.current_phase == Phase::LiveVictoryDetermination
    }

    /// Get all triggerable abilities for a card given current game state
    pub fn get_triggerable_abilities<'a>(
        &self,
        card: &'a crate::card::Card,
        trigger: AbilityTrigger,
        player: &Player,
    ) -> Vec<&'a crate::card::Ability> {
        card.abilities.iter().filter(|ability| {
            match trigger {
                AbilityTrigger::Activation => {
                    // Activation abilities can always be triggered (subject to conditions)
                    ability.triggers.as_ref().map_or(false, |t| t == "起動")
                }
                AbilityTrigger::Debut => {
                    ability.triggers.as_ref().map_or(false, |t| t == "登場")
                        && self.should_trigger_debut(player, card)
                }
                AbilityTrigger::LiveStart => {
                    ability.triggers.as_ref().map_or(false, |t| t == "ライブ開始時")
                        && self.should_trigger_live_start(player)
                }
                AbilityTrigger::LiveSuccess => {
                    ability.triggers.as_ref().map_or(false, |t| t == "ライブ成功時")
                        && self.should_trigger_live_success(player)
                }
                AbilityTrigger::PerformancePhaseStart => {
                    ability.triggers.as_ref().map_or(false, |t| t == "パフォーマンスフェイズの始めに")
                }
            }
        }).collect()
    }

    // ============== DURATION MANAGEMENT ==============

    /// Add a temporary effect with duration
    pub fn add_temporary_effect(
        &mut self,
        effect_type: String,
        duration: Duration,
        target_player_id: String,
        description: String,
    ) {
        self.temporary_effects.push(TemporaryEffect {
            effect_type,
            duration,
            created_turn: self.turn_number,
            created_phase: self.current_phase.clone(),
            target_player_id,
            description,
        });
    }

    /// Check and remove expired effects based on current game state
    pub fn check_expired_effects(&mut self) {
        let mut expired_indices = Vec::new();

        for (i, effect) in self.temporary_effects.iter().enumerate() {
            let is_expired = match effect.duration {
                Duration::LiveEnd => {
                    // Expire when leaving Live phase
                    self.current_turn_phase != TurnPhase::Live
                }
                Duration::ThisTurn => {
                    // Expire when turn number changes
                    self.turn_number > effect.created_turn
                }
                Duration::ThisLive => {
                    // Expire when leaving Live phase
                    self.current_turn_phase != TurnPhase::Live
                }
                Duration::Permanent => false,
            };

            if is_expired {
                expired_indices.push(i);
            }
        }

        // Remove expired effects (in reverse order to maintain indices)
        for i in expired_indices.into_iter().rev() {
            let effect = self.temporary_effects.remove(i);
            // Revert the effect based on effect type
            // For now, this is a simplified implementation
            // Full implementation would track and revert specific effect changes
            match effect.effect_type.as_str() {
                "activation_cost_increase" => {
                    // Remove cost increase from prohibition effects
                    self.prohibition_effects.retain(|p| !p.contains(&effect.effect_type));
                }
                "activation_cost_decrease" => {
                    // Remove cost decrease from prohibition effects
                    self.prohibition_effects.retain(|p| !p.contains(&effect.effect_type));
                }
                _ => {
                    // Log other effect expirations
                    eprintln!("Expired effect: {}", effect.description);
                }
            }
        }
    }

    /// Get active temporary effects for a specific player
    pub fn get_active_effects_for_player(&self, player_id: &str) -> Vec<&TemporaryEffect> {
        self.temporary_effects
            .iter()
            .filter(|e| e.target_player_id == player_id)
            .collect()
    }

    /// Save current state to history before making a change
    pub fn save_state(&mut self) {
        // Clear future when making a new change
        self.future.clear();
        
        // Add current state to history
        self.history.push(self.clone());
        
        // Limit history size
        if self.history.len() > self.max_history_size {
            self.history.drain(..1);
        }
    }

    /// Undo to previous state
    pub fn undo(&mut self) -> Result<(), String> {
        if self.history.is_empty() {
            return Err("No history to undo".to_string());
        }
        
        // Save current state to future
        self.future.push(self.clone());
        
        // Restore previous state
        let previous = self.history.pop().unwrap();
        *self = previous;
        
        Ok(())
    }

    /// Redo to next state
    pub fn redo(&mut self) -> Result<(), String> {
        if self.future.is_empty() {
            return Err("No future to redo".to_string());
        }
        
        // Save current state to history
        self.history.push(self.clone());
        
        // Restore next state
        let next = self.future.pop().unwrap();
        *self = next;
        
        Ok(())
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }
}
