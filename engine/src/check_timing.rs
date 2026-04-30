use crate::card::{Ability, AbilityEffect};
use crate::game_state::{GameState, Phase};
use crate::player::Player;

/// Check timing system for rule-based ability resolution (Rule 9.5)
pub struct CheckTimingSystem {
    pub pending_triggers: Vec<PendingTrigger>,
    pub active_triggers: Vec<ActiveTrigger>,
}

#[derive(Debug, Clone)]
pub struct PendingTrigger {
    pub ability: Ability,
    pub player_id: String,
    pub trigger_type: String,
    pub trigger_count: u32, // For multiple triggers of same ability
}

#[derive(Debug, Clone)]
pub struct ActiveTrigger {
    pub ability: Ability,
    pub player_id: String,
    pub execution_order: u32,
}

impl CheckTimingSystem {
    pub fn new() -> Self {
        Self {
            pending_triggers: Vec::new(),
            active_triggers: Vec::new(),
        }
    }

    /// Add a trigger to the pending queue (Rule 9.7.2)
    pub fn add_trigger(&mut self, ability: Ability, player_id: String, trigger_type: String) {
        // Use ability text as identifier since there's no id field
        let ability_key = ability.full_text.clone();
        
        // Check if this trigger type already exists for this ability
        if let Some(existing) = self.pending_triggers.iter_mut()
            .find(|t| t.ability.full_text == ability_key && t.player_id == player_id) {
            existing.trigger_count += 1;
        } else {
            self.pending_triggers.push(PendingTrigger {
                ability,
                player_id,
                trigger_type,
                trigger_count: 1,
            });
        }
    }

    /// Process check timing - resolve all rule processes first, then abilities (Rule 9.5.1)
    pub fn process_check_timing(
        &mut self,
        game_state: &mut GameState,
        active_player_id: &str,
    ) -> Result<Vec<String>, String> {
        let mut resolved = Vec::new();

        // Rule 9.5.3.1: Execute all rule processes first
        // Process any pending rule-based actions (e.g., turn-based effects, timeout checks)
        // These are non-ability game actions that must resolve before player triggers
        // TODO: Implement process_rule_processes
        // self.process_rule_processes(game_state, active_player_id, &mut resolved);

        // Rule 9.5.3.2: Active player resolves one of their triggers
        if let Some(trigger_index) = self.find_active_player_trigger(active_player_id) {
            let trigger = self.pending_triggers.remove(trigger_index);
            
            // Add to active triggers for execution
            self.active_triggers.push(ActiveTrigger {
                ability: trigger.ability.clone(),
                player_id: trigger.player_id.clone(),
                execution_order: self.active_triggers.len() as u32,
            });

            resolved.push(format!("Active player trigger: {} for {}", 
                trigger.trigger_type, trigger.player_id));
        }

        // Rule 9.5.3.3: Non-active player resolves one of their triggers
        let non_active_player_id = if active_player_id == "player1" { "player2" } else { "player1" };
        if let Some(trigger_index) = self.find_active_player_trigger(non_active_player_id) {
            let trigger = self.pending_triggers.remove(trigger_index);
            
            self.active_triggers.push(ActiveTrigger {
                ability: trigger.ability.clone(),
                player_id: trigger.player_id.clone(),
                execution_order: self.active_triggers.len() as u32,
            });

            resolved.push(format!("Non-active player trigger: {} for {}", 
                trigger.trigger_type, trigger.player_id));
        }

        // Rule 9.5.3.4: Continue until no more triggers
        if !self.pending_triggers.is_empty() {
            // Recursively process remaining triggers
            let more_resolved = self.process_check_timing(game_state, active_player_id)?;
            resolved.extend(more_resolved);
        }

        Ok(resolved)
    }

    /// Find trigger for active player (Rule 9.5.3.2)
    fn find_active_player_trigger(&self, player_id: &str) -> Option<usize> {
        self.pending_triggers.iter()
            .position(|t| t.player_id == player_id)
    }

    /// Check if any abilities trigger at this phase (Rule 9.7)
    pub fn check_phase_triggers(&mut self, game_state: &GameState, phase: &Phase) {
        // Need mutable access to add triggers
        // Check for phase-specific triggers
        match phase {
            Phase::Active => {
                // Check for "ターンの始めに" (turn start) triggers
                self.check_turn_start_triggers(game_state);
            }
            Phase::LiveStart => {
                // Check for "ライブ開始時" (live start) triggers
                self.check_live_start_triggers(game_state);
            }
            Phase::LiveSuccess => {
                // Check for "ライブ成功時" (live success) triggers
                self.check_live_success_triggers(game_state);
            }
            _ => {}
        }
    }

    fn check_turn_start_triggers(&mut self, game_state: &GameState) {
        // Check both players for turn start abilities
        for player in [&game_state.player1, &game_state.player2] {
            // Check stage members
            for &card_id in &player.stage.stage {
                if card_id != crate::constants::EMPTY_SLOT {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        for (_idx, ability) in card.abilities.iter().enumerate() {
                            if let Some(ref triggers) = ability.triggers {
                                if triggers.contains("ターンの始めに") || triggers.contains("turn_start") {
                                    self.add_pending_trigger(PendingTrigger {
                                        ability: ability.clone(),
                                        player_id: player.id.clone(),
                                        trigger_type: "turn_start".to_string(),
                                        trigger_count: 1,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn check_live_start_triggers(&mut self, game_state: &GameState) {
        // Check both players for live start abilities
        for player in [&game_state.player1, &game_state.player2] {
            // Check stage members
            for &card_id in &player.stage.stage {
                if card_id != crate::constants::EMPTY_SLOT {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        for (_idx, ability) in card.abilities.iter().enumerate() {
                            if let Some(ref triggers) = ability.triggers {
                                if triggers.contains("ライブ開始時") || triggers.contains("live_start") {
                                    self.add_pending_trigger(PendingTrigger {
                                        ability: ability.clone(),
                                        player_id: player.id.clone(),
                                        trigger_type: "live_start".to_string(),
                                        trigger_count: 1,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn check_live_success_triggers(&mut self, game_state: &GameState) {
        // Check both players for live success abilities
        for player in [&game_state.player1, &game_state.player2] {
            // Check stage members
            for &card_id in &player.stage.stage {
                if card_id != crate::constants::EMPTY_SLOT {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        for (_idx, ability) in card.abilities.iter().enumerate() {
                            if let Some(ref triggers) = ability.triggers {
                                if triggers.contains("ライブ成功時") || triggers.contains("live_success") {
                                    self.add_pending_trigger(PendingTrigger {
                                        ability: ability.clone(),
                                        player_id: player.id.clone(),
                                        trigger_type: "live_success".to_string(),
                                        trigger_count: 1,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn add_pending_trigger(&mut self, trigger: PendingTrigger) {
        // Add trigger to the pending queue for processing
        self.pending_triggers.push(trigger);
    }

    /// Clear all triggers (for end of timing)
    pub fn clear_triggers(&mut self) {
        self.pending_triggers.clear();
        self.active_triggers.clear();
    }
}
