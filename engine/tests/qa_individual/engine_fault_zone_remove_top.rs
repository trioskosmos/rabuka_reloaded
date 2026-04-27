// Q245: remove_top_card should not use unnecessary unwrap
// Fault: zones.rs line 323 uses unwrap() after checking is_empty, but the unwrap is unnecessary
// and could be simplified to use Option directly

use crate::qa_individual::common::*;
use rabuka_engine::game_state::{GameState, Phase};
use rabuka_engine::zones::LiveCardZone;

#[test]
fn test_q245_remove_top_card_unnecessary_unwrap() {
    // Test: remove_top_card should handle empty zones gracefully without unnecessary unwrap
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    
    // Test removing from empty zone
    let result = game_state.player1.live_card_zone.remove_top_card();
    assert!(result.is_none(), "Should return None for empty zone");
    
    // Add a card and remove it
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(1)
        .collect();
    
    if let Some(&card_id) = energy_card_ids.first() {
        game_state.player1.live_card_zone.cards.push(card_id);
        let result = game_state.player1.live_card_zone.remove_top_card();
        assert_eq!(result, Some(card_id), "Should return the card that was added");
    }
    
    // ENGINE FAULT: In zones.rs line 323, the code uses:
    // Some(self.cards.drain(..1).next().unwrap())
    // The unwrap() is unnecessary because drain(..1) will always return an iterator
    // with at least one element if the vector is not empty (which is already checked).
    // This can be simplified to:
    // self.cards.drain(..1).next()
    // which returns None if empty, Some(card) if not empty.
    
    // For now, this test documents the fault
    // A fix would be to remove the unnecessary unwrap
}
