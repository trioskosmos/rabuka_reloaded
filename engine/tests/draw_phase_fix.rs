#[cfg(test)]
mod tests {
    use helpers::*;
    use rabuka_engine::turn::TurnEngine;
    use rabuka_engine::game_setup::ActionType;

    #[test]
    fn test_draw_phase_no_unwanted_discards() {
        println!("Testing draw phase doesn't discard cards based on stage members");
        
        let db = load_database();
        let mut game = TestGame::new(db);

        // Setup: 2 members on stage
        let member1 = game.id("PL!N-sd1-001-SP");
        let member2 = game.id("PL!N-sd1-002-SP");
        let filler = game.id("PL!-sd1-010-SD");
        
        game.state.player1.stage.stage = [member1, member2, -1];
        game.state.player1.hand.cards.clear();
        
        // Setup deck with known cards
        game.state.player1.main_deck.cards.clear();
        for _ in 0..10 { 
            game.state.player1.main_deck.cards.push(filler); 
        }
        
        let hand_before = game.state.player1.hand.cards.len();
        let deck_before = game.state.player1.main_deck.cards.len();
        let discard_before = game.state.player1.waitroom.cards.len();
        
        // Execute draw phase
        TurnEngine::advance_phase(&mut game.state);
        
        let hand_after = game.state.player1.hand.cards.len();
        let deck_after = game.state.player1.main_deck.cards.len();
        let discard_after = game.state.player1.waitroom.cards.len();
        
        println!("Before: hand={}, deck={}, discard={}", hand_before, deck_before, discard_before);
        println!("After: hand={}, deck={}, discard={}", hand_after, deck_after, discard_after);
        
        // Should draw exactly 1 card, no discards
        assert_eq!(hand_after, hand_before + 1, "Should draw exactly 1 card");
        assert_eq!(deck_after, deck_before - 1, "Deck should lose exactly 1 card");
        assert_eq!(discard_after, discard_before, "Should not discard any cards during draw phase");
        
        println!("✓ Draw phase fix verified - no unwanted discards!");
    }
}
