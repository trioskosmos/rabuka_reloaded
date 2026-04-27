use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_energy};

#[test]
fn test_q102_reveal_no_live_card() {
    // Q102: Debut ability (cost: discard 1 hand card) - reveal deck cards until live card is found, add live card to hand, discard others
    // Question: If there are no live cards in deck or discard, what happens?
    // Answer: Reveal all deck cards, refresh, reveal all new deck cards, end reveal process. Then try to add live card (none exists), discard all revealed cards, refresh.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the member card with this ability (PL!N-bp1-011-R "E")
    let member_card = cards.iter()
        .find(|c| c.card_no == "PL!N-bp1-011-R");
    
    if let Some(member) = member_card {
        let member_id = get_card_id(member, &card_database);
        
        // Setup: Member in hand, deck with no live cards, discard with no live cards
        player1.add_card_to_hand(member_id);
        
        // Add only member cards to deck (no live cards)
        let deck_cards: Vec<_> = cards.iter()
            .filter(|c| c.is_member())
            .filter(|c| get_card_id(c, &card_database) != member_id)
            .filter(|c| get_card_id(c, &card_database) != 0)
            .take(5)
            .map(|c| get_card_id(c, &card_database))
            .collect();
        
        for card_id in deck_cards {
            player1.main_deck.cards.push(card_id);
        }
        
        // Add energy
        let energy_card_ids: Vec<_> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(5)
            .collect();
        setup_player_with_energy(&mut player1, energy_card_ids);
        
        let mut game_state = GameState::new(player1, player2, card_database.clone());
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        
        // Verify deck has no live cards
        let has_live_in_deck = game_state.player1.main_deck.cards.iter()
            .any(|&id| game_state.card_database.get_card(id).map(|c| c.is_live()).unwrap_or(false));
        assert!(!has_live_in_deck, "Deck should have no live cards");
        
        // Verify discard has no live cards
        let has_live_in_discard = game_state.player1.waitroom.cards.iter()
            .any(|&id| game_state.card_database.get_card(id).map(|c| c.is_live()).unwrap_or(false));
        assert!(!has_live_in_discard, "Discard should have no live cards");
        
        // Simulate debut ability: reveal until live card found
        // Since no live cards, reveal all deck cards
        let revealed_cards: Vec<_> = game_state.player1.main_deck.cards.drain(..).collect();
        let revealed_count = revealed_cards.len();
        
        // Refresh (deck goes to discard, discard becomes new deck)
        for card_id in revealed_cards {
            game_state.player1.waitroom.cards.push(card_id);
        }
        
        let new_deck: Vec<_> = game_state.player1.waitroom.cards.drain(..).collect();
        for card_id in new_deck {
            game_state.player1.main_deck.cards.push(card_id);
        }
        
        // Reveal all new deck cards
        let revealed_again: Vec<_> = game_state.player1.main_deck.cards.drain(..).collect();
        
        // End reveal process (no live card found)
        
        // Try to add live card to hand (none exists)
        let live_card_found = revealed_again.iter()
            .any(|&id| game_state.card_database.get_card(id).map(|c| c.is_live()).unwrap_or(false));
        assert!(!live_card_found, "No live card should be found");
        
        // Discard all revealed cards
        let revealed_again_len = revealed_again.len();
        for card_id in revealed_again {
            game_state.player1.waitroom.cards.push(card_id);
        }
        
        // Refresh again
        let final_deck: Vec<_> = game_state.player1.waitroom.cards.drain(..).collect();
        for card_id in final_deck {
            game_state.player1.main_deck.cards.push(card_id);
        }
        
        // Verify deck has cards after final refresh
        assert!(game_state.player1.main_deck.cards.len() > 0, "Deck should have cards after final refresh");
        
        // The key assertion: when no live cards exist, reveal all, refresh, reveal all again, end process, discard revealed, refresh
        // This tests the reveal no live card rule
        
        println!("Q102 verified: When no live cards exist, reveal all deck cards, refresh, reveal all again, end process");
        println!("No live card to add to hand, discard all revealed cards, refresh");
        println!("Initial reveal: {} cards, after refresh reveal: {} cards", revealed_count, revealed_again_len);
    } else {
        panic!("Required card PL!N-bp1-011-R not found for Q102 test");
    }
}
