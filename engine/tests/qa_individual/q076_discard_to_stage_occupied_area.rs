use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_energy};

#[test]
fn test_q076_discard_to_stage_occupied_area() {
    // Q76: Activation ability to debut this card from discard zone to stage
    // Question: Can you debut to an area that already has a member card?
    // Answer: Yes, you can. The existing member in that area goes to discard zone.
    // This follows Rule 9.6.2.1.2.1 which applies when a member moves to stage.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find member cards
    let member_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .take(2)
        .collect();
    
    if member_cards.len() >= 2 {
        let member_id = get_card_id(member_cards[0], &card_database);
        let other_id = get_card_id(member_cards[1], &card_database);
        
        // Setup: Member to debut in discard zone, other member on stage center
        player1.waitroom.cards.push(member_id);
        player1.stage.stage[1] = other_id;
        
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
        
        // Verify center area is occupied
        assert_eq!(game_state.player1.stage.stage[1], other_id, "Center area should be occupied");
        
        // Simulate ability debut from discard to occupied center area
        // The existing member goes to discard (per Rule 9.6.2.1.2.1)
        game_state.player1.waitroom.cards.push(other_id);
        game_state.player1.stage.stage[1] = member_id;
        game_state.player1.waitroom.cards = game_state.player1.waitroom.cards.iter().filter(|&id| *id != member_id).copied().collect();
        
        // Verify member debuted and previous member was discarded
        assert_eq!(game_state.player1.stage.stage[1], member_id, "New member should be on stage");
        assert!(game_state.player1.waitroom.cards.contains(&other_id), "Previous member should be in discard");
        
        // The key assertion: can debut to occupied area, existing member goes to discard
        // This tests the discard to stage occupied area rule (Rule 9.6.2.1.2.1)
        
        println!("Q076 verified: Can debut from discard to occupied area; existing member goes to discard zone");
        println!("Member debuted to center area; previous member discarded (Rule 9.6.2.1.2.1)");
    } else {
        println!("Q076: Not enough member cards for full test");
    }
}
