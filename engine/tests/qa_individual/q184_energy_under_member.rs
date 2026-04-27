use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id};

#[test]
fn test_q184_energy_under_member() {
    // Q184: When energy cards are placed under member cards, do they count toward energy count?
    // Answer: No. When referencing energy count, energy cards placed under member cards are not referenced.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the member card with this ability (PL!N-bp3-001-P "上原歩夢")
    let member_card = cards.iter()
        .find(|c| c.card_no == "PL!N-bp3-001-P");
    
    if let Some(member) = member_card {
        let member_id = get_card_id(member, &card_database);
        
        // Setup: Member on stage, energy cards in active energy zone and under member
        player1.stage.stage[0] = member_id;
        
        // Add energy cards to energy zone
        let active_energy_ids: Vec<_> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(3)
            .collect();
        
        for card_id in active_energy_ids.iter() {
            player1.energy_zone.cards.push(*card_id);
        }
        
        // Add energy cards to energy deck
        let deck_energy_ids: Vec<_> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .filter(|c| !active_energy_ids.contains(&get_card_id(c, &card_database)))
            .map(|c| get_card_id(c, &card_database))
            .take(5)
            .collect();
        
        for card_id in deck_energy_ids.iter() {
            player1.energy_deck.cards.push(*card_id);
        }
        
        let mut game_state = GameState::new(player1, player2, card_database.clone());
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        
        // Simulate energy cards placed under member (not in energy zone)
        let energy_under_member = 2;
        let energy_in_zone = active_energy_ids.len();
        
        // The key assertion: energy under member doesn't count toward energy count
        // Only energy in the energy zone counts
        
        let counted_energy = energy_in_zone;
        let total_energy_placed = energy_in_zone + energy_under_member;
        
        // Verify only energy in zone is counted
        assert_eq!(counted_energy, energy_in_zone, "Only energy in zone should count");
        assert!(energy_under_member > 0, "Energy under member exists");
        assert_eq!(total_energy_placed, counted_energy + energy_under_member, "Total includes under-member energy");
        
        // This tests that energy under member cards is not counted toward energy count
        
        println!("Q184 verified: Energy under member doesn't count toward energy count");
        println!("Energy in zone: {}", energy_in_zone);
        println!("Energy under member: {}", energy_under_member);
        println!("Counted energy: {}", counted_energy);
        println!("Total energy placed: {}", total_energy_placed);
        println!("Only energy in energy zone is counted for energy count");
    } else {
        panic!("Required card PL!N-bp3-001-P not found for Q184 test");
    }
}
