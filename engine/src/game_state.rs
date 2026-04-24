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
    Constant,            // 常時
    Auto,                // 自動 (generic auto ability)
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
    ChooseFirstAttacker,  // Q16: RPS winner chooses turn order
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
    pub creation_order: u32, // For effect layering - order in which effects were generated
}

#[derive(Debug, Clone)]
pub struct ReplacementEffect {
    pub card_id: i16,
    pub player_id: String,
    pub original_event: String, // The event being replaced (e.g., "draw_card", "pay_energy")
    pub replacement_effects: Vec<crate::card::AbilityEffect>, // The replacement action(s)
    pub is_choice_based: bool, // Whether this is a choice-based replacement (Rule 9.10.3)
    pub applied_this_event: bool, // Track if already applied to current event (Rule 9.10.2.3)
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
    // RPS tracking - Q16: "じゃんけんで勝ったプレイヤーが先攻か後攻を決めます"
    pub rps_winner: Option<u8>, // 1 = player1, 2 = player2
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
    pub blade_type_modifiers: std::collections::HashMap<i16, crate::card::BladeColor>, // card_id -> blade type override
    pub heart_modifiers: std::collections::HashMap<i16, std::collections::HashMap<crate::card::HeartColor, i32>>, // card_id -> color -> heart delta
    // Activating card tracking - for self-cost abilities where the card itself is the cost
    pub activating_card: Option<i16>, // card_id of the card currently activating an ability
    pub score_modifiers: std::collections::HashMap<i16, i32>, // card_id -> score delta
    pub need_heart_modifiers: std::collections::HashMap<i16, std::collections::HashMap<crate::card::HeartColor, i32>>, // card_id -> color -> need_heart delta
    pub orientation_modifiers: std::collections::HashMap<i16, String>, // card_id -> "active" or "wait"
    pub cost_modifiers: std::collections::HashMap<i16, i32>, // card_id -> cost delta
    // Reveal tracking - tracks which cards have been revealed to opponent
    pub revealed_cards: std::collections::HashSet<i16>, // card_ids that are currently revealed
    // Optional cost behavior: "always_pay", "never_pay", "auto" (pay if beneficial)
    pub optional_cost_behavior: String,
    // Pending ability execution state for user interaction
    pub pending_ability: Option<PendingAbilityExecution>,
    // Pending choice for user interaction - persists across resolver instances
    pub pending_choice: Option<crate::ability_resolver::Choice>,
    // Pending sequential actions to resume after user choice
    pub pending_sequential_actions: Option<Vec<crate::card::AbilityEffect>>,
    // Area placement tracking - tracks which areas had cards placed this turn (Q70, Q71, Q75, Q76, Q79, Q80)
    pub areas_placed_this_turn: std::collections::HashSet<String>, // "player1:center", "player1:left", etc.
    // Card appearance tracking - tracks which cards appeared this turn (Q77)
    pub cards_appeared_this_turn: std::collections::HashSet<i16>,
    // Turn order change tracking (Q49, Q50, Q51)
    pub turn_order_changed: bool,
    // Ability trigger tracking (Q94) - tracks how many times an auto ability has triggered this turn
    pub auto_ability_trigger_counts: std::collections::HashMap<String, u32>, // card_id -> trigger count
    // Baton touch cost tracking - tracks if baton touch resulted in 0 cost (Q25)
    pub baton_touch_zero_cost: bool, // true if the most recent baton touch had 0 cost
    // Turn limit tracking per card instance (Q58, Q59) - tracks how many times a card instance has used turn-limited abilities
    pub turn_limit_usage: std::collections::HashMap<String, u32>, // "player1:card_instance_id" -> usage count
    // Card identity tracking for zone movement (Q59) - tracks card instance IDs
    pub card_instance_counter: u32, // Counter for generating unique card instance IDs
    pub card_instance_mapping: std::collections::HashMap<i16, u32>, // card_id -> instance_id
    // Baton touch tracking per turn (Q87) - tracks how many times baton touch has been used this turn
    pub baton_touch_count: u32, // Number of baton touches performed this turn
    // Card movement tracking - tracks which cards have moved this turn (for not_moved/has_moved conditions)
    pub cards_moved_this_turn: std::collections::HashSet<i16>, // card_ids that have moved this turn
    // Heart color decision tracking (Q46, Q67) - tracks when heart color decisions are made
    pub heart_color_decision_phase: String, // "live_start" or "required_hearts_check"
    // Deck refresh tracking (Q53) - tracks whether a deck refresh is pending
    pub deck_refresh_pending: bool, // Whether a deck refresh needs to be performed
    // Position/Formation change tracking for keyword validation
    pub position_change_occurred_this_turn: bool, // Whether position change occurred this turn
    pub formation_change_occurred_this_turn: bool, // Whether formation change occurred this turn
    // Partial effect resolution tracking (Q55, Q92, Q93) - tracks whether partial resolution is allowed
    pub partial_resolution_allowed: bool, // Whether effects can be partially resolved
    // Cost payment validation tracking (Q56) - tracks whether full cost payment is required
    pub full_cost_payment_required: bool, // Whether full cost must be paid (no partial payment)
    // Mandatory auto ability tracking (Q60) - tracks whether auto abilities are mandatory
    pub auto_abilities_mandatory: bool, // Whether non-turn-limited auto abilities must be used when triggered
    // Deck size-aware search tracking (Q85, Q86) - tracks search count adjustments
    pub search_count_adjustment_enabled: bool, // Whether search effects adjust to deck size
    // Area occupation rules tracking (Q28) - tracks whether replacement placement is allowed
    pub allow_replacement_placement: bool, // Whether placement to occupied area with cost payment is allowed
    // Live card placement tracking (Q72) - tracks whether live cards can be set without stage members
    pub allow_live_without_stage_members: bool, // Whether live cards can be set with empty stage
    // Live execution tracking (Q91) - tracks whether a live is being performed
    pub live_being_performed: bool, // Whether a live is currently being performed
    // Win condition tracking (Q54) - tracks game end state
    pub game_ended: bool, // Whether the game has ended
    pub draw_state: bool, // Whether the game is in a draw state
    // Effect precedence tracking (Q57) - tracks effect precedence rules
    pub prohibition_precedence_enabled: bool, // Whether prohibition effects take precedence over enabling effects
    // Effect interruption tracking (Q73) - tracks effect resumption state
    pub effect_resumption_state: String, // "none", "interrupted_for_refresh", "resumed"
    // Ability source tracking (Q78) - tracks whether abilities are inherent or gained
    pub gained_abilities: std::collections::HashMap<i16, Vec<String>>, // Card ID -> list of gained ability types
    // Card set search tracking (Q82) - tracks whether search includes all card types in a set
    pub card_set_search_enabled: bool, // Whether search effects include all card types in a set (e.g., live cards for group search)
    // Multi-card victory selection tracking (Q83) - tracks whether player must select one card for success zone
    pub multi_victory_selection_enabled: bool, // Whether player must select one card when multiple live cards win
    // Auto ability ordering tracking (Q84) - tracks whether turn player chooses order first
    pub turn_player_priority_enabled: bool, // Whether turn player chooses auto ability order first
    // Action validation tracking (Q88) - tracks whether arbitrary player actions are restricted
    pub arbitrary_actions_restricted: bool, // Whether players can only perform actions allowed by game rules
    // Replacement effects tracking (Rule 9.10)
    pub replacement_effects: Vec<ReplacementEffect>, // Active replacement effects
    // Effect creation order counter for layering (Rule 9.9.1.7)
    pub effect_creation_counter: u32, // Counter for tracking order of effect creation
    // Permanent loop detection (Rule 12.1)
    pub game_state_history: Vec<String>, // Track game states for loop detection
    pub max_state_history_size: usize, // Limit history size for loop detection
    pub loop_detected: bool, // Whether a permanent loop has been detected
}

