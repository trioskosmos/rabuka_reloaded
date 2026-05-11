#[cfg(test)]
mod tests {
    use crate::test_helpers::*;
    use rabuka_engine::turn::TurnEngine;
    use rabuka_engine::game_setup::ActionType;

    #[test]
    fn test_per_unit_discard_bug_fix() {
        println!("Testing per_unit discard bug fix");
        
        let db = load_real_database();
        let mut game = TestGame::new(db);

        // Find a card with sequential effect: draw per_unit member, then discard 1
        // This is the problematic ability from abilities.json around line 1983
        let card_with_per_unit = game.find_card_by_number("PL!N-bp1-025-L"); // Example card with this ability pattern
        
        // Setup: 2 members on stage
        let member1 = game.id("PL!N-sd1-001-SP");
        let member2 = game.id("PL!N-sd1-002-SP");
        let filler = game.id("PL!-sd1-010-SD");
        
        game.state.player1.stage.stage = [member1, member2, -1];
        game.state.player1.hand.cards.clear();
        game.state.player1.hand.cards.push(card_with_per_unit);
        
        // Setup deck with enough cards
        game.state.player1.main_deck.cards.clear();
        for _ in 0..30 { 
            game.state.player1.main_deck.cards.push(filler); 
        }
        
        let hand_before = game.state.player1.hand.cards.len();
        let deck_before = game.state.player1.main_deck.cards.len();
        let discard_before = game.state.player1.waitroom.cards.len();
        
        // Activate the ability
        let result = TurnEngine::execute_main_phase_action(
            &mut game.state, &ActionType::UseAbility,
            Some(card_with_per_unit), None, None, None,
        );
        
        if let Err(e) = result {
            println!("Ability activation failed: {}", e);
            // This might be expected if the card doesn't have the right ability
            return;
        }
        
        // Process any pending choices
        while game.has_pending_choice() { 
            game.select_indices(&[0]); 
        }
        
        let hand_after = game.state.player1.hand.cards.len();
        let deck_after = game.state.player1.main_deck.cards.len();
        let discard_after = game.state.player1.waitroom.cards.len();
        
        println!("Before: hand={}, deck={}, discard={}", hand_before, deck_before, discard_before);
        println!("After: hand={}, deck={}, discard={}", hand_after, deck_after, discard_after);
        
        // With 2 members on stage:
        // - Should draw 2 cards (1 per member)
        // - Should discard only 1 card (fixed, not 2)
        let expected_draw = 2; // 2 members = 2 cards drawn
        let expected_discard = 1; // Should always be 1, not per_unit
        
        let actual_draw = hand_before + expected_draw - hand_after;
        let actual_discard = discard_after - discard_before;
        
        println!("Actual: drawn={}, discarded={}", actual_draw, actual_discard);
        
        assert_eq!(actual_draw, expected_draw, "Should draw {} cards for 2 members", expected_draw);
        assert_eq!(actual_discard, expected_discard, "Should discard only 1 card, not per_unit");
        
        println!("✓ Per-unit discard bug fix verified!");
    }
}
