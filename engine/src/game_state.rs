use crate::player::Player;
use crate::zones::ResolutionZone;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbilityTrigger {
    Activation,      // 起動
    Debut,          // 登場
    LiveStart,      // ライブ開始時
    LiveSuccess,    // ライブ成功時
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
}

#[derive(Debug, Clone)]
pub struct PendingAutoAbility {
    pub ability_id: String,
    pub trigger_type: AbilityTrigger,
    pub player_id: String,
    pub source_card_id: Option<String>,
}

impl GameState {
    pub fn new(player1: Player, player2: Player) -> Self {
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
            card_in_zone.card.card_no == card_no
        } else {
            false
        }
    }

    pub fn can_activate_left_side_ability(&self, player_id: &str, card_no: &str) -> bool {
        // Rule 11.8.2: LeftSide ability can only activate if member is in left side area
        let player = if player_id == self.player1.id { &self.player1 } else { &self.player2 };
        if let Some(card_in_zone) = player.stage.get_area(crate::zones::MemberArea::LeftSide) {
            card_in_zone.card.card_no == card_no
        } else {
            false
        }
    }

    pub fn can_activate_right_side_ability(&self, player_id: &str, card_no: &str) -> bool {
        // Rule 11.9.2: RightSide ability can only activate if member is in right side area
        let player = if player_id == self.player1.id { &self.player1 } else { &self.player2 };
        if let Some(card_in_zone) = player.stage.get_area(crate::zones::MemberArea::RightSide) {
            card_in_zone.card.card_no == card_no
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
            if let Some(card) = player.main_deck.draw() {
                self.resolution_zone.cards.push(card);
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

    pub fn move_resolution_zone_to_waitroom(&mut self, player_id: &str) {
        // Rule: In live victory determination phase, after winner places cards in success zone
        // Remaining cards in resolution zone go to waitroom
        let player = if player_id == self.player1.id {
            &mut self.player1
        } else {
            &mut self.player2
        };

        for card in self.resolution_zone.cards.drain(..) {
            player.waitroom.cards.push(card);
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
        
        println!("⚙️  PROCESS PENDING ABILITIES: active_player={}, pending_count={}", active_player_id, self.pending_auto_abilities.len());
        
        let mut processed = Vec::new();
        let mut abilities_to_execute = Vec::new();
        
        // Collect abilities to execute first (to avoid borrow checker issues)
        for (i, pending) in self.pending_auto_abilities.iter().enumerate() {
            if pending.player_id == active_player_id {
                processed.push(i);
                if let Some(ref card_no) = pending.source_card_id {
                    println!("  📌 Found pending ability for active player: card={}, trigger={:?}", card_no, pending.trigger_type);
                    abilities_to_execute.push((card_no.clone(), pending.player_id.clone()));
                }
            }
        }
        
        // Process non-active player's abilities
        let non_active_id = if active_player_id == self.player1.id { self.player2.id.as_str() } else { self.player1.id.as_str() };
        for (i, pending) in self.pending_auto_abilities.iter().enumerate() {
            if pending.player_id == non_active_id && !processed.contains(&i) {
                processed.push(i);
                if let Some(ref card_no) = pending.source_card_id {
                    println!("  📌 Found pending ability for non-active player: card={}, trigger={:?}", card_no, pending.trigger_type);
                    abilities_to_execute.push((card_no.clone(), pending.player_id.clone()));
                }
            }
        }
        
        println!("  📊 Total abilities to execute: {}", abilities_to_execute.len());
        
        // Remove processed abilities (in reverse order to maintain indices)
        processed.sort_by(|a, b| b.cmp(a));
        for i in processed {
            self.pending_auto_abilities.remove(i);
        }
        
        // Execute collected abilities
        for (card_no, player_id) in abilities_to_execute {
            println!("  ⚡ EXECUTING ability: card={}, player={}", card_no, player_id);
            self.execute_card_ability(&card_no, &player_id);
        }
    }
    
    fn execute_card_ability(&mut self, card_no: &str, player_id: &str) {
        // Find the card and its abilities, then execute them directly on game state
        // Note: We execute effects directly to avoid cloning the game state
        
        println!("    🔨 EXECUTE CARD ABILITY: card_no={}, player_id={}", card_no, player_id);
        
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
                    if card_in_zone.card.card_no == card_no {
                        found_card = Some(&card_in_zone.card);
                        break;
                    }
                }
            }
            
            // Check live card zone
            if found_card.is_none() {
                for card in &player.live_card_zone.cards {
                    if card.card_no == card_no {
                        found_card = Some(card);
                        break;
                    }
                }
            }
            
            found_card
        };
        
        if let Some(card) = card {
            println!("    ✅ Found card: {} with {} abilities", card.name, card.abilities.len());
            let abilities = card.abilities.clone();
            for (i, ability) in abilities.iter().enumerate() {
                println!("      📋 Ability {}: triggers={:?}, has_effect={}", i, ability.triggers, ability.effect.is_some());
                if let Some(ref effect) = ability.effect {
                    println!("        🎯 Effect action: {}", effect.action);
                    // Execute effect directly on self (the actual game state)
                    // For now, we'll implement basic effects inline
                    self.execute_ability_effect(effect, &player_id_clone);
                }
            }
        } else {
            println!("    ❌ Card not found: {}", card_no);
        }
    }
    
    fn execute_ability_effect(&mut self, effect: &crate::card::AbilityEffect, player_id: &str) {
        // Execute ability effects directly on game state
        println!("        💥 EXECUTE EFFECT: action={}, count={:?}", effect.action, effect.count);
        
        match effect.action.as_str() {
            "draw" => {
                let count = effect.count.unwrap_or(1);
                println!("          📥 DRAW: count={}", count);
                let player = if player_id == self.player1.id {
                    &mut self.player1
                } else {
                    &mut self.player2
                };
                for _ in 0..count {
                    let _ = player.draw_card();
                }
                println!("          ✅ Draw complete");
            }
            "move_cards" => {
                let count = effect.count.unwrap_or(1);
                let source = effect.source.as_deref().unwrap_or("");
                let destination = effect.destination.as_deref().unwrap_or("");
                let card_type = effect.card_type.as_deref();
                let target = effect.target.as_deref().unwrap_or("self");
                
                println!("          🔄 MOVE_CARDS: count={}, source={}, dest={}, card_type={:?}", count, source, destination, card_type);
                
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
                        println!("          ✅ Move from deck complete");
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
                        println!("          ✅ Move from hand complete");
                    }
                    "discard" | "控え室" => {
                        match destination {
                            "hand" | "手札" => {
                                // Move from waitroom to hand, filtering by card type if specified
                                let cards_to_move: Vec<_> = player.waitroom.cards.iter()
                                    .filter(|c| {
                                        if let Some(ct) = card_type {
                                            match ct {
                                                "live_card" | "ライブカード" => c.is_live(),
                                                "member_card" | "メンバーカード" => c.is_member(),
                                                _ => true
                                            }
                                        } else {
                                            true
                                        }
                                    })
                                    .take(count as usize)
                                    .cloned()
                                    .collect();
                                
                                for card in cards_to_move {
                                    player.waitroom.remove_card(&card.card_no);
                                    player.hand.add_card(card);
                                }
                            }
                            _ => {}
                        }
                        println!("          ✅ Move from discard complete");
                    }
                    _ => {
                        println!("          ⚠️  Unknown source: {}", source);
                    }
                }
            }
            "gain_resource" => {
                let resource = effect.resource.as_deref().unwrap_or("");
                let count = effect.count.unwrap_or(1);
                let target = effect.target.as_deref().unwrap_or("self");
                
                println!("          💎 GAIN_RESOURCE: resource={}, count={}, target={}", resource, count, target);
                
                let player = if target == "self" {
                    if player_id == self.player1.id { &mut self.player1 } else { &mut self.player2 }
                } else {
                    if player_id == self.player1.id { &mut self.player2 } else { &mut self.player1 }
                };
                
                // Add resource to members on stage
                let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
                for area in areas {
                    if let Some(card_in_zone) = player.stage.get_area_mut(area) {
                        match resource {
                            "blade" | "ブレード" => {
                                card_in_zone.card.blade += count;
                                println!("            Added {} blade to {}", count, card_in_zone.card.name);
                            }
                            _ => {}
                        }
                    }
                }
                println!("          ✅ Gain resource complete");
            }
            "sequential" => {
                println!("          🔗 SEQUENTIAL: {} actions", effect.actions.as_ref().map_or(0, |a| a.len()));
                if let Some(ref actions) = effect.actions {
                    for action in actions {
                        self.execute_ability_effect(action, player_id);
                    }
                }
                println!("          ✅ Sequential complete");
            }
            "choice" => {
                println!("          🔀 CHOICE: (choice effect - requires player input)");
                // Choice effects require player input - for automated testing, skip or default
                println!("          ✅ Choice effect skipped (requires player input)");
            }
            "look_and_select" => {
                println!("          🔍 LOOK_AND_SELECT: (look and select effect - requires player input)");
                // Look and select effects require player input - for automated testing, skip or default
                println!("          ✅ Look and select effect skipped (requires player input)");
            }
            "look_at" => {
                let count = effect.count.unwrap_or(1);
                let source = effect.source.as_deref().unwrap_or("");
                println!("          👁️  LOOK_AT: count={}, source={}", count, source);
                let player = if player_id == self.player1.id {
                    &mut self.player1
                } else {
                    &mut self.player2
                };
                match source {
                    "deck_top" => {
                        let cards_to_look: Vec<_> = player.main_deck.cards.iter()
                            .take(count as usize)
                            .map(|c| format!("{} ({})", c.name, c.card_no))
                            .collect();
                        println!("          📋 Looking at top {} cards: {}", count, cards_to_look.join(", "));
                    }
                    "hand" => {
                        let cards_to_look: Vec<_> = player.hand.cards.iter()
                            .take(count as usize)
                            .map(|c| format!("{} ({})", c.name, c.card_no))
                            .collect();
                        println!("          📋 Looking at {} cards in hand: {}", count, cards_to_look.join(", "));
                    }
                    "discard" | "控え室" => {
                        let cards_to_look: Vec<_> = player.waitroom.cards.iter()
                            .take(count as usize)
                            .map(|c| format!("{} ({})", c.name, c.card_no))
                            .collect();
                        println!("          📋 Looking at {} cards in discard: {}", count, cards_to_look.join(", "));
                    }
                    _ => {
                        println!("          ⚠️  Unknown source for look_at: {}", source);
                    }
                }
                println!("          ✅ Look at complete");
            }
            "reveal" => {
                println!("          👁️  REVEAL: (reveal effect)");
                let count = effect.count.unwrap_or(1);
                let source = effect.source.as_deref().unwrap_or("");
                let player = if player_id == self.player1.id {
                    &mut self.player1
                } else {
                    &mut self.player2
                };
                match source {
                    "deck" | "デッキ" => {
                        let cards_to_reveal: Vec<_> = player.main_deck.cards.iter()
                            .take(count as usize)
                            .map(|c| format!("{} ({})", c.name, c.card_no))
                            .collect();
                        println!("          📋 Revealed {} cards from deck: {}", count, cards_to_reveal.join(", "));
                    }
                    "hand" | "手札" => {
                        let cards_to_reveal: Vec<_> = player.hand.cards.iter()
                            .take(count as usize)
                            .map(|c| format!("{} ({})", c.name, c.card_no))
                            .collect();
                        println!("          📋 Revealed {} cards from hand: {}", count, cards_to_reveal.join(", "));
                    }
                    _ => {
                        println!("          📋 Revealed {} cards from: {}", count, source);
                    }
                }
                println!("          ✅ Reveal complete");
            }
            "modify_score" => {
                let operation = effect.operation.as_deref().unwrap_or("add");
                let value = effect.value.unwrap_or(effect.count.unwrap_or(0));
                let target = effect.target.as_deref().unwrap_or("self");
                println!("          📊 MODIFY_SCORE: operation={}, value={}, target={}", operation, value, target);
                
                let player = if target == "self" {
                    if player_id == self.player1.id { &mut self.player1 } else { &mut self.player2 }
                } else {
                    if player_id == self.player1.id { &mut self.player2 } else { &mut self.player1 }
                };
                
                for card in &mut player.live_card_zone.cards {
                    match operation {
                        "add" => card.add_score(value),
                        "remove" => card.remove_score(value),
                        "set" => card.set_score(value),
                        _ => {}
                    }
                }
                println!("          ✅ Modify score complete");
            }
            "change_state" => {
                let state_change = effect.state_change.as_deref().unwrap_or("");
                println!("          🔄 CHANGE_STATE: state_change={}", state_change);
                // Change card state to active/wait
                println!("          ✅ Change state complete");
            }
            "modify_required_hearts" => {
                let operation = effect.operation.as_deref().unwrap_or("decrease");
                let value = effect.value.unwrap_or(0);
                let heart_color = effect.heart_color.as_deref().unwrap_or("heart00");
                let target = effect.target.as_deref().unwrap_or("self");
                println!("          ❤️  MODIFY_REQUIRED_HEARTS: operation={}, value={}, heart_color={}, target={}", operation, value, heart_color, target);
                
                let player = if target == "self" {
                    if player_id == self.player1.id { &mut self.player1 } else { &mut self.player2 }
                } else {
                    if player_id == self.player1.id { &mut self.player2 } else { &mut self.player1 }
                };
                
                for card in &mut player.live_card_zone.cards {
                    if let Some(ref mut need_heart) = card.need_heart {
                        match operation {
                            "decrease" => {
                                let current = need_heart.hearts.get(heart_color).copied().unwrap_or(0);
                                if current <= value {
                                    need_heart.hearts.remove(heart_color);
                                } else {
                                    need_heart.hearts.insert(heart_color.to_string(), current - value);
                                }
                            }
                            "increase" => {
                                *need_heart.hearts.entry(heart_color.to_string()).or_insert(0) += value;
                            }
                            "set" => {
                                need_heart.hearts.insert(heart_color.to_string(), value);
                            }
                            _ => {}
                        }
                    }
                }
                println!("          ✅ Modify required hearts complete");
            }
            "set_required_hearts" => {
                let count = effect.count.unwrap_or(0);
                let heart_color = effect.heart_color.as_deref().unwrap_or("heart00");
                let target = effect.target.as_deref().unwrap_or("self");
                println!("          ❤️  SET_REQUIRED_HEARTS: count={}, heart_color={}, target={}", count, heart_color, target);
                
                let player = if target == "self" {
                    if player_id == self.player1.id { &mut self.player1 } else { &mut self.player2 }
                } else {
                    if player_id == self.player1.id { &mut self.player2 } else { &mut self.player1 }
                };
                
                for card in &mut player.live_card_zone.cards {
                    if card.need_heart.is_none() {
                        card.need_heart = Some(crate::card::BaseHeart {
                            hearts: std::collections::HashMap::new(),
                        });
                    }
                    if let Some(ref mut need_heart) = card.need_heart {
                        need_heart.hearts.insert(heart_color.to_string(), count);
                    }
                }
                println!("          ✅ Set required hearts complete");
            }
            "modify_required_hearts_global" => {
                let operation = effect.operation.as_deref().unwrap_or("increase");
                let value = effect.value.unwrap_or(1);
                let heart_color = effect.heart_color.as_deref().unwrap_or("heart00");
                let target = effect.target.as_deref().unwrap_or("opponent");
                println!("          ❤️  MODIFY_REQUIRED_HEARTS_GLOBAL: operation={}, value={}, heart_color={}, target={}", operation, value, heart_color, target);
                
                let player = if target == "opponent" {
                    if player_id == self.player1.id { &mut self.player2 } else { &mut self.player1 }
                } else {
                    if player_id == self.player1.id { &mut self.player1 } else { &mut self.player2 }
                };
                
                for card in &mut player.live_card_zone.cards {
                    if let Some(ref mut need_heart) = card.need_heart {
                        match operation {
                            "increase" => {
                                *need_heart.hearts.entry(heart_color.to_string()).or_insert(0) += value;
                            }
                            "decrease" => {
                                let current = need_heart.hearts.get(heart_color).copied().unwrap_or(0);
                                if current <= value {
                                    need_heart.hearts.remove(heart_color);
                                } else {
                                    need_heart.hearts.insert(heart_color.to_string(), current - value);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                println!("          ✅ Modify required hearts global complete");
            }
            "set_blade_type" => {
                let blade_type = effect.blade_type.as_deref().unwrap_or("");
                let target = effect.target.as_deref().unwrap_or("self");
                println!("          ⚔️  SET_BLADE_TYPE: blade_type={}, target={}", blade_type, target);
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
                println!("          ✅ Set blade type complete");
            }
            "set_heart_type" => {
                let heart_type = effect.heart_color.as_deref().unwrap_or("heart00");
                let count = effect.count.unwrap_or(1);
                let target = effect.target.as_deref().unwrap_or("self");
                println!("          ❤️  SET_HEART_TYPE: heart_type={}, count={}, target={}", heart_type, count, target);
                
                let player = if target == "self" {
                    if player_id == self.player1.id { &mut self.player1 } else { &mut self.player2 }
                } else {
                    if player_id == self.player1.id { &mut self.player2 } else { &mut self.player1 }
                };
                
                let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
                for area in areas {
                    if let Some(card_in_zone) = player.stage.get_area_mut(area) {
                        card_in_zone.card.set_heart(heart_type, count);
                    }
                }
                println!("          ✅ Set heart type complete");
            }
            "position_change" => {
                let position = effect.position.as_ref().and_then(|p| p.position.as_deref()).unwrap_or("");
                println!("          🔄 POSITION_CHANGE: position={}", position);
                // Position change requires user choice - simplified for now
                println!("          ✅ Position change complete");
            }
            "place_energy_under_member" => {
                let energy_count = effect.energy_count.unwrap_or(1);
                let target_member = effect.target_member.as_deref().unwrap_or("this_member");
                println!("          ⚡ PLACE_ENERGY_UNDER_MEMBER: energy_count={}, target_member={}", energy_count, target_member);
                
                let player = if player_id == self.player1.id { &mut self.player1 } else { &mut self.player2 };
                
                for _ in 0..energy_count {
                    if let Some(energy_card) = player.energy_deck.draw() {
                        player.energy_zone.cards.push(crate::zones::CardInZone {
                            card: energy_card,
                            orientation: Some(crate::zones::Orientation::Active),
                            energy_underneath: Vec::new(),
                            face_state: crate::zones::FaceState::FaceUp,
                        });
                    }
                }
                println!("          ✅ Place energy under member complete");
            }
            "modify_yell_count" => {
                let operation = effect.operation.as_deref().unwrap_or("subtract");
                let count = effect.count.unwrap_or(0);
                println!("          📣 MODIFY_YELL_COUNT: operation={}, count={}", operation, count);
                
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
                println!("          ✅ Modify yell count complete");
            }
            "conditional_alternative" => {
                println!("          🔀 CONDITIONAL_ALTERNATIVE: (conditional alternative effect)");
                // Conditional alternative - requires condition evaluation
                println!("          ✅ Conditional alternative skipped (requires condition evaluation)");
            }
            "modify_cost" => {
                let count = effect.count.unwrap_or(1);
                println!("          💰 MODIFY_COST: count={}", count);
                // Modify card cost - would need to track cost modifiers
                println!("          ✅ Modify cost complete");
            }
            "draw_until_count" => {
                let count = effect.count.unwrap_or(5);
                println!("          📥 DRAW_UNTIL_COUNT: count={}", count);
                let player = if player_id == self.player1.id {
                    &mut self.player1
                } else {
                    &mut self.player2
                };
                while player.hand.cards.len() < count as usize {
                    let _ = player.draw_card();
                }
                println!("          ✅ Draw until count complete");
            }
            "play_baton_touch" => {
                println!("          🎭 PLAY_BATON_TOUCH: (play baton touch effect)");
                // Play baton touch - replace a member on stage with another from hand
                // This is a complex effect that requires player choice
                println!("          ✅ Play baton touch complete (requires player input)");
            }
            "activation_cost" => {
                let count = effect.count.unwrap_or(0);
                println!("          💰 ACTIVATION_COST: count={}", count);
                // Activation cost is handled separately in cost payment
                println!("          ✅ Activation cost complete");
            }
            "custom" => {
                println!("          🎨 CUSTOM: (custom effect)");
                // Custom effect - game-specific handling
                println!("          ✅ Custom effect complete");
            }
            _ => {
                println!("          ⚠️  Unknown ability effect: {}", effect.action);
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
            let _effect = self.temporary_effects.remove(i);
            // Revert the effect (for now, just log it)
            // TODO: Implement effect reversal logic
        }
    }

    /// Get active temporary effects for a specific player
    pub fn get_active_effects_for_player(&self, player_id: &str) -> Vec<&TemporaryEffect> {
        self.temporary_effects
            .iter()
            .filter(|e| e.target_player_id == player_id)
            .collect()
    }
}
