// AI module for automated game play
// This module contains AI decision-making logic separate from the game engine
// The game engine provides legal actions, and the AI chooses from them
//
// DO NOT SIMPLIFY THE GAME FOR THE AI. FOLLOW RULES.TXT. MAKE SURE ALL GAME MECHANICS ARE PRESENT.

use crate::player::Player;

/// AI strategy for choosing actions
pub struct AIPlayer {
    #[allow(dead_code)]
    name: String,
}

impl AIPlayer {
    pub fn new(name: String) -> Self {
        AIPlayer { name }
    }

    /// Choose an action from available legal actions
    /// Returns the index of the chosen action
    pub fn choose_action(&self, actions: &[crate::game_setup::Action]) -> usize {
        use rand::Rng;
        
        if actions.is_empty() {
            return 0;
        }
        
        // Prefer SkipMulligan when available (AI strategy: skip mulligan)
        for (i, action) in actions.iter().enumerate() {
            if action.action_type == crate::game_setup::ActionType::SkipMulligan {
                return i;
            }
        }
        
        // Count action types to make phase-aware decisions
        let set_live_card_count = actions.iter()
            .filter(|a| a.action_type == crate::game_setup::ActionType::SetLiveCard)
            .count();
        let play_member_count = actions.iter()
            .filter(|a| a.action_type == crate::game_setup::ActionType::PlayMemberToStage)
            .count();
        let use_ability_count = actions.iter()
            .filter(|a| a.action_type == crate::game_setup::ActionType::UseAbility)
            .count();
        let has_pass = actions.iter()
            .any(|a| a.action_type == crate::game_setup::ActionType::Pass);
        
        // In LiveCardSet: Always pass if no SetLiveCard actions available
        // Otherwise, set 1-3 random cards then pass
        if has_pass {
            if set_live_card_count == 0 {
                // No cards to set, must pass
                for (i, action) in actions.iter().enumerate() {
                    if action.action_type == crate::game_setup::ActionType::Pass {
                        return i;
                    }
                }
            } else {
                // Have cards to set - 70% chance to set a card, 30% to pass
                let mut rng = rand::thread_rng();
                if rng.gen_bool(0.3) {
                    for (i, action) in actions.iter().enumerate() {
                        if action.action_type == crate::game_setup::ActionType::Pass {
                            return i;
                        }
                    }
                }
            }
        }
        
        // Prioritize SetLiveCard action when available (to set live cards during LiveCardSet phase)
        for (i, action) in actions.iter().enumerate() {
            if action.action_type == crate::game_setup::ActionType::SetLiveCard {
                return i;
            }
        }
        
        // In Main phase: Prioritize playing members and using abilities over passing
        // Only pass if no other actions available
        if play_member_count > 0 || use_ability_count > 0 {
            // Prefer PlayMemberToStage first
            for (i, action) in actions.iter().enumerate() {
                if action.action_type == crate::game_setup::ActionType::PlayMemberToStage {
                    return i;
                }
            }
            // Then UseAbility
            for (i, action) in actions.iter().enumerate() {
                if action.action_type == crate::game_setup::ActionType::UseAbility {
                    return i;
                }
            }
        }
        
        // Pass for other phases when available or when no other actions in Main phase
        for (i, action) in actions.iter().enumerate() {
            if action.action_type == crate::game_setup::ActionType::Pass {
                return i;
            }
        }
        
        // Simple random choice for other actions
        let mut rng = rand::thread_rng();
        rng.gen_range(0..actions.len())
    }

    /// Choose which live cards to set
    /// Returns the number of live cards to set (0-3)
    pub fn choose_live_cards_to_set(_player: &Player) -> usize {
        use rand::Rng;
        
        let mut rng = rand::thread_rng();
        rng.gen_range(0..=3)
    }

    /// Choose which live card to move to success zone
    /// Returns the index of the card to move (0-based)
    pub fn choose_live_card_for_success(player: &Player) -> usize {
        use rand::Rng;
        
        if player.live_card_zone.cards.is_empty() {
            return 0;
        }
        
        let mut rng = rand::thread_rng();
        rng.gen_range(0..player.live_card_zone.cards.len())
    }
}
