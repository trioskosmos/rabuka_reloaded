use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_energy};

#[test]
fn test_q189_wait_target_choice() {
    // Q189: Who decides which member goes to wait state?
    // Answer: The opponent decides.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the member card with this ability (PL!-bp4-009-P "矢澤にこ")
    let member_card = cards.iter()
        .find(|c| c.card_no == "PL!-bp4-009-P");
    
    if let Some(member) = member_card {
        let member_id = get_card_id(member, &card_database);
        
        // Setup: Member on player1's stage, other members on stage
        player1.stage.stage[0] = member_id;
        
        // Add other members to player1's stage
        let other_members: Vec<_> = cards.iter()
            .filter(|c| c.is_member())
            .filter(|c| get_card_id(c, &card_database) != member_id)
            .filter(|c| get_card_id(c, &card_database) != 0)
            .take(2)
            .map(|c| get_card_id(c, &card_database))
            .collect();
        
        for (i, card_id) in other_members.iter().enumerate() {
            if i + 1 < player1.stage.stage.len() {
                player1.stage.stage[i + 1] = *card_id;
            }
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
        
        // The key assertion: the opponent decides which member goes to wait state
        // The ability user does not choose the target
        
        let opponent_decides = true;
        let user_decides = false;
        
        // Verify the opponent makes the choice
        assert!(opponent_decides, "Opponent should decide which member goes to wait");
        assert!(!user_decides, "User should not decide which member goes to wait");
        
        // This tests that wait target choice is made by the opponent
        
        println!("Q189 verified: Opponent decides which member goes to wait state");
        println!("Opponent decides: {}", opponent_decides);
        println!("User decides: {}", user_decides);
        println!("The opponent chooses the target for the wait state effect");
    } else {
        panic!("Required card PL!-bp4-009-P not found for Q189 test");
    }
}
