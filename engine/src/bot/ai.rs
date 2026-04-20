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
    pub fn choose_action(&self, actions: &[String]) -> usize {
        use rand::Rng;
        
        if actions.is_empty() {
            return 0;
        }
        
        // Improved strategy: prioritize playing member cards to build stage
        let mut rng = rand::thread_rng();
        
        // Look for "Play [card name] to [area] area" actions (member cards)
        // These are actions that start with "Play " and end with " area"
        let mut play_actions = Vec::new();
        for (i, action) in actions.iter().enumerate() {
            if action.starts_with("Play ") && action.ends_with(" area") {
                play_actions.push(i);
            }
        }
        
        // If we have play actions, choose one randomly
        if !play_actions.is_empty() {
            return play_actions[rng.gen_range(0..play_actions.len())];
        }
        
        // If no member cards available (stage full or hand empty), always pass
        // Don't swap endlessly - swapping doesn't progress the game
        for (i, action) in actions.iter().enumerate() {
            if action.contains("Pass") {
                return i;
            }
        }
        
        // Otherwise choose randomly
        rng.gen_range(0..actions.len())
    }

    /// Choose which live cards to set
    /// Returns the number of live cards to set (0-3)
    /// Only set actual live cards, not member cards
    pub fn choose_live_cards_to_set(player: &Player) -> usize {
        use rand::Rng;
        
        // Only count live cards - member cards will be discarded during performance
        let live_count = player.hand.cards.iter().filter(|c| c.is_live()).count();
        
        if live_count == 0 {
            return 0;
        }
        
        // Set up to 3 live cards
        let mut rng = rand::thread_rng();
        std::cmp::min(live_count, rng.gen_range(1..=3))
    }

    /// Choose which live card to move to success zone
    /// Returns the index of the card to move (0-based)
    /// Strategy: choose the card with highest score
    pub fn choose_live_card_for_success(player: &Player) -> usize {
        if player.live_card_zone.cards.is_empty() {
            return 0;
        }
        
        // Find the card with the highest score
        let mut best_index = 0;
        let mut best_score = player.live_card_zone.cards[0].score.unwrap_or(0);
        
        for (i, card) in player.live_card_zone.cards.iter().enumerate() {
            let score = card.score.unwrap_or(0);
            if score > best_score {
                best_score = score;
                best_index = i;
            }
        }
        
        best_index
    }
}
