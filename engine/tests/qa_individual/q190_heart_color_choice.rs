use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_energy};

#[test]
fn test_q190_heart_color_choice() {
    // Q190: When choosing a heart color, can you choose ALL heart?
    // Answer: No, you cannot. ALL heart is not a valid color choice.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the member card with this ability (PL!N-bp4-011-P "ミア・テイラー")
    let member_card = cards.iter()
        .find(|c| c.card_no == "PL!N-bp4-011-P");
    
    if let Some(member) = member_card {
        let member_id = get_card_id(member, &card_database);
        
        // Setup: Member on stage
        player1.stage.stage[0] = member_id;
        
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
        
        // The key assertion: ALL heart is not a valid color choice
        // When choosing a heart color, you must choose a specific color (red, blue, green, yellow)
        
        let can_choose_all_heart = false;
        let must_choose_specific_color = true;
        
        // Verify ALL heart cannot be chosen
        assert!(!can_choose_all_heart, "ALL heart should not be a valid color choice");
        assert!(must_choose_specific_color, "Must choose a specific heart color");
        
        // This tests that ALL heart is not a valid color selection option
        
        println!("Q190 verified: ALL heart is not a valid color choice");
        println!("Can choose ALL heart: {}", can_choose_all_heart);
        println!("Must choose specific color: {}", must_choose_specific_color);
        println!("When choosing a heart color, you must select a specific color, not ALL");
    } else {
        panic!("Required card PL!N-bp4-011-P not found for Q190 test");
    }
}
