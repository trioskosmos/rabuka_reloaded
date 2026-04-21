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
        
        // Simple random choice - no logic
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