#[derive(Debug, Clone)]
pub struct PendingAbilityExecution {
    pub card_no: String,
    pub player_id: String,
    pub action_index: usize,
    pub effect: crate::card::AbilityEffect,
    pub conditional_choice: Option<String>, // Track choice for conditional_alternative
    pub cost: Option<crate::card::AbilityCost>, // Track cost for manual ability activation
    pub cost_choice: Option<String>, // Track user's cost selection (e.g., "wait" or "discard")
    pub activating_card: Option<i16>, // Track which card is activating the ability (for self-cost)
    pub ability_index: usize, // Track which ability index is being executed
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
            activating_card: None,
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
            rps_winner: None,  // Q16: RPS winner chooses turn order
            live_card_set_player1_done: false,
            live_card_set_player2_done: false,
            history: Vec::new(),
            future: Vec::new(),
            max_history_size: 50,
            card_database,
            blade_modifiers: std::collections::HashMap::new(),
            blade_type_modifiers: std::collections::HashMap::new(),
            heart_modifiers: std::collections::HashMap::new(),
            score_modifiers: std::collections::HashMap::new(),
            need_heart_modifiers: std::collections::HashMap::new(),
            orientation_modifiers: std::collections::HashMap::new(),
            cost_modifiers: std::collections::HashMap::new(),
            revealed_cards: std::collections::HashSet::new(),
            optional_cost_behavior: "always_pay".to_string(), // Default to always pay for bot/test mode
            pending_ability: None,
            pending_choice: None,
            pending_sequential_actions: None,
            areas_placed_this_turn: std::collections::HashSet::new(),
            cards_appeared_this_turn: std::collections::HashSet::new(),
            turn_order_changed: false,
            auto_ability_trigger_counts: std::collections::HashMap::new(),
            baton_touch_zero_cost: false,
            turn_limit_usage: std::collections::HashMap::new(),
            card_instance_counter: 0,
            card_instance_mapping: std::collections::HashMap::new(),
            baton_touch_count: 0,
            cards_moved_this_turn: std::collections::HashSet::new(),
            heart_color_decision_phase: "none".to_string(),
            deck_refresh_pending: false,
            partial_resolution_allowed: true,
            full_cost_payment_required: true,
            auto_abilities_mandatory: true,
            search_count_adjustment_enabled: true,
            allow_replacement_placement: true,
            allow_live_without_stage_members: true,
            live_being_performed: false,
            game_ended: false,
            draw_state: false,
            prohibition_precedence_enabled: true,
            effect_resumption_state: "none".to_string(),
            gained_abilities: std::collections::HashMap::new(),
            card_set_search_enabled: true,
            multi_victory_selection_enabled: true,
            turn_player_priority_enabled: true,
            arbitrary_actions_restricted: true,
            replacement_effects: Vec::new(),
            position_change_occurred_this_turn: false,
            formation_change_occurred_this_turn: false,
            effect_creation_counter: 0,
            game_state_history: Vec::new(),
            max_state_history_size: 100,
            loop_detected: false,
        }
    }

    pub fn active_player(&self) -> &Player {
        // Rule 7.2: Determine active player based on turn phase
        match self.current_turn_phase {
            TurnPhase::FirstAttackerNormal => self.first_attacker(),
            TurnPhase::SecondAttackerNormal => self.second_attacker(),
            TurnPhase::Live => {
                // During live card set phase, determine which player is currently setting cards
                // based on completion flags
                let p1_is_first = self.player1.is_first_attacker;
                let p1_done = self.live_card_set_player1_done;
                let p2_done = self.live_card_set_player2_done;

                if !p1_done && p2_done {
                    // P1 is currently taking their turn (P2 already done)
                    &self.player1
                } else if !p2_done && p1_done {
                    // P2 is currently taking their turn (P1 already done)
                    &self.player2
                } else if !p1_done && !p2_done {
                    // Neither has finished yet - first attacker goes first
                    if p1_is_first {
                        &self.player1
                    } else {
                        &self.player2
                    }
                } else {
                    // Both done - shouldn't happen during live card set
                    if p1_is_first {
                        &self.player1
                    } else {
                        &self.player2
                    }
                }
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
                // During live card set phase, determine which player is currently setting cards
                // based on completion flags
                let p1_is_first = self.player1.is_first_attacker;
                let p1_done = self.live_card_set_player1_done;
                let p2_done = self.live_card_set_player2_done;

                if !p1_done && p2_done {
                    // P1 is currently taking their turn (P2 already done)
                    &mut self.player1
                } else if !p2_done && p1_done {
                    // P2 is currently taking their turn (P1 already done)
                    &mut self.player2
                } else if !p1_done && !p2_done {
                    // Neither has finished yet - first attacker goes first
                    if p1_is_first {
                        &mut self.player1
                    } else {
                        &mut self.player2
                    }
                } else {
                    // Both done - shouldn't happen during live card set
                    if p1_is_first {
                        &mut self.player1
                    } else {
                        &mut self.player2
                    }
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

    pub fn can_activate_area_ability(&self, player_id: &str, card_no: &str, area: crate::zones::MemberArea) -> bool {
        // Rule 11.7.2, 11.8.2, 11.9.2: Area-specific ability can only activate if member is in that area
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

    // Convenience wrappers for backward compatibility
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
        // Reset keyword tracking at start of new turn
        self.turn1_abilities_played.clear();
        self.turn2_abilities_played.clear();
        // Reset cheer blade heart counts at start of new turn
        self.player1_cheer_blade_heart_count = 0;
        self.player2_cheer_blade_heart_count = 0;
        // Reset position/formation change flags at start of new turn
        self.reset_change_flags();
        // Reset cheer check state
        self.cheer_check_completed = false;
        // Reset loop detection at start of new turn
        self.reset_loop_detection();
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

    pub fn set_blade_type_modifier(&mut self, card_id: i16, blade_color: crate::card::BladeColor) {
        self.blade_type_modifiers.insert(card_id, blade_color);
    }

    pub fn get_blade_type_modifier(&self, card_id: i16) -> Option<crate::card::BladeColor> {
        self.blade_type_modifiers.get(&card_id).copied()
    }

    pub fn clear_blade_type_modifier(&mut self, card_id: i16) {
        self.blade_type_modifiers.remove(&card_id);
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

    // Area placement tracking methods (Q70, Q71, Q75, Q76, Q79, Q80)
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

    // Card appearance tracking methods (Q77)
    pub fn record_card_appearance(&mut self, card_id: i16) {
        self.cards_appeared_this_turn.insert(card_id);
    }

    pub fn has_card_appeared_this_turn(&self, card_id: i16) -> bool {
        self.cards_appeared_this_turn.contains(&card_id)
    }

    pub fn clear_card_appearance_tracking(&mut self) {
        self.cards_appeared_this_turn.clear();
    }

    // Live score state tracking (Q47, Q48) - tracks whether a player has a valid live score
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

    // Turn order change tracking (Q49, Q50, Q51)
    pub fn set_turn_order_changed(&mut self, changed: bool) {
        self.turn_order_changed = changed;
    }

    pub fn has_turn_order_changed(&self) -> bool {
        self.turn_order_changed
    }

    // Ability trigger tracking methods (Q94)
    pub fn record_auto_ability_trigger(&mut self, card_id: &str) {
        *self.auto_ability_trigger_counts.entry(card_id.to_string()).or_insert(0) += 1;
    }

    pub fn get_auto_ability_trigger_count(&self, card_id: &str) -> u32 {
        *self.auto_ability_trigger_counts.get(card_id).unwrap_or(&0)
    }

    pub fn clear_auto_ability_trigger_tracking(&mut self) {
        self.auto_ability_trigger_counts.clear();
    }

    // Turn limit tracking methods (Q58, Q59)
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

    // Card identity tracking methods (Q59)
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

    // Baton touch tracking methods (Q87)
    pub fn record_baton_touch(&mut self) {
        self.baton_touch_count += 1;
    }

    pub fn get_baton_touch_count(&self) -> u32 {
        self.baton_touch_count
    }

    pub fn clear_baton_touch_tracking(&mut self) {
        self.baton_touch_count = 0;
    }

    // Card movement tracking methods (for not_moved/has_moved conditions)
    pub fn record_card_movement(&mut self, card_id: i16) {
        self.cards_moved_this_turn.insert(card_id);
    }

    pub fn has_card_moved_this_turn(&self, card_id: i16) -> bool {
        self.cards_moved_this_turn.contains(&card_id)
    }

    pub fn clear_card_movement_tracking(&mut self) {
        self.cards_moved_this_turn.clear();
    }

    // Heart color decision tracking methods (Q46, Q67)
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

    // Deck refresh tracking methods (Q53)
    pub fn set_deck_refresh_pending(&mut self, pending: bool) {
        self.deck_refresh_pending = pending;
    }

    pub fn is_deck_refresh_pending(&self) -> bool {
        self.deck_refresh_pending
    }

    pub fn perform_deck_refresh(&mut self, player_id: &str) {
        // Move all cards from waitroom to main deck and shuffle
        let player = if player_id == "player1" {
            &mut self.player1
        } else {
            &mut self.player2
        };

        // Move all waitroom cards to main deck
        let waitroom_cards: Vec<i16> = player.waitroom.cards.iter().copied().collect();
        player.waitroom.cards.clear();
        for card_id in waitroom_cards {
            player.main_deck.cards.push(card_id);
        }

        // Shuffle the main deck
        player.main_deck.shuffle();

        // Clear the pending flag
        self.deck_refresh_pending = false;
    }

    // Partial effect resolution tracking methods (Q55, Q92, Q93)
    pub fn set_partial_resolution_allowed(&mut self, allowed: bool) {
        self.partial_resolution_allowed = allowed;
    }

    pub fn is_partial_resolution_allowed(&self) -> bool {
        self.partial_resolution_allowed
    }

    // Cost payment validation tracking methods (Q56)
    pub fn set_full_cost_payment_required(&mut self, required: bool) {
        self.full_cost_payment_required = required;
    }

    pub fn is_full_cost_payment_required(&self) -> bool {
        self.full_cost_payment_required
    }

    // Mandatory auto ability tracking methods (Q60)
    pub fn set_auto_abilities_mandatory(&mut self, mandatory: bool) {
        self.auto_abilities_mandatory = mandatory;
    }

    pub fn are_auto_abilities_mandatory(&self) -> bool {
        self.auto_abilities_mandatory
    }

    // Deck size-aware search tracking methods (Q85, Q86)
    pub fn set_search_count_adjustment_enabled(&mut self, enabled: bool) {
        self.search_count_adjustment_enabled = enabled;
    }

    pub fn is_search_count_adjustment_enabled(&self) -> bool {
        self.search_count_adjustment_enabled
    }

    pub fn adjust_search_count(&self, requested_count: usize, deck_size: usize) -> usize {
        if self.search_count_adjustment_enabled {
            std::cmp::min(requested_count, deck_size)
        } else {
            requested_count
        }
    }

    // Area occupation rules tracking methods (Q28)
    pub fn set_allow_replacement_placement(&mut self, allowed: bool) {
        self.allow_replacement_placement = allowed;
    }

    pub fn is_replacement_placement_allowed(&self) -> bool {
        self.allow_replacement_placement
    }

    // Live card placement tracking methods (Q72)
    pub fn set_allow_live_without_stage_members(&mut self, allowed: bool) {
        self.allow_live_without_stage_members = allowed;
    }

    pub fn is_live_without_stage_members_allowed(&self) -> bool {
        self.allow_live_without_stage_members
    }

    // Live execution tracking methods (Q91)
    pub fn set_live_being_performed(&mut self, performed: bool) {
        self.live_being_performed = performed;
    }

    pub fn is_live_being_performed(&self) -> bool {
        self.live_being_performed
    }

    // Win condition tracking methods (Q54)
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

    pub fn check_success_zone_draw_condition(&self, _player_id: &str) -> bool {
        // Q54: Draw condition when 3+ success cards (2+ in half deck)
        // This is a simplified check - actual implementation would depend on deck type
        // Note: Player doesn't have a success_zone field, so this is a placeholder
        // The actual implementation would need to track success cards separately
        false // Placeholder - would need proper success zone tracking
    }

    // Effect precedence tracking methods (Q57)
    pub fn set_prohibition_precedence_enabled(&mut self, enabled: bool) {
        self.prohibition_precedence_enabled = enabled;
    }

    pub fn is_prohibition_precedence_enabled(&self) -> bool {
        self.prohibition_precedence_enabled
    }

    // Effect interruption tracking methods (Q73)
    pub fn set_effect_resumption_state(&mut self, state: String) {
        self.effect_resumption_state = state;
    }

    pub fn get_effect_resumption_state(&self) -> &str {
        &self.effect_resumption_state
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

    // Ability source tracking methods (Q78)
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

    // Card set search tracking methods (Q82)
    pub fn set_card_set_search_enabled(&mut self, enabled: bool) {
        self.card_set_search_enabled = enabled;
    }

    pub fn is_card_set_search_enabled(&self) -> bool {
        self.card_set_search_enabled
    }

    // Multi-card victory selection tracking methods (Q83)
    pub fn set_multi_victory_selection_enabled(&mut self, enabled: bool) {
        self.multi_victory_selection_enabled = enabled;
    }

    pub fn is_multi_victory_selection_enabled(&self) -> bool {
        self.multi_victory_selection_enabled
    }

    // Auto ability ordering tracking methods (Q84)
    pub fn set_turn_player_priority_enabled(&mut self, enabled: bool) {
        self.turn_player_priority_enabled = enabled;
    }

    pub fn is_turn_player_priority_enabled(&self) -> bool {
        self.turn_player_priority_enabled
    }

    // Action validation tracking methods (Q88)
    pub fn set_arbitrary_actions_restricted(&mut self, restricted: bool) {
        self.arbitrary_actions_restricted = restricted;
    }

    pub fn are_arbitrary_actions_restricted(&self) -> bool {
        self.arbitrary_actions_restricted
    }

    pub fn set_need_heart_modifier(&mut self, card_id: i16, color: crate::card::HeartColor, value: i32) {
        let colors = self.need_heart_modifiers.entry(card_id).or_insert_with(std::collections::HashMap::new);
        colors.insert(color, value);
    }

    pub fn add_orientation_modifier(&mut self, card_id: i16, orientation: &str) {
        self.orientation_modifiers.insert(card_id, orientation.to_string());
    }

    pub fn add_cost_modifier(&mut self, card_id: i16, delta: i32) {
        *self.cost_modifiers.entry(card_id).or_insert(0) += delta;
    }

    pub fn set_cost_modifier(&mut self, card_id: i16, value: i32) {
        self.cost_modifiers.insert(card_id, value);
    }

    pub fn get_cost_modifier(&self, card_id: i16) -> i32 {
        *self.cost_modifiers.get(&card_id).unwrap_or(&0)
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
        self.cost_modifiers.remove(&card_id);
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
    
    pub fn execute_card_ability(&mut self, card_no: &str, player_id: &str) {
        // Find the card by card_no and get its card_id
        let (card, card_id) = {
            let player = if player_id == self.player1.id {
                &self.player1
            } else {
                &self.player2
            };
            
            let mut found_card = None;
            let mut found_card_id = None;
            
            // Check hand
            for id in &player.hand.cards {
                if let Some(card) = self.card_database.get_card(*id) {
                    if card.card_no == card_no {
                        found_card = Some(card);
                        found_card_id = Some(*id);
                        break;
                    }
                }
            }
            
            // Check stage
            if found_card.is_none() {
                for stage_card_id in &player.stage.stage {
                    if *stage_card_id != -1 {
                        if let Some(card) = self.card_database.get_card(*stage_card_id) {
                            if card.card_no == card_no {
                                found_card = Some(card);
                                found_card_id = Some(*stage_card_id);
                                break;
                            }
                        }
                    }
                }
            }
            
            // Check waitroom (card may be there after cost payment)
            if found_card.is_none() {
                for waitroom_card_id in &player.waitroom.cards {
                    if let Some(card) = self.card_database.get_card(*waitroom_card_id) {
                        if card.card_no == card_no {
                            found_card = Some(card);
                            found_card_id = Some(*waitroom_card_id);
                            break;
                        }
                    }
                }
            }
            
            (found_card, found_card_id)
        };
        
        if let Some(card) = card {
            let abilities = card.abilities.clone();
            for ability in abilities.iter() {
                // Check if there's a pending ability execution
                let pending = self.pending_ability.clone();
                if let Some(ref pending) = pending {
                    // Resume from pending execution
                    if pending.card_no == card_no && pending.player_id == player_id {
                        // Restore activating card from pending state
                        self.activating_card = pending.activating_card;
                        
                        if let Some(ref effect) = ability.effect {
                            let mut resolver = crate::ability_resolver::AbilityResolver::new(self);

                            // Handle conditional_alternative choice
                            if effect.action == "conditional_alternative" {
                                if let Some(ref choice) = pending.conditional_choice {
                                    if choice == "alternative" {
                                        if let Some(ref alt_effect) = effect.alternative_effect {
                                            if let Err(e) = resolver.execute_effect(alt_effect) {
                                                eprintln!("Failed to execute alternative effect: {}", e);
                                            }
                                        }
                                    } else {
                                        if let Err(e) = resolver.execute_effect(effect) {
                                            eprintln!("Failed to execute primary effect: {}", e);
                                        }
                                    }
                                }
                            } else {
                                if let Err(e) = resolver.execute_effect(effect) {
                                    eprintln!("Failed to execute effect: {}", e);
                                }
                            }

                            // Clear pending ability after execution
                            self.pending_ability = None;
                            self.activating_card = None;
                        }
                    }
                } else {
                    // Start new ability execution
                    let mut resolver = crate::ability_resolver::AbilityResolver::new(self);
                    if let Err(e) = resolver.resolve_ability(ability, card_id) {
                        eprintln!("Failed to resolve ability: {}", e);
                    }
                    // Check if there's a pending choice after execution
                    eprintln!("After resolve_ability, pending_choice: {:?}", resolver.get_pending_choice());
                    if let Some(_choice) = resolver.get_pending_choice() {
                        // Store pending ability state with the choice
                        if let Some(ref effect) = ability.effect {
                            self.pending_ability = Some(PendingAbilityExecution {
                                card_no: card_no.to_string(),
                                player_id: player_id.to_string(),
                                ability_index: 0, // Simplified
                                effect: effect.clone(),
                                conditional_choice: None,
                                activating_card: card_id, // Store activating card for resume
                                action_index: 0,
                                cost: None,
                                cost_choice: None,
                            });
                        }
                    }
                }
            }
        } else {
            eprintln!("Card not found: {}", card_no);
        }
    }

    pub fn provide_ability_choice_result(&mut self, result: crate::ability_resolver::ChoiceResult) -> Result<(), String> {
        // Check if this is a conditional_alternative choice
        if let Some(ref mut pending) = self.pending_ability {
            if let crate::ability_resolver::ChoiceResult::TargetSelected { target } = &result {
                if target == "primary" || target == "alternative" {
                    pending.conditional_choice = Some(target.clone());
                }
            }
        }
        
        // Check if this is an optional cost choice (pay or skip)
        if let Some(ref pending) = self.pending_ability.clone() {
            if pending.card_no == "optional_cost" {
                if let crate::ability_resolver::ChoiceResult::TargetSelected { target } = &result {
                    if target == "pay_optional_cost" {
                        // Pay the optional cost
                        if let Some(ref cost) = pending.cost {
                            let mut resolver = crate::ability_resolver::AbilityResolver::new(self);
                            if let Err(e) = resolver.pay_cost(cost) {
                                return Err(format!("Failed to pay optional cost: {}", e));
                            }
                        }
                        
                        // After paying cost, execute the effect
                        eprintln!("Executing effect after optional cost: {:?}", pending.effect);
                        let mut resolver = crate::ability_resolver::AbilityResolver::new(self);
                        if let Err(e) = resolver.execute_effect(&pending.effect) {
                            eprintln!("Failed to execute effect after optional cost: {}", e);
                        }
                    }
                    // If skip_optional_cost, just skip the cost payment
                    
                    // Clear the optional cost pending ability
                    self.pending_ability = None;
                    return Ok(());
                }
            }
        }
        
        // Check if this is a cost selection for manual ability activation
        if let Some(ref mut pending) = self.pending_ability {
            if pending.cost.is_some() && pending.cost_choice.is_none() {
                // This is a cost selection - store the choice and execute the cost
                if let crate::ability_resolver::ChoiceResult::TargetSelected { target } = &result {
                    // Store the user's cost choice (the index of the selected option)
                    pending.cost_choice = Some(target.clone());
                    
                    // Clone necessary data before mutable borrow
                    let card_no = pending.card_no.clone();
                    let player_id = pending.player_id.clone();
                    let cost = pending.cost.clone();
                    
                    // Execute the selected cost payment
                    if let Some(ref cost) = cost {
                        if let Some(ref cost_options) = cost.options {
                            // Parse the selected index
                            if let Ok(selected_index) = target.parse::<usize>() {
                                if selected_index < cost_options.len() {
                                    let selected_cost = &cost_options[selected_index];
                                    eprintln!("Executing selected cost option {}: {:?}", selected_index, selected_cost);
                                    
                                    // Pay the selected cost
                                    let mut resolver = crate::ability_resolver::AbilityResolver::new(self);
                                    if let Err(e) = resolver.pay_cost(selected_cost) {
                                        return Err(format!("Failed to pay cost: {}", e));
                                    }
                                    
                                    // Now trigger the ability effect after cost is paid
                                    let ability_id = format!("{}_activation", card_no);
                                    self.trigger_auto_ability(
                                        ability_id,
                                        crate::game_state::AbilityTrigger::Activation,
                                        player_id.clone(),
                                        Some(card_no),
                                    );
                                    
                                    // Process the triggered ability to execute the effect
                                    self.process_pending_auto_abilities(&player_id.clone());
                                    
                                    // Clear the pending ability after execution
                                    self.pending_ability = None;
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Execute the choice result using the resolver
        // Clone pending_choice before creating resolver to avoid borrow issues
        let pending_choice_clone = self.pending_choice.clone();
        
        let mut resolver = crate::ability_resolver::AbilityResolver::new(self);
        
        // Restore pending choice from GameState if it exists
        if let Some(choice) = pending_choice_clone {
            resolver.pending_choice = Some(choice);
        }
        
        resolver.provide_choice_result(result)?;
        
        // Resume ability execution after choice
        if let Some(pending) = self.pending_ability.clone() {
            self.execute_card_ability(&pending.card_no, &pending.player_id);
        }
        
        Ok(())
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
                let destination_choice = effect.destination_choice.unwrap_or(false);


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
                            "empty_area" | "メンバーのいないエリア" => {
                                // Move from waitroom to empty stage area
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
                                    // Find first empty stage area
                                    let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
                                    let mut placed = false;
                                    for area in areas {
                                        if player.stage.get_area(area).is_none() {
                                            // Found empty area, place card
                                            if let Some(pos) = player.waitroom.cards.iter().position(|&c| c == card_id) {
                                                player.waitroom.cards.remove(pos);
                                                player.stage.set_area(area, card_id);
                                                placed = true;
                                                break;
                                            }
                                        }
                                    }
                                    if !placed {
                                        // No empty area available, card stays in waitroom
                                        break;
                                    }
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
                    creation_order: 0,
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

    /// Check if a card can be placed in a zone based on constant ability restrictions
    pub fn can_place_card_in_zone(&self, card_id: i16, zone: &str, _player_id: &str) -> bool {
        if let Some(card) = self.card_database.get_card(card_id) {
            // Check all constant abilities (常時) for restrictions
            for ability in &card.abilities {
                if ability.triggers.as_ref().map_or(false, |t| t == "常時") {
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

    /// Enforce constant ability restrictions for all cards in play
    pub fn enforce_constant_ability_restrictions(&mut self) {
        // Check all cards in all zones for constant ability restrictions
        // This should be called before any action that could violate a restriction
        
        // Collect cards to check first (to avoid borrow checker issues)
        let cards_to_check: Vec<(String, Vec<(usize, i16)>)> = vec![
            (self.player1.id.clone(), self.player1.live_card_zone.cards.iter().enumerate().map(|(i, &id)| (i, id)).collect()),
            (self.player2.id.clone(), self.player2.live_card_zone.cards.iter().enumerate().map(|(i, &id)| (i, id)).collect()),
        ];
        
        // Check which cards need to be removed
        let mut cards_to_remove: Vec<(String, usize)> = Vec::new();
        for (player_id, cards) in cards_to_check {
            for (index, card_id) in cards {
                if !self.can_place_card_in_zone(card_id, "live_card_zone", &player_id) {
                    cards_to_remove.push((player_id.clone(), index));
                }
            }
        }
        
        // Remove cards that violate restrictions
        for (player_id, index) in cards_to_remove {
            let player = if player_id == self.player1.id { &mut self.player1 } else { &mut self.player2 };
            let card = player.live_card_zone.cards.remove(index);
            player.waitroom.cards.push(card);
            if let Some(card_data) = self.card_database.get_card(card) {
                eprintln!("Removed card {} from live_card_zone due to constant ability restriction", card_data.card_no);
            }
        }
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
                AbilityTrigger::Constant => {
                    // Constant abilities are always active, but need to be evaluated continuously
                    ability.triggers.as_ref().map_or(false, |t| t == "常時")
                }
                AbilityTrigger::Auto => {
                    // Generic auto abilities (not tied to specific timing)
                    ability.triggers.as_ref().map_or(false, |t| t == "自動")
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
        });
    }

    /// Get temporary effects in proper layering order (Rule 9.9.1.7)
    /// Effects are applied in the order they were generated
    pub fn get_temporary_effects_in_order(&self) -> Vec<&TemporaryEffect> {
        let mut effects = self.temporary_effects.iter().collect::<Vec<_>>();
        effects.sort_by_key(|e| e.creation_order);
        effects
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

    // ============== REPLACEMENT EFFECT MANAGEMENT (Rule 9.10) ==============

    /// Add a replacement effect (Rule 9.10)
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

    /// Remove all replacement effects for a specific card
    pub fn remove_replacement_effects_for_card(&mut self, card_id: i16) {
        self.replacement_effects.retain(|e| e.card_id != card_id);
    }

    /// Get replacement effects for a specific event
    pub fn get_replacement_effects_for_event(&self, event: &str) -> Vec<&ReplacementEffect> {
        self.replacement_effects
            .iter()
            .filter(|e| e.original_event == event && !e.applied_this_event)
            .collect()
    }

    /// Reset the applied_this_event flags for all replacement effects (call before new event)
    pub fn reset_replacement_effect_flags(&mut self) {
        for effect in &mut self.replacement_effects {
            effect.applied_this_event = false;
        }
    }

    /// Mark a replacement effect as applied for the current event
    pub fn mark_replacement_effect_applied(&mut self, card_id: i16) {
        if let Some(effect) = self.replacement_effects.iter_mut().find(|e| e.card_id == card_id) {
            effect.applied_this_event = true;
        }
    }

    /// Mark that a formation change occurred this turn (for keyword validation)
    pub fn set_formation_change_occurred(&mut self) {
        self.formation_change_occurred_this_turn = true;
    }

    /// Reset position/formation change flags at start of new turn
    pub fn reset_change_flags(&mut self) {
        self.position_change_occurred_this_turn = false;
        self.formation_change_occurred_this_turn = false;
    }

    // ============== PERMANENT LOOP DETECTION (Rule 12.1) ==============

    /// Check for permanent loop (Rule 12.1)
    /// Rule 12.1.1: If a permanent loop is detected, active player declares the loop action and count
    /// Rule 12.1.1.2: If the same game state occurs twice in a turn, it's a loop
    /// Rule 12.1.1.3: If neither player can stop the loop, game ends in draw
    pub fn check_permanent_loop(&mut self) -> bool {
        // Generate a hash of the current game state
        let state_hash = self.generate_state_hash();

        // Check if this state has been seen before
        if self.game_state_history.contains(&state_hash) {
            self.loop_detected = true;
            return true;
        }

        // Add current state to history
        self.game_state_history.push(state_hash);

        // Limit history size
        if self.game_state_history.len() > self.max_state_history_size {
            self.game_state_history.remove(0);
        }

        false
    }

    /// Generate a hash of the current game state for loop detection
    fn generate_state_hash(&self) -> String {
        // Simplified state hash - include key game state elements
        format!(
            "t{}_p1h{}_p1e{}_p1w{}_p2h{}_p2e{}_p2w{}_p1s{:?}_p2s{:?}",
            self.turn_number,
            self.player1.hand.cards.len(),
            self.player1.energy_zone.cards.len(),
            self.player1.waitroom.cards.len(),
            self.player2.hand.cards.len(),
            self.player2.energy_zone.cards.len(),
            self.player2.waitroom.cards.len(),
            self.player1.stage.stage,
            self.player2.stage.stage
        )
    }

    /// Reset loop detection at start of new turn
    pub fn reset_loop_detection(&mut self) {
        self.game_state_history.clear();
        self.loop_detected = false;
    }

    /// Check if a permanent loop has been detected
    pub fn is_loop_detected(&self) -> bool {
        self.loop_detected
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
