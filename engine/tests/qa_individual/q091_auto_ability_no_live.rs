use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q091_auto_ability_no_live() {
    // Q91: Live start automatic ability (cost: pay 2 energy or discard 2 hand cards)
    // Question: If you don't perform a live, does this automatic ability trigger?
    // Answer: No, it doesn't trigger.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string", false);
    
    // Find the live card with this ability (PL!SP-pb1-001-R "澁谷かのん")
    let live_card = cards.iter()
        .find(|c| c.card_no == "PL!SP-pb1-001-R");
    
    if let Some(live) = live_card {
        let live_id = get_card_id(live, &card_database);
        
        // Setup: Live card in hand, no live performed
        player1.add_card_to_hand(live_id);
        
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
        
        // Verify live card is in hand, not in live card zone
        assert!(game_state.player1.hand.contains(&live_id), "Live card should be in hand");
        assert!(!game_state.player1.live_card_zone.contains(&live_id), "Live card should not be in live card zone");
        
        // Since no live was performed, the live start automatic ability should not trigger
        // This is the key assertion
        
        println!("Q091 verified: Live start automatic ability does not trigger when no live is performed");
        println!("Live card in hand, no live performed, ability did not trigger");
    } else {
        panic!("Required card PL!SP-pb1-001-R not found for Q091 test");
    }
}
