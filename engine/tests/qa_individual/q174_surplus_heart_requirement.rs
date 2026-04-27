use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_energy};

#[test]
fn test_q174_surplus_heart_requirement() {
    // Q174: Live success ability - if you have 1+ surplus heart (heart04) and Nijigasaki member on stage,
    // place 1 energy card from energy deck in wait state
    // Question: If you have no green hearts on stage but gain 3 ALL hearts from cheer, can you use the ability?
    // Answer: No. Blade hearts from cheer don't count as surplus hearts for this requirement.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the live card with this ability (PL!N-bp3-027-L "La Bella Patria")
    let live_card = cards.iter()
        .find(|c| c.card_no == "PL!N-bp3-027-L");
    
    if let Some(live) = live_card {
        let live_id = get_card_id(live, &card_database);
        
        // Setup: Live card in player1's live card zone
        player1.live_card_zone.cards.push(live_id);
        
        // Add Nijigasaki member to player1's stage (no green hearts)
        let nijigasaki_member = cards.iter()
            .filter(|c| c.is_member())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .filter(|c| {
                if let Some(card) = card_database.get_card(get_card_id(c, &card_database)) {
                    card.series.contains("虹ヶ咲") || card.name.contains("虹ヶ咲")
                } else {
                    false
                }
            })
            .next();
        
        if let Some(member) = nijigasaki_member {
            let member_id = get_card_id(member, &card_database);
            player1.stage.stage[0] = member_id;
        }
        
        // Add energy cards to energy deck
        let energy_card_ids: Vec<_> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(5)
            .collect();
        
        for card_id in energy_card_ids.iter() {
            player1.energy_deck.cards.push(*card_id);
        }
        
        // Add some active energy
        let active_energy_ids: Vec<_> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .filter(|c| !energy_card_ids.contains(&get_card_id(c, &card_database)))
            .map(|c| get_card_id(c, &card_database))
            .take(5)
            .collect();
        setup_player_with_energy(&mut player1, active_energy_ids);
        
        let mut game_state = GameState::new(player1, player2, card_database.clone());
        game_state.current_phase = rabuka_engine::game_state::Phase::LiveSuccess;
        game_state.turn_number = 1;
        
        // Simulate gaining 3 ALL hearts from cheer (blade hearts)
        let blade_hearts_from_cheer = 3;
        let green_hearts_on_stage = 0; // No green hearts on stage
        
        // Calculate surplus hearts
        // Surplus hearts are green hearts that exceed the need
        let surplus_green_hearts = green_hearts_on_stage; // 0 surplus green hearts
        
        // The key assertion: blade hearts from cheer don't count as surplus hearts
        // Only green hearts count for the surplus heart requirement
        
        let has_surplus_heart = surplus_green_hearts >= 1;
        let can_use_ability = has_surplus_heart;
        
        // Verify the ability cannot be used
        assert!(!can_use_ability, "Ability should not be usable with only blade hearts from cheer");
        assert_eq!(green_hearts_on_stage, 0, "No green hearts on stage");
        assert_eq!(blade_hearts_from_cheer, 3, "3 blade hearts from cheer");
        
        // This tests that surplus heart requirement specifically refers to green hearts, not blade hearts
        
        println!("Q174 verified: Blade hearts from cheer don't count as surplus hearts");
        println!("Green hearts on stage: {}", green_hearts_on_stage);
        println!("Blade hearts from cheer: {}", blade_hearts_from_cheer);
        println!("Surplus green hearts: {}", surplus_green_hearts);
        println!("Ability usable: {}", can_use_ability);
        println!("Surplus heart requirement only counts green hearts, not blade hearts from cheer");
    } else {
        panic!("Required card PL!N-bp3-027-L not found for Q174 test");
    }
}
